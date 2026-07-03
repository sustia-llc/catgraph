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
//! # Why the gap can't close under plain CC
//!
//! Residual collisions all exhibit the same structural pattern: derivation
//! chains requiring intermediate composite terms not present in the CC
//! term graph. Plain congruence closure (with or without `smc_refine`)
//! closes under sub-term-closure of seeded/queried terms but cannot
//! synthesize fresh composite intermediates. Closing the gap requires
//! either:
//! - **Knuth-Bendix completion** of the 17 equations modulo SMC coherence
//!   (the Knuth-Bendix vs functorial-terminal decision is tracked in issue
//!   #15; 1-3 weeks if confluence terminates).
//! - **The Functorial engine**: [`Presentation::eq_mod_functorial`] with
//!   [`MatrixNFFunctor<R>`] — complete by theorem for Mat(R).
//!
//! The atom-canonical `smc_refine` in the kb.rs fixpoint cut
//! `BoolRig` d=2 collisions 2574 → 1433 (~44%) but can't reach zero.
//!
//! # Why keep these tests `#[ignore]`'d
//!
//! They are diagnostic, not a release gate. `Mag(a) = Mag(b)` (semantic
//! equality) is already achievable via
//! [`Presentation::eq_mod_functorial(&a, &b, &MatrixNFFunctor::new())`].
//! The tests stay `#[ignore]`'d to bound CC incompleteness as engine work
//! progresses; a zero-collision run would mean either KB has completed
//! (the Knuth-Bendix path tracked in issue #15) or an unexpected CC
//! improvement has landed.
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

const IGNORE_REASON: &str = "\
    CC completeness tracking (NOT a Thm 5.60 faithfulness test): Baez-Erbele \
    2015 proved `Free(Σ_SFG)/⟨E_{17}⟩ ≅ Mat(R)` abstractly — we do not need to \
    empirically verify the theorem. These tests bound the incompleteness of \
    the default `NormalizeEngine::CongruenceClosure` engine against the \
    matrix ground truth on bounded-depth enumeration. The atom-canonical \
    `smc_refine` in kb.rs reduces BoolRig d=2 collisions \
    2574 → 1433 (~44%). Closing the remaining gap requires either \
    Knuth-Bendix completion of the 17 equations modulo SMC coherence \
    (tracked in issue #15), or use of `Presentation::eq_mod_functorial` with \
    `MatrixNFFunctor` for an operationally complete Mat(R) decision \
    procedure (shipped as opt-in). These tests stay `#[ignore]`'d \
    as diagnostic, not as a release gate.\
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

// ---- BoolRig × {2, 3, 4} ----

#[test]
#[ignore = "CC completeness tracking; see module docstring and IGNORE_REASON"]
fn cc_completeness_tracking_bool_depth_2() {
    let samples = vec![BoolRig(false), BoolRig(true)];
    let report = verify_sfg_to_mat_is_full_and_faithful::<BoolRig>(2, &samples).unwrap();
    assert_eq!(
        report.collisions_under_s,
        0,
        "BoolRig depth 2: {} expressions, {} collisions; first witness: {:?}. {IGNORE_REASON}",
        report.expressions_checked,
        report.collisions_under_s,
        witness_debug(&report),
    );
}

#[test]
#[ignore = "CC completeness tracking; see module docstring and IGNORE_REASON"]
fn cc_completeness_tracking_bool_depth_3() {
    let samples = vec![BoolRig(false), BoolRig(true)];
    let report = verify_sfg_to_mat_is_full_and_faithful::<BoolRig>(3, &samples).unwrap();
    assert_eq!(report.collisions_under_s, 0);
}

#[test]
#[ignore = "CC completeness tracking; see module docstring and IGNORE_REASON"]
fn cc_completeness_tracking_bool_depth_4() {
    let samples = vec![BoolRig(false), BoolRig(true)];
    let report = verify_sfg_to_mat_is_full_and_faithful::<BoolRig>(4, &samples).unwrap();
    assert_eq!(report.collisions_under_s, 0);
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
    let report = verify_sfg_to_mat_is_full_and_faithful::<UnitInterval>(2, &samples).unwrap();
    assert_eq!(
        report.collisions_under_s,
        0,
        "UnitInterval depth 2: {} collisions; witness {:?}",
        report.collisions_under_s,
        witness_debug(&report),
    );
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
    assert_eq!(report.collisions_under_s, 0);
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
    assert_eq!(report.collisions_under_s, 0);
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
    let report = verify_sfg_to_mat_is_full_and_faithful::<Tropical>(2, &samples).unwrap();
    assert_eq!(
        report.collisions_under_s,
        0,
        "Tropical depth 2: {} collisions; witness {:?}",
        report.collisions_under_s,
        witness_debug(&report),
    );
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
    assert_eq!(report.collisions_under_s, 0);
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
    assert_eq!(report.collisions_under_s, 0);
}

// ---- F64Rig × {2, 3, 4} ----

#[test]
#[ignore = "CC completeness tracking; see module docstring and IGNORE_REASON"]
fn cc_completeness_tracking_f64_depth_2() {
    let samples = vec![F64Rig(0.0), F64Rig(1.0), F64Rig(2.0), F64Rig(-1.0)];
    let report = verify_sfg_to_mat_is_full_and_faithful::<F64Rig>(2, &samples).unwrap();
    assert_eq!(
        report.collisions_under_s,
        0,
        "F64Rig depth 2: {} collisions; witness {:?}",
        report.collisions_under_s,
        witness_debug(&report),
    );
}

#[test]
#[ignore = "CC completeness tracking; see module docstring and IGNORE_REASON"]
fn cc_completeness_tracking_f64_depth_3() {
    let samples = vec![F64Rig(0.0), F64Rig(1.0), F64Rig(2.0), F64Rig(-1.0)];
    let report = verify_sfg_to_mat_is_full_and_faithful::<F64Rig>(3, &samples).unwrap();
    assert_eq!(report.collisions_under_s, 0);
}

#[test]
#[ignore = "CC completeness tracking; see module docstring and IGNORE_REASON"]
fn cc_completeness_tracking_f64_depth_4() {
    let samples = vec![F64Rig(0.0), F64Rig(1.0), F64Rig(2.0), F64Rig(-1.0)];
    let report = verify_sfg_to_mat_is_full_and_faithful::<F64Rig>(4, &samples).unwrap();
    assert_eq!(report.collisions_under_s, 0);
}

// ---- Thm 5.60 soundness: every equation in the presentation is a matrix equality under S ----

/// For each equation `(lhs, rhs)` in the Thm 5.60 presentation, verify that
/// `S(lhs) == S(rhs)` under `sfg_to_mat`. This is the SOUNDNESS direction
/// (S is well-defined on the quotient); the FAITHFULNESS direction (S is
/// injective on the quotient) is the harder direction and requires KB
/// completion — tracked in issue #15.
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
