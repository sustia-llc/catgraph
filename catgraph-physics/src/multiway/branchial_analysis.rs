//! Graph algorithms for branchial graphs and multiway evolution graphs via
//! petgraph and rustworkx-core.
//!
//! Two petgraph shims live here:
//!
//! - [`BranchialGraph::to_petgraph`] → an undirected `UnGraph`, feeding the
//!   coloring, k-core, articulation-point, and all-pairs-distance wrappers.
//! - [`MultiwayEvolutionGraph::to_petgraph`] → a directed `DiGraph`, feeding
//!   the centrality wrappers [`multiway_betweenness`] and [`multiway_katz`].

use std::collections::HashMap;
use std::convert::Infallible;

use petgraph::graph::{DiGraph, NodeIndex, UnGraph};

use super::branchial::BranchialGraph;
use super::evolution_graph::{MultiwayEvolutionGraph, MultiwayNodeId};

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

impl<S, T> MultiwayEvolutionGraph<S, T> {
    /// Directed petgraph export. Nodes carry [`MultiwayNodeId`] in ascending
    /// `(step, branch_id)` order, `order[i]` at `NodeIndex::new(i)`; edges are
    /// unweighted parent → child, multi-edges preserved; an edge is emitted
    /// only when both endpoints are registered nodes.
    ///
    /// # Examples
    ///
    /// ```
    /// use catgraph_physics::multiway::MultiwayEvolutionGraph;
    ///
    /// let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
    /// let root = graph.add_root(0);
    /// graph.add_fork(root, vec![(1, (), 0), (2, (), 1)]);
    ///
    /// let (pg, order) = graph.to_petgraph();
    /// assert_eq!(pg.node_count(), 3);
    /// assert_eq!(pg.edge_count(), 2);
    /// assert_eq!(order[0], root);
    /// ```
    #[must_use]
    pub fn to_petgraph(&self) -> (DiGraph<MultiwayNodeId, ()>, Vec<MultiwayNodeId>) {
        let order = self.node_ids_sorted();
        let mut pg = DiGraph::with_capacity(order.len(), self.edge_count());
        let mut node_map: HashMap<MultiwayNodeId, NodeIndex> = HashMap::with_capacity(order.len());

        for &id in &order {
            node_map.insert(id, pg.add_node(id));
        }

        for &from in &order {
            let (Some(&source), Some(edges)) = (node_map.get(&from), self.get_forward_edges(&from))
            else {
                continue;
            };
            for edge in edges {
                if let Some(&target) = node_map.get(&edge.to) {
                    pg.add_edge(source, target, ());
                }
            }
        }

        (pg, order)
    }
}

/// Node count from which [`multiway_betweenness`] runs on rayon (`parallel` on).
#[cfg(feature = "parallel")]
const BETWEENNESS_PARALLEL_THRESHOLD: usize = 50;

/// Unreachable threshold: the Brandes sweep is always sequential (`parallel` off).
#[cfg(not(feature = "parallel"))]
const BETWEENNESS_PARALLEL_THRESHOLD: usize = usize::MAX;

/// Brandes betweenness centrality, endpoints excluded, keyed by
/// [`MultiwayNodeId`]. `normalized` divides by `(n-1)(n-2)`.
///
/// From `BETWEENNESS_PARALLEL_THRESHOLD` (50) nodes with `parallel` on the
/// sweep runs on rayon and scores are not bit-reproducible across runs.
///
/// # Examples
///
/// ```
/// use catgraph_physics::multiway::{MultiwayEvolutionGraph, multiway_betweenness};
///
/// let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
/// let root = graph.add_root(0);
/// let kids = graph.add_fork(root, vec![(1, (), 0), (2, (), 1)]);
/// graph.add_sequential_step(kids[0], 3, ());
///
/// let scores = multiway_betweenness(&graph, false);
/// assert!((scores[&kids[0]] - 1.0).abs() < 1e-12);
/// assert!(scores[&root].abs() < 1e-12);
/// ```
#[must_use]
pub fn multiway_betweenness<S, T>(
    graph: &MultiwayEvolutionGraph<S, T>,
    normalized: bool,
) -> HashMap<MultiwayNodeId, f64> {
    let (pg, order) = graph.to_petgraph();
    let scores = rustworkx_core::centrality::betweenness_centrality(
        &pg,
        false,
        normalized,
        BETWEENNESS_PARALLEL_THRESHOLD,
    );

    order
        .into_iter()
        .enumerate()
        .filter_map(|(i, id)| scores.get(i).copied().flatten().map(|score| (id, score)))
        .collect()
}

/// Katz centrality `x = αAᵀx + β`, β = 1, L2-normalised, keyed by
/// [`MultiwayNodeId`].
///
/// `alpha`, `max_iter`, `tol` default to `0.1`, `1000`, `1e-6`.
///
/// Returns `Some(empty)` on an empty graph. Returns `None` when the iteration
/// does not converge within `max_iter` — reachable when `ρ(A) ≥ 1/alpha`,
/// i.e. a cycle introduced via
/// [`add_merge_edge`](MultiwayEvolutionGraph::add_merge_edge) — or the fixed
/// point has zero norm.
///
/// # Examples
///
/// ```
/// use catgraph_physics::multiway::{MultiwayEvolutionGraph, multiway_katz};
///
/// let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
/// let root = graph.add_root(0);
/// let kids = graph.add_fork(root, vec![(1, (), 0), (2, (), 1), (3, (), 2)]);
///
/// let scores = multiway_katz(&graph, None, None, None).expect("star fork converges");
/// assert!(scores[&root] > 0.4);
/// assert!(scores[&kids[0]] > scores[&root]);
/// ```
#[must_use]
pub fn multiway_katz<S, T>(
    graph: &MultiwayEvolutionGraph<S, T>,
    alpha: Option<f64>,
    max_iter: Option<usize>,
    tol: Option<f64>,
) -> Option<HashMap<MultiwayNodeId, f64>> {
    let (pg, order) = graph.to_petgraph();

    // rustworkx's convergence test `sum < n * tol` never fires at n = 0.
    if order.is_empty() {
        return Some(HashMap::new());
    }

    let result: Result<Option<Vec<f64>>, Infallible> = rustworkx_core::centrality::katz_centrality(
        &pg,
        |_| Ok(1.0),
        alpha,
        None,
        None,
        max_iter,
        tol,
    );

    let scores = match result {
        Ok(scores) => scores,
        Err(never) => match never {},
    }?;

    Some(
        order
            .into_iter()
            .enumerate()
            .filter_map(|(i, id)| scores.get(i).copied().map(|score| (id, score)))
            .collect(),
    )
}

/// Greedy coloring, keyed by [`MultiwayNodeId`], colors 0-based; the color
/// count is an upper bound on the chromatic number.
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

/// Core number (largest `k` whose k-core contains the node), keyed by
/// [`MultiwayNodeId`].
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

/// Nodes whose removal disconnects the graph.
#[must_use]
pub fn branchial_articulation_points(graph: &BranchialGraph) -> Vec<MultiwayNodeId> {
    let (pg, _) = graph.to_petgraph();
    let artics = rustworkx_core::connectivity::articulation_points(&pg, None);

    artics
        .into_iter()
        .filter_map(|idx| pg.node_weight(idx).copied())
        .collect()
}

/// Node count from which [`branchial_distance_matrix`] runs on rayon (`parallel` on).
#[cfg(feature = "parallel")]
const APSP_PARALLEL_THRESHOLD: usize = 300;

/// Unreachable threshold: the all-pairs sweep is always sequential (`parallel` off).
#[cfg(not(feature = "parallel"))]
const APSP_PARALLEL_THRESHOLD: usize = usize::MAX;

/// All-pairs unweighted hop distances in `graph.nodes` order: `dist[i][j]`
/// from `nodes[i]` to `nodes[j]`, `0.0` on the diagonal, `f64::INFINITY` when
/// unreachable.
pub(crate) fn branchial_distance_matrix(graph: &BranchialGraph) -> Vec<Vec<f64>> {
    let (pg, _) = graph.to_petgraph();
    let matrix = rustworkx_core::shortest_path::distance_matrix(
        &pg,
        APSP_PARALLEL_THRESHOLD,
        false,
        f64::INFINITY,
    );

    matrix.outer_iter().map(|row| row.to_vec()).collect()
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

    /// Path 7-2-9 plus disjoint edge 4-1, branch ids non-ascending; then the
    /// 0×0 and 1×1 shapes.
    #[test]
    fn distance_matrix_marks_unreachable_and_keeps_node_order() {
        use super::super::evolution_graph::BranchId;

        let id = |branch: usize| MultiwayNodeId::new(BranchId(branch), 0);
        let graph = BranchialGraph {
            step: 0,
            nodes: vec![id(7), id(2), id(9), id(4), id(1)],
            edges: vec![(id(7), id(2)), (id(2), id(9)), (id(4), id(1))],
        };

        let inf = f64::INFINITY;
        assert_eq!(
            branchial_distance_matrix(&graph),
            vec![
                vec![0.0, 1.0, 2.0, inf, inf],
                vec![1.0, 0.0, 1.0, inf, inf],
                vec![2.0, 1.0, 0.0, inf, inf],
                vec![inf, inf, inf, 0.0, 1.0],
                vec![inf, inf, inf, 1.0, 0.0],
            ]
        );

        let empty = BranchialGraph {
            step: 0,
            nodes: Vec::new(),
            edges: Vec::new(),
        };
        assert!(branchial_distance_matrix(&empty).is_empty());

        let singleton = BranchialGraph {
            step: 0,
            nodes: vec![id(7)],
            edges: Vec::new(),
        };
        assert_eq!(branchial_distance_matrix(&singleton), vec![vec![0.0]]);
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

    // --- multiway (evolution-graph) centrality ------------------------------

    /// root → {a, b} → merge.
    fn diamond() -> (
        MultiwayEvolutionGraph<i32, ()>,
        MultiwayNodeId,
        MultiwayNodeId,
        MultiwayNodeId,
        MultiwayNodeId,
    ) {
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let root = graph.add_root(0);
        let kids = graph.add_fork(root, vec![(1, (), 0), (2, (), 1)]);
        let merge = graph.add_sequential_step(kids[0], 3, ());
        graph.add_merge_edge(kids[1], merge, ());
        (graph, root, kids[0], kids[1], merge)
    }

    #[test]
    fn evolution_to_petgraph_is_faithful_and_deterministically_ordered() {
        // 1 root + 5 forks + 5 sequential steps.
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let root = graph.add_root(0);
        let kids = graph.add_fork(
            root,
            vec![(1, (), 0), (2, (), 1), (3, (), 2), (4, (), 3), (5, (), 4)],
        );
        for (i, &kid) in kids.iter().enumerate() {
            graph.add_sequential_step(kid, 100 + i32::try_from(i).expect("i < 5"), ());
        }

        let (pg, order) = graph.to_petgraph();

        assert_eq!(pg.node_count(), graph.node_count());
        assert_eq!(pg.edge_count(), graph.edge_count());
        assert_eq!(order.len(), graph.node_count());

        for (i, &id) in order.iter().enumerate() {
            assert_eq!(pg.node_weight(NodeIndex::new(i)), Some(&id));
        }

        let keys: Vec<(usize, usize)> = order.iter().map(|id| (id.step, id.branch_id.0)).collect();
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(keys, sorted, "node order must be canonical, not map order");
        assert_eq!(order[0], root, "the step-0 root sorts first");

        for &id in &order {
            let Some(edges) = graph.get_forward_edges(&id) else {
                continue;
            };
            for edge in edges {
                let source = NodeIndex::new(
                    order
                        .iter()
                        .position(|&o| o == edge.from)
                        .expect("edge source is a node of the graph"),
                );
                let target = NodeIndex::new(
                    order
                        .iter()
                        .position(|&o| o == edge.to)
                        .expect("edge target is a node of the graph"),
                );
                assert!(pg.find_edge(source, target).is_some());
            }
        }
    }

    #[test]
    fn betweenness_on_diamond_matches_hand_computation() {
        let (graph, root, a, b, merge) = diamond();
        let raw = multiway_betweenness(&graph, false);

        // (root, merge) has two shortest paths, one through each of a, b: 1/2 each.
        assert!((raw[&a] - 0.5).abs() < 1e-12, "a scored {}", raw[&a]);
        assert!((raw[&b] - 0.5).abs() < 1e-12, "b scored {}", raw[&b]);
        assert!(raw[&root].abs() < 1e-12);
        assert!(raw[&merge].abs() < 1e-12);

        // (n-1)(n-2) = 6.
        let normalized = multiway_betweenness(&graph, true);
        assert!((normalized[&a] - 0.5 / 6.0).abs() < 1e-12);
        assert_eq!(normalized.len(), 4);
    }

    /// Star fork, α = 0.1, β = 1: fixed point root = 1, child = 1.1;
    /// L2-normalised root = 1/√(1 + 3·1.1²).
    #[test]
    fn katz_star_fork_hand_values() {
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let root = graph.add_root(0);
        let kids = graph.add_fork(root, vec![(1, (), 0), (2, (), 1), (3, (), 2)]);

        let scores = multiway_katz(&graph, None, None, None).expect("star fork converges");
        let expected_root = 1.0 / (1.0 + 3.0 * 1.1_f64.powi(2)).sqrt();
        assert!(
            (scores[&root] - expected_root).abs() < 1e-9,
            "root scored {}, expected {expected_root}",
            scores[&root]
        );
        for &kid in &kids {
            assert!(scores[&kid] > scores[&root]);
        }
        assert_eq!(scores.len(), 4);
    }

    /// Empty graph: `Some(empty)` from katz, empty map from betweenness.
    #[test]
    fn empty_graph_scores_empty_rather_than_failing_to_converge() {
        let graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();

        let katz = multiway_katz(&graph, None, None, None);
        assert_eq!(
            katz,
            Some(HashMap::new()),
            "an empty graph must score empty, not None"
        );
        assert!(multiway_betweenness(&graph, false).is_empty());
    }

    #[test]
    fn katz_and_betweenness_cover_every_node() {
        let (graph, ..) = diamond();
        let katz = multiway_katz(&graph, Some(0.25), None, None).expect("diamond converges");
        let betweenness = multiway_betweenness(&graph, false);

        assert_eq!(katz.len(), graph.node_count());
        assert_eq!(betweenness.len(), graph.node_count());
        let norm: f64 = katz.values().map(|v| v * v).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() < 1e-9, "‖x‖₂ = {norm}");
    }
}
