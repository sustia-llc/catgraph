//! The magnitude claim, end to end.
//!
//! The four acceptance gates listed in `catgraph-magnitude/README.md`
//! §Acceptance gates, from one file: BV 2025 Prop 3.10's closed form
//! `Mag(tM) = (t − 1)·Σ H_t(p_x) + #(T(⊥))` against `LmCategory::magnitude`,
//! and `Mag(2M)` against the hand-computed `2.48`; BV 2025 Rem 3.11 / Eq (12)
//! Shannon recovery `d/dt Mag(tM)|_{t=1} = Σ H(p_x)` by central finite
//! difference; Leinster 2013 Prop 2.1.3 chain-sum Möbius against the
//! Gaussian-elimination Möbius, with `Err` on non-scattered input; BV 2025
//! Prop 3.14's magnitude-homology Euler-characteristic identity within an
//! analytical truncation bound. Alongside them, `verify_mobius_recursion`
//! returns `Err` naming the direction and the index when it is handed a μ with
//! one entry perturbed.
//!
//! # Input space
//!
//! Prop 3.10 and Rem 3.11 run on the hand fixture `A = {a}`, `N = 1` (four
//! states, `#T(⊥) = 2`) at `t ∈ {0.5, 1.5, 2.0, 5.0}` and at `t = 1 ± 1e-4`.
//! Prop 2.1.3 runs on uniform-distance spaces `scattered_uniform_space(n,
//! slack)` over `n ∈ 2..=5` and `slack ∈ [0.5, 3.0]` under proptest, plus the
//! hand fixtures `(4, 0.1)`, `(2, 1.0)` and `(2, 0.5)`, the empty and singleton
//! spaces, and two non-scattered four-point spaces (`d = 0.1`, `d = 1.05`).
//! Prop 3.14 runs on five fixtures with `n ∈ {2, 3, 4, 5}`, `t ∈ {2.0, 2.5,
//! 3.0, 4.0}` and `max_degree ∈ {2, 3, 4}`. The Möbius-recursion arm runs on
//! the three-object linear poset over `Z`, once with the exact μ and once with
//! `μ[0][1]` replaced by `0`.
//!
//! # References
//!
//! Prop 3.10's right-hand side is `tsallis_entropy` summed over the fixture's
//! own non-terminating transition rows, so the reference not derived from the
//! implementation is the hand value `Mag(2M) = 2.48` asserted beside it. Rem
//! 3.11 compares a finite difference of `magnitude` against the Shannon sum of
//! the same rows. Prop 2.1.3 compares the chain-sum route against the
//! Gaussian-elimination route. Prop 3.14 compares the homology route against
//! the Möbius-inverse route within a bound computed in this file from `n`,
//! `d_min` and `max_degree`. The Möbius-recursion arm's references are the
//! Phil Hall μ of the three-chain and the identity `μ · ζ = ζ · μ = I`
//! (Leinster 2008 Def 1.1).
//!
//! # Reach
//!
//! `fixture_3_5point_path_t_2_5` is `ignore`d under `debug_assertions`, so the
//! debug lane does not exercise it; the CI release-test job runs this file with
//! `--release` (#11). The `verify_mobius_recursion` arm perturbs one entry of
//! one fixture, and its assertions touch the right-inverse (`μ · ζ`) branch,
//! not the left-inverse (`ζ · μ`) branch. `INCREMENTAL_REL_TOL` and
//! `coalition_value_delta` are crate-root re-exports that no file under
//! `catgraph-magnitude/tests` or `catgraph-magnitude/examples` names (`rg -n
//! 'INCREMENTAL_REL_TOL|coalition_value_delta' catgraph-magnitude/tests
//! catgraph-magnitude/examples -l` → no matches); they are a `const` and a
//! `fn`, so the lists below, which range over `pub struct|enum|trait|type`
//! declarations, do not name them.
//!
//! # covers:
//!
//! `LmCategory` `NodeId` `PosetCategory`
//!
//! # not-covered:
//!
//! `Chain` `ChainIndex` `ChannelCouplings` `Coalition` `CoalitionEvaluator`
//! `ConditionReport` `Copresheaf` `EvalPath` `EvalScratch` `FactorizationPath`
//! `IntegerLikeRig` `JoinReport` `MixedClass` `ModulatedCouplings`
//! `ProbCospan` `Ring` `RoleFibrationProof` `RoleGrid` `RoleId`
//! `RoleModulation` `RoleShares` `TropCospan` `WeightedCospan`
//! `ZeroDiversityProof` `ZetaFactorization`

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation
)]

use catgraph_applied::lawvere_metric::LawvereMetricSpace;
use catgraph_applied::rig::{F64Rig, Tropical};
use catgraph_magnitude::Z;
use catgraph_magnitude::chain_complex::euler_char_identity_at;
use catgraph_magnitude::lm_category::LmCategory;
use catgraph_magnitude::magnitude::{is_scattered, mobius_function, tsallis_entropy};
use catgraph_magnitude::mobius_chains::{
    mobius_function_via_chains, mobius_function_via_chains_exact, verify_mobius_recursion,
};
use catgraph_magnitude::poset_category::PosetCategory;
use catgraph_magnitude::weighted_cospan::NodeId;
use catgraph_testutil::{approx_rel, assert_approx_rel};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// BV 2025 Prop 3.10 + Rem 3.11 — the hand LM fixture
// ---------------------------------------------------------------------------

/// `Mag(2M)` on [`build_bv_lm`], computed by hand from BV 2025 Eq (10):
/// `4 − (0.36 + 0.16 + 0.36) − 1`.
const HAND_MAG_AT_T_2: f64 = 2.48;

/// Agreement demanded of `magnitude` against the two Prop 3.10 references.
const PROP_3_10_ABS_TOL: f64 = 1e-9;

/// Agreement demanded of the Rem 3.11 central finite difference against the
/// Shannon sum, at step `h = 1e-4`.
const REM_3_11_ABS_TOL: f64 = 1e-6;

/// The Rem 3.11 finite-difference step; above `TSALLIS_SHANNON_EPS = 1e-6`, so
/// both `Mag(1 ± h)` take the Tsallis branch.
const FD_STEP: f64 = 1e-4;

/// The four-state hand fixture: `A = {a}`, cutoff `N = 1`, states
/// `⊥, ⊥a, ⊥†, ⊥a†`, terminating `{⊥†, ⊥a†}`, rows `p_⊥ = (a: 0.6, †: 0.4)` and
/// `p_⊥a = (†: 1.0)`.
fn build_bv_lm() -> LmCategory {
    let mut m = LmCategory::new(vec!["s0".into(), "s0a".into(), "s0t".into(), "s0at".into()]);
    m.mark_terminating("s0t");
    m.mark_terminating("s0at");
    m.add_transition("s0", "s0a", 0.6).unwrap();
    m.add_transition("s0", "s0t", 0.4).unwrap();
    m.add_transition("s0a", "s0at", 1.0).unwrap();
    m
}

/// `Σ_{x ∉ T(⊥)} H_t(p_x)` over the recorded transition rows.
fn tsallis_sum(m: &LmCategory, t: f64) -> f64 {
    m.objects()
        .iter()
        .filter(|x| !m.terminating().contains(*x))
        .map(|x| {
            let probs: Vec<f64> = m
                .transitions()
                .get(x)
                .map(|r| r.values().copied().collect())
                .unwrap_or_default();
            tsallis_entropy(&probs, t)
        })
        .sum()
}

/// The `t = 1` (Shannon) evaluation of [`tsallis_sum`].
fn shannon_sum(m: &LmCategory) -> f64 {
    tsallis_sum(m, 1.0)
}

#[test]
fn bv_2025_prop_3_10_closed_form() {
    let m = build_bv_lm();
    let mut max_residual: f64 = 0.0;
    for &t in &[0.5_f64, 1.5, 2.0, 5.0] {
        let lhs = m.magnitude(t).expect("zeta_t should be invertible");
        let rhs = (t - 1.0) * tsallis_sum(&m, t) + (m.terminating().len() as f64);
        let residual = (lhs - rhs).abs();
        max_residual = max_residual.max(residual);
        assert!(
            residual < PROP_3_10_ABS_TOL,
            "Prop 3.10 failed at t={t}: lhs={lhs}, rhs={rhs}, residual={residual}"
        );
    }

    // #309: the absolute anchor. Both sides above recompute from the same
    // fixture, so a misbuilt fixture leaves the differential comparison green;
    // `2.48` is a hand value that does not.
    let mag_2 = m
        .magnitude(2.0)
        .expect("zeta_t should be invertible at t=2");
    let anchor_residual = (mag_2 - HAND_MAG_AT_T_2).abs();
    assert!(
        anchor_residual < PROP_3_10_ABS_TOL,
        "Prop 3.10 absolute anchor failed: Mag(2M)={mag_2}, \
         hand value={HAND_MAG_AT_T_2}, residual={anchor_residual:e}, \
         tolerance={PROP_3_10_ABS_TOL:e}"
    );

    eprintln!("BV 2025 Prop 3.10: max |lhs − rhs| over 4 t-values = {max_residual:e}");
    eprintln!("BV 2025 Prop 3.10: |Mag(2M) − 2.48| = {anchor_residual:e}");
}

#[test]
fn bv_2025_rem_3_11_shannon_recovery() {
    let m = build_bv_lm();
    let mag_plus = m
        .magnitude(1.0 + FD_STEP)
        .expect("zeta_t invertible at 1+h");
    let mag_minus = m
        .magnitude(1.0 - FD_STEP)
        .expect("zeta_t invertible at 1-h");
    let lhs = (mag_plus - mag_minus) / (2.0 * FD_STEP);
    let rhs = shannon_sum(&m);
    let residual = (lhs - rhs).abs();
    assert!(
        residual < REM_3_11_ABS_TOL,
        "Rem 3.11 / Eq (12) failed: lhs={lhs}, rhs={rhs}, residual={residual}"
    );
    eprintln!("BV 2025 Rem 3.11 / Eq (12): |fd − shannon| = {residual:e}");
}

// ---------------------------------------------------------------------------
// Leinster 2013 Prop 2.1.3 — chain-sum vs matrix-inversion Möbius
// ---------------------------------------------------------------------------

/// Relative agreement demanded of the two Möbius routes on well-separated
/// spaces (`slack ≥ 0.5`).
const MOBIUS_REL_TOL: f64 = 1e-9;

/// Relative agreement demanded at the scatteredness boundary (`slack = 0.1`),
/// where `r = (n − 1)·e^(−d) → 1⁻` and the truncated chain sum carries a
/// heavier tail.
const MOBIUS_BOUNDARY_REL_TOL: f64 = 1e-8;

/// Absolute bound for entries that are exactly `1.0` by construction (the
/// singleton space's `μ[0][0]`).
const EXACT_ABS_TOL: f64 = 1e-12;

/// Absolute floor for entries that are structurally zero, which have no scale
/// for a relative bound to multiply.
const MOBIUS_ABS_TOL: f64 = 1e-12;

/// Uniform-distance space on `n` points with off-diagonal `d = ln(n − 1) +
/// slack`, so `slack > 0` is exactly the scatteredness margin.
fn scattered_uniform_space(n: usize, slack: f64) -> LawvereMetricSpace<usize> {
    let mut space: LawvereMetricSpace<usize> = LawvereMetricSpace::new((0..n).collect());
    let d = ((n - 1) as f64).ln() + slack;
    for a in 0..n {
        for b in 0..n {
            if a == b {
                space.set_distance(a, b, Tropical(0.0));
            } else {
                space.set_distance(a, b, Tropical(d));
            }
        }
    }
    space
}

/// Uniform-distance space on `n` points with off-diagonal `d`, built directly
/// so the caller can sit below the scatteredness threshold.
fn uniform_space_at_distance(n: usize, d: f64) -> LawvereMetricSpace<usize> {
    let mut space: LawvereMetricSpace<usize> = LawvereMetricSpace::new((0..n).collect());
    for a in 0..n {
        for b in 0..n {
            if a == b {
                space.set_distance(a, b, Tropical(0.0));
            } else {
                space.set_distance(a, b, Tropical(d));
            }
        }
    }
    space
}

/// Compare the two Möbius routes entry by entry on a scattered space.
fn assert_routes_agree(space: &LawvereMetricSpace<usize>, n: usize, rel_tol: f64, label: &str) {
    let mu_inv = mobius_function::<F64Rig>(space).expect("matrix-inversion succeeded");
    let mu_chains = mobius_function_via_chains::<F64Rig>(space).expect("chain-sum succeeded");

    for i in 0..n {
        for j in 0..n {
            let inv_val = mu_inv.entries()[i][j].0;
            let chains_val = mu_chains.entries()[i][j].0;
            assert_approx_rel!(
                inv_val,
                chains_val,
                rel_tol,
                MOBIUS_ABS_TOL,
                "{label} μ[{i}][{j}]: inversion vs chains"
            );
        }
    }
}

#[test]
fn chain_sum_equals_matrix_inversion_on_4_state_scattered() {
    let space = scattered_uniform_space(4, 0.1);
    assert!(is_scattered(&space));
    assert_routes_agree(
        &space,
        4,
        MOBIUS_BOUNDARY_REL_TOL,
        "boundary (n=4, slack=0.1)",
    );
}

#[test]
fn chain_sum_equals_matrix_inversion_at_regression_seed_n2_slack_0_5() {
    // The seed proptest recorded for this claim (`n = 2, slack = 0.5`), kept as
    // a deterministic fixture.
    let space = scattered_uniform_space(2, 0.5);
    assert!(is_scattered(&space));
    assert_routes_agree(&space, 2, MOBIUS_REL_TOL, "seed (n=2, slack=0.5)");
}

#[test]
fn non_scattered_returns_err_on_chain_sum() {
    // d = 0.1 < log(3) ≈ 1.099 ⇒ not scattered.
    let space = uniform_space_at_distance(4, 0.1);
    assert!(!is_scattered(&space));

    let result = mobius_function_via_chains::<F64Rig>(&space);
    assert!(
        result.is_err(),
        "chain-sum on non-scattered space should Err"
    );
}

#[test]
fn boundary_near_non_scattered_returns_err_on_chain_sum() {
    // d = 1.05 < log(3) ≈ 1.0986 ⇒ not scattered, but only barely: the strict
    // `>` in Def 2.1.2 must reject it.
    let space = uniform_space_at_distance(4, 1.05);
    assert!(
        !is_scattered(&space),
        "d = 1.05 < log(3) ≈ 1.0986 ⇒ must classify non-scattered"
    );
    let result = mobius_function_via_chains::<F64Rig>(&space);
    assert!(
        result.is_err(),
        "chain-sum on boundary-near non-scattered space should Err"
    );
}

#[test]
fn chain_sum_empty_space() {
    let space: LawvereMetricSpace<usize> = LawvereMetricSpace::new(vec![]);
    let mu = mobius_function_via_chains::<F64Rig>(&space).expect("empty space ok");
    assert_eq!(mu.rows(), 0);
    assert_eq!(mu.cols(), 0);
}

#[test]
fn chain_sum_one_point_space() {
    let mut space: LawvereMetricSpace<usize> = LawvereMetricSpace::new(vec![0]);
    space.set_distance(0, 0, Tropical(0.0));
    let mu = mobius_function_via_chains::<F64Rig>(&space).expect("singleton ok");
    assert_eq!(mu.rows(), 1);
    assert_approx_rel!(mu.entries()[0][0].0, 1.0, 0.0, EXACT_ABS_TOL);
}

#[test]
fn chain_sum_two_point_space_matches_inversion() {
    // n = 2 ⇒ the scatteredness threshold is log(1) = 0; any d > 0 is scattered.
    let space = scattered_uniform_space(2, 1.0);
    assert!(is_scattered(&space));
    assert_routes_agree(&space, 2, MOBIUS_REL_TOL, "two-point (n=2, slack=1.0)");
}

proptest! {
    /// Equivalence on uniform-distance scattered spaces of size 2-5, `slack`
    /// bounded away from 0 so `r = (n − 1)·e^(−d)` stays below 1 for the
    /// truncated DFS.
    #[test]
    fn chain_sum_equals_matrix_inversion_on_random_scattered(
        n in 2usize..=5,
        slack in 0.5f64..3.0,
    ) {
        let space = scattered_uniform_space(n, slack);
        prop_assume!(is_scattered(&space));

        let mu_inv = mobius_function::<F64Rig>(&space)
            .map_err(|e| TestCaseError::fail(format!("inv: {e:?}")))?;
        let mu_chains = mobius_function_via_chains::<F64Rig>(&space)
            .map_err(|e| TestCaseError::fail(format!("chains: {e:?}")))?;

        for i in 0..n {
            for j in 0..n {
                let inv_val = mu_inv.entries()[i][j].0;
                let chains_val = mu_chains.entries()[i][j].0;
                prop_assert!(
                    approx_rel(inv_val, chains_val, MOBIUS_REL_TOL, MOBIUS_ABS_TOL),
                    "n={n} slack={slack} μ[{i}][{j}]: inv={inv_val}, chains={chains_val}, \
                     residual={:.3e}",
                    (inv_val - chains_val).abs()
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// BV 2025 Prop 3.14 — magnitude-homology Euler-characteristic identity
// ---------------------------------------------------------------------------

/// Floor for accumulated rounding in the Möbius matrix-inverse path over
/// `F64Rig` at the shipped fixture sizes (`n ≤ 5`).
const F64_FLOOR: f64 = 1e-9;

/// Upper bound on the omitted-`k > max_degree` chain contribution:
/// `n · r^(max_degree+1) / (1 − r)` with `r = (n − 1) · exp(−d_min_scaled)`.
fn analytical_residual_bound(n: usize, d_min_scaled: f64, max_degree: usize) -> f64 {
    assert!(n >= 2, "trivial space; acceptance test does not apply");
    let n_f = n as f64;
    let r = (n_f - 1.0) * (-d_min_scaled).exp();
    assert!(
        r < 1.0,
        "geometric ratio r = {r} ≥ 1; Prop 3.14 chain-sum diverges. \
         For convergence, require t · d_min_original > ln(n − 1), i.e., \
         t > ln({n} − 1) / d_min_original."
    );
    let exponent = (max_degree + 1) as i32;
    n_f * r.powi(exponent) / (1.0 - r)
}

fn check_agrees_within_bound(
    space: &LawvereMetricSpace<NodeId>,
    t: f64,
    max_degree: usize,
    d_min_original: f64,
) {
    let (via_hom, via_mag) = euler_char_identity_at::<F64Rig>(space, t, max_degree)
        .expect("euler_char_identity_at succeeds on the shipped fixtures");
    let n = space.size();
    let bound = analytical_residual_bound(n, t * d_min_original, max_degree);
    let tol = bound + F64_FLOOR;
    let abs_delta = (via_hom - via_mag).abs();
    assert!(
        abs_delta <= tol,
        "Prop 3.14 bound violated: \
         |via_homology − via_magnitude| = {abs_delta:.3e}, \
         analytical_bound = {bound:.3e}, \
         tolerance (bound + 1e-9) = {tol:.3e}, \
         (n={n}, t={t}, d_min_original={d_min_original}, max_degree={max_degree}, \
          via_homology={via_hom}, via_magnitude={via_mag})"
    );
}

#[test]
fn fixture_1_4state_scattered_t_2() {
    // d(i, j) = 2 for i ≠ j ⇒ d_min_original = 2.0
    let space = LawvereMetricSpace::from_distance_fn(4, |a, b| if a == b { 0.0 } else { 2.0 });
    check_agrees_within_bound(&space, 2.0, 4, 2.0);
}

#[test]
fn fixture_2_3point_line_t_3() {
    // 3-point geodesic line; d_min_original = 1.0
    let space = LawvereMetricSpace::from_distance_fn(3, |a, b| {
        let table = [[0.0, 1.0, 2.0], [1.0, 0.0, 1.0], [2.0, 1.0, 0.0]];
        table[a][b]
    });
    check_agrees_within_bound(&space, 3.0, 4, 1.0);
}

#[cfg_attr(
    debug_assertions,
    ignore = "30s release / 15+ min debug; covered by the CI release-test job (#11)"
)]
#[test]
fn fixture_3_5point_path_t_2_5() {
    // 5-point geodesic path; d_min_original = 1.0
    let space =
        LawvereMetricSpace::from_distance_fn(5, |a, b| ((a as i64) - (b as i64)).abs() as f64);
    check_agrees_within_bound(&space, 2.5, 4, 1.0);
}

#[test]
fn fixture_4_random_4point_metric_t_3() {
    // Symmetric metric satisfying the triangle inequality; d_min_original = 1.0
    // (entry [2][3]).
    let table = [
        [0.0, 1.5, 2.0, 3.0],
        [1.5, 0.0, 2.5, 2.0],
        [2.0, 2.5, 0.0, 1.0],
        [3.0, 2.0, 1.0, 0.0],
    ];
    let space = LawvereMetricSpace::from_distance_fn(4, |a, b| table[a][b]);
    check_agrees_within_bound(&space, 3.0, 3, 1.0);
}

#[test]
fn fixture_5_2point_t_4() {
    // 2-point space; d_min_original = 1.0
    let space = LawvereMetricSpace::from_distance_fn(2, |a, b| if a == b { 0.0 } else { 1.0 });
    check_agrees_within_bound(&space, 4.0, 2, 1.0);
}

// ---------------------------------------------------------------------------
// Leinster 2008 Def 1.1 — the Möbius-recursion verifier on a wrong μ
// ---------------------------------------------------------------------------

#[test]
fn mobius_recursion_rejects_perturbed_mu() {
    // 3-chain 0 ≤ 1 ≤ 2. Phil Hall: μ = [[1, -1, 0], [0, 1, -1], [0, 0, 1]],
    // ζ = [[1, 1, 1], [0, 1, 1], [0, 0, 1]].
    let cat = PosetCategory::<u32>::from_partial_order(vec![0, 1, 2], |a, b| a <= b);
    let mut mu = mobius_function_via_chains_exact::<u32, Z>(&cat)
        .expect("integer-exact Möbius on a circuit-free 3-chain");
    assert_eq!(mu.entries()[0][1], Z::from(-1_i64));
    verify_mobius_recursion(&cat, &mu).expect("μ · ζ = ζ · μ = I on the exact μ");

    // #308: one entry off. (μ' · ζ)[0][1] = 1 · 1 + 0 · 1 = 1, against the
    // Kronecker delta's 0, so the right-inverse pass fails first and at (0, 1).
    mu.entries_mut()[0][1] = Z::from(0_i64);
    let err = verify_mobius_recursion(&cat, &mu)
        .expect_err("perturbed μ[0][1] = 0 must not verify as an inverse of ζ");
    let message = err.to_string();
    assert!(
        message.contains("right inverse"),
        "observed: {message}; expected the message to name the right-inverse direction"
    );
    assert!(
        message.contains("(0, 1)"),
        "observed: {message}; expected the message to name the index (0, 1)"
    );
}
