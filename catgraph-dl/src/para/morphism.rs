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
//! Closure convention: `Fn((P, X)) -> Y`; [`ParaMorphism::compose`] returns
//! `Fn(((Q, P), X)) -> Z`.

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
    /// `(P, f) ; (Q, g) = ((Q, P), h)` with `h(((q, p), x)) = g((q, f((p, x))))`
    /// (CDL §3.1), for any `C: Actegory<SetMonoidal>`.
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
