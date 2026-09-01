//! Frobenius-layer tests (Phase S4, F&S 2019 *Hypergraph Categories*).
//!
//! Covers the S4 milestone law — the Hadamard SCFM on `R^dim` satisfies all
//! **nine** Def 2.5 equations (`to_mat_kron` semantic check, Ex 2.16); each
//! equation's `d=2` `MatKron` image (shared across the slots the paper's
//! algebra makes coincide) plus its own concrete syntax — the #15
//! boundary (each `E_frob` axiom is provable by its own presentation), the spider
//! calculus (fusion, diagonal pattern, the `η;ε` empty spider, cup/cap parity
//! with `MatKron`, the compact-closed snake), the braid shuffle direction, the
//! `to_mat_kron` error paths (`User` out of domain, cell-count overflow on the
//! product paths), the reserved-token shadowing, the textual round-trip over the
//! `FrobeniusOr<Sig>` sum type, and the S3 `eval` interop seam.

mod common;

use catgraph_applied::mat_kron::MatKron;
use catgraph_applied::prop::{Free, PropExpr, mono_word};
use catgraph_syntax::errors::SyntaxError;
use catgraph_syntax::eval::{ArrowModel, eval};
use catgraph_syntax::frobenius::{
    FrobeniusOr, cap, cup, hypergraph_presentation, scfm_equations, spider, to_mat_kron,
};
use catgraph_syntax::text::{GeneratorSyntax, parse, print};
use common::{ShadowSig, Sig, arb_expr, arb_frob_gen, arb_frob_leaf, g};
use proptest::prelude::*;

/// Map a monochromatic Frobenius term to its `MatKron<i64>` image on `R^d`
/// (turbofish shim). The single-colour palette supplies the source word and a
/// constant dimension function.
fn mk(expr: &PropExpr<FrobeniusOr<Sig>>, d: usize) -> MatKron<i64> {
    mk_res(expr, d).expect("Frobenius term maps into MatKron")
}

/// The fallible form of [`mk`], for the error-path tests.
fn mk_res(expr: &PropExpr<FrobeniusOr<Sig>>, d: usize) -> Result<MatKron<i64>, SyntaxError> {
    to_mat_kron::<Sig, i64, _>(expr, &mono_word(expr.source()), &|(): &()| d)
}

// ---- The S4 milestone law: the nine SCFM equations hold semantically ---------

#[test]
fn scfm_nine_laws_hold_in_hadamard_matkron() {
    // Ex 2.16: the Hadamard SCFM on R^d satisfies every Def 2.5 equation. dim ≥ 2
    // (dim = 1 degenerates — every object is 1 — so the check would be vacuous).
    let laws = scfm_equations::<Sig>(());
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

/// `(name, image at d = 2, lhs, rhs)`.
type ScfmGolden = (&'static str, Vec<Vec<i64>>, &'static str, &'static str);

/// Per-axiom goldens for `scfm_equations`, in the order the function returns
/// them (the rustdoc's numbered list).
///
/// The image is the matrix **both** sides map to — that equality is the law —
/// so it fixes each slot only up to semantic equality. Slots 2, 5 and 9 share
/// `identity(2)` and slots 7 and 8 share the `μ ; δ` spider, as the table shows;
/// the two concrete syntaxes fix the terms themselves.
fn scfm_goldens() -> [ScfmGolden; 9] {
    [
        (
            "associativity",
            // c³ → c: 1 exactly where all three legs share a basis index, at
            // row 0 (000) → col 0 and row 7 (111) → col 1.
            vec![
                vec![1, 0],
                vec![0, 0],
                vec![0, 0],
                vec![0, 0],
                vec![0, 0],
                vec![0, 0],
                vec![0, 0],
                vec![0, 1],
            ],
            "mu * id(1) ; mu",
            "id(1) * mu ; mu",
        ),
        (
            "unitality (left)",
            // c → c: the unit is absorbed, leaving identity(2).
            vec![vec![1, 0], vec![0, 1]],
            "eta * id(1) ; mu",
            "id(1)",
        ),
        (
            "commutativity",
            // c² → c: the shuffle is absorbed, leaving mu(2).
            vec![vec![1, 0], vec![0, 0], vec![0, 0], vec![0, 1]],
            "braid(1,1) ; mu",
            "mu",
        ),
        (
            "coassociativity",
            // c → c³: e_i ↦ e_i ⊗ e_i ⊗ e_i, at col 0 and col 7.
            vec![vec![1, 0, 0, 0, 0, 0, 0, 0], vec![0, 0, 0, 0, 0, 0, 0, 1]],
            "delta ; delta * id(1)",
            "delta ; id(1) * delta",
        ),
        (
            "counitality (left)",
            // c → c: the counit is absorbed, leaving identity(2).
            vec![vec![1, 0], vec![0, 1]],
            "delta ; epsilon * id(1)",
            "id(1)",
        ),
        (
            "cocommutativity",
            // c → c²: the shuffle is absorbed, leaving delta(2).
            vec![vec![1, 0, 0, 0], vec![0, 0, 0, 1]],
            "delta ; braid(1,1)",
            "delta",
        ),
        (
            "Frobenius-L",
            // c² → c²: the (2,2) spider — 1 iff both legs share a basis index.
            vec![
                vec![1, 0, 0, 0],
                vec![0, 0, 0, 0],
                vec![0, 0, 0, 0],
                vec![0, 0, 0, 1],
            ],
            "delta * id(1) ; id(1) * mu",
            "mu ; delta",
        ),
        (
            "Frobenius-R",
            // c² → c²: the same (2,2) spider as Frobenius-L — both equal μ ; δ.
            vec![
                vec![1, 0, 0, 0],
                vec![0, 0, 0, 0],
                vec![0, 0, 0, 0],
                vec![0, 0, 0, 1],
            ],
            "id(1) * delta ; mu * id(1)",
            "mu ; delta",
        ),
        (
            "speciality",
            // c → c: the split-then-merge is identity(2).
            vec![vec![1, 0], vec![0, 1]],
            "delta ; mu",
            "id(1)",
        ),
    ]
}

#[test]
fn each_scfm_equation_matches_its_own_golden() {
    let laws = scfm_equations::<Sig>(());
    let goldens = scfm_goldens();
    assert_eq!(
        laws.len(),
        goldens.len(),
        "Def 2.5 has nine equations (Ex 2.8)"
    );
    for (i, ((lhs, rhs), (name, image, lhs_text, rhs_text))) in
        laws.iter().zip(goldens.iter()).enumerate()
    {
        let n = i + 1;
        assert_eq!(print(lhs), *lhs_text, "equation {n} ({name}) lhs syntax");
        assert_eq!(print(rhs), *rhs_text, "equation {n} ({name}) rhs syntax");
        assert_eq!(
            mk(lhs, 2).entries(),
            image.as_slice(),
            "equation {n} ({name}) lhs image at d=2"
        );
        assert_eq!(
            mk(rhs, 2).entries(),
            image.as_slice(),
            "equation {n} ({name}) rhs image at d=2"
        );
    }
}

// ---- The #15 boundary: axioms are provable, completeness is not claimed ------

#[test]
fn e_frob_axioms_prove_themselves() {
    // Each of the nine E_frob equations is an AXIOM of the presentation, so a
    // correct congruence closure must identify its own two sides: eq_mod returns
    // Ok(Some(true)) for every one. This pins axiom-provability — NOT completeness
    // on DERIVED equalities (#15): a derived spider fusion may legitimately get
    // None or Some(false) under the sound-but-incomplete engine, which is why
    // to_mat_kron (the sound semantic checker) carries the fusion laws instead.
    let pres = hypergraph_presentation::<Sig>([()], Vec::<(PropExpr<Sig>, PropExpr<Sig>)>::new())
        .expect("E_frob is parallel by construction");
    for (i, (lhs, rhs)) in scfm_equations::<Sig>(()).into_iter().enumerate() {
        assert_eq!(
            pres.eq_mod(&lhs, &rhs)
                .expect("eq_mod is infallible for this presentation"),
            Some(true),
            "E_frob axiom {} should be provable by its own presentation",
            i + 1
        );
    }
}

// ---- Spider calculus ---------------------------------------------------------

#[test]
fn spiders_reduce_to_the_frobenius_generators() {
    // The one-collapse/one-expand spiders ARE the generators, semantically.
    assert_eq!(mk(&spider::<Sig>((), 2, 1), 3), MatKron::<i64>::mu(3));
    assert_eq!(mk(&spider::<Sig>((), 1, 2), 3), MatKron::<i64>::delta(3));
    assert_eq!(mk(&spider::<Sig>((), 0, 1), 3), MatKron::<i64>::eta(3));
    assert_eq!(mk(&spider::<Sig>((), 1, 0), 3), MatKron::<i64>::epsilon(3));
}

#[test]
fn spider_has_all_equal_diagonal_pattern() {
    // spider(2,2) on R^2 is the 4×4 matrix that is 1 iff all legs share the same
    // basis index: rows/cols encode (a,b) as a*2+b, so only (0,0)→(0,0) and
    // (1,1)→(1,1) are 1 — the SCFM "all legs equal" spider.
    let s = mk(&spider::<Sig>((), 2, 2), 2);
    let expected: Vec<Vec<i64>> = vec![
        vec![1, 0, 0, 0],
        vec![0, 0, 0, 0],
        vec![0, 0, 0, 0],
        vec![0, 0, 0, 1],
    ];
    assert_eq!(s.entries(), expected.as_slice());
}

#[test]
fn spiders_fuse() {
    // spider(m,k) ; spider(k,n) ≡ spider(m,n) for k ≥ 1 (the fusion law).
    for (m, k, n) in [(2, 1, 2), (1, 2, 1), (0, 1, 2), (2, 2, 3), (2, 3, 1)] {
        let fused = Free::compose(spider::<Sig>((), m, k), spider::<Sig>((), k, n))
            .expect("spider(m,k):m→k meets spider(k,n):k→n");
        let direct = spider::<Sig>((), m, n);
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
        g(FrobeniusOr::<Sig>::Eta(())),
        g(FrobeniusOr::<Sig>::Epsilon(())),
    )
    .expect("η:0→1 ; ε:1→0");
    assert_eq!(spider::<Sig>((), 0, 0), expected);
}

#[test]
fn spider_early_returns_drop_the_identity_leg() {
    // m == 1 returns expand(n) directly (top node is δ ; …, NOT a leading
    // collapse(1) = id(1) compose); n == 1 returns collapse(m) directly. Together
    // they subsume the identity: spider(1,1) = expand(1) = the literal id(1).
    match spider::<Sig>((), 1, 2) {
        PropExpr::Compose(left, _) => {
            assert_eq!(
                *left,
                g(FrobeniusOr::<Sig>::Delta(())),
                "spider(1,2) = δ ; (…)"
            );
        }
        other => panic!("spider(1,2) should be a compose δ ; (…), got {other:?}"),
    }
    assert_eq!(
        spider::<Sig>((), 1, 1),
        Free::<FrobeniusOr<Sig>>::identity(1)
    );
}

#[test]
fn cup_and_cap_match_matkron() {
    for d in [2usize, 3] {
        assert_eq!(mk(&cup::<Sig>(()), d), MatKron::<i64>::cup(d));
        assert_eq!(mk(&cap::<Sig>(()), d), MatKron::<i64>::cap(d));
    }
}

#[test]
fn compact_closed_snake() {
    // (cup ⊗ id) ; (id ⊗ cap) ≡ id(1) — the zigzag identity.
    let id1 = Free::<FrobeniusOr<Sig>>::identity(1);
    let snake = Free::compose(
        Free::tensor(cup::<Sig>(()), id1.clone()),
        Free::tensor(id1, cap::<Sig>(())),
    )
    .expect("(cup ⊗ id):1→3 meets (id ⊗ cap):3→1");
    for d in [2usize, 3] {
        assert_eq!(mk(&snake, d), MatKron::<i64>::identity(d), "snake at d={d}");
    }
}

// ---- Braid shuffle direction -------------------------------------------------

#[test]
fn braid_maps_with_correct_shuffle_direction() {
    // Braid(m,n) ↦ braiding(dim^m, dim^n). The asymmetric case pins the argument
    // order: an m/n swap would map to braiding(dim^n, dim^m), a different
    // permutation — so braid(2,1) at dim 2 must be braiding(4,2), and must NOT
    // equal braiding(2,4). Only braid(1,1) is symmetric, so this is the check a
    // swapped mapping would slip past.
    let got = mk(&Free::<FrobeniusOr<Sig>>::braid(2, 1), 2);
    assert_eq!(got, MatKron::<i64>::braiding(4, 2));
    assert_ne!(got, MatKron::<i64>::braiding(2, 4));
}

// ---- to_mat_kron error paths -------------------------------------------------

#[test]
fn user_generator_is_out_of_domain() {
    let e = g(FrobeniusOr::User(Sig::Copy));
    match mk_res(&e, 2) {
        Err(SyntaxError::NonFrobenius { generator }) => {
            assert!(generator.contains("Copy"), "got: {generator}");
        }
        other => panic!("expected NonFrobenius, got {other:?}"),
    }
}

#[test]
fn cell_count_overflow_errors_on_the_product_paths() {
    // A wire of colour c maps to R^dims(c), so an interface WORD maps to the
    // product of its letters' dimensions. The guard reports the multiplication
    // that overflowed: `product` is what had accumulated, `factor` the next term.

    // Identity(100) at dim 2: the source word's own product 2^100 overflows, at
    // the 64th letter (2^63 · 2).
    match mk_res(&Free::<FrobeniusOr<Sig>>::identity(100), 2) {
        Err(SyntaxError::DimensionOverflow { product, factor }) => {
            assert_eq!((product, factor), (1usize << 63, 2));
        }
        other => panic!("Identity(100): expected DimensionOverflow, got {other:?}"),
    }

    // Braid(20,20) at dim 2 ↦ braiding(2^20, 2^20), a (2^20 · 2^20)² = 2^80-cell
    // matrix — the PRODUCT path a per-interface guard would miss: both blocks
    // (2^20) and even the whole 40-wire interface (2^40) fit, but the
    // perfect-shuffle matrix they build does not. The guard fires on the leaf,
    // before braiding allocates, on the rows·cols step.
    match mk_res(&Free::<FrobeniusOr<Sig>>::braid(20, 20), 2) {
        Err(SyntaxError::DimensionOverflow { product, factor }) => {
            assert_eq!((product, factor), (1usize << 40, 1usize << 40));
        }
        other => panic!("Braid(20,20): expected DimensionOverflow, got {other:?}"),
    }

    // Mu at a large dim ↦ mu(dim), a dim² × dim = dim³-cell matrix. At dim = 2^22
    // that is 2^66 cells (> usize) even though the per-wire dim itself fits — the
    // dim² product path the old per-interface guard did not check at all. The
    // guard fires on the leaf before mu allocates.
    let big_dim = 1usize << 22;
    match mk_res(&g(FrobeniusOr::<Sig>::Mu(())), big_dim) {
        Err(SyntaxError::DimensionOverflow { product, factor }) => {
            assert_eq!((product, factor), (1usize << 44, big_dim));
        }
        other => panic!("Mu at dim 2^22: expected DimensionOverflow, got {other:?}"),
    }
}

// ---- Textual round-trip over the sum type ------------------------------------

#[test]
fn parses_mixed_frobenius_and_user_tokens() {
    // Frobenius-only: mu ; delta : 2 → 2.
    let frob = parse::<FrobeniusOr<Sig>>("mu ; delta").expect("mu ; delta parses");
    let expected_frob = Free::compose(g(FrobeniusOr::<Sig>::Mu(())), g(FrobeniusOr::Delta(())))
        .expect("μ:2→1 ; δ:1→2");
    assert_eq!(frob, expected_frob);

    // Mixed: Sig::Copy is 1 → 2 and mu is 2 → 1, so `copy ; mu` composes to 1 → 1.
    let mixed = parse::<FrobeniusOr<Sig>>("copy ; mu").expect("copy ; mu parses");
    let expected_mixed = Free::compose(
        g(FrobeniusOr::User(Sig::Copy)),
        g(FrobeniusOr::<Sig>::Mu(())),
    )
    .expect("copy:1→2 ; μ:2→1");
    assert_eq!(mixed, expected_mixed);
}

#[test]
fn frobenius_names_shadow_user_tokens() {
    // `parse_token` tries the four Frobenius names FIRST, so a user generator
    // spelled `mu` (here ShadowSig) is shadowed: `"mu"` parses to Mu, never
    // User(ShadowSig).
    assert_eq!(
        FrobeniusOr::<ShadowSig>::parse_token("mu"),
        Some(FrobeniusOr::Mu(()))
    );
    // Consequently the clause-1 round-trip breaks for that User generator: it
    // prints `"mu"` but reparses to Mu (the S2 BadSig-style negative check).
    let shadowed = FrobeniusOr::User(ShadowSig);
    assert_eq!(shadowed.print_token(), "mu");
    assert_ne!(
        FrobeniusOr::<ShadowSig>::parse_token(&shadowed.print_token()),
        Some(shadowed)
    );
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
            FrobeniusOr::Mu(()) | FrobeniusOr::User(Sig::Add) => vec![inputs.iter().sum()],
            FrobeniusOr::Eta(()) | FrobeniusOr::User(Sig::Unit) => vec![0],
            FrobeniusOr::Delta(()) | FrobeniusOr::User(Sig::Copy) => {
                let x = inputs[0];
                vec![x, x]
            }
            FrobeniusOr::Epsilon(()) | FrobeniusOr::User(Sig::Counit) => vec![],
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
