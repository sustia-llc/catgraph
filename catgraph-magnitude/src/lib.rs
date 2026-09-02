//! # catgraph-magnitude
//!
//! Magnitude of enriched categories over the catgraph cospan substrate.
//! Anchors: Bradley & Vigneaux, *The Magnitude of Categories of Texts Enriched
//! by Language Models* (2025); Bradley–Terilla–Vlassopoulos, *An Enriched
//! Category Theory of Language* (2021); Leinster 2008 / 2013 / 2017.
//!
//! ## Modules
//!
//! - [`weighted_cospan`] — `catgraph::Cospan` carrying per-edge weights in a
//!   rig `Q`.
//! - [`magnitude`] — Tsallis entropy `H_t(p) = (1 − Σ pᵢᵗ)/(t−1)`, Möbius
//!   inversion over a [`Ring`] by Gaussian elimination, weighting and
//!   coweighting (Leinster 2013 Def 1.1.1), the scattered predicate
//!   (Def 2.1.2), and magnitude as a Möbius sum.
//! - [`mobius_chains`] — Möbius inversion as the von-Neumann series
//!   `μ = Σ (−1)ᵏ Mᵏ` with `M = ζ − I` (Leinster 2013 Prop 2.1.3, Leinster
//!   2008 Cor 1.5), and the per-grade signed chain count (Leinster–Shulman
//!   2017 §3 grading).
//! - [`chain_complex`] — magnitude-homology chain complex `(C_{k,ℓ}, ∂_k)` over
//!   a Lawvere metric, its ranks, and the Euler-characteristic identity
//!   (Leinster–Shulman 2017 §3, BV 2025 Prop 3.14).
//! - [`snf`] — Smith Normal Form over `MatR<Q>` (Storjohann §7; algorithmic
//!   reference [events555/modularsnf](https://github.com/events555/modularsnf)
//!   @ `d62535e`, Apache-2.0).
//! - [`poset_category`] — [`PosetCategory`], the input type for integer-exact
//!   Möbius inversion (Leinster 2008 Cor 1.5).
//! - [`lm_category`] — materialized language-model transition table with
//!   `Mag(tM)` (BV 2025 Prop 3.10).
//! - [`yoneda`] and [`semantic`] — the BTV 2021 Yoneda embedding `x ↦ L(x, −)`
//!   with its asymmetric semantic hom and distance (Lemma 2 Eq 11 / §5), plus
//!   bidirectional nearest-meaning ranking and symmetric single-linkage
//!   clustering over it.
//! - [`determinism`] — the count of covering distance-0 (`π = 1`) transitions,
//!   `MH_1(ℓ = 0)`.
//! - [`coalition`] — coalition diversity `Mag(tA|members)` over an
//!   [`EnrichedCategory`]`<UnitInterval>` restricted to members, max-product
//!   closed and perfectly-coupled quotiented (BV 2025 §3.5 Eq 7 Möbius sum);
//!   [`coalition_value`] is the pinned `t = 1` scalar.
//! - [`coalition_eval`] — incremental `Mag(S ∪ {x})` over a cached base
//!   coalition, reporting the update path, the Schur complement, and an exact
//!   zero-diversity proof when one of three structurally decidable classes
//!   fires. An **extension** beside the anchor engine.
//! - [`coalition_typed`] — role decomposition of `Mag = Σ w`, role-interaction
//!   modulation of couplings with an exactly factorizing product coalition, and
//!   per-pair channel vectors in `[0,1]^C` collapsed by `|v|_θ = Π v_c^{θ_c}`
//!   (Leinster 2013 Lemma 1.1.4, §1.3, Prop 1.4.3, Prop 2.3.6; Leinster 2008
//!   Prop 2.8). An **extension** — no typed or colored magnitude exists in the
//!   anchor literature.
//! - `magnitude_f64` (feature `f64-fast`, off by default) — one symmetric ζ
//!   factorization (Cholesky → Bunch–Kaufman `LBLT` → rig-generic
//!   Gauss–Jordan) serving weighting, coweighting, magnitude and Möbius over
//!   `f64`, plus `cond₁(ζ)` reporting. Numerical, not paper-anchored.
//!
//! ## Substrate
//!
//! Re-exports the Tier 3 enrichment infrastructure from `catgraph-applied` —
//! [`Rig`], [`Zero`], [`One`], [`UnitInterval`], [`Tropical`], [`F64Rig`],
//! [`BoolRig`], [`EnrichedCategory`], [`HomMap`], [`LawvereMetricSpace`],
//! [`MatR`], [`ZAlgebra`], [`Z`] — and [`CatgraphError`] from `catgraph`.
//!
//! ## Algebraic scoping
//!
//! Möbius inversion ships in two flavours. The field-fast path,
//! [`mobius_function`](magnitude::mobius_function)`::<Q: Ring + Div + From<f64>>`,
//! runs Gaussian elimination on `[ζ | I]` and requires multiplicative inverses.
//! The chain-sum path,
//! [`mobius_function_via_chains`](mobius_chains::mobius_function_via_chains)`::<Q: Ring + From<f64>>`,
//! sums `μ = Σ (−1)ᵏ Mᵏ` with `M = ζ − I`, needs no `Div`, and requires
//! scattered input (Leinster 2013 Def 2.1.2: `d(a, b) > log(#A − 1)`).
//!
//! `Tropical`-valued and `BoolRig`-valued magnitude are out of scope: the
//! scalar rig `k` is determined by V (V = `[0,∞]` ⇒ k = ℝ), Leinster 2013 §1.3
//! Examples 1.3.1.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod ring;

pub mod lm_category;
pub mod magnitude;
#[cfg(feature = "f64-fast")]
pub mod magnitude_f64;
pub mod weighted_cospan;

pub mod yoneda;
pub use yoneda::{Copresheaf, semantic_distance, semantic_distance_sym, semantic_hom};

pub mod semantic;
pub use semantic::{cluster_semantic_sym, k_nearest_from, k_nearest_to};

pub mod determinism;

pub mod coalition;
pub use coalition::{
    Coalition, coalition_magnitude, coalition_magnitude_from_couplings, coalition_value,
};

pub mod coalition_eval;
pub use coalition_eval::{
    CoalitionEvaluator, EvalPath, EvalScratch, INCREMENTAL_REL_TOL, JoinReport, ZeroDiversityProof,
    coalition_value_delta,
};

pub mod coalition_typed;
pub use coalition_typed::{
    ChannelCouplings, MixedClass, ModulatedCouplings, RoleFibrationProof, RoleGrid, RoleId,
    RoleModulation, RoleShares, modulate, role_grid,
};

pub mod mobius_chains;

pub mod chain_complex;
pub mod snf;

pub mod poset_category;
pub use poset_category::PosetCategory;

// Tier 3 enrichment substrate, re-exported from catgraph-applied.
pub use catgraph::errors::CatgraphError;
pub use catgraph_applied::enriched::{EnrichedCategory, HomMap};
pub use catgraph_applied::lawvere_metric::LawvereMetricSpace;
pub use catgraph_applied::mat::MatR;
pub use catgraph_applied::rig::{BoolRig, F64Rig, One, Rig, Tropical, UnitInterval, Zero};

// `ZAlgebra` names a Z-algebra — a unital-ring extension carrying a canonical
// `ℤ → R` homomorphism — not an integer-valued type.
pub use catgraph_applied::ZAlgebra;
pub use catgraph_applied::z::Z;

pub use ring::Ring;

/// Threshold for the Shannon branch of
/// [`tsallis_entropy`](magnitude::tsallis_entropy): for `|t − 1| < ε` it
/// returns `-Σ pᵢ ln pᵢ` instead of `(1 − Σ pᵢᵗ)/(t − 1)`.
///
/// A Rem 3.11 / Eq (12) finite-difference step `h` must satisfy
/// `h > TSALLIS_SHANNON_EPS`; at `h ≤ ε` both `f(1+h)` and `f(1−h)` take the
/// Shannon branch and the central difference is identically zero.
pub const TSALLIS_SHANNON_EPS: f64 = 1e-6;

/// Absolute tolerance for triangle-inequality checks on `−ln`-derived
/// distances, in the distance (log) domain.
///
/// In [`coalition`] and [`weighted_cospan`], `−ln π(x, z) ≤ (−ln π(x, y)) +
/// (−ln π(y, z))` is compared across a `−ln`-of-product versus sum-of-`−ln`s
/// rewrite, and the two forms differ by ULPs of rounding on non-dyadic
/// couplings. Consumed via
/// [`LawvereMetricSpace::triangle_inequality_holds_within`](catgraph_applied::lawvere_metric::LawvereMetricSpace::triangle_inequality_holds_within).
pub(crate) const TRIANGLE_FLOAT_TOL: f64 = 1e-9;
