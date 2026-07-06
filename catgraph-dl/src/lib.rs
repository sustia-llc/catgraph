//! # catgraph-dl
//!
//! Categorical Deep Learning substrate for the catgraph workspace. Anchored to
//! Gavranović, Lessard, Dudzik, von Glehn, Araújo, Veličković, *Categorical
//! Deep Learning is an Algebraic Theory of All Architectures*, ICML 2024
//! ([arXiv:2402.15332v2](https://arxiv.org/abs/2402.15332)).
//!
//! ## Scope
//!
//! Seven public modules. The crate is types + (co)algebra wrappers
//! over `(Set, ×, 1)` by default; non-`(Set, ×, 1)` `MonoidalCategory`
//! instances are deferred until a downstream consumer surfaces.
//!
//! - [`para`] — the 2-category `Para`(M, C). Objects of `C`, 1-morphisms
//!   `(P ∈ M, f : P ▶ X → Y)`, 2-morphisms = reparameterizations
//!   `r : P' → P`. CDL §3.1. Concrete `(Set, ×, 1)` instance via
//!   [`para::SetMonoidal`] / [`para::SetActegory`]; downstream `(Set, ×, 1)`-
//!   flavoured ZSTs opt into the canonical bodies via
//!   [`para::SetCategoryDefaults`].
//! - [`algebra`] — `FAlgebra<F>`, `FCoalgebra<F>`, `MonadAlgebra<M>` plus
//!   homomorphism wrappers `FAlgebraHom` / `FCoalgebraHom` /
//!   `MonadAlgebraHom` with caller-attested `verify_commutes`. The
//!   `Z2Group`-action GDL recovery test in `tests/algebra_homomorphisms.rs`
//!   is the headline reification of CDL §2.1 Ex 2.6 (equivariant maps as
//!   monad-algebra homomorphisms). CDL §2.
//! - [`free_monad`] — explicit `FreeMnd(F)(Z) = Fix(X ↦ F(X) + Z)` and the
//!   cofree-comonad dual via GAT projection. `ListEndo<A>` / `TreeEndo<A>`
//!   bijection witnesses for CDL Examples B.19 / B.20. CDL Proposition B.18.
//! - [`architectures`] — five typed (co)algebra-as-architecture unrollers
//!   (Folding RNN, Unfolding RNN, Recursive NN, Mealy cell, Moore cell)
//!   each shipping a `FreeMnd`-equivalence test in
//!   `tests/architecture_unrollers.rs` reifying CDL Remark 2.13. CDL
//!   Appendix I + Appendix J + Appendix K.
//! - [`endofunctor`] — the `deep_causality_haft` `HKT` / `Functor` witness
//!   substrate (single import seam), shared by `algebra::` and
//!   `free_monad::`. Replaces the former hand-rolled `EndoFunctor` trait
//!   (issue #12).
//! - [`natural`] — first-class [`natural::NaturalTransformation<F, G>`]
//!   (component family `α_X : F(X) → G(X)`; Gavranović et al. Def 1.5) with
//!   [`natural::IsoForward`] / [`natural::IsoBackward`] adapters over haft's
//!   `NaturalIso`, and the blanket [`natural::Pointed`] endofunctor marker
//!   `(F, σ)` with `σ = ` haft's `Pure` (CDL Def B.3). Issue #41.
//! - [`container`] — the [`container::Container`] shape/position presentation of
//!   a polynomial endofunctor `⟦S ◁ P⟧(X) = Σ_{s} X^{P(s)}`
//!   (Abbott–Altenkirch–Ghani 2003, via CDL), finitary (`Vec`-of-contents)
//!   presentation. Issue #41.
//! - `hopf_fibration` (private) — namespace stub for Dudzik's carry-operation
//!   conjecture. Pre-publication research; not in CDL ICML 2024. Not part
//!   of the public surface. See ⚠️ CAREFUL section below for the 2026-05-06
//!   Filter Equivariants follow-up evidence update.
//!
//! ## Substrate
//!
//! Re-exports the Tier 3 enrichment infrastructure from `catgraph-applied`
//! — [`Rig`], [`UnitInterval`], [`Tropical`], [`F64Rig`], [`BoolRig`],
//! [`EnrichedCategory`], [`HomMap`], [`LawvereMetricSpace`].
//!
//! ## Relationship to other workspace members
//!
//! - **`catgraph-applied`** — provides `Rig` and the `EnrichedCategory<V>`
//!   substrate. `catgraph-dl::para::Actegory<M, C>` is the 2-categorical
//!   refinement: `Rig` gives elements; `Actegory` gives morphisms and the
//!   coherence witness `μ : Q ⊗ (P ▶ X) → (Q ⊗ P) ▶ X`.
//! - **`catgraph-magnitude`** — orthogonal; magnitude is a scalar invariant
//!   (Möbius sum over a `Ring`-enriched category), Para is the 2-category of
//!   parametric morphisms. Future bridge: `catgraph-magnitude`
//!   Para-over-Rig actegory-enriched magnitude (deferred).
//! - **`catgraph-physics`** — `evolution_cospan` is a *deterministic
//!   projection* of a Para F-algebra trajectory; `FreeMnd(F)` specialises to
//!   cospan chains when `F` is the cospan-step endofunctor. Cross-reference
//!   only; no code shared.
//!
//! ## Deferred surfaces
//!
//! Surfaces explicitly held until a downstream consumer surfaces a need.
//! See the "Deferred surfaces" section of the crate README for the full
//! list. Highlights:
//!
//! - **Non-`(Set, ×, 1)` `MonoidalCategory` instances** — R-module actegory,
//!   hyperdoctrine, vector-bundle, fibration-based. Trait surface admits
//!   them; concrete instances deferred. The [`para::SetCategoryDefaults`]
//!   sub-trait closes the boilerplate gap for `(Set, ×, 1)`-flavoured ZSTs
//!   only.
//! - **The Hopf-fibration / carry-operation construction** — private
//!   namespace stub only; held until a Dudzik preprint exists. See ⚠️
//!   CAREFUL section below for the 2026-05-06 evidence update.
//! - **Truly-infinite final-coalgebra semantics** for [`architectures::UnfoldingRnn`]
//!   (lazy / `Iterator` / `tokio_stream::Stream` carrier). Bounded-depth
//!   `unroll_to_vec` is the shipped surface; lazy variant deferred.
//! - **Upstream haft adoption of `Pointed` / `NaturalTransformation`** — the
//!   first-class surfaces themselves shipped in [`natural`] and [`container`]
//!   ([#41](https://github.com/sustia-llc/catgraph/issues/41)); what remains
//!   deferred is proposing `Pointed` / `NaturalTransformation` to
//!   `deep_causality_haft` itself, so cg-dl re-exports rather than defines
//!   them — tracked as
//!   [#62](https://github.com/sustia-llc/catgraph/issues/62).
//! - **Symbiogenesis / Levin bioelectric / active inference / cellular-
//!   automata coalitions** — ambitious tier; lands in a future external
//!   sibling `catgraph-coalition-dl`, not here.
//!
//! ## ⚠️ CAREFUL — provenance of the Hopf-fibration claim
//!
//! The private `hopf_fibration` module reserves namespace for Andrew
//! Dudzik's transcript-only conjecture about modular-arithmetic carry as a
//! non-trivial `S¹`-fibration of `S³ → S²`. **This is not a result of the
//! published CDL ICML 2024 paper.** Treat as pre-publication research; do
//! not cite the Hopf-fibration claim as co-authored by Gavranović et al.
//! until a preprint exists.
//!
//! **2026-05-06 evidence update.** The most recent published Dudzik-co-authored
//! work, *Filter Equivariant Functions* ([arXiv:2507.08796v1](https://arxiv.org/abs/2507.08796),
//! July 2025), §6 explicitly puts ripple-carry addition **outside** the FE
//! framework. As of 2026-05-06 no Hopf-fibration / carry-operation preprint
//! exists. The `hopf_fibration` private namespace stub is therefore kept
//! reserved with no public API. See `src/hopf_fibration/mod.rs` for the full
//! evidence trail.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod algebra;
pub mod architectures;
pub mod container;
pub mod endofunctor;
pub mod free_monad;
mod hopf_fibration;
pub mod natural;
pub mod para;

// Top-level convenience re-export: the endofunctor abstraction is now
// `deep_causality_haft`'s `HKT` (object map) + `Functor` (morphism map),
// shared between `algebra::` (F-algebras and homomorphisms) and
// `free_monad::` (recursive `FreeMnd` / `CofreeCmnd`). `Either` is the sum
// carried by `TreeEndo`. `Pure` and `NaturalIso` are mirrored here too:
// implementing `Pointed` downstream requires `Pure<Self>`, and driving the
// `IsoForward` / `IsoBackward` adapters requires naming `NaturalIso`. The
// former `catgraph_dl::EndoFunctor` path is removed (breaking; issue #12).
pub use endofunctor::{
    Either, EndoWitness, Functor, HKT, NaturalIso, NoConstraint, Pure, Satisfies,
};

// The first-class natural-transformation / pointed-endofunctor / container
// surfaces layered on the endofunctor witnesses (issue #41). Same crate-root
// re-export convention as the modules above.
pub use container::Container;
pub use natural::{IsoBackward, IsoForward, NaturalTransformation, Pointed};

// Re-exports of the Tier 3 enrichment substrate from catgraph-applied. Same
// pattern as `catgraph-magnitude` — a single import path for downstream
// consumers needing both the `Rig` scalar abstraction and CDL's `Para`
// 2-category construction.
pub use catgraph_applied::enriched::{EnrichedCategory, HomMap};
pub use catgraph_applied::lawvere_metric::LawvereMetricSpace;
pub use catgraph_applied::rig::{BoolRig, F64Rig, Rig, Tropical, UnitInterval};
