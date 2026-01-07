# 🧠 Memory MCP Server

[![Release](https://github.com/pomazanbohdan/memory-mcp-1file/actions/workflows/release.yml/badge.svg)](https://github.com/pomazanbohdan/memory-mcp-1file/actions/workflows/release.yml)
[![Docker](https://img.shields.io/badge/docker-ghcr.io-blue.svg)](https://github.com/pomazanbohdan/memory-mcp-1file/pkgs/container/memory-mcp-1file)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-d64e25.svg)](https://www.rust-lang.org)
[![Architecture](https://img.shields.io/badge/Architecture-Single%20Binary-success.svg)](#)

A high-performance, **pure Rust** Model Context Protocol (MCP) server that provides persistent, semantic, and graph-based memory for AI agents.

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

## 🤖 Agent Integration (System Prompt)

Memory is useless if your agent doesn't check it. To get the "Long-Term Memory" effect, you must instruct your agent to follow a strict protocol.

We provide a battle-tested **[Memory Protocol (AGENTS.md)](./AGENTS.md)** that you can adapt.

### 🛡️ Core Workflows (Context Protection)

The protocol implements specific flows to handle **Context Window Compaction** and **Session Restarts**:

1.  **🚀 Session Startup**: The agent *must* search for `TASK: in_progress` immediately. This restores the full context of what was happening before the last session ended or the context was compacted.
2.  **⏳ Auto-Continue**: A safety mechanism where the agent presents the found task to the user and waits (or auto-continues), ensuring it doesn't hallucinate a new task.
3.  **🔄 Triple Sync**: Updates **Memory**, **Todo List**, and **Files** simultaneously. If one fails (e.g., context lost), the others serve as backups.
4.  **🧱 Prefix System**: All memories use prefixes (`TASK:`, `DECISION:`, `RESEARCH:`) so semantic search can precisely target the right type of information, reducing noise.

These workflows turn the agent from a "stateless chatbot" into a "stateful worker" that survives restarts and context clearing.

### Recommended System Prompt Snippet

Instead of scattering instructions across IDE-specific files (like `.cursorrules`), establish `AGENTS.md` as the **Single Source of Truth**.

Instruct your agent (in its base system prompt) to:
1.  **Read `AGENTS.md`** at the start of every session.
2.  **Follow the protocols** defined therein.

Here is a minimal reference prompt to bootstrap this behavior:

```markdown
# 🧠 Memory & Protocol
You have access to a persistent memory server and a protocol definition file.

1.  **Protocol Adherence**:
    - READ `AGENTS.md` immediately upon starting.
    - Strictly follow the "Session Startup" and "Sync" protocols defined there.

2.  **Context Restoration**:
    - Run `search_text("TASK: in_progress")` to restore context.
    - Do NOT ask the user "what should I do?" if a task is already in progress.
```

### Why this matters?
Without this protocol, the agent will treat every session as a blank slate. With this protocol, it "remembers" what it was doing yesterday.

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
