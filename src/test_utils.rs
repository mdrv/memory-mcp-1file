use std::sync::Arc;
use tempfile::TempDir;

use crate::config::{AppConfig, AppState};
use crate::embedding::{EmbeddingConfig, EmbeddingService, ModelType};
use crate::storage::SurrealStorage;

pub struct TestContext {
    pub state: Arc<AppState>,
    pub _temp_dir: TempDir, // Kept to ensure directory lives as long as context
}

impl TestContext {
    pub async fn new() -> Self {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path();

        // Initialize Storage
        let storage = Arc::new(
            SurrealStorage::new(db_path)
                .await
                .expect("Failed to init storage"),
        );

        // Initialize Mock Embedding
        let embedding_config = EmbeddingConfig {
            model: ModelType::Mock,
            cache_size: 100,
            batch_size: 10,
        };
        let embedding = Arc::new(EmbeddingService::new(embedding_config));
        embedding.start_loading();

        // Wait for embedding service to be ready (usually instant for Mock)
        let mut attempts = 0;
        while embedding.status() != crate::embedding::EmbeddingStatus::Ready {
            if attempts > 10 {
                panic!("Mock embedding service failed to start");
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            attempts += 1;
        }

        let config = AppConfig {
            data_dir: db_path.to_path_buf(),
            model: "mock".to_string(),
            cache_size: 100,
            batch_size: 10,
            timeout_ms: 5000,
            log_level: "debug".to_string(),
        };

        let state = Arc::new(AppState {
            config,
            storage,
            embedding,
        });

        Self {
            state,
            _temp_dir: temp_dir,
        }
    }
}
