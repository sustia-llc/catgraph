//! Integration tests for the catgraph categorical bridge.
//!
//! Verifies `RewriteRule` -> Span conversion, `HypergraphEvolution` -> Cospan chain,
//! cospan composability, and span property validation using catgraph's Composable trait.

use catgraph_physics::hypergraph::{Hypergraph, HypergraphEvolution, RewriteRule};

// ---------------------------------------------------------------------------
// Rule to Span conversion
// ---------------------------------------------------------------------------

#[test]
fn wolfram_a_to_bb_rule_to_span() {
    let rule = RewriteRule::wolfram_a_to_bb();
    let span = rule.to_span();

    // L has variables {0, 1, 2}, R has variables {0, 1, 2}
    assert_eq!(span.left(), &[0u32, 1, 2]);
    assert_eq!(span.right(), &[0u32, 1, 2]);

    // K = {0, 1, 2} (all preserved) -> 3 middle pairs
    assert_eq!(span.middle_pairs().len(), 3);
}

#[test]
fn edge_split_rule_to_span() {
    // {0, 1} -> {0, 2}, {2, 1}
    let rule = RewriteRule::edge_split();
    let span = rule.to_span();

    // Left variables: {0, 1}, Right variables: {0, 1, 2}
    assert_eq!(span.left().len(), 2);
    assert_eq!(span.right().len(), 3);

    // Only {0, 1} are preserved (appear in both sides)
    let preserved = rule.preserved_variables();
    assert!(preserved.contains(&0));
    assert!(preserved.contains(&1));

    // Variable 2 is created (only on right)
    let created = rule.created_variables();
    assert!(created.contains(&2));
}

#[test]
fn rule_to_rewrite_span_and_back_to_catgraph_span() {
    let rule = RewriteRule::wolfram_a_to_bb();

    // Two paths to a Span: direct and through RewriteSpan
    let direct_span = rule.to_span();
    let rewrite_span = rule.to_rewrite_span();
    let indirect_span = rewrite_span.to_span();

    // Both should produce the same left and right element counts
    assert_eq!(direct_span.left().len(), indirect_span.left().len());
    assert_eq!(direct_span.right().len(), indirect_span.right().len());
    assert_eq!(
        direct_span.middle_pairs().len(),
        indirect_span.middle_pairs().len()
    );
}

// ---------------------------------------------------------------------------
// Evolution to Cospan chain
// ---------------------------------------------------------------------------

#[test]
fn deterministic_evolution_to_cospan_chain() {
    let rule = RewriteRule::wolfram_a_to_bb();
    let graph = Hypergraph::from_edges(vec![vec![0, 1, 2]]);

    let evolution = HypergraphEvolution::run(&graph, &[rule], 3);
    let cospans = evolution.to_cospan_chain();

    // Should have one cospan per evolution step (along deterministic path)
    assert!(!cospans.is_empty());

    // Each cospan should have non-empty left and right boundaries
    for cospan in &cospans {
        assert!(!cospan.left_to_middle().is_empty());
        assert!(!cospan.right_to_middle().is_empty());
        assert!(!cospan.middle().is_empty());
    }
}

#[test]
fn cospan_chain_composability_contiguous_steps() {
    let rule = RewriteRule::wolfram_a_to_bb();
    let graph = Hypergraph::from_edges(vec![vec![0, 1, 2]]);

    let evolution = HypergraphEvolution::run(&graph, &[rule], 3);
    let cospans = evolution.to_cospan_chain();

    if cospans.len() >= 2 {
        // Right boundary of cospan i should match left boundary of cospan i+1
        for i in 0..cospans.len() - 1 {
            assert_eq!(
                cospans[i].right_to_middle().len(),
                cospans[i + 1].left_to_middle().len(),
                "Boundary mismatch between cospan {} and {}",
                i,
                i + 1
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Span properties
// ---------------------------------------------------------------------------

#[test]
fn span_left_right_sizes_match_rule_structure() {
    // Rule: {0,1,2} -> {0,1},{1,2}
    // Left unique vars: {0,1,2} = 3
    // Right unique vars: {0,1,2} = 3
    let rule = RewriteRule::wolfram_a_to_bb();
    let span = rule.to_span();

    assert_eq!(span.left().len(), 3);
    assert_eq!(span.right().len(), 3);

    // Self-loop creation: {0,1} -> {0,1},{1,1}
    // Left: {0,1} = 2, Right: {0,1} = 2
    let loop_rule = RewriteRule::create_self_loop();
    let loop_span = loop_rule.to_span();

    assert_eq!(loop_span.left().len(), 2);
    assert_eq!(loop_span.right().len(), 2);
}

// ---------------------------------------------------------------------------
// Cospan labels and middle structure
// ---------------------------------------------------------------------------

#[test]
fn cospan_middle_represents_vertex_union() {
    let rule = RewriteRule::wolfram_a_to_bb();
    let graph = Hypergraph::from_edges(vec![vec![0, 1, 2]]);

    let evolution = HypergraphEvolution::run(&graph, &[rule], 1);
    let cospans = evolution.to_cospan_chain();

    if !cospans.is_empty() {
        let cospan = &cospans[0];

        // The middle (apex) should contain labels that are vertex IDs
        // from the union of parent and child states
        assert!(!cospan.middle().is_empty());

        // Left boundary maps parent vertices into the apex
        // Right boundary maps child vertices into the apex
        assert!(!cospan.left_to_middle().is_empty());
        assert!(!cospan.right_to_middle().is_empty());
    }
}

#[test]
fn single_step_evolution_produces_one_cospan() {
    let rule = RewriteRule::wolfram_a_to_bb();
    let graph = Hypergraph::from_edges(vec![vec![0, 1, 2]]);

    let evolution = HypergraphEvolution::run(&graph, &[rule], 1);
    let cospans = evolution.to_cospan_chain();

    assert_eq!(cospans.len(), 1);
}

#[test]
fn no_steps_produces_empty_cospan_chain() {
    let rule = RewriteRule::wolfram_a_to_bb();
    // No matching edges -> no evolution steps
    let graph = Hypergraph::from_edges(vec![vec![0, 1]]); // binary, rule needs ternary

    let evolution = HypergraphEvolution::run(&graph, &[rule], 3);
    let cospans = evolution.to_cospan_chain();

    // No steps means no cospans
    assert!(cospans.is_empty());
}
