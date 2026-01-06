# Research: Memory MCP Server

**Feature**: 001-memory-mcp-server
**Date**: 2026-01-06
**Purpose**: Phase 0 research findings and technology validation

## Executive Summary

All technical decisions from `doc/OLD/` have been validated. No NEEDS CLARIFICATION items remain. This research consolidates existing design documents into actionable implementation guidance.

---

## 1. Embedding Framework: Candle

### Decision
Use **Candle** (pure Rust) for embeddings instead of ONNX Runtime.

### Rationale
- Pure Rust: No Python/C++ dependencies, single binary distribution
- HuggingFace native: Direct model loading from Hub
- CPU-first: Works without GPU, optional CUDA support
- Actively maintained by HuggingFace team

### Alternatives Considered
| Option | Rejected Because |
|--------|------------------|
| ONNX Runtime | Requires C++ runtime, complex build |
| rust-bert | Heavier, more dependencies |
| Ollama API | External service, not self-contained |
| OpenAI API | Cloud dependency, privacy concerns |

### Implementation Notes
```rust
// Cargo.toml
candle-core = "0.9.1"
candle-nn = "0.9.1"
candle-transformers = "0.9.1"
tokenizers = "0.22.2"
hf-hub = "0.3"
```

**Critical**: Use `BertModel` for E5/BGE models. Mean pooling + L2 normalization required.

---

## 2. Database: SurrealDB Embedded

### Decision
Use **SurrealDB** with embedded `kv-surrealkv` backend.

### Rationale
- Single file storage: No external database server
- Native HNSW: Vector similarity built-in
- Native BM25: Full-text search built-in
- Graph relations: First-class RELATE syntax
- Pure Rust: Compiles into single binary

### Alternatives Considered
| Option | Rejected Because |
|--------|------------------|
| SQLite + pgvector | Requires extension, more complex |
| LanceDB | Less mature graph support |
| Milvus/Qdrant | External service, not embedded |
| Redis | Persistence complexity |

### Implementation Notes
```rust
// Cargo.toml
surrealdb = { version = "2", default-features = false, features = ["kv-surrealkv", "rustls"] }
```

**Critical**: HNSW index must specify DIMENSION matching model (768 for e5_multi).

---

## 3. MCP Protocol: rmcp

### Decision
Use **rmcp** crate with `tool_router` macro pattern.

### Rationale
- Official Rust SDK for MCP
- Macro-based tool registration reduces boilerplate
- stdio transport built-in
- Active development, MCP spec compliant

### Alternatives Considered
| Option | Rejected Because |
|--------|------------------|
| Manual JSON-RPC | Too much boilerplate |
| mcp-rust (unofficial) | Less maintained |
| Python SDK | Different language |

### Implementation Notes
```rust
// Cargo.toml
rmcp = { version = "0.12.0", features = ["server", "transport-io", "macros"] }
```

**Pattern**:
```rust
#[tool_router]
impl MemoryMcpServer {
    #[tool(description = "Store a memory")]
    async fn store_memory(&self, content: String) -> Result<CallToolResult, McpError> {
        // ...
    }
}
```

---

## 4. Embedding Models

### Decision
Support 4 models with `e5_multi` as default.

### Model Comparison
| Model | ID | Dims | Size | Use Case |
|-------|-----|------|------|----------|
| E5 Small | `e5_small` | 384 | 134 MB | Fast, English |
| E5 Multi | `e5_multi` | 768 | 1.1 GB | **Default**, multilingual |
| Nomic | `nomic` | 768 | 1.9 GB | Apache 2.0 license |
| BGE-M3 | `bge_m3` | 1024 | 2.3 GB | Long context (8K) |

### HuggingFace Repo IDs
```rust
match model {
    E5Small => "intfloat/multilingual-e5-small",
    E5Multi => "intfloat/multilingual-e5-base",
    Nomic => "nomic-ai/nomic-embed-text-v1.5",
    BgeM3 => "BAAI/bge-m3",
}
```

---

## 5. Hybrid Search Algorithm

### Decision
Use RRF (Reciprocal Rank Fusion) + PPR (Personalized PageRank).

### Algorithm
```
1. Vector HNSW -> top 50
2. BM25 FTS -> top 50
3. RRF Merge (k=60) -> top 20 seeds
4. Hub Dampening: weight = score / sqrt(degree)
5. PPR Diffusion (damping=0.5, max_iter=15)
6. Final: 0.40*vec + 0.15*bm25 + 0.45*ppr
```

### Rationale
- RRF is robust to score scale differences
- PPR with 0.5 damping (HippoRAG) gives better associative recall than 0.85
- Hub dampening prevents over-connected nodes from dominating

---

## 6. Code Chunking Strategy

### Decision
Use **tree-sitter** + **code-splitter** for AST-aware chunking.

### Rationale
- Preserves semantic boundaries (functions, classes)
- Context injection: parent scope name added to chunk
- Language-aware: Different grammars per language

### Supported Languages
Rust, Python, JavaScript, TypeScript, Go (via tree-sitter grammars)

### Fallback
Fixed-size chunking (100 lines) for unsupported languages.

### Implementation Notes
```rust
// Cargo.toml
tree-sitter = "0.26"
code-splitter = "0.4"
```

---

## 7. Caching Strategy

### Decision
Two-tier caching: L1 in-memory LRU, L2 in database.

### L1 Cache (EmbeddingCache)
- Size: 1000 entries (configurable)
- Key: blake3(normalize(text) || model_version)
- Eviction: LRU

### L2 Cache (SurrealDB)
- Embeddings stored with content
- Re-embedding only on content change

### Rationale
- Search queries hit L1 frequently (same query patterns)
- Stored memories don't need re-embedding

---

## 8. Error Handling Pattern

### Decision
Two-tier error model per MCP spec.

| Type | When | Return |
|------|------|--------|
| Tool Error | Logic error (not found, invalid) | `Ok(CallToolResult::error(...))` |
| Protocol Error | Invalid request, server crash | `Err(McpError)` |

### Rationale
- MCP spec requires tool errors be recoverable
- LLM can retry tool errors with different input
- Protocol errors are fatal

---

## 9. Token Protection

### Decision
Never serialize embeddings in MCP responses.

### Implementation
```rust
#[serde(skip_serializing)]
pub embedding: Option<Vec<f32>>,
```

### Rationale
- 768 floats * 4 bytes = 3KB per result
- At 50 results = 150KB wasted tokens
- LLM doesn't need embeddings, only content

---

## 10. Background Model Loading

### Decision
Load embedding model asynchronously, non-blocking startup.

### Implementation
```rust
pub struct EmbeddingService {
    engine: Arc<RwLock<Option<EmbeddingEngine>>>,
    status: Arc<AtomicU8>,  // 0=Loading, 1=Ready, 2=Error
}

// Startup: spawn_blocking for model load
// Tools: wait_ready() with timeout
```

### Rationale
- Server responds to tools/list within 1 second
- Model download/load can take 10-60 seconds
- get_status shows embedding.status for readiness check

---

## 11. File Watching (Deferred)

### Decision
File watching for auto-reindex is **optional/deferred**.

### Rationale
- Core functionality works without it
- Adds complexity (debouncing, event handling)
- Can be added in Phase 9+ if needed

### If Implemented
```rust
// Cargo.toml
notify = "8"
```

---

## 12. Cross-Compilation

### Decision
Build for 3 targets: Linux (glibc), Linux (musl), Windows.

### Configuration (.cargo/config.toml)
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

### Build Dependencies (Fedora)
```bash
sudo dnf install mold clang lld mingw64-gcc musl-gcc musl-libc-static
```

---

## Validation Checklist

| Item | Status | Notes |
|------|--------|-------|
| Candle embedding | Verified in doc/OLD | e5_small tested |
| SurrealDB HNSW | Verified in doc/OLD | 768d index works |
| rmcp tool_router | Verified in doc/OLD | 20 tools registered |
| RRF + PPR | Algorithm defined | Weights tuned |
| tree-sitter | Verified in doc/OLD | Rust/Python/JS tested |
| Background loading | Pattern defined | Non-blocking startup |
| Docker build | Pattern defined | Multi-stage + local |

---

## References

Source documents from `doc/OLD/`:
- ARCHITECTURE.md - High-level design
- MCP-API-REFERENCE.md - All 20 tools documented
- embedding-pipeline.md - Candle integration details
- hybrid-retrieval.md - RRF + PPR algorithm
- storage-backend.md - SurrealDB schema
- code-search.md - Tree-sitter chunking
- graph-traversal.md - PPR implementation
- mcp-server.md - rmcp patterns
- testing-strategy.md - Test categories
- DEVELOPMENT.md - Build configuration
