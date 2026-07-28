//! Boundary-word equality on `Presentation::add_equation` (#79 P2).
//!
//! An equation carries no *declared* source word, so `add_equation` infers one:
//! fresh variables stand for the unknown source colors and both sides are
//! threaded through the same variables, so a constraint discovered on either
//! side propagates to the other. These tests pin the four rejections (source /
//! target × color / length), and the two acceptances that a naive
//! "every side must have a concrete word" reading would wrongly reject.

use std::borrow::Cow;

use catgraph::errors::CatgraphError;
use catgraph_applied::prop::presentation::Presentation;
use catgraph_applied::prop::{Free, PropExpr, PropSignature};

// ---- Test signature over Λ = {A, B} -----------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Wire {
    A,
    B,
}

const W_A: &[Wire] = &[Wire::A];
const W_B: &[Wire] = &[Wire::B];
const W_AB: &[Wire] = &[Wire::A, Wire::B];
const W_BA: &[Wire] = &[Wire::B, Wire::A];

/// `F : A → B`, `G : B → A`, `H : A B → B A`, `H2 : B A → A B` (H's mirror),
/// `P : A → B` (a second generator parallel to `F`, so `F = P` is a valid
/// equation), `Q : B → B`, and `Wide : A → A B` (the only arity-changing one).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Two {
    F,
    G,
    H,
    H2,
    P,
    Q,
    Wide,
}

impl PropSignature for Two {
    type Color = Wire;

    fn source_word(&self) -> Cow<'_, [Wire]> {
        Cow::Borrowed(match self {
            Two::F | Two::P | Two::Wide => W_A,
            Two::G | Two::Q => W_B,
            Two::H => W_AB,
            Two::H2 => W_BA,
        })
    }

    fn target_word(&self) -> Cow<'_, [Wire]> {
        Cow::Borrowed(match self {
            Two::F | Two::P | Two::Q => W_B,
            Two::G => W_A,
            Two::H => W_BA,
            Two::H2 | Two::Wide => W_AB,
        })
    }
}

fn g(x: Two) -> PropExpr<Two> {
    Free::<Two>::generator(x)
}

// ---- Acceptance -------------------------------------------------------------

#[test]
fn parallel_colored_equation_is_accepted() {
    // F : A → B and P : A → B are parallel — same source word, same target word.
    let mut pres = Presentation::<Two>::new();
    pres.add_equation(g(Two::F), g(Two::P))
        .expect("A → B on both sides");

    // A composite is fine too: F ; Q : A → B (B in the middle) equals P.
    let f_then_q = Free::compose(g(Two::F), g(Two::Q)).expect("F : A → B ; Q : B → B");
    pres.add_equation(f_then_q, g(Two::P))
        .expect("A → B on both sides");
}

#[test]
fn fully_polymorphic_equation_is_accepted() {
    // Braid involution: σ ; σ = id(2). Neither side mentions a generator, so
    // the source variables stay unbound and the equation holds at every word.
    let mut pres = Presentation::<Two>::new();
    let braid = Free::<Two>::braid(1, 1);
    let sigma_twice = Free::compose(braid.clone(), braid).expect("2 → 2 ; 2 → 2");
    pres.add_equation(sigma_twice, Free::identity(2))
        .expect("color-polymorphic on both sides");
}

#[test]
fn jointly_constrained_polymorphic_equation_is_accepted() {
    // `id(2) = σ` is not well-formed at *every* word: threading `v0 v1` gives
    // targets `v0 v1` and `v1 v0`, whose pairwise unification aliases v0 ~ v1.
    // The equation is exactly the claim "on any two wires of the SAME color,
    // swapping is the identity" — accepted, with the constraint inferred and
    // then discarded (nothing rewrites `ColoredExpr` by user equations).
    let mut pres = Presentation::<Two>::new();
    pres.add_equation(Free::<Two>::identity(2), Free::<Two>::braid(1, 1))
        .expect("jointly satisfiable: the two source positions share a color");

    // The constraint is not stored — the equation is kept verbatim.
    assert_eq!(pres.equations().len(), 1);
    assert_eq!(pres.equations()[0].0, PropExpr::Identity(2));
}

// ---- Rejection: colors ------------------------------------------------------

#[test]
fn source_color_mismatch_between_sides_is_rejected() {
    // G : B → A on the left, F : A → B on the right. The lhs binds the single
    // source variable to B; the rhs then demands A at the same position.
    let mut pres = Presentation::<Two>::new();
    match pres.add_equation(g(Two::G), g(Two::F)) {
        Err(CatgraphError::Composition { message }) => {
            assert!(message.contains('F'), "names the rhs generator: {message}");
            assert!(message.contains('A') && message.contains('B'), "{message}");
        }
        other => panic!("expected a source color conflict, got {other:?}"),
    }
}

#[test]
fn target_color_mismatch_between_sides_is_rejected() {
    // F : A → B and G ; F… simpler: both sides consume A, but produce B vs A.
    // `F : A → B` against `id(1)`, which relays the source color A.
    let mut pres = Presentation::<Two>::new();
    match pres.add_equation(g(Two::F), Free::<Two>::identity(1)) {
        Err(CatgraphError::Composition { message }) => {
            assert!(
                message.contains("target position 0"),
                "names the target position: {message}"
            );
            assert!(message.contains('A') && message.contains('B'), "{message}");
        }
        other => panic!("expected a target color conflict, got {other:?}"),
    }
}

// ---- Rejection: lengths -----------------------------------------------------

#[test]
fn source_arity_mismatch_between_sides_is_rejected() {
    // H : A B → B A is 2 → 2; F : A → B is 1 → 1. The rhs is handed the lhs's
    // two-letter source word.
    let mut pres = Presentation::<Two>::new();
    let result = pres.add_equation(g(Two::H), g(Two::F));
    assert!(
        matches!(
            result,
            Err(CatgraphError::CompositionSizeMismatch {
                expected: 1,
                actual: 2
            })
        ),
        "expected the rhs to reject a 2-letter word, got {result:?}"
    );
}

#[test]
fn target_arity_mismatch_between_sides_is_rejected() {
    // Both sides consume A, but `Wide : A → A B` produces two letters where
    // `F : A → B` produces one.
    let mut pres = Presentation::<Two>::new();
    let result = pres.add_equation(g(Two::Wide), g(Two::F));
    assert!(
        matches!(
            result,
            Err(CatgraphError::CompositionSizeMismatch {
                expected: 2,
                actual: 1
            })
        ),
        "expected a target-length mismatch, got {result:?}"
    );
}

#[test]
fn cross_side_variable_conflict_is_rejected() {
    // Both sides relay a *variable* into target position 0, but each side bound
    // the other's variable to a different color, so the conflict is discovered
    // only when the two target words meet.
    //
    //   lhs = (H ⊗ id(2)) ; σ_{2,2}   consumes v0 v1 as `A B`, relays v2 v3
    //                                 into positions 0,1
    //   rhs = id(2) ⊗ H2              consumes v2 v3 as `B A`, relays v0 v1
    //                                 into positions 0,1
    //
    // Position 0 pairs v2 (bound B by the rhs) against v0 (bound A by the lhs).
    let lhs = Free::compose(
        Free::tensor(g(Two::H), Free::<Two>::identity(2)),
        Free::<Two>::braid(2, 2),
    )
    .expect("4 → 4 ; 4 → 4");
    let rhs = Free::tensor(Free::<Two>::identity(2), g(Two::H2));

    let mut pres = Presentation::<Two>::new();
    match pres.add_equation(lhs, rhs) {
        Err(CatgraphError::Composition { message }) => {
            assert!(
                message.contains("target position 0"),
                "names the target position: {message}"
            );
            assert!(message.contains('A') && message.contains('B'), "{message}");
        }
        other => panic!("expected a cross-side variable conflict, got {other:?}"),
    }
}

// ---- Word threading inside a side -------------------------------------------

#[test]
fn ill_colored_composite_side_is_rejected() {
    // `F ; F` passes `Free::compose`'s arity check (1 = 1) but F emits B and
    // consumes A. The equation's rhs is irrelevant — the lhs alone is ill-formed.
    let f_then_f = Free::compose(g(Two::F), g(Two::F)).expect("arity check accepts F ; F");
    let mut pres = Presentation::<Two>::new();
    match pres.add_equation(f_then_f, g(Two::P)) {
        Err(CatgraphError::Composition { message }) => {
            assert!(message.contains('F'), "names the generator: {message}");
        }
        other => panic!("expected a color conflict inside the lhs, got {other:?}"),
    }
}

#[test]
fn braid_carries_colors_across_an_equation() {
    // H : A B → B A equals σ_{1,1} at the word A B: the braid block-swaps
    // `A B` to `B A`, which is exactly H's target word.
    let mut pres = Presentation::<Two>::new();
    pres.add_equation(g(Two::H), Free::<Two>::braid(1, 1))
        .expect("both sides A B → B A");

    // Sanity on the words this test rests on.
    assert_eq!(&*Two::H.source_word(), W_AB);
    assert_eq!(&*Two::H.target_word(), W_BA);
    assert_eq!(&*Two::F.source_word(), W_A);
    assert_eq!(&*Two::F.target_word(), W_B);
}
