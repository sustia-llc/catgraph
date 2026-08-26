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
//! Weight tying is reparameterization by the diagonal comonoid
//! `Δ_P : P → P × P` (CDL Thm G.10).
//!
//! Instances: `(Set, ×, 1)` — [`SetMonoidal`] / [`SetActegory`]; the R-module
//! actegory `(FinReal, ⊕, R⁰)` — [`RMonoidal`] / [`RActegory`] / [`RModule`] /
//! [`DirectSum`] with `f64` aliases [`F64Monoidal`] / [`F64Actegory`] /
//! [`F64Module`]. [`ParaMorphism::compose`] and [`Reparameterization::apply`]
//! are `SetMonoidal`-specialised. Feature `ad` adds `Dual<f64>` as a scalar.
//!
//! Closure convention: `Fn((P, X)) -> Y`.

mod actegory;
#[cfg(feature = "ad")]
pub mod ad;
mod comonoid;
#[cfg(feature = "ad")]
mod dual;
mod module_actegory;
mod monoidal_category;
mod morphism;
mod reparameterization;

pub use actegory::{Actegory, SetActegory};
pub use comonoid::{Comonoid, DiagonalComonoid, tie_weights};
pub use module_actegory::{
    DirectSum, F64Actegory, F64Module, F64Monoidal, F64Morphism, F64Object, RActegory, RModule,
    RMonoidal, RMorphism, RObject,
};
pub use monoidal_category::{
    MonoidalCategory, MonoidalTag, SetCategoryDefaults, SetMonoidal, SetMorphism, SetObject,
    private::Sealed,
};
pub use morphism::{Para, ParaMorphism};
pub use reparameterization::Reparameterization;
