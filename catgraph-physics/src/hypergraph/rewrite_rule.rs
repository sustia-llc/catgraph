//! Rewrite rules for hypergraph transformation.
//!
//! Implements the Double-Pushout (DPO) approach to graph rewriting,
//! where a rule is represented as a span L ← K → R. DPO is the classical
//! algebraic approach of Ehrig–Pfender–Schneider (1973) — \[EPS73\] in
//! `docs/ANCHORS.md` (uncached attribution).

use super::hyperedge::Hyperedge;
use super::hypergraph::Hypergraph;
use std::collections::HashMap;

/// A span representing a rewrite rule: L ← K → R
///
/// In the DPO (Double-Pushout) approach:
/// - **L** (Left): The pattern to match in the host graph
/// - **K** (Kernel): The interface - elements preserved during rewrite
/// - **R** (Right): The replacement structure
///
/// The morphisms l: K → L and r: K → R define how the kernel embeds
/// into both sides.
///
/// # Example
///
/// For the classic Wolfram Physics rule `{{x,y,y}} → {{x,y},{y,z}}`:
/// - L = one ternary hyperedge
/// - K = preserved vertex (y)
/// - R = two binary hyperedges sharing vertex y
#[derive(Debug, Clone)]
pub struct RewriteSpan {
    /// Left-hand side pattern to match.
    pub left: Hypergraph,

    /// Kernel (interface) - vertices/edges preserved during rewrite.
    pub kernel: Hypergraph,

    /// Right-hand side replacement.
    pub right: Hypergraph,

    /// Morphism l: K → L (maps kernel vertices to left vertices).
    pub left_map: HashMap<usize, usize>,

    /// Morphism r: K → R (maps kernel vertices to right vertices).
    pub right_map: HashMap<usize, usize>,
}

impl RewriteSpan {
    /// Creates a new `RewriteSpan` from its components.
    ///
    /// # Arguments
    ///
    /// * `left` - The left-hand side pattern (to match)
    /// * `kernel` - The preserved subgraph (interface)
    /// * `right` - The right-hand side replacement
    /// * `left_map` - Morphism K → L (kernel vertex → left vertex)
    /// * `right_map` - Morphism K → R (kernel vertex → right vertex)
    #[must_use]
    pub fn new(
        left: Hypergraph,
        kernel: Hypergraph,
        right: Hypergraph,
        left_map: HashMap<usize, usize>,
        right_map: HashMap<usize, usize>,
    ) -> Self {
        Self {
            left,
            kernel,
            right,
            left_map,
            right_map,
        }
    }
}

/// A simplified rewrite rule for pattern-based replacement.
///
/// This is a more user-friendly interface that doesn't require
/// explicit kernel specification. The kernel is inferred from
/// shared variables between left and right patterns.
///
/// # Example
///
/// ```rust
/// use catgraph_physics::hypergraph::RewriteRule;
///
/// // Rule: {0, 1, 2} → {0, 1}, {1, 2}
/// // Variables 0, 1, 2 are pattern variables, not literal vertex IDs
/// let rule = RewriteRule::from_pattern(
///     vec![vec![0, 1, 2]],           // Left: one ternary hyperedge
///     vec![vec![0, 1], vec![1, 2]],  // Right: two binary hyperedges
/// );
///
/// assert_eq!(rule.left_arity(), 1);
/// assert_eq!(rule.right_arity(), 2);
/// ```
#[derive(Debug, Clone)]
pub struct RewriteRule {
    /// Left-hand side pattern (hyperedges with pattern variables).
    left: Vec<Hyperedge>,

    /// Right-hand side pattern.
    right: Vec<Hyperedge>,

    /// Number of pattern variables used.
    num_variables: usize,

    /// Human-readable name/description.
    name: Option<String>,
}

impl RewriteRule {
    /// Creates a rewrite rule from pattern specifications.
    ///
    /// Pattern variables are non-negative integers that can be matched
    /// to any vertex in the host graph.
    ///
    /// # Arguments
    ///
    /// * `left` - Left-hand side pattern (list of hyperedges as vertex lists)
    /// * `right` - Right-hand side pattern
    ///
    /// # Example
    ///
    /// ```rust
    /// use catgraph_physics::hypergraph::RewriteRule;
    ///
    /// // Self-loop splitting rule: {x, x} → {x, y}, {y, x}
    /// let rule = RewriteRule::from_pattern(
    ///     vec![vec![0, 0]],
    ///     vec![vec![0, 1], vec![1, 0]],
    /// );
    /// ```
    #[must_use]
    pub fn from_pattern(left: Vec<Vec<usize>>, right: Vec<Vec<usize>>) -> Self {
        let left_edges: Vec<_> = left.into_iter().map(Hyperedge::new).collect();
        let right_edges: Vec<_> = right.into_iter().map(Hyperedge::new).collect();

        // Count pattern variables
        let mut max_var = 0;
        for edge in &left_edges {
            for &v in edge.vertices() {
                max_var = max_var.max(v);
            }
        }
        for edge in &right_edges {
            for &v in edge.vertices() {
                max_var = max_var.max(v);
            }
        }

        Self {
            left: left_edges,
            right: right_edges,
            num_variables: max_var + 1,
            name: None,
        }
    }

    /// Creates a named rewrite rule.
    #[must_use]
    pub fn named(name: &str, left: Vec<Vec<usize>>, right: Vec<Vec<usize>>) -> Self {
        let mut rule = Self::from_pattern(left, right);
        rule.name = Some(name.to_string());
        rule
    }

    /// Returns the name of this rule (if set).
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the left-hand side pattern.
    #[must_use]
    pub fn left(&self) -> &[Hyperedge] {
        &self.left
    }

    /// Returns the right-hand side pattern.
    #[must_use]
    pub fn right(&self) -> &[Hyperedge] {
        &self.right
    }

    /// Returns the number of hyperedges in the left pattern.
    #[must_use]
    pub fn left_arity(&self) -> usize {
        self.left.len()
    }

    /// Returns the number of hyperedges in the right pattern.
    #[must_use]
    pub fn right_arity(&self) -> usize {
        self.right.len()
    }

    /// Returns the number of pattern variables.
    #[must_use]
    pub fn num_variables(&self) -> usize {
        self.num_variables
    }

    /// Returns the pattern variables used only on the left (deleted).
    #[must_use]
    pub fn deleted_variables(&self) -> Vec<usize> {
        let left_vars: std::collections::HashSet<_> = self
            .left
            .iter()
            .flat_map(|e| e.vertices().iter().copied())
            .collect();
        let right_vars: std::collections::HashSet<_> = self
            .right
            .iter()
            .flat_map(|e| e.vertices().iter().copied())
            .collect();

        left_vars.difference(&right_vars).copied().collect()
    }

    /// Returns the pattern variables used only on the right (created).
    #[must_use]
    pub fn created_variables(&self) -> Vec<usize> {
        let left_vars: std::collections::HashSet<_> = self
            .left
            .iter()
            .flat_map(|e| e.vertices().iter().copied())
            .collect();
        let right_vars: std::collections::HashSet<_> = self
            .right
            .iter()
            .flat_map(|e| e.vertices().iter().copied())
            .collect();

        right_vars.difference(&left_vars).copied().collect()
    }

    /// Returns the pattern variables preserved (in both left and right).
    #[must_use]
    pub fn preserved_variables(&self) -> Vec<usize> {
        let left_vars: std::collections::HashSet<_> = self
            .left
            .iter()
            .flat_map(|e| e.vertices().iter().copied())
            .collect();
        let right_vars: std::collections::HashSet<_> = self
            .right
            .iter()
            .flat_map(|e| e.vertices().iter().copied())
            .collect();

        left_vars.intersection(&right_vars).copied().collect()
    }

    /// Finds all matches of this rule's left-hand side in the given hypergraph.
    ///
    /// A match is a mapping from pattern variables to actual vertices such that
    /// all left-hand side hyperedges exist in the graph.
    ///
    /// # Returns
    ///
    /// A vector of matches, where each match is a mapping from pattern variable
    /// to actual vertex ID.
    #[must_use]
    pub fn find_matches(&self, graph: &Hypergraph) -> Vec<RewriteMatch> {
        if self.left.is_empty() {
            // Empty pattern matches everywhere
            return vec![RewriteMatch {
                variable_map: HashMap::new(),
                matched_edges: vec![],
            }];
        }

        // For each edge in the left pattern, find compatible edges in graph
        let first_pattern = &self.left[0];
        let mut matches = Vec::new();

        // Find edges that could match the first pattern edge
        for (edge_idx, edge) in graph.edges().enumerate() {
            if edge.arity() != first_pattern.arity() {
                continue;
            }

            // Try to build a variable mapping
            let mut var_map = HashMap::new();
            let mut valid = true;

            for (&pv, &ev) in first_pattern.vertices().iter().zip(edge.vertices()) {
                if let Some(&existing) = var_map.get(&pv) {
                    if existing != ev {
                        valid = false;
                        break;
                    }
                } else {
                    var_map.insert(pv, ev);
                }
            }

            if !valid {
                continue;
            }

            // If more pattern edges, check they all match with this mapping
            if self.left.len() == 1 {
                matches.push(RewriteMatch {
                    variable_map: var_map,
                    matched_edges: vec![edge_idx],
                });
            } else {
                // Try to extend the mapping to cover all pattern edges
                if let Some(full_match) = self.extend_match(graph, var_map, vec![edge_idx], 1) {
                    matches.push(full_match);
                }
            }
        }

        matches
    }

    /// Extends a partial match to cover all pattern edges.
    fn extend_match(
        &self,
        graph: &Hypergraph,
        var_map: HashMap<usize, usize>,
        matched_edges: Vec<usize>,
        pattern_idx: usize,
    ) -> Option<RewriteMatch> {
        if pattern_idx >= self.left.len() {
            return Some(RewriteMatch {
                variable_map: var_map,
                matched_edges,
            });
        }

        let pattern_edge = &self.left[pattern_idx];

        // Find edges that could match this pattern edge
        for (edge_idx, edge) in graph.edges().enumerate() {
            if matched_edges.contains(&edge_idx) {
                continue; // Already matched
            }
            if edge.arity() != pattern_edge.arity() {
                continue;
            }

            // Try to extend the variable mapping
            let mut new_map = var_map.clone();
            let mut valid = true;

            for (&pv, &ev) in pattern_edge.vertices().iter().zip(edge.vertices()) {
                if let Some(&existing) = new_map.get(&pv) {
                    if existing != ev {
                        valid = false;
                        break;
                    }
                } else {
                    new_map.insert(pv, ev);
                }
            }

            if !valid {
                continue;
            }

            // Recursively try to match remaining patterns
            let mut new_matched = matched_edges.clone();
            new_matched.push(edge_idx);

            if let Some(result) = self.extend_match(graph, new_map, new_matched, pattern_idx + 1) {
                return Some(result);
            }
        }

        None
    }

    /// Applies this rule to a hypergraph at the given match.
    ///
    /// # Arguments
    ///
    /// * `graph` - The hypergraph to rewrite (will be modified)
    /// * `match_` - A valid match from `find_matches`
    /// * `next_vertex_id` - Counter for generating new vertex IDs
    ///
    /// # Returns
    ///
    /// A map from newly created pattern variables to their assigned vertex IDs.
    ///
    /// # Panics
    ///
    /// Panics if a pattern variable in the right-hand side is not in the variable map.
    pub fn apply(
        &self,
        graph: &mut Hypergraph,
        match_: &RewriteMatch,
        next_vertex_id: &mut usize,
    ) -> HashMap<usize, usize> {
        // Remove matched edges (in reverse order to preserve indices)
        let mut to_remove = match_.matched_edges.clone();
        to_remove.sort_unstable();
        to_remove.reverse();
        for edge_idx in to_remove {
            graph.remove_edge(edge_idx);
        }

        // Build complete variable map (existing + new)
        let mut full_map = match_.variable_map.clone();

        // Assign IDs to new variables
        let created = self.created_variables();
        let mut new_vars = HashMap::new();
        for var in created {
            let id = *next_vertex_id;
            *next_vertex_id += 1;
            full_map.insert(var, id);
            new_vars.insert(var, id);
        }

        // Add right-hand side edges with mapped vertices
        for pattern_edge in &self.right {
            let actual_vertices: Vec<_> = pattern_edge
                .vertices()
                .iter()
                .map(|&pv| *full_map.get(&pv).unwrap())
                .collect();
            graph.add_hyperedge(actual_vertices);
        }

        new_vars
    }
}

/// A match of a rewrite rule's left-hand side in a host hypergraph.
///
/// Contains the mapping from pattern variables to actual vertex IDs
/// and the indices of matched hyperedges. Produced by
/// [`RewriteRule::find_matches`] and consumed by [`RewriteRule::apply`].
#[derive(Debug, Clone)]
pub struct RewriteMatch {
    /// Mapping from pattern variables to actual vertex IDs.
    pub variable_map: HashMap<usize, usize>,

    /// Indices of matched hyperedges in the host graph.
    pub matched_edges: Vec<usize>,
}

impl RewriteMatch {
    /// Returns the actual vertex ID for a pattern variable.
    #[must_use]
    pub fn get(&self, pattern_var: usize) -> Option<usize> {
        self.variable_map.get(&pattern_var).copied()
    }
}

// ============================================================================
// Common Rules
// ============================================================================

impl RewriteRule {
    /// The classic Wolfram Physics "A→BB" rule.
    ///
    /// `{{x, y, z}} → {{x, y}, {y, z}}`
    ///
    /// Splits a ternary hyperedge into two binary ones.
    #[must_use]
    pub fn wolfram_a_to_bb() -> Self {
        Self::named("A→BB", vec![vec![0, 1, 2]], vec![vec![0, 1], vec![1, 2]])
    }

    /// Self-loop creation rule.
    ///
    /// `{{x, y}} → {{x, y}, {y, y}}`
    #[must_use]
    pub fn create_self_loop() -> Self {
        Self::named(
            "create-loop",
            vec![vec![0, 1]],
            vec![vec![0, 1], vec![1, 1]],
        )
    }

    /// Edge splitting rule.
    ///
    /// `{{x, y}} → {{x, z}, {z, y}}`
    ///
    /// Inserts a new vertex in the middle of an edge.
    #[must_use]
    pub fn edge_split() -> Self {
        Self::named("edge-split", vec![vec![0, 1]], vec![vec![0, 2], vec![2, 1]])
    }

    /// Triangle rule (creates structure).
    ///
    /// `{{x, y}} → {{x, y}, {y, z}, {z, x}}`
    #[must_use]
    pub fn triangle() -> Self {
        Self::named(
            "triangle",
            vec![vec![0, 1]],
            vec![vec![0, 1], vec![1, 2], vec![2, 0]],
        )
    }

    /// Collapse rule (merges structure).
    ///
    /// `{{x, y}, {y, z}} → {{x, z}}`
    #[must_use]
    pub fn collapse() -> Self {
        Self::named("collapse", vec![vec![0, 1], vec![1, 2]], vec![vec![0, 2]])
    }
}

impl std::fmt::Display for RewriteRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = &self.name {
            write!(f, "{name}: ")?;
        }

        // Left side
        write!(f, "{{")?;
        for (i, edge) in self.left.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{edge}")?;
        }
        write!(f, "}} → {{")?;

        // Right side
        for (i, edge) in self.right.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{edge}")?;
        }
        write!(f, "}}")
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_from_pattern() {
        let rule = RewriteRule::from_pattern(vec![vec![0, 1, 2]], vec![vec![0, 1], vec![1, 2]]);

        assert_eq!(rule.left_arity(), 1);
        assert_eq!(rule.right_arity(), 2);
        assert_eq!(rule.num_variables(), 3);
    }

    #[test]
    fn test_rule_variables() {
        let rule = RewriteRule::from_pattern(vec![vec![0, 1]], vec![vec![0, 2], vec![2, 1]]);

        assert!(rule.deleted_variables().is_empty());
        assert_eq!(rule.created_variables(), vec![2]);
        assert!(rule.preserved_variables().contains(&0));
        assert!(rule.preserved_variables().contains(&1));
    }

    #[test]
    fn test_find_matches() {
        let graph = Hypergraph::from_edges(vec![vec![0, 1, 2], vec![3, 4, 5]]);

        let rule = RewriteRule::from_pattern(
            vec![vec![0, 1, 2]], // Match any ternary edge
            vec![vec![0, 1], vec![1, 2]],
        );

        let matches = rule.find_matches(&graph);
        assert_eq!(matches.len(), 2); // Both edges match the pattern
    }

    #[test]
    fn test_apply_rule() {
        let mut graph = Hypergraph::from_edges(vec![vec![0, 1, 2]]);
        let rule = RewriteRule::wolfram_a_to_bb();

        let matches = rule.find_matches(&graph);
        assert_eq!(matches.len(), 1);

        let mut next_id = 3;
        let _new_vars = rule.apply(&mut graph, &matches[0], &mut next_id);

        assert_eq!(graph.edge_count(), 2); // One removed, two added
        // The edges should be {0, 1} and {1, 2}
    }

    #[test]
    fn test_edge_split_rule() {
        let mut graph = Hypergraph::from_edges(vec![vec![0, 1]]);
        let rule = RewriteRule::edge_split();

        let matches = rule.find_matches(&graph);
        let mut next_id = 2;
        let new_vars = rule.apply(&mut graph, &matches[0], &mut next_id);

        assert_eq!(graph.edge_count(), 2);
        assert_eq!(new_vars.len(), 1); // One new vertex created
    }

    #[test]
    fn test_rule_display() {
        let rule = RewriteRule::wolfram_a_to_bb();
        let display = format!("{rule}");
        assert!(display.contains("A→BB"));
        assert!(display.contains("→"));
    }

    #[test]
    fn test_multi_edge_pattern() {
        let graph = Hypergraph::from_edges(vec![vec![0, 1], vec![1, 2], vec![2, 3]]);

        // Pattern matching two consecutive edges
        let rule = RewriteRule::from_pattern(vec![vec![0, 1], vec![1, 2]], vec![vec![0, 2]]);

        let matches = rule.find_matches(&graph);
        // Should find: (0,1)+(1,2) and (1,2)+(2,3)
        assert_eq!(matches.len(), 2);
    }
}
