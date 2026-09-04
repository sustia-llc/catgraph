//! Error type for the textual surface.
//!
//! [`SyntaxError`] carries the failure modes the crate's surfaces can produce:
//! [`Parse`](SyntaxError::Parse) from the textual layer, the interpreter
//! variants [`WireCount`](SyntaxError::WireCount) and
//! [`ModelArity`](SyntaxError::ModelArity) from [`crate::eval`], the
//! Frobenius-functor variants [`NonFrobenius`](SyntaxError::NonFrobenius) and
//! [`DimensionOverflow`](SyntaxError::DimensionOverflow) from
//! [`to_mat_kron`](crate::frobenius::to_mat_kron), and a transparent passthrough
//! for the arity failures that originate in
//! [`catgraph-applied`](catgraph_applied) (every
//! [`Free::compose`](catgraph_applied::prop::Free::compose) is a
//! [`CatgraphError`](catgraph::errors::CatgraphError)).

use thiserror::Error;

/// Failures raised by `catgraph-syntax`'s textual surface.
///
/// `#[non_exhaustive]`: new variants may be added without a breaking change,
/// so downstream `match`es must carry a `_ =>` catch-all arm.
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

    /// A wire count did not match a declared arity — always a *term, bundle, or
    /// caller* fault, never a model one (a misbehaving model surfaces as
    /// [`ModelArity`](SyntaxError::ModelArity)). Raised by
    /// [`eval`](crate::eval::eval) (`context` one of `"id"`, `"braid"`,
    /// `"generator"`, `"compose"`), by
    /// [`SfgModel::apply_generator`](crate::eval::SfgModel) called directly
    /// with a wrong-length bundle (`context` = `"SfgModel generator input
    /// arity"`), by [`Wires::unflatten`](crate::traced::Wires::unflatten) fed a
    /// value vector whose length is not the bundle's `COUNT` (`context` =
    /// `"Wires::unflatten bundle length"` or `"Wires::unflatten wire"`), and by
    /// [`traced_generator`](crate::traced::traced_generator) when an arrow's
    /// typed interface disagrees with the generator's declared arity
    /// (`context` = `"traced_generator input bundle vs generator source
    /// arity"` or `"...output bundle vs generator target arity"`).
    #[error("wire-count mismatch at a `{context}` node: expected {expected}, got {actual}")]
    WireCount {
        /// The declared arity the check expected — the source arity when a *source*
        /// interface fails, the target arity when a *target* interface fails, or a
        /// bundle's `COUNT` for the [`crate::traced`] producers (`context`
        /// disambiguates which).
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
    /// generator, which is out of the semantic functor's domain. The
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

    /// A [`to_mat_kron`](crate::frobenius::to_mat_kron) node's dense cell
    /// count overflowed `usize`. A wire of colour `c` maps to `R^dims(c)`, so
    /// an interface word maps to the product of its letters' dimensions and a
    /// node's dense matrix has `product(source word) · product(target word)`
    /// cells; `product` is the running product accumulated before the
    /// overflowing multiplication, and `factor` is the value it was
    /// multiplied by. This guards the cell count, not memory pressure — a
    /// huge-but-representable matrix still allocates.
    #[error("dense cell count overflowed usize: {product} × {factor} exceeds usize::MAX")]
    DimensionOverflow {
        /// The running product accumulated before the overflowing multiplication.
        product: usize,
        /// The value it was multiplied by — a wire dimension when a word's own
        /// product overflows, or the target word's dimension in the final
        /// `rows · cols` cell-count guard.
        factor: usize,
    },

    /// A term interpreter ([`eval`](crate::eval::eval),
    /// [`to_mat_kron`](crate::frobenius::to_mat_kron)) or the user-term
    /// inclusion [`lift_user`](crate::frobenius::lift_user) refused a term whose
    /// structural nesting depth exceeds
    /// [`MAX_TERM_DEPTH`](crate::depth::MAX_TERM_DEPTH), measured pre-flight
    /// by [`term_depth`](crate::depth::term_depth) before the interpreter
    /// recurses. Parsed terms are already capped by the parser's
    /// `MAX_NESTING_DEPTH`; this guards programmatically-built terms.
    #[error("term nesting depth {depth} exceeds MAX_TERM_DEPTH ({limit})")]
    RecursionLimit {
        /// The term's measured structural depth.
        depth: usize,
        /// The configured limit, [`MAX_TERM_DEPTH`](crate::depth::MAX_TERM_DEPTH).
        limit: usize,
    },

    /// An arity check in the underlying free-prop engine failed — for example a
    /// composition whose interfaces do not meet. Surfaced transparently from
    /// [`catgraph::errors::CatgraphError`].
    #[error(transparent)]
    Catgraph(#[from] catgraph::errors::CatgraphError),
}
