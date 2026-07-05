//! CC completeness tracking for `S: SFG_R → Mat(R)` on bounded enumerations.
//!
//! # What these tests actually measure
//!
//! The 12 `cc_completeness_tracking_*` tests below are **NOT** Thm 5.60
//! faithfulness tests — that theorem is already proved abstractly by
//! Baez-Erbele 2015 (`Free(Σ_SFG)/⟨E_{17}⟩ ≅ Mat(R)`, with `sfg_to_mat`
//! realising the isomorphism). We do not need to verify an established
//! theorem; this suite predates that reframing and was originally mis-named.
//!
//! What the harness actually does: it enumerates SFG expressions up to
//! bounded depth, buckets them by `Presentation::eq_mod` under the 17 Thm
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
//! Mat(R) (complete by theorem — F&S Thm 5.53 / Baez-Erbele 2015). Plain
//! congruence closure stays **incomplete by design**: residual collisions all
//! exhibit the same structural pattern — derivation chains needing intermediate
//! composite terms not present in the CC term graph, which plain congruence
//! closure (with or without `smc_refine`) cannot synthesize. Closing that gap
//! by Knuth-Bendix completion of the 17 equations modulo SMC coherence is
//! demoted to a time-boxed feasibility spike (issue #57), relevant only for a
//! future non-Mat(R) presentation that lacks a semantic functor.
//!
//! # Bounded regression trackers (not a release gate)
//!
//! These stay `#[ignore]`'d — they are diagnostic, and `S(a) == S(b)` (matrix
//! equality under `sfg_to_mat`) is already decidable via
//! [`Presentation::eq_mod_functorial(&a, &b, &MatrixNFFunctor::new())`]. The
//! four depth-2 tests are **bounded regression trackers**. The three
//! deterministic rigs are **pinned exactly** (`assert_eq!`), so both a rise
//! (CC/NF regression) and a silent drop (KB-like progress, or an unsound CC
//! over-merge — both must be noticed) trip the test; F64Rig is a float-jitter
//! **band**. Depth-3/4 stay `assert_eq!(.., 0)`: they are unmeasured (depth 3 is
//! over 10 min/rig in release, depth 4 larger still), so on a manual `--ignored`
//! run the assert's LEFT value IS the true depth-N count (not an expectation).
//!
//! Fresh collision/expression counts (post-#14 NF, release, depth 2):
//!
//! | rig          | collisions | expressions |
//! |--------------|-----------:|------------:|
//! | BoolRig      |       1301 |       20324 |
//! | UnitInterval |       1856 |       31337 |
//! | Tropical     |       2526 |       46810 |
//! | F64Rig       |      ~2777 |       46810 |
//!
//! BoolRig lineage: 2574 plain CC → 1433 atom-canonical `smc_refine` → 1301
//! post-#14 layer-ordering NF (~49% total).
//!
//! BoolRig/UnitInterval/Tropical counts are deterministic and their tracker
//! bounds are pinned exactly. F64Rig's count is float-nondeterministic
//! (observed 2776–2778 — signed-zero `Hash`/`Eq` interacts with HashMap
//! ordering), so its tracker is an inclusive jitter band (`2770..=2790`,
//! tracked as #58). All baselines live in the `BASELINE_*_D2` module consts.
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

// Post-#14 depth-2 collision baselines — the single Rust source of truth for
// each number (mirrored in the module docstring table). BoolRig/UnitInterval/
// Tropical are deterministic → pinned exactly; F64Rig is float-nondeterministic
// (signed-zero `Hash`/`Eq` × HashMap ordering; observed 2776–2778) → an
// inclusive jitter band, tracked as #58.
const BASELINE_BOOL_D2: usize = 1301;
const BASELINE_UNIT_INTERVAL_D2: usize = 1856;
const BASELINE_TROPICAL_D2: usize = 2526;
const BASELINE_F64_D2: std::ops::RangeInclusive<usize> = 2770..=2790;

const IGNORE_REASON: &str = "\
    CC completeness tracking (NOT a Thm 5.60 faithfulness test): Baez-Erbele \
    2015 proved `Free(Σ_SFG)/⟨E_{17}⟩ ≅ Mat(R)` abstractly — we do not need to \
    empirically verify the theorem. These tests bound the incompleteness of \
    the default `NormalizeEngine::CongruenceClosure` engine against the \
    matrix ground truth on bounded-depth enumeration. Issue #15 is resolved \
    functorial-terminal: `Presentation::eq_mod_functorial` with \
    `MatrixNFFunctor` is the complete, terminal Mat(R) decision procedure; \
    plain CC stays incomplete by design (Knuth-Bendix completion demoted to \
    the #57 feasibility spike). The depth-2 tests are bounded regression \
    trackers at the post-#14 NF baselines (see the module docstring table and \
    the `BASELINE_*_D2` consts): the three deterministic rigs are pinned \
    exactly, F64Rig is a jitter band (#58). `#[ignore]`'d as diagnostic, not a \
    release gate.\
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
    R: catgraph_applied::rig::Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static,
{
    report.witnesses.first().map(|(a, b)| {
        (
            format!("{:?}", a.as_prop_expr()),
            format!("{:?}", b.as_prop_expr()),
        )
    })
}

/// Two-sided exact pin for a deterministic rig's depth-2 collision count. A rise
/// is a CC/NF regression; a silent drop is KB-like progress OR an unsound CC
/// over-merge — both must be noticed, so this pins rather than bounds.
fn assert_exact_baseline<R>(
    rig: &str,
    report: &catgraph_applied::graphical_linalg::FaithfulnessReport<R>,
    baseline: usize,
) where
    R: catgraph_applied::rig::Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static,
{
    assert_eq!(
        report.collisions_under_s,
        baseline,
        "{rig} depth 2: {} expressions, {} collisions != pinned post-#14 baseline {baseline} \
         (a rise = CC/NF regression; a drop = KB-like progress or unsound CC over-merge — \
         re-baseline only after confirming which). First witness: {:?}. {IGNORE_REASON}",
        report.expressions_checked,
        report.collisions_under_s,
        witness_debug(report),
    );
}

// ---- BoolRig × {2, 3, 4} ----

#[test]
#[ignore = "CC completeness tracking; see module docstring and IGNORE_REASON"]
fn cc_completeness_tracking_bool_depth_2() {
    // Post-#14 NF baseline: 1301 collisions / 20324 expressions (deterministic;
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
    // Post-#14 NF baseline: 1856 collisions / 31337 expressions (deterministic;
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
    // Post-#14 NF baseline: 2526 collisions / 46810 expressions (deterministic;
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
    // Post-#14 NF baseline: ~2777 collisions / 46810 expressions. Unlike the
    // other three rigs, F64Rig's collision count is float-nondeterministic
    // (observed 2776–2778 across runs — signed-zero Hash/Eq interacts with
    // HashMap ordering; tracked in #58), so it is checked against an inclusive
    // jitter band rather than an exact pin. A real CC/NF regression moves the
    // count structurally, far outside the band.
    let samples = vec![F64Rig(0.0), F64Rig(1.0), F64Rig(2.0), F64Rig(-1.0)];
    let report = verify_sfg_to_mat_is_full_and_faithful::<F64Rig>(2, &samples).unwrap();
    assert!(
        BASELINE_F64_D2.contains(&report.collisions_under_s),
        "F64Rig depth 2: {} expressions, {} collisions outside jitter band {:?} (#58); first witness: {:?}. {IGNORE_REASON}",
        report.expressions_checked,
        report.collisions_under_s,
        BASELINE_F64_D2,
        witness_debug(&report),
    );
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
/// Functorial engine (`eq_mod_functorial`, complete by Baez-Erbele 2015) —
/// issue #15 resolved functorial-terminal, with syntactic Knuth-Bendix
/// completion demoted to the #57 feasibility spike.
fn assert_soundness_for_rig<R>(rig_samples: &[R]) -> String
where
    R: Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static,
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
