# Graph Traversal — Design Document

## Overview

Graph algorithms for knowledge graph traversal and ranking. Uses `petgraph` for in-memory graph operations.

## Personalized PageRank (PPR)

Core algorithm for graph-aware ranking in `recall` tool.

### Why PPR?

- **SOTA quality**: HippoRAG-level performance
- **Adaptive depth**: Damping factor controls spread (no fixed hop limit)
- **Hub dampening**: Prevents over-weighted central nodes
- **Lightweight**: ~50 lines, petgraph only

### Implementation

```rust
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

/// Personalized PageRank (PPR) — HippoRAG style
/// 
/// * `personalization`: seed nodes weighted by vector similarity
/// * `damping`: 0.5 (HippoRAG recommendation, NOT 0.85)
/// * `tolerance`: 1e-6 for convergence
pub fn personalized_page_rank<N, E>(
    graph: &DiGraph<N, E>,
    personalization: &HashMap<NodeIndex, f64>,
    damping: f64,
    tolerance: f64,
) -> HashMap<NodeIndex, f64> {
    let num_nodes = graph.node_count();
    if num_nodes == 0 { return HashMap::new(); }

    // Normalize personalization (teleport probability)
    let total: f64 = personalization.values().sum();
    let teleport: HashMap<_, _> = if total > 0.0 {
        personalization.iter().map(|(&n, &w)| (n, w / total)).collect()
    } else {
        graph.node_indices().map(|n| (n, 1.0 / num_nodes as f64)).collect()
    };

    // Initialize ranks uniformly
    let mut ranks: HashMap<NodeIndex, f64> = graph.node_indices()
        .map(|n| (n, 1.0 / num_nodes as f64))
        .collect();

    // Power iteration (max 15 iterations)
    for _ in 0..15 {
        let mut new_ranks = HashMap::with_capacity(num_nodes);
        let mut diff = 0.0;

        for node in graph.node_indices() {
            let incoming_sum: f64 = graph
                .neighbors_directed(node, petgraph::Direction::Incoming)
                .map(|neighbor| {
                    let out_degree = graph.neighbors(neighbor).count() as f64;
                    if out_degree > 0.0 { ranks[&neighbor] / out_degree } else { 0.0 }
                })
                .sum();

            let teleport_prob = *teleport.get(&node).unwrap_or(&0.0);
            let new_rank = (1.0 - damping) * teleport_prob + damping * incoming_sum;
            
            diff += (new_rank - ranks[&node]).abs();
            new_ranks.insert(node, new_rank);
        }

        ranks = new_ranks;
        if diff < tolerance { break; }
    }

    ranks
}
```

### Parameters

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| damping | 0.5 | HippoRAG recommendation for associative memory |
| tolerance | 1e-6 | Standard convergence threshold |
| max_iter | 15 | Sufficient for convergence |

### Damping Factor Comparison

| Value | Use Case |
|-------|----------|
| 0.85 | Classic PageRank (web pages) |
| 0.50 | HippoRAG (associative memory) |

α = 0.5 allows more "teleportation" to seed nodes, keeping results closer to the query.

## Graph Building

Convert SurrealDB relations to petgraph:

```rust
pub struct Subgraph {
    pub nodes: Vec<Entity>,
    pub edges: Vec<Relation>,
}

impl Subgraph {
    pub fn to_petgraph(&self) -> DiGraph<String, f32> {
        let mut graph = DiGraph::new();
        let mut node_indices: HashMap<String, NodeIndex> = HashMap::new();
        
        // Add nodes
        for node in &self.nodes {
            let idx = graph.add_node(node.id.clone());
            node_indices.insert(node.id.clone(), idx);
        }
        
        // Add edges
        for edge in &self.edges {
            if let (Some(&from), Some(&to)) = (
                node_indices.get(&edge.from_id),
                node_indices.get(&edge.to_id),
            ) {
                graph.add_edge(from, to, edge.weight);
            }
        }
        
        graph
    }
}
```

## SurrealDB Graph Queries

### Get Related Entities

```sql
-- Single hop
SELECT ->relations->entities AS related
FROM entities:$id;

-- Multi-hop (2 levels)
SELECT ->relations->entities->relations->entities AS related_2hop
FROM entities:$id;

-- With depth parameter
SELECT ->relations.{1..$depth}->entities AS related
FROM entities:$id;

-- Bi-directional
SELECT 
    ->relations->entities AS outgoing,
    <-relations<-entities AS incoming
FROM entities:$id;
```

### Get Subgraph for Seeds

```sql
LET $seed_ids = $seeds.*.id;

-- Get nodes (seeds + 1-hop neighbors)
LET $nodes = (
    SELECT * FROM entities 
    WHERE id IN $seed_ids
    OR id IN (SELECT VALUE ->relations->entities.id FROM entities WHERE id IN $seed_ids)
    OR id IN (SELECT VALUE <-relations<-entities.id FROM entities WHERE id IN $seed_ids)
);

-- Get edges between these nodes
LET $node_ids = $nodes.*.id;
LET $edges = (
    SELECT * FROM relations 
    WHERE in IN $node_ids AND out IN $node_ids
);

RETURN { nodes: $nodes, edges: $edges };
```

### Create Relation

```sql
RELATE entities:$from->relations->entities:$to SET 
    relation_type = $type,
    weight = $weight,
    valid_from = time::now();
```

## Traversal Patterns

### Pattern: Anchor + Expand

```rust
async fn get_related(
    &self,
    entity_id: &str,
    depth: u32,
    direction: Direction,
) -> Result<Vec<Entity>> {
    let query = match direction {
        Direction::Outgoing => format!(
            "SELECT ->relations.{{1..{}}}.->entities.* AS related FROM entities:{}",
            depth, entity_id
        ),
        Direction::Incoming => format!(
            "SELECT <-relations.{{1..{}}}.<-entities.* AS related FROM entities:{}",
            depth, entity_id
        ),
        Direction::Both => format!(
            "SELECT 
                ->relations.{{1..{depth}}}.->entities.* AS outgoing,
                <-relations.{{1..{depth}}}.<-entities.* AS incoming
            FROM entities:{id}",
            depth = depth, id = entity_id
        ),
    };
    
    self.db.query(query).await
}
```

### Pattern: Path Finding

```rust
async fn find_path(
    &self,
    from_id: &str,
    to_id: &str,
    max_depth: u32,
) -> Result<Option<Vec<String>>> {
    // Build subgraph around both endpoints
    let subgraph = self.get_subgraph_between(from_id, to_id, max_depth).await?;
    let graph = subgraph.to_petgraph();
    
    // Use petgraph's pathfinding
    let from_idx = graph.node_indices().find(|&i| graph[i] == from_id);
    let to_idx = graph.node_indices().find(|&i| graph[i] == to_id);
    
    match (from_idx, to_idx) {
        (Some(from), Some(to)) => {
            let path = petgraph::algo::astar(
                &graph,
                from,
                |n| n == to,
                |_| 1,  // uniform edge weight
                |_| 0,  // no heuristic
            );
            Ok(path.map(|(_, nodes)| {
                nodes.iter().map(|&n| graph[n].clone()).collect()
            }))
        }
        _ => Ok(None),
    }
}
```

## Community Detection (Future)

Leiden algorithm via `fa-leiden-cd` crate:

```rust
use fa_leiden_cd::leiden;

pub fn detect_communities(
    graph: &DiGraph<String, f32>,
    resolution: f64,
) -> HashMap<NodeIndex, usize> {
    // Convert to adjacency list format
    let edges: Vec<(usize, usize)> = graph.edge_indices()
        .map(|e| {
            let (from, to) = graph.edge_endpoints(e).unwrap();
            (from.index(), to.index())
        })
        .collect();
    
    let communities = leiden(&edges, resolution);
    
    graph.node_indices()
        .enumerate()
        .map(|(i, n)| (n, communities[i]))
        .collect()
}
```

## Performance

| Operation | Complexity | Latency (100 nodes) |
|-----------|------------|---------------------|
| PPR iteration | O(E) | ~0.1ms |
| Full PPR (15 iter) | O(15E) | ~2ms |
| Subgraph extraction | O(V + E) | ~5ms |
| Path finding (A*) | O(E log V) | ~1ms |
