use std::collections::HashMap;
use std::path::Path;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::Surreal;

use super::StorageBackend;
use crate::types::{
    CodeChunk, Direction, Entity, IndexStatus, Memory, MemoryUpdate, Relation, ScoredCodeChunk,
    SearchResult,
};
use crate::Result;

pub struct SurrealStorage {
    db: Surreal<Db>,
}

impl SurrealStorage {
    pub async fn new(data_dir: &Path) -> Result<Self> {
        let db_path = data_dir.join("db");
        std::fs::create_dir_all(&db_path)?;

        let db: Surreal<Db> = Surreal::new::<SurrealKv>(db_path).await?;
        db.use_ns("memory").use_db("main").await?;

        let schema = include_str!("schema.surql");
        db.query(schema).await?;

        Ok(Self { db })
    }
}

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let rand: u64 = (now as u64) ^ (std::process::id() as u64);
    format!("{:016x}{:04x}", now as u64, rand & 0xFFFF)
}

#[async_trait]
impl StorageBackend for SurrealStorage {
    async fn create_memory(&self, mut memory: Memory) -> Result<String> {
        let id = generate_id();
        memory.id = Some(surrealdb::sql::Thing::from(("memories", id.as_str())));
        let _: Option<Memory> = self.db.create(("memories", &id)).content(memory).await?;
        Ok(id)
    }

    async fn get_memory(&self, id: &str) -> Result<Option<Memory>> {
        let result: Option<Memory> = self.db.select(("memories", id)).await?;
        Ok(result)
    }

    async fn update_memory(&self, id: &str, update: MemoryUpdate) -> Result<Memory> {
        let existing: Option<Memory> = self.db.select(("memories", id)).await?;
        let mut memory = existing.ok_or_else(|| crate::types::AppError::NotFound(id.to_string()))?;

        if let Some(content) = update.content {
            memory.content = content;
        }
        if let Some(memory_type) = update.memory_type {
            memory.memory_type = memory_type;
        }
        if let Some(metadata) = update.metadata {
            memory.metadata = Some(metadata);
        }

        let updated: Option<Memory> = self.db.update(("memories", id)).content(memory).await?;
        updated.ok_or_else(|| crate::types::AppError::NotFound(id.to_string()))
    }

    async fn delete_memory(&self, id: &str) -> Result<bool> {
        let deleted: Option<Memory> = self.db.delete(("memories", id)).await?;
        Ok(deleted.is_some())
    }

    async fn list_memories(&self, limit: usize, offset: usize) -> Result<Vec<Memory>> {
        let query = "SELECT * FROM memories ORDER BY ingestion_time DESC LIMIT $limit START $offset";
        let mut response = self
            .db
            .query(query)
            .bind(("limit", limit))
            .bind(("offset", offset))
            .await?;
        let memories: Vec<Memory> = response.take(0)?;
        Ok(memories)
    }

    async fn count_memories(&self) -> Result<usize> {
        let mut response = self.db.query("SELECT count() FROM memories GROUP ALL").await?;
        let result: Option<serde_json::Value> = response.take(0)?;
        let count = result
            .and_then(|v| v.get("count").and_then(|c| c.as_u64()))
            .unwrap_or(0) as usize;
        Ok(count)
    }

    async fn vector_search(&self, embedding: &[f32], limit: usize) -> Result<Vec<SearchResult>> {
        let query = r#"
            SELECT *, vector::similarity::cosine(embedding, $vec) AS score 
            FROM memories 
            WHERE embedding IS NOT NULL 
              AND (valid_until IS NULL OR valid_until > time::now())
            ORDER BY score DESC 
            LIMIT $limit
        "#;
        let mut response = self
            .db
            .query(query)
            .bind(("vec", embedding.to_vec()))
            .bind(("limit", limit))
            .await?;
        let results: Vec<SearchResult> = response.take(0)?;
        Ok(results)
    }

    async fn vector_search_code(
        &self,
        embedding: &[f32],
        project_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ScoredCodeChunk>> {
        let query = r#"
            SELECT *, vector::similarity::cosine(embedding, $vec) AS score 
            FROM code_chunks 
            WHERE embedding IS NOT NULL
              AND ($project_id IS NULL OR project_id = $project_id)
            ORDER BY score DESC 
            LIMIT $limit
        "#;
        let mut response = self
            .db
            .query(query)
            .bind(("vec", embedding.to_vec()))
            .bind(("project_id", project_id.map(String::from)))
            .bind(("limit", limit))
            .await?;
        let results: Vec<ScoredCodeChunk> = response.take(0)?;
        Ok(results)
    }

    async fn bm25_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let sql = r#"
            SELECT *, search::score(0) AS score 
            FROM memories 
            WHERE content @0@ $query
              AND (valid_until IS NULL OR valid_until > time::now())
            ORDER BY score DESC 
            LIMIT $limit
        "#;
        let mut response = self
            .db
            .query(sql)
            .bind(("query", query.to_string()))
            .bind(("limit", limit))
            .await?;
        let results: Vec<SearchResult> = response.take(0)?;
        Ok(results)
    }

    async fn bm25_search_code(
        &self,
        query: &str,
        project_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ScoredCodeChunk>> {
        let sql = r#"
            SELECT *, search::score(0) AS score 
            FROM code_chunks 
            WHERE content @0@ $query
              AND ($project_id IS NULL OR project_id = $project_id)
            ORDER BY score DESC 
            LIMIT $limit
        "#;
        let mut response = self
            .db
            .query(sql)
            .bind(("query", query.to_string()))
            .bind(("project_id", project_id.map(String::from)))
            .bind(("limit", limit))
            .await?;
        let results: Vec<ScoredCodeChunk> = response.take(0)?;
        Ok(results)
    }

    async fn create_entity(&self, mut entity: Entity) -> Result<String> {
        let id = generate_id();
        entity.id = Some(surrealdb::sql::Thing::from(("entities", id.as_str())));
        let _: Option<Entity> = self.db.create(("entities", &id)).content(entity).await?;
        Ok(id)
    }

    async fn get_entity(&self, id: &str) -> Result<Option<Entity>> {
        let result: Option<Entity> = self.db.select(("entities", id)).await?;
        Ok(result)
    }

    async fn search_entities(&self, query: &str, limit: usize) -> Result<Vec<Entity>> {
        let sql = r#"
            SELECT *, search::score(0) AS score 
            FROM entities 
            WHERE name @0@ $query
            ORDER BY score DESC 
            LIMIT $limit
        "#;
        let mut response = self
            .db
            .query(sql)
            .bind(("query", query.to_string()))
            .bind(("limit", limit))
            .await?;
        let results: Vec<Entity> = response.take(0)?;
        Ok(results)
    }

    async fn create_relation(&self, relation: Relation) -> Result<String> {
        let sql = r#"
            RELATE $from_entity->relations->$to_entity 
            SET relation_type = $relation_type, weight = $weight
        "#;
        let from = relation.from_entity.clone();
        let to = relation.to_entity.clone();
        let rel_type = relation.relation_type.clone();
        let mut response = self
            .db
            .query(sql)
            .bind(("from_entity", from))
            .bind(("to_entity", to))
            .bind(("relation_type", rel_type))
            .bind(("weight", relation.weight))
            .await?;
        let created: Option<Relation> = response.take(0)?;
        let id = created
            .and_then(|r| r.id)
            .map(|t| t.id.to_string())
            .unwrap_or_else(generate_id);
        Ok(id)
    }

    async fn get_related(
        &self,
        entity_id: &str,
        depth: usize,
        direction: Direction,
    ) -> Result<(Vec<Entity>, Vec<Relation>)> {
        let depth = depth.min(3);
        let arrow = match direction {
            Direction::Outgoing => "->",
            Direction::Incoming => "<-",
            Direction::Both => "<->",
        };
        let sql = format!(
            "SELECT {}relations.{} AS related FROM entities:{} FETCH related",
            arrow, depth, entity_id
        );
        let mut response = self.db.query(&sql).await?;
        let entities: Vec<Entity> = response.take(0).unwrap_or_default();
        let relations: Vec<Relation> = vec![];
        Ok((entities, relations))
    }

    async fn get_subgraph(&self, entity_ids: &[String]) -> Result<(Vec<Entity>, Vec<Relation>)> {
        if entity_ids.is_empty() {
            return Ok((vec![], vec![]));
        }
        let ids_str = entity_ids
            .iter()
            .map(|id| format!("entities:{}", id))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT * FROM relations WHERE in IN [{}] AND out IN [{}]",
            ids_str, ids_str
        );
        let mut response = self.db.query(&sql).await?;
        let relations: Vec<Relation> = response.take(0).unwrap_or_default();

        let entity_sql = format!("SELECT * FROM entities WHERE id IN [{}]", ids_str);
        let mut entity_response = self.db.query(&entity_sql).await?;
        let entities: Vec<Entity> = entity_response.take(0).unwrap_or_default();

        Ok((entities, relations))
    }

    async fn get_node_degrees(&self, entity_ids: &[String]) -> Result<HashMap<String, usize>> {
        let mut degrees = HashMap::new();
        for id in entity_ids {
            let sql = format!(
                "SELECT count() FROM relations WHERE in = entities:{} OR out = entities:{} GROUP ALL",
                id, id
            );
            let mut response = self.db.query(&sql).await?;
            let result: Option<serde_json::Value> = response.take(0).ok().flatten();
            let count = result
                .and_then(|v| v.get("count").and_then(|c| c.as_u64()))
                .unwrap_or(0) as usize;
            degrees.insert(id.clone(), count);
        }
        Ok(degrees)
    }

    async fn get_valid(&self, user_id: Option<&str>, limit: usize) -> Result<Vec<Memory>> {
        let sql = r#"
            SELECT * FROM memories 
            WHERE (valid_until IS NULL OR valid_until > time::now())
              AND ($user_id IS NULL OR user_id = $user_id)
            ORDER BY ingestion_time DESC
            LIMIT $limit
        "#;
        let mut response = self
            .db
            .query(sql)
            .bind(("user_id", user_id.map(String::from)))
            .bind(("limit", limit))
            .await?;
        let memories: Vec<Memory> = response.take(0)?;
        Ok(memories)
    }

    async fn get_valid_at(
        &self,
        timestamp: DateTime<Utc>,
        user_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Memory>> {
        let sql = r#"
            SELECT * FROM memories 
            WHERE valid_from <= $timestamp 
              AND (valid_until IS NULL OR valid_until > $timestamp)
              AND ($user_id IS NULL OR user_id = $user_id)
            ORDER BY ingestion_time DESC
            LIMIT $limit
        "#;
        let mut response = self
            .db
            .query(sql)
            .bind(("timestamp", timestamp))
            .bind(("user_id", user_id.map(String::from)))
            .bind(("limit", limit))
            .await?;
        let memories: Vec<Memory> = response.take(0)?;
        Ok(memories)
    }

    async fn invalidate(
        &self,
        id: &str,
        reason: Option<&str>,
        _superseded_by: Option<&str>,
    ) -> Result<bool> {
        let sql = r#"
            UPDATE memories SET 
                valid_until = time::now(),
                invalidation_reason = $reason
            WHERE id = $id
        "#;
        let mut response = self
            .db
            .query(sql)
            .bind(("id", format!("memories:{}", id)))
            .bind(("reason", reason.map(String::from)))
            .await?;
        let updated: Option<Memory> = response.take(0).ok().flatten();
        Ok(updated.is_some())
    }

    async fn create_code_chunk(&self, mut chunk: CodeChunk) -> Result<String> {
        let id = generate_id();
        chunk.id = Some(surrealdb::sql::Thing::from(("code_chunks", id.as_str())));
        let _: Option<CodeChunk> = self.db.create(("code_chunks", &id)).content(chunk).await?;
        Ok(id)
    }

    async fn create_code_chunks_batch(&self, chunks: Vec<CodeChunk>) -> Result<usize> {
        let count = chunks.len();
        for chunk in chunks {
            self.create_code_chunk(chunk).await?;
        }
        Ok(count)
    }

    async fn delete_project_chunks(&self, project_id: &str) -> Result<usize> {
        let sql = "DELETE FROM code_chunks WHERE project_id = $project_id RETURN BEFORE";
        let mut response = self.db.query(sql).bind(("project_id", project_id.to_string())).await?;
        let deleted: Vec<CodeChunk> = response.take(0).unwrap_or_default();
        Ok(deleted.len())
    }

    async fn get_index_status(&self, project_id: &str) -> Result<Option<IndexStatus>> {
        let sql = "SELECT * FROM index_status WHERE project_id = $project_id LIMIT 1";
        let mut response = self.db.query(sql).bind(("project_id", project_id.to_string())).await?;
        let result: Vec<IndexStatus> = response.take(0).unwrap_or_default();
        Ok(result.into_iter().next())
    }

    async fn update_index_status(&self, status: IndexStatus) -> Result<()> {
        let sql = r#"
            UPSERT index_status SET
                project_id = $project_id,
                status = $status,
                total_files = $total_files,
                indexed_files = $indexed_files,
                total_chunks = $total_chunks,
                started_at = $started_at,
                completed_at = $completed_at,
                error_message = $error_message
            WHERE project_id = $project_id
        "#;
        let project_id = status.project_id.clone();
        let status_str = status.status.to_string();
        self.db
            .query(sql)
            .bind(("project_id", project_id))
            .bind(("status", status_str))
            .bind(("total_files", status.total_files))
            .bind(("indexed_files", status.indexed_files))
            .bind(("total_chunks", status.total_chunks))
            .bind(("started_at", status.started_at))
            .bind(("completed_at", status.completed_at))
            .bind(("error_message", status.error_message))
            .await?;
        Ok(())
    }

    async fn list_projects(&self) -> Result<Vec<String>> {
        let sql = "SELECT DISTINCT project_id FROM code_chunks";
        let mut response = self.db.query(sql).await?;
        let results: Vec<serde_json::Value> = response.take(0).unwrap_or_default();
        let projects = results
            .into_iter()
            .filter_map(|v| v.get("project_id").and_then(|p| p.as_str()).map(String::from))
            .collect();
        Ok(projects)
    }

    async fn health_check(&self) -> Result<bool> {
        self.db.query("INFO FOR DB").await?;
        Ok(true)
    }
}
