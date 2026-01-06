mod cache;
mod config;
mod engine;
mod service;

pub use cache::{CacheStats, EmbeddingCache};
pub use config::{EmbeddingConfig, ModelType};
pub use engine::EmbeddingEngine;
pub use service::EmbeddingService;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingStatus {
    Loading,
    Ready,
    Error,
}
