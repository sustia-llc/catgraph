//! The shared `EndoFunctor` trait — the minimal "Functor" type-class
//! used by both [`crate::algebra`] (F-algebras and their homomorphisms)
//! and [`crate::free_monad`] (recursive `FreeMnd`/`CofreeCmnd`).
//!
//! This module is the canonical home of `EndoFunctor` after the Phase
//! DL-2 reconciliation. Agents C and D both introduced near-identical
//! local copies (one in `algebra/group_action.rs`, one in
//! `free_monad/free_mnd.rs`); both have been replaced with re-exports
//! pointing here.
//!
//! # The trait
//!
//! ```text
//! trait EndoFunctor {
//!     type Apply<X>;
//!     fn fmap<X, Y, G>(fx: Apply<X>, f: G) -> Apply<Y>
//!     where G: Fn(X) -> Y;
//! }
//! ```
//!
//! `Apply<X>` is the object map of the endofunctor (a Generic Associated
//! Type — the same HKT-emulation pattern Agent A used for
//! `MonoidalCategory::Tensor` and `Actegory::ActionResult`). `fmap` is
//! the morphism map; implementors must guarantee the **functor laws**:
//!
//! ```text
//! fmap(fx, |x| x) == fx                             (identity)
//! fmap(fmap(fx, f), g) == fmap(fx, |x| g(f(x)))     (composition)
//! ```
//!
//! These are documented obligations, not machine-checked at compile
//! time. A non-functorial `EndoFunctor` is a soundness defect — it will
//! cause F-algebra homomorphism diagrams to fail to commute even for
//! morphisms that "should" commute.
//!
//! # Why named-generic `G: Fn(X) -> Y` rather than `impl Fn(X) -> Y`
//!
//! Both forms are caller-equivalent. The named-generic form is slightly
//! more flexible for downstream `for<F: EndoFunctor>` quantification and
//! is consistent with Agent A's `MonoidalCategory::tensor_objects<A, B>`
//! pattern. Picked per Agent C's recommendation in their final report.
//!
//! # Concrete instances in the workspace
//!
//! | Endofunctor | Type | `Apply<X>` |
//! |---|---|---|
//! | `1 + A × −` | `crate::free_monad::list_endo::ListEndo<A>` | `Option<(A, X)>` |
//! | `A + (−)²` | `crate::free_monad::tree_endo::TreeEndo<A>` | `Either<A, (X, X)>` |
//! | `G × −` | [`crate::algebra::GroupActionEndo<G>`] | `(G, X)` |

/// A minimal "Functor" trait — the categorical endofunctor `F : C → C`
/// with `C = Set`, encoded via a GAT.
///
/// See the module-level documentation for laws and concrete instances.
pub trait EndoFunctor {
    /// The object map: apply the endofunctor to a type `X`.
    ///
    /// For `ListEndo<A>` this is `Option<(A, X)>`; for `TreeEndo<A>` this
    /// is `Either<A, (X, X)>`; for `GroupActionEndo<G>` this is `(G, X)`.
    type Apply<X>;

    /// The morphism map: lift `f : X → Y` to `F(X) → F(Y)`.
    ///
    /// Implementors must satisfy the functor laws (identity,
    /// composition); see the module-level documentation.
    fn fmap<X, Y, G>(fx: Self::Apply<X>, f: G) -> Self::Apply<Y>
    where
        G: Fn(X) -> Y;
}
