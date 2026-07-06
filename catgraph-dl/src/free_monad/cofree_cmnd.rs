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
//! scope for the current surface.)
//!
//! ## Recursion encoding
//!
//! Same `Box<F::Type<…>>` indirection as [`super::free_mnd::FreeMnd`].
//! The struct is non-recursive at the field level because `tail` is
//! boxed; the recursion lives inside `F::Type<Self>`.

use crate::endofunctor::EndoWitness;

/// The cofree comonad `CofreeCmnd(F)(Z) = Fix(X ↦ F(X) × Z)`.
///
/// CDL Proposition B.18 dual. A value carries a `head : Z` and a
/// `tail : F(CofreeCmnd(F)(Z))` — categorically, the counit and the
/// "next-step" projection of the cofree comonad.
///
/// # Constraint restriction
///
/// The `F: EndoWitness` bound — the dual of [`super::free_mnd::FreeMnd`]'s —
/// packages `HKT<Constraint = NoConstraint> + Functor<Self>`: an unconstrained
/// object map plus a morphism map. It therefore admits only haft witnesses with
/// `Constraint = NoConstraint`; a constrained witness (`Constraint !=
/// NoConstraint`) cannot instantiate `CofreeCmnd`. This is deliberate — CDL's
/// ambient category is `Set` and every shipped witness is unconstrained. The
/// general self-referential `where CofreeCmnd<F, Z>: Satisfies<F::Constraint>`
/// form was rejected as viral (it would thread through the struct and every
/// manual `Clone`/`Debug`/`PartialEq`/`Eq` impl); the `EndoWitness` bound also
/// restores the "F is an endofunctor" invariant a bare `HKT` bound would drop.
///
/// # Examples
///
/// For a trivial endofunctor with `Type<X> = ()`, the cofree comonad
/// collapses to a one-shot stream: `head` carries the only payload and
/// `tail` is `()`. See `tests/free_monad_bijections.rs`'s
/// `cofree_cmnd_smoke` test for the construction.
pub struct CofreeCmnd<F: EndoWitness, Z> {
    /// The counit projection — the "current" value at this layer.
    pub head: Z,
    /// The recursive tail — `F` applied to the next layer of the
    /// comonad. Boxed for finite-size discipline.
    pub tail: Box<F::Type<CofreeCmnd<F, Z>>>,
}

impl<F: EndoWitness, Z> CofreeCmnd<F, Z> {
    /// Build a cofree-comonad layer from its head and tail.
    pub fn new(head: Z, tail: F::Type<CofreeCmnd<F, Z>>) -> Self {
        Self {
            head,
            tail: Box::new(tail),
        }
    }
}

// `Clone` is a manual impl for the same GAT-projection reason as
// `FreeMnd` — the derive macro can't always solve `F::Type<Self>: Clone`
// through the projection.
impl<F: EndoWitness, Z: Clone> Clone for CofreeCmnd<F, Z>
where
    F::Type<CofreeCmnd<F, Z>>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            head: self.head.clone(),
            tail: self.tail.clone(),
        }
    }
}

impl<F: EndoWitness, Z: core::fmt::Debug> core::fmt::Debug for CofreeCmnd<F, Z>
where
    F::Type<CofreeCmnd<F, Z>>: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CofreeCmnd")
            .field("head", &self.head)
            .field("tail", &self.tail)
            .finish()
    }
}

impl<F: EndoWitness, Z: PartialEq> PartialEq for CofreeCmnd<F, Z>
where
    F::Type<CofreeCmnd<F, Z>>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.head == other.head && self.tail == other.tail
    }
}

impl<F: EndoWitness, Z: Eq> Eq for CofreeCmnd<F, Z> where F::Type<CofreeCmnd<F, Z>>: Eq {}
