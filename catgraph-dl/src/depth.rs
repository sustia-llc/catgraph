//! Iterative structural depth of the tree carriers ([`tree_depth`] /
//! [`free_mnd_depth`]) and an opt-in ceiling ([`MAX_TREE_DEPTH`],
//! [`guard_tree_depth`] / [`guard_free_mnd_depth`] →
//! [`DepthError::TreeDepthExceeded`]) for callers whose own code walks the
//! carriers recursively. No entry in this crate calls the guard.

use core::convert::Infallible;

use crate::endofunctor::{Either, Free, FreeView};
use crate::errors::DepthError;
use crate::free_monad::tree_endo::{BinaryTree, TreeEndo, TreeView};

/// Recursion ceiling for callers' own recursive walks; equal to
/// `catgraph-syntax`'s `MAX_TERM_DEPTH` by convention, not enforced by any
/// entry in this crate.
pub const MAX_TREE_DEPTH: usize = 256;

/// Shared iterative walk behind both public measures: a DFS worklist, popped
/// from the end, whose peak occupancy is the structural depth plus one pending
/// sibling per level (≈ depth + 1) — **not** a breadth-first level frontier.
/// Pre-sized to `MAX_TREE_DEPTH + 2`, so measuring any carrier the guard
/// *accepts* performs exactly one allocation and never regrows; only a
/// rejected-and-deeper-than-the-limit *balanced* shape can outgrow it, on the
/// error path.
fn depth_by<'a, T>(root: &'a T, children: impl Fn(&'a T) -> Option<(&'a T, &'a T)>) -> usize {
    let mut worklist: Vec<(&'a T, usize)> = Vec::with_capacity(MAX_TREE_DEPTH + 2);
    worklist.push((root, 1));
    let mut max_depth = 0usize;
    while let Some((node, depth)) = worklist.pop() {
        max_depth = max_depth.max(depth);
        if let Some((left, right)) = children(node) {
            worklist.push((left, depth + 1));
            worklist.push((right, depth + 1));
        }
    }
    max_depth
}

/// The structural nesting depth of `tree`: the longest root-to-leaf path, with
/// a [`TreeView::Leaf`] counting as depth `1`. Iterative (see the private
/// `depth_by`'s note in the source), so measuring an arbitrarily deep tree
/// never itself overflows.
#[must_use]
pub fn tree_depth<A>(tree: &BinaryTree<A>) -> usize {
    depth_by(tree, |node| match node.as_view() {
        TreeView::Leaf(_) => None,
        TreeView::Node(children) => Some((&children.0, &children.1)),
    })
}

/// The structural nesting depth of a `Free<TreeEndo<A>, Infallible>` spine —
/// the same measure [`tree_depth`] applies to the [`BinaryTree<A>`] it is
/// isomorphic to, so the bijection preserves depth.
///
/// A leaf cell (`Suspend(Left(a))`) has depth `1`; an internal cell
/// (`Suspend(Right((l, r)))`) is one deeper than its deepest subtree. The
/// `Pure` arm is statically unreachable (`Infallible` has no values) and is
/// discharged by exhaustion, exactly as the bijection helpers do. Iterative,
/// like [`tree_depth`].
#[must_use]
pub fn free_mnd_depth<A>(free: &Free<TreeEndo<A>, Infallible>) -> usize {
    depth_by(free, |node| match node.as_view() {
        // `Infallible` has no values; discharged by exhaustion.
        FreeView::Pure(z) => match *z {},
        FreeView::Suspend(Either::Left(_)) => None,
        FreeView::Suspend(Either::Right((left, right))) => Some((left.as_ref(), right.as_ref())),
    })
}

/// Reject `tree` if its structural depth exceeds [`MAX_TREE_DEPTH`], before
/// **your own** recursive walk descends into it.
///
/// Nothing in this crate calls it: every crate-owned walk is iterative (see the
/// module docs). It is borrowing, so a rejected value is still yours — and its
/// drop is iterative too.
///
/// # Errors
///
/// Returns [`DepthError::TreeDepthExceeded`], carrying the measured depth and
/// the limit, if the tree is too deep.
pub fn guard_tree_depth<A>(tree: &BinaryTree<A>) -> Result<(), DepthError> {
    check(tree_depth(tree))
}

/// Reject a `Free<TreeEndo<A>, Infallible>` spine if its structural depth
/// exceeds [`MAX_TREE_DEPTH`], before **your own** recursive walk descends into
/// it. The [`guard_tree_depth`] sibling for the `Free` encoding; same
/// caller-facing posture.
///
/// # Errors
///
/// Returns [`DepthError::TreeDepthExceeded`], carrying the measured depth and
/// the limit, if the spine is too deep.
pub fn guard_free_mnd_depth<A>(free: &Free<TreeEndo<A>, Infallible>) -> Result<(), DepthError> {
    check(free_mnd_depth(free))
}

/// Shared verdict for both guards: one place where the limit is compared, so
/// the two entries cannot drift apart.
fn check(depth: usize) -> Result<(), DepthError> {
    if depth > MAX_TREE_DEPTH {
        return Err(DepthError::TreeDepthExceeded {
            depth,
            limit: MAX_TREE_DEPTH,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit coverage is the *measure semantics* on tiny hand-built shapes.
    //! The opt-in guard's at-limit / one-over boundary (exact `{ depth, limit }`
    //! payload included) is pinned integration-side —
    //! `tests/free_monad_bijections.rs::opt_in_depth_guard_boundary` — using the
    //! shared `tests/common` spine fixtures, so the boundary and the fixture
    //! builders have exactly one home each.

    use super::*;

    #[test]
    fn depth_counts_the_longest_root_to_leaf_path() {
        // A leaf is depth 1, in both encodings.
        assert_eq!(tree_depth(&BinaryTree::leaf(7_u8)), 1);
        let free_leaf: Free<TreeEndo<u8>, Infallible> = Free::suspend(Either::Left(0));
        assert_eq!(free_mnd_depth(&free_leaf), 1);

        // Node(Node(Leaf, Leaf), Leaf) — the deep branch is the left one.
        let depth3 = BinaryTree::node(
            BinaryTree::node(BinaryTree::leaf(1_u8), BinaryTree::leaf(2_u8)),
            BinaryTree::leaf(3_u8),
        );
        assert_eq!(tree_depth(&depth3), 3);

        // The mirror image measures the same — the walk takes the max, not the
        // left spine.
        let depth3_right = BinaryTree::node(
            BinaryTree::leaf(4_u8),
            BinaryTree::node(BinaryTree::leaf(5_u8), BinaryTree::leaf(6_u8)),
        );
        assert_eq!(tree_depth(&depth3_right), 3);

        // The same depth-3 shape in the `Free` encoding measures identically —
        // the bijection preserves depth.
        let free3: Free<TreeEndo<u8>, Infallible> = Free::suspend(Either::Right((
            Box::new(Free::suspend(Either::Right((
                Box::new(Free::suspend(Either::Left(1))),
                Box::new(Free::suspend(Either::Left(2))),
            )))),
            Box::new(Free::suspend(Either::Left(3))),
        )));
        assert_eq!(free_mnd_depth(&free3), 3);
    }
}
