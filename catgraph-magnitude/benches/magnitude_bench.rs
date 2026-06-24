//! Criterion benchmarks for `magnitude` on acyclic chain LMs.
//!
//! Three sizes measure the cost of the Möbius-inversion + magnitude pipeline:
//!
//! - `mag_lm_10`   — 10-state acyclic chain at `t = 2.0`.
//! - `mag_lm_100`  — 100-state acyclic chain at `t = 2.0`.
//! - `mag_lm_1000` — 1000-state acyclic chain at `t = 2.0`.
//!
//! **Fixture construction:** uses the same deterministic inline PCG-64-style
//! LCG as `tests/lm_category.rs` (`build_random_tree_lm`).  No `rand` dep.
//!
//! **Complexity:** `magnitude(t)` calls `mobius_function`, which performs
//! Gaussian elimination on the n×n zeta matrix — O(n³) with small constants
//! because the matrix is dense (all prefix-pair distances are finite for a
//! connected chain).  Expect ~8× increase per 2× n at sizes above 100.

use catgraph_magnitude::lm_category::LmCategory;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

/// Build a deterministic forward-chain `n`-state LM using a minimal inline
/// LCG (identical to `tests/lm_category.rs::build_random_tree_lm`).
///
/// State `i` may only transition to states `j > i`.  The last state is the
/// sole terminating state.  All transition rows are renormalised to sum to 1.
#[allow(clippy::cast_precision_loss)]
fn build_chain_lm(n: usize, seed: u64) -> LmCategory {
    let mut state = seed | 1;
    let mut next = || -> f64 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((state >> 33) as f64) / ((1u64 << 31) as f64)
    };

    let names: Vec<String> = (0..n).map(|i| format!("s{i}")).collect();
    let mut m = LmCategory::new(names.clone());
    m.mark_terminating(&names[n - 1]);

    for i in 0..(n - 1) {
        let mut raw: Vec<f64> = Vec::with_capacity(n - i - 1);
        for _ in (i + 1)..n {
            raw.push(next());
        }
        let total: f64 = raw.iter().sum();
        if total < 1e-9 {
            continue;
        }
        for (k, &r) in raw.iter().enumerate() {
            let p = r / total;
            if p > 0.0 {
                m.add_transition(&names[i], &names[i + 1 + k], p).unwrap();
            }
        }
    }
    m
}

fn bench_magnitude(c: &mut Criterion) {
    let mut group = c.benchmark_group("magnitude");

    for &n in &[10usize, 100, 1000] {
        // Pre-build the fixture outside the timed region so we only measure
        // the magnitude computation itself.
        let lm = build_chain_lm(n, 42);
        group.bench_with_input(BenchmarkId::new("mag_lm", n), &lm, |b, m| {
            b.iter(|| {
                m.magnitude(2.0)
                    .expect("zeta_t should be invertible at t=2")
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_magnitude);
criterion_main!(benches);
