//! Operadic substitution trait for plugging one n-ary operation into a slot of another.

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
