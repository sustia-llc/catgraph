//! Shared test fixtures for `catgraph-syntax`.
//!
//! The workspace convention (mirroring catgraph-dl's `tests/common/mod.rs`) is
//! to keep signature fixtures here and **extend, never fork** them across test
//! files. [`Sig`] is a small monochromatic signature whose four generators have
//! deliberately chosen arities so that both precedence orderings
//! (`a ; b * c` and `(a ; b) * c`) are arity-valid from the *same* atoms —
//! `Unit` has source `0`, which is exactly the condition that makes both parses
//! typecheck.

use catgraph_applied::prop::PropSignature;
use catgraph_syntax::text::GeneratorSyntax;

/// A four-generator monochromatic test signature.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Sig {
    /// `copy : 1 → 2`.
    Copy,
    /// `add : 2 → 1`.
    Add,
    /// `unit : 0 → 1`.
    Unit,
    /// `counit : 1 → 0`.
    Counit,
}

impl PropSignature for Sig {
    fn source(&self) -> usize {
        match self {
            Sig::Copy => 1,
            Sig::Add => 2,
            Sig::Unit => 0,
            Sig::Counit => 1,
        }
    }

    fn target(&self) -> usize {
        match self {
            Sig::Copy => 2,
            Sig::Add => 1,
            Sig::Unit => 1,
            Sig::Counit => 0,
        }
    }
}

impl GeneratorSyntax for Sig {
    fn print_token(&self) -> String {
        match self {
            Sig::Copy => "copy",
            Sig::Add => "add",
            Sig::Unit => "unit",
            Sig::Counit => "counit",
        }
        .to_string()
    }

    fn parse_token(token: &str) -> Option<Self> {
        match token {
            "copy" => Some(Sig::Copy),
            "add" => Some(Sig::Add),
            "unit" => Some(Sig::Unit),
            "counit" => Some(Sig::Counit),
            _ => None,
        }
    }
}
