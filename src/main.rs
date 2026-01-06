use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;

use memory_mcp::config::{AppConfig, AppState};
use memory_mcp::embedding::{EmbeddingConfig, EmbeddingService, ModelType};
use memory_mcp::server::MemoryMcpServer;
use memory_mcp::storage::SurrealStorage;

#[derive(Parser)]
#[command(name = "memory-mcp")]
#[command(about = "MCP memory server for AI agents")]
struct Cli {
    #[arg(long, default_value_os_t = default_data_dir())]
    data_dir: PathBuf,

    #[arg(long, default_value = "e5_multi")]
    model: String,

    #[arg(long, default_value = "1000")]
    cache_size: usize,

    #[arg(long, default_value = "32")]
    batch_size: usize,

    #[arg(long, default_value = "30000")]
    timeout: u64,

    #[arg(long, default_value = "info")]
    log_level: String,

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

    let embedding_config = EmbeddingConfig {
        model,
        cache_size: cli.cache_size,
        batch_size: cli.batch_size,
    };
    let embedding = Arc::new(EmbeddingService::new(embedding_config));
    embedding.start_loading();

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

    let server = MemoryMcpServer::new(state.clone());
    let transport = rmcp::transport::io::stdio();

    let service = rmcp::service::serve_server(server, transport).await?;

    tracing::info!("Server started, waiting for signals...");

    #[cfg(unix)]
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;

    tokio::select! {
        res = service.waiting() => {
            if let Err(e) = res {
                tracing::error!("Server error: {}", e);
            }
        },
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Shutting down gracefully... (SIGINT)");
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
        }
    }

    tracing::info!("Closing database connections...");
    drop(state);

    tracing::info!("Shutdown complete");
    Ok(())
}
