//! The binary-tree endofunctor `TreeEndo<A> : X ↦ A + X²`.
//!
//! CDL Example B.20. The free monad on this endofunctor is the binary
//! tree with leaves in `A + Z`:
//!
//! ```text
//! FreeMnd(A + (−)²)(Z) ≅ Tree(A + Z)
//! ```
//!
//! `A + X²` is [`Either<A, (X, X)>`]: `Left(a)` a leaf, `Right((l, r))` a
//! node. [`BinaryTree<A>`] is the concrete carrier; [`tree_to_free_mnd`] /
//! [`free_mnd_to_tree`] witness the iso to `Free<TreeEndo<A>, Infallible>`.
//! Every walk in this module is an explicit heap worklist.
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

/// One cell of a [`BinaryTree<A>`], read through [`BinaryTree::into_view`] /
/// [`BinaryTree::as_view`]: a leaf, or a node boxing its subtree pair (one
/// allocation per internal node; `BinaryTree<A>` and `TreeView<A>` are the
/// same size). `Debug` prints `Node(<left>, <right>)`.
#[derive(Clone, PartialEq, Eq)]
pub enum TreeView<A> {
    /// A leaf labelled by `A`.
    Leaf(A),
    /// An internal node with left and right subtrees, boxed as one pair.
    Node(Box<(BinaryTree<A>, BinaryTree<A>)>),
}

/// `Leaf(<a>)` / `Node(<left>, <right>)`, subtrees through [`BinaryTree`]'s
/// iterative `Debug`.
impl<A: fmt::Debug> fmt::Debug for TreeView<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TreeView::Leaf(a) => f.debug_tuple("Leaf").field(a).finish(),
            TreeView::Node(children) => f
                .debug_tuple("Node")
                .field(&children.0)
                .field(&children.1)
                .finish(),
        }
    }
}

/// Binary tree with leaves in `A` (CDL Ex B.20): [`leaf`](BinaryTree::leaf) /
/// [`node`](BinaryTree::node), read through [`into_view`](BinaryTree::into_view)
/// / [`as_view`](BinaryTree::as_view). `Clone`, `PartialEq`, `Debug` and
/// `Drop` are iterative. `Drop` is hand-written, so a borrowed payload must
/// outlive the carrier:
///
/// ```compile_fail,E0597
/// # use catgraph_dl::free_monad::tree_endo::BinaryTree;
/// let tree;
/// let payload = String::from("x");
/// tree = BinaryTree::leaf(payload.as_str());
/// # let _ = &tree;
/// ```
///
/// ```
/// # use catgraph_dl::free_monad::tree_endo::BinaryTree;
/// let payload = String::from("x");
/// let tree = BinaryTree::leaf(payload.as_str());
/// # let _ = &tree;
/// ```
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

    /// Build an internal node by boxing the supplied subtrees as one pair —
    /// **one** allocation per internal node, not two (see [`TreeView`]).
    #[must_use]
    #[inline]
    pub fn node(left: Self, right: Self) -> Self {
        Self::from_view(TreeView::Node(Box::new((left, right))))
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
            if let TreeView::Node(children) = view {
                // Move the pair out of its `Box` — the allocation dies here and
                // the two husks below have their cells taken, so their own
                // `Drop` is a no-op and the recursion stops one level down.
                let (mut left, mut right) = *children;
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
                    TreeView::Node(children) => {
                        work.push(CloneStep::Assemble);
                        work.push(CloneStep::Visit(&children.1));
                        work.push(CloneStep::Visit(&children.0));
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
                (TreeView::Node(a), TreeView::Node(b)) => {
                    work.push((&a.0, &b.0));
                    work.push((&a.1, &b.1));
                }
                _ => return false,
            }
        }
        true
    }
}

impl<A: Eq> Eq for BinaryTree<A> {}

/// Iterative, streaming `Debug` — byte-identical to the derived output it
/// replaced, for every format spec it carries (see below).
///
/// Every byte of the output is written exactly once, so `{:?}` is Θ(total
/// output). `{:#?}` is Θ(total output) too, but a pretty rendering indents every
/// line by its nesting depth, so *that* output is quadratic in the depth of a
/// caterpillar. Neither aborts on a degenerate spine, which the derive did.
///
/// Carries alternate, precision and width; fill, alignment, sign, zero-pad
/// and debug-hex flags render as if absent.
impl<A: fmt::Debug> fmt::Debug for BinaryTree<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        super::write_debug(self, f)
    }
}

/// The shape of one cell, for the shared streaming renderer: `Leaf(a)` has no
/// recursion slot, `Node` has **two** — the boxed pair is an allocation detail,
/// not a slot, so the rendering stays the two-field `Node(<left>, <right>)`.
impl<A: fmt::Debug> super::DebugNode for BinaryTree<A> {
    fn slots(&self) -> Vec<&Self> {
        match self.as_view() {
            TreeView::Leaf(_) => Vec::new(),
            TreeView::Node(children) => vec![&children.0, &children.1],
        }
    }

    fn fmt_cell(&self, f: &mut fmt::Formatter<'_>, holes: &[&dyn fmt::Debug]) -> fmt::Result {
        match (self.as_view(), holes) {
            (TreeView::Leaf(a), _) => f.debug_tuple("Leaf").field(a).finish(),
            (TreeView::Node(..), [left, right]) => {
                f.debug_tuple("Node").field(left).field(right).finish()
            }
            // Unreachable: `holes` always has one entry per `slots` entry, and a
            // node has exactly two. A formatting error rather than a panic.
            (TreeView::Node(..), _) => Err(fmt::Error),
        }
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
                TreeView::Node(children) => {
                    let (left, right) = *children;
                    work.push(BijectionStep::Assemble);
                    work.push(BijectionStep::Descend(right));
                    work.push(BijectionStep::Descend(left));
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
            TreeView::Node(children) => {
                assert_eq!(children.0, BinaryTree::leaf(1_u8));
                assert_eq!(children.1, BinaryTree::leaf(2_u8));
            }
            TreeView::Leaf(_) => panic!("a node must view as Node"),
        }
        let rebuilt = BinaryTree::from_view(tree.clone().into_view());
        assert_eq!(rebuilt, tree);
    }

    /// [`TreeView`]'s **own** `Debug` is public output too, and it must keep
    /// printing `Node` as a **two-field** tuple variant now that the variant
    /// holds one boxed pair.
    ///
    /// This is the pin on the hand-written impl that replaced the derive: a
    /// `#[derive(Debug)]` on `Node(Box<(BinaryTree<A>, BinaryTree<A>)>)` renders
    /// the pair as a single field — `Node((Leaf(1), Leaf(2)))`, one extra paren
    /// pair — which would have been a silent regression in both the compact and
    /// the pretty form. `BinaryTree`'s own rendering is pinned separately (see
    /// `debug_reproduces_the_derived_shape` above and the derived-twin oracle in
    /// the parent module); this asserts the *view* agrees with it character for
    /// character, which is what it did before the reshape.
    #[test]
    fn the_view_debug_keeps_two_fields_on_node() {
        let tree = BinaryTree::node(BinaryTree::leaf(1_u8), BinaryTree::leaf(2_u8));
        let view = tree.as_view();

        assert_eq!(format!("{view:?}"), "Node(Leaf(1), Leaf(2))");
        assert_eq!(
            format!("{view:#?}"),
            "Node(\n    Leaf(\n        1,\n    ),\n    Leaf(\n        2,\n    ),\n)"
        );

        // The leaf arm, and agreement with the carrier's own rendering — the
        // property that held while `TreeView` still derived `Debug`.
        assert_eq!(format!("{:?}", tree.as_view()), format!("{tree:?}"));
        assert_eq!(format!("{:#?}", tree.as_view()), format!("{tree:#?}"));
        let leaf = BinaryTree::leaf(9_u8);
        assert_eq!(format!("{:?}", leaf.as_view()), "Leaf(9)");
    }
}
