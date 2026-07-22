//! Benchmark: `Vec<Vec<f64>>` vs `DMatrix<f64>` for Wasserstein distance matrix.
//!
//! Evaluates whether switching to nalgebra `DMatrix` improves cache locality
//! for the transportation simplex solver. Decision criteria:
//! - ≥15% faster at size 100+: create follow-up refactor task
//! - <15% faster or slower: no action

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use nalgebra::DMatrix;

use catgraph_physics::multiway::wasserstein_1;
use catgraph_testutil::Lcg;

/// Build a deterministic distance matrix and uniform distributions of size `n`.
#[allow(clippy::cast_precision_loss)]
fn make_test_data(n: usize) -> (Vec<f64>, Vec<f64>, Vec<Vec<f64>>) {
    // Simple deterministic PRNG (LCG). This bench historically used increment 1
    // (not the standard MMIX increment the other call sites use); preserved via
    // `with_increment(42, 1)` to keep the stream byte-identical (#33).
    let mut rng = Lcg::with_increment(42, 1);

    let mass = 1.0 / n as f64;
    let mu = vec![mass; n];
    let nu = vec![mass; n];

    // Symmetric distance matrix
    let mut dist = vec![vec![0.0; n]; n];
    #[allow(
        clippy::needless_range_loop,
        reason = "each iteration writes the transpose entry dist[j][i] as well as dist[i][j]; iter_mut/enumerate cannot express the cross-row symmetric write"
    )]
    for i in 0..n {
        for j in (i + 1)..n {
            let d = rng.next_f64() * 10.0 + 0.1;
            dist[i][j] = d;
            dist[j][i] = d;
        }
    }

    (mu, nu, dist)
}

/// Copy of `wasserstein_1` that takes `DMatrix` — converts to `Vec<Vec<f64>>` internally.
/// Measures conversion overhead + any cache benefit from `DMatrix` construction.
#[allow(clippy::cast_precision_loss)]
fn wasserstein_1_via_dmatrix(mu: &[f64], nu: &[f64], distance: &DMatrix<f64>) -> f64 {
    let m = mu.len();
    let n = nu.len();
    let dist_vecs: Vec<Vec<f64>> = (0..m)
        .map(|i| (0..n).map(|j| distance[(i, j)]).collect())
        .collect();
    wasserstein_1(mu, nu, &dist_vecs)
}

fn wasserstein_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("wasserstein_distance_matrix");

    for &size in &[10, 50, 100, 200] {
        let (mu, nu, dist_vecs) = make_test_data(size);
        let dist_dmatrix = DMatrix::from_fn(size, size, |i, j| dist_vecs[i][j]);

        group.bench_with_input(BenchmarkId::new("vec_of_vecs", size), &size, |b, _| {
            b.iter(|| wasserstein_1(&mu, &nu, &dist_vecs));
        });

        group.bench_with_input(
            BenchmarkId::new("dmatrix_via_convert", size),
            &size,
            |b, _| {
                b.iter(|| wasserstein_1_via_dmatrix(&mu, &nu, &dist_dmatrix));
            },
        );
    }

    group.finish();
}

criterion_group!(benches, wasserstein_benchmark);
criterion_main!(benches);
