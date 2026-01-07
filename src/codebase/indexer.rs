use std::path::Path;
use std::sync::Arc;

use tokio::fs;

use crate::config::AppState;
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

    if let Err(e) = state.embedding.wait_for_ready().await {
        tracing::error!("Skipping indexing because model failed to load: {}", e);
        status.status = IndexState::Failed;
        state.storage.update_index_status(status.clone()).await?;
        return Err(e);
    }

    let batch_size = 100;
    let mut chunk_buffer = Vec::with_capacity(batch_size);

    for file_path in &files {
        let content = match fs::read_to_string(file_path).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to read file {:?}: {}", file_path, e);
                continue;
            }
        };

        let chunks = chunk_file(file_path, &content, &project_id);

        for mut chunk in chunks {
            if let Ok(emb) = state.embedding.embed(&chunk.content).await {
                chunk.embedding = Some(emb);
            }

            chunk_buffer.push(chunk);
            status.total_chunks += 1;

            if chunk_buffer.len() >= batch_size {
                state
                    .storage
                    .create_code_chunks_batch(std::mem::take(&mut chunk_buffer))
                    .await?;
            }
        }

        status.indexed_files += 1;
    }

    if !chunk_buffer.is_empty() {
        state.storage.create_code_chunks_batch(chunk_buffer).await?;
    }

    status.status = IndexState::Completed;
    status.completed_at = Some(surrealdb::sql::Datetime::default());

    state.storage.update_index_status(status.clone()).await?;

    Ok(status)
}

/// Incremental re-index for changed files only
pub async fn incremental_index(
    state: Arc<AppState>,
    project_id: &str,
    changed_paths: Vec<std::path::PathBuf>,
) -> Result<usize> {
    let mut updated = 0;

    for path in changed_paths {
        let path_str = path.to_string_lossy().to_string();

        if !path.exists() {
            match state
                .storage
                .delete_chunks_by_path(project_id, &path_str)
                .await
            {
                Ok(deleted) => {
                    if deleted > 0 {
                        tracing::debug!(path = %path_str, deleted, "Removed chunks for deleted file");
                        updated += 1;
                    }
                }
                Err(e) => {
                    tracing::warn!(path = %path_str, error = %e, "Failed to delete chunks");
                }
            }
            continue;
        }

        let content = match tokio::fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(path = %path_str, error = %e, "Failed to read file");
                continue;
            }
        };

        let new_hash = blake3::hash(content.as_bytes()).to_hex().to_string();

        let existing_chunks = state
            .storage
            .get_chunks_by_path(project_id, &path_str)
            .await
            .unwrap_or_default();

        if let Some(first_chunk) = existing_chunks.first() {
            if first_chunk.content_hash == new_hash {
                continue;
            }
        }

        let _ = state
            .storage
            .delete_chunks_by_path(project_id, &path_str)
            .await;

        let chunks = super::chunker::chunk_file(&path, &content, project_id);

        if let Err(e) = state.embedding.wait_for_ready().await {
            tracing::warn!(
                "Skipping incremental index for {:?}: model not ready ({})",
                path,
                e
            );
            continue;
        }

        for mut chunk in chunks {
            if let Ok(emb) = state.embedding.embed(&chunk.content).await {
                chunk.embedding = Some(emb);
            }

            if let Err(e) = state.storage.create_code_chunk(chunk).await {
                tracing::warn!(path = %path_str, error = %e, "Failed to create chunk");
            }
        }

        updated += 1;
    }

    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestContext;
    use std::fs;

    #[tokio::test]
    async fn test_indexer_batching() {
        let ctx = TestContext::new().await;
        let project_dir = ctx._temp_dir.path().join("test_project");
        fs::create_dir_all(&project_dir).unwrap();

        for i in 0..150 {
            let file_path = project_dir.join(format!("file_{}.rs", i));
            fs::write(file_path, format!("fn test_{}() {{}}", i)).unwrap();
        }

        let status = index_project(ctx.state.clone(), &project_dir)
            .await
            .unwrap();

        assert_eq!(status.total_files, 150);
        assert_eq!(status.total_chunks, 150);

        let chunks = ctx
            .state
            .storage
            .bm25_search_code("fn test", None, 200)
            .await
            .unwrap();
        assert_eq!(chunks.len(), 150);
    }
}
