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
//! Morphisms of `Free(G)` are arity-tracked expression trees ([`PropExpr`]);
//! the smart constructors on [`Free`] enforce arity at construction —
//! composition requires a matching interface, tensor concatenates. Every
//! morphism of the free prop has a `PropExpr` witness, and equality on
//! [`PropExpr`] is **structural**, so distinct witnesses may denote the same
//! morphism; equivalence modulo the SMC axioms (interchange, unitors, braiding
//! naturality) lives in the [`presentation`] module (Def 5.33).
//!
//! `PropExpr<G>` implements [`Composable<Vec<()>>`], [`Monoidal`],
//! [`HasIdentity<Vec<()>>`] and [`SymmetricMonoidalMorphism<()>`], with the
//! prop object `n ∈ ℕ` encoded as a `Vec<()>` of length `n`.

use std::borrow::Cow;
use std::marker::PhantomData;

use catgraph::category::{Composable, HasIdentity};
use catgraph::errors::CatgraphError;
use catgraph::monoidal::{Monoidal, SymmetricMonoidalMorphism};
use permutations::Permutation;

/// The monochromatic word `•ⁿ` over the one-element palette `Λ = {•}`, which
/// this crate spells `()`.
///
/// Single-sorted [`PropSignature`] impls build their interface words with it:
/// `fn source_word(&self) -> Cow<'_, [()]> { mono_word(self.source()) }`.
#[must_use]
pub fn mono_word(n: usize) -> Cow<'static, [()]> {
    Cow::Owned(vec![(); n])
}

/// A signature `(G, s, t)` for a free prop: every generator declares a source
/// word [`PropSignature::source_word`] and a target word
/// [`PropSignature::target_word`] over a color alphabet `Λ`
/// ([`PropSignature::Color`]); the arities [`PropSignature::source`] /
/// [`PropSignature::target`] are their lengths.
///
/// F&S 2019 Def 3.9 takes the objects of a Λ-colored prop to be the free monoid
/// `List(Λ)` — objectwise-free over the palette — and Thm 3.14 builds the free
/// hypergraph category `Cospan_Λ` over it, so a generator's interface is a
/// *word* over `Λ`, not merely a natural number. Choosing `Color = ()` collapses
/// `List(Λ)` back to `ℕ` and recovers the single-sorted prop of F&S 2018
/// Def 5.25; [`mono_word`] is the helper for that case.
///
/// # Invariant: overridden arities must equal the word lengths
///
/// `source` / `target` are **provided** as `source_word().len()` /
/// `target_word().len()`. An impl may override them — every single-sorted impl
/// does, and then derives its words from them — but an override must stay equal
/// to the corresponding word length.
///
/// # Invariant: `Eq`, `Hash` and `Ord` agree
///
/// A signature carrying an `f64` must implement `Eq`/`Hash` on the same
/// `to_bits` payload with `-0.0` normalized to `0.0`, and `Ord` via
/// `f64::total_cmp` on that payload, as the shipped rigs in [`crate::rig`] do.
pub trait PropSignature: Clone + PartialEq + Eq + std::hash::Hash + std::fmt::Debug + Ord {
    /// The color alphabet `Λ`. `()` recovers the single-sorted prop.
    type Color: Clone + Eq + std::hash::Hash + std::fmt::Debug;

    /// Source word `s(g) ∈ List(Λ)` — the colors of the input ports, in order.
    fn source_word(&self) -> Cow<'_, [Self::Color]>;
    /// Target word `t(g) ∈ List(Λ)` — the colors of the output ports, in order.
    fn target_word(&self) -> Cow<'_, [Self::Color]>;

    /// Source arity `s(g) ∈ ℕ`. Provided as the length of [`source_word`];
    /// an override must agree with it.
    ///
    /// [`source_word`]: PropSignature::source_word
    fn source(&self) -> usize {
        self.source_word().len()
    }
    /// Target arity `t(g) ∈ ℕ`. Provided as the length of [`target_word`];
    /// an override must agree with it.
    ///
    /// [`target_word`]: PropSignature::target_word
    fn target(&self) -> usize {
        self.target_word().len()
    }
}

/// Arity-tracked free-prop expression tree over a signature `G`.
///
/// Every node carries enough information to recover the arity of the subterm
/// rooted at it via [`PropExpr::source`] and [`PropExpr::target`] — `Compose`
/// chains resolve in O(height), while `Tensor` visits both halves, so the worst
/// case is proportional to the subterm's size. Smart constructors on [`Free`]
/// produce only well-formed expressions; raw variant construction is available
/// but callers must uphold the composition-arity invariant themselves.
///
/// In the `source`/`target` walk and in the colored `check`/`infer` interpreters, arity sums
/// that would overflow `usize` saturate to `usize::MAX` — a sentinel no real
/// wire bundle can have, so length checks report
/// [`CatgraphError::CompositionSizeMismatch`] instead of wrapping. Passes that
/// size collections from an arity screen the sentinel out with
/// [`PropExpr::arities_fit`] instead.
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
    ///
    /// A `Braid` or `Tensor` whose halves sum past `usize::MAX` saturates
    /// there rather than overflowing; `usize::MAX` matches no real wire
    /// bundle, so the composition checks report
    /// [`CatgraphError::CompositionSizeMismatch`] instead of wrapping.
    /// Saturation is not injective: two independently overflowing arities
    /// both read `usize::MAX` and compare equal.
    #[must_use]
    pub fn source(&self) -> usize {
        match self {
            PropExpr::Identity(n) => *n,
            PropExpr::Braid(m, n) => m.saturating_add(*n),
            PropExpr::Generator(g) => g.source(),
            PropExpr::Compose(f, _) => f.source(),
            PropExpr::Tensor(f, g) => f.source().saturating_add(g.source()),
        }
    }

    /// Target arity of this morphism.
    ///
    /// Saturates at `usize::MAX` on overflow, exactly as [`PropExpr::source`].
    #[must_use]
    pub fn target(&self) -> usize {
        match self {
            PropExpr::Identity(n) => *n,
            PropExpr::Braid(m, n) => m.saturating_add(*n),
            PropExpr::Generator(g) => g.target(),
            PropExpr::Compose(_, g) => g.target(),
            PropExpr::Tensor(f, g) => f.target().saturating_add(g.target()),
        }
    }

    /// The exact `(source, target)` arity pair of this subterm, or `None` if
    /// **any** `Braid` or `Tensor` width anywhere in it sums past `usize::MAX`.
    ///
    /// The checked companion to [`source`](PropExpr::source) /
    /// [`target`](PropExpr::target), which saturate and so cannot tell an
    /// overflowed width from a genuine `usize::MAX` one.
    ///
    /// Unlike `source`/`target`, this inspects the whole subterm on every
    /// variant, including the half of a `Compose` that does not contribute to
    /// the requested boundary: an overflow buried there is still an overflow.
    #[must_use]
    pub fn checked_arities(&self) -> Option<(usize, usize)> {
        match self {
            PropExpr::Identity(n) => Some((*n, *n)),
            PropExpr::Braid(m, n) => {
                let width = m.checked_add(*n)?;
                Some((width, width))
            }
            PropExpr::Generator(g) => Some((g.source(), g.target())),
            PropExpr::Compose(f, g) => {
                let (f_source, _) = f.checked_arities()?;
                let (_, g_target) = g.checked_arities()?;
                Some((f_source, g_target))
            }
            PropExpr::Tensor(f, g) => {
                let (f_source, f_target) = f.checked_arities()?;
                let (g_source, g_target) = g.checked_arities()?;
                Some((
                    f_source.checked_add(g_source)?,
                    f_target.checked_add(g_target)?,
                ))
            }
        }
    }

    /// Whether every arity sum in this subterm fits in `usize` — i.e. whether
    /// [`source`](PropExpr::source) and [`target`](PropExpr::target) report
    /// exact values rather than the saturated sentinel.
    ///
    /// It screens overflow, not magnitude: a huge arity written *literally*
    /// (e.g. `Identity(usize::MAX)`) involves no sum and passes, yet is just as
    /// infeasible for a consumer that sizes a collection from it — bounding it
    /// stays the caller's obligation.
    #[must_use]
    pub fn arities_fit(&self) -> bool {
        self.checked_arities().is_some()
    }

    /// The braiding network shared by
    /// [`from_permutation_on_domain`](SymmetricMonoidalMorphism::from_permutation_on_domain)
    /// and
    /// [`from_permutation_on_codomain`](SymmetricMonoidalMorphism::from_permutation_on_codomain),
    /// which coincide on this carrier.
    fn braiding_expr(p: &Permutation, arity: usize) -> Result<Self, CatgraphError> {
        let n = arity;
        if p.len() != n {
            return Err(CatgraphError::Composition {
                message: format!(
                    "PropExpr::from_permutation: permutation has len {} but {n} types provided",
                    p.len(),
                ),
            });
        }
        // Input-indexed: the wire at position i is routed to position perm[i].
        let perm: Vec<usize> = (0..n).map(|i| p.apply(i)).collect();
        let mut expr = PropExpr::Identity(n);
        for t in adjacent_swaps(&perm) {
            // A swap at t exchanges positions t and t+1 of an n-element array,
            // so t + 2 <= n and the right-hand identity has width n - t - 2 >= 0.
            let right = n
                .checked_sub(t)
                .and_then(|rest| rest.checked_sub(2))
                .expect("invariant: adjacent_swaps only emits t with t + 1 < n, so t + 2 <= n");
            let layer = Free::<G>::tensor(
                Free::<G>::tensor(Free::identity(t), Free::braid(1, 1)),
                Free::identity(right),
            );
            expr = Free::<G>::compose(expr, layer)?;
        }
        Ok(expr)
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
/// Callers consuming the sequence forward rebuild the permutation as a braid
/// network; callers whose perms are output-indexed consume it reversed.
///
/// `O(k²)` swaps for `k = perm.len()`; the empty vector is returned when `perm`
/// is already sorted (including `k ≤ 1`). Applying the returned swaps to `perm`
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
    /// The pure-braiding morphism `n → n` realizing the permutation `p`, as a
    /// network of adjacent transpositions.
    ///
    /// **Convention:** a wire entering at position `i` exits at position
    /// `perm[i] = p.apply(i)` — the perm is *input-indexed*. Under the functor
    /// `S` of F&S Thm 5.53 ([`crate::sfg_to_mat`]) the result is therefore the
    /// permutation matrix with `entries[i][p.apply(i)] = 1`, i.e. exactly
    /// [`MatR::permutation_matrix`](crate::mat::MatR::permutation_matrix). This
    /// is the whole workspace's convention: `Cospan`, `Span`, `Corel` and
    /// `FrobeniusMorphism` build the same wiring.
    ///
    /// **Construction:** `perm` is bubble-sorted into ascending order, and each
    /// adjacent swap at position `t` becomes one braid layer
    /// `Identity(t) ⊗ Braid(1, 1) ⊗ Identity(n-t-2)`, composed in the swap
    /// order (input side first). `O(n²)` layers for `n` wires; the identity
    /// permutation (including `n ≤ 1`) sorts with no swaps and yields
    /// `Identity(n)`.
    ///
    /// Those layers are `O(n²)` *deep*, not merely `O(n²)` many: the result is a
    /// left-nested `Compose` spine that every consumer walks recursively, so a
    /// few hundred wires is enough to overflow a default stack. Bounding arity
    /// magnitude remains the caller's obligation.
    ///
    /// `types` contributes only its length: objects of a prop are natural
    /// numbers (encoded `Vec<()>`), so there is no color to place on one side
    /// or the other, and
    /// [`from_permutation_on_codomain`](SymmetricMonoidalMorphism::from_permutation_on_codomain)
    /// returns the same morphism.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] if `p.len() != types.len()`.
    fn from_permutation_on_domain(p: Permutation, types: &[()]) -> Result<Self, CatgraphError> {
        Self::braiding_expr(&p, types.len())
    }

    /// The same morphism as
    /// [`from_permutation_on_domain`](SymmetricMonoidalMorphism::from_permutation_on_domain)
    /// — see there for why the two coincide on a single-sorted carrier, and for
    /// the S-functor square.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] if `p.len() != types.len()`.
    fn from_permutation_on_codomain(p: Permutation, types: &[()]) -> Result<Self, CatgraphError> {
        Self::braiding_expr(&p, types.len())
    }

    /// Permute the wires on one side of `self`, in place: postcompose a
    /// braiding onto the codomain (`of_codomain = true`) or precompose one onto
    /// the domain (`of_codomain = false`).
    ///
    /// Source and target *counts* are invariant either way — the spliced
    /// braiding is an endomorphism of the permuted side's arity.
    ///
    /// # Which side gets `p` and which gets `p.inv()`
    ///
    /// The semantics of record are [`MatR`](crate::mat::MatR)'s `permute_side`:
    /// *right-multiply by `P` to permute columns (codomain side); left-multiply
    /// by `Pᵀ` to permute rows (domain side)*, with
    /// `P = MatR::permutation_matrix(p)` — exactly what
    /// [`from_permutation_on_domain`](SymmetricMonoidalMorphism::from_permutation_on_domain)
    /// realizes under `S`. That reads:
    ///
    /// - **codomain:** `self ; from_permutation_on_domain(p)` (`self · P`)
    /// - **domain:** `from_permutation_on_domain(p.inv()) ; self` (`Pᵀ · self`)
    ///
    /// The domain side takes `p.inv()` because `Pᵀ = P⁻¹` for a permutation
    /// matrix. **The two sides are therefore not symmetric**, and passing `p`
    /// on both would silently transpose one of them.
    ///
    /// # Cost
    ///
    /// One
    /// [`from_permutation_on_domain`](SymmetricMonoidalMorphism::from_permutation_on_domain)
    /// network per call: `O(n²)` braid layers for `n` wires on the permuted
    /// side, forming a `Compose` spine `O(n²)` **deep** that every consumer
    /// walks recursively.
    ///
    /// # Length mismatch
    ///
    /// The trait signature is non-fallible, so a `p` whose length does not
    /// match the permuted side's arity — a caller bug — leaves `self`
    /// **unchanged** rather than panicking.
    fn permute_side(&mut self, p: &Permutation, of_codomain: bool) {
        let n = if of_codomain {
            self.target()
        } else {
            self.source()
        };
        if p.len() != n {
            // A length mismatch is a caller bug: leave `self` unchanged.
            return;
        }
        // Codomain side postcomposes `P`; domain side precomposes `Pᵀ = P⁻¹`.
        let perm = if of_codomain { p.clone() } else { p.inv() };
        let types = as_object(n);
        let braid: PropExpr<G> =
            <Self as SymmetricMonoidalMorphism<()>>::from_permutation_on_domain(perm, &types)
                .expect(
                    "invariant: from_permutation_on_domain's only error is a \
                     p.len() != types.len() mismatch, and p.len() == n == types.len() was just \
                     checked (p.inv() preserves length)",
                );
        let old = std::mem::replace(self, PropExpr::Identity(0));
        *self = if of_codomain {
            PropExpr::Compose(Box::new(old), Box::new(braid))
        } else {
            PropExpr::Compose(Box::new(braid), Box::new(old))
        };
    }
}

pub mod colored;
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
