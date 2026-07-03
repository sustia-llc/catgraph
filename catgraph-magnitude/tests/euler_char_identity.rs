//! BV 2025 Prop 3.14 acceptance gate: numerical and structural paths must
//! agree on `Mag(tM)` modulo the geometric truncation residual at finite
//! `max_degree`.
//!
//! The numerical path (`magnitude::magnitude`) is exact via Möbius matrix
//! inverse. The structural path truncates the homology sum at `max_degree`,
//! so the disagreement is bounded analytically by the omitted-`k` chain
//! contribution.
//!
//! Acceptance: `|via_homology − via_magnitude| ≤ analytical_bound + 1e-9`,
//! where `analytical_bound = n · r^(max_degree+1) / (1 − r)` and
//! `r = (n − 1) · exp(−t · d_min_original)`. This is the tight upper bound
//! on the omitted chain contribution: chain count at degree `k` is bounded
//! by `n · (n − 1)^k`, each chain has length `≥ k · d_min_scaled`, summing
//! over ℓ gives `n · ((n − 1) · e^(−d_min_scaled))^k`, and the geometric
//! tail from `k = max_degree + 1` collapses to the closed form above.
//!
//! The `1e-9` floor accounts for accumulated
//! rounding in `magnitude::magnitude`'s Möbius matrix-inverse Gaussian
//! elimination at the shipped fixture sizes (`n ≤ 5`). Tighten if a future
//! fixture pushes `n` past ~16, where the elimination's worst-case error
//! growth becomes load-bearing.

use catgraph_applied::lawvere_metric::LawvereMetricSpace;
use catgraph_applied::rig::F64Rig;
use catgraph_magnitude::chain_complex::euler_char_identity_at;
use catgraph_magnitude::weighted_cospan::NodeId;

/// Floor for accumulated rounding noise in the Möbius matrix-inverse path
/// (`n × n` Gaussian elimination over `F64Rig`, with `n ≤ 5` for the shipped
/// fixtures). f64 epsilon is `~2.22e-16`; `1e-9` carries ~7 orders of
/// magnitude of safety margin against the worst-case elimination error
/// growth at this `n` regime.
const F64_FLOOR: f64 = 1e-9;

/// Tight upper bound on the omitted-`k > max_degree` chain contribution to
/// the magnitude-homology Euler characteristic.
fn analytical_residual_bound(n: usize, d_min_scaled: f64, max_degree: usize) -> f64 {
    assert!(n >= 2, "trivial space; acceptance test does not apply");
    #[allow(
        clippy::cast_precision_loss,
        reason = "n ≤ 5 in the shipped fixtures; trivially fits f64 mantissa"
    )]
    let n_f = n as f64;
    let r = (n_f - 1.0) * (-d_min_scaled).exp();
    assert!(
        r < 1.0,
        "geometric ratio r = {r} ≥ 1; Prop 3.14 chain-sum diverges. \
         For convergence, require t · d_min_original > ln(n − 1), i.e., \
         t > ln({n} − 1) / d_min_original."
    );
    #[allow(
        clippy::cast_possible_wrap,
        clippy::cast_possible_truncation,
        reason = "max_degree ≤ 4 in the shipped fixtures; powi argument trivially fits i32"
    )]
    let exponent = (max_degree + 1) as i32;
    n_f * r.powi(exponent) / (1.0 - r)
}

fn check_agrees_within_bound(
    space: &LawvereMetricSpace<NodeId>,
    t: f64,
    max_degree: usize,
    d_min_original: f64,
) {
    let (via_hom, via_mag) = euler_char_identity_at::<F64Rig>(space, t, max_degree).unwrap();
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
    ignore = "30s release / 15+ min debug; see feedback_cg_mag_release_mode_required.md"
)]
#[test]
fn fixture_3_5point_path_t_2_5() {
    // 5-point geodesic path; d_min_original = 1.0
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_wrap,
        reason = "n = 5; arguments trivially fit i64 and f64 mantissa"
    )]
    let space =
        LawvereMetricSpace::from_distance_fn(5, |a, b| ((a as i64) - (b as i64)).abs() as f64);
    check_agrees_within_bound(&space, 2.5, 4, 1.0);
}

#[test]
fn fixture_4_random_4point_metric_t_3() {
    // Symmetric metric satisfying triangle inequality; d_min_original = 1.0 (entry [2][3])
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
