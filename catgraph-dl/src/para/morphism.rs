//! `Para(M, C)` 1-morphisms — parametric maps `(P, f)`.
//!
//! CDL §3.1. A 1-morphism `X → Y` in `Para(M, C)` is a pair
//! `(P ∈ M, f : P ▶ X → Y)`. Sequential composition is
//!
//! ```text
//! (P, f) : X → Y    (Q, g) : Y → Z
//! ─────────────────────────────────
//!         (Q ⊗ P, h) : X → Z
//!
//! where  h : (Q ⊗ P) ▶ X --μ--> Q ▶ (P ▶ X) --Q ▶ f--> Q ▶ Y --g--> Z
//! ```
//!
//! For the [`super::SetActegory`] instance, `▶` is Cartesian product, so
//! every action is a tuple constructor and the compiled `h` is a closure
//! that destructures `((q, p), x)`, applies `f((p, x)) → y`, then applies
//! `g((q, y)) → z`.
//!
//! ## Closure convention
//!
//! Underlying maps `f : P ▶ X → Y` are encoded as `Fn((P, X)) -> Y`
//! (tuple-as-single-argument). This matches the `architectures::*`
//! scaffold convention (`fn((f32, u8, u32)) -> u32`). The composed action
//! [`ParaMorphism::compose`] returns is itself a closure of the same shape
//! — `Fn(((Q, P), X)) -> Z` — so chains compose without ceremony.
//!
//! ## Lints
//!
//! - `clippy::many_single_char_names` is allowed inside [`ParaMorphism::compose`]:
//!   the names `p`, `q`, `f`, `g`, `h`, `x`, `y`, `z` are the standard
//!   mathematical letters from CDL §3.1; renaming them would obscure the
//!   correspondence with the paper.
//! - `clippy::type_complexity` is allowed at the module level for the
//!   `ParaMorphism<SetMonoidal, C, (Q, P), impl Fn(((Q, P), X)) -> Z>`
//!   return type of [`ParaMorphism::compose`]: every type parameter is
//!   load-bearing (the GAT-style HKT encoding has no kind machinery to
//!   abbreviate them), and a `type` alias would still need every
//!   parameter.

use core::marker::PhantomData;

use super::actegory::Actegory;
use super::monoidal_category::MonoidalCategory;

/// Type-level handle for the 2-category `Para(M, C)`.
///
/// Carries no runtime data; serves as the namespace under which `Para`
/// 1-morphisms and 2-morphisms are typed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Para<M: MonoidalCategory, C: Actegory<M>>(PhantomData<(M, C)>);

impl<M: MonoidalCategory, C: Actegory<M>> Para<M, C> {
    /// Construct a fresh `Para<M, C>` namespace handle.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

/// A 1-morphism `X → Y` in `Para(M, C)` — the pair `(P, f)`.
///
/// `P` is the parameter object (in `M`); `f` is the underlying map
/// `P ▶ X → Y` in `C`.
///
/// Underlying-map closures use the tuple-input convention `Fn((P, X)) -> Y`.
#[derive(Debug, Clone)]
pub struct ParaMorphism<M, C, P, F>
where
    M: MonoidalCategory,
    C: Actegory<M>,
{
    /// The parameter object `P ∈ M`.
    pub parameter: P,
    /// The underlying morphism `f : P ▶ X → Y` in `C`.
    pub action: F,
    _phantom: PhantomData<(M, C)>,
}

impl<M, C, P, F> ParaMorphism<M, C, P, F>
where
    M: MonoidalCategory,
    C: Actegory<M>,
{
    /// Build a `Para` 1-morphism from a parameter object and an action map.
    pub fn new(parameter: P, action: F) -> Self {
        Self {
            parameter,
            action,
            _phantom: PhantomData,
        }
    }

    /// Apply the underlying map to a `(parameter, x)` pair.
    ///
    /// Convenience for testing: invokes `f((p, x))` where `p` is a clone of
    /// `self.parameter`. The closure convention is tuple-input, so this is
    /// just `(self.action)((p, x))`.
    pub fn apply<X, Y>(&self, x: X) -> Y
    where
        P: Clone,
        F: Fn((P, X)) -> Y,
    {
        (self.action)((self.parameter.clone(), x))
    }
}

impl<C, P, F> ParaMorphism<super::monoidal_category::SetMonoidal, C, P, F>
where
    C: super::actegory::Actegory<super::monoidal_category::SetMonoidal>,
{
    /// Sequential composition `(P, f) ; (Q, g) = (Q ⊗ P, h)` in
    /// `Para(SetMonoidal, C)` for any `C: Actegory<SetMonoidal>`.
    ///
    /// CDL §3.1. Takes `self : X → Y` (parameter `P`, action
    /// `f : P × X → Y`) and `other : Y → Z` (parameter `Q`, action
    /// `g : Q × Y → Z`); returns the composite `X → Z` whose parameter is
    /// `(Q, P)` (the `SetMonoidal` tensor) and whose action `h` is the
    /// composite
    ///
    /// ```text
    /// h((q, p), x) = g((q, f((p, x))))
    /// ```
    ///
    /// Identical to threading through the explicit μ on the actegory `C`.
    /// For `C = SetActegory` this is the tuple-action μ:
    /// `μ((q, p), x) = (q, (p, x))`. v0.4.0 widens from the v0.3.x
    /// `SetActegory`-bound impl; the body is structurally agnostic.
    ///
    /// # Type parameters
    ///
    /// - `Q` — parameter type of the second morphism.
    /// - `G` — closure type of `g : Q × Y → Z`.
    /// - `X`, `Y`, `Z` — carrier types in the underlying category of `C`.
    ///
    /// # Returns
    ///
    /// A `ParaMorphism` with parameter `(Q, P)` and action of type
    /// `impl Fn(((Q, P), X)) -> Z`. Returned via `impl Trait` in struct-
    /// position is impossible, so the closure type is exposed as a fresh
    /// generic on the returned struct (Rust monomorphizes it; the caller
    /// never names it).
    #[allow(
        clippy::many_single_char_names,
        clippy::type_complexity,
        reason = "p, q, f, g, h, x, y, z are CDL §3.1 standard names; renaming obscures the math. The fully-qualified return type has every parameter load-bearing — a type alias would still need every parameter."
    )]
    pub fn compose<Q, G, X, Y, Z>(
        self,
        other: ParaMorphism<super::monoidal_category::SetMonoidal, C, Q, G>,
    ) -> ParaMorphism<super::monoidal_category::SetMonoidal, C, (Q, P), impl Fn(((Q, P), X)) -> Z>
    where
        F: Fn((P, X)) -> Y,
        G: Fn((Q, Y)) -> Z,
    {
        let ParaMorphism {
            parameter: p,
            action: f,
            ..
        } = self;
        let ParaMorphism {
            parameter: q,
            action: g,
            ..
        } = other;

        let h = move |((q_in, p_in), x): ((Q, P), X)| -> Z {
            // μ((q, p), x) = (q, (p, x))  — implicit in the destructure above.
            // Q ▶ f : (q, (p, x)) ↦ (q, f((p, x)))  — applied to second slot.
            let y = f((p_in, x));
            // g : (q, y) ↦ z.
            g((q_in, y))
        };

        ParaMorphism::new((q, p), h)
    }
}
