use async_trait::async_trait;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ShutdownPriority {
    First = 0,
    #[default]
    Normal = 50,
    Last = 100,
}

#[derive(Debug)]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
}

#[derive(Debug)]
pub struct ComponentHealth {
    pub status: HealthStatus,
}

impl Default for ComponentHealth {
    fn default() -> Self {
        Self {
            status: HealthStatus::Healthy,
        }
    }
}

#[derive(Debug)]
pub enum ShutdownResult {
    Complete { items_processed: usize },
    Partial { remaining: usize },
    Error(String),
}

#[async_trait]
pub trait Component: Send + Sync {
    fn name(&self) -> &'static str;

    fn shutdown_priority(&self) -> ShutdownPriority {
        ShutdownPriority::Normal
    }

    async fn health(&self) -> ComponentHealth {
        ComponentHealth::default()
    }

    async fn shutdown(&self, timeout: Duration) -> ShutdownResult;

    async fn force_stop(&self);
}
