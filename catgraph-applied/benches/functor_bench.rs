//! Bench groups for the matrix functor `S : SFG_R → Mat(R)` (FS18 Thm 5.53).
//!
//! Paper anchors:
//! - **FS18 §5.3 Thm 5.53** — the functor `S : SFG_R → Mat(R)` realised by
//!   `sfg_to_mat`. Fong-Spivak, *Seven Sketches in Compositionality*
//!   (arXiv:1803.05316v3).
//! - **FS18 §5.4 Thm 5.60** — the `Free(Σ_SFG)/⟨E_{18}⟩ ≅ Mat(R)`
//!   presentation. Proved by F&S Thm 5.60 (via Baez-Erbele, *Categories in
//!   Control* (2015, arXiv:1405.6881), for fields; Wadsley–Woods *PROPs for
//!   Linear Systems* (arXiv:1505.00048) for commutative rigs, cf. BE15 §6).
//!   cg-applied does NOT re-verify this theorem at runtime.
//!
//! ## `cc_incompleteness_count::*` groups — REMOVED (owner decision 2026-08-02, #189)
//!
//! This file previously carried two `d=2` groups,
//! `functor::cc_incompleteness_count::{bool, f64rig}/2`, benching
//! `graphical_linalg::verify_sfg_to_mat_is_full_and_faithful` — the CC-engine
//! incompleteness witness count (a progress signal, not a correctness gate;
//! the `tests/graphical_linalg.rs` module docstring stays the authoritative
//! semantics). The #189 partition change (greedy representative scan →
//! all-pairs union-find components) moved one `d=2` verifier call from ≈7 s
//! to ≈120 s (`bool`) / ≈129 s (`f64rig`), both `p = 0.00` per criterion
//! (measured 2026-08-01; means over `[119.19, 120.26, 121.83]` s and
//! `[127.52, 128.90, 130.44]` s respectively), putting the pair at
//! ≈45 min wall per bench run with `sample_size(10)` already at criterion's
//! floor — no configuration brings the groups back under a minute. Owner
//! decision 2026-08-02 (recorded on #189): **drop both groups**; the
//! witness-count signal lives entirely in the `#[ignore]`'d
//! `cc_completeness_tracking_*` trackers in `tests/graphical_linalg.rs`
//! (run with `--ignored`), whose pins the cc-pin guard enforces at
//! 748/1114/1594/1590 (post-#189). The `size_bound=3` bool group had already
//! been dropped pre-#189 (#59: one `d=3` call exceeds 590 s). Note the
//! removed `f64rig` group's witness count was never a tracked pin — its
//! 2-sample fixture (`{0.0, 1.0}`, deliberately fast-but-degenerate) yields
//! a different count from the 4-sample test fixture behind the 1590 pin.
//!
//! Follows the workspace bench-file conventions (module-level imports,
//! `drop(black_box(...))` for `Result`-returning hot-path calls,
//! `std::hint::black_box`, per-file `SEED` constant).
//!
//! ## Bench-size bracket
//!
//! Criterion's `BenchmarkId::from_parameter(d)` axes the depth as a parameter,
//! so the final benchmark IDs displayed by criterion are
//! `functor::sfg_to_mat::{f64, bool}/{3, 5, 7}` — the two `benchmark_group`
//! names below + the depth parameter, NOT the `_d{3,5,7}`-suffix form used in
//! the plan + design doc as a prescriptive example.
//!
//! - **`sfg_to_mat::{f64, bool}` groups: depth `d ∈ {3, 5, 7}`.** Balanced
//!   binary trees of pure `Compose(Scalar(r), Scalar(r))` nodes — no
//!   `Tensor` wiring; the fixture intentionally stays arity-1 `1 → 1`
//!   throughout so the cost class isolated is the *recursion* + *functor
//!   evaluation* cost, NOT the matmul-size cost. Depth 7 produces a tree
//!   with 128 `Scalar` leaves + 127 `Compose` internal nodes. With `1×1`
//!   matrices at every level, each matmul collapses to a single `R::mul`
//!   invocation, so the cumulative cost is `O(2^d)` scalar `R::Mul` + `O(2^d)`
//!   `Result`-wrapping at internal nodes (NOT `O(n²)` matmul — that cost
//!   class would require `Tensor` widening to expose non-`1×1` matrices,
//!   deferred).
//!
//! ## Trait-bound dispatch tier
//!
//! Both groups dispatch through the [`Rig`] blanket impl only.
//! `F64Rig`'s `Neg`/`Sub`/`Div`/`From<i64>` inherent extensions (see the
//! [`F64Rig`] impl blocks in `rig.rs`) are NOT exercised by `sfg_to_mat`
//! (Thm 5.53 + Def 5.50 are pure-rig theorems per CLAUDE.md). A reviewer
//! expecting the `F64Rig` vs `BoolRig` contrast to expose `Ring` / `Field` /
//! `ZAlgebra` tower-tier dispatch cost will not find it here — the bench
//! measures pure-rig monomorphisation only. The genuine contrast between
//! the two rigs is per-operation arithmetic cost (~1 cycle bool ∨/∧, ~3-5
//! cycles f64 +/*) plus the Cayley-table-collapse asymmetry described above.
//!
//! ## Reproducibility
//!
//! No randomness — all bench fixtures are pure constructive walks over the
//! `SignalFlowGraph` smart-constructor surface. The per-file `SEED`
//! constant from the `mat_ops_bench` precedent is retained as a placeholder
//! for future randomised fixtures (currently unused; note that this file is
//! fully deterministic).
//!
//! Fixture allocation cost is amortised at setup, NOT inside `bencher.iter`.
//! At depth 7 the `build_sfg_fixture_d` recursion constructs 127 `Compose`
//! nodes once; `sfg_to_mat` then clones the SFG node graph per iteration
//! (~128 leaf clones, ~127 internal `Compose` clones; for both `BoolRig`
//! and `F64Rig` these clones lower to `mem::copy` since both rigs are
//! `Copy`). The measured cost IS the per-iteration clone + functorial-
//! evaluation cost; fixture construction is one-shot and not part of the
//! steady-state measurement.
//!
//! [`Rig`]: catgraph_applied::rig::Rig
//! [`F64Rig`]: catgraph_applied::rig::F64Rig

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

use catgraph_applied::{
    rig::{BoolRig, F64Rig},
    sfg::SignalFlowGraph,
    sfg_to_mat::sfg_to_mat,
};

/// Reserved per-file seed slot. Currently unused — all
/// fixtures are deterministic constructive walks. Retained so a future
/// randomised-fixture addition has a documented seed handle ready.
#[allow(dead_code)]
const SEED: u64 = 0xCAFE_BABE_DEAD_BEEF;

// ---------------------------------------------------------------------------
// Fixture builders
// ---------------------------------------------------------------------------

/// Build a balanced `SignalFlowGraph<R>` fixture of `Compose(Scalar, Scalar)`
/// pairs at logical depth `d`.
///
/// The recursion shape:
///
/// - **`d = 0`**: `Scalar(R::one()) : 1 → 1` — a single `1×1` matrix leaf.
/// - **`d > 0`**: `Compose(build(d-1), build(d-1))` — sequential composition
///   that preserves the `1 → 1` arity while doubling the underlying
///   `PropExpr` node count.
///
/// The fixture is arity-1 throughout (matrices stay `1×1`), so each `matmul`
/// inside `sfg_to_mat` is a single-scalar multiply. The `O(2^d)` node growth
/// is what's actually being characterised, not matmul size. Depth-7 fixtures
/// have 128 `Scalar` leaves and 127 `Compose` nodes.
///
/// `compose` returns `Result` only because the public `SignalFlowGraph::compose`
/// surface is arity-checked; here the construction is arity-correct by
/// construction, so the `.expect` is a maintenance-bug indicator only.
fn build_sfg_fixture_d<R>(depth: usize) -> SignalFlowGraph<R>
where
    R: catgraph_applied::rig::Rig + std::fmt::Debug + Eq + std::hash::Hash + Ord + 'static,
{
    if depth == 0 {
        // Base: Scalar(one) is 1 → 1, the smallest non-identity matrix-leaf.
        SignalFlowGraph::<R>::scalar(R::one())
    } else {
        let half = build_sfg_fixture_d::<R>(depth - 1);
        half.compose(&half)
            .expect("fixture: arity-correct by construction (1 → 1 throughout)")
    }
}

/// Count the number of `PropExpr` nodes in the fixture — for throughput
/// reporting. A balanced binary tree of depth `d` has `2^(d+1) - 1` total
/// nodes (`2^d` leaves + `2^d - 1` internal Compose nodes).
const fn fixture_node_count(depth: u32) -> u64 {
    (1u64 << (depth + 1)) - 1
}

// ---------------------------------------------------------------------------
// Group 1 — `functor::sfg_to_mat_d{3,5,7}::f64` (functorial evaluation cost)
// ---------------------------------------------------------------------------

fn bench_sfg_to_mat_f64(c: &mut Criterion) {
    let mut group = c.benchmark_group("functor::sfg_to_mat::f64");

    for &d in &[3u32, 5, 7] {
        let sfg: SignalFlowGraph<F64Rig> = build_sfg_fixture_d::<F64Rig>(d as usize);

        // Throughput in elements: report the count of PropExpr nodes
        // touched per evaluation. For a balanced d-deep tree of
        // Compose(Scalar, Scalar) nodes this is exactly `2^(d+1) - 1`.
        // The reported rate is then nodes-per-second, which gives a
        // direct cross-depth comparison (constant per-node work — one
        // matmul at each Compose, one generator-table lookup at each
        // Scalar leaf).
        group.throughput(Throughput::Elements(fixture_node_count(d)));

        group.bench_with_input(BenchmarkId::from_parameter(d), &d, |bencher, _| {
            bencher.iter(|| {
                // `sfg_to_mat` is `Result`-returning + hot-path; the
                // bench-file precedent (mat_ops_bench:116, :135) — use
                // `drop(black_box(...))` to make the anti-elision intent
                // structural rather than relying on Result-Drop side
                // effects.
                drop(black_box(sfg_to_mat(black_box(&sfg))));
            });
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Group 2 — `functor::sfg_to_mat::bool/{3,5,7}` (Cayley-table arithmetic
//   contrast: bool ∨/∧ idempotent vs f64 +/* non-idempotent)
// ---------------------------------------------------------------------------
//
// Note: BoolRig and F64Rig are BOTH `Copy` (rig.rs:55 and rig.rs:224), so
// `.clone()` calls inside `sfg_to_mat_inner` lower to `mem::copy` at codegen
// for either rig — there is no genuine clone-cost dimension measured here.
// The true contrast is per-operation arithmetic cost (~1 cycle bool ∨/∧
// vs ~3-5 cycles f64 +/*) plus the Cayley-table collapse from idempotent
// `∨/∧` (BoolRig) vs free `+/*` (F64Rig) discussed in the module rustdoc.

fn bench_sfg_to_mat_bool(c: &mut Criterion) {
    let mut group = c.benchmark_group("functor::sfg_to_mat::bool");

    for &d in &[3u32, 5, 7] {
        let sfg: SignalFlowGraph<BoolRig> = build_sfg_fixture_d::<BoolRig>(d as usize);

        group.throughput(Throughput::Elements(fixture_node_count(d)));

        group.bench_with_input(BenchmarkId::from_parameter(d), &d, |bencher, _| {
            bencher.iter(|| {
                drop(black_box(sfg_to_mat(black_box(&sfg))));
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_sfg_to_mat_f64, bench_sfg_to_mat_bool);
criterion_main!(benches);
