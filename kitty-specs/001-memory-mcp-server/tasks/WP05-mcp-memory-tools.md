---
work_package_id: WP05
title: "MCP Server + Memory Tools"
phase: "Phase 4"
priority: P1
subtasks: ["T033", "T034", "T035", "T036", "T037", "T038", "T039", "T040", "T041"]
lane: planned
dependencies: ["WP03", "WP04"]
history:
  - date: 2026-01-06
    action: created
    by: spec-kitty.tasks
---

# WP05: MCP Server + Memory Tools

## Objective

Create the MCP server with 5 basic memory operations. After this WP, the server responds to `tools/list` and all memory CRUD tools are functional.

## Context

This is the integration point where storage and embedding layers meet the MCP protocol. The server uses rmcp with `#[tool_router]` macro pattern.

**Reference**:
- `kitty-specs/001-memory-mcp-server/contracts/mcp-tools.md` - Tool schemas
- `kitty-specs/001-memory-mcp-server/research.md` - rmcp patterns

## Subtasks

### T033: Create MemoryMcpServer struct

**Location**: `src/server/handler.rs`

```rust
use std::sync::Arc;
use rmcp::prelude::*;

use crate::config::AppState;

pub struct MemoryMcpServer {
    state: Arc<AppState>,
}

impl MemoryMcpServer {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}
```

---

### T034: Implement ServerHandler trait

```rust
use rmcp::server::ServerHandler;

impl ServerHandler for MemoryMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            name: "memory-mcp".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            protocol_version: "2024-11-05".to_string(),
            capabilities: Capabilities {
                tools: Some(ToolsCapability {}),
                ..Default::default()
            },
            instructions: Some(
                "AI agent memory server with knowledge graph and code search. 20 tools available."
                    .to_string(),
            ),
        }
    }
}
```

---

### T035: Implement tool: store_memory

```rust
#[tool_router]
impl MemoryMcpServer {
    /// Store a new memory. Returns the memory ID.
    #[tool(description = "Store a new memory. Returns the memory ID.")]
    async fn store_memory(
        &self,
        /// The content to store as a memory
        content: String,
        /// Type of memory: episodic, semantic, or procedural
        memory_type: Option<String>,
        /// Optional user ID for multi-tenant isolation
        user_id: Option<String>,
        /// Optional metadata as JSON object
        metadata: Option<serde_json::Value>,
    ) -> Result<CallToolResult, McpError> {
        // Check embedding ready
        if self.state.embedding.status() != EmbeddingStatus::Ready {
            return Ok(CallToolResult::error("Embedding service not ready. Please try again."));
        }
        
        // Generate embedding
        let embedding = self.state.embedding.embed(&content).await
            .map_err(|e| McpError::internal(e.to_string()))?;
        
        // Create memory
        let memory = Memory {
            id: None,
            content,
            embedding: Some(embedding),
            memory_type: memory_type.map(|s| s.parse().unwrap_or_default()).unwrap_or_default(),
            user_id,
            metadata,
            event_time: chrono::Utc::now(),
            ingestion_time: chrono::Utc::now(),
            valid_from: chrono::Utc::now(),
            valid_until: None,
            importance_score: 1.0,
            invalidation_reason: None,
        };
        
        let id = self.state.storage.create_memory(memory).await
            .map_err(|e| McpError::internal(e.to_string()))?;
        
        Ok(CallToolResult::success(serde_json::json!({ "id": id })))
    }
}
```

---

### T036: Implement tool: get_memory

```rust
    /// Get a memory by its ID. Returns the full memory object or an error if not found.
    #[tool(description = "Get a memory by its ID. Returns the full memory object or an error if not found.")]
    async fn get_memory(
        &self,
        /// The memory ID to retrieve
        id: String,
    ) -> Result<CallToolResult, McpError> {
        match self.state.storage.get_memory(&id).await {
            Ok(Some(memory)) => Ok(CallToolResult::success(serde_json::to_value(memory).unwrap())),
            Ok(None) => Ok(CallToolResult::error(format!("Memory not found: {}", id))),
            Err(e) => Err(McpError::internal(e.to_string())),
        }
    }
```

---

### T037: Implement tool: update_memory

```rust
    /// Update an existing memory. Only provided fields will be updated.
    #[tool(description = "Update an existing memory. Only provided fields will be updated.")]
    async fn update_memory(
        &self,
        /// The memory ID to update
        id: String,
        /// New content (optional, keeps existing if not provided)
        content: Option<String>,
        /// New memory type (optional)
        memory_type: Option<String>,
        /// New metadata (optional)
        metadata: Option<serde_json::Value>,
    ) -> Result<CallToolResult, McpError> {
        // If content changed, need to re-embed
        let embedding = if let Some(ref new_content) = content {
            if self.state.embedding.status() != EmbeddingStatus::Ready {
                return Ok(CallToolResult::error("Embedding service not ready. Please try again."));
            }
            Some(self.state.embedding.embed(new_content).await
                .map_err(|e| McpError::internal(e.to_string()))?)
        } else {
            None
        };
        
        let update = MemoryUpdate {
            content,
            embedding,
            memory_type: memory_type.map(|s| s.parse().unwrap_or_default()),
            metadata,
        };
        
        match self.state.storage.update_memory(&id, update).await {
            Ok(memory) => Ok(CallToolResult::success(serde_json::to_value(memory).unwrap())),
            Err(AppError::MemoryNotFound(id)) => Ok(CallToolResult::error(format!("Memory not found: {}", id))),
            Err(e) => Err(McpError::internal(e.to_string())),
        }
    }
```

---

### T038: Implement tool: delete_memory

```rust
    /// Delete a memory by its ID. Returns true if deleted, false if not found.
    #[tool(description = "Delete a memory by its ID. Returns true if deleted, false if not found.")]
    async fn delete_memory(
        &self,
        /// The memory ID to delete
        id: String,
    ) -> Result<CallToolResult, McpError> {
        match self.state.storage.delete_memory(&id).await {
            Ok(deleted) => Ok(CallToolResult::success(serde_json::json!({ "deleted": deleted }))),
            Err(e) => Err(McpError::internal(e.to_string())),
        }
    }
```

---

### T039: Implement tool: list_memories

```rust
    /// List memories with pagination. Returns array of memories sorted by newest first.
    #[tool(description = "List memories with pagination. Returns array of memories sorted by newest first.")]
    async fn list_memories(
        &self,
        /// Maximum number of memories to return (default: 20, max: 100)
        limit: Option<usize>,
        /// Offset for pagination (default: 0)
        offset: Option<usize>,
    ) -> Result<CallToolResult, McpError> {
        let limit = limit.unwrap_or(20).min(100);
        let offset = offset.unwrap_or(0);
        
        let memories = self.state.storage.list_memories(limit, offset).await
            .map_err(|e| McpError::internal(e.to_string()))?;
        let total = self.state.storage.count_memories().await
            .map_err(|e| McpError::internal(e.to_string()))?;
        
        Ok(CallToolResult::success(serde_json::json!({
            "memories": memories,
            "total": total,
            "limit": limit,
            "offset": offset
        })))
    }
```

---

### T040: Create main.rs with CLI

**Location**: `src/main.rs`

```rust
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use rmcp::prelude::*;

use memory_mcp::{AppConfig, AppState};
use memory_mcp::storage::SurrealStorage;
use memory_mcp::embedding::{EmbeddingConfig, EmbeddingService, ModelType};
use memory_mcp::server::MemoryMcpServer;

#[derive(Parser)]
#[command(name = "memory-mcp")]
#[command(about = "MCP memory server for AI agents")]
struct Cli {
    /// Data directory for database
    #[arg(long, default_value_os_t = default_data_dir())]
    data_dir: PathBuf,
    
    /// Embedding model (e5_small, e5_multi, nomic, bge_m3)
    #[arg(long, default_value = "e5_multi")]
    model: String,
    
    /// Embedding cache size
    #[arg(long, default_value = "1000")]
    cache_size: usize,
    
    /// Batch size for embedding
    #[arg(long, default_value = "32")]
    batch_size: usize,
    
    /// Request timeout in milliseconds
    #[arg(long, default_value = "30000")]
    timeout: u64,
    
    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,
    
    /// List available models and exit
    #[arg(long)]
    list_models: bool,
}

fn default_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("memory-mcp")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    if cli.list_models {
        println!("Available models:");
        println!("  e5_small  - 384 dimensions, 134 MB");
        println!("  e5_multi  - 768 dimensions, 1.1 GB (default)");
        println!("  nomic     - 768 dimensions, 1.9 GB");
        println!("  bge_m3    - 1024 dimensions, 2.3 GB");
        return Ok(());
    }
    
    // Setup logging
    tracing_subscriber::fmt()
        .with_env_filter(&cli.log_level)
        .with_writer(std::io::stderr)
        .init();
    
    // Initialize storage
    let storage = Arc::new(SurrealStorage::new(&cli.data_dir).await?);
    
    // Initialize embedding
    let model: ModelType = cli.model.parse()
        .map_err(|e: String| anyhow::anyhow!(e))?;
    let embedding_config = EmbeddingConfig {
        model,
        cache_size: cli.cache_size,
        batch_size: cli.batch_size,
    };
    let embedding = Arc::new(EmbeddingService::new(embedding_config));
    embedding.start_loading();
    
    // Create state
    let state = Arc::new(AppState {
        config: AppConfig {
            data_dir: cli.data_dir,
            model: cli.model,
            cache_size: cli.cache_size,
            batch_size: cli.batch_size,
            timeout_ms: cli.timeout,
            log_level: cli.log_level,
        },
        storage,
        embedding,
    });
    
    // Create and run server
    let server = MemoryMcpServer::new(state);
    rmcp::serve(server, rmcp::transport::stdio()).await?;
    
    Ok(())
}
```

---

### T041: Wire up AppState

Ensure `AppState` in `src/config.rs` has correct Arc types and all fields accessible to server handlers.

---

## Definition of Done

1. `cargo run` starts server within 1 second
2. Server responds to `tools/list` with 5 memory tools
3. All 5 memory tools functional via MCP protocol
4. store_memory completes in < 100ms (after model loaded)
5. Error handling returns tool errors, not protocol errors

## Risks

| Risk | Mitigation |
|------|------------|
| rmcp macro issues | Fallback to manual tool registration |
| ID format consistency | Use nanoid-style 20-char alphanumeric |

## Reviewer Guidance

- Verify all tool descriptions match contracts/mcp-tools.md
- Check error messages are user-friendly
- Confirm embedding not serialized in responses
- Test with stdio transport
