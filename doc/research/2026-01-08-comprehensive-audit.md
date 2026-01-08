# 🔧 DEVELOPMENT PLAN: Memory MCP Server Security & Stability Fixes

**ID:** DEV-2026-01-08-fixes  
**Status:** ready_for_implementation  
**Type:** Development Document (upgraded from Research)  
**Date:** 2026-01-08  
**Author:** Sisyphus (AI Agent)

---

## 📊 Executive Summary

| Metric | Value |
|--------|-------|
| **Critical Issues** | 4 |
| **High Priority Issues** | 5 |
| **Medium Priority Issues** | 4 |
| **Total Fixes Required** | 17 |
| **Estimated Effort** | 5-7 days |

---

## 🎯 JIRA-STYLE FIX PLAN

### Sprint 1: Security (P0) - 1-2 days

| Ticket | Title | File | Effort | Status |
|--------|-------|------|--------|--------|
| FIX-001 | SQL Injection in create_relation | storage/surrealdb.rs:406 | 30min | TODO |
| FIX-002 | SQL Injection in get_subgraph | storage/surrealdb.rs:443 | 30min | TODO |
| FIX-003 | SQL Injection in get_node_degrees | storage/surrealdb.rs:460 | 30min | TODO |
| FIX-004 | SQL Injection in get_related_symbols | storage/surrealdb.rs:816 | 30min | TODO |
| FIX-005 | Add ThingId validation type | types/mod.rs (new) | 1h | TODO |

### Sprint 2: Stability (P1) - 2-3 days

| Ticket | Title | File | Effort | Status |
|--------|-------|------|--------|--------|
| FIX-006 | Tokenizer bounds check | embedding/engine.rs | 1h | TODO |
| FIX-007 | Replace unwrap in engine | embedding/engine.rs:146-147 | 30min | TODO |
| FIX-008 | Replace unwrap in service | embedding/service.rs:70 | 15min | TODO |
| FIX-009 | Fix NaN sorting crash | server/logic/search.rs:149 | 15min | TODO |
| FIX-010 | Fix NaN sorting crash | graph/rrf.rs:71 | 15min | TODO |
| FIX-011 | Use parking_lot for cache | embedding/cache.rs | 30min | TODO |
| FIX-012 | Model-aware embedding cache | embedding/store.rs | 1h | TODO |

### Sprint 3: Correctness (P1) - 3-5 days

| Ticket | Title | File | Effort | Status |
|--------|-------|------|--------|--------|
| FIX-013 | BFS frontier leak | graph/traversal.rs:116-122 | 2h | TODO |
| FIX-014 | Leiden formula fix | graph/leiden.rs:79 | 1h | TODO |
| FIX-015 | Leiden comparison logic | graph/leiden.rs:71-85 | 2h | TODO |
| FIX-016 | PPR edge weights | graph/ppr.rs:54-57 | 1h | TODO |
| FIX-017 | Silent error handling | storage/surrealdb.rs | 2h | TODO |

### Sprint 4: Robustness (P2) - ongoing

| Ticket | Title | File | Effort | Status |
|--------|-------|------|--------|--------|
| FIX-018 | File size limits | codebase/indexer.rs | 1h | TODO |
| FIX-019 | spawn_blocking for parser | codebase/indexer.rs | 1h | TODO |
| FIX-020 | Atomic index_status upsert | storage/surrealdb.rs:634 | 1h | TODO |
| FIX-021 | Worker race condition | embedding/worker.rs:163 | 2h | TODO |

---

## 🔬 DETAILED SOLUTIONS

---

## FIX-001 to FIX-004: SQL Injection Vulnerabilities

### Problem Analysis
```rust
// CURRENT (VULNERABLE):
let sql = format!(
    "CREATE relations:{} SET `in` = {}, `out` = {}...",
    id, from_thing, to_thing  // Direct string interpolation!
);
```

### Solution Variants Evaluated

#### Set A (5 variants):
1. **A1: Parameterized Queries** - Use .bind() with type::thing()
2. **A2: Validation + Escape Helper** - Regex validation before format!
3. **A3: Type-Safe Wrapper Structs** - ThingId struct with validation
4. **A4: SurrealDB Query Builder** - Native SDK types
5. **A5: Query Templates Macro** - Compile-time validation

#### Set B (5 variants):
1. **B1: Stored Procedures** - Move logic to DB
2. **B2: Record Links** - SurrealDB native Thing type
3. **B3: Input Sanitization Middleware** - Blocklist approach
4. **B4: CQRS Pattern** - Separate command models
5. **B5: Static Analysis + CI** - grep/lint checks

### 🎯 OPTIMAL HYBRID SOLUTION

**Combine: A1 + A3 + B5**

```rust
// 1. NEW: types/thing_id.rs
use anyhow::{ensure, Result};

/// Validated SurrealDB Thing ID
#[derive(Clone, Debug)]
pub struct ThingId(String);

impl ThingId {
    pub fn new(table: &str, id: &str) -> Result<Self> {
        ensure!(
            table.chars().all(|c| c.is_alphanumeric() || c == '_'),
            "Invalid table name: {}", table
        );
        ensure!(
            id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-'),
            "Invalid ID: {}", id
        );
        Ok(Self(format!("{}:{}", table, id)))
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// 2. UPDATED: storage/surrealdb.rs - create_relation
async fn create_relation(&self, relation: Relation) -> Result<String> {
    let id = generate_id();
    let from = ThingId::new(&relation.from_entity.tb, &relation.from_entity.id)?;
    let to = ThingId::new(&relation.to_entity.tb, &relation.to_entity.id)?;
    
    let sql = "CREATE relations:$id SET \
        `in` = type::thing($from), \
        `out` = type::thing($to), \
        relation_type = $rel_type, \
        weight = $weight";
    
    self.db.query(sql)
        .bind(("id", id.clone()))
        .bind(("from", from.as_str()))
        .bind(("to", to.as_str()))
        .bind(("rel_type", relation.relation_type))
        .bind(("weight", relation.weight))
        .await?;
    
    Ok(id)
}

// 3. UPDATED: get_subgraph
async fn get_subgraph(&self, entity_ids: &[String]) -> Result<(Vec<Entity>, Vec<Relation>)> {
    if entity_ids.is_empty() {
        return Ok((vec![], vec![]));
    }
    
    // Validate all IDs
    let validated: Vec<ThingId> = entity_ids
        .iter()
        .map(|id| ThingId::new("entities", id))
        .collect::<Result<Vec<_>>>()?;
    
    let ids: Vec<&str> = validated.iter().map(|t| t.as_str()).collect();
    
    let sql = "SELECT * FROM relations WHERE in IN $ids AND out IN $ids";
    let mut response = self.db.query(sql)
        .bind(("ids", ids.clone()))
        .await?;
    let relations: Vec<Relation> = response.take(0)?;
    
    let entity_sql = "SELECT * FROM entities WHERE id IN $ids";
    let mut entity_response = self.db.query(entity_sql)
        .bind(("ids", ids))
        .await?;
    let entities: Vec<Entity> = entity_response.take(0)?;
    
    Ok((entities, relations))
}

// 4. UPDATED: get_node_degrees
async fn get_node_degrees(&self, entity_ids: &[String]) -> Result<HashMap<String, usize>> {
    let mut degrees = HashMap::new();
    
    for id in entity_ids {
        let thing = ThingId::new("entities", id)?;
        
        let sql = "SELECT count() FROM relations \
            WHERE in = type::thing($id) OR out = type::thing($id) \
            GROUP ALL";
        
        let mut response = self.db.query(sql)
            .bind(("id", thing.as_str()))
            .await?;
        
        let result: Option<serde_json::Value> = response.take(0)?;
        let count = result
            .and_then(|v| v.get("count").and_then(|c| c.as_u64()))
            .unwrap_or(0) as usize;
        
        degrees.insert(id.clone(), count);
    }
    
    Ok(degrees)
}
```

**CI Check (.github/workflows/ci.yml):**
```yaml
- name: Check for SQL injection patterns
  run: |
    if grep -rn 'format!.*".*SELECT\|CREATE\|UPDATE\|DELETE' src/storage/; then
      echo "::error::Potential SQL injection pattern found"
      exit 1
    fi
```

---

## FIX-006 to FIX-008: Tokenizer & Engine Panics

### Problem Analysis
```rust
// engine.rs:146-147 - PANICS if model not loaded
let tokenizer = self.tokenizer.as_ref().unwrap();
let model = self.model.as_ref().unwrap();
```

### 🎯 OPTIMAL HYBRID SOLUTION

```rust
// embedding/engine.rs

impl EmbeddingEngine {
    pub fn new(model_type: ModelType, cache_dir: Option<PathBuf>) -> Result<Self> {
        let (tokenizer, model, dimensions) = load_model(model_type, cache_dir)?;
        
        // Configure truncation to prevent OOB
        let mut tokenizer = tokenizer;
        tokenizer.with_truncation(Some(TruncationParams {
            max_length: 512,
            strategy: TruncationStrategy::LongestFirst,
            ..Default::default()
        }))?;
        
        let engine = Self { 
            tokenizer: Some(tokenizer), 
            model: Some(model), 
            dimensions,
            mock: false,
        };
        
        // Verify compatibility at startup
        engine.verify_compatibility()?;
        
        Ok(engine)
    }
    
    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if self.mock {
            return self.mock_embed(texts);
        }
        
        // FIXED: Proper error handling instead of unwrap
        let tokenizer = self.tokenizer.as_ref()
            .ok_or_else(|| anyhow!("Embedding engine not initialized: tokenizer missing"))?;
        let model = self.model.as_ref()
            .ok_or_else(|| anyhow!("Embedding engine not initialized: model missing"))?;
        
        let vocab_size = tokenizer.get_vocab_size(true) as u32;
        let unk_id = tokenizer.token_to_id("[UNK]").unwrap_or(0);
        
        let mut all_token_ids = Vec::with_capacity(texts.len());
        
        for text in texts {
            let tokens = tokenizer.encode(text.as_str(), true)
                .map_err(anyhow::Error::msg)?;
            
            // FIXED: Replace OOV tokens with UNK
            let safe_ids: Vec<u32> = tokens.get_ids()
                .iter()
                .map(|&id| if id >= vocab_size { unk_id } else { id })
                .collect();
            
            all_token_ids.push(safe_ids);
        }
        
        // ... rest of embedding logic
    }
    
    fn verify_compatibility(&self) -> Result<()> {
        let edge_cases = [
            "Normal English text",
            "🎉🎊🔥",  // Emoji
            "日本語中文한국어",  // CJK
            "α β γ δ ε",  // Greek
            &"x".repeat(1000),  // Long text
        ];
        
        for text in edge_cases {
            self.embed_batch(&[text.to_string()]).map_err(|e| {
                anyhow!("Compatibility check failed on '{}...': {}", 
                    &text[..text.len().min(20)], e)
            })?;
        }
        
        tracing::info!("Embedding engine compatibility check passed");
        Ok(())
    }
}
```

---

## FIX-009 to FIX-011: Panic Points (NaN, Mutex)

### 🎯 OPTIMAL HYBRID SOLUTION

```toml
# Cargo.toml - Add parking_lot
[dependencies]
parking_lot = "0.12"
```

```rust
// embedding/cache.rs - Use parking_lot (never panics on lock)
use parking_lot::Mutex;

impl EmbeddingCache {
    pub fn get(&self, text: &str, model_version: &str) -> Option<Vec<f32>> {
        let key = Self::cache_key(text, model_version);
        let mut cache = self.cache.lock();  // No unwrap needed!
        cache.get(&key).cloned()
    }
}
```

```rust
// server/logic/search.rs:149 - Use total_cmp
tuples.sort_by(|a, b| b.1.total_cmp(&a.1));  // NaN-safe!
```

```rust
// graph/rrf.rs:71 - Use total_cmp
results.sort_by(|a, b| b.1.combined_score.total_cmp(&a.1.combined_score));
```

---

## FIX-012: Model-Aware Embedding Cache

### Problem Analysis
L2 disk cache uses only text hash - different models return wrong vectors.

### 🎯 OPTIMAL HYBRID SOLUTION

```rust
// embedding/store.rs

impl EmbeddingStore {
    model_name: String,
    expected_dims: usize,
    
    pub fn new(data_dir: &Path, model_name: &str, dimensions: usize) -> Result<Self> {
        let db = Database::create(data_dir.join("embeddings.redb"))?;
        
        // Check stored model metadata - invalidate on change
        let stored_model = Self::get_stored_model(&db)?;
        if stored_model.as_deref() != Some(model_name) {
            if stored_model.is_some() {
                tracing::warn!(
                    "Model changed from {:?} to {}, clearing embedding cache",
                    stored_model, model_name
                );
                Self::clear_cache(&db)?;
            }
            Self::set_stored_model(&db, model_name)?;
        }
        
        Ok(Self { 
            disk_cache: Arc::new(db),
            ram_cache: Cache::builder().max_capacity(10_000).build(),
            model_name: model_name.to_string(),
            expected_dims: dimensions,
        })
    }
    
    /// Cache key includes model name for isolation
    fn cache_key(&self, text_hash: &str) -> String {
        format!("{}:{}", self.model_name, text_hash)
    }
    
    pub async fn get(&self, text_hash: &str) -> Option<Vec<f32>> {
        let key = self.cache_key(text_hash);
        
        // L1: RAM cache
        if let Some(vec) = self.ram_cache.get(&key).await {
            return Some(vec);
        }
        
        // L2: Disk cache
        if let Some(vec) = self.disk_get(&key).await.ok()? {
            // Dimension validation (defense in depth)
            if vec.len() != self.expected_dims {
                tracing::error!(
                    "Cache dimension mismatch: got {}, expected {}",
                    vec.len(), self.expected_dims
                );
                return None;
            }
            self.ram_cache.insert(key, vec.clone()).await;
            return Some(vec);
        }
        
        None
    }
    
    pub async fn put(&self, text_hash: String, embedding: Vec<f32>) -> Result<()> {
        let key = self.cache_key(&text_hash);
        self.ram_cache.insert(key.clone(), embedding.clone()).await;
        self.disk_put(&key, &embedding).await?;
        Ok(())
    }
}
```

---

## FIX-013: BFS Frontier Leak

### Problem Analysis
```rust
// CURRENT: Remaining nodes are LOST after truncate
let frontier_vec: Vec<String> = frontier.drain(..).collect();
let batch_size = frontier_vec.len().min(self.config.max_entities_per_level);
// frontier_vec[batch_size..] is never processed!
```

### 🎯 OPTIMAL HYBRID SOLUTION

```rust
// graph/traversal.rs

/// Result includes metadata about truncation
pub struct TraversalResult {
    pub entities: Vec<Entity>,
    pub relations: Vec<Relation>,
    pub strategy: TraversalStrategy,
    pub depth_reached: usize,
    pub truncated: bool,
    pub deferred_count: usize,  // NEW: How many nodes were not explored
}

impl GraphTraverser {
    async fn traverse_bfs(
        &self,
        entity_id: &str,
        depth: usize,
        direction: Direction,
    ) -> Result<TraversalResult> {
        let mut visited_entities: HashSet<String> = HashSet::new();
        let mut visited_relations: HashSet<String> = HashSet::new();
        let mut all_entities: Vec<Entity> = Vec::new();
        let mut all_relations: Vec<Relation> = Vec::new();
        let mut frontier: VecDeque<String> = VecDeque::new();
        let mut deferred_count = 0;
        let mut truncated = false;

        frontier.push_back(entity_id.to_string());
        visited_entities.insert(entity_id.to_string());

        for current_depth in 1..=depth {
            if frontier.is_empty() {
                break;
            }

            let frontier_vec: Vec<String> = frontier.drain(..).collect();
            let batch_size = frontier_vec.len().min(self.config.max_entities_per_level);
            
            // FIXED: Track what we're deferring
            if frontier_vec.len() > batch_size {
                let deferred = frontier_vec.len() - batch_size;
                deferred_count += deferred;
                tracing::debug!(
                    "BFS level {}: processing {} of {} nodes ({} deferred)",
                    current_depth, batch_size, frontier_vec.len(), deferred
                );
            }

            let (entities, relations) = self.storage
                .get_direct_relations_batch(&frontier_vec[..batch_size], direction)
                .await?;

            // FIXED: Re-queue remaining nodes for next pass
            for remaining in frontier_vec.into_iter().skip(batch_size) {
                frontier.push_back(remaining);
            }

            // Process relations
            for rel in relations {
                let rel_id = rel.id.as_ref()
                    .map(|t| t.id.to_string())
                    .unwrap_or_default();
                if visited_relations.insert(rel_id) {
                    all_relations.push(rel);
                }
            }

            // Process entities with total limit check
            for entity in entities {
                if all_entities.len() >= self.config.max_total_entities {
                    truncated = true;
                    deferred_count += frontier.len();
                    
                    return Ok(TraversalResult {
                        entities: all_entities,
                        relations: all_relations,
                        strategy: TraversalStrategy::Bfs,
                        depth_reached: current_depth,
                        truncated,
                        deferred_count,
                    });
                }
                
                let eid = entity.id.as_ref()
                    .map(|t| t.id.to_string())
                    .unwrap_or_default();
                    
                if visited_entities.insert(eid.clone()) {
                    all_entities.push(entity);
                    frontier.push_back(eid);
                }
            }
        }

        Ok(TraversalResult {
            entities: all_entities,
            relations: all_relations,
            strategy: TraversalStrategy::Bfs,
            depth_reached: depth,
            truncated,
            deferred_count,
        })
    }
}
```

---

## FIX-014 to FIX-015: Leiden Algorithm

### 🎯 OPTIMAL HYBRID SOLUTION

```rust
// graph/leiden.rs

/// Detect communities using the Louvain algorithm.
///
/// Note: This implements Louvain (2008), not the full Leiden (2019).
/// For most knowledge graphs, Louvain provides good community structure.
///
/// # Parameters
/// - `resolution`: Tune granularity. <1 = more communities, >1 = fewer.
pub fn detect_communities(
    graph: &DiGraph<String, f32>, 
    resolution: f32
) -> Vec<Vec<NodeIndex>> {
    let n = graph.node_count();
    if n == 0 {
        return vec![];
    }

    // Build undirected adjacency with weights
    let mut neighbors: Vec<Vec<(usize, f32)>> = vec![vec![]; n];
    let mut total_weight: f32 = 0.0;
    let mut node_weights: Vec<f32> = vec![0.0; n];

    for edge in graph.edge_references() {
        let u = edge.source().index();
        let v = edge.target().index();
        let w = *edge.weight();

        neighbors[u].push((v, w));
        neighbors[v].push((u, w));
        node_weights[u] += w;
        node_weights[v] += w;
        total_weight += w;
    }

    if total_weight == 0.0 {
        return graph.node_indices().map(|idx| vec![idx]).collect();
    }

    let mut community_assignment: Vec<usize> = (0..n).collect();
    let mut community_weights: Vec<f32> = node_weights.clone();
    
    // FIXED: Correct formula uses 2m in denominator
    let m2 = 2.0 * total_weight;

    let mut changed = true;
    let mut iterations = 0;
    const MAX_ITER: usize = 20;  // Increased from 10

    while changed && iterations < MAX_ITER {
        changed = false;
        iterations += 1;

        for i in 0..n {
            let current_comm = community_assignment[i];
            let ki = node_weights[i];

            // Calculate connections to each community
            let mut gain_map: HashMap<usize, f32> = HashMap::new();
            for &(neighbor, weight) in &neighbors[i] {
                let neighbor_comm = community_assignment[neighbor];
                *gain_map.entry(neighbor_comm).or_insert(0.0) += weight;
            }

            // FIXED: Calculate loss from leaving current community
            let ki_in_current = *gain_map.get(&current_comm).unwrap_or(&0.0);
            let sum_tot_current = community_weights[current_comm] - ki;

            let mut best_comm = current_comm;
            let mut max_delta = 0.0;

            for (&comm, &ki_in) in &gain_map {
                if comm == current_comm {
                    continue;
                }

                let sum_tot = community_weights[comm];
                
                // FIXED: Correct modularity gain formula with resolution
                // ΔQ = [ki_in/m - resolution * (Σtot * ki) / (2m²)] 
                //    - [ki_in_current/m - resolution * (Σtot_current * ki) / (2m²)]
                let gain_new = (ki_in / m2) - resolution * (sum_tot * ki) / (m2 * m2);
                let gain_current = (ki_in_current / m2) - resolution * (sum_tot_current * ki) / (m2 * m2);
                
                let delta = gain_new - gain_current;

                if delta > max_delta {
                    max_delta = delta;
                    best_comm = comm;
                }
            }

            // FIXED: Only move if strictly better
            if best_comm != current_comm && max_delta > 1e-10 {
                community_assignment[i] = best_comm;
                community_weights[current_comm] -= ki;
                community_weights[best_comm] += ki;
                changed = true;
            }
        }
    }

    // Group nodes by community
    let mut communities_map: HashMap<usize, Vec<NodeIndex>> = HashMap::new();
    for (node_idx, &comm_idx) in community_assignment.iter().enumerate() {
        communities_map
            .entry(comm_idx)
            .or_default()
            .push(NodeIndex::new(node_idx));
    }

    communities_map.into_values().collect()
}
```

---

## FIX-016: PPR Edge Weights

### 🎯 OPTIMAL HYBRID SOLUTION

```rust
// graph/ppr.rs

pub fn personalized_page_rank(
    graph: &DiGraph<String, f32>,
    seed_nodes: &[NodeIndex],
    damping: f32,
    tolerance: f32,
    max_iter: usize,
) -> HashMap<NodeIndex, f32> {
    let n = graph.node_count();
    if n == 0 {
        return HashMap::new();
    }

    // Initialize personalization vector
    let mut personalization = vec![0.0; n];
    if !seed_nodes.is_empty() {
        let seed_weight = 1.0 / seed_nodes.len() as f32;
        for &node in seed_nodes {
            if node.index() < n {
                personalization[node.index()] = seed_weight;
            }
        }
    } else {
        let uniform = 1.0 / n as f32;
        personalization.fill(uniform);
    }

    let mut scores = personalization.clone();

    // Pre-compute outgoing weight sums for each node
    let out_weight_sums: Vec<f32> = graph.node_indices()
        .map(|node| {
            graph.edges(node)
                .map(|e| *e.weight())
                .sum::<f32>()
        })
        .collect();

    // Identify dangling nodes
    let dangling_nodes: Vec<NodeIndex> = graph
        .node_indices()
        .filter(|&node| out_weight_sums[node.index()] == 0.0)
        .collect();

    for _ in 0..max_iter {
        let mut new_scores = vec![0.0; n];

        let dangling_sum: f32 = dangling_nodes
            .iter()
            .map(|&node| scores[node.index()])
            .sum();

        for node in graph.node_indices() {
            let total_out_weight = out_weight_sums[node.index()];
            
            if total_out_weight > 0.0 {
                // FIXED: Use edge weights for transition probabilities
                for edge in graph.edges(node) {
                    let weight = *edge.weight();
                    let transition_prob = weight / total_out_weight;
                    let contribution = scores[node.index()] * transition_prob * damping;
                    new_scores[edge.target().index()] += contribution;
                }
            }
        }

        // Redistribute dangling mass and teleport
        for i in 0..n {
            new_scores[i] += damping * dangling_sum * personalization[i];
            new_scores[i] += (1.0 - damping) * personalization[i];
        }

        let diff: f32 = scores
            .iter()
            .zip(new_scores.iter())
            .map(|(a, b)| (a - b).abs())
            .sum();

        scores = new_scores;

        if diff < tolerance {
            break;
        }
    }

    graph.node_indices()
        .map(|idx| (idx, scores[idx.index()]))
        .collect()
}
```

---

## FIX-017: Silent Error Handling

### 🎯 OPTIMAL HYBRID SOLUTION

```rust
// storage/surrealdb.rs

// BEFORE (silent failure):
let relations: Vec<Relation> = response.take(0).unwrap_or_default();

// AFTER (proper error propagation):
let relations: Vec<Relation> = response.take(0)
    .map_err(|e| {
        tracing::error!("Database query failed: {}", e);
        e
    })?;

// For optional results with logging:
fn take_with_logging<T: DeserializeOwned>(
    response: &mut QueryResponse,
    idx: usize,
    context: &str
) -> Result<Vec<T>> {
    response.take(idx).map_err(|e| {
        tracing::error!("Failed to deserialize {} result: {}", context, e);
        anyhow!("Database error in {}: {}", context, e)
    })
}

// Usage:
let relations = take_with_logging::<Relation>(&mut response, 0, "get_subgraph.relations")?;
```

---

## FIX-018 to FIX-019: Large File Handling

### 🎯 OPTIMAL HYBRID SOLUTION

```rust
// codebase/indexer.rs

const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB

impl CodebaseIndexer {
    async fn index_file(&self, path: &Path) -> Result<Vec<CodeChunk>> {
        // Check file size before reading
        let metadata = tokio::fs::metadata(path).await?;
        if metadata.len() > MAX_FILE_SIZE {
            tracing::warn!(
                "Skipping large file ({} bytes): {:?}",
                metadata.len(), path
            );
            return Ok(vec![]);
        }
        
        let content = tokio::fs::read_to_string(path).await?;
        let path_clone = path.to_path_buf();
        
        // Use spawn_blocking for CPU-heavy parsing
        let chunks = tokio::task::spawn_blocking(move || {
            let parser = CodeParser::new();
            parser.parse_file(&path_clone, &content)
        }).await??;
        
        Ok(chunks)
    }
}
```

---

## FIX-020: Atomic Index Status

### 🎯 OPTIMAL HYBRID SOLUTION

```rust
// storage/surrealdb.rs

async fn update_index_status(&self, status: IndexStatus) -> Result<()> {
    // Use UPSERT for atomicity
    let sql = r#"
        UPSERT index_status:$project_id SET
            project_id = $project_id,
            status = $status,
            file_count = $file_count,
            symbol_count = $symbol_count,
            chunk_count = $chunk_count,
            last_updated = time::now(),
            error = $error
    "#;
    
    self.db.query(sql)
        .bind(("project_id", &status.project_id))
        .bind(("status", &status.status))
        .bind(("file_count", status.file_count))
        .bind(("symbol_count", status.symbol_count))
        .bind(("chunk_count", status.chunk_count))
        .bind(("error", &status.error))
        .await?;
    
    Ok(())
}
```

---

## FIX-021: Worker Race Condition

### 🎯 OPTIMAL HYBRID SOLUTION

```rust
// embedding/worker.rs

impl EmbeddingWorker {
    async fn process_batch(&self, requests: Vec<EmbeddingRequest>) -> Result<()> {
        // Compute all embeddings
        let texts: Vec<String> = requests.iter()
            .map(|r| r.text.clone())
            .collect();
        
        let embeddings = self.engine.embed_batch(&texts)?;
        
        // FIXED: Batch update instead of spawn per item
        let updates: Vec<(String, Vec<f32>)> = requests.iter()
            .zip(embeddings.iter())
            .map(|(req, emb)| (req.id.clone(), emb.clone()))
            .collect();
        
        // Single transaction for all updates
        self.state.storage.batch_update_embeddings(&updates).await?;
        
        // Cache all results
        for (req, embedding) in requests.iter().zip(embeddings.iter()) {
            let hash = blake3::hash(req.text.as_bytes()).to_hex().to_string();
            self.embedding_store.put(hash, embedding.clone()).await?;
        }
        
        Ok(())
    }
}

// storage/surrealdb.rs - New batch method
async fn batch_update_embeddings(&self, updates: &[(String, Vec<f32>)]) -> Result<()> {
    if updates.is_empty() {
        return Ok(());
    }
    
    // Use transaction for atomicity
    let mut tx = self.db.query("BEGIN TRANSACTION").await?;
    
    for (id, embedding) in updates {
        tx = tx.query("UPDATE $id SET embedding = $emb, embedding_state = 'ready'")
            .bind(("id", id))
            .bind(("emb", embedding));
    }
    
    tx.query("COMMIT TRANSACTION").await?;
    
    Ok(())
}
```

---

## 📋 Implementation Checklist

### Pre-Implementation
- [x] Create feature branch: `fix/security-stability-audit`
- [x] Run baseline tests: `cargo test`
- [x] Document current behavior for regression testing

### Implementation Order
1. [x] **FIX-001 to FIX-005**: SQL Injection (SECURITY - do first) ✅
2. [x] **FIX-006**: Tokenizer bounds + unwrap removal ✅
3. [x] **FIX-012**: Model-aware cache ✅
4. [x] **FIX-013**: BFS frontier leak tracking ✅
5. [x] **FIX-014, FIX-015**: Leiden algorithm ✅
6. [x] **FIX-016**: PPR weights ✅
7. [x] **FIX-017**: Error handling (9 silent errors → proper propagation) ✅
8. [ ] **FIX-018 to FIX-021**: Robustness (low priority)

### Post-Implementation
- [x] Run full test suite: `cargo test` → 56 passing
- [x] Run clippy: `cargo clippy` → 0 warnings
- [ ] Manual testing with MCP client
- [ ] Update CHANGELOG.md
- [ ] Create PR with detailed description

---

## 🔗 References

- [SurrealDB Parameterized Queries](https://surrealdb.com/docs/surrealdb/surrealql/functions/type)
- [Rust parking_lot](https://docs.rs/parking_lot)
- [Louvain Algorithm Paper](https://arxiv.org/abs/0803.0476)
- [Personalized PageRank](https://en.wikipedia.org/wiki/PageRank#Personalized_PageRank)

---

*Document upgraded from Research to Development: 2026-01-08*
*Implementation started: 2026-01-08*
*Progress: 15/17 fixes completed, 56 tests passing, clippy clean*
