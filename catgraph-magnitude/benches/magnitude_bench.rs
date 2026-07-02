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

use std::hint::black_box;

use catgraph_magnitude::coalition_eval::CoalitionEvaluator;
use catgraph_magnitude::coalition_value;
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

/// Number of candidates swept against the fixed coalition in the incremental
/// benchmark (per the #31 brief: ~8 candidates per sweep).
const SWEEP_CANDIDATES: usize = 8;

/// `(agents, couplings, members)` for a coalition fixture — a `usize` agent
/// pool with a sparse coupling table and a member-index subset.
type CoalitionFixture = (Vec<usize>, Vec<(usize, usize, f64)>, Vec<usize>);

/// Deterministic coalition fixture: `m` members plus [`SWEEP_CANDIDATES`]
/// candidate agents, over a dense random coupling table (same inline LCG as
/// [`build_chain_lm`]). Agents are `usize` indices `0..(m + SWEEP_CANDIDATES)`;
/// members are `0..m`; candidates are `m..(m + SWEEP_CANDIDATES)`.
///
/// Couplings avoid `1.0` so no skeletal merge collapses the members (keeping
/// the `k = m` skeleton the fast path is measured on).
fn build_coalition_fixture(m: usize, seed: u64) -> CoalitionFixture {
    let n = m + SWEEP_CANDIDATES;
    let mut state = seed | 1;
    let mut next = || -> f64 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        // Map into (0.05, 0.95] — bounded away from 0 (dense closure) and 1
        // (no perfect-coupling merges).
        0.05 + 0.90 * ((state >> 33) as f64) / ((1u64 << 31) as f64)
    };

    let agents: Vec<usize> = (0..n).collect();
    let mut couplings: Vec<(usize, usize, f64)> = Vec::new();
    for i in 0..n {
        for j in 0..n {
            if i != j {
                couplings.push((i, j, next()));
            }
        }
    }
    let members: Vec<usize> = (0..m).collect();
    (agents, couplings, members)
}

/// #31 incremental-magnitude benchmark: a fixed coalition `S`, sweep
/// [`SWEEP_CANDIDATES`] candidates, comparing
/// - **fresh**: two full [`coalition_value`] evaluations per candidate
///   (`Mag(S)` recomputed each time + `Mag(S ∪ {x})`), vs
/// - **evaluator**: one [`CoalitionEvaluator`] built once, then
///   [`CoalitionEvaluator::value_with`] per candidate.
fn bench_incremental(c: &mut Criterion) {
    let mut group = c.benchmark_group("coalition_incremental");

    for &m in &[4usize, 8, 16] {
        let (agents, couplings, members) = build_coalition_fixture(m, 0xA11CE);
        let candidates: Vec<usize> = (m..(m + SWEEP_CANDIDATES)).collect();

        // (a) Fresh: two full evaluations per candidate.
        group.bench_with_input(BenchmarkId::new("fresh_sweep", m), &m, |b, _| {
            b.iter(|| {
                let mut acc = 0.0_f64;
                for &cand in &candidates {
                    let base = coalition_value(&agents, &couplings, &members)
                        .expect("base coalition must evaluate");
                    let mut members_x = members.clone();
                    members_x.push(cand);
                    let with = coalition_value(&agents, &couplings, &members_x)
                        .expect("candidate coalition must evaluate");
                    acc += base + with;
                }
                black_box(acc)
            });
        });

        // (b) Evaluator: build once, incremental value_with per candidate.
        group.bench_with_input(BenchmarkId::new("evaluator_sweep", m), &m, |b, _| {
            b.iter(|| {
                let ev = CoalitionEvaluator::new(&agents, &couplings, &members, 1.0)
                    .expect("base coalition must evaluate");
                let mut acc = ev.base_value();
                for &cand in &candidates {
                    acc += ev.value_with(cand).expect("candidate must evaluate");
                }
                black_box(acc)
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_magnitude, bench_incremental);
criterion_main!(benches);
