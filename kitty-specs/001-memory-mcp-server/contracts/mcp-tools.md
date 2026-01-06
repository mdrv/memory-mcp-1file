# MCP Tools Contract: Memory MCP Server

**Feature**: 001-memory-mcp-server
**Date**: 2026-01-06
**Protocol**: MCP 2024-11-05 (JSON-RPC 2.0 over stdio)

---

## Server Info

```json
{
  "name": "memory-mcp",
  "version": "0.1.0",
  "protocolVersion": "2024-11-05",
  "capabilities": { "tools": {} },
  "instructions": "AI agent memory server with knowledge graph and code search. 21 tools available."
}
```

---

## Tool Categories

| Category | Tools | Count |
|----------|-------|-------|
| Memory Operations | store_memory, get_memory, update_memory, delete_memory, list_memories | 5 |
| Search | search, search_text, recall | 3 |
| Knowledge Graph | create_entity, create_relation, get_related | 3 |
| Temporal | get_valid, get_valid_at, invalidate | 3 |
| Code Search | index_project, search_code, get_index_status, list_projects, delete_project | 5 |
| System | get_status, reset_all_memory | 2 |
| **Total** | | **21** |

---

## Memory Operations (5 tools)

### 1. store_memory

Store a new memory with automatic embedding generation.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "content": {
      "description": "The content to store as a memory",
      "type": "string"
    },
    "memory_type": {
      "description": "Type of memory: episodic, semantic, or procedural",
      "type": "string",
      "enum": ["episodic", "semantic", "procedural"],
      "nullable": true
    },
    "user_id": {
      "description": "Optional user ID for multi-tenant isolation",
      "type": "string",
      "nullable": true
    },
    "metadata": {
      "description": "Optional metadata as JSON object",
      "type": "object",
      "nullable": true
    }
  },
  "required": ["content"]
}
```

**Output:**
```json
{ "id": "string (20 char alphanumeric)" }
```

**Errors:**
- "Embedding service not ready. Please try again." (model loading)

---

### 2. get_memory

Get a memory by its ID.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "id": {
      "description": "The memory ID to retrieve",
      "type": "string"
    }
  },
  "required": ["id"]
}
```

**Output:** Memory object (without embedding)
```json
{
  "id": { "tb": "memories", "id": { "String": "abc123" } },
  "content": "string",
  "memory_type": "semantic",
  "event_time": "2026-01-06T12:00:00Z",
  "ingestion_time": "2026-01-06T12:00:00Z",
  "valid_from": "2026-01-06T12:00:00Z",
  "valid_until": null,
  "importance_score": 1.0,
  "metadata": {}
}
```

**Errors:**
- "Memory not found: {id}"

---

### 3. update_memory

Update an existing memory. Only provided fields will be updated.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "id": {
      "description": "The memory ID to update",
      "type": "string"
    },
    "content": {
      "description": "New content (optional, re-embeds if changed)",
      "type": "string",
      "nullable": true
    },
    "memory_type": {
      "description": "New memory type (optional)",
      "type": "string",
      "nullable": true
    },
    "metadata": {
      "description": "New metadata (optional)",
      "type": "object",
      "nullable": true
    }
  },
  "required": ["id"]
}
```

**Output:** Updated Memory object

**Errors:**
- "Memory not found: {id}"
- "Embedding service not ready" (if content changed)

---

### 4. delete_memory

Delete a memory by ID (hard delete).

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "id": {
      "description": "The memory ID to delete",
      "type": "string"
    }
  },
  "required": ["id"]
}
```

**Output:**
```json
{ "deleted": true }
```

---

### 5. list_memories

List memories with pagination, sorted by newest first.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "limit": {
      "description": "Maximum number to return (default: 20, max: 100)",
      "type": "integer",
      "minimum": 1,
      "maximum": 100,
      "default": 20
    },
    "offset": {
      "description": "Offset for pagination (default: 0)",
      "type": "integer",
      "minimum": 0,
      "default": 0
    }
  }
}
```

**Output:**
```json
{
  "memories": [],
  "total": 0,
  "limit": 20,
  "offset": 0
}
```

---

## Search (3 tools)

### 6. search

Semantic vector search over memories using HNSW index.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "query": {
      "description": "The search query text (will be embedded)",
      "type": "string"
    },
    "limit": {
      "description": "Maximum results (default: 10, max: 50)",
      "type": "integer",
      "minimum": 1,
      "maximum": 50,
      "default": 10
    }
  },
  "required": ["query"]
}
```

**Output:**
```json
{
  "results": [
    {
      "id": "string",
      "content": "string",
      "memory_type": "semantic",
      "score": 0.95
    }
  ],
  "count": 1,
  "query": "string"
}
```

**Errors:**
- "Embedding service not ready. Please try again."

---

### 7. search_text

Full-text keyword search using BM25 algorithm.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "query": {
      "description": "The keyword query for full-text search",
      "type": "string"
    },
    "limit": {
      "description": "Maximum results (default: 10, max: 50)",
      "type": "integer",
      "minimum": 1,
      "maximum": 50,
      "default": 10
    }
  },
  "required": ["query"]
}
```

**Output:**
```json
{
  "results": [],
  "count": 0,
  "query": "string"
}
```

**Note:** Does NOT require embedding model.

---

### 8. recall

Hybrid search combining vector similarity, BM25 keywords, and graph context (PPR). Can search memories, code, or both with Z-Score normalization.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "query": {
      "description": "Search query",
      "type": "string"
    },
    "limit": {
      "description": "Maximum results (default: 10, max: 50)",
      "type": "integer",
      "default": 10
    },
    "include_memories": {
      "description": "Include memory results (default: true)",
      "type": "boolean",
      "default": true
    },
    "include_code": {
      "description": "Include code chunk results (default: false)",
      "type": "boolean",
      "default": false
    },
    "project_id": {
      "description": "Filter code results by project (optional, requires include_code=true)",
      "type": "string",
      "nullable": true
    },
    "language": {
      "description": "Filter code by language: rust, python, javascript, typescript, go (optional)",
      "type": "string",
      "nullable": true
    },
    "vector_weight": {
      "description": "Weight for vector similarity (default: 0.40)",
      "type": "number",
      "default": 0.40
    },
    "bm25_weight": {
      "description": "Weight for BM25 (default: 0.15)",
      "type": "number",
      "default": 0.15
    },
    "ppr_weight": {
      "description": "Weight for PPR graph ranking (default: 0.45)",
      "type": "number",
      "default": 0.45
    }
  },
  "required": ["query"]
}
```

**Output:**
```json
{
  "results": [
    {
      "id": "string",
      "content": "string",
      "source": "memory|code",
      "score": 0.85,
      "vector_score": 0.90,
      "bm25_score": 0.70,
      "ppr_score": 0.80,
      "file_path": "src/main.rs",
      "start_line": 10,
      "end_line": 25
    }
  ],
  "query": "string",
  "subgraph_nodes": 5,
  "memories_count": 3,
  "code_count": 2
}
```

**Note:** When both memories and code are included, scores are Z-Score normalized before merging.

---

## Knowledge Graph (3 tools)

### 9. create_entity

Create a knowledge graph entity.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "name": {
      "description": "Entity name",
      "type": "string"
    },
    "entity_type": {
      "description": "Type: person, project, concept, file, etc.",
      "type": "string",
      "nullable": true
    },
    "description": {
      "description": "Optional description",
      "type": "string",
      "nullable": true
    },
    "user_id": {
      "description": "Optional user ID for isolation",
      "type": "string",
      "nullable": true
    }
  },
  "required": ["name"]
}
```

**Output:**
```json
{ "id": "string" }
```

---

### 10. create_relation

Create a directed relation between two entities.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "from_entity": {
      "description": "Source entity ID",
      "type": "string"
    },
    "to_entity": {
      "description": "Target entity ID",
      "type": "string"
    },
    "relation_type": {
      "description": "Relation type: works_on, knows, uses, etc.",
      "type": "string"
    },
    "weight": {
      "description": "Relation weight (0.0-1.0, default: 1.0)",
      "type": "number",
      "minimum": 0,
      "maximum": 1,
      "default": 1.0
    }
  },
  "required": ["from_entity", "to_entity", "relation_type"]
}
```

**Output:**
```json
{ "id": "string" }
```

---

### 11. get_related

Get entities related to a given entity via graph traversal.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "entity_id": {
      "description": "Entity ID to start from",
      "type": "string"
    },
    "depth": {
      "description": "Traversal depth (1-3, default: 1)",
      "type": "integer",
      "minimum": 1,
      "maximum": 3,
      "default": 1
    },
    "direction": {
      "description": "Traversal direction",
      "type": "string",
      "enum": ["outgoing", "incoming", "both"],
      "default": "outgoing"
    }
  },
  "required": ["entity_id"]
}
```

**Output:**
```json
{
  "entities": [],
  "relations": [],
  "entity_count": 0,
  "relation_count": 0
}
```

---

## Temporal (3 tools)

### 12. get_valid

Get all currently valid memories.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "user_id": {
      "description": "Optional user ID filter",
      "type": "string",
      "nullable": true
    },
    "limit": {
      "description": "Maximum to return (default: 20, max: 100)",
      "type": "integer",
      "default": 20
    }
  }
}
```

**Output:**
```json
{
  "results": [],
  "count": 0
}
```

---

### 13. get_valid_at

Get memories valid at a specific point in time.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "timestamp": {
      "description": "Timestamp in ISO 8601 format",
      "type": "string",
      "format": "date-time"
    },
    "user_id": {
      "description": "Optional user ID filter",
      "type": "string",
      "nullable": true
    },
    "limit": {
      "description": "Maximum to return (default: 20, max: 100)",
      "type": "integer",
      "default": 20
    }
  },
  "required": ["timestamp"]
}
```

**Output:**
```json
{
  "results": [],
  "count": 0,
  "timestamp": "2026-01-06T12:00:00Z"
}
```

---

### 14. invalidate

Invalidate (soft-delete) a memory. Sets valid_until to now.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "id": {
      "description": "Memory ID to invalidate",
      "type": "string"
    },
    "reason": {
      "description": "Optional reason for invalidation",
      "type": "string",
      "nullable": true
    },
    "superseded_by": {
      "description": "Optional ID of memory that supersedes this one",
      "type": "string",
      "nullable": true
    }
  },
  "required": ["id"]
}
```

**Output:**
```json
{ "invalidated": true }
```

---

## Code Search (5 tools)

### 15. index_project

Index a codebase for semantic search.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "path": {
      "description": "Path to project root directory",
      "type": "string"
    },
    "watch": {
      "description": "Enable file watching for auto-reindex (optional, deferred)",
      "type": "boolean",
      "default": false
    }
  },
  "required": ["path"]
}
```

**Output:**
```json
{
  "project_id": "my-project",
  "files_indexed": 42,
  "chunks_created": 156
}
```

**Errors:**
- "Embedding service not ready"
- "Invalid path: {path}"

---

### 16. search_code

Semantic search over indexed code.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "query": {
      "description": "Search query text",
      "type": "string"
    },
    "project_id": {
      "description": "Filter by project (optional)",
      "type": "string",
      "nullable": true
    },
    "limit": {
      "description": "Maximum results (default: 10, max: 50)",
      "type": "integer",
      "default": 10
    }
  },
  "required": ["query"]
}
```

**Output:**
```json
{
  "results": [
    {
      "id": "string",
      "file_path": "src/main.rs",
      "content": "fn main() {...}",
      "language": "rust",
      "start_line": 1,
      "end_line": 10,
      "chunk_type": "function",
      "name": "main",
      "score": 0.92
    }
  ],
  "count": 1
}
```

---

### 17. get_index_status

Get indexing status for a project.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "project_id": {
      "description": "Project ID to check",
      "type": "string"
    }
  },
  "required": ["project_id"]
}
```

**Output:**
```json
{
  "project_id": "my-project",
  "status": "indexing",
  "total_files": 42,
  "indexed_files": 21,
  "total_chunks": 78,
  "progress_percent": 50.0,
  "eta_seconds": 15,
  "started_at": "2026-01-06T12:00:00Z",
  "completed_at": null
}
```

**Status Values:**
- "pending" - Not started
- "indexing" - In progress (progress_percent and eta_seconds available)
- "completed" - Finished successfully
- "failed" - Error occurred

**Note:** `eta_seconds` is calculated on-the-fly based on elapsed time and progress percentage.

---

### 18. list_projects

List all indexed projects.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {}
}
```

**Output:**
```json
{
  "projects": ["my-project", "another-repo"],
  "count": 2
}
```

---

### 19. delete_project

Delete all indexed code chunks for a project.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "project_id": {
      "description": "Project ID to delete",
      "type": "string"
    }
  },
  "required": ["project_id"]
}
```

**Output:**
```json
{ "chunks_deleted": 156 }
```

---

## System (2 tools)

### 20. get_status

Get server status and health information.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {}
}
```

**Output:**
```json
{
  "version": "0.1.0",
  "status": "healthy",
  "memories_count": 100,
  "entities_count": 25,
  "code_chunks_count": 500,
  "embedding": {
    "status": "ready",
    "model": "e5_multi",
    "dimensions": 768,
    "cache_stats": {
      "hits": 50,
      "misses": 10,
      "size": 60
    }
  },
  "db_dimension": 768
}
```

**Embedding Status Values:**
- "loading" - Model downloading/initializing
- "ready" - Model ready for use
- "error" - Model failed to load

---

### 21. reset_all_memory

Delete ALL data (memories, entities, relations, code_chunks). Requires explicit confirmation.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "confirm": {
      "description": "Must be true to confirm deletion. Safety check.",
      "type": "boolean"
    }
  },
  "required": ["confirm"]
}
```

**Output (if confirm=true):**
```json
{
  "reset": true,
  "deleted": {
    "memories": 100,
    "entities": 25,
    "relations": 50,
    "code_chunks": 500
  }
}
```

**Output (if confirm=false or missing):**
```json
{
  "reset": false,
  "message": "Confirmation required. Set confirm=true to delete all data."
}
```

**Errors:**
- This tool does not error - it returns reset=false if not confirmed.

---

## Error Response Format

All tool errors return:
```json
{
  "content": [
    {
      "type": "text",
      "text": "Error message here"
    }
  ],
  "isError": true
}
```

## Common Errors

| Error Message | Cause |
|---------------|-------|
| "Embedding service not ready. Please try again." | Model still loading |
| "Memory not found: {id}" | Invalid memory ID |
| "Database error. Please try again." | SurrealDB operation failed |
| "Invalid path: {path}" | File system error |
