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
//! In Rust we encode `A + X²` as [`Either<A, (X, X)>`], the crate's own sum.
//! The `Left(a)` summand is a tree leaf with payload `a : A`; the
//! `Right((l, r))` summand is an internal node with left/right subtrees.
//!
//! ## Carrier type
//!
//! [`BinaryTree<A>`] is the explicit carrier exposed for ergonomics —
//! constructing a `BinaryTree::leaf(0)` is friendlier than spelling out
//! `Free::suspend(Either::Left(0))`. The two helpers
//! [`tree_to_free_mnd`] and [`free_mnd_to_tree`] witness the iso to
//! `Free<TreeEndo<A>, Infallible>`.
//!
//! ## Iteration discipline (v0.14.0, issue [#200])
//!
//! Every walk in this module is an **explicit heap worklist**. That is a change
//! of posture: tree walks here used to be recursive on the grounds that trees
//! are tree-shaped, with the [`crate::depth`] pre-flight guard (#231) bounding
//! how deep a caller could go before the recursion aborted the process. The
//! guard could never cover the whole surface — a *rejected* tree was still
//! dropped by value, and [`BinaryTree`]'s own drop glue recursed — so the
//! walkers were rewritten instead of bounded.
//!
//! Consequently [`tree_to_free_mnd`] and [`free_mnd_to_tree`] are **infallible
//! again**: there is no depth at which they can fail. [`crate::depth`]'s
//! measures and guard remain as an opt-in service for callers whose *own* code
//! walks these carriers recursively.
//!
//! [#200]: https://github.com/sustia-llc/catgraph/issues/200
//!
//! # Why `Infallible`?
//!
//! Rust's `!` (`never_type`) is unstable. `core::convert::Infallible` is
//! the stable inhabitant of the same denotation — there are no values of
//! `Infallible`, so a `Free<F, Infallible>` cannot have a `Pure` leaf.
//! All leaves must come through `Suspend`, i.e. through the `Left(a)`
//! summand of `TreeEndo<A>`.

use core::convert::Infallible;
use core::fmt;
use core::marker::PhantomData;

use crate::container::Container;
use crate::endofunctor::{DebugFunctor, Either, EqFunctor, Free, FreeView, Functor, HKT};

use super::{DebugFields, Rendered, render};

/// The message on every `expect` guarding [`BinaryTree`]'s transient empty
/// cell — the same invariant `Free`/`Cofree` state.
const CELL: &str = "invariant: a live BinaryTree always holds its cell; it is emptied \
                    only by into_view/drop, which discard the husk immediately";

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
    type Type<X> = Either<A, (X, X)>;
}

impl<A> Functor<Self> for TreeEndo<A> {
    fn fmap<X, Y, Func>(fx: Either<A, (X, X)>, mut f: Func) -> Either<A, (Y, Y)>
    where
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

// Opt-in structural equality for `Free<TreeEndo<A>, Z>`: compare the *shape* of
// the functor hole `Either<A, (T, T)>` — which summand, and the leaf label —
// leaving the two subtree slots to the carrier's worklist. Bounded
// `A: PartialEq`; the slots need no bound at all here.
impl<A: PartialEq> EqFunctor for TreeEndo<A> {
    fn eq_shape<T>(a: &Either<A, (T, T)>, b: &Either<A, (T, T)>) -> bool {
        match (a, b) {
            (Either::Left(x), Either::Left(y)) => x == y,
            // Both node summands: same shape, arity 2. The subtrees are the
            // carrier's business.
            (Either::Right(_), Either::Right(_)) => true,
            _ => false,
        }
    }
}

// Opt-in `Debug` for `Free<TreeEndo<A>, Z>`: reproduce `Either`'s derived shape
// (`Left(a)` / `Right((l, r))`) with the two subtree slots supplied
// pre-rendered. Bounded `A: Debug`.
impl<A: fmt::Debug> DebugFunctor for TreeEndo<A> {
    fn fmt_shape<T>(
        fa: &Either<A, (T, T)>,
        f: &mut fmt::Formatter<'_>,
        contents: &[&dyn fmt::Debug],
    ) -> fmt::Result {
        match (fa, contents) {
            (Either::Left(a), _) => f.debug_tuple("Left").field(a).finish(),
            // `Either`'s derive prints the node summand as `Right((l, r))` —
            // one field, itself a pair.
            (Either::Right(_), [left, right]) => {
                f.debug_tuple("Right").field(&(left, right)).finish()
            }
            // Unreachable for contents taken from `fa` (arity 2); a formatting
            // error rather than a panic if a caller ever supplied otherwise.
            (Either::Right(_), _) => Err(fmt::Error),
        }
    }
}

/// Container presentation of `A + (−)²` (Abbott–Altenkirch–Ghani 2003, via
/// CDL). Shapes are `Either<A, ()>`: the leaf summand `Left(a)` (arity 0 — a
/// leaf carries its label but no recursive slot) and the node summand
/// `Right(())` (arity 2 — the two subtree slots).
impl<A> Container for TreeEndo<A> {
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

    fn contents<X>(fx: &Either<A, (X, X)>) -> Vec<&X> {
        match fx {
            Either::Left(_) => Vec::new(),
            Either::Right((l, r)) => vec![l, r],
        }
    }
}

/// One cell of a [`BinaryTree<A>`]: a labelled leaf, or an internal node with
/// left and right subtrees.
///
/// This is the shape [`BinaryTree`] used to *be*; it is still how a caller
/// reads one, through [`BinaryTree::into_view`] (by value) and
/// [`BinaryTree::as_view`] (by reference). See [`BinaryTree`] for why the shape
/// is no longer the carrier itself.
///
/// `Box` indirection on `Node` is required by the standard recursive-type
/// finite-size discipline. Dropping a `TreeView` drops two
/// `Box<BinaryTree<A>>` values, and *their* `Drop` is iterative — one level of
/// nesting here, none below it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeView<A> {
    /// A leaf labelled by `A`.
    Leaf(A),
    /// An internal node with left and right subtrees.
    Node(Box<BinaryTree<A>>, Box<BinaryTree<A>>),
}

/// Carrier type for binary trees with leaves in `A`.
///
/// CDL Example B.20. Build with [`leaf`](BinaryTree::leaf) /
/// [`node`](BinaryTree::node), read with [`into_view`](BinaryTree::into_view) /
/// [`as_view`](BinaryTree::as_view) — see [`TreeView`] for the two shapes.
///
/// # Why a struct with a private cell (v0.14.0, issue [#200])
///
/// `BinaryTree` used to be the two-variant enum now called [`TreeView`], with
/// derived `Clone`/`PartialEq`/`Debug` and the compiler's drop glue. All four
/// recursed over the `Box` spine, so cloning, comparing, printing **or merely
/// dropping** a degenerate caterpillar aborted the process — and the abort on
/// drop was reachable from a value the #231 depth guard had just *rejected*,
/// since the fallible entries took their input by value. The four impls below
/// are hand-written and iterative instead.
///
/// A hand-written [`Drop`] is what forces the reshape: it forbids moving out of
/// the value (`error[E0509]`), which a public-variant by-value `match` does.
/// The cell is an `Option` so `Drop` and `into_view` can take it without
/// `unsafe` (the crate is `#![forbid(unsafe_code)]`), and it is `Some` for every
/// observable value.
///
/// [#200]: https://github.com/sustia-llc/catgraph/issues/200
pub struct BinaryTree<A> {
    /// `Some` for every observable value; see [`CELL`].
    cell: Option<TreeView<A>>,
}

impl<A> BinaryTree<A> {
    /// Wrap a cell as a carrier value. The inverse of
    /// [`into_view`](Self::into_view).
    #[must_use]
    #[inline]
    pub fn from_view(view: TreeView<A>) -> Self {
        Self { cell: Some(view) }
    }

    /// Build a leaf.
    #[must_use]
    #[inline]
    pub fn leaf(a: A) -> Self {
        Self::from_view(TreeView::Leaf(a))
    }

    /// Build an internal node by boxing the supplied subtrees.
    #[must_use]
    #[inline]
    pub fn node(left: Self, right: Self) -> Self {
        Self::from_view(TreeView::Node(Box::new(left), Box::new(right)))
    }

    /// Consume the tree and hand back its cell, for a by-value `match`.
    /// Replaces the former `match tree { BinaryTree::Leaf(a) => .., .. }`.
    #[must_use]
    #[inline]
    pub fn into_view(mut self) -> TreeView<A> {
        self.cell.take().expect(CELL)
    }

    /// Borrow the cell, for a by-reference `match`. Match ergonomics bind the
    /// payloads by reference, so `match tree.as_view() { TreeView::Leaf(a) => …
    /// }` gives `a : &A`.
    #[must_use]
    #[inline]
    pub fn as_view(&self) -> &TreeView<A> {
        self.cell.as_ref().expect(CELL)
    }
}

impl<A> From<TreeView<A>> for BinaryTree<A> {
    #[inline]
    fn from(view: TreeView<A>) -> Self {
        Self::from_view(view)
    }
}

/// Iterative drop glue: a deep caterpillar dies on the heap, never on the stack.
///
/// This is the half of issue [#200](https://github.com/sustia-llc/catgraph/issues/200)
/// the #231 pre-flight guard could not reach — rejecting a by-value input did
/// not save that input's own recursive drop.
impl<A> Drop for BinaryTree<A> {
    fn drop(&mut self) {
        let Some(view) = self.cell.take() else {
            return;
        };
        let mut pending = vec![view];
        while let Some(view) = pending.pop() {
            // `Leaf(a)` drops its label here; a node hands its children over.
            if let TreeView::Node(mut left, mut right) = view {
                if let Some(child) = left.cell.take() {
                    pending.push(child);
                }
                if let Some(child) = right.cell.take() {
                    pending.push(child);
                }
            }
        }
    }
}

/// One step of the iterative [`Clone`].
enum CloneStep<'a, A> {
    /// Clone this subtree.
    Visit(&'a BinaryTree<A>),
    /// Its two children are cloned: pop them and rebuild the node.
    Assemble,
}

/// Iterative structural clone — the derived one recursed over the `Box` spine.
impl<A: Clone> Clone for BinaryTree<A> {
    fn clone(&self) -> Self {
        let mut work: Vec<CloneStep<'_, A>> = vec![CloneStep::Visit(self)];
        let mut done: Vec<BinaryTree<A>> = Vec::new();
        while let Some(step) = work.pop() {
            match step {
                CloneStep::Visit(tree) => match tree.as_view() {
                    TreeView::Leaf(a) => done.push(BinaryTree::leaf(a.clone())),
                    TreeView::Node(left, right) => {
                        work.push(CloneStep::Assemble);
                        work.push(CloneStep::Visit(right.as_ref()));
                        work.push(CloneStep::Visit(left.as_ref()));
                    }
                },
                CloneStep::Assemble => {
                    let right = done.pop().expect(ASSEMBLE);
                    let left = done.pop().expect(ASSEMBLE);
                    done.push(BinaryTree::node(left, right));
                }
            }
        }
        done.pop().expect(ROOT)
    }
}

/// Iterative structural equality — the derived one recursed over the spine.
impl<A: PartialEq> PartialEq for BinaryTree<A> {
    fn eq(&self, other: &Self) -> bool {
        let mut work: Vec<(&Self, &Self)> = vec![(self, other)];
        while let Some((left, right)) = work.pop() {
            match (left.as_view(), right.as_view()) {
                (TreeView::Leaf(a), TreeView::Leaf(b)) => {
                    if a != b {
                        return false;
                    }
                }
                (TreeView::Node(l1, r1), TreeView::Node(l2, r2)) => {
                    work.push((l1.as_ref(), l2.as_ref()));
                    work.push((r1.as_ref(), r2.as_ref()));
                }
                _ => return false,
            }
        }
        true
    }
}

impl<A: Eq> Eq for BinaryTree<A> {}

/// One step of the iterative [`Debug`] rendering.
enum RenderStep<'a, A> {
    /// Render this subtree once its children are rendered.
    Visit(&'a BinaryTree<A>),
    /// Its two children are rendered: pop them and render the node.
    Assemble,
}

/// Iterative `Debug`, byte-identical to the derived output it replaced.
///
/// Every level's text is materialised bottom-up, so `{:?}` costs the total
/// output and `{:#?}` costs that times the depth (its per-level re-indentation
/// touches every line again). Neither aborts on a degenerate spine, which the
/// derive did.
impl<A: fmt::Debug> fmt::Debug for BinaryTree<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let alternate = f.alternate();
        let mut work: Vec<RenderStep<'_, A>> = vec![RenderStep::Visit(self)];
        let mut done: Vec<String> = Vec::new();
        while let Some(step) = work.pop() {
            match step {
                RenderStep::Visit(tree) => match tree.as_view() {
                    TreeView::Leaf(a) => {
                        done.push(render(
                            alternate,
                            &DebugFields("Leaf", &[a as &dyn fmt::Debug]),
                        ));
                    }
                    TreeView::Node(left, right) => {
                        work.push(RenderStep::Assemble);
                        work.push(RenderStep::Visit(right.as_ref()));
                        work.push(RenderStep::Visit(left.as_ref()));
                    }
                },
                RenderStep::Assemble => {
                    let right = Rendered(done.pop().expect(ASSEMBLE));
                    let left = Rendered(done.pop().expect(ASSEMBLE));
                    done.push(render(
                        alternate,
                        &DebugFields("Node", &[&left as &dyn fmt::Debug, &right]),
                    ));
                }
            }
        }
        f.write_str(&done.pop().expect(ROOT))
    }
}

/// `expect` message for the two-children pop of an `Assemble` step.
const ASSEMBLE: &str = "invariant: an Assemble step is pushed only after its two children's steps, \
     each of which pushes exactly one result";

/// `expect` message for the single result a completed walk leaves behind.
const ROOT: &str = "invariant: the walk pushes exactly one result for the root";

/// One step of the iterative [`tree_to_free_mnd`] / [`free_mnd_to_tree`] walks.
enum BijectionStep<A> {
    /// Take this carrier apart.
    Descend(A),
    /// Its two children are converted: pop them and rebuild the node.
    Assemble,
}

/// Embed a [`BinaryTree<A>`] into the free monad over `TreeEndo<A>`.
///
/// CDL Example B.20. Witnesses the forward direction of the iso
/// `BinaryTree<A> ≅ Free<TreeEndo<A>, Infallible>`.
///
/// `Infallible` is the stable proxy for the never type `!`. Leaves of
/// the tree become `Suspend(Left(a))` cells; internal nodes become
/// `Suspend(Right((l', r')))` cells with (boxed) embedded subtrees. `Pure` is
/// unreachable — the `Z` slot is `Infallible`.
///
/// Infallible at any depth: the walk is an explicit heap worklist, and both
/// carriers' drop glue is iterative too, so nothing on this path can overflow
/// the stack (issue [#200]; this signature dropped its `Result` in the v0.14.0
/// window, reverting the #231 guard's `-> Result<_, DepthError>`).
///
/// [#200]: https://github.com/sustia-llc/catgraph/issues/200
#[must_use]
pub fn tree_to_free_mnd<A>(tree: BinaryTree<A>) -> Free<TreeEndo<A>, Infallible> {
    let mut work: Vec<BijectionStep<BinaryTree<A>>> = vec![BijectionStep::Descend(tree)];
    let mut done: Vec<Free<TreeEndo<A>, Infallible>> = Vec::new();
    while let Some(step) = work.pop() {
        match step {
            BijectionStep::Descend(tree) => match tree.into_view() {
                TreeView::Leaf(a) => done.push(Free::suspend(Either::Left(a))),
                TreeView::Node(left, right) => {
                    work.push(BijectionStep::Assemble);
                    work.push(BijectionStep::Descend(*right));
                    work.push(BijectionStep::Descend(*left));
                }
            },
            BijectionStep::Assemble => {
                let right = done.pop().expect(ASSEMBLE);
                let left = done.pop().expect(ASSEMBLE);
                // Each recursive slot is boxed *inside* the `Either` hole.
                done.push(Free::suspend(Either::Right((
                    Box::new(left),
                    Box::new(right),
                ))));
            }
        }
    }
    done.pop().expect(ROOT)
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
/// Infallible at any depth, for the same reason [`tree_to_free_mnd`] is
/// (issue [#200]).
///
/// [#200]: https://github.com/sustia-llc/catgraph/issues/200
#[must_use]
pub fn free_mnd_to_tree<A>(input: Free<TreeEndo<A>, Infallible>) -> BinaryTree<A> {
    let mut work: Vec<BijectionStep<Free<TreeEndo<A>, Infallible>>> =
        vec![BijectionStep::Descend(input)];
    let mut done: Vec<BinaryTree<A>> = Vec::new();
    while let Some(step) = work.pop() {
        match step {
            BijectionStep::Descend(cell) => match cell.into_view() {
                // `Infallible` has no values; this arm is statically
                // unreachable but we discharge it explicitly so the function is
                // total.
                FreeView::Pure(z) => match z {},
                FreeView::Suspend(node) => match node {
                    Either::Left(a) => done.push(BinaryTree::leaf(a)),
                    Either::Right((left, right)) => {
                        work.push(BijectionStep::Assemble);
                        work.push(BijectionStep::Descend(*right));
                        work.push(BijectionStep::Descend(*left));
                    }
                },
            },
            BijectionStep::Assemble => {
                let right = done.pop().expect(ASSEMBLE);
                let left = done.pop().expect(ASSEMBLE);
                done.push(BinaryTree::node(left, right));
            }
        }
    }
    done.pop().expect(ROOT)
}

#[cfg(test)]
mod tests {
    use super::{BinaryTree, TreeView};

    /// The hand-written iterative [`fmt::Debug`](core::fmt::Debug) must be
    /// **byte-identical** to the `#[derive(Debug)]` it replaced at #200 — a
    /// derive nothing can compare against any more, so the expected strings are
    /// spelled out here.
    ///
    /// Both forms are pinned. `{:#?}`'s per-level indentation is the part an
    /// iterative rendering could plausibly get wrong, and it is also why the
    /// pretty form costs more than the compact one on a deep spine (see the
    /// impl's note).
    #[test]
    fn debug_reproduces_the_derived_shape() {
        let leaf = BinaryTree::leaf(1_u8);
        assert_eq!(format!("{leaf:?}"), "Leaf(1)");
        assert_eq!(format!("{leaf:#?}"), "Leaf(\n    1,\n)");

        let node = BinaryTree::node(BinaryTree::leaf(1_u8), BinaryTree::leaf(2_u8));
        assert_eq!(format!("{node:?}"), "Node(Leaf(1), Leaf(2))");
        assert_eq!(
            format!("{node:#?}"),
            "Node(\n    Leaf(\n        1,\n    ),\n    Leaf(\n        2,\n    ),\n)"
        );

        // Three levels, so the nested indentation compounds — the case a
        // level-at-a-time renderer gets wrong if it forgets that the enclosing
        // pad adapter re-indents what it is handed.
        let deep = BinaryTree::node(node, BinaryTree::leaf(3_u8));
        assert_eq!(format!("{deep:?}"), "Node(Node(Leaf(1), Leaf(2)), Leaf(3))");
        assert_eq!(
            format!("{deep:#?}"),
            "Node(\n    Node(\n        Leaf(\n            1,\n        ),\n        Leaf(\n            2,\n        ),\n    ),\n    Leaf(\n        3,\n    ),\n)"
        );
    }

    /// `into_view` / `as_view` are the carrier's whole read surface since #200,
    /// and `from_view` closes the loop. Round-tripping a value through them must
    /// change nothing.
    #[test]
    fn views_round_trip() {
        let tree = BinaryTree::node(BinaryTree::leaf(1_u8), BinaryTree::leaf(2_u8));
        match tree.as_view() {
            TreeView::Node(left, right) => {
                assert_eq!(**left, BinaryTree::leaf(1_u8));
                assert_eq!(**right, BinaryTree::leaf(2_u8));
            }
            TreeView::Leaf(_) => panic!("a node must view as Node"),
        }
        let rebuilt = BinaryTree::from_view(tree.clone().into_view());
        assert_eq!(rebuilt, tree);
    }
}
