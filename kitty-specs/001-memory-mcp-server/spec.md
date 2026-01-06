# Feature Specification: Memory MCP Server

**Feature Branch**: `001-memory-mcp-server`  
**Created**: 2026-01-06  
**Status**: Draft  
**Input**: Self-contained MCP memory server for AI agents - single-file Rust implementation based on doc/OLD/ specifications

## Overview

Memory MCP Server is a self-contained persistent memory layer for LLM-based agents that integrates via Model Context Protocol (MCP). One binary, no external services - embeddings computed locally via Candle (pure Rust).

**Key Value Proposition**:
- Single binary - download and run
- Offline-first - embeddings computed locally
- Privacy - data stays on machine
- Fast - native Rust, HNSW indexes, graph traversal

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Store and Retrieve Memory (Priority: P1)

AI agent stores facts learned during conversation and retrieves them in future sessions.

**Why this priority**: Core functionality - without memory storage and retrieval, the server has no value.

**Independent Test**: Can be tested by storing a memory via `store_memory`, then retrieving it via `get_memory` - delivers persistent context across sessions.

**Acceptance Scenarios**:

1. **Given** the MCP server is running, **When** agent calls `store_memory` with content "User prefers TypeScript over JavaScript", **Then** server returns a memory ID and content is persisted with auto-generated embedding
2. **Given** a memory was previously stored, **When** agent calls `get_memory` with the ID, **Then** server returns the full memory object with content, type, and timestamps
3. **Given** a memory exists, **When** agent calls `delete_memory` with the ID, **Then** server confirms deletion and memory is no longer retrievable

---

### User Story 2 - Semantic Search (Priority: P1)

AI agent searches for relevant memories using natural language queries.

**Why this priority**: Equal to storage - agents need to find relevant memories semantically, not just by ID.

**Independent Test**: Can be tested by storing 5 memories about different topics, then searching for "programming languages" - should return relevant memories ranked by similarity.

**Acceptance Scenarios**:

1. **Given** multiple memories are stored, **When** agent calls `search` with query "database design patterns", **Then** server returns memories semantically similar to the query, ordered by relevance
2. **Given** memories exist with varied content, **When** agent calls `search_text` with keywords "SurrealDB storage", **Then** server returns memories containing those keywords via BM25 ranking
3. Given memories and knowledge graph exist, When agent calls `recall` with a query, Then server returns hybrid results combining vector similarity, BM25, and graph context
4. Given both code chunks and memory items exist, When agent calls `recall` or search, Then results are normalized (Z-Score) so code doesn't dominate text memories

---

### User Story 3 - Knowledge Graph (Priority: P2)

AI agent builds a knowledge graph of entities and their relationships.

**Why this priority**: Enhances recall quality through graph-based ranking, but core functionality works without it.

**Independent Test**: Can be tested by creating two entities (Person, Project), relating them with "works_on", then traversing - delivers structured knowledge representation.

**Acceptance Scenarios**:

1. **Given** the server is running, **When** agent calls `create_entity` with name "Alice" and type "person", **Then** server returns entity ID and entity is searchable
2. **Given** two entities exist, **When** agent calls `create_relation` from "Alice" to "memory-mcp" with type "works_on", **Then** server creates directed edge and returns relation ID
3. Given entity "Alice" has relations, When agent calls `get_related` with entity_id and depth 2, Then server returns all entities within 2 hops
4. Given a complex graph, When agent calls `detect_communities`, Then server returns grouped entities using Leiden algorithm

---

### User Story 4 - Temporal Queries (Priority: P2)

AI agent queries what was known at a specific point in time.

**Why this priority**: Enables fact versioning and correction, but basic memory works without temporal features.

**Independent Test**: Can be tested by storing a memory, invalidating it, then querying valid memories - demonstrates time-aware knowledge.

**Acceptance Scenarios**:

1. **Given** memories exist with various valid_from dates, **When** agent calls `get_valid`, **Then** server returns only currently valid memories
2. **Given** a memory was valid at timestamp T, **When** agent calls `get_valid_at` with timestamp T, **Then** server returns that memory in results
3. **Given** a fact becomes outdated, **When** agent calls `invalidate` with memory ID and reason, **Then** memory is soft-deleted (valid_until set) but remains queryable historically

---

### User Story 5 - Code Search (Priority: P3)

AI agent indexes and searches a codebase semantically.

**Why this priority**: Powerful feature for developer agents, but the core memory system is fully functional without it.

**Independent Test**: Can be tested by indexing a small project, then searching "authentication middleware" - returns relevant code snippets with file paths and line numbers.

**Acceptance Scenarios**:

1. Given a path to project root, When agent calls `index_project` with `watch=true`, Then server scans files AND starts a background watcher for changes
2. Given watcher is running, When user saves a file, Then server waits 500ms (debounce) and re-indexes only that file
3. **Given** a project is indexed, **When** agent calls `search_code` with query "error handling", **Then** server returns code chunks ranked by relevance with file paths and line numbers
4. **Given** indexing is in progress, **When** agent calls `get_index_status`, **Then** server returns progress (files indexed, chunks created, status)
5. **Given** multiple projects are indexed, **When** agent calls `list_projects`, **Then** server returns all indexed project IDs
6. **Given** a project is indexed, **When** agent calls `delete_project`, **Then** server removes all code chunks for that project

---

### User Story 6 - Server Health Monitoring (Priority: P3)

AI agent checks server status and embedding model readiness.

**Why this priority**: Operational tool, not core functionality.

**Independent Test**: Can be tested by calling `get_status` after startup - shows version, database status, embedding model status.

**Acceptance Scenarios**:

1. **Given** server just started, **When** agent calls `get_status`, **Then** server returns version, database status, and embedding status (may be "loading")
2. **Given** embedding model finished loading, **When** agent calls `get_status`, **Then** embedding status shows "ready" with model name and dimensions

---

### Edge Cases

- What happens when embedding model is still loading during search? Server returns "Embedding service not ready" error
- What happens when memory ID doesn't exist? Server returns "Memory not found: {id}" error  
- What happens when project path is invalid? Server returns appropriate file system error
- What happens when model download fails? Server continues with embedding status "error", non-embedding tools still work
- What happens when database is corrupted? Server returns "Database error" and logs details to stderr

## Requirements *(mandatory)*

### Functional Requirements - Memory Operations

- **FR-001**: System MUST store memories with content, optional type (episodic/semantic/procedural), optional metadata
- **FR-002**: System MUST auto-generate embeddings for stored content using configured model
- **FR-003**: System MUST retrieve memories by ID with all fields except embedding
- **FR-004**: System MUST update memory content and re-embed when content changes
- **FR-005**: System MUST delete memories by ID (hard delete)
- **FR-006**: System MUST list memories with pagination (limit, offset), sorted by newest first

### Functional Requirements - Search

- **FR-007**: System MUST perform vector similarity search using HNSW index
- **FR-008**: System MUST perform keyword search using BM25 index
- FR-009: System MUST perform hybrid search combining vector (0.40), BM25 (0.15), and PPR (0.45) scores
- FR-009a: System MUST apply Z-Score normalization to vector/BM25 scores before merging to balance different query types (code vs text)
- **FR-010**: System MUST apply temporal filter to search (exclude invalid memories)
- **FR-011**: System MUST limit results to max 50 per query

### Functional Requirements - Knowledge Graph

- **FR-012**: System MUST create entities with name, type, optional description
- **FR-013**: System MUST create directed relations between entities with type and weight
- FR-014: System MUST traverse graph to specified depth in specified direction (outgoing/incoming/both)
- FR-015: System MUST support Personalized PageRank for graph-aware ranking
- FR-015a: System MUST implement Leiden algorithm for community detection in the knowledge graph

### Functional Requirements - Temporal

- **FR-016**: System MUST track valid_from timestamp for all memories (defaults to creation time)
- **FR-017**: System MUST support valid_until timestamp for invalidated memories
- **FR-018**: System MUST return memories valid at specific point in time
- **FR-019**: System MUST soft-delete via invalidation with optional reason

### Functional Requirements - Code Search

- FR-020: System MUST scan directory respecting .gitignore and .memoryignore
- FR-020a: System MUST implement a debounced file watcher to auto-reindex changed files (500ms debounce)
- **FR-021**: System MUST chunk code files using tree-sitter AST parsing
- **FR-022**: System MUST inject hierarchical context (parent scope) into chunks
- **FR-023**: System MUST batch embed chunks with adaptive sizing (max 8192 tokens)
- **FR-023a**: System MUST implement retry logic for batch embedding failures (3 retries with exponential backoff: 100ms, 500ms, 2s)
- **FR-024**: System MUST track indexing progress and status per project
- **FR-024a**: System MUST isolate code chunks by project_id field (NOT SurrealDB namespaces - single namespace, filter by field)
- **FR-025**: System MUST delete all chunks for a project on demand

### Functional Requirements - Embedding

- **FR-026**: System MUST support 4 embedding models: e5_small (384d), e5_multi (768d, default - best multilingual support via XLM-RoBERTa), nomic (768d, requires custom NomicBert impl), bge_m3 (1024d)
- **FR-027**: System MUST download models from HuggingFace Hub on first use
- **FR-028**: System MUST load model in background (non-blocking startup)
- **FR-029**: System MUST cache embeddings in LRU cache (default 1000 entries)
- **FR-030**: System MUST never serialize embeddings in MCP responses (token protection)

### Functional Requirements - Dimension Mismatch Handling

- **FR-041**: System MUST persist selected model dimension in database metadata table on first startup
- **FR-042**: System MUST check dimension compatibility on every startup: compare configured model dimension with stored dimension in DB
- **FR-043**: On dimension mismatch, system MUST offer two resolution strategies via CLI flags:
  - `--force-model`: Override configured model to use one matching stored dimension (e.g., if DB has 768d vectors, auto-select e5_multi)
  - `--reset-memory`: Delete ALL existing memories, entities, relations, code_chunks and reinitialize with new model dimension
- **FR-044**: Without explicit flag on dimension mismatch, system MUST refuse to start with clear error message: "Dimension mismatch: database has Xd vectors, configured model produces Yd. Use --force-model or --reset-memory"
- **FR-045**: System MUST provide MCP tool `reset_all_memory` requiring explicit `confirm: true` parameter to clear all data programmatically

### Functional Requirements - MCP Protocol

- **FR-031**: System MUST implement MCP protocol over stdio (JSON-RPC 2.0)
- **FR-032**: System MUST provide 21 tools with JSON Schema for arguments
- **FR-033**: System MUST return tool errors as CallToolResult.error (not protocol errors)
- **FR-034**: System MUST log to stderr (stdout reserved for MCP protocol)

### Functional Requirements - Configuration

- **FR-035**: System MUST support configuration via environment variables and CLI flags with the following mapping (CLI takes precedence over ENV):

| CLI Flag | ENV Variable | Default | Description |
|----------|--------------|---------|-------------|
| `--data-dir` | `MEMORY_MCP_DATA_DIR` | `~/.local/share/memory-mcp` | Database storage path |
| `--model` | `MEMORY_MCP_MODEL` | `e5_multi` | Embedding model (e5_small, e5_multi, nomic, bge_m3) |
| `--cache-size` | `MEMORY_MCP_CACHE_SIZE` | `1000` | LRU embedding cache entries |
| `--batch-size` | `MEMORY_MCP_BATCH_SIZE` | `32` | Embedding batch size |
| `--timeout` | `MEMORY_MCP_TIMEOUT` | `60` | Operation timeout in seconds (60s allows for model download) |
| `--log-level` | `MEMORY_MCP_LOG_LEVEL` | `info` | Logging level (trace, debug, info, warn, error) |
| `--force-model` | `MEMORY_MCP_FORCE_MODEL` | `false` | Auto-select model matching DB dimension |
| `--reset-memory` | `MEMORY_MCP_RESET_MEMORY` | `false` | Clear all data on dimension mismatch |

- **FR-035a**: CLI flags MUST take precedence over environment variables
- **FR-035b**: System MUST validate all configuration values and fail fast with clear error messages
- **FR-036**: System MUST use default data directory (~/.local/share/memory-mcp)
- **FR-037**: System MUST support --list-models flag to show available models and exit

### Functional Requirements - Docker

- **FR-038**: System MUST provide production Dockerfile (multi-stage, compiles in container)
- **FR-039**: System MUST provide development Dockerfile.local (uses pre-built binary)
- **FR-040**: System MUST persist data via Docker volume mount

### Key Entities

- **Memory**: Content, embedding (hidden), memory_type, user_id, metadata, timestamps (event_time, ingestion_time, valid_from, valid_until), importance_score
- **Entity**: Name, entity_type, description, embedding (hidden), created_at
- **Relation**: From entity, to entity, relation_type, weight, valid_from, valid_until
- **CodeChunk**: file_path, content, language, start_line, end_line, chunk_type, name, embedding (hidden), content_hash, project_id, indexed_at
- **IndexStatus**: project_id, status, total_files, indexed_files, total_chunks, started_at, completed_at

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `store_memory` completes in under 100ms (excluding model load)
- **SC-002**: `search` (vector) completes in under 20ms for 10,000 memories
- **SC-003**: `search_text` (BM25) completes in under 30ms for 10,000 memories
- **SC-004**: `recall` (hybrid) completes in under 100ms for 10,000 memories
- **SC-005**: `index_project` processes 100 files in under 5 minutes
- **SC-006**: Binary size under 30 MB (excluding model files)
- **SC-007**: Model files download on first run (134 MB - 2.3 GB depending on model)
- **SC-008**: Server starts and responds to `tools/list` within 1 second (model loads in background)
- **SC-009**: All 84+ tests pass (47 unit + 37 integration)
- **SC-010**: Docker image builds successfully for linux/amd64

## Technical Stack

| Component | Technology | Version |
|-----------|------------|---------|
| Language | Rust | 2021 edition |
| Database | SurrealDB (embedded kv-surrealkv) | 2.x |
| Embeddings | Candle (pure Rust) | 0.9.1 |
| Tokenizer | tokenizers | 0.22.2 |
| MCP Protocol | rmcp | 0.12.0 |
| Graph | petgraph | 0.6 |
| Code parsing | tree-sitter | 0.26 |
| Code splitting | code-splitter | 0.4 |
| File watching | notify | 8.x |
| CLI | clap | 4.x |
| Serialization | serde, serde_json | 1.x |
| Error handling | anyhow, thiserror | 1.x, 2.x |
| Caching | lru | 0.12+ |
| Hashing | blake3 | 1.x |
| Parallelism | rayon | 1.10 |

## Assumptions

- Target platform: Linux x86_64 (primary), with cross-compilation to Windows and macOS
- Model files stored in shared HuggingFace cache (~/.cache/huggingface/)
- SurrealDB data stored in ~/.local/share/memory-mcp/db/
- Single-tenant usage (no multi-user authentication)
- Model selection is fixed at startup (cannot switch models at runtime)
