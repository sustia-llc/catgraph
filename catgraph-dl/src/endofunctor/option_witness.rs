// Portions derived from deep_causality_haft 0.4.2 (the crate this substrate
// replaced at #222), used under the MIT license:
// SPDX-License-Identifier: MIT
// Copyright (c) 2023 - 2026. The DeepCausality Authors.
// Copyright (c) 2026 sustia-llc.

//! [`OptionWitness`] — the ready-made `Option<T>` endofunctor.
//!
//! This module is private; the witness is re-exported through
//! [`crate::endofunctor`].

use core::fmt;

use crate::container::Container;
use crate::endofunctor::{DebugFunctor, EqFunctor, Functor, HKT, Pure};

/// Zero-sized witness for the `Option<T>` type constructor.
///
/// Carries no CDL content of its own; it is the stock witness the crate's law
/// tests need for a *second*, non-crate-shaped functor (the cross-witness
/// natural iso `Option<((), X)> ≅ Option<X>`), and the branching functor of the
/// bounded-stream carrier `Cofree<OptionWitness, O>` behind the
/// coalgebra-direction unrollers (CDL Remark H.6).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OptionWitness;

impl HKT for OptionWitness {
    type Type<T> = Option<T>;
}

impl Functor<Self> for OptionWitness {
    fn fmap<A, B, Func>(m_a: Option<A>, f: Func) -> Option<B>
    where
        Func: FnMut(A) -> B,
    {
        m_a.map(f)
    }
}

/// The point `σ_X(x) = Some(x)`, making `Option` a
/// [`Pointed`](crate::natural::Pointed) endofunctor: `fmap` never turns a
/// `Some` into a `None`, so `fmap(pure(x), f) == pure(f(x))` holds.
impl Pure<Self> for OptionWitness {
    fn pure<T>(value: T) -> Option<T> {
        Some(value)
    }
}

impl EqFunctor for OptionWitness {
    fn eq_shape<T>(a: &Option<T>, b: &Option<T>) -> bool {
        // No label of its own: the shape is just which summand.
        a.is_some() == b.is_some()
    }
}

impl DebugFunctor for OptionWitness {
    fn fmt_shape<T>(
        fa: &Option<T>,
        f: &mut fmt::Formatter<'_>,
        contents: &[&dyn fmt::Debug],
    ) -> fmt::Result {
        match (fa, contents) {
            (None, _) => f.write_str("None"),
            (Some(_), [inner]) => f.debug_tuple("Some").field(inner).finish(),
            // Unreachable for contents taken from `fa` (arity 1); a formatting
            // error rather than a panic if a caller ever supplied otherwise.
            (Some(_), _) => Err(fmt::Error),
        }
    }
}

/// Container presentation of `1 + −` (Abbott–Altenkirch–Ghani 2003, via CDL).
/// Shapes are the two summands, carried as a `bool` (`is_some`): `false` is the
/// unit summand (arity 0) and `true` the singleton (arity 1 — the one recursive
/// slot).
///
/// Added in the v0.14.0 window because
/// [`Cofree::unfold`](crate::free_monad::Cofree::unfold) became an
/// explicit-worklist walk over [`Container`] (issue
/// [#200](https://github.com/sustia-llc/catgraph/issues/200)): without this
/// impl the bounded-stream carrier `Cofree<OptionWitness, O>` — the whole
/// reason this witness exists — would have lost its generator.
impl Container for OptionWitness {
    type Shape = bool;

    fn arity(shape: &Self::Shape) -> usize {
        usize::from(*shape)
    }

    fn decompose<X>(fx: Option<X>) -> (Self::Shape, Vec<X>) {
        match fx {
            None => (false, Vec::new()),
            Some(x) => (true, vec![x]),
        }
    }

    fn recompose<X>(shape: Self::Shape, contents: Vec<X>) -> Option<Option<X>> {
        if shape {
            // Arity 1: `TryFrom<Vec<X>> for [X; 1]` rejects any other length.
            let [x] = <[X; 1]>::try_from(contents).ok()?;
            Some(Some(x))
        } else {
            contents.is_empty().then_some(None)
        }
    }

    fn contents<X>(fx: &Option<X>) -> Vec<&X> {
        fx.as_ref().into_iter().collect()
    }
}

#[cfg(test)]
mod test {
    use super::OptionWitness;
    use crate::free_monad::Cofree;

    /// The stock witness's `EqFunctor`/`DebugFunctor` capabilities, exercised
    /// at the stream carrier they exist for — `==` and the derive-shaped
    /// `{:?}` both route through them.
    #[test]
    fn capabilities_reach_the_stream_carrier() {
        let one = Cofree::<OptionWitness, u32>::new(1, Some(Box::new(Cofree::new(2, None))));
        let same = Cofree::<OptionWitness, u32>::new(1, Some(Box::new(Cofree::new(2, None))));
        let other = Cofree::<OptionWitness, u32>::new(1, None);

        assert_eq!(one, same);
        assert_ne!(one, other);
        assert_eq!(
            format!("{one:?}"),
            "Cofree { head: 1, tail: Some(Cofree { head: 2, tail: None }) }"
        );
    }
}
