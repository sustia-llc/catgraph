//! The binary-tree endofunctor `TreeEndo<A> : X ↦ A + X²`.
//!
//! CDL Example B.20. The free monad on this endofunctor is the binary
//! tree with leaves in `A + Z`:
//!
//! ```text
//! FreeMnd(A + (−)²)(Z) ≅ Tree(A + Z)
//! ```
//!
//! With `Z = !` (the never type, modelled here as `core::convert::Infallible`)
//! the encoding collapses to leaves drawn purely from `A`. With `Z = ()`
//! leaves are `A + ()` — either an actual `A` or a "hole" placeholder.
//!
//! In Rust we encode `A + X²` as [`Either<A, (X, X)>`] from
//! `deep_causality_haft`. The `Left(a)` summand is a tree leaf with payload
//! `a : A`; the `Right((l, r))` summand is an internal node with left/right
//! subtrees.
//!
//! ## Carrier type
//!
//! [`BinaryTree<A>`] is the explicit carrier exposed for ergonomics —
//! constructing a `BinaryTree::Leaf(0)` is friendlier than spelling out
//! `FreeMnd::Roll(Box::new(Either::Left(0)))`. The two helpers
//! [`tree_to_free_mnd`] and [`free_mnd_to_tree`] witness the iso to
//! `FreeMnd<TreeEndo<A>, Infallible>`.
//!
//! ## Iteration discipline
//!
//! Tree walks here are *recursive* — unlike the list helpers, where we
//! avoid stack pressure with a loop. Trees are inherently tree-shaped;
//! recursive walks are the idiomatic choice and tests stay shallow
//! (depth ≤ 3) so stack consumption is bounded.
//!
//! # Why `Infallible`?
//!
//! Rust's `!` (`never_type`) is unstable. `core::convert::Infallible` is
//! the stable inhabitant of the same denotation — there are no values of
//! `Infallible`, so a `FreeMnd<F, Infallible>` cannot have a `Pure` leaf.
//! All leaves must come through `Roll`, i.e. through the `Left(a)`
//! summand of `TreeEndo<A>`.

use core::convert::Infallible;
use core::marker::PhantomData;

use crate::endofunctor::{Either, Functor, HKT, NoConstraint, Satisfies};

use super::free_mnd::FreeMnd;

/// The endofunctor `A + (−)²` for a fixed leaf alphabet `A`.
///
/// The `Type<X>` projection is `Either<A, (X, X)>` — `Left(a)` for a
/// leaf, `Right((l, r))` for an internal node with subtrees `l`, `r`.
///
/// CDL Example B.20. The free monad on this endofunctor is `Tree(A + Z)`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TreeEndo<A>(PhantomData<A>);

impl<A> TreeEndo<A> {
    /// Construct a fresh `TreeEndo<A>` type witness. Zero-sized.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<A> HKT for TreeEndo<A> {
    type Constraint = NoConstraint;
    type Type<X> = Either<A, (X, X)>;
}

impl<A> Functor<Self> for TreeEndo<A> {
    fn fmap<X, Y, Func>(fx: Either<A, (X, X)>, mut f: Func) -> Either<A, (Y, Y)>
    where
        X: Satisfies<NoConstraint>,
        Y: Satisfies<NoConstraint>,
        Func: FnMut(X) -> Y,
    {
        // Identity law: `fmap(Left(a), _) = Left(a)`, `fmap(Right((l, r)),
        // id) = Right((l, r))`. Composition law: tuple-map of `f` then `g`
        // collapses to a single tuple-map of `g ∘ f`. `f` is called twice
        // (once per subtree) — fine under `FnMut`.
        match fx {
            Either::Left(a) => Either::Left(a),
            Either::Right((l, r)) => Either::Right((f(l), f(r))),
        }
    }
}

/// Carrier type for binary trees with leaves in `A`.
///
/// CDL Example B.20. Two constructors:
///
/// - [`BinaryTree::Leaf`] — a leaf labelled by `A`.
/// - [`BinaryTree::Node`] — an internal node with left and right subtrees.
///
/// `Box` indirection on `Node` is required by the standard recursive-type
/// finite-size discipline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryTree<A> {
    /// A leaf labelled by `A`.
    Leaf(A),
    /// An internal node with left and right subtrees.
    Node(Box<BinaryTree<A>>, Box<BinaryTree<A>>),
}

impl<A> BinaryTree<A> {
    /// Build a leaf.
    #[must_use]
    pub fn leaf(a: A) -> Self {
        Self::Leaf(a)
    }

    /// Build an internal node by boxing the supplied subtrees.
    #[must_use]
    pub fn node(left: Self, right: Self) -> Self {
        Self::Node(Box::new(left), Box::new(right))
    }
}

/// Embed a [`BinaryTree<A>`] into the free monad over `TreeEndo<A>`.
///
/// CDL Example B.20. Witnesses the forward direction of the iso
/// `BinaryTree<A> ≅ FreeMnd<TreeEndo<A>, Infallible>`.
///
/// `Infallible` is the stable proxy for the never type `!`. Leaves of
/// the tree become `Roll(Left(a))` cells; internal nodes become
/// `Roll(Right((l', r')))` cells with recursively-embedded subtrees.
/// `Pure` is unreachable — the `Z` slot is `Infallible`.
#[must_use]
pub fn tree_to_free_mnd<A>(tree: BinaryTree<A>) -> FreeMnd<TreeEndo<A>, Infallible> {
    match tree {
        BinaryTree::Leaf(a) => FreeMnd::roll(Either::Left(a)),
        BinaryTree::Node(left, right) => {
            let l = tree_to_free_mnd(*left);
            let r = tree_to_free_mnd(*right);
            FreeMnd::roll(Either::Right((l, r)))
        }
    }
}

/// Project a `FreeMnd<TreeEndo<A>, Infallible>` back to a [`BinaryTree<A>`].
///
/// CDL Example B.20. Inverse of [`tree_to_free_mnd`].
///
/// The `Infallible` terminator means the `Pure` arm is never reachable;
/// if a (presumably user-constructed) value somehow lands on `Pure`,
/// `Infallible` semantics let us discharge the impossible case with the
/// idiomatic `match z {}` exhaustion.
#[must_use]
pub fn free_mnd_to_tree<A>(input: FreeMnd<TreeEndo<A>, Infallible>) -> BinaryTree<A> {
    match input {
        // `Infallible` has no values; this arm is statically unreachable
        // but we discharge it explicitly so the function is total.
        FreeMnd::Pure(z) => match z {},
        FreeMnd::Roll(boxed) => match *boxed {
            Either::Left(a) => BinaryTree::Leaf(a),
            Either::Right((l, r)) => BinaryTree::node(free_mnd_to_tree(l), free_mnd_to_tree(r)),
        },
    }
}
