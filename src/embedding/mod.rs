pub mod adaptive_queue;
mod cache;
mod cleanup;
mod config;
pub mod coordinator;
mod engine;
pub mod handle;
pub mod hasher;
pub mod metrics;
pub mod policy;
mod service;
pub mod store;
pub mod worker;

pub use adaptive_queue::{AdaptiveEmbeddingQueue, AdaptiveQueueConfig};
pub use cache::{CacheStats, EmbeddingCache};
pub use cleanup::{cleanup_model_cache, CleanupConfig, CleanupResult};
pub use config::{EmbeddingConfig, ModelType};
pub use coordinator::EmbeddingCoordinator;
pub use engine::EmbeddingEngine;
pub use handle::WorkerHandle;
pub use hasher::ContentHasher;
pub use metrics::EmbeddingMetrics;
pub use policy::{EmbedStrategy, EmbeddingPolicy};
pub use service::EmbeddingService;
pub use store::EmbeddingStore;
pub use worker::{EmbeddingRequest, EmbeddingTarget, EmbeddingWorker};

/// Loading phase for detailed progress tracking
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadingPhase {
    Starting,
    CleaningCache,
    FetchingConfig,
    FetchingTokenizer,
    FetchingWeights,
    VerifyingWeights,
    LoadingModel,
    WarmingUp,
}

impl std::fmt::Display for LoadingPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Starting => write!(f, "Starting..."),
            Self::CleaningCache => write!(f, "Cleaning stale cache..."),
            Self::FetchingConfig => write!(f, "Fetching config..."),
            Self::FetchingTokenizer => write!(f, "Fetching tokenizer..."),
            Self::FetchingWeights => write!(f, "Downloading model weights..."),
            Self::VerifyingWeights => write!(f, "Verifying model integrity (hashing)... This is a one-time process for the model loading."),
            Self::LoadingModel => write!(f, "Loading model into memory..."),
            Self::WarmingUp => write!(f, "Warming up model..."),
        }
    }
}

/// Detailed embedding service status with progress info
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum EmbeddingStatus {
    Loading {
        phase: LoadingPhase,
        elapsed_seconds: u64,
        eta_seconds: Option<u64>,
        cached: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        progress_percent: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        downloaded_mb: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        total_mb: Option<f32>,
    },
    Ready,
    Error {
        message: String,
    },
}

impl EmbeddingStatus {
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }

    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading { .. })
    }
}
