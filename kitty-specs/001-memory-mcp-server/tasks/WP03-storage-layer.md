---
work_package_id: WP03
title: "Storage Layer"
phase: "Phase 2"
priority: P1
subtasks: ["T013", "T014", "T015", "T016", "T017", "T018", "T019", "T020", "T021", "T022", "T023", "T024", "T025"]
lane: planned
dependencies: ["WP02"]
history:
  - date: 2026-01-06
    action: created
    by: spec-kitty.tasks
---

# WP03: Storage Layer

## Objective

Implement the storage abstraction trait and complete SurrealDB implementation with all CRUD, search, graph, temporal, and code operations.

## Context

This is the data persistence layer. All 20 MCP tools depend on storage operations defined here.

**Can run in parallel with WP04** (Embedding Layer) - they share only types from WP02.

**Reference**:
- `kitty-specs/001-memory-mcp-server/data-model.md` - Schema definition
- `kitty-specs/001-memory-mcp-server/research.md` - SurrealDB patterns

## Subtasks

### T013: Create storage/traits.rs

**Location**: `src/storage/traits.rs`

Define the `StorageBackend` trait with async methods:

```rust
use async_trait::async_trait;
use crate::types::*;
use crate::Result;

#[async_trait]
pub trait StorageBackend: Send + Sync {
    // Memory CRUD
    async fn create_memory(&self, memory: Memory) -> Result<String>;
    async fn get_memory(&self, id: &str) -> Result<Option<Memory>>;
    async fn update_memory(&self, id: &str, update: MemoryUpdate) -> Result<Memory>;
    async fn delete_memory(&self, id: &str) -> Result<bool>;
    async fn list_memories(&self, limit: usize, offset: usize) -> Result<Vec<Memory>>;
    async fn count_memories(&self) -> Result<usize>;
    
    // Vector search
    async fn vector_search(&self, embedding: &[f32], limit: usize) -> Result<Vec<SearchResult>>;
    async fn vector_search_code(&self, embedding: &[f32], project_id: Option<&str>, limit: usize) -> Result<Vec<ScoredCodeChunk>>;
    
    // BM25 search
    async fn bm25_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>;
    async fn bm25_search_code(&self, query: &str, project_id: Option<&str>, limit: usize) -> Result<Vec<ScoredCodeChunk>>;
    
    // Entity operations
    async fn create_entity(&self, entity: Entity) -> Result<String>;
    async fn get_entity(&self, id: &str) -> Result<Option<Entity>>;
    async fn search_entities(&self, query: &str, limit: usize) -> Result<Vec<Entity>>;
    
    // Relation operations
    async fn create_relation(&self, relation: Relation) -> Result<String>;
    async fn get_related(&self, entity_id: &str, depth: usize, direction: Direction) -> Result<(Vec<Entity>, Vec<Relation>)>;
    async fn get_subgraph(&self, entity_ids: &[String]) -> Result<(Vec<Entity>, Vec<Relation>)>;
    async fn get_node_degrees(&self, entity_ids: &[String]) -> Result<std::collections::HashMap<String, usize>>;
    
    // Temporal operations
    async fn get_valid(&self, user_id: Option<&str>, limit: usize) -> Result<Vec<Memory>>;
    async fn get_valid_at(&self, timestamp: chrono::DateTime<chrono::Utc>, user_id: Option<&str>, limit: usize) -> Result<Vec<Memory>>;
    async fn invalidate(&self, id: &str, reason: Option<&str>, superseded_by: Option<&str>) -> Result<bool>;
    
    // Code operations
    async fn create_code_chunk(&self, chunk: CodeChunk) -> Result<String>;
    async fn create_code_chunks_batch(&self, chunks: Vec<CodeChunk>) -> Result<usize>;
    async fn delete_project_chunks(&self, project_id: &str) -> Result<usize>;
    async fn get_index_status(&self, project_id: &str) -> Result<Option<IndexStatus>>;
    async fn update_index_status(&self, status: IndexStatus) -> Result<()>;
    async fn list_projects(&self) -> Result<Vec<String>>;
    
    // System
    async fn health_check(&self) -> Result<bool>;
}
```

Add `async-trait = "0.1"` to Cargo.toml.

---

### T014: Create storage/schema.surql

**Location**: `src/storage/schema.surql`

Use the exact schema from `data-model.md` section "SurrealDB Schema (schema.surql)":
- memories table with HNSW index (DIMENSION 768)
- entities table with HNSW index
- relations table (TYPE RELATION)
- code_chunks table with multiple indexes
- index_status table

**Critical**: HNSW DIMENSION must be 768 for e5_multi default model.

---

### T015: Implement SurrealStorage init/connect

**Location**: `src/storage/surrealdb.rs`

```rust
pub struct SurrealStorage {
    db: Surreal<Db>,
}

impl SurrealStorage {
    pub async fn new(data_dir: &Path) -> Result<Self> {
        let db_path = data_dir.join("db");
        std::fs::create_dir_all(&db_path)?;
        
        let db = Surreal::new::<SpeeDb>(db_path).await?;
        db.use_ns("memory").use_db("main").await?;
        
        // Apply schema
        let schema = include_str!("schema.surql");
        db.query(schema).await?;
        
        Ok(Self { db })
    }
}
```

---

### T016: Implement Memory CRUD

Methods: `create_memory`, `get_memory`, `update_memory`, `delete_memory`

**create_memory**:
- Generate 20-char alphanumeric ID using nanoid pattern
- Insert with all fields including embedding
- Return ID string

**get_memory**:
- Query by ID
- Return Option<Memory>

**update_memory**:
- Fetch existing memory
- Apply only non-None fields from MemoryUpdate
- Re-save
- Return updated Memory

**delete_memory**:
- DELETE FROM memories WHERE id = $id
- Return true if deleted

---

### T017: Implement Memory list/count

**list_memories**:
```sql
SELECT * FROM memories 
ORDER BY ingestion_time DESC 
LIMIT $limit START $offset
```

**count_memories**:
```sql
SELECT count() FROM memories GROUP ALL
```

---

### T018: Implement Vector search

**vector_search**:
```sql
SELECT *, vector::similarity::cosine(embedding, $vec) AS score 
FROM memories 
WHERE embedding IS NOT NULL 
  AND (valid_until IS NULL OR valid_until > time::now())
ORDER BY score DESC 
LIMIT $limit
```

**vector_search_code**:
```sql
SELECT *, vector::similarity::cosine(embedding, $vec) AS score 
FROM code_chunks 
WHERE embedding IS NOT NULL
  AND ($project_id IS NULL OR project_id = $project_id)
ORDER BY score DESC 
LIMIT $limit
```

---

### T019: Implement BM25 search

**bm25_search**:
```sql
SELECT *, search::score(0) AS score 
FROM memories 
WHERE content @0@ $query
  AND (valid_until IS NULL OR valid_until > time::now())
ORDER BY score DESC 
LIMIT $limit
```

**bm25_search_code** - similar pattern with code_chunks table.

---

### T020: Implement Entity operations

**create_entity**:
- Generate ID
- INSERT with embedding from name

**get_entity**:
- SELECT by ID

**search_entities**:
- BM25 search on name field

---

### T021: Implement Relation operations

**create_relation**:
```sql
RELATE $from_entity->relations->$to_entity 
SET relation_type = $type, weight = $weight
```

**get_related**:
- Use recursive graph traversal up to depth
- Direction controls: `->`, `<-`, or both

**get_subgraph**:
- Fetch all relations between given entity IDs

**get_node_degrees**:
- Count incoming + outgoing edges per node

---

### T022: Implement Temporal operations

**get_valid**:
```sql
SELECT * FROM memories 
WHERE valid_until IS NULL OR valid_until > time::now()
LIMIT $limit
```

**get_valid_at**:
```sql
SELECT * FROM memories 
WHERE valid_from <= $timestamp 
  AND (valid_until IS NULL OR valid_until > $timestamp)
LIMIT $limit
```

**invalidate**:
```sql
UPDATE memories SET 
  valid_until = time::now(),
  invalidation_reason = $reason
WHERE id = $id
```

---

### T023: Implement Code operations

**create_code_chunk**: Single insert with embedding

**create_code_chunks_batch**: 
- Use transaction for atomic insert
- INSERT multiple chunks

**delete_project_chunks**:
```sql
DELETE FROM code_chunks WHERE project_id = $project_id
```

---

### T024: Implement Index status

**get_index_status**: SELECT by project_id

**update_index_status**: UPSERT by project_id

**list_projects**:
```sql
SELECT DISTINCT project_id FROM code_chunks
```

---

### T025: Implement health_check

```rust
async fn health_check(&self) -> Result<bool> {
    self.db.query("INFO FOR DB").await?;
    Ok(true)
}
```

---

## Definition of Done

1. All trait methods implemented
2. Integration tests with tempdir pass
3. HNSW dimension = 768
4. Vector search returns results with similarity scores
5. Temporal filtering works correctly

## Risks

| Risk | Mitigation |
|------|------------|
| SurrealDB query syntax changes | Pin version, test thoroughly |
| Graph traversal performance | Limit depth to 3 max |

## Reviewer Guidance

- Verify HNSW DIMENSION 768 in schema
- Check temporal filters in all search queries
- Confirm relation direction handling
- Test with mock 768-dim vectors
