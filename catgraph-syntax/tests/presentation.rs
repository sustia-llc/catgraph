//! Presentation-file print/parse tests (Phase S2, Seven Sketches Def 5.33).

mod common;

use catgraph_applied::prop::Free;
use catgraph_applied::prop::presentation::Presentation;
use catgraph_syntax::errors::SyntaxError;
use catgraph_syntax::text::{parse_presentation, print_presentation};
use common::{Sig, g};

/// `copy ; add` and `id(1)` are both `1 → 1`, an arity-valid equation pair.
fn sample_presentation() -> Presentation<Sig> {
    let mut p = Presentation::<Sig>::new();
    let lhs = Free::compose(g(Sig::Copy), g(Sig::Add)).unwrap();
    p.add_equation(lhs, Free::identity(1)).unwrap();
    // A second `2 → 2` equation exercises multi-line files.
    let braid = Free::<Sig>::braid(1, 1);
    p.add_equation(braid, Free::identity(2)).unwrap();
    p
}

#[test]
fn presentation_round_trips() {
    let p = sample_presentation();
    let text = print_presentation(&p);
    assert_eq!(text, "copy ; add = id(1)\nbraid(1,1) = id(2)");

    let reparsed = parse_presentation::<Sig>(&text).unwrap();
    assert_eq!(reparsed.equations(), p.equations());
}

#[test]
fn blank_and_whitespace_lines_are_skipped() {
    let text = "\ncopy ; add = id(1)\n   \n\nbraid(1,1) = id(2)\n";
    let reparsed = parse_presentation::<Sig>(text).unwrap();
    assert_eq!(reparsed.equations(), sample_presentation().equations());
}

#[test]
fn arity_mismatch_line_is_catgraph() {
    // copy : 1 → 2, add : 2 → 1 — the two sides have mismatched arity, so
    // `add_equation` rejects it and the failure passes through as Catgraph.
    assert!(matches!(
        parse_presentation::<Sig>("copy = add"),
        Err(SyntaxError::Catgraph(_))
    ));
}

#[test]
fn line_without_equals_is_parse_error() {
    match parse_presentation::<Sig>("copy ; add") {
        Err(SyntaxError::Parse { offset, .. }) => assert_eq!(offset, 0),
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[test]
fn line_with_two_equals_is_parse_error() {
    assert!(matches!(
        parse_presentation::<Sig>("copy = id(1) = id(1)"),
        Err(SyntaxError::Parse { .. })
    ));
}

#[test]
fn parse_error_offset_is_whole_input_relative() {
    // The bad token `nope` sits on the second line; its reported offset must be
    // relative to the whole input, not the line.
    let text = "braid(1,1) = id(2)\ncopy ; nope = id(1)";
    let line2_start = "braid(1,1) = id(2)\n".len();
    match parse_presentation::<Sig>(text) {
        Err(SyntaxError::Parse { offset, message }) => {
            assert_eq!(offset, line2_start + 7, "offset should point at `nope`");
            assert!(message.contains("nope"), "got: {message}");
        }
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[test]
fn empty_input_is_empty_presentation() {
    let p = parse_presentation::<Sig>("").unwrap();
    assert!(p.equations().is_empty());
    assert_eq!(print_presentation(&p), "");
}
