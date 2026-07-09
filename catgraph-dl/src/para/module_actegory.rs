//! The `R`-module actegory `(FinReal, ⊕, R⁰)` acting on itself — the first
//! non-`(Set, ×, 1)` [`MonoidalCategory`] / [`Actegory`] instance.
//!
//! ## Paper anchors (verified against `docs/2402.15332v2.pdf`)
//!
//! The umbrella issue ([#36](https://github.com/sustia-llc/catgraph/issues/36))
//! cites "CDL §3.1" for the R-module actegory. §3 of the main body ("2-Categories
//! and Parametric Morphisms") introduces `Para`; the **formal** actegory
//! definition and the concrete module example both live in the appendices, so
//! the precise anchors used here are:
//!
//! - **Definition E.2** (*Actegories*, after Capucci & Gavranović 2023) — an
//!   `M`-actegory `C` is a category `C` with a functor `▶ : M × C → C` and
//!   natural isomorphisms
//!   `η_X : I ▶ X ≅ X` (unitor) and
//!   `µ_{M,N} : (M ⊗ N) ▶ X ≅ M ▶ (N ▶ X)` (multiplicator),
//!   satisfying the pentagonator (Eq. 7) and the left/right unitor diagrams
//!   (Eq. 8). This is the surface [`Actegory`] models: [`Actegory::act`] is the
//!   underlying map of `▶`, [`Actegory::compose_action`] is `µ`.
//! - **Example E.4** (*Monoidal action*) — "any monoidal category gives rise to
//!   a self-action". [`F64Actegory`] is exactly this self-action of the
//!   monoidal category [`F64Monoidal`] on itself, with `▶ = ⊗ = ⊕`.
//! - **Example G.3** (*Real Vector Spaces and Smooth Maps*) — "Consider the
//!   **cartesian** category `Smooth` whose objects are real vector spaces …
//!   As this category is cartesian, we can form `Para(Smooth)`". This is the
//!   gradient-based-learning `Para(…)` construction. It fixes the monoidal
//!   product below.
//!
//! ## Why the monoidal product is the direct sum `⊕`, not the tensor `⊗_R`
//!
//! Example G.3 forms `Para(Smooth)` over the **cartesian** monoidal structure
//! of real vector spaces. For finite-dimensional real modules the categorical
//! product is the biproduct — `Rᵐ × Rⁿ ≅ Rᵐ⁺ⁿ` — i.e. the **direct sum `⊕`**,
//! with monoidal unit the zero module `R⁰`. The tensor product `⊗_R` is a
//! *different* (closed) monoidal structure with unit `R¹ = R`; it is the setting
//! for multilinear algebra, not the parameter-concatenation used by
//! gradient-based-learning `Para` constructions. So `[F64Monoidal]` realises
//! `(FinReal, ⊕, R⁰)`: [`F64Monoidal::tensor_objects`] pairs blocks and
//! [`DirectSum::flatten`] concatenates their coordinates.
//!
//! ## Carriers (module-appropriate, **not** the `(Set, ×, 1)` tuple)
//!
//! [`F64Module`] is the object carrier — a finite-dimensional real module,
//! `Vec<f64>`-backed, an element of `Rⁿ` for `R = f64`. It carries genuine
//! `R`-module structure ([`F64Module::zeros`], [`F64Module::basis`],
//! [`F64Module::add`], [`F64Module::scale`], [`F64Module::direct_sum`]); this is
//! where the reserved `deep_causality_num` `Zero` / `One` finally activate
//! (issue #36) — `Zero::zero()` is the additive identity `0 ∈ R` filling the
//! zero vector, `One::one()` is the multiplicative identity `1 ∈ R` marking each
//! standard-basis generator.
//!
//! The object-level tensor is the dedicated [`DirectSum`] carrier — deliberately
//! **not** the Rust tuple `(A, B)` that the `(Set, ×, 1)` blanket
//! [`SetCategoryDefaults`](super::SetCategoryDefaults) uses — so [`F64Monoidal`]
//! is a genuine non-`Set` instance rather than an alias of
//! [`SetMonoidal`](super::SetMonoidal). It does **not** opt into
//! `SetCategoryDefaults`; the [`MonoidalCategory`] / [`Actegory`] impls are
//! hand-written with `DirectSum`-appropriate bodies.
//!
//! ## Coherence
//!
//! On `DirectSum` the associator and unitors are exact re-associations (pure
//! data movement, no arithmetic), so Mac Lane's pentagon and triangle hold on
//! the nose — machine-checked in `tests/module_actegory_laws.rs` via
//! `common::assert_direct_sum_coherence` (the `DirectSum` analogue of the tuple
//! `assert_monoidal_coherence`, since that helper is `SetCategoryDefaults`-bound
//! and tuple-shaped). The `R`-module axioms that exercise `Zero` / `One`, and
//! the concrete `⊕`-monoid laws on coordinates, are law-tested in the same file.
//!
//! ## Base ring as a compile-time type
//!
//! [`F64Monoidal`] / [`F64Actegory`] are zero-sized: the base ring is the
//! *type* `f64`, statically known, so this instance needs no runtime payload in
//! the `&self` slot. The slot (see the "Why methods take `&self`" section on
//! [`MonoidalCategory`](super::MonoidalCategory)) remains reserved for an
//! instance whose ring is a **runtime value** — e.g. `Z/nZ` with a modulus `n`
//! chosen at construction — which would carry `n` in the receiver.

use deep_causality_num::{One, Zero};

use super::actegory::Actegory;
use super::monoidal_category::MonoidalCategory;

/// The direct-sum tensor carrier `A ⊕ B`.
///
/// The object-level tensor of [`F64Monoidal`] (and the action result of
/// [`F64Actegory`]). A dedicated newtype rather than the Rust tuple `(A, B)`:
/// this is what makes [`F64Monoidal`] a genuine non-`Set` monoidal category
/// instead of an alias of the `(Set, ×, 1)` blanket. As a *set* the direct sum
/// of two modules is their cartesian product of coordinate blocks, so the two
/// slots `.0` / `.1` hold the summands; [`DirectSum::flatten`] realises the
/// direct sum of two concrete [`F64Module`]s as one concatenated module.
///
/// CDL Definition E.2 / Example E.4.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DirectSum<A, B>(pub A, pub B);

impl<A, B> DirectSum<A, B> {
    /// Construct a direct sum `A ⊕ B` from its two summands.
    pub const fn new(left: A, right: B) -> Self {
        Self(left, right)
    }

    /// Destructure into the underlying `(left, right)` pair of summands.
    pub fn into_pair(self) -> (A, B) {
        (self.0, self.1)
    }
}

impl DirectSum<F64Module, F64Module> {
    /// Realise the abstract direct sum `V ⊕ W` of two concrete modules as the
    /// single concatenated module `Rᵐ⁺ⁿ` — the biproduct carrier of
    /// `Rᵐ × Rⁿ ≅ Rᵐ⁺ⁿ` (Example G.3, cartesian structure of real vector
    /// spaces). Coordinates of the left summand precede those of the right.
    #[must_use]
    pub fn flatten(self) -> F64Module {
        self.0.direct_sum(self.1)
    }
}

/// A finite-dimensional real module `Rⁿ` over the scalar ring `R = f64`.
///
/// The object carrier of the `R`-module actegory. Backed by `Vec<f64>`; the
/// dimension `n` is the vector length. This is the free `R`-module on `n`
/// generators, so it carries the full `R`-module structure:
///
/// - additive identity [`F64Module::zeros`] (`0 ∈ Rⁿ`, each entry
///   `<f64 as Zero>::zero()`),
/// - standard basis [`F64Module::basis`] (`eᵢ`, a single
///   `<f64 as One>::one()` at position `i`),
/// - vector addition [`F64Module::add`] (dimension-guarded),
/// - scalar multiplication [`F64Module::scale`] (`r · v`),
/// - direct sum [`F64Module::direct_sum`] (`⊕`, the monoidal product).
///
/// CDL Definition E.2 (the objects of the category `C` the actegory acts on);
/// Example G.3 (real vector spaces).
///
/// # Float honesty
///
/// Equality is structural `Vec<f64>` equality. The module-axiom identities that
/// assert *exact* equality (`1 · v = v`, `0 · v = 0`, `v + 0 = v`) are exact in
/// IEEE-754 for finite inputs — multiplying by exactly `1.0` and adding exactly
/// `0.0` are representable-preserving. General [`F64Module::add`] /
/// [`F64Module::scale`] on arbitrary reals are subject to ordinary
/// floating-point rounding and are **not** asserted associative/distributive on
/// the nose; tests use the NaN-free `finite_f64` strategy.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct F64Module(Vec<f64>);

impl F64Module {
    /// Wrap a coordinate vector as a module element of dimension `coords.len()`.
    #[must_use]
    pub fn new(coords: Vec<f64>) -> Self {
        Self(coords)
    }

    /// The zero-dimensional module `R⁰` — the monoidal unit of `⊕`.
    ///
    /// `R⁰` has exactly one element (the empty coordinate tuple), so it is the
    /// concrete realisation of the [`MonoidalCategory::Unit`] `()` for
    /// [`F64Monoidal`]: `R⁰ ⊕ V ≅ V` and `V ⊕ R⁰ ≅ V`.
    #[must_use]
    pub fn zero_dim() -> Self {
        Self(Vec::new())
    }

    /// The additive identity `0 ∈ Rⁿ` — every coordinate the ring zero
    /// `<f64 as Zero>::zero()`.
    #[must_use]
    pub fn zeros(dim: usize) -> Self {
        Self(vec![<f64 as Zero>::zero(); dim])
    }

    /// The `i`-th standard basis vector `eᵢ ∈ Rⁿ`: the ring one
    /// `<f64 as One>::one()` at position `i`, the ring zero elsewhere. Returns
    /// `None` when `i` is out of range (`i >= dim`).
    ///
    /// Witnesses that `F64Module` is the *free* `R`-module on `dim` generators;
    /// this is the canonical site where the multiplicative identity `1 ∈ R`
    /// appears in the module structure.
    #[must_use]
    pub fn basis(dim: usize, i: usize) -> Option<Self> {
        if i >= dim {
            return None;
        }
        let mut coords = vec![<f64 as Zero>::zero(); dim];
        coords[i] = <f64 as One>::one();
        Some(Self(coords))
    }

    /// The dimension `n` (number of coordinates).
    #[must_use]
    pub fn dim(&self) -> usize {
        self.0.len()
    }

    /// Borrow the coordinates as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &[f64] {
        &self.0
    }

    /// Consume into the underlying coordinate vector.
    #[must_use]
    pub fn into_vec(self) -> Vec<f64> {
        self.0
    }

    /// Vector addition `u + v` in `Rⁿ`, coordinate-wise. Returns `None` when the
    /// dimensions differ (addition is only defined within one module `Rⁿ`).
    #[must_use]
    pub fn add(&self, other: &Self) -> Option<Self> {
        if self.dim() != other.dim() {
            return None;
        }
        Some(Self(
            self.0.iter().zip(&other.0).map(|(a, b)| a + b).collect(),
        ))
    }

    /// Scalar multiplication `r · v` in `Rⁿ`, coordinate-wise.
    #[must_use]
    pub fn scale(&self, r: f64) -> Self {
        Self(self.0.iter().map(|x| r * x).collect())
    }

    /// Direct sum `u ⊕ v` — the monoidal product `⊗ = ⊕` realised on
    /// coordinates as concatenation: `Rᵐ ⊕ Rⁿ = Rᵐ⁺ⁿ`, left block first.
    ///
    /// CDL Example E.4 / G.3. The monoid `(F64Module, ⊕, R⁰)` on dimensions is
    /// the concrete witness that [`F64Monoidal`] is monoidal.
    #[must_use]
    pub fn direct_sum(self, other: Self) -> Self {
        let mut coords = self.0;
        coords.extend(other.0);
        Self(coords)
    }
}

/// Object-kind marker for [`F64Monoidal`] / [`F64Actegory`].
///
/// The type-level witness that objects are finite-dimensional real modules
/// (values of [`F64Module`]), mirroring [`SetObject`](super::SetObject).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct F64Object;

/// Morphism-kind marker for [`F64Monoidal`] / [`F64Actegory`].
///
/// Morphisms of the module category are `R`-linear maps, carried at the value
/// level; this is the type-level witness, mirroring
/// [`SetMorphism`](super::SetMorphism).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct F64Morphism;

/// The monoidal category `(FinReal, ⊕, R⁰)` of finite-dimensional real modules
/// under **direct sum**.
///
/// The first non-`(Set, ×, 1)` [`MonoidalCategory`] instance. Objects are
/// [`F64Module`]s; the tensor `⊗ = ⊕` is the [`DirectSum`] carrier; the unit `I`
/// is the zero module `R⁰`, represented by `()` (its one element). The
/// associator and unitors are exact `DirectSum` re-associations.
///
/// CDL Definition E.2 / Example E.4 / Example G.3. See the module docs for the
/// `⊕`-vs-`⊗_R` decision and the base-ring-as-type note.
///
/// Zero-sized: the base ring `f64` is a compile-time type, so no runtime payload
/// is carried. Does **not** opt into
/// [`SetCategoryDefaults`](super::SetCategoryDefaults) — the impl is
/// hand-written with `DirectSum` bodies.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct F64Monoidal;

impl F64Monoidal {
    /// Construct a fresh `F64Monoidal`. Zero-sized; cost-free.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl MonoidalCategory for F64Monoidal {
    type Object = F64Object;
    type Morphism = F64Morphism;
    /// The monoidal unit `I = R⁰`. `R⁰` is a one-element module, so `()` is its
    /// faithful carrier (concretely [`F64Module::zero_dim`]).
    type Unit = ();
    /// The object-level tensor `A ⊗ B = A ⊕ B`.
    type Tensor<A, B> = DirectSum<A, B>;

    fn tensor_objects<A, B>(&self, a: A, b: B) -> Self::Tensor<A, B> {
        DirectSum(a, b)
    }

    fn unit(&self) -> Self::Unit {}

    fn associate<A, B, C>(
        &self,
        nested: Self::Tensor<Self::Tensor<A, B>, C>,
    ) -> Self::Tensor<A, Self::Tensor<B, C>> {
        // α : (A ⊕ B) ⊕ C → A ⊕ (B ⊕ C) — exact re-association.
        let DirectSum(DirectSum(a, b), c) = nested;
        DirectSum(a, DirectSum(b, c))
    }

    fn left_unitor<A>(&self, paired: Self::Tensor<Self::Unit, A>) -> A {
        // λ : R⁰ ⊕ A → A.
        let DirectSum((), a) = paired;
        a
    }

    fn right_unitor<A>(&self, paired: Self::Tensor<A, Self::Unit>) -> A {
        // ρ : A ⊕ R⁰ → A.
        let DirectSum(a, ()) = paired;
        a
    }
}

/// The self-action `▶ = ⊕` of [`F64Monoidal`] on itself — the `R`-module
/// actegory.
///
/// CDL Example E.4 (a monoidal category acts on itself). The action of a
/// parameter module `P` on a carrier module `X` is the direct sum `P ⊕ X`; the
/// multiplicator `µ : Q ▶ (P ▶ X) → (Q ⊗ P) ▶ X` is the exact `DirectSum`
/// re-association matching [`F64Monoidal`]'s tensor. This is the actegory the
/// gradient-based-learning `Para(F64Monoidal, F64Actegory)` construction runs
/// over (Example G.3), where parameter concatenation `⊕` composes learnable
/// weights.
///
/// Zero-sized (see [`F64Monoidal`]); does not opt into any blanket.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct F64Actegory;

impl F64Actegory {
    /// Construct a fresh `F64Actegory`. Zero-sized; cost-free.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Realise the action on two concrete modules: `P ▶ X = P ⊕ X` as the
    /// single concatenated module. The concrete counterpart of the generic
    /// [`Actegory::act`], whose [`DirectSum`] result [`DirectSum::flatten`]
    /// collapses to this.
    #[must_use]
    pub fn act_modules(&self, parameter: F64Module, x: F64Module) -> F64Module {
        parameter.direct_sum(x)
    }
}

impl Actegory<F64Monoidal> for F64Actegory {
    type Object = F64Object;
    type Morphism = F64Morphism;
    /// `P ▶ X = P ⊕ X`.
    type ActionResult<P, X> = DirectSum<P, X>;

    fn act<P, X>(&self, parameter: P, x: X) -> Self::ActionResult<P, X> {
        DirectSum(parameter, x)
    }

    fn compose_action<Q, P, X>(
        &self,
        q: Q,
        p: P,
        x: X,
    ) -> Self::ActionResult<<F64Monoidal as MonoidalCategory>::Tensor<Q, P>, X> {
        // µ : Q ▶ (P ▶ X) = Q ⊕ (P ⊕ X)  →  (Q ⊗ P) ▶ X = (Q ⊕ P) ⊕ X.
        DirectSum(DirectSum(q, p), x)
    }
}
