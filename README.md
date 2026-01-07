# 🧠 Memory MCP Server

[![Release](https://github.com/pomazanbohdan/memory-mcp-1file/actions/workflows/release.yml/badge.svg)](https://github.com/pomazanbohdan/memory-mcp-1file/actions/workflows/release.yml)
[![Docker](https://img.shields.io/badge/docker-ghcr.io-blue.svg)](https://github.com/pomazanbohdan/memory-mcp-1file/pkgs/container/memory-mcp-1file)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A high-performance **Model Context Protocol (MCP)** server that provides persistent, semantic, and graph-based memory for AI agents (Claude, Cursor, etc.).

It goes beyond simple file storage by combining:
1.  **Vector Search** (FastEmbed) for semantic similarity.
2.  **Knowledge Graph** (PetGraph) for entity relationships.
3.  **Code Indexing** for understanding your codebase.
4.  **Hybrid Retrieval** (Reciprocal Rank Fusion) for best results.

---

## 🚀 Quick Start

### Option 1: Docker (Recommended)

No installation required. Run directly from GitHub Container Registry.

**Interactive Run (Test):**
```bash
# Create a volume for persistent data
docker volume create mcp-data

# Run
docker run -i --rm -v mcp-data:/data ghcr.io/pomazanbohdan/memory-mcp-1file:latest
```

### Option 2: Local Binary

If you have Rust installed:

```bash
cargo install --path .
memory-mcp
```

---

## 🔌 Client Configuration

### Claude Desktop

Add this to your `claude_desktop_config.json` (usually in `~/Library/Application Support/Claude/` on macOS or `%APPDATA%\Claude\` on Windows).

**Using Docker (Easiest):**
```json
{
  "mcpServers": {
    "memory": {
      "command": "docker",
      "args": [
        "run",
        "-i",
        "--rm",
        "-v",
        "mcp-data:/data",
        "ghcr.io/pomazanbohdan/memory-mcp-1file:latest"
      ]
    }
  }
}
```

**Using Local Binary:**
```json
{
  "mcpServers": {
    "memory": {
      "command": "memory-mcp",
      "args": ["--data-dir", "/Users/yourname/.local/share/memory-mcp"]
    }
  }
}
```

### Cursor (IDE)

1.  Go to **Cursor Settings** > **Features** > **MCP Servers**.
2.  Click **+ Add New MCP Server**.
3.  **Type**: `stdio`
4.  **Name**: `memory`
5.  **Command**:
    ```bash
    docker run -i --rm -v mcp-data:/data ghcr.io/pomazanbohdan/memory-mcp-1file:latest
    ```

---

## ✨ Key Features

- **Semantic Memory**: Stores text with vector embeddings (`e5-small` by default) for "vibe-based" retrieval.
- **Graph Memory**: Tracks entities (`User`, `Project`, `Tech`) and their relations (`uses`, `likes`). Supports PageRank-based traversal.
- **Code Intelligence**: Indexes local project directories (AST-based chunking) to answer questions about your code.
- **Temporal Validity**: Memories can have `valid_from` and `valid_until` dates.
- **SurrealDB Backend**: Fast, embedded, single-file database.

---

## 🛠️ Tools Available

The server exposes **21 tools** to the AI model.

### 🧠 Core Memory
| Tool | Description |
|------|-------------|
| `store_memory` | Store a new memory with content and optional metadata. |
| `recall` | **Hybrid search** (Vector + Keyword + Graph). Best for general questions. |
| `search` | Pure vector search. |
| `search_text` | Exact keyword match (BM25). |
| `get_valid` | Get currently active memories (filters out expired ones). |

### 🕸️ Knowledge Graph
| Tool | Description |
|------|-------------|
| `create_entity` | Define a node (e.g., "React", "Authentication"). |
| `create_relation` | Link nodes (e.g., "Project" -> "uses" -> "React"). |
| `get_related` | Find connected concepts. |

### 💻 Codebase
| Tool | Description |
|------|-------------|
| `index_project` | Scan a local folder for code. |
| `search_code` | Semantic search over code chunks. |

---

## ⚙️ Configuration

Environment variables or CLI args:

| Arg | Env | Default | Description |
|-----|-----|---------|-------------|
| `--data-dir` | `DATA_DIR` | `./data` | DB location |
| `--model` | `EMBEDDING_MODEL` | `e5_multi` | Embedding model (`e5_small`, `e5_multi`, `nomic`, `bge_m3`) |
| `--log-level` | `LOG_LEVEL` | `info` | Verbosity |

## License

MIT
