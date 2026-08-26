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

/// Opt-in shape equality for a witness's `Type<T>`: constructor and label
/// slots, never the contents. Gives [`Free`](crate::free_monad::Free) and
/// [`Cofree`](crate::free_monad::Cofree) their `PartialEq`. As reflexive,
/// symmetric and transitive as the label comparisons (partial for float
/// labels); two shapes called equal must have the same number of content
/// positions.
pub trait EqFunctor: HKT {
    /// Shape equality of two `Self::Type<T>` containers: same constructor and
    /// same labels, **ignoring** the contents at every position.
    fn eq_shape<T>(a: &Self::Type<T>, b: &Self::Type<T>) -> bool;
}

/// Opt-in `Debug` for a witness's `Type<T>` given its contents already
/// rendered; the twin of [`EqFunctor`].
pub trait DebugFunctor: HKT {
    /// Format the shape into `f`, writing each entry of `contents` (one per
    /// position, in [`Container::contents`](crate::container::Container::contents)
    /// order) directly into `f` via `Formatter` methods, exactly once; the
    /// carrier splices the real child at that position and fails with
    /// [`fmt::Error`] if an entry was rendered elsewhere. `f` carries
    /// `alternate`, `precision` and `width` only.
    ///
    /// # Errors
    ///
    /// [`fmt::Error`] from the writer, or when `contents.len()` disagrees with
    /// the structure's arity.
    fn fmt_shape<T>(
        fa: &Self::Type<T>,
        f: &mut fmt::Formatter<'_>,
        contents: &[&dyn fmt::Debug],
    ) -> fmt::Result;
}
