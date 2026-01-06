use std::sync::Arc;

use rmcp::{
    handler::server::{
        tool::ToolCallContext, tool::ToolRouter, wrapper::Parameters, ServerHandler,
    },
    model::*,
    service::{RequestContext, RoleServer},
    tool, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::AppState;
use crate::embedding::EmbeddingStatus;
use crate::graph::{rrf_merge, DEFAULT_BM25_WEIGHT, DEFAULT_PPR_WEIGHT, DEFAULT_VECTOR_WEIGHT};
use crate::storage::StorageBackend;
use crate::types::{Memory, MemoryType, MemoryUpdate, ScoredMemory};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StoreMemoryParams {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetMemoryParams {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpdateMemoryParams {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeleteMemoryParams {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListMemoriesParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchParams {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecallParams {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_weight: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bm25_weight: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ppr_weight: Option<f32>,
}

#[derive(Clone)]
pub struct MemoryMcpServer {
    state: Arc<AppState>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl MemoryMcpServer {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Store a new memory. Returns the memory ID.")]
    async fn store_memory(
        &self,
        params: Parameters<StoreMemoryParams>,
    ) -> Result<CallToolResult, ErrorData> {
        if self.state.embedding.status() != EmbeddingStatus::Ready {
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": "Embedding service not ready. Please try again." })
                    .to_string(),
            )]));
        }

        let embedding = match self.state.embedding.embed(&params.0.content).await {
            Ok(e) => e,
            Err(e) => {
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::json!({ "error": e.to_string() }).to_string(),
                )]));
            }
        };

        let mem_type: MemoryType = params
            .0
            .memory_type
            .as_ref()
            .and_then(|s: &String| s.parse().ok())
            .unwrap_or_default();

        let memory = Memory {
            id: None,
            content: params.0.content,
            embedding: Some(embedding),
            memory_type: mem_type,
            user_id: params.0.user_id,
            metadata: params.0.metadata,
            event_time: chrono::Utc::now(),
            ingestion_time: chrono::Utc::now(),
            valid_from: chrono::Utc::now(),
            valid_until: None,
            importance_score: 1.0,
            invalidation_reason: None,
        };

        match self.state.storage.create_memory(memory).await {
            Ok(id) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "id": id }).to_string(),
            )])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": e.to_string() }).to_string(),
            )])),
        }
    }

    #[tool(
        description = "Get a memory by its ID. Returns the full memory object or an error if not found."
    )]
    async fn get_memory(
        &self,
        params: Parameters<GetMemoryParams>,
    ) -> Result<CallToolResult, ErrorData> {
        match self.state.storage.get_memory(&params.0.id).await {
            Ok(Some(memory)) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string(&memory).unwrap_or_default(),
            )])),
            Ok(None) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": format!("Memory not found: {}", params.0.id) })
                    .to_string(),
            )])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": e.to_string() }).to_string(),
            )])),
        }
    }

    #[tool(description = "Update an existing memory. Only provided fields will be updated.")]
    async fn update_memory(
        &self,
        params: Parameters<UpdateMemoryParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let update = MemoryUpdate {
            content: params.0.content,
            memory_type: params.0.memory_type.and_then(|s: String| s.parse().ok()),
            metadata: params.0.metadata,
        };

        match self.state.storage.update_memory(&params.0.id, update).await {
            Ok(memory) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string(&memory).unwrap_or_default(),
            )])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": e.to_string() }).to_string(),
            )])),
        }
    }

    #[tool(description = "Delete a memory by its ID. Returns true if deleted, false if not found.")]
    async fn delete_memory(
        &self,
        params: Parameters<DeleteMemoryParams>,
    ) -> Result<CallToolResult, ErrorData> {
        match self.state.storage.delete_memory(&params.0.id).await {
            Ok(deleted) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "deleted": deleted }).to_string(),
            )])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": e.to_string() }).to_string(),
            )])),
        }
    }

    #[tool(
        description = "List memories with pagination. Returns array of memories sorted by newest first."
    )]
    async fn list_memories(
        &self,
        params: Parameters<ListMemoriesParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = params.0.limit.unwrap_or(20).min(100);
        let offset = params.0.offset.unwrap_or(0);

        let memories = match self.state.storage.list_memories(limit, offset).await {
            Ok(m) => m,
            Err(e) => {
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::json!({ "error": e.to_string() }).to_string(),
                )]));
            }
        };

        let total = self.state.storage.count_memories().await.unwrap_or(0);

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::json!({
                "memories": memories,
                "total": total,
                "limit": limit,
                "offset": offset
            })
            .to_string(),
        )]))
    }

    #[tool(description = "Semantic search over memories. Returns memories most similar to the query, ordered by relevance.")]
    async fn search(
        &self,
        params: Parameters<SearchParams>,
    ) -> Result<CallToolResult, ErrorData> {
        if self.state.embedding.status() != EmbeddingStatus::Ready {
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": "Embedding service not ready" }).to_string(),
            )]));
        }

        let query_embedding = match self.state.embedding.embed(&params.0.query).await {
            Ok(e) => e,
            Err(e) => {
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::json!({ "error": e.to_string() }).to_string(),
                )]));
            }
        };

        let limit = params.0.limit.unwrap_or(10).min(50);
        let results = match self.state.storage.vector_search(&query_embedding, limit).await {
            Ok(r) => r,
            Err(e) => {
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::json!({ "error": e.to_string() }).to_string(),
                )]));
            }
        };

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::json!({
                "results": results,
                "count": results.len(),
                "query": params.0.query
            })
            .to_string(),
        )]))
    }

    #[tool(description = "Full-text keyword search over memories using BM25. Best for exact keyword matching.")]
    async fn search_text(
        &self,
        params: Parameters<SearchParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = params.0.limit.unwrap_or(10).min(50);
        let results = match self.state.storage.bm25_search(&params.0.query, limit).await {
            Ok(r) => r,
            Err(e) => {
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::json!({ "error": e.to_string() }).to_string(),
                )]));
            }
        };

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::json!({
                "results": results,
                "count": results.len(),
                "query": params.0.query
            })
            .to_string(),
        )]))
    }

    #[tool(description = "Hybrid search combining vector similarity, BM25 keywords, and graph context. Best quality retrieval.")]
    async fn recall(
        &self,
        params: Parameters<RecallParams>,
    ) -> Result<CallToolResult, ErrorData> {
        if self.state.embedding.status() != EmbeddingStatus::Ready {
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": "Embedding service not ready" }).to_string(),
            )]));
        }

        let query_embedding = match self.state.embedding.embed(&params.0.query).await {
            Ok(e) => e,
            Err(e) => {
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::json!({ "error": e.to_string() }).to_string(),
                )]));
            }
        };

        let limit = params.0.limit.unwrap_or(20).min(100);
        let fetch_limit = limit * 3;

        let vector_weight = params.0.vector_weight.unwrap_or(DEFAULT_VECTOR_WEIGHT);
        let bm25_weight = params.0.bm25_weight.unwrap_or(DEFAULT_BM25_WEIGHT);
        let ppr_weight = params.0.ppr_weight.unwrap_or(DEFAULT_PPR_WEIGHT);

        let vector_results = self
            .state
            .storage
            .vector_search(&query_embedding, fetch_limit)
            .await
            .unwrap_or_default();

        let bm25_results = self
            .state
            .storage
            .bm25_search(&params.0.query, fetch_limit)
            .await
            .unwrap_or_default();

        let vector_tuples: Vec<_> = vector_results
            .iter()
            .map(|r| (r.id.clone(), r.score))
            .collect();
        let bm25_tuples: Vec<_> = bm25_results
            .iter()
            .map(|r| (r.id.clone(), r.score))
            .collect();
        let ppr_tuples: Vec<(String, f32)> = vec![];

        let merged = rrf_merge(
            &vector_tuples,
            &bm25_tuples,
            &ppr_tuples,
            vector_weight,
            bm25_weight,
            ppr_weight,
            limit,
        );

        let mut content_map: std::collections::HashMap<String, (&str, MemoryType)> =
            std::collections::HashMap::new();
        for r in &vector_results {
            content_map.insert(r.id.clone(), (&r.content, r.memory_type.clone()));
        }
        for r in &bm25_results {
            content_map
                .entry(r.id.clone())
                .or_insert((&r.content, r.memory_type.clone()));
        }

        let scored_memories: Vec<ScoredMemory> = merged
            .into_iter()
            .filter_map(|(id, scores)| {
                content_map.get(&id).map(|(content, mem_type)| ScoredMemory {
                    id: id.clone(),
                    content: content.to_string(),
                    memory_type: mem_type.clone(),
                    score: scores.combined_score,
                    vector_score: scores.vector_score,
                    bm25_score: scores.bm25_score,
                    ppr_score: scores.ppr_score,
                })
            })
            .collect();

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::json!({
                "memories": scored_memories,
                "count": scored_memories.len(),
                "query": params.0.query,
                "weights": {
                    "vector": vector_weight,
                    "bm25": bm25_weight,
                    "ppr": ppr_weight
                }
            })
            .to_string(),
        )]))
    }
}

impl ServerHandler for MemoryMcpServer {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                ..ServerCapabilities::default()
            },
            server_info: Implementation {
                name: "memory-mcp".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                title: None,
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "AI agent memory server with semantic search, knowledge graph, and code search."
                    .into(),
            ),
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(self.tool_router.list_all()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let tool_context = ToolCallContext::new(self, request, context);
        self.tool_router.call(tool_context).await
    }
}
