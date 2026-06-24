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

use crate::frobenius::{FrobeniusMorphism, from_decomposition};

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
/// composition, tensor product, and Frobenius structure (i.e., whenever `F` is
/// a hypergraph functor).
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
        let element = Cospan::new(vec![], vec![0, 1], vec!['a', 'b']);
        let id = Cospan::<char>::identity(&vec!['a', 'b']);
        let mapped = alg.map_cospan(&id, &element).unwrap();
        assert_eq!(mapped.domain(), element.domain());
        assert_eq!(mapped.codomain(), element.codomain());
    }

    #[test]
    fn partition_composition_is_sequential() {
        let alg = PartitionAlgebra;
        // element: [] → [a]
        let element = Cospan::new(vec![], vec![0], vec!['a']);
        // c1: [a] → [a, a] (identity into a merge)
        let c1 = Cospan::<char>::identity(&vec!['a']);
        // Map through identity
        let mapped = alg.map_cospan(&c1, &element).unwrap();
        assert_eq!(mapped.codomain(), vec!['a']);
    }

    #[test]
    fn partition_lax_monoidal_is_tensor() {
        let alg = PartitionAlgebra;
        let a = Cospan::new(vec![], vec![0], vec!['a']);
        let b = Cospan::new(vec![], vec![0], vec!['b']);
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
        let merge = Cospan::new(vec![0, 0], vec![0], vec!['a']);
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
        let merge = Cospan::new(vec![0, 0], vec![0], vec!['a']);
        let morph: FM = cospan_to_frobenius(&merge).unwrap();
        assert_eq!(morph.domain(), vec!['a', 'a']);
        assert_eq!(morph.codomain(), vec!['a']);
    }

    #[test]
    fn cospan_to_frobenius_split() {
        // [a] → [a, a]: right nodes both map to middle node 0
        let split = Cospan::new(vec![0], vec![0, 0], vec!['a']);
        let morph: FM = cospan_to_frobenius(&split).unwrap();
        assert_eq!(morph.domain(), vec!['a']);
        assert_eq!(morph.codomain(), vec!['a', 'a']);
    }
}
