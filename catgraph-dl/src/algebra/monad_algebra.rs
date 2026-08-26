//! Monad algebras — F-algebras compatible with monad unit and multiplication.
//!
//! CDL Definition 2.3. An algebra `(A, a : M(A) → A)` for a monad
//! `(M, η, μ)` satisfies:
//!
//! ```text
//! a ∘ η_A = id_A                      (unit law)
//! a ∘ M(a) = a ∘ μ_A                  (associativity)
//! ```
//!
//! CDL Ex 2.4: group actions are algebras of `G × −`. CDL Ex 2.6: equivariant
//! maps are their homomorphisms.
//!
//! Construction enforces no law. Sampled verifiers: [`MonadAlgebra::verify_unit_law`],
//! [`MonadAlgebra::verify_assoc_law`], [`MonadAlgebraHom::verify_unit_coherence`],
//! [`MonadAlgebraHom::verify_mult_coherence`], with `η = Pure`, `μ = Monad::join`.
//! Only the F-algebra square ([`FAlgebraHom::verify_commutes`]) discriminates
//! homomorphisms.

use core::marker::PhantomData;

use super::f_algebra::{FAlgebra, FAlgebraHom};
use crate::endofunctor::{EndoWitness, Monad};

/// An algebra `(A, a : M(A) → A)` for a monad `M`.
///
/// CDL Definition 2.3. The implementor must guarantee compatibility with the
/// monad unit and multiplication. Construction does not enforce it, but the
/// two laws are machine-checkable against caller samples via
/// [`MonadAlgebra::verify_unit_law`] and [`MonadAlgebra::verify_assoc_law`].
#[derive(Debug, Clone)]
pub struct MonadAlgebra<M, A, S> {
    /// The underlying F-algebra. `M` is reused as the endofunctor name.
    pub algebra: FAlgebra<M, A, S>,
    _phantom: PhantomData<M>,
}

impl<M, A, S> MonadAlgebra<M, A, S> {
    /// Wrap an F-algebra as a monad algebra. Construction does not enforce
    /// the unit + associativity laws; check them with
    /// [`MonadAlgebra::verify_unit_law`] / [`MonadAlgebra::verify_assoc_law`].
    pub fn new(algebra: FAlgebra<M, A, S>) -> Self {
        Self {
            algebra,
            _phantom: PhantomData,
        }
    }
}

impl<M, A, S> MonadAlgebra<M, A, S>
where
    M: EndoWitness + Monad<M>,
{
    /// Verify the monad-algebra **unit law** `a ∘ η_A = id_A` on a single
    /// sample `x : A` (CDL Definition 2.3). With `η = ` [`Pure`](crate::endofunctor::Pure),
    /// this checks `a(M::pure(x)) == x`.
    ///
    /// **Caller-sampled**, not exhaustive — the law is universally quantified
    /// over `A`, but Rust has no way to enumerate it; mirrors
    /// [`FAlgebraHom::verify_commutes`]'s honesty. For the group-action monad
    /// `G × −`, `η(x) = (e, x)`, so this asserts `a((e, x)) == x`.
    ///
    /// # Type parameters
    ///
    /// - `A: Clone` — `x` is consumed by `pure` and compared afterwards.
    /// - `A: PartialEq` — needed to compare the two sides.
    /// - `S: Fn(M::Type<A>) -> A` — the structure map `a`.
    pub fn verify_unit_law(&self, x: A) -> bool
    where
        A: Clone + PartialEq,
        S: Fn(M::Type<A>) -> A,
    {
        let lhs: A = (self.algebra.structure_map)(M::pure(x.clone()));
        lhs == x
    }

    /// Verify the monad-algebra **associativity law** `a ∘ M(a) = a ∘ μ_A` on a
    /// single sample `mma : M(M(A))` (CDL Definition 2.3). With `μ = ` the
    /// provided [`Monad::join`], this checks
    /// `a(M::fmap(mma, a)) == a(M::join(mma))`.
    ///
    /// **Caller-sampled**, not exhaustive (same caveat as
    /// [`verify_unit_law`](Self::verify_unit_law)). For the group-action monad
    /// `G × −` this is the action axiom `g1 ▶ (g2 ▶ x) == (g1 · g2) ▶ x`.
    ///
    /// # Type parameters
    ///
    /// - `M::Type<M::Type<A>>: Clone` — the nested sample feeds both legs.
    /// - `A: PartialEq` — needed to compare the two sides.
    /// - `S: Fn(M::Type<A>) -> A` — the structure map `a`; `&S` is itself
    ///   `FnMut` whenever `S: Fn`, so the `fmap` leg borrows rather than
    ///   clones (no `Clone` bound).
    pub fn verify_assoc_law(&self, mma: M::Type<M::Type<A>>) -> bool
    where
        M::Type<M::Type<A>>: Clone,
        A: PartialEq,
        S: Fn(M::Type<A>) -> A,
    {
        let a = &self.algebra.structure_map;

        // LHS: a ∘ M(a) — fmap the structure map over the inner layer, then a.
        let lhs: A = a(M::fmap(mma.clone(), a));

        // RHS: a ∘ μ_A — μ is the provided `join`.
        let rhs: A = a(M::join(mma));

        lhs == rhs
    }
}

/// Monad-algebra homomorphism `f : (A, a) → (B, b)` (CDL Def 2.3 / Def 2.5 /
/// Ex 2.6): `f ∘ a = b ∘ M(f)`, `M(f) ∘ η_A = η_B ∘ f`, and the source
/// algebra's associativity post-composed with `f`. Construction enforces
/// nothing; `algebra_hom.verify_commutes` is the only sampled check that can
/// reject a non-homomorphism — the two coherence verifiers hold for every `f`
/// between lawful algebras of a lawful monad. For `M = G × −` a homomorphism
/// is a `G`-equivariant map.
#[derive(Debug, Clone)]
pub struct MonadAlgebraHom<M, A, B, FromS, ToS, MapS> {
    /// The underlying F-algebra homomorphism. The F-algebra commuting
    /// square is the only law machine-checked here.
    pub algebra_hom: FAlgebraHom<M, A, B, FromS, ToS, MapS>,
    _phantom: PhantomData<M>,
}

impl<M, A, B, FromS, ToS, MapS> MonadAlgebraHom<M, A, B, FromS, ToS, MapS> {
    /// Wrap an F-algebra homomorphism; no law is checked.
    pub fn new(algebra_hom: FAlgebraHom<M, A, B, FromS, ToS, MapS>) -> Self {
        Self {
            algebra_hom,
            _phantom: PhantomData,
        }
    }
}

impl<M, A, B, FromS, ToS, MapS> MonadAlgebraHom<M, A, B, FromS, ToS, MapS>
where
    M: EndoWitness + Monad<M>,
{
    /// `M::fmap(M::pure(x), f) == M::pure(f(x))` on one sample — η-naturality
    /// at `f`, which holds for every `f` under a lawful witness; it never
    /// consults either algebra.
    pub fn verify_unit_coherence(&self, x: A) -> bool
    where
        A: Clone,
        M::Type<B>: PartialEq,
        MapS: Fn(A) -> B,
    {
        // LHS: M(f) ∘ η_A — lift f over the unit `(e, x)`.
        let lhs: M::Type<B> = M::fmap(M::pure(x.clone()), &self.algebra_hom.map);

        // RHS: η_B ∘ f — unit of `f(x)`.
        let rhs: M::Type<B> = M::pure((self.algebra_hom.map)(x));

        lhs == rhs
    }

    /// `f(a(M::fmap(mma, a))) == f(a(M::join(mma)))` on one sample, `a` the
    /// source structure map — holds for every `f` whenever the source algebra
    /// is associative; it never consults the target algebra.
    pub fn verify_mult_coherence(&self, mma: M::Type<M::Type<A>>) -> bool
    where
        M::Type<M::Type<A>>: Clone,
        B: PartialEq,
        FromS: Fn(M::Type<A>) -> A,
        MapS: Fn(A) -> B,
    {
        let a = &self.algebra_hom.from.structure_map;
        let f = &self.algebra_hom.map;

        // LHS: f ∘ a ∘ M(a).
        let lhs: B = f(a(M::fmap(mma.clone(), a)));

        // RHS: f ∘ a ∘ μ_A.
        let rhs: B = f(a(M::join(mma)));

        lhs == rhs
    }
}
