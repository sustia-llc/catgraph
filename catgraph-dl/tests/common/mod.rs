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
use catgraph_dl::para::{Actegory, DirectSum, F64Actegory, F64Module, MonoidalCategory};
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

/// Shorthand for the object-level tensor GAT of a monoidal category `M`, used
/// to spell the nested turbofish/annotation types in
/// [`assert_monoidal_coherence`] without the fully-qualified
/// `<M as MonoidalCategory>::Tensor<..>` at every site.
type Ten<M, A, B> = <M as MonoidalCategory>::Tensor<A, B>;

/// Assert the **pentagon** and **triangle** coherence laws (plus unitor
/// sanity) for **any** [`MonoidalCategory`] `M` on four sample values. Mac
/// Lane's coherence theorem; CDL §3.1 (the parameter category `M` of
/// `Para(M, C)` is monoidal), and — for the [`DirectSum`] carrier
/// ([`F64Monoidal`](catgraph_dl::para::F64Monoidal)) — CDL Definition E.2
/// (actegory coherence) / Example G.3 (the cartesian `⊕` structure of real
/// vector spaces used by gradient-based-learning `Para(…)` constructions).
///
/// Generic over the trait: the `α ⊗ id` / `id ⊗ α` pentagon and triangle legs
/// are expressed through [`MonoidalCategory::tensor_morphisms`] (the
/// applying-form morphism tensor, issue #65), so the **one** checker serves
/// both the `(Set, ×, 1)` tuple carrier
/// ([`SetMonoidal`](catgraph_dl::para::SetMonoidal) and any
/// `SetCategoryDefaults`-flavoured ZST, `Tensor<A, B> = (A, B)`, `Unit = ()`)
/// and the [`DirectSum`] carrier ([`F64Monoidal`](catgraph_dl::para::F64Monoidal),
/// `Tensor<A, B> = DirectSum<A, B>`, `Unit = ()`) without spelling the legs by
/// hand per instance. For both shipped instances the associator/unitors are
/// exact isomorphisms (bona-fide bijections, not "up to iso"), so the laws
/// hold on the nose for arbitrary object types — the caller's `A`/`B`/`C`/`D`
/// samples stand in for objects.
///
/// - **Pentagon** — on `((A⊗B)⊗C)⊗D` the two routes to `A⊗(B⊗(C⊗D))` agree:
///   - route 1: `associate` at `(A⊗B, C, D)` then `associate` at `(A, B, C⊗D)`;
///   - route 2: `associate(A, B, C) ⊗ id_D` (via `tensor_morphisms`), then
///     `associate` at `(A, B⊗C, D)`, then `id_A ⊗ associate(B, C, D)` (via
///     `tensor_morphisms`).
///
///   Two-routes-agree **is** the pentagon law (there is no carrier-agnostic
///   "concrete re-association" target to compare against, unlike the earlier
///   tuple-only checker).
/// - **Triangle** — on `(A⊗I)⊗B`: `ρ ⊗ id_B` (via `tensor_morphisms`) equals
///   `id_A ⊗ λ` after `associate` — both collapse the unit.
/// - **Unitor sanity** — `left_unitor(I ⊗ a) == a` and
///   `right_unitor(a ⊗ I) == a`.
pub fn assert_monoidal_coherence<M, A, B, C, D>(m: &M, a: A, b: B, c: C, d: D)
where
    M: MonoidalCategory,
    A: Clone + PartialEq + core::fmt::Debug,
    B: Clone,
    C: Clone,
    D: Clone,
    Ten<M, A, Ten<M, B, Ten<M, C, D>>>: PartialEq + core::fmt::Debug,
    Ten<M, A, B>: PartialEq + core::fmt::Debug,
{
    // --- Pentagon ---------------------------------------------------------
    // Two independent copies of the fully-left-nested start ((a⊗b)⊗c)⊗d;
    // route 1 and route 2 each consume one.
    let start1 = m.tensor_objects(
        m.tensor_objects(m.tensor_objects(a.clone(), b.clone()), c.clone()),
        d.clone(),
    );
    let start2 = m.tensor_objects(
        m.tensor_objects(m.tensor_objects(a.clone(), b.clone()), c.clone()),
        d.clone(),
    );

    // Route 1 (bottom): associate at (A⊗B, C, D), then at (A, B, C⊗D).
    let r1_mid = m.associate::<Ten<M, A, B>, C, D>(start1); // (a⊗b) ⊗ (c⊗d)
    let route1 = m.associate::<A, B, Ten<M, C, D>>(r1_mid); // a ⊗ (b ⊗ (c⊗d))

    // Route 2 (top): (associate(A,B,C) ⊗ id_D) via tensor_morphisms, then
    // associate(A, B⊗C, D), then (id_A ⊗ associate(B,C,D)) via tensor_morphisms.
    let step_aid = m.tensor_morphisms(
        start2,
        |abc: Ten<M, Ten<M, A, B>, C>| m.associate::<A, B, C>(abc),
        |d: D| d,
    ); // (a ⊗ (b⊗c)) ⊗ d
    let step_mid = m.associate::<A, Ten<M, B, C>, D>(step_aid); // a ⊗ ((b⊗c) ⊗ d)
    let route2 = m.tensor_morphisms(
        step_mid,
        |a: A| a,
        |bcd: Ten<M, Ten<M, B, C>, D>| m.associate::<B, C, D>(bcd),
    ); // a ⊗ (b ⊗ (c⊗d))

    assert_eq!(route1, route2, "monoidal pentagon: two routes agree");

    // --- Triangle ---------------------------------------------------------
    // Two copies of (a ⊗ I) ⊗ b — route A and route B each consume one.
    let tri_start_a = m.tensor_objects(m.tensor_objects(a.clone(), m.unit()), b.clone());
    let tri_start_b = m.tensor_objects(m.tensor_objects(a.clone(), m.unit()), b.clone());

    // Route A: (ρ ⊗ id_B) via tensor_morphisms.
    let route_a = m.tensor_morphisms(
        tri_start_a,
        |au: Ten<M, A, M::Unit>| m.right_unitor::<A>(au),
        |b: B| b,
    ); // a ⊗ b

    // Route B: associate then (id_A ⊗ λ) via tensor_morphisms.
    let tri_assoc = m.associate::<A, M::Unit, B>(tri_start_b); // a ⊗ (I ⊗ b)
    let route_b = m.tensor_morphisms(
        tri_assoc,
        |a: A| a,
        |ub: Ten<M, M::Unit, B>| m.left_unitor::<B>(ub),
    ); // a ⊗ b

    assert_eq!(route_a, route_b, "monoidal triangle: two routes agree");

    // --- Unitor sanity ----------------------------------------------------
    assert_eq!(
        m.left_unitor::<A>(m.tensor_objects(m.unit(), a.clone())),
        a,
        "left unitor λ(I ⊗ a) = a"
    );
    assert_eq!(
        m.right_unitor::<A>(m.tensor_objects(a.clone(), m.unit())),
        a,
        "right unitor ρ(a ⊗ I) = a"
    );
}

/// Assert the `R`-module axioms for [`F64Module`] on one sample coordinate
/// vector `coords` and one scalar `r` — the identities that make
/// `deep_causality_num`'s `Zero` / `One` load-bearing (issue #36).
///
/// - **Additive identity** (`Zero`): `v + 0 = v` and `0 + v = v`.
/// - **Scalar unit** (`One`): `1 · v = v`.
/// - **Scalar zero** (`Zero`): `0 · v = 0` (the zero module of the same
///   dimension).
///
/// Equality is `f64` `PartialEq`, which identifies `-0.0` and `+0.0` — these
/// identities hold under that equality for finite inputs, but signed-zero bit
/// patterns are **not** preserved (`-0.0 + 0.0 = +0.0`; `0.0 · (-1.0) = -0.0`),
/// so this helper certifies `PartialEq`-equality, not bit-exactness. See the
/// "Float honesty" note on `F64Module`.
/// - **Basis coherence** (`One` / `Zero`): each standard basis vector `eᵢ` has
///   `1` at `i` and `0` elsewhere; scaling `eᵢ` by `r` places `r` at `i`.
///
/// CDL Definition E.2 (objects of the acted-on category `C`) / Example G.3.
pub fn assert_f64_module_axioms(coords: Vec<f64>, r: f64) {
    let v = F64Module::new(coords);
    let n = v.dim();
    let zero = F64Module::zeros(n);

    // Additive identity — both sides, exact.
    assert_eq!(
        v.add(&zero).as_ref(),
        Some(&v),
        "additive identity v + 0 = v"
    );
    assert_eq!(
        zero.add(&v).as_ref(),
        Some(&v),
        "additive identity 0 + v = v"
    );

    // Scalar unit and scalar zero — exact.
    assert_eq!(v.scale(1.0), v, "scalar unit 1 · v = v");
    assert_eq!(v.scale(0.0), zero, "scalar zero 0 · v = 0");

    // Dimension mismatch guards addition.
    if n > 0 {
        let shorter = F64Module::zeros(n - 1);
        assert_eq!(
            v.add(&shorter),
            None,
            "addition rejects a dimension mismatch"
        );
    }

    // Basis coherence — `eᵢ` has 1 at i, 0 elsewhere; `r · eᵢ` has r at i.
    for i in 0..n {
        let e_i = F64Module::basis(n, i).expect("i < n so basis is defined");
        for (j, &x) in e_i.as_slice().iter().enumerate() {
            let expected = if j == i { 1.0 } else { 0.0 };
            assert_eq!(x, expected, "basis e_{i} coordinate {j}");
        }
        let scaled = e_i.scale(r);
        assert_eq!(scaled.as_slice()[i], r * 1.0, "r · e_{i} has r at slot {i}");
    }
    // `basis` is out of range past the dimension.
    assert_eq!(
        F64Module::basis(n, n),
        None,
        "basis rejects i == dim (out of range)"
    );
}

/// Assert the concrete `⊕`-monoid laws for [`F64Module`] direct sum on three
/// samples — the coordinate-level witness that `(FinReal, ⊕, R⁰)` is monoidal.
///
/// - **Dimensions add**: `dim(u ⊕ v) = dim(u) + dim(v)`.
/// - **Concatenation**: coordinates of `u ⊕ v` are `u`'s followed by `v`'s.
/// - **Unit laws**: `R⁰ ⊕ v = v = v ⊕ R⁰` (zero-dim module is the unit).
/// - **Associativity**: `(u ⊕ v) ⊕ w = u ⊕ (v ⊕ w)` on the nose.
/// - **`DirectSum::flatten` agrees** with `direct_sum` on the generic
///   [`Actegory::act`] result.
///
/// CDL Example E.4 (self-action) / Example G.3 (`Rᵐ ⊕ Rⁿ = Rᵐ⁺ⁿ`).
pub fn assert_direct_sum_monoid(u: Vec<f64>, v: Vec<f64>, w: Vec<f64>) {
    let (mu, mv, mw) = (
        F64Module::new(u.clone()),
        F64Module::new(v.clone()),
        F64Module::new(w),
    );
    let unit = F64Module::zero_dim();

    // Dimensions add; coordinates concatenate.
    let uv = mu.clone().direct_sum(mv.clone());
    assert_eq!(uv.dim(), mu.dim() + mv.dim(), "⊕ dimensions add");
    let expected: Vec<f64> = u.iter().chain(&v).copied().collect();
    assert_eq!(uv.as_slice(), expected.as_slice(), "⊕ concatenates blocks");

    // Unit laws (zero-dim module is the ⊕-unit).
    assert_eq!(
        unit.clone().direct_sum(mv.clone()),
        mv,
        "left unit R⁰ ⊕ v = v"
    );
    assert_eq!(
        mv.clone().direct_sum(unit.clone()),
        mv,
        "right unit v ⊕ R⁰ = v"
    );

    // Associativity on the nose.
    let left = mu.clone().direct_sum(mv.clone()).direct_sum(mw.clone());
    let right = mu.clone().direct_sum(mv.clone().direct_sum(mw.clone()));
    assert_eq!(left, right, "⊕ associativity (u ⊕ v) ⊕ w = u ⊕ (v ⊕ w)");

    // `DirectSum::flatten` realises the same concatenation as the generic
    // `act` (a `DirectSum` pair) collapsed via `flatten`.
    let acteg = F64Actegory::new();
    let generic: DirectSum<F64Module, F64Module> = acteg.act(mu.clone(), mv.clone());
    assert_eq!(
        generic.flatten(),
        mu.direct_sum(mv),
        "act(p, x).flatten() == p ⊕ x"
    );
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
