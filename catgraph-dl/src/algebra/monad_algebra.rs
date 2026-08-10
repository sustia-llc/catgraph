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
//! Construction wrappers + the commuting-square verifier inherited via the
//! underlying [`FAlgebraHom`] check. Coherence with `η` (= `Pure`) and
//! `μ` (= the provided `Monad::join`) is now **machine-checked** against
//! caller-supplied samples: [`MonadAlgebra::verify_unit_law`] /
//! [`MonadAlgebra::verify_assoc_law`] on an algebra, and
//! [`MonadAlgebraHom::verify_unit_coherence`] /
//! [`MonadAlgebraHom::verify_mult_coherence`] on a homomorphism. The verifiers
//! bound `M: EndoWitness + Monad<M>` and are **caller-sampled**, not exhaustive
//! (same honesty as [`FAlgebraHom::verify_commutes`]); construction still does
//! not enforce the laws. Note the hom-side coherence checks probe the ambient
//! monad/algebra structure, not `f` itself — only the F-algebra square
//! discriminates homs (see the ⚠️ scope note on [`MonadAlgebraHom`]).

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
///    machine-checked here.
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
/// **Scope:** construction witnesses the type but enforces no law. All three
/// are machine-checkable against caller samples: the F-algebra square (point 1)
/// via `self.algebra_hom.verify_commutes(...)`, and the unit + multiplication
/// coherence (point 2) via [`MonadAlgebraHom::verify_unit_coherence`] and
/// [`MonadAlgebraHom::verify_mult_coherence`]. `η` is
/// [`Pure`](crate::endofunctor::Pure) and `μ` the provided
/// [`Monad::join`]; the verifiers bound
/// `M: EndoWitness + Monad<M>` and are caller-sampled, not exhaustive.
///
/// ⚠️ **Only the square (point 1) discriminates homs.** The two point-2
/// diagrams are consequences of the *ambient* structures — the left is
/// η-naturality (a law of the monad witness, true for every `f`), the right is
/// the source algebra's associativity post-composed with `f` — so the point-2
/// verifiers pass for any `f` between lawful algebras of a lawful monad,
/// including non-homomorphisms. Certifying a hom therefore requires
/// `verify_commutes`; the point-2 verifiers detect broken *witness or algebra*
/// structure reached through the hom. See each verifier's rustdoc and the
/// boundary demonstration in `tests/monad_algebra_laws.rs`.
///
/// # Certifying a homomorphism — the full recipe
///
/// Because no single verifier decides "`f` is a hom between *lawful* algebras",
/// certification is a **three-part conjunction** (CDL Def 2.3 + Def 2.5; Mac Lane
/// CWM VI.2):
///
/// 1. **source algebra lawful** — [`MonadAlgebra::verify_unit_law`] +
///    [`MonadAlgebra::verify_assoc_law`] on the `from` algebra;
/// 2. **target algebra lawful** — the same two on the `to` algebra. The hom holds
///    its algebras as bare [`FAlgebra`] fields, so rewrap first:
///    `MonadAlgebra::new(hom.algebra_hom.to.clone())`;
/// 3. **the hom square** — `hom.algebra_hom.verify_commutes(..)` (the *only*
///    discriminating check).
///
/// The end-to-end recipe — positive plus three negatives each failing exactly
/// one part — is exercised by `full_monad_algebra_hom_certification_recipe` in
/// `tests/monad_algebra_laws.rs`. (No `verify_all` convenience is provided: the
/// rewrap is a one-liner and a bundled entry point would hide which part failed —
/// minimal-ceremony posture.)
///
/// # CDL Example 2.6 — GDL recovery
///
/// When `M = G × −` is the group-action monad for a group `G`, a
/// monad-algebra homomorphism is exactly a **`G`-equivariant map**
/// — `f(g ▶ x) = g ▶ f(x)`. The `Z2`-equivariance test in
/// `tests/algebra_homomorphisms.rs` exhibits the square (point 1) directly for
/// the pointwise absolute-value map, and `tests/monad_algebra_laws.rs`
/// machine-checks point 2 for the same map: because `Z2` has
/// `η(x) = (e, x)` and `μ((g1, (g2, x))) = (g1 · g2, x)`, both coherence
/// verifiers return `true` across arbitrary samples.
///
/// # See also
///
/// - [`FAlgebraHom`] — point (1) verifier.
/// - [`super::FCoalgebraHom`] — dual construction.
#[derive(Debug, Clone)]
pub struct MonadAlgebraHom<M, A, B, FromS, ToS, MapS> {
    /// The underlying F-algebra homomorphism. The F-algebra commuting
    /// square is the only law machine-checked here.
    pub algebra_hom: FAlgebraHom<M, A, B, FromS, ToS, MapS>,
    _phantom: PhantomData<M>,
}

impl<M, A, B, FromS, ToS, MapS> MonadAlgebraHom<M, A, B, FromS, ToS, MapS> {
    /// Wrap an F-algebra homomorphism as a monad-algebra homomorphism.
    ///
    /// # Caller obligations (construction enforces none)
    ///
    /// In addition to the F-algebra square (verify via
    /// `self.algebra_hom.verify_commutes(...)`), a monad-algebra homomorphism
    /// must satisfy:
    ///
    /// 1. `M(f) ∘ η_A = η_B ∘ f` (preservation of the monad unit).
    /// 2. `f ∘ a ∘ M(a) = f ∘ a ∘ μ_A` (compatibility with monad
    ///    multiplication).
    ///
    /// Both are machine-checkable against caller samples via
    /// [`MonadAlgebraHom::verify_unit_coherence`] (law 1) and
    /// [`MonadAlgebraHom::verify_mult_coherence`] (law 2) — but note both hold
    /// automatically for *any* `f` between lawful algebras of a lawful monad
    /// (see the ⚠️ scope note on [`MonadAlgebraHom`]); the square is the
    /// discriminating condition. Construction enforces none of the three.
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
    /// Verify the **unit-preservation** law `M(f) ∘ η_A = η_B ∘ f` on a single
    /// sample `x : A` — the left diagram in [`MonadAlgebraHom`]'s rustdoc. With
    /// `η = ` [`Pure`](crate::endofunctor::Pure), this checks
    /// `M::fmap(M::pure(x), f) == M::pure(f(x))`.
    ///
    /// ⚠️ **This law cannot reject a non-homomorphism.** `η_A` and `η_B` are
    /// both `M::pure` (same monad), so the equation is exactly **η-naturality
    /// at `f`** (CDL Def 1.5 applied to `η : id ⇒ M`) — it holds for *every*
    /// function `f` whenever the witness's `pure`/`fmap` are lawful, and never
    /// consults either algebra's structure map. It is a witness-lawfulness
    /// probe, useful for catching a broken `Pure`/`Functor` impl. The
    /// *discriminating* hom condition is the F-algebra square — run
    /// `self.algebra_hom.verify_commutes(...)` (CDL Definition 2.5).
    /// `tests/monad_algebra_laws.rs` demonstrates the boundary: a
    /// non-equivariant map passes this check while failing the square.
    ///
    /// **Caller-sampled**, not exhaustive.
    ///
    /// # Type parameters
    ///
    /// - `A: Clone` — `x` feeds both sides.
    /// - `M::Type<B>: PartialEq` — the two sides live in `M(B)`.
    /// - `MapS: Fn(A) -> B` — the underlying morphism `f`; the `fmap` leg
    ///   borrows it (`&MapS: FnMut` whenever `MapS: Fn`).
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

    /// Verify the **multiplication-compatibility** law
    /// `f ∘ a ∘ M(a) = f ∘ a ∘ μ_A` on a single sample `mma : M(M(A))`
    /// (CDL Definition 2.3's associativity diagram post-composed with `f`;
    /// `a` is the **source** algebra's structure map). With `μ = ` the
    /// provided [`Monad::join`], this checks
    /// `f(a(M::fmap(mma, a))) == f(a(M::join(mma)))`.
    ///
    /// ⚠️ **This law cannot reject a non-homomorphism.** Whenever the source
    /// algebra alone satisfies its associativity law (see
    /// [`MonadAlgebra::verify_assoc_law`]), both legs agree *before* `f` is
    /// applied, so the check holds for **every** `f` and never consults the
    /// target algebra `b`. It probes the source algebra's lawfulness through
    /// the hom wrapper. The *discriminating* hom condition is the F-algebra
    /// square — run `self.algebra_hom.verify_commutes(...)`
    /// (CDL Definition 2.5). `tests/monad_algebra_laws.rs` demonstrates the
    /// boundary: a non-equivariant map passes this check while failing the
    /// square.
    ///
    /// **Caller-sampled**, not exhaustive.
    ///
    /// # Type parameters
    ///
    /// - `M::Type<M::Type<A>>: Clone` — the nested sample feeds both legs.
    /// - `B: PartialEq` — the two sides live in `B` (after applying `f`).
    /// - `FromS: Fn(M::Type<A>) -> A` — the source structure map `a`; the
    ///   `fmap` leg borrows it (`&FromS: FnMut` whenever `FromS: Fn`).
    /// - `MapS: Fn(A) -> B` — the underlying morphism `f`.
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
