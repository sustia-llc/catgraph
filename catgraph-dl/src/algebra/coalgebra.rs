//! F-coalgebras — pairs `(A, a : A → F(A))`.
//!
//! CDL Definition B.2. Dual to F-algebras: structure map points outward.
//! Coalgebras model potentially-infinite computation (productivity rather
//! than termination — Atkey & `McBride` 2013).
//!
//! Specialisations of interest:
//! - `F = O × −` → `(Stream(O), ⟨output, next⟩)` is the final coalgebra
//!   (CDL Example H.4).
//! - `F = I → O × −` → `(Mealy_{O,I}, next)` is the final coalgebra
//!   (CDL Example 2.11).
//! - `F = O × (I → −)` → `(Moore_{O,I}, ⟨output, nextStep⟩)` is the final
//!   coalgebra (CDL Example H.7).

use core::marker::PhantomData;

use crate::endofunctor::EndoWitness;

/// An F-coalgebra `(A, a : A → F(A))`.
///
/// CDL Definition B.2. Dual of [`FAlgebra`](super::FAlgebra).
///
/// Opaque struct holding the carrier and the outgoing structure-map
/// closure. [`FCoalgebraHom`] provides the dual commuting square.
#[derive(Debug, Clone)]
pub struct FCoalgebra<F, A, S> {
    /// The carrier object `A`.
    pub carrier: A,
    /// The structure map `a : A → F(A)`.
    pub structure_map: S,
    _phantom: PhantomData<F>,
}

impl<F, A, S> FCoalgebra<F, A, S> {
    /// Build an F-coalgebra from its carrier and structure map.
    pub fn new(carrier: A, structure_map: S) -> Self {
        Self {
            carrier,
            structure_map,
            _phantom: PhantomData,
        }
    }
}

/// An F-coalgebra **homomorphism** `f : (A, a) → (B, b)`.
///
/// CDL Definition B.2 (dual). Given two F-coalgebras `(A, a)` and
/// `(B, b)` for the same endofunctor `F`, an F-coalgebra homomorphism is
/// a morphism `f : A → B` making the *dual* square commute:
///
/// ```text
///  A   --- f -----> B
///  |                |
///  a                b
///  v                v
/// F(A) -- F(f) --> F(B)
/// ```
///
/// i.e. `F(f) ∘ a = b ∘ f`.
///
/// Construction does not check the square — call
/// [`Self::verify_commutes`] with a sample `x : A` to check at runtime.
#[derive(Debug, Clone)]
pub struct FCoalgebraHom<F, A, B, FromS, ToS, MapS> {
    /// The source coalgebra `(A, a)`.
    pub from: FCoalgebra<F, A, FromS>,
    /// The target coalgebra `(B, b)`.
    pub to: FCoalgebra<F, B, ToS>,
    /// The underlying morphism `f : A → B`.
    pub map: MapS,
    _phantom: PhantomData<F>,
}

impl<F, A, B, FromS, ToS, MapS> FCoalgebraHom<F, A, B, FromS, ToS, MapS> {
    /// Build an F-coalgebra homomorphism from two coalgebras and a map
    /// `f : A → B`.
    ///
    /// **Does not** verify the dual commuting square. Call
    /// [`Self::verify_commutes`] explicitly.
    pub fn new(from: FCoalgebra<F, A, FromS>, to: FCoalgebra<F, B, ToS>, map: MapS) -> Self {
        Self {
            from,
            to,
            map,
            _phantom: PhantomData,
        }
    }
}

impl<F, A, B, FromS, ToS, MapS> FCoalgebraHom<F, A, B, FromS, ToS, MapS>
where
    F: EndoWitness,
{
    /// Verify the dual commuting square `F(f) ∘ a = b ∘ f` on a single
    /// sample `x : A`.
    ///
    /// Caller-sampled, not exhaustive (same caveat as
    /// [`super::FAlgebraHom::verify_commutes`]).
    ///
    /// # Type parameters
    ///
    /// - `A: Clone` — `x` is consumed twice (by `a` directly, and by `f`
    ///   followed by `b`).
    /// - `F::Type<B>: PartialEq` — needed to compare the two paths
    ///   (both produce `F(B)`).
    /// - `MapS: Fn(A) -> B + Clone` — `f` is invoked twice.
    /// - `FromS: Fn(A) -> F::Type<A>` — the source structure map.
    /// - `ToS: Fn(B) -> F::Type<B>` — the target structure map.
    pub fn verify_commutes(&self, x: A) -> bool
    where
        A: Clone,
        F::Type<B>: PartialEq,
        MapS: Fn(A) -> B + Clone,
        FromS: Fn(A) -> F::Type<A>,
        ToS: Fn(B) -> F::Type<B>,
    {
        // LHS: F(f) ∘ a — apply source structure map then fmap f.
        let fa: F::Type<A> = (self.from.structure_map)(x.clone());
        let f = self.map.clone();
        let lhs: F::Type<B> = F::fmap(fa, f);

        // RHS: b ∘ f — apply f then target structure map.
        let rhs: F::Type<B> = (self.to.structure_map)((self.map)(x));

        lhs == rhs
    }
}
