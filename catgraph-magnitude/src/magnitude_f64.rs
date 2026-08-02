#![cfg(feature = "f64-fast")]
//! `f64` fast path for the magnitude pipeline — feature `f64-fast`.
//!
//! One **symmetric factorization** of the zeta matrix replaces the three
//! independent hand-rolled Gauss–Jordan eliminations of [`crate::magnitude`]
//! ([`mobius_function`](crate::magnitude::mobius_function),
//! [`weighting`](crate::magnitude::weighting), and
//! [`coweighting`](crate::magnitude::coweighting) each carry their own copy of
//! the elimination loop). ζ is symmetric in the typical `−ln π` case — see the
//! symmetry discussion in [`crate::magnitude::coweighting`]'s docs, quoting
//! Leinster 2013 §1.1's "often our matrix ζ will be symmetric, in which case
//! weightings and coweightings are essentially the same" — so a single
//! factorization serves every downstream query.
//!
//! ## Paper anchors vs numerics
//!
//! The *quantities* are paper-anchored:
//!
//! - weighting / coweighting — Leinster 2013 §1.1 Def 1.1.1 (`ζ · w = u_I`,
//!   `v · ζ = u_J^T`), with `Σⱼ w(j) = Σᵢ v(i)` by Lemma 1.1.2;
//! - `μ = ζ⁻¹` and `w(j) = Σᵢ μ(j, i)` — Leinster 2013 §1.1 Lemma 1.1.4;
//! - magnitude `Mag(tM) = Σᵢⱼ μ_t[i][j] = Σⱼ w(j) = 1ᵀ ζ⁻¹ 1` — BV 2025 §3.5
//!   Eq (7) (see [`magnitude_f64`] for the `t`-scaled entry point).
//!
//! The *factorization choice* is *numerical*, not paper-anchored: nothing in
//! Leinster 2013 / BV 2025 prescribes Cholesky, Bunch–Kaufman, or Gaussian
//! elimination. This module picks whichever is applicable and records the
//! choice in [`FactorizationPath`].
//!
//! ## Route selection
//!
//! [`ZetaFactorization::new`] builds ζ once (through the shared
//! [`zeta_from_scaled_distance`](crate::magnitude) kernel, so entries are
//! ULP-identical to the generic path's), records `‖ζ‖₁`, then:
//!
//! 1. **exactly symmetric** (`ζ[i][j] == ζ[j][i]` bitwise for every pair) —
//!    try `Cholesky` ([`FactorizationPath::Cholesky`]); ζ is positive-definite
//!    for the common uniform-distance / negative-type cases.
//! 2. Cholesky rejected (ζ symmetric but **indefinite** — legal: an arbitrary
//!    symmetric `LawvereMetricSpace` need not induce a positive-definite ζ) —
//!    Bunch–Kaufman `LBLT` ([`FactorizationPath::Lblt`]), probed once at
//!    construction for a structurally-zero pivot.
//! 3. **asymmetric** ζ, or an `LBLT` that reports a zero pivot — fall back to
//!    the untouched rig-generic Gauss–Jordan functions instantiated at
//!    `Q = `[`F64Rig`] ([`FactorizationPath::GaussJordan`]). Lawvere
//!    `[0, ∞]`-enrichment drops the symmetry axiom, so asymmetric spaces are
//!    legal input and must not silently take a symmetric route (nalgebra's
//!    `Cholesky` / `LBLT` read only the lower triangle and would otherwise
//!    factor the wrong matrix).
//!
//! **Error parity.** An **exactly** singular ζ produces
//! [`CatgraphError::Composition`] on every route, exactly as the generic path
//! does: routes 1 and 2 are only entered once the factorization has certified
//! invertibility, and route 3 *is* the generic path. A merely **numerically**
//! near-singular ζ (e.g. `t → 0`, where `ζ_t → J` is rank 1) errors on
//! *neither* — Cholesky accepts it on a tiny positive pivot and Gauss–Jordan
//! finds a tiny non-zero pivot, so both return garbage rather than `Err`. That
//! parity is preserved but is not a safety net: [`ConditionReport`] is the
//! mitigation, since `cond₁(ζ)` blows up long before either route fails.
//!
//! **Default builds are unaffected.** The feature is off by default and no
//! existing function is touched, so default-build results stay bit-identical.
//!
//! ## Conditioning
//!
//! [`ConditionReport`] carries the induced 1-norm condition number
//! `cond₁(ζ) = ‖ζ‖₁ · ‖ζ⁻¹‖₁` whenever μ is materialized
//! ([`ZetaFactorization::condition_report`]), and a solve-only lower bound
//! otherwise ([`ZetaFactorization::condition_lower_bound`]). The struct is the
//! extension point for richer diagnostics.

use nalgebra::linalg::{Cholesky, LBLT};
use nalgebra::{DMatrix, DVector, Dyn};

use catgraph::errors::CatgraphError;
use catgraph_applied::mat::MatR;

use crate::magnitude::{materialize_objects, scaled_space, zeta_from_scaled_distance};
use crate::weighted_cospan::NodeId;
use crate::{F64Rig, LawvereMetricSpace};

/// Which factorization answered the queries on a [`ZetaFactorization`].
///
/// Purely a **numerical** diagnostic — the route does not change the
/// paper-anchored quantity being computed, only how it is obtained. Recorded
/// once at construction; every method on the handle honours it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FactorizationPath {
    /// ζ is exactly symmetric and positive-definite: `ζ = L Lᵀ`.
    Cholesky,
    /// ζ is exactly symmetric but indefinite: Bunch–Kaufman `P ζ Pᵀ = L B Lᵀ`
    /// with 1×1 and 2×2 diagonal blocks.
    Lblt,
    /// ζ is asymmetric, or Bunch–Kaufman hit a structurally-zero pivot:
    /// the untouched rig-generic Gauss–Jordan path at `Q = `[`F64Rig`].
    GaussJordan,
}

/// Conditioning diagnostics for a [`ZetaFactorization`].
///
/// All norms are the **induced 1-norm** (max absolute column sum,
/// `nalgebra::Matrix::one_norm`). Fields are private with accessors so that
/// later diagnostics can be added without a breaking change.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConditionReport {
    path: FactorizationPath,
    zeta_one_norm: f64,
    mu_one_norm: Option<f64>,
    cond_1: Option<f64>,
    cond_1_lower_bound: f64,
}

impl ConditionReport {
    /// The factorization route that produced this report.
    #[must_use]
    pub fn path(&self) -> FactorizationPath {
        self.path
    }

    /// `‖ζ‖₁` — the induced 1-norm of the zeta matrix. Always available
    /// (computed once at construction, before any factorization).
    #[must_use]
    pub fn zeta_one_norm(&self) -> f64 {
        self.zeta_one_norm
    }

    /// `‖μ‖₁ = ‖ζ⁻¹‖₁`, or `None` on a solve-only report where μ was never
    /// materialized.
    #[must_use]
    pub fn mu_one_norm(&self) -> Option<f64> {
        self.mu_one_norm
    }

    /// `cond₁(ζ) = ‖ζ‖₁ · ‖ζ⁻¹‖₁`, or `None` on a solve-only report.
    ///
    /// Submultiplicativity gives `cond₁(ζ) ≥ ‖ζ ζ⁻¹‖₁ = ‖I‖₁ = 1` for any
    /// invertible ζ with `n ≥ 1` (up to floating-point roundoff).
    ///
    /// **Degenerate `n = 0`.** The empty space has `‖ζ‖₁ = ‖μ‖₁ = 0`, so an
    /// exact report on it is `Some(0.0)` — below the `≥ 1` floor, which only
    /// applies for `n ≥ 1`. The value is reported as-is rather than special-cased
    /// so that `cond_1 == zeta_one_norm * mu_one_norm` holds unconditionally.
    #[must_use]
    pub fn cond_1(&self) -> Option<f64> {
        self.cond_1
    }

    /// A **lower bound** on `cond₁(ζ)`, always available — never the condition
    /// number itself, and never an upper bound.
    ///
    /// Derived from the weighting alone: `ζ w = u_I` gives `w = ζ⁻¹ u_I`, so
    /// `‖ζ⁻¹‖₁ ≥ ‖ζ⁻¹ u_I‖₁ / ‖u_I‖₁ = ‖w‖₁ / n`, hence
    /// `cond₁(ζ) ≥ ‖ζ‖₁ · ‖w‖₁ / n`. The bound holds on **every** route,
    /// symmetric or not, and is `0.0` for the empty space (`n = 0`).
    ///
    /// The tighter `‖ζ‖₁ · ‖w‖∞` is deliberately *not* used, because it is not
    /// **route-uniform**: `w` is a row-sum vector of `ζ⁻¹`, so `‖w‖∞ ≤ ‖ζ⁻¹‖∞`,
    /// which coincides with `‖ζ⁻¹‖₁` only when `ζ⁻¹` is symmetric. That does
    /// hold on the two symmetric routes (where the tighter bound would be
    /// valid), but not on the asymmetric Gauss–Jordan fallback. One bound that
    /// holds everywhere beats three bounds that need a route to interpret.
    #[must_use]
    pub fn cond_1_lower_bound(&self) -> f64 {
        self.cond_1_lower_bound
    }
}

/// The private factorization payload. `GaussJordan` carries nothing — that
/// route re-enters the rig-generic functions in [`crate::magnitude`], which
/// rebuild ζ themselves through the same shared kernel.
enum Factorization {
    Cholesky(Cholesky<f64, Dyn>),
    Lblt(LBLT<f64, Dyn>),
    GaussJordan,
}

/// A one-shot symmetric factorization of a Lawvere metric space's zeta matrix.
///
/// Built from the space **as given** — no `t`-scaling, mirroring
/// [`mobius_function`](crate::magnitude::mobius_function),
/// [`weighting`](crate::magnitude::weighting), and
/// [`coweighting`](crate::magnitude::coweighting). Use [`magnitude_f64`] (or
/// [`scaled_space`](crate::magnitude) followed by [`ZetaFactorization::new`])
/// for the `t`-scaled BV 2025 §3.5 Eq (7) quantity.
///
/// The handle borrows the space because the
/// [`GaussJordan`](FactorizationPath::GaussJordan) fallback re-enters the
/// rig-generic functions, which take `&LawvereMetricSpace<NodeId>`.
///
/// # Examples
///
/// ```
/// use catgraph_applied::lawvere_metric::LawvereMetricSpace;
/// use catgraph_magnitude::magnitude_f64::{FactorizationPath, ZetaFactorization};
///
/// // 4-point space, all distinct distances 2.0 ⇒ ζ = (1 − c)I + cJ, positive-definite.
/// let space = LawvereMetricSpace::from_distance_fn(4, |a, b| if a == b { 0.0 } else { 2.0 });
/// let fact = ZetaFactorization::new(&space);
/// assert_eq!(fact.path(), FactorizationPath::Cholesky);
///
/// let w = fact.weighting().expect("ζ is positive-definite");
/// let mag = fact.magnitude().expect("ζ is positive-definite");
/// assert!((mag - w.iter().sum::<f64>()).abs() < 1e-12);
/// ```
pub struct ZetaFactorization<'a> {
    space: &'a LawvereMetricSpace<NodeId>,
    n: usize,
    zeta_one_norm: f64,
    factorization: Factorization,
}

impl<'a> ZetaFactorization<'a> {
    /// Build ζ from `space` and factor it, recording the route in
    /// [`path`](Self::path).
    ///
    /// Infallible: a ζ that neither Cholesky nor Bunch–Kaufman can handle
    /// degrades to the [`GaussJordan`](FactorizationPath::GaussJordan) route,
    /// which reports **exact** singularity later (per method, as `Err`) exactly
    /// as the generic functions do. Merely *near*-singular ζ is not rejected on
    /// any route — see [`condition_report`](Self::condition_report).
    ///
    /// ζ entries are built through the crate-shared
    /// `zeta_from_scaled_distance` kernel over the
    /// `materialize_objects` order, so they are ULP-identical to the entries
    /// the generic path and [`crate::coalition_eval`] construct.
    #[must_use]
    pub fn new(space: &'a LawvereMetricSpace<NodeId>) -> Self {
        let objects: Vec<NodeId> = materialize_objects(space);
        let n = objects.len();

        let zeta = DMatrix::<f64>::from_fn(n, n, |i, j| {
            zeta_from_scaled_distance(space.distance(&objects[i], &objects[j]).0)
        });
        let zeta_one_norm = zeta.one_norm();

        // Exact (bitwise) symmetry only. A NaN entry compares unequal to
        // itself and therefore routes to Gauss–Jordan, which is the safe
        // choice: nalgebra's Cholesky / LBLT read the lower triangle alone and
        // would silently factor a *different* matrix on asymmetric input.
        // `clippy::float_cmp` (advisory pedantic) suggests an epsilon compare —
        // deliberately not wanted here: a merely *near*-symmetric ζ is still
        // not the matrix the lower-triangle-only routines would factor, so
        // anything short of bit equality must fall back.
        #[allow(clippy::float_cmp)]
        let symmetric = (0..n).all(|i| ((i + 1)..n).all(|j| zeta[(i, j)] == zeta[(j, i)]));

        let factorization = if !symmetric {
            Factorization::GaussJordan
        } else if let Some(chol) = Cholesky::new(zeta.clone()) {
            Factorization::Cholesky(chol)
        } else {
            // Symmetric but not positive-definite — Bunch–Kaufman is the
            // symmetric-indefinite factorization. `LBLT::new` is infallible;
            // `solve` fails iff a structurally-zero pivot was recorded,
            // independent of the right-hand side, so one probe here settles
            // the route for every later query. `solve_mut` is used over `solve`
            // to skip the RHS clone (it overwrites the buffer in place and
            // returns the bool directly). The O(n²) forward/backward
            // substitution the probe pays for is unavoidable: `LBLT`'s
            // `zero_pivot` field is private, so the bool is the only public
            // read of it.
            let lblt = LBLT::new(zeta);
            let mut probe = DVector::<f64>::zeros(n);
            if lblt.solve_mut(&mut probe) {
                Factorization::Lblt(lblt)
            } else {
                Factorization::GaussJordan
            }
        };

        Self {
            space,
            n,
            zeta_one_norm,
            factorization,
        }
    }

    /// The factorization route taken at construction.
    #[must_use]
    pub fn path(&self) -> FactorizationPath {
        match self.factorization {
            Factorization::Cholesky(_) => FactorizationPath::Cholesky,
            Factorization::Lblt(_) => FactorizationPath::Lblt,
            Factorization::GaussJordan => FactorizationPath::GaussJordan,
        }
    }

    /// Number of objects `n` — the order of ζ.
    #[must_use]
    pub fn size(&self) -> usize {
        self.n
    }

    /// `true` for the empty space (`n = 0`).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// `‖ζ‖₁`, the induced 1-norm of the zeta matrix (max absolute column sum).
    #[must_use]
    pub fn zeta_one_norm(&self) -> f64 {
        self.zeta_one_norm
    }

    /// The weighting `w` with `ζ · w = u_I` (Leinster 2013 §1.1 Def 1.1.1).
    ///
    /// One triangular solve on the shared factorization; on the
    /// [`GaussJordan`](FactorizationPath::GaussJordan) route this delegates to
    /// [`crate::magnitude::weighting`]`::<`[`F64Rig`]`>`.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] when ζ is **exactly** singular —
    /// reachable only on the Gauss–Jordan route, since the other two routes
    /// certified invertibility at construction. A near-singular ζ returns `Ok`
    /// with a numerically meaningless `w` on *every* route (including the
    /// generic one, so parity holds); [`ConditionReport`] is the mitigation.
    pub fn weighting(&self) -> Result<Vec<f64>, CatgraphError> {
        match &self.factorization {
            Factorization::Cholesky(chol) => Ok(to_vec(&chol.solve(&self.ones()))),
            Factorization::Lblt(lblt) => Ok(to_vec(
                &lblt
                    .solve(&self.ones())
                    .ok_or_else(|| singular("weighting solve"))?,
            )),
            Factorization::GaussJordan => {
                crate::magnitude::weighting::<F64Rig>(self.space).map(unwrap_rig)
            }
        }
    }

    /// The coweighting `v` with `v · ζ = u_J^T` (Leinster 2013 §1.1 Def 1.1.1).
    ///
    /// On the symmetric routes `ζᵀ = ζ`, so the coweighting **is** the
    /// weighting (Leinster 2013 §1.1: "often our matrix ζ will be symmetric, in
    /// which case weightings and coweightings are essentially the same") and
    /// the same solve is reused. On the
    /// [`GaussJordan`](FactorizationPath::GaussJordan) route this delegates to
    /// [`crate::magnitude::coweighting`]`::<`[`F64Rig`]`>`, which solves the
    /// transposed system and generally returns a *different* vector — with the
    /// same sum, by Leinster 2013 Lemma 1.1.2.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] when ζ is singular (Gauss–Jordan
    /// route only).
    pub fn coweighting(&self) -> Result<Vec<f64>, CatgraphError> {
        match &self.factorization {
            Factorization::Cholesky(_) | Factorization::Lblt(_) => self.weighting(),
            Factorization::GaussJordan => {
                crate::magnitude::coweighting::<F64Rig>(self.space).map(unwrap_rig)
            }
        }
    }

    /// Magnitude `Σⱼ w(j) = 1ᵀ ζ⁻¹ 1` of the space **as given** (BV 2025 §3.5
    /// Eq (7) via Leinster 2013 §1.1 Lemma 1.1.4 — the weighting sum equals the
    /// Möbius entry sum).
    ///
    /// No μ is materialized: this is one solve plus a reduction, against the
    /// generic path's full `n × 2n` inversion.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] when ζ is singular (Gauss–Jordan
    /// route only).
    pub fn magnitude(&self) -> Result<f64, CatgraphError> {
        Ok(self.weighting()?.iter().sum())
    }

    /// The Möbius function `μ = ζ⁻¹` (Leinster 2013 / Leinster–Shulman §2),
    /// in the same [`MatR`] shape the generic
    /// [`mobius_function`](crate::magnitude::mobius_function) returns.
    ///
    /// Cholesky route: `Cholesky::inverse`. Bunch–Kaufman route: the `n` solves
    /// against `I`, issued as one multi-column solve. Gauss–Jordan route:
    /// delegates to the generic function.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] when ζ is singular (Gauss–Jordan
    /// route only), or if the `n × n` shape is somehow rejected by
    /// [`MatR::new`] (unreachable — the shape is derived from the solve output).
    pub fn mobius_function(&self) -> Result<MatR<F64Rig>, CatgraphError> {
        match &self.factorization {
            Factorization::Cholesky(chol) => mat_from_dmatrix(&chol.inverse()),
            Factorization::Lblt(lblt) => {
                let inv = lblt
                    .solve(&DMatrix::<f64>::identity(self.n, self.n))
                    .ok_or_else(|| singular("Möbius inversion"))?;
                mat_from_dmatrix(&inv)
            }
            Factorization::GaussJordan => crate::magnitude::mobius_function::<F64Rig>(self.space),
        }
    }

    /// Exact conditioning report: materializes μ, so
    /// [`cond_1`](ConditionReport::cond_1) is `Some(‖ζ‖₁ · ‖μ‖₁)`.
    ///
    /// Costs one μ materialization on top of the factorization. Use
    /// [`condition_lower_bound`](Self::condition_lower_bound) when only a
    /// solve has been paid for.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] when ζ is singular (propagated
    /// from [`mobius_function`](Self::mobius_function)).
    pub fn condition_report(&self) -> Result<ConditionReport, CatgraphError> {
        let mu = self.mobius_function()?;

        // One row-major pass accumulates both norms:
        //   * `‖μ‖₁` = max absolute **column** sum (the induced 1-norm);
        //   * `‖w‖₁` where `w = μ · u_I` is the **row**-sum vector of ζ⁻¹
        //     (Leinster 2013 Lemma 1.1.4) — free here, and it makes the lower
        //     bound agree with the solve-based constructor to solve tolerance.
        //     NOT bit-identical: μ row sums and a triangular solve are
        //     different arithmetic (measured a few ULPs apart on the asymmetric
        //     fixture), which is why the parity test compares within `1e-9`.
        let mut col_sums = vec![0.0_f64; self.n];
        let mut w_one_norm = 0.0_f64;
        for row in mu.entries() {
            let mut row_sum = 0.0_f64;
            for (col_sum, entry) in col_sums.iter_mut().zip(row) {
                *col_sum += entry.0.abs();
                row_sum += entry.0;
            }
            w_one_norm += row_sum.abs();
        }
        let mu_one_norm = col_sums.into_iter().fold(0.0_f64, f64::max);

        Ok(ConditionReport {
            path: self.path(),
            zeta_one_norm: self.zeta_one_norm,
            mu_one_norm: Some(mu_one_norm),
            cond_1: Some(self.zeta_one_norm * mu_one_norm),
            cond_1_lower_bound: self.lower_bound(w_one_norm),
        })
    }

    /// Solve-only conditioning report: no μ, so
    /// [`cond_1`](ConditionReport::cond_1) is `None` and only
    /// [`cond_1_lower_bound`](ConditionReport::cond_1_lower_bound) is populated.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] when ζ is singular (propagated
    /// from [`weighting`](Self::weighting)).
    pub fn condition_lower_bound(&self) -> Result<ConditionReport, CatgraphError> {
        let w = self.weighting()?;
        let w_one_norm: f64 = w.iter().map(|x| x.abs()).sum();
        Ok(ConditionReport {
            path: self.path(),
            zeta_one_norm: self.zeta_one_norm,
            mu_one_norm: None,
            cond_1: None,
            cond_1_lower_bound: self.lower_bound(w_one_norm),
        })
    }

    /// `cond₁(ζ) ≥ ‖ζ‖₁ · ‖w‖₁ / n` — see
    /// [`ConditionReport::cond_1_lower_bound`] for the derivation. `0.0` for
    /// the empty space.
    fn lower_bound(&self, w_one_norm: f64) -> f64 {
        if self.n == 0 {
            return 0.0;
        }
        // `n` is a matrix order; it is far below f64's exact-integer range.
        #[allow(clippy::cast_precision_loss)]
        let n = self.n as f64;
        self.zeta_one_norm * w_one_norm / n
    }

    /// The all-ones right-hand side `u_I` of Leinster 2013 §1.1 Def 1.1.1.
    fn ones(&self) -> DVector<f64> {
        DVector::from_element(self.n, 1.0)
    }
}

/// Magnitude `Mag(tM)` of a Lawvere metric space at scale `t` through the
/// factorization fast path — the `f64` parallel of
/// [`magnitude`](crate::magnitude::magnitude)`::<`[`F64Rig`]`>(space, t)`
/// (BV 2025 §3.5 Eq (7); Leinster 2013 §2.2).
///
/// Scales through the crate-shared
/// [`scaled_space`](crate::magnitude) helper — the same loop
/// [`magnitude`](crate::magnitude::magnitude) and
/// [`crate::coalition_eval`] use — then factors the scaled space and returns
/// `Σⱼ w(j)`. No μ is materialized.
///
/// # Errors
///
/// Returns [`CatgraphError::Composition`] when the `t`-scaled ζ is singular,
/// matching the generic path's failure mode.
///
/// # Examples
///
/// ```
/// use catgraph_applied::lawvere_metric::LawvereMetricSpace;
/// use catgraph_applied::rig::F64Rig;
/// use catgraph_magnitude::magnitude::magnitude;
/// use catgraph_magnitude::magnitude_f64::magnitude_f64;
///
/// let space = LawvereMetricSpace::from_distance_fn(5, |a, b| if a == b { 0.0 } else { 1.5 });
/// let fast = magnitude_f64(&space, 2.0).expect("invertible at t = 2");
/// let generic: F64Rig = magnitude(&space, 2.0).expect("invertible at t = 2");
/// assert!((fast - generic.0).abs() < 1e-9);
/// ```
pub fn magnitude_f64(space: &LawvereMetricSpace<NodeId>, t: f64) -> Result<f64, CatgraphError> {
    let scaled = scaled_space(space, t);
    ZetaFactorization::new(&scaled).magnitude()
}

/// Singular-ζ error, worded like the generic path's (`magnitude.rs`) messages.
fn singular(stage: &str) -> CatgraphError {
    CatgraphError::Composition {
        message: format!("zeta matrix is singular ({stage}, f64 fast path)"),
    }
}

/// Strip the [`F64Rig`] newtype off a generic-path result vector.
fn unwrap_rig(v: Vec<F64Rig>) -> Vec<f64> {
    v.into_iter().map(|q| q.0).collect()
}

/// Column-major `DVector`/`DMatrix` column to a plain `Vec<f64>`.
fn to_vec(v: &DVector<f64>) -> Vec<f64> {
    v.iter().copied().collect()
}

/// Wrap an `n × n` `DMatrix<f64>` as [`MatR<F64Rig>`], the shape the generic
/// [`mobius_function`](crate::magnitude::mobius_function) returns.
///
/// Deliberately local rather than reusing `catgraph_applied::mat_f64`: that
/// module sits behind applied's own `f64-rig` feature, and this crate's
/// `f64-fast` feature enables only `dep:nalgebra`, so the four-line conversion
/// is cheaper than pulling a second crate's feature into the build.
fn mat_from_dmatrix(m: &DMatrix<f64>) -> Result<MatR<F64Rig>, CatgraphError> {
    let rows = m.nrows();
    let cols = m.ncols();
    let entries: Vec<Vec<F64Rig>> = (0..rows)
        .map(|i| (0..cols).map(|j| F64Rig(m[(i, j)])).collect())
        .collect();
    MatR::new(rows, cols, entries)
}
