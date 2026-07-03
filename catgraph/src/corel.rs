//! Corelation: jointly-surjective cospan, composed by pushout.
//!
//! Dual of [`Rel`](crate::span::Rel); wraps [`Cospan`]
//! the way `Rel` wraps [`Span`](crate::span::Span).
//!
//! Realizes F&S 2018 (Seven Sketches) Example 6.64: Corel as a hypergraph category.

use std::fmt::Debug;

use crate::{cospan::Cospan, errors::CatgraphError};

/// A corelation: jointly-surjective cospan.
///
/// The dual of [`Rel`](crate::span::Rel). Composition is pushout composition
/// on the underlying cospan; this preserves joint surjectivity.
#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct Corel<Lambda: Eq + Sized + Debug + Copy>(Cospan<Lambda>);

impl<Lambda: Eq + Sized + Debug + Copy> Corel<Lambda> {
    /// Construct a corelation from a cospan, failing if the cospan is not jointly surjective.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Corel`] if the cospan is not jointly surjective.
    pub fn new(cospan: Cospan<Lambda>) -> Result<Self, CatgraphError> {
        if !cospan.is_jointly_surjective() {
            return Err(CatgraphError::Corel {
                message: "cospan is not jointly surjective, cannot form a corelation".to_string(),
            });
        }
        Ok(Self(cospan))
    }

    /// Construct a corelation without checking joint surjectivity.
    /// Caller must guarantee the invariant.
    #[must_use]
    pub fn new_unchecked(cospan: Cospan<Lambda>) -> Self {
        Self(cospan)
    }

    /// View the underlying cospan (for bridge-crate access).
    #[must_use]
    pub fn as_cospan(&self) -> &Cospan<Lambda> {
        &self.0
    }

    /// Return the equivalence classes on `domain ⊔ middle ⊔ codomain` induced
    /// by the cospan: two elements are equivalent iff they map to the same middle vertex.
    ///
    /// Flat index layout: `0..domain_len` for left-leg entries,
    /// `domain_len..(domain_len + middle_len)` for middle vertices,
    /// and `(domain_len + middle_len)..total` for right-leg entries.
    ///
    /// Middle-vertex indices (flat indices in `dom_len..(dom_len + mid_len)`)
    /// are unconditionally inserted into their own class: joint surjectivity
    /// guarantees each middle vertex appears in at least one boundary leg, but
    /// the returned sets always include the middle-vertex index itself alongside
    /// the boundary indices that map to it.
    #[must_use]
    pub fn equivalence_classes(&self) -> Vec<std::collections::HashSet<usize>> {
        let dom_len = self.0.left_to_middle().len();
        let mid_len = self.0.middle().len();
        let cod_len = self.0.right_to_middle().len();

        let mut buckets: Vec<std::collections::HashSet<usize>> =
            vec![std::collections::HashSet::new(); mid_len];

        // Left leg: flat index i belongs to class left_to_middle[i].
        for (i, &m) in self.0.left_to_middle().iter().enumerate() {
            buckets[m].insert(i);
        }
        // Middle vertices: flat index dom_len + j belongs to class j.
        for (j, bucket) in buckets.iter_mut().enumerate() {
            bucket.insert(dom_len + j);
        }
        // Right leg: flat index dom_len + mid_len + k belongs to class right_to_middle[k].
        for (k, &m) in self.0.right_to_middle().iter().enumerate() {
            buckets[m].insert(dom_len + mid_len + k);
        }

        // Joint surjectivity guarantees no empty bucket, but guard anyway.
        buckets.retain(|b| !b.is_empty());
        let _ = cod_len;
        buckets
    }

    /// True iff flat-indexed elements `a` and `b` are in the same equivalence class.
    #[must_use]
    pub fn merges(&self, a: usize, b: usize) -> bool {
        let classes = self.equivalence_classes();
        classes.iter().any(|c| c.contains(&a) && c.contains(&b))
    }

    /// True iff this corelation is the n-element identity partition: every class
    /// contains exactly one domain element, one matching middle vertex, and one
    /// codomain element (paired by index).
    #[must_use]
    pub fn is_identity_partition(&self) -> bool {
        let dom = self.0.left_to_middle();
        let cod = self.0.right_to_middle();
        if dom.len() != cod.len() {
            return false;
        }
        if self.0.middle().len() != dom.len() {
            return false;
        }
        dom.iter().enumerate().all(|(i, &m)| m == i) && cod.iter().enumerate().all(|(i, &m)| m == i)
    }

    /// True iff every equivalence class of `self` sits inside a single class of `other`.
    ///
    /// "Refines" = self's partition is at least as fine as other's. Both corelations
    /// must agree on domain and codomain.
    ///
    /// # Middle-index semantics
    ///
    /// The flat-index scheme of [`Self::equivalence_classes`] includes middle-vertex
    /// indices alongside domain and codomain indices. Because `self` and `other`
    /// can have middle vertices at different flat offsets, middle-vertex elements
    /// of `self` do not in general appear in `other`'s equivalence classes and
    /// are silently skipped during the refinement check. The predicate is
    /// therefore evaluated only over the shared boundary (domain ⊔ codomain),
    /// which is the mathematically meaningful notion of partition refinement
    /// on the cospan interface.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Corel`] if domain or codomain disagree.
    pub fn refines(&self, other: &Self) -> Result<bool, CatgraphError> {
        use crate::category::Composable;
        if self.domain() != other.domain() || self.codomain() != other.codomain() {
            return Err(CatgraphError::Corel {
                message: format!(
                    "domain/codomain mismatch: self ({}, {}), other ({}, {})",
                    self.domain().len(),
                    self.codomain().len(),
                    other.domain().len(),
                    other.codomain().len()
                ),
            });
        }
        let self_classes = self.equivalence_classes();
        let other_classes = other.equivalence_classes();
        for self_class in &self_classes {
            let mut covering_other: Option<usize> = None;
            for elem in self_class {
                let Some(other_idx) = other_classes.iter().position(|o| o.contains(elem)) else {
                    continue;
                };
                match covering_other {
                    None => covering_other = Some(other_idx),
                    Some(existing) if existing == other_idx => {}
                    Some(_) => return Ok(false),
                }
            }
        }
        Ok(true)
    }

    /// Coarsest common refinement: the finest partition that both `self` and `other` refine.
    /// This is the meet in the partition lattice.
    ///
    /// Implementation: union-find over domain ⊔ self-middle ⊔ other-middle ⊔ codomain,
    /// seeded by both cospans' leg maps.
    ///
    // TODO(perf, #37): parallelize the per-root class-extraction loops (dom + cod) via
    // `rayon_cond::CondIterator` once hot-path workload warrants it. Union-find
    // itself stays sequential (path compression mutates during `.find`), but the
    // extraction is embarrassingly parallel once the UF is built. Re-evaluate
    // when large (thousand-node) CCR workloads land;
    // `tests/rayon_equivalence.rs::ccr_deterministic_across_runs` upgrades to
    // a full parallel-vs-sequential equivalence test at that point.
    ///
    /// # Lambda witness selection
    ///
    /// When a class in the resulting refinement has middle-vertex representatives
    /// in both `self` and `other` (necessarily with potentially different `Lambda`
    /// values), this implementation selects the `self`-cospan label. The choice
    /// is deterministic but biased: callers that need a symmetric or
    /// caller-supplied merge rule should post-process the result.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Corel`] if domain or codomain disagree.
    ///
    /// # Panics
    ///
    /// Panics only if the joint-surjectivity invariant is violated (every boundary
    /// element's union-find root must have at least one middle-vertex member). Both
    /// input [`Corel`] values already uphold this invariant via [`Corel::new`].
    pub fn coarsest_common_refinement(&self, other: &Self) -> Result<Self, CatgraphError> {
        use crate::category::Composable;
        use union_find::{QuickUnionUf, UnionBySize, UnionFind};

        if self.domain() != other.domain() || self.codomain() != other.codomain() {
            return Err(CatgraphError::Corel {
                message: "domain/codomain mismatch in coarsest_common_refinement".to_string(),
            });
        }

        let dom_len = self.domain().len();
        let cod_len = self.codomain().len();
        let self_mid_len = self.0.middle().len();
        let other_mid_len = other.0.middle().len();

        let self_mid_start = dom_len;
        let other_mid_start = dom_len + self_mid_len;
        let cod_start = dom_len + self_mid_len + other_mid_len;
        let total = cod_start + cod_len;

        let mut uf: QuickUnionUf<UnionBySize> = QuickUnionUf::new(total);

        // Self-cospan unions.
        for (i, &m) in self.0.left_to_middle().iter().enumerate() {
            uf.union(i, self_mid_start + m);
        }
        for (k, &m) in self.0.right_to_middle().iter().enumerate() {
            uf.union(cod_start + k, self_mid_start + m);
        }
        // Other-cospan unions.
        for (i, &m) in other.0.left_to_middle().iter().enumerate() {
            uf.union(i, other_mid_start + m);
        }
        for (k, &m) in other.0.right_to_middle().iter().enumerate() {
            uf.union(cod_start + k, other_mid_start + m);
        }

        // Extract classes. Each root → a new middle vertex. Lambda witness
        // comes from self-middle first, then other-middle.
        let mut root_to_mid: std::collections::HashMap<usize, usize> =
            std::collections::HashMap::new();
        let mut middle: Vec<Lambda> = Vec::new();
        let mut left: Vec<usize> = Vec::with_capacity(dom_len);
        let mut right: Vec<usize> = Vec::with_capacity(cod_len);

        let self_middle = self.0.middle().to_vec();
        let other_middle = other.0.middle().to_vec();

        // Dom loop: assign each domain element to its class's middle index.
        for i in 0..dom_len {
            let r = uf.find(i);
            let mid_idx = if let Some(&idx) = root_to_mid.get(&r) {
                idx
            } else {
                let lambda = (self_mid_start..self_mid_start + self_mid_len)
                    .find(|&j| uf.find(j) == r)
                    .map(|j| self_middle[j - self_mid_start])
                    .or_else(|| {
                        (other_mid_start..other_mid_start + other_mid_len)
                            .find(|&j| uf.find(j) == r)
                            .map(|j| other_middle[j - other_mid_start])
                    })
                    .expect("jointly surjective invariant ensures boundary element has a middle");
                let new_idx = middle.len();
                root_to_mid.insert(r, new_idx);
                middle.push(lambda);
                new_idx
            };
            left.push(mid_idx);
        }
        // Cod loop: same pattern.
        for k in 0..cod_len {
            let r = uf.find(cod_start + k);
            let mid_idx = if let Some(&idx) = root_to_mid.get(&r) {
                idx
            } else {
                let lambda = (self_mid_start..self_mid_start + self_mid_len)
                    .find(|&j| uf.find(j) == r)
                    .map(|j| self_middle[j - self_mid_start])
                    .or_else(|| {
                        (other_mid_start..other_mid_start + other_mid_len)
                            .find(|&j| uf.find(j) == r)
                            .map(|j| other_middle[j - other_mid_start])
                    })
                    .expect("jointly surjective invariant ensures boundary element has a middle");
                let new_idx = middle.len();
                root_to_mid.insert(r, new_idx);
                middle.push(lambda);
                new_idx
            };
            right.push(mid_idx);
        }

        let cospan = Cospan::new(left, right, middle);
        Corel::new(cospan)
    }
}

// Trait impls — all delegate to the underlying Cospan.

impl<Lambda> crate::category::HasIdentity<Vec<Lambda>> for Corel<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug,
{
    fn identity(on_this: &Vec<Lambda>) -> Self {
        // Cospan::identity on an n-element set is jointly surjective
        // (both legs are the identity map, hitting every middle vertex).
        Self(Cospan::<Lambda>::identity(on_this))
    }
}

impl<Lambda> crate::category::Composable<Vec<Lambda>> for Corel<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug,
{
    fn compose(&self, other: &Self) -> Result<Self, CatgraphError> {
        // Pushout composition of jointly-surjective cospans is jointly surjective.
        self.0.compose(&other.0).map(Self::new_unchecked)
    }

    fn domain(&self) -> Vec<Lambda> {
        self.0.domain()
    }

    fn codomain(&self) -> Vec<Lambda> {
        self.0.codomain()
    }

    fn composable(&self, other: &Self) -> Result<(), CatgraphError> {
        self.0.composable(&other.0)
    }
}

impl<Lambda> crate::monoidal::Monoidal for Corel<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug,
{
    fn monoidal(&mut self, other: Self) {
        // Disjoint union of jointly-surjective cospans is jointly surjective.
        self.0.monoidal(other.0);
    }
}

impl<Lambda> crate::monoidal::MonoidalMorphism<Vec<Lambda>> for Corel<Lambda> where
    Lambda: Sized + Eq + Copy + Debug
{
}

impl<Lambda> crate::monoidal::SymmetricMonoidalMorphism<Lambda> for Corel<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug,
{
    fn from_permutation(
        p: permutations::Permutation,
        types: &[Lambda],
        types_as_on_domain: bool,
    ) -> Result<Self, CatgraphError> {
        // Cospan::from_permutation on an n-element set produces a jointly-surjective cospan.
        Cospan::<Lambda>::from_permutation(p, types, types_as_on_domain).map(Self::new_unchecked)
    }

    fn permute_side(&mut self, p: &permutations::Permutation, of_codomain: bool) {
        self.0.permute_side(p, of_codomain);
    }
}

impl<Lambda> crate::hypergraph_category::HypergraphCategory<Lambda> for Corel<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug,
{
    fn unit(z: Lambda) -> Self {
        // η: [] → [z]. Right leg hits the single middle vertex.
        Self::new_unchecked(Cospan::<Lambda>::unit(z))
    }

    fn counit(z: Lambda) -> Self {
        // ε: [z] → []. Left leg hits the single middle vertex.
        Self::new_unchecked(Cospan::<Lambda>::counit(z))
    }

    fn multiplication(z: Lambda) -> Self {
        // μ: [z, z] → [z].
        Self::new_unchecked(Cospan::<Lambda>::multiplication(z))
    }

    fn comultiplication(z: Lambda) -> Self {
        // δ: [z] → [z, z].
        Self::new_unchecked(Cospan::<Lambda>::comultiplication(z))
    }

    fn cup(z: Lambda) -> Result<Self, CatgraphError> {
        Cospan::<Lambda>::cup(z).map(Self::new_unchecked)
    }

    fn cap(z: Lambda) -> Result<Self, CatgraphError> {
        Cospan::<Lambda>::cap(z).map(Self::new_unchecked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corel_new_accepts_jointly_surjective() {
        let c = Cospan::new(vec![0], vec![1], vec!['a', 'b']);
        let result = Corel::new(c);
        assert!(result.is_ok());
    }

    #[test]
    fn corel_new_rejects_non_surjective() {
        let c = Cospan::new(vec![0], vec![1], vec!['a', 'b', 'c']);
        let result = Corel::new(c);
        assert!(matches!(result, Err(CatgraphError::Corel { .. })));
    }

    #[test]
    fn corel_new_unchecked_bypasses_validation() {
        let c = Cospan::new(vec![0], vec![1], vec!['a', 'b', 'c']);
        let _corel = Corel::new_unchecked(c);
        // no panic, no error — invariant is caller's responsibility
    }

    #[test]
    fn corel_as_cospan_returns_underlying() {
        let c = Cospan::new(vec![0], vec![1], vec!['a', 'b']);
        let corel = Corel::new(c).unwrap();
        assert_eq!(corel.as_cospan().middle(), &['a', 'b']);
    }

    #[test]
    fn corel_identity_is_jointly_surjective() {
        use crate::category::HasIdentity;
        let types = vec!['a', 'b'];
        let id = Corel::<char>::identity(&types);
        assert!(id.as_cospan().is_jointly_surjective());
        assert_eq!(id.as_cospan().middle(), &['a', 'b']);
    }

    #[test]
    fn corel_compose_identity_left_is_noop() {
        use crate::category::{Composable, HasIdentity};
        let types = vec!['a'];
        let id = Corel::<char>::identity(&types);
        let composed = id.compose(&id).unwrap();
        assert!(composed.as_cospan().is_jointly_surjective());
        assert_eq!(composed.as_cospan().middle(), &['a']);
    }

    #[test]
    fn corel_domain_codomain_from_underlying_cospan() {
        use crate::category::Composable;
        let c = Cospan::new(vec![0], vec![1], vec!['a', 'b']);
        let corel = Corel::new(c).unwrap();
        assert_eq!(corel.domain(), vec!['a']);
        assert_eq!(corel.codomain(), vec!['b']);
    }

    #[test]
    fn corel_monoidal_preserves_surjectivity() {
        use crate::monoidal::Monoidal;
        let c1 = Cospan::new(vec![0], vec![0], vec!['a']);
        let c2 = Cospan::new(vec![0], vec![0], vec!['b']);
        let mut corel1 = Corel::new(c1).unwrap();
        let corel2 = Corel::new(c2).unwrap();
        corel1.monoidal(corel2);
        assert!(corel1.as_cospan().is_jointly_surjective());
    }

    #[test]
    fn corel_unit_counit_jointly_surjective() {
        use crate::hypergraph_category::HypergraphCategory;
        let eta = Corel::<char>::unit('a');
        let epsilon = Corel::<char>::counit('a');
        assert!(eta.as_cospan().is_jointly_surjective());
        assert!(epsilon.as_cospan().is_jointly_surjective());
    }

    #[test]
    fn corel_mu_delta_jointly_surjective() {
        use crate::hypergraph_category::HypergraphCategory;
        let mu = Corel::<char>::multiplication('a');
        let delta = Corel::<char>::comultiplication('a');
        assert!(mu.as_cospan().is_jointly_surjective());
        assert!(delta.as_cospan().is_jointly_surjective());
    }

    #[test]
    fn corel_cup_cap_well_formed() {
        use crate::category::Composable;
        use crate::hypergraph_category::HypergraphCategory;
        let cup = Corel::<char>::cup('a').unwrap();
        let cap = Corel::<char>::cap('a').unwrap();
        assert!(cup.as_cospan().is_jointly_surjective());
        assert!(cap.as_cospan().is_jointly_surjective());
        assert_eq!(cup.domain().len(), 0);
        assert_eq!(cup.codomain().len(), 2);
        assert_eq!(cap.domain().len(), 2);
        assert_eq!(cap.codomain().len(), 0);
    }

    #[test]
    fn corel_equivalence_classes_split() {
        // Cospan [0] → [1] with middle ['a', 'b']: two separate classes.
        let c = Cospan::new(vec![0], vec![1], vec!['a', 'b']);
        let corel = Corel::new(c).unwrap();
        let classes = corel.equivalence_classes();
        assert_eq!(classes.len(), 2);
    }

    #[test]
    fn corel_equivalence_classes_merged() {
        // Cospan [0] → [0] with middle ['a']: one class.
        let c = Cospan::new(vec![0], vec![0], vec!['a']);
        let corel = Corel::new(c).unwrap();
        let classes = corel.equivalence_classes();
        assert_eq!(classes.len(), 1);
    }

    #[test]
    fn corel_merges_true_when_same_class() {
        use crate::hypergraph_category::HypergraphCategory;
        let mu = Corel::<char>::multiplication('a');
        // μ: [a, a] → [a] — both domain entries merge with each other.
        // Flat indices: [0, 1] = domain entries, [2] = middle vertex, [3] = codomain.
        assert!(mu.merges(0, 1));
    }

    #[test]
    fn corel_merges_false_when_different_classes() {
        let c = Cospan::new(vec![0], vec![1], vec!['a', 'b']);
        let corel = Corel::new(c).unwrap();
        // Flat: [0] = dom, [1, 2] = middle, [3] = cod. dom(0) is in class 0; cod(3) is in class 1.
        assert!(!corel.merges(0, 3));
    }

    #[test]
    fn corel_is_identity_partition_true_for_identity() {
        use crate::category::HasIdentity;
        let id = Corel::<char>::identity(&vec!['a', 'b', 'c']);
        assert!(id.is_identity_partition());
    }

    #[test]
    fn corel_is_identity_partition_false_for_mu() {
        use crate::hypergraph_category::HypergraphCategory;
        let mu = Corel::<char>::multiplication('a');
        assert!(!mu.is_identity_partition());
    }

    #[test]
    fn corel_refines_self() {
        let c = Cospan::new(vec![0], vec![1], vec!['a', 'b']);
        let corel = Corel::new(c).unwrap();
        let same = Corel::new(Cospan::new(vec![0], vec![1], vec!['a', 'b'])).unwrap();
        assert!(corel.refines(&same).unwrap());
    }

    #[test]
    fn corel_refines_coarser_but_not_converse() {
        // fine: [a, a] → [a, a] with each domain paired to its own codomain (two classes).
        // coarse: [a, a] → [a, a] with everything merged (one class).
        let fine = Corel::new(Cospan::new(vec![0, 1], vec![0, 1], vec!['a', 'a'])).unwrap();
        let coarse = Corel::new(Cospan::new(vec![0, 0], vec![0, 0], vec!['a'])).unwrap();
        assert!(fine.refines(&coarse).unwrap());
        assert!(!coarse.refines(&fine).unwrap());
    }

    #[test]
    fn corel_ccr_matches_self_when_both_equal() {
        let a = Corel::new(Cospan::new(vec![0], vec![1], vec!['a', 'b'])).unwrap();
        let b = Corel::new(Cospan::new(vec![0], vec![1], vec!['a', 'b'])).unwrap();
        let ccr = a.coarsest_common_refinement(&b).unwrap();
        assert_eq!(
            ccr.equivalence_classes().len(),
            a.equivalence_classes().len()
        );
    }
}
