//! Error types for catgraph operations.
//!
//! [`CatgraphError`] is the unified error enum returned by fallible operations
//! across all modules. Variants are grouped by the subsystem that produces them:
//! composition (cospans, spans, morphisms), interpretation (Frobenius DAGs),
//! operadic substitution, relation algebra, Petri nets, and finite set morphisms.

use thiserror::Error;

use crate::finset::{TryFromFinSetError, TryFromInjError, TryFromSurjError};

/// Unified error type for catgraph operations.
///
/// Each variant captures enough context (sizes, indices, labels) for the caller
/// to diagnose the failure without re-inspecting the operands.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CatgraphError {
    /// Domain/codomain sizes do not match at the composition interface.
    #[error("composition error: interface size mismatch (expected {expected}, got {actual})")]
    CompositionSizeMismatch { expected: usize, actual: usize },

    /// Lambda types disagree at a specific boundary index during composition.
    #[error(
        "composition error: label mismatch at index {index} (expected {expected:?}, got {actual:?})"
    )]
    CompositionLabelMismatch {
        index: usize,
        expected: String,
        actual: String,
    },

    /// General composition failure (e.g. non-composable morphisms).
    #[error("composition error: {message}")]
    Composition { message: String },

    /// [`MorphismSystem::fill_black_boxes`](crate::frobenius::MorphismSystem::fill_black_boxes)
    /// could not resolve a named morphism (cycle, missing definition, etc.).
    #[error("interpret error: {context}")]
    Interpret { context: String },

    /// Operadic substitution failed (boundary mismatch, missing inner circle, etc.).
    #[error("operadic error: {message}")]
    Operadic { message: String },

    /// Relation algebra operation failed (incompatible domains, invalid construction, etc.).
    #[error("relation error: {message}")]
    Relation { message: String },

    /// Corelation operation failed (not jointly surjective, incompatible domains, etc.).
    #[error("corelation error: {message}")]
    Corel { message: String },

    /// Prop presentation / term-rewriting failed.
    #[error("presentation error: {message}")]
    Presentation { message: String },

    /// Signal flow graph → matrix functor (`S: SFG_R → Mat(R)`) failed.
    #[error("sfg functor error: {message}")]
    SfgFunctor { message: String },

    /// Runtime rig-axiom violation (debug-mode check).
    #[error("rig axiom violation: {axiom} witness {witness}")]
    RigAxiomViolation {
        axiom: &'static str,
        witness: String,
    },

    /// Petri net operation failed (out-of-bounds transition, not enabled, etc.).
    #[error("petri net error: {message}")]
    PetriNet { message: String },

    /// Finite set morphism construction or conversion failed.
    #[error("finite set error: {message}")]
    FinSet { message: String },

    /// A term interpreter refused a term whose structural nesting depth exceeds
    /// its recursion limit — a guard against stack overflow on unbounded,
    /// programmatically-built terms. Shared with `catgraph-syntax`'s
    /// `SyntaxError::RecursionLimit` so interpreters whose error type is fixed to
    /// `CatgraphError` (e.g. a `CompleteFunctor`) report the same shape.
    #[error("recursion limit: term nesting depth {depth} exceeds limit ({limit})")]
    RecursionLimit { depth: usize, limit: usize },
}

impl From<TryFromSurjError> for CatgraphError {
    fn from(e: TryFromSurjError) -> Self {
        Self::FinSet {
            message: e.to_string(),
        }
    }
}

impl From<TryFromInjError> for CatgraphError {
    fn from(e: TryFromInjError) -> Self {
        Self::FinSet {
            message: e.to_string(),
        }
    }
}

impl From<TryFromFinSetError> for CatgraphError {
    fn from(e: TryFromFinSetError) -> Self {
        Self::FinSet {
            message: e.to_string(),
        }
    }
}
