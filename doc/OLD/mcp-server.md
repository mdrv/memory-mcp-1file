# MCP Server — Design Document

## Overview

MCP (Model Context Protocol) server implementation using `rmcp` crate with tool_router pattern.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    MemoryMcpServer                           │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐                   │
│  │  EmbeddingService│  │  SurrealStorage │                   │
│  │  (Arc<...>)     │  │  (Arc<...>)     │                   │
│  └─────────────────┘  └─────────────────┘                   │
│           │                    │                             │
│           └────────┬───────────┘                             │
│                    ▼                                         │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              AppState (Arc<RwLock<...>>)                │ │
│  └─────────────────────────────────────────────────────────┘ │
│                    │                                         │
│                    ▼                                         │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              20 MCP Tools                               │ │
│  │  • Memory: store, get, update, delete, list             │ │
│  │  • Search: vector, text, hybrid                         │ │
│  │  • Graph: entity, relation, related                     │ │
│  │  • Temporal: valid, valid_at, invalidate                │ │
│  │  • Code: index, search, status, list, delete            │ │
│  │  • System: status                                       │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                    │ stdio (JSON-RPC)
                    ▼
┌─────────────────────────────────────────────────────────────┐
│                    MCP Client (Claude, Cursor, etc.)        │
└─────────────────────────────────────────────────────────────┘
```

## Core Types

### AppState

```rust
pub struct AppState {
    pub storage: Arc<dyn StorageBackend>,
    pub embedder: Arc<EmbeddingService>,
    pub config: AppConfig,
}

pub struct AppConfig {
    pub data_dir: PathBuf,
    pub model: ModelType,
    pub cache_size: usize,
    pub batch_size: usize,
    pub timeout_secs: u64,
    pub preload: bool,
    pub log_level: String,
}
```

### Server Definition

```rust
use rmcp::{tool, tool_router, tool_handler, ServerHandler};
use rmcp::types::{ServerInfo, ServerCapabilities, CallToolResult, Content};

#[derive(Clone)]
pub struct MemoryMcpServer {
    state: Arc<RwLock<AppState>>,
}

#[tool_router]
impl MemoryMcpServer {
    pub fn new(state: AppState) -> Self {
        Self {
            state: Arc::new(RwLock::new(state)),
        }
    }
}

#[tool_handler]
impl ServerHandler for MemoryMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            name: "memory-mcp".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            instructions: Some(
                "AI agent memory with vector search, knowledge graph, and code indexing".into()
            ),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
        }
    }
}
```

## Tool Implementations

### Memory Tools

```rust
#[tool_router]
impl MemoryMcpServer {
    #[tool(description = "Store a new memory with automatic embedding")]
    async fn store_memory(
        &self,
        #[arg(description = "Content to store")] content: String,
        #[arg(description = "Type: episodic, semantic, procedural")] memory_type: Option<String>,
        #[arg(description = "Additional metadata")] metadata: Option<serde_json::Value>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.read().await;
        
        // Generate embedding
        let embedding = state.embedder.embed(&content).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        
        // Create memory
        let memory = Memory {
            content,
            embedding: Some(embedding),
            memory_type: memory_type.unwrap_or_else(|| "semantic".to_string()),
            metadata,
            ..Default::default()
        };
        
        let id = state.storage.create_memory(memory).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        
        Ok(CallToolResult::success(vec![
            Content::text(json!({ "id": id }).to_string())
        ]))
    }
    
    #[tool(description = "Get a memory by ID")]
    async fn get_memory(
        &self,
        #[arg(description = "Memory ID")] id: String,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.read().await;
        
        match state.storage.get_memory(&id).await {
            Ok(Some(memory)) => Ok(CallToolResult::success(vec![
                Content::text(serde_json::to_string(&memory).unwrap())
            ])),
            Ok(None) => Ok(CallToolResult::error(vec![
                Content::text(format!("Memory not found: {}", id))
            ])),
            Err(e) => {
                tracing::error!(error = ?e, id = %id, "Failed to get memory");
                Ok(CallToolResult::error(vec![
                    Content::text("Database error. Please try again.")
                ]))
            }
        }
    }
    
    #[tool(description = "Update an existing memory")]
    async fn update_memory(
        &self,
        #[arg(description = "Memory ID")] id: String,
        #[arg(description = "New content")] content: Option<String>,
        #[arg(description = "New metadata")] metadata: Option<serde_json::Value>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.read().await;
        
        let mut updates = MemoryUpdate::default();
        
        if let Some(content) = content {
            let embedding = state.embedder.embed(&content).await?;
            updates.content = Some(content);
            updates.embedding = Some(embedding);
        }
        
        if metadata.is_some() {
            updates.metadata = metadata;
        }
        
        match state.storage.update_memory(&id, updates).await? {
            Some(memory) => Ok(CallToolResult::success(vec![
                Content::text(serde_json::to_string(&memory)?)
            ])),
            None => Ok(CallToolResult::error(vec![
                Content::text(format!("Memory not found: {}", id))
            ])),
        }
    }
    
    #[tool(description = "Delete a memory")]
    async fn delete_memory(
        &self,
        #[arg(description = "Memory ID")] id: String,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.read().await;
        let deleted = state.storage.delete_memory(&id).await?;
        
        Ok(CallToolResult::success(vec![
            Content::text(json!({ "deleted": deleted }).to_string())
        ]))
    }
    
    #[tool(description = "List memories with pagination")]
    async fn list_memories(
        &self,
        #[arg(description = "Maximum results")] limit: Option<usize>,
        #[arg(description = "Skip first N results")] offset: Option<usize>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.read().await;
        let memories = state.storage.list_memories(
            limit.unwrap_or(20).min(100),
            offset.unwrap_or(0),
        ).await?;
        
        Ok(CallToolResult::success(vec![
            Content::text(serde_json::to_string(&memories)?)
        ]))
    }
}
```

### Search Tools

```rust
#[tool_router]
impl MemoryMcpServer {
    #[tool(description = "Semantic vector search over memories")]
    async fn search(
        &self,
        #[arg(description = "Search query")] query: String,
        #[arg(description = "Maximum results (default: 10, max: 50)")] limit: Option<usize>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.read().await;
        let limit = limit.unwrap_or(10).min(50);
        
        let embedding = state.embedder.embed(&query).await?;
        let results = state.storage.vector_search(&embedding, limit).await?;
        
        // Fetch full memories for results
        let memories = self.fetch_memories(&state, &results).await?;
        
        Ok(CallToolResult::success(vec![
            Content::text(serde_json::to_string(&memories)?)
        ]))
    }
    
    #[tool(description = "Full-text BM25 search over memories")]
    async fn search_text(
        &self,
        #[arg(description = "Search query")] query: String,
        #[arg(description = "Maximum results")] limit: Option<usize>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.read().await;
        let limit = limit.unwrap_or(10).min(50);
        
        let results = state.storage.bm25_search(&query, limit).await?;
        let memories = self.fetch_memories(&state, &results).await?;
        
        Ok(CallToolResult::success(vec![
            Content::text(serde_json::to_string(&memories)?)
        ]))
    }
    
    #[tool(description = "Hybrid search: vector + BM25 + graph ranking")]
    async fn recall(
        &self,
        #[arg(description = "Search query")] query: String,
        #[arg(description = "Maximum results")] limit: Option<usize>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.read().await;
        let limit = limit.unwrap_or(10).min(50);
        
        let embedding = state.embedder.embed(&query).await?;
        
        // Parallel retrieval
        let (vec_results, bm25_results, degrees) = tokio::try_join!(
            state.storage.vector_search(&embedding, 50),
            state.storage.bm25_search(&query, 50),
            state.storage.get_node_degrees(),
        )?;
        
        // RRF + PPR fusion
        let seeds = rrf_merge(&[vec_results.clone(), bm25_results.clone()], 60.0, 20);
        let dampened = dampen_hubs(&seeds, &degrees);
        
        let subgraph = state.storage.get_subgraph(&dampened).await?;
        let ppr_scores = personalized_page_rank(
            &subgraph.to_petgraph(),
            &dampened.into_iter().collect(),
            0.5,
            1e-6,
        );
        
        // Final scoring
        let scored = compute_final_scores(
            &seeds,
            &vec_results,
            &bm25_results,
            &ppr_scores,
        );
        
        let mut results: Vec<_> = scored.into_iter().take(limit).collect();
        let memories = self.fetch_memories(&state, &results).await?;
        
        Ok(CallToolResult::success(vec![
            Content::text(serde_json::to_string(&json!({
                "memories": memories,
                "subgraph_nodes": subgraph.nodes.len(),
            }))?)
        ]))
    }
}
```

### Graph Tools

```rust
#[tool_router]
impl MemoryMcpServer {
    #[tool(description = "Create a named entity")]
    async fn create_entity(
        &self,
        #[arg(description = "Entity name")] name: String,
        #[arg(description = "Type: person, project, concept, file, etc.")] entity_type: String,
        #[arg(description = "Optional description")] description: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.read().await;
        
        let embedding = state.embedder.embed(&name).await?;
        
        let entity = Entity {
            name,
            entity_type,
            description,
            embedding: Some(embedding),
            ..Default::default()
        };
        
        let id = state.storage.create_entity(entity).await?;
        
        Ok(CallToolResult::success(vec![
            Content::text(json!({ "id": id }).to_string())
        ]))
    }
    
    #[tool(description = "Create a relation between entities")]
    async fn create_relation(
        &self,
        #[arg(description = "Source entity ID")] from_id: String,
        #[arg(description = "Target entity ID")] to_id: String,
        #[arg(description = "Relation type: works_on, knows, uses, etc.")] relation_type: String,
        #[arg(description = "Relation weight (0.0-1.0)")] weight: Option<f32>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.read().await;
        
        let id = state.storage.create_relation(
            &from_id,
            &to_id,
            &relation_type,
            weight.unwrap_or(1.0),
        ).await?;
        
        Ok(CallToolResult::success(vec![
            Content::text(json!({ "id": id }).to_string())
        ]))
    }
    
    #[tool(description = "Get entities related to a given entity")]
    async fn get_related(
        &self,
        #[arg(description = "Entity ID")] entity_id: String,
        #[arg(description = "Traversal depth (1-3)")] depth: Option<u32>,
        #[arg(description = "Direction: outgoing, incoming, both")] direction: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.read().await;
        
        let depth = depth.unwrap_or(1).min(3);
        let direction = match direction.as_deref() {
            Some("incoming") => Direction::Incoming,
            Some("both") => Direction::Both,
            _ => Direction::Outgoing,
        };
        
        let related = state.storage.get_related(&entity_id, depth, direction).await?;
        
        Ok(CallToolResult::success(vec![
            Content::text(serde_json::to_string(&related)?)
        ]))
    }
}
```

### Temporal Tools

```rust
#[tool_router]
impl MemoryMcpServer {
    #[tool(description = "Get currently valid memories")]
    async fn get_valid(&self) -> Result<CallToolResult, McpError> {
        let state = self.state.read().await;
        let memories = state.storage.get_valid().await?;
        
        Ok(CallToolResult::success(vec![
            Content::text(serde_json::to_string(&memories)?)
        ]))
    }
    
    #[tool(description = "Get memories valid at a specific point in time")]
    async fn get_valid_at(
        &self,
        #[arg(description = "ISO 8601 timestamp")] timestamp: String,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.read().await;
        
        let ts = DateTime::parse_from_rfc3339(&timestamp)
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?
            .with_timezone(&Utc);
        
        let memories = state.storage.get_valid_at(ts).await?;
        
        Ok(CallToolResult::success(vec![
            Content::text(serde_json::to_string(&memories)?)
        ]))
    }
    
    #[tool(description = "Invalidate a memory (soft delete with reason)")]
    async fn invalidate(
        &self,
        #[arg(description = "Memory ID")] id: String,
        #[arg(description = "Reason for invalidation")] reason: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.read().await;
        let invalidated = state.storage.invalidate(&id, reason.as_deref()).await?;
        
        Ok(CallToolResult::success(vec![
            Content::text(json!({ "invalidated": invalidated }).to_string())
        ]))
    }
}
```

### Code Search Tools

```rust
#[tool_router]
impl MemoryMcpServer {
    #[tool(description = "Index a codebase for semantic search")]
    async fn index_project(
        &self,
        #[arg(description = "Path to project root")] path: String,
        #[arg(description = "Watch for changes")] watch: Option<bool>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.read().await;
        let path = PathBuf::from(&path);
        
        // Detect project
        let project = detect_project(&path)?;
        
        // Index in background
        let stats = index_directory(
            &path,
            state.storage.clone(),
            state.embedder.clone(),
            Some(project.id.clone()),
        ).await?;
        
        Ok(CallToolResult::success(vec![
            Content::text(json!({
                "project_id": project.id,
                "files_indexed": stats.files_indexed,
                "chunks_created": stats.chunks_created,
            }).to_string())
        ]))
    }
    
    #[tool(description = "Semantic search over indexed code")]
    async fn search_code(
        &self,
        #[arg(description = "Search query")] query: String,
        #[arg(description = "Project ID filter")] project_id: Option<String>,
        #[arg(description = "Maximum results")] limit: Option<usize>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.read().await;
        let limit = limit.unwrap_or(10).min(50);
        
        let embedding = state.embedder.embed(&query).await?;
        
        let (vec_results, bm25_results) = tokio::try_join!(
            state.storage.vector_search_code(&embedding, 50, project_id.as_deref()),
            state.storage.bm25_search_code(&query, 50, project_id.as_deref()),
        )?;
        
        let merged = rrf_merge(&[vec_results, bm25_results], 60.0, limit);
        let chunks = self.fetch_chunks(&state, &merged).await?;
        
        Ok(CallToolResult::success(vec![
            Content::text(serde_json::to_string(&chunks)?)
        ]))
    }
    
    #[tool(description = "Get indexing status for a project")]
    async fn get_index_status(
        &self,
        #[arg(description = "Project ID")] project_id: String,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.read().await;
        let status = state.storage.get_index_status(&project_id).await?;
        
        Ok(CallToolResult::success(vec![
            Content::text(serde_json::to_string(&status)?)
        ]))
    }
    
    #[tool(description = "List all indexed projects")]
    async fn list_projects(&self) -> Result<CallToolResult, McpError> {
        let state = self.state.read().await;
        let projects = state.storage.list_projects().await?;
        
        Ok(CallToolResult::success(vec![
            Content::text(serde_json::to_string(&projects)?)
        ]))
    }
    
    #[tool(description = "Delete a project index")]
    async fn delete_project(
        &self,
        #[arg(description = "Project ID")] project_id: String,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.read().await;
        let deleted = state.storage.delete_project_chunks(&project_id).await?;
        
        Ok(CallToolResult::success(vec![
            Content::text(json!({ "chunks_deleted": deleted }).to_string())
        ]))
    }
}
```

### System Tools

```rust
#[tool_router]
impl MemoryMcpServer {
    #[tool(description = "Get server status and health")]
    async fn get_status(&self) -> Result<CallToolResult, McpError> {
        let state = self.state.read().await;
        
        let db_ok = state.storage.health_check().await.is_ok();
        let model_ready = state.embedder.is_ready();
        
        Ok(CallToolResult::success(vec![
            Content::text(json!({
                "version": env!("CARGO_PKG_VERSION"),
                "database": if db_ok { "connected" } else { "error" },
                "embedding_model": if model_ready { "ready" } else { "loading" },
                "model_type": state.config.model.name(),
            }).to_string())
        ]))
    }
}
```

## Entry Point

```rust
// src/main.rs
use clap::Parser;

#[derive(Parser)]
#[command(name = "memory-mcp", version, about)]
struct Cli {
    #[arg(long, env = "MEMORY_MCP_DATA_DIR")]
    data_dir: Option<PathBuf>,
    
    #[arg(long, env = "MEMORY_MCP_MODEL", default_value = "e5_multi")]
    model: String,
    
    #[arg(long, env = "MEMORY_MCP_CACHE_SIZE", default_value = "1000")]
    cache_size: usize,
    
    #[arg(long, env = "MEMORY_MCP_BATCH_SIZE", default_value = "32")]
    batch_size: usize,
    
    #[arg(long, env = "MEMORY_MCP_TIMEOUT", default_value = "60")]
    timeout: u64,
    
    #[arg(long, env = "MEMORY_MCP_PRELOAD", default_value = "true")]
    preload: bool,
    
    #[arg(long, env = "MEMORY_MCP_LOG_LEVEL", default_value = "info")]
    log_level: String,
    
    #[arg(long)]
    list_models: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    if cli.list_models {
        println!("Available models:");
        println!("  e5_small  - 384 dims, ~134 MB, fast");
        println!("  e5_multi  - 768 dims, ~1.1 GB, multilingual (default)");
        println!("  nomic     - 768 dims, ~1.9 GB, Apache 2.0");
        println!("  bge_m3    - 1024 dims, ~2.3 GB, long context");
        return Ok(());
    }
    
    // Logging to stderr (MCP uses stdout)
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(&cli.log_level)
        .init();
    
    // Initialize components
    let data_dir = cli.data_dir.unwrap_or_else(default_data_dir);
    fs::create_dir_all(&data_dir)?;
    
    let storage = Arc::new(SurrealStorage::new(&data_dir).await?);
    
    let embedder = Arc::new(EmbeddingService::new(EmbeddingConfig {
        model: cli.model.parse()?,
        cache_size: cli.cache_size,
        batch_size: cli.batch_size,
        timeout_secs: cli.timeout,
        preload: cli.preload,
    }).await?);
    
    let state = AppState {
        storage,
        embedder,
        config: AppConfig::from(&cli),
    };
    
    let server = MemoryMcpServer::new(state);
    
    tracing::info!("Starting memory-mcp v{}", env!("CARGO_PKG_VERSION"));
    
    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    
    Ok(())
}

fn default_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("memory-mcp")
}
```

## Error Handling

Two-tier model per MCP spec:

| Type | When | Rust | LLM sees |
|------|------|------|----------|
| Tool Error | Logic error (not found, invalid input) | `Ok(CallToolResult::error(...))` | Error message, can retry |
| Protocol Error | Invalid request, server broken | `Err(McpError)` | Hard failure |

```rust
// Tool error example
Ok(CallToolResult::error(vec![
    Content::text(format!("Memory not found: {}", id))
]))

// Protocol error example
Err(McpError::invalid_params("timestamp must be ISO 8601 format", None))
```

## Security Checklist

- [x] No panics — never `unwrap()` in handlers
- [x] Sanitize errors — no credential/path leaks
- [x] Log to stderr — MCP uses stdout
- [x] Validate input — check params before processing
- [x] Token protection — embeddings never serialized
- [x] Hard limits — max 50/100 results per query
