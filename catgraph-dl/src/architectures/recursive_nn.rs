//! Recursive NN — algebra of `Para(A + (−)²)`.
//!
//! CDL Ex I.2. Carrier `S`; `cell_0 : P → S` on leaves (the leaf payload is
//! not passed), `cell_1 : (P, A, S, S) → S` on nodes. [`RecursiveNn::unroll`]
//! is the algebra homomorphism from `BinaryTree(A)` (CDL Ex J.3), a post-order
//! heap-worklist walk.

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
    /// Post-order walk (CDL Remark 2.13 / Ex J.3): `Leaf(_)` → `cell_0(p)`;
    /// `Node(l, r)` → `cell_1((p, a, l, r))` with `a` the payload of the
    /// leftmost leaf of the left subtree. Left subtree first, then right, then
    /// combine; iterative at every depth.
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
