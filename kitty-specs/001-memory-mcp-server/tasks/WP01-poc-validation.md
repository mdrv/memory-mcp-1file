---
work_package_id: WP01
title: "PoC Validation"
phase: "Phase 0"
priority: P0
subtasks: ["T001", "T002", "T003", "T003a"]
lane: "done"
dependencies: []
history:
  - date: 2026-01-06
    action: created
    by: spec-kitty.tasks
  - date: 2026-01-06
    action: completed
    by: sisyphus
    note: "All 3 PoCs validated: Candle (384d, L2=1.0), rmcp (MCP handshake OK), SurrealDB (HNSW cosine search OK)"
---

# WP01: PoC Validation

## Objective

Validate highest-risk components (Candle, rmcp, SurrealDB) before full implementation. This is a gate - all 3 PoCs must pass before proceeding to Phase 1.

## Context

The Memory MCP Server depends on three relatively new or complex technologies:
1. **Candle** - Pure Rust ML framework for embeddings (not ONNX)
2. **rmcp** - Official Rust MCP SDK (evolving API)
3. **SurrealDB** - Embedded DB with HNSW vector index (newer feature)

Failure in any of these would require significant architectural changes.

**Location**: Create all PoC code in `_tmp/poc/` directory (NOT in main src/)

## Subtasks

### T001: Candle PoC [P]

**Goal**: Load e5_small model and generate embeddings

**Implementation**:

1. Create `_tmp/poc/candle-poc/Cargo.toml`:
```toml
[package]
name = "candle-poc"
version = "0.1.0"
edition = "2021"

[dependencies]
candle-core = "0.9.1"
candle-nn = "0.9.1"
candle-transformers = "0.9.1"
tokenizers = "0.22.2"
hf-hub = "0.3"
anyhow = "1"
```

2. Create `_tmp/poc/candle-poc/src/main.rs`:
   - Use `hf_hub::api::sync::Api` to download `intfloat/multilingual-e5-small`
   - Load tokenizer from repo
   - Load model weights into `BertModel`
   - Tokenize "hello world"
   - Run forward pass
   - Apply mean pooling over sequence
   - Apply L2 normalization
   - Assert output is 384-dimensional vector

**Pass Criteria**:
- Model downloads successfully
- Returns `Vec<f32>` with exactly 384 elements
- Vector is normalized (L2 norm ≈ 1.0)

---

### T002: rmcp PoC [P]

**Goal**: Minimal MCP server with 1 dummy tool

**Implementation**:

1. Create `_tmp/poc/rmcp-poc/Cargo.toml`:
```toml
[package]
name = "rmcp-poc"
version = "0.1.0"
edition = "2021"

[dependencies]
rmcp = { version = "0.12.0", features = ["server", "transport-io", "macros"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

2. Create `_tmp/poc/rmcp-poc/src/main.rs`:
   - Define `DummyServer` struct
   - Implement `ServerHandler` trait with `get_info` returning server name/version
   - Use `#[tool_router]` macro
   - Implement single tool `ping` that returns `{ "pong": true }`
   - Run server with stdio transport

**Pass Criteria**:
- Responds to `initialize` request
- `tools/list` returns the `ping` tool schema
- `tools/call` with `ping` returns `{ "pong": true }`

**Test Command**:
```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"test"}}}' | cargo run
```

---

### T003: SurrealDB PoC [P]

**Goal**: Embedded DB with HNSW vector search

**Implementation**:

1. Create `_tmp/poc/surrealdb-poc/Cargo.toml`:
```toml
[package]
name = "surrealdb-poc"
version = "0.1.0"
edition = "2021"

[dependencies]
surrealdb = { version = "2", default-features = false, features = ["kv-surrealkv", "rustls"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
```

2. Create `_tmp/poc/surrealdb-poc/src/main.rs`:
   - Connect to embedded DB in temp directory
   - Create schema:
     ```sql
     DEFINE TABLE memories SCHEMAFULL;
     DEFINE FIELD content ON memories TYPE string;
     DEFINE FIELD embedding ON memories TYPE array<float>;
     DEFINE INDEX idx_vec ON memories FIELDS embedding HNSW DIMENSION 384 DIST COSINE;
     ```
   - Insert 3 test records with 384-dim random vectors
   - Query: vector search with cosine similarity
   - Verify results return with similarity scores

**Pass Criteria**:
- DB creates in temp directory
- Schema applies without error
- HNSW index created (DIMENSION 384)
- Vector search returns results ordered by similarity

---

---

### T003a: Write tests for WP01 components

**Goal**: Ensure PoC components have basic automated verification.

**Implementation**:

1. Create basic tests for each PoC
   - Candle: Verify embedding dimensions and normalization
   - rmcp: Verify request/response handling
   - SurrealDB: Verify connection and basic CRUD

**Pass Criteria**:
- `cargo test` passes in PoC directories
- Automated verification of PoC functions

---

## Definition of Done

1. All 3 PoC binaries compile with `cargo build`
2. Each PoC runs successfully and produces expected output
3. Document any API quirks or gotchas discovered
4. If ANY PoC fails:
   - Document the failure
   - Propose alternative (e.g., different crate version, fallback approach)
   - Do NOT proceed to WP02 until resolved

## Risks

| Risk | Mitigation |
|------|------------|
| Candle API instability | Pin version 0.9.1, document any workarounds |
| rmcp macro changes | Fallback to manual ServerHandler impl if needed |
| SurrealDB HNSW bugs | Test with real vectors, not random, if issues |

## Parallel Opportunities

All 3 subtasks (T001, T002, T003) are completely independent and can be developed in parallel by different developers or executed concurrently.

## Reviewer Guidance

- Verify each PoC runs independently
- Check that pass criteria are objectively met
- Review any documented gotchas for implications on main implementation
- Confirm `_tmp/poc/` structure - NOT in main src/

## Activity Log

- 2026-01-06T16:22:37Z -- agent -- lane=doing -- Started implementation via workflow command
