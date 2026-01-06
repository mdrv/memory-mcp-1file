# Hybrid Retrieval — Design Document

## Overview

Hybrid search combining Vector similarity, BM25 full-text, and Personalized PageRank (PPR) for graph-aware ranking.

## Algorithm (V8: Unified PPR + BM25)

```
┌─────────────────────────────────────────────────────────────┐
│              UNIFIED HYBRID RETRIEVAL                        │
├─────────────────────────────────────────────────────────────┤
│  1. TEMPORAL PRE-FILTER                                      │
│     WHERE valid_from <= $now AND valid_until IS NONE/> $now  │
├─────────────────────────────────────────────────────────────┤
│  2. SEED RETRIEVAL (parallel)                                │
│     a) Vector HNSW → top-50                                  │
│     b) BM25 FTS → top-50                                     │
│     c) RRF Merge (k=60) → top-20 seeds                       │
├─────────────────────────────────────────────────────────────┤
│  3. HUB DAMPENING                                            │
│     weight = score / sqrt(degree)                            │
├─────────────────────────────────────────────────────────────┤
│  4. PPR DIFFUSION (petgraph)                                 │
│     damping α = 0.5, max_iter = 15                           │
├─────────────────────────────────────────────────────────────┤
│  5. FINAL SCORING                                            │
│     score = 0.40×vec + 0.15×bm25 + 0.45×ppr                  │
├─────────────────────────────────────────────────────────────┤
│  6. TOKEN BUDGET SELECTION                                   │
│     Fill until context budget reached                        │
└─────────────────────────────────────────────────────────────┘
```

## Scoring Formula

```
final_score = α × vector_sim + β × bm25_score + γ × ppr_score

α = 0.40 (semantic relevance)
β = 0.15 (keyword match)
γ = 0.45 (graph importance via PPR)
```

## Components

### Reciprocal Rank Fusion (RRF)

```rust
pub fn rrf_merge<T: Clone + Eq + Hash>(
    rankings: &[Vec<(T, f32)>],
    k: f32,
    limit: usize,
) -> Vec<(T, f32)> {
    let mut scores: HashMap<T, f32> = HashMap::new();
    
    for ranking in rankings {
        for (rank, (item, _score)) in ranking.iter().enumerate() {
            let rrf_score = 1.0 / (k + rank as f32 + 1.0);
            *scores.entry(item.clone()).or_insert(0.0) += rrf_score;
        }
    }
    
    let mut results: Vec<_> = scores.into_iter().collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    results.truncate(limit);
    results
}
```

### Hub Dampening

Prevents hub nodes from dominating results:

```rust
fn dampen_hubs(
    seeds: &[(String, f32)],
    degrees: &HashMap<String, usize>,
) -> Vec<(String, f32)> {
    seeds.iter().map(|(id, score)| {
        let degree = *degrees.get(id).unwrap_or(&1);
        let dampened = score / (degree as f32).sqrt();
        (id.clone(), dampened)
    }).collect()
}
```

### Personalized PageRank

See `graph-traversal.md` for full implementation.

## Implementation

```rust
pub async fn recall(
    &self,
    query: &str,
    limit: usize,
) -> Result<RecallResult> {
    let query_embedding = self.embedder.embed(query).await?;
    
    // 1. Parallel seed retrieval
    let (vec_results, bm25_results, degrees) = tokio::try_join!(
        self.storage.vector_search(&query_embedding, 50),
        self.storage.bm25_search(query, 50),
        self.storage.get_node_degrees(),
    )?;
    
    // 2. RRF merge
    let seeds = rrf_merge(
        &[vec_results.clone(), bm25_results.clone()],
        60.0,
        20,
    );
    
    // 3. Hub dampening
    let dampened = dampen_hubs(&seeds, &degrees);
    
    // 4. Build graph + PPR
    let subgraph = self.storage.get_subgraph(&dampened).await?;
    let ppr_scores = personalized_page_rank(
        &subgraph.to_petgraph(),
        &dampened.into_iter().collect(),
        0.5,   // damping
        1e-6,  // tolerance
    );
    
    // 5. Final scoring
    let scored: Vec<ScoredMemory> = seeds.iter().map(|(id, _)| {
        let vec_score = vec_results.iter()
            .find(|(i, _)| i == id)
            .map(|(_, s)| *s)
            .unwrap_or(0.0);
        let bm25_score = bm25_results.iter()
            .find(|(i, _)| i == id)
            .map(|(_, s)| *s)
            .unwrap_or(0.0);
        let ppr_score = ppr_scores.get(id).copied().unwrap_or(0.0);
        
        let final_score = 0.40 * vec_score + 0.15 * bm25_score + 0.45 * ppr_score;
        
        ScoredMemory {
            id: id.clone(),
            score: final_score,
            ..
        }
    }).collect();
    
    // 6. Sort and limit
    let mut results = scored;
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    results.truncate(limit);
    
    Ok(RecallResult {
        memories: results,
        subgraph: Some(subgraph),
    })
}
```

## SurrealDB Queries

### Vector Search

```sql
SELECT id, content, 
       vector::similarity::cosine(embedding, $query) AS score
FROM memories 
WHERE embedding <|50|> $query
  AND valid_from <= $now 
  AND (valid_until IS NONE OR valid_until > $now)
ORDER BY score DESC
LIMIT 50;
```

### BM25 Search

```sql
SELECT id, content,
       search::score(1) AS score
FROM memories 
WHERE content @1@ $query
  AND valid_from <= $now 
  AND (valid_until IS NONE OR valid_until > $now)
ORDER BY score DESC
LIMIT 50;
```

### Get Node Degrees

```sql
SELECT id, 
       count(<-relations) + count(->relations) AS degree
FROM entities;
```

## Code Search Variant

For code search, PPR is omitted (no graph for code chunks):

```rust
pub async fn search_code(
    &self,
    query: &str,
    project_id: Option<&str>,
    limit: usize,
) -> Result<Vec<CodeChunk>> {
    let query_embedding = self.embedder.embed(query).await?;
    
    // Parallel retrieval
    let (vec_results, bm25_results) = tokio::try_join!(
        self.storage.vector_search_code(&query_embedding, 50, project_id),
        self.storage.bm25_search_code(query, 50, project_id),
    )?;
    
    // RRF merge only (no PPR for code)
    let merged = rrf_merge(&[vec_results, bm25_results], 60.0, limit);
    
    Ok(merged)
}
```

## Performance

| Operation | Latency | Notes |
|-----------|---------|-------|
| Vector search | ~5ms | HNSW index |
| BM25 search | ~10ms | FTS index |
| RRF merge | <1ms | In-memory |
| PPR (20 nodes) | ~2ms | petgraph |
| Total recall | ~20ms | Parallel retrieval |

## Tuning Parameters

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| RRF k | 60 | Standard for similar-length rankings |
| Vector weight (α) | 0.40 | Semantic relevance important |
| BM25 weight (β) | 0.15 | Keyword match as tiebreaker |
| PPR weight (γ) | 0.45 | Graph context most important for recall |
| PPR damping | 0.5 | HippoRAG recommendation |
| Seed limit | 50 | Balance quality/speed |
| Final limit | 20 | Typical context window |
