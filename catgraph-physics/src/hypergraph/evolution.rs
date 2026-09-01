//! Hypergraph evolution tracking and causal invariance analysis.
//!
//! This module tracks the history of hypergraph rewrites and provides
//! tools for analyzing causal invariance via Wilson loops.

use super::causal_graph::{CausalEvent, CausalGraph, EdgeId};
use super::hypergraph::Hypergraph;
use super::rewrite_rule::{RewriteMatch, RewriteRule};
use std::collections::{HashMap, HashSet};

/// A single step in the multiway evolution of a hypergraph.
///
/// Records which rule was applied, the match site, the resulting
/// hypergraph state (with fingerprint), and the parent step for
/// tree traversal.
#[derive(Debug, Clone)]
pub struct HypergraphStep {
    /// The rule that was applied.
    pub rule_index: usize,

    /// The match where the rule was applied.
    pub match_info: RewriteMatch,

    /// State of the hypergraph after this step.
    pub state: Hypergraph,

    /// Fingerprint of the state (for fast comparison).
    pub fingerprint: u64,

    /// Step number (0-indexed).
    pub step: usize,

    /// Parent step index (None for initial state).
    pub parent: Option<usize>,

    /// Branch ID (for multiway evolution).
    pub branch_id: usize,
}

/// A node in the hypergraph evolution graph (multiway systems).
///
/// Stores the full hypergraph state at a given depth, with a fingerprint
/// for fast equality checks and optional parent/transition provenance.
#[derive(Debug, Clone)]
pub struct HypergraphNode {
    /// Unique ID for this node.
    pub id: usize,

    /// The hypergraph state at this node.
    pub state: Hypergraph,

    /// Fingerprint for fast comparison.
    pub fingerprint: u64,

    /// Step (depth) in the evolution.
    pub step: usize,

    /// Parent node ID (None for root).
    pub parent: Option<usize>,

    /// Rule and match that led to this state (None for root).
    pub transition: Option<(usize, RewriteMatch)>,
}

/// A Wilson loop in the hypergraph evolution history.
///
/// A closed path in the rewrite history graph, analogous to a Wilson loop
/// in lattice gauge theory. The loop runs from the common ancestor down one
/// branch, across the two branch tips (which carry isomorphic states), and back
/// up the other branch to the ancestor; its holonomy compares the causal graphs
/// the two branches induce.
#[derive(Debug, Clone)]
pub struct WilsonLoop {
    /// Node IDs forming the loop, opening and closing at `base`.
    pub path: Vec<usize>,

    /// Starting/ending node ID: the branches' common ancestor.
    pub base: usize,

    /// `1.0` when the two branches induce isomorphic causal graphs, `0.0`
    /// otherwise — including when either branch has no causal graph and when
    /// the isomorphism search reaches its step budget.
    pub holonomy: f64,

    /// Length of the loop.
    pub length: usize,
}

/// Result of one confluence-witness pass over the explored fragment.
#[derive(Debug, Clone)]
pub struct CausalInvarianceResult {
    /// Whether every analyzed loop has holonomy `1.0`.
    pub is_invariant: bool,

    /// Mean of `1.0 - holonomy` over the analyzed loops.
    pub average_deviation: f64,

    /// Maximum of `1.0 - holonomy` over the analyzed loops.
    pub max_deviation: f64,

    /// Number of Wilson loops analyzed.
    pub loops_analyzed: usize,

    /// Wilson loops with holonomy below `1.0`.
    pub non_trivial_loops: Vec<WilsonLoop>,
}

/// Evolution of a hypergraph under rewrite rules.
///
/// Tracks the history of rewrites and supports both deterministic
/// (single path) and non-deterministic (multiway) evolution.
#[derive(Debug, Clone)]
pub struct HypergraphEvolution {
    /// All nodes in the evolution graph.
    nodes: Vec<HypergraphNode>,

    /// Rules used in this evolution.
    rules: Vec<RewriteRule>,

    /// Map from fingerprint to node IDs (for detecting merges).
    fingerprint_to_nodes: HashMap<u64, Vec<usize>>,

    /// Maximum step reached.
    max_step: usize,

    /// Next vertex ID for new vertices.
    next_vertex_id: usize,

    /// Hyperedge-instance identities of each node's state, positionally
    /// parallel to that state's edge list.
    edge_ids: Vec<Vec<EdgeId>>,

    /// Update event that produced each node (`None` for the root).
    events: Vec<Option<CausalEvent>>,

    /// Next hyperedge-instance identity.
    next_edge_id: usize,
}

impl HypergraphEvolution {
    /// Creates a new evolution starting from the given hypergraph.
    #[must_use]
    pub fn new(initial: Hypergraph, rules: Vec<RewriteRule>) -> Self {
        let fingerprint = initial.fingerprint();
        let max_vertex = initial.vertices().max().unwrap_or(0);
        let root_edge_ids: Vec<EdgeId> = (0..initial.edge_count()).map(EdgeId).collect();

        let root = HypergraphNode {
            id: 0,
            state: initial,
            fingerprint,
            step: 0,
            parent: None,
            transition: None,
        };

        let mut fingerprint_to_nodes = HashMap::new();
        fingerprint_to_nodes.insert(fingerprint, vec![0]);

        Self {
            nodes: vec![root],
            rules,
            fingerprint_to_nodes,
            max_step: 0,
            next_vertex_id: max_vertex + 1,
            next_edge_id: root_edge_ids.len(),
            edge_ids: vec![root_edge_ids],
            events: vec![None],
        }
    }

    /// Runs deterministic evolution for the given number of steps.
    ///
    /// At each step, applies the first matching rule at the first match.
    ///
    /// # Arguments
    ///
    /// * `initial` - Starting hypergraph
    /// * `rules` - Rewrite rules to apply
    /// * `max_steps` - Maximum number of rewrite steps
    ///
    /// # Returns
    ///
    /// An evolution with the deterministic trace.
    #[must_use]
    pub fn run(initial: &Hypergraph, rules: &[RewriteRule], max_steps: usize) -> Self {
        let mut evolution = Self::new(initial.clone(), rules.to_vec());
        let mut current_id = 0;

        for _ in 0..max_steps {
            let node = &evolution.nodes[current_id];
            let state = node.state.clone();

            // Find first applicable rule
            let mut applied = false;
            for (rule_idx, rule) in rules.iter().enumerate() {
                let matches = rule.find_matches(&state);
                if !matches.is_empty() {
                    // Apply first match
                    let new_id = evolution.apply_rule(current_id, rule_idx, &matches[0]);
                    current_id = new_id;
                    applied = true;
                    break;
                }
            }

            if !applied {
                break; // No rules apply
            }
        }

        evolution
    }

    /// Runs multiway (non-deterministic) evolution.
    ///
    /// Explores all possible rule applications up to limits.
    ///
    /// # Arguments
    ///
    /// * `initial` - Starting hypergraph
    /// * `rules` - Rewrite rules to apply
    /// * `max_steps` - Maximum depth
    /// * `max_nodes` - Maximum total nodes to explore
    ///
    /// # Returns
    ///
    /// An evolution with the multiway graph.
    #[must_use]
    pub fn run_multiway(
        initial: &Hypergraph,
        rules: &[RewriteRule],
        max_steps: usize,
        max_nodes: usize,
    ) -> Self {
        let mut evolution = Self::new(initial.clone(), rules.to_vec());
        let mut frontier = vec![0usize]; // Nodes to expand

        while !frontier.is_empty() && evolution.nodes.len() < max_nodes {
            let current_id = frontier.remove(0);
            let node = &evolution.nodes[current_id];

            if node.step >= max_steps {
                continue;
            }

            let state = node.state.clone();

            // Find all applicable rules and matches
            for (rule_idx, rule) in rules.iter().enumerate() {
                let matches = rule.find_matches(&state);
                for match_ in matches {
                    if evolution.nodes.len() >= max_nodes {
                        break;
                    }
                    let new_id = evolution.apply_rule(current_id, rule_idx, &match_);
                    frontier.push(new_id);
                }
            }
        }

        evolution
    }

    /// Applies a rule at a specific node and match.
    ///
    /// # Returns
    ///
    /// The ID of the newly created node.
    fn apply_rule(&mut self, parent_id: usize, rule_idx: usize, match_: &RewriteMatch) -> usize {
        let parent = &self.nodes[parent_id];
        let mut new_state = parent.state.clone();
        let parent_step = parent.step;
        let mut ids = self.edge_ids[parent_id].clone();

        // Apply the rule
        let rule = &self.rules[rule_idx];
        let effect = rule.apply_effect(&mut new_state, match_, &mut self.next_vertex_id);

        // Carry hyperedge-instance identity across the rewrite: the removed
        // slots drop their identities, the appended slots mint fresh ones.
        // `removed_edges` is descending, so each index still addresses the
        // instance it addressed before any removal.
        let mut consumed = Vec::with_capacity(effect.removed_edges.len());
        for &edge_idx in &effect.removed_edges {
            consumed.push(ids.remove(edge_idx));
        }
        consumed.reverse();

        let mut produced = Vec::with_capacity(effect.added_edges.len());
        for _ in &effect.added_edges {
            let id = EdgeId(self.next_edge_id);
            self.next_edge_id += 1;
            ids.push(id);
            produced.push(id);
        }

        let fingerprint = new_state.fingerprint();
        let new_id = self.nodes.len();
        let new_step = parent_step + 1;

        let node = HypergraphNode {
            id: new_id,
            state: new_state,
            fingerprint,
            step: new_step,
            parent: Some(parent_id),
            transition: Some((rule_idx, match_.clone())),
        };

        self.nodes.push(node);
        self.edge_ids.push(ids);
        self.events.push(Some(CausalEvent {
            rule_index: rule_idx,
            consumed,
            produced,
        }));
        self.fingerprint_to_nodes
            .entry(fingerprint)
            .or_default()
            .push(new_id);
        self.max_step = self.max_step.max(new_step);

        new_id
    }

    // ========================================================================
    // Accessors
    // ========================================================================

    /// Returns the number of nodes in the evolution.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the maximum step reached.
    #[must_use]
    pub fn max_step(&self) -> usize {
        self.max_step
    }

    /// Returns a reference to a node by ID.
    #[must_use]
    pub fn get_node(&self, id: usize) -> Option<&HypergraphNode> {
        self.nodes.get(id)
    }

    /// Returns the root (initial) node.
    #[must_use]
    pub fn root(&self) -> &HypergraphNode {
        &self.nodes[0]
    }

    /// Returns all leaf nodes (nodes with no children).
    #[must_use]
    pub fn leaves(&self) -> Vec<usize> {
        let parents: HashSet<_> = self.nodes.iter().filter_map(|n| n.parent).collect();

        (0..self.nodes.len())
            .filter(|id| !parents.contains(id))
            .collect()
    }

    /// Returns nodes at a specific step.
    #[must_use]
    pub fn nodes_at_step(&self, step: usize) -> Vec<usize> {
        self.nodes
            .iter()
            .filter(|n| n.step == step)
            .map(|n| n.id)
            .collect()
    }

    /// Returns the hyperedge-instance identities of a node's state,
    /// positionally parallel to that state's edge list.
    ///
    /// `None` for a node ID outside the evolution.
    #[must_use]
    pub fn edge_identities(&self, node_id: usize) -> Option<&[EdgeId]> {
        self.edge_ids.get(node_id).map(Vec::as_slice)
    }

    /// Returns the update event that produced a node.
    ///
    /// `None` for the root and for a node ID outside the evolution.
    #[must_use]
    pub fn event(&self, node_id: usize) -> Option<&CausalEvent> {
        self.events.get(node_id).and_then(Option::as_ref)
    }

    /// Returns the causal graph of the update events on the path
    /// `ancestor → node_id`, excluding `ancestor`'s own event.
    ///
    /// `None` when either ID is outside the evolution or `ancestor` is not on
    /// `node_id`'s path to the root.
    #[must_use]
    pub fn causal_graph_between(&self, ancestor: usize, node_id: usize) -> Option<CausalGraph> {
        if ancestor >= self.nodes.len() || node_id >= self.nodes.len() {
            return None;
        }

        let mut chain = Vec::new();
        let mut current = node_id;
        while current != ancestor {
            chain.push(current);
            current = self.nodes[current].parent?;
        }
        chain.reverse();

        let events = chain
            .iter()
            .filter_map(|&id| self.events[id].clone())
            .collect();
        Some(CausalGraph::from_events(events))
    }

    /// Returns the causal graph of the update events on the path from the root
    /// to `node_id`.
    ///
    /// `None` for a node ID outside the evolution.
    #[must_use]
    pub fn causal_graph(&self, node_id: usize) -> Option<CausalGraph> {
        self.causal_graph_between(0, node_id)
    }

    /// Finds merge points (nodes with same fingerprint from different parents).
    #[must_use]
    pub fn find_merges(&self) -> Vec<Vec<usize>> {
        self.fingerprint_to_nodes
            .values()
            .filter(|ids| ids.len() > 1)
            .cloned()
            .collect()
    }

    // ========================================================================
    // Causal Invariance Analysis
    // ========================================================================

    /// Finds all Wilson loops (closed paths) in the evolution graph.
    ///
    /// A Wilson loop exists when two different paths from the root
    /// lead to isomorphic hypergraph states.
    #[must_use]
    pub fn find_wilson_loops(&self) -> Vec<WilsonLoop> {
        let mut loops = Vec::new();

        // Find merge points (same fingerprint from different paths)
        for ids in self.fingerprint_to_nodes.values() {
            if ids.len() < 2 {
                continue;
            }

            // For each pair of nodes with same fingerprint
            for i in 0..ids.len() {
                for j in (i + 1)..ids.len() {
                    let id1 = ids[i];
                    let id2 = ids[j];

                    // Check if they're actually isomorphic (not just same fingerprint)
                    let n1 = &self.nodes[id1];
                    let n2 = &self.nodes[id2];

                    if n1.state.is_isomorphic_to(&n2.state) {
                        // Found a Wilson loop
                        let path1 = self.path_to_root(id1);
                        let path2 = self.path_to_root(id2);

                        // Find common ancestor
                        let path1_set: HashSet<_> = path1.iter().copied().collect();
                        let ancestor = path2
                            .iter()
                            .find(|id| path1_set.contains(id))
                            .copied()
                            .unwrap_or(0);

                        // Build the loop path: ancestor → id1, across to id2,
                        // then id2 → ancestor.
                        let mut loop_path = Vec::new();
                        for &id in &path1 {
                            loop_path.push(id);
                            if id == ancestor {
                                break;
                            }
                        }
                        loop_path.reverse();

                        for &id in &path2 {
                            if id == ancestor {
                                break;
                            }
                            loop_path.push(id);
                        }
                        loop_path.push(ancestor);

                        // Compute holonomy
                        let holonomy = self.compute_holonomy(ancestor, id1, id2);

                        loops.push(WilsonLoop {
                            path: loop_path.clone(),
                            base: ancestor,
                            holonomy,
                            length: loop_path.len(),
                        });
                    }
                }
            }
        }

        loops
    }

    /// Returns the path from a node to the root.
    fn path_to_root(&self, node_id: usize) -> Vec<usize> {
        let mut path = vec![node_id];
        let mut current = node_id;

        while let Some(parent) = self.nodes[current].parent {
            path.push(parent);
            current = parent;
        }

        path
    }

    /// Computes the holonomy of the loop based at `ancestor` closing the
    /// branches to `id1` and `id2`.
    ///
    /// `1.0` when the causal graphs the two branches induce are isomorphic,
    /// `0.0` otherwise — including when either branch has no causal graph and
    /// when the isomorphism search reaches its step budget.
    fn compute_holonomy(&self, ancestor: usize, id1: usize, id2: usize) -> f64 {
        let (Some(branch1), Some(branch2)) = (
            self.causal_graph_between(ancestor, id1),
            self.causal_graph_between(ancestor, id2),
        ) else {
            return 0.0;
        };

        if branch1.is_isomorphic_to(&branch2) {
            1.0
        } else {
            0.0
        }
    }

    /// Reports whether any explored pair of isomorphic-state nodes separates
    /// the causal graphs of the two branches reaching them.
    ///
    /// `is_invariant` is true when every such pair's two branch causal graphs
    /// compare isomorphic; a comparison that reaches
    /// [`CausalGraph::MAX_SEARCH_STEPS`] counts as separating.
    /// \[Gor20a\] states causal invariance over the whole multiway system,
    /// which no finite exploration decides; this ranges over the
    /// isomorphic-state pairs `run` / `run_multiway` reached, and a fragment
    /// holding no such pair is true.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn analyze_causal_invariance(&self) -> CausalInvarianceResult {
        let loops = self.find_wilson_loops();

        if loops.is_empty() {
            return CausalInvarianceResult {
                is_invariant: true, // No loops = trivially invariant
                average_deviation: 0.0,
                max_deviation: 0.0,
                loops_analyzed: 0,
                non_trivial_loops: vec![],
            };
        }

        let deviations: Vec<_> = loops.iter().map(|l| 1.0 - l.holonomy).collect();

        let average_deviation = deviations.iter().sum::<f64>() / deviations.len() as f64;
        let max_deviation = deviations.iter().copied().fold(0.0, f64::max);

        let non_trivial_loops: Vec<_> = loops.into_iter().filter(|l| l.holonomy < 1.0).collect();
        let is_invariant = non_trivial_loops.is_empty();

        CausalInvarianceResult {
            is_invariant,
            average_deviation,
            max_deviation,
            loops_analyzed: deviations.len(),
            non_trivial_loops,
        }
    }

    /// Returns [`analyze_causal_invariance`](Self::analyze_causal_invariance)'s
    /// `is_invariant`: true when every explored pair of isomorphic-state nodes
    /// has isomorphic branch causal graphs, a comparison reaching
    /// [`CausalGraph::MAX_SEARCH_STEPS`] counting as separating.
    ///
    /// A reading of the explored fragment, not a verdict of causal
    /// invariance.
    #[must_use]
    pub fn is_causally_invariant(&self) -> bool {
        self.analyze_causal_invariance().is_invariant
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Returns statistics about the evolution.
    #[must_use]
    pub fn statistics(&self) -> EvolutionStatistics {
        let leaves = self.leaves();
        let merges = self.find_merges();

        let branch_count = leaves.len();
        let merge_count = merges.len();

        // Count rule applications
        let mut rule_counts = vec![0; self.rules.len()];
        for node in &self.nodes {
            if let Some((rule_idx, _)) = &node.transition {
                rule_counts[*rule_idx] += 1;
            }
        }

        EvolutionStatistics {
            total_nodes: self.nodes.len(),
            max_step: self.max_step,
            branch_count,
            merge_count,
            rule_applications: rule_counts,
        }
    }
}

/// Statistics about a hypergraph evolution.
#[derive(Debug, Clone)]
pub struct EvolutionStatistics {
    /// Total number of nodes explored.
    pub total_nodes: usize,

    /// Maximum depth reached.
    pub max_step: usize,

    /// Number of distinct branches (leaf nodes).
    pub branch_count: usize,

    /// Number of merge points (confluence).
    pub merge_count: usize,

    /// Number of times each rule was applied.
    pub rule_applications: Vec<usize>,
}

impl std::fmt::Display for EvolutionStatistics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Evolution Statistics:")?;
        writeln!(f, "  Total nodes: {}", self.total_nodes)?;
        writeln!(f, "  Max step: {}", self.max_step)?;
        writeln!(f, "  Branches: {}", self.branch_count)?;
        writeln!(f, "  Merges: {}", self.merge_count)?;
        for (i, count) in self.rule_applications.iter().enumerate() {
            writeln!(f, "  Rule {i}: {count} applications")?;
        }
        Ok(())
    }
}

impl std::fmt::Display for CausalInvarianceResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Causal Invariance Analysis:")?;
        writeln!(
            f,
            "  Causally invariant: {}",
            if self.is_invariant { "YES" } else { "NO" }
        )?;
        writeln!(f, "  Loops analyzed: {}", self.loops_analyzed)?;
        writeln!(f, "  Average deviation: {:.6}", self.average_deviation)?;
        writeln!(f, "  Max deviation: {:.6}", self.max_deviation)?;
        writeln!(f, "  Non-trivial loops: {}", self.non_trivial_loops.len())?;
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::causal_graph::EventId;
    use super::*;

    #[test]
    fn test_evolution_deterministic() {
        let initial = Hypergraph::from_edges(vec![vec![0, 1, 2]]);
        let rules = vec![RewriteRule::wolfram_a_to_bb()];

        let evolution = HypergraphEvolution::run(&initial, &rules, 10);

        assert!(evolution.node_count() >= 2);
        assert_eq!(evolution.root().state.edge_count(), 1);
    }

    #[test]
    fn test_evolution_multiway() {
        let initial = Hypergraph::from_edges(vec![vec![0, 1, 2], vec![1, 2, 3]]);
        let rules = vec![RewriteRule::wolfram_a_to_bb()];

        let evolution = HypergraphEvolution::run_multiway(&initial, &rules, 3, 50);

        // Should have multiple branches
        assert!(evolution.node_count() > 1);
    }

    #[test]
    fn test_evolution_statistics() {
        let initial = Hypergraph::from_edges(vec![vec![0, 1, 2]]);
        let rules = vec![RewriteRule::wolfram_a_to_bb()];

        let evolution = HypergraphEvolution::run(&initial, &rules, 5);
        let stats = evolution.statistics();

        assert!(stats.total_nodes >= 1);
        assert!(!stats.rule_applications.is_empty());
    }

    #[test]
    fn test_causal_invariance_trivial() {
        // Single path evolution is trivially invariant
        let initial = Hypergraph::from_edges(vec![vec![0, 1]]);
        let rules = vec![RewriteRule::edge_split()];

        let evolution = HypergraphEvolution::run(&initial, &rules, 3);
        let result = evolution.analyze_causal_invariance();

        // No branches, so trivially invariant
        assert!(result.is_invariant || result.loops_analyzed == 0);
    }

    #[test]
    fn test_find_merges() {
        let initial = Hypergraph::from_edges(vec![vec![0, 1, 2], vec![2, 3, 4]]);
        let rules = vec![RewriteRule::wolfram_a_to_bb()];

        let evolution = HypergraphEvolution::run_multiway(&initial, &rules, 2, 20);
        let _merges = evolution.find_merges();

        // May or may not have merges depending on the specific evolution
    }

    #[test]
    fn test_path_to_root() {
        let initial = Hypergraph::from_edges(vec![vec![0, 1, 2]]);
        let rules = vec![RewriteRule::wolfram_a_to_bb()];

        let evolution = HypergraphEvolution::run(&initial, &rules, 3);

        // Last node's path should include root
        let last_id = evolution.node_count() - 1;
        let path = evolution.path_to_root(last_id);

        assert!(path.contains(&0)); // Root is in path
        assert_eq!(path[0], last_id); // Starts with the node
        assert_eq!(*path.last().unwrap(), 0); // Ends at root
    }

    /// `{{0,1,2},{1,2,3}}` under `A→BB`, the deterministic trace: instance
    /// identities carried across two rewrites.
    #[test]
    fn hyperedge_instance_identity_survives_rewrites() {
        let initial = Hypergraph::from_edges(vec![vec![0, 1, 2], vec![1, 2, 3]]);
        let rules = vec![RewriteRule::wolfram_a_to_bb()];
        let evolution = HypergraphEvolution::run(&initial, &rules, 2);

        assert_eq!(evolution.node_count(), 3, "root plus two rewrites");

        // The root mints one identity per initial edge.
        assert_eq!(
            evolution.edge_identities(0),
            Some([EdgeId(0), EdgeId(1)].as_slice())
        );

        // Step 1 consumes the first ternary edge; the second edge keeps
        // identity 1 even though its positional index moved from 1 to 0.
        assert_eq!(
            evolution.edge_identities(1),
            Some([EdgeId(1), EdgeId(2), EdgeId(3)].as_slice()),
            "surviving edge keeps EdgeId(1) after the slot shift"
        );
        assert_eq!(
            evolution.event(1),
            Some(&CausalEvent {
                rule_index: 0,
                consumed: vec![EdgeId(0)],
                produced: vec![EdgeId(2), EdgeId(3)],
            })
        );

        // Step 2 consumes the edge the root minted as identity 1.
        assert_eq!(
            evolution.edge_identities(2),
            Some([EdgeId(2), EdgeId(3), EdgeId(4), EdgeId(5)].as_slice())
        );
        assert_eq!(
            evolution.event(2),
            Some(&CausalEvent {
                rule_index: 0,
                consumed: vec![EdgeId(1)],
                produced: vec![EdgeId(4), EdgeId(5)],
            })
        );

        assert_eq!(evolution.event(0), None, "the root has no event");
        assert_eq!(evolution.edge_identities(3), None, "no node 3 in this run");

        // Every identity minted across the multiway run is distinct.
        let multiway = HypergraphEvolution::run_multiway(&initial, &rules, 3, 50);
        let mut minted: Vec<EdgeId> = multiway
            .edge_identities(0)
            .expect("invariant: the root always exists")
            .to_vec();
        for id in 1..multiway.node_count() {
            let event = multiway
                .event(id)
                .expect("invariant: non-root nodes carry an event");
            minted.extend(event.produced.iter().copied());
        }
        let total = minted.len();
        minted.sort_unstable();
        minted.dedup();
        assert_eq!(
            minted.len(),
            total,
            "{total} identities minted over {} nodes, {} distinct",
            multiway.node_count(),
            minted.len()
        );
    }

    /// Branch causal graphs of the two 2-step orders in two fixtures: one whose
    /// updates are independent, one whose second update consumes the first's
    /// output.
    #[test]
    fn branch_causal_graph_links_producer_to_consumer() {
        let independent = HypergraphEvolution::run_multiway(
            &Hypergraph::from_edges(vec![vec![0, 1, 2], vec![1, 2, 3]]),
            &[RewriteRule::wolfram_a_to_bb()],
            3,
            50,
        );
        let branch = independent.causal_graph(3).expect("node 3 exists");
        assert_eq!(branch.event_count(), 2, "two rewrites on the path 0 → 3");
        assert_eq!(
            branch.causal_edge_count(),
            0,
            "neither A→BB application consumes the other's output; got {:?}",
            branch.causal_edges().collect::<Vec<_>>()
        );

        let dependent = HypergraphEvolution::run_multiway(
            &Hypergraph::from_edges(vec![vec![0, 1], vec![1, 2], vec![2, 3]]),
            &[RewriteRule::collapse()],
            3,
            50,
        );
        let chain = dependent.causal_graph(3).expect("node 3 exists");
        assert_eq!(chain.event_count(), 2, "two rewrites on the path 0 → 3");
        assert_eq!(
            chain.causal_edges().collect::<Vec<_>>(),
            vec![(EventId(0), EventId(1))],
            "the second collapse consumes the edge the first produced"
        );
        assert!(
            !branch.is_isomorphic_to(&chain),
            "0 causal edges against 1 is a separation"
        );

        // A node that is not a descendant has no branch causal graph.
        assert_eq!(dependent.causal_graph_between(1, 2), None);
        assert_eq!(dependent.causal_graph_between(0, 99), None);
        assert_eq!(dependent.causal_graph_between(99, 3), None);
    }

    /// `{{0,1,2},{1,2,3}}` under `A→BB` to depth 3: one merge, one Wilson loop,
    /// isomorphic branch causal graphs.
    #[test]
    fn confluent_fixture_closes_its_loop_at_the_ancestor() {
        let initial = Hypergraph::from_edges(vec![vec![0, 1, 2], vec![1, 2, 3]]);
        let evolution =
            HypergraphEvolution::run_multiway(&initial, &[RewriteRule::wolfram_a_to_bb()], 3, 50);

        let stats = evolution.statistics();
        assert_eq!(stats.total_nodes, 5, "root, two step-1 nodes, two step-2");
        assert_eq!(stats.max_step, 2);
        assert_eq!(stats.branch_count, 2);
        assert_eq!(stats.merge_count, 1);
        assert_eq!(stats.rule_applications, vec![4]);
        assert_eq!(evolution.find_merges().len(), 1);

        let loops = evolution.find_wilson_loops();
        assert_eq!(loops.len(), 1, "one merge yields one Wilson loop");
        let wilson = &loops[0];
        assert_eq!(wilson.base, 0, "the branches' common ancestor is the root");
        assert_eq!(
            wilson.path.first(),
            Some(&wilson.base),
            "the loop opens at its base; got path {:?}",
            wilson.path
        );
        assert_eq!(
            wilson.path.last(),
            Some(&wilson.base),
            "the loop closes at its base; got path {:?}",
            wilson.path
        );
        assert_eq!(wilson.length, wilson.path.len());
        assert!(
            (wilson.holonomy - 1.0).abs() < 1e-12,
            "isomorphic branch causal graphs give holonomy 1.0, got {}",
            wilson.holonomy
        );

        let result = evolution.analyze_causal_invariance();
        assert!(result.is_invariant);
        assert_eq!(result.loops_analyzed, 1);
        assert!(result.non_trivial_loops.is_empty());
        assert!(
            result.max_deviation.abs() < 1e-12,
            "expected 0.0, got {}",
            result.max_deviation
        );
        assert!(evolution.is_causally_invariant());
    }

    /// `{{0,1},{1,2},{2,3},{3,4}}` under `collapse` to depth 4: 6 of its 18
    /// loops reach a shared state through non-isomorphic causal graphs.
    #[test]
    fn non_confluent_fixture_separates_branch_causal_graphs() {
        let initial = Hypergraph::from_edges(vec![vec![0, 1], vec![1, 2], vec![2, 3], vec![3, 4]]);
        let evolution =
            HypergraphEvolution::run_multiway(&initial, &[RewriteRule::collapse()], 4, 200);

        let stats = evolution.statistics();
        assert_eq!(stats.total_nodes, 16);
        assert_eq!(stats.max_step, 3);
        assert_eq!(stats.branch_count, 6);
        assert_eq!(stats.merge_count, 4);
        assert_eq!(stats.rule_applications, vec![15]);
        assert_eq!(evolution.find_merges().len(), 4);

        let loops = evolution.find_wilson_loops();
        assert_eq!(loops.len(), 18, "18 isomorphic-state pairs");

        let separating = loops.iter().filter(|l| l.holonomy < 1.0).count();
        let closing = loops
            .iter()
            .filter(|l| (l.holonomy - 1.0).abs() < 1e-12)
            .count();
        let zero = loops.iter().filter(|l| l.holonomy.abs() < 1e-12).count();
        assert_eq!(
            (separating, closing, zero),
            (6, 12, 6),
            "expected (6 separating, 12 closing, 6 exactly zero); holonomies {:?}",
            loops.iter().map(|l| l.holonomy).collect::<Vec<_>>()
        );

        for wilson in &loops {
            assert_eq!(
                wilson.path.first(),
                Some(&wilson.base),
                "loop opens at its base"
            );
            assert_eq!(
                wilson.path.last(),
                Some(&wilson.base),
                "loop closes at its base"
            );
        }

        let result = evolution.analyze_causal_invariance();
        assert!(
            !result.is_invariant,
            "6 separating loops must sink the witness"
        );
        assert!(!evolution.is_causally_invariant());
        assert_eq!(result.loops_analyzed, 18);
        assert_eq!(result.non_trivial_loops.len(), 6);
        assert!(
            (result.max_deviation - 1.0).abs() < 1e-12,
            "expected 1.0, got {}",
            result.max_deviation
        );
        assert!(
            (result.average_deviation - 6.0 / 18.0).abs() < 1e-12,
            "expected 6/18, got {}",
            result.average_deviation
        );
    }

    #[test]
    fn test_nodes_at_step() {
        let initial = Hypergraph::from_edges(vec![vec![0, 1, 2]]);
        let rules = vec![RewriteRule::wolfram_a_to_bb()];

        let evolution = HypergraphEvolution::run(&initial, &rules, 3);

        let step_0 = evolution.nodes_at_step(0);
        assert_eq!(step_0.len(), 1);
        assert_eq!(step_0[0], 0);
    }
}
