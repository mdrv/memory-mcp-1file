use std::path::PathBuf;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use tokio::sync::mpsc;

use crate::embedding::{EmbeddingRequest, EmbeddingService, EmbeddingStore};
use crate::storage::SurrealStorage;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub data_dir: PathBuf,
    pub model: String,
    pub cache_size: usize,
    pub batch_size: usize,
    pub timeout_ms: u64,
    pub log_level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            data_dir: dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("memory-mcp"),
            model: "e5_multi".to_string(),
            cache_size: 1000,
            batch_size: 8,
            timeout_ms: 30000,
            log_level: "info".to_string(),
        }
    }
}

pub struct IndexMonitor {
    pub total_files: AtomicU32,
    pub indexed_files: AtomicU32,
}

impl Default for IndexMonitor {
    fn default() -> Self {
        Self {
            total_files: AtomicU32::new(0),
            indexed_files: AtomicU32::new(0),
        }
    }
}

pub struct AppState {
    pub config: AppConfig,
    pub storage: Arc<SurrealStorage>,
    pub embedding: Arc<EmbeddingService>,
    pub embedding_store: Arc<EmbeddingStore>,
    pub embedding_queue: mpsc::Sender<EmbeddingRequest>,
    pub monitor: Arc<IndexMonitor>,
}
