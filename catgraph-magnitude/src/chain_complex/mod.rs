//! Magnitude-homology chain complex over a Lawvere metric space.
//!
//! Per Leinster-Shulman 2017 §2, this module materialises:
//! - [`Chain`]: simple-chain newtype `(a_0, ..., a_k)` with `a_{j-1} ≠ a_j`.
//! - [`enumerate_chains`]: DFS up to caller-supplied length cutoff.
//! - [`ChainIndex`]: `(k, ℓ)`-bucketed index per LS 2017 §2 grading.
//! - [`boundary_matrix<Q>`]: alternating-sum drop-one-vertex face map.
//!
//! ## Pseudo-metric widening
//!
//! [`Chain::is_finite_in`] accepts pseudo-metric spaces (LS 2017
//! Ex 2.9) — distinct points may have zero distance. There is no
//! `d > 0.0` clause; the widening is monotone for strict
//! metrics (the acceptance fixtures continue to pass).
//!
//! ## Module split
//!
//! Rank-recovery + acceptance gate live in the sibling
//! [`homology`] submodule. Public API surface is preserved via the
//! re-exports below; external callers continue to import through
//! `chain_complex::{magnitude_homology_rank, euler_char_identity_at}`.

pub mod homology;
pub use homology::{IntegerLikeRig, euler_char_identity_at, magnitude_homology_rank};

use std::collections::BTreeMap;

use catgraph::errors::CatgraphError;
use catgraph_applied::lawvere_metric::LawvereMetricSpace;
use catgraph_applied::mat::MatR;
use catgraph_applied::rig::Rig;

use crate::weighted_cospan::NodeId;

/// A simple chain `(x₀, x₁, …, x_k)` in a Lawvere metric space.
///
/// Per Leinster–Shulman 2017 §2, a `k`-chain is a `(k+1)`-tuple of points.
/// Simplicity (consecutive entries distinct) is enforced at construction.
/// Finite-distance and simplicity (`+∞` rejected; distinct consecutive points
/// required) is checked by [`Chain::is_finite_in`]. Pseudo-metric `d == 0`
/// between distinct points is accepted (LS 2017 Def 3.3).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Chain {
    points: Vec<NodeId>,
}

impl Chain {
    /// Build a chain from a sequence of points. Consecutive duplicates are
    /// allowed at construction (degenerate chains may arise during boundary
    /// computation); [`Chain::is_finite_in`] flags them.
    #[must_use]
    pub fn new(points: Vec<NodeId>) -> Self {
        Self { points }
    }

    /// Homological degree `k`: one less than the number of points. A 0-chain
    /// `(x₀)` has degree 0 by Leinster–Shulman 2017 convention.
    #[must_use]
    pub fn degree(&self) -> usize {
        self.points.len().saturating_sub(1)
    }

    /// The underlying point sequence.
    #[must_use]
    pub fn points(&self) -> &[NodeId] {
        &self.points
    }

    /// Length grading `ℓ = Σ_{j=1}^{k} d(x_{j-1}, x_j)`. Returns `f64::INFINITY`
    /// if any consecutive pair has infinite distance.
    ///
    /// # Note
    ///
    /// `length` may return `f64::INFINITY` for unreachable chains; callers
    /// that want only finite-length chains should gate on
    /// [`Self::is_finite_in`] first.
    #[must_use]
    pub fn length(&self, space: &LawvereMetricSpace<NodeId>) -> f64 {
        let mut acc = 0.0;
        for win in self.points.windows(2) {
            let d = space.distance(&win[0], &win[1]).0;
            if !d.is_finite() {
                return f64::INFINITY;
            }
            acc += d;
        }
        acc
    }

    /// Returns true iff the chain has finite length under the Lawvere metric
    /// (no `+∞` edge) AND consists of pairwise-distinct consecutive points
    /// (LS 2017 Def 3.3 simplicity).
    ///
    /// **Pseudo-metric widening.** There is no `d > 0.0` requirement between
    /// consecutive points; pseudo-metric `[0, ∞]`-categories (LS 2017
    /// Example 2.9), where distinct objects may have zero distance before
    /// skeletal collapse, enumerate correctly. The widening is **monotone for
    /// strict metrics** (LS 2017 Example 2.7): on a strict metric `d = 0` only
    /// when points coincide, which is already rejected by the `win[0] != win[1]`
    /// clause, so the acceptance fixtures continue to pass verbatim.
    #[must_use]
    pub fn is_finite_in(&self, space: &LawvereMetricSpace<NodeId>) -> bool {
        for win in self.points.windows(2) {
            if win[0] == win[1] {
                return false;
            }
            let d = space.distance(&win[0], &win[1]).0;
            if !d.is_finite() {
                return false;
            }
        }
        true
    }
}

/// Enumerate all simple chains of degree ≤ `max_degree` in a Lawvere metric
/// space. Output includes degree-0 chains (single points) up through
/// degree-`max_degree` chains. Filters out chains containing any consecutive
/// pair with infinite distance (pseudo-metric `d == 0` between
/// distinct points is accepted; see [`Chain::is_finite_in`]).
///
/// Complexity: `O(n^(max_degree + 1))` worst case for an `n`-element space.
/// Practical for `n ≤ 20` and `max_degree ≤ 5`, the typical regime for
/// magnitude-homology applications.
///
/// # Panics
///
/// Will not panic in practice: the DFS stack is seeded with non-empty
/// 0-chains and only extended with non-empty chains, so the
/// `.expect("non-empty by construction")` invariant holds for every
/// popped prefix.
#[must_use]
pub fn enumerate_chains(space: &LawvereMetricSpace<NodeId>, max_degree: usize) -> Vec<Chain> {
    let n = space.size();
    let mut out = Vec::new();
    // Degree 0 (points).
    for i in 0..n {
        out.push(Chain::new(vec![i]));
    }
    // Degree 1..=max_degree via DFS.
    let mut stack: Vec<Chain> = (0..n).map(|i| Chain::new(vec![i])).collect();
    while let Some(prefix) = stack.pop() {
        if prefix.degree() >= max_degree {
            continue;
        }
        let last = *prefix.points().last().expect("non-empty by construction");
        for j in 0..n {
            let next = j;
            if next == last {
                continue; // simplicity
            }
            let d = space.distance(&last, &next).0;
            // finite-distance restriction (pseudo-metric d == 0
            // between distinct points accepted)
            if !d.is_finite() {
                continue;
            }
            let mut extended = prefix.points().to_vec();
            extended.push(next);
            let chain = Chain::new(extended);
            out.push(chain.clone());
            stack.push(chain);
        }
    }
    out
}

/// Length-bucketed index of all simple chains in a Lawvere metric space.
///
/// Buckets are keyed by `(degree, length_bucket_id)` with bucket IDs derived
/// from `f64` length values via tolerance-aware rounding. Default tolerance:
/// `1e-9 * min_off_diagonal_distance` (or `1e-12` if min off-diagonal is 0).
pub struct ChainIndex {
    /// `BTreeMap<(degree, length_bucket_id), Vec<Chain>>`.
    /// `length_bucket_id` is `(length / tolerance).round() as i64`.
    buckets: BTreeMap<(usize, i64), Vec<Chain>>,
    tolerance: f64,
    /// Set of distinct `ℓ` values that actually appear (sorted ascending).
    grades: Vec<f64>,
}

impl ChainIndex {
    /// Build the chain index for all simple chains of degree ≤ `max_degree`.
    #[must_use]
    pub fn new(space: &LawvereMetricSpace<NodeId>, max_degree: usize) -> Self {
        let chains = enumerate_chains(space, max_degree);
        let tolerance = Self::default_tolerance(space);
        let mut buckets: BTreeMap<(usize, i64), Vec<Chain>> = BTreeMap::new();
        let mut grade_set: std::collections::BTreeSet<i64> = std::collections::BTreeSet::new();
        for c in chains {
            let ell = c.length(space);
            if !ell.is_finite() {
                continue;
            }
            // Bucket-id rounding: for any practical metric space the bucket
            // id is bounded by `(max_length / min_off_diag) * 1e9`, well
            // within i64 range. Truncation/sign-loss casts are sound.
            #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
            let bucket = (ell / tolerance).round() as i64;
            grade_set.insert(bucket);
            buckets.entry((c.degree(), bucket)).or_default().push(c);
        }
        #[allow(clippy::cast_precision_loss)]
        let grades = grade_set
            .into_iter()
            .map(|b| b as f64 * tolerance)
            .collect();
        Self {
            buckets,
            tolerance,
            grades,
        }
    }

    fn default_tolerance(space: &LawvereMetricSpace<NodeId>) -> f64 {
        let n = space.size();
        let mut min_off = f64::INFINITY;
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }
                let d = space.distance(&i, &j).0;
                if d.is_finite() && d > 0.0 && d < min_off {
                    min_off = d;
                }
            }
        }
        if min_off.is_finite() && min_off > 0.0 {
            min_off * 1e-9
        } else {
            1e-12
        }
    }

    /// Chains at homological degree `k` and length grade `ell` (within tolerance).
    #[must_use]
    pub fn chains_at(&self, k: usize, ell: f64) -> &[Chain] {
        // Same truncation-bound argument as `new`.
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        let bucket = (ell / self.tolerance).round() as i64;
        self.buckets
            .get(&(k, bucket))
            .map_or(&[][..], |v| v.as_slice())
    }

    /// All distinct `ℓ` values appearing in the index, ascending.
    ///
    /// Each value is reconstructed from a bucket id via `id as f64 * tolerance`,
    /// so callers see the bucketised representative rather than the literal
    /// `Chain::length` of any one chain at that grade. Round-trip invariant:
    /// `chains_at(k, grades()[i])` always returns the chains for bucket `i`,
    /// because `chains_at` re-bucketises via `(ell / tolerance).round()` —
    /// which is the inverse of the reconstruction. ULP error on the round-trip
    /// is at most one tolerance step, immaterial for the `e^(−ℓ)` weighting in
    /// [`euler_char_identity_at`] at the shipped fixture sizes
    /// (n ≤ 5, distances `[1.0, 4.0]`).
    #[must_use]
    pub fn grades(&self) -> &[f64] {
        &self.grades
    }

    /// Tolerance used for bucket-id rounding.
    #[must_use]
    pub fn tolerance(&self) -> f64 {
        self.tolerance
    }
}

/// Build the boundary matrix `∂_k: C_{k,ℓ} → C_{k-1,ℓ}` per LS 2017 Def 2.5,
/// restricted to the length grade `ell`.
///
/// Rows are indexed by `(k-1)`-chains at grade `ell`; columns by `k`-chains
/// at grade `ell`. Entries are signed integers cast into `Q` via `Q::from(i64)`
/// for `±1`. The geodesic condition `d(x_{i-1}, x_{i+1}) = d(x_{i-1}, x_i) +
/// d(x_i, x_{i+1})` is checked within `2 * tolerance` of the index. Per LS 2017
/// Def 2.5, `i` ranges over interior indices `1..=k-1` only — the endpoints
/// `x_0` and `x_k` are never omitted (only interior omissions can preserve the
/// length grade).
///
/// # Errors
///
/// `Err(CatgraphError::Composition)` if a boundary chain falls outside the
/// length-`ell` bucket (numerical instability).
pub fn boundary_matrix<Q>(
    idx: &ChainIndex,
    space: &LawvereMetricSpace<NodeId>,
    k: usize,
    ell: f64,
) -> Result<MatR<Q>, CatgraphError>
where
    Q: Rig + From<i64>,
{
    let cols = idx.chains_at(k, ell);
    let rows = if k > 0 {
        idx.chains_at(k - 1, ell)
    } else {
        &[][..]
    };

    // Column index lookup for ∂-chain → column id.
    let mut row_lookup = std::collections::HashMap::<&Chain, usize>::new();
    for (i, c) in rows.iter().enumerate() {
        row_lookup.insert(c, i);
    }

    let mut entries: Vec<Vec<Q>> = vec![vec![Q::zero(); cols.len()]; rows.len()];

    for (col_idx, chain) in cols.iter().enumerate() {
        let pts = chain.points();
        if pts.len() < 2 {
            continue; // ∂ of a 0-chain is empty
        }
        // i ranges over interior indices 1..=k-1 per LS 2017 Def 2.5.
        for i in 1..pts.len().saturating_sub(1) {
            let d_left = space.distance(&pts[i - 1], &pts[i]).0;
            let d_right = space.distance(&pts[i], &pts[i + 1]).0;
            let d_skip = space.distance(&pts[i - 1], &pts[i + 1]).0;
            // Geodesic condition with index tolerance.
            if (d_skip - (d_left + d_right)).abs() > 2.0 * idx.tolerance() {
                continue;
            }
            // Build the omitted chain.
            let mut omitted = pts.to_vec();
            omitted.remove(i);
            let omitted_chain = Chain::new(omitted);
            let row_idx = row_lookup.get(&omitted_chain).copied().ok_or_else(|| {
                CatgraphError::Composition {
                    message: format!(
                        "boundary chain not in (k-1)-grade-{ell} bucket — likely tolerance issue"
                    ),
                }
            })?;
            // Sign (-1)^i; convert to Q via i64.
            let sign: i64 = if i % 2 == 0 { 1 } else { -1 };
            let signed_one: Q = Q::from(sign);
            entries[row_idx][col_idx] = entries[row_idx][col_idx].clone() + signed_one;
        }
    }
    MatR::<Q>::new(rows.len(), cols.len(), entries)
}
