//! The `R`-module actegory `(FinReal, ⊕, R⁰)` acting on itself.
//!
//! Anchors (arXiv:2402.15332v2): Def E.2 (actegory — `▶ : M × C → C`,
//! unitor `η_X : I ▶ X ≅ X`, multiplicator `µ_{M,N} : (M ⊗ N) ▶ X ≅ M ▶ (N ▶ X)`,
//! Eq. 7–8) is what [`Actegory`] models ([`Actegory::act`] = `▶`,
//! [`Actegory::compose_action`] = `µ`); Ex E.4 (self-action of a monoidal
//! category) is [`RActegory`] with `▶ = ⊗ = ⊕`; Ex G.3 (`Para(Smooth)` over the
//! **cartesian** structure of real vector spaces) fixes the product as the
//! biproduct `Rᵐ × Rⁿ ≅ Rᵐ⁺ⁿ`, i.e. `⊕` with unit `R⁰`.
//!
//! Carriers: [`RModule<S>`] (`Vec<S>`-backed `Sⁿ`; `zeros`, `basis`, `add`,
//! `scale`, `direct_sum`; `Zero`/`One` supply `0` and `1`), the tensor
//! [`DirectSum`] with [`DirectSum::flatten`] concatenating coordinates.
//! [`RMonoidal<S>`] / [`RActegory<S>`] are zero-sized with the ring as the
//! type parameter `S`; [`F64Monoidal`] / [`F64Actegory`] / [`F64Module`] are
//! the `S = f64` aliases. Associator and unitors are exact re-associations.

use core::marker::PhantomData;
use core::ops::{Add, Mul};

use catgraph_applied::rig::{One, Zero};

use super::actegory::Actegory;
use super::monoidal_category::MonoidalCategory;

/// Direct-sum tensor carrier `A ⊕ B`: the tensor of [`RMonoidal`] and the
/// action result of [`RActegory`] (CDL Def E.2 / Ex E.4). With `serde` it
/// round-trips whenever both summands do; it carries no cross-slot invariant.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DirectSum<A, B>(pub A, pub B);

impl<S> DirectSum<RModule<S>, RModule<S>> {
    /// `V ⊕ W` as the concatenated module `Rᵐ⁺ⁿ`, left coordinates first.
    #[must_use]
    pub fn flatten(self) -> RModule<S> {
        self.0.direct_sum(self.1)
    }
}

/// Free module `Sⁿ` over the scalar ring `S`, `Vec<S>`-backed (CDL Def E.2,
/// Ex G.3): [`zeros`](RModule::zeros), [`basis`](RModule::basis),
/// dimension-guarded [`add`](RModule::add), [`scale`](RModule::scale),
/// [`direct_sum`](RModule::direct_sum). Bounds are per method.
/// [`F64Module`] is `S = f64`.
///
/// For float `S`: equality is `PartialEq` on coordinates, so `-0.0 == +0.0`
/// but signed-zero bit patterns are not preserved by `scale`/`add`, and
/// `add`/`scale` are not associative or distributive on the nose.
///
/// # Serde (feature `serde`)
///
/// Round-trips as the bare coordinate vector for any `S` with serde impls
/// (`f64`, std primitives, `Dual<f64>` under `ad`; not the `catgraph-applied`
/// rig scalars). Deserialization checks nothing beyond what [`RModule::new`]
/// accepts: [`dim`](Self::dim) is the payload's length; only
/// [`add`](Self::add) rejects a mismatch; the wire shape carries no type tag
/// (`[0.5, 1.5]` reads back as an `RModule<f64>` of dim 2, a
/// [`DirectSum<f64, f64>`](DirectSum), or a `Dual<f64>`); `serde_json`
/// writes non-finite scalars as `null`, which does not read back into `f64`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
