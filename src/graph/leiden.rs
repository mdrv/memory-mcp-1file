use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::HashMap;

/// Detect communities using a simplified Leiden-like algorithm.
///
/// For simplicity, this implementation performs local modularity maximization
/// (Louvain-like) with multiple passes to refine communities.
pub fn detect_communities(graph: &DiGraph<String, f32>) -> Vec<Vec<NodeIndex>> {
    let n = graph.node_count();
    if n == 0 {
        return vec![];
    }

    // Convert directed graph to an undirected adjacency list for modularity calculation
    let mut neighbors: Vec<Vec<(usize, f32)>> = vec![vec![]; n];
    let mut total_weight: f32 = 0.0;
    let mut node_weights: Vec<f32> = vec![0.0; n];

    for edge in graph.edge_references() {
        let u = edge.source().index();
        let v = edge.target().index();
        let w = edge.weight();

        neighbors[u].push((v, *w));
        neighbors[v].push((u, *w));
        node_weights[u] += w;
        node_weights[v] += w;
        total_weight += w;
    }

    if total_weight == 0.0 {
        // No edges, each node is its own community
        return graph.node_indices().map(|idx| vec![idx]).collect();
    }

    // Initial partition: each node in its own community
    let mut community_assignment: Vec<usize> = (0..n).collect();
    let mut community_weights: Vec<f32> = node_weights.clone();
    let _community_internal_weights: Vec<f32> = vec![0.0; n];

    let m2 = total_weight; // total weight m (already doubled by counting each edge once as u-v and v-u if undirected, but here we summed all weights)
                           // Actually total_weight is sum of all edges. In undirected modularity formula it's often 2m.
                           // If we count each edge once, sum of node weights is 2m.

    let mut changed = true;
    let mut iterations = 0;
    const MAX_ITER: usize = 10;

    while changed && iterations < MAX_ITER {
        changed = false;
        iterations += 1;

        for i in 0..n {
            let current_comm = community_assignment[i];
            let ki = node_weights[i];

            // Calculate gain for each neighboring community
            let mut gain_map: HashMap<usize, f32> = HashMap::new();
            for &(neighbor, weight) in &neighbors[i] {
                let neighbor_comm = community_assignment[neighbor];
                *gain_map.entry(neighbor_comm).or_insert(0.0) += weight;
            }

            let mut best_comm = current_comm;
            let mut max_gain = 0.0;

            // Remove node i from its current community for calculation
            let _ki_in_current = *gain_map.get(&current_comm).unwrap_or(&0.0);

            for (&comm, &ki_in) in &gain_map {
                if comm == current_comm {
                    continue;
                }

                // Simplified modularity gain formula:
                // delta_Q = (ki_in / m) - (sum_tot * ki / 2m^2)
                let sum_tot = community_weights[comm];
                let gain = ki_in - (sum_tot * ki) / m2;

                if gain > max_gain {
                    max_gain = gain;
                    best_comm = comm;
                }
            }

            if best_comm != current_comm && max_gain > 0.0 {
                // Move node i to best_comm
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
