//! Span of finite sets (dual of cospan) and the `Rel` relation algebra wrapper.
//!
//! Composition is via pullback. Middle pairs map into Lambda-typed domain and codomain sets.

use crate::errors::{BoundaryLeg, CatgraphError};

use {
    crate::{
        category::{Composable, HasIdentity},
        monoidal::SymmetricMonoidalMorphism,
        monoidal::{Monoidal, MonoidalMorphism},
        utils::{in_place_permute, represents_id},
    },
    either::Either::{self, Left, Right},
    std::{collections::HashSet, fmt::Debug},
};

type LeftIndex = usize;
type RightIndex = usize;
type MiddleIndex = usize;

/// A span of finite sets: the middle (apex) set maps into Lambda-typed left (domain) and right (codomain) sets.
#[derive(Clone)]
pub struct Span<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug,
{
    /// Pairs `(left_idx, right_idx)` forming the apex-to-boundary leg maps.
    middle: Vec<(LeftIndex, RightIndex)>,
    /// Lambda-typed domain set.
    left: Vec<Lambda>,
    /// Lambda-typed codomain set.
    right: Vec<Lambda>,
    is_left_id: bool,
    is_right_id: bool,
}

impl<Lambda> Span<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug,
{
    /// Debug-asserts structural invariants: leg indices in bounds, type consistency, identity flags.
    ///
    /// Every check is written **inside** its `debug_assert!`, so the whole
    /// method compiles away in release. That is load-bearing rather than
    /// cosmetic: the label-agreement check indexes both boundaries, so when it
    /// was computed into a `let` first (as it was before
    /// [#256](https://github.com/sustia-llc/catgraph/issues/256)) an
    /// out-of-bounds middle pair panicked with a bare "index out of bounds"
    /// even in release — which is neither what the method's name promises nor
    /// what [`Span::new_unchecked`] documents. The bounds assertions still
    /// precede the label assertion, so a debug build reports the specific
    /// invariant rather than an index panic.
    pub fn assert_valid(&self, check_id_strong: bool, check_id_weak: bool) {
        debug_assert!(
            self.middle.iter().all(|(z, _)| *z < self.left.len()),
            "A target for one of the left arrows was out of bounds"
        );
        debug_assert!(
            self.middle.iter().all(|(_, z)| *z < self.right.len()),
            "A target for one of the right arrows was out of bounds"
        );
        debug_assert!(
            self.middle
                .iter()
                .all(|(z1, z2)| self.left[*z1] == self.right[*z2]),
            "There was a left and right linked by something in the span, but their lambda types didn't match"
        );
        if check_id_strong || (check_id_weak && self.is_left_id) {
            debug_assert_eq!(
                represents_id(self.middle_to_left().into_iter()),
                self.is_left_id,
                "The identity nature of the left arrow was wrong"
            );
        }
        if check_id_strong || (check_id_weak && self.is_right_id) {
            debug_assert_eq!(
                represents_id(self.middle_to_right().into_iter()),
                self.is_right_id,
                "The identity nature of the right arrow was wrong"
            );
        }
    }

    /// Construct a span from domain labels, codomain labels, and middle pairs, computing identity flags.
    ///
    /// Every middle pair is checked against both boundaries **in every build
    /// profile**: each component must be in bounds, and the two labels it names
    /// must agree. This is the trust-boundary constructor: use it for pairs
    /// arriving from outside the crate — a store, a wire format, a parser, a
    /// user. Internal callers building a span from data that is correct by
    /// construction should use [`new_unchecked`](Self::new_unchecked), which
    /// costs nothing in release.
    ///
    /// Mirrors [`Rel::new`](Rel::new) / [`Corel::new`](crate::corel::Corel::new):
    /// the checked constructor owns the plain name.
    ///
    /// # Errors
    ///
    /// - [`CatgraphError::ConstructionMiddlePairOutOfBounds`] if a pair's `.0`
    ///   is at or beyond `left.len()`, or its `.1` at or beyond `right.len()`,
    ///   naming which half was out of range, the pair's position **in the
    ///   middle-pair list**, the out-of-range target and the boundary size.
    ///   Deliberately *not*
    ///   [`CatgraphError::ConstructionIndexOutOfBounds`], which a cospan raises:
    ///   there the offending element is an entry of the named leg, here it is a
    ///   middle pair, so the position fields index different things.
    /// - [`CatgraphError::ConstructionLabelMismatch`] if a pair is in bounds on
    ///   both sides but `left[pair.0] != right[pair.1]`, naming the pair's
    ///   position, both indices and both labels.
    ///
    /// Pairs are scanned in ascending position order, and within a pair the
    /// bounds checks precede the label check, so the reported failure is the
    /// first one in that order.
    pub fn new(
        left: Vec<Lambda>,
        right: Vec<Lambda>,
        middle: Vec<(LeftIndex, RightIndex)>,
    ) -> Result<Self, CatgraphError> {
        for (pair_position, &(z1, z2)) in middle.iter().enumerate() {
            if z1 >= left.len() {
                return Err(CatgraphError::ConstructionMiddlePairOutOfBounds {
                    leg: BoundaryLeg::Domain,
                    pair_position,
                    target: z1,
                    target_len: left.len(),
                });
            }
            if z2 >= right.len() {
                return Err(CatgraphError::ConstructionMiddlePairOutOfBounds {
                    leg: BoundaryLeg::Codomain,
                    pair_position,
                    target: z2,
                    target_len: right.len(),
                });
            }
            if left[z1] != right[z2] {
                return Err(CatgraphError::ConstructionLabelMismatch {
                    position: pair_position,
                    left_index: z1,
                    right_index: z2,
                    left_label: format!("{:?}", left[z1]),
                    right_label: format!("{:?}", right[z2]),
                });
            }
        }
        Ok(Self::new_unchecked(left, right, middle))
    }

    /// Construct a span without checking the middle pairs against the boundaries.
    ///
    /// Both the bounds and the label-agreement invariants are the caller's
    /// responsibility; they are re-checked by `debug_assert!` only, so a release
    /// build accepts a malformed pair and defers the failure to whatever indexes
    /// it later. Use this where the pairs are correct **by construction** —
    /// composition results, identity and dagger builders, permutation builders,
    /// monoidal products — and [`new`](Self::new) everywhere data crosses a
    /// trust boundary.
    ///
    /// Mirrors [`Rel::new_unchecked`](Rel::new_unchecked) /
    /// [`Corel::new_unchecked`](crate::corel::Corel::new_unchecked).
    #[must_use]
    pub fn new_unchecked(
        left: Vec<Lambda>,
        right: Vec<Lambda>,
        middle: Vec<(LeftIndex, RightIndex)>,
    ) -> Self {
        let is_left_id = represents_id(middle.iter().map(|tup| tup.0));
        let is_right_id = represents_id(middle.iter().map(|tup| tup.1));
        let answer = Self {
            middle,
            left,
            right,
            is_left_id,
            is_right_id,
        };
        answer.assert_valid(false, false);
        answer
    }

    #[must_use]
    pub fn left(&self) -> &[Lambda] {
        &self.left
    }

    #[must_use]
    pub fn right(&self) -> &[Lambda] {
        &self.right
    }

    #[must_use]
    pub fn middle_pairs(&self) -> &[(LeftIndex, RightIndex)] {
        &self.middle
    }

    #[must_use]
    pub fn is_left_identity(&self) -> bool {
        self.is_left_id
    }

    #[must_use]
    pub fn is_right_identity(&self) -> bool {
        self.is_right_id
    }

    #[must_use]
    pub fn middle_to_left(&self) -> Vec<LeftIndex> {
        self.middle.iter().map(|tup| tup.0).collect()
    }

    #[must_use]
    pub fn middle_to_right(&self) -> Vec<RightIndex> {
        self.middle.iter().map(|tup| tup.1).collect()
    }

    /// Add a boundary node with the given label. `Left` adds to domain, `Right` to codomain.
    ///
    /// The new node is not yet in the image of any middle pair; the caller should
    /// add middle entries via [`add_middle`](Self::add_middle) to connect it.
    ///
    /// # Errors
    ///
    /// **None today — this always returns `Ok`**, and no error arm is described
    /// because there is none. A span's legs point *out* of the apex, so a
    /// boundary node here is a **label rather than an index**: there is no
    /// argument to bounds-check, appending one leaves every existing middle pair
    /// in bounds and label-agreeing, and the identity flags are computed from
    /// the middle pairs alone, which this call does not touch. The `Result` is
    /// a **shape-parity** decision
    /// ([#289](https://github.com/sustia-llc/catgraph/issues/289); owner
    /// directive of 2026-08-22, superseding the issue's per-type reading):
    /// [`Cospan::add_boundary_node`](crate::cospan::Cospan::add_boundary_node)
    /// and
    /// [`NamedCospan::add_boundary_node`](crate::named_cospan::NamedCospan::add_boundary_node)
    /// are fallible for reasons that genuinely apply to them (an out-of-bounds
    /// apex index; a duplicate port name), and the three mutators share one
    /// return type so a caller can treat them alike. No error is anticipated:
    /// the one `Span` follow-up on this method,
    /// [#345](https://github.com/sustia-llc/catgraph/issues/345), is a
    /// flag-semantics change that would land in this method *and* in
    /// [`add_boundary_node_unchecked`](Self::add_boundary_node_unchecked)
    /// alike, and carries no error arm. `add_boundary_node_unchecked` is the
    /// same operation without the wrapper.
    ///
    /// ⚠ **The flags this leaves alone mean less than a `Cospan`'s** — tracked
    /// as [#345](https://github.com/sustia-llc/catgraph/issues/345).
    /// [`new_unchecked`](Self::new_unchecked) computes `is_left_id` /
    /// `is_right_id` as `represents_id` over the middle-pair components with
    /// **no** conjunct against `left.len()` / `right.len()` — the conjunct
    /// [#289](https://github.com/sustia-llc/catgraph/issues/289) added to
    /// [`Cospan`](crate::cospan::Cospan). So after
    /// `Span::identity(&['a', 'b']).add_boundary_node(Left('c'))` the span is no
    /// longer an identity (the middle no longer covers the domain) while
    /// `is_left_identity()` still reports `true`. Nothing mis-composes on it —
    /// `Span::compose` deliberately does not fast-path on the flags, unlike
    /// `Cospan::compose_with_quotient` — but the public accessor is weaker than
    /// its cospan namesake, and tightening it is a separate change with its own
    /// breaking surface.
    pub fn add_boundary_node(
        &mut self,
        new_boundary: Either<Lambda, Lambda>,
    ) -> Result<Either<LeftIndex, RightIndex>, CatgraphError> {
        Ok(self.add_boundary_node_unchecked(new_boundary))
    }

    /// Add a boundary node with the given label, without the parity `Result`.
    ///
    /// Named for parity with
    /// [`Cospan::add_boundary_node_unchecked`](crate::cospan::Cospan::add_boundary_node_unchecked)
    /// and
    /// [`NamedCospan::add_boundary_node_unchecked`](crate::named_cospan::NamedCospan::add_boundary_node_unchecked),
    /// but **not** the same kind of method: those two skip a real check and
    /// carry a `debug_assert!` in its place, so a release build accepts invalid
    /// data through them. This one skips nothing — a label has no invariant to
    /// check — so it is equally safe on untrusted input today. Prefer
    /// [`add_boundary_node`](Self::add_boundary_node) anyway: it is the name
    /// the other two types' callers already write, and the one that would
    /// carry a check if `Span` ever gained a precondition — none is
    /// anticipated; [#345](https://github.com/sustia-llc/catgraph/issues/345)
    /// is a flag-semantics change that lands in both methods alike.
    pub fn add_boundary_node_unchecked(
        &mut self,
        new_boundary: Either<Lambda, Lambda>,
    ) -> Either<LeftIndex, RightIndex> {
        match new_boundary {
            Left(z) => {
                self.left.push(z);
                Left(self.left.len() - 1)
            }
            Right(z) => {
                self.right.push(z);
                Right(self.right.len() - 1)
            }
        }
    }

    /// Add a middle pair mapping to the given left and right indices. Returns the new middle index.
    ///
    /// Both invariants [`new`](Self::new) checks for a whole pair list are
    /// checked here for the one new pair, and in the same order: bounds first,
    /// on the domain half before the codomain half, then label agreement. The
    /// **bounds** failure is reported with `new`'s own variant, so those two
    /// ways of arriving at a middle pair report the identical error. The label
    /// failure is not: this method keeps its pre-existing
    /// [`CatgraphError::Composition`] where `new` raises
    /// [`CatgraphError::ConstructionLabelMismatch`] — a difference #289 did not
    /// close, since changing it would break every caller matching on the
    /// current variant. See `# Errors` below for what each arm actually
    /// returns. Before
    /// [#289](https://github.com/sustia-llc/catgraph/issues/289) this method
    /// reached the labels through `self.left[new_middle.0]` first, so an
    /// out-of-bounds pair panicked with a bare slice message even though
    /// [`CatgraphError::ConstructionMiddlePairOutOfBounds`] exists precisely to
    /// name it.
    ///
    /// # Errors
    ///
    /// - [`CatgraphError::ConstructionMiddlePairOutOfBounds`] if `new_middle.0`
    ///   is at or beyond `left.len()`, or `new_middle.1` at or beyond
    ///   `right.len()`, naming which half was out of range, the **position the
    ///   pair would have occupied** in the middle-pair list (its current
    ///   length), the out-of-range target and the boundary size. The domain half
    ///   is checked before the codomain half.
    /// - [`CatgraphError::Composition`] if the pair is in bounds on both sides
    ///   but the two labels it names disagree.
    ///
    /// On `Err` the span is left exactly as it was — no pair is pushed and the
    /// identity flags are not touched.
    pub fn add_middle(
        &mut self,
        new_middle: (LeftIndex, RightIndex),
    ) -> Result<MiddleIndex, CatgraphError> {
        for (leg, target, target_len) in [
            (BoundaryLeg::Domain, new_middle.0, self.left.len()),
            (BoundaryLeg::Codomain, new_middle.1, self.right.len()),
        ] {
            if target >= target_len {
                return Err(CatgraphError::ConstructionMiddlePairOutOfBounds {
                    leg,
                    pair_position: self.middle.len(),
                    target,
                    target_len,
                });
            }
        }
        let type_left = self.left[new_middle.0];
        let type_right = self.right[new_middle.1];
        if type_left != type_right {
            return Err(CatgraphError::Composition {
                message: format!("Mismatched lambda values {type_left:?} and {type_right:?}"),
            });
        }
        self.middle.push(new_middle);
        self.is_left_id = false;
        self.is_right_id = false;
        Ok(self.middle.len() - 1)
    }

    /// Apply a function to all boundary labels, producing a new span.
    pub fn map<F, Mu>(&self, f: F) -> Span<Mu>
    where
        F: Fn(Lambda) -> Mu,
        Mu: Sized + Eq + Copy + Debug,
    {
        // Correct by construction: `f` is applied pointwise, so both boundaries
        // keep their lengths (bounds preserved) and any pair whose labels agreed
        // before still names two labels that are images of one value under `f`.
        Span::new_unchecked(
            self.left.iter().map(|l| f(*l)).collect(),
            self.right.iter().map(|l| f(*l)).collect(),
            self.middle.clone(),
        )
    }

    /// True if the leg maps are jointly injective (no duplicate middle pairs).
    /// A jointly injective span can be lifted to a [`Rel`].
    #[must_use]
    pub fn is_jointly_injective(&self) -> bool {
        crate::utils::is_unique(&self.middle)
    }

    /// Swap domain and codomain (dagger / adjoint involution).
    #[must_use]
    pub fn dagger(&self) -> Self {
        // Correct by construction: boundaries and pair components are swapped
        // together, so bounds and label agreement transpose with them.
        Self::new_unchecked(
            self.codomain(),
            self.domain(),
            self.middle.iter().map(|(z, w)| (*w, *z)).collect(),
        )
    }
}

impl<Lambda> HasIdentity<Vec<Lambda>> for Span<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug,
{
    fn identity(on_this: &Vec<Lambda>) -> Self {
        Self {
            middle: (0..on_this.len()).map(|idx| (idx, idx)).collect(),
            left: on_this.clone(),
            right: on_this.clone(),
            is_left_id: true,
            is_right_id: true,
        }
    }
}

impl<Lambda> Composable<Vec<Lambda>> for Span<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug,
{
    fn composable(&self, other: &Self) -> Result<(), CatgraphError> {
        crate::utils::same_labels_check(self.right.iter(), other.left.iter())
            .map_err(|message| CatgraphError::Composition { message })
    }

    fn compose(&self, other: &Self) -> Result<Self, CatgraphError> {
        self.composable(other)?;
        // could shortuct if self.is_right_id or other.is_left_id, but unnecessary
        let max_middle = self.middle.len().max(other.middle.len());
        // Correct by construction: the middle starts empty (only pre-sized), and
        // every subsequent pair goes through `add_middle`, which re-checks labels.
        let mut answer = Self::new_unchecked(
            self.left.clone(),
            other.right.clone(),
            Vec::with_capacity(max_middle),
        );
        for (sl, sr) in &self.middle {
            for (ol, or) in &other.middle {
                if sr == ol {
                    let mid_added = answer.add_middle((*sl, *or));
                    match mid_added {
                        Ok(_) => {}
                        Err(z) => {
                            return Err(CatgraphError::Composition {
                                message: format!(
                                    "{z}\nShould be unreachable if composability already said it was all okay."
                                ),
                            });
                        }
                    }
                }
            }
        }
        Ok(answer)
    }

    fn domain(&self) -> Vec<Lambda> {
        self.left.clone()
    }

    fn codomain(&self) -> Vec<Lambda> {
        self.right.clone()
    }
}

impl<Lambda> Monoidal for Span<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug,
{
    fn monoidal(&mut self, mut other: Self) {
        self.is_left_id &= other.is_left_id;
        self.is_right_id &= other.is_right_id;
        let left_shift = self.left.len();
        let right_shift = self.right.len();
        other.middle.iter_mut().for_each(|(v1, v2)| {
            *v1 += left_shift;
            *v2 += right_shift;
        });
        self.middle.extend(other.middle);
        self.left.extend(other.left);
        self.right.extend(other.right);
    }
}

impl<Lambda> MonoidalMorphism<Vec<Lambda>> for Span<Lambda> where Lambda: Sized + Eq + Copy + Debug {}

impl<Lambda> SymmetricMonoidalMorphism<Lambda> for Span<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug,
{
    /// Splices the braiding for `p` onto the codomain or the braiding for `p⁻¹`
    /// onto the domain — see the trait's contract.
    ///
    /// A `Span` carries its wiring and its labels in *different* fields, which
    /// is what makes this the one impl that could be — and was — incoherent
    /// with itself. The apex pairs are the wiring, so moving the wire at slot
    /// `i` to slot `p.apply(i)` is `*v = p.apply(*v)` on the relevant
    /// component. `left`/`right` are label *words*, and a label has to travel
    /// with its wire, so the same move reads `new[k] == old[p.inv().apply(k)]`
    /// on them: `in_place_permute(word, &p.inv())`. The two are inverse to each
    /// other by construction, which is exactly the identity
    /// [`assert_valid`](Span::assert_valid) checks.
    ///
    /// ⚠ **Breaking at #258.** This used to apply `p` to *both*, which is not
    /// two conventions but a broken span: the pair said the wire had moved one
    /// way and the word said its label had moved the other, so
    /// `Span::identity(&['a','b','c']).permute_side(&rotation_left(3,1), true)`
    /// wired domain `'a'` to a codomain slot labelled `'c'`. Since workstream A
    /// moved `assert_valid` inside a `debug_assert!`, a release build accepted
    /// it silently.
    fn permute_side(&mut self, p: &permutations::Permutation, of_codomain: bool) {
        let p_inv = p.inv();
        if of_codomain {
            self.is_right_id = false;
            in_place_permute(&mut self.right, &p_inv);
            self.middle.iter_mut().for_each(|(_, v2)| {
                *v2 = p.apply(*v2);
            });
        } else {
            self.is_left_id = false;
            in_place_permute(&mut self.left, &p_inv);
            self.middle.iter_mut().for_each(|(v1, _)| {
                *v1 = p.apply(*v1);
            });
        }
    }

    /// `domain() == types`, `codomain()[k] == types[p.inv().apply(k)]`.
    ///
    /// Apex element `idx` links domain wire `idx` to codomain wire
    /// `p.apply(idx)` — the wiring the trait specifies.
    fn from_permutation_on_domain(
        p: permutations::Permutation,
        types: &[Lambda],
    ) -> Result<Self, CatgraphError> {
        if p.len() != types.len() {
            return Err(CatgraphError::CompositionSizeMismatch {
                expected: types.len(),
                actual: p.len(),
            });
        }
        Ok(Self {
            left: types.to_vec(),
            middle: (0..types.len()).map(|idx| (idx, p.apply(idx))).collect(),
            right: p.inv().permute(types),
            is_left_id: true,
            is_right_id: false,
        })
    }

    /// `codomain() == types`, `domain()[i] == types[p.apply(i)]`.
    ///
    /// # ⚠ Behaviour change at #258
    ///
    /// The pre-#258 `from_permutation(p, types, /* types_as_on_domain */ false)`
    /// built `left: p.inv().permute(types)` with apex pairs
    /// `(p.apply(idx), idx)`, i.e. it linked domain wire `j` to codomain wire
    /// `p.inv().apply(j)`. That realized **`p⁻¹`**, not `p`, and put the domain
    /// labels in the matching inverted order — so it disagreed with
    /// [`Cospan::from_permutation_on_codomain`](crate::cospan::Cospan) on both
    /// the wiring *and* the domain object for every non-involutive `p`. The
    /// `types_as_on_domain = true` branch was always correct; only this one
    /// drifted, and the two in-crate tests covering it asserted only
    /// `codomain()`, `domain().len()`, and label agreement across the apex —
    /// all of which an inverted wiring satisfies.
    ///
    /// The identity flags moved with the wiring, and for the same reason: they
    /// describe the two *apex leg maps*, not the label vectors. The apex pairs
    /// are now `(idx, p.apply(idx))`, so the left leg is the identity map and
    /// the right leg is `p` — the mirror image of what the old pairs
    /// `(p.apply(idx), idx)` gave.
    fn from_permutation_on_codomain(
        p: permutations::Permutation,
        types: &[Lambda],
    ) -> Result<Self, CatgraphError> {
        if p.len() != types.len() {
            return Err(CatgraphError::CompositionSizeMismatch {
                expected: types.len(),
                actual: p.len(),
            });
        }
        Ok(Self {
            left: p.permute(types),
            middle: (0..types.len()).map(|idx| (idx, p.apply(idx))).collect(),
            right: types.to_vec(),
            is_left_id: true,
            is_right_id: false,
        })
    }
}

/// A relation: a jointly-injective span, i.e. a subset of domain x codomain.
///
/// Supports relational algebra: union, intersection, complement, subsumption,
/// and classification (reflexive, symmetric, transitive, equivalence, partial order).
#[repr(transparent)]
pub struct Rel<Lambda: Eq + Sized + Debug + Copy>(Span<Lambda>);

impl<Lambda> HasIdentity<Vec<Lambda>> for Rel<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug,
{
    fn identity(on_this: &Vec<Lambda>) -> Self {
        Self(Span::<Lambda>::identity(on_this))
    }
}

impl<Lambda> Composable<Vec<Lambda>> for Rel<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug,
{
    fn compose(&self, other: &Self) -> Result<Self, CatgraphError> {
        self.0.compose(&other.0).map(|x| Self(x))
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

impl<Lambda> Monoidal for Rel<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug,
{
    fn monoidal(&mut self, other: Self) {
        self.0.monoidal(other.0);
    }
}

impl<Lambda> MonoidalMorphism<Vec<Lambda>> for Rel<Lambda> where Lambda: Sized + Eq + Copy + Debug {}

impl<Lambda: Eq + Sized + Debug + Copy> Rel<Lambda> {
    /// View the underlying span (for bridge crate access).
    #[must_use]
    pub fn as_span(&self) -> &Span<Lambda> {
        &self.0
    }

    /// Construct a relation from a span, failing if the span is not jointly injective.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Relation`] if the span is not jointly injective.
    pub fn new(x: Span<Lambda>) -> Result<Self, CatgraphError> {
        if !x.is_jointly_injective() {
            return Err(CatgraphError::Relation {
                message: "span is not jointly injective, cannot form a relation".to_string(),
            });
        }
        Ok(Self(x))
    }

    /// Construct a relation without checking joint injectivity. Caller must guarantee the invariant.
    #[must_use]
    pub fn new_unchecked(x: Span<Lambda>) -> Self {
        Self(x)
    }

    /// True if every pair in `other` also appears in `self`. Fails on domain/codomain mismatch.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Relation`] if the relations have different domains or codomains.
    pub fn subsumes(&self, other: &Rel<Lambda>) -> Result<bool, CatgraphError> {
        if self.domain() != other.domain() || self.codomain() != other.codomain() {
            return Err(CatgraphError::Relation {
                message: format!(
                    "domain/codomain mismatch: self ({}, {}), other ({}, {})",
                    self.domain().len(),
                    self.codomain().len(),
                    other.domain().len(),
                    other.codomain().len()
                ),
            });
        }

        let self_pairs: HashSet<(usize, usize)> = self.0.middle.iter().copied().collect();
        let other_pairs: HashSet<(usize, usize)> = other.0.middle.iter().copied().collect();

        Ok(self_pairs.is_superset(&other_pairs))
    }

    /// Set union of two relations. Fails on domain/codomain mismatch.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Relation`] if the relations have different domains or codomains.
    ///
    /// # Panics
    ///
    /// Panics if internal `add_middle` fails — should not occur with valid inputs.
    pub fn union(&self, other: &Self) -> Result<Self, CatgraphError> {
        if self.domain() != other.domain() || self.codomain() != other.codomain() {
            return Err(CatgraphError::Relation {
                message: format!(
                    "domain/codomain mismatch: self ({}, {}), other ({}, {})",
                    self.domain().len(),
                    self.codomain().len(),
                    other.domain().len(),
                    other.codomain().len()
                ),
            });
        }

        let self_pairs: HashSet<(usize, usize)> = self.0.middle.iter().copied().collect();
        let mut ret_val = self.0.clone();
        for (x, y) in &other.0.middle {
            if !self_pairs.contains(&(*x, *y)) {
                // Labels guaranteed to match since `other` was validated at creation.
                ret_val.add_middle((*x, *y)).unwrap();
            }
        }
        Ok(Self(ret_val))
    }

    /// Set intersection of two relations. Fails on domain/codomain mismatch.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Relation`] if the relations have different domains or codomains.
    ///
    /// # Panics
    ///
    /// Panics if internal `add_middle` fails — should not occur with valid inputs.
    pub fn intersection(&self, other: &Self) -> Result<Self, CatgraphError> {
        if self.domain() != other.domain() || self.codomain() != other.codomain() {
            return Err(CatgraphError::Relation {
                message: format!(
                    "domain/codomain mismatch: self ({}, {}), other ({}, {})",
                    self.domain().len(),
                    self.codomain().len(),
                    other.domain().len(),
                    other.codomain().len()
                ),
            });
        }

        let capacity = self.0.middle.len().min(other.0.middle.len());
        // Empty middle, so trivially valid; `add_middle` guards every insertion.
        let mut ret_val = Span::<Lambda>::new_unchecked(
            self.domain(),
            self.codomain(),
            Vec::with_capacity(capacity),
        );

        let self_pairs: HashSet<(usize, usize)> = self.0.middle.iter().copied().collect();
        let other_pairs: HashSet<(usize, usize)> = other.0.middle.iter().copied().collect();

        let in_common = self_pairs.intersection(&other_pairs);
        for (x, y) in in_common {
            ret_val.add_middle((*x, *y)).unwrap();
        }
        Ok(Self(ret_val))
    }

    /// Complement: `(domain x codomain) \ self`. Fails if label mismatches prevent construction.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError`] if the relation is not homogeneous (domain labels != codomain labels).
    pub fn complement(&self) -> Result<Self, CatgraphError> {
        let source_size = self.domain().len();
        let target_size = self.codomain().len();

        let capacity = source_size * target_size - self.0.middle.len();
        // Empty middle, so trivially valid; `add_middle` guards every insertion.
        let mut ret_val = Span::<Lambda>::new_unchecked(
            self.domain(),
            self.codomain(),
            Vec::with_capacity(capacity),
        );

        let self_pairs: HashSet<(usize, usize)> = self.0.middle.iter().copied().collect();

        for x in 0..source_size {
            for y in 0..target_size {
                if !self_pairs.contains(&(x, y)) {
                    ret_val.add_middle((x, y))?;
                }
            }
        }
        Ok(Self(ret_val))
    }

    /// True if domain and codomain are identical (required for reflexivity/symmetry/transitivity).
    #[must_use]
    pub fn is_homogeneous(&self) -> bool {
        self.0.domain() == self.0.codomain()
    }

    /// # Panics
    /// Panics if the relation is not homogeneous (domain != codomain).
    #[must_use]
    pub fn is_reflexive(&self) -> bool {
        let identity_rel = Self::new_unchecked(Span::<Lambda>::identity(&self.0.domain()));
        self.subsumes(&identity_rel).unwrap()
    }

    /// True if no diagonal pair `(x,x)` is present. Returns false on label mismatch.
    #[must_use]
    pub fn is_irreflexive(&self) -> bool {
        self.complement().is_ok_and(|x| x.is_reflexive())
    }

    /// # Panics
    /// Panics if the relation is not homogeneous (domain != codomain).
    #[must_use]
    pub fn is_symmetric(&self) -> bool {
        let dagger = Self::new_unchecked(self.0.dagger());
        self.subsumes(&dagger).unwrap()
    }

    /// # Panics
    /// Panics if the relation is not homogeneous (domain != codomain).
    #[must_use]
    pub fn is_antisymmetric(&self) -> bool {
        let dagger = Self::new_unchecked(self.0.dagger());
        let intersect = self.intersection(&dagger).unwrap();
        let identity_rel = Self::new_unchecked(Span::<Lambda>::identity(&self.0.domain()));
        identity_rel.subsumes(&intersect).unwrap()
    }

    /// # Panics
    /// Panics if the relation is not homogeneous (domain != codomain).
    #[must_use]
    pub fn is_transitive(&self) -> bool {
        // compose can't fail: homogeneous relation has matching domain/codomain
        let twice = Self::new_unchecked(self.0.compose(&self.0).unwrap());
        self.subsumes(&twice).unwrap()
    }

    /// True if the relation is an equivalence relation (reflexive, symmetric, transitive).
    #[must_use]
    pub fn is_equivalence_rel(&self) -> bool {
        self.is_homogeneous() && self.is_reflexive() && self.is_symmetric() && self.is_transitive()
    }

    /// True if the relation is a partial order (reflexive, antisymmetric, transitive).
    #[must_use]
    pub fn is_partial_order(&self) -> bool {
        self.is_homogeneous()
            && self.is_reflexive()
            && self.is_antisymmetric()
            && self.is_transitive()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::category::{Composable, HasIdentity};
    use crate::monoidal::Monoidal;

    // ---- #256: `new` validates in EVERY profile, `new_unchecked` does not ----

    /// `Span::new` refuses a middle pair that points outside either boundary,
    /// and says which leg, which pair, which target, and how big that boundary
    /// was.
    ///
    /// The check under test is the *unconditional* one in `new`, not the
    /// `debug_assert!` in `assert_valid` — this test therefore states the
    /// release-build behaviour and must be run under `--release` too.
    #[test]
    fn span_new_rejects_out_of_bounds_middle_pairs() {
        use crate::errors::{BoundaryLeg, CatgraphError};

        // `Span` has no `Debug`, so `expect_err` is unavailable — destructure.
        // Domain side: pair 1's `.0` is 4, the domain has 2 elements.
        let Err(err) = Span::new(vec!['a', 'b'], vec!['a', 'b'], vec![(0, 0), (4, 1)]) else {
            panic!("left index 4 is out of bounds for a 2-element domain")
        };
        assert_eq!(
            err,
            CatgraphError::ConstructionMiddlePairOutOfBounds {
                leg: BoundaryLeg::Domain,
                pair_position: 1,
                target: 4,
                target_len: 2,
            }
        );
        assert_eq!(
            err.to_string(),
            "construction error: middle pair 1 targets domain index 4, but the domain has 2 element(s)"
        );

        // Codomain side: pair 1's `.1` is 6, the codomain has 2 elements.
        let Err(err) = Span::new(vec!['a', 'b'], vec!['a', 'b'], vec![(0, 0), (1, 6)]) else {
            panic!("right index 6 is out of bounds for a 2-element codomain")
        };
        assert_eq!(
            err,
            CatgraphError::ConstructionMiddlePairOutOfBounds {
                leg: BoundaryLeg::Codomain,
                pair_position: 1,
                target: 6,
                target_len: 2,
            }
        );

        // Both components bad: the domain component is checked first.
        let Err(err) = Span::new(vec!['a'], vec!['a'], vec![(3, 8)]) else {
            panic!("both components are out of bounds")
        };
        assert_eq!(
            err,
            CatgraphError::ConstructionMiddlePairOutOfBounds {
                leg: BoundaryLeg::Domain,
                pair_position: 0,
                target: 3,
                target_len: 1,
            },
            "the domain component must win the race, otherwise the reported leg is not deterministic"
        );

        // `pair_position` indexes the MIDDLE-PAIR list, not the named leg —
        // the distinction this variant exists for. Here the domain has 3
        // elements and every one is in range, yet the offending pair sits at
        // position 0 of a one-element middle list: a consumer reading
        // `(leg, pair_position)` as a leg offset would look at domain entry 0,
        // which is perfectly valid.
        let Err(err) = Span::new(vec!['a', 'b', 'c'], vec!['a'], vec![(9, 0)]) else {
            panic!("pair 0's domain component is out of bounds")
        };
        assert_eq!(
            err,
            CatgraphError::ConstructionMiddlePairOutOfBounds {
                leg: BoundaryLeg::Domain,
                pair_position: 0,
                target: 9,
                target_len: 3,
            }
        );

        // The valid neighbour of the first case still constructs.
        assert!(Span::new(vec!['a', 'b'], vec!['a', 'b'], vec![(0, 0), (1, 1)]).is_ok());
    }

    /// `Span::new` refuses an in-bounds pair whose two labels disagree — a span
    /// witnesses a relation between *equally labelled* boundary elements.
    ///
    /// Release-build behaviour, same as the bounds test above.
    #[test]
    fn span_new_rejects_label_mismatch() {
        use crate::errors::CatgraphError;

        // Pair 0 agrees ('a' = 'a'); pair 1 does not ('b' vs 'c').
        let Err(err) = Span::new(vec!['a', 'b'], vec!['a', 'c'], vec![(0, 0), (1, 1)]) else {
            panic!("pair 1 links 'b' on the domain to 'c' on the codomain")
        };
        assert_eq!(
            err,
            CatgraphError::ConstructionLabelMismatch {
                position: 1,
                left_index: 1,
                right_index: 1,
                left_label: "'b'".to_string(),
                right_label: "'c'".to_string(),
            }
        );
        assert_eq!(
            err.to_string(),
            "construction error: middle pair 1 links domain index 1 to codomain index 1, \
             but their labels disagree ('b' vs 'c')"
        );

        // The pairing, not the boundaries, is what decides. Same two boundaries,
        // two different pairings: the diagonal mismatches, the anti-diagonal is fine.
        assert!(Span::new(vec!['a', 'b'], vec!['b', 'a'], vec![(0, 0)]).is_err());
        assert!(Span::new(vec!['a', 'b'], vec!['b', 'a'], vec![(0, 1), (1, 0)]).is_ok());
    }

    /// `new_unchecked` keeps the pre-#256 contract: both invariants are the
    /// caller's, checked by `debug_assert!` only. Release-only, because in a
    /// debug build those `debug_assert!`s fire — which IS the contract.
    #[cfg(not(debug_assertions))]
    #[test]
    fn span_new_unchecked_accepts_what_new_refuses() {
        let out_of_bounds = Span::new_unchecked(vec!['a'], vec!['a'], vec![(3, 0)]);
        assert_eq!(out_of_bounds.middle_pairs(), &[(3, 0)]);
        assert!(Span::new(vec!['a'], vec!['a'], vec![(3, 0)]).is_err());

        let mismatched = Span::new_unchecked(vec!['a'], vec!['b'], vec![(0, 0)]);
        assert_eq!(mismatched.middle_pairs(), &[(0, 0)]);
        assert!(Span::new(vec!['a'], vec!['b'], vec![(0, 0)]).is_err());
    }

    #[test]
    fn span_new_and_accessors() {
        // Create a simple span with matching types
        let left = vec!['a', 'b'];
        let right = vec!['a', 'b'];
        let middle = vec![(0, 0), (1, 1)];
        let span = Span::new(left.clone(), right.clone(), middle).unwrap();

        assert_eq!(span.domain(), left);
        assert_eq!(span.codomain(), right);
        assert_eq!(span.middle_to_left(), vec![0, 1]);
        assert_eq!(span.middle_to_right(), vec![0, 1]);
    }

    #[test]
    fn span_identity() {
        let types = vec!['x', 'y', 'z'];
        let id = Span::identity(&types);

        assert_eq!(id.domain(), types);
        assert_eq!(id.codomain(), types);
        assert_eq!(id.middle_to_left(), vec![0, 1, 2]);
        assert_eq!(id.middle_to_right(), vec![0, 1, 2]);
        assert!(id.is_jointly_injective());
    }

    #[test]
    fn span_compose_identity() {
        let types = vec!['a', 'b'];
        let id = Span::identity(&types);
        let result = id.compose(&id);
        assert!(result.is_ok());
        let composed = result.unwrap();
        assert_eq!(composed.domain(), types);
        assert_eq!(composed.codomain(), types);
    }

    #[test]
    fn span_compose_general() {
        // f: {0,1} -> {a,b} x {a,b} where f maps to (0,0) and (1,1)
        // g: {0,1} -> {a,b} x {a,b} where g maps to (0,0) and (1,1)
        // f;g should have middle elements where f's right matches g's left
        let left = vec!['a', 'b'];
        let right = vec!['a', 'b'];
        let f = Span::new(left.clone(), right.clone(), vec![(0, 0), (1, 1)]).unwrap();
        let g = Span::new(left.clone(), right.clone(), vec![(0, 0), (1, 1)]).unwrap();

        let result = f.compose(&g);
        assert!(result.is_ok());
    }

    #[test]
    fn span_compose_mismatch() {
        // Spans with matching internal types but incompatible interfaces
        let span1 = Span::new(vec!['a'], vec!['a'], vec![(0, 0)]).unwrap();
        let span2 = Span::new(vec!['b'], vec!['b'], vec![(0, 0)]).unwrap();

        let result = span1.compose(&span2);
        assert!(result.is_err());
    }

    #[test]
    fn span_monoidal() {
        let span1 = Span::new(vec!['a'], vec!['a'], vec![(0, 0)]).unwrap();
        let span2 = Span::new(vec!['b'], vec!['b'], vec![(0, 0)]).unwrap();

        let mut combined = span1;
        combined.monoidal(span2);

        assert_eq!(combined.domain(), vec!['a', 'b']);
        assert_eq!(combined.codomain(), vec!['a', 'b']);
    }

    #[test]
    fn span_dagger() {
        // middle pairs must have matching types at their positions
        let span = Span::new(vec!['a', 'b'], vec!['a', 'b'], vec![(0, 0), (1, 1)]).unwrap();
        let dagger = span.dagger();

        assert_eq!(dagger.domain(), vec!['a', 'b']);
        assert_eq!(dagger.codomain(), vec!['a', 'b']);
    }

    #[test]
    fn span_is_jointly_injective() {
        // Injective: no duplicate pairs
        let span1 = Span::new(vec!['a', 'b'], vec!['a', 'b'], vec![(0, 0), (1, 1)]).unwrap();
        assert!(span1.is_jointly_injective());

        // Not injective: duplicate pair
        let span2 = Span::new(vec!['a', 'b'], vec!['a', 'b'], vec![(0, 0), (0, 0)]).unwrap();
        assert!(!span2.is_jointly_injective());
    }

    #[test]
    fn span_add_middle() {
        let mut span = Span::new(vec!['a', 'b'], vec!['a', 'b'], vec![]).unwrap();

        // Add matching types
        let result = span.add_middle((0, 0));
        assert!(result.is_ok());

        // Add mismatched types
        let result = span.add_middle((0, 1));
        assert!(result.is_err());
    }

    #[test]
    fn span_map() {
        let span = Span::new(vec![1, 2], vec![1, 2], vec![(0, 0), (1, 1)]).unwrap();
        let mapped = span.map(|x| x * 10);

        assert_eq!(mapped.domain(), vec![10, 20]);
        assert_eq!(mapped.codomain(), vec![10, 20]);
    }

    #[test]
    fn span_add_boundary_node() {
        // Start with matching types
        let mut span = Span::new(vec!['a'], vec!['a'], vec![(0, 0)]).unwrap();

        let left_idx = span.add_boundary_node(Left('c')).unwrap();
        assert!(matches!(left_idx, Left(1)));

        let right_idx = span.add_boundary_node(Right('d')).unwrap();
        assert!(matches!(right_idx, Right(1)));

        // #289's parity `Result` is always `Ok` — a boundary node here is a
        // label, not an index — so the `_unchecked` sibling is the same
        // operation with the wrapper removed, not a check skipped.
        let e_idx = span.add_boundary_node_unchecked(Left('e'));
        assert!(matches!(e_idx, Left(2)));

        assert_eq!(span.left(), &['a', 'c', 'e']);
        assert_eq!(span.right(), &['a', 'd']);
    }

    // Rel tests
    #[test]
    fn rel_identity() {
        let types = vec!['a', 'b', 'c'];
        let id = Rel::identity(&types);

        assert_eq!(id.domain(), types);
        assert_eq!(id.codomain(), types);
    }

    #[test]
    fn rel_compose() {
        let types = vec!['a', 'b'];
        let id = Rel::identity(&types);

        let result = id.compose(&id);
        assert!(result.is_ok());
    }

    #[test]
    fn rel_monoidal() {
        let rel1 = Rel::identity(&vec!['a']);
        let rel2 = Rel::identity(&vec!['b']);

        let mut combined = rel1;
        combined.monoidal(rel2);

        assert_eq!(combined.domain(), vec!['a', 'b']);
    }

    #[test]
    fn rel_subsumes() {
        let types = vec!['a', 'b'];
        let full = Rel::new(Span::new(types.clone(), types.clone(), vec![(0, 0), (1, 1)]).unwrap())
            .unwrap();
        let partial =
            Rel::new(Span::new(types.clone(), types.clone(), vec![(0, 0)]).unwrap()).unwrap();

        assert!(full.subsumes(&partial).unwrap());
        assert!(!partial.subsumes(&full).unwrap());
    }

    #[test]
    fn rel_intersection() {
        let types = vec!['a', 'b'];
        let rel1 = Rel::new(Span::new(types.clone(), types.clone(), vec![(0, 0), (1, 1)]).unwrap())
            .unwrap();
        let rel2 =
            Rel::new(Span::new(types.clone(), types.clone(), vec![(0, 0)]).unwrap()).unwrap();

        let intersect = rel1.intersection(&rel2).unwrap();
        assert_eq!(intersect.0.middle.len(), 1);
    }

    #[test]
    fn rel_union() {
        let types = vec!['a', 'b'];
        let rel1 =
            Rel::new(Span::new(types.clone(), types.clone(), vec![(0, 0)]).unwrap()).unwrap();
        let rel2 =
            Rel::new(Span::new(types.clone(), types.clone(), vec![(1, 1)]).unwrap()).unwrap();

        let union = rel1.union(&rel2).unwrap();
        assert_eq!(union.0.middle.len(), 2);
    }

    #[test]
    fn rel_is_homogeneous() {
        let same = Rel::new(Span::new(vec!['a'], vec!['a'], vec![(0, 0)]).unwrap()).unwrap();
        assert!(same.is_homogeneous());

        // For non-homogeneous, use empty middle to avoid type mismatch validation
        let diff = Rel::new(Span::new(vec!['a'], vec!['b'], vec![]).unwrap()).unwrap();
        assert!(!diff.is_homogeneous());
    }

    #[test]
    fn rel_is_reflexive() {
        let types = vec!['a', 'b'];
        let reflexive = Rel::identity(&types);
        assert!(reflexive.is_reflexive());

        // For not reflexive, we can use a relation that's missing the diagonal
        // Use same type at all positions
        let same_types = vec!['a', 'a'];
        let not_reflexive =
            Rel::new(Span::new(same_types.clone(), same_types.clone(), vec![(0, 1)]).unwrap())
                .unwrap();
        assert!(!not_reflexive.is_reflexive());
    }

    #[test]
    fn rel_is_symmetric() {
        let types = vec!['a', 'a'];
        // Symmetric: contains both (0,1) and (1,0)
        let symmetric =
            Rel::new(Span::new(types.clone(), types.clone(), vec![(0, 1), (1, 0)]).unwrap())
                .unwrap();
        assert!(symmetric.is_symmetric());

        // Not symmetric: only (0,1)
        let not_symmetric =
            Rel::new(Span::new(types.clone(), types.clone(), vec![(0, 1)]).unwrap()).unwrap();
        assert!(!not_symmetric.is_symmetric());
    }

    #[test]
    fn rel_is_antisymmetric() {
        let types = vec!['a', 'a'];
        // Antisymmetric: identity relation
        let antisymmetric = Rel::identity(&types);
        assert!(antisymmetric.is_antisymmetric());
    }

    #[test]
    fn rel_is_transitive() {
        let types = vec!['a', 'a', 'a'];
        // Identity is transitive
        let transitive = Rel::identity(&types);
        assert!(transitive.is_transitive());
    }

    #[test]
    fn rel_is_equivalence() {
        let types = vec!['a', 'a'];
        // Identity is an equivalence relation
        let equiv = Rel::identity(&types);
        assert!(equiv.is_equivalence_rel());
    }

    #[test]
    fn rel_is_partial_order() {
        let types = vec!['a', 'a'];
        // Identity is a partial order
        let po = Rel::identity(&types);
        assert!(po.is_partial_order());
    }

    #[test]
    fn rel_complement_non_square() {
        // Non-square: 3-element domain, 2-element codomain
        // All labels 'a' so any (x, y) pair passes the type-match check
        let domain = vec!['a', 'a', 'a'];
        let codomain = vec!['a', 'a'];
        let pairs = vec![(0, 0), (2, 1)];
        let original_count = pairs.len();

        let rel = Rel::new(Span::new(domain.clone(), codomain.clone(), pairs).unwrap()).unwrap();

        let comp = rel.complement().expect("complement should succeed");

        // Full Cartesian product has 3*2 = 6 pairs
        let expected_complement_size = 3 * 2 - original_count;
        assert_eq!(
            comp.0.middle.len(),
            expected_complement_size,
            "complement of non-square relation should have source_size*target_size - original_count pairs"
        );

        // Verify specific pairs: complement should contain exactly the 4 pairs NOT in the original
        let comp_pairs: HashSet<(usize, usize)> = comp.0.middle.iter().copied().collect();
        assert!(!comp_pairs.contains(&(0, 0)), "(0,0) was in original");
        assert!(!comp_pairs.contains(&(2, 1)), "(2,1) was in original");
        assert!(comp_pairs.contains(&(0, 1)));
        assert!(comp_pairs.contains(&(1, 0)));
        assert!(comp_pairs.contains(&(1, 1)));
        assert!(comp_pairs.contains(&(2, 0)));

        // Involution property: complement(complement(r)) == r
        let double_comp = comp.complement().expect("double complement should succeed");
        let original_pairs: HashSet<(usize, usize)> = rel.0.middle.iter().copied().collect();
        let roundtrip_pairs: HashSet<(usize, usize)> =
            double_comp.0.middle.iter().copied().collect();
        assert_eq!(
            original_pairs, roundtrip_pairs,
            "complement(complement(r)) should equal r"
        );
    }

    #[test]
    fn span_from_permutation_identity_domain() {
        use crate::monoidal::SymmetricMonoidalMorphism;
        use permutations::Permutation;

        let types = vec!['a', 'b', 'c'];
        let id_perm = Permutation::identity(3);
        let span = Span::from_permutation_on_domain(id_perm, &types).unwrap();

        assert_eq!(span.domain(), types);
        assert_eq!(span.codomain(), types);
        // Identity permutation: middle should map each index to itself
        assert_eq!(span.middle_to_left(), vec![0, 1, 2]);
        assert_eq!(span.middle_to_right(), vec![0, 1, 2]);
    }

    #[test]
    fn span_from_permutation_identity_codomain() {
        use crate::monoidal::SymmetricMonoidalMorphism;
        use permutations::Permutation;

        let types = vec!['x', 'y'];
        let id_perm = Permutation::identity(2);
        let span = Span::from_permutation_on_codomain(id_perm, &types).unwrap();

        assert_eq!(span.domain(), types);
        assert_eq!(span.codomain(), types);
        assert_eq!(span.middle_to_left(), vec![0, 1]);
        assert_eq!(span.middle_to_right(), vec![0, 1]);
    }

    /// `on_domain` on a hand-computed fixture — every field, not just the
    /// `types` side.
    ///
    /// `rotation_left(3, 1)` has `p(i) = (i+1) % 3` and `p⁻¹(k) = (k+2) % 3`,
    /// so with `types` on the domain the contract forces
    /// `codomain[k] = types[p⁻¹(k)] = ['c','a','b']` and the apex must link
    /// domain wire `i` to codomain wire `p(i)`.
    #[test]
    fn span_from_permutation_rotation_domain() {
        use crate::monoidal::SymmetricMonoidalMorphism;
        use permutations::Permutation;

        let types = vec!['a', 'b', 'c'];
        let rot = Permutation::rotation_left(3, 1);
        let span = Span::from_permutation_on_domain(rot, &types).unwrap();

        assert_eq!(span.domain(), vec!['a', 'b', 'c']);
        assert_eq!(span.codomain(), vec!['c', 'a', 'b']);
        // Wiring, spelled out: apex pair k links domain k to codomain (k+1)%3.
        assert_eq!(span.middle_pairs(), &[(0, 1), (1, 2), (2, 0)]);
        for (left_idx, right_idx) in span.middle.iter().copied() {
            assert_eq!(span.left[left_idx], span.right[right_idx]);
        }
    }

    /// `on_codomain` on the same fixture — the #258 regression pin.
    ///
    /// With `types` on the codomain the contract forces
    /// `domain[i] = types[p(i)] = ['b','c','a']`, and the wiring is still
    /// `i ↦ p(i)`. The pre-#258 body produced `domain = ['c','a','b']` and
    /// pairs `[(1,0),(2,1),(0,2)]`, i.e. it realized `p⁻¹`. Both of the old
    /// assertions here — `codomain()` and label agreement across the apex —
    /// are satisfied by that wrong answer, which is why it survived.
    #[test]
    fn span_from_permutation_rotation_codomain() {
        use crate::monoidal::SymmetricMonoidalMorphism;
        use permutations::Permutation;

        let types = vec!['a', 'b', 'c'];
        let rot = Permutation::rotation_left(3, 1);
        let span = Span::from_permutation_on_codomain(rot, &types).unwrap();

        assert_eq!(span.codomain(), vec!['a', 'b', 'c']);
        assert_eq!(
            span.domain(),
            vec!['b', 'c', 'a'],
            "domain[i] must be types[p(i)]; the pre-#258 body gave types[p.inv()(i)]"
        );
        assert_eq!(
            span.middle_pairs(),
            &[(0, 1), (1, 2), (2, 0)],
            "wiring must be i -> p(i); the pre-#258 body wired i -> p.inv()(i)"
        );
        for (left_idx, right_idx) in span.middle.iter().copied() {
            assert_eq!(span.left[left_idx], span.right[right_idx]);
        }
    }

    /// Both constructors of #258 build the *same* underlying span, differing
    /// only in the labels — so relabelling one gives the other exactly.
    ///
    /// Anchored independently of either body: `p.permute(types)` is computed
    /// here, and the two sides come from two different functions.
    #[test]
    fn span_from_permutation_relabelling_law() {
        use crate::monoidal::SymmetricMonoidalMorphism;
        use crate::utils::rand_perm;
        use rand::SeedableRng;

        let mut rng = rand::rngs::StdRng::seed_from_u64(0x258);
        for n in 2..7usize {
            let types: Vec<usize> = (0..n).map(|i| i + 500).collect();
            let p = rand_perm(n, n * 2, &mut rng);
            let on_cod = Span::from_permutation_on_codomain(p.clone(), &types).unwrap();
            let on_dom = Span::from_permutation_on_domain(p.clone(), &p.permute(&types)).unwrap();
            assert_eq!(on_cod.domain(), on_dom.domain(), "n={n} p={p:?}");
            assert_eq!(on_cod.codomain(), on_dom.codomain(), "n={n} p={p:?}");
            assert_eq!(
                on_cod.middle_pairs(),
                on_dom.middle_pairs(),
                "n={n} p={p:?}"
            );
        }
    }

    // ---- #289: `add_middle` reports the bound it used to panic on ----

    /// `Span::add_middle` rejects an out-of-bounds pair with the variant
    /// `Span::new` already raises for the identical input shape, and leaves the
    /// span untouched.
    ///
    /// # What this ranges over
    ///
    /// Three pairs: one out of range on the domain half (`usize::MAX`, the
    /// falsification-priority value from the audit), one out of range on the
    /// codomain half, and one out of range on **both**, which pins the
    /// domain-before-codomain order. It does not range over boundary sizes or
    /// `Lambda` types, and it says nothing about the label-mismatch arm, which
    /// `span_add_middle_type_mismatch_returns_error` in
    /// `tests/mutation_workflows.rs` already covers.
    #[test]
    fn span_add_middle_rejects_out_of_bounds_pairs() {
        use crate::errors::BoundaryLeg;

        let mut s = Span::new(vec!['a', 'b'], vec!['a'], vec![]).unwrap();

        let err = s
            .add_middle((usize::MAX, 0))
            .expect_err("usize::MAX is out of bounds for a 2-element domain");
        assert_eq!(
            err,
            CatgraphError::ConstructionMiddlePairOutOfBounds {
                leg: BoundaryLeg::Domain,
                pair_position: 0,
                target: usize::MAX,
                target_len: 2,
            },
            "before #289 this input panicked with a bare slice message"
        );
        assert_eq!(
            err.to_string(),
            format!(
                "construction error: middle pair 0 targets domain index {}, but the domain has 2 element(s)",
                usize::MAX
            )
        );

        let err = s
            .add_middle((0, 5))
            .expect_err("codomain index 5 is out of bounds for a 1-element codomain");
        assert_eq!(
            err,
            CatgraphError::ConstructionMiddlePairOutOfBounds {
                leg: BoundaryLeg::Codomain,
                pair_position: 0,
                target: 5,
                target_len: 1,
            }
        );

        // Both halves bad: the domain half is checked first, as in `new`.
        let err = s
            .add_middle((9, 9))
            .expect_err("both halves are out of range");
        assert_eq!(
            err,
            CatgraphError::ConstructionMiddlePairOutOfBounds {
                leg: BoundaryLeg::Domain,
                pair_position: 0,
                target: 9,
                target_len: 2,
            }
        );

        assert!(
            s.middle_pairs().is_empty(),
            "a rejected add_middle must not have pushed a pair"
        );

        // `pair_position` tracks the list, so a later rejection reports a later
        // position — the field is not a constant 0.
        s.add_middle((0, 0))
            .expect("(0, 0) is in bounds and both labels are 'a'");
        let err = s
            .add_middle((0, 4))
            .expect_err("codomain index 4 is out of bounds");
        assert_eq!(
            err,
            CatgraphError::ConstructionMiddlePairOutOfBounds {
                leg: BoundaryLeg::Codomain,
                pair_position: 1,
                target: 4,
                target_len: 1,
            }
        );
    }
}
