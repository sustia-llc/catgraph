//! Typed-magnitude valuation surface — roles and channels over the coalition
//! engine (#211, EQ4).
//!
//! Three **additive** layers sit beside the untouched coalition engine. The
//! enrichment of record stays `UnitInterval` (the BTV 2021 `[0,1]` Viterbi rig,
//! lifted by `d = −ln π` into the shared `ζ_t = exp(−t·d)` kernel); nothing here
//! changes a single existing signature, and every layer either *reports* over
//! already-computed data or *transforms the input couplings* before the ordinary
//! entry points consume them.
//!
//! - **T1 — role-share diagnostics.** [`CoalitionEvaluator::role_shares`]
//!   decomposes `Mag(S) = Σ_c w_c` (the weighting of Leinster 2013 Lemma 1.1.4,
//!   already cached by the evaluator) across a caller-supplied role partition of
//!   the members. Types enter *reporting only*; no number moves.
//! - **T2 — role-modulated couplings + the role-grid certificate.**
//!   [`modulate`] applies a role-interaction table `ρ : R × R → [0,1]`
//!   multiplicatively, `π′ = ρ(r_i, r_j) · π` — additively in the metric,
//!   `d′ = d_ρ + d`. [`role_grid`] constructs the product of a role table and a
//!   fiber table, on which the magnitude factorizes exactly
//!   ([`RoleGrid::proof`]).
//! - **T3 — channel-valued couplings + a declared collapse.**
//!   [`ChannelCouplings`] carries a per-pair vector `v ∈ [0,1]^C`;
//!   [`ChannelCouplings::collapse`] contracts it to the scalar enrichment
//!   through a **declared** monoid homomorphism `|v|_θ = Π_c v_c^{θ_c}`.
//!
//! # Extension marker
//!
//! **No "typed" or "colored" magnitude exists in this crate's anchor
//! literature** (Bradley–Vigneaux 2025, BTV 2021, Leinster 2008 / 2013,
//! Leinster–Shulman 2017). The composite surface in this module is an
//! **extension**, marked in the same style as `catgraph-syntax`'s `MatKron`: the
//! individual mechanisms it rests on *are* anchored —
//!
//! - the weighting decomposition `Mag = Σ w` — Leinster 2013 §1.1 Lemma 1.1.4;
//! - `|A ⊗ B| = |A|·|B|` for a product of enriched categories — Leinster 2013
//!   ([arXiv:1012.5857](https://arxiv.org/abs/1012.5857)) Prop 1.4.3 / Prop 2.3.6,
//!   with the weighted categorical (fibration) case at Leinster 2008
//!   ([arXiv:math/0610260](https://arxiv.org/abs/math/0610260)) Prop 2.8;
//! - "magnitude is defined relative to a monoid homomorphism out of the
//!   enriching object monoid" — Leinster 2013 §1.3;
//!
//! — but their assembly into a role/channel vocabulary for coalitions is ours,
//! not a paper's.
//!
//! # The role-grid factorization certificate (T2)
//!
//! [`role_grid`] builds the product of a role table `ρ` (`R` roles) and a fiber
//! table `σ` (`n` positions): agents are pairs `(r, x)` laid out at the flat
//! index `r·n + x`, and **every** ordered cross pair carries the product
//! coupling
//!
//! ```text
//! π((r, x), (r′, x′)) = ρ(r, r′) · σ(x, x′).
//! ```
//!
//! On this construction the coalition magnitude factorizes exactly, and
//! [`RoleGrid::proof`] reports `|ρ| · |σ|` as the expected value. The argument
//! has three steps, and the **diagonal-1.0 precondition** [`role_grid`]
//! enforces on both factors is load-bearing in the first and third.
//!
//! ## Step 1 — product couplings survive the max-product closure
//!
//! `Coalition::from_enriched` closes the coupling table under the Viterbi
//! (max-product) Bellman–Ford closure. Write `ρ̄`, `σ̄` for the closures of the
//! factors and `π̄` for the closure of the product table. Then `π̄ = ρ̄ ⊗ σ̄`:
//!
//! - **`π̄ ≤ ρ̄ ⊗ σ̄` (upper bound).** Any path
//!   `(r₀,x₀) → (r₁,x₁) → … → (r_k,x_k)` has weight
//!   `Π_i ρ(r_{i−1}, r_i) · σ(x_{i−1}, x_i)`, which regroups into
//!   `[Π_i ρ(r_{i−1}, r_i)] · [Π_i σ(x_{i−1}, x_i)]` — the weight of the path's
//!   **role projection** times the weight of its **fiber projection**. Each
//!   projection is a walk in its own factor (a step that holds the role fixed
//!   projects to the idle step `ρ(r, r) = 1`, which by the diagonal-1.0
//!   precondition is free rather than an unmodelled jump), so each is bounded by
//!   the corresponding factor closure.
//! - **`π̄ ≥ ρ̄ ⊗ σ̄` (achieved).** Take an optimal role path
//!   `r₀ → … → r_p` realizing `ρ̄(r₀, r_p)` and an optimal fiber path
//!   `x₀ → … → x_q` realizing `σ̄(x₀, x_q)` (max-product optima with weights
//!   `≤ 1` never revisit a node, so consecutive nodes differ). Concatenate them
//!   as a **two-phase** path in the product: first move the role while holding
//!   the position at `x₀` (each step costs `ρ(r_{i−1}, r_i) · σ(x₀, x₀) =
//!   ρ(r_{i−1}, r_i) · 1`), then move the position while holding the role at
//!   `r_p` (each step costs `ρ(r_p, r_p) · σ(x_{j−1}, x_j) = 1 · σ(x_{j−1},
//!   x_j)`). Its total weight is exactly `ρ̄(r₀, r_p) · σ̄(x₀, x_q)`.
//!
//! Both inequalities hold, so `π̄ = ρ̄ ⊗ σ̄`. Under the BTV lift this is the
//! statement that the product metric is the `ℓ¹` sum
//! `d((r,x), (r′,x′)) = d_ρ(r, r′) + d_σ(x, x′)`.
//!
//! ## Step 2 — magnitude of a product is the product of magnitudes
//!
//! With an `ℓ¹`-additive distance the scaled zeta matrix is a Kronecker
//! product, `ζ_t = ζ_t^ρ ⊗ ζ_t^σ` (because `exp(−t(a+b)) = exp(−ta)·exp(−tb)`),
//! hence `ζ_t⁻¹ = (ζ_t^ρ)⁻¹ ⊗ (ζ_t^σ)⁻¹` and
//!
//! ```text
//! Mag = 1ᵀ ζ_t⁻¹ 1 = (1ᵀ (ζ_t^ρ)⁻¹ 1) · (1ᵀ (ζ_t^σ)⁻¹ 1) = |ρ|_t · |σ|_t.
//! ```
//!
//! This is Leinster 2013 Prop 1.4.3 / Prop 2.3.6 (`|A ⊗ B| = |A||B|`); the
//! weighted categorical form of the same statement — magnitude of a fibration as
//! base times fiber — is Leinster 2008 Prop 2.8. Invertibility likewise
//! factorizes: `ζ_t` is invertible iff both factor zetas are, so [`RoleGrid::proof`]
//! errors on exactly the singular factors the grid evaluation would.
//!
//! ## Step 3 — skeletalization commutes with the product
//!
//! The coalition path quotients members at mutual closure `1.0` before
//! inverting. For values in `[0, 1]`, `fl(a·b) == 1.0 ⟺ a == 1.0 ∧ b == 1.0`
//! (the #153 H1 lemma), so `(r,x) ~ (r′,x′)` in the product **iff** `r ~ r′` in
//! `ρ̄` and `x ~ x′` in `σ̄`. The skeleton of the product is therefore the
//! product of the skeletons, and the same identity holds after quotienting —
//! which matters because [`RoleGrid::proof`] computes each factor magnitude
//! through the ordinary [`coalition_magnitude_from_couplings`] entry point,
//! which skeletalizes too.
//!
//! ## What the certificate is, and is not
//!
//! The certificate is **mathematical and construction-carried**: it holds
//! because [`role_grid`] *built* the product-form couplings, not because
//! anything was detected in the numbers. There is deliberately **no detection
//! path** for arbitrary float inputs — recognizing "is this table a product?"
//! from `f64` data is a bitwise knife-edge hunt, and #153 settled the house rule
//! that we prove from structure instead.
//!
//! Float evaluation only *approximates* both sides: the grid closure multiplies
//! `fl(ρ·σ)` entries along paths while the factor closures multiply `ρ` and `σ`
//! entries separately, so the two routes re-associate the same real products
//! differently. Tests compare within a **relative tolerance**; bit-identity is
//! not claimed anywhere in this module.
//!
//! # The θ-collapse (T3)
//!
//! A channel vector `v ∈ [0,1]^C` is contracted by
//! `|v|_θ = Π_c v_c^{θ_c}` with `θ_c ≥ 0`. For each fixed `θ` this is a monoid
//! homomorphism `([0,1]^C, ·, 1) → ([0,1], ·, 1)` — it maps the componentwise
//! unit to `1` and satisfies `|v ⊙ w|_θ = |v|_θ · |w|_θ` — which is exactly the
//! "size" datum Leinster 2013 §1.3 requires for magnitude of a `V`-enriched
//! category to be defined. Under the `−ln` lift it reads as the convex-style
//! combination `d = Σ_c θ_c d_c` of the per-channel metrics.
//!
//! `θ` is an **experiment parameter, not a theorem**: every `θ` yields a
//! legitimate size homomorphism and the anchor literature picks no canonical
//! one. Recording which `θ` produced a number is the caller's discipline.

use std::collections::BTreeMap;

use crate::coalition::{coalition_magnitude_from_couplings, skeletal_classes};
use crate::coalition_eval::CoalitionEvaluator;
use crate::{CatgraphError, UnitInterval};

/// A role identifier. The vocabulary is **caller-owned** — this crate never
/// interprets a role, it only partitions by equality — matching the coalition
/// surface's plain-index discipline (agents are `usize` indices, not a domain
/// type).
pub type RoleId = usize;

// ---------------------------------------------------------------------------
// T1 — role-share diagnostics
// ---------------------------------------------------------------------------

/// A skeletal class whose members do **not** all carry the same role, reported
/// by [`RoleShares::mixed_classes`] instead of being attributed to any role.
///
/// The coalition engine quotients members at mutual closure `1.0` (perfectly
/// coupled both ways ⇒ Lawvere distance `0`), and it does so **regardless of
/// role**. The cached weighting lives on that quotient, so a class spanning two
/// roles carries a weight that belongs to neither: the unquotiented `ζ` is
/// singular there and per-member weightings are not unique, so splitting the
/// class weight would be an invention. It is flagged here instead.
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

/// The role decomposition of a coalition's magnitude
/// ([`CoalitionEvaluator::role_shares`], #211 T1).
///
/// Fields are private with accessors so later diagnostics can be added without
/// a breaking change — the `JoinReport` / `ConditionReport` house style of
/// #153 / #165. Not `Copy`: it owns the per-role map and the mixed-class list.
///
/// # Contract
///
/// - [`share`](Self::share)`(r)` is the sum of the cached weighting over
///   skeletal classes **all** of whose members carry role `r` (Leinster 2013
///   Lemma 1.1.4: `Mag = Σ_c w_c`).
/// - A role-mixed class contributes to **no** share; it is reported by
///   [`mixed_classes`](Self::mixed_classes).
/// - [`is_exact`](Self::is_exact) is `true` iff no class is mixed. "Exact" is
///   about **attribution**: every class weight is attributed wholly to one role,
///   nothing is split or estimated. It is *not* a bit-identity claim —
///   `Σ_r share(r)` equals [`CoalitionEvaluator::base_value`] only up to
///   floating-point re-association, since the per-role bucketed sums
///   re-associate the row-major accumulation `base_value` uses. Compare with a
///   **relative tolerance**.
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
    /// Decompose the cached `Mag(S)` across a role partition of the members
    /// (#211 T1).
    ///
    /// `roles` is indexed by **member-local position** — `roles[i]` is the role
    /// of `members()[i]`, *not* of agent `i` — so it must have exactly
    /// `members().len()` entries. (Contrast [`modulate`], whose `roles` slice is
    /// indexed by **agent** index.)
    ///
    /// Numbers are untouched: this reads the weighting the evaluator already
    /// cached (`w = μ·1`, Leinster 2013 Lemma 1.1.4) and buckets it by role. See
    /// [`RoleShares`] for the exactness contract — in particular that a
    /// role-mixed skeletal class is reported rather than split, and that
    /// `Σ_r share(r)` matches [`base_value`](Self::base_value) only up to
    /// float re-association.
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

        // The evaluator caches only the class representatives, not the
        // member → class map, so recompute the map with the very helper the
        // slow path uses on the very table the evaluator cached. Deterministic
        // and identical to the partition the cached `reps`/weighting came from
        // (asserted below in debug).
        let (member_classes, recomputed_reps) = skeletal_classes(self.closed_table(), m);
        let reps = self.class_reps();
        let weighting = self.weighting_vec();
        debug_assert_eq!(
            recomputed_reps, reps,
            "role_shares must recover the same skeleton the evaluator cached"
        );
        debug_assert_eq!(
            reps.len(),
            weighting.len(),
            "the cached weighting is indexed by skeletal class"
        );

        // Distinct roles per class, in first-seen order (`k` is small — the
        // skeleton of one coalition — so the linear `contains` beats a set).
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

/// A validated square table of interaction strengths in `[0, 1]`.
///
/// Used in two roles (both are "a `[0,1]`-valued square table", so one type
/// serves): as the role-interaction table `ρ : R × R → [0,1]` of [`modulate`],
/// and as either factor of a [`role_grid`].
///
/// **Asymmetry is explicitly allowed.** `ρ(r, r′)` need not equal `ρ(r′, r)` —
/// "how much a planner's output matters to a worker" is genuinely not "how much
/// a worker's output matters to a planner". The coalition engine handles
/// asymmetric couplings natively (Lawvere `[0, ∞]`-enrichment is one-directional
/// by construction). The one downstream consequence worth knowing: an asymmetric
/// `ρ` makes the modulated coupling table asymmetric, which reduces engagement
/// of the symmetric `f64-fast` factorization path.
///
/// Validation mirrors [`UnitInterval::new`] — every entry must be non-NaN and in
/// `[0, 1]` — by calling it, so range errors are reported identically to every
/// other coupling in the crate.
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
    /// // Asymmetric on purpose: role 0 attends to role 1 more than the reverse.
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
    /// any. Exact comparison is deliberate — see [`role_grid`]'s precondition.
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
/// [`modulate`], carried as its own type so a modulated table is never confused
/// with a raw one.
///
/// Private field with an accessor (the #153 / #165 house style), so the witness
/// can gain provenance later without a breaking change.
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

/// Apply a role-interaction table to a coupling list: `π′ = ρ(r_from, r_to) · π`
/// (#211 T2).
///
/// This is a **pure input transformation** — the result feeds every existing
/// coalition entry point unchanged, and the engine is not aware anything typed
/// happened. Under the BTV lift `d = −ln π` the modulation is additive in the
/// metric, `d′ = d_ρ + d`, so it can only *lengthen* distances (`ρ ≤ 1`), never
/// shorten them.
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

/// The exact expected magnitude of a [`RoleGrid`], carried by construction
/// (#211 T2).
///
/// Not a measurement of the grid: [`RoleGrid::proof`] evaluates each **factor**
/// through the ordinary public coalition entry point and multiplies, which by
/// the module's factorization argument is what the grid must evaluate to. Use it
/// as ground truth against an actual grid evaluation — within a relative
/// tolerance, since the two float routes re-associate the same real products
/// differently.
///
/// Private fields with accessors (the #153 / #165 house style).
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
/// [`role_grid`] (#211 T2).
///
/// Agents are pairs `(role, position)` at the flat index `role·n + position`
/// (`n` = the fiber's size), and every ordered cross pair carries
/// `π((r,x), (r′,x′)) = ρ(r, r′) · σ(x, x′)`. Because the grid was *constructed*
/// this way, its magnitude factorizes exactly — see the module docs for the
/// argument, and [`proof`](Self::proof) for the certificate.
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

    /// The product couplings, ready for any existing coalition entry point.
    ///
    /// Self-pairs are omitted (the identity axiom fixes the diagonal) and so are
    /// zero products (indistinguishable from an absent coupling in the `[0,1]`
    /// rig). Otherwise **every** ordered cross pair is present, so the list has
    /// up to `(R·n)² − R·n` entries — quadratic in the agent count, which bounds
    /// the practical grid size.
    #[must_use]
    pub fn couplings(&self) -> &[(usize, usize, f64)] {
        &self.couplings
    }

    /// The construction-carried certificate: each factor's magnitude at scale
    /// `t`, and their product.
    ///
    /// Both factor magnitudes are computed through the **existing public**
    /// [`coalition_magnitude_from_couplings`] entry point over all of that
    /// factor's points — there is no private re-implementation of magnitude
    /// here, so the certificate and an ordinary evaluation share every kernel
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
    /// is singular — which by the module's Step 2 is exactly when the grid's own
    /// zeta is singular.
    pub fn proof(&self, t: f64) -> Result<RoleFibrationProof, CatgraphError> {
        Ok(RoleFibrationProof {
            role_magnitude: self.role_space.magnitude_at(t)?,
            fiber_magnitude: self.fiber.magnitude_at(t)?,
            t,
        })
    }
}

/// Build the product coalition of a role table and a fiber table, whose
/// magnitude factorizes exactly (#211 T2).
///
/// Agents are pairs `(role, position)` flattened to `role·n + position`; the
/// coupling of an ordered pair is the product
/// `π((r,x), (r′,x′)) = ρ(r, r′) · σ(x, x′)`. See the module docs for the
/// factorization argument and its anchors (Leinster 2013 Prop 1.4.3 /
/// Prop 2.3.6; Leinster 2008 Prop 2.8) — and for the statement that this is a
/// **mathematical, construction-carried** certificate, not a detection of
/// product structure in arbitrary floats.
///
/// # Precondition: both diagonals are exactly `1.0`
///
/// This is validated, not assumed, and it is **load-bearing**. The coalition
/// engine forces the diagonal of the closed table to `1.0` (the identity axiom,
/// `d(x, x) = 0`), so on the grid the self-coupling is `1` whatever the factors
/// say. The identity `(ρ ⊗ σ)` with a forced unit diagonal `= (ρ) ⊗ (σ)` — and
/// with it the two-phase path of the module's Step 1, whose idle steps cost
/// `ρ(r, r)` and `σ(x, x)` — holds only when the factor diagonals are *already*
/// `1.0`. A factor with `ρ(r, r) < 1` would be silently promoted on the grid but
/// not in its own factor evaluation, and the certificate would be wrong. The
/// check is exact `== 1.0`, matching the engine's own exact identity handling.
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

/// Couplings valued in `[0,1]^C` — one component per **channel** (a facet of the
/// interaction: a capability class, a modality, a trust dimension) — plus the
/// declared collapse back to the scalar enrichment (#211 T3).
///
/// This is the full "typed coupling": rather than pre-collapsing per-facet
/// scores by hand before calling the coalition surface, a caller records them
/// all and declares the collapse explicitly with
/// [`collapse`](Self::collapse)`(θ)`.
///
/// Entries are keyed by the ordered pair `(from, to)` with **last-write-wins**
/// on repeats, matching `HomMap::set_hom`'s overwrite and the evaluator's
/// documented duplicate convention. Iteration and [`collapse`](Self::collapse)
/// output are in ascending `(from, to)` order, so results are reproducible.
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
    /// Returns [`CatgraphError::Composition`] if `n_channels == 0` (the empty
    /// product collapses every coupling to `1.0`, which is a degenerate table,
    /// not a useful one).
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
    ///   fixes the diagonal, so the coalition entry points reject self-couplings
    ///   — rejected here too rather than deferred), or if `v.len()` differs from
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
    /// - **`Σ_c θ_c = 1` is a documented convention, not an enforced
    ///   constraint.** The simplex is the natural reading (under `−ln` the
    ///   collapse is `d = Σ_c θ_c d_c`, a convex combination of the per-channel
    ///   metrics, so the result stays commensurable with an untyped coupling),
    ///   but enforcing an exact float sum is a knife-edge test that would reject
    ///   perfectly reasonable weight vectors over rounding. Off-simplex `θ` is
    ///   accepted and simply rescales the metric.
    /// - `θ = e_c` (one at channel `c`, zero elsewhere) recovers channel `c`
    ///   exactly: `v_c^1 = v_c` and `v_j^0 = 1`.
    /// - Rust's `powf` follows IEEE 754: **`0.0_f64.powf(0.0) == 1.0`**, so a
    ///   zero-valued channel carrying zero weight drops out of the product
    ///   rather than annihilating it. In particular `θ = 0` (all zeros) collapses
    ///   **every** coupling to `1.0` — a fully-coupled, fully-degenerate table.
    ///   That is the caller's responsibility, not an error.
    ///
    /// Because every `v_c ∈ [0,1]` and every `θ_c ≥ 0`, the result is in
    /// `[0, 1]` by construction and is never NaN.
    ///
    /// Anchor: each `θ` is a monoid homomorphism
    /// `([0,1]^C, ·, 1) → ([0,1], ·, 1)`, the "size" datum Leinster 2013 §1.3
    /// requires for magnitude of a `V`-enriched category to be defined — so the
    /// magnitude is well-defined **per choice of θ**, and the anchor literature
    /// supplies no canonical θ.
    ///
    /// # Examples
    ///
    /// ```
    /// use catgraph_magnitude::ChannelCouplings;
    ///
    /// let mut cc = ChannelCouplings::new(2).unwrap();
    /// cc.set(0, 1, vec![0.5, 0.8]).unwrap();
    ///
    /// // θ = e_0 recovers channel 0 exactly.
    /// assert_eq!(cc.collapse(&[1.0, 0.0]).unwrap(), vec![(0, 1, 0.5)]);
    ///
    /// // The even mix is the geometric mean.
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
