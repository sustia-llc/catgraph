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

/// Safety multiplier on the symmetric-eigensolver backward-error bound.
///
/// A backward-stable symmetric eigensolver returns eigenvalues accurate to
/// `O(ε · n · ‖L‖₂)`, and for a Laplacian `‖L‖₂ = λ_max`. The zero test in
/// [`BranchialSpectrum::eigenvalue_zero_tolerance`] is that bound times this
/// factor, which absorbs the constant hidden by the `O(·)` without growing so
/// large that a genuinely small λ₂ is swallowed. (For scale: a path graph on
/// `n` nodes has `λ₂ ≈ π²/n²`, which stays orders of magnitude above the
/// tolerance well past the documented ~2000-node practical bound.)
const EIGENVALUE_ZERO_FACTOR: f64 = 100.0;

/// Hard cap on k-means sweeps in [`BranchialSpectrum::spectral_clustering`].
///
/// Reached only when assignments keep oscillating. In practice the sweep exits
/// on assignment stability (checked first, before the centroid update), with
/// the movement tolerance below as the secondary stop.
const KMEANS_MAX_ITERATIONS: usize = 100;

/// Convergence stop for k-means: the sweep ends once no centroid moved more
/// than this (as a **squared** distance) since the previous sweep.
const KMEANS_TOLERANCE: f64 = 1e-24;

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

        // Degrees are hoisted out of the cell closure below. This is a clarity
        // change, NOT a complexity one: `from_fn` evaluates the closure once per
        // cell, and the old inline `(0..n).filter(|&k| adj[i][k]).count()` sat
        // behind `if i == j`, so the O(n) scan ran on the n diagonal cells only
        // — n·O(n) = Θ(n²), the same Θ(n²) this loop does. #166 called the old
        // form O(n³); it was not. What it *was* is easy to misread as O(n³),
        // since the scan is textually inside an n² closure.
        let degrees: Vec<f64> = (0..n)
            .map(|i| adj[i].iter().filter(|&&edge| edge).count() as f64)
            .collect();

        // Build Laplacian L = D − A directly as dense matrix
        let laplacian = DMatrix::from_fn(n, n, |i, j| {
            if i == j {
                degrees[i]
            } else if adj[i][j] {
                -1.0
            } else {
                0.0
            }
        });

        // Symmetric eigendecomposition
        let eigen = nalgebra::SymmetricEigen::new(laplacian);
        let mut eigenvectors = eigen.eigenvectors;

        // Sort eigenvalues ascending (nalgebra returns unsorted)
        let mut permutation: Vec<usize> = (0..n).collect();
        permutation.sort_by(|&a, &b| {
            eigen.eigenvalues[a]
                .partial_cmp(&eigen.eigenvalues[b])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let eigenvalues: Vec<f64> = permutation.iter().map(|&i| eigen.eigenvalues[i]).collect();

        // Reorder eigenvectors to match. Applying the permutation in place
        // costs the same column moves but allocates no second n×n matrix,
        // unlike rebuilding the whole thing cell by cell.
        permute_columns(&mut eigenvectors, &permutation);

        Self {
            eigenvalues,
            eigenvectors,
            node_order,
        }
    }

    /// Tolerance below which an eigenvalue counts as zero.
    ///
    /// Scales with the spectrum rather than being a fixed absolute constant:
    /// `EIGENVALUE_ZERO_FACTOR · ε · n · max(λ_max, 1)`, mirroring the
    /// `O(ε · n · ‖L‖₂)` backward-error bound of a symmetric eigensolver. A
    /// fixed absolute threshold misclassifies zeros on large or weight-scaled
    /// graphs — and hence miscounts components — because the solver's own error
    /// grows with `n` and `λ_max` while the threshold does not.
    ///
    /// The `max(λ_max, 1)` floor keeps the tolerance positive for an edgeless
    /// graph, where every eigenvalue (λ_max included) is zero and a purely
    /// relative tolerance would collapse to zero and count no components at all.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn eigenvalue_zero_tolerance(&self) -> f64 {
        let n = self.eigenvalues.len();
        if n == 0 {
            return 0.0;
        }
        let lambda_max = self.eigenvalues.last().copied().unwrap_or(0.0).abs();
        EIGENVALUE_ZERO_FACTOR * f64::EPSILON * n as f64 * lambda_max.max(1.0)
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
    /// Returns 0.0 for empty, disconnected, or single-node graphs.
    #[must_use]
    pub fn spectral_gap(&self) -> f64 {
        // An empty spectrum has a zero tolerance (nothing to scale against), so
        // the `< tolerance` test below cannot catch its λ_max = 0 and the
        // division would yield 0/0 = NaN.
        if self.eigenvalues.is_empty() {
            return 0.0;
        }
        let lambda_2 = self.algebraic_connectivity();
        let lambda_max = self.eigenvalues.last().copied().unwrap_or(0.0);
        if lambda_max.abs() < self.eigenvalue_zero_tolerance() {
            0.0
        } else {
            lambda_2 / lambda_max
        }
    }

    /// Number of connected components (multiplicity of eigenvalue 0).
    ///
    /// Zero detection uses the spectrum-scaled
    /// [`eigenvalue_zero_tolerance`](Self::eigenvalue_zero_tolerance).
    #[must_use]
    pub fn connected_components(&self) -> usize {
        if self.eigenvalues.is_empty() {
            return 0;
        }
        let tolerance = self.eigenvalue_zero_tolerance();
        self.eigenvalues
            .iter()
            .filter(|&&v| v.abs() < tolerance)
            .count()
            .max(1)
    }

    /// Fiedler vector — eigenvector for λ₂.
    ///
    /// Signs partition the graph into two clusters (spectral bisection).
    /// Returns `None` for single-node graphs.
    ///
    /// This **copies** the column into an owned [`DVector`]. Callers that only
    /// read the vector can borrow it instead — `spectrum.eigenvectors.column(1)`
    /// is the same data as a zero-copy view.
    #[must_use]
    pub fn fiedler_vector(&self) -> Option<DVector<f64>> {
        if self.eigenvalues.len() < 2 {
            return None;
        }
        Some(self.eigenvectors.column(1).into())
    }

    /// Spectral clustering: k-means on eigenvectors `1..=k` of L, returning a
    /// cluster id in `0..k` per node index. Deterministic: farthest-first
    /// seeds, ties to the lowest index, stop on stable assignment, centroid
    /// movement below `1e-24` (squared), or 100 sweeps; empty clusters are
    /// reseeded. Fewer than `k` labels may be returned when the embedding has
    /// fewer than `k` distinct points.
    ///
    /// # Panics
    ///
    /// Panics if `k` is 0 or greater than the number of nodes.
    #[must_use]
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

        let mut centroids = farthest_first_init(&points, k);
        let mut assignments = vec![usize::MAX; n];

        for _ in 0..KMEANS_MAX_ITERATIONS {
            // Assign each point to nearest centroid
            let mut changed = false;
            for (i, point) in points.iter().enumerate() {
                let nearest = nearest_centroid(point, &centroids);
                if assignments[i] != nearest {
                    assignments[i] = nearest;
                    changed = true;
                }
            }

            if !changed {
                break;
            }

            let previous = centroids.clone();
            update_centroids(&points, &assignments, &mut centroids);
            if max_centroid_shift(&previous, &centroids) <= KMEANS_TOLERANCE {
                break;
            }
        }

        assignments
    }
}

/// Apply a column permutation to `matrix` in place.
///
/// `permutation[i]` is the index of the column that must end up at position
/// `i`. Applied by walking the permutation's cycles, which costs
/// `n − (number of cycles)` swaps — the optimum — and allocates no second
/// matrix. (Individual columns may move more than once: within a `k`-cycle the
/// carried column is swapped `k − 1` times as it travels to its slot.)
fn permute_columns(matrix: &mut DMatrix<f64>, permutation: &[usize]) {
    let n = permutation.len();
    let mut placed = vec![false; n];

    for start in 0..n {
        if placed[start] {
            continue;
        }
        let mut current = start;
        loop {
            placed[current] = true;
            let source = permutation[current];
            if placed[source] {
                break;
            }
            matrix.swap_columns(current, source);
            current = source;
        }
    }
}

/// Squared Euclidean distance between two equal-length points.
fn squared_distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum()
}

/// Index of the centroid nearest to `point`; ties break toward the lowest index.
fn nearest_centroid(point: &[f64], centroids: &[Vec<f64>]) -> usize {
    let mut best = 0;
    let mut best_distance = f64::INFINITY;
    for (index, centroid) in centroids.iter().enumerate() {
        let distance = squared_distance(point, centroid);
        if distance < best_distance {
            best = index;
            best_distance = distance;
        }
    }
    best
}

/// Deterministic k-means seeding by farthest-first traversal (Gonzalez).
///
/// The first seed is `points[0]`; each subsequent seed is the point whose
/// distance to the nearest already-chosen seed is largest, with ties broken
/// toward the lowest index. This is the deterministic analogue of k-means++
/// (which samples that same distance distribution at random) and replaces the
/// previous "first `k` pairwise-distinct points" rule, which could seed every
/// centroid inside one dense cluster and leave the rest of the spectrum
/// unrepresented.
///
/// Fewer than `k` distinct points means some seeds necessarily coincide, and
/// the duplicates are kept as-is. Reseeding cannot rescue that case — identical
/// points always land in the same cluster, so the number of non-empty clusters
/// can never exceed the number of distinct points. The surplus clusters simply
/// stay empty; see [`BranchialSpectrum::spectral_clustering`].
fn farthest_first_init(points: &[Vec<f64>], k: usize) -> Vec<Vec<f64>> {
    let mut centroids = Vec::with_capacity(k);
    centroids.push(points[0].clone());

    // Distance from each point to the nearest seed chosen so far.
    let mut nearest: Vec<f64> = points
        .iter()
        .map(|p| squared_distance(p, &centroids[0]))
        .collect();

    while centroids.len() < k {
        let mut best = 0;
        let mut best_distance = f64::NEG_INFINITY;
        for (index, &distance) in nearest.iter().enumerate() {
            if distance > best_distance {
                best = index;
                best_distance = distance;
            }
        }
        centroids.push(points[best].clone());

        let seed = &centroids[centroids.len() - 1];
        for (index, point) in points.iter().enumerate() {
            nearest[index] = nearest[index].min(squared_distance(point, seed));
        }
    }

    centroids
}

/// Recompute each centroid as the mean of its assigned points.
///
/// A cluster that received no points is **reseeded** onto the point currently
/// farthest from its own centroid (ties toward the lowest index) instead of
/// being left at a stale or zero centroid, which would keep it permanently
/// empty and silently return fewer than `k` clusters.
///
/// This rescues clusters emptied by a bad centroid, which is the common case.
/// It cannot rescue a cluster that is empty because the embedding has fewer
/// than `k` distinct points — there, every donor sits at distance zero and the
/// reseeded centroid just duplicates an existing one.
#[allow(clippy::cast_precision_loss)]
fn update_centroids(points: &[Vec<f64>], assignments: &[usize], centroids: &mut [Vec<f64>]) {
    for centroid in centroids.iter_mut() {
        centroid.fill(0.0);
    }

    let mut counts = vec![0_usize; centroids.len()];
    for (point, &cluster) in points.iter().zip(assignments.iter()) {
        counts[cluster] += 1;
        for (slot, &value) in centroids[cluster].iter_mut().zip(point.iter()) {
            *slot += value;
        }
    }
    for (centroid, &count) in centroids.iter_mut().zip(counts.iter()) {
        if count > 0 {
            for value in centroid.iter_mut() {
                *value /= count as f64;
            }
        }
    }

    // Reseed empty clusters, one at a time so two of them cannot claim the
    // same donor point.
    let mut claimed: Vec<usize> = Vec::new();
    for cluster in 0..centroids.len() {
        if counts[cluster] > 0 {
            continue;
        }
        let mut donor = None;
        let mut donor_distance = f64::NEG_INFINITY;
        for (index, point) in points.iter().enumerate() {
            if claimed.contains(&index) {
                continue;
            }
            let distance = squared_distance(point, &centroids[assignments[index]]);
            if distance > donor_distance {
                donor = Some(index);
                donor_distance = distance;
            }
        }
        if let Some(index) = donor {
            centroids[cluster].clone_from(&points[index]);
            claimed.push(index);
        }
    }
}

/// Largest squared distance any centroid moved between two sweeps.
fn max_centroid_shift(previous: &[Vec<f64>], current: &[Vec<f64>]) -> f64 {
    previous
        .iter()
        .zip(current.iter())
        .map(|(before, after)| squared_distance(before, after))
        .fold(0.0, f64::max)
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

        let tolerance = spectrum.eigenvalue_zero_tolerance();
        assert_eq!(spectrum.eigenvalues.len(), 1);
        assert!(spectrum.eigenvalues[0].abs() < tolerance);
        assert!((spectrum.algebraic_connectivity()).abs() < tolerance);
        assert!((spectrum.spectral_gap()).abs() < tolerance);
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
        assert!(spectrum.eigenvalues[0].abs() < spectrum.eigenvalue_zero_tolerance());
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
        assert!(spectrum.eigenvalues[0].abs() < spectrum.eigenvalue_zero_tolerance());
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
        assert!(spectrum.algebraic_connectivity().abs() < spectrum.eigenvalue_zero_tolerance());
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

    // --- eigenvector permutation (finding 2) -------------------------------

    #[test]
    fn permute_columns_applies_a_three_cycle() {
        // Columns tagged by their first entry so the move is readable.
        let mut m = DMatrix::from_row_slice(2, 3, &[0.0, 1.0, 2.0, 10.0, 11.0, 12.0]);
        // Position 0 takes old column 2, position 1 takes old 0, position 2 old 1.
        permute_columns(&mut m, &[2, 0, 1]);

        assert_eq!(m[(0, 0)], 2.0);
        assert_eq!(m[(0, 1)], 0.0);
        assert_eq!(m[(0, 2)], 1.0);
        assert_eq!(m[(1, 0)], 12.0);
        assert_eq!(m[(1, 1)], 10.0);
        assert_eq!(m[(1, 2)], 11.0);
    }

    #[test]
    fn permute_columns_is_identity_on_the_identity_permutation() {
        let original = DMatrix::from_row_slice(2, 3, &[0.0, 1.0, 2.0, 10.0, 11.0, 12.0]);
        let mut m = original.clone();
        permute_columns(&mut m, &[0, 1, 2]);
        assert_eq!(m, original);
    }

    #[test]
    fn permute_columns_handles_disjoint_cycles() {
        // Two independent 2-cycles: (0 1) and (2 3).
        let mut m = DMatrix::from_row_slice(1, 4, &[0.0, 1.0, 2.0, 3.0]);
        permute_columns(&mut m, &[1, 0, 3, 2]);
        assert_eq!(m.as_slice(), &[1.0, 0.0, 3.0, 2.0]);
    }

    #[test]
    fn eigenvectors_stay_paired_with_their_eigenvalues() {
        // K₃'s Laplacian is degenerate (λ = 3 twice), so the solver's raw order
        // is not the sorted order — the permutation must carry the vectors with
        // it. Check L·vᵢ = λᵢ·vᵢ column by column after sorting.
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let root = graph.add_root(0);
        graph.add_fork(root, vec![(1, (), 0), (2, (), 1), (3, (), 2)]);

        let branchial = BranchialGraph::from_evolution_at_step(&graph, 1);
        let spectrum = BranchialSpectrum::from_branchial(&branchial);

        let n = spectrum.eigenvalues.len();
        let (_, adj) = branchial.adjacency_matrix();
        let laplacian = DMatrix::from_fn(n, n, |i, j| {
            if i == j {
                adj[i].iter().filter(|&&e| e).count() as f64
            } else if adj[i][j] {
                -1.0
            } else {
                0.0
            }
        });

        for (i, &lambda) in spectrum.eigenvalues.iter().enumerate() {
            let v = spectrum.eigenvectors.column(i);
            let residual = (&laplacian * v) - v * lambda;
            assert!(
                residual.norm() < 1e-9,
                "column {i} is not the eigenvector for λ = {lambda}"
            );
        }
    }

    // --- zero tolerance (finding 3) ----------------------------------------

    #[test]
    fn zero_tolerance_scales_with_lambda_max_and_n() {
        let small = BranchialSpectrum {
            eigenvalues: vec![0.0, 1.0, 2.0],
            eigenvectors: DMatrix::zeros(3, 3),
            node_order: Vec::new(),
        };
        let large = BranchialSpectrum {
            eigenvalues: vec![0.0, 1.0, 1.0e6],
            eigenvectors: DMatrix::zeros(3, 3),
            node_order: Vec::new(),
        };

        // Same n, spectrum scaled up by ~5e5 — the tolerance must follow, which
        // a fixed absolute 1e-10 would not.
        assert!(large.eigenvalue_zero_tolerance() > small.eigenvalue_zero_tolerance() * 1.0e5);

        // ...and it must still be far below any eigenvalue that is genuinely
        // nonzero, so components are not over-counted.
        assert!(large.eigenvalue_zero_tolerance() < 1.0);
        assert_eq!(large.connected_components(), 1);
    }

    #[test]
    fn all_zero_spectrum_counts_every_node_as_a_component() {
        // λ_max = 0 — a purely relative tolerance would collapse to zero here
        // and count nothing; the max(λ_max, 1) floor keeps it positive.
        let spectrum = BranchialSpectrum {
            eigenvalues: vec![0.0; 4],
            eigenvectors: DMatrix::zeros(4, 4),
            node_order: Vec::new(),
        };
        assert!(spectrum.eigenvalue_zero_tolerance() > 0.0);
        assert_eq!(spectrum.connected_components(), 4);
        assert!((spectrum.spectral_gap()).abs() < f64::EPSILON);
    }

    #[test]
    fn empty_spectrum_has_zero_tolerance_and_no_components() {
        let spectrum = BranchialSpectrum {
            eigenvalues: Vec::new(),
            eigenvectors: DMatrix::zeros(0, 0),
            node_order: Vec::new(),
        };
        assert!((spectrum.eigenvalue_zero_tolerance()).abs() < f64::EPSILON);
        assert_eq!(spectrum.connected_components(), 0);
        // The tolerance is exactly zero here, so the `λ_max < tolerance` guard
        // in spectral_gap cannot fire — without its own empty check the
        // division below would be 0/0.
        let gap = spectrum.spectral_gap();
        assert!(
            !gap.is_nan(),
            "spectral_gap must not return NaN on an empty spectrum"
        );
        assert!(gap.abs() < f64::EPSILON);
    }

    #[test]
    fn empty_graph_spectral_gap_is_zero_not_nan() {
        // Same path, reached through the public constructor rather than a
        // hand-built struct: an evolution graph with no nodes.
        let graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let branchial = BranchialGraph::from_evolution_at_step(&graph, 0);
        let spectrum = BranchialSpectrum::from_branchial(&branchial);

        assert!(spectrum.eigenvalues.is_empty());
        assert!(!spectrum.spectral_gap().is_nan());
        assert!((spectrum.spectral_gap()).abs() < f64::EPSILON);
    }

    // --- k-means (finding 6) -----------------------------------------------

    #[test]
    fn farthest_first_spreads_seeds_across_clusters() {
        // Three tight blobs. "First k distinct points" would seed all three
        // centroids inside the first blob; farthest-first must pick one per blob.
        let points = vec![
            vec![0.0],
            vec![0.001],
            vec![0.002],
            vec![10.0],
            vec![10.001],
            vec![100.0],
        ];
        let centroids = farthest_first_init(&points, 3);
        assert_eq!(centroids.len(), 3);

        let mut seeds: Vec<f64> = centroids.iter().map(|c| c[0]).collect();
        seeds.sort_by(|a, b| a.partial_cmp(b).expect("finite seeds"));
        assert!(seeds[0] < 1.0, "one seed in the blob at 0");
        assert!((seeds[1] - 10.0).abs() < 1.0, "one seed in the blob at 10");
        assert!(
            (seeds[2] - 100.0).abs() < 1.0,
            "one seed in the blob at 100"
        );
    }

    #[test]
    fn update_centroids_reseeds_an_empty_cluster() {
        let points = vec![vec![0.0], vec![0.0], vec![50.0]];
        // Everything assigned to cluster 0 — cluster 1 is empty.
        let assignments = vec![0, 0, 0];
        let mut centroids = vec![vec![0.0], vec![999.0]];

        update_centroids(&points, &assignments, &mut centroids);

        // Cluster 0 is the mean of its three points...
        assert!((centroids[0][0] - 50.0 / 3.0).abs() < 1e-12);
        // ...and cluster 1 was moved onto the outlier, not left at 999.
        assert!((centroids[1][0] - 50.0).abs() < 1e-12);
    }

    #[test]
    fn clustering_k3_yields_three_distinct_clusters() {
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let root = graph.add_root(0);
        graph.add_fork(root, vec![(1, (), 0), (2, (), 1), (3, (), 2)]);

        let branchial = BranchialGraph::from_evolution_at_step(&graph, 1);
        let spectrum = BranchialSpectrum::from_branchial(&branchial);

        let clusters = spectrum.spectral_clustering(3);
        assert_eq!(clusters.len(), 3);
        let distinct: std::collections::BTreeSet<usize> = clusters.iter().copied().collect();
        assert_eq!(
            distinct.len(),
            3,
            "empty-cluster reseeding should keep all three clusters populated"
        );
    }

    #[test]
    fn clustering_is_deterministic_across_runs() {
        let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
        let root = graph.add_root(0);
        graph.add_fork(root, vec![(1, (), 0), (2, (), 1), (3, (), 2)]);

        let branchial = BranchialGraph::from_evolution_at_step(&graph, 1);
        let spectrum = BranchialSpectrum::from_branchial(&branchial);

        let first = spectrum.spectral_clustering(2);
        for _ in 0..4 {
            assert_eq!(spectrum.spectral_clustering(2), first);
        }
    }
}
