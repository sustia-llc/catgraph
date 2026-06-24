//! The cofree comonad `CofreeCmnd(F)(Z) = Fix(X ↦ F(X) × Z)`.
//!
//! CDL Proposition B.18 (dual). The cofree comonad on an endofunctor `F`
//! is the greatest fixed point of `X ↦ F(X) × Z`, presented coinductively
//! as a record with two fields:
//!
//! ```text
//! head : Z
//! tail : F(CofreeCmnd(F)(Z))
//! ```
//!
//! In Rust the coinductive presentation collapses to an inductive record
//! with a `Box`-indirected `tail` — every constructor at the value level
//! stores a finite witness. (Truly infinite `CofreeCmnd` values, e.g. for
//! `F = Stream`, would need `Lazy` / `Thunk` indirection; that is out of
//! scope for DL-2.)
//!
//! ## Recursion encoding
//!
//! Same `Box<F::Apply<…>>` indirection as [`super::free_mnd::FreeMnd`].
//! The struct is non-recursive at the field level because `tail` is
//! boxed; the recursion lives inside `F::Apply<Self>`.

use super::free_mnd::EndoFunctor;

/// The cofree comonad `CofreeCmnd(F)(Z) = Fix(X ↦ F(X) × Z)`.
///
/// CDL Proposition B.18 dual. A value carries a `head : Z` and a
/// `tail : F(CofreeCmnd(F)(Z))` — categorically, the counit and the
/// "next-step" projection of the cofree comonad.
///
/// # Examples
///
/// For a trivial endofunctor with `Apply<X> = ()`, the cofree comonad
/// collapses to a one-shot stream: `head` carries the only payload and
/// `tail` is `()`. See `tests/free_monad_bijections.rs`'s
/// `cofree_cmnd_smoke` test for the construction.
pub struct CofreeCmnd<F: EndoFunctor, Z> {
    /// The counit projection — the "current" value at this layer.
    pub head: Z,
    /// The recursive tail — `F` applied to the next layer of the
    /// comonad. Boxed for finite-size discipline.
    pub tail: Box<F::Apply<CofreeCmnd<F, Z>>>,
}

impl<F: EndoFunctor, Z> CofreeCmnd<F, Z> {
    /// Build a cofree-comonad layer from its head and tail.
    pub fn new(head: Z, tail: F::Apply<CofreeCmnd<F, Z>>) -> Self {
        Self {
            head,
            tail: Box::new(tail),
        }
    }
}

// `Clone` is a manual impl for the same GAT-projection reason as
// `FreeMnd` — the derive macro can't always solve `F::Apply<Self>: Clone`
// through the projection.
impl<F: EndoFunctor, Z: Clone> Clone for CofreeCmnd<F, Z>
where
    F::Apply<CofreeCmnd<F, Z>>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            head: self.head.clone(),
            tail: self.tail.clone(),
        }
    }
}

impl<F: EndoFunctor, Z: core::fmt::Debug> core::fmt::Debug for CofreeCmnd<F, Z>
where
    F::Apply<CofreeCmnd<F, Z>>: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CofreeCmnd")
            .field("head", &self.head)
            .field("tail", &self.tail)
            .finish()
    }
}

impl<F: EndoFunctor, Z: PartialEq> PartialEq for CofreeCmnd<F, Z>
where
    F::Apply<CofreeCmnd<F, Z>>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.head == other.head && self.tail == other.tail
    }
}

impl<F: EndoFunctor, Z: Eq> Eq for CofreeCmnd<F, Z> where F::Apply<CofreeCmnd<F, Z>>: Eq {}
