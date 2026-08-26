// Portions derived from deep_causality_haft 0.4.2 (the crate this substrate
// replaced at #222), used under the MIT license:
// SPDX-License-Identifier: MIT
// Copyright (c) 2023 - 2026. The DeepCausality Authors.
// Copyright (c) 2026 sustia-llc.

//! The free monad carrier `Free<F, A> = Pure a | Suspend (f (Free f a))`.
//!
//! This module is private; the carrier is re-exported through
//! [`crate::free_monad`] and [`crate::endofunctor`], so the load-bearing prose
//! lives on the items below.

use core::fmt;
use core::marker::PhantomData;

use crate::container::Container;
use crate::endofunctor::{DebugFunctor, EndoWitness, EqFunctor, HKT, Pure};

/// The message on every `expect` guarding the transient empty cell.
///
/// [`Free`]'s cell is `None` only *between* the moment [`Free::into_view`] or
/// [`Free::drop`] takes it and the moment the husk is discarded — never while a
/// value is observable. Any `expect` on it is discharged by that invariant.
const CELL: &str = "invariant: a live Free always holds its cell; it is emptied \
                    only by into_view/drop, which discard the husk immediately";

/// One cell of [`Free`]: `Pure(a)` or `Suspend(F::Type<Box<Free>>)`, read
/// through [`Free::into_view`] / [`Free::as_view`] (CDL Prop B.18).
pub enum FreeView<F, A>
where
    F: EndoWitness,
{
    /// A pure value — the leaf, and the monadic unit `η`.
    Pure(A),
    /// An operation node: an `F`-structure of sub-programs.
    Suspend(F::Type<Box<Free<F, A>>>),
}

/// Free monad on `F` (CDL Prop B.18): construct with [`pure`](Free::pure) /
/// [`suspend`](Free::suspend) / [`from_view`](Free::from_view) or
/// `From<FreeView<..>>`, read with
/// [`into_view`](Free::into_view) / [`as_view`](Free::as_view), interpret with
/// [`fold`](Free::fold). `fold`, `Drop`, `PartialEq` and `Debug` are heap
/// worklists; `fold`, `==` and `{:?}` require `F: `[`Container`].
///
/// `Drop` is hand-written, so a borrowed payload must outlive the carrier:
///
/// ```compile_fail,E0597
/// # use catgraph_dl::endofunctor::OptionWitness;
/// # use catgraph_dl::free_monad::Free;
/// let free: Free<OptionWitness, &str>;
/// let payload = String::from("x");
/// free = Free::pure(payload.as_str());
/// # let _ = &free;
/// ```
///
/// ```
/// # use catgraph_dl::endofunctor::OptionWitness;
/// # use catgraph_dl::free_monad::Free;
/// let payload = String::from("x");
/// let free: Free<OptionWitness, &str> = Free::pure(payload.as_str());
/// # let _ = &free;
/// ```
pub struct Free<F, A>
where
    F: EndoWitness,
{
    /// `Some` for every observable value; see [`CELL`].
    cell: Option<FreeView<F, A>>,
}

impl<F, A> Free<F, A>
where
    F: EndoWitness,
{
    /// Wrap a cell as a carrier value. The inverse of
    /// [`into_view`](Self::into_view).
    #[must_use]
    #[inline]
    pub fn from_view(view: FreeView<F, A>) -> Self {
        Self { cell: Some(view) }
    }

    /// The monadic unit `η`: a pure leaf. Replaces the former `Free::Pure(a)`
    /// constructor.
    #[must_use]
    #[inline]
    pub fn pure(value: A) -> Self {
        Self::from_view(FreeView::Pure(value))
    }

    /// An operation node over an `F`-structure of sub-programs. Replaces the
    /// former `Free::Suspend(fx)` constructor.
    #[must_use]
    #[inline]
    pub fn suspend(node: F::Type<Box<Free<F, A>>>) -> Self {
        Self::from_view(FreeView::Suspend(node))
    }

    /// Consume the carrier and hand back its cell, for a by-value `match`.
    /// Replaces the former `match free { Free::Pure(a) => .., .. }`.
    #[must_use]
    #[inline]
    pub fn into_view(mut self) -> FreeView<F, A> {
        self.cell.take().expect(CELL)
    }

    /// Borrow the cell, for a by-reference `match`. Match ergonomics bind the
    /// payloads by reference, so `match free.as_view() { FreeView::Pure(a) => …
    /// }` gives `a : &A`.
    #[must_use]
    #[inline]
    pub fn as_view(&self) -> &FreeView<F, A> {
        self.cell.as_ref().expect(CELL)
    }
}

impl<F, A> From<FreeView<F, A>> for Free<F, A>
where
    F: EndoWitness,
{
    #[inline]
    fn from(view: FreeView<F, A>) -> Self {
        Self::from_view(view)
    }
}

/// Iterative drop glue: a deep spine dies on the heap, never on the stack.
///
/// Each popped cell hands its recursive slots to the worklist and its labels to
/// the ordinary drop of `F::Type<()>` — the shape with every slot already
/// emptied. The `Box<Free<..>>` husks handed to the closure have had their own
/// cell taken, so their `Drop` is a no-op and the recursion stops one level
/// down, every time.
///
/// This is the half of issue [#200](https://github.com/sustia-llc/catgraph/issues/200)
/// that the #231 pre-flight guard could not reach: a guard that *rejects* a
/// too-deep value by value still had to drop it.
impl<F, A> Drop for Free<F, A>
where
    F: EndoWitness,
{
    fn drop(&mut self) {
        let Some(view) = self.cell.take() else {
            return;
        };
        let mut pending = vec![view];
        while let Some(view) = pending.pop() {
            if let FreeView::Suspend(node) = view {
                // The mapped-to-`()` shape keeps the witness's own labels and
                // drops them here; the slots are moved into `pending` first.
                let _shape: F::Type<()> = F::fmap(node, |mut boxed: Box<Free<F, A>>| {
                    if let Some(child) = boxed.cell.take() {
                        pending.push(child);
                    }
                });
            }
        }
    }
}

/// One step of the explicit-worklist [`Free::fold`].
///
/// A local `enum` inside the method body cannot name the method's generics, so
/// it lives here, private to the module.
enum FoldStep<F, A>
where
    F: Container,
{
    /// Take this sub-program apart.
    Descend(Free<F, A>),
    /// Its children are all folded: pop `arity` results and apply the algebra.
    Assemble(F::Shape, usize),
}

impl<F, A> Free<F, A>
where
    F: EndoWitness,
{
    /// Catamorphism (CDL Remark 2.13): `pure_case` on leaves,
    /// `algebra : F::Type<X> → X` on nodes; iterative over a heap worklist,
    /// children folded left to right.
    pub fn fold<X, P, Alg>(self, pure_case: &P, algebra: &Alg) -> X
    where
        F: Container,
        P: Fn(A) -> X,
        Alg: Fn(F::Type<X>) -> X,
    {
        let mut work: Vec<FoldStep<F, A>> = vec![FoldStep::Descend(self)];
        let mut done: Vec<X> = Vec::new();
        while let Some(step) = work.pop() {
            match step {
                FoldStep::Descend(node) => match node.into_view() {
                    FreeView::Pure(a) => done.push(pure_case(a)),
                    FreeView::Suspend(fa) => {
                        let (shape, children) = F::decompose(fa);
                        work.push(FoldStep::Assemble(shape, children.len()));
                        // Pushed in reverse so the first position is popped
                        // first and results land in position order.
                        for child in children.into_iter().rev() {
                            work.push(FoldStep::Descend(*child));
                        }
                    }
                },
                FoldStep::Assemble(shape, arity) => {
                    let contents = done.split_off(done.len() - arity);
                    // `arity` came from the same `decompose` that produced
                    // `shape`, so container law 2 makes `recompose` total here.
                    let fx = F::recompose(shape, contents).expect(
                        "invariant: recompose is given the exact arity decompose reported \
                         for this shape (Container law 2)",
                    );
                    done.push(algebra(fx));
                }
            }
        }
        done.pop()
            .expect("invariant: the walk pushes exactly one result for the root")
    }
}

// Opt-in `PartialEq`/`Debug` route through the witness capability traits —
// `crate::endofunctor::EqFunctor` is the canonical statement of why a
// `#[derive]` (or any projection-bound hand impl) would overflow the trait
// solver (`error[E0275]`). No `Eq` marker: `eq_shape` is only PartialEq-strength
// over the witness's own label slots (see `EqFunctor`'s law note), so claiming
// total equivalence would be false for float-labelled witnesses.
//
// Both are explicit-worklist walks over the spine (#200). The capability
// supplies the *shape* half — labels and constructor — and `Container::contents`
// supplies the recursive slots the worklist queues, which is why both impls
// bound on `Container` as well.

/// Structural equality: equal leaves, or operation nodes whose shapes agree
/// (through the functor's `eq_shape`) and whose contents agree pairwise.
///
/// Iterative: the content pairs go on a heap worklist, so comparing two deep
/// spines cannot overflow.
impl<F, A> PartialEq for Free<F, A>
where
    F: EqFunctor + Container,
    A: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        let mut work: Vec<(&Self, &Self)> = vec![(self, other)];
        while let Some((left, right)) = work.pop() {
            match (left.as_view(), right.as_view()) {
                (FreeView::Pure(a), FreeView::Pure(b)) => {
                    if a != b {
                        return false;
                    }
                }
                (FreeView::Suspend(x), FreeView::Suspend(y)) => {
                    if !F::eq_shape(x, y) {
                        return false;
                    }
                    let (xs, ys) = (F::contents(x), F::contents(y));
                    // Equal shapes have equal arity for a lawful container;
                    // checked rather than trusted.
                    if xs.len() != ys.len() {
                        return false;
                    }
                    for (cx, cy) in xs.into_iter().zip(ys) {
                        work.push((cx.as_ref(), cy.as_ref()));
                    }
                }
                _ => return false,
            }
        }
        true
    }
}

/// `Debug` mirrors the derive shape (`Pure(..)` / `Suspend(..)`), formatting the
/// operation node through the functor's `fmt_shape`.
///
/// Iterative and streaming: each cell's own shape is laid out once and written
/// straight to the formatter, with the children visited on an explicit stack, so
/// a deep spine can neither overflow nor cost more than the output it produces —
/// Θ(total output) for `{:?}`. `{:#?}` is Θ(total output) too, but that output is
/// itself quadratic in the depth of a spine, because a pretty rendering indents
/// every line by its nesting depth.
///
/// Carries alternate, precision and width; fill, alignment, sign, zero-pad
/// and debug-hex flags render as if absent. A payload `Debug` error
/// propagates as `fmt::Error`.
impl<F, A> fmt::Debug for Free<F, A>
where
    F: DebugFunctor + Container,
    A: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        super::write_debug(self, f)
    }
}

/// The shape of one cell, for the shared streaming renderer: `Pure(a)` has no
/// recursion slot, `Suspend(fa)` has the witness's own contents.
impl<F, A> super::DebugNode for Free<F, A>
where
    F: DebugFunctor + Container,
    A: fmt::Debug,
{
    fn slots(&self) -> Vec<&Self> {
        match self.as_view() {
            FreeView::Pure(_) => Vec::new(),
            FreeView::Suspend(fa) => F::contents(fa).into_iter().map(|c| c.as_ref()).collect(),
        }
    }

    fn fmt_cell(&self, f: &mut fmt::Formatter<'_>, holes: &[&dyn fmt::Debug]) -> fmt::Result {
        match self.as_view() {
            FreeView::Pure(a) => f.debug_tuple("Pure").field(a).finish(),
            FreeView::Suspend(fa) => f
                .debug_tuple("Suspend")
                .field(&super::FmtShape::<F, _>(fa, holes))
                .finish(),
        }
    }
}

/// The [`HKT`] witness for the free monad over the operation functor `F`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FreeWitness<F>(PhantomData<F>);

impl<F> HKT for FreeWitness<F>
where
    F: EndoWitness,
{
    type Type<T> = Free<F, T>;
}

impl<F> Pure<FreeWitness<F>> for FreeWitness<F>
where
    F: EndoWitness,
{
    #[inline]
    fn pure<T>(value: T) -> Free<F, T> {
        Free::pure(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endofunctor::{Either, OptionWitness};
    use crate::free_monad::tree_endo::TreeEndo;

    /// The iterative `Debug` must render exactly what the pre-#200 recursive one
    /// did: `Pure(..)` / `Suspend(..)` around the witness's own shape, in both
    /// the compact and the pretty form. Nothing else compares the carriers'
    /// output against a fixed string at more than one level of nesting.
    #[test]
    fn debug_reproduces_the_pre_reshape_shape() {
        type F = TreeEndo<u8>;
        let leaf = || Free::<F, u8>::suspend(Either::Left(7_u8));
        assert_eq!(format!("{:?}", leaf()), "Suspend(Left(7))");

        let node: Free<F, u8> = Free::suspend(Either::Right((Box::new(leaf()), Box::new(leaf()))));
        assert_eq!(
            format!("{node:?}"),
            "Suspend(Right((Suspend(Left(7)), Suspend(Left(7)))))"
        );

        // The `Pure` arm, and the pretty form's compounding indentation.
        let pure: Free<OptionWitness, u8> = Free::pure(3);
        assert_eq!(format!("{pure:?}"), "Pure(3)");
        assert_eq!(format!("{pure:#?}"), "Pure(\n    3,\n)");

        let cons: Free<OptionWitness, u8> = Free::suspend(Some(Box::new(Free::pure(3_u8))));
        assert_eq!(format!("{cons:?}"), "Suspend(Some(Pure(3)))");
        assert_eq!(
            format!("{cons:#?}"),
            "Suspend(\n    Some(\n        Pure(\n            3,\n        ),\n    ),\n)"
        );
    }

    // The cell's size is pinned for **every** carrier by
    // `crate::free_monad::tests::the_private_cell_costs_at_most_one_word`,
    // which supersedes the single-instantiation check that used to live here:
    // `Free<OptionWitness, u64>` is the one place the `Option` wrapper is free,
    // so asserting only it passed while two carriers were 50 % wider.
}
