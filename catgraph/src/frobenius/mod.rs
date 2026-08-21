//! Frobenius algebra string diagrams: generators, layers, morphisms, and DAG-based interpretation.
//!
//! A Frobenius algebra has six generators (unit, counit, multiplication, comultiplication,
//! braiding, identity) composed into layered morphisms and interpreted via `MorphismSystem`.

mod morphism_system;
mod operations;
#[cfg(test)]
mod to_cospan_pin;
mod trait_impl;

/// The Prop 3.8 semantics map, re-exported so `frobenius::frobenius_to_cospan`
/// keeps naming it.
///
/// There was briefly a second implementation of this map here, added under #283
/// while [#284] was adding the `cospan_algebra` one on a parallel branch; #336
/// unified them after measuring they agree over 383 terms (see the
/// `to_cospan_pin` test module). ⚠ The surviving function's bounds require
/// `Send + Sync` on both parameters and it rejects an `UnSpecifiedBox` with
/// [`CatgraphError::Interpret`](crate::errors::CatgraphError::Interpret) rather
/// than `Composition` — both changes for callers of this path.
///
/// [#284]: https://github.com/sustia-llc/catgraph/issues/284
pub use crate::cospan_algebra::frobenius_to_cospan;
pub use morphism_system::{Contains, InterpretableMorphism, MorphismSystem};
pub use operations::{
    FrobeniusMorphism, FrobeniusOperation, from_decomposition, special_frobenius_morphism,
};
pub use trait_impl::Frobenius;
