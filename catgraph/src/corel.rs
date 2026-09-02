//! Corelation: jointly-surjective cospan, composed by pushout **then
//! restriction to the outer boundary**.
//!
//! Dual of [`Rel`](crate::span::Rel); wraps [`Cospan`]
//! the way `Rel` wraps [`Span`](crate::span::Span).
//!
//! Realizes F&S 2018 (Seven Sketches) Example 6.64: Corel as a hypergraph category.

use std::fmt::Debug;

use crate::{cospan::Cospan, errors::CatgraphError};

/// A corelation: jointly-surjective cospan.
///
/// The dual of [`Rel`](crate::span::Rel). Composition is pushout composition on
/// the underlying cospan **followed by the restriction to the outer boundary**
/// (F&S 2018 Ex 4.61 fn. 2 step (iii)) — the pushout on its own does *not*
/// preserve joint surjectivity, and that restriction is what makes `compose`'s
/// result something [`Corel::new`](Self::new) would accept. See
/// [`Composable::compose`](crate::category::Composable::compose)'s notes on
/// this type.
///
/// [`from_cospan_dropping_bubbles`](Self::from_cospan_dropping_bubbles) is the
/// same restriction exposed as a total map from `Cospan`, and is the only way
/// to move a bubble-carrying cospan into this type — [`new`](Self::new) refuses
/// one.
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

    /// The **total** map `Cospan → Corel`: drop every apex vertex neither leg
    /// reaches, then reindex both legs onto the survivors.
    ///
    /// Defined on every `Cospan<Lambda>`, with a jointly-surjective image, so
    /// [`new`](Self::new) accepts `q(c).as_cospan().clone()` for every `c`.
    ///
    /// A dropped vertex is exactly a scalar / bubble in
    /// [`CospanCanon`](crate::cospan_canon::CospanCanon)'s sense — both
    /// preimages empty — so `q(c).as_cospan().canonical_form().scalar_count()`
    /// is `0` for every `c`. The surviving vertices keep their **relative
    /// order**: survivor `i` precedes survivor `j` in the image apex iff it did
    /// in `c`'s apex. Both legs are rewritten through the same
    /// old-index → new-index renumbering, so no boundary wire changes which
    /// class it sits in, and `q(c).domain() == c.domain()` and likewise for the
    /// codomain.
    ///
    /// On an already jointly-surjective input it returns the cospan unchanged.
    ///
    /// This is not
    /// [`Cospan::canonical_form`](crate::cospan::Cospan::canonical_form)'s
    /// treatment of scalars: a canonical form keeps them, and this map drops
    /// them on the way into `Corel`, whose objects are equivalence relations on
    /// the boundary alone.
    ///
    /// # Panics
    ///
    /// Panics if `cospan` violates the leg-bounds invariant every `Cospan`
    /// constructor upholds (a leg entry at or beyond `middle().len()`), which
    /// is reachable only through a release-mode
    /// [`Cospan::new_unchecked`](crate::cospan::Cospan::new_unchecked) misuse.
    ///
    /// # Examples
    ///
    /// ```
    /// use catgraph::{corel::Corel, cospan::Cospan};
    ///
    /// // Apex vertex 1 ('z') is reached by neither leg.
    /// let c = Cospan::new(vec![0], vec![2], vec!['a', 'z', 'b']).unwrap();
    /// assert!(Corel::new(c.clone()).is_err());
    ///
    /// let q = Corel::from_cospan_dropping_bubbles(c);
    /// assert_eq!(q.as_cospan().middle(), &['a', 'b']);
    /// assert_eq!(q.as_cospan().left_to_middle(), &[0]);
    /// assert_eq!(q.as_cospan().right_to_middle(), &[1]);
    /// ```
    #[must_use]
    pub fn from_cospan_dropping_bubbles(cospan: Cospan<Lambda>) -> Self {
        let apex_len = cospan.middle().len();
        let mut reached = vec![false; apex_len];
        for &m in cospan.left_to_middle() {
            assert!(m < apex_len, "left leg entry {m} is outside the apex");
            reached[m] = true;
        }
        for &m in cospan.right_to_middle() {
            assert!(m < apex_len, "right leg entry {m} is outside the apex");
            reached[m] = true;
        }
        if reached.iter().all(|r| *r) {
            // Already jointly surjective — the identity, not a rebuilt copy.
            return Self(cospan);
        }

        // Order-preserving renumbering of the survivors. `usize::MAX` marks a
        // dropped vertex and is never read: only leg entries index into
        // `renumber`, and every leg entry is `reached`.
        let mut renumber = vec![usize::MAX; apex_len];
        let mut middle = Vec::with_capacity(apex_len);
        for (old, &is_reached) in reached.iter().enumerate() {
            if is_reached {
                renumber[old] = middle.len();
                middle.push(cospan.middle()[old]);
            }
        }
        let left = cospan
            .left_to_middle()
            .iter()
            .map(|&m| renumber[m])
            .collect();
        let right = cospan
            .right_to_middle()
            .iter()
            .map(|&m| renumber[m])
            .collect();
        // Correct by construction: every entry is a `middle.len()` captured
        // before the corresponding push, so both legs land inside `middle`.
        Self(Cospan::new_unchecked(left, right, middle))
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

        // Correct by construction: every index pushed into `left`/`right` is a
        // `middle.len()` captured before the corresponding `middle.push`.
        let cospan = Cospan::new_unchecked(left, right, middle);
        Corel::new(cospan)
    }
}

// Trait impls — all delegate to the underlying Cospan EXCEPT `Composable`,
// which performs F&S 2018 Ex 4.61 fn. 2's step (iii) on top of the pushout, so
// `Corel` is not a transparent newtype for composition.

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
    /// Corelation composition: pushout on the underlying cospans, **then the
    /// restriction to the outer boundary**.
    ///
    /// F&S 2018 (*Seven Sketches*) Example 4.61 fn. 2 gives composition of
    /// corelations `α : A → B`, `β : B → C` in three steps: (i) read both as
    /// relations on `A ⊔ B ⊔ C`, (ii) take the transitive closure of their
    /// union, (iii) **restrict to an equivalence relation on `A ⊔ C`**.
    /// [`Cospan::compose`](crate::category::Composable::compose)'s pushout is
    /// (i) + (ii); step (iii) is
    /// [`from_cospan_dropping_bubbles`](Self::from_cospan_dropping_bubbles),
    /// and it is not optional.
    ///
    /// Step (iii) is load-bearing: the pushout of two jointly-surjective
    /// cospans need not be jointly surjective. `0 → {m} ← 1` composed with
    /// `1 → {m} ← 0` glues the two boundary-only vertices into a class no
    /// outer leg reaches; with step (iii) the composite is the empty
    /// corelation.
    ///
    /// # Encoding of a composite
    ///
    /// The **relation** on `domain ⊔ codomain` carries no dropped class, so it
    /// is unchanged by step (iii); the encoding is:
    ///
    /// - a smaller apex wherever a composition merged two boundary-only
    ///   vertices, so anything counting apex vertices, reading
    ///   [`as_cospan`](Self::as_cospan)`.middle()`, or hashing / comparing the
    ///   underlying [`Cospan`] sees a different value;
    /// - [`equivalence_classes`](Self::equivalence_classes) lays its flat
    ///   indices out as `0..dom_len` │ `dom_len..dom_len + mid_len` │
    ///   `dom_len + mid_len..`, so a smaller apex **shifts every codomain flat
    ///   index down**, and the class count drops with the bubble;
    /// - therefore [`merges`](Self::merges) (whose arguments are flat indices),
    ///   [`is_identity_partition`](Self::is_identity_partition) and
    ///   `equivalence_classes().len()` all change on such a composite.
    ///
    /// ⚠ [`refines`](Self::refines) matches classes by flat index across two
    /// values and silently skips elements it cannot find in the other, so on a
    /// composite whose apex shrank it can skip the whole boundary and answer
    /// `true`.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] if the two interfaces do not
    /// type-match, or if the internal pushout fails.
    fn compose(&self, other: &Self) -> Result<Self, CatgraphError> {
        self.0
            .compose(&other.0)
            .map(Self::from_cospan_dropping_bubbles)
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
    fn from_permutation_on_domain(
        p: permutations::Permutation,
        types: &[Lambda],
    ) -> Result<Self, CatgraphError> {
        // Cospan's permutation builders on an n-element set produce a
        // jointly-surjective cospan, so the Corel invariant holds by construction.
        Cospan::<Lambda>::from_permutation_on_domain(p, types).map(Self::new_unchecked)
    }

    fn from_permutation_on_codomain(
        p: permutations::Permutation,
        types: &[Lambda],
    ) -> Result<Self, CatgraphError> {
        Cospan::<Lambda>::from_permutation_on_codomain(p, types).map(Self::new_unchecked)
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
        let c = Cospan::new(vec![0], vec![1], vec!['a', 'b']).unwrap();
        let result = Corel::new(c);
        assert!(result.is_ok());
    }

    #[test]
    fn corel_new_rejects_non_surjective() {
        let c = Cospan::new(vec![0], vec![1], vec!['a', 'b', 'c']).unwrap();
        let result = Corel::new(c);
        assert!(matches!(result, Err(CatgraphError::Corel { .. })));
    }

    #[test]
    fn corel_new_unchecked_bypasses_validation() {
        let c = Cospan::new(vec![0], vec![1], vec!['a', 'b', 'c']).unwrap();
        let _corel = Corel::new_unchecked(c);
        // no panic, no error — invariant is caller's responsibility
    }

    #[test]
    fn corel_as_cospan_returns_underlying() {
        let c = Cospan::new(vec![0], vec![1], vec!['a', 'b']).unwrap();
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
        let c = Cospan::new(vec![0], vec![1], vec!['a', 'b']).unwrap();
        let corel = Corel::new(c).unwrap();
        assert_eq!(corel.domain(), vec!['a']);
        assert_eq!(corel.codomain(), vec!['b']);
    }

    #[test]
    fn corel_monoidal_preserves_surjectivity() {
        use crate::monoidal::Monoidal;
        let c1 = Cospan::new(vec![0], vec![0], vec!['a']).unwrap();
        let c2 = Cospan::new(vec![0], vec![0], vec!['b']).unwrap();
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
        let c = Cospan::new(vec![0], vec![1], vec!['a', 'b']).unwrap();
        let corel = Corel::new(c).unwrap();
        let classes = corel.equivalence_classes();
        assert_eq!(classes.len(), 2);
    }

    #[test]
    fn corel_equivalence_classes_merged() {
        // Cospan [0] → [0] with middle ['a']: one class.
        let c = Cospan::new(vec![0], vec![0], vec!['a']).unwrap();
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
        let c = Cospan::new(vec![0], vec![1], vec!['a', 'b']).unwrap();
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
        let c = Cospan::new(vec![0], vec![1], vec!['a', 'b']).unwrap();
        let corel = Corel::new(c).unwrap();
        let same = Corel::new(Cospan::new(vec![0], vec![1], vec!['a', 'b']).unwrap()).unwrap();
        assert!(corel.refines(&same).unwrap());
    }

    #[test]
    fn corel_refines_coarser_but_not_converse() {
        // fine: [a, a] → [a, a] with each domain paired to its own codomain (two classes).
        // coarse: [a, a] → [a, a] with everything merged (one class).
        let fine =
            Corel::new(Cospan::new(vec![0, 1], vec![0, 1], vec!['a', 'a']).unwrap()).unwrap();
        let coarse = Corel::new(Cospan::new(vec![0, 0], vec![0, 0], vec!['a']).unwrap()).unwrap();
        assert!(fine.refines(&coarse).unwrap());
        assert!(!coarse.refines(&fine).unwrap());
    }

    #[test]
    fn corel_ccr_matches_self_when_both_equal() {
        let a = Corel::new(Cospan::new(vec![0], vec![1], vec!['a', 'b']).unwrap()).unwrap();
        let b = Corel::new(Cospan::new(vec![0], vec![1], vec!['a', 'b']).unwrap()).unwrap();
        let ccr = a.coarsest_common_refinement(&b).unwrap();
        assert_eq!(
            ccr.equivalence_classes().len(),
            a.equivalence_classes().len()
        );
    }
}
