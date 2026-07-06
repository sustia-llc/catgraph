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

use catgraph_dl::{
    Container, EndoWitness, Functor, HKT, NaturalTransformation, NoConstraint, Pointed, Satisfies,
};

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

/// Assert the **naturality** law for a natural transformation `N : F ⇒ G` on a
/// single sample `fa : F::Type<i32>`.
///
/// Checks `transform(F::fmap(fa, h)) == G::fmap(transform(fa), h)` for a pure
/// morphism `h` (wrapping arithmetic, so both legs stay equal across the full
/// `i32` range — same pure-morphism caveat as [`assert_functor_laws`]).
/// Gavranović et al., ICML 2024, Def 1.5.
pub fn assert_natural_transformation_naturality<N, F, G>(fa: F::Type<i32>)
where
    N: NaturalTransformation<F, G>,
    F: EndoWitness,
    G: EndoWitness,
    F::Type<i32>: Clone,
    G::Type<i32>: PartialEq + core::fmt::Debug,
{
    let h = |v: i32| v.wrapping_add(3);
    let lhs = N::transform(F::fmap(fa.clone(), h));
    let rhs = G::fmap(N::transform(fa), h);
    assert_eq!(lhs, rhs, "natural transformation naturality law");
}

/// Assert the **σ-naturality** law for a pointed endofunctor `(F, σ)` on a
/// single sample `x : i32`.
///
/// Checks `F::fmap(F::pure(x), f) == F::pure(f(x))` for a pure morphism `f`
/// (`σ` commutes with `fmap`). CDL Def B.3.
pub fn assert_pointed_naturality<F>(x: i32)
where
    F: Pointed,
    F::Type<i32>: PartialEq + core::fmt::Debug,
{
    let f = |v: i32| v.wrapping_mul(2);
    let lhs = F::fmap(F::pure(x), f);
    let rhs = F::pure(f(x));
    assert_eq!(lhs, rhs, "pointed σ-naturality law");
}

/// Assert the **container laws** for a witness `F` on a single sample
/// `fx : F::Type<i32>`.
///
/// - Round-trip: `recompose(decompose(fx)) == Some(fx)`.
/// - Arity coherence: `decompose(fx).1.len() == arity(shape)`, and `recompose`
///   rejects (returns `None`) a contents `Vec` whose length ≠ arity — probed
///   in **both** directions (one too many, and one too few when arity > 0),
///   since the law is an iff.
/// - `fmap` coherence: `decompose(F::fmap(fx, f)) == (shape, contents.map(f))`
///   — shape fixed, contents mapped in position order.
///
/// Abbott–Altenkirch–Ghani 2003, via CDL. `f` is a pure morphism.
pub fn assert_container_laws<F>(fx: F::Type<i32>)
where
    F: Container,
    F::Type<i32>: Clone + PartialEq + core::fmt::Debug,
    F::Shape: Clone,
{
    let f = |v: i32| v.wrapping_add(1);

    // One decompose serves every probe below. `Shape: Clone` is a helper-side
    // bound only (all shipped shapes are `Clone`), not a `Container`
    // requirement.
    let (shape, contents) = F::decompose(fx.clone());

    // Arity coherence (decompose length).
    let arity = F::arity(&shape);
    assert_eq!(
        contents.len(),
        arity,
        "container arity coherence: decompose length == arity"
    );

    // fmap coherence: shape fixed, contents mapped in position order.
    let (shape_after, contents_after) = F::decompose(F::fmap(fx.clone(), f));
    assert_eq!(
        shape, shape_after,
        "container fmap coherence: shape fixed under fmap"
    );
    let mapped_contents: Vec<i32> = contents.iter().copied().map(f).collect();
    assert_eq!(
        contents_after, mapped_contents,
        "container fmap coherence: contents mapped in position order"
    );

    // Round-trip: recompose ∘ decompose == Some.
    let rebuilt = F::recompose(shape.clone(), contents.clone());
    assert_eq!(
        rebuilt,
        Some(fx),
        "container round-trip: recompose(decompose(fx)) == Some(fx)"
    );

    // Arity coherence (recompose rejection): the law is an *iff*, so probe
    // both directions of `len != arity` — one content too many...
    let mut over = contents.clone();
    over.push(0);
    assert!(
        F::recompose(shape.clone(), over).is_none(),
        "container arity coherence: recompose rejects length > arity"
    );
    // ...and, for shapes with at least one slot, one too few.
    if arity > 0 {
        let mut under = contents;
        under.pop();
        assert!(
            F::recompose(shape, under).is_none(),
            "container arity coherence: recompose rejects length < arity"
        );
    }
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
