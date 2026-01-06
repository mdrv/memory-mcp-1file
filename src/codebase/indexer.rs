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
            if state.embedding.is_ready() {
                if let Ok(emb) = state.embedding.embed(&chunk.content).await {
                    chunk.embedding = Some(emb);
                }
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

    // Flush remaining chunks
    if !chunk_buffer.is_empty() {
        state.storage.create_code_chunks_batch(chunk_buffer).await?;
    }

    status.status = IndexState::Completed;
    status.completed_at = Some(surrealdb::sql::Datetime::default());

    state.storage.update_index_status(status.clone()).await?;

    Ok(status)
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

        // Create 150 files to force batching (batch size is 100)
        for i in 0..150 {
            let file_path = project_dir.join(format!("file_{}.rs", i));
            fs::write(file_path, format!("fn test_{}() {{}}", i)).unwrap();
        }

        let status = index_project(ctx.state.clone(), &project_dir)
            .await
            .unwrap();

        assert_eq!(status.total_files, 150);
        // Each file has 1 chunk
        assert_eq!(status.total_chunks, 150);

        // Verify in DB
        // Using bm25_search_code with a high limit to fetch all
        let chunks = ctx
            .state
            .storage
            .bm25_search_code("fn test", None, 200)
            .await
            .unwrap();
        assert_eq!(chunks.len(), 150);
    }
}
