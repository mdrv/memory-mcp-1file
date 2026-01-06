use std::path::PathBuf;
use std::sync::Arc;

use crate::embedding::EmbeddingService;
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
            batch_size: 32,
            timeout_ms: 30000,
            log_level: "info".to_string(),
        }
    }
}

pub struct AppState {
    pub config: AppConfig,
    pub storage: Arc<SurrealStorage>,
    pub embedding: Arc<EmbeddingService>,
}
