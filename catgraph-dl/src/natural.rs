//! [`NaturalTransformation<F, G>`] — a component family `α_X : F(X) → G(X)`
//! (CDL Def 1.5), a static method on a zero-sized witness — and [`Pointed`],
//! a pointed endofunctor `(F, σ)` with `σ = `[`Pure`] (CDL Def B.3).
//! [`IsoForward`] / [`IsoBackward`] adapt a [`NaturalIso<F, G>`] to each
//! direction. `Pointed` instances: [`crate::algebra::GroupActionEndo<G>`]
//! (`σ(x) = (G::identity(), x)`) and
//! [`OptionWitness`](crate::endofunctor::OptionWitness) (`σ(x) = Some(x)`);
//! `ListEndo` and `TreeEndo` ship no `Pure`.

use core::marker::PhantomData;

use crate::endofunctor::{EndoWitness, NaturalIso, Pure};

/// Natural transformation `α : F ⇒ G` (CDL Def 1.5): [`transform`](Self::transform)
/// is the component `α_X : F(X) → G(X)`. Naturality, required for every pure
/// `h : A → B`:
///
/// ```text
/// transform(F::fmap(fa, h)) == G::fmap(transform(fa), h)
/// ```
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
/// The point `σ` is exactly [`Pure`] (`σ_X(x) = F::pure(x)`, the
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
