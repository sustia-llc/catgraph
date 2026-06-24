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

use super::group_action::EndoFunctor;

/// An F-algebra `(A, a : F(A) → A)`.
///
/// CDL Definition 2.8. The `carrier` is the underlying type; the
/// `structure_map` is a closure or function value implementing
/// `F(Carrier) → Carrier`.
///
/// **Phase DL-1 scaffold:** opaque struct holding the carrier and the
/// structure-map closure. **Phase DL-2** adds [`FAlgebraHom`] for
/// homomorphisms-of-algebras and the commuting-square verification entry
/// point [`FAlgebraHom::verify_commutes`].
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
/// i.e. `f ∘ a = b ∘ F(f)`.
///
/// Construction does **not** check the square — verification is an
/// explicit caller-driven step via [`Self::verify_commutes`]. This is
/// deliberate: equality on `B` is in general unknown to the type system,
/// and the structure-map closures may not be `PartialEq`-friendly.
///
/// # Examples
///
/// See `tests/algebra_homomorphisms.rs` for the GDL-equivariance recovery
/// example: the absolute-value map `Vec<f64> → Vec<f64>` is a
/// `Z2`-equivariant F-algebra homomorphism for the negation action; the
/// coordinate projection `x ↦ x[0]` is not.
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
    /// Build an F-algebra homomorphism from two algebras and a map
    /// `f : A → B`.
    ///
    /// **Does not** verify the commuting square. Call
    /// [`Self::verify_commutes`] explicitly with a sample `fa: F(A)` to
    /// check at runtime.
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
    F: EndoFunctor,
{
    /// Verify the commuting square `f ∘ a = b ∘ F(f)` on a single sample
    /// `fa: F(A)`.
    ///
    /// This is **caller-sampled**, not exhaustive — the math is universally
    /// quantified over `F(A)`, but Rust's type system has no way to
    /// enumerate that domain. The acceptance harness in
    /// `tests/algebra_homomorphisms.rs` calls this on small representative
    /// samples (e.g. `(Z2Group(true), vec![1.0, -2.0, 3.0])`).
    ///
    /// # Type parameters
    ///
    /// - `F::Apply<A>: Clone` — the sample is consumed twice (once by `a`
    ///   directly, once by `F(f)` followed by `b`).
    /// - `B: PartialEq` — needed to compare the two paths.
    /// - `MapS: Fn(A) -> B + Clone` — `f` is invoked twice (once on the
    ///   `a`-then-`f` path, once inside `F(f)`); cloning the closure is
    ///   the simplest way to satisfy the `'static` bounds `EndoFunctor`'s
    ///   `fmap` signature imposes via `impl Fn`.
    /// - `FromS: Fn(F::Apply<A>) -> A` — the source structure map.
    /// - `ToS: Fn(F::Apply<B>) -> B` — the target structure map.
    ///
    /// # Returns
    ///
    /// `true` if `f(a(fa)) == b(F(f)(fa))` for the given sample;
    /// `false` otherwise.
    pub fn verify_commutes(&self, fa: F::Apply<A>) -> bool
    where
        F::Apply<A>: Clone,
        B: PartialEq,
        MapS: Fn(A) -> B + Clone,
        FromS: Fn(F::Apply<A>) -> A,
        ToS: Fn(F::Apply<B>) -> B,
    {
        // LHS: f ∘ a — apply source structure map then f.
        let lhs: B = (self.map)((self.from.structure_map)(fa.clone()));

        // RHS: b ∘ F(f) — fmap f over fa, then apply target structure map.
        let f = self.map.clone();
        let fb: F::Apply<B> = F::fmap(fa, f);
        let rhs: B = (self.to.structure_map)(fb);

        lhs == rhs
    }
}
