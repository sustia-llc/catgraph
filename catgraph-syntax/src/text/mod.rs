//! Textual surface for free-prop terms.
//!
//! Phase S1 ships the **printer**: a structural, total renderer of
//! [`PropExpr<G>`](catgraph_applied::prop::PropExpr) into the concrete syntax
//! `expr := term (';' term)*`, `term := factor (('*') factor)*`,
//! `factor := id(n) | braid(m,n) | GENERATOR | '(' expr ')'`. The matching
//! recursive-descent parser (and the round-trip law tests) land in Phase S2.
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
/// Implementors must satisfy, for every generator `g`:
///
/// ```text
/// Self::parse_token(&g.print_token()) == Some(g)
/// ```
///
/// i.e. printing a generator and parsing the result recovers the original
/// generator. The contract is machine-checked per signature by a proptest from
/// Phase S2 onward; S1 ships the trait and the printer that consumes
/// [`print_token`](GeneratorSyntax::print_token).
///
/// Tokens should be single lexical atoms (no embedded `;`, `*`, parentheses, or
/// whitespace) so they compose cleanly with the surrounding grammar.
pub trait GeneratorSyntax: PropSignature {
    /// The concrete token for this generator (e.g. `"copy"`, `"add"`).
    fn print_token(&self) -> String;

    /// Parse a token back into a generator, or `None` if it names no generator
    /// of this signature.
    fn parse_token(token: &str) -> Option<Self>
    where
        Self: Sized;
}
