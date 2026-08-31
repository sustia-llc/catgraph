// Portions derived from deep_causality_haft 0.4.2 (the crate this substrate
// replaced at #222), used under the MIT license:
// SPDX-License-Identifier: MIT
// Copyright (c) 2023 - 2026. The DeepCausality Authors.
// Copyright (c) 2026 sustia-llc.

//! The cofree comonad carrier `Cofree<F, A> = a :< f (Cofree f a)`.
//!
//! This module is private; the carrier is re-exported through
//! [`crate::free_monad`] and [`crate::endofunctor`], so the load-bearing prose
//! lives on the items below.

use core::fmt;
use core::marker::PhantomData;

use crate::container::Container;
use crate::endofunctor::{DebugFunctor, EndoWitness, EqFunctor, HKT};

/// The message on every `expect` guarding the transient empty cell — the dual
/// of `free.rs`'s. [`Cofree`]'s cell is `None` only between
/// [`Cofree::into_parts`]/[`Cofree::drop`] taking it and the husk being
/// discarded.
const CELL: &str = "invariant: a live Cofree always holds its cell; it is emptied \
                    only by into_parts/drop, which discard the husk immediately";

/// The label-and-children pair a [`Cofree`] node carries.
///
/// Not public: the shape is an invariant of the carrier, not a place to edit —
/// the accessors below are the surface. It is a distinct type only so
/// [`Cofree`] can hold it behind an `Option` and hand it out whole; see
/// [`Cofree`] for why that indirection exists.
///
/// `pub(super)` — with its fields still private — solely so
/// `crate::free_monad`'s size pin can name the cell it measures `Cofree`
/// against, rather than a hand-written stand-in that could drift from it. The
/// enclosing `cofree` module is itself private, so nothing escapes the crate.
pub(super) struct CofreeCell<F, A>
where
    F: EndoWitness,
{
    head: A,
    tail: F::Type<Box<Cofree<F, A>>>,
}

/// Cofree comonad on `F` (CDL Prop B.18; bounded stream prefixes over
/// `OptionWitness`, CDL Remark H.6): a `head` label and an `F`-structure of
/// children, through [`new`](Cofree::new) /
/// [`head`](Cofree::head) / [`tail`](Cofree::tail) /
/// [`into_parts`](Cofree::into_parts). One machine word larger than its cell.
/// Finitely constructible only over functors with an empty shape;
/// [`unfold`](Cofree::unfold) terminates iff the coalgebra's `F`-structure is
/// eventually empty.
///
/// `Drop` is hand-written, so a borrowed payload must outlive the carrier:
///
/// ```compile_fail,E0597
/// # use catgraph_dl::endofunctor::OptionWitness;
/// # use catgraph_dl::free_monad::Cofree;
/// let cofree: Cofree<OptionWitness, &str>;
/// let payload = String::from("x");
/// cofree = Cofree::new(payload.as_str(), None);
/// # let _ = &cofree;
/// ```
///
/// ```
/// # use catgraph_dl::endofunctor::OptionWitness;
/// # use catgraph_dl::free_monad::Cofree;
/// let payload = String::from("x");
/// let cofree: Cofree<OptionWitness, &str> = Cofree::new(payload.as_str(), None);
/// # let _ = &cofree;
/// ```
pub struct Cofree<F, A>
where
    F: EndoWitness,
{
    /// `Some` for every observable value; see [`CELL`].
    cell: Option<CofreeCell<F, A>>,
}

impl<F, A> Cofree<F, A>
where
    F: EndoWitness,
{
    /// Construct a node from its label and its `F`-structure of sub-trees.
    #[inline]
    #[must_use]
    pub fn new(head: A, tail: F::Type<Box<Cofree<F, A>>>) -> Self {
        Cofree {
            cell: Some(CofreeCell { head, tail }),
        }
    }

    /// The label at this node.
    #[inline]
    #[must_use]
    pub fn head(&self) -> &A {
        &self.cell.as_ref().expect(CELL).head
    }

    /// The `F`-structure of child sub-trees at this node — the borrowing
    /// accessor paired with [`into_parts`](Cofree::into_parts)'s by-value
    /// form, mirroring [`head`](Cofree::head).
    #[inline]
    #[must_use]
    pub fn tail(&self) -> &F::Type<Box<Cofree<F, A>>> {
        &self.cell.as_ref().expect(CELL).tail
    }

    /// Decompose the node into its label and its `F`-structure of sub-trees.
    #[inline]
    #[must_use]
    pub fn into_parts(mut self) -> (A, F::Type<Box<Cofree<F, A>>>) {
        let CofreeCell { head, tail } = self.cell.take().expect(CELL);
        (head, tail)
    }
}

/// Iterative drop glue — the dual of [`Free`](crate::free_monad::Free)'s, and
/// the reason the carrier holds its cell behind an `Option`.
///
/// A `Cofree` spine is exactly what [`Cofree::unfold`] produces, so this is the
/// drop path a deep unfold's result takes. Each popped cell's label dies here
/// and its children go on the worklist; the emptied `Box<Cofree<..>>` husks
/// drop as no-ops.
impl<F, A> Drop for Cofree<F, A>
where
    F: EndoWitness,
{
    fn drop(&mut self) {
        let Some(cell) = self.cell.take() else {
            return;
        };
        let mut pending = vec![cell];
        while let Some(CofreeCell { head: _head, tail }) = pending.pop() {
            let _shape: F::Type<()> = F::fmap(tail, |mut boxed: Box<Cofree<F, A>>| {
                if let Some(child) = boxed.cell.take() {
                    pending.push(child);
                }
            });
        }
    }
}

/// One step of the explicit-worklist [`Cofree::unfold`]. A local `enum` inside
/// the method body cannot name the method's generics, so it lives here.
enum UnfoldStep<F, A, X>
where
    F: Container,
{
    /// Run the coalgebra on this seed.
    Expand(X),
    /// Its `arity` children are grown: pop them and assemble this node.
    Assemble(A, F::Shape, usize),
}

impl<F, A> Cofree<F, A>
where
    F: EndoWitness,
{
    /// Anamorphism `unfold c x = let (a, fx) = c x in a :< fmap (unfold c) fx`;
    /// iterative over a heap worklist, children assembled in position order.
    /// Terminates iff `coalg`'s `F`-structure is eventually empty.
    pub fn unfold<X, C>(seed: X, coalg: &C) -> Cofree<F, A>
    where
        F: Container,
        C: Fn(X) -> (A, F::Type<X>),
    {
        let mut work: Vec<UnfoldStep<F, A, X>> = vec![UnfoldStep::Expand(seed)];
        let mut done: Vec<Cofree<F, A>> = Vec::new();
        while let Some(step) = work.pop() {
            match step {
                UnfoldStep::Expand(x) => {
                    let (a, fx) = coalg(x);
                    let (shape, seeds) = F::decompose(fx);
                    work.push(UnfoldStep::Assemble(a, shape, seeds.len()));
                    // Pushed in reverse so the first position is expanded first
                    // and results land in position order.
                    for child in seeds.into_iter().rev() {
                        work.push(UnfoldStep::Expand(child));
                    }
                }
                UnfoldStep::Assemble(a, shape, arity) => {
                    let children: Vec<Box<Cofree<F, A>>> = done
                        .split_off(done.len() - arity)
                        .into_iter()
                        .map(Box::new)
                        .collect();
                    // `arity` came from the same `decompose` that produced
                    // `shape`, so container law 2 makes `recompose` total here.
                    let tail = F::recompose(shape, children).expect(
                        "invariant: recompose is given the exact arity decompose reported \
                         for this shape (Container law 2)",
                    );
                    done.push(Cofree::new(a, tail));
                }
            }
        }
        done.pop()
            .expect("invariant: the walk pushes exactly one node for the root seed")
    }
}

// Opt-in `PartialEq`/`Debug` through the functor's capability traits — the
// same cycle-free mechanism `crate::endofunctor::EqFunctor` documents (a
// projection bound on `F::Type<Box<Cofree<F, A>>>` would overflow the trait
// solver, `error[E0275]`). No `Eq` marker, for the reason stated there: the
// capability is only PartialEq-strength over the witness's own label slots.
//
// Both are explicit-worklist walks over the spine (#200); the capability
// supplies the shape half and `Container::contents` the recursion slots, hence
// the extra `Container` bound.

/// Structural equality: equal labels, `F`-structures of equal shape (through the
/// functor's `eq_shape`), and pairwise-equal sub-trees.
///
/// Iterative: the sub-tree pairs go on a heap worklist, so comparing two deep
/// spines cannot overflow.
impl<F, A> PartialEq for Cofree<F, A>
where
    F: EqFunctor + Container,
    A: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        let mut work: Vec<(&Self, &Self)> = vec![(self, other)];
        while let Some((left, right)) = work.pop() {
            if left.head() != right.head() {
                return false;
            }
            let (x, y) = (left.tail(), right.tail());
            if !F::eq_shape(x, y) {
                return false;
            }
            let (xs, ys) = (F::contents(x), F::contents(y));
            // Equal shapes have equal arity for a lawful container; checked
            // rather than trusted.
            if xs.len() != ys.len() {
                return false;
            }
            for (cx, cy) in xs.into_iter().zip(ys) {
                work.push((cx.as_ref(), cy.as_ref()));
            }
        }
        true
    }
}

/// `Debug` mirrors the derive shape (`Cofree { head, tail }`), formatting the
/// `F`-structure through the functor's `fmt_shape`.
///
/// Iterative and streaming, with the same cost note as
/// [`Free`](crate::free_monad::Free)'s `Debug`: every byte of the output is
/// written once, so `{:?}` is Θ(total output) and cannot overflow. `{:#?}` is
/// Θ(total output) too — but a pretty rendering indents every line by its
/// nesting depth, so that output is itself quadratic in the depth of a spine.
///
/// Carries alternate, precision and width; fill, alignment, sign, zero-pad
/// and debug-hex flags render as if absent. A payload `Debug` error
/// propagates as `fmt::Error`.
impl<F, A> fmt::Debug for Cofree<F, A>
where
    F: DebugFunctor + Container,
    A: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        super::write_debug(self, f)
    }
}

/// The shape of one node, for the shared streaming renderer: the `head` label
/// plus the witness's own contents as the recursion slots.
impl<F, A> super::DebugNode for Cofree<F, A>
where
    F: DebugFunctor + Container,
    A: fmt::Debug,
{
    fn slots(&self) -> Vec<&Self> {
        F::contents(self.tail())
            .into_iter()
            .map(|c| c.as_ref())
            .collect()
    }

    fn fmt_cell(&self, f: &mut fmt::Formatter<'_>, holes: &[&dyn fmt::Debug]) -> fmt::Result {
        f.debug_struct("Cofree")
            .field("head", self.head())
            .field("tail", &super::FmtShape::<F, _>(self.tail(), holes))
            .finish()
    }
}

/// The [`HKT`] witness for the cofree comonad over the functor `F` (dual of
/// [`FreeWitness`](crate::free_monad::FreeWitness)).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CofreeWitness<F>(PhantomData<F>);

impl<F> HKT for CofreeWitness<F>
where
    F: EndoWitness,
{
    type Type<T> = Cofree<F, T>;
}
