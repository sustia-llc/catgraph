//! Algebras over an operad.
//!
//! F&S *Seven Sketches* §6.5 **Def 6.99.** An *algebra* for an operad `O` is a
//! functor `F : O → Set`: it sends each type of `O` to a carrier set `F(X)` and
//! each `n`-ary operation `o ∈ O(X_1, …, X_n; Y)` to a function
//! `F(o) : F(X_1) × … × F(X_n) → F(Y)`, so that substitution in `O` becomes
//! composition of functions and identities in `O` become identity functions.
//!
//! [`OperadAlgebra`] covers the single-sorted case — one carrier set per operad
//! ([`OperadAlgebra::Element`]) and one
//! [`evaluate`](OperadAlgebra::evaluate) interpreting each operation as
//! `Elementⁿ → Element`. It is parameterised over the operad type `O` and the
//! input-label type `Input`, so it applies to [`crate::e1_operad::E1`],
//! [`crate::e2_operad::E2`], and [`crate::wiring_diagram::WiringDiagram`] alike.
//! [`CircAlgebra`] is the Ex 6.100 example `Circ : Cospan → Set` specialised to
//! [`crate::wiring_diagram::WiringDiagram`]; see
//! `examples/operad_algebra_circ.rs`.

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
    /// [`CatgraphError`] if `inputs` do not match the operation's declared
    /// arity, or on a domain-specific evaluation failure.
    fn evaluate(&self, op: &O, inputs: &[Self::Element]) -> Result<Self::Element, CatgraphError>;
}

// ---- Ex 6.100: Circ : WiringDiagram → Set ----------------------------------

/// F&S *Seven Sketches* **Ex 6.100.** `Circ : Cospan → Set` specialised to
/// [`WiringDiagram`]: the carrier `F(c)` is the number of outer-circle ports of
/// a circuit with circle-shape `c`, and `evaluate(op, inputs)` returns the
/// outer-port count of `op` for any inputs. Outer-port counts are stable under
/// operadic substitution — plugging a diagram in changes `op`'s inner circles,
/// not its outer one.
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
/// This is the Def 6.99 functoriality axiom in the single-sorted case, and it
/// holds as stated only for algebras whose evaluate-function discards its
/// inputs.
///
/// # Errors
///
/// [`CatgraphError`] if any of the three evaluate/substitution calls fail, or
/// if the before/after outputs differ.
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
