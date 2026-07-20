//! Spectral analysis of branchial graph Laplacians.
//!
//! Computes eigendecomposition of the graph Laplacian L = D − A, exposing:
//! - Algebraic connectivity (λ₂ / Fiedler value)
//! - Spectral gap (λ₂ / `λ_max`)
//! - Connected component count (multiplicity of eigenvalue 0)
//! - Fiedler vector (spectral bisection)
//! - Spectral clustering (k-means on leading eigenvectors)
//!
//! The algebraic-connectivity (λ₂) reducibility/irreducibility proxy is a
//! **catgraph extrapolation** over Gorard's branchial-graph substrate — the
//! cited paper (arXiv:2301.04690) has no spectral/Laplacian content; see
//! `docs/ANCHORS.md`. The reading: higher λ₂ means stronger entanglement
//! between parallel branches.

use nalgebra::{DMatrix, DVector};

use super::branchial::BranchialGraph;
use super::evolution_graph::MultiwayNodeId;

/// Tolerance for floating-point zero detection in eigenvalue comparisons.
const EIGENVALUE_ZERO_THRESHOLD: f64 = 1e-10;

/// Spectral analysis of a branchial graph's Laplacian.
///
/// The graph Laplacian L = D − A encodes the combinatorial structure
/// of branching. Its spectrum reveals:
/// - λ₁ = 0 always (constant eigenvector)
/// - λ₂ = algebraic connectivity (Fiedler value) — catgraph's computational-
///   reducibility proxy (an extrapolation over the branchial substrate, not
///   a Gorard result; see `docs/ANCHORS.md`)
/// - Multiplicity of λ = 0 gives the number of connected components
#[derive(Clone, Debug)]
pub struct BranchialSpectrum {
    /// Eigenvalues of L, sorted ascending (λ₁ ≤ λ₂ ≤ ... ≤ `λ_n`).
    pub eigenvalues: Vec<f64>,
    /// Eigenvectors as columns, same order as eigenvalues.
    /// Column i is the eigenvector for `eigenvalues[i]`.
    pub eigenvectors: DMatrix<f64>,
    /// Node IDs in the order used for matrix construction.
    /// Eigenvectors row i corresponds to `node_order[i]`.
    pub node_order: Vec<MultiwayNodeId>,
}

impl BranchialSpectrum {
    /// Compute spectral decomposition of the branchial graph Laplacian.
    ///
    /// Builds L = D − A as a dense matrix, then applies symmetric
    /// eigendecomposition. Eigenvalues are sorted ascending.
    ///
    /// # Performance
    ///
    /// O(n³) for eigendecomposition. Practical up to ~2000 nodes.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn from_branchial(graph: &BranchialGraph) -> Self {
        let (node_order, adj) = graph.adjacency_matrix();
        let n = node_order.len();

        if n == 0 {
            return Self {
                eigenvalues: Vec::new(),
                eigenvectors: DMatrix::zeros(0, 0),
                node_order,
            };
        }

        // Build Laplacian L = D − A directly as dense matrix
        let laplacian = DMatrix::from_fn(n, n, |i, j| {
            if i == j {
                (0..n).filter(|&k| adj[i][k]).count() as f64
            } else if adj[i][j] {
                -1.0
            } else {
                0.0
            }
        });

        // Symmetric eigendecomposition
        let eigen = nalgebra::SymmetricEigen::new(laplacian);

        // Sort eigenvalues ascending (nalgebra returns unsorted)
        let mut indexed: Vec<(usize, f64)> = eigen
            .eigenvalues
            .iter()
            .enumerate()
            .map(|(i, &v)| (i, v))
            .collect();
        indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let eigenvalues: Vec<f64> = indexed.iter().map(|&(_, v)| v).collect();

        // Reorder eigenvectors to match sorted eigenvalues
        let eigenvectors =
            DMatrix::from_fn(n, n, |row, col| eigen.eigenvectors[(row, indexed[col].0)]);

        Self {
            eigenvalues,
            eigenvectors,
            node_order,
        }
    }

    /// Algebraic connectivity (Fiedler value) — second-smallest eigenvalue.
    ///
    /// Returns 0.0 for disconnected graphs, positive for connected.
    /// Higher values indicate stronger connectivity / more "entangled"
    /// branching.
    #[must_use]
    pub fn algebraic_connectivity(&self) -> f64 {
        self.eigenvalues.get(1).copied().unwrap_or(0.0)
    }

    /// Spectral gap: λ₂ / `λ_max`. Normalized measure of connectivity.
    ///
    /// Returns 0.0 for disconnected or single-node graphs.
    #[must_use]
    pub fn spectral_gap(&self) -> f64 {
        let lambda_2 = self.algebraic_connectivity();
        let lambda_max = self.eigenvalues.last().copied().unwrap_or(0.0);
        if lambda_max.abs() < EIGENVALUE_ZERO_THRESHOLD {
            0.0
        } else {
            lambda_2 / lambda_max
        }
    }

    /// Number of connected components (multiplicity of eigenvalue 0).
    ///
    /// Uses `EIGENVALUE_ZERO_THRESHOLD` (`1e-10`) for floating-point zero detection.
    #[must_use]
    pub fn connected_components(&self) -> usize {
        if self.eigenvalues.is_empty() {
            return 0;
        }
        self.eigenvalues
            .iter()
            .filter(|&&v| v.abs() < EIGENVALUE_ZERO_THRESHOLD)
            .count()
            .max(1)
    }

    /// Fiedler vector — eigenvector for λ₂.
    ///
    /// Signs partition the graph into two clusters (spectral bisection).
    /// Returns `None` for single-node graphs.
    #[must_use]
    pub fn fiedler_vector(&self) -> Option<DVector<f64>> {
        if self.eigenvalues.len() < 2 {
            return None;
        }
        Some(self.eigenvectors.column(1).into())
    }

    /// Spectral clustering into `k` groups via k-means on the first `k`
    /// eigenvectors of L (normalized cuts approximation).
    ///
    /// Returns a `Vec` mapping each node index to a cluster ID `(0..k)`.
    /// Uses simple k-means with deterministic initialization.
    ///
    /// # Panics
    ///
    /// Panics if `k` is 0 or greater than the number of nodes.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn spectral_clustering(&self, k: usize) -> Vec<usize> {
        let n = self.eigenvalues.len();
        assert!(k > 0 && k <= n, "k must be in 1..=n");

        if k == 1 {
            return vec![0; n];
        }

        // Project each node onto eigenvectors 1..k (skip constant eigenvector 0)
        let dim = (k - 1).min(n - 1);
        if dim == 0 {
            return vec![0; n];
        }

        let points: Vec<Vec<f64>> = (0..n)
            .map(|i| (1..=dim).map(|j| self.eigenvectors[(i, j)]).collect())
            .collect();

        // k-means: initialize with first k distinct points
        let mut centroids: Vec<Vec<f64>> = Vec::with_capacity(k);
        for point in &points {
            if centroids.len() >= k {
                break;
            }
            let is_dup = centroids.iter().any(|c| {
                c.iter()
                    .zip(point.iter())
                    .all(|(a, b)| (a - b).abs() < 1e-12)
            });
            if !is_dup {
                centroids.push(point.clone());
            }
        }
        while centroids.len() < k {
            centroids.push(vec![0.0; dim]);
        }

        let mut assignments = vec![0_usize; n];
        for _ in 0..100 {
            // Assign each point to nearest centroid
            let mut changed = false;
            for (i, point) in points.iter().enumerate() {
                let nearest = centroids
                    .iter()
                    .enumerate()
                    .map(|(ci, c)| {
                        let dist_sq: f64 = point
                            .iter()
                            .zip(c.iter())
                            .map(|(a, b)| (a - b).powi(2))
                            .sum();
                        (ci, dist_sq)
                    })
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map_or(0, |(ci, _)| ci);
                if assignments[i] != nearest {
                    assignments[i] = nearest;
                    changed = true;
                }
            }

            if !changed {
                break;
            }

            // Update centroids
            for c in &mut centroids {
                c.fill(0.0);
            }
            let mut counts = vec![0_usize; k];
            for (i, point) in points.iter().enumerate() {
                let ci = assignments[i];
                counts[ci] += 1;
                for (d, &val) in point.iter().enumerate() {
                    centroids[ci][d] += val;
                }
            }
            for (ci, c) in centroids.iter_mut().enumerate() {
                if counts[ci] > 0 {
                    for v in c.iter_mut() {
                        *v /= counts[ci] as f64;
                    }
                }
            }
        }

        assignments
    }
}

#[cfg(test)]
mod tests {
    use super::super::evolution_graph::MultiwayEvolutionGraph;
    use super::*;

    #[test]
    fn single_node_spectrum() {
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        graph.add_root(0);

        let branchial = BranchialGraph::from_evolution_at_step(&graph, 0);
        let spectrum = BranchialSpectrum::from_branchial(&branchial);

        assert_eq!(spectrum.eigenvalues.len(), 1);
        assert!(spectrum.eigenvalues[0].abs() < EIGENVALUE_ZERO_THRESHOLD);
        assert!((spectrum.algebraic_connectivity()).abs() < EIGENVALUE_ZERO_THRESHOLD);
        assert!((spectrum.spectral_gap()).abs() < EIGENVALUE_ZERO_THRESHOLD);
        assert_eq!(spectrum.connected_components(), 1);
        assert!(spectrum.fiedler_vector().is_none());
    }

    #[test]
    fn two_connected_nodes() {
        // K₂: L = [[1, -1], [-1, 1]], eigenvalues = [0, 2]
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let root = graph.add_root(0);
        graph.add_fork(root, vec![(1, (), 0), (2, (), 1)]);

        let branchial = BranchialGraph::from_evolution_at_step(&graph, 1);
        let spectrum = BranchialSpectrum::from_branchial(&branchial);

        assert_eq!(spectrum.eigenvalues.len(), 2);
        assert!(spectrum.eigenvalues[0].abs() < EIGENVALUE_ZERO_THRESHOLD);
        assert!((spectrum.eigenvalues[1] - 2.0).abs() < 1e-9);
        assert!((spectrum.algebraic_connectivity() - 2.0).abs() < 1e-9);
        assert!((spectrum.spectral_gap() - 1.0).abs() < 1e-9);
        assert_eq!(spectrum.connected_components(), 1);
    }

    #[test]
    fn triangle_k3_spectrum() {
        // K₃: L has eigenvalues [0, 3, 3]
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let root = graph.add_root(0);
        graph.add_fork(root, vec![(1, (), 0), (2, (), 1), (3, (), 2)]);

        let branchial = BranchialGraph::from_evolution_at_step(&graph, 1);
        let spectrum = BranchialSpectrum::from_branchial(&branchial);

        assert_eq!(spectrum.eigenvalues.len(), 3);
        assert!(spectrum.eigenvalues[0].abs() < EIGENVALUE_ZERO_THRESHOLD);
        assert!((spectrum.eigenvalues[1] - 3.0).abs() < 1e-9);
        assert!((spectrum.eigenvalues[2] - 3.0).abs() < 1e-9);
        assert!((spectrum.algebraic_connectivity() - 3.0).abs() < 1e-9);
    }

    #[test]
    fn disconnected_graph_two_components() {
        // Two separate roots → two components at step 0
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        graph.add_root(0);
        graph.add_root(1);

        let branchial = BranchialGraph::from_evolution_at_step(&graph, 0);
        let spectrum = BranchialSpectrum::from_branchial(&branchial);

        assert_eq!(spectrum.connected_components(), 2);
        assert!(spectrum.algebraic_connectivity().abs() < EIGENVALUE_ZERO_THRESHOLD);
    }

    #[test]
    fn fiedler_vector_bisects_k2() {
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let root = graph.add_root(0);
        graph.add_fork(root, vec![(1, (), 0), (2, (), 1)]);

        let branchial = BranchialGraph::from_evolution_at_step(&graph, 1);
        let spectrum = BranchialSpectrum::from_branchial(&branchial);

        let fiedler = spectrum
            .fiedler_vector()
            .expect("K₂ should have a Fiedler vector");
        assert_eq!(fiedler.len(), 2);
        // Signs should be opposite (one positive, one negative)
        assert!(
            fiedler[0] * fiedler[1] < 0.0,
            "Fiedler vector should bisect K₂"
        );
    }

    #[test]
    fn spectral_clustering_k2_into_two() {
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let root = graph.add_root(0);
        graph.add_fork(root, vec![(1, (), 0), (2, (), 1)]);

        let branchial = BranchialGraph::from_evolution_at_step(&graph, 1);
        let spectrum = BranchialSpectrum::from_branchial(&branchial);

        let clusters = spectrum.spectral_clustering(2);
        assert_eq!(clusters.len(), 2);
        assert_ne!(clusters[0], clusters[1]);
    }
}
