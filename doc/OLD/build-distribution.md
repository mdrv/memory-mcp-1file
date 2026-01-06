# Build & Distribution — Design Document

## Overview

Build configuration and distribution strategy for memory-mcp single binary.

## Binary Size Target

| Component | Estimated Size |
|-----------|---------------|
| SurrealDB (embedded) | ~15-20 MB |
| Candle (ML runtime) | ~5 MB |
| MCP + other deps | ~5 MB |
| **Total** | **~25-30 MB** |

Note: Models are downloaded separately (~134 MB - 2.3 GB).

## Build Configuration

### Cargo.toml

```toml
[package]
name = "memory-mcp"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <you@example.com>"]
description = "Self-contained memory server for AI agents"
license = "MIT"
repository = "https://github.com/user/memory-mcp"
keywords = ["mcp", "memory", "ai", "vector-search"]
categories = ["command-line-utilities", "database"]

[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# Database
surrealdb = { version = "2", default-features = false, features = ["kv-surrealkv"] }

# Embeddings (pure Rust)
candle-core = "0.8"
candle-nn = "0.8"
candle-transformers = "0.8"
hf-hub = "0.3"
tokenizers = "0.21"

# MCP Protocol
rmcp = { version = "0.1", features = ["server", "macros", "transport-io"] }

# Graph
petgraph = "0.6"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# CLI
clap = { version = "4", features = ["derive", "env"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Error handling
anyhow = "1"
thiserror = "2"

# Utils
chrono = { version = "0.4", features = ["serde"] }
dirs = "5"

# Code parsing
tree-sitter = "0.26"
code-splitter = "0.4"

# File operations
walkdir = "2"
ignore = "0.4"
notify = "8"
blake3 = "1"

# Parallelism
rayon = "1.10"

# Caching
lru = "0.12"

[dev-dependencies]
tempfile = "3"
tokio-test = "0.4"
criterion = { version = "0.5", features = ["async_tokio"] }

[features]
default = []
cuda = ["candle-core/cuda"]  # GPU acceleration

[profile.release]
lto = "thin"
codegen-units = 1
opt-level = 3
strip = true

[profile.release-small]
inherits = "release"
lto = "fat"
opt-level = "z"
panic = "abort"
```

### .cargo/config.toml

```toml
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "target-cpu=native"]

[target.x86_64-apple-darwin]
rustflags = ["-C", "target-cpu=native"]

[target.aarch64-apple-darwin]
rustflags = ["-C", "target-cpu=native"]

[build]
# Parallel compilation
jobs = 8
```

## Build Commands

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Small release build
cargo build --profile release-small

# With GPU support
cargo build --release --features cuda

# Cross-compile for Linux (from macOS)
cross build --release --target x86_64-unknown-linux-gnu
```

## Cross-Compilation

### Using cross

```bash
# Install cross
cargo install cross

# Build for Linux
cross build --release --target x86_64-unknown-linux-gnu
cross build --release --target aarch64-unknown-linux-gnu

# Build for Windows
cross build --release --target x86_64-pc-windows-gnu
```

### Cross.toml

```toml
[build.env]
passthrough = [
    "RUST_BACKTRACE",
]

[target.x86_64-unknown-linux-gnu]
image = "ghcr.io/cross-rs/x86_64-unknown-linux-gnu:main"

[target.aarch64-unknown-linux-gnu]
image = "ghcr.io/cross-rs/aarch64-unknown-linux-gnu:main"
```

## Docker

### Dockerfile (Production)

Multi-stage Alpine build that compiles in container:

```dockerfile
# Build stage
FROM rust:1.83-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy src to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# Copy real source
COPY src ./src

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

ENV MEMORY_MCP_DATA_DIR=/data
ENV MEMORY_MCP_LOG_LEVEL=info

ENTRYPOINT ["memory-mcp"]
```

### Docker Compose

```yaml
version: '3.8'

services:
  memory-mcp:
    build: .
    image: memory-mcp:latest
    volumes:
      - memory-data:/data
      - hf-cache:/root/.cache/huggingface
    environment:
      MEMORY_MCP_DATA_DIR: /data
      MEMORY_MCP_MODEL: e5_multi
      MEMORY_MCP_LOG_LEVEL: info
    stdin_open: true
    tty: true

volumes:
  memory-data:
  hf-cache:
```

### Dockerfile.local (Development)

Uses pre-built local binary for fast iteration (~1 min vs ~4 min):

```dockerfile
# Dockerfile.local - uses pre-built binary from host
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy pre-built binary from host
COPY target/release/memory-mcp-1file /usr/local/bin/memory-mcp-1file

RUN mkdir -p /data

ENV MEMORY_MCP_DATA_DIR=/data
ENV MEMORY_MCP_LOG_LEVEL=info

ENTRYPOINT ["memory-mcp-1file"]
```

### .dockerignore

```
target/
!target/release/memory-mcp-1file
.git/
*.md
doc/
```

### Building Docker Image

```bash
# Production build (compiles in container)
docker build -t memory-mcp:latest .

# Development build (uses local binary - faster)
cargo build --release
docker build -f Dockerfile.local -t memory-mcp:dev .

# Run
docker run -i --rm \
  -v memory-data:/data \
  -v hf-cache:/root/.cache/huggingface \
  memory-mcp:dev

# Push to registry
docker tag memory-mcp:latest ghcr.io/user/memory-mcp:latest
docker push ghcr.io/user/memory-mcp:latest
```

## NPM Wrapper

### package.json

```json
{
  "name": "@anthropic/memory-mcp",
  "version": "0.1.0",
  "description": "Self-contained memory server for AI agents",
  "bin": {
    "memory-mcp": "./bin/memory-mcp"
  },
  "scripts": {
    "postinstall": "node scripts/install.js"
  },
  "files": [
    "bin/",
    "scripts/"
  ],
  "os": ["darwin", "linux", "win32"],
  "cpu": ["x64", "arm64"],
  "keywords": ["mcp", "memory", "ai", "claude"],
  "license": "MIT"
}
```

### scripts/install.js

```javascript
const https = require('https');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const VERSION = process.env.npm_package_version || '0.1.0';
const PLATFORM = process.platform;
const ARCH = process.arch;

const BINARIES = {
  'darwin-x64': `memory-mcp-${VERSION}-darwin-x64`,
  'darwin-arm64': `memory-mcp-${VERSION}-darwin-arm64`,
  'linux-x64': `memory-mcp-${VERSION}-linux-x64`,
  'linux-arm64': `memory-mcp-${VERSION}-linux-arm64`,
  'win32-x64': `memory-mcp-${VERSION}-windows-x64.exe`,
};

const key = `${PLATFORM}-${ARCH}`;
const binary = BINARIES[key];

if (!binary) {
  console.error(`Unsupported platform: ${key}`);
  process.exit(1);
}

const url = `https://github.com/user/memory-mcp/releases/download/v${VERSION}/${binary}`;
const binDir = path.join(__dirname, '..', 'bin');
const binPath = path.join(binDir, PLATFORM === 'win32' ? 'memory-mcp.exe' : 'memory-mcp');

fs.mkdirSync(binDir, { recursive: true });

console.log(`Downloading memory-mcp for ${key}...`);

const file = fs.createWriteStream(binPath);
https.get(url, (response) => {
  if (response.statusCode === 302 || response.statusCode === 301) {
    https.get(response.headers.location, (r) => r.pipe(file));
  } else {
    response.pipe(file);
  }
  
  file.on('finish', () => {
    file.close();
    if (PLATFORM !== 'win32') {
      fs.chmodSync(binPath, 0o755);
    }
    console.log('memory-mcp installed successfully!');
  });
}).on('error', (err) => {
  console.error(`Download failed: ${err.message}`);
  process.exit(1);
});
```

### bin/memory-mcp (wrapper script)

```bash
#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');

const binary = path.join(__dirname, process.platform === 'win32' ? 'memory-mcp.exe' : 'memory-mcp');

const child = spawn(binary, process.argv.slice(2), {
  stdio: 'inherit',
});

child.on('exit', (code) => {
  process.exit(code);
});
```

## GitHub Releases

### Release Workflow

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            name: linux-x64
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            name: linux-arm64
          - os: macos-latest
            target: x86_64-apple-darwin
            name: darwin-x64
          - os: macos-latest
            target: aarch64-apple-darwin
            name: darwin-arm64
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            name: windows-x64
    
    runs-on: ${{ matrix.os }}
    
    steps:
      - uses: actions/checkout@v4
      
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      
      - name: Install cross (Linux ARM)
        if: matrix.name == 'linux-arm64'
        run: cargo install cross
      
      - name: Build
        run: |
          if [ "${{ matrix.name }}" = "linux-arm64" ]; then
            cross build --release --target ${{ matrix.target }}
          else
            cargo build --release --target ${{ matrix.target }}
          fi
        shell: bash
      
      - name: Package
        run: |
          cd target/${{ matrix.target }}/release
          if [ "${{ matrix.os }}" = "windows-latest" ]; then
            7z a ../../../memory-mcp-${{ matrix.name }}.zip memory-mcp.exe
          else
            tar czf ../../../memory-mcp-${{ matrix.name }}.tar.gz memory-mcp
          fi
        shell: bash
      
      - uses: actions/upload-artifact@v4
        with:
          name: memory-mcp-${{ matrix.name }}
          path: memory-mcp-${{ matrix.name }}.*

  release:
    needs: build
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/download-artifact@v4
      
      - uses: softprops/action-gh-release@v1
        with:
          files: |
            memory-mcp-*/memory-mcp-*
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

## Installation Methods

### Direct Download

```bash
# Linux x64
curl -L https://github.com/user/memory-mcp/releases/latest/download/memory-mcp-linux-x64.tar.gz | tar xz
sudo mv memory-mcp /usr/local/bin/

# macOS
curl -L https://github.com/user/memory-mcp/releases/latest/download/memory-mcp-darwin-arm64.tar.gz | tar xz
sudo mv memory-mcp /usr/local/bin/
```

### Cargo Install

```bash
cargo install memory-mcp
```

### NPM

```bash
npx @anthropic/memory-mcp
```

### Docker

```bash
docker run -i ghcr.io/user/memory-mcp:latest
```

### Homebrew (Future)

```bash
brew install user/tap/memory-mcp
```

## Data Directory Layout

```
~/.local/share/memory-mcp/    # Linux
~/Library/Application Support/memory-mcp/  # macOS
%APPDATA%/memory-mcp/         # Windows
├── db/                       # SurrealDB data
│   └── ...
└── config.json               # Optional user config

~/.cache/huggingface/         # Shared model cache
└── hub/
    └── models--intfloat--multilingual-e5-base/
        └── ...
```

## OpenCode MCP Configuration

### opencode.json

```json
{
  "mcp": {
    "memory-1file": {
      "type": "local",
      "command": ["docker", "run", "-i", "--rm", 
        "-v", "memory-data:/data", 
        "-v", "hf-cache:/root/.cache/huggingface", 
        "-e", "MEMORY_MCP_LOG_LEVEL=warn", 
        "memory-mcp:dev"
      ],
      "enabled": true
    }
  }
}
```

### Or direct binary:

```json
{
  "mcp": {
    "memory-1file": {
      "type": "local",
      "command": ["/path/to/memory-mcp-1file"],
      "enabled": true
    }
  }
}
```

## Background Model Loading

The embedding model loads asynchronously in background on startup. Server starts immediately and responds to MCP requests while model loads.

### Architecture

```
EmbeddingService
├── new() → spawns background loading task
├── load_handle: Mutex<Option<JoinHandle>> — tracks loading task
├── wait_load_complete() — awaits JoinHandle (for shutdown)
├── wait_ready() — waits until state == Ready
└── State: Unloaded → Loading → Ready/Failed

main.rs
├── Start SurrealDB
├── Create EmbeddingService (starts background load)
├── Start MCP server (doesn't wait for model)
├── Wait for MCP to close
└── Wait for model loading to complete (graceful shutdown)
```

### Healthcheck

Use `get_status` tool to check embedding status:

```json
{"status": "healthy", "embedding": {"status": "loading"}}
// or
{"status": "healthy", "embedding": {"status": "ready", "model": "e5_multi"}}
```

## Build Profile Optimization

For faster incremental builds:

```toml
[profile.release]
lto = "thin"        # Was: lto = true (slow)
codegen-units = 16  # Was: codegen-units = 1 (slow)
opt-level = 3
strip = true
```

Build time: ~4 min → ~1 min for incremental builds.
