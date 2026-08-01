//! Λ-colored well-formedness pass (#79 P1).
//!
//! Colors flow top-down: an `Identity` / `Braid` carries no intrinsic word, so
//! well-formedness is a check pass over the pair `(source word, expr)` rather
//! than a smart-constructor condition. The decisive case these tests pin is the
//! *arity-equal / color-unequal* composition, which the `usize`-only
//! `Free::compose` check accepts and `check` must reject.

use std::borrow::Cow;

use catgraph::errors::CatgraphError;
use catgraph_applied::prop::colored::{ColoredExpr, check};
use catgraph_applied::prop::presentation::Presentation;
use catgraph_applied::prop::{Free, PropExpr, PropSignature, mono_word};

// ---- Two-color test signature over Λ = {A, B} -------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum Wire {
    A,
    B,
}

const W_A: &[Wire] = &[Wire::A];
const W_B: &[Wire] = &[Wire::B];
const W_AB: &[Wire] = &[Wire::A, Wire::B];
const W_BA: &[Wire] = &[Wire::B, Wire::A];

/// `F : A → B`, `G : B → A`, `H : A B → B A`. Words are stored, so
/// `source_word` / `target_word` hand back `Cow::Borrowed` — no allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum Two {
    F,
    G,
    H,
}

impl PropSignature for Two {
    type Color = Wire;

    fn source_word(&self) -> Cow<'_, [Wire]> {
        Cow::Borrowed(match self {
            Two::F => W_A,
            Two::G => W_B,
            Two::H => W_AB,
        })
    }

    fn target_word(&self) -> Cow<'_, [Wire]> {
        Cow::Borrowed(match self {
            Two::F => W_B,
            Two::G => W_A,
            Two::H => W_BA,
        })
    }
}

// ---- Monochromatic test signature (Λ = {•}, spelled `()`) -------------------

/// `Copy : 1 → 2`, `Add : 2 → 1` — arities overridden, words derived from them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Mono {
    Copy,
    Add,
}

impl PropSignature for Mono {
    type Color = ();

    fn source_word(&self) -> Cow<'_, [()]> {
        mono_word(self.source())
    }

    fn target_word(&self) -> Cow<'_, [()]> {
        mono_word(self.target())
    }

    fn source(&self) -> usize {
        match self {
            Mono::Copy => 1,
            Mono::Add => 2,
        }
    }

    fn target(&self) -> usize {
        match self {
            Mono::Copy => 2,
            Mono::Add => 1,
        }
    }
}

// ---- check: well-formed flows ----------------------------------------------

#[test]
fn compose_threads_words_through_the_interface() {
    // F ; G : A → A.
    let expr = Free::compose(Free::generator(Two::F), Free::generator(Two::G))
        .expect("F(1→1) ; G(1→1) arity-composes");
    assert_eq!(check(&expr, W_A).expect("A → B → A"), vec![Wire::A]);
}

#[test]
fn tensor_concatenates_the_two_output_words() {
    // F ⊗ G : A B → B A.
    let expr = Free::tensor(Free::generator(Two::F), Free::generator(Two::G));
    assert_eq!(
        check(&expr, W_AB).expect("A B → B A"),
        vec![Wire::B, Wire::A]
    );
}

#[test]
fn generator_word_is_matched_not_merely_counted() {
    let expr: PropExpr<Two> = Free::generator(Two::H);
    assert_eq!(
        check(&expr, W_AB).expect("A B → B A"),
        vec![Wire::B, Wire::A]
    );
    // Same arity (2), wrong colors.
    assert!(matches!(
        check(&expr, W_BA),
        Err(CatgraphError::Composition { .. })
    ));
}

// ---- check: the decisive rejection -----------------------------------------

/// The case a `usize`-only interface check cannot see: `F ; F` has matching
/// arities (`1 = 1`) so `Free::compose` accepts it, but `F` outputs `B` and
/// consumes `A`.
#[test]
fn arity_equal_color_unequal_composition_is_rejected() {
    let expr = Free::compose(Free::generator(Two::F), Free::generator(Two::F))
        .expect("arity check accepts F ; F");
    assert_eq!(expr.source(), 1);
    assert_eq!(expr.target(), 1);

    match check(&expr, W_A) {
        Err(CatgraphError::Composition { message }) => {
            assert!(
                message.contains('F'),
                "message names the generator: {message}"
            );
            assert!(message.contains('A') && message.contains('B'), "{message}");
        }
        other => panic!("expected a color mismatch, got {other:?}"),
    }
}

#[test]
fn wrong_length_is_a_size_mismatch_not_a_color_mismatch() {
    let expr: PropExpr<Two> = Free::generator(Two::H);
    assert!(matches!(
        check(&expr, W_A),
        Err(CatgraphError::CompositionSizeMismatch {
            expected: 2,
            actual: 1
        })
    ));

    let id: PropExpr<Two> = Free::identity(2);
    assert!(matches!(
        check(&id, W_A),
        Err(CatgraphError::CompositionSizeMismatch {
            expected: 2,
            actual: 1
        })
    ));

    // A tensor whose input is too short to even reach the right factor.
    let wide = Free::tensor(Free::<Two>::identity(2), Free::generator(Two::F));
    assert!(matches!(
        check(&wide, W_A),
        Err(CatgraphError::CompositionSizeMismatch { .. })
    ));
}

// ---- check: color-polymorphic atoms ----------------------------------------

#[test]
fn braid_block_swaps_the_word() {
    // σ_{1,2} over A·BB ↦ BB·A.
    let expr: PropExpr<Two> = Free::braid(1, 2);
    let input = [Wire::A, Wire::B, Wire::B];
    assert_eq!(
        check(&expr, &input).expect("1+2 wires"),
        vec![Wire::B, Wire::B, Wire::A]
    );

    // σ_{2,1} over AB·B ↦ B·AB.
    let expr: PropExpr<Two> = Free::braid(2, 1);
    assert_eq!(
        check(&expr, &input).expect("2+1 wires"),
        vec![Wire::B, Wire::A, Wire::B]
    );
}

#[test]
fn identity_is_color_polymorphic() {
    let id: PropExpr<Two> = Free::identity(2);
    assert_eq!(check(&id, W_AB).expect("A B"), vec![Wire::A, Wire::B]);
    assert_eq!(check(&id, W_BA).expect("B A"), vec![Wire::B, Wire::A]);
    let bb = [Wire::B, Wire::B];
    assert_eq!(check(&id, &bb).expect("B B"), vec![Wire::B, Wire::B]);
}

// ---- ColoredExpr ------------------------------------------------------------

#[test]
fn colored_expr_records_both_boundary_words() {
    let expr = Free::tensor(Free::generator(Two::F), Free::generator(Two::G));
    let m = ColoredExpr::new(W_AB.to_vec(), expr.clone()).expect("well-formed");
    assert_eq!(m.source_word(), W_AB);
    assert_eq!(m.target_word(), W_BA);
    assert_eq!(m.expr(), &expr);

    let (s, t, e) = m.into_inner();
    assert_eq!(s, W_AB);
    assert_eq!(t, W_BA);
    assert_eq!(e, expr);
}

#[test]
fn colored_expr_new_rejects_ill_colored_terms() {
    let expr = Free::compose(Free::generator(Two::F), Free::generator(Two::F))
        .expect("arity check accepts F ; F");
    assert!(ColoredExpr::new(W_A.to_vec(), expr).is_err());
}

/// Interchange: `(F ⊗ id) ; (id ⊗ G)` and `(id ⊗ G) ; (F ⊗ id)` are two
/// writings of the same morphism `A B → B A`. Structurally distinct, `nf`-equal,
/// same boundary words — so `eq_colored` holds and `==` does not.
#[test]
fn eq_colored_identifies_an_interchange_pair() {
    let f_first = Free::tensor(Free::generator(Two::F), Free::<Two>::identity(1)); // A B → B B
    let g_second = Free::tensor(Free::<Two>::identity(1), Free::generator(Two::G)); // B B → B A
    let lhs = Free::compose(f_first, g_second).expect("2→2 ; 2→2");

    let g_first = Free::tensor(Free::<Two>::identity(1), Free::generator(Two::G)); // A B → A A
    let f_second = Free::tensor(Free::generator(Two::F), Free::<Two>::identity(1)); // A A → B A
    let rhs = Free::compose(g_first, f_second).expect("2→2 ; 2→2");

    assert_ne!(lhs, rhs, "the two writings are structurally distinct");

    let a = ColoredExpr::new(W_AB.to_vec(), lhs).expect("A B → B A");
    let b = ColoredExpr::new(W_AB.to_vec(), rhs).expect("A B → B A");
    assert_eq!(a.target_word(), W_BA);
    assert_eq!(b.target_word(), W_BA);
    assert!(a.eq_colored(&b));
    assert_ne!(a, b, "structural `==` is the pre-quotient equality");
}

/// Same normal form, different words: the boundary-word conjunct is what
/// separates them. A monochromatic-only comparison would call these equal.
#[test]
fn eq_colored_separates_equal_normal_forms_with_different_words() {
    let id_a = ColoredExpr::new(W_A.to_vec(), Free::<Two>::identity(1)).expect("A → A");
    let id_b = ColoredExpr::new(W_B.to_vec(), Free::<Two>::identity(1)).expect("B → B");
    assert!(!id_a.eq_colored(&id_b));

    let braid = Free::<Two>::braid(1, 1);
    let mixed = ColoredExpr::new(W_AB.to_vec(), braid.clone()).expect("A B → B A");
    let uniform = ColoredExpr::new(vec![Wire::A, Wire::A], braid).expect("A A → A A");
    assert_eq!(mixed.expr(), uniform.expr());
    assert!(!mixed.eq_colored(&uniform));
}

// ---- Monochromatic path -----------------------------------------------------

#[test]
fn mono_check_passes_whenever_arities_are_well_formed() {
    // copy ; add : 1 → 1.
    let expr = Free::compose(Free::generator(Mono::Copy), Free::generator(Mono::Add))
        .expect("copy(1→2) ; add(2→1)");
    let out = check(&expr, &[()]).expect("mono words never mismatch on color");
    assert_eq!(out.len(), 1);

    // Only lengths can go wrong.
    assert!(matches!(
        check(&expr, &[(), ()]),
        Err(CatgraphError::CompositionSizeMismatch {
            expected: 1,
            actual: 2
        })
    ));

    let m = ColoredExpr::new(vec![()], expr).expect("well-formed");
    assert_eq!(m.source_word().len(), 1);
    assert_eq!(m.target_word().len(), 1);
}

#[test]
fn mono_words_are_zst_backed_and_never_allocate() {
    let w = mono_word(4);
    assert_eq!(w.len(), 4);
    assert!(matches!(w, Cow::Owned(_)));
    // A `Vec<()>` allocates nothing; its capacity is the ZST sentinel.
    assert_eq!(w.into_owned().capacity(), usize::MAX);
    assert!(mono_word(0).is_empty());

    // The trait's provided arities agree with the stored word lengths.
    assert_eq!(Mono::Copy.source_word().len(), Mono::Copy.source());
    assert_eq!(Mono::Copy.target_word().len(), Mono::Copy.target());
    assert_eq!(Two::H.source(), 2);
    assert_eq!(Two::H.target(), 2);
}

// ---- Overflowing wire counts (#180) -----------------------------------------

/// A directly-constructed `Braid(usize::MAX, 1)` is documented-legal, and its
/// `m + n` used to overflow — a debug-build panic, a release-build wrap onto a
/// small arity that a short input could spuriously satisfy. Saturating to
/// `usize::MAX` matches no slice length, so `check` reports the mismatch.
#[test]
fn check_rejects_an_overflowing_braid_instead_of_panicking() {
    let over: PropExpr<Two> = PropExpr::Braid(usize::MAX, 1);
    assert_eq!(over.source(), usize::MAX);
    for input in [&[][..], W_A, W_AB] {
        assert!(matches!(
            check(&over, input),
            Err(CatgraphError::CompositionSizeMismatch {
                expected: usize::MAX,
                ..
            })
        ));
    }

    // Tensor arm: the split point comes from `f.source()`, so hardening the
    // fold covers it — the left factor's saturated arity is never reached by a
    // real input, and `expect_at_least` reports that rather than wrapping.
    let wide = Free::tensor(PropExpr::<Two>::Braid(usize::MAX, 0), PropExpr::Braid(1, 0));
    assert_eq!(wide.source(), usize::MAX);
    assert!(matches!(
        check(&wide, W_AB),
        Err(CatgraphError::CompositionSizeMismatch {
            expected: usize::MAX,
            actual: 2
        })
    ));
}

/// The `infer` sibling, reached through its only public entry point. The
/// overflowing braid sits on the RHS: `check_equation` sizes the fresh source
/// word from `lhs.source()`, so a small LHS keeps the test cheap while the
/// RHS still drives `infer`'s `Braid` arm.
#[test]
fn add_equation_rejects_an_overflowing_braid_on_the_rhs() {
    let mut p: Presentation<Two> = Presentation::new();
    let lhs: PropExpr<Two> = Free::identity(2);
    let rhs: PropExpr<Two> = PropExpr::Braid(usize::MAX, 1);
    assert!(matches!(
        p.add_equation(lhs, rhs),
        Err(CatgraphError::CompositionSizeMismatch {
            expected: usize::MAX,
            actual: 2
        })
    ));
    assert!(p.equations().is_empty());
}

// ---- serde ------------------------------------------------------------------

#[cfg(feature = "serde")]
mod serde_boundary {
    use super::{ColoredExpr, Free, Two, W_AB, W_BA};

    #[test]
    fn colored_expr_json_round_trip_is_identity() {
        let expr = Free::tensor(Free::generator(Two::F), Free::generator(Two::G));
        let m = ColoredExpr::new(W_AB.to_vec(), expr).expect("A B → B A");

        let json = serde_json::to_string(&m).expect("serialize");
        let back: ColoredExpr<Two> = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(back, m);
        assert_eq!(back.source_word(), W_AB);
        assert_eq!(back.target_word(), W_BA);
        assert!(back.eq_colored(&m));
    }
}
