#![cfg(feature = "f64-fast")]
//! `f64` fast-path acceptance: the ζ factorization must agree with the
//! untouched rig-generic Gauss–Jordan path on every route (#165).
//!
//! Three independent Möbius witnesses are cross-checked where all three apply
//! (scattered spaces): Gaussian elimination
//! ([`mobius_function`]), the von-Neumann chain sum
//! ([`mobius_function_via_chains`]), and the symmetric factorization
//! ([`ZetaFactorization::mobius_function`]). Comparison is by **tolerance**,
//! never bitwise — the factorization reassociates the arithmetic and moves the
//! last ULPs. The `1e-9` absolute per-entry tolerance mirrors the existing
//! `tests/canonical.rs` cross-check.

use catgraph_applied::lawvere_metric::LawvereMetricSpace;
use catgraph_applied::rig::{F64Rig, Tropical};
use catgraph_magnitude::lm_category::LmCategory;
use catgraph_magnitude::magnitude::{is_scattered, magnitude, mobius_function};
use catgraph_magnitude::magnitude_f64::{FactorizationPath, ZetaFactorization, magnitude_f64};
use catgraph_magnitude::mobius_chains::mobius_function_via_chains;
use catgraph_testutil::Lcg;
use nalgebra::DMatrix;
use nalgebra::linalg::LBLT;

/// Per-entry absolute tolerance, matching `tests/canonical.rs`.
const TOL: f64 = 1e-9;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Uniform-distance space: `ζ = (1 − c)I + cJ` with `c = e^(−d) ∈ (0, 1)`,
/// whose eigenvalues `1 − c` (multiplicity `n − 1`) and `1 + (n − 1)c` are all
/// positive — so ζ is positive-definite and the **Cholesky** route is taken.
/// `slack` is added on top of the Leinster Def 2.1.2 scatteredness threshold
/// `log(n − 1)`, so the chain-sum witness applies too.
fn scattered_uniform_space(n: usize, slack: f64) -> LawvereMetricSpace<usize> {
    #[allow(clippy::cast_precision_loss)]
    let d = ((n - 1) as f64).ln() + slack;
    LawvereMetricSpace::from_distance_fn(n, |a, b| if a == b { 0.0 } else { d })
}

/// Symmetric but **indefinite** ζ on `n` points — the Bunch–Kaufman (`LBLT`)
/// route.
///
/// A "star": object `0` is *close* to every other object (`ζ = 0.9`) while the
/// `m = n − 1` leaves are mutually *far* (`ζ = 0.1`). In block form
/// `ζ = [[1, 0.9·1ᵀ], [0.9·1, B]]` with `B = 0.9·I_m + 0.1·J_m`, the Schur
/// complement of the `(0, 0)` entry is `B − 0.81·J_m = 0.9·I_m − 0.71·J_m`,
/// whose eigenvalues are `0.9 − 0.71·m` (once, from the all-ones eigenvector)
/// and `0.9` (`m − 1` times). For `m ≥ 2` the first is **negative**, so ζ has
/// exactly one negative eigenvalue: symmetric, non-singular, and **indefinite**
/// — Cholesky must reject it and Bunch–Kaufman must handle it. At `n = 3` this
/// is the `det = −0.468 < 0` odd-negative-eigenvalue check on
/// `[[1, .9, .9], [.9, 1, .1], [.9, .1, 1]]`.
///
/// Lawvere-legal: zero diagonal, strictly positive off-diagonal distances. The
/// triangle inequality is violated (`d(1,2) > d(1,0) + d(0,2)`), which is fine
/// — `LawvereMetricSpace` does not enforce it on `set_distance`.
fn symmetric_indefinite_star(n: usize) -> LawvereMetricSpace<usize> {
    let close = -(0.9_f64).ln();
    let far = -(0.1_f64).ln();
    LawvereMetricSpace::from_distance_fn(n, |a, b| match (a, b) {
        (x, y) if x == y => 0.0,
        (0, _) | (_, 0) => close,
        _ => far,
    })
}

/// Rebuild ζ for a space through the **public** surface only — `exp(−d)` over
/// the `0..n` object order — as an `nalgebra::DMatrix<f64>`.
///
/// Lets the LBLT structural / backward-error checks drive nalgebra directly
/// without the crate exposing any internals.
fn zeta_matrix(space: &LawvereMetricSpace<usize>) -> DMatrix<f64> {
    let n = space.size();
    DMatrix::from_fn(n, n, |i, j| (-space.distance(&i, &j).0).exp())
}

/// Asymmetric ζ — Lawvere `[0, ∞]`-enrichment drops the symmetry axiom, so
/// this is legal input and must route to the generic **Gauss–Jordan** path.
fn asymmetric_space() -> LawvereMetricSpace<usize> {
    LawvereMetricSpace::from_distance_fn(4, |a, b| {
        if a == b {
            0.0
        } else if a < b {
            0.7
        } else {
            2.3
        }
    })
}

/// Exactly singular ζ: two objects at distance `0` in both directions give
/// duplicate rows `[1, 1]`.
fn singular_space() -> LawvereMetricSpace<usize> {
    LawvereMetricSpace::from_distance_fn(2, |_, _| 0.0)
}

/// Seeded symmetric random space via the shared deterministic LCG. Distances
/// are drawn well above `log(n − 1)` for the sizes used here, keeping ζ
/// diagonally dominant (hence positive-definite).
fn random_symmetric_space(n: usize, seed: u64) -> LawvereMetricSpace<usize> {
    // `| 1` seed prep stays at the call site (#33).
    let mut rng = Lcg::new(seed | 1);
    let mut space: LawvereMetricSpace<usize> = LawvereMetricSpace::new((0..n).collect());
    for a in 0..n {
        space.set_distance(a, a, Tropical(0.0));
    }
    for a in 0..n {
        for b in (a + 1)..n {
            let d = 2.5 + 2.0 * rng.next_f64();
            space.set_distance(a, b, Tropical(d));
            space.set_distance(b, a, Tropical(d));
        }
    }
    space
}

/// The `t`-scaled copy of a space — the test-side mirror of the crate-private
/// `scaled_space` helper that [`magnitude_f64`] applies internally. Needed
/// because the factorization route is a property of the *scaled* ζ.
fn scale(space: &LawvereMetricSpace<usize>, t: f64) -> LawvereMetricSpace<usize> {
    let n = space.size();
    LawvereMetricSpace::from_distance_fn(n, |a, b| t * space.distance(&a, &b).0)
}

// ---------------------------------------------------------------------------
// Route selection
// ---------------------------------------------------------------------------

#[test]
fn factorization_routes_are_selected_as_documented() {
    assert_eq!(
        ZetaFactorization::new(&scattered_uniform_space(5, 0.5)).path(),
        FactorizationPath::Cholesky,
        "uniform-distance ζ = (1−c)I + cJ is positive-definite"
    );
    assert_eq!(
        ZetaFactorization::new(&symmetric_indefinite_star(3)).path(),
        FactorizationPath::Lblt,
        "symmetric with det < 0 ⇒ indefinite ⇒ Bunch-Kaufman"
    );
    assert_eq!(
        ZetaFactorization::new(&asymmetric_space()).path(),
        FactorizationPath::GaussJordan,
        "asymmetric ζ must not take a lower-triangle-only symmetric route"
    );
    assert_eq!(
        ZetaFactorization::new(&singular_space()).path(),
        FactorizationPath::GaussJordan,
        "exactly singular symmetric ζ: Cholesky rejects, LBLT reports a zero pivot"
    );

    // Sizes / accessors on every route.
    let uniform = scattered_uniform_space(5, 0.5);
    let fact = ZetaFactorization::new(&uniform);
    assert_eq!(fact.size(), 5);
    assert!(!fact.is_empty());
    // ‖ζ‖₁ for the uniform fixture: one 1.0 diagonal + (n−1) off-diagonals.
    let c = (-(4.0_f64.ln() + 0.5)).exp();
    assert!((fact.zeta_one_norm() - (1.0 + 4.0 * c)).abs() < 1e-12);
}

/// Pins the route claim `benches/magnitude_bench.rs` makes about its
/// `magnitude_f64/mag_lm` group.
///
/// A forward-only chain LM can only reach `j > i`, so `d(j, i) = Tropical(+∞)`
/// (hence `ζ[j][i] = 0`) while `ζ[i][j] > 0` — ζ is **asymmetric** and the
/// group therefore measures the Gauss–Jordan fallback, never Cholesky/LBLT.
/// The asymmetry is *structural* (unreachable backward pairs), so it does not
/// depend on the transition probabilities the bench fixture draws.
#[test]
fn forward_chain_lm_routes_to_the_gauss_jordan_fallback() {
    let names: Vec<String> = (0..6).map(|i| format!("s{i}")).collect();
    let mut lm = LmCategory::new(names.clone());
    lm.mark_terminating(&names[5]);
    for i in 0..5 {
        lm.add_transition(&names[i], &names[i + 1], 1.0)
            .expect("forward chain transition is valid");
    }

    let space = lm.enriched_space().expect("enriched space builds");
    assert_eq!(
        ZetaFactorization::new(&space).path(),
        FactorizationPath::GaussJordan,
        "forward-only chain LM has unreachable backward pairs ⇒ asymmetric ζ"
    );
}

// ---------------------------------------------------------------------------
// Three-witness Möbius agreement
// ---------------------------------------------------------------------------

#[test]
fn three_witness_mobius_agreement_on_scattered_fixtures() {
    for (n, slack) in [(2usize, 1.0), (3, 0.5), (4, 0.1), (5, 0.75), (6, 2.0)] {
        let space = scattered_uniform_space(n, slack);
        assert!(is_scattered(&space), "n={n} slack={slack}");

        let gauss = mobius_function::<F64Rig>(&space).expect("Gauss-Jordan inversion");
        let chains = mobius_function_via_chains::<F64Rig>(&space).expect("chain sum");
        let fact = ZetaFactorization::new(&space);
        assert_eq!(fact.path(), FactorizationPath::Cholesky);
        let fast = fact.mobius_function().expect("factorization inversion");

        for i in 0..n {
            for j in 0..n {
                let g = gauss.entries()[i][j].0;
                let c = chains.entries()[i][j].0;
                let f = fast.entries()[i][j].0;
                assert!(
                    (g - c).abs() < TOL && (g - f).abs() < TOL,
                    "n={n} slack={slack} μ[{i}][{j}]: gauss={g:.12} chains={c:.12} fast={f:.12}"
                );
            }
        }
    }
}

#[test]
fn mobius_agrees_with_generic_on_the_lblt_and_gauss_jordan_routes() {
    for (label, space, expected) in [
        (
            "lblt",
            symmetric_indefinite_star(3),
            FactorizationPath::Lblt,
        ),
        (
            "asymmetric",
            asymmetric_space(),
            FactorizationPath::GaussJordan,
        ),
    ] {
        let n = space.size();
        let gauss = mobius_function::<F64Rig>(&space).expect("Gauss-Jordan inversion");
        let fact = ZetaFactorization::new(&space);
        assert_eq!(fact.path(), expected, "{label}");
        let fast = fact.mobius_function().expect("factorization inversion");

        for i in 0..n {
            for j in 0..n {
                let g = gauss.entries()[i][j].0;
                let f = fast.entries()[i][j].0;
                assert!(
                    (g - f).abs() < TOL,
                    "{label} μ[{i}][{j}]: gauss={g:.12} fast={f:.12}"
                );
            }
        }
    }
}

/// The Bunch–Kaufman route, structurally.
///
/// A 3×3 star is not enough coverage on its own: its single 2×2 pivot lands at
/// `k = 1` with `k + 2 == n`, so `LBLT`'s **rank-2 trailing-submatrix update**
/// (nalgebra 0.35.0 `linalg/lblt.rs:199-220`) has an empty trailing block and
/// never executes. The 5×5 star puts a 2×2 pivot at `k = 1` with `k + 2 < n`,
/// so the rank-2 update does run.
///
/// A 2×2 block at leading index `k` is visible from the public API as a
/// non-zero sub-diagonal entry `d()[(k + 1, k)]`. Both sizes also get a
/// **backward-error** guard on the factorization identity `L·D·Lᵀ = ζ` (with
/// `L = l_permuted()`, i.e. already in the original basis).
#[test]
fn lblt_route_exercises_the_rank_two_trailing_update_and_reconstructs_zeta() {
    for (n, needs_trailing_update) in [(3usize, false), (5usize, true)] {
        let space = symmetric_indefinite_star(n);
        assert_eq!(
            ZetaFactorization::new(&space).path(),
            FactorizationPath::Lblt,
            "n={n}: star fixture must be symmetric-indefinite"
        );

        let zeta = zeta_matrix(&space);
        let lblt = LBLT::new(zeta.clone());
        let d = lblt.d();
        let l = lblt.l_permuted();

        let two_by_two: Vec<usize> = (0..n - 1).filter(|&k| d[(k + 1, k)].abs() > 0.0).collect();
        assert!(
            !two_by_two.is_empty(),
            "n={n}: expected at least one 2x2 Bunch-Kaufman pivot, d = {d}"
        );
        assert_eq!(
            two_by_two.iter().any(|&k| k + 2 < n),
            needs_trailing_update,
            "n={n}: 2x2 pivots at {two_by_two:?}; trailing-update expectation mismatch"
        );

        // Backward error of the factorization, normalized by ‖ζ‖₁·n.
        #[allow(clippy::cast_precision_loss)]
        let scale_n = n as f64;
        let residual = (&l * &d * l.transpose() - &zeta).one_norm();
        let backward_error = residual / (zeta.one_norm() * scale_n);
        assert!(
            backward_error < 1e-14,
            "n={n}: ‖L·D·Lᵀ − ζ‖₁ / (‖ζ‖₁·n) = {backward_error:.3e} exceeds 1e-14"
        );
    }
}

// ---------------------------------------------------------------------------
// magnitude parity across sizes, scales and routes
// ---------------------------------------------------------------------------

#[test]
fn magnitude_f64_matches_generic_across_sizes_and_scales() {
    for n in [1usize, 2, 3, 5, 8, 13] {
        let space = random_symmetric_space(n, 0xC0FFEE);
        for t in [0.5_f64, 1.0, 2.0, 5.0] {
            let scaled = scale(&space, t);
            let fact = ZetaFactorization::new(&scaled);
            assert_eq!(
                fact.path(),
                FactorizationPath::Cholesky,
                "n={n} t={t}: seeded symmetric fixture is positive-definite"
            );

            let fast = magnitude_f64(&space, t).expect("fast magnitude");
            let generic: F64Rig = magnitude(&space, t).expect("generic magnitude");
            assert!(
                (fast - generic.0).abs() < TOL,
                "n={n} t={t}: fast={fast:.12} generic={:.12}",
                generic.0
            );
        }
    }
}

#[test]
fn magnitude_f64_matches_generic_on_the_lblt_route() {
    let space = symmetric_indefinite_star(3);
    for t in [1.0_f64, 1.5, 3.0] {
        let scaled = scale(&space, t);
        let path = ZetaFactorization::new(&scaled).path();
        let fast = magnitude_f64(&space, t).expect("fast magnitude");
        let generic: F64Rig = magnitude(&space, t).expect("generic magnitude");
        assert!(
            (fast - generic.0).abs() < TOL,
            "t={t} path={path:?}: fast={fast:.12} generic={:.12}",
            generic.0
        );
    }
    // At t = 1 the fixture is the documented indefinite case.
    assert_eq!(
        ZetaFactorization::new(&scale(&space, 1.0)).path(),
        FactorizationPath::Lblt
    );
}

#[test]
fn magnitude_f64_matches_generic_on_the_asymmetric_fallback_route() {
    let space = asymmetric_space();
    for t in [0.5_f64, 1.0, 2.0] {
        assert_eq!(
            ZetaFactorization::new(&scale(&space, t)).path(),
            FactorizationPath::GaussJordan
        );
        let fast = magnitude_f64(&space, t).expect("fast magnitude");
        let generic: F64Rig = magnitude(&space, t).expect("generic magnitude");
        assert!(
            (fast - generic.0).abs() < TOL,
            "t={t}: fast={fast:.12} generic={:.12}",
            generic.0
        );
    }
}

// ---------------------------------------------------------------------------
// weighting / coweighting parity
// ---------------------------------------------------------------------------

#[test]
fn weighting_and_coweighting_match_generic_on_symmetric_routes() {
    for (label, space) in [
        ("cholesky", scattered_uniform_space(6, 0.4)),
        ("lblt", symmetric_indefinite_star(3)),
    ] {
        let n = space.size();
        let fact = ZetaFactorization::new(&space);
        let w_fast = fact.weighting().expect("fast weighting");
        let v_fast = fact.coweighting().expect("fast coweighting");
        let w_gen =
            catgraph_magnitude::magnitude::weighting::<F64Rig>(&space).expect("generic weighting");
        let v_gen = catgraph_magnitude::magnitude::coweighting::<F64Rig>(&space)
            .expect("generic coweighting");

        for i in 0..n {
            assert!(
                (w_fast[i] - w_gen[i].0).abs() < TOL,
                "{label} w[{i}]: fast={} generic={}",
                w_fast[i],
                w_gen[i].0
            );
            assert!(
                (v_fast[i] - v_gen[i].0).abs() < TOL,
                "{label} v[{i}]: fast={} generic={}",
                v_fast[i],
                v_gen[i].0
            );
            // Symmetric ζ ⇒ weighting = coweighting (Leinster 2013 §1.1).
            assert!((w_fast[i] - v_fast[i]).abs() < TOL, "{label} w != v at {i}");
        }

        // Magnitude = Σ w = Σ v (Leinster 2013 Lemma 1.1.2).
        let mag = fact.magnitude().expect("fast magnitude");
        assert!((mag - w_fast.iter().sum::<f64>()).abs() < 1e-12, "{label}");
        assert!((mag - v_fast.iter().sum::<f64>()).abs() < TOL, "{label}");
    }
}

#[test]
fn asymmetric_coweighting_differs_from_weighting_but_sums_agree() {
    let space = asymmetric_space();
    let fact = ZetaFactorization::new(&space);
    assert_eq!(fact.path(), FactorizationPath::GaussJordan);

    let w = fact.weighting().expect("fast weighting");
    let v = fact.coweighting().expect("fast coweighting");

    // Genuinely different vectors on an asymmetric ζ …
    assert!(
        w.iter().zip(&v).any(|(a, b)| (a - b).abs() > 1e-6),
        "asymmetric ζ should give w != v; w={w:?} v={v:?}"
    );
    // … with equal sums (Leinster 2013 Lemma 1.1.2).
    let sum_w: f64 = w.iter().sum();
    let sum_v: f64 = v.iter().sum();
    assert!((sum_w - sum_v).abs() < TOL, "Σw={sum_w:.12} Σv={sum_v:.12}");
    assert!((fact.magnitude().expect("fast magnitude") - sum_w).abs() < 1e-12);
}

// ---------------------------------------------------------------------------
// Error / empty parity
// ---------------------------------------------------------------------------

#[test]
fn singular_zeta_errors_on_both_routes() {
    let space = singular_space();
    assert!(
        mobius_function::<F64Rig>(&space).is_err(),
        "generic path must reject duplicate rows"
    );
    assert!(catgraph_magnitude::magnitude::weighting::<F64Rig>(&space).is_err());

    let fact = ZetaFactorization::new(&space);
    assert_eq!(fact.path(), FactorizationPath::GaussJordan);
    assert!(fact.weighting().is_err(), "fast weighting must reject");
    assert!(fact.coweighting().is_err(), "fast coweighting must reject");
    assert!(fact.magnitude().is_err(), "fast magnitude must reject");
    assert!(fact.mobius_function().is_err(), "fast Möbius must reject");
    assert!(fact.condition_report().is_err());
    assert!(fact.condition_lower_bound().is_err());
    assert!(
        magnitude_f64(&space, 1.0).is_err(),
        "scaled entry point too"
    );
}

#[test]
fn empty_space_parity() {
    let space: LawvereMetricSpace<usize> = LawvereMetricSpace::new(vec![]);
    let fact = ZetaFactorization::new(&space);
    assert_eq!(fact.size(), 0);
    assert!(fact.is_empty());

    assert!(fact.weighting().expect("empty weighting").is_empty());
    assert!(fact.coweighting().expect("empty coweighting").is_empty());
    let mu = fact.mobius_function().expect("empty Möbius");
    assert_eq!((mu.rows(), mu.cols()), (0, 0));

    let fast = fact.magnitude().expect("empty magnitude");
    let generic: F64Rig = magnitude(&space, 1.0).expect("generic empty magnitude");
    assert!((fast - generic.0).abs() < f64::EPSILON);
    assert!((magnitude_f64(&space, 1.0).expect("scaled empty") - generic.0).abs() < f64::EPSILON);

    let report = fact.condition_lower_bound().expect("empty report");
    assert!((report.cond_1_lower_bound() - 0.0).abs() < f64::EPSILON);
    assert!(report.cond_1().is_none());

    // Degenerate n = 0 exact report: ‖ζ‖₁ = ‖μ‖₁ = 0, so cond₁ reports
    // `Some(0.0)` — deliberately below the ≥ 1 floor, which only applies for
    // n ≥ 1 (documented on `ConditionReport::cond_1`). Pinned here so the
    // degenerate value cannot drift silently.
    let exact = fact.condition_report().expect("empty exact report");
    assert_eq!(exact.path(), fact.path());
    assert!((exact.zeta_one_norm() - 0.0).abs() < f64::EPSILON);
    assert_eq!(exact.mu_one_norm(), Some(0.0));
    assert_eq!(exact.cond_1(), Some(0.0));
    assert!((exact.cond_1_lower_bound() - 0.0).abs() < f64::EPSILON);
}

// ---------------------------------------------------------------------------
// Condition reporting
// ---------------------------------------------------------------------------

#[test]
fn condition_report_is_at_least_one_and_bounds_are_sound() {
    for (label, space) in [
        ("cholesky", scattered_uniform_space(5, 0.6)),
        ("lblt", symmetric_indefinite_star(3)),
        ("gauss_jordan", asymmetric_space()),
    ] {
        let fact = ZetaFactorization::new(&space);
        let exact = fact.condition_report().expect("exact report");
        let cheap = fact.condition_lower_bound().expect("solve-only report");

        assert_eq!(exact.path(), fact.path(), "{label}");
        assert_eq!(cheap.path(), fact.path(), "{label}");
        assert!(
            (exact.zeta_one_norm() - fact.zeta_one_norm()).abs() < f64::EPSILON,
            "{label}"
        );

        let cond = exact.cond_1().expect("μ materialized ⇒ exact cond₁");
        // cond₁(ζ) = ‖ζ‖₁‖ζ⁻¹‖₁ ≥ ‖ζ ζ⁻¹‖₁ = ‖I‖₁ = 1 (submultiplicativity).
        assert!(cond >= 1.0 - TOL, "{label}: cond₁ = {cond}");
        assert!(
            (cond - exact.zeta_one_norm() * exact.mu_one_norm().expect("‖μ‖₁")).abs() < TOL,
            "{label}"
        );

        // The lower bound is exactly that — never above the true cond₁ …
        assert!(
            exact.cond_1_lower_bound() <= cond * (1.0 + 1e-12),
            "{label}: bound {} > cond₁ {cond}",
            exact.cond_1_lower_bound()
        );
        // … and the two constructors derive it identically (μ row sums = w).
        assert!(
            (exact.cond_1_lower_bound() - cheap.cond_1_lower_bound()).abs() < TOL,
            "{label}: exact-report bound {} vs solve-only {}",
            exact.cond_1_lower_bound(),
            cheap.cond_1_lower_bound()
        );
        assert!(cheap.cond_1().is_none(), "{label}");
        assert!(cheap.mu_one_norm().is_none(), "{label}");
    }
}

#[test]
fn condition_number_grows_as_t_shrinks_on_a_near_all_ones_fixture() {
    // Uniform 6-point space: ζ_t = (1 − c)I + cJ with c = e^(−t·d) → 1 as
    // t → 0, i.e. ζ_t → J, which is rank 1. Positive-definite throughout, so
    // the Cholesky route holds and cond₁ must blow up monotonically.
    let base = scattered_uniform_space(6, 0.5);
    let mut previous = 0.0_f64;
    for t in [1.0_f64, 0.5, 0.1, 0.01, 0.001] {
        let scaled = scale(&base, t);
        let fact = ZetaFactorization::new(&scaled);
        assert_eq!(fact.path(), FactorizationPath::Cholesky, "t={t}");
        let cond = fact
            .condition_report()
            .expect("positive-definite ⇒ invertible")
            .cond_1()
            .expect("μ materialized");
        assert!(
            cond > previous,
            "cond₁ must grow as t shrinks: t={t} cond={cond:.6e} previous={previous:.6e}"
        );
        previous = cond;
    }
    assert!(
        previous > 1e3,
        "cond₁ at t = 0.001 should be large, got {previous:.6e}"
    );
}
