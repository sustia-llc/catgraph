//! Place/transition Petri nets with Lambda-typed places.
//!
//! A Petri net is a bipartite directed graph of *places* (holding tokens) and
//! *transitions* (consuming/producing tokens). Each transition has weighted
//! pre-arcs (inputs) and post-arcs (outputs); firing a transition is an atomic
//! consume-then-produce step. Markings are sparse token assignments.
//!
//! Firing is pure: [`PetriNet::fire`] returns a new [`Marking`] without mutating
//! the original. Reachability analysis ([`PetriNet::reachable`], [`PetriNet::can_reach`])
//! uses bounded BFS over the firing graph.
//!
//! ## Cospan bridge
//!
//! [`PetriNet::from_cospan`] and [`PetriNet::transition_as_cospan`] connect Petri
//! nets to catgraph's pushout and monoidal infrastructure: left-leg multiplicities
//! become pre-arc weights and right-leg multiplicities become post-arc weights.
//! This gives Petri nets source/target (cospan) semantics where places are the
//! middle set and transitions are the morphisms.
//!
//! ## Composition
//!
//! [`PetriNet::parallel`] takes the disjoint union of places, transitions and
//! boundary legs (monoidal product). [`PetriNet::sequential`] merges sink
//! places of one net with Lambda-matching source places of another.
//! [`Composable::compose`] is neither of those: it composes over the declared
//! boundary through the decorated-cospan pushout.
//!
//! See also `examples/petri_net.rs` for chemical reaction modelling.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

use num::ToPrimitive;
use rust_decimal::Decimal;

use catgraph::category::{Composable, HasIdentity};
use catgraph::cospan::Cospan;
use catgraph::errors::{BoundaryLeg, CatgraphError};
use catgraph::hypergraph_category::HypergraphCategory;
use catgraph::monoidal::{Monoidal, SymmetricMonoidalMorphism};
use catgraph::utils::in_place_permute;
use permutations::Permutation;

use crate::decorated_cospan::{DecoratedCospan, Decoration};

/// A single transition in a Petri net.
///
/// A transition fires by consuming tokens from its pre-set and producing tokens
/// into its post-set. Each arc carries a weight indicating how many tokens are
/// consumed or produced at the connected place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transition {
    /// Pre-arcs: `(place_index, weight)` pairs specifying how many tokens
    /// must be present (and are consumed) at each input place.
    pre: Vec<(usize, Decimal)>,
    /// Post-arcs: `(place_index, weight)` pairs specifying how many tokens
    /// are produced at each output place.
    post: Vec<(usize, Decimal)>,
}

impl Transition {
    /// Construct a transition from its pre-arc and post-arc weight lists.
    #[must_use]
    pub fn new(pre: Vec<(usize, Decimal)>, post: Vec<(usize, Decimal)>) -> Self {
        Self { pre, post }
    }

    /// The pre-arcs (input places and their weights).
    #[must_use]
    pub fn pre(&self) -> &[(usize, Decimal)] {
        &self.pre
    }

    /// The post-arcs (output places and their weights).
    #[must_use]
    pub fn post(&self) -> &[(usize, Decimal)] {
        &self.post
    }

    /// Relabel place indices through a quotient map.
    ///
    /// Each place index `i` in the pre/post arcs is replaced by `quotient[i]`.
    /// This is the action of the decoration functor `F` on the coequalizer
    /// quotient arising in decorated-cospan composition (Fong–Spivak
    /// Def 6.75): when two apex vertices are identified during a pushout, any
    /// transition referring to either must have its arc endpoints redirected
    /// to the identified representative.
    ///
    /// When the quotient collapses two distinct places onto the same target,
    /// arcs referring to them are merged with summed [`Decimal`] multiplicities —
    /// pre-arcs and post-arcs are deduplicated independently so self-loops
    /// (matching pre and post place) are preserved as two separate arcs.
    /// After dedup, pre/post arcs are sorted ascending by place index for a
    /// canonical representation.
    ///
    /// # Panics
    ///
    /// Panics if any `(place, _)` pair references an index outside
    /// `quotient.len()`.
    #[must_use]
    pub fn relabel(&self, quotient: &[usize]) -> Transition {
        let mut pre: Vec<(usize, Decimal)> =
            self.pre.iter().map(|(p, w)| (quotient[*p], *w)).collect();
        let mut post: Vec<(usize, Decimal)> =
            self.post.iter().map(|(p, w)| (quotient[*p], *w)).collect();
        Self::dedup_arcs(&mut pre);
        Self::dedup_arcs(&mut post);
        Transition { pre, post }
    }

    /// Merge arcs referencing the same place by summing their [`Decimal`]
    /// weights, then sort ascending by place index. Called by
    /// [`Transition::relabel`] after the quotient may have collapsed distinct
    /// source places onto the same target.
    fn dedup_arcs(arcs: &mut Vec<(usize, Decimal)>) {
        arcs.sort_unstable_by_key(|&(p, _)| p);
        let mut i = 0;
        while i + 1 < arcs.len() {
            if arcs[i].0 == arcs[i + 1].0 {
                let next_weight = arcs[i + 1].1;
                arcs[i].1 += next_weight;
                arcs.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }
}

/// A token distribution across places in a Petri net.
///
/// Stored as a sparse map from place index to token count; places with zero
/// tokens are not stored. Two markings are equal iff they assign the same
/// token count to every place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Marking {
    /// Sparse map: place index → nonzero token count.
    tokens: HashMap<usize, Decimal>,
}

impl Marking {
    /// Create an empty marking (no tokens anywhere).
    #[must_use]
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
        }
    }

    /// Create a marking from `(place_index, count)` pairs.
    /// Pairs with count zero are silently dropped.
    #[must_use]
    pub fn from_vec(pairs: Vec<(usize, Decimal)>) -> Self {
        let tokens: HashMap<usize, Decimal> =
            pairs.into_iter().filter(|(_, c)| !c.is_zero()).collect();
        Self { tokens }
    }

    /// Set the token count at a place. Setting to zero removes the entry.
    pub fn set(&mut self, place: usize, count: Decimal) {
        if count.is_zero() {
            self.tokens.remove(&place);
        } else {
            self.tokens.insert(place, count);
        }
    }

    /// Get the token count at a place (zero if absent).
    #[must_use]
    pub fn get(&self, place: usize) -> Decimal {
        self.tokens.get(&place).copied().unwrap_or(Decimal::ZERO)
    }

    /// The underlying sparse token map.
    #[must_use]
    pub fn tokens(&self) -> &HashMap<usize, Decimal> {
        &self.tokens
    }
}

impl Default for Marking {
    fn default() -> Self {
        Self::new()
    }
}

/// Hash by sorting entries by place index so that equal markings always
/// produce the same hash regardless of `HashMap` iteration order.
impl Hash for Marking {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut entries: Vec<_> = self.tokens.iter().collect();
        entries.sort_by_key(|(k, _)| *k);
        for (k, v) in entries {
            k.hash(state);
            v.hash(state);
        }
    }
}

/// A place/transition Petri net with Lambda-typed places and a declared
/// open boundary.
///
/// The type parameter `Lambda` labels each place (e.g. `char` for chemical
/// species like `'H'`, `'O'`, `'W'`). The net stores a flat vector of places,
/// a vector of [`Transition`]s whose arc indices refer into the places vector,
/// and two boundary legs — `left` and `right` — that also index the places
/// vector and carry the net's [`Composable::domain`] / [`Composable::codomain`]
/// words. The legs are the [`Cospan`] shape: a leg is a function from boundary
/// slot to place index.
///
/// All firing operations are pure: they return new [`Marking`]s without mutating
/// the net or the input marking.
#[derive(Clone, Debug)]
pub struct PetriNet<Lambda: Sized + Eq + Copy + Debug> {
    /// Lambda-typed places, indexed by position.
    places: Vec<Lambda>,
    /// Transitions with weighted arcs referencing place indices.
    transitions: Vec<Transition>,
    /// Domain leg: maps each domain boundary slot to a place index.
    left: Vec<usize>,
    /// Codomain leg: maps each codomain boundary slot to a place index.
    right: Vec<usize>,
}

impl<Lambda> PetriNet<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug,
{
    /// Construct a Petri net, checking that every arc **and** every boundary
    /// leg entry indexes a place of this net.
    ///
    /// That property is what makes a net *well-formed*;
    /// [`transition_as_cospan`](Self::transition_as_cospan) builds its legs
    /// directly out of arc place indices, and
    /// [`domain`](Composable::domain) / [`codomain`](Composable::codomain)
    /// read `left` / `right` through `places`.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::PetriNet`] naming the transition, the side, and
    /// the offending index if any `pre` or `post` arc references a place index
    /// at or beyond `places.len()`. Transitions are scanned in order and, within
    /// a transition, `pre` before `post`.
    ///
    /// Returns [`CatgraphError::ConstructionIndexOutOfBounds`] naming the leg,
    /// the position and the target if any `left` or `right` entry targets a
    /// place index at or beyond `places.len()`. Arcs are scanned before legs,
    /// and `left` before `right`.
    pub fn new(
        places: Vec<Lambda>,
        transitions: Vec<Transition>,
        left: Vec<usize>,
        right: Vec<usize>,
    ) -> Result<Self, CatgraphError> {
        for (transition, t) in transitions.iter().enumerate() {
            for (side, arcs) in [("pre", &t.pre), ("post", &t.post)] {
                for &(place, _) in arcs {
                    if place >= places.len() {
                        return Err(CatgraphError::PetriNet {
                            message: format!(
                                "transition {transition}'s {side} arc references place {place}, \
                                 but the net has {} place(s)",
                                places.len()
                            ),
                        });
                    }
                }
            }
        }
        for (leg, entries) in [
            (BoundaryLeg::Domain, left.as_slice()),
            (BoundaryLeg::Codomain, right.as_slice()),
        ] {
            for (position, &target) in entries.iter().enumerate() {
                if target >= places.len() {
                    return Err(CatgraphError::ConstructionIndexOutOfBounds {
                        leg,
                        position,
                        target,
                        target_len: places.len(),
                    });
                }
            }
        }
        Ok(Self::new_unchecked(places, transitions, left, right))
    }

    /// Construct a Petri net without checking that its arcs or its boundary
    /// legs index its places.
    ///
    /// The well-formedness invariant is the caller's responsibility. Use this
    /// where the arcs and legs are correct **by construction** — a net
    /// assembled from another net's own places, or a literal fixture — and
    /// [`new`](Self::new) everywhere the places, transitions and legs cross a
    /// trust boundary.
    ///
    /// Mirrors [`Cospan::new_unchecked`](catgraph::cospan::Cospan::new_unchecked).
    #[must_use]
    pub fn new_unchecked(
        places: Vec<Lambda>,
        transitions: Vec<Transition>,
        left: Vec<usize>,
        right: Vec<usize>,
    ) -> Self {
        Self {
            places,
            transitions,
            left,
            right,
        }
    }

    /// The Lambda-typed places.
    #[must_use]
    pub fn places(&self) -> &[Lambda] {
        &self.places
    }

    /// The domain leg: place index per domain boundary slot.
    #[must_use]
    pub fn left_to_place(&self) -> &[usize] {
        &self.left
    }

    /// The codomain leg: place index per codomain boundary slot.
    #[must_use]
    pub fn right_to_place(&self) -> &[usize] {
        &self.right
    }

    /// The transitions in this net.
    #[must_use]
    pub fn transitions(&self) -> &[Transition] {
        &self.transitions
    }

    /// Number of places.
    #[must_use]
    pub fn place_count(&self) -> usize {
        self.places.len()
    }

    /// Number of transitions.
    #[must_use]
    pub fn transition_count(&self) -> usize {
        self.transitions.len()
    }

    /// Returns indices of transitions enabled under the given marking.
    ///
    /// A transition is enabled when every pre-arc place holds at least as many
    /// tokens as the arc weight requires.
    #[must_use]
    pub fn enabled(&self, marking: &Marking) -> Vec<usize> {
        self.transitions
            .iter()
            .enumerate()
            .filter(|(_, t)| t.pre.iter().all(|(p, w)| marking.get(*p) >= *w))
            .map(|(i, _)| i)
            .collect()
    }

    /// Fire a transition, returning the new marking.
    ///
    /// Subtracts pre-arc weights and adds post-arc weights. The operation is
    /// pure: the input marking is not modified.
    ///
    /// # Errors
    /// Returns [`CatgraphError::PetriNet`] if the transition index is out of
    /// bounds or the transition is not enabled under the given marking.
    pub fn fire(&self, transition: usize, marking: &Marking) -> Result<Marking, CatgraphError> {
        if transition >= self.transitions.len() {
            return Err(CatgraphError::PetriNet {
                message: format!(
                    "transition {} out of bounds (net has {} transitions)",
                    transition,
                    self.transitions.len()
                ),
            });
        }
        let t = &self.transitions[transition];
        for (p, w) in &t.pre {
            if marking.get(*p) < *w {
                return Err(CatgraphError::PetriNet {
                    message: format!("transition {transition} not enabled under current marking"),
                });
            }
        }
        let mut result = marking.clone();
        for (p, w) in &t.pre {
            let c = result.get(*p) - w;
            result.set(*p, c);
        }
        for (p, w) in &t.post {
            let c = result.get(*p) + w;
            result.set(*p, c);
        }
        Ok(result)
    }

    /// Pre-arc weight for a (place, transition) pair. Zero if no arc.
    #[must_use]
    pub fn arc_weight_pre(&self, place: usize, transition: usize) -> Decimal {
        self.transitions.get(transition).map_or(Decimal::ZERO, |t| {
            t.pre
                .iter()
                .filter(|(p, _)| *p == place)
                .map(|(_, w)| w)
                .sum()
        })
    }

    /// Post-arc weight for a (place, transition) pair. Zero if no arc.
    #[must_use]
    pub fn arc_weight_post(&self, place: usize, transition: usize) -> Decimal {
        self.transitions.get(transition).map_or(Decimal::ZERO, |t| {
            t.post
                .iter()
                .filter(|(p, _)| *p == place)
                .map(|(_, w)| w)
                .sum()
        })
    }

    /// Places with no post-arcs from any transition (no transition produces tokens here).
    #[must_use]
    pub fn source_places(&self) -> Vec<usize> {
        (0..self.places.len())
            .filter(|p| {
                !self
                    .transitions
                    .iter()
                    .any(|t| t.post.iter().any(|(tp, _)| tp == p))
            })
            .collect()
    }

    /// Places with no pre-arcs to any transition (no transition consumes tokens from here).
    #[must_use]
    pub fn sink_places(&self) -> Vec<usize> {
        (0..self.places.len())
            .filter(|p| {
                !self
                    .transitions
                    .iter()
                    .any(|t| t.pre.iter().any(|(tp, _)| tp == p))
            })
            .collect()
    }

    /// All markings reachable from the initial marking within `max_depth` firing steps.
    ///
    /// Performs a breadth-first search over the firing graph. The returned set
    /// always includes the initial marking itself.
    #[must_use]
    pub fn reachable(&self, initial: &Marking, max_depth: usize) -> Vec<Marking> {
        let mut visited: HashSet<Marking> = HashSet::new();
        let mut queue: VecDeque<(Marking, usize)> = VecDeque::new();
        visited.insert(initial.clone());
        queue.push_back((initial.clone(), 0));
        while let Some((marking, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            for t in self.enabled(&marking) {
                if let Ok(next) = self.fire(t, &marking)
                    && visited.insert(next.clone())
                {
                    queue.push_back((next, depth + 1));
                }
            }
        }
        visited.into_iter().collect()
    }

    /// True if the target marking is reachable from `initial` within `max_depth` steps.
    ///
    /// Short-circuits as soon as the target is found. Returns `true` immediately
    /// if `initial == target`.
    #[must_use]
    pub fn can_reach(&self, initial: &Marking, target: &Marking, max_depth: usize) -> bool {
        if initial == target {
            return true;
        }
        let mut visited: HashSet<Marking> = HashSet::new();
        let mut queue: VecDeque<(Marking, usize)> = VecDeque::new();
        visited.insert(initial.clone());
        queue.push_back((initial.clone(), 0));
        while let Some((marking, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            for t in self.enabled(&marking) {
                if let Ok(next) = self.fire(t, &marking) {
                    if &next == target {
                        return true;
                    }
                    if visited.insert(next.clone()) {
                        queue.push_back((next, depth + 1));
                    }
                }
            }
        }
        false
    }

    /// Construct a single-transition Petri net from a cospan.
    ///
    /// The cospan's middle set becomes the places and both its legs are stored
    /// verbatim as the net's boundary, so `domain()` / `codomain()` reproduce
    /// [`Cospan::domain`] / [`Cospan::codomain`] entry for entry. Left-leg
    /// multiplicities (how many domain nodes map to each middle node) become
    /// pre-arc weights; right-leg multiplicities become post-arc weights. This
    /// establishes the cospan bridge between Petri net firing semantics and
    /// categorical composition.
    ///
    /// The arc lists are aggregated through a [`HashMap`] and then sorted
    /// ascending by place index, so the transition's `pre` and `post` are in
    /// that order for every input cospan.
    #[must_use]
    pub fn from_cospan(cospan: &Cospan<Lambda>) -> Self {
        let places = cospan.middle().to_vec();
        let mut pre_counts: HashMap<usize, Decimal> = HashMap::new();
        for &idx in cospan.left_to_middle() {
            *pre_counts.entry(idx).or_insert(Decimal::ZERO) += Decimal::ONE;
        }
        let mut post_counts: HashMap<usize, Decimal> = HashMap::new();
        for &idx in cospan.right_to_middle() {
            *post_counts.entry(idx).or_insert(Decimal::ZERO) += Decimal::ONE;
        }
        let mut pre: Vec<(usize, Decimal)> = pre_counts.into_iter().collect();
        pre.sort_unstable_by_key(|&(p, _)| p);
        let mut post: Vec<(usize, Decimal)> = post_counts.into_iter().collect();
        post.sort_unstable_by_key(|&(p, _)| p);
        // Correct by construction: the arc indices and the stored legs are the
        // cospan's own leg entries and the places are its middle set, so every
        // arc and every leg entry is in range by `Cospan`'s bounds invariant.
        Self::new_unchecked(
            places,
            vec![Transition::new(pre, post)],
            cospan.left_to_middle().to_vec(),
            cospan.right_to_middle().to_vec(),
        )
    }

    /// Convert a single transition to its cospan representation.
    ///
    /// Each pre-arc weight becomes a multiplicity in the left (domain) leg,
    /// and each post-arc weight becomes a multiplicity in the right (codomain)
    /// leg. Inverse of [`PetriNet::from_cospan`] for single-transition nets.
    ///
    /// # Panics
    ///
    /// Panics if `transition >= self.transitions.len()`, if an arc weight
    /// carries a negative sign, or if an arc weight is at least 2⁶⁴ after its
    /// fraction is truncated.
    ///
    /// The legs are arc place indices, so they must index this net's places —
    /// [`PetriNet::new`] establishes that, and a net built with
    /// [`PetriNet::new_unchecked`] from arcs that do not yields a structurally
    /// invalid cospan.
    #[must_use]
    pub fn transition_as_cospan(&self, transition: usize) -> Cospan<Lambda> {
        let t = &self.transitions[transition];
        let mut left = Vec::new();
        for (p, w) in &t.pre {
            let count = w.to_u64().expect("integer weight for cospan expansion");
            for _ in 0..count {
                left.push(*p);
            }
        }
        let mut right = Vec::new();
        for (p, w) in &t.post {
            let count = w.to_u64().expect("integer weight for cospan expansion");
            for _ in 0..count {
                right.push(*p);
            }
        }
        // Correct by construction: the legs are arc place indices, and
        // `PetriNet::new` refuses a net whose arcs do not index its own places.
        Cospan::new_unchecked(left, right, self.places.clone())
    }

    /// Parallel composition (monoidal product): disjoint union of places,
    /// transitions and boundary legs.
    ///
    /// Place indices in `other` — in its arcs and in both its legs — are
    /// shifted by `self.place_count()`. The domain word of the result is
    /// `self.domain()` followed by `other.domain()`, and likewise for the
    /// codomain. Neither net is modified; a new combined net is returned.
    #[must_use]
    pub fn parallel(&self, other: &Self) -> Self {
        let offset = self.places.len();
        let mut places = self.places.clone();
        places.extend_from_slice(&other.places);
        let mut transitions = self.transitions.clone();
        for t in &other.transitions {
            let pre: Vec<(usize, Decimal)> = t.pre.iter().map(|(p, w)| (p + offset, *w)).collect();
            let post: Vec<(usize, Decimal)> =
                t.post.iter().map(|(p, w)| (p + offset, *w)).collect();
            transitions.push(Transition::new(pre, post));
        }
        let mut left = self.left.clone();
        left.extend(other.left.iter().map(|p| p + offset));
        let mut right = self.right.clone();
        right.extend(other.right.iter().map(|p| p + offset));
        // Correct by construction: `places` is the concatenation of both nets'
        // places, `self`'s arcs and legs are unchanged, and `other`'s are
        // shifted by exactly the offset at which its places were appended.
        Self::new_unchecked(places, transitions, left, right)
    }

    /// Sequential composition: merge sink places of `self` with source places of `other`.
    ///
    /// Matching is by Lambda equality: each unmatched source place in `other` is
    /// paired with an unused sink place in `self` that carries the same Lambda type.
    /// Unmatched places from `other` are appended as new places. The result's
    /// domain leg is `self`'s and its codomain leg is `other`'s, remapped
    /// through the place merge.
    ///
    /// This is **not** [`Composable::compose`], which composes over the
    /// declared boundary by pushout; `sequential` matches on sink/source
    /// places and ignores both legs' contents.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError`] if boundary place types don't match between nets.
    pub fn sequential(&self, other: &Self) -> Result<Self, CatgraphError> {
        let self_sinks = self.sink_places();
        let other_sources = other.source_places();
        let mut merge_map: HashMap<usize, usize> = HashMap::new();
        let mut used_sinks: HashSet<usize> = HashSet::new();
        for &os in &other_sources {
            for &ss in &self_sinks {
                if !used_sinks.contains(&ss) && self.places[ss] == other.places[os] {
                    merge_map.insert(os, ss);
                    used_sinks.insert(ss);
                    break;
                }
            }
        }
        let mut places = self.places.clone();
        let mut other_index_map: Vec<usize> = Vec::with_capacity(other.places.len());
        for (i, &lambda) in other.places.iter().enumerate() {
            if let Some(&merged_idx) = merge_map.get(&i) {
                other_index_map.push(merged_idx);
            } else {
                other_index_map.push(places.len());
                places.push(lambda);
            }
        }
        let mut transitions = self.transitions.clone();
        for t in &other.transitions {
            let pre: Vec<(usize, Decimal)> = t
                .pre
                .iter()
                .map(|(p, w)| (other_index_map[*p], *w))
                .collect();
            let post: Vec<(usize, Decimal)> = t
                .post
                .iter()
                .map(|(p, w)| (other_index_map[*p], *w))
                .collect();
            transitions.push(Transition::new(pre, post));
        }
        let right: Vec<usize> = other.right.iter().map(|p| other_index_map[*p]).collect();
        // Correct by construction: `other`'s arc and leg indices are remapped
        // through `other_index_map`, whose every entry is either an existing
        // `places` index or one pushed onto `places` here, and `self`'s are
        // unchanged.
        Ok(Self::new_unchecked(
            places,
            transitions,
            self.left.clone(),
            right,
        ))
    }
}

/// Decoration functor `F : (FinSet, +) → (Set, ×)` whose apex value is a
/// list of Petri-net transitions on that apex.
///
/// Concretely, `F(N) = PetriApex`: the decorations living over an apex
/// of size `|N|` are transition lists whose pre/post arcs reference indices
/// in `{0, …, |N|-1}`, carried alongside `|N|` itself. The empty apex carries
/// the empty transition list, the laxator `combine` concatenates the two
/// lists with every arc place index of the second operand shifted by the
/// first operand's `n`, and `pushforward` applies the apex quotient to every
/// transition's arc endpoints via [`Transition::relabel`].
///
/// Used with [`DecoratedCospan`] this exhibits `PetriNet<Lambda>` as an
/// instance of the generic decorated-cospan construction (Fong–Spivak
/// Thm 6.77), with the cospan's middle set playing the role of the Petri
/// net's place set and the decoration carrying the transitions.
///
/// This is a zero-sized marker type; `Lambda` is tracked at the type level
/// only so that a `PetriDecoration<char>` and a `PetriDecoration<u32>` are
/// distinct decoration functors.
#[derive(Debug)]
pub struct PetriDecoration<Lambda: Sized + Eq + Copy + Debug>(std::marker::PhantomData<Lambda>);

/// The apex value of [`PetriDecoration`]: a transition list together with the
/// number of apex places its arc indices range over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PetriApex {
    /// The apex cardinality; the arc place indices in `transitions` lie in
    /// `0..n`.
    pub n: usize,
    /// The transitions carried over that apex.
    pub transitions: Vec<Transition>,
}

impl<Lambda> Decoration for PetriDecoration<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug + 'static,
{
    type Apex = PetriApex;

    fn empty(n: usize) -> Self::Apex {
        PetriApex {
            n,
            transitions: Vec::new(),
        }
    }

    fn combine(a: Self::Apex, b: Self::Apex) -> Self::Apex {
        let shift = a.n;
        let mut transitions = a.transitions;
        transitions.extend(b.transitions.into_iter().map(|t| Transition {
            pre: t.pre.iter().map(|&(p, w)| (p + shift, w)).collect(),
            post: t.post.iter().map(|&(p, w)| (p + shift, w)).collect(),
        }));
        PetriApex {
            n: a.n + b.n,
            transitions,
        }
    }

    fn pushforward(d: Self::Apex, quotient: &[usize]) -> Self::Apex {
        PetriApex {
            n: quotient.iter().copied().max().map_or(0, |m| m + 1),
            transitions: d
                .transitions
                .into_iter()
                .map(|t| t.relabel(quotient))
                .collect(),
        }
    }
}

impl<Lambda> PetriNet<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug + 'static,
{
    /// Expose this Petri net as a decorated cospan whose apex is its place
    /// set, whose legs are its boundary legs, and whose decoration is its
    /// transition list.
    ///
    /// The underlying cospan is `Cospan::new_unchecked(left, right, places)`.
    /// No information is quotiented or projected, and
    /// [`PetriNet::from_decorated_cospan`] is its exact inverse — places,
    /// both legs, and transition order.
    #[must_use]
    pub fn to_decorated_cospan(&self) -> DecoratedCospan<Lambda, PetriDecoration<Lambda>> {
        // Correct by construction: the legs and places are this net's own, and
        // `PetriNet::new` refuses a net whose legs do not index its places.
        DecoratedCospan::new(
            Cospan::new_unchecked(self.left.clone(), self.right.clone(), self.places.clone()),
            PetriApex {
                n: self.places.len(),
                transitions: self.transitions.clone(),
            },
        )
    }

    /// Rebuild a Petri net from a decorated cospan.
    ///
    /// The cospan's middle set becomes the places, its `left_to_middle` /
    /// `right_to_middle` become the boundary legs verbatim, and the decoration
    /// list becomes the transitions verbatim. When `dec` was produced by
    /// [`PetriNet::to_decorated_cospan`] this is an exact roundtrip; for
    /// other decorated cospans with a `PetriDecoration` it is still
    /// well-defined as long as every transition's arc indices lie within
    /// `dec.cospan.middle().len()` (the [`Transition`] constructor does
    /// not check this — callers are responsible for that invariant). The
    /// decoration's `n` is discarded; the place count is the cospan's middle
    /// length.
    #[must_use]
    pub fn from_decorated_cospan(dec: DecoratedCospan<Lambda, PetriDecoration<Lambda>>) -> Self {
        Self {
            places: dec.cospan.middle().to_vec(),
            left: dec.cospan.left_to_middle().to_vec(),
            right: dec.cospan.right_to_middle().to_vec(),
            transitions: dec.decoration.transitions,
        }
    }
}

// ---------------------------------------------------------------------------
// Category-theoretic trait impls — Thm 6.77 specialized to `PetriDecoration`
// ---------------------------------------------------------------------------
//
// The four trait impls below (`HasIdentity`, `Monoidal`, `Composable`,
// `SymmetricMonoidalMorphism`) exhibit `PetriNet<Lambda>` as a hypergraph
// category in the sense of Fong–Spivak (Def 6.60). All four supertraits are
// required by the `HypergraphCategory<Lambda>` blanket bounds.
//
// The boundary is declared, not reconstructed: `Composable::domain` /
// `Composable::codomain` read `left` / `right` through `places`, so they
// reproduce `Cospan::domain` / `Cospan::codomain` of the cospan
// `PetriNet::to_decorated_cospan` exposes. `Composable::compose` is that
// bridge — `Cospan::compose_with_quotient` followed by
// `PetriDecoration::pushforward` — and `Monoidal::monoidal` is
// `PetriNet::parallel`, which concatenates places, transitions and both legs
// with `other`'s indices shifted by `self.place_count()`.

impl<Lambda> HasIdentity<Vec<Lambda>> for PetriNet<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug,
{
    /// Identity morphism on a tensor word.
    ///
    /// Delegates to [`Cospan::identity`] and wraps the result with
    /// [`PetriNet::from_cospan`]. The resulting net has one place per entry
    /// of `obj`, both legs the identity on those places, and a single
    /// transition whose pre- and post-arcs each have weight 1 at every place —
    /// the "pure relay" that fires unchanged.
    fn identity(obj: &Vec<Lambda>) -> Self {
        PetriNet::from_cospan(&Cospan::identity(obj))
    }
}

impl<Lambda> Monoidal for PetriNet<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug,
{
    /// Tensor product: disjoint union of places, transitions and boundary legs.
    ///
    /// Delegates to [`PetriNet::parallel`], which shifts `other`'s place
    /// indices by `self.place_count()` and concatenates the transition lists
    /// and both legs.
    fn monoidal(&mut self, other: Self) {
        *self = self.parallel(&other);
    }
}

impl<Lambda> Composable<Vec<Lambda>> for PetriNet<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug + 'static,
{
    /// Composition through the decorated-cospan bridge: the pushout of the two
    /// nets' cospans, with the concatenated transition lists pushed forward
    /// along the coequalizer quotient.
    ///
    /// The composite's domain is `self.domain()` and its codomain is
    /// `other.codomain()`.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] if `self.codomain()` does not
    /// type-match `other.domain()`, forwarded from
    /// [`Cospan::compose_with_quotient`].
    fn compose(&self, other: &Self) -> Result<Self, CatgraphError> {
        Ok(Self::from_decorated_cospan(
            self.to_decorated_cospan()
                .compose(&other.to_decorated_cospan())?,
        ))
    }

    /// The domain leg read through the places: `places[left[i]]` at slot `i`.
    fn domain(&self) -> Vec<Lambda> {
        self.left.iter().map(|&p| self.places[p]).collect()
    }

    /// The codomain leg read through the places: `places[right[k]]` at slot `k`.
    fn codomain(&self) -> Vec<Lambda> {
        self.right.iter().map(|&p| self.places[p]).collect()
    }
}

impl<Lambda> SymmetricMonoidalMorphism<Lambda> for PetriNet<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug + 'static,
{
    /// Braiding on a [`PetriNet`] boundary: permutes the stored leg of the
    /// side `of_codomain` names.
    ///
    /// The wire at slot `i` of that side moves to slot `p.apply(i)`, which on
    /// the leg vector is `in_place_permute(leg, &p.inv())`. Places,
    /// transitions and arc contents are unchanged, as is the other leg.
    ///
    /// If `p.len()` does not equal the permuted leg's length, the call is a
    /// no-op — the signature is non-fallible, and this is the defensive arm
    /// the trait's `# Length` section names.
    fn permute_side(&mut self, p: &Permutation, of_codomain: bool) {
        let leg = if of_codomain {
            &mut self.right
        } else {
            &mut self.left
        };
        if p.len() != leg.len() {
            return;
        }
        in_place_permute(leg, &p.inv());
    }

    /// Construct a pure-braiding `PetriNet` from a permutation, with `types`
    /// labelling the domain.
    ///
    /// Delegates to [`DecoratedCospan`]'s constructor with the empty
    /// [`PetriDecoration`] and rebuilds via [`PetriNet::from_decorated_cospan`],
    /// which retains both legs — so `domain() == types` and
    /// `codomain()[k] == types[p.inv().apply(k)]`, on a net with no
    /// transitions.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError`] if the permutation size does not match
    /// `types.len()` (forwarded from [`Cospan`]).
    fn from_permutation_on_domain(p: Permutation, types: &[Lambda]) -> Result<Self, CatgraphError> {
        let dec =
            <DecoratedCospan<Lambda, PetriDecoration<Lambda>> as SymmetricMonoidalMorphism<
                Lambda,
            >>::from_permutation_on_domain(p, types)?;
        Ok(PetriNet::from_decorated_cospan(dec))
    }

    /// As [`from_permutation_on_domain`](SymmetricMonoidalMorphism::from_permutation_on_domain),
    /// with `types` labelling the codomain instead.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError`] if the permutation size does not match
    /// `types.len()` (forwarded from [`Cospan`]).
    fn from_permutation_on_codomain(
        p: Permutation,
        types: &[Lambda],
    ) -> Result<Self, CatgraphError> {
        let dec =
            <DecoratedCospan<Lambda, PetriDecoration<Lambda>> as SymmetricMonoidalMorphism<
                Lambda,
            >>::from_permutation_on_codomain(p, types)?;
        Ok(PetriNet::from_decorated_cospan(dec))
    }
}

/// Hypergraph-category structure on `PetriNet` (Fong–Spivak Thm 6.77 specialized
/// to the [`PetriDecoration`] functor).
///
/// Each Frobenius generator delegates to [`Cospan`]'s corresponding generator
/// and wraps the result with [`PetriNet::from_cospan`]. The resulting nets are
/// single-transition: the cospan's boundary multiplicities land in the
/// transition's pre/post arc weights and the apex set becomes the place set.
impl<Lambda> HypergraphCategory<Lambda> for PetriNet<Lambda>
where
    Lambda: Sized + Eq + Copy + Debug + 'static,
{
    fn unit(z: Lambda) -> Self {
        PetriNet::from_cospan(&Cospan::unit(z))
    }

    fn counit(z: Lambda) -> Self {
        PetriNet::from_cospan(&Cospan::counit(z))
    }

    fn multiplication(z: Lambda) -> Self {
        PetriNet::from_cospan(&Cospan::multiplication(z))
    }

    fn comultiplication(z: Lambda) -> Self {
        PetriNet::from_cospan(&Cospan::comultiplication(z))
    }

    fn cup(z: Lambda) -> Result<Self, CatgraphError> {
        Ok(PetriNet::from_cospan(&Cospan::cup(z)?))
    }

    fn cap(z: Lambda) -> Result<Self, CatgraphError> {
        Ok(PetriNet::from_cospan(&Cospan::cap(z)?))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rust_decimal::Decimal;

    /// Shorthand for `Decimal::from(n)`.
    fn d(n: i64) -> Decimal {
        Decimal::from(n)
    }

    #[test]
    fn marking_new_is_empty() {
        let m = Marking::new();
        assert_eq!(m.get(0), Decimal::ZERO);
        assert!(m.tokens().is_empty());
    }

    #[test]
    fn marking_from_vec_filters_zeros() {
        let m = Marking::from_vec(vec![(0, d(3)), (1, d(0)), (2, d(1))]);
        assert_eq!(m.get(0), d(3));
        assert_eq!(m.get(1), Decimal::ZERO);
        assert_eq!(m.get(2), d(1));
        assert_eq!(m.tokens().len(), 2);
    }

    #[test]
    fn marking_set_and_get() {
        let mut m = Marking::new();
        m.set(0, d(5));
        assert_eq!(m.get(0), d(5));
        m.set(0, Decimal::ZERO);
        assert_eq!(m.get(0), Decimal::ZERO);
        assert!(m.tokens().is_empty());
    }

    #[test]
    fn petri_net_construction() {
        let net: PetriNet<char> = PetriNet::new(
            vec!['H', 'O', 'W'],
            vec![Transition::new(vec![(0, d(2)), (1, d(1))], vec![(2, d(2))])],
            vec![0, 0, 1],
            vec![2, 2],
        )
        .unwrap();
        assert_eq!(net.place_count(), 3);
        assert_eq!(net.transition_count(), 1);
        assert_eq!(net.left_to_place(), &[0, 0, 1]);
        assert_eq!(net.right_to_place(), &[2, 2]);
        assert_eq!(net.domain(), vec!['H', 'H', 'O']);
        assert_eq!(net.codomain(), vec!['W', 'W']);
    }

    /// `PetriNet::new` refuses a net whose arcs do not index its own places —
    /// the property [`PetriNet::transition_as_cospan`] relies on.
    #[test]
    fn petri_net_new_rejects_arcs_outside_its_places() {
        // One place, but the transition's pre-arc names place 5.
        let err = PetriNet::new(
            vec!['a'],
            vec![Transition::new(vec![(5, d(1))], vec![])],
            vec![],
            vec![],
        )
        .expect_err("arc 5 is out of range for a 1-place net");
        assert!(
            matches!(&err, CatgraphError::PetriNet { message }
                if message.contains("transition 0") && message.contains("pre")
                    && message.contains("place 5") && message.contains("1 place")),
            "{err}"
        );

        // The post side is checked too, and the transition index is reported.
        let err = PetriNet::new(
            vec!['a'],
            vec![
                Transition::new(vec![(0, d(1))], vec![]),
                Transition::new(vec![], vec![(2, d(1))]),
            ],
            vec![],
            vec![],
        )
        .expect_err("arc 2 is out of range for a 1-place net");
        assert!(
            matches!(&err, CatgraphError::PetriNet { message }
                if message.contains("transition 1") && message.contains("post")
                    && message.contains("place 2")),
            "{err}"
        );

        // Widening the net so the arcs land in range is accepted — the
        // transitions themselves were never the problem.
        assert!(
            PetriNet::new(
                vec!['a', 'b', 'c'],
                vec![Transition::new(vec![(2, d(1))], vec![])],
                vec![],
                vec![],
            )
            .is_ok()
        );

        // `new_unchecked` still accepts what `new` refuses: that is its contract,
        // and it is what `transition_as_cospan`'s `# Panics` note now describes.
        let malformed = PetriNet::new_unchecked(
            vec!['a'],
            vec![Transition::new(vec![(5, d(1))], vec![])],
            vec![],
            vec![],
        );
        assert_eq!(malformed.place_count(), 1);
    }

    /// `PetriNet::new` refuses a boundary leg that does not index its places,
    /// naming the leg, the position and the target — the property
    /// [`Composable::domain`] / [`Composable::codomain`] rely on when they
    /// index `places` by leg entry.
    ///
    /// **What this ranges over.** A 1-place net, one out-of-range entry per
    /// leg, the arcs-before-legs ordering, and the `left`-before-`right`
    /// ordering. It does not sweep leg lengths.
    #[test]
    fn petri_net_new_rejects_legs_outside_its_places() {
        let err = PetriNet::new(vec!['a'], vec![], vec![0, 3], vec![])
            .expect_err("domain leg entry 3 is out of range for a 1-place net");
        assert!(
            matches!(
                err,
                CatgraphError::ConstructionIndexOutOfBounds {
                    leg: BoundaryLeg::Domain,
                    position: 1,
                    target: 3,
                    target_len: 1,
                }
            ),
            "expected domain/1/3/1, got: {err:?}"
        );

        let err = PetriNet::new(vec!['a'], vec![], vec![0], vec![0, 0, 2])
            .expect_err("codomain leg entry 2 is out of range for a 1-place net");
        assert!(
            matches!(
                err,
                CatgraphError::ConstructionIndexOutOfBounds {
                    leg: BoundaryLeg::Codomain,
                    position: 2,
                    target: 2,
                    target_len: 1,
                }
            ),
            "expected codomain/2/2/1, got: {err:?}"
        );

        // `left` is scanned before `right`: a net faulty on both reports the
        // domain leg.
        let err = PetriNet::new(vec!['a'], vec![], vec![3], vec![3])
            .expect_err("both leg entries are out of range for a 1-place net");
        assert!(
            matches!(
                err,
                CatgraphError::ConstructionIndexOutOfBounds {
                    leg: BoundaryLeg::Domain,
                    position: 0,
                    target: 3,
                    target_len: 1,
                }
            ),
            "left is scanned before right, expected domain/0/3/1, got: {err:?}"
        );

        // Arcs are scanned before legs: a net faulty on both reports the arc.
        let err = PetriNet::new(
            vec!['a'],
            vec![Transition::new(vec![(4, d(1))], vec![])],
            vec![7],
            vec![],
        )
        .expect_err("both the arc and the leg are out of range");
        assert!(
            matches!(&err, CatgraphError::PetriNet { message } if message.contains("place 4")),
            "arcs are scanned before legs, got: {err:?}"
        );

        // In-range legs are accepted, and `new_unchecked` still takes what
        // `new` refuses.
        assert!(PetriNet::<char>::new(vec!['a', 'b'], vec![], vec![1, 0], vec![0]).is_ok());
        let malformed = PetriNet::new_unchecked(vec!['a'], vec![], vec![9], vec![]);
        assert_eq!(malformed.left_to_place(), &[9]);
    }

    #[test]
    fn transition_accessors() {
        let t = Transition::new(vec![(0, d(1)), (1, d(2))], vec![(2, d(3))]);
        assert_eq!(t.pre(), &[(0, d(1)), (1, d(2))]);
        assert_eq!(t.post(), &[(2, d(3))]);
    }

    // Helper: 2H2 + O2 -> 2H2O, opened on ['H','H','O'] → ['W','W'].
    fn combustion_net() -> PetriNet<char> {
        PetriNet::new(
            vec!['H', 'O', 'W'],
            vec![Transition::new(vec![(0, d(2)), (1, d(1))], vec![(2, d(2))])],
            vec![0, 0, 1],
            vec![2, 2],
        )
        .unwrap()
    }

    #[test]
    fn enabled_sufficient_tokens() {
        let net = combustion_net();
        let m = Marking::from_vec(vec![(0, d(4)), (1, d(2))]);
        assert_eq!(net.enabled(&m), vec![0]);
    }

    #[test]
    fn enabled_insufficient_tokens() {
        let net = combustion_net();
        let m = Marking::from_vec(vec![(0, d(1)), (1, d(2))]);
        assert!(net.enabled(&m).is_empty());
    }

    #[test]
    fn fire_success() {
        let net = combustion_net();
        let m = Marking::from_vec(vec![(0, d(2)), (1, d(1))]);
        let result = net.fire(0, &m).unwrap();
        assert_eq!(result.get(0), Decimal::ZERO);
        assert_eq!(result.get(1), Decimal::ZERO);
        assert_eq!(result.get(2), d(2));
    }

    #[test]
    fn fire_not_enabled() {
        let net = combustion_net();
        let m = Marking::from_vec(vec![(0, d(1))]);
        assert!(matches!(
            net.fire(0, &m).unwrap_err(),
            CatgraphError::PetriNet { .. }
        ));
    }

    #[test]
    fn fire_out_of_bounds() {
        let net = combustion_net();
        let m = Marking::new();
        assert!(matches!(
            net.fire(5, &m).unwrap_err(),
            CatgraphError::PetriNet { .. }
        ));
    }

    #[test]
    fn fire_preserves_other_places() {
        let net = combustion_net();
        let m = Marking::from_vec(vec![(0, d(4)), (1, d(2)), (2, d(3))]);
        let result = net.fire(0, &m).unwrap();
        assert_eq!(result.get(0), d(2));
        assert_eq!(result.get(1), d(1));
        assert_eq!(result.get(2), d(5));
    }

    #[test]
    fn arc_weight_pre_existing() {
        let net = combustion_net();
        assert_eq!(net.arc_weight_pre(0, 0), d(2));
        assert_eq!(net.arc_weight_pre(1, 0), d(1));
    }

    #[test]
    fn arc_weight_pre_missing() {
        let net = combustion_net();
        assert_eq!(net.arc_weight_pre(2, 0), Decimal::ZERO);
    }

    #[test]
    fn arc_weight_post_existing() {
        let net = combustion_net();
        assert_eq!(net.arc_weight_post(2, 0), d(2));
    }

    #[test]
    fn arc_weight_post_missing() {
        let net = combustion_net();
        assert_eq!(net.arc_weight_post(0, 0), Decimal::ZERO);
    }

    #[test]
    fn source_places_combustion() {
        let net = combustion_net();
        let sources = net.source_places();
        assert!(sources.contains(&0));
        assert!(sources.contains(&1));
        assert!(!sources.contains(&2));
    }

    #[test]
    fn sink_places_combustion() {
        let net = combustion_net();
        let sinks = net.sink_places();
        assert!(sinks.contains(&2));
        assert!(!sinks.contains(&0));
    }

    #[test]
    fn reachable_single_step() {
        let net = combustion_net();
        let m0 = Marking::from_vec(vec![(0, d(2)), (1, d(1))]);
        let reachable = net.reachable(&m0, 1);
        assert_eq!(reachable.len(), 2);
        assert!(reachable.contains(&m0));
        assert!(reachable.contains(&Marking::from_vec(vec![(2, d(2))])));
    }

    #[test]
    fn reachable_no_enabled() {
        let net = combustion_net();
        let m0 = Marking::new();
        let reachable = net.reachable(&m0, 10);
        assert_eq!(reachable.len(), 1);
    }

    #[test]
    fn can_reach_true() {
        let net = combustion_net();
        let m0 = Marking::from_vec(vec![(0, d(2)), (1, d(1))]);
        let target = Marking::from_vec(vec![(2, d(2))]);
        assert!(net.can_reach(&m0, &target, 5));
    }

    #[test]
    fn can_reach_false() {
        let net = combustion_net();
        let m0 = Marking::from_vec(vec![(0, d(2)), (1, d(1))]);
        let target = Marking::from_vec(vec![(2, d(99))]);
        assert!(!net.can_reach(&m0, &target, 10));
    }

    #[test]
    fn reachable_multi_step() {
        let net = combustion_net();
        let m0 = Marking::from_vec(vec![(0, d(4)), (1, d(2))]);
        let reachable = net.reachable(&m0, 3);
        assert_eq!(reachable.len(), 3);
        assert!(reachable.contains(&Marking::from_vec(vec![(2, d(4))])));
    }

    #[test]
    fn from_cospan_single_transition() {
        let cospan: Cospan<char> =
            Cospan::new(vec![0, 1, 1, 1], vec![2, 2], vec!['N', 'H', 'A']).unwrap();
        let net = PetriNet::from_cospan(&cospan);
        assert_eq!(net.place_count(), 3);
        assert_eq!(net.transition_count(), 1);
        assert_eq!(net.arc_weight_pre(0, 0), d(1));
        assert_eq!(net.arc_weight_pre(1, 0), d(3));
        assert_eq!(net.arc_weight_post(2, 0), d(2));
    }

    /// `from_cospan` emits `pre` and `post` sorted ascending by place index.
    ///
    /// **What this ranges over.** One cospan over five middle places whose
    /// left leg `[4, 3, 2, 1, 0, 0]` and right leg `[3, 0, 4, 1, 2, 2]` each
    /// visit those places out of ascending order, on `Lambda = char`. It does
    /// not sweep leg shapes or middle sizes.
    #[test]
    fn from_cospan_sorts_arcs_by_place_index() {
        let cospan: Cospan<char> = Cospan::new(
            vec![4, 3, 2, 1, 0, 0],
            vec![3, 0, 4, 1, 2, 2],
            vec!['a', 'b', 'c', 'd', 'e'],
        )
        .unwrap();
        let net = PetriNet::from_cospan(&cospan);
        let pre = net.transitions()[0].pre();
        assert_eq!(
            pre,
            &[(0, d(2)), (1, d(1)), (2, d(1)), (3, d(1)), (4, d(1))],
            "left leg [4, 3, 2, 1, 0, 0] must aggregate to pre \
             [(0, 2), (1, 1), (2, 1), (3, 1), (4, 1)], got: {pre:?}"
        );
        let post = net.transitions()[0].post();
        assert_eq!(
            post,
            &[(0, d(1)), (1, d(1)), (2, d(2)), (3, d(1)), (4, d(1))],
            "right leg [3, 0, 4, 1, 2, 2] must aggregate to post \
             [(0, 1), (1, 1), (2, 2), (3, 1), (4, 1)], got: {post:?}"
        );
    }

    #[test]
    fn transition_as_cospan_roundtrip() {
        let net = combustion_net();
        let cospan = net.transition_as_cospan(0);
        let roundtrip = PetriNet::from_cospan(&cospan);
        assert_eq!(roundtrip.place_count(), net.place_count());
        assert_eq!(roundtrip.arc_weight_pre(0, 0), net.arc_weight_pre(0, 0));
        assert_eq!(roundtrip.arc_weight_pre(1, 0), net.arc_weight_pre(1, 0));
        assert_eq!(roundtrip.arc_weight_post(2, 0), net.arc_weight_post(2, 0));
    }

    #[test]
    fn parallel_composition() {
        let a: PetriNet<char> = PetriNet::new(
            vec!['a', 'b'],
            vec![Transition::new(vec![(0, d(1))], vec![(1, d(1))])],
            vec![0],
            vec![1],
        )
        .unwrap();
        let b: PetriNet<char> = PetriNet::new(
            vec!['c', 'd'],
            vec![Transition::new(vec![(0, d(1))], vec![(1, d(1))])],
            vec![0],
            vec![1],
        )
        .unwrap();
        let combined = a.parallel(&b);
        assert_eq!(combined.place_count(), 4);
        assert_eq!(combined.transition_count(), 2);
        assert_eq!(combined.arc_weight_pre(2, 1), d(1));
        assert_eq!(combined.arc_weight_post(3, 1), d(1));
        // Both legs concatenate, `b`'s shifted by `a.place_count()`.
        assert_eq!(combined.left_to_place(), &[0, 2]);
        assert_eq!(combined.right_to_place(), &[1, 3]);
        assert_eq!(combined.domain(), vec!['a', 'c']);
        assert_eq!(combined.codomain(), vec!['b', 'd']);
    }

    #[test]
    fn sequential_composition() {
        let a: PetriNet<char> = PetriNet::new(
            vec!['a', 'b'],
            vec![Transition::new(vec![(0, d(1))], vec![(1, d(1))])],
            vec![0],
            vec![1],
        )
        .unwrap();
        let b: PetriNet<char> = PetriNet::new(
            vec!['b', 'c'],
            vec![Transition::new(vec![(0, d(1))], vec![(1, d(1))])],
            vec![0],
            vec![1],
        )
        .unwrap();
        let composed = a.sequential(&b).unwrap();
        assert_eq!(composed.place_count(), 3);
        assert_eq!(composed.transition_count(), 2);
        // `sequential` keeps `a`'s domain leg and remaps `b`'s codomain leg
        // through the place merge.
        assert_eq!(composed.domain(), vec!['a']);
        assert_eq!(composed.codomain(), vec!['c']);
    }

    #[test]
    fn petri_net_as_decorated_cospan_roundtrip() {
        let pn = combustion_net();
        let t_count = pn.transition_count();
        let p_count = pn.place_count();

        let dec = pn.to_decorated_cospan();
        // Apex (middle set) matches the place set exactly, and both legs are
        // the net's own — not the identity on the apex.
        assert_eq!(dec.cospan.middle().len(), p_count);
        assert_eq!(dec.cospan.left_to_middle(), pn.left_to_place());
        assert_eq!(dec.cospan.right_to_middle(), pn.right_to_place());
        // Decoration carries the full transition list, over an apex sized to
        // the place set.
        assert_eq!(dec.decoration.transitions.len(), t_count);
        assert_eq!(dec.decoration.n, p_count);

        let pn2: PetriNet<char> = PetriNet::from_decorated_cospan(dec);
        assert_eq!(pn2.transition_count(), t_count);
        assert_eq!(pn2.place_count(), p_count);
        assert_eq!(pn2.places(), pn.places());
        assert_eq!(pn2.left_to_place(), pn.left_to_place());
        assert_eq!(pn2.right_to_place(), pn.right_to_place());
        assert_eq!(pn2.transitions(), pn.transitions());
        // Arc weights survive the roundtrip byte-for-byte.
        assert_eq!(pn2.arc_weight_pre(0, 0), pn.arc_weight_pre(0, 0));
        assert_eq!(pn2.arc_weight_pre(1, 0), pn.arc_weight_pre(1, 0));
        assert_eq!(pn2.arc_weight_post(2, 0), pn.arc_weight_post(2, 0));
    }

    #[test]
    fn transition_relabel_maps_arc_indices() {
        // Identify places 0 and 2 onto 0; place 1 stays as 1.
        let t = Transition::new(vec![(0, d(2)), (1, d(1))], vec![(2, d(3))]);
        let relabelled = t.relabel(&[0, 1, 0]);
        assert_eq!(relabelled.pre(), &[(0, d(2)), (1, d(1))]);
        assert_eq!(relabelled.post(), &[(0, d(3))]);
    }

    #[test]
    fn petri_decoration_pushforward_relabels_transitions() {
        use crate::decorated_cospan::Decoration;
        let transitions = vec![
            Transition::new(vec![(0, d(1))], vec![(1, d(1))]),
            Transition::new(vec![(2, d(2))], vec![(0, d(1))]),
        ];
        // Quotient: places 0 and 2 merge to 0, place 1 becomes 1.
        let pushed = <PetriDecoration<char> as Decoration>::pushforward(
            PetriApex { n: 3, transitions },
            &[0, 1, 0],
        );
        assert_eq!(pushed.n, 2);
        assert_eq!(pushed.transitions[0].pre(), &[(0, d(1))]);
        assert_eq!(pushed.transitions[0].post(), &[(1, d(1))]);
        assert_eq!(pushed.transitions[1].pre(), &[(0, d(2))]);
        assert_eq!(pushed.transitions[1].post(), &[(0, d(1))]);
    }

    /// `pushforward` reports the quotient's image size, `max + 1`, and 0 on an
    /// empty quotient.
    ///
    /// **What this ranges over.** Two quotients on `PetriDecoration<char>`:
    /// the non-surjective `[0, 1, 1]` over an apex of 3 carrying one
    /// transition, and the empty quotient carrying no transitions. It does not
    /// sweep apex sizes or quotient shapes.
    #[test]
    fn petri_decoration_pushforward_apex_size_is_the_quotient_image() {
        use crate::decorated_cospan::Decoration;
        let pushed = <PetriDecoration<char> as Decoration>::pushforward(
            PetriApex {
                n: 3,
                transitions: vec![Transition::new(vec![(2, d(1))], vec![(0, d(1))])],
            },
            &[0, 1, 1],
        );
        assert_eq!(
            pushed.n, 2,
            "quotient [0, 1, 1] has image {{0, 1}}, so the apex size must be 2, got: {}",
            pushed.n
        );

        let empty = <PetriDecoration<char> as Decoration>::pushforward(
            PetriApex {
                n: 3,
                transitions: Vec::new(),
            },
            &[],
        );
        assert_eq!(
            empty.n, 0,
            "an empty quotient has empty image, so the apex size must be 0, got: {}",
            empty.n
        );
    }

    #[test]
    fn sequential_no_matching_boundary() {
        let a: PetriNet<char> = PetriNet::new(
            vec!['a', 'b'],
            vec![Transition::new(vec![(0, d(1))], vec![(1, d(1))])],
            vec![0],
            vec![1],
        )
        .unwrap();
        let b: PetriNet<char> = PetriNet::new(
            vec!['x', 'y'],
            vec![Transition::new(vec![(0, d(1))], vec![(1, d(1))])],
            vec![0],
            vec![1],
        )
        .unwrap();
        let composed = a.sequential(&b).unwrap();
        assert_eq!(composed.place_count(), 4);
    }

    // ---------------------------------------------------------------------
    // HypergraphCategory impl — Thm 6.77 specialized to PetriDecoration
    // ---------------------------------------------------------------------

    #[test]
    fn petri_net_unit_has_correct_shape() {
        use catgraph::category::Composable;
        use catgraph::hypergraph_category::HypergraphCategory;

        let eta = PetriNet::<char>::unit('p');
        // Unit η: [] → [p]. Domain is empty; codomain is the single place 'p'.
        assert!(eta.domain().is_empty());
        assert_eq!(eta.codomain(), vec!['p']);
        // The apex (place set) is {'p'}; the net has a single transition
        // whose post-arc produces one token at place 0.
        assert_eq!(eta.places(), &['p']);
        assert_eq!(eta.transition_count(), 1);
        assert!(eta.transitions()[0].pre().is_empty());
    }

    #[test]
    fn petri_net_counit_has_correct_shape() {
        use catgraph::category::Composable;
        use catgraph::hypergraph_category::HypergraphCategory;

        let eps = PetriNet::<char>::counit('p');
        // Counit ε: [p] → []. Domain is the single place; codomain is empty.
        assert_eq!(eps.domain(), vec!['p']);
        assert!(eps.codomain().is_empty());
        assert_eq!(eps.places(), &['p']);
        assert_eq!(eps.transition_count(), 1);
        assert!(eps.transitions()[0].post().is_empty());
    }

    #[test]
    fn petri_net_mu_delta_have_correct_shape() {
        use catgraph::category::Composable;
        use catgraph::hypergraph_category::HypergraphCategory;

        // Multiplication μ: [p, p] → [p]. Two input tokens merge to one.
        let mu = PetriNet::<char>::multiplication('p');
        assert_eq!(mu.domain(), vec!['p', 'p']);
        assert_eq!(mu.codomain(), vec!['p']);
        assert_eq!(mu.places(), &['p']);
        assert_eq!(mu.arc_weight_pre(0, 0), d(2));
        assert_eq!(mu.arc_weight_post(0, 0), d(1));

        // Comultiplication δ: [p] → [p, p]. One input token splits into two.
        let delta = PetriNet::<char>::comultiplication('p');
        assert_eq!(delta.domain(), vec!['p']);
        assert_eq!(delta.codomain(), vec!['p', 'p']);
        assert_eq!(delta.places(), &['p']);
        assert_eq!(delta.arc_weight_pre(0, 0), d(1));
        assert_eq!(delta.arc_weight_post(0, 0), d(2));
    }

    #[test]
    fn petri_net_cup_cap_have_correct_shape() {
        use catgraph::category::Composable;
        use catgraph::hypergraph_category::HypergraphCategory;

        // Cup η;δ: [] → [p, p]. Create a pair of tokens from nothing.
        let cup = PetriNet::<char>::cup('p').unwrap();
        assert!(cup.domain().is_empty());
        assert_eq!(cup.codomain(), vec!['p', 'p']);

        // Cap μ;ε: [p, p] → []. Destroy a pair of tokens.
        let cap = PetriNet::<char>::cap('p').unwrap();
        assert_eq!(cap.domain(), vec!['p', 'p']);
        assert!(cap.codomain().is_empty());
    }

    #[test]
    fn petri_net_identity_is_single_relay_transition() {
        use catgraph::category::{Composable, HasIdentity};

        let id: PetriNet<char> = PetriNet::identity(&vec!['a', 'b']);
        // The boundary is the stored identity leg, so the words are ordered,
        // not just the right multiset.
        assert_eq!(id.domain(), vec!['a', 'b']);
        assert_eq!(id.codomain(), vec!['a', 'b']);
        assert_eq!(id.left_to_place(), &[0, 1]);
        assert_eq!(id.right_to_place(), &[0, 1]);
        assert_eq!(id.places(), &['a', 'b']);
        assert_eq!(id.transition_count(), 1);
    }

    #[test]
    fn petri_net_monoidal_concatenates_places() {
        use catgraph::category::Composable;
        use catgraph::hypergraph_category::HypergraphCategory;
        use catgraph::monoidal::Monoidal;

        let mut eta_a = PetriNet::<char>::unit('a');
        let eta_b = PetriNet::<char>::unit('b');
        eta_a.monoidal(eta_b);
        // η_a ⊗ η_b : [] → [a, b]
        assert!(eta_a.domain().is_empty());
        assert_eq!(eta_a.codomain(), vec!['a', 'b']);
        assert_eq!(eta_a.place_count(), 2);
    }
}
