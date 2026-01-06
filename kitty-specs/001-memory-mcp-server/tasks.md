# Tasks: Memory MCP Server

**Feature**: 001-memory-mcp-server  
**Date**: 2026-01-06  
**Phases**: 10 | **Work Packages**: 10 | **Subtasks**: 79

---

## Overview

This task breakdown follows the Risk-First + Vertical Slices approach from `plan.md`. Phases 2 & 3 can run in parallel. All tasks have been derived from the implementation plan and MCP tools contract.

### Work Package Summary

| ID | Name | Subtasks | Priority | Dependencies |
|----|------|----------|----------|--------------|
| WP01 | PoC Validation | 3 | P0 | None |
| WP02 | Core Foundation | 9 | P0 | WP01 |
| WP03 | Storage Layer | 13 | P1 | WP02 |
| WP04 | Embedding Layer | 7 | P1 | WP02 (parallel with WP03) |
| WP05 | MCP Server + Memory Tools | 9 | P1 | WP03, WP04 |
| WP06 | Search Tools | 4 | P1 | WP05 |
| WP07 | Graph Tools + PPR | 6 | P2 | WP06 |
| WP08 | Temporal Tools | 3 | P2 | WP05 |
| WP09 | Code Search Tools | 8 | P3 | WP05 |
| WP10 | System Tool + Polish | 7 | P3 | WP07, WP08, WP09 |

### Dependency Graph

```
WP01 → WP02 → WP03 ─┐
              ├─────→ WP05 → WP06 → WP07 ─┐
         WP04 ─┘                          ├→ WP10
                     WP05 → WP08 ─────────┤
                     WP05 → WP09 ─────────┘
```

---

## WP01: PoC Validation

**Objective**: Validate highest-risk components before full implementation  
**Priority**: P0 (Gate - must pass before Phase 1)  
**Prompt**: [WP01-poc-validation.md](./tasks/WP01-poc-validation.md)

### Subtasks

- [x] **T001** [P]: Candle PoC - load e5_small model, embed "hello world", return 384-dim vector
- [x] **T002** [P]: rmcp PoC - minimal MCP server with 1 dummy tool, respond to tools/list
- [x] **T003** [P]: SurrealDB PoC - embedded DB, CRUD memory, HNSW index vector search
- [ ] **T003a**: Write tests for WP01 components

### Success Criteria

- All 3 PoC binaries compile and run without errors
- Candle returns 384-dimensional vector
- rmcp responds to `tools/list` JSON-RPC call
- SurrealDB vector search returns results with cosine similarity

### Parallel Opportunities

All 3 subtasks are independent and can be developed in parallel.

---

## WP02: Core Foundation

**Objective**: Project structure + all type definitions  
**Priority**: P0  
**Dependencies**: WP01  
**Prompt**: [WP02-core-foundation.md](./tasks/WP02-core-foundation.md)

### Subtasks

- [x] **T004**: Create Cargo.toml with all dependencies (surrealdb, candle, rmcp, etc.)
- [x] **T005**: Create .cargo/config.toml for linker optimization (mold, lld)
- [x] **T006**: Implement src/types/error.rs (AppError via thiserror, Result alias)
- [x] **T007**: Implement src/types/memory.rs (Memory, MemoryType, MemoryUpdate)
- [x] **T008**: Implement src/types/entity.rs (Entity, Relation, Direction)
- [x] **T009**: Implement src/types/code.rs (CodeChunk, ChunkType, Language, IndexStatus)
- [x] **T010**: Implement src/types/search.rs (SearchResult, RecallResult, ScoredMemory)
- [x] **T011**: Create src/config.rs (AppConfig, AppState)
- [x] **T012**: Create src/lib.rs with pub mod declarations
- [ ] **T012a**: Write tests for WP02 components

### Success Criteria

- `cargo check` passes with no errors
- All types implement Serialize, Deserialize
- `embedding` fields have `#[serde(skip_serializing)]` (token protection)

---

## WP03: Storage Layer

**Objective**: Storage abstraction + SurrealDB implementation  
**Priority**: P1  
**Dependencies**: WP02  
**Prompt**: [WP03-storage-layer.md](./tasks/WP03-storage-layer.md)

### Subtasks

- [x] **T013**: Create src/storage/traits.rs (StorageBackend async trait)
- [x] **T014**: Create src/storage/schema.surql (DDL for all tables and indexes)
- [x] **T015**: Implement src/storage/surrealdb.rs - SurrealStorage::new, connect, apply schema
- [x] **T016**: Implement Memory CRUD: create_memory, get_memory, update_memory, delete_memory
- [x] **T017**: Implement Memory list: list_memories, count_memories (pagination)
- [x] **T018**: Implement Vector search: vector_search, vector_search_code
- [x] **T019**: Implement BM25 search: bm25_search, bm25_search_code
- [x] **T020**: Implement Entity ops: create_entity, get_entity, search_entities
- [x] **T021**: Implement Relation ops: create_relation, get_related, get_subgraph, get_node_degrees
- [x] **T022**: Implement Temporal ops: get_valid, get_valid_at, invalidate
- [x] **T023**: Implement Code ops: create_code_chunk, create_code_chunks_batch, delete_project_chunks
- [x] **T024**: Implement Index status: get_index_status, update_index_status, list_projects
- [x] **T025**: Implement System: health_check
- [ ] **T025a**: Write tests for WP03 components

### Success Criteria

- All trait methods implemented for SurrealStorage
- Integration tests pass with tempdir (dynamic dimension based on model)
- HNSW index created with dimension matching configured model (384, 768, or 1024)
- db_config table stores model dimension on first init

---

## WP04: Embedding Layer

**Objective**: Full embedding service with caching  
**Priority**: P1  
**Dependencies**: WP02 (can run parallel with WP03)  
**Prompt**: [WP04-embedding-layer.md](./tasks/WP04-embedding-layer.md)

### Subtasks

- [x] **T026**: Create src/embedding/config.rs (ModelType enum, EmbeddingConfig)
- [x] **T027**: Implement src/embedding/engine.rs - EmbeddingEngine model loading via hf-hub
- [x] **T028**: Implement src/embedding/engine.rs - embed() function with mean pooling, L2 norm
- [x] **T029**: Implement src/embedding/cache.rs (EmbeddingCache LRU with blake3 keys)
- [x] **T030**: Implement src/embedding/service.rs (EmbeddingService async wrapper)
- [x] **T031**: Implement background model loading (spawn_blocking, non-blocking startup)
- [x] **T032**: Implement status tracking: Loading → Ready → Error states
- [ ] **T032a**: Write tests for WP04 components

### Success Criteria

- `EmbeddingService::embed("text")` returns `Vec<f32>` matching model dimension
- Cache hit ratio trackable via stats
- Server starts within 1 second (model loads in background)

---

## WP05: MCP Server + Memory Tools

**Objective**: Working MCP server with 5 basic memory operations  
**Priority**: P1  
**Dependencies**: WP03, WP04  
**Prompt**: [WP05-mcp-memory-tools.md](./tasks/WP05-mcp-memory-tools.md)

### Subtasks

- [x] **T033**: Create src/server/handler.rs (MemoryMcpServer struct with AppState)
- [x] **T034**: Implement ServerHandler trait (get_info with server metadata)
- [x] **T035**: Implement tool: store_memory (content, memory_type, user_id, metadata)
- [x] **T036**: Implement tool: get_memory (id → Memory)
- [x] **T037**: Implement tool: update_memory (id, optional fields, re-embed on content change)
- [x] **T038**: Implement tool: delete_memory (id → hard delete)
- [x] **T039**: Implement tool: list_memories (limit, offset → paginated list)
- [x] **T040**: Create src/main.rs with clap CLI + ENV support via MEMORY_MCP_* prefix (--data-dir, --model, --cache-size, --batch-size, --timeout, --log-level, --list-models, --force-model, --reset-memory). CLI takes precedence over ENV.
- [ ] **T040a**: Implement dimension mismatch check on startup: compare model dimension with db_config.dimension, apply --force-model or --reset-memory logic per FR-041-044
- [x] **T041**: Wire up AppState with storage + embedding services
- [ ] **T041a**: Write tests for WP05 components

### Success Criteria

- `cargo run` responds to MCP `tools/list` within 1 second
- All 5 memory tools functional via MCP protocol
- store_memory completes in < 100ms (excluding model load)

---

## WP06: Search Tools

**Objective**: Vector + BM25 + Hybrid search (3 tools)  
**Priority**: P1  
**Dependencies**: WP05  
**Prompt**: [WP06-search-tools.md](./tasks/WP06-search-tools.md)

### Subtasks

- [x] **T042**: Create src/graph/rrf.rs (rrf_merge function with k=60)
- [x] **T042a**: Implement Z-Score normalization in rrf.rs for balancing code/text scores
- [x] **T043**: Implement tool: search (vector similarity via HNSW, limit max 50 results per FR-011)
- [x] **T044**: Implement tool: search_text (BM25 full-text search, limit max 50 results per FR-011)
- [x] **T045**: Implement tool: recall (hybrid merge, PPR=0 placeholder, limit max 50 results per FR-011)
- [ ] **T045a**: Write tests for WP06 components

### Success Criteria

- search completes in < 20ms for 10K memories
- search_text completes in < 30ms for 10K memories
- recall returns merged results with vector_score, bm25_score, ppr_score (0)

---

## WP07: Graph Tools + PPR

**Objective**: Knowledge graph operations + Personalized PageRank  
**Priority**: P2  
**Dependencies**: WP06  
**Prompt**: [WP07-graph-tools-ppr.md](./tasks/WP07-graph-tools-ppr.md)

### Subtasks

- [x] **T046**: Create src/graph/ppr.rs (personalized_page_rank: damping=0.5, tolerance=1e-6, max_iter=15)
- [x] **T047**: Implement tool: create_entity (name, entity_type, description, user_id)
- [x] **T048**: Implement tool: create_relation (from_entity, to_entity, relation_type, weight)
- [x] **T049**: Implement tool: get_related (entity_id, depth, direction)
- [x] **T050**: Update recall tool to use real PPR scores
- [x] **T051**: Implement hub dampening (weight = score / sqrt(degree))
- [ ] **T051a**: Implement tool: detect_communities (Leiden algorithm)
- [ ] **T051b**: Write tests for WP07 components

### Success Criteria

- PPR converges within 15 iterations
- get_related returns entities within specified depth
- recall hybrid weights: vector=0.40, bm25=0.15, ppr=0.45

---

## WP08: Temporal Tools

**Objective**: Bi-temporal query support (3 tools)  
**Priority**: P2  
**Dependencies**: WP05  
**Prompt**: [WP08-temporal-tools.md](./tasks/WP08-temporal-tools.md)

### Subtasks

- [x] **T052**: Implement tool: get_valid (currently valid memories)
- [x] **T053**: Implement tool: get_valid_at (memories valid at timestamp)
- [x] **T054**: Implement tool: invalidate (soft-delete with reason, superseded_by)
- [ ] **T054a**: Write tests for WP08 components

### Success Criteria

- Invalidated memories excluded from get_valid
- get_valid_at returns memories where valid_from ≤ timestamp < valid_until

---

## WP09: Code Search Tools

**Objective**: Codebase indexing + semantic code search (5 tools)  
**Priority**: P3  
**Dependencies**: WP05  
**Prompt**: [WP09-code-search-tools.md](./tasks/WP09-code-search-tools.md)

### Subtasks

- [x] **T055**: Create src/codebase/scanner.rs (scan_directory, is_code_file, respect .gitignore/.memoryignore)
- [ ] **T055a**: Create src/codebase/watcher.rs (Debounced 500ms file watcher, auto-reindex)
- [x] **T056**: Create src/codebase/chunker.rs (chunk_file with tree-sitter AST parsing)
- [x] **T057**: Create src/codebase/indexer.rs (index_directory, batch embedding, adaptive sizing max 8192 tokens)
- [x] **T058**: Implement tool: index_project (path, mandatory watch flag)
- [x] **T059**: Implement tool: search_code (query, optional project_id filter)
- [x] **T060**: Implement tool: get_index_status (project_id → IndexStatus)
- [x] **T061**: Implement tool: list_projects (→ project IDs)
- [x] **T062**: Implement tool: delete_project (project_id → chunks deleted)
- [ ] **T062a**: Write tests for WP09 components

### Success Criteria

- index_project processes 100 files in < 5 minutes
- search_code returns results with file_path, start_line, end_line
- Tree-sitter chunking preserves function/class boundaries

---

## WP10: System Tool + Polish

**Objective**: Final tool + production readiness  
**Priority**: P3  
**Dependencies**: WP07, WP08, WP09  
**Prompt**: [WP10-system-polish.md](./tasks/WP10-system-polish.md)

### Subtasks

- [x] **T063**: Implement tool: get_status (version, memories_count, embedding status, model dimension)
- [x] **T063a**: Implement tool: reset_all_memory (confirm: true required, deletes all memories/entities/relations/code_chunks)
- [ ] **T064**: Create Dockerfile (multi-stage production build)
- [ ] **T065**: Create Dockerfile.local (dev build with pre-built binary)
- [x] **T066**: Error message cleanup and consistency
- [x] **T067**: Logging improvements (tracing to stderr)
- [ ] **T068**: Create README.md with usage documentation (21 tools, ENV vars table)
- [ ] **T069**: E2E test suite validation (target: 84+ tests)
- [ ] **T069a**: Write tests for WP10 components

### Success Criteria

- Docker image builds for linux/amd64
- Binary size < 30MB
- All 84+ tests pass
- README documents all 21 tools and MEMORY_MCP_* environment variables

---

## Subtask Index

| ID | Phase | Description | Parallel |
|----|-------|-------------|----------|
| T001 | 0 | Candle PoC | [P] |
| T002 | 0 | rmcp PoC | [P] |
| T003 | 0 | SurrealDB PoC | [P] |
| T004 | 1 | Cargo.toml dependencies | |
| T005 | 1 | .cargo/config.toml linker | |
| T006 | 1 | types/error.rs | |
| T007 | 1 | types/memory.rs | |
| T008 | 1 | types/entity.rs | |
| T009 | 1 | types/code.rs | |
| T010 | 1 | types/search.rs | |
| T011 | 1 | config.rs | |
| T012 | 1 | lib.rs | |
| T013 | 2 | storage/traits.rs | |
| T014 | 2 | storage/schema.surql | |
| T015 | 2 | SurrealStorage init | |
| T016 | 2 | Memory CRUD | |
| T017 | 2 | Memory list/count | |
| T018 | 2 | Vector search | |
| T019 | 2 | BM25 search | |
| T020 | 2 | Entity ops | |
| T021 | 2 | Relation ops | |
| T022 | 2 | Temporal ops | |
| T023 | 2 | Code ops | |
| T024 | 2 | Index status | |
| T025 | 2 | health_check | |
| T026 | 3 | embedding/config.rs | |
| T027 | 3 | EmbeddingEngine loading | |
| T028 | 3 | embed() + pooling | |
| T029 | 3 | EmbeddingCache | |
| T030 | 3 | EmbeddingService | |
| T031 | 3 | Background loading | |
| T032 | 3 | Status tracking | |
| T033 | 4 | MemoryMcpServer struct | |
| T034 | 4 | ServerHandler trait | |
| T035 | 4 | tool: store_memory | |
| T036 | 4 | tool: get_memory | |
| T037 | 4 | tool: update_memory | |
| T038 | 4 | tool: delete_memory | |
| T039 | 4 | tool: list_memories | |
| T040 | 4 | main.rs CLI | |
| T041 | 4 | AppState wiring | |
| T042 | 5 | graph/rrf.rs | |
| T043 | 5 | tool: search | |
| T044 | 5 | tool: search_text | |
| T045 | 5 | tool: recall (hybrid) | |
| T046 | 6 | graph/ppr.rs | |
| T047 | 6 | tool: create_entity | |
| T048 | 6 | tool: create_relation | |
| T049 | 6 | tool: get_related | |
| T050 | 6 | recall PPR integration | |
| T051 | 6 | Hub dampening | |
| T052 | 7 | tool: get_valid | |
| T053 | 7 | tool: get_valid_at | |
| T054 | 7 | tool: invalidate | |
| T055 | 8 | codebase/scanner.rs | |
| T056 | 8 | codebase/chunker.rs | |
| T057 | 8 | codebase/indexer.rs | |
| T058 | 8 | tool: index_project | |
| T059 | 8 | tool: search_code | |
| T060 | 8 | tool: get_index_status | |
| T061 | 8 | tool: list_projects | |
| T062 | 8 | tool: delete_project | |
| T063 | 9 | tool: get_status | |
| T064 | 9 | Dockerfile | |
| T065 | 9 | Dockerfile.local | |
| T066 | 9 | Error cleanup | |
| T067 | 9 | Logging improvements | |
| T068 | 9 | README.md | |
| T069 | 9 | E2E test validation | |
| T003a | 0 | Write tests for WP01 | |
| T012a | 1 | Write tests for WP02 | |
| T025a | 2 | Write tests for WP03 | |
| T032a | 3 | Write tests for WP04 | |
| T041a | 4 | Write tests for WP05 | |
| T045a | 5 | Write tests for WP06 | |
| T051b | 6 | Write tests for WP07 | |
| T054a | 7 | Write tests for WP08 | |
| T062a | 8 | Write tests for WP09 | |
| T069a | 9 | Write tests for WP10 | |
