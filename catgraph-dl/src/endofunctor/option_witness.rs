//! [`OptionWitness`] — the ready-made `Option<T>` endofunctor.
//!
//! This module is private; the witness is re-exported through
//! [`crate::endofunctor`].

use core::fmt;

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
    fn eq_type<T: PartialEq>(a: &Option<T>, b: &Option<T>) -> bool {
        a == b
    }
}

impl DebugFunctor for OptionWitness {
    fn fmt_type<T: fmt::Debug>(fa: &Option<T>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(fa, f)
    }
}
