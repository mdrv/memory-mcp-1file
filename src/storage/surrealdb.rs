use std::collections::HashMap;
use std::path::Path;

use async_trait::async_trait;
use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::sql::Datetime;
use surrealdb::Surreal;

use super::StorageBackend;
use crate::types::{
    CodeChunk, CodeSymbol, Direction, Entity, IndexStatus, Memory, MemoryUpdate, Relation,
    ScoredCodeChunk, SearchResult, SymbolRelation,
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

    pub async fn check_dimension(&self, expected: usize) -> Result<()> {
        let mut response = self.db.query("INFO FOR TABLE memories").await?;
        let result: Option<serde_json::Value> = response.take(0)?;

        if let Some(info) = result {
            if let Some(indexes) = info.get("indexes").and_then(|i| i.as_object()) {
                if let Some(idx_def) = indexes.get("idx_memories_vec").and_then(|v| v.as_str()) {
                    if let Some(dim) = self.extract_dimension(idx_def) {
                        if dim != expected {
                            return Err(crate::types::AppError::DimensionMismatch {
                                model: expected,
                                db: dim,
                            });
                        }
                        tracing::info!(model = expected, db = dim, "Dimension check passed");
                        return Ok(());
                    }
                }
            }
        }

        Ok(())
    }

    fn extract_dimension(&self, def: &str) -> Option<usize> {
        def.split("DIMENSION ")
            .nth(1)?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
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
        let mut memory =
            existing.ok_or_else(|| crate::types::AppError::NotFound(id.to_string()))?;

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
        let query =
            "SELECT * FROM memories ORDER BY ingestion_time DESC LIMIT $limit START $offset";
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
        let mut response = self
            .db
            .query("SELECT count() FROM memories GROUP ALL")
            .await?;
        let result: Option<serde_json::Value> = response.take(0)?;
        let count = result
            .and_then(|v| v.get("count").and_then(|c| c.as_u64()))
            .unwrap_or(0) as usize;
        Ok(count)
    }

    async fn vector_search(&self, embedding: &[f32], limit: usize) -> Result<Vec<SearchResult>> {
        let query = r#"
            SELECT meta::id(id) AS id, content, memory_type, vector::similarity::cosine(embedding, $vec) AS score, metadata 
            FROM memories 
            WHERE embedding IS NOT NULL 
              AND (valid_until = NONE OR valid_until > time::now())
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
            SELECT 
                meta::id(id) AS id,
                file_path,
                content,
                language,
                start_line,
                end_line,
                chunk_type,
                name,
                vector::similarity::cosine(embedding, $vec) AS score 
            FROM code_chunks
            WHERE embedding IS NOT NONE
              AND ($project_id = NONE OR project_id = $project_id)
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
            SELECT meta::id(id) AS id, content, memory_type, search::score(0) AS score, metadata 
            FROM memories 
            WHERE content @0@ $query
              AND (valid_until = NONE OR valid_until > time::now())
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
            SELECT 
                meta::id(id) AS id,
                file_path,
                content,
                language,
                start_line,
                end_line,
                chunk_type,
                name,
                search::score(0) AS score 
            FROM code_chunks 
            WHERE content @0@ $query
              AND ($project_id = NONE OR project_id = $project_id)
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
        let id = generate_id();
        let from_thing = format!("{}:{}", relation.from_entity.tb, relation.from_entity.id);
        let to_thing = format!("{}:{}", relation.to_entity.tb, relation.to_entity.id);

        let sql = format!(
            "CREATE relations:{} SET `in` = {}, `out` = {}, relation_type = $rel_type, weight = $weight",
            id, from_thing, to_thing
        );

        self.db
            .query(&sql)
            .bind(("rel_type", relation.relation_type))
            .bind(("weight", relation.weight))
            .await?;

        Ok(id)
    }

    async fn get_related(
        &self,
        entity_id: &str,
        depth: usize,
        direction: Direction,
    ) -> Result<(Vec<Entity>, Vec<Relation>)> {
        let _depth = depth.clamp(1, 3);
        let entity_thing = format!("entities:{}", entity_id);

        let sql = match direction {
            Direction::Outgoing => {
                "SELECT * FROM relations WHERE `in` = type::thing($entity_id)"
            }
            Direction::Incoming => {
                "SELECT * FROM relations WHERE `out` = type::thing($entity_id)"
            }
            Direction::Both => {
                "SELECT * FROM relations WHERE `in` = type::thing($entity_id) OR `out` = type::thing($entity_id)"
            }
        };

        let mut response = self
            .db
            .query(sql)
            .bind(("entity_id", entity_thing.clone()))
            .await?;

        let relations: Vec<Relation> = response.take(0).unwrap_or_default();

        let mut entity_ids: Vec<String> = vec![];
        for rel in &relations {
            match direction {
                Direction::Outgoing => {
                    entity_ids.push(rel.to_entity.id.to_string());
                }
                Direction::Incoming => {
                    entity_ids.push(rel.from_entity.id.to_string());
                }
                Direction::Both => {
                    if rel.from_entity.id.to_string() != entity_id {
                        entity_ids.push(rel.from_entity.id.to_string());
                    }
                    if rel.to_entity.id.to_string() != entity_id {
                        entity_ids.push(rel.to_entity.id.to_string());
                    }
                }
            }
        }

        let mut entities: Vec<Entity> = vec![];
        for eid in entity_ids {
            if let Some(entity) = self.get_entity(&eid).await? {
                entities.push(entity);
            }
        }

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
            let mut response = self.db.query(sql).await?;
            let result: Option<serde_json::Value> = response.take(0).ok().flatten();
            let count = result
                .and_then(|v| v.get("count").and_then(|c| c.as_u64()))
                .unwrap_or(0) as usize;
            degrees.insert(id.clone(), count);
        }
        Ok(degrees)
    }

    async fn get_all_entities(&self) -> Result<Vec<Entity>> {
        let mut response = self.db.query("SELECT * FROM entities").await?;
        let entities: Vec<Entity> = response.take(0)?;
        Ok(entities)
    }

    async fn get_all_relations(&self) -> Result<Vec<Relation>> {
        let mut response = self.db.query("SELECT * FROM relations").await?;
        let relations: Vec<Relation> = response.take(0)?;
        Ok(relations)
    }

    async fn get_valid(&self, user_id: Option<&str>, limit: usize) -> Result<Vec<Memory>> {
        let sql = r#"
            SELECT * FROM memories 
            WHERE (valid_until = NONE OR valid_until > time::now())
              AND ($user_id = NONE OR user_id = $user_id)
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
        timestamp: Datetime,
        user_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Memory>> {
        let sql = r#"
            SELECT * FROM memories 
            WHERE valid_from <= $timestamp 
              AND (valid_until = NONE OR valid_until > $timestamp)
              AND ($user_id = NONE OR user_id = $user_id)
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
            UPDATE type::thing("memories", $id) SET 
                valid_until = time::now(),
                invalidation_reason = $reason
        "#;
        let mut response = self
            .db
            .query(sql)
            .bind(("id", id.to_string()))
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

    async fn create_code_chunks_batch(&self, mut chunks: Vec<CodeChunk>) -> Result<Vec<String>> {
        let count = chunks.len();
        if count == 0 {
            return Ok(vec![]);
        }

        for chunk in &mut chunks {
            if chunk.id.is_none() {
                let id = generate_id();
                chunk.id = Some(surrealdb::sql::Thing::from(("code_chunks", id.as_str())));
            }
        }

        let created: Vec<CodeChunk> = self.db.insert("code_chunks").content(chunks).await?;

        let ids = created
            .into_iter()
            .filter_map(|c| c.id.map(|t| t.to_string()))
            .collect();

        Ok(ids)
    }

    async fn delete_project_chunks(&self, project_id: &str) -> Result<usize> {
        let sql = "DELETE FROM code_chunks WHERE project_id = $project_id RETURN BEFORE";
        let mut response = self
            .db
            .query(sql)
            .bind(("project_id", project_id.to_string()))
            .await?;
        let deleted: Vec<CodeChunk> = response.take(0).unwrap_or_default();
        Ok(deleted.len())
    }

    async fn delete_chunks_by_path(&self, project_id: &str, file_path: &str) -> Result<usize> {
        let sql = "DELETE FROM code_chunks WHERE project_id = $project_id AND file_path = $file_path RETURN BEFORE";
        let mut response = self
            .db
            .query(sql)
            .bind(("project_id", project_id.to_string()))
            .bind(("file_path", file_path.to_string()))
            .await?;
        let deleted: Vec<CodeChunk> = response.take(0).unwrap_or_default();
        Ok(deleted.len())
    }

    async fn get_chunks_by_path(
        &self,
        project_id: &str,
        file_path: &str,
    ) -> Result<Vec<CodeChunk>> {
        let sql =
            "SELECT * FROM code_chunks WHERE project_id = $project_id AND file_path = $file_path";
        let mut response = self
            .db
            .query(sql)
            .bind(("project_id", project_id.to_string()))
            .bind(("file_path", file_path.to_string()))
            .await?;
        let chunks: Vec<CodeChunk> = response.take(0).unwrap_or_default();
        Ok(chunks)
    }

    async fn get_index_status(&self, project_id: &str) -> Result<Option<IndexStatus>> {
        let sql = "SELECT * FROM index_status WHERE project_id = $project_id LIMIT 1";
        let mut response = self
            .db
            .query(sql)
            .bind(("project_id", project_id.to_string()))
            .await?;
        let result: Vec<IndexStatus> = response.take(0).unwrap_or_default();
        Ok(result.into_iter().next())
    }

    async fn update_index_status(&self, status: IndexStatus) -> Result<()> {
        let id = ("index_status", &status.project_id);
        let _: Option<IndexStatus> = self.db.update(id).content(status).await?;
        Ok(())
    }

    async fn delete_index_status(&self, project_id: &str) -> Result<()> {
        let sql = "DELETE FROM index_status WHERE project_id = $project_id";
        self.db
            .query(sql)
            .bind(("project_id", project_id.to_string()))
            .await?;
        Ok(())
    }

    async fn list_projects(&self) -> Result<Vec<String>> {
        let sql = "SELECT project_id FROM code_chunks GROUP BY project_id";
        let mut response = self.db.query(sql).await?;
        let results: Vec<serde_json::Value> = response.take(0).unwrap_or_default();
        let projects = results
            .into_iter()
            .filter_map(|v| {
                v.get("project_id")
                    .and_then(|p| p.as_str())
                    .map(String::from)
            })
            .collect();
        Ok(projects)
    }

    async fn create_code_symbol(&self, mut symbol: CodeSymbol) -> Result<String> {
        let id = ("code_symbols", &symbol.unique_key());
        symbol.id = None;
        let _: Option<CodeSymbol> = self.db.create(id).content(symbol).await?;
        Ok(format!("code_symbols:{}", id.1))
    }

    async fn create_code_symbols_batch(&self, symbols: Vec<CodeSymbol>) -> Result<Vec<String>> {
        if symbols.is_empty() {
            return Ok(vec![]);
        }
        let created: Vec<CodeSymbol> = self.db.insert("code_symbols").content(symbols).await?;

        let ids = created
            .into_iter()
            .filter_map(|s| s.id.map(|t| t.to_string()))
            .collect();

        Ok(ids)
    }

    async fn update_symbol_embedding(&self, id: &str, embedding: Vec<f32>) -> Result<()> {
        let sql = "UPDATE code_symbols SET embedding = $embedding WHERE id = $id";
        let _ = self
            .db
            .query(sql)
            .bind(("embedding", embedding))
            .bind(("id", id.to_string()))
            .await?;
        Ok(())
    }

    async fn update_chunk_embedding(&self, id: &str, embedding: Vec<f32>) -> Result<()> {
        let sql = "UPDATE code_chunks SET embedding = $embedding WHERE id = $id";
        let _ = self
            .db
            .query(sql)
            .bind(("embedding", embedding))
            .bind(("id", id.to_string()))
            .await?;
        Ok(())
    }

    async fn create_symbol_relation(&self, relation: SymbolRelation) -> Result<String> {
        let sql = "RELATE $from->symbol_relation->$to CONTENT $content";
        let from = relation.from_symbol.clone();
        let to = relation.to_symbol.clone();

        let _ = self
            .db
            .query(sql)
            .bind(("from", from))
            .bind(("to", to))
            .bind(("content", relation))
            .await?;
        Ok("relation_created".to_string())
    }

    async fn delete_project_symbols(&self, project_id: &str) -> Result<usize> {
        let sql = "DELETE code_symbols WHERE project_id = $project_id";
        let _ = self
            .db
            .query(sql)
            .bind(("project_id", project_id.to_string()))
            .await?;
        Ok(0)
    }

    async fn delete_symbols_by_path(&self, project_id: &str, file_path: &str) -> Result<usize> {
        let sql = "DELETE code_symbols WHERE project_id = $project_id AND file_path = $file_path";
        let _ = self
            .db
            .query(sql)
            .bind(("project_id", project_id.to_string()))
            .bind(("file_path", file_path.to_string()))
            .await?;
        Ok(0)
    }

    async fn get_symbol_callers(&self, symbol_id: &str) -> Result<Vec<CodeSymbol>> {
        // Query to get Incoming edges (calls) -> From Node
        let sql = r#"
            SELECT * FROM code_symbols 
            WHERE id IN (
                SELECT VALUE in FROM symbol_relation 
                WHERE out = type::thing($id) AND relation_type = 'calls'
            )
        "#;

        let mut response = self
            .db
            .query(sql)
            .bind(("id", symbol_id.to_string()))
            .await?;

        let symbols: Vec<CodeSymbol> = response.take(0)?;
        Ok(symbols)
    }

    async fn get_symbol_callees(&self, symbol_id: &str) -> Result<Vec<CodeSymbol>> {
        let sql = "SELECT * FROM (SELECT ->symbol_relation[WHERE relation_type = 'calls'].out as callees FROM type::thing($id)).callees";
        let mut response = self
            .db
            .query(sql)
            .bind(("id", symbol_id.to_string()))
            .await?;
        let result: Option<Vec<CodeSymbol>> = response.take(0)?;
        Ok(result.unwrap_or_default())
    }

    async fn get_related_symbols(
        &self,
        symbol_id: &str,
        depth: usize,
        direction: Direction,
    ) -> Result<(Vec<CodeSymbol>, Vec<SymbolRelation>)> {
        let _depth = depth.clamp(1, 3);
        // Ensure symbol_id is in correct format "code_symbols:..."
        let symbol_thing = if !symbol_id.contains(':') {
            format!("code_symbols:{}", symbol_id)
        } else {
            symbol_id.to_string()
        };

        let sql = match direction {
            Direction::Outgoing => {
                "SELECT * FROM symbol_relation WHERE `in` = type::thing($id)"
            }
            Direction::Incoming => {
                "SELECT * FROM symbol_relation WHERE `out` = type::thing($id)"
            }
            Direction::Both => {
                "SELECT * FROM symbol_relation WHERE `in` = type::thing($id) OR `out` = type::thing($id)"
            }
        };

        let mut response = self
            .db
            .query(sql)
            .bind(("id", symbol_thing.clone()))
            .await?;

        let relations: Vec<SymbolRelation> = response.take(0).unwrap_or_default();

        let mut symbol_ids: Vec<String> = vec![];
        for rel in &relations {
            match direction {
                Direction::Outgoing => {
                    symbol_ids.push(rel.to_symbol.to_string());
                }
                Direction::Incoming => {
                    symbol_ids.push(rel.from_symbol.to_string());
                }
                Direction::Both => {
                    let from_str = rel.from_symbol.to_string();
                    let to_str = rel.to_symbol.to_string();

                    if from_str != symbol_thing {
                        symbol_ids.push(from_str);
                    }
                    if to_str != symbol_thing {
                        symbol_ids.push(to_str);
                    }
                }
            }
        }

        // Fetch symbols by ID
        // Note: SurrealDB thing IDs in relation are strings like "code_symbols:id"
        // We can fetch them directly.
        let mut symbols: Vec<CodeSymbol> = vec![];
        for sid in symbol_ids {
            // Need to parse ID part if it's "table:id" format
            let id_part = if let Some(idx) = sid.find(':') {
                &sid[idx + 1..]
            } else {
                &sid
            };

            // Re-using a get_symbol logic would be better, but we don't have get_symbol_by_id yet.
            // Let's do a direct select
            let s: Option<CodeSymbol> = self.db.select(("code_symbols", id_part)).await?;
            if let Some(sym) = s {
                symbols.push(sym);
            }
        }

        Ok((symbols, relations))
    }

    async fn search_symbols(
        &self,
        query: &str,
        project_id: Option<&str>,
    ) -> Result<Vec<CodeSymbol>> {
        let mut sql = "SELECT * FROM code_symbols WHERE name ~ $query".to_string();
        if let Some(_) = project_id {
            sql.push_str(" AND project_id = $project_id");
        }
        sql.push_str(" LIMIT 20");

        let mut query_builder = self.db.query(sql).bind(("query", query.to_string()));
        if let Some(pid) = project_id {
            query_builder = query_builder.bind(("project_id", pid.to_string()));
        }

        let mut response = query_builder.await?;
        let symbols: Vec<CodeSymbol> = response.take(0)?;
        Ok(symbols)
    }

    async fn health_check(&self) -> Result<bool> {
        self.db.query("INFO FOR DB").await?;
        Ok(true)
    }

    async fn reset_db(&self) -> Result<()> {
        self.db
            .query(
                r#"
            BEGIN TRANSACTION;
            DELETE memories;
            DELETE entities;
            DELETE relations;
            DELETE code_chunks;
            DELETE code_symbols;
            DELETE symbol_relation;
            DELETE index_status;
            COMMIT TRANSACTION;
            "#,
            )
            .await?;
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        self.db.query("RETURN true").await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Entity, Memory, MemoryType, MemoryUpdate, Relation};
    use surrealdb::sql::{Datetime, Thing};
    use tempfile::tempdir;

    async fn setup_test_db() -> (SurrealStorage, tempfile::TempDir) {
        let tmp = tempdir().unwrap();
        let storage = SurrealStorage::new(tmp.path()).await.unwrap();
        (storage, tmp)
    }

    #[tokio::test]
    async fn test_memory_crud() {
        let (storage, _tmp) = setup_test_db().await;

        let memory = Memory {
            id: None,
            content: "Test memory content".to_string(),
            embedding: Some(vec![0.1; 768]),
            memory_type: MemoryType::Semantic,
            user_id: Some("user1".to_string()),
            metadata: None,
            event_time: Datetime::default(),
            ingestion_time: Datetime::default(),
            valid_from: Datetime::default(),
            valid_until: None,
            importance_score: 1.0,
            invalidation_reason: None,
        };

        let id = storage.create_memory(memory.clone()).await.unwrap();
        assert!(!id.is_empty());

        let retrieved = storage
            .get_memory(&id)
            .await
            .unwrap()
            .expect("Memory not found");
        assert_eq!(retrieved.content, memory.content);
        assert_eq!(retrieved.user_id, memory.user_id);

        let update = MemoryUpdate {
            content: Some("Updated content".to_string()),
            memory_type: None,
            metadata: None,
        };
        let updated = storage.update_memory(&id, update).await.unwrap();
        assert_eq!(updated.content, "Updated content");

        let list = storage.list_memories(10, 0).await.unwrap();
        assert_eq!(list.len(), 1);

        let deleted = storage.delete_memory(&id).await.unwrap();
        assert!(deleted);
        assert!(storage.get_memory(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_bm25_search() {
        let (storage, _tmp) = setup_test_db().await;

        storage
            .create_memory(Memory {
                id: None,
                content: "Rust programming language".to_string(),
                embedding: Some(vec![0.0; 768]),
                memory_type: MemoryType::Semantic,
                user_id: None,
                metadata: None,
                event_time: Datetime::default(),
                ingestion_time: Datetime::default(),
                valid_from: Datetime::default(),
                valid_until: None,
                importance_score: 1.0,
                invalidation_reason: None,
            })
            .await
            .unwrap();

        storage
            .create_memory(Memory {
                id: None,
                content: "Python scripting".to_string(),
                embedding: Some(vec![0.0; 768]),
                memory_type: MemoryType::Semantic,
                user_id: None,
                metadata: None,
                event_time: Datetime::default(),
                ingestion_time: Datetime::default(),
                valid_from: Datetime::default(),
                valid_until: None,
                importance_score: 1.0,
                invalidation_reason: None,
            })
            .await
            .unwrap();

        let results = storage.bm25_search("Rust", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("Rust"));
    }

    #[tokio::test]
    async fn test_entity_and_relation() {
        let (storage, _tmp) = setup_test_db().await;

        let e1_id = storage
            .create_entity(Entity {
                id: None,
                name: "Entity 1".to_string(),
                entity_type: "person".to_string(),
                description: None,
                embedding: None,
                user_id: None,
                created_at: Datetime::default(),
            })
            .await
            .unwrap();

        let e2_id = storage
            .create_entity(Entity {
                id: None,
                name: "Entity 2".to_string(),
                entity_type: "place".to_string(),
                description: None,
                embedding: None,
                user_id: None,
                created_at: Datetime::default(),
            })
            .await
            .unwrap();

        let _rel_id = storage
            .create_relation(Relation {
                id: None,
                from_entity: Thing::from(("entities".to_string(), e1_id.clone())),
                to_entity: Thing::from(("entities".to_string(), e2_id.clone())),
                relation_type: "lives_in".to_string(),
                weight: 1.0,
                valid_from: Datetime::default(),
                valid_until: None,
            })
            .await
            .unwrap();

        let (related, _) = storage
            .get_related(&e1_id, 1, Direction::Outgoing)
            .await
            .unwrap();
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].name, "Entity 2");
    }

    #[tokio::test]
    async fn test_symbol_call_hierarchy() {
        let (storage, _tmp) = setup_test_db().await;
        use crate::types::{CodeRelationType, CodeSymbol, SymbolRelation, SymbolType};

        // 1. Create Symbols: Caller -> Callee
        let caller = CodeSymbol::new(
            "main".to_string(),
            SymbolType::Function,
            "main.rs".to_string(),
            1,
            5,
            "test_project".to_string(),
        );
        let caller_id = storage.create_code_symbol(caller).await.unwrap();

        let callee = CodeSymbol::new(
            "helper".to_string(),
            SymbolType::Function,
            "helper.rs".to_string(),
            10,
            15,
            "test_project".to_string(),
        );
        let callee_id = storage.create_code_symbol(callee).await.unwrap();

        // 2. Create Relation: main calls helper
        let relation = SymbolRelation::new(
            surrealdb::sql::Thing::from((
                "code_symbols".to_string(),
                caller_id.split(':').nth(1).unwrap().to_string(),
            )),
            surrealdb::sql::Thing::from((
                "code_symbols".to_string(),
                callee_id.split(':').nth(1).unwrap().to_string(),
            )),
            CodeRelationType::Calls,
            "main.rs".to_string(),
            3,
        );
        storage.create_symbol_relation(relation).await.unwrap();

        // 3. Test get_symbol_callees (Outgoing)
        // main -> ? (should be helper)
        let callees = storage.get_symbol_callees(&caller_id).await.unwrap();
        assert_eq!(callees.len(), 1, "Should find 1 callee");
        assert_eq!(callees[0].name, "helper");

        // 4. Test get_symbol_callers (Incoming)
        // ? -> helper (should be main)
        let callers = storage.get_symbol_callers(&callee_id).await.unwrap();
        assert_eq!(callers.len(), 1, "Should find 1 caller");
        assert_eq!(callers[0].name, "main");
    }

    #[tokio::test]
    async fn test_temporal_validation() {
        let (storage, _tmp) = setup_test_db().await;

        let id = storage
            .create_memory(Memory {
                id: None,
                content: "Temporary memory".to_string(),
                embedding: Some(vec![0.0; 768]),
                memory_type: MemoryType::Semantic,
                user_id: None,
                metadata: None,
                event_time: Datetime::default(),
                ingestion_time: Datetime::default(),
                valid_from: Datetime::default(),
                valid_until: None,
                importance_score: 1.0,
                invalidation_reason: None,
            })
            .await
            .unwrap();

        let valid = storage.get_valid(None, 10).await.unwrap();
        assert_eq!(valid.len(), 1);

        storage
            .invalidate(&id, Some("test reason"), None)
            .await
            .unwrap();

        let valid_after = storage.get_valid(None, 10).await.unwrap();
        assert_eq!(valid_after.len(), 0);
    }

    #[tokio::test]
    async fn test_reset_db() {
        let (storage, _tmp) = setup_test_db().await;

        storage
            .create_memory(Memory {
                id: None,
                content: "To be deleted".to_string(),
                embedding: None,
                memory_type: MemoryType::Semantic,
                user_id: None,
                metadata: None,
                event_time: Datetime::default(),
                ingestion_time: Datetime::default(),
                valid_from: Datetime::default(),
                valid_until: None,
                importance_score: 1.0,
                invalidation_reason: None,
            })
            .await
            .unwrap();

        assert_eq!(storage.count_memories().await.unwrap(), 1);

        storage.reset_db().await.unwrap();

        assert_eq!(storage.count_memories().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_bulk_insert_code_chunks() {
        let (storage, _tmp) = setup_test_db().await;
        use crate::types::{ChunkType, CodeChunk, Language};

        let chunks: Vec<CodeChunk> = (0..50)
            .map(|i| CodeChunk {
                id: None,
                file_path: format!("src/file_{}.rs", i),
                content: format!("fn test_{}() {{}}", i),
                language: Language::Rust,
                start_line: 1,
                end_line: 3,
                chunk_type: ChunkType::Function,
                name: Some(format!("test_{}", i)),
                embedding: Some(vec![0.1; 768]),
                content_hash: format!("hash_{}", i),
                project_id: Some("test_project".to_string()),
                indexed_at: Datetime::default(),
            })
            .collect();

        let count = storage.create_code_chunks_batch(chunks).await.unwrap();
        assert_eq!(count, 50);

        let _status = storage.get_index_status("test_project").await.unwrap();
        // that's handled by the indexer. But we can verify chunks exist.

        let results = storage
            .bm25_search_code("test", Some("test_project"), 100)
            .await
            .unwrap();
        assert_eq!(results.len(), 50);
    }
}
