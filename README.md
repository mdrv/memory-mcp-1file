# Memory MCP Server

A Model Context Protocol (MCP) server that provides persistent memory for AI agents with semantic search, knowledge graph, and code search capabilities.

## Features

- **Semantic Memory**: Store and retrieve information using vector embeddings (FastEmbed).
- **Graph Memory**: Manage entities and relations with PageRank-based traversal.
- **Code Search**: Index and search local project codebases.
- **Hybrid Retrieval**: Combine vector search, BM25 keyword search, and graph algorithms (PPR) for optimal recall.
- **Temporal Validity**: Track when memories are valid (`valid_from`, `valid_until`).
- **SurrealDB Backend**: Embedded, high-performance database.

## Installation

### From Source

```bash
git clone <repository-url>
cd memory-mcp
cargo install --path .
```

### Docker

```bash
docker build -t memory-mcp .
docker run -v $(pwd)/data:/data memory-mcp
```

## Usage

Start the server:

```bash
memory-mcp
```

### Configuration

You can configure the server using CLI arguments.

| Argument | Default | Description |
|----------|---------|-------------|
| `--data-dir` | `~/.local/share/memory-mcp` | Directory for database storage |
| `--model` | `e5_multi` | Embedding model (`e5_small`, `e5_multi`, `nomic`, `bge_m3`) |
| `--log-level` | `info` | Logging verbosity |
| `--cache-size` | `1000` | Number of embeddings to cache in memory |
| `--batch-size` | `32` | Batch size for embedding generation |
| `--timeout` | `30000` | Request timeout in milliseconds |

To list available models and their sizes:
```bash
memory-mcp --list-models
```

## MCP Tools Reference

The server exposes **21 tools** for comprehensive memory management.

### Memory Operations

| Tool | Description |
|------|-------------|
| `store_memory` | Store a new memory with content and optional metadata. |
| `get_memory` | Retrieve a memory by its ID. |
| `update_memory` | Update content or metadata of an existing memory. |
| `delete_memory` | Delete a memory by ID. |
| `list_memories` | List recent memories with pagination. |
| `invalidate` | Soft-delete a memory (mark as invalid from now on). |
| `get_valid` | Get all currently valid memories. |
| `get_valid_at` | Get memories that were valid at a specific timestamp. |

### Search & Retrieval

| Tool | Description |
|------|-------------|
| `search` | Semantic search using vector similarity. |
| `search_text` | Keyword search using BM25. |
| `recall` | Hybrid search combining Vector + BM25 + Knowledge Graph (PPR). |

### Knowledge Graph

| Tool | Description |
|------|-------------|
| `create_entity` | Create a node in the knowledge graph. |
| `create_relation` | Connect two entities with a directed relation. |
| `get_related` | Find related entities up to a specified depth. |

### Codebase Indexing

| Tool | Description |
|------|-------------|
| `index_project` | Index a local directory for code search. |
| `search_code` | Search indexed code using natural language. |
| `get_index_status`| Check the status of a project indexing job. |
| `list_projects` | List all indexed projects. |
| `delete_project` | Remove a project and its code chunks from the index. |

### System

| Tool | Description |
|------|-------------|
| `get_status` | Get server health and stats (memory count, version). |
| `reset_all_memory`| Clear all data (requires confirmation). |

## Examples

### Storing a Memory

```json
{
  "name": "store_memory",
  "arguments": {
    "content": "The user prefers Python for data analysis tasks.",
    "memory_type": "semantic",
    "metadata": {
      "confidence": 0.9,
      "source": "user_chat"
    }
  }
}
```

### Hybrid Search (Recall)

```json
{
  "name": "recall",
  "arguments": {
    "query": "What are the user's coding preferences?",
    "limit": 5,
    "vector_weight": 0.7,
    "bm25_weight": 0.3
  }
}
```

### Knowledge Graph Relation

```json
{
  "name": "create_relation",
  "arguments": {
    "from_entity": "user_id_123",
    "to_entity": "python_lang",
    "relation_type": "prefers",
    "weight": 1.0
  }
}
```

## Architecture

- **Storage**: SurrealDB (Embedded)
- **Vectors**: fastembed-rs (ONNX Runtime)
- **Graph**: petgraph for in-memory traversal (PPR)

## License

MIT
