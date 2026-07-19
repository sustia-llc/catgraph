//! Bench groups for `Presentation`, `CongruenceClosure`, and `smc_nf::nf`
//! (CC engine + Layer-1 Joyal-Street normal-form perf characterization).
//!
//! Paper anchors:
//! - **FS18 §5.2 Def 5.2 + Def 5.25 + Def 5.33** — `Free(G)` + presentation.
//!   Fong-Spivak, *Seven Sketches in Compositionality* (arXiv:1803.05316v3).
//! - **FS18 §5.4 Thm 5.60** — the `Free(Σ_SFG)/⟨E_{18}⟩ ≅ Mat(R)`
//!   presentation. Proved by F&S Thm 5.60 (via Baez-Erbele, *Categories in
//!   Control* (2015, arXiv:1405.6881), for fields; Wadsley–Woods *PROPs for
//!   Linear Systems* (arXiv:1505.00048) for commutative rigs, cf. BE15 §6).
//!   cg-applied does NOT re-verify this theorem at
//!   runtime; this bench measures the perf of `Presentation::eq_mod`'s
//!   decision procedure on a representative subset of the equations + a set
//!   of hand-curated probe pairs, NOT theorem verification.
//! - **Joyal-Street 1991** — the Layer-1 normal form decided by
//!   [`smc_nf::nf`]: canonicalize `PropExpr` up to SMC coherence (associator,
//!   unitors, interchange, braid naturality, σ²=id) without consulting user
//!   equations.
//!
//! Follows the workspace bench-file conventions (`std::hint::black_box`,
//! `drop(black_box(...))` for `Result`-returning hot-path calls,
//! module-level imports, per-file `SEED` constant).
//!
//! ## Feature-gating
//!
//! This bench file is **entirely inert without `--features internal-bench`**.
//! Without the feature, the file compiles to just an empty `fn main() {}`,
//! producing an empty bench harness when `harness = false`. With the feature
//! ON, all 5 bench groups exercise.
//!
//! The substantive bench body lives in a single `#[cfg(feature =
//! "internal-bench")] mod inner` block so the cfg sprinkling stays at one
//! site (entire module gated), not per-function. The `atom_canonical_n*`
//! group exercises the feature-gated
//! [`CongruenceClosure::atom_canonical_for_bench`] shim that exposes the
//! private `atom_canonical` method behind a `SemVer` (semantic-versioning)
//! non-guarantee.
//!
//! Run:
//! ```text
//! cargo bench -p catgraph-applied --bench presentation_bench \
//!     --features internal-bench
//! ```
//!
//! ## What the `atom_canonical_n*` group actually measures
//!
//! The function [`CongruenceClosure::atom_canonical_for_bench`] is a public
//! feature-gated shim over the private `atom_canonical` selector that
//! resolves a `TermId`'s union-find class to its preferred atom member
//! (lowest-`node_kind`, smallest `TermId`). The naive implementation is
//! `O(n)` per call (linear scan over `reverse` filtering by `find`); calling
//! it from inside the `smc_refine` fixpoint contributes the
//! `O(n²)/n³` perf-tail called out as a deferred atom-canonical TODO. This
//! group quantifies that tail at three problem sizes (`n ∈ {32, 64, 128}`).
//! The KB-vs-Functorial decision is resolved (functorial-terminal, #15); this
//! measured baseline now serves the #57 KB feasibility spike rather than a
//! back-of-envelope estimate.
//!
//! ## Bench-size brackets
//!
//! - **`eq_mod_thm_5_60::cc`: 20 probe pairs** synthesised by walking
//!   `matr_presentation::<BoolRig>(&[BoolRig(true), BoolRig(false)])` and
//!   taking the first 20 equation `(lhs, rhs)` pairs. CC-dominated regime
//!   (none of the pairs are pre-NF-equal so the NF short-circuit falls
//!   through to the CC engine).
//!
//! - **`eq_mod_thm_5_60::nf_short_circuit`: 20 reflexive probe pairs** —
//!   `(lhs, lhs.clone())` for each of the same 20 fixture pairs. These hit
//!   the NF short-circuit on line 309 of `presentation/mod.rs`
//!   (`if smc_nf::nf(a) == smc_nf::nf(b) { return Ok(Some(true)); }`) and
//!   never consult the CC engine, so the wall-clock ratio between this
//!   group and `cc` is the short-circuit speedup factor.
//!
//! - **`cc_new::seed_{4, 16, 32}_eqs`** — `CongruenceClosure::new` on a
//!   prefix of the `matr_presentation::<BoolRig>` seed (4/16/32 are perf
//!   brackets, not the 18-schema Thm 5.60 count) +
//!   inflated repetition to hit higher equation counts.
//!
//! - **`atom_canonical_n{32, 64, 128}`** — `n` synthesised generators
//!   inserted into a fresh `CongruenceClosure`, then `atom_canonical_for_bench`
//!   called on the final inserted root. Per-call cost is `O(n)`; the bracket
//!   tracks the linear growth.
//!
//! - **`smc_nf::nf_d{3, 5, 7}::{pure_braid, pure_tensor, mixed}`** — three
//!   shape families crossed with three depths. Pure-braid stresses the
//!   braid-prefix collection step; pure-tensor stresses the layer
//!   coalescing step; mixed exercises both.
//!
//! ## Reproducibility
//!
//! Per-file `SEED = 0xCAFE_BABE_DEAD_BEEF`. The fixtures
//! here are largely constructive (no randomised sampling), but the seed
//! is retained as a documented handle for any future randomised-fixture
//! additions and to keep the file-level seed convention uniform across the
//! bench files.
//!
//! [`CongruenceClosure::atom_canonical_for_bench`]: catgraph_applied::prop::presentation::kb::CongruenceClosure::atom_canonical_for_bench

#[cfg(feature = "internal-bench")]
mod inner {
    use std::hint::black_box;

    use criterion::{BenchmarkId, Criterion, Throughput};

    use catgraph_applied::graphical_linalg::matr_presentation;
    use catgraph_applied::prop::presentation::kb::CongruenceClosure;
    use catgraph_applied::prop::presentation::smc_nf;
    use catgraph_applied::prop::{Free, PropExpr};
    use catgraph_applied::rig::BoolRig;
    use catgraph_applied::sfg::SfgGenerator;

    /// Per-file seed. Documented in the module rustdoc as the file-level seed
    /// handle; currently unused (this file's fixtures are deterministic
    /// constructive walks) but retained to keep the seed convention uniform
    /// across the bench files. Flag if any future randomised-fixture group is
    /// added that does NOT thread this seed.
    #[expect(
        dead_code,
        reason = "placeholder for future randomised fixtures; see file-level rustdoc Reproducibility section"
    )]
    const SEED: u64 = 0xCAFE_BABE_DEAD_BEEF;

    /// Compact alias for the `BoolRig`-flavoured `PropExpr` equation pair
    /// used by the `cc` / `nf_short_circuit` / `cc_new` groups. Reduces type
    /// complexity in the fixture builder's return signature
    /// (`clippy::type_complexity`).
    type BoolEqPair = (
        PropExpr<SfgGenerator<BoolRig>>,
        PropExpr<SfgGenerator<BoolRig>>,
    );

    // -----------------------------------------------------------------------
    // Fixture builders
    // -----------------------------------------------------------------------

    /// Build the 18-equation Thm 5.60 presentation over `BoolRig` and return
    /// the first 20 `(lhs, rhs)` equation pairs. Used to synthesise the `cc`
    /// and `nf_short_circuit` probe pairs.
    ///
    /// `BoolRig` is the smallest rig that exercises all 18 equations + scalar
    /// substitutions; `rig_samples = [true, false]` provides two distinct
    /// scalars without combinatorial blow-up (compare with `F64Rig`, which
    /// the `functor_bench` reserves to `d=2` only for the same reason).
    fn build_eq_mod_fixture_pairs() -> Vec<BoolEqPair> {
        let pres = matr_presentation::<BoolRig>(&[BoolRig(true), BoolRig(false)])
            .expect("matr_presentation: hardcoded equations are arity-correct by construction");
        pres.equations().iter().take(20).cloned().collect()
    }

    /// Build a `PropExpr<SfgGenerator<BoolRig>>` of pure-braid shape at
    /// logical depth `d`.
    ///
    /// Shape: `Compose(braid_1_1(), Compose(braid_1_1(), ...))` nested `d`
    /// times over the 2-string braid generator. Source/target both `2`, so
    /// the chain is arity-correct.
    fn build_pure_braid(d: usize) -> PropExpr<SfgGenerator<BoolRig>> {
        let braid = Free::<SfgGenerator<BoolRig>>::braid(1, 1);
        let mut expr = braid.clone();
        for _ in 0..d {
            expr = Free::<SfgGenerator<BoolRig>>::compose(expr, braid.clone())
                .expect("pure-braid fixture: arity-correct by construction (2 → 2)");
        }
        expr
    }

    /// Build a `PropExpr<SfgGenerator<BoolRig>>` of pure-tensor shape at
    /// logical depth `d`.
    ///
    /// Shape: `Tensor(id_1, Tensor(id_1, ...))` nested `d` deep. Arity grows
    /// as `d + 1`, which is fine for `nf` (the layer-coalescing canonicaliser
    /// doesn't care about arity). No `Braid` nodes appear — exercises the
    /// layer-coalescing step in isolation.
    fn build_pure_tensor(d: usize) -> PropExpr<SfgGenerator<BoolRig>> {
        let id1 = Free::<SfgGenerator<BoolRig>>::identity(1);
        let mut expr = id1.clone();
        for _ in 0..d {
            expr = Free::<SfgGenerator<BoolRig>>::tensor(id1.clone(), expr);
        }
        expr
    }

    /// Build a mixed-shape `PropExpr<SfgGenerator<BoolRig>>` at logical depth
    /// `d` — alternating `Compose(braid, ...)` and `Tensor(id_0, ...)` steps.
    /// Both the braid-prefix collection step and the layer-coalescing step
    /// contribute to the per-fixpoint-iteration cost.
    fn build_mixed(d: usize) -> PropExpr<SfgGenerator<BoolRig>> {
        let braid = Free::<SfgGenerator<BoolRig>>::braid(1, 1);
        let mut expr = braid.clone();
        for i in 0..d {
            if i % 2 == 0 {
                expr = Free::<SfgGenerator<BoolRig>>::compose(expr, braid.clone())
                    .expect("mixed fixture: arity-correct (Compose(2 → 2, 2 → 2))");
            } else {
                // Tensor with id_0 preserves arity exactly; the layer-
                // coalescing step still has to walk the layer count.
                expr = Free::<SfgGenerator<BoolRig>>::tensor(
                    Free::<SfgGenerator<BoolRig>>::identity(0),
                    expr,
                );
            }
        }
        expr
    }

    /// Approximate node count in a linear-chain `PropExpr` of depth `d`. Used
    /// as a throughput unit for the `smc_nf::nf` groups (one rewrite-pass
    /// step touches every node). Reviewers: this is intentionally an
    /// under-count of the true work (`nf` runs a fixpoint over a 5-pass
    /// loop, so the actual node-touch count is roughly `5 * (d + 1)` per
    /// fixpoint iteration); we report the bare-bones node count so
    /// cross-depth comparisons remain interpretable as "ns per fixture
    /// node".
    const fn smc_nf_node_count(d: u32) -> u64 {
        (d as u64) + 1
    }

    // -----------------------------------------------------------------------
    // Group 1 — `presentation::eq_mod_thm_5_60::cc` (CC-dominated regime)
    // -----------------------------------------------------------------------

    pub(super) fn bench_eq_mod_cc(c: &mut Criterion) {
        let mut group = c.benchmark_group("presentation::eq_mod_thm_5_60::cc");

        let pres = matr_presentation::<BoolRig>(&[BoolRig(true), BoolRig(false)])
            .expect("matr_presentation: hardcoded equations are arity-correct");
        let pairs = build_eq_mod_fixture_pairs();

        // Throughput in elements: 20 probe pairs per iteration.
        group.throughput(Throughput::Elements(pairs.len() as u64));

        group.bench_function("20_pairs", |bencher| {
            bencher.iter(|| {
                for (lhs, rhs) in &pairs {
                    // `eq_mod` is `Result<Option<bool>, _>`-returning + hot
                    // path; the bench-file precedent — `drop(black_box(...))` for
                    // anti-elision.
                    drop(black_box(pres.eq_mod(black_box(lhs), black_box(rhs))));
                }
            });
        });

        group.finish();
    }

    // -----------------------------------------------------------------------
    // Group 2 — `presentation::eq_mod_thm_5_60::nf_short_circuit` (NF-hit
    // regime)
    // -----------------------------------------------------------------------

    pub(super) fn bench_eq_mod_nf_short_circuit(c: &mut Criterion) {
        let mut group = c.benchmark_group("presentation::eq_mod_thm_5_60::nf_short_circuit");

        let pres = matr_presentation::<BoolRig>(&[BoolRig(true), BoolRig(false)])
            .expect("matr_presentation: hardcoded equations are arity-correct");
        // Reflexive: pair each LHS with a clone of itself. These trigger the
        // NF short-circuit on line 309 of `presentation/mod.rs` and never
        // consult the CC engine.
        let pairs: Vec<(_, _)> = build_eq_mod_fixture_pairs()
            .into_iter()
            .map(|(lhs, _rhs)| (lhs.clone(), lhs))
            .collect();

        group.throughput(Throughput::Elements(pairs.len() as u64));

        group.bench_function("20_reflexive_pairs", |bencher| {
            bencher.iter(|| {
                for (lhs, rhs) in &pairs {
                    drop(black_box(pres.eq_mod(black_box(lhs), black_box(rhs))));
                }
            });
        });

        group.finish();
    }

    // -----------------------------------------------------------------------
    // Group 3 — `presentation::cc_new::seed_{4, 16, 32}_eqs` (CC seed cost)
    // -----------------------------------------------------------------------

    pub(super) fn bench_cc_new(c: &mut Criterion) {
        let mut group = c.benchmark_group("presentation::cc_new");

        // 16-equation bracket taken as a prefix of the `matr_presentation`
        // seed; pad to 32 by duplicating that 16-equation prefix once (CC's
        // hash-cons folds identical equations into a single insert, so the
        // cost growth tracks unique-equation count more than wall-clock seed
        // count). Truncate to 4 for the smallest bracket. The 4/16/32 numbers
        // are perf brackets, not the Thm 5.60 schema count (which is 18).
        let base_pairs = build_eq_mod_fixture_pairs();
        let mut padded = base_pairs.clone();
        padded.extend(base_pairs.iter().cloned());

        let fixtures: Vec<(usize, Vec<_>)> = vec![
            (4, base_pairs.iter().take(4).cloned().collect()),
            (16, base_pairs.iter().take(16).cloned().collect()),
            (32, padded),
        ];

        for (n_eqs, equations) in &fixtures {
            group.throughput(Throughput::Elements(*n_eqs as u64));

            group.bench_with_input(
                BenchmarkId::from_parameter(format!("seed_{n_eqs}_eqs")),
                n_eqs,
                |bencher, _| {
                    bencher.iter(|| {
                        drop(black_box(CongruenceClosure::<SfgGenerator<BoolRig>>::new(
                            black_box(equations),
                        )));
                    });
                },
            );
        }

        group.finish();
    }

    // -----------------------------------------------------------------------
    // Group 4 — `presentation::atom_canonical_n{32, 64, 128}` (private-shim
    // bench for the O(n²)/n³ atom-canonical perf-tail)
    // -----------------------------------------------------------------------

    pub(super) fn bench_atom_canonical(c: &mut Criterion) {
        let mut group = c.benchmark_group("presentation::atom_canonical");

        for &n in &[32usize, 64, 128] {
            // Build n reflexive equations of `Compose(copy, id_2 → 2 wrap)`
            // shape. Each is arity-correct (copy: 1 → 2; wrap LHS with id_2);
            // reflexive seeding means CC's propagate-fixpoint immediately
            // settles (no real congruence cascade), keeping the construction
            // cost predictable. Every equation contributes 3 nodes
            // (Compose + Copy + Tensor(id, id)), hash-consed, so
            // `reverse.len()` grows linearly with n.
            let copy = Free::<SfgGenerator<BoolRig>>::generator(SfgGenerator::Copy);
            let id1 = Free::<SfgGenerator<BoolRig>>::identity(1);

            let mut equations: Vec<(_, _)> = Vec::with_capacity(n);
            for _i in 0..n {
                let lhs = Free::<SfgGenerator<BoolRig>>::compose(
                    copy.clone(),
                    Free::<SfgGenerator<BoolRig>>::tensor(id1.clone(), id1.clone()),
                )
                .expect("atom_canonical fixture: arity-correct (1 → 2 ; 2 → 2)");
                equations.push((lhs.clone(), lhs));
            }

            group.throughput(Throughput::Elements(n as u64));

            group.bench_with_input(BenchmarkId::from_parameter(n), &n, |bencher, _| {
                bencher.iter(|| {
                    // Re-seed inside the iter to avoid amortising
                    // construction cost across iterations. The construction
                    // cost dominates for small `n`; the `atom_canonical`
                    // call is what we *want* to measure. Note that
                    // criterion's HTML report's "mean" includes the
                    // construction. For a future regression-guard run,
                    // compute the construction-vs-shim ratio by comparing
                    // against `cc_new::seed_{4, 16, 32}_eqs` at the same
                    // n.
                    let mut engine = CongruenceClosure::<SfgGenerator<BoolRig>>::new(&equations);
                    let probe = Free::<SfgGenerator<BoolRig>>::generator(SfgGenerator::Copy);
                    let id = engine.add_term_for_bench(&probe);
                    // The shim returns `Option<Node<G>>`. `Node<G>` for
                    // `G = SfgGenerator<BoolRig>` is non-Drop (BoolRig is
                    // Copy), so the `drop(black_box(...))` precedent
                    // doesn't apply (clippy::drop_non_drop). Black-box the
                    // return directly — the bencher loop's iteration
                    // boundary already serves as the anti-elision fence.
                    black_box(engine.atom_canonical_for_bench(black_box(id)));
                });
            });
        }

        group.finish();
    }

    // -----------------------------------------------------------------------
    // Group 5 — `presentation::smc_nf::nf_d{3, 5, 7}::{pure_braid,
    // pure_tensor, mixed}`
    // -----------------------------------------------------------------------

    pub(super) fn bench_smc_nf(c: &mut Criterion) {
        let mut group = c.benchmark_group("presentation::smc_nf::nf");

        for &d in &[3u32, 5, 7] {
            let pure_braid = build_pure_braid(d as usize);
            let pure_tensor = build_pure_tensor(d as usize);
            let mixed = build_mixed(d as usize);

            group.throughput(Throughput::Elements(smc_nf_node_count(d)));

            group.bench_with_input(
                BenchmarkId::from_parameter(format!("d{d}::pure_braid")),
                &d,
                |bencher, _| {
                    bencher.iter(|| {
                        drop(black_box(smc_nf::nf(black_box(&pure_braid))));
                    });
                },
            );
            group.bench_with_input(
                BenchmarkId::from_parameter(format!("d{d}::pure_tensor")),
                &d,
                |bencher, _| {
                    bencher.iter(|| {
                        drop(black_box(smc_nf::nf(black_box(&pure_tensor))));
                    });
                },
            );
            group.bench_with_input(
                BenchmarkId::from_parameter(format!("d{d}::mixed")),
                &d,
                |bencher, _| {
                    bencher.iter(|| {
                        drop(black_box(smc_nf::nf(black_box(&mixed))));
                    });
                },
            );
        }

        group.finish();
    }
}

// Group registration + binary entry-point at crate root. `criterion_group!`
// expands to a `pub fn benches() { ... }`-style aggregator; `criterion_main!`
// generates `fn main()` that runs the aggregator. Both must live at the
// crate root (not inside `mod inner`) because `main` is the binary entry-
// point. `inner::bench_*` are `pub(super)` so the crate-root macro
// invocation can reach them.
#[cfg(feature = "internal-bench")]
criterion::criterion_group!(
    benches,
    inner::bench_eq_mod_cc,
    inner::bench_eq_mod_nf_short_circuit,
    inner::bench_cc_new,
    inner::bench_atom_canonical,
    inner::bench_smc_nf
);

// ---------------------------------------------------------------------------
// Crate-root binary entry-point — dual-branch cfg pattern
// ---------------------------------------------------------------------------
//
// Two `#[cfg]`-mutually-exclusive `main` definitions:
//
//   - With `--features internal-bench`: `criterion::criterion_main!(benches)`
//     expands to criterion's standard `main` that runs every group registered
//     in `benches`.
//
//   - Without the feature: the stub `fn main() {}` below makes the entire
//     bench inert — `cargo bench --bench presentation_bench` compiles to an
//     empty bench harness (no `criterion_main!` expansion, no group
//     registrations).
//
// Both branches use plain `//` comments, NOT `///` rustdoc — `criterion_main!`
// rejects doc comments because its expansion site is a macro invocation, and
// keeping the comment styles uniform across the two branches avoids a
// rustfmt asymmetry. Future maintainers: do NOT add `///` here without
// verifying both branches still expand cleanly under
// `--features internal-bench` AND default builds.

#[cfg(feature = "internal-bench")]
criterion::criterion_main!(benches);

#[cfg(not(feature = "internal-bench"))]
fn main() {}
