//! Chain-sum Möbius inversion via Leinster 2013 Prop 2.1.3.
//!
//! Computes
//! `μ_A(a, b) = Σ_{k≥0} (−1)ᵏ · Σ_{a=a₀≠…≠a_k=b} ζ_A(a₀,a₁) · ζ_A(a₁,a₂) · … · ζ_A(a_{k−1},a_k)`
//! by accumulating matrix powers of `M = ζ − I` (the off-diagonal part of
//! ζ). The k = 0 term contributes the identity matrix `I` (empty chains
//! at `a == b`). Subsequent terms `(−1)ᵏ Mᵏ` exactly equal the per-entry
//! chain-product sum of length-`k` simple chains (the diagonal of `M` is
//! `0`, so any `Mᵏ[a][b]` automatically satisfies the
//! `a_{j-1} ≠ a_j` simple-chain constraint).
//!
//! ## Algebraic identity (von Neumann series)
//!
//! Per Leinster 2013 Prop 2.1.3 and the geometric-series identity for matrix
//! inverses:
//!
//! ```text
//! μ = ζ⁻¹ = (I + M)⁻¹ = Σ_{k=0}^∞ (−1)ᵏ Mᵏ
//! ```
//!
//! with `M = ζ − I`. Scatteredness (Def 2.1.2: `d(a, b) > log(#A − 1)`) gives
//! the per-entry geometric bound `|μ_{A,k}(a,b)| ≤ ((n − 1) · e^(−ε))ᵏ`
//! (Prop 2.1.3 proof, page 11) with `ε = min_{a≠b} d(a, b)`, equivalently
//! `‖M‖_∞ ≤ (n − 1) · e^(−ε) < 1`, so the Neumann series converges absolutely.
//! The chain-sum formula and the matrix-inversion path
//! [`crate::magnitude::mobius_function`] are algebraically identical; matrix
//! inversion is `O(n³)` once, matrix-power accumulation `O(K · n³)`.
//!
//! ## Convergence
//!
//! The partial sum `Σ_{k=0}^K (−1)ᵏ Mᵏ` differs from `μ` by at most
//! `rᴷ⁺¹ / (1 − r)` per entry with `r = (n − 1) · e^(−ε)`. The truncation depth
//! is `K = min(⌈log(τ) / log(r)⌉, K_MAX)` with `τ = 1e-13` and `K_MAX = 200`.
//! On spaces with `r ≥ 0.94`, [`mobius_function_via_chains`] returns `Err`
//! rather than truncating; [`crate::magnitude::mobius_function`] inverts ζ
//! directly and has no such cap.

use std::hash::Hash;

use catgraph::errors::CatgraphError;

use crate::magnitude::{is_scattered, materialize_objects};
use crate::poset_category::PosetCategory;
use crate::weighted_cospan::NodeId;
use crate::{LawvereMetricSpace, Rig, Ring};
use catgraph_applied::ZAlgebra;
use catgraph_applied::mat::MatR;

/// Tolerance for the geometric-tail residual; truncation depth is chosen so
/// that the partial-sum error per entry is below this threshold.
const CHAIN_SUM_RESIDUAL_TOL: f64 = 1e-13;

/// Hard cap on truncation depth; declines spaces with geometric ratio
/// `r > 0.94` (which would need K > 200 to reach the residual tolerance).
const CHAIN_SUM_MAX_DEPTH: usize = 200;

/// Möbius matrix via Leinster 2013 Prop 2.1.3 chain-sum formula.
///
/// `μ_A(a, b) = Σ_{k=0}^∞ (−1)ᵏ · Σ_{a=a₀≠…≠a_k=b} ζ_A(a₀,a₁) · … · ζ_A(a_{k−1},a_k)`
///
/// where `ζ_A(x, y) = exp(−d(x, y))` is the Lawvere-metric similarity.
/// Implemented via von-Neumann-series matrix-power accumulation
/// `μ = Σ_{k=0}^K (−1)ᵏ Mᵏ` with `M = ζ − I` and `K` chosen so the
/// geometric-tail residual is below `CHAIN_SUM_RESIDUAL_TOL`.
///
/// The bound `Q: Ring + From<f64>` is weaker than
/// [`crate::magnitude::mobius_function`]'s `Q: Ring + Div + From<f64>` —
/// matrix-power accumulation inverts nothing. The `is_zero()` short-circuit in
/// `matmul` and the `r == 0.0` early return assume `Q::is_zero()` matches
/// `f64 == 0.0`, which [`crate::F64Rig`] provides and `Tropical` (rig zero
/// `+∞`) does not.
///
/// The input must be scattered ([`is_scattered`] returns `true`). On a
/// scattered space this and [`crate::magnitude::mobius_function`] return the
/// same matrix to `1e-9` relative on well-separated fixtures and `1e-8`
/// relative at the scatteredness boundary (`tests/canonical.rs`).
///
/// # Errors
///
/// - `CatgraphError::Composition { message: "space is not scattered: ..." }`
///   when [`is_scattered`] returns `false`. Chain-sum has no convergence
///   guarantee; **caller fallback:** [`crate::magnitude::mobius_function`]
///   (which requires invertible ζ but does not require scatteredness).
/// - `CatgraphError::Composition { message: "near-boundary scattered ..." }`
///   when the geometric ratio `r ≥ 0.94`, requiring more than
///   `CHAIN_SUM_MAX_DEPTH` truncation steps to reach numerical tolerance.
///   Indicates the space is scattered but close to the boundary; **caller
///   fallback:** [`crate::magnitude::mobius_function`] (which inverts ζ
///   directly without truncation).
pub fn mobius_function_via_chains<Q>(
    space: &LawvereMetricSpace<NodeId>,
) -> Result<MatR<Q>, CatgraphError>
where
    Q: Ring + From<f64>,
{
    if !is_scattered(space) {
        return Err(CatgraphError::Composition {
            message: "space is not scattered (Leinster 2013 Def 2.1.2 requires \
                      d(a,b) > log(#A−1) for all distinct a,b); chain-sum Möbius \
                      has no convergence guarantee — fall back to \
                      magnitude::mobius_function::<Q>"
                .to_string(),
        });
    }

    let objects: Vec<NodeId> = materialize_objects(space);
    let n = objects.len();

    if n == 0 {
        return MatR::new(0, 0, Vec::new());
    }
    if n == 1 {
        // Single point: ζ = (1), μ = (1).
        return MatR::new(1, 1, vec![vec![Q::one()]]);
    }

    // Determine the geometric ratio r = (n − 1) · e^(−ε) where ε is the
    // smallest off-diagonal Lawvere distance. Used to compute truncation
    // depth K so the tail residual r^(K+1) / (1 − r) is below tolerance.
    let mut min_off_diagonal = f64::INFINITY;
    for (i, a) in objects.iter().enumerate() {
        for (j, b) in objects.iter().enumerate() {
            if i != j {
                let d = space.distance(a, b).0;
                if d < min_off_diagonal {
                    min_off_diagonal = d;
                }
            }
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let r = ((n - 1) as f64) * (-min_off_diagonal).exp();
    if r >= 0.94 {
        return Err(CatgraphError::Composition {
            message: format!(
                "near-boundary scattered space (geometric ratio r = {r:.4} \
                 ≥ 0.94 would require >200 truncation depth to reach 1e-13 \
                 residual); use magnitude::mobius_function::<Q> instead, \
                 which inverts ζ directly without truncation"
            ),
        });
    }
    // K = ⌈log(τ) / log(r)⌉, capped at CHAIN_SUM_MAX_DEPTH. At r == 0 (discrete
    // topology: every off-diagonal d = +∞, so M = 0) one no-op iteration leaves
    // μ = I, which is the Lemma 1.1.4 answer on a discrete space.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let k_required = if r > 0.0 {
        (CHAIN_SUM_RESIDUAL_TOL.ln() / r.ln()).ceil() as usize
    } else {
        1
    };
    let max_k = k_required.min(CHAIN_SUM_MAX_DEPTH);

    // Build M = ζ − I (off-diagonal part of ζ).
    let m: Vec<Vec<Q>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| {
                    if i == j {
                        Q::zero()
                    } else {
                        let d = space.distance(&objects[i], &objects[j]);
                        Q::from((-d.0).exp())
                    }
                })
                .collect()
        })
        .collect();

    // μ accumulator starts at the k = 0 term: identity matrix.
    let mut mu: Vec<Vec<Q>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| if i == j { Q::one() } else { Q::zero() })
                .collect()
        })
        .collect();

    // Running M^k. Initially M^1 = M (we start the loop at k = 1).
    let mut m_k = m.clone();
    let mut sign_positive = false; // k = 1 is negative sign

    for k in 1..=max_k {
        // Add (−1)^k · m_k to mu.
        for i in 0..n {
            for j in 0..n {
                let term = m_k[i][j].clone();
                let signed = if sign_positive { term } else { -term };
                let cur = mu[i][j].clone();
                mu[i][j] = cur + signed;
            }
        }
        if k == max_k {
            break;
        }
        m_k = matmul::<Q>(&m_k, &m, n);
        sign_positive = !sign_positive;
    }

    MatR::new(n, n, mu)
}

/// Naive O(n³) matrix multiplication on row-major `Vec<Vec<Q>>`.
fn matmul<Q>(a: &[Vec<Q>], b: &[Vec<Q>], n: usize) -> Vec<Vec<Q>>
where
    Q: Ring + From<f64>,
{
    let mut out: Vec<Vec<Q>> = vec![vec![Q::zero(); n]; n];
    for i in 0..n {
        for k in 0..n {
            let a_ik = a[i][k].clone();
            if a_ik.is_zero() {
                continue;
            }
            for j in 0..n {
                let prod = a_ik.clone() * b[k][j].clone();
                let cur = out[i][j].clone();
                out[i][j] = cur + prod;
            }
        }
    }
    out
}

// ============================================================================
// Length-graded chain-sum (per-grade chain-count diagnostic).
// ============================================================================

/// Per-grade signed chain counts: returns `Vec<(ℓ, partial_sum_at_ell)>`
/// where each `partial_sum_at_ell = Σ_k (-1)^k · |chains at (k, ℓ)|` is the
/// **entry-sum** of `(-1)^k Mᵏ` restricted to the length-`ℓ` chain bucket
/// (the un-weighted variant; see "Acceptance-gate relationship" below).
///
/// # Acceptance-gate relationship
///
/// This is not the numerical path of the BV 2025 Prop 3.14 acceptance gate:
/// [`crate::chain_complex::euler_char_identity_at`] compares against
/// [`crate::magnitude::magnitude`], the matrix-inverse Möbius. Recovering the
/// Prop 3.14 numerical RHS from the per-grade `partial_sum_at_ell` here needs
/// the `q^ℓ = e^(−tℓ)` weighting, `Mag(tM) ?= Σ_ℓ e^(−tℓ) · partial_sum_at_ell`.
///
/// # Algebraic context
///
/// Per Leinster–Shulman 2017 §2, simple chains `(x₀, …, x_k)` with grade
/// `ℓ = Σ d(x_{j−1}, x_j)` form `C_{k,ℓ}(M)`. The sign pattern `(-1)^k` is the
/// same as the von-Neumann series for `μ = (I + M)⁻¹` (Leinster 2013
/// Prop 2.1.3); on a scattered space the per-grade contributions sum to the
/// entry-sum of the Möbius matrix `μ` at each grade (un-weighted).
///
/// The bound `Q: Rig + From<f64>` is weaker than
/// [`mobius_function_via_chains`]'s `Q: Ring + From<f64>`: the `(-1)^k` sign is
/// applied by pre-negating the count in `f64` before lifting via `Q::from(_)`,
/// so `Q` needs no additive inverses. The result is meaningful only for rigs
/// whose `From<f64>` honors negative values — `F64Rig`, not `BoolRig`,
/// `UnitInterval` or `Tropical`.
///
/// # Errors
///
/// Returns [`CatgraphError`] if chain enumeration fails.
pub fn chain_count_signed_graded<Q>(
    space: &LawvereMetricSpace<NodeId>,
    max_chain_length: usize,
) -> Result<Vec<(f64, Q)>, CatgraphError>
where
    Q: Rig + From<f64>,
{
    let idx = crate::chain_complex::ChainIndex::new(space, max_chain_length);
    let mut out = Vec::new();
    for &ell in idx.grades() {
        let mut acc = Q::zero();
        for k in 0..=max_chain_length {
            let chains = idx.chains_at(k, ell);
            // Skip empty (k, ell) buckets to avoid an unnecessary `acc.clone()`
            // on a zero-add. Most (k, ell) buckets are empty for sparse fixtures.
            if chains.is_empty() {
                continue;
            }
            // (-1)^k · |chains| → entry-sum of (-1)^k Mᵏ restricted to grade ℓ.
            // Pre-negate in f64 (the rig embedding) so the `Rig` bound suffices
            // — no `Neg` on `Q` needed; see bound discussion in the doc above.
            #[allow(clippy::cast_precision_loss)]
            let count = chains.len() as f64;
            let signed = if k % 2 == 0 {
                Q::from(count)
            } else {
                Q::from(-count)
            };
            acc = acc.clone() + signed;
        }
        out.push((ell, acc));
    }
    Ok(out)
}

// ============================================================================
// Integer-exact Möbius via Leinster 2008 Cor 1.5 chain sum.
// ============================================================================

/// Integer-exact Möbius inversion via Leinster 2008 Cor 1.5.
///
/// Paper anchor: Leinster, *The Euler characteristic of a category*
/// (arXiv:math/0610260v1, 2008): Cor 1.5 (page 6) for the integer Möbius
/// formula. The nilpotency/termination bound is implicit in Cor 1.5 +
/// circuit-freeness: the nondegenerate-path count vanishes for `n ≥ |𝔸|`
/// on circuit-free 𝔸 (no separate proposition needed).
///
/// For a finite skeletal category 𝔸 whose only endomorphisms are identities
/// (equivalently (Leinster 2008 Lemma 1.3, p. 5): the non-identity arrow graph is circuit-free), the
/// Möbius function takes **integer** values
///
/// ```text
/// μ(a, b) = Σ_{n≥0} (−1)ⁿ · #{nondegenerate n-paths from a to b}.
/// ```
///
/// This function realises that formula as the alternating-sign matrix-power
/// accumulation
///
/// ```text
/// μ = Σ_{k=0}^K (−1)ᵏ Mᵏ,   where  M = ζ − I  (off-diagonal part of ζ).
/// ```
///
/// Off-diagonal `M[i][j]` counts arrows `i → j` (so `Mᵏ[i][j]` is the
/// number of non-degenerate length-`k` paths from `i` to `j` exactly). Per
/// Cor 1.5's implicit termination (the nondegenerate-path count vanishes for
/// `k ≥ |𝔸|` on circuit-free 𝔸), the series terminates by `k = |objects|` because `M` is
/// nilpotent on circuit-free 𝔸; this implementation **early-terminates** as
/// soon as `Mᵏ` becomes the zero matrix, which is often well before `n`.
///
/// # Bounds
///
/// `Q: Ring + ZAlgebra`. [`ZAlgebra`] extends [`Ring`] with `Neg + Sub` and
/// the `Q::from_i64` lifting constructor; `Z(BigInt)` from `catgraph-applied`
/// is the canonical instance.
///
/// # Errors
///
/// Returns [`CatgraphError::Composition`] if [`MatR::new`] rejects the final
/// matrix, or if a `ζ` arrow count exceeds `i64::MAX`: counts from
/// [`PosetCategory::zeta_matrix`] are `u64` and cross into `i64` for
/// [`ZAlgebra::from_i64`], and an over-large count returns an error naming the
/// offending entry rather than wrapping negative.
///
/// # Examples
///
/// ```
/// use catgraph_applied::z::Z;
/// use catgraph_magnitude::poset_category::PosetCategory;
/// use catgraph_magnitude::mobius_chains::mobius_function_via_chains_exact;
///
/// // 3-chain 0 ≤ 1 ≤ 2. Phil Hall μ has -1 on the immediate-cover band.
/// let cat = PosetCategory::<u32>::from_partial_order(vec![0, 1, 2], |a, b| a <= b);
/// let mu = mobius_function_via_chains_exact::<u32, Z>(&cat).unwrap();
/// assert_eq!(mu.entries()[0][1], Z::from(-1_i64));
/// assert_eq!(mu.entries()[0][2], Z::from(0_i64));
/// ```
pub fn mobius_function_via_chains_exact<N, Q>(
    cat: &PosetCategory<N>,
) -> Result<MatR<Q>, CatgraphError>
where
    N: Clone + Eq + Hash,
    Q: Ring + ZAlgebra,
{
    let n = cat.size();
    if n == 0 {
        return Ok(MatR::new(0, 0, Vec::new())
            .expect("MatR::new on shape-correct construction (n=0 short-circuit)"));
    }
    if n == 1 {
        return Ok(MatR::new(1, 1, vec![vec![Q::one()]])
            .expect("MatR::new on shape-correct construction (n=1 short-circuit)"));
    }

    // Step 1. Lift ζ (Vec<Vec<u64>>) to M (Vec<Vec<Q>>) with the diagonal
    // zeroed out — M is the off-diagonal arrow-count part of ζ.
    let zeta = cat.zeta_matrix();
    let m: Vec<Vec<Q>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| {
                    if i == j {
                        Ok(Q::zero())
                    } else {
                        zeta_entry_to_q::<Q>(zeta[i][j], i, j)
                    }
                })
                .collect::<Result<Vec<Q>, CatgraphError>>()
        })
        .collect::<Result<Vec<Vec<Q>>, CatgraphError>>()?;

    // Step 2. μ accumulator starts at the k = 0 term: the identity matrix.
    let mut mu: Vec<Vec<Q>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| if i == j { Q::one() } else { Q::zero() })
                .collect()
        })
        .collect();

    // Step 3. Running power M^k; initially M^1 = M for the k = 1 iteration.
    let mut m_k = m.clone();
    let mut sign_positive = false; // k = 1 starts at negative sign

    // Step 4. Accumulate (−1)^k M^k for k = 1..=n; per Cor 1.5's implicit termination
    // (nondegenerate-path count vanishes for k ≥ |𝔸| on circuit-free 𝔸) the loop is
    // guaranteed to early-terminate at most by k = n.
    for _k in 1..=n {
        let mut all_zero = true;
        for i in 0..n {
            for j in 0..n {
                let term = m_k[i][j].clone();
                if !term.is_zero() {
                    all_zero = false;
                }
                let signed = if sign_positive { term } else { -term };
                let cur = mu[i][j].clone();
                mu[i][j] = cur + signed;
            }
        }
        if all_zero {
            // M^k = 0 ⇒ all subsequent terms vanish; series is exact.
            break;
        }
        // Update M^k ← M^k · M for the next iteration.
        m_k = matmul_q(&m_k, &m, n);
        sign_positive = !sign_positive;
    }

    // Step 5. Wrap as MatR<Q>.
    MatR::new(n, n, mu)
}

/// Verify the Möbius recursion `μ · ζ = I` **and** `ζ · μ = I` over `Q`.
///
/// Bidirectional check: per Leinster 2008 Def 1.1 (p. 4), in the
/// finite-dimensional incidence algebra `R(𝔸)` a one-sided inverse implies
/// a two-sided inverse ("by finite-dimensionality, either one implies the
/// other") — so either direction alone is algebraically sufficient. This
/// verifier nevertheless checks both `μ · ζ = I` (right inverse) and
/// `ζ · μ = I` (left inverse).
///
/// # Errors
///
/// Returns [`CatgraphError::Composition`] when either `(μ · ζ)[i][j]` or
/// `(ζ · μ)[i][j]` differs from the Kronecker delta `δᵢⱼ` at any `(i, j)`.
/// The error message names the direction (right or left inverse), the first
/// failing index, and the expected vs actual entry values. The same variant
/// reports a `ζ` arrow count above `i64::MAX`.
///
/// # Examples
///
/// ```
/// use catgraph_applied::z::Z;
/// use catgraph_magnitude::poset_category::PosetCategory;
/// use catgraph_magnitude::mobius_chains::{
///     mobius_function_via_chains_exact, verify_mobius_recursion,
/// };
///
/// let cat = PosetCategory::<u32>::from_partial_order(vec![0, 1, 2], |a, b| a <= b);
/// let mu = mobius_function_via_chains_exact::<u32, Z>(&cat).unwrap();
/// verify_mobius_recursion(&cat, &mu).expect("μ · ζ = ζ · μ = I on the 3-chain");
/// ```
pub fn verify_mobius_recursion<N, Q>(
    cat: &PosetCategory<N>,
    mu: &MatR<Q>,
) -> Result<(), CatgraphError>
where
    N: Clone + Eq + Hash,
    Q: Ring + ZAlgebra + std::fmt::Debug,
{
    let n = cat.size();
    let zeta = cat.zeta_matrix();
    let zeta_q: Vec<Vec<Q>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| zeta_entry_to_q::<Q>(zeta[i][j], i, j))
                .collect::<Result<Vec<Q>, CatgraphError>>()
        })
        .collect::<Result<Vec<Vec<Q>>, CatgraphError>>()?;

    // Right inverse: μ · ζ = I (upper-triangular recursion, Leinster 2008
    // Def 1.1).
    let mu_zeta = matmul_q(mu.entries(), &zeta_q, n);
    for (i, row) in mu_zeta.iter().enumerate() {
        for (j, entry) in row.iter().enumerate() {
            let expected = if i == j { Q::one() } else { Q::zero() };
            if *entry != expected {
                return Err(CatgraphError::Composition {
                    message: format!(
                        "μ · ζ ≠ I at ({i}, {j}) [right inverse]: \
                         expected {expected:?}, got {entry:?}"
                    ),
                });
            }
        }
    }

    // Left inverse: ζ · μ = I (lower-triangular recursion).
    let zeta_mu = matmul_q(&zeta_q, mu.entries(), n);
    for (i, row) in zeta_mu.iter().enumerate() {
        for (j, entry) in row.iter().enumerate() {
            let expected = if i == j { Q::one() } else { Q::zero() };
            if *entry != expected {
                return Err(CatgraphError::Composition {
                    message: format!(
                        "ζ · μ ≠ I at ({i}, {j}) [left inverse]: \
                         expected {expected:?}, got {entry:?}"
                    ),
                });
            }
        }
    }

    Ok(())
}

/// Lift one `ζ` arrow count into `Q` via the `ℤ → Q` ring homomorphism
/// [`ZAlgebra::from_i64`].
///
/// [`PosetCategory::zeta_matrix`] counts arrows as `u64`, so the count crosses
/// into `i64`; a count above `i64::MAX` is rejected rather than wrapped
/// negative.
///
/// # Errors
///
/// Returns [`CatgraphError::Composition`] naming the `(i, j)` position and the
/// offending count when it exceeds `i64::MAX`.
fn zeta_entry_to_q<Q>(count: u64, i: usize, j: usize) -> Result<Q, CatgraphError>
where
    Q: ZAlgebra,
{
    let signed = i64::try_from(count).map_err(|_| CatgraphError::Composition {
        message: format!(
            "ζ arrow count {count} at ({i}, {j}) exceeds i64::MAX ({}); \
             the ℤ → Q lift via ZAlgebra::from_i64 cannot represent it",
            i64::MAX
        ),
    })?;
    Ok(Q::from_i64(signed))
}

/// O(n³) generic matrix multiplication on row-major `Vec<Vec<Q>>`.
///
/// Skips the inner-product accumulation for any zero `a[i][k]` entry.
fn matmul_q<Q>(a: &[Vec<Q>], b: &[Vec<Q>], n: usize) -> Vec<Vec<Q>>
where
    Q: Ring + Clone,
{
    let mut out: Vec<Vec<Q>> = vec![vec![Q::zero(); n]; n];
    for i in 0..n {
        for k in 0..n {
            let a_ik = a[i][k].clone();
            if a_ik.is_zero() {
                continue;
            }
            for j in 0..n {
                let prod = a_ik.clone() * b[k][j].clone();
                let cur = out[i][j].clone();
                out[i][j] = cur + prod;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use catgraph_applied::z::Z;

    // The `i64::MAX` rejection branch is unreachable through the public
    // entry points with any constructible fixture: it needs a single ζ entry
    // counting more than 2⁶³ arrows between one pair of objects, which no
    // in-memory `PosetCategory` can hold. The lift is therefore tested at the
    // helper directly rather than by contorting the public API into producing
    // an impossible poset.
    #[test]
    fn zeta_entry_to_q_lifts_small_counts() {
        assert_eq!(zeta_entry_to_q::<Z>(3, 0, 1).unwrap(), Z::from(3_i64));
        assert_eq!(zeta_entry_to_q::<Z>(0, 2, 2).unwrap(), Z::from(0_i64));
    }

    #[test]
    fn zeta_entry_to_q_rejects_counts_above_i64_max() {
        let too_big = u64::try_from(i64::MAX).expect("i64::MAX is non-negative") + 1;
        let err = zeta_entry_to_q::<Z>(too_big, 4, 7)
            .expect_err("a count above i64::MAX must be rejected, not wrapped");
        let message = err.to_string();
        assert!(
            message.contains("(4, 7)"),
            "message names the entry: {message}"
        );
        assert!(
            message.contains("exceeds i64::MAX"),
            "message names the bound: {message}"
        );
    }
}
