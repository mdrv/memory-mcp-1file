---
work_package_id: WP02
title: "Core Foundation"
phase: "Phase 1"
priority: P0
subtasks: ["T004", "T005", "T006", "T007", "T008", "T009", "T010", "T011", "T012"]
lane: planned
dependencies: ["WP01"]
history:
  - date: 2026-01-06
    action: created
    by: spec-kitty.tasks
---

# WP02: Core Foundation

## Objective

Establish the project structure and implement all type definitions. After this work package, `cargo check` must pass.

## Context

This is the foundation for all other work packages. Types defined here will be used throughout storage, embedding, and server layers.

**Reference Documents**:
- `kitty-specs/001-memory-mcp-server/data-model.md` - Complete type definitions
- `kitty-specs/001-memory-mcp-server/plan.md` - Project structure

## Subtasks

### T004: Create Cargo.toml

**Location**: Project root `/Cargo.toml`

```toml
[package]
name = "memory-mcp"
version = "0.1.0"
edition = "2021"
description = "MCP memory server for AI agents"
license = "MIT"

[dependencies]
# Database
surrealdb = { version = "2", default-features = false, features = ["kv-surrealkv", "rustls"] }

# Embeddings
candle-core = "0.9.1"
candle-nn = "0.9.1"
candle-transformers = "0.9.1"
tokenizers = "0.22.2"
hf-hub = "0.3"

# MCP Protocol
rmcp = { version = "0.12.0", features = ["server", "transport-io", "macros"] }

# Graph
petgraph = "0.6"

# Code parsing
tree-sitter = "0.26"
code-splitter = "0.4"

# Async runtime
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Error handling
anyhow = "1"
thiserror = "2"

# CLI
clap = { version = "4", features = ["derive"] }

# Utilities
chrono = { version = "0.4", features = ["serde"] }
lru = "0.12"
blake3 = "1"
rayon = "1.10"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
tempfile = "3"

[profile.release]
lto = true
codegen-units = 1
strip = true
```

---

### T005: Create .cargo/config.toml

**Location**: `.cargo/config.toml`

```toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

[target.x86_64-unknown-linux-musl]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

---

### T006: Implement types/error.rs

**Location**: `src/types/error.rs`

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Embedding error: {0}")]
    Embedding(String),
    
    #[error("Embedding service not ready. Please try again.")]
    EmbeddingNotReady,
    
    #[error("Memory not found: {0}")]
    MemoryNotFound(String),
    
    #[error("Entity not found: {0}")]
    EntityNotFound(String),
    
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    #[error("Indexing error: {0}")]
    Indexing(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, AppError>;
```

---

### T007: Implement types/memory.rs

**Location**: `src/types/memory.rs`

Follow `data-model.md` exactly:
- `Memory` struct with all fields from data model
- `MemoryType` enum: Episodic, Semantic, Procedural
- `MemoryUpdate` struct for partial updates
- `#[serde(skip_serializing)]` on `embedding` field
- Default implementations where specified

**Critical**: The `embedding` field must NEVER be serialized in MCP responses.

---

### T008: Implement types/entity.rs

**Location**: `src/types/entity.rs`

Follow `data-model.md`:
- `Entity` struct
- `Relation` struct with `#[serde(rename = "in")]` for from_entity
- `Direction` enum: Outgoing, Incoming, Both
- `#[serde(skip_serializing)]` on `embedding` field

---

### T009: Implement types/code.rs

**Location**: `src/types/code.rs`

Follow `data-model.md`:
- `CodeChunk` struct
- `ChunkType` enum: Function, Class, Struct, Module, Impl, Other
- `Language` enum: Rust, Python, JavaScript, TypeScript, Go, Unknown
- `IndexStatus` struct
- `IndexState` enum: Indexing, Completed, Failed

---

### T010: Implement types/search.rs

**Location**: `src/types/search.rs`

Follow `data-model.md`:
- `SearchResult` struct (for vector/BM25)
- `RecallResult` struct (for hybrid)
- `ScoredMemory` struct with vector_score, bm25_score, ppr_score
- `CodeSearchResult` struct
- `ScoredCodeChunk` struct

---

### T011: Create config.rs

**Location**: `src/config.rs`

```rust
use std::path::PathBuf;
use std::sync::Arc;

use crate::embedding::EmbeddingService;
use crate::storage::SurrealStorage;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub data_dir: PathBuf,
    pub model: String,
    pub cache_size: usize,
    pub batch_size: usize,
    pub timeout_ms: u64,
    pub log_level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            data_dir: dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("memory-mcp"),
            model: "e5_multi".to_string(),
            cache_size: 1000,
            batch_size: 32,
            timeout_ms: 30000,
            log_level: "info".to_string(),
        }
    }
}

pub struct AppState {
    pub config: AppConfig,
    pub storage: Arc<SurrealStorage>,
    pub embedding: Arc<EmbeddingService>,
}
```

**Note**: Add `dirs = "5"` to Cargo.toml dependencies.

---

### T012: Create lib.rs

**Location**: `src/lib.rs`

```rust
pub mod config;
pub mod types;
pub mod storage;
pub mod embedding;
pub mod graph;
pub mod codebase;
pub mod server;

pub use config::{AppConfig, AppState};
pub use types::error::{AppError, Result};
```

Also create module files:
- `src/types/mod.rs` - re-exports all types
- `src/storage/mod.rs` - placeholder
- `src/embedding/mod.rs` - placeholder
- `src/graph/mod.rs` - placeholder
- `src/codebase/mod.rs` - placeholder
- `src/server/mod.rs` - placeholder

---

## Definition of Done

1. `cargo check` passes with no errors
2. All types from `data-model.md` are implemented
3. All `embedding` fields have `#[serde(skip_serializing)]`
4. Default implementations work correctly
5. No TODO comments left in type definitions

## Risks

| Risk | Mitigation |
|------|------------|
| SurrealDB `Thing` type compatibility | Use surrealdb::sql::Thing, import correctly |
| DateTime serialization format | Use chrono with serde feature, ISO 8601 format |

## Reviewer Guidance

- Verify all fields from `data-model.md` are present
- Check `#[serde(skip_serializing)]` on ALL embedding fields
- Confirm `cargo check` passes
- Review Default implementations for correctness
