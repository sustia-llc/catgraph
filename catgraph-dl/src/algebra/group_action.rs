//! Group-action endofunctor `F = G × −` and the `Z2` recovery example.
//!
//! CDL Example 2.4 / Example 2.6. Given a group `G`, the endofunctor
//! `F(X) = G × X` carries the structure of a *monad* whose algebras are
//! exactly **`G`-sets** (sets equipped with a left action of `G`). An
//! F-algebra homomorphism between two such algebras is then exactly a
//! **`G`-equivariant map** — the central concept of Geometric Deep
//! Learning.
//!
//! This module provides:
//!
//! - [`Group`] — abelian-or-otherwise group operation surface.
//! - [`Z2Group`] — the cyclic group of order 2 as a unit struct
//!   (`identity = false`, `compose = XOR`).
//! - [`GroupActionEndo<G>`] — the type-level witness for the endofunctor
//!   `F(X) = G × X`.
//!
//! ## CDL Example 2.6 (GDL recovery, in code)
//!
//! Two `Z2`-actions on `Vec<f64>` —
//!
//! - the canonical action `g ▶ x = if g { −x } else { x }`,
//! - and the *trivial* action `g ▶ x = x`.
//!
//! An F-algebra homomorphism between them is precisely a `Z2`-equivariant
//! map. The acceptance test [`tests/algebra_homomorphisms.rs`][test]
//! exhibits two concrete maps:
//!
//! - `f(x) = x[0]` — coordinate projection — fails the equivariance
//!   square (asymmetric under negation).
//! - `f(x) = x.iter().map(|v| v.abs()).collect()` — pointwise absolute
//!   value — satisfies the equivariance square because `|−x_i| = |x_i|`.
//!
//! [test]: ../../../../tests/algebra_homomorphisms.rs

use core::marker::PhantomData;

use crate::container::Container;
use crate::endofunctor::{Functor, HKT, Monad, NoConstraint, Pure, Satisfies};

/// A group with associative binary `compose` and identity `identity`.
///
/// Implementors must satisfy:
///
/// ```text
/// compose(identity(), g) = g                          (left identity)
/// compose(g, identity()) = g                          (right identity)
/// compose(compose(a, b), c) = compose(a, compose(b, c))   (associativity)
/// ```
///
/// Inverses are not required by this trait — many of the F-algebra
/// constructions in CDL §2 use only the monoid structure of `(G, ·, e)`.
/// Add a separate `GroupInverse` trait if needed for downstream proofs.
pub trait Group: Sized {
    /// The binary group operation `g1 · g2`.
    fn compose(g1: Self, g2: Self) -> Self;

    /// The identity element `e` of the group.
    fn identity() -> Self;
}

/// The cyclic group of order 2, represented as a Boolean.
///
/// `identity` is `false` (the additive identity in `Z/2Z`);
/// `compose` is XOR (the additive operation modulo 2). The non-trivial
/// element `true` is its own inverse — `g · g = e` for `g = true`.
///
/// Used in [`tests/algebra_homomorphisms.rs`][test] to instantiate the
/// canonical "negation" action of `Z2` on `Vec<f64>` and to exhibit the
/// GDL-equivariance recovery.
///
/// [test]: ../../../../tests/algebra_homomorphisms.rs
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Z2Group(pub bool);

impl Group for Z2Group {
    fn compose(g1: Self, g2: Self) -> Self {
        Self(g1.0 ^ g2.0)
    }

    fn identity() -> Self {
        Self(false)
    }
}

/// Type-level witness for the endofunctor `F(X) = G × X`.
///
/// CDL Example 2.4. The `Type<X>` GAT projects to the Rust tuple
/// `(G, X)` — the same encoding used in
/// [`tests/scaffold_smoke.rs`][smoke]'s `GroupActionEndo<G>` placeholder
/// (the placeholder was replaced wholesale by this real implementation).
///
/// `fmap(g, f)` lifts the morphism on the second slot only — the group
/// element is preserved untouched. This is the standard "constant on the
/// first factor" lifting for product endofunctors.
///
/// [smoke]: ../../../../tests/scaffold_smoke.rs
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GroupActionEndo<G>(PhantomData<G>);

impl<G> GroupActionEndo<G> {
    /// Construct a fresh `GroupActionEndo<G>` type witness.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<G> HKT for GroupActionEndo<G> {
    type Constraint = NoConstraint;
    type Type<X> = (G, X);
}

impl<G> Functor<Self> for GroupActionEndo<G> {
    fn fmap<X, Y, Func>(fx: (G, X), mut f: Func) -> (G, Y)
    where
        X: Satisfies<NoConstraint>,
        Y: Satisfies<NoConstraint>,
        Func: FnMut(X) -> Y,
    {
        let (g, x) = fx;
        (g, f(x))
    }
}

/// The writer-functor point `σ_X(x) = (e, x)` making `F(X) = G × X` a
/// **pointed endofunctor** (CDL Def B.3): the point pairs `x` with the group
/// identity `e = G::identity()`. σ-naturality
/// `fmap(pure(x), f) == pure(f(x))` holds because `fmap` preserves the group
/// slot untouched — both sides carry `e` and apply `f` to the second slot.
/// This is the crate's own inhabitant of [`crate::natural::Pointed`] (haft
/// witnesses re-exported through the seam, e.g. `OptionWitness`, are also
/// pointed via their upstream `Pure` impls); see that module for why
/// `ListEndo` / `TreeEndo` ship no point.
impl<G: Group> Pure<Self> for GroupActionEndo<G> {
    fn pure<X>(value: X) -> (G, X)
    where
        X: Satisfies<NoConstraint>,
    {
        (G::identity(), value)
    }
}

/// The **writer monad** over the monoid `(G, ·, e)`: `F(X) = G × X` with unit
/// `η = pure` (`x ↦ (e, x)`) and multiplication `μ = join` collapsing a nested
/// group pair via `compose` — `join((g1, (g2, x))) == (g1 · g2, x)`, exactly the
/// μ documented in `monad_algebra.rs`'s CDL Example 2.6 note (CDL Def 2.1 /
/// Ex 2.2). `bind((g, x), f)` runs `f(x) = (g2, y)` and accumulates the group
/// slot: `(g · g2, y)`.
///
/// The monad laws are discharged by the [`Group`] contract:
///
/// - **Left unit** `bind(pure(x), f) == f(x)` — `pure(x) = (e, x)`, so
///   `bind` yields `(e · g2, y) = (g2, y) = f(x)` by `G`'s left-identity law.
/// - **Right unit** `bind(m, pure) == m` — `bind((g, x), pure) = (g · e, x) =
///   (g, x)` by `G`'s right-identity law.
/// - **Associativity** `bind(bind(m, f), h) == bind(m, |x| bind(f(x), h))` —
///   both legs accumulate `g · g2 · g3` in the group slot; equality is `G`'s
///   associativity law.
///
/// These are the monad-algebra coherence obligations that
/// [`crate::algebra::MonadAlgebra::verify_unit_law`] /
/// [`verify_assoc_law`](crate::algebra::MonadAlgebra::verify_assoc_law) and the
/// [`MonadAlgebraHom`](crate::algebra::MonadAlgebraHom) verifiers machine-check
/// against concrete samples.
impl<G: Group> Monad<Self> for GroupActionEndo<G> {
    fn bind<X, Y, Func>(m_a: (G, X), mut f: Func) -> (G, Y)
    where
        X: Satisfies<NoConstraint>,
        Y: Satisfies<NoConstraint>,
        Func: FnMut(X) -> (G, Y),
    {
        let (g, x) = m_a;
        let (g2, y) = f(x);
        (G::compose(g, g2), y)
    }
}

/// Container presentation of `G × −` (Abbott–Altenkirch–Ghani 2003, via CDL).
/// There is a single position shape per group element: `Shape = G`, and every
/// shape has arity 1 (the single `X` slot). `G: PartialEq + Debug` so the shape
/// carries into the machine-checked container laws.
impl<G: PartialEq + core::fmt::Debug> Container for GroupActionEndo<G> {
    type Shape = G;

    fn arity(_shape: &Self::Shape) -> usize {
        1
    }

    fn decompose<X>(fx: (G, X)) -> (Self::Shape, Vec<X>) {
        let (g, x) = fx;
        (g, vec![x])
    }

    fn recompose<X>(shape: Self::Shape, contents: Vec<X>) -> Option<(G, X)> {
        // Arity 1: `TryFrom<Vec<X>> for [X; 1]` rejects any other length.
        let [x] = <[X; 1]>::try_from(contents).ok()?;
        Some((shape, x))
    }
}

#[cfg(test)]
mod tests {
    use super::{Functor, Group, GroupActionEndo, HKT, Z2Group};

    /// Confirms the `Z2` group laws (identity, associativity, self-inverse
    /// of the non-trivial element) and the `fmap` shape of
    /// `GroupActionEndo<Z2>`. Single consolidated test per project TDD
    /// convention.
    #[test]
    fn z2_group_laws_and_endofunctor_fmap_smoke() {
        // Local alias: `GroupActionEndo<Z2>` as the endofunctor under test.
        type F = GroupActionEndo<Z2Group>;

        // Identity laws.
        let e = Z2Group::identity();
        assert_eq!(e, Z2Group(false));
        for g in [Z2Group(false), Z2Group(true)] {
            assert_eq!(Z2Group::compose(e, g), g, "left identity for {g:?}");
            assert_eq!(Z2Group::compose(g, e), g, "right identity for {g:?}");
        }
        // Associativity (only 8 cases; check them all).
        for a in [false, true] {
            for b in [false, true] {
                for c in [false, true] {
                    let lhs =
                        Z2Group::compose(Z2Group::compose(Z2Group(a), Z2Group(b)), Z2Group(c));
                    let rhs =
                        Z2Group::compose(Z2Group(a), Z2Group::compose(Z2Group(b), Z2Group(c)));
                    assert_eq!(lhs, rhs, "associativity at ({a}, {b}, {c})");
                }
            }
        }
        // `true` is its own inverse.
        assert_eq!(Z2Group::compose(Z2Group(true), Z2Group(true)), e);

        // `GroupActionEndo<Z2>::fmap` lifts only the second slot.
        let fa: <F as HKT>::Type<i32> = (Z2Group(true), 5);
        let fb: <F as HKT>::Type<i32> = F::fmap(fa, |x| x * 2);
        assert_eq!(fb, (Z2Group(true), 10));

        // fmap preserves the group element across changes of return type.
        let fc: <F as HKT>::Type<String> = F::fmap((Z2Group(false), 7_i32), |x| x.to_string());
        assert_eq!(fc, (Z2Group(false), "7".to_string()));

        // The full identity + composition functor laws for `GroupActionEndo`
        // are covered generically in `tests/functor_laws.rs` via the shared
        // `assert_functor_laws` helper; not duplicated here.
    }
}
