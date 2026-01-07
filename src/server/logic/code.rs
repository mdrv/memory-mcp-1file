use std::sync::Arc;

use rmcp::model::CallToolResult;
use serde_json::json;

use crate::config::AppState;
use crate::server::params::{
    DeleteProjectParams, GetCalleesParams, GetCallersParams, GetIndexStatusParams,
    IndexProjectParams, ListProjectsParams, SearchCodeParams, SearchSymbolsParams,
};
use crate::storage::StorageBackend;

use super::{
    error_response, normalize_limit, strip_symbol_embeddings, success_json, success_serialize,
};

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
                tracing::info!(project_id = %project_id, "Re-indexing project (was completed)");
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
        Ok(Some(mut status)) => {
            if status.status == crate::types::IndexState::Indexing {
                let indexed = state
                    .monitor
                    .indexed_files
                    .load(std::sync::atomic::Ordering::Relaxed);
                let total = state
                    .monitor
                    .total_files
                    .load(std::sync::atomic::Ordering::Relaxed);

                if indexed > 0 {
                    status.indexed_files = std::cmp::max(status.indexed_files, indexed);
                }
                if total > 0 {
                    status.total_files = std::cmp::max(status.total_files, total);
                }
            }
            Ok(success_serialize(&status))
        }
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
    let _ = state
        .storage
        .delete_project_symbols(&params.project_id)
        .await;

    let _ = state.storage.delete_index_status(&params.project_id).await;

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

pub async fn search_symbols(
    state: &Arc<AppState>,
    params: SearchSymbolsParams,
) -> anyhow::Result<CallToolResult> {
    match state
        .storage
        .search_symbols(&params.query, params.project_id.as_deref())
        .await
    {
        Ok(mut symbols) => {
            let count_before = symbols.len();
            strip_symbol_embeddings(&mut symbols);

            if let Some(first) = symbols.first() {
                if first.embedding.is_some() {
                    tracing::error!("Failed to strip embedding for symbol {}", first.name);
                }
            }

            Ok(success_json(json!({
                "results": symbols,
                "count": count_before,
                "query": params.query
            })))
        }
        Err(e) => Ok(error_response(e)),
    }
}

pub async fn get_callers(
    state: &Arc<AppState>,
    params: GetCallersParams,
) -> anyhow::Result<CallToolResult> {
    match state.storage.get_symbol_callers(&params.symbol_id).await {
        Ok(mut callers) => {
            strip_symbol_embeddings(&mut callers);
            Ok(success_json(json!({
                "results": callers,
                "count": callers.len(),
                "symbol_id": params.symbol_id
            })))
        }
        Err(e) => Ok(error_response(e)),
    }
}

pub async fn get_callees(
    state: &Arc<AppState>,
    params: GetCalleesParams,
) -> anyhow::Result<CallToolResult> {
    match state.storage.get_symbol_callees(&params.symbol_id).await {
        Ok(mut callees) => {
            strip_symbol_embeddings(&mut callees);
            Ok(success_json(json!({
                "results": callees,
                "count": callees.len(),
                "symbol_id": params.symbol_id
            })))
        }
        Err(e) => Ok(error_response(e)),
    }
}

pub async fn get_related_symbols(
    state: &Arc<AppState>,
    params: crate::server::params::GetRelatedSymbolsParams,
) -> anyhow::Result<CallToolResult> {
    use crate::types::Direction;

    let depth = params.depth.unwrap_or(1).min(3);
    let direction: Direction = params
        .direction
        .as_ref()
        .and_then(|s| s.parse().ok())
        .unwrap_or_default();

    match state
        .storage
        .get_related_symbols(&params.symbol_id, depth, direction)
        .await
    {
        Ok((mut symbols, relations)) => {
            strip_symbol_embeddings(&mut symbols);
            Ok(success_json(json!({
                "symbols": symbols,
                "relations": relations,
                "symbol_count": symbols.len(),
                "relation_count": relations.len()
            })))
        }
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
