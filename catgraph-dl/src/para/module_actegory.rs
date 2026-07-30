//! The `R`-module actegory `(FinReal, ⊕, R⁰)` acting on itself — the first
//! non-`(Set, ×, 1)` [`MonoidalCategory`] / [`Actegory`] instance.
//!
//! ## Paper anchors (verified against [arXiv:2402.15332v2](https://arxiv.org/abs/2402.15332))
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
//!   a self-action". [`RActegory`] is exactly this self-action of the
//!   monoidal category [`RMonoidal`] on itself, with `▶ = ⊗ = ⊕`.
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
//! gradient-based-learning `Para` constructions. So `[RMonoidal]` realises
//! `(FinReal, ⊕, R⁰)`: [`RMonoidal::tensor_objects`] pairs blocks and
//! [`DirectSum::flatten`] concatenates their coordinates.
//!
//! ## Carriers (module-appropriate, **not** the `(Set, ×, 1)` tuple)
//!
//! [`RModule<S>`] is the object carrier — a finite-dimensional module over the
//! scalar ring `S`, `Vec<S>`-backed, an element of `Sⁿ`. It carries genuine
//! `R`-module structure ([`RModule::zeros`], [`RModule::basis`],
//! [`RModule::add`], [`RModule::scale`], [`RModule::direct_sum`]); this is
//! where the reserved `deep_causality_num` `Zero` / `One` finally activate
//! (issue #36) — `Zero::zero()` is the additive identity `0 ∈ R` filling the
//! zero vector, `One::one()` is the multiplicative identity `1 ∈ R` marking each
//! standard-basis generator. [`F64Module`] is the `S = f64` alias.
//!
//! The object-level tensor is the dedicated [`DirectSum`] carrier — deliberately
//! **not** the Rust tuple `(A, B)` that the `(Set, ×, 1)` blanket
//! [`SetCategoryDefaults`](super::SetCategoryDefaults) uses — so [`RMonoidal`]
//! is a genuine non-`Set` instance rather than an alias of
//! [`SetMonoidal`](super::SetMonoidal). It does **not** opt into
//! `SetCategoryDefaults`; the [`MonoidalCategory`] / [`Actegory`] impls are
//! hand-written with `DirectSum`-appropriate bodies.
//!
//! ## Coherence
//!
//! On `DirectSum` the associator and unitors are exact re-associations (pure
//! data movement, no arithmetic), so Mac Lane's pentagon and triangle hold on
//! the nose — machine-checked in `tests/module_actegory_laws.rs` via the
//! **generic** `common::assert_monoidal_coherence` (the same driver used for
//! the `(Set, ×, 1)` tuple carrier). Since
//! [`MonoidalCategory::tensor_morphisms`] landed
//! ([#65](https://github.com/sustia-llc/catgraph/issues/65)) the `α ⊗ id` /
//! `id ⊗ α` pentagon/triangle legs are expressed through that method rather
//! than hand-spelled per instance — for `DirectSum` it maps the two summands,
//! and this instance supplies the [`DirectSum`]-shaped body. Honesty note: the
//! [`MonoidalCategory`] impl itself is object-agnostic pure re-association
//! (the trait's GATs place no bound on `A`, `B` — `tensor_objects` accepts
//! any types, exactly like `SetMonoidal`'s); what makes this instance the
//! `R`-module actegory is the [`DirectSum`] carrier plus the concrete module
//! layer ([`RModule`], [`DirectSum::flatten`]) that realises `⊕` on actual
//! coordinates. The `R`-module axioms that exercise `Zero` / `One`, and the
//! concrete `⊕`-monoid laws on coordinates, are law-tested in the same file.
//!
//! ## Base ring as a compile-time type
//!
//! [`RMonoidal<S>`] / [`RActegory<S>`] are zero-sized: the base ring is the
//! *type parameter* `S`, statically known at every use site, so this instance
//! needs no runtime payload in the `&self` slot. The default instantiation is
//! `S = f64` ([`F64Monoidal`] / [`F64Actegory`]), but any scalar type
//! satisfying the per-method bounds works — the ring genuinely lives in the
//! type system. The slot (see the "Why methods take `&self`" section on
//! [`MonoidalCategory`](super::MonoidalCategory)) remains reserved for an
//! instance whose ring is a **runtime value** — e.g. `Z/nZ` with a modulus `n`
//! chosen at construction — which would carry `n` in the receiver.

use core::marker::PhantomData;
use core::ops::{Add, Mul};

use deep_causality_num::{One, Zero};

use super::actegory::Actegory;
use super::monoidal_category::MonoidalCategory;

/// The direct-sum tensor carrier `A ⊕ B`.
///
/// The object-level tensor of [`RMonoidal`] (and the action result of
/// [`RActegory`]). A dedicated newtype rather than the Rust tuple `(A, B)`:
/// this is what makes [`RMonoidal`] a genuine non-`Set` monoidal category
/// instead of an alias of the `(Set, ×, 1)` blanket. As a *set* the direct sum
/// of two modules is their cartesian product of coordinate blocks, so the two
/// slots `.0` / `.1` hold the summands; [`DirectSum::flatten`] realises the
/// direct sum of two concrete [`RModule`]s as one concatenated module.
///
/// CDL Definition E.2 / Example E.4.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DirectSum<A, B>(pub A, pub B);

impl<S> DirectSum<RModule<S>, RModule<S>> {
    /// Realise the abstract direct sum `V ⊕ W` of two concrete modules as the
    /// single concatenated module `Rᵐ⁺ⁿ` — the biproduct carrier of
    /// `Rᵐ × Rⁿ ≅ Rᵐ⁺ⁿ` (Example G.3, cartesian structure of real vector
    /// spaces). Coordinates of the left summand precede those of the right.
    #[must_use]
    pub fn flatten(self) -> RModule<S> {
        self.0.direct_sum(self.1)
    }
}

/// A finite-dimensional free module `Sⁿ` over the scalar ring `S`.
///
/// The object carrier of the `R`-module actegory. Backed by `Vec<S>`; the
/// dimension `n` is the vector length. This is the free `R`-module on `n`
/// generators, so it carries the full `R`-module structure:
///
/// - additive identity [`RModule::zeros`] (`0 ∈ Sⁿ`, each entry
///   `<S as Zero>::zero()`),
/// - standard basis [`RModule::basis`] (`eᵢ`, a single
///   `<S as One>::one()` at position `i`),
/// - vector addition [`RModule::add`] (dimension-guarded),
/// - scalar multiplication [`RModule::scale`] (`r · v`),
/// - direct sum [`RModule::direct_sum`] (`⊕`, the monoidal product).
///
/// The scalar ring is the type parameter `S`; [`F64Module`] is the `S = f64`
/// instantiation used throughout the crate. Bounds are stated **per method**,
/// not on the struct, so a scalar type only has to satisfy what the operations
/// it is actually used with require.
///
/// CDL Definition E.2 (the objects of the category `C` the actegory acts on);
/// Example G.3 (real vector spaces).
///
/// # Float honesty (`S = f64` and other IEEE float scalars)
///
/// This section is about IEEE-754 semantics, not about the generic structure —
/// it applies whenever `S` is a floating-point type (in particular to the
/// [`F64Module`] alias), and not to exact scalar rings.
///
/// Equality is structural `Vec<f64>` equality via `f64` `PartialEq`, which
/// **identifies `-0.0` and `+0.0`**. The module-axiom identities
/// (`1 · v = v`, `0 · v = 0`, `v + 0 = v`) hold exactly *under that equality*
/// for finite inputs — but signed-zero **bit patterns are not preserved**:
/// IEEE-754 gives `0.0 · (-1.0) = -0.0` (so `0 · v` need not be bitwise
/// `zeros()`) and `-0.0 + 0.0 = +0.0` (so `v + 0` can flip a sign bit of `v`).
/// Do not rely on these identities for bit-exactness (same family as the
/// [#58](https://github.com/sustia-llc/catgraph/issues/58) `F64Rig`
/// signed-zero note). General [`RModule::add`] / [`RModule::scale`] on
/// arbitrary reals are subject to ordinary floating-point rounding and are
/// **not** asserted associative/distributive on the nose; tests use the
/// NaN-free `finite_f64` strategy.
#[derive(Debug, Clone, PartialEq)]
pub struct RModule<S>(Vec<S>);

/// The finite-dimensional real module `Rⁿ` over `R = f64` — the default
/// instantiation of [`RModule`], and the carrier the rest of the crate uses.
pub type F64Module = RModule<f64>;

// Hand-written so the empty module is available for every `S`, including
// scalar types that are not themselves `Default` (the derive would add a
// spurious `S: Default` bound).
impl<S> Default for RModule<S> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<S> RModule<S> {
    /// Wrap a coordinate vector as a module element of dimension `coords.len()`.
    #[must_use]
    pub fn new(coords: Vec<S>) -> Self {
        Self(coords)
    }

    /// The zero-dimensional module `R⁰` — the monoidal unit of `⊕`.
    ///
    /// `R⁰` has exactly one element (the empty coordinate tuple), so it is the
    /// concrete realisation of the [`MonoidalCategory::Unit`] `()` for
    /// [`RMonoidal`]: `R⁰ ⊕ V ≅ V` and `V ⊕ R⁰ ≅ V`.
    #[must_use]
    pub fn zero_dim() -> Self {
        Self(Vec::new())
    }

    /// The dimension `n` (number of coordinates).
    #[must_use]
    pub fn dim(&self) -> usize {
        self.0.len()
    }

    /// Borrow the coordinates as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &[S] {
        &self.0
    }

    /// Consume into the underlying coordinate vector.
    #[must_use]
    pub fn into_vec(self) -> Vec<S> {
        self.0
    }

    /// Direct sum `u ⊕ v` — the monoidal product `⊗ = ⊕` realised on
    /// coordinates as concatenation: `Rᵐ ⊕ Rⁿ = Rᵐ⁺ⁿ`, left block first.
    ///
    /// CDL Example E.4 / G.3. The monoid `(RModule<S>, ⊕, R⁰)` on dimensions is
    /// the concrete witness that [`RMonoidal`] is monoidal.
    #[must_use]
    pub fn direct_sum(self, other: Self) -> Self {
        let mut coords = self.0;
        coords.extend(other.0);
        Self(coords)
    }
}

impl<S: Zero + Clone> RModule<S> {
    /// The additive identity `0 ∈ Sⁿ` — every coordinate the ring zero
    /// `<S as Zero>::zero()`.
    #[must_use]
    pub fn zeros(dim: usize) -> Self {
        Self(vec![<S as Zero>::zero(); dim])
    }
}

impl<S: Zero + One + Clone> RModule<S> {
    /// The `i`-th standard basis vector `eᵢ ∈ Sⁿ`: the ring one
    /// `<S as One>::one()` at position `i`, the ring zero elsewhere. Returns
    /// `None` when `i` is out of range (`i >= dim`).
    ///
    /// Witnesses that `RModule<S>` is the *free* `R`-module on `dim`
    /// generators; this is the canonical site where the multiplicative
    /// identity `1 ∈ R` appears in the module structure.
    #[must_use]
    pub fn basis(dim: usize, i: usize) -> Option<Self> {
        if i >= dim {
            return None;
        }
        let mut coords = vec![<S as Zero>::zero(); dim];
        coords[i] = <S as One>::one();
        Some(Self(coords))
    }
}

impl<S: Clone + Add<Output = S>> RModule<S> {
    /// Vector addition `u + v` in `Sⁿ`, coordinate-wise. Returns `None` when the
    /// dimensions differ (addition is only defined within one module `Sⁿ`).
    #[must_use]
    pub fn add(&self, other: &Self) -> Option<Self> {
        if self.dim() != other.dim() {
            return None;
        }
        Some(Self(
            self.0
                .iter()
                .zip(&other.0)
                .map(|(a, b)| a.clone() + b.clone())
                .collect(),
        ))
    }
}

impl<S: Clone + Mul<Output = S>> RModule<S> {
    /// Scalar multiplication `r · v` in `Sⁿ`, coordinate-wise.
    #[must_use]
    pub fn scale(&self, r: S) -> Self {
        Self(self.0.iter().map(|x| r.clone() * x.clone()).collect())
    }
}

/// Object-kind marker for [`RMonoidal`] / [`RActegory`].
///
/// The type-level witness that objects are finite-dimensional free modules
/// over `S` (values of [`RModule<S>`]), mirroring [`SetObject`](super::SetObject).
///
/// Zero-sized: the scalar parameter is carried as `PhantomData<fn() -> S>`, a
/// function-pointer phantom so that `Copy` / `Send` / `Sync` hold for every
/// `S` unconditionally.
pub struct RObject<S>(PhantomData<fn() -> S>);

/// The `S = f64` object-kind marker — the default instantiation of [`RObject`].
pub type F64Object = RObject<f64>;

/// Morphism-kind marker for [`RMonoidal`] / [`RActegory`].
///
/// Morphisms of the module category are `R`-linear maps, carried at the value
/// level; this is the type-level witness, mirroring
/// [`SetMorphism`](super::SetMorphism). Zero-sized, with the same
/// function-pointer phantom as [`RObject`].
pub struct RMorphism<S>(PhantomData<fn() -> S>);

/// The `S = f64` morphism-kind marker — the default instantiation of
/// [`RMorphism`].
pub type F64Morphism = RMorphism<f64>;

/// The monoidal category `(FinReal, ⊕, R⁰)` of finite-dimensional modules over
/// the scalar ring `S` under **direct sum**.
///
/// The first non-`(Set, ×, 1)` [`MonoidalCategory`] instance. Objects are
/// [`RModule<S>`]s; the tensor `⊗ = ⊕` is the [`DirectSum`] carrier; the unit
/// `I` is the zero module `R⁰`, represented by `()` (its one element). The
/// associator and unitors are exact `DirectSum` re-associations.
///
/// CDL Definition E.2 / Example E.4 / Example G.3. See the module docs for the
/// `⊕`-vs-`⊗_R` decision and the base-ring-as-type note.
///
/// Zero-sized: the base ring is the compile-time type parameter `S` (the
/// crate's default instantiation is [`F64Monoidal`]), so no runtime payload is
/// carried. Does **not** opt into
/// [`SetCategoryDefaults`](super::SetCategoryDefaults) — the impl is
/// hand-written with `DirectSum` bodies.
pub struct RMonoidal<S>(PhantomData<fn() -> S>);

/// The `S = f64` monoidal category `(FinReal, ⊕, R⁰)` — the default
/// instantiation of [`RMonoidal`].
pub type F64Monoidal = RMonoidal<f64>;

/// The self-action `▶ = ⊕` of [`RMonoidal`] on itself — the `R`-module
/// actegory.
///
/// CDL Example E.4 (a monoidal category acts on itself). The action of a
/// parameter module `P` on a carrier module `X` is the direct sum `P ⊕ X`; the
/// multiplicator `µ : Q ▶ (P ▶ X) → (Q ⊗ P) ▶ X` is the exact `DirectSum`
/// re-association matching [`RMonoidal`]'s tensor. This is the actegory the
/// gradient-based-learning `Para(RMonoidal<S>, RActegory<S>)` construction runs
/// over (Example G.3), where parameter concatenation `⊕` composes learnable
/// weights.
///
/// Zero-sized (see [`RMonoidal`]); does not opt into any blanket.
pub struct RActegory<S>(PhantomData<fn() -> S>);

/// The `S = f64` self-action — the default instantiation of [`RActegory`].
pub type F64Actegory = RActegory<f64>;

/// Hand-written `Debug` / `Default` / `Clone` / `Copy` / `PartialEq` / `Eq` for
/// the zero-sized phantom markers. Derives would add a spurious `S: …` bound to
/// types that carry no `S` value at all.
macro_rules! zst_phantom_impls {
    ($($ty:ident),+ $(,)?) => {$(
        impl<S> core::fmt::Debug for $ty<S> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(stringify!($ty))
            }
        }

        impl<S> Default for $ty<S> {
            fn default() -> Self {
                Self(PhantomData)
            }
        }

        impl<S> Clone for $ty<S> {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<S> Copy for $ty<S> {}

        impl<S> PartialEq for $ty<S> {
            fn eq(&self, _other: &Self) -> bool {
                true
            }
        }

        impl<S> Eq for $ty<S> {}
    )+};
}

zst_phantom_impls!(RObject, RMorphism, RMonoidal, RActegory);

impl<S> RMonoidal<S> {
    /// Construct a fresh `RMonoidal<S>`. Zero-sized; cost-free.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<S> MonoidalCategory for RMonoidal<S> {
    type Object = RObject<S>;
    type Morphism = RMorphism<S>;
    /// The monoidal unit `I = R⁰`. `R⁰` is a one-element module, so `()` is its
    /// faithful carrier (concretely [`RModule::zero_dim`]).
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

    fn tensor_morphisms<A, B, C, D>(
        &self,
        ab: DirectSum<A, B>,
        mut f: impl FnMut(A) -> C,
        mut g: impl FnMut(B) -> D,
    ) -> DirectSum<C, D> {
        // f ⊗ g : (A ⊕ B) → (C ⊕ D) — map each summand.
        let DirectSum(a, b) = ab;
        DirectSum(f(a), g(b))
    }
}

impl<S> RActegory<S> {
    /// Construct a fresh `RActegory<S>`. Zero-sized; cost-free.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<S> Actegory<RMonoidal<S>> for RActegory<S> {
    type Object = RObject<S>;
    type Morphism = RMorphism<S>;
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
    ) -> Self::ActionResult<<RMonoidal<S> as MonoidalCategory>::Tensor<Q, P>, X> {
        // µ : Q ▶ (P ▶ X) = Q ⊕ (P ⊕ X)  →  (Q ⊗ P) ▶ X = (Q ⊕ P) ⊕ X.
        DirectSum(DirectSum(q, p), x)
    }
}
