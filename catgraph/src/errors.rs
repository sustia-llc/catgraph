//! Error types for catgraph operations.
//!
//! [`CatgraphError`] is the unified error enum returned by fallible operations
//! across all modules. Variants are grouped by the subsystem that produces them:
//! construction (cospans, spans, named cospans), canonical forms
//! ([`CospanCanon`](crate::cospan_canon::CospanCanon)), composition (cospans,
//! spans, morphisms), interpretation (Frobenius DAGs), operadic substitution,
//! relation algebra, Petri nets, and finite set morphisms.

use std::fmt;

use thiserror::Error;

use crate::finset::{TryFromFinSetError, TryFromInjError, TryFromSurjError};

/// Which boundary leg of a [`Cospan`](crate::cospan::Cospan) or
/// [`Span`](crate::span::Span) a construction failure was found on.
///
/// Rendered as `domain` / `codomain` in error messages, which is the same
/// vocabulary the downstream `catgraph-surreal` store already reports leg
/// corruption with. The two legs are the whole story structurally, so this
/// enum is deliberately **not** `#[non_exhaustive]` — matching it exhaustively
/// is safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoundaryLeg {
    /// The domain leg: `Cospan::left_to_middle`, or the `.0` component of a
    /// [`Span`](crate::span::Span)'s middle pairs.
    Domain,
    /// The codomain leg: `Cospan::right_to_middle`, or the `.1` component of a
    /// [`Span`](crate::span::Span)'s middle pairs.
    Codomain,
}

impl BoundaryLeg {
    /// The leg's name as it appears in error messages.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Domain => "domain",
            Self::Codomain => "codomain",
        }
    }
}

impl fmt::Display for BoundaryLeg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Unified error type for catgraph operations.
///
/// Each variant captures enough context (sizes, indices, labels) for the caller
/// to diagnose the failure without re-inspecting the operands.
///
/// `#[non_exhaustive]`: the enum spans every subsystem in the crate and is
/// expected to keep growing as subsystems learn to reject inputs they used to
/// accept — the construction variants below are exactly that (issue
/// [#256](https://github.com/sustia-llc/catgraph/issues/256)). Downstream
/// `match`es must carry a wildcard arm, so a later variant is not a breaking
/// change. This mirrors `catgraph-syntax`'s `SyntaxError` and `catgraph-dl`'s
/// `DepthError`, both of which were already `#[non_exhaustive]`; core was the
/// last crate whose error enum was not.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum CatgraphError {
    /// A [`Cospan`](crate::cospan::Cospan) or [`Span`](crate::span::Span) leg
    /// entry targets an index outside the set it must land in.
    ///
    /// For a cospan the legs land in the apex (middle) set, so `target_len` is
    /// the apex size; for a span the middle pairs land in the boundary sets, so
    /// `target_len` is the size of the named boundary. Raised by
    /// [`Cospan::new`](crate::cospan::Cospan::new) and
    /// [`Span::new`](crate::span::Span::new) — the `_unchecked` constructors
    /// leave this to a `debug_assert!`.
    #[error(
        "construction error: {leg} leg entry {position} targets index {target}, but the target set has {target_len} element(s)"
    )]
    ConstructionIndexOutOfBounds {
        /// Which leg the offending entry belongs to.
        leg: BoundaryLeg,
        /// The offending entry's position within that leg.
        position: usize,
        /// The out-of-range index the entry targets.
        target: usize,
        /// The size of the set the entry had to land in.
        target_len: usize,
    },

    /// A [`Span`](crate::span::Span) middle pair links a domain element to a
    /// codomain element carrying a different label.
    ///
    /// A span's apex witnesses a relation between *equally labelled* boundary
    /// elements, so `left[pair.0] == right[pair.1]` is a structural invariant,
    /// not a preference. Raised by [`Span::new`](crate::span::Span::new).
    #[error(
        "construction error: middle pair {position} links domain index {left_index} to codomain index {right_index}, but their labels disagree ({left_label} vs {right_label})"
    )]
    ConstructionLabelMismatch {
        /// The offending pair's position in the middle-pair list.
        position: usize,
        /// The domain index the pair names.
        left_index: usize,
        /// The codomain index the pair names.
        right_index: usize,
        /// `Debug` rendering of the label at `left_index`.
        left_label: String,
        /// `Debug` rendering of the label at `right_index`.
        right_label: String,
    },

    /// A [`NamedCospan`](crate::named_cospan::NamedCospan) was handed a port-name
    /// list whose length does not match the boundary it names.
    ///
    /// A named cospan's whole purpose is that every boundary port carries a
    /// stable name, so "one name per port" is a structural invariant of the type
    /// rather than a convenience: with the lists out of step, port `i`'s name is
    /// some other port's name or missing entirely, and every name-keyed operation
    /// on that side silently addresses the wrong port. Raised by
    /// [`NamedCospan::new`](crate::named_cospan::NamedCospan::new) — the
    /// `_unchecked` constructor leaves this to a `debug_assert!`.
    #[error(
        "construction error: the {leg} has {boundary_len} port(s) but {name_count} port name(s) were supplied; there must be exactly one name per port"
    )]
    ConstructionNameCountMismatch {
        /// Which boundary the name list failed to match.
        leg: BoundaryLeg,
        /// The number of ports on that boundary — `left.len()` or `right.len()`.
        boundary_len: usize,
        /// The number of names supplied for it.
        name_count: usize,
    },

    /// A [`CospanCanon`](crate::cospan_canon::CospanCanon)'s class vector is not
    /// sorted under [`ApexClass`](crate::cospan_canon::ApexClass)'s `Ord`.
    ///
    /// That sort is what makes a canonical form invariant under relabelling of
    /// apex vertices, so an unsorted vector is not a canonical form at all — it
    /// would compare unequal to the
    /// [`canonical_form`](crate::cospan::Cospan::canonical_form) of every
    /// cospan, including the one it was derived from.
    ///
    /// Equal neighbours are legal and are not reported: repeated scalars (same
    /// label, both preimages empty) are exactly the duplicates the form must
    /// keep. Raised by
    /// [`CospanCanon::from_parts`](crate::cospan_canon::CospanCanon::from_parts).
    #[error(
        "canonical form error: class at position {position} sorts before its predecessor; the class vector must be sorted under ApexClass's Ord"
    )]
    CanonClassesNotSorted {
        /// Position of the first class that sorts strictly before the class
        /// preceding it. Always `>= 1`.
        position: usize,
    },

    /// One [`ApexClass`](crate::cospan_canon::ApexClass) preimage inside a
    /// [`CospanCanon`](crate::cospan_canon::CospanCanon) is not **strictly**
    /// ascending.
    ///
    /// Strict, not merely non-decreasing: a repeated index inside one preimage
    /// would already mean that leg is not a function, which the partition check
    /// reports as [`CanonPreimageNotAPartition`](Self::CanonPreimageNotAPartition).
    /// Raised by
    /// [`CospanCanon::from_parts`](crate::cospan_canon::CospanCanon::from_parts).
    #[error(
        "canonical form error: the {leg} preimage of class {class_position} is not strictly ascending at position {position}"
    )]
    CanonPreimageNotAscending {
        /// Which of the two preimages the offending entry belongs to.
        leg: BoundaryLeg,
        /// The offending class's position in the class vector.
        class_position: usize,
        /// The offending entry's position within that preimage. Always `>= 1`,
        /// and the entry at `position - 1` is the one it fails to exceed.
        position: usize,
    },

    /// An [`ApexClass`](crate::cospan_canon::ApexClass) preimage inside a
    /// [`CospanCanon`](crate::cospan_canon::CospanCanon) names a boundary index
    /// outside the boundary it indexes.
    ///
    /// Distinct from
    /// [`ConstructionIndexOutOfBounds`](Self::ConstructionIndexOutOfBounds),
    /// which points the *other* way: there a leg entry overshoots the apex, here
    /// an apex class's preimage overshoots a boundary, and locating it needs the
    /// class's position as well as the position within the preimage. Raised by
    /// [`CospanCanon::from_parts`](crate::cospan_canon::CospanCanon::from_parts).
    #[error(
        "canonical form error: the {leg} preimage of class {class_position} names index {index} at position {position}, but the {leg} boundary has {boundary_len} element(s)"
    )]
    CanonPreimageOutOfBounds {
        /// Which of the two preimages the offending entry belongs to.
        leg: BoundaryLeg,
        /// The offending class's position in the class vector.
        class_position: usize,
        /// The offending entry's position within that preimage.
        position: usize,
        /// The out-of-range boundary index the entry names.
        index: usize,
        /// The size of the boundary it had to index — `dom_len` or `cod_len`.
        boundary_len: usize,
    },

    /// The preimages of one leg, taken across all of a
    /// [`CospanCanon`](crate::cospan_canon::CospanCanon)'s classes, do not
    /// partition that boundary: some index occurs zero times, or more than once.
    ///
    /// This is the "each leg is a *function*" property, and it is what makes
    /// non-bubble class signatures pairwise-distinct — without it, bubbles stop
    /// being the only legitimate duplicates and the form no longer decides apex
    /// isomorphism. Raised by
    /// [`CospanCanon::from_parts`](crate::cospan_canon::CospanCanon::from_parts).
    #[error(
        "canonical form error: {leg} index {index} occurs in {occurrences} class preimage(s), expected exactly 1 — the {leg} preimages must partition 0..{boundary_len}"
    )]
    CanonPreimageNotAPartition {
        /// Which leg's preimages fail to partition their boundary.
        leg: BoundaryLeg,
        /// The least boundary index whose occurrence count is not 1.
        index: usize,
        /// How many class preimages contain it: `0` when no class claims it,
        /// `>= 2` when several do.
        occurrences: usize,
        /// The size of the boundary the preimages had to partition —
        /// `dom_len` or `cod_len`.
        boundary_len: usize,
    },

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
