//! Frobenius-layer tests (Phase S4, F&S 2019 *Hypergraph Categories*).
//!
//! Covers the S4 milestone law — the Hadamard SCFM on `R^dim` satisfies all
//! **nine** Def 2.5 equations (`to_mat_kron` semantic check, Ex 2.16) — the #15
//! soundness boundary (`eq_mod` over `E_frob` never claims a false inequality),
//! the spider calculus (fusion, diagonal pattern, the `η;ε` empty spider, cup/cap
//! parity with `MatKron`, the compact-closed snake), the `to_mat_kron` error
//! paths (`User` out of domain, `dim^k` overflow), the textual round-trip over
//! the `FrobeniusOr<Sig>` sum type, and the S3 `eval` interop seam.

mod common;

use catgraph_applied::mat_kron::MatKron;
use catgraph_applied::prop::{Free, PropExpr};
use catgraph_syntax::errors::SyntaxError;
use catgraph_syntax::eval::{ArrowModel, eval};
use catgraph_syntax::frobenius::{
    FrobeniusOr, cap, cup, hypergraph_presentation, scfm_equations, spider, to_mat_kron,
};
use catgraph_syntax::text::{GeneratorSyntax, parse, print};
use common::{Sig, arb_expr, arb_frob_gen, arb_frob_leaf};
use proptest::prelude::*;

/// Map a Frobenius term to its `MatKron<i64>` image on `R^d` (turbofish shim).
fn mk(expr: &PropExpr<FrobeniusOr<Sig>>, d: usize) -> MatKron<i64> {
    to_mat_kron::<Sig, i64>(expr, d).expect("Frobenius term maps into MatKron")
}

// ---- The S4 milestone law: the nine SCFM equations hold semantically ---------

#[test]
fn scfm_nine_laws_hold_in_hadamard_matkron() {
    // Ex 2.16: the Hadamard SCFM on R^d satisfies every Def 2.5 equation. dim ≥ 2
    // (dim = 1 degenerates — every object is 1 — so the check would be vacuous).
    let laws = scfm_equations::<Sig>();
    assert_eq!(laws.len(), 9, "Def 2.5 has nine equations (Ex 2.8)");
    for (i, (lhs, rhs)) in laws.iter().enumerate() {
        for d in [2usize, 3] {
            assert_eq!(
                mk(lhs, d),
                mk(rhs, d),
                "SCFM equation {} failed at d={d}",
                i + 1
            );
        }
    }
}

// ---- The #15 boundary: sound, syntactically incomplete -----------------------

#[test]
fn scfm_boundary_is_sound_never_false() {
    // eq_mod over E_frob is sound but incomplete (#15): Some(true) is a proof, a
    // None is not a disproof, and Some(false) must NEVER occur (that would be a
    // false inequality claim). We assert the soundness half and record the
    // true-vs-None split for the log (the exact split is engine-internal and is
    // deliberately NOT asserted — pinning it would freeze CC internals).
    let pres = hypergraph_presentation::<Sig>(Vec::<(PropExpr<Sig>, PropExpr<Sig>)>::new())
        .expect("E_frob is arity-matched");
    let mut some_true = 0usize;
    let mut none = 0usize;
    for (lhs, rhs) in scfm_equations::<Sig>() {
        match pres
            .eq_mod(&lhs, &rhs)
            .expect("eq_mod is infallible for this presentation")
        {
            Some(true) => some_true += 1,
            None => none += 1,
            Some(false) => panic!("#15 soundness violated: eq_mod claimed a false inequality"),
        }
    }
    assert_eq!(
        some_true + none,
        9,
        "each equation is classified true-or-None"
    );
    println!("E_frob eq_mod split: Some(true) = {some_true}, None = {none}");
}

// ---- Spider calculus ---------------------------------------------------------

#[test]
fn spiders_reduce_to_the_frobenius_generators() {
    // The one-collapse/one-expand spiders ARE the generators, semantically.
    assert_eq!(mk(&spider::<Sig>(2, 1), 3), MatKron::<i64>::mu(3));
    assert_eq!(mk(&spider::<Sig>(1, 2), 3), MatKron::<i64>::delta(3));
    assert_eq!(mk(&spider::<Sig>(0, 1), 3), MatKron::<i64>::eta(3));
    assert_eq!(mk(&spider::<Sig>(1, 0), 3), MatKron::<i64>::epsilon(3));
}

#[test]
fn spider_has_all_equal_diagonal_pattern() {
    // spider(2,2) on R^2 is the 4×4 matrix that is 1 iff all legs share the same
    // basis index: rows/cols encode (a,b) as a*2+b, so only (0,0)→(0,0) and
    // (1,1)→(1,1) are 1 — the SCFM "all legs equal" spider.
    let s = mk(&spider::<Sig>(2, 2), 2);
    assert_eq!((s.rows(), s.cols()), (4, 4));
    for i in 0..4 {
        for j in 0..4 {
            let expected = i64::from((i == 0 && j == 0) || (i == 3 && j == 3));
            assert_eq!(s.entries()[i][j], expected, "spider(2,2) entry [{i}][{j}]");
        }
    }
}

#[test]
fn spiders_fuse() {
    // spider(m,k) ; spider(k,n) ≡ spider(m,n) for k ≥ 1 (the fusion law).
    for (m, k, n) in [(2, 1, 2), (1, 2, 1), (0, 1, 2), (2, 2, 3), (2, 3, 1)] {
        let fused = Free::compose(spider::<Sig>(m, k), spider::<Sig>(k, n))
            .expect("spider(m,k):m→k meets spider(k,n):k→n");
        let direct = spider::<Sig>(m, n);
        for d in [2usize, 3] {
            assert_eq!(
                mk(&fused, d),
                mk(&direct, d),
                "fusion ({m},{k},{n}) failed at d={d}"
            );
        }
    }
}

#[test]
fn empty_spider_is_eta_then_epsilon() {
    // Design choice: spider(0,0) = η ; ε, by construction (structural equality).
    let expected = Free::compose(
        Free::generator(FrobeniusOr::<Sig>::Eta),
        Free::generator(FrobeniusOr::<Sig>::Epsilon),
    )
    .expect("η:0→1 ; ε:1→0");
    assert_eq!(spider::<Sig>(0, 0), expected);
}

#[test]
fn identity_spider_is_id_one() {
    // Design choice: spider(1,1) is the literal id(1) (the canonical identity spider).
    assert_eq!(spider::<Sig>(1, 1), Free::<FrobeniusOr<Sig>>::identity(1));
}

#[test]
fn cup_and_cap_match_matkron() {
    for d in [2usize, 3] {
        assert_eq!(mk(&cup::<Sig>(), d), MatKron::<i64>::cup(d));
        assert_eq!(mk(&cap::<Sig>(), d), MatKron::<i64>::cap(d));
    }
}

#[test]
fn compact_closed_snake() {
    // (cup ⊗ id) ; (id ⊗ cap) ≡ id(1) — the zigzag identity.
    let id1 = Free::<FrobeniusOr<Sig>>::identity(1);
    let snake = Free::compose(
        Free::tensor(cup::<Sig>(), id1.clone()),
        Free::tensor(id1, cap::<Sig>()),
    )
    .expect("(cup ⊗ id):1→3 meets (id ⊗ cap):3→1");
    for d in [2usize, 3] {
        assert_eq!(mk(&snake, d), MatKron::<i64>::identity(d), "snake at d={d}");
    }
}

// ---- to_mat_kron error paths -------------------------------------------------

#[test]
fn user_generator_is_out_of_domain() {
    let e = Free::generator(FrobeniusOr::User(Sig::Copy));
    match to_mat_kron::<Sig, i64>(&e, 2) {
        Err(SyntaxError::NonFrobenius { generator }) => {
            assert!(generator.contains("Copy"), "got: {generator}");
        }
        other => panic!("expected NonFrobenius, got {other:?}"),
    }
}

#[test]
fn dimension_overflow_errors_cleanly() {
    // Identity(100) ↦ object dim^100; 2^100 overflows usize — a clean error, no
    // panic and no attempt to allocate an astronomically large matrix.
    let e = Free::<FrobeniusOr<Sig>>::identity(100);
    match to_mat_kron::<Sig, i64>(&e, 2) {
        Err(SyntaxError::DimensionOverflow { dim, exponent }) => {
            assert_eq!((dim, exponent), (2, 100));
        }
        other => panic!("expected DimensionOverflow, got {other:?}"),
    }
}

// ---- Textual round-trip over the sum type ------------------------------------

#[test]
fn parses_mixed_frobenius_and_user_tokens() {
    // Frobenius-only: mu ; delta : 2 → 2.
    let frob = parse::<FrobeniusOr<Sig>>("mu ; delta").expect("mu ; delta parses");
    let expected_frob = Free::compose(
        Free::generator(FrobeniusOr::<Sig>::Mu),
        Free::generator(FrobeniusOr::Delta),
    )
    .expect("μ:2→1 ; δ:1→2");
    assert_eq!(frob, expected_frob);

    // Mixed: Sig::Copy is 1 → 2 and mu is 2 → 1, so `copy ; mu` composes to 1 → 1.
    let mixed = parse::<FrobeniusOr<Sig>>("copy ; mu").expect("copy ; mu parses");
    let expected_mixed = Free::compose(
        Free::generator(FrobeniusOr::User(Sig::Copy)),
        Free::generator(FrobeniusOr::<Sig>::Mu),
    )
    .expect("copy:1→2 ; μ:2→1");
    assert_eq!(mixed, expected_mixed);
}

proptest! {
    /// Clause-1 round-trip for every `FrobeniusOr<Sig>` generator (the four
    /// Frobenius names plus every `User(g)`).
    #[test]
    fn frob_generator_clause1_round_trips(generator in arb_frob_gen()) {
        prop_assert_eq!(
            FrobeniusOr::<Sig>::parse_token(&generator.print_token()),
            Some(generator)
        );
    }

    /// Whole-expression round-trip `parse(&print(e)) == Ok(e)` over the sum type
    /// — the S2 printer/parser handle `FrobeniusOr<Sig>` with no change.
    #[test]
    fn frob_whole_expression_round_trips(e in arb_expr(arb_frob_leaf())) {
        prop_assert_eq!(parse::<FrobeniusOr<Sig>>(&print(&e)), Ok(e));
    }
}

// ---- S3 interop: the sum type slots into ArrowModel unchanged ----------------

/// A tiny commutative-monoid-on-`i64` semantics over `FrobeniusOr<Sig>`, proving
/// the sum type is an ordinary signature for the S3 [`eval`] engine (a
/// compile-and-run seam, not new theory). μ / `User(Add)` add; η / `User(Unit)`
/// emit `0`; δ / `User(Copy)` duplicate; ε / `User(Counit)` discard.
struct FrobMonoidModel;

impl ArrowModel<FrobeniusOr<Sig>> for FrobMonoidModel {
    type Value = i64;

    fn apply_generator(
        &self,
        generator: &FrobeniusOr<Sig>,
        inputs: Vec<i64>,
    ) -> Result<Vec<i64>, SyntaxError> {
        Ok(match generator {
            FrobeniusOr::Mu | FrobeniusOr::User(Sig::Add) => vec![inputs.iter().sum()],
            FrobeniusOr::Eta | FrobeniusOr::User(Sig::Unit) => vec![0],
            FrobeniusOr::Delta | FrobeniusOr::User(Sig::Copy) => {
                let x = inputs[0];
                vec![x, x]
            }
            FrobeniusOr::Epsilon | FrobeniusOr::User(Sig::Counit) => vec![],
        })
    }
}

#[test]
fn frobenius_generators_evaluate_under_a_model() {
    let model = FrobMonoidModel;

    // mu ; delta : [3, 4] ↦ μ → [7] ↦ δ → [7, 7].
    let e = parse::<FrobeniusOr<Sig>>("mu ; delta").expect("parses");
    assert_eq!(eval(&e, &model, vec![3, 4]), Ok(vec![7, 7]));

    // copy ; mu : [5] ↦ copy → [5, 5] ↦ μ → [10] (a User generator and a
    // Frobenius one in the same term).
    let mixed = parse::<FrobeniusOr<Sig>>("copy ; mu").expect("parses");
    assert_eq!(eval(&mixed, &model, vec![5]), Ok(vec![10]));
}
