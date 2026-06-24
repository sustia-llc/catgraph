//! Hypergraph evolution tracking and causal invariance analysis.
//!
//! This module tracks the history of hypergraph rewrites and provides
//! tools for analyzing causal invariance via Wilson loops.

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
/// in lattice gauge theory. The holonomy (product of transformations
/// around the loop) measures deviation from path-independence:
/// holonomy = 1.0 means the system is causally invariant along this loop.
#[derive(Debug, Clone)]
pub struct WilsonLoop {
    /// Sequence of node IDs forming the loop.
    pub path: Vec<usize>,

    /// Starting/ending node ID.
    pub base: usize,

    /// Holonomy value (1.0 = perfect closure, causally invariant).
    pub holonomy: f64,

    /// Length of the loop.
    pub length: usize,
}

/// Result of causal invariance analysis.
#[derive(Debug, Clone)]
pub struct CausalInvarianceResult {
    /// Whether the system is causally invariant.
    pub is_invariant: bool,

    /// Average holonomy deviation from 1.0.
    pub average_deviation: f64,

    /// Maximum holonomy deviation from 1.0.
    pub max_deviation: f64,

    /// Number of Wilson loops analyzed.
    pub loops_analyzed: usize,

    /// Wilson loops with significant deviation.
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
}

impl HypergraphEvolution {
    /// Creates a new evolution starting from the given hypergraph.
    #[must_use]
    pub fn new(initial: Hypergraph, rules: Vec<RewriteRule>) -> Self {
        let fingerprint = initial.fingerprint();
        let max_vertex = initial.vertices().max().unwrap_or(0);

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

        // Apply the rule
        let rule = &self.rules[rule_idx];
        rule.apply(&mut new_state, match_, &mut self.next_vertex_id);

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

                        // Build the loop path
                        let mut loop_path = Vec::new();

                        // Path from ancestor to id1
                        for &id in path1.iter().rev() {
                            loop_path.push(id);
                            if id == ancestor {
                                break;
                            }
                        }

                        // Path from id2 back to ancestor
                        let mut path2_segment = Vec::new();
                        for &id in &path2 {
                            if id == ancestor {
                                break;
                            }
                            path2_segment.push(id);
                        }
                        path2_segment.reverse();
                        loop_path.extend(path2_segment);

                        // Compute holonomy
                        let holonomy = self.compute_holonomy(&loop_path);

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

    /// Computes the holonomy of a loop.
    ///
    /// Holonomy measures how much the state changes when going around a loop.
    /// - Holonomy = 1.0: Perfect closure (causally invariant)
    /// - Holonomy < 1.0: State differs after traversing the loop
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    fn compute_holonomy(&self, loop_path: &[usize]) -> f64 {
        if loop_path.len() < 2 {
            return 1.0;
        }

        let start_node = &self.nodes[loop_path[0]];
        let end_node = &self.nodes[*loop_path.last().unwrap()];

        // Compare states using isomorphism check
        if start_node.state.is_isomorphic_to(&end_node.state) {
            1.0
        } else {
            // Compute similarity based on structural overlap
            let start_edges = start_node.state.edge_count();
            let end_edges = end_node.state.edge_count();

            if start_edges == 0 && end_edges == 0 {
                return 1.0;
            }

            // Simple similarity measure
            let common_vertices = start_node
                .state
                .vertices()
                .filter(|v| end_node.state.contains_vertex(*v))
                .count();
            let total_vertices = start_node
                .state
                .vertex_count()
                .max(end_node.state.vertex_count());

            if total_vertices == 0 {
                1.0
            } else {
                common_vertices as f64 / total_vertices as f64
            }
        }
    }

    /// Analyzes causal invariance of the evolution.
    ///
    /// A system is causally invariant if all Wilson loops have holonomy = 1.0,
    /// meaning the final state is independent of the order of rule applications.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
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

        let deviations: Vec<_> = loops.iter().map(|l| (1.0 - l.holonomy).abs()).collect();

        let average_deviation = deviations.iter().sum::<f64>() / deviations.len() as f64;
        let max_deviation = deviations.iter().copied().fold(0.0, f64::max);

        // Consider loops with deviation > 0.01 as non-trivial
        let non_trivial_loops: Vec<_> = loops
            .into_iter()
            .filter(|l| (1.0 - l.holonomy).abs() > 0.01)
            .collect();

        let is_invariant = max_deviation < 0.01;

        CausalInvarianceResult {
            is_invariant,
            average_deviation,
            max_deviation,
            loops_analyzed: deviations.len(),
            non_trivial_loops,
        }
    }

    /// Checks if the system is causally invariant.
    ///
    /// This is a quick check that returns true if all explored paths
    /// that lead to isomorphic states have holonomy ≈ 1.0.
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
