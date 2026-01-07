# Code Graph Implementation Plan

**Epic ID:** EPIC-001-codegraph
**Status:** Active
**Created:** 2026-01-07
**Target Languages:** Rust, Python, TypeScript, JavaScript, Go, Java (ALL AT ONCE)

---

## Executive Summary

Implement automatic code graph generation in Memory MCP using tree-sitter for AST parsing. Extract symbols (functions, classes, methods) and their relationships (calls, imports, contains) and store them as entities/relations in SurrealDB.

---

## Work Packages

### WP01: Dependencies & Types (Foundation)

**Priority:** P0 - Blocker
**Estimate:** 30 min
**Dependencies:** None

#### Tasks

| ID | Task | File | Status |
|----|------|------|--------|
| T01.1 | Add tree-sitter language crates to Cargo.toml | `Cargo.toml` | pending |
| T01.2 | Create `CodeSymbol` struct | `src/types/symbol.rs` | pending |
| T01.3 | Create `SymbolType` enum | `src/types/symbol.rs` | pending |
| T01.4 | Create `CodeRelation` struct | `src/types/symbol.rs` | pending |
| T01.5 | Create `CodeRelationType` enum | `src/types/symbol.rs` | pending |
| T01.6 | Export types in `src/types/mod.rs` | `src/types/mod.rs` | pending |

#### Acceptance Criteria
- [ ] `cargo check` passes
- [ ] All types are accessible from `crate::types`

---

### WP02: Tree-sitter Parser Module (Core)

**Priority:** P0 - Blocker
**Estimate:** 2-3 hours
**Dependencies:** WP01

#### Tasks

| ID | Task | File | Status |
|----|------|------|--------|
| T02.1 | Create `parser` module structure | `src/codebase/parser/mod.rs` | pending |
| T02.2 | Implement `LanguageSupport` trait | `src/codebase/parser/languages.rs` | pending |
| T02.3 | Implement Rust language support | `src/codebase/parser/languages.rs` | pending |
| T02.4 | Implement Python language support | `src/codebase/parser/languages.rs` | pending |
| T02.5 | Implement TypeScript language support | `src/codebase/parser/languages.rs` | pending |
| T02.6 | Implement JavaScript language support | `src/codebase/parser/languages.rs` | pending |
| T02.7 | Implement Go language support | `src/codebase/parser/languages.rs` | pending |
| T02.8 | Implement Java language support | `src/codebase/parser/languages.rs` | pending |
| T02.9 | Create `CodeParser` struct | `src/codebase/parser/mod.rs` | pending |
| T02.10 | Implement `parse_file()` method | `src/codebase/parser/mod.rs` | pending |
| T02.11 | Implement `extract_symbols()` | `src/codebase/parser/extractor.rs` | pending |
| T02.12 | Implement `extract_relations()` | `src/codebase/parser/extractor.rs` | pending |
| T02.13 | Export parser in `src/codebase/mod.rs` | `src/codebase/mod.rs` | pending |

#### Acceptance Criteria
- [ ] Parser correctly extracts functions/classes for all 6 languages
- [ ] Parser correctly extracts call references
- [ ] Unit tests pass for each language

---

### WP03: Storage Layer Extension (Persistence)

**Priority:** P0 - Blocker
**Estimate:** 1 hour
**Dependencies:** WP01

#### Tasks

| ID | Task | File | Status |
|----|------|------|--------|
| T03.1 | Add `code_symbols` table to schema | `src/storage/schema.surql` | pending |
| T03.2 | Add trait methods for symbols | `src/storage/traits.rs` | pending |
| T03.3 | Implement `create_code_symbol()` | `src/storage/surrealdb.rs` | pending |
| T03.4 | Implement `create_code_symbols_batch()` | `src/storage/surrealdb.rs` | pending |
| T03.5 | Implement `create_symbol_relation()` | `src/storage/surrealdb.rs` | pending |
| T03.6 | Implement `get_symbol_callers()` | `src/storage/surrealdb.rs` | pending |
| T03.7 | Implement `get_symbol_callees()` | `src/storage/surrealdb.rs` | pending |
| T03.8 | Implement `search_symbols()` | `src/storage/surrealdb.rs` | pending |
| T03.9 | Implement `delete_project_symbols()` | `src/storage/surrealdb.rs` | pending |

#### Acceptance Criteria
- [ ] Symbols can be created and retrieved
- [ ] Relations between symbols work via RELATE
- [ ] Integration tests pass

---

### WP04: Indexer Integration (Connect Parser to Indexer)

**Priority:** P1 - High
**Estimate:** 1 hour
**Dependencies:** WP02, WP03

#### Tasks

| ID | Task | File | Status |
|----|------|------|--------|
| T04.1 | Initialize CodeParser in indexer | `src/codebase/indexer.rs` | pending |
| T04.2 | Call parser for each file | `src/codebase/indexer.rs` | pending |
| T04.3 | Store extracted symbols | `src/codebase/indexer.rs` | pending |
| T04.4 | Store extracted relations | `src/codebase/indexer.rs` | pending |
| T04.5 | Update incremental_index for symbols | `src/codebase/indexer.rs` | pending |
| T04.6 | Add symbol count to IndexStatus | `src/types/code.rs` | pending |

#### Acceptance Criteria
- [ ] `index_project` creates symbols and relations
- [ ] Re-indexing cleans up old symbols
- [ ] IndexStatus shows symbol count

---

### WP05: MCP Tools (API Exposure)

**Priority:** P1 - High
**Estimate:** 1 hour
**Dependencies:** WP04

#### Tasks

| ID | Task | File | Status |
|----|------|------|--------|
| T05.1 | Add `GetCallersParams` | `src/server/params.rs` | pending |
| T05.2 | Add `GetCalleesParams` | `src/server/params.rs` | pending |
| T05.3 | Add `SearchSymbolsParams` | `src/server/params.rs` | pending |
| T05.4 | Implement `get_callers` logic | `src/server/logic/code.rs` | pending |
| T05.5 | Implement `get_callees` logic | `src/server/logic/code.rs` | pending |
| T05.6 | Implement `search_symbols` logic | `src/server/logic/code.rs` | pending |
| T05.7 | Register tools in handler | `src/server/handler.rs` | pending |

#### Acceptance Criteria
- [ ] `get_callers` returns functions that call a given symbol
- [ ] `get_callees` returns functions called by a given symbol
- [ ] `search_symbols` finds symbols by name pattern

---

### WP06: Tests & Documentation

**Priority:** P2 - Medium
**Estimate:** 1 hour
**Dependencies:** WP05

#### Tasks

| ID | Task | File | Status |
|----|------|------|--------|
| T06.1 | Add parser unit tests | `src/codebase/parser/mod.rs` | pending |
| T06.2 | Add storage integration tests | `src/storage/surrealdb.rs` | pending |
| T06.3 | Add end-to-end test | `src/server/logic/code.rs` | pending |
| T06.4 | Update README with new tools | `README.md` | pending |
| T06.5 | Add example queries | `doc/examples/` | pending |

#### Acceptance Criteria
- [ ] All tests pass
- [ ] Documentation updated
- [ ] `cargo test` clean

---

## Dependency Graph

```
WP01 (Types)
   ↓
   ├──────────────────┐
   ↓                  ↓
WP02 (Parser)    WP03 (Storage)
   ↓                  ↓
   └────────┬─────────┘
            ↓
      WP04 (Indexer)
            ↓
      WP05 (MCP Tools)
            ↓
      WP06 (Tests/Docs)
```

## Optimal Execution Order

1. **WP01** - Quick foundation, unblocks everything
2. **WP02 + WP03** - Can be done in parallel (different modules)
3. **WP04** - Connects parser to storage
4. **WP05** - Exposes via MCP
5. **WP06** - Polish and documentation

## Files to Create

```
src/
├── types/
│   └── symbol.rs          # NEW: CodeSymbol, CodeRelation
├── codebase/
│   ├── parser/            # NEW DIRECTORY
│   │   ├── mod.rs         # CodeParser
│   │   ├── languages.rs   # Language-specific queries
│   │   └── extractor.rs   # Symbol/relation extraction
│   └── mod.rs             # MODIFY: add parser export
├── storage/
│   ├── schema.surql       # MODIFY: add code_symbols table
│   ├── traits.rs          # MODIFY: add symbol methods
│   └── surrealdb.rs       # MODIFY: implement symbol methods
└── server/
    ├── params.rs          # MODIFY: add new params
    ├── handler.rs         # MODIFY: add new tools
    └── logic/
        └── code.rs        # MODIFY: add new logic
```

## Files to Modify

| File | Changes |
|------|---------|
| `Cargo.toml` | Add 7 tree-sitter crates |
| `src/types/mod.rs` | Export symbol types |
| `src/types/code.rs` | Add symbol_count to IndexStatus |
| `src/codebase/mod.rs` | Export parser module |
| `src/storage/schema.surql` | Add code_symbols table |
| `src/storage/traits.rs` | Add 7 trait methods |
| `src/storage/surrealdb.rs` | Implement 7 methods |
| `src/codebase/indexer.rs` | Integrate parser |
| `src/server/params.rs` | Add 3 param structs |
| `src/server/handler.rs` | Add 3 tool handlers |
| `src/server/logic/code.rs` | Add 3 logic functions |
| `README.md` | Document new tools |

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Tree-sitter version conflicts | Use `tree-sitter-language` crate |
| Large binary size | Accept for now, optimize later |
| Incomplete call graph | Document limitations |
| Performance on large repos | Batch processing, already in place |

---

## Ready to Start

**Next Action:** Begin WP01 - Add dependencies and create types

**Command to start:**
```bash
# Switch to development mode
cargo check  # Verify current state
```
