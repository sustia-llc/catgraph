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
use crate::errors::{BoundaryLeg, CatgraphError};

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
/// [`new`](Self::new) does **not** establish either — a single class cannot,
/// since both are statements about the whole class vector.
/// [`CospanCanon::from_parts`] is where they are checked, and it rejects rather
/// than repairs.
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
    /// Assemble a class signature from its parts, **without establishing either
    /// invariant above**.
    ///
    /// This is the reload path's building block: a [`CospanCanon`] read back out
    /// of a store is reassembled class by class and handed to
    /// [`CospanCanon::from_parts`], which is where the invariants are checked.
    /// A class cannot check them on its own — the partition property is a
    /// statement about the *whole* class vector, and the preimage-ordering one
    /// has to be re-checked there anyway, since a class can also arrive from
    /// [`CospanCanon::classes`].
    ///
    /// It deliberately does **not** sort the preimages. An unsorted preimage is
    /// corrupt data, `from_parts` reports it as
    /// [`CatgraphError::CanonPreimageNotAscending`], and silently repairing it
    /// here would hide the corruption at exactly the point a caller most needs
    /// to hear about it — as well as making that rejection untestable, since no
    /// test could then build the malformed value to feed in.
    ///
    /// # Examples
    ///
    /// ```
    /// use catgraph::cospan_canon::{ApexClass, CospanCanon};
    ///
    /// // A wire (dom 0 ↔ cod 0) beside a bubble, in canonical ('a' < 'b') order.
    /// let canon = CospanCanon::from_parts(
    ///     1,
    ///     1,
    ///     vec![
    ///         ApexClass::new('a', vec![], vec![]),
    ///         ApexClass::new('b', vec![0], vec![0]),
    ///     ],
    /// )
    /// .unwrap();
    /// assert_eq!(canon.scalar_count(), 1);
    /// ```
    #[must_use]
    pub fn new(label: Lambda, dom_preimage: Vec<usize>, cod_preimage: Vec<usize>) -> Self {
        Self {
            label,
            dom_preimage,
            cod_preimage,
        }
    }

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
///
/// # Round trip
///
/// The form is reconstructible, not merely readable:
/// [`from_parts`](Self::from_parts) rebuilds one from `(dom_len, cod_len,
/// classes)` after re-establishing every invariant above, and
/// [`to_cospan`](Self::to_cospan) turns it back into a witnessing [`Cospan`].
/// Those invariants are exactly *sufficient* for that reconstruction, which is
/// what makes the validation testable rather than merely asserted:
///
/// ```text
/// c.canonical_form().to_cospan().canonical_form() == c.canonical_form()
/// ```
///
/// So a consumer can persist a canonical form and reload it without keeping the
/// originating cospan — the round trip is the oracle that says the reloaded
/// value is the same comparison value it was.
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

impl<Lambda: Ord> CospanCanon<Lambda> {
    /// Rebuild a canonical form from its parts, re-establishing every invariant
    /// the private construction path guarantees.
    ///
    /// This is the reload half of [`classes`](Self::classes): a consumer that
    /// persisted a canonical form can read it back without keeping (or
    /// re-encoding) the originating [`Cospan`]. Because `CospanCanon`'s `Eq` and
    /// `Hash` decide *apex isomorphism* only by virtue of those invariants, a
    /// constructor that skipped them would hand back a value comparing unequal
    /// to the [`canonical_form`](Cospan::canonical_form) of every cospan — worse
    /// than no constructor at all.
    ///
    /// The invariants are not merely necessary, they are **sufficient**: they
    /// are exactly enough to rebuild a witnessing cospan, which is what
    /// [`to_cospan`](Self::to_cospan) does and what makes this validation
    /// testable rather than merely asserted —
    /// `c.canonical_form().to_cospan().canonical_form() == c.canonical_form()`.
    ///
    /// **Scalars survive.** A class with both preimages empty is a bubble
    /// (`η # ε`), legal, and kept: `Cospan` is the theory of *special*, not
    /// extra-special, commutative Frobenius monoids. Bubbles are also the only
    /// legitimate *duplicate* classes, so the sortedness check accepts equal
    /// neighbours.
    ///
    /// # Rejects, does not repair
    ///
    /// Nothing here sorts the input into shape. The intended input is reloaded
    /// data, and silently repairing a malformed value would hide corruption
    /// exactly where the caller most needs to hear about it. Same posture as
    /// [`Cospan::new`], one type up.
    ///
    /// # Errors
    ///
    /// Checks run in this order, and the first failure is the one reported:
    ///
    /// 1. [`CatgraphError::CanonClassesNotSorted`] — `classes` is not sorted
    ///    under [`ApexClass`]'s `Ord` (lexicographic on `(label, dom_preimage,
    ///    cod_preimage)`). Equal neighbours are accepted.
    /// 2. Per class, domain preimage before codomain preimage, ascending
    ///    through each:
    ///    [`CatgraphError::CanonPreimageOutOfBounds`] — an entry names an index
    ///    at or beyond `dom_len` / `cod_len`; then
    ///    [`CatgraphError::CanonPreimageNotAscending`] — an entry fails to
    ///    strictly exceed its predecessor.
    /// 3. [`CatgraphError::CanonPreimageNotAPartition`] — some index in
    ///    `0..dom_len` (then `0..cod_len`) does not occur in exactly one class's
    ///    preimage.
    ///
    /// # Examples
    ///
    /// ```
    /// use catgraph::cospan::Cospan;
    /// use catgraph::cospan_canon::{ApexClass, CospanCanon};
    ///
    /// // μ: 2 → 1 ← 1, both inputs and the output on one apex vertex.
    /// let mu = Cospan::<char>::new(vec![0, 0], vec![0], vec!['m']).unwrap();
    /// let hand_built =
    ///     CospanCanon::from_parts(2, 1, vec![ApexClass::new('m', vec![0, 1], vec![0])]).unwrap();
    /// assert_eq!(hand_built, mu.canonical_form());
    ///
    /// // An index claimed by no class is rejected rather than repaired.
    /// assert!(CospanCanon::from_parts(2, 1, vec![ApexClass::new('m', vec![0], vec![0])]).is_err());
    /// ```
    pub fn from_parts(
        dom_len: usize,
        cod_len: usize,
        classes: Vec<ApexClass<Lambda>>,
    ) -> Result<Self, CatgraphError> {
        // 1. The class vector is sorted. `<` and not `<=` in the search
        //    predicate: equal neighbours are legal, and they are exactly the
        //    repeated scalars (same label, both preimages empty) whose
        //    multiplicity this form exists to count.
        if let Some(position) = (1..classes.len()).find(|&i| classes[i] < classes[i - 1]) {
            return Err(CatgraphError::CanonClassesNotSorted { position });
        }

        // 2. Every preimage entry indexes its boundary, and every preimage is
        //    strictly ascending.
        for (class_position, class) in classes.iter().enumerate() {
            for (leg, entries, boundary_len) in [
                (BoundaryLeg::Domain, class.dom_preimage.as_slice(), dom_len),
                (
                    BoundaryLeg::Codomain,
                    class.cod_preimage.as_slice(),
                    cod_len,
                ),
            ] {
                for (position, &index) in entries.iter().enumerate() {
                    if index >= boundary_len {
                        return Err(CatgraphError::CanonPreimageOutOfBounds {
                            leg,
                            class_position,
                            position,
                            index,
                            boundary_len,
                        });
                    }
                    if position > 0 && index <= entries[position - 1] {
                        return Err(CatgraphError::CanonPreimageNotAscending {
                            leg,
                            class_position,
                            position,
                        });
                    }
                }
            }
        }

        // 3. Each leg is a function: every boundary index occurs in exactly one
        //    class's preimage. Step 2 established that every entry is in range,
        //    so these tallies cannot index out of bounds.
        let mut dom_occurrences = vec![0_usize; dom_len];
        let mut cod_occurrences = vec![0_usize; cod_len];
        for class in &classes {
            for &i in &class.dom_preimage {
                dom_occurrences[i] += 1;
            }
            for &k in &class.cod_preimage {
                cod_occurrences[k] += 1;
            }
        }
        for (leg, tally, boundary_len) in [
            (BoundaryLeg::Domain, &dom_occurrences, dom_len),
            (BoundaryLeg::Codomain, &cod_occurrences, cod_len),
        ] {
            if let Some((index, &occurrences)) = tally.iter().enumerate().find(|&(_, &n)| n != 1) {
                return Err(CatgraphError::CanonPreimageNotAPartition {
                    leg,
                    index,
                    occurrences,
                    boundary_len,
                });
            }
        }

        Ok(Self {
            dom_len,
            cod_len,
            classes,
        })
    }
}

impl<Lambda> CospanCanon<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug,
{
    /// Rebuild a cospan witnessing this isomorphism class.
    ///
    /// The apex is the classes in canonical order, one vertex per class carrying
    /// that class's label; `left[i]` is the class whose `dom_preimage` contains
    /// `i`, and `right[k]` the class whose `cod_preimage` contains `k`. The
    /// three invariants [`from_parts`](Self::from_parts) checks are exactly what
    /// makes that reconstruction total and unambiguous, so this method is the
    /// oracle for that validation rather than a convenience on top of it:
    ///
    /// ```
    /// use catgraph::cospan::Cospan;
    ///
    /// let c = Cospan::<char>::new(vec![0, 2], vec![2, 0], vec!['z', 'q', 'm']).unwrap();
    /// let canon = c.canonical_form();
    /// assert_eq!(canon.to_cospan().canonical_form(), canon);
    /// ```
    ///
    /// **A witness, not *the* witness.** The apex comes back in canonical
    /// (sorted) order, which need not be the order the originating cospan used,
    /// so the result is generally not
    /// [`structurally_equal`](Cospan::structurally_equal) to it. That is the
    /// point: everything a canonical form retains is retained, and the apex
    /// labelling it deliberately forgot stays forgotten.
    ///
    /// **Scalars are placed.** A bubble class contributes an apex vertex that no
    /// leg reaches, so `k` bubbles round-trip as `k` bubbles.
    #[must_use]
    pub fn to_cospan(&self) -> Cospan<Lambda> {
        let middle: Vec<Lambda> = self.classes.iter().map(|class| class.label).collect();

        // Poison fill rather than 0: the preimages partition `0..dom_len` and
        // `0..cod_len`, so every slot below is written exactly once. A surviving
        // `usize::MAX` would mean that invariant had been broken, and it trips
        // `new_unchecked`'s bounds `debug_assert!` loudly instead of silently
        // pointing the leg at apex vertex 0.
        let mut left = vec![usize::MAX; self.dom_len];
        let mut right = vec![usize::MAX; self.cod_len];
        for (apex, class) in self.classes.iter().enumerate() {
            for &i in &class.dom_preimage {
                left[i] = apex;
            }
            for &k in &class.cod_preimage {
                right[k] = apex;
            }
        }

        // `new_unchecked`: every leg entry written above is a class index, and
        // the apex has exactly one vertex per class, so the bounds invariant
        // `new` checks holds by construction — for a value that reached here,
        // which is every value of this type, since the fields are private and
        // the only two producers are `canonical_form` and the validating
        // `from_parts`.
        Cospan::new_unchecked(left, right, middle)
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
    use super::{ApexClass, CospanCanon};
    use crate::cospan::Cospan;
    use crate::errors::{BoundaryLeg, CatgraphError};

    /// The identity on `n` wires and any apex-reordered presentation of it
    /// canonicalise equally — as whole values *and* class-by-class; a genuinely
    /// different wiring does not.
    #[test]
    fn identity_canonical_is_stable_under_apex_reorder() {
        // id(2): 2 → 2 ← 2, each wire its own apex vertex.
        let id2 = Cospan::<()>::new(vec![0, 1], vec![0, 1], vec![(), ()]).unwrap();
        // Same morphism, apex vertices swapped: wire 0 → apex 1, wire 1 → apex 0.
        let id2_swapped = Cospan::<()>::new(vec![1, 0], vec![1, 0], vec![(), ()]).unwrap();
        let canon = id2.canonical_form();
        let canon_swapped = id2_swapped.canonical_form();
        assert_eq!(canon, canon_swapped);
        // Relabelling invariance is visible in the exposed representation too:
        // the sorted class slices agree element for element.
        assert_eq!(canon.classes(), canon_swapped.classes());

        // The braid 2 → 2 (swap) is a different morphism, and the difference is
        // visible in the classes: its two vertices pair dom i with cod 1-i.
        let braid = Cospan::<()>::new(vec![0, 1], vec![1, 0], vec![(), ()]).unwrap();
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
        let cospan =
            Cospan::<char>::new(vec![0, 3, 1], vec![2, 0], vec!['c', 'a', 'a', 'b']).unwrap();
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
        let cospan = Cospan::<char>::new(vec![0, 0], vec![1, 0], vec!['z', 'n', 'm']).unwrap();
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
        let cospan =
            Cospan::<char>::new(vec![2, 0, 2, 1], vec![1, 2, 0, 2, 0], vec!['a'; 3]).unwrap();
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
        let id0 = Cospan::<()>::new(vec![], vec![], vec![]).unwrap();
        let one_bubble = Cospan::<()>::new(vec![], vec![], vec![()]).unwrap();
        let two_bubbles = Cospan::<()>::new(vec![], vec![], vec![(), ()]).unwrap();

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
        let mu = Cospan::<()>::new(vec![0, 0], vec![0], vec![()]).unwrap();
        // δ-shape: 1 → 1 ← 2 (transpose boundary).
        let delta = Cospan::<()>::new(vec![0], vec![0, 0], vec![()]).unwrap();
        assert_ne!(mu.canonical_form(), delta.canonical_form());
        assert_eq!(mu.canonical_form().dom_len(), 2);
        assert_eq!(mu.canonical_form().cod_len(), 1);
    }

    /// Two structurally different presentations of the same "cup" merge — a
    /// single apex joining both boundary wires — canonicalise equally.
    #[test]
    fn same_merge_different_apex_labels_are_equal() {
        // 1 → 1 ← 1 with everything on apex 0.
        let a = Cospan::<()>::new(vec![0], vec![0], vec![()]).unwrap();
        // 1 → 1 ← 1 built with a spare (unhit) apex vertex present in `b` only:
        // that extra vertex is a bubble, so it is NOT equal to `a`.
        let b = Cospan::<()>::new(vec![0], vec![0], vec![(), ()]).unwrap();
        assert_ne!(a.canonical_form(), b.canonical_form());
        assert_eq!(b.canonical_form().scalar_count(), 1);
    }

    /// The oracle for [`CospanCanon::from_parts`]: the three invariants it
    /// checks are exactly enough to rebuild a witnessing cospan, so
    /// `canonical_form → to_cospan → canonical_form` is the identity.
    ///
    /// Run over a corpus rather than one shape, because the ways this can break
    /// are shape-specific: dropping scalars only shows up on a cospan that has
    /// them, emitting the apex in construction rather than canonical order only
    /// shows up when the two differ, and the empty boundaries only exercise the
    /// degenerate fills. Each fixture's scalar count is asserted, so a fixture
    /// cannot quietly stop covering the case it is named for.
    #[test]
    fn canonical_form_round_trips_through_to_cospan() {
        let corpus: Vec<(&str, Cospan<char>, usize)> = vec![
            (
                "id(3)",
                Cospan::new(vec![0, 1, 2], vec![0, 1, 2], vec!['a', 'b', 'c']).unwrap(),
                0,
            ),
            (
                "braid(2)",
                Cospan::new(vec![0, 1], vec![1, 0], vec!['p', 'q']).unwrap(),
                0,
            ),
            (
                "merge μ: 2 → 1 ← 1",
                Cospan::new(vec![0, 0], vec![0], vec!['m']).unwrap(),
                0,
            ),
            (
                "transpose δ: 1 → 1 ← 2",
                Cospan::new(vec![0], vec![0, 0], vec!['d']).unwrap(),
                0,
            ),
            (
                "one wire beside one scalar",
                Cospan::new(vec![0], vec![0], vec!['w', 's']).unwrap(),
                1,
            ),
            (
                "three scalars sharing a label, beside one wire",
                Cospan::new(vec![1], vec![1], vec!['s', 'w', 's', 's']).unwrap(),
                3,
            ),
            (
                "id₀: empty boundaries, empty apex",
                Cospan::new(vec![], vec![], vec![]).unwrap(),
                0,
            ),
            (
                "empty domain: 0 → 1 ← 2",
                Cospan::new(vec![], vec![0, 0], vec!['c']).unwrap(),
                0,
            ),
            (
                "empty codomain: 2 → 1 ← 0",
                Cospan::new(vec![0, 0], vec![], vec!['u']).unwrap(),
                0,
            ),
            (
                "apex built in reverse canonical order",
                Cospan::new(vec![2, 0], vec![1, 2, 0], vec!['z', 'n', 'a']).unwrap(),
                0,
            ),
            (
                "reverse order, duplicate labels, and a scalar",
                Cospan::new(vec![1, 1], vec![0], vec!['b', 'a', 'a']).unwrap(),
                1,
            ),
        ];

        for (name, cospan, expected_scalars) in &corpus {
            let canon = cospan.canonical_form();
            assert_eq!(
                canon.scalar_count(),
                *expected_scalars,
                "fixture `{name}` no longer has the scalars it is named for"
            );

            let rebuilt = canon.to_cospan();
            let round_tripped = rebuilt.canonical_form();

            // Sharper than the whole-value compare below when the apex itself
            // changes size — which is what dropping the scalars looks like.
            assert_eq!(
                rebuilt.middle().len(),
                canon.apex_len(),
                "`{name}`: to_cospan produced {} apex vertices for {} classes",
                rebuilt.middle().len(),
                canon.apex_len()
            );
            assert_eq!(rebuilt.left_to_middle().len(), canon.dom_len(), "`{name}`");
            assert_eq!(rebuilt.right_to_middle().len(), canon.cod_len(), "`{name}`");

            assert_eq!(
                round_tripped, canon,
                "`{name}`: the round trip changed the canonical form\n  before: {canon:?}\n  after:  {round_tripped:?}"
            );

            // The other direction of the same claim: `from_parts` accepts what
            // `canonical_form` produces, so the invariants it enforces are the
            // ones `canonical_form` establishes — no stricter, or genuine forms
            // would be rejected here.
            let reloaded =
                CospanCanon::from_parts(canon.dom_len(), canon.cod_len(), canon.classes().to_vec())
                    .unwrap_or_else(|e| {
                        panic!("`{name}`: from_parts rejected a genuine canonical form: {e}")
                    });
            assert_eq!(reloaded, canon, "`{name}`: from_parts altered the value");
        }
    }

    /// The poison fill in [`CospanCanon::to_cospan`] is load-bearing, not
    /// decoration: given a value whose preimages do *not* partition the boundary
    /// — unreachable through the public API, so hand-assembled here from the
    /// private fields — the unwritten leg slot survives as `usize::MAX` and
    /// trips `Cospan::new_unchecked`'s bounds `debug_assert!`, instead of
    /// silently pointing that wire at apex vertex 0.
    ///
    /// This is what licenses `new_unchecked` in `to_cospan`: the "correct by
    /// construction" claim rests on `from_parts`, and if that ever stopped
    /// holding the failure is loud rather than a wrong cospan. Debug-only, since
    /// `new_unchecked`'s checks compile away in release exactly as documented.
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "left arrows was out of bounds")]
    fn to_cospan_poison_fill_catches_a_non_partitioning_preimage() {
        // dom_len 2 with only index 0 claimed.
        let malformed = CospanCanon {
            dom_len: 2,
            cod_len: 0,
            classes: vec![ApexClass::new('a', vec![0], vec![])],
        };
        // The fixture must be a value `from_parts` rejects, or this pins
        // nothing. A failure here panics with a *different* message, which
        // `should_panic(expected = ...)` still reports as a test failure.
        assert!(
            CospanCanon::from_parts(
                malformed.dom_len,
                malformed.cod_len,
                malformed.classes.clone(),
            )
            .is_err(),
            "fixture is accepted by from_parts, so it is not the malformed case this test needs"
        );
        let _ = malformed.to_cospan();
    }

    /// [`CospanCanon::from_parts`] rejects each of its three invariants'
    /// violations with a specific, located error — and accepts the repaired
    /// input, so no rejection above can be passing for an unrelated reason.
    #[test]
    fn from_parts_rejects_each_invariant_violation() {
        // ---- 1. the class vector is not sorted under `ApexClass`'s `Ord` ----
        let sorted = vec![
            ApexClass::new('a', vec![1], vec![]),
            ApexClass::new('b', vec![0], vec![]),
        ];
        let mut unsorted = sorted.clone();
        unsorted.reverse();
        assert_eq!(
            CospanCanon::from_parts(2, 0, unsorted).unwrap_err(),
            CatgraphError::CanonClassesNotSorted { position: 1 }
        );
        // The same classes in canonical order are fine, so the rejection is
        // about the order and nothing else.
        assert!(CospanCanon::from_parts(2, 0, sorted).is_ok());
        // Equal neighbours are NOT a violation: repeated scalars are exactly the
        // duplicates a special (not extra-special) Frobenius form must keep.
        assert!(
            CospanCanon::from_parts(
                0,
                0,
                vec![
                    ApexClass::new('s', vec![], vec![]),
                    ApexClass::new('s', vec![], vec![]),
                ]
            )
            .is_ok()
        );

        // ---- 2. a preimage is not strictly ascending ----
        assert_eq!(
            CospanCanon::from_parts(
                3,
                0,
                vec![
                    ApexClass::new('a', vec![0], vec![]),
                    ApexClass::new('b', vec![2, 1], vec![]),
                ]
            )
            .unwrap_err(),
            CatgraphError::CanonPreimageNotAscending {
                leg: BoundaryLeg::Domain,
                class_position: 1,
                position: 1,
            }
        );
        assert!(
            CospanCanon::from_parts(
                3,
                0,
                vec![
                    ApexClass::new('a', vec![0], vec![]),
                    ApexClass::new('b', vec![1, 2], vec![]),
                ]
            )
            .is_ok()
        );
        // The codomain leg is checked too, and "ascending" is strict: a repeat
        // inside one preimage is caught here, not left to the partition tally.
        assert_eq!(
            CospanCanon::from_parts(0, 2, vec![ApexClass::new('a', vec![], vec![1, 1])])
                .unwrap_err(),
            CatgraphError::CanonPreimageNotAscending {
                leg: BoundaryLeg::Codomain,
                class_position: 0,
                position: 1,
            }
        );
        assert!(
            CospanCanon::from_parts(0, 2, vec![ApexClass::new('a', vec![], vec![0, 1])]).is_ok()
        );

        // ---- 3. the preimages do not partition the boundary ----
        // (a) an entry outside the boundary it indexes
        assert_eq!(
            CospanCanon::from_parts(1, 0, vec![ApexClass::new('a', vec![1], vec![])]).unwrap_err(),
            CatgraphError::CanonPreimageOutOfBounds {
                leg: BoundaryLeg::Domain,
                class_position: 0,
                position: 0,
                index: 1,
                boundary_len: 1,
            }
        );
        assert!(CospanCanon::from_parts(1, 0, vec![ApexClass::new('a', vec![0], vec![])]).is_ok());

        // (b) one boundary index claimed by two classes — the leg is not a
        //     function, so the two signatures would no longer be distinct
        assert_eq!(
            CospanCanon::from_parts(
                1,
                0,
                vec![
                    ApexClass::new('a', vec![0], vec![]),
                    ApexClass::new('b', vec![0], vec![]),
                ]
            )
            .unwrap_err(),
            CatgraphError::CanonPreimageNotAPartition {
                leg: BoundaryLeg::Domain,
                index: 0,
                occurrences: 2,
                boundary_len: 1,
            }
        );
        // Widening the boundary so the two claims land on different indices is
        // accepted — the classes themselves were never the problem.
        assert!(
            CospanCanon::from_parts(
                2,
                0,
                vec![
                    ApexClass::new('a', vec![0], vec![]),
                    ApexClass::new('b', vec![1], vec![]),
                ]
            )
            .is_ok()
        );

        // (c) a boundary index claimed by nobody
        assert_eq!(
            CospanCanon::from_parts(2, 0, vec![ApexClass::new('a', vec![0], vec![])]).unwrap_err(),
            CatgraphError::CanonPreimageNotAPartition {
                leg: BoundaryLeg::Domain,
                index: 1,
                occurrences: 0,
                boundary_len: 2,
            }
        );
        // (d) …and on the codomain leg. A bubble does not excuse an unclaimed
        //     boundary index: the scalar below covers no wire.
        assert_eq!(
            CospanCanon::from_parts(0, 1, vec![ApexClass::new('a', vec![], vec![])]).unwrap_err(),
            CatgraphError::CanonPreimageNotAPartition {
                leg: BoundaryLeg::Codomain,
                index: 0,
                occurrences: 0,
                boundary_len: 1,
            }
        );
        // The same bubble beside a class that does claim the wire is accepted.
        assert!(
            CospanCanon::from_parts(
                0,
                1,
                vec![
                    ApexClass::new('a', vec![], vec![]),
                    ApexClass::new('b', vec![], vec![0]),
                ]
            )
            .is_ok()
        );
    }

    /// A hand-built form compares equal to the `canonical_form()` of the cospan
    /// it describes, and reconstructs a witness of that same class — i.e.
    /// [`CospanCanon::from_parts`] yields *usable* values, not merely
    /// well-formed ones.
    #[test]
    fn from_parts_accepts_a_hand_built_form_equal_to_the_cospans_own() {
        // 2 → apex(4) ← 3, apex labels deliberately out of canonical order:
        //   v0 ('z'): dom {0}, cod {1}  — hit by both legs
        //   v1 ('n'): dom {},  cod {2}  — right leg only
        //   v2 ('m'): dom {1}, cod {0}  — hit by both legs
        //   v3 ('q'): dom {},  cod {}   — bubble
        let cospan = Cospan::<char>::new(vec![0, 2], vec![2, 0, 1], vec!['z', 'n', 'm', 'q'])
            .expect("fixture legs are in bounds");

        let hand_built = CospanCanon::from_parts(
            2,
            3,
            vec![
                ApexClass::new('m', vec![1], vec![0]),
                ApexClass::new('n', vec![], vec![2]),
                ApexClass::new('q', vec![], vec![]),
                ApexClass::new('z', vec![0], vec![1]),
            ],
        )
        .unwrap();

        assert_eq!(hand_built, cospan.canonical_form());
        assert_eq!(hand_built.classes(), cospan.canonical_form().classes());
        assert_eq!(hand_built.dom_len(), 2);
        assert_eq!(hand_built.cod_len(), 3);
        assert_eq!(hand_built.apex_len(), 4);
        assert_eq!(hand_built.scalar_count(), 1);

        // It reconstructs a witness of the same class — but not the original
        // presentation: `to_cospan` emits the apex in canonical order, and that
        // difference is precisely the apex labelling the form forgets.
        let witness = hand_built.to_cospan();
        assert_eq!(witness.canonical_form(), cospan.canonical_form());
        assert!(!witness.structurally_equal(&cospan));
        assert_eq!(witness.middle(), ['m', 'n', 'q', 'z'].as_slice());
        assert_eq!(witness.left_to_middle(), [3_usize, 0].as_slice());
        assert_eq!(witness.right_to_middle(), [0_usize, 3, 1].as_slice());
        // The bubble ('q', class 2) is in the apex although no leg reaches it.
        assert!(!witness.left_to_middle().contains(&2));
        assert!(!witness.right_to_middle().contains(&2));
    }
}
