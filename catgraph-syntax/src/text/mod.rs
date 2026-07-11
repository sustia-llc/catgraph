//! Textual surface for free-prop terms.
//!
//! The surface is a lossless round-trip pair over the concrete syntax
//! `expr := term (';' term)*`, `term := factor (('⊗' | '*') factor)*`,
//! `factor := id(n) | braid(m,n) | GENERATOR | '(' expr ')'`:
//!
//! - the **printer** ([`print::Pretty`] / [`print::print`]) is a structural,
//!   total renderer of [`PropExpr<G>`](catgraph_applied::prop::PropExpr). It
//!   emits ASCII only (`*`); the Unicode tensor `⊗` is an **input** synonym.
//! - the **parser** ([`parse::parse`]) is a hand-rolled recursive-descent
//!   reader accepting both `*` and `⊗`, building exclusively through the
//!   [`Free`](catgraph_applied::prop::Free) smart constructors so every parse
//!   is arity-sound by construction. Nesting depth is bounded
//!   ([`parse::MAX_NESTING_DEPTH`], untrusted input).
//! - [`presentation`] renders and reads presentation files
//!   (one `lhs = rhs` per line, Seven Sketches Def 5.33).
//!
//! The round-trip target is `parse(&print(e)) == Ok(e)` structurally (same
//! tree; the printer never normalises). It is machine-checked by the S2
//! proptests.
//!
//! The bridge between a signature's generators and their concrete tokens is the
//! [`GeneratorSyntax`] trait.

pub mod parse;
pub mod presentation;
pub mod print;

pub use parse::{MAX_NESTING_DEPTH, parse};
pub use presentation::{parse_presentation, print_presentation};
pub use print::{Pretty, print};

use catgraph_applied::prop::PropSignature;

/// A prop signature whose generators have a concrete textual token.
///
/// This extends [`PropSignature`] (Seven Sketches Def 5.25 — the source/target
/// arities of each generator) with the lexical layer the textual surface needs:
/// each generator prints to a token, and a token parses back to a generator.
///
/// # Round-trip contract
///
/// Implementors **must** satisfy, for every generator `g`:
///
/// 1. `Self::parse_token(&g.print_token()) == Some(g)` — printing a generator
///    and parsing the result recovers the original generator; and
/// 2. **`print_token` returns a single lexical atom**: it must contain no
///    `;`, `*`, `⊗`, `=`, parenthesis, or whitespace, and must not equal the
///    reserved grammar keywords **`id`** or **`braid`**. (`=` is reserved as
///    the presentation-file equation separator; the parser's lexer treats it
///    as a delimiter, so a token containing it cannot re-lex as one atom.)
///
/// Both clauses are load-bearing. The printer emits tokens **verbatim, with no
/// validation or escaping** — a token violating clause 2 produces output that
/// re-lexes as multiple tokens (or as the built-in `id(n)` / `braid(m,n)`
/// atoms), so the printed term does **not** reparse to the same tree even
/// though the token-level clause 1 may hold. Clause 1 is machine-checked per
/// signature by a proptest from Phase S2 onward; clause 2 is additionally
/// exercised by S2's whole-expression round-trip proptests (a violating
/// implementation fails them). S1 ships the trait and the printer that
/// consumes [`print_token`](GeneratorSyntax::print_token).
pub trait GeneratorSyntax: PropSignature {
    /// The concrete token for this generator (e.g. `"copy"`, `"add"`).
    fn print_token(&self) -> String;

    /// Parse a token back into a generator, or `None` if it names no generator
    /// of this signature.
    fn parse_token(token: &str) -> Option<Self>
    where
        Self: Sized;
}
