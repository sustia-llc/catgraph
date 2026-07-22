//! [`CoalitionEvaluator`] — incremental coalition magnitude (#31).
//!
//! The downstream koalisi A/B runs a coalition **decision loop**: for a fixed
//! coalition `S`, sweep every candidate agent `x` and score `Mag(S ∪ {x})` to
//! pick the best join. Done naively via [`coalition_value`](crate::coalition_value)
//! that is one full evaluation per candidate — an `O(m³)` restrict-then-close
//! Bellman–Ford closure plus an `O(k³)` Möbius inversion — even though every
//! candidate coalition differs from `S` by a single member.
//!
//! This module caches the expensive parts of `S` **once** and answers each
//! `Mag(S ∪ {x})` query with an `O(m² + k²)` bordered-matrix update:
//!
//! - the closed `m × m` coupling table of `S` (extracted from the base
//!   [`Coalition`]) — reused to border the candidate in one `O(m²)` pass,
//! - the skeletal, `t`-scaled `ζ⁻¹ = μ` of `S` (the same Gaussian inverse
//!   [`magnitude`] computes) plus its row-sum
//!   ([`weighting`](crate::magnitude::weighting)) and column-sum
//!   ([`coweighting`](crate::magnitude::coweighting)) vectors — reused in the
//!   Schur-complement update below.
//!
//! # The two paths (BV 2025 §3.5 Eq 7 Möbius sum)
//!
//! Adding `x` borders the coalition's Lawvere metric space with one new point.
//! Its `ζ`-matrix becomes the bordered `ζ′ = [[ζ_S, u], [vᵀ, 1]]`, where `u`/`v`
//! are the `exp(−t·d)` similarities from/to `x` (see [`CoalitionEvaluator::value_with`]).
//! Magnitude is `Mag = 1ᵀ (ζ′)⁻¹ 1` (Eq 7), the sum of all entries of the
//! inverse.
//!
//! - **Fast path** — when `x` neither improves any interior member-to-member
//!   closure nor merges into an existing skeletal class, `ζ_S` is unchanged and
//!   the blockwise (Schur) inverse gives a closed form (`O(m² + k²)`, no fresh
//!   inversion). See [`CoalitionEvaluator::value_with`] for the derivation.
//! - **Slow path** — otherwise the closure among old members changes (a new
//!   through-`x` shortcut) or the skeleton shrinks (`x` is a perfect clone), so
//!   `ζ_S` is stale. We border the *closed* table in `O(m²)` (still skipping the
//!   `O(m³)` fresh closure — the cached table plus `x`'s borders suffice) and
//!   re-run the crate's shared skeletalize + [`magnitude`]
//!   helpers on the `(m+1)`-point space.
//!
//! # Numerical contract (#31 amendment, 2026-07-02)
//!
//! 1. **Base value bit-identical.** [`CoalitionEvaluator::base_value`] is `Σ μ`
//!    over the cached skeletal ζ⁻¹, accumulated row-major exactly as
//!    `magnitude` sums the Möbius inverse — `==` (exact) to
//!    [`coalition_magnitude_from_couplings`](crate::coalition::coalition_magnitude_from_couplings) at the same `t`, since both invert
//!    the identically-built scaled space and sum it the same way.
//! 2. **Incremental value within tolerance.** [`CoalitionEvaluator::value_with`]
//!    equals the fresh `Mag(S ∪ {x})` within relative tolerance
//!    [`INCREMENTAL_REL_TOL`]. The fast path reuses the cached `μ` (bit-identical
//!    to what a fresh `k × k` inversion would produce) and adds the Schur
//!    algebra, so it differs from a fresh `(k+1) × (k+1)` inversion only by
//!    floating-point reassociation.
//! 3. **Rank-order identity.** Over a candidate sweep against a fixed `S`,
//!    incremental values rank candidates identically to fresh values (asserted
//!    in the module tests).
//!
//! # Non-goal: the leave path
//!
//! This module accelerates **joins** (`S ∪ {x}`) only. Removal (`S ∖ {x}`) is
//! *not* symmetric: bordering adds a row/column to the closed table, but a
//! max-product closure cannot be "downdated" — dropping `x` can lengthen paths
//! that routed through it, and recovering them needs the couplings `x` shadowed,
//! which the bordered form discarded. Leaves stay fresh (build a new evaluator
//! over the reduced member set).
//!
//! ## Module-level lint
//!
//! The border/Schur loops index multiple parallel vectors/matrices at once
//! (`closed[i][k]·g_in[k]`, `mu[i][a]·u[a]`, `closed[i][j]` vs `c[i]·r[j]`), so
//! the loop index is the primary variable, not just a counter.
//! `clippy::needless_range_loop`'s enumerate-rewrite would only cover one of the
//! indexed operands. Module-level `#![allow]` mirrors the same escape in
//! `magnitude.rs` (its Gaussian-elimination loops) rather than repeating a
//! per-site allow.
#![allow(clippy::needless_range_loop)]

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use crate::coalition::{
    Coalition, build_coupling_category, build_skeletal_space, skeletal_classes,
};
use crate::magnitude::{magnitude, mobius_function, scaled_space, zeta_from_scaled_distance};
use crate::{CatgraphError, F64Rig};

/// Relative tolerance for the #31 incremental-vs-fresh contract (point 2 of the
/// 2026-07-02 amendment).
///
/// The fast path reuses the cached skeletal `μ` and layers a Schur-complement
/// update on top; the slow path borders the cached closed table and re-inverts.
/// Both differ from a fully fresh evaluation only by floating-point
/// reassociation of the same real quantities, which stays well under `1e-9`
/// relative. Genuine divergence (a wrong branch, a stale cache) would exceed it
/// by many orders of magnitude.
pub const INCREMENTAL_REL_TOL: f64 = 1e-9;

/// Relative threshold below which the fast path's Schur complement `s` is deemed
/// too ill-conditioned to trust, and [`CoalitionEvaluator::value_with`] falls
/// back to the slow path instead of the closed-form update.
///
/// The fast path's `Mag′ = base + (1−p)(1−q)/s` and a fully fresh evaluation are
/// two different arithmetic routes to the same real number; they agree within
/// [`INCREMENTAL_REL_TOL`] **only while `s` is well-conditioned**. When the
/// bordered `ζ′` is near-singular — `s = 1 − vᵀμu` a catastrophic-cancellation
/// residue, or a *near*-clone candidate (closed product `0.99999999`, escaping
/// the exact-`1.0` skeletal-merge test) — dividing by a tiny `s` amplifies that
/// residue past tolerance while fresh evaluation may legitimately error. Routing
/// such borders through `CoalitionEvaluator::value_with_slow` (the same
/// helpers fresh evaluation uses) gives them fresh-equivalent treatment: a
/// finite value when well-defined, an `Err` exactly when the re-inversion is
/// singular. `1e-12` sits far below any genuine coalition's `s` yet safely above
/// the cancellation floor.
pub const SCHUR_SLOW_FALLBACK_TOL: f64 = 1e-12;

/// Which update path [`CoalitionEvaluator::value_with`] took for a candidate.
///
/// Exposed to the module tests (via the private `value_with_impl`) so they can
/// assert that a given fixture deliberately exercises the intended branch;
/// [`CoalitionEvaluator::value_with`] discards it and returns only the scalar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EvalPath {
    /// Bordered Schur update against the cached `μ` (no fresh inversion).
    Fast,
    /// Re-skeletalize + re-invert the bordered `(m+1)`-point table.
    Slow,
}

/// Caches a base coalition `S` so per-candidate `Mag(S ∪ {x})` queries skip the
/// `O(m³)` closure and (on the fast path) the `O(k³)` Möbius inversion (#31).
///
/// Build one with [`CoalitionEvaluator::new`] and reuse the handle across a
/// candidate sweep (that reuse is the whole point — a one-shot pair should use
/// [`coalition_value_delta`]). The evaluator is immutable; joins never mutate
/// the cached `S`.
///
/// The struct stores only agent **indices** and `f64` data, so it is not
/// generic over the agent domain `O`; [`CoalitionEvaluator::new`] takes the
/// `agents: &[O]` slice and candidates are indices into it.
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

/// Caller-owned scratch buffers for [`CoalitionEvaluator::value_with_scratch`]
/// (#33).
///
/// Each [`CoalitionEvaluator::value_with`] call heap-allocates seven short-lived
/// `Vec`s — `g_in`/`g_out`/`c`/`r` (length `m`) plus `u`/`v`/`w_u` (length `k`) —
/// whose sizes are fixed by the cached coalition. Across an 8-candidate koalisi
/// join sweep that is ~56 malloc/free pairs per decision on the µs hot path. A
/// caller that builds one `EvalScratch` and threads it through
/// [`CoalitionEvaluator::value_with_scratch`] reuses those allocations across the
/// whole sweep instead.
///
/// # Reuse contract
///
/// The buffers hold **no cross-call state**: every call resizes them to the
/// current coalition and overwrites every live entry before reading it, so a
/// reused `EvalScratch` yields results **bit-identical** to a fresh one (verified
/// by the module tests). Reuse is purely an allocation optimization — the scratch
/// carries nothing between candidates. The buffers grow to the largest coalition
/// they have served and never shrink the backing capacity, so one scratch per
/// worker thread amortizes to zero steady-state allocation.
///
/// The evaluator stays `&self` (shareable, `Sync`); the mutable state lives here,
/// caller-owned, so no interior mutability is introduced. Build with
/// [`EvalScratch::new`] (empty — buffers size on first use).
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
    /// Validation mirrors [`coalition_magnitude_from_couplings`](crate::coalition::coalition_magnitude_from_couplings) **exactly** —
    /// same order, same rejected cases — so an evaluator constructs iff that
    /// function would succeed on `(agents, couplings, members, t)`: member
    /// indices are validated first, then coupling indices, self-couplings, and
    /// probabilities (via [`UnitInterval::new`](crate::UnitInterval::new)); the base [`Coalition`] is then
    /// built through the identical `HomMap` + [`Coalition::from_enriched`] path
    /// (restrict-then-close + skeletalize).
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

    /// The cached fresh `Mag(S)` at the evaluator's `t` (contract point 1:
    /// `==` exact to [`coalition_magnitude_from_couplings`](crate::coalition::coalition_magnitude_from_couplings)).
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

    /// `Mag(S ∪ {candidate})` at the evaluator's `t`, incrementally (#31).
    ///
    /// `candidate` is an agent index; only its **direct** couplings to/from
    /// members enter (restrict-then-close: mediation through a non-member is
    /// dropped, exactly as [`Coalition::from_enriched`] restricts).
    ///
    /// # Algorithm
    ///
    /// Let `g_in[i]` / `g_out[i]` be the direct `member_i → x` / `x → member_i`
    /// couplings (`0` if absent). Border the cached closure in one `O(m²)` pass:
    ///
    /// - `c[i] = closed(i → x) = maxₖ closed[i][k]·g_in[k]`  (`closed[i][i]=1`
    ///   covers the direct edge),
    /// - `r[j] = closed(x → j) = maxₖ g_out[k]·closed[k][j]`.
    ///
    /// One pass is exact: a max-product optimum (weights `≤ 1`, cycles never
    /// improve) is a simple path through `x` at most once, and prefixes/suffixes
    /// of optima are optimal within `S`.
    ///
    /// Two `O(m²)` tests then select the path:
    /// - **interior improvement** — `∃ i≠j: c[i]·r[j] > closed[i][j]` (`x` opens
    ///   a better member-to-member path, so the cached `ζ_S` is stale);
    /// - **skeletal merge** — `∃ i: c[i] == 1 && r[i] == 1` (`x` is a perfect
    ///   clone of member `i`, so the skeleton shrinks; this also fires when `x`
    ///   *bridges* two classes, since that needs mutual-`1.0` with `x`).
    ///
    /// **Fast path** (neither test fires). `x` is a fresh skeletal class and the
    /// `k × k` block `ζ_S` is unchanged, so the bordered
    /// `ζ′ = [[ζ_S, u], [vᵀ, 1]]` has a blockwise (Schur) inverse. With
    /// `u[a] = exp(−t·d(rep_a → x))`, `v[a] = exp(−t·d(x → rep_a))` (the exact
    /// exp route [`mobius_function`] uses —
    /// **not** `powf` — for ULP consistency with the cached `μ`; `0` when the
    /// coupling is `0`):
    ///
    /// - `w_u = μ·u`, Schur complement `s = 1 − vᵀμu = 1 − v·w_u`,
    /// - `p = 1ᵀμu = coweighting·u`, `q = vᵀμ1 = v·weighting`,
    /// - **`Mag′ = Mag(S) + (1 − p)(1 − q)/s`**.
    ///
    /// Derivation: summing the blockwise inverse of `ζ′`, the top-left block
    /// contributes `Mag(S) + pq/s`, the two borders `−p/s` and `−q/s`, and the
    /// corner `1/s`; `pq − p − q + 1 = (1−p)(1−q)`. The interior-improvement
    /// test is exactly the precondition for `ζ_S` (hence `μ`) to stay valid —
    /// when it fails, no bordered Schur form over the cached `μ` is correct, so
    /// we fall through.
    ///
    /// **Slow path** (improvement or merge). Border the *closed* table —
    /// `closed′[i][j] = max(closed[i][j], c[i]·r[j])`, last row/col from `c`/`r`,
    /// corner `1.0` — then re-skeletalize and re-invert with the crate's shared
    /// [`crate::magnitude::magnitude`] helpers on the `(m+1)`-point
    /// space. This still skips the `O(m³)` fresh closure (the cached table plus
    /// `x`'s borders are its full closure) and inherits correctness from the
    /// shared helpers.
    ///
    /// The result matches the fresh `Mag(S ∪ {x})` within
    /// [`INCREMENTAL_REL_TOL`] (BV 2025 §3.5 Eq 7).
    ///
    /// A near-singular bordered `ζ′` (Schur complement `s` within
    /// [`SCHUR_SLOW_FALLBACK_TOL`] of singular) is routed through the slow path
    /// rather than the closed form — see `value_with_fast`.
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
    /// instead of allocating the seven per-call `Vec`s (#33).
    ///
    /// Identical in every respect to [`value_with`](Self::value_with) — same
    /// arithmetic, same paths, **bit-identical** result — except that the border
    /// and Schur working vectors are drawn from `scratch` and reused across calls.
    /// The intended use is a candidate **sweep** against a fixed `S`: build one
    /// `EvalScratch` (or one per worker thread), then call this per candidate so
    /// the koalisi join sweep pays no per-decision allocation.
    ///
    /// `scratch` carries **no state between calls** — each call resizes and fully
    /// overwrites the buffers it reads (see [`EvalScratch`]'s reuse contract), so
    /// the same `scratch` may be reused across candidates, across evaluators, and
    /// even across coalitions of different sizes with no contamination.
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

    /// Core of [`value_with`](Self::value_with), also returning which
    /// [`EvalPath`] was taken (for test assertions). Writes its border/Schur
    /// working vectors into the caller-owned `scratch` (#33).
    #[allow(clippy::similar_names)] // `c`/`r`, `u`/`v`, `p`/`q` are the paper's border names.
    fn value_with_impl(
        &self,
        candidate: usize,
        scratch: &mut EvalScratch,
    ) -> Result<(f64, EvalPath), CatgraphError> {
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

        // Branch tests (O(m²), short-circuiting).
        let interior_improvement = (0..m)
            .any(|i| (0..m).any(|j| i != j && scratch.c[i] * scratch.r[j] > self.closed[i][j]));
        let skeletal_merge = (0..m).any(|i| scratch.c[i] == 1.0 && scratch.r[i] == 1.0);

        if interior_improvement || skeletal_merge {
            return self.value_with_slow(scratch, m);
        }
        self.value_with_fast(scratch, m)
    }

    /// Fast path: bordered Schur update against the cached skeletal `μ`.
    ///
    /// Falls back to [`value_with_slow`](Self::value_with_slow) when the Schur
    /// complement `s` is near-singular (relative to `vᵀμu`, threshold
    /// [`SCHUR_SLOW_FALLBACK_TOL`]): the closed-form update and fresh evaluation
    /// agree only while `s` is well-conditioned, so an ill-conditioned border is
    /// routed through the fresh-equivalent slow path (finite when well-defined,
    /// `Err` exactly when the re-inversion is singular) instead of dividing by a
    /// catastrophic-cancellation residue.
    #[allow(clippy::similar_names)]
    fn value_with_fast(
        &self,
        scratch: &mut EvalScratch,
        m: usize,
    ) -> Result<(f64, EvalPath), CatgraphError> {
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

        // w_u = μ·u ; Schur complement s = 1 − vᵀμu. Accumulation order matches
        // the prior iterator `.sum()` (row-major, from 0.0), so the result stays
        // bit-identical to the pre-#33 allocating path.
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
            return self.value_with_slow(scratch, m);
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
        Ok((mag, EvalPath::Fast))
    }

    /// Slow path: border the closed table, then re-skeletalize + re-invert on
    /// the `(m+1)`-point space with the crate's shared helpers.
    fn value_with_slow(
        &self,
        scratch: &EvalScratch,
        m: usize,
    ) -> Result<(f64, EvalPath), CatgraphError> {
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
        Ok((mag.0, EvalPath::Slow))
    }
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
/// scale `t = 1` — the one-shot entry point of #31.
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

    /// A tiny deterministic LCG (same shape as the bench / `lm_category` tests)
    /// — no `rand` dep. Yields `f64` in `[0, 1)`.
    struct Lcg(u64);
    impl Lcg {
        fn new(seed: u64) -> Self {
            Lcg(seed | 1)
        }
        fn next_f64(&mut self) -> f64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((self.0 >> 33) as f64) / ((1u64 << 31) as f64)
        }
    }

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
    // Deterministic seeded grid over m ∈ 2..=10 pools with several candidates.
    // Asserts fresh/incremental error-parity AND value equality within
    // tolerance for every (S, x) — this also exercises the singular branch if
    // any grid point hits it (see the report note: an exact post-skeletal
    // singular ζ is not hand-constructible).
    // -----------------------------------------------------------------------
    #[test]
    fn seeded_grid_fresh_vs_incremental() {
        // A fixed 12-agent pool named s0..s11.
        const NAMES: [&str; 12] = [
            "s0", "s1", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11",
        ];
        let mut lcg = Lcg::new(0xC0FFEE);
        let n = NAMES.len();

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
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Rank-order identity (contract point 3): a fixed S, ≥5 candidates with
    // distinct values, argsort by fresh == argsort by incremental.
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
    // #33 scratch buffers: `value_with_scratch` is bit-identical to `value_with`,
    // a reused scratch is contamination-free across a candidate sweep, and error
    // cases still error identically.
    // -----------------------------------------------------------------------

    /// The dense seeded grid (same shape as `seeded_grid_fresh_vs_incremental`)
    /// but comparing the scratch path against the allocating path with `==`
    /// (exact), not a tolerance. Covers both fast- and slow-path candidates.
    #[test]
    fn value_with_scratch_bit_identical_to_value_with() {
        const NAMES: [&str; 12] = [
            "s0", "s1", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11",
        ];
        let mut lcg = Lcg::new(0xC0FFEE);
        let n = NAMES.len();

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
                // A single scratch reused across the whole candidate sweep — this
                // is exactly the koalisi call pattern and the contamination guard.
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
                    }
                }
            }
        }
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

    /// Mirror of the bench's `build_fast_path_fixture` (`benches/magnitude_bench.rs`):
    /// the `value_with_hit` / `hit_scratch` benches only measure the fast (Schur)
    /// path if this construction actually takes it. Pinned here so a fixture drift
    /// that silently pushes the bench onto the slow path is caught by the test
    /// suite rather than skewing the #33 numbers.
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
