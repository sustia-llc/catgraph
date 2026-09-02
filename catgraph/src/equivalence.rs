//! Equivalence between hypergraph categories and cospan-algebras
//! (Fong-Spivak §4, Theorem 4.13).
//!
//! Implements both directions:
//! - §4.1 (H → A): already done via [`NameAlgebra`](crate::cospan_algebra::NameAlgebra)
//! - §4.2 (A → H): [`CospanAlgebraMorphism`] constructs a hypergraph category from a cospan-algebra

use std::fmt::Debug;
use std::sync::Arc;

use permutations::Permutation;

use crate::{
    category::{Composable, HasIdentity},
    cospan::Cospan,
    cospan_algebra::CospanAlgebra,
    errors::CatgraphError,
    hypergraph_category::HypergraphCategory,
    monoidal::{Monoidal, SymmetricMonoidalMorphism},
};

/// Build the composition cospan `comp^Y_{X,Z}` (Example 3.5, Eq. 14).
///
/// Domain: `X ⊕ Y ⊕ Y ⊕ Z` (the two copies of Y are merged in the middle).
/// Codomain: `X ⊕ Z`.
/// Middle: `X ⊕ Y ⊕ Z`.
///
/// The left leg maps both Y copies to the same middle nodes; X and Z pass through.
/// The right leg maps X and Z through, skipping Y.
///
/// # Arity arithmetic
///
/// Every index sum below is bounded by `middle`, which is built first: a `Vec`
/// holds at most `usize::MAX` elements, so reaching the next line at all proves
/// `m + n + k` fits — and with it `m + n`, `m + k`, and the `m + n .. m + n + k`
/// range bound. The one sum not covered by that argument is the left leg's own
/// length, `m + 2n + k`, which adds a second copy of `Y`; it is spelled against
/// `middle.len()`.
#[allow(clippy::many_single_char_names)]
pub fn comp_cospan<Lambda>(x: &[Lambda], y: &[Lambda], z: &[Lambda]) -> Cospan<Lambda>
where
    Lambda: Eq + Copy + Debug,
{
    let m = x.len();
    let n = y.len();
    let k = z.len();

    // Middle = X ++ Y ++ Z
    let middle: Vec<Lambda> = x.iter().chain(y.iter()).chain(z.iter()).copied().collect();

    // Left map: domain = X ⊕ Y ⊕ Y ⊕ Z → middle. Capacity is `m + 2n + k`, i.e.
    // `middle.len() + n` — a hint, so saturating is exact where it matters; in
    // the (memory-infeasible) saturated case `with_capacity` fails loud rather
    // than handing back a wrapped hint.
    let mut left = Vec::with_capacity(middle.len().saturating_add(n));
    // X part: i → i
    left.extend(0..m);
    // First Y copy: m+j → m+j
    left.extend(m..m + n);
    // Second Y copy: m+n+j → m+j (merge!)
    left.extend(m..m + n);
    // Z part: m+2n+k' → m+n+k'
    left.extend(m + n..m + n + k);

    // Right map: codomain = X ⊕ Z → middle
    let mut right = Vec::with_capacity(m + k);
    // X part: i → i
    right.extend(0..m);
    // Z part: m+k' → m+n+k'
    right.extend(m + n..m + n + k);

    // Correct by construction: every index pushed above is derived from the
    // `m`/`n`/`k` slice lengths that also determine `middle.len() == m + n + k`.
    Cospan::new_unchecked(left, right, middle)
}

// ---------------------------------------------------------------------------
// CospanAlgebraMorphism (§4.2, Lemma 4.8)
// ---------------------------------------------------------------------------

/// A morphism in the hypergraph category `H_A` constructed from a cospan-algebra `A`
/// (Fong-Spivak §4.2, Lemma 4.8).
///
/// A morphism `X → Y` in `H_A` is an element of `A(X ⊕ Y)`.
/// Composition uses the comp cospan (Eq. 32), identity uses the cup cospan (Eq. 33).
pub struct CospanAlgebraMorphism<A, Lambda>
where
    A: CospanAlgebra<Lambda>,
    Lambda: Eq + Copy + Debug,
{
    algebra: Arc<A>,
    element: A::Elem,
    domain: Vec<Lambda>,
    codomain: Vec<Lambda>,
}

/// Hand-written rather than derived: the algebra is held behind an [`Arc`], so
/// cloning a morphism never clones `A` and must not demand `A: Clone`. The
/// derived impl asked for exactly that, which put every zero-sized algebra —
/// [`PartitionAlgebra`](crate::cospan_algebra::PartitionAlgebra) included —
/// outside `Clone` for no reason and kept this carrier out of any test written
/// against a `Clone` bound.
impl<A, Lambda> Clone for CospanAlgebraMorphism<A, Lambda>
where
    A: CospanAlgebra<Lambda>,
    A::Elem: Clone,
    Lambda: Eq + Copy + Debug,
{
    fn clone(&self) -> Self {
        Self {
            algebra: Arc::clone(&self.algebra),
            element: self.element.clone(),
            domain: self.domain.clone(),
            codomain: self.codomain.clone(),
        }
    }
}

impl<A, Lambda> CospanAlgebraMorphism<A, Lambda>
where
    A: CospanAlgebra<Lambda>,
    Lambda: Eq + Copy + Debug,
{
    /// Create a morphism from an algebra, element, domain, and codomain.
    ///
    /// The element should be in `A(domain ⊕ codomain)`.
    pub fn new(
        algebra: Arc<A>,
        element: A::Elem,
        domain: Vec<Lambda>,
        codomain: Vec<Lambda>,
    ) -> Self {
        Self {
            algebra,
            element,
            domain,
            codomain,
        }
    }

    /// Access the underlying element in `A(X ⊕ Y)`.
    #[must_use]
    pub fn element(&self) -> &A::Elem {
        &self.element
    }

    /// Get a reference to the algebra.
    #[must_use]
    pub fn algebra(&self) -> &Arc<A> {
        &self.algebra
    }
}

impl<A, Lambda> CospanAlgebraMorphism<A, Lambda>
where
    A: CospanAlgebra<Lambda>,
    A::Elem: Clone,
    Lambda: Eq + Copy + Debug,
{
    /// Identity morphism on object X in `H_A` (Eq. 33, cup diagram).
    ///
    /// The identity is `A(s)(γ)` where `s: ∅ → X⊕X` is the cup cospan
    /// that pairs each type with itself.
    ///
    /// # Panics
    ///
    /// Panics if the algebra's `map_cospan` fails on the cup cospan from `∅`,
    /// which cannot happen for a well-formed `CospanAlgebra` implementation.
    pub fn identity_in(algebra: Arc<A>, on_this: &[Lambda]) -> Self {
        let n = on_this.len();

        // Cup cospan: ∅ → X⊕X, middle = X, right = [0,..,n-1, 0,..,n-1]
        let right: Vec<usize> = (0..n).chain(0..n).collect();
        let middle: Vec<Lambda> = on_this.to_vec();
        // Correct by construction: `right` is `(0..n) ++ (0..n)` against an
        // n-element apex.
        let cup_s = Cospan::new_unchecked(vec![], right, middle);

        let element = algebra
            .map_cospan(&cup_s, &algebra.unit())
            .expect("cup cospan from ∅ always valid");

        Self {
            algebra,
            element,
            domain: on_this.to_vec(),
            codomain: on_this.to_vec(),
        }
    }

    /// Build a structural morphism from a cospan `s: ∅ → X⊕Y` (Eq. 33).
    fn structural_from_cospan(
        algebra: &Arc<A>,
        s: &Cospan<Lambda>,
        domain: Vec<Lambda>,
        codomain: Vec<Lambda>,
    ) -> Self {
        let element = algebra
            .map_cospan(s, &algebra.unit())
            .expect("structural cospan from ∅ always valid");
        Self {
            algebra: Arc::clone(algebra),
            element,
            domain,
            codomain,
        }
    }
}

impl<A, Lambda> Composable<Vec<Lambda>> for CospanAlgebraMorphism<A, Lambda>
where
    A: CospanAlgebra<Lambda>,
    A::Elem: Clone,
    Lambda: Eq + Copy + Debug,
{
    fn compose(&self, other: &Self) -> Result<Self, CatgraphError> {
        if self.codomain != other.domain {
            return Err(CatgraphError::CompositionLabelMismatch {
                index: 0,
                expected: format!("{:?}", self.codomain),
                actual: format!("{:?}", other.domain),
            });
        }

        // Eq. 32: γ then A(comp)
        let combined = self.algebra.lax_monoidal(&self.element, &other.element);
        let comp = comp_cospan(&self.domain, &self.codomain, &other.codomain);
        let result = self.algebra.map_cospan(&comp, &combined)?;

        Ok(Self {
            algebra: Arc::clone(&self.algebra),
            element: result,
            domain: self.domain.clone(),
            codomain: other.codomain.clone(),
        })
    }

    fn domain(&self) -> Vec<Lambda> {
        self.domain.clone()
    }

    fn codomain(&self) -> Vec<Lambda> {
        self.codomain.clone()
    }
}

impl<A, Lambda> HasIdentity<Vec<Lambda>> for CospanAlgebraMorphism<A, Lambda>
where
    A: CospanAlgebra<Lambda> + Default,
    A::Elem: Clone,
    Lambda: Eq + Copy + Debug,
{
    fn identity(on_this: &Vec<Lambda>) -> Self {
        let algebra = Arc::new(A::default());
        Self::identity_in(algebra, on_this)
    }
}

// ---------------------------------------------------------------------------
// Frobenius generator helpers (§4.2)
// ---------------------------------------------------------------------------

#[allow(clippy::needless_pass_by_value)]
impl<A, Lambda> CospanAlgebraMorphism<A, Lambda>
where
    A: CospanAlgebra<Lambda>,
    A::Elem: Clone,
    Lambda: Eq + Copy + Debug,
{
    /// Unit η in `H_A`: `[] → [z]`.
    ///
    /// Structural morphism from the cospan `∅ → [z]`.
    pub fn unit_in(algebra: Arc<A>, z: Lambda) -> Self {
        // Correct by construction: literal leg into a literal one-vertex apex.
        let s = Cospan::new_unchecked(vec![], vec![0], vec![z]);
        Self::structural_from_cospan(&algebra, &s, vec![], vec![z])
    }

    /// Counit ε in `H_A`: `[z] → []`.
    ///
    /// Structural morphism from the cospan `∅ → [z]` with swapped domain/codomain.
    pub fn counit_in(algebra: Arc<A>, z: Lambda) -> Self {
        // Correct by construction: literal leg into a literal one-vertex apex.
        let s = Cospan::new_unchecked(vec![], vec![0], vec![z]);
        Self::structural_from_cospan(&algebra, &s, vec![z], vec![])
    }

    /// Multiplication μ in `H_A`: `[z, z] → [z]`.
    ///
    /// Structural morphism from the cospan `∅ → [z,z,z]` with all three
    /// right-leg indices mapped to 0 (merge).
    pub fn multiplication_in(algebra: Arc<A>, z: Lambda) -> Self {
        // Correct by construction: literal leg into a literal three-vertex apex.
        let s = Cospan::new_unchecked(vec![], vec![0, 0, 0], vec![z, z, z]);
        Self::structural_from_cospan(&algebra, &s, vec![z, z], vec![z])
    }

    /// Comultiplication δ in `H_A`: `[z] → [z, z]`.
    ///
    /// Structural morphism from the cospan `∅ → [z,z,z]` with all three
    /// right-leg indices mapped to 0 (split).
    pub fn comultiplication_in(algebra: Arc<A>, z: Lambda) -> Self {
        // Correct by construction: literal leg into a literal three-vertex apex.
        let s = Cospan::new_unchecked(vec![], vec![0, 0, 0], vec![z, z, z]);
        Self::structural_from_cospan(&algebra, &s, vec![z], vec![z, z])
    }
}

// ---------------------------------------------------------------------------
// Monoidal impl (§4.2, tensor product in H_A)
// ---------------------------------------------------------------------------

impl<A, Lambda> Monoidal for CospanAlgebraMorphism<A, Lambda>
where
    A: CospanAlgebra<Lambda>,
    A::Elem: Clone,
    Lambda: Eq + Copy + Debug,
{
    /// Tensor product `f ⊗ g` in `H_A`.
    ///
    /// For `f: W→X` (element in `A(W⊕X)`) and `g: Y→Z` (element in `A(Y⊕Z)`):
    /// 1. Use `lax_monoidal` to get element in `A(W⊕X⊕Y⊕Z)`
    /// 2. Apply interchange cospan to reorder to `A(W⊕Y⊕X⊕Z)`
    /// 3. Result has domain `W⊕Y`, codomain `X⊕Z`
    fn monoidal(&mut self, other: Self) {
        let combined = self.algebra.lax_monoidal(&self.element, &other.element);

        let wl = self.domain.len();
        let xl = self.codomain.len();
        let yl = other.domain.len();
        let zl = other.codomain.len();

        // If X or Y is empty, no interchange needed — just concatenate.
        if xl == 0 || yl == 0 {
            self.element = combined;
        } else {
            // Interchange cospan: W⊕X⊕Y⊕Z → W⊕Y⊕X⊕Z
            // Middle = W⊕Y⊕X⊕Z (target ordering), size = wl+yl+xl+zl
            let total = wl + xl + yl + zl;
            let mut left = Vec::with_capacity(total);
            // W part: i → i
            left.extend(0..wl);
            // X part: wl+j → wl+yl+j
            for j in 0..xl {
                left.push(wl + yl + j);
            }
            // Y part: wl+xl+j → wl+j
            for j in 0..yl {
                left.push(wl + j);
            }
            // Z part: wl+xl+yl+j → wl+yl+xl+j
            left.extend(wl + yl + xl..total);

            // Right: identity (codomain = middle)
            let right: Vec<usize> = (0..total).collect();

            // Middle types in target ordering: W⊕Y⊕X⊕Z
            let middle: Vec<Lambda> = self
                .domain
                .iter()
                .chain(other.domain.iter())
                .chain(self.codomain.iter())
                .chain(other.codomain.iter())
                .copied()
                .collect();

            // Correct by construction: every index pushed into `left` is below
            // `total`, `right` is `0..total`, and the apex has exactly `total`
            // vertices (`wl + yl + xl + zl`).
            let interchange = Cospan::new_unchecked(left, right, middle);
            self.element = self
                .algebra
                .map_cospan(&interchange, &combined)
                .expect("interchange cospan is structurally valid");
        }

        self.domain.extend_from_slice(&other.domain);
        self.codomain.extend_from_slice(&other.codomain);
    }
}

// ---------------------------------------------------------------------------
// SymmetricMonoidalMorphism impl
// ---------------------------------------------------------------------------

impl<A, Lambda> SymmetricMonoidalMorphism<Lambda> for CospanAlgebraMorphism<A, Lambda>
where
    A: CospanAlgebra<Lambda> + Default,
    A::Elem: Clone,
    Lambda: Eq + Copy + Debug,
{
    /// Splices the braiding for `p` onto the codomain, or the braiding for
    /// `p⁻¹` onto the domain — see the trait's contract.
    ///
    /// The **element** is the morphism here; `domain`/`codomain` are only the
    /// words naming its interface. So permuting a side means pushing the
    /// element through the relabelling cospan that moves the chosen block's
    /// wire `i` to slot `p.apply(i)` and leaves the other block alone:
    ///
    /// ```text
    /// s : (X ⊕ Y) → (X' ⊕ Y')    apex = X ⊕ Y, left leg = id, right leg = the move
    /// ```
    ///
    /// `A(s)` is then the functorial action of that cospan, exactly as
    /// [`Monoidal::monoidal`] uses an interchange cospan. On an identity this
    /// composes to
    /// [`from_permutation_on_domain`](Self::from_permutation_on_domain)'s own
    /// element.
    fn permute_side(&mut self, p: &Permutation, of_codomain: bool) {
        let m = self.domain.len();
        let n = self.codomain.len();
        if p.len() != if of_codomain { n } else { m } {
            // Defensive no-op: the trait signature is non-fallible, so a
            // length mismatch — a caller bug — leaves `self` unchanged, as on
            // `MatR` and `PropExpr`.
            return;
        }
        let p_inv = p.inv();

        // Apex = the current interface `X ⊕ Y`, left leg the identity, so this
        // cospan is a pure relabelling. Its right leg sends the *new* slot `k`
        // of the permuted block to the apex vertex carrying the wire that used
        // to sit at slot `p.inv().apply(k)`; the untouched block passes through.
        let right: Vec<usize> = if of_codomain {
            (0..m).chain((0..n).map(|k| m + p_inv.apply(k))).collect()
        } else {
            (0..m).map(|i| p_inv.apply(i)).chain(m..m + n).collect()
        };
        let middle: Vec<Lambda> = self
            .domain
            .iter()
            .chain(self.codomain.iter())
            .copied()
            .collect();

        // Correct by construction: the apex has `m + n` vertices, the left leg
        // is `0..m + n`, and every index in `right` is below `m + n`.
        let relabel = Cospan::new_unchecked((0..m + n).collect(), right, middle);
        self.element = self
            .algebra
            .map_cospan(&relabel, &self.element)
            .expect("relabelling cospan over the element's own interface is structurally valid");

        let side = if of_codomain {
            &mut self.codomain
        } else {
            &mut self.domain
        };
        let permuted: Vec<Lambda> = p_inv.permute(side);
        *side = permuted;
    }

    /// `domain() == types`, `codomain()[k] == types[p.inv().apply(k)]`.
    ///
    /// Requires `A: Default` to create an algebra instance.
    ///
    /// The apex is `types` (`n` vertices) and the right leg is
    /// `(0..n) ++ (p.inv().apply(0..n))`, which degenerates to
    /// [`identity_in`](Self::identity_in)'s `(0..n) ++ (0..n)` exactly when `p`
    /// is the identity.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError`] if the permutation size does not match the `types` length.
    fn from_permutation_on_domain(p: Permutation, types: &[Lambda]) -> Result<Self, CatgraphError> {
        if p.len() != types.len() {
            return Err(CatgraphError::CompositionSizeMismatch {
                expected: types.len(),
                actual: p.len(),
            });
        }
        let algebra = Arc::new(A::default());
        let n = types.len();
        let domain = types.to_vec();
        // Hoisted: `inv()` allocates a `Vec` and inverts in O(n), so calling it
        // inside the `map` closure below would do that once per element.
        let p_inv = p.inv();
        let codomain: Vec<Lambda> = p_inv.permute(types);

        // Apex = the domain word. Domain wire `i` sits on apex vertex `i`;
        // codomain wire `k` sits on apex vertex `p.inv().apply(k)`, so the two
        // meet exactly when `k == p.apply(i)`. Labels agree by construction:
        // `apex[p.inv().apply(k)] == types[p.inv().apply(k)] == codomain[k]`.
        let right: Vec<usize> = (0..n).chain((0..n).map(|k| p_inv.apply(k))).collect();

        // Correct by construction: every index in `right` is below `n`, the
        // apex has `n` vertices, and `right.len() == 2n == |domain ⊕ codomain|`.
        let s = Cospan::new_unchecked(vec![], right, domain.clone());
        Ok(Self::structural_from_cospan(&algebra, &s, domain, codomain))
    }

    /// `codomain() == types`, `domain()[i] == types[p.apply(i)]`.
    ///
    /// Mirror of
    /// [`from_permutation_on_domain`](Self::from_permutation_on_domain).
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError`] if the permutation size does not match the `types` length.
    fn from_permutation_on_codomain(
        p: Permutation,
        types: &[Lambda],
    ) -> Result<Self, CatgraphError> {
        if p.len() != types.len() {
            return Err(CatgraphError::CompositionSizeMismatch {
                expected: types.len(),
                actual: p.len(),
            });
        }
        let algebra = Arc::new(A::default());
        let n = types.len();
        let domain: Vec<Lambda> = p.permute(types);
        let codomain = types.to_vec();

        // Apex = the codomain word. Domain wire `i` sits on apex vertex
        // `p.apply(i)`, codomain wire `k` on apex vertex `k`.
        let right: Vec<usize> = (0..n).map(|i| p.apply(i)).chain(0..n).collect();

        // Correct by construction: same argument as `from_permutation_on_domain`.
        let s = Cospan::new_unchecked(vec![], right, codomain.clone());
        Ok(Self::structural_from_cospan(&algebra, &s, domain, codomain))
    }
}

// ---------------------------------------------------------------------------
// HypergraphCategory impl (§4.2, Lemma 4.8)
// ---------------------------------------------------------------------------

impl<A, Lambda> HypergraphCategory<Lambda> for CospanAlgebraMorphism<A, Lambda>
where
    A: CospanAlgebra<Lambda> + Default,
    A::Elem: Clone,
    Lambda: Eq + Copy + Debug,
{
    fn unit(z: Lambda) -> Self {
        Self::unit_in(Arc::new(A::default()), z)
    }

    fn counit(z: Lambda) -> Self {
        Self::counit_in(Arc::new(A::default()), z)
    }

    fn multiplication(z: Lambda) -> Self {
        Self::multiplication_in(Arc::new(A::default()), z)
    }

    fn comultiplication(z: Lambda) -> Self {
        Self::comultiplication_in(Arc::new(A::default()), z)
    }

    fn cup(z: Lambda) -> Result<Self, CatgraphError> {
        Self::unit(z).compose(&Self::comultiplication(z))
    }

    fn cap(z: Lambda) -> Result<Self, CatgraphError> {
        Self::multiplication(z).compose(&Self::counit(z))
    }
}

// ---------------------------------------------------------------------------
// Lemma 4.9: F_α io functor from a morphism α: A → B of cospan-algebras
// ---------------------------------------------------------------------------

/// Lemma 4.9 — induced io hypergraph functor `F_α: H_A → H_B` from a
/// monoidal natural transformation `α: A → B`.
///
/// Given a cospan-algebra morphism `α` (represented here by a closure
/// `Fn(&A::Elem) -> B::Elem`), the induced functor `F_α` acts on morphisms
/// of `H_A` by applying `α` pointwise to the underlying algebra element:
///
/// ```text
/// F_α(f: X → Y in H_A) := α_{X⊕Y}(elem_f) ∈ B(X⊕Y)
/// ```
///
/// which is exactly a morphism `X → Y` in `H_B`. On objects, `F_α` is the
/// identity (both `H_A` and `H_B` share the same object set `List(Λ)`).
///
/// Lemma 4.9's content is that this pointwise construction preserves
/// composition, tensor product, identity, and Frobenius structure whenever `α`
/// is a monoidal natural transformation between the source and target
/// cospan-algebras.
///
/// # Arguments
///
/// - `alpha`: the component function `A::Elem → B::Elem` realising the
///   natural transformation
/// - `beta_algebra`: the target algebra `B` (needed to tag the resulting
///   `CospanAlgebraMorphism`)
/// - `f`: the source morphism in `H_A`
pub fn functor_from_algebra_morphism<A, B, Lambda, Alpha>(
    alpha: &Alpha,
    beta_algebra: Arc<B>,
    f: &CospanAlgebraMorphism<A, Lambda>,
) -> CospanAlgebraMorphism<B, Lambda>
where
    A: CospanAlgebra<Lambda>,
    B: CospanAlgebra<Lambda>,
    B::Elem: Clone,
    Lambda: Eq + Copy + Debug,
    Alpha: Fn(&A::Elem) -> B::Elem,
{
    // Access private struct fields directly — we're in the same module as
    // `CospanAlgebraMorphism`, so no accessor methods are required and no
    // `A::Elem: Clone` bound is imposed on the caller.
    let new_element = alpha(&f.element);
    CospanAlgebraMorphism::new(
        beta_algebra,
        new_element,
        f.domain.clone(),
        f.codomain.clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::category::Composable;

    #[test]
    fn comp_cospan_single_type_each() {
        let comp = comp_cospan(&['a'], &['b'], &['c']);
        assert_eq!(comp.domain(), vec!['a', 'b', 'b', 'c']);
        assert_eq!(comp.codomain(), vec!['a', 'c']);
        assert_eq!(comp.middle(), &['a', 'b', 'c']);
    }

    #[test]
    fn comp_cospan_multi_type() {
        let comp = comp_cospan(&['a', 'b'], &['c'], &['d', 'e']);
        assert_eq!(comp.domain(), vec!['a', 'b', 'c', 'c', 'd', 'e']);
        assert_eq!(comp.codomain(), vec!['a', 'b', 'd', 'e']);
        assert_eq!(comp.middle(), &['a', 'b', 'c', 'd', 'e']);
    }

    #[test]
    fn comp_cospan_empty_y() {
        let comp = comp_cospan(&['a'], &[], &['b']);
        assert_eq!(comp.domain(), vec!['a', 'b']);
        assert_eq!(comp.codomain(), vec!['a', 'b']);
    }

    #[test]
    fn comp_cospan_empty_x_and_z() {
        let comp = comp_cospan::<char>(&[], &['a'], &[]);
        assert_eq!(comp.domain(), vec!['a', 'a']);
        assert!(comp.codomain().is_empty());
        assert_eq!(comp.middle(), &['a']);
    }

    // --- CospanAlgebraMorphism with PartitionAlgebra ---

    use crate::category::HasIdentity;
    use crate::cospan_algebra::PartitionAlgebra;
    use std::sync::Arc;

    type PartMorph = CospanAlgebraMorphism<PartitionAlgebra, char>;

    fn part_algebra() -> Arc<PartitionAlgebra> {
        Arc::new(PartitionAlgebra)
    }

    fn structural_elem(alg: &PartitionAlgebra, cospan_s: &Cospan<char>) -> Cospan<char> {
        use crate::cospan_algebra::CospanAlgebra;
        alg.map_cospan(cospan_s, &alg.unit())
            .expect("structural element")
    }

    #[test]
    fn identity_domain_codomain() {
        let alg = part_algebra();
        let id = PartMorph::identity_in(Arc::clone(&alg), &['a', 'b']);
        assert_eq!(id.domain(), vec!['a', 'b']);
        assert_eq!(id.codomain(), vec!['a', 'b']);
    }

    #[test]
    fn identity_empty_object() {
        let alg = part_algebra();
        let id = PartMorph::identity_in(Arc::clone(&alg), &[]);
        assert!(id.domain().is_empty());
        assert!(id.codomain().is_empty());
    }

    #[test]
    fn compose_types() {
        let alg = part_algebra();
        let s_ab = Cospan::new(vec![], vec![0, 1], vec!['a', 'b']).unwrap();
        let s_bc = Cospan::new(vec![], vec![0, 1], vec!['b', 'c']).unwrap();
        let f = PartMorph::new(
            Arc::clone(&alg),
            structural_elem(&alg, &s_ab),
            vec!['a'],
            vec!['b'],
        );
        let g = PartMorph::new(
            Arc::clone(&alg),
            structural_elem(&alg, &s_bc),
            vec!['b'],
            vec!['c'],
        );
        let fg = f.compose(&g).unwrap();
        assert_eq!(fg.domain(), vec!['a']);
        assert_eq!(fg.codomain(), vec!['c']);
    }

    #[test]
    fn compose_mismatched_fails() {
        let alg = part_algebra();
        let s_ab = Cospan::new(vec![], vec![0, 1], vec!['a', 'b']).unwrap();
        let s_cd = Cospan::new(vec![], vec![0, 1], vec!['c', 'd']).unwrap();
        let f = PartMorph::new(
            Arc::clone(&alg),
            structural_elem(&alg, &s_ab),
            vec!['a'],
            vec!['b'],
        );
        let g = PartMorph::new(
            Arc::clone(&alg),
            structural_elem(&alg, &s_cd),
            vec!['c'],
            vec!['d'],
        );
        assert!(f.compose(&g).is_err());
    }

    #[test]
    fn has_identity_via_default() {
        let id = PartMorph::identity(&vec!['x']);
        assert_eq!(id.domain(), vec!['x']);
        assert_eq!(id.codomain(), vec!['x']);
    }

    // --- Monoidal tests ---

    use crate::monoidal::Monoidal;

    #[test]
    fn monoidal_product_domain_codomain() {
        let alg = part_algebra();
        let mut f = PartMorph::identity_in(Arc::clone(&alg), &['a']);
        let g = PartMorph::identity_in(Arc::clone(&alg), &['b']);
        f.monoidal(g);
        assert_eq!(f.domain(), vec!['a', 'b']);
        assert_eq!(f.codomain(), vec!['a', 'b']);
    }

    // --- Frobenius generator tests ---

    #[test]
    fn frobenius_unit_types() {
        let alg = part_algebra();
        let eta = PartMorph::unit_in(Arc::clone(&alg), 'a');
        assert!(eta.domain().is_empty());
        assert_eq!(eta.codomain(), vec!['a']);
    }

    #[test]
    fn frobenius_counit_types() {
        let alg = part_algebra();
        let eps = PartMorph::counit_in(Arc::clone(&alg), 'a');
        assert_eq!(eps.domain(), vec!['a']);
        assert!(eps.codomain().is_empty());
    }

    #[test]
    fn frobenius_multiplication_types() {
        let alg = part_algebra();
        let mu = PartMorph::multiplication_in(Arc::clone(&alg), 'a');
        assert_eq!(mu.domain(), vec!['a', 'a']);
        assert_eq!(mu.codomain(), vec!['a']);
    }

    #[test]
    fn frobenius_comultiplication_types() {
        let alg = part_algebra();
        let delta = PartMorph::comultiplication_in(Arc::clone(&alg), 'a');
        assert_eq!(delta.domain(), vec!['a']);
        assert_eq!(delta.codomain(), vec!['a', 'a']);
    }

    #[test]
    fn special_frobenius_in_h_part() {
        let alg = part_algebra();
        let delta = PartMorph::comultiplication_in(Arc::clone(&alg), 'a');
        let mu = PartMorph::multiplication_in(Arc::clone(&alg), 'a');
        let result = delta.compose(&mu).unwrap();
        assert_eq!(result.domain(), vec!['a']);
        assert_eq!(result.codomain(), vec!['a']);
    }
}
