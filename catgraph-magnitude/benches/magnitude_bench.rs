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

use catgraph_magnitude::coalition::coalition_magnitude_from_couplings;
use catgraph_magnitude::coalition_eval::{CoalitionEvaluator, EvalScratch};
use catgraph_magnitude::coalition_value;
use catgraph_magnitude::lm_category::LmCategory;
use catgraph_testutil::Lcg;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

// NOTE: `EvalScratch` + the `evaluator_rebuild/{fresh,new}` and
// `coalition_value_with/{hit,hit_scratch}` benches below are #33; the scratch
// surface must exist in `coalition_eval` for this bench to compile.

/// Build a deterministic forward-chain `n`-state LM using a minimal inline
/// LCG (identical to `tests/lm_category.rs::build_random_tree_lm`).
///
/// State `i` may only transition to states `j > i`.  The last state is the
/// sole terminating state.  All transition rows are renormalised to sum to 1.
fn build_chain_lm(n: usize, seed: u64) -> LmCategory {
    // `| 1` seed prep stays at the call site (#33).
    let mut rng = Lcg::new(seed | 1);

    let names: Vec<String> = (0..n).map(|i| format!("s{i}")).collect();
    let mut m = LmCategory::new(names.clone());
    m.mark_terminating(&names[n - 1]);

    for i in 0..(n - 1) {
        let mut raw: Vec<f64> = Vec::with_capacity(n - i - 1);
        for _ in (i + 1)..n {
            raw.push(rng.next_f64());
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
    // `| 1` seed prep stays at the call site (#33).
    let mut rng = Lcg::new(seed | 1);

    let agents: Vec<usize> = (0..n).collect();
    let mut couplings: Vec<(usize, usize, f64)> = Vec::new();
    for i in 0..n {
        for j in 0..n {
            if i != j {
                // Map a raw draw into (0.05, 0.95] — bounded away from 0 (dense
                // closure) and 1 (no perfect-coupling merges). Kept as call-site
                // arithmetic on `next_f64()`: the divisor `2^31` is an exact
                // power of two, so `0.90 * raw / 2^31` and `0.90 * (raw / 2^31)`
                // are bit-identical, but a `next_in(0.05, 0.95)` helper would
                // compute `0.95 - 0.05 != 0.90` and change the stream (#33).
                couplings.push((i, j, 0.05 + 0.90 * rng.next_f64()));
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

/// A fast-path coalition fixture (#33): an `m`-member forward chain
/// (`i → i+1` at `0.5`, so the skeleton is full — `k = m` — and ζ is
/// non-singular) plus one candidate (index `m`) weakly single-coupled to member
/// `0` (`0.2` both ways). That candidate neither improves an interior member
/// path nor merges a skeletal class, so `value_with` takes the **fast** (bordered
/// Schur) path — the branch the scratch buffers accelerate. The path is asserted
/// in the `coalition_eval` unit test `bench_fast_path_fixture_is_fast` on the same
/// construction.
///
/// Returns `(agents, couplings, members, candidate)`.
#[allow(clippy::type_complexity)]
fn build_fast_path_fixture(m: usize) -> (Vec<usize>, Vec<(usize, usize, f64)>, Vec<usize>, usize) {
    let agents: Vec<usize> = (0..=m).collect(); // 0..m members + candidate `m`
    let mut couplings: Vec<(usize, usize, f64)> = Vec::new();
    for i in 0..(m - 1) {
        couplings.push((i, i + 1, 0.5));
    }
    couplings.push((0, m, 0.2)); // member 0 → candidate
    couplings.push((m, 0, 0.2)); // candidate → member 0
    let members: Vec<usize> = (0..m).collect();
    (agents, couplings, members, m)
}

/// #33 (B) — isolate the `CoalitionEvaluator::new` rebuild cost against the fresh
/// `coalition_magnitude_from_couplings` path on the *same* fixture, to reproduce
/// (or refute) the koalisi K6 report of a ~10–15× `new()`/fresh ratio attributed
/// to cache extraction. Both are measured at `m = 8` and `m = 16` on
/// [`build_coalition_fixture`].
///
/// **Result (2026-07-22, #33): the 10–15× ratio does NOT reproduce.** Measured
/// on this dense fixture:
/// - `fresh/8` ≈ 54 µs, `new/8` ≈ 57 µs → **1.05×**;
/// - `fresh/16` ≈ 179 µs, `new/16` ≈ 188 µs → **1.05×**.
///
/// A throwaway sweep over `m ∈ {3,4,6,8,16}` on both dense and sparse coupling
/// tables held the ratio in **1.02–1.16×** everywhere, and `fresh` never fell
/// near the ~2 µs koalisi reported (the smallest was ~5.8 µs at sparse `m = 3`).
/// Cache extraction (closed-table copy + reps + μ materialization +
/// weighting/coweighting + base_mag) is ~5% of `new()`; both routes are dominated
/// by the shared O(m³) closure + O(k³) inversion. The koalisi 27–30 µs-vs-2 µs
/// gap is therefore a consumer-side measurement artifact (its "fresh" baseline is
/// not the same-coalition `coalition_magnitude_from_couplings`), NOT the cache
/// extraction — so the #33 (B) optimization was correctly a no-op (do not
/// optimize a phantom). This bench stays as the standing apples-to-apples guard.
fn bench_evaluator_new(c: &mut Criterion) {
    let mut group = c.benchmark_group("evaluator_rebuild");

    for &m in &[8usize, 16] {
        let (agents, couplings, members) = build_coalition_fixture(m, 0xA11CE);

        // Fresh `Mag(S)` — build category + restrict-close-skeletalize + invert.
        group.bench_with_input(BenchmarkId::new("fresh", m), &m, |b, _| {
            b.iter(|| {
                coalition_magnitude_from_couplings(&agents, &couplings, &members, 1.0)
                    .expect("base coalition must evaluate")
            });
        });

        // `new()` — the same fresh work PLUS the cache extraction (closed table
        // copy, reps, μ materialization, weighting/coweighting, base_mag).
        group.bench_with_input(BenchmarkId::new("new", m), &m, |b, _| {
            b.iter(|| {
                CoalitionEvaluator::new(&agents, &couplings, &members, 1.0)
                    .expect("base coalition must evaluate")
            });
        });
    }

    group.finish();
}

/// #33 (A) — one prebuilt evaluator, a single **fast-path** `value_with` call per
/// iteration, comparing the allocating [`CoalitionEvaluator::value_with`] against
/// the scratch-reusing [`CoalitionEvaluator::value_with_scratch`]. The
/// per-call-allocation win the scratch buffers buy is the delta between the two.
/// Measured at `m = 8` and `m = 16` on [`build_fast_path_fixture`].
///
/// **Result (2026-07-22, #33):**
/// - `hit/8` ≈ 1.06 µs, `hit_scratch/8` ≈ 0.90 µs → **~15% faster**;
/// - `hit/16` ≈ 2.67 µs, `hit_scratch/16` ≈ 2.42 µs → **~9% faster**.
///
/// The seven per-call `Vec` allocations are a modest fraction of the O(m² + k²)
/// fast-path arithmetic, so the win is real but small — worth it for a hot
/// koalisi join sweep, not urgent. Slow-path candidates see no change (their cost
/// is the re-inversion, which the scratch does not touch).
fn bench_value_with(c: &mut Criterion) {
    let mut group = c.benchmark_group("coalition_value_with");

    for &m in &[8usize, 16] {
        let (agents, couplings, members, candidate) = build_fast_path_fixture(m);
        let ev = CoalitionEvaluator::new(&agents, &couplings, &members, 1.0)
            .expect("fast-path coalition must evaluate");

        // (a) Allocating fast path: 7 fresh Vecs per call.
        group.bench_with_input(BenchmarkId::new("hit", m), &m, |b, _| {
            b.iter(|| ev.value_with(candidate).expect("candidate must evaluate"));
        });

        // (b) Scratch fast path: buffers reused across calls (one scratch built
        // outside the timing loop, exactly the koalisi sweep pattern).
        let mut scratch = EvalScratch::new();
        group.bench_with_input(BenchmarkId::new("hit_scratch", m), &m, |b, _| {
            b.iter(|| {
                ev.value_with_scratch(candidate, &mut scratch)
                    .expect("candidate must evaluate")
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_magnitude,
    bench_incremental,
    bench_evaluator_new,
    bench_value_with
);
criterion_main!(benches);
