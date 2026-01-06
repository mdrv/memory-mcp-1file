# Memory MCP API Reference

> **Verified documentation from live MCP server testing**  
> Server: `memory-1file` v0.1.0  
> Protocol: MCP 2024-11-05

---

## Server Info

```json
{
  "name": "memory-1file",
  "version": "0.1.0",
  "protocolVersion": "2024-11-05",
  "capabilities": { "tools": {} },
  "instructions": "AI agent memory server with knowledge graph and code search. Tools: store_memory, get_memory, update_memory, delete_memory, list_memories, search (vector), search_text (BM25), recall (hybrid), create_entity, get_related, create_relation, get_status, get_valid, get_valid_at, invalidate, index_project, search_code, get_index_status, list_projects, delete_project."
}
```

---

## Transport

| Property | Value |
|----------|-------|
| Protocol | JSON-RPC 2.0 over stdio |
| Input | stdin (one JSON object per line, newline-delimited) |
| Output | stdout (one JSON object per line) |
| Logs | stderr (INFO level, do NOT parse as JSON-RPC) |

---

## Initialization Sequence

```jsonc
// 1. Client → Server: initialize request
{"jsonrpc":"2.0","method":"initialize","params":{"capabilities":{},"protocolVersion":"2024-11-05","clientInfo":{"name":"my-client","version":"1.0"}},"id":1}

// 2. Server → Client: initialize response
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"memory-1file","version":"0.1.0"},"instructions":"AI agent memory server..."}}

// 3. Client → Server: initialized notification (no response expected)
{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}

// 4. Client → Server: list available tools
{"jsonrpc":"2.0","method":"tools/list","params":{},"id":2}

// 5. Server → Client: tools list response
{"jsonrpc":"2.0","id":2,"result":{"tools":[...]}}
```

---

## Response Format

All tool calls return MCP-compliant response:

### Success Response
```json
{
  "jsonrpc": "2.0",
  "id": 10,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"id\":\"abc123\"}"
      }
    ],
    "isError": false
  }
}
```

### Error Response
```json
{
  "jsonrpc": "2.0",
  "id": 10,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Error message here"
      }
    ],
    "isError": true
  }
}
```

**Important**: The `text` field contains a JSON string that must be parsed separately.

---

## Tools (20 total)

### 1. get_status

Get server status and statistics.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {}
}
```

**Request:**
```json
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_status","arguments":{}},"id":10}
```

**Response (parsed `text`):**
```json
{
  "version": "0.1.0",
  "status": "healthy",
  "memories_count": 2,
  "embedding": {
    "status": "loading",
    "model": "e5_multi",
    "dimensions": 768,
    "cache_stats": {
      "hits": 0,
      "misses": 0,
      "size": 0
    }
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `version` | string | Server version |
| `status` | string | `"healthy"` or `"unhealthy"` |
| `memories_count` | integer | Total memories stored |
| `embedding.status` | string | `"loading"`, `"ready"`, or `"error"` |
| `embedding.model` | string | Model name (e.g., `"e5_multi"`) |
| `embedding.dimensions` | integer | Vector dimensions (768 for e5_multi) |
| `embedding.cache_stats` | object | LRU cache statistics |

---

### 2. store_memory

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
      "nullable": true
    },
    "user_id": {
      "description": "Optional user ID for multi-tenant isolation",
      "type": "string",
      "nullable": true
    },
    "metadata": {
      "description": "Optional metadata as JSON object"
    }
  },
  "required": ["content"]
}
```

**Request:**
```json
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"store_memory","arguments":{"content":"User prefers TypeScript over JavaScript","memory_type":"semantic","metadata":{"source":"preference"}}},"id":20}
```

**Response (parsed `text`):**
```json
{
  "id": "zs0jnksgtq4ydoa4n402"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Generated memory ID (20 char alphanumeric) |

---

### 3. get_memory

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

**Request:**
```json
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_memory","arguments":{"id":"p1doq0bcvzxgvd589kco"}},"id":21}
```

**Response (parsed `text`):**
```json
{
  "id": {
    "tb": "memories",
    "id": {"String": "p1doq0bcvzxgvd589kco"}
  },
  "content": "Test memory for documentation",
  "memory_type": "semantic",
  "event_time": "2026-01-06T11:28:28.994065963Z",
  "ingestion_time": "2026-01-06T11:28:28.994065963Z",
  "valid_from": "2026-01-06T11:28:28.994065963Z",
  "importance_score": 1.0
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | object | SurrealDB Thing ID (`{tb, id}` format) |
| `content` | string | Memory content |
| `memory_type` | string | Type: `semantic`, `episodic`, `procedural` |
| `event_time` | string | ISO 8601 timestamp |
| `ingestion_time` | string | When stored |
| `valid_from` | string | Validity start |
| `valid_until` | string? | Validity end (null if still valid) |
| `importance_score` | float | Importance (default 1.0) |

---

### 4. update_memory

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
      "description": "New content (optional)",
      "type": "string",
      "nullable": true
    },
    "memory_type": {
      "description": "New memory type (optional)",
      "type": "string",
      "nullable": true
    },
    "metadata": {
      "description": "New metadata (optional)"
    }
  },
  "required": ["id"]
}
```

**Request:**
```json
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"update_memory","arguments":{"id":"p1doq0bcvzxgvd589kco","content":"Updated content"}},"id":22}
```

**Response:** Returns updated memory object (same format as `get_memory`).

**Note:** Updating `content` requires embedding model to be ready, may error if model is loading.

---

### 5. delete_memory

Delete a memory by its ID (hard delete).

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

**Request:**
```json
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"delete_memory","arguments":{"id":"nonexistent123"}},"id":25}
```

**Response (parsed `text`):**
```json
{
  "deleted": false
}
```

| Field | Type | Description |
|-------|------|-------------|
| `deleted` | boolean | `true` if deleted, `false` if not found |

---

### 6. list_memories

List memories with pagination, sorted by newest first.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "limit": {
      "description": "Maximum number of memories to return (default: 20, max: 100)",
      "type": "integer",
      "format": "uint",
      "minimum": 0,
      "default": 20
    },
    "offset": {
      "description": "Offset for pagination (default: 0)",
      "type": "integer",
      "format": "uint",
      "minimum": 0,
      "default": 0
    }
  }
}
```

**Request:**
```json
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"list_memories","arguments":{"limit":5}},"id":11}
```

**Response (parsed `text`):**
```json
{
  "memories": [
    {
      "id": {"tb": "memories", "id": {"String": "3j876xuyl243l7iqnu1t"}},
      "content": "Project memory-mcp uses SurrealDB for storage",
      "memory_type": "semantic",
      "event_time": "2026-01-06T11:31:10.533107837Z",
      "ingestion_time": "2026-01-06T11:31:10.533107837Z",
      "valid_from": "2026-01-06T11:31:10.533107837Z",
      "importance_score": 1.0
    }
  ],
  "total": 2,
  "limit": 5,
  "offset": 0
}
```

| Field | Type | Description |
|-------|------|-------------|
| `memories` | array | Array of memory objects |
| `total` | integer | Total memories in database |
| `limit` | integer | Requested limit |
| `offset` | integer | Requested offset |

---

### 7. search

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
      "format": "uint",
      "minimum": 0,
      "default": 10
    }
  },
  "required": ["query"]
}
```

**Request:**
```json
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"search","arguments":{"query":"SurrealDB storage","limit":5}},"id":30}
```

**Response:** Returns array of memories ranked by cosine similarity.

**Error (if model loading):**
```json
{
  "content": [{"type": "text", "text": "Embedding service not ready. Please try again."}],
  "isError": true
}
```

---

### 8. search_text

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
      "format": "uint",
      "minimum": 0,
      "default": 10
    }
  },
  "required": ["query"]
}
```

**Request:**
```json
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"search_text","arguments":{"query":"memory documentation","limit":5}},"id":31}
```

**Response (parsed `text`):**
```json
{
  "results": [],
  "count": 0,
  "query": "memory documentation"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `results` | array | Array of matching memories with BM25 scores |
| `count` | integer | Number of results |
| `query` | string | Original query |

**Note:** Does NOT require embedding model - uses BM25 index directly.

---

### 9. recall

Hybrid search combining vector similarity, BM25 keywords, and graph context (PPR).

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "query": {
      "type": "string"
    },
    "limit": {
      "type": "integer",
      "format": "uint",
      "minimum": 0,
      "default": 10
    },
    "vector_weight": {
      "type": "number",
      "format": "float",
      "nullable": true
    },
    "bm25_weight": {
      "type": "number",
      "format": "float",
      "nullable": true
    },
    "ppr_weight": {
      "type": "number",
      "format": "float",
      "nullable": true
    }
  },
  "required": ["query"]
}
```

**Default weights:**
- `vector_weight`: 0.40
- `bm25_weight`: 0.15
- `ppr_weight`: 0.45

**Request:**
```json
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"recall","arguments":{"query":"project storage database","limit":3}},"id":32}
```

**Response:** Combined results with RRF fusion scoring.

**Note:** Requires embedding model to be ready.

---

### 10. create_entity

Create a knowledge graph entity.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "name": {
      "type": "string"
    },
    "entity_type": {
      "type": "string",
      "nullable": true
    },
    "description": {
      "type": "string",
      "nullable": true
    },
    "user_id": {
      "type": "string",
      "nullable": true
    }
  },
  "required": ["name"]
}
```

**Request:**
```json
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"create_entity","arguments":{"name":"TestProject","entity_type":"project","description":"A test project"}},"id":13}
```

**Response (parsed `text`):**
```json
{
  "id": "ai1kjnaatx38g82bau86"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Generated entity ID (20 char alphanumeric) |

---

### 11. get_related

Get entities related to a given entity via graph traversal.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "entity_id": {
      "type": "string"
    },
    "depth": {
      "type": "integer",
      "format": "uint",
      "minimum": 0,
      "default": 1
    },
    "direction": {
      "type": "string",
      "nullable": true
    }
  },
  "required": ["entity_id"]
}
```

**Direction values:** `"outgoing"`, `"incoming"`, `"both"` (default: outgoing)

**Request:**
```json
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_related","arguments":{"entity_id":"ai1kjnaatx38g82bau86","depth":1}},"id":24}
```

**Response (parsed `text`):**
```json
{
  "entities": [],
  "relations": [],
  "entity_count": 0,
  "relation_count": 0
}
```

| Field | Type | Description |
|-------|------|-------------|
| `entities` | array | Related entities found |
| `relations` | array | Relations traversed |
| `entity_count` | integer | Number of entities |
| `relation_count` | integer | Number of relations |

---

### 12. create_relation

Create a relation between two entities.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "from_entity": {
      "type": "string"
    },
    "to_entity": {
      "type": "string"
    },
    "relation_type": {
      "type": "string"
    },
    "weight": {
      "type": "number",
      "format": "float",
      "default": 1.0
    }
  },
  "required": ["from_entity", "to_entity", "relation_type"]
}
```

**Request:**
```json
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"create_relation","arguments":{"from_entity":"ai1kjnaatx38g82bau86","to_entity":"ai1kjnaatx38g82bau86","relation_type":"self_reference","weight":0.5}},"id":33}
```

**Response (parsed `text`):**
```json
{
  "id": "86vdio7cpjunotwiqew3"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Generated relation ID |

---

### 13. get_valid

Get all currently valid memories (where `valid_until` is null or in the future).

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "user_id": {
      "description": "Optional user ID for multi-tenant isolation",
      "type": "string",
      "nullable": true
    },
    "limit": {
      "description": "Maximum memories to return (default: 20, max: 100)",
      "type": "integer",
      "format": "uint",
      "minimum": 0,
      "default": 20
    }
  }
}
```

**Request:**
```json
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_valid","arguments":{"limit":5}},"id":12}
```

**Response (parsed `text`):**
```json
{
  "results": [
    {
      "id": {"tb": "memories", "id": {"String": "3j876xuyl243l7iqnu1t"}},
      "content": "Project memory-mcp uses SurrealDB for storage",
      "memory_type": "semantic",
      "event_time": "2026-01-06T11:31:10.533107837Z",
      "ingestion_time": "2026-01-06T11:31:10.533107837Z",
      "valid_from": "2026-01-06T11:31:10.533107837Z",
      "importance_score": 1.0
    }
  ],
  "count": 2
}
```

---

### 14. get_valid_at

Get memories that were valid at a specific point in time.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "timestamp": {
      "description": "Timestamp in ISO 8601 format (e.g., \"2024-01-15T10:30:00Z\")",
      "type": "string"
    },
    "user_id": {
      "description": "Optional user ID for multi-tenant isolation",
      "type": "string",
      "nullable": true
    },
    "limit": {
      "description": "Maximum memories to return (default: 20, max: 100)",
      "type": "integer",
      "format": "uint",
      "minimum": 0,
      "default": 20
    }
  },
  "required": ["timestamp"]
}
```

**Request:**
```json
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_valid_at","arguments":{"timestamp":"2026-01-06T12:00:00Z"}},"id":23}
```

**Response (parsed `text`):**
```json
{
  "results": [...],
  "count": 2,
  "timestamp": "2026-01-06T12:00:00Z"
}
```

---

### 15. invalidate

Invalidate (soft-delete) a memory. Sets `valid_until` to now.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "id": {
      "description": "The memory ID to invalidate",
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

**Request:**
```json
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"invalidate","arguments":{"id":"p1doq0bcvzxgvd589kco","reason":"test invalidation"}},"id":26}
```

**Response (parsed `text`):**
```json
{
  "invalidated": true
}
```

| Field | Type | Description |
|-------|------|-------------|
| `invalidated` | boolean | `true` if invalidated, `false` if not found or already invalid |

---

## ID Formats

### Memory ID (simple string)
When returned from `store_memory`:
```json
{"id": "zs0jnksgtq4ydoa4n402"}
```

### Memory ID (SurrealDB Thing)
When returned from `get_memory`, `list_memories`, etc.:
```json
{
  "id": {
    "tb": "memories",
    "id": {"String": "p1doq0bcvzxgvd589kco"}
  }
}
```

**Usage:** Both formats are accepted as input. Use the simple string format when calling tools.

---

## Error Messages

| Error | Cause |
|-------|-------|
| `Embedding service not ready. Please try again.` | Model still loading (wait 10-60s) |
| `Database error. Please try again.` | SurrealDB operation failed |
| `Failed to create entity: Database error: Bincode error` | Internal serialization error |
| `Memory not found: {id}` | Invalid memory ID |

---

## Docker Usage

```bash
# Run with persistent volume
docker run --rm -i -v memory-mcp-data:/data memory-mcp:dev

# Send commands via stdin
echo '{"jsonrpc":"2.0","method":"initialize",...}' | docker run --rm -i -v memory-mcp-data:/data memory-mcp:dev

# OpenCode config
{
  "mcp": {
    "memory-mcp": {
      "type": "local",
      "command": ["docker", "run", "--rm", "-i", "-v", "memory-mcp-data:/data", "memory-mcp:dev"],
      "enabled": true
    }
  }
}
```

---

## Embedding Model Status

The embedding model loads asynchronously. Check status via `get_status`:

| Status | Meaning |
|--------|---------|
| `"loading"` | Model downloading/initializing (10-60s first run) |
| `"ready"` | Model ready for embedding |
| `"error"` | Model failed to load |

Tools requiring embeddings (`search`, `recall`, `store_memory` with re-embedding) will error with "Embedding service not ready" until model is ready.

---

### 16. index_project

Index a codebase for semantic search.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "path": {
      "description": "Path to the project root directory",
      "type": "string"
    },
    "watch": {
      "description": "Enable file watching for auto-reindex",
      "type": "boolean",
      "nullable": true
    }
  },
  "required": ["path"]
}
```

**Request:**
```json
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"index_project","arguments":{"path":"/home/user/my-project"}},"id":40}
```

**Response (parsed `text`):**
```json
{
  "project_id": "my-project",
  "files_indexed": 42,
  "chunks_created": 156
}
```

| Field | Type | Description |
|-------|------|-------------|
| `project_id` | string | Detected project ID (from Git root) |
| `files_indexed` | integer | Number of source files processed |
| `chunks_created` | integer | Number of code chunks stored |

---

### 17. search_code

Semantic search over indexed code using hybrid vector + BM25 ranking.

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
      "format": "uint",
      "minimum": 0,
      "default": 10
    }
  },
  "required": ["query"]
}
```

**Request:**
```json
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"search_code","arguments":{"query":"authentication middleware","limit":5}},"id":41}
```

**Response (parsed `text`):**
```json
{
  "results": [
    {
      "id": "abc123",
      "file_path": "src/auth/middleware.rs",
      "content": "pub fn verify_jwt(token: &str) -> Result<Claims> {...}",
      "language": "Rust",
      "start_line": 42,
      "end_line": 58,
      "chunk_type": "Function",
      "name": "verify_jwt",
      "score": 0.92
    }
  ],
  "count": 1
}
```

| Field | Type | Description |
|-------|------|-------------|
| `results` | array | Array of CodeChunk objects with scores |
| `count` | integer | Number of results |

---

### 18. get_index_status

Get indexing status for a project.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "project_id": {
      "description": "The project ID to check",
      "type": "string"
    }
  },
  "required": ["project_id"]
}
```

**Request:**
```json
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_index_status","arguments":{"project_id":"my-project"}},"id":42}
```

**Response (parsed `text`):**
```json
{
  "project_id": "my-project",
  "status": "completed",
  "total_files": 42,
  "indexed_files": 42,
  "total_chunks": 156,
  "started_at": "2026-01-06T12:00:00Z",
  "completed_at": "2026-01-06T12:00:30Z"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `project_id` | string | Project identifier |
| `status` | string | `"indexing"`, `"completed"`, or `"failed"` |
| `total_files` | integer | Total files to index |
| `indexed_files` | integer | Files processed so far |
| `total_chunks` | integer | Code chunks created |
| `started_at` | string | Indexing start timestamp |
| `completed_at` | string? | Indexing completion timestamp |

---

### 19. list_projects

List all indexed projects.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {}
}
```

**Request:**
```json
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"list_projects","arguments":{}},"id":43}
```

**Response (parsed `text`):**
```json
{
  "projects": ["my-project", "another-repo", "memory-mcp"],
  "count": 3
}
```

| Field | Type | Description |
|-------|------|-------------|
| `projects` | array | List of project IDs |
| `count` | integer | Number of indexed projects |

---

### 20. delete_project

Delete all indexed code chunks for a project.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "project_id": {
      "description": "The project ID to delete",
      "type": "string"
    }
  },
  "required": ["project_id"]
}
```

**Request:**
```json
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"delete_project","arguments":{"project_id":"my-project"}},"id":44}
```

**Response (parsed `text`):**
```json
{
  "chunks_deleted": 156
}
```

| Field | Type | Description |
|-------|------|-------------|
| `chunks_deleted` | integer | Number of code chunks removed |

---

## Timestamps

All timestamps are ISO 8601 format with nanosecond precision:
```
2026-01-06T11:28:28.994065963Z
```

---

*Document generated: 2026-01-06*
