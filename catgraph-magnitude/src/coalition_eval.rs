//! [`CoalitionEvaluator`] — incremental coalition magnitude over a fixed base
//! coalition `S`.
//!
//! The evaluator caches, once per `S`: the closed `m × m` member coupling
//! table, and the skeletal `t`-scaled `ζ⁻¹ = μ` with its row-sum
//! ([`weighting`](crate::magnitude::weighting)) and column-sum
//! ([`coweighting`](crate::magnitude::coweighting)) vectors. Each `Mag(S ∪ {x})`
//! query is then an `O(m² + k²)` bordered-matrix update instead of a fresh
//! `O(m³)` closure plus `O(k³)` inversion.
//!
//! # The two paths (BV 2025 §3.5 Eq 7 Möbius sum)
//!
//! Adding `x` borders the coalition's Lawvere metric space with one new point,
//! `ζ′ = [[ζ_S, u], [vᵀ, 1]]`, where `u`/`v` are the `exp(−t·d)` similarities
//! from/to `x`; magnitude is `Mag = 1ᵀ (ζ′)⁻¹ 1` (Eq 7).
//!
//! - **Fast path** — when `x` neither improves any interior member-to-member
//!   closure nor merges into an existing skeletal class, `ζ_S` is unchanged and
//!   the blockwise (Schur) inverse gives a closed form in `O(m² + k²)` with no
//!   fresh inversion. See [`CoalitionEvaluator::value_with`].
//! - **Slow path** — otherwise `ζ_S` is stale: the closed table is bordered in
//!   `O(m²)` and the crate's shared skeletalize and [`magnitude`] helpers re-run
//!   on the `(m+1)`-point space.
//!
//! # Numerical contract
//!
//! 1. [`CoalitionEvaluator::base_value`] is `Σ μ` over the cached skeletal
//!    `ζ⁻¹`, accumulated row-major exactly as `magnitude` sums the Möbius
//!    inverse, so it is `==` to
//!    [`coalition_magnitude_from_couplings`](crate::coalition::coalition_magnitude_from_couplings)
//!    at the same `t`.
//! 2. [`CoalitionEvaluator::value_with`] equals the fresh `Mag(S ∪ {x})` within
//!    relative tolerance [`INCREMENTAL_REL_TOL`].
//! 3. Over a candidate sweep against a fixed `S`, incremental values rank
//!    candidates identically to fresh values.
//!
//! # Non-goal: the leave path
//!
//! Only joins (`S ∪ {x}`) are accelerated. A max-product closure cannot be
//! downdated — dropping `x` can lengthen paths that routed through it, and the
//! couplings `x` shadowed are not in the bordered form — so a removal needs a
//! new evaluator over the reduced member set.
// The border/Schur loops index several parallel vectors at once
// (`closed[i][k]·g_in[k]`, `mu[i][a]·u[a]`, `closed[i][j]` vs `c[i]·r[j]`), so
// an enumerate-rewrite would cover only one indexed operand.
#![allow(clippy::needless_range_loop)]

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use crate::coalition::{
    Coalition, build_coupling_category, build_skeletal_space, skeletal_classes,
};
use crate::magnitude::{magnitude, mobius_function, scaled_space, zeta_from_scaled_distance};
use crate::{CatgraphError, F64Rig};

/// Relative tolerance holding between an incremental value and a fresh
/// evaluation of the same `Mag(S ∪ {x})`.
///
/// Both routes compute the same real quantity and differ only by
/// floating-point reassociation.
pub const INCREMENTAL_REL_TOL: f64 = 1e-9;

/// Relative threshold below which the fast path's Schur complement `s` is
/// treated as too ill-conditioned, and [`CoalitionEvaluator::value_with`] takes
/// the slow path instead of the closed-form update.
///
/// The fast path's `Mag′ = base + (1−p)(1−q)/s` agrees with a fresh evaluation
/// within [`INCREMENTAL_REL_TOL`] only while `s` is well-conditioned: on a
/// near-singular bordered `ζ′` — `s = 1 − vᵀμu` a cancellation residue, or a
/// near-clone candidate escaping the exact-`1.0` skeletal-merge test — dividing
/// by a tiny `s` amplifies that residue past tolerance. Such borders instead go
/// through the helpers fresh evaluation uses: a finite value when well-defined,
/// an `Err` exactly when the re-inversion is singular.
pub const SCHUR_SLOW_FALLBACK_TOL: f64 = 1e-12;

/// Which update path [`CoalitionEvaluator::value_with`] took for a candidate,
/// surfaced by [`JoinReport::path`].
///
/// `Fast` is the closed-form bordered-Schur update against the cached skeletal
/// `μ`, taken when `x` neither improves an interior member-to-member closure
/// nor merges into an existing skeletal class **and** the Schur complement `s`
/// is well-conditioned. `Slow` and `SlowNearSingular` are the same
/// border-then-re-skeletalize-and-re-invert route, reached from the branch
/// tests and from the [`SCHUR_SLOW_FALLBACK_TOL`] guard on `s` respectively.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EvalPath {
    /// Bordered Schur update against the cached `μ` (no fresh inversion).
    Fast,
    /// Re-skeletalize + re-invert the bordered `(m+1)`-point table, entered on
    /// an interior improvement or a skeletal merge.
    Slow,
    /// The same re-skeletalize + re-invert route, entered from the fast branch
    /// because `|s| ≤ SCHUR_SLOW_FALLBACK_TOL · (1 + |vᵀμu|)`.
    SlowNearSingular,
}

/// A structural certificate that a candidate's real diversity increment is
/// exactly `0`.
///
/// Each variant is a predicate over already-computed data — the bordered
/// closure vectors `c` / `r` and the cached closed table — decided by exact
/// `f64` comparison, with no tolerance. Exactness rests on `fl(a·b) == 1.0 ⟺
/// a == 1.0 ∧ b == 1.0` for `a, b ∈ [0, 1]`: the closure and border passes
/// perform only products and comparisons seeded at `1.0`, so `c[i] == 1.0`
/// holds iff an all-exactly-`1.0` coupling path exists.
///
/// A proof means the increment is zero **as a real number**; the returned
/// [`JoinReport::value`] may still differ from [`JoinReport::base`] by
/// floating-point roundoff, so branch on the proof rather than on
/// [`JoinReport::increment`].
///
/// # Precedence
///
/// At most one proof is reported. [`SkeletalMerge`](Self::SkeletalMerge) routes
/// through [`EvalPath::Slow`] while the two duplicate proofs are
/// [`EvalPath::Fast`]-only, so those are mutually exclusive; when an incoming
/// and an outgoing duplicate both hold, the incoming one is reported.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ZeroDiversityProof {
    /// `x` is a perfect (mutual-`1.0`) clone of `member` **and** opens no new
    /// interior shortcut: `(∃i: c[i] == 1.0 && r[i] == 1.0) && !interior_improvement`.
    ///
    /// `member` is the first such `i` in local `0..m` order.
    ///
    /// The `¬interior_improvement` conjunct is load-bearing: a mutual-`1.0`
    /// clone can simultaneously open a better `j → x → k` shortcut, which
    /// changes the closure and the magnitude, so no proof is issued there.
    SkeletalMerge {
        /// Local index (into [`CoalitionEvaluator::members`]) of the merged member.
        member: usize,
    },
    /// The candidate's incoming border replicates a member's closed column:
    /// `∃a: c[a] == 1.0 && ∀i: c[i] == closed[i][a]` (bitwise `f64` equality).
    ///
    /// Then the bordered similarity vector is `u = ζ_S·e_a`, so
    /// `p = 1ᵀζ_S⁻¹u = 1` **as a real number** and the fast path's increment
    /// `(1−p)(1−q)/s` is exactly `0` for any `q` and any `s ≠ 0`.
    ///
    /// The `c[a] == 1.0` prefilter is a necessary condition of the `∀` clause
    /// (`closed[a][a] == 1.0` by the identity axiom), so it prunes candidates
    /// without weakening the predicate.
    ///
    /// **Fast-path-scoped.** See [`JoinReport`]'s contract.
    IncomingProfileDuplicate {
        /// Local index of the member whose closed column `c` replicates.
        member: usize,
    },
    /// The transpose: the candidate's outgoing border replicates a member's
    /// closed **row**, `∃a: r[a] == 1.0 && ∀j: r[j] == closed[a][j]`.
    ///
    /// Then `v = e_aᵀζ_S`, so `q = vᵀζ_S⁻¹1 = 1` as a real number and the
    /// increment `(1−p)(1−q)/s` is again exactly `0`.
    ///
    /// **Fast-path-scoped.** See [`JoinReport`]'s contract.
    OutgoingProfileDuplicate {
        /// Local index of the member whose closed row `r` replicates.
        member: usize,
    },
}

/// The outcome of one [`CoalitionEvaluator::value_with_report`] query: the same
/// scalar [`value_with`](CoalitionEvaluator::value_with) returns, plus the
/// structural facts the scalar discards.
///
/// # Contract
///
/// - [`zero_proof`](Self::zero_proof) `== Some(_)` (equivalently
///   [`is_provably_zero`](Self::is_provably_zero) `== true`) ⇒ the **real**
///   increment is exactly `0`. The returned [`value`](Self::value) may still
///   differ from [`base`](Self::base) by floating-point roundoff, so callers
///   branch on the proof, not on `increment() == 0.0`.
/// - [`zero_proof`](Self::zero_proof) `== None` means **not proven**, never
///   "nonzero": zeros arising as affine-combination coincidences, and zeros on
///   the slow/interior route, carry no proof.
/// - The two duplicate proofs are **fast-path-scoped** — attached only when the
///   value came out of the closed-form Schur branch
///   ([`schur_complement`](Self::schur_complement) `== Some(s)`). Under
///   interior improvement the cached `ζ_S` is stale and the `u = ζ_S·e_a`
///   premise fails, and a near-singular border diverted by the
///   [`SCHUR_SLOW_FALLBACK_TOL`] guard to [`EvalPath::SlowNearSingular`]
///   likewise reports no proof.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JoinReport {
    value: f64,
    base: f64,
    zero_proof: Option<ZeroDiversityProof>,
    path: EvalPath,
    schur_complement: Option<f64>,
}

impl JoinReport {
    /// `Mag(S ∪ {x})` — bit-identical to
    /// [`value_with`](CoalitionEvaluator::value_with) on the same candidate.
    #[must_use]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// `Mag(S)` — the evaluator's cached
    /// [`base_value`](CoalitionEvaluator::base_value).
    #[must_use]
    pub fn base(&self) -> f64 {
        self.base
    }

    /// `value() − base()`, the computed increment.
    ///
    /// A float difference, not a decision procedure: a proven-zero candidate
    /// can return a nonzero increment of order `1e-14`. Use
    /// [`zero_proof`](Self::zero_proof) / [`is_provably_zero`](Self::is_provably_zero)
    /// to decide "adds nothing".
    #[must_use]
    pub fn increment(&self) -> f64 {
        self.value - self.base
    }

    /// The structural certificate, if one of the three exact classes fired.
    ///
    /// `None` means **not proven**, never "nonzero" — see the contract above.
    #[must_use]
    pub fn zero_proof(&self) -> Option<ZeroDiversityProof> {
        self.zero_proof
    }

    /// `true` iff a [`ZeroDiversityProof`] was found, i.e. the **real** increment
    /// is exactly `0`. `false` means *not proven*, not "nonzero".
    #[must_use]
    pub fn is_provably_zero(&self) -> bool {
        self.zero_proof.is_some()
    }

    /// Which update route produced [`value`](Self::value).
    #[must_use]
    pub fn path(&self) -> EvalPath {
        self.path
    }

    /// The Schur complement `s = 1 − vᵀμu`, `Some(s)` **iff** the closed-form
    /// fast branch produced the value (so it is `Some` exactly when
    /// [`path`](Self::path) is [`EvalPath::Fast`], and `None` on both
    /// [`EvalPath::Slow`] and [`EvalPath::SlowNearSingular`]).
    ///
    /// Exposed as conditioning telemetry: `det ζ′ = det(ζ_S)·s`, so a small `|s|`
    /// is a near-singular bordered `ζ′`.
    #[must_use]
    pub fn schur_complement(&self) -> Option<f64> {
        self.schur_complement
    }
}

/// Everything one `value_with_*` evaluation produces. The public entry points
/// project out what they expose; `REPORT`-gated fields are left at their inert
/// defaults on the non-reporting monomorphization (see `value_with_core`).
struct EvalOutcome {
    value: f64,
    path: EvalPath,
    zero_proof: Option<ZeroDiversityProof>,
    schur_complement: Option<f64>,
}

/// Caches a base coalition `S` so per-candidate `Mag(S ∪ {x})` queries skip the
/// `O(m³)` closure and, on the fast path, the `O(k³)` Möbius inversion.
///
/// Build one with [`CoalitionEvaluator::new`] and reuse the handle across a
/// candidate sweep; a one-shot pair is served by [`coalition_value_delta`]. The
/// evaluator is immutable — joins never mutate the cached `S`.
///
/// It stores agent **indices** and `f64` data only, so it is not generic over
/// the agent domain `O`; [`CoalitionEvaluator::new`] takes the `agents: &[O]`
/// slice and candidates are indices into it.
#[derive(Clone, Debug)]
pub struct CoalitionEvaluator {
    /// Coalition members as agent indices, in local `0..m` order.
    members: Vec<usize>,
    /// The pinned scale the cache was built at.
    t: f64,
    /// Number of agents — candidate-index bound.
    n_agents: usize,
    /// Validated member-incident couplings `(from, to) → prob`, last-write-wins
    /// on duplicates (matching `HomMap::set_hom`'s overwrite). Read to border a
    /// candidate against a member.
    couplings: HashMap<(usize, usize), f64>,
    /// Closed `m × m` member coupling table (diagonal `1.0`; `0.0` for absent
    /// pairs) — bit-identical to the base [`Coalition`]'s closure.
    closed: Vec<Vec<f64>>,
    /// Skeleton class representatives: `reps[c]` is the first member (local
    /// index) of class `c`, in `0..k` where `k = reps.len()`.
    reps: Vec<usize>,
    /// Skeletal, `t`-scaled `ζ⁻¹ = μ` of `S`, dense `k × k`.
    mu: Vec<Vec<f64>>,
    /// Weighting `w = μ · 1` (row sums of `μ`) — Leinster 2013 Lemma 1.1.4.
    weighting: Vec<f64>,
    /// Coweighting `v = 1ᵀ · μ` (column sums of `μ`).
    coweighting: Vec<f64>,
    /// Cached fresh `Mag(S)` (= `Σ μ`, contract point 1).
    base_mag: f64,
}

/// Caller-owned scratch buffers for [`CoalitionEvaluator::value_with_scratch`],
/// replacing the seven per-call `Vec` allocations — `g_in`, `g_out`, `c`, `r`
/// of length `m` and `u`, `v`, `w_u` of length `k` — with reuse across a
/// candidate sweep.
///
/// # Reuse contract
///
/// The buffers hold no cross-call state: every call resizes them to the current
/// coalition and overwrites every live entry before reading it, so a reused
/// `EvalScratch` yields results bit-identical to a fresh one. Capacity grows to
/// the largest coalition served and never shrinks.
///
/// The evaluator stays `&self`; the mutable state lives here, caller-owned.
/// Build with [`EvalScratch::new`] — the buffers size on first use.
#[derive(Clone, Debug, Default)]
pub struct EvalScratch {
    /// Direct `member_i → candidate` couplings (length `m`).
    g_in: Vec<f64>,
    /// Direct `candidate → member_i` couplings (length `m`).
    g_out: Vec<f64>,
    /// Bordered closure `c[i] = closed(i → x)` (length `m`).
    c: Vec<f64>,
    /// Bordered closure `r[j] = closed(x → j)` (length `m`).
    r: Vec<f64>,
    /// Border similarity `u[a] = ζ(rep_a → x)` over skeleton classes (length `k`).
    u: Vec<f64>,
    /// Border similarity `v[a] = ζ(x → rep_a)` over skeleton classes (length `k`).
    v: Vec<f64>,
    /// `w_u = μ · u` (length `k`).
    w_u: Vec<f64>,
}

impl EvalScratch {
    /// A fresh, empty scratch. The buffers size themselves on the first
    /// [`CoalitionEvaluator::value_with_scratch`] call and are reused thereafter.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl CoalitionEvaluator {
    /// Build an evaluator for coalition `S = members` over `agents`, at scale
    /// `t`, from a sparse coupling table.
    ///
    /// Validation mirrors
    /// [`coalition_magnitude_from_couplings`](crate::coalition::coalition_magnitude_from_couplings)
    /// in order and rejected cases, so an evaluator constructs iff that function
    /// would succeed on `(agents, couplings, members, t)`: member indices first,
    /// then coupling indices, self-couplings, and probabilities (via
    /// [`UnitInterval::new`](crate::UnitInterval::new)). The base [`Coalition`]
    /// is built through the same `HomMap` + [`Coalition::from_enriched`]
    /// restrict-then-close-then-skeletalize path.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError`] if:
    /// - a `members` index is out of range for `agents`,
    /// - a coupling index is out of range, or a coupling is a self-loop
    ///   `(i, i, _)` (the identity axiom fixes the diagonal to `1.0`),
    /// - some probability is outside `[0, 1]` (via [`UnitInterval::new`](crate::UnitInterval::new)),
    /// - `members` is empty / has a duplicate / names a non-agent (from
    ///   [`Coalition::from_enriched`]), or
    /// - the `t`-scaled skeletal `ζ` of `S` is singular (from
    ///   [`mobius_function`]).
    pub fn new<O>(
        agents: &[O],
        couplings: &[(usize, usize, f64)],
        members: &[usize],
        t: f64,
    ) -> Result<Self, CatgraphError>
    where
        O: Copy + Eq + Hash + Debug + 'static,
    {
        // Validate + build the enriched category through the SAME helper
        // `coalition_magnitude_from_couplings` uses, so construction accepts /
        // rejects identically; `coupling_map` holds the member-incident
        // couplings the border reads.
        let (cat, member_objs, coupling_map) =
            build_coupling_category(agents, couplings, members, "CoalitionEvaluator::new")?;

        let coalition = Coalition::from_enriched(&cat, &member_objs)?;
        let m = coalition.len();

        // Extract the closed table exactly as stored: diagonal `1.0`, `0.0` for
        // absent off-diagonal pairs. This equals `bellman_ford_closure`'s output
        // bit-for-bit (the weights were written from it), so the incremental
        // borders stay consistent with fresh evaluation.
        let wc = coalition.as_weighted_cospan();
        let closed: Vec<Vec<f64>> = (0..m)
            .map(|i| (0..m).map(|j| wc.weight(i, j).value()).collect())
            .collect();

        // Reuse the coalition's cached skeleton rather than re-skeletalizing.
        // `skeletal_classes` numbers classes in first-seen order with the first
        // member as representative, so the reps are the positions where each new
        // class index first appears in `member_classes()`.
        let member_classes = coalition.member_classes();
        let mut reps: Vec<usize> = Vec::new();
        for (i, &c) in member_classes.iter().enumerate() {
            if c == reps.len() {
                reps.push(i);
            }
        }

        // Cache the skeletal, t-scaled ζ⁻¹ from the coalition's own space via the
        // shared `scaled_space` — the exact scaling `magnitude` inverts, so the
        // cached μ matches a fresh inversion bit-for-bit.
        let scaled = scaled_space(coalition.space(), t);
        let mu_mat = mobius_function::<F64Rig>(&scaled)?;
        let mu: Vec<Vec<f64>> = mu_mat
            .entries()
            .iter()
            .map(|row| row.iter().map(|e| e.0).collect())
            .collect();

        // Weighting = μ row sums; coweighting = μ column sums (Leinster 2013
        // Lemma 1.1.4 / §1.1). These are the border reductions the Schur update
        // needs: `q = v · weighting`, `p = coweighting · u`.
        let k = mu.len();
        let weighting: Vec<f64> = (0..k).map(|i| mu[i].iter().copied().sum()).collect();
        let coweighting: Vec<f64> = (0..k).map(|j| (0..k).map(|i| mu[i][j]).sum()).collect();

        // Contract point 1: Mag(S) = Σ μ, accumulated row-major into a single
        // f64 exactly as `magnitude` sums the Möbius inverse (F64Rig add is plain
        // `+`, from 0.0), so `base_value()` stays bit-identical to a fresh
        // `coalition_magnitude_from_couplings` without a second inversion.
        let mut base_mag = 0.0_f64;
        for i in 0..k {
            for j in 0..k {
                base_mag += mu[i][j];
            }
        }

        Ok(Self {
            members: members.to_vec(),
            t,
            n_agents: agents.len(),
            couplings: coupling_map,
            closed,
            reps,
            mu,
            weighting,
            coweighting,
            base_mag,
        })
    }

    /// The cached `Mag(S)` at the evaluator's `t`, `==` exact to
    /// [`coalition_magnitude_from_couplings`](crate::coalition::coalition_magnitude_from_couplings).
    #[must_use]
    pub fn base_value(&self) -> f64 {
        self.base_mag
    }

    /// The coalition members as agent indices, in local `0..m` order.
    #[must_use]
    pub fn members(&self) -> &[usize] {
        &self.members
    }

    /// The scale `t` the cache was built at.
    #[must_use]
    pub fn t(&self) -> f64 {
        self.t
    }

    /// The cached closed `m × m` member coupling table (diagonal `1.0`, `0.0`
    /// for absent pairs), member-local order.
    ///
    pub(crate) fn closed_table(&self) -> &[Vec<f64>] {
        &self.closed
    }

    /// The cached skeletal class representatives — `reps[c]` is the first member
    /// (local index) of class `c`, in `0..k`.
    pub(crate) fn class_reps(&self) -> &[usize] {
        &self.reps
    }

    /// The cached weighting `w = μ · 1` over skeletal classes (Leinster 2013
    /// Lemma 1.1.4), indexed identically to [`class_reps`](Self::class_reps);
    /// `Σ w = base_value()` mathematically — in floats only up to
    /// re-association (`base_mag` accumulates row-major over `μ`, the
    /// weighting sums each row separately).
    pub(crate) fn weighting_vec(&self) -> &[f64] {
        &self.weighting
    }

    /// `Mag(S ∪ {candidate})` at the evaluator's `t`, incrementally.
    ///
    /// `candidate` is an agent index; only its **direct** couplings to/from
    /// members enter (restrict-then-close: mediation through a non-member is
    /// dropped, as [`Coalition::from_enriched`] restricts).
    ///
    /// # Algorithm
    ///
    /// With `g_in[i]` / `g_out[i]` the direct `member_i → x` / `x → member_i`
    /// couplings (`0` if absent), one `O(m²)` pass borders the cached closure:
    ///
    /// - `c[i] = closed(i → x) = maxₖ closed[i][k]·g_in[k]`  (`closed[i][i]=1`
    ///   covers the direct edge),
    /// - `r[j] = closed(x → j) = maxₖ g_out[k]·closed[k][j]`.
    ///
    /// Two `O(m²)` tests then select the path:
    /// - **interior improvement** — `∃ i≠j: c[i]·r[j] > closed[i][j]`, so the
    ///   cached `ζ_S` is stale;
    /// - **skeletal merge** — `∃ i: c[i] == 1 && r[i] == 1`, so the skeleton
    ///   shrinks (this also fires when `x` bridges two classes, which needs
    ///   mutual-`1.0` with `x`).
    ///
    /// **Fast path** (neither test fires). The bordered
    /// `ζ′ = [[ζ_S, u], [vᵀ, 1]]` has a blockwise (Schur) inverse, with
    /// `u[a] = exp(−t·d(rep_a → x))` and `v[a] = exp(−t·d(x → rep_a))` computed
    /// through the same `exp` route [`mobius_function`] uses (`0` when the
    /// coupling is `0`):
    ///
    /// - `w_u = μ·u`, Schur complement `s = 1 − vᵀμu = 1 − v·w_u`,
    /// - `p = 1ᵀμu = coweighting·u`, `q = vᵀμ1 = v·weighting`,
    /// - `Mag′ = Mag(S) + (1 − p)(1 − q)/s`.
    ///
    /// **Slow path** (improvement or merge). The closed table is bordered —
    /// `closed′[i][j] = max(closed[i][j], c[i]·r[j])`, last row/col from `c`/`r`,
    /// corner `1.0` — then re-skeletalized and re-inverted with the crate's
    /// shared [`crate::magnitude::magnitude`] helpers on the `(m+1)`-point
    /// space.
    ///
    /// The result matches the fresh `Mag(S ∪ {x})` within
    /// [`INCREMENTAL_REL_TOL`] (BV 2025 §3.5 Eq 7). A near-singular bordered
    /// `ζ′` (Schur complement `s` within [`SCHUR_SLOW_FALLBACK_TOL`] of
    /// singular) takes the slow path rather than the closed form.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] if `candidate` is out of range for
    /// `agents`, is already a member, or the bordered `ζ′` is singular — surfaced
    /// by the slow-path re-inversion (the fast path defers a near-singular border
    /// there rather than erroring itself).
    pub fn value_with(&self, candidate: usize) -> Result<f64, CatgraphError> {
        let mut scratch = EvalScratch::new();
        self.value_with_impl(candidate, &mut scratch)
            .map(|(v, _)| v)
    }

    /// `Mag(S ∪ {candidate})`, reusing caller-owned [`EvalScratch`] buffers
    /// instead of allocating the seven per-call `Vec`s.
    ///
    /// Same arithmetic, same paths, bit-identical result to
    /// [`value_with`](Self::value_with); only the border and Schur working
    /// vectors are drawn from `scratch`. Each call resizes and fully overwrites
    /// the buffers it reads, so one `scratch` serves candidates, evaluators, and
    /// coalitions of different sizes without contamination.
    ///
    /// # Errors
    ///
    /// Identical to [`value_with`](Self::value_with): [`CatgraphError::Composition`]
    /// if `candidate` is out of range, is already a member, or the bordered `ζ′`
    /// is singular (surfaced by the slow-path re-inversion).
    pub fn value_with_scratch(
        &self,
        candidate: usize,
        scratch: &mut EvalScratch,
    ) -> Result<f64, CatgraphError> {
        self.value_with_impl(candidate, scratch).map(|(v, _)| v)
    }

    /// `Mag(S ∪ {candidate})` **plus** the structural facts the scalar entry
    /// points discard — the update path, the Schur complement, and an exact
    /// [`ZeroDiversityProof`] when one of the three decidable classes fires.
    ///
    /// The returned [`JoinReport::value`] is bit-identical to
    /// [`value_with`](Self::value_with) on the same candidate — same arithmetic,
    /// same accumulation order, same branch selection.
    ///
    /// Read [`JoinReport`]'s contract before branching on the result: a proof
    /// certifies the **real** increment is exactly `0` while the returned value
    /// may differ from the base by roundoff, and `None` means *not proven*, not
    /// "nonzero".
    ///
    /// # Errors
    ///
    /// Identical to [`value_with`](Self::value_with) — same validation, same
    /// order, same messages: [`CatgraphError::Composition`] if `candidate` is out
    /// of range for `agents`, is already a member, or the bordered `ζ′` is
    /// singular (surfaced by the slow-path re-inversion).
    pub fn value_with_report(&self, candidate: usize) -> Result<JoinReport, CatgraphError> {
        let mut scratch = EvalScratch::new();
        self.value_with_report_scratch(candidate, &mut scratch)
    }

    /// [`value_with_report`](Self::value_with_report), reusing caller-owned
    /// [`EvalScratch`] buffers instead of allocating the seven per-call `Vec`s.
    ///
    /// Stands to [`value_with_report`](Self::value_with_report) as
    /// [`value_with_scratch`](Self::value_with_scratch) stands to
    /// [`value_with`](Self::value_with): bit-identical results, no cross-call
    /// state in `scratch`, and one buffer set serves both the reporting and the
    /// scalar entry points.
    ///
    /// # Errors
    ///
    /// Identical to [`value_with_report`](Self::value_with_report).
    pub fn value_with_report_scratch(
        &self,
        candidate: usize,
        scratch: &mut EvalScratch,
    ) -> Result<JoinReport, CatgraphError> {
        let outcome = self.value_with_core::<true>(candidate, scratch)?;
        Ok(JoinReport {
            value: outcome.value,
            base: self.base_mag,
            zero_proof: outcome.zero_proof,
            path: outcome.path,
            schur_complement: outcome.schur_complement,
        })
    }

    /// Core of [`value_with`](Self::value_with), also returning which
    /// [`EvalPath`] was taken. Writes its border/Schur working vectors into the
    /// caller-owned `scratch`.
    fn value_with_impl(
        &self,
        candidate: usize,
        scratch: &mut EvalScratch,
    ) -> Result<(f64, EvalPath), CatgraphError> {
        self.value_with_core::<false>(candidate, scratch)
            .map(|o| (o.value, o.path))
    }

    /// The single evaluation body behind every entry point.
    ///
    /// `REPORT` is a const parameter, so the proof scans exist only in the
    /// reporting monomorphization. The scans perform no arithmetic that feeds
    /// the value — they are `==` comparisons against already-computed data (see
    /// [`ZeroDiversityProof`]).
    #[allow(clippy::similar_names)] // `c`/`r`, `u`/`v`, `p`/`q` are the paper's border names.
    fn value_with_core<const REPORT: bool>(
        &self,
        candidate: usize,
        scratch: &mut EvalScratch,
    ) -> Result<EvalOutcome, CatgraphError> {
        if candidate >= self.n_agents {
            return Err(CatgraphError::Composition {
                message: format!(
                    "CoalitionEvaluator::value_with: candidate index {candidate} out of range for {} agents",
                    self.n_agents
                ),
            });
        }
        if self.members.contains(&candidate) {
            return Err(CatgraphError::Composition {
                message: format!(
                    "CoalitionEvaluator::value_with: candidate {candidate} is already a member of the coalition"
                ),
            });
        }

        let m = self.members.len();

        // Size the caller-owned scratch to this coalition. Every entry in
        // `[0, m)` / `[0, k)` is overwritten before it is read below, so a reused
        // scratch (from a prior candidate, evaluator, or differently-sized
        // coalition) carries no stale state — `resize` only adjusts the length,
        // and the fills that follow set every live entry.
        scratch.g_in.resize(m, 0.0);
        scratch.g_out.resize(m, 0.0);
        scratch.c.resize(m, 0.0);
        scratch.r.resize(m, 0.0);

        // Direct member↔candidate generators (restrict-then-close: only these
        // enter; non-member mediation is absent).
        for i in 0..m {
            scratch.g_in[i] = self
                .couplings
                .get(&(self.members[i], candidate))
                .copied()
                .unwrap_or(0.0);
            scratch.g_out[i] = self
                .couplings
                .get(&(candidate, self.members[i]))
                .copied()
                .unwrap_or(0.0);
        }

        // Border the cached closure — one exact O(m²) pass (see the method docs).
        for i in 0..m {
            let mut ci = 0.0_f64;
            let mut ri = 0.0_f64;
            for k in 0..m {
                ci = ci.max(self.closed[i][k] * scratch.g_in[k]);
                ri = ri.max(scratch.g_out[k] * self.closed[k][i]);
            }
            scratch.c[i] = ci;
            scratch.r[i] = ri;
        }

        // Borders are constant within a skeletal class (Kolmogorov quotient of
        // the closure: members at mutual distance 0 have equal closed distances
        // to any point), which is what lets the fast path reduce `c`/`r` to
        // class representatives. Recompute the per-member class map here (debug
        // only) rather than caching it — the field would otherwise be read
        // nowhere else.
        debug_assert!(
            {
                let (member_classes, _) = skeletal_classes(&self.closed, m);
                (0..m).all(|i| {
                    let ra = self.reps[member_classes[i]];
                    scratch.c[i] == scratch.c[ra] && scratch.r[i] == scratch.r[ra]
                })
            },
            "coalition border must be constant within each skeletal ~-class"
        );

        // Branch tests (O(m²), short-circuiting). The merge test uses `position`
        // rather than `any` — identical work and identical short-circuit, but it
        // also names the merged member for `ZeroDiversityProof::SkeletalMerge`.
        let interior_improvement = (0..m)
            .any(|i| (0..m).any(|j| i != j && scratch.c[i] * scratch.r[j] > self.closed[i][j]));
        let merge_member = skeletal_merge_member(scratch, m);

        if interior_improvement || merge_member.is_some() {
            let mut outcome = self.value_with_slow(scratch, m)?;
            // Merge-only (`merge ∧ ¬interior`) is the exact zero-diversity
            // predicate; merge ∧ interior is not, so it carries no proof.
            if REPORT
                && !interior_improvement
                && let Some(member) = merge_member
            {
                outcome.zero_proof = Some(ZeroDiversityProof::SkeletalMerge { member });
            }
            return Ok(outcome);
        }
        self.value_with_fast::<REPORT>(scratch, m)
    }

    /// The two fast-path-only exact tie classes: an incoming or outgoing
    /// **profile duplicate**.
    ///
    /// Incoming (`∃a: c[a] == 1.0 && ∀i: c[i] == closed[i][a]`) gives
    /// `u = ζ_S·e_a`, hence `p = 1` exactly as a real number; outgoing is the
    /// transpose and gives `q = 1`. Either makes the fast path's increment
    /// `(1−p)(1−q)/s` exactly zero. Incoming is tested first, so it wins when
    /// both hold (see [`ZeroDiversityProof`]'s precedence note).
    ///
    /// Bitwise `==` throughout; the `c[a] == 1.0` / `r[a] == 1.0` prefilters are
    /// necessary conditions of their `∀` clauses (`closed[a][a] == 1.0`), so
    /// they prune without weakening.
    #[allow(clippy::float_cmp)]
    fn profile_duplicate_proof(
        &self,
        scratch: &EvalScratch,
        m: usize,
    ) -> Option<ZeroDiversityProof> {
        for a in 0..m {
            if scratch.c[a] == 1.0 && (0..m).all(|i| scratch.c[i] == self.closed[i][a]) {
                return Some(ZeroDiversityProof::IncomingProfileDuplicate { member: a });
            }
        }
        for a in 0..m {
            if scratch.r[a] == 1.0 && (0..m).all(|j| scratch.r[j] == self.closed[a][j]) {
                return Some(ZeroDiversityProof::OutgoingProfileDuplicate { member: a });
            }
        }
        None
    }

    /// Fast path: bordered Schur update against the cached skeletal `μ`.
    ///
    /// Falls back to [`value_with_slow`](Self::value_with_slow) when the Schur
    /// complement `s` is near-singular (relative to `vᵀμu`, threshold
    /// [`SCHUR_SLOW_FALLBACK_TOL`]): the closed-form update and fresh evaluation
    /// agree only while `s` is well-conditioned, so an ill-conditioned border is
    /// routed through the fresh-equivalent slow path (finite when well-defined,
    /// `Err` exactly when the re-inversion is singular) instead of dividing by a
    /// catastrophic-cancellation residue, and reported as
    /// [`EvalPath::SlowNearSingular`].
    #[allow(clippy::similar_names)]
    fn value_with_fast<const REPORT: bool>(
        &self,
        scratch: &mut EvalScratch,
        m: usize,
    ) -> Result<EvalOutcome, CatgraphError> {
        let k = self.mu.len();

        // Border similarities via the exact exp route (not powf) — `u`/`v` over
        // skeleton classes, using each class representative's border. Same
        // resize-then-fill contract as the `m`-length buffers above.
        scratch.u.resize(k, 0.0);
        scratch.v.resize(k, 0.0);
        for a in 0..k {
            scratch.u[a] = zeta_entry(scratch.c[self.reps[a]], self.t);
            scratch.v[a] = zeta_entry(scratch.r[self.reps[a]], self.t);
        }

        // w_u = μ·u ; Schur complement s = 1 − vᵀμu, accumulated row-major
        // from 0.0.
        scratch.w_u.resize(k, 0.0);
        for i in 0..k {
            let mut acc = 0.0_f64;
            for a in 0..k {
                acc += self.mu[i][a] * scratch.u[a];
            }
            scratch.w_u[i] = acc;
        }
        let mut vmu = 0.0_f64;
        for a in 0..k {
            vmu += scratch.v[a] * scratch.w_u[a];
        }
        let s = 1.0 - vmu;

        // Near-singular bordered ζ′ (det(ζ′) = det(ζ_S)·s, ζ_S invertible): the
        // Schur division would amplify cancellation noise past tolerance, so
        // defer to the fresh-equivalent slow path.
        if s.abs() <= SCHUR_SLOW_FALLBACK_TOL * (1.0 + vmu.abs()) {
            let mut outcome = self.value_with_slow(scratch, m)?;
            outcome.path = EvalPath::SlowNearSingular;
            return Ok(outcome);
        }

        // p = 1ᵀμu = coweighting·u ; q = vᵀμ1 = v·weighting (dual borders).
        let mut p = 0.0_f64;
        for a in 0..k {
            p += self.coweighting[a] * scratch.u[a];
        }
        let mut q = 0.0_f64;
        for a in 0..k {
            q += scratch.v[a] * self.weighting[a];
        }

        let mag = self.base_mag + (1.0 - p) * (1.0 - q) / s;
        // Duplicate proofs are attached only here — after the near-singular
        // guard above has been cleared, i.e. only when this branch actually
        // produced the value (JoinReport's fast-path-scoped contract).
        let zero_proof = if REPORT {
            self.profile_duplicate_proof(scratch, m)
        } else {
            None
        };
        Ok(EvalOutcome {
            value: mag,
            path: EvalPath::Fast,
            zero_proof,
            schur_complement: Some(s),
        })
    }

    /// Slow path: border the closed table, then re-skeletalize + re-invert on
    /// the `(m+1)`-point space with the crate's shared helpers.
    fn value_with_slow(
        &self,
        scratch: &EvalScratch,
        m: usize,
    ) -> Result<EvalOutcome, CatgraphError> {
        let c = &scratch.c;
        let r = &scratch.r;
        let mut closed_p = vec![vec![0.0_f64; m + 1]; m + 1];
        for i in 0..m {
            for j in 0..m {
                // Old member-only path, or the new through-x shortcut c[i]·r[j].
                closed_p[i][j] = self.closed[i][j].max(c[i] * r[j]);
            }
            closed_p[i][m] = c[i]; // i → x
            closed_p[m][i] = r[i]; // x → i
        }
        closed_p[m][m] = 1.0; // identity axiom d(x, x) = 0

        let (_, reps) = skeletal_classes(&closed_p, m + 1);
        let space = build_skeletal_space(&closed_p, &reps);
        // Same triangle-inequality guard `Coalition::from_enriched` runs on the
        // fresh closure — the bordered table must stay a valid Lawvere metric.
        debug_assert!(
            space.triangle_inequality_holds_within(crate::TRIANGLE_FLOAT_TOL),
            "bordered coalition closure must satisfy the triangle inequality \
             (within TRIANGLE_FLOAT_TOL)"
        );
        let mag: F64Rig = magnitude(&space, self.t)?;
        Ok(EvalOutcome {
            value: mag.0,
            path: EvalPath::Slow,
            // The caller attaches the merge-only proof (`value_with_core`) and
            // re-labels the near-singular diversion (`value_with_fast`); the
            // slow route itself certifies nothing, and `s` was never formed (or,
            // on the diversion, was not trusted).
            zero_proof: None,
            schur_complement: None,
        })
    }
}

/// First member `i` (local order) with `c[i] == 1.0 && r[i] == 1.0` — the
/// skeletal-merge branch test, returning the witness instead of a bare bool so
/// [`ZeroDiversityProof::SkeletalMerge`] can name it. `position`/`find` does the
/// same work and short-circuits at the same place `any` did.
///
/// The comparisons are exact: the closure and border passes are
/// products-and-maxima seeded at `1.0`, so `c[i] == 1.0` holds iff an
/// all-exactly-`1.0` coupling path exists. Near-clones are a separate class,
/// handled by [`SCHUR_SLOW_FALLBACK_TOL`].
#[allow(clippy::float_cmp)]
fn skeletal_merge_member(scratch: &EvalScratch, m: usize) -> Option<usize> {
    (0..m).find(|&i| scratch.c[i] == 1.0 && scratch.r[i] == 1.0)
}

/// `ζ`-similarity for a coupling `π` at scale `t`, through the crate's single
/// zeta kernel [`zeta_from_scaled_distance`].
///
/// `build_skeletal_space` stores `d = −ln π`, the scaling lifts it to `t·d`, and
/// the kernel reads `exp(−(t·d))`. Routing through the shared kernel (rather than
/// `π.powf(t)`) keeps a candidate's border ULP-identical to the cached `μ`.
/// `π = 0` ⇒ `d = +∞` ⇒ `exp(−∞) = 0`.
#[inline]
fn zeta_entry(coupling: f64, t: f64) -> f64 {
    zeta_from_scaled_distance(t * -coupling.ln())
}

/// Paired evaluation `(Mag(S), Mag(S ∪ {candidate}))` at the pinned canonical
/// scale `t = 1`.
///
/// Constructs a [`CoalitionEvaluator`] at `t = 1` and returns its
/// [`base_value`](CoalitionEvaluator::base_value) paired with
/// [`value_with(candidate)`](CoalitionEvaluator::value_with). The base component
/// is `==` exact to [`coalition_value`](crate::coalition_value); the incremental
/// component matches a fresh `coalition_value` on `S ∪ {candidate}` within
/// [`INCREMENTAL_REL_TOL`].
///
/// For a candidate **sweep** against a fixed `S`, build the evaluator once and
/// call [`value_with`](CoalitionEvaluator::value_with) per candidate — this
/// helper rebuilds the cache each call and is only for a single pair.
///
/// # Errors
///
/// Propagates every error of [`CoalitionEvaluator::new`] (invalid members /
/// couplings / probabilities, singular base `ζ`) and of
/// [`CoalitionEvaluator::value_with`] (candidate out of range, candidate already
/// a member, singular bordered `ζ`).
pub fn coalition_value_delta<O>(
    agents: &[O],
    couplings: &[(usize, usize, f64)],
    members: &[usize],
    candidate: usize,
) -> Result<(f64, f64), CatgraphError>
where
    O: Copy + Eq + Hash + Debug + 'static,
{
    let evaluator = CoalitionEvaluator::new(agents, couplings, members, 1.0)?;
    let base = evaluator.base_value();
    let with = evaluator.value_with(candidate)?;
    Ok((base, with))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coalition::coalition_magnitude_from_couplings;
    use catgraph_testutil::Lcg;

    /// Fresh `Mag(S ∪ {candidate})` via the public plain-data path (members in
    /// `S`-then-`candidate` order — magnitude is order-invariant).
    fn fresh_with(
        agents: &[&'static str],
        couplings: &[(usize, usize, f64)],
        members: &[usize],
        candidate: usize,
        t: f64,
    ) -> Result<f64, CatgraphError> {
        let mut members_x = members.to_vec();
        members_x.push(candidate);
        coalition_magnitude_from_couplings(agents, couplings, &members_x, t)
    }

    fn rel_close(a: f64, b: f64) -> bool {
        (a - b).abs() <= INCREMENTAL_REL_TOL * a.abs().max(b.abs()).max(1.0)
    }

    /// Bump `hits`, indexed `[Fast, Slow, SlowNearSingular]`.
    fn tally(path: EvalPath, hits: &mut [usize; 3]) {
        match path {
            EvalPath::Fast => hits[0] += 1,
            EvalPath::Slow => hits[1] += 1,
            EvalPath::SlowNearSingular => hits[2] += 1,
        }
    }

    // -----------------------------------------------------------------------
    // Contract point 1: base value is bit-identical to the fresh free function.
    // -----------------------------------------------------------------------
    #[test]
    fn base_value_bit_identical_to_fresh() {
        // chain, diamond, cyclic fixtures.
        let chain = (
            vec!["a", "b", "c"],
            vec![(0usize, 1usize, 0.7f64), (1, 2, 0.5)],
            vec![0usize, 1, 2],
        );
        let diamond = (
            vec!["a", "b", "c", "d"],
            vec![
                (0usize, 1usize, 0.6f64),
                (0, 2, 0.4),
                (1, 3, 0.5),
                (2, 3, 0.9),
            ],
            vec![0usize, 1, 2, 3],
        );
        let cyclic = (
            vec!["a", "b"],
            vec![(0usize, 1usize, 0.5f64), (1, 0, 0.5)],
            vec![0usize, 1],
        );
        for (agents, couplings, members) in [chain, diamond, cyclic] {
            for t in [1.0_f64, 2.0] {
                let ev = CoalitionEvaluator::new(&agents, &couplings, &members, t).unwrap();
                let fresh =
                    coalition_magnitude_from_couplings(&agents, &couplings, &members, t).unwrap();
                assert_eq!(
                    ev.base_value(),
                    fresh,
                    "base value must be bit-identical to fresh at t = {t}"
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // Fast path: candidate weakly coupled to a single member — no interior
    // improvement, no merge.
    // -----------------------------------------------------------------------
    #[test]
    fn fast_path_weak_single_coupling() {
        let agents = ["a", "b", "c", "x"];
        // chain a→b→c plus weak x↔c (0.2 both ways).
        let couplings = [
            (0usize, 1usize, 0.7f64),
            (1, 2, 0.5),
            (2, 3, 0.2),
            (3, 2, 0.2),
        ];
        let members = [0usize, 1, 2];
        let t = 1.0;
        let ev = CoalitionEvaluator::new(&agents, &couplings, &members, t).unwrap();
        let (inc, path) = ev.value_with_impl(3, &mut EvalScratch::new()).unwrap();
        assert_eq!(
            path,
            EvalPath::Fast,
            "weak single coupling must take fast path"
        );
        let fresh = fresh_with(&agents, &couplings, &members, 3, t).unwrap();
        assert!(rel_close(inc, fresh), "fast: inc {inc} vs fresh {fresh}");
    }

    // -----------------------------------------------------------------------
    // Slow path via interior improvement: x strongly bridges two weakly-coupled
    // members.
    // -----------------------------------------------------------------------
    #[test]
    fn slow_path_interior_improvement() {
        let agents = ["a", "b", "x"];
        // a→b weak (0.1); x strongly links a→x→b (0.99·0.99 = 0.98 ≫ 0.1).
        let couplings = [(0usize, 1usize, 0.1f64), (0, 2, 0.99), (2, 1, 0.99)];
        let members = [0usize, 1];
        let t = 1.0;
        let ev = CoalitionEvaluator::new(&agents, &couplings, &members, t).unwrap();
        let (inc, path) = ev.value_with_impl(2, &mut EvalScratch::new()).unwrap();
        assert_eq!(
            path,
            EvalPath::Slow,
            "bridging improvement must take slow path"
        );
        let fresh = fresh_with(&agents, &couplings, &members, 2, t).unwrap();
        assert!(
            rel_close(inc, fresh),
            "slow-improve: inc {inc} vs fresh {fresh}"
        );
    }

    // -----------------------------------------------------------------------
    // Slow path via skeletal merge: x is a mutual-1.0 clone of a member.
    // -----------------------------------------------------------------------
    #[test]
    fn slow_path_skeletal_merge() {
        let agents = ["a", "b", "x"];
        // a→b 0.5; x ⇄ b at 1.0 (perfect clone of b) — skeleton must shrink.
        let couplings = [(0usize, 1usize, 0.5f64), (1, 2, 1.0), (2, 1, 1.0)];
        let members = [0usize, 1];
        let t = 1.0;
        let ev = CoalitionEvaluator::new(&agents, &couplings, &members, t).unwrap();
        let (inc, path) = ev.value_with_impl(2, &mut EvalScratch::new()).unwrap();
        assert_eq!(path, EvalPath::Slow, "mutual-1.0 clone must take slow path");
        let fresh = fresh_with(&agents, &couplings, &members, 2, t).unwrap();
        assert!(
            rel_close(inc, fresh),
            "slow-merge: inc {inc} vs fresh {fresh}"
        );
        // The clone collapses: {a,b,x} has the same effective size as {a,b}.
        let base = ev.base_value();
        assert!(
            rel_close(inc, base),
            "clone of b adds no diversity: {inc} vs base {base}"
        );
    }

    // -----------------------------------------------------------------------
    // Near-singular Schur diversion: the fast branch is entered and then
    // diverted by the `SCHUR_SLOW_FALLBACK_TOL` guard.
    // -----------------------------------------------------------------------

    /// Two uncoupled members give `ζ_S = I`, so the bordered Schur complement is
    /// `s = 1 − ζ(c₀)·ζ(r₀)` on the candidate's border alone. With
    /// `c₀ = 1 − 5e-13` and `r₀ = 1.0` the branch tests are both false
    /// (`c₀ ≠ 1.0` for the merge test, no `i ≠ j` shortcut for the interior
    /// test), so the fast branch is entered, and `s ≈ 5e-13` is inside
    /// `SCHUR_SLOW_FALLBACK_TOL · (1 + |vᵀμu|) ≈ 2e-12`. The control fixture is
    /// the same shape at `c₀ = 0.5`.
    #[test]
    fn near_singular_border_diverts_to_slow() {
        let agents = ["m0", "m1", "x"];
        let members = [0usize, 1];
        let t = 1.0;

        let near = [(0usize, 2usize, 1.0 - 5e-13), (2, 0, 1.0)];
        let ev = CoalitionEvaluator::new(&agents, &near, &members, t).unwrap();
        let rep = ev.value_with_report(2).unwrap();
        assert_eq!(
            rep.path(),
            EvalPath::SlowNearSingular,
            "a border with s ≈ 5e-13 must be diverted out of the fast branch"
        );
        assert_eq!(
            rep.schur_complement(),
            None,
            "the diversion reports no Schur complement"
        );
        assert_eq!(
            rep.zero_proof(),
            None,
            "the diversion carries no zero-diversity proof"
        );
        assert!(
            rep.value().is_finite(),
            "diverted value must be finite, got {}",
            rep.value()
        );
        let fresh = fresh_with(&agents, &near, &members, 2, t).unwrap();
        assert!(
            rel_close(rep.value(), fresh),
            "diverted {} vs fresh {fresh}",
            rep.value()
        );

        let far = [(0usize, 2usize, 0.5f64), (2, 0, 1.0)];
        let ev_far = CoalitionEvaluator::new(&agents, &far, &members, t).unwrap();
        let rep_far = ev_far.value_with_report(2).unwrap();
        assert_eq!(
            rep_far.path(),
            EvalPath::Fast,
            "the same shape at c₀ = 0.5 stays on the fast branch"
        );
        let s_far = rep_far
            .schur_complement()
            .expect("invariant: the fast branch reports a Schur complement");
        assert!(
            (s_far - 0.5).abs() <= 4.0 * f64::EPSILON,
            "control fixture: s = 1 − 0.5·1.0, got {s_far} (|Δ| = {} > 4 ε = {})",
            (s_far - 0.5).abs(),
            4.0 * f64::EPSILON
        );
    }

    // -----------------------------------------------------------------------
    // Deterministic seeded grid over m ∈ 2..=10 pools with several candidates.
    // Asserts fresh/incremental error-parity AND value equality within
    // tolerance for every (S, x) — this also exercises the singular branch if
    // any grid point hits it (see the report note: an exact post-skeletal
    // singular ζ is not hand-constructible).
    //
    // Also carries the rank-order identity (contract point 3) as a per-`(S, t)`
    // argsort comparison, and the per-`EvalPath` hit counts over the grid.
    // -----------------------------------------------------------------------
    #[test]
    fn seeded_grid_fresh_vs_incremental() {
        // A fixed 12-agent pool named s0..s11.
        const NAMES: [&str; 12] = [
            "s0", "s1", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11",
        ];
        // `| 1` (seed prep) stays at the call site — see catgraph-testutil (#33).
        let mut lcg = Lcg::new(0xC0FFEE | 1);
        let n = NAMES.len();
        let mut hits = [0usize; 3];
        let mut tight_pairs = 0usize;

        for m in 2..=10usize {
            // Random dense coupling table over all 12 agents (some structure so
            // both branches arise): each ordered pair gets a coupling with 60%
            // probability, value in (0, 1]; occasionally 1.0 to force merges.
            let mut couplings: Vec<(usize, usize, f64)> = Vec::new();
            for i in 0..n {
                for j in 0..n {
                    if i == j {
                        continue;
                    }
                    if lcg.next_f64() < 0.6 {
                        let mut p = lcg.next_f64();
                        if p == 0.0 {
                            p = 0.01;
                        }
                        // ~8% of edges snap to 1.0 to provoke skeletal merges.
                        if lcg.next_f64() < 0.08 {
                            p = 1.0;
                        }
                        couplings.push((i, j, p));
                    }
                }
            }

            let members: Vec<usize> = (0..m).collect();
            for t in [1.0_f64, 2.0] {
                let ev = match CoalitionEvaluator::new(&NAMES, &couplings, &members, t) {
                    Ok(ev) => ev,
                    Err(_) => continue, // singular base — skip this (S, t)
                };
                // `(incremental, fresh)` for the candidates that evaluated on
                // both routes — the rank-order population for this `(S, t)`.
                let mut ranked: Vec<(f64, f64)> = Vec::new();
                for candidate in m..n {
                    let inc = ev.value_with(candidate);
                    let fresh = fresh_with(&NAMES, &couplings, &members, candidate, t);
                    assert_eq!(
                        inc.is_ok(),
                        fresh.is_ok(),
                        "m={m} t={t} cand={candidate}: error-parity fresh/incremental"
                    );
                    if let (Ok(inc), Ok(fresh)) = (inc, fresh) {
                        assert!(
                            rel_close(inc, fresh),
                            "m={m} t={t} cand={candidate}: inc {inc} vs fresh {fresh}"
                        );
                        let (_, path) = ev
                            .value_with_impl(candidate, &mut EvalScratch::new())
                            .expect("invariant: value_with just succeeded on this candidate");
                        tally(path, &mut hits);
                        ranked.push((inc, fresh));
                    }
                }

                // Rank-order identity (contract point 3) on this candidate
                // population — argsort by incremental == argsort by fresh. The
                // `tight_pairs` counter below states over how many pairs this
                // adds to the per-candidate value assertion.
                let mut order_inc: Vec<usize> = (0..ranked.len()).collect();
                let mut order_fresh: Vec<usize> = (0..ranked.len()).collect();
                order_inc.sort_by(|&a, &b| ranked[a].0.total_cmp(&ranked[b].0));
                order_fresh.sort_by(|&a, &b| ranked[a].1.total_cmp(&ranked[b].1));
                assert_eq!(
                    order_inc, order_fresh,
                    "m={m} t={t}: incremental ranking {order_inc:?} vs fresh {order_fresh:?} \
                     over {ranked:?}"
                );

                // Pairs closer than twice the per-candidate tolerance: the ones
                // whose order the value assertion above does not already fix.
                for a in 0..ranked.len() {
                    for b in (a + 1)..ranked.len() {
                        let (_, fresh_a) = ranked[a];
                        let (_, fresh_b) = ranked[b];
                        let sep =
                            2.0 * INCREMENTAL_REL_TOL * fresh_a.abs().max(fresh_b.abs()).max(1.0);
                        if (fresh_a - fresh_b).abs() <= sep {
                            tight_pairs += 1;
                        }
                    }
                }
            }
        }

        assert_eq!(
            hits,
            [44usize, 64, 0],
            "EvalPath hits [Fast, Slow, SlowNearSingular] over the seeded grid"
        );
        assert_eq!(
            tight_pairs, 0,
            "candidate pairs within 2·INCREMENTAL_REL_TOL on the fresh route — the \
             population over which the ranking assertion is not already implied by \
             the per-candidate value assertion"
        );
    }

    // -----------------------------------------------------------------------
    // Rank-order identity (contract point 3): a fixed S, ≥5 candidates with
    // distinct values, argsort by fresh == argsort by incremental. Every
    // candidate here takes the fast path (asserted below); the same identity
    // over a population containing both routes is carried by
    // `seeded_grid_fresh_vs_incremental`.
    // -----------------------------------------------------------------------
    #[test]
    fn rank_order_identity() {
        let agents = ["m0", "m1", "m2", "c0", "c1", "c2", "c3", "c4", "c5"];
        // Base S = {m0, m1, m2}: a weak chain.
        let mut couplings = vec![(0usize, 1usize, 0.4f64), (1, 2, 0.3)];
        // Six candidates coupled to m0 with strictly-increasing strength ⇒
        // distinct Mag(S ∪ {c}).
        let cand_probs = [0.15, 0.30, 0.45, 0.60, 0.75, 0.90];
        for (k, &p) in cand_probs.iter().enumerate() {
            let c = 3 + k;
            couplings.push((0, c, p));
            couplings.push((c, 0, p));
        }
        let members = [0usize, 1, 2];
        let t = 1.0;
        let ev = CoalitionEvaluator::new(&agents, &couplings, &members, t).unwrap();

        let candidates: Vec<usize> = (3..9).collect();
        let inc: Vec<f64> = candidates
            .iter()
            .map(|&c| ev.value_with(c).unwrap())
            .collect();
        let fresh: Vec<f64> = candidates
            .iter()
            .map(|&c| fresh_with(&agents, &couplings, &members, c, t).unwrap())
            .collect();

        // Distinct values (so the ranking is unambiguous).
        for a in 0..inc.len() {
            for b in (a + 1)..inc.len() {
                assert!(
                    (fresh[a] - fresh[b]).abs() > 1e-6,
                    "candidates must be distinguishable"
                );
            }
        }

        let mut order_inc: Vec<usize> = (0..inc.len()).collect();
        let mut order_fresh: Vec<usize> = (0..fresh.len()).collect();
        order_inc.sort_by(|&a, &b| inc[a].partial_cmp(&inc[b]).unwrap());
        order_fresh.sort_by(|&a, &b| fresh[a].partial_cmp(&fresh[b]).unwrap());
        assert_eq!(
            order_inc, order_fresh,
            "incremental ranking must equal fresh ranking"
        );

        let mut hits = [0usize; 3];
        for &c in &candidates {
            let (_, path) = ev.value_with_impl(c, &mut EvalScratch::new()).unwrap();
            tally(path, &mut hits);
        }
        assert_eq!(
            hits,
            [6usize, 0, 0],
            "EvalPath hits [Fast, Slow, SlowNearSingular] over this fixture's candidates"
        );
    }

    // -----------------------------------------------------------------------
    // coalition_value_delta: base exact, incremental within tolerance vs two
    // fresh coalition_value calls.
    // -----------------------------------------------------------------------
    #[test]
    fn value_delta_matches_two_fresh_calls() {
        use crate::coalition_value;
        let agents = ["a", "b", "c", "x"];
        let couplings = [
            (0usize, 1usize, 0.7f64),
            (1, 2, 0.5),
            (0, 3, 0.6),
            (3, 1, 0.4),
        ];
        let members = [0usize, 1, 2];
        let (base, with) = coalition_value_delta(&agents, &couplings, &members, 3).unwrap();

        let fresh_base = coalition_value(&agents, &couplings, &members).unwrap();
        let mut members_x = members.to_vec();
        members_x.push(3);
        let fresh_with = coalition_value(&agents, &couplings, &members_x).unwrap();

        assert_eq!(
            base, fresh_base,
            "delta base must be bit-identical to coalition_value(S)"
        );
        assert!(
            rel_close(with, fresh_with),
            "delta with {with} vs fresh {fresh_with}"
        );
    }

    // -----------------------------------------------------------------------
    // t ≠ 1 equality (t = 2.0) on a diamond fixture.
    // -----------------------------------------------------------------------
    #[test]
    fn incremental_at_t2() {
        let agents = ["a", "b", "c", "d", "x"];
        let couplings = [
            (0usize, 1usize, 0.6f64),
            (0, 2, 0.4),
            (1, 3, 0.5),
            (2, 3, 0.9),
            (3, 4, 0.3),
            (4, 3, 0.3),
        ];
        let members = [0usize, 1, 2, 3];
        let t = 2.0;
        let ev = CoalitionEvaluator::new(&agents, &couplings, &members, t).unwrap();
        let inc = ev.value_with(4).unwrap();
        let fresh = fresh_with(&agents, &couplings, &members, 4, t).unwrap();
        assert!(rel_close(inc, fresh), "t=2: inc {inc} vs fresh {fresh}");
    }

    // -----------------------------------------------------------------------
    // Error cases: candidate already a member, candidate out of range.
    // -----------------------------------------------------------------------
    #[test]
    fn error_cases() {
        let agents = ["a", "b", "c"];
        let couplings = [(0usize, 1usize, 0.7f64), (1, 2, 0.5)];
        let members = [0usize, 1];
        let ev = CoalitionEvaluator::new(&agents, &couplings, &members, 1.0).unwrap();

        assert!(
            ev.value_with(0).is_err(),
            "candidate already a member must error"
        );
        assert!(
            ev.value_with(1).is_err(),
            "candidate already a member must error"
        );
        assert!(
            ev.value_with(3).is_err(),
            "candidate out of range must error"
        );

        // Construction-time validation mirrors coalition_magnitude_from_couplings.
        assert!(
            CoalitionEvaluator::new(&agents, &[(0, 9, 0.5)], &members, 1.0).is_err(),
            "out-of-range coupling must error"
        );
        assert!(
            CoalitionEvaluator::new(&agents, &[(1, 1, 0.5)], &members, 1.0).is_err(),
            "self-coupling must error"
        );
        assert!(
            CoalitionEvaluator::new(&agents, &[(0, 1, 1.5)], &members, 1.0).is_err(),
            "out-of-[0,1] probability must error"
        );
        assert!(
            CoalitionEvaluator::new(&agents, &couplings, &[9], 1.0).is_err(),
            "out-of-range member must error"
        );
    }

    // -----------------------------------------------------------------------
    // Isolated candidate (no couplings) adds exactly 1 to the magnitude — a
    // fresh new point at infinite distance. Sanity-checks the fast-path corner
    // (p = q = 0, s = 1).
    // -----------------------------------------------------------------------
    #[test]
    fn isolated_candidate_adds_one() {
        let agents = ["a", "b", "x"];
        let couplings = [(0usize, 1usize, 0.5f64), (1, 0, 0.5)];
        let members = [0usize, 1];
        let ev = CoalitionEvaluator::new(&agents, &couplings, &members, 1.0).unwrap();
        let (inc, path) = ev.value_with_impl(2, &mut EvalScratch::new()).unwrap();
        assert_eq!(path, EvalPath::Fast);
        let fresh = fresh_with(&agents, &couplings, &members, 2, 1.0).unwrap();
        assert!(rel_close(inc, fresh));
        assert!(
            rel_close(inc, ev.base_value() + 1.0),
            "isolated point adds exactly 1"
        );
    }

    // -----------------------------------------------------------------------
    // Scratch buffers: `value_with_scratch` is bit-identical to `value_with`,
    // a reused scratch is contamination-free across a candidate sweep, and error
    // cases still error identically.
    // -----------------------------------------------------------------------

    /// The dense seeded grid (same shape as `seeded_grid_fresh_vs_incremental`)
    /// but comparing the scratch path against the allocating path with `==`
    /// (exact), not a tolerance. The per-`EvalPath` hit counts asserted at the
    /// end state which routes the grid takes, tallied on a fresh scratch per
    /// candidate.
    #[test]
    fn value_with_scratch_bit_identical_to_value_with() {
        const NAMES: [&str; 12] = [
            "s0", "s1", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11",
        ];
        // `| 1` (seed prep) stays at the call site — see catgraph-testutil (#33).
        let mut lcg = Lcg::new(0xC0FFEE | 1);
        let n = NAMES.len();
        let mut hits = [0usize; 3];

        for m in 2..=10usize {
            let mut couplings: Vec<(usize, usize, f64)> = Vec::new();
            for i in 0..n {
                for j in 0..n {
                    if i == j {
                        continue;
                    }
                    if lcg.next_f64() < 0.6 {
                        let mut p = lcg.next_f64();
                        if p == 0.0 {
                            p = 0.01;
                        }
                        if lcg.next_f64() < 0.08 {
                            p = 1.0;
                        }
                        couplings.push((i, j, p));
                    }
                }
            }

            let members: Vec<usize> = (0..m).collect();
            for t in [1.0_f64, 2.0] {
                let ev = match CoalitionEvaluator::new(&NAMES, &couplings, &members, t) {
                    Ok(ev) => ev,
                    Err(_) => continue,
                };
                // A single scratch reused across the whole candidate sweep.
                let mut scratch = EvalScratch::new();
                for candidate in m..n {
                    let plain = ev.value_with(candidate);
                    let scr = ev.value_with_scratch(candidate, &mut scratch);
                    assert_eq!(
                        plain.is_ok(),
                        scr.is_ok(),
                        "m={m} t={t} cand={candidate}: error-parity scratch/plain"
                    );
                    if let (Ok(plain), Ok(scr)) = (plain, scr) {
                        assert_eq!(
                            plain, scr,
                            "m={m} t={t} cand={candidate}: scratch must be bit-identical"
                        );
                        let (_, path) = ev
                            .value_with_impl(candidate, &mut EvalScratch::new())
                            .expect("invariant: value_with just succeeded on this candidate");
                        tally(path, &mut hits);
                    }
                }
            }
        }

        assert_eq!(
            hits,
            [44usize, 64, 0],
            "EvalPath hits [Fast, Slow, SlowNearSingular] tallied on a fresh scratch per candidate"
        );
    }

    /// A reused scratch (fed a fast-path candidate, then a slow-path candidate,
    /// then the fast-path candidate again) yields the *same* value for the
    /// fast-path candidate as a pristine scratch — i.e. the intervening
    /// slow-path call left no residue.
    #[test]
    fn reused_scratch_no_cross_call_contamination() {
        // a→b→c chain; x2 (idx 3) weakly single-coupled ⇒ fast; x3 (idx 4)
        // strongly bridges a↔c ⇒ slow (interior improvement).
        let agents = ["a", "b", "c", "x2", "x3"];
        let couplings = [
            (0usize, 1usize, 0.7f64),
            (1, 2, 0.5),
            (2, 3, 0.2),
            (3, 2, 0.2),
            (0, 4, 0.99),
            (4, 2, 0.99),
        ];
        let members = [0usize, 1, 2];
        let ev = CoalitionEvaluator::new(&agents, &couplings, &members, 1.0).unwrap();

        let pristine = ev.value_with_scratch(3, &mut EvalScratch::new()).unwrap();

        let mut scratch = EvalScratch::new();
        let first = ev.value_with_scratch(3, &mut scratch).unwrap();
        let _slow = ev.value_with_scratch(4, &mut scratch).unwrap();
        let again = ev.value_with_scratch(3, &mut scratch).unwrap();

        assert_eq!(first, pristine, "first reuse must match a pristine scratch");
        assert_eq!(
            again, pristine,
            "fast-path value after an intervening slow-path call must be unchanged"
        );
    }

    /// A scratch reused across evaluators of *different* member counts stays
    /// correct — `resize` + full overwrite handles the size change.
    #[test]
    fn reused_scratch_across_differently_sized_coalitions() {
        let agents = ["a", "b", "c", "d", "x"];
        let couplings = [
            (0usize, 1usize, 0.6f64),
            (1, 2, 0.5),
            (2, 3, 0.4),
            (3, 4, 0.3),
            (4, 3, 0.3),
        ];
        let big_members = [0usize, 1, 2, 3];
        let small_members = [0usize, 1];
        let ev_big = CoalitionEvaluator::new(&agents, &couplings, &big_members, 1.0).unwrap();
        let ev_small = CoalitionEvaluator::new(&agents, &couplings, &small_members, 1.0).unwrap();

        let mut scratch = EvalScratch::new();
        // Serve the large coalition first (grows the buffers), then the small.
        let _ = ev_big.value_with_scratch(4, &mut scratch).unwrap();
        let small_reused = ev_small.value_with_scratch(4, &mut scratch);
        let small_fresh = ev_small.value_with_scratch(4, &mut EvalScratch::new());
        assert_eq!(small_reused.is_ok(), small_fresh.is_ok());
        if let (Ok(a), Ok(b)) = (small_reused, small_fresh) {
            assert_eq!(a, b, "shrinking the served coalition must not contaminate");
        }
    }

    /// Error cases error identically through the scratch entry point.
    #[test]
    fn value_with_scratch_error_parity() {
        let agents = ["a", "b", "c"];
        let couplings = [(0usize, 1usize, 0.7f64), (1, 2, 0.5)];
        let members = [0usize, 1];
        let ev = CoalitionEvaluator::new(&agents, &couplings, &members, 1.0).unwrap();
        let mut scratch = EvalScratch::new();
        assert!(ev.value_with_scratch(0, &mut scratch).is_err(), "member");
        assert!(ev.value_with_scratch(1, &mut scratch).is_err(), "member");
        assert!(
            ev.value_with_scratch(3, &mut scratch).is_err(),
            "out of range"
        );
        // A well-formed call after the error calls still succeeds (no poisoning).
        assert!(ev.value_with_scratch(2, &mut scratch).is_ok());
    }

    // -----------------------------------------------------------------------
    // The reporting API: `value_with_report(_scratch)` → `JoinReport` with an
    // exact `ZeroDiversityProof`.
    // -----------------------------------------------------------------------

    /// The 12-agent pool the seeded corpora are drawn over.
    const REPORT_NAMES: [&str; 12] = [
        "s0", "s1", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11",
    ];

    /// One iteration's coupling table for the seeded grid, drawing from `lcg` in
    /// exactly the order `seeded_grid_fresh_vs_incremental` /
    /// `value_with_scratch_bit_identical_to_value_with` draw it (60% density,
    /// value in `(0, 1]`, `snap` of them snapped to `1.0` to provoke skeletal
    /// merges). `snap = 0.08` reproduces the crate's own grid; `snap ∈ {0.25,
    /// 0.45}` is where the merge-heavy population lives.
    fn seeded_couplings(lcg: &mut Lcg, n: usize, snap: f64) -> Vec<(usize, usize, f64)> {
        let mut couplings: Vec<(usize, usize, f64)> = Vec::new();
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }
                if lcg.next_f64() < 0.6 {
                    let mut p = lcg.next_f64();
                    if p == 0.0 {
                        p = 0.01;
                    }
                    if lcg.next_f64() < snap {
                        p = 1.0;
                    }
                    couplings.push((i, j, p));
                }
            }
        }
        couplings
    }

    /// The report entry points are purely additive: `value()` is **bit-identical**
    /// to `value_with`, the scratch variant is bit-identical to the allocating
    /// one (report fields included), `base()` is the evaluator's cached base, and
    /// `schur_complement()` is `Some` exactly on the fast path.
    #[test]
    fn value_with_report_parity() {
        // `| 1` (seed prep) stays at the call site — see catgraph-testutil (#33).
        let mut lcg = Lcg::new(0xC0FFEE | 1);
        let n = REPORT_NAMES.len();
        // One scratch reused across the whole sweep.
        let mut scratch = EvalScratch::new();

        for m in 2..=10usize {
            let couplings = seeded_couplings(&mut lcg, n, 0.08);
            let members: Vec<usize> = (0..m).collect();
            for t in [1.0_f64, 2.0] {
                let ev = match CoalitionEvaluator::new(&REPORT_NAMES, &couplings, &members, t) {
                    Ok(ev) => ev,
                    Err(_) => continue, // singular base — skip this (S, t)
                };
                for candidate in m..n {
                    let plain = ev.value_with(candidate);
                    let rep = ev.value_with_report(candidate);
                    let rep_scr = ev.value_with_report_scratch(candidate, &mut scratch);
                    assert_eq!(
                        plain.is_ok(),
                        rep.is_ok(),
                        "m={m} t={t} cand={candidate}: error-parity report/value_with"
                    );
                    assert_eq!(
                        plain.is_ok(),
                        rep_scr.is_ok(),
                        "m={m} t={t} cand={candidate}: error-parity report-scratch/value_with"
                    );
                    if let (Ok(plain), Ok(rep), Ok(rep_scr)) = (plain, rep, rep_scr) {
                        assert_eq!(
                            plain,
                            rep.value(),
                            "m={m} t={t} cand={candidate}: report value must be bit-identical"
                        );
                        assert_eq!(
                            rep.value(),
                            rep_scr.value(),
                            "m={m} t={t} cand={candidate}: scratch report must be bit-identical"
                        );
                        assert_eq!(rep.zero_proof(), rep_scr.zero_proof(), "proof parity");
                        assert_eq!(rep.path(), rep_scr.path(), "path parity");
                        assert_eq!(
                            rep.schur_complement(),
                            rep_scr.schur_complement(),
                            "schur parity"
                        );
                        assert_eq!(
                            rep.base(),
                            ev.base_value(),
                            "report base is the cached base"
                        );
                        assert_eq!(
                            rep.increment(),
                            rep.value() - rep.base(),
                            "increment is value − base"
                        );
                        assert_eq!(
                            rep.schur_complement().is_some(),
                            rep.path() == EvalPath::Fast,
                            "m={m} t={t} cand={candidate}: schur_complement is Some iff fast"
                        );
                    }
                }
            }
        }
    }

    /// Error parity of the report entry points with `value_with` — same cases,
    /// and a well-formed call after the failures still succeeds (no poisoning).
    #[test]
    fn value_with_report_error_parity() {
        let agents = ["a", "b", "c"];
        let couplings = [(0usize, 1usize, 0.7f64), (1, 2, 0.5)];
        let members = [0usize, 1];
        let ev = CoalitionEvaluator::new(&agents, &couplings, &members, 1.0).unwrap();
        let mut scratch = EvalScratch::new();

        for bad in [0usize, 1, 3] {
            assert_eq!(
                ev.value_with(bad).is_err(),
                ev.value_with_report(bad).is_err(),
                "candidate {bad}: report must error exactly when value_with does"
            );
            assert!(
                ev.value_with_report_scratch(bad, &mut scratch).is_err(),
                "candidate {bad}: report-scratch must error"
            );
        }
        assert!(ev.value_with_report(2).is_ok());
        assert!(ev.value_with_report_scratch(2, &mut scratch).is_ok());
    }

    /// Merge-only (`skeletal_merge ∧ ¬interior_improvement`) — the exact
    /// zero-diversity predicate. Same fixture as `slow_path_skeletal_merge`:
    /// `a→b` at `0.5`, `x ⇄ b` at `1.0`.
    #[test]
    fn report_merge_only_proof() {
        let agents = ["a", "b", "x"];
        let couplings = [(0usize, 1usize, 0.5f64), (1, 2, 1.0), (2, 1, 1.0)];
        let members = [0usize, 1];
        let t = 1.0;
        let ev = CoalitionEvaluator::new(&agents, &couplings, &members, t).unwrap();
        let rep = ev.value_with_report(2).unwrap();

        assert_eq!(
            rep.path(),
            EvalPath::Slow,
            "mutual-1.0 clone takes the slow path"
        );
        assert_eq!(
            rep.schur_complement(),
            None,
            "no Schur complement on the slow route"
        );
        assert_eq!(
            rep.zero_proof(),
            Some(ZeroDiversityProof::SkeletalMerge { member: 1 }),
            "x is a perfect clone of member 1 and opens no shortcut"
        );
        assert!(rep.is_provably_zero());
        // Memo §4: all 268 merge-only candidates are exactly zero on the
        // incremental route AND bit-identical to fresh (max rel. deviation 0.0).
        assert_eq!(rep.increment(), 0.0, "merge-only increment is exactly 0.0");
        assert_eq!(rep.value(), rep.base(), "merge-only value == base, bitwise");
        let fresh = fresh_with(&agents, &couplings, &members, 2, t).unwrap();
        assert_eq!(rep.value(), fresh, "merge-only is fresh-bitwise-equal");
    }

    /// Merge **∧** interior improvement: `x` is a mutual-`1.0` clone of member
    /// `b` but also bridges the weakly-coupled `a → c`, so the closure changes
    /// and the magnitude moves. No proof may be claimed here.
    #[test]
    fn report_merge_and_interior_carries_no_proof() {
        let agents = ["a", "b", "c", "x"];
        let couplings = [
            (0usize, 1usize, 0.3f64), // weak chain a→b
            (1, 2, 0.3),              // weak chain b→c  (so closed(a→c) = 0.09)
            (1, 3, 1.0),              // x ⇄ b at 1.0 — perfect clone of b
            (3, 1, 1.0),
            (0, 3, 0.95), // …but a→x→c is 0.9025 ≫ 0.09: a genuine shortcut
            (3, 2, 0.95),
        ];
        let members = [0usize, 1, 2];
        let t = 1.0;
        let ev = CoalitionEvaluator::new(&agents, &couplings, &members, t).unwrap();
        let (_, path) = ev.value_with_impl(3, &mut EvalScratch::new()).unwrap();
        assert_eq!(path, EvalPath::Slow, "merge ∧ interior takes the slow path");

        let rep = ev.value_with_report(3).unwrap();
        assert_eq!(
            rep.zero_proof(),
            None,
            "merge ∧ interior must NOT claim zero diversity"
        );
        assert!(!rep.is_provably_zero());
        assert!(
            rep.increment().abs() > 1e-6,
            "the shortcut genuinely moves the magnitude: increment {}",
            rep.increment()
        );
        let fresh = fresh_with(&agents, &couplings, &members, 3, t).unwrap();
        assert!(
            rel_close(rep.value(), fresh),
            "merge ∧ interior: inc {} vs fresh {fresh}",
            rep.value()
        );
    }

    /// Incoming profile duplicate. A single `1.0` coupling
    /// `a → x` with `x` otherwise silent makes the border `c` replicate member
    /// `a`'s closed **column** bitwise, so `u = ζ_S·e_a` and `p = 1` exactly.
    /// Second sub-case: `x` also carries a *dominated* outgoing coupling, so
    /// `q ≠ 1` and `s ≠ 1` — the proof must still fire and still certify zero.
    #[test]
    fn report_incoming_profile_duplicate() {
        let agents = ["a", "b", "c", "x"];
        let base_couplings = [(0usize, 1usize, 0.7f64), (1, 2, 0.5), (0, 3, 1.0)];
        let dominated = [
            (0usize, 1usize, 0.7f64),
            (1, 2, 0.5),
            (0, 3, 1.0),
            (3, 2, 0.1), // x→c is dominated by the existing a→b→c path
        ];
        let members = [0usize, 1, 2];
        let t = 1.0;

        for (label, couplings) in [
            ("silent-x", &base_couplings[..]),
            ("dominated-outgoing", &dominated[..]),
        ] {
            let ev = CoalitionEvaluator::new(&agents, couplings, &members, t).unwrap();
            let rep = ev.value_with_report(3).unwrap();

            assert_eq!(
                rep.path(),
                EvalPath::Fast,
                "{label}: must take the fast path"
            );
            assert!(
                rep.schur_complement().is_some(),
                "{label}: fast path reports s"
            );
            assert_eq!(
                rep.zero_proof(),
                Some(ZeroDiversityProof::IncomingProfileDuplicate { member: 0 }),
                "{label}: c replicates member 0's closed column"
            );
            assert!(rep.is_provably_zero());
            assert!(
                rep.increment().abs() <= 1e-12,
                "{label}: computed increment {} must be roundoff-small \
                 (measured on both fixtures: exactly 0.0)",
                rep.increment()
            );
            // The certified fact, checked against a genuinely fresh evaluation
            // of S ∪ {x}: the real increment is exactly 0. Asserted at 1e-12
            // *relative*, not bitwise — measured here, `silent-x` is in fact
            // fresh-bitwise-equal to the base but `dominated-outgoing` is
            // 2.22e-16 absolute (1.23e-16 relative) away, since the fresh route
            // inverts the full (m+1)×(m+1) ζ′ rather than bordering.
            let fresh = fresh_with(&agents, couplings, &members, 3, t).unwrap();
            let base = ev.base_value();
            assert!(
                (fresh - base).abs() <= 1e-12 * base.abs().max(1.0),
                "{label}: fresh Mag(S ∪ {{x}}) {fresh} must equal base {base}"
            );
        }
    }

    /// Outgoing profile duplicate — the transpose fixture: a single `1.0`
    /// coupling `x → a` with `x` otherwise silent makes `r` replicate member
    /// `a`'s closed **row**, so `v = e_aᵀζ_S` and `q = 1` exactly.
    #[test]
    fn report_outgoing_profile_duplicate() {
        let agents = ["a", "b", "c", "x"];
        let couplings = [(0usize, 1usize, 0.7f64), (1, 2, 0.5), (3, 0, 1.0)];
        let members = [0usize, 1, 2];
        let t = 1.0;
        let ev = CoalitionEvaluator::new(&agents, &couplings, &members, t).unwrap();
        let rep = ev.value_with_report(3).unwrap();

        assert_eq!(rep.path(), EvalPath::Fast);
        assert_eq!(
            rep.zero_proof(),
            Some(ZeroDiversityProof::OutgoingProfileDuplicate { member: 0 }),
            "r replicates member 0's closed row"
        );
        assert!(rep.is_provably_zero());
        assert!(
            rep.increment().abs() <= 1e-12,
            "computed increment {} must be roundoff-small",
            rep.increment()
        );
        let fresh = fresh_with(&agents, &couplings, &members, 3, t).unwrap();
        let base = ev.base_value();
        assert!(
            (fresh - base).abs() <= 1e-12 * base.abs().max(1.0),
            "fresh Mag(S ∪ {{x}}) {fresh} must equal base {base} (within 1e-12 relative)"
        );
    }

    /// Soundness sweep over the seeded corpus: every
    /// proof-carrying candidate really is a zero — a genuinely fresh
    /// `Mag(S ∪ {x})` sits within `1e-12` relative of `Mag(S)` — and no proof is
    /// ever attached to a route that cannot carry it (duplicates are fast-path
    /// only; the slow path carries at most a merge proof).
    #[test]
    fn report_proof_soundness_sweep() {
        let n = REPORT_NAMES.len();
        let mut merge_proofs = 0usize;
        let mut incoming_proofs = 0usize;
        let mut outgoing_proofs = 0usize;

        // The crate's own grid plus three independent snap-to-1.0 tables; the
        // higher snap rates populate the merge-only class.
        // `| 1` (seed prep) stays at the call site — see catgraph-testutil (#33).
        for (seed, snap) in [
            (0x00C0_FFEE_u64, 0.08_f64),
            (0x0153_BEEF, 0.08),
            (0x0153_CAFE, 0.25),
            (0x0153_D00D, 0.45),
        ] {
            let mut lcg = Lcg::new(seed | 1);
            for m in 2..=10usize {
                let couplings = seeded_couplings(&mut lcg, n, snap);
                let members: Vec<usize> = (0..m).collect();
                for t in [1.0_f64, 2.0] {
                    let ev = match CoalitionEvaluator::new(&REPORT_NAMES, &couplings, &members, t) {
                        Ok(ev) => ev,
                        Err(_) => continue,
                    };
                    let base = ev.base_value();
                    for candidate in m..n {
                        let Ok(rep) = ev.value_with_report(candidate) else {
                            continue;
                        };
                        let Some(proof) = rep.zero_proof() else {
                            continue;
                        };
                        match proof {
                            ZeroDiversityProof::SkeletalMerge { member } => {
                                merge_proofs += 1;
                                assert!(member < m, "merge witness must be a member index");
                                assert_eq!(
                                    rep.path(),
                                    EvalPath::Slow,
                                    "seed={seed:#x} m={m} t={t} cand={candidate}: \
                                     merge proof is slow-route only"
                                );
                                // Memo §4: merge-only candidates are exactly
                                // zero on the incremental route (268/268 there;
                                // 14/14 across these corpora).
                                assert_eq!(
                                    rep.value(),
                                    base,
                                    "seed={seed:#x} m={m} t={t} cand={candidate}: \
                                     merge-only must be value == base, bitwise"
                                );
                            }
                            ZeroDiversityProof::IncomingProfileDuplicate { member }
                            | ZeroDiversityProof::OutgoingProfileDuplicate { member } => {
                                if matches!(
                                    proof,
                                    ZeroDiversityProof::IncomingProfileDuplicate { .. }
                                ) {
                                    incoming_proofs += 1;
                                } else {
                                    outgoing_proofs += 1;
                                }
                                assert!(member < m, "duplicate witness must be a member index");
                                assert_eq!(
                                    rep.path(),
                                    EvalPath::Fast,
                                    "seed={seed:#x} m={m} t={t} cand={candidate}: \
                                     duplicate proofs are fast-path scoped"
                                );
                                assert!(
                                    rep.schur_complement().is_some(),
                                    "seed={seed:#x} m={m} t={t} cand={candidate}: \
                                     fast route must report s"
                                );
                            }
                        }
                        // The certified claim: the REAL increment is exactly
                        // zero, so an independent fresh evaluation of S ∪ {x}
                        // lands on the base and the returned increment is
                        // roundoff only (worst over these corpora: |increment|
                        // 5.33e-15, fresh-vs-base 3.25e-15 relative).
                        assert!(
                            rep.increment().abs() <= 1e-12,
                            "seed={seed:#x} m={m} t={t} cand={candidate} proof={proof:?}: \
                             computed increment {} exceeds roundoff",
                            rep.increment()
                        );
                        let fresh = fresh_with(&REPORT_NAMES, &couplings, &members, candidate, t)
                            .expect("a proof-carrying candidate must evaluate fresh");
                        assert!(
                            (fresh - base).abs() <= 1e-12 * base.abs().max(1.0),
                            "seed={seed:#x} m={m} t={t} cand={candidate} proof={proof:?}: \
                             fresh {fresh} vs base {base}"
                        );
                    }
                }
            }
        }

        // The corpora must actually exercise all three classes, or the sweep
        // above proves nothing.
        assert!(
            merge_proofs > 0,
            "corpora must contain merge-only candidates"
        );
        assert!(
            incoming_proofs > 0,
            "corpora must contain incoming profile duplicates"
        );
        assert!(
            outgoing_proofs > 0,
            "corpora must contain outgoing profile duplicates"
        );
    }

    /// An isolated candidate adds exactly `1` — the opposite of zero diversity.
    /// Sanity guard that the proof scans do not fire on the `c = r = 0` corner.
    #[test]
    fn report_isolated_candidate_has_no_proof() {
        let agents = ["a", "b", "x"];
        let couplings = [(0usize, 1usize, 0.5f64), (1, 0, 0.5)];
        let members = [0usize, 1];
        let ev = CoalitionEvaluator::new(&agents, &couplings, &members, 1.0).unwrap();
        let rep = ev.value_with_report(2).unwrap();

        assert_eq!(rep.path(), EvalPath::Fast);
        assert_eq!(rep.zero_proof(), None, "an isolated point adds diversity");
        assert!(!rep.is_provably_zero());
        assert!(
            rel_close(rep.increment(), 1.0),
            "isolated point adds exactly 1, got {}",
            rep.increment()
        );
        assert_eq!(rep.schur_complement(), Some(1.0), "p = q = 0 ⇒ s = 1");
    }

    /// Mirror of the bench's `build_fast_path_fixture` (`benches/magnitude_bench.rs`):
    /// the `value_with_hit` / `hit_scratch` benches only measure the fast (Schur)
    /// path if this construction actually takes it.
    #[test]
    fn bench_fast_path_fixture_is_fast() {
        for m in [8usize, 16] {
            let agents: Vec<usize> = (0..=m).collect();
            let mut couplings: Vec<(usize, usize, f64)> = Vec::new();
            for i in 0..(m - 1) {
                couplings.push((i, i + 1, 0.5));
            }
            couplings.push((0, m, 0.2));
            couplings.push((m, 0, 0.2));
            let members: Vec<usize> = (0..m).collect();
            let ev = CoalitionEvaluator::new(&agents, &couplings, &members, 1.0).unwrap();
            // Full skeleton (no perfect-coupling merges) — the fast path does the
            // full O(k²) Schur work the bench means to measure.
            assert_eq!(ev.mu.len(), m, "chain fixture must keep k = m");
            let (_, path) = ev
                .value_with_impl(m, &mut EvalScratch::new())
                .expect("candidate must evaluate");
            assert_eq!(
                path,
                EvalPath::Fast,
                "m={m}: bench fixture must be fast-path"
            );
        }
    }
}
