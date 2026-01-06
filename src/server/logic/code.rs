use std::sync::Arc;

use rmcp::model::{CallToolResult, Content};
use serde_json::json;

use crate::config::AppState;
use crate::embedding::EmbeddingStatus;
use crate::server::params::{
    DeleteProjectParams, GetIndexStatusParams, IndexProjectParams, ListProjectsParams,
    SearchCodeParams,
};
use crate::storage::StorageBackend;

pub async fn index_project(
    state: &Arc<AppState>,
    params: IndexProjectParams,
) -> anyhow::Result<CallToolResult> {
    let path = std::path::Path::new(&params.path);

    if !path.exists() {
        return Ok(CallToolResult::success(vec![Content::text(
            json!({ "error": format!("Path does not exist: {}", params.path) }).to_string(),
        )]));
    }

    match crate::codebase::index_project(state.clone(), path).await {
        Ok(status) => Ok(CallToolResult::success(vec![Content::text(
            json!({
                "project_id": status.project_id,
                "status": status.status.to_string(),
                "total_files": status.total_files,
                "indexed_files": status.indexed_files,
                "total_chunks": status.total_chunks
            })
            .to_string(),
        )])),
        Err(e) => Ok(CallToolResult::success(vec![Content::text(
            json!({ "error": e.to_string() }).to_string(),
        )])),
    }
}

pub async fn search_code(
    state: &Arc<AppState>,
    params: SearchCodeParams,
) -> anyhow::Result<CallToolResult> {
    if state.embedding.status() != EmbeddingStatus::Ready {
        return Ok(CallToolResult::success(vec![Content::text(
            json!({ "error": "Embedding service not ready" }).to_string(),
        )]));
    }

    let query_embedding = state.embedding.embed(&params.query).await?;

    let limit = params.limit.unwrap_or(10).min(50);
    match state
        .storage
        .vector_search_code(&query_embedding, params.project_id.as_deref(), limit)
        .await
    {
        Ok(results) => Ok(CallToolResult::success(vec![Content::text(
            json!({
                "results": results,
                "count": results.len(),
                "query": params.query
            })
            .to_string(),
        )])),
        Err(e) => Ok(CallToolResult::success(vec![Content::text(
            json!({ "error": e.to_string() }).to_string(),
        )])),
    }
}

pub async fn get_index_status(
    state: &Arc<AppState>,
    params: GetIndexStatusParams,
) -> anyhow::Result<CallToolResult> {
    match state.storage.get_index_status(&params.project_id).await {
        Ok(Some(status)) => Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&status).unwrap_or_default(),
        )])),
        Ok(None) => Ok(CallToolResult::success(vec![Content::text(
            json!({ "error": format!("Project not found: {}", params.project_id) }).to_string(),
        )])),
        Err(e) => Ok(CallToolResult::success(vec![Content::text(
            json!({ "error": e.to_string() }).to_string(),
        )])),
    }
}

pub async fn list_projects(
    state: &Arc<AppState>,
    _params: ListProjectsParams,
) -> anyhow::Result<CallToolResult> {
    match state.storage.list_projects().await {
        Ok(projects) => Ok(CallToolResult::success(vec![Content::text(
            json!({
                "projects": projects,
                "count": projects.len()
            })
            .to_string(),
        )])),
        Err(e) => Ok(CallToolResult::success(vec![Content::text(
            json!({ "error": e.to_string() }).to_string(),
        )])),
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
        Ok(deleted) => Ok(CallToolResult::success(vec![Content::text(
            json!({
                "deleted_chunks": deleted,
                "project_id": params.project_id
            })
            .to_string(),
        )])),
        Err(e) => Ok(CallToolResult::success(vec![Content::text(
            json!({ "error": e.to_string() }).to_string(),
        )])),
    }
}
