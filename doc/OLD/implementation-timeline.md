# Implementation Timeline

## Total Estimated Time: ~12 hours

---

## Етап 0: Project Detection (30 хв)

**Завдання:**
- Implement `detect_project()` function
- Git root traversal via `.git` directory search
- Extract repository name from path
- Generate namespace string

**Files:**
- `src/project/mod.rs` — [NEW]
- `src/project/namespace.rs` — [NEW]

**Testing:**
- Unit test with test Git repo fixture

---

## Етап 1: Dependencies (10 хв)

**Завдання:**
- Update `Cargo.toml` with new dependencies

**Dependencies:**
```toml
code-splitter = "0.1"
tree-sitter = "0.26"
tree-sitter-rust = "0.24"
tree-sitter-python = "0.25"
tree-sitter-javascript = "0.25"
tree-sitter-typescript = "0.23"
walkdir = "2"
ignore = "0.4"
notify = "8"
rayon = "1.10"
```

**Testing:**
- `cargo check`

---

## Етап 2-3: File Scanner (45 хв)

**Завдання:**
- Implement `scan_directory()` with walkdir
- .gitignore support via `ignore` crate
- Language detection by extension
- Custom `.memoryignore` support

**Files:**
- `src/codebase/scanner.rs` — [NEW]

**Testing:**
- Unit test with fixture directory
- Verify .gitignore exclusions work

---

## Етап 4: Code Chunker (30 хв)

**Завдання:**
- Wrapper around `code-splitter` crate
- Implement `OVERLAP_TOKENS` constant (50)
- Hierarchical context injection
- Fallback to fixed-size chunking

**Files:**
- `src/codebase/chunker.rs` — [NEW]

**Testing:**
- Unit test: Rust file → expected chunks
- Unit test: Python file → expected chunks
- Unit test: Unknown extension → fallback

---

## Етап 5: Indexer (2 год)

**Завдання:**
- Implement `index_directory()` orchestration
- Adaptive batch sizing (<8192 tokens)
- Parallel chunking via rayon
- Async embedding + DB writes
- Retry logic with exponential backoff

**Files:**
- `src/codebase/indexer.rs` — [NEW]

**Testing:**
- Integration test: index small project
- Benchmark: throughput on 100 files

---

## Етап 6: Types (20 хв)

**Завдання:**
- Define `CodeChunk` struct
- Define `Language` enum
- Define `ChunkType` enum
- Define `SearchResult` enum (for recall)
- Implement serialization traits

**Files:**
- `src/types/code.rs` — [NEW]
- `src/types/search.rs` — [NEW]

**Testing:**
- Unit test: serde round-trip

---

## Етап 7: Storage (2 год)

**Завдання:**
- Extend `StorageBackend` trait
- Implement namespace management
- Add `code_chunks` table to schema
- Implement vector/BM25 search for code
- Add progress tracking table

**Files:**
- `src/storage/traits.rs` — [MODIFY]
- `src/storage/surrealdb.rs` — [MODIFY]
- `src/storage/schema.surql` — [MODIFY]

**Testing:**
- Integration test: CRUD operations
- Integration test: search round-trip

---

## Етап 8: MCP Tools (2.5 год)

**Завдання:**
- Implement `index_project` tool
- Implement `search_code` tool
- Implement `get_index_status` tool
- Implement `list_projects` tool
- Implement `delete_project` tool
- Update argument schemas with JsonSchema

**Files:**
- `src/server/handler.rs` — [MODIFY]

**Testing:**
- Integration test: call each tool via MCP
- Verify JSON schema validation

---

## Етап 9: Recall Integration (2 год)

**Завдання:**
- Update `RecallArgs` with new fields
- Implement parallel search (memories + code)
- Implement Z-score normalization
- Add breakdown statistics
- Update response format

**Files:**
- `src/server/handler.rs` — [MODIFY]
- `src/graph/rrf.rs` or `src/search/normalize.rs` — [NEW]

**Testing:**
- Integration test: unified search
- Unit test: Z-score normalization
- Verify score fairness

---

## Етап 10: File Watcher (1 год)

**Завдання:**
- Implement `CodebaseWatcher` with notify crate
- Debouncing at 500ms
- Event batching
- Background task spawning

**Files:**
- `src/codebase/watcher.rs` — [NEW]

**Testing:**
- Integration test: modify file → reindex triggered
- Test debouncing behavior

---

## Етап 11: Progress Reporting (30 хв)

**Завдання:**
- Implement `IndexingProgress` struct
- ETA calculation
- Progress percentage
- Store in `index_status` table

**Files:**
- `src/codebase/progress.rs` — [NEW]

**Testing:**
- Unit test: ETA calculation
- Integration test: status updates during indexing

---

## Verification (1 год)

**Завдання:**
- End-to-end test: index real project → search
- Performance benchmarks
- Memory usage profiling
- Manual testing with MCP client

**Artifacts:**
- `tests/integration/codebase_test.rs`
- `benches/indexing.rs`

---

## Summary Timeline

| Phase | Hours | Cumulative |
|-------|-------|------------|
| 0. Project Detection | 0.5 | 0.5 |
| 1. Dependencies | 0.2 | 0.7 |
| 2-3. File Scanner | 0.75 | 1.45 |
| 4. Code Chunker | 0.5 | 1.95 |
| 5. Indexer | 2.0 | 3.95 |
| 6. Types | 0.3 | 4.25 |
| 7. Storage | 2.0 | 6.25 |
| 8. MCP Tools | 2.5 | 8.75 |
| 9. Recall Integration | 2.0 | 10.75 |
| 10. File Watcher | 1.0 | 11.75 |
| 11. Progress | 0.5 | 12.25 |
| Verification | 1.0 | **13.25** |

**Note**: Timeline assumes no major blockers. Adjust based on:
- Team experience with SurrealDB
- Familiarity with tree-sitter
- Testing depth requirements

---

## Critical Path

```
Dependencies (0.2h)
    ↓
Types (0.3h)
    ↓
Storage Schema (part of 7, 1h)
    ↓
Chunker (0.5h) + Scanner (0.75h) [parallel]
    ↓
Indexer (2h)
    ↓
MCP Tools (2.5h)
    ↓
Recall Integration (2h)
```

**Minimum time (critical path only)**: ~9.25 hours

**Parallelizable work**:
- File Watcher (can be done independently)
- Progress Reporting (can be done independently)
- Documentation updates (ongoing)

---

## Risk Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Tree-sitter integration issues | High | Fallback to fixed-size chunking |
| SurrealDB namespace bugs | High | Extensive testing, fallback to single namespace |
| Performance slower than expected | Medium | Profiling, adaptive batch sizing |
| Memory usage too high | Medium | Streaming, LRU cache tuning |
| MCP schema validation errors | Low | JsonSchema testing before deployment |

---

## Milestones

### M1: Core Indexing (Day 1, ~4h)
- Dependencies, Types, Storage, Chunker, Scanner

### M2: MCP Integration (Day 1-2, ~5h)
- Indexer, MCP Tools

### M3: Advanced Features (Day 2, ~4h)
- Recall Integration, File Watcher, Progress

### M4: Polish (Day 2, ~1h)
- Verification, Benchmarks, Documentation
