//! Shared test fixtures for `catgraph-syntax`.
//!
//! The workspace convention (mirroring catgraph-dl's `tests/common/mod.rs`) is
//! to keep signature fixtures here and **extend, never fork** them across test
//! files. [`Sig`] is a small monochromatic signature whose four generators have
//! deliberately chosen arities so that both precedence orderings
//! (`a ; b * c` and `(a ; b) * c`) are arity-valid from the *same* atoms —
//! `Unit` has source `0`, which is exactly the condition that makes both parses
//! typecheck.
//!
//! S2 additions: [`BadSig`], a deliberately clause-2-violating signature (its
//! token contains whitespace) used to prove the round-trip suite *catches*
//! violating implementations; and the proptest strategies
//! ([`arb_expr`] plus the per-signature leaf/generator strategies) shared by
//! the parser round-trip tests.
//!
//! `#![allow(dead_code)]`: each integration-test binary `mod common;`-includes
//! this whole module but uses only the parts it needs, so the unused items are
//! expected per compilation unit.
#![allow(dead_code)]

use catgraph_applied::prop::{Free, PropExpr, PropSignature};
use catgraph_applied::sfg::SfgGenerator;
use catgraph_syntax::text::GeneratorSyntax;
use proptest::prelude::*;

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

/// A deliberately clause-2-**violating** signature: its token contains a space,
/// so a printed generator re-lexes as two atoms and does not reparse to the
/// original generator. Used by the negative round-trip test to prove the suite
/// detects violating implementations.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BadSig;

impl PropSignature for BadSig {
    fn source(&self) -> usize {
        1
    }

    fn target(&self) -> usize {
        1
    }
}

impl GeneratorSyntax for BadSig {
    fn print_token(&self) -> String {
        // Clause-2 violation: the whitespace splits into two atoms on re-lex.
        "bad token".to_string()
    }

    fn parse_token(token: &str) -> Option<Self> {
        (token == "bad token").then_some(BadSig)
    }
}

/// Recursive strategy for arity-valid [`PropExpr<G>`] built through
/// [`Free`]: children are tensored (always valid) or composed **only** when
/// `left.target() == right.source()`, so every generated term typechecks.
pub fn arb_expr<G, S>(leaf: S) -> impl Strategy<Value = PropExpr<G>>
where
    G: PropSignature + 'static,
    S: Strategy<Value = PropExpr<G>> + 'static,
{
    leaf.prop_recursive(6, 64, 2, |inner| {
        (inner.clone(), inner).prop_map(|(f, g)| {
            if f.target() == g.source() {
                Free::compose(f, g).expect("target == source is checked in the guard")
            } else {
                Free::tensor(f, g)
            }
        })
    })
}

/// Leaf strategy over [`Sig`]: generators, identities `id(0..=3)`, braids
/// `braid(0..=2, 0..=2)`.
pub fn arb_sig_leaf() -> impl Strategy<Value = PropExpr<Sig>> {
    prop_oneof![
        Just(Free::generator(Sig::Copy)),
        Just(Free::generator(Sig::Add)),
        Just(Free::generator(Sig::Unit)),
        Just(Free::generator(Sig::Counit)),
        (0usize..=3).prop_map(Free::<Sig>::identity),
        (0usize..=2, 0usize..=2).prop_map(|(m, n)| Free::<Sig>::braid(m, n)),
    ]
}

/// Strategy over bare [`Sig`] generators (clause-1 round-trip).
pub fn arb_sig_gen() -> impl Strategy<Value = Sig> {
    prop_oneof![
        Just(Sig::Copy),
        Just(Sig::Add),
        Just(Sig::Unit),
        Just(Sig::Counit),
    ]
}

/// Leaf strategy over `SfgGenerator<i64>`: the five SFG generators (with a
/// random `i64` scalar), identities, and braids.
pub fn arb_sfg_leaf() -> impl Strategy<Value = PropExpr<SfgGenerator<i64>>> {
    prop_oneof![
        Just(Free::generator(SfgGenerator::Copy)),
        Just(Free::generator(SfgGenerator::Discard)),
        Just(Free::generator(SfgGenerator::Add)),
        Just(Free::generator(SfgGenerator::Zero)),
        any::<i64>().prop_map(|r| Free::generator(SfgGenerator::Scalar(r))),
        (0usize..=3).prop_map(Free::<SfgGenerator<i64>>::identity),
        (0usize..=2, 0usize..=2).prop_map(|(m, n)| Free::<SfgGenerator<i64>>::braid(m, n)),
    ]
}

/// Strategy over bare `SfgGenerator<i64>` generators (clause-1 round-trip);
/// `Scalar` ranges over all `i64`, exercising the `scalar:<r>` token fidelity.
pub fn arb_sfg_gen() -> impl Strategy<Value = SfgGenerator<i64>> {
    prop_oneof![
        Just(SfgGenerator::Copy),
        Just(SfgGenerator::Discard),
        Just(SfgGenerator::Add),
        Just(SfgGenerator::Zero),
        any::<i64>().prop_map(SfgGenerator::Scalar),
    ]
}
