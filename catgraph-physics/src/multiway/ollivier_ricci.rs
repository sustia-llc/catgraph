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

use std::collections::{HashMap, VecDeque};
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
    /// Compute Ollivier-Ricci curvature from a branchial graph.
    ///
    /// Algorithm:
    /// 1. Build adjacency lists and a node-index mapping.
    /// 2. All-pairs BFS shortest paths (unweighted).
    /// 3. For each edge: build uniform neighbor distributions, compute
    ///    `W₁(μ_x, μ_y)`, then `κ(x, y) = 1 - W₁ / d(x, y)`.
    /// 4. Vertex Ricci = average of incident edge curvatures.
    /// 5. Scalar = normalized sum of vertex curvatures.
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
                adj[ia].push(ib);
                adj[ib].push(ia);
            }
        }

        // --- 2. All-pairs BFS shortest paths ---
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

/// All-pairs BFS shortest paths on an unweighted undirected graph.
///
/// Returns a distance matrix `dist[i][j]` where `f64::INFINITY` means
/// unreachable.
fn all_pairs_bfs(adj: &[Vec<usize>], n: usize) -> Vec<Vec<f64>> {
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
}
