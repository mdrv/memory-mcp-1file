---
work_package_id: WP06
title: "Search Tools"
phase: "Phase 5"
priority: P1
subtasks: ["T042", "T043", "T044", "T045", "T045a"]
lane: done
dependencies: ["WP05"]
history:
  - date: 2026-01-06
    action: created
    by: spec-kitty.tasks
  - date: 2026-01-06
    action: completed
    by: build
---

# WP06: Search Tools

## Objective

Implement vector search, BM25 search, and hybrid recall with RRF merge.

## Context

Search is the core value proposition - agents need to find relevant memories semantically. This WP adds 3 search tools.

**Reference**:
- `kitty-specs/001-memory-mcp-server/research.md` - RRF algorithm
- `kitty-specs/001-memory-mcp-server/contracts/mcp-tools.md` - Tool schemas

## Subtasks

### T042: Create graph/rrf.rs

**Location**: `src/graph/rrf.rs`

Implement Reciprocal Rank Fusion:

```rust
use std::collections::HashMap;

/// Merge ranked lists using Reciprocal Rank Fusion
/// 
/// RRF score = sum(1 / (k + rank_i)) for each list where item appears
/// 
/// k = 60 is standard (prevents high-ranked items from dominating)
pub fn rrf_merge<T: Eq + std::hash::Hash + Clone>(
    lists: Vec<Vec<(T, f32)>>,
    k: usize,
    limit: usize,
) -> Vec<(T, f32)> {
    let mut scores: HashMap<T, f32> = HashMap::new();
    
    for list in lists {
        for (rank, (item, _original_score)) in list.into_iter().enumerate() {
            let rrf_contribution = 1.0 / (k + rank + 1) as f32;
            *scores.entry(item).or_insert(0.0) += rrf_contribution;
        }
    }
    
    let mut results: Vec<_> = scores.into_iter().collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    results.truncate(limit);
    results
}

/// Weighted merge for hybrid search
/// Combines pre-computed scores with weights
pub fn weighted_merge(
    items: &[(String, f32, f32, f32)], // (id, vec_score, bm25_score, ppr_score)
    weights: (f32, f32, f32),           // (vec_weight, bm25_weight, ppr_weight)
) -> Vec<(String, f32)> {
    let (w_vec, w_bm25, w_ppr) = weights;
    
    items.iter()
        .map(|(id, vec, bm25, ppr)| {
            let combined = w_vec * vec + w_bm25 * bm25 + w_ppr * ppr;
            (id.clone(), combined)
        })
        .collect()
}
```

---

### T043: Implement tool: search

```rust
    /// Semantic search over memories. Returns memories most similar to the query, ordered by relevance.
    #[tool(description = "Semantic search over memories. Returns memories most similar to the query, ordered by relevance.")]
    async fn search(
        &self,
        /// The search query text (will be embedded and used for similarity search)
        query: String,
        /// Maximum number of results to return (default: 10, max: 50)
        limit: Option<usize>,
    ) -> Result<CallToolResult, McpError> {
        // Check embedding ready
        if self.state.embedding.status() != EmbeddingStatus::Ready {
            return Ok(CallToolResult::error("Embedding service not ready. Please try again."));
        }
        
        let limit = limit.unwrap_or(10).min(50);
        
        // Generate query embedding
        let embedding = self.state.embedding.embed(&query).await
            .map_err(|e| McpError::internal(e.to_string()))?;
        
        // Vector search
        let results = self.state.storage.vector_search(&embedding, limit).await
            .map_err(|e| McpError::internal(e.to_string()))?;
        
        Ok(CallToolResult::success(serde_json::json!({
            "results": results,
            "count": results.len(),
            "query": query
        })))
    }
```

---

### T044: Implement tool: search_text

```rust
    /// Full-text keyword search over memories using BM25. Best for exact keyword matching.
    #[tool(description = "Full-text keyword search over memories using BM25. Best for exact keyword matching.")]
    async fn search_text(
        &self,
        /// The keyword query for full-text search
        query: String,
        /// Maximum number of results (default: 10, max: 50)
        limit: Option<usize>,
    ) -> Result<CallToolResult, McpError> {
        let limit = limit.unwrap_or(10).min(50);
        
        // BM25 search (no embedding needed)
        let results = self.state.storage.bm25_search(&query, limit).await
            .map_err(|e| McpError::internal(e.to_string()))?;
        
        Ok(CallToolResult::success(serde_json::json!({
            "results": results,
            "count": results.len(),
            "query": query
        })))
    }
```

---

### T045: Implement tool: recall (hybrid)

```rust
    /// Hybrid search combining vector similarity, BM25 keywords, and graph context. Best quality retrieval.
    #[tool(description = "Hybrid search combining vector similarity, BM25 keywords, and graph context. Best quality retrieval.")]
    async fn recall(
        &self,
        /// Search query
        query: String,
        /// Maximum number of results (default: 10, max: 50)
        limit: Option<usize>,
        /// Weight for vector similarity (default: 0.40)
        vector_weight: Option<f32>,
        /// Weight for BM25 (default: 0.15)
        bm25_weight: Option<f32>,
        /// Weight for PPR graph ranking (default: 0.45)
        ppr_weight: Option<f32>,
    ) -> Result<CallToolResult, McpError> {
        // Check embedding ready
        if self.state.embedding.status() != EmbeddingStatus::Ready {
            return Ok(CallToolResult::error("Embedding service not ready. Please try again."));
        }
        
        let limit = limit.unwrap_or(10).min(50);
        let w_vec = vector_weight.unwrap_or(0.40);
        let w_bm25 = bm25_weight.unwrap_or(0.15);
        let w_ppr = ppr_weight.unwrap_or(0.45);
        
        // Generate query embedding
        let embedding = self.state.embedding.embed(&query).await
            .map_err(|e| McpError::internal(e.to_string()))?;
        
        // Get top 50 from each source for RRF
        let vec_results = self.state.storage.vector_search(&embedding, 50).await
            .map_err(|e| McpError::internal(e.to_string()))?;
        let bm25_results = self.state.storage.bm25_search(&query, 50).await
            .map_err(|e| McpError::internal(e.to_string()))?;
        
        // RRF merge (k=60)
        use crate::graph::rrf::rrf_merge;
        let vec_list: Vec<_> = vec_results.iter().map(|r| (r.id.clone(), r.score)).collect();
        let bm25_list: Vec<_> = bm25_results.iter().map(|r| (r.id.clone(), r.score)).collect();
        
        let merged = rrf_merge(vec![vec_list, bm25_list], 60, 20);
        
        // Build result with individual scores
        let mut scored_memories = Vec::new();
        for (id, rrf_score) in merged.into_iter().take(limit) {
            // Find original scores
            let vec_score = vec_results.iter()
                .find(|r| r.id == id)
                .map(|r| r.score)
                .unwrap_or(0.0);
            let bm25_score = bm25_results.iter()
                .find(|r| r.id == id)
                .map(|r| r.score)
                .unwrap_or(0.0);
            
            // PPR = 0 for now (placeholder until WP07)
            let ppr_score = 0.0;
            
            // Weighted final score
            let final_score = w_vec * vec_score + w_bm25 * bm25_score + w_ppr * ppr_score;
            
            // Get memory content
            if let Ok(Some(memory)) = self.state.storage.get_memory(&id).await {
                scored_memories.push(ScoredMemory {
                    id: id.clone(),
                    content: memory.content,
                    memory_type: memory.memory_type,
                    score: final_score,
                    vector_score: vec_score,
                    bm25_score,
                    ppr_score,
                });
            }
        }
        
        Ok(CallToolResult::success(serde_json::json!({
            "results": scored_memories,
            "count": scored_memories.len(),
            "query": query,
            "weights": {
                "vector": w_vec,
                "bm25": w_bm25,
                "ppr": w_ppr
            }
        })))
    }
```

---

---

### T045a: Write tests for WP06 components

**Goal**: Verify search tools.

**Implementation**:
- Verify RRF merge logic
- Verify `recall` tool weights
- Test search performance/latency

**Pass Criteria**:
- `cargo test` passes

---

## Definition of Done

1. search completes in < 20ms for 10K memories
2. search_text completes in < 30ms for 10K memories
3. recall returns merged results with all 3 score components
4. RRF merge with k=60 implemented correctly
5. PPR = 0 placeholder (to be updated in WP07)

## Risks

| Risk | Mitigation |
|------|------------|
| RRF score normalization | Use raw RRF scores, document range |
| Empty result handling | Return empty array, not error |

## Reviewer Guidance

- Verify RRF k=60 constant
- Check default weights: vector=0.40, bm25=0.15, ppr=0.45
- Confirm search_text works without embedding model
- Validate temporal filtering in storage queries
