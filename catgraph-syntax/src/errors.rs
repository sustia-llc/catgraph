//! Error type for the textual surface.
//!
//! [`SyntaxError`] is intentionally minimal: it carries only the variants this
//! phase's surface can produce, plus a transparent passthrough for the arity
//! failures that originate in [`catgraph-applied`](catgraph_applied) (every
//! [`Free::compose`](catgraph_applied::prop::Free::compose) is a
//! [`CatgraphError`](catgraph::errors::CatgraphError)). Semantic variants
//! (`WireCount`, `ModelArity`) arrive with the interpreter phase (S3) — no
//! speculative variants ahead of a constructor.

use thiserror::Error;

/// Failures raised by `catgraph-syntax`'s textual surface.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
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

    /// An arity check in the underlying free-prop engine failed — for example a
    /// composition whose interfaces do not meet. Surfaced transparently from
    /// [`catgraph::errors::CatgraphError`].
    #[error(transparent)]
    Catgraph(#[from] catgraph::errors::CatgraphError),
}
