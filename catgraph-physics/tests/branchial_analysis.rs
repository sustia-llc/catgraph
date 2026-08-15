#![allow(
    clippy::cast_possible_truncation,  // test fixture sizes are small and fit in i32
    clippy::cast_possible_wrap,
)]
// Exercises both the rustworkx-gated `branchial_coloring` / `branchial_core_numbers`
// and the spectral-gated `BranchialSpectrum` (issue #43), so it needs
// both features; excluded from `--no-default-features` builds and from builds that
// enable only one of the two. Spectrum-only coverage lives in the inline unit tests
// in `branchial_spectrum.rs`. Why the `rustworkx` gate exists at all: see
// `catgraph-physics/README.md` "Dependencies".
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
    use catgraph_physics::multiway::{BranchId, BranchialGraph, MultiwayNodeId};
    use proptest::prelude::*;
    use rustworkx_core::generators::{
        barabasi_albert_graph, gnp_random_graph, grid_graph, path_graph, petersen_graph,
        random_regular_graph,
    };
    use rustworkx_core::petgraph::graph::UnGraph;

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

    // --- Widened topologies (#163) ------------------------------------------
    //
    // `arb_branched_graph` above is the *evolution-level* strategy: one root
    // plus N forks. Every node at step 1 then shares the root as an ancestor,
    // so its branchial graph is always Kₙ — the proptests below it never see a
    // sparse, irregular, or disconnected shape.
    //
    // An arbitrary topology cannot be injected at the evolution level:
    // `MultiwayEvolutionGraph` is constructible only through `add_root` /
    // `add_fork` / `add_sequential_step`, and its fields are private. The
    // insertion point is one layer down — `BranchialGraph`'s `nodes` and
    // `edges` are public, so a generated `UnGraph` translates straight into
    // one. That still serves the same three properties (λ₂, zero-eigenvalue
    // multiplicity, proper coloring), because those are *branchial*
    // properties.
    //
    // Every seed below is a pinned literal, so a shrunk counterexample index
    // reproduces the exact graph. Topology only: numeric fixtures stay on the
    // byte-identity-pinned `catgraph-testutil` LCG (#33).

    /// Number of pinned topology fixtures; see [`topology_fixture`].
    const TOPOLOGY_FIXTURE_COUNT: usize = 7;

    /// Reinterpret a generated undirected graph as a branchial graph at `step`.
    ///
    /// Generated node `i` becomes `MultiwayNodeId::new(BranchId(i), step)`;
    /// generated edges become common-ancestor edges verbatim. The result is a
    /// `BranchialGraph` no `MultiwayEvolutionGraph` could produce, which is
    /// exactly the point.
    fn branchial_from_ungraph(step: usize, g: &UnGraph<(), ()>) -> BranchialGraph {
        let nodes: Vec<MultiwayNodeId> = (0..g.node_count())
            .map(|i| MultiwayNodeId::new(BranchId(i), step))
            .collect();
        let edges = g
            .edge_indices()
            .filter_map(|e| g.edge_endpoints(e))
            .map(|(a, b)| (nodes[a.index()], nodes[b.index()]))
            .collect();
        BranchialGraph { step, nodes, edges }
    }

    /// The `i`-th pinned topology, as a branchial graph at step 1.
    ///
    /// Indices are stable: `0` sparse Erdős–Rényi (usually disconnected),
    /// `1` dense Erdős–Rényi, `2` Barabási–Albert (scale-free hubs),
    /// `3` 3-regular, `4` path, `5` Petersen, `6` 4×6 grid.
    fn topology_fixture(i: usize) -> BranchialGraph {
        let g: UnGraph<(), ()> = match i {
            0 => gnp_random_graph(24, 0.06, Some(0x00C0_FFEE), || (), || ())
                .expect("gnp_random_graph(24, 0.06) has valid arguments"),
            1 => gnp_random_graph(30, 0.5, Some(0x0BAD_C0DE), || (), || ())
                .expect("gnp_random_graph(30, 0.5) has valid arguments"),
            2 => barabasi_albert_graph(40, 3, Some(0x0000_5EED), None, || (), || ())
                .expect("barabasi_albert_graph(40, 3) has valid arguments"),
            3 => random_regular_graph(20, 3, Some(0x0000_D1CE), || (), || ())
                .expect("random_regular_graph(20, 3) has an even node*degree product"),
            4 => path_graph(Some(25), None, || (), || (), false)
                .expect("path_graph(25) has valid arguments"),
            5 => petersen_graph(5, 2, || (), || ())
                .expect("petersen_graph(5, 2) is the Petersen graph"),
            _ => grid_graph(Some(4), Some(6), None, || (), || (), false)
                .expect("grid_graph(4, 6) has valid arguments"),
        };
        branchial_from_ungraph(1, &g)
    }

    /// Draw one of the pinned topologies. Shrinks toward fixture 0.
    fn arb_generated_branchial() -> impl Strategy<Value = (usize, BranchialGraph)> {
        (0..TOPOLOGY_FIXTURE_COUNT).prop_map(|i| (i, topology_fixture(i)))
    }

    /// The widening is non-vacuous: the fixture set really does leave the
    /// star / Kₙ regime that `arb_branched_graph` is confined to.
    ///
    /// Without this pin, every proptest below could still be running on
    /// complete graphs and nothing would fail.
    #[test]
    fn generated_topologies_escape_the_complete_graph_regime() {
        let all: Vec<BranchialGraph> = (0..TOPOLOGY_FIXTURE_COUNT).map(topology_fixture).collect();

        assert!(
            all.iter().any(|bg| bg.connected_components() > 1),
            "at least one fixture must be disconnected — Kₙ never is"
        );
        assert!(
            all.iter()
                .any(|bg| bg.connected_components() == 1 && !bg.is_fully_connected()),
            "at least one fixture must be connected but incomplete"
        );
        // Degree is not constant across the set (BA hubs vs the 3-regular and
        // Petersen fixtures), so the coloring / k-core paths see real variety.
        let max_degrees: std::collections::HashSet<usize> = all
            .iter()
            .map(|bg| {
                bg.nodes
                    .iter()
                    .map(|&n| bg.edges.iter().filter(|(a, b)| *a == n || *b == n).count())
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        assert!(
            max_degrees.len() >= 3,
            "expected ≥3 distinct max-degrees across fixtures, got {max_degrees:?}"
        );
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

        /// Same two spectral properties as the two proptests above, but over
        /// sparse / irregular / disconnected topologies rather than Kₙ (#163).
        #[test]
        fn generated_topology_spectral_invariants((i, branchial) in arb_generated_branchial()) {
            let spectrum = BranchialSpectrum::from_branchial(&branchial);
            let components = branchial.connected_components();

            prop_assert_eq!(
                spectrum.connected_components(),
                components,
                "fixture {}: zero-eigenvalue multiplicity must equal the component count",
                i
            );

            // λ₂ > 0 exactly when the graph is connected. On Kₙ the guard was
            // `is_fully_connected()`, which no sparse fixture would ever pass.
            if components == 1 && branchial.node_count() >= 2 {
                prop_assert!(
                    spectrum.algebraic_connectivity() > 0.0,
                    "fixture {}: connected graph must have λ₂ > 0, got {}",
                    i,
                    spectrum.algebraic_connectivity()
                );
            }
        }

        /// Coloring / k-core / articulation-point invariants on the widened
        /// topologies (#163).
        #[test]
        fn generated_topology_graph_algorithm_invariants((i, branchial) in arb_generated_branchial()) {
            let coloring = branchial_coloring(&branchial);
            let cores = branchial_core_numbers(&branchial);

            for (a, b) in &branchial.edges {
                prop_assert_ne!(
                    coloring.get(a),
                    coloring.get(b),
                    "fixture {}: adjacent nodes must have different colors",
                    i
                );
            }

            for &node in &branchial.nodes {
                let degree = branchial
                    .edges
                    .iter()
                    .filter(|(a, b)| *a == node || *b == node)
                    .count();
                let core = cores.get(&node).copied().unwrap_or(0);
                prop_assert!(
                    core <= degree,
                    "fixture {}: core number {} exceeds degree {}",
                    i,
                    core,
                    degree
                );
            }

            // Articulation points are a subset of the node set, and a
            // biconnected graph has none. Both are only meaningful once the
            // topologies stop being complete.
            let artics = catgraph_physics::multiway::branchial_articulation_points(&branchial);
            for a in &artics {
                prop_assert!(
                    branchial.nodes.contains(a),
                    "fixture {}: articulation point {:?} is not a node of the graph",
                    i,
                    a
                );
            }
        }
    }
}
