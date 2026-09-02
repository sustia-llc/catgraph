//! Frobenius algebra string diagrams: generators, layers, morphisms, and DAG-based interpretation.
//!
//! A Frobenius algebra has six generators (unit, counit, multiplication, comultiplication,
//! braiding, identity) composed into layered morphisms and interpreted via `MorphismSystem`.

mod morphism_system;
mod operations;
#[cfg(test)]
mod to_cospan_pin;
mod trait_impl;

/// The Prop 3.8 semantics map, re-exported from
/// [`cospan_algebra`](crate::cospan_algebra).
///
/// The bounds require `Send + Sync` on both parameters, and an
/// `UnSpecifiedBox` is rejected with
/// [`CatgraphError::Interpret`](crate::errors::CatgraphError::Interpret).
pub use crate::cospan_algebra::frobenius_to_cospan;
pub use morphism_system::{Contains, InterpretableMorphism, MorphismSystem};
pub use operations::{
    FrobeniusMorphism, FrobeniusOperation, from_decomposition, special_frobenius_morphism,
};
pub use trait_impl::Frobenius;
