//! Magnitude functions — Phases 6A.2 + 6A.3 + 6F.
//!
//! - [`tsallis_entropy`] — Tsallis q-entropy with Shannon-recovery special
//!   case at `t = 1` (BV 2025 §3 / Tsallis 1988).
//! - [`mobius_function`] — Möbius inversion `ζ · μ = I` over a ring (Leinster
//!   2013 / Leinster-Shulman §2). v0.1.0 implements the matrix-inverse path
//!   via Gaussian elimination, requiring `Q: Ring + Div + From<f64>`.
//! - [`magnitude`] — magnitude `Mag(tM) = Σᵢⱼ μ_t[i][j]` of a Lawvere metric
//!   space at scale `t`, computed by Möbius-inverting the t-scaled zeta
//!   matrix and summing all entries (BV 2025 §3.5, Eq 7).
//! - [`weighting`] / [`coweighting`] — paper-foundational (co)weighting
//!   primitives (Leinster 2013 §1.1 Def 1.1.1). v0.2.0 addition.
//! - [`is_scattered`] — Leinster Def 2.1.2 predicate `d(a,b) > log(#A−1)`,
//!   the convergence precondition for the chain-sum Möbius formula in
//!   [`crate::mobius_chains`]. v0.2.0 addition.
//!
//! ## Module-level lints
//!
//! All Gaussian-elimination loops in this module index into BOTH `aug[col]`
//! (read pivot row) AND `aug[r]` (write target row) inside the inner loop.
//! `clippy::needless_range_loop` would suggest iterating-by-value, but we
//! need indexed writes back into `aug[r][k]`, so the index `k` is the
//! primary loop variable, not just a counter. Module-level `#![allow]` below
//! removes 6 per-site duplicates (v0.2.1 reviewer #3 Minor #6).
#![allow(clippy::needless_range_loop)]

use std::ops::Div;

use catgraph::errors::CatgraphError;

use crate::weighted_cospan::NodeId;
use crate::{LawvereMetricSpace, Ring, TSALLIS_SHANNON_EPS};
use catgraph_applied::mat::MatR;

/// Materialize the object list of a Lawvere metric space as `Vec<NodeId>`.
///
/// Private helper. Replaces the verbose
/// `<LawvereMetricSpace<NodeId> as crate::EnrichedCategory<crate::Tropical>>::objects(space).collect()`
/// FQN dispatch repeated across `mobius_function`, `magnitude`, `weighting`,
/// `coweighting`, `is_scattered`, and `mobius_chains::mobius_function_via_chains`.
/// Added in v0.2.1 per the post-Phase-6F three-reviewer pass.
pub(crate) fn materialize_objects(space: &LawvereMetricSpace<NodeId>) -> Vec<NodeId> {
    <LawvereMetricSpace<NodeId> as crate::EnrichedCategory<crate::Tropical>>::objects(space)
        .collect()
}

/// Tsallis q-entropy `H_t(p) = (1 − Σ pᵢᵗ) / (t − 1)` for `t ≠ 1`.
///
/// At `t = 1` the limit is Shannon entropy `H₁(p) = -Σ pᵢ ln pᵢ`. Tsallis
/// 1988 / Havrda-Charvát 1967 introduce the parametric family; BV 2025 §3
/// uses it as the per-state language-model entropy in the closed-form
/// magnitude expression of Thm 3.10.
///
/// **Shannon special case.** When `|t − 1| < TSALLIS_SHANNON_EPS` (= `1e-6`),
/// the function returns `-Σ pᵢ ln pᵢ` directly to avoid catastrophic
/// cancellation in the `(1 − Σ pᵢᵗ) / (t − 1) ≈ 0/0` regime. Per Phase 6A
/// execution plan amend 5: the Cor 3.14 finite-difference step `h` MUST
/// satisfy `h > TSALLIS_SHANNON_EPS`; otherwise both `f(1+h)` and `f(1−h)`
/// evaluate the Shannon branch and the central difference collapses
/// identically to zero.
///
/// **Conventions.**
/// - Shannon branch: `0 · ln 0 = 0` by limit (terms with `pᵢ = 0` are skipped).
/// - Tsallis branch: `0^t = 0` for `t > 0`; `f64::powf` already returns `0.0`
///   for `0.0_f64.powf(t)` when `t > 0`, so zero-probability terms contribute
///   `0` to the sum without special handling.
/// - The function does NOT validate `Σ pᵢ = 1` — callers requiring a true
///   probability distribution must normalize beforehand. This keeps the
///   function compatible with random-vector proptest fixtures.
///
/// # Precondition
///
/// `t > 0` per BV 2025 Prop 3.6 (and per Tsallis 1988 §2 — the Tsallis
/// q-entropy family is defined for `q > 0`). At `t < 0`, `0.0_f64.powf(t)`
/// returns `+∞` and propagates to the sum, polluting the result with
/// non-finite values; at `t = 0`, the Tsallis formula degenerates to
/// `(1 − n) / (−1)` for any non-zero distribution. v0.2.1 adds a
/// `debug_assert!(t > 0.0)` entry guard mirroring `LmCategory::magnitude`'s
/// v0.1.1 documentary check (per H.3 verdict #4 and v0.2.0 reviewer #3 I-2).
/// Callers operating in release mode with `t ≤ 0` get the documented NaN /
/// `+∞` pollution; the function does not return `Result` to keep the hot path
/// branch-free.
///
/// # Returns
///
/// `f64::NAN` only if `p` contains a NaN entry (propagates through `ln` and
/// `powf`), or `+∞` if `t < 0` and `p` contains a zero entry. Otherwise a
/// finite value.
#[inline]
#[must_use]
pub fn tsallis_entropy(p: &[f64], t: f64) -> f64 {
    debug_assert!(
        t > 0.0,
        "tsallis_entropy precondition violated: t = {t} ≤ 0; \
                  see BV 2025 Prop 3.6 / Tsallis 1988 §2"
    );
    if (t - 1.0).abs() < TSALLIS_SHANNON_EPS {
        // Shannon branch: H₁(p) = -Σ pᵢ ln pᵢ, with `0 · ln 0 = 0` by limit.
        let mut sum = 0.0;
        for &pi in p {
            if pi > 0.0 {
                sum -= pi * pi.ln();
            }
        }
        sum
    } else {
        // Tsallis branch: H_t(p) = (1 − Σ pᵢᵗ) / (t − 1).
        // `0.0_f64.powf(t)` is `0.0` for `t > 0`; for the unusual `t < 0`
        // case, callers are responsible for excluding zero-probability terms.
        let sum_pow: f64 = p.iter().map(|&pi| pi.powf(t)).sum();
        (1.0 - sum_pow) / (t - 1.0)
    }
}

/// Möbius function of an enriched category, returned as an `n × n` matrix
/// of shape over `Q`, where `n = space.objects().count()`.
///
/// Per Leinster 2013 / Leinster-Shulman §2, the Möbius function `μ` is the
/// inverse of the zeta matrix `ζ` defined entrywise by
/// `ζ[i][j] = exp(-d(objects[i], objects[j]))` embedded into `Q` via
/// `Q::from(_: f64)`. Here `d` is the Lawvere distance carried by `space`.
///
/// **Bound: `Q: Ring + Div + From<f64>` — i.e. `Q` is a (commutative) field
/// for v0.1.0.** Gaussian elimination needs additive inverses (the `Ring`
/// bound, supplied by `Neg + Sub`) AND multiplicative inverses (the `Div`
/// bound, supplied by `Q / Q → Q`). Among the workspace's four concrete
/// rigs only [`crate::F64Rig`] satisfies all three; [`crate::BoolRig`],
/// [`crate::UnitInterval`], and [`crate::Tropical`] are excluded. The
/// chain-sum variant `mobius_function_via_chains<Q: Rig>` per Leinster-
/// Shulman's explicit formula is deferred to v0.2.0 — see crate root docs.
///
/// **Conversion `f64 → Q`.** The zeta matrix entries `exp(-d(i, j))` are
/// computed in `f64` then converted to `Q` via `Q::from(_)`. v0.1.0's only
/// `Ring + Div`-satisfying rig is `F64Rig`, which has the conversion
/// trivially.
///
/// # Errors
///
/// Returns [`CatgraphError::Composition`] when zeta is singular — i.e. when
/// Gaussian elimination cannot find a non-zero pivot in some column. No
/// Möbius function exists for that enriched category.
///
/// # Panics
///
/// Does not panic. Singular zeta returns `Err`; the implementation never
/// indexes out of bounds (matrix is `n × 2n` augmented and indices are
/// always `< n` or `< 2n` by construction).
pub fn mobius_function<Q>(space: &LawvereMetricSpace<NodeId>) -> Result<MatR<Q>, CatgraphError>
where
    Q: Ring + Div<Output = Q> + From<f64>,
{
    let objects: Vec<NodeId> = materialize_objects(space);
    let n = objects.len();

    if n == 0 {
        // Empty category — Möbius function is the 0×0 matrix.
        return MatR::new(0, 0, Vec::new());
    }

    // Build the n × 2n augmented matrix [ζ | I] in `Vec<Vec<Q>>`. We do not
    // use `MatR` here because Gaussian elimination needs in-place row swaps
    // and arithmetic on individual entries — operations the immutable
    // `MatR` API does not expose.
    let mut aug: Vec<Vec<Q>> = (0..n)
        .map(|i| {
            let mut row: Vec<Q> = Vec::with_capacity(2 * n);
            // Left half: zeta[i][j] = exp(-d(objects[i], objects[j])).
            // Tropical(+∞) (unset distance) ⇒ exp(-∞) = 0; Tropical(0) ⇒
            // exp(0) = 1. f64::exp handles both correctly.
            for j in 0..n {
                let d = space.distance(&objects[i], &objects[j]);
                let zeta_ij: f64 = (-d.0).exp();
                row.push(Q::from(zeta_ij));
            }
            // Right half: identity.
            for j in 0..n {
                if i == j {
                    row.push(Q::one());
                } else {
                    row.push(Q::zero());
                }
            }
            row
        })
        .collect();

    // Gaussian-Jordan elimination with partial pivoting (find any non-zero
    // pivot — full pivoting is unnecessary for f64-backed rigs and rules
    // out the general `Q: Ring` future case).
    for col in 0..n {
        // Find a pivot row `pivot >= col` with non-zero entry in column `col`.
        let pivot = (col..n).find(|&r| !aug[r][col].is_zero());
        let Some(pivot) = pivot else {
            return Err(CatgraphError::Composition {
                message: format!("zeta matrix is singular at column {col}"),
            });
        };
        if pivot != col {
            aug.swap(col, pivot);
        }

        // Normalize pivot row: divide every entry in row `col` by the pivot.
        // Cloning the pivot value (rather than borrowing) sidesteps the
        // simultaneous-borrow conflict with `aug[col][k]`.
        let inv_pivot = Q::one() / aug[col][col].clone();
        // `needless_range_loop` would suggest iterating over the row, but we
        // need an indexed write back into `aug[col][k]`, so the index is the
        // primary loop variable, not just a counter.

        for k in 0..(2 * n) {
            let new_val = aug[col][k].clone() * inv_pivot.clone();
            aug[col][k] = new_val;
        }

        // Eliminate column `col` from every other row. We index into
        // BOTH `aug[col]` (read pivot row) and `aug[r]` (write target row)
        // inside the inner loop, so a flat `for k in 0..(2*n)` is the
        // simplest disambiguation; an iterator would require a `split_at_mut`
        // dance that doesn't improve readability.
        for r in 0..n {
            if r == col || aug[r][col].is_zero() {
                continue;
            }
            let factor = aug[r][col].clone();

            for k in 0..(2 * n) {
                let pivot_kth = aug[col][k].clone();
                let row_kth = aug[r][k].clone();
                aug[r][k] = row_kth - factor.clone() * pivot_kth;
            }
        }
    }

    // Extract the right half (now ζ⁻¹ = μ) into an n × n entries vector.
    let mu_entries: Vec<Vec<Q>> = aug
        .into_iter()
        .map(|row| row.into_iter().skip(n).collect())
        .collect();

    MatR::new(n, n, mu_entries)
}

/// Magnitude of an enriched (Lawvere) metric space at scale `t`.
///
/// Computes `Mag(tM) = Σᵢⱼ μ_t[i][j]` where `μ_t` is the Möbius function of
/// the t-scaled space — distances multiplied by `t`, equivalently
/// `ζ_t[i][j] = exp(-t · d(i, j))` (BV 2025 §3.5; Leinster 2013, Section 2.2).
///
/// **Bound: `Q: Ring + Div + From<f64>`.** Same algebraic surface as
/// [`mobius_function`]. Among the workspace's four concrete rigs only
/// [`crate::F64Rig`] satisfies all three; callers needing a scalar `f64`
/// reduction can apply `.0` (for `F64Rig`) or `.into()` to the returned `Q`.
///
/// # Errors
///
/// Returns [`CatgraphError::Composition`] when the t-scaled zeta is singular
/// (propagated from [`mobius_function`]).
///
/// # Notes on `t`
///
/// BV 2025 Prop 3.6 establishes invertibility for any `t > 0` in the
/// language-model setting. The scaling is performed by constructing a fresh
/// [`LawvereMetricSpace`] with every recorded distance multiplied by `t`;
/// unset distances (`Tropical(+∞)`) remain `+∞` because `t · ∞ = ∞` for any
/// finite positive `t` (`f64` arithmetic gives `t * f64::INFINITY = +∞`).
pub fn magnitude<Q>(space: &LawvereMetricSpace<NodeId>, t: f64) -> Result<Q, CatgraphError>
where
    Q: Ring + Div<Output = Q> + From<f64>,
{
    // Materialize the object list once, in deterministic Vec<NodeId> order.
    let objects: Vec<NodeId> = materialize_objects(space);

    // Build a t-scaled copy: distance(a, b) = t · old(a, b). Unset distances
    // (`Tropical(+∞)`) are preserved by `f64` infinity arithmetic.
    let mut scaled = LawvereMetricSpace::new(objects.clone());
    for a in &objects {
        for b in &objects {
            let d = space.distance(a, b);
            scaled.set_distance(*a, *b, crate::Tropical(t * d.0));
        }
    }

    // Möbius-invert and sum every entry of the resulting `n × n` matrix.
    let mu = mobius_function::<Q>(&scaled)?;
    let n = mu.rows();
    let mut sum = Q::zero();
    for i in 0..n {
        for j in 0..n {
            sum = sum + mu.entries()[i][j].clone();
        }
    }
    Ok(sum)
}

// ============================================================================
// v0.2.0 — Phase 6F additions: weighting, coweighting, is_scattered
// ============================================================================

/// Weighting on a Lawvere metric space's similarity matrix ζ.
///
/// Per Leinster 2013 §1.1 Def 1.1.1, a **weighting** on ζ is a column vector
/// `w ∈ Q^I` such that `ζ · w = u_I` where `u_I = (1, 1, …, 1)ᵀ` is the
/// all-ones column. When such a vector exists, the magnitude of ζ is
/// `Σⱼ w(j)`. By Lemma 1.1.2, this equals `Σᵢ v(i)` where `v` is any
/// coweighting (see [`coweighting`]).
///
/// **Bound: `Q: Ring + Div + From<f64>`** — same algebraic surface as
/// [`mobius_function`]. The right-hand side `u_I` and the Gaussian-elimination
/// solve are both performed in `Q`. Among the workspace's four concrete rigs
/// only [`crate::F64Rig`] satisfies all three in v0.2.0.
///
/// **Relationship to [`mobius_function`].** When ζ is invertible, the unique
/// weighting equals the j-th row sum of `μ = ζ⁻¹` (Leinster Lemma 1.1.4):
/// `w(j) = Σᵢ μ(j, i)` where `μ(j, i)` is the matrix entry at row `j`,
/// column `i` (source-target convention). This function takes the more
/// direct path — solve `ζ · w = u_I` by Gaussian-Jordan elimination on
/// the augmented system `[ζ | u_I]`. The two paths agree numerically to
/// within `f64` tolerance.
///
/// # Errors
///
/// Returns [`CatgraphError::Composition`] when ζ is singular (no pivot
/// found in some column during Gaussian elimination), in which case no
/// weighting exists.
pub fn weighting<Q>(space: &LawvereMetricSpace<NodeId>) -> Result<Vec<Q>, CatgraphError>
where
    Q: Ring + Div<Output = Q> + From<f64>,
{
    let objects: Vec<NodeId> = materialize_objects(space);
    let n = objects.len();

    if n == 0 {
        return Ok(Vec::new());
    }

    // Augmented matrix `[ζ | u_I]` of shape n × (n + 1). The right-hand side
    // is a single column of ones rather than the n×n identity used by
    // `mobius_function`.
    let mut aug: Vec<Vec<Q>> = (0..n)
        .map(|i| {
            let mut row: Vec<Q> = Vec::with_capacity(n + 1);
            for j in 0..n {
                let d = space.distance(&objects[i], &objects[j]);
                let zeta_ij: f64 = (-d.0).exp();
                row.push(Q::from(zeta_ij));
            }
            row.push(Q::one());
            row
        })
        .collect();

    // Gaussian-Jordan elimination with partial pivoting (mirrors `mobius_function`).
    for col in 0..n {
        let pivot = (col..n).find(|&r| !aug[r][col].is_zero());
        let Some(pivot) = pivot else {
            return Err(CatgraphError::Composition {
                message: format!("zeta matrix is singular at column {col} (weighting solve)"),
            });
        };
        if pivot != col {
            aug.swap(col, pivot);
        }

        let inv_pivot = Q::one() / aug[col][col].clone();

        for k in 0..=n {
            let new_val = aug[col][k].clone() * inv_pivot.clone();
            aug[col][k] = new_val;
        }

        for r in 0..n {
            if r == col || aug[r][col].is_zero() {
                continue;
            }
            let factor = aug[r][col].clone();

            for k in 0..=n {
                let pivot_kth = aug[col][k].clone();
                let row_kth = aug[r][k].clone();
                aug[r][k] = row_kth - factor.clone() * pivot_kth;
            }
        }
    }

    // The last column of the row-reduced augmented matrix is `w`. Direct
    // indexing reads safer than `nth(n).expect(...)` after `into_iter()`
    // (v0.2.1 reviewer #1 I-1 — ergonomics fix).
    Ok(aug.into_iter().map(|mut row| row.swap_remove(n)).collect())
}

/// Coweighting on a Lawvere metric space's similarity matrix ζ.
///
/// Per Leinster 2013 §1.1 Def 1.1.1, a **coweighting** on ζ is a row vector
/// `v ∈ Q^J` such that `v · ζ = u_J^T` (the all-ones row). Symmetric to
/// [`weighting`]; magnitude exists iff both exist, and `Σⱼ w(j) = Σᵢ v(i)`
/// (Lemma 1.1.2).
///
/// **Implementation.** Equivalent to solving `ζᵀ · v = u_J` (transposed
/// system). For symmetric ζ — the typical Lawvere-metric case via the
/// `-ln π` embedding on a symmetric `WeightedCospan<Λ, UnitInterval>` —
/// weightings and coweightings are essentially the same (Leinster 2013
/// §1.1 last paragraph; "if our matrix ζ will be symmetric, in which case
/// weightings and coweightings are essentially the same"), so
/// `coweighting(space) == weighting(space)` numerically. For asymmetric ζ
/// (Lawvere `[0,∞]`-enrichment drops the symmetry axiom; user-built
/// asymmetric `LawvereMetricSpace` is allowed), the two functions return
/// distinct vectors that nonetheless sum to the same scalar (Lemma 1.1.2).
///
/// # Errors
///
/// Returns [`CatgraphError::Composition`] when ζ is singular and no
/// coweighting exists.
pub fn coweighting<Q>(space: &LawvereMetricSpace<NodeId>) -> Result<Vec<Q>, CatgraphError>
where
    Q: Ring + Div<Output = Q> + From<f64>,
{
    let objects: Vec<NodeId> = materialize_objects(space);
    let n = objects.len();

    if n == 0 {
        return Ok(Vec::new());
    }

    // Augmented matrix `[ζᵀ | u_J]` — same as weighting but with ζ transposed.
    let mut aug: Vec<Vec<Q>> = (0..n)
        .map(|i| {
            let mut row: Vec<Q> = Vec::with_capacity(n + 1);
            for j in 0..n {
                // Transposed: row i, col j ↦ ζ(objects[j], objects[i]).
                let d = space.distance(&objects[j], &objects[i]);
                let zeta_ji: f64 = (-d.0).exp();
                row.push(Q::from(zeta_ji));
            }
            row.push(Q::one());
            row
        })
        .collect();

    for col in 0..n {
        let pivot = (col..n).find(|&r| !aug[r][col].is_zero());
        let Some(pivot) = pivot else {
            return Err(CatgraphError::Composition {
                message: format!("zeta matrix is singular at column {col} (coweighting solve)"),
            });
        };
        if pivot != col {
            aug.swap(col, pivot);
        }

        let inv_pivot = Q::one() / aug[col][col].clone();

        for k in 0..=n {
            let new_val = aug[col][k].clone() * inv_pivot.clone();
            aug[col][k] = new_val;
        }

        for r in 0..n {
            if r == col || aug[r][col].is_zero() {
                continue;
            }
            let factor = aug[r][col].clone();

            for k in 0..=n {
                let pivot_kth = aug[col][k].clone();
                let row_kth = aug[r][k].clone();
                aug[r][k] = row_kth - factor.clone() * pivot_kth;
            }
        }
    }

    Ok(aug.into_iter().map(|mut row| row.swap_remove(n)).collect())
}

/// Returns `true` iff the Lawvere metric space is **scattered** in the
/// sense of Leinster 2013 Def 2.1.2 — i.e. for all distinct `a, b ∈ A`,
/// `d(a, b) > log(#A − 1)`.
///
/// Scatteredness is the **convergence precondition** for the chain-sum
/// Möbius formula [`crate::mobius_chains::mobius_function_via_chains`]
/// (Leinster 2013 Prop 2.1.3). Under scatteredness, the geometric bound
/// `r = (n − 1) · e^(−ε) < 1` holds (with `ε = min_{a≠b} d(a, b)`), so the
/// infinite chain-sum converges absolutely.
///
/// **Vacuous cases.** Empty spaces and one-point spaces are trivially
/// scattered (no distinct-pair to check). The implementation returns
/// `true` immediately for `n ≤ 1` and skips computing `log(n − 1)`
/// (which would yield `log(-1) = NaN` or `log(0) = −∞`).
///
/// **Distance representation.** Distances live in `Tropical(f64)`. The
/// `+∞` sentinel for unset pairs satisfies `+∞ > log(n − 1)` for any
/// finite `n`, so unset pairs auto-pass the scatteredness check.
#[must_use]
pub fn is_scattered(space: &LawvereMetricSpace<NodeId>) -> bool {
    let objects: Vec<NodeId> = materialize_objects(space);
    let n = objects.len();
    if n <= 1 {
        return true;
    }
    // (n-1) is bounded by metric-space sizes that fit in f64's 52-bit mantissa
    // in any realistic setting (well below 2^52 ≈ 4.5e15 nodes).
    #[allow(clippy::cast_precision_loss)]
    let log_n_minus_1 = ((n - 1) as f64).ln();
    for (i, a) in objects.iter().enumerate() {
        for b in objects.iter().skip(i + 1) {
            // Lawvere asymmetry: check both directions.
            if space.distance(a, b).0 <= log_n_minus_1 || space.distance(b, a).0 <= log_n_minus_1 {
                return false;
            }
        }
    }
    true
}

/// Möbius-invertibility predicate via the scatteredness threshold
/// (Leinster 2013 §2.1, Def 2.1.2 + Prop 2.1.3).
///
/// For a Lawvere metric space `M` and `t > 0`, returns `true` iff `tM` is
/// **scattered**, i.e. `t · d(a, b) > log(n − 1)` for all distinct `a, b ∈ M`.
/// Scatteredness is sufficient for the Möbius series
/// `μ = Σ_{k≥0} (−1)ᵏ Mᵏ` (with `M = ζ − I`) to converge absolutely
/// (Prop 2.1.3), which in turn implies `ζ_{tM}` is invertible. With distances
/// uniformly bounded below by `min_{a≠b} d(a, b)` and `t · min ≥ log(n − 1)`,
/// the per-row infinity-norm of `M` is at most `(n − 1) · e^(−t · min) ≤ 1 − ε`,
/// giving the von-Neumann convergence criterion.
///
/// In v0.3.0 this function checked `t > log(n − 1)` (assuming `min_{a≠b} d ≥ 1`).
/// In v0.3.1 the citation is corrected per Phase G paper-audit reviewer
/// finding (I-2): Prop 2.4.17 was a transcription error; the threshold
/// is the §2.1 scatteredness threshold (Def 2.1.2) plus Prop 2.1.3 chain-sum
/// convergence.
///
/// This is **conservative**: returning `false` here does not prove `tM` is
/// non-invertible (the predicate merely fails the cheap scatteredness
/// sufficient condition); returning `true` proves invertibility.
///
/// Implementation: returns `true` iff `t > log(n − 1) + ε` for `ε = 1e-9`;
/// otherwise `false`. Caller can still call [`magnitude`] and let it fail
/// numerically — this is purely an ergonomic short-circuit to avoid Möbius
/// inversion on inputs that are guaranteed to require fallback paths.
///
/// **Vacuous cases.** For `n ≤ 1`, the space is degenerate (single-point or
/// empty); the function returns `true`. The threshold `t > log(n − 1)` is
/// `log(0) = −∞` (always true) at `n = 1` and `log(−1) = NaN` at `n = 0`,
/// neither of which is useful — early-return short-circuits both.
///
/// # Examples
///
/// ```
/// use catgraph_applied::lawvere_metric::LawvereMetricSpace;
/// use catgraph_magnitude::magnitude::is_mobius_invertible_at;
///
/// let space = LawvereMetricSpace::from_distance_fn(4, |a, b| if a == b { 0.0 } else { 2.0 });
/// // log(4 − 1) ≈ 1.0986; t = 2 satisfies the threshold.
/// assert!(is_mobius_invertible_at(&space, 2.0));
/// // t = 0.5 falls below the threshold; the cheap predicate refuses to certify.
/// assert!(!is_mobius_invertible_at(&space, 0.5));
/// ```
#[must_use]
pub fn is_mobius_invertible_at(space: &LawvereMetricSpace<NodeId>, t: f64) -> bool {
    let n = space.size();
    if n <= 1 {
        return true;
    }
    #[allow(clippy::cast_precision_loss)]
    let log_n_minus_1 = ((n - 1) as f64).ln();
    t > log_n_minus_1 + 1e-9
}

/// Diagnostic companion to [`is_scattered`]: returns the first violator pair
/// `((a, b), d(a, b), log(#A − 1))` if the space is not scattered, or `None`
/// if scattered (or vacuously scattered for `n ≤ 1`).
///
/// Useful for caller-side error reporting when
/// [`crate::mobius_chains::mobius_function_via_chains`] returns
/// `Err(CatgraphError::Composition)` with the "not scattered" message —
/// `scatteredness_witness` identifies *which* pair triggered the rejection.
///
/// Both directions of the Lawvere asymmetry are checked; the returned `(a, b)`
/// is the pair with the violating direction (`a → b` if that direction failed,
/// otherwise the `b → a` direction).
///
/// **v0.3.0 substrate hook.** Per the Phase 6F three-reviewer pass (Rust A-2),
/// the v0.3.0 magnitude-homology chain complex will use the violator pairs as
/// boundary-map kernel generators (the pairs at scatteredness boundary
/// correspond to chain complex elements with non-trivial `H_{0,ℓ}` for some
/// `ℓ`). v0.2.1 ships only the diagnostic predicate; v0.3.0 will consume it
/// in the chain-complex construction.
///
/// # Examples
///
/// ```
/// use catgraph_applied::lawvere_metric::LawvereMetricSpace;
/// use catgraph_applied::rig::Tropical;
/// use catgraph_magnitude::magnitude::{is_scattered, scatteredness_witness};
///
/// // 4-state space with d = 0.1 between all distinct pairs;
/// // log(3) ≈ 1.099 > 0.1 ⇒ not scattered.
/// let mut space: LawvereMetricSpace<usize> = LawvereMetricSpace::new(vec![0, 1, 2, 3]);
/// for a in 0..4 {
///     for b in 0..4 {
///         space.set_distance(a, b, if a == b { Tropical(0.0) } else { Tropical(0.1) });
///     }
/// }
/// assert!(!is_scattered(&space));
/// let witness = scatteredness_witness(&space).expect("not scattered ⇒ witness exists");
/// assert_eq!(witness.0, (0, 1));      // first violator pair
/// assert!((witness.1 - 0.1).abs() < 1e-12);
/// assert!((witness.2 - ((4 - 1) as f64).ln()).abs() < 1e-12);
/// ```
#[must_use]
pub fn scatteredness_witness(
    space: &LawvereMetricSpace<NodeId>,
) -> Option<((NodeId, NodeId), f64, f64)> {
    let objects: Vec<NodeId> = materialize_objects(space);
    let n = objects.len();
    if n <= 1 {
        return None;
    }
    #[allow(clippy::cast_precision_loss)]
    let log_n_minus_1 = ((n - 1) as f64).ln();
    for (i, a) in objects.iter().enumerate() {
        for b in objects.iter().skip(i + 1) {
            let d_ab = space.distance(a, b).0;
            if d_ab <= log_n_minus_1 {
                return Some(((*a, *b), d_ab, log_n_minus_1));
            }
            let d_ba = space.distance(b, a).0;
            if d_ba <= log_n_minus_1 {
                return Some(((*b, *a), d_ba, log_n_minus_1));
            }
        }
    }
    None
}
