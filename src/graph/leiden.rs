use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::HashMap;

/// Detect communities using the Louvain algorithm for modularity maximization.
///
/// This implementation performs local modularity optimization with iterative refinement.
/// For most knowledge graphs, this provides good community structure detection.
///
/// # Parameters
/// - `graph`: Directed graph with String node weights and f32 edge weights
///
/// # Returns
/// Vector of communities, where each community is a vector of node indices
pub fn detect_communities(graph: &DiGraph<String, f32>) -> Vec<Vec<NodeIndex>> {
    detect_communities_with_resolution(graph, 1.0)
}

/// Detect communities with configurable resolution parameter.
///
/// # Parameters  
/// - `resolution`: Tune granularity. <1 = more communities, >1 = fewer.
pub fn detect_communities_with_resolution(
    graph: &DiGraph<String, f32>,
    resolution: f32,
) -> Vec<Vec<NodeIndex>> {
    let n = graph.node_count();
    if n == 0 {
        return vec![];
    }

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

    let m2 = 2.0 * total_weight;

    let mut changed = true;
    let mut iterations = 0;
    const MAX_ITER: usize = 20;

    while changed && iterations < MAX_ITER {
        changed = false;
        iterations += 1;

        for i in 0..n {
            let current_comm = community_assignment[i];
            let ki = node_weights[i];

            let mut gain_map: HashMap<usize, f32> = HashMap::new();
            for &(neighbor, weight) in &neighbors[i] {
                let neighbor_comm = community_assignment[neighbor];
                *gain_map.entry(neighbor_comm).or_insert(0.0) += weight;
            }

            let ki_in_current = *gain_map.get(&current_comm).unwrap_or(&0.0);
            let sum_tot_current = community_weights[current_comm] - ki;

            let mut best_comm = current_comm;
            let mut max_delta: f32 = 0.0;

            for (&comm, &ki_in) in &gain_map {
                if comm == current_comm {
                    continue;
                }

                let sum_tot = community_weights[comm];

                let gain_new = (ki_in / m2) - resolution * (sum_tot * ki) / (m2 * m2);
                let gain_current =
                    (ki_in_current / m2) - resolution * (sum_tot_current * ki) / (m2 * m2);

                let delta = gain_new - gain_current;

                if delta > max_delta {
                    max_delta = delta;
                    best_comm = comm;
                }
            }

            if best_comm != current_comm && max_delta > 1e-10 {
                community_assignment[i] = best_comm;
                community_weights[current_comm] -= ki;
                community_weights[best_comm] += ki;
                changed = true;
            }
        }
    }

    let mut communities_map: HashMap<usize, Vec<NodeIndex>> = HashMap::new();
    for (node_idx, &comm_idx) in community_assignment.iter().enumerate() {
        communities_map
            .entry(comm_idx)
            .or_default()
            .push(NodeIndex::new(node_idx));
    }

    communities_map.into_values().collect()
}
