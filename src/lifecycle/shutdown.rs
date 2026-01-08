use super::{ComponentRegistry, ShutdownResult};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownPhase {
    Running,
    DrainQueues,
    FlushStorage,
    ForceStop,
    Complete,
}

pub struct ShutdownCoordinator {
    registry: Arc<ComponentRegistry>,
    phase_tx: watch::Sender<ShutdownPhase>,
    phase_rx: watch::Receiver<ShutdownPhase>,
}

impl ShutdownCoordinator {
    pub fn new(registry: Arc<ComponentRegistry>) -> Self {
        let (phase_tx, phase_rx) = watch::channel(ShutdownPhase::Running);
        Self {
            registry,
            phase_tx,
            phase_rx,
        }
    }

    pub fn phase_receiver(&self) -> watch::Receiver<ShutdownPhase> {
        self.phase_rx.clone()
    }

    pub async fn shutdown(&self, total_timeout: Duration) -> Vec<(&'static str, ShutdownResult)> {
        let phase_timeout = total_timeout / 3;
        let mut results = Vec::new();

        let mut components = self.registry.get_all().await;
        components.sort_by_key(|c| c.shutdown_priority());

        let _ = self.phase_tx.send(ShutdownPhase::DrainQueues);
        tracing::info!("Shutdown Phase 1: Draining queues");

        for component in &components {
            let name = component.name();
            tracing::info!(name, "Shutting down component");

            let result =
                tokio::time::timeout(phase_timeout, component.shutdown(phase_timeout)).await;

            let shutdown_result = match result {
                Ok(r) => r,
                Err(_) => ShutdownResult::Partial { remaining: 0 },
            };

            results.push((name, shutdown_result));
        }

        let _ = self.phase_tx.send(ShutdownPhase::FlushStorage);
        tracing::info!("Shutdown Phase 2: Flushing storage");

        let _ = self.phase_tx.send(ShutdownPhase::ForceStop);
        tracing::info!("Shutdown Phase 3: Force stopping remaining");

        for component in components.iter().rev() {
            component.force_stop().await;
        }

        let _ = self.phase_tx.send(ShutdownPhase::Complete);
        tracing::info!("Shutdown complete");

        results
    }
}
