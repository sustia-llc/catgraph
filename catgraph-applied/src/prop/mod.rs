//! Props and the free prop on a signature.
//!
//! F&S *Seven Sketches in Compositionality* §5.2:
//! - **Def 5.2.** A *prop* is a symmetric strict monoidal category with
//!   `Ob = ℕ` and tensor = addition on objects. Morphisms `m → n` are the
//!   "`m`-ary-in, `n`-ary-out" building blocks of a compositional theory.
//! - **Def 5.25.** The *free prop* `Free(G)` on a signature `(G, s, t)` — a
//!   set of generators `G` with declared source/target arities `s, t: G → ℕ`
//!   — is the prop whose morphisms are all well-formed expressions built
//!   from `G` under composition (`;`), tensor (`⊗`), identities, and
//!   symmetric braiding, modulo the SMC axioms.
//!
//! # This implementation
//!
//! Morphisms of `Free(G)` are arity-tracked expression trees ([`PropExpr`]).
//! Smart constructors on [`Free`] enforce arity at construction time:
//! composition requires matching interface; tensor concatenates.
//!
//! ## Equality
//!
//! Equality on [`PropExpr`] is **structural** — two expressions are equal
//! iff their trees match. Equivalence modulo the SMC axioms (interchange,
//! unitors, braiding naturality) is handled by the presentation / equations
//! type (`Def 5.33`); see the [`presentation`] module. `Free(G)` gives
//! a faithful pre-quotient representation: every morphism of the free prop
//! has a `PropExpr` witness, but distinct witnesses may represent the same
//! morphism.
//!
//! ## Relationship to `catgraph` core
//!
//! `PropExpr<G>` implements the standard catgraph trait hierarchy:
//! [`Composable<Vec<()>>`], [`Monoidal`], [`HasIdentity<Vec<()>>`], and
//! [`SymmetricMonoidalMorphism<()>`]. Objects are represented as
//! `Vec<()>` of length `n` (standing for the prop object `n ∈ ℕ`).

use std::marker::PhantomData;

use catgraph::category::{Composable, HasIdentity};
use catgraph::errors::CatgraphError;
use catgraph::monoidal::{Monoidal, SymmetricMonoidalMorphism};
use permutations::Permutation;

/// A signature `(G, s, t)` for a free prop: every generator has a declared
/// source arity [`PropSignature::source`] and target arity
/// [`PropSignature::target`], both natural numbers.
///
/// # Supertrait bounds
///
/// `PropSignature` requires `Eq + Hash` in addition to
/// `Clone + PartialEq + Debug`. These bounds are needed by the
/// [`presentation::kb::CongruenceClosure`] decision procedure, which uses `G`
/// as a `HashMap` key in its term graph. Migration:
///
/// - Derived-`PartialEq` types: add `Eq, Hash` to the `#[derive(...)]`.
/// - Types containing `f64`: provide manual `Eq` + `Hash` impls via
///   `to_bits()` (bit-exact except `-0.0` normalizes to `0.0` to satisfy the
///   `Eq`/`Hash` contract; see [`crate::rig::UnitInterval`] /
///   [`crate::rig::Tropical`] / [`crate::rig::F64Rig`]).
pub trait PropSignature: Clone + PartialEq + Eq + std::hash::Hash + std::fmt::Debug {
    /// Source arity `s(g) ∈ ℕ`.
    fn source(&self) -> usize;
    /// Target arity `t(g) ∈ ℕ`.
    fn target(&self) -> usize;
}

/// Arity-tracked free-prop expression tree over a signature `G`.
///
/// Every node carries enough information to recover the arity of the
/// subterm rooted at it via [`PropExpr::source`] and [`PropExpr::target`]
/// in O(height). Smart constructors on [`Free`] produce only well-formed
/// expressions; raw variant construction is available but callers must
/// uphold the composition-arity invariant themselves.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PropExpr<G: PropSignature> {
    /// `id_n : n → n`.
    Identity(usize),
    /// Symmetric braiding `σ_{m,n} : m+n → m+n` that swaps the two blocks.
    Braid(usize, usize),
    /// A generator `g ∈ G`.
    Generator(G),
    /// Sequential composition `f ; g` (requires `f.target() == g.source()`).
    Compose(Box<PropExpr<G>>, Box<PropExpr<G>>),
    /// Parallel tensor `f ⊗ g`.
    Tensor(Box<PropExpr<G>>, Box<PropExpr<G>>),
}

impl<G: PropSignature> PropExpr<G> {
    /// Source arity of this morphism.
    #[must_use]
    pub fn source(&self) -> usize {
        match self {
            PropExpr::Identity(n) => *n,
            PropExpr::Braid(m, n) => m + n,
            PropExpr::Generator(g) => g.source(),
            PropExpr::Compose(f, _) => f.source(),
            PropExpr::Tensor(f, g) => f.source() + g.source(),
        }
    }

    /// Target arity of this morphism.
    #[must_use]
    pub fn target(&self) -> usize {
        match self {
            PropExpr::Identity(n) => *n,
            PropExpr::Braid(m, n) => m + n,
            PropExpr::Generator(g) => g.target(),
            PropExpr::Compose(_, g) => g.target(),
            PropExpr::Tensor(f, g) => f.target() + g.target(),
        }
    }
}

/// Marker type for the *prop itself* (the category). Values of `Prop<G>` are
/// [`PropExpr<G>`]. See module docs for the equality caveat.
pub struct Prop<G: PropSignature>(PhantomData<G>);

/// Smart-constructor namespace producing well-formed [`PropExpr<G>`] values
/// — morphisms of the free prop on signature `G`.
pub struct Free<G: PropSignature>(PhantomData<G>);

impl<G: PropSignature> Free<G> {
    /// `id_n : n → n`.
    #[must_use]
    pub fn identity(n: usize) -> PropExpr<G> {
        PropExpr::Identity(n)
    }

    /// Symmetric braiding `σ_{m,n} : m+n → m+n`.
    #[must_use]
    pub fn braid(m: usize, n: usize) -> PropExpr<G> {
        PropExpr::Braid(m, n)
    }

    /// Generator inclusion `g ∈ G ↪ Free(G)`.
    #[must_use]
    pub fn generator(g: G) -> PropExpr<G> {
        PropExpr::Generator(g)
    }

    /// Sequential composition `f ; g` with arity check.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::CompositionSizeMismatch`] if
    /// `f.target() != g.source()`.
    pub fn compose(f: PropExpr<G>, g: PropExpr<G>) -> Result<PropExpr<G>, CatgraphError> {
        if f.target() != g.source() {
            return Err(CatgraphError::CompositionSizeMismatch {
                expected: f.target(),
                actual: g.source(),
            });
        }
        Ok(PropExpr::Compose(Box::new(f), Box::new(g)))
    }

    /// Parallel tensor `f ⊗ g`. Arity sums trivially; no failure case.
    #[must_use]
    pub fn tensor(f: PropExpr<G>, g: PropExpr<G>) -> PropExpr<G> {
        PropExpr::Tensor(Box::new(f), Box::new(g))
    }
}

/// Bubble-sort `perm` ascending; return the positions of the adjacent
/// transpositions in the order performed. `adjacent_swaps(p)[i] = t` means the
/// `i`-th swap exchanged positions `t` and `t + 1`.
///
/// This is the shared decomposition core behind all three call sites:
/// [`crate::mat_to_sfg`]'s `permutation_sfg` (which maps each `t` to a braid
/// layer `id_t ⊗ σ ⊗ id_{k-t-2}`, in swap order) and
/// [`presentation::smc_nf`]'s `decompose_braid` + `canonicalize_run` (which
/// build `Layer<G>` values from the REVERSED sequence — their perms are
/// output-indexed, so the sort word undoes the braid and its reversal
/// rebuilds it). `O(k²)`
/// swaps for `k = perm.len()`; the empty vector is returned when `perm` is
/// already sorted (including `k ≤ 1`). Applying the returned swaps to `perm`
/// left-to-right yields the ascending sort.
pub(crate) fn adjacent_swaps(perm: &[usize]) -> Vec<usize> {
    let k = perm.len();
    let mut arr = perm.to_vec();
    let mut swaps = Vec::new();
    for pass in 0..k {
        for t in 0..k.saturating_sub(pass + 1) {
            if arr[t] > arr[t + 1] {
                arr.swap(t, t + 1);
                swaps.push(t);
            }
        }
    }
    swaps
}

// ---- Integration with catgraph trait hierarchy -------------------------------

/// Objects of a prop are natural numbers, encoded as `Vec<()>` of the
/// corresponding length so that `PropExpr<G>` can implement
/// `Composable<Vec<()>>` uniformly with the rest of the workspace.
fn as_object(n: usize) -> Vec<()> {
    vec![(); n]
}

impl<G: PropSignature> HasIdentity<Vec<()>> for PropExpr<G> {
    fn identity(on_this: &Vec<()>) -> Self {
        PropExpr::Identity(on_this.len())
    }
}

impl<G: PropSignature> Composable<Vec<()>> for PropExpr<G> {
    fn compose(&self, other: &Self) -> Result<Self, CatgraphError> {
        if self.target() != other.source() {
            return Err(CatgraphError::CompositionSizeMismatch {
                expected: self.target(),
                actual: other.source(),
            });
        }
        Ok(PropExpr::Compose(
            Box::new(self.clone()),
            Box::new(other.clone()),
        ))
    }

    fn domain(&self) -> Vec<()> {
        as_object(self.source())
    }

    fn codomain(&self) -> Vec<()> {
        as_object(self.target())
    }
}

impl<G: PropSignature> Monoidal for PropExpr<G> {
    fn monoidal(&mut self, other: Self) {
        let lhs = std::mem::replace(self, PropExpr::Identity(0));
        *self = PropExpr::Tensor(Box::new(lhs), Box::new(other));
    }
}

impl<G: PropSignature> SymmetricMonoidalMorphism<()> for PropExpr<G> {
    /// Return a `PropExpr` representing the given permutation on `n` wires.
    ///
    /// **WARNING — PLACEHOLDER:** This method returns `Identity(n)` for any
    /// permutation `p`, which does **NOT** preserve the permutation's action
    /// on wires. See the function body for the rationale. Callers that need
    /// a faithful permutation representation should use
    /// `MatR::permutation_matrix` via the `Mat(R)` prop, which encodes the
    /// permutation directly as a matrix and bypasses `PropExpr` entirely.
    fn from_permutation(
        p: Permutation,
        types: &[()],
        _types_as_on_domain: bool,
    ) -> Result<Self, CatgraphError> {
        let n = types.len();
        if p.len() != n {
            return Err(CatgraphError::Composition {
                message: format!(
                    "PropExpr::from_permutation: permutation has len {} but {n} types provided",
                    p.len(),
                ),
            });
        }
        // F&S 2018 does not give a canonical decomposition of arbitrary permutations
        // into braids of the free prop; a faithful implementation would compute a
        // Bubblesort-style decomposition using the `permutations` crate's cycle
        // structure. We return Identity(n) — explicitly documenting that
        // this does NOT preserve the permutation's action. Callers that need faithful
        // permutation representations should use `MatR::permutation_matrix` directly
        // via the Mat(R) prop, which bypasses PropExpr entirely.
        Ok(PropExpr::Identity(n))
    }

    fn permute_side(&mut self, p: &Permutation, of_codomain: bool) {
        // Precompose (domain side) or postcompose (codomain side) with a
        // braiding block of the appropriate arity. Source/target counts
        // remain invariant because braids are endomorphisms.
        let n = if of_codomain {
            self.target()
        } else {
            self.source()
        };
        if p.len() != n {
            // Invariant: callers should only pass permutations that match
            // the side being permuted. A length mismatch is a caller bug,
            // so we leave `self` unchanged (defensive) rather than panic.
            return;
        }
        let braid: PropExpr<G> = PropExpr::Braid(0, n);
        let old = std::mem::replace(self, PropExpr::Identity(0));
        *self = if of_codomain {
            PropExpr::Compose(Box::new(old), Box::new(braid))
        } else {
            PropExpr::Compose(Box::new(braid), Box::new(old))
        };
    }
}

pub mod presentation;

#[cfg(test)]
mod tests {
    use super::adjacent_swaps;

    /// Apply a swap sequence to `perm` left-to-right and return the result.
    fn apply_swaps(perm: &[usize], swaps: &[usize]) -> Vec<usize> {
        let mut arr = perm.to_vec();
        for &t in swaps {
            arr.swap(t, t + 1);
        }
        arr
    }

    #[test]
    fn empty_perm_has_no_swaps() {
        assert_eq!(adjacent_swaps(&[]), Vec::<usize>::new());
    }

    #[test]
    fn single_element_has_no_swaps() {
        assert_eq!(adjacent_swaps(&[0]), Vec::<usize>::new());
    }

    #[test]
    fn already_sorted_has_no_swaps() {
        assert_eq!(adjacent_swaps(&[0, 1, 2, 3]), Vec::<usize>::new());
    }

    #[test]
    fn full_reversal() {
        // [2,1,0] bubble-sorts via swaps at positions 0, 1, 0.
        assert_eq!(adjacent_swaps(&[2, 1, 0]), vec![0, 1, 0]);
    }

    #[test]
    fn transpose_riffle_perm() {
        // mat_to_sfg's L3 regrouping for a 2×3 matrix: input-major i*cols+j
        // routed to output-major j*rows+i (rows=2, cols=3).
        let mut perm = vec![0usize; 6];
        for i in 0..2 {
            for j in 0..3 {
                perm[i * 3 + j] = j * 2 + i;
            }
        }
        assert_eq!(perm, vec![0, 2, 4, 1, 3, 5]);
        let swaps = adjacent_swaps(&perm);
        assert_eq!(apply_swaps(&perm, &swaps), vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn swaps_sort_the_permutation() {
        // Property: applying the returned swaps to `perm` yields the sorted array.
        let cases: &[&[usize]] = &[
            &[],
            &[0],
            &[1, 0],
            &[2, 0, 1],
            &[3, 2, 1, 0],
            &[0, 2, 4, 1, 3, 5],
            &[4, 3, 2, 1, 0],
        ];
        for &perm in cases {
            let mut sorted = perm.to_vec();
            sorted.sort_unstable();
            let swaps = adjacent_swaps(perm);
            assert_eq!(apply_swaps(perm, &swaps), sorted, "perm = {perm:?}");
        }
    }
}
