# 🧠 Memory MCP Server

[![Release](https://github.com/pomazanbohdan/memory-mcp-1file/actions/workflows/release.yml/badge.svg)](https://github.com/pomazanbohdan/memory-mcp-1file/actions/workflows/release.yml)
[![Docker](https://img.shields.io/badge/docker-ghcr.io-blue.svg)](https://github.com/pomazanbohdan/memory-mcp-1file/pkgs/container/memory-mcp-1file)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Architecture](https://img.shields.io/badge/Architecture-Single%20Binary-success.svg)](#)

A high-performance **Model Context Protocol (MCP)** server that provides persistent, semantic, and graph-based memory for AI agents.

Works perfectly with:
*   **Claude Desktop**
*   **Claude Code** (CLI)
*   **Cursor**
*   **OpenCode**
*   **Cline** / **Roo Code**
*   Any other MCP-compliant client.

### 🏆 The "All-in-One" Advantage

Unlike other memory solutions that require a complex stack (Python + Vector DB + Graph DB), this project is **a single, self-contained executable**.

*   ✅ **No External Database** (SurrealDB is embedded)
*   ✅ **No Python Dependencies** (Embedding models run via embedded ONNX runtime)
*   ✅ **No API Keys Required** (All models run locally on CPU)
*   ✅ **Zero Setup** (Just run one Docker container or binary)

It combines:
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

### Universal Docker Configuration (Any IDE/CLI)

To use this MCP server with any client (**Claude Code**, **OpenCode**, **Cline**, etc.), use the following Docker command structure.

**Key Requirements:**
1.  **Memory Volume**: `-v mcp-data:/data` (Persists your graph and embeddings)
2.  **Project Volume**: `-v $(pwd):/project:ro` (Allows the server to read and index your code)
3.  **Init Process**: `--init` (Ensures the server shuts down cleanly)

#### JSON Configuration (Claude Desktop, etc.)

Add this to your configuration file (e.g., `claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "memory": {
      "command": "docker",
      "args": [
        "run",
        "--init",
        "-i",
        "--rm",
        "-v", "mcp-data:/data",
        "-v", "/absolute/path/to/your/project:/project:ro",
        "ghcr.io/pomazanbohdan/memory-mcp-1file:latest"
      ]
    }
  }
}
```

> **Note:** Replace `/absolute/path/to/your/project` with the actual path you want to index. In some environments (like Cursor or VSCode extensions), you might be able to use variables like `${workspaceFolder}`, but absolute paths are most reliable for Docker.

### Cursor (Specific Instructions)

1.  Go to **Cursor Settings** > **Features** > **MCP Servers**.
2.  Click **+ Add New MCP Server**.
3.  **Type**: `stdio`
4.  **Name**: `memory`
5.  **Command**:
    ```bash
    docker run --init -i --rm -v mcp-data:/data -v "/Users/yourname/projects/current:/project:ro" ghcr.io/pomazanbohdan/memory-mcp-1file:latest
    ```
    *(Remember to update the project path when switching workspaces if you need code indexing)*

### OpenCode / CLI

```bash
docker run --init -i --rm \
  -v mcp-data:/data \
  -v $(pwd):/project:ro \
  ghcr.io/pomazanbohdan/memory-mcp-1file:latest
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
