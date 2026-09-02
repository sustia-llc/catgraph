//! Roles and channels over the coalition engine: three layers beside the
//! coalition entry points, all valued in `UnitInterval` and lifted by
//! `d = −ln π` into the shared `ζ_t = exp(−t·d)` kernel.
//!
//! - [`CoalitionEvaluator::role_shares`] decomposes `Mag(S) = Σ_c w_c` (the
//!   weighting of Leinster 2013 Lemma 1.1.4) across a caller-supplied role
//!   partition of the members, reporting over already-computed data.
//! - [`modulate`] applies a role-interaction table `ρ : R × R → [0,1]`
//!   multiplicatively, `π′ = ρ(r_i, r_j) · π` — additively in the metric,
//!   `d′ = d_ρ + d`. [`role_grid`] constructs the product of a role table and a
//!   fiber table, on which the magnitude factorizes exactly
//!   ([`RoleGrid::proof`]).
//! - [`ChannelCouplings`] carries a per-pair vector `v ∈ [0,1]^C`;
//!   [`ChannelCouplings::collapse`] contracts it to the scalar enrichment
//!   through a declared monoid homomorphism `|v|_θ = Π_c v_c^{θ_c}`.
//!
//! # Extension marker
//!
//! No typed or colored magnitude exists in this crate's anchor literature
//! (Bradley–Vigneaux 2025, BTV 2021, Leinster 2008 / 2013, Leinster–Shulman
//! 2017). This module is an **extension**; the mechanisms it rests on are
//! anchored —
//!
//! - the weighting decomposition `Mag = Σ w` — Leinster 2013 §1.1 Lemma 1.1.4;
//! - `|A ⊗ B| = |A|·|B|` for a product of enriched categories — Leinster 2013
//!   ([arXiv:1012.5857](https://arxiv.org/abs/1012.5857)) Prop 1.4.3 / Prop 2.3.6,
//!   with the weighted categorical (fibration) case at Leinster 2008
//!   ([arXiv:math/0610260](https://arxiv.org/abs/math/0610260)) Prop 2.8;
//! - "magnitude is defined relative to a monoid homomorphism out of the
//!   enriching object monoid" — Leinster 2013 §1.3;
//!
//! — but their assembly into a role/channel vocabulary for coalitions is not a
//! paper's.
//!
//! # The role-grid factorization certificate
//!
//! On a [`role_grid`] product — agents are pairs `(r, x)` at the flat index
//! `r·n + x`, every ordered cross pair carrying
//! `π((r, x), (r′, x′)) = ρ(r, r′) · σ(x, x′)` — the max-product closure is the
//! product of the factor closures, the scaled zeta is the Kronecker product
//! `ζ_t = ζ_t^ρ ⊗ ζ_t^σ`, the skeleton of the product is the product of the
//! skeletons, and the magnitude is `|ρ|_t · |σ|_t` (Leinster 2013 Prop 1.4.3 /
//! Prop 2.3.6, with Thm 2.3.11 / Ex 2.3.12(i) for the fibration reading;
//! weighted categorical case Leinster 2008 Prop 2.8). [`RoleGrid::proof`]
//! reports that product. Both the closure identity and the skeleton identity
//! rest on the exactly-`1.0` factor diagonals [`role_grid`] enforces.
//!
//! The certificate holds because [`role_grid`] *constructed* the product-form
//! couplings; there is no detection path for arbitrary float input. Grid
//! evaluation and certificate agree only within a **relative tolerance** — the
//! two routes re-associate the same real products differently — and the
//! deviation grows with the conditioning of `ζ_t`: couplings at `1 − O(1e-12)`,
//! just below the merge knife-edge, reach relative deviations of `~1e-2`. A
//! `1e-9` tolerance holds away from near-1 non-merged couplings. Singularity
//! factorizes with the determinant (`det(A ⊗ B) = det(A)ⁿ det(B)ᴿ`), so in exact
//! arithmetic [`RoleGrid::proof`] errors exactly when the grid evaluation would.
//!
//! # The θ-collapse
//!
//! For each fixed `θ` with every `θ_c ≥ 0`, `|v|_θ = Π_c v_c^{θ_c}` is a monoid
//! homomorphism `([0,1]^C, ·, 1) → ([0,1], ·, 1)` — it maps the componentwise
//! unit to `1` and satisfies `|v ⊙ w|_θ = |v|_θ · |w|_θ` — the "size" datum
//! Leinster 2013 §1.3 requires for the magnitude of a `V`-enriched category to
//! be defined. Under the `−ln` lift it reads as `d = Σ_c θ_c d_c`. Every `θ`
//! yields a size homomorphism and the anchor literature picks no canonical one.
//!
//! `powf` can round up to exactly `1.0` from a strictly-sub-1 base (e.g.
//! `0.9999999999999999_f64.powf(1e-18) == 1.0`), so a θ-collapsed table can
//! carry exact-`1.0` couplings — and so trigger skeletal merges — between agents
//! that no channel perfectly couples.

use std::collections::BTreeMap;

use crate::coalition::{coalition_magnitude_from_couplings, skeletal_classes};
use crate::coalition_eval::CoalitionEvaluator;
use crate::{CatgraphError, UnitInterval};

/// A role identifier. The vocabulary is caller-owned; this module partitions by
/// equality only.
pub type RoleId = usize;

// ---------------------------------------------------------------------------
// T1 — role-share diagnostics
// ---------------------------------------------------------------------------

/// A skeletal class whose members do not all carry the same role, reported by
/// [`RoleShares::mixed_classes`] instead of being attributed to any role.
///
/// The coalition engine quotients members at mutual closure `1.0` regardless of
/// role, and the cached weighting lives on that quotient; a class spanning two
/// roles carries a weight attributed to neither.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MixedClass {
    rep: usize,
    roles: Vec<RoleId>,
}

impl MixedClass {
    /// The class representative, as a **member-local** index into
    /// [`CoalitionEvaluator::members`] (not an agent index).
    #[must_use]
    pub fn rep(&self) -> usize {
        self.rep
    }

    /// The distinct roles the class spans, ascending. Always at least two
    /// entries — a single-role class is not mixed.
    #[must_use]
    pub fn roles(&self) -> &[RoleId] {
        &self.roles
    }
}

/// The role decomposition of a coalition's magnitude, from
/// [`CoalitionEvaluator::role_shares`].
///
/// # Contract
///
/// - [`share`](Self::share)`(r)` is the sum of the cached weighting over
///   skeletal classes all of whose members carry role `r` (Leinster 2013
///   Lemma 1.1.4: `Mag = Σ_c w_c`).
/// - A role-mixed class contributes to no share; it is reported by
///   [`mixed_classes`](Self::mixed_classes).
/// - [`is_exact`](Self::is_exact) is `true` iff no class is mixed — every class
///   weight is attributed wholly to one role, nothing split or estimated.
/// - When `is_exact()`, `Σ_r share(r)` equals
///   [`CoalitionEvaluator::base_value`] up to float re-association (the
///   per-role bucketed sums re-associate the row-major accumulation
///   `base_value` uses); compare within a relative tolerance.
/// - When `!is_exact()`, the sum falls short of `base_value` by the total
///   weight of the mixed classes — a structural shortfall that can be
///   arbitrarily large, not a rounding effect.
#[derive(Clone, Debug, PartialEq)]
pub struct RoleShares {
    /// Role → attributed weight. Every role appearing in the caller's partition
    /// is present (possibly at `0.0`); nothing else is.
    shares: BTreeMap<RoleId, f64>,
    mixed: Vec<MixedClass>,
}

impl RoleShares {
    /// The magnitude share attributed to `role`, or `None` if that role id never
    /// appeared in the partition.
    ///
    /// `Some(0.0)` and `None` are different answers: a role that appears only
    /// inside role-mixed classes is present with share `0.0`, whereas an id that
    /// was never supplied is `None`.
    #[must_use]
    pub fn share(&self, role: RoleId) -> Option<f64> {
        self.shares.get(&role).copied()
    }

    /// `true` iff no skeletal class spans two roles — i.e. every class weight is
    /// attributed wholly to one role (see the type's contract; this is an
    /// attribution claim, not a bit-identity claim).
    #[must_use]
    pub fn is_exact(&self) -> bool {
        self.mixed.is_empty()
    }

    /// The role-mixed classes, in skeletal-class order. Empty iff
    /// [`is_exact`](Self::is_exact).
    #[must_use]
    pub fn mixed_classes(&self) -> &[MixedClass] {
        &self.mixed
    }

    /// How many **distinct** roles the partition used (mixed-only roles
    /// included).
    #[must_use]
    pub fn n_roles(&self) -> usize {
        self.shares.len()
    }
}

impl CoalitionEvaluator {
    /// Decompose the cached `Mag(S)` across a role partition of the members by
    /// bucketing the cached weighting (`w = μ·1`, Leinster 2013 Lemma 1.1.4).
    ///
    /// `roles` is indexed by **member-local position** — `roles[i]` is the role
    /// of `members()[i]`, not of agent `i` — so it must have exactly
    /// `members().len()` entries. (Contrast [`modulate`], whose `roles` slice is
    /// indexed by agent index.) See [`RoleShares`] for the exactness contract.
    ///
    /// # Examples
    ///
    /// ```
    /// use catgraph_magnitude::CoalitionEvaluator;
    ///
    /// // a → b (0.7), b → c (0.5); no perfectly-coupled pair, so every member
    /// // is its own skeletal class and Mag(S) = 0.3 + 0.5 + 1.0 = 1.8.
    /// let agents = ["a", "b", "c"];
    /// let couplings = [(0, 1, 0.7), (1, 2, 0.5)];
    /// let ev = CoalitionEvaluator::new(&agents, &couplings, &[0, 1, 2], 1.0).unwrap();
    ///
    /// // a and b are "workers" (role 0), c is a "planner" (role 1).
    /// let shares = ev.role_shares(&[0, 0, 1]).unwrap();
    /// assert!(shares.is_exact());
    /// assert_eq!(shares.n_roles(), 2);
    /// assert!((shares.share(0).unwrap() - 0.8).abs() < 1e-12);
    /// assert!((shares.share(1).unwrap() - 1.0).abs() < 1e-12);
    /// assert_eq!(shares.share(7), None); // never supplied
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] if `roles.len()` differs from the
    /// number of coalition members.
    pub fn role_shares(&self, roles: &[RoleId]) -> Result<RoleShares, CatgraphError> {
        let m = self.members().len();
        if roles.len() != m {
            return Err(CatgraphError::Composition {
                message: format!(
                    "CoalitionEvaluator::role_shares: roles has length {} but the coalition has {m} members (roles are indexed by member-local position)",
                    roles.len()
                ),
            });
        }

        // The evaluator caches class representatives but not the member → class
        // map, so recompute it with the helper the slow path uses.
        let (member_classes, recomputed_reps) = skeletal_classes(self.closed_table(), m);
        let reps = self.class_reps();
        let weighting = self.weighting_vec();
        assert_eq!(
            recomputed_reps, reps,
            "invariant: role_shares must recover the same skeleton the evaluator cached"
        );
        assert_eq!(
            reps.len(),
            weighting.len(),
            "invariant: the cached weighting is indexed by skeletal class"
        );

        let mut class_roles: Vec<Vec<RoleId>> = vec![Vec::new(); reps.len()];
        // Seed every supplied role so `share` distinguishes "0.0" from "unknown".
        let mut shares: BTreeMap<RoleId, f64> = BTreeMap::new();
        for (i, &role) in roles.iter().enumerate() {
            shares.entry(role).or_insert(0.0);
            let bucket = &mut class_roles[member_classes[i]];
            if !bucket.contains(&role) {
                bucket.push(role);
            }
        }

        let mut mixed: Vec<MixedClass> = Vec::new();
        for (c, class) in class_roles.iter().enumerate() {
            let mut class_roles_sorted = class.clone();
            class_roles_sorted.sort_unstable();
            if let [only] = class_roles_sorted[..] {
                let entry = shares
                    .get_mut(&only)
                    .expect("invariant: every class role was seeded from `roles` above");
                *entry += weighting[c];
            } else {
                mixed.push(MixedClass {
                    rep: reps[c],
                    roles: class_roles_sorted,
                });
            }
        }

        Ok(RoleShares { shares, mixed })
    }
}

// ---------------------------------------------------------------------------
// T2 — role-modulated couplings
// ---------------------------------------------------------------------------

/// A validated square table of interaction strengths in `[0, 1]`, serving both
/// as the role-interaction table `ρ : R × R → [0,1]` of [`modulate`] and as
/// either factor of a [`role_grid`].
///
/// Asymmetry is allowed: `ρ(r, r′)` need not equal `ρ(r′, r)`, and the
/// coalition engine consumes asymmetric couplings. A consumer that feeds a
/// modulated table to `magnitude_f64` directly should know that the symmetric
/// `f64-fast` factorization engages only on exactly-symmetric ζ.
///
/// Every entry is validated through [`UnitInterval::new`]: non-NaN and in
/// `[0, 1]`.
#[derive(Clone, Debug, PartialEq)]
pub struct RoleModulation {
    rho: Vec<Vec<f64>>,
}

impl RoleModulation {
    /// Build from a square table, validating shape and every entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use catgraph_magnitude::RoleModulation;
    ///
    /// let rho = RoleModulation::new(vec![vec![1.0, 0.5], vec![0.25, 1.0]]).unwrap();
    /// assert_eq!(rho.n_roles(), 2);
    /// assert_eq!(rho.rho(0, 1), Some(0.5));
    /// assert_eq!(rho.rho(1, 0), Some(0.25));
    /// assert_eq!(rho.rho(2, 0), None);
    /// ```
    ///
    /// # Errors
    ///
    /// - [`CatgraphError::Composition`] if the table is empty or not square
    ///   (some row's length differs from the number of rows).
    /// - [`CatgraphError::RigAxiomViolation`] if an entry is NaN or outside
    ///   `[0, 1]` (propagated from [`UnitInterval::new`]).
    pub fn new(rho: Vec<Vec<f64>>) -> Result<Self, CatgraphError> {
        let n = rho.len();
        if n == 0 {
            return Err(CatgraphError::Composition {
                message: "RoleModulation::new: the table is empty (at least one role is required)"
                    .to_string(),
            });
        }
        for (i, row) in rho.iter().enumerate() {
            if row.len() != n {
                return Err(CatgraphError::Composition {
                    message: format!(
                        "RoleModulation::new: table must be square — row {i} has {} entries, expected {n}",
                        row.len()
                    ),
                });
            }
            for &v in row {
                UnitInterval::new(v)?;
            }
        }
        Ok(Self { rho })
    }

    /// The number of roles `R` (the table is `R × R`). Always `≥ 1`.
    #[must_use]
    pub fn n_roles(&self) -> usize {
        self.rho.len()
    }

    /// `ρ(from, to)`, or `None` if either index is out of range.
    #[must_use]
    pub fn rho(&self, from: RoleId, to: RoleId) -> Option<f64> {
        self.rho.get(from)?.get(to).copied()
    }

    /// The first `(index, value)` whose diagonal entry is not exactly `1.0`, if
    /// any. The comparison is exact.
    #[allow(clippy::float_cmp)]
    fn first_non_unit_diagonal(&self) -> Option<(usize, f64)> {
        self.rho
            .iter()
            .enumerate()
            .find(|(i, row)| row[*i] != 1.0)
            .map(|(i, row)| (i, row[i]))
    }

    /// The strictly-positive off-diagonal entries as coalition coupling triples.
    ///
    /// The diagonal is dropped (the identity axiom fixes it) and zero entries
    /// are dropped (a zero coupling and an absent one are the same point in the
    /// `[0,1]` rig), so the result is accepted by
    /// [`coalition_magnitude_from_couplings`], which rejects self-couplings.
    fn as_couplings(&self) -> Vec<(usize, usize, f64)> {
        let mut out = Vec::new();
        for (i, row) in self.rho.iter().enumerate() {
            for (j, &v) in row.iter().enumerate() {
                if i != j && v > 0.0 {
                    out.push((i, j, v));
                }
            }
        }
        out
    }

    /// Magnitude of this table read as a coalition over `0..n_roles()`, at
    /// scale `t`, through the ordinary public entry point.
    fn magnitude_at(&self, t: f64) -> Result<f64, CatgraphError> {
        let agents: Vec<usize> = (0..self.n_roles()).collect();
        coalition_magnitude_from_couplings(&agents, &self.as_couplings(), &agents, t)
    }
}

/// Couplings with a role modulation already applied — the output of
/// [`modulate`], carried as its own type so a modulated table is distinct from
/// a raw one.
#[derive(Clone, Debug, PartialEq)]
pub struct ModulatedCouplings {
    couplings: Vec<(usize, usize, f64)>,
}

impl ModulatedCouplings {
    /// The modulated `(from, to, π′)` triples, ready for any existing entry
    /// point — [`CoalitionEvaluator::new`],
    /// [`coalition_value`](crate::coalition_value),
    /// [`coalition_magnitude_from_couplings`], …
    ///
    /// One output triple per input triple, in input order. A `ρ` entry of `0`
    /// yields an explicit `0.0` coupling, which the engine treats exactly as an
    /// absent one.
    #[must_use]
    pub fn couplings(&self) -> &[(usize, usize, f64)] {
        &self.couplings
    }
}

/// Apply a role-interaction table to a coupling list:
/// `π′ = ρ(r_from, r_to) · π`.
///
/// The result feeds every coalition entry point unchanged. Under the BTV lift
/// `d = −ln π` the modulation is additive in the metric, `d′ = d_ρ + d`, so it
/// only lengthens distances (`ρ ≤ 1`).
///
/// `roles` is indexed by **agent** index — `roles[i]` is the role of agent `i` —
/// and must cover every index appearing in `couplings`. (Contrast
/// [`CoalitionEvaluator::role_shares`], whose slice is member-local.)
///
/// # Examples
///
/// ```
/// use catgraph_magnitude::{modulate, RoleModulation};
///
/// let rho = RoleModulation::new(vec![vec![1.0, 0.5], vec![0.25, 1.0]]).unwrap();
/// let roles = [0, 1, 1]; // agent 0 is role 0; agents 1 and 2 are role 1
/// let couplings = [(0, 1, 0.8), (1, 2, 0.4), (1, 0, 0.6)];
///
/// let modulated = modulate(&couplings, &roles, &rho).unwrap();
/// assert_eq!(
///     modulated.couplings(),
///     // 0.5·0.8, 1.0·0.4 (within-role), 0.25·0.6 (the asymmetric direction)
///     &[(0, 1, 0.4), (1, 2, 0.4), (1, 0, 0.15)]
/// );
/// ```
///
/// # Errors
///
/// - [`CatgraphError::Composition`] if some `roles` entry is `≥ rho.n_roles()`,
///   or if a coupling names an agent index `≥ roles.len()`.
/// - [`CatgraphError::RigAxiomViolation`] if an input probability is NaN or
///   outside `[0, 1]` (propagated from [`UnitInterval::new`], the same validator
///   the coalition entry points use).
pub fn modulate(
    couplings: &[(usize, usize, f64)],
    roles: &[RoleId],
    rho: &RoleModulation,
) -> Result<ModulatedCouplings, CatgraphError> {
    let n_roles = rho.n_roles();
    for (agent, &role) in roles.iter().enumerate() {
        if role >= n_roles {
            return Err(CatgraphError::Composition {
                message: format!(
                    "modulate: agent {agent} has role {role}, out of range for a {n_roles}-role modulation"
                ),
            });
        }
    }

    let mut out = Vec::with_capacity(couplings.len());
    for &(from, to, p) in couplings {
        if from >= roles.len() || to >= roles.len() {
            return Err(CatgraphError::Composition {
                message: format!(
                    "modulate: coupling ({from}, {to}) names an agent with no role — `roles` covers {} agents",
                    roles.len()
                ),
            });
        }
        let checked = UnitInterval::new(p)?.value();
        let factor = rho
            .rho(roles[from], roles[to])
            .expect("invariant: both roles were checked against rho.n_roles() above");
        out.push((from, to, factor * checked));
    }
    Ok(ModulatedCouplings { couplings: out })
}

// ---------------------------------------------------------------------------
// T2 — the role grid and its factorization certificate
// ---------------------------------------------------------------------------

/// The expected magnitude of a [`RoleGrid`], carried by construction.
///
/// Not a measurement of the grid: [`RoleGrid::proof`] evaluates each factor
/// through the ordinary public coalition entry point and multiplies. Compare it
/// against a grid evaluation within a relative tolerance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoleFibrationProof {
    role_magnitude: f64,
    fiber_magnitude: f64,
    t: f64,
}

impl RoleFibrationProof {
    /// `|ρ|_t` — the magnitude of the role factor alone.
    #[must_use]
    pub fn role_magnitude(&self) -> f64 {
        self.role_magnitude
    }

    /// `|σ|_t` — the magnitude of the fiber factor alone.
    #[must_use]
    pub fn fiber_magnitude(&self) -> f64 {
        self.fiber_magnitude
    }

    /// `|ρ|_t · |σ|_t` — what the whole grid evaluates to (Leinster 2013
    /// Prop 1.4.3 / Prop 2.3.6; weighted case Leinster 2008 Prop 2.8).
    #[must_use]
    pub fn expected_magnitude(&self) -> f64 {
        self.role_magnitude * self.fiber_magnitude
    }

    /// The scale the proof was computed at.
    #[must_use]
    pub fn t(&self) -> f64 {
        self.t
    }
}

/// A product coalition `role × fiber` with product-form couplings, built by
/// [`role_grid`].
///
/// Agents are pairs `(role, position)` at the flat index `role·n + position`
/// (`n` = the fiber's size), and every ordered cross pair carries
/// `π((r,x), (r′,x′)) = ρ(r, r′) · σ(x, x′)`. Its magnitude factorizes exactly;
/// [`proof`](Self::proof) carries the certificate.
#[derive(Clone, Debug, PartialEq)]
pub struct RoleGrid {
    role_space: RoleModulation,
    fiber: RoleModulation,
    couplings: Vec<(usize, usize, f64)>,
}

impl RoleGrid {
    /// Total agent count `R · n`.
    #[must_use]
    pub fn n_agents(&self) -> usize {
        self.role_space.n_roles() * self.fiber.n_roles()
    }

    /// The flat agent index `role·n + pos`, or `None` if either coordinate is
    /// out of range.
    #[must_use]
    pub fn agent_index(&self, role: RoleId, pos: usize) -> Option<usize> {
        let n = self.fiber.n_roles();
        if role >= self.role_space.n_roles() || pos >= n {
            return None;
        }
        Some(role * n + pos)
    }

    /// The product couplings, ready for any coalition entry point.
    ///
    /// Self-pairs and zero products are omitted; every other ordered cross pair
    /// is present, so the list holds up to `(R·n)² − R·n` entries.
    #[must_use]
    pub fn couplings(&self) -> &[(usize, usize, f64)] {
        &self.couplings
    }

    /// The construction-carried certificate: each factor's magnitude at scale
    /// `t`, and their product.
    ///
    /// Both factor magnitudes go through the public
    /// [`coalition_magnitude_from_couplings`] entry point over all of that
    /// factor's points, so certificate and grid evaluation share every kernel
    /// (closure, skeletalization, `ζ_t`, Möbius inversion).
    ///
    /// # Examples
    ///
    /// ```
    /// use catgraph_magnitude::{coalition_magnitude_from_couplings, role_grid, RoleModulation};
    ///
    /// let rho = RoleModulation::new(vec![vec![1.0, 0.5], vec![0.5, 1.0]]).unwrap();
    /// let sigma = RoleModulation::new(vec![vec![1.0, 0.6], vec![0.3, 1.0]]).unwrap();
    /// let grid = role_grid(&rho, &sigma).unwrap();
    /// assert_eq!(grid.n_agents(), 4);
    ///
    /// let agents: Vec<usize> = (0..grid.n_agents()).collect();
    /// let actual =
    ///     coalition_magnitude_from_couplings(&agents, grid.couplings(), &agents, 1.0).unwrap();
    /// let expected = grid.proof(1.0).unwrap().expected_magnitude();
    /// assert!((actual - expected).abs() <= 1e-9 * expected.abs().max(1.0));
    /// ```
    ///
    /// # Panics
    ///
    /// Debug-only, inherited from
    /// [`coalition_magnitude`](crate::coalition_magnitude): panics if `t ≤ 0`
    /// (BV 2025 §3 studies `t > 0`; behavior at `t ≤ 0` is unspecified).
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] if either factor's `t`-scaled zeta
    /// is singular. The role factor is checked first, so if both factors are
    /// singular only its error surfaces.
    pub fn proof(&self, t: f64) -> Result<RoleFibrationProof, CatgraphError> {
        Ok(RoleFibrationProof {
            role_magnitude: self.role_space.magnitude_at(t)?,
            fiber_magnitude: self.fiber.magnitude_at(t)?,
            t,
        })
    }
}

/// Build the product coalition of a role table and a fiber table, whose
/// magnitude factorizes exactly.
///
/// Agents are pairs `(role, position)` flattened to `role·n + position`; the
/// coupling of an ordered pair is the product
/// `π((r,x), (r′,x′)) = ρ(r, r′) · σ(x, x′)`. See the module docs for the
/// factorization and its anchors (Leinster 2013 Prop 1.4.3 / Prop 2.3.6;
/// Leinster 2008 Prop 2.8).
///
/// # Precondition: both diagonals are exactly `1.0`
///
/// Validated by an exact `== 1.0` check on each factor. The coalition engine
/// forces the closed table's diagonal to `1.0` (the identity axiom
/// `d(x, x) = 0`), so a factor with `ρ(r, r) < 1` would be promoted on the grid
/// but not in its own factor evaluation, making the certificate false.
///
/// # Examples
///
/// ```
/// use catgraph_magnitude::{role_grid, RoleModulation};
///
/// let rho = RoleModulation::new(vec![vec![1.0, 0.5], vec![0.5, 1.0]]).unwrap();
/// let sigma = RoleModulation::new(vec![vec![1.0, 0.9], vec![0.0, 1.0]]).unwrap();
/// let grid = role_grid(&rho, &sigma).unwrap();
///
/// assert_eq!(grid.n_agents(), 4);
/// assert_eq!(grid.agent_index(1, 0), Some(2));
/// // (0,0) → (1,1) carries ρ(0,1)·σ(0,1) = 0.5·0.9 = 0.45
/// assert!(grid.couplings().contains(&(0, 3, 0.45)));
/// ```
///
/// # Errors
///
/// Returns [`CatgraphError::Composition`] if either factor has a diagonal entry
/// that is not exactly `1.0`, naming the factor and the offending index.
pub fn role_grid(
    role_space: &RoleModulation,
    fiber: &RoleModulation,
) -> Result<RoleGrid, CatgraphError> {
    for (label, table) in [("role_space", role_space), ("fiber", fiber)] {
        if let Some((i, v)) = table.first_non_unit_diagonal() {
            return Err(CatgraphError::Composition {
                message: format!(
                    "role_grid: {label} diagonal entry ({i}, {i}) is {v}, not exactly 1.0 — the grid's forced unit diagonal would make the factorization certificate false"
                ),
            });
        }
    }

    let n = fiber.n_roles();
    let mut couplings = Vec::new();
    for (r, rho_row) in role_space.rho.iter().enumerate() {
        for (x, sigma_row) in fiber.rho.iter().enumerate() {
            let from = r * n + x;
            for (r2, &rr) in rho_row.iter().enumerate() {
                for (x2, &ss) in sigma_row.iter().enumerate() {
                    if r == r2 && x == x2 {
                        continue; // the identity axiom owns the diagonal
                    }
                    let p = rr * ss;
                    if p > 0.0 {
                        couplings.push((from, r2 * n + x2, p));
                    }
                }
            }
        }
    }

    Ok(RoleGrid {
        role_space: role_space.clone(),
        fiber: fiber.clone(),
        couplings,
    })
}

// ---------------------------------------------------------------------------
// T3 — channel-valued couplings + declared collapse
// ---------------------------------------------------------------------------

/// Couplings valued in `[0,1]^C` — one component per channel — plus the
/// declared collapse back to the scalar enrichment via
/// [`collapse`](Self::collapse)`(θ)`.
///
/// Entries are keyed by the ordered pair `(from, to)` with last-write-wins on
/// repeats, matching `HomMap::set_hom`'s overwrite. Iteration and
/// [`collapse`](Self::collapse) output are in ascending `(from, to)` order.
#[derive(Clone, Debug, PartialEq)]
pub struct ChannelCouplings {
    n_channels: usize,
    entries: BTreeMap<(usize, usize), Vec<f64>>,
}

impl ChannelCouplings {
    /// An empty table over `n_channels` channels.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] if `n_channels == 0`.
    pub fn new(n_channels: usize) -> Result<Self, CatgraphError> {
        if n_channels == 0 {
            return Err(CatgraphError::Composition {
                message: "ChannelCouplings::new: n_channels must be at least 1".to_string(),
            });
        }
        Ok(Self {
            n_channels,
            entries: BTreeMap::new(),
        })
    }

    /// The number of channels `C` every vector must have.
    #[must_use]
    pub fn n_channels(&self) -> usize {
        self.n_channels
    }

    /// Record the channel vector of the ordered pair `from → to`, overwriting
    /// any previous vector for that pair.
    ///
    /// # Errors
    ///
    /// - [`CatgraphError::Composition`] if `from == to` (the identity axiom
    ///   fixes the diagonal), or if `v.len()` differs from
    ///   [`n_channels`](Self::n_channels).
    /// - [`CatgraphError::RigAxiomViolation`] if a component is NaN or outside
    ///   `[0, 1]` (propagated from [`UnitInterval::new`]).
    pub fn set(&mut self, from: usize, to: usize, v: Vec<f64>) -> Result<(), CatgraphError> {
        if from == to {
            return Err(CatgraphError::Composition {
                message: format!(
                    "ChannelCouplings::set: self-coupling ({from}, {from}) — the identity axiom fixes the diagonal to 1.0"
                ),
            });
        }
        if v.len() != self.n_channels {
            return Err(CatgraphError::Composition {
                message: format!(
                    "ChannelCouplings::set: pair ({from}, {to}) has {} channel components, expected {}",
                    v.len(),
                    self.n_channels
                ),
            });
        }
        for &c in &v {
            UnitInterval::new(c)?;
        }
        self.entries.insert((from, to), v);
        Ok(())
    }

    /// Contract every channel vector to a scalar coupling through the declared
    /// homomorphism `|v|_θ = Π_c v_c^{θ_c}`, yielding triples the ordinary
    /// coalition entry points consume.
    ///
    /// # The θ contract
    ///
    /// - `θ.len()` must equal [`n_channels`](Self::n_channels), and every `θ_c`
    ///   must be finite and `≥ 0`.
    /// - `Σ_c θ_c = 1` is a convention, not an enforced constraint. Off-simplex
    ///   `θ` is accepted and rescales the metric.
    /// - `θ = e_c` (one at channel `c`, zero elsewhere) recovers channel `c`
    ///   exactly: `v_c^1 = v_c` and `v_j^0 = 1`.
    /// - `0.0_f64.powf(0.0) == 1.0`, so a zero-valued channel carrying zero
    ///   weight drops out of the product; `θ = 0` collapses every coupling to
    ///   `1.0`.
    ///
    /// Because every `v_c ∈ [0,1]` and every `θ_c ≥ 0`, the result is never
    /// NaN, and it is in `[0, 1]` on any libm whose `pow` error stays under one
    /// ulp (Rust documents `powf` precision as platform-dependent).
    ///
    /// Anchor: each `θ` is a monoid homomorphism
    /// `([0,1]^C, ·, 1) → ([0,1], ·, 1)`, the "size" datum Leinster 2013 §1.3
    /// requires for magnitude of a `V`-enriched category to be defined.
    ///
    /// # Examples
    ///
    /// ```
    /// use catgraph_magnitude::ChannelCouplings;
    ///
    /// let mut cc = ChannelCouplings::new(2).unwrap();
    /// cc.set(0, 1, vec![0.5, 0.8]).unwrap();
    ///
    /// assert_eq!(cc.collapse(&[1.0, 0.0]).unwrap(), vec![(0, 1, 0.5)]);
    ///
    /// let mixed = cc.collapse(&[0.5, 0.5]).unwrap();
    /// assert!((mixed[0].2 - 0.4_f64.sqrt()).abs() < 1e-12);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] if `θ` has the wrong length or
    /// contains a NaN, infinite, or negative weight.
    pub fn collapse(&self, theta: &[f64]) -> Result<Vec<(usize, usize, f64)>, CatgraphError> {
        if theta.len() != self.n_channels {
            return Err(CatgraphError::Composition {
                message: format!(
                    "ChannelCouplings::collapse: theta has {} weights, expected {}",
                    theta.len(),
                    self.n_channels
                ),
            });
        }
        for (c, &w) in theta.iter().enumerate() {
            if !w.is_finite() || w < 0.0 {
                return Err(CatgraphError::Composition {
                    message: format!(
                        "ChannelCouplings::collapse: theta[{c}] = {w} — collapse weights must be finite and non-negative"
                    ),
                });
            }
        }

        Ok(self
            .entries
            .iter()
            .map(|(&(from, to), v)| {
                let value = v
                    .iter()
                    .zip(theta)
                    .fold(1.0_f64, |acc, (&vc, &wc)| acc * vc.powf(wc));
                (from, to, value)
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rel_close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
    }

    // -----------------------------------------------------------------------
    // RoleModulation validation — shape, range, and the accessors.
    // -----------------------------------------------------------------------
    #[test]
    fn role_modulation_validation() {
        assert!(RoleModulation::new(vec![]).is_err(), "empty table rejected");
        assert!(
            RoleModulation::new(vec![vec![1.0, 0.5], vec![0.5]]).is_err(),
            "ragged table rejected"
        );
        assert!(
            RoleModulation::new(vec![vec![1.0, 1.5], vec![0.5, 1.0]]).is_err(),
            "entry above 1 rejected"
        );
        assert!(
            RoleModulation::new(vec![vec![1.0, f64::NAN], vec![0.5, 1.0]]).is_err(),
            "NaN entry rejected"
        );
        assert!(
            RoleModulation::new(vec![vec![1.0, -0.1], vec![0.5, 1.0]]).is_err(),
            "negative entry rejected"
        );

        let ok = RoleModulation::new(vec![vec![1.0, 0.5], vec![0.25, 1.0]]).unwrap();
        assert_eq!(ok.n_roles(), 2);
        assert_eq!(ok.rho(0, 1), Some(0.5));
        assert_eq!(ok.rho(1, 0), Some(0.25), "asymmetry preserved");
        assert_eq!(ok.rho(0, 2), None);
        assert_eq!(ok.rho(2, 0), None);
    }

    // -----------------------------------------------------------------------
    // A single-role, single-position grid is the trivial 1-point coalition.
    // -----------------------------------------------------------------------
    #[test]
    fn role_grid_singleton() {
        let one = RoleModulation::new(vec![vec![1.0]]).unwrap();
        let grid = role_grid(&one, &one).unwrap();
        assert_eq!(grid.n_agents(), 1);
        assert!(grid.couplings().is_empty());
        assert_eq!(grid.agent_index(0, 0), Some(0));
        assert_eq!(grid.agent_index(1, 0), None);

        let proof = grid.proof(1.0).unwrap();
        assert!(rel_close(proof.role_magnitude(), 1.0));
        assert!(rel_close(proof.fiber_magnitude(), 1.0));
        assert!(rel_close(proof.expected_magnitude(), 1.0));
        assert_eq!(proof.t(), 1.0);
    }

    // -----------------------------------------------------------------------
    // The grid's flat layout: `role * n + pos`, cross couplings are products.
    // -----------------------------------------------------------------------
    #[test]
    fn role_grid_layout_and_products() {
        let rho = RoleModulation::new(vec![vec![1.0, 0.5], vec![0.2, 1.0]]).unwrap();
        let sigma = RoleModulation::new(vec![
            vec![1.0, 0.9, 0.1],
            vec![0.0, 1.0, 0.9],
            vec![0.0, 0.0, 1.0],
        ])
        .unwrap();
        let grid = role_grid(&rho, &sigma).unwrap();

        assert_eq!(grid.n_agents(), 6);
        assert_eq!(grid.agent_index(0, 2), Some(2));
        assert_eq!(grid.agent_index(1, 0), Some(3));
        assert_eq!(grid.agent_index(0, 3), None);

        // (0,0) → (1,1): ρ(0,1)·σ(0,1) = 0.5·0.9
        assert!(grid.couplings().contains(&(0, 4, 0.5 * 0.9)));
        // (1,0) → (1,2): within-role, so the role factor is the unit diagonal.
        assert!(grid.couplings().contains(&(3, 5, 0.1)));
        // No self-pair, and no zero product: σ(1,0) = 0 kills (0,1) → (0,0).
        assert!(grid.couplings().iter().all(|&(i, j, p)| i != j && p > 0.0));
    }

    // -----------------------------------------------------------------------
    // The diagonal-1.0 precondition is enforced on BOTH factors.
    // -----------------------------------------------------------------------
    #[test]
    fn role_grid_rejects_non_unit_diagonal() {
        let good = RoleModulation::new(vec![vec![1.0, 0.5], vec![0.5, 1.0]]).unwrap();
        let bad = RoleModulation::new(vec![vec![0.9, 0.5], vec![0.5, 1.0]]).unwrap();
        assert!(role_grid(&bad, &good).is_err(), "bad role factor rejected");
        assert!(role_grid(&good, &bad).is_err(), "bad fiber factor rejected");
        assert!(role_grid(&good, &good).is_ok());
    }

    // -----------------------------------------------------------------------
    // ChannelCouplings validation + last-write-wins.
    // -----------------------------------------------------------------------
    #[test]
    fn channel_couplings_validation() {
        assert!(ChannelCouplings::new(0).is_err(), "zero channels rejected");

        let mut cc = ChannelCouplings::new(2).unwrap();
        assert_eq!(cc.n_channels(), 2);
        assert!(cc.set(0, 0, vec![0.5, 0.5]).is_err(), "self-coupling");
        assert!(cc.set(0, 1, vec![0.5]).is_err(), "wrong vector length");
        assert!(cc.set(0, 1, vec![0.5, 1.4]).is_err(), "component above 1");
        assert!(cc.set(0, 1, vec![0.5, f64::NAN]).is_err(), "NaN component");

        cc.set(0, 1, vec![0.5, 0.5]).unwrap();
        cc.set(0, 1, vec![0.25, 1.0]).unwrap(); // last write wins
        assert_eq!(cc.collapse(&[1.0, 0.0]).unwrap(), vec![(0, 1, 0.25)]);

        assert!(cc.collapse(&[1.0]).is_err(), "theta length mismatch");
        assert!(cc.collapse(&[1.0, -0.5]).is_err(), "negative weight");
        assert!(cc.collapse(&[1.0, f64::NAN]).is_err(), "NaN weight");
        assert!(
            cc.collapse(&[1.0, f64::INFINITY]).is_err(),
            "infinite weight"
        );
    }
}
