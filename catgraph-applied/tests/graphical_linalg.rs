//! CC completeness tracking for `S: SFG_R → Mat(R)` on bounded enumerations.
//!
//! # What these tests actually measure
//!
//! The 12 `cc_completeness_tracking_*` tests below are **NOT** Thm 5.60
//! faithfulness tests — that theorem is already proved abstractly by F&S
//! Thm 5.60 (`Free(Σ_SFG)/⟨E_{18}⟩ ≅ Mat(R)`, with `sfg_to_mat` realising the
//! isomorphism; proof via Baez-Erbele 2015 for fields, Wadsley–Woods
//! arXiv:1505.00048 for commutative rigs, cf. BE15 §6). We do not need to
//! verify an established theorem; this suite predates that reframing and was
//! originally mis-named.
//!
//! What the harness actually does: it enumerates SFG expressions up to
//! bounded depth, buckets them by matrix image under `sfg_to_mat`, and inside
//! each bucket takes the **connected components** of the graph whose edges are
//! the pairs `Presentation::eq_mod` proves equal under the 18 Thm 5.60
//! equations. A bucket of `k` components contributes `k − 1` "collisions" —
//! pairs of expressions CC leaves `eq_mod`-separated that the matrix functor
//! identifies, i.e. witnesses of the default [`CongruenceClosure`] engine's
//! syntactic incompleteness relative to the complete semantic engine
//! `NormalizeEngine::Functorial(MatrixNFFunctor)`. Components, not a greedy
//! scan against class representatives: `eq_mod` is not transitive, so only the
//! component count is a function of the relation
//! ([#189](https://github.com/sustia-llc/catgraph/issues/189); see below).
//!
//! # Resolved: the Functorial engine is the terminal Mat(R) decision path
//!
//! The Knuth-Bendix-vs-functorial decision (issue #15) is **resolved:
//! functorial-terminal**. [`Presentation::eq_mod_functorial`] with
//! [`MatrixNFFunctor<R>`] is the terminal, complete decision procedure for
//! Mat(R) (complete by theorem — F&S Thm 5.53 / Thm 5.60; via Baez-Erbele 2015
//! for fields, Wadsley–Woods arXiv:1505.00048 for commutative rigs). Plain
//! congruence closure stays **incomplete by design**: residual collisions all
//! exhibit the same structural pattern — derivation chains needing intermediate
//! composite terms not present in the CC term graph, which plain congruence
//! closure (with or without `smc_refine`) cannot synthesize. Closing that gap
//! by Knuth-Bendix completion of the 18 equations modulo SMC coherence is
//! demoted to a time-boxed feasibility spike (issue #57), relevant only for a
//! future non-Mat(R) presentation that lacks a semantic functor.
//!
//! # Bounded regression trackers (not a release gate)
//!
//! These stay `#[ignore]`'d — they are diagnostic, and `S(a) == S(b)` (matrix
//! equality under `sfg_to_mat`) is already decidable via
//! [`Presentation::eq_mod_functorial(&a, &b, &MatrixNFFunctor::new())`]. The
//! four depth-2 tests are **bounded regression trackers**. All four rigs are
//! **pinned exactly** (`assert_eq!`), so any move — rise or drop — trips the
//! test and demands an explanation. NOTE (#55 PR2 metric lesson, **narrowed at
//! #57 a1** — see "What this count measures changed" below): this count
//! conflates NF canonicality with bounded-depth *equational reach* (E_18
//! congruence bridges are co-adapted to the exact NFs), so a pin move is
//! evidence of change, not by itself proof of an NF regression or improvement.
//! Since a1 the *short-circuit* half of that is gone — the SMC layer is decided
//! exactly by content — but NF still reaches the count through
//! `kb::CongruenceClosure`'s `smc_refine`, so the caution stands.
//! Canonicality is judged by the
//! `smc_canonicality_probes` module in `smc_nf_completeness.rs` (SMC-equal
//! pairs asserted NF-equal directly — the unconfoundable metric); these pins
//! detect *unexplained* deltas. (F64Rig was a float-jitter band until #58 normalized
//! signed zero in the rig Hash impls; see below.) Depth-3/4 stay
//! `assert_eq!(.., 0)`: they are unmeasured (depth 3 is
//! over 10 min/rig in release, depth 4 larger still — and both grew further
//! when #189 made the within-bucket pass all-pairs), so on a manual `--ignored`
//! run the assert's LEFT value IS the true depth-N count (not an expectation).
//!
//! Fresh collision/expression counts (post-#189, release, depth 2):
//!
//! | rig          | collisions | expressions |
//! |--------------|-----------:|------------:|
//! | BoolRig      |        748 |       20324 |
//! | UnitInterval |       1114 |       31337 |
//! | Tropical     |       1594 |       46810 |
//! | F64Rig       |       1590 |       46810 |
//!
//! **What this count measures changed at #57 a1 (2026-07-29), partly.**
//! `Presentation::eq_mod` settles SMC coherence by *content* equality, which
//! decides it exactly (Lemma 4.1, `docs/SMC-NF-RECONCILIATION.md` §4.2), so
//! every same-matrix pair that is SMC-equal is merged before congruence closure
//! is consulted. What that removes is the **short-circuit** conflation: no
//! residual below is attributable to NF *incompleteness at the SMC layer*,
//! because that layer is now decided exactly.
//!
//! It does **not** make these counts NF-independent. `nf` is still the
//! canonicalizer inside `kb::CongruenceClosure`'s `smc_refine` fixpoint, which
//! rebuilds every term with atom-canonical substitutions, normalizes it with
//! `nf`, and merges the term's class with the normal form's — so an NF change
//! can still move these pins, through the user-equation layer rather than
//! through the SMC one. Treat a move as demanding an explanation exactly as
//! before.
//! [#173](https://github.com/sustia-llc/catgraph/issues/173)'s "conflation"
//! note is therefore **partially addressed**, not discharged; whether it closes
//! is an owner call, not this test's to make.
//!
//! BoolRig lineage: 2574 plain CC → 1433 atom-canonical `smc_refine` → 1301
//! post-#14 layer-ordering NF → 1142 post-E_18 (D7/D8 scalar add/zero added)
//! → 972 post-#55-PR1 (Step 6 η-before-ε within-layer reorder closed the
//! zero-arity tensor-order split; UnitInterval 1634 → 1400, Tropical
//! 2234 → 1930, F64Rig 2229 → 1925 in the same change)
//! → 979 post-#55-PR2 (rule-(i) component-anchored η placement + Step 7
//! component-block reorder; UnitInterval → 1432, Tropical → 2017,
//! F64Rig → 2012)
//! → 980 post-#174 (UnitInterval → 1433, Tropical → 2018, F64Rig → 2013)
//! → 952 post-#57-a1 (UnitInterval → 1397, Tropical → 1974, F64Rig → 1969)
//! → 748 post-#189 (UnitInterval → 1114, Tropical → 1594, F64Rig → 1590).
//! Only the last arrow is a change of *metric* rather than of NF or
//! presentation: #189 replaced the greedy class partition with connected
//! components, which is coarser or equal, so every rig fell without anything
//! about `eq_mod` moving.
//! Both rises are **equational-reach churn**, not NF regressions: the depth-2
//! E_18 congruence bridges are co-adapted to whatever the exact NFs were, and
//! redistributing NFs breaks some equation-mediated identifications while the
//! NF itself strictly improves. Canonicality is witnessed directly by the
//! `smc_canonicality_probes` module (see the module docstring note above), not
//! by this count; rigidity is proven on the fragment `𝔉′` only (§4.4
//! Theorem 4.5, canonicality there conditional on the filed cut-asymmetry
//! fix — rigidity on `𝔉` itself is withdrawn; the dominant remaining freedom
//! is `η` placement slack). Completing the presentation to all 18 F&S/BE15
//! relations gives CC more equations to identify with, lowering the residual
//! collision count.
//!
//! The **#57-a1 drop is not churn**, and it is worth being precise about what
//! was guaranteed in advance and what was measured. This metric buckets by
//! matrix image first, and Thm 5.60 makes the matrix ground truth, so
//! everything inside a bucket really is equal in the presented prop: a bucket
//! splitting into `k` `eq_mod`-components contributes `k − 1`, and the count
//! therefore measures equalities `eq_mod` *fails to prove*.
//!
//! What is **forced** is a statement about the underlying relation, not about
//! the count. Wiring content in replaced the `nf(a) == nf(b)` short-circuit
//! with content equality, and the content relation **contains** the NF relation
//! — `nf` preserves content (§4.3 Lemma 4.2), so equal normal forms force equal
//! content. The relation therefore only grew: no pair that `eq_mod` could
//! previously prove equal became unprovable.
//!
//! At the time that did **not** force the count down. The partition was then
//! built *greedily* — each expression joined the first class whose
//! representative it matched — over an `eq_mod` that is **not transitive**:
//! `Scalar(false)` ~ `Discard ; Zero` ~ `Discard ⊗ Zero` while `Scalar(false)`
//! ≁ `Discard ⊗ Zero` (#189 measured 10 490 ordered violating triples on a
//! 120-expression pool of parallel `1 → 1` arrows, zero `None` verdicts). Over a
//! non-transitive relation the greedy class count is not a function of the
//! relation at all — it depends on enumeration order — so enlarging the
//! relation had no monotonicity theorem behind it, and the #57-a1 direction was
//! **empirical**: all four rigs fell.
//!
//! **[#189](https://github.com/sustia-llc/catgraph/issues/189) closed that
//! hole.** The partition is now the connected components of the same
//! `Some(true)` edge set. Components are the transitive closure of that set, so
//! the count *is* a function of the relation, and a relation that only grows
//! can only gain edges and therefore only merge components: the counts move
//! **monotonically down** under relation growth, which is the argument this
//! file's re-pins are now read under. The switch itself was a one-off
//! re-baseline — every greedy class sits inside one component, so components
//! are coarser-or-equal and all four rigs necessarily fell (952 → 748,
//! 1397 → 1114, 1974 → 1594, 1969 → 1590), with no change to `eq_mod`, `nf`, or
//! the presentation. What the count still cannot do is certify canonicality;
//! that remains the `smc_canonicality_probes`' job.
//!
//! The containment is pinned as a test on 2000 unrelated pairs in
//! `tests/content_equality_corpus.rs` (`cross_corpus_pairs_are_separated`),
//! though note it only bites where `nf` agrees, which is rare on unrelated
//! pairs — the direct check of Lemma 4.2 is
//! `nf_preserves_content_across_the_corpus` in the same file. Note also what
//! this metric cannot see: merges only ever happen *within* a matrix bucket, so
//! it is structurally blind to a false equality across buckets — soundness is
//! covered by that file's negative controls and by `thm_5_60_soundness_*` here,
//! not by these counts.
//!
//! The #174 delta is **+1 on every rig, one new collision pair and none lost**,
//! and it is attributable to the *free-site retirement* (§2.6) rather than to
//! the Step 6½ column pass: the pass's own residuals need expression depth ≥ 3
//! and are structurally invisible here, and the pins did not move when the pass
//! alone was measured. The pair (BoolRig) is
//! `Tensor(Tensor(Braid(1,1), Generator(Discard)), Compose(Generator(Zero), Generator(Copy)))`
//! against
//! `Tensor(Tensor(Braid(1,1), Generator(Zero)), Tensor(Generator(Discard), Generator(Zero)))`
//! — two expressions the simplified tied comparator now separates. The same act
//! closes CE-A3 (`nested_source_block_converges_with_free_writing`), so the
//! collision and the closure are inseparable.
//!
//! All four counts are deterministic and their trackers are pinned exactly.
//! F64Rig's count was float-nondeterministic until #58 (observed 2478–2480 —
//! signed-zero `Hash`/`Eq` interacted with HashMap ordering): the fixture's
//! `-1.0 × 0.0` yields `-0.0`, which the rig `Hash` impls hashed differently
//! from `0.0` while the derived `PartialEq` treated them equal, splitting a
//! congruence class. #58 normalized `-0.0` to `0.0` in those `Hash` impls,
//! restoring the `Eq`/`Hash` contract and making F64Rig an exact pin (2229
//! then; 1925 post-#55-PR1; 2012 post-#55-PR2; 2013 post-#174; 1969 post-#57-a1;
//! 1590 post-#189). All baselines
//! live in the `BASELINE_*_D2` module consts.
//!
//! #58 had a sequel: the harness's *bucket* key was a `Debug` string until
//! [#167](https://github.com/sustia-llc/catgraph/issues/167), and derived
//! `Debug` on `F64Rig(pub f64)` renders the sign bit — reinstating the very
//! `-0.0`/`0.0` split #58 closed, one layer up. The key is the matrix value
//! now. None of the four baselines moved: `MatR::matmul` accumulates from
//! `R::zero()`, so `-1.0 × 0.0` is summed into `+0.0` before it is stored and
//! these fixtures never build a `-0.0` entry. It is reachable through
//! `rig_samples`, which is why `signed_zero_is_one_matrix_bucket_not_two`
//! (non-ignored, below) pins it.
//!
//! [`CongruenceClosure`]: catgraph_applied::prop::presentation::NormalizeEngine::CongruenceClosure
//! [`MatrixNFFunctor<R>`]: catgraph_applied::prop::presentation::functorial::MatrixNFFunctor
//! [`Presentation::eq_mod_functorial`]: catgraph_applied::prop::presentation::Presentation::eq_mod_functorial

use catgraph_applied::{
    graphical_linalg::{matr_presentation, verify_sfg_to_mat_is_full_and_faithful},
    mat::MatR,
    rig::{BoolRig, F64Rig, Rig, Tropical, UnitInterval},
    sfg::SignalFlowGraph,
    sfg_to_mat::sfg_to_mat,
};

// ---- Smoke tests (always active): the presentation builds across all rigs ----

/// #167: `verify_sfg_to_mat_is_full_and_faithful` buckets by the matrix
/// **value**, not by a `Debug` rendering of it.
///
/// The retired key was `format!("{}×{} {:?}", rows, cols, entries)`. `Debug` on
/// `F64Rig(pub f64)` is derived, so it renders the sign bit — reinstating,
/// in the bucketing, the very `-0.0`/`0.0` split #58 closed in the rig
/// `Hash` impls. This test pins both halves of the replacement and the
/// reachability claim the rustdoc makes about it.
#[test]
fn signed_zero_is_one_matrix_bucket_not_two() {
    /// Exactly the key the harness builds. `MatR` itself cannot be the key —
    /// it derives only `Clone, Debug, PartialEq` — so shape rides alongside
    /// the entries (`Zero : 0 → 1` and `Discard : 1 → 0` are the shapes that
    /// need it: their `entries()` carry no column count).
    fn key(m: &MatR<F64Rig>) -> (usize, usize, Vec<Vec<F64Rig>>) {
        (m.rows(), m.cols(), m.entries().to_vec())
    }

    // Both values hashed with the SAME `BuildHasher`; a per-call `RandomState`
    // reseeds and would make even identical values disagree.
    fn hashes_agree<T: std::hash::Hash>(a: &T, b: &T) -> bool {
        use std::hash::{BuildHasher, RandomState};
        let state = RandomState::new();
        state.hash_one(a) == state.hash_one(b)
    }

    let pos = MatR::new(1, 1, vec![vec![F64Rig(0.0)]]).unwrap();
    let neg = MatR::new(1, 1, vec![vec![F64Rig(-0.0)]]).unwrap();

    // The retired key really did split them...
    assert_ne!(
        format!("{}×{} {:?}", pos.rows(), pos.cols(), pos.entries()),
        format!("{}×{} {:?}", neg.rows(), neg.cols(), neg.entries()),
        "if `Debug` ever stopped rendering the sign bit this test would be \
         vacuous — it asserts the old key's defect, not the new key's virtue"
    );

    // ...and the live key does not. `Eq` alone is not enough for a `HashMap`:
    // the entries must hash alike too, which is what #58 bought.
    assert_eq!(key(&pos), key(&neg));
    assert!(hashes_agree(&key(&pos), &key(&neg)));

    // Reachability, in both directions.
    //
    // A `Scalar` generator puts its sample straight into a 1×1 matrix, so a
    // caller passing `F64Rig(-0.0)` in `rig_samples` — a public argument —
    // triggers the split immediately.
    let from_scalar = sfg_to_mat(&SignalFlowGraph::scalar(F64Rig(-0.0))).unwrap();
    assert!(from_scalar.entries()[0][0].0.is_sign_negative());

    // The shipped depth-2 fixtures cannot: every other entry comes from
    // `R::zero()`, `R::one()`, or `MatR::matmul`, and matmul accumulates from
    // `R::zero()`, so the `-1.0 × 0.0` the #58 note names is summed into
    // `+0.0` before it is ever stored. That is why the pinned baselines below
    // do not move under #167 — and this assertion is what makes the claim fail
    // loudly if the accumulation is ever restructured.
    let via_matmul = sfg_to_mat(
        &SignalFlowGraph::scalar(F64Rig(-1.0))
            .compose(&SignalFlowGraph::scalar(F64Rig(0.0)))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(via_matmul.entries()[0][0], F64Rig(0.0));
    assert!(via_matmul.entries()[0][0].0.is_sign_positive());
}

#[test]
fn matr_presentation_builds_bool() {
    let samples = vec![BoolRig(false), BoolRig(true)];
    matr_presentation::<BoolRig>(&samples).unwrap();
}

#[test]
fn matr_presentation_builds_f64() {
    let samples = vec![F64Rig(0.0), F64Rig(1.0), F64Rig(2.0)];
    matr_presentation::<F64Rig>(&samples).unwrap();
}

#[test]
fn matr_presentation_builds_tropical() {
    let samples = vec![Tropical(f64::INFINITY), Tropical(0.0), Tropical(1.0)];
    matr_presentation::<Tropical>(&samples).unwrap();
}

#[test]
fn matr_presentation_builds_unit_interval() {
    let samples = vec![
        UnitInterval::new(0.0).unwrap(),
        UnitInterval::new(0.5).unwrap(),
        UnitInterval::new(1.0).unwrap(),
    ];
    matr_presentation::<UnitInterval>(&samples).unwrap();
}

// Post-#189 depth-2 collision baselines (the partition switched from a greedy
// scan against class representatives to the connected components of the
// `eq_mod`-equality graph) — the single Rust source of truth for each number
// (mirrored in the module docstring table; the pin-guard in
// `scripts/check_audit_counts.py` scans the prose sites against these). All
// four rigs are deterministic → pinned exactly (F64Rig was a
// float-nondeterministic jitter band until #58 normalized signed zero in the
// rig Hash impls). Prior pins: 952/1397/1974/1969 (post-#57-a1);
// 980/1433/2018/2013 (post-#174); 979/1432/2017/2012 (post-#55-PR2);
// 972/1400/1930/1925 (post-#55-PR1); 1142/1634/2234/2229 (post-E_18/#58).
// The #189 drop is a metric change, not an NF or presentation change:
// components are coarser-or-equal than any greedy partition of the same edge
// set, so all four had to fall. The earlier rises are equational-reach churn —
// see the module docstring and `smc_canonicality_probes`.
const BASELINE_BOOL_D2: usize = 748;
const BASELINE_UNIT_INTERVAL_D2: usize = 1114;
const BASELINE_TROPICAL_D2: usize = 1594;
const BASELINE_F64_D2: usize = 1590;

const IGNORE_REASON: &str = "\
    CC completeness tracking (NOT a Thm 5.60 faithfulness test): F&S Thm 5.60 \
    proves `Free(Σ_SFG)/⟨E_{18}⟩ ≅ Mat(R)` abstractly (via Baez-Erbele 2015 for \
    fields, Wadsley–Woods arXiv:1505.00048 for commutative rigs) — we do not \
    need to empirically verify the theorem. These tests bound the incompleteness of \
    the default `NormalizeEngine::CongruenceClosure` engine against the \
    matrix ground truth on bounded-depth enumeration. Issue #15 is resolved \
    functorial-terminal: `Presentation::eq_mod_functorial` with \
    `MatrixNFFunctor` is the complete, terminal Mat(R) decision procedure; \
    plain CC stays incomplete by design (Knuth-Bendix completion demoted to \
    the #57 feasibility spike). The depth-2 tests are bounded regression \
    trackers at the pinned NF baselines (see the module docstring table and \
    the `BASELINE_*_D2` consts): all four rigs are pinned exactly (F64Rig was a \
    jitter band until #58 normalized signed zero in the rig Hash impls). \
    `#[ignore]`'d as diagnostic, not a release gate.\
";

// Shared message for the unmeasured depth-3/4 trackers: the assert's LEFT value
// in the failure output IS the true depth-N collision count, not an expectation
// of 0 (they are far too slow to pin; see the module docstring).
const UNMEASURED_MSG: &str = "\
    unmeasured depth-3/4 diagnostic: the assert's LEFT value is the true \
    collision count at this depth (NOT an expectation of 0); this run is far too \
    slow to pin a baseline — see the module docstring. Not a release gate.\
";

fn witness_debug<R>(
    report: &catgraph_applied::graphical_linalg::FaithfulnessReport<R>,
) -> Option<(String, String)>
where
    R: catgraph_applied::rig::Rig + std::fmt::Debug + Eq + std::hash::Hash + Ord + 'static,
{
    report.witnesses.first().map(|(a, b)| {
        (
            format!("{:?}", a.as_prop_expr()),
            format!("{:?}", b.as_prop_expr()),
        )
    })
}

/// Two-sided exact pin for a deterministic rig's depth-2 collision count. Any
/// move — rise or drop — must be noticed and explained, so this pins rather
/// than bounds. The count conflates canonicality with bounded-depth
/// equational reach (#55 PR2 metric lesson): judge NF changes by the
/// `smc_canonicality_probes`, use this pin to catch *unexplained* deltas
/// (e.g. an unsound CC over-merge, or an accidental NF/corpus change).
fn assert_exact_baseline<R>(
    rig: &str,
    report: &catgraph_applied::graphical_linalg::FaithfulnessReport<R>,
    baseline: usize,
) where
    R: catgraph_applied::rig::Rig + std::fmt::Debug + Eq + std::hash::Hash + Ord + 'static,
{
    assert_eq!(
        report.collisions_under_s,
        baseline,
        "{rig} depth 2: {} expressions, {} collisions != pinned baseline {baseline} \
         (the count conflates canonicality with depth-2 equational reach — diagnose \
         via the smc_canonicality_probes and the witness diff; re-baseline only after \
         the delta is explained). First witness: {:?}. {IGNORE_REASON}",
        report.expressions_checked,
        report.collisions_under_s,
        witness_debug(report),
    );
}

// ---- BoolRig × {2, 3, 4} ----

#[test]
#[ignore = "CC completeness tracking; see module docstring and IGNORE_REASON"]
fn cc_completeness_tracking_bool_depth_2() {
    // Post-#189 baseline: 748 collisions / 20324 expressions (deterministic;
    // pinned exactly via `assert_exact_baseline`).
    let samples = vec![BoolRig(false), BoolRig(true)];
    let report = verify_sfg_to_mat_is_full_and_faithful::<BoolRig>(2, &samples).unwrap();
    assert_exact_baseline("BoolRig", &report, BASELINE_BOOL_D2);
}

#[test]
#[ignore = "CC completeness tracking; see module docstring and IGNORE_REASON"]
fn cc_completeness_tracking_bool_depth_3() {
    let samples = vec![BoolRig(false), BoolRig(true)];
    let report = verify_sfg_to_mat_is_full_and_faithful::<BoolRig>(3, &samples).unwrap();
    assert_eq!(report.collisions_under_s, 0, "{UNMEASURED_MSG}");
}

#[test]
#[ignore = "CC completeness tracking; see module docstring and IGNORE_REASON"]
fn cc_completeness_tracking_bool_depth_4() {
    let samples = vec![BoolRig(false), BoolRig(true)];
    let report = verify_sfg_to_mat_is_full_and_faithful::<BoolRig>(4, &samples).unwrap();
    assert_eq!(report.collisions_under_s, 0, "{UNMEASURED_MSG}");
}

// ---- UnitInterval × {2, 3, 4} ----

#[test]
#[ignore = "CC completeness tracking; see module docstring and IGNORE_REASON"]
fn cc_completeness_tracking_unit_interval_depth_2() {
    let samples = vec![
        UnitInterval::new(0.0).unwrap(),
        UnitInterval::new(0.5).unwrap(),
        UnitInterval::new(1.0).unwrap(),
    ];
    // Post-#189 baseline: 1114 collisions / 31337 expressions (deterministic;
    // pinned exactly).
    let report = verify_sfg_to_mat_is_full_and_faithful::<UnitInterval>(2, &samples).unwrap();
    assert_exact_baseline("UnitInterval", &report, BASELINE_UNIT_INTERVAL_D2);
}

#[test]
#[ignore = "CC completeness tracking; see module docstring and IGNORE_REASON"]
fn cc_completeness_tracking_unit_interval_depth_3() {
    let samples = vec![
        UnitInterval::new(0.0).unwrap(),
        UnitInterval::new(0.5).unwrap(),
        UnitInterval::new(1.0).unwrap(),
    ];
    let report = verify_sfg_to_mat_is_full_and_faithful::<UnitInterval>(3, &samples).unwrap();
    assert_eq!(report.collisions_under_s, 0, "{UNMEASURED_MSG}");
}

#[test]
#[ignore = "CC completeness tracking; see module docstring and IGNORE_REASON"]
fn cc_completeness_tracking_unit_interval_depth_4() {
    let samples = vec![
        UnitInterval::new(0.0).unwrap(),
        UnitInterval::new(0.5).unwrap(),
        UnitInterval::new(1.0).unwrap(),
    ];
    let report = verify_sfg_to_mat_is_full_and_faithful::<UnitInterval>(4, &samples).unwrap();
    assert_eq!(report.collisions_under_s, 0, "{UNMEASURED_MSG}");
}

// ---- Tropical × {2, 3, 4} ----

#[test]
#[ignore = "CC completeness tracking; see module docstring and IGNORE_REASON"]
fn cc_completeness_tracking_tropical_depth_2() {
    let samples = vec![
        Tropical(f64::INFINITY),
        Tropical(0.0),
        Tropical(1.0),
        Tropical(2.0),
    ];
    // Post-#189 baseline: 1594 collisions / 46810 expressions (deterministic;
    // pinned exactly).
    let report = verify_sfg_to_mat_is_full_and_faithful::<Tropical>(2, &samples).unwrap();
    assert_exact_baseline("Tropical", &report, BASELINE_TROPICAL_D2);
}

#[test]
#[ignore = "CC completeness tracking; see module docstring and IGNORE_REASON"]
fn cc_completeness_tracking_tropical_depth_3() {
    let samples = vec![
        Tropical(f64::INFINITY),
        Tropical(0.0),
        Tropical(1.0),
        Tropical(2.0),
    ];
    let report = verify_sfg_to_mat_is_full_and_faithful::<Tropical>(3, &samples).unwrap();
    assert_eq!(report.collisions_under_s, 0, "{UNMEASURED_MSG}");
}

#[test]
#[ignore = "CC completeness tracking; see module docstring and IGNORE_REASON"]
fn cc_completeness_tracking_tropical_depth_4() {
    let samples = vec![
        Tropical(f64::INFINITY),
        Tropical(0.0),
        Tropical(1.0),
        Tropical(2.0),
    ];
    let report = verify_sfg_to_mat_is_full_and_faithful::<Tropical>(4, &samples).unwrap();
    assert_eq!(report.collisions_under_s, 0, "{UNMEASURED_MSG}");
}

// ---- F64Rig × {2, 3, 4} ----

#[test]
#[ignore = "CC completeness tracking; see module docstring and IGNORE_REASON"]
fn cc_completeness_tracking_f64_depth_2() {
    // Post-#189 baseline: 1590 collisions / 46810 expressions (deterministic;
    // pinned exactly). Before #58, F64Rig's count was float-nondeterministic
    // (signed-zero Hash/Eq interacted with HashMap ordering); normalizing `-0.0`
    // to `0.0` in the rig Hash impls restored the Eq/Hash contract, merging the
    // `-0.0` produced by `-1.0 × 0.0` with `0.0` and making the count exact.
    let samples = vec![F64Rig(0.0), F64Rig(1.0), F64Rig(2.0), F64Rig(-1.0)];
    let report = verify_sfg_to_mat_is_full_and_faithful::<F64Rig>(2, &samples).unwrap();
    assert_exact_baseline("F64Rig", &report, BASELINE_F64_D2);
}

#[test]
#[ignore = "CC completeness tracking; see module docstring and IGNORE_REASON"]
fn cc_completeness_tracking_f64_depth_3() {
    let samples = vec![F64Rig(0.0), F64Rig(1.0), F64Rig(2.0), F64Rig(-1.0)];
    let report = verify_sfg_to_mat_is_full_and_faithful::<F64Rig>(3, &samples).unwrap();
    assert_eq!(report.collisions_under_s, 0, "{UNMEASURED_MSG}");
}

#[test]
#[ignore = "CC completeness tracking; see module docstring and IGNORE_REASON"]
fn cc_completeness_tracking_f64_depth_4() {
    let samples = vec![F64Rig(0.0), F64Rig(1.0), F64Rig(2.0), F64Rig(-1.0)];
    let report = verify_sfg_to_mat_is_full_and_faithful::<F64Rig>(4, &samples).unwrap();
    assert_eq!(report.collisions_under_s, 0, "{UNMEASURED_MSG}");
}

// ---- Thm 5.60 soundness: every equation in the presentation is a matrix equality under S ----

/// For each equation `(lhs, rhs)` in the Thm 5.60 presentation, verify that
/// `S(lhs) == S(rhs)` under `sfg_to_mat`. This is the SOUNDNESS direction
/// (S is well-defined on the quotient); the FAITHFULNESS direction (S is
/// injective on the quotient) is decided operationally by the terminal
/// Functorial engine (`eq_mod_functorial`, complete by F&S Thm 5.60 — via
/// Baez-Erbele 2015 for fields, Wadsley–Woods arXiv:1505.00048 for commutative
/// rigs) — issue #15 resolved functorial-terminal, with syntactic Knuth-Bendix
/// completion demoted to the #57 feasibility spike.
fn assert_soundness_for_rig<R>(rig_samples: &[R]) -> String
where
    R: Rig + std::fmt::Debug + Eq + std::hash::Hash + Ord + 'static,
{
    let presentation = matr_presentation::<R>(rig_samples).expect("matr_presentation builds");

    let mut violations: Vec<String> = Vec::new();
    for (i, (lhs, rhs)) in presentation.equations().iter().enumerate() {
        let lhs_sfg = SignalFlowGraph::<R>::from_prop_expr(lhs.clone());
        let rhs_sfg = SignalFlowGraph::<R>::from_prop_expr(rhs.clone());

        let lhs_mat = sfg_to_mat(&lhs_sfg);
        let rhs_mat = sfg_to_mat(&rhs_sfg);

        match (lhs_mat, rhs_mat) {
            (Ok(a), Ok(b)) => {
                if a != b {
                    violations.push(format!(
                        "eq #{i}: sfg_to_mat(lhs) != sfg_to_mat(rhs)\n  lhs={lhs:?}\n  rhs={rhs:?}\n  S(lhs)={a:?}\n  S(rhs)={b:?}"
                    ));
                }
            }
            (e_a, e_b) => {
                violations.push(format!(
                    "eq #{i}: sfg_to_mat failed: lhs={e_a:?}, rhs={e_b:?}"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Soundness violations: {}",
        violations.join("\n\n")
    );
    format!("{} equations sound under S", presentation.equations().len())
}

#[test]
fn thm_5_60_soundness_f64() {
    let samples = vec![F64Rig(0.0), F64Rig(1.0), F64Rig(2.0), F64Rig(-1.0)];
    let report = assert_soundness_for_rig::<F64Rig>(&samples);
    println!("F64Rig: {report}");
}

#[test]
fn thm_5_60_soundness_bool() {
    let samples = vec![BoolRig(false), BoolRig(true)];
    let report = assert_soundness_for_rig::<BoolRig>(&samples);
    println!("BoolRig: {report}");
}

#[test]
fn thm_5_60_soundness_unit_interval() {
    let samples = vec![
        UnitInterval::new(0.0).unwrap(),
        UnitInterval::new(0.5).unwrap(),
        UnitInterval::new(1.0).unwrap(),
    ];
    let report = assert_soundness_for_rig::<UnitInterval>(&samples);
    println!("UnitInterval: {report}");
}

#[test]
fn thm_5_60_soundness_tropical() {
    let samples = vec![
        Tropical(f64::INFINITY),
        Tropical(0.0),
        Tropical(1.0),
        Tropical(2.0),
    ];
    let report = assert_soundness_for_rig::<Tropical>(&samples);
    println!("Tropical: {report}");
}
