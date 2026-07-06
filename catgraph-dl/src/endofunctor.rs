//! Endofunctor substrate — the `deep_causality_haft` HKT/Functor witnesses
//! used by both [`crate::algebra`] (F-algebras and their homomorphisms) and
//! [`crate::free_monad`] (recursive `FreeMnd`/`CofreeCmnd`).
//!
//! This module was, prior to issue #12, the canonical home of a hand-rolled
//! `EndoFunctor` trait (a GAT `type Apply<X>` plus an `fmap`). That trait has
//! been removed in favour of the equivalent, already-proven witnesses from
//! `deep_causality_haft` v0.3.3. The module survives as the single import
//! point (and documentation home) for the adopted names.
//!
//! # Witness-first static dispatch
//!
//! Where the old trait fused object-map and morphism-map into one trait, haft
//! splits them:
//!
//! ```text
//! trait HKT              { type Constraint: ?Sized; type Type<T> where T: Satisfies<Self::Constraint>; }
//! trait Functor<F: HKT>  { fn fmap<A, B, Func>(m_a: F::Type<A>, f: Func) -> F::Type<B> where ...; }
//! ```
//!
//! [`HKT::Type`] is the object map of the endofunctor `F : Set → Set` (a
//! Generic Associated Type); [`Functor::fmap`] is the morphism map. A witness
//! is a zero-sized token implementing both `HKT` and `Functor<Self>`; `fmap`
//! is a static method — call `W::fmap(x, f)`, never a value method. Every
//! witness shipped here uses [`NoConstraint`] (the universal constraint whose
//! blanket `impl<T> Satisfies<NoConstraint> for T` admits any inner type),
//! matching CDL's ambient category `C = Set`.
//!
//! Because haft splits the two maps into separate traits, an `HKT`-only bound
//! would admit an fmap-less carrier — a categorically meaningless
//! "endofunctor". [`EndoWitness`] repackages the invariant the old fused trait
//! carried: `HKT<Constraint = NoConstraint> + Functor<Self>` (unconstrained
//! object map **and** morphism map). Downstream carriers (`FreeMnd`,
//! `CofreeCmnd`, the F-(co)algebra verifiers) bound on `EndoWitness` so the
//! type system again enforces "F is an endofunctor on Set".
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
//! These are documented obligations, not machine-checked at compile time —
//! see the [`Functor`] rustdoc in `deep_causality_haft` for the canonical
//! statement. A non-functorial witness is a soundness defect: it will cause
//! F-algebra homomorphism diagrams to fail to commute even for morphisms that
//! "should" commute. The three witnesses below now carry explicit
//! identity/composition tests (Gavranović et al., ICML 2024).
//!
//! The laws are stated for **pure (state-free) morphisms**. haft's `fmap` takes
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
//!
//! # Co-design note (#41)
//!
//! haft 0.3.3 ships [`Pure`](deep_causality_haft::Pure) and
//! [`NaturalIso`](deep_causality_haft::NaturalIso) but not `Pointed` /
//! `NaturalTransformation`; the first-class `NaturalTransformation<F, G>` /
//! `Pointed<F>` surfaces cg-dl documents as deferred obligations are layered
//! on in #41.

// The adopted names live in `deep_causality_haft`; re-exported here so the
// rest of the crate imports them from a single seam (`crate::endofunctor`).
// `Either` is the sum used by `TreeEndo`'s `Type<X> = Either<A, (X, X)>`.
pub use deep_causality_haft::{Either, Functor, HKT, NoConstraint, Satisfies};

/// An **endofunctor on `Set`** — the invariant the pre-#12 `EndoFunctor` trait
/// carried, repackaged over the split haft witnesses.
///
/// `deep_causality_haft` splits an endofunctor into two traits: [`HKT`] (the
/// object map `Type<X>`) and [`Functor`] (the morphism map `fmap`). An
/// `HKT`-only bound is therefore too weak — it would admit a carrier that
/// supplies the object map but no `fmap`, i.e. a type constructor that is not a
/// functor. `EndoWitness` is the conjunction: an unconstrained object map
/// (`Constraint = NoConstraint`, so any inner type is admissible — CDL's
/// ambient category is `Set`) **and** a morphism map (`Functor<Self>`).
///
/// It is a blanket-implemented marker: any type satisfying the two bounds
/// implements it automatically, so witnesses (`ListEndo`, `TreeEndo`,
/// `GroupActionEndo`) never name it — they just `impl HKT + Functor<Self>`.
/// Carriers that must enforce "F is an endofunctor" (`FreeMnd`, `CofreeCmnd`,
/// `FAlgebraHom` / `FCoalgebraHom`) bound on `EndoWitness` instead of the bare
/// `HKT<Constraint = NoConstraint>`. `HKT` is a supertrait, so the recursive
/// `F::Type<…>` projections resolve through it unchanged.
pub trait EndoWitness: HKT<Constraint = NoConstraint> + Functor<Self> + Sized {}

impl<T: HKT<Constraint = NoConstraint> + Functor<T>> EndoWitness for T {}
