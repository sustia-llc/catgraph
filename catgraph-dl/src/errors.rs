//! Error types owned by `catgraph-dl`.
//!
//! The crate was error-type-free until issue
//! [#231](https://github.com/sustia-llc/catgraph/issues/231): every public
//! surface was either total or delegated its failure mode to a caller-supplied
//! closure. The recursion guard in [`crate::depth`] is the one surface that
//! *rejects* an input, so it needs a type to reject it with.
//!
//! One type today, [`DepthError`], carrying the same `{ depth, limit }` shape
//! `catgraph-syntax`'s `SyntaxError::RecursionLimit` uses (#99) — the workspace
//! reports "too deep" with one payload, whichever crate does the rejecting.
//! Since v0.14.0 nothing in this crate *returns* it: the guard is opt-in and
//! caller-driven (see [`crate::depth`]).

use thiserror::Error;

/// [`crate::depth`]'s opt-in guard refused a carrier because its structural
/// nesting depth exceeds [`MAX_TREE_DEPTH`](crate::depth::MAX_TREE_DEPTH).
///
/// **No `catgraph-dl` entry returns this any more.** It was the rejection of
/// the #231 pre-flight guard that stood in front of this crate's three
/// tree-recursive entries; [#200] made those walks iterative in the v0.14.0
/// window, so they are infallible and the guard is a service a *caller* invokes
/// before its own recursive walk. The type stays published because that service
/// still needs a vocabulary — the same `{ depth, limit }` payload
/// `catgraph-syntax`'s `SyntaxError::RecursionLimit` uses (#99).
///
/// `#[non_exhaustive]`: further carriers would add variants here. Match with a
/// wildcard arm from outside the crate.
///
/// [#200]: https://github.com/sustia-llc/catgraph/issues/200
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum DepthError {
    /// A tree-shaped carrier — [`BinaryTree<A>`](crate::free_monad::tree_endo::BinaryTree)
    /// or the `Free<TreeEndo<A>, Infallible>` spine it is isomorphic to — was
    /// deeper than [`MAX_TREE_DEPTH`](crate::depth::MAX_TREE_DEPTH).
    ///
    /// A caller whose own walk recurses over the carrier's structure risks a
    /// **stack overflow** on an unbounded, *programmatically*-built tree — an
    /// abort, not a catchable error. The guard measures depth
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
