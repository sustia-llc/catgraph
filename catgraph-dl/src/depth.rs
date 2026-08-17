//! Structural depth of the tree carriers, and an **opt-in** recursion guard for
//! callers who walk them recursively themselves (issues
//! [#231](https://github.com/sustia-llc/catgraph/issues/231) and
//! [#200](https://github.com/sustia-llc/catgraph/issues/200)).
//!
//! # This module no longer guards anything inside the crate
//!
//! It once did. Three of this crate's entries —
//! [`tree_to_free_mnd`](crate::free_monad::tree_endo::tree_to_free_mnd),
//! [`free_mnd_to_tree`](crate::free_monad::tree_endo::free_mnd_to_tree) (the CDL
//! Example B.20 bijection witnesses) and
//! [`RecursiveNn::unroll`](crate::architectures::RecursiveNn::unroll) (the CDL
//! Example J.3 tree unroller) — walked a tree carrier by **recursing** over its
//! structure, so a degenerate left caterpillar deep enough overflowed the stack:
//! an abort, not a catchable error. #231 made those three fallible, pre-flighting
//! [`guard_tree_depth`] / [`guard_free_mnd_depth`] against [`MAX_TREE_DEPTH`]
//! and returning [`DepthError::TreeDepthExceeded`] rather than recursing.
//!
//! That was option 2 of #200 — bounding the failure mode rather than removing
//! it. The v0.14.0 window took option 1 instead: **every walk in the crate is
//! now an explicit heap worklist**, including [`Free::fold`],
//! [`Cofree::unfold`](crate::endofunctor::Cofree::unfold), the three entries
//! above, and — the half no pre-flight guard could ever have covered — the
//! carriers' `Drop`, `Clone`, `PartialEq` and `Debug`. Nothing in the crate can
//! overflow on a deep carrier any more, so the three entries are **infallible
//! again** and nothing here is called on their behalf.
//!
//! # What this module is for now
//!
//! Two things, both caller-facing:
//!
//! - [`tree_depth`] / [`free_mnd_depth`] are the plain structural measure of a
//!   carrier — a useful observation in its own right, and iterative, so
//!   measuring an arbitrarily deep carrier never overflows.
//! - [`MAX_TREE_DEPTH`] and the two `guard_*` helpers are a ready-made ceiling
//!   for a caller whose *own* code recurses over these carriers: a hand-written
//!   `match` walk, a `fold` algebra that recurses, a serializer. The crate's own
//!   tests used to write exactly such walks. Applying the guard is now the
//!   caller's decision, and [`crate::errors::DepthError`] is the
//!   vocabulary for the rejection — the same `{ depth, limit }` payload
//!   `catgraph-syntax`'s `MAX_TERM_DEPTH` guard (#99) reports.
//!
//! Because the guards no longer stand in front of a by-value entry, the
//! rejection-path hazard #231 documented is gone too: `guard_tree_depth` takes
//! its argument **by reference**, and dropping the rejected value afterwards is
//! iterative.
//!
//! [#200]: https://github.com/sustia-llc/catgraph/issues/200

use core::convert::Infallible;

use crate::endofunctor::{Either, Free, FreeView};
use crate::errors::DepthError;
use crate::free_monad::tree_endo::{BinaryTree, TreeEndo, TreeView};

/// A recursion ceiling for callers who walk the tree carriers recursively.
///
/// No longer enforced by any entry in this crate (see the module docs): every
/// crate-owned walk is iterative since the v0.14.0 window. It stays published
/// because a caller's own recursive walk still needs a number, and this one is
/// justified below.
///
/// **Why 256.** Deliberately equal to `catgraph-syntax`'s `MAX_TERM_DEPTH`
/// (#99), so the workspace keeps one recursion ceiling by convention — a
/// *prose* convention: the two constants are independent `pub const`s justified
/// differently (syntax budgets heavy interpreter frames on a 2 MiB stack; this
/// one budgets light walker frames with wasm margin), and nothing ties them at
/// compile time. The value is safe by a wide margin rather than by a fine
/// measurement, and the margin is the point — a ceiling that sits just under the
/// overflow threshold on *one* machine is no ceiling on a smaller stack:
///
/// - **The measurements of record.** Recorded 2026-08-01 with the
///   `benches/free_cofree_shapes.rs` harness, while the walks were still
///   recursive: a 4 096-leaf left caterpillar (~4 095 nested frames per walk, in
///   every one of construction, `fold`, `unfold` and drop glue) ran comfortably
///   on the 8 MiB stack criterion's main thread gets. Re-measured 2026-08-16 on
///   a **2 MiB** thread, by reverting each impl in turn: the recursive drop glue
///   survived 8 192 and aborted at 16 384; the recursive `Debug`, whose frames
///   are much fatter, survived 4 096 and aborted at 8 192. 256 sits 16× below
///   the first figure and 32× below the tightest of the second set.
/// - **Rust test threads default to 2 MiB**, a quarter of an 8 MiB main thread —
///   which is exactly the gap the re-measurement above quantifies. (#99 learned
///   it the hard way from the other side: a 1 024-deep `to_cospan` *did*
///   overflow a 2 MiB test thread, because its frames were heavy.) A caller's
///   own walk may have frames heavier than any of the ones measured here, which
///   is the second reason for the margin.
/// - **wasm targets are smaller again.** `wasm32-*` links a shadow stack
///   defaulting to about a megabyte, and an embedder may configure less. 256
///   light frames costs tens of kilobytes there, not hundreds. (This crate's own
///   walks are iterative and so bounded by heap, not by that shadow stack.)
///
/// Legitimate CDL-shaped inputs are nowhere near it. The crate's own fixtures
/// top out at depth 4; a *balanced* tree at depth 256 would hold 2²⁵⁵ leaves
/// (leaf depth is 1, so depth `d` holds `2^(d−1)`), and no tree that fits in
/// memory reaches the limit except a near-degenerate spine — which is exactly
/// what [#200] was about.
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
