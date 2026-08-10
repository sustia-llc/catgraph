//! Structural depth of the tree carriers, and the crate's pre-flight recursion
//! guard (issue [#231](https://github.com/sustia-llc/catgraph/issues/231),
//! adopting option 2 of
//! [#200](https://github.com/sustia-llc/catgraph/issues/200)).
//!
//! Three of this crate's entries walk a tree carrier by **recursing** over its
//! structure:
//!
//! - [`tree_to_free_mnd`](crate::free_monad::tree_endo::tree_to_free_mnd) and
//!   [`free_mnd_to_tree`](crate::free_monad::tree_endo::free_mnd_to_tree) — the
//!   CDL Example B.20 bijection witnesses, and
//! - [`RecursiveNn::unroll`](crate::architectures::RecursiveNn::unroll) — the
//!   CDL Example J.3 tree unroller.
//!
//! (The list-shaped siblings do not need a guard: `vec_to_free_mnd` /
//! `free_mnd_to_vec` and `FoldingRnn::unroll` are loops and folds, and
//! `UnfoldingRnn` / `MealyCell` / `MooreCell` are `from_fn` state machines.)
//!
//! Nothing bounds how deep a caller's tree is. A hand-built CDL fixture is
//! depth 3 or 4, but a *programmatic* driver — a genome, a parser, a generator —
//! can produce a degenerate left caterpillar of any depth at all, and one deep
//! enough overflows the stack: an abort, not a catchable error. Each entry
//! therefore pre-flights [`guard_tree_depth`] / [`guard_free_mnd_depth`] against
//! [`MAX_TREE_DEPTH`] before recursing, and returns
//! [`DepthError::TreeDepthExceeded`] instead. This mirrors `catgraph-syntax`'s
//! `MAX_TERM_DEPTH` guard (#99), down to the `{ depth, limit }` payload.
//!
//! The measurement itself is **iterative** (an explicit heap worklist), so
//! guarding an arbitrarily deep carrier never overflows on the way to reporting
//! that it is too deep.
//!
//! # Scope — what the guard does and does not cover
//!
//! The guard covers **this crate's own walker entries**, and only the walk
//! itself. Everything below is now crate-owned — the carriers came in-tree with
//! [#222](https://github.com/sustia-llc/catgraph/issues/222), so an iterative
//! rewrite or a hand-written `Drop` is writable here rather than upstream — but
//! #222 was a port, not a fix, and the guard-at-the-entries posture stands.
//! [#200] stays open as the tracker, and carries the fix design:
//!
//! - **The recursion schemes.** [`Free::fold`] and
//!   [`Cofree::unfold`](crate::endofunctor::Cofree::unfold) recurse over the
//!   spine with no guard in front of them, as the crate's own bench and law
//!   tests reach them directly, and the carriers' recursive `Box` drop glue is
//!   compiler-generated. The carriers' capability-routed `==` and `{:?}`
//!   (their opt-in `PartialEq`/`Debug`) recurse over the same spine, equally
//!   unguarded.
//! - **The concrete tree carrier.** [`BinaryTree`]'s recursion is wider than the
//!   guarded walkers: the drop glue recurses over the same spine when a value
//!   dies — **including a value the guard has just rejected**, since the
//!   fallible entries take it by value — and the derived `Clone` / `PartialEq` /
//!   `Debug` impls recurse identically (`tree.clone()`, `==`, or logging a
//!   suspect tree with `{:?}` can abort where the guarded walk would have
//!   errored).
//!
//! In practice the residuals sit far above the guard: the pre-guard
//! measurement of record (see [`MAX_TREE_DEPTH`]) put the abort threshold in
//! the thousands of frames on a main thread, and the guard rejects at 256.
//!
//! [#200]: https://github.com/sustia-llc/catgraph/issues/200

use core::convert::Infallible;

use crate::endofunctor::{Either, Free};
use crate::errors::DepthError;
use crate::free_monad::tree_endo::{BinaryTree, TreeEndo};

/// Maximum structural nesting depth the crate's tree-recursive entries accept.
///
/// **Why 256.** Deliberately equal to `catgraph-syntax`'s `MAX_TERM_DEPTH`
/// (#99), so the workspace keeps one recursion ceiling by convention — a
/// *prose* convention: the two constants are independent `pub const`s justified
/// differently (syntax budgets heavy interpreter frames on a 2 MiB stack; this
/// one budgets light walker frames with wasm margin), and nothing ties them at
/// compile time. The value is safe by a wide margin rather than by a fine
/// measurement, and the margin is the point — a guard whose limit sits just
/// under the overflow threshold on *one* machine is not a guard on a smaller
/// stack:
///
/// - **The pre-guard measurement of record** (recorded 2026-08-01 with the
///   `benches/free_cofree_shapes.rs` harness, before this guard existed): a
///   4 096-leaf left caterpillar (~4 095 nested frames per walk, in every one
///   of construction, `fold`, `unfold` and drop glue) ran comfortably on the
///   8 MiB stack criterion's main thread gets. 256 sits **16×** below that.
///   Post-guard, the guarded public entries refuse a spine that deep, so the
///   measurement is a historical record, not a reproducible row (the unguarded
///   `fold`/`unfold` remain the way to re-measure the deep regime).
/// - **Rust test threads default to 2 MiB**, a quarter of the measured budget;
///   scaling the measured-safe depth by that ratio leaves roughly a thousand
///   frames, and 256 is a quarter of *that* again. (#99 learned this the hard
///   way from the other side: a 1 024-deep `to_cospan` *did* overflow a 2 MiB
///   test thread, because its frames were heavy. The tree walkers' frames are
///   light — a match, two recursive calls, one moved carrier — but a limit
///   chosen for light frames only would not survive a heavier walker being
///   added later.)
/// - **wasm targets are smaller again.** `wasm32-*` links a shadow stack
///   defaulting to about a megabyte, and an embedder may configure less. 256
///   light frames costs tens of kilobytes there, not hundreds.
///
/// Legitimate CDL-shaped inputs are nowhere near it. The crate's own fixtures
/// top out at depth 4; a *balanced* tree at depth 256 would hold 2²⁵⁵ leaves
/// (leaf depth is 1, so depth `d` holds `2^(d−1)`), and no tree that fits in
/// memory reaches the limit except a near-degenerate spine — which is exactly
/// what [#200] is about.
///
/// [#200]: https://github.com/sustia-llc/catgraph/issues/200
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
/// a [`BinaryTree::Leaf`] counting as depth `1`. Iterative (see the private
/// `depth_by`'s note in the source), so measuring an arbitrarily deep tree
/// never itself overflows.
#[must_use]
pub fn tree_depth<A>(tree: &BinaryTree<A>) -> usize {
    depth_by(tree, |node| match node {
        BinaryTree::Leaf(_) => None,
        BinaryTree::Node(left, right) => Some((left.as_ref(), right.as_ref())),
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
    depth_by(free, |node| match node {
        // `Infallible` has no values; discharged by exhaustion.
        Free::Pure(z) => match *z {},
        Free::Suspend(Either::Left(_)) => None,
        Free::Suspend(Either::Right((left, right))) => Some((left.as_ref(), right.as_ref())),
    })
}

/// Reject `tree` if its structural depth exceeds [`MAX_TREE_DEPTH`], before a
/// walker recurses over it.
///
/// # Errors
///
/// Returns [`DepthError::TreeDepthExceeded`], carrying the measured depth and
/// the limit, if the tree is too deep.
pub fn guard_tree_depth<A>(tree: &BinaryTree<A>) -> Result<(), DepthError> {
    check(tree_depth(tree))
}

/// Reject a `Free<TreeEndo<A>, Infallible>` spine if its structural depth
/// exceeds [`MAX_TREE_DEPTH`], before a walker recurses over it.
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
    //! The guard's at-limit / one-over boundary (exact `{ depth, limit }`
    //! payload included) is pinned integration-side —
    //! `tests/free_monad_bijections.rs::tree_bijection_depth_guard` and
    //! `tests/architecture_unrollers.rs::recursive_nn_depth_guard` — using the
    //! shared `tests/common` spine fixtures, so the boundary and the fixture
    //! builders have exactly one home each.

    use super::*;

    #[test]
    fn depth_counts_the_longest_root_to_leaf_path() {
        // A leaf is depth 1, in both encodings.
        assert_eq!(tree_depth(&BinaryTree::leaf(7_u8)), 1);
        let free_leaf: Free<TreeEndo<u8>, Infallible> = Free::Suspend(Either::Left(0));
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
        let free3: Free<TreeEndo<u8>, Infallible> = Free::Suspend(Either::Right((
            Box::new(Free::Suspend(Either::Right((
                Box::new(Free::Suspend(Either::Left(1))),
                Box::new(Free::Suspend(Either::Left(2))),
            )))),
            Box::new(Free::Suspend(Either::Left(3))),
        )));
        assert_eq!(free_mnd_depth(&free3), 3);
    }
}
