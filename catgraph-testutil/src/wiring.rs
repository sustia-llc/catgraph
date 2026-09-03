//! Index-wiring references for tensor and pushout claims.
//!
//! Everything here is plain `Vec<usize>` index data with no catgraph edge, so a
//! test comparing a catgraph value against one of these compares against a
//! reference that is not derived from the implementation under test.

use std::fmt::Debug;

/// One index vector together with the amount [`Wiring::shift_concat`] raises a
/// right operand's entries by.
///
/// For an index vector, `slots` is the size of the space it indexes into. A leg
/// with `slots == 0` carries data no shift applies to — a label word — and
/// `shift_concat` concatenates it unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Leg {
    /// The shift a right operand's entries take, and the size of the index
    /// space for an index vector.
    pub slots: usize,
    /// The entries.
    pub entries: Vec<usize>,
}

impl Leg {
    /// A leg with the given shift and entries.
    #[must_use]
    pub fn new(slots: usize, entries: Vec<usize>) -> Self {
        Self { slots, entries }
    }

    /// A leg over `codes.len()` label positions that no shift applies to.
    #[must_use]
    pub fn word(codes: Vec<usize>) -> Self {
        Self {
            slots: 0,
            entries: codes,
        }
    }
}

/// A morphism's wiring: an ordered list of legs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wiring {
    /// The legs, in a per-carrier fixed order.
    pub legs: Vec<Leg>,
}

impl Wiring {
    /// A wiring from its legs.
    #[must_use]
    pub fn new(legs: Vec<Leg>) -> Self {
        Self { legs }
    }

    /// `self ++ shift(other)`: leg by leg, `other`'s entries raised by `self`'s
    /// slot count for that leg and appended, and the two slot counts summed.
    ///
    /// # Panics
    ///
    /// Panics when the two wirings have different leg counts.
    #[must_use]
    pub fn shift_concat(&self, other: &Self) -> Self {
        assert_eq!(
            self.legs.len(),
            other.legs.len(),
            "shift_concat: leg counts differ ({} vs {})",
            self.legs.len(),
            other.legs.len()
        );
        let legs = self
            .legs
            .iter()
            .zip(&other.legs)
            .map(|(left, right)| {
                let mut entries = left.entries.clone();
                entries.extend(right.entries.iter().map(|e| e + left.slots));
                Leg::new(left.slots + right.slots, entries)
            })
            .collect();
        Self { legs }
    }
}

/// What a [`CospanWiring`] operation rejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WiringError {
    /// A leg entry names an apex position at or beyond the apex size.
    OutOfBounds {
        /// `"dom"` or `"cod"`.
        leg: &'static str,
        /// The position in the leg.
        position: usize,
        /// The out-of-range entry.
        entry: usize,
        /// The apex size the entry had to be below.
        apex_len: usize,
    },
    /// The left operand's codomain and the right operand's domain differ in size.
    BoundaryMismatch {
        /// The left operand's codomain size.
        cod_len: usize,
        /// The right operand's domain size.
        dom_len: usize,
    },
}

/// A cospan's wiring: two boundary-indexed legs into a labelled apex.
///
/// `dom[i]` is the apex position domain index `i` lands on, `cod[j]` the apex
/// position codomain index `j` lands on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CospanWiring<L> {
    apex: Vec<L>,
    dom: Vec<usize>,
    cod: Vec<usize>,
}

impl<L> CospanWiring<L> {
    /// A cospan wiring, with both legs bounds-checked against `apex`.
    ///
    /// # Errors
    ///
    /// [`WiringError::OutOfBounds`] when a leg entry is at or beyond
    /// `apex.len()`; the domain leg is checked before the codomain leg, and
    /// each is checked in ascending position order.
    pub fn new(apex: Vec<L>, dom: Vec<usize>, cod: Vec<usize>) -> Result<Self, WiringError> {
        let apex_len = apex.len();
        for (name, leg) in [("dom", &dom), ("cod", &cod)] {
            for (position, &entry) in leg.iter().enumerate() {
                if entry >= apex_len {
                    return Err(WiringError::OutOfBounds {
                        leg: name,
                        position,
                        entry,
                        apex_len,
                    });
                }
            }
        }
        Ok(Self { apex, dom, cod })
    }

    /// The apex labels.
    #[must_use]
    pub fn apex(&self) -> &[L] {
        &self.apex
    }

    /// The domain leg.
    #[must_use]
    pub fn dom(&self) -> &[usize] {
        &self.dom
    }

    /// The codomain leg.
    #[must_use]
    pub fn cod(&self) -> &[usize] {
        &self.cod
    }

    /// The two legs as a [`Wiring`], domain first, both over `apex().len()`
    /// slots.
    #[must_use]
    pub fn to_wiring(&self) -> Wiring {
        Wiring::new(vec![
            Leg::new(self.apex.len(), self.dom.clone()),
            Leg::new(self.apex.len(), self.cod.clone()),
        ])
    }
}

impl<L: Clone> CospanWiring<L> {
    /// The disjoint-union tensor: apexes concatenated, both legs shift-concatenated.
    #[must_use]
    pub fn tensor(&self, other: &Self) -> Self {
        let wiring = self.to_wiring().shift_concat(&other.to_wiring());
        let mut apex = self.apex.clone();
        apex.extend(other.apex.iter().cloned());
        Self {
            apex,
            dom: wiring.legs[0].entries.clone(),
            cod: wiring.legs[1].entries.clone(),
        }
    }
}

/// A canonical partition signature: boundary sizes plus one sorted entry per
/// apex class.
///
/// Each entry is `(label, sorted domain preimage, sorted codomain preimage)`,
/// and the entries are sorted under that triple's lexicographic order, so the
/// value is invariant under renumbering of the apex.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartitionSignature<L> {
    /// The domain size.
    pub dom_len: usize,
    /// The codomain size.
    pub cod_len: usize,
    /// The sorted class entries.
    pub classes: Vec<(L, Vec<usize>, Vec<usize>)>,
}

impl<L> PartitionSignature<L> {
    /// The number of classes whose two preimages are both empty.
    #[must_use]
    pub fn scalar_count(&self) -> usize {
        self.classes
            .iter()
            .filter(|(_, dom, cod)| dom.is_empty() && cod.is_empty())
            .count()
    }

    /// The signature with the both-preimages-empty classes removed.
    #[must_use]
    pub fn without_scalars(self) -> Self
    where
        L: Clone,
    {
        Self {
            dom_len: self.dom_len,
            cod_len: self.cod_len,
            classes: self
                .classes
                .into_iter()
                .filter(|(_, dom, cod)| !(dom.is_empty() && cod.is_empty()))
                .collect(),
        }
    }
}

impl<L: Clone + Ord> CospanWiring<L> {
    /// The canonical partition signature of this wiring.
    #[must_use]
    pub fn signature(&self) -> PartitionSignature<L> {
        let mut classes: Vec<(L, Vec<usize>, Vec<usize>)> = self
            .apex
            .iter()
            .cloned()
            .map(|label| (label, Vec::new(), Vec::new()))
            .collect();
        for (index, &target) in self.dom.iter().enumerate() {
            classes[target].1.push(index);
        }
        for (index, &target) in self.cod.iter().enumerate() {
            classes[target].2.push(index);
        }
        classes.sort();
        PartitionSignature {
            dom_len: self.dom.len(),
            cod_len: self.cod.len(),
            classes,
        }
    }

    /// The pushout of `self` and `other` along their shared boundary: the
    /// composite's wiring.
    ///
    /// Apex positions `0..self.apex().len()` are `self`'s and the rest are
    /// `other`'s; `self.cod()[b]` is glued to `other.dom()[b]` for every
    /// boundary position `b`. Classes are numbered by ascending least member,
    /// and a class's label is its least member's.
    ///
    /// # Errors
    ///
    /// [`WiringError::BoundaryMismatch`] when `self.cod()` and `other.dom()`
    /// differ in length.
    pub fn pushout(&self, other: &Self) -> Result<Self, WiringError> {
        if self.cod.len() != other.dom.len() {
            return Err(WiringError::BoundaryMismatch {
                cod_len: self.cod.len(),
                dom_len: other.dom.len(),
            });
        }
        let split = self.apex.len();
        let mut union_find = UnionFind::new(split + other.apex.len());
        for (left, right) in self.cod.iter().zip(&other.dom) {
            union_find.union(*left, split + *right);
        }

        let mut class_of = vec![usize::MAX; union_find.len()];
        let mut apex: Vec<L> = Vec::new();
        for position in 0..union_find.len() {
            let root = union_find.find(position);
            if class_of[root] == usize::MAX {
                class_of[root] = apex.len();
                apex.push(if position < split {
                    self.apex[position].clone()
                } else {
                    other.apex[position - split].clone()
                });
            }
            class_of[position] = class_of[root];
        }

        Ok(Self {
            apex,
            dom: self.dom.iter().map(|&v| class_of[v]).collect(),
            cod: other.cod.iter().map(|&v| class_of[split + v]).collect(),
        })
    }
}

/// Disjoint sets over `0..n` with path halving and union by size.
struct UnionFind {
    parent: Vec<usize>,
    size: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            size: vec![1; n],
        }
    }

    fn len(&self) -> usize {
        self.parent.len()
    }

    fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }

    fn union(&mut self, a: usize, b: usize) {
        let (mut ra, mut rb) = (self.find(a), self.find(b));
        if ra == rb {
            return;
        }
        if self.size[ra] < self.size[rb] {
            std::mem::swap(&mut ra, &mut rb);
        }
        self.parent[rb] = ra;
        self.size[ra] += self.size[rb];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `shift_concat` on hand-written legs, including the empty operand and a
    /// zero-slot leg (where the shift is the identity and the law is plain
    /// concatenation).
    #[test]
    fn shift_concat_hand_anchored() {
        let f = Wiring::new(vec![Leg::new(2, vec![1, 0]), Leg::new(2, vec![0])]);
        let g = Wiring::new(vec![Leg::new(3, vec![2]), Leg::new(3, vec![0, 1])]);
        assert_eq!(
            f.shift_concat(&g),
            Wiring::new(vec![Leg::new(5, vec![1, 0, 4]), Leg::new(5, vec![0, 2, 3]),])
        );

        let empty = Wiring::new(vec![Leg::new(0, vec![]), Leg::new(0, vec![])]);
        assert_eq!(f.shift_concat(&empty), f);
        assert_eq!(empty.shift_concat(&f), f);

        let zero_slots = Wiring::new(vec![Leg::new(0, vec![])]);
        let words = Wiring::new(vec![Leg::new(0, vec![])]);
        assert_eq!(zero_slots.shift_concat(&words), zero_slots);
    }

    /// The pushout on hand-derived cases: gluing two wires into one, gluing a
    /// merge against a split, and a boundary of size zero (disjoint union).
    #[test]
    fn pushout_hand_anchored() {
        // id ; id on one wire: one class, both boundaries on it.
        let id = CospanWiring::new(vec!['z'], vec![0], vec![0]).unwrap();
        let composite = id.pushout(&id).unwrap();
        assert_eq!(
            (composite.apex(), composite.dom(), composite.cod()),
            (&['z'][..], &[0][..], &[0][..])
        );

        // mu ; delta: [z,z] -> [z] -> [z,z]. Both apex vertices glue to one.
        let mu = CospanWiring::new(vec!['z'], vec![0, 0], vec![0]).unwrap();
        let delta = CospanWiring::new(vec!['z'], vec![0], vec![0, 0]).unwrap();
        let spider = mu.pushout(&delta).unwrap();
        assert_eq!(
            (spider.apex(), spider.dom(), spider.cod()),
            (&['z'][..], &[0, 0][..], &[0, 0][..])
        );

        // eta ; eps: [] -> [z] -> []. One class, no boundary — a scalar.
        let eta = CospanWiring::new(vec!['z'], vec![], vec![0]).unwrap();
        let eps = CospanWiring::new(vec!['z'], vec![0], vec![]).unwrap();
        let bubble = eta.pushout(&eps).unwrap();
        assert_eq!(bubble.apex().len(), 1);
        assert_eq!(bubble.signature().scalar_count(), 1);

        // Zero-length boundary: nothing glues, the apexes concatenate.
        let disjoint = eps.pushout(&eta).unwrap();
        assert_eq!(disjoint.apex(), &['z', 'z'][..]);
        assert_eq!((disjoint.dom(), disjoint.cod()), (&[0][..], &[1][..]));
    }

    /// The signature sorts classes and preimages, so two apex numberings of one
    /// cospan give one value; the class multiset and the boundary sizes still
    /// separate different cospans.
    #[test]
    fn signature_is_apex_order_invariant() {
        let a = CospanWiring::new(vec!['a', 'b'], vec![0, 1], vec![1]).unwrap();
        let b = CospanWiring::new(vec!['b', 'a'], vec![1, 0], vec![0]).unwrap();
        assert_eq!(a.signature(), b.signature());

        let different = CospanWiring::new(vec!['a', 'b'], vec![0, 0], vec![1]).unwrap();
        assert_ne!(a.signature(), different.signature());

        assert_eq!(
            a.signature().classes,
            vec![('a', vec![0], vec![]), ('b', vec![1], vec![0]),]
        );
    }

    #[test]
    fn construction_and_composition_reject_bad_shapes() {
        assert_eq!(
            CospanWiring::new(vec!['z'], vec![0, 1], vec![]).unwrap_err(),
            WiringError::OutOfBounds {
                leg: "dom",
                position: 1,
                entry: 1,
                apex_len: 1,
            }
        );
        let f = CospanWiring::new(vec!['z'], vec![], vec![0]).unwrap();
        let g = CospanWiring::new(vec!['z'], vec![0, 0], vec![]).unwrap();
        assert_eq!(
            f.pushout(&g).unwrap_err(),
            WiringError::BoundaryMismatch {
                cod_len: 1,
                dom_len: 2,
            }
        );
    }

    /// `tensor` is `shift_concat` on the two legs plus apex concatenation.
    #[test]
    fn tensor_is_shift_concat_plus_apex_concat() {
        let f = CospanWiring::new(vec!['a'], vec![0, 0], vec![0]).unwrap();
        let g = CospanWiring::new(vec!['b', 'c'], vec![1], vec![0, 1]).unwrap();
        let t = f.tensor(&g);
        assert_eq!(t.apex(), &['a', 'b', 'c'][..]);
        assert_eq!((t.dom(), t.cod()), (&[0, 0, 2][..], &[0, 1, 2][..]));
        assert_eq!(t.to_wiring(), f.to_wiring().shift_concat(&g.to_wiring()));
    }
}
