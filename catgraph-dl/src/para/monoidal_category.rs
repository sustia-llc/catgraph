//! Monoidal category of parameter spaces.
//!
//! CDL §3.1: `Para(M, C)` is parameterised by a monoidal category
//! `(M, ⊗, I)`. This module exposes the [`MonoidalCategory`] trait surface and
//! provides the concrete instance [`SetMonoidal`] — the monoidal category
//! `(Set, ×, 1)` with object-level tensor `(A, B) ↦ (A, B)` and unit
//! `1 = ()`. CDL takes `(Set, ×, 1)` as the default and it is the only
//! instance currently shipped.
//!
//! ## HKT shape
//!
//! Rust has no kind `* -> * -> *`, so the object-level tensor is encoded as
//! a Generic Associated Type [`MonoidalCategory::Tensor`]. For
//! [`SetMonoidal`] this projects to the Rust tuple `(A, B)`; for the unit
//! object [`SetMonoidal::Unit`] projects to `()`.
//!
//! Coherence isomorphisms (associator, unitors) are exposed as method
//! signatures on values; for `SetMonoidal` they are concrete tuple
//! re-associations and are exact (not "up to iso"). Future instances
//! over richer monoidal categories will need these slots, so the trait
//! surface is widened now to keep their addition a non-breaking transition for
//! [`SetMonoidal`] consumers.
//!
//! Closure convention across the `para` module: `Fn((P, X)) -> Y`
//! (tuple-as-single-argument). This mirrors the `architectures::*` scaffold
//! which uses `fn((f32, u8, u32)) -> u32` etc.
//!
//! ## Why methods take `&self`
//!
//! All five [`MonoidalCategory`] methods (`tensor_objects`, `unit`,
//! `associate`, `left_unitor`, `right_unitor`) take `&self` even though the
//! only shipped instance ([`SetMonoidal`]) is a zero-sized type for which
//! the receiver is unobservable. The `&self` slot is a deliberate
//! future-proofing choice: future instances over richer monoidal categories
//! will carry **runtime data** — an `R`-module instance carries its base
//! ring; a hyperdoctrine instance carries the fibration's projection map; a
//! vector-bundle instance carries its connection. Freezing the trait surface
//! today at "static methods only" would force a breaking change later when
//! those non-`(Set, ×, 1)` instances land.
//!
//! This is a narrow divergence from the `causality:hkt-type-system`
//! convention in `deep_causality_haft`'s core `Functor`/`Monad`
//! witnesses, which use static dispatch exclusively (`VecWitness::fmap(v,
//! f)`, not `v.fmap(f)`, with the witness as a type-level token never
//! instantiated). HAFT itself accommodates runtime payload via a separate
//! `Context` type parameter — e.g. `Adjunction<L, R, Context>` (unified,
//! replacing `BoundedAdjunction`) and the `Effect5::Fixed{1..4}`
//! payload slots. cg-dl's [`MonoidalCategory`] folds the equivalent
//! runtime-payload role into the `&self` receiver instead of carrying a
//! separate `Context` parameter — so the divergence is in the *placement*
//! of runtime data (receiver vs context-parameter), not in HAFT's
//! capacity to carry runtime data at all. cg-dl's choice keeps the API
//! shape aligned with idiomatic Rust trait-method calls (`monoidal.tensor_objects(a, b)`)
//! at the cost of forgoing HAFT's witness-first static-dispatch
//! convention.
//!
//! Since #12, cg-dl's *endofunctor* layer (`ListEndo` / `TreeEndo` /
//! `GroupActionEndo`) adopts HAFT's `HKT` / `Functor` witnesses directly and
//! follows the witness-first static-dispatch convention (`ListEndo::fmap(x,
//! f)`). [`MonoidalCategory`] is therefore the deliberate *local* exception:
//! it keeps the `&self` receiver for the runtime-payload slot while the rest
//! of the crate aligns with HAFT's static dispatch.
//!
//! See `causality:hkt-type-system` skill Gotcha #6 ("Static dispatch only —
//! call `VecWitness::fmap(v, f)`, never `v.fmap(f)`. The witness is a
//! type-level token.") for the HAFT `Functor`/`Monad` static-only side of
//! the comparison; cg-dl deliberately diverges here for the R-module /
//! hyperdoctrine / vector-bundle slot, picking up `&self` payload where
//! HAFT would use a `Context` type parameter.
//!
//! **Rationale validation:** a downstream coalition
//! `impl Actegory<SetMonoidal>` for `UnitIntervalQ` / `TropicalQ` /
//! `QuantaleDefault` is the first consumer expected to carry
//! runtime data (Tropical zero / one for the underlying min-plus
//! semiring; BTV21 free-monoid generator references; Lawvere-metric
//! embedding parameter). The shipped surface commits to the `&self` slot for
//! future-proofing; the audit checkpoint fires at that consumer's
//! post-shipping review and either ratifies the choice or opens a
//! follow-up to consider static dispatch. See
//! [`AUDIT-CHECKPOINT-v0.4.0.md`](../../docs/AUDIT-CHECKPOINT-v0.4.0.md)
//! for audit criteria.
//!
//! ## `(Set, ×, 1)`-flavoured ZSTs via [`SetCategoryDefaults`]
//!
//! Defining a fresh `(Set, ×, 1)`-flavoured ZST instance (with `Tensor<A, B>
//! = (A, B)` and `Unit = ()`) used to require reproducing all five method
//! bodies pointwise. The [`SetCategoryDefaults`] opt-in marker trait now
//! supplies the bodies via a blanket impl: downstream users write
//! `impl SetCategoryDefaults for MyMonoidal {}` (empty body) and the
//! [`MonoidalCategory`] impl is automatic. [`SetMonoidal`] itself uses this
//! path. See [`SetCategoryDefaults`] for the full design rationale, the
//! `: Sized` bound rationale, the conflict-guard caveat for downstream
//! users, and a doctest exercising all five method bodies.

use core::marker::PhantomData;

/// A monoidal category `(M, ⊗, I)` of parameter spaces.
///
/// CDL §3.1 — the parameter category for the 2-category `Para(M, C)`.
///
/// The trait carries:
///
/// - [`MonoidalCategory::Object`] — the *kind* of objects of `M` (use a
///   marker type when objects are themselves generic, as in `SetMonoidal`).
/// - [`MonoidalCategory::Morphism`] — the kind of morphisms of `M`.
/// - [`MonoidalCategory::Unit`] — the monoidal unit `I`.
/// - [`MonoidalCategory::Tensor`] — the object-level tensor product GAT
///   `(A, B) ↦ A ⊗ B`.
/// - Coherence isomorphisms [`MonoidalCategory::associate`],
///   [`MonoidalCategory::left_unitor`], [`MonoidalCategory::right_unitor`].
///
/// For `SetMonoidal`, the GATs project to Rust tuples and the coherence
/// isomorphisms are exact, not "up to iso".
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
}

/// Sealing module for [`SetCategoryDefaults`].
///
/// Downstream users opting into the `(Set, ×, 1)` blanket via
/// [`SetCategoryDefaults`] must ALSO `impl Sealed for MyMonoidal {}` —
/// the dual-impl requirement is the "soft seal" that surfaces the
/// commitment-to-`(Set, ×, 1)` decision at the impl site. See the
/// "## ⚠️ Soft-seal" section in [`SetCategoryDefaults`]'s
/// rustdoc for the full rationale.
pub mod private {
    /// Sealing trait for [`super::SetCategoryDefaults`]. Implementing this
    /// trait signals deliberate commitment to the canonical
    /// `(Set, ×, 1)`-flavoured `MonoidalCategory` body supplied by the
    /// blanket impl. See the "## ⚠️ Soft-seal" section in
    /// [`super::SetCategoryDefaults`]'s rustdoc.
    pub trait Sealed {}
}

/// Opt-in marker trait for `(Set, ×, 1)`-flavoured monoidal-category ZSTs.
///
/// CDL §3.1 default. Implementing `SetCategoryDefaults` for a zero-sized
/// type opts the type into a blanket [`MonoidalCategory`] impl that fixes:
///
/// - [`MonoidalCategory::Object`] = [`SetObject`]
/// - [`MonoidalCategory::Morphism`] = [`SetMorphism`]
/// - [`MonoidalCategory::Unit`] = `()`
/// - [`MonoidalCategory::Tensor<A, B>`] = `(A, B)`
///
/// All five method bodies (`tensor_objects`, `unit`, `associate`,
/// `left_unitor`, `right_unitor`) are supplied by the blanket impl as the
/// canonical Cartesian-product / tuple-re-association forms. Downstream
/// users defining a fresh `(Set, ×, 1)`-flavoured naming-witness ZST get
/// `MonoidalCategory` for free without reproducing the bodies.
///
/// ## `: Sized` bound
///
/// The trait carries a `: Sized` supertrait bound. This is a soft witness
/// that the trait is intended for **zero-sized witness types** (or other
/// `Sized` carriers) — the canonical `(Set, ×, 1)` flavour does not need a
/// runtime-sized payload. A downstream attempt to write
/// `impl SetCategoryDefaults for &'a [u8]` (or any `?Sized` carrier) will
/// fail at the bound site rather than silently picking up the blanket and
/// surprising the caller later. The bound costs nothing at the canonical
/// shipping call sites: [`SetMonoidal`] is a unit struct (`Sized` via the
/// default `Sized` bound); the doctest's `MyMonoidal` is too.
///
/// ## ⚠️ Soft-seal
///
/// `SetCategoryDefaults` carries a `: private::Sealed` supertrait bound.
/// Downstream users must `impl Sealed for MyMonoidal {}` AND
/// `impl SetCategoryDefaults for MyMonoidal {}` (two impls) to opt into
/// the `(Set, ×, 1)` blanket. The dual-impl requirement is the
/// commitment-signalling mechanism: a downstream user who writes only
/// `impl SetCategoryDefaults for MyMonoidal {}` (without the parallel
/// `Sealed` impl) gets a clear `Sealed: not satisfied` diagnostic at the
/// impl site, rather than the harder-to-diagnose
/// `conflicting implementations of MonoidalCategory` coherence error
/// that the conflict-guard caveat (below) warns about.
///
/// Rationale: the earlier `Sized`-only bound let a downstream user collide
/// `impl SetCategoryDefaults for MyType {}` + a hand-rolled
/// `impl MonoidalCategory for MyType { ... }` and discover the
/// coherence error LATE (the diagnostic does not name
/// `SetCategoryDefaults` as the proximal cause). The soft-seal
/// surfaces the commitment at compile time at the impl site, where the
/// fix is local and the diagnostic is direct. See
/// [`private::Sealed`] for the trait. The sealing approach (option (a))
/// was chosen over the rejected alternatives.
///
/// ## ⚠️ Conflict-guard caveat (superseded by the soft-seal but still valid)
///
/// **Implementing `SetCategoryDefaults` for a type commits the type to the
/// canonical `(Set, ×, 1)` `MonoidalCategory` body via the blanket impl.**
/// A downstream user who writes both
///
/// ```text
/// impl Sealed for MyType {}
/// impl SetCategoryDefaults for MyType {}
/// impl MonoidalCategory for MyType { type Tensor<A, B> = SomethingElse; ... }
/// ```
///
/// will hit a `conflicting implementations of trait MonoidalCategory for
/// type MyType` compile error from the trait-coherence checker (the
/// soft-seal does not prevent this third case — a deliberate `Sealed`
/// impl + a deliberate hand-rolled `MonoidalCategory` impl is the bypass
/// path the seal does not block). The diagnostic does not name
/// `SetCategoryDefaults` as the proximal cause, so the convention is
/// **don't combine the two impl paths**. If a non-`(Set, ×, 1)`-flavoured
/// `MonoidalCategory` impl is wanted, write `impl MonoidalCategory for
/// MyType { ... }` directly WITHOUT `SetCategoryDefaults` / `Sealed`. If a
/// `(Set, ×, 1)`-flavoured impl is wanted, opt into the dual-impl
/// (`Sealed` + `SetCategoryDefaults`) and let the blanket supply the body.
///
/// ## Implementation note (option (γ-ii))
///
/// At design phase, three options were considered for supplying the default
/// bodies:
///
/// - **(α)** marker trait + blanket impl gated on the marker.
/// - **(β)** `#[derive(SetMonoidal)]` proc-macro.
/// - **(γ)** sub-trait with GAT default-method bodies on `MonoidalCategory`.
///
/// Option (γ) as originally sketched (sub-trait with default-method bodies
/// inherited by the supertrait's impls) does not type-check on stable Rust:
/// a sub-trait cannot override a supertrait's method bodies. The closest
/// equivalent that compiles is (γ-ii) — a blanket impl carrying the bodies,
/// gated by `SetCategoryDefaults` opt-in. Functionally this is a renamed
/// (α) since both use a marker-with-blanket-impl pattern; the trade-off is
/// essentially zero, and the `SetCategoryDefaults` name better signals the
/// "(Set, ×, 1)-flavoured defaults" intent than a generic `Marker` name.
///
/// # Examples
///
/// Defining a fresh `(Set, ×, 1)`-flavoured ZST and getting
/// [`MonoidalCategory`] for free — exercises all five method bodies:
///
/// ```
/// use catgraph_dl::para::{MonoidalCategory, Sealed, SetCategoryDefaults};
///
/// #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
/// struct MyMonoidal;
///
/// // Dual-impl soft-seal: Sealed first, then SetCategoryDefaults.
/// impl Sealed for MyMonoidal {}
/// impl SetCategoryDefaults for MyMonoidal {}
///
/// // MonoidalCategory comes for free via the blanket impl. All five
/// // method bodies are live; verify each one.
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
