pub mod code;
pub mod graph;
pub mod memory;
pub mod search;
pub mod system;

use rmcp::model::{CallToolResult, Content};
use serde_json::json;

use crate::embedding::EmbeddingStatus;
use crate::types::{CodeSymbol, Entity, Memory};

// ============================================================================
// Logic Constants & Helpers
// ============================================================================

pub const DEFAULT_LIMIT: usize = 20;
pub const MAX_LIMIT: usize = 100;

pub fn normalize_limit(limit: Option<usize>) -> usize {
    limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT)
}

// ============================================================================
// Response Helpers (deduplication)
// ============================================================================

/// Create error response from any Display type
pub fn error_response(e: impl std::fmt::Display) -> CallToolResult {
    CallToolResult::success(vec![Content::text(
        json!({ "error": e.to_string() }).to_string(),
    )])
}

/// Create success response from JSON value
pub fn success_json(value: serde_json::Value) -> CallToolResult {
    CallToolResult::success(vec![Content::text(value.to_string())])
}

/// Create success response from serializable value
pub fn success_serialize<T: serde::Serialize>(value: &T) -> CallToolResult {
    CallToolResult::success(vec![Content::text(
        serde_json::to_string(value).unwrap_or_default(),
    )])
}

// ============================================================================
// Embedding Helpers
// ============================================================================

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

pub fn strip_symbol_embeddings(symbols: &mut [CodeSymbol]) {
    for s in symbols.iter_mut() {
        s.embedding = None;
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

/// Macro to check embedding status and return early if not ready
#[macro_export]
macro_rules! ensure_embedding_ready {
    ($state:expr) => {
        let status = $state.embedding.status().await;
        if !status.is_ready() {
            return Ok($crate::server::logic::embedding_loading_response(&status));
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_limit() {
        assert_eq!(normalize_limit(None), DEFAULT_LIMIT);
        assert_eq!(normalize_limit(Some(10)), 10);
        assert_eq!(normalize_limit(Some(50)), 50);
        assert_eq!(normalize_limit(Some(100)), 100);
        assert_eq!(normalize_limit(Some(101)), MAX_LIMIT);
        assert_eq!(normalize_limit(Some(1000)), MAX_LIMIT);
    }
}
