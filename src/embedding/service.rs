use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::cache::{CacheStats, EmbeddingCache};
use super::config::{EmbeddingConfig, ModelType};
use super::engine::EmbeddingEngine;
use super::EmbeddingStatus;
use crate::types::{AppError, Result};

pub struct EmbeddingService {
    engine: Arc<RwLock<Option<EmbeddingEngine>>>,
    cache: EmbeddingCache,
    config: EmbeddingConfig,
    status: Arc<AtomicU8>,
}

impl EmbeddingService {
    pub fn new(config: EmbeddingConfig) -> Self {
        Self {
            engine: Arc::new(RwLock::new(None)),
            cache: EmbeddingCache::new(config.cache_size),
            config,
            status: Arc::new(AtomicU8::new(0)),
        }
    }

    pub fn start_loading(&self) {
        let engine_state = self.engine.clone();
        let status = self.status.clone();
        let model = self.config.model;

        if model == ModelType::Mock {
            status.store(1, Ordering::Relaxed);
            tracing::info!("Mock embedding model ready");
            return;
        }

        std::thread::spawn(move || {
            tracing::info!("Loading embedding model: {:?}", model);

            match EmbeddingEngine::new(model) {
                Ok(e) => {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("Failed to build runtime");
                    rt.block_on(async {
                        let mut guard = engine_state.write().await;
                        *guard = Some(e);
                    });
                    status.store(1, Ordering::Relaxed);
                    tracing::info!("Embedding model ready");
                }
                Err(e) => {
                    tracing::error!("Failed to load embedding model: {}", e);
                    status.store(2, Ordering::Relaxed);
                }
            }
        });
    }

    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let model_ver = self.config.model.repo_id();
        if let Some(cached) = self.cache.get(text, model_ver) {
            return Ok(cached);
        }

        if self.config.model == ModelType::Mock {
            let dim = self.config.model.dimensions();
            let mut vec = vec![0.0; dim];
            // Simple hash-based deterministic mock embedding
            let hash = blake3::hash(text.as_bytes());
            let bytes = hash.as_bytes();
            for i in 0..dim.min(32) {
                vec[i] = (bytes[i % 32] as f32) / 255.0;
            }
            self.cache.put(text, model_ver, vec.clone());
            return Ok(vec);
        }

        let guard = self.engine.read().await;
        let engine = guard.as_ref().ok_or(AppError::EmbeddingNotReady)?;

        let embedding = engine
            .embed(text)
            .map_err(|e| AppError::Embedding(e.to_string()))?;

        self.cache.put(text, model_ver, embedding.clone());

        Ok(embedding)
    }

    pub fn status(&self) -> EmbeddingStatus {
        match self.status.load(Ordering::Relaxed) {
            0 => EmbeddingStatus::Loading,
            1 => EmbeddingStatus::Ready,
            _ => EmbeddingStatus::Error,
        }
    }

    pub fn model(&self) -> ModelType {
        self.config.model
    }

    pub fn dimensions(&self) -> usize {
        self.config.model.dimensions()
    }

    pub fn cache_stats(&self) -> CacheStats {
        self.cache.stats()
    }
}
