use std::path::Path;
use std::sync::Arc;

use tokio::fs;

use crate::config::AppState;
use crate::embedding::EmbeddingStatus;
use crate::storage::StorageBackend;
use crate::types::{IndexState, IndexStatus};
use crate::Result;

use super::chunker::chunk_file;
use super::scanner::scan_directory;

pub async fn index_project(state: Arc<AppState>, project_path: &Path) -> Result<IndexStatus> {
    let project_id = project_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut status = IndexStatus::new(project_id.clone());

    state.storage.delete_project_chunks(&project_id).await?;

    let files = scan_directory(project_path)?;
    status.total_files = files.len() as u32;

    state.storage.update_index_status(status.clone()).await?;

    let mut all_chunks = Vec::new();

    for file_path in &files {
        let content = match fs::read_to_string(file_path).await {
            Ok(c) => c,
            Err(_) => continue,
        };

        let chunks = chunk_file(file_path, &content, &project_id);
        all_chunks.extend(chunks);
        status.indexed_files += 1;
    }

    if state.embedding.status() == EmbeddingStatus::Ready {
        for chunk in &mut all_chunks {
            if let Ok(emb) = state.embedding.embed(&chunk.content).await {
                chunk.embedding = Some(emb);
            }
        }
    }

    status.total_chunks = all_chunks.len() as u32;

    if !all_chunks.is_empty() {
        state.storage.create_code_chunks_batch(all_chunks).await?;
    }

    status.status = IndexState::Completed;
    status.completed_at = Some(chrono::Utc::now());

    state.storage.update_index_status(status.clone()).await?;

    Ok(status)
}
