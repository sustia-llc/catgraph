//! Canonical form for cospans up to apex isomorphism — a decidable equality
//! on parallel cospans.
//!
//! Two parallel cospans `X → a ← Y` and `X → a' ← Y` are equal *as morphisms
//! of the cospan category* iff there is a bijection of apexes `a ≅ a'`
//! commuting with both legs (F&S 2019, §3; the boundary objects `X`, `Y` are
//! fixed, only the apex is quotiented). [`Cospan`] deliberately derives no
//! `PartialEq` (its cached `is_left_id`/`is_right_id` flags can lag the maps
//! they summarise, and raw structural equality is apex-order sensitive), so
//! this module supplies the *semantic* comparison.
//!
//! # Why this is a complete decision for special Frobenius monoids
//!
//! By F&S 2019 **Proposition 3.8**, `(Cospan, ⊕)` is the theory of **special**
//! commutative Frobenius monoids: SCFMs in a symmetric monoidal `(C, ⊗)`
//! correspond one-to-one with strict SM functors `Cospan → C`. So two
//! spider/Frobenius terms are equal under the SCFM axioms iff their images in
//! `Cospan` are isomorphic. [`CospanCanon`] decides that isomorphism, hence
//! decides SCFM-equality — the target of the [#80] complete-functor route in
//! `catgraph-syntax`.
//!
//! **Special, not extra-special.** Cospan keeps *scalars*: the closed bubble
//! `η # ε` is a `0 → 0` cospan whose single apex vertex is hit by neither leg,
//! and it is a genuine non-identity (distinct from `id₀`). The canonical form
//! records apex-only vertices as classes with empty preimages, so `k` bubbles
//! are distinguished from `k-1`. (Corelations — jointly-surjective cospans,
//! [`crate::corel::Corel`] — are the *extra-special* quotient that discards
//! scalars; they are the wrong target for the special theory.)
//!
//! [#80]: https://github.com/sustia-llc/catgraph/issues/80

use std::fmt::Debug;
use std::hash::Hash;

use crate::cospan::Cospan;

/// A canonical, hashable representative of a [`Cospan`]'s apex-isomorphism
/// class.
///
/// Equality on `CospanCanon` decides equality of parallel cospans as cospan
/// morphisms: `a.canonical_form() == b.canonical_form()` iff `a` and `b` are
/// isomorphic (same boundary, apex bijection commuting with both legs).
///
/// # Representation
///
/// Each apex vertex is summarised by its `(label, sorted domain preimage,
/// sorted codomain preimage)`. Because each leg is a *function*, every
/// boundary index lands in exactly one vertex's preimage, so non-bubble
/// vertices carry pairwise-distinct signatures; only apex-only **bubbles**
/// (empty preimages, equal label) can share a signature, and those are
/// exactly the scalars we want to compare by multiplicity. Sorting the vector
/// of signatures canonicalises the (multi)set, making the whole value
/// order-invariant under apex relabelling.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CospanCanon<Lambda> {
    /// Domain (left boundary) size — pins the object `X`.
    dom_len: usize,
    /// Codomain (right boundary) size — pins the object `Y`.
    cod_len: usize,
    /// Sorted multiset of apex-vertex signatures
    /// `(label, sorted dom preimage, sorted cod preimage)`.
    classes: Vec<(Lambda, Vec<usize>, Vec<usize>)>,
}

impl<Lambda> CospanCanon<Lambda> {
    /// The domain (left boundary) size.
    #[must_use]
    pub fn dom_len(&self) -> usize {
        self.dom_len
    }

    /// The codomain (right boundary) size.
    #[must_use]
    pub fn cod_len(&self) -> usize {
        self.cod_len
    }

    /// The number of **scalar** apex vertices (bubbles): apex vertices hit by
    /// neither leg.
    #[must_use]
    pub fn scalar_count(&self) -> usize {
        self.classes
            .iter()
            .filter(|(_, dom, cod)| dom.is_empty() && cod.is_empty())
            .count()
    }

    /// The total number of apex vertices (connected components of the diagram,
    /// including scalars).
    #[must_use]
    pub fn apex_len(&self) -> usize {
        self.classes.len()
    }
}

impl<Lambda> Cospan<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug + Ord + Hash,
{
    /// Canonicalise this cospan up to apex isomorphism.
    ///
    /// See the [module documentation](self) for why the result is a complete
    /// invariant for parallel-cospan equality (and hence for special
    /// commutative Frobenius equality, F&S 2019 Prop 3.8).
    #[must_use]
    pub fn canonical_form(&self) -> CospanCanon<Lambda> {
        let left = self.left_to_middle();
        let right = self.right_to_middle();
        let middle = self.middle();

        // One signature slot per apex vertex, seeded with its label.
        let mut classes: Vec<(Lambda, Vec<usize>, Vec<usize>)> = middle
            .iter()
            .map(|&l| (l, Vec::new(), Vec::new()))
            .collect();

        // Boundary indices are pushed in ascending order, so each preimage
        // vector is already sorted — no per-vector sort needed.
        for (i, &m) in left.iter().enumerate() {
            classes[m].1.push(i);
        }
        for (k, &m) in right.iter().enumerate() {
            classes[m].2.push(k);
        }

        // Canonicalise the multiset: sorting makes the value invariant under
        // any relabelling of apex vertices.
        classes.sort();

        CospanCanon {
            dom_len: left.len(),
            cod_len: right.len(),
            classes,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cospan::Cospan;

    /// The identity on `n` wires and any apex-reordered presentation of it
    /// canonicalise equally; a genuinely different wiring does not.
    #[test]
    fn identity_canonical_is_stable_under_apex_reorder() {
        // id(2): 2 → 2 ← 2, each wire its own apex vertex.
        let id2 = Cospan::<()>::new(vec![0, 1], vec![0, 1], vec![(), ()]);
        // Same morphism, apex vertices swapped: wire 0 → apex 1, wire 1 → apex 0.
        let id2_swapped = Cospan::<()>::new(vec![1, 0], vec![1, 0], vec![(), ()]);
        assert_eq!(id2.canonical_form(), id2_swapped.canonical_form());

        // The braid 2 → 2 (swap) is a different morphism.
        let braid = Cospan::<()>::new(vec![0, 1], vec![1, 0], vec![(), ()]);
        assert_ne!(id2.canonical_form(), braid.canonical_form());
    }

    /// Scalars (bubbles) are kept: `η # ε` (a `0 → 0` cospan with one apex-only
    /// vertex) is distinct from `id₀`, and two bubbles differ from one.
    #[test]
    fn scalars_are_counted_not_collapsed() {
        let id0 = Cospan::<()>::new(vec![], vec![], vec![]);
        let one_bubble = Cospan::<()>::new(vec![], vec![], vec![()]);
        let two_bubbles = Cospan::<()>::new(vec![], vec![], vec![(), ()]);

        assert_eq!(id0.canonical_form().scalar_count(), 0);
        assert_eq!(one_bubble.canonical_form().scalar_count(), 1);
        assert_eq!(two_bubbles.canonical_form().scalar_count(), 2);

        assert_ne!(id0.canonical_form(), one_bubble.canonical_form());
        assert_ne!(one_bubble.canonical_form(), two_bubbles.canonical_form());
    }

    /// Parallel cospans with the same apex partition but different boundary
    /// sizes are distinguished.
    #[test]
    fn boundary_sizes_are_part_of_the_form() {
        // μ-shape: 2 → 1 ← 1 (both inputs and the output share one apex).
        let mu = Cospan::<()>::new(vec![0, 0], vec![0], vec![()]);
        // δ-shape: 1 → 1 ← 2 (transpose boundary).
        let delta = Cospan::<()>::new(vec![0], vec![0, 0], vec![()]);
        assert_ne!(mu.canonical_form(), delta.canonical_form());
        assert_eq!(mu.canonical_form().dom_len(), 2);
        assert_eq!(mu.canonical_form().cod_len(), 1);
    }

    /// Two structurally different presentations of the same "cup" merge — a
    /// single apex joining both boundary wires — canonicalise equally.
    #[test]
    fn same_merge_different_apex_labels_are_equal() {
        // 1 → 1 ← 1 with everything on apex 0.
        let a = Cospan::<()>::new(vec![0], vec![0], vec![()]);
        // 1 → 1 ← 1 built with a spare (unhit) apex vertex present in `b` only:
        // that extra vertex is a bubble, so it is NOT equal to `a`.
        let b = Cospan::<()>::new(vec![0], vec![0], vec![(), ()]);
        assert_ne!(a.canonical_form(), b.canonical_form());
        assert_eq!(b.canonical_form().scalar_count(), 1);
    }
}
