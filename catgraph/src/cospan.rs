//! Cospan of finite sets with typed middle vertices.
//!
//! Composition is via pushout (union-find, nearly linear time). Source/target semantics:
//! an edge `[a,b] -> [c,d]` forms the bipartite complete subgraph between sources and targets.

use {
    crate::{
        category::{Composable, HasIdentity},
        errors::{BoundaryLeg, CatgraphError},
        finset::FinSetMap,
        monoidal::SymmetricMonoidalMorphism,
        monoidal::{Monoidal, MonoidalMorphism},
        utils::{EitherExt, in_place_permute, represents_id},
    },
    either::Either::{self, Left, Right},
    log::warn,
    permutations::Permutation,
    std::{collections::HashMap, fmt::Debug},
    union_find::{UnionBySize, UnionFind},
};

type LeftIndex = usize;
type RightIndex = usize;
type MiddleIndex = usize;
type MiddleIndexOrLambda<Lambda> = Either<MiddleIndex, Lambda>;

/// The predicate both identity flags cache: the leg is a bijection onto the
/// whole apex — as long as the apex **and** `[0, 1, …, n-1]` in order.
///
/// One spelling for every site that computes a flag from scratch
/// (`new_unchecked`, `assert_valid`'s strong arm, `connect_pair`):
/// [#289](https://github.com/sustia-llc/catgraph/issues/289) found three arms
/// that had re-spelled it by hand and lost the length conjunct.
fn leg_is_identity(leg: &[MiddleIndex], apex_len: usize) -> bool {
    leg.len() == apex_len && represents_id(leg.iter().copied())
}

/// A cospan of finite sets: left (domain) and right (codomain) legs map into a Lambda-typed middle set.
#[derive(Clone, Debug)]
pub struct Cospan<Lambda: Sized + Eq + Copy + Debug> {
    /// Domain leg: maps each left boundary node to a middle index.
    left: Vec<MiddleIndex>,
    /// Codomain leg: maps each right boundary node to a middle index.
    right: Vec<MiddleIndex>,
    /// The middle (apex) set, with Lambda-typed vertices.
    middle: Vec<Lambda>,
    is_left_id: bool,
    is_right_id: bool,
}

impl<Lambda> Cospan<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug,
{
    /// Debug-asserts structural invariants: leg indices in bounds, and — when
    /// asked — each identity flag equal to the predicate it caches.
    ///
    /// That predicate is [`new_unchecked`](Self::new_unchecked)'s, in both
    /// arms: `leg.len() == middle.len() && represents_id(leg)`, spelled once
    /// as the private `leg_is_identity` and shared with `new_unchecked` and
    /// [`connect_pair`](Self::connect_pair). The length conjunct is not decoration — without it the strong arm rejects valid
    /// cospans, e.g. `Cospan::new(vec![0], vec![0, 1], vec!['a', 'b'])`, whose
    /// `is_left_id` is correctly `false` while `represents_id([0])` is `true`.
    ///
    /// `check_id_weak` asks the same question of a flag that is currently
    /// `true` only, which is the direction that matters for a value built by
    /// the mutators: they can leave a `false` conservatively behind the
    /// predicate (`&=` never restores a `true`), so the strong arm is for
    /// values whose flags should be exact.
    ///
    /// Every check is written **inside** its `debug_assert!`, so the whole
    /// method compiles away in release — which is what
    /// [`Cospan::new_unchecked`] documents, and what keeps its release cost at
    /// zero. (Its `Span` counterpart needed the same shape for a stronger
    /// reason; see [`Span::assert_valid`](crate::span::Span::assert_valid).)
    pub fn assert_valid(&self, check_id_strong: bool, check_id_weak: bool) {
        debug_assert!(
            self.left.iter().all(|z| *z < self.middle.len()),
            "A target for one of the left arrows was out of bounds"
        );
        debug_assert!(
            self.right.iter().all(|z| *z < self.middle.len()),
            "A target for one of the right arrows was out of bounds"
        );
        if check_id_strong || (check_id_weak && self.is_left_id) {
            debug_assert_eq!(
                leg_is_identity(&self.left, self.middle.len()),
                self.is_left_id,
                "The identity nature of the left arrow was wrong"
            );
        }
        if check_id_strong || (check_id_weak && self.is_right_id) {
            debug_assert_eq!(
                leg_is_identity(&self.right, self.middle.len()),
                self.is_right_id,
                "The identity nature of the right arrow was wrong"
            );
        }
    }

    /// Construct a cospan from explicit leg maps and middle set, computing identity flags.
    ///
    /// Both legs are checked against the apex **in every build profile**. This is
    /// the trust-boundary constructor: use it for leg maps arriving from outside
    /// the crate — a store, a wire format, a parser, a user. Internal callers
    /// building a cospan from data that is correct by construction should use
    /// [`new_unchecked`](Self::new_unchecked), which costs nothing in release.
    ///
    /// Mirrors [`Corel::new`](crate::corel::Corel::new) /
    /// [`Rel::new`](crate::span::Rel::new): the checked constructor owns the
    /// plain name.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::ConstructionIndexOutOfBounds`] if any `left` or
    /// `right` entry targets an index at or beyond `middle.len()`, naming the
    /// leg, the entry's position within it, the out-of-range target, and the
    /// apex size. The domain leg is scanned before the codomain leg, and each
    /// leg in ascending position order, so the reported failure is the first
    /// one in that order.
    pub fn new(
        left: Vec<MiddleIndex>,
        right: Vec<MiddleIndex>,
        middle: Vec<Lambda>,
    ) -> Result<Self, CatgraphError> {
        let middle_size = middle.len();
        for (leg, entries) in [
            (BoundaryLeg::Domain, left.as_slice()),
            (BoundaryLeg::Codomain, right.as_slice()),
        ] {
            for (position, &target) in entries.iter().enumerate() {
                if target >= middle_size {
                    return Err(CatgraphError::ConstructionIndexOutOfBounds {
                        leg,
                        position,
                        target,
                        target_len: middle_size,
                    });
                }
            }
        }
        Ok(Self::new_unchecked(left, right, middle))
    }

    /// Construct a cospan without checking that either leg lands inside the apex.
    ///
    /// The bounds invariant is the caller's responsibility; it is re-checked by
    /// a `debug_assert!` only, so a release build accepts an out-of-bounds leg
    /// and defers the failure to whatever indexes it later. Use this where the
    /// leg maps are correct **by construction** — composition results, identity
    /// and Frobenius generators, permutation builders, monoidal products — and
    /// [`new`](Self::new) everywhere data crosses a trust boundary.
    ///
    /// Mirrors [`Corel::new_unchecked`](crate::corel::Corel::new_unchecked) /
    /// [`Rel::new_unchecked`](crate::span::Rel::new_unchecked).
    #[must_use]
    pub fn new_unchecked(
        left: Vec<MiddleIndex>,
        right: Vec<MiddleIndex>,
        middle: Vec<Lambda>,
    ) -> Self {
        // Identity requires the leg to be a bijection onto the full middle set:
        // values must be [0, 1, ..., n-1] AND length must equal middle.len()
        let is_left_id = leg_is_identity(&left, middle.len());
        let is_right_id = leg_is_identity(&right, middle.len());
        let answer = Self {
            left,
            right,
            middle,
            is_left_id,
            is_right_id,
        };
        answer.assert_valid(false, false);
        answer
    }

    /// The cospan with empty domain, codomain, and middle set.
    #[must_use]
    pub fn empty() -> Self {
        Self::new_unchecked(vec![], vec![], vec![])
    }

    /// True when all three sets (left, right, middle) are empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.left.is_empty() && self.right.is_empty() && self.middle.is_empty()
    }

    #[must_use]
    pub fn left_to_middle(&self) -> &[MiddleIndex] {
        &self.left
    }

    #[must_use]
    pub fn right_to_middle(&self) -> &[MiddleIndex] {
        &self.right
    }

    #[must_use]
    pub fn middle(&self) -> &[Lambda] {
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

    /// Structural equality on the underlying `(left, right, middle)` triple.
    ///
    /// Equivalent to `PartialEq` on the public state, but `Cospan` intentionally
    /// does NOT derive `PartialEq` because the cached `is_left_id`/`is_right_id`
    /// flags can make structurally equal cospans compare unequal (the flags are
    /// updated by mutating constructors and may lag relative to the maps they
    /// summarize). Use this method when you need to compare cospans for shape
    /// equality across crate boundaries — Phase 6B (`catgraph-coalition`)
    /// snapshot-vs-expected assertions are the motivating consumer.
    #[must_use]
    pub fn structurally_equal(&self, other: &Self) -> bool {
        self.left == other.left && self.right == other.right && self.middle == other.middle
    }

    /// True if every middle (apex) vertex is in the image of the left or right leg.
    ///
    /// Corelations (dual of relations in `Rel`) require this property —
    /// see [`crate::corel::Corel`] (F&S 2018 Ex 6.64).
    #[must_use]
    pub fn is_jointly_surjective(&self) -> bool {
        let middle_size = self.middle.len();
        if middle_size == 0 {
            return true;
        }
        let mut covered = vec![false; middle_size];
        for &i in &self.left {
            covered[i] = true;
        }
        for &i in &self.right {
            covered[i] = true;
        }
        covered.iter().all(|c| *c)
    }

    /// Add a boundary node targeting an existing middle index. `Left` adds to domain, `Right` to codomain.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::ConstructionIndexOutOfBounds`] if the target
    /// index is at or beyond `middle.len()`; see
    /// [`add_boundary_node`](Self::add_boundary_node) for the fields and for the
    /// no-change-on-error guarantee.
    pub fn add_boundary_node_known_target(
        &mut self,
        new_arrow: Either<MiddleIndex, MiddleIndex>,
    ) -> Result<Either<LeftIndex, RightIndex>, CatgraphError> {
        self.add_boundary_node(new_arrow.bimap(|z| Left(z), |z| Left(z)))
    }

    /// Add a boundary node that creates a new middle vertex with the given label.
    /// `Left` adds to domain, `Right` to codomain.
    ///
    /// Infallible, and deliberately so: the new vertex is appended by this call,
    /// so the entry it produces is in bounds by construction. There is no
    /// precondition on the argument, so there is nothing for a `Result` to
    /// report — unlike
    /// [`add_boundary_node_known_target`](Self::add_boundary_node_known_target),
    /// whose caller-supplied index can miss the apex.
    ///
    /// It grows the **apex**, so it moves *both* identity flags: the leg it
    /// pushes to keeps its flag (leg and apex grow together), and the other
    /// leg's flag is cleared, because that leg is now strictly shorter than the
    /// apex and so cannot be the identity on it.
    pub fn add_boundary_node_unknown_target(
        &mut self,
        new_arrow: Either<Lambda, Lambda>,
    ) -> Either<LeftIndex, RightIndex> {
        // The `Right(label)` arms cannot fail; the index is minted here.
        self.add_boundary_node_unchecked(new_arrow.bimap(|z| Right(z), |z| Right(z)))
    }

    /// Add a boundary node mapping to a new or existing middle vertex.
    ///
    /// Outer `Left`/`Right` selects domain/codomain side.
    /// Inner `Left(idx)` targets existing middle; `Right(label)` creates a new middle vertex.
    ///
    /// The `Left(idx)` target is checked against the apex **in every build
    /// profile**, so this mutator cannot re-open the hole that
    /// [`new`](Self::new) closes: the legs of an in-bounds cospan stay in
    /// bounds across it. The identity flags stay honest in one direction only —
    /// neither flag is left as a stale `true`, but a `true` that this call
    /// clears is never restored, so a `false` can be conservative (as
    /// [`structurally_equal`](Self::structurally_equal) already warns).
    /// Internal callers pushing entries that are correct by construction — the
    /// pushout builder is the crate's only one — should use
    /// [`add_boundary_node_unchecked`](Self::add_boundary_node_unchecked), which
    /// costs nothing in release.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::ConstructionIndexOutOfBounds`] if an
    /// `Left(tgt_idx)` target is at or beyond `middle.len()`, naming the leg the
    /// node would have joined, the **position it would have occupied** in that
    /// leg (its current length), the out-of-range target, and the apex size. On
    /// `Err` the cospan is left exactly as it was — no leg entry is pushed and
    /// no identity flag is touched.
    pub fn add_boundary_node(
        &mut self,
        new_arrow: Either<MiddleIndexOrLambda<Lambda>, MiddleIndexOrLambda<Lambda>>,
    ) -> Result<Either<LeftIndex, RightIndex>, CatgraphError> {
        let bounds = match &new_arrow {
            Left(Left(tgt_idx)) => Some((BoundaryLeg::Domain, self.left.len(), *tgt_idx)),
            Right(Left(tgt_idx)) => Some((BoundaryLeg::Codomain, self.right.len(), *tgt_idx)),
            Left(Right(_)) | Right(Right(_)) => None,
        };
        if let Some((leg, position, target)) = bounds
            && target >= self.middle.len()
        {
            return Err(CatgraphError::ConstructionIndexOutOfBounds {
                leg,
                position,
                target,
                target_len: self.middle.len(),
            });
        }
        Ok(self.add_boundary_node_unchecked(new_arrow))
    }

    /// Add a boundary node without checking that an existing-target index lands
    /// inside the apex.
    ///
    /// The bounds invariant is the caller's responsibility; it is re-checked by
    /// a `debug_assert!` only, so a release build accepts an out-of-bounds leg
    /// entry and defers the failure to whatever indexes it later. Use this where
    /// the target is correct **by construction** — the pushout builder in
    /// [`compose_with_quotient`](Self::compose_with_quotient) is the crate's only
    /// such caller — and [`add_boundary_node`](Self::add_boundary_node)
    /// everywhere the index crosses a trust boundary.
    ///
    /// Mirrors [`new_unchecked`](Self::new_unchecked)'s posture: same invariant,
    /// same debug-only enforcement, same zero release cost.
    ///
    /// # Panics
    ///
    /// In debug builds only, panics if an `Left(tgt_idx)` target is at or beyond
    /// `middle.len()`.
    pub fn add_boundary_node_unchecked(
        &mut self,
        new_arrow: Either<MiddleIndexOrLambda<Lambda>, MiddleIndexOrLambda<Lambda>>,
    ) -> Either<LeftIndex, RightIndex> {
        match new_arrow {
            Left(tgt_info) => {
                match tgt_info {
                    Left(tgt_idx) => {
                        debug_assert!(
                            tgt_idx < self.middle.len(),
                            "A target for one of the left arrows was out of bounds"
                        );
                        self.left.push(tgt_idx);
                        // Exact, not conservative: a `true` flag meant
                        // `left.len() == middle.len()` on entry, and this arm
                        // grows the leg without the apex, so no value of
                        // `tgt_idx` can keep the leg an identity. (Pre-#289 the
                        // update was `&= left.len() - 1 == tgt_idx`, which kept
                        // a `true` for `tgt_idx == middle.len()` — an
                        // out-of-range entry, refused by `add_boundary_node`
                        // since #289; the review of that fix then showed the
                        // replacement conjunct was equivalent to this clear on
                        // every reachable state.)
                        self.is_left_id = false;
                    }
                    Right(new_lambda) => {
                        self.left.push(self.middle.len());
                        self.middle.push(new_lambda);
                        self.is_left_id &= self.left.len() == self.middle.len();
                        // This arm grows the **apex**, so it is not only the
                        // pushed leg's flag that moves: the codomain leg keeps
                        // its length while the apex gains a vertex, so a
                        // codomain identity stops being one. `is_right_id ==
                        // true` implied `right.len() == middle.len()` before
                        // the push, hence `right.len() < middle.len()` after —
                        // and no leg shorter than the apex can be the identity
                        // on it, so `false` is exact here, not conservative.
                        self.is_right_id = false;
                    }
                }
                Left(self.left.len() - 1)
            }
            Right(tgt_info) => {
                match tgt_info {
                    Left(tgt_idx) => {
                        debug_assert!(
                            tgt_idx < self.middle.len(),
                            "A target for one of the right arrows was out of bounds"
                        );
                        self.right.push(tgt_idx);
                        // The domain arm's comment above applies verbatim.
                        self.is_right_id = false;
                    }
                    Right(new_lambda) => {
                        self.right.push(self.middle.len());
                        self.middle.push(new_lambda);
                        self.is_right_id &= self.right.len() == self.middle.len();
                        // The mirror of the domain arm: the apex grew, the
                        // domain leg did not, so `is_left_id` is exactly
                        // `false` now.
                        self.is_left_id = false;
                    }
                }
                Right(self.right.len() - 1)
            }
        }
    }

    /// Remove a boundary node from domain (`Left`) or codomain (`Right`) via `swap_remove`.
    ///
    /// # Panics
    ///
    /// Panics — in **every** build profile — if `which_node` is at or beyond the
    /// length of the leg it names, the empty leg included. The empty case is why
    /// the check is explicit rather than left to `swap_remove`: the pre-#289
    /// identity-flag update read `leg.len() - 1` first, which underflowed on an
    /// empty leg (debug panic, release wrap to `usize::MAX`), so the failure
    /// used to be reported as arithmetic rather than as the violated
    /// precondition.
    pub fn delete_boundary_node(&mut self, which_node: Either<LeftIndex, RightIndex>) {
        match which_node {
            Left(z) => {
                assert!(
                    z < self.left.len(),
                    "delete_boundary_node: domain index {z} is out of bounds; the domain has {} port(s)",
                    self.left.len()
                );
                self.left.swap_remove(z);
                // Exact: a `true` flag meant `left.len() == middle.len()`, and
                // the leg just shrank without the apex — whichever port went.
                // (Pre-#289: `&= z == left.len() - 1`, which kept the flag when
                // the port removed was the last.)
                self.is_left_id = false;
            }
            Right(z) => {
                assert!(
                    z < self.right.len(),
                    "delete_boundary_node: codomain index {z} is out of bounds; the codomain has {} port(s)",
                    self.right.len()
                );
                self.right.swap_remove(z);
                self.is_right_id = false;
            }
        }
    }

    /// The apex vertex a boundary node maps to.
    ///
    /// # Panics
    ///
    /// Panics — in every build profile — if `node` is at or beyond the length of
    /// the leg it names. `context` names the calling method so the message says
    /// which precondition was violated rather than only that a slice was indexed
    /// out of range.
    fn boundary_target(&self, node: Either<LeftIndex, RightIndex>, context: &str) -> MiddleIndex {
        match node {
            Left(z) => {
                assert!(
                    z < self.left.len(),
                    "{context}: domain index {z} is out of bounds; the domain has {} port(s)",
                    self.left.len()
                );
                self.left[z]
            }
            Right(z) => {
                assert!(
                    z < self.right.len(),
                    "{context}: codomain index {z} is out of bounds; the codomain has {} port(s)",
                    self.right.len()
                );
                self.right[z]
            }
        }
    }

    /// True if both boundary nodes map to the same middle vertex.
    ///
    /// # Panics
    ///
    /// Panics — in every build profile — if either node is at or beyond the
    /// length of the leg it names.
    #[must_use]
    pub fn map_to_same(
        &self,
        node_1: Either<LeftIndex, RightIndex>,
        node_2: Either<LeftIndex, RightIndex>,
    ) -> bool {
        let mid_for_node_1 = self.boundary_target(node_1, "map_to_same");
        let mid_for_node_2 = self.boundary_target(node_2, "map_to_same");
        mid_for_node_1 == mid_for_node_2
    }

    /// Merge the middle vertices that two boundary nodes map to.
    ///
    /// No-op if they already share a vertex. Warns and makes no change if their labels differ.
    ///
    /// A merge shrinks the apex by one while both legs keep their length, so a
    /// leg that was the identity on the old apex is not one on the new — and a
    /// leg that was *not* may now be (one port per surviving vertex, in
    /// order). Since [#289](https://github.com/sustia-llc/catgraph/issues/289)
    /// both identity flags are **recomputed** from the merged value: exact in
    /// both directions on the merge path, unlike the `&=` updates elsewhere,
    /// which can only clear. The two no-op arms (same vertex; label mismatch)
    /// return before it and leave the cache as it was, so a `false` another
    /// mutator left conservatively is *not* refreshed by a no-op merge. Before
    /// #289 the flags were not touched, and the stale `true` reached
    /// `perform_pushout`'s fast path as a silent wrong composition. The other
    /// direction is observable too: a merge can turn a flag **on**, and a
    /// composite built after it may then come back with a different
    /// (isomorphic) apex order than the union-find path gives for the same
    /// operands — `structurally_equal` sees the difference, `canonical_form`
    /// does not.
    ///
    /// The leg remap after the `swap_remove` keeps the vertex node 1 maps to
    /// even when that vertex is the **last** apex index and has just been moved
    /// into node 2's slot. Pre-#289 it wrote node 1's old index there, which
    /// put `middle.len()` into both legs and left the two ports unmerged
    /// (measured: `connect_pair(Left(1), Left(0))` on
    /// `new([0, 1], [0, 1], ['a', 'a'])` gave `left == [1, 0]` over a 1-vertex
    /// apex, `map_to_same` false) — silently in every profile, since this
    /// method runs no `assert_valid`.
    ///
    /// # Panics
    ///
    /// Panics — in every build profile — if either node is at or beyond the
    /// length of the leg it names. It also indexes the apex with the two leg
    /// entries, so a cospan built through
    /// [`new_unchecked`](Self::new_unchecked) or
    /// [`add_boundary_node_unchecked`](Self::add_boundary_node_unchecked) in
    /// violation of the bounds invariant panics here on the raw index; that is
    /// the deferral those methods document, not a precondition of this one.
    pub fn connect_pair(
        &mut self,
        node_1: Either<LeftIndex, RightIndex>,
        node_2: Either<LeftIndex, RightIndex>,
    ) {
        let mid_for_node_1 = self.boundary_target(node_1, "connect_pair");
        let mid_for_node_2 = self.boundary_target(node_2, "connect_pair");
        if mid_for_node_1 == mid_for_node_2 {
            return;
        }
        let type_ = self.middle[mid_for_node_1];
        if type_ != self.middle[mid_for_node_2] {
            warn!("Incompatible types. No change made.");
            return;
        }
        let _ = self.middle.swap_remove(mid_for_node_2);
        let old_last = self.middle.len();
        // `swap_remove` moved the vertex that was at `old_last` into slot
        // `mid_for_node_2` — unless `mid_for_node_2` WAS `old_last`, in which
        // case nothing moved and the second branch below is unreachable (the
        // first one already caught every entry equal to it). If the moved
        // vertex is node 1's, the survivor now lives at `mid_for_node_2`, not
        // at its old index; writing the old index — as this did before #289's
        // review — put `old_last == middle.len()` into both legs.
        let keep = if mid_for_node_1 == old_last {
            mid_for_node_2
        } else {
            mid_for_node_1
        };
        for leg in [&mut self.left, &mut self.right] {
            for v in leg.iter_mut() {
                if *v == mid_for_node_2 {
                    *v = keep;
                } else if *v == old_last {
                    *v = mid_for_node_2;
                }
            }
        }
        // #289: the apex just lost a vertex while both legs kept their length,
        // so neither cached flag can be trusted. Recompute both from the
        // definition rather than `&=` them — exact in both directions, and what
        // keeps `perform_pushout`'s fast path off a merged apex. Measured before
        // this existed: `Cospan::new(vec![0, 1], vec![0, 1], vec!['a', 'a'])`
        // merged at `Left(0)` / `Left(1)` kept `(true, true)` and composed with
        // `identity(&['a', 'a'])` to `right == [0, 1]` over a 2-vertex apex,
        // where the reference composition gives `right == [0, 0]` over one.
        self.is_left_id = leg_is_identity(&self.left, self.middle.len());
        self.is_right_id = leg_is_identity(&self.right, self.middle.len());
    }

    /// Append a new vertex to the middle set with the given label. Returns its index.
    pub fn add_middle(&mut self, new_middle: Lambda) -> MiddleIndex {
        self.middle.push(new_middle);
        self.is_left_id = false;
        self.is_right_id = false;
        self.middle.len() - 1
    }

    /// Apply a function to all middle vertex labels, producing a new cospan.
    pub fn map<F, Mu>(&self, f: F) -> Cospan<Mu>
    where
        F: Fn(Lambda) -> Mu,
        Mu: Sized + Eq + Copy + Debug,
    {
        // Correct by construction: the legs are copied verbatim and `f` is
        // applied pointwise, so the apex keeps its length and every leg entry
        // stays in bounds.
        Cospan::new_unchecked(
            self.left.clone(),
            self.right.clone(),
            self.middle.iter().map(|l| f(*l)).collect(),
        )
    }
}

/// Fold-compose a chain of cospans into a single composite cospan.
///
/// Given a sequence `c_0, c_1, ..., c_{n-1}` of `Cospan<Lambda>` values,
/// returns `c_0 ; c_1 ; ... ; c_{n-1}` by successive pushout composition.
/// The first cospan in the iterator seeds the accumulator; each subsequent
/// cospan must be composable (its domain must match the running codomain),
/// otherwise composition fails at the first mismatch.
///
/// This is the canonical way to build a composite cospan from a chain, and
/// is used (for example) by temporal / interval-decomposed cospan sequences
/// in downstream consumers.
///
/// # Errors
///
/// - `CatgraphError::Composition { message: "empty cospan chain" }` if the
///   iterator yields no cospans.
/// - Any `CatgraphError` returned by an intermediate `Cospan::compose` call
///   when adjacent cospans' interfaces don't line up.
pub fn compose_chain<Lambda, I>(cospans: I) -> Result<Cospan<Lambda>, CatgraphError>
where
    Lambda: Eq + Sized + Copy + Debug,
    I: IntoIterator<Item = Cospan<Lambda>>,
{
    let mut iter = cospans.into_iter();
    let first = iter.next().ok_or_else(|| CatgraphError::Composition {
        message: "empty cospan chain".to_string(),
    })?;
    iter.try_fold(first, |acc, c| acc.compose(&c))
}

impl<Lambda> HasIdentity<Vec<Lambda>> for Cospan<Lambda>
where
    Lambda: Eq + Copy + Debug,
{
    #[allow(clippy::implicit_clone)]
    fn identity(types: &Vec<Lambda>) -> Self {
        let num_types = types.len();
        Self {
            left: (0..num_types).collect(),
            right: (0..num_types).collect(),
            middle: types.to_vec(),
            is_left_id: true,
            is_right_id: true,
        }
    }
}

impl<Lambda> Monoidal for Cospan<Lambda>
where
    Lambda: Eq + Sized + Copy + Debug,
{
    fn monoidal(&mut self, mut other: Self) {
        let middle_shift = self.middle.len();
        other.left.iter_mut().for_each(|v| *v += middle_shift);
        other.right.iter_mut().for_each(|v| *v += middle_shift);
        self.left.extend(other.left);
        self.right.extend(other.right);
        self.middle.extend(other.middle);
        self.is_left_id &= other.is_left_id;
        self.is_right_id &= other.is_right_id;
    }
}

impl<Lambda> Cospan<Lambda>
where
    Lambda: Eq + Sized + Copy + Debug,
{
    /// Pushout composition returning both the composed cospan and the
    /// `old_apex_index → new_apex_index` quotient map produced by the
    /// union-find coequalizer.
    ///
    /// Indexing convention for the returned `Vec<usize>`:
    /// - positions `0..self.middle.len()` map `self`'s middle indices;
    /// - positions `self.middle.len()..self.middle.len()+other.middle.len()`
    ///   map `other`'s middle indices;
    /// - both ranges map into `0..composed.middle.len()`.
    ///
    /// Callers that don't need the quotient should use
    /// [`Composable::compose`], which
    /// wraps this and discards the map.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] if the right boundary of `self`
    /// does not type-match the left boundary of `other`, or if the internal
    /// union-find pushout fails.
    pub fn compose_with_quotient(&self, other: &Self) -> Result<(Self, Vec<usize>), CatgraphError> {
        self.composable(other)?;
        let (pushout_target, left_to_pushout, right_to_pushout, representative) =
            perform_pushout::<union_find::QuickUnionUf<union_find::UnionBySize>>(
                &self.right,
                self.middle.len(),
                self.is_right_id,
                &other.left,
                other.middle.len(),
                other.is_left_id,
            )
            .map_err(|e| CatgraphError::Composition {
                message: e.to_string(),
            })?;
        // Correct by construction: all three vectors start empty (they are only
        // pre-sized), and every subsequent push goes through `add_middle` /
        // `add_boundary_node_unchecked`, which maintain the bounds invariant —
        // the apex is filled from `representative` before any boundary node is
        // added, and every `*_to_pushout` entry is a class number below
        // `pushout_target`.
        let mut composition = Self::new_unchecked(
            Vec::with_capacity(self.left.len()),
            Vec::with_capacity(other.right.len()),
            Vec::with_capacity(pushout_target),
        );
        for repr in representative {
            composition.add_middle(match repr {
                Left(z) => self.middle[z],
                Right(z) => other.middle[z],
            });
        }
        for target_in_self_middle in &self.left {
            let target_in_pushout = left_to_pushout[*target_in_self_middle];
            composition.add_boundary_node_unchecked(Left(Left(target_in_pushout)));
        }
        for target_in_other_middle in &other.right {
            let target_in_pushout = right_to_pushout[*target_in_other_middle];
            composition.add_boundary_node_unchecked(Right(Left(target_in_pushout)));
        }
        let mut quotient = left_to_pushout;
        quotient.extend(right_to_pushout);
        Ok((composition, quotient))
    }
}

impl<Lambda> Composable<Vec<Lambda>> for Cospan<Lambda>
where
    Lambda: Eq + Sized + Copy + Debug,
{
    fn composable(&self, other: &Self) -> Result<(), CatgraphError> {
        let self_interface = self.right.iter().map(|mid| self.middle[*mid]);
        let other_interface = other.left.iter().map(|mid| other.middle[*mid]);

        crate::utils::same_labels_check(self_interface, other_interface)
            .map_err(|message| CatgraphError::Composition { message })
    }

    fn compose(&self, other: &Self) -> Result<Self, CatgraphError> {
        self.compose_with_quotient(other).map(|(c, _)| c)
    }

    fn domain(&self) -> Vec<Lambda> {
        self.left.iter().map(|mid| self.middle[*mid]).collect()
    }

    fn codomain(&self) -> Vec<Lambda> {
        self.right.iter().map(|mid| self.middle[*mid]).collect()
    }
}

impl<Lambda> MonoidalMorphism<Vec<Lambda>> for Cospan<Lambda> where Lambda: Eq + Sized + Copy + Debug
{}

impl<Lambda> SymmetricMonoidalMorphism<Lambda> for Cospan<Lambda>
where
    Lambda: Eq + Sized + Copy + Debug,
{
    /// Splices the braiding for `p` onto the right leg (`of_right_leg`) or the
    /// braiding for `p⁻¹` onto the left leg — see the trait's contract.
    ///
    /// A leg vector is a function `slot ↦ apex vertex`, so moving the wire at
    /// slot `i` to slot `p.apply(i)` is post-composition of that function with
    /// `p⁻¹`: `new[k] == old[p.inv().apply(k)]`, which is exactly
    /// `in_place_permute(leg, &p.inv())`. The same expression serves both legs
    /// because the asymmetry the trait documents lives in the *wiring*, not in
    /// the relabelling — for `of_right_leg = false` this leg vector is the
    /// composite's, i.e. `old ∘ p⁻¹`, which is what precomposing `β(p⁻¹)`
    /// produces.
    ///
    /// ⚠ **Breaking at #258.** This used to pass `p` rather than `p.inv()`, so
    /// it realized `β(p⁻¹)` on the codomain where `MatR`/`PropExpr` realize
    /// `β(p)`. Nothing saw it: no test drove one permutation through two
    /// carriers, and within `Cospan` an inverted braiding is still a
    /// consistent braiding.
    fn permute_side(&mut self, p: &Permutation, of_right_leg: bool) {
        in_place_permute(
            if of_right_leg {
                self.is_right_id = false;
                &mut self.right
            } else {
                self.is_left_id = false;
                &mut self.left
            },
            &p.inv(),
        );
    }

    /// `domain() == types`, `codomain()[k] == types[p.inv().apply(k)]`.
    ///
    /// The apex is `types` itself and the left leg is the identity, so a
    /// domain wire `i` sits on apex vertex `i`; the right leg is
    /// `p.inv().permute(0..n)`, so codomain wire `k` sits on apex vertex
    /// `p.inv().apply(k)`. Domain `i` and codomain `k` therefore meet exactly
    /// when `k == p.apply(i)` — the wiring the trait specifies. The inverse in
    /// the *leg vector* is what makes the wiring non-inverted; reading `p.inv()`
    /// off this line as "this impl realizes `p⁻¹`" is the mistake #258 was
    /// filed about.
    ///
    /// The same placement is what makes `from(p₁) ; from(p₂) == from(p₁ ; p₂)`
    /// hold, with `;` cospan composition on the left and permutation
    /// composition on the right.
    fn from_permutation_on_domain(p: Permutation, types: &[Lambda]) -> Result<Self, CatgraphError> {
        let num_types = types.len();
        if p.len() != num_types {
            return Err(CatgraphError::CompositionSizeMismatch {
                expected: num_types,
                actual: p.len(),
            });
        }
        let id_temp = (0..num_types).collect::<Vec<usize>>();
        Ok(Self {
            left: id_temp.clone(),
            right: p.inv().permute(&id_temp),
            middle: types.to_vec(),
            is_left_id: true,
            is_right_id: false,
        })
    }

    /// `codomain() == types`, `domain()[i] == types[p.apply(i)]`.
    ///
    /// Mirror of [`from_permutation_on_domain`](Self::from_permutation_on_domain):
    /// the right leg is the identity and the left leg is `p.permute(0..n)`, so
    /// domain wire `i` sits on apex vertex `p.apply(i)` and codomain wire `k`
    /// on apex vertex `k` — again meeting exactly when `k == p.apply(i)`.
    fn from_permutation_on_codomain(
        p: Permutation,
        types: &[Lambda],
    ) -> Result<Self, CatgraphError> {
        let num_types = types.len();
        if p.len() != num_types {
            return Err(CatgraphError::CompositionSizeMismatch {
                expected: num_types,
                actual: p.len(),
            });
        }
        let id_temp = (0..num_types).collect::<Vec<usize>>();
        Ok(Self {
            left: p.permute(&id_temp),
            right: id_temp,
            middle: types.to_vec(),
            is_left_id: false,
            is_right_id: true,
        })
    }
}

/// `(pushout_size, left_reindex, right_reindex, representatives)`.
type PushoutResult = (
    MiddleIndex,
    Vec<MiddleIndex>,
    Vec<MiddleIndex>,
    Vec<Either<LeftIndex, RightIndex>>,
);

/// Compute the pushout of two finite-set leg maps via union-find.
///
/// Fast-paths when either leg is an identity. Returns reindexing maps and
/// a representative (Left or Right original index) for each equivalence class.
fn perform_pushout<T>(
    left_leg: &[LeftIndex],
    left_leg_max_target: LeftIndex,
    left_leg_id: bool,
    right_leg: &[RightIndex],
    right_leg_max_target: RightIndex,
    right_leg_id: bool,
) -> Result<PushoutResult, &'static str>
where
    T: UnionFind<UnionBySize>,
{
    if left_leg.len() != right_leg.len() {
        return Err("Mismatch in cardinalities of common interface");
    }
    if left_leg_id {
        let pushout_target = right_leg_max_target;
        let left_to_pushout = right_leg.to_vec();
        let right_to_pushout = (0..right_leg_max_target).collect::<FinSetMap>();
        let representative = (0..right_leg_max_target).map(Right);
        return Ok((
            pushout_target,
            left_to_pushout,
            right_to_pushout,
            representative.collect(),
        ));
    }
    if right_leg_id {
        let pushout_target = left_leg_max_target;
        let right_to_pushout = left_leg.to_vec();
        let left_to_pushout = (0..left_leg_max_target).collect::<FinSetMap>();
        let representative = (0..left_leg_max_target).map(Left);
        return Ok((
            pushout_target,
            left_to_pushout,
            right_to_pushout,
            representative.collect(),
        ));
    }

    let mut uf = T::new(left_leg_max_target + right_leg_max_target);
    for idx in 0..left_leg.len() {
        let left_z = left_leg[idx];
        let right_z = right_leg[idx] + left_leg_max_target;
        uf.union(left_z, right_z);
    }
    let mut set_to_part_num = HashMap::new();
    let mut current_set_number = 0;
    let mut left_to_pushout: Vec<MiddleIndex> = Vec::with_capacity(left_leg_max_target);
    let expected_num_sets = uf.size();
    let mut representative = Vec::with_capacity(expected_num_sets);
    for idx in 0..left_leg_max_target {
        let which_set = uf.find(idx);
        if let Some(z) = set_to_part_num.get(&which_set) {
            left_to_pushout.push(*z);
        } else {
            set_to_part_num.insert(which_set, current_set_number);
            left_to_pushout.push(current_set_number);
            current_set_number += 1;
            representative.push(Left(idx));
        }
    }
    let mut right_to_pushout: Vec<MiddleIndex> = Vec::with_capacity(right_leg_max_target);
    for idx in 0..right_leg_max_target {
        let which_set = uf.find(idx + left_leg_max_target);
        if let Some(z) = set_to_part_num.get(&which_set) {
            right_to_pushout.push(*z);
        } else {
            set_to_part_num.insert(which_set, current_set_number);
            right_to_pushout.push(current_set_number);
            current_set_number += 1;
            representative.push(Right(idx));
        }
    }
    let pushout_target = current_set_number;
    Ok((
        pushout_target,
        left_to_pushout,
        right_to_pushout,
        representative,
    ))
}

#[cfg(test)]
mod test {
    #[allow(unused_imports)]
    use crate::{
        category::{Composable, HasIdentity},
        monoidal::SymmetricMonoidalMorphism,
        monoidal::{Monoidal, MonoidalMorphism},
    };
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    // ---- #256: `new` validates in EVERY profile, `new_unchecked` does not ----

    /// `Cospan::new` refuses an out-of-bounds leg entry on either side, and says
    /// which leg, which position, which target, and how big the apex was.
    ///
    /// The check under test is the *unconditional* one in `new`, not the
    /// `debug_assert!` in `assert_valid` — this test therefore states the
    /// release-build behaviour and must be run under `--release` too.
    #[test]
    fn cospan_new_rejects_out_of_bounds_leg_entries() {
        use super::Cospan;
        use crate::errors::{BoundaryLeg, CatgraphError};

        // Domain leg: entry 1 targets apex index 2, apex has 2 vertices.
        let err = Cospan::<char>::new(vec![0, 2], vec![1], vec!['a', 'b'])
            .expect_err("left leg index 2 is out of bounds for a 2-vertex apex");
        assert_eq!(
            err,
            CatgraphError::ConstructionIndexOutOfBounds {
                leg: BoundaryLeg::Domain,
                position: 1,
                target: 2,
                target_len: 2,
            }
        );
        assert_eq!(
            err.to_string(),
            "construction error: domain leg entry 1 targets index 2, but the target set has 2 element(s)"
        );

        // Codomain leg: entry 1 targets apex index 7, apex has 2 vertices.
        let err = Cospan::<char>::new(vec![0], vec![1, 7], vec!['a', 'b'])
            .expect_err("right leg index 7 is out of bounds for a 2-vertex apex");
        assert_eq!(
            err,
            CatgraphError::ConstructionIndexOutOfBounds {
                leg: BoundaryLeg::Codomain,
                position: 1,
                target: 7,
                target_len: 2,
            }
        );

        // Both legs bad: the domain leg is scanned first, so it is reported.
        let err = Cospan::<char>::new(vec![5], vec![9], vec!['a'])
            .expect_err("both legs are out of bounds");
        assert_eq!(
            err,
            CatgraphError::ConstructionIndexOutOfBounds {
                leg: BoundaryLeg::Domain,
                position: 0,
                target: 5,
                target_len: 1,
            },
            "the domain leg must win the race, otherwise the reported leg is not deterministic"
        );

        // The valid neighbour of the first case still constructs.
        assert!(Cospan::<char>::new(vec![0, 1], vec![1], vec!['a', 'b']).is_ok());
    }

    /// `new_unchecked` keeps the pre-#256 contract: the bounds invariant is the
    /// caller's, checked by `debug_assert!` only. Release-only, because in a
    /// debug build that `debug_assert!` fires — which IS the contract.
    #[cfg(not(debug_assertions))]
    #[test]
    fn cospan_new_unchecked_accepts_what_new_refuses() {
        use super::Cospan;
        let bad = Cospan::<char>::new_unchecked(vec![0, 2], vec![1], vec!['a', 'b']);
        assert_eq!(bad.left_to_middle(), &[0, 2]);
        assert_eq!(bad.middle().len(), 2);
        assert!(
            Cospan::<char>::new(vec![0, 2], vec![1], vec!['a', 'b']).is_err(),
            "the same input must be refused by the checked constructor"
        );
    }

    #[test]
    fn empty_cospan() {
        use super::Cospan;
        let empty_cospan = Cospan::<u32>::empty();
        assert!(empty_cospan.is_empty());
    }

    #[test]
    fn compose_chain_empty_is_error() {
        use super::{Cospan, compose_chain};
        let empty: Vec<Cospan<u32>> = vec![];
        let result = compose_chain(empty);
        assert!(result.is_err(), "empty chain should return Err");
    }

    #[test]
    fn compose_chain_single_is_identity_on_input() {
        use super::{Cospan, compose_chain};
        let c = Cospan::new(vec![0], vec![1], vec![10u32, 20]).unwrap();
        let result = compose_chain(vec![c.clone()]).unwrap();
        assert_eq!(result.domain(), c.domain());
        assert_eq!(result.codomain(), c.codomain());
        assert_eq!(result.middle(), c.middle());
    }

    #[test]
    fn compose_chain_pair_matches_manual_compose() {
        use super::{Cospan, compose_chain};
        // Three composable u32-typed cospans representing a contiguous
        // interval chain [0,1] ; [1,2] ; [2,3]. Each cospan has the
        // interval structure used by stokes: left=[0], right=[1], middle=[t_i, t_{i+1}].
        let c0 = Cospan::new(vec![0], vec![1], vec![0u32, 1]).unwrap();
        let c1 = Cospan::new(vec![0], vec![1], vec![1u32, 2]).unwrap();
        let c2 = Cospan::new(vec![0], vec![1], vec![2u32, 3]).unwrap();

        let folded = compose_chain(vec![c0.clone(), c1.clone(), c2.clone()]).unwrap();
        let manual = c0.compose(&c1).unwrap().compose(&c2).unwrap();

        assert_eq!(folded.domain(), manual.domain());
        assert_eq!(folded.codomain(), manual.codomain());
        assert_eq!(folded.middle(), manual.middle());
        // Domain should be [0u32], codomain [3u32]
        assert_eq!(folded.domain(), vec![0u32]);
        assert_eq!(folded.codomain(), vec![3u32]);
    }

    #[test]
    fn compose_chain_propagates_mismatch_error() {
        use super::{Cospan, compose_chain};
        // Second cospan's left boundary type [5] doesn't match first's right [2].
        let c0 = Cospan::new(vec![0], vec![1], vec![1u32, 2]).unwrap();
        let c1 = Cospan::new(vec![0], vec![1], vec![5u32, 6]).unwrap();
        let result = compose_chain(vec![c0, c1]);
        assert!(result.is_err(), "mismatched chain should return Err");
    }

    #[test]
    fn left_only_cospan() {
        use super::Cospan;
        use either::{Left, Right};
        let mut cospan = Cospan::<u32>::empty();
        cospan.add_boundary_node(Left(Right(1))).unwrap();
        cospan.add_boundary_node(Left(Right(2))).unwrap();
        cospan.add_boundary_node(Left(Right(3))).unwrap();
        cospan.add_boundary_node(Left(Left(1))).unwrap();
        assert_eq!(cospan.left.len(), 4);
        assert_eq!(cospan.right.len(), 0);
        assert_eq!(cospan.middle.len(), 3);
        assert_eq!(cospan.left, vec![0, 1, 2, 1]);
        assert_eq!(cospan.middle, vec![1, 2, 3]);
    }

    #[test]
    fn permutatation_manual() {
        use super::Cospan;
        // A literal, not an RNG (#232): the five bools are arbitrary payload the
        // assertions carry through, so entropy bought nothing — and a visible
        // mixed pattern guarantees the middle comparison stays discriminating
        // (a seeded stream could silently degenerate to all-equal).
        let whatever_types = vec![true, false, true, false, true];
        let mut full_types: Vec<bool> = vec![true, true];
        full_types.extend(whatever_types.clone());
        let cospan = Cospan::<bool>::new((0..=6).collect(), vec![1, 0, 2, 3], full_types).unwrap();
        assert!(cospan.is_left_id);
        assert!(!cospan.is_right_id);
        let cospan2 = Cospan::<bool>::new(
            vec![0, 1, 2, 3],
            vec![1, 0, 2, 3],
            vec![true, true, whatever_types[0], whatever_types[1]],
        )
        .unwrap();
        let res = cospan.compose(&cospan2);
        let mut exp_middle = vec![true, true];
        exp_middle.extend(whatever_types.clone());
        match res {
            Ok(real_res) => {
                assert_eq!(real_res.left, (0..=6).collect::<Vec<_>>());
                assert_eq!(real_res.right, vec![0, 1, 2, 3]);
                assert_eq!(real_res.middle, exp_middle);
            }
            Err(e) => {
                panic!("Could not compose simple example\n{e:?}")
            }
        }
    }

    #[test]
    fn permutatation_manual_labelled() {
        use super::Cospan;
        use permutations::Permutation;
        #[derive(Debug, PartialEq, Eq, Clone, Copy)]
        enum Color {
            Red,
            Green,
            Blue,
        }
        let cospan = Cospan::<Color>::from_permutation_on_domain(
            Permutation::rotation_left(3, 1),
            &[Color::Red, Color::Green, Color::Blue],
        )
        .unwrap();
        let cospan_2 = Cospan::<Color>::from_permutation_on_domain(
            Permutation::rotation_left(3, 2),
            &[Color::Blue, Color::Red, Color::Green],
        )
        .unwrap();
        let mid_interface_1 = cospan.codomain();
        let mid_interface_2 = cospan_2.domain();
        let comp = cospan.compose(&cospan_2);
        match comp {
            Ok(real_res) => {
                let expected_res = Cospan::identity(&vec![Color::Red, Color::Green, Color::Blue]);
                assert_eq!(expected_res.left, real_res.left);
                assert_eq!(expected_res.right, real_res.right);
                assert_eq!(expected_res.middle, real_res.middle);
            }
            Err(e) => {
                panic!(
                    "Could not compose simple example because {mid_interface_1:?} did not match {mid_interface_2:?}\n{e:?}"
                );
            }
        }
        let cospan = Cospan::<Color>::from_permutation_on_codomain(
            Permutation::rotation_left(3, 1),
            &[Color::Red, Color::Green, Color::Blue],
        )
        .unwrap();
        let cospan_2 = Cospan::<Color>::from_permutation_on_codomain(
            Permutation::rotation_left(3, 2),
            &[Color::Green, Color::Blue, Color::Red],
        )
        .unwrap();
        let mid_interface_1 = cospan.codomain();
        let mid_interface_2 = cospan_2.domain();
        let comp = cospan.compose(&cospan_2);
        match comp {
            Ok(real_res) => {
                let expected_res = Cospan::identity(&vec![Color::Green, Color::Blue, Color::Red]);
                assert_eq!(expected_res.left, real_res.left);
                assert_eq!(expected_res.right, real_res.right);
                assert_eq!(expected_res.middle, real_res.middle);
            }
            Err(e) => {
                panic!(
                    "Could not compose simple example because {mid_interface_1:?} did not match {mid_interface_2:?}\n{e:?}"
                );
            }
        }
    }

    #[test]
    fn non_square_composition() {
        // Compose two cospans where domain size != codomain size
        // f: 2 -> 3 (domain=2, codomain=3)
        // g: 3 -> 1 (domain=3, codomain=1)
        // result: 2 -> 1
        use super::Cospan;

        // f: domain {0,1} -> middle {A,B,C} -> codomain {0,1,2}
        // left=[0,1], right=[0,1,2], middle=[10,20,30]
        // domain labels: [10,20], codomain labels: [10,20,30]
        let f = Cospan::<u32>::new(vec![0, 1], vec![0, 1, 2], vec![10, 20, 30]).unwrap();
        assert_eq!(f.domain(), vec![10, 20]);
        assert_eq!(f.codomain(), vec![10, 20, 30]);

        // g: domain {0,1,2} -> middle {X,Y,Z} -> codomain {0}
        // For composability, g.domain() must match f.codomain() = [10,20,30]
        // left=[0,1,2], right=[0], middle=[10,20,30]
        // All three codomain nodes of f map to separate middle nodes in g,
        // but the single codomain node of g maps to middle[0].
        let g = Cospan::<u32>::new(vec![0, 1, 2], vec![0], vec![10, 20, 30]).unwrap();
        assert_eq!(g.domain(), vec![10, 20, 30]);
        assert_eq!(g.codomain(), vec![10]);

        let result = f.compose(&g).expect("composition should succeed");
        assert_eq!(result.left.len(), 2, "result domain size should be 2");
        assert_eq!(result.right.len(), 1, "result codomain size should be 1");
        assert_eq!(result.domain().len(), 2);
        assert_eq!(result.codomain().len(), 1);
    }

    #[test]
    fn composition_error_size_mismatch() {
        // Compose two cospans with incompatible codomain/domain sizes
        use super::Cospan;

        // f: codomain has 2 elements
        let f = Cospan::<u32>::new(vec![0, 1], vec![0, 1], vec![10, 20]).unwrap();
        // g: domain has 3 elements (mismatch with f's codomain)
        let g = Cospan::<u32>::new(vec![0, 1, 2], vec![0], vec![10, 20, 30]).unwrap();

        let result = f.compose(&g);
        assert!(
            result.is_err(),
            "should fail: codomain size 2 != domain size 3"
        );
        let err = result.unwrap_err();
        match err {
            crate::errors::CatgraphError::Composition { message } => {
                assert!(
                    message.contains("Mismatch") || message.contains("cardinalities"),
                    "error should mention mismatch: {message}"
                );
            }
            other => panic!("expected Composition error, got {other:?}"),
        }
    }

    #[test]
    fn composition_error_label_mismatch() {
        // Compose two cospans where sizes match but labels differ
        use super::Cospan;

        // f: codomain labels = [10, 20]
        let f = Cospan::<u32>::new(vec![0, 1], vec![0, 1], vec![10, 20]).unwrap();
        // g: domain labels = [10, 30] (second label differs)
        let g = Cospan::<u32>::new(vec![0, 1], vec![0], vec![10, 30]).unwrap();

        let result = f.compose(&g);
        assert!(result.is_err(), "should fail: label mismatch at index 1");
        let err = result.unwrap_err();
        match err {
            crate::errors::CatgraphError::Composition { message } => {
                assert!(
                    message.contains("Mismatch") || message.contains("labels"),
                    "error should mention label mismatch: {message}"
                );
            }
            other => panic!("expected Composition error, got {other:?}"),
        }
    }

    #[test]
    fn identity_composition_roundtrip_left() {
        // id ; f = f (composing identity on the left yields equivalent result)
        use super::Cospan;

        let f = Cospan::<u32>::new(vec![0, 1, 2], vec![0, 1], vec![10, 20, 30]).unwrap();
        let dom = f.domain();
        let id_left = Cospan::<u32>::identity(&dom);

        let result = id_left.compose(&f).expect("id ; f should compose");
        assert_eq!(result.domain(), f.domain());
        assert_eq!(result.codomain(), f.codomain());
        assert_eq!(result.left.len(), f.left.len());
        assert_eq!(result.right.len(), f.right.len());
    }

    #[test]
    fn identity_composition_roundtrip_right() {
        // f ; id = f (composing identity on the right yields equivalent result)
        use super::Cospan;

        // Use a cospan where the right leg is NOT identity (right=[1,0])
        // so the pushout fast path for left_leg_id is not triggered.
        // domain=2, codomain=2, middle has nodes for both sides.
        let f = Cospan::<u32>::new(vec![0, 1], vec![1, 0], vec![10, 20]).unwrap();
        let cod = f.codomain();
        let id_right = Cospan::<u32>::identity(&cod);

        let result = f.compose(&id_right).expect("f ; id should compose");
        assert_eq!(result.domain(), f.domain());
        assert_eq!(result.codomain(), f.codomain());
        assert_eq!(result.left.len(), f.left.len());
        assert_eq!(result.right.len(), f.right.len());
    }

    #[test]
    fn identity_compose_both_sides() {
        // id ; f ; id = f
        use super::Cospan;

        let f = Cospan::<u32>::new(vec![0, 1], vec![0, 2], vec![10, 20, 30]).unwrap();
        let dom = f.domain();
        let cod = f.codomain();
        let id_left = Cospan::<u32>::identity(&dom);
        let id_right = Cospan::<u32>::identity(&cod);

        let step1 = id_left.compose(&f).expect("id ; f should compose");
        let result = step1
            .compose(&id_right)
            .expect("(id ; f) ; id should compose");
        assert_eq!(result.domain(), f.domain());
        assert_eq!(result.codomain(), f.codomain());
        assert_eq!(result.left.len(), f.left.len());
        assert_eq!(result.right.len(), f.right.len());
    }

    #[test]
    fn monoidal_product_sizes() {
        // Monoidal product of two cospans should combine domain/codomain
        use super::Cospan;

        let a = Cospan::<u32>::new(vec![0, 1], vec![0], vec![10, 20]).unwrap();
        let b = Cospan::<u32>::new(vec![0], vec![0, 1], vec![30, 40]).unwrap();

        let mut product = a.clone();
        product.monoidal(b.clone());

        // domain size = a.domain + b.domain = 2 + 1 = 3
        assert_eq!(product.left.len(), 3);
        // codomain size = a.codomain + b.codomain = 1 + 2 = 3
        assert_eq!(product.right.len(), 3);
        // middle size = a.middle + b.middle = 2 + 2 = 4
        assert_eq!(product.middle.len(), 4);

        // domain labels are concatenation
        assert_eq!(product.domain(), vec![10, 20, 30]);
        // codomain labels are concatenation
        assert_eq!(product.codomain(), vec![10, 30, 40]);
    }

    #[test]
    fn monoidal_product_with_empty() {
        // Monoidal product with empty cospan is a no-op
        use super::Cospan;

        let a = Cospan::<u32>::new(vec![0, 1], vec![0, 1], vec![10, 20]).unwrap();
        let empty = Cospan::<u32>::empty();

        let mut product = a.clone();
        product.monoidal(empty);

        assert_eq!(product.left, a.left);
        assert_eq!(product.right, a.right);
        assert_eq!(product.middle, a.middle);
    }

    #[test]
    fn monoidal_product_validity() {
        // After monoidal product, the result should pass validity checks
        use super::Cospan;

        let a = Cospan::<u32>::new(vec![0, 1, 0], vec![1, 0], vec![10, 20]).unwrap();
        let b = Cospan::<u32>::new(vec![0], vec![0, 1, 2], vec![30, 40, 50]).unwrap();

        let mut product = a.clone();
        product.monoidal(b.clone());
        // Should not panic
        product.assert_valid(false, true);
    }

    #[test]
    fn permutation_automatic() {
        use super::Cospan;
        use crate::utils::{in_place_permute, rand_perm};
        use rand::{distr::Uniform, prelude::Distribution};
        let n_max = 10;
        let between = Uniform::<usize>::try_from(2..n_max).unwrap();
        let mut rng = StdRng::seed_from_u64(789);
        let n = between.sample(&mut rng);
        let p1 = rand_perm(n, n * 2, &mut rng);
        let p2 = rand_perm(n, n * 2, &mut rng);
        let prod = p1.clone() * p2.clone();
        let domain_types = (0..n).map(|idx| idx + 100).collect::<Vec<usize>>();
        let mut types_at_this_stage = domain_types.clone();
        let cospan_p1 = Cospan::from_permutation_on_domain(p1.clone(), &domain_types).unwrap();
        in_place_permute(&mut types_at_this_stage, &p1.inv());
        let cospan_p2 =
            Cospan::from_permutation_on_domain(p2.clone(), &types_at_this_stage).unwrap();
        in_place_permute(&mut types_at_this_stage, &p2.inv());
        let cospan_prod = cospan_p1.compose(&cospan_p2);
        match cospan_prod {
            Ok(real_res) => {
                let expected_res = Cospan::from_permutation_on_domain(prod, &domain_types).unwrap();
                assert_eq!(real_res.left, expected_res.left);
                assert_eq!(real_res.right, expected_res.right);
                assert_eq!(real_res.middle, expected_res.middle);
                assert_eq!(real_res.domain(), domain_types);
                assert_eq!(real_res.codomain(), types_at_this_stage);
            }
            Err(e) => {
                panic!("Could not compose simple example\n{e:?}")
            }
        }
        let domain_types = (0..n).map(|idx| idx + 10).collect::<Vec<usize>>();
        let p1 = rand_perm(n, n * 2, &mut rng);
        let p2 = rand_perm(n, n * 2, &mut rng);
        let prod = p1.clone() * p2.clone();
        let mut types_at_this_stage = domain_types.clone();
        in_place_permute(&mut types_at_this_stage, &p1.inv());
        let cospan_p1 =
            Cospan::from_permutation_on_codomain(p1.clone(), &types_at_this_stage.clone()).unwrap();
        in_place_permute(&mut types_at_this_stage, &p2.inv());
        let cospan_p2 =
            Cospan::from_permutation_on_codomain(p2.clone(), &types_at_this_stage).unwrap();
        let cospan_prod = cospan_p1.compose(&cospan_p2);
        match cospan_prod {
            Ok(real_res) => {
                let expected_res =
                    Cospan::from_permutation_on_codomain(prod, &types_at_this_stage).unwrap();
                assert_eq!(real_res.left, expected_res.left);
                assert_eq!(real_res.right, expected_res.right);
                assert_eq!(real_res.middle, expected_res.middle);
                assert_eq!(real_res.domain(), domain_types);
                assert_eq!(real_res.codomain(), types_at_this_stage);
            }
            Err(e) => {
                panic!("Could not compose simple example\n{e:?}")
            }
        }
    }

    #[test]
    fn cospan_is_jointly_surjective() {
        use super::Cospan;
        // Surjective: every middle index appears in left or right leg
        let c1 = Cospan::new(vec![0], vec![1], vec!['a', 'b']).unwrap();
        assert!(c1.is_jointly_surjective());

        // Not surjective: middle index 2 appears in neither leg
        let c2 = Cospan::new(vec![0], vec![1], vec!['a', 'b', 'c']).unwrap();
        assert!(!c2.is_jointly_surjective());

        // Empty middle is vacuously surjective
        let c3 = Cospan::<char>::new(vec![], vec![], vec![]).unwrap();
        assert!(c3.is_jointly_surjective());

        // Middle index appears in both legs — still surjective
        let c4 = Cospan::new(vec![0], vec![0], vec!['a']).unwrap();
        assert!(c4.is_jointly_surjective());
    }

    // ---- #289: the mutators check in EVERY profile too ----

    /// `add_boundary_node` reports an out-of-bounds target with the same variant
    /// and the same four fields `new` uses, on both sides, and leaves the cospan
    /// untouched.
    ///
    /// # What this ranges over
    ///
    /// Two targets on each of the two legs: the first index past the apex, and
    /// one far past it. It does **not** range over apex sizes, `Lambda` types,
    /// or the `Right(label)` arm (which has no precondition); the claim is about
    /// this method's `Left(idx)` arms only, and the boundary value `tgt_idx ==
    /// leg.len()` gets its own test below.
    #[test]
    fn cospan_add_boundary_node_rejects_out_of_bounds_targets() {
        use super::Cospan;
        use crate::errors::{BoundaryLeg, CatgraphError};
        use either::Either::{Left, Right};

        let mut c = Cospan::<char>::new(vec![0], vec![0], vec!['a']).unwrap();

        // Domain: the new port would sit at position 1 (the leg holds one), and
        // targets apex index 1 of a 1-vertex apex.
        let err = c
            .add_boundary_node_known_target(Left(1))
            .expect_err("apex index 1 is out of bounds for a 1-vertex apex");
        assert_eq!(
            err,
            CatgraphError::ConstructionIndexOutOfBounds {
                leg: BoundaryLeg::Domain,
                position: 1,
                target: 1,
                target_len: 1,
            }
        );
        assert_eq!(
            err.to_string(),
            "construction error: domain leg entry 1 targets index 1, but the target set has 1 element(s)"
        );

        // Codomain, far past the end.
        let err = c
            .add_boundary_node_known_target(Right(7))
            .expect_err("apex index 7 is out of bounds for a 1-vertex apex");
        assert_eq!(
            err,
            CatgraphError::ConstructionIndexOutOfBounds {
                leg: BoundaryLeg::Codomain,
                position: 1,
                target: 7,
                target_len: 1,
            }
        );

        // Neither rejection mutated anything.
        assert_eq!(
            (c.left_to_middle(), c.right_to_middle(), c.middle().len()),
            (&[0][..], &[0][..], 1),
            "a rejected add_boundary_node must leave the cospan exactly as it was"
        );

        // The in-bounds call still works, through the same entry point.
        assert_eq!(c.add_boundary_node_known_target(Left(0)).unwrap(), Left(1));
        assert_eq!(c.left_to_middle(), &[0, 0]);
    }

    /// The identity flag no longer survives a boundary-node push that leaves the
    /// leg out of step with the apex — the #289 corruption, in both the shapes
    /// that reached it.
    ///
    /// # What this ranges over
    ///
    /// Two shapes on the **domain** leg only, both starting from a flag that is
    /// legitimately `true`:
    ///
    /// 1. `tgt_idx == leg.len()`, the one out-of-bounds target that satisfies
    ///    the old `leg.len() - 1 == tgt_idx` conjunct. This one is now refused
    ///    outright, so the flag cannot move.
    /// 2. The apex-growing `Right(label)` arm on the **other** leg, which is
    ///    what used to leave this flag a stale `true`: it grew the apex past
    ///    the domain leg while updating only the codomain flag. The flag must
    ///    clear, and must agree with a fresh construction from the same three
    ///    vectors.
    ///
    /// It covers the **domain** leg only. The `Left(idx)` arms themselves are
    /// now an unconditional clear (`is_left_id = false`): a `true` flag meant
    /// the leg already matched the apex, and that arm grows the leg without the
    /// apex, so no index keeps it an identity — there is no per-index conjunct
    /// left to pin (#289's review showed the first draft's
    /// `&= leg.len() - 1 == tgt_idx && leg.len() == middle.len()` was
    /// equivalent to the clear on every reachable state). Shape (1) is refused
    /// before the arm runs; the in-bounds add in (2) lands on a flag the apex
    /// growth already cleared and checks it stays cleared. The codomain mirror
    /// of shape (2) is pinned by `tests/checked_mutators.rs`
    /// (`cospan_unknown_target_add_clears_the_partner_legs_identity_flag`, which
    /// sweeps both arms) and the codomain refusal of shape (1) by
    /// `cospan_identity_flag_is_not_corrupted_at_the_boundary_index` there. The
    /// two arms are separate expressions — while #289 was being written the
    /// codomain one was in fact left behind while every domain-side assertion
    /// stayed green — so do not delete any of these believing another
    /// generalises.
    ///
    /// It does not range over `delete_boundary_node` (pinned separately below,
    /// on both legs) or over apex sizes beyond the two used here.
    #[test]
    fn cospan_identity_flag_survives_neither_shape_of_the_289_corruption() {
        use super::Cospan;
        use crate::errors::{BoundaryLeg, CatgraphError};
        use either::Either::{Left, Right};

        // (1) tgt_idx == leg.len(). Identity on ['a','b']: left == [0, 1], apex
        // has 2 vertices, so 2 is simultaneously `left.len()` and out of bounds.
        let mut id = Cospan::<char>::identity(&vec!['a', 'b']);
        assert!(
            id.is_left_identity(),
            "fixture must start as a left identity"
        );
        let err = id
            .add_boundary_node_known_target(Left(2))
            .expect_err("apex index 2 is out of bounds for a 2-vertex apex");
        assert_eq!(
            err,
            CatgraphError::ConstructionIndexOutOfBounds {
                leg: BoundaryLeg::Domain,
                position: 2,
                target: 2,
                target_len: 2,
            }
        );
        assert_eq!(
            (id.left_to_middle(), id.is_left_identity()),
            (&[0, 1][..], true),
            "the rejected push must not have appended 2 to the leg, and must not \
             have moved the flag; before #289 the leg became [0, 1, 2] with the \
             flag still true"
        );

        // (2) The apex grows on the *codomain* side. `is_left_id` is
        // legitimately true on entry and must not survive: the domain leg keeps
        // its single entry while the apex gains vertices.
        let mut c = Cospan::<char>::new(vec![0], vec![], vec!['a']).unwrap();
        assert!(
            c.is_left_identity(),
            "left == [0] on a 1-vertex apex is the identity"
        );
        c.add_boundary_node_unknown_target(Right('b'));
        assert_eq!(c.middle().len(), 2);
        assert!(
            !c.is_left_identity(),
            "left == [0] on a 2-vertex apex misses apex vertex 1. This read true \
             before the `Right(label)` arms were fixed to clear the partner \
             flag, and `perform_pushout` fast-paths on it"
        );

        c.add_boundary_node_unknown_target(Right('c'));
        // An in-bounds domain-side add on the leg that has fallen behind the
        // apex. The flag is already `false` from (2) — the pre-#289
        // `left.len() - 1 == tgt_idx` alone would have kept a `false` too — so
        // this checks only that the add keeps the flag false and in agreement
        // with a fresh construction.
        c.add_boundary_node_known_target(Left(1))
            .expect("apex index 1 is in bounds for a 3-vertex apex");
        assert_eq!(c.left_to_middle(), &[0, 1]);
        assert!(
            !c.is_left_identity(),
            "left == [0, 1] on a 3-vertex apex is not an identity: it misses apex \
             vertex 2"
        );
        // The reference: what `new_unchecked` would compute for the same data.
        assert!(
            !Cospan::new_unchecked(
                c.left_to_middle().to_vec(),
                c.right_to_middle().to_vec(),
                c.middle().to_vec(),
            )
            .is_left_identity(),
            "the mutator's flag must agree with a fresh construction from the \
             same three vectors"
        );
    }

    /// `delete_boundary_node` names the invariant it needs instead of
    /// underflowing, and clears an identity flag the removal invalidates.
    ///
    /// # What this ranges over
    ///
    /// The empty-leg case on the domain (the underflow), one non-empty
    /// out-of-range index on the codomain, and the flag update for a
    /// delete-the-last-port on **both** legs — the two flag arms are separate
    /// expressions in the source, so each is asserted on its own fixture. It
    /// does not range over deleting a non-last port, or over apex sizes beyond
    /// the one used here.
    #[test]
    fn cospan_delete_boundary_node_states_its_invariant_and_clears_the_flag() {
        use super::Cospan;
        use either::Either::{Left, Right};

        // Deleting the last port of an identity leaves left == [0] on a 2-vertex
        // apex, which is not an identity. Before #289 `is_left_id &= z ==
        // len - 1` held it true.
        let mut id = Cospan::<char>::identity(&vec!['a', 'b']);
        assert!(id.is_left_identity());
        id.delete_boundary_node(Left(1));
        assert_eq!(id.left_to_middle(), &[0]);
        assert!(
            !id.is_left_identity(),
            "left == [0] on a 2-vertex apex misses apex vertex 1"
        );

        // The codomain mirror, asserted separately because it is a separate
        // expression in the source.
        let mut id = Cospan::<char>::identity(&vec!['a', 'b']);
        assert!(id.is_right_identity());
        id.delete_boundary_node(Right(1));
        assert_eq!(id.right_to_middle(), &[0]);
        assert!(
            !id.is_right_identity(),
            "right == [0] on a 2-vertex apex misses apex vertex 1"
        );

        // The empty leg: the case that used to underflow `len() - 1`.
        let empty_domain = std::panic::catch_unwind(|| {
            let mut c = Cospan::<char>::new(vec![], vec![0], vec!['a']).unwrap();
            c.delete_boundary_node(Left(0));
        });
        let message = *empty_domain
            .expect_err("deleting from an empty domain must panic")
            .downcast::<String>()
            .expect("the panic payload is a formatted String");
        assert_eq!(
            message,
            "delete_boundary_node: domain index 0 is out of bounds; the domain has 0 port(s)",
            "the empty-leg case is the one that used to underflow `len() - 1`"
        );

        // Out of range on a non-empty leg.
        let out_of_range = std::panic::catch_unwind(|| {
            let mut c = Cospan::<char>::new(vec![], vec![0], vec!['a']).unwrap();
            c.delete_boundary_node(Right(3));
        });
        let message = *out_of_range
            .expect_err("codomain index 3 does not exist")
            .downcast::<String>()
            .expect("the panic payload is a formatted String");
        assert_eq!(
            message,
            "delete_boundary_node: codomain index 3 is out of bounds; the codomain has 1 port(s)"
        );
    }

    /// `map_to_same` and `connect_pair` name the offending port and boundary
    /// rather than panicking on a raw slice index.
    ///
    /// # What this ranges over
    ///
    /// One out-of-range port per method, on a different leg each, chosen so that
    /// the shared `boundary_target` helper is exercised through both callers and
    /// both of its arms. It does not range over the *second* argument position
    /// of either method, which reaches the same helper.
    #[test]
    fn cospan_port_indexing_mutators_name_their_invariant() {
        use super::Cospan;
        use either::Either::{Left, Right};

        let map_to_same = std::panic::catch_unwind(|| {
            let c = Cospan::<char>::new(vec![0], vec![0], vec!['a']).unwrap();
            let _ = c.map_to_same(Left(0), Left(4));
        });
        let message = *map_to_same
            .expect_err("domain port 4 does not exist")
            .downcast::<String>()
            .expect("the panic payload is a formatted String");
        assert_eq!(
            message,
            "map_to_same: domain index 4 is out of bounds; the domain has 1 port(s)"
        );

        let connect_pair = std::panic::catch_unwind(|| {
            let mut c = Cospan::<char>::new(vec![0], vec![0], vec!['a']).unwrap();
            c.connect_pair(Right(2), Left(0));
        });
        let message = *connect_pair
            .expect_err("codomain port 2 does not exist")
            .downcast::<String>()
            .expect("the panic payload is a formatted String");
        assert_eq!(
            message,
            "connect_pair: codomain index 2 is out of bounds; the codomain has 1 port(s)"
        );
    }
}
