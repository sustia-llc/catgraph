//! The free monad carrier `Free<F, A> = Pure a | Suspend (f (Free f a))`.
//!
//! This module is private; the carrier is re-exported through
//! [`crate::free_monad`] and [`crate::endofunctor`], so the load-bearing prose
//! lives on the items below.

use core::fmt;
use core::marker::PhantomData;

use crate::endofunctor::{DebugFunctor, EqFunctor, Functor, HKT, Pure};

/// The free monad on the operation functor `F`: `Pure(a) | Suspend(F(Free))`.
///
/// CDL Proposition B.18. A *program* is a tree of `F`-shaped operation nodes
/// terminated by pure leaves; a *handler* is an `F`-algebra that folds the tree
/// into a result — [`fold`](Free::fold), the CDL Remark 2.13 catamorphism.
///
/// The recursion indirection sits **inside** the functor hole
/// (`Suspend(F::Type<Box<Free<F, A>>>)`), not around the applied functor, so a
/// carrier value costs exactly one `Box` per recursive hole.
///
/// `F` is an [`HKT`] witness; the recursion-consuming [`fold`](Free::fold)
/// additionally needs its [`Functor`] instance. Variants are public — pattern
/// matching *is* the carrier's surface, exactly as it is for the
/// [`BinaryTree`](crate::free_monad::tree_endo::BinaryTree) it is isomorphic to.
pub enum Free<F, A>
where
    F: HKT,
{
    /// A pure value — the leaf, and the monadic unit `η`.
    Pure(A),
    /// An operation node: an `F`-structure of sub-programs.
    Suspend(F::Type<Box<Free<F, A>>>),
}

impl<F, A> Free<F, A>
where
    F: HKT + Functor<F>,
{
    /// Interpret the program with a handler: `pure_case` for leaves and an
    /// `algebra : F::Type<X> → X` for operation nodes.
    ///
    /// This is the catamorphism that gives the operations meaning — CDL
    /// Remark 2.13's algebra-hom unroller, and the "handler" of the
    /// algebraic-effect reading.
    ///
    /// Recurses over the spine, so a caller holding a programmatically-built
    /// carrier should pre-flight [`crate::depth::free_mnd_depth`] (see that
    /// module's **Scope** section).
    pub fn fold<X, P, Alg>(self, pure_case: &P, algebra: &Alg) -> X
    where
        P: Fn(A) -> X,
        Alg: Fn(F::Type<X>) -> X,
    {
        match self {
            Free::Pure(a) => pure_case(a),
            Free::Suspend(fa) => algebra(F::fmap(fa, |boxed: Box<Free<F, A>>| {
                (*boxed).fold(pure_case, algebra)
            })),
        }
    }
}

// `Free` has no *derived* `PartialEq`/`Eq`/`Debug`. A `#[derive]`, or any hand
// impl gated on the GAT-projection field bound `F::Type<Box<Free<F, A>>>: Trait`,
// makes the instance conditional on that projection, so discharging it at a
// concrete witness re-enters the trait solver and overflows (`error[E0275]`).
// The impls below route the recursion through the witness's `EqFunctor` /
// `DebugFunctor` capability instead, which discharges against *this* impl's
// stable bounds and terminates — see `crate::endofunctor::EqFunctor`.

/// Structural equality: equal leaves, or equal operation nodes compared through
/// the functor's `eq_type`.
impl<F, A> PartialEq for Free<F, A>
where
    F: EqFunctor,
    A: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Free::Pure(a), Free::Pure(b)) => a == b,
            (Free::Suspend(x), Free::Suspend(y)) => F::eq_type(x, y),
            _ => false,
        }
    }
}

/// `Eq` is the marker upgrade of the structural `PartialEq`.
impl<F, A> Eq for Free<F, A>
where
    F: EqFunctor,
    A: Eq,
{
}

/// `Debug` mirrors the derive shape (`Pure(..)` / `Suspend(..)`), formatting the
/// operation node through the functor's `fmt_type`.
impl<F, A> fmt::Debug for Free<F, A>
where
    F: DebugFunctor,
    A: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Free::Pure(a) => {
                f.write_str("Pure(")?;
                fmt::Debug::fmt(a, f)?;
                f.write_str(")")
            }
            Free::Suspend(x) => {
                f.write_str("Suspend(")?;
                F::fmt_type(x, f)?;
                f.write_str(")")
            }
        }
    }
}

/// The [`HKT`] witness for the free monad over the operation functor `F`.
pub struct FreeWitness<F>(PhantomData<F>);

impl<F> HKT for FreeWitness<F>
where
    F: HKT,
{
    type Type<T> = Free<F, T>;
}

impl<F> Pure<FreeWitness<F>> for FreeWitness<F>
where
    F: HKT + Functor<F>,
{
    #[inline]
    fn pure<T>(value: T) -> Free<F, T> {
        Free::Pure(value)
    }
}
