---
work_package_id: WP07
title: "Graph Tools + PPR"
phase: "Phase 6"
priority: P2
subtasks: ["T046", "T047", "T048", "T049", "T050", "T051"]
lane: planned
dependencies: ["WP06"]
history:
  - date: 2026-01-06
    action: created
    by: spec-kitty.tasks
---

# WP07: Graph Tools + PPR

## Objective

Implement knowledge graph operations and Personalized PageRank for graph-aware ranking in recall.

## Context

The knowledge graph enables structured relationships between concepts. PPR provides graph-based relevance scoring that enhances hybrid search.

**Reference**:
- `kitty-specs/001-memory-mcp-server/research.md` - PPR algorithm (HippoRAG damping=0.5)

## Subtasks

### T046: Create graph/ppr.rs

**Location**: `src/graph/ppr.rs`

Implement Personalized PageRank using petgraph:

```rust
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

/// Personalized PageRank for graph-aware ranking
/// 
/// Parameters (HippoRAG-inspired):
/// - damping = 0.5 (higher teleport probability for better associative recall)
/// - tolerance = 1e-6 (convergence threshold)
/// - max_iter = 15 (sufficient for convergence)
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
    
    // Initialize personalization vector (uniform over seeds)
    let mut personalization = vec![0.0; n];
    if !seed_nodes.is_empty() {
        let seed_weight = 1.0 / seed_nodes.len() as f32;
        for &node in seed_nodes {
            personalization[node.index()] = seed_weight;
        }
    } else {
        // No seeds = uniform distribution
        let uniform = 1.0 / n as f32;
        personalization.fill(uniform);
    }
    
    // Initialize scores
    let mut scores = personalization.clone();
    
    // Power iteration
    for _ in 0..max_iter {
        let mut new_scores = vec![0.0; n];
        
        // Distribute scores along edges
        for node in graph.node_indices() {
            let out_degree = graph.edges(node).count();
            if out_degree > 0 {
                let share = scores[node.index()] / out_degree as f32;
                for edge in graph.edges(node) {
                    new_scores[edge.target().index()] += share * damping;
                }
            }
        }
        
        // Add teleport probability
        for i in 0..n {
            new_scores[i] += (1.0 - damping) * personalization[i];
        }
        
        // Check convergence
        let diff: f32 = scores.iter()
            .zip(new_scores.iter())
            .map(|(a, b)| (a - b).abs())
            .sum();
        
        scores = new_scores;
        
        if diff < tolerance {
            break;
        }
    }
    
    // Convert to HashMap
    graph.node_indices()
        .map(|idx| (idx, scores[idx.index()]))
        .collect()
}

/// Hub dampening: reduce score for highly connected nodes
/// This prevents "hub" nodes from dominating results
pub fn apply_hub_dampening(
    scores: &mut HashMap<NodeIndex, f32>,
    degrees: &HashMap<NodeIndex, usize>,
) {
    for (node, score) in scores.iter_mut() {
        if let Some(&degree) = degrees.get(node) {
            if degree > 0 {
                *score /= (degree as f32).sqrt();
            }
        }
    }
}
```

---

### T047: Implement tool: create_entity

```rust
    /// Create a knowledge graph entity. Returns the entity ID.
    #[tool(description = "Create a knowledge graph entity. Returns the entity ID.")]
    async fn create_entity(
        &self,
        /// Entity name
        name: String,
        /// Type: person, project, concept, file, etc.
        entity_type: Option<String>,
        /// Optional description
        description: Option<String>,
        /// Optional user ID for isolation
        user_id: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        // Optionally embed the name for semantic entity search
        let embedding = if self.state.embedding.status() == EmbeddingStatus::Ready {
            Some(self.state.embedding.embed(&name).await
                .map_err(|e| McpError::internal(e.to_string()))?)
        } else {
            None
        };
        
        let entity = Entity {
            id: None,
            name,
            entity_type: entity_type.unwrap_or_else(|| "unknown".to_string()),
            description,
            embedding,
            user_id,
            created_at: chrono::Utc::now(),
        };
        
        let id = self.state.storage.create_entity(entity).await
            .map_err(|e| McpError::internal(e.to_string()))?;
        
        Ok(CallToolResult::success(serde_json::json!({ "id": id })))
    }
```

---

### T048: Implement tool: create_relation

```rust
    /// Create a relation between two entities.
    #[tool(description = "Create a relation between two entities.")]
    async fn create_relation(
        &self,
        /// Source entity ID
        from_entity: String,
        /// Target entity ID
        to_entity: String,
        /// Relation type: works_on, knows, uses, etc.
        relation_type: String,
        /// Relation weight (0.0-1.0, default: 1.0)
        weight: Option<f32>,
    ) -> Result<CallToolResult, McpError> {
        let relation = Relation {
            id: None,
            from_entity: from_entity.parse().map_err(|_| McpError::invalid_params("Invalid from_entity ID"))?,
            to_entity: to_entity.parse().map_err(|_| McpError::invalid_params("Invalid to_entity ID"))?,
            relation_type,
            weight: weight.unwrap_or(1.0).clamp(0.0, 1.0),
            valid_from: chrono::Utc::now(),
            valid_until: None,
        };
        
        let id = self.state.storage.create_relation(relation).await
            .map_err(|e| McpError::internal(e.to_string()))?;
        
        Ok(CallToolResult::success(serde_json::json!({ "id": id })))
    }
```

---

### T049: Implement tool: get_related

```rust
    /// Get entities related to a given entity via graph traversal.
    #[tool(description = "Get entities related to a given entity via graph traversal.")]
    async fn get_related(
        &self,
        /// Entity ID to start from
        entity_id: String,
        /// Traversal depth (1-3, default: 1)
        depth: Option<usize>,
        /// Traversal direction: outgoing, incoming, both
        direction: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        let depth = depth.unwrap_or(1).min(3);
        let direction: Direction = direction
            .map(|s| s.parse().unwrap_or_default())
            .unwrap_or_default();
        
        let (entities, relations) = self.state.storage
            .get_related(&entity_id, depth, direction)
            .await
            .map_err(|e| McpError::internal(e.to_string()))?;
        
        Ok(CallToolResult::success(serde_json::json!({
            "entities": entities,
            "relations": relations,
            "entity_count": entities.len(),
            "relation_count": relations.len()
        })))
    }
```

---

### T050: Update recall tool to use real PPR scores

Modify the `recall` tool in `server/handler.rs`:

```rust
// In recall tool, after RRF merge:

// Build graph from entities related to search results
let result_ids: Vec<String> = merged.iter().map(|(id, _)| id.clone()).collect();
let (entities, relations) = self.state.storage.get_subgraph(&result_ids).await
    .map_err(|e| McpError::internal(e.to_string()))?;

// Build petgraph
let mut graph = DiGraph::<String, f32>::new();
let mut node_map: HashMap<String, NodeIndex> = HashMap::new();

for entity in &entities {
    if let Some(ref id) = entity.id {
        let idx = graph.add_node(id.to_string());
        node_map.insert(id.to_string(), idx);
    }
}

for relation in &relations {
    if let (Some(from_idx), Some(to_idx)) = (
        node_map.get(&relation.from_entity.to_string()),
        node_map.get(&relation.to_entity.to_string()),
    ) {
        graph.add_edge(*from_idx, *to_idx, relation.weight);
    }
}

// Compute PPR with RRF seeds
let seed_nodes: Vec<NodeIndex> = merged.iter()
    .take(20)
    .filter_map(|(id, _)| node_map.get(id).copied())
    .collect();

use crate::graph::ppr::{personalized_page_rank, apply_hub_dampening};
let mut ppr_scores = personalized_page_rank(&graph, &seed_nodes, 0.5, 1e-6, 15);

// Apply hub dampening
let degrees = self.state.storage.get_node_degrees(&result_ids).await
    .map_err(|e| McpError::internal(e.to_string()))?;
let degree_map: HashMap<NodeIndex, usize> = degrees.iter()
    .filter_map(|(id, deg)| node_map.get(id).map(|idx| (*idx, *deg)))
    .collect();
apply_hub_dampening(&mut ppr_scores, &degree_map);

// Use PPR scores in final calculation
for memory in &mut scored_memories {
    if let Some(&idx) = node_map.get(&memory.id) {
        memory.ppr_score = ppr_scores.get(&idx).copied().unwrap_or(0.0);
    }
    memory.score = w_vec * memory.vector_score + w_bm25 * memory.bm25_score + w_ppr * memory.ppr_score;
}
```

---

### T051: Implement hub dampening

Already included in T046 (`apply_hub_dampening` function).

Ensure it's called in the recall flow to prevent highly-connected nodes from dominating results.

---

## Definition of Done

1. create_entity and create_relation store graph data
2. get_related traverses up to depth 3
3. PPR converges within 15 iterations
4. Hub dampening reduces hub node scores
5. recall uses real PPR scores (not placeholder 0)
6. Default weights: vector=0.40, bm25=0.15, ppr=0.45

## Risks

| Risk | Mitigation |
|------|------------|
| PPR convergence issues | Use tolerance check, cap iterations at 15 |
| Graph size explosion | Limit depth to 3, cap subgraph size |

## Reviewer Guidance

- Verify damping = 0.5 (HippoRAG recommendation)
- Check hub dampening formula: score / sqrt(degree)
- Confirm seed nodes come from RRF top-20
- Test with small graph first (3-5 entities)
