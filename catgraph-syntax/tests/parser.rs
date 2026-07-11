//! Parser tests for the textual free-prop surface (Phase S2).
//!
//! Covers the round-trip law `parse(&print(e)) == Ok(e)` (Sig and
//! `SfgGenerator<i64>`), the [`GeneratorSyntax`] clause-1 token round-trip, the
//! clause-2 negative check, Unicode `⊗` acceptance, precedence/associativity
//! goldens (the [`printer_golden`](../printer_golden.rs) cases read in
//! reverse), the nesting-depth bound, and error-offset diagnostics.

mod common;

use catgraph_applied::prop::Free;
use catgraph_applied::sfg::SfgGenerator;
use catgraph_syntax::errors::SyntaxError;
use catgraph_syntax::text::{GeneratorSyntax, MAX_NESTING_DEPTH, parse, print};
use common::{
    Sig, arb_expr, arb_sfg_gen, arb_sfg_leaf, arb_sig_gen, arb_sig_leaf, g, precedence_goldens,
};
use proptest::prelude::*;

// ---- Round-trip proptests ----------------------------------------------------

proptest! {
    #[test]
    fn sig_expression_roundtrips(e in arb_expr(arb_sig_leaf())) {
        let printed = print(&e);
        prop_assert_eq!(parse::<Sig>(&printed), Ok(e));
    }

    #[test]
    fn sfg_expression_roundtrips(e in arb_expr(arb_sfg_leaf())) {
        let printed = print(&e);
        prop_assert_eq!(parse::<SfgGenerator<i64>>(&printed), Ok(e));
    }

    // Clause 1: printing then parsing a token recovers the generator.
    #[test]
    fn sig_token_clause1(generator in arb_sig_gen()) {
        prop_assert_eq!(Sig::parse_token(&generator.print_token()), Some(generator));
    }

    #[test]
    fn sfg_token_clause1(generator in arb_sfg_gen()) {
        prop_assert_eq!(
            SfgGenerator::<i64>::parse_token(&generator.print_token()),
            Some(generator),
        );
    }
}

/// Clause-2 enforcement: a signature whose token violates clause 2 (contains
/// whitespace) does NOT round-trip — the printed form fails to reparse to the
/// original generator. This proves the round-trip suite catches violators.
#[test]
fn clause2_violation_breaks_roundtrip() {
    use common::BadSig;
    let e = Free::generator(BadSig);
    let printed = print(&e); // "bad token"
    assert_eq!(printed, "bad token");
    // Re-lexes as two atoms; the first ("bad") is not a known generator token.
    assert!(parse::<BadSig>(&printed).is_err());
}

// ---- Unicode tensor ----------------------------------------------------------

#[test]
fn unicode_tensor_is_a_synonym_for_star() {
    // `⊗` (U+2297) parses identically to `*` everywhere.
    assert_eq!(parse::<Sig>("copy ⊗ counit"), parse::<Sig>("copy * counit"));
    // Mixed operators in one input.
    assert_eq!(
        parse::<Sig>("copy ⊗ counit * unit"),
        parse::<Sig>("copy * counit * unit"),
    );
    // With no surrounding whitespace.
    assert_eq!(parse::<Sig>("copy⊗counit"), parse::<Sig>("copy*counit"));
}

// ---- Precedence / associativity goldens (printer_golden.rs, reversed) --------

/// The whole precedence / associativity golden set, read in reverse:
/// `parse(text) == Ok(term)` over the shared table (the printer suite asserts
/// `print(term) == text` over the same table). Covers tensor-binds-tighter,
/// both mixed-precedence operand positions, left-associative flattening, and
/// right-nested parenthesisation.
#[test]
fn precedence_goldens_parse() {
    for (term, text) in precedence_goldens() {
        assert_eq!(parse::<Sig>(text), Ok(term), "parsing {text:?}");
    }
}

#[test]
fn atoms_parse() {
    assert_eq!(parse::<Sig>("id(2)"), Ok(Free::<Sig>::identity(2)));
    assert_eq!(parse::<Sig>("id(0)"), Ok(Free::<Sig>::identity(0)));
    assert_eq!(parse::<Sig>("braid(1,2)"), Ok(Free::<Sig>::braid(1, 2)));
    assert_eq!(parse::<Sig>("copy"), Ok(g(Sig::Copy)));
    // Redundant parentheses collapse.
    assert_eq!(parse::<Sig>("((copy))"), Ok(g(Sig::Copy)));
    // Internal whitespace in keyword arguments is tolerated.
    assert_eq!(parse::<Sig>("id( 2 )"), Ok(Free::<Sig>::identity(2)));
    assert_eq!(parse::<Sig>("braid(1, 2)"), Ok(Free::<Sig>::braid(1, 2)));
}

#[test]
fn adjacent_argument_atoms_do_not_fuse() {
    // `id(1 2)` must error, not silently parse as `id(12)`.
    assert!(matches!(
        parse::<Sig>("id(1 2)"),
        Err(SyntaxError::Parse { .. })
    ));
    // Likewise a whitespace-split first braid arity must not fuse to `12`.
    assert!(matches!(
        parse::<Sig>("braid(1 2,3)"),
        Err(SyntaxError::Parse { .. })
    ));
    // A braid missing its comma is an error, never a fused single arity.
    assert!(matches!(
        parse::<Sig>("braid(1 2)"),
        Err(SyntaxError::Parse { .. })
    ));
}

#[test]
fn braid_second_argument_error_offset_points_at_it() {
    // The bad argument is `x` at byte 8; the offset must name IT, not the
    // start of the argument list.
    match parse::<Sig>("braid(1,x)") {
        Err(SyntaxError::Parse { offset, message }) => {
            assert_eq!(offset, 8, "offset should point at `x`");
            assert!(message.contains('x'), "got: {message}");
        }
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[test]
fn scalar_token_parses() {
    let s = SfgGenerator::Scalar(-7i64);
    assert_eq!(
        parse::<SfgGenerator<i64>>("scalar:-7"),
        Ok(Free::generator(s))
    );
    // A scalar token in composed position: scalar (1 → 1) ; copy (1 → 2) is
    // arity-valid, so this pins a SUCCESSFUL composed parse (a `copy ; scalar`
    // ordering would be arity-invalid and assert nothing about the parser).
    let expected = Free::compose(
        Free::generator(SfgGenerator::Scalar(3i64)),
        Free::generator(SfgGenerator::Copy),
    )
    .unwrap();
    assert_eq!(parse::<SfgGenerator<i64>>("scalar:3 ; copy"), Ok(expected));
}

// ---- Nesting-depth bound -----------------------------------------------------

#[test]
fn moderate_nesting_parses() {
    let depth = 100;
    let input = format!("{}copy{}", "(".repeat(depth), ")".repeat(depth));
    assert_eq!(parse::<Sig>(&input), Ok(g(Sig::Copy)));
}

#[test]
fn over_deep_nesting_errors_without_overflow() {
    let depth = MAX_NESTING_DEPTH + 50;
    let input = format!("{}copy{}", "(".repeat(depth), ")".repeat(depth));
    match parse::<Sig>(&input) {
        Err(SyntaxError::Parse { message, .. }) => {
            assert!(message.contains("MAX_NESTING_DEPTH"), "got: {message}");
        }
        other => panic!("expected a depth-bound Parse error, got {other:?}"),
    }
}

// ---- Error offsets -----------------------------------------------------------

#[test]
fn unknown_generator_offset_points_at_token() {
    match parse::<Sig>("copy ; nope") {
        Err(SyntaxError::Parse { offset, message }) => {
            assert_eq!(offset, 7, "offset should point at `nope`");
            assert!(message.contains("nope"), "got: {message}");
        }
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[test]
fn trailing_input_offset() {
    match parse::<Sig>("copy )") {
        Err(SyntaxError::Parse { offset, .. }) => assert_eq!(offset, 5),
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[test]
fn dangling_operator_reports_end_of_input() {
    match parse::<Sig>("copy ;") {
        Err(SyntaxError::Parse { offset, .. }) => assert_eq!(offset, 6),
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[test]
fn bare_keyword_without_parens_errors() {
    assert!(matches!(parse::<Sig>("id"), Err(SyntaxError::Parse { .. })));
    assert!(matches!(
        parse::<Sig>("braid"),
        Err(SyntaxError::Parse { .. })
    ));
}

#[test]
fn overflowing_arity_errors_cleanly() {
    // 10^40 overflows usize without panicking.
    assert!(matches!(
        parse::<Sig>("id(10000000000000000000000000000000000000000)"),
        Err(SyntaxError::Parse { .. })
    ));
}

#[test]
fn empty_input_errors() {
    assert!(matches!(
        parse::<Sig>(""),
        Err(SyntaxError::Parse { offset: 0, .. })
    ));
    assert!(matches!(
        parse::<Sig>("   "),
        Err(SyntaxError::Parse { .. })
    ));
}

// ---- Arity failure passes through as Catgraph --------------------------------

#[test]
fn arity_mismatch_surfaces_as_catgraph() {
    // copy : 1 → 2, so `copy ; copy` fails the composition arity check (2 ≠ 1).
    assert!(matches!(
        parse::<Sig>("copy ; copy"),
        Err(SyntaxError::Catgraph(_))
    ));
}
