//! Categorical span representation of rewrite rules.
//!
//! Converts [`RewriteRule`] to [`Span`] from catgraph, enabling
//! compositional analysis of rewrite systems in the category of spans.

use catgraph::span::Span;
use std::collections::{BTreeSet, HashMap};

use super::hypergraph::Hypergraph;
use super::rewrite_rule::{RewriteRule, RewriteSpan, RewriteSpanError, SpanSide};

impl RewriteRule {
    /// Span `L ← K → R`: `L`/`R` = the distinct variables of each pattern as
    /// `u32` labels, `K` = variables in both, each mapped to its index on
    /// either side.
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

        // Correct by construction: `preserved` is the intersection of the two
        // variable sets, so each pair is in bounds on both sides and both labels
        // are the same variable ID.
        Span::new_unchecked(left_labels, right_labels, middle)
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

        RewriteSpan::try_new(left, kernel, right, left_map, right_map).expect(
            "invariant: the kernel is the preserved variables, each an identity-mapped vertex of both L and R",
        )
    }
}

// ============================================================================
// RewriteSpan → Span
// ============================================================================

impl RewriteSpan {
    /// Span `L ← K → R`: the legs are this span's L and R vertices as `u32`
    /// labels, one middle pair per kernel vertex carrying its `left_map` /
    /// `right_map` images' positions.
    ///
    /// Every kernel vertex contributes a pair or the call fails.
    ///
    /// # Precondition
    ///
    /// A span's middle pair links two boundary elements carrying the **same**
    /// label, and the labels here are vertex IDs, so a kernel vertex must have
    /// the same ID under `left_map` and `right_map`. [`RewriteSpan`]'s fields
    /// are public, so a caller-assembled value can break it; the span is built
    /// with [`Span::new_unchecked`], whose label check is `debug_assert!`-only.
    ///
    /// # Errors
    ///
    /// - [`RewriteSpanError::UnmappedKernelVertex`] if a kernel vertex is
    ///   absent from `left_map` or from `right_map`.
    /// - [`RewriteSpanError::ImageNotAVertex`] if a kernel vertex's image is
    ///   not a vertex of that side's hypergraph.
    /// - [`RewriteSpanError::NonInjectiveMap`] if two kernel vertices share an
    ///   image under `left_map` or under `right_map`.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    pub fn to_span(&self) -> Result<Span<u32>, RewriteSpanError> {
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
        let mut middle: Vec<(usize, usize)> = Vec::with_capacity(self.kernel.vertex_count());
        let mut left_preimages: HashMap<usize, usize> = HashMap::new();
        let mut right_preimages: HashMap<usize, usize> = HashMap::new();
        for k_vert in self.kernel.vertices() {
            let l_idx = index_of(
                k_vert,
                &self.left_map,
                &left_index,
                &mut left_preimages,
                SpanSide::Left,
            )?;
            let r_idx = index_of(
                k_vert,
                &self.right_map,
                &right_index,
                &mut right_preimages,
                SpanSide::Right,
            )?;
            middle.push((l_idx, r_idx));
        }
        middle.sort_unstable();

        let left_labels: Vec<u32> = left_verts.iter().map(|&v| v as u32).collect();
        let right_labels: Vec<u32> = right_verts.iter().map(|&v| v as u32).collect();
        // Bounds hold by construction (`left_index` / `right_index` lookups);
        // label agreement is the documented precondition above.
        Ok(Span::new_unchecked(left_labels, right_labels, middle))
    }
}

/// Position of `k_vert`'s image under `map` in `index`, on the named side,
/// recording the image in `preimages` against the kernel vertex that took it.
fn index_of(
    k_vert: usize,
    map: &HashMap<usize, usize>,
    index: &HashMap<usize, usize>,
    preimages: &mut HashMap<usize, usize>,
    side: SpanSide,
) -> Result<usize, RewriteSpanError> {
    let image = *map
        .get(&k_vert)
        .ok_or(RewriteSpanError::UnmappedKernelVertex {
            vertex: k_vert,
            side,
        })?;
    let position = index
        .get(&image)
        .copied()
        .ok_or(RewriteSpanError::ImageNotAVertex {
            vertex: k_vert,
            image,
            side,
        })?;
    if let Some(vertex_a) = preimages.insert(image, k_vert) {
        return Err(RewriteSpanError::NonInjectiveMap {
            vertex_a,
            vertex_b: k_vert,
            image,
            side,
        });
    }
    Ok(position)
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
        assert_eq!(
            span.middle_pairs(),
            &[(0, 0), (1, 1), (2, 2)],
            "all three variables are preserved, each at the same index on both sides"
        );
    }

    #[test]
    fn test_edge_split_span() {
        // {0,1} → {0,2},{2,1}
        // L vars = {0,1}, R vars = {0,1,2}, K = {0,1}
        let rule = RewriteRule::edge_split();
        let span = rule.to_span();

        assert_eq!(span.left(), &[0u32, 1]); // L has vars 0, 1
        assert_eq!(span.right(), &[0u32, 1, 2]); // R has vars 0, 1, 2
        assert_eq!(
            span.middle_pairs(),
            &[(0, 0), (1, 1)],
            "K = {{0,1}}; the created variable 2 sits at right index 2 and is unpaired"
        );
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
        let span_from_rspan = rspan.to_span().expect("identity maps embed");

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

    /// Re-checks each span through [`Span::new`]. In the debug profile the
    /// operative check is core's `assert_valid` inside `Span::new_unchecked`,
    /// which panics before this call is reached; in the release profile, where
    /// that `debug_assert!` compiles away, it is this [`Span::new`] call.
    fn recheck(span: &Span<u32>) -> Result<Span<u32>, catgraph::errors::CatgraphError> {
        Span::new(
            span.left().to_vec(),
            span.right().to_vec(),
            span.middle_pairs().to_vec(),
        )
    }

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
            let span = rule.to_span();
            assert!(
                !span.left().is_empty() || !span.right().is_empty(),
                "rule '{rule}' should produce non-trivial span"
            );
            let rechecked = recheck(&span);
            assert!(
                rechecked.is_ok(),
                "rule '{rule}' direct span: {:?}",
                rechecked.err()
            );

            let rspan = rule.to_rewrite_span();
            let span2 = rspan.to_span().expect("identity maps embed");
            let rechecked2 = recheck(&span2);
            assert!(
                rechecked2.is_ok(),
                "rule '{rule}' RewriteSpan span: {:?}",
                rechecked2.err()
            );
            assert_eq!(
                span2.middle_pairs().len(),
                rspan.kernel.vertex_count(),
                "rule '{rule}': every kernel vertex contributes a middle pair"
            );
        }
    }

    // ── Kernel vertices that do not embed ──────────────────────────────

    /// A kernel vertex missing from a map, one whose image is not a vertex of
    /// that side's hypergraph, and two sharing an image — each on both sides:
    /// `to_span` errors rather than omitting or duplicating a pair.
    #[test]
    fn to_span_errors_on_kernel_vertices_that_do_not_embed() {
        let left = Hypergraph::from_edges(vec![vec![0, 1]]);
        let right = Hypergraph::from_edges(vec![vec![0, 1]]);
        let mut kernel = Hypergraph::new();
        for v in [0, 1, 2] {
            kernel.add_vertex(Some(v));
        }

        let unmapped = RewriteSpan {
            left: left.clone(),
            kernel: kernel.clone(),
            right: right.clone(),
            left_map: HashMap::from([(0, 0), (1, 1)]),
            right_map: HashMap::from([(0, 0), (1, 1), (2, 2)]),
        };
        assert_eq!(
            unmapped.to_span().err(),
            Some(RewriteSpanError::UnmappedKernelVertex {
                vertex: 2,
                side: SpanSide::Left,
            }),
            "kernel vertex 2 has no left_map entry; dropping it would leave 2 pairs for a 3-vertex kernel"
        );

        let mut two_vertex_kernel = Hypergraph::new();
        for v in [0, 1] {
            two_vertex_kernel.add_vertex(Some(v));
        }
        let stray = RewriteSpan {
            left: left.clone(),
            kernel: two_vertex_kernel.clone(),
            right: right.clone(),
            left_map: HashMap::from([(0, 0), (1, 1)]),
            right_map: HashMap::from([(0, 0), (1, 9)]),
        };
        assert_eq!(
            stray.to_span().err(),
            Some(RewriteSpanError::ImageNotAVertex {
                vertex: 1,
                image: 9,
                side: SpanSide::Right,
            }),
            "9 is not a vertex of R = {{0,1}}; dropping it would leave 1 pair for a 2-vertex kernel"
        );

        let unmapped_right = RewriteSpan {
            left: Hypergraph::from_edges(vec![vec![0, 1], vec![1, 2]]),
            kernel,
            right: right.clone(),
            left_map: HashMap::from([(0, 0), (1, 1), (2, 2)]),
            right_map: HashMap::from([(0, 0), (1, 1)]),
        };
        assert_eq!(
            unmapped_right.to_span().err(),
            Some(RewriteSpanError::UnmappedKernelVertex {
                vertex: 2,
                side: SpanSide::Right,
            }),
            "kernel vertex 2 has no right_map entry"
        );

        let stray_left = RewriteSpan {
            left,
            kernel: two_vertex_kernel,
            right,
            left_map: HashMap::from([(0, 0), (1, 9)]),
            right_map: HashMap::from([(0, 0), (1, 1)]),
        };
        assert_eq!(
            stray_left.to_span().err(),
            Some(RewriteSpanError::ImageNotAVertex {
                vertex: 1,
                image: 9,
                side: SpanSide::Left,
            }),
            "9 is not a vertex of L = {{0,1}}"
        );

        let wide = Hypergraph::from_edges(vec![vec![3, 5, 7]]);
        let mut spread_kernel = Hypergraph::new();
        for v in [3, 5] {
            spread_kernel.add_vertex(Some(v));
        }

        let collide_left = RewriteSpan {
            left: wide.clone(),
            kernel: spread_kernel.clone(),
            right: wide.clone(),
            left_map: HashMap::from([(3, 7), (5, 7)]),
            right_map: HashMap::from([(3, 3), (5, 5)]),
        };
        assert_eq!(
            collide_left.to_span().err(),
            Some(RewriteSpanError::NonInjectiveMap {
                vertex_a: 3,
                vertex_b: 5,
                image: 7,
                side: SpanSide::Left,
            }),
            "kernel vertices 3 and 5 share left image 7, at left index 2; pairing both would repeat (2, _)"
        );

        let collide_right = RewriteSpan {
            left: wide.clone(),
            kernel: spread_kernel,
            right: wide,
            left_map: HashMap::from([(3, 3), (5, 5)]),
            right_map: HashMap::from([(3, 7), (5, 7)]),
        };
        assert_eq!(
            collide_right.to_span().err(),
            Some(RewriteSpanError::NonInjectiveMap {
                vertex_a: 3,
                vertex_b: 5,
                image: 7,
                side: SpanSide::Right,
            }),
            "kernel vertices 3 and 5 share right image 7, at right index 2; pairing both would repeat (_, 2)"
        );
    }
}
