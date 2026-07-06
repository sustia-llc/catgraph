//! Shared test scaffolding for the catgraph-dl integration tests.
//!
//! Two reusables live here:
//!
//! - [`assert_functor_laws`] — a witness-generic identity + composition law
//!   check, fed per-witness sample values (`tests/functor_laws.rs`).
//! - [`UnitEndo`] — the single trivial endofunctor with `Type<X> = ()`, used as
//!   a type-level placeholder wherever a test needs "an endofunctor witness"
//!   with no semantics (`tests/scaffold_smoke.rs`, `tests/free_monad_bijections.rs`).
//!   Per-file tag types alias it back to descriptive names.
//!
//! `#![allow(dead_code)]`: each integration-test binary `mod common;`-includes
//! this whole module but uses only the parts it needs, so the unused items are
//! expected per compilation unit.
#![allow(dead_code)]

use core::marker::PhantomData;

use catgraph_dl::{EndoWitness, Functor, HKT, NoConstraint, Satisfies};

/// Assert the functor **identity** and **composition** laws for the witness
/// `F` on a single sample `fx : F::Type<i32>`.
///
/// - Identity: `fmap(fx, id) == fx`.
/// - Composition: `fmap(fmap(fx, f), g) == fmap(fx, g ∘ f)`.
///
/// The morphisms are pure `i32 -> i32` maps using wrapping arithmetic, so both
/// legs stay equal across the full `i32` range without overflow panics (see the
/// pure-morphism caveat in `catgraph_dl::endofunctor`). Cites the
/// `deep_causality_haft` `Functor` law docs and Gavranović et al., ICML 2024.
pub fn assert_functor_laws<F>(fx: F::Type<i32>)
where
    F: EndoWitness,
    F::Type<i32>: Clone + PartialEq + core::fmt::Debug,
{
    // Identity law.
    let id = F::fmap(fx.clone(), |x| x);
    assert_eq!(id, fx, "functor identity law");

    // Composition law: fmap(fmap(fx, f), g) == fmap(fx, g ∘ f).
    let f = |v: i32| v.wrapping_add(3);
    let g = |v: i32| v.wrapping_mul(2);
    let seq = F::fmap(F::fmap(fx.clone(), f), g);
    let fused = F::fmap(fx, |v| g(f(v)));
    assert_eq!(seq, fused, "functor composition law");
}

/// A trivial endofunctor witness with `Type<X> = ()` — "no recursive slot at
/// all". `Tag` is a phantom discriminator so each test file can alias it back
/// to a descriptive name (`StreamEndo`, `TrivialEndo`, …) without colliding.
///
/// `fmap` returns its (unit) argument, so the body is total without spelling a
/// bare `-> ()`.
pub struct UnitEndo<Tag>(PhantomData<Tag>);

impl<Tag> HKT for UnitEndo<Tag> {
    type Constraint = NoConstraint;
    type Type<X> = ();
}

impl<Tag> Functor<Self> for UnitEndo<Tag> {
    fn fmap<X, Y, Func>(m_a: <Self as HKT>::Type<X>, _f: Func) -> <Self as HKT>::Type<Y>
    where
        X: Satisfies<NoConstraint>,
        Y: Satisfies<NoConstraint>,
        Func: FnMut(X) -> Y,
    {
        m_a
    }
}
