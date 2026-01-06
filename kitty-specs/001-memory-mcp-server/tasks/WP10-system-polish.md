---
work_package_id: WP10
title: "System Tool + Polish"
phase: "Phase 9"
priority: P3
subtasks: ["T063", "T064", "T065", "T066", "T067", "T068", "T069"]
lane: planned
dependencies: ["WP07", "WP08", "WP09"]
history:
  - date: 2026-01-06
    action: created
    by: spec-kitty.tasks
---

# WP10: System Tool + Polish

## Objective

Add the final system status tool, create Docker images, clean up error messages, improve logging, and validate the full test suite.

## Context

This is the final work package. After completion, the server is production-ready.

## Subtasks

### T063: Implement tool: get_status

```rust
    /// Get server status and statistics. Returns version, memory count, and health info.
    #[tool(description = "Get server status and statistics. Returns version, memory count, and health info.")]
    async fn get_status(&self) -> Result<CallToolResult, McpError> {
        let memories_count = self.state.storage.count_memories().await
            .map_err(|e| McpError::internal(e.to_string()))?;
        
        let embedding_status = self.state.embedding.status();
        let cache_stats = self.state.embedding.cache_stats();
        let model = self.state.embedding.model();
        
        let db_healthy = self.state.storage.health_check().await.unwrap_or(false);
        
        Ok(CallToolResult::success(serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "status": if db_healthy { "healthy" } else { "degraded" },
            "memories_count": memories_count,
            "embedding": {
                "status": embedding_status,
                "model": format!("{:?}", model).to_lowercase(),
                "dimensions": model.dimensions(),
                "cache_stats": cache_stats
            }
        })))
    }
```

---

### T064: Create Dockerfile (production)

**Location**: `Dockerfile`

```dockerfile
# Multi-stage production build
FROM rust:1.75-bookworm AS builder

WORKDIR /app
COPY . .

# Install mold linker for faster builds
RUN apt-get update && apt-get install -y mold

# Build release
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/memory-mcp /usr/local/bin/

# Create data directory
RUN mkdir -p /data

ENV DATA_DIR=/data
ENV RUST_LOG=info

ENTRYPOINT ["memory-mcp"]
CMD ["--data-dir", "/data"]
```

---

### T065: Create Dockerfile.local (development)

**Location**: `Dockerfile.local`

```dockerfile
# Development build - uses pre-built binary
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy pre-built binary
COPY target/release/memory-mcp /usr/local/bin/

RUN mkdir -p /data

ENV DATA_DIR=/data
ENV RUST_LOG=info

ENTRYPOINT ["memory-mcp"]
CMD ["--data-dir", "/data"]
```

**Build Instructions**:
```bash
# First build locally
cargo build --release

# Then build dev image
docker build -f Dockerfile.local -t memory-mcp:dev .
```

---

### T066: Error message cleanup

Review all error messages across the codebase and ensure consistency:

**Patterns to follow**:
- `"Memory not found: {id}"` - for missing resources
- `"Embedding service not ready. Please try again."` - for loading states
- `"Database error. Please try again."` - for transient DB issues
- `"Invalid path: {path}"` - for file system errors
- `"Invalid timestamp format. Use ISO 8601 (e.g., 2024-01-15T10:30:00Z)"` - for parsing errors

**Rules**:
1. User-facing errors should be helpful, not stack traces
2. Log full details to stderr, return friendly message to tool
3. Include the problematic value in the message when safe
4. End with period, no trailing newlines

---

### T067: Logging improvements

Ensure all logging goes to stderr (stdout reserved for MCP protocol):

```rust
// In main.rs
tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(cli.log_level.parse().unwrap_or(tracing::Level::INFO.into())))
    .with_writer(std::io::stderr)  // CRITICAL: stderr only
    .with_target(false)
    .with_ansi(false)  // No color codes in logs
    .init();
```

**Log levels**:
- `info` - Startup, shutdown, model loading status
- `warn` - Recoverable errors, deprecation warnings
- `error` - Failures that affect functionality
- `debug` - Request/response details
- `trace` - Very verbose, embedding vectors, etc.

---

### T068: Create README.md

**Location**: `README.md`

```markdown
# Memory MCP Server

Self-contained MCP memory server for AI agents. Single binary, offline-first, privacy-focused.

## Features

- **20 MCP Tools**: Memory CRUD, semantic search, knowledge graph, temporal queries, code search
- **Offline Embeddings**: Candle-based (pure Rust), no API calls
- **Hybrid Search**: Vector (HNSW) + BM25 + Personalized PageRank
- **Code Indexing**: Tree-sitter AST chunking, semantic code search

## Installation

### From Source

```bash
cargo install --path .
```

### Docker

```bash
docker run -v ~/.memory-mcp:/data ghcr.io/your-org/memory-mcp:latest
```

## Usage

```bash
# Start server (stdio transport)
memory-mcp

# With custom model
memory-mcp --model e5_small

# List available models
memory-mcp --list-models
```

## MCP Client Configuration

### Claude Desktop

```json
{
  "mcpServers": {
    "memory": {
      "command": "memory-mcp",
      "args": ["--data-dir", "~/.memory-mcp"]
    }
  }
}
```

## Available Tools

| Tool | Description |
|------|-------------|
| store_memory | Store a new memory |
| get_memory | Retrieve memory by ID |
| update_memory | Update existing memory |
| delete_memory | Delete memory |
| list_memories | List with pagination |
| search | Vector similarity search |
| search_text | BM25 keyword search |
| recall | Hybrid search (vector + BM25 + PPR) |
| create_entity | Create knowledge graph entity |
| create_relation | Create graph relation |
| get_related | Traverse graph |
| get_valid | Currently valid memories |
| get_valid_at | Memories valid at timestamp |
| invalidate | Soft-delete memory |
| index_project | Index codebase |
| search_code | Semantic code search |
| get_index_status | Check indexing progress |
| list_projects | List indexed projects |
| delete_project | Remove indexed project |
| get_status | Server health status |

## Models

| Model | Dimensions | Size | Use Case |
|-------|------------|------|----------|
| e5_small | 384 | 134 MB | Fast, English |
| e5_multi | 768 | 1.1 GB | Default, multilingual |
| nomic | 768 | 1.9 GB | Apache 2.0 license |
| bge_m3 | 1024 | 2.3 GB | Long context (8K) |

## License

MIT
```

---

### T069: E2E test suite validation

Run the complete test suite and ensure 84+ tests pass:

```bash
# Run all tests
cargo test

# With verbose output
cargo test -- --nocapture

# Specific test categories
cargo test storage_test
cargo test embedding_test
cargo test graph_test
cargo test handler_test
cargo test e2e_test
```

**Test coverage targets**:
- Storage CRUD: 15+ tests
- Vector search: 5+ tests
- BM25 search: 5+ tests
- Graph traversal: 8+ tests
- Temporal queries: 6+ tests
- MCP handlers: 20+ tests (one per tool)
- E2E integration: 25+ tests

Document any pre-existing test failures or skipped tests.

---

## Definition of Done

1. get_status tool returns accurate health information
2. Docker image builds for linux/amd64
3. Binary size < 30 MB
4. All error messages follow consistent pattern
5. Logs go to stderr only
6. README documents all 20 tools
7. 84+ tests pass (document any exceptions)

## Risks

| Risk | Mitigation |
|------|------------|
| Docker build issues | Test both Dockerfile variants |
| Test flakiness | Use tempdir, avoid time-dependent tests |
| Binary size bloat | Enable LTO, strip symbols |

## Reviewer Guidance

- Verify no logs to stdout (breaks MCP protocol)
- Check README accuracy against actual tool schemas
- Run test suite, note any failures
- Confirm Docker images work with volume mounts
