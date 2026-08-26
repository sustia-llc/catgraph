//! Monoidal category `(M, ⊗, I)` of parameter spaces (CDL §3.1): the
//! [`MonoidalCategory`] trait, the `(Set, ×, 1)` instance [`SetMonoidal`]
//! (`Tensor<A, B> = (A, B)`, `Unit = ()`), and the [`SetCategoryDefaults`]
//! opt-in blanket for further `(Set, ×, 1)`-flavoured ZSTs. Methods take
//! `&self`; the ring-valued instance is [`RMonoidal`](super::RMonoidal).
//!
//! Closure convention across `para`: `Fn((P, X)) -> Y`.

use core::marker::PhantomData;

/// A monoidal category `(M, ⊗, I)` (CDL §3.1): object/morphism kind markers,
/// [`Unit`](MonoidalCategory::Unit), the tensor GAT
/// [`Tensor`](MonoidalCategory::Tensor), the associator and unitors, and the
/// morphism tensor. Implementors must satisfy Mac Lane's pentagon and
/// triangle, with `α` = [`associate`](MonoidalCategory::associate),
/// `λ` = [`left_unitor`](MonoidalCategory::left_unitor),
/// `ρ` = [`right_unitor`](MonoidalCategory::right_unitor):
///
/// ```text
/// Pentagon (on ((A ⊗ B) ⊗ C) ⊗ D):
///   α_{A,B,C⊗D} ∘ α_{A⊗B,C,D}
///     = (id_A ⊗ α_{B,C,D}) ∘ α_{A,B⊗C,D} ∘ (α_{A,B,C} ⊗ id_D)
///
/// Triangle (on (A ⊗ I) ⊗ B):
///   (id_A ⊗ λ_B) ∘ α_{A,I,B} = ρ_A ⊗ id_B
/// ```
pub trait MonoidalCategory {
    /// Marker for the kind of objects of `M`. For `SetMonoidal` this is the
    /// uninhabited [`SetObject`] tag — actual objects are Rust types
    /// `A: 'static` carried as type parameters at the value level.
    type Object;

    /// Marker for the kind of morphisms of `M`. For `SetMonoidal` this is
    /// the uninhabited [`SetMorphism`] tag — actual morphisms are Rust
    /// closures carried at the value level.
    type Morphism;

    /// The monoidal unit `I`. For `SetMonoidal` this is `()`.
    type Unit;

    /// The object-level tensor product `A ⊗ B`. For `SetMonoidal` this is
    /// the Rust tuple `(A, B)`.
    type Tensor<A, B>;

    /// Object-level tensor of two values: pair them.
    ///
    /// CDL §3.1 — the object map of `⊗ : M × M → M`. For `SetMonoidal` this
    /// is `(a, b) ↦ (a, b)`.
    fn tensor_objects<A, B>(&self, a: A, b: B) -> Self::Tensor<A, B>;

    /// The monoidal unit `I`. For `SetMonoidal` this returns `()`.
    fn unit(&self) -> Self::Unit;

    /// Associator coherence isomorphism `α : (A ⊗ B) ⊗ C → A ⊗ (B ⊗ C)`.
    ///
    /// For `SetMonoidal` this is the tuple re-association
    /// `((a, b), c) ↦ (a, (b, c))`.
    fn associate<A, B, C>(
        &self,
        nested: Self::Tensor<Self::Tensor<A, B>, C>,
    ) -> Self::Tensor<A, Self::Tensor<B, C>>;

    /// Left unitor coherence `λ : I ⊗ A → A`.
    ///
    /// For `SetMonoidal` this is `((), a) ↦ a`.
    fn left_unitor<A>(&self, paired: Self::Tensor<Self::Unit, A>) -> A;

    /// Right unitor coherence `ρ : A ⊗ I → A`.
    ///
    /// For `SetMonoidal` this is `(a, ()) ↦ a`.
    fn right_unitor<A>(&self, paired: Self::Tensor<A, Self::Unit>) -> A;

    /// Morphism-level tensor `f ⊗ g : (A ⊗ B) → (C ⊗ D)` in applying form:
    /// map the two components of a tensored value by `f` and `g` respectively.
    /// CDL §3.1 — the morphism map of `⊗ : M × M → M`. For `SetMonoidal` this is
    /// `((a, b)) ↦ (f(a), g(b))`.
    fn tensor_morphisms<A, B, C, D>(
        &self,
        ab: Self::Tensor<A, B>,
        f: impl FnMut(A) -> C,
        g: impl FnMut(B) -> D,
    ) -> Self::Tensor<C, D>;
}

/// Sealing module for [`SetCategoryDefaults`]: an opt-in needs both
/// `impl Sealed for T {}` and `impl SetCategoryDefaults for T {}`.
pub mod private {
    /// Sealing trait for [`super::SetCategoryDefaults`].
    pub trait Sealed {}
}

/// Opt-in marker for `(Set, ×, 1)`-flavoured ZSTs: a blanket
/// [`MonoidalCategory`] impl with `Object = `[`SetObject`],
/// `Morphism = `[`SetMorphism`], `Unit = ()`, `Tensor<A, B> = (A, B)` and
/// tuple bodies for every method. Requires `impl `[`private::Sealed`]` for T {}`
/// alongside. A type with this impl cannot also implement `MonoidalCategory`
/// by hand (coherence conflict).
///
/// # Examples
///
/// ```
/// use catgraph_dl::para::{MonoidalCategory, Sealed, SetCategoryDefaults};
///
/// #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
/// struct MyMonoidal;
///
/// impl Sealed for MyMonoidal {}
/// impl SetCategoryDefaults for MyMonoidal {}
///
/// let m = MyMonoidal;
/// assert_eq!(m.tensor_objects(1_i32, "two"), (1_i32, "two"));
/// assert_eq!(m.unit(), ());
/// assert_eq!(m.associate(((1_i32, "two"), 3.0_f64)), (1_i32, ("two", 3.0_f64)));
/// assert_eq!(m.left_unitor::<i32>(((), 42_i32)), 42_i32);
/// assert_eq!(m.right_unitor::<i32>((99_i32, ())), 99_i32);
/// ```
pub trait SetCategoryDefaults: private::Sealed + Sized {}

impl<T: SetCategoryDefaults> MonoidalCategory for T {
    type Object = SetObject;
    type Morphism = SetMorphism;
    type Unit = ();
    type Tensor<A, B> = (A, B);

    fn tensor_objects<A, B>(&self, a: A, b: B) -> Self::Tensor<A, B> {
        (a, b)
    }

    fn unit(&self) -> Self::Unit {}

    fn associate<A, B, C>(
        &self,
        nested: Self::Tensor<Self::Tensor<A, B>, C>,
    ) -> Self::Tensor<A, Self::Tensor<B, C>> {
        let ((a, b), c) = nested;
        (a, (b, c))
    }

    fn left_unitor<A>(&self, paired: Self::Tensor<Self::Unit, A>) -> A {
        let ((), a) = paired;
        a
    }

    fn right_unitor<A>(&self, paired: Self::Tensor<A, Self::Unit>) -> A {
        let (a, ()) = paired;
        a
    }

    fn tensor_morphisms<A, B, C, D>(
        &self,
        ab: (A, B),
        mut f: impl FnMut(A) -> C,
        mut g: impl FnMut(B) -> D,
    ) -> (C, D) {
        let (a, b) = ab;
        (f(a), g(b))
    }
}

/// Phantom marker witnessing that a type names a monoidal category.
///
/// Used as a type-level tag in `Para<M, C>` even after the
/// [`MonoidalCategory`] body lands — `Para` is a 2-category namespace
/// handle that carries no runtime data.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MonoidalTag<M>(PhantomData<M>);

impl<M> MonoidalTag<M> {
    /// Construct a fresh `MonoidalTag<M>`.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

/// Object-kind marker for [`SetMonoidal`].
///
/// CDL takes `(Set, ×, 1)` as the default monoidal category. Every Rust
/// type `A: 'static` is regarded as a Set object; this marker is the
/// type-level *witness* that `SetMonoidal::Object` is "the kind of Set
/// objects" without committing to one concrete type.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SetObject;

/// Morphism-kind marker for [`SetMonoidal`].
///
/// Mirrors [`SetObject`] — a witness that morphisms in `Set` are Rust
/// closures carried at the value level rather than constrained to one
/// concrete morphism type.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SetMorphism;

/// The monoidal category `(Set, ×, 1)` — Cartesian product on Rust types.
///
/// CDL §3.1 default. Objects are Rust types `A: 'static`; morphisms are
/// Rust closures `Fn(A) -> B`; `⊗` is the tuple constructor; `I = ()`.
///
/// All coherence isomorphisms are *exact* — the tuple re-association
/// `((a, b), c) ↔ (a, (b, c))` and the unitor projections `((), a) ↔ a`
/// are bona-fide bijections in `Set`, not "up to iso" as in a general
/// monoidal category. This makes [`SetMonoidal`] the trivial-coherence
/// reference instance against which future instances will be compared.
///
/// The [`MonoidalCategory`] impl is supplied via the
/// [`SetCategoryDefaults`] blanket: this struct opts in with an empty
/// `impl SetCategoryDefaults for SetMonoidal {}`. The behaviour is
/// pointwise identical to the earlier hand-written impl — the blanket simply
/// hoists the bodies into one place so downstream `(Set, ×, 1)`-flavoured
/// ZSTs can share them.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SetMonoidal;

impl SetMonoidal {
    /// Construct a fresh `SetMonoidal` instance. Zero-sized; cost-free.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

// Dual-impl soft-seal: Sealed first, then SetCategoryDefaults.
impl private::Sealed for SetMonoidal {}
impl SetCategoryDefaults for SetMonoidal {}
