//! Storage backend trait definition
//!
//! Defines the async interface for all storage operations.
//! Implemented by SurrealStorage.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::types::{
    CodeChunk, Direction, Entity, IndexStatus, Memory, MemoryUpdate, Relation, ScoredCodeChunk,
    SearchResult,
};
use crate::Result;

/// Storage backend trait for all database operations
#[async_trait]
pub trait StorageBackend: Send + Sync {
    // ─────────────────────────────────────────────────────────────────────────
    // Memory CRUD
    // ─────────────────────────────────────────────────────────────────────────

    /// Store a new memory, returns the generated ID
    async fn create_memory(&self, memory: Memory) -> Result<String>;

    /// Get a memory by ID
    async fn get_memory(&self, id: &str) -> Result<Option<Memory>>;

    /// Update an existing memory
    async fn update_memory(&self, id: &str, update: MemoryUpdate) -> Result<Memory>;

    /// Delete a memory by ID, returns true if deleted
    async fn delete_memory(&self, id: &str) -> Result<bool>;

    /// List memories with pagination, sorted by ingestion_time DESC
    async fn list_memories(&self, limit: usize, offset: usize) -> Result<Vec<Memory>>;

    /// Count total number of memories
    async fn count_memories(&self) -> Result<usize>;

    // ─────────────────────────────────────────────────────────────────────────
    // Vector search
    // ─────────────────────────────────────────────────────────────────────────

    /// Vector similarity search on memories
    async fn vector_search(&self, embedding: &[f32], limit: usize) -> Result<Vec<SearchResult>>;

    /// Vector similarity search on code chunks
    async fn vector_search_code(
        &self,
        embedding: &[f32],
        project_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ScoredCodeChunk>>;

    // ─────────────────────────────────────────────────────────────────────────
    // BM25 search
    // ─────────────────────────────────────────────────────────────────────────

    /// Full-text BM25 search on memories
    async fn bm25_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>;

    /// Full-text BM25 search on code chunks
    async fn bm25_search_code(
        &self,
        query: &str,
        project_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ScoredCodeChunk>>;

    // ─────────────────────────────────────────────────────────────────────────
    // Entity operations
    // ─────────────────────────────────────────────────────────────────────────

    /// Create a new entity, returns the generated ID
    async fn create_entity(&self, entity: Entity) -> Result<String>;

    /// Get an entity by ID
    async fn get_entity(&self, id: &str) -> Result<Option<Entity>>;

    /// Search entities by name using BM25
    async fn search_entities(&self, query: &str, limit: usize) -> Result<Vec<Entity>>;

    // ─────────────────────────────────────────────────────────────────────────
    // Relation operations
    // ─────────────────────────────────────────────────────────────────────────

    /// Create a relation between two entities, returns the relation ID
    async fn create_relation(&self, relation: Relation) -> Result<String>;

    /// Get related entities via graph traversal
    async fn get_related(
        &self,
        entity_id: &str,
        depth: usize,
        direction: Direction,
    ) -> Result<(Vec<Entity>, Vec<Relation>)>;

    /// Get subgraph containing specified entities and their relations
    async fn get_subgraph(&self, entity_ids: &[String]) -> Result<(Vec<Entity>, Vec<Relation>)>;

    /// Get the degree (number of connections) for each entity
    async fn get_node_degrees(&self, entity_ids: &[String]) -> Result<HashMap<String, usize>>;

    // ─────────────────────────────────────────────────────────────────────────
    // Temporal operations
    // ─────────────────────────────────────────────────────────────────────────

    /// Get currently valid memories (valid_until is None or in the future)
    async fn get_valid(&self, user_id: Option<&str>, limit: usize) -> Result<Vec<Memory>>;

    /// Get memories that were valid at a specific point in time
    async fn get_valid_at(
        &self,
        timestamp: DateTime<Utc>,
        user_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Memory>>;

    /// Invalidate a memory (soft delete by setting valid_until)
    async fn invalidate(
        &self,
        id: &str,
        reason: Option<&str>,
        superseded_by: Option<&str>,
    ) -> Result<bool>;

    // ─────────────────────────────────────────────────────────────────────────
    // Code operations
    // ─────────────────────────────────────────────────────────────────────────

    /// Create a single code chunk, returns the generated ID
    async fn create_code_chunk(&self, chunk: CodeChunk) -> Result<String>;

    /// Create multiple code chunks in a batch, returns count of created chunks
    async fn create_code_chunks_batch(&self, chunks: Vec<CodeChunk>) -> Result<usize>;

    /// Delete all code chunks for a project, returns count of deleted chunks
    async fn delete_project_chunks(&self, project_id: &str) -> Result<usize>;

    /// Get indexing status for a project
    async fn get_index_status(&self, project_id: &str) -> Result<Option<IndexStatus>>;

    /// Update/upsert indexing status for a project
    async fn update_index_status(&self, status: IndexStatus) -> Result<()>;

    /// List all indexed project IDs
    async fn list_projects(&self) -> Result<Vec<String>>;

    // ─────────────────────────────────────────────────────────────────────────
    // System
    // ─────────────────────────────────────────────────────────────────────────

    /// Check if the database is healthy and responsive
    async fn health_check(&self) -> Result<bool>;
}
