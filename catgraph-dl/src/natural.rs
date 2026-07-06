//! Natural transformations and pointed endofunctors over the `Set` substrate.
//!
//! Two first-class surfaces layered on the [`crate::endofunctor`] witnesses
//! (issue #41):
//!
//! - [`NaturalTransformation<F, G>`] — a component family `α_X : F(X) → G(X)`
//!   witnessing a natural transformation `α : F ⇒ G` (Gavranović et al., ICML
//!   2024, Def 1.5).
//! - [`Pointed`] — a pointed endofunctor `(F, σ)` with `σ : id ⇒ F` supplied
//!   by haft's [`Pure`] (CDL Def B.3, Appendix B.1).
//!
//! Both mirror the crate's witness-first static-dispatch style: the transform
//! is a static method on a zero-sized witness (`W::transform(fa)`), never a
//! value method, exactly as [`Functor::fmap`](crate::endofunctor::Functor::fmap) is.
//!
//! # Why adapter witnesses instead of a blanket `NaturalIso` impl
//!
//! Every [`NaturalIso<F, G>`] gives *two* natural transformations — `to_target`
//! (`F ⇒ G`) and `to_source` (`G ⇒ F`). A pair of blanket impls
//! `impl<W: NaturalIso<F, G>> NaturalTransformation<F, G> for W` and
//! `impl<W: NaturalIso<F, G>> NaturalTransformation<G, F> for W` would overlap:
//! choosing the trait parameters `(A, B)` in the first (with `F = A, G = B`) and
//! in the second (with `F = B, G = A`) both yield
//! `NaturalTransformation<A, B> for W`, a conflicting-implementations error. The
//! two ZST adapters [`IsoForward`] and [`IsoBackward`] carry the direction in
//! the *witness type* instead, so the two transformations never collide.
//!
//! # Non-instances of `Pointed`
//!
//! Neither `free_monad` witness ships a [`Pure`] impl, for two different —
//! and instructive — reasons:
//!
//! - [`crate::free_monad::list_endo::ListEndo<A>`] (`Type<X> = Option<(A, X)>`)
//!   *does* admit a natural point: the constant family `σ_X(x) = None`
//!   commutes with every `fmap`. But it is degenerate — an algebra `(A, a)`
//!   for the pointed endofunctor must satisfy `a ∘ σ_A = id_A` (the CDL
//!   Def B.3 algebra diagram), and `a(None) = x` for **all** `x` forces `A`
//!   to be a singleton. A point that trivialises every algebra is
//!   deliberately not shipped; a non-degenerate point into the `Some` summand
//!   would need a canonical `A`, which a bare type parameter does not supply.
//! - [`crate::free_monad::tree_endo::TreeEndo<A>`] (`Type<X> = Either<A, (X, X)>`)
//!   admits the diagonal `σ_X(x) = Right((x, x))`, which **is** natural in
//!   `Set` — but it is not representable under [`Pure`]'s signature:
//!   `pure<T>(value: T)` receives `value` by move with no `Clone` bound (and
//!   an impl cannot add one), so `x` cannot be duplicated into both slots. A
//!   Rust-representability obstruction, not a naturality failure.
//!
//! [`crate::algebra::GroupActionEndo<G>`] (`Type<X> = (G, X)`) is the crate's
//! own pointed witness: `σ(x) = (G::identity(), x)` — the writer-functor
//! point. Its σ-naturality holds because `fmap` never touches the group slot.
//! The blanket impl also admits haft witnesses re-exported through the seam:
//! [`OptionWitness`](crate::endofunctor::OptionWitness) implements `Pure`
//! upstream (`pure(x) = Some(x)`), so it too is [`Pointed`] here; its
//! σ-naturality is law-checked alongside `GroupActionEndo`'s in
//! `tests/natural_pointed_laws.rs`.

use core::marker::PhantomData;

use crate::endofunctor::{EndoWitness, NaturalIso, Pure};

/// A **natural transformation** `α : F ⇒ G` between two endofunctors on `Set`
/// (Gavranović et al., ICML 2024, Def 1.5).
///
/// The single static method [`transform`](Self::transform) is the component
/// family `α_X : F(X) → G(X)`, uniform in `X`. As with [`Functor::fmap`](crate::endofunctor::Functor::fmap), the
/// witness is a zero-sized token and `transform` is called statically —
/// `W::transform(fx)`, never a value method.
///
/// # Naturality law
///
/// Implementors must guarantee, for every **pure** morphism `h : A → B`,
///
/// ```text
/// transform(F::fmap(fa, h)) == G::fmap(transform(fa), h)
/// ```
///
/// This is a documented obligation, machine-checked for the shipped instances
/// in `tests/natural_pointed_laws.rs`. As with the functor laws (see the
/// [`crate::endofunctor`] `FnMut` caveat), the law is stated for pure
/// (state-free) morphisms only: a stateful closure can observe a different call
/// order between the two legs and that divergence is an artefact of the
/// closure, not a non-natural transformation.
pub trait NaturalTransformation<F: EndoWitness, G: EndoWitness> {
    /// The component `α_X : F(X) → G(X)` of the natural transformation at the
    /// object `X = T`.
    fn transform<T>(fa: F::Type<T>) -> G::Type<T>;
}

/// Adapter witness turning a [`NaturalIso<F, G>`] into the forward natural
/// transformation `F ⇒ G` (its `to_target` leg).
///
/// See the [module docs](self) for why the two iso directions are carried by
/// distinct adapter types rather than blanket impls.
pub struct IsoForward<W>(PhantomData<W>);

/// Adapter witness turning a [`NaturalIso<F, G>`] into the backward natural
/// transformation `G ⇒ F` (its `to_source` leg).
///
/// Note the direction: `IsoForward<W>` implements `NaturalTransformation<F, G>`
/// whereas `IsoBackward<W>` implements `NaturalTransformation<G, F>`, both for
/// the same `W: NaturalIso<F, G>`.
pub struct IsoBackward<W>(PhantomData<W>);

impl<W, F, G> NaturalTransformation<F, G> for IsoForward<W>
where
    W: NaturalIso<F, G>,
    F: EndoWitness,
    G: EndoWitness,
{
    fn transform<T>(fa: F::Type<T>) -> G::Type<T> {
        // `Satisfies<F::Constraint>` / `Satisfies<G::Constraint>` discharge
        // automatically: both constraints are `NoConstraint` (an `EndoWitness`
        // invariant) and `impl<T> Satisfies<NoConstraint> for T` is blanket.
        W::to_target(fa)
    }
}

impl<W, F, G> NaturalTransformation<G, F> for IsoBackward<W>
where
    W: NaturalIso<F, G>,
    F: EndoWitness,
    G: EndoWitness,
{
    fn transform<T>(ga: G::Type<T>) -> F::Type<T> {
        W::to_source(ga)
    }
}

/// A **pointed endofunctor** `(F, σ)` on `Set` (CDL Def B.3, Appendix B.1): an
/// endofunctor together with a natural transformation `σ : id ⇒ F`.
///
/// The point `σ` is exactly haft's [`Pure`] (`σ_X(x) = F::pure(x)`, the
/// natural transformation `η : Id → F`). This is a blanket-implemented marker,
/// mirroring [`EndoWitness`]: any endofunctor that also implements `Pure<Self>`
/// is pointed automatically, so instances never name `Pointed`.
///
/// # σ-naturality law
///
/// Implementors must guarantee, for every **pure** morphism `f : A → B`,
///
/// ```text
/// Self::fmap(Self::pure(x), f) == Self::pure(f(x))
/// ```
///
/// i.e. `σ` commutes with `fmap` (`F(f) ∘ σ_A = σ_B ∘ f`). This is a
/// documented obligation, machine-checked for the shipped instance in
/// `tests/natural_pointed_laws.rs`. See the [module docs](self) for why
/// `ListEndo` / `TreeEndo` are *not* pointed.
pub trait Pointed: EndoWitness + Pure<Self> {}

impl<T: EndoWitness + Pure<T>> Pointed for T {}
