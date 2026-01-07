use std::sync::Arc;

use rmcp::model::CallToolResult;
use serde_json::json;

use crate::config::AppState;
use crate::server::params::{
    DeleteProjectParams, GetIndexStatusParams, IndexProjectParams, ListProjectsParams,
    SearchCodeParams,
};
use crate::storage::StorageBackend;

use super::{error_response, normalize_limit, success_json, success_serialize};

pub async fn index_project(
    state: &Arc<AppState>,
    params: IndexProjectParams,
) -> anyhow::Result<CallToolResult> {
    let path = std::path::Path::new(&params.path);

    if !path.exists() {
        return Ok(error_response(format!(
            "Path does not exist: {}",
            params.path
        )));
    }

    let project_id = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Check current status
    if let Ok(Some(status)) = state.storage.get_index_status(&project_id).await {
        match status.status {
            crate::types::IndexState::Indexing => {
                // Already indexing - return current progress
                return Ok(success_json(json!({
                    "project_id": project_id,
                    "status": "indexing",
                    "total_files": status.total_files,
                    "indexed_files": status.indexed_files,
                    "total_chunks": status.total_chunks,
                    "message": "Indexing already in progress"
                })));
            }
            crate::types::IndexState::Completed => {
                // Already completed - return status
                return Ok(success_json(json!({
                    "project_id": project_id,
                    "status": "completed",
                    "total_files": status.total_files,
                    "indexed_files": status.indexed_files,
                    "total_chunks": status.total_chunks,
                    "message": "Already indexed. Use delete_project first to re-index."
                })));
            }
            _ => {}
        }
    }

    // Spawn indexing in background
    let state_clone = state.clone();
    let path_clone = params.path.clone();

    tokio::spawn(async move {
        let path = std::path::Path::new(&path_clone);
        match crate::codebase::index_project(state_clone, path).await {
            Ok(status) => {
                tracing::info!(
                    project_id = %status.project_id,
                    files = status.indexed_files,
                    chunks = status.total_chunks,
                    "Indexing completed"
                );
            }
            Err(e) => {
                tracing::error!("Indexing failed: {}", e);
            }
        }
    });

    // Return immediately
    Ok(success_json(json!({
        "project_id": project_id,
        "status": "indexing",
        "message": "Indexing started in background. Use get_index_status to check progress."
    })))
}

pub async fn search_code(
    state: &Arc<AppState>,
    params: SearchCodeParams,
) -> anyhow::Result<CallToolResult> {
    crate::ensure_embedding_ready!(state);

    // Check if project is being indexed
    if let Some(ref project_id) = params.project_id {
        if let Ok(Some(status)) = state.storage.get_index_status(project_id).await {
            if status.status == crate::types::IndexState::Indexing {
                return Ok(success_json(json!({
                    "status": "indexing",
                    "project_id": project_id,
                    "progress": format!("{}/{} files", status.indexed_files, status.total_files),
                    "total_chunks": status.total_chunks,
                    "message": "Indexing in progress. Results may be incomplete."
                })));
            }
        }
    }

    let query_embedding = state.embedding.embed(&params.query).await?;

    let limit = normalize_limit(params.limit);
    match state
        .storage
        .vector_search_code(&query_embedding, params.project_id.as_deref(), limit)
        .await
    {
        Ok(results) => Ok(success_json(json!({
            "results": results,
            "count": results.len(),
            "query": params.query
        }))),
        Err(e) => Ok(error_response(e)),
    }
}

pub async fn get_index_status(
    state: &Arc<AppState>,
    params: GetIndexStatusParams,
) -> anyhow::Result<CallToolResult> {
    match state.storage.get_index_status(&params.project_id).await {
        Ok(Some(status)) => Ok(success_serialize(&status)),
        Ok(None) => Ok(error_response(format!(
            "Project not found: {}",
            params.project_id
        ))),
        Err(e) => Ok(error_response(e)),
    }
}

pub async fn list_projects(
    state: &Arc<AppState>,
    _params: ListProjectsParams,
) -> anyhow::Result<CallToolResult> {
    match state.storage.list_projects().await {
        Ok(projects) => Ok(success_json(json!({
            "projects": projects,
            "count": projects.len()
        }))),
        Err(e) => Ok(error_response(e)),
    }
}

pub async fn delete_project(
    state: &Arc<AppState>,
    params: DeleteProjectParams,
) -> anyhow::Result<CallToolResult> {
    match state
        .storage
        .delete_project_chunks(&params.project_id)
        .await
    {
        Ok(deleted) => Ok(success_json(json!({
            "deleted_chunks": deleted,
            "project_id": params.project_id
        }))),
        Err(e) => Ok(error_response(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestContext;
    use std::fs;

    #[tokio::test]
    async fn test_code_logic_flow() {
        let ctx = TestContext::new().await;
        let project_path = ctx._temp_dir.path().join("test_project_logic");
        fs::create_dir_all(&project_path).unwrap();
        fs::write(
            project_path.join("main.rs"),
            "fn main() { println!(\"Hello\"); }",
        )
        .unwrap();

        let index_params = IndexProjectParams {
            path: project_path.to_string_lossy().to_string(),
        };

        // 1. Trigger Indexing
        let result = index_project(&ctx.state, index_params).await.unwrap();
        // Should return "indexing" status immediately
        if let rmcp::model::RawContent::Text(t) = &result.content[0].raw {
            assert!(t.text.contains("indexing"));
        } else {
            panic!("Expected text content");
        }

        // 2. Wait for indexing to complete
        // Since it's a background task, we poll get_index_status
        let status_params = GetIndexStatusParams {
            project_id: "test_project_logic".to_string(),
        };

        let mut retries = 0;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            let res = get_index_status(&ctx.state, status_params.clone())
                .await
                .unwrap();
            if let rmcp::model::RawContent::Text(t) = &res.content[0].raw {
                if t.text.contains("completed") {
                    break;
                }
            }
            retries += 1;
            if retries > 50 {
                panic!("Indexing timed out");
            }
        }

        // 3. Search Code
        let search_params = SearchCodeParams {
            query: "Hello".to_string(),
            project_id: Some("test_project_logic".to_string()),
            limit: Some(5),
        };
        let search_res = search_code(&ctx.state, search_params).await.unwrap();

        if let rmcp::model::RawContent::Text(t) = &search_res.content[0].raw {
            assert!(t.text.contains("main.rs"));
            assert!(t.text.contains("Hello"));
        } else {
            panic!("Expected text content");
        }
    }
}
