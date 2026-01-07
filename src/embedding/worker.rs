use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};
use tracing::instrument;

use super::engine::EmbeddingEngine;
use super::store::EmbeddingStore;

#[derive(Debug)]
pub enum EmbeddingTarget {
    Symbol(String),
    Chunk(String),
}

pub struct EmbeddingRequest {
    pub text: String,
    pub responder: Option<oneshot::Sender<Vec<f32>>>,
    pub target: Option<EmbeddingTarget>,
}

pub struct EmbeddingWorker {
    queue: mpsc::Receiver<EmbeddingRequest>,
    engine: Arc<tokio::sync::RwLock<Option<EmbeddingEngine>>>,
    store: Arc<EmbeddingStore>,
    storage: Arc<crate::storage::SurrealStorage>,
}

impl EmbeddingWorker {
    pub fn new(
        queue: mpsc::Receiver<EmbeddingRequest>,
        engine: Arc<tokio::sync::RwLock<Option<EmbeddingEngine>>>,
        store: Arc<EmbeddingStore>,
        state: Arc<crate::config::AppState>,
    ) -> Self {
        Self {
            queue,
            engine,
            store,
            storage: state.storage.clone(),
        }
    }

    pub async fn run(mut self) {
        let mut batch = Vec::with_capacity(32);
        let deadline = tokio::time::sleep(Duration::from_millis(100));
        tokio::pin!(deadline);

        loop {
            // Process pending batch if ready
            if !batch.is_empty() {
                if !self.process_batch(&mut batch).await {
                    // Engine not ready, retry after delay
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                }
                deadline
                    .as_mut()
                    .reset(tokio::time::Instant::now() + Duration::from_millis(100));
            }

            tokio::select! {
                Some(req) = self.queue.recv() => {
                    batch.push(req);
                    if batch.len() >= 32 {
                        if !self.process_batch(&mut batch).await {
                            // Engine not ready, retry loop
                            tokio::time::sleep(Duration::from_millis(500)).await;
                        } else {
                            deadline.as_mut().reset(tokio::time::Instant::now() + Duration::from_millis(100));
                        }
                    }
                }
                _ = &mut deadline => {
                    if !batch.is_empty() {
                        if !self.process_batch(&mut batch).await {
                            // Retry loop
                        } else {
                            deadline.as_mut().reset(tokio::time::Instant::now() + Duration::from_millis(100));
                        }
                    } else {
                        deadline.as_mut().reset(tokio::time::Instant::now() + Duration::from_millis(100));
                    }
                }
            }
        }
    }

    #[instrument(skip(self, batch), fields(batch_size = batch.len()))]
    async fn process_batch(&self, batch: &mut Vec<EmbeddingRequest>) -> bool {
        if batch.is_empty() {
            return true;
        }

        let guard = self.engine.read().await;
        let engine = match guard.as_ref() {
            Some(e) => e,
            None => {
                // Return false to indicate retry needed
                return false;
            }
        };

        let mut final_embeddings = Vec::with_capacity(batch.len());
        let mut misses_indices = Vec::new();
        let mut misses_texts = Vec::new();

        for (i, req) in batch.iter().enumerate() {
            let hash = blake3::hash(req.text.as_bytes()).to_hex().to_string();

            if let Some(vec) = self.store.get(&hash).await {
                final_embeddings.push(Some(vec));
            } else {
                final_embeddings.push(None);
                misses_indices.push(i);
                misses_texts.push(req.text.clone());
            }
        }

        if !misses_texts.is_empty() {
            match engine.embed_batch(&misses_texts) {
                Ok(new_embeddings) => {
                    for (local_idx, vec) in new_embeddings.into_iter().enumerate() {
                        let original_idx = misses_indices[local_idx];
                        let req = &batch[original_idx];
                        let hash = blake3::hash(req.text.as_bytes()).to_hex().to_string();

                        let _ = self.store.put(hash, vec.clone()).await;
                        final_embeddings[original_idx] = Some(vec);
                    }
                }
                Err(e) => {
                    tracing::error!("Batch embedding failed: {}", e);
                    // Drop failing batch to avoid stuck loop on inference error
                }
            }
        }

        for (req, emb_opt) in batch.drain(..).zip(final_embeddings) {
            if let Some(emb) = emb_opt {
                if let Some(tx) = req.responder {
                    let _ = tx.send(emb.clone());
                }

                if let Some(target) = req.target {
                    let storage = self.storage.clone();
                    let embedding = emb;
                    tokio::spawn(async move {
                        use crate::storage::StorageBackend;
                        match target {
                            EmbeddingTarget::Symbol(id) => {
                                if let Err(e) =
                                    storage.update_symbol_embedding(&id, embedding).await
                                {
                                    tracing::warn!("Failed to update symbol embedding: {}", e);
                                }
                            }
                            EmbeddingTarget::Chunk(id) => {
                                if let Err(e) = storage.update_chunk_embedding(&id, embedding).await
                                {
                                    tracing::warn!("Failed to update chunk embedding: {}", e);
                                }
                            }
                        }
                    });
                }
            } else {
                if let Some(tx) = req.responder {
                    let _ = tx.send(vec![]);
                }
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::{EmbeddingConfig, EmbeddingService, ModelType};
    use crate::storage::SurrealStorage;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_worker_initialization() {
        let dir = tempdir().unwrap();
        let storage = Arc::new(SurrealStorage::new(dir.path()).await.unwrap());
        let store = Arc::new(EmbeddingStore::new(dir.path()).unwrap());

        let config = EmbeddingConfig {
            model: ModelType::Mock,
            cache_size: 100,
            batch_size: 10,
            cache_dir: None,
        };
        let service = Arc::new(EmbeddingService::new(config));

        let (tx, rx) = mpsc::channel(100);

        let _worker = EmbeddingWorker::new(
            rx,
            service.get_engine(),
            store.clone(),
            Arc::new(crate::config::AppState {
                config: crate::config::AppConfig::default(),
                storage,
                embedding: service,
                embedding_store: store,
                embedding_queue: tx,
                monitor: Arc::new(crate::config::IndexMonitor::default()),
            }),
        );
    }
}
