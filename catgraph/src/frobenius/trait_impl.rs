use std::fmt::Debug;

use crate::errors::CatgraphError;

use {
    crate::{
        category::{ComposableMutating, HasIdentity},
        monoidal::{MonoidalMutatingMorphism, SymmetricMonoidalMorphism},
    },
    permutations::Permutation,
};

use super::{
    morphism_system::InterpretableMorphism,
    operations::{FrobeniusMorphism, FrobeniusOperation, special_frobenius_morphism},
};

/// Trait for morphisms in a symmetric monoidal category where each basic object is a Frobenius algebra.
///
/// Implementors provide interpretations of the four Frobenius generators (unit, counit,
/// multiplication, comultiplication); braiding and identity come from `SymmetricMonoidalMorphism`.
pub trait Frobenius<
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
>:
    SymmetricMonoidalMorphism<Lambda> + HasIdentity<Vec<Lambda>> + MonoidalMutatingMorphism<Vec<Lambda>>
{
    /// Interpret the unit η: \[\] → \[z\].
    fn interpret_unit(z: Lambda) -> Self;
    /// Interpret the counit ε: \[z\] → \[\].
    fn interpret_counit(z: Lambda) -> Self;
    /// Interpret the multiplication μ: \[z, z\] → \[z\].
    fn interpret_multiplication(z: Lambda) -> Self;
    /// Interpret the comultiplication δ: \[z\] → \[z, z\].
    fn interpret_comultiplication(z: Lambda) -> Self;

    /// Interpret a single `FrobeniusOperation` as `Self`, delegating black boxes to the closure.
    ///
    /// # Errors
    ///
    /// - Black box interpretation fails or operation is invalid.
    fn basic_interpret<F>(
        single_step: &FrobeniusOperation<Lambda, BlackBoxLabel>,
        black_box_interpreter: &F,
    ) -> Result<Self, CatgraphError>
    where
        F: Fn(&BlackBoxLabel, &[Lambda], &[Lambda]) -> Result<Self, CatgraphError>,
    {
        Ok(match single_step {
            FrobeniusOperation::Unit(z) => Self::interpret_unit(*z),
            FrobeniusOperation::Counit(z) => Self::interpret_counit(*z),
            FrobeniusOperation::Multiplication(z) => Self::interpret_multiplication(*z),
            FrobeniusOperation::Comultiplication(z) => Self::interpret_comultiplication(*z),
            FrobeniusOperation::Identity(z) => Self::identity(&vec![*z]),
            FrobeniusOperation::SymmetricBraiding(z1, z2) => {
                let transposition = Permutation::try_from(vec![0, 1]).unwrap();
                Self::from_permutation(transposition, &[*z1, *z2], true)?
            }
            FrobeniusOperation::UnSpecifiedBox(bbl, z1, z2) => black_box_interpreter(bbl, z1, z2)?,
            FrobeniusOperation::Spider(z, d1, d2) => {
                let broken_down = special_frobenius_morphism(*d1, *d2, *z);
                Self::interpret_frob(&broken_down, black_box_interpreter)?
            }
        })
    }

    /// Interpret a full `FrobeniusMorphism` by composing layer-by-layer, each layer built
    /// from monoidal products of `basic_interpret` calls.
    ///
    /// # Errors
    ///
    /// - Any layer's interpretation fails.
    fn interpret_frob<F>(
        morphism: &FrobeniusMorphism<Lambda, BlackBoxLabel>,
        black_box_interpreter: &F,
    ) -> Result<Self, CatgraphError>
    where
        F: Fn(&BlackBoxLabel, &[Lambda], &[Lambda]) -> Result<Self, CatgraphError>,
    {
        let mut answer = Self::identity(&morphism.domain());
        for layer in &morphism.layers {
            if layer.blocks.is_empty() {
                return Err(CatgraphError::Interpret {
                    context: "somehow an empty layer in a frobenius morphism???".to_string(),
                });
            }
            let first = &layer.blocks[0];
            let mut cur_layer = Self::basic_interpret(&first.op, black_box_interpreter)?;
            for block in &layer.blocks[1..] {
                cur_layer.monoidal(Self::basic_interpret(&block.op, black_box_interpreter)?);
            }
            answer.compose(cur_layer)?;
        }
        Ok(answer)
    }
}

/// Canonical self-interpretation: each generator becomes a single-layer morphism.
impl<Lambda, BlackBoxLabel> Frobenius<Lambda, BlackBoxLabel>
    for FrobeniusMorphism<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
{
    fn interpret_unit(z: Lambda) -> Self {
        FrobeniusOperation::Unit(z).into()
    }
    fn interpret_counit(z: Lambda) -> Self {
        FrobeniusOperation::Counit(z).into()
    }
    fn interpret_multiplication(z: Lambda) -> Self {
        FrobeniusOperation::Multiplication(z).into()
    }
    fn interpret_comultiplication(z: Lambda) -> Self {
        FrobeniusOperation::Comultiplication(z).into()
    }

    /// Identity interpretation: wraps the operation as-is, ignoring the black box interpreter.
    fn basic_interpret<F>(
        single_step: &FrobeniusOperation<Lambda, BlackBoxLabel>,
        _black_box_interpreter: &F,
    ) -> Result<Self, CatgraphError>
    where
        F: Fn(&BlackBoxLabel, &[Lambda], &[Lambda]) -> Result<Self, CatgraphError>,
    {
        Ok(single_step.clone().into())
    }

    /// Identity interpretation: clones the morphism as-is, ignoring the black box interpreter.
    fn interpret_frob<F>(
        morphism: &FrobeniusMorphism<Lambda, BlackBoxLabel>,
        _black_box_interpreter: &F,
    ) -> Result<Self, CatgraphError>
    where
        F: Fn(&BlackBoxLabel, &[Lambda], &[Lambda]) -> Result<Self, CatgraphError>,
    {
        Ok(morphism.clone())
    }
}

/// Blanket impl: any `Frobenius` implementor can interpret a `FrobeniusMorphism` description.
impl<Lambda, BlackBoxLabel, T>
    InterpretableMorphism<FrobeniusMorphism<Lambda, BlackBoxLabel>, Lambda, BlackBoxLabel> for T
where
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
    T: Frobenius<Lambda, BlackBoxLabel>,
{
    fn interpret<F>(
        gens: &FrobeniusMorphism<Lambda, BlackBoxLabel>,
        black_box_interpreter: F,
    ) -> Result<Self, CatgraphError>
    where
        F: Fn(&BlackBoxLabel, &[Lambda], &[Lambda]) -> Result<Self, CatgraphError>,
    {
        Self::interpret_frob(gens, &black_box_interpreter)
    }
}
