# Quickstart: Memory MCP Server

**Feature**: 001-memory-mcp-server
**Date**: 2026-01-06

---

## Prerequisites

- Rust 1.75+ (2021 edition)
- ~2GB disk space (for model cache)
- Linux x86_64 (primary), Windows/macOS (supported)

### Build Dependencies (Linux)

```bash
# Fedora/RHEL
sudo dnf install -y mold clang lld

# Ubuntu/Debian
sudo apt install -y mold clang lld

# For cross-compilation (optional)
sudo dnf install -y mingw64-gcc musl-gcc musl-libc-static
```

---

## Quick Start

### 1. Clone and Build

```bash
git clone <repository>
cd memory-mcp

# Development build
cargo build

# Release build (optimized)
cargo build --release
```

### 2. Run the Server

```bash
# Default configuration
./target/release/memory-mcp

# With custom model
./target/release/memory-mcp --model e5_small

# List available models
./target/release/memory-mcp --list-models
```

### 3. Test with MCP Client

```bash
# Initialize
echo '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","clientInfo":{"name":"test","version":"1.0"}},"id":1}' | ./target/release/memory-mcp

# List tools
echo '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":2}' | ./target/release/memory-mcp
```

---

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `MEMORY_MCP_DATA_DIR` | `~/.local/share/memory-mcp` | Data directory |
| `MEMORY_MCP_MODEL` | `e5_multi` | Embedding model |
| `MEMORY_MCP_CACHE_SIZE` | `1000` | LRU cache entries |
| `MEMORY_MCP_BATCH_SIZE` | `32` | Embedding batch size |
| `MEMORY_MCP_TIMEOUT` | `60` | Model load timeout (seconds) |
| `MEMORY_MCP_LOG_LEVEL` | `info` | Log level (error/warn/info/debug) |

### CLI Flags

```bash
memory-mcp [OPTIONS]

OPTIONS:
    --data-dir <PATH>      Data directory
    --model <MODEL>        Embedding model (e5_small, e5_multi, nomic, bge_m3)
    --cache-size <N>       LRU cache size
    --batch-size <N>       Embedding batch size
    --timeout <SECONDS>    Model load timeout
    --log-level <LEVEL>    Log level
    --list-models          Show available models and exit
    -h, --help             Show help
    -V, --version          Show version
```

---

## IDE Integration

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

## Docker

### Build

```bash
# Production (multi-stage, builds in container)
docker build -t memory-mcp:latest .

# Development (uses pre-built binary)
cargo build --release
docker build -f Dockerfile.local -t memory-mcp:dev .
```

### Run

```bash
# With persistent volume
docker run -i --rm -v memory-mcp-data:/data memory-mcp:latest

# With custom model
docker run -i --rm -v memory-mcp-data:/data \
  -e MEMORY_MCP_MODEL=e5_small \
  memory-mcp:latest
```

### Docker with OpenCode

```json
{
  "mcp": {
    "memory-mcp": {
      "type": "local",
      "command": ["docker", "run", "--rm", "-i", 
        "-v", "memory-mcp-data:/data", 
        "memory-mcp:dev"],
      "enabled": true
    }
  }
}
```

---

## Directory Structure

```
~/.local/share/memory-mcp/
└── db/                    # SurrealDB data files

~/.cache/huggingface/
└── hub/
    └── models--intfloat--multilingual-e5-base/  # Model cache
```

---

## Testing

```bash
# Run all tests
cargo test

# Run unit tests only
cargo test --lib

# Run integration tests
cargo test --test '*'

# Run tests requiring model download (slow)
cargo test -- --ignored

# Run specific test
cargo test test_memory_crud

# With logging
RUST_LOG=debug cargo test -- --nocapture
```

---

## Performance Targets

| Operation | Target Latency |
|-----------|----------------|
| `store_memory` | < 100ms |
| `search` (vector) | < 20ms |
| `search_text` (BM25) | < 30ms |
| `recall` (hybrid) | < 100ms |
| `index_project` (100 files) | < 5 min |
| Server startup | < 1s (model loads in background) |

---

## First Run Behavior

1. Server starts immediately (< 1s)
2. Embedding model downloads in background (1-2 min for e5_multi)
3. `get_status` shows `embedding.status: "loading"` during download
4. Tools requiring embeddings return error until model ready
5. `search_text` works immediately (BM25 only, no embeddings)

---

## Troubleshooting

### Model Download Fails

```bash
# Check HuggingFace cache
ls ~/.cache/huggingface/hub/

# Clear and retry
rm -rf ~/.cache/huggingface/hub/models--intfloat--*
./target/release/memory-mcp
```

### Database Corruption

```bash
# Remove and recreate
rm -rf ~/.local/share/memory-mcp/db/
./target/release/memory-mcp
```

### Embedding Service Not Ready

Wait for model to load (check `get_status`), or use smaller model:
```bash
./target/release/memory-mcp --model e5_small
```

---

## Next Steps

After setup:
1. Store some memories: `store_memory`
2. Search semantically: `search`
3. Build knowledge graph: `create_entity`, `create_relation`
4. Index a codebase: `index_project`

See `contracts/mcp-tools.md` for full API reference.
