//! BV 2025 Prop 3.14 acceptance demo — 5-fixture path-C analytical-bound suite.
//!
//! Runnable demo mirroring `tests/euler_char_identity.rs`. For each of the five
//! shipped fixtures (`4state_scattered`, `3point_line`, `5point_path`,
//! `random_4point`, `2point`), constructs the underlying `LawvereMetricSpace`,
//! evaluates
//! [`euler_char_identity_at`] over `F64Rig`, and prints
//! `(via_homology, via_magnitude, |Δ|, analytical_bound, margin)` in the same
//! format as the `BV25-AUDIT.md` deltas section.
//!
//! Exits with code 1 if any fixture's margin is negative (i.e., `|Δ| > bound`),
//! making the demo viable as a CI regression gate alongside the test suite.
//!
//! ## Paper anchor
//!
//! Bradley & Vigneaux, *Magnitude of Language Models* (arXiv:2501.06662v2,
//! 2025), Prop 3.14 — magnitude-homology Euler-characteristic identity. The
//! tight upper bound on the omitted-`k > max_degree` chain contribution is
//!
//! ```text
//! analytical_bound = n · r^(max_degree+1) / (1 − r)
//! r                = (n − 1) · exp(−d_min_scaled)
//! d_min_scaled     = t · d_min_original
//! ```
//!
//! See `tests/euler_char_identity.rs::analytical_residual_bound` (lines 39-60)
//! for the derivation (chain count at degree `k` is `≤ n · (n−1)^k`; each chain
//! has length `≥ k · d_min_scaled`; summing over `ℓ` and tailing from
//! `k = max_degree + 1` collapses to the closed form above).

use catgraph_magnitude::chain_complex::euler_char_identity_at;
use catgraph_magnitude::{F64Rig, LawvereMetricSpace};

/// Floor for accumulated rounding noise in the Möbius matrix-inverse path
/// (`n × n` Gaussian elimination over `F64Rig`, with `n ≤ 5` for the shipped
/// fixtures). Mirrors `tests/euler_char_identity.rs::F64_FLOOR`.
const F64_FLOOR: f64 = 1e-9;

/// Tight upper bound on the omitted-`k > max_degree` chain contribution.
/// Mirrors `tests/euler_char_identity.rs::analytical_residual_bound` exactly.
fn analytical_residual_bound(n: usize, d_min_scaled: f64, max_degree: usize) -> f64 {
    assert!(n >= 2, "trivial space; acceptance demo does not apply");
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

/// One row of the demo: name + space-builder + parameters.
struct Fixture {
    name: &'static str,
    space: LawvereMetricSpace<usize>,
    t: f64,
    max_degree: usize,
    d_min_original: f64,
}

fn fixtures() -> Vec<Fixture> {
    vec![
        // fixture_1: 4 states, off-diagonal d=2.
        Fixture {
            name: "4state_scattered",
            space: LawvereMetricSpace::from_distance_fn(4, |a, b| if a == b { 0.0 } else { 2.0 }),
            t: 2.0,
            max_degree: 4,
            d_min_original: 2.0,
        },
        // fixture_2: 3-point geodesic line.
        Fixture {
            name: "3point_line",
            space: LawvereMetricSpace::from_distance_fn(3, |a, b| {
                let table = [[0.0, 1.0, 2.0], [1.0, 0.0, 1.0], [2.0, 1.0, 0.0]];
                table[a][b]
            }),
            t: 3.0,
            max_degree: 4,
            d_min_original: 1.0,
        },
        // fixture_3: 5-point geodesic path.
        Fixture {
            name: "5point_path",
            space: LawvereMetricSpace::from_distance_fn(5, |a, b| {
                #[allow(
                    clippy::cast_precision_loss,
                    reason = "n = 5; abs_diff result trivially fits f64 mantissa"
                )]
                let d = a.abs_diff(b) as f64;
                d
            }),
            t: 2.5,
            max_degree: 4,
            d_min_original: 1.0,
        },
        // fixture_4: random 4-point metric (symmetric, triangle-inequality-satisfying).
        Fixture {
            name: "random_4point",
            space: LawvereMetricSpace::from_distance_fn(4, |a, b| {
                let table = [
                    [0.0, 1.5, 2.0, 3.0],
                    [1.5, 0.0, 2.5, 2.0],
                    [2.0, 2.5, 0.0, 1.0],
                    [3.0, 2.0, 1.0, 0.0],
                ];
                table[a][b]
            }),
            t: 3.0,
            max_degree: 3,
            d_min_original: 1.0,
        },
        // fixture_5: 2-point space.
        Fixture {
            name: "2point",
            space: LawvereMetricSpace::from_distance_fn(2, |a, b| if a == b { 0.0 } else { 1.0 }),
            t: 4.0,
            max_degree: 2,
            d_min_original: 1.0,
        },
    ]
}

/// Runs the 5-fixture path-C acceptance demo.
///
/// **Release mode is mandatory.** `fixture_3_5point_path_t_2_5` is
/// debug-mode-slow (Möbius matrix-inverse over `F64Rig` on the n=5 chain
/// index). Invoke as:
///
/// ```sh
/// cargo run --example prop_3_14_acceptance -p catgraph-magnitude --release
/// ```
///
/// Exits 0 if every fixture's margin is non-negative (i.e., `|Δ| ≤ bound`),
/// 1 otherwise — the regression-gate signal for CI consumers.
fn main() {
    println!("BV 2025 Prop 3.14 acceptance — 5-fixture path-C demo\n");
    println!(
        "{:>20} {:>12} {:>12} {:>12} {:>12} {:>10}",
        "fixture", "via_hom", "via_mag", "|Δ|", "bound", "margin"
    );

    let mut any_negative = false;
    for fx in fixtures() {
        let (via_hom, via_mag) =
            euler_char_identity_at::<F64Rig>(&fx.space, fx.t, fx.max_degree).unwrap();
        let delta = (via_hom - via_mag).abs();
        let bound =
            analytical_residual_bound(fx.space.size(), fx.t * fx.d_min_original, fx.max_degree);
        let tol = bound + F64_FLOOR;
        let margin = tol - delta;
        if margin < 0.0 {
            any_negative = true;
        }
        println!(
            "{:>20} {:>12.6} {:>12.6} {:>12.2e} {:>12.2e} {:>10.2e}",
            fx.name, via_hom, via_mag, delta, bound, margin
        );
    }

    if any_negative {
        eprintln!(
            "\nREGRESSION: fixture(s) violated Prop 3.14 acceptance bound \
             (margin < 0 ⇒ |Δ| > analytical_bound + F64_FLOOR)."
        );
        std::process::exit(1);
    }
}
