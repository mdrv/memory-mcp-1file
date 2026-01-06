# Development Setup & Known Issues

## Build Configuration (Based on Actual Implementation)

### Cross-Compilation Targets

The project successfully builds for 3 targets:
- **Linux (glibc)**: `x86_64-unknown-linux-gnu`
- **Linux (musl/Alpine)**: `x86_64-unknown-linux-musl`  
- **Windows**: `x86_64-pc-windows-gnu`

### Linker Configuration

`.cargo/config.toml` is configured with:
- **Linux**: `mold` linker (requires: `sudo dnf install mold clang`)
- **Alpine**: `mold` linker (requires: `musl-gcc musl-libc-static`)
- **Windows**: `lld` linker (requires: `sudo dnf install lld mingw64-gcc`)

**Build command:**
```bash
cargo build --release
```
Outputs all 3 binaries simultaneously (~5-6 min build time).

---

## Dependency Versions (Verified Working)

```toml
[dependencies]
candle-core = "0.9.1"
candle-nn = "0.9.1"
candle-transformers = "0.9.1"
tokenizers = "0.22.2"
safetensors = "0.7.0"
dirs = "6.0.0"
lru = "0.16.2"
surrealdb = { version = "2", default-features = false, features = ["kv-surrealkv", "rustls"] }
rmcp = { version = "0.12.0", features = ["server", "transport-io", "macros"] }
```

---

## Common Build Issues & Solutions

### Issue 1: Corrupted Cargo Cache

**Symptoms:**
```
error: failed to unpack package `winapi-x86_64-pc-windows-gnu v0.4.0`
Caused by: invalid gzip header
```

**Solution:**
```bash
find ~/.cargo/registry/cache -name "winapi-x86_64-pc-windows-gnu*" -delete
find ~/.cargo/registry/cache -name "windows-*" -delete
find ~/.cargo/registry/src -name "winapi-x86_64-pc-windows-gnu*" -exec rm -rf {} +
cargo clean
cargo build --release
```

### Issue 2: Missing Linkers

**Symptoms:**
```
error: linking with `x86_64-w64-mingw32-gcc` failed
collect2: fatal error: cannot find 'ld'
```

**Solution (Fedora/RHEL):**
```bash
sudo dnf install -y lld mold clang musl-gcc musl-libc-static mingw64-gcc
```

### Issue 3: Compilation Errors After Cache Clear

**Symptom:**
```
error[E0560]: struct `types::memory::Memory` does not have field `embedding`
```

**Solution:**
```bash
cargo clean  # Clean stale build artifacts
cargo check  # Verify compilation
```

---

## Docker Development Setup

### Fast Development Build (Uses Pre-built Binary)

```dockerfile
# Dockerfile.local
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY target/release/memory-mcp-1file /usr/local/bin/memory-mcp-1file

RUN mkdir -p /data

ENV MEMORY_MCP_DATA_DIR=/data
ENV MEMORY_MCP_LOG_LEVEL=info

ENTRYPOINT ["memory-mcp-1file"]
```

**Build:** ~1 min vs ~4 min for production build

**Usage:**
```bash
cargo build --release
docker build -f Dockerfile.local -t memory-mcp:dev .
docker run -i --rm -v memory-data:/data memory-mcp:dev
```

---

## OpenCode MCP Configuration

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

## Project Structure (Actual vs Planned)

**Current State:**
```
memory-mcp-1file/
├── .cargo/config.toml     ✅ Configured for 3 targets
├── Cargo.toml             ❌ Missing - needs creation
├── src/                   ❌ Missing - needs implementation
├── doc/                   ✅ Complete documentation
└── opencode.json          ✅ MCP config
```

**Next Step:** Create `Cargo.toml` and `src/` structure according to design docs.

---

## Testing MCP Server

### Initialization Sequence
```jsonc
// 1. Initialize
{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","clientInfo":{"name":"test","version":"1.0"}},"id":1}

// 2. Notify initialized
{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}

// 3. List tools
{"jsonrpc":"2.0","method":"tools/list","params":{},"id":2}

// 4. Call tool
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_status","arguments":{}},"id":3}
```

### Expected `get_status` Response
```json
{
  "version": "0.1.0",
  "status": "healthy",
  "memories_count": 0,
  "embedding": {
    "status": "loading",  // or "ready"
    "model": "e5_multi",
    "dimensions": 768
  }
}
```

---

## Model Download Locations

```bash
# Linux/Mac
~/.cache/huggingface/hub/models--intfloat--multilingual-e5-base/

# Windows  
%LOCALAPPDATA%\huggingface\hub\models--intfloat--multilingual-e5-base\
```

**First run:** Model downloads automatically (1.1 GB for e5_multi), takes 1-2 min.

---

## Performance Benchmarks (Expected)

| Operation | Target Latency |
|-----------|----------------|
| `store_memory` | 50-100ms |
| `search` (vector) | 10-20ms |
| `search_text` (BM25) | 15-30ms |
| `recall` (hybrid) | 50-100ms |
| `index_project` (100 files) | 2-5 min |

---

## CRITICAL: Agent Rules

From `AGENTS.md`:
- ❌ **NEVER** use root directory for external repositories
- ✅ **ALWAYS** use `_tmp/` directory for external repos

---

*Last updated: 2026-01-06 (based on actual build sessions)*
