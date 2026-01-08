use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use super::metrics::EmbeddingMetrics;
use crate::lifecycle::{
    Component, ComponentHealth, HealthStatus, ShutdownPriority, ShutdownResult,
};

pub struct WorkerHandle {
    handle: Mutex<Option<JoinHandle<usize>>>,
    metrics: Arc<EmbeddingMetrics>,
}

impl WorkerHandle {
    pub fn new(handle: JoinHandle<usize>, metrics: Arc<EmbeddingMetrics>) -> Self {
        Self {
            handle: Mutex::new(Some(handle)),
            metrics,
        }
    }

    pub fn metrics(&self) -> &EmbeddingMetrics {
        &self.metrics
    }

    pub async fn take_handle(&self) -> Option<JoinHandle<usize>> {
        self.handle.lock().await.take()
    }
}

#[async_trait]
impl Component for WorkerHandle {
    fn name(&self) -> &'static str {
        "embedding_worker"
    }

    fn shutdown_priority(&self) -> ShutdownPriority {
        ShutdownPriority::Normal
    }

    async fn health(&self) -> ComponentHealth {
        let queue_depth = self.metrics.get_queue_depth();
        if queue_depth > 500 {
            ComponentHealth {
                status: HealthStatus::Degraded {
                    reason: format!("High queue depth: {}", queue_depth),
                },
            }
        } else {
            ComponentHealth::default()
        }
    }

    async fn shutdown(&self, timeout: Duration) -> ShutdownResult {
        let queue_depth = self.metrics.get_queue_depth();
        tracing::info!(queue_depth, "Embedding worker shutting down");

        if let Some(handle) = self.take_handle().await {
            let result = tokio::time::timeout(timeout, handle).await;
            match result {
                Ok(Ok(processed)) => {
                    return ShutdownResult::Complete {
                        items_processed: processed,
                    };
                }
                Ok(Err(e)) => {
                    tracing::error!("Worker task failed: {}", e);
                }
                Err(_) => {
                    tracing::warn!("Worker shutdown timed out");
                }
            }
        }

        let remaining = self.metrics.get_queue_depth();
        if remaining == 0 {
            ShutdownResult::Complete {
                items_processed: self
                    .metrics
                    .processed_total
                    .load(std::sync::atomic::Ordering::Relaxed)
                    as usize,
            }
        } else {
            ShutdownResult::Partial { remaining }
        }
    }

    async fn force_stop(&self) {
        // Nothing to do - handle will be aborted when dropped
        tracing::warn!("Force stopping embedding worker");
    }
}
