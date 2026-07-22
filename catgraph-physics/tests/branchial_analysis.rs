#![allow(
    clippy::cast_possible_truncation,  // test fixture sizes are small and fit in i32
    clippy::cast_possible_wrap,
)]
// Exercises both the rustworkx-gated `branchial_coloring` / `branchial_core_numbers`
// (issue #10) and the spectral-gated `BranchialSpectrum` (issue #43), so it needs
// both features; excluded from `--no-default-features` builds and from builds that
// enable only one of the two. Spectrum-only coverage lives in the inline unit tests
// in `branchial_spectrum.rs`.
#![cfg(all(feature = "rustworkx", feature = "spectral"))]

//! Integration tests for branchial analysis on real multiway evolutions.

use catgraph_physics::multiway::{
    BranchialSpectrum, MultiwayEvolutionGraph, branchial_coloring, branchial_core_numbers,
    extract_branchial_foliation,
};

/// Build a string-rewriting multiway system with branching at each step.
fn build_string_rewrite_multiway() -> MultiwayEvolutionGraph<String, String> {
    let mut graph = MultiwayEvolutionGraph::new();
    let root = graph.add_root("A".to_string());

    // Step 1: "A" → "AB" or "A" → "BA"
    let step1 = graph.add_fork(
        root,
        vec![
            ("AB".to_string(), "A→AB".to_string(), 0),
            ("BA".to_string(), "A→BA".to_string(), 1),
        ],
    );

    // Step 2: each branch forks again
    let _step2a = graph.add_fork(
        step1[0],
        vec![
            ("ABB".to_string(), "A→AB".to_string(), 0),
            ("BAB".to_string(), "A→BA".to_string(), 1),
        ],
    );

    let _step2b = graph.add_fork(
        step1[1],
        vec![
            ("BAB2".to_string(), "A→AB".to_string(), 0),
            ("BBA".to_string(), "A→BA".to_string(), 1),
        ],
    );

    graph
}

#[test]
fn branchial_spectrum_across_foliation() {
    let graph = build_string_rewrite_multiway();
    let foliation = extract_branchial_foliation(&graph);

    // Step 0: 1 node
    let spec0 = BranchialSpectrum::from_branchial(&foliation[0]);
    assert_eq!(spec0.eigenvalues.len(), 1);

    // Step 1: 2 nodes, both share root → K₂, λ₂ = 2.0
    let spec1 = BranchialSpectrum::from_branchial(&foliation[1]);
    assert!((spec1.algebraic_connectivity() - 2.0).abs() < 1e-9);
    assert_eq!(spec1.connected_components(), 1);

    // Step 2: 4 nodes, branching structure
    if foliation.len() > 2 {
        let spec2 = BranchialSpectrum::from_branchial(&foliation[2]);
        assert!(spec2.algebraic_connectivity() > 0.0, "should be connected");
        assert_eq!(spec2.connected_components(), 1);
    }
}

#[test]
fn coloring_valid_across_foliation() {
    let graph = build_string_rewrite_multiway();
    let foliation = extract_branchial_foliation(&graph);

    for bg in &foliation {
        if bg.node_count() < 2 {
            continue;
        }
        let coloring = branchial_coloring(bg);

        for (a, b) in &bg.edges {
            assert_ne!(
                coloring.get(a),
                coloring.get(b),
                "adjacent nodes at step {} must have different colors",
                bg.step
            );
        }
    }
}

#[test]
fn core_numbers_consistent_with_degree() {
    let graph = build_string_rewrite_multiway();
    let foliation = extract_branchial_foliation(&graph);

    for bg in &foliation {
        if bg.node_count() < 2 {
            continue;
        }
        let cores = branchial_core_numbers(bg);

        for &node in &bg.nodes {
            let degree = bg
                .edges
                .iter()
                .filter(|(a, b)| *a == node || *b == node)
                .count();
            let core = cores.get(&node).copied().unwrap_or(0);
            assert!(
                core <= degree,
                "core number {core} exceeds degree {degree} for node {node:?} at step {}",
                bg.step
            );
        }
    }
}

mod proptests {
    use super::*;
    use catgraph_physics::multiway::BranchialGraph;
    use proptest::prelude::*;

    /// Generate a multiway graph with `n_forks` children from a single root.
    fn arb_branched_graph(
        max_forks: usize,
    ) -> impl Strategy<Value = MultiwayEvolutionGraph<i32, ()>> {
        (1..=max_forks).prop_map(|n_forks| {
            let mut graph = MultiwayEvolutionGraph::new();
            let root = graph.add_root(0);
            let states: Vec<(i32, (), usize)> =
                (0..n_forks).map(|i| (i as i32 + 1, (), i)).collect();
            graph.add_fork(root, states);
            graph
        })
    }

    proptest! {
        #[test]
        fn connected_branchial_has_positive_lambda2(graph in arb_branched_graph(10)) {
            let branchial = BranchialGraph::from_evolution_at_step(&graph, 1);
            if branchial.node_count() >= 2 && branchial.is_fully_connected() {
                let spectrum = BranchialSpectrum::from_branchial(&branchial);
                prop_assert!(spectrum.algebraic_connectivity() > 0.0);
            }
        }

        #[test]
        fn zero_eigenvalue_multiplicity_equals_components(graph in arb_branched_graph(10)) {
            let branchial = BranchialGraph::from_evolution_at_step(&graph, 1);
            if branchial.node_count() >= 1 {
                let spectrum = BranchialSpectrum::from_branchial(&branchial);
                let spectral_components = spectrum.connected_components();
                let graph_components = branchial.connected_components();
                prop_assert_eq!(spectral_components, graph_components);
            }
        }

        #[test]
        fn coloring_valid_no_adjacent_same_color(graph in arb_branched_graph(10)) {
            let branchial = BranchialGraph::from_evolution_at_step(&graph, 1);
            if branchial.node_count() >= 2 {
                let coloring = branchial_coloring(&branchial);
                for (a, b) in &branchial.edges {
                    prop_assert_ne!(
                        coloring.get(a),
                        coloring.get(b),
                        "adjacent nodes must have different colors"
                    );
                }
            }
        }
    }
}
