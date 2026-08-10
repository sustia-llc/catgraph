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
//! `Free::Suspend(Either::Left(0))`. The two helpers
//! [`tree_to_free_mnd`] and [`free_mnd_to_tree`] witness the iso to
//! `Free<TreeEndo<A>, Infallible>`.
//!
//! ## Iteration discipline
//!
//! Tree walks here are *recursive* — unlike the list helpers, where we
//! avoid stack pressure with a loop. Trees are inherently tree-shaped;
//! recursive walks are the idiomatic choice.
//!
//! Bounded stack consumption is no longer left to the tests staying shallow:
//! both helpers pre-flight [`crate::depth`]'s
//! [`MAX_TREE_DEPTH`](crate::depth::MAX_TREE_DEPTH) guard and return
//! [`DepthError`] rather than recursing over a carrier deep enough to overflow
//! (issue [#231](https://github.com/sustia-llc/catgraph/issues/231)). The
//! private `*_inner` walkers below run *after* the guard, so they are bounded
//! by construction. See [`crate::depth`]'s "Scope" section for what the guard
//! does **not** cover — direct `deep_causality_haft` `Free::fold` calls, and the
//! recursive drop glue of the `Box`-nested carriers themselves.
//!
//! # Why `Infallible`?
//!
//! Rust's `!` (`never_type`) is unstable. `core::convert::Infallible` is
//! the stable inhabitant of the same denotation — there are no values of
//! `Infallible`, so a `Free<F, Infallible>` cannot have a `Pure` leaf.
//! All leaves must come through `Suspend`, i.e. through the `Left(a)`
//! summand of `TreeEndo<A>`.

use core::convert::Infallible;
use core::marker::PhantomData;

use crate::container::Container;
use crate::depth::{guard_free_mnd_depth, guard_tree_depth};
use crate::endofunctor::{
    DebugFunctor, Either, EqFunctor, Free, Functor, HKT, NoConstraint, Satisfies,
};
use crate::errors::DepthError;

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

// Opt-in structural equality for `Free<TreeEndo<A>, Z>`: route the comparison of
// the functor hole `Either<A, (T, T)>` through haft `Either`'s derived `==`
// (which derives `PartialEq`). Bounded `A: PartialEq`; `T: PartialEq` from the
// trait method.
impl<A: PartialEq> EqFunctor for TreeEndo<A> {
    fn eq_type<T: PartialEq>(a: &Either<A, (T, T)>, b: &Either<A, (T, T)>) -> bool {
        a == b
    }
}

// Opt-in `Debug` for `Free<TreeEndo<A>, Z>`: delegate the functor hole to
// haft `Either`'s derived `Debug`. Bounded `A: Debug`.
impl<A: core::fmt::Debug> DebugFunctor for TreeEndo<A> {
    fn fmt_type<T: core::fmt::Debug>(
        fa: &Either<A, (T, T)>,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        core::fmt::Debug::fmt(fa, f)
    }
}

/// Container presentation of `A + (−)²` (Abbott–Altenkirch–Ghani 2003, via
/// CDL). Shapes are `Either<A, ()>`: the leaf summand `Left(a)` (arity 0 — a
/// leaf carries its label but no recursive slot) and the node summand
/// `Right(())` (arity 2 — the two subtree slots). `A: PartialEq + Debug` so the
/// shape carries into the machine-checked container laws.
impl<A: PartialEq + core::fmt::Debug> Container for TreeEndo<A> {
    type Shape = Either<A, ()>;

    fn arity(shape: &Self::Shape) -> usize {
        match shape {
            Either::Left(_) => 0,
            Either::Right(()) => 2,
        }
    }

    fn decompose<X>(fx: Either<A, (X, X)>) -> (Self::Shape, Vec<X>) {
        match fx {
            Either::Left(a) => (Either::Left(a), Vec::new()),
            Either::Right((l, r)) => (Either::Right(()), vec![l, r]),
        }
    }

    fn recompose<X>(shape: Self::Shape, contents: Vec<X>) -> Option<Either<A, (X, X)>> {
        match shape {
            // Leaf shape (arity 0): reconstruct iff no contents were supplied.
            Either::Left(a) => contents.is_empty().then_some(Either::Left(a)),
            // Node shape (arity 2): `TryFrom<Vec<X>> for [X; 2]` rejects any
            // other length.
            Either::Right(()) => {
                let [l, r] = <[X; 2]>::try_from(contents).ok()?;
                Some(Either::Right((l, r)))
            }
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
///
/// The derived `Clone` / `PartialEq` / `Debug` — and the compiler's drop
/// glue — recurse over the same `Box` spine the #231 guard bounds for the
/// walker entries, and sit **outside** that guard: cloning, comparing, or
/// `{:?}`-logging an over-deep tree can abort where the guarded walk would
/// have errored. This is a crate-owned residual (iterative impls are writable
/// here without touching haft); [`crate::depth`]'s Scope section and #200
/// record it.
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
/// `BinaryTree<A> ≅ Free<TreeEndo<A>, Infallible>`.
///
/// `Infallible` is the stable proxy for the never type `!`. Leaves of
/// the tree become `Suspend(Left(a))` cells; internal nodes become
/// `Suspend(Right((l', r')))` cells with recursively-embedded (boxed)
/// subtrees. `Pure` is unreachable — the `Z` slot is `Infallible`.
///
/// # Errors
///
/// Returns [`DepthError::TreeDepthExceeded`] if `tree` is deeper than
/// [`MAX_TREE_DEPTH`](crate::depth::MAX_TREE_DEPTH) — the pre-flight recursion
/// guard (#231). The *measurement* is iterative and cannot overflow; note the
/// rejected `tree` is still consumed and dropped here, and [`BinaryTree`]'s
/// drop glue recurses over its spine, so a value deep enough to abort in
/// `Drop` aborts regardless ([`crate::depth`]'s Scope section and #200 carry
/// that residual — pre-flight [`guard_tree_depth`] by reference first if you
/// need to keep or leak an over-deep value). A tree at or below the limit is
/// embedded exactly as before the guard existed.
pub fn tree_to_free_mnd<A>(
    tree: BinaryTree<A>,
) -> Result<Free<TreeEndo<A>, Infallible>, DepthError> {
    guard_tree_depth(&tree)?;
    Ok(tree_to_free_mnd_inner(tree))
}

/// The recursive body of [`tree_to_free_mnd`], run only after the depth guard
/// has accepted the whole tree — so its recursion is bounded by
/// [`MAX_TREE_DEPTH`](crate::depth::MAX_TREE_DEPTH).
fn tree_to_free_mnd_inner<A>(tree: BinaryTree<A>) -> Free<TreeEndo<A>, Infallible> {
    match tree {
        BinaryTree::Leaf(a) => Free::Suspend(Either::Left(a)),
        BinaryTree::Node(left, right) => {
            let l = tree_to_free_mnd_inner(*left);
            let r = tree_to_free_mnd_inner(*right);
            // haft boxes each recursive slot *inside* the `Either` hole.
            Free::Suspend(Either::Right((Box::new(l), Box::new(r))))
        }
    }
}

/// Project a `Free<TreeEndo<A>, Infallible>` back to a [`BinaryTree<A>`].
///
/// CDL Example B.20. Inverse of [`tree_to_free_mnd`].
///
/// The `Infallible` terminator means the `Pure` arm is never reachable;
/// if a (presumably user-constructed) value somehow lands on `Pure`,
/// `Infallible` semantics let us discharge the impossible case with the
/// idiomatic `match z {}` exhaustion.
///
/// # Errors
///
/// Returns [`DepthError::TreeDepthExceeded`] if `input`'s spine is deeper than
/// [`MAX_TREE_DEPTH`](crate::depth::MAX_TREE_DEPTH) — the pre-flight recursion
/// guard (#231). The *measurement* is iterative and cannot overflow; note the
/// rejected `input` is still consumed and dropped here, and `Free`'s drop glue
/// recurses over its spine, so a value deep enough to abort in `Drop` aborts
/// regardless (see [`crate::depth`]'s Scope section — pre-flight
/// [`guard_free_mnd_depth`] by reference first to keep an over-deep value
/// alive). A spine at or below the limit is projected exactly as before the
/// guard existed. Note that a `Free` produced by [`tree_to_free_mnd`] has
/// already passed the same check (the bijection preserves depth); the guard
/// here is for spines built by hand — nested `Free::Suspend` cells, as the
/// crate's own tests do — which this function cannot know were pre-flighted.
pub fn free_mnd_to_tree<A>(
    input: Free<TreeEndo<A>, Infallible>,
) -> Result<BinaryTree<A>, DepthError> {
    guard_free_mnd_depth(&input)?;
    Ok(free_mnd_to_tree_inner(input))
}

/// The recursive body of [`free_mnd_to_tree`], run only after the depth guard
/// has accepted the whole spine — so its recursion is bounded by
/// [`MAX_TREE_DEPTH`](crate::depth::MAX_TREE_DEPTH).
fn free_mnd_to_tree_inner<A>(input: Free<TreeEndo<A>, Infallible>) -> BinaryTree<A> {
    match input {
        // `Infallible` has no values; this arm is statically unreachable
        // but we discharge it explicitly so the function is total.
        Free::Pure(z) => match z {},
        Free::Suspend(node) => match node {
            Either::Left(a) => BinaryTree::Leaf(a),
            Either::Right((l, r)) => {
                BinaryTree::node(free_mnd_to_tree_inner(*l), free_mnd_to_tree_inner(*r))
            }
        },
    }
}
