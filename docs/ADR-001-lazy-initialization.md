# ADR-001: Lazy Initialization for Fast MCP Handshake

**Status:** Proposed  
**Date:** 2026-01-12  
**Context:** "Failed to get tools" error due to slow server initialization

## Problem

MCP clients (OpenCode, Claude Desktop) timeout waiting for `tools/list` response because server blocks on SurrealDB initialization before starting MCP handshake.

```
Current timeline:
0ms      → SurrealStorage::new().await (BLOCKING ~500ms+)
500ms+   → serve_server() starts
600ms+   → MCP handshake begins
???      → Client timeout (default 5s in some configs)
         → "Failed to get tools"
```

## Decision

Implement **Lazy Initialization with Graceful Degradation**:

1. Start MCP server immediately with `OnceCell<SurrealStorage>`
2. Initialize storage in background via `tokio::spawn`
3. Return graceful errors for tool calls during initialization
4. Add `get_status` tool for clients to check readiness

## Solution Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│ NEW INITIALIZATION TIMELINE                                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  0ms     LazyState::new() - storage: OnceCell::const_new()     │
│  1ms     tokio::spawn(init_storage_background())               │
│  2ms     serve_server() STARTS ← MCP handshake immediate       │
│  10ms    tools/list responds (all tools, static list)          │
│  ~500ms  Storage ready, tools fully functional                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Implementation

### 1. LazyState (config.rs)

```rust
use tokio::sync::OnceCell;
use std::sync::atomic::{AtomicU8, Ordering};

pub const STATUS_INITIALIZING: u8 = 0;
pub const STATUS_READY: u8 = 1;
pub const STATUS_ERROR: u8 = 2;

pub struct LazyState {
    pub config: AppConfig,
    pub storage: Arc<OnceCell<SurrealStorage>>,
    pub embedding: Arc<EmbeddingService>,
    pub embedding_store: Arc<OnceCell<EmbeddingStore>>,
    pub init_status: Arc<AtomicU8>,
    pub init_error: Arc<RwLock<Option<String>>>,
    // ... other fields
}

impl LazyState {
    pub fn new(config: AppConfig, embedding: Arc<EmbeddingService>) -> Self {
        Self {
            config,
            storage: Arc::new(OnceCell::const_new()),
            embedding,
            embedding_store: Arc::new(OnceCell::const_new()),
            init_status: Arc::new(AtomicU8::new(STATUS_INITIALIZING)),
            init_error: Arc::new(RwLock::new(None)),
        }
    }
    
    pub fn is_ready(&self) -> bool {
        self.init_status.load(Ordering::Acquire) == STATUS_READY
    }
}
```

### 2. Background Initialization (main.rs)

```rust
async fn main() -> anyhow::Result<()> {
    // ... parse args, setup logging
    
    let embedding = Arc::new(EmbeddingService::new(/* ... */));
    embedding.start_loading();
    
    let state = Arc::new(LazyState::new(config, embedding));
    let server = MemoryMcpServer::new(state.clone());
    
    // Background initialization
    let init_state = state.clone();
    let data_dir = data_dir.clone();
    tokio::spawn(async move {
        match init_storage(&data_dir, &init_state).await {
            Ok(()) => {
                init_state.init_status.store(STATUS_READY, Ordering::Release);
                tracing::info!("Storage initialization complete");
            }
            Err(e) => {
                *init_state.init_error.write().await = Some(e.to_string());
                init_state.init_status.store(STATUS_ERROR, Ordering::Release);
                tracing::error!("Storage initialization failed: {}", e);
            }
        }
    });
    
    // MCP starts IMMEDIATELY
    let transport = rmcp::transport::io::stdio();
    serve_server(server, transport).await?;
    
    Ok(())
}

async fn init_storage(data_dir: &Path, state: &LazyState) -> anyhow::Result<()> {
    let storage = SurrealStorage::new(data_dir).await?;
    storage.check_dimension(state.config.embedding_dimensions).await?;
    
    let embedding_store = EmbeddingStore::new(/* ... */);
    
    state.storage.set(storage).map_err(|_| anyhow!("Storage already set"))?;
    state.embedding_store.set(embedding_store).map_err(|_| anyhow!("EmbeddingStore already set"))?;
    
    Ok(())
}
```

### 3. Graceful Tool Errors (handler.rs)

```rust
impl MemoryMcpServer {
    /// Get storage or return graceful MCP error
    fn require_storage(&self) -> Result<&SurrealStorage, ErrorData> {
        match self.state.init_status.load(Ordering::Acquire) {
            STATUS_INITIALIZING => Err(ErrorData::new(
                ErrorCode::ServerError(-32002),
                "Server initializing, please retry in 1-2 seconds",
                None::<()>,
            )),
            STATUS_ERROR => {
                let error = self.state.init_error.blocking_read();
                Err(ErrorData::new(
                    ErrorCode::ServerError(-32003),
                    format!("Server initialization failed: {}", error.as_deref().unwrap_or("unknown")),
                    None::<()>,
                ))
            }
            STATUS_READY => self.state.storage.get().ok_or_else(|| {
                ErrorData::new(ErrorCode::InternalError, "Storage not available", None::<()>)
            }),
            _ => unreachable!(),
        }
    }
}

// Usage in tools:
#[tool(description = "Store a memory")]
async fn store_memory(&self, params: StoreMemoryParams) -> Result<ToolResult, ErrorData> {
    let storage = self.require_storage()?;
    // ... rest of implementation
}
```

### 4. Status Tool (handler.rs)

```rust
#[tool(description = "Get server initialization status and health")]
async fn get_status(&self) -> Result<ToolResult, ErrorData> {
    let status = match self.state.init_status.load(Ordering::Acquire) {
        STATUS_INITIALIZING => "initializing",
        STATUS_READY => "ready",
        STATUS_ERROR => "error",
        _ => "unknown",
    };
    
    let error = self.state.init_error.read().await.clone();
    
    Ok(ToolResult::text(serde_json::to_string_pretty(&json!({
        "status": status,
        "storage_ready": self.state.storage.get().is_some(),
        "embedding_ready": self.state.embedding.is_ready(),
        "embedding_model": self.state.config.embedding_model,
        "error": error,
    }))?))
}
```

## Error Handling Matrix

| State        | tools/list | tool/call (storage-dependent) |
|--------------|------------|-------------------------------|
| INITIALIZING | All tools  | Error: "initializing, retry"  |
| READY        | All tools  | Normal execution              |
| ERROR        | All tools  | Error: "init failed: {msg}"   |

## Alternatives Considered

| # | Alternative | Why Not Chosen |
|---|-------------|----------------|
| 1 | Increase client timeout | Doesn't fix root cause, poor UX |
| 2 | Empty tools/list + notify | Client sees empty tools initially |
| 3 | Two-phase init | More complex, requires custom ServerHandler |
| 4 | Background thread (std::thread) | Harder sync, two runtimes |
| 5 | Blocking init with progress | Still blocks handshake |

## Consequences

### Positive
- MCP handshake completes in <10ms
- No more "Failed to get tools" errors
- Graceful error messages during initialization
- Backward compatible (tools list unchanged)
- `get_status` tool for debugging

### Negative
- Tool calls may fail during first 1-2 seconds
- Slightly more complex codebase
- Need to update all storage-dependent tools

### Neutral
- Client may need retry logic (most already have it)

## References

- [SEP-1539: MCP Timeout Coordination](https://github.com/modelcontextprotocol/modelcontextprotocol/issues/1539)
- [tokio::sync::OnceCell](https://docs.rs/tokio/latest/tokio/sync/struct.OnceCell.html)
- [MCP Specification: tools/list](https://spec.modelcontextprotocol.io/specification/server/tools/)
