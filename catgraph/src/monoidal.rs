//! Monoidal and symmetric monoidal category traits, plus a generic layered morphism type.
//!
//! `Monoidal` provides the tensor product. `SymmetricMonoidalMorphism` adds permutation-based
//! braiding. `GenericMonoidalMorphism` is a concrete layered representation with black-box blocks.

use std::marker::PhantomData;

use itertools::Itertools;

use crate::{
    errors::CatgraphError,
    frobenius::{Contains, InterpretableMorphism},
};

use {
    crate::category::{Composable, ComposableMutating, HasIdentity},
    std::fmt::Debug,
};

/// Tensor product: mutates `self` to become `self ⊗ other`.
pub trait Monoidal {
    /// Replaces `self` with the tensor product `self ⊗ other`.
    fn monoidal(&mut self, other: Self);
}

/// A single horizontal layer of black-box blocks in a monoidal morphism.
///
/// Each block is labelled with `BoxType`; the layer maps `left_type` to `right_type`.
#[derive(PartialEq, Eq, Clone)]
pub struct GenericMonoidalMorphismLayer<BoxType, Lambda>
where
    Lambda: Eq + Copy,
    BoxType: Eq + Clone,
{
    pub blocks: Vec<BoxType>,
    pub left_type: Vec<Lambda>,
    pub right_type: Vec<Lambda>,
}

impl<Lambda, BoxType> Contains<BoxType> for GenericMonoidalMorphismLayer<BoxType, Lambda>
where
    Lambda: Eq + Copy,
    BoxType: Eq + Clone,
{
    fn contained_labels(&self) -> Vec<BoxType> {
        self.blocks.clone()
    }
}

impl<BoxType, Lambda> GenericMonoidalMorphismLayer<BoxType, Lambda>
where
    Lambda: Eq + Copy,
    BoxType: Eq + Clone,
{
    #[must_use]
    pub fn new() -> Self {
        Self {
            blocks: vec![],
            left_type: vec![],
            right_type: vec![],
        }
    }
}

impl<BoxType, Lambda> Default for GenericMonoidalMorphismLayer<BoxType, Lambda>
where
    Lambda: Eq + Copy,
    BoxType: Eq + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<BoxType, Lambda> HasIdentity<Vec<Lambda>> for GenericMonoidalMorphismLayer<BoxType, Lambda>
where
    Lambda: Eq + Copy,
    BoxType: Eq + Clone + HasIdentity<Lambda>,
{
    fn identity(on_type: &Vec<Lambda>) -> Self {
        let mut answer = Self::new();
        for cur_type in on_type {
            let op = BoxType::identity(cur_type);
            answer.blocks.push(op);
            answer.left_type.push(*cur_type);
            answer.right_type.push(*cur_type);
        }
        answer
    }
}

impl<BoxType, Lambda> Monoidal for GenericMonoidalMorphismLayer<BoxType, Lambda>
where
    Lambda: Eq + Copy,
    BoxType: Eq + Clone,
{
    fn monoidal(&mut self, other: Self) {
        self.blocks.extend(other.blocks);
        self.left_type.extend(other.left_type);
        self.right_type.extend(other.right_type);
    }
}

/// A layered monoidal morphism built from black-box blocks.
///
/// Interpreted into a concrete category by supplying a function from `BoxType`
/// to actual morphisms, then composing layers and tensoring blocks within each layer.
#[derive(Clone, PartialEq, Eq)]
pub struct GenericMonoidalMorphism<BoxType, Lambda>
where
    Lambda: Eq + Copy,
    BoxType: Eq + Clone,
{
    layers: Vec<GenericMonoidalMorphismLayer<BoxType, Lambda>>,
}

impl<BoxType, Lambda> Default for GenericMonoidalMorphism<BoxType, Lambda>
where
    Lambda: Eq + Copy,
    BoxType: Eq + Clone,
{
    fn default() -> Self {
        Self { layers: vec![] }
    }
}

impl<Lambda, BoxType> Contains<BoxType> for GenericMonoidalMorphism<BoxType, Lambda>
where
    Lambda: Eq + Copy,
    BoxType: Eq + Clone,
{
    fn contained_labels(&self) -> Vec<BoxType> {
        #[allow(clippy::redundant_closure_for_method_calls)]
        self.layers
            .iter()
            .flat_map(|layer| layer.contained_labels())
            .collect_vec()
    }
}

impl<Lambda, BoxType> GenericMonoidalMorphism<BoxType, Lambda>
where
    Lambda: Eq + Copy,
    BoxType: Eq + Clone,
{
    #[must_use]
    pub fn new() -> Self {
        Self { layers: vec![] }
    }

    /// Number of sequential layers in this morphism.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.layers.len()
    }

    fn append_layer(
        &mut self,
        next_layer: GenericMonoidalMorphismLayer<BoxType, Lambda>,
    ) -> Result<(), CatgraphError> {
        let last_so_far = self.layers.pop();
        match last_so_far {
            None => {
                self.layers.push(next_layer);
            }
            Some(v) => {
                if v.right_type != next_layer.left_type {
                    return Err(CatgraphError::Composition {
                        message: "type mismatch in morphims composition".to_string(),
                    });
                }
                self.layers.push(v);
                self.layers.push(next_layer);
            }
        }
        Ok(())
    }

    /// Consumes the morphism, returning its layers.
    #[must_use]
    pub fn extract_layers(self) -> Vec<GenericMonoidalMorphismLayer<BoxType, Lambda>> {
        self.layers
    }
}

impl<Lambda, BoxType> HasIdentity<Vec<Lambda>> for GenericMonoidalMorphism<BoxType, Lambda>
where
    Lambda: Eq + Copy,
    BoxType: Eq + Clone + HasIdentity<Lambda>,
{
    fn identity(on_this: &Vec<Lambda>) -> Self {
        Self {
            layers: vec![<_>::identity(on_this)],
        }
    }
}

impl<Lambda, BoxType> Monoidal for GenericMonoidalMorphism<BoxType, Lambda>
where
    Lambda: Eq + Copy + Debug,
    BoxType: Eq + Clone + HasIdentity<Lambda>,
{
    #[allow(clippy::assigning_clones)]
    fn monoidal(&mut self, other: Self) {
        let self_len = self.layers.len();
        let others_len = other.layers.len();
        let mut last_other_type: Vec<Lambda> = vec![];
        let mut last_self_type: Vec<Lambda> = vec![];
        for (n, cur_self_layer) in self.layers.iter_mut().enumerate() {
            last_self_type = cur_self_layer.right_type.clone();
            if n < other.layers.len() {
                last_other_type = other.layers[n].right_type.clone();
                cur_self_layer.monoidal(other.layers[n].clone());
            } else {
                cur_self_layer.monoidal(<_>::identity(&last_other_type));
            }
        }
        for n in self_len..others_len {
            let mut new_layer = GenericMonoidalMorphismLayer::identity(&last_self_type);
            new_layer.monoidal(other.layers[n].clone());
            let _ = self.append_layer(new_layer);
        }
    }
}

fn layers_composable<Lambda, BoxType>(
    l: &[GenericMonoidalMorphismLayer<BoxType, Lambda>],
    r: &[GenericMonoidalMorphismLayer<BoxType, Lambda>],
) -> Result<(), CatgraphError>
where
    Lambda: Eq + Copy + Debug,
    BoxType: Eq + Clone,
{
    if l.is_empty() || r.is_empty() {
        if l.is_empty() && r.is_empty() {
            return Ok(());
        }
        let interface = if l.is_empty() {
            &r[0].left_type
        } else {
            &l.last().unwrap().right_type
        };
        return if interface.is_empty() {
            Ok(())
        } else {
            Err(CatgraphError::CompositionSizeMismatch {
                expected: 0,
                actual: interface.len(),
            })
        };
    }
    let self_interface = &l.last().unwrap().right_type;
    let other_interface = &r[0].left_type;
    if self_interface.len() != other_interface.len() {
        Err(CatgraphError::CompositionSizeMismatch {
            expected: self_interface.len(),
            actual: other_interface.len(),
        })
    } else if self_interface != other_interface {
        for idx in 0..self_interface.len() {
            let w1 = self_interface[idx];
            let w2 = other_interface[idx];
            if w1 != w2 {
                return Err(CatgraphError::CompositionLabelMismatch {
                    index: idx,
                    expected: format!("{w1:?}"),
                    actual: format!("{w2:?}"),
                });
            }
        }
        Err(CatgraphError::Composition {
            message: "Mismatch in labels of common interface at some unknown index.".to_string(),
        })
    } else {
        Ok(())
    }
}

impl<Lambda, BoxType> ComposableMutating<Vec<Lambda>> for GenericMonoidalMorphism<BoxType, Lambda>
where
    Lambda: Eq + Copy + Debug,
    BoxType: Eq + Clone,
{
    fn composable(&self, other: &Self) -> Result<(), CatgraphError> {
        layers_composable(&self.layers, &other.layers)
    }

    fn compose(&mut self, other: Self) -> Result<(), CatgraphError> {
        for next_layer in other.layers {
            self.append_layer(next_layer)?;
        }
        Ok(())
    }

    fn domain(&self) -> Vec<Lambda> {
        self.layers
            .first()
            .map(|x| x.left_type.clone())
            .unwrap_or_default()
    }

    fn codomain(&self) -> Vec<Lambda> {
        self.layers
            .last()
            .map(|x| x.right_type.clone())
            .unwrap_or_default()
    }
}

/// A composable morphism that also supports tensor product (immutable composition).
#[allow(clippy::module_name_repetitions)]
pub trait MonoidalMorphism<T: Eq>: Monoidal + Composable<T> {}
/// A composable morphism that also supports tensor product (in-place composition).
#[allow(clippy::module_name_repetitions)]
pub trait MonoidalMutatingMorphism<T: Eq>: Monoidal + ComposableMutating<T> {}

// Blanket: GenericMonoidalMorphism satisfies MonoidalMutatingMorphism by
// concatenating blocks (monoidal) and appending layers (compose).
impl<Lambda, BoxType> MonoidalMutatingMorphism<Vec<Lambda>>
    for GenericMonoidalMorphism<BoxType, Lambda>
where
    Lambda: Eq + Copy + Debug,
    BoxType: Eq + HasIdentity<Lambda> + Clone,
{
}

#[allow(dead_code)] // constructed via From::from() in InterpretableMorphism impls
struct InterpretableNoMut<T, Lambda>
where
    Lambda: Eq,
    T: Monoidal + Composable<Vec<Lambda>> + HasIdentity<Vec<Lambda>>,
{
    me: T,
    dummy: PhantomData<Lambda>,
}

impl<T, Lambda> From<T> for InterpretableNoMut<T, Lambda>
where
    Lambda: Eq,
    T: Monoidal + Composable<Vec<Lambda>> + HasIdentity<Vec<Lambda>>,
{
    fn from(me: T) -> Self {
        Self {
            me,
            dummy: PhantomData,
        }
    }
}

#[allow(dead_code)] // constructed via From::from() in InterpretableMorphism impls
struct InterpretableMut<T, Lambda>
where
    Lambda: Eq,
    T: Monoidal + ComposableMutating<Vec<Lambda>> + HasIdentity<Vec<Lambda>>,
{
    me: T,
    dummy: PhantomData<Lambda>,
}

impl<T, Lambda> From<T> for InterpretableMut<T, Lambda>
where
    Lambda: Eq,
    T: Monoidal + ComposableMutating<Vec<Lambda>> + HasIdentity<Vec<Lambda>>,
{
    fn from(me: T) -> Self {
        Self {
            me,
            dummy: PhantomData,
        }
    }
}

// Interpret a GenericMonoidalMorphism into a concrete monoidal morphism type
// by tensoring blocks within each layer, then composing layers sequentially.
impl<Lambda, BoxType, T>
    InterpretableMorphism<GenericMonoidalMorphism<BoxType, Lambda>, Lambda, BoxType>
    for InterpretableMut<T, Lambda>
where
    Lambda: Eq + Copy + Debug,
    BoxType: Eq + Clone,
    T: Monoidal + ComposableMutating<Vec<Lambda>> + HasIdentity<Vec<Lambda>>,
{
    fn interpret<F>(
        morphism: &GenericMonoidalMorphism<BoxType, Lambda>,
        black_box_interpreter: F,
    ) -> Result<Self, CatgraphError>
    where
        F: Fn(&BoxType, &[Lambda], &[Lambda]) -> Result<Self, CatgraphError>,
    {
        let mut answer = T::identity(&morphism.domain());
        for layer in &morphism.layers {
            let Some(first) = &layer.blocks.first() else {
                return Err(CatgraphError::Interpret {
                    context: "somehow an empty layer in a generica monoidal morphism???"
                        .to_string(),
                });
            };
            let mut cur_layer = black_box_interpreter(first, &[], &[]).map(|z| z.me)?;
            for block in &layer.blocks[1..] {
                cur_layer.monoidal(black_box_interpreter(block, &[], &[]).map(|z| z.me)?);
            }
            answer.compose(cur_layer)?;
        }
        Ok(Self::from(answer))
    }
}

// Same interpretation as InterpretableMut, but using immutable Composable::compose.
impl<Lambda, BoxType, T>
    InterpretableMorphism<GenericMonoidalMorphism<BoxType, Lambda>, Lambda, BoxType>
    for InterpretableNoMut<T, Lambda>
where
    Lambda: Eq + Copy + Debug,
    BoxType: Eq + Clone,
    T: Monoidal + Composable<Vec<Lambda>> + HasIdentity<Vec<Lambda>>,
{
    fn interpret<F>(
        morphism: &GenericMonoidalMorphism<BoxType, Lambda>,
        black_box_interpreter: F,
    ) -> Result<Self, CatgraphError>
    where
        F: Fn(&BoxType, &[Lambda], &[Lambda]) -> Result<Self, CatgraphError>,
    {
        let mut answer = T::identity(&morphism.domain());
        for layer in &morphism.layers {
            let Some(first) = &layer.blocks.first() else {
                return Err(CatgraphError::Interpret {
                    context: "somehow an empty layer in a generica monoidal morphism???"
                        .to_string(),
                });
            };
            let mut cur_layer = black_box_interpreter(first, &[], &[]).map(|z| z.me)?;
            for block in &layer.blocks[1..] {
                cur_layer.monoidal(black_box_interpreter(block, &[], &[]).map(|z| z.me)?);
            }
            answer = answer.compose(&cur_layer)?;
        }
        Ok(Self::from(answer))
    }
}

// ── Traits from symmetric_monoidal ──

use permutations::Permutation;

/// Symmetric monoidal morphism: supports pre/post-composition with permutations.
///
/// `types` is the typed tensor factors; `types_as_on_domain` controls which side is permuted.
#[allow(clippy::module_name_repetitions)]
pub trait SymmetricMonoidalMorphism<T: Eq> {
    /// Pre- or post-composes this morphism with permutation `p`.
    fn permute_side(&mut self, p: &Permutation, of_codomain: bool);
    /// Constructs a morphism that applies permutation `p` to typed tensor factors.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError`] if the permutation size does not match the `types` length.
    fn from_permutation(
        p: Permutation,
        types: &[T],
        types_as_on_domain: bool,
    ) -> Result<Self, CatgraphError>
    where
        Self: Sized;
}

/// Like [`SymmetricMonoidalMorphism`] but for the discrete category (`FinSet`),
/// where objects are `usize` cardinalities rather than typed tensor factor slices.
#[allow(clippy::module_name_repetitions)]
pub trait SymmetricMonoidalDiscreteMorphism<T: Eq> {
    /// Pre- or post-composes this morphism with permutation `p`.
    fn permute_side(&mut self, p: &Permutation, of_codomain: bool);
    /// Constructs a morphism from a permutation on a set of the given size.
    fn from_permutation(p: Permutation, types: T, types_as_on_domain: bool) -> Self;
}

#[cfg(test)]
mod test {
    use super::*;

    // Simple test type for boxes
    #[derive(Clone, PartialEq, Eq, Debug)]
    struct SimpleBox {
        input: Vec<char>,
        output: Vec<char>,
    }

    impl HasIdentity<char> for SimpleBox {
        fn identity(on_this: &char) -> Self {
            SimpleBox {
                input: vec![*on_this],
                output: vec![*on_this],
            }
        }
    }

    #[test]
    fn layer_new() {
        let layer: GenericMonoidalMorphismLayer<SimpleBox, char> =
            GenericMonoidalMorphismLayer::new();
        assert!(layer.blocks.is_empty());
        assert!(layer.left_type.is_empty());
        assert!(layer.right_type.is_empty());
    }

    #[test]
    fn layer_identity() {
        let types = vec!['a', 'b', 'c'];
        let layer: GenericMonoidalMorphismLayer<SimpleBox, char> =
            GenericMonoidalMorphismLayer::identity(&types);

        assert_eq!(layer.blocks.len(), 3);
        assert_eq!(layer.left_type, types);
        assert_eq!(layer.right_type, types);
    }

    #[test]
    fn layer_monoidal() {
        let layer1: GenericMonoidalMorphismLayer<SimpleBox, char> =
            GenericMonoidalMorphismLayer::identity(&vec!['a']);
        let layer2: GenericMonoidalMorphismLayer<SimpleBox, char> =
            GenericMonoidalMorphismLayer::identity(&vec!['b']);

        let mut combined = layer1;
        combined.monoidal(layer2);

        assert_eq!(combined.blocks.len(), 2);
        assert_eq!(combined.left_type, vec!['a', 'b']);
        assert_eq!(combined.right_type, vec!['a', 'b']);
    }

    #[test]
    fn layer_contained_labels() {
        let types = vec!['a', 'b'];
        let layer: GenericMonoidalMorphismLayer<SimpleBox, char> =
            GenericMonoidalMorphismLayer::identity(&types);

        let labels = layer.contained_labels();
        assert_eq!(labels.len(), 2);
    }

    #[test]
    fn morphism_new() {
        let morphism: GenericMonoidalMorphism<SimpleBox, char> = GenericMonoidalMorphism::new();
        assert_eq!(morphism.depth(), 0);
    }

    #[test]
    fn morphism_identity() {
        let types = vec!['a', 'b'];
        let morphism: GenericMonoidalMorphism<SimpleBox, char> =
            GenericMonoidalMorphism::identity(&types);

        assert_eq!(morphism.depth(), 1);
        assert_eq!(morphism.domain(), types);
        assert_eq!(morphism.codomain(), types);
    }

    #[test]
    fn morphism_monoidal() {
        let m1: GenericMonoidalMorphism<SimpleBox, char> =
            GenericMonoidalMorphism::identity(&vec!['a']);
        let m2: GenericMonoidalMorphism<SimpleBox, char> =
            GenericMonoidalMorphism::identity(&vec!['b']);

        let mut combined = m1;
        combined.monoidal(m2);

        assert_eq!(combined.domain(), vec!['a', 'b']);
        assert_eq!(combined.codomain(), vec!['a', 'b']);
    }

    #[test]
    fn morphism_compose() {
        let types = vec!['a', 'b'];
        let m1: GenericMonoidalMorphism<SimpleBox, char> =
            GenericMonoidalMorphism::identity(&types);
        let m2: GenericMonoidalMorphism<SimpleBox, char> =
            GenericMonoidalMorphism::identity(&types);

        let mut composed = m1;
        let result = composed.compose(m2);
        assert!(result.is_ok());
        assert_eq!(composed.depth(), 2);
    }

    #[test]
    fn morphism_compose_mismatch() {
        let m1: GenericMonoidalMorphism<SimpleBox, char> =
            GenericMonoidalMorphism::identity(&vec!['a']);
        let m2: GenericMonoidalMorphism<SimpleBox, char> =
            GenericMonoidalMorphism::identity(&vec!['b']);

        let mut composed = m1;
        let result = composed.compose(m2);
        assert!(result.is_err());
    }

    #[test]
    fn morphism_composable() {
        let types = vec!['a'];
        let m1: GenericMonoidalMorphism<SimpleBox, char> =
            GenericMonoidalMorphism::identity(&types);
        let m2: GenericMonoidalMorphism<SimpleBox, char> =
            GenericMonoidalMorphism::identity(&types);

        assert!(m1.composable(&m2).is_ok());

        let m3: GenericMonoidalMorphism<SimpleBox, char> =
            GenericMonoidalMorphism::identity(&vec!['b']);
        assert!(m1.composable(&m3).is_err());
    }

    #[test]
    fn morphism_extract_layers() {
        let types = vec!['a', 'b'];
        let morphism: GenericMonoidalMorphism<SimpleBox, char> =
            GenericMonoidalMorphism::identity(&types);

        let layers = morphism.extract_layers();
        assert_eq!(layers.len(), 1);
    }

    #[test]
    fn morphism_contained_labels() {
        let types = vec!['a', 'b'];
        let morphism: GenericMonoidalMorphism<SimpleBox, char> =
            GenericMonoidalMorphism::identity(&types);

        let labels = morphism.contained_labels();
        assert_eq!(labels.len(), 2);
    }

    #[test]
    fn morphism_append_layer() {
        let types = vec!['a'];
        let mut morphism: GenericMonoidalMorphism<SimpleBox, char> =
            GenericMonoidalMorphism::identity(&types);

        let layer: GenericMonoidalMorphismLayer<SimpleBox, char> =
            GenericMonoidalMorphismLayer::identity(&types);

        let result = morphism.append_layer(layer);
        assert!(result.is_ok());
        assert_eq!(morphism.depth(), 2);
    }

    #[test]
    fn morphism_append_layer_empty() {
        let types = vec!['a'];
        let mut morphism: GenericMonoidalMorphism<SimpleBox, char> = GenericMonoidalMorphism::new();

        let layer: GenericMonoidalMorphismLayer<SimpleBox, char> =
            GenericMonoidalMorphismLayer::identity(&types);

        let result = morphism.append_layer(layer);
        assert!(result.is_ok());
        assert_eq!(morphism.depth(), 1);
    }

    #[test]
    fn layers_composable_both_empty() {
        let l: Vec<GenericMonoidalMorphismLayer<SimpleBox, char>> = vec![];
        let r: Vec<GenericMonoidalMorphismLayer<SimpleBox, char>> = vec![];
        assert!(layers_composable(&l, &r).is_ok());
    }

    #[test]
    fn layers_composable_one_empty() {
        let types = vec!['a'];
        let layer: GenericMonoidalMorphismLayer<SimpleBox, char> =
            GenericMonoidalMorphismLayer::identity(&types);
        let l = vec![layer];
        let r: Vec<GenericMonoidalMorphismLayer<SimpleBox, char>> = vec![];

        // Non-empty interface with empty other side should fail
        assert!(layers_composable(&l, &r).is_err());
    }

    #[test]
    fn monoidal_different_depths() {
        // Test monoidal with morphisms of different depths
        let mut m1: GenericMonoidalMorphism<SimpleBox, char> =
            GenericMonoidalMorphism::identity(&vec!['a']);
        let mut m2: GenericMonoidalMorphism<SimpleBox, char> =
            GenericMonoidalMorphism::identity(&vec!['b']);

        // Make m2 deeper by composing with itself
        let m2_copy: GenericMonoidalMorphism<SimpleBox, char> =
            GenericMonoidalMorphism::identity(&vec!['b']);
        let _ = m2.compose(m2_copy);

        m1.monoidal(m2);
        assert_eq!(m1.domain(), vec!['a', 'b']);
    }
}
