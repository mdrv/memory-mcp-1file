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
use crate::types::{Direction, Entity, Memory, MemoryType, MemoryUpdate, Relation, ScoredMemory};

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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateEntityParams {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateRelationParams {
    pub from_entity: String,
    pub to_entity: String,
    pub relation_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetRelatedParams {
    pub entity_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetValidParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetValidAtParams {
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InvalidateParams {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetStatusParams {
    #[serde(skip)]
    _placeholder: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IndexProjectParams {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchCodeParams {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetIndexStatusParams {
    pub project_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListProjectsParams {
    #[serde(skip)]
    _placeholder: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeleteProjectParams {
    pub project_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResetAllMemoryParams {
    pub confirm: bool,
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
        use crate::graph::{apply_hub_dampening, personalized_page_rank, PPR_DAMPING, PPR_MAX_ITER, PPR_TOLERANCE};
        use petgraph::graph::{DiGraph, NodeIndex};
        use std::collections::HashMap;

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

        let all_ids: Vec<String> = vector_results
            .iter()
            .chain(bm25_results.iter())
            .map(|r| r.id.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let ppr_tuples: Vec<(String, f32)> = if !all_ids.is_empty() {
            match self.state.storage.get_subgraph(&all_ids).await {
                Ok((entities, relations)) if !entities.is_empty() => {
                    let mut graph: DiGraph<String, f32> = DiGraph::new();
                    let mut node_map: HashMap<String, NodeIndex> = HashMap::new();

                    for entity in &entities {
                        if let Some(ref id) = entity.id {
                            let id_str = id.id.to_string();
                            let idx = graph.add_node(id_str.clone());
                            node_map.insert(id_str, idx);
                        }
                    }

                    for relation in &relations {
                        let from_str = relation.from_entity.id.to_string();
                        let to_str = relation.to_entity.id.to_string();
                        if let (Some(&from_idx), Some(&to_idx)) =
                            (node_map.get(&from_str), node_map.get(&to_str))
                        {
                            graph.add_edge(from_idx, to_idx, relation.weight);
                        }
                    }

                    let seed_nodes: Vec<NodeIndex> = all_ids
                        .iter()
                        .take(20)
                        .filter_map(|id| node_map.get(id).copied())
                        .collect();

                    let mut ppr_scores =
                        personalized_page_rank(&graph, &seed_nodes, PPR_DAMPING, PPR_TOLERANCE, PPR_MAX_ITER);

                    let degrees: HashMap<NodeIndex, usize> = graph
                        .node_indices()
                        .map(|idx| (idx, graph.edges(idx).count()))
                        .collect();
                    apply_hub_dampening(&mut ppr_scores, &degrees);

                    let reverse_map: HashMap<NodeIndex, String> = node_map
                        .iter()
                        .map(|(id, idx)| (*idx, id.clone()))
                        .collect();

                    let mut tuples: Vec<_> = ppr_scores
                        .into_iter()
                        .filter_map(|(idx, score)| {
                            reverse_map.get(&idx).map(|id| (id.clone(), score))
                        })
                        .collect();
                    tuples.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
                    tuples
                }
                _ => vec![],
            }
        } else {
            vec![]
        };

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

    #[tool(description = "Create a knowledge graph entity. Returns the entity ID.")]
    async fn create_entity(
        &self,
        params: Parameters<CreateEntityParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let entity = Entity {
            id: None,
            name: params.0.name.clone(),
            entity_type: params.0.entity_type.unwrap_or_else(|| "unknown".to_string()),
            description: params.0.description.clone(),
            embedding: None,
            user_id: params.0.user_id.clone(),
            created_at: chrono::Utc::now(),
        };

        match self.state.storage.create_entity(entity).await {
            Ok(id) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "id": id }).to_string(),
            )])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": e.to_string() }).to_string(),
            )])),
        }
    }

    #[tool(description = "Create a relation between two entities.")]
    async fn create_relation(
        &self,
        params: Parameters<CreateRelationParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let relation = Relation {
            id: None,
            from_entity: surrealdb::sql::Thing::from(("entities".to_string(), params.0.from_entity.clone())),
            to_entity: surrealdb::sql::Thing::from(("entities".to_string(), params.0.to_entity.clone())),
            relation_type: params.0.relation_type.clone(),
            weight: params.0.weight.unwrap_or(1.0).clamp(0.0, 1.0),
            valid_from: chrono::Utc::now(),
            valid_until: None,
        };

        match self.state.storage.create_relation(relation).await {
            Ok(id) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "id": id }).to_string(),
            )])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": e.to_string() }).to_string(),
            )])),
        }
    }

    #[tool(description = "Get entities related to a given entity via graph traversal.")]
    async fn get_related(
        &self,
        params: Parameters<GetRelatedParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let depth = params.0.depth.unwrap_or(1).min(3);
        let direction: Direction = params
            .0
            .direction
            .as_ref()
            .and_then(|s| s.parse().ok())
            .unwrap_or_default();

        match self
            .state
            .storage
            .get_related(&params.0.entity_id, depth, direction)
            .await
        {
            Ok((entities, relations)) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "entities": entities,
                    "relations": relations,
                    "entity_count": entities.len(),
                    "relation_count": relations.len()
                })
                .to_string(),
            )])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": e.to_string() }).to_string(),
            )])),
        }
    }

    #[tool(description = "Get all currently valid memories. Returns memories where valid_until is not set or is in the future.")]
    async fn get_valid(
        &self,
        params: Parameters<GetValidParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = params.0.limit.unwrap_or(20).min(100);

        match self
            .state
            .storage
            .get_valid(params.0.user_id.as_deref(), limit)
            .await
        {
            Ok(memories) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "memories": memories,
                    "count": memories.len()
                })
                .to_string(),
            )])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": e.to_string() }).to_string(),
            )])),
        }
    }

    #[tool(description = "Get memories that were valid at a specific point in time. Timestamp in ISO 8601 format.")]
    async fn get_valid_at(
        &self,
        params: Parameters<GetValidAtParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = params.0.limit.unwrap_or(20).min(100);

        let ts: chrono::DateTime<chrono::Utc> = match params.0.timestamp.parse() {
            Ok(t) => t,
            Err(_) => {
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::json!({ "error": "Invalid timestamp format. Use ISO 8601 (e.g., 2024-01-15T10:30:00Z)" }).to_string(),
                )]));
            }
        };

        match self
            .state
            .storage
            .get_valid_at(ts, params.0.user_id.as_deref(), limit)
            .await
        {
            Ok(memories) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "memories": memories,
                    "count": memories.len(),
                    "timestamp": params.0.timestamp
                })
                .to_string(),
            )])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": e.to_string() }).to_string(),
            )])),
        }
    }

    #[tool(description = "Invalidate (soft-delete) a memory. Sets valid_until to now and optionally links to replacement.")]
    async fn invalidate(
        &self,
        params: Parameters<InvalidateParams>,
    ) -> Result<CallToolResult, ErrorData> {
        match self
            .state
            .storage
            .invalidate(
                &params.0.id,
                params.0.reason.as_deref(),
                params.0.superseded_by.as_deref(),
            )
            .await
        {
            Ok(success) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "invalidated": success }).to_string(),
            )])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": e.to_string() }).to_string(),
            )])),
        }
    }

    #[tool(description = "Get server status and statistics. Returns version, memory count, and health info.")]
    async fn get_status(
        &self,
        _params: Parameters<GetStatusParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let memories_count = self.state.storage.count_memories().await.unwrap_or(0);
        let db_healthy = self.state.storage.health_check().await.unwrap_or(false);
        let embedding_status = self.state.embedding.status();

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::json!({
                "version": env!("CARGO_PKG_VERSION"),
                "status": if db_healthy { "healthy" } else { "degraded" },
                "memories_count": memories_count,
                "embedding": {
                    "status": format!("{:?}", embedding_status),
                    "model": "e5_multi_768d",
                    "dimensions": 768
                }
            })
            .to_string(),
        )]))
    }

    #[tool(description = "Index a project directory for code search. Returns indexing status.")]
    async fn index_project(
        &self,
        params: Parameters<IndexProjectParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = std::path::Path::new(&params.0.path);

        if !path.exists() {
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": format!("Path does not exist: {}", params.0.path) })
                    .to_string(),
            )]));
        }

        match crate::codebase::index_project(self.state.clone(), path).await {
            Ok(status) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "project_id": status.project_id,
                    "status": status.status.to_string(),
                    "total_files": status.total_files,
                    "indexed_files": status.indexed_files,
                    "total_chunks": status.total_chunks
                })
                .to_string(),
            )])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": e.to_string() }).to_string(),
            )])),
        }
    }

    #[tool(description = "Search indexed code using semantic similarity. Returns matching code chunks.")]
    async fn search_code(
        &self,
        params: Parameters<SearchCodeParams>,
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
        match self
            .state
            .storage
            .vector_search_code(&query_embedding, params.0.project_id.as_deref(), limit)
            .await
        {
            Ok(results) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "results": results,
                    "count": results.len(),
                    "query": params.0.query
                })
                .to_string(),
            )])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": e.to_string() }).to_string(),
            )])),
        }
    }

    #[tool(description = "Get indexing status for a project.")]
    async fn get_index_status(
        &self,
        params: Parameters<GetIndexStatusParams>,
    ) -> Result<CallToolResult, ErrorData> {
        match self
            .state
            .storage
            .get_index_status(&params.0.project_id)
            .await
        {
            Ok(Some(status)) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string(&status).unwrap_or_default(),
            )])),
            Ok(None) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": format!("Project not found: {}", params.0.project_id) })
                    .to_string(),
            )])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": e.to_string() }).to_string(),
            )])),
        }
    }

    #[tool(description = "List all indexed projects.")]
    async fn list_projects(
        &self,
        _params: Parameters<ListProjectsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        match self.state.storage.list_projects().await {
            Ok(projects) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "projects": projects,
                    "count": projects.len()
                })
                .to_string(),
            )])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": e.to_string() }).to_string(),
            )])),
        }
    }

    #[tool(description = "Delete a project and all its indexed code chunks.")]
    async fn delete_project(
        &self,
        params: Parameters<DeleteProjectParams>,
    ) -> Result<CallToolResult, ErrorData> {
        match self
            .state
            .storage
            .delete_project_chunks(&params.0.project_id)
            .await
        {
            Ok(deleted) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "deleted_chunks": deleted,
                    "project_id": params.0.project_id
                })
                .to_string(),
            )])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": e.to_string() }).to_string(),
            )])),
        }
    }

    #[tool(description = "Reset all memory data. Requires confirm=true. DANGER: Deletes all memories, entities, relations, and code chunks.")]
    async fn reset_all_memory(
        &self,
        params: Parameters<ResetAllMemoryParams>,
    ) -> Result<CallToolResult, ErrorData> {
        if !params.0.confirm {
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({ "error": "Must set confirm=true to reset all data" }).to_string(),
            )]));
        }

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::json!({
                "reset": true,
                "warning": "All data has been cleared"
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
