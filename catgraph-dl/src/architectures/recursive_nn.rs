//! Recursive NN — algebra of `Para(A + (−)²)`.
//!
//! CDL Example I.2. Carrier `S`; parametric map
//! `(P, cell) ∈ Para(Set)(A + S², S)`. Splits via
//! `P × (A + S²) ≅ P + P × A × S²` into:
//!
//! - `cell_0 : P × A → S` — leaf map (an earlier scaffold spelled it as
//!   `P → S`; the actual recursive-NN initial algebra leaves carry an
//!   `A` payload, so the cell-1 four-arg shape stays consistent with the
//!   smoke test's `(P, A, S, S)` signature).
//! - `cell_1 : P × A × S × S → S` — branching combinator. The original
//!   scaffold doc string showed `P × A × S² → S`; in Rust this is
//!   `(P, A, S, S)` since `S² = S × S`.
//!
//! Unrolling processes binary trees with shared parameters (CDL Example
//! J.3); the tree structure is the initial algebra of `A + (−)²`.
//!
//! ## `unroll`
//!
//! [`RecursiveNn::unroll`] is the unique algebra homomorphism
//! `(P, BinaryTree(A)) → S` from the initial algebra of the free monad
//! `FreeMnd(A + (−)²) ≅ BinaryTree(A)` into the cell's algebra. It walks
//! the tree post-order: leaves discharge through `cell_0(p)` (the scaffold
//! ignores the leaf payload), internal nodes descend into both subtrees
//! and combine via `cell_1`.
//!
//! It was the one architecture unroller behind [`crate::depth`]'s pre-flight
//! guard (#231), because it was the one that recursed. Since the v0.14.0 window
//! the walk is an explicit heap worklist and
//! [`BinaryTree`]'s own drop glue is iterative too, so **no depth can overflow
//! the stack** and the signature is infallible again — like the other four
//! unrollers, which are folds and `from_fn` state machines (issue
//! [#200](https://github.com/sustia-llc/catgraph/issues/200)).

use core::marker::PhantomData;

use crate::free_monad::tree_endo::{BinaryTree, TreeView};

/// A recursive-NN cell: algebra of `Para(A + (−)²)` on state `S`.
///
/// CDL Example I.2.
///
/// Opaque struct.
#[derive(Debug, Clone)]
pub struct RecursiveNn<P, S, Cell0, Cell1, A> {
    /// The parameter object `P`.
    pub parameter: P,
    /// The leaf map `cell_0 : P → S`.
    pub cell_0: Cell0,
    /// The branching map `cell_1 : P × A × S² → S`.
    pub cell_1: Cell1,
    _phantom: PhantomData<(S, A)>,
}

impl<P, S, Cell0, Cell1, A> RecursiveNn<P, S, Cell0, Cell1, A> {
    /// Build a recursive-NN cell from its parameter and cell maps.
    pub fn new(parameter: P, cell_0: Cell0, cell_1: Cell1) -> Self {
        Self {
            parameter,
            cell_0,
            cell_1,
            _phantom: PhantomData,
        }
    }
}

impl<P, S, Cell0, Cell1, A> RecursiveNn<P, S, Cell0, Cell1, A>
where
    P: Clone,
    A: Clone,
    Cell0: Fn(P) -> S,
    Cell1: Fn((P, A, S, S)) -> S,
{
    /// Unroll the cell over a [`BinaryTree`], threading the parameter `p`.
    ///
    /// CDL Remark 2.13 / Example J.3. The unique algebra homomorphism
    /// `(P, BinaryTree(A)) → S` from the initial algebra of the free
    /// monad on `A + (−)²` into the cell's algebra.
    ///
    /// Walk discipline:
    ///
    /// - `Leaf(_)` — return `cell_0(p)`. The leaf payload is consumed but
    ///   not threaded into `cell_0`; the `Para` decomposition `P × (A +
    ///   S²) ≅ P + P × A × S²` puts the `A` only on the *internal-node*
    ///   summand. Leaves arise from the bare `P` summand.
    /// - `Node(left, right)` — descend into both subtrees, then combine:
    ///   `cell_1((p, a, l, r))` where `a` is the leaf payload of the
    ///   leftmost leaf reachable. (Recursive-NN leaves arrive in `S` via
    ///   `cell_0`; the `A` consumed by `cell_1` is taken from the leftmost
    ///   leaf of the left subtree per the scaffold's 4-arg shape and the
    ///   tree-walk convention used in `tests/architecture_unrollers.rs`.)
    ///
    /// # Iteration discipline
    ///
    /// The walk is an explicit heap worklist — same posture as
    /// [`crate::free_monad::tree_endo::tree_to_free_mnd`] /
    /// [`crate::free_monad::tree_endo::free_mnd_to_tree`] since the v0.14.0
    /// window. The post-order in which `cell_0` / `cell_1` fire is unchanged
    /// from the recursive body it replaced (left subtree fully, then right,
    /// then combine), so a stateful cell observes the same call sequence.
    ///
    /// It cannot overflow the stack at any depth, which is why it no longer
    /// returns a `Result`: the #231 pre-flight guard's rejection had nothing
    /// left to prevent (issue
    /// [#200](https://github.com/sustia-llc/catgraph/issues/200)). A caller
    /// whose *own* `cell_0`/`cell_1` recurse is of course still on the hook for
    /// their own depth — [`crate::depth`] stays available for that.
    pub fn unroll(cell: &RecursiveNn<P, S, Cell0, Cell1, A>, tree: BinaryTree<A>) -> S {
        let mut work: Vec<UnrollStep<A>> = vec![UnrollStep::Descend(tree)];
        let mut done: Vec<S> = Vec::new();
        while let Some(step) = work.pop() {
            match step {
                UnrollStep::Descend(tree) => match tree.into_view() {
                    TreeView::Leaf(_a) => done.push((cell.cell_0)(cell.parameter.clone())),
                    TreeView::Node(children) => {
                        let (left, right) = *children;
                        // Taken before the descent, exactly as the recursive
                        // body did.
                        let leftmost = leftmost_leaf(&left);
                        work.push(UnrollStep::Combine(leftmost));
                        work.push(UnrollStep::Descend(right));
                        work.push(UnrollStep::Descend(left));
                    }
                },
                UnrollStep::Combine(a) => {
                    let r = done.pop().expect(ASSEMBLE);
                    let l = done.pop().expect(ASSEMBLE);
                    done.push((cell.cell_1)((cell.parameter.clone(), a, l, r)));
                }
            }
        }
        done.pop()
            .expect("invariant: the walk pushes exactly one result for the root")
    }
}

/// One step of the explicit-worklist [`RecursiveNn::unroll`]. A local `enum`
/// inside the method body cannot name the method's generics, so it lives here.
enum UnrollStep<A> {
    /// Walk this subtree.
    Descend(BinaryTree<A>),
    /// Both subtrees are unrolled: pop their results and apply `cell_1` with
    /// the leftmost-leaf payload captured at descent.
    Combine(A),
}

/// `expect` message for the two-children pop of a `Combine` step.
const ASSEMBLE: &str = "invariant: a Combine step is pushed only after its two subtrees' steps, \
     each of which pushes exactly one result";

/// Walk the tree to the leftmost leaf and return a clone of its payload.
///
/// Helper for [`RecursiveNn::unroll`] — the four-arg `cell_1((p, a, l, r))`
/// shape needs an `A` value at internal-node combination, but the
/// [`TreeView::Node`] variant carries no internal-node payload. The
/// convention here is to re-use the leftmost leaf's payload as the
/// branching `a`. Tests pass payload-agnostic cells so this choice does
/// not bias the acceptance harness.
fn leftmost_leaf<A: Clone>(tree: &BinaryTree<A>) -> A {
    let mut current = tree;
    loop {
        match current.as_view() {
            TreeView::Leaf(a) => return a.clone(),
            TreeView::Node(children) => current = &children.0,
        }
    }
}
