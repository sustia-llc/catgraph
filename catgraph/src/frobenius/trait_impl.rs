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
    /// The `Cospan`-valued twin of this default lives in
    /// [`cospan_algebra::frobenius_to_cospan`](crate::cospan_algebra::frobenius_to_cospan)
    /// (its private `generator_to_cospan` helper); the two must agree
    /// generator-for-generator. That crate-internal copy exists because `Cospan`
    /// is [`Composable`](crate::category::Composable), not the
    /// [`ComposableMutating`](crate::category::ComposableMutating) this trait's
    /// supertraits require, so it cannot implement `Frobenius` and reuse this
    /// body.
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
                // σ: [z1, z2] → [z2, z1]. The permutation is the *transposition*
                // `[1, 0]`; `[0, 1]` is the identity and would make this default
                // silently interpret every braiding as `id` (this line read
                // `[0, 1]` until #284's doc-truth pass — the bug was invisible
                // because `FrobeniusMorphism`, the only implementor, overrides
                // `basic_interpret`).
                let transposition = Permutation::try_from(vec![1, 0])
                    .expect("invariant: [1, 0] is a permutation of 0..2");
                Self::from_permutation_on_domain(transposition, &[*z1, *z2])?
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
    /// The `Cospan`-valued twin of this default is
    /// [`cospan_algebra::frobenius_to_cospan`](crate::cospan_algebra::frobenius_to_cospan),
    /// which has the same shape (identity-on-domain seed, per-layer monoidal
    /// fold, compose in order); see [`basic_interpret`](Self::basic_interpret)
    /// for why it is a separate function rather than a call into this one.
    ///
    /// ⚠ **They differ on a block-free layer**, contrary to what this
    /// cross-reference used to claim. This default `continue`s past one;
    /// `frobenius_to_cospan` builds `Cospan::identity(&Vec::new())` and composes
    /// it. On a malformed morphism whose block-free *empty-interface* layer
    /// follows a layer with a non-empty `right_type`, `frobenius_to_cospan`
    /// returns `CatgraphError::Composition` while this default silently skips
    /// the layer and returns a morphism whose codomain disagrees with
    /// `morphism.codomain()`. Unreachable through the public constructors today
    /// (`layers` is `pub(crate)` and `rebuild_from_ops` recomputes types), so it
    /// is a latent divergence rather than a live bug — but this path is the
    /// weaker of the two, because it succeeds.
    ///
    /// # Errors
    ///
    /// - Any layer's interpretation fails.
    /// - A layer has no blocks *and* a non-empty interface — a malformed
    ///   morphism. A block-free layer whose interface is empty is `id_I` and is
    ///   interpreted, not rejected: that is how `FrobeniusMorphism::identity`
    ///   represents the identity on the empty type list, and rejecting it (as
    ///   this default did before #284) makes a legal value uninterpretable.
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
                if !layer.left_type.is_empty() || !layer.right_type.is_empty() {
                    return Err(CatgraphError::Interpret {
                        context: format!(
                            "block-free FrobeniusMorphism layer with a {}→{} interface",
                            layer.left_type.len(),
                            layer.right_type.len()
                        ),
                    });
                }
                // `id_I` is the unit of the fold: contribute nothing.
                continue;
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

#[cfg(test)]
mod tests {
    use super::{Frobenius, FrobeniusMorphism, FrobeniusOperation};
    use crate::{
        category::{ComposableMutating, HasIdentity},
        cospan_algebra::frobenius_to_cospan,
        errors::CatgraphError,
        monoidal::{Monoidal, MonoidalMutatingMorphism, SymmetricMonoidalMorphism},
    };
    use permutations::Permutation;

    type Inner = FrobeniusMorphism<char, String>;

    /// A `Frobenius` implementor that supplies **only** the four generator
    /// methods and so runs the trait's `basic_interpret` / `interpret_frob`
    /// *defaults* — the bodies `FrobeniusMorphism` itself overrides, and which
    /// no test could otherwise reach.
    ///
    /// Everything else delegates to a wrapped `FrobeniusMorphism`, so a
    /// difference between the default and the override is a difference in the
    /// default, not in the carrier.
    #[derive(PartialEq, Eq, Clone)]
    struct Defaulting(Inner);

    impl Monoidal for Defaulting {
        fn monoidal(&mut self, other: Self) {
            self.0.monoidal(other.0);
        }
    }

    impl HasIdentity<Vec<char>> for Defaulting {
        fn identity(on_this: &Vec<char>) -> Self {
            Self(Inner::identity(on_this))
        }
    }

    impl ComposableMutating<Vec<char>> for Defaulting {
        fn compose(&mut self, other: Self) -> Result<(), CatgraphError> {
            self.0.compose(other.0)
        }
        fn domain(&self) -> Vec<char> {
            self.0.domain()
        }
        fn codomain(&self) -> Vec<char> {
            self.0.codomain()
        }
    }

    impl MonoidalMutatingMorphism<Vec<char>> for Defaulting {}

    impl SymmetricMonoidalMorphism<char> for Defaulting {
        fn permute_side(&mut self, p: &Permutation, of_codomain: bool) {
            self.0.permute_side(p, of_codomain);
        }
        fn from_permutation_on_domain(
            p: Permutation,
            types: &[char],
        ) -> Result<Self, CatgraphError> {
            Inner::from_permutation_on_domain(p, types).map(Self)
        }
        fn from_permutation_on_codomain(
            p: Permutation,
            types: &[char],
        ) -> Result<Self, CatgraphError> {
            Inner::from_permutation_on_codomain(p, types).map(Self)
        }
    }

    impl Frobenius<char, String> for Defaulting {
        fn interpret_unit(z: char) -> Self {
            Self(FrobeniusOperation::Unit(z).into())
        }
        fn interpret_counit(z: char) -> Self {
            Self(FrobeniusOperation::Counit(z).into())
        }
        fn interpret_multiplication(z: char) -> Self {
            Self(FrobeniusOperation::Multiplication(z).into())
        }
        fn interpret_comultiplication(z: char) -> Self {
            Self(FrobeniusOperation::Comultiplication(z).into())
        }
    }

    fn no_boxes(label: &String, _: &[char], _: &[char]) -> Result<Defaulting, CatgraphError> {
        Err(CatgraphError::Interpret {
            context: format!("no black boxes in this test ({label})"),
        })
    }

    /// The `basic_interpret` default sends `σ: [z, w] → [w, z]` to a real
    /// transposition.
    ///
    /// This line built `Permutation::try_from(vec![0, 1])` — the *identity* —
    /// until #284. Nothing caught it because `FrobeniusMorphism`, the crate's
    /// only `Frobenius` implementor, overrides `basic_interpret`; `Defaulting`
    /// exists to reach the default body. The assertion is on the **codomain**,
    /// which is what the permutation decides: the identity permutation leaves
    /// it `['a', 'b']`, the transposition makes it `['b', 'a']`.
    ///
    /// **Space:** one braiding, `('a', 'b')` — two *distinct* labels, so the
    /// identity and the transposition are distinguishable at all (`('a', 'a')`
    /// would make this vacuous).
    #[test]
    fn basic_interpret_default_braiding_is_a_transposition() {
        let braid = FrobeniusOperation::SymmetricBraiding('a', 'b');
        let got = Defaulting::basic_interpret(&braid, &no_boxes).expect("braiding interprets");
        assert_eq!(got.domain(), vec!['a', 'b'], "braiding domain");
        assert_eq!(
            got.codomain(),
            vec!['b', 'a'],
            "braiding codomain: ['a', 'b'] would mean the permutation is the \
             identity, not the transposition"
        );
        // And it agrees with the override, which is the semantics the rest of
        // the crate actually runs on.
        let via_override: Inner = braid.into();
        assert_eq!(got.codomain(), via_override.codomain());
    }

    /// The `interpret_frob` default accepts `identity(&vec![])`.
    ///
    /// That value's single layer is block-free with an empty interface — `id_I`
    /// — and the default rejected *every* block-free layer until #284
    /// ("somehow an empty layer in a frobenius morphism???"), so a legal value
    /// the crate constructs was uninterpretable. A block-free layer with a
    /// non-empty interface is still rejected; that half is unreachable through
    /// the public constructors (`rebuild_from_ops` recomputes the types from
    /// the ops), so it is not asserted here.
    ///
    /// **Space:** `identity(&vec![])` and, as a control that the accept is not
    /// an accept-everything, `identity(&vec!['a'])` and `μ;δ`.
    #[test]
    fn interpret_frob_default_accepts_the_empty_identity() {
        let empty_id: Inner = Inner::identity(&vec![]);
        let got = Defaulting::interpret_frob(&empty_id, &no_boxes)
            .expect("id_I is a legal morphism, not a malformed layer");
        assert!(got.domain().is_empty(), "id_I domain");
        assert!(got.codomain().is_empty(), "id_I codomain");

        let mut mu_delta: Inner = FrobeniusOperation::Multiplication('a').into();
        mu_delta
            .compose(FrobeniusOperation::Comultiplication('a').into())
            .expect("μ;δ interfaces match");
        for (label, term) in [
            ("id_a", Inner::identity(&vec!['a'])),
            ("mu_delta", mu_delta),
        ] {
            let round = Defaulting::interpret_frob(&term, &no_boxes)
                .unwrap_or_else(|e| panic!("{label} interprets: {e:?}"));
            assert_eq!(
                frobenius_to_cospan(&round.0).unwrap().canonical_form(),
                frobenius_to_cospan(&term).unwrap().canonical_form(),
                "{label}: the default interpretation is not the term itself"
            );
        }
    }
}
