//! Magnitude-homology rank recovery + BV 2025 Prop 3.14 acceptance gate.
//!
//! Module split: the chain-complex substrate (LS 2017 §2 materialisation —
//! [`Chain`](super::Chain), [`enumerate_chains`](super::enumerate_chains),
//! [`ChainIndex`], [`boundary_matrix`]) lives in [`super`]; the
//! rank-recovery + acceptance machinery lives here.
//!
//! ## Rig widening
//!
//! [`IntegerLikeRig`] parameterizes the rank-recovery surface over both
//! `F64Rig` and `Z(BigInt)`. Existing `F64Rig` callers
//! compile unchanged via the blanket impl; `magnitude_homology_rank::<Z>`
//! and `euler_char_identity_at::<F64Rig>` produce identical ranks on
//! identical inputs.

use catgraph::errors::CatgraphError;
use catgraph_applied::lawvere_metric::LawvereMetricSpace;
use catgraph_applied::mat::MatR;
use catgraph_applied::rig::{F64Rig, Rig};
use catgraph_applied::z::Z;

use super::{ChainIndex, boundary_matrix};
use crate::weighted_cospan::NodeId;

/// Rank-recovery primes: primary Mersenne `2^31 − 1`, secondary cross-check,
/// tertiary fallback. All three are large primes near `i64::MAX/2` so
/// divisibility by any small integer invariant factor is vanishingly
/// unlikely. Multi-prime CRT reconstruction is deferred (#35).
const RANK_RECOVERY_PRIMES: [i64; 3] = [
    2_147_483_647, // primary — Mersenne 2^31 − 1
    2_147_483_629, // secondary — cross-check
    2_147_483_587, // tertiary — fallback if primary/secondary disagree
];

/// Rigs carrying integer-exact (or `i64`-coercible) arithmetic for the
/// rank-recovery interior of [`magnitude_homology_rank`] and
/// [`euler_char_identity_at`].
///
/// Replaces a private `type RankQ = F64Rig;` alias. Existing
/// `F64Rig` callers continue to compile unchanged via the blanket impl;
/// `Z(BigInt)` is a first-class rank-recovery rig.
///
/// ## Why a trait, not a type alias
///
/// A type alias would coerce any `Rig` argument to `F64Rig`, since the
/// private `snf_rank_over_zp` helper reads the `F64Rig` tuple-struct field
/// `x.0` directly. With [`Z(BigInt)`](Z) as a second integer-exact rig
/// (Leinster 2008 Cor 1.5 substrate), the trait lets both share the
/// rank-recovery interior without runtime branching.
///
/// ## Fallibility
///
/// [`to_i64`](IntegerLikeRig::to_i64) is fallible: [`Z(BigInt)`](Z)
/// values may exceed `i64` range. In practice, boundary-matrix entries
/// carry only `±1` or `0` (LS 2017 Def 2.5 alternating-sum face map),
/// so the fallible signature defends against future regressions (e.g.
/// a magnitude-homology fixture that builds non-binary boundary entries)
/// rather than guarding the shipped path.
///
/// ## Forward-look
///
/// Multi-prime CRT extends the rank-recovery interior to multi-prime
/// reconstruction; the [`IntegerLikeRig`] surface is the natural seam for
/// that work.
///
/// ## Bounds
///
/// `Rig` already entails `Clone`; an explicit `+ Clone` would be
/// redundant and would obscure the actual added bound (`From<i64>`).
/// `'static` is likewise unnecessary — no callee in the rank-recovery
/// interior boxes or downcasts `Q`, so omitting it leaves room for
/// borrowed-arena rigs (e.g. a `&'a BigInt` arena rig).
pub trait IntegerLikeRig: Rig + From<i64> {
    /// Convert to an `i64` for the SNF-mod-p rank-recovery interior.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] if the value exceeds `i64`
    /// range. Boundary-matrix entries are bounded `±1` or `0` on the shipped
    /// fixtures, so the error path defends against future regressions.
    fn to_i64(&self) -> Result<i64, CatgraphError>;
}

impl IntegerLikeRig for F64Rig {
    fn to_i64(&self) -> Result<i64, CatgraphError> {
        // Boundary-matrix entries are integer-valued f64 (±1 or 0);
        // round-to-nearest is a no-op in practice. Debug-only regression
        // guard: if a future fixture builds non-integer-valued boundary
        // entries, the assertion fires; release builds preserve the
        // round-to-nearest defence.
        debug_assert!(
            (self.0 - self.0.round()).abs() < 1e-9,
            "F64Rig::to_i64: non-integer-valued boundary entry {} would be silently rounded; \
             a future fixture has broken the ±1/0 invariant",
            self.0
        );
        #[allow(
            clippy::cast_possible_truncation,
            reason = "boundary-matrix entries are integer-valued f64 (±1 or 0); round() is a no-op"
        )]
        Ok(self.0.round() as i64)
    }
}

impl IntegerLikeRig for Z {
    fn to_i64(&self) -> Result<i64, CatgraphError> {
        use num::ToPrimitive;
        self.0.to_i64().ok_or_else(|| CatgraphError::Composition {
            message: format!(
                "Z value {} exceeds i64 range; rank-recovery interior requires \
                 i64-fitting boundary entries (typically ±1). Escalate to a \
                 BigInt-native rank path.",
                self.0
            ),
        })
    }
}

/// Rank of `H_{k,ℓ}(M) = ker(∂_k) / im(∂_{k+1})` over ℤ via single-prime SNF
/// + 2-prime cross-check.
///
/// Computes `rank(B mod p) over Z/p` for the primary Mersenne prime; cross-
/// checks against a secondary large prime; on disagreement, falls through to
/// a tertiary prime. For all shipped magnitude-homology fixtures,
/// the primary prime is sufficient — the cross-check exists to catch
/// pathological matrices where invariant factors happen to be divisible by
/// `2^31 − 1`. Multi-prime CRT reconstruction (full integer invariant-factor
/// recovery, not just rank) is deferred (#35).
///
/// Computes: `rank(H_{k,ℓ}) = cols(∂_k) − rank(∂_k) − rank(∂_{k+1})`.
///
/// Special cases:
/// - `k == 0`: `rank(H_{0,ℓ}) = cols(∂_0) − rank(∂_1)` (∂_0 has empty image,
///   so subtract only ∂_1's rank).
/// - Empty bucket: returns 0.
///
/// # Errors
///
/// - Boundary-matrix construction failure (length-grade tolerance issue).
/// - All three rank-recovery primes disagree on at least one boundary
///   matrix's rank (vanishingly unlikely on representative fixtures; if it
///   fires, escalate to the multi-prime CRT path).
pub fn magnitude_homology_rank<Q: IntegerLikeRig>(
    idx: &ChainIndex,
    space: &LawvereMetricSpace<NodeId>,
    k: usize,
    ell: f64,
) -> Result<usize, CatgraphError> {
    let n_cols_k = idx.chains_at(k, ell).len();
    let rank_partial_k = if k > 0 {
        let bk = boundary_matrix::<Q>(idx, space, k, ell)?;
        snf_rank_with_cross_check(&bk)?
    } else {
        0
    };
    let rank_partial_kplus1 = {
        let bk1 = boundary_matrix::<Q>(idx, space, k + 1, ell)?;
        snf_rank_with_cross_check(&bk1)?
    };
    let kernel_dim = n_cols_k.saturating_sub(rank_partial_k);
    Ok(kernel_dim.saturating_sub(rank_partial_kplus1))
}

/// Single-prime + cross-check rank recovery over `RANK_RECOVERY_PRIMES`.
/// Returns the rank if the primary and secondary primes agree; falls through
/// to the tertiary if they disagree; errors if all three disagree.
fn snf_rank_with_cross_check<Q: IntegerLikeRig>(m: &MatR<Q>) -> Result<usize, CatgraphError> {
    let r0 = snf_rank_over_zp(m, RANK_RECOVERY_PRIMES[0])?;
    let r1 = snf_rank_over_zp(m, RANK_RECOVERY_PRIMES[1])?;
    if r0 == r1 {
        return Ok(r0);
    }
    // Disagreement: probe the tertiary prime; majority wins.
    let r2 = snf_rank_over_zp(m, RANK_RECOVERY_PRIMES[2])?;
    if r0 == r2 || r1 == r2 {
        return Ok(r2);
    }
    Err(CatgraphError::Composition {
        message: format!(
            "magnitude_homology_rank: all three rank-recovery primes disagree: \
             ranks {r0}, {r1}, {r2} on primes {}, {}, {}; escalate to \
             multi-prime CRT",
            RANK_RECOVERY_PRIMES[0], RANK_RECOVERY_PRIMES[1], RANK_RECOVERY_PRIMES[2],
        ),
    })
}

/// Compute rank of an [`MatR<Q>`](MatR) matrix via SNF over `Z/p`.
///
/// Generic over any [`IntegerLikeRig`] `Q`. Each entry is coerced to `i64`
/// via [`IntegerLikeRig::to_i64`] before reduction modulo `p`.
///
/// # Errors
///
/// - [`CatgraphError::Composition`] if an entry exceeds `i64` range via
///   [`IntegerLikeRig::to_i64`] (shouldn't fire on the boundary
///   matrices, where entries are bounded `±1` or `0`).
/// - Propagates [`CatgraphError`] from [`crate::snf::smith_normal_form`].
///   The internal preconditions (positive prime `p` from `RANK_RECOVERY_PRIMES`,
///   rectangular `a`) hold by construction, so this `Result` is
///   always `Ok` on shipped fixtures; the error path exists to defend against
///   future regressions in `smith_normal_form` (e.g. tightened modulus
///   preconditions, integer-overflow rewrites).
fn snf_rank_over_zp<Q: IntegerLikeRig>(m: &MatR<Q>, p: i64) -> Result<usize, CatgraphError> {
    let rows = m.rows();
    let cols = m.cols();
    if rows == 0 || cols == 0 {
        return Ok(0);
    }
    let a: Vec<Vec<i64>> = (0..rows)
        .map(|i| {
            (0..cols)
                .map(|j| {
                    let rounded = m.entries()[i][j].to_i64()?;
                    Ok(crate::snf::zmod::posmod(rounded, p))
                })
                .collect::<Result<Vec<i64>, CatgraphError>>()
        })
        .collect::<Result<Vec<Vec<i64>>, CatgraphError>>()?;
    let (_u, _v, s) = crate::snf::smith_normal_form(&a, p)?;
    // Rank = count of non-zero diagonal invariants of S. Two-clause filter is
    // defense-in-depth: under the canonicalised `[0, p)` range, `gcd(s, p) != p`
    // is equivalent to `s != 0`, but the redundant `&& s[i][i] != 0` guards
    // against future canonicalisation drift in `smith_normal_form`.
    Ok((0..rows.min(cols))
        .filter(|&i| crate::snf::zmod::gcd_raw(s[i][i], p) != p && s[i][i] != 0)
        .count())
}

/// BV 2025 Prop 3.14 acceptance gate.
///
/// **Mixed parametricity:** the structural rank-recovery path
/// is parameterised over `Q: IntegerLikeRig`, but the numerical path always
/// runs through `crate::magnitude::magnitude::<F64Rig>` — the matrix-inverse
/// Möbius requires `Ring + Div + From<f64>`, strictly narrower than
/// `IntegerLikeRig`. Q-typed numerical computation is deferred
/// (multi-prime CRT). Both invocations of `euler_char_identity_at::<F64Rig>`
/// and `euler_char_identity_at::<Z>` therefore return `(f64, f64)` with the
/// same numerical second component.
///
/// Returns `(via_homology, via_magnitude)`:
///
/// - `via_homology = Σ_ℓ e^(−ℓ) · Σ_k (−1)^k · rank(H_{k,ℓ}(M_t))`
///   computed over the t-pre-scaled space `M_t = scale(M, t)`. The BV 2025
///   Prop 3.14 statement reads `Mag(tM) = Σ_ℓ q^ℓ · Σ_k (−1)^k · rank(H_{k,ℓ}(M))`
///   with `q = e^(−t)`; because the space is pre-scaled by `t`, the weight
///   `q^ℓ_orig = e^(−t · ℓ_orig)` collapses to `e^(−ℓ_scaled)` in our pre-
///   scaled coordinates. (Cross-link to Leinster–Shulman 2017 Theorem 3.5 /
///   Cor 7.15 for the metric-space specialisation directly used here.)
/// - `via_magnitude = Mag(tM)` via
///   [`crate::magnitude::magnitude`] (chain-sum Möbius value when ζ is
///   invertible). **Note:** this is NOT [`crate::mobius_chains::chain_count_signed_graded`]
///   (renamed from `mobius_chains_graded`), which is a per-grade
///   chain-count diagnostic that does not weight by `e^(−ℓ)`; see that
///   function's rustdoc for the reconciliation.
///
/// Both should agree per BV 2025 Prop 3.14, **modulo the geometric
/// truncation residual** at finite `max_degree`. The numerical path is
/// exact via Möbius matrix inverse; the structural path truncates the
/// homology sum at `max_degree`. The exact upper bound on the omitted
/// contribution is `n · r^(max_degree+1) / (1 − r)` where
/// `r = (n − 1) · exp(−d_min_scaled)`; see
/// `tests/euler_char_identity.rs` for the analytical-bound tolerance
/// used by the acceptance suite.
///
/// `space` is the Lawvere metric space `M`; `t` scales it (so the test
/// computes `Mag(tM)`); `max_degree` truncates the inner homology sum
/// `Σ_k (−1)^k · rank(H_{k,ℓ})` at `k ≤ max_degree`. Larger `max_degree`
/// drives the truncation residual toward 0 at cost `O(n^(max_degree+1))`
/// chain enumeration.
///
/// # Performance
///
/// The outer-`k` loop caches `prev_rank = rank(∂_k)` from the previous
/// iteration so each ∂_k is built and SNF'd exactly once per `(k, ℓ)`
/// cell. This bypasses [`magnitude_homology_rank`], which would otherwise
/// rebuild ∂_k twice per outer iteration (once as ∂_k at step `k`, once
/// as ∂_{k+1} at step `k − 1`). ~2× SNF speedup over the naïve loop.
///
/// # Errors
///
/// Returns [`CatgraphError::Composition`] when:
/// - chain enumeration / boundary-matrix construction fails (length-grade
///   tolerance issue),
/// - all three rank-recovery primes disagree on at least one boundary's rank,
/// - the magnitude path's Möbius inversion fails (singular ζ).
///
/// # Example
///
/// ```
/// use catgraph_applied::lawvere_metric::LawvereMetricSpace;
/// use catgraph_applied::rig::F64Rig;
/// use catgraph_magnitude::chain_complex::euler_char_identity_at;
///
/// // 2-point Lawvere space with d(a, b) = 1 for a ≠ b; t = 4, max_degree = 2.
/// let space = LawvereMetricSpace::from_distance_fn(2, |a, b| if a == b { 0.0 } else { 1.0 });
/// let (via_homology, via_magnitude) =
///     euler_char_identity_at::<F64Rig>(&space, 4.0, 2).unwrap();
///
/// // Truncation residual at this fixture is bounded by
/// //   bound = 2 · (e^(-4))^3 / (1 - e^(-4)) ≈ 1.25e-5
/// // and the numerical path is exact, so |Δ| ≤ bound + 1e-9.
/// assert!((via_homology - via_magnitude).abs() < 1e-3);
/// ```
pub fn euler_char_identity_at<Q: IntegerLikeRig>(
    space: &LawvereMetricSpace<NodeId>,
    t: f64,
    max_degree: usize,
) -> Result<(f64, f64), CatgraphError> {
    let scaled = scale_lawvere_space(space, t);
    let idx = ChainIndex::new(&scaled, max_degree);

    // Structural path: Σ_ℓ e^(−ℓ) · Σ_k (−1)^k · rank(H_{k,ℓ}(M_t)).
    //
    // We pre-scale distances by t so the BV 2025 weight `e^(−tℓ)` over the
    // original space M corresponds to `e^(−ℓ)` over the scaled space M_t.
    //
    // Performance: cache `prev_rank = rank(∂_k)` across the outer-`k`
    // iterations so each boundary matrix is built and SNF'd exactly once
    // per `(k, ℓ)` cell. Bypasses `magnitude_homology_rank` (which would
    // rebuild ∂_k both as ∂_k at step `k` and as ∂_{k+1} at step `k − 1`).
    let mut via_hom = 0.0;
    for &ell in idx.grades() {
        let mut alt: i64 = 0;
        // Loop invariant: entering iteration `k`, `prev_rank == rank(∂_k)`.
        // Seed: `rank(∂_0) = 0` by convention (∂_0 has empty domain).
        // Update: at end of iteration `k`, `prev_rank = rank(∂_{k+1})`, so
        // it correctly holds `rank(∂_k)` for the next iteration.
        let mut prev_rank: usize = 0;
        for k in 0..=max_degree {
            let n_cols_k = idx.chains_at(k, ell).len();
            // Empty-bucket short-circuit: when no degree-`k` chains live at
            // grade `ell`, both `kernel_dim` and `rank(∂_{k+1})` are forced
            // to 0 (chains-at-(k+1, ell) requires a degree-k face), so we
            // skip the boundary build + SNF entirely. Reset `prev_rank` to
            // 0 to match `rank(∂_{k+1}) = 0` for the next iteration.
            if n_cols_k == 0 {
                prev_rank = 0;
                continue;
            }
            let bk1 = boundary_matrix::<Q>(&idx, &scaled, k + 1, ell)?;
            let rank_kplus1 = snf_rank_with_cross_check(&bk1)?;
            let kernel_dim = n_cols_k.saturating_sub(prev_rank);
            let h_rank = kernel_dim.saturating_sub(rank_kplus1);
            #[allow(
                clippy::cast_possible_wrap,
                reason = "ranks bounded by chain count (≤ n·(n−1)^max_degree); never approach i64::MAX"
            )]
            let h_rank_signed = h_rank as i64;
            alt += if k % 2 == 0 {
                h_rank_signed
            } else {
                -h_rank_signed
            };
            prev_rank = rank_kplus1;
        }
        #[allow(
            clippy::cast_precision_loss,
            reason = "alternating-rank sum is small in practice; f64 has 53-bit mantissa"
        )]
        let alt_f = alt as f64;
        via_hom += (-ell).exp() * alt_f;
    }

    // Numerical path: `magnitude::magnitude`. Pass `t = 1.0`
    // because the space is already pre-scaled by t. Stays `F64Rig`-mono here
    // because the matrix-inverse Möbius path requires `Ring + Div + From<f64>`,
    // a strictly narrower bound than `IntegerLikeRig` (the rig widening only
    // touches the SNF-mod-p rank-recovery interior, not the numerical
    // magnitude path). Parameterizing the numerical path over `Q` is a
    // deferred multi-prime CRT prerequisite.
    let via_mag = crate::magnitude::magnitude::<F64Rig>(&scaled, 1.0)
        .map(|q| q.0)
        .map_err(|e| CatgraphError::Composition {
            message: format!("euler_char_identity_at: magnitude path failed: {e:?}"),
        })?;

    Ok((via_hom, via_mag))
}

/// Build a fresh `LawvereMetricSpace` whose distances are `t` times those of
/// `space`. Private helper used by [`euler_char_identity_at`] to feed both
/// paths a pre-scaled space.
///
/// Infallible: positive scaling of a Lawvere metric preserves the
/// non-negativity and triangle-inequality axioms verbatim.
fn scale_lawvere_space(space: &LawvereMetricSpace<NodeId>, t: f64) -> LawvereMetricSpace<NodeId> {
    let n = space.size();
    LawvereMetricSpace::from_distance_fn(n, |a, b| space.distance(&a, &b).0 * t)
}
