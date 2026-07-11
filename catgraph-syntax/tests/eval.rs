//! Interpreter law tests (Phase S3).
//!
//! Covers golden evaluations (braid block-rotation, identity pass-through, the
//! SFG generator shapes, hand-computed Σ_SFG terms), functoriality proptests
//! (compose is pipe, tensor splits/concats), the error paths
//! ([`SyntaxError::WireCount`] and [`SyntaxError::ModelArity`]), and the S3
//! milestone law — the **basis-row cross-check** against the Thm 5.53/5.60
//! matrix functor — plus a parse → eval → matrix end-to-end smoke test.

mod common;

use catgraph_applied::prop::presentation::functorial::{CompleteFunctor, MatrixNFFunctor};
use catgraph_applied::prop::{Free, PropExpr};
use catgraph_applied::sfg::SfgGenerator;
use catgraph_syntax::errors::SyntaxError;
use catgraph_syntax::eval::{ArrowModel, SfgModel, eval};
use catgraph_syntax::text::parse;
use common::{Sig, arb_expr, arb_sfg_leaf_bounded, basis_i64};
use proptest::prelude::*;

/// Shorthand: an SFG generator leaf term over `i64`.
fn leaf(g: SfgGenerator<i64>) -> PropExpr<SfgGenerator<i64>> {
    Free::generator(g)
}

fn model() -> SfgModel<i64> {
    SfgModel::<i64>::new()
}

/// The canonical SFG adapter `p → q`: discard all `p` inputs, then emit `q`
/// zeros. Lets a test compose two independently generated terms with mismatched
/// interfaces into an arity-valid pair *by construction*, so the compose law is
/// exercised without rejection sampling.
fn adapter(p: usize, q: usize) -> PropExpr<SfgGenerator<i64>> {
    let discard_all = (0..p).fold(Free::<SfgGenerator<i64>>::identity(0), |acc, _| {
        Free::tensor(acc, leaf(SfgGenerator::Discard))
    });
    let emit_zeros = (0..q).fold(Free::<SfgGenerator<i64>>::identity(0), |acc, _| {
        Free::tensor(acc, leaf(SfgGenerator::Zero))
    });
    Free::compose(discard_all, emit_zeros).expect("p → 0 → q composes")
}

// ---- Golden evaluations ------------------------------------------------------

#[test]
fn identity_passes_wires_through() {
    let e = Free::<SfgGenerator<i64>>::identity(3);
    assert_eq!(eval(&e, &model(), vec![10, 20, 30]), Ok(vec![10, 20, 30]));
}

#[test]
fn braid_block_rotates() {
    // σ_{2,1} : [a, b | c] ↦ [c | a, b].
    let e = Free::<SfgGenerator<i64>>::braid(2, 1);
    assert_eq!(eval(&e, &model(), vec![10, 20, 30]), Ok(vec![30, 10, 20]));

    // σ_{1,1} is the plain swap.
    let swap = Free::<SfgGenerator<i64>>::braid(1, 1);
    assert_eq!(eval(&swap, &model(), vec![1, 2]), Ok(vec![2, 1]));
}

#[test]
fn generator_shapes() {
    assert_eq!(
        eval(&leaf(SfgGenerator::Copy), &model(), vec![7]),
        Ok(vec![7, 7])
    );
    assert_eq!(
        eval(&leaf(SfgGenerator::Discard), &model(), vec![7]),
        Ok(vec![])
    );
    assert_eq!(
        eval(&leaf(SfgGenerator::Add), &model(), vec![3, 4]),
        Ok(vec![7])
    );
    assert_eq!(
        eval(&leaf(SfgGenerator::Zero), &model(), vec![]),
        Ok(vec![0])
    );
    assert_eq!(
        eval(&leaf(SfgGenerator::Scalar(5)), &model(), vec![3]),
        Ok(vec![15]),
    );
}

#[test]
fn hand_computed_sfg_terms() {
    // copy ; add : 1 → 1 doubles its input (x ↦ [x, x] ↦ [x + x]).
    let double = Free::compose(leaf(SfgGenerator::Copy), leaf(SfgGenerator::Add)).unwrap();
    assert_eq!(eval(&double, &model(), vec![4]), Ok(vec![8]));

    // copy ; (scalar(2) ⊗ scalar(3)) ; add : 1 → 1 computes x ↦ [5x].
    let scaled = Free::tensor(leaf(SfgGenerator::Scalar(2)), leaf(SfgGenerator::Scalar(3)));
    let five_x = Free::compose(
        Free::compose(leaf(SfgGenerator::Copy), scaled).unwrap(),
        leaf(SfgGenerator::Add),
    )
    .unwrap();
    assert_eq!(eval(&five_x, &model(), vec![6]), Ok(vec![30]));
}

// ---- Functoriality proptests -------------------------------------------------

proptest! {
    /// `eval(f ; g, x) == eval(g, eval(f, x))`. The second operand is `f`'s
    /// target adapted into `body`'s interface, so every generated pair composes
    /// by construction (no rejection sampling); the law itself is structural and
    /// holds for any arity-compatible pair.
    #[test]
    fn compose_is_pipe(
        f in arb_expr(arb_sfg_leaf_bounded()),
        body in arb_expr(arb_sfg_leaf_bounded()),
        seed in -3i64..=3,
    ) {
        let g = Free::compose(adapter(f.target(), body.source()), body)
            .expect("adapter target == body source");
        let composed = Free::compose(f.clone(), g.clone()).expect("f target == g source");
        let m = model();
        let x: Vec<i64> = (0..f.source()).map(|k| seed + k as i64).collect();
        let via_composed = eval(&composed, &m, x.clone());
        let via_pipe = eval(&f, &m, x).and_then(|mid| eval(&g, &m, mid));
        prop_assert_eq!(via_composed, via_pipe);
    }

    /// `eval(f ⊗ g, xf ++ xg) == eval(f, xf) ++ eval(g, xg)`.
    #[test]
    fn tensor_splits_and_concats(
        f in arb_expr(arb_sfg_leaf_bounded()),
        g in arb_expr(arb_sfg_leaf_bounded()),
        seed in -3i64..=3,
    ) {
        let m = model();
        let xf: Vec<i64> = (0..f.source()).map(|k| seed + k as i64).collect();
        let xg: Vec<i64> = (0..g.source()).map(|k| seed - k as i64).collect();
        let tensor = Free::tensor(f.clone(), g.clone());
        let mut input = xf.clone();
        input.extend(xg.clone());
        let joined = eval(&tensor, &m, input);
        let expected = match (eval(&f, &m, xf), eval(&g, &m, xg)) {
            (Ok(mut a), Ok(b)) => {
                a.extend(b);
                Ok(a)
            }
            (Err(e), _) | (Ok(_), Err(e)) => Err(e),
        };
        prop_assert_eq!(joined, expected);
    }

    /// The S3 milestone law — the **basis-row cross-check**. For a random SFG
    /// term `e`, feeding the `i`-th standard basis vector (length `e.source()`)
    /// through the interpreter reproduces **row `i`** of the matrix
    /// `MatrixNFFunctor.apply(e)`. Under the Def 5.50 / Remark 5.49 row-vector
    /// convention (`m → n` is an `m × n` matrix, action `x ↦ x · M`), this pins
    /// the interpreter to the Thm 5.53 / 5.60 functor.
    #[test]
    fn eval_matches_matrix_functor_basis_rows(e in arb_expr(arb_sfg_leaf_bounded())) {
        let matrix = MatrixNFFunctor::<i64>::new()
            .apply(&e)
            .expect("SFG term applies to a matrix");
        let m = model();
        for i in 0..e.source() {
            let out = eval(&e, &m, basis_i64(e.source(), i));
            prop_assert_eq!(out, Ok(matrix.entries()[i].clone()));
        }
    }
}

// ---- Error paths -------------------------------------------------------------

#[test]
fn wrong_input_length_is_wire_count() {
    let e = Free::<SfgGenerator<i64>>::identity(2);
    match eval(&e, &model(), vec![1]) {
        Err(SyntaxError::WireCount {
            expected, actual, ..
        }) => {
            assert_eq!((expected, actual), (2, 1));
        }
        other => panic!("expected WireCount, got {other:?}"),
    }
}

/// A deliberately arity-lying model: it always returns zero outputs, regardless
/// of the generator's declared target. `eval` must catch the shape violation.
struct LyingModel;

impl ArrowModel<Sig> for LyingModel {
    type Value = ();

    fn apply_generator(&self, _generator: &Sig, _inputs: Vec<()>) -> Result<Vec<()>, SyntaxError> {
        Ok(vec![])
    }
}

#[test]
fn lying_model_triggers_model_arity() {
    // Sig::Copy is 1 → 2; the input length matches source (1), so the failure
    // is the model's wrong output count (0 ≠ 2), not a WireCount.
    let e = Free::generator(Sig::Copy);
    match eval(&e, &LyingModel, vec![()]) {
        Err(SyntaxError::ModelArity {
            expected, actual, ..
        }) => {
            assert_eq!((expected, actual), (2, 0));
        }
        other => panic!("expected ModelArity, got {other:?}"),
    }
}

// ---- End-to-end smoke: parse → eval → matrix action --------------------------

#[test]
fn parse_eval_matches_matrix_action() {
    // Parse a Σ_SFG term from text, evaluate it, and confirm it agrees with the
    // matrix functor both row-by-row and on a concrete value.
    let e = parse::<SfgGenerator<i64>>("copy ; add").unwrap();
    let matrix = MatrixNFFunctor::<i64>::new().apply(&e).unwrap();
    let m = model();
    for i in 0..e.source() {
        assert_eq!(
            eval(&e, &m, basis_i64(e.source(), i)),
            Ok(matrix.entries()[i].clone()),
        );
    }
    // copy ; add doubles its input.
    assert_eq!(eval(&e, &m, vec![4]), Ok(vec![8]));
}
