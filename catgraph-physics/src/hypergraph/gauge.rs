//! Lattice-gauge reading of hypergraph rewriting: rewrite rules as gauge
//! transformations, closed rewrite paths as Wilson loops, an identity
//! holonomy as flat.
//!
//! A link carries a [`LinkVariable`]: `DMatrix<f64>` at side `link_dim`,
//! `Rotation3<f64>` and `UnitQuaternion<f64>` at `link_dim` 3, `Isometry3<f64>`
//! at `link_dim` 4. The holonomy of a closed path is the ordered product
//! `U_k · … · U_1`; its Wilson value is the trace of the carrier's defining
//! representation over that representation's dimension, and flatness compares
//! that representation entrywise against the identity.
//!
//! Provenance (`docs/ANCHORS.md`): inspired by \[Gor20a\]; the matrix link
//! variable, the path-ordered holonomy, the Wilson loop as a normalized trace
//! and the gauge transformation by vertex conjugation are standard lattice
//! gauge theory \[Wil74\]; "causal invariance ⟺ flat holonomy" is a catgraph
//! gloss, not a cited theorem.

/// A gauge group: Lie-algebra dimension, abelianness, spacetime dimension,
/// name, and structure constants.
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

/// [`GaugeGroup`] whose generators are rewrite rules: `LIE_ALGEBRA_DIM` = rule
/// count, non-abelian.
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

/// Plaquette action `S = −ln(holonomy)`: `0.0` for `holonomy >= 1.0`,
/// `f64::INFINITY` for `holonomy <= 0.0`.
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

/// Sum of [`plaquette_action`] over `holonomies`.
#[must_use]
pub fn total_action(holonomies: &[f64]) -> f64 {
    holonomies.iter().map(|&h| plaquette_action(h)).sum()
}

// ============================================================================
// HypergraphLattice
// ============================================================================

use std::collections::HashMap;
use std::fmt;

use nalgebra::{DMatrix, Isometry3, Matrix3, Matrix4, Rotation3, UnitQuaternion};

use super::hypergraph::Hypergraph;
use super::rewrite_rule::RewriteRule;

// ============================================================================
// LinkVariable
// ============================================================================

/// Tolerance on a typed link variable's departure from its carrier's own
/// invariant: entries of `R · Rᵀ − I` for `Rotation3<f64>`, and `|‖q‖ − 1|`
/// for `UnitQuaternion<f64>` and for the rotation of `Isometry3<f64>`.
pub const TYPED_LINK_TOL: f64 = 1e-9;

/// A value carried by a directed lattice link: composable, and read through a
/// defining representation whose dimension is the one
/// [`identity`](LinkVariable::identity) was given.
pub trait LinkVariable: Clone + fmt::Debug + PartialEq {
    /// The identity of defining-representation dimension `dim`, or `None`
    /// when the carrier holds no element of that dimension.
    fn identity(dim: usize) -> Option<Self>;

    /// The composite `self · inner`.
    fn compose(&self, inner: &Self) -> Self;

    /// The carrier's inverse construction applied to `self`, or `None`.
    fn inverse(&self) -> Option<Self>;

    /// Reports whether `self` is admissible at defining-representation
    /// dimension `dim`.
    fn is_admissible(&self, dim: usize) -> bool;

    /// Trace of the defining representation of `self` over that
    /// representation's dimension.
    fn wilson(&self) -> f64;

    /// Reports whether every entry of the defining representation of `self`
    /// minus the identity is below `eps` in absolute value.
    fn is_flat(&self, eps: f64) -> bool;
}

/// Square real matrices of any side, composing by matrix product.
impl LinkVariable for DMatrix<f64> {
    /// The `dim` × `dim` identity, or `None` when `dim` is `0`.
    fn identity(dim: usize) -> Option<Self> {
        (dim > 0).then(|| Self::identity(dim, dim))
    }

    /// The matrix product `self * inner`, whose shape needs `self`'s column
    /// count to equal `inner`'s row count.
    fn compose(&self, inner: &Self) -> Self {
        self * inner
    }

    /// `self.clone().try_inverse()`.
    fn inverse(&self) -> Option<Self> {
        self.clone().try_inverse()
    }

    /// Reports whether `self` is `dim` × `dim`, every entry is finite, and
    /// `self.clone().try_inverse()` is `Some`.
    fn is_admissible(&self, dim: usize) -> bool {
        if self.nrows() != dim || self.ncols() != dim {
            return false;
        }
        if !self.iter().all(|entry| entry.is_finite()) {
            return false;
        }
        self.clone().try_inverse().is_some()
    }

    /// `tr(self) / self.nrows()`, over square `self`.
    #[allow(clippy::cast_precision_loss)]
    fn wilson(&self) -> f64 {
        self.trace() / self.nrows() as f64
    }

    /// Reports whether every entry of `self` minus the identity of `self`'s
    /// shape is below `eps` in absolute value.
    fn is_flat(&self, eps: f64) -> bool {
        let identity = Self::identity(self.nrows(), self.ncols());
        (self - identity).iter().all(|d| d.abs() < eps)
    }
}

/// SO(3) rotations in their 3 × 3 defining representation.
impl LinkVariable for Rotation3<f64> {
    /// The identity rotation when `dim` is `3`, `None` otherwise.
    fn identity(dim: usize) -> Option<Self> {
        (dim == 3).then(Self::identity)
    }

    /// The rotation `self * inner`.
    fn compose(&self, inner: &Self) -> Self {
        self * inner
    }

    /// `Some` of the transposed rotation.
    fn inverse(&self) -> Option<Self> {
        Some(Self::inverse(self))
    }

    /// Reports whether `dim` is `3`, every entry of `self`'s 3 × 3 matrix `R`
    /// is finite, every entry of `R · Rᵀ − I` is below [`TYPED_LINK_TOL`] in
    /// absolute value, and `det(R)` is positive.
    fn is_admissible(&self, dim: usize) -> bool {
        if dim != 3 {
            return false;
        }
        let matrix = self.matrix();
        if !matrix.iter().all(|entry| entry.is_finite()) {
            return false;
        }
        let gram = matrix * matrix.transpose();
        (gram - Matrix3::<f64>::identity())
            .iter()
            .all(|d| d.abs() < TYPED_LINK_TOL)
            && matrix.determinant() > 0.0
    }

    /// `tr(R) / 3` for `self`'s 3 × 3 matrix `R`.
    fn wilson(&self) -> f64 {
        self.matrix().trace() / 3.0
    }

    /// Reports whether every entry of `self`'s 3 × 3 matrix minus the
    /// identity is below `eps` in absolute value.
    fn is_flat(&self, eps: f64) -> bool {
        (self.matrix() - Matrix3::<f64>::identity())
            .iter()
            .all(|d| d.abs() < eps)
    }
}

/// Unit quaternions read through the 3 × 3 rotation they represent.
impl LinkVariable for UnitQuaternion<f64> {
    /// The identity quaternion when `dim` is `3`, `None` otherwise.
    fn identity(dim: usize) -> Option<Self> {
        (dim == 3).then(Self::identity)
    }

    /// The unit quaternion `self * inner`.
    fn compose(&self, inner: &Self) -> Self {
        self * inner
    }

    /// `Some` of the conjugate quaternion.
    fn inverse(&self) -> Option<Self> {
        Some(Self::inverse(self))
    }

    /// Reports whether `dim` is `3`, every coordinate of `self` is finite,
    /// and `‖self‖` differs from `1` by less than [`TYPED_LINK_TOL`].
    fn is_admissible(&self, dim: usize) -> bool {
        dim == 3
            && self.coords.iter().all(|entry| entry.is_finite())
            && (self.coords.norm() - 1.0).abs() < TYPED_LINK_TOL
    }

    /// `tr(R) / 3` for `self`'s 3 × 3 rotation matrix `R`.
    fn wilson(&self) -> f64 {
        self.to_rotation_matrix().matrix().trace() / 3.0
    }

    /// Reports whether every entry of `self`'s 3 × 3 rotation matrix minus
    /// the identity is below `eps` in absolute value.
    fn is_flat(&self, eps: f64) -> bool {
        (self.to_rotation_matrix().matrix() - Matrix3::<f64>::identity())
            .iter()
            .all(|d| d.abs() < eps)
    }
}

/// SE(3) rigid motions read through their 4 × 4 homogeneous representation.
impl LinkVariable for Isometry3<f64> {
    /// The identity isometry when `dim` is `4`, `None` otherwise.
    fn identity(dim: usize) -> Option<Self> {
        (dim == 4).then(Self::identity)
    }

    /// The isometry `self * inner`.
    fn compose(&self, inner: &Self) -> Self {
        self * inner
    }

    /// `Some` of `Isometry3::inverse`: the transposed rotation and the negated,
    /// back-rotated translation.
    fn inverse(&self) -> Option<Self> {
        Some(Self::inverse(self))
    }

    /// Reports whether `dim` is `4`, every translation component of `self` is
    /// finite, and `self`'s rotation is admissible as a
    /// [`UnitQuaternion<f64>`](UnitQuaternion) at `3`.
    fn is_admissible(&self, dim: usize) -> bool {
        dim == 4
            && self
                .translation
                .vector
                .iter()
                .all(|entry| entry.is_finite())
            && self.rotation.is_admissible(3)
    }

    /// `tr(H) / 4` for `self`'s 4 × 4 homogeneous matrix `H`, equal to
    /// `(tr(R) + 1) / 4` for `self`'s rotation `R`.
    fn wilson(&self) -> f64 {
        self.to_homogeneous().trace() / 4.0
    }

    /// Reports whether every entry of `self`'s 4 × 4 homogeneous matrix minus
    /// the identity is below `eps` in absolute value.
    fn is_flat(&self, eps: f64) -> bool {
        (self.to_homogeneous() - Matrix4::<f64>::identity())
            .iter()
            .all(|d| d.abs() < eps)
    }
}

/// `D`-dimensional lattice of hypergraph states; links carry [`LinkVariable`]
/// values of defining-representation dimension `link_dim`.
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
///     1,
/// );
///
/// let initial = Hypergraph::from_edges(vec![vec![0, 1, 2]]);
/// lattice.set_state(&[2], initial);
/// lattice.apply_rewrite(&[2], 0);
/// ```
#[derive(Debug, Clone)]
pub struct HypergraphLattice<const D: usize, L: LinkVariable = DMatrix<f64>> {
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

    /// Link variable of each directed link.
    /// Key: (site, `neighbor_site`) pair
    /// Value: the link variable carried by that link
    transitions: HashMap<(Vec<usize>, Vec<usize>), L>,

    /// Defining-representation dimension of every link variable.
    link_dim: usize,

    /// The carrier's identity at `link_dim`, from which every holonomy folds.
    identity: L,

    /// Total number of rewrite steps applied.
    step_count: usize,

    /// Recorded Wilson loops: the site cycle and its Wilson value.
    wilson_loops: Vec<(Vec<Vec<usize>>, f64)>,
}

impl<const D: usize, L: LinkVariable> HypergraphLattice<D, L> {
    /// Creates a `D`-dimensional hypergraph lattice of the given site
    /// `dimensions`, gauge `group` and rewrite `rules`, whose links carry
    /// `L` values of defining-representation dimension `link_dim`.
    ///
    /// # Panics
    ///
    /// Panics when `L::identity(link_dim)` is `None`.
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
    ///     1,
    /// );
    /// assert_eq!(lattice.link_dim(), 1);
    /// ```
    #[must_use]
    pub fn new(
        dimensions: [usize; D],
        group: HypergraphRewriteGroup,
        rules: Vec<RewriteRule>,
        link_dim: usize,
    ) -> Self {
        let identity = L::identity(link_dim).unwrap_or_else(|| {
            panic!("link_dim {link_dim} has no identity in this link-variable carrier")
        });
        Self {
            dimensions,
            group,
            rules,
            next_vertex_id: 0,
            states: HashMap::new(),
            transitions: HashMap::new(),
            link_dim,
            identity,
            step_count: 0,
            wilson_loops: Vec::new(),
        }
    }

    /// Returns the defining-representation dimension of the lattice's link
    /// variables.
    #[inline]
    #[must_use]
    pub const fn link_dim(&self) -> usize {
        self.link_dim
    }

    /// Returns the link variable recorded on the directed link `from` → `to`,
    /// or `None` when that link carries none.
    #[must_use]
    pub fn link(&self, from: &[usize; D], to: &[usize; D]) -> Option<&L> {
        self.transitions.get(&(from.to_vec(), to.to_vec()))
    }

    /// Stores `state` at `site` and returns `true`.
    ///
    /// Returns `false` and stores nothing when any coordinate of `site` is at
    /// or beyond the corresponding lattice dimension.
    pub fn set_state(&mut self, site: &[usize; D], state: Hypergraph) -> bool {
        if !Self::is_valid_site(site, &self.dimensions) {
            return false;
        }
        self.states.insert(site.to_vec(), state);
        true
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

    /// Reports whether `link` is admissible as a link variable of this
    /// lattice: [`LinkVariable::is_admissible`] at this lattice's `link_dim`.
    fn is_admissible(&self, link: &L) -> bool {
        link.is_admissible(self.link_dim)
    }

    /// Records `link` on the directed link `from` → `to` and returns `true`.
    ///
    /// Returns `false` and records nothing when either endpoint has a
    /// coordinate at or beyond the corresponding lattice dimension, or when
    /// `link` is not [`LinkVariable::is_admissible`] at this lattice's
    /// `link_dim`.
    ///
    /// Links recorded here are the ones
    /// [`loop_holonomy`](Self::loop_holonomy) traverses.
    pub fn record_transition(&mut self, from: &[usize; D], to: &[usize; D], link: L) -> bool {
        if !Self::is_valid_site(from, &self.dimensions)
            || !Self::is_valid_site(to, &self.dimensions)
        {
            return false;
        }
        if !self.is_admissible(&link) {
            return false;
        }
        self.transitions.insert((from.to_vec(), to.to_vec()), link);
        true
    }

    /// Ordered product `U_k · … · U_1` of the recorded link variables around
    /// the closed cycle `sites`, or `None` when some link of the cycle carries
    /// no recorded transition.
    ///
    /// An empty cycle yields the carrier's identity at `link_dim`.
    fn cycle_holonomy(&self, sites: &[Vec<usize>]) -> Option<L> {
        let mut holonomy = self.identity.clone();
        for i in 0..sites.len() {
            let key = (sites[i].clone(), sites[(i + 1) % sites.len()].clone());
            let link = self.transitions.get(&key)?;
            holonomy = link.compose(&holonomy);
        }
        Some(holonomy)
    }

    /// Ordered product `U_k · … · U_1` of the link variables around the closed
    /// loop `path`, which wraps from its last site back to its first.
    ///
    /// Returns `None` when any link of the loop carries no recorded
    /// transition; an empty `path` yields the carrier's identity at
    /// `link_dim`. A site may occur more than once in `path` and contributes
    /// one factor per visit; a one-site `path` traverses that site's
    /// self-link. The product of finite link variables can overflow to a
    /// non-finite value.
    #[must_use]
    pub fn loop_holonomy(&self, path: &[&[usize; D]]) -> Option<L> {
        let sites: Vec<Vec<usize>> = path.iter().map(|s| s.to_vec()).collect();
        self.cycle_holonomy(&sites)
    }

    /// Wilson value [`LinkVariable::wilson`] of `path`'s holonomy.
    ///
    /// Returns `None` exactly when
    /// [`loop_holonomy`](Self::loop_holonomy) does.
    #[must_use]
    pub fn wilson_loop(&self, path: &[&[usize; D]]) -> Option<f64> {
        self.loop_holonomy(path).map(|holonomy| holonomy.wilson())
    }

    /// Reports whether `path`'s holonomy is [`LinkVariable::is_flat`] at
    /// `eps`.
    ///
    /// Returns `None` exactly when
    /// [`loop_holonomy`](Self::loop_holonomy) does.
    #[must_use]
    pub fn is_flat(&self, path: &[&[usize; D]], eps: f64) -> Option<bool> {
        self.loop_holonomy(path)
            .map(|holonomy| holonomy.is_flat(eps))
    }

    /// Reports whether `path`'s holonomy is flat at `1e-6`.
    ///
    /// Returns `None` exactly when
    /// [`loop_holonomy`](Self::loop_holonomy) does.
    #[must_use]
    pub fn is_causally_invariant(&self, path: &[&[usize; D]]) -> Option<bool> {
        self.is_flat(path, 1e-6)
    }

    /// Plaquette action [`plaquette_action`] of `path`'s Wilson value.
    ///
    /// Returns `None` exactly when [`wilson_loop`](Self::wilson_loop) does.
    #[must_use]
    pub fn plaquette_action(&self, path: &[&[usize; D]]) -> Option<f64> {
        self.wilson_loop(path).map(super::gauge::plaquette_action)
    }

    /// Replaces every link variable `U` on `x` → `y` with `g_y · U · g_x⁻¹`,
    /// recomputes the recorded Wilson values from their site cycles, and
    /// returns `true`. A site absent from `g` transforms by the identity, and
    /// a key of `g` that is no recorded link's endpoint is unused, whether it
    /// is off the lattice, of a length other than `D`, or an in-bounds site
    /// of length `D` that no recorded link touches.
    ///
    /// Returns `false` and changes nothing when a value of `g` is not
    /// [`LinkVariable::is_admissible`] at this lattice's `link_dim`.
    pub fn gauge_transform(&mut self, g: &HashMap<Vec<usize>, L>) -> bool {
        if !g.values().all(|value| self.is_admissible(value)) {
            return false;
        }

        let identity = self.identity.clone();
        let inverses: HashMap<Vec<usize>, L> = g
            .iter()
            .map(|(site, value)| {
                (
                    site.clone(),
                    value.inverse().expect(
                        "invariant: is_admissible accepted every value of g, so each has an \
                         inverse",
                    ),
                )
            })
            .collect();

        self.transitions = self
            .transitions
            .iter()
            .map(|((from, to), link)| {
                let left = g.get(to).unwrap_or(&identity);
                let right = inverses.get(from).unwrap_or(&identity);
                (
                    (from.clone(), to.clone()),
                    left.compose(&link.compose(right)),
                )
            })
            .collect();

        self.wilson_loops = self
            .wilson_loops
            .iter()
            .map(|(sites, _)| {
                let holonomy = self.cycle_holonomy(sites).expect(
                    "invariant: gauge_transform keeps the link key set, so a recorded cycle \
                     still resolves",
                );
                (sites.clone(), holonomy.wilson())
            })
            .collect();

        true
    }

    /// Replaces the recorded Wilson loops with the elementary plaquettes of
    /// the lattice — the length-4 cycles `s`, `s + e_i`, `s + e_i + e_j`,
    /// `s + e_j` for every site `s` and every axis pair `i < j` that stays in
    /// bounds.
    ///
    /// A plaquette is recorded only when all four of its links carry a
    /// transition recorded by
    /// [`record_transition`](Self::record_transition). Nothing is recorded
    /// when `max_length < 4` or `D < 2`.
    pub fn find_wilson_loops(&mut self, max_length: usize) {
        self.wilson_loops.clear();

        if max_length < 4 || D < 2 {
            return;
        }
        if self.dimensions.contains(&0) {
            return;
        }

        let mut site = [0usize; D];
        loop {
            for i in 0..D {
                for j in (i + 1)..D {
                    if site[i] + 1 >= self.dimensions[i] || site[j] + 1 >= self.dimensions[j] {
                        continue;
                    }

                    let mut corner_i = site;
                    corner_i[i] += 1;
                    let mut corner_ij = site;
                    corner_ij[i] += 1;
                    corner_ij[j] += 1;
                    let mut corner_j = site;
                    corner_j[j] += 1;

                    let sites = vec![
                        site.to_vec(),
                        corner_i.to_vec(),
                        corner_ij.to_vec(),
                        corner_j.to_vec(),
                    ];

                    if let Some(holonomy) = self.cycle_holonomy(&sites) {
                        let wilson = holonomy.wilson();
                        self.wilson_loops.push((sites, wilson));
                    }
                }
            }

            // Lexicographic odometer over the lattice sites, last axis fastest.
            let mut axis = D;
            loop {
                if axis == 0 {
                    return;
                }
                axis -= 1;
                site[axis] += 1;
                if site[axis] < self.dimensions[axis] {
                    break;
                }
                site[axis] = 0;
            }
        }
    }

    /// Returns the recorded Wilson loops: each one's site cycle, and the
    /// Wilson value stored for that cycle by the last
    /// [`find_wilson_loops`](Self::find_wilson_loops) or
    /// [`gauge_transform`](Self::gauge_transform).
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

    /// Mean of the Wilson values stored for the recorded Wilson loops by the
    /// last [`find_wilson_loops`](Self::find_wilson_loops) or
    /// [`gauge_transform`](Self::gauge_transform).
    ///
    /// Returns `None` when no Wilson loops are recorded.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    #[must_use]
    pub fn average_holonomy(&self) -> Option<f64> {
        if self.wilson_loops.is_empty() {
            return None;
        }

        let sum: f64 = self.wilson_loops.iter().map(|(_, w)| w).sum();
        Some(sum / self.wilson_loops.len() as f64)
    }

    /// Reports whether every recorded Wilson loop's holonomy, recomputed from
    /// the current link variables of that loop's site cycle, is flat at
    /// `1e-6`.
    ///
    /// Returns `None` when no Wilson loops are recorded.
    #[must_use]
    pub fn is_globally_causally_invariant(&self) -> Option<bool> {
        if self.wilson_loops.is_empty() {
            return None;
        }
        Some(self.wilson_loops.iter().all(|(sites, _)| {
            self.cycle_holonomy(sites)
                .is_some_and(|holonomy| holonomy.is_flat(1e-6))
        }))
    }

    /// Sum of the plaquette actions of the Wilson values stored for the
    /// recorded Wilson loops by the last
    /// [`find_wilson_loops`](Self::find_wilson_loops) or
    /// [`gauge_transform`](Self::gauge_transform).
    ///
    /// Returns `0.0` when no Wilson loops are recorded.
    #[must_use]
    pub fn total_plaquette_action(&self) -> f64 {
        self.wilson_loops
            .iter()
            .map(|(_, w)| plaquette_action(*w))
            .sum()
    }
}

impl<const D: usize> Default for HypergraphLattice<D, DMatrix<f64>> {
    fn default() -> Self {
        let dims = [1; D];
        Self::new(dims, HypergraphRewriteGroup::new(3), vec![], 1)
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

    /// The 1 × 1 link variable carrying `value`.
    fn link1(value: f64) -> DMatrix<f64> {
        DMatrix::from_element(1, 1, value)
    }

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
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![], 1);

        assert_eq!(lattice.dimensions(), &[5]);
        assert_eq!(lattice.step_count(), 0);
        assert_eq!(lattice.site_count(), 0);
    }

    #[test]
    fn test_lattice_2d_creation() {
        let lattice: HypergraphLattice<2> =
            HypergraphLattice::new([10, 10], HypergraphRewriteGroup::new(4), vec![], 1);

        assert_eq!(lattice.dimensions(), &[10, 10]);
        assert_eq!(lattice.group().num_rules(), 4);
    }

    #[test]
    fn test_lattice_set_get_state() {
        let mut lattice: HypergraphLattice<1> =
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![], 1);

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
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![rule], 1);

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
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![rule], 1);

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
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(2), vec![], 1);

        let site = [1];
        let success = lattice.apply_rewrite(&site, 0); // No rules at all

        assert!(!success);
        assert_eq!(lattice.step_count(), 0);
    }

    #[test]
    fn test_lattice_wilson_loop_empty() {
        let lattice: HypergraphLattice<1> =
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![], 1);

        let path: Vec<&[usize; 1]> = vec![];
        let h = lattice.wilson_loop(&path);

        assert_eq!(h, Some(1.0)); // Empty path is trivial
    }

    #[test]
    fn test_lattice_causal_invariance() {
        let mut lattice: HypergraphLattice<1> =
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![], 1);

        let sites = vec![
            &[0usize] as &[usize; 1],
            &[1usize] as &[usize; 1],
            &[0usize],
        ];

        // With no transitions recorded, the loop has no holonomy at all.
        assert_eq!(lattice.is_causally_invariant(&sites), None);

        assert!(lattice.record_transition(&[0], &[1], link1(2.0)));
        assert!(lattice.record_transition(&[1], &[0], link1(0.5)));
        assert_eq!(lattice.is_causally_invariant(&[&[0], &[1]]), Some(true));
    }

    #[test]
    fn test_lattice_plaquette_action() {
        let mut lattice: HypergraphLattice<1> =
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![], 1);

        let sites = vec![&[0usize] as &[usize; 1], &[1usize]];
        assert_eq!(lattice.plaquette_action(&sites), None);

        assert!(lattice.record_transition(&[0], &[1], link1(1.0)));
        assert!(lattice.record_transition(&[1], &[0], link1(1.0)));

        // Perfect holonomy (1.0) gives zero action
        let action = lattice.plaquette_action(&sites).expect(
            "invariant: both links of the two-site loop were just recorded, so the holonomy exists",
        );
        assert!(action >= 0.0);
        assert!((action - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_lattice_2d_valid_site() {
        let lattice: HypergraphLattice<2> =
            HypergraphLattice::new([5, 5], HypergraphRewriteGroup::new(3), vec![], 1);

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
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![], 1);

        let avg = lattice.average_holonomy();
        assert_eq!(avg, None); // No loops recorded yet
    }

    #[test]
    fn test_lattice_global_causal_invariance() {
        let lattice: HypergraphLattice<1> =
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![], 1);

        // No loops recorded = no verdict
        assert_eq!(lattice.is_globally_causally_invariant(), None);
    }

    #[test]
    fn test_lattice_total_plaquette_action() {
        let lattice: HypergraphLattice<1> =
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![], 1);

        let action = lattice.total_plaquette_action();
        assert_eq!(action, 0.0); // No loops = zero action
    }

    #[test]
    fn test_lattice_rules_accessor() {
        use super::super::rewrite_rule::RewriteRule;

        let rules = vec![RewriteRule::wolfram_a_to_bb(), RewriteRule::edge_split()];
        let lattice: HypergraphLattice<1> =
            HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), rules, 1);

        assert_eq!(lattice.rules().len(), 2);
    }
}
