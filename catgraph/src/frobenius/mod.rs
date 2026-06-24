//! Frobenius algebra string diagrams: generators, layers, morphisms, and DAG-based interpretation.
//!
//! A Frobenius algebra has six generators (unit, counit, multiplication, comultiplication,
//! braiding, identity) composed into layered morphisms and interpreted via `MorphismSystem`.

mod morphism_system;
mod operations;
mod trait_impl;

pub use morphism_system::{Contains, InterpretableMorphism, MorphismSystem};
pub use operations::{
    FrobeniusMorphism, FrobeniusOperation, from_decomposition, special_frobenius_morphism,
};
pub use trait_impl::Frobenius;
