use std::sync::Arc;

use rmcp::model::{CallToolResult, Content};
use serde_json::json;

use crate::config::AppState;
use crate::embedding::EmbeddingStatus;
use crate::server::params::{
    DeleteMemoryParams, GetMemoryParams, GetValidAtParams, GetValidParams, InvalidateParams,
    ListMemoriesParams, StoreMemoryParams, UpdateMemoryParams,
};
use crate::storage::StorageBackend;
use crate::types::{Memory, MemoryType, MemoryUpdate};

pub async fn store_memory(
    state: &Arc<AppState>,
    params: StoreMemoryParams,
) -> anyhow::Result<CallToolResult> {
    if state.embedding.status() != EmbeddingStatus::Ready {
        return Ok(CallToolResult::success(vec![Content::text(
            json!({ "error": "Embedding service not ready. Please try again." }).to_string(),
        )]));
    }

    let embedding = state.embedding.embed(&params.content).await?;

    let mem_type: MemoryType = params
        .memory_type
        .as_ref()
        .and_then(|s| s.parse().ok())
        .unwrap_or_default();

    let now = surrealdb::sql::Datetime::default();
    let memory = Memory {
        id: None,
        content: params.content,
        embedding: Some(embedding),
        memory_type: mem_type,
        user_id: params.user_id,
        metadata: params.metadata,
        event_time: now.clone(),
        ingestion_time: now.clone(),
        valid_from: now,
        valid_until: None,
        importance_score: 1.0,
        invalidation_reason: None,
    };

    match state.storage.create_memory(memory).await {
        Ok(id) => Ok(CallToolResult::success(vec![Content::text(
            json!({ "id": id }).to_string(),
        )])),
        Err(e) => Ok(CallToolResult::success(vec![Content::text(
            json!({ "error": e.to_string() }).to_string(),
        )])),
    }
}

pub async fn get_memory(
    state: &Arc<AppState>,
    params: GetMemoryParams,
) -> anyhow::Result<CallToolResult> {
    match state.storage.get_memory(&params.id).await {
        Ok(Some(memory)) => Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&memory).unwrap_or_default(),
        )])),
        Ok(None) => Ok(CallToolResult::success(vec![Content::text(
            json!({ "error": format!("Memory not found: {}", params.id) }).to_string(),
        )])),
        Err(e) => Ok(CallToolResult::success(vec![Content::text(
            json!({ "error": e.to_string() }).to_string(),
        )])),
    }
}

pub async fn update_memory(
    state: &Arc<AppState>,
    params: UpdateMemoryParams,
) -> anyhow::Result<CallToolResult> {
    let update = MemoryUpdate {
        content: params.content,
        memory_type: params.memory_type.and_then(|s| s.parse().ok()),
        metadata: params.metadata,
    };

    match state.storage.update_memory(&params.id, update).await {
        Ok(memory) => Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&memory).unwrap_or_default(),
        )])),
        Err(e) => Ok(CallToolResult::success(vec![Content::text(
            json!({ "error": e.to_string() }).to_string(),
        )])),
    }
}

pub async fn delete_memory(
    state: &Arc<AppState>,
    params: DeleteMemoryParams,
) -> anyhow::Result<CallToolResult> {
    match state.storage.delete_memory(&params.id).await {
        Ok(deleted) => Ok(CallToolResult::success(vec![Content::text(
            json!({ "deleted": deleted }).to_string(),
        )])),
        Err(e) => Ok(CallToolResult::success(vec![Content::text(
            json!({ "error": e.to_string() }).to_string(),
        )])),
    }
}

pub async fn list_memories(
    state: &Arc<AppState>,
    params: ListMemoriesParams,
) -> anyhow::Result<CallToolResult> {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);

    let memories = match state.storage.list_memories(limit, offset).await {
        Ok(m) => m,
        Err(e) => {
            return Ok(CallToolResult::success(vec![Content::text(
                json!({ "error": e.to_string() }).to_string(),
            )]));
        }
    };

    let total = state.storage.count_memories().await.unwrap_or(0);

    Ok(CallToolResult::success(vec![Content::text(
        json!({
            "memories": memories,
            "total": total,
            "limit": limit,
            "offset": offset
        })
        .to_string(),
    )]))
}

pub async fn get_valid(
    state: &Arc<AppState>,
    params: GetValidParams,
) -> anyhow::Result<CallToolResult> {
    let limit = params.limit.unwrap_or(20).min(100);

    match state
        .storage
        .get_valid(params.user_id.as_deref(), limit)
        .await
    {
        Ok(memories) => Ok(CallToolResult::success(vec![Content::text(
            json!({
                "memories": memories,
                "count": memories.len()
            })
            .to_string(),
        )])),
        Err(e) => Ok(CallToolResult::success(vec![Content::text(
            json!({ "error": e.to_string() }).to_string(),
        )])),
    }
}

pub async fn get_valid_at(
    state: &Arc<AppState>,
    params: GetValidAtParams,
) -> anyhow::Result<CallToolResult> {
    let limit = params.limit.unwrap_or(20).min(100);

    let ts: chrono::DateTime<chrono::Utc> = match params.timestamp.parse() {
        Ok(t) => t,
        Err(_) => {
            return Ok(CallToolResult::success(vec![Content::text(
                json!({ "error": "Invalid timestamp format. Use ISO 8601" }).to_string(),
            )]));
        }
    };

    match state
        .storage
        .get_valid_at(ts, params.user_id.as_deref(), limit)
        .await
    {
        Ok(memories) => Ok(CallToolResult::success(vec![Content::text(
            json!({
                "memories": memories,
                "count": memories.len(),
                "timestamp": params.timestamp
            })
            .to_string(),
        )])),
        Err(e) => Ok(CallToolResult::success(vec![Content::text(
            json!({ "error": e.to_string() }).to_string(),
        )])),
    }
}

pub async fn invalidate(
    state: &Arc<AppState>,
    params: InvalidateParams,
) -> anyhow::Result<CallToolResult> {
    match state
        .storage
        .invalidate(
            &params.id,
            params.reason.as_deref(),
            params.superseded_by.as_deref(),
        )
        .await
    {
        Ok(success) => Ok(CallToolResult::success(vec![Content::text(
            json!({ "invalidated": success }).to_string(),
        )])),
        Err(e) => Ok(CallToolResult::success(vec![Content::text(
            json!({ "error": e.to_string() }).to_string(),
        )])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestContext;

    #[tokio::test]
    async fn test_memory_crud_logic() {
        let ctx = TestContext::new().await;

        // 1. Store
        let params = StoreMemoryParams {
            content: "Logic test memory".to_string(),
            memory_type: Some("semantic".to_string()),
            user_id: Some("user1".to_string()),
            metadata: None,
        };
        let result = store_memory(&ctx.state, params).await.unwrap();
        let val = serde_json::to_value(&result).unwrap();
        let text = val["content"][0]["text"].as_str().unwrap();
        let json: serde_json::Value = serde_json::from_str(text).unwrap();
        let id = json["id"].as_str().unwrap().to_string();

        // 2. Get
        let get_params = GetMemoryParams { id: id.clone() };
        let result = get_memory(&ctx.state, get_params).await.unwrap();
        let val = serde_json::to_value(&result).unwrap();
        let text = val["content"][0]["text"].as_str().unwrap();
        let memory_json: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(memory_json["content"], "Logic test memory");

        // 3. List
        let list_params = ListMemoriesParams {
            limit: Some(10),
            offset: None,
        };
        let result = list_memories(&ctx.state, list_params).await.unwrap();
        let val = serde_json::to_value(&result).unwrap();
        let text = val["content"][0]["text"].as_str().unwrap();
        let list_json: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(list_json["memories"].as_array().unwrap().len(), 1);
    }
}
