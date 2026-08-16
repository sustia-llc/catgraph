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
    /// Convert to a petgraph directed graph for algorithm application.
    ///
    /// The directed sibling of [`BranchialGraph::to_petgraph`]. Nodes carry
    /// [`MultiwayNodeId`], edges are unweighted and point parent → child.
    /// Forward edges are exported in node order, including [merge
    /// edges](MultiwayEvolutionGraph::add_merge_edge); the result is therefore
    /// a multigraph exactly when the source is.
    ///
    /// Returns `(graph, order)` where `order[i]` is the [`MultiwayNodeId`]
    /// stored at `NodeIndex::new(i)`.
    ///
    /// # Dangling edges are dropped
    ///
    /// An edge is exported only when **both** endpoints are registered nodes.
    /// [`add_merge_edge`](MultiwayEvolutionGraph::add_merge_edge) validates
    /// neither endpoint, so an edge to a stale or foreign
    /// [`MultiwayNodeId`] is counted by
    /// [`edge_count`](MultiwayEvolutionGraph::edge_count) but silently absent
    /// here. Consequence worth stating plainly: on such a graph the
    /// centralities below are computed over a **different topology than the
    /// graph reports**, with no error and no diagnostic. If that matters,
    /// check `pg.edge_count() == graph.edge_count()` at the call site.
    ///
    /// # Determinism
    ///
    /// Node order is deterministic: `(step, branch_id)` ascending. The
    /// evolution graph stores nodes in a `HashMap`, so exporting in map order
    /// would give a different index assignment on every run — and every
    /// index-keyed centrality score computed downstream would move with it.
    /// Sorting up front is what makes the *index assignment*, and hence
    /// [`multiway_katz`], reproducible.
    ///
    /// ⚠ It does **not** make [`multiway_betweenness`] bit-reproducible once
    /// the parallel path engages (≥ 50 nodes, `parallel` feature on):
    /// rustworkx accumulates per-source Brandes partials into a shared
    /// `RwLock`-guarded buffer from rayon workers, so the f64 summation order —
    /// and the low bits of every score — varies run to run. Exactly-tied nodes
    /// can therefore reorder between runs in a downstream ranking, and the
    /// scores are not bit-pinnable. Build without `parallel` for a
    /// bit-reproducible sweep at any size.
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
///
/// **Gated on the `parallel` feature.** This is the crate's first rayon call
/// site, and `parallel` is the crate's single-threading switch, so with the
/// feature off the threshold is `usize::MAX` and `CondIterator` can never take
/// the rayon path. Without that gate a consumer building
/// `--no-default-features --features rustworkx` — a WASI or otherwise
/// single-threaded target — would get rayon work spawned from inside
/// `multiway_betweenness` on any evolution graph of ≥ 50 nodes, and on a
/// no-threads wasm target rayon's global pool init fails at *runtime*.
#[cfg(feature = "parallel")]
const BETWEENNESS_PARALLEL_THRESHOLD: usize = 50;

/// Sequential stand-in for [`BETWEENNESS_PARALLEL_THRESHOLD`] when the
/// `parallel` feature is off: no node count can reach it, so the Brandes sweep
/// always runs on the calling thread.
#[cfg(not(feature = "parallel"))]
const BETWEENNESS_PARALLEL_THRESHOLD: usize = usize::MAX;

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
/// counts (the standard Brandes convention). At or above
/// `BETWEENNESS_PARALLEL_THRESHOLD` (50) nodes — and only with the `parallel`
/// feature on — the sweep runs on rayon via rustworkx-core's `CondIterator`;
/// `RAYON_NUM_THREADS` controls the width. With `parallel` off the threshold is
/// `usize::MAX`, so the sweep is always sequential.
///
/// Returns a map from [`MultiwayNodeId`] to score. The node *ordering* is
/// deterministic, but on the parallel path the scores are **not**
/// bit-reproducible — see the Determinism section of
/// [`MultiwayEvolutionGraph::to_petgraph`] for why, and for the build that is.
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
/// Issue #161 asked for eigenvector centrality. **It is undefined on the
/// intended object.** A multiway evolution built only from
/// [`add_root`](MultiwayEvolutionGraph::add_root) /
/// [`add_fork`](MultiwayEvolutionGraph::add_fork) /
/// [`add_sequential_step`](MultiwayEvolutionGraph::add_sequential_step) is a
/// step-graded DAG — every edge runs from step `t` to step `t + 1` — so its
/// adjacency matrix is nilpotent and its spectral radius is 0. There is no
/// dominant eigenvector for power iteration to converge to: rustworkx-core's
/// `eigenvector_centrality` returns `Ok(None)` at its defaults, and if forced
/// to terminate it reports a *sink indicator* rather than a centrality — the
/// branching junctions this function exists to score come out at ≈ 0, which is
/// the exact opposite of the intended reading. Katz's `+ β` term is what makes
/// it well-defined here: on a nilpotent adjacency the iteration terminates
/// exactly, and the β floor keeps sources positive. This is not an
/// approximation of eigenvector centrality on a DAG — it is the replacement
/// for a quantity that does not exist.
///
/// # ⚠ The DAG grading is a convention, not an invariant
///
/// [`add_merge_edge`](MultiwayEvolutionGraph::add_merge_edge) takes an
/// arbitrary `(from, to)` pair and validates neither the endpoints nor their
/// steps, so a caller *can* introduce a same-step or backward edge — and its
/// own documented workflow permits it, since
/// [`add_fork`](MultiwayEvolutionGraph::add_fork) leaves every step-`t + 1`
/// sibling in the active-state set, so a step-`t + 2` node may legally merge
/// into a step-`t + 1` one.
///
/// With such a back-edge the adjacency is **no longer nilpotent** and the
/// termination guarantee above does not hold. If `ρ(A) ≥ 1/alpha` the Katz
/// iteration diverges, burns all `max_iter` rounds on values running to
/// infinity or `NaN`, and yields `None`. Read a `None` on a graph carrying
/// merge edges as *"α is too large for this spectral radius"* at least as
/// readily as "needs more iterations"; retry with a smaller `alpha` before
/// raising `max_iter`.
///
/// # Arguments
///
/// `alpha` (attenuation, default `0.1`), `max_iter` (default `1000`), and
/// `tol` (default `1e-6`) pass straight through to rustworkx-core;
/// `None` takes its default. β is left at the scalar `1.0`.
///
/// Note that rustworkx-core 0.18's own doc comment for `katz_centrality` says
/// `max_iter` defaults to 100; the code reads `unwrap_or(1000)`. 1000 is the
/// effective default and the one documented here.
///
/// # Returns
///
/// `Some(map)` from [`MultiwayNodeId`] to score, L2-normalized over all nodes.
/// An empty graph returns `Some(empty map)`, matching
/// [`multiway_betweenness`] — rustworkx's convergence test is
/// `sum < node_count * tol`, which is `0.0 < 0.0` for an empty graph, so
/// without the short-circuit it would spin `max_iter` times and report a
/// non-convergence that never happened.
///
/// `None` when rustworkx-core declines to produce a vector on a **non-empty**
/// graph: the iteration did not converge within `max_iter` (see the
/// back-edge caveat above), or the converged vector had zero norm. That case
/// is surfaced rather than collapsed to zeros or to an empty map, because
/// "did not converge" and "every state scores 0" are different facts about
/// the evolution.
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

    // rustworkx's convergence test is `sum < node_count as f64 * tol`, i.e.
    // `0.0 < 0.0` — false — on an empty graph, so it would exhaust `max_iter`
    // and hand back `Ok(None)`. Reporting "did not converge" for a graph with
    // nothing to score is both wrong and asymmetric with `multiway_betweenness`,
    // which returns an empty map here.
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

/// Node count at which [`branchial_distance_matrix`] hands its per-source BFS
/// sweep to rayon.
///
/// 300 is the value rustworkx-core's own `distance_matrix` documentation names
/// as a good default — six times the 50 used for the Brandes sweep, a BFS row
/// being much the cheaper unit of work.
///
/// **Gated on the `parallel` feature**, for the same reason as
/// `BETWEENNESS_PARALLEL_THRESHOLD`: `parallel` is this crate's
/// single-threading switch, and once `node_count() >= parallel_threshold`
/// `distance_matrix` branches straight onto rayon's `into_par_iter()`. Without
/// the gate a consumer building `--no-default-features --features rustworkx` —
/// a WASI or otherwise single-threaded target — would get rayon work spawned
/// from inside [`OllivierRicciCurvature`] on any branchial graph of ≥ 300
/// nodes, and on a no-threads wasm target rayon's global pool init fails at
/// *runtime*. With the feature off the threshold is `usize::MAX`, which no node
/// count reaches, so the sweep always runs on the calling thread.
///
/// (`distance_matrix` branches on a plain `if`, not the `rayon_cond`
/// `CondIterator` the centrality functions use. Different mechanism, identical
/// hazard.)
///
/// [`OllivierRicciCurvature`]: super::ollivier_ricci::OllivierRicciCurvature
#[cfg(feature = "parallel")]
const APSP_PARALLEL_THRESHOLD: usize = 300;

/// Sequential stand-in for `APSP_PARALLEL_THRESHOLD` when the `parallel`
/// feature is off: no node count can reach it, so the all-pairs sweep always
/// runs on the calling thread.
#[cfg(not(feature = "parallel"))]
const APSP_PARALLEL_THRESHOLD: usize = usize::MAX;

/// All-pairs shortest-path distances on a branchial graph, in node order.
///
/// `dist[i][j]` is the unweighted hop distance from `graph.nodes[i]` to
/// `graph.nodes[j]`. The diagonal is `0.0` and an unreachable pair is
/// `f64::INFINITY` — the convention
/// [`OllivierRicciCurvature`](super::ollivier_ricci::OllivierRicciCurvature)
/// reads, and the reason `null_value` is passed as `f64::INFINITY` rather than
/// rustworkx's own `0.0` example value.
///
/// # Index alignment
///
/// [`BranchialGraph::to_petgraph`] adds nodes in `graph.nodes` order and never
/// removes one, so `node_bound() == node_count()` and `to_index(i) == i`. That
/// is exactly the case in which `distance_matrix` skips its internal node
/// remapping and indexes by `to_index`, so row `i` of the result is
/// `graph.nodes[i]` — no permutation to undo.
///
/// # Determinism
///
/// Bit-reproducible at any size, parallel path included — unlike
/// [`multiway_betweenness`]. `distance_matrix` fills disjoint rows: each
/// per-source BFS writes only into its own row view, and every value written is
/// an integer-valued hop counter rather than an accumulated sum. There is no
/// shared f64 buffer whose summation order could vary with thread scheduling,
/// which is precisely what makes the Brandes accumulation non-reproducible.
///
/// # Cost
///
/// rustworkx expands the BFS frontier with `FixedBitSet` word operations, which
/// is `O(n²/64)` per source regardless of edge count — a win over a queue-based
/// `O(n + m)` sweep on dense graphs and a loss on very sparse ones. Branchial
/// graphs sit on the dense end by construction: nodes are joined when they
/// share *any* ancestor, so a foliation grown from a single root is complete or
/// near-complete at every step
/// ([`BranchialGraph::is_fully_connected`] exists to check exactly that).
///
/// # Allocation
///
/// `distance_matrix` hands back an `ndarray` 2-D array; this copies it into
/// `Vec<Vec<f64>>` at the boundary. The copy is deliberate: it keeps `ndarray`
/// out of both this signature and `Cargo.toml`. `ndarray` is present
/// transitively beneath rustworkx-core, but *declaring* it would version-lock
/// this crate to rustworkx-core's choice of it, and the slim/WASM feature story
/// ("one gate drops the whole rustworkx-core → petgraph + ndarray chain") would
/// stop being true. The copy is `O(n²)` against an `O(n³/64)` sweep, so it does
/// not change the shape of the cost.
pub(crate) fn branchial_distance_matrix(graph: &BranchialGraph) -> Vec<Vec<f64>> {
    let (pg, _) = graph.to_petgraph();
    let matrix = rustworkx_core::shortest_path::distance_matrix(
        &pg,
        APSP_PARALLEL_THRESHOLD,
        // `pg` is already undirected, so `neighbors` yields both directions;
        // `as_undirected` would only chain Incoming with Outgoing redundantly.
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

    /// The all-pairs sweep behind `OllivierRicciCurvature` (#162).
    ///
    /// Three things have to hold at once, and each is a way the rustworkx swap
    /// could have gone wrong silently:
    ///
    /// 1. **`null_value`.** An unreachable pair must read `f64::INFINITY`, the
    ///    sentinel `from_branchial` skips on — not rustworkx's own doc-example
    ///    `0.0`, which would read as "adjacent" and divide by zero.
    /// 2. **The diagonal.** `distance_matrix` initialises every cell to
    ///    `null_value` and only the traversal writes `0.0`, so the diagonal
    ///    being `0.0` is a property of the traversal, not of the fill.
    /// 3. **Index alignment.** The branch ids here are deliberately *not*
    ///    ascending: any export that sorted or hashed nodes rather than keeping
    ///    `graph.nodes` order would permute the rows and still look plausible.
    #[test]
    fn distance_matrix_marks_unreachable_and_keeps_node_order() {
        use super::super::evolution_graph::BranchId;

        let id = |branch: usize| MultiwayNodeId::new(BranchId(branch), 0);
        // Path 7-2-9, plus a disjoint edge 4-1. Branch ids are shuffled so that
        // row order can only come from `nodes`, not from the ids themselves.
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

        // Degenerate shapes the curvature entry point short-circuits on, pinned
        // here because the matrix helper is what would panic if they were not
        // handled: 0×0 and a lone 0.0 diagonal.
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

    /// An empty evolution has nothing to score — that is not a convergence
    /// failure, and both centralities must say so the same way.
    ///
    /// Without the short-circuit in `multiway_katz`, rustworkx's convergence
    /// test (`sum < node_count as f64 * tol`, i.e. `0.0 < 0.0`) never fires, so
    /// it spins `max_iter` times and returns `Ok(None)` — reporting "did not
    /// converge" for a graph with no nodes, and disagreeing with
    /// `multiway_betweenness`, which returns an empty map.
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
        // Katz is L2-normalized over all nodes.
        let norm: f64 = katz.values().map(|v| v * v).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() < 1e-9, "‖x‖₂ = {norm}");
    }
}
