pub mod code;
pub mod graph;
pub mod memory;
pub mod search;
pub mod system;

use rmcp::model::{CallToolResult, Content};
use serde_json::json;

use crate::embedding::EmbeddingStatus;
use crate::types::{Entity, Memory};

pub fn strip_embedding(memory: &mut Memory) {
    memory.embedding.take();
}

pub fn strip_embeddings(memories: &mut [Memory]) {
    for m in memories {
        m.embedding.take();
    }
}

pub fn strip_entity_embeddings(entities: &mut [Entity]) {
    for e in entities {
        e.embedding.take();
    }
}

pub fn embedding_loading_response(status: &EmbeddingStatus) -> CallToolResult {
    match status {
        EmbeddingStatus::Loading {
            phase,
            elapsed_seconds,
            eta_seconds,
            cached,
            progress_percent,
            downloaded_mb,
            total_mb,
        } => {
            let mut response = json!({
                "status": "loading",
                "message": format!("Model loading: {}", phase),
                "phase": phase,
                "elapsed_seconds": elapsed_seconds,
                "eta_seconds": eta_seconds,
                "cached": cached,
                "retry_after_seconds": eta_seconds.unwrap_or(5).min(10)
            });

            if let Some(pct) = progress_percent {
                response["progress_percent"] = json!(pct);
            }
            if let (Some(dl), Some(total)) = (downloaded_mb, total_mb) {
                response["downloaded_mb"] = json!(dl);
                response["total_mb"] = json!(total);
            }

            CallToolResult::success(vec![Content::text(response.to_string())])
        }
        EmbeddingStatus::Error { message } => CallToolResult::success(vec![Content::text(
            json!({
                "status": "error",
                "error": message
            })
            .to_string(),
        )]),
        EmbeddingStatus::Ready => {
            CallToolResult::success(vec![Content::text(json!({"status": "ready"}).to_string())])
        }
    }
}
