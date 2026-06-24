//! Graph algorithms for branchial graphs via petgraph and rustworkx-core.
//!
//! Provides a [`BranchialGraph::to_petgraph`] conversion and thin wrappers
//! around rustworkx-core algorithms for coloring, k-core decomposition,
//! and articulation point detection.

use std::collections::HashMap;

use petgraph::graph::{NodeIndex, UnGraph};

use super::branchial::BranchialGraph;
use super::evolution_graph::MultiwayNodeId;

impl BranchialGraph {
    /// Convert to a petgraph undirected graph for algorithm application.
    ///
    /// Nodes carry [`MultiwayNodeId`], edges are unweighted.
    /// Returns `(graph, index_map)` where `index_map[i]` is the
    /// [`NodeIndex`] for `self.nodes[i]`.
    #[must_use]
    pub fn to_petgraph(&self) -> (UnGraph<MultiwayNodeId, ()>, Vec<NodeIndex>) {
        let mut pg = UnGraph::new_undirected();
        let mut node_map: HashMap<MultiwayNodeId, NodeIndex> = HashMap::new();

        // Add nodes
        let idx_map: Vec<NodeIndex> = self
            .nodes
            .iter()
            .map(|&id| {
                let idx = pg.add_node(id);
                node_map.insert(id, idx);
                idx
            })
            .collect();

        // Add edges
        for &(a, b) in &self.edges {
            if let (Some(&ia), Some(&ib)) = (node_map.get(&a), node_map.get(&b)) {
                pg.add_edge(ia, ib, ());
            }
        }

        (pg, idx_map)
    }
}

/// Graph coloring of a branchial graph.
///
/// Returns a map from [`MultiwayNodeId`] to color index (0-based).
/// Uses rustworkx-core greedy coloring — the number of colors used
/// is an upper bound on the chromatic number. For branchial graphs,
/// this measures the minimum "dimensions of branching" needed to
/// separate all causally-related branches.
#[must_use]
pub fn branchial_coloring(graph: &BranchialGraph) -> HashMap<MultiwayNodeId, usize> {
    let (pg, _) = graph.to_petgraph();
    let color_map = rustworkx_core::coloring::greedy_node_color(&pg);

    let mut result = HashMap::new();
    for (i, &node_id) in graph.nodes.iter().enumerate() {
        if let Some(&color) = color_map.get(&NodeIndex::new(i)) {
            result.insert(node_id, color);
        }
    }
    result
}

/// K-core decomposition of a branchial graph.
///
/// Returns a map from [`MultiwayNodeId`] to its core number.
/// The k-core is the maximal subgraph where every vertex has degree ≥ k.
/// High core numbers in branchial graphs indicate regions of dense
/// computational interaction between branches.
#[must_use]
pub fn branchial_core_numbers(graph: &BranchialGraph) -> HashMap<MultiwayNodeId, usize> {
    let (pg, _) = graph.to_petgraph();
    let cores = rustworkx_core::connectivity::core_number(&pg);

    let mut result = HashMap::new();
    for (i, &node_id) in graph.nodes.iter().enumerate() {
        if let Some(&core) = cores.get(&NodeIndex::new(i)) {
            result.insert(node_id, core);
        }
    }
    result
}

/// Articulation points of a branchial graph.
///
/// Returns node IDs whose removal would disconnect the branchial graph.
/// These are critical branching junctions — removing one disconnects
/// the parallel computation structure.
#[must_use]
pub fn branchial_articulation_points(graph: &BranchialGraph) -> Vec<MultiwayNodeId> {
    let (pg, _) = graph.to_petgraph();
    let artics = rustworkx_core::connectivity::articulation_points(&pg, None);

    artics
        .into_iter()
        .filter_map(|idx| pg.node_weight(idx).copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::evolution_graph::MultiwayEvolutionGraph;
    use super::*;

    #[test]
    fn to_petgraph_preserves_structure() {
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let root = graph.add_root(0);
        graph.add_fork(root, vec![(1, (), 0), (2, (), 1), (3, (), 2)]);

        let branchial = BranchialGraph::from_evolution_at_step(&graph, 1);
        let (pg, idx_map) = branchial.to_petgraph();

        assert_eq!(pg.node_count(), 3);
        assert_eq!(pg.edge_count(), 3); // K₃
        assert_eq!(idx_map.len(), 3);
    }

    #[test]
    fn coloring_k3_uses_three_colors() {
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let root = graph.add_root(0);
        graph.add_fork(root, vec![(1, (), 0), (2, (), 1), (3, (), 2)]);

        let branchial = BranchialGraph::from_evolution_at_step(&graph, 1);
        let coloring = branchial_coloring(&branchial);

        let num_colors = coloring
            .values()
            .copied()
            .collect::<std::collections::HashSet<_>>()
            .len();
        assert_eq!(num_colors, 3);

        // No two adjacent nodes share a color
        for (a, b) in &branchial.edges {
            assert_ne!(
                coloring[a], coloring[b],
                "adjacent nodes must have different colors"
            );
        }
    }

    #[test]
    fn coloring_k2_uses_two_colors() {
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let root = graph.add_root(0);
        graph.add_fork(root, vec![(1, (), 0), (2, (), 1)]);

        let branchial = BranchialGraph::from_evolution_at_step(&graph, 1);
        let coloring = branchial_coloring(&branchial);

        let num_colors = coloring
            .values()
            .copied()
            .collect::<std::collections::HashSet<_>>()
            .len();
        assert_eq!(num_colors, 2);
    }

    #[test]
    fn core_numbers_k3() {
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let root = graph.add_root(0);
        graph.add_fork(root, vec![(1, (), 0), (2, (), 1), (3, (), 2)]);

        let branchial = BranchialGraph::from_evolution_at_step(&graph, 1);
        let cores = branchial_core_numbers(&branchial);

        // Every node in K₃ has degree 2, so core number = 2
        for &core in cores.values() {
            assert_eq!(core, 2);
        }
    }

    #[test]
    fn articulation_points_k3_biconnected() {
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let root = graph.add_root(0);
        graph.add_fork(root, vec![(1, (), 0), (2, (), 1), (3, (), 2)]);

        let branchial = BranchialGraph::from_evolution_at_step(&graph, 1);
        let artics = branchial_articulation_points(&branchial);

        // K₃ is biconnected — no articulation points
        assert!(artics.is_empty());
    }
}
