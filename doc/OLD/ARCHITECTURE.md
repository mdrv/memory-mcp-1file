# Architecture

## High-Level Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        MCP Client (Claude, etc.)                │
└─────────────────────────────────────────────────────────────────┘
                                │ stdio
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Memory MCP Server                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   Handler   │  │  Embedding  │  │      Codebase           │  │
│  │  (20 tools) │  │   Service   │  │  (scanner, chunker,     │  │
│  │             │  │   (Candle)  │  │   indexer, watcher)     │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
│         │                │                    │                  │
│         └────────────────┼────────────────────┘                  │
│                          ▼                                       │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                    Storage Backend                          │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │ │
│  │  │  Memories   │  │  Entities   │  │    Code Chunks      │  │ │
│  │  │  (HNSW+BM25)│  │  (Graph)    │  │    (HNSW+BM25)      │  │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘  │ │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                   SurrealDB (Embedded)                          │
│                   ~/.local/share/memory-mcp/db/                 │
└─────────────────────────────────────────────────────────────────┘
```

## Module Structure

```
src/
├── lib.rs              # Public exports
├── main.rs             # Entry point
├── types/
│   ├── mod.rs          # Re-exports
│   ├── memory.rs       # Memory, MemoryType
│   ├── entity.rs       # Entity, EntityType, Relation
│   ├── error.rs        # AppError, Result
│   ├── code.rs         # CodeChunk, ChunkType, Language
│   └── search.rs       # SearchResult enum
├── storage/
│   ├── mod.rs          # Re-exports
│   ├── traits.rs       # StorageBackend trait
│   ├── surrealdb.rs    # SurrealDB implementation
│   └── schema.surql    # Database schema
├── embedding/
│   ├── mod.rs          # Re-exports
│   ├── config.rs       # EmbeddingConfig, ModelInfo
│   ├── engine.rs       # InnerEngine, EmbeddingEngine
│   ├── cache.rs        # EmbeddingCache (LRU)
│   └── service.rs      # EmbeddingService (async wrapper)
├── graph/
│   ├── mod.rs          # Graph algorithms
│   └── ppr.rs          # Personalized PageRank
├── project/
│   ├── mod.rs          # Re-exports
│   └── namespace.rs    # ProjectInfo, detect_project()
├── codebase/
│   ├── mod.rs          # Re-exports
│   ├── scanner.rs      # scan_directory(), is_code_file()
│   ├── chunker.rs      # chunk_file(), tree-sitter parsing
│   ├── indexer.rs      # index_directory(), batch embedding
│   ├── progress.rs     # IndexingProgress
│   └── watcher.rs      # CodebaseWatcher (file watching)
└── server/
    ├── mod.rs          # Re-exports
    └── handler.rs      # MemoryMcpServer, 20 MCP tools
```

## Data Flow

### Memory Storage
```
User Input → embed() → Memory { content, embedding } → SurrealDB
```

### Memory Retrieval (Hybrid)
```
Query → embed() → parallel:
  ├── vector_search() → top 50 by cosine similarity
  ├── bm25_search() → top 50 by keyword match
  └── get_node_degrees() → graph connectivity
      ↓
RRF Merge (k=60) → PPR Dampen → Top N results
```

### Code Indexing
```
Project Path → detect_project() → scan_directory() 
    → parallel chunk_file() (rayon) 
    → adaptive_batches() 
    → embed_batch() with retry 
    → create_code_chunks_batch()
```

### Code Search
```
Query → embed() → parallel:
  ├── vector_search_code() → top 50
  └── bm25_search_code() → top 50
      ↓
RRF Merge (k=60) → Top N results
```

## Key Design Decisions

1. **Embedded DB** — SurrealDB runs in-process, no external service
2. **Pure Rust ML** — Candle for embeddings, no Python/ONNX runtime
3. **HNSW for vectors** — O(log n) similarity search
4. **Namespace isolation** — Each project gets separate namespace
5. **Token protection** — Embeddings never serialized in responses
6. **Hybrid search** — RRF merge of vector + BM25 + graph signals
