//! Golden-output tests for the structural pretty-printer (Phase S1).
//!
//! Every term is built through [`Free`] smart constructors only; `compose` is
//! fallible (arity-checked) so its `Result` is unwrapped in-test. The goldens
//! pin the grammar/precedence rules documented on
//! [`catgraph_syntax::text::print`].

mod common;

use catgraph_syntax::text::{Pretty, print};
use common::{Sig, g, precedence_goldens};

/// The whole precedence / associativity golden set: `print(term) == text` over
/// the shared table (the parser suite asserts the reverse over the same table).
/// Covers atoms, `id(0)`, tensor-binds-tighter, both mixed-precedence operand
/// positions, left-associative flattening, and right-nested parenthesisation.
/// Two distinct trees over the same atoms (`copy ; add * unit` vs
/// `(copy ; add) * unit`) render distinctly, so the reassociation cases in the
/// table already witness structure-preserving output.
#[test]
fn precedence_goldens_print() {
    for (term, text) in precedence_goldens() {
        assert_eq!(print(&term), text, "printing {term:?}");
    }
}

#[test]
fn pretty_adapter_agrees_with_print() {
    // The `Pretty` Display adapter agrees with the free `print` function.
    assert_eq!(Pretty(&g(Sig::Add)).to_string(), "add");
}
