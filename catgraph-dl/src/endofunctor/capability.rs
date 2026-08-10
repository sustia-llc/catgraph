// Portions derived from deep_causality_haft 0.4.2 (the crate this substrate
// replaced at #222), used under the MIT license:
// SPDX-License-Identifier: MIT
// Copyright (c) 2023 - 2026. The DeepCausality Authors.
// Copyright (c) 2026 sustia-llc.

//! Witness capabilities: [`EqFunctor`] and [`DebugFunctor`], the opt-in routes
//! by which a witness gives the recursive carriers their `PartialEq`/`Eq` and
//! `Debug` instances.
//!
//! The mechanism is documented on [`EqFunctor`] — this module is private and
//! re-exported through [`crate::endofunctor`], so anything stated here alone
//! would not render.

use core::fmt;

use crate::endofunctor::HKT;

/// Witness capability: `Type<T>` can be compared structurally whenever the
/// payload `T: PartialEq`.
///
/// The `PartialEq` analogue of [`Functor`](crate::endofunctor::Functor) — where
/// `Functor` lets a witness supply `fmap`, `EqFunctor` lets it supply its
/// container's structural equality. It is what gives
/// [`Free`](crate::free_monad::Free) and [`Cofree`](crate::free_monad::Cofree)
/// their `PartialEq`/`Eq`.
///
/// # Why a capability, not `#[derive]`
///
/// Both carriers store their recursive child under a GAT projection —
/// `Suspend(F::Type<Box<Free<F, A>>>)`. A `#[derive]`, or any hand impl gated on
/// the projection bound `F::Type<Box<Free<F, A>>>: PartialEq`, makes the
/// instance *conditional on that projection*, so discharging it at a concrete
/// witness re-enters the trait solver and overflows (`error[E0275]`). Routing
/// the comparison through `eq_type` breaks the cycle: the recursion discharges
/// against the carrier's own impl and its stable bounds (`F: EqFunctor`,
/// `A: PartialEq`), exactly as a plain recursive
/// `enum List { Nil, Cons(i32, Box<List>) }` does.
///
/// # Opt-in
///
/// A witness opts in by implementing this trait; a witness that does not simply
/// has no `==` for its carriers, and nothing else changes.
///
/// # Law
///
/// `eq_type` is the container's structural equality: exactly as reflexive,
/// symmetric and transitive as the comparisons it delegates to — the payload
/// `T`'s `PartialEq` *and the witness's own label comparisons*. A
/// float-labelled witness (e.g. `ListEndo<f64>`, whose `eq_type` compares the
/// `f64` label) is only partial: `NaN != NaN` even at a total-`PartialEq`
/// payload type. This is why the carriers expose `PartialEq` and no `Eq`
/// marker — claiming total equivalence at the carrier while the capability is
/// partial over label slots would violate `Eq`'s contract.
pub trait EqFunctor: HKT {
    /// Structural equality of two `Self::Type<T>` containers, given
    /// `T: PartialEq`.
    fn eq_type<T: PartialEq>(a: &Self::Type<T>, b: &Self::Type<T>) -> bool;
}

/// Witness capability: `Type<T>` can be [`Debug`](core::fmt::Debug)-formatted
/// whenever the payload `T: Debug`.
///
/// The twin of [`EqFunctor`], opt-in the same way and for the same reason — see
/// that trait for why a `#[derive]` on the carriers overflows the trait solver.
pub trait DebugFunctor: HKT {
    /// `Debug`-format a `Self::Type<T>` container into `f`, given `T: Debug`.
    fn fmt_type<T: fmt::Debug>(fa: &Self::Type<T>, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}
