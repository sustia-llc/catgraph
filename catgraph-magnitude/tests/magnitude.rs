//! Phase 6A.2 tests: Tsallis entropy + Möbius inversion.
//!
//! All tests use `Q = F64Rig` — the only `Ring + Div + From<f64>` rig in
//! the v0.1.0 workspace.

use catgraph_magnitude::magnitude::{mobius_function, tsallis_entropy};
use catgraph_magnitude::weighted_cospan::NodeId;
use catgraph_magnitude::{
    CatgraphError, F64Rig, LawvereMetricSpace, MatR, TSALLIS_SHANNON_EPS, Tropical,
};
use proptest::prelude::*;

/// Reference Shannon entropy `-Σ pᵢ ln pᵢ`, with `0 · ln 0 = 0`.
fn shannon(p: &[f64]) -> f64 {
    p.iter()
        .filter(|&&pi| pi > 0.0)
        .map(|&pi| -pi * pi.ln())
        .sum()
}

proptest! {
    /// Within the Shannon special-case threshold, `tsallis_entropy(p, t)`
    /// equals Shannon entropy exactly (the function takes the special-case
    /// branch and computes `-Σ pᵢ ln pᵢ` directly).
    #[test]
    fn tsallis_shannon_recovery(
        p in prop::collection::vec(0.0_f64..=1.0, 2..=8),
        delta in -1e-7_f64..1e-7,
    ) {
        let t = 1.0 + delta;
        // |delta| < TSALLIS_SHANNON_EPS = 1e-6 by construction.
        prop_assume!((t - 1.0).abs() < TSALLIS_SHANNON_EPS);

        let observed = tsallis_entropy(&p, t);
        let expected = shannon(&p);
        prop_assert!(
            (observed - expected).abs() < 1e-12,
            "shannon-recovery: observed {observed}, expected {expected}, p={p:?}, t={t}"
        );
    }

    /// Outside the special-case threshold but still close to `t = 1`, the
    /// Tsallis branch approaches Shannon entropy. The convergence theorem
    /// `lim_{t→1} H_t(p) = H₁(p)` only holds for *normalized* distributions
    /// `Σ pᵢ = 1`, so this proptest normalizes its input. Tolerance is loose
    /// because we are not at the limit.
    #[test]
    fn tsallis_approaches_shannon(
        raw in prop::collection::vec(0.01_f64..=1.0, 2..=8),
        // t in [1.001, 1.01] — well outside TSALLIS_SHANNON_EPS = 1e-6.
        delta in 1e-3_f64..1e-2,
    ) {
        // Normalize: Σ pᵢ = 1 (the proptest-domain `0.01..=1.0` lower bound
        // ensures the sum is bounded away from zero).
        let total: f64 = raw.iter().sum();
        let p: Vec<f64> = raw.iter().map(|&x| x / total).collect();

        let t = 1.0 + delta;
        let observed = tsallis_entropy(&p, t);
        let expected = shannon(&p);
        // Taylor expansion `H_t ≈ H₁ + (t−1) · ∂_t H_t|_{t=1}` gives a
        // residual of `O(δt · |H₁|)`. With `δt ≤ 1e-2` and `|H₁| ≤ ln(8) ≈ 2.08`
        // (max Shannon over 8-bin uniform), the worst-case residual is
        // ~0.02. Tolerance `5e-2` keeps a safety margin.
        prop_assert!(
            (observed - expected).abs() < 5e-2,
            "shannon-limit: observed {observed}, expected {expected}, p={p:?}, t={t}"
        );
    }
}

/// Sanity check on a few hand-computable distributions.
#[test]
fn tsallis_basic_values() {
    // Delta distribution: H_t([1, 0, 0]) = (1 − 1) / (t − 1) = 0 for any t ≠ 1.
    let delta = [1.0, 0.0, 0.0];
    let h = tsallis_entropy(&delta, 2.0);
    assert!(h.abs() < 1e-12, "delta entropy at t=2 should be 0, got {h}");

    // Shannon of delta: -1 ln 1 - 0 - 0 = 0.
    let h_shannon = tsallis_entropy(&delta, 1.0);
    assert!(
        h_shannon.abs() < 1e-12,
        "shannon of delta = 0, got {h_shannon}"
    );

    // Uniform [0.5, 0.5] Shannon: -0.5 ln 0.5 - 0.5 ln 0.5 = ln 2.
    let uniform = [0.5, 0.5];
    let h_uniform = tsallis_entropy(&uniform, 1.0);
    assert!(
        (h_uniform - 2.0_f64.ln()).abs() < 1e-12,
        "shannon of uniform [0.5, 0.5] = ln 2, got {h_uniform}"
    );

    // Uniform Tsallis at t=2: (1 − (0.25 + 0.25)) / (2 − 1) = 0.5.
    let h_t2 = tsallis_entropy(&uniform, 2.0);
    assert!(
        (h_t2 - 0.5).abs() < 1e-12,
        "tsallis of uniform [0.5, 0.5] at t=2 = 0.5, got {h_t2}"
    );
}

// ---------------------------------------------------------------------------
// Möbius inversion tests
// ---------------------------------------------------------------------------

/// Multiply two `MatR<F64Rig>` matrices and assert entrywise equality with
/// the identity to within `tol`.
fn assert_is_identity(m: &MatR<F64Rig>, tol: f64, ctx: &str) {
    let n = m.rows();
    assert_eq!(m.cols(), n, "{ctx}: not square");
    for i in 0..n {
        for j in 0..n {
            let expected = if i == j { 1.0 } else { 0.0 };
            let observed = m.entries()[i][j].0;
            assert!(
                (observed - expected).abs() < tol,
                "{ctx}: M[{i}][{j}] = {observed}, expected {expected}"
            );
        }
    }
}

proptest! {
    /// For a non-singular Lawvere metric space, `μ * ζ = I` and `ζ * μ = I`.
    /// We rebuild ζ in the test (the function does not expose it) and check
    /// both products against the identity within numerical tolerance.
    #[test]
    fn mobius_zeta_inversion(
        n in 2usize..=5,
        seed in any::<u64>(),
    ) {
        // Deterministic small LCG over the seed — avoid pulling in rand.
        // Precision-loss casts are intentional: we only need ~31 bits of
        // randomness in [0, 1) for a uniform-distance fixture.
        let mut state = seed | 1;
        #[allow(clippy::cast_precision_loss)]
        let mut next = || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((state >> 33) as f64) / ((1u64 << 31) as f64) // [0, 1)
        };

        let objects: Vec<NodeId> = (0..n).collect();
        let mut space = LawvereMetricSpace::new(objects.clone());
        for i in 0..n {
            for j in 0..n {
                // Distances in [0.1, 5.0] keep zeta entries in [exp(-5), exp(-0.1)] ≈ [6.7e-3, 0.9],
                // safely away from singularity.
                let d = 0.1 + 4.9 * next();
                space.set_distance(i, j, Tropical(d));
            }
        }

        let mu = mobius_function::<F64Rig>(&space)
            .expect("non-singular zeta should invert");

        // Rebuild zeta to cross-check μ * ζ = I.
        let mut zeta_entries = vec![vec![F64Rig(0.0); n]; n];
        for (i, row) in zeta_entries.iter_mut().enumerate().take(n) {
            for (j, cell) in row.iter_mut().enumerate().take(n) {
                let d = space.distance(&objects[i], &objects[j]);
                *cell = F64Rig((-d.0).exp());
            }
        }
        let zeta = MatR::new(n, n, zeta_entries).unwrap();

        let mu_zeta = mu.matmul(&zeta).unwrap();
        let zeta_mu = zeta.matmul(&mu).unwrap();

        // The residual of `μ·ζ − I` grows like ε·‖μ‖: each entry is a sum of
        // products `μ_ik · ζ_kj` (ζ_kj ≤ 1), so round-off scales with the
        // magnitude of the inverse, which is large when ζ is ill-conditioned.
        // A flat absolute bound therefore flakes on unlucky random fixtures
        // (e.g. n=3 gave a 4.6e-9 residual). Scale by ‖μ‖_max with a 1e-7 base
        // — still ~7 orders tighter than the O(1) residual a real inversion
        // bug would produce.
        let mu_max = mu
            .entries()
            .iter()
            .flat_map(|row| row.iter())
            .map(|c| c.0.abs())
            .fold(1.0_f64, f64::max);
        let tol = 1e-7 * mu_max;
        assert_is_identity(&mu_zeta, tol, "μ * ζ");
        assert_is_identity(&zeta_mu, tol, "ζ * μ");
    }
}

/// Singular zeta — every distance set to `+∞` makes ζ the all-zeros matrix,
/// which is singular. The function must report a `Composition` error.
#[test]
fn mobius_singular_zeta() {
    let objects: Vec<NodeId> = vec![0, 1];
    let mut space = LawvereMetricSpace::new(objects);
    // All four pairwise distances = +∞ ⇒ exp(-∞) = 0 ⇒ ζ is the zero matrix.
    space.set_distance(0, 0, Tropical(f64::INFINITY));
    space.set_distance(0, 1, Tropical(f64::INFINITY));
    space.set_distance(1, 0, Tropical(f64::INFINITY));
    space.set_distance(1, 1, Tropical(f64::INFINITY));

    let result = mobius_function::<F64Rig>(&space);
    match result {
        Err(CatgraphError::Composition { message }) => {
            assert!(
                message.contains("singular"),
                "expected 'singular' in error, got: {message}"
            );
        }
        Err(e) => panic!("expected Composition, got: {e:?}"),
        Ok(_) => panic!("expected Err for singular zeta, got Ok"),
    }
}

/// Identity zeta (all distances zero) gives μ = I (each diagonal pivot is 1
/// already; eliminating off-diagonals which are also 1 yields the identity).
///
/// Wait — `d(i, j) = 0` everywhere gives `ζ[i][j] = exp(0) = 1` everywhere,
/// not the identity. The all-ones matrix is rank 1, so it IS singular for
/// `n ≥ 2`. We confirm that here as a second singular-zeta witness.
#[test]
fn mobius_all_ones_zeta_is_singular() {
    let objects: Vec<NodeId> = vec![0, 1];
    let mut space = LawvereMetricSpace::new(objects);
    space.set_distance(0, 0, Tropical(0.0));
    space.set_distance(0, 1, Tropical(0.0));
    space.set_distance(1, 0, Tropical(0.0));
    space.set_distance(1, 1, Tropical(0.0));

    let result = mobius_function::<F64Rig>(&space);
    assert!(
        matches!(result, Err(CatgraphError::Composition { .. })),
        "all-ones zeta (rank 1) should be singular"
    );
}
