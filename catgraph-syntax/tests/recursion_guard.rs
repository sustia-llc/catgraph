//! The term interpreters guard their recursion against unbounded,
//! programmatically-built terms (#99): an over-deep term is rejected with a
//! catchable error instead of overflowing the stack.

mod common;
use common::{Sig, sfg_model};

use catgraph::errors::CatgraphError;
use catgraph_applied::prop::{Free, PropExpr, PropSignature};
use catgraph_applied::rig::BoolRig;
use catgraph_applied::sfg::SfgGenerator;
use catgraph_syntax::cospan_functor::to_cospan;
use catgraph_syntax::depth::MAX_TERM_DEPTH;
use catgraph_syntax::errors::SyntaxError;
use catgraph_syntax::eval::eval;
use catgraph_syntax::frobenius::{FrobeniusOr, to_mat_kron};

/// A left-nested `id(1) ; id(1) ; … ; id(1)` chain of structural depth `d`,
/// built iteratively so construction (and `Drop` at these modest depths) does
/// not itself recurse deeply. Stays `1 → 1` at every prefix, so it typechecks
/// for every interpreter.
fn deep_id_chain<G: PropSignature>(d: usize) -> PropExpr<G> {
    let mut expr = Free::<G>::identity(1);
    for _ in 1..d {
        expr = Free::compose(expr, Free::<G>::identity(1)).expect("id(1) ; id(1)");
    }
    expr
}

#[test]
fn eval_rejects_over_deep_term() {
    let over: PropExpr<SfgGenerator<i64>> = deep_id_chain(MAX_TERM_DEPTH + 1);
    let err = eval(&over, &sfg_model(), vec![1_i64]).expect_err("must guard, not overflow");
    assert!(matches!(err, SyntaxError::RecursionLimit { limit, .. } if limit == MAX_TERM_DEPTH));

    // A term exactly at the limit still evaluates (guard is not over-strict).
    let at: PropExpr<SfgGenerator<i64>> = deep_id_chain(MAX_TERM_DEPTH);
    assert_eq!(
        eval(&at, &sfg_model(), vec![42_i64]).expect("at-limit is fine"),
        vec![42]
    );
}

#[test]
fn to_mat_kron_rejects_over_deep_term() {
    let over: PropExpr<FrobeniusOr<Sig>> = deep_id_chain(MAX_TERM_DEPTH + 1);
    let err = to_mat_kron::<Sig, BoolRig>(&over, 2).expect_err("must guard, not overflow");
    assert!(matches!(err, SyntaxError::RecursionLimit { depth, limit }
        if depth == MAX_TERM_DEPTH + 1 && limit == MAX_TERM_DEPTH));

    // At the limit it recurses safely (the guard must not sit above the
    // interpreter's own stack-overflow point).
    let at: PropExpr<FrobeniusOr<Sig>> = deep_id_chain(MAX_TERM_DEPTH);
    assert!(to_mat_kron::<Sig, BoolRig>(&at, 2).is_ok());
}

#[test]
fn to_cospan_rejects_over_deep_term() {
    let over: PropExpr<FrobeniusOr<Sig>> = deep_id_chain(MAX_TERM_DEPTH + 1);
    // The Cospan functor reports the guard through the shared
    // `CatgraphError::RecursionLimit` (the `CompleteFunctor` contract fixes the
    // error type to `CatgraphError`, but the variant matches the other
    // interpreters' `SyntaxError::RecursionLimit` shape).
    let err = to_cospan::<Sig>(&over).expect_err("must guard, not overflow");
    match err {
        CatgraphError::RecursionLimit { depth, limit } => {
            assert_eq!(depth, MAX_TERM_DEPTH + 1);
            assert_eq!(limit, MAX_TERM_DEPTH);
        }
        other => panic!("expected RecursionLimit, got {other:?}"),
    }

    // At the limit it succeeds.
    let at: PropExpr<FrobeniusOr<Sig>> = deep_id_chain(MAX_TERM_DEPTH);
    assert!(to_cospan::<Sig>(&at).is_ok());
}
