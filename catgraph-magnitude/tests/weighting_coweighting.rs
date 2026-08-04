//! Weighting + coweighting primitive tests (Leinster 2013 §1.1 Def 1.1.1 + Lemma 1.1.2 + Lemma 1.1.4).
//!
//! Acceptance for the foundational (co)weighting primitives that the
//! matrix-inversion path bypasses in favour of its more restrictive route.

use catgraph_applied::lawvere_metric::LawvereMetricSpace;
use catgraph_applied::rig::{F64Rig, Tropical};
use catgraph_magnitude::magnitude::{coweighting, magnitude, mobius_function, weighting};
use catgraph_testutil::assert_approx_rel;

/// Relative agreement demanded of the Leinster (co)weighting identities (#169).
///
/// These are identities between two arithmetic routes to the same real number
/// (a weighting sum versus a magnitude, a weighting entry versus a Möbius row
/// sum), so the meaningful bound is on significant digits rather than absolute
/// units, and it stays meaningful if a fixture's magnitude grows.
///
/// Relative to the pre-#169 absolute `1e-9`, this bound is **not uniform** —
/// which is the point, and is worth stating exactly rather than glossing:
///
/// - Sum identities on `build_uniform_space(4, 2.0)` compare against the
///   magnitude `2.8449`, so the bound is `2.84e-9` — **2.84× looser** than
///   before. Measured residuals there are `0.0` and `4.44e-16`, i.e. six-plus
///   orders of headroom, so the loosening is nominal.
/// - Entry-wise identities compare sub-unit values (`w_j = 0.7112`), so the
///   bound is `7.11e-10` — slightly **tighter** than before.
const WEIGHTING_REL_TOL: f64 = 1e-9;

/// Absolute floor for the same identities — a weighting entry may legitimately
/// sit at zero, where a relative bound would demand bit-exact equality.
const WEIGHTING_ABS_TOL: f64 = 1e-12;

/// Bound for values that are **exactly** `1.0` by construction (the one-point
/// space's weighting).
///
/// Pure-absolute (`rel = 0.0`): the measured residual is bit-exactly `0.0`, so
/// the `1e-12` the pre-#169 assertion used is earned. A relative bound against
/// a value of `1.0` would have silently widened it to `1e-9`.
const EXACT_ABS_TOL: f64 = 1e-12;

/// Build a 4-state symmetric Lawvere metric space with all-equal pairwise
/// distance `d`. ζ entries are `e^(-d)`. Diagonal is set to 0 by convention
/// (identity axiom `d(x, x) = 0` ⇒ `ζ(x, x) = 1`).
fn build_uniform_space(n: usize, d: f64) -> LawvereMetricSpace<usize> {
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

/// Lemma 1.1.2 — `Σⱼ w(j) = Σᵢ v(i) = magnitude`.
#[test]
fn weighting_sum_equals_coweighting_sum_equals_magnitude() {
    let space = build_uniform_space(4, 2.0);
    let w = weighting::<F64Rig>(&space).expect("weighting succeeds");
    let v = coweighting::<F64Rig>(&space).expect("coweighting succeeds");
    let mag = magnitude::<F64Rig>(&space, 1.0).expect("magnitude succeeds");

    let sum_w: f64 = w.iter().map(|q| q.0).sum();
    let sum_v: f64 = v.iter().map(|q| q.0).sum();

    assert_approx_rel!(
        sum_w,
        sum_v,
        WEIGHTING_REL_TOL,
        WEIGHTING_ABS_TOL,
        "Lemma 1.1.2: Σw vs Σv"
    );
    assert_approx_rel!(
        sum_w,
        mag.0,
        WEIGHTING_REL_TOL,
        WEIGHTING_ABS_TOL,
        "Lemma 1.1.2: Σw vs magnitude"
    );
}

/// Lemma 1.1.4 — when ζ is invertible, the unique weighting equals the row-sum of `μ = ζ⁻¹`.
#[test]
fn weighting_equals_mobius_row_sum_when_invertible() {
    let space = build_uniform_space(4, 2.0);
    let w = weighting::<F64Rig>(&space).expect("weighting succeeds");
    let mu = mobius_function::<F64Rig>(&space).expect("mobius_function succeeds");

    let n = w.len();
    for (j, w_j) in w.iter().enumerate() {
        // w(j) should equal Σᵢ μ(j, i).
        let mu_row_sum: f64 = (0..n).map(|i| mu.entries()[j][i].0).sum();
        assert_approx_rel!(
            w_j.0,
            mu_row_sum,
            WEIGHTING_REL_TOL,
            WEIGHTING_ABS_TOL,
            "Lemma 1.1.4: w[{j}] vs μ row-sum[{j}]"
        );
    }
}

/// Symmetric ζ ⇒ weighting and coweighting agree numerically.
#[test]
fn symmetric_space_weighting_equals_coweighting() {
    let space = build_uniform_space(4, 2.0);
    let w = weighting::<F64Rig>(&space).expect("weighting succeeds");
    let v = coweighting::<F64Rig>(&space).expect("coweighting succeeds");

    for i in 0..w.len() {
        assert_approx_rel!(
            w[i].0,
            v[i].0,
            WEIGHTING_REL_TOL,
            WEIGHTING_ABS_TOL,
            "symmetric ζ: w[{i}] vs v[{i}]"
        );
    }
}

/// Singular ζ (all-zero matrix achieved by `d → +∞` everywhere except
/// diagonal) — actually still has weighting (identity case). Build a
/// truly degenerate case: identical rows.
#[test]
fn singular_zeta_returns_err() {
    // 2-state space with identical rows in ζ: d(0,0) = d(0,1) = 0 ⇒ ζ row 0 = (1, 1).
    // d(1,0) = d(1,1) = 0 ⇒ ζ row 1 = (1, 1). ζ is rank-1, singular.
    let mut space: LawvereMetricSpace<usize> = LawvereMetricSpace::new(vec![0, 1]);
    space.set_distance(0, 0, Tropical(0.0));
    space.set_distance(0, 1, Tropical(0.0));
    space.set_distance(1, 0, Tropical(0.0));
    space.set_distance(1, 1, Tropical(0.0));

    assert!(weighting::<F64Rig>(&space).is_err());
    assert!(coweighting::<F64Rig>(&space).is_err());
}

/// Empty space — weighting and coweighting return empty Vec without error.
#[test]
fn empty_space_returns_empty_vec() {
    let space: LawvereMetricSpace<usize> = LawvereMetricSpace::new(vec![]);
    let w = weighting::<F64Rig>(&space).expect("empty weighting ok");
    let v = coweighting::<F64Rig>(&space).expect("empty coweighting ok");
    assert!(w.is_empty());
    assert!(v.is_empty());
}

/// One-point space — weighting = (1).
#[test]
fn one_point_space_weighting_is_one() {
    let mut space: LawvereMetricSpace<usize> = LawvereMetricSpace::new(vec![0]);
    space.set_distance(0, 0, Tropical(0.0));
    let w = weighting::<F64Rig>(&space).expect("singleton ok");
    assert_eq!(w.len(), 1);
    assert_approx_rel!(w[0].0, 1.0, 0.0, EXACT_ABS_TOL);
}
