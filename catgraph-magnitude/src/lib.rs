//! # catgraph-magnitude
//!
//! Magnitude of enriched categories for the catgraph workspace. Anchored to
//! Bradley & Vigneaux, *Magnitude of Language Models* (2025) and to
//! Leinster, *The magnitude of metric spaces* (2013).
//!
//! ## Scope
//!
//! - [`WeightedCospan<Λ, Q>`](weighted_cospan::WeightedCospan) — newtype over
//!   `catgraph::Cospan` with per-edge weights in a rig `Q`.
//! - [`tsallis_entropy`](magnitude::tsallis_entropy) — `H_t(p) = (1 − Σ pᵢᵗ)/(t−1)`,
//!   special-cased to Shannon at `|t−1| < TSALLIS_SHANNON_EPS`.
//! - [`mobius_function`](magnitude::mobius_function) — Möbius inversion
//!   `ζ · μ = I` over a [`Ring`] via Gaussian elimination (field-fast path,
//!   v0.1.x; requires `Q: Ring + Div + From<f64>`).
//! - [`mobius_function_via_chains`](mobius_chains::mobius_function_via_chains)
//!   — Leinster 2013 Prop 2.1.3 chain-sum formula, realized as the
//!   von-Neumann series `μ = Σ (−1)ᵏ Mᵏ` with `M = ζ − I` (v0.2.0;
//!   requires `Q: Ring + From<f64>` + scattered input).
//! - [`chain_count_signed_graded`](mobius_chains::chain_count_signed_graded) — per-grade
//!   signed chain-count diagnostic (renamed v0.4.0 §1.19 from
//!   `mobius_chains_graded`); Leinster 2013 Prop 2.1.3 + LS 2017 §2 grading
//!   (v0.3.0; requires `Q: Rig + From<f64>`).
//! - [`is_mobius_invertible_at`](magnitude::is_mobius_invertible_at) — Leinster
//!   2013 Prop 2.4.17 ergonomic Möbius-existence threshold check (v0.3.0).
//! - [`chain_complex`] — LS 2017 §2 magnitude-homology chain complex `(C_{k,ℓ}, ∂_k)`
//!   over Lawvere metric: [`Chain`](chain_complex::Chain),
//!   [`enumerate_chains`](chain_complex::enumerate_chains),
//!   [`ChainIndex`](chain_complex::ChainIndex),
//!   [`boundary_matrix`](chain_complex::boundary_matrix),
//!   [`magnitude_homology_rank`](chain_complex::magnitude_homology_rank),
//!   [`euler_char_identity_at`](chain_complex::euler_char_identity_at) (v0.3.0).
//! - [`snf`] — custom Storjohann §7 Smith Normal Form backend over `MatR<Q>`
//!   (algorithmic reference: [events555/modularsnf](https://github.com/events555/modularsnf)
//!   @ `d62535e`, Apache-2.0; dev-only oracle gated by `modularsnf-oracle`,
//!   NOT a runtime dep) (v0.3.0).
//! - [`weighting`](magnitude::weighting) / [`coweighting`](magnitude::coweighting)
//!   — Leinster 2013 §1.1 Def 1.1.1 paper-foundational primitives (v0.2.0).
//! - [`is_scattered`](magnitude::is_scattered) — Leinster 2013 Def 2.1.2
//!   convergence predicate (v0.2.0).
//! - [`magnitude`](magnitude::magnitude) — magnitude via Möbius sum.
//! - [`LmCategory`](lm_category::LmCategory) — materialized language-model
//!   transition table with `Mag(tM)` per BV 2025 Thm 3.10.
//! - [`semantic`] — comparison / clustering over the BTV 2021 Yoneda embedding:
//!   [`LmCategory::yoneda_all`](lm_category::LmCategory::yoneda_all) (batch
//!   meanings), [`k_nearest_from`] / [`k_nearest_to`] (bidirectional
//!   nearest-meaning ranking, asymmetric per BTV §5), and
//!   [`cluster_semantic_sym`] (symmetric single-linkage convenience) (#21).
//!
//! ## Substrate
//!
//! Re-exports the Tier 3 enrichment infrastructure from `catgraph-applied`
//! v0.5.x — [`Rig`], [`UnitInterval`], [`Tropical`], [`F64Rig`], [`BoolRig`],
//! [`EnrichedCategory`], [`HomMap`], [`LawvereMetricSpace`].
//!
//! ## Algebraic scoping
//!
//! Möbius inversion ships in two paper-faithful flavours:
//!
//! - **Field-fast path** —
//!   [`mobius_function`](magnitude::mobius_function)`::<Q: Ring + Div + From<f64>>`
//!   via Gaussian elimination on `[ζ | I]`. Requires multiplicative
//!   inverses (the `Div` bound). v0.1.x.
//! - **Chain-sum path** —
//!   [`mobius_function_via_chains`](mobius_chains::mobius_function_via_chains)`::<Q: Ring + From<f64>>`
//!   via the von-Neumann series `μ = Σ (−1)ᵏ Mᵏ` with `M = ζ − I`
//!   (algebraically identical to Leinster 2013 Prop 2.1.3's
//!   chain-sum-of-ζ-products by `Mᵏ[a][b] = Σ chain-products of length k`).
//!   No `Div` needed; requires the input to be **scattered** (Leinster
//!   Def 2.1.2: `d(a, b) > log(#A − 1)`). v0.2.0.
//!
//! Among the workspace's four concrete rigs, only `F64Rig` satisfies
//! either bound in v0.2.0; the wider `Q: Ring + From<f64>` bound on the
//! chain-sum path is forward-compat for any future `Ring`-rig.
//!
//! **`Tropical`-valued / `BoolRig`-valued magnitude is out of scope per
//! Leinster 2013 §1.3 Examples 1.3.1**: the scalar rig `k` is determined
//! by V (V = `[0,∞]` ⇒ k = ℝ). See `docs/BV25-AUDIT.md` §"Out of scope
//! (v0.2.x)" for the full citation chain.
//!
//! ## Numerical scoping
//!
//! [`TSALLIS_SHANNON_EPS`] = `1e-6` is the threshold below which
//! [`tsallis_entropy`](magnitude::tsallis_entropy) returns the Shannon limit
//! `-Σ pᵢ ln pᵢ` directly, avoiding catastrophic cancellation in the
//! `O(1e-9) / 1e-9` regime around `t = 1`. The Cor 3.14 finite-difference
//! step `h` must satisfy `h > TSALLIS_SHANNON_EPS` so both `f(1±h)` evaluate
//! the Tsallis branch (recommended `h = 1e-4`, ~2 decimal margin above the
//! threshold while staying near `f64`'s `ε^(1/3) ≈ 6e-6` truncation+roundoff
//! optimum).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod ring;

// Phase 6A.1 / 6A.2 / 6A.3 module stubs — populated in subsequent commits.
pub mod lm_category;
pub mod magnitude;
pub mod weighted_cospan;

// D2 — BTV 2021 Yoneda embedding `x ↦ L(x, −)`: representable copresheaves
// (meaning-as-distribution) over the LM-enriched category + the asymmetric
// semantic hom/distance (BTV 2021 Lemma 2 Eq 11 / §5).
pub mod yoneda;
pub use yoneda::{Copresheaf, semantic_distance, semantic_distance_sym, semantic_hom};

// #21 — semantic text comparison/clustering over the Yoneda embedding: batch
// Yoneda image (`yoneda_all`), bidirectional k-nearest ranking, and symmetric
// single-linkage clustering (BTV 2021 Lemma 2 Eq 11 / §5 asymmetry).
pub mod semantic;
pub use semantic::{cluster_semantic_sym, k_nearest_from, k_nearest_to};

// Deterministic-transition rank — `MH_1(ℓ=0)` = #covering distance-0 (π=1)
// transitions (a BV 2025 / Leinster–Shulman structural invariant; see module).
pub mod determinism;

// Phase 6F (v0.2.0) — chain-sum Möbius via Leinster 2013 Prop 2.1.3.
pub mod mobius_chains;

// Phase 6F (v0.3.0) — magnitude homology (BV 2025 Prop 3.14, Leinster–Shulman 2017 §2)
// + custom Storjohann SNF over `MatR<Q>` (algorithmic reference: events555/modularsnf @ d62535e).
pub mod chain_complex;
pub mod snf;

// Phase H (v0.4.0 §1.17) — `PosetCategory<NodeId>` input type for integer-exact
// Möbius inversion (Leinster 2008 *The Euler characteristic of a category* Cor 1.5).
pub mod poset_category;
pub use poset_category::PosetCategory;

// Re-exports of the Tier 3 enrichment substrate from catgraph-applied.
pub use catgraph::errors::CatgraphError;
pub use catgraph_applied::enriched::{EnrichedCategory, HomMap};
pub use catgraph_applied::lawvere_metric::LawvereMetricSpace;
pub use catgraph_applied::mat::MatR;
pub use catgraph_applied::rig::{BoolRig, F64Rig, Rig, Tropical, UnitInterval};

// v0.4.0 §1.17 substrate re-exports for integer-exact Möbius
// (renamed v0.6.0: `Integer` → `ZAlgebra`; the trait names a Z-algebra
// — a unital-ring extension carrying a canonical `ℤ → R` homomorphism —
// not the mathematical concept of an integer-valued type. Cf. Bourbaki
// *Algèbre* Ch. I §8 — ℤ as initial object of the category of unital rings).
pub use catgraph_applied::ZAlgebra;
pub use catgraph_applied::z::Z;

pub use ring::Ring;

/// Threshold for the Shannon special case in
/// [`tsallis_entropy`](magnitude::tsallis_entropy). For `|t − 1| < ε`, the
/// function returns `-Σ pᵢ ln pᵢ` directly, avoiding catastrophic cancellation
/// in the `(1 − Σ pᵢᵗ)/(t − 1)` ≈ `0/0` regime.
///
/// The Cor 3.14 finite-difference step `h` MUST satisfy
/// `h > TSALLIS_SHANNON_EPS`, otherwise both `f(1+h)` and `f(1−h)` evaluate
/// the Shannon branch and the central difference collapses to identically
/// zero. Recommended `h = 1e-4` — see crate-level docs.
pub const TSALLIS_SHANNON_EPS: f64 = 1e-6;
