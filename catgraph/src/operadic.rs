//! Operadic substitution trait for plugging one n-ary operation into a slot of another.
//!
//! The compositional viewpoint is F&S 2019 Eq (6); the operadic framing itself is
//! Spivak 2013, *The Operad of Wiring Diagrams* (arXiv:1305.0297), as credited in
//! the F&S introduction. Concrete operad implementations live in `catgraph-applied`.

use crate::errors::CatgraphError;

/// An operad element supporting substitution of one operation into an input slot.
#[allow(clippy::module_name_repetitions)]
pub trait Operadic<InputLabel> {
    /// Substitute `other_obj` into the input slot identified by `which_input`.
    ///
    /// Fails if `which_input` does not match any input of `self`, or if the
    /// output type of `other_obj` is incompatible with the designated slot.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError`] if `which_input` is not found or the boundary is incompatible.
    fn operadic_substitution(
        &mut self,
        which_input: InputLabel,
        other_obj: Self,
    ) -> Result<(), CatgraphError>;
}
