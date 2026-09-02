//! Ollivier-Ricci curvature backend for branchial graphs.
//!
//! Implements [`DiscreteCurvature`] using the Ollivier-Ricci definition:
//!
//! ```text
//! κ(x, y) = 1 - W₁(μ_x, μ_y) / d(x, y)
//! ```
//!
//! where `μ_x` is the uniform distribution over neighbors of `x`, and
//! `W₁` is the Wasserstein-1 (earth mover's) distance computed by the
//! transportation simplex solver in [`super::wasserstein`].
//!
//! # Curvature interpretation
//!
//! - **κ > 0**: Neighbors of x and y overlap significantly (sphere-like).
//! - **κ = 0**: Neighbors are exactly at the same distance as x, y (flat).
//! - **κ < 0**: Neighbors spread apart further than x, y (saddle-like).

use std::collections::HashMap;
use std::collections::VecDeque;
use std::hash::Hash;

use super::branchial::BranchialGraph;
use super::curvature::{CurvatureFoliation, DiscreteCurvature};
use super::evolution_graph::MultiwayEvolutionGraph;
use super::evolution_graph::MultiwayNodeId;
use super::wasserstein::wasserstein_1;

/// Ollivier-Ricci curvature computed from a branchial graph.
///
/// Each edge (x, y) receives a curvature value κ(x, y), and per-vertex
/// Ricci curvature is the average of incident edge curvatures.
#[derive(Clone, Debug)]
pub struct OllivierRicciCurvature {
    /// Per-edge curvatures: `((u, v), κ)`.
    edge_curvatures: Vec<((usize, usize), f64)>,
    /// Per-vertex Ricci curvature (average of incident edge curvatures).
    vertex_curvatures: Vec<f64>,
    /// Scalar curvature R (normalized sum of vertex curvatures).
    scalar: f64,
    /// Dimension (number of branches / nodes).
    dim: usize,
    /// Time step this curvature was computed for.
    time_step: usize,
}

/// Curvature foliation parameterized by the Ollivier-Ricci backend.
///
/// Convenience alias: each time step carries an [`OllivierRicciCurvature`]
/// computed from the branchial graph at that step.
pub type OllivierFoliation = CurvatureFoliation<OllivierRicciCurvature>;

impl OllivierRicciCurvature {
    /// Ollivier–Ricci curvature of a branchial graph: unweighted hop metric
    /// (rustworkx-core with the `rustworkx` feature, queue BFS otherwise),
    /// uniform neighbour distributions, `κ(x, y) = 1 − W₁(μ_x, μ_y) / d(x, y)`
    /// per edge, vertex Ricci = mean of incident edge curvatures, scalar =
    /// normalised sum of vertex curvatures.
    ///
    /// A pair listed more than once in `branchial.edges`, in either
    /// orientation, is one undirected edge; a self-loop `(a, a)` enters neither
    /// the adjacency nor the scored edges.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::similar_names
    )]
    pub fn from_branchial(branchial: &BranchialGraph) -> Self {
        let n = branchial.nodes.len();

        // Trivial: 0 or 1 node → flat
        if n <= 1 {
            return Self {
                edge_curvatures: Vec::new(),
                vertex_curvatures: vec![0.0; n],
                scalar: 0.0,
                dim: n,
                time_step: branchial.step,
            };
        }

        // --- 1. Node-index mapping + adjacency lists ---
        let idx_of: HashMap<MultiwayNodeId, usize> = branchial
            .nodes
            .iter()
            .enumerate()
            .map(|(i, &node)| (node, i))
            .collect();

        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        for &(a, b) in &branchial.edges {
            if let (Some(&ia), Some(&ib)) = (idx_of.get(&a), idx_of.get(&b)) {
                if ia == ib {
                    continue;
                }
                if !adj[ia].contains(&ib) {
                    adj[ia].push(ib);
                }
                if !adj[ib].contains(&ia) {
                    adj[ib].push(ia);
                }
            }
        }

        // --- 2. All-pairs BFS shortest paths ---
        #[cfg(feature = "rustworkx")]
        let dist = super::branchial_analysis::branchial_distance_matrix(branchial);
        #[cfg(not(feature = "rustworkx"))]
        let dist = all_pairs_bfs(&adj, n);

        // --- 3. Edge curvatures ---
        let mut edge_curvatures: Vec<((usize, usize), f64)> = Vec::new();

        for &(a, b) in &branchial.edges {
            let Some(&ia) = idx_of.get(&a) else {
                continue;
            };
            let Some(&ib) = idx_of.get(&b) else {
                continue;
            };
            let (u, v) = if ia < ib { (ia, ib) } else { (ib, ia) };

            // Skip if already computed (undirected)
            if edge_curvatures.iter().any(|&(e, _)| e == (u, v)) {
                continue;
            }

            let d_uv = dist[u][v];
            if d_uv == 0.0 || d_uv == f64::INFINITY {
                continue;
            }

            let kappa = edge_ollivier_ricci(&adj, &dist, u, v, n);
            edge_curvatures.push(((u, v), kappa));
        }

        // --- 4. Vertex Ricci ---
        let mut vertex_curvatures = vec![0.0_f64; n];
        let mut vertex_degree = vec![0_usize; n];

        for &((u, v), kappa) in &edge_curvatures {
            vertex_curvatures[u] += kappa;
            vertex_curvatures[v] += kappa;
            vertex_degree[u] += 1;
            vertex_degree[v] += 1;
        }
        for i in 0..n {
            if vertex_degree[i] > 0 {
                vertex_curvatures[i] /= vertex_degree[i] as f64;
            }
        }

        // --- 5. Scalar curvature ---
        let scalar = if n > 0 {
            vertex_curvatures.iter().sum::<f64>() / n as f64
        } else {
            0.0
        };

        Self {
            edge_curvatures,
            vertex_curvatures,
            scalar,
            dim: n,
            time_step: branchial.step,
        }
    }

    /// Compute curvature from a multiway evolution graph at a specific step.
    #[must_use]
    pub fn from_evolution_at_step<S: Clone + Hash, T: Clone>(
        graph: &MultiwayEvolutionGraph<S, T>,
        step: usize,
    ) -> Self {
        let branchial = BranchialGraph::from_evolution_at_step(graph, step);
        Self::from_branchial(&branchial)
    }

    /// Whether the branchial structure is geometrically simple.
    ///
    /// A simple structure is flat with dimension <= 2.
    #[must_use]
    pub fn is_geometrically_simple(&self) -> bool {
        self.is_flat() && self.dim <= 2
    }

    /// Branchial complexity as a dimensionless ratio in `[0, 1]`.
    ///
    /// Computed from the absolute scalar curvature normalized against
    /// the theoretical maximum for a graph of this size.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn branchial_complexity(&self) -> f64 {
        if self.dim <= 1 {
            return 0.0;
        }
        // κ ≤ 1 holds definitionally (W₁ ≥ 0 ⇒ κ = 1 − W₁/d ≤ 1), but the
        // two-sided |κ| ≤ 1 clamp is a normalization convention, not a
        // theorem — negative Ollivier curvature on unweighted graphs is not
        // bounded below by −1 in the standard literature ([Oll09], uncached;
        // see docs/ANCHORS.md). The clamp just keeps this ratio in [0, 1].
        self.scalar.abs().min(1.0)
    }
}

impl DiscreteCurvature for OllivierRicciCurvature {
    fn scalar_curvature(&self) -> f64 {
        self.scalar
    }

    fn is_flat(&self) -> bool {
        self.scalar.abs() < 1e-10 && self.edge_curvatures.iter().all(|&(_, k)| k.abs() < 1e-10)
    }

    fn ricci_curvature(&self, vertex: usize) -> f64 {
        self.vertex_curvatures.get(vertex).copied().unwrap_or(0.0)
    }

    fn sectional_curvature(&self, i: usize, j: usize) -> f64 {
        let (u, v) = if i < j { (i, j) } else { (j, i) };
        self.edge_curvatures
            .iter()
            .find(|&&(e, _)| e == (u, v))
            .map_or(0.0, |&(_, k)| k)
    }

    #[allow(clippy::cast_precision_loss)]
    fn irreducibility_indicator(&self) -> f64 {
        // Absolute scalar curvature plus variance of edge curvatures.
        // Higher variance → more heterogeneous branching → more irreducible.
        if self.edge_curvatures.is_empty() {
            return 0.0;
        }

        let abs_scalar = self.scalar.abs();
        let n = self.edge_curvatures.len() as f64;
        let mean: f64 = self.edge_curvatures.iter().map(|&(_, k)| k).sum::<f64>() / n;
        let variance: f64 = self
            .edge_curvatures
            .iter()
            .map(|&(_, k)| (k - mean).powi(2))
            .sum::<f64>()
            / n;

        abs_scalar + variance.sqrt()
    }

    fn dimension(&self) -> usize {
        self.dim
    }

    fn step(&self) -> usize {
        self.time_step
    }
}

impl std::fmt::Display for OllivierRicciCurvature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Ollivier-Ricci Curvature (step {}):", self.time_step)?;
        writeln!(f, "  Dimension: {}", self.dim)?;
        writeln!(f, "  Scalar curvature R: {:.6}", self.scalar)?;
        writeln!(f, "  Edges analyzed: {}", self.edge_curvatures.len())?;
        writeln!(f, "  Is flat: {}", self.is_flat())?;
        writeln!(
            f,
            "  Irreducibility indicator: {:.6}",
            self.irreducibility_indicator()
        )?;
        write!(
            f,
            "  Branchial complexity: {:.4}",
            self.branchial_complexity()
        )
    }
}

impl OllivierFoliation {
    /// Compute a foliation from a full multiway evolution graph.
    #[must_use]
    pub fn from_evolution<S: Clone + Hash, T: Clone>(graph: &MultiwayEvolutionGraph<S, T>) -> Self {
        let max_step = graph.max_step();
        let curvatures: Vec<OllivierRicciCurvature> = (0..=max_step)
            .map(|step| OllivierRicciCurvature::from_evolution_at_step(graph, step))
            .collect();
        Self::from_curvatures(curvatures)
    }
}

// ============================================================================
// Internal helpers
// ============================================================================

/// All-pairs unweighted hop distances, `f64::INFINITY` when unreachable;
/// the `rustworkx`-off stand-in for `branchial_analysis::branchial_distance_matrix`.
#[cfg_attr(feature = "rustworkx", allow(dead_code))]
pub(crate) fn all_pairs_bfs(adj: &[Vec<usize>], n: usize) -> Vec<Vec<f64>> {
    let mut dist = vec![vec![f64::INFINITY; n]; n];

    for (source, row) in dist.iter_mut().enumerate().take(n) {
        row[source] = 0.0;
        let mut queue = VecDeque::new();
        queue.push_back(source);

        while let Some(u) = queue.pop_front() {
            let d = row[u] + 1.0;
            for &v in &adj[u] {
                if d < row[v] {
                    row[v] = d;
                    queue.push_back(v);
                }
            }
        }
    }

    dist
}

/// Compute Ollivier-Ricci curvature for a single edge (u, v).
///
/// `κ(u, v) = 1 - W₁(μ_u, μ_v) / d(u, v)`
///
/// where `μ_x` is the uniform distribution over neighbors of x.
#[allow(clippy::cast_precision_loss)]
fn edge_ollivier_ricci(adj: &[Vec<usize>], dist: &[Vec<f64>], u: usize, v: usize, n: usize) -> f64 {
    let neighbors_u = &adj[u];
    let neighbors_v = &adj[v];

    // Isolated nodes: no neighbors → curvature undefined, treat as 0.
    if neighbors_u.is_empty() || neighbors_v.is_empty() {
        return 0.0;
    }

    // Build support = union of neighbors_u and neighbors_v.
    let mut support: Vec<usize> = Vec::with_capacity(neighbors_u.len() + neighbors_v.len());
    support.extend_from_slice(neighbors_u);
    for &w in neighbors_v {
        if !support.contains(&w) {
            support.push(w);
        }
    }
    support.sort_unstable();

    let s = support.len();

    // Build distributions μ_u and μ_v over the support.
    let mass_u = 1.0 / neighbors_u.len() as f64;
    let mass_v = 1.0 / neighbors_v.len() as f64;

    let mut mu = vec![0.0; s];
    let mut nu = vec![0.0; s];

    let support_idx: HashMap<usize, usize> = support
        .iter()
        .enumerate()
        .map(|(i, &node)| (node, i))
        .collect();

    for &w in neighbors_u {
        if let Some(&idx) = support_idx.get(&w) {
            mu[idx] = mass_u;
        }
    }
    for &w in neighbors_v {
        if let Some(&idx) = support_idx.get(&w) {
            nu[idx] = mass_v;
        }
    }

    // Ground metric restricted to support.
    let _ = n; // n available if needed for full-graph distances
    let ground: Vec<Vec<f64>> = support
        .iter()
        .map(|&i| support.iter().map(|&j| dist[i][j]).collect())
        .collect();

    let w1 = wasserstein_1(&mu, &nu, &ground);
    let d_uv = dist[u][v];

    1.0 - w1 / d_uv
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::evolution_graph::BranchId;
    use super::super::test_topologies::{DISTANCE_SUMMARY, adjacency, topology_fixture};

    fn make_id(branch: usize, step: usize) -> MultiwayNodeId {
        MultiwayNodeId::new(BranchId(branch), step)
    }

    #[allow(clippy::needless_pass_by_value)] // callers pass literal vec![] repeatedly
    fn make_branchial(
        step: usize,
        branch_ids: Vec<usize>,
        edge_pairs: Vec<(usize, usize)>,
    ) -> BranchialGraph {
        let nodes: Vec<MultiwayNodeId> = branch_ids.iter().map(|&b| make_id(b, step)).collect();
        let edges: Vec<(MultiwayNodeId, MultiwayNodeId)> = edge_pairs
            .iter()
            .map(|&(a, b)| (make_id(a, step), make_id(b, step)))
            .collect();
        BranchialGraph { step, nodes, edges }
    }

    /// `K_4` (complete graph on 4 vertices): every edge should have positive
    /// Ollivier-Ricci curvature because neighbors overlap heavily.
    #[test]
    fn complete_graph_k4_has_positive_curvature() {
        let branchial = make_branchial(
            0,
            vec![0, 1, 2, 3],
            vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)],
        );
        let curv = OllivierRicciCurvature::from_branchial(&branchial);

        for &((u, v), kappa) in &curv.edge_curvatures {
            assert!(
                kappa > 0.0,
                "K4 edge ({u},{v}) should have positive curvature, got {kappa}"
            );
        }
        assert!(curv.scalar > 0.0, "K4 scalar curvature should be positive");
    }

    /// Tree with branching: 0-{1,2,3}, 3-{4,5}. Edge (0,3) has κ < 0
    /// because the neighborhoods of 0 and 3 point in opposite directions —
    /// transporting mass from {1,2,3} to {0,4,5} costs more than d(0,3)=1.
    ///
    /// Pinned exactly at −2/3, because that number is the sharpest available
    /// check on the all-pairs pass feeding it (#162). μ₀ sits on {1,2,3} and μ₃
    /// on {0,4,5}, each at mass 1/3, with ground costs `d(1,0)=d(2,0)=d(3,·)=1`
    /// and `d(1,4)=d(1,5)=d(2,4)=d(2,5)=3`. Every optimal assignment routes one
    /// unit through the 3-hop leg, giving `W₁ = (1 + 1 + 3)/3 = 5/3` and
    /// `κ = 1 − 5/3`. It therefore reads distance-2 *and* distance-3 entries of
    /// the matrix: a swapped row, an off-by-one hop count, or a `null_value`
    /// leaking into the support would all move it, where a bare `κ < 0` would
    /// not.
    #[test]
    fn branching_tree_has_negative_curvature() {
        // Graph: 0-1, 0-2, 0-3, 3-4, 3-5
        let branchial = make_branchial(
            0,
            vec![0, 1, 2, 3, 4, 5],
            vec![(0, 1), (0, 2), (0, 3), (3, 4), (3, 5)],
        );
        let curv = OllivierRicciCurvature::from_branchial(&branchial);

        // Find the bridge edge (0, 3)
        let bridge = curv
            .edge_curvatures
            .iter()
            .find(|&&(e, _)| e == (0, 3))
            .map(|&(_, k)| k);

        assert!(
            bridge.is_some(),
            "Edge (0,3) should exist in curvature data"
        );
        let kappa = bridge.unwrap();
        assert!(
            kappa < 0.0,
            "Bridge edge (0,3) should have negative curvature, got {kappa}"
        );
        assert!(
            (kappa - (-2.0 / 3.0)).abs() < 1e-12,
            "Bridge edge (0,3) should be exactly 1 - 5/3, got {kappa}"
        );
    }

    /// A disconnected branchial graph — the shape that puts `f64::INFINITY`
    /// into the distance matrix.
    ///
    /// Two independent "universes": K₂ on {0,1} and K₃ on {2,3,4}. Every
    /// cross-component pair is unreachable, so the whole top-right block of the
    /// matrix is the `null_value`. Curvature must be computed per component and
    /// come out at the values those components would give in isolation —
    /// κ = 0 on the K₂ edge, κ = 1/2 on each K₃ edge (half of μ's mass already
    /// sits on the shared third vertex, so only the other half moves, one hop).
    ///
    /// This is the case that a `null_value` mistake shows up in and nothing
    /// else does: pass rustworkx's own doc-example `0.0` instead of
    /// `f64::INFINITY` and every unreachable pair reads as *adjacent*, which
    /// `from_branchial` would neither skip nor notice.
    #[test]
    fn disconnected_components_curve_independently() {
        let branchial =
            make_branchial(0, vec![0, 1, 2, 3, 4], vec![(0, 1), (2, 3), (2, 4), (3, 4)]);
        let curv = OllivierRicciCurvature::from_branchial(&branchial);

        assert!(curv.sectional_curvature(0, 1).abs() < 1e-12);
        for (u, v) in [(2, 3), (2, 4), (3, 4)] {
            let kappa = curv.sectional_curvature(u, v);
            assert!(
                (kappa - 0.5).abs() < 1e-12,
                "K3 edge ({u},{v}) should be 1/2, got {kappa}"
            );
        }

        // Vertex Ricci averages incident edges; scalar averages over all 5
        // nodes: (0 + 0 + 1/2 + 1/2 + 1/2) / 5.
        assert!(curv.ricci_curvature(0).abs() < 1e-12);
        assert!((curv.ricci_curvature(2) - 0.5).abs() < 1e-12);
        assert!((curv.scalar_curvature() - 0.3).abs() < 1e-12);
        assert_eq!(curv.edge_curvatures.len(), 4);
    }

    /// A single node is trivially flat with dimension 1 and scalar 0.
    #[test]
    fn single_node_is_flat() {
        let branchial = make_branchial(3, vec![42], vec![]);
        let curv = OllivierRicciCurvature::from_branchial(&branchial);

        assert_eq!(curv.dimension(), 1);
        assert!(curv.is_flat());
        assert!((curv.scalar_curvature() - 0.0).abs() < 1e-10);
        assert_eq!(curv.step(), 3);
    }

    /// `K_2`: two nodes connected by one edge.
    /// Each node has exactly one neighbor (the other node). The neighbor
    /// distributions are Dirac masses at opposite endpoints, so
    /// W₁ = d(0,1) = 1, giving κ = 1 - 1/1 = 0.
    #[test]
    fn two_connected_nodes_curvature() {
        let branchial = make_branchial(0, vec![0, 1], vec![(0, 1)]);
        let curv = OllivierRicciCurvature::from_branchial(&branchial);

        assert_eq!(curv.edge_curvatures.len(), 1);
        let kappa = curv.edge_curvatures[0].1;
        assert!(
            kappa.abs() < 1e-10,
            "K2 edge curvature should be 0, got {kappa}"
        );
    }

    /// Dimension and step are correctly propagated.
    #[test]
    fn dimension_and_step_are_correct() {
        let branchial = make_branchial(7, vec![10, 20, 30], vec![(10, 20), (20, 30)]);
        let curv = OllivierRicciCurvature::from_branchial(&branchial);

        assert_eq!(curv.dimension(), 3);
        assert_eq!(curv.step(), 7);
    }

    /// Generic trait conformance: flat (single isolated node).
    #[test]
    fn trait_conformance_flat() {
        use super::super::curvature::test_helpers::assert_trait_conformance;
        let bg = make_branchial(3, vec![0], vec![]);
        let curv = OllivierRicciCurvature::from_branchial(&bg);
        assert_trait_conformance(&curv, 1, 3);
    }

    /// Generic trait conformance: nontrivial `K_4` complete graph.
    #[test]
    fn trait_conformance_nontrivial() {
        use super::super::curvature::test_helpers::assert_trait_conformance;
        let bg = make_branchial(
            0,
            vec![0, 1, 2, 3],
            vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)],
        );
        let curv = OllivierRicciCurvature::from_branchial(&bg);
        assert_trait_conformance(&curv, 4, 0);
    }

    /// The irreducibility indicator is always non-negative.
    #[test]
    fn irreducibility_indicator_is_non_negative() {
        // Test across several graph shapes
        let graphs = [
            make_branchial(0, vec![0], vec![]),
            make_branchial(0, vec![0, 1], vec![(0, 1)]),
            make_branchial(0, vec![0, 1, 2], vec![(0, 1), (1, 2)]),
            make_branchial(
                0,
                vec![0, 1, 2, 3],
                vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)],
            ),
        ];

        for (i, g) in graphs.iter().enumerate() {
            let curv = OllivierRicciCurvature::from_branchial(g);
            assert!(
                curv.irreducibility_indicator() >= 0.0,
                "Graph {i}: indicator should be >= 0, got {}",
                curv.irreducibility_indicator()
            );
        }
    }

    /// Every [`topology_fixture`] has the pinned all-pairs distance summary:
    /// sum of the finite entries, count of the infinite ones, largest finite
    /// entry. On every feature lane.
    #[test]
    fn topology_fixture_distances_are_pinned() {
        for (i, &(want_sum, want_infinite, want_max)) in DISTANCE_SUMMARY.iter().enumerate() {
            let bg = topology_fixture(i);
            let dist = all_pairs_bfs(&adjacency(&bg), bg.nodes.len());

            let finite = dist.iter().flatten().copied().filter(|d| d.is_finite());
            let sum: f64 = finite.clone().sum();
            let max = finite.fold(f64::NEG_INFINITY, f64::max);
            let infinite = dist.iter().flatten().filter(|d| d.is_infinite()).count();

            assert!(
                sum == want_sum,
                "fixture {i}: distance sum {sum}, pinned {want_sum}"
            );
            assert_eq!(infinite, want_infinite, "fixture {i}: infinite entries");
            assert!(
                max == want_max,
                "fixture {i}: largest hop {max}, pinned {want_max}"
            );
        }
    }

    /// Asserts every curvature `from_branchial` exposes for `label` equals the
    /// one `want` exposes: scalar, `ricci_curvature` on `0..n`, and
    /// `sectional_curvature` on each pair of `scored`.
    fn assert_same_curvature(
        got: &OllivierRicciCurvature,
        want: &OllivierRicciCurvature,
        scored: &[(usize, usize)],
        n: usize,
        label: &str,
    ) {
        assert_eq!(
            got.edge_curvatures.len(),
            want.edge_curvatures.len(),
            "{label}: edge curvature count {} vs {}",
            got.edge_curvatures.len(),
            want.edge_curvatures.len()
        );
        assert!(
            got.scalar_curvature() == want.scalar_curvature(),
            "{label}: scalar {} vs {}",
            got.scalar_curvature(),
            want.scalar_curvature()
        );
        for v in 0..n {
            assert!(
                got.ricci_curvature(v) == want.ricci_curvature(v),
                "{label}: vertex {v} Ricci {} vs {}",
                got.ricci_curvature(v),
                want.ricci_curvature(v)
            );
        }
        for &(u, v) in scored {
            assert!(
                got.sectional_curvature(u, v) == want.sectional_curvature(u, v),
                "{label}: edge ({u},{v}) κ {} vs {}",
                got.sectional_curvature(u, v),
                want.sectional_curvature(u, v)
            );
        }
    }

    /// Shape and range of `from_branchial` on [`topology_fixture`] indices 0,
    /// 3, 4, 5 and 6: dimension, per-vertex and per-edge counts, `κ ∈ [-2, 1]`,
    /// and that scalar, every per-vertex and every per-edge curvature is
    /// unchanged both by duplicating every edge in reverse and by appending one
    /// self-loop on an endpoint of the first listed edge. On every feature lane.
    /// Curvature *values* are not pinned against literals.
    #[test]
    fn topology_fixtures_reach_from_branchial() {
        for i in [0, 3, 4, 5, 6] {
            let bg = topology_fixture(i);
            let n = bg.nodes.len();
            let curv = OllivierRicciCurvature::from_branchial(&bg);

            assert_eq!(curv.dimension(), n, "fixture {i}: dimension");
            assert_eq!(
                curv.vertex_curvatures.len(),
                n,
                "fixture {i}: vertex curvature count"
            );

            // Distinct unordered non-loop pairs — exactly the set
            // `from_branchial` scores.
            let idx_of: HashMap<MultiwayNodeId, usize> = bg
                .nodes
                .iter()
                .enumerate()
                .map(|(k, &node)| (node, k))
                .collect();
            let mut scored: Vec<(usize, usize)> = bg
                .edges
                .iter()
                .filter_map(|&(a, b)| {
                    let ia = *idx_of.get(&a)?;
                    let ib = *idx_of.get(&b)?;
                    let (u, v) = if ia < ib { (ia, ib) } else { (ib, ia) };
                    (u != v).then_some((u, v))
                })
                .collect();
            scored.sort_unstable();
            scored.dedup();
            assert_eq!(
                curv.edge_curvatures.len(),
                scored.len(),
                "fixture {i}: edge curvature count"
            );

            for &((u, v), kappa) in &curv.edge_curvatures {
                assert!(
                    kappa.is_finite() && (-2.0..=1.0).contains(&kappa),
                    "fixture {i}: edge ({u},{v}) curvature {kappa} outside [-2, 1]"
                );
            }
            for (v, &kappa) in curv.vertex_curvatures.iter().enumerate() {
                assert!(
                    kappa.is_finite() && (-2.0..=1.0).contains(&kappa),
                    "fixture {i}: vertex {v} curvature {kappa} outside [-2, 1]"
                );
            }

            // Each edge repeated in the opposite orientation: the undirected
            // dedup must collapse the copies back to the same curvatures.
            let mut doubled = bg.clone();
            doubled
                .edges
                .extend(bg.edges.iter().map(|&(a, b)| (b, a)).collect::<Vec<_>>());
            assert_same_curvature(
                &OllivierRicciCurvature::from_branchial(&doubled),
                &curv,
                &scored,
                n,
                &format!("fixture {i}: every edge duplicated in reverse"),
            );

            // One self-loop on an endpoint of the first listed edge — a vertex
            // with an incident scored edge — appended: it contributes no
            // adjacency, so every curvature is the plain one.
            let mut looped = bg.clone();
            let loop_at = bg.edges[0].0;
            looped.edges.push((loop_at, loop_at));
            assert_same_curvature(
                &OllivierRicciCurvature::from_branchial(&looped),
                &curv,
                &scored,
                n,
                &format!("fixture {i}: one self-loop on an edge endpoint appended"),
            );
        }
    }

    /// Every edge of the path [`topology_fixture`] (index 4) has κ = 0 exactly,
    /// under `sectional_curvature`, plain and with every edge duplicated in
    /// reverse.
    #[test]
    fn path_fixture_edges_are_exactly_flat() {
        let bg = topology_fixture(4);
        let n = bg.nodes.len();
        assert_eq!(n, 25, "path fixture node count");
        assert_eq!(bg.edges.len(), 24, "path fixture edge count");

        let mut doubled = bg.clone();
        doubled
            .edges
            .extend(bg.edges.iter().map(|&(a, b)| (b, a)).collect::<Vec<_>>());

        for (label, graph) in [("plain", &bg), ("reverse-duplicated", &doubled)] {
            let curv = OllivierRicciCurvature::from_branchial(graph);
            assert_eq!(curv.edge_curvatures.len(), 24, "{label}: scored edge count");
            for u in 0..24 {
                assert!(
                    curv.edge_curvatures.iter().any(|&(e, _)| e == (u, u + 1)),
                    "{label}: path edge ({u},{}) is not scored",
                    u + 1
                );
                let kappa = curv.sectional_curvature(u, u + 1);
                assert!(
                    kappa == 0.0,
                    "{label}: path edge ({u},{}) κ {kappa}, expected 0",
                    u + 1
                );
            }
            assert!(
                curv.scalar_curvature() == 0.0,
                "{label}: path scalar {}, expected 0",
                curv.scalar_curvature()
            );
        }
    }

    /// The `rustworkx` all-pairs pass and the queue BFS agree bit-for-bit on
    /// every seeded topology fixture.
    #[cfg(feature = "rustworkx")]
    #[test]
    fn distance_matrix_matches_queue_bfs_on_topology_fixtures() {
        use super::super::test_topologies::{TOPOLOGY_FIXTURE_COUNT, assert_same_matrix};

        for i in 0..TOPOLOGY_FIXTURE_COUNT {
            let bg = topology_fixture(i);
            let n = bg.nodes.len();
            assert_same_matrix(
                &super::super::branchial_analysis::branchial_distance_matrix(&bg),
                &all_pairs_bfs(&adjacency(&bg), n),
                &format!("fixture {i}: rustworkx distance matrix vs queue BFS"),
            );
        }
    }
}
