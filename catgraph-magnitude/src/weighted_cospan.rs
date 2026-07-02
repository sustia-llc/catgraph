//! [`WeightedCospan<Lambda, Q>`] — `catgraph::Cospan<Lambda>` decorated with
//! per-edge weights drawn from a rig `Q`.
//!
//! Phase 6A.1 of the catgraph-magnitude roadmap. The newtype wraps the F&S
//! 2019 cospan with a sparse [`HashMap<(NodeId, NodeId), Q>`] of weights, one
//! per implied edge. The "implied edges" of a cospan are the bipartite
//! product of left-leg targets and right-leg targets via the apex (middle)
//! set: every `(left_target, right_target)` pair receives an edge.
//!
//! When `Q = UnitInterval`, [`WeightedCospan::into_metric_space`] lifts the
//! weighted cospan into a [`LawvereMetricSpace<NodeId>`] via the `-ln π`
//! embedding (Lawvere 1973; BTV 2021 §1.4). The general `Q` case is deferred
//! to v0.2.0 — magnitude over arbitrary rigs needs a base-change choice that
//! is not unique.
//!
//! ## Type aliases
//!
//! - [`ProbCospan<Lambda>`] = `WeightedCospan<Lambda, UnitInterval>` —
//!   probability-weighted; the BV 2025 §3 LM transition-weight setting.
//! - [`TropCospan<Lambda>`] = `WeightedCospan<Lambda, Tropical>` —
//!   distance-weighted; the v0.2.0 tropical-magnitude path.

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use catgraph::cospan::Cospan;
use catgraph::errors::CatgraphError;
use deep_causality_num::Zero;

use crate::{LawvereMetricSpace, Rig, Tropical, UnitInterval};

/// Node identifier inside a [`WeightedCospan`].
///
/// One-to-one with the apex (middle) index of the underlying [`Cospan`]:
/// `NodeId(i)` refers to the i-th element of [`Cospan::middle`]. The
/// [`Cospan::left_to_middle`] and [`Cospan::right_to_middle`] slices are
/// already `&[usize]` indexing into that middle, so a `NodeId = usize` alias
/// keeps the bridge transparent.
///
/// If a follow-up needs a side-aware coordinate (e.g. a `(Side, usize)`
/// enum to distinguish "node reached via left leg" vs. "node reached via
/// right leg"), we'll introduce it in v0.1.1 — v0.1.0 stays minimal.
pub type NodeId = usize;

/// A [`Cospan<Lambda>`] decorated with per-edge weights in a rig `Q`.
///
/// **Edge convention.** The implied edges of a cospan `(L → M ← R)` are the
/// bipartite product of the left-leg images and right-leg images inside the
/// apex `M`. That is, every pair `(i, j)` with `i ∈ left_to_middle()` and
/// `j ∈ right_to_middle()` is an edge. This is the F&S 2019 §1 reading of a
/// cospan as the bipartite hypergraph between its source and target sets.
///
/// **Sparse storage.** Weights live in a [`HashMap<(NodeId, NodeId), Q>`].
/// Absent entries return `Q::zero()` from [`weight`](Self::weight) — the rig
/// "no edge" convention (additive identity).
#[derive(Clone, Debug)]
pub struct WeightedCospan<Lambda, Q>
where
    Lambda: Sized + Eq + Copy + Debug,
    Q: Rig,
{
    cospan: Cospan<Lambda>,
    weights: HashMap<(NodeId, NodeId), Q>,
}

impl<Lambda, Q> WeightedCospan<Lambda, Q>
where
    Lambda: Sized + Eq + Copy + Debug,
    Q: Rig,
{
    /// Build a weighted cospan whose every implied edge carries the same
    /// `weight`.
    ///
    /// The implied edges are the bipartite product
    /// `left_to_middle() × right_to_middle()`. Duplicate `(i, j)` pairs (which
    /// can arise when two left ports map to the same middle index) collapse
    /// to a single entry in the weight map; the weight is identical, so the
    /// collapse is information-preserving.
    // Takes `weight: Q` by value to match the call-site ergonomics of
    // passing `Q::one()` / `Q::zero()` directly without an extra `&`.
    #[allow(clippy::needless_pass_by_value)]
    pub fn from_cospan_uniform(cospan: Cospan<Lambda>, weight: Q) -> Self {
        let mut weights: HashMap<(NodeId, NodeId), Q> = HashMap::new();
        for &i in cospan.left_to_middle() {
            for &j in cospan.right_to_middle() {
                weights.insert((i, j), weight.clone());
            }
        }
        Self { cospan, weights }
    }

    /// Build a weighted cospan whose implied edges are weighted by a
    /// caller-supplied function.
    ///
    /// `weight_fn(i, j)` is invoked once per implied edge `(i, j) ∈
    /// left_to_middle() × right_to_middle()`. Order of invocation follows the
    /// `Vec<MiddleIndex>` traversal of the legs and is therefore
    /// deterministic, but callers should not depend on it (it is a `HashMap`
    /// insertion sequence).
    pub fn from_cospan_with_weights<F>(cospan: Cospan<Lambda>, weight_fn: F) -> Self
    where
        F: Fn(NodeId, NodeId) -> Q,
    {
        let mut weights: HashMap<(NodeId, NodeId), Q> = HashMap::new();
        for &i in cospan.left_to_middle() {
            for &j in cospan.right_to_middle() {
                weights.insert((i, j), weight_fn(i, j));
            }
        }
        Self { cospan, weights }
    }

    /// Weight of the edge from node `i` to node `j`.
    ///
    /// Returns `Q::zero()` when no edge has been recorded — the rig
    /// "no edge" / additive-identity convention. This is consistent with
    /// [`LawvereMetricSpace::distance`] returning `Tropical::zero() =
    /// Tropical(+∞)` for unset distances, since the `-ln π` embedding maps
    /// `UnitInterval::zero() = 0.0` (probability of impossible) to
    /// `Tropical(+∞)` (infinite distance).
    #[must_use]
    pub fn weight(&self, i: NodeId, j: NodeId) -> Q {
        self.weights.get(&(i, j)).cloned().unwrap_or_else(Q::zero)
    }

    /// Set the weight of the edge `(i, j)`. Overwrites any prior value.
    ///
    /// Does not validate that `(i, j)` is one of the cospan's "implied
    /// edges" — callers are free to record weights for non-implied pairs,
    /// e.g. self-loops `(i, i)` needed for the BV 2025 LM identity-axiom
    /// requirement.
    pub fn set_weight(&mut self, i: NodeId, j: NodeId, w: Q) {
        self.weights.insert((i, j), w);
    }

    /// Borrow the underlying [`Cospan<Lambda>`] without copying.
    #[must_use]
    pub fn as_cospan(&self) -> &Cospan<Lambda> {
        &self.cospan
    }
}

impl<Lambda> WeightedCospan<Lambda, UnitInterval>
where
    Lambda: Sized + Eq + Copy + Debug,
{
    /// Lift the weighted cospan into a [`LawvereMetricSpace<NodeId>`] via the
    /// `-ln π` embedding (Lawvere 1973).
    ///
    /// **Object set.** The resulting metric space has one object per apex
    /// (middle) index — i.e. `NodeId(0), …, NodeId(m-1)` where `m =
    /// cospan.middle().len()`. Boundary (left/right) ports are not directly
    /// represented; the cospan's leg maps embed them into the apex.
    ///
    /// **Distance.** For `(a, b) ∈ NodeId²` the probability `prob(a, b)` is
    /// the recorded weight (or `UnitInterval::zero() = 0.0` if absent). The
    /// embedding then computes `d(a, b) = -ln(prob(a, b))`, with `d(a, b) =
    /// +∞` when `prob = 0`. This is exactly the
    /// [`BaseChange<UnitInterval> for Tropical`](catgraph_applied::rig::BaseChange)
    /// recipe; no re-derivation here.
    ///
    /// **Identity axiom.** Lawvere metric spaces require `d(x, x) = 0`
    /// (i.e. `prob(x, x) = 1`). This method does not enforce that — callers
    /// who need the identity axiom must insert `set_weight(i, i,
    /// UnitInterval::new(1.0).unwrap())` for every `i` before invoking
    /// [`into_metric_space`](Self::into_metric_space). See
    /// [`LawvereMetricSpace::from_unit_interval`] documentation for the
    /// full caller obligation list.
    #[must_use]
    pub fn into_metric_space(self) -> LawvereMetricSpace<NodeId> {
        let m = self.cospan.middle().len();
        let objects: Vec<NodeId> = (0..m).collect();
        LawvereMetricSpace::from_unit_interval(objects, |a: &NodeId, b: &NodeId| {
            self.weights
                .get(&(*a, *b))
                .copied()
                .unwrap_or_else(UnitInterval::zero)
        })
    }

    /// Lift the weighted cospan into a [`LawvereMetricSpace<NodeId>`] AND
    /// validate the Lawvere metric axioms (v0.1.1).
    ///
    /// Performs the same `-ln π` embedding as
    /// [`into_metric_space`](Self::into_metric_space), then runs the
    /// triangle-inequality scan
    /// (`d(x, z) ≤ d(x, y) + d(y, z)` for all triples) before returning.
    /// Returns an `Err` if any triple violates the inequality, surfacing
    /// malformed weight matrices at construction time rather than letting
    /// them propagate into [`magnitude`](crate::magnitude::magnitude).
    ///
    /// **Tree-additivity fast path.** The triangle-inequality check is the
    /// upper bound `d(x, z) ≤ d(x, y) + d(y, z)`. For "tree-shaped" LMs in
    /// the BV 2025 §2.15 prefix-extension setting, the equality
    /// `d(x, z) = d(x, y) + d(y, z)` holds along the unique forward path —
    /// stronger than the inequality. This implementation does the upper
    /// bound only; v0.2.0+ may add an opt-in tree-additivity equality check
    /// (a TODO tracks the perf opportunity once Phase 6C surfaces a
    /// concrete profile).
    ///
    /// **Identity axiom.** Same caveat as
    /// [`into_metric_space`](Self::into_metric_space) — callers must seed
    /// `(i, i)` self-weights to `1.0` to satisfy `d(x, x) = 0`. The diagonal
    /// is NOT validated here; the v0.5.4 [`hom`](catgraph_applied::lawvere_metric::LawvereMetricSpace)
    /// diagonal default does not affect the underlying `distance` table
    /// that this scan reads.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] when triangle inequality is
    /// violated for some triple `(x, y, z)` by more than a small absolute
    /// float tolerance (`TRIANGLE_FLOAT_TOL`, `1e-9` in the distance/log
    /// domain). The tolerance absorbs the ULP-scale noise from the
    /// `−ln`-of-product vs sum-of-`−ln` rewrite; genuine violations (orders of
    /// magnitude above the tolerance) still surface as `Err`.
    pub fn into_validated_metric_space(self) -> Result<LawvereMetricSpace<NodeId>, CatgraphError> {
        let space = self.into_metric_space();
        if space.triangle_inequality_holds_within(crate::TRIANGLE_FLOAT_TOL) {
            Ok(space)
        } else {
            Err(CatgraphError::Composition {
                message: "WeightedCospan::into_validated_metric_space: triangle inequality \
                          d(x, z) ≤ d(x, y) + d(y, z) violated for some triple"
                    .to_owned(),
            })
        }
    }
}

/// Probability-weighted cospan: edges carry [`UnitInterval`] weights.
///
/// The Phase 6A.3 `LmCategory` realization of BV 2025 §3 language-model
/// transitions stores its weights in a `ProbCospan<NodeId>` (or a closely
/// related materialized table).
pub type ProbCospan<Lambda> = WeightedCospan<Lambda, UnitInterval>;

/// Distance-weighted cospan: edges carry [`Tropical`] weights directly.
///
/// Used in the v0.2.0 tropical-magnitude path, where Möbius inversion is
/// performed in the (min, +) semiring rather than via a base-change from
/// `UnitInterval`.
pub type TropCospan<Lambda> = WeightedCospan<Lambda, Tropical>;

// `NodeId = usize` is `Copy + Eq + Hash`, which the `LawvereMetricSpace<T>`
// type parameter requires. Sanity check at compile time:
const _: fn() = || {
    fn assert_node_id_bounds<T: Clone + Eq + Hash>() {}
    assert_node_id_bounds::<NodeId>();
};
