//! Tests for the v0.5.1 `NormalizeResult` struct semantics.

use catgraph_applied::prop::presentation::Presentation;
use catgraph_applied::prop::{Free, PropExpr, PropSignature};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum G {
    A,
    B,
}

impl PropSignature for G {
    fn source(&self) -> usize {
        1
    }
    fn target(&self) -> usize {
        1
    }
}

fn a() -> PropExpr<G> {
    Free::<G>::generator(G::A)
}
fn b() -> PropExpr<G> {
    Free::<G>::generator(G::B)
}

#[test]
fn normalize_result_converged_on_simple_reduction() {
    let mut p = Presentation::<G>::new();
    p.add_equation(a(), b()).unwrap();
    let result = p.normalize(&a()).unwrap();
    assert!(result.converged, "simple A→B rewrite should converge");
    assert_eq!(result.expr, b());
    // Iteration 1: SMC no-op on A, user equation A→B rewrites A to B. Since
    // B != A the loop updates current=B and continues. Iteration 2: SMC
    // no-op on B, user equation A→B doesn't match (lhs=A, term=B), so
    // after_user == current and the fixpoint check fires. With the
    // post-increment semantics (`step + 1`), steps_taken = 2.
    assert_eq!(result.steps_taken, 2);
}

#[test]
fn normalize_result_hits_bound_on_cycle() {
    // A → A;A grows monotonically (LHS always matches the leftmost A in the
    // RHS), guaranteeing the depth bound is hit regardless of depth parity.
    // Contrast A ↔ B, which converges in 1 iteration because
    // `apply_user_equations` runs both equations sequentially in a single
    // pass.
    let mut p = Presentation::<G>::with_depth(4);
    let a_then_a = Free::<G>::compose(a(), a()).unwrap();
    p.add_equation(a(), a_then_a).unwrap();
    let result = p.normalize(&a()).unwrap();
    assert!(
        !result.converged,
        "A → A;A expansion at depth 4 should hit bound"
    );
    assert_eq!(result.steps_taken, 4);
}

#[test]
fn normalize_result_preserves_original_on_zero_equations() {
    let p = Presentation::<G>::new();
    let result = p.normalize(&a()).unwrap();
    assert!(result.converged);
    assert_eq!(result.expr, a());
    // Iteration 1: SMC no-op + empty user-equation list (no-op) → fixpoint
    // detected immediately. With post-increment semantics, steps_taken = 1.
    assert_eq!(result.steps_taken, 1);
}
