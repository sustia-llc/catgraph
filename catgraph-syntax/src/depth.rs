//! Structural depth of a term and the shared interpreter recursion guard (#99).
//!
//! The term interpreters ([`eval`](crate::eval::eval),
//! [`to_mat_kron`](crate::frobenius::to_mat_kron), and the Cospan functor's
//! [`to_cospan`](crate::cospan_functor::to_cospan)) recurse over the
//! `Compose`/`Tensor` structure of a term. The [parser](mod@crate::text::parse)
//! already caps *parsed* terms at
//! the [parser](mod@crate::text::parse)'s `MAX_NESTING_DEPTH`, but a term built
//! **programmatically** via `Free::compose`/`Free::tensor` is unbounded, so a
//! pathologically deep term would overflow the stack (an abort, not a catchable
//! error). Each interpreter therefore pre-flights [`guard_term_depth`] (or, for
//! the `CatgraphError`-typed Cospan functor, [`term_depth`] against
//! [`MAX_TERM_DEPTH`] directly) before recursing.
//!
//! The measurement itself is **iterative** (an explicit heap stack), so guarding
//! a deep term never overflows on the way to reporting that it is too deep.
//!
//! # Scope
//!
//! This guard bounds the *interpreters'* recursion. It does **not** address the
//! recursive `Drop` of a deeply-nested [`PropExpr`] value (dropping a
//! 10⁵-deep term recurses in `Drop`); that is an upstream `catgraph-applied`
//! concern, independent of any interpreter.

use catgraph_applied::prop::{PropExpr, PropSignature};

use crate::errors::SyntaxError;

/// Maximum structural nesting depth the term interpreters accept.
///
/// Deliberately equal to the [parser](mod@crate::text::parse)'s
/// `MAX_NESTING_DEPTH`: the interpreters then accept **exactly** the terms the
/// parser can produce, and no more. The
/// value is also the ceiling that keeps the interpreters' *own* recursion safe —
/// their frames (matrix Kronecker products, cospan pushouts) are far heavier than
/// the parser's, so the limit must sit below where that recursion overflows, not
/// merely prevent unbounded depth. 256 heavy frames is comfortably within a
/// small (2 MiB) thread stack; legitimate hand-built terms are nowhere near it.
pub const MAX_TERM_DEPTH: usize = 256;

/// The structural nesting depth of `expr`: the longest root-to-leaf path through
/// `Compose`/`Tensor` nodes (a leaf — `Identity`/`Braid`/`Generator` — has
/// depth `1`).
///
/// Computed **iteratively** with an explicit stack, so measuring an arbitrarily
/// deep term never itself overflows.
#[must_use]
pub fn term_depth<G: PropSignature>(expr: &PropExpr<G>) -> usize {
    let mut stack = vec![(expr, 1usize)];
    let mut max_depth = 0usize;
    while let Some((node, depth)) = stack.pop() {
        max_depth = max_depth.max(depth);
        match node {
            PropExpr::Compose(f, g) | PropExpr::Tensor(f, g) => {
                stack.push((f, depth + 1));
                stack.push((g, depth + 1));
            }
            PropExpr::Identity(_) | PropExpr::Braid(_, _) | PropExpr::Generator(_) => {}
        }
    }
    max_depth
}

/// Reject `expr` if its structural depth exceeds [`MAX_TERM_DEPTH`], before an
/// interpreter recurses over it.
///
/// # Errors
///
/// Returns [`SyntaxError::RecursionLimit`] if the term is too deep.
pub fn guard_term_depth<G: PropSignature>(expr: &PropExpr<G>) -> Result<(), SyntaxError> {
    let depth = term_depth(expr);
    if depth > MAX_TERM_DEPTH {
        return Err(SyntaxError::RecursionLimit {
            depth,
            limit: MAX_TERM_DEPTH,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use catgraph_applied::prop::Free;
    use catgraph_applied::sfg::SfgGenerator;

    type G = SfgGenerator<i64>;

    #[test]
    fn depth_of_leaves_is_one() {
        assert_eq!(term_depth(&Free::<G>::identity(1)), 1);
        assert_eq!(term_depth(&Free::<G>::braid(1, 1)), 1);
        assert_eq!(term_depth(&Free::generator(SfgGenerator::<i64>::Copy)), 1);
    }

    #[test]
    fn depth_counts_the_longest_nesting_path() {
        // id ; (id ; id) — right-nested compose of depth 3.
        let inner = Free::compose(Free::<G>::identity(1), Free::<G>::identity(1)).unwrap();
        let outer = Free::compose(Free::<G>::identity(1), inner).unwrap();
        assert_eq!(term_depth(&outer), 3);

        // A tensor of a depth-3 term with a leaf is depth 4.
        let t = Free::tensor(outer, Free::<G>::identity(1));
        assert_eq!(term_depth(&t), 4);
    }

    #[test]
    fn guard_passes_at_limit_and_fails_just_over() {
        // A left-nested compose chain of exactly MAX_TERM_DEPTH depth passes.
        let at_limit = deep_chain(MAX_TERM_DEPTH);
        assert_eq!(term_depth(&at_limit), MAX_TERM_DEPTH);
        assert!(guard_term_depth(&at_limit).is_ok());

        // One deeper fails with RecursionLimit carrying the measured depth.
        let over = deep_chain(MAX_TERM_DEPTH + 1);
        match guard_term_depth(&over) {
            Err(SyntaxError::RecursionLimit { depth, limit }) => {
                assert_eq!(depth, MAX_TERM_DEPTH + 1);
                assert_eq!(limit, MAX_TERM_DEPTH);
            }
            other => panic!("expected RecursionLimit, got {other:?}"),
        }
    }

    /// A left-nested `id(1) ; id(1) ; … ; id(1)` chain of structural depth `d`
    /// (`d ≥ 1`). Built iteratively so construction does not recurse.
    fn deep_chain(d: usize) -> PropExpr<G> {
        let mut expr = Free::<G>::identity(1);
        for _ in 1..d {
            expr = Free::compose(expr, Free::<G>::identity(1)).unwrap();
        }
        expr
    }
}
