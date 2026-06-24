//! Branchial graph extraction from multiway evolution.
//!
//! A **branchial graph** at time step t represents the tensor product
//! structure of parallel computations:
//! - Vertices = states at time t (the branchlike hypersurface Σ`_t)`
//! - Edges = states that share a common ancestor
//!
//! The branchial structure captures "which computations are in parallel"
//! at each moment, which is essential for multicomputational irreducibility.
//!
//! ## Mathematical Background
//!
//! In the symmetric monoidal category ⟨𝒯, ⊗, I⟩:
//! - The branchial graph at step t represents states connected by ⊗
//! - The edge count measures "branchial distance"
//! - Full connectivity means all branches share a common ancestor

use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;

use super::evolution_graph::{MergePoint, MultiwayEvolutionGraph, MultiwayNodeId};

/// A branchial graph at a specific time step.
///
/// Captures the tensor product structure of parallel branches,
/// showing which states are "simultaneous" in the multiway evolution.
#[derive(Clone, Debug)]
pub struct BranchialGraph {
    /// The time step this graph represents.
    pub step: usize,
    /// Node IDs present at this step.
    pub nodes: Vec<MultiwayNodeId>,
    /// Edges between nodes sharing a common ancestor.
    /// Each edge connects two nodes at the same step.
    pub edges: Vec<(MultiwayNodeId, MultiwayNodeId)>,
}

impl BranchialGraph {
    /// Build a branchial graph from a `MultiwayEvolutionGraph` at a specific step.
    ///
    /// Two nodes at the same step are connected if they share a common ancestor.
    /// Ancestor sets are pre-computed once per node (O(n)) rather than per pair
    /// (O(n^2)), reducing overall complexity from O(n^3) to O(n^2).
    #[must_use]
    pub fn from_evolution_at_step<S: Clone + Hash, T: Clone>(
        graph: &MultiwayEvolutionGraph<S, T>,
        step: usize,
    ) -> Self {
        let nodes = graph.node_ids_at_step(step);

        // Pre-compute ancestor sets once per node (not per pair)
        let ancestor_sets: Vec<HashSet<MultiwayNodeId>> = nodes
            .iter()
            .map(|&node| Self::collect_ancestors(graph, node))
            .collect();

        // Check pairwise intersection using cached sets
        let mut edges = Vec::new();
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                if !ancestor_sets[i].is_disjoint(&ancestor_sets[j]) {
                    edges.push((nodes[i], nodes[j]));
                }
            }
        }

        Self { step, nodes, edges }
    }

    /// Collect all ancestors of a node (BFS backwards).
    fn collect_ancestors<S: Clone + Hash, T: Clone>(
        graph: &MultiwayEvolutionGraph<S, T>,
        node: MultiwayNodeId,
    ) -> HashSet<MultiwayNodeId> {
        let mut ancestors = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(node);

        while let Some(current) = queue.pop_front() {
            if ancestors.insert(current)
                && let Some(edges) = graph.get_backward_edges(&current)
            {
                for edge in edges {
                    queue.push_back(edge.from);
                }
            }
        }

        ancestors
    }

    /// Number of nodes at this step (parallel branches).
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges (measure of "branchial connectivity").
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Check if fully connected (all branches share a common ancestor).
    ///
    /// A fully connected branchial graph means all parallel computations
    /// originated from the same initial state.
    #[must_use]
    pub fn is_fully_connected(&self) -> bool {
        if self.nodes.len() <= 1 {
            return true;
        }

        // For n nodes to be fully connected, need n*(n-1)/2 edges
        let max_edges = self.nodes.len() * (self.nodes.len() - 1) / 2;
        self.edges.len() == max_edges
    }

    /// Compute branchial adjacency matrix.
    ///
    /// Returns (`node_order`, matrix) where `matrix[i][j]` = true if connected.
    #[must_use]
    pub fn adjacency_matrix(&self) -> (Vec<MultiwayNodeId>, Vec<Vec<bool>>) {
        let n = self.nodes.len();
        let mut matrix = vec![vec![false; n]; n];

        // Create node index map
        let node_indices: HashMap<MultiwayNodeId, usize> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, &id)| (id, i))
            .collect();

        for (a, b) in &self.edges {
            if let (Some(&i), Some(&j)) = (node_indices.get(a), node_indices.get(b)) {
                matrix[i][j] = true;
                matrix[j][i] = true; // Symmetric
            }
        }

        (self.nodes.clone(), matrix)
    }

    /// Count connected components in the branchial graph.
    ///
    /// Each component represents an independent "universe" of computation.
    #[must_use]
    pub fn connected_components(&self) -> usize {
        if self.nodes.is_empty() {
            return 0;
        }

        let mut visited = HashSet::new();
        let mut components = 0;

        // Build adjacency list
        let mut adj: HashMap<MultiwayNodeId, Vec<MultiwayNodeId>> = HashMap::new();
        for &node in &self.nodes {
            adj.insert(node, Vec::new());
        }
        for (a, b) in &self.edges {
            adj.entry(*a).or_default().push(*b);
            adj.entry(*b).or_default().push(*a);
        }

        for &start in &self.nodes {
            if !visited.contains(&start) {
                // BFS to mark all reachable
                let mut queue = VecDeque::new();
                queue.push_back(start);
                while let Some(node) = queue.pop_front() {
                    if visited.insert(node)
                        && let Some(neighbors) = adj.get(&node)
                    {
                        for &neighbor in neighbors {
                            if !visited.contains(&neighbor) {
                                queue.push_back(neighbor);
                            }
                        }
                    }
                }
                components += 1;
            }
        }

        components
    }
}

/// Extract branchial graphs at all time steps (branchial foliation).
///
/// Returns a sequence of branchial graphs, one for each step from 0 to `max_step`.
#[must_use]
pub fn extract_branchial_foliation<S: Clone + Hash, T: Clone>(
    graph: &MultiwayEvolutionGraph<S, T>,
) -> Vec<BranchialGraph> {
    (0..=graph.max_step())
        .map(|step| BranchialGraph::from_evolution_at_step(graph, step))
        .collect()
}

/// Compute parallel-branch step pairs from the branchial foliation.
///
/// At each step boundary `(t, t+1)`, yields one `(t, t+1)` entry per node at
/// step `t` that has at least one forward transition. Downstream consumers
/// (e.g. `irreducible`) wrap the raw `(usize, usize)` pairs in their own
/// interval types — this helper keeps catgraph interval-type-free.
#[must_use]
pub fn branchial_parallel_step_pairs<S: Clone + Hash, T: Clone>(
    graph: &MultiwayEvolutionGraph<S, T>,
) -> Vec<Vec<(usize, usize)>> {
    let foliation = extract_branchial_foliation(graph);

    // TODO(rayon-1.12): candidate for `par_array_windows::<2>()` from the
    // new `ParallelSlice` API if `foliation.len()` is large enough. Per-pair
    // work is cheap here (filter/map over nodes), so a bench is needed before
    // swapping — see `~/.claude/summaries/rayon-summary-0.md`.
    foliation
        .windows(2)
        .map(|pair| {
            pair[0]
                .nodes
                .iter()
                .filter(|&node_id| graph.get_forward_edges(node_id).is_some())
                .map(|_| (pair[0].step, pair[1].step))
                .collect()
        })
        .collect()
}

/// Aggregate summary of branchial structure across all time steps.
///
/// Computed from a branchial foliation via [`BranchialSummary::from_foliation`].
/// Reports peak branching, total connectivity, and per-step statistics.
#[derive(Clone, Debug)]
pub struct BranchialSummary {
    /// Maximum branch count at any step.
    pub max_parallel_branches: usize,
    /// Step with maximum branching.
    pub peak_branching_step: usize,
    /// Total edge count across all branchial graphs.
    pub total_branchial_edges: usize,
    /// Number of steps where graph is fully connected.
    pub fully_connected_steps: usize,
    /// Average branches per step.
    pub average_branches: f64,
    /// Per-step statistics.
    pub per_step: Vec<BranchialStepStats>,
}

/// Statistics for a single step's branchial graph.
#[derive(Clone, Debug)]
pub struct BranchialStepStats {
    /// Time step this snapshot represents.
    pub step: usize,
    /// Number of parallel branches (nodes in `Σ_t`).
    pub node_count: usize,
    /// Number of common-ancestor edges between branches.
    pub edge_count: usize,
    /// Number of connected components (independent "universes").
    pub components: usize,
    /// Whether all branches share a common ancestor.
    pub is_fully_connected: bool,
}

impl BranchialSummary {
    /// Compute summary from branchial foliation.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    #[must_use]
    pub fn from_foliation(foliation: &[BranchialGraph]) -> Self {
        if foliation.is_empty() {
            return Self {
                max_parallel_branches: 0,
                peak_branching_step: 0,
                total_branchial_edges: 0,
                fully_connected_steps: 0,
                average_branches: 0.0,
                per_step: Vec::new(),
            };
        }

        let per_step: Vec<BranchialStepStats> = foliation
            .iter()
            .map(|bg| BranchialStepStats {
                step: bg.step,
                node_count: bg.node_count(),
                edge_count: bg.edge_count(),
                components: bg.connected_components(),
                is_fully_connected: bg.is_fully_connected(),
            })
            .collect();

        let max_parallel_branches = per_step.iter().map(|s| s.node_count).max().unwrap_or(0);
        let peak_branching_step = per_step
            .iter()
            .max_by_key(|s| s.node_count)
            .map_or(0, |s| s.step);
        let total_branchial_edges = per_step.iter().map(|s| s.edge_count).sum();
        let fully_connected_steps = per_step.iter().filter(|s| s.is_fully_connected).count();
        let total_branches: usize = per_step.iter().map(|s| s.node_count).sum();
        let average_branches = if foliation.is_empty() {
            0.0
        } else {
            total_branches as f64 / foliation.len() as f64
        };

        Self {
            max_parallel_branches,
            peak_branching_step,
            total_branchial_edges,
            fully_connected_steps,
            average_branches,
            per_step,
        }
    }
}

/// Find all merge points in a multiway graph.
///
/// A merge point is a node with multiple incoming edges from different branches.
#[must_use]
pub fn find_all_merge_points<S: Clone + Hash, T: Clone>(
    graph: &MultiwayEvolutionGraph<S, T>,
) -> Vec<MergePoint> {
    let merge_ids = graph.find_merge_points();

    merge_ids
        .into_iter()
        .filter_map(|id| {
            graph.get_backward_edges(&id).map(|edges| MergePoint {
                merged_node: id,
                parent_nodes: edges.iter().map(|e| e.from).collect(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::evolution_graph::MultiwayEvolutionGraph;
    use super::*;

    #[test]
    fn test_branchial_graph_single_branch() {
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let root = graph.add_root(0);
        graph.add_sequential_step(root, 1, ());

        let branchial = BranchialGraph::from_evolution_at_step(&graph, 0);
        assert_eq!(branchial.node_count(), 1);
        assert_eq!(branchial.edge_count(), 0);
        assert!(branchial.is_fully_connected());
    }

    #[test]
    fn test_branchial_graph_fork() {
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let root = graph.add_root(0);
        graph.add_fork(root, vec![(1, (), 0), (2, (), 1), (3, (), 2)]);

        // At step 1, we have 3 nodes that all share the root as common ancestor
        let branchial = BranchialGraph::from_evolution_at_step(&graph, 1);
        assert_eq!(branchial.node_count(), 3);
        // All 3 nodes share common ancestor, so should have 3 edges (fully connected)
        assert_eq!(branchial.edge_count(), 3); // 3 choose 2 = 3
        assert!(branchial.is_fully_connected());
    }

    #[test]
    fn test_extract_branchial_foliation() {
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let root = graph.add_root(0);
        let ids = graph.add_fork(root, vec![(1, (), 0), (2, (), 1)]);
        graph.add_sequential_step(ids[0], 10, ());
        graph.add_sequential_step(ids[1], 20, ());

        let foliation = extract_branchial_foliation(&graph);
        assert_eq!(foliation.len(), 3); // steps 0, 1, 2

        assert_eq!(foliation[0].node_count(), 1); // root only
        assert_eq!(foliation[1].node_count(), 2); // two branches
        assert_eq!(foliation[2].node_count(), 2); // two branches continue
    }

    #[test]
    fn test_branchial_summary() {
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let root = graph.add_root(0);
        graph.add_fork(root, vec![(1, (), 0), (2, (), 1), (3, (), 2)]);

        let foliation = extract_branchial_foliation(&graph);
        let summary = BranchialSummary::from_foliation(&foliation);

        assert_eq!(summary.max_parallel_branches, 3);
        assert_eq!(summary.peak_branching_step, 1);
    }

    #[test]
    fn test_connected_components() {
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();

        // Create two separate roots (disconnected components)
        let root1 = graph.add_root(0);
        let root2 = graph.add_root(100);

        graph.add_sequential_step(root1, 1, ());
        graph.add_sequential_step(root2, 101, ());

        let branchial = BranchialGraph::from_evolution_at_step(&graph, 1);
        assert_eq!(branchial.node_count(), 2);
        assert_eq!(branchial.edge_count(), 0); // No common ancestor
        assert_eq!(branchial.connected_components(), 2);
        assert!(!branchial.is_fully_connected());
    }

    #[test]
    fn test_adjacency_matrix() {
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let root = graph.add_root(0);
        graph.add_fork(root, vec![(1, (), 0), (2, (), 1)]);

        let branchial = BranchialGraph::from_evolution_at_step(&graph, 1);
        let (nodes, matrix) = branchial.adjacency_matrix();

        assert_eq!(nodes.len(), 2);
        assert_eq!(matrix.len(), 2);
        // Both nodes share common ancestor, so should be connected
        assert!(matrix[0][1]);
        assert!(matrix[1][0]);
    }
}
