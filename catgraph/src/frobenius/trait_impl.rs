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
    /// `Spider(z, 0, 0)` — the bubble `η;ε` — is special-cased in **both**
    /// twins, for the same reason: [`special_frobenius_morphism`] hands back the
    /// *simplified* term, and the simplifier's rule 3 cancels `η;ε` under the
    /// extra-special axiom, which the special theories these functions interpret
    /// into do not satisfy. Recursing there would return `id_I` for a
    /// non-identity scalar. Every other arity recurses.
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
            FrobeniusOperation::Spider(z, 0, 0) => {
                // The bubble `η;ε`, built directly — the same carve-out
                // `generator_to_cospan` takes, and for the same reason (#284).
                // `special_frobenius_morphism` returns the **simplified** term,
                // and `two_layer_simplify`'s rule 3 cancels `η;ε` outright (the
                // *extra-special* axiom), so the general arm below would recurse
                // into an emptied term and hand back `Self::identity(&[])`. In
                // the merely *special* theories this trait is meant to
                // interpret into — `Cospan` is one — the bubble is a genuine
                // `0 → 0` non-identity, so that answer is unsound. Pinned by
                // `tests::basic_interpret_default_spider_zero_zero_is_the_bubble`
                // (which needs a `Cospan`-backed implementor: rule 3 makes the
                // divergence invisible to `FrobeniusMorphism`-backed ones).
                let mut bubble = Self::interpret_unit(*z);
                bubble.compose(Self::interpret_counit(*z))?;
                bubble
            }
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
        category::{Composable, ComposableMutating, HasIdentity},
        cospan::Cospan,
        cospan_algebra::frobenius_to_cospan,
        errors::CatgraphError,
        hypergraph_category::HypergraphCategory,
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
    ///
    /// ⚠ **Blind at `Spider(z, 0, 0)`.** The carrier is a `FrobeniusMorphism`,
    /// whose layer simplifier cancels `η;ε` (rule 3, the extra-special axiom),
    /// so the bubble and `id_I` are the *same value* here and no assertion on
    /// this type can separate the fixed `(0, 0)` arm from the broken one. That
    /// generator is pinned on [`CospanBacked`] instead.
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

    /// A second `Frobenius` implementor that also runs the trait *defaults*, but
    /// over a [`Cospan`] carrier instead of a `FrobeniusMorphism` one.
    ///
    /// [`Defaulting`] cannot see the `Spider(z, 0, 0)` arm at all — its carrier
    /// quotients by the extra-special axiom, which identifies the bubble with
    /// `id_I`. `Cospan` is the merely **special** theory: there the bubble is a
    /// genuine `0 → 0` non-identity (apex 1, one scalar class), so the two
    /// answers are distinguishable, and this is the carrier the crate's own
    /// `Cospan`-valued twin `frobenius_to_cospan` uses.
    ///
    /// The wrapper exists only because `Cospan` is
    /// [`Composable`] (immutable) while `Frobenius` requires
    /// [`ComposableMutating`] — exactly the mismatch
    /// [`Frobenius::basic_interpret`]'s rustdoc names as the reason the twin is
    /// a separate function. Every method here delegates, so a difference between
    /// this and `frobenius_to_cospan` is a difference in the default body.
    struct CospanBacked(Cospan<char>);

    impl Monoidal for CospanBacked {
        fn monoidal(&mut self, other: Self) {
            self.0.monoidal(other.0);
        }
    }

    impl HasIdentity<Vec<char>> for CospanBacked {
        fn identity(on_this: &Vec<char>) -> Self {
            Self(Cospan::identity(on_this))
        }
    }

    impl ComposableMutating<Vec<char>> for CospanBacked {
        fn compose(&mut self, other: Self) -> Result<(), CatgraphError> {
            self.0 = Composable::compose(&self.0, &other.0)?;
            Ok(())
        }
        fn domain(&self) -> Vec<char> {
            Composable::domain(&self.0)
        }
        fn codomain(&self) -> Vec<char> {
            Composable::codomain(&self.0)
        }
    }

    impl MonoidalMutatingMorphism<Vec<char>> for CospanBacked {}

    impl SymmetricMonoidalMorphism<char> for CospanBacked {
        fn permute_side(&mut self, p: &Permutation, of_codomain: bool) {
            self.0.permute_side(p, of_codomain);
        }
        fn from_permutation_on_domain(
            p: Permutation,
            types: &[char],
        ) -> Result<Self, CatgraphError> {
            Cospan::from_permutation_on_domain(p, types).map(Self)
        }
        fn from_permutation_on_codomain(
            p: Permutation,
            types: &[char],
        ) -> Result<Self, CatgraphError> {
            Cospan::from_permutation_on_codomain(p, types).map(Self)
        }
    }

    impl Frobenius<char, String> for CospanBacked {
        fn interpret_unit(z: char) -> Self {
            Self(Cospan::unit(z))
        }
        fn interpret_counit(z: char) -> Self {
            Self(Cospan::counit(z))
        }
        fn interpret_multiplication(z: char) -> Self {
            Self(Cospan::multiplication(z))
        }
        fn interpret_comultiplication(z: char) -> Self {
            Self(Cospan::comultiplication(z))
        }
    }

    fn no_cospan_boxes(
        label: &String,
        _: &[char],
        _: &[char],
    ) -> Result<CospanBacked, CatgraphError> {
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

    /// The `basic_interpret` default sends `Spider(z, 0, 0)` to the bubble
    /// `η;ε`, not to `id_I`.
    ///
    /// Until #284's second pass this arm recursed into
    /// `special_frobenius_morphism(0, 0, z)`, whose `η;ε` the layer simplifier's
    /// rule 3 has *already cancelled* under the extra-special axiom; the
    /// resulting empty term then interpreted to `Self::identity(&[])`. The
    /// `Cospan`-valued twin `generator_to_cospan` had the identical bug and was
    /// fixed one commit earlier, which left the two disagreeing at exactly this
    /// generator — contradicting
    /// [`basic_interpret`](Frobenius::basic_interpret)'s own "must agree
    /// generator-for-generator". Any *special-but-not-extra-special* implementor
    /// inheriting the default got `id_I` for a non-identity scalar.
    ///
    /// **Why the carrier is a `Cospan` and not a `FrobeniusMorphism`:**
    /// [`Defaulting`] is structurally incapable of observing this. Its carrier
    /// quotients by rule 3, so the bubble and `id_I` are the same value there
    /// and every assertion on it passes either way. `Cospan` is the merely
    /// special theory, where the bubble is a genuine `0 → 0` non-identity.
    ///
    /// **Space (narrow, stated honestly):** one generator, `Spider('a', 0, 0)`,
    /// on one label, through one `Cospan`-backed implementor. It does *not*
    /// range over other labels, other carriers, or the `(0, n)`/`(m, 0)` spiders
    /// — those still recurse, and the `(1, 1)`/`(2, 2)` controls below check
    /// only that the new arm did not swallow the general one.
    #[test]
    fn basic_interpret_default_spider_zero_zero_is_the_bubble() {
        let bubble_op: FrobeniusOperation<char, String> = FrobeniusOperation::Spider('a', 0, 0);
        let got = CospanBacked::basic_interpret(&bubble_op, &no_cospan_boxes)
            .expect("the (0, 0) spider interprets");
        let canon = got.0.canonical_form();

        assert_eq!(
            canon.apex_len(),
            1,
            "Spider('a', 0, 0) apex: got {}, want 1. A 0 means the default \
             recursed into special_frobenius_morphism(0, 0, 'a') — the term \
             rule 3 has already emptied — and returned id_I.",
            canon.apex_len()
        );
        assert_eq!(
            canon.scalar_count(),
            1,
            "Spider('a', 0, 0) scalars: got {}, want 1 (the bubble is one \
             scalar class); 0 is the id_I answer.",
            canon.scalar_count()
        );

        // The bubble spelled out by hand on the same carrier.
        let by_hand = Composable::compose(&Cospan::unit('a'), &Cospan::counit('a'))
            .expect("η;ε composes: counit's domain is unit's codomain");
        assert_eq!(
            canon,
            by_hand.canonical_form(),
            "the default's (0, 0) spider is not η;ε"
        );

        // …and the generator-for-generator agreement with the `Cospan`-valued
        // twin that `basic_interpret`'s rustdoc claims.
        let twin: Inner = bubble_op.clone().into();
        assert_eq!(
            canon,
            frobenius_to_cospan(&twin)
                .expect("the (0, 0) spider interprets")
                .canonical_form(),
            "the default and generator_to_cospan disagree at Spider('a', 0, 0)"
        );

        // Controls: the new arm is narrow — every other arity still recurses,
        // and still agrees with the twin.
        for (label, d1, d2) in [("1x1", 1, 1), ("2x2", 2, 2)] {
            let op: FrobeniusOperation<char, String> = FrobeniusOperation::Spider('a', d1, d2);
            let via_default = CospanBacked::basic_interpret(&op, &no_cospan_boxes)
                .unwrap_or_else(|e| panic!("{label} spider interprets: {e:?}"));
            let via_twin: Inner = op.into();
            assert_eq!(
                via_default.0.canonical_form(),
                frobenius_to_cospan(&via_twin)
                    .unwrap_or_else(|e| panic!("{label} twin interprets: {e:?}"))
                    .canonical_form(),
                "{label}: the (0, 0) carve-out changed a general spider"
            );
        }
    }
}
