//! Cospan-algebras: lax symmetric monoidal functors from cospans to sets
//! (Fong-Spivak §2.1, Definition 2.2).
//!
//! A cospan-algebra `(Λ, a)` consists of a label set `Λ` and a lax symmetric
//! monoidal functor `a: Cospan_Λ → C` for some target category `C`.
//!
//! The [`CospanAlgebra`] trait captures this functoriality element-wise:
//! - [`map_cospan`](CospanAlgebra::map_cospan) transforms a single element under a cospan
//! - [`lax_monoidal`](CospanAlgebra::lax_monoidal) combines elements from `a(x)` and `a(y)` into `a(x ⊕ y)`
//! - [`unit`](CospanAlgebra::unit) provides the element of `a(I)`
//!
//! ## Implementations
//!
//! - [`PartitionAlgebra`]: the initial cospan-algebra where `a(x) = Cospan(0, x)` (Example 2.3)
//! - [`NameAlgebra`]: `a(x) = H(I, P(x))` — named morphisms via the compact closed structure (Prop 3.2)

use std::fmt::Debug;

use crate::{
    category::{Composable, HasIdentity},
    cospan::Cospan,
    errors::CatgraphError,
    monoidal::Monoidal,
};

/// A lax symmetric monoidal functor `a: Cospan_Λ → C`, operating element-wise.
///
/// `Elem` is the type of elements in the target sets `a(x)`.
/// The functor maps each cospan `c: m → p ← n` to a function `a(c): a(m) → a(n)`,
/// realized by [`map_cospan`](Self::map_cospan) applied to individual elements.
pub trait CospanAlgebra<Lambda: Eq + Copy + Debug> {
    /// Element type in the target category.
    type Elem;

    /// Apply the functorial action of a cospan to an element.
    ///
    /// Given `c: m → p ← n` and `e ∈ a(dom(c))`, produces `a(c)(e) ∈ a(cod(c))`.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError`] if the element is incompatible with the cospan's domain.
    fn map_cospan(
        &self,
        cospan: &Cospan<Lambda>,
        element: &Self::Elem,
    ) -> Result<Self::Elem, CatgraphError>;

    /// Lax monoidal coherence map: `a(x) × a(y) → a(x ⊕ y)`.
    ///
    /// Combines an element from `a(x)` and an element from `a(y)` into
    /// an element of `a(x ⊕ y)`.
    fn lax_monoidal(&self, a: &Self::Elem, b: &Self::Elem) -> Self::Elem;

    /// Unit coherence: the distinguished element of `a(I) = a([])`.
    fn unit(&self) -> Self::Elem;
}

// ---------------------------------------------------------------------------
// PartitionAlgebra (Example 2.3)
// ---------------------------------------------------------------------------

/// The partition cospan-algebra: `Part_Λ(x) = Cospan_Λ(0, x)`.
///
/// An element of `Part(x)` is a cospan from `[]` to `x` — it describes a way
/// to partition `x` into labeled groups. This is the initial cospan-algebra
/// (every cospan-algebra receives a unique map from `Part`).
///
/// - `map_cospan`: pushout composition `e ; c` where `e: [] → m` and `c: m → p ← n`.
/// - `lax_monoidal`: monoidal product of cospans.
/// - `unit`: the empty cospan `[] → [] ← []`.
#[derive(Default)]
pub struct PartitionAlgebra;

impl<Lambda> CospanAlgebra<Lambda> for PartitionAlgebra
where
    Lambda: Eq + Copy + Debug,
{
    type Elem = Cospan<Lambda>;

    fn map_cospan(
        &self,
        cospan: &Cospan<Lambda>,
        element: &Self::Elem,
    ) -> Result<Self::Elem, CatgraphError> {
        // element: [] → m (a partition of the domain)
        // cospan: m → p ← n
        // result: element ; cospan = [] → p ← n ... but we want the
        // induced element in a(cod(c)), which is a cospan [] → n.
        //
        // Composing element ([] → m) with cospan (m → n via pushout)
        // gives a cospan [] → n, which is an element of Part(n).
        element.compose(cospan)
    }

    fn lax_monoidal(&self, a: &Self::Elem, b: &Self::Elem) -> Self::Elem {
        let mut result = a.clone();
        result.monoidal(b.clone());
        result
    }

    fn unit(&self) -> Self::Elem {
        Cospan::empty()
    }
}

// ---------------------------------------------------------------------------
// NameAlgebra (§4.1: A_H(x) = H(I, P(x)))
// ---------------------------------------------------------------------------

use crate::frobenius::{
    FrobeniusMorphism, FrobeniusOperation, from_decomposition, special_frobenius_morphism,
};

/// The name cospan-algebra: `A_H(x) = H(I, P(x))` — named morphisms.
///
/// An element of `A_H(x)` is a `FrobeniusMorphism` with domain `[]` and codomain `x`
/// (a "name" in the sense of Fong-Spivak Prop 3.2).
///
/// - `map_cospan`: interprets the cospan as a Frobenius morphism via
///   [`from_decomposition`], then composes with the named element.
/// - `lax_monoidal`: monoidal product of morphisms.
/// - `unit`: identity on `[]`.
///
/// The `BlackBoxLabel` type parameter is carried for compatibility with the
/// Frobenius morphism infrastructure.
pub struct NameAlgebra<BlackBoxLabel: Eq + Clone + Send + Sync> {
    _phantom: std::marker::PhantomData<BlackBoxLabel>,
}

impl<BlackBoxLabel: Eq + Clone + Send + Sync> NameAlgebra<BlackBoxLabel> {
    /// Create a new `NameAlgebra` instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<BlackBoxLabel: Eq + Clone + Send + Sync> Default for NameAlgebra<BlackBoxLabel> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Lambda, BlackBoxLabel> CospanAlgebra<Lambda> for NameAlgebra<BlackBoxLabel>
where
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
{
    type Elem = FrobeniusMorphism<Lambda, BlackBoxLabel>;

    fn map_cospan(
        &self,
        cospan: &Cospan<Lambda>,
        element: &Self::Elem,
    ) -> Result<Self::Elem, CatgraphError> {
        // element: [] → domain(cospan) as a FrobeniusMorphism
        // cospan: domain → codomain
        // We interpret the cospan as a FrobeniusMorphism and compose.
        let cospan_morph: FrobeniusMorphism<Lambda, BlackBoxLabel> = cospan_to_frobenius(cospan)?;
        let mut result = element.clone();
        crate::category::ComposableMutating::compose(&mut result, cospan_morph)?;
        Ok(result)
    }

    fn lax_monoidal(&self, a: &Self::Elem, b: &Self::Elem) -> Self::Elem {
        let mut result = a.clone();
        result.monoidal(b.clone());
        result
    }

    fn unit(&self) -> Self::Elem {
        FrobeniusMorphism::identity(&vec![])
    }
}

// ---------------------------------------------------------------------------
// Lemma 4.3: A_F natural transformation from a hypergraph functor F: H → H'
// ---------------------------------------------------------------------------

/// Lemma 4.3 — induced cospan-algebra morphism `α: A_H → A_H'` from a hypergraph
/// functor `F: H → H'`.
///
/// The element-wise formula is `α_x(e) := F(e)` — a simple pointwise application
/// of the functor to each element of `A_H(x) = H(I, P(x))`. The content of
/// Lemma 4.3 is that this pointwise map is automatically a monoidal natural
/// transformation between the two name-algebras whenever `F` preserves
/// composition, tensor product, and Frobenius structure.
///
/// The paper states Lemma 4.3 for an **io** (identity-on-objects) hypergraph
/// functor over a fixed `Λ`; this function additionally accepts cross-label
/// functors (`L1 ≠ L2`, e.g. `RelabelingFunctor`) — a beyond-paper
/// generalization in the direction of the paper's Eq (29) naturality square
/// (the cross-Λ `Cospan_f` machinery itself is not implemented; see
/// `docs/FS19-AUDIT.md` and #109).
///
/// This free function is the direct embodiment of the paper's construction:
/// it is a thin wrapper around `F.map_mor(e)` whose purpose is to make the
/// Lemma 4.3 correspondence explicit at the type level. Callers plug in any
/// [`HypergraphFunctor`](crate::hypergraph_functor::HypergraphFunctor) impl and
/// obtain the induced algebra map for free.
///
/// # Type parameters
///
/// - `L1`: source label set
/// - `L2`: target label set
/// - `Src`: source category morphism type (must impl
///   [`HypergraphCategory<L1>`](crate::hypergraph_category::HypergraphCategory))
/// - `Tgt`: target category morphism type (must impl
///   [`HypergraphCategory<L2>`](crate::hypergraph_category::HypergraphCategory))
/// - `F`: the hypergraph functor
///
/// # Errors
///
/// Propagates any [`CatgraphError`] returned by `functor.map_mor`.
///
/// # Verification
///
/// Naturality, monoidality, and unit preservation of the induced morphism are
/// verified as proptests in `tests/cospan_algebra.rs` for two concrete cases:
///
/// - `F = RelabelingFunctor` on `Cospan<L1> → Cospan<L2>` with two
///   [`PartitionAlgebra`] instances
/// - `F = CospanToFrobeniusFunctor` on `Cospan<L> → FrobeniusMorphism<L, BL>`
///   relating [`PartitionAlgebra`] to [`NameAlgebra`]
pub fn functor_induced_algebra_map<L1, L2, Src, Tgt, F>(
    functor: &F,
    element: &Src,
) -> Result<Tgt, CatgraphError>
where
    L1: Eq + Copy + Debug,
    L2: Eq + Copy + Debug,
    Src: crate::hypergraph_category::HypergraphCategory<L1>,
    Tgt: crate::hypergraph_category::HypergraphCategory<L2>,
    F: crate::hypergraph_functor::HypergraphFunctor<L1, L2, Src, Tgt>,
{
    functor.map_mor(element)
}

/// Convert a `Cospan<Lambda>` into a `FrobeniusMorphism` by decomposing
/// each leg through epi-mono factorization (Fong-Spivak Lemma 3.6).
///
/// This is the morphism-mapping component of the hypergraph functor
/// `Cospan_Λ → FrobeniusMorphism_Λ` (Prop 3.8).
///
/// # Errors
///
/// Returns [`CatgraphError`] if the epi-mono decomposition fails for either leg.
pub fn cospan_to_frobenius<Lambda, BlackBoxLabel>(
    cospan: &Cospan<Lambda>,
) -> Result<FrobeniusMorphism<Lambda, BlackBoxLabel>, CatgraphError>
where
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
{
    use crate::category::ComposableMutating;
    use crate::finset::Decomposition;

    let domain = cospan.domain();
    let codomain = cospan.codomain();
    let middle = cospan.middle();
    let middle_len = middle.len();

    // Identity fast path
    if domain == codomain && cospan.left_to_middle() == cospan.right_to_middle() {
        return Ok(FrobeniusMorphism::identity(&domain));
    }

    // Compute leftover for a leg: codomain elements beyond max image index.
    let leftover = |map: &[usize]| -> usize {
        if map.is_empty() {
            middle_len
        } else {
            let max_idx = map.iter().copied().max().unwrap_or(0);
            middle_len.saturating_sub(max_idx + 1)
        }
    };

    // Build the left leg: domain → middle
    let left_map = cospan.left_to_middle().to_vec();
    let left_leftover = leftover(&left_map);
    let left_decomp = Decomposition::try_from((left_map, left_leftover)).map_err(|e| {
        CatgraphError::Composition {
            message: format!("left leg decomposition failed: {e}"),
        }
    })?;
    let left_morph: FrobeniusMorphism<Lambda, BlackBoxLabel> =
        from_decomposition(left_decomp, &domain, middle)?;

    // Build the right leg: codomain → middle, then flip to get middle → codomain
    let right_map = cospan.right_to_middle().to_vec();
    let right_leftover = leftover(&right_map);
    let right_decomp = Decomposition::try_from((right_map, right_leftover)).map_err(|e| {
        CatgraphError::Composition {
            message: format!("right leg decomposition failed: {e}"),
        }
    })?;
    let mut right_morph: FrobeniusMorphism<Lambda, BlackBoxLabel> =
        from_decomposition(right_decomp, &codomain, middle)?;
    // hflip reverses the morphism (dagger in the Frobenius sense).
    right_morph.hflip(&std::convert::identity);

    // Compose: domain → middle → codomain
    let mut result = left_morph;
    ComposableMutating::compose(&mut result, right_morph)?;
    Ok(result)
}

/// Interpret a single Frobenius generator as a `Cospan<Lambda>`.
///
/// The four Frobenius generators and the identity come straight from
/// [`HypergraphCategory`](crate::hypergraph_category::HypergraphCategory); the
/// braiding `σ: [z, w] → [w, z]` is the two-vertex apex whose right leg is the
/// transposition. `Spider(z, d1, d2)` is interpreted by recursing on the
/// generator decomposition [`special_frobenius_morphism`] gives it — with one
/// deliberate exception. What that function returns is the **simplified** term,
/// not the raw decomposition, so recursion inherits the simplifier's quotient;
/// at `(0, 0)` that quotient is the extra-special axiom and it empties the term
/// entirely. The `(0, 0)` arm therefore builds the bubble `η;ε` directly. Away
/// from `(0, 0)` the two agree and the spider's semantics here are the crate's
/// own rather than a second guess at them.
fn generator_to_cospan<Lambda, BlackBoxLabel>(
    op: &FrobeniusOperation<Lambda, BlackBoxLabel>,
) -> Result<Cospan<Lambda>, CatgraphError>
where
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
{
    use crate::category::Composable;
    use crate::hypergraph_category::HypergraphCategory;

    Ok(match op {
        FrobeniusOperation::Unit(z) => Cospan::unit(*z),
        FrobeniusOperation::Counit(z) => Cospan::counit(*z),
        FrobeniusOperation::Multiplication(z) => Cospan::multiplication(*z),
        FrobeniusOperation::Comultiplication(z) => Cospan::comultiplication(*z),
        FrobeniusOperation::Identity(z) => Cospan::identity(&vec![*z]),
        // σ: [z, w] → [w, z]. Correct by construction: both legs are
        // permutations of `0..2` and the apex has exactly two vertices.
        FrobeniusOperation::SymmetricBraiding(z, w) => {
            Cospan::new_unchecked(vec![0, 1], vec![1, 0], vec![*z, *w])
        }
        // The bubble, built directly. `special_frobenius_morphism` returns the
        // **simplified** term, and `two_layer_simplify`'s rule 3 cancels `η;ε`
        // outright — the *extra-special* axiom. `Cospan` is the **special**
        // theory, in which the bubble is a genuine `0 → 0` non-identity, so
        // recursing here would interpret a term the simplifier has already
        // emptied and hand back `apex 0`, breaking soundness against any
        // SCFM-equal spelling of the same scalar. Pinned by
        // `tests::scfm_equal_scalars_have_equal_images`.
        FrobeniusOperation::Spider(z, 0, 0) => Cospan::unit(*z).compose(&Cospan::counit(*z))?,
        FrobeniusOperation::Spider(z, d1, d2) => frobenius_to_cospan(
            &special_frobenius_morphism::<Lambda, BlackBoxLabel>(*d1, *d2, *z),
        )?,
        FrobeniusOperation::UnSpecifiedBox(_, srcs, tgts) => {
            return Err(CatgraphError::Interpret {
                context: format!(
                    "frobenius_to_cospan has no interpretation for a black box \
                     ({} in, {} out): Cospan is the free hypergraph category on \
                     the generators alone",
                    srcs.len(),
                    tgts.len()
                ),
            });
        }
    })
}

/// Interpret a `FrobeniusMorphism<Lambda, _>` as a `Cospan<Lambda>` — inverse
/// in spirit to [`cospan_to_frobenius`].
///
/// Each layer is the monoidal product of its blocks' generator cospans, and the
/// layers are composed in order. The `Frobenius`-valued twin of this function is
/// [`Frobenius::interpret_frob`](crate::frobenius::Frobenius::interpret_frob),
/// which has the same shape; `Cospan` is [`Composable`], not
/// [`ComposableMutating`](crate::category::ComposableMutating), so it cannot
/// implement `Frobenius` and reuse that body.
///
/// # What Prop 3.8 does and does not license here
///
/// F&S 2019 Prop 3.8 is a one-to-one correspondence between special commutative
/// Frobenius monoids (SCFMs) in a symmetric monoidal category `C` and strict
/// symmetric monoidal functors `(Cospan, ⊕) → (C, ⊗)`. **Both** of its
/// directions concern functors *out of* `Cospan`, so neither of them is this
/// map. What it licenses is this: the black-box-free part of
/// `FrobeniusMorphism_Λ` is the free SCFM prop on `Λ`, `Cospan_Λ` carries an
/// SCFM structure on each object `[l]` (Example 2.8), and Prop 3.8 turns that
/// structure into the interpreting functor this function computes.
///
/// ⚠ **Neither direction of the resulting equality test is exact against SCFM,
/// and an earlier revision of this paragraph claimed the sound half.** Both
/// failures are measured, and both trace to the same place — `Cospan` is the
/// **special** theory, in which the bubble `η;ε` is a genuine `0 → 0`
/// non-identity, while `FrobeniusMorphism`'s layer simplifier quotients by the
/// *extra-special* axiom on top of SCFM.
///
/// - **Not sound.** `two_layer_simplify`'s rule 3 cancels a spelled `η;ε`
///   outright, so it interprets to the empty cospan (`apex 0`), while
///   `Spider(z, 0, 0)` — the same scalar, built directly by
///   `generator_to_cospan` rather than through the simplifier — interprets to
///   the bubble (`apex 1`, one scalar class). Two SCFM-equal terms, different
///   images.
/// - **Not complete.** That same cancellation sends the spelled `η;ε` and
///   `FrobeniusMorphism::identity(&vec![])` to the same empty cospan, and they
///   are not SCFM-equal.
///
/// Both witnesses are pinned in this module's
/// `tests::scalar_bubbles_are_lost_in_both_directions`, and the spider half of
/// soundness — which *is* repaired — in
/// `tests::scfm_equal_scalars_have_equal_images`. See also
/// [`cospan_canon`](crate::cospan_canon)'s module docs. So read
/// [`Cospan::canonical_form`](crate::cospan_canon::CospanCanon) on these images
/// as a *semantic* equality that is **incomparable** with SCFM on scalars —
/// still far better than `==` on `FM`, whose derived `Eq` compares layer vectors
/// and so separates diagrams that are equal on any reading, but not a decision
/// procedure for SCFM-equality until rule 3's scalar cancellation is addressed.
///
/// # Examples
///
/// Two syntactically different terms, equal under the Frobenius axioms:
///
/// ```
/// use catgraph::category::ComposableMutating;
/// use catgraph::cospan_algebra::frobenius_to_cospan;
/// use catgraph::frobenius::{FrobeniusMorphism, FrobeniusOperation};
///
/// type FM = FrobeniusMorphism<char, String>;
///
/// // μ ; δ, spelled out …
/// let mut spelled: FM = FrobeniusOperation::Multiplication('a').into();
/// spelled.compose(FrobeniusOperation::Comultiplication('a').into()).unwrap();
/// // … and the same map as a single 2-to-2 spider.
/// let spider: FM = FrobeniusOperation::Spider('a', 2, 2).into();
///
/// assert_eq!(
///     frobenius_to_cospan(&spelled).unwrap().canonical_form(),
///     frobenius_to_cospan(&spider).unwrap().canonical_form(),
/// );
/// ```
///
/// # Errors
///
/// - [`CatgraphError::Interpret`] if the morphism contains an
///   `UnSpecifiedBox`: a black box has no interpretation in the free
///   hypergraph category. Pass it through
///   [`MorphismSystem`](crate::frobenius::MorphismSystem) first if it needs one.
/// - [`CatgraphError::Interpret`] if a layer has no blocks but a non-empty
///   interface (a malformed morphism). A block-free layer whose interface is
///   empty is `id_I` and is interpreted, not rejected — that is how
///   `FrobeniusMorphism::identity(&vec![])` is represented.
/// - Any [`CatgraphError`] from the underlying cospan composition.
pub fn frobenius_to_cospan<Lambda, BlackBoxLabel>(
    morphism: &FrobeniusMorphism<Lambda, BlackBoxLabel>,
) -> Result<Cospan<Lambda>, CatgraphError>
where
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
{
    use crate::category::ComposableMutating;

    let mut answer = Cospan::identity(&ComposableMutating::domain(morphism));
    for layer in &morphism.layers {
        // A block-free layer is `id_I`, which is how `FrobeniusMorphism::identity`
        // represents the identity on the empty type list — legal, and the unit of
        // the fold below. A block-free layer with a *non-empty* interface is
        // malformed, and says so rather than silently contributing `id_I`.
        let mut current = match layer.blocks.first() {
            Some(first) => generator_to_cospan(&first.op)?,
            None => {
                if !layer.left_type.is_empty() || !layer.right_type.is_empty() {
                    return Err(CatgraphError::Interpret {
                        context: format!(
                            "block-free FrobeniusMorphism layer with a {}→{} interface",
                            layer.left_type.len(),
                            layer.right_type.len()
                        ),
                    });
                }
                Cospan::identity(&Vec::new())
            }
        };
        for block in layer.blocks.iter().skip(1) {
            current.monoidal(generator_to_cospan(&block.op)?);
        }
        answer = answer.compose(&current)?;
    }
    Ok(answer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::category::{Composable, ComposableMutating, HasIdentity};

    // --- PartitionAlgebra ---

    #[test]
    fn partition_unit_is_empty_cospan() {
        let alg = PartitionAlgebra;
        let u: Cospan<char> = alg.unit();
        assert!(u.domain().is_empty());
        assert!(u.codomain().is_empty());
    }

    #[test]
    fn partition_identity_cospan_preserves_element() {
        let alg = PartitionAlgebra;
        // element: [] → [a, b]
        let element = Cospan::new(vec![], vec![0, 1], vec!['a', 'b']).unwrap();
        let id = Cospan::<char>::identity(&vec!['a', 'b']);
        let mapped = alg.map_cospan(&id, &element).unwrap();
        assert_eq!(mapped.domain(), element.domain());
        assert_eq!(mapped.codomain(), element.codomain());
    }

    #[test]
    fn partition_composition_is_sequential() {
        let alg = PartitionAlgebra;
        // element: [] → [a]
        let element = Cospan::new(vec![], vec![0], vec!['a']).unwrap();
        // c1: [a] → [a, a] (identity into a merge)
        let c1 = Cospan::<char>::identity(&vec!['a']);
        // Map through identity
        let mapped = alg.map_cospan(&c1, &element).unwrap();
        assert_eq!(mapped.codomain(), vec!['a']);
    }

    #[test]
    fn partition_lax_monoidal_is_tensor() {
        let alg = PartitionAlgebra;
        let a = Cospan::new(vec![], vec![0], vec!['a']).unwrap();
        let b = Cospan::new(vec![], vec![0], vec!['b']).unwrap();
        let combined = alg.lax_monoidal(&a, &b);
        assert!(combined.domain().is_empty());
        assert_eq!(combined.codomain(), vec!['a', 'b']);
    }

    // --- NameAlgebra ---

    type FM = FrobeniusMorphism<char, String>;

    #[test]
    fn name_unit_is_empty_identity() {
        let alg = NameAlgebra::<String>::new();
        let u: FM = alg.unit();
        assert!(u.domain().is_empty());
        assert!(u.codomain().is_empty());
    }

    #[test]
    fn name_identity_cospan_preserves_element() {
        let alg = NameAlgebra::<String>::new();
        // element: [] → [a] (a named morphism — use unit η: [] → [a])
        let element: FM = crate::frobenius::FrobeniusOperation::Unit('a').into();
        let id_cospan = Cospan::<char>::identity(&vec!['a']);
        let mapped = alg.map_cospan(&id_cospan, &element).unwrap();
        assert_eq!(mapped.domain(), element.domain());
        assert_eq!(mapped.codomain(), element.codomain());
    }

    #[test]
    fn name_lax_monoidal_is_tensor() {
        let alg = NameAlgebra::<String>::new();
        let a: FM = crate::frobenius::FrobeniusOperation::Unit('a').into();
        let b: FM = crate::frobenius::FrobeniusOperation::Unit('b').into();
        let combined = alg.lax_monoidal(&a, &b);
        assert!(combined.domain().is_empty());
        assert_eq!(combined.codomain(), vec!['a', 'b']);
    }

    #[test]
    fn name_map_through_merge_cospan() {
        let alg = NameAlgebra::<String>::new();
        // element: [] → [a, a] (name of something with codomain [a,a])
        let element: FM = crate::compact_closed::cup_single('a');
        assert!(element.domain().is_empty());
        assert_eq!(element.codomain(), vec!['a', 'a']);

        // merge cospan: [a, a] → [a] (both inputs map to same middle node)
        let merge = Cospan::new(vec![0, 0], vec![0], vec!['a']).unwrap();
        let mapped = alg.map_cospan(&merge, &element).unwrap();
        assert!(mapped.domain().is_empty());
        assert_eq!(mapped.codomain(), vec!['a']);
    }

    #[test]
    fn cospan_to_frobenius_identity() {
        let id = Cospan::<char>::identity(&vec!['a', 'b']);
        let morph: FM = cospan_to_frobenius(&id).unwrap();
        assert_eq!(morph.domain(), vec!['a', 'b']);
        assert_eq!(morph.codomain(), vec!['a', 'b']);
    }

    #[test]
    fn cospan_to_frobenius_merge() {
        // [a, a] → [a]: both left nodes map to middle node 0
        let merge = Cospan::new(vec![0, 0], vec![0], vec!['a']).unwrap();
        let morph: FM = cospan_to_frobenius(&merge).unwrap();
        assert_eq!(morph.domain(), vec!['a', 'a']);
        assert_eq!(morph.codomain(), vec!['a']);
    }

    #[test]
    fn cospan_to_frobenius_split() {
        // [a] → [a, a]: right nodes both map to middle node 0
        let split = Cospan::new(vec![0], vec![0, 0], vec!['a']).unwrap();
        let morph: FM = cospan_to_frobenius(&split).unwrap();
        assert_eq!(morph.domain(), vec!['a']);
        assert_eq!(morph.codomain(), vec!['a', 'a']);
    }

    // --- frobenius_to_cospan (#284) ---

    /// Every generator lands on the cospan `HypergraphCategory` names for it.
    ///
    /// **Space:** the four Frobenius generators, the identity on one and two
    /// wires, and the braiding — i.e. every `FrobeniusOperation` variant except
    /// `Spider` (covered below) and `UnSpecifiedBox` (rejected, below). One
    /// label pair `('a', 'b')` only.
    #[test]
    fn frobenius_to_cospan_sends_generators_to_generators() {
        use crate::hypergraph_category::HypergraphCategory;

        let cases: Vec<(&str, FM, Cospan<char>)> = vec![
            (
                "eta",
                FrobeniusOperation::Unit('a').into(),
                Cospan::unit('a'),
            ),
            (
                "epsilon",
                FrobeniusOperation::Counit('a').into(),
                Cospan::counit('a'),
            ),
            (
                "mu",
                FrobeniusOperation::Multiplication('a').into(),
                Cospan::multiplication('a'),
            ),
            (
                "delta",
                FrobeniusOperation::Comultiplication('a').into(),
                Cospan::comultiplication('a'),
            ),
            (
                "id_a",
                FrobeniusOperation::Identity('a').into(),
                Cospan::identity(&vec!['a']),
            ),
            (
                "braid",
                FrobeniusOperation::SymmetricBraiding('a', 'b').into(),
                Cospan::new(vec![0, 1], vec![1, 0], vec!['a', 'b']).unwrap(),
            ),
        ];
        for (label, term, expected) in cases {
            let got = frobenius_to_cospan(&term).unwrap();
            assert_eq!(
                got.canonical_form(),
                expected.canonical_form(),
                "{label}: got apex {} / {:?}, expected apex {} / {:?}",
                got.middle().len(),
                got.canonical_form().classes(),
                expected.middle().len(),
                expected.canonical_form().classes(),
            );
        }
    }

    /// Prop 3.8 as a round trip: `cospan → frobenius → cospan` returns the
    /// cospan you started with, up to apex isomorphism.
    ///
    /// This is what makes [`frobenius_to_cospan`] usable as a *semantic oracle*
    /// (see `tests/compact_closed.rs`): it is checked against
    /// [`cospan_to_frobenius`], written independently in this module, rather
    /// than against itself.
    ///
    /// **Space:** the seven **scalar-free** cospans below, over `{'a','b'}` —
    /// identities, all four generators, and a merge-and-pass. Cospans carrying a
    /// bubble are deliberately *excluded*: scalars do not survive the trip. That
    /// exclusion is not a gap left silent — it is measured, with both of its
    /// separate causes, in `scalar_bubbles_are_lost_in_both_directions` below.
    /// The converse round trip (`frobenius → cospan → frobenius`) is *not* claimed:
    /// it cannot hold on the nose, since many layer vectors share one cospan.
    #[test]
    fn cospan_frobenius_cospan_round_trips() {
        use crate::hypergraph_category::HypergraphCategory;

        let cases: Vec<(&str, Cospan<char>)> = vec![
            ("id_ab", Cospan::identity(&vec!['a', 'b'])),
            ("id_empty", Cospan::identity(&vec![])),
            ("eta", Cospan::unit('a')),
            ("epsilon", Cospan::counit('a')),
            ("mu", Cospan::multiplication('a')),
            ("delta", Cospan::comultiplication('a')),
            (
                "merge_and_pass",
                Cospan::new(vec![0, 0], vec![0, 1], vec!['a', 'a']).unwrap(),
            ),
        ];
        for (label, cospan) in cases {
            let term: FM = cospan_to_frobenius(&cospan).unwrap();
            let back = frobenius_to_cospan(&term).unwrap();
            assert_eq!(
                back.canonical_form(),
                cospan.canonical_form(),
                "{label}: round trip gave apex {} (scalars {}), started from apex {} (scalars {})",
                back.canonical_form().apex_len(),
                back.canonical_form().scalar_count(),
                cospan.canonical_form().apex_len(),
                cospan.canonical_form().scalar_count(),
            );
        }
    }

    /// The **sound** direction, pinned on the pair that refuted it.
    ///
    /// `Spider(z, 0, 0)` *is* the bubble `η;ε`, and `η ; δ ; (ε ⊗ ε)` is
    /// SCFM-equal to it (counitality on one leg of `δ`). Soundness —
    /// SCFM-equal terms have equal images — therefore requires equal canonical
    /// forms here. Before the `generator_to_cospan` fix this was **false**:
    /// the spider arm interpreted `special_frobenius_morphism`'s *simplified*
    /// output, in which rule 3 had already cancelled the `η;ε`, so the spider
    /// gave `apex 0 / scalars 0` against the spelled term's `apex 1 /
    /// scalars 1`. `Cospan` is the **special**, not extra-special, theory: the
    /// bubble is a genuine `0 → 0` non-identity and must survive.
    ///
    /// **Space:** the `0 → 0` scalar and the same scalar beside `id_a`. The
    /// second case is the one that shows the failure was not confined to the
    /// empty object — it is a live shape inside what `samples()` ranges over.
    #[test]
    fn scfm_equal_scalars_have_equal_images() {
        use crate::monoidal::Monoidal;

        // η ; δ ; (ε ⊗ ε) — the bubble, spelled out.
        let mut spelled: FM = FrobeniusOperation::Unit('a').into();
        spelled
            .compose(FrobeniusOperation::Comultiplication('a').into())
            .unwrap();
        let mut counits: FM = FrobeniusOperation::Counit('a').into();
        counits.monoidal(FrobeniusOperation::Counit('a').into());
        spelled.compose(counits).unwrap();

        let bubble: FM = FrobeniusOperation::Spider('a', 0, 0).into();

        let spelled_canon = frobenius_to_cospan(&spelled).unwrap().canonical_form();
        let bubble_canon = frobenius_to_cospan(&bubble).unwrap().canonical_form();
        assert_eq!(
            bubble_canon,
            spelled_canon,
            "SCFM-equal 0→0 scalars must have equal images: \
             Spider(a,0,0) gave apex {} scalars {}, η;δ;(ε⊗ε) gave apex {} scalars {}",
            bubble_canon.apex_len(),
            bubble_canon.scalar_count(),
            spelled_canon.apex_len(),
            spelled_canon.scalar_count(),
        );
        assert_eq!(
            bubble_canon.scalar_count(),
            1,
            "the bubble is a scalar, not the empty cospan"
        );

        // Not confined to the empty object: `id_a ⊗ bubble` against
        // `id_a ⊗ (spelled bubble)`, two SCFM-equal `a → a` terms.
        let mut beside_bubble: FM = FrobeniusOperation::Identity('a').into();
        beside_bubble.monoidal(bubble);
        let mut beside_spelled: FM = FrobeniusOperation::Identity('a').into();
        beside_spelled.monoidal(spelled);

        let beside_bubble_canon = frobenius_to_cospan(&beside_bubble)
            .unwrap()
            .canonical_form();
        let beside_spelled_canon = frobenius_to_cospan(&beside_spelled)
            .unwrap()
            .canonical_form();
        assert_eq!(
            beside_bubble_canon,
            beside_spelled_canon,
            "id_a ⊗ scalar: apex {} scalars {} vs apex {} scalars {}",
            beside_bubble_canon.apex_len(),
            beside_bubble_canon.scalar_count(),
            beside_spelled_canon.apex_len(),
            beside_spelled_canon.scalar_count(),
        );
    }

    /// Spiders go through the crate's own generator decomposition, so an
    /// `(m, n)` spider is one apex vertex joining all `m + n` boundary wires.
    ///
    /// **Space:** `(m, n)` for `m, n <= 3` **except `(0, 0)`**, label `'a'` only.
    /// `(0, 0)` is the bubble, which the layer simplifier cancels — measured in
    /// `scalar_bubbles_are_lost_in_both_directions` rather than quietly skipped.
    #[test]
    fn frobenius_to_cospan_spiders_are_single_apex_vertices() {
        for m in 0..=3usize {
            for n in 0..=3usize {
                if m == 0 && n == 0 {
                    continue;
                }
                let spider: FM = FrobeniusOperation::Spider('a', m, n).into();
                let canon = frobenius_to_cospan(&spider).unwrap().canonical_form();
                assert_eq!(canon.dom_len(), m, "spider({m},{n}) domain");
                assert_eq!(canon.cod_len(), n, "spider({m},{n}) codomain");
                assert_eq!(
                    canon.apex_len(),
                    1,
                    "spider({m},{n}) should be one apex vertex, got {} ({:?})",
                    canon.apex_len(),
                    canon.classes()
                );
                assert_eq!(canon.scalar_count(), 0, "spider({m},{n}) scalar count");
            }
        }
    }

    /// ⚠ **Measured discrepancy, pinned as-is, not endorsed.**
    ///
    /// `Cospan` is the theory of **special**, not extra-special, commutative
    /// Frobenius monoids: the closed bubble `η # ε` is a genuine `0 → 0`
    /// non-identity and `k` bubbles are distinguished from `k-1` (see
    /// [`cospan_canon`](crate::cospan_canon)'s module docs). Neither
    /// `cospan_to_frobenius` nor `frobenius_to_cospan` preserves that today, and
    /// the two losses have **different causes** — established by disabling each
    /// in turn:
    ///
    /// 1. **`cospan_to_frobenius`'s identity fast path.** Its guard is
    ///    `domain == codomain && left_to_middle() == right_to_middle()`, so it
    ///    returns `identity(&domain)` and **discards every apex vertex neither
    ///    leg reaches, at any arity** — not only at `0 → 0`. Disabling
    ///    `two_layer_simplify`'s rule 3 does not change the `0 → 0` half, which
    ///    is what localises *that* half here rather than in the simplifier.
    /// 2. **`two_layer_simplify` rule 3.** It cancels `η(z);ε(z)` outright
    ///    ("scalar loop") — the *extra-special* axiom — so a spelled-out `η;ε`
    ///    interprets to `apex_len() == 0`; with rule 3 disabled it interprets to
    ///    `apex_len() == 1` instead, which is what localises this half to the
    ///    simplifier. **`Spider(z, 0, 0)` is no longer an instance of this.**
    ///    Its arm in `generator_to_cospan` builds the bubble directly rather
    ///    than recursing into `special_frobenius_morphism`'s already-simplified
    ///    output; recursing broke the *sound* direction outright, and the repair
    ///    is pinned by `scfm_equal_scalars_have_equal_images`.
    ///
    /// **Away from `0 → 0` the two causes overlap**, and the pin says so rather
    /// than mis-attributing the loss. `Cospan::new(vec![0], vec![0],
    /// vec!['a', 'b'])` — `id_a` beside a bubble — trips the fast-path guard
    /// (`domain == codomain`, legs equal) and loses its bubble; but narrowing
    /// the guard to `0 → 0` alone does **not** restore it, because the general
    /// decomposition path then produces `η('b');ε('b')` in the middle and rule 3
    /// cancels *that*. Measured: with the guard narrowed the term is still
    /// `identity(['a'])`; with rule 3 also disabled it is a depth-2 term whose
    /// bubble survives. So a fix to either cause alone leaves this case broken,
    /// which is why it is asserted separately below.
    ///
    /// The pin exists so all of this is *visible and testable*. ⚠ **What it
    /// actually signals is cause 2, not "either cause" — measured, and an
    /// earlier revision of this docstring claimed otherwise.** Correcting cause
    /// 1 in the strongest sensible form (guarding the fast path with
    /// `&& cospan.is_left_identity()` so it can never drop an unreached apex
    /// vertex) leaves this test GREEN along with the whole lib suite: with rule
    /// 3 still live the decomposition path emits `η;ε`, rule 3 cancels it, and
    /// every assertion below is satisfied for the same reason as before.
    /// Correcting cause 2 (rule 3 disabled) does turn it red. So a cause-1-only
    /// fix is exactly the partial fix this pin does **not** catch; treat the
    /// `id_a`-beside-a-bubble assertion as the both-causes signal and this
    /// sentence as the honest statement of the gap. It is not a claim that the
    /// current behaviour is right.
    #[test]
    fn scalar_bubbles_are_lost_in_both_directions() {
        let one_bubble = Cospan::new(vec![], vec![], vec!['a']).unwrap();
        let two_bubbles = Cospan::new(vec![], vec![], vec!['a', 'a']).unwrap();
        assert_eq!(one_bubble.canonical_form().scalar_count(), 1);
        assert_eq!(two_bubbles.canonical_form().scalar_count(), 2);
        assert_ne!(
            one_bubble.canonical_form(),
            two_bubbles.canonical_form(),
            "as cospans, one bubble and two are different morphisms"
        );

        // Cause 1: the identity fast path collapses both to id_I.
        let one_term: FM = cospan_to_frobenius(&one_bubble).unwrap();
        let two_term: FM = cospan_to_frobenius(&two_bubbles).unwrap();
        assert!(
            one_term == two_term,
            "the identity fast path maps every 0→0 cospan to the same term \
             (depths {} and {})",
            one_term.depth(),
            two_term.depth()
        );
        for (label, term) in [("one_bubble", &one_term), ("two_bubbles", &two_term)] {
            let back = frobenius_to_cospan(term).unwrap().canonical_form();
            assert_eq!(
                back.apex_len(),
                0,
                "{label}: round trip gave apex {}, the cospan had 1 or 2",
                back.apex_len()
            );
        }

        // Both causes at once, away from 0 → 0: the fast-path guard is about the
        // *legs*, not the arity, so `id_a` beside a bubble trips it; and even
        // with the guard narrowed to 0 → 0 the decomposition path emits
        // η('b');ε('b'), which rule 3 then cancels. Measured both ways — guard
        // narrowed: still `identity(['a'])`; guard narrowed *and* rule 3 off:
        // a depth-2 term keeping the bubble. So neither fix alone revives this
        // case — this assertion goes red only once *both* land, which is
        // exactly the signal a partial fix must not be allowed to hide.
        let id_a_and_bubble = Cospan::new(vec![0], vec![0], vec!['a', 'b']).unwrap();
        assert_eq!(id_a_and_bubble.canonical_form().apex_len(), 2);
        assert_eq!(id_a_and_bubble.canonical_form().scalar_count(), 1);
        let collapsed: FM = cospan_to_frobenius(&id_a_and_bubble).unwrap();
        assert!(
            collapsed == FM::identity(&vec!['a']),
            "an unreached apex vertex is dropped at arity 1 too, not just at \
             0→0 (depth {}; the bubble-keeping term measured with both causes \
             disabled has depth 2)",
            collapsed.depth()
        );
        let collapsed_back = frobenius_to_cospan(&collapsed).unwrap().canonical_form();
        assert_eq!(
            collapsed_back.apex_len(),
            1,
            "round trip gave apex {}, the cospan had 2 (one of them a bubble)",
            collapsed_back.apex_len()
        );

        // Cause 2, still live: rule 3 cancels `η;ε` in the *term* algebra, so a
        // spelled-out `η;ε` loses its bubble on the way to `Cospan`.
        let mut eta_eps: FM = FrobeniusOperation::Unit('a').into();
        eta_eps
            .compose(FrobeniusOperation::Counit('a').into())
            .unwrap();
        let eta_eps_canon = frobenius_to_cospan(&eta_eps).unwrap().canonical_form();
        assert_eq!(
            eta_eps_canon.apex_len(),
            0,
            "η;ε gave apex {}, the cospan bubble has 1 — rule 3 still cancels it",
            eta_eps_canon.apex_len()
        );

        // …but the (0,0) SPIDER no longer does. `generator_to_cospan` builds the
        // bubble directly instead of recursing into the simplifier's output, which
        // is what makes the sound direction hold — see
        // `scfm_equal_scalars_have_equal_images`. This assertion is the guard on
        // that repair: it goes red if the spider arm is reverted to recursion.
        let spider_0_0: FM = FrobeniusOperation::Spider('a', 0, 0).into();
        let spider_canon = frobenius_to_cospan(&spider_0_0).unwrap().canonical_form();
        assert_eq!(
            spider_canon.scalar_count(),
            1,
            "Spider(0,0) gave apex {} scalars {}, the cospan bubble has 1 scalar",
            spider_canon.apex_len(),
            spider_canon.scalar_count()
        );
    }

    /// A black box has no image in the free hypergraph category, and says so
    /// rather than inventing one.
    #[test]
    fn frobenius_to_cospan_rejects_black_boxes() {
        let boxed: FM =
            FrobeniusOperation::UnSpecifiedBox("f".to_string(), vec!['a'], vec!['b']).into();
        let err = frobenius_to_cospan(&boxed).unwrap_err();
        assert!(
            matches!(err, CatgraphError::Interpret { .. }),
            "expected Interpret, got {err:?}"
        );
    }
}
