# Storage Backend — Design Document

## Overview

Trait-based storage abstraction with SurrealDB as primary implementation.

## StorageBackend Trait

```rust
use async_trait::async_trait;

#[async_trait]
pub trait StorageBackend: Send + Sync {
    // === Memory CRUD ===
    async fn create_memory(&self, memory: Memory) -> Result<String>;
    async fn get_memory(&self, id: &str) -> Result<Option<Memory>>;
    async fn update_memory(&self, id: &str, updates: MemoryUpdate) -> Result<Option<Memory>>;
    async fn delete_memory(&self, id: &str) -> Result<bool>;
    async fn list_memories(&self, limit: usize, offset: usize) -> Result<Vec<Memory>>;
    
    // === Vector Search ===
    async fn vector_search(
        &self, 
        query: &[f32], 
        limit: usize,
    ) -> Result<Vec<(String, f32)>>;
    
    async fn vector_search_code(
        &self,
        query: &[f32],
        limit: usize,
        project_id: Option<&str>,
    ) -> Result<Vec<(String, f32)>>;
    
    // === Full-Text Search ===
    async fn bm25_search(&self, query: &str, limit: usize) -> Result<Vec<(String, f32)>>;
    async fn bm25_search_code(
        &self,
        query: &str,
        limit: usize,
        project_id: Option<&str>,
    ) -> Result<Vec<(String, f32)>>;
    
    // === Entity CRUD ===
    async fn create_entity(&self, entity: Entity) -> Result<String>;
    async fn get_entity(&self, id: &str) -> Result<Option<Entity>>;
    async fn search_entities(&self, query: &str, limit: usize) -> Result<Vec<Entity>>;
    
    // === Relations ===
    async fn create_relation(
        &self,
        from_id: &str,
        to_id: &str,
        relation_type: &str,
        weight: f32,
    ) -> Result<String>;
    
    async fn get_related(
        &self,
        entity_id: &str,
        depth: u32,
        direction: Direction,
    ) -> Result<Vec<Entity>>;
    
    async fn get_subgraph(&self, seeds: &[(String, f32)]) -> Result<Subgraph>;
    async fn get_node_degrees(&self) -> Result<HashMap<String, usize>>;
    
    // === Temporal ===
    async fn get_valid(&self) -> Result<Vec<Memory>>;
    async fn get_valid_at(&self, timestamp: DateTime<Utc>) -> Result<Vec<Memory>>;
    async fn invalidate(&self, id: &str, reason: Option<&str>) -> Result<bool>;
    
    // === Code Chunks ===
    async fn create_code_chunk(&self, chunk: CodeChunk) -> Result<String>;
    async fn create_code_chunks_batch(
        &self,
        chunks: &[CodeChunk],
        embeddings: Vec<Vec<f32>>,
    ) -> Result<Vec<String>>;
    async fn delete_project_chunks(&self, project_id: &str) -> Result<usize>;
    async fn get_index_status(&self, project_id: &str) -> Result<Option<IndexStatus>>;
    async fn list_projects(&self) -> Result<Vec<String>>;
    
    // === System ===
    async fn health_check(&self) -> Result<()>;
}
```

## SurrealDB Implementation

```rust
use surrealdb::{Surreal, engine::local::Db};
use surrealdb::engine::local::SurrealKv;

pub struct SurrealStorage {
    db: Surreal<Db>,
}

impl SurrealStorage {
    pub async fn new(data_dir: &Path) -> Result<Self> {
        let db_path = data_dir.join("db");
        fs::create_dir_all(&db_path)?;
        
        let db = Surreal::new::<SurrealKv>(db_path).await?;
        db.use_ns("memory").use_db("main").await?;
        
        // Initialize schema
        Self::init_schema(&db).await?;
        
        Ok(Self { db })
    }
    
    async fn init_schema(db: &Surreal<Db>) -> Result<()> {
        db.query(include_str!("schema.surql")).await?;
        Ok(())
    }
}
```

### Memory Operations

```rust
#[async_trait]
impl StorageBackend for SurrealStorage {
    async fn create_memory(&self, memory: Memory) -> Result<String> {
        let result: Option<Memory> = self.db
            .create("memories")
            .content(memory)
            .await?;
        
        Ok(result.unwrap().id.unwrap().to_string())
    }
    
    async fn get_memory(&self, id: &str) -> Result<Option<Memory>> {
        let result: Option<Memory> = self.db
            .select(("memories", id))
            .await?;
        
        Ok(result)
    }
    
    async fn update_memory(&self, id: &str, updates: MemoryUpdate) -> Result<Option<Memory>> {
        let result: Option<Memory> = self.db
            .update(("memories", id))
            .merge(updates)
            .await?;
        
        Ok(result)
    }
    
    async fn delete_memory(&self, id: &str) -> Result<bool> {
        let result: Option<Memory> = self.db
            .delete(("memories", id))
            .await?;
        
        Ok(result.is_some())
    }
    
    async fn list_memories(&self, limit: usize, offset: usize) -> Result<Vec<Memory>> {
        let mut result = self.db
            .query("SELECT * FROM memories ORDER BY created_at DESC LIMIT $limit START $offset")
            .bind(("limit", limit))
            .bind(("offset", offset))
            .await?;
        
        Ok(result.take(0)?)
    }
}
```

### Vector Search

```rust
impl SurrealStorage {
    async fn vector_search(
        &self,
        query: &[f32],
        limit: usize,
    ) -> Result<Vec<(String, f32)>> {
        let mut result = self.db
            .query(r#"
                SELECT id, 
                       vector::similarity::cosine(embedding, $query) AS score
                FROM memories 
                WHERE embedding <|$limit|> $query
                  AND valid_from <= time::now()
                  AND (valid_until IS NONE OR valid_until > time::now())
                ORDER BY score DESC
            "#)
            .bind(("query", query.to_vec()))
            .bind(("limit", limit))
            .await?;
        
        let rows: Vec<SearchRow> = result.take(0)?;
        Ok(rows.into_iter().map(|r| (r.id, r.score)).collect())
    }
    
    async fn vector_search_code(
        &self,
        query: &[f32],
        limit: usize,
        project_id: Option<&str>,
    ) -> Result<Vec<(String, f32)>> {
        let project_filter = project_id
            .map(|p| format!("AND project_id = '{}'", p))
            .unwrap_or_default();
        
        let query_str = format!(r#"
            SELECT id,
                   vector::similarity::cosine(embedding, $query) AS score
            FROM code_chunks
            WHERE embedding <|$limit|> $query
            {}
            ORDER BY score DESC
        "#, project_filter);
        
        let mut result = self.db
            .query(&query_str)
            .bind(("query", query.to_vec()))
            .bind(("limit", limit))
            .await?;
        
        let rows: Vec<SearchRow> = result.take(0)?;
        Ok(rows.into_iter().map(|r| (r.id, r.score)).collect())
    }
}
```

### BM25 Search

```rust
impl SurrealStorage {
    async fn bm25_search(&self, query: &str, limit: usize) -> Result<Vec<(String, f32)>> {
        let mut result = self.db
            .query(r#"
                SELECT id, search::score(1) AS score
                FROM memories
                WHERE content @1@ $query
                  AND valid_from <= time::now()
                  AND (valid_until IS NONE OR valid_until > time::now())
                ORDER BY score DESC
                LIMIT $limit
            "#)
            .bind(("query", query))
            .bind(("limit", limit))
            .await?;
        
        let rows: Vec<SearchRow> = result.take(0)?;
        Ok(rows.into_iter().map(|r| (r.id, r.score)).collect())
    }
}
```

### Graph Operations

```rust
impl SurrealStorage {
    async fn create_relation(
        &self,
        from_id: &str,
        to_id: &str,
        relation_type: &str,
        weight: f32,
    ) -> Result<String> {
        let mut result = self.db
            .query(r#"
                RELATE entities:$from->relations->entities:$to SET
                    relation_type = $type,
                    weight = $weight,
                    valid_from = time::now()
            "#)
            .bind(("from", from_id))
            .bind(("to", to_id))
            .bind(("type", relation_type))
            .bind(("weight", weight))
            .await?;
        
        let relation: Option<Relation> = result.take(0)?;
        Ok(relation.unwrap().id.to_string())
    }
    
    async fn get_related(
        &self,
        entity_id: &str,
        depth: u32,
        direction: Direction,
    ) -> Result<Vec<Entity>> {
        let query = match direction {
            Direction::Outgoing => format!(
                "SELECT VALUE ->relations.{{1..{}}}.->entities FROM entities:{}",
                depth, entity_id
            ),
            Direction::Incoming => format!(
                "SELECT VALUE <-relations.{{1..{}}}.<-entities FROM entities:{}",
                depth, entity_id
            ),
            Direction::Both => format!(
                "SELECT VALUE array::union(
                    ->relations.{{1..{depth}}}.->entities,
                    <-relations.{{1..{depth}}}.<-entities
                ) FROM entities:{id}",
                depth = depth, id = entity_id
            ),
        };
        
        let mut result = self.db.query(&query).await?;
        Ok(result.take(0)?)
    }
    
    async fn get_subgraph(&self, seeds: &[(String, f32)]) -> Result<Subgraph> {
        let seed_ids: Vec<&str> = seeds.iter().map(|(id, _)| id.as_str()).collect();
        
        let mut result = self.db
            .query(r#"
                LET $seeds = $seed_ids;
                
                LET $nodes = (
                    SELECT * FROM entities 
                    WHERE id IN $seeds
                    OR id IN (SELECT VALUE ->relations->entities.id FROM entities WHERE id IN $seeds)
                    OR id IN (SELECT VALUE <-relations<-entities.id FROM entities WHERE id IN $seeds)
                );
                
                LET $node_ids = $nodes.*.id;
                LET $edges = (
                    SELECT * FROM relations 
                    WHERE in IN $node_ids AND out IN $node_ids
                );
                
                RETURN { nodes: $nodes, edges: $edges };
            "#)
            .bind(("seed_ids", seed_ids))
            .await?;
        
        Ok(result.take(0)?)
    }
    
    async fn get_node_degrees(&self) -> Result<HashMap<String, usize>> {
        let mut result = self.db
            .query(r#"
                SELECT id,
                       count(<-relations) + count(->relations) AS degree
                FROM entities
            "#)
            .await?;
        
        let rows: Vec<DegreeRow> = result.take(0)?;
        Ok(rows.into_iter().map(|r| (r.id, r.degree)).collect())
    }
}
```

### Temporal Operations

```rust
impl SurrealStorage {
    async fn get_valid(&self) -> Result<Vec<Memory>> {
        let mut result = self.db
            .query(r#"
                SELECT * FROM memories
                WHERE valid_from <= time::now()
                  AND (valid_until IS NONE OR valid_until > time::now())
                ORDER BY created_at DESC
            "#)
            .await?;
        
        Ok(result.take(0)?)
    }
    
    async fn get_valid_at(&self, timestamp: DateTime<Utc>) -> Result<Vec<Memory>> {
        let mut result = self.db
            .query(r#"
                SELECT * FROM memories
                WHERE valid_from <= $ts
                  AND (valid_until IS NONE OR valid_until > $ts)
                ORDER BY created_at DESC
            "#)
            .bind(("ts", timestamp))
            .await?;
        
        Ok(result.take(0)?)
    }
    
    async fn invalidate(&self, id: &str, reason: Option<&str>) -> Result<bool> {
        let mut result = self.db
            .query(r#"
                UPDATE memories:$id SET
                    valid_until = time::now(),
                    invalidation_reason = $reason
                WHERE valid_until IS NONE
            "#)
            .bind(("id", id))
            .bind(("reason", reason))
            .await?;
        
        let updated: Option<Memory> = result.take(0)?;
        Ok(updated.is_some())
    }
}
```

### Code Chunks

```rust
impl SurrealStorage {
    async fn create_code_chunks_batch(
        &self,
        chunks: &[CodeChunk],
        embeddings: Vec<Vec<f32>>,
    ) -> Result<Vec<String>> {
        let mut ids = Vec::with_capacity(chunks.len());
        
        for (chunk, embedding) in chunks.iter().zip(embeddings) {
            let mut chunk = chunk.clone();
            chunk.embedding = Some(embedding);
            
            let result: Option<CodeChunk> = self.db
                .create("code_chunks")
                .content(chunk)
                .await?;
            
            ids.push(result.unwrap().id.unwrap().to_string());
        }
        
        Ok(ids)
    }
    
    async fn delete_project_chunks(&self, project_id: &str) -> Result<usize> {
        let result = self.db
            .query("DELETE FROM code_chunks WHERE project_id = $pid RETURN BEFORE")
            .bind(("pid", project_id))
            .await?;
        
        Ok(result.num_statements())
    }
}
```

## Schema File (schema.surql)

```sql
-- Memories
DEFINE TABLE IF NOT EXISTS memories SCHEMAFULL;
DEFINE FIELD content          ON memories TYPE string;
DEFINE FIELD embedding        ON memories TYPE option<array<float>>;
DEFINE FIELD memory_type      ON memories TYPE string DEFAULT 'semantic';
DEFINE FIELD user_id          ON memories TYPE option<string>;
DEFINE FIELD metadata         ON memories TYPE option<object>;
DEFINE FIELD created_at       ON memories TYPE datetime DEFAULT time::now();
DEFINE FIELD valid_from       ON memories TYPE datetime DEFAULT time::now();
DEFINE FIELD valid_until      ON memories TYPE option<datetime>;
DEFINE FIELD invalidation_reason ON memories TYPE option<string>;

DEFINE INDEX IF NOT EXISTS idx_memories_vec ON memories 
    FIELDS embedding HNSW DIMENSION 768 DIST COSINE;
DEFINE INDEX IF NOT EXISTS idx_memories_fts ON memories 
    FIELDS content SEARCH ANALYZER simple BM25;

-- Entities
DEFINE TABLE IF NOT EXISTS entities SCHEMAFULL;
DEFINE FIELD name             ON entities TYPE string;
DEFINE FIELD entity_type      ON entities TYPE string;
DEFINE FIELD description      ON entities TYPE option<string>;
DEFINE FIELD embedding        ON entities TYPE option<array<float>>;
DEFINE FIELD created_at       ON entities TYPE datetime DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_entities_vec ON entities 
    FIELDS embedding HNSW DIMENSION 768 DIST COSINE;
DEFINE INDEX IF NOT EXISTS idx_entities_fts ON entities 
    FIELDS name SEARCH ANALYZER simple BM25;

-- Relations
DEFINE TABLE IF NOT EXISTS relations TYPE RELATION IN entities OUT entities SCHEMAFULL;
DEFINE FIELD relation_type    ON relations TYPE string;
DEFINE FIELD weight           ON relations TYPE float DEFAULT 1.0;
DEFINE FIELD valid_from       ON relations TYPE datetime DEFAULT time::now();
DEFINE FIELD valid_until      ON relations TYPE option<datetime>;

-- Code Chunks
DEFINE TABLE IF NOT EXISTS code_chunks SCHEMAFULL;
DEFINE FIELD file_path        ON code_chunks TYPE string;
DEFINE FIELD content          ON code_chunks TYPE string;
DEFINE FIELD language         ON code_chunks TYPE string DEFAULT 'unknown';
DEFINE FIELD start_line       ON code_chunks TYPE int;
DEFINE FIELD end_line         ON code_chunks TYPE int;
DEFINE FIELD chunk_type       ON code_chunks TYPE string DEFAULT 'other';
DEFINE FIELD name             ON code_chunks TYPE option<string>;
DEFINE FIELD embedding        ON code_chunks TYPE option<array<float>>;
DEFINE FIELD content_hash     ON code_chunks TYPE string;
DEFINE FIELD project_id       ON code_chunks TYPE option<string>;
DEFINE FIELD indexed_at       ON code_chunks TYPE datetime DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_chunks_vec ON code_chunks 
    FIELDS embedding HNSW DIMENSION 768 DIST COSINE;
DEFINE INDEX IF NOT EXISTS idx_chunks_fts ON code_chunks 
    FIELDS content SEARCH ANALYZER simple BM25;
DEFINE INDEX IF NOT EXISTS idx_chunks_path ON code_chunks FIELDS file_path;
DEFINE INDEX IF NOT EXISTS idx_chunks_project ON code_chunks FIELDS project_id;

-- Index Status
DEFINE TABLE IF NOT EXISTS index_status SCHEMAFULL;
DEFINE FIELD project_id       ON index_status TYPE string;
DEFINE FIELD status           ON index_status TYPE string;
DEFINE FIELD total_files      ON index_status TYPE int DEFAULT 0;
DEFINE FIELD indexed_files    ON index_status TYPE int DEFAULT 0;
DEFINE FIELD total_chunks     ON index_status TYPE int DEFAULT 0;
DEFINE FIELD started_at       ON index_status TYPE datetime;
DEFINE FIELD completed_at     ON index_status TYPE option<datetime>;
```

## Connection Options

```rust
pub struct StorageConfig {
    pub data_dir: PathBuf,
    pub namespace: String,
    pub database: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("memory-mcp"),
            namespace: "memory".to_string(),
            database: "main".to_string(),
        }
    }
}
```
