//! The free monad `FreeMnd(F)(Z) = Fix(X ↦ F(X) + Z)`.
//!
//! CDL Proposition B.18. The free monad on an endofunctor `F` is the
//! least fixed point of `X ↦ F(X) + Z`, i.e. the inductive type built from
//! two constructors:
//!
//! ```text
//! Pure : Z       → FreeMnd(F)(Z)
//! Roll : F(FreeMnd(F)(Z)) → FreeMnd(F)(Z)
//! ```
//!
//! ## HKT shape
//!
//! Rust has no kind `* -> *`, so the endofunctor is encoded as a
//! [`deep_causality_haft`] witness — object map
//! [`HKT::Type`](crate::endofunctor::HKT::Type), morphism map
//! [`Functor::fmap`](crate::endofunctor::Functor::fmap) — bundled by the
//! [`EndoWitness`] alias (`HKT<Constraint = NoConstraint> + Functor<Self>`).
//! `FreeMnd` bounds `F: EndoWitness` so the type system enforces that `F` is a
//! genuine endofunctor (not a bare object map), and — because the constraint is
//! [`NoConstraint`](crate::endofunctor::NoConstraint) — the recursive `Type`
//! projection is universally nameable (the blanket admits any inner type).
//!
//! ## Recursion encoding
//!
//! `FreeMnd<F, Z>` is recursive (`Roll` carries a value of type
//! `F::Type<FreeMnd<F, Z>>`), so the variant must be heap-indirected. We
//! `Box` the entire applied functor — concrete instances such as
//! [`super::list_endo::ListEndo`] then carry their own indirection only for
//! the recursive slot, not the whole functorial value.
//!
//! ## Functor laws
//!
//! `F: EndoWitness` must satisfy the identity + composition functor laws — see
//! [`crate::endofunctor`] for the canonical statement (and the pure-morphism
//! caveat). Not runtime-checked: we never invoke `fmap` during construction or
//! destruction of `FreeMnd`; a violator produces a type-legal but
//! mathematically meaningless free monad.

use crate::endofunctor::EndoWitness;

/// The free monad `FreeMnd(F)(Z) = Fix(X ↦ F(X) + Z)`.
///
/// CDL Proposition B.18. Two constructors:
///
/// - [`FreeMnd::Pure`] — embed a `Z` value (the "leaf" / terminator slot).
/// - [`FreeMnd::Roll`] — wrap an `F(FreeMnd(F)(Z))` value (the recursive
///   "node" slot).
///
/// The `Box` indirection on the `Roll` variant is required by Rust's
/// finite-size discipline: `FreeMnd<F, Z>` recursively contains itself
/// inside `F::Type<…>`.
///
/// # Examples
///
/// The empty list (terminator `()`) is `FreeMnd::Pure(())`. The two-element
/// list `[1, 2]` over `ListEndo<u32>` is the explicit cons-cell tower
///
/// ```text
/// FreeMnd::Roll(Box::new(Some((1, FreeMnd::Roll(Box::new(Some((2, FreeMnd::Pure(()))))))))))
/// ```
///
/// See [`super::list_endo`] for the `Vec`-bijection helpers and
/// [`super::tree_endo`] for the analogous binary-tree helpers.
///
/// # Constraint restriction
///
/// The `F: EndoWitness` bound fixes `Constraint = NoConstraint`, so only
/// *unconstrained* haft witnesses can instantiate `FreeMnd` — a witness whose
/// `Constraint != NoConstraint` (e.g. a hypothetical `CausalTensor`-style
/// carrier with an inner-type bound) is rejected. This is deliberate: CDL's
/// ambient category is `Set`, and all three shipped witnesses are
/// unconstrained. The fully general self-referential form — bounding each
/// impl on `where FreeMnd<F, Z>: Satisfies<F::Constraint>` — was considered and
/// rejected as viral: it would thread that clause through the enum plus every
/// manual `Clone`/`Debug`/`PartialEq`/`Eq` impl for no practical gain.
pub enum FreeMnd<F: EndoWitness, Z> {
    /// Embed a `Z` value (terminator). Categorically the unit of the free
    /// monad: `η_Z : Z → FreeMnd(F)(Z)`.
    Pure(Z),
    /// Wrap an `F`-applied recursive structure. Categorically the algebra
    /// map of the initial algebra of `X ↦ F(X) + Z` restricted to the `F(X)`
    /// summand.
    Roll(Box<F::Type<FreeMnd<F, Z>>>),
}

impl<F: EndoWitness, Z> FreeMnd<F, Z> {
    /// Construct the `Pure(z)` leaf.
    pub fn pure(z: Z) -> Self {
        Self::Pure(z)
    }

    /// Construct the `Roll(fx)` node by boxing the supplied `F`-applied
    /// recursive structure.
    pub fn roll(fx: F::Type<FreeMnd<F, Z>>) -> Self {
        Self::Roll(Box::new(fx))
    }
}

// `Default` over `Z: Default` — produces `Pure(Z::default())`. This keeps
// the `FreeMnd::new()` compatibility constructor surface usable by
// `tests/scaffold_smoke.rs::free_monad_witnesses_construct`, where every
// reference uses `Z = ()` (i.e. the unit terminator). Callers should
// prefer the explicit `pure` / `roll` constructors.
impl<F: EndoWitness, Z: Default> Default for FreeMnd<F, Z> {
    fn default() -> Self {
        Self::Pure(Z::default())
    }
}

impl<F: EndoWitness, Z: Default> FreeMnd<F, Z> {
    /// Compatibility constructor — `Pure(Z::default())`.
    ///
    /// Prefer the explicit [`FreeMnd::pure`] / [`FreeMnd::roll`]
    /// constructors which carry actual data. This entry point is retained
    /// solely to keep `tests/scaffold_smoke.rs` compiling.
    #[must_use]
    pub fn new() -> Self {
        Self::Pure(Z::default())
    }
}

// `Clone` is a manual impl: the `derive(Clone)` macro emits a bound
// `F::Type<FreeMnd<F, Z>>: Clone` that the trait-resolution machinery
// can't always discharge through the GAT projection. The hand-rolled impl
// states the bound directly on `F::Type<Self>`.
impl<F: EndoWitness, Z: Clone> Clone for FreeMnd<F, Z>
where
    F::Type<FreeMnd<F, Z>>: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Pure(z) => Self::Pure(z.clone()),
            Self::Roll(fx) => Self::Roll(fx.clone()),
        }
    }
}

// Same for `Debug` — `derive(Debug)` would synthesise a bound that
// trait-resolution rejects through the GAT.
impl<F: EndoWitness, Z: core::fmt::Debug> core::fmt::Debug for FreeMnd<F, Z>
where
    F::Type<FreeMnd<F, Z>>: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Pure(z) => f.debug_tuple("Pure").field(z).finish(),
            Self::Roll(fx) => f.debug_tuple("Roll").field(fx).finish(),
        }
    }
}

// `PartialEq` follows the same manual-bound pattern.
impl<F: EndoWitness, Z: PartialEq> PartialEq for FreeMnd<F, Z>
where
    F::Type<FreeMnd<F, Z>>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Pure(a), Self::Pure(b)) => a == b,
            (Self::Roll(a), Self::Roll(b)) => a == b,
            _ => false,
        }
    }
}

impl<F: EndoWitness, Z: Eq> Eq for FreeMnd<F, Z> where F::Type<FreeMnd<F, Z>>: Eq {}
