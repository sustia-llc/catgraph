//! Error types owned by `catgraph-dl`.
//!
//! The crate was error-type-free until issue
//! [#231](https://github.com/sustia-llc/catgraph/issues/231): every public
//! surface was either total or delegated its failure mode to a caller-supplied
//! closure. The pre-flight recursion guard in [`crate::depth`] is the first
//! surface that must *reject* an input, so it needs a type to reject it with.
//!
//! One type today, [`DepthError`], carrying the same `{ depth, limit }` shape
//! `catgraph-syntax`'s `SyntaxError::RecursionLimit` uses (#99) — the workspace
//! reports "too deep" with one payload, whichever crate does the rejecting.

use thiserror::Error;

/// A depth-recursive `catgraph-dl` entry refused its input **before** walking
/// it, because the input's structural nesting depth exceeds the crate's guard
/// limit.
///
/// This is a pre-flight rejection, never a change of meaning for an accepted
/// input: every input at or below the limit behaves exactly as it did before
/// the guard existed (issue
/// [#231](https://github.com/sustia-llc/catgraph/issues/231), adopting option 2
/// of [#200](https://github.com/sustia-llc/catgraph/issues/200)).
///
/// `#[non_exhaustive]`: the guard is expected to grow — [#200] stays open as
/// the tracker for the recursion this crate cannot cover from its own entries
/// (see the "Scope" section of [`crate::depth`]), and further carriers would
/// add variants here. Match with a wildcard arm from outside the crate.
///
/// [#200]: https://github.com/sustia-llc/catgraph/issues/200
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum DepthError {
    /// A tree-shaped carrier — [`BinaryTree<A>`](crate::free_monad::tree_endo::BinaryTree)
    /// or the `Free<TreeEndo<A>, Infallible>` spine it is isomorphic to — was
    /// deeper than [`MAX_TREE_DEPTH`](crate::depth::MAX_TREE_DEPTH).
    ///
    /// The crate's tree walkers recurse over the carrier's structure, so an
    /// unbounded, *programmatically*-built tree would otherwise risk a **stack
    /// overflow** — an abort, not a catchable error. The guard measures depth
    /// [iteratively](crate::depth::tree_depth), so reporting that a tree is too
    /// deep never overflows on the way to the report.
    #[error("tree nesting depth {depth} exceeds MAX_TREE_DEPTH ({limit})")]
    TreeDepthExceeded {
        /// The carrier's measured structural depth (a leaf has depth `1`).
        depth: usize,
        /// The configured limit, [`MAX_TREE_DEPTH`](crate::depth::MAX_TREE_DEPTH).
        limit: usize,
    },
}
