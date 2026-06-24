//! Weighting + coweighting primitive tests (Leinster 2013 §1.1 Def 1.1.1 + Lemma 1.1.2 + Lemma 1.1.4).
//!
//! v0.2.0 acceptance for the foundational (co)weighting primitives that
//! v0.1.x bypassed in favour of the more restrictive matrix-inversion path.

use catgraph_applied::lawvere_metric::LawvereMetricSpace;
use catgraph_applied::rig::{F64Rig, Tropical};
use catgraph_magnitude::magnitude::{coweighting, magnitude, mobius_function, weighting};

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

    assert!(
        (sum_w - sum_v).abs() < 1e-9,
        "Σw = {sum_w}, Σv = {sum_v}, residual = {:.2e}",
        (sum_w - sum_v).abs()
    );
    assert!(
        (sum_w - mag.0).abs() < 1e-9,
        "Σw = {sum_w}, magnitude = {}, residual = {:.2e}",
        mag.0,
        (sum_w - mag.0).abs()
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
        assert!(
            (w_j.0 - mu_row_sum).abs() < 1e-9,
            "w[{j}] = {}, μ row-sum[{j}] = {}, residual = {:.2e}",
            w_j.0,
            mu_row_sum,
            (w_j.0 - mu_row_sum).abs()
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
        assert!(
            (w[i].0 - v[i].0).abs() < 1e-9,
            "symmetric ζ: w[{i}] = {} != v[{i}] = {}",
            w[i].0,
            v[i].0
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
    assert!((w[0].0 - 1.0).abs() < 1e-12);
}
