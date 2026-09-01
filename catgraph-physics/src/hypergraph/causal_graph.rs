//! Update events and the causal graph they induce.
//!
//! An update event is one rewrite-rule application: it consumes the hyperedge
//! instances its match selected and produces the instances the rule's
//! right-hand side appended. A [`CausalGraph`] carries one vertex per event and
//! a directed edge `A → B` for every hyperedge instance that `A` produced and
//! `B` consumed.
//!
//! Hyperedge-instance identity is [`EdgeId`]: minted once per instance and
//! carried across rewrites, so an instance stays distinguishable from the
//! positional index that the host graph's edge list gives it.
//!
//! Causal invariance in \[Gor20a\] (`docs/ANCHORS.md`) ranges over the causal
//! graphs induced by different updating orders; [`CausalGraph::compare`] is the
//! comparison it ranges over.

use std::collections::{BTreeSet, HashMap, HashSet};

/// Identity of one hyperedge instance, stable across rewrites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeId(pub usize);

/// Position of one event in a [`CausalGraph`]'s event list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(pub usize);

/// One update event: the hyperedge instances a single rule application
/// consumed and produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CausalEvent {
    /// Index of the applied rule in the evolution's rule list.
    pub rule_index: usize,

    /// Instances removed by the rewrite, in ascending host-graph index order.
    pub consumed: Vec<EdgeId>,

    /// Instances appended by the rewrite, in right-hand-side order.
    pub produced: Vec<EdgeId>,
}

/// Outcome of comparing two causal graphs up to isomorphism.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalComparison {
    /// A bijection of event sets preserving the causal edges in both
    /// directions exists.
    Isomorphic,

    /// No such bijection exists.
    NotIsomorphic,

    /// The search reached [`CausalGraph::MAX_SEARCH_STEPS`] without settling
    /// the question.
    Undecided,
}

/// One vertex per update event, `A → B` when `B` consumed an instance `A`
/// produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CausalGraph {
    events: Vec<CausalEvent>,
    edges: BTreeSet<(usize, usize)>,
}

impl CausalGraph {
    /// Upper bound on candidate assignments tried by [`Self::compare`].
    pub const MAX_SEARCH_STEPS: usize = 200_000;

    /// Builds the causal graph of `events`, read in application order.
    ///
    /// An instance consumed by an event that no earlier event in `events`
    /// produced contributes no edge; such instances are the ones the branch
    /// started from.
    #[must_use]
    pub fn from_events(events: Vec<CausalEvent>) -> Self {
        let mut producer: HashMap<EdgeId, usize> = HashMap::new();
        let mut edges = BTreeSet::new();

        for (index, event) in events.iter().enumerate() {
            for id in &event.consumed {
                if let Some(&source) = producer.get(id)
                    && source != index
                {
                    edges.insert((source, index));
                }
            }
            for id in &event.produced {
                producer.insert(*id, index);
            }
        }

        Self { events, edges }
    }

    /// Returns the number of events.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Returns the number of causal edges.
    #[must_use]
    pub fn causal_edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Returns true when there are no events.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Returns the events in application order.
    #[must_use]
    pub fn events(&self) -> &[CausalEvent] {
        &self.events
    }

    /// Returns the causal edges in ascending `(source, target)` order.
    pub fn causal_edges(&self) -> impl Iterator<Item = (EventId, EventId)> + '_ {
        self.edges
            .iter()
            .map(|&(source, target)| (EventId(source), EventId(target)))
    }

    /// Compares this causal graph with `other` up to isomorphism.
    ///
    /// Event count, causal-edge count, and a colour refinement on the disjoint
    /// union settle the negative cases; a backtracking search over the refined
    /// colour classes settles the rest, and reports
    /// [`CausalComparison::Undecided`] after [`Self::MAX_SEARCH_STEPS`]
    /// candidate assignments. `rule_index` is data on the events and is not
    /// part of the relation.
    #[must_use]
    pub fn compare(&self, other: &Self) -> CausalComparison {
        let n = self.events.len();
        if n != other.events.len() || self.edges.len() != other.edges.len() {
            return CausalComparison::NotIsomorphic;
        }
        if n == 0 {
            return CausalComparison::Isomorphic;
        }

        // Refine on the disjoint union so the two colourings are comparable.
        let mut predecessors = vec![Vec::new(); 2 * n];
        let mut successors = vec![Vec::new(); 2 * n];
        for &(source, target) in &self.edges {
            successors[source].push(target);
            predecessors[target].push(source);
        }
        for &(source, target) in &other.edges {
            successors[n + source].push(n + target);
            predecessors[n + target].push(n + source);
        }
        let colours = refine(2 * n, &predecessors, &successors);

        let mut left: Vec<u64> = colours[..n].to_vec();
        let mut right: Vec<u64> = colours[n..].to_vec();
        left.sort_unstable();
        right.sort_unstable();
        if left != right {
            return CausalComparison::NotIsomorphic;
        }

        let candidates: Vec<Vec<usize>> = (0..n)
            .map(|v| (0..n).filter(|&w| colours[v] == colours[n + w]).collect())
            .collect();
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by_key(|&v| candidates[v].len());

        let left_edges: HashSet<(usize, usize)> = self.edges.iter().copied().collect();
        let right_edges: HashSet<(usize, usize)> = other.edges.iter().copied().collect();

        let mut mapping = vec![usize::MAX; n];
        let mut used = vec![false; n];
        let mut steps = 0usize;

        match backtrack(
            0,
            &order,
            &candidates,
            &left_edges,
            &right_edges,
            &mut mapping,
            &mut used,
            &mut steps,
        ) {
            Some(true) => CausalComparison::Isomorphic,
            Some(false) => CausalComparison::NotIsomorphic,
            None => CausalComparison::Undecided,
        }
    }

    /// Returns true when [`Self::compare`] reports
    /// [`CausalComparison::Isomorphic`].
    ///
    /// [`CausalComparison::Undecided`] returns false.
    #[must_use]
    pub fn is_isomorphic_to(&self, other: &Self) -> bool {
        matches!(self.compare(other), CausalComparison::Isomorphic)
    }
}

/// Colour refinement on a directed graph: each round replaces a vertex's colour
/// by its old colour together with the sorted colour multisets of its
/// predecessors and successors, stopping when the partition stops getting
/// finer.
fn refine(n: usize, predecessors: &[Vec<usize>], successors: &[Vec<usize>]) -> Vec<u64> {
    let mut colours = vec![0u64; n];
    let mut classes = 1usize;

    for _ in 0..n {
        let mut signatures: Vec<(u64, Vec<u64>, Vec<u64>)> = Vec::with_capacity(n);
        for v in 0..n {
            let mut before: Vec<u64> = predecessors[v].iter().map(|&u| colours[u]).collect();
            let mut after: Vec<u64> = successors[v].iter().map(|&u| colours[u]).collect();
            before.sort_unstable();
            after.sort_unstable();
            signatures.push((colours[v], before, after));
        }

        let mut distinct = signatures.clone();
        distinct.sort();
        distinct.dedup();
        if distinct.len() == classes {
            break;
        }

        classes = distinct.len();
        colours = signatures
            .iter()
            .map(|signature| {
                let index = distinct
                    .binary_search(signature)
                    .expect("invariant: every signature is in the deduplicated signature list");
                index as u64
            })
            .collect();
    }

    colours
}

/// Extends a partial event bijection.
///
/// `Some(true)` = a full bijection was found, `Some(false)` = the candidate
/// space was exhausted, `None` = the step budget was reached.
#[allow(clippy::too_many_arguments)]
fn backtrack(
    depth: usize,
    order: &[usize],
    candidates: &[Vec<usize>],
    left_edges: &HashSet<(usize, usize)>,
    right_edges: &HashSet<(usize, usize)>,
    mapping: &mut [usize],
    used: &mut [bool],
    steps: &mut usize,
) -> Option<bool> {
    if depth == order.len() {
        return Some(true);
    }

    let v = order[depth];
    for &w in &candidates[v] {
        *steps += 1;
        if *steps > CausalGraph::MAX_SEARCH_STEPS {
            return None;
        }
        if used[w] {
            continue;
        }

        let consistent = order[..depth].iter().all(|&u| {
            let image = mapping[u];
            left_edges.contains(&(u, v)) == right_edges.contains(&(image, w))
                && left_edges.contains(&(v, u)) == right_edges.contains(&(w, image))
        });
        if !consistent {
            continue;
        }

        mapping[v] = w;
        used[w] = true;
        let deeper = backtrack(
            depth + 1,
            order,
            candidates,
            left_edges,
            right_edges,
            mapping,
            used,
            steps,
        );
        mapping[v] = usize::MAX;
        used[w] = false;

        match deeper {
            Some(true) => return Some(true),
            None => return None,
            Some(false) => {}
        }
    }

    Some(false)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn event(rule_index: usize, consumed: &[usize], produced: &[usize]) -> CausalEvent {
        CausalEvent {
            rule_index,
            consumed: consumed.iter().copied().map(EdgeId).collect(),
            produced: produced.iter().copied().map(EdgeId).collect(),
        }
    }

    #[test]
    fn produced_then_consumed_is_the_only_edge_source() {
        // Event 0 produces 10 and 11; event 1 consumes 10 (dependent) and the
        // pre-existing instance 1 (no producer in this list, so no edge).
        let chain =
            CausalGraph::from_events(vec![event(0, &[0], &[10, 11]), event(0, &[10, 1], &[12])]);
        assert_eq!(chain.event_count(), 2, "two events were supplied");
        assert_eq!(
            chain.causal_edge_count(),
            1,
            "only instance 10 links a producer to a consumer; got edges {:?}",
            chain.causal_edges().collect::<Vec<_>>()
        );
        assert_eq!(
            chain.causal_edges().collect::<Vec<_>>(),
            vec![(EventId(0), EventId(1))],
            "the edge runs producer → consumer"
        );

        // Same two events consuming only pre-existing instances: no edges.
        let antichain =
            CausalGraph::from_events(vec![event(0, &[0], &[10, 11]), event(0, &[1], &[12])]);
        assert_eq!(antichain.event_count(), 2);
        assert_eq!(
            antichain.causal_edge_count(),
            0,
            "neither event consumes an instance the other produced"
        );
    }

    #[test]
    fn comparison_separates_chain_from_antichain_and_ignores_rule_index() {
        let chain_a = CausalGraph::from_events(vec![event(0, &[0], &[10]), event(0, &[10], &[11])]);
        let chain_b = CausalGraph::from_events(vec![event(1, &[5], &[20]), event(1, &[20], &[21])]);
        let antichain =
            CausalGraph::from_events(vec![event(0, &[0], &[10]), event(0, &[1], &[11])]);

        assert_eq!(
            chain_a.compare(&chain_b),
            CausalComparison::Isomorphic,
            "two 2-event chains are isomorphic under relabelled instance ids"
        );
        assert_eq!(
            chain_a.compare(&antichain),
            CausalComparison::NotIsomorphic,
            "a 2-event chain has 1 causal edge, a 2-event antichain has 0"
        );

        // rule_index differs on every event but the relation ignores it.
        let chain_c = CausalGraph::from_events(vec![event(7, &[0], &[10]), event(9, &[10], &[11])]);
        assert!(
            chain_a.is_isomorphic_to(&chain_c),
            "rule_index is not part of the isomorphism relation"
        );
    }

    #[test]
    fn comparison_separates_equal_edge_counts_by_shape() {
        // Both have 3 events and 2 edges: a path 0→1→2 against a fork 0→1, 0→2.
        let path = CausalGraph::from_events(vec![
            event(0, &[0], &[10]),
            event(0, &[10], &[11]),
            event(0, &[11], &[12]),
        ]);
        let fork = CausalGraph::from_events(vec![
            event(0, &[0], &[10, 11]),
            event(0, &[10], &[12]),
            event(0, &[11], &[13]),
        ]);

        assert_eq!(path.event_count(), fork.event_count(), "3 events each");
        assert_eq!(
            path.causal_edge_count(),
            fork.causal_edge_count(),
            "2 causal edges each"
        );
        assert_eq!(
            path.compare(&fork),
            CausalComparison::NotIsomorphic,
            "a path and a fork on the same event/edge counts are not isomorphic"
        );
        assert_eq!(
            path.compare(&path.clone()),
            CausalComparison::Isomorphic,
            "a causal graph is isomorphic to itself"
        );
    }

    #[test]
    fn empty_graphs_are_isomorphic() {
        let empty = CausalGraph::from_events(vec![]);
        assert!(empty.is_empty());
        assert_eq!(empty.event_count(), 0);
        assert_eq!(
            empty.compare(&CausalGraph::default()),
            CausalComparison::Isomorphic
        );
    }
}
