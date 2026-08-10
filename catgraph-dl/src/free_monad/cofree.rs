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

use crate::endofunctor::{DebugFunctor, EqFunctor, Functor, HKT};

/// The cofree comonad on the functor `F`: a `head` label and an `F`-structure of
/// child sub-trees.
///
/// CDL Proposition B.18, the dual of [`Free`](crate::free_monad::Free): where
/// `Free` is a coproduct (an operation tree terminated by pure leaves), `Cofree`
/// is a product. It is the carrier for annotated trees and, over
/// [`OptionWitness`](crate::endofunctor::OptionWitness), for bounded stream
/// prefixes (CDL Remark H.6).
///
/// Fields are private — the shape is an invariant of the carrier, not a place to
/// edit. Use [`new`](Cofree::new) / [`head`](Cofree::head) /
/// [`tail`](Cofree::tail) / [`into_parts`](Cofree::into_parts).
///
/// # Finiteness
///
/// In pure theory `Cofree` is coinductive (infinite). In strict Rust it is
/// *finitely constructible* only over functors admitting an **empty** shape —
/// `Option`, a list functor that bottoms out. [`unfold`](Cofree::unfold) is the
/// generator; it terminates iff the coalgebra's `F`-structure is eventually
/// empty.
pub struct Cofree<F, A>
where
    F: HKT,
{
    head: A,
    tail: F::Type<Box<Cofree<F, A>>>,
}

impl<F, A> Cofree<F, A>
where
    F: HKT,
{
    /// Construct a node from its label and its `F`-structure of sub-trees.
    #[inline]
    pub fn new(head: A, tail: F::Type<Box<Cofree<F, A>>>) -> Self {
        Cofree { head, tail }
    }

    /// The label at this node.
    #[inline]
    pub fn head(&self) -> &A {
        &self.head
    }

    /// The `F`-structure of child sub-trees at this node — the borrowing
    /// accessor paired with [`into_parts`](Cofree::into_parts)'s by-value
    /// form, mirroring [`head`](Cofree::head).
    #[inline]
    pub fn tail(&self) -> &F::Type<Box<Cofree<F, A>>> {
        &self.tail
    }

    /// Decompose the node into its label and its `F`-structure of sub-trees.
    #[inline]
    pub fn into_parts(self) -> (A, F::Type<Box<Cofree<F, A>>>) {
        (self.head, self.tail)
    }
}

impl<F, A> Cofree<F, A>
where
    F: HKT + Functor<F>,
{
    /// The anamorphism, dual of [`Free::fold`](crate::free_monad::Free::fold):
    /// grow a tree from a `seed` and a `coalg`ebra producing, at each step, this
    /// node's label and the `F`-structure of child seeds —
    /// `unfold c x = let (a, fx) = c x in a :< fmap (unfold c) fx`.
    ///
    /// Terminates iff `coalg`'s `F`-structure is eventually empty (see
    /// [`Cofree`]'s finiteness note); `coalg` is borrowed and threaded through
    /// every hole, so it needs no `Clone`. Recurses as it grows — the residual
    /// [`crate::depth`]'s **Scope** section records.
    pub fn unfold<X, C>(seed: X, coalg: &C) -> Cofree<F, A>
    where
        C: Fn(X) -> (A, F::Type<X>),
    {
        let (a, fx) = coalg(seed);
        let tail = F::fmap(fx, |x: X| Box::new(Cofree::unfold(x, coalg)));
        Cofree { head: a, tail }
    }
}

// Opt-in `PartialEq`/`Debug` through the functor's capability traits — the
// same cycle-free mechanism `crate::endofunctor::EqFunctor` documents (a
// projection bound on `F::Type<Box<Cofree<F, A>>>` would overflow the trait
// solver, `error[E0275]`). No `Eq` marker, for the reason stated there: the
// capability is only PartialEq-strength over the witness's own label slots.

/// Structural equality: equal labels and equal `F`-structures of sub-trees,
/// compared through the functor's `eq_type`.
impl<F, A> PartialEq for Cofree<F, A>
where
    F: EqFunctor,
    A: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.head == other.head && F::eq_type(&self.tail, &other.tail)
    }
}

/// `Debug` mirrors the derive shape (`Cofree { head, tail }`), formatting the
/// `F`-structure through the functor's `fmt_type`.
impl<F, A> fmt::Debug for Cofree<F, A>
where
    F: DebugFunctor,
    A: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cofree")
            .field("head", &self.head)
            .field("tail", &super::FmtType::<F, _>(&self.tail))
            .finish()
    }
}

/// The [`HKT`] witness for the cofree comonad over the functor `F` (dual of
/// [`FreeWitness`](crate::free_monad::FreeWitness)).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CofreeWitness<F>(PhantomData<F>);

impl<F> HKT for CofreeWitness<F>
where
    F: HKT,
{
    type Type<T> = Cofree<F, T>;
}
