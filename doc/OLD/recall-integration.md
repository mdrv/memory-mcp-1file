# Recall Integration — Design Document

## Overview

Integration of code search into the existing `recall` tool for unified hybrid search across memories, entities, and code chunks.

## Problem Statement

Currently, users need two separate tools:
- `search` / `recall` — for memories
- `search_code` — for code

This creates fragmentation where LLM must decide which tool to use, potentially missing cross-domain insights (e.g., documentation + implementation).

## Solution: Unified Recall

Extend `recall` to search **both** memories and code simultaneously, returning unified results ranked by hybrid scoring.

---

## Architecture

### Before (Current)

```
recall(query) → [
  vector_search(memories),
  bm25_search(memories),
] → RRF merge → PPR → Results
```

### After (Unified)

```
recall(query) → [
  vector_search(memories),
  vector_search(code),
  bm25_search(memories),
  bm25_search(code),
] → Z-score normalize → RRF merge → PPR → Unified Results
```

---

## Data Model

### SearchResult Enum

```rust
// src/types/search.rs

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum SearchResult {
    Memory {
        id: String,
        content: String,
        memory_type: String,
        score: f32,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<serde_json::Value>,
    },
    CodeChunk {
        id: String,
        content: String,
        file_path: String,
        language: String,
        start_line: u32,
        end_line: u32,
        chunk_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        score: f32,
    },
}

impl SearchResult {
    pub fn id(&self) -> &str {
        match self {
            SearchResult::Memory { id, .. } => id,
            SearchResult::CodeChunk { id, .. } => id,
        }
    }
    
    pub fn score(&self) -> f32 {
        match self {
            SearchResult::Memory { score, .. } => *score,
            SearchResult::CodeChunk { score, .. } => *score,
        }
    }
    
    pub fn set_score(&mut self, new_score: f32) {
        match self {
            SearchResult::Memory { score, .. } => *score = new_score,
            SearchResult::CodeChunk { score, .. } => *score = new_score,
        }
    }
}

impl From<Memory> for SearchResult {
    fn from(m: Memory) -> Self {
        SearchResult::Memory {
            id: m.id_string().unwrap_or_default(),
            content: m.content,
            memory_type: m.memory_type,
            metadata: m.metadata,
            score: 0.0,  // Set later
        }
    }
}

impl From<CodeChunk> for SearchResult {
    fn from(c: CodeChunk) -> Self {
        SearchResult::CodeChunk {
            id: c.id_string().unwrap_or_default(),
            content: c.content,
            file_path: c.file_path,
            language: c.language.to_string(),
            start_line: c.start_line,
            end_line: c.end_line,
            chunk_type: c.chunk_type.to_string(),
            name: c.name,
            score: 0.0,  // Set later
        }
    }
}
```

---

## RecallArgs Extension

```rust
// src/server/handler.rs

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RecallArgs {
    pub query: String,
    
    #[serde(default = "default_search_limit")]
    pub limit: usize,
    
    // Existing weights
    pub vector_weight: Option<f32>,
    pub bm25_weight: Option<f32>,
    pub ppr_weight: Option<f32>,
    
    // NEW: Type filters
    #[serde(default = "default_true")]
    pub include_memories: Option<bool>,
    
    #[serde(default = "default_true")]
    pub include_code: Option<bool>,
    
    // NEW: Code-specific filters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,  // "rust", "python", "javascript"
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

fn default_true() -> Option<bool> { Some(true) }
fn default_search_limit() -> usize { 10 }
```

---

## Implementation

### Recall Handler (Updated)

```rust
#[tool(description = "Hybrid search: memories + code + entities (best quality)")]
async fn recall(&self, params: Parameters<RecallArgs>) -> Result<CallToolResult> {
    let state = self.state.read().await;
    let args = params.0;
    
    let limit = args.limit.min(50);
    let vec_weight = args.vector_weight.unwrap_or(0.40);
    let bm25_weight = args.bm25_weight.unwrap_or(0.15);
    let ppr_weight = args.ppr_weight.unwrap_or(0.45);
    
    // 1. Embed query
    let query_embedding = state.embedder.embed(&args.query).await?;
    
    // 2. Parallel search across sources
    let include_memories = args.include_memories.unwrap_or(true);
    let include_code = args.include_code.unwrap_or(true);
    
    let mut all_results: Vec<SearchResult> = Vec::new();
    
    if include_memories {
        let (mem_vec, mem_bm25) = tokio::join!(
            state.storage.vector_search_memories(query_embedding.clone(), 25),
            state.storage.bm25_search_memories(&args.query, 25),
        );
        
        all_results.extend(
            rrf_merge(&[mem_vec?, mem_bm25?], RRF_K, 50)
                .into_iter()
                .map(SearchResult::from)
        );
    }
    
    if include_code {
        let (code_vec, code_bm25) = tokio::join!(
            state.storage.vector_search_code(query_embedding.clone(), 25),
            state.storage.bm25_search_code(&args.query, 25),
        );
        
        all_results.extend(
            rrf_merge(&[code_vec?, code_bm25?], RRF_K, 50)
                .into_iter()
                .map(SearchResult::from)
        );
    }
    
    // 3. Z-score normalization (fair competition)
    let normalized = z_score_normalize(all_results);
    
    // 4. Apply PPR + final ranking
    let seed_ids: Vec<String> = normalized.iter()
        .map(|r| r.id().to_string())
        .collect();
    
    let degrees = state.storage.get_node_degrees(&seed_ids).await?;
    let dampened = dampen_hub_weights(&seed_ids, &degrees);
    
    let mut final_results: Vec<(SearchResult, f32)> = normalized
        .into_iter()
        .map(|r| {
            let id = r.id().to_string();
            let ppr_score = dampened.get(&id).copied().unwrap_or(0.0);
            let final_score = 
                vec_weight * r.score() + 
                bm25_weight * r.score() + 
                ppr_weight * ppr_score;
            (r, final_score)
        })
        .collect();
    
    final_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    final_results.truncate(limit);
    
    // 5. Build response with breakdown
    let (memories, code): (Vec<_>, Vec<_>) = final_results
        .iter()
        .partition(|(r, _)| matches!(r, SearchResult::Memory { .. }));
    
    let response = json!({
        "results": final_results.iter().map(|(r, score)| {
            let mut result = serde_json::to_value(r).unwrap();
            result["score"] = json!(score);
            result
        }).collect::<Vec<_>>(),
        "count": final_results.len(),
        "breakdown": {
            "memories": memories.len(),
            "code": code.len(),
        },
        "query": args.query,
        "weights": {
            "vector": vec_weight,
            "bm25": bm25_weight,
            "ppr": ppr_weight,
        }
    });
    
    Ok(CallToolResult::success(vec![Content::text(response.to_string())]))
}
```

---

## Storage Backend Extensions

```rust
// src/storage/traits.rs

#[async_trait]
pub trait StorageBackend: Send + Sync {
    // Existing methods
    async fn vector_search(&self, query: Vec<f32>, limit: usize) -> Result<Vec<Memory>>;
    async fn bm25_search(&self, query: &str, limit: usize) -> Result<Vec<Memory>>;
    
    // NEW: Explicit memory methods (backwards compat)
    async fn vector_search_memories(&self, query: Vec<f32>, limit: usize) -> Result<Vec<Memory>> {
        self.vector_search(query, limit).await
    }
    
    async fn bm25_search_memories(&self, query: &str, limit: usize) -> Result<Vec<Memory>> {
        self.bm25_search(query, limit).await
    }
    
    // NEW: Code search methods
    async fn vector_search_code(&self, query: Vec<f32>, limit: usize) -> Result<Vec<CodeChunk>>;
    async fn bm25_search_code(&self, query: &str, limit: usize) -> Result<Vec<CodeChunk>>;
}
```

### SurrealDB Implementation

```rust
// src/storage/surrealdb.rs

impl StorageBackend for SurrealStorage {
    async fn vector_search_code(&self, query: Vec<f32>, limit: usize) -> Result<Vec<CodeChunk>> {
        let sql = format!(
            r#"
            SELECT *, vector::distance::knn() AS distance
            FROM code_chunks 
            WHERE embedding <|{}, 40|> $query
            ORDER BY distance ASC
            "#,
            limit
        );
        
        self.db
            .query(&sql)
            .bind(("query", query))
            .await?
            .check()?
            .take(0)
    }
    
    async fn bm25_search_code(&self, query: &str, limit: usize) -> Result<Vec<CodeChunk>> {
        let sql = format!(
            r#"
            SELECT *, search::score(1) as score
            FROM code_chunks
            WHERE content @1@ $query
            ORDER BY score DESC
            LIMIT {}
            "#,
            limit
        );
        
        self.db
            .query(&sql)
            .bind(("query", query))
            .await?
            .check()?
            .take(0)
    }
}
```

---

## Z-Score Normalization

```rust
// src/graph/rrf.rs or src/search/normalize.rs

fn z_score_normalize(results: Vec<SearchResult>) -> Vec<SearchResult> {
    // 1. Partition by type
    let (mut code, mut memory): (Vec<_>, Vec<_>) = results
        .into_iter()
        .partition(|r| matches!(r, SearchResult::CodeChunk { .. }));
    
    // 2. Normalize each group separately
    normalize_group(&mut code);
    normalize_group(&mut memory);
    
    // 3. Merge back
    code.extend(memory);
    code
}

fn normalize_group(results: &mut [SearchResult]) {
    if results.is_empty() {
        return;
    }
    
    let scores: Vec<f32> = results.iter().map(|r| r.score()).collect();
    let mean = scores.iter().sum::<f32>() / scores.len() as f32;
    let variance = scores.iter()
        .map(|s| (s - mean).powi(2))
        .sum::<f32>() / scores.len() as f32;
    let std_dev = variance.sqrt();
    
    if std_dev < 1e-6 {
        // All scores identical
        for r in results.iter_mut() {
            r.set_score(0.0);
        }
        return;
    }
    
    for r in results.iter_mut() {
        let normalized = (r.score() - mean) / std_dev;
        r.set_score(normalized);
    }
}
```

---

## Example Responses

### Mixed Results

```json
{
  "results": [
    {
      "type": "CodeChunk",
      "file_path": "src/auth/middleware.rs",
      "content": "pub fn verify_jwt(token: &str) -> Result<Claims> {...}",
      "language": "Rust",
      "start_line": 42,
      "end_line": 58,
      "chunk_type": "Function",
      "name": "verify_jwt",
      "score": 0.92
    },
    {
      "type": "Memory",
      "content": "JWT tokens expire after 24h, refresh tokens after 30 days",
      "memory_type": "semantic",
      "score": 0.87
    }
  ],
  "count": 2,
  "breakdown": {
    "memories": 1,
    "code": 1
  },
  "query": "JWT authentication",
  "weights": {
    "vector": 0.40,
    "bm25": 0.15,
    "ppr": 0.45
  }
}
```

### Code-Only Filter

```json
// Request
{
  "tool": "recall",
  "arguments": {
    "query": "database connection pooling",
    "include_memories": false,
    "include_code": true,
    "language": "rust"
  }
}

// Response
{
  "results": [
    {
      "type": "CodeChunk",
      "file_path": "src/db/pool.rs",
      // ...
    }
  ],
  "breakdown": {
    "memories": 0,
    "code": 5
  }
}
```

---

## Migration Path

### Phase 1: Add New Methods (Backwards Compatible)
- Add `vector_search_code()`, `bm25_search_code()` to trait
- Implement in SurrealStorage
- `recall` remains unchanged

### Phase 2: Extend RecallArgs (Opt-in)
- Add optional `include_code`, `language` fields
- Default: `include_code: true` (for unified search)
- Old clients work as before (no code results if field absent)

### Phase 3: Update Documentation
- Update `recall` description in tool list
- Add examples with code results
- Migration guide for existing integrations

---

## Performance Impact

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Latency (recall) | ~100ms | ~150ms | +50% (parallel) |
| Token usage | ~1KB/result | ~1.5KB/result | +50% (more fields) |
| Relevance | Good | Excellent | Code context |

**Mitigation**: Hard cap at 50 total results, embedding excluded from serialization.

---

## Testing Strategy

### Unit Tests
```rust
#[test]
fn test_z_score_normalization() {
    let results = vec![
        SearchResult::Memory { score: 0.9, .. },
        SearchResult::CodeChunk { score: 0.5, .. },
    ];
    
    let normalized = z_score_normalize(results);
    // Assert scores are comparable
}
```

### Integration Test
```rust
#[tokio::test]
async fn test_recall_unified_search() {
    // Setup: index memories + code
    // Query: "authentication"
    // Assert: both types in results
}
```

---

## Future Enhancements

- **Entity integration**: Include entities in unified search
- **Weighted sources**: Custom weights per source type
- **Faceted results**: Group by file/module/date
- **Snippet highlighting**: Show matching context
