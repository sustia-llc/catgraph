//! Shared test scaffolding for the catgraph-dl integration tests.
//!
//! Reusables:
//!
//! - [`assert_functor_laws`] — a witness-generic identity + composition law
//!   check, fed per-witness sample values (`tests/functor_laws.rs`).
//! - [`UnitEndo`] — the single trivial endofunctor with `Type<X> = ()`, used as
//!   a type-level placeholder wherever a test needs "an endofunctor witness"
//!   with no semantics (`tests/scaffold_smoke.rs`, `tests/free_monad_bijections.rs`).
//!   Per-file tag types alias it back to descriptive names.
//! - The law helpers for the #41/#40 surfaces
//!   ([`assert_natural_transformation_naturality`], [`assert_pointed_naturality`],
//!   [`assert_container_laws`], [`assert_monoidal_coherence`]).
//! - The canonical `Z2`-on-`Vec<f64>` GDL fixtures (CDL Example 2.6) shared by
//!   `tests/algebra_homomorphisms.rs` and `tests/monad_algebra_laws.rs`:
//!   [`negation_action`] / [`trivial_action`] / [`abs_map`] / [`first_coord`],
//!   the [`Z2Endo`] / [`Z2Action`] / [`VecMap`] aliases, and the NaN-free
//!   [`finite_f64`] proptest strategy.
//!
//! `#![allow(dead_code)]`: each integration-test binary `mod common;`-includes
//! this whole module but uses only the parts it needs, so the unused items are
//! expected per compilation unit.
#![allow(dead_code)]

use core::marker::PhantomData;

use catgraph_dl::algebra::{GroupActionEndo, Z2Group};
use catgraph_dl::para::{MonoidalCategory, SetCategoryDefaults};
use catgraph_dl::{
    Container, EndoWitness, Functor, HKT, NaturalTransformation, NoConstraint, Pointed, Satisfies,
};

use proptest::prelude::*;

/// The `Z2` group-action endofunctor `Z2 × −` used by the algebra law suites.
pub type Z2Endo = GroupActionEndo<Z2Group>;

/// Structure-map type: a `Z2`-action on `Vec<f64>`.
pub type Z2Action = fn((Z2Group, Vec<f64>)) -> Vec<f64>;

/// Map type for `Vec<f64> → Vec<f64>` homomorphism candidates.
pub type VecMap = fn(Vec<f64>) -> Vec<f64>;

/// The canonical `Z2`-action on `Vec<f64>` by pointwise negation:
/// `g ▶ x = if g { −x } else { x }`. A genuine group action, hence a monad
/// algebra of `Z2 × −`; source algebra of every GDL-recovery fixture
/// (CDL Example 2.6).
pub fn negation_action((g, x): (Z2Group, Vec<f64>)) -> Vec<f64> {
    if g.0 {
        x.into_iter().map(|v| -v).collect()
    } else {
        x
    }
}

/// The trivial `Z2`-action `g ▶ y = y` — target algebra of the GDL-invariance
/// shape (CDL Example 2.6).
pub fn trivial_action((_g, y): (Z2Group, Vec<f64>)) -> Vec<f64> {
    y
}

/// Pointwise absolute value — the `Z2`-invariant GDL-recovery map
/// (CDL Example 2.6): `|−x_i| = |x_i|`, so it satisfies the equivariance
/// square between the negation and trivial actions.
pub fn abs_map(x: Vec<f64>) -> Vec<f64> {
    x.into_iter().map(f64::abs).collect()
}

/// First-coordinate projection wrapped as a singleton `f(x) = vec![x[0]]` —
/// **not** `Z2`-equivariant (the canonical negative case). Panics on the empty
/// vector, so callers must supply a non-empty sample.
pub fn first_coord(x: Vec<f64>) -> Vec<f64> {
    vec![x[0]]
}

/// Finite `f64` proptest strategy — bounded range keeps `PartialEq` meaningful
/// (no NaN).
pub fn finite_f64() -> impl Strategy<Value = f64> {
    -1e6f64..1e6f64
}

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

/// Assert the **pentagon** and **triangle** coherence laws (plus unitor
/// sanity) for a `(Set, ×, 1)`-flavoured monoidal category `M` on four sample
/// values. Mac Lane's coherence theorem; CDL §3.1 cites the monoidal structure
/// of the parameter category.
///
/// `M: SetCategoryDefaults` pins the blanket bodies `Tensor<A, B> = (A, B)` and
/// `Unit = ()`, so the two GATs normalise to concrete Rust tuples and the
/// coherence isomorphisms are exact (bona-fide bijections in `Set`, not "up to
/// iso"). The trait carries **no morphism-tensor operation** (see the
/// `MonoidalCategory` rustdoc), so wherever a route needs `α ⊗ id` or
/// `id ⊗ α` the component manipulation is spelled **manually** on the tuples.
///
/// - **Pentagon** — on `(((a, b), c), d)` the two routes from
///   `((A⊗B)⊗C)⊗D` to `A⊗(B⊗(C⊗D))` agree:
///   - route 1: `associate` at `(A⊗B, C, D)` then `associate` at `(A, B, C⊗D)`;
///   - route 2: `(associate(A, B, C)) ⊗ id_D` (manual), then `associate` at
///     `(A, B⊗C, D)`, then `id_A ⊗ associate(B, C, D)` (manual).
/// - **Triangle** — on `((a, ()), b)`: `right_unitor ⊗ id_B` (manual) equals
///   `id_A ⊗ left_unitor` after `associate` — both yield `(a, b)`.
/// - **Unitor sanity** — `left_unitor(((), a)) == a`,
///   `right_unitor((a, ())) == a`, `unit() == ()`.
pub fn assert_monoidal_coherence<M: SetCategoryDefaults>(m: &M, a: i32, b: u8, c: i64, d: bool) {
    // --- Pentagon ---------------------------------------------------------
    // Route 1 (bottom): associate at (A⊗B, C, D), then at (A, B, C⊗D).
    let start = (((a, b), c), d);
    let r1_mid = m.associate::<(i32, u8), i64, bool>(start); // ((a, b), (c, d))
    let route1 = m.associate::<i32, u8, (i64, bool)>(r1_mid); // (a, (b, (c, d)))

    // Route 2 (top): (associate(A,B,C) ⊗ id_D), then associate(A, B⊗C, D),
    // then (id_A ⊗ associate(B,C,D)). The `⊗ id` legs are spelled manually.
    let (inner_abc, d2) = start; // inner_abc = ((a, b), c)
    let assoc_abc = m.associate::<i32, u8, i64>(inner_abc); // (a, (b, c))
    let step1 = (assoc_abc, d2); // ((a, (b, c)), d)
    let step2 = m.associate::<i32, (u8, i64), bool>(step1); // (a, ((b, c), d))
    let (a3, bcd) = step2; // a3 = a, bcd = ((b, c), d)
    let route2 = (a3, m.associate::<u8, i64, bool>(bcd)); // (a, (b, (c, d)))

    assert_eq!(route1, route2, "monoidal pentagon: two routes agree");
    assert_eq!(
        route1,
        (a, (b, (c, d))),
        "monoidal pentagon: exact re-association"
    );

    // --- Triangle ---------------------------------------------------------
    // Route A: (right_unitor ⊗ id_B) applied manually to ((a, ()), b).
    let tri_start = ((a, ()), b);
    let (a_unit, b_a) = tri_start; // a_unit = (a, ()), b_a = b
    let route_a = (m.right_unitor::<i32>(a_unit), b_a); // (a, b)

    // Route B: associate then (id_A ⊗ left_unitor).
    let tri_assoc = m.associate::<i32, (), u8>(tri_start); // (a, ((), b))
    let (a_b, unit_b) = tri_assoc; // a_b = a, unit_b = ((), b)
    let route_b = (a_b, m.left_unitor::<u8>(unit_b)); // (a, b)

    assert_eq!(route_a, route_b, "monoidal triangle: two routes agree");
    assert_eq!(route_a, (a, b), "monoidal triangle: exact unitor collapse");

    // --- Unitor sanity ----------------------------------------------------
    assert_eq!(
        m.left_unitor::<i32>(((), a)),
        a,
        "left unitor λ(((), a)) = a"
    );
    assert_eq!(
        m.right_unitor::<i32>((a, ())),
        a,
        "right unitor ρ((a, ())) = a"
    );
    assert_eq!(m.unit(), (), "monoidal unit is ()");
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
