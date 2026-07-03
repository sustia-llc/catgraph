//! Algebras over an operad.
//!
//! F&S *Seven Sketches* §6.5 **Def 6.99.** An *algebra* for an operad `O` is
//! a functor `F : O → Set`. Concretely, `F` sends each type of `O` to a
//! carrier set `F(X)` and each `n`-ary operation `o ∈ O(X_1, …, X_n; Y)` to
//! a function `F(o) : F(X_1) × … × F(X_n) → F(Y)` such that substitution in
//! `O` corresponds to composition of functions, and identities in `O` map
//! to identity functions on carriers.
//!
//! # This implementation
//!
//! The [`OperadAlgebra`] trait captures the single-sorted case: one carrier
//! set per operad (the associated type [`OperadAlgebra::Element`]) and a
//! uniform [`evaluate`](OperadAlgebra::evaluate) method that interprets
//! each operation of `O` as a function `Elementⁿ → Element`. Multi-sorted
//! (typed) operads are a future refinement.
//!
//! The trait is parameterised over the operad type `O` and the input-label
//! type `Input` so that the same algebra notion applies to all concrete
//! operads defined in this crate ([`crate::e1_operad::E1`],
//! [`crate::e2_operad::E2`], [`crate::wiring_diagram::WiringDiagram`]).
//!
//! # Ex 6.100 worked example
//!
//! [`CircAlgebra`] implements the textbook's named example
//! `Circ : Cospan → Set` specialised to
//! [`crate::wiring_diagram::WiringDiagram`]. See the example
//! `examples/operad_algebra_circ.rs` for a substitution-preservation demo.

use std::fmt::Debug;

use catgraph::errors::CatgraphError;
use catgraph::operadic::Operadic;

use crate::wiring_diagram::WiringDiagram;

/// A single-sorted algebra `F : O → Set` for an operad `O`.
pub trait OperadAlgebra<O, Input>
where
    O: Operadic<Input>,
{
    /// Carrier set `F(X)` — one element type shared across all types of `O`.
    type Element: Clone;

    /// Interpret an operation `op` of arity `n` as a function
    /// `Elementⁿ → Element`.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError`] when the caller-supplied `inputs` do not
    /// match the operation's declared arity or when the algebra cannot
    /// evaluate the operation for a domain-specific reason.
    fn evaluate(&self, op: &O, inputs: &[Self::Element]) -> Result<Self::Element, CatgraphError>;
}

// ---- Ex 6.100: Circ : WiringDiagram → Set ----------------------------------

/// F&S *Seven Sketches* **Ex 6.100.** `Circ : Cospan → Set` specialised to
/// [`WiringDiagram`]. A minimal, faithful instance: the carrier `F(c)` is
/// the natural number of outer-circle ports of a circuit with circle-shape
/// `c`, and `evaluate(op, inputs)` returns the outer-port count of `op`
/// regardless of the input circuits. This witnesses the theorem that
/// outer-port counts are stable under operadic substitution — the inner
/// circles of `op` change when another diagram is plugged in, but the
/// outer circle is invariant.
///
/// Richer circuit carriers (e.g. resistor network decorations as in the
/// textbook's resistor-circuit running example) require a functorial
/// bridge from [`DecoratedCospan`](crate::decorated_cospan::DecoratedCospan)
/// and are deferred to a future release.
#[derive(Default, Clone, Copy, Debug)]
pub struct CircAlgebra;

impl<Lambda, InterCircle, IntraCircle>
    OperadAlgebra<WiringDiagram<Lambda, InterCircle, IntraCircle>, InterCircle> for CircAlgebra
where
    Lambda: Eq + Copy + Debug + Send + Sync,
    InterCircle: Eq + Copy + Send + Sync,
    IntraCircle: Eq + Copy + Send + Sync,
{
    type Element = usize;

    fn evaluate(
        &self,
        op: &WiringDiagram<Lambda, InterCircle, IntraCircle>,
        _inputs: &[Self::Element],
    ) -> Result<Self::Element, CatgraphError> {
        Ok(op.inner().right_names().len())
    }
}

/// Verify that an operad algebra commutes with substitution: for any
/// outer operation `outer`, input slot `slot`, and inner operation `inner`,
///
/// ```text
/// evaluate(outer[slot := inner], inputs) == evaluate(outer, inputs)
/// ```
///
/// This is the single-sorted form of the Def 6.99 functoriality axiom
/// specialised to algebras whose evaluate-function discards inputs. For
/// algebras that use their inputs non-trivially, the RHS would be
/// `evaluate(outer, inputs_with_slot_recomputed)` — out of scope here.
///
/// # Errors
///
/// Returns [`CatgraphError`] when any of the three evaluate/substitution
/// calls fail, or when the before/after outputs differ.
pub fn check_substitution_preserved<A, O, Input>(
    algebra: &A,
    outer: O,
    slot: Input,
    inner: O,
    inputs: &[A::Element],
) -> Result<(), CatgraphError>
where
    A: OperadAlgebra<O, Input>,
    A::Element: PartialEq + Debug,
    O: Operadic<Input> + Clone,
{
    let before = algebra.evaluate(&outer, inputs)?;
    let mut substituted = outer;
    substituted.operadic_substitution(slot, inner)?;
    let after = algebra.evaluate(&substituted, inputs)?;
    if before != after {
        return Err(CatgraphError::Operadic {
            message: format!(
                "OperadAlgebra: substitution not preserved (before = {before:?}, after = {after:?})",
            ),
        });
    }
    Ok(())
}
