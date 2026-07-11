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

use catgraph_applied::mat::MatR;
use catgraph_applied::prop::{Free, PropExpr, PropSignature};
use catgraph_applied::sfg::SfgGenerator;
use catgraph_syntax::frobenius::FrobeniusOr;
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

/// A signature whose one generator's token is `"mu"` — deliberately colliding
/// with a reserved Frobenius name (arity `2 → 1`, matching `Mu`, so an equation
/// using it would even typecheck). Used by the S4 negative test to prove
/// `FrobeniusOr`'s Frobenius-first `parse_token` **shadows** such a user token
/// (so its clause-1 round-trip breaks), the analogue of S2's [`BadSig`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ShadowSig;

impl PropSignature for ShadowSig {
    fn source(&self) -> usize {
        2
    }

    fn target(&self) -> usize {
        1
    }
}

impl GeneratorSyntax for ShadowSig {
    fn print_token(&self) -> String {
        // Collides with the reserved Frobenius token `mu`.
        "mu".to_string()
    }

    fn parse_token(token: &str) -> Option<Self> {
        (token == "mu").then_some(ShadowSig)
    }
}

/// Wrap any signature's generator in a `PropExpr` leaf — the shared fixture
/// shim every integration suite uses (extend here, never fork per test binary).
pub fn g<G: PropSignature>(s: G) -> PropExpr<G> {
    Free::generator(s)
}

/// The precedence / associativity golden table: `(term, concrete syntax)` pairs
/// where `print(term) == text` **and** `parse(text) == Ok(term)`. It is the one
/// source of truth for both directions — [`printer_golden`](../printer_golden.rs)
/// asserts the printer over it and [`parser`](../parser.rs) asserts the parser
/// over it, so the two surfaces cannot drift apart on the precedence rules.
///
/// Every `compose` is arity-valid by the deliberate [`Sig`] arities (see the
/// module docs); `.expect` documents that invariant at the fixture.
pub fn precedence_goldens() -> Vec<(PropExpr<Sig>, &'static str)> {
    let compose = |f: PropExpr<Sig>, h: PropExpr<Sig>| {
        Free::compose(f, h).expect("golden compositions are arity-valid by construction")
    };
    vec![
        // Atoms.
        (Free::<Sig>::identity(2), "id(2)"),
        (Free::<Sig>::identity(0), "id(0)"),
        (Free::<Sig>::braid(1, 2), "braid(1,2)"),
        (g(Sig::Copy), "copy"),
        // Tensor binds tighter than compose: no parens needed.
        (
            compose(g(Sig::Copy), Free::tensor(g(Sig::Add), g(Sig::Unit))),
            "copy ; add * unit",
        ),
        // A looser-binding compose as the LEFT tensor operand must parenthesize.
        (
            Free::tensor(compose(g(Sig::Copy), g(Sig::Add)), g(Sig::Unit)),
            "(copy ; add) * unit",
        ),
        // ... and as the RIGHT tensor operand.
        (
            Free::tensor(g(Sig::Unit), compose(g(Sig::Copy), g(Sig::Add))),
            "unit * (copy ; add)",
        ),
        // Left-associative chains print flat.
        (
            compose(compose(g(Sig::Copy), g(Sig::Add)), g(Sig::Copy)),
            "copy ; add ; copy",
        ),
        (
            Free::tensor(Free::tensor(g(Sig::Copy), g(Sig::Copy)), g(Sig::Copy)),
            "copy * copy * copy",
        ),
        // Right-nested same-operator subterms must parenthesize.
        (
            compose(g(Sig::Copy), compose(g(Sig::Add), g(Sig::Copy))),
            "copy ; (add ; copy)",
        ),
        (
            Free::tensor(g(Sig::Copy), Free::tensor(g(Sig::Copy), g(Sig::Copy))),
            "copy * (copy * copy)",
        ),
    ]
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

/// Wrap a bare-generator strategy into a leaf strategy with the shared
/// identity (`id(0..=3)`) and braid (`braid(0..=2, 0..=2)`) arms — one
/// definition of the leaf shape, so adding a generator to a signature's
/// `arb_*_gen` list automatically extends its round-trip coverage.
pub fn arb_leaf_from<G>(
    generator: impl Strategy<Value = G> + 'static,
) -> impl Strategy<Value = PropExpr<G>>
where
    G: PropSignature + 'static,
{
    prop_oneof![
        generator.prop_map(Free::generator),
        (0usize..=3).prop_map(Free::<G>::identity),
        (0usize..=2, 0usize..=2).prop_map(|(m, n)| Free::<G>::braid(m, n)),
    ]
}

/// Leaf strategy over [`Sig`]: generators, identities, braids.
pub fn arb_sig_leaf() -> impl Strategy<Value = PropExpr<Sig>> {
    arb_leaf_from(arb_sig_gen())
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
    arb_leaf_from(arb_sfg_gen())
}

/// Strategy over bare `SfgGenerator<i64>` generators, parameterised by the
/// `Scalar`-payload strategy — the one definition of the generator shape. The
/// four nullary generators are fixed; only the scalar range varies between the
/// full-range and bounded wrappers below.
pub fn arb_sfg_gen_with(
    scalar: impl Strategy<Value = i64> + 'static,
) -> impl Strategy<Value = SfgGenerator<i64>> {
    prop_oneof![
        Just(SfgGenerator::Copy),
        Just(SfgGenerator::Discard),
        Just(SfgGenerator::Add),
        Just(SfgGenerator::Zero),
        scalar.prop_map(SfgGenerator::Scalar),
    ]
}

/// Strategy over bare `SfgGenerator<i64>` generators (clause-1 round-trip);
/// `Scalar` ranges over all `i64`, exercising the `scalar:<r>` token fidelity.
pub fn arb_sfg_gen() -> impl Strategy<Value = SfgGenerator<i64>> {
    arb_sfg_gen_with(any::<i64>())
}

/// Bounded-scalar leaf strategy over `SfgGenerator<i64>` for the S3 arithmetic
/// law tests. Identical to [`arb_sfg_leaf`] except `Scalar` ranges over `-3..=3`.
///
/// The interpreter and the Thm 5.53 matrix functor perform plain `i64` `+`/`*`
/// (profile-dependent: debug panics on overflow, release wraps) — neither is
/// "checked" arithmetic. Scalars multiply along a composition chain, so an
/// unbounded range would eventually overflow; the `-3..=3` bound makes overflow
/// *astronomically improbable* across the `prop_recursive(6, 64, 2)` term shapes
/// these tests generate (bounded height, so bounded products), though not
/// mathematically impossible. The round-trip suites keep the full-range
/// [`arb_sfg_leaf`]; only the arithmetic evaluations need the bound.
pub fn arb_sfg_leaf_bounded() -> impl Strategy<Value = PropExpr<SfgGenerator<i64>>> {
    arb_leaf_from(arb_sfg_gen_bounded())
}

/// Bounded-scalar variant of [`arb_sfg_gen`]: `Scalar` in `-3..=3` (see
/// [`arb_sfg_leaf_bounded`] for the overflow rationale).
pub fn arb_sfg_gen_bounded() -> impl Strategy<Value = SfgGenerator<i64>> {
    arb_sfg_gen_with(-3i64..=3)
}

/// Strategy over bare [`FrobeniusOr<Sig>`] generators (S4 clause-1 round-trip):
/// the four Frobenius generators plus a `User(g)` wrapping any [`Sig`] generator.
///
/// [`Sig`]'s tokens (`copy`/`add`/`unit`/`counit`) do not collide with the
/// reserved Frobenius names (`mu`/`eta`/`delta`/`epsilon`), so every value here
/// round-trips — the reserved-token shadowing documented on the `FrobeniusOr`
/// `GeneratorSyntax` impl does not bite this signature.
pub fn arb_frob_gen() -> impl Strategy<Value = FrobeniusOr<Sig>> {
    prop_oneof![
        Just(FrobeniusOr::Mu),
        Just(FrobeniusOr::Eta),
        Just(FrobeniusOr::Delta),
        Just(FrobeniusOr::Epsilon),
        arb_sig_gen().prop_map(FrobeniusOr::User),
    ]
}

/// Leaf strategy over [`FrobeniusOr<Sig>`]: generators, identities, braids —
/// reusing the shared [`arb_leaf_from`] shape so S4 round-trip coverage tracks
/// the same leaf definition as every other signature.
pub fn arb_frob_leaf() -> impl Strategy<Value = PropExpr<FrobeniusOr<Sig>>> {
    arb_leaf_from(arb_frob_gen())
}

/// A length-`len` standard basis **row** vector over `i64`: `1` at index `i`,
/// `0` elsewhere — taken as **row `i` of the `len × len` identity** so the basis
/// convention has a single definition ([`MatR::identity`]). Feeding it through
/// the S3 interpreter under `SfgModel` selects **row `i`** of the Thm 5.53
/// matrix (Def 5.50 / Remark 5.49 row-vector convention).
pub fn basis_i64(len: usize, i: usize) -> Vec<i64> {
    MatR::<i64>::identity(len).entries()[i].clone()
}
