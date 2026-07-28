//! Λ-colored Frobenius layer ([#79](https://github.com/sustia-llc/catgraph/issues/79)
//! P3a, F&S 2019 *Hypergraph Categories*).
//!
//! The palette is `Λ = {A, B}` with `dim(A) = 2`, `dim(B) = 3` (`common::Hue`).
//! Coverage:
//!
//! - colored spider **words** — `Mu(A) : [A, A] → [A]` (Def 2.5 at a colour);
//! - `scfm_equations(c)` is nine equations *per colour* and
//!   `hypergraph_presentation([A, B], …)` seeds 2 × 9 (Lemma 3.10: a Frobenius
//!   structure per `l ∈ Λ` induces the whole hypergraph structure);
//! - colored `to_cospan`: correct apex labelling, colour mismatch rejected, and
//!   the colored `CospanFunctor` deciding a two-colour spider fusion through
//!   `eq_mod_functorial_colored` (Ex 3.12 + Thm 3.14);
//! - non-parallel morphisms are **not** equal even when their images coincide;
//! - colored `to_mat_kron`: per-colour dimensions, mixed-word product
//!   dimensions, colour mismatch, and the reworked word-product overflow guard;
//! - serde round-trip of a colour-carrying `FrobeniusOr`.

mod common;

use catgraph::errors::CatgraphError;
use catgraph_applied::mat_kron::MatKron;
use catgraph_applied::prop::colored::ColoredExpr;
use catgraph_applied::prop::presentation::Presentation;
use catgraph_applied::prop::presentation::functorial::ColoredCompleteFunctor;
use catgraph_applied::prop::{Free, PropExpr, PropSignature};
use catgraph_syntax::cospan_functor::{CospanFunctor, to_cospan};
use catgraph_syntax::errors::SyntaxError;
use catgraph_syntax::frobenius::{
    FrobeniusOr, cap, cup, hypergraph_presentation, scfm_equations, spider, to_mat_kron,
};
use common::{ColoredSig, Hue, hue_dim};

type Term = PropExpr<FrobeniusOr<ColoredSig>>;

fn mk(expr: &Term, word: &[Hue]) -> Result<MatKron<i64>, SyntaxError> {
    to_mat_kron::<ColoredSig, i64, _>(expr, word, &hue_dim)
}

fn colored(word: Vec<Hue>, expr: Term) -> ColoredExpr<FrobeniusOr<ColoredSig>> {
    ColoredExpr::new(word, expr).expect("fixture terms are word-well-formed")
}

// ---- Colored spider words ----------------------------------------------------

#[test]
fn spider_variants_carry_their_colour() {
    // Def 2.5 at colour c: μ_c : [c,c] → [c], η_c : [] → [c], δ_c : [c] → [c,c],
    // ε_c : [c] → []. Different colours are different generators.
    let mu_a = FrobeniusOr::<ColoredSig>::Mu(Hue::A);
    assert_eq!(&*mu_a.source_word(), &[Hue::A, Hue::A]);
    assert_eq!(&*mu_a.target_word(), &[Hue::A]);
    assert_eq!((mu_a.source(), mu_a.target()), (2, 1));

    let eta_b = FrobeniusOr::<ColoredSig>::Eta(Hue::B);
    assert!(eta_b.source_word().is_empty());
    assert_eq!(&*eta_b.target_word(), &[Hue::B]);

    let delta_b = FrobeniusOr::<ColoredSig>::Delta(Hue::B);
    assert_eq!(&*delta_b.source_word(), &[Hue::B]);
    assert_eq!(&*delta_b.target_word(), &[Hue::B, Hue::B]);

    let epsilon_a = FrobeniusOr::<ColoredSig>::Epsilon(Hue::A);
    assert_eq!(&*epsilon_a.source_word(), &[Hue::A]);
    assert!(epsilon_a.target_word().is_empty());

    assert_ne!(mu_a, FrobeniusOr::Mu(Hue::B));

    // The user generator's own colored words pass through unchanged.
    let user = FrobeniusOr::User(ColoredSig::Swap);
    assert_eq!(&*user.source_word(), &[Hue::A, Hue::B]);
    assert_eq!(&*user.target_word(), &[Hue::B, Hue::A]);
}

#[test]
fn spider_builder_threads_its_colour() {
    // spider(B, 3, 2) is a B-coloured junction: source B³, target B².
    let s = spider::<ColoredSig>(Hue::B, 3, 2);
    let checked = colored(vec![Hue::B; 3], s);
    assert_eq!(checked.target_word(), &[Hue::B, Hue::B]);

    // …and it rejects the wrong colour outright.
    let s = spider::<ColoredSig>(Hue::B, 3, 2);
    assert!(ColoredExpr::new(vec![Hue::A; 3], s).is_err());

    // cup/cap at a colour: [] → [A, A] and [A, A] → [].
    assert_eq!(
        colored(Vec::new(), cup::<ColoredSig>(Hue::A)).target_word(),
        &[Hue::A, Hue::A]
    );
    assert!(
        colored(vec![Hue::A, Hue::A], cap::<ColoredSig>(Hue::A))
            .target_word()
            .is_empty()
    );
}

// ---- Nine equations per colour -----------------------------------------------

#[test]
fn scfm_equations_are_nine_at_each_colour() {
    for c in [Hue::A, Hue::B] {
        let laws = scfm_equations::<ColoredSig>(c);
        assert_eq!(
            laws.len(),
            9,
            "Def 2.5 has nine equations (Ex 2.8) at {c:?}"
        );
        // Every pair is parallel over the one-letter source word its spiders
        // force, so an independent `Presentation` accepts all nine.
        let mut pres = Presentation::<FrobeniusOr<ColoredSig>>::new();
        for (lhs, rhs) in laws {
            pres.add_equation(lhs, rhs)
                .expect("scfm_equations are parallel by construction");
        }
        assert_eq!(pres.equations().len(), 9);
    }
}

#[test]
fn hypergraph_presentation_seeds_nine_per_palette_colour() {
    // Lemma 3.10: a Frobenius structure per l ∈ Λ induces the whole hypergraph
    // structure, so the palette's 2 × 9 equations present the colored theory.
    let pres = hypergraph_presentation::<ColoredSig>(
        [Hue::A, Hue::B],
        Vec::<(PropExpr<ColoredSig>, PropExpr<ColoredSig>)>::new(),
    )
    .expect("E_frob is parallel at every colour");
    assert_eq!(pres.equations().len(), 18);

    // User equations are lifted alongside, before the per-colour blocks.
    let user_eq = (
        Free::<ColoredSig>::identity(2),
        Free::<ColoredSig>::identity(2),
    );
    let pres = hypergraph_presentation::<ColoredSig>([Hue::A], [user_eq])
        .expect("a parallel user equation lifts");
    assert_eq!(pres.equations().len(), 10);
}

// ---- Colored to_cospan -------------------------------------------------------

#[test]
fn two_colour_term_maps_to_a_labelled_cospan() {
    // δ_A ⊗ ε_B : [A, B] → [A, A]. Three apex vertices: one A (shared by the
    // δ's input and both its outputs) and one B (hit only by ε's input).
    let term: Term = Free::tensor(
        Free::generator(FrobeniusOr::Delta(Hue::A)),
        Free::generator(FrobeniusOr::Epsilon(Hue::B)),
    );
    let cospan = to_cospan::<ColoredSig>(&term, &[Hue::A, Hue::B]).expect("well-coloured");
    assert_eq!(cospan.middle(), &[Hue::A, Hue::B]);
    assert_eq!(cospan.left_to_middle(), &[0, 1]);
    assert_eq!(cospan.right_to_middle(), &[0, 0]);

    // An identity labels one apex per incoming wire, in order.
    let id2 = to_cospan::<ColoredSig>(
        &Free::<FrobeniusOr<ColoredSig>>::identity(2),
        &[Hue::B, Hue::A],
    )
    .expect("identities are colour-polymorphic");
    assert_eq!(id2.middle(), &[Hue::B, Hue::A]);

    // A braid keeps the incoming labels and permutes the codomain leg.
    let braid = to_cospan::<ColoredSig>(
        &Free::<FrobeniusOr<ColoredSig>>::braid(1, 1),
        &[Hue::A, Hue::B],
    )
    .expect("braids are colour-polymorphic");
    assert_eq!(braid.middle(), &[Hue::A, Hue::B]);
    assert_eq!(braid.right_to_middle(), &[1, 0]);
}

#[test]
fn colour_mismatch_inside_a_term_is_rejected() {
    // η_A ; ε_B — arities meet (1 = 1) but the colours do not.
    let term: Term = Free::compose(
        Free::generator(FrobeniusOr::Eta(Hue::A)),
        Free::generator(FrobeniusOr::Epsilon(Hue::B)),
    )
    .expect("η:0→1 ; ε:1→0");
    let err = to_cospan::<ColoredSig>(&term, &[]).expect_err("A ≠ B at the interface");
    match err {
        CatgraphError::Composition { message } => {
            assert!(message.contains("Epsilon(B)"), "got: {message}");
        }
        other => panic!("expected Composition, got {other:?}"),
    }
}

#[test]
fn colored_functor_decides_a_two_colour_fusion() {
    // Spider fusion at each colour, inside one two-colour term:
    //   (spider(A,2,1) ⊗ spider(B,1,2)) ; … vs the fused form.
    // Concretely: (μ_A ⊗ δ_B) : [A, A, B] → [A, B, B], against the same map
    // built as spider(A,2,1) ⊗ spider(B,1,2) — equal by fusion within a colour,
    // and the functor must decide it.
    let f = CospanFunctor::new();
    let source = vec![Hue::A, Hue::A, Hue::B];

    let generators: Term = Free::tensor(
        Free::generator(FrobeniusOr::Mu(Hue::A)),
        Free::generator(FrobeniusOr::Delta(Hue::B)),
    );
    let spiders: Term = Free::tensor(
        spider::<ColoredSig>(Hue::A, 2, 1),
        spider::<ColoredSig>(Hue::B, 1, 2),
    );

    let a = colored(source.clone(), generators);
    let b = colored(source.clone(), spiders);
    assert_eq!(a.target_word(), &[Hue::A, Hue::B, Hue::B]);

    let pres = hypergraph_presentation::<ColoredSig>(
        [Hue::A, Hue::B],
        Vec::<(PropExpr<ColoredSig>, PropExpr<ColoredSig>)>::new(),
    )
    .expect("E_frob at both colours");
    assert_eq!(
        pres.eq_mod_functorial_colored(&a, &b, &f)
            .expect("the fragment is User-free"),
        Some(true)
    );

    // A genuinely different wiring at the same boundary is separated: replacing
    // μ_A by (id ⊗ ε_A) discards the second A instead of merging it.
    let discard: Term = Free::tensor(
        Free::tensor(
            Free::<FrobeniusOr<ColoredSig>>::identity(1),
            Free::generator(FrobeniusOr::Epsilon(Hue::A)),
        ),
        spider::<ColoredSig>(Hue::B, 1, 2),
    );
    let c = colored(source, discard);
    assert_eq!(
        pres.eq_mod_functorial_colored(&a, &c, &f)
            .expect("still User-free"),
        Some(false)
    );
}

/// A deliberately degenerate colored functor: every term maps to `()`, so image
/// equality on its own would identify *everything*. It exists to pin that
/// `eq_mod_functorial_colored`'s boundary-word test is load-bearing at the trait
/// level, not merely a shortcut that `CospanFunctor` happens to make redundant.
struct CollapseFunctor;

impl ColoredCompleteFunctor<FrobeniusOr<ColoredSig>> for CollapseFunctor {
    type Target = ();

    fn apply_colored(
        &self,
        _expr: &ColoredExpr<FrobeniusOr<ColoredSig>>,
    ) -> Result<(), CatgraphError> {
        Ok(())
    }
}

#[test]
fn non_parallel_morphisms_are_unequal_whatever_the_images_say() {
    let pres = hypergraph_presentation::<ColoredSig>(
        [Hue::A, Hue::B],
        Vec::<(PropExpr<ColoredSig>, PropExpr<ColoredSig>)>::new(),
    )
    .expect("E_frob at both colours");

    let ab = colored(
        vec![Hue::A, Hue::B],
        Free::<FrobeniusOr<ColoredSig>>::identity(2),
    );
    let ba = colored(
        vec![Hue::B, Hue::A],
        Free::<FrobeniusOr<ColoredSig>>::identity(2),
    );

    // With every image equal, only the boundary-word test can separate them.
    assert_eq!(
        pres.eq_mod_functorial_colored(&ab, &ba, &CollapseFunctor)
            .expect("total functor"),
        Some(false),
        "differing boundary words must decide unequal"
    );
    // …while a parallel pair still reaches the functor and takes its verdict.
    assert_eq!(
        pres.eq_mod_functorial_colored(&ab, &ab, &CollapseFunctor)
            .expect("total functor"),
        Some(true)
    );

    // `CospanFunctor` is finer than the contract requires — it labels each
    // boundary index with its colour, so it separates the pair on its own too.
    let f = CospanFunctor::new();
    assert_ne!(
        f.apply_colored(&ab).expect("User-free"),
        f.apply_colored(&ba).expect("User-free")
    );
}

// ---- Colored to_mat_kron -----------------------------------------------------

#[test]
fn per_colour_dimensions() {
    // μ_A lands on MatKron::mu(2); μ_B on MatKron::mu(3).
    let mu_a: Term = Free::generator(FrobeniusOr::Mu(Hue::A));
    assert_eq!(
        mk(&mu_a, &[Hue::A, Hue::A]).expect("well-coloured"),
        MatKron::<i64>::mu(2)
    );
    let mu_b: Term = Free::generator(FrobeniusOr::Mu(Hue::B));
    assert_eq!(
        mk(&mu_b, &[Hue::B, Hue::B]).expect("well-coloured"),
        MatKron::<i64>::mu(3)
    );

    // A mixed-word identity has the PRODUCT dimension 2 · 3 = 6, not a power.
    let id2: Term = Free::<FrobeniusOr<ColoredSig>>::identity(2);
    assert_eq!(
        mk(&id2, &[Hue::A, Hue::B]).expect("colour-polymorphic"),
        MatKron::<i64>::identity(6)
    );

    // The braid's two blocks keep their own dimensions: braiding(2, 3) ≠ (3, 2).
    let braid: Term = Free::<FrobeniusOr<ColoredSig>>::braid(1, 1);
    let got = mk(&braid, &[Hue::A, Hue::B]).expect("colour-polymorphic");
    assert_eq!(got, MatKron::<i64>::braiding(2, 3));
    assert_ne!(got, MatKron::<i64>::braiding(3, 2));
}

#[test]
fn colour_mismatch_is_rejected_by_the_matrix_functor() {
    let mu_a: Term = Free::generator(FrobeniusOr::Mu(Hue::A));
    match mk(&mu_a, &[Hue::A, Hue::B]) {
        Err(SyntaxError::Catgraph(CatgraphError::Composition { message })) => {
            assert!(message.contains("Mu(A)"), "got: {message}");
        }
        other => panic!("expected a colour Composition error, got {other:?}"),
    }
    // Wrong length is the size-mismatch variant, matching `colored::check`.
    assert!(matches!(
        mk(&mu_a, &[Hue::A]),
        Err(SyntaxError::Catgraph(
            CatgraphError::CompositionSizeMismatch {
                expected: 2,
                actual: 1
            }
        ))
    ));
}

#[test]
fn word_product_overflow_still_fires_with_per_colour_dimensions() {
    // A 41-letter B word: 3^40 ≈ 1.2e19 still fits usize (max ≈ 1.8e19) but
    // 3^41 does not, so the word's own product overflows on the last letter and
    // the guard reports exactly that multiplication.
    let id41: Term = Free::<FrobeniusOr<ColoredSig>>::identity(41);
    match mk(&id41, &[Hue::B; 41]) {
        Err(SyntaxError::DimensionOverflow { product, factor }) => {
            assert_eq!(product, 3usize.pow(40));
            assert_eq!(factor, 3);
        }
        other => panic!("expected DimensionOverflow, got {other:?}"),
    }

    // A shorter mixed word whose own product fits but whose rows · cols does not.
    let mut word = vec![Hue::B; 20];
    word.extend(vec![Hue::A; 20]);
    let id40: Term = Free::<FrobeniusOr<ColoredSig>>::identity(40);
    let rows = 3usize.pow(20) * 2usize.pow(20);
    match mk(&id40, &word) {
        Err(SyntaxError::DimensionOverflow { product, factor }) => {
            assert_eq!((product, factor), (rows, rows));
        }
        other => panic!("expected DimensionOverflow, got {other:?}"),
    }
}

// ---- Serde -------------------------------------------------------------------

#[cfg(feature = "serde")]
#[test]
fn colored_frobenius_json_round_trip() {
    let term: Term = Free::tensor(
        Free::generator(FrobeniusOr::Mu(Hue::A)),
        Free::generator(FrobeniusOr::Epsilon(Hue::B)),
    );
    let json = serde_json::to_string(&term).expect("serialize");
    let back: Term = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(
        term, back,
        "a colour payload must survive a JSON round-trip"
    );

    for v in [
        FrobeniusOr::<ColoredSig>::Mu(Hue::A),
        FrobeniusOr::Eta(Hue::B),
        FrobeniusOr::Delta(Hue::A),
        FrobeniusOr::Epsilon(Hue::B),
        FrobeniusOr::User(ColoredSig::Swap),
    ] {
        let json = serde_json::to_string(&v).expect("serialize");
        let back: FrobeniusOr<ColoredSig> = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(v, back);
    }
}
