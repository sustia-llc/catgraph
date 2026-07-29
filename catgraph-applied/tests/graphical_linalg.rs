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
//! bounded depth, buckets them by `Presentation::eq_mod` under the 18 Thm
//! 5.60 equations, then checks that every bucket maps to a single matrix
//! under `sfg_to_mat`. A "collision" is a pair of expressions CC decides
//! are `eq_mod`-distinct that the matrix functor identifies — i.e., a
//! witness of the default [`CongruenceClosure`] engine's syntactic
//! incompleteness relative to the complete semantic engine
//! `NormalizeEngine::Functorial(MatrixNFFunctor)`.
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
//! test and demands an explanation. NOTE (#55 PR2 metric lesson): this count
//! conflates NF canonicality with bounded-depth *equational reach* (E_18
//! congruence bridges are co-adapted to the exact NFs), so a pin move is
//! evidence of change, not by itself proof of an NF regression or
//! improvement. Canonicality is judged by the `smc_canonicality_probes`
//! module in `smc_nf_completeness.rs` (SMC-equal pairs asserted NF-equal
//! directly — the unconfoundable metric); these pins detect *unexplained*
//! deltas. (F64Rig was a float-jitter band until #58 normalized
//! signed zero in the rig Hash impls; see below.) Depth-3/4 stay
//! `assert_eq!(.., 0)`: they are unmeasured (depth 3 is
//! over 10 min/rig in release, depth 4 larger still), so on a manual `--ignored`
//! run the assert's LEFT value IS the true depth-N count (not an expectation).
//!
//! Fresh collision/expression counts (post-#174, release, depth 2):
//!
//! | rig          | collisions | expressions |
//! |--------------|-----------:|------------:|
//! | BoolRig      |        980 |       20324 |
//! | UnitInterval |       1433 |       31337 |
//! | Tropical     |       2018 |       46810 |
//! | F64Rig       |       2013 |       46810 |
//!
//! BoolRig lineage: 2574 plain CC → 1433 atom-canonical `smc_refine` → 1301
//! post-#14 layer-ordering NF → 1142 post-E_18 (D7/D8 scalar add/zero added)
//! → 972 post-#55-PR1 (Step 6 η-before-ε within-layer reorder closed the
//! zero-arity tensor-order split; UnitInterval 1634 → 1400, Tropical
//! 2234 → 1930, F64Rig 2229 → 1925 in the same change)
//! → 979 post-#55-PR2 (rule-(i) component-anchored η placement + Step 7
//! component-block reorder; UnitInterval → 1432, Tropical → 2017,
//! F64Rig → 2012)
//! → 980 post-#174 (UnitInterval → 1433, Tropical → 2018, F64Rig → 2013).
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
//! then; 1925 post-#55-PR1; 2012 post-#55-PR2; 2013 post-#174). All baselines
//! live in the `BASELINE_*_D2` module consts.
//!
//! [`CongruenceClosure`]: catgraph_applied::prop::presentation::NormalizeEngine::CongruenceClosure
//! [`MatrixNFFunctor<R>`]: catgraph_applied::prop::presentation::functorial::MatrixNFFunctor
//! [`Presentation::eq_mod_functorial`]: catgraph_applied::prop::presentation::Presentation::eq_mod_functorial

use catgraph_applied::{
    graphical_linalg::{matr_presentation, verify_sfg_to_mat_is_full_and_faithful},
    rig::{BoolRig, F64Rig, Rig, Tropical, UnitInterval},
    sfg::SignalFlowGraph,
    sfg_to_mat::sfg_to_mat,
};

// ---- Smoke tests (always active): the presentation builds across all rigs ----

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

// Post-#174 depth-2 collision baselines (Step 6½ column pass + the free-site
// retirement of the component-anchored η slot walk) — the single Rust source of
// truth for each number (mirrored in the module docstring table; the
// pin-guard in `scripts/check_audit_counts.py` scans the prose sites against
// these). All four rigs are deterministic → pinned exactly (F64Rig was a
// float-nondeterministic jitter band until #58 normalized signed zero in the
// rig Hash impls). Prior pins: 979/1432/2017/2012 (post-#55-PR2);
// 972/1400/1930/1925 (post-#55-PR1); 1142/1634/2234/2229 (post-E_18/#58).
// Both rises are equational-reach churn — see the module docstring and
// `smc_canonicality_probes`.
const BASELINE_BOOL_D2: usize = 980;
const BASELINE_UNIT_INTERVAL_D2: usize = 1433;
const BASELINE_TROPICAL_D2: usize = 2018;
const BASELINE_F64_D2: usize = 2013;

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
    // Post-#174 baseline: 980 collisions / 20324 expressions (deterministic;
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
    // Post-#174 baseline: 1433 collisions / 31337 expressions (deterministic;
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
    // Post-#174 baseline: 2018 collisions / 46810 expressions (deterministic;
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
    // Post-#174 baseline: 2013 collisions / 46810 expressions (deterministic;
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
