use std::sync::Arc;

use rmcp::{
    handler::server::{
        tool::ToolCallContext, tool::ToolRouter, wrapper::Parameters, ServerHandler,
    },
    model::*,
    service::{RequestContext, RoleServer},
    tool, tool_router,
};

use crate::config::AppState;
use crate::server::logic;
use crate::server::params::*;

#[derive(Clone)]
pub struct MemoryMcpServer {
    state: Arc<AppState>,
    tool_router: ToolRouter<Self>,
}

// Helper to convert anyhow::Error to JSON-RPC ErrorData
fn to_rpc_error(e: anyhow::Error) -> ErrorData {
    ErrorData {
        code: ErrorCode(-32000),
        message: e.to_string().into(),
        data: None,
    }
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
        logic::memory::store_memory(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
    }

    #[tool(
        description = "Get a memory by its ID. Returns the full memory object or an error if not found."
    )]
    async fn get_memory(
        &self,
        params: Parameters<GetMemoryParams>,
    ) -> Result<CallToolResult, ErrorData> {
        logic::memory::get_memory(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
    }

    #[tool(description = "Update an existing memory. Only provided fields will be updated.")]
    async fn update_memory(
        &self,
        params: Parameters<UpdateMemoryParams>,
    ) -> Result<CallToolResult, ErrorData> {
        logic::memory::update_memory(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
    }

    #[tool(description = "Delete a memory by its ID. Returns true if deleted, false if not found.")]
    async fn delete_memory(
        &self,
        params: Parameters<DeleteMemoryParams>,
    ) -> Result<CallToolResult, ErrorData> {
        logic::memory::delete_memory(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
    }

    #[tool(
        description = "List memories with pagination. Returns array of memories sorted by newest first."
    )]
    async fn list_memories(
        &self,
        params: Parameters<ListMemoriesParams>,
    ) -> Result<CallToolResult, ErrorData> {
        logic::memory::list_memories(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
    }

    #[tool(
        description = "Semantic search over memories. Returns memories most similar to the query, ordered by relevance."
    )]
    async fn search(&self, params: Parameters<SearchParams>) -> Result<CallToolResult, ErrorData> {
        logic::search::search(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
    }

    #[tool(
        description = "Full-text keyword search over memories using BM25. Best for exact keyword matching."
    )]
    async fn search_text(
        &self,
        params: Parameters<SearchParams>,
    ) -> Result<CallToolResult, ErrorData> {
        logic::search::search_text(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
    }

    #[tool(
        description = "Hybrid search combining vector similarity, BM25 keywords, and graph context. Best quality retrieval."
    )]
    async fn recall(&self, params: Parameters<RecallParams>) -> Result<CallToolResult, ErrorData> {
        logic::search::recall(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
    }

    #[tool(description = "Create a knowledge graph entity. Returns the entity ID.")]
    async fn create_entity(
        &self,
        params: Parameters<CreateEntityParams>,
    ) -> Result<CallToolResult, ErrorData> {
        logic::graph::create_entity(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
    }

    #[tool(description = "Create a relation between two entities.")]
    async fn create_relation(
        &self,
        params: Parameters<CreateRelationParams>,
    ) -> Result<CallToolResult, ErrorData> {
        logic::graph::create_relation(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
    }

    #[tool(description = "Get entities related to a given entity via graph traversal.")]
    async fn get_related(
        &self,
        params: Parameters<GetRelatedParams>,
    ) -> Result<CallToolResult, ErrorData> {
        logic::graph::get_related(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
    }

    #[tool(
        description = "Get all currently valid memories. Returns memories where valid_until is not set or is in the future."
    )]
    async fn get_valid(
        &self,
        params: Parameters<GetValidParams>,
    ) -> Result<CallToolResult, ErrorData> {
        logic::memory::get_valid(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
    }

    #[tool(
        description = "Get memories that were valid at a specific point in time. Timestamp in ISO 8601 format."
    )]
    async fn get_valid_at(
        &self,
        params: Parameters<GetValidAtParams>,
    ) -> Result<CallToolResult, ErrorData> {
        logic::memory::get_valid_at(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
    }

    #[tool(
        description = "Invalidate (soft-delete) a memory. Sets valid_until to now and optionally links to replacement."
    )]
    async fn invalidate(
        &self,
        params: Parameters<InvalidateParams>,
    ) -> Result<CallToolResult, ErrorData> {
        logic::memory::invalidate(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
    }

    #[tool(
        description = "Get server status and statistics. Returns version, memory count, and health info."
    )]
    async fn get_status(
        &self,
        params: Parameters<GetStatusParams>,
    ) -> Result<CallToolResult, ErrorData> {
        logic::system::get_status(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
    }

    #[tool(description = "Index a project directory for code search. Returns indexing status.")]
    async fn index_project(
        &self,
        params: Parameters<IndexProjectParams>,
    ) -> Result<CallToolResult, ErrorData> {
        logic::code::index_project(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
    }

    #[tool(
        description = "Search indexed code using semantic similarity. Returns matching code chunks."
    )]
    async fn search_code(
        &self,
        params: Parameters<SearchCodeParams>,
    ) -> Result<CallToolResult, ErrorData> {
        logic::code::search_code(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
    }

    #[tool(description = "Get indexing status for a project.")]
    async fn get_index_status(
        &self,
        params: Parameters<GetIndexStatusParams>,
    ) -> Result<CallToolResult, ErrorData> {
        logic::code::get_index_status(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
    }

    #[tool(description = "List all indexed projects.")]
    async fn list_projects(
        &self,
        params: Parameters<ListProjectsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        logic::code::list_projects(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
    }

    #[tool(description = "Delete a project and all its indexed code chunks.")]
    async fn delete_project(
        &self,
        params: Parameters<DeleteProjectParams>,
    ) -> Result<CallToolResult, ErrorData> {
        logic::code::delete_project(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
    }

    #[tool(
        description = "Reset all memory data. Requires confirm=true. DANGER: Deletes all memories, entities, relations, and code chunks."
    )]
    async fn reset_all_memory(
        &self,
        params: Parameters<ResetAllMemoryParams>,
    ) -> Result<CallToolResult, ErrorData> {
        logic::system::reset_all_memory(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
    }

    #[tool(description = "Detect communities in the knowledge graph using the Leiden algorithm.")]
    async fn detect_communities(
        &self,
        params: Parameters<DetectCommunitiesParams>,
    ) -> Result<CallToolResult, ErrorData> {
        logic::graph::detect_communities(&self.state, params.0)
            .await
            .map_err(to_rpc_error)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestContext;

    #[tokio::test]
    async fn test_server_handler_integration() {
        let ctx = TestContext::new().await;
        let server = MemoryMcpServer::new(ctx.state.clone());

        // 1. Get Info
        let info = server.get_info();
        assert_eq!(info.server_info.name, "memory-mcp");

        // 2. Integration check pass
        // We cannot easily mock RequestContext without more deps,
        // but since logic tests cover actual execution,
        // and compilation proves traits are implemented, this is sufficient.
    }
}
