# Implementation Plan: Memory MCP Server

**Branch**: `001-memory-mcp-server` | **Date**: 2026-01-06 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/kitty-specs/001-memory-mcp-server/spec.md`

## Summary

Self-contained MCP memory server for AI agents in a single Rust binary. Implements 20 MCP tools covering memory CRUD, semantic/hybrid search, knowledge graph, temporal queries, and code search. Uses SurrealDB (embedded) for storage, Candle (pure Rust) for embeddings, and rmcp for MCP protocol.

**Approach**: Risk-First + Vertical Slices across 10 phases, starting with PoC validation of highest-risk components (Candle, rmcp, SurrealDB).

## Technical Context

**Language/Version**: Rust 2021 edition
**Primary Dependencies**: 
- surrealdb 2.x (embedded kv-surrealkv)
- candle-core/nn/transformers 0.9.1
- rmcp 0.12.0
- petgraph 0.6
- tree-sitter 0.26, code-splitter 0.4
- tokio 1.x, serde 1.x, clap 4.x

**Storage**: SurrealDB embedded at `~/.local/share/memory-mcp/db/`
**Testing**: cargo test (unit + integration), 84+ tests target
**Target Platform**: Linux x86_64 (primary), Windows, macOS
**Project Type**: Single binary CLI application
**Performance Goals**: 
- store_memory < 100ms
- search (vector) < 20ms for 10K memories
- search_text (BM25) < 30ms for 10K memories
- recall (hybrid) < 100ms for 10K memories
- Server startup < 1s (model loads in background)

**Constraints**: 
- Binary size < 30MB (excluding model files)
- Model files 134MB - 2.3GB depending on selection
- Single-tenant, offline-first, privacy-focused

**Scale/Scope**: 10K+ memories, multiple indexed projects, 20 MCP tools

## Constitution Check

*Constitution is template-only (no specific principles defined). Proceeding with industry best practices.*

**Applied Standards**:
- Test-first: Unit tests for pure logic, integration tests for storage/embedding
- Single responsibility: Modular structure (types, storage, embedding, graph, codebase, server)
- Error handling: thiserror for types, anyhow for application
- Security: No credential leaks, logs to stderr, token protection (embeddings never serialized)

## Project Structure

### Documentation (this feature)

```
kitty-specs/001-memory-mcp-server/
├── spec.md              # Feature specification
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (MCP tool schemas)
├── checklists/          # Requirements checklist
└── tasks.md             # Phase 2 output (created by /spec-kitty.tasks)
```

### Source Code (repository root)

```
memory-mcp/
├── .cargo/
│   └── config.toml           # Linker config (mold for Linux, lld for Windows)
├── src/
│   ├── lib.rs                # pub mod declarations, re-exports
│   ├── main.rs               # CLI entry point (clap)
│   ├── config.rs             # AppConfig, AppState structs
│   ├── types/
│   │   ├── mod.rs            # Re-exports
│   │   ├── memory.rs         # Memory, MemoryType, MemoryUpdate
│   │   ├── entity.rs         # Entity, Relation, Direction
│   │   ├── code.rs           # CodeChunk, ChunkType, Language, IndexStatus
│   │   ├── search.rs         # SearchResult, RecallResult, ScoredMemory
│   │   └── error.rs          # AppError (thiserror), Result<T> alias
│   ├── storage/
│   │   ├── mod.rs            # Re-exports
│   │   ├── traits.rs         # StorageBackend trait (async_trait)
│   │   ├── surrealdb.rs      # SurrealStorage implementation
│   │   └── schema.surql      # DDL for tables and indexes
│   ├── embedding/
│   │   ├── mod.rs            # Re-exports
│   │   ├── config.rs         # ModelType enum, EmbeddingConfig
│   │   ├── engine.rs         # EmbeddingEngine (sync Candle wrapper)
│   │   ├── cache.rs          # EmbeddingCache (LRU + blake3 keys)
│   │   └── service.rs        # EmbeddingService (async, background preload)
│   ├── graph/
│   │   ├── mod.rs            # Re-exports
│   │   ├── ppr.rs            # Personalized PageRank (petgraph)
│   │   └── rrf.rs            # Reciprocal Rank Fusion
│   ├── codebase/
│   │   ├── mod.rs            # Re-exports
│   │   ├── scanner.rs        # scan_directory(), is_code_file()
│   │   ├── chunker.rs        # chunk_file(), tree-sitter parsing
│   │   ├── indexer.rs        # index_directory(), batch embedding
│   │   └── watcher.rs        # CodebaseWatcher (optional, deferred)
│   └── server/
│       ├── mod.rs            # Re-exports
│       └── handler.rs        # MemoryMcpServer + 20 MCP tools
├── tests/
│   ├── storage_test.rs       # Storage CRUD, search
│   ├── embedding_test.rs     # Embedding (ignored - requires model)
│   ├── graph_test.rs         # PPR, graph traversal
│   ├── temporal_test.rs      # Temporal validity
│   ├── handler_test.rs       # MCP tool handlers
│   └── e2e_test.rs           # Full E2E suite
├── Cargo.toml
├── Dockerfile                # Production multi-stage build
├── Dockerfile.local          # Dev build (pre-built binary)
└── README.md
```

**Structure Decision**: Single project with modular internal structure. No workspace - all code in one crate for simpler build and distribution.

## Implementation Phases

### Phase 0: Risk Validation (PoC)

**Objective**: Validate highest-risk components before full implementation.

| Component | Validation Task | Pass Criteria |
|-----------|-----------------|---------------|
| Candle | Load e5_small, embed "hello world" | Returns 384-dim vector |
| rmcp | Minimal MCP server, 1 dummy tool | Responds to tools/list |
| SurrealDB | Embedded DB, CRUD, HNSW index | Vector search works |

**Deliverable**: `_tmp/poc/` with 3 standalone test binaries
**Gate**: All 3 pass -> continue. Any fails -> resolve before Phase 1.

### Phase 1: Core Foundation

**Objective**: Project structure + all type definitions

**Tasks**:
1. Create Cargo.toml with all dependencies
2. Create .cargo/config.toml for linker optimization
3. Implement types/error.rs (AppError, Result alias)
4. Implement types/memory.rs (Memory, MemoryType, MemoryUpdate)
5. Implement types/entity.rs (Entity, Relation, Direction)
6. Implement types/code.rs (CodeChunk, ChunkType, Language, IndexStatus)
7. Implement types/search.rs (SearchResult, RecallResult, ScoredMemory)
8. Create config.rs (AppConfig, AppState)
9. Create lib.rs with pub mod declarations

**Tests**: Unit tests for serde serialization, Default impls
**Deliverable**: `cargo check` passes

### Phase 2: Storage Layer

**Objective**: Storage abstraction + SurrealDB implementation

**Tasks**:
1. Create storage/traits.rs (StorageBackend trait)
2. Create storage/schema.surql (DDL)
3. Implement storage/surrealdb.rs (SurrealStorage)
4. Memory CRUD: create, get, update, delete, list, count
5. Vector search: vector_search, vector_search_code
6. BM25 search: bm25_search, bm25_search_code
7. Entity ops: create_entity, get_entity, search_entities
8. Relation ops: create_relation, get_related, get_subgraph, get_node_degrees
9. Temporal ops: get_valid, get_valid_at, invalidate
10. Code ops: create_code_chunk, create_code_chunks_batch, delete_project_chunks, get_index_status, list_projects
11. System: health_check

**Tests**: Integration tests with tempdir
**Deliverable**: Storage layer works with mock embeddings (zero vectors)

### Phase 3: Embedding Layer

**Objective**: Full embedding service with caching

**Tasks**:
1. Create embedding/config.rs (ModelType, EmbeddingConfig)
2. Implement embedding/engine.rs (EmbeddingEngine with Candle)
3. Implement embedding/cache.rs (EmbeddingCache LRU)
4. Implement embedding/service.rs (EmbeddingService async wrapper)
5. Background model loading (non-blocking startup)
6. Status tracking: Loading -> Ready -> Error

**Models**: e5_small (384d), e5_multi (768d, default), nomic (768d), bge_m3 (1024d)

**Tests**: 
- Unit: cache hit/miss, key normalization
- Integration: `#[ignore]` tests requiring model download

**Deliverable**: `EmbeddingService::embed("text")` returns `Vec<f32>`

### Phase 4: MCP Server + Memory Tools (5 tools)

**Objective**: Working MCP server with basic memory operations

**Tasks**:
1. Create server/handler.rs (MemoryMcpServer)
2. Implement ServerHandler trait (get_info)
3. Implement tool: store_memory
4. Implement tool: get_memory
5. Implement tool: update_memory
6. Implement tool: delete_memory
7. Implement tool: list_memories
8. Create main.rs with clap CLI
9. Wire up AppState with storage + embedding

**CLI Flags**: --data-dir, --model, --cache-size, --batch-size, --timeout, --log-level, --list-models

**Tests**: Handler tests with MockEmbeddingService
**Deliverable**: `cargo run` responds to MCP `tools/list`

### Phase 5: Search Tools (3 tools)

**Objective**: Vector + BM25 + Hybrid search

**Tasks**:
1. Create graph/rrf.rs (rrf_merge function, z_score_normalize)
2. Implement tool: search (vector similarity)
3. Implement tool: search_text (BM25)
4. Implement tool: recall (hybrid, PPR=0 until Phase 6)

**Algorithm**: RRF merge with k=60, weights: vector=0.40, bm25=0.15, ppr=0.45, with **Z-Score Normalization**


**Tests**: Search ranking with 10+ memories
**Deliverable**: All 3 search tools functional

### Phase 6: Graph Tools (3 tools) + PPR

**Objective**: Knowledge graph + Personalized PageRank

**Tasks**:
1. Create graph/ppr.rs (personalized_page_rank)
2. Implement tool: create_entity
3. Implement tool: create_relation
4. Implement tool: get_related
5. Update recall tool to use real PPR scores

**PPR Parameters**: damping=0.5 (HippoRAG), tolerance=1e-6, max_iter=15

**Tests**: Graph traversal, PPR convergence
**Deliverable**: Graph tools + enhanced recall with PPR

### Phase 7: Temporal Tools (3 tools)

**Objective**: Bi-temporal query support

**Tasks**:
1. Implement tool: get_valid
2. Implement tool: get_valid_at
3. Implement tool: invalidate

**Tests**: Temporal validity checks
**Deliverable**: Full temporal support

### Phase 8: Code Search Tools (5 tools)

**Objective**: Codebase indexing + semantic code search

**Tasks**:
1. Create codebase/scanner.rs (scan_directory, is_code_file)
2. Create codebase/chunker.rs (chunk_file, tree-sitter)
3. Create codebase/indexer.rs (index_directory, batch embedding)
4. Create codebase/watcher.rs (Debounced 500ms file watcher)
4. Implement tool: index_project
5. Implement tool: search_code
6. Implement tool: get_index_status
7. Implement tool: list_projects
8. Implement tool: delete_project

**Features**: 
- Respects .gitignore + .memoryignore
- Tree-sitter AST chunking with context injection
- Adaptive batch sizing by tokens

**Tests**: Index small project, search code
**Deliverable**: Code search functional

### Phase 9: System Tool + Polish

**Objective**: Final tool + production readiness

**Tasks**:
1. Implement tool: get_status
2. Create Dockerfile (multi-stage production)
3. Create Dockerfile.local (dev build)
4. Error message cleanup
5. Logging improvements (tracing to stderr)
6. README.md with usage docs

**Tests**: Full E2E suite (84+ tests)
**Deliverable**: Production-ready binary

## Dependency Graph

```
Phase 0 (PoC) ─────────────────────────────────────────────────┐
    │                                                          │
    ▼                                                          │
Phase 1 (Types) ───────────────────────────────────────────────┤
    │                                                          │
    ├──────────────────┬───────────────────┐                   │
    ▼                  ▼                   ▼                   │
Phase 2 (Storage)  Phase 3 (Embedding)  [parallel]             │
    │                  │                                       │
    └────────┬─────────┘                                       │
             ▼                                                 │
Phase 4 (MCP + Memory Tools) ──────────────────────────────────┤
             │                                                 │
             ▼                                                 │
Phase 5 (Search Tools) ────────────────────────────────────────┤
             │                                                 │
             ▼                                                 │
Phase 6 (Graph Tools + PPR) ───────────────────────────────────┤
             │                                                 │
             ▼                                                 │
Phase 7 (Temporal Tools) ──────────────────────────────────────┤
             │                                                 │
             ▼                                                 │
Phase 8 (Code Search Tools) ───────────────────────────────────┤
             │                                                 │
             ▼                                                 │
Phase 9 (Polish) ──────────────────────────────────────────────┘
```

## Parallel Work Analysis

### Sequential vs Parallel

**Sequential (must be in order)**:
- Phase 0 -> Phase 1 (types needed for everything)
- Phase 4 depends on Phase 2 + Phase 3
- Phase 6 depends on Phase 5 (recall enhancement)

**Parallel opportunities**:
- Phase 2 (Storage) and Phase 3 (Embedding) can be developed in parallel
- Phase 7 and Phase 8 have minimal dependencies on each other

### Work Distribution (if multiple agents)

| Agent | Phases | Files |
|-------|--------|-------|
| Agent A | 0, 1, 2, 4 | types/, storage/, main.rs |
| Agent B | 3, 5, 6 | embedding/, graph/ |
| Agent C | 7, 8, 9 | server/ (temporal/code tools), codebase/ |

### Coordination Points

- After Phase 1: Types API frozen
- After Phase 4: MCP server API frozen
- Before Phase 9: Integration testing

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Candle API changes | Medium | Critical | Phase 0 PoC, pin version |
| rmcp macro issues | Medium | High | Fallback to manual impl |
| SurrealDB HNSW perf | Low | High | Early benchmark |
| tree-sitter compat | Low | Medium | Fallback to fixed-size chunks |

## Success Criteria

From spec.md SC-001 to SC-010:
- [SC-001] store_memory < 100ms
- [SC-002] search < 20ms for 10K memories
- [SC-003] search_text < 30ms for 10K memories
- [SC-004] recall < 100ms for 10K memories
- [SC-005] index_project < 5 min for 100 files
- [SC-006] Binary < 30MB
- [SC-007] Model download on first run
- [SC-008] Server startup < 1s
- [SC-009] 84+ tests pass
- [SC-010] Docker image builds
