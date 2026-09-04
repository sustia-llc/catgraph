//! The syntax claim, end to end.
//!
//! A term of the free prop survives the textual surface, and each of the two
//! semantic fragments agrees with an independently-implemented functor.
//! `parse(print(t)) == t` on a proptest corpus of `PropExpr<FrobeniusOr<Sig>>`
//! and on a right fold printed at the parser's deepest accepted nesting; on the
//! signal-flow fragment `eval` reproduces row `i` of `MatrixNFFunctor`'s matrix
//! from the `i`-th basis vector and both agree with a hand-written generator
//! table (F&S 2018 Thm 5.53 / 5.60); on the Frobenius fragment `CospanFunctor`
//! decides the nine `E_frob` equations equal and separates `braid(1,1)` from
//! `id(2)` (F&S 2019 Prop 3.8, `CospanCanon` the oracle); `to_mat_kron` sends
//! `μ ⊗ η` to a literal `MatKron` golden; `lift_user` maps a braid- and
//! generator-bearing equation to a hand-built lifted pair and refuses a term
//! past `MAX_TERM_DEPTH`; and every `FrobeniusOr` variant survives a JSON round
//! trip.
//!
//! # Input space
//!
//! The round-trip corpus is `arb_expr(arb_frob_leaf())` — the four spiders at
//! the monochromatic colour plus a `User(Sig)` generator, `id(0..=3)` and
//! `braid(0..=2, 0..=2)` leaves, closed under `compose`/`tensor` by
//! `prop_recursive(6, 64, 2)`. The deep round trip is one right fold of `id(1)`
//! legs at each of the two nesting counts either side of the parser bound. The
//! `eval` corpus is `arb_expr(arb_sfg_leaf_bounded())` — the five
//! `SfgGenerator` variants with `Scalar` in `-3..=3` — read over `i64`, and its
//! compose and tensor squares run on ordered pairs drawn from that same
//! strategy. The generator table, the `MatKron` golden, the nine equations, the
//! spider separations and the `lift_user` rows are single fixed terms at one
//! colour; the `to_mat_kron` rows run at `dim = 2` over `i64` and over
//! `BoolRig`. The depth rows are the two terms at `MAX_TERM_DEPTH` and
//! `MAX_TERM_DEPTH + 1`. The serde rows carry `SfgGenerator<i64>` payloads and
//! run only under `--features serde`.
//!
//! # References
//!
//! `eval` is compared against `MatrixNFFunctor` (applied's `sfg_to_mat`, which
//! maps generators to matrices and folds with matmul and block-diagonal) and
//! against a hand-written generator table read off F&S Thm 5.53's row-vector
//! convention. The Frobenius rows are compared against `CospanCanon`, catgraph
//! core's apex-isomorphism invariant, reached through `CospanFunctor::apply`,
//! and against `to_mat_kron`'s Hadamard image; the `MatKron` golden and the
//! lifted equation pair are hand-written values, built from the target
//! constructors rather than from the mapping under test.
//!
//! # Reach
//!
//! The corpus is monochromatic: the colour-annotated token forms and the
//! colored functor entry points are exercised by `tests/colored_text.rs` and
//! `tests/colored_frobenius.rs`. The precedence and error-offset tables live in
//! `tests/parser.rs` and `tests/printer_golden.rs`, the presentation-file
//! surface in `tests/presentation.rs` and `tests/persistence.rs`, the nine
//! `MatKron` equation goldens and the spider algebra in `tests/frobenius.rs`,
//! the interpreter depth guards in `tests/recursion_guard.rs`, and the typed
//! builder and its Arrow algebra in `tests/traced.rs` and
//! `tests/arrow_laws.rs`.
//!
//! # covers:
//!
//! `ArrowModel` `CospanFunctor` `FrobeniusEquation` `FrobeniusOr`
//! `GeneratorSyntax` `SfgModel` `SyntaxError`
//!
//! # not-covered:
//!
//! `Arrow` `ArrowBuilder` `ColorSyntax` `Compose` `Fanout` `First` `Id` `Lift`
//! `PairSwap` `Pretty` `Sealed` `Second` `Split` `ThenFn` `Traced` `Wire`
//! `WireCount` `Wires` `WiresInternal`

mod common;

use catgraph::errors::CatgraphError;
use catgraph_applied::mat::MatR;
use catgraph_applied::mat_kron::MatKron;
use catgraph_applied::prop::presentation::functorial::{CompleteFunctor, MatrixNFFunctor};
use catgraph_applied::prop::{Free, PropExpr, mono_word};
use catgraph_applied::rig::{BoolRig, Checked};
use catgraph_applied::sfg::SfgGenerator;
use catgraph_syntax::cospan_functor::{CospanFunctor, to_cospan};
use catgraph_syntax::depth::{MAX_TERM_DEPTH, term_depth};
use catgraph_syntax::errors::SyntaxError;
use catgraph_syntax::eval::{ArrowModel, SfgModel, eval};
use catgraph_syntax::frobenius::{
    FrobeniusEquation, FrobeniusOr, hypergraph_presentation, lift_user, scfm_equations, to_mat_kron,
};
use catgraph_syntax::text::{MAX_NESTING_DEPTH, parse, print};
use common::sfg_model as model;
use common::{Sig, arb_expr, arb_frob_leaf, arb_sfg_leaf_bounded, basis_i64, g};
use proptest::prelude::*;

/// A Frobenius term over the monochromatic [`Sig`] palette.
type Term = PropExpr<FrobeniusOr<Sig>>;

/// An SFG term over `i64`.
type Sfg = PropExpr<SfgGenerator<i64>>;

fn mu() -> Term {
    Free::generator(FrobeniusOr::Mu(()))
}
fn eta() -> Term {
    Free::generator(FrobeniusOr::Eta(()))
}
fn delta() -> Term {
    Free::generator(FrobeniusOr::Delta(()))
}
fn epsilon() -> Term {
    Free::generator(FrobeniusOr::Epsilon(()))
}
fn id(n: usize) -> Term {
    Free::<FrobeniusOr<Sig>>::identity(n)
}
fn compose(f: Term, h: Term) -> Term {
    Free::compose(f, h).expect("invariant: the fixtures below are arity-matched by construction")
}

/// `to_mat_kron` over `BoolRig` at the monochromatic dimension 2.
fn mk_bool(expr: &Term) -> Result<MatKron<BoolRig>, SyntaxError> {
    to_mat_kron::<Sig, BoolRig, _>(expr, &mono_word(expr.source()), &|(): &()| 2)
}

// ---- Textual round trip ------------------------------------------------------

proptest! {
    /// `parse(print(t)) == t` over the Frobenius corpus.
    #[test]
    fn print_then_parse_is_the_identity_on_the_frobenius_corpus(t in arb_expr(arb_frob_leaf())) {
        prop_assert_eq!(parse::<FrobeniusOr<Sig>>(&print(&t)), Ok(t));
    }
}

/// A right fold `id(1) ; (id(1) ; (… ; id(1)))` of `legs` identity legs. Printed,
/// it carries `legs - 2` nested parentheses for `legs >= 2`, each rendered as the
/// three-byte run `"; ("`.
fn right_fold_identities(legs: usize) -> PropExpr<Sig> {
    let mut expr = Free::<Sig>::identity(1);
    for _ in 1..legs {
        expr = Free::compose(Free::<Sig>::identity(1), expr)
            .expect("invariant: id(1) ; id(1) is arity-matched at every prefix");
    }
    expr
}

/// The round trip at the parser's nesting bound, both sides of it. `parse` enters
/// at depth 0 and rejects a `(` seen at `depth >= MAX_NESTING_DEPTH`, so a text
/// with `MAX_NESTING_DEPTH` nested parentheses is the deepest accepted one and
/// `MAX_NESTING_DEPTH + 1` is a `Parse` error.
#[test]
fn deep_right_fold_round_trips_at_the_parser_bound() {
    let deepest = right_fold_identities(MAX_NESTING_DEPTH + 2);
    let text = print(&deepest);
    assert_eq!(
        text.matches("; (").count(),
        MAX_NESTING_DEPTH,
        "a {}-leg right fold must print {MAX_NESTING_DEPTH} nesting parentheses",
        MAX_NESTING_DEPTH + 2
    );
    assert_eq!(parse::<Sig>(&text), Ok(deepest));

    let over = right_fold_identities(MAX_NESTING_DEPTH + 3);
    let over_text = print(&over);
    assert_eq!(over_text.matches("; (").count(), MAX_NESTING_DEPTH + 1);
    match parse::<Sig>(&over_text) {
        Err(SyntaxError::Parse { message, .. }) => {
            assert!(message.contains("MAX_NESTING_DEPTH"), "got: {message}");
        }
        Ok(_) => panic!(
            "expected a depth-bound Parse error, parsed Ok at {} nesting parentheses (bound {MAX_NESTING_DEPTH})",
            over_text.matches("; (").count()
        ),
        Err(other) => panic!("expected a depth-bound Parse error, got {other:?}"),
    }
}

// ---- Signal-flow fragment: eval vs the Thm 5.53 matrix functor ---------------

/// The hand-written Thm 5.53 generator table: `m → n` is an `m × n` matrix under
/// the Def 5.50 / Remark 5.49 row-vector convention.
fn generator_table() -> Vec<(SfgGenerator<i64>, MatR<i64>)> {
    let m = |rows: usize, cols: usize, entries: Vec<Vec<i64>>| {
        MatR::new(rows, cols, entries).expect("invariant: the table's shapes are written to match")
    };
    vec![
        (SfgGenerator::Copy, m(1, 2, vec![vec![1, 1]])),
        (SfgGenerator::Discard, m(1, 0, vec![vec![]])),
        (SfgGenerator::Add, m(2, 1, vec![vec![1], vec![1]])),
        (SfgGenerator::Zero, m(0, 1, vec![])),
        (SfgGenerator::Scalar(5), m(1, 1, vec![vec![5]])),
    ]
}

/// Each generator's matrix is the hand-written one, and the interpreter's action
/// on the standard basis reproduces that matrix row by row.
#[test]
fn generator_matrices_match_the_hand_anchored_table() {
    let functor = MatrixNFFunctor::<i64>::new();
    for (generator, expected) in generator_table() {
        let term: Sfg = Free::generator(generator.clone());
        let image = functor
            .apply(&term)
            .expect("invariant: a bare SFG generator is in the functor's domain");
        assert_eq!(
            &image, &expected,
            "S({generator:?}) disagrees with the Thm 5.53 table"
        );
        for i in 0..term.source() {
            assert_eq!(
                eval(&term, &model(), basis_i64(term.source(), i)),
                Ok(expected.entries()[i].clone()),
                "eval({generator:?}) on basis row {i} disagrees with the table"
            );
        }
    }
}

proptest! {
    /// Feeding the `i`-th standard basis vector through the interpreter
    /// reproduces row `i` of `MatrixNFFunctor`'s matrix.
    #[test]
    fn eval_matches_the_matrix_functor_on_basis_rows(e in arb_expr(arb_sfg_leaf_bounded())) {
        let matrix = MatrixNFFunctor::<i64>::new()
            .apply(&e)
            .expect("invariant: an SFG term is in the functor's domain");
        let m = model();
        for i in 0..e.source() {
            prop_assert_eq!(eval(&e, &m, basis_i64(e.source(), i)), Ok(matrix.entries()[i].clone()));
        }
    }
}

/// A **value-preserving** SFG adapter `p → q`: it passes the first `min(p, q)`
/// input values through, padding with zeros (`p < q`) or discarding the surplus
/// (`p > q`), so a `Compose` arm that reordered values while preserving counts
/// breaks the compose law. It also makes every generated `(f, adapter ; body)`
/// pair compose by construction.
fn adapter(p: usize, q: usize) -> Sfg {
    if p <= q {
        let zeros = (0..q - p).fold(Free::<SfgGenerator<i64>>::identity(0), |acc, _| {
            Free::tensor(acc, g(SfgGenerator::Zero))
        });
        Free::tensor(Free::<SfgGenerator<i64>>::identity(p), zeros)
    } else {
        let discards = (0..p - q).fold(Free::<SfgGenerator<i64>>::identity(0), |acc, _| {
            Free::tensor(acc, g(SfgGenerator::Discard))
        });
        Free::tensor(Free::<SfgGenerator<i64>>::identity(q), discards)
    }
}

proptest! {
    /// `eval(f ; g, x) == eval(g, eval(f, x))`.
    #[test]
    fn eval_of_a_composite_is_the_pipe(
        f in arb_expr(arb_sfg_leaf_bounded()),
        body in arb_expr(arb_sfg_leaf_bounded()),
        seed in -3i64..=3,
    ) {
        let rhs = Free::compose(adapter(f.target(), body.source()), body)
            .expect("adapter target == body source");
        let composed = Free::compose(f.clone(), rhs.clone()).expect("f target == rhs source");
        let m = model();
        let x: Vec<i64> = (0..f.source()).map(|k| seed + k as i64).collect();
        let via_composed = eval(&composed, &m, x.clone());
        let via_pipe = eval(&f, &m, x).and_then(|mid| eval(&rhs, &m, mid));
        prop_assert_eq!(via_composed, via_pipe);
    }

    /// `eval(f ⊗ g, xf ++ xg) == eval(f, xf) ++ eval(g, xg)`.
    #[test]
    fn eval_of_a_tensor_splits_and_concats(
        f in arb_expr(arb_sfg_leaf_bounded()),
        h in arb_expr(arb_sfg_leaf_bounded()),
        seed in -3i64..=3,
    ) {
        let m = model();
        let xf: Vec<i64> = (0..f.source()).map(|k| seed + k as i64).collect();
        let xh: Vec<i64> = (0..h.source()).map(|k| seed - k as i64).collect();
        let tensor = Free::tensor(f.clone(), h.clone());
        let mut input = xf.clone();
        input.extend(xh.clone());
        let joined = eval(&tensor, &m, input);
        let expected = match (eval(&f, &m, xf), eval(&h, &m, xh)) {
            (Ok(mut a), Ok(b)) => {
                a.extend(b);
                Ok(a)
            }
            (Err(e), _) | (Ok(_), Err(e)) => Err(e),
        };
        prop_assert_eq!(joined, expected);
    }
}

/// The interpreter's action on the structural nodes and on two hand-computed
/// composites.
#[test]
fn eval_golden_terms() {
    let m = model();
    assert_eq!(
        eval(
            &Free::<SfgGenerator<i64>>::identity(3),
            &m,
            vec![10, 20, 30]
        ),
        Ok(vec![10, 20, 30])
    );
    // σ_{2,1} : [a, b | c] ↦ [c | a, b]; σ_{1,1} is the plain swap.
    assert_eq!(
        eval(
            &Free::<SfgGenerator<i64>>::braid(2, 1),
            &m,
            vec![10, 20, 30]
        ),
        Ok(vec![30, 10, 20])
    );
    assert_eq!(
        eval(&Free::<SfgGenerator<i64>>::braid(1, 1), &m, vec![1, 2]),
        Ok(vec![2, 1])
    );
    assert_eq!(eval(&g(SfgGenerator::Copy), &m, vec![7]), Ok(vec![7, 7]));
    assert_eq!(eval(&g(SfgGenerator::Discard), &m, vec![7]), Ok(vec![]));
    assert_eq!(eval(&g(SfgGenerator::Add), &m, vec![3, 4]), Ok(vec![7]));
    assert_eq!(eval(&g(SfgGenerator::Zero), &m, vec![]), Ok(vec![0]));
    assert_eq!(eval(&g(SfgGenerator::Scalar(5)), &m, vec![3]), Ok(vec![15]));

    // copy ; add : 1 → 1 doubles its input.
    let double = compose_sfg(g(SfgGenerator::Copy), g(SfgGenerator::Add));
    assert_eq!(eval(&double, &m, vec![4]), Ok(vec![8]));

    // copy ; (scalar(2) ⊗ scalar(3)) ; add : 1 → 1 computes x ↦ [5x].
    let scaled = Free::tensor(g(SfgGenerator::Scalar(2)), g(SfgGenerator::Scalar(3)));
    let five_x = compose_sfg(
        compose_sfg(g(SfgGenerator::Copy), scaled),
        g(SfgGenerator::Add),
    );
    assert_eq!(eval(&five_x, &m, vec![6]), Ok(vec![30]));
}

fn compose_sfg(f: Sfg, h: Sfg) -> Sfg {
    Free::compose(f, h).expect("invariant: the fixtures above are arity-matched by construction")
}

/// A deliberately arity-lying model: it always returns zero outputs, regardless
/// of the generator's declared target.
struct LyingModel;

impl ArrowModel<Sig> for LyingModel {
    type Value = ();

    fn apply_generator(&self, _generator: &Sig, _inputs: Vec<()>) -> Result<Vec<()>, SyntaxError> {
        Ok(vec![])
    }
}

/// The interpreter's three failure modes: a wrong-length input bundle, a model
/// whose output count disagrees with the generator's target arity, and a
/// directly-constructed `Braid(usize::MAX, 1)` whose width is consumed by the
/// braid arm.
#[test]
fn eval_error_paths() {
    match eval(&Free::<SfgGenerator<i64>>::identity(2), &model(), vec![1]) {
        Err(SyntaxError::WireCount {
            expected, actual, ..
        }) => assert_eq!((expected, actual), (2, 1)),
        other => panic!("expected WireCount, got {other:?}"),
    }

    // Sig::Copy is 1 → 2; the input length matches source (1), so the failure is
    // the model's wrong output count (0 ≠ 2), not a WireCount.
    match eval(&Free::generator(Sig::Copy), &LyingModel, vec![()]) {
        Err(SyntaxError::ModelArity {
            expected, actual, ..
        }) => assert_eq!((expected, actual), (2, 0)),
        other => panic!("expected ModelArity, got {other:?}"),
    }

    // The braid has to sit under a `Compose` for the arm to be reached: `eval`'s
    // entry compares `expr.source()` against the input length first.
    let nested: Sfg = PropExpr::Compose(
        Box::new(Free::<SfgGenerator<i64>>::identity(2)),
        Box::new(PropExpr::Braid(usize::MAX, 1)),
    );
    assert_eq!(nested.source(), 2);
    match eval(&nested, &model(), vec![10, 20]) {
        Err(SyntaxError::WireCount {
            expected,
            actual,
            context,
        }) => assert_eq!((expected, actual, context), (usize::MAX, 2, "braid")),
        other => panic!("expected WireCount, got {other:?}"),
    }
    let bare: Sfg = PropExpr::Braid(usize::MAX, 1);
    match eval(&bare, &model(), vec![10, 20]) {
        Err(SyntaxError::WireCount {
            expected, actual, ..
        }) => assert_eq!((expected, actual), (usize::MAX, 2)),
        other => panic!("expected WireCount, got {other:?}"),
    }
}

/// A term read from text evaluates like the hand-built one, and under
/// `Checked<i64>` an overflowing scalar chain surfaces as `⊥` in the output
/// vector rather than as a silently wrapped value.
#[test]
fn parse_then_eval_and_checked_overflow() {
    let e = parse::<SfgGenerator<i64>>("copy ; add").expect("copy ; add parses");
    assert_eq!(eval(&e, &model(), vec![4]), Ok(vec![8]));

    let over = parse::<SfgGenerator<Checked<i64>>>("scalar_4000000000 ; scalar_4000000000")
        .expect("the scalar tokens parse under Checked<i64>'s FromStr");
    let out = eval(
        &over,
        &SfgModel::<Checked<i64>>::new(),
        vec![Checked::new(1_i64)],
    )
    .expect("1 → 1 term applied to a single wire");
    assert_eq!(out.len(), 1);
    assert!(
        out[0].is_poisoned(),
        "overflow must surface as ⊥, got {:?}",
        out[0]
    );

    let small =
        parse::<SfgGenerator<Checked<i64>>>("scalar_3 ; scalar_5").expect("small scalars parse");
    assert_eq!(
        eval(
            &small,
            &SfgModel::<Checked<i64>>::new(),
            vec![Checked::new(2_i64)]
        ),
        Ok(vec![Checked::new(30_i64)])
    );

    // `⊥` is a single lexical atom, so `scalar_⊥` satisfies the token contract.
    let poisoned = g(SfgGenerator::Scalar(Checked::<i64>::Poison));
    let text = print(&poisoned);
    assert_eq!(text, "scalar_⊥");
    assert_eq!(parse::<SfgGenerator<Checked<i64>>>(&text), Ok(poisoned));
}

// ---- Frobenius fragment: CospanFunctor decides E_frob ------------------------

/// Every one of the nine `E_frob` equations is decided equal by the functor,
/// directly and through the presentation registry, and the functor decides a
/// genuine SCFM equality that the congruence-closure engine does not.
#[test]
fn cospan_functor_decides_the_nine_e_frob_equations() {
    let f = CospanFunctor::new();
    let pres = hypergraph_presentation::<Sig>([()], []).expect("no user equations to lift");
    let equations: Vec<FrobeniusEquation<Sig>> = scfm_equations::<Sig>(());
    assert_eq!(
        equations.len(),
        9,
        "the E_frob presentation must present nine equations"
    );
    assert_eq!(
        pres.equations().len(),
        9,
        "hypergraph_presentation must install the nine E_frob equations"
    );
    for (lhs, rhs) in &equations {
        let fa = f.apply(lhs).expect("the spider fragment is User-free");
        let fb = f.apply(rhs).expect("the spider fragment is User-free");
        assert_eq!(fa, fb, "the functor failed to equate an E_frob equation");
        assert_eq!(
            pres.eq_mod_functorial(lhs, rhs, &f)
                .expect("the functor applies on User-free terms"),
            Some(true),
        );
        // Sound agreement: the independently-implemented Prop 3.8 checker
        // equates both sides too.
        assert_eq!(
            mk_bool(lhs).expect("User-free"),
            mk_bool(rhs).expect("User-free"),
            "to_mat_kron disagrees with an E_frob equation"
        );
    }

    // A two-legged bubble collapsing to the one-legged one — `η ; δ ; (ε ⊗ ε) =
    // η ; ε` — holds by the counit law under the `η ; – ; ε` context. The
    // syntactic engine returns no definite `Some(true)`; the complete functor
    // does.
    let a = compose(compose(eta(), delta()), Free::tensor(epsilon(), epsilon()));
    let b = compose(eta(), epsilon());
    assert_ne!(
        pres.eq_mod(&a, &b).expect("eq_mod runs"),
        Some(true),
        "if CC now decides this, pick a harder witness — the test must show a gap",
    );
    assert_eq!(
        pres.eq_mod_functorial(&a, &b, &f).expect("functor applies"),
        Some(true)
    );
}

/// The functor separates terms the nine equations do not identify: the braid
/// from the identity at the same boundary shape (the pairing of domain to
/// codomain legs, not their counts), two different spiders, the merge from a
/// discard, and the closed bubble from `id₀` — a scalar `to_mat_kron` loses over
/// an idempotent rig.
#[test]
fn cospan_functor_separates_what_e_frob_does_not_identify() {
    let f = CospanFunctor::new();

    // `braid(1,1)` and `id(2)` agree on every apex-vertex count and on both
    // boundary sizes; only the domain-to-codomain pairing tells them apart.
    let braid = Free::<FrobeniusOr<Sig>>::braid(1, 1);
    let braid_image = f.apply(&braid).expect("User-free");
    let id2_image = f.apply(&id(2)).expect("User-free");
    assert_eq!(
        (braid_image.dom_len(), braid_image.cod_len()),
        (id2_image.dom_len(), id2_image.cod_len()),
        "the two witnesses must share a boundary shape",
    );
    assert_eq!(
        braid_image.apex_len(),
        id2_image.apex_len(),
        "the two witnesses must share an apex-vertex count",
    );
    assert_ne!(
        braid_image,
        id2_image,
        "braid(1,1) and id(2) must not be identified: {:?} vs {:?}",
        braid_image.classes(),
        id2_image.classes(),
    );

    // Different boundary shape, and different wiring at a fixed boundary.
    assert_ne!(
        f.apply(&mu()).expect("User-free"),
        f.apply(&delta()).expect("User-free")
    );
    let discard_right = Free::tensor(id(1), epsilon());
    assert_ne!(
        f.apply(&mu()).expect("User-free"),
        f.apply(&discard_right).expect("User-free")
    );

    // Scalars are kept: Cospan is special, not extra-special.
    let bubble = compose(eta(), epsilon());
    let two_bubbles = Free::tensor(bubble.clone(), bubble.clone());
    let bubble_image = f.apply(&bubble).expect("User-free");
    assert_eq!(bubble_image.scalar_count(), 1);
    assert_eq!(f.apply(&id(0)).expect("User-free").scalar_count(), 0);
    assert_eq!(f.apply(&two_bubbles).expect("User-free").scalar_count(), 2);
    assert_ne!(
        bubble_image,
        f.apply(&id(0)).expect("User-free"),
        "η;ε must not equal id₀"
    );
    assert_ne!(
        bubble_image,
        f.apply(&two_bubbles).expect("User-free"),
        "one bubble ≠ two bubbles"
    );

    // Over BoolRig the same scalar is invisible to to_mat_kron — the cospan
    // target is strictly finer.
    assert_eq!(
        mk_bool(&bubble).expect("User-free"),
        mk_bool(&id(0)).expect("User-free"),
        "BoolRig is idempotent, so the bubble scalar is invisible to to_mat_kron"
    );
}

/// The fragment boundary: a `User` generator has no cospan image, bare or
/// buried, and an interface mismatch inside a `Compose` is caught by the
/// top-down word pass before the pushout runs.
#[test]
fn cospan_functor_rejects_terms_outside_the_fragment() {
    let f = CospanFunctor::new();
    let term: Term = Free::generator(FrobeniusOr::User(Sig::Copy));
    let err = f
        .apply(&term)
        .expect_err("User generators must be rejected");
    assert!(matches!(err, CatgraphError::Presentation { .. }), "{err:?}");

    let buried = Free::tensor(mu(), Free::generator(FrobeniusOr::User(Sig::Add)));
    assert!(to_cospan::<Sig>(&buried, &mono_word(buried.source())).is_err());

    // η : 0 → 1 composed with μ : 2 → 1 — interface 1 ≠ 2.
    let bad = PropExpr::Compose(Box::new(eta()), Box::new(mu()));
    let err = to_cospan::<Sig>(&bad, &mono_word(bad.source())).expect_err("interface mismatch");
    assert!(
        matches!(
            err,
            CatgraphError::CompositionSizeMismatch {
                expected: 2,
                actual: 1
            }
        ),
        "got: {err:?}"
    );
}

/// `to_mat_kron`'s `Tensor` arm is `left ⊗ right`, in that order: `μ ⊗ η` at
/// `dim = 2` is the literal 4×4 matrix below, whose entries differ from the
/// swapped product `η ⊗ μ`.
#[test]
fn mat_kron_tensor_takes_its_factors_left_then_right() {
    let term = Free::tensor(mu(), eta());
    let image = to_mat_kron::<Sig, i64, _>(&term, &mono_word(term.source()), &|(): &()| 2)
        .expect("μ ⊗ η is User-free");
    let expected = vec![
        vec![1, 1, 0, 0],
        vec![0, 0, 0, 0],
        vec![0, 0, 0, 0],
        vec![0, 0, 1, 1],
    ];
    assert_eq!(
        (image.rows(), image.cols()),
        (4, 4),
        "μ ⊗ η at dim 2 is a 4×4 matrix"
    );
    assert_eq!(
        image.entries(),
        expected.as_slice(),
        "μ ⊗ η disagrees with the golden; the swapped product η ⊗ μ is \
         [[1,0,1,0],[0,0,0,0],[0,0,0,0],[0,1,0,1]]"
    );
    // The swapped product is a different matrix at this shape, so the golden
    // above separates the two factor orders rather than holding for both.
    assert_ne!(
        MatKron::<i64>::eta(2)
            .kron(&MatKron::<i64>::mu(2))
            .entries(),
        expected.as_slice(),
    );
}

// ---- lift_user ---------------------------------------------------------------

/// The user-term inclusion `Free(G) ↪ Free(FrobeniusOr<G>)` is structural and
/// arity-preserving, and `hypergraph_presentation` carries both sides of a user
/// equation through it.
#[test]
fn lift_user_maps_a_braid_bearing_equation_structurally() {
    // lhs = (copy ⊗ id(1)) ; braid(1,2), rhs = id(1) ⊗ copy — both 2 → 3,
    // non-reflexive, and the braid's two widths differ.
    let lhs: PropExpr<Sig> = Free::compose(
        Free::tensor(g(Sig::Copy), Free::<Sig>::identity(1)),
        Free::<Sig>::braid(1, 2),
    )
    .expect("invariant: (copy ⊗ id(1)) : 2 → 3 meets braid(1,2) : 3 → 3");
    let rhs: PropExpr<Sig> = Free::tensor(Free::<Sig>::identity(1), g(Sig::Copy));

    let user = |s: Sig| -> Term { Free::generator(FrobeniusOr::User(s)) };
    let expected_lhs = compose(
        Free::tensor(user(Sig::Copy), id(1)),
        Free::<FrobeniusOr<Sig>>::braid(1, 2),
    );
    let expected_rhs = Free::tensor(id(1), user(Sig::Copy));
    assert_ne!(expected_lhs, expected_rhs, "the pair must be non-reflexive");

    assert_eq!(
        lift_user(lhs.clone()).expect("the lhs is arity-sound"),
        expected_lhs
    );
    assert_eq!(
        lift_user(rhs.clone()).expect("the rhs is arity-sound"),
        expected_rhs
    );

    // Both sides reach the presentation, in that order.
    let pres =
        hypergraph_presentation::<Sig>([()], [(lhs, rhs)]).expect("the user equation is parallel");
    assert_eq!(
        pres.equations()[0],
        (expected_lhs, expected_rhs),
        "the user equation must be lifted side by side"
    );
}

/// `lift_user` pre-flights the shared depth guard, so a programmatically-built
/// term past `MAX_TERM_DEPTH` is refused rather than recursed over.
#[test]
fn lift_user_guards_its_recursion_depth() {
    let deep = |d: usize| {
        let mut expr = Free::<Sig>::identity(1);
        for _ in 1..d {
            expr = Free::compose(expr, Free::<Sig>::identity(1)).expect("id(1) ; id(1)");
        }
        expr
    };
    match lift_user(deep(MAX_TERM_DEPTH + 1)) {
        Err(SyntaxError::RecursionLimit { depth, limit }) => {
            assert_eq!((depth, limit), (MAX_TERM_DEPTH + 1, MAX_TERM_DEPTH));
        }
        Ok(lifted) => panic!(
            "expected RecursionLimit, lifted a term of depth {} (limit {MAX_TERM_DEPTH})",
            term_depth(&lifted)
        ),
        Err(other) => panic!("expected RecursionLimit, got {other:?}"),
    }
    assert!(
        lift_user(deep(MAX_TERM_DEPTH)).is_ok(),
        "a term at the limit must still lift"
    );
}

// ---- serde ------------------------------------------------------------------

/// The variant's own name, through an exhaustive `match` — a new `FrobeniusOr`
/// variant is a compile error here, so the witness list below cannot silently
/// fall behind the enum.
#[cfg(feature = "serde")]
fn variant_name<G: catgraph_applied::prop::PropSignature>(v: &FrobeniusOr<G>) -> &'static str {
    match v {
        FrobeniusOr::Mu(_) => "Mu",
        FrobeniusOr::Eta(_) => "Eta",
        FrobeniusOr::Delta(_) => "Delta",
        FrobeniusOr::Epsilon(_) => "Epsilon",
        FrobeniusOr::User(_) => "User",
    }
}

/// Every `FrobeniusOr` variant, and a term mixing spiders with a payload-bearing
/// `User` generator, survive a JSON round trip.
#[cfg(feature = "serde")]
#[test]
fn frobenius_terms_and_every_variant_round_trip_through_json() {
    use std::collections::BTreeSet;

    type Gen = FrobeniusOr<SfgGenerator<i64>>;

    let witnesses: Vec<Gen> = vec![
        FrobeniusOr::Mu(()),
        FrobeniusOr::Eta(()),
        FrobeniusOr::Delta(()),
        FrobeniusOr::Epsilon(()),
        FrobeniusOr::User(SfgGenerator::Add),
    ];
    let names: BTreeSet<&'static str> = witnesses.iter().map(variant_name).collect();
    assert_eq!(
        names.len(),
        witnesses.len(),
        "each witness must name a distinct variant, got {names:?}"
    );
    for v in &witnesses {
        let json = serde_json::to_string(v).expect("serialize");
        let back: Gen = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(
            v,
            &back,
            "{} did not survive the round trip",
            variant_name(v)
        );
    }

    // η ; δ tensored with a `User` scalar — 1 → 3.
    let left = Free::compose(
        Free::generator(FrobeniusOr::Eta(())),
        Free::generator(FrobeniusOr::Delta(())),
    )
    .expect("eta(0→1) ; delta(1→2)");
    let term: PropExpr<Gen> = Free::tensor(
        left,
        Free::generator(FrobeniusOr::User(SfgGenerator::Scalar(7))),
    );
    let json = serde_json::to_string(&term).expect("serialize");
    let back: PropExpr<Gen> = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(
        term, back,
        "a FrobeniusOr term must survive a JSON round trip"
    );
}
