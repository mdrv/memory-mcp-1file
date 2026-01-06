# Memory MCP Server — Task Board

> Jira-style project tracking for memory-mcp development

---

## Progress Overview

| Epic | Name | Status | Sessions | Priority |
|------|------|--------|----------|----------|
| [Epic 1](#epic-1-project-setup) | Project Setup | `[x]` **Done** | 1 | P0 |
| [Epic 2](#epic-2-storage-layer) | Storage Layer | `[x]` **Done** | 1 | P0 |
| [Epic 3](#epic-3-mcp-server) | MCP Server | `[x]` **Done** | 1-2 | P0 |
| [Epic 4](#epic-4-embedding-engine) | Embedding Engine | `[x]` **Done** | 2 | P1 |
| [Epic 5](#epic-5-vector-search) | Vector Search | `[x]` **Done** | 2-3 | P1 |
| [Epic 6](#epic-6-full-text-search) | Full-Text Search | `[x]` **Done** | 4 | P2 |
| [Epic 7](#epic-7-hybrid-retrieval) | Hybrid Retrieval | `[x]` **Done** | 4-5 | P2 |
| [Epic 8](#epic-8-knowledge-graph) | Knowledge Graph | `[x]` **Done** | 5 | P2 |
| [Epic 9](#epic-9-temporal-engine) | Temporal Engine | `[x]` **Done** | 6 | P3 |
| [Epic 10](#epic-10-code-search) | Code Search | `[x]` **Done** | 7+ | P3 |
| [Epic 11](#epic-11-testing--polish) | Testing & Polish | `[ ]` Pending | — | P3 |

**Total MCP Tools**: 20  
**Total Tests**: 84+ (47 unit + 37 integration)

---

## Session Log

### Session 1: 2026-01-02
**Epic**: Epic 1-2 — Project Setup + Storage Layer  
**Completed**:
- Created Cargo.toml with SurrealDB + rmcp dependencies
- Implemented SurrealStorage with embedded kv-surrealkv
- Created schema with memories, entities, relations tables
- Basic CRUD operations for memories
- Unit tests for storage layer

**Key Decisions**:
- Use `surrealdb::sql::Thing` for record IDs
- Use `surrealdb::sql::Datetime` instead of `chrono::DateTime`
- Schema uses `DEFINE TABLE OVERWRITE` for idempotency
- All queries use `.check()` for fail-fast error handling

**Files Created**:
- `Cargo.toml`
- `src/lib.rs`, `src/main.rs`
- `src/types/mod.rs`, `memory.rs`, `entity.rs`, `error.rs`
- `src/storage/mod.rs`, `traits.rs`, `surrealdb.rs`
- `tests/storage_test.rs`

**Blockers**: None  
**Next**: Epic 3 — MCP Server

---

### Session 2: 2026-01-02
**Epic**: Epic 3-4 — MCP Server + Embedding Engine  
**Completed**:
- Implemented MemoryMcpServer with rmcp tool_router pattern
- Added 5 memory tools: store, get, update, delete, list
- Created EmbeddingService with Candle (pure Rust)
- Model download via hf-hub
- LRU cache for embeddings
- Background model loading (non-blocking startup)

**Key Decisions**:
- Use `#[tool_router]` + `#[tool_handler]` pattern from rmcp
- `Parameters<T>` wrapper for tool arguments
- Embedding never serialized: `#[serde(skip_serializing)]`
- Default model: e5_multi (768 dims, multilingual)

**Files Created**:
- `src/server/mod.rs`, `handler.rs`
- `src/embedding/mod.rs`, `config.rs`, `engine.rs`, `cache.rs`, `service.rs`
- `tests/handler_test.rs`

**Blockers**: None  
**Next**: Epic 5 — Vector Search

---

### Session 3: 2026-01-03
**Epic**: Epic 5 — Vector Search  
**Completed**:
- Added `search` MCP tool (vector similarity)
- Implemented HNSW index in SurrealDB schema
- Added search with temporal filter (valid_from/valid_until)
- Total MCP tools: 6

**Key Decisions**:
- HNSW DIMENSION 768 DIST COSINE for vector index
- Temporal filter: `WHERE valid_from <= $now AND (valid_until IS NONE OR valid_until > $now)`

**Files Modified**:
- `src/storage/surrealdb.rs` — vector_search implementation
- `src/server/handler.rs` — search tool

**Blockers**: None  
**Next**: Epic 6 — Full-Text Search

---

### Session 4: 2026-01-03
**Epic**: Epic 6-7 — Full-Text Search + Hybrid Retrieval  
**Completed**:
- Added `search_text` tool (BM25)
- Added `recall` tool (hybrid: vector + BM25 + PPR)
- Implemented RRF merge algorithm
- Implemented PPR with petgraph
- Total MCP tools: 8

**Key Decisions**:
- RRF k=60 for merge
- Scoring weights: α=0.40 (vec), β=0.15 (bm25), γ=0.45 (ppr)
- PPR damping=0.5 (HippoRAG style)

**Files Created**:
- `src/graph/mod.rs`, `ppr.rs`, `rrf.rs`
- `tests/recall_test.rs`, `tests/bm25_test.rs`

**Blockers**: None  
**Next**: Epic 8 — Knowledge Graph

---

### Session 5: 2026-01-04
**Epic**: Epic 8 — Knowledge Graph  
**Completed**:
- Added `create_entity`, `create_relation`, `get_related` tools
- Implemented graph traversal via SurrealDB RELATE
- Added `get_status` system tool
- Total MCP tools: 12

**Key Decisions**:
- Relations as TYPE RELATION IN entities OUT entities
- Depth parameter with `->relations.{1..$depth}->entities` syntax
- Direction enum: Outgoing, Incoming, Both

**Files Modified**:
- `src/storage/traits.rs` — graph methods
- `src/storage/surrealdb.rs` — graph implementation
- `src/server/handler.rs` — graph tools

**Files Created**:
- `tests/graph_test.rs`

**Blockers**: None  
**Next**: Epic 9 — Temporal Engine

---

### Session 6: 2026-01-05
**Epic**: Epic 9 — Temporal Engine  
**Completed**: 
- Extended StorageBackend trait with 3 temporal methods
- Implemented `get_valid_memories()`, `get_valid_at()`, `invalidate_memory()` in SurrealStorage
- Added 3 MCP tools: `get_valid`, `get_valid_at`, `invalidate`
- Created `tests/temporal_test.rs` with 3 integration tests
- Fixed SurrealDB query syntax (use `type::thing()` for dynamic record IDs)
- All 84 tests passing (47 unit + 37 integration)
- Total MCP tools: 15

**Key Decisions**:
- Schema already had `valid_from`/`valid_until` fields — no schema changes needed
- Timestamp parsing: `chrono::DateTime::parse_from_rfc3339()` → `surrealdb::sql::Datetime`
- `invalidate_memory` uses `type::thing('memories', $id)` syntax for dynamic ID binding
- Temporal filter: `WHERE valid_from <= $at AND (valid_until IS NONE OR valid_until > $at)`

**Files Modified**:
- `src/storage/traits.rs` — +3 trait methods
- `src/storage/surrealdb.rs` — +3 implementations
- `src/server/handler.rs` — +3 tools, +3 Args structs, updated ServerInfo

**Files Created**:
- `tests/temporal_test.rs` — 3 temporal tests

**Blockers**: None  
**Next**: Epic 10 — Code Search

---

### Session 7+: 2026-01-05+
**Epic**: Epic 10 — Code Search  
**Completed**:
- Added 5 code search tools: `index_project`, `search_code`, `get_index_status`, `list_projects`, `delete_project`
- Implemented tree-sitter AST-based chunking via code-splitter
- Added project namespace isolation
- Z-score normalization for unified recall
- Docker images (Dockerfile + Dockerfile.local)
- Background model loading optimization
- Total MCP tools: 20

**Key Decisions**:
- Code chunks stored in `code_chunks` table with project_id
- Hierarchical context injection for better search
- Adaptive batch sizing (by tokens, not count)
- File watcher with 500ms debouncing

**Files Created**:
- `src/codebase/mod.rs`, `scanner.rs`, `chunker.rs`, `indexer.rs`, `progress.rs`, `watcher.rs`
- `src/project/mod.rs`, `namespace.rs`
- `src/types/code.rs`, `search.rs`
- `Dockerfile`, `Dockerfile.local`
- `tests/codebase_test.rs`

**Blockers**: None  
**Next**: Epic 11 — Testing & Polish

---

## Epic Details

### Epic 1: Project Setup
**Goal**: Create project skeleton with dependencies

| # | Task | Status |
|---|------|--------|
| 1.1 | Create Cargo.toml | `[x]` |
| 1.2 | Setup module structure | `[x]` |
| 1.3 | Define types (Memory, Entity, Error) | `[x]` |

---

### Epic 2: Storage Layer
**Goal**: Embedded SurrealDB with CRUD

| # | Task | Status |
|---|------|--------|
| 2.1 | SurrealDB connection (kv-surrealkv) | `[x]` |
| 2.2 | Schema creation | `[x]` |
| 2.3 | Memory CRUD operations | `[x]` |
| 2.4 | Unit tests | `[x]` |

---

### Epic 3: MCP Server
**Goal**: Working MCP server with basic tools

| # | Task | Status |
|---|------|--------|
| 3.1 | rmcp integration with tool_router | `[x]` |
| 3.2 | 5 memory tools | `[x]` |
| 3.3 | CLI with clap | `[x]` |
| 3.4 | Handler tests | `[x]` |

---

### Epic 4: Embedding Engine
**Goal**: Pure Rust embeddings with Candle

| # | Task | Status |
|---|------|--------|
| 4.1 | Candle integration | `[x]` |
| 4.2 | Model download (hf-hub) | `[x]` |
| 4.3 | LRU cache | `[x]` |
| 4.4 | Background loading | `[x]` |
| 4.5 | 4 model configs | `[x]` |

---

### Epic 5: Vector Search
**Goal**: Semantic similarity search

| # | Task | Status |
|---|------|--------|
| 5.1 | HNSW index in schema | `[x]` |
| 5.2 | vector_search implementation | `[x]` |
| 5.3 | `search` MCP tool | `[x]` |
| 5.4 | Temporal filter | `[x]` |

---

### Epic 6: Full-Text Search
**Goal**: BM25 keyword search

| # | Task | Status |
|---|------|--------|
| 6.1 | BM25 index in schema | `[x]` |
| 6.2 | bm25_search implementation | `[x]` |
| 6.3 | `search_text` MCP tool | `[x]` |
| 6.4 | BM25 tests | `[x]` |

---

### Epic 7: Hybrid Retrieval
**Goal**: RRF + PPR fusion

| # | Task | Status |
|---|------|--------|
| 7.1 | RRF merge algorithm | `[x]` |
| 7.2 | Hub dampening | `[x]` |
| 7.3 | PPR with petgraph | `[x]` |
| 7.4 | `recall` MCP tool | `[x]` |
| 7.5 | Integration tests | `[x]` |

---

### Epic 8: Knowledge Graph
**Goal**: Entity/Relation management

| # | Task | Status |
|---|------|--------|
| 8.1 | Entity CRUD | `[x]` |
| 8.2 | Relation CRUD | `[x]` |
| 8.3 | Graph traversal | `[x]` |
| 8.4 | `create_entity`, `create_relation`, `get_related` tools | `[x]` |
| 8.5 | `get_status` tool | `[x]` |
| 8.6 | Graph tests | `[x]` |

---

### Epic 9: Temporal Engine
**Goal**: Bi-temporal queries + invalidation

| # | Task | Status |
|---|------|--------|
| 9.1 | Extend StorageBackend trait (+3 methods) | `[x]` |
| 9.2 | Implement temporal methods in SurrealStorage | `[x]` |
| 9.3 | Add 3 MCP tools (get_valid, get_valid_at, invalidate) | `[x]` |
| 9.4 | Write integration tests | `[x]` |
| 9.5 | Validation (build, clippy, test) | `[x]` |

**Validation Checkpoint**:
- [x] Temporal filter correct
- [x] Invalidation sets valid_until
- [x] get_valid_at returns correct snapshot
- [x] All 84 tests passing

---

### Epic 10: Code Search
**Goal**: Semantic code search with AST chunking

| # | Task | Status |
|---|------|--------|
| 10.1 | Project detection (Git root) | `[x]` |
| 10.2 | File scanner (walkdir + ignore) | `[x]` |
| 10.3 | Code chunker (tree-sitter) | `[x]` |
| 10.4 | Indexer with adaptive batching | `[x]` |
| 10.5 | 5 MCP tools (index, search, status, list, delete) | `[x]` |
| 10.6 | Docker images | `[x]` |
| 10.7 | Integration tests | `[x]` |

---

### Epic 11: Testing & Polish
**Goal**: Final polish and documentation

| # | Task | Status |
|---|------|--------|
| 11.1 | Missing unit tests | `[ ]` |
| 11.2 | README.md in project root | `[x]` |
| 11.3 | NPM wrapper | `[ ]` |
| 11.4 | Release binaries | `[ ]` |
| 11.5 | Community detection (Leiden) | `[ ]` |
| 11.6 | File watcher for auto-reindex | `[ ]` |

---

## Delegation Rules

| Task Type | Assign To | Examples |
|-----------|-----------|----------|
| CRUD operations | Junior | Memory/Entity CRUD, simple queries |
| MCP tool additions | Junior | Adding new tools with existing patterns |
| Algorithm implementation | Lead | PPR, RRF, embeddings |
| Schema changes | Lead | Database schema modifications |
| Architecture decisions | Lead + Oracle | Major design changes |
| Bug fixes (simple) | Junior | Syntax errors, missing fields |
| Bug fixes (complex) | Lead | Race conditions, edge cases |

---

## Quality Gates

Before marking Epic as Done:
- [ ] `cargo check` passes
- [ ] `cargo clippy` clean (no warnings)
- [ ] `cargo test` all passing
- [ ] New tests for new functionality
- [ ] Documentation updated if needed

---

## Technical Decisions Log

### SurrealDB Patterns
- Use `type::thing('table', $id)` for dynamic record IDs (NOT `table:$id`)
- Use `DEFINE TABLE OVERWRITE` for idempotent schema
- All queries use `.check()` for fail-fast errors
- Timestamps: `surrealdb::sql::Datetime` (NOT chrono)

### Embedding Patterns
- Never serialize embeddings: `#[serde(skip_serializing)]`
- Background loading: server starts immediately
- LRU cache with blake3 key: `hash(text || model_id)`

### MCP Patterns
- `Parameters<T>` wrapper for tool arguments
- Two-tier errors: `Ok(CallToolResult::error(...))` for tool errors, `Err(McpError)` for protocol errors
- Hard limits: max 50/100 results per query

---

*Last updated: 2026-01-06*
