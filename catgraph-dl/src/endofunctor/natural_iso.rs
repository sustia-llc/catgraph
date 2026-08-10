//! Natural isomorphism between two arity-1 witnesses, plus its law helpers.
//!
//! The laws live on the [`NaturalIso`] rustdoc — this module is private and
//! re-exported through [`crate::endofunctor`], so anything stated here alone
//! would not render.

use crate::endofunctor::{Functor, HKT};

/// Natural isomorphism between the arity-1 witnesses `F` and `G`: an
/// isomorphism `F::Type<T> ↔ G::Type<T>` that holds for every `T` and commutes
/// with `fmap`.
///
/// `From`/`Into` cannot express it — the two sides are *type constructors*, and
/// a witness is a zero-sized marker with no values to convert. The
/// [`IsoForward`](crate::natural::IsoForward) /
/// [`IsoBackward`](crate::natural::IsoBackward) adapters turn either leg into a
/// first-class [`NaturalTransformation`](crate::natural::NaturalTransformation).
///
/// # Laws
///
/// For every `T` and every `h : T → U`:
///
/// 1. **Round-trip identity**, both directions independently —
///    `to_source(to_target(fa)) == fa` and `to_target(to_source(ga)) == ga`.
/// 2. **Naturality** — `to_target(F::fmap(fa, h)) == G::fmap(to_target(fa), h)`.
///
/// [`assert_natural_iso_round_trip`] and [`assert_natural_iso_naturality`] check
/// them against caller-supplied samples.
pub trait NaturalIso<F, G>
where
    F: HKT,
    G: HKT,
{
    /// Maps `F::Type<T>` to `G::Type<T>` while preserving structure.
    fn to_target<T>(fa: F::Type<T>) -> G::Type<T>;

    /// Reverse of [`to_target`](Self::to_target).
    fn to_source<T>(ga: G::Type<T>) -> F::Type<T>;
}

/// Asserts the round-trip identity law for a [`NaturalIso<F, G>`] impl in both
/// directions independently.
///
/// Both checks are necessary. If `ga` were derived from `fa` via `to_target`,
/// the reverse check would only exercise the subset of `G::Type<T>` reachable
/// through the forward map, and a `to_source` that collapses distinct `G` values
/// onto the same `F` would pass undetected. Callers supply an independent `ga`
/// so such witnesses cannot slip through.
///
/// # Panics
///
/// Panics if either round trip fails to return the original value.
pub fn assert_natural_iso_round_trip<W, F, G, T>(fa: F::Type<T>, ga: G::Type<T>)
where
    W: NaturalIso<F, G>,
    F: HKT,
    G: HKT,
    F::Type<T>: Clone + PartialEq + core::fmt::Debug,
    G::Type<T>: Clone + PartialEq + core::fmt::Debug,
{
    let g_from_f: G::Type<T> = W::to_target(fa.clone());
    let fa_back: F::Type<T> = W::to_source(g_from_f);
    assert_eq!(
        fa, fa_back,
        "NaturalIso round-trip F -> G -> F failed: original {fa:?} differs from to_source(to_target(original))"
    );

    let f_from_g: F::Type<T> = W::to_source(ga.clone());
    let ga_back: G::Type<T> = W::to_target(f_from_g);
    assert_eq!(
        ga, ga_back,
        "NaturalIso round-trip G -> F -> G failed: original {ga:?} differs from to_target(to_source(original))"
    );
}

/// Asserts the naturality law for a [`NaturalIso<F, G>`] impl: the iso commutes
/// with `fmap` from both witnesses' [`Functor`] impls.
///
/// Checks `to_target(F::fmap(fa, h)) == G::fmap(to_target(fa), h)` for the
/// supplied `fa` and `h`. The symmetric law via `to_source` follows from forward
/// naturality together with the round-trip law; deep coverage composes this with
/// [`assert_natural_iso_round_trip`].
///
/// # Panics
///
/// Panics if the two legs of the naturality square disagree.
pub fn assert_natural_iso_naturality<W, F, G, A, B, Func>(fa: F::Type<A>, h: Func)
where
    W: NaturalIso<F, G>,
    F: HKT + Functor<F>,
    G: HKT + Functor<G>,
    F::Type<A>: Clone,
    G::Type<B>: PartialEq + core::fmt::Debug,
    Func: FnMut(A) -> B + Clone,
{
    let lhs: G::Type<B> = W::to_target(<F as Functor<F>>::fmap(fa.clone(), h.clone()));
    let rhs: G::Type<B> = <G as Functor<G>>::fmap(W::to_target(fa), h);
    assert_eq!(
        lhs, rhs,
        "NaturalIso naturality failed: to_target(F::fmap(fa, h)) != G::fmap(to_target(fa), h)"
    );
}
