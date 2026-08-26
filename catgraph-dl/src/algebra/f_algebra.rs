//! F-algebras — pairs `(A, a : F(A) → A)`.
//!
//! CDL Definition 2.8. No coherence equations (`F` is a bare endofunctor),
//! distinguishing this from monad algebras (CDL Definition 2.3).
//!
//! Specialisations of interest:
//! - `F = 1 + A × −` → `(List(A), [Nil, Cons])` is the initial algebra
//!   (CDL Example 2.9).
//! - `F = A + (−)²` → `(Tree(A), [Leaf, Node])` is the initial algebra
//!   (CDL Example 2.10).

use core::marker::PhantomData;

use crate::endofunctor::EndoWitness;

/// An F-algebra `(A, a : F(A) → A)`.
///
/// CDL Definition 2.8. The `carrier` is the underlying type; the
/// `structure_map` is a closure or function value implementing
/// `F(Carrier) → Carrier`.
///
/// Opaque struct holding the carrier and the structure-map closure.
/// [`FAlgebraHom`] provides homomorphisms-of-algebras and the
/// commuting-square verification entry point
/// [`FAlgebraHom::verify_commutes`].
#[derive(Debug, Clone)]
pub struct FAlgebra<F, A, S> {
    /// The carrier object `A`.
    pub carrier: A,
    /// The structure map `a : F(A) → A`.
    pub structure_map: S,
    _phantom: PhantomData<F>,
}

impl<F, A, S> FAlgebra<F, A, S> {
    /// Build an F-algebra from its carrier and structure map.
    pub fn new(carrier: A, structure_map: S) -> Self {
        Self {
            carrier,
            structure_map,
            _phantom: PhantomData,
        }
    }
}

/// An F-algebra **homomorphism** `f : (A, a) → (B, b)`.
///
/// CDL Definition 2.5. Given two F-algebras `(A, a)` and `(B, b)` for the
/// same endofunctor `F`, an F-algebra homomorphism is a morphism
/// `f : A → B` making the following square commute:
///
/// ```text
/// F(A) -- F(f) --> F(B)
///  |                |
///  a                b
///  v                v
///  A   --- f -----> B
/// ```
///
/// i.e. `f ∘ a = b ∘ F(f)`. Construction does not check the square;
/// [`Self::verify_commutes`] samples it.
#[derive(Debug, Clone)]
pub struct FAlgebraHom<F, A, B, FromS, ToS, MapS> {
    /// The source algebra `(A, a)`.
    pub from: FAlgebra<F, A, FromS>,
    /// The target algebra `(B, b)`.
    pub to: FAlgebra<F, B, ToS>,
    /// The underlying morphism `f : A → B`.
    pub map: MapS,
    _phantom: PhantomData<F>,
}

impl<F, A, B, FromS, ToS, MapS> FAlgebraHom<F, A, B, FromS, ToS, MapS> {
    /// Wrap two algebras and `f : A → B`; the square is not checked.
    pub fn new(from: FAlgebra<F, A, FromS>, to: FAlgebra<F, B, ToS>, map: MapS) -> Self {
        Self {
            from,
            to,
            map,
            _phantom: PhantomData,
        }
    }
}

impl<F, A, B, FromS, ToS, MapS> FAlgebraHom<F, A, B, FromS, ToS, MapS>
where
    F: EndoWitness,
{
    /// `f(a(fa)) == b(F(f)(fa))` on one sample.
    pub fn verify_commutes(&self, fa: F::Type<A>) -> bool
    where
        F::Type<A>: Clone,
        B: PartialEq,
        MapS: Fn(A) -> B + Clone,
        FromS: Fn(F::Type<A>) -> A,
        ToS: Fn(F::Type<B>) -> B,
    {
        // LHS: f ∘ a — apply source structure map then f.
        let lhs: B = (self.map)((self.from.structure_map)(fa.clone()));

        // RHS: b ∘ F(f) — fmap f over fa, then apply target structure map.
        let f = self.map.clone();
        let fb: F::Type<B> = F::fmap(fa, f);
        let rhs: B = (self.to.structure_map)(fb);

        lhs == rhs
    }
}
