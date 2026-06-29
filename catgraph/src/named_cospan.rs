//! Cospan with named boundary nodes (ports).
//!
//! Wraps [`Cospan`] and attaches unique names to domain and codomain nodes,
//! enabling port-level mutation, lookup, and predicate-based search.

use crate::errors::CatgraphError;

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
    pub fn assert_valid_nohash(&self, check_id: bool) {
        self.cospan.assert_valid(check_id, true);
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
    /// Name uniqueness is assumed but not enforced (port names may lack `Hash`).
    ///
    /// # Panics
    ///
    /// Panics if `left_names.len() != left.len()` or `right_names.len() != right.len()`.
    #[must_use]
    pub fn new(
        left: Vec<MiddleIndex>,
        right: Vec<MiddleIndex>,
        middle: Vec<Lambda>,
        left_names: Vec<LeftPortName>,
        right_names: Vec<RightPortName>,
    ) -> Self {
        assert!(
            left_names.len() == left.len(),
            "There must be names for everything in the domain and no others"
        );
        assert!(
            right_names.len() == right.len(),
            "There must be names for everything in the codomain and no others"
        );
        Self {
            cospan: Cospan::new(left, right, middle),
            left_names,
            right_names,
        }
    }

    /// The named cospan with empty domain, codomain, and middle set.
    #[must_use]
    pub fn empty() -> Self {
        Self::new(vec![], vec![], vec![], vec![], vec![])
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

    /// Build a named cospan from a permutation, deriving port names from `prenames`.
    ///
    /// When `types_as_on_domain` is true, `types` and `prenames` order follows the domain side;
    /// the codomain side is reordered by the permutation (and vice versa for false).
    ///
    /// # Panics
    ///
    /// Panics if `types.len() != prenames.len()`.
    #[allow(clippy::needless_pass_by_value)]
    pub fn from_permutation_extra_data<T, F>(
        p: Permutation,
        types: &[Lambda],
        types_as_on_domain: bool,
        prenames: &[T],
        prename_to_name: F,
    ) -> Self
    where
        T: Clone,
        F: Fn(T) -> (LeftPortName, RightPortName),
    {
        assert_eq!(types.len(), prenames.len());
        let cospan = Cospan::from_permutation(p.clone(), types, types_as_on_domain).unwrap();
        let (left_names, right_names) = if types_as_on_domain {
            (
                prenames
                    .iter()
                    .map(|pre| prename_to_name(pre.clone()).0)
                    .collect(),
                p.inv()
                    .permute(prenames)
                    .iter()
                    .map(|pre| prename_to_name(pre.clone()).1)
                    .collect(),
            )
        } else {
            (
                p.permute(prenames)
                    .iter()
                    .map(|pre| prename_to_name(pre.clone()).0)
                    .collect(),
                prenames
                    .iter()
                    .map(|pre| prename_to_name(pre.clone()).1)
                    .collect(),
            )
        };

        Self {
            cospan,
            left_names,
            right_names,
        }
    }

    /// Add a named boundary node targeting an existing middle index. Side determined by `new_name`.
    pub fn add_boundary_node_known_target(
        &mut self,
        new_arrow: MiddleIndex,
        new_name: Either<LeftPortName, RightPortName>,
    ) -> Either<LeftIndex, RightIndex> {
        self.add_boundary_node(Left(new_arrow), new_name)
    }

    /// Add a named boundary node that creates a new middle vertex with the given label.
    pub fn add_boundary_node_unknown_target(
        &mut self,
        new_arrow: Lambda,
        new_name: Either<LeftPortName, RightPortName>,
    ) -> Either<LeftIndex, RightIndex> {
        self.add_boundary_node(Right(new_arrow), new_name)
    }

    /// Add a named boundary node to new or existing middle vertex.
    ///
    /// # Panics
    ///
    /// Panics if the new name already exists on the relevant boundary side.
    pub fn add_boundary_node(
        &mut self,
        new_arrow: MiddleIndexOrLambda<Lambda>,
        new_name: Either<LeftPortName, RightPortName>,
    ) -> Either<LeftIndex, RightIndex> {
        self.cospan.add_boundary_node(match new_name {
            Left(new_name_real) => {
                assert!(!self.left_names.contains(&new_name_real));
                self.left_names.push(new_name_real);
                Left(new_arrow)
            }
            Right(new_name_real) => {
                assert!(!self.right_names.contains(&new_name_real));
                self.right_names.push(new_name_real);
                Right(new_arrow)
            }
        })
    }

    /// Remove a boundary node by index, keeping names in sync (uses `swap_remove` internally).
    pub fn delete_boundary_node(&mut self, which_node: Either<LeftIndex, RightIndex>) {
        /*
        CAUTION : relies on knowing that cospan uses swap_remove when deleting a node
            the implementation of delete_boundary_node on Cospan<Lambda>
        */
        match which_node {
            Left(z) => {
                self.left_names.swap_remove(z);
            }
            Right(z) => {
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
    /// When `at_most_one` is true, short-circuits after the first match.
    /// Parallelized with rayon when total boundary size >= 256.
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

    /// Append a new vertex to the middle set with the given label.
    pub fn add_middle(&mut self, new_middle: Lambda) {
        self.cospan.add_middle(new_middle);
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
    pub fn assert_valid(&self, check_id: bool) {
        self.assert_valid_nohash(check_id);
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
    fn permute_side(&mut self, p: &Permutation, of_right_leg: bool) {
        if of_right_leg {
            in_place_permute(&mut self.right_names, p);
        } else {
            in_place_permute(&mut self.left_names, p);
        }
        self.cospan.permute_side(p, of_right_leg);
    }

    fn from_permutation(
        _p: Permutation,
        _types: &[Lambda],
        _types_as_on_domain: bool,
    ) -> Result<Self, CatgraphError> {
        Err(CatgraphError::Composition {
            message: "NamedCospan::from_permutation requires port name data; use from_permutation_extra_data instead".to_string(),
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

    #[test]
    fn named_cospan_new() {
        let cospan: NamedCospan<char, &str, &str> = NamedCospan::new(
            vec![0, 1],
            vec![0],
            vec!['a', 'b'],
            vec!["x", "y"],
            vec!["z"],
        );
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
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec!["left1"], vec!["right1"]);

        // Add left boundary node pointing to existing middle
        let idx = cospan.add_boundary_node_known_target(0, Left("left2"));
        assert!(matches!(idx, Left(_)));
        assert_eq!(cospan.left_names().len(), 2);

        // Add right boundary node pointing to existing middle
        let idx = cospan.add_boundary_node_known_target(0, Right("right2"));
        assert!(matches!(idx, Right(_)));
        assert_eq!(cospan.right_names().len(), 2);
    }

    #[test]
    fn named_cospan_add_boundary_node_unknown_target() {
        let mut cospan: NamedCospan<char, &str, &str> =
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec!["left1"], vec!["right1"]);

        // Add left boundary with new middle node
        let idx = cospan.add_boundary_node_unknown_target('b', Left("left2"));
        assert!(matches!(idx, Left(_)));

        // Add right boundary with new middle node
        let idx = cospan.add_boundary_node_unknown_target('c', Right("right2"));
        assert!(matches!(idx, Right(_)));
    }

    #[test]
    fn named_cospan_delete_boundary_node() {
        let mut cospan: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0, 1], vec![0], vec!['a', 'b'], vec![1, 2], vec![3]);

        cospan.delete_boundary_node(Left(0));
        assert_eq!(cospan.left_names().len(), 1);

        cospan.delete_boundary_node(Right(0));
        assert_eq!(cospan.right_names().len(), 0);
    }

    #[test]
    fn named_cospan_delete_boundary_node_by_name() {
        let mut cospan: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0, 1], vec![0], vec!['a', 'b'], vec![1, 2], vec![3]);

        cospan.delete_boundary_node_by_name(Left(1));
        assert_eq!(cospan.left_names().len(), 1);
        assert!(!cospan.left_names().contains(&1));

        cospan.delete_boundary_node_by_name(Right(3));
        assert!(cospan.right_names().is_empty());
    }

    #[test]
    fn named_cospan_map_to_same() {
        let mut cospan: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0, 0], vec![0], vec!['a'], vec![1, 2], vec![3]);

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
        );

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
        );

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
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec![1], vec![2]);

        cospan.change_boundary_node_name(Left((1, 10)));
        assert_eq!(cospan.left_names(), &vec![10]);

        cospan.change_boundary_node_name(Right((2, 20)));
        assert_eq!(cospan.right_names(), &vec![20]);
    }

    #[test]
    fn named_cospan_change_boundary_node_names() {
        let mut cospan: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0, 1], vec![0], vec!['a', 'b'], vec![1, 2], vec![3]);

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
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec![1], vec![2]);

        cospan.add_middle('b');
        // Middle now has 2 elements
    }

    #[test]
    fn named_cospan_map() {
        let cospan: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec![1], vec![2]);

        let mapped = cospan.map(|c| c.to_ascii_uppercase());
        assert_eq!(mapped.domain(), vec!['A']);
    }

    #[test]
    fn named_cospan_monoidal() {
        let cospan1: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec![1], vec![2]);
        let cospan2: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0], vec![0], vec!['b'], vec![3], vec![4]);

        let mut combined = cospan1;
        combined.monoidal(cospan2);

        assert_eq!(combined.left_names(), &vec![1, 3]);
        assert_eq!(combined.right_names(), &vec![2, 4]);
    }

    #[test]
    fn named_cospan_compose() {
        let cospan1: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec![1], vec![2]);
        let cospan2: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec![3], vec![4]);

        let composed = cospan1.compose(&cospan2);
        assert!(composed.is_ok());
        let result = composed.unwrap();
        assert_eq!(result.left_names(), &vec![1]);
        assert_eq!(result.right_names(), &vec![4]);
    }

    #[test]
    fn named_cospan_composable() {
        let cospan1: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec![1], vec![2]);
        let cospan2: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec![3], vec![4]);

        assert!(cospan1.composable(&cospan2).is_ok());

        let cospan3: NamedCospan<char, i32, i32> =
            NamedCospan::new(vec![0], vec![0], vec!['b'], vec![5], vec![6]);
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
        );

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
            NamedCospan::new(vec![0], vec![0], vec!['a'], vec![1], vec![2]);
        cospan.assert_valid(false);
        cospan.assert_valid_nohash(false);
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
        let type_names_on_source = true;
        let cospan = NamedCospan::<Color, Color, Color>::from_permutation_extra_data(
            Permutation::rotation_left(3, 1),
            &full_types,
            type_names_on_source,
            &full_types,
            |z| (z, z),
        );
        let cospan_2 = NamedCospan::<Color, Color, Color>::from_permutation_extra_data(
            Permutation::rotation_left(3, 2),
            &[Color::Blue, Color::Red, Color::Green],
            type_names_on_source,
            &[Color::Green, Color::Blue, Color::Red],
            |z| (z, z),
        );
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

        let type_names_on_source = false;
        let cospan = NamedCospan::<Color, Color, Color>::from_permutation_extra_data(
            Permutation::rotation_left(3, 1),
            &full_types,
            type_names_on_source,
            &full_types,
            |z| (z, z),
        );
        let cospan_2 = NamedCospan::<Color, Color, Color>::from_permutation_extra_data(
            Permutation::rotation_left(3, 2),
            &[Color::Green, Color::Blue, Color::Red],
            type_names_on_source,
            &[Color::Green, Color::Blue, Color::Red],
            |z| (z, z),
        );
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
            let p1 = rand_perm(n, n * 2, &mut rng);
            let p2 = rand_perm(n, n * 2, &mut rng);
            let prod = p1.clone() * p2.clone();
            let cospan_p1 = NamedCospan::from_permutation_extra_data(
                p1,
                &(0..n).map(|_| ()).collect::<Vec<_>>(),
                types_as_on_source,
                &(0..n).collect::<Vec<usize>>(),
                |_| ((), ()),
            );
            let cospan_p2 = NamedCospan::from_permutation_extra_data(
                p2,
                &(0..n).map(|_| ()).collect::<Vec<_>>(),
                types_as_on_source,
                &(0..n).collect::<Vec<_>>(),
                |_| ((), ()),
            );
            let cospan_prod = cospan_p1.compose(&cospan_p2);
            match cospan_prod {
                Ok(real_res) => {
                    let expected_res = NamedCospan::from_permutation_extra_data(
                        prod,
                        &(0..n).map(|_| ()).collect::<Vec<_>>(),
                        types_as_on_source,
                        &(0..n).collect::<Vec<usize>>(),
                        |_| ((), ()),
                    );
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
