//! Cospan chain bridge for hypergraph evolution.
//!
//! Converts [`HypergraphEvolution`] deterministic paths to composable
//! [`Cospan`] chains from catgraph, enabling pushout-based composition
//! of rewrite traces.

use catgraph::category::Composable;
use catgraph::cospan::Cospan;
use catgraph::errors::CatgraphError;
use std::collections::{BTreeSet, HashMap};

use super::evolution::HypergraphEvolution;

impl HypergraphEvolution {
    /// Converts the evolution into a chain of composable cospans.
    ///
    /// Each rewrite step Gi → Gi+1 produces a cospan:
    ///
    /// ```text
    ///     Gi_boundary ──→ apex ←── Gi+1_boundary
    /// ```
    ///
    /// where the apex is the union of all vertices from both states,
    /// and the boundary maps send each state's vertices to their
    /// positions in the apex. Preserved vertices map to the same
    /// apex element, creating the categorical "gluing."
    ///
    /// The returned cospans are composable: the right boundary of
    /// cospan i matches the left boundary of cospan i+1.
    ///
    /// # Returns
    ///
    /// A vector of cospans along the deterministic (root-to-last-node) path.
    /// Empty if the evolution has only the root node.
    #[must_use]
    pub fn to_cospan_chain(&self) -> Vec<Cospan<u32>> {
        let path = self.deterministic_path();
        if path.len() < 2 {
            return vec![];
        }

        // TODO(rayon-1.12): candidate for `par_array_windows::<2>()` from
        // the new `ParallelSlice` API. `build_cospan_for_pair` performs a
        // pushout (union-find) per pair, so there's real per-pair work —
        // benchable on long deterministic paths.
        path.windows(2)
            .map(|w| self.build_cospan_for_pair(w[0], w[1]))
            .collect()
    }

    /// Returns the deterministic path from root to the deepest node.
    ///
    /// Follows the first child at each step (deterministic choice).
    fn deterministic_path(&self) -> Vec<usize> {
        let mut path = vec![0]; // Start at root
        let mut current = 0;

        loop {
            // Find first child of current
            let mut found_child = false;
            for id in (current + 1)..self.node_count() {
                if let Some(node) = self.get_node(id)
                    && node.parent == Some(current)
                {
                    path.push(id);
                    current = id;
                    found_child = true;
                    break;
                }
            }
            if !found_child {
                break;
            }
        }

        path
    }

    /// Builds a cospan from a parent-child node pair.
    ///
    /// The apex is the union of both vertex sets, with labels as vertex IDs.
    ///
    /// # Panics
    ///
    /// Panics if `parent_id` or `child_id` does not correspond to a valid node in the evolution.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    #[must_use]
    pub fn build_cospan_for_pair(&self, parent_id: usize, child_id: usize) -> Cospan<u32> {
        let parent = self.get_node(parent_id).unwrap();
        let child = self.get_node(child_id).unwrap();

        let parent_verts: Vec<usize> = parent.state.vertices().collect();
        let child_verts: Vec<usize> = child.state.vertices().collect();

        let mut apex_set: BTreeSet<usize> = BTreeSet::new();
        apex_set.extend(&parent_verts);
        apex_set.extend(&child_verts);
        let apex_sorted: Vec<usize> = apex_set.iter().copied().collect();

        let apex_index: HashMap<usize, usize> = apex_sorted
            .iter()
            .enumerate()
            .map(|(i, &v)| (v, i))
            .collect();

        let left: Vec<usize> = parent_verts.iter().map(|v| apex_index[v]).collect();
        let right: Vec<usize> = child_verts.iter().map(|v| apex_index[v]).collect();
        let middle: Vec<u32> = apex_sorted.iter().map(|&v| v as u32).collect();

        Cospan::new(left, right, middle)
    }

    /// Composes the deterministic cospan chain into a single composite cospan
    /// representing the global transformation from initial to final state.
    ///
    /// The composite's domain = root vertex IDs, codomain = final vertex IDs.
    ///
    /// # Panics
    ///
    /// Panics if adjacent cospans in the chain are not composable.
    ///
    /// # Errors
    ///
    /// Returns `CatgraphError::Composition` if the cospan chain is empty.
    pub fn compose_cospan_chain(&self) -> Result<Cospan<u32>, CatgraphError> {
        let chain = self.to_cospan_chain();
        chain
            .into_iter()
            .reduce(|acc, c| {
                acc.compose(&c)
                    .expect("evolution cospans must be composable")
            })
            .ok_or_else(|| CatgraphError::Composition {
                message: "empty cospan chain".to_string(),
            })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hypergraph::hypergraph::Hypergraph;
    use crate::hypergraph::rewrite_rule::RewriteRule;

    #[test]
    fn test_deterministic_evolution_cospan_chain() {
        let initial = Hypergraph::from_edges(vec![vec![0, 1, 2]]);
        let rules = vec![RewriteRule::wolfram_a_to_bb()];
        let evolution = HypergraphEvolution::run(&initial, &rules, 3);

        let cospans = evolution.to_cospan_chain();

        // Should have at least 1 cospan (one rewrite step)
        assert!(!cospans.is_empty());

        // Each cospan's left boundary should have the parent's vertex count
        // and right boundary should have the child's vertex count
        for cospan in &cospans {
            assert!(!cospan.left_to_middle().is_empty());
            assert!(!cospan.right_to_middle().is_empty());
            // All indices should be valid (< middle.len())
            let middle_len = cospan.middle().len();
            assert!(cospan.left_to_middle().iter().all(|&i| i < middle_len));
            assert!(cospan.right_to_middle().iter().all(|&i| i < middle_len));
        }
    }

    #[test]
    fn test_cospan_chain_preserves_shared_vertices() {
        // A→BB: {0,1,2} → {0,1},{1,2}
        // All vertices preserved, so parent and child share the same apex positions
        let initial = Hypergraph::from_edges(vec![vec![0, 1, 2]]);
        let rules = vec![RewriteRule::wolfram_a_to_bb()];
        let evolution = HypergraphEvolution::run(&initial, &rules, 1);

        let cospans = evolution.to_cospan_chain();
        assert_eq!(cospans.len(), 1);

        let cospan = &cospans[0];
        // For A→BB with no new vertices, parent and child have same vertices
        // so left and right should map to the same apex positions
        assert_eq!(cospan.left_to_middle(), cospan.right_to_middle());
    }

    #[test]
    fn test_cospan_chain_with_new_vertices() {
        // edge-split: {0,1} → {0,2},{2,1} — creates new vertex
        let initial = Hypergraph::from_edges(vec![vec![0, 1]]);
        let rules = vec![RewriteRule::edge_split()];
        let evolution = HypergraphEvolution::run(&initial, &rules, 1);

        let cospans = evolution.to_cospan_chain();
        assert_eq!(cospans.len(), 1);

        let cospan = &cospans[0];
        // Parent has 2 vertices {0,1}, child has 3 {0,1,2}
        assert_eq!(cospan.left_to_middle().len(), 2);
        assert_eq!(cospan.right_to_middle().len(), 3);
        // Apex should have 3 elements (union)
        assert_eq!(cospan.middle().len(), 3);
    }

    #[test]
    fn test_empty_evolution_no_cospans() {
        // No applicable rules → no steps → no cospans
        let initial = Hypergraph::from_edges(vec![vec![0, 1]]);
        let rules = vec![RewriteRule::wolfram_a_to_bb()]; // needs ternary edge
        let evolution = HypergraphEvolution::run(&initial, &rules, 10);

        let cospans = evolution.to_cospan_chain();
        assert!(cospans.is_empty());
    }

    #[test]
    fn test_multi_step_cospan_chain() {
        // Run multiple edge splits
        let initial = Hypergraph::from_edges(vec![vec![0, 1]]);
        let rules = vec![RewriteRule::edge_split()];
        let evolution = HypergraphEvolution::run(&initial, &rules, 3);

        let cospans = evolution.to_cospan_chain();
        assert_eq!(cospans.len(), 3);

        // Chain should be composable: right boundary of step i
        // has same size as left boundary of step i+1
        for i in 0..cospans.len() - 1 {
            assert_eq!(
                cospans[i].right_to_middle().len(),
                cospans[i + 1].left_to_middle().len(),
                "cospan chain boundary mismatch at step {i}"
            );
        }
    }

    // ── Cospan label values ─────────────────────────────────────────────

    #[test]
    fn test_cospan_apex_labels_are_vertex_ids() {
        // A→BB preserves all vertices: apex labels should be {0, 1, 2}
        let initial = Hypergraph::from_edges(vec![vec![0, 1, 2]]);
        let rules = vec![RewriteRule::wolfram_a_to_bb()];
        let evolution = HypergraphEvolution::run(&initial, &rules, 1);

        let cospans = evolution.to_cospan_chain();
        assert_eq!(cospans.len(), 1);

        // Apex should contain actual vertex IDs
        assert_eq!(cospans[0].middle(), &[0u32, 1, 2]);
    }

    #[test]
    fn test_cospan_chain_composable_via_catgraph() {
        let initial = Hypergraph::from_edges(vec![vec![0, 1]]);
        let rules = vec![RewriteRule::edge_split()];
        let evolution = HypergraphEvolution::run(&initial, &rules, 3);

        let cospans = evolution.to_cospan_chain();
        assert_eq!(cospans.len(), 3);

        // Verify composability via catgraph's Composable trait
        for i in 0..cospans.len() - 1 {
            assert!(
                cospans[i].composable(&cospans[i + 1]).is_ok(),
                "cospans {} and {} should be composable",
                i,
                i + 1
            );
        }
    }

    // ── Cospan composition ────────────────────────────────────────────

    #[test]
    fn test_compose_single_step() {
        let initial = Hypergraph::from_edges(vec![vec![0, 1, 2]]);
        let rules = vec![RewriteRule::wolfram_a_to_bb()];
        let evolution = HypergraphEvolution::run(&initial, &rules, 1);

        let composite = evolution.compose_cospan_chain().unwrap();

        // For A→BB with all preserved, domain = codomain = {0, 1, 2}
        assert_eq!(composite.domain(), vec![0u32, 1, 2]);
        assert_eq!(composite.codomain(), vec![0u32, 1, 2]);
    }

    #[test]
    fn test_compose_multi_step() {
        let initial = Hypergraph::from_edges(vec![vec![0, 1]]);
        let rules = vec![RewriteRule::edge_split()];
        let evolution = HypergraphEvolution::run(&initial, &rules, 3);

        let composite = evolution.compose_cospan_chain().unwrap();

        // Domain should be root vertices
        assert_eq!(composite.domain(), vec![0u32, 1]);
    }

    #[test]
    fn test_compose_empty_chain_error() {
        let initial = Hypergraph::from_edges(vec![vec![0, 1]]);
        let rules = vec![RewriteRule::wolfram_a_to_bb()]; // won't match binary edge
        let evolution = HypergraphEvolution::run(&initial, &rules, 10);

        assert!(evolution.compose_cospan_chain().is_err());
    }
}
