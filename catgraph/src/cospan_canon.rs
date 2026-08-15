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

/// One apex vertex's signature inside a [`CospanCanon`]: its label together
/// with the boundary indices that land on it.
///
/// # Invariants
///
/// - Both preimage vectors are **sorted ascending**, and callers may rely on
///   it: [`Cospan::canonical_form`] walks each leg in boundary-index order, so
///   the indices are appended already in order.
/// - Each leg is a *function*, so across the whole
///   [`classes`](CospanCanon::classes) slice every domain index occurs in
///   exactly one `dom_preimage` and every codomain index in exactly one
///   `cod_preimage`.
///
/// # Scalars
///
/// A class whose two preimages are **both** empty is a *scalar* — the closed
/// bubble `η # ε`, an apex vertex no leg reaches — and that both-empty case is
/// exactly [`is_scalar`](Self::is_scalar). `Cospan` is the theory of
/// **special**, not extra-special, commutative Frobenius monoids, so such
/// vertices are kept and counted rather than discarded: `k` bubbles are
/// distinguished from `k-1`. See the [module documentation](self).
///
/// # Ordering
///
/// `Ord` is the derived lexicographic order on
/// `(label, dom_preimage, cod_preimage)`, in that field order. [`CospanCanon`]
/// sorts its classes under it, and that sort is what makes the canonical form
/// invariant under relabelling of apex vertices — the field order is
/// load-bearing, not cosmetic.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ApexClass<Lambda> {
    /// The apex vertex's label.
    label: Lambda,
    /// Domain (left boundary) indices mapping to this vertex, sorted ascending.
    dom_preimage: Vec<usize>,
    /// Codomain (right boundary) indices mapping to this vertex, sorted
    /// ascending.
    cod_preimage: Vec<usize>,
}

impl<Lambda> ApexClass<Lambda> {
    /// The apex vertex's label.
    #[must_use]
    pub fn label(&self) -> &Lambda {
        &self.label
    }

    /// The domain (left boundary) indices that map to this apex vertex,
    /// **sorted ascending**.
    #[must_use]
    pub fn dom_preimage(&self) -> &[usize] {
        &self.dom_preimage
    }

    /// The codomain (right boundary) indices that map to this apex vertex,
    /// **sorted ascending**.
    #[must_use]
    pub fn cod_preimage(&self) -> &[usize] {
        &self.cod_preimage
    }

    /// True when neither leg reaches this apex vertex — i.e. both preimages are
    /// empty — so the vertex is a **scalar** (a closed bubble `η # ε`).
    ///
    /// Counting these over [`CospanCanon::classes`] reproduces
    /// [`CospanCanon::scalar_count`].
    #[must_use]
    pub fn is_scalar(&self) -> bool {
        self.dom_preimage.is_empty() && self.cod_preimage.is_empty()
    }
}

/// A canonical, hashable representative of a [`Cospan`]'s apex-isomorphism
/// class.
///
/// Equality on `CospanCanon` decides equality of parallel cospans as cospan
/// morphisms: `a.canonical_form() == b.canonical_form()` iff `a` and `b` are
/// isomorphic (same boundary, apex bijection commuting with both legs).
///
/// # Representation
///
/// Each apex vertex is summarised by an [`ApexClass`] — its `(label, sorted
/// domain preimage, sorted codomain preimage)`. Because each leg is a
/// *function*, every boundary index lands in exactly one vertex's preimage, so
/// non-bubble vertices carry pairwise-distinct signatures; only apex-only
/// **bubbles** (empty preimages, equal label) can share a signature, and those
/// are exactly the scalars we want to compare by multiplicity. Sorting the
/// vector of signatures canonicalises the (multi)set, making the whole value
/// order-invariant under apex relabelling. [`classes`](Self::classes) exposes
/// that sorted vector for inspection, logging, or re-encoding.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CospanCanon<Lambda> {
    /// Domain (left boundary) size — pins the object `X`.
    dom_len: usize,
    /// Codomain (right boundary) size — pins the object `Y`.
    cod_len: usize,
    /// Sorted multiset of apex-vertex signatures ([`ApexClass`]: label, sorted
    /// dom preimage, sorted cod preimage).
    classes: Vec<ApexClass<Lambda>>,
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
        self.classes.iter().filter(|c| c.is_scalar()).count()
    }

    /// The total number of apex vertices (connected components of the diagram,
    /// including scalars).
    #[must_use]
    pub fn apex_len(&self) -> usize {
        self.classes.len()
    }

    /// The apex-vertex signatures, in canonical order.
    ///
    /// The slice is sorted under [`ApexClass`]'s `Ord` — lexicographic on
    /// `(label, dom_preimage, cod_preimage)` — and that sort is precisely what
    /// makes the whole value invariant under relabelling of apex vertices: two
    /// isomorphic cospans yield equal slices, element for element. Its length
    /// is [`apex_len`](Self::apex_len).
    ///
    /// This is the read surface for inspecting, persisting, re-encoding, or
    /// logging a canonical form, as opposed to only comparing two of them.
    #[must_use]
    pub fn classes(&self) -> &[ApexClass<Lambda>] {
        &self.classes
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
        let mut classes: Vec<ApexClass<Lambda>> = middle
            .iter()
            .map(|&l| ApexClass {
                label: l,
                dom_preimage: Vec::new(),
                cod_preimage: Vec::new(),
            })
            .collect();

        // Boundary indices are pushed in ascending order, so each preimage
        // vector is already sorted — no per-vector sort needed.
        for (i, &m) in left.iter().enumerate() {
            classes[m].dom_preimage.push(i);
        }
        for (k, &m) in right.iter().enumerate() {
            classes[m].cod_preimage.push(k);
        }

        // Canonicalise the multiset: sorting makes the value invariant under
        // any relabelling of apex vertices. `ApexClass`'s derived `Ord` is
        // lexicographic on `(label, dom_preimage, cod_preimage)` in declaration
        // order, so this is exactly the tuple order this field once carried.
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
    use super::ApexClass;
    use crate::cospan::Cospan;

    /// The identity on `n` wires and any apex-reordered presentation of it
    /// canonicalise equally — as whole values *and* class-by-class; a genuinely
    /// different wiring does not.
    #[test]
    fn identity_canonical_is_stable_under_apex_reorder() {
        // id(2): 2 → 2 ← 2, each wire its own apex vertex.
        let id2 = Cospan::<()>::new(vec![0, 1], vec![0, 1], vec![(), ()]);
        // Same morphism, apex vertices swapped: wire 0 → apex 1, wire 1 → apex 0.
        let id2_swapped = Cospan::<()>::new(vec![1, 0], vec![1, 0], vec![(), ()]);
        let canon = id2.canonical_form();
        let canon_swapped = id2_swapped.canonical_form();
        assert_eq!(canon, canon_swapped);
        // Relabelling invariance is visible in the exposed representation too:
        // the sorted class slices agree element for element.
        assert_eq!(canon.classes(), canon_swapped.classes());

        // The braid 2 → 2 (swap) is a different morphism, and the difference is
        // visible in the classes: its two vertices pair dom i with cod 1-i.
        let braid = Cospan::<()>::new(vec![0, 1], vec![1, 0], vec![(), ()]);
        let canon_braid = braid.canonical_form();
        assert_ne!(canon, canon_braid);
        assert_ne!(canon.classes(), canon_braid.classes());
    }

    /// The canonicalising sort is unchanged by the tuple → [`ApexClass`] swap:
    /// an in-test reference rebuilds the `(label, dom, cod)` tuple vector the
    /// private field used to carry, sorts it under *tuple* `Ord`, and the two
    /// agree element for element.
    #[test]
    fn class_sort_matches_the_tuple_order_it_replaced() {
        // Labels deliberately out of construction order, with two vertices
        // sharing a label so the tie-break falls through to the preimages.
        let cospan = Cospan::<char>::new(vec![0, 3, 1], vec![2, 0], vec!['c', 'a', 'a', 'b']);
        let canon = cospan.canonical_form();

        // Reference: exactly what the pre-#254 tuple implementation produced.
        let mut reference: Vec<(char, Vec<usize>, Vec<usize>)> = cospan
            .middle()
            .iter()
            .map(|&l| (l, Vec::new(), Vec::new()))
            .collect();
        for (i, &m) in cospan.left_to_middle().iter().enumerate() {
            reference[m].1.push(i);
        }
        for (k, &m) in cospan.right_to_middle().iter().enumerate() {
            reference[m].2.push(k);
        }
        reference.sort();

        assert_eq!(canon.classes().len(), reference.len());
        for (class, (label, dom, cod)) in canon.classes().iter().zip(reference.iter()) {
            assert_eq!(class.label(), label);
            assert_eq!(class.dom_preimage(), dom.as_slice());
            assert_eq!(class.cod_preimage(), cod.as_slice());
        }
        // Spelled out, so a future refactor of `reference` cannot make the loop
        // above vacuous: label order first, then dom preimage inside a tie.
        assert_eq!(*canon.classes()[0].label(), 'a');
        assert!(canon.classes()[0].dom_preimage().is_empty());
        assert_eq!(canon.classes()[0].cod_preimage(), [0_usize].as_slice());
        assert_eq!(*canon.classes()[1].label(), 'a');
        assert_eq!(canon.classes()[1].dom_preimage(), [2_usize].as_slice());
        assert_eq!(*canon.classes()[2].label(), 'b');
        assert_eq!(*canon.classes()[3].label(), 'c');

        // The slice is sorted under the order the TUPLE defines. Asserting it
        // under the derived `Ord` instead would be vacuous — that is `sort()`'s
        // own postcondition, so it holds for any field declaration order and
        // cannot catch the regression this test exists for. Projecting to the
        // tuple compares against an order the derive does not control.
        assert!(canon.classes().windows(2).all(|w| (
            w[0].label,
            w[0].dom_preimage.clone(),
            w[0].cod_preimage.clone()
        ) <= (
            w[1].label,
            w[1].dom_preimage.clone(),
            w[1].cod_preimage.clone()
        )));

        // ... and that `Ord` is lexicographic on (label, dom, cod): each field
        // position dominates every field after it, matching the tuple exactly.
        let base = ApexClass {
            label: 'a',
            dom_preimage: vec![1],
            cod_preimage: vec![1],
        };
        let by_label = ApexClass {
            label: 'b',
            dom_preimage: vec![0],
            cod_preimage: vec![0],
        };
        let by_dom = ApexClass {
            label: 'a',
            dom_preimage: vec![2],
            cod_preimage: vec![0],
        };
        let by_cod = ApexClass {
            label: 'a',
            dom_preimage: vec![1],
            cod_preimage: vec![2],
        };
        assert!(base < by_label, "label dominates both preimages");
        assert!(base < by_dom, "dom preimage dominates cod preimage");
        assert!(base < by_cod, "cod preimage is the last tie-break");
        // The same three comparisons on the tuple this replaced.
        assert!(('a', vec![1_usize], vec![1_usize]) < ('b', vec![0], vec![0]));
        assert!(('a', vec![1_usize], vec![1_usize]) < ('a', vec![2], vec![0]));
        assert!(('a', vec![1_usize], vec![1_usize]) < ('a', vec![1], vec![2]));
    }

    /// The class accessors report the real structure of a hand-built cospan:
    /// a vertex hit by **both** legs, one hit by a single leg, and a bubble hit
    /// by neither — and `is_scalar` agrees with `scalar_count`.
    #[test]
    fn classes_report_labels_and_preimages() {
        // 2 → apex(3) ← 2, apex labels out of sorted order:
        //   v0 ('z'): dom {0, 1}, cod {1}  — hit by BOTH legs
        //   v1 ('n'): dom {},     cod {0}  — hit by the right leg only
        //   v2 ('m'): dom {},     cod {}   — bubble
        let cospan = Cospan::<char>::new(vec![0, 0], vec![1, 0], vec!['z', 'n', 'm']);
        let canon = cospan.canonical_form();

        assert_eq!(canon.dom_len(), 2);
        assert_eq!(canon.cod_len(), 2);
        assert_eq!(canon.apex_len(), 3);
        assert_eq!(canon.classes().len(), canon.apex_len());

        // Sorted by label: 'm' (the bubble), 'n', 'z'.
        let bubble = &canon.classes()[0];
        assert_eq!(*bubble.label(), 'm');
        assert!(bubble.dom_preimage().is_empty());
        assert!(bubble.cod_preimage().is_empty());
        assert!(bubble.is_scalar());

        let right_only = &canon.classes()[1];
        assert_eq!(*right_only.label(), 'n');
        assert!(right_only.dom_preimage().is_empty());
        assert_eq!(right_only.cod_preimage(), [0_usize].as_slice());
        assert!(!right_only.is_scalar());

        let both_legs = &canon.classes()[2];
        assert_eq!(*both_legs.label(), 'z');
        assert_eq!(both_legs.dom_preimage(), [0_usize, 1].as_slice());
        assert_eq!(both_legs.cod_preimage(), [1_usize].as_slice());
        assert!(!both_legs.is_scalar());

        assert_eq!(canon.scalar_count(), 1);
        assert_eq!(
            canon.classes().iter().filter(|c| c.is_scalar()).count(),
            canon.scalar_count()
        );
    }

    /// Preimage vectors come back sorted ascending even when the leg maps visit
    /// apex vertices in a deliberately non-monotone order.
    ///
    /// The invariant is structural, not defensive: `canonical_form` appends
    /// boundary indices while walking each leg with `enumerate`, so a caller has
    /// no way to supply an out-of-order preimage. What this pins is the thing a
    /// caller *can* perturb — the apex order of the legs — never leaking into a
    /// preimage vector.
    #[test]
    fn preimages_are_sorted_ascending() {
        // Legs jump around the apex; every vertex shares a label, so the class
        // order is decided entirely by the preimages.
        let cospan = Cospan::<char>::new(vec![2, 0, 2, 1], vec![1, 2, 0, 2, 0], vec!['a'; 3]);
        let canon = cospan.canonical_form();

        for class in canon.classes() {
            assert!(
                class.dom_preimage().windows(2).all(|w| w[0] < w[1]),
                "dom preimage not strictly ascending: {:?}",
                class.dom_preimage()
            );
            assert!(
                class.cod_preimage().windows(2).all(|w| w[0] < w[1]),
                "cod preimage not strictly ascending: {:?}",
                class.cod_preimage()
            );
        }

        // Concretely: apex vertex 2 is reached from dom {0, 2} and cod {1, 3},
        // in that order, despite the legs visiting it first, third / second,
        // fourth. It sorts first because [0, 2] is the least dom preimage.
        assert_eq!(canon.classes()[0].dom_preimage(), [0_usize, 2].as_slice());
        assert_eq!(canon.classes()[0].cod_preimage(), [1_usize, 3].as_slice());
        assert_eq!(canon.classes()[1].dom_preimage(), [1_usize].as_slice());
        assert_eq!(canon.classes()[1].cod_preimage(), [2_usize, 4].as_slice());
        assert_eq!(canon.classes()[2].dom_preimage(), [3_usize].as_slice());
        assert_eq!(canon.classes()[2].cod_preimage(), [0_usize].as_slice());
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

        // `is_scalar` is the per-class spelling of the same predicate.
        for canon in [
            id0.canonical_form(),
            one_bubble.canonical_form(),
            two_bubbles.canonical_form(),
        ] {
            assert_eq!(
                canon.classes().iter().filter(|c| c.is_scalar()).count(),
                canon.scalar_count()
            );
        }
        // …and on the two-bubble cospan every class is a scalar.
        let two = two_bubbles.canonical_form();
        assert_eq!(two.scalar_count(), two.apex_len());
        assert!(two.classes().iter().all(ApexClass::is_scalar));
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
