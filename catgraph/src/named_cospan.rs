//! Cospan with named boundary nodes (ports).
//!
//! Wraps [`Cospan`] and attaches unique names to domain and codomain nodes,
//! enabling port-level mutation, lookup, and predicate-based search.

use crate::errors::{BoundaryLeg, CatgraphError};

use {
    crate::{
        category::{Composable, HasIdentity},
        cospan::Cospan,
        monoidal::SymmetricMonoidalMorphism,
        monoidal::{Monoidal, MonoidalMorphism},
        utils::in_place_permute,
    },
    either::Either::{self, Left, Right},
    log::warn,
    permutations::Permutation,
    std::fmt::Debug,
};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Threshold for parallelizing predicate filtering on named cospan boundaries
/// when the `parallel` feature is enabled. Predicate checks are cheap, so we
/// require larger collections before fanning work across workers.
#[cfg(feature = "parallel")]
const PARALLEL_PREDICATE_THRESHOLD: usize = 256;

type LeftIndex = usize;
type RightIndex = usize;
type MiddleIndex = usize;
type MiddleIndexOrLambda<Lambda> = Either<MiddleIndex, Lambda>;

/// A cospan with named boundary nodes (ports) for stable identity across reorderings.
#[derive(Clone)]
pub struct NamedCospan<Lambda: Sized + Eq + Copy + Debug, LeftPortName, RightPortName> {
    cospan: Cospan<Lambda>,
    left_names: Vec<LeftPortName>,
    right_names: Vec<RightPortName>,
}

impl<Lambda, LeftPortName, RightPortName> NamedCospan<Lambda, LeftPortName, RightPortName>
where
    Lambda: Sized + Eq + Copy + Debug,
    LeftPortName: Eq,
    RightPortName: Eq,
{
    /// Debug-asserts cospan validity and name-count consistency (does not check uniqueness).
    pub fn assert_valid_nohash(&self) {
        self.cospan.assert_valid();
        debug_assert_eq!(
            self.cospan.left_to_middle().len(),
            self.left_names.len(),
            "There was a mismatch between the domain size and the list of their names"
        );
        debug_assert_eq!(
            self.cospan.right_to_middle().len(),
            self.right_names.len(),
            "There was a mismatch between the codomain size and the list of their names"
        );
    }
}

impl<Lambda, LeftPortName, RightPortName> NamedCospan<Lambda, LeftPortName, RightPortName>
where
    Lambda: Sized + Eq + Copy + Debug,
    LeftPortName: Eq + Clone,
    RightPortName: Eq,
{
    /// Construct from explicit legs, middle set, and port names.
    ///
    /// Both structural invariants are checked **in every build profile**: one
    /// name per boundary port, and both leg maps landing inside the apex.
    /// Callers building from data that is correct by construction should use
    /// [`new_unchecked`](Self::new_unchecked).
    ///
    /// Name **uniqueness** is assumed, not enforced here: a duplicate admitted
    /// by this constructor is first reported when a later
    /// [`add_boundary_node`](Self::add_boundary_node) returns
    /// [`ConstructionDuplicatePortName`](crate::errors::CatgraphError::ConstructionDuplicatePortName)
    /// with the `existing_position` of the first copy.
    /// [`assert_valid`](Self::assert_valid) checks uniqueness under `Hash` in
    /// debug builds.
    ///
    /// # Errors
    ///
    /// - [`CatgraphError::ConstructionNameCountMismatch`] if
    ///   `left_names.len() != left.len()` or `right_names.len() != right.len()`.
    /// - [`CatgraphError::ConstructionIndexOutOfBounds`] if any `left` or `right`
    ///   entry targets an index at or beyond `middle.len()`; this check is
    ///   [`Cospan::new`](Cospan::new)'s.
    ///
    /// The name counts are checked before the leg bounds, and the domain side
    /// before the codomain side, so the reported failure is the first one in
    /// that order.
    pub fn new(
        left: Vec<MiddleIndex>,
        right: Vec<MiddleIndex>,
        middle: Vec<Lambda>,
        left_names: Vec<LeftPortName>,
        right_names: Vec<RightPortName>,
    ) -> Result<Self, CatgraphError> {
        for (leg, boundary_len, name_count) in [
            (BoundaryLeg::Domain, left.len(), left_names.len()),
            (BoundaryLeg::Codomain, right.len(), right_names.len()),
        ] {
            if name_count != boundary_len {
                return Err(CatgraphError::ConstructionNameCountMismatch {
                    leg,
                    boundary_len,
                    name_count,
                });
            }
        }
        Ok(Self {
            cospan: Cospan::new(left, right, middle)?,
            left_names,
            right_names,
        })
    }

    /// Construct without checking the name counts or the leg bounds.
    ///
    /// Both invariants are the caller's responsibility; both are re-checked by
    /// `debug_assert!` only (via [`assert_valid_nohash`](Self::assert_valid_nohash)),
    /// so a release build accepts a mismatched name list or an out-of-bounds leg
    /// and defers the failure to whatever indexes it later. Use this where the
    /// data is correct **by construction** and [`new`](Self::new) everywhere it
    /// crosses a trust boundary.
    ///
    /// Mirrors [`Cospan::new_unchecked`](Cospan::new_unchecked): the whole check
    /// set compiles away in release, so the constructor costs nothing there.
    #[must_use]
    pub fn new_unchecked(
        left: Vec<MiddleIndex>,
        right: Vec<MiddleIndex>,
        middle: Vec<Lambda>,
        left_names: Vec<LeftPortName>,
        right_names: Vec<RightPortName>,
    ) -> Self {
        let answer = Self {
            cospan: Cospan::new_unchecked(left, right, middle),
            left_names,
            right_names,
        };
        answer.assert_valid_nohash();
        answer
    }

    /// The named cospan with empty domain, codomain, and middle set.
    #[must_use]
    pub fn empty() -> Self {
        Self::new_unchecked(vec![], vec![], vec![], vec![], vec![])
    }

    #[must_use]
    pub const fn cospan(&self) -> &Cospan<Lambda> {
        &self.cospan
    }

    #[must_use]
    pub const fn left_names(&self) -> &Vec<LeftPortName> {
        &self.left_names
    }

    #[must_use]
    pub const fn right_names(&self) -> &Vec<RightPortName> {
        &self.right_names
    }

    /// Identity cospan with port names derived from `prenames` via `prename_to_name`.
    ///
    /// # Panics
    ///
    /// Panics if `types.len() != prenames.len()`.
    pub fn identity<T, F>(types: &[Lambda], prenames: &[T], prename_to_name: F) -> Self
    where
        F: Fn(T) -> (LeftPortName, RightPortName),
        T: Clone,
    {
        assert_eq!(types.len(), prenames.len());
        let (left_names, right_names) = prenames.iter().map(|x| prename_to_name(x.clone())).unzip();

        Self {
            cospan: Cospan::identity(&types.to_vec()),
            left_names,
            right_names,
        }
    }

    /// Build a named cospan from a permutation, with `types` and `prenames`
    /// labelling the **domain**.
    ///
    /// The named counterpart of
    /// [`Cospan::from_permutation_on_domain`](crate::cospan::Cospan::from_permutation_on_domain):
    /// the left port names follow `prenames` as given, and the right port names
    /// are reordered by `p.inv()`, matching how the codomain *labels* are
    /// reordered.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::CompositionSizeMismatch`] if `p`, `types` and
    /// `prenames` do not all have the same length. `prenames` is checked first,
    /// then `p` against `types` by the cospan builder.
    #[allow(clippy::needless_pass_by_value)]
    pub fn from_permutation_extra_data_on_domain<T, F>(
        p: Permutation,
        types: &[Lambda],
        prenames: &[T],
        prename_to_name: F,
    ) -> Result<Self, CatgraphError>
    where
        T: Clone,
        F: Fn(T) -> (LeftPortName, RightPortName),
    {
        if types.len() != prenames.len() {
            return Err(CatgraphError::CompositionSizeMismatch {
                expected: types.len(),
                actual: prenames.len(),
            });
        }
        let cospan = Cospan::from_permutation_on_domain(p.clone(), types)?;
        Ok(Self {
            cospan,
            left_names: prenames
                .iter()
                .map(|pre| prename_to_name(pre.clone()).0)
                .collect(),
            right_names: p
                .inv()
                .permute(prenames)
                .iter()
                .map(|pre| prename_to_name(pre.clone()).1)
                .collect(),
        })
    }

    /// Build a named cospan from a permutation, with `types` and `prenames`
    /// labelling the **codomain**.
    ///
    /// The named counterpart of
    /// [`Cospan::from_permutation_on_codomain`](crate::cospan::Cospan::from_permutation_on_codomain):
    /// the right port names follow `prenames` as given, and the left port names
    /// are reordered by `p`.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::CompositionSizeMismatch`] if `p`, `types` and
    /// `prenames` do not all have the same length.
    #[allow(clippy::needless_pass_by_value)]
    pub fn from_permutation_extra_data_on_codomain<T, F>(
        p: Permutation,
        types: &[Lambda],
        prenames: &[T],
        prename_to_name: F,
    ) -> Result<Self, CatgraphError>
    where
        T: Clone,
        F: Fn(T) -> (LeftPortName, RightPortName),
    {
        if types.len() != prenames.len() {
            return Err(CatgraphError::CompositionSizeMismatch {
                expected: types.len(),
                actual: prenames.len(),
            });
        }
        let cospan = Cospan::from_permutation_on_codomain(p.clone(), types)?;
        Ok(Self {
            cospan,
            left_names: p
                .permute(prenames)
                .iter()
                .map(|pre| prename_to_name(pre.clone()).0)
                .collect(),
            right_names: prenames
                .iter()
                .map(|pre| prename_to_name(pre.clone()).1)
                .collect(),
        })
    }

    /// Add a named boundary node targeting an existing middle index. Side determined by `new_name`.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::ConstructionDuplicatePortName`] if `new_name` is
    /// already taken on that boundary, or
    /// [`CatgraphError::ConstructionIndexOutOfBounds`] if `new_arrow` is at or
    /// beyond the apex size; see
    /// [`add_boundary_node`](Self::add_boundary_node) for the order and for the
    /// no-change-on-error guarantee.
    pub fn add_boundary_node_known_target(
        &mut self,
        new_arrow: MiddleIndex,
        new_name: Either<LeftPortName, RightPortName>,
    ) -> Result<Either<LeftIndex, RightIndex>, CatgraphError> {
        self.add_boundary_node(Left(new_arrow), new_name)
    }

    /// Add a named boundary node that creates a new middle vertex with the given label.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::ConstructionDuplicatePortName`] if `new_name` is
    /// already taken on that boundary. The apex index cannot be out of bounds
    /// here — the vertex is minted by this call — so that is the only failure.
    pub fn add_boundary_node_unknown_target(
        &mut self,
        new_arrow: Lambda,
        new_name: Either<LeftPortName, RightPortName>,
    ) -> Result<Either<LeftIndex, RightIndex>, CatgraphError> {
        self.add_boundary_node(Right(new_arrow), new_name)
    }

    /// Add a named boundary node to new or existing middle vertex.
    ///
    /// # Errors
    ///
    /// - [`CatgraphError::ConstructionDuplicatePortName`] if `new_name` already
    ///   names a port on the boundary it selects, giving that port's position.
    /// - [`CatgraphError::ConstructionIndexOutOfBounds`] if `new_arrow` is
    ///   `Left(tgt_idx)` with `tgt_idx` at or beyond the apex size; this check is
    ///   [`Cospan::add_boundary_node`](Cospan::add_boundary_node)'s.
    ///
    /// The name is checked before the index, so a call that violates both is
    /// reported as the duplicate name. On `Err` the named cospan is left exactly
    /// as it was: neither the name list nor the leg is pushed to.
    pub fn add_boundary_node(
        &mut self,
        new_arrow: MiddleIndexOrLambda<Lambda>,
        new_name: Either<LeftPortName, RightPortName>,
    ) -> Result<Either<LeftIndex, RightIndex>, CatgraphError> {
        // Resolve the collision before touching either list, so an `Err` leaves
        // the value untouched rather than half-mutated.
        let (leg, collision) = match &new_name {
            Left(name) => (
                BoundaryLeg::Domain,
                self.left_names.iter().position(|r| r == name),
            ),
            Right(name) => (
                BoundaryLeg::Codomain,
                self.right_names.iter().position(|r| r == name),
            ),
        };
        if let Some(existing_position) = collision {
            return Err(CatgraphError::ConstructionDuplicatePortName {
                leg,
                existing_position,
            });
        }
        let arrow = match new_name {
            Left(new_name_real) => {
                // Bounds first: the cospan must not be mutated if the name list
                // is not, and vice versa.
                let index = self.cospan.add_boundary_node(Left(new_arrow))?;
                self.left_names.push(new_name_real);
                index
            }
            Right(new_name_real) => {
                let index = self.cospan.add_boundary_node(Right(new_arrow))?;
                self.right_names.push(new_name_real);
                index
            }
        };
        Ok(arrow)
    }

    /// Add a named boundary node without checking the port name or the apex index.
    ///
    /// Both invariants are the caller's responsibility; both are re-checked by
    /// `debug_assert!` only (name uniqueness here, apex bounds in
    /// [`Cospan::add_boundary_node_unchecked`](Cospan::add_boundary_node_unchecked)),
    /// so a release build accepts a duplicate name or an out-of-bounds entry and
    /// defers the failure to whatever reads it later. Use this where the data is
    /// correct **by construction** and
    /// [`add_boundary_node`](Self::add_boundary_node) everywhere it crosses a
    /// trust boundary.
    ///
    /// Mirrors [`new_unchecked`](Self::new_unchecked): the whole check set
    /// compiles away in release, so the mutator costs nothing there.
    ///
    /// # Panics
    ///
    /// In debug builds only, panics if `new_name` is already taken on that
    /// boundary or if `new_arrow` is an out-of-bounds apex index.
    pub fn add_boundary_node_unchecked(
        &mut self,
        new_arrow: MiddleIndexOrLambda<Lambda>,
        new_name: Either<LeftPortName, RightPortName>,
    ) -> Either<LeftIndex, RightIndex> {
        self.cospan.add_boundary_node_unchecked(match new_name {
            Left(new_name_real) => {
                debug_assert!(
                    !self.left_names.contains(&new_name_real),
                    "There was already a node on the left with the specified new name"
                );
                self.left_names.push(new_name_real);
                Left(new_arrow)
            }
            Right(new_name_real) => {
                debug_assert!(
                    !self.right_names.contains(&new_name_real),
                    "There was already a node on the right with the specified new name"
                );
                self.right_names.push(new_name_real);
                Right(new_arrow)
            }
        })
    }

    /// Remove a boundary node by index, keeping names in sync (uses `swap_remove` internally).
    ///
    /// # Panics
    ///
    /// Panics — in **every** build profile — if `which_node` is at or beyond the
    /// length of the boundary it names, the empty boundary included. The name
    /// list is `swap_remove`d first, so the check has to happen here as well as
    /// in [`Cospan::delete_boundary_node`](Cospan::delete_boundary_node): the
    /// underlying cospan's message would otherwise never be reached, and the
    /// name list would be left one shorter than the leg.
    pub fn delete_boundary_node(&mut self, which_node: Either<LeftIndex, RightIndex>) {
        /*
        CAUTION : relies on knowing that cospan uses swap_remove when deleting a node
            the implementation of delete_boundary_node on Cospan<Lambda>
        */
        match which_node {
            Left(z) => {
                assert!(
                    z < self.left_names.len(),
                    "delete_boundary_node: domain index {z} is out of bounds; the domain has {} port(s)",
                    self.left_names.len()
                );
                self.left_names.swap_remove(z);
            }
            Right(z) => {
                assert!(
                    z < self.right_names.len(),
                    "delete_boundary_node: codomain index {z} is out of bounds; the codomain has {} port(s)",
                    self.right_names.len()
                );
                self.right_names.swap_remove(z);
            }
        }
        self.cospan.delete_boundary_node(which_node);
    }

    /// Check if two named ports map to the same middle vertex. Returns false if either name is missing.
    pub fn map_to_same(
        &mut self,
        node_1_name: Either<LeftPortName, RightPortName>,
        node_2_name: Either<LeftPortName, RightPortName>,
    ) -> bool {
        let node_1_loc = self.find_node_by_name(node_1_name);
        let node_2_loc = self.find_node_by_name(node_2_name);
        if let Some((node_1_loc_real, node_2_loc_real)) = node_1_loc.zip(node_2_loc) {
            self.cospan.map_to_same(node_1_loc_real, node_2_loc_real)
        } else {
            false
        }
    }

    /// Merge the middle vertices behind two named ports. No-op if names not found or labels differ.
    pub fn connect_pair(
        &mut self,
        node_1_name: Either<LeftPortName, RightPortName>,
        node_2_name: Either<LeftPortName, RightPortName>,
    ) {
        let node_1_loc = self.find_node_by_name(node_1_name);
        let node_2_loc = self.find_node_by_name(node_2_name);
        if let Some((node_1_loc_real, node_2_loc_real)) = node_1_loc.zip(node_2_loc) {
            self.cospan.connect_pair(node_1_loc_real, node_2_loc_real);
        }
    }

    fn find_node_by_name(
        &self,
        desired_name: Either<LeftPortName, RightPortName>,
    ) -> Option<Either<LeftIndex, RightIndex>> {
        match desired_name {
            Left(desired_name_left) => {
                let index_in_left: Option<LeftIndex> =
                    self.left_names.iter().position(|r| *r == desired_name_left);
                index_in_left.map(Left)
            }
            Right(desired_name_right) => {
                let index_in_right: Option<RightIndex> = self
                    .right_names
                    .iter()
                    .position(|r| *r == desired_name_right);
                index_in_right.map(Right)
            }
        }
    }

    /// Find boundary nodes whose names satisfy the given predicates.
    ///
    /// # Output order
    ///
    /// When `at_most_one` is false, the result lists all left matches in
    /// ascending index order, followed by all right matches in ascending index
    /// order. When `at_most_one` is true, it short-circuits and returns the
    /// first (lowest-index) left match if any exists, otherwise the first
    /// (lowest-index) right match, otherwise the empty vector.
    ///
    /// Parallelized with rayon when total boundary size >= 256; the ordering
    /// above holds identically on the parallel and sequential arms.
    pub fn find_nodes_by_name_predicate<F, G>(
        &self,
        left_pred: F,
        right_pred: G,
        at_most_one: bool,
    ) -> Vec<Either<LeftIndex, RightIndex>>
    where
        F: Fn(LeftPortName) -> bool + Sync,
        G: Fn(RightPortName) -> bool + Sync,
        LeftPortName: Clone + Send + Sync,
        RightPortName: Clone + Send + Sync,
    {
        if at_most_one {
            let index_in_left: Option<LeftIndex> =
                self.left_names.iter().position(|r| left_pred(r.clone()));
            match index_in_left {
                None => {
                    let index_in_right: Option<RightIndex> =
                        self.right_names.iter().position(|r| right_pred(r.clone()));

                    index_in_right.map(Right).into_iter().collect()
                }
                Some(z) => {
                    vec![Left(z)]
                }
            }
        } else {
            // With the `parallel` feature on, `with_min_len` tells rayon's
            // LengthSplitter not to subdivide below the threshold, so small
            // inputs run as a single sequential task and large inputs fan out
            // across workers. Without the feature (e.g. `wasm32-wasip1`
            // single-threaded), fall back to a plain sequential iterator.
            #[cfg(feature = "parallel")]
            let mut matched_indices: Vec<Either<LeftIndex, RightIndex>> = self
                .left_names
                .par_iter()
                .with_min_len(PARALLEL_PREDICATE_THRESHOLD)
                .enumerate()
                .filter_map(|(index, r)| left_pred(r.clone()).then_some(Left(index)))
                .collect();
            #[cfg(not(feature = "parallel"))]
            let mut matched_indices: Vec<Either<LeftIndex, RightIndex>> = self
                .left_names
                .iter()
                .enumerate()
                .filter_map(|(index, r)| left_pred(r.clone()).then_some(Left(index)))
                .collect();

            #[cfg(feature = "parallel")]
            let right_indices: Vec<_> = self
                .right_names
                .par_iter()
                .with_min_len(PARALLEL_PREDICATE_THRESHOLD)
                .enumerate()
                .filter_map(|(index, r)| right_pred(r.clone()).then_some(Right(index)))
                .collect();
            #[cfg(not(feature = "parallel"))]
            let right_indices: Vec<_> = self
                .right_names
                .iter()
                .enumerate()
                .filter_map(|(index, r)| right_pred(r.clone()).then_some(Right(index)))
                .collect();

            matched_indices.extend(right_indices);
            matched_indices
        }
    }

    /// Delete a boundary node by name. Warns and makes no change if the name is not found.
    ///
    /// Cannot panic, unlike the index-taking
    /// [`delete_boundary_node`](Self::delete_boundary_node) it forwards to: the
    /// index comes from a `position` lookup on the very list being shortened, so
    /// it is in bounds by construction, and a missing name returns early.
    pub fn delete_boundary_node_by_name(
        &mut self,
        which_node: Either<LeftPortName, RightPortName>,
    ) {
        let which_node_idx = match which_node {
            Left(z) => {
                let index = self.left_names.iter().position(|r| *r == z);
                let Some(idx_left) = index else {
                    warn!("Node to be deleted does not exist. No change made.");
                    return;
                };
                Left(idx_left)
            }
            Right(z) => {
                let index = self.right_names.iter().position(|r| *r == z);
                let Some(idx_right) = index else {
                    warn!("Node to be deleted does not exist. No change made.");
                    return;
                };
                Right(idx_right)
            }
        };
        self.delete_boundary_node(which_node_idx);
    }

    /// Rename all ports on one side by applying a function. `Left(f)` renames domain, `Right(f)` codomain.
    pub fn change_boundary_node_names<FL, FR>(&mut self, f: Either<FL, FR>)
    where
        FL: Fn(&mut LeftPortName),
        FR: Fn(&mut RightPortName),
    {
        match f {
            Left(left_fun) => {
                for cur_left_name in &mut self.left_names {
                    left_fun(cur_left_name);
                }
            }
            Right(right_fun) => {
                for cur_right_name in &mut self.right_names {
                    right_fun(cur_right_name);
                }
            }
        }
    }

    /// Rename a single port from `old_name` to `new_name`. Warns if old name not found.
    ///
    /// # Panics
    ///
    /// Panics if `new_name` already exists on the boundary.
    pub fn change_boundary_node_name(
        &mut self,
        name_pair: Either<(LeftPortName, LeftPortName), (RightPortName, RightPortName)>,
    ) {
        match name_pair {
            Left((z1, z2)) => {
                let Some(idx_left) = self.left_names.iter().position(|r| *r == z1) else {
                    warn!("Node to be changed does not exist. No change made.");
                    return;
                };
                assert!(
                    !self.left_names.contains(&z2),
                    "There was already a node on the left with the specified new name"
                );
                self.left_names[idx_left] = z2;
            }
            Right((z1, z2)) => {
                let Some(idx_right) = self.right_names.iter().position(|r| *r == z1) else {
                    warn!("Node to be changed does not exist. No change made.");
                    return;
                };
                assert!(
                    !self.right_names.contains(&z2),
                    "There was already a node on the right with the specified new name"
                );
                self.right_names[idx_right] = z2;
            }
        }
    }

    /// Append a new vertex to the middle set with the given label. Returns its index.
    ///
    /// The index is what
    /// [`add_boundary_node_known_target`](Self::add_boundary_node_known_target)
    /// takes, so returning it — as
    /// [`Cospan::add_middle`](Cospan::add_middle) always has — is what lets the
    /// two be chained without counting vertices by hand.
    pub fn add_middle(&mut self, new_middle: Lambda) -> MiddleIndex {
        self.cospan.add_middle(new_middle)
    }

    /// Apply a function to all middle vertex labels, preserving port names.
    pub fn map<F, Mu>(&self, f: F) -> NamedCospan<Mu, LeftPortName, RightPortName>
    where
        F: Fn(Lambda) -> Mu,
        Mu: Sized + Eq + Copy + Debug,
        RightPortName: Clone,
    {
        NamedCospan {
            cospan: self.cospan.map(f),
            left_names: self.left_names.clone(),
            right_names: self.right_names.clone(),
        }
    }
}

impl<Lambda, LeftPortName, RightPortName> NamedCospan<Lambda, LeftPortName, RightPortName>
where
    Lambda: Sized + Eq + Copy + Debug,
    LeftPortName: Eq + std::hash::Hash,
    RightPortName: Eq + std::hash::Hash,
{
    /// Full validity check including name uniqueness (requires `Hash`).
    ///
    /// Lost its `check_id: bool` with
    /// [`assert_valid_nohash`](Self::assert_valid_nohash)'s; see there.
    pub fn assert_valid(&self) {
        self.assert_valid_nohash();
        debug_assert!(
            crate::utils::is_unique(&self.left_names),
            "There was a duplicate name on the domain"
        );
        debug_assert!(
            crate::utils::is_unique(&self.right_names),
            "There was a duplicate name on the codomain"
        );
    }
}

impl<Lambda, LeftPortName, RightPortName> Monoidal
    for NamedCospan<Lambda, LeftPortName, RightPortName>
where
    Lambda: Sized + Eq + Copy + Debug,
    LeftPortName: Eq + Clone,
    RightPortName: Eq,
{
    fn monoidal(&mut self, other: Self) {
        self.cospan.monoidal(other.cospan);
        // Name uniqueness across self and other is not checked here.
        self.left_names.extend(other.left_names);
        self.right_names.extend(other.right_names);
    }
}

impl<Lambda, LeftPortName, RightPortName> Composable<Vec<Lambda>>
    for NamedCospan<Lambda, LeftPortName, RightPortName>
where
    Lambda: Sized + Eq + Copy + Debug,
    LeftPortName: Eq + Clone,
    RightPortName: Eq + Clone,
{
    fn composable(&self, other: &Self) -> Result<(), CatgraphError> {
        self.cospan.composable(&other.cospan)
    }

    fn compose(&self, other: &Self) -> Result<Self, CatgraphError> {
        Ok(Self {
            cospan: self.cospan.compose(&other.cospan)?,
            left_names: self.left_names.clone(),
            right_names: other.right_names.clone(),
        })
    }

    fn domain(&self) -> Vec<Lambda> {
        self.cospan.domain()
    }

    fn codomain(&self) -> Vec<Lambda> {
        self.cospan.codomain()
    }
}

impl<Lambda, LeftPortName, RightPortName> MonoidalMorphism<Vec<Lambda>>
    for NamedCospan<Lambda, LeftPortName, RightPortName>
where
    Lambda: Sized + Eq + Copy + Debug,
    LeftPortName: Eq + Clone,
    RightPortName: Eq + Clone,
{
}

impl<Lambda, LeftPortName, RightPortName> SymmetricMonoidalMorphism<Lambda>
    for NamedCospan<Lambda, LeftPortName, RightPortName>
where
    Lambda: Sized + Eq + Copy + Debug,
    LeftPortName: Eq + Clone,
    RightPortName: Eq + Clone,
{
    /// Delegates to [`Cospan::permute_side`] and carries the port names with
    /// their wires.
    ///
    /// A name word travels exactly as a label word does, so it takes the same
    /// `p.inv()` [`Cospan`] applies to the leg — see the trait's contract.
    fn permute_side(&mut self, p: &Permutation, of_right_leg: bool) {
        let p_inv = p.inv();
        if of_right_leg {
            in_place_permute(&mut self.right_names, &p_inv);
        } else {
            in_place_permute(&mut self.left_names, &p_inv);
        }
        self.cospan.permute_side(p, of_right_leg);
    }

    /// Always fails: a named cospan cannot be built from a permutation alone.
    ///
    /// Port names are not derivable from `types`, so there is no value to
    /// return.
    ///
    /// # Errors
    ///
    /// Always returns [`CatgraphError::Composition`], directing the caller to
    /// [`NamedCospan::from_permutation_extra_data_on_domain`].
    fn from_permutation_on_domain(
        _p: Permutation,
        _types: &[Lambda],
    ) -> Result<Self, CatgraphError> {
        Err(CatgraphError::Composition {
            message: "NamedCospan::from_permutation_on_domain requires port name data; use from_permutation_extra_data_on_domain instead".to_string(),
        })
    }

    /// Always fails, for the same reason as
    /// [`from_permutation_on_domain`](Self::from_permutation_on_domain).
    ///
    /// # Errors
    ///
    /// Always returns [`CatgraphError::Composition`], directing the caller to
    /// [`NamedCospan::from_permutation_extra_data_on_codomain`].
    fn from_permutation_on_codomain(
        _p: Permutation,
        _types: &[Lambda],
    ) -> Result<Self, CatgraphError> {
        Err(CatgraphError::Composition {
            message: "NamedCospan::from_permutation_on_codomain requires port name data; use from_permutation_extra_data_on_codomain instead".to_string(),
        })
    }
}

#[cfg(test)]
mod test {
    #[allow(unused_imports)]
    use super::*;
    use crate::{category::Composable, monoidal::Monoidal, monoidal::SymmetricMonoidalMorphism};
    use either::Either::{Left, Right};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    // ---- `new` validates in EVERY profile, `new_unchecked` does not ----

    /// Fixture: a domain-leg and a codomain-leg entry overshooting the apex,
    /// with correct name counts, through `NamedCospan::new`.
    ///
    /// Expected: `ConstructionIndexOutOfBounds` with `Cospan::new`'s payload
    /// and message on each side. The check is unconditional, not a
    /// `debug_assert!`, so this states the release-build behaviour too.
    #[test]
    fn named_cospan_new_rejects_out_of_bounds_leg_entries() {
        use crate::errors::{BoundaryLeg, CatgraphError};

        // Domain leg: entry 1 targets apex index 2, apex has 2 vertices.
        // Both name counts are correct, so nothing but the leg check can fire.
        // `NamedCospan` has no `Debug` impl, so `expect_err` is unavailable;
        // `let`-`else` pins the same thing without widening the public API.
        let Err(err) = NamedCospan::<char, &str, &str>::new(
            vec![0, 2],
            vec![1],
            vec!['a', 'b'],
            vec!["x", "y"],
            vec!["z"],
        ) else {
            panic!("left leg index 2 is out of bounds for a 2-vertex apex");
        };
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

        // The repaired input — same shape, leg entry pulled back in bounds —
        // constructs, so the rejection above cannot be blamed on anything else.
        let repaired = NamedCospan::<char, &str, &str>::new(
            vec![0, 1],
            vec![1],
            vec!['a', 'b'],
            vec!["x", "y"],
            vec!["z"],
        )
        .expect("the repaired domain leg is in bounds");
        assert_eq!(repaired.cospan().left_to_middle(), &[0, 1]);

        // Codomain leg: entry 1 targets apex index 7, apex has 2 vertices.
        let Err(err) = NamedCospan::<char, &str, &str>::new(
            vec![0],
            vec![1, 7],
            vec!['a', 'b'],
            vec!["x"],
            vec!["y", "z"],
        ) else {
            panic!("right leg index 7 is out of bounds for a 2-vertex apex");
        };
        assert_eq!(
            err,
            CatgraphError::ConstructionIndexOutOfBounds {
                leg: BoundaryLeg::Codomain,
                position: 1,
                target: 7,
                target_len: 2,
            }
        );

        let repaired = NamedCospan::<char, &str, &str>::new(
            vec![0],
            vec![1, 0],
            vec!['a', 'b'],
            vec!["x"],
            vec!["y", "z"],
        )
        .expect("the repaired codomain leg is in bounds");
        assert_eq!(repaired.cospan().right_to_middle(), &[1, 0]);
    }

    /// Fixture: a port-name list one short of its boundary, on the domain side
    /// and then on the codomain side, with in-bounds legs.
    ///
    /// Expected: `ConstructionNameCountMismatch` naming the leg, the boundary
    /// length and the name count — an error, not a panic.
    #[test]
    fn named_cospan_new_rejects_name_count_mismatch() {
        use crate::errors::{BoundaryLeg, CatgraphError};

        // Domain: 2 ports, 1 name. Legs are in bounds, so only this can fire.
        let Err(err) = NamedCospan::<char, &str, &str>::new(
            vec![0, 1],
            vec![0],
            vec!['a', 'b'],
            vec!["x"],
            vec!["z"],
        ) else {
            panic!("2 domain ports were given 1 name");
        };
        assert_eq!(
            err,
            CatgraphError::ConstructionNameCountMismatch {
                leg: BoundaryLeg::Domain,
                boundary_len: 2,
                name_count: 1,
            }
        );
        assert_eq!(
            err.to_string(),
            "construction error: the domain has 2 port(s) but 1 port name(s) were supplied; \
             there must be exactly one name per port"
        );

        // Repaired: the missing name supplied, everything else identical.
        let repaired = NamedCospan::<char, &str, &str>::new(
            vec![0, 1],
            vec![0],
            vec!['a', 'b'],
            vec!["x", "y"],
            vec!["z"],
        )
        .expect("2 domain ports with 2 names construct");
        assert_eq!(repaired.left_names(), &vec!["x", "y"]);

        // Codomain: 1 port, 2 names.
        let Err(err) = NamedCospan::<char, &str, &str>::new(
            vec![0, 1],
            vec![0],
            vec!['a', 'b'],
            vec!["x", "y"],
            vec!["z", "w"],
        ) else {
            panic!("1 codomain port was given 2 names");
        };
        assert_eq!(
            err,
            CatgraphError::ConstructionNameCountMismatch {
                leg: BoundaryLeg::Codomain,
                boundary_len: 1,
                name_count: 2,
            }
        );

        let repaired = NamedCospan::<char, &str, &str>::new(
            vec![0, 1],
            vec![0],
            vec!['a', 'b'],
            vec!["x", "y"],
            vec!["z"],
        )
        .expect("1 codomain port with 1 name constructs");
        assert_eq!(repaired.right_names(), &vec!["z"]);

        // Both sides mismatched: the domain is scanned first, so it is reported.
        let Err(err) = NamedCospan::<char, &str, &str>::new(
            vec![0, 1],
            vec![0],
            vec!['a', 'b'],
            vec![],
            vec!["z", "w"],
        ) else {
            panic!("both name lists are the wrong length");
        };
        assert_eq!(
            err,
            CatgraphError::ConstructionNameCountMismatch {
                leg: BoundaryLeg::Domain,
                boundary_len: 2,
                name_count: 0,
            },
            "the domain side must win the race, otherwise the reported leg is not deterministic"
        );

        // Name counts are checked BEFORE the leg bounds: with both broken, the
        // name-count failure is the one reported. This pins the documented
        // order, which is what makes the reported error independent of whether
        // the apex happens to be malformed too.
        let Err(err) = NamedCospan::<char, &str, &str>::new(
            vec![0, 9],
            vec![0],
            vec!['a', 'b'],
            vec!["x"],
            vec!["z"],
        ) else {
            panic!("the domain name count is wrong AND its leg overshoots the apex");
        };
        assert_eq!(
            err,
            CatgraphError::ConstructionNameCountMismatch {
                leg: BoundaryLeg::Domain,
                boundary_len: 2,
                name_count: 1,
            },
            "name counts are documented to be checked before the leg bounds"
        );
    }

    /// `new_unchecked` takes both invariants on trust: a release build accepts a
    /// leg that overshoots the apex and a name list of the wrong length, exactly
    /// as `Cospan::new_unchecked` accepts the former.
    ///
    /// Release-only, because in a debug build the `debug_assert!`s in
    /// `assert_valid_nohash` fire — which IS the contract. Same shape as
    /// `cospan::test::cospan_new_unchecked_accepts_what_new_refuses`.
    #[cfg(not(debug_assertions))]
    #[test]
    fn named_cospan_new_unchecked_accepts_what_new_refuses() {
        // Out-of-bounds domain leg.
        let bad = NamedCospan::<char, &str, &str>::new_unchecked(
            vec![0, 2],
            vec![1],
            vec!['a', 'b'],
            vec!["x", "y"],
            vec!["z"],
        );
        assert_eq!(bad.cospan().left_to_middle(), &[0, 2]);
        assert_eq!(bad.cospan().middle().len(), 2);
        assert!(
            NamedCospan::<char, &str, &str>::new(
                vec![0, 2],
                vec![1],
                vec!['a', 'b'],
                vec!["x", "y"],
                vec!["z"],
            )
            .is_err(),
            "the same input must be refused by the checked constructor"
        );

        // Name list shorter than its boundary.
        let bad = NamedCospan::<char, &str, &str>::new_unchecked(
            vec![0, 1],
            vec![0],
            vec!['a', 'b'],
            vec!["x"],
            vec!["z"],
        );
        assert_eq!(bad.cospan().left_to_middle().len(), 2);
        assert_eq!(
            bad.left_names().len(),
            1,
            "new_unchecked keeps the caller's name list verbatim, mismatch and all"
        );
        assert!(
            NamedCospan::<char, &str, &str>::new(
                vec![0, 1],
                vec![0],
                vec!['a', 'b'],
                vec!["x"],
                vec!["z"],
            )
            .is_err(),
            "the same input must be refused by the checked constructor"
        );
    }

    #[test]
    fn named_cospan_new() {
        let cospan: NamedCospan<char, &str, &str> = NamedCospan::new(
            vec![0, 1],
            vec![0],
            vec!['a', 'b'],
            vec!["x", "y"],
            vec!["z"],
        )
        .unwrap();
        assert_eq!(cospan.left_names().len(), 2);
        assert_eq!(cospan.right_names().len(), 1);
    }

    #[test]
    fn named_cospan_empty() {
        let cospan: NamedCospan<char, &str, &str> = NamedCospan::empty();
        assert!(cospan.left_names().is_empty());
        assert!(cospan.right_names().is_empty());
    }

    #[test]
    fn named_cospan_identity() {
        let types = vec!['a', 'b', 'c'];
        let prenames = vec![1, 2, 3];
        let cospan: NamedCospan<char, i32, i32> =
            NamedCospan::identity(&types, &prenames, |n| (n, n * 10));
        assert_eq!(cospan.left_names(), &vec![1, 2, 3]);
        assert_eq!(cospan.right_names(), &vec![10, 20, 30]);
    }

    #[test]
    fn named_cospan_add_boundary_node_known_target() {
        let mut cospan: NamedCospan<char, &str, &str> =
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec!["left1"], vec!["right1"]).unwrap();

        // Add left boundary node pointing to existing middle
        let idx = cospan
            .add_boundary_node_known_target(0, Left("left2"))
            .unwrap();
        assert!(matches!(idx, Left(_)));
        assert_eq!(cospan.left_names().len(), 2);

        // Add right boundary node pointing to existing middle
        let idx = cospan
            .add_boundary_node_known_target(0, Right("right2"))
            .unwrap();
        assert!(matches!(idx, Right(_)));
        assert_eq!(cospan.right_names().len(), 2);
    }

    #[test]
    fn named_cospan_add_boundary_node_unknown_target() {
        let mut cospan: NamedCospan<char, &str, &str> =
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec!["left1"], vec!["right1"]).unwrap();

        // Add left boundary with new middle node
        let idx = cospan
            .add_boundary_node_unknown_target('b', Left("left2"))
            .unwrap();
        assert!(matches!(idx, Left(_)));

        // Add right boundary with new middle node
        let idx = cospan
            .add_boundary_node_unknown_target('c', Right("right2"))
            .unwrap();
        assert!(matches!(idx, Right(_)));
    }

    #[test]
    fn named_cospan_delete_boundary_node() {
        let mut cospan: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0, 1], vec![0], vec!['a', 'b'], vec![1, 2], vec![3]).unwrap();

        cospan.delete_boundary_node(Left(0));
        assert_eq!(cospan.left_names().len(), 1);

        cospan.delete_boundary_node(Right(0));
        assert_eq!(cospan.right_names().len(), 0);
    }

    #[test]
    fn named_cospan_delete_boundary_node_by_name() {
        let mut cospan: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0, 1], vec![0], vec!['a', 'b'], vec![1, 2], vec![3]).unwrap();

        cospan.delete_boundary_node_by_name(Left(1));
        assert_eq!(cospan.left_names().len(), 1);
        assert!(!cospan.left_names().contains(&1));

        cospan.delete_boundary_node_by_name(Right(3));
        assert!(cospan.right_names().is_empty());
    }

    #[test]
    fn named_cospan_map_to_same() {
        let mut cospan: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0, 0], vec![0], vec!['a'], vec![1, 2], vec![3]).unwrap();

        // Both left nodes map to same middle
        assert!(cospan.map_to_same(Left(1), Left(2)));
        // Left and right map to same
        assert!(cospan.map_to_same(Left(1), Right(3)));
        // Non-existent node
        assert!(!cospan.map_to_same(Left(999), Left(1)));
    }

    #[test]
    fn named_cospan_connect_pair() {
        let mut cospan: NamedCospan<char, i32, i32> = NamedCospan::new(
            vec![0, 1],
            vec![0, 1],
            vec!['a', 'a'],
            vec![1, 2],
            vec![3, 4],
        )
        .unwrap();

        // Connect two nodes with same label
        cospan.connect_pair(Left(1), Left(2));
        // After connecting, they should map to same
        assert!(cospan.map_to_same(Left(1), Left(2)));
    }

    #[test]
    fn named_cospan_find_nodes_by_name_predicate() {
        let cospan: NamedCospan<char, i32, i32> = NamedCospan::new(
            vec![0, 1, 2],
            vec![0, 1],
            vec!['a', 'b', 'c'],
            vec![1, 2, 3],
            vec![4, 5],
        )
        .unwrap();

        // Find nodes with even names
        let found = cospan.find_nodes_by_name_predicate(|n| n % 2 == 0, |n| n % 2 == 0, false);
        assert_eq!(found.len(), 2); // 2 on left, 4 on right

        // Find at most one
        let found_one = cospan.find_nodes_by_name_predicate(|n| n % 2 == 0, |n| n % 2 == 0, true);
        assert_eq!(found_one.len(), 1);
    }

    #[test]
    fn named_cospan_change_boundary_node_name() {
        let mut cospan: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec![1], vec![2]).unwrap();

        cospan.change_boundary_node_name(Left((1, 10)));
        assert_eq!(cospan.left_names(), &vec![10]);

        cospan.change_boundary_node_name(Right((2, 20)));
        assert_eq!(cospan.right_names(), &vec![20]);
    }

    #[test]
    fn named_cospan_change_boundary_node_names() {
        let mut cospan: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0, 1], vec![0], vec!['a', 'b'], vec![1, 2], vec![3]).unwrap();

        // Change all left names
        let left_fn = |n: &mut i32| *n *= 10;
        cospan.change_boundary_node_names::<_, fn(&mut i32)>(Left(left_fn));
        assert_eq!(cospan.left_names(), &vec![10, 20]);

        // Change all right names
        let right_fn = |n: &mut i32| *n *= 100;
        cospan.change_boundary_node_names::<fn(&mut i32), _>(Right(right_fn));
        assert_eq!(cospan.right_names(), &vec![300]);
    }

    #[test]
    fn named_cospan_add_middle() {
        let mut cospan: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec![1], vec![2]).unwrap();

        cospan.add_middle('b');
        // Middle now has 2 elements
    }

    #[test]
    fn named_cospan_map() {
        let cospan: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec![1], vec![2]).unwrap();

        let mapped = cospan.map(|c| c.to_ascii_uppercase());
        assert_eq!(mapped.domain(), vec!['A']);
    }

    #[test]
    fn named_cospan_monoidal() {
        let cospan1: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec![1], vec![2]).unwrap();
        let cospan2: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0], vec![0], vec!['b'], vec![3], vec![4]).unwrap();

        let mut combined = cospan1;
        combined.monoidal(cospan2);

        assert_eq!(combined.left_names(), &vec![1, 3]);
        assert_eq!(combined.right_names(), &vec![2, 4]);
    }

    #[test]
    fn named_cospan_compose() {
        let cospan1: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec![1], vec![2]).unwrap();
        let cospan2: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec![3], vec![4]).unwrap();

        let composed = cospan1.compose(&cospan2);
        assert!(composed.is_ok());
        let result = composed.unwrap();
        assert_eq!(result.left_names(), &vec![1]);
        assert_eq!(result.right_names(), &vec![4]);
    }

    #[test]
    fn named_cospan_composable() {
        let cospan1: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec![1], vec![2]).unwrap();
        let cospan2: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec![3], vec![4]).unwrap();

        assert!(cospan1.composable(&cospan2).is_ok());

        let cospan3: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0], vec![0], vec!['b'], vec![5], vec![6]).unwrap();
        assert!(cospan1.composable(&cospan3).is_err());
    }

    #[test]
    fn named_cospan_permute_side() {
        use permutations::Permutation;

        let mut cospan: NamedCospan<char, i32, i32> = NamedCospan::new(
            vec![0, 1],
            vec![0, 1],
            vec!['a', 'b'],
            vec![1, 2],
            vec![3, 4],
        )
        .unwrap();

        let p = Permutation::rotation_left(2, 1);

        // Permute left side
        cospan.permute_side(&p, false);
        assert_eq!(cospan.left_names(), &vec![2, 1]);

        // Permute right side
        cospan.permute_side(&p, true);
        assert_eq!(cospan.right_names(), &vec![4, 3]);
    }

    #[test]
    fn named_cospan_assert_valid() {
        let cospan: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec![1], vec![2]).unwrap();
        cospan.assert_valid();
        cospan.assert_valid_nohash();
    }

    #[test]
    fn permutatation_manual() {
        use super::NamedCospan;
        use permutations::Permutation;
        #[derive(Debug, PartialEq, Eq, Clone, Copy)]
        enum Color {
            Red,
            Green,
            Blue,
        }
        let full_types: Vec<Color> = vec![Color::Red, Color::Green, Color::Blue];
        let cospan = NamedCospan::<Color, Color, Color>::from_permutation_extra_data_on_domain(
            Permutation::rotation_left(3, 1),
            &full_types,
            &full_types,
            |z| (z, z),
        )
        .unwrap();
        let cospan_2 = NamedCospan::<Color, Color, Color>::from_permutation_extra_data_on_domain(
            Permutation::rotation_left(3, 2),
            &[Color::Blue, Color::Red, Color::Green],
            &[Color::Green, Color::Blue, Color::Red],
            |z| (z, z),
        )
        .unwrap();
        let mid_interface_1 = cospan.codomain();
        let mid_interface_2 = cospan_2.domain();
        let comp = cospan.compose(&cospan_2);
        #[allow(clippy::match_wild_err_arm)]
        match comp {
            Ok(real_res) => {
                let expected_res = NamedCospan::identity(&full_types, &full_types, |z| (z, z));
                assert_eq!(expected_res.domain(), real_res.domain());
                assert_eq!(expected_res.codomain(), real_res.codomain());
            }
            Err(_e) => {
                panic!(
                    "Could not compose simple example because {mid_interface_1:?} did not match {mid_interface_2:?}"
                );
            }
        }

        let cospan = NamedCospan::<Color, Color, Color>::from_permutation_extra_data_on_codomain(
            Permutation::rotation_left(3, 1),
            &full_types,
            &full_types,
            |z| (z, z),
        )
        .unwrap();
        let cospan_2 = NamedCospan::<Color, Color, Color>::from_permutation_extra_data_on_codomain(
            Permutation::rotation_left(3, 2),
            &[Color::Green, Color::Blue, Color::Red],
            &[Color::Green, Color::Blue, Color::Red],
            |z| (z, z),
        )
        .unwrap();
        let mid_interface_1 = cospan.codomain();
        let mid_interface_2 = cospan_2.domain();
        let comp = cospan.compose(&cospan_2);
        #[allow(clippy::match_wild_err_arm)]
        match comp {
            Ok(real_res) => {
                let expected_res = NamedCospan::identity(
                    &[Color::Green, Color::Blue, Color::Red],
                    &[Color::Green, Color::Blue, Color::Red],
                    |z| (z, z),
                );
                assert_eq!(expected_res.domain(), real_res.domain());
                assert_eq!(expected_res.codomain(), real_res.codomain());
            }
            Err(_e) => {
                panic!(
                    "Could not compose simple example because {mid_interface_1:?} did not match {mid_interface_2:?}"
                );
            }
        }
    }

    #[test]
    fn permutatation_automatic() {
        use super::NamedCospan;
        use crate::utils::rand_perm;
        use rand::RngExt;
        let n_max = 10;
        let mut rng = StdRng::seed_from_u64(4001);
        let n = rng.random_range(2..n_max);

        for trial_num in 0..20 {
            let types_as_on_source = trial_num % 2 == 0;
            let build = |p: permutations::Permutation| {
                let types = (0..n).map(|_| ()).collect::<Vec<_>>();
                let prenames = (0..n).collect::<Vec<usize>>();
                if types_as_on_source {
                    NamedCospan::from_permutation_extra_data_on_domain(p, &types, &prenames, |_| {
                        ((), ())
                    })
                } else {
                    NamedCospan::from_permutation_extra_data_on_codomain(
                        p,
                        &types,
                        &prenames,
                        |_| ((), ()),
                    )
                }
                .unwrap()
            };
            let p1 = rand_perm(n, n * 2, &mut rng);
            let p2 = rand_perm(n, n * 2, &mut rng);
            let prod = p1.clone() * p2.clone();
            let cospan_p1 = build(p1);
            let cospan_p2 = build(p2);
            let cospan_prod = cospan_p1.compose(&cospan_p2);
            match cospan_prod {
                Ok(real_res) => {
                    let expected_res = build(prod);
                    assert_eq!(real_res.domain(), expected_res.domain());
                    assert_eq!(real_res.codomain(), expected_res.codomain());
                    assert_eq!(real_res.left_names, expected_res.left_names);
                    assert_eq!(real_res.right_names, expected_res.right_names);
                    assert_eq!(
                        real_res.cospan.left_to_middle(),
                        expected_res.cospan.left_to_middle()
                    );
                    assert_eq!(
                        real_res.cospan.right_to_middle(),
                        expected_res.cospan.right_to_middle()
                    );
                }
                Err(e) => {
                    panic!("Could not compose simple example {e:?}")
                }
            }
        }
    }
}
