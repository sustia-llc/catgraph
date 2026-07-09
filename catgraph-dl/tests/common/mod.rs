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
use catgraph_dl::para::{
    Actegory, DirectSum, F64Actegory, F64Module, F64Monoidal, MonoidalCategory, SetCategoryDefaults,
};
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

/// Assert the **pentagon** and **triangle** coherence laws (plus unitor
/// sanity) for the direct-sum monoidal category [`F64Monoidal`] on four sample
/// values. Mac Lane's coherence theorem; CDL Definition E.2 (actegory
/// coherence) / Example G.3 (the cartesian `⊕` structure of real vector
/// spaces).
///
/// The `DirectSum` analogue of [`assert_monoidal_coherence`]: [`F64Monoidal`]'s
/// tensor is the [`DirectSum`] carrier, not the tuple, so the
/// `SetCategoryDefaults`-bound tuple checker does not apply. The associator and
/// unitors are exact `DirectSum` re-associations (pure data movement, no `f64`
/// arithmetic), so the laws hold on the nose for arbitrary object types — the
/// sample types below (`i32`, `u8`, `i64`, `bool`) stand in for module objects.
/// As with the tuple checker, the trait carries **no morphism-tensor
/// operation**, so wherever a route needs `α ⊗ id` or `id ⊗ α` the component
/// manipulation is spelled **manually** on the `DirectSum` values.
pub fn assert_direct_sum_coherence(m: &F64Monoidal, a: i32, b: u8, c: i64, d: bool) {
    // --- Pentagon ---------------------------------------------------------
    // Route 1 (bottom): associate at (A⊕B, C, D), then at (A, B, C⊕D).
    let start = DirectSum(DirectSum(DirectSum(a, b), c), d);
    let r1_mid = m.associate::<DirectSum<i32, u8>, i64, bool>(start); // (a⊕b) ⊕ (c⊕d)
    let route1 = m.associate::<i32, u8, DirectSum<i64, bool>>(r1_mid); // a ⊕ (b ⊕ (c⊕d))

    // Route 2 (top): (associate(A,B,C) ⊗ id_D), then associate(A, B⊕C, D),
    // then (id_A ⊗ associate(B,C,D)). The `⊗ id` legs are spelled manually.
    let DirectSum(inner_abc, d2) = start; // inner_abc = (a⊕b) ⊕ c
    let assoc_abc = m.associate::<i32, u8, i64>(inner_abc); // a ⊕ (b ⊕ c)
    let step1 = DirectSum(assoc_abc, d2); // (a ⊕ (b⊕c)) ⊕ d
    let step2 = m.associate::<i32, DirectSum<u8, i64>, bool>(step1); // a ⊕ ((b⊕c) ⊕ d)
    let DirectSum(a3, bcd) = step2; // a3 = a, bcd = (b⊕c) ⊕ d
    let route2 = DirectSum(a3, m.associate::<u8, i64, bool>(bcd)); // a ⊕ (b ⊕ (c⊕d))

    assert_eq!(route1, route2, "direct-sum pentagon: two routes agree");
    assert_eq!(
        route1,
        DirectSum(a, DirectSum(b, DirectSum(c, d))),
        "direct-sum pentagon: exact re-association"
    );

    // --- Triangle ---------------------------------------------------------
    // Route A: (right_unitor ⊗ id_B) applied manually to (a ⊕ R⁰) ⊕ b.
    let tri_start = DirectSum(DirectSum(a, ()), b);
    let DirectSum(a_unit, b_a) = tri_start; // a_unit = a ⊕ R⁰, b_a = b
    let route_a = DirectSum(m.right_unitor::<i32>(a_unit), b_a); // a ⊕ b

    // Route B: associate then (id_A ⊗ left_unitor).
    let tri_assoc = m.associate::<i32, (), u8>(tri_start); // a ⊕ (R⁰ ⊕ b)
    let DirectSum(a_b, unit_b) = tri_assoc; // a_b = a, unit_b = R⁰ ⊕ b
    let route_b = DirectSum(a_b, m.left_unitor::<u8>(unit_b)); // a ⊕ b

    assert_eq!(route_a, route_b, "direct-sum triangle: two routes agree");
    assert_eq!(
        route_a,
        DirectSum(a, b),
        "direct-sum triangle: exact unitor collapse"
    );

    // --- Unitor sanity ----------------------------------------------------
    assert_eq!(
        m.left_unitor::<i32>(DirectSum((), a)),
        a,
        "left unitor λ(R⁰ ⊕ a) = a"
    );
    assert_eq!(
        m.right_unitor::<i32>(DirectSum(a, ())),
        a,
        "right unitor ρ(a ⊕ R⁰) = a"
    );
    assert_eq!(m.unit(), (), "monoidal unit is R⁰ ≅ ()");
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
