//! Endofunctor substrate — the HKT/Functor witnesses used by both
//! [`crate::algebra`] (F-algebras and their homomorphisms) and
//! [`crate::free_monad`] (the recursive [`Free`]/[`Cofree`] carriers).
//!
//! The substrate is **catgraph's own** since
//! [#222](https://github.com/sustia-llc/catgraph/issues/222): the witness tower,
//! the [`Either`] sum, the [`EqFunctor`]/[`DebugFunctor`] capabilities, the
//! [`NaturalIso`] surface, the stock [`OptionWitness`], and the two carriers are
//! all defined in this crate (trait and carrier shapes derived from
//! `deep_causality_haft` 0.4.2, MIT — attributed in each defining file), and
//! this module is the substrate's single import point. Documentation is
//! unidirectional: the traits' prose lives here, the carriers' lives in
//! [`crate::free_monad`], and the carrier re-export below is a compatibility
//! mirror, not a second documentation home. It arrived in three steps: the pre-#12 hand-rolled
//! `EndoFunctor` trait (a GAT `type Apply<X>` plus an `fmap`) was replaced by
//! external split witnesses in #12; the carriers followed in #93; #222 brings
//! the consumed surface in-tree — the traits below unchanged in shape apart from
//! the dropped constraint slot, the carriers pared to the methods this crate
//! uses (see [`crate::free_monad`]).
//!
//! # Witness-first static dispatch
//!
//! Object map and morphism map live in separate traits:
//!
//! ```text
//! trait HKT              { type Type<T>; }
//! trait Functor<F: HKT>  { fn fmap<A, B, Func>(m_a: F::Type<A>, f: Func) -> F::Type<B> where ...; }
//! ```
//!
//! [`HKT::Type`] is the object map of the endofunctor `F : Set → Set` (a
//! Generic Associated Type); [`Functor::fmap`] is the morphism map. A witness is
//! a zero-sized token implementing both `HKT` and `Functor<Self>`; `fmap` is a
//! static method — call `W::fmap(x, f)`, never a value method. The object map
//! admits **any** inner type: there is no constraint slot to declare, which is
//! CDL's ambient category `C = Set` written into the trait.
//!
//! Because the two maps are separate traits, an `HKT`-only bound would admit an
//! fmap-less carrier — a categorically meaningless "endofunctor".
//! [`EndoWitness`] repackages the invariant the old fused trait carried:
//! `HKT + Functor<Self>` (object map **and** morphism map). The F-(co)algebra
//! verifiers bound on `EndoWitness` so the type system again enforces "F is an
//! endofunctor on Set". [`Free`] / [`Cofree`] bound only `HKT` on their data and
//! add `Functor<F>` on the recursion-consuming methods.
//!
//! # Functor laws
//!
//! Implementors must guarantee the **functor laws**:
//!
//! ```text
//! fmap(fx, |x| x) == fx                             (identity)
//! fmap(fmap(fx, f), g) == fmap(fx, |x| g(f(x)))     (composition)
//! ```
//!
//! These are documented obligations, not machine-checked at compile time — see
//! the [`Functor`] rustdoc for the canonical statement. A non-functorial witness
//! is a soundness defect: it will cause F-algebra homomorphism diagrams to fail
//! to commute even for morphisms that "should" commute. The three witnesses
//! below carry explicit identity/composition tests (Gavranović et al., ICML
//! 2024).
//!
//! The laws are stated for **pure (state-free) morphisms**. `fmap` takes
//! `FnMut`, so a *stateful* closure can observe a different call order or count
//! between the two legs of the composition law — e.g. `TreeEndo`'s `Right` arm
//! calls the morphism twice and the two legs interleave the `f`/`g` calls
//! differently. Such a divergence is an artefact of the stateful closure, **not**
//! evidence of a non-functorial witness; feed the laws only pure morphisms.
//!
//! # Concrete instances in the workspace
//!
//! | Endofunctor | Witness | `Type<X>` |
//! |---|---|---|
//! | `1 + A × −` | [`crate::free_monad::list_endo::ListEndo<A>`] | `Option<(A, X)>` |
//! | `A + (−)²` | [`crate::free_monad::tree_endo::TreeEndo<A>`] | [`Either<A, (X, X)>`] |
//! | `G × −` | [`crate::algebra::GroupActionEndo<G>`] | `(G, X)` |
//! | `1 + −` | [`OptionWitness`] | `Option<X>` |
//!
//! # Recursion across this seam is gone (#231 → #200, closed at v0.14.0)
//!
//! [`Free::fold`] and [`Cofree::unfold`] used to recurse with nothing in front
//! of them, as did the carriers' compiler-generated drop glue and their
//! capability-routed `==` / `{:?}`. #231 bounded what it could reach with a
//! pre-flight guard at the crate's three walker entries; #200's remaining
//! surface — everything reachable *without* going through those entries — is
//! closed at v0.14.0 by rewriting the walks themselves. Every carrier operation
//! is now an explicit heap worklist, so no spine is too deep, and
//! [`crate::depth`] is a caller-facing measure rather than a guard the crate
//! relies on.
//!
//! Two capability consequences are visible from this seam:
//!
//! - [`EqFunctor`] and [`DebugFunctor`] are **shape-level**: `eq_shape` /
//!   `fmt_shape` decide the constructor and labels and leave the recursion
//!   slots to the carrier's worklist. Their payload-recursing predecessors
//!   (`eq_type` / the two-argument `fmt_type`) are gone.
//! - The recursion schemes, `==` and `{:?}` bound on
//!   [`Container`](crate::container::Container), the only capability that can
//!   pull a generic witness's recursion slots out and put results back.
//!
//! # Co-design note (#41)
//!
//! The substrate supplies [`Pure`] and [`NaturalIso`]; the first-class surfaces
//! layered on top are the crate's own — [`crate::natural::NaturalTransformation`]
//! and [`crate::natural::Pointed`] (built on [`Pure`] and [`NaturalIso`] via the
//! re-exports below) and [`crate::container::Container`]. Issue #41 shipped
//! them; #62, which tracked proposing the first two to the then-external
//! substrate, was closed as superseded and is moot now that the substrate is
//! ours.

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
