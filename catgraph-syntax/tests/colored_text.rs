//! The Λ-colored **textual** surface
//! ([#79](https://github.com/sustia-llc/catgraph/issues/79) P3b).
//!
//! The palette is `Λ = {A, B}` (`common::Hue`), which has **no implicit sort**,
//! so every spider prints annotated and a bare `mu` is a parse error. Coverage:
//!
//! - colored spider tokens `mu@A` / `eta@A` / `delta@A` / `epsilon@A`, the
//!   clause-1 token round-trip over the colored signature, and the S2
//!   whole-expression round-trip law extended to it;
//! - the declaration grammar `TOKEN : COLOR* -> COLOR*` — a whole colored file
//!   printing declarations first and re-parsing to the same equation list;
//! - the **mixed** palette (test-local `Tint`), which has both an implicit sort
//!   and a visible one — the only shape that exercises a bare spider token and
//!   an `•`-spelled declaration position in the *same* file;
//! - the declaration **drift checks**: unknown generator token, unknown colour
//!   token, and a declared word disagreeing with the signature's own;
//! - the malformed-declaration diagnostics (extra `:`, missing/duplicate `->`,
//!   multiple tokens before the `:`) and their byte offsets.

mod common;

use std::borrow::Cow;

use catgraph_applied::prop::presentation::Presentation;
use catgraph_applied::prop::{Free, PropSignature};
use catgraph_syntax::errors::SyntaxError;
use catgraph_syntax::frobenius::FrobeniusOr;
use catgraph_syntax::text::{
    ColorSyntax, GeneratorSyntax, parse, parse_presentation, print, print_presentation,
};
use common::{ColoredSig, Hue, arb_colored_frob_gen, arb_colored_frob_leaf, arb_expr};
use proptest::prelude::*;

type Gen = FrobeniusOr<ColoredSig>;

/// A colored presentation over `Λ = {A, B}`: speciality at `A` (Def 2.5 #9) and
/// the relation naming `swap` the braid it is called after.
fn colored_presentation() -> Presentation<Gen> {
    let mut p = Presentation::<Gen>::new();
    let speciality = Free::compose(
        Free::generator(FrobeniusOr::Delta(Hue::A)),
        Free::generator(FrobeniusOr::Mu(Hue::A)),
    )
    .expect("δ_A : [A] → [A,A] ; μ_A : [A,A] → [A]");
    p.add_equation(speciality, Free::identity(1))
        .expect("both sides read [A] → [A]");

    let swap_is_braid = Free::compose(
        Free::generator(FrobeniusOr::User(ColoredSig::Swap)),
        Free::braid(1, 1),
    )
    .expect("swap : [A,B] → [B,A] ; braid(1,1) : [B,A] → [A,B]");
    p.add_equation(swap_is_braid, Free::identity(2))
        .expect("both sides read [A,B] → [A,B]");
    p
}

/// The file `colored_presentation` prints: three declarations in `Ord` order
/// (`Mu < Eta < Delta < Epsilon < User`), then the two equations.
const COLORED_FILE: &str = "\
mu@A : A A -> A
delta@A : A -> A A
swap : A B -> B A
delta@A ; mu@A = id(1)
swap ; braid(1,1) = id(2)";

// ---- Colored spider tokens ---------------------------------------------------

#[test]
fn spiders_print_and_parse_annotated() {
    for (generator, token) in [
        (FrobeniusOr::<ColoredSig>::Mu(Hue::A), "mu@A"),
        (FrobeniusOr::Eta(Hue::B), "eta@B"),
        (FrobeniusOr::Delta(Hue::A), "delta@A"),
        (FrobeniusOr::Epsilon(Hue::B), "epsilon@B"),
        (FrobeniusOr::User(ColoredSig::Swap), "swap"),
    ] {
        assert_eq!(generator.print_token(), token);
        assert_eq!(Gen::parse_token(token), Some(generator));
    }
}

#[test]
fn the_monochromatic_token_is_unchanged() {
    // `()` is the all-implicit palette, so the pre-#79 spelling survives.
    use common::Sig;
    assert_eq!(FrobeniusOr::<Sig>::Mu(()).print_token(), "mu");
    assert_eq!(
        FrobeniusOr::<Sig>::parse_token("mu"),
        Some(FrobeniusOr::Mu(()))
    );
}

#[test]
fn a_bare_spider_needs_an_implicit_sort() {
    // `Hue::implicit()` is `None`, so there is no colour to default a bare `mu`
    // to and the token is rejected rather than silently mistyped.
    assert_eq!(Gen::parse_token("mu"), None);
    assert!(parse::<Gen>("mu").is_err());
}

#[test]
fn an_unknown_colour_annotation_is_rejected() {
    assert_eq!(Gen::parse_token("mu@X"), None);
    assert_eq!(Gen::parse_token("mu@"), None);
    match parse::<Gen>("mu@X") {
        Err(SyntaxError::Parse { offset, message }) => {
            assert_eq!(offset, 0);
            assert!(message.contains("mu@X"), "got: {message}");
        }
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[test]
fn a_colour_joiner_cannot_be_spelled_by_a_user_token() {
    // `@` is clause-2 reserved, so `mu@A` is unreachable as a `G`-token: it is
    // always the colour-annotated spider. This is the `@` analogue of the `mu`
    // shadowing documented on the `GeneratorSyntax` impl — and the reason a
    // user token may not contain `@` at all.
    assert_eq!(Gen::parse_token("mu@A"), Some(FrobeniusOr::Mu(Hue::A)));
    assert_eq!(ColoredSig::parse_token("mu@A"), None);
}

proptest! {
    /// Clause 1 over the colored signature.
    #[test]
    fn colored_token_clause1(generator in arb_colored_frob_gen()) {
        prop_assert_eq!(Gen::parse_token(&generator.print_token()), Some(generator));
    }

    /// The S2 round-trip law, extended to a colored signature.
    #[test]
    fn colored_expression_roundtrips(e in arb_expr(arb_colored_frob_leaf())) {
        let printed = print(&e);
        prop_assert_eq!(parse::<Gen>(&printed), Ok(e));
    }
}

// ---- Declaration grammar -----------------------------------------------------

#[test]
fn colored_presentation_round_trips_with_declarations() {
    let p = colored_presentation();
    let text = print_presentation(&p);
    assert_eq!(text, COLORED_FILE);

    let reparsed = parse_presentation::<Gen>(&text).expect("the printed file re-reads");
    assert_eq!(reparsed.equations(), p.equations());
}

#[test]
fn a_declaration_free_colored_file_still_parses() {
    let equations = "delta@A ; mu@A = id(1)\nswap ; braid(1,1) = id(2)";
    let p = parse_presentation::<Gen>(equations).expect("declarations are optional");
    assert_eq!(p.equations(), colored_presentation().equations());
}

#[test]
fn declarations_may_come_after_the_equations() {
    let text = "delta@A ; mu@A = id(1)\nmu@A : A A -> A";
    assert!(parse_presentation::<Gen>(text).is_ok());
}

#[test]
fn an_empty_colour_word_is_written_as_nothing() {
    // η_B : [] → [B]; the source side of the declaration is simply empty.
    let mut p = Presentation::<Gen>::new();
    p.add_equation(
        Free::generator(FrobeniusOr::Eta(Hue::B)),
        Free::generator(FrobeniusOr::Eta(Hue::B)),
    )
    .expect("reflexive, hence parallel");
    let text = print_presentation(&p);
    assert!(
        text.starts_with("eta@B : -> B\n"),
        "expected an empty source word, got: {text}"
    );
    assert!(parse_presentation::<Gen>(&text).is_ok());
}

// ---- A palette with an implicit sort *and* a visible one ---------------------

/// The mixed case neither `Hue` (all-visible) nor `()` (all-implicit) can show:
/// `Plain` is the implicit sort — bare on a spider token, `•` where a
/// declaration word must fill the position — and `Vivid` prints `V`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Tint {
    Plain,
    Vivid,
}

impl ColorSyntax for Tint {
    fn print_color(&self) -> Option<String> {
        match self {
            Tint::Plain => None,
            Tint::Vivid => Some("V".to_string()),
        }
    }

    fn parse_color(token: &str) -> Option<Self> {
        match token {
            "•" => Some(Tint::Plain),
            "V" => Some(Tint::Vivid),
            _ => None,
        }
    }

    fn implicit() -> Option<Self> {
        Some(Tint::Plain)
    }
}

/// `mix : [Plain, Vivid] → [Vivid]` over the [`Tint`] palette.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Mix;

impl PropSignature for Mix {
    type Color = Tint;

    fn source_word(&self) -> Cow<'_, [Tint]> {
        Cow::Owned(vec![Tint::Plain, Tint::Vivid])
    }

    fn target_word(&self) -> Cow<'_, [Tint]> {
        Cow::Owned(vec![Tint::Vivid])
    }
}

impl GeneratorSyntax for Mix {
    fn print_token(&self) -> String {
        "mix".to_string()
    }

    fn parse_token(token: &str) -> Option<Self> {
        (token == "mix").then_some(Mix)
    }
}

#[test]
fn an_implicit_sort_is_bare_on_a_token_and_a_dot_in_a_word() {
    // ε_Plain prints bare while ε_Vivid is annotated…
    assert_eq!(
        FrobeniusOr::<Mix>::Epsilon(Tint::Plain).print_token(),
        "epsilon"
    );
    assert_eq!(
        FrobeniusOr::<Mix>::parse_token("epsilon"),
        Some(FrobeniusOr::Epsilon(Tint::Plain))
    );
    assert_eq!(
        FrobeniusOr::<Mix>::Epsilon(Tint::Vivid).print_token(),
        "epsilon@V"
    );

    // …and `mix ; ε_Vivid = ε_Plain ⊗ ε_Vivid` ("mix then discard = discard
    // both") prints declarations whose implicit positions are spelled `•`.
    let mut p = Presentation::<FrobeniusOr<Mix>>::new();
    let lhs = Free::compose(
        Free::generator(FrobeniusOr::User(Mix)),
        Free::generator(FrobeniusOr::Epsilon(Tint::Vivid)),
    )
    .expect("mix : 2 → 1 ; ε : 1 → 0");
    let rhs = Free::tensor(
        Free::generator(FrobeniusOr::Epsilon(Tint::Plain)),
        Free::generator(FrobeniusOr::Epsilon(Tint::Vivid)),
    );
    p.add_equation(lhs, rhs)
        .expect("both sides read [Plain, Vivid] → []");

    let text = print_presentation(&p);
    assert_eq!(
        text,
        "epsilon : • ->\n\
         epsilon@V : V ->\n\
         mix : • V -> V\n\
         mix ; epsilon@V = epsilon * epsilon@V"
    );
    assert_eq!(
        parse_presentation::<FrobeniusOr<Mix>>(&text)
            .expect("the printed file re-reads")
            .equations(),
        p.equations()
    );
}

// ---- Declaration drift checks ------------------------------------------------

#[test]
fn an_unknown_generator_token_is_rejected() {
    match parse_presentation::<Gen>("nope : A -> A") {
        Err(SyntaxError::Parse { offset, message }) => {
            assert_eq!(offset, 0);
            assert!(message.contains("nope"), "got: {message}");
        }
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[test]
fn an_unknown_colour_token_is_rejected() {
    match parse_presentation::<Gen>("mu@A : A X -> A") {
        Err(SyntaxError::Parse { offset, message }) => {
            assert_eq!(offset, 9, "offset should point at `X`");
            assert!(message.contains('X'), "got: {message}");
        }
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[test]
fn a_word_mismatch_names_both_words() {
    // μ_A's source word is [A, A]; the file says [A, B].
    match parse_presentation::<Gen>("mu@A : A B -> A") {
        Err(SyntaxError::Parse { message, .. }) => {
            assert!(message.contains("mu@A"), "names the generator: {message}");
            assert!(
                message.contains("[A, B]"),
                "names the declared word: {message}"
            );
            assert!(
                message.contains("[A, A]"),
                "names the actual word: {message}"
            );
        }
        other => panic!("expected Parse, got {other:?}"),
    }
    // …and the target side is checked too (μ_A's is [A], not [A, A]).
    assert!(matches!(
        parse_presentation::<Gen>("mu@A : A A -> A A"),
        Err(SyntaxError::Parse { .. })
    ));
}

// ---- Malformed declarations --------------------------------------------------

#[test]
fn a_second_colon_is_rejected_at_its_own_offset() {
    match parse_presentation::<Gen>("mu@A : A A -> A : A") {
        Err(SyntaxError::Parse { offset, message }) => {
            assert_eq!(offset, 16);
            assert!(message.contains("exactly one"), "got: {message}");
        }
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[test]
fn a_missing_or_duplicated_arrow_is_rejected() {
    for line in ["mu@A : A A A", "mu@A : A A -> A -> A"] {
        match parse_presentation::<Gen>(line) {
            Err(SyntaxError::Parse { message, .. }) => {
                assert!(message.contains("->"), "got: {message}");
            }
            other => panic!("expected Parse for {line:?}, got {other:?}"),
        }
    }
}

#[test]
fn the_declared_token_must_be_a_single_atom() {
    match parse_presentation::<Gen>("mu@A mu@A : A A -> A") {
        Err(SyntaxError::Parse { message, .. }) => {
            assert!(
                message.contains("exactly one generator token"),
                "got: {message}"
            );
        }
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[test]
fn declaration_offsets_are_whole_input_relative() {
    let text = "delta@A ; mu@A = id(1)\nnope : A -> A";
    let line2_start = "delta@A ; mu@A = id(1)\n".len();
    match parse_presentation::<Gen>(text) {
        Err(SyntaxError::Parse { offset, .. }) => assert_eq!(offset, line2_start),
        other => panic!("expected Parse, got {other:?}"),
    }
}
