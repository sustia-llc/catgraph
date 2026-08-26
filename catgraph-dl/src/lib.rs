//! # catgraph-dl
//!
//! Categorical Deep Learning substrate for the catgraph workspace. Anchored to
//! Gavranović, Lessard, Dudzik, von Glehn, Araújo, Veličković, *Categorical
//! Deep Learning is an Algebraic Theory of All Architectures*, ICML 2024
//! ([arXiv:2402.15332v2](https://arxiv.org/abs/2402.15332)).
//!
//! - [`para`] — the 2-category `Para(M, C)` (CDL §3.1): `(Set, ×, 1)` via
//!   [`para::SetMonoidal`] / [`para::SetActegory`] / [`para::SetCategoryDefaults`];
//!   the R-module actegory `(FinReal, ⊕, R⁰)` via [`para::RMonoidal`] /
//!   [`para::RActegory`] / [`para::RModule`] and their `f64` aliases
//!   (CDL Def E.2 / Ex E.4 / Ex G.3). Feature `ad`: `Dual<f64>` scalar
//!   (forward-mode AD). Feature `serde`: derives on the parameter carriers.
//! - [`algebra`] — `FAlgebra<F>`, `FCoalgebra<F>`, `MonadAlgebra<M>`, their
//!   homomorphism wrappers with sampled `verify_*` checks (CDL §2).
//! - [`free_monad`] — `FreeMnd(F)`, `CofreeCmnd(F)`, [`Free`] / [`Cofree`],
//!   `ListEndo` / `TreeEndo` bijections (CDL Prop B.18, Ex B.19 / B.20).
//! - [`architectures`] — Folding RNN, Unfolding RNN, Recursive NN, Mealy and
//!   Moore cells as (co)algebra unrollers (CDL App I, App J).
//! - [`endofunctor`] — the `HKT` / `Functor` witness substrate.
//! - [`natural`] — [`natural::NaturalTransformation<F, G>`] (Def 1.5),
//!   [`natural::Pointed`] with `σ = ` [`Pure`] (CDL Def B.3).
//! - [`container`] — [`container::Container`], polynomial endofunctor
//!   `⟦S ◁ P⟧(X) = Σ_{s} X^{P(s)}` (Abbott–Altenkirch–Ghani 2003).
//! - [`depth`] — opt-in depth measures and guards for the tree carriers.
//! - [`errors`] — [`errors::DepthError`].
//! - `hopf_fibration` (private) — namespace stub for Dudzik's carry-operation
//!   conjecture; not in CDL ICML 2024, no public API, no preprint as of
//!   2026-05-06 (see `src/hopf_fibration/mod.rs`).
//!
//! Re-exports from `catgraph-applied`: [`Rig`], [`UnitInterval`], [`Tropical`],
//! [`F64Rig`], [`BoolRig`], [`EnrichedCategory`], [`HomMap`],
//! [`LawvereMetricSpace`].

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod algebra;
pub mod architectures;
pub mod container;
pub mod depth;
pub mod endofunctor;
pub mod errors;
pub mod free_monad;
mod hopf_fibration;
pub mod natural;
pub mod para;

// Top-level convenience re-export: the endofunctor abstraction is `HKT` (object
// map) + `Functor` (morphism map), shared between `algebra::` (F-algebras and
// homomorphisms) and `free_monad::` (the recursive `Free` / `Cofree` carriers).
// `Either` is the sum carried by `TreeEndo`. `Pure`, `NaturalIso`, and `Monad`
// are mirrored here too: implementing `Pointed` downstream requires
// `Pure<Self>`, driving the `IsoForward` / `IsoBackward` adapters requires
// naming `NaturalIso`, and the monad-algebra verifiers bound
// `M: EndoWitness + Monad<M>`. `Free` / `Cofree` (+ their `FreeWitness` /
// `CofreeWitness` HKT witnesses) and the `EqFunctor` / `DebugFunctor`
// capability traits their opt-in `Eq`/`Debug` route through are mirrored for
// the free-monad surface (issue #93). The former `catgraph_dl::EndoFunctor`
// path is removed (breaking; issue #12), as are `NoConstraint` / `Satisfies`
// (breaking; issue #222 — the object map carries no constraint slot).
pub use endofunctor::{
    Cofree, CofreeWitness, DebugFunctor, Either, EndoWitness, EqFunctor, Free, FreeView,
    FreeWitness, Functor, HKT, Monad, NaturalIso, Pure,
};

// The first-class natural-transformation / pointed-endofunctor / container
// surfaces layered on the endofunctor witnesses (issue #41). Same crate-root
// re-export convention as the modules above.
pub use container::Container;
pub use natural::{IsoBackward, IsoForward, NaturalTransformation, Pointed};

// The recursion guard's rejection (issue #231). Mirrored at the crate root on
// the same convention as the surfaces above: three public entries return it, so
// callers should not have to reach into `errors::` to name it.
pub use errors::DepthError;

// Re-exports of the Tier 3 enrichment substrate from catgraph-applied. Same
// pattern as `catgraph-magnitude` — a single import path for downstream
// consumers needing both the `Rig` scalar abstraction and CDL's `Para`
// 2-category construction.
pub use catgraph_applied::enriched::{EnrichedCategory, HomMap};
pub use catgraph_applied::lawvere_metric::LawvereMetricSpace;
pub use catgraph_applied::rig::{BoolRig, F64Rig, One, Rig, Tropical, UnitInterval, Zero};
