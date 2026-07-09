//! The 2-category `Para`(M, C) — parametric morphisms.
//!
//! CDL §3.1. Objects are objects of an `M`-actegory `C`; 1-morphisms `X → Y`
//! are pairs `(P ∈ M, f : P ▶ X → Y)`; 2-morphisms `(P, f) ⇒ (P', f')` are
//! reparameterizations `r : P' → P` making the parameter-substitution
//! triangle commute.
//!
//! Sequential composition of `(P, f) : X → Y` with `(Q, g) : Y → Z` gives
//! `(Q ⊗ P, h)` where
//!
//! ```text
//! h : (Q ⊗ P) ▶ X --μ--> Q ▶ (P ▶ X) --Q ▶ f--> Q ▶ Y --g--> Z
//! ```
//!
//! Weight tying is the special case of reparameterization by the diagonal
//! comonoid `Δ_P : P → P × P` (CDL Theorem G.10 — lax algebras for `Para(T)`
//! induce comonoids).
//!
//! ## Status
//!
//! Bodies present for the concrete `M = (Set, ×, 1)` actegory acting on
//! `Set` by Cartesian product — see [`SetMonoidal`] / [`SetActegory`].
//! [`ParaMorphism::compose`] and [`Reparameterization::apply`] have
//! Set-specialised implementations.
//!
//! A second, non-`(Set, ×, 1)` instance is also shipped: the R-module actegory
//! `(FinReal, ⊕, R⁰)` — see [`F64Monoidal`] / [`F64Actegory`] / [`F64Module`]
//! (issue #36). It is the direct-sum monoidal category of finite-dimensional
//! real modules acting on itself; its object-level tensor is the dedicated
//! [`DirectSum`] carrier rather than the `(Set, ×, 1)` tuple, so it does not
//! opt into [`SetCategoryDefaults`]. `ParaMorphism` / `Reparameterization`
//! composition remains Set-specialised; other monoidal categories
//! (hyperdoctrine, vector-bundle, fibration) are still deferred.
//!
//! ## Closure convention
//!
//! Underlying maps `f : P ▶ X → Y` use the tuple-input convention
//! `Fn((P, X)) -> Y`. This matches `architectures::*` (e.g.
//! `fn((f32, u8, u32)) -> u32`) and lets the composed/reparameterised
//! actions compose without intermediate adapters.

mod actegory;
mod comonoid;
mod module_actegory;
mod monoidal_category;
mod morphism;
mod reparameterization;

pub use actegory::{Actegory, SetActegory};
pub use comonoid::{Comonoid, DiagonalComonoid, tie_weights};
pub use module_actegory::{DirectSum, F64Actegory, F64Module, F64Monoidal, F64Morphism, F64Object};
pub use monoidal_category::{
    MonoidalCategory, MonoidalTag, SetCategoryDefaults, SetMonoidal, SetMorphism, SetObject,
    private::Sealed,
};
pub use morphism::{Para, ParaMorphism};
pub use reparameterization::Reparameterization;
