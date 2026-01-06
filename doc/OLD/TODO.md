# Memory MCP Server — Implementation Plan

> **Мета**: Самодостатній MCP сервер пам'яті для AI-агентів в одному Rust бінарнику.

---

## Design Decisions (Resolved)

| Питання | Рішення |
|---------|---------|
| Database | SurrealDB (embedded, kv-surrealkv) |
| Embeddings | Candle (pure Rust) — NOT ONNX |
| Default model | `e5_multi` (768 dims, multilingual) |
| MCP crate | `rmcp` з tool_router pattern |
| Graph algorithms | `petgraph` для PPR |
| Leiden algorithm | `fa-leiden-cd` — lightweight |
| PPR damping factor | 0.5 (HippoRAG style) |
| Hybrid scoring weights | α=0.40 (vec), β=0.15 (bm25), γ=0.45 (ppr) |
| Code parsing | `tree-sitter` + `code-splitter` |
| File watching | `notify` crate |

---

## Implementation Status

### Phase 1: Foundation ✅
- [x] Cargo.toml з SurrealDB + rmcp
- [x] SurrealDB embedded connection
- [x] Schema creation
- [x] Basic MCP server (stdio)
- [x] Tools: `store_memory`, `get_memory`, `delete_memory`, `update_memory`, `list_memories`

### Phase 2: Embeddings ✅
- [x] Candle integration (pure Rust)
- [x] Model download & caching (`~/.cache/huggingface/`)
- [x] 4 models: e5_small, e5_multi, nomic, bge_m3
- [x] Embedding on insert
- [x] `search` tool (vector similarity)
- [x] LRU cache for embeddings

### Phase 3: Hybrid Search ✅
- [x] FTS index (BM25)
- [x] `search_text` tool
- [x] `recall` tool (hybrid scoring)
- [x] RRF merge algorithm
- [x] PPR implementation on petgraph

### Phase 4: Knowledge Graph ✅
- [x] Entity/Relation CRUD
- [x] `create_entity`, `create_relation`
- [x] `get_related` with depth
- [x] Graph traversal via SurrealDB

### Phase 5: Temporal ✅
- [x] Bi-temporal fields (valid_from, valid_until)
- [x] `get_valid`, `get_valid_at`
- [x] `invalidate` tool

### Phase 6: Code Search ✅
- [x] `index_project` — scan and index codebase
- [x] `search_code` — semantic code search
- [x] `get_index_status` — check indexing progress
- [x] `list_projects` — list indexed projects
- [x] `delete_project` — remove project index
- [x] Tree-sitter AST-based chunking
- [x] Namespace isolation per project

### Phase 7: Polish (In Progress)
- [x] Docker image (Dockerfile + Dockerfile.local)
- [x] Background model loading (non-blocking startup)
- [x] Build optimization (lto=thin, faster incremental)
- [ ] NPM wrapper
- [ ] Release binaries
- [ ] Community detection (Leiden)
- [ ] File watcher for auto-reindex

---

## MCP Tools (20 total)

### Memory Operations
| Tool | Parameters | Description |
|------|------------|-------------|
| `store_memory` | content, memory_type?, metadata? | Store with auto-embedding |
| `get_memory` | id | Get by ID |
| `update_memory` | id, content?, metadata? | Update memory |
| `delete_memory` | id | Delete memory |
| `list_memories` | limit?, offset? | Paginated list |

### Search & Retrieval
| Tool | Parameters | Description |
|------|------------|-------------|
| `search` | query, limit? | Vector similarity search |
| `search_text` | query, limit? | BM25 full-text search |
| `recall` | query, limit? | Hybrid (vector + BM25 + PPR) |

### Knowledge Graph
| Tool | Parameters | Description |
|------|------------|-------------|
| `create_entity` | name, entity_type, description? | Create entity node |
| `create_relation` | from_id, to_id, relation_type | Create directed edge |
| `get_related` | entity_id, depth?, direction? | Traverse graph |

### Temporal
| Tool | Parameters | Description |
|------|------------|-------------|
| `get_valid` | — | Currently valid memories |
| `get_valid_at` | timestamp | Valid at point in time |
| `invalidate` | id, reason? | Soft-delete with reason |

### Code Search
| Tool | Parameters | Description |
|------|------------|-------------|
| `index_project` | path, watch? | Index codebase |
| `search_code` | query, project_id?, limit? | Semantic code search |
| `get_index_status` | project_id | Check indexing status |
| `list_projects` | — | List indexed projects |
| `delete_project` | project_id | Remove project index |

### System
| Tool | Parameters | Description |
|------|------------|-------------|
| `get_status` | — | Health check |

---

## Tech Stack

| Component | Technology | Size |
|-----------|------------|------|
| Language | Rust | - |
| Database | SurrealDB (embedded, kv-surrealkv) | ~20 MB |
| Vector search | SurrealDB HNSW | included |
| Full-text | SurrealDB BM25 | included |
| Graph | SurrealDB native + petgraph | included |
| Embeddings | Candle (pure Rust) | ~5 MB |
| Models | HuggingFace Hub | 134 MB - 2.3 GB |
| MCP Protocol | rmcp crate | ~100 KB |
| Code parsing | tree-sitter + code-splitter | ~2 MB |
| File watching | notify | ~100 KB |

---

## Embedding Models

| ID | Model | Size | Dims | Description |
|----|-------|------|------|-------------|
| `e5_small` | intfloat/multilingual-e5-small | ~134 MB | 384 | Fast, English-focused |
| `e5_multi` | intfloat/multilingual-e5-base | ~1.1 GB | 768 | **Default** — multilingual |
| `nomic` | nomic-ai/nomic-embed-text-v1.5 | ~1.9 GB | 768 | MoE, Apache 2.0 |
| `bge_m3` | BAAI/bge-m3 | ~2.3 GB | 1024 | Long context (8K) |

---

## Configuration

| Environment Variable | CLI Flag | Default | Description |
|---------------------|----------|---------|-------------|
| `MEMORY_MCP_DATA_DIR` | `--data-dir` | `~/.local/share/memory-mcp` | Data directory |
| `MEMORY_MCP_MODEL` | `--model` | `e5_multi` | Embedding model |
| `MEMORY_MCP_CACHE_SIZE` | `--cache-size` | `1000` | Embedding cache entries |
| `MEMORY_MCP_BATCH_SIZE` | `--batch-size` | `32` | Max batch size |
| `MEMORY_MCP_TIMEOUT` | `--timeout` | `60` | Model load timeout (seconds) |
| `MEMORY_MCP_LOG_LEVEL` | `--log-level` | `info` | Log level |

> **Note**: Embedding model loads in background on startup. Server responds immediately.

---

## Data Directory Structure

```
# Per-project database
~/.local/share/memory-mcp/
└── db/                    # SurrealDB data files

# Shared models cache
~/.cache/huggingface/      # HuggingFace model cache
└── hub/
    └── models--intfloat--multilingual-e5-base/
```

---

## Hybrid Retrieval Algorithm (V8)

```
┌─────────────────────────────────────────────────────────────┐
│              UNIFIED HYBRID RETRIEVAL                        │
├─────────────────────────────────────────────────────────────┤
│  1. SEED RETRIEVAL                                           │
│     a) Vector HNSW → top-50                                  │
│     b) BM25 FTS → top-50                                     │
│     c) RRF Merge (k=60) → top-20 seeds                       │
├─────────────────────────────────────────────────────────────┤
│  2. HUB DAMPENING                                            │
│     weight = score / sqrt(degree)                            │
├─────────────────────────────────────────────────────────────┤
│  3. PPR DIFFUSION (petgraph)                                 │
│     damping α = 0.5, max_iter = 15                           │
├─────────────────────────────────────────────────────────────┤
│  4. FINAL SCORING                                            │
│     score = 0.40×vec + 0.15×bm25 + 0.45×ppr                  │
└─────────────────────────────────────────────────────────────┘
```

---

## SurrealDB Schema

```sql
-- Memories
DEFINE TABLE memories SCHEMAFULL;
DEFINE FIELD content          ON memories TYPE string;
DEFINE FIELD embedding        ON memories TYPE option<array<float>>;
DEFINE FIELD memory_type      ON memories TYPE string;
DEFINE FIELD user_id          ON memories TYPE option<string>;
DEFINE FIELD metadata         ON memories TYPE option<object>;
DEFINE FIELD created_at       ON memories TYPE datetime DEFAULT time::now();
DEFINE FIELD valid_from       ON memories TYPE datetime DEFAULT time::now();
DEFINE FIELD valid_until      ON memories TYPE option<datetime>;

DEFINE INDEX idx_memories_vec ON memories FIELDS embedding HNSW DIMENSION 768 DIST COSINE;
DEFINE INDEX idx_memories_fts ON memories FIELDS content SEARCH ANALYZER simple BM25;

-- Entities
DEFINE TABLE entities SCHEMAFULL;
DEFINE FIELD name             ON entities TYPE string;
DEFINE FIELD entity_type      ON entities TYPE string;
DEFINE FIELD description      ON entities TYPE option<string>;
DEFINE FIELD embedding        ON entities TYPE option<array<float>>;
DEFINE FIELD created_at       ON entities TYPE datetime DEFAULT time::now();

DEFINE INDEX idx_entities_vec ON entities FIELDS embedding HNSW DIMENSION 768 DIST COSINE;
DEFINE INDEX idx_entities_fts ON entities FIELDS name SEARCH ANALYZER simple BM25;

-- Relations (graph edges)
DEFINE TABLE relations TYPE RELATION IN entities OUT entities SCHEMAFULL;
DEFINE FIELD relation_type    ON relations TYPE string;
DEFINE FIELD weight           ON relations TYPE float DEFAULT 1.0;
DEFINE FIELD valid_from       ON relations TYPE datetime DEFAULT time::now();
DEFINE FIELD valid_until      ON relations TYPE option<datetime>;

-- Code Chunks
DEFINE TABLE code_chunks SCHEMAFULL;
DEFINE FIELD file_path        ON code_chunks TYPE string;
DEFINE FIELD content          ON code_chunks TYPE string;
DEFINE FIELD language         ON code_chunks TYPE string;
DEFINE FIELD start_line       ON code_chunks TYPE int;
DEFINE FIELD end_line         ON code_chunks TYPE int;
DEFINE FIELD chunk_type       ON code_chunks TYPE string;
DEFINE FIELD name             ON code_chunks TYPE option<string>;
DEFINE FIELD embedding        ON code_chunks TYPE option<array<float>>;
DEFINE FIELD content_hash     ON code_chunks TYPE string;
DEFINE FIELD project_id       ON code_chunks TYPE option<string>;
DEFINE FIELD indexed_at       ON code_chunks TYPE datetime DEFAULT time::now();

DEFINE INDEX idx_chunks_vec ON code_chunks FIELDS embedding HNSW DIMENSION 768 DIST COSINE;
DEFINE INDEX idx_chunks_fts ON code_chunks FIELDS content SEARCH ANALYZER simple BM25;
DEFINE INDEX idx_chunks_path ON code_chunks FIELDS file_path;
DEFINE INDEX idx_chunks_project ON code_chunks FIELDS project_id;

-- Index Status
DEFINE TABLE index_status SCHEMAFULL;
DEFINE FIELD project_id       ON index_status TYPE string;
DEFINE FIELD status           ON index_status TYPE string;
DEFINE FIELD total_files      ON index_status TYPE int;
DEFINE FIELD indexed_files    ON index_status TYPE int;
DEFINE FIELD total_chunks     ON index_status TYPE int;
DEFINE FIELD started_at       ON index_status TYPE datetime;
DEFINE FIELD completed_at     ON index_status TYPE option<datetime>;
```

---

## Project Structure

```
memory-mcp/
├── src/
│   ├── lib.rs              # Public exports
│   ├── main.rs             # Entry point, CLI
│   ├── types/
│   │   ├── mod.rs
│   │   ├── memory.rs       # Memory, MemoryType
│   │   ├── entity.rs       # Entity, Relation
│   │   ├── error.rs        # AppError
│   │   └── code.rs         # CodeChunk, Language
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── traits.rs       # StorageBackend trait
│   │   └── surrealdb.rs    # SurrealDB implementation
│   ├── embedding/
│   │   ├── mod.rs
│   │   ├── config.rs       # Model configs
│   │   ├── engine.rs       # Candle inference
│   │   ├── cache.rs        # LRU cache
│   │   └── service.rs      # Async wrapper
│   ├── graph/
│   │   ├── mod.rs
│   │   ├── ppr.rs          # Personalized PageRank
│   │   └── rrf.rs          # Reciprocal Rank Fusion
│   ├── codebase/
│   │   ├── mod.rs
│   │   ├── scanner.rs      # File scanning
│   │   ├── chunker.rs      # Tree-sitter chunking
│   │   └── indexer.rs      # Batch indexing
│   └── server/
│       ├── mod.rs
│       └── handler.rs      # MCP tools (20)
├── doc/
│   ├── README.md
│   ├── ARCHITECTURE.md
│   └── design/
│       ├── TODO.md         # This file
│       └── code-search.md
├── Cargo.toml
└── README.md
```

---

## Cargo.toml Dependencies

```toml
[package]
name = "memory-mcp"
version = "0.1.0"
edition = "2021"

[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# Database
surrealdb = { version = "2", default-features = false, features = ["kv-surrealkv"] }

# Embeddings (pure Rust)
candle-core = "0.8"
candle-nn = "0.8"
candle-transformers = "0.8"
hf-hub = "0.3"
tokenizers = "0.21"

# MCP Protocol
rmcp = { version = "0.1", features = ["server", "macros", "transport-io"] }

# Graph
petgraph = "0.6"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# CLI
clap = { version = "4", features = ["derive", "env"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Error handling
anyhow = "1"
thiserror = "2"

# Utils
chrono = { version = "0.4", features = ["serde"] }

# Code parsing
tree-sitter = "0.26"
code-splitter = "0.4"

# File operations
walkdir = "2"
ignore = "0.4"
notify = "8"
blake3 = "1"

# Parallelism
rayon = "1.10"
```

---

## IDE Integrations

### Claude Desktop
```json
{
  "mcpServers": {
    "memory": {
      "command": "/path/to/memory-mcp",
      "env": {
        "MEMORY_MCP_MODEL": "e5_multi"
      }
    }
  }
}
```

### Cursor IDE
```json
{
  "name": "memory",
  "command": "/path/to/memory-mcp",
  "args": ["--data-dir", "./memory"]
}
```

### OpenCode
```json
{
  "mcp": {
    "memory": {
      "type": "stdio",
      "command": "memory-mcp"
    }
  }
}
```

---

## Comparison

| Feature | memory-mcp | Mem0 | Zep | ChromaDB |
|---------|-----------|------|-----|----------|
| Single binary | ✅ | ❌ | ❌ | ❌ |
| Built-in embeddings | ✅ Candle | ❌ API | ❌ API | ❌ API |
| Works offline | ✅ | ❌ | ❌ | ⚠️ |
| Vector search | ✅ HNSW | ✅ | ✅ | ✅ |
| Knowledge graph | ✅ native | ⚠️ | ✅ Neo4j | ❌ |
| Temporal validity | ✅ | ❌ | ✅ | ❌ |
| Code search | ✅ | ❌ | ❌ | ❌ |
| Multilingual | ✅ 100+ | ⚠️ | ⚠️ | ⚠️ |

---

*Last updated: 2026-01-06*
