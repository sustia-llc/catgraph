//! Error type for the textual surface.
//!
//! [`SyntaxError`] carries the failure modes the crate's surfaces can produce:
//! [`Parse`](SyntaxError::Parse) from the textual layer (S1/S2), the two
//! interpreter variants [`WireCount`](SyntaxError::WireCount) and
//! [`ModelArity`](SyntaxError::ModelArity) from the S3 evaluator
//! ([`crate::eval`]), the two S4 Frobenius-functor variants
//! [`NonFrobenius`](SyntaxError::NonFrobenius) and
//! [`DimensionOverflow`](SyntaxError::DimensionOverflow) from
//! [`to_mat_kron`](crate::frobenius::to_mat_kron), and a transparent passthrough
//! for the arity failures that originate in
//! [`catgraph-applied`](catgraph_applied) (every
//! [`Free::compose`](catgraph_applied::prop::Free::compose) is a
//! [`CatgraphError`](catgraph::errors::CatgraphError)).

use thiserror::Error;

/// Failures raised by `catgraph-syntax`'s textual surface.
///
/// `#[non_exhaustive]`: later phases add variants (S4's Frobenius layer landed
/// [`NonFrobenius`](SyntaxError::NonFrobenius) /
/// [`DimensionOverflow`](SyntaxError::DimensionOverflow); S5's typed builder is
/// still to come), so downstream `match`es must carry a wildcard arm — a new
/// variant is not a breaking change. Match with a `_ =>` catch-all.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum SyntaxError {
    /// The parser rejected the input at byte `offset` with `message`.
    ///
    /// Produced by the expression parser
    /// ([`text::parse::parse`](crate::text::parse::parse)) and the
    /// presentation reader
    /// ([`text::presentation::parse_presentation`](crate::text::presentation::parse_presentation));
    /// the printer never produces it. `offset` is always relative to the
    /// **full input handed to the entry point** — for a presentation file that
    /// is the whole multi-line text, not the failing line or side.
    #[error("parse error at offset {offset}: {message}")]
    Parse {
        /// Byte offset into the source text where the failure was detected.
        offset: usize,
        /// Human-readable description of what was expected.
        message: String,
    },

    /// The number of wires flowing into a sub-morphism did not match its
    /// declared source arity — always a *term or caller* fault, never a model
    /// one (a misbehaving model surfaces as [`ModelArity`](SyntaxError::ModelArity)).
    ///
    /// Two producers, both in the interpreter ([`crate::eval`]):
    ///
    /// - [`eval`](crate::eval::eval) itself — the top-level input length differs
    ///   from `expr.source()`, or (for a directly-constructed, ill-formed
    ///   [`PropExpr`](catgraph_applied::prop::PropExpr)) an interior node draws
    ///   the wrong wire count off the cursor. Here `context` names the node kind
    ///   the mismatch was detected at (`"id"`, `"braid"`, `"generator"`,
    ///   `"compose"`).
    /// - [`SfgModel::apply_generator`](crate::eval::SfgModel) called *directly*
    ///   with a wrong-length bundle — `context` is `"SfgModel generator input
    ///   arity"`. (Routed through [`eval`](crate::eval::eval) this cannot happen,
    ///   since `eval` hands each generator exactly its source arity.)
    #[error("wire-count mismatch at a `{context}` node: expected {expected}, got {actual}")]
    WireCount {
        /// The declared source arity the node expected.
        expected: usize,
        /// The actual number of wires supplied.
        actual: usize,
        /// A short description of where the mismatch was detected.
        context: &'static str,
    },

    /// A model's [`apply_generator`](crate::eval::ArrowModel::apply_generator)
    /// returned a number of output wires that disagrees with the generator's
    /// declared target arity — a broken [`ArrowModel`](crate::eval::ArrowModel)
    /// implementation, caught by [`eval`](crate::eval::eval) before the wrong
    /// bundle propagates.
    #[error("model returned {actual} outputs for generator `{generator}`, expected {expected}")]
    ModelArity {
        /// The offending generator, rendered via its `Debug` impl.
        generator: String,
        /// The generator's declared target arity.
        expected: usize,
        /// The number of outputs the model actually returned.
        actual: usize,
    },

    /// A [`to_mat_kron`](crate::frobenius::to_mat_kron) mapping hit a `User`
    /// generator, which is **out of the semantic functor's domain** (S4). The
    /// Prop 3.8 functor `Cospan → MatKron(R)` is defined only on the Frobenius
    /// generators μ/η/δ/ε and the SMC structure; a signature generator carries
    /// no MatKron image, so the mapping stops here rather than guess one.
    /// `generator` is the offending `User(g)`'s inner `g` rendered via `Debug`.
    #[error(
        "non-Frobenius generator `{generator}` has no MatKron image (out of the Prop 3.8 functor's domain)"
    )]
    NonFrobenius {
        /// The offending user generator `g`, rendered via its `Debug` impl.
        generator: String,
    },

    /// A [`to_mat_kron`](crate::frobenius::to_mat_kron) wire interface of `k`
    /// wires maps to the object dimension `dim^k`, and that power overflowed
    /// `usize`. Reported instead of panicking (or attempting to allocate an
    /// astronomically large matrix). Raised for the `dim.checked_pow(k)` of an
    /// `Identity(k)` / `Braid(m, n)` node whose wire count is too large for the
    /// chosen `dim`.
    #[error("dimension dim^k overflowed usize: dim = {dim}, k = {exponent}")]
    DimensionOverflow {
        /// The per-wire object dimension `dim`.
        dim: usize,
        /// The wire-count exponent `k` (so the object is `dim^k`).
        exponent: usize,
    },

    /// An arity check in the underlying free-prop engine failed — for example a
    /// composition whose interfaces do not meet. Surfaced transparently from
    /// [`catgraph::errors::CatgraphError`].
    #[error(transparent)]
    Catgraph(#[from] catgraph::errors::CatgraphError),
}
