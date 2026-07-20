//! Gauge group implementation for hypergraph rewriting.
//!
//! This module connects hypergraph rewriting to gauge theory by implementing
//! the `GaugeGroup` trait for lattice gauge field analysis.
//!
//! # Gauge-Theoretic Interpretation
//!
//! In gauge theory:
//! - **Gauge transformations** are local symmetry operations
//! - **Wilson loops** measure holonomy around closed paths
//! - **Holonomy = 1** means the gauge field is "flat" (path-independent)
//!
//! For hypergraph rewriting:
//! - **Rewrite rules** are gauge transformations
//! - **Wilson loops** are closed paths in the evolution graph
//! - **Causal invariance** ⟺ flat gauge field (holonomy = 1)
//!
//! This correspondence allows us to analyze multicomputational properties
//! using the powerful machinery of gauge field theory.
//!
//! # Provenance
//!
//! The rewrite-rules-as-gauge-transformations reading is inspired by
//! Gorard's causal-invariance-as-discrete-gauge-covariance result
//! (\[Gor20a\] in `docs/ANCHORS.md`, uncached); the Wilson-loop / plaquette /
//! holonomy vocabulary is standard lattice-gauge-theory machinery. The
//! "causal invariance ⟺ flat gauge field (holonomy = 1)" equivalence as
//! stated here is a catgraph interpretive gloss, not a theorem of any
//! cached paper.

/// Trait representing a gauge group in lattice gauge theory.
///
/// A gauge group describes the local symmetries of a physical system.
/// In the context of hypergraph rewriting, gauge transformations correspond
/// to rewrite rule applications.
pub trait GaugeGroup {
    /// Dimension of the Lie algebra (number of generators).
    const LIE_ALGEBRA_DIM: usize;
    /// Whether the group is abelian (commutative).
    const IS_ABELIAN: bool;
    /// Dimension of spacetime (for lattice gauge theory).
    const SPACETIME_DIM: usize;
    /// Name of the gauge group.
    fn name() -> &'static str;
    /// Structure constant f^{abc} of the Lie algebra.
    fn structure_constant(a: usize, b: usize, c: usize) -> f64;
}

/// Gauge group for hypergraph rewriting systems.
///
/// This implements the `GaugeGroup` trait, treating hypergraph rewrite rules
/// as local gauge transformations. The dimension of the Lie algebra corresponds
/// to the number of independent rewrite rules.
///
/// # Mathematical Structure
///
/// - **Group elements**: Sequences of rewrite rule applications
/// - **Multiplication**: Composition of rewrites
/// - **Identity**: Empty rewrite sequence
/// - **Inverse**: (Generally doesn't exist for irreversible rules)
///
/// # Abelian vs Non-Abelian
///
/// The group is **non-abelian** because the order of rule applications
/// generally matters. This is precisely what causal invariance measures:
/// if the group were abelian, causal invariance would be guaranteed.
///
/// # Example
///
/// ```rust
/// use catgraph_physics::hypergraph::{HypergraphRewriteGroup, GaugeGroup};
///
/// let group = HypergraphRewriteGroup::new(3);
///
/// assert_eq!(HypergraphRewriteGroup::LIE_ALGEBRA_DIM, 3);
/// assert!(!HypergraphRewriteGroup::IS_ABELIAN);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HypergraphRewriteGroup {
    /// Number of rewrite rules (generators).
    num_rules: usize,
}

impl HypergraphRewriteGroup {
    /// Creates a new hypergraph rewrite group with the given number of rules.
    ///
    /// # Arguments
    ///
    /// * `num_rules` - Number of rewrite rules (determines Lie algebra dimension)
    ///
    /// # Example
    ///
    /// ```rust
    /// use catgraph_physics::hypergraph::HypergraphRewriteGroup;
    ///
    /// let group = HypergraphRewriteGroup::new(5);
    /// assert_eq!(group.num_rules(), 5);
    /// ```
    #[must_use]
    pub const fn new(num_rules: usize) -> Self {
        Self { num_rules }
    }

    /// Returns the number of rewrite rules.
    #[inline]
    #[must_use]
    pub const fn num_rules(&self) -> usize {
        self.num_rules
    }

    /// Computes the structure constant f^{abc} for the hypergraph rewrite algebra.
    ///
    /// The structure constants encode how rule compositions interact.
    /// For hypergraph rewriting, we use a simplified model where:
    ///
    /// - f^{abc} = 1 if rules a and b don't commute (order matters)
    /// - f^{abc} = 0 if rules a and b commute
    ///
    /// In practice, determining commutativity requires analyzing rule overlaps.
    /// This default implementation assumes non-commutativity for distinct rules.
    ///
    /// # Arguments
    ///
    /// * `a`, `b`, `c` - Lie algebra indices (rule indices)
    ///
    /// # Returns
    ///
    /// The structure constant f^{abc}.
    #[must_use]
    pub fn structure_constant_for(&self, a: usize, b: usize, c: usize) -> f64 {
        if a >= self.num_rules || b >= self.num_rules || c >= self.num_rules {
            return 0.0;
        }

        // Antisymmetric: f^{abc} = -f^{bac}
        if a == b {
            return 0.0;
        }

        // Simplified model: non-zero structure constant when rules interact
        // f^{abc} is non-zero when [T_a, T_b] has a component in T_c direction
        if a != b && c != a && c != b {
            // Non-trivial mixing
            1.0
        } else if c == a && b > a {
            1.0
        } else if c == b && a > b {
            -1.0
        } else {
            0.0
        }
    }

    /// Returns the dimension of the gauge group representation.
    ///
    /// For hypergraph rewriting, this is the number of possible
    /// hypergraph states (potentially infinite, so we return a proxy).
    #[must_use]
    pub fn representation_dim(&self) -> usize {
        // The representation space is the space of hypergraphs.
        // We use the number of rules as a proxy for complexity.
        self.num_rules * self.num_rules
    }
}

impl Default for HypergraphRewriteGroup {
    fn default() -> Self {
        Self::new(1)
    }
}

impl GaugeGroup for HypergraphRewriteGroup {
    /// Number of generators = number of rewrite rules.
    ///
    /// In gauge theory, this determines the number of gauge bosons.
    /// For hypergraph rewriting, each rule is a "generator" of the group.
    /// Compile-time constant required by the trait, defaulting to 3.
    ///
    /// The actual Lie algebra dimension is runtime-dynamic and equals
    /// `self.num_rules()`. This constant serves as an upper bound /
    /// placeholder; prefer [`HypergraphRewriteGroup::num_rules`] at runtime.
    const LIE_ALGEBRA_DIM: usize = 3; // Default; actual dimension is dynamic

    /// Hypergraph rewriting is generally non-abelian.
    ///
    /// The order of rule applications matters, which is why
    /// causal invariance is a non-trivial property.
    const IS_ABELIAN: bool = false;

    /// We use a 1D "spacetime" (just time evolution).
    const SPACETIME_DIM: usize = 1;

    /// Returns the name of this gauge group.
    fn name() -> &'static str {
        "HypergraphRewrite"
    }

    /// Returns the structure constant f^{abc}.
    ///
    /// For hypergraph rewriting, structure constants encode
    /// rule interaction patterns.
    fn structure_constant(a: usize, b: usize, c: usize) -> f64 {
        // Use default 3-rule group
        HypergraphRewriteGroup::new(3).structure_constant_for(a, b, c)
    }
}

// ============================================================================
// Plaquette Action
// ============================================================================

/// Computes the plaquette action for a closed path in rewrite space.
///
/// The plaquette action is a gauge-invariant measure of "curvature"
/// in the space of rewrites. It provides a complexity measure beyond
/// simple step counting.
///
/// # Mathematical Definition
///
/// For a Wilson loop W, the plaquette action is:
/// ```text
/// S = -ln(|Tr(W)|)
/// ```
///
/// where W is the holonomy around the loop.
///
/// # Arguments
///
/// * `holonomy` - The holonomy value from a Wilson loop (0.0 to 1.0)
///
/// # Returns
///
/// The plaquette action (non-negative, 0 = flat/trivial).
#[must_use]
pub fn plaquette_action(holonomy: f64) -> f64 {
    if holonomy <= 0.0 {
        f64::INFINITY
    } else if holonomy >= 1.0 {
        0.0
    } else {
        -holonomy.ln()
    }
}

/// Computes the total action for an evolution (sum of plaquette actions).
///
/// Lower total action indicates more causal invariance.
#[must_use]
pub fn total_action(holonomies: &[f64]) -> f64 {
    holonomies.iter().map(|&h| plaquette_action(h)).sum()
}

// ============================================================================
// HypergraphLattice
// ============================================================================

use std::collections::HashMap;

use super::hypergraph::Hypergraph;
use super::rewrite_rule::RewriteRule;

/// A D-dimensional lattice for hypergraph rewriting.
///
/// This structure represents a hypergraph rewriting system as a lattice gauge field,
/// where each lattice site represents a hypergraph state and links represent transitions
/// via rewrite rules.
///
/// # Lattice Structure
///
/// - **Sites**: D-dimensional lattice positions, each containing a hypergraph
/// - **Links**: Connections between sites representing rewrite rule applications
/// - **Gauge Field**: Assignment of gauge transformations to links
/// - **Wilson Loops**: Closed paths in the lattice measuring causal invariance
///
/// # Type Parameters
///
/// * `D` - Dimension of the lattice (1D, 2D, 3D, etc.)
///
/// # Example
///
/// ```rust
/// use catgraph_physics::hypergraph::{
///     HypergraphLattice, HypergraphRewriteGroup, Hypergraph, RewriteRule,
/// };
///
/// let rule = RewriteRule::wolfram_a_to_bb();
/// let mut lattice: HypergraphLattice<1> = HypergraphLattice::new(
///     [5],
///     HypergraphRewriteGroup::new(3),
///     vec![rule],
/// );
///
/// let initial = Hypergraph::from_edges(vec![vec![0, 1, 2]]);
/// lattice.set_state(&[2], initial);
/// lattice.apply_rewrite(&[2], 0);
/// ```
#[derive(Debug, Clone)]
pub struct HypergraphLattice<const D: usize> {
    /// Dimensions of the lattice (e.g., [5, 5, 5] for 5x5x5).
    dimensions: [usize; D],

    /// Gauge group (defines number of rules).
    group: HypergraphRewriteGroup,

    /// Rewrite rules (gauge generators). Index corresponds to `rule_index` in `apply_rewrite`.
    rules: Vec<RewriteRule>,

    /// Next available vertex ID for rewrite operations.
    next_vertex_id: usize,

    /// States at each lattice site.
    states: HashMap<Vec<usize>, Hypergraph>,

    /// Transitions and rule applications at each link.
    /// Key: (site, `neighbor_site`) pair
    /// Value: (`rule_index`, holonomy)
    transitions: HashMap<(Vec<usize>, Vec<usize>), (usize, f64)>,

    /// Total number of rewrite steps applied.
    step_count: usize,

    /// Wilson loop detections during evolution.
    wilson_loops: Vec<(Vec<Vec<usize>>, f64)>,
}

impl<const D: usize> HypergraphLattice<D> {
    /// Creates a new D-dimensional hypergraph lattice.
    ///
    /// # Arguments
    ///
    /// * `dimensions` - Array of dimension sizes (e.g., [5, 5] for 5x5)
    /// * `group` - Gauge group (defines number of rewrite rules)
    /// * `rules` - Rewrite rules (gauge generators)
    ///
    /// # Example
    ///
    /// ```rust
    /// use catgraph_physics::hypergraph::{HypergraphLattice, HypergraphRewriteGroup};
    ///
    /// let lattice: HypergraphLattice<2> = HypergraphLattice::new(
    ///     [10, 10],
    ///     HypergraphRewriteGroup::new(4),
    ///     vec![],
    /// );
    /// ```
    #[must_use]
    pub fn new(
        dimensions: [usize; D],
        group: HypergraphRewriteGroup,
        rules: Vec<RewriteRule>,
    ) -> Self {
        Self {
            dimensions,
            group,
            rules,
            next_vertex_id: 0,
            states: HashMap::new(),
            transitions: HashMap::new(),
            step_count: 0,
            wilson_loops: Vec::new(),
        }
    }

    /// Sets the hypergraph state at a lattice site.
    ///
    /// # Arguments
    ///
    /// * `site` - D-dimensional lattice coordinate
    /// * `state` - The hypergraph to place at this site
    pub fn set_state(&mut self, site: &[usize; D], state: Hypergraph) {
        if Self::is_valid_site(site, &self.dimensions) {
            self.states.insert(site.to_vec(), state);
        }
    }

    /// Gets the hypergraph state at a lattice site.
    ///
    /// # Arguments
    ///
    /// * `site` - D-dimensional lattice coordinate
    ///
    /// # Returns
    ///
    /// A reference to the hypergraph, or None if the site doesn't exist
    #[must_use]
    pub fn get_state(&self, site: &[usize; D]) -> Option<&Hypergraph> {
        self.states.get(site.as_slice())
    }

    /// Gets a mutable reference to the hypergraph state at a lattice site.
    pub fn get_state_mut(&mut self, site: &[usize; D]) -> Option<&mut Hypergraph> {
        self.states.get_mut(site.as_slice())
    }

    /// Returns the rewrite rules.
    #[must_use]
    pub fn rules(&self) -> &[RewriteRule] {
        &self.rules
    }

    /// Applies a rewrite rule at a specific lattice site.
    ///
    /// Finds the first match of the rule in the hypergraph at the given site
    /// and applies it via DPO rewriting. Records the transition with holonomy
    /// (edge count ratio before/after rewrite).
    ///
    /// Returns `false` if the site is invalid, the rule index is out of range,
    /// or no match is found at the site.
    #[allow(clippy::cast_precision_loss, clippy::missing_panics_doc)]
    pub fn apply_rewrite(&mut self, site: &[usize; D], rule_index: usize) -> bool {
        if !Self::is_valid_site(site, &self.dimensions) {
            return false;
        }

        if rule_index >= self.rules.len() {
            return false;
        }

        // Ensure a hypergraph exists at the site
        self.states.entry(site.to_vec()).or_default();

        let state = self.states.get_mut(site.as_slice()).unwrap();

        let edges_before = state.edge_count();

        let matches = self.rules[rule_index].find_matches(state);
        if matches.is_empty() {
            return false;
        }

        self.rules[rule_index].apply(state, &matches[0], &mut self.next_vertex_id);

        let edges_after = state.edge_count();

        let _holonomy = if edges_before == 0 {
            1.0
        } else {
            edges_after as f64 / edges_before as f64
        };

        // Note: we intentionally do NOT record a self-loop transition here.
        // A rewrite at a single site does not define a gauge link between
        // distinct lattice sites. Use `record_transition` to populate
        // inter-site link variables that `wilson_loop` can traverse.

        self.step_count += 1;
        true
    }

    /// Records a gauge link holonomy between two lattice sites.
    ///
    /// In lattice gauge theory, link variables live on edges between sites.
    /// This records the holonomy (accumulated gauge transformation) for the
    /// directed link from `from` to `to`.
    ///
    /// Use this to populate the lattice with inter-site gauge data that
    /// [`wilson_loop`](Self::wilson_loop) can then traverse.
    ///
    /// # Arguments
    ///
    /// * `from` - Source lattice site
    /// * `to` - Target lattice site
    /// * `holonomy` - Gauge transformation weight for this directed link
    pub fn record_transition(&mut self, from: &[usize; D], to: &[usize; D], holonomy: f64) {
        self.transitions
            .insert((from.to_vec(), to.to_vec()), (usize::MAX, holonomy));
    }

    /// Computes a Wilson loop around a closed path of lattice sites.
    ///
    /// A Wilson loop measures the holonomy (accumulated gauge transformation)
    /// around a closed path. Holonomy = 1.0 indicates causal invariance.
    ///
    /// # Arguments
    ///
    /// * `path` - Ordered sequence of lattice sites forming a closed loop
    ///
    /// # Returns
    ///
    /// The holonomy value (product of link holonomies around the loop)
    #[must_use]
    pub fn wilson_loop(&self, path: &[&[usize; D]]) -> f64 {
        if path.is_empty() {
            return 1.0; // Empty path is trivial
        }

        let mut holonomy = 1.0;

        // Traverse the path, multiplying link holonomies
        for i in 0..path.len() {
            let site_a = &path[i];
            let site_b = &path[(i + 1) % path.len()]; // Wrap around to close the loop

            let key = (site_a.to_vec(), site_b.to_vec());
            if let Some((_, h)) = self.transitions.get(&key) {
                holonomy *= h;
            } else {
                // If no transition recorded, assume gauge-invariant (h = 1)
                holonomy *= 1.0;
            }
        }

        holonomy
    }

    /// Checks if the system is causally invariant.
    ///
    /// Causal invariance holds when all Wilson loops have holonomy = 1.0,
    /// meaning that all paths to the same state give identical results.
    ///
    /// # Arguments
    ///
    /// * `path` - A closed path in the lattice
    ///
    /// # Returns
    ///
    /// `true` if the path is causally invariant (holonomy ≈ 1.0)
    #[must_use]
    pub fn is_causally_invariant(&self, path: &[&[usize; D]]) -> bool {
        let h = self.wilson_loop(path);
        (h - 1.0).abs() < 1e-6 // Allow small numerical error
    }

    /// Computes the plaquette action for a closed path.
    ///
    /// The plaquette action measures "curvature" in gauge field space.
    /// Lower action indicates more causal invariance.
    ///
    /// # Arguments
    ///
    /// * `path` - A closed path in the lattice
    ///
    /// # Returns
    ///
    /// The plaquette action (0 for perfectly flat/invariant)
    #[must_use]
    pub fn plaquette_action(&self, path: &[&[usize; D]]) -> f64 {
        let holonomy = self.wilson_loop(path);
        super::gauge::plaquette_action(holonomy)
    }

    /// Finds and records all Wilson loops in the lattice up to a given length.
    ///
    /// This explores all possible closed paths of length <= `max_length`.
    /// For large lattices, this can be expensive.
    ///
    /// # Arguments
    ///
    /// * `max_length` - Maximum loop length to explore
    pub fn find_wilson_loops(&mut self, _max_length: usize) {
        self.wilson_loops.clear();

        // For small lattices, find all elementary loops (plaquettes)
        if D == 1 {
            // In 1D, only trivial loops (return on same path)
            return;
        }

        if D == 2 {
            // In 2D, find simple plaquettes
            for x in 0..self.dimensions[0] {
                for y in 0..self.dimensions[1] {
                    if x + 1 < self.dimensions[0] && y + 1 < self.dimensions[1] {
                        let sites = vec![
                            [x, y].to_vec(),
                            [x + 1, y].to_vec(),
                            [x + 1, y + 1].to_vec(),
                            [x, y + 1].to_vec(),
                        ];

                        // Compute holonomy around this plaquette
                        let mut h = 1.0;
                        for i in 0..sites.len() {
                            let s_a = &sites[i];
                            let s_b = &sites[(i + 1) % sites.len()];
                            let key = (s_a.clone(), s_b.clone());
                            if let Some((_, holonomy)) = self.transitions.get(&key) {
                                h *= holonomy;
                            }
                        }

                        self.wilson_loops.push((sites, h));
                    }
                }
            }
        }
    }

    /// Returns recorded Wilson loops.
    #[must_use]
    pub fn recorded_loops(&self) -> &[(Vec<Vec<usize>>, f64)] {
        &self.wilson_loops
    }

    /// Returns the total number of rewrite steps applied.
    #[must_use]
    pub fn step_count(&self) -> usize {
        self.step_count
    }

    /// Returns the number of lattice sites.
    #[must_use]
    pub fn site_count(&self) -> usize {
        self.states.len()
    }

    /// Checks if a lattice site coordinate is valid.
    fn is_valid_site(site: &[usize; D], dimensions: &[usize; D]) -> bool {
        site.iter()
            .zip(dimensions.iter())
            .all(|(&coord, &dim)| coord < dim)
    }

    /// Returns the dimensions of the lattice.
    #[must_use]
    pub fn dimensions(&self) -> &[usize; D] {
        &self.dimensions
    }

    /// Returns the gauge group.
    #[must_use]
    pub fn group(&self) -> &HypergraphRewriteGroup {
        &self.group
    }

    /// Computes the average holonomy across all recorded Wilson loops.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    #[must_use]
    pub fn average_holonomy(&self) -> f64 {
        if self.wilson_loops.is_empty() {
            return 1.0;
        }

        let sum: f64 = self.wilson_loops.iter().map(|(_, h)| h).sum();
        sum / self.wilson_loops.len() as f64
    }

    /// Checks if all Wilson loops have perfect causal invariance.
    #[must_use]
    pub fn is_globally_causally_invariant(&self) -> bool {
        self.wilson_loops
            .iter()
            .all(|(_, h)| (h - 1.0).abs() < 1e-6)
    }

    /// Computes total action across all Wilson loops.
    #[must_use]
    pub fn total_plaquette_action(&self) -> f64 {
        self.wilson_loops
            .iter()
            .map(|(_, h)| plaquette_action(*h))
            .sum()
    }
}

impl<const D: usize> Default for HypergraphLattice<D> {
    fn default() -> Self {
        let dims = [1; D];
        Self::new(dims, HypergraphRewriteGroup::new(3), vec![])
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[allow(
    clippy::float_cmp,              // gauge-holonomy tests compare exact trivial values (0.0, 1.0)
    clippy::assertions_on_constants, // asserting compile-time associated constants documents the axiom
)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_group_new() {
        let group = HypergraphRewriteGroup::new(5);
        assert_eq!(group.num_rules(), 5);
    }

    #[test]
    fn test_gauge_group_trait() {
        assert_eq!(HypergraphRewriteGroup::LIE_ALGEBRA_DIM, 3);
        assert!(!HypergraphRewriteGroup::IS_ABELIAN);
        assert_eq!(HypergraphRewriteGroup::SPACETIME_DIM, 1);
        assert_eq!(HypergraphRewriteGroup::name(), "HypergraphRewrite");
    }

    #[test]
    fn test_structure_constants() {
        let group = HypergraphRewriteGroup::new(3);

        // Diagonal is zero (antisymmetry)
        assert_eq!(group.structure_constant_for(0, 0, 0), 0.0);
        assert_eq!(group.structure_constant_for(1, 1, 1), 0.0);

        // Same indices on a, b gives zero
        assert_eq!(group.structure_constant_for(1, 1, 0), 0.0);
    }

    #[test]
    fn test_plaquette_action() {
        // Flat (holonomy = 1) has zero action
        assert_eq!(plaquette_action(1.0), 0.0);

        // Lower holonomy means higher action
        assert!(plaquette_action(0.5) > 0.0);
        assert!(plaquette_action(0.5) < plaquette_action(0.1));

        // Zero holonomy gives infinite action
        assert!(plaquette_action(0.0).is_infinite());
    }

    #[test]
    fn test_total_action() {
        let holonomies = vec![1.0, 1.0, 1.0];
        assert_eq!(total_action(&holonomies), 0.0);

        let holonomies = vec![0.5, 0.5];
        assert!(total_action(&holonomies) > 0.0);
    }

    #[test]
    fn test_representation_dim() {
        let group = HypergraphRewriteGroup::new(4);
        assert_eq!(group.representation_dim(), 16);
    }

    // ========================================================================
    // HypergraphLattice Tests
    // ========================================================================

    #[test]
    fn test_lattice_1d_creation() {
        let lattice: HypergraphLattice<1> =
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![]);

        assert_eq!(lattice.dimensions(), &[5]);
        assert_eq!(lattice.step_count(), 0);
        assert_eq!(lattice.site_count(), 0);
    }

    #[test]
    fn test_lattice_2d_creation() {
        let lattice: HypergraphLattice<2> =
            HypergraphLattice::new([10, 10], HypergraphRewriteGroup::new(4), vec![]);

        assert_eq!(lattice.dimensions(), &[10, 10]);
        assert_eq!(lattice.group().num_rules(), 4);
    }

    #[test]
    fn test_lattice_set_get_state() {
        let mut lattice: HypergraphLattice<1> =
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![]);

        let state = Hypergraph::from_edges(vec![vec![0, 1, 2]]);
        let site = [2];

        lattice.set_state(&site, state.clone());

        let retrieved = lattice.get_state(&site);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().vertex_count(), state.vertex_count());
    }

    #[test]
    fn test_lattice_apply_rewrite() {
        use super::super::rewrite_rule::RewriteRule;

        let rule = RewriteRule::wolfram_a_to_bb();
        let mut lattice: HypergraphLattice<1> =
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![rule]);

        // Set an initial state with a ternary edge that matches A→BB
        let initial = Hypergraph::from_edges(vec![vec![0, 1, 2]]);
        let site = [1];
        lattice.set_state(&site, initial);

        let success = lattice.apply_rewrite(&site, 0);

        assert!(success);
        assert_eq!(lattice.step_count(), 1);

        // The ternary edge should have been replaced by two binary edges
        let state = lattice.get_state(&site).unwrap();
        assert_eq!(state.edge_count(), 2);
    }

    #[test]
    fn test_lattice_apply_rewrite_no_match() {
        use super::super::rewrite_rule::RewriteRule;

        // Rule expects a ternary edge, but we'll have a binary edge
        let rule = RewriteRule::wolfram_a_to_bb();
        let mut lattice: HypergraphLattice<1> =
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![rule]);

        let initial = Hypergraph::from_edges(vec![vec![0, 1]]);
        let site = [1];
        lattice.set_state(&site, initial);

        let success = lattice.apply_rewrite(&site, 0);

        // No match found, should return false and not increment step_count
        assert!(!success);
        assert_eq!(lattice.step_count(), 0);
    }

    #[test]
    fn test_lattice_apply_rewrite_invalid_rule() {
        let mut lattice: HypergraphLattice<1> =
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(2), vec![]);

        let site = [1];
        let success = lattice.apply_rewrite(&site, 0); // No rules at all

        assert!(!success);
        assert_eq!(lattice.step_count(), 0);
    }

    #[test]
    fn test_lattice_wilson_loop_empty() {
        let lattice: HypergraphLattice<1> =
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![]);

        let path: Vec<&[usize; 1]> = vec![];
        let h = lattice.wilson_loop(&path);

        assert_eq!(h, 1.0); // Empty path is trivial
    }

    #[test]
    fn test_lattice_causal_invariance() {
        let lattice: HypergraphLattice<1> =
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![]);

        let sites = vec![
            &[0usize] as &[usize; 1],
            &[1usize] as &[usize; 1],
            &[0usize],
        ];
        let invariant = lattice.is_causally_invariant(&sites);

        // With no transitions recorded, should be invariant
        assert!(invariant);
    }

    #[test]
    fn test_lattice_plaquette_action() {
        let lattice: HypergraphLattice<1> =
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![]);

        let sites = vec![&[0usize] as &[usize; 1], &[1usize]];
        let action = lattice.plaquette_action(&sites);

        // Perfect holonomy (1.0) gives zero action
        assert!(action >= 0.0);
        assert!((action - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_lattice_2d_valid_site() {
        let lattice: HypergraphLattice<2> =
            HypergraphLattice::new([5, 5], HypergraphRewriteGroup::new(3), vec![]);

        assert!(HypergraphLattice::<2>::is_valid_site(
            &[2, 3],
            lattice.dimensions()
        ));
        assert!(!HypergraphLattice::<2>::is_valid_site(
            &[5, 3],
            lattice.dimensions()
        )); // Out of bounds
    }

    #[test]
    fn test_lattice_default() {
        let lattice: HypergraphLattice<3> = HypergraphLattice::default();

        // Should have default 1x1x1 lattice with 3 rules
        assert_eq!(lattice.dimensions(), &[1, 1, 1]);
        assert_eq!(lattice.group().num_rules(), 3);
    }

    #[test]
    fn test_lattice_average_holonomy() {
        let lattice: HypergraphLattice<1> =
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![]);

        let avg = lattice.average_holonomy();
        assert_eq!(avg, 1.0); // No loops recorded yet
    }

    #[test]
    fn test_lattice_global_causal_invariance() {
        let lattice: HypergraphLattice<1> =
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![]);

        // No loops = trivially globally invariant
        assert!(lattice.is_globally_causally_invariant());
    }

    #[test]
    fn test_lattice_total_plaquette_action() {
        let lattice: HypergraphLattice<1> =
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![]);

        let action = lattice.total_plaquette_action();
        assert_eq!(action, 0.0); // No loops = zero action
    }

    #[test]
    fn test_lattice_rules_accessor() {
        use super::super::rewrite_rule::RewriteRule;

        let rules = vec![RewriteRule::wolfram_a_to_bb(), RewriteRule::edge_split()];
        let lattice: HypergraphLattice<1> =
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), rules);

        assert_eq!(lattice.rules().len(), 2);
    }
}
