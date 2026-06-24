//! Categorical span representation of rewrite rules.
//!
//! Converts [`RewriteRule`] to [`Span`] from catgraph, enabling
//! compositional analysis of rewrite systems in the category of spans.

use catgraph::span::Span;
use std::collections::{BTreeSet, HashMap};

use super::hypergraph::Hypergraph;
use super::rewrite_rule::{RewriteRule, RewriteSpan};

impl RewriteRule {
    /// Converts this rewrite rule to its categorical span representation.
    ///
    /// A rewrite rule L → R with shared variables K is naturally a span:
    ///
    /// ```text
    ///     L ←── K ──→ R
    /// ```
    ///
    /// - L elements = unique variables in the left pattern
    /// - R elements = unique variables in the right pattern
    /// - K elements = preserved variables (appear in both L and R)
    /// - Each K element maps to its index in L and its index in R
    ///
    /// Labels are `u32` variable IDs, so the span carries which variables
    /// are on each side (e.g., `left() = [0, 1, 2]` for variables 0, 1, 2).
    ///
    /// # Example
    ///
    /// ```rust
    /// use catgraph_physics::hypergraph::RewriteRule;
    ///
    /// // Wolfram A→BB: {0,1,2} → {0,1},{1,2}
    /// let rule = RewriteRule::wolfram_a_to_bb();
    /// let span = rule.to_span();
    ///
    /// // L has 3 variables (0,1,2), R has 3 variables (0,1,2)
    /// assert_eq!(span.left(), &[0u32, 1, 2]);
    /// assert_eq!(span.right(), &[0u32, 1, 2]);
    /// // K = {0,1,2} (all preserved) → 3 middle pairs
    /// assert_eq!(span.middle_pairs().len(), 3);
    /// ```
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    #[must_use]
    pub fn to_span(&self) -> Span<u32> {
        // Collect unique variables from each side, sorted for determinism
        let left_vars: BTreeSet<usize> = self
            .left()
            .iter()
            .flat_map(|e| e.vertices().iter().copied())
            .collect();
        let right_vars: BTreeSet<usize> = self
            .right()
            .iter()
            .flat_map(|e| e.vertices().iter().copied())
            .collect();

        let left_sorted: Vec<usize> = left_vars.iter().copied().collect();
        let right_sorted: Vec<usize> = right_vars.iter().copied().collect();

        // Build index maps: variable → position in sorted vec
        let left_index: HashMap<usize, usize> = left_sorted
            .iter()
            .enumerate()
            .map(|(i, &v)| (v, i))
            .collect();
        let right_index: HashMap<usize, usize> = right_sorted
            .iter()
            .enumerate()
            .map(|(i, &v)| (v, i))
            .collect();

        // Kernel = preserved variables (in both L and R)
        let preserved = self.preserved_variables();
        let mut middle: Vec<(usize, usize)> = preserved
            .iter()
            .map(|&v| (left_index[&v], right_index[&v]))
            .collect();
        // Sort for deterministic output
        middle.sort_unstable();

        // Labels are variable IDs (as u32)
        let left_labels: Vec<u32> = left_sorted.iter().map(|&v| v as u32).collect();
        let right_labels: Vec<u32> = right_sorted.iter().map(|&v| v as u32).collect();

        Span::new(left_labels, right_labels, middle)
    }

    /// Builds the full `RewriteSpan` (L ← K → R) with explicit kernel hypergraph.
    ///
    /// This constructs the kernel as a hypergraph containing only the preserved
    /// vertices and the identity morphisms K → L and K → R.
    #[must_use]
    pub fn to_rewrite_span(&self) -> RewriteSpan {
        let preserved: BTreeSet<usize> = self.preserved_variables().into_iter().collect();

        // Build left hypergraph from pattern
        let mut left = Hypergraph::new();
        for edge in self.left() {
            left.add_hyperedge(edge.vertices().to_vec());
        }

        // Build right hypergraph from pattern
        let mut right = Hypergraph::new();
        for edge in self.right() {
            right.add_hyperedge(edge.vertices().to_vec());
        }

        // Kernel contains only preserved vertices (no edges — they transform)
        let mut kernel = Hypergraph::new();
        for &v in &preserved {
            kernel.add_vertex(Some(v));
        }

        // Identity morphisms: kernel vars map to themselves in L and R
        let left_map: HashMap<usize, usize> = preserved.iter().map(|&v| (v, v)).collect();
        let right_map: HashMap<usize, usize> = preserved.iter().map(|&v| (v, v)).collect();

        RewriteSpan::new(left, kernel, right, left_map, right_map)
    }
}

// ============================================================================
// RewriteSpan → Span
// ============================================================================

impl RewriteSpan {
    /// Converts this `RewriteSpan` to a catgraph `Span<u32>`.
    ///
    /// Uses the `left_map` and `right_map` morphisms to build the span's
    /// middle pairs, mapping kernel elements to their positions in L and R.
    /// Labels are vertex IDs (as `u32`).
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    #[must_use]
    pub fn to_span(&self) -> Span<u32> {
        let left_verts: Vec<usize> = self.left.vertices().collect();
        let right_verts: Vec<usize> = self.right.vertices().collect();

        // Build index maps
        let left_index: HashMap<usize, usize> = left_verts
            .iter()
            .enumerate()
            .map(|(i, &v)| (v, i))
            .collect();
        let right_index: HashMap<usize, usize> = right_verts
            .iter()
            .enumerate()
            .map(|(i, &v)| (v, i))
            .collect();

        // Each kernel vertex maps through left_map to L and right_map to R
        let mut middle: Vec<(usize, usize)> = Vec::new();
        for k_vert in self.kernel.vertices() {
            if let (Some(&l_vert), Some(&r_vert)) =
                (self.left_map.get(&k_vert), self.right_map.get(&k_vert))
                && let (Some(&l_idx), Some(&r_idx)) =
                    (left_index.get(&l_vert), right_index.get(&r_vert))
            {
                middle.push((l_idx, r_idx));
            }
        }
        middle.sort_unstable();

        let left_labels: Vec<u32> = left_verts.iter().map(|&v| v as u32).collect();
        let right_labels: Vec<u32> = right_verts.iter().map(|&v| v as u32).collect();
        Span::new(left_labels, right_labels, middle)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wolfram_a_to_bb_span() {
        // {0,1,2} → {0,1},{1,2}
        // L vars = {0,1,2}, R vars = {0,1,2}, K = {0,1,2} (all preserved)
        let rule = RewriteRule::wolfram_a_to_bb();
        let span = rule.to_span();

        assert_eq!(span.left(), &[0u32, 1, 2]);
        assert_eq!(span.right(), &[0u32, 1, 2]);
        assert_eq!(span.middle_pairs().len(), 3);

        // All three kernel elements map identity: (0,0), (1,1), (2,2)
        for &(l, r) in span.middle_pairs() {
            assert_eq!(l, r, "preserved vars should map to same index");
        }
    }

    #[test]
    fn test_edge_split_span() {
        // {0,1} → {0,2},{2,1}
        // L vars = {0,1}, R vars = {0,1,2}, K = {0,1}
        let rule = RewriteRule::edge_split();
        let span = rule.to_span();

        assert_eq!(span.left(), &[0u32, 1]); // L has vars 0, 1
        assert_eq!(span.right(), &[0u32, 1, 2]); // R has vars 0, 1, 2
        assert_eq!(span.middle_pairs().len(), 2); // K = {0, 1}
    }

    #[test]
    fn test_triangle_rule_span() {
        // {0,1} → {0,1},{1,2},{2,0}
        // L vars = {0,1}, R vars = {0,1,2}, K = {0,1}
        let rule = RewriteRule::triangle();
        let span = rule.to_span();

        assert_eq!(span.left(), &[0u32, 1]);
        assert_eq!(span.right(), &[0u32, 1, 2]);
        assert_eq!(span.middle_pairs().len(), 2);
    }

    #[test]
    fn test_collapse_rule_span() {
        // {0,1},{1,2} → {0,2}
        // L vars = {0,1,2}, R vars = {0,2}, K = {0,2}
        let rule = RewriteRule::collapse();
        let span = rule.to_span();

        assert_eq!(span.left(), &[0u32, 1, 2]);
        assert_eq!(span.right(), &[0u32, 2]);
        assert_eq!(span.middle_pairs().len(), 2);
    }

    #[test]
    fn test_create_self_loop_span() {
        // {0,1} → {0,1},{1,1}
        // L vars = {0,1}, R vars = {0,1}, K = {0,1}
        let rule = RewriteRule::create_self_loop();
        let span = rule.to_span();

        assert_eq!(span.left(), &[0u32, 1]);
        assert_eq!(span.right(), &[0u32, 1]);
        assert_eq!(span.middle_pairs().len(), 2);
    }

    // ── RewriteRule::to_rewrite_span ───────────────────────────────────

    #[test]
    fn test_rewrite_span_roundtrip() {
        let rule = RewriteRule::wolfram_a_to_bb();
        let rspan = rule.to_rewrite_span();

        // Kernel should have 3 preserved vertices
        assert_eq!(rspan.kernel.vertex_count(), 3);
        // Left should have 1 edge (ternary)
        assert_eq!(rspan.left.edge_count(), 1);
        // Right should have 2 edges (binary)
        assert_eq!(rspan.right.edge_count(), 2);

        // Converting RewriteSpan to catgraph Span should match direct conversion
        let span_from_rule = rule.to_span();
        let span_from_rspan = rspan.to_span();

        assert_eq!(span_from_rule.left(), span_from_rspan.left());
        assert_eq!(span_from_rule.right(), span_from_rspan.right());
        assert_eq!(
            span_from_rule.middle_pairs(),
            span_from_rspan.middle_pairs()
        );
    }

    #[test]
    fn test_edge_split_rewrite_span() {
        let rule = RewriteRule::edge_split();
        let rspan = rule.to_rewrite_span();

        // Kernel: vars {0,1} (preserved)
        assert_eq!(rspan.kernel.vertex_count(), 2);
        // Created var: 2 (only in right)
        assert!(rspan.right.vertices().any(|v| v == 2));
        assert!(!rspan.left.vertices().any(|v| v == 2));
    }

    // ── Span validity ──────────────────────────────────────────────────

    #[test]
    fn test_all_common_rules_produce_valid_spans() {
        let rules = vec![
            RewriteRule::wolfram_a_to_bb(),
            RewriteRule::edge_split(),
            RewriteRule::triangle(),
            RewriteRule::collapse(),
            RewriteRule::create_self_loop(),
        ];

        for rule in &rules {
            // to_span() calls Span::new() which calls assert_valid()
            let span = rule.to_span();
            assert!(
                !span.left().is_empty() || !span.right().is_empty(),
                "rule '{rule}' should produce non-trivial span"
            );

            // to_rewrite_span() + to_span() should also be valid
            let rspan = rule.to_rewrite_span();
            let _span2 = rspan.to_span();
        }
    }
}
