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

/// Witness capability: two `Type<T>` structures can be compared **by shape**.
///
/// The `PartialEq` analogue of [`Functor`](crate::endofunctor::Functor) — where
/// `Functor` lets a witness supply `fmap`, `EqFunctor` lets it supply its
/// container's shape equality. It is what gives
/// [`Free`](crate::free_monad::Free) and [`Cofree`](crate::free_monad::Cofree)
/// their `PartialEq`.
///
/// # Why a capability, not `#[derive]`
///
/// Both carriers store their recursive child under a GAT projection —
/// `Suspend(F::Type<Box<Free<F, A>>>)`. A `#[derive]`, or any hand impl gated on
/// the projection bound `F::Type<Box<Free<F, A>>>: PartialEq`, makes the
/// instance *conditional on that projection*, so discharging it at a concrete
/// witness re-enters the trait solver and overflows (`error[E0275]`). Routing
/// the comparison through `eq_shape` breaks the cycle: the recursion discharges
/// against the carrier's own impl and its stable bounds (`F: EqFunctor`,
/// `A: PartialEq`), exactly as a plain recursive
/// `enum List { Nil, Cons(i32, Box<List>) }` does.
///
/// # Opt-in
///
/// A witness opts in by implementing this trait; a witness that does not simply
/// has no `==` for its carriers, and nothing else changes.
///
/// # Shape-level, not payload-level (v0.14.0)
///
/// [`eq_shape`](Self::eq_shape) compares the **non-recursive** part of the
/// structure only — constructor choice plus the witness's own label slots —
/// and never looks inside a content position. That is what makes the carriers'
/// `==` an explicit-worklist walk rather than a recursive one (issue
/// [#200](https://github.com/sustia-llc/catgraph/issues/200)): the carrier
/// pairs the content positions itself, via
/// [`Container::contents`](crate::container::Container::contents), and pushes
/// them onto a heap worklist. Its predecessor `eq_type<T: PartialEq>` compared
/// the contents *inside the witness*, so a deep spine recursed through the
/// witness's own `==` — frames the carrier could not heapify.
///
/// # Law
///
/// `eq_shape` is the container's shape equality: exactly as reflexive,
/// symmetric and transitive as the witness's own label comparisons. A
/// float-labelled witness (e.g. `ListEndo<f64>`, whose `eq_shape` compares the
/// `f64` label) is only partial: `NaN != NaN`. This is why the carriers expose
/// `PartialEq` and no `Eq` marker — claiming total equivalence at the carrier
/// while the capability is partial over label slots would violate `Eq`'s
/// contract.
///
/// A witness must also keep `eq_shape` **coherent with arity**: two structures
/// it calls equal must have the same number of content positions, so the
/// carrier's pairing is total. (Automatic for a lawful
/// [`Container`](crate::container::Container) — equal shapes have equal
/// arities — and the carriers fall back to `false` on a length mismatch rather
/// than trusting it.)
pub trait EqFunctor: HKT {
    /// Shape equality of two `Self::Type<T>` containers: same constructor and
    /// same labels, **ignoring** the contents at every position.
    fn eq_shape<T>(a: &Self::Type<T>, b: &Self::Type<T>) -> bool;
}

/// Witness capability: `Type<T>` can be [`Debug`](core::fmt::Debug)-formatted,
/// given its contents already rendered.
///
/// The twin of [`EqFunctor`], opt-in the same way and for the same reason — see
/// that trait for why a `#[derive]` on the carriers overflows the trait solver,
/// and why the capability is shape-level.
pub trait DebugFunctor: HKT {
    /// `Debug`-format a `Self::Type<T>` container into `f`, writing `contents`
    /// in place of the values at the content positions.
    ///
    /// `contents` carries one entry per position, in position order (i.e.
    /// exactly [`Container::contents`](crate::container::Container::contents)'
    /// order), already rendered by the carrier — so this method must **not**
    /// format the `T` values themselves. That inversion is what keeps the
    /// carriers' `{:?}` off the stack for a deep spine.
    ///
    /// # Errors
    ///
    /// Returns [`fmt::Error`] if the underlying writer fails. A witness may
    /// also return it when `contents.len()` disagrees with the structure's
    /// arity — an unreachable state for a caller that took `contents` from the
    /// same value, and a formatting error rather than a panic if it ever
    /// happened.
    fn fmt_shape<T>(
        fa: &Self::Type<T>,
        f: &mut fmt::Formatter<'_>,
        contents: &[&dyn fmt::Debug],
    ) -> fmt::Result;
}
