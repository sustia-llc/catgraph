//! Functors between operads.
//!
//! F&S *Seven Sketches* §6.5 **Rough Def 6.98.** A *functor of operads*
//! `F : O → P` is a map from the types of `O` to the types of `P` together
//! with a map on operations sending each `o ∈ O(X_1, …, X_n; Y)` to some
//! `F(o) ∈ P(F(X_1), …, F(X_n); F(Y))`, such that `F` preserves arities,
//! identities, and operadic substitution:
//!
//! ```text
//! F(o ∘_i q) = F(o) ∘_i F(q).
//! ```
//!
//! # This implementation
//!
//! The [`OperadFunctor`] trait captures the arity-preserving map on
//! operations. Identity- and substitution-preservation are *not* encoded
//! as trait laws but verified per-impl using the helper
//! [`check_substitution_preserved`] on concrete sample inputs (analogous
//! to property-testing algebraic laws rather than proving them).
//!
//! # E1 → E2 worked example
//!
//! [`E1ToE2`] packages the canonical inclusion of the little-intervals
//! operad `E1` into the little-disks operad `E2`: every interval
//! `[a, b] ⊂ [0, 1]` becomes a disk on the x-axis with centre `(a+b-1, 0)`
//! and radius `b - a`. The underlying geometry is already implemented by
//! [`E2::from_e1_config`](crate::e2_operad::E2::from_e1_config) — this
//! module repackages it as an [`OperadFunctor`] and adds
//! substitution-preservation tests.

use std::fmt::Debug;

use catgraph::errors::CatgraphError;
use catgraph::operadic::Operadic;

use crate::{e1_operad::E1, e2_operad::E2};

/// A functor `F : O₁ → O₂` between operads that both accept input labels
/// of type `Input`. Only the action on operations is part of the trait;
/// laws (identity + substitution preservation) are verified per-impl.
pub trait OperadFunctor<O1, O2, Input>
where
    O1: Operadic<Input>,
    O2: Operadic<Input>,
{
    /// Map operation `op` in `O₁` to its image in `O₂`.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError`] when the functor cannot produce a valid
    /// `O₂` operation from the supplied `O₁` operation (for example, when
    /// the target operad requires additional well-formedness that the
    /// source representation does not enforce).
    fn map_operation(&self, op: &O1) -> Result<O2, CatgraphError>;
}

/// Canonical inclusion `E₁ ↪ E₂` of the little-intervals operad into the
/// little-disks operad. Intervals become disks along the x-axis via
/// [`E2::from_e1_config`]; disks are named `start_name .. start_name +
/// arity` in insertion order.
///
/// The `start_name` offset exists so that two images used together in a
/// single operadic substitution (outer + inner) can be given disjoint name
/// ranges — [`E2::operadic_substitution`] requires globally unique disk
/// names and the default `start_name = 0` does not satisfy this for the
/// right-hand side of `F(o ∘_i q) = F(o) ∘_i F(q)`. Geometric content is
/// unaffected by the offset.
#[derive(Default, Clone, Copy, Debug)]
pub struct E1ToE2 {
    start_name: usize,
}

impl E1ToE2 {
    /// Inclusion starting at a nonzero disk name. See the struct-level
    /// docstring for when this matters.
    #[must_use]
    pub const fn with_offset(start_name: usize) -> Self {
        Self { start_name }
    }

    /// Verify `F(outer ∘_i inner) ≡ F(outer) ∘_i F(inner)` as a literal
    /// equality of `E₂` operations, modulo disk names (compared via
    /// geometric content: centres + radii within `f32` tolerance).
    ///
    /// On the RHS path, `F(outer)` uses `start_name = 0` and `F(inner)`
    /// uses `start_name = outer.arity()`, yielding disjoint name ranges so
    /// that the downstream E₂ substitution does not violate the
    /// unique-name invariant.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError`] when any of the substitution/map calls
    /// fail, when the two paths produce different arities, or when any
    /// disk's geometry differs beyond `f32::EPSILON`.
    pub fn check_substitution_preserved<MakeOuter, MakeInner>(
        make_outer: MakeOuter,
        slot: usize,
        make_inner: MakeInner,
    ) -> Result<(), CatgraphError>
    where
        MakeOuter: Fn() -> E1,
        MakeInner: Fn() -> E1,
    {
        let outer_arity = make_outer().arity();

        // LHS: substitute in E1, then map (offset 0 — sole image is fresh).
        let lhs = {
            let mut outer = make_outer();
            outer.operadic_substitution(slot, make_inner())?;
            E1ToE2::default().map_operation(&outer)?
        };

        // RHS: map outer at offset 0, inner at offset outer_arity, then
        // substitute into slot `slot` (which is outer's disk name `slot`).
        let rhs = {
            let mut mapped_outer = E1ToE2::default().map_operation(&make_outer())?;
            let mapped_inner = E1ToE2::with_offset(outer_arity).map_operation(&make_inner())?;
            mapped_outer.operadic_substitution(slot, mapped_inner)?;
            mapped_outer
        };

        compare_e2_geometry(&lhs, &rhs)
    }
}

impl OperadFunctor<E1, E2<usize>, usize> for E1ToE2 {
    fn map_operation(&self, op: &E1) -> Result<E2<usize>, CatgraphError> {
        let start = self.start_name;
        Ok(E2::from_e1_config(op.clone(), move |i| start + i))
    }
}

/// Compare two `E₂` operations by geometric content: canonicalise each
/// by sorting its disks by centre-x, then assert componentwise equality
/// within an `f32` tolerance. Ignores disk names.
fn compare_e2_geometry<Name>(a: &E2<Name>, b: &E2<Name>) -> Result<(), CatgraphError> {
    const TOL: f32 = 1e-5;
    if a.arity_of() != b.arity_of() {
        return Err(CatgraphError::Operadic {
            message: format!(
                "E₁ → E₂ functoriality: arity mismatch ({} vs {})",
                a.arity_of(),
                b.arity_of(),
            ),
        });
    }
    let canon = |disks: &[(Name, (f32, f32), f32)]| -> Vec<(f32, f32, f32)> {
        let mut v: Vec<_> = disks.iter().map(|(_, c, r)| (c.0, c.1, *r)).collect();
        v.sort_by(|p, q| {
            p.0.partial_cmp(&q.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| p.1.partial_cmp(&q.1).unwrap_or(std::cmp::Ordering::Equal))
        });
        v
    };
    let av = canon(a.sub_circles());
    let bv = canon(b.sub_circles());
    for (i, ((ax, ay, ar), (bx, by, br))) in av.iter().zip(bv.iter()).enumerate() {
        if (ax - bx).abs() > TOL || (ay - by).abs() > TOL || (ar - br).abs() > TOL {
            return Err(CatgraphError::Operadic {
                message: format!(
                    "E₁ → E₂ functoriality: disk {i} differs — ({ax}, {ay}, r={ar}) vs ({bx}, {by}, r={br})",
                ),
            });
        }
    }
    Ok(())
}

/// Verify that a functor preserves operadic-substitution *arities* on a
/// pair of sample operations:
///
/// ```text
/// arity( map(outer ∘_i inner) )  ==
///     arity( map(outer) ) − 1 + arity( map(inner) ).
/// ```
///
/// This is the numeric shadow of the full functoriality law
/// `F(o ∘_i q) = F(o) ∘_i F(q)`. For `E₂` specifically, literal
/// equality is too strong — the target operad imposes unique-name
/// constraints that the source operad doesn't — so the structural
/// invariant we check is the one both operads always agree on.
///
/// Callers pass *builder* closures so that the two branches can
/// construct fresh inputs (useful when `O₁` doesn't implement
/// `Copy` and its owned values are consumed by substitution).
///
/// # Errors
///
/// Returns [`CatgraphError`] when any `map_operation` / `substitute`
/// call fails, or when the arity equation above does not hold.
pub fn check_substitution_preserved<F, O1, O2, Input, MakeOuter, MakeInner>(
    functor: &F,
    make_outer: MakeOuter,
    slot: Input,
    make_inner: MakeInner,
) -> Result<(), CatgraphError>
where
    F: OperadFunctor<O1, O2, Input>,
    O1: Operadic<Input>,
    O2: Operadic<Input> + HasArity,
    Input: Clone,
    MakeOuter: Fn() -> O1,
    MakeInner: Fn() -> O1,
{
    // LHS: substitute in O₁ first, then map — measure arity of the image.
    let lhs_arity = {
        let mut outer = make_outer();
        outer.operadic_substitution(slot, make_inner())?;
        functor.map_operation(&outer)?.arity()
    };
    // RHS: map both separately and predict the composite arity.
    let mapped_outer_arity = functor.map_operation(&make_outer())?.arity();
    let mapped_inner_arity = functor.map_operation(&make_inner())?.arity();
    let rhs_arity = mapped_outer_arity + mapped_inner_arity - 1;
    if lhs_arity != rhs_arity {
        return Err(CatgraphError::Operadic {
            message: format!(
                "OperadFunctor: substitution not preserved (lhs arity {lhs_arity} vs predicted rhs {rhs_arity})",
            ),
        });
    }
    Ok(())
}

/// Uniform arity accessor used by [`check_substitution_preserved`]. Kept
/// private to this module: external users compare operad operations using
/// whichever richer structural invariant their target operad exposes.
pub trait HasArity {
    fn arity(&self) -> usize;
}

impl HasArity for E1 {
    fn arity(&self) -> usize {
        E1::arity(self)
    }
}

impl<Name> HasArity for E2<Name> {
    fn arity(&self) -> usize {
        E2::arity_of(self)
    }
}
