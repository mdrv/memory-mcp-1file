# Data Model: Memory MCP Server

**Feature**: 001-memory-mcp-server
**Date**: 2026-01-06
**Source**: spec.md FR-001 to FR-040, doc/OLD/storage-backend.md

---

## Entity Relationship Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         ENTITIES                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐         ┌──────────────┐                      │
│  │   Memory     │         │   Entity     │                      │
│  ├──────────────┤         ├──────────────┤                      │
│  │ id           │         │ id           │                      │
│  │ content      │         │ name         │                      │
│  │ embedding    │         │ entity_type  │                      │
│  │ memory_type  │         │ description  │                      │
│  │ user_id      │         │ embedding    │                      │
│  │ metadata     │         │ created_at   │                      │
│  │ event_time   │         └──────────────┘                      │
│  │ ingestion_time         │              │                      │
│  │ valid_from   │         │              │                      │
│  │ valid_until  │    ┌────┴────┐         │                      │
│  │ importance   │    │ Relation │◄────────┘                      │
│  └──────────────┘    ├──────────┤                                │
│                      │ from     │ (Entity)                       │
│                      │ to       │ (Entity)                       │
│  ┌──────────────┐    │ type     │                                │
│  │  CodeChunk   │    │ weight   │                                │
│  ├──────────────┤    │ valid_*  │                                │
│  │ id           │    └──────────┘                                │
│  │ file_path    │                                                │
│  │ content      │    ┌──────────────┐                            │
│  │ language     │    │ IndexStatus  │                            │
│  │ start_line   │    ├──────────────┤                            │
│  │ end_line     │    │ project_id   │                            │
│  │ chunk_type   │    │ status       │                            │
│  │ name         │    │ total_files  │                            │
│  │ embedding    │    │ indexed_files│                            │
│  │ content_hash │    │ total_chunks │                            │
│  │ project_id   │    │ started_at   │                            │
│  │ indexed_at   │    │ completed_at │                            │
│  └──────────────┘    └──────────────┘                            │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

---

## 1. Memory

Core entity for storing agent memories with semantic embeddings.

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `id` | Thing | Auto | - | SurrealDB record ID (table:id format) |
| `content` | String | **Yes** | - | Memory content text |
| `embedding` | Option<Vec<f32>> | No | None | Vector embedding (768d for e5_multi) |
| `memory_type` | String | No | "semantic" | Type: episodic, semantic, procedural |
| `user_id` | Option<String> | No | None | Multi-tenant isolation key |
| `metadata` | Option<Object> | No | None | Arbitrary JSON metadata |
| `event_time` | Datetime | No | now() | When the event occurred |
| `ingestion_time` | Datetime | No | now() | When stored in database |
| `valid_from` | Datetime | No | now() | Validity start (bi-temporal) |
| `valid_until` | Option<Datetime> | No | None | Validity end (soft delete) |
| `importance_score` | f32 | No | 1.0 | Importance weight |
| `invalidation_reason` | Option<String> | No | None | Reason for invalidation |

### Rust Type

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,
    
    pub content: String,
    
    #[serde(skip_serializing)]  // Token protection
    pub embedding: Option<Vec<f32>>,
    
    #[serde(default = "default_memory_type")]
    pub memory_type: MemoryType,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    
    #[serde(default = "Utc::now")]
    pub event_time: DateTime<Utc>,
    
    #[serde(default = "Utc::now")]
    pub ingestion_time: DateTime<Utc>,
    
    #[serde(default = "Utc::now")]
    pub valid_from: DateTime<Utc>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<DateTime<Utc>>,
    
    #[serde(default = "default_importance")]
    pub importance_score: f32,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalidation_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MemoryType {
    Episodic,   // Events, experiences
    #[default]
    Semantic,   // Facts, knowledge
    Procedural, // How-to, skills
}
```

### Indexes

| Name | Fields | Type | Notes |
|------|--------|------|-------|
| `idx_memories_vec` | embedding | HNSW | DIMENSION 768, DIST COSINE |
| `idx_memories_fts` | content | SEARCH | BM25, ANALYZER simple |

### Validation Rules

- `content`: Non-empty, max 100KB
- `memory_type`: One of: episodic, semantic, procedural
- `importance_score`: 0.0 to 10.0
- `embedding`: Must match model dimension (384/768/1024)

---

## 2. Entity

Knowledge graph node representing a named concept.

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `id` | Thing | Auto | - | SurrealDB record ID |
| `name` | String | **Yes** | - | Entity name |
| `entity_type` | String | No | "unknown" | Type: person, project, concept, file, etc. |
| `description` | Option<String> | No | None | Optional description |
| `embedding` | Option<Vec<f32>> | No | None | Embedding of name |
| `user_id` | Option<String> | No | None | Multi-tenant isolation |
| `created_at` | Datetime | No | now() | Creation timestamp |

### Rust Type

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,
    
    pub name: String,
    
    #[serde(default)]
    pub entity_type: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    #[serde(skip_serializing)]
    pub embedding: Option<Vec<f32>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
}
```

### Indexes

| Name | Fields | Type | Notes |
|------|--------|------|-------|
| `idx_entities_vec` | embedding | HNSW | DIMENSION 768, DIST COSINE |
| `idx_entities_fts` | name | SEARCH | BM25, ANALYZER simple |

---

## 3. Relation

Directed edge between entities in the knowledge graph.

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `id` | Thing | Auto | - | SurrealDB relation ID |
| `in` | Thing | **Yes** | - | Source entity (from) |
| `out` | Thing | **Yes** | - | Target entity (to) |
| `relation_type` | String | **Yes** | - | Type: works_on, knows, uses, etc. |
| `weight` | f32 | No | 1.0 | Edge weight (0.0-1.0) |
| `valid_from` | Datetime | No | now() | Validity start |
| `valid_until` | Option<Datetime> | No | None | Validity end |

### Rust Type

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,
    
    #[serde(rename = "in")]
    pub from_entity: Thing,
    
    #[serde(rename = "out")]
    pub to_entity: Thing,
    
    pub relation_type: String,
    
    #[serde(default = "default_weight")]
    pub weight: f32,
    
    #[serde(default = "Utc::now")]
    pub valid_from: DateTime<Utc>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    #[default]
    Outgoing,
    Incoming,
    Both,
}
```

### SurrealDB Definition

```sql
DEFINE TABLE relations TYPE RELATION IN entities OUT entities SCHEMAFULL;
```

---

## 4. CodeChunk

Indexed code fragment for semantic code search.

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `id` | Thing | Auto | - | SurrealDB record ID |
| `file_path` | String | **Yes** | - | Relative path from project root |
| `content` | String | **Yes** | - | Code content (with context) |
| `language` | String | No | "unknown" | Programming language |
| `start_line` | u32 | **Yes** | - | Start line number (1-based) |
| `end_line` | u32 | **Yes** | - | End line number (1-based) |
| `chunk_type` | String | No | "other" | Type: function, class, struct, module, impl |
| `name` | Option<String> | No | None | Name of function/class if applicable |
| `embedding` | Option<Vec<f32>> | No | None | Vector embedding |
| `content_hash` | String | **Yes** | - | blake3 hash for deduplication |
| `project_id` | Option<String> | No | None | Project identifier |
| `indexed_at` | Datetime | No | now() | Indexing timestamp |

### Rust Type

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,
    
    pub file_path: String,
    pub content: String,
    
    #[serde(default)]
    pub language: Language,
    
    pub start_line: u32,
    pub end_line: u32,
    
    #[serde(default)]
    pub chunk_type: ChunkType,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    
    #[serde(skip_serializing)]
    pub embedding: Option<Vec<f32>>,
    
    pub content_hash: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    
    #[serde(default = "Utc::now")]
    pub indexed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ChunkType {
    Function,
    Class,
    Struct,
    Module,
    Impl,
    #[default]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    #[default]
    Unknown,
}
```

### Indexes

| Name | Fields | Type | Notes |
|------|--------|------|-------|
| `idx_chunks_vec` | embedding | HNSW | DIMENSION 768, DIST COSINE |
| `idx_chunks_fts` | content | SEARCH | BM25 |
| `idx_chunks_path` | file_path | STANDARD | Fast path lookup |
| `idx_chunks_project` | project_id | STANDARD | Project filtering |
| `idx_chunks_hash` | content_hash | STANDARD | Deduplication |

---

## 5. IndexStatus

Progress tracking for codebase indexing.

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `id` | Thing | Auto | - | SurrealDB record ID |
| `project_id` | String | **Yes** | - | Project identifier |
| `status` | String | **Yes** | - | Status: indexing, completed, failed |
| `total_files` | u32 | No | 0 | Total files to index |
| `indexed_files` | u32 | No | 0 | Files indexed so far |
| `total_chunks` | u32 | No | 0 | Total chunks created |
| `started_at` | Datetime | **Yes** | - | Indexing start time |
| `completed_at` | Option<Datetime> | No | None | Completion time |
| `error_message` | Option<String> | No | None | Error if failed |

### Rust Type

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,
    
    pub project_id: String,
    pub status: IndexState,
    
    #[serde(default)]
    pub total_files: u32,
    
    #[serde(default)]
    pub indexed_files: u32,
    
    #[serde(default)]
    pub total_chunks: u32,
    
    pub started_at: DateTime<Utc>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexState {
    Indexing,
    Completed,
    Failed,
}
```

---

## SurrealDB Schema (schema.surql)

```sql
-- Memories table
DEFINE TABLE IF NOT EXISTS memories SCHEMAFULL;
DEFINE FIELD content          ON memories TYPE string;
DEFINE FIELD embedding        ON memories TYPE option<array<float>>;
DEFINE FIELD memory_type      ON memories TYPE string DEFAULT 'semantic';
DEFINE FIELD user_id          ON memories TYPE option<string>;
DEFINE FIELD metadata         ON memories TYPE option<object>;
DEFINE FIELD event_time       ON memories TYPE datetime DEFAULT time::now();
DEFINE FIELD ingestion_time   ON memories TYPE datetime DEFAULT time::now();
DEFINE FIELD valid_from       ON memories TYPE datetime DEFAULT time::now();
DEFINE FIELD valid_until      ON memories TYPE option<datetime>;
DEFINE FIELD importance_score ON memories TYPE float DEFAULT 1.0;
DEFINE FIELD invalidation_reason ON memories TYPE option<string>;

DEFINE INDEX IF NOT EXISTS idx_memories_vec ON memories 
    FIELDS embedding HNSW DIMENSION 768 DIST COSINE;
DEFINE INDEX IF NOT EXISTS idx_memories_fts ON memories 
    FIELDS content SEARCH ANALYZER simple BM25;

-- Entities table
DEFINE TABLE IF NOT EXISTS entities SCHEMAFULL;
DEFINE FIELD name             ON entities TYPE string;
DEFINE FIELD entity_type      ON entities TYPE string DEFAULT 'unknown';
DEFINE FIELD description      ON entities TYPE option<string>;
DEFINE FIELD embedding        ON entities TYPE option<array<float>>;
DEFINE FIELD user_id          ON entities TYPE option<string>;
DEFINE FIELD created_at       ON entities TYPE datetime DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_entities_vec ON entities 
    FIELDS embedding HNSW DIMENSION 768 DIST COSINE;
DEFINE INDEX IF NOT EXISTS idx_entities_fts ON entities 
    FIELDS name SEARCH ANALYZER simple BM25;

-- Relations table (graph edges)
DEFINE TABLE IF NOT EXISTS relations TYPE RELATION IN entities OUT entities SCHEMAFULL;
DEFINE FIELD relation_type    ON relations TYPE string;
DEFINE FIELD weight           ON relations TYPE float DEFAULT 1.0;
DEFINE FIELD valid_from       ON relations TYPE datetime DEFAULT time::now();
DEFINE FIELD valid_until      ON relations TYPE option<datetime>;

-- Code chunks table
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
DEFINE INDEX IF NOT EXISTS idx_chunks_hash ON code_chunks FIELDS content_hash;

-- Index status table
DEFINE TABLE IF NOT EXISTS index_status SCHEMAFULL;
DEFINE FIELD project_id       ON index_status TYPE string;
DEFINE FIELD status           ON index_status TYPE string;
DEFINE FIELD total_files      ON index_status TYPE int DEFAULT 0;
DEFINE FIELD indexed_files    ON index_status TYPE int DEFAULT 0;
DEFINE FIELD total_chunks     ON index_status TYPE int DEFAULT 0;
DEFINE FIELD started_at       ON index_status TYPE datetime;
DEFINE FIELD completed_at     ON index_status TYPE option<datetime>;
DEFINE FIELD error_message    ON index_status TYPE option<string>;
```

---

## Search Result Types

### SearchResult (for vector/BM25 search)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    pub memory_type: MemoryType,
    pub score: f32,
    pub metadata: Option<serde_json::Value>,
}
```

### RecallResult (for hybrid search)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallResult {
    pub memories: Vec<ScoredMemory>,
    pub query: String,
    pub subgraph_nodes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredMemory {
    pub id: String,
    pub content: String,
    pub memory_type: MemoryType,
    pub score: f32,
    pub vector_score: f32,
    pub bm25_score: f32,
    pub ppr_score: f32,
}
```

### CodeSearchResult

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSearchResult {
    pub results: Vec<ScoredCodeChunk>,
    pub count: usize,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredCodeChunk {
    pub id: String,
    pub file_path: String,
    pub content: String,
    pub language: Language,
    pub start_line: u32,
    pub end_line: u32,
    pub chunk_type: ChunkType,
    pub name: Option<String>,
    pub score: f32,
}
```

---

## State Transitions

### Memory Lifecycle

```
Created (valid_from=now, valid_until=None)
    │
    ├─── update_memory() ──► Updated (same id, new content/embedding)
    │
    ├─── invalidate() ──► Invalidated (valid_until=now)
    │
    └─── delete_memory() ──► Deleted (hard delete)
```

### IndexStatus Lifecycle

```
index_project() called
    │
    ▼
Indexing (status="indexing")
    │
    ├─── Success ──► Completed (status="completed", completed_at=now)
    │
    └─── Failure ──► Failed (status="failed", error_message set)
```
