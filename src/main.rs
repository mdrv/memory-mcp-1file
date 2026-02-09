use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use memory_mcp::config::{AppConfig, AppState};
use memory_mcp::embedding::{
    EmbeddingConfig, EmbeddingService, EmbeddingStore, EmbeddingWorker, ModelType,
};
use memory_mcp::server::MemoryMcpServer;
use memory_mcp::storage::{StorageBackend, SurrealStorage};

#[derive(Parser)]
#[command(name = "memory-mcp")]
#[command(about = "MCP memory server for AI agents")]
struct Cli {
    #[arg(long, env, default_value_os_t = default_data_dir())]
    data_dir: PathBuf,

    #[arg(long, env = "EMBEDDING_MODEL", default_value = "e5_multi")]
    model: String,

    #[arg(long, env, default_value = "1000")]
    cache_size: usize,

    #[arg(long, env, default_value = "8")]
    batch_size: usize,

    #[arg(long, env = "TIMEOUT_MS", default_value = "30000")]
    timeout: u64,

    #[arg(long, env = "LOG_LEVEL", default_value = "info")]
    log_level: String,

    /// Idle timeout in minutes. Server exits if no requests for this duration. 0 = disabled.
    #[arg(long, env, default_value = "30")]
    idle_timeout: u64,

    /// Reconnect timeout in seconds before shutdown after connection loss.
    #[arg(long, env, default_value = "10")]
    reconnect_timeout: u64,

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

    tracing_subscriber::fmt()
        .with_env_filter(&cli.log_level)
        .with_writer(std::io::stderr)
        .init();

    let storage = Arc::new(SurrealStorage::new(&cli.data_dir).await?);

    let model: ModelType = cli.model.parse().map_err(|e: String| anyhow::anyhow!(e))?;

    if let Err(e) = storage.check_dimension(model.dimensions()).await {
        tracing::error!("{}", e);
        std::process::exit(1);
    }

    // Initialize Embedding Store (L1/L2 Cache)
    let embedding_store = Arc::new(EmbeddingStore::new(&cli.data_dir, model.repo_id())?);

    let embedding_config = EmbeddingConfig {
        model,
        cache_size: cli.cache_size,
        batch_size: cli.batch_size,
        cache_dir: Some(cli.data_dir.join("models")),
    };
    let embedding = Arc::new(EmbeddingService::new(embedding_config));
    embedding.start_loading();

    let metrics = std::sync::Arc::new(memory_mcp::embedding::EmbeddingMetrics::new());
    let (queue_tx, queue_rx) = tokio::sync::mpsc::channel(1000);
    let adaptive_queue =
        memory_mcp::embedding::AdaptiveEmbeddingQueue::with_defaults(queue_tx, metrics.clone());

    let state = Arc::new(AppState {
        config: AppConfig {
            data_dir: cli.data_dir,
            model: cli.model,
            cache_size: cli.cache_size,
            batch_size: cli.batch_size,
            timeout_ms: cli.timeout,
            log_level: cli.log_level,
        },
        storage: storage.clone(),
        embedding: embedding.clone(),
        embedding_store: embedding_store.clone(),
        embedding_queue: adaptive_queue,
        progress: memory_mcp::config::IndexProgressTracker::new(),
        db_semaphore: Arc::new(tokio::sync::Semaphore::new(10)),
    });

    let worker = EmbeddingWorker::new(
        queue_rx,
        embedding.get_engine(),
        embedding_store.clone(),
        state.clone(),
    );
    tokio::spawn(async move {
        match tokio::spawn(worker.run()).await {
            Ok(count) => tracing::info!(count, "Embedding worker finished"),
            Err(e) => tracing::error!("Embedding worker panicked: {}", e),
        }
    });

    let monitor_state = state.clone();
    tokio::spawn(memory_mcp::embedding::run_completion_monitor(monitor_state));

    let server = MemoryMcpServer::new(state.clone());

    // Auto-start codebase manager if /project exists
    let transport = rmcp::transport::io::stdio();

    let service = rmcp::service::serve_server(server, transport).await?;

    tracing::info!(
        reconnect_timeout_sec = cli.reconnect_timeout,
        "Server started, waiting for signals..."
    );

    #[cfg(unix)]
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;

    let reconnect_timeout = Duration::from_secs(cli.reconnect_timeout);
    let shutdown_reason: &str;

    tokio::select! {
        res = service.waiting() => {
            match res {
                Err(e) => {
                    tracing::error!("Server error: {}", e);
                    shutdown_reason = "server_error";
                }
                Ok(_) => {
                    tracing::info!(
                        timeout_sec = cli.reconnect_timeout,
                        "Connection closed, waiting for reconnect..."
                    );

                    let reconnected = tokio::select! {
                        _ = tokio::time::sleep(reconnect_timeout) => false,
                        _ = tokio::signal::ctrl_c() => {
                            tracing::info!("Received SIGINT during reconnect wait");
                            false
                        }
                    };

                    if reconnected {
                        shutdown_reason = "reconnected";
                    } else {
                        tracing::info!("No reconnect within timeout, shutting down");
                        shutdown_reason = "connection_timeout";
                    }
                }
            }
        },
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Shutting down gracefully... (SIGINT)");
            shutdown_reason = "sigint";
        },
        _ = async {
            #[cfg(unix)]
            {
                terminate.recv().await;
            }
            #[cfg(not(unix))]
            {
                std::future::pending::<()>().await;
            }
        } => {
            tracing::info!("Shutting down gracefully... (SIGTERM)");
            shutdown_reason = "sigterm";
        }
    }

    tracing::info!(reason = shutdown_reason, "Initiating graceful shutdown...");

    tracing::info!("Flushing database...");
    if let Err(e) = state.storage.shutdown().await {
        tracing::warn!("Database shutdown error: {}", e);
    }

    tracing::info!("Shutdown complete");
    Ok(())
}
