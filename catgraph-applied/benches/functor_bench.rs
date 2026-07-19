//! Bench groups for the matrix functor `S : SFG_R → Mat(R)` (FS18 Thm 5.53)
//! and CC-engine completeness tracking via
//! [`catgraph_applied::graphical_linalg::verify_sfg_to_mat_is_full_and_faithful`].
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
//! ## What the `cc_incompleteness_count_*` groups actually measure
//!
//! The function
//! [`verify_sfg_to_mat_is_full_and_faithful`] is NOT a faithfulness gate
//! (F&S Thm 5.60 already proves the theorem holds — via Baez-Erbele 2015 for
//! fields, Wadsley–Woods arXiv:1505.00048 for commutative rigs; cg-applied does
//! not re-prove established theorems). It returns **CC-engine incompleteness
//! witnesses** — pairs of SFG expressions that the matrix functor `S`
//! distinguishes which the default [`CongruenceClosure`] engine does NOT
//! identify. The witness count (1142 for `size_bound=2` on `BoolRig`,
//! post-E_18) stays nonzero **by design**: the terminal Mat(R) decision path
//! is the Functorial engine (issue #15 resolved functorial-terminal; syntactic
//! Knuth-Bendix completion is the #57 feasibility spike). See the
//! `tests/graphical_linalg.rs` module docstring for the authoritative semantics.
//!
//! The bench tracks this count as a **performance + progress signal on
//! CC-engine completeness**, not as a correctness gate. An NF improvement that
//! drives the witness count down would also show up here as a wall-clock change.
//!
//! Follows the workspace bench-file conventions (module-level imports,
//! `drop(black_box(...))` for `Result`-returning hot-path calls,
//! `std::hint::black_box`, per-file `SEED` constant).
//!
//! ## Bench-size bracket
//!
//! Criterion's `BenchmarkId::from_parameter(d)` axes the depth as a parameter,
//! so the final benchmark IDs displayed by criterion are
//! `functor::sfg_to_mat::{f64, bool}/{3, 5, 7}` and
//! `functor::cc_incompleteness_count::{bool, f64rig}/{2, 3}` — the four
//! `benchmark_group` names below + the depth parameter, NOT the
//! `_d{3,5,7}`-suffix form used in the plan + design doc as a prescriptive
//! example.
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
//! - **`cc_incompleteness_count::bool` at `size_bound = 2`** — produces
//!   1142 CC-incompleteness witnesses on `BoolRig` (post-E_18) — the
//!   measured empirical baseline. The witness
//!   count is the size of the gap between
//!   [`NormalizeEngine::CongruenceClosure`] (syntactic, incomplete) and
//!   [`NormalizeEngine::Functorial`] (semantic, complete-by-Thm-5.60); the
//!   bench tracks this gap as a progress signal, not as a Thm 5.60
//!   faithfulness check.
//!
//! - **Witness-count asymmetry (`BoolRig` vs `F64Rig`).** At `size_bound = 2`
//!   `BoolRig` produces 1142 witnesses (post-E_18); `F64Rig` produces a
//!   larger count (~2478) that blows up combinatorially at `size_bound = 3`.
//!   The mechanism is
//!   algebraic, not a measurement artefact: `BoolRig` is idempotent
//!   (`a ∨ a = a`, `a ∧ a = a`) so the D1 Cayley table
//!   `r_a ; r_b = r_{a*b}` collapses the 2×2 cross-product to a small set
//!   of distinct products, whereas `F64Rig`'s free `+/*` on `{0.0, 1.0}`
//!   generates fresh scalar atoms (`1+1 = 2`) that compound at the next
//!   bound step.
//!
//! - **`cc_incompleteness_count::bool` at `size_bound = 3`** — the "under 60s"
//!   / `sample_size = 10` figures here were **design-doc estimates, not
//!   measurements**. Ground truth (release): one `d=2` verifier call is ~7.6 s
//!   (so the `d=2` bench at criterion defaults is ~13 min), and one `d=3` call
//!   exceeds 590 s (>10 min) — no criterion configuration brings the `d=3`
//!   group under 60 s. Marker flag below; re-budgeting these groups (or folding
//!   the `d=3`-bool bracket back to `d=2`) is tracked in #59.
//!
//! - **`cc_incompleteness_count::f64rig` at `size_bound = 2`, `F64Rig` only.**
//!   Per design doc §3.3.2: `F64Rig` scalar sampling combinatorially blows
//!   up at `d=3`, so the `F64Rig` variant runs at `d=2` only. The sample
//!   choice `{F64Rig(0.0), F64Rig(1.0)}` is deliberately "fast-but-degenerate"
//!   — `F64Rig(0.0)` is the additive identity AND absorbing under `Mul`, so
//!   3 of 4 D1 cross-product entries short-circuit; only `1·1 = 1`
//!   exercises the free-multiplication path. A non-degenerate alternative
//!   (e.g. `{2.0, 3.0}`) is a deferred future addition.
//!
//! ## Trait-bound dispatch tier
//!
//! All four groups dispatch through the [`Rig`] blanket impl only.
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
//! [`CongruenceClosure`]: catgraph_applied::prop::presentation::NormalizeEngine::CongruenceClosure
//! [`NormalizeEngine::CongruenceClosure`]: catgraph_applied::prop::presentation::NormalizeEngine::CongruenceClosure
//! [`NormalizeEngine::Functorial`]: catgraph_applied::prop::presentation::NormalizeEngine::Functorial
//! [`PropExpr`]: catgraph_applied::prop::PropExpr
//! [`Rig`]: catgraph_applied::rig::Rig
//! [`F64Rig`]: catgraph_applied::rig::F64Rig
//! [`verify_sfg_to_mat_is_full_and_faithful`]: catgraph_applied::graphical_linalg::verify_sfg_to_mat_is_full_and_faithful

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

use catgraph_applied::{
    graphical_linalg::verify_sfg_to_mat_is_full_and_faithful,
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
    R: catgraph_applied::rig::Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static,
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

// ---------------------------------------------------------------------------
// Group 3 — `functor::cc_incompleteness_count_d{2,3}::bool`
// ---------------------------------------------------------------------------
//
// `verify_sfg_to_mat_is_full_and_faithful::<BoolRig>(size_bound=2)` returns
// 1142 CC-incompleteness witnesses (measured empirically, post-E_18; see the
// `tests/graphical_linalg.rs` module docstring for authoritative semantics).
// The `size_bound=3` variant is far heavier — one d=3 call exceeds 590 s in
// release (>10 min), so the "60s wall budget" below was a design-doc estimate,
// not a measured bound; the reduced sample count + warmup only cap the sample
// count, they cannot reach it. Re-budgeting/folding is tracked in #59.

fn bench_cc_incompleteness_count_bool(c: &mut Criterion) {
    let mut group = c.benchmark_group("functor::cc_incompleteness_count::bool");

    // BoolRig samples used by the `D-group` scalar equations inside
    // `matr_presentation` — must be non-empty and ideally non-trivial.
    // BoolRig has only two values (true / false); we provide both so
    // `D1: r_a ; r_b = r_{a*b}` enumerates the full 2×2 Cayley table for
    // a tight presentation.
    let rig_samples = vec![BoolRig(true), BoolRig(false)];

    // d=2: the canonical signal (1142 witnesses, post-E_18).
    group.throughput(Throughput::Elements(1));
    group.bench_function(BenchmarkId::from_parameter(2u32), |bencher| {
        bencher.iter(|| {
            drop(black_box(
                verify_sfg_to_mat_is_full_and_faithful::<BoolRig>(
                    black_box(2),
                    black_box(&rig_samples),
                ),
            ));
        });
    });

    // Criterion `benchmark_group` config (sample_size / warm_up_time /
    // measurement_time) applies to all benches registered AFTER the config
    // call — d=2 above already ran with criterion defaults (100 samples,
    // 3s warmup); only the d=3 bench below picks up the reduced settings.
    // This ordering is intentional: d=2 needs full-sample statistical
    // fidelity for the 1142-witness baseline; d=3 is wall-time-budget-
    // constrained.
    //
    // d=3: heavy — one d=3 verifier call exceeds 590 s in release, so no
    // criterion config brings this group under the design-doc "60s wall
    // budget"; the reduced sample count + warmup only cap the sample count.
    // Re-budgeting, or dropping the d=3-bool variant in favour of d=2, is
    // tracked in #59 (see the module rustdoc note above).
    group.sample_size(10);
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(20));

    group.bench_function(BenchmarkId::from_parameter(3u32), |bencher| {
        bencher.iter(|| {
            drop(black_box(
                verify_sfg_to_mat_is_full_and_faithful::<BoolRig>(
                    black_box(3),
                    black_box(&rig_samples),
                ),
            ));
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Group 4 — `functor::cc_incompleteness_count::f64rig/2`
// ---------------------------------------------------------------------------
//
// `F64Rig` scalar sampling combinatorially blows up at d=3 (design doc
// §3.3.2); the F64Rig variant runs at d=2 only. Two rig samples (`0.0` and
// `1.0`) keep the `D1` Cayley-table iteration minimal — but note that
// `F64Rig(0.0)` is the additive identity AND absorbing under `Mul`, so 3
// of the 4 D1 cross-product entries `(a, b) ∈ {0, 1}²` hit the absorbing-
// zero short-circuit branch and only `1·1 = 1` exercises the free-
// multiplication path. This is a deliberate "fast-but-degenerate" choice
// — for an authentic F64-arithmetic signal the samples would be
// `{2.0, 3.0}` (producing 4 fresh atoms `{4, 6, 6, 9}` that force the CC
// engine through real work). A future addition would be the
// natural place to expose a non-degenerate algebraic-structure dimension
// via a Tropical bench group (idempotent additive + free multiplicative —
// a structurally different rig from both BoolRig + F64Rig).

fn bench_cc_incompleteness_count_f64rig(c: &mut Criterion) {
    let mut group = c.benchmark_group("functor::cc_incompleteness_count::f64rig");

    // Two samples → 4-entry D1 cross-product (a, b) ∈ {0, 1}² inside
    // `matr_presentation`. Both values are `Copy` + bit-exact `Eq + Hash`
    // (manual impls via `to_bits()`); no NaN risk for these literals.
    let rig_samples = vec![F64Rig(0.0), F64Rig(1.0)];

    group.throughput(Throughput::Elements(1));
    group.bench_function(BenchmarkId::from_parameter(2u32), |bencher| {
        bencher.iter(|| {
            drop(black_box(verify_sfg_to_mat_is_full_and_faithful::<F64Rig>(
                black_box(2),
                black_box(&rig_samples),
            )));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_sfg_to_mat_f64,
    bench_sfg_to_mat_bool,
    bench_cc_incompleteness_count_bool,
    bench_cc_incompleteness_count_f64rig
);
criterion_main!(benches);
