//! Tsallis entropy + Möbius inversion tests.
//!
//! All tests use `Q = F64Rig` — the only `Ring + Div + From<f64>` rig in
//! the workspace.

use catgraph_magnitude::magnitude::{mobius_function, tsallis_entropy};
use catgraph_magnitude::weighted_cospan::NodeId;
use catgraph_magnitude::{
    CatgraphError, F64Rig, LawvereMetricSpace, MatR, TSALLIS_SHANNON_EPS, Tropical,
};
use catgraph_testutil::Lcg;
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
    ///
    /// `|δ| < 1e-7` by the range, and `t − 1.0` is exact for `|δ| ≤ 1`, so
    /// `|t − 1| < TSALLIS_SHANNON_EPS = 1e-6` holds by construction.
    #[test]
    fn tsallis_shannon_recovery(
        p in prop::collection::vec(0.0_f64..=1.0, 2..=8),
        delta in -1e-7_f64..1e-7,
    ) {
        let t = 1.0 + delta;

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

// ---------------------------------------------------------------------------
// The TSALLIS_SHANNON_EPS branch switch
// ---------------------------------------------------------------------------

/// Budget, asserted by [`tsallis_band_tolerances_are_measured`], on
/// `|H_t(p) − H₁(p)| / |t − 1|` over the Tsallis side of the step band
/// `|t − 1| ∈ [1e-9, 1e-2)`, for `p` over the domain of [`normalized_p`].
///
/// In `u = t − 1` the Tsallis branch expands as `H_t = H₁ − u·A₂/2 − u²·A₃/6
/// − …` with `Aₖ = Σ pᵢ lnᵏ pᵢ`, so the ratio is `A₂/2 + |u|·|A₃|/6 + O(u²)`.
/// At the 8-bin uniform distribution and the top of the band that is
/// `ln²8/2 + 1e-2·ln³8/6 = 2.162 + 0.015`. The budget keeps a margin over it;
/// [`tsallis_band_tolerances_are_measured`] sweeps the same domain
/// deterministically and asserts the observed maximum ratio stays under it.
const BAND_RESIDUAL_RATIO: f64 = 3.0;

/// Budget on `|H_{t₊}(p) − H_{t₋}(p)|` for `t₊`, `t₋` the two adjacent `f64`s
/// straddling the `TSALLIS_SHANNON_EPS` branch cut, for `p` from
/// [`normalized_p`].
///
/// One side takes the Shannon branch and returns `-Σ pᵢ ln pᵢ`; the other
/// divides by `|t − 1| ≈ 1e-6`, so the gap carries the leading Taylor term
/// `|u|·A₂/2`, which is `1e-6·ln²8/2 = 2.16e-6` at the 8-bin uniform
/// distribution, plus that branch's numerator rounding scaled by `1/|u| ≈ 1e6`.
/// The budget keeps a margin over it;
/// [`tsallis_band_tolerances_are_measured`] asserts the observed maximum gap
/// stays under it.
const BRANCH_CUT_GAP: f64 = 5e-6;

/// The threshold `TSALLIS_SHANNON_EPS` is documented at, as a literal.
///
/// [`tsallis_branch_contract_over_the_step_band`] splits its two arms on this
/// rather than on the constant, so a change to the constant's value reaches
/// that property as a failing case instead of moving the property's own split
/// along with the implementation's.
const DOCUMENTED_EPS: f64 = 1e-6;

/// Coefficient of the `u²` term in the bound
/// [`tsallis_branch_contract_over_the_step_band`] puts on the Tsallis branch's
/// deviation from its own two-term expansion, in `u = t − 1`.
///
/// `H_t = H₁ − u·A₂/2 − u²·A₃/6 − u³·A₄/24 − …` with `Aₖ = Σ pᵢ lnᵏ pᵢ`, so the
/// deviation from `H₁ − u·A₂/2` is `u²·(|A₃|/6 + |u|·A₄/24 + O(u²))`. At the
/// 8-bin uniform distribution and the top of the band that coefficient is
/// `ln³8/6 + 1e-2·ln⁴8/24 = 1.499 + 0.008`; the budget keeps a margin over it.
const TAYLOR_REMAINDER_K: f64 = 3.0;

/// Numerator-rounding allowance in the same bound, entering as `_/|u|`.
///
/// The Tsallis branch divides `1 − Σ pᵢᵗ` by `u`, so an absolute error in that
/// sum — at most 8 `powf` results and 7 additions, against `ULP(1) = 2.2e-16`
/// — reaches the returned value scaled by `1/|u|`, which is `1e6` at the
/// bottom of the band. The budget is ~450 ULP of 1.
const TAYLOR_ROUNDING_FLOOR: f64 = 1e-13;

/// A probability vector: 2..=8 entries drawn from `[0.01, 1.0]` and rescaled to
/// `Σ pᵢ = 1`. Entries are then `≥ 0.01/8 > 0`, so the Shannon branch's
/// `pᵢ > 0` filter drops nothing.
fn normalized_p() -> impl Strategy<Value = Vec<f64>> {
    prop::collection::vec(0.01_f64..=1.0, 2..=8).prop_map(|raw| {
        let total: f64 = raw.iter().sum();
        raw.iter().map(|&x| x / total).collect()
    })
}

/// A finite-difference step `h` over `[1e-9, 1e-2)`, log-uniform: the decade is
/// sampled, so each of the seven decades carries equal weight.
fn log_uniform_step() -> impl Strategy<Value = f64> {
    (1.0_f64..10.0, 3_i32..=9).prop_map(|(mantissa, decade)| mantissa * 10_f64.powi(-decade))
}

/// The two adjacent `f64`s straddling the branch cut above `t = 1`: `.0` is the
/// largest `t > 1` for which `(t − 1).abs() < TSALLIS_SHANNON_EPS` holds, `.1`
/// its immediate successor, for which it does not.
fn straddle_above() -> (f64, f64) {
    let mut shannon_side = 1.0 + TSALLIS_SHANNON_EPS;
    for _ in 0..4 {
        if (shannon_side - 1.0).abs() < TSALLIS_SHANNON_EPS {
            break;
        }
        shannon_side = shannon_side.next_down();
    }
    let tsallis_side = shannon_side.next_up();
    assert!(
        (shannon_side - 1.0).abs() < TSALLIS_SHANNON_EPS
            && (tsallis_side - 1.0).abs() >= TSALLIS_SHANNON_EPS,
        "straddle_above: {shannon_side} and {tsallis_side} do not straddle the cut \
         at eps {TSALLIS_SHANNON_EPS}"
    );
    (shannon_side, tsallis_side)
}

/// The mirror of [`straddle_above`] below `t = 1`: `.0` is the smallest `t < 1`
/// for which `(t − 1).abs() < TSALLIS_SHANNON_EPS` holds, `.1` its immediate
/// predecessor, for which it does not.
fn straddle_below() -> (f64, f64) {
    let mut shannon_side = 1.0 - TSALLIS_SHANNON_EPS;
    for _ in 0..4 {
        if (shannon_side - 1.0).abs() < TSALLIS_SHANNON_EPS {
            break;
        }
        shannon_side = shannon_side.next_up();
    }
    let tsallis_side = shannon_side.next_down();
    assert!(
        (shannon_side - 1.0).abs() < TSALLIS_SHANNON_EPS
            && (tsallis_side - 1.0).abs() >= TSALLIS_SHANNON_EPS,
        "straddle_below: {shannon_side} and {tsallis_side} do not straddle the cut \
         at eps {TSALLIS_SHANNON_EPS}"
    );
    (shannon_side, tsallis_side)
}

proptest! {
    /// Over the finite-difference step band `|t − 1| ∈ [1e-9, 1e-2)` on both
    /// sides of `t = 1`, `tsallis_entropy` follows the branch contract at
    /// `DOCUMENTED_EPS`: below the threshold it returns Shannon entropy
    /// `H₁ = -Σ pᵢ ln pᵢ` to within `1e-12`; at or above it, the Tsallis
    /// branch's two-term expansion `H₁ − u·A₂/2` — `u = t − 1`,
    /// `A₂ = Σ pᵢ ln²pᵢ` — to within
    /// `TAYLOR_REMAINDER_K·u² + TAYLOR_ROUNDING_FLOOR/|u|`.
    ///
    /// The Tsallis arm is two-sided in `u`: an implementation returning `H₁`
    /// for every `t` omits the `u·A₂/2` term and fails it.
    ///
    /// The range spans the gap between `tsallis_shannon_recovery`'s `|δ| <
    /// 1e-7` and `tsallis_approaches_shannon`'s `δ ∈ [1e-3, 1e-2)`, and covers
    /// the `t < 1` side of the latter.
    #[test]
    fn tsallis_branch_contract_over_the_step_band(
        p in normalized_p(),
        h in log_uniform_step(),
        below_one in any::<bool>(),
    ) {
        let t = if below_one { 1.0 - h } else { 1.0 + h };
        // Exact for `|h| ≤ 1` (Sterbenz), so `u` is the step the Tsallis
        // branch divides by, not the drawn `h`.
        let u = t - 1.0;
        let observed = tsallis_entropy(&p, t);
        let shannon_value = shannon(&p);

        if u.abs() < DOCUMENTED_EPS {
            let residual = (observed - shannon_value).abs();
            prop_assert!(
                residual < 1e-12,
                "shannon branch: observed {observed}, expected {shannon_value} \
                 (residual {residual:.3e}, allowed 1e-12), p={p:?}, t={t}"
            );
        } else {
            let a2: f64 = p.iter().map(|&pi| pi * pi.ln() * pi.ln()).sum();
            let expected = shannon_value - u * a2 / 2.0;
            let residual = (observed - expected).abs();
            let allowed = TAYLOR_REMAINDER_K * u * u + TAYLOR_ROUNDING_FLOOR / u.abs();
            prop_assert!(
                residual <= allowed,
                "tsallis branch: observed {observed}, expected {expected} \
                 (residual {residual:.3e}, allowed {allowed:.3e}), \
                 shannon {shannon_value}, a2 {a2}, u {u:e}, p={p:?}, t={t}"
            );
        }
    }

    /// Across the `TSALLIS_SHANNON_EPS` branch cut — the two adjacent `f64`s
    /// on either side of it, above and below `t = 1` — the Shannon-branch and
    /// Tsallis-branch values differ by at most `BRANCH_CUT_GAP`.
    #[test]
    fn tsallis_branch_cut_is_continuous(p in normalized_p()) {
        for (t_shannon, t_tsallis) in [straddle_above(), straddle_below()] {
            let shannon_value = tsallis_entropy(&p, t_shannon);
            let tsallis_value = tsallis_entropy(&p, t_tsallis);
            let gap = (tsallis_value - shannon_value).abs();
            prop_assert!(
                gap <= BRANCH_CUT_GAP,
                "branch cut t_shannon={t_shannon} t_tsallis={t_tsallis}: \
                 shannon {shannon_value}, tsallis {tsallis_value} \
                 (gap {gap:.3e}, allowed {BRANCH_CUT_GAP:.3e}), p={p:?}"
            );
        }
    }
}

/// Deterministic sweep of the domains behind [`BAND_RESIDUAL_RATIO`] and
/// [`BRANCH_CUT_GAP`], asserting each observed maximum stays under its budget
/// and printing both maxima with the drawn `t` and `p` that produced them.
#[test]
fn tsallis_band_tolerances_are_measured() {
    const DISTRIBUTIONS: usize = 4_000;

    let mut rng = Lcg::new(0x0022_5F2D);
    let mut max_ratio = 0.0_f64;
    let mut ratio_witness = (Vec::new(), f64::NAN);
    let mut max_gap = 0.0_f64;
    let mut gap_witness = (Vec::new(), f64::NAN);
    let mut band_cases = 0_usize;
    let mut shannon_cases = 0_usize;

    for _ in 0..DISTRIBUTIONS {
        let n = rng.next_usize(2, 8);
        let raw: Vec<f64> = (0..n).map(|_| 0.01 + 0.99 * rng.next_f64()).collect();
        let total: f64 = raw.iter().sum();
        let p: Vec<f64> = raw.iter().map(|&x| x / total).collect();
        let reference = shannon(&p);

        for decade in 3..=9_i32 {
            let h = (1.0 + 9.0 * rng.next_f64()) * 10_f64.powi(-decade);
            for step in [h, -h] {
                let t = 1.0 + step;
                let u = t - 1.0;
                if u.abs() < TSALLIS_SHANNON_EPS {
                    shannon_cases += 1;
                    continue;
                }
                band_cases += 1;
                let ratio = (tsallis_entropy(&p, t) - reference).abs() / u.abs();
                if ratio > max_ratio {
                    max_ratio = ratio;
                    ratio_witness = (p.clone(), t);
                }
            }
        }

        for (t_shannon, t_tsallis) in [straddle_above(), straddle_below()] {
            let gap = (tsallis_entropy(&p, t_tsallis) - tsallis_entropy(&p, t_shannon)).abs();
            if gap > max_gap {
                max_gap = gap;
                gap_witness = (p.clone(), t_tsallis);
            }
        }
    }

    println!(
        "\ntsallis band: {band_cases} Tsallis-side and {shannon_cases} Shannon-side cases, \
         max |residual|/|t−1| = {max_ratio:.6} at t={}, p={:?}",
        ratio_witness.1, ratio_witness.0
    );
    println!(
        "\ntsallis branch cut: max gap = {max_gap:.6e} at t={}, p={:?}",
        gap_witness.1, gap_witness.0
    );

    assert!(
        max_ratio <= BAND_RESIDUAL_RATIO,
        "band residual ratio: observed {max_ratio}, budget {BAND_RESIDUAL_RATIO}, \
         at t={}, p={:?}",
        ratio_witness.1,
        ratio_witness.0
    );
    assert!(
        max_gap <= BRANCH_CUT_GAP,
        "branch-cut gap: observed {max_gap}, budget {BRANCH_CUT_GAP}, at t={}, p={:?}",
        gap_witness.1,
        gap_witness.0
    );
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
        // `| 1` seed prep stays at the call site (#33).
        let mut rng = Lcg::new(seed | 1);

        let objects: Vec<NodeId> = (0..n).collect();
        let mut space = LawvereMetricSpace::new(objects.clone());
        for i in 0..n {
            for j in 0..n {
                // Distances in [0.1, 5.0] keep zeta entries in [exp(-5), exp(-0.1)] ≈ [6.7e-3, 0.9],
                // safely away from singularity.
                let d = 0.1 + 4.9 * rng.next_f64();
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
