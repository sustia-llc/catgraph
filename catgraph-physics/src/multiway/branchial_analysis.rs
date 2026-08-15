//! Graph algorithms for branchial graphs and multiway evolution graphs via
//! petgraph and rustworkx-core.
//!
//! Two petgraph shims live here:
//!
//! - [`BranchialGraph::to_petgraph`] → an undirected `UnGraph`, feeding the
//!   coloring, k-core, and articulation-point wrappers.
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
    /// Convert to a petgraph directed graph for algorithm application.
    ///
    /// The directed sibling of [`BranchialGraph::to_petgraph`]. Nodes carry
    /// [`MultiwayNodeId`], edges are unweighted and point parent → child.
    /// Every forward edge is exported verbatim, including [merge
    /// edges](MultiwayEvolutionGraph::add_merge_edge); the result is therefore
    /// a multigraph exactly when the source is.
    ///
    /// Returns `(graph, order)` where `order[i]` is the [`MultiwayNodeId`]
    /// stored at `NodeIndex::new(i)`.
    ///
    /// # Determinism
    ///
    /// Node order is deterministic: `(step, branch_id)` ascending. The
    /// evolution graph stores nodes in a `HashMap`, so exporting in map order
    /// would give a different index assignment on every run — and every
    /// index-keyed centrality score computed downstream would move with it.
    /// Sorting up front is what makes [`multiway_betweenness`] and
    /// [`multiway_katz`] reproducible.
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
    /// // The root is at step 0, so it sorts first.
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

/// Node count at which [`multiway_betweenness`] hands the Brandes sweep to
/// rustworkx-core's parallel `CondIterator`.
///
/// 50 is the value rustworkx-core's own documentation recommends as the point
/// where parallelism starts paying for itself.
const BETWEENNESS_PARALLEL_THRESHOLD: usize = 50;

/// Betweenness centrality of a multiway evolution graph — branching-junction
/// load.
///
/// Scores each state by the fraction of shortest branch-paths that run through
/// it, so the states that many distinct computational histories must pass
/// through score highest. In multiway terms these are the bottlenecks of the
/// evolution: fork points whose children reconverge, and merge points that
/// several branches funnel into.
///
/// `normalized` divides by the number of node pairs, making scores comparable
/// across graphs of different sizes. Endpoints are excluded from the path
/// counts (the standard Brandes convention). Above
/// `BETWEENNESS_PARALLEL_THRESHOLD` (50) nodes the sweep runs on rayon via
/// rustworkx-core's `CondIterator`; `RAYON_NUM_THREADS` controls the width.
///
/// Returns a map from [`MultiwayNodeId`] to score. Node order — and hence the
/// scores — is deterministic; see
/// [`MultiwayEvolutionGraph::to_petgraph`].
///
/// # Examples
///
/// ```
/// use catgraph_physics::multiway::{MultiwayEvolutionGraph, multiway_betweenness};
///
/// // root → a → leaf, with a second child `b` hanging off the root.
/// let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
/// let root = graph.add_root(0);
/// let kids = graph.add_fork(root, vec![(1, (), 0), (2, (), 1)]);
/// graph.add_sequential_step(kids[0], 3, ());
///
/// let scores = multiway_betweenness(&graph, false);
/// // Only `root → a → leaf` has an interior node, so `a` carries it all.
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

/// Katz centrality of a multiway evolution graph — α-damped inbound path
/// count.
///
/// Iterates `x ← αAᵀx + β` to a fixed point, so a state's score accumulates
/// one α-damped contribution per distinct computational history reaching it:
/// deep, heavily-reconverged states score high, and a root is floored at β
/// rather than driven to zero.
///
/// # Why Katz and not eigenvector centrality
///
/// Issue #161 asked for eigenvector centrality. **It is undefined on this
/// object.** A multiway evolution graph is a DAG — every edge goes from step
/// `t` to step `t + 1` — so its adjacency matrix is nilpotent and its spectral
/// radius is 0. There is no dominant eigenvector for power iteration to
/// converge to: rustworkx-core's `eigenvector_centrality` returns `Ok(None)`
/// at its defaults, and if forced to terminate it reports a *sink indicator*
/// rather than a centrality — the branching junctions this function exists to
/// score come out at ≈ 0, which is the exact opposite of the intended
/// reading. Katz's `+ β` term is what makes it well-defined here: the
/// iteration terminates exactly on a nilpotent adjacency, and the β floor
/// keeps sources positive. This is not an approximation of eigenvector
/// centrality on a DAG — it is the replacement for a quantity that does not
/// exist.
///
/// # Arguments
///
/// `alpha` (attenuation, default `0.1`), `max_iter` (default `1000`), and
/// `tol` (default `1e-6`) pass straight through to rustworkx-core;
/// `None` takes its default. β is left at the scalar `1.0`.
///
/// # Returns
///
/// `Some(map)` from [`MultiwayNodeId`] to score, L2-normalized over all nodes.
/// `None` when rustworkx-core declines to produce a vector: the power
/// iteration did not converge within `max_iter`, or the converged vector had
/// zero norm. That case is surfaced rather than collapsed to zeros or to an
/// empty map, because "did not converge" and "every state scores 0" are
/// different facts about the evolution.
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
/// // The root is a source: floored at β, never ≈ 0 the way eigenvector
/// // centrality would leave it.
/// assert!(scores[&root] > 0.4);
/// // Children pick up the root's α-damped contribution on top of their own β.
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
    let result: Result<Option<Vec<f64>>, Infallible> = rustworkx_core::centrality::katz_centrality(
        &pg,
        |_| Ok(1.0),
        alpha,
        None,
        None,
        max_iter,
        tol,
    );

    // Edges are unweighted and the weight closure above returns `Ok`
    // unconditionally, so `E` is `Infallible` and the error arm is
    // uninhabited — `match never {}` discharges it totally, no `unwrap`.
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

    // --- multiway (evolution-graph) centrality, issue #161 ------------------

    /// Root → 2 children, both reconverging on one merge node.
    ///
    /// ```text
    ///     root
    ///     /  \
    ///    a    b
    ///     \  /
    ///     merge
    /// ```
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
        // Wide enough that a `HashMap`-order export would essentially never
        // come out sorted by accident: 1 root + 5 forks + 5 sequential steps.
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

        // `order[i]` really is the weight at index `i`.
        for (i, &id) in order.iter().enumerate() {
            assert_eq!(pg.node_weight(NodeIndex::new(i)), Some(&id));
        }

        // Ascending `(step, branch_id)` — the property a `HashMap` export loses.
        let keys: Vec<(usize, usize)> = order.iter().map(|id| (id.step, id.branch_id.0)).collect();
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(keys, sorted, "node order must be canonical, not map order");
        assert_eq!(order[0], root, "the step-0 root sorts first");

        // Every forward edge survived, directed parent → child.
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

        // The only (s, t) pair with an interior node is (root, merge), which
        // has two shortest paths — root→a→merge and root→b→merge. Each of `a`
        // and `b` lies on exactly one, so each scores 1/2. `root` and `merge`
        // are endpoints of every path they touch, so both score 0.
        assert!((raw[&a] - 0.5).abs() < 1e-12, "a scored {}", raw[&a]);
        assert!((raw[&b] - 0.5).abs() < 1e-12, "b scored {}", raw[&b]);
        assert!(raw[&root].abs() < 1e-12);
        assert!(raw[&merge].abs() < 1e-12);

        // Normalization on a directed graph divides by (n-1)(n-2) = 6.
        let normalized = multiway_betweenness(&graph, true);
        assert!((normalized[&a] - 0.5 / 6.0).abs() < 1e-12);
        assert_eq!(normalized.len(), 4);
    }

    #[test]
    fn katz_floors_the_root_where_eigenvector_centrality_degenerates() {
        // A star fork: the root is *the* branching junction, so any centrality
        // worth the name must score it well above zero.
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let root = graph.add_root(0);
        let kids = graph.add_fork(root, vec![(1, (), 0), (2, (), 1), (3, (), 2)]);
        let (pg, _) = graph.to_petgraph();

        // (1) Eigenvector centrality is undefined here. The adjacency of a
        //     step-graded DAG is nilpotent, so its spectral radius is 0 and
        //     power iteration has no dominant eigenvector to find: at the
        //     rustworkx-core defaults it simply declines to answer.
        let default_eigen: Result<Option<Vec<f64>>, Infallible> =
            rustworkx_core::centrality::eigenvector_centrality(&pg, |_| Ok(1.0), None, None);
        assert_eq!(
            default_eigen.expect("weight closure is infallible"),
            None,
            "eigenvector centrality must not converge on a nilpotent adjacency"
        );

        // (2) Forced to terminate, it reports a *sink indicator*: the root —
        //     the very junction we want scored — decays toward 0 while the
        //     leaves take everything. This is the behaviour #161 would have
        //     shipped, and the reason the issue's `multiway_eigenvector` was
        //     replaced by `multiway_katz`.
        let forced: Result<Option<Vec<f64>>, Infallible> =
            rustworkx_core::centrality::eigenvector_centrality(
                &pg,
                |_| Ok(1.0),
                Some(100_000),
                None,
            );
        let forced = forced
            .expect("weight closure is infallible")
            .expect("100k iterations converge");
        assert!(
            forced[0] < 1e-2,
            "forced eigenvector centrality should collapse the root, got {}",
            forced[0]
        );
        assert!(forced[1] > 0.5, "leaves absorb the whole vector");

        // (3) Katz floors the root at β/‖x‖ instead. Unnormalized the fixed
        //     point is root = β = 1 and each child = αβ + β = 1.1, so after L2
        //     normalization root = 1/√(1 + 3·1.1²).
        let scores = multiway_katz(&graph, None, None, None).expect("star fork converges");
        let expected_root = 1.0 / (1.0 + 3.0 * 1.1_f64.powi(2)).sqrt();
        assert!(
            (scores[&root] - expected_root).abs() < 1e-9,
            "root scored {}, expected {expected_root}",
            scores[&root]
        );
        assert!(
            scores[&root] > 0.4,
            "the branching junction must stay well clear of zero"
        );
        for &kid in &kids {
            assert!(
                scores[&kid] > scores[&root],
                "children pick up the root's α-damped contribution on top of β"
            );
        }
        assert_eq!(scores.len(), 4);
    }

    #[test]
    fn katz_and_betweenness_cover_every_node() {
        let (graph, ..) = diamond();
        let katz = multiway_katz(&graph, Some(0.25), None, None).expect("diamond converges");
        let betweenness = multiway_betweenness(&graph, false);

        assert_eq!(katz.len(), graph.node_count());
        assert_eq!(betweenness.len(), graph.node_count());
        // Katz is L2-normalized over all nodes.
        let norm: f64 = katz.values().map(|v| v * v).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() < 1e-9, "‖x‖₂ = {norm}");
    }
}
