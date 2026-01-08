use std::sync::Arc;
use tokio::sync::mpsc;

use crate::types::{EmbedResult, EmbedTarget};

use super::hasher::ContentHasher;
use super::policy::{EmbedStrategy, EmbeddingPolicy};
use super::service::EmbeddingService;
use super::worker::EmbeddingRequest;

pub struct EmbeddingCoordinator {
    service: Arc<EmbeddingService>,
    queue: mpsc::Sender<EmbeddingRequest>,
}

impl EmbeddingCoordinator {
    pub fn new(service: Arc<EmbeddingService>, queue: mpsc::Sender<EmbeddingRequest>) -> Self {
        Self { service, queue }
    }

    pub async fn embed_for_record(
        &self,
        target: EmbedTarget,
        content: &str,
        old_content_hash: Option<&str>,
    ) -> anyhow::Result<EmbedResult> {
        if !ContentHasher::needs_reembed(old_content_hash, content) {
            return Ok(EmbedResult::Unchanged);
        }

        let new_hash = ContentHasher::hash(content);

        match EmbeddingPolicy::decide(target, content.len()) {
            EmbedStrategy::Sync => {
                let embedding = self.service.embed(content).await?;
                Ok(EmbedResult::Ready {
                    embedding,
                    content_hash: new_hash,
                })
            }
            EmbedStrategy::Async => {
                let req = EmbeddingRequest {
                    text: content.to_string(),
                    responder: None,
                    target: Some(super::worker::EmbeddingTarget::Chunk(String::new())),
                    retry_count: 0,
                };
                self.queue.send(req).await?;
                Ok(EmbedResult::Pending {
                    content_hash: new_hash,
                })
            }
        }
    }

    pub async fn embed_sync(&self, content: &str) -> anyhow::Result<(Vec<f32>, String)> {
        let hash = ContentHasher::hash(content);
        let embedding = self.service.embed(content).await?;
        Ok((embedding, hash))
    }
}
