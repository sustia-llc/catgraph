//! [`Hypergraph`] — a CRUD hypergraph container (K1 backend, sustia-llc/koalisi#4).
//!
//! This is the zero-dependency replacement for the yamafaktory `hypergraph`
//! crate (v4.2.0) that the downstream **koalisi** coalition layer re-backs its
//! `TemporalHypergraph` on. catgraph's *hypergraph categories*
//! (`Cospan`/`NamedCospan`/`HypergraphCategory`) are the categorical structure,
//! **not** an n-ary hyperedge data structure — no such container existed in core
//! or applied. This module supplies the operations koalisi's topology layer
//! calls (a full call-site survey of `koalisi/src/topology/temporal.rs`), with
//! signatures adapted where the K1 re-back can improve on yamafaktory — e.g.
//! [`clear_hyperedges`](Hypergraph::clear_hyperedges) is **infallible** here
//! (`()`), so koalisi's `clear_hyperedges()?` call site simply drops the `?`.
//! It also offers a categorical *view* back into catgraph's [`Cospan`]
//! ([`hyperedge_as_cospan`](Hypergraph::hyperedge_as_cospan)).
//!
//! It uses plain `Vec`/`HashMap` and monotonic counters — **zero new
//! dependencies** — at coalition scale (tens–hundreds of vertices), correctness
//! over micro-performance.
//!
//! # Three load-bearing semantics
//!
//! These three properties are relied upon by koalisi's event-sourced replay and
//! are therefore contractual, not incidental:
//!
//! 1. **Stable, never-reused indices.** [`VertexIndex`] / [`HyperedgeIndex`] are
//!    handed out by monotonic counters and are **never reused** after a removal —
//!    not even across [`Hypergraph::clear`]. koalisi's event log stores raw
//!    indices and replays them; reuse would alias a replayed index onto a
//!    different entity. Removing vertex `b` from `{a, b, c}` and then adding `d`
//!    gives `d` a *fresh* index; `a` and `c` keep theirs; `b`'s index errors
//!    forever after.
//! 2. **Hyperedges are ORDERED `Vec<VertexIndex>` with duplicates allowed.** A
//!    hyperedge is an ordered member list; the same vertex may appear more than
//!    once, and order is preserved through every read, [`reverse`](Hypergraph::reverse_hyperedge),
//!    and [`join`](Hypergraph::join_hyperedges). (Duplicates are legal *here* but
//!    the magnitude layer rejects them — see the consumer-path note below.)
//! 3. **`Copy` weights returned by value.** Vertex/hyperedge weights are `Copy`
//!    and read out by value (never by reference), matching koalisi's atomic
//!    read-modify-write pattern under a single lock.
//!
//! # Divergences from yamafaktory `hypergraph` v4.2.0 (all deliberate)
//!
//! This is the canonical enumeration; per-item docs point back here.
//!
//! - **No-op updates return `Ok`, not `Err`.**
//!   [`update_vertex_weight`](Hypergraph::update_vertex_weight),
//!   [`update_hyperedge_weight`](Hypergraph::update_hyperedge_weight), and
//!   [`update_hyperedge_vertices`](Hypergraph::update_hyperedge_vertices) accept
//!   an unchanged value/list and return `Ok(())`. yamafaktory errors on an
//!   unchanged update (`…Unchanged`). This is the divergence that fixes
//!   koalisi's idempotency wart: `CoalitionManager::try_join_coalition`'s
//!   docstring promises re-join "is idempotent if `agent` is already a member",
//!   but on yamafaktory a re-join of an already-present agent left the member
//!   list unchanged and errored. Making the no-op succeed makes the documented
//!   behavior true.
//! - **Infallible clears.** [`clear_hyperedges`](Hypergraph::clear_hyperedges)
//!   and [`clear`](Hypergraph::clear) return `()`, not `Result` (nothing can
//!   fail); koalisi's `?` call sites drop the operator.
//! - **Relaxed generic bounds.** `V, HE: Copy + Eq + Debug` only — no `Display`,
//!   no `Into<usize>`, no `Hash` requirement (yamafaktory required more). Weights
//!   are keyed by internal `usize`, never hashed, so `Hash` is not needed.
//! - **No serde.** Serialization is not an applied concern (and would be a new
//!   dependency). koalisi wraps [`VertexIndex`] / [`HyperedgeIndex`] in its own
//!   serde newtypes for the event log.
//!
//! # The real consumer path (K1 → K2)
//!
//! The magnitude layer does **not** consume a cospan. `Coalition` has no cospan
//! constructor (and deliberately gains none — plain constructors only, per the
//! #23 tripwire). The actual path from this container to a diversity scalar is:
//!
//! 1. [`get_hyperedge_vertices`](Hypergraph::get_hyperedge_vertices) — read a
//!    coalition's ordered member indices.
//! 2. koalisi maps those members' capabilities to pairwise couplings
//!    `(from, to, prob)`.
//! 3. `catgraph_magnitude::coalition_value(agents, couplings, members)` returns
//!    the pinned-`t = 1` diversity scalar.
//!
//! **Dedup before step 3.** Hyperedges legally carry duplicate members
//! (semantic #2), but `coalition_value` / `Coalition::from_enriched` **error** on
//! a duplicate member (a repeated agent would seed two ∞-separated nodes). A
//! caller feeding a hyperedge's member list to the magnitude layer must
//! deduplicate it first.
//!
//! # The categorical view
//!
//! [`hyperedge_as_cospan`](Hypergraph::hyperedge_as_cospan) reads a hyperedge as
//! the **identity cospan over its member index list** — middle = the ordered
//! member [`VertexIndex`] list (duplicates preserved), both legs the identity
//! `0..k`. This is *not* a "discrete" graph: under the `WeightedCospan`
//! implied-edge reading (see `catgraph-magnitude`'s `weighted_cospan.rs`), an
//! identity cospan's edges are the bipartite product of its leg targets — i.e.
//! **all `(i, j)` member pairs**, precisely the coupling slots the magnitude
//! layer would fill. It carries the same middle currency (member *identities*)
//! that `Coalition::from_enriched` builds its internal cospan from — so it is the
//! natural handle for cospan-level composition **within applied**, not a shortcut
//! into the magnitude layer (which takes the plain-data path above).

use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug, Display};

use catgraph::category::HasIdentity;
use catgraph::cospan::Cospan;

/// Stable, never-reused index of a vertex in a [`Hypergraph`].
///
/// Handed out by a monotonic counter; see the module-level "stable indices"
/// contract. Prefer [`VertexIndex::get`] / `From`/`Into` at API boundaries.
///
/// The inner `usize` is `pub` and [`From<usize>`] exists **only** so a consumer
/// can rehydrate an index it previously observed (koalisi replays raw indices
/// from its event log). Indices are semantically **opaque, never-reused
/// capabilities** — forging an arbitrary value, or reusing a removed one, breaks
/// the replay contract. Construct fresh indices only via
/// [`Hypergraph::add_vertex`].
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub struct VertexIndex(pub usize);

/// Stable, never-reused index of a hyperedge in a [`Hypergraph`].
///
/// Handed out by a monotonic counter; see the module-level "stable indices"
/// contract.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub struct HyperedgeIndex(pub usize);

impl VertexIndex {
    /// The underlying `usize` index.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

impl HyperedgeIndex {
    /// The underlying `usize` index.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

impl Display for VertexIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

impl Display for HyperedgeIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "e{}", self.0)
    }
}

impl From<usize> for VertexIndex {
    fn from(u: usize) -> Self {
        Self(u)
    }
}

impl From<usize> for HyperedgeIndex {
    fn from(u: usize) -> Self {
        Self(u)
    }
}

impl From<VertexIndex> for usize {
    fn from(v: VertexIndex) -> Self {
        v.0
    }
}

impl From<HyperedgeIndex> for usize {
    fn from(e: HyperedgeIndex) -> Self {
        e.0
    }
}

/// Structured error for [`Hypergraph`] operations.
///
/// Non-generic (unlike yamafaktory's error, which is parameterized by the weight
/// types) — each variant carries the offending index or a reason so the caller
/// can diagnose the failure without re-inspecting the graph. Derived with
/// `thiserror`, matching `catgraph`'s `CatgraphError` pattern (#28); the
/// `Display` strings are unchanged from the original hand-rolled impl.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HypergraphError {
    /// No vertex with this index exists (removed, or never added).
    #[error("hypergraph error: vertex {0} not found")]
    VertexNotFound(VertexIndex),
    /// No hyperedge with this index exists (removed, or never added).
    #[error("hypergraph error: hyperedge {0} not found")]
    HyperedgeNotFound(HyperedgeIndex),
    /// A hyperedge was requested with an empty vertex list (not allowed — a
    /// hyperedge must connect at least one vertex).
    #[error("hypergraph error: a hyperedge must have at least one vertex")]
    HyperedgeNoVertices,
    /// [`join_hyperedges`](Hypergraph::join_hyperedges) needs at least two
    /// distinct, existing hyperedges (fewer than two, or a repeated index).
    #[error("hypergraph error: join requires at least two distinct, existing hyperedges")]
    InvalidJoin,
    /// [`contract_hyperedge_vertices`](Hypergraph::contract_hyperedge_vertices)
    /// was given a target or a to-contract vertex that is not currently a member
    /// of the hyperedge's list.
    #[error("hypergraph error: invalid contraction: {reason}")]
    InvalidContraction {
        /// Human-readable reason (which vertex, which hyperedge).
        reason: String,
    },
    /// One or more referenced vertices do not exist (add/update-hyperedge with an
    /// unknown vertex). Carries the offending indices.
    #[error("hypergraph error: vertices not found: [{}]", join_indices(.0))]
    VerticesNotFound(Vec<VertexIndex>),
}

/// Comma-join for [`HypergraphError::VerticesNotFound`]'s `Display` — keeps the
/// derived message byte-identical to the original hand-rolled `[v0, v1, …]` form.
fn join_indices(vs: &[VertexIndex]) -> String {
    vs.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

/// A CRUD hypergraph over `Copy` vertex weights `V` and hyperedge weights `HE`.
///
/// See the module docs for the three load-bearing semantics (stable/never-reused
/// indices, ordered duplicate-allowing hyperedge lists, `Copy` weights by value)
/// and the deliberate divergences from yamafaktory `hypergraph` v4.2.0.
///
/// # Bounds
///
/// `V, HE: Copy + Eq + Debug`. This is **relaxed** relative to yamafaktory
/// (which additionally required `Display`/`Into<usize>`/`Hash`): weights are
/// stored in a `HashMap` keyed by the internal `usize`, never by the weight
/// itself, so no `Hash` bound is needed. `Eq` powers hyperedge idempotency and
/// nothing else; `Copy` lets reads return by value.
#[derive(Clone, Debug)]
pub struct Hypergraph<V, HE>
where
    V: Copy + Eq + Debug,
    HE: Copy + Eq + Debug,
{
    /// Vertices keyed by stable index → weight.
    vertices: HashMap<usize, V>,
    /// Hyperedges keyed by stable index → (ordered member list, weight).
    hyperedges: HashMap<usize, (Vec<VertexIndex>, HE)>,
    /// Monotonic vertex-index counter; only ever increases (never reset).
    next_vertex: usize,
    /// Monotonic hyperedge-index counter; only ever increases (never reset).
    next_hyperedge: usize,
}

impl<V, HE> Default for Hypergraph<V, HE>
where
    V: Copy + Eq + Debug,
    HE: Copy + Eq + Debug,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<V, HE> Hypergraph<V, HE>
where
    V: Copy + Eq + Debug,
    HE: Copy + Eq + Debug,
{
    /// An empty hypergraph with both index counters at `0`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            vertices: HashMap::new(),
            hyperedges: HashMap::new(),
            next_vertex: 0,
            next_hyperedge: 0,
        }
    }

    // ------------------------------------------------------------------
    // Vertices
    // ------------------------------------------------------------------

    /// Add a vertex with `weight`, returning its fresh stable index. Infallible.
    pub fn add_vertex(&mut self, weight: V) -> VertexIndex {
        let idx = self.next_vertex;
        self.next_vertex += 1;
        self.vertices.insert(idx, weight);
        VertexIndex(idx)
    }

    /// The weight of a vertex, by value (`Copy`).
    ///
    /// # Errors
    ///
    /// [`HypergraphError::VertexNotFound`] if the index is unknown/removed.
    pub fn get_vertex_weight(&self, index: VertexIndex) -> Result<V, HypergraphError> {
        self.vertices
            .get(&index.0)
            .copied()
            .ok_or(HypergraphError::VertexNotFound(index))
    }

    /// Set a vertex's weight.
    ///
    /// **Divergence:** setting the *same* weight is a no-op that returns `Ok`
    /// (yamafaktory errors on an unchanged update). See the module docs.
    ///
    /// # Errors
    ///
    /// [`HypergraphError::VertexNotFound`] if the index is unknown/removed.
    pub fn update_vertex_weight(
        &mut self,
        index: VertexIndex,
        weight: V,
    ) -> Result<(), HypergraphError> {
        match self.vertices.get_mut(&index.0) {
            Some(slot) => {
                *slot = weight;
                Ok(())
            }
            None => Err(HypergraphError::VertexNotFound(index)),
        }
    }

    /// Remove a vertex, **cascading** into the hyperedges that referenced it.
    ///
    /// Every hyperedge has this vertex filtered out of its member list (all
    /// occurrences). A hyperedge whose list becomes **empty** as a result is
    /// removed entirely; otherwise it survives with the vertex filtered out. The
    /// removed vertex index is never reused.
    ///
    /// # Errors
    ///
    /// [`HypergraphError::VertexNotFound`] if the index is unknown/removed.
    pub fn remove_vertex(&mut self, index: VertexIndex) -> Result<(), HypergraphError> {
        if self.vertices.remove(&index.0).is_none() {
            return Err(HypergraphError::VertexNotFound(index));
        }
        let mut emptied: Vec<usize> = Vec::new();
        for (&he, (members, _)) in &mut self.hyperedges {
            if members.contains(&index) {
                members.retain(|&v| v != index);
                if members.is_empty() {
                    emptied.push(he);
                }
            }
        }
        for he in emptied {
            self.hyperedges.remove(&he);
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Hyperedges
    // ------------------------------------------------------------------

    /// Add a hyperedge over an ORDERED `vertices` list with `weight`.
    ///
    /// The vertex list is ordered and the same vertex MAY repeat.
    /// **Idempotent on duplicates:** if a hyperedge with an identical *(ordered
    /// vertices, weight)* pair already exists, its existing index is returned and
    /// no new hyperedge is created (linear scan).
    ///
    /// Insertion cannot itself create two identical edges, but a
    /// [`remove_vertex`](Hypergraph::remove_vertex) **cascade can**: filtering a
    /// vertex out of two distinct edges may collapse them to the same
    /// `(vertices, weight)`. To keep the returned index deterministic under
    /// replay, this scans **all** matches and returns the **smallest** index.
    ///
    /// # Errors
    ///
    /// - [`HypergraphError::HyperedgeNoVertices`] if `vertices` is empty.
    /// - [`HypergraphError::VerticesNotFound`] if any listed vertex is unknown.
    pub fn add_hyperedge(
        &mut self,
        vertices: Vec<VertexIndex>,
        weight: HE,
    ) -> Result<HyperedgeIndex, HypergraphError> {
        if vertices.is_empty() {
            return Err(HypergraphError::HyperedgeNoVertices);
        }
        let missing: Vec<VertexIndex> = vertices
            .iter()
            .copied()
            .filter(|v| !self.vertices.contains_key(&v.0))
            .collect();
        if !missing.is_empty() {
            return Err(HypergraphError::VerticesNotFound(missing));
        }
        // Idempotency: return the SMALLEST existing index whose (vertices,
        // weight) is identical. `min` over all matches makes the result
        // independent of HashMap iteration order, which matters because a
        // remove_vertex cascade can leave two edges identical.
        let existing = self
            .hyperedges
            .iter()
            .filter(|(_, (members, w))| *members == vertices && *w == weight)
            .map(|(&he, _)| he)
            .min();
        if let Some(he) = existing {
            return Ok(HyperedgeIndex(he));
        }
        let idx = self.next_hyperedge;
        self.next_hyperedge += 1;
        self.hyperedges.insert(idx, (vertices, weight));
        Ok(HyperedgeIndex(idx))
    }

    /// The ordered member list of a hyperedge, as an **owned** order-preserving
    /// clone.
    ///
    /// Returns an owned `Vec` (not a borrow) because koalisi's caller uses it as
    /// the read-half of an atomic read-modify-write: the member list is read,
    /// mutated, and written back under one write lock, so a borrow into `self`
    /// could not be held across the mutation. For read-only inspection with no
    /// following write, prefer the borrowing
    /// [`hyperedge_vertices`](Hypergraph::hyperedge_vertices).
    ///
    /// **Dedup before the magnitude layer:** the returned list may contain
    /// duplicate members (semantic #2), but `coalition_value` /
    /// `Coalition::from_enriched` reject duplicates — deduplicate first (see the
    /// module's "real consumer path" note).
    ///
    /// # Errors
    ///
    /// [`HypergraphError::HyperedgeNotFound`] if the index is unknown/removed.
    pub fn get_hyperedge_vertices(
        &self,
        index: HyperedgeIndex,
    ) -> Result<Vec<VertexIndex>, HypergraphError> {
        self.hyperedges
            .get(&index.0)
            .map(|(members, _)| members.clone())
            .ok_or(HypergraphError::HyperedgeNotFound(index))
    }

    /// Borrow a hyperedge's ordered member list for read-only inspection (no
    /// clone).
    ///
    /// Use this when you only need to read the members and are **not** about to
    /// mutate the same hyperedge (which would require the borrow to end first —
    /// use the owned [`get_hyperedge_vertices`](Hypergraph::get_hyperedge_vertices)
    /// for the read-modify-write case).
    ///
    /// **Dedup before the magnitude layer** — same caveat as
    /// [`get_hyperedge_vertices`](Hypergraph::get_hyperedge_vertices).
    ///
    /// # Errors
    ///
    /// [`HypergraphError::HyperedgeNotFound`] if the index is unknown/removed.
    pub fn hyperedge_vertices(
        &self,
        index: HyperedgeIndex,
    ) -> Result<&[VertexIndex], HypergraphError> {
        self.hyperedges
            .get(&index.0)
            .map(|(members, _)| members.as_slice())
            .ok_or(HypergraphError::HyperedgeNotFound(index))
    }

    /// The weight of a hyperedge, by value (`Copy`).
    ///
    /// # Errors
    ///
    /// [`HypergraphError::HyperedgeNotFound`] if the index is unknown/removed.
    pub fn get_hyperedge_weight(&self, index: HyperedgeIndex) -> Result<HE, HypergraphError> {
        self.hyperedges
            .get(&index.0)
            .map(|(_, w)| *w)
            .ok_or(HypergraphError::HyperedgeNotFound(index))
    }

    /// Set a hyperedge's weight.
    ///
    /// **Divergence:** setting the *same* weight is a no-op that returns `Ok`.
    ///
    /// # Errors
    ///
    /// [`HypergraphError::HyperedgeNotFound`] if the index is unknown/removed.
    pub fn update_hyperedge_weight(
        &mut self,
        index: HyperedgeIndex,
        weight: HE,
    ) -> Result<(), HypergraphError> {
        match self.hyperedges.get_mut(&index.0) {
            Some((_, w)) => {
                *w = weight;
                Ok(())
            }
            None => Err(HypergraphError::HyperedgeNotFound(index)),
        }
    }

    /// Replace a hyperedge's ordered member list.
    ///
    /// **Divergence (load-bearing for koalisi):** passing an *unchanged* list
    /// returns `Ok` (see the module docs). yamafaktory errors on an unchanged
    /// update; making it succeed is what makes
    /// `CoalitionManager::try_join_coalition`'s documented re-join idempotency
    /// ("idempotent if `agent` is already a member") actually hold — a re-join of
    /// an already-present agent leaves the list unchanged.
    ///
    /// # Errors
    ///
    /// - [`HypergraphError::HyperedgeNotFound`] if the index is unknown/removed.
    /// - [`HypergraphError::HyperedgeNoVertices`] if the new list is empty.
    /// - [`HypergraphError::VerticesNotFound`] if any listed vertex is unknown.
    pub fn update_hyperedge_vertices(
        &mut self,
        index: HyperedgeIndex,
        vertices: Vec<VertexIndex>,
    ) -> Result<(), HypergraphError> {
        if !self.hyperedges.contains_key(&index.0) {
            return Err(HypergraphError::HyperedgeNotFound(index));
        }
        if vertices.is_empty() {
            return Err(HypergraphError::HyperedgeNoVertices);
        }
        let missing: Vec<VertexIndex> = vertices
            .iter()
            .copied()
            .filter(|v| !self.vertices.contains_key(&v.0))
            .collect();
        if !missing.is_empty() {
            return Err(HypergraphError::VerticesNotFound(missing));
        }
        // Overwrite unconditionally — an unchanged list is an Ok no-op.
        // Existence was verified above, so the slot is present.
        let (members, _) = self
            .hyperedges
            .get_mut(&index.0)
            .expect("invariant: hyperedge existence checked above");
        *members = vertices;
        Ok(())
    }

    /// Remove a hyperedge. Its index is never reused.
    ///
    /// # Errors
    ///
    /// [`HypergraphError::HyperedgeNotFound`] if the index is unknown/removed.
    pub fn remove_hyperedge(&mut self, index: HyperedgeIndex) -> Result<(), HypergraphError> {
        self.hyperedges
            .remove(&index.0)
            .map(|_| ())
            .ok_or(HypergraphError::HyperedgeNotFound(index))
    }

    /// Reverse a hyperedge's ordered member list in place. A palindromic list is
    /// a no-op that returns `Ok`.
    ///
    /// # Errors
    ///
    /// [`HypergraphError::HyperedgeNotFound`] if the index is unknown/removed.
    pub fn reverse_hyperedge(&mut self, index: HyperedgeIndex) -> Result<(), HypergraphError> {
        match self.hyperedges.get_mut(&index.0) {
            Some((members, _)) => {
                members.reverse();
                Ok(())
            }
            None => Err(HypergraphError::HyperedgeNotFound(index)),
        }
    }

    /// Join hyperedges: concatenate every listed hyperedge's member list (in
    /// argument order) into the FIRST edge's list, remove the tail edges, and
    /// return the first index.
    ///
    /// Requires at least two **distinct existing** hyperedges. Order and
    /// duplicates are preserved in the concatenated result.
    ///
    /// **Weight semantics:** the joined edge keeps the **first** edge's weight;
    /// the tail edges' weights are discarded along with the tail edges. This
    /// matches yamafaktory v4.2.0 exactly — its `join_hyperedges` moves all
    /// vertices into `hyperedges[0]` and removes the tail, never touching
    /// weights.
    ///
    /// # Errors
    ///
    /// - [`HypergraphError::InvalidJoin`] if fewer than two indices are given or
    ///   any index is repeated (an edge cannot be joined with itself).
    /// - [`HypergraphError::HyperedgeNotFound`] if any listed index is unknown.
    pub fn join_hyperedges(
        &mut self,
        indices: &[HyperedgeIndex],
    ) -> Result<HyperedgeIndex, HypergraphError> {
        if indices.len() < 2 {
            return Err(HypergraphError::InvalidJoin);
        }
        let mut seen: HashSet<HyperedgeIndex> = HashSet::with_capacity(indices.len());
        for &he in indices {
            if !seen.insert(he) {
                return Err(HypergraphError::InvalidJoin);
            }
        }
        for &he in indices {
            if !self.hyperedges.contains_key(&he.0) {
                return Err(HypergraphError::HyperedgeNotFound(he));
            }
        }
        let first = indices[0];
        // Gather the tail members (in argument order) before mutating `first`.
        let mut appended: Vec<VertexIndex> = Vec::new();
        for &he in &indices[1..] {
            appended.extend_from_slice(&self.hyperedges[&he.0].0);
        }
        // Existence of every index was verified above.
        let (members, _) = self
            .hyperedges
            .get_mut(&first.0)
            .expect("invariant: hyperedge existence checked above");
        members.extend(appended);
        for &he in &indices[1..] {
            self.hyperedges.remove(&he.0);
        }
        Ok(first)
    }

    /// Contract a set of vertices within one hyperedge onto a `target` vertex.
    ///
    /// Within that hyperedge's list only: every occurrence of each vertex in
    /// `vertices` is replaced by `target`, then **adjacent** runs of `target`
    /// (created by the replacement) are collapsed to a single `target`. Returns
    /// the new member list.
    ///
    /// An **empty** `vertices` list is a true no-op: the current member list is
    /// returned unchanged, with no collapse of pre-existing adjacent `target`
    /// runs (there was no replacement to merge).
    ///
    /// The collapse is **adjacent-only** — this mirrors yamafaktory `hypergraph`
    /// v4.2.0's observable behavior. koalisi's wrapper
    /// (`topology/temporal.rs::contract_hyperedge_vertices`) is a pass-through
    /// with no documented full-dedup expectation and no business-logic caller, so
    /// the adjacent-only contract stands: a list `[t, x, t]` where `x` is
    /// unrelated stays `[t, x, t]`, and `[a, t, a]`-contract-`a`→`t` becomes
    /// `[t]` (the two `t`s that surround the contracted `a` are adjacent after
    /// replacement).
    ///
    /// # Errors
    ///
    /// - [`HypergraphError::HyperedgeNotFound`] if the index is unknown/removed.
    /// - [`HypergraphError::InvalidContraction`] if `target` or any vertex in
    ///   `vertices` is not currently a member of the hyperedge's list.
    pub fn contract_hyperedge_vertices(
        &mut self,
        index: HyperedgeIndex,
        vertices: Vec<VertexIndex>,
        target: VertexIndex,
    ) -> Result<Vec<VertexIndex>, HypergraphError> {
        let (members, _) = self
            .hyperedges
            .get_mut(&index.0)
            .ok_or(HypergraphError::HyperedgeNotFound(index))?;
        // Empty to-contract set → true no-op (do NOT collapse pre-existing runs).
        if vertices.is_empty() {
            return Ok(members.clone());
        }
        if !members.contains(&target) {
            return Err(HypergraphError::InvalidContraction {
                reason: format!("target {target} is not a member of hyperedge {index}"),
            });
        }
        let to_contract: HashSet<VertexIndex> = vertices.iter().copied().collect();
        for &v in &to_contract {
            if !members.contains(&v) {
                return Err(HypergraphError::InvalidContraction {
                    reason: format!("vertex {v} to contract is not a member of hyperedge {index}"),
                });
            }
        }
        // Replace occurrences with `target`, then collapse ADJACENT `target` runs.
        let mut out: Vec<VertexIndex> = Vec::with_capacity(members.len());
        for &x in members.iter() {
            let mapped = if to_contract.contains(&x) { target } else { x };
            if mapped == target && out.last() == Some(&target) {
                continue;
            }
            out.push(mapped);
        }
        *members = out.clone();
        Ok(out)
    }

    // ------------------------------------------------------------------
    // Counts, clears, iteration
    // ------------------------------------------------------------------

    /// The number of vertices currently in the graph.
    #[must_use]
    pub fn count_vertices(&self) -> usize {
        self.vertices.len()
    }

    /// The number of hyperedges currently in the graph.
    #[must_use]
    pub fn count_hyperedges(&self) -> usize {
        self.hyperedges.len()
    }

    /// Remove every hyperedge (vertices untouched). **Index counters are NOT
    /// reset** — indices are never reused, even across a clear.
    pub fn clear_hyperedges(&mut self) {
        self.hyperedges.clear();
    }

    /// Remove every vertex and hyperedge. **Index counters are NOT reset** —
    /// indices are never reused, even across a clear.
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.hyperedges.clear();
    }

    /// All vertex indices, **sorted ascending** for deterministic iteration.
    #[must_use]
    pub fn vertex_indices(&self) -> Vec<VertexIndex> {
        let mut out: Vec<VertexIndex> = self.vertices.keys().map(|&k| VertexIndex(k)).collect();
        out.sort_unstable();
        out
    }

    /// All hyperedge indices, **sorted ascending** for deterministic iteration.
    #[must_use]
    pub fn hyperedge_indices(&self) -> Vec<HyperedgeIndex> {
        let mut out: Vec<HyperedgeIndex> =
            self.hyperedges.keys().map(|&k| HyperedgeIndex(k)).collect();
        out.sort_unstable();
        out
    }

    /// All `(index, weight)` vertex pairs, **sorted by index** (deterministic).
    #[must_use]
    pub fn vertices(&self) -> Vec<(VertexIndex, V)> {
        let mut out: Vec<(VertexIndex, V)> = self
            .vertices
            .iter()
            .map(|(&k, &w)| (VertexIndex(k), w))
            .collect();
        out.sort_unstable_by_key(|(i, _)| *i);
        out
    }

    /// All `(index, members, weight)` hyperedge triples, **sorted by index**
    /// (deterministic). Member lists are order-preserving clones.
    #[must_use]
    pub fn hyperedges(&self) -> Vec<(HyperedgeIndex, Vec<VertexIndex>, HE)> {
        let mut out: Vec<(HyperedgeIndex, Vec<VertexIndex>, HE)> = self
            .hyperedges
            .iter()
            .map(|(&k, (members, w))| (HyperedgeIndex(k), members.clone(), *w))
            .collect();
        out.sort_unstable_by_key(|(i, _, _)| *i);
        out
    }

    // ------------------------------------------------------------------
    // Categorical bridge
    // ------------------------------------------------------------------

    /// Read a hyperedge as the **identity cospan over its member index list**.
    ///
    /// The middle (apex) is the hyperedge's ordered member [`VertexIndex`] list
    /// (duplicates preserved); both legs are the identity `0..k`, so the result
    /// satisfies `is_left_identity() && is_right_identity()`. The middle currency
    /// is the member *identities* — the same currency
    /// `Coalition::from_enriched` builds its own internal cospan from — **not**
    /// vertex weights (two distinct vertices with equal weights must stay
    /// distinct).
    ///
    /// This is a categorical *view* for cospan-level composition **within
    /// applied**, not a shortcut into the magnitude layer. Under the
    /// `WeightedCospan` implied-edge reading, an identity cospan's edges are the
    /// bipartite product of its leg targets — the complete set of `(i, j)` member
    /// pairs, i.e. every coupling slot — so it is emphatically **not** a discrete
    /// graph. See the module's "categorical view" and "real consumer path"
    /// notes; the magnitude layer takes the plain-data
    /// [`get_hyperedge_vertices`](Hypergraph::get_hyperedge_vertices) →
    /// `coalition_value` path, and duplicate members must be removed before that
    /// step.
    ///
    /// Built via [`HasIdentity::identity`](catgraph::category::HasIdentity) on
    /// the member list.
    ///
    /// # Errors
    ///
    /// [`HypergraphError::HyperedgeNotFound`] if the index is unknown/removed.
    pub fn hyperedge_as_cospan(
        &self,
        index: HyperedgeIndex,
    ) -> Result<Cospan<VertexIndex>, HypergraphError> {
        let (members, _) = self
            .hyperedges
            .get(&index.0)
            .ok_or(HypergraphError::HyperedgeNotFound(index))?;
        Ok(<Cospan<VertexIndex> as HasIdentity<Vec<VertexIndex>>>::identity(members))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Index newtypes: Display, From/Into, get.
    // ------------------------------------------------------------------
    #[test]
    fn index_newtype_conversions() {
        let v: VertexIndex = 3usize.into();
        let e = HyperedgeIndex::from(7);
        assert_eq!(v.get(), 3);
        assert_eq!(usize::from(v), 3);
        assert_eq!(e.get(), 7);
        assert_eq!(usize::from(e), 7);
        assert_eq!(format!("{v}"), "v3");
        assert_eq!(format!("{e}"), "e7");
        // Ord for sorting determinism.
        assert!(VertexIndex(1) < VertexIndex(2));
    }

    // ------------------------------------------------------------------
    // Load-bearing invariant #1: stable, never-reused indices across
    // interleaved add/remove (and across clear).
    // ------------------------------------------------------------------
    #[test]
    fn indices_are_stable_and_never_reused() {
        let mut g: Hypergraph<char, u8> = Hypergraph::new();
        let a = g.add_vertex('a');
        let b = g.add_vertex('b');
        let c = g.add_vertex('c');
        assert_eq!((a, b, c), (VertexIndex(0), VertexIndex(1), VertexIndex(2)));

        g.remove_vertex(b).unwrap();
        let d = g.add_vertex('d');
        // d gets a FRESH index (3), NOT b's reclaimed 1.
        assert_eq!(d, VertexIndex(3));
        // a and c are unchanged; b errors forever.
        assert_eq!(g.get_vertex_weight(a).unwrap(), 'a');
        assert_eq!(g.get_vertex_weight(c).unwrap(), 'c');
        assert_eq!(
            g.get_vertex_weight(b),
            Err(HypergraphError::VertexNotFound(b))
        );
        assert_eq!(g.vertex_indices(), vec![a, c, d]);

        // clear() does NOT reset the counter: the next vertex is 4.
        g.clear();
        let e = g.add_vertex('e');
        assert_eq!(e, VertexIndex(4));
    }

    // ------------------------------------------------------------------
    // remove_vertex cascade: sole-vertex edge removed; multi-vertex edge
    // filtered (all occurrences).
    // ------------------------------------------------------------------
    #[test]
    fn remove_vertex_cascade_both_arms() {
        let mut g: Hypergraph<char, u8> = Hypergraph::new();
        let a = g.add_vertex('a');
        let b = g.add_vertex('b');

        // Sole-vertex edge over {b} — removed entirely when b goes.
        let sole = g.add_hyperedge(vec![b], 1).unwrap();
        // Multi-vertex edge with b appearing twice — filtered to just [a, a].
        let multi = g.add_hyperedge(vec![a, b, a, b], 2).unwrap();

        g.remove_vertex(b).unwrap();

        assert_eq!(
            g.get_hyperedge_vertices(sole),
            Err(HypergraphError::HyperedgeNotFound(sole))
        );
        assert_eq!(g.get_hyperedge_vertices(multi).unwrap(), vec![a, a]);
        assert_eq!(g.count_hyperedges(), 1);
    }

    // ------------------------------------------------------------------
    // add_hyperedge: empty / unknown-vertex errors; idempotency returns the
    // SAME index; ordered duplicate vertices preserved.
    // ------------------------------------------------------------------
    #[test]
    fn add_hyperedge_validation_and_idempotency() {
        let mut g: Hypergraph<char, u8> = Hypergraph::new();
        let a = g.add_vertex('a');
        let b = g.add_vertex('b');

        assert_eq!(
            g.add_hyperedge(vec![], 1),
            Err(HypergraphError::HyperedgeNoVertices)
        );
        assert_eq!(
            g.add_hyperedge(vec![a, VertexIndex(99)], 1),
            Err(HypergraphError::VerticesNotFound(vec![VertexIndex(99)]))
        );

        // Ordered list with a duplicate vertex is preserved verbatim.
        let e1 = g.add_hyperedge(vec![a, b, a], 5).unwrap();
        assert_eq!(g.get_hyperedge_vertices(e1).unwrap(), vec![a, b, a]);

        // Idempotent on identical (vertices, weight): same index, no new edge.
        let e2 = g.add_hyperedge(vec![a, b, a], 5).unwrap();
        assert_eq!(e1, e2);
        assert_eq!(g.count_hyperedges(), 1);

        // Different weight → new edge; different order → new edge.
        let e3 = g.add_hyperedge(vec![a, b, a], 6).unwrap();
        let e4 = g.add_hyperedge(vec![a, a, b], 5).unwrap();
        assert_ne!(e3, e1);
        assert_ne!(e4, e1);
        assert_eq!(g.count_hyperedges(), 3);
    }

    // ------------------------------------------------------------------
    // No-op updates return Ok (the yamafaktory divergence), for all three
    // update operations; unknown targets still error.
    // ------------------------------------------------------------------
    #[test]
    fn noop_updates_return_ok() {
        let mut g: Hypergraph<char, u8> = Hypergraph::new();
        let a = g.add_vertex('a');
        let b = g.add_vertex('b');
        let e = g.add_hyperedge(vec![a, b], 1).unwrap();

        // Same vertex weight → Ok.
        assert!(g.update_vertex_weight(a, 'a').is_ok());
        // Same hyperedge weight → Ok.
        assert!(g.update_hyperedge_weight(e, 1).is_ok());
        // Same (unchanged) vertex list → Ok. THE try_join_coalition idempotency
        // fix (re-join of an already-present agent leaves the list unchanged).
        assert!(g.update_hyperedge_vertices(e, vec![a, b]).is_ok());

        // Changed values also Ok and observable.
        g.update_vertex_weight(a, 'z').unwrap();
        assert_eq!(g.get_vertex_weight(a).unwrap(), 'z');

        // Unknown targets error.
        assert_eq!(
            g.update_vertex_weight(VertexIndex(99), 'x'),
            Err(HypergraphError::VertexNotFound(VertexIndex(99)))
        );
        assert_eq!(
            g.update_hyperedge_weight(HyperedgeIndex(99), 9),
            Err(HypergraphError::HyperedgeNotFound(HyperedgeIndex(99)))
        );
        // update_hyperedge_vertices: empty and unknown-vertex errors.
        assert_eq!(
            g.update_hyperedge_vertices(e, vec![]),
            Err(HypergraphError::HyperedgeNoVertices)
        );
        assert_eq!(
            g.update_hyperedge_vertices(e, vec![a, VertexIndex(99)]),
            Err(HypergraphError::VerticesNotFound(vec![VertexIndex(99)]))
        );
    }

    // ------------------------------------------------------------------
    // reverse: order flips; palindrome no-op; duplicate preserved through it.
    // ------------------------------------------------------------------
    #[test]
    fn reverse_hyperedge_semantics() {
        let mut g: Hypergraph<char, u8> = Hypergraph::new();
        let a = g.add_vertex('a');
        let b = g.add_vertex('b');
        let c = g.add_vertex('c');

        let e = g.add_hyperedge(vec![a, b, c], 1).unwrap();
        g.reverse_hyperedge(e).unwrap();
        assert_eq!(g.get_hyperedge_vertices(e).unwrap(), vec![c, b, a]);

        // Palindrome (with duplicate) is a no-op.
        let p = g.add_hyperedge(vec![a, b, a], 2).unwrap();
        g.reverse_hyperedge(p).unwrap();
        assert_eq!(g.get_hyperedge_vertices(p).unwrap(), vec![a, b, a]);

        assert_eq!(
            g.reverse_hyperedge(HyperedgeIndex(99)),
            Err(HypergraphError::HyperedgeNotFound(HyperedgeIndex(99)))
        );
    }

    // ------------------------------------------------------------------
    // join: argument-order concatenation into the first edge, tail removal,
    // fewer-than-2 / duplicate / unknown errors.
    // ------------------------------------------------------------------
    #[test]
    fn join_hyperedges_semantics() {
        let mut g: Hypergraph<char, u8> = Hypergraph::new();
        let a = g.add_vertex('a');
        let b = g.add_vertex('b');
        let c = g.add_vertex('c');

        let e0 = g.add_hyperedge(vec![a], 1).unwrap();
        let e1 = g.add_hyperedge(vec![b, c], 2).unwrap();
        let e2 = g.add_hyperedge(vec![c, a], 3).unwrap();

        let joined = g.join_hyperedges(&[e0, e1, e2]).unwrap();
        assert_eq!(joined, e0);
        // Concatenated in argument order into the first edge's list.
        assert_eq!(g.get_hyperedge_vertices(e0).unwrap(), vec![a, b, c, c, a]);
        // First edge's weight retained; tail edges removed.
        assert_eq!(g.get_hyperedge_weight(e0).unwrap(), 1);
        assert_eq!(g.count_hyperedges(), 1);
        assert_eq!(
            g.get_hyperedge_vertices(e1),
            Err(HypergraphError::HyperedgeNotFound(e1))
        );

        // Fewer than two → InvalidJoin.
        assert_eq!(g.join_hyperedges(&[e0]), Err(HypergraphError::InvalidJoin));
        // Repeated index → InvalidJoin.
        assert_eq!(
            g.join_hyperedges(&[e0, e0]),
            Err(HypergraphError::InvalidJoin)
        );
        // Unknown index → HyperedgeNotFound.
        assert_eq!(
            g.join_hyperedges(&[e0, HyperedgeIndex(99)]),
            Err(HypergraphError::HyperedgeNotFound(HyperedgeIndex(99)))
        );
    }

    // ------------------------------------------------------------------
    // contract: replace + adjacent-collapse; error when target/listed vertex
    // absent; non-adjacent target NOT collapsed.
    // ------------------------------------------------------------------
    #[test]
    fn contract_hyperedge_vertices_semantics() {
        let mut g: Hypergraph<char, u8> = Hypergraph::new();
        let a = g.add_vertex('a');
        let b = g.add_vertex('b');
        let c = g.add_vertex('c');
        let t = g.add_vertex('t');

        // Target t is a member. [t, a, t, b] contract {a} → t
        // ⇒ replace a with t ⇒ [t, t, t, b] ⇒ collapse adjacent t ⇒ [t, b].
        let e2 = g.add_hyperedge(vec![t, a, t, b], 2).unwrap();
        let out2 = g.contract_hyperedge_vertices(e2, vec![a], t).unwrap();
        assert_eq!(out2, vec![t, b]);
        assert_eq!(g.get_hyperedge_vertices(e2).unwrap(), vec![t, b]);

        // Non-adjacent target is NOT collapsed: [t, b, t] contract {} → t ⇒
        // no replacements, the two t's are non-adjacent ⇒ unchanged.
        let e3 = g.add_hyperedge(vec![t, b, t], 3).unwrap();
        let out3 = g.contract_hyperedge_vertices(e3, vec![], t).unwrap();
        assert_eq!(out3, vec![t, b, t]);

        // Duplicate vertices in the to-contract list are handled once (HashSet):
        // [t, a, a, b] contract {a, a} → t ⇒ [t, t, t, b] ⇒ collapse ⇒ [t, b].
        let e5 = g.add_hyperedge(vec![t, a, a, b], 5).unwrap();
        let out5 = g.contract_hyperedge_vertices(e5, vec![a, a], t).unwrap();
        assert_eq!(out5, vec![t, b]);

        // Errors: target not a member; listed vertex not a member.
        let e4 = g.add_hyperedge(vec![a, b], 4).unwrap();
        assert!(matches!(
            g.contract_hyperedge_vertices(e4, vec![a], t),
            Err(HypergraphError::InvalidContraction { .. })
        ));
        assert!(matches!(
            g.contract_hyperedge_vertices(e4, vec![c], a),
            Err(HypergraphError::InvalidContraction { .. })
        ));
        assert_eq!(
            g.contract_hyperedge_vertices(HyperedgeIndex(99), vec![a], a),
            Err(HypergraphError::HyperedgeNotFound(HyperedgeIndex(99)))
        );
    }

    // ------------------------------------------------------------------
    // Empty to-contract set is a TRUE no-op: pre-existing adjacent target
    // runs are NOT collapsed (regression — the loop used to run
    // unconditionally and merge [t, t, b] → [t, b]).
    // ------------------------------------------------------------------
    #[test]
    fn contract_empty_vertices_is_true_noop() {
        let mut g: Hypergraph<char, u8> = Hypergraph::new();
        let t = g.add_vertex('t');
        let b = g.add_vertex('b');

        // Pre-existing adjacent t-run [t, t, b]; contract {} → t must NOT merge.
        let e = g.add_hyperedge(vec![t, t, b], 1).unwrap();
        let out = g.contract_hyperedge_vertices(e, vec![], t).unwrap();
        assert_eq!(out, vec![t, t, b]);
        assert_eq!(g.get_hyperedge_vertices(e).unwrap(), vec![t, t, b]);
    }

    // ------------------------------------------------------------------
    // clears do not reset counters; clear_hyperedges leaves vertices.
    // ------------------------------------------------------------------
    #[test]
    fn clears_preserve_counters() {
        let mut g: Hypergraph<char, u8> = Hypergraph::new();
        let a = g.add_vertex('a');
        let b = g.add_vertex('b');
        let _e = g.add_hyperedge(vec![a, b], 1).unwrap();

        g.clear_hyperedges();
        assert_eq!(g.count_hyperedges(), 0);
        assert_eq!(g.count_vertices(), 2);
        // Next hyperedge index is fresh (1), not reset to 0.
        let e2 = g.add_hyperedge(vec![a], 2).unwrap();
        assert_eq!(e2, HyperedgeIndex(1));

        g.clear();
        assert_eq!(g.count_vertices(), 0);
        // Next vertex index is fresh (2), not reset to 0.
        let c = g.add_vertex('c');
        assert_eq!(c, VertexIndex(2));
    }

    // ------------------------------------------------------------------
    // Cospan view: hyperedge over [a, b, c] → identity cospan whose MIDDLE is
    // the member INDEX list (identities, not weights), order + duplicates
    // preserved, both legs identity.
    // ------------------------------------------------------------------
    #[test]
    fn hyperedge_as_cospan_is_identity_over_member_indices() {
        let mut g: Hypergraph<char, u8> = Hypergraph::new();
        // Two vertices with EQUAL weights must stay distinguishable in the
        // middle — the reason the currency is identities, not weights.
        let a = g.add_vertex('x');
        let b = g.add_vertex('x');
        let c = g.add_vertex('y');

        let e = g.add_hyperedge(vec![a, b, c], 1).unwrap();
        let cospan = g.hyperedge_as_cospan(e).unwrap();
        assert_eq!(cospan.middle(), &[a, b, c]);
        // a and b are distinct indices despite equal weights.
        assert_ne!(cospan.middle()[0], cospan.middle()[1]);
        assert!(cospan.is_left_identity());
        assert!(cospan.is_right_identity());

        // Duplicates in the member list are preserved in the middle.
        let dup = g.add_hyperedge(vec![a, a, b], 2).unwrap();
        let cospan_dup = g.hyperedge_as_cospan(dup).unwrap();
        assert_eq!(cospan_dup.middle(), &[a, a, b]);
        assert!(cospan_dup.is_left_identity() && cospan_dup.is_right_identity());

        assert_eq!(
            g.hyperedge_as_cospan(HyperedgeIndex(99)).unwrap_err(),
            HypergraphError::HyperedgeNotFound(HyperedgeIndex(99))
        );
    }

    // ------------------------------------------------------------------
    // Borrowing accessor returns the same members without cloning.
    // ------------------------------------------------------------------
    #[test]
    fn hyperedge_vertices_borrow_matches_owned() {
        let mut g: Hypergraph<char, u8> = Hypergraph::new();
        let a = g.add_vertex('a');
        let b = g.add_vertex('b');
        let e = g.add_hyperedge(vec![a, b, a], 1).unwrap();
        assert_eq!(g.hyperedge_vertices(e).unwrap(), &[a, b, a]);
        assert_eq!(
            g.hyperedge_vertices(e).unwrap(),
            g.get_hyperedge_vertices(e).unwrap().as_slice()
        );
        assert_eq!(
            g.hyperedge_vertices(HyperedgeIndex(99)),
            Err(HypergraphError::HyperedgeNotFound(HyperedgeIndex(99)))
        );
    }

    // ------------------------------------------------------------------
    // add_hyperedge idempotency stays DETERMINISTIC when a remove_vertex
    // cascade collapses two distinct edges to the same (vertices, weight):
    // the SMALLEST matching index is returned.
    // ------------------------------------------------------------------
    #[test]
    fn add_hyperedge_idempotency_deterministic_after_cascade_collision() {
        let mut g: Hypergraph<char, u8> = Hypergraph::new();
        let a = g.add_vertex('a');
        let b = g.add_vertex('b');
        let x = g.add_vertex('x');
        let y = g.add_vertex('y');

        // e0 = [a, x, b] @ 7, e1 = [a, y, b] @ 7 — distinct now.
        let e0 = g.add_hyperedge(vec![a, x, b], 7).unwrap();
        let e1 = g.add_hyperedge(vec![a, y, b], 7).unwrap();
        assert_ne!(e0, e1);

        // Cascade both x and y out → both edges become ([a, b], 7): a collision.
        g.remove_vertex(x).unwrap();
        g.remove_vertex(y).unwrap();
        assert_eq!(g.get_hyperedge_vertices(e0).unwrap(), vec![a, b]);
        assert_eq!(g.get_hyperedge_vertices(e1).unwrap(), vec![a, b]);

        // Re-adding ([a, b], 7) must deterministically return the SMALLEST
        // matching index (e0), regardless of HashMap iteration order, and add
        // no new edge.
        let before = g.count_hyperedges();
        for _ in 0..8 {
            assert_eq!(g.add_hyperedge(vec![a, b], 7).unwrap(), e0.min(e1));
        }
        assert_eq!(g.count_hyperedges(), before);
    }

    // ------------------------------------------------------------------
    // Deterministic iteration accessors are sorted by index.
    // ------------------------------------------------------------------
    #[test]
    fn iteration_accessors_are_sorted() {
        let mut g: Hypergraph<char, u8> = Hypergraph::new();
        let a = g.add_vertex('a');
        let b = g.add_vertex('b');
        let c = g.add_vertex('c');
        g.remove_vertex(b).unwrap();
        assert_eq!(g.vertex_indices(), vec![a, c]);
        assert_eq!(g.vertices(), vec![(a, 'a'), (c, 'c')]);

        let e0 = g.add_hyperedge(vec![a], 1).unwrap();
        let e1 = g.add_hyperedge(vec![c], 2).unwrap();
        assert_eq!(g.hyperedge_indices(), vec![e0, e1]);
        assert_eq!(g.hyperedges(), vec![(e0, vec![a], 1), (e1, vec![c], 2)]);
    }
}
