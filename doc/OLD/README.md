# Memory MCP Server

Self-contained memory server for AI agents. One binary, no external services.

## What is this?

A persistent memory layer for LLM-based agents that integrates via [Model Context Protocol (MCP)](https://modelcontextprotocol.io/). Store facts, track relationships, search by meaning — all locally.

## Why?

AI agents forget everything between sessions. Existing solutions (Mem0, Zep, ChromaDB) require complex setup, external databases, or cloud APIs.

**memory-mcp** is different:
- **Single binary** — download and run
- **Offline-first** — embeddings computed locally via Candle (pure Rust)
- **Privacy** — your data stays on your machine
- **Fast** — native Rust, HNSW indexes, graph traversal

## Tech Stack

| Component | Technology |
|-----------|------------|
| Database | SurrealDB (embedded, pure Rust) |
| Vector search | HNSW index via SurrealDB |
| Full-text search | BM25 via SurrealDB |
| Embeddings | Candle (pure Rust ML) |
| Graph | Native SurrealDB relations + petgraph |
| Protocol | MCP over stdio |
| Parsing | tree-sitter (AST-based code chunking) |
| File watching | notify crate |

## Available Embedding Models

| Model | Dimensions | Size | Description |
|-------|-----------|------|-------------|
| `e5_small` | 384 | ~134 MB | Fast, English-focused |
| `e5_multi` | 768 | ~1.1 GB | Multilingual, 100+ languages (default) |
| `nomic` | 768 | ~1.9 GB | MoE architecture, Apache 2.0 |
| `bge_m3` | 1024 | ~2.3 GB | Long context (8K tokens) |

## Configuration

| Environment Variable | CLI Flag | Default | Description |
|---------------------|----------|---------|-------------|
| `MEMORY_MCP_DATA_DIR` | `--data-dir` | `~/.local/share/memory-mcp` | Data directory |
| `MEMORY_MCP_MODEL` | `--model` | `e5_multi` | Embedding model |
| `MEMORY_MCP_CACHE_SIZE` | `--cache-size` | `1000` | Embedding cache entries |
| `MEMORY_MCP_BATCH_SIZE` | `--batch-size` | `32` | Max batch size |
| `MEMORY_MCP_TIMEOUT` | `--timeout` | `60` | Model load timeout (seconds) |
| `MEMORY_MCP_LOG_LEVEL` | `--log-level` | `info` | Log level |

> **Note**: Embedding model loads in background on startup. Server responds immediately; use `get_status` to check embedding readiness.

## Features

### Memory Operations
- **store** — save memory with auto-embedding
- **get/update/delete** — standard CRUD
- **list** — paginated listing

### Search & Retrieval
- **vector search** — semantic similarity via HNSW
- **text search** — keyword matching via BM25
- **hybrid recall** — combines vector + BM25 + graph ranking

### Knowledge Graph
- **entities** — named objects (people, projects, concepts)
- **relations** — directed, weighted connections
- **traversal** — find related entities by depth

### Temporal
- **validity tracking** — facts have valid_from/valid_until
- **point-in-time queries** — what was true at specific date
- **soft delete** — invalidate with reason

### Code Search (New)
- **index_project** — scan and index codebase
- **search_code** — semantic code search with RRF
- **list_projects** — show indexed projects
- **delete_project** — remove project index

## Retrieval Algorithm

Hybrid search combining three signals:

```
score = 0.40 × vector_similarity 
      + 0.15 × bm25_score 
      + 0.45 × personalized_pagerank
```

Uses Personalized PageRank (PPR) with α=0.5 for graph-aware ranking.

## MCP Tools (20 total)

1. store_memory
2. get_memory
3. update_memory
4. delete_memory
5. list_memories
6. search (vector)
7. search_text (BM25)
8. recall (hybrid)
9. create_entity
10. get_related
11. create_relation
12. get_status
13. get_valid
14. get_valid_at
15. invalidate
16. index_project
17. search_code
18. get_index_status
19. list_projects
20. delete_project

## License

MIT
