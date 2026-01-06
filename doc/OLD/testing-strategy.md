# Testing Strategy — Design Document

## Overview

Testing approach for memory-mcp with focus on reliability and maintainability.

## Test Categories

### 1. Unit Tests

Located in each module file (`#[cfg(test)]` blocks).

```rust
// src/embedding/cache.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cache_insert_get() {
        let cache = EmbeddingCache::new(10);
        let embedding = vec![1.0, 2.0, 3.0];
        
        cache.insert("test".to_string(), embedding.clone());
        
        assert_eq!(cache.get("test"), Some(embedding));
    }
    
    #[test]
    fn test_cache_lru_eviction() {
        let cache = EmbeddingCache::new(2);
        
        cache.insert("a".to_string(), vec![1.0]);
        cache.insert("b".to_string(), vec![2.0]);
        cache.insert("c".to_string(), vec![3.0]); // evicts "a"
        
        assert!(cache.get("a").is_none());
        assert!(cache.get("b").is_some());
        assert!(cache.get("c").is_some());
    }
    
    #[test]
    fn test_cache_key_normalization() {
        let key1 = EmbeddingCache::cache_key("Hello World", "e5_multi");
        let key2 = EmbeddingCache::cache_key("  hello world  ", "e5_multi");
        
        assert_eq!(key1, key2);
    }
}
```

### 2. Integration Tests

Located in `tests/` directory.

```rust
// tests/storage_test.rs
use memory_mcp::{SurrealStorage, Memory, StorageBackend};
use tempfile::tempdir;

#[tokio::test]
async fn test_memory_crud() {
    let dir = tempdir().unwrap();
    let storage = SurrealStorage::new(dir.path()).await.unwrap();
    
    // Create
    let memory = Memory {
        content: "Test content".to_string(),
        embedding: Some(vec![0.1; 768]),
        ..Default::default()
    };
    let id = storage.create_memory(memory).await.unwrap();
    
    // Read
    let fetched = storage.get_memory(&id).await.unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().content, "Test content");
    
    // Update
    let updates = MemoryUpdate {
        content: Some("Updated content".to_string()),
        ..Default::default()
    };
    let updated = storage.update_memory(&id, updates).await.unwrap();
    assert_eq!(updated.unwrap().content, "Updated content");
    
    // Delete
    let deleted = storage.delete_memory(&id).await.unwrap();
    assert!(deleted);
    
    // Verify deleted
    let fetched = storage.get_memory(&id).await.unwrap();
    assert!(fetched.is_none());
}

#[tokio::test]
async fn test_vector_search() {
    let dir = tempdir().unwrap();
    let storage = SurrealStorage::new(dir.path()).await.unwrap();
    
    // Insert memories with embeddings
    for i in 0..10 {
        let mut embedding = vec![0.0; 768];
        embedding[i] = 1.0; // Different embeddings
        
        storage.create_memory(Memory {
            content: format!("Memory {}", i),
            embedding: Some(embedding),
            ..Default::default()
        }).await.unwrap();
    }
    
    // Search with query similar to first memory
    let mut query = vec![0.0; 768];
    query[0] = 1.0;
    
    let results = storage.vector_search(&query, 3).await.unwrap();
    
    assert_eq!(results.len(), 3);
    // First result should be closest match
}
```

```rust
// tests/graph_test.rs
use memory_mcp::{SurrealStorage, Entity, StorageBackend, Direction};
use tempfile::tempdir;

#[tokio::test]
async fn test_entity_relation() {
    let dir = tempdir().unwrap();
    let storage = SurrealStorage::new(dir.path()).await.unwrap();
    
    // Create entities
    let alice = storage.create_entity(Entity {
        name: "Alice".to_string(),
        entity_type: "person".to_string(),
        ..Default::default()
    }).await.unwrap();
    
    let project = storage.create_entity(Entity {
        name: "memory-mcp".to_string(),
        entity_type: "project".to_string(),
        ..Default::default()
    }).await.unwrap();
    
    // Create relation
    storage.create_relation(&alice, &project, "works_on", 1.0).await.unwrap();
    
    // Traverse
    let related = storage.get_related(&alice, 1, Direction::Outgoing).await.unwrap();
    
    assert_eq!(related.len(), 1);
    assert_eq!(related[0].name, "memory-mcp");
}

#[tokio::test]
async fn test_subgraph_extraction() {
    let dir = tempdir().unwrap();
    let storage = SurrealStorage::new(dir.path()).await.unwrap();
    
    // Build small graph
    let a = storage.create_entity(entity("A")).await.unwrap();
    let b = storage.create_entity(entity("B")).await.unwrap();
    let c = storage.create_entity(entity("C")).await.unwrap();
    
    storage.create_relation(&a, &b, "knows", 1.0).await.unwrap();
    storage.create_relation(&b, &c, "knows", 1.0).await.unwrap();
    
    // Extract subgraph from A
    let seeds = vec![(a.clone(), 1.0)];
    let subgraph = storage.get_subgraph(&seeds).await.unwrap();
    
    // Should include A and B (1-hop)
    assert!(subgraph.nodes.len() >= 2);
}
```

```rust
// tests/temporal_test.rs
use memory_mcp::{SurrealStorage, Memory, StorageBackend};
use chrono::{Utc, Duration};
use tempfile::tempdir;

#[tokio::test]
async fn test_temporal_validity() {
    let dir = tempdir().unwrap();
    let storage = SurrealStorage::new(dir.path()).await.unwrap();
    
    // Create memory
    let id = storage.create_memory(Memory {
        content: "Valid fact".to_string(),
        ..Default::default()
    }).await.unwrap();
    
    // Should be valid now
    let valid = storage.get_valid().await.unwrap();
    assert!(valid.iter().any(|m| m.id.as_ref().map(|i| i.to_string()) == Some(id.clone())));
    
    // Invalidate
    storage.invalidate(&id, Some("outdated")).await.unwrap();
    
    // Should no longer be valid
    let valid = storage.get_valid().await.unwrap();
    assert!(!valid.iter().any(|m| m.id.as_ref().map(|i| i.to_string()) == Some(id.clone())));
}

#[tokio::test]
async fn test_point_in_time_query() {
    let dir = tempdir().unwrap();
    let storage = SurrealStorage::new(dir.path()).await.unwrap();
    
    let now = Utc::now();
    let past = now - Duration::hours(1);
    
    // Create memory valid now
    storage.create_memory(Memory {
        content: "Current fact".to_string(),
        ..Default::default()
    }).await.unwrap();
    
    // Query at past time - should not include new memory
    let at_past = storage.get_valid_at(past).await.unwrap();
    assert!(at_past.is_empty());
    
    // Query at now - should include
    let at_now = storage.get_valid_at(now).await.unwrap();
    assert!(!at_now.is_empty());
}
```

```rust
// tests/recall_test.rs
use memory_mcp::{SurrealStorage, EmbeddingService, Memory, rrf_merge};
use tempfile::tempdir;

#[tokio::test]
async fn test_rrf_merge() {
    let vec_results = vec![
        ("a".to_string(), 0.9),
        ("b".to_string(), 0.8),
        ("c".to_string(), 0.7),
    ];
    
    let bm25_results = vec![
        ("b".to_string(), 0.95),
        ("d".to_string(), 0.85),
        ("a".to_string(), 0.75),
    ];
    
    let merged = rrf_merge(&[vec_results, bm25_results], 60.0, 5);
    
    // "a" and "b" appear in both, should rank higher
    assert!(merged.iter().position(|(id, _)| id == "a").unwrap() < 3);
    assert!(merged.iter().position(|(id, _)| id == "b").unwrap() < 3);
}

#[tokio::test]
async fn test_hybrid_recall() {
    let dir = tempdir().unwrap();
    let storage = SurrealStorage::new(dir.path()).await.unwrap();
    
    // Setup: create memories with both content and embeddings
    // This is a full integration test
    
    // ... implementation
}
```

```rust
// tests/bm25_test.rs
use memory_mcp::{SurrealStorage, Memory, StorageBackend};
use tempfile::tempdir;

#[tokio::test]
async fn test_bm25_search() {
    let dir = tempdir().unwrap();
    let storage = SurrealStorage::new(dir.path()).await.unwrap();
    
    // Insert memories with different content
    storage.create_memory(Memory {
        content: "Rust programming language is fast".to_string(),
        embedding: Some(vec![0.0; 768]),
        ..Default::default()
    }).await.unwrap();
    
    storage.create_memory(Memory {
        content: "Python is great for data science".to_string(),
        embedding: Some(vec![0.0; 768]),
        ..Default::default()
    }).await.unwrap();
    
    // Search for "Rust"
    let results = storage.bm25_search("Rust programming", 10).await.unwrap();
    
    assert!(!results.is_empty());
    // First result should contain "Rust"
}
```

### 3. Embedding Tests

```rust
// tests/embedding_test.rs
use memory_mcp::{EmbeddingService, EmbeddingConfig, ModelType};

#[tokio::test]
#[ignore = "requires model download"]
async fn test_embedding_service() {
    let service = EmbeddingService::new(EmbeddingConfig {
        model: ModelType::E5Small, // Smallest for faster test
        preload: true,
        timeout_secs: 120,
        ..Default::default()
    }).await.unwrap();
    
    let embedding = service.embed("Hello, world!").await.unwrap();
    
    assert_eq!(embedding.len(), 384); // e5_small dimensions
    
    // Embeddings should be normalized
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 0.01);
}

#[tokio::test]
#[ignore = "requires model download"]
async fn test_embedding_similarity() {
    let service = EmbeddingService::new(EmbeddingConfig {
        model: ModelType::E5Small,
        preload: true,
        ..Default::default()
    }).await.unwrap();
    
    let emb1 = service.embed("The cat sat on the mat").await.unwrap();
    let emb2 = service.embed("A cat was sitting on a rug").await.unwrap();
    let emb3 = service.embed("Quantum physics is complex").await.unwrap();
    
    let sim_12 = cosine_similarity(&emb1, &emb2);
    let sim_13 = cosine_similarity(&emb1, &emb3);
    
    // Similar sentences should have higher similarity
    assert!(sim_12 > sim_13);
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
```

### 4. MCP Handler Tests

```rust
// tests/handler_test.rs
use memory_mcp::{MemoryMcpServer, AppState, SurrealStorage, EmbeddingService};
use rmcp::ServerHandler;
use tempfile::tempdir;
use serde_json::json;

#[tokio::test]
async fn test_store_memory_tool() {
    let (server, _dir) = setup_test_server().await;
    
    let result = server.call_tool("store_memory", json!({
        "content": "Test memory content"
    })).await.unwrap();
    
    assert!(!result.is_error);
    let response: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert!(response.get("id").is_some());
}

#[tokio::test]
async fn test_get_memory_not_found() {
    let (server, _dir) = setup_test_server().await;
    
    let result = server.call_tool("get_memory", json!({
        "id": "nonexistent"
    })).await.unwrap();
    
    assert!(result.is_error);
    assert!(result.content[0].text.contains("not found"));
}

#[tokio::test]
async fn test_search_with_limit() {
    let (server, _dir) = setup_test_server().await;
    
    // Store some memories first
    for i in 0..10 {
        server.call_tool("store_memory", json!({
            "content": format!("Memory number {}", i)
        })).await.unwrap();
    }
    
    let result = server.call_tool("search", json!({
        "query": "memory",
        "limit": 5
    })).await.unwrap();
    
    let memories: Vec<serde_json::Value> = serde_json::from_str(&result.content[0].text).unwrap();
    assert!(memories.len() <= 5);
}

async fn setup_test_server() -> (MemoryMcpServer, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let storage = Arc::new(SurrealStorage::new(dir.path()).await.unwrap());
    
    // Use mock embedder for faster tests
    let embedder = Arc::new(MockEmbeddingService::new());
    
    let state = AppState {
        storage,
        embedder,
        config: Default::default(),
    };
    
    (MemoryMcpServer::new(state), dir)
}
```

### 5. PPR Algorithm Tests

```rust
// tests/ppr_test.rs
use memory_mcp::personalized_page_rank;
use petgraph::graph::DiGraph;
use std::collections::HashMap;

#[test]
fn test_ppr_simple_chain() {
    // A -> B -> C
    let mut graph = DiGraph::new();
    let a = graph.add_node("A");
    let b = graph.add_node("B");
    let c = graph.add_node("C");
    graph.add_edge(a, b, 1.0);
    graph.add_edge(b, c, 1.0);
    
    // Personalization on A
    let mut pers = HashMap::new();
    pers.insert(a, 1.0);
    
    let ranks = personalized_page_rank(&graph, &pers, 0.5, 1e-6);
    
    // A should have highest rank (seed node)
    assert!(ranks[&a] > ranks[&b]);
    assert!(ranks[&b] > ranks[&c]);
}

#[test]
fn test_ppr_hub_detection() {
    // Hub: A connects to B, C, D
    let mut graph = DiGraph::new();
    let a = graph.add_node("A");
    let b = graph.add_node("B");
    let c = graph.add_node("C");
    let d = graph.add_node("D");
    graph.add_edge(a, b, 1.0);
    graph.add_edge(a, c, 1.0);
    graph.add_edge(a, d, 1.0);
    
    // Personalization on B
    let mut pers = HashMap::new();
    pers.insert(b, 1.0);
    
    let ranks = personalized_page_rank(&graph, &pers, 0.5, 1e-6);
    
    // B is seed, should have high rank
    // A connects to B, should have some rank
    assert!(ranks[&b] > 0.0);
}

#[test]
fn test_ppr_convergence() {
    // Cycle: A -> B -> C -> A
    let mut graph = DiGraph::new();
    let a = graph.add_node("A");
    let b = graph.add_node("B");
    let c = graph.add_node("C");
    graph.add_edge(a, b, 1.0);
    graph.add_edge(b, c, 1.0);
    graph.add_edge(c, a, 1.0);
    
    let mut pers = HashMap::new();
    pers.insert(a, 1.0);
    
    let ranks = personalized_page_rank(&graph, &pers, 0.5, 1e-6);
    
    // All ranks should sum to ~1.0
    let total: f64 = ranks.values().sum();
    assert!((total - 1.0).abs() < 0.1);
}
```

## Test Configuration

### Cargo.toml

```toml
[dev-dependencies]
tempfile = "3"
tokio-test = "0.4"
criterion = { version = "0.5", features = ["async_tokio"] }

[[test]]
name = "integration"
path = "tests/lib.rs"

[[bench]]
name = "embedding"
harness = false

[[bench]]
name = "search"
harness = false
```

### Running Tests

```bash
# All tests
cargo test

# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test '*'

# With model download (slow)
cargo test -- --ignored

# Specific test
cargo test test_memory_crud

# With logging
RUST_LOG=debug cargo test -- --nocapture
```

## Benchmarks

```rust
// benches/embedding.rs
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use memory_mcp::{EmbeddingService, EmbeddingConfig, ModelType};
use tokio::runtime::Runtime;

fn bench_embedding(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let service = rt.block_on(async {
        EmbeddingService::new(EmbeddingConfig {
            model: ModelType::E5Small,
            preload: true,
            ..Default::default()
        }).await.unwrap()
    });
    
    let texts = vec![
        "Short text",
        "A medium length sentence with more words",
        "A longer piece of text that contains multiple sentences. It should test how the model handles longer inputs with more context.",
    ];
    
    for text in texts {
        c.bench_with_input(
            BenchmarkId::new("embed", text.len()),
            &text,
            |b, &text| {
                b.to_async(&rt).iter(|| service.embed(text))
            },
        );
    }
}

criterion_group!(benches, bench_embedding);
criterion_main!(benches);
```

## CI Configuration

```yaml
# .github/workflows/test.yml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      
      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
      
      - name: Run tests
        run: cargo test --all-features
      
      - name: Run clippy
        run: cargo clippy -- -D warnings
      
      - name: Check formatting
        run: cargo fmt --check

  integration:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      
      - name: Run integration tests
        run: cargo test --test '*' -- --ignored
        timeout-minutes: 30
```

## Mocking

### Mock Embedding Service

```rust
pub struct MockEmbeddingService {
    dims: usize,
}

impl MockEmbeddingService {
    pub fn new() -> Self {
        Self { dims: 768 }
    }
}

#[async_trait]
impl EmbeddingProvider for MockEmbeddingService {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Deterministic embedding based on text hash
        let hash = blake3::hash(text.as_bytes());
        let bytes = hash.as_bytes();
        
        let embedding: Vec<f32> = (0..self.dims)
            .map(|i| {
                let byte = bytes[i % 32] as f32;
                (byte / 255.0) * 2.0 - 1.0  // Normalize to [-1, 1]
            })
            .collect();
        
        // Normalize
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        Ok(embedding.into_iter().map(|x| x / norm).collect())
    }
    
    fn is_ready(&self) -> bool {
        true
    }
}
```
