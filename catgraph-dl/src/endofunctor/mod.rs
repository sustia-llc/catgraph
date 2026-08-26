//! Endofunctor substrate: the HKT/Functor witness tower, [`Either`], the
//! [`EqFunctor`] / [`DebugFunctor`] shape capabilities, [`NaturalIso`], and
//! the stock [`OptionWitness`]. Trait and carrier shapes derive from
//! `deep_causality_haft` 0.4.2 (MIT, attributed in each defining file).
//!
//! ```text
//! trait HKT              { type Type<T>; }
//! trait Functor<F: HKT>  { fn fmap<A, B, Func>(m_a: F::Type<A>, f: Func) -> F::Type<B> where ...; }
//! ```
//!
//! [`HKT::Type`] is the object map of `F : Set → Set`, [`Functor::fmap`] the
//! morphism map; a witness is a zero-sized type implementing both, and `fmap`
//! is a static method. [`EndoWitness`] = `HKT + Functor<Self> + Sized`, the
//! bound the F-(co)algebra verifiers use.
//!
//! Functor laws, required of every witness for pure morphisms (`fmap` takes
//! `FnMut`, so a stateful closure may observe a different call order between
//! the two legs):
//!
//! ```text
//! fmap(fx, |x| x) == fx                             (identity)
//! fmap(fmap(fx, f), g) == fmap(fx, |x| g(f(x)))     (composition)
//! ```
//!
//! | Endofunctor | Witness | `Type<X>` |
//! |---|---|---|
//! | `1 + A × −` | [`crate::free_monad::list_endo::ListEndo<A>`] | `Option<(A, X)>` |
//! | `A + (−)²` | [`crate::free_monad::tree_endo::TreeEndo<A>`] | [`Either<A, (X, X)>`] |
//! | `G × −` | [`crate::algebra::GroupActionEndo<G>`] | `(G, X)` |
//! | `1 + −` | [`OptionWitness`] | `Option<X>` |
//!
//! [`EqFunctor`] / [`DebugFunctor`] decide a cell's constructor and labels;
//! the carriers walk the recursion slots through
//! [`Container`](crate::container::Container).

mod capability;
mod either;
mod hkt;
mod natural_iso;
mod option_witness;

pub use capability::{DebugFunctor, EqFunctor};
pub use either::Either;
pub use hkt::{Functor, HKT, Monad, Pure};
pub use natural_iso::NaturalIso;
pub use option_witness::OptionWitness;

// Compatibility mirror: the recursive carriers are defined and documented in
// `crate::free_monad` (the CDL Proposition B.18 module); they are surfaced
// here only so the rest of the crate keeps importing the whole substrate from
// one seam. `FreeView` joins them since #200 — it is how a `Free` is matched.
pub use crate::free_monad::{Cofree, CofreeWitness, Free, FreeView, FreeWitness};

// The natural-iso law helpers. `doc(hidden)` because they are test support, not
// part of the crate's documented surface — but `pub`, because integration tests
// under `tests/` reach them through this seam like everything else.
#[doc(hidden)]
pub use natural_iso::{assert_natural_iso_naturality, assert_natural_iso_round_trip};

/// An **endofunctor on `Set`** — the invariant the pre-#12 `EndoFunctor` trait
/// carried, repackaged over the split witnesses.
///
/// An endofunctor is split across two traits: [`HKT`] (the object map
/// `Type<X>`) and [`Functor`] (the morphism map `fmap`). An `HKT`-only bound is
/// therefore too weak — it would admit a carrier that supplies the object map
/// but no `fmap`, i.e. a type constructor that is not a functor. `EndoWitness`
/// is the conjunction.
///
/// It is a blanket-implemented marker: any type satisfying the two bounds
/// implements it automatically, so witnesses (`ListEndo`, `TreeEndo`,
/// `GroupActionEndo`) never name it — they just `impl HKT + Functor<Self>`.
/// Verifiers that must enforce "F is an endofunctor" (`FAlgebraHom` /
/// `FCoalgebraHom`) bound on `EndoWitness` instead of the bare [`HKT`]. `HKT` is
/// a supertrait, so the recursive `F::Type<…>` projections resolve through it
/// unchanged.
pub trait EndoWitness: HKT + Functor<Self> + Sized {}

impl<T: HKT + Functor<T>> EndoWitness for T {}
