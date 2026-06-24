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
//! CDL Example 2.4: group actions arise as algebras of the group-action
//! monad `G × −`. CDL Example 2.6: equivariant maps are group-action
//! monad-algebra homomorphisms — the categorical recovery of Geometric
//! Deep Learning.
//!
//! **Phase DL-2 scaffold:** Construction wrappers + the
//! commuting-square verifier inherited via the underlying [`FAlgebraHom`]
//! check. Coherence with `η` and `μ` is a **documented obligation**, not
//! a machine-checked invariant — see [`MonadAlgebraHom`] below.

use core::marker::PhantomData;

use super::f_algebra::{FAlgebra, FAlgebraHom};

/// An algebra `(A, a : M(A) → A)` for a monad `M`.
///
/// CDL Definition 2.3. The implementor must guarantee compatibility with
/// the monad unit and multiplication; this is checked in Phase DL-2.
#[derive(Debug, Clone)]
pub struct MonadAlgebra<M, A, S> {
    /// The underlying F-algebra. `M` is reused as the endofunctor name.
    pub algebra: FAlgebra<M, A, S>,
    _phantom: PhantomData<M>,
}

impl<M, A, S> MonadAlgebra<M, A, S> {
    /// Wrap an F-algebra as a monad algebra. The caller is responsible for
    /// the unit + associativity laws (verification arrives in Phase DL-2).
    pub fn new(algebra: FAlgebra<M, A, S>) -> Self {
        Self {
            algebra,
            _phantom: PhantomData,
        }
    }
}

/// A monad-algebra **homomorphism** `f : (A, a) → (B, b)`.
///
/// CDL Definition 2.3 / Example 2.6. Given two monad algebras
/// `(A, a : M(A) → A)` and `(B, b : M(B) → B)` for the same monad
/// `(M, η, μ)`, a monad-algebra homomorphism is a morphism `f : A → B`
/// satisfying **both**:
///
/// 1. **The F-algebra commuting square** —
///    `f ∘ a = b ∘ M(f)` (CDL Definition 2.5). This is the same square
///    checked by [`FAlgebraHom::verify_commutes`], and is the only law
///    machine-checked in Phase DL-2.
///
/// 2. **Monad unit + multiplication coherence** — `f` must additionally
///    respect the monad's unit `η` and multiplication `μ`. Concretely,
///    a monad-algebra homomorphism between algebras of `(M, η, μ)` is a
///    **`M`-equivariant map** in the sense that the following two
///    diagrams commute (in addition to (1)):
///
///    ```text
///        η_A             M(M(A)) ─── M(a) ───▶ M(A)
///    A ─────▶ M(A)          │                   │
///    │         │           μ_A                 a
///    f       M(f)            ▼                  ▼
///    ▼         ▼            M(A) ──── a ──────▶ A
///    B ─────▶ M(B)
///         η_B
///    ```
///
///    The left diagram says `M(f) ∘ η_A = η_B ∘ f` (preservation of the
///    unit); the right diagram is the monad-algebra associativity that
///    every algebra independently must satisfy.
///
/// **Phase DL-2 scope:** only the F-algebra square (point 1) is
/// machine-checked. The unit + multiplication coherence (point 2) is a
/// **documented obligation**: the type system witnesses construction but
/// does not verify these laws. Future phases (DL-3+) will add a
/// `Monad` trait carrying `η`, `μ`, and the corresponding verification
/// entry points.
///
/// # CDL Example 2.6 — GDL recovery
///
/// When `M = G × −` is the group-action monad for a group `G`, a
/// monad-algebra homomorphism is exactly a **`G`-equivariant map**
/// — `f(g ▶ x) = g ▶ f(x)`. The `Z2`-equivariance test in
/// `tests/algebra_homomorphisms.rs` exhibits this directly: the
/// pointwise absolute-value map satisfies the equivariance square (point
/// 1), and (because `Z2` has trivial unit/multiplication coherence —
/// `η(x) = (e, x)`, `μ((g1, (g2, x))) = (g1 · g2, x)`) it also
/// satisfies points 2 by inspection.
///
/// # See also
///
/// - [`FAlgebraHom`] — point (1) verifier.
/// - [`super::FCoalgebraHom`] — dual construction.
#[derive(Debug, Clone)]
pub struct MonadAlgebraHom<M, A, B, FromS, ToS, MapS> {
    /// The underlying F-algebra homomorphism. The F-algebra commuting
    /// square is the only law machine-checked at Phase DL-2.
    pub algebra_hom: FAlgebraHom<M, A, B, FromS, ToS, MapS>,
    _phantom: PhantomData<M>,
}

impl<M, A, B, FromS, ToS, MapS> MonadAlgebraHom<M, A, B, FromS, ToS, MapS> {
    /// Wrap an F-algebra homomorphism as a monad-algebra homomorphism.
    ///
    /// # Caller obligations (not machine-checked)
    ///
    /// In addition to the F-algebra square (which can be verified via
    /// `self.algebra_hom.verify_commutes(...)`), the caller is responsible
    /// for:
    ///
    /// 1. `M(f) ∘ η_A = η_B ∘ f` (preservation of the monad unit).
    /// 2. `f ∘ a ∘ M(a) = f ∘ a ∘ μ_A` (compatibility with monad
    ///    multiplication).
    ///
    /// Phase DL-3+ will introduce a `Monad` trait carrying `η` and `μ`
    /// and add machine-checked verifiers for these laws. For now they are
    /// **caller-attested** — construction does not enforce them.
    pub fn new(algebra_hom: FAlgebraHom<M, A, B, FromS, ToS, MapS>) -> Self {
        Self {
            algebra_hom,
            _phantom: PhantomData,
        }
    }
}
