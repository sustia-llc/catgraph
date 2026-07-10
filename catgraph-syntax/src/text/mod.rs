//! Textual surface for free-prop terms.
//!
//! Phase S1 ships the **printer**: a structural, total renderer of
//! [`PropExpr<G>`](catgraph_applied::prop::PropExpr) into the concrete syntax
//! `expr := term (';' term)*`, `term := factor (('⊗' | '*') factor)*`,
//! `factor := id(n) | braid(m,n) | GENERATOR | '(' expr ')'`. The printer
//! emits ASCII only (`*`); the Unicode tensor `⊗` is an **input** synonym the
//! Phase-S2 parser accepts per the approved design — an S2 implementation
//! that lexed only `*` would silently narrow the design's input alphabet.
//! The matching recursive-descent parser (and the round-trip law tests) land
//! in Phase S2.
//!
//! The bridge between a signature's generators and their concrete tokens is the
//! [`GeneratorSyntax`] trait; the printer itself is [`print::Pretty`] /
//! [`print::print`].

pub mod print;

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
///    `;`, `*`, `⊗`, parenthesis, or whitespace, and must not equal the
///    reserved grammar keywords **`id`** or **`braid`**.
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
