#![allow(
    clippy::cast_possible_wrap,        // test fixture sizes fit in i32 by construction
    clippy::similar_names,             // algebraic pairings produce near-identical binding names
)]

//! Integration tests for `catgraph_physics::multiway` module.
//!
//! Tests the generic multiway evolution infrastructure: graph construction,
//! branchial foliation, discrete curvature, and Ollivier-Ricci analysis.
//! Uses simple integer states and trivial transitions to exercise the API
//! without domain-specific computation models.

use catgraph_physics::multiway::{
    BranchId, BranchialGraph, BranchialSummary, DiscreteCurvature, MultiwayEvolutionGraph,
    MultiwayNodeId, OllivierFoliation, OllivierRicciCurvature, branchial_parallel_step_pairs,
    extract_branchial_foliation, run_multiway_bfs, wasserstein_1,
};

// ---------------------------------------------------------------------------
// Graph construction
// ---------------------------------------------------------------------------

#[test]
fn graph_construction_add_root_sequential_fork() {
    let mut graph: MultiwayEvolutionGraph<i32, &str> = MultiwayEvolutionGraph::new();

    // Add root
    let root = graph.add_root(0);
    assert_eq!(graph.node_count(), 1);
    assert_eq!(root.step, 0);
    assert_eq!(root.branch_id, BranchId(0));

    // Sequential step
    let n1 = graph.add_sequential_step(root, 1, "step");
    assert_eq!(graph.node_count(), 2);
    assert_eq!(n1.step, 1);
    assert_eq!(n1.branch_id, root.branch_id);

    // Fork into 3 branches
    let branches = graph.add_fork(n1, vec![(10, "left", 0), (20, "mid", 1), (30, "right", 2)]);
    assert_eq!(branches.len(), 3);
    assert_eq!(graph.node_count(), 5); // root + n1 + 3 fork children
    assert_eq!(graph.edge_count(), 4); // root->n1 + 3 fork edges
    assert!(graph.is_fork_point(&n1));

    let stats = graph.statistics();
    assert_eq!(stats.total_nodes, 5);
    assert_eq!(stats.total_edges, 4);
    assert_eq!(stats.fork_count, 1);
    assert_eq!(stats.leaf_count, 3);
    assert_eq!(stats.root_count, 1);
    assert_eq!(stats.max_depth, 2);
}

// ---------------------------------------------------------------------------
// run_multiway_bfs
// ---------------------------------------------------------------------------

#[test]
fn run_multiway_bfs_binary_branching() {
    // Each integer state n produces two successors: 2n+1 and 2n+2 (binary tree).
    let graph: MultiwayEvolutionGraph<i32, ()> = run_multiway_bfs(
        0_i32,
        |&n| vec![(2 * n + 1, (), 0), (2 * n + 2, (), 1)],
        3,   // max_steps (depth 3)
        100, // generous branch budget
    );

    // At depth 0: 1 node (root=0)
    // At depth 1: 2 nodes (1, 2)
    // At depth 2: 4 nodes (3,4,5,6)
    // At depth 3: 8 nodes (7..14)
    // Total = 1 + 2 + 4 + 8 = 15
    assert_eq!(graph.node_count(), 15);
    assert_eq!(graph.max_step(), 3);

    let stats = graph.statistics();
    assert!(stats.fork_count > 0, "binary tree should have fork points");
    assert_eq!(stats.leaf_count, 8, "8 leaves at depth 3");
}

#[test]
fn run_multiway_bfs_deterministic_chain() {
    // Single successor per state: produces a linear chain.
    let graph: MultiwayEvolutionGraph<i32, &str> = run_multiway_bfs(
        0_i32,
        |&n| {
            if n < 5 {
                vec![(n + 1, "inc", 0)]
            } else {
                vec![] // terminal
            }
        },
        10,
        10,
    );

    // States 0, 1, 2, 3, 4, 5 (6 nodes, 5 edges, no forks)
    assert_eq!(graph.node_count(), 6);
    assert_eq!(graph.edge_count(), 5);
    assert_eq!(graph.statistics().fork_count, 0);
}

#[test]
fn run_multiway_bfs_respects_branch_limit() {
    // Each state produces 3 successors: exponential growth.
    // Without a limit, 5 steps of ternary branching would produce 3^5 = 243 leaves.
    // With a budget of 10, growth should be substantially curtailed.
    let graph: MultiwayEvolutionGraph<i32, ()> = run_multiway_bfs(
        0_i32,
        |&n| {
            vec![
                (n * 10 + 1, (), 0),
                (n * 10 + 2, (), 1),
                (n * 10 + 3, (), 2),
            ]
        },
        5,
        10, // branch budget
    );

    // The budget limits how many forks are explored from the frontier, but
    // items already queued may still be processed. Verify the graph is
    // substantially smaller than the unbounded case (243 leaves).
    assert!(
        graph.statistics().leaf_count < 50,
        "leaf count {} should be much less than unbounded 243",
        graph.statistics().leaf_count
    );
}

// ---------------------------------------------------------------------------
// Branchial foliation
// ---------------------------------------------------------------------------

#[test]
fn branchial_foliation_from_forking_graph() {
    let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
    let root = graph.add_root(0);
    let ids = graph.add_fork(root, vec![(1, (), 0), (2, (), 1), (3, (), 2)]);

    // Extend one branch further
    graph.add_sequential_step(ids[0], 10, ());

    let foliation = extract_branchial_foliation(&graph);
    // Steps 0, 1, 2
    assert_eq!(foliation.len(), 3);
    assert_eq!(foliation[0].node_count(), 1); // root
    assert_eq!(foliation[1].node_count(), 3); // 3 fork children
    assert_eq!(foliation[2].node_count(), 1); // only ids[0]'s continuation

    // At step 1, all 3 nodes share a common ancestor (root), so fully connected.
    assert!(foliation[1].is_fully_connected());
}

// ---------------------------------------------------------------------------
// BranchialSummary
// ---------------------------------------------------------------------------

#[test]
fn branchial_summary_peak_branching() {
    // Build: root -> fork(4 branches) -> one branch extends
    let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
    let root = graph.add_root(0);
    let ids = graph.add_fork(root, vec![(1, (), 0), (2, (), 1), (3, (), 2), (4, (), 3)]);
    graph.add_sequential_step(ids[0], 10, ());

    let foliation = extract_branchial_foliation(&graph);
    let summary = BranchialSummary::from_foliation(&foliation);

    assert_eq!(summary.max_parallel_branches, 4);
    assert_eq!(summary.peak_branching_step, 1);
    assert!(
        summary.average_branches > 1.0,
        "average branches should exceed 1.0 for a forking graph"
    );
    // Step 0 has 1 node (trivially connected), step 1 has 4 (fully connected)
    assert!(summary.fully_connected_steps >= 2);
}

// ---------------------------------------------------------------------------
// branchial_parallel_step_pairs
// ---------------------------------------------------------------------------

#[test]
fn parallel_step_pairs_match_branch_counts() {
    let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
    let root = graph.add_root(0);
    let ids = graph.add_fork(root, vec![(1, (), 0), (2, (), 1)]);
    // Extend both branches one more step
    graph.add_sequential_step(ids[0], 10, ());
    graph.add_sequential_step(ids[1], 20, ());

    let parallel = branchial_parallel_step_pairs(&graph);

    // Foliation has steps 0, 1, 2 -> two windows: [0->1] and [1->2]
    assert_eq!(parallel.len(), 2);

    // Window [0->1]: root has outgoing edges -> 1 pair
    assert_eq!(parallel[0].len(), 1);
    assert_eq!(parallel[0][0], (0, 1));

    // Window [1->2]: both fork children have outgoing edges -> 2 pairs
    assert_eq!(parallel[1].len(), 2);
    assert!(parallel[1].iter().all(|&p| p == (1, 2)));
}

// ---------------------------------------------------------------------------
// OllivierRicciCurvature
// ---------------------------------------------------------------------------

#[test]
fn ollivier_ricci_from_forking_evolution() {
    let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
    let root = graph.add_root(0);
    graph.add_fork(root, vec![(1, (), 0), (2, (), 1), (3, (), 2)]);

    // Curvature at step 1 (3 fully connected nodes from common ancestor)
    let curv = OllivierRicciCurvature::from_evolution_at_step(&graph, 1);

    assert_eq!(curv.dimension(), 3);
    assert_eq!(curv.step(), 1);
    // K_3 (complete graph on 3 vertices): positive curvature
    assert!(
        curv.scalar_curvature() > 0.0,
        "K3 should have positive scalar curvature, got {}",
        curv.scalar_curvature()
    );
    assert!(curv.irreducibility_indicator() >= 0.0);
}

#[test]
fn ollivier_ricci_single_node_is_flat() {
    let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
    graph.add_root(42);

    let curv = OllivierRicciCurvature::from_evolution_at_step(&graph, 0);

    assert_eq!(curv.dimension(), 1);
    assert!(curv.is_flat());
    assert!((curv.scalar_curvature()).abs() < 1e-10);
}

// ---------------------------------------------------------------------------
// OllivierFoliation
// ---------------------------------------------------------------------------

#[test]
fn ollivier_foliation_from_evolution() {
    // Build a small forking evolution: root -> 3 branches -> each extends 1 step
    let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
    let root = graph.add_root(0);
    let ids = graph.add_fork(root, vec![(1, (), 0), (2, (), 1), (3, (), 2)]);
    for (i, &id) in ids.iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        graph.add_sequential_step(id, (i as i32 + 1) * 10, ());
    }

    let foliation = OllivierFoliation::from_evolution(&graph);

    // Steps 0, 1, 2 -> 3 curvature values
    assert_eq!(foliation.curvatures.len(), 3);

    // Step 0: single node -> flat
    assert!(foliation.curvatures[0].is_flat());

    // Step 1: K_3 -> positive curvature -> not flat
    assert!(!foliation.curvatures[1].is_flat());

    // average_irreducibility should be non-negative
    assert!(foliation.average_irreducibility() >= 0.0);

    // Not globally flat (step 1 has curvature)
    assert!(!foliation.is_globally_flat());
}

// ---------------------------------------------------------------------------
// CurvatureFoliation (generic)
// ---------------------------------------------------------------------------

#[test]
fn curvature_foliation_irreducibility_profile_length() {
    // Build a 5-step deterministic chain, then check profile length
    let graph: MultiwayEvolutionGraph<i32, ()> = run_multiway_bfs(
        0_i32,
        |&n| {
            if n < 4 { vec![(n + 1, (), 0)] } else { vec![] }
        },
        10,
        10,
    );

    let foliation = OllivierFoliation::from_evolution(&graph);
    let profile = foliation.irreducibility_profile();

    // max_step = 4, so steps 0..=4 -> 5 entries
    assert_eq!(profile.len(), 5);

    // All deterministic (1 node per step) -> all flat -> all indicators = 0
    for (step, &ind) in profile.iter().enumerate() {
        assert!(
            ind.abs() < 1e-10,
            "step {step}: deterministic chain should have indicator ~0, got {ind}"
        );
    }
}

// ---------------------------------------------------------------------------
// Wasserstein W1
// ---------------------------------------------------------------------------

#[test]
fn wasserstein_1_known_distributions() {
    // Dirac at 0 vs Dirac at 2 with distance matrix d(i,j) = |i - j|
    let mu = vec![1.0, 0.0, 0.0];
    let nu = vec![0.0, 0.0, 1.0];
    let dist = vec![
        vec![0.0, 1.0, 2.0],
        vec![1.0, 0.0, 1.0],
        vec![2.0, 1.0, 0.0],
    ];

    let w1 = wasserstein_1(&mu, &nu, &dist);
    assert!(
        (w1 - 2.0).abs() < 1e-9,
        "W1(delta_0, delta_2) should be 2.0, got {w1}"
    );

    // Symmetry
    let w1_rev = wasserstein_1(&nu, &mu, &dist);
    assert!(
        (w1 - w1_rev).abs() < 1e-9,
        "W1 should be symmetric: {w1} vs {w1_rev}"
    );

    // Self-distance = 0
    let w1_self = wasserstein_1(&mu, &mu, &dist);
    assert!(
        w1_self.abs() < 1e-9,
        "W1(mu, mu) should be 0, got {w1_self}"
    );
}

#[test]
fn wasserstein_1_triangle_inequality() {
    let mu = vec![0.5, 0.3, 0.2];
    let nu = vec![0.2, 0.5, 0.3];
    let rho = vec![0.1, 0.1, 0.8];
    let dist = vec![
        vec![0.0, 1.0, 2.0],
        vec![1.0, 0.0, 1.0],
        vec![2.0, 1.0, 0.0],
    ];

    let w_mn = wasserstein_1(&mu, &nu, &dist);
    let w_nr = wasserstein_1(&nu, &rho, &dist);
    let w_mr = wasserstein_1(&mu, &rho, &dist);

    assert!(
        w_mr <= w_mn + w_nr + 1e-9,
        "Triangle inequality: W(mu,rho)={w_mr} > W(mu,nu)+W(nu,rho)={}",
        w_mn + w_nr
    );
}

// ---------------------------------------------------------------------------
// Cycles across branches
// ---------------------------------------------------------------------------

#[test]
fn cycles_across_branches_detected() {
    let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
    let root = graph.add_root(100);

    // Fork into two branches with different intermediate states
    let ids = graph.add_fork(root, vec![(1, (), 0), (2, (), 1)]);

    // Both branches converge to the same state (42)
    graph.add_sequential_step(ids[0], 42, ());
    graph.add_sequential_step(ids[1], 42, ());

    let cycles = graph.find_cycles_across_branches();
    // The two nodes with state=42 share the same fingerprint
    assert!(
        !cycles.is_empty(),
        "should detect cycle when two branches reach the same state"
    );

    // Verify the cycle involves different branches
    let cross_branch = cycles.iter().any(|c: &_| !c.is_same_branch());
    assert!(
        cross_branch,
        "at least one cycle should span different branches"
    );
}

// ---------------------------------------------------------------------------
// BranchialGraph API
// ---------------------------------------------------------------------------

#[test]
fn branchial_graph_adjacency_and_components() {
    // Two independent roots -> disconnected branchial graph at step 1
    let mut graph: MultiwayEvolutionGraph<i32, ()> = MultiwayEvolutionGraph::new();
    let r1 = graph.add_root(0);
    let r2 = graph.add_root(100);
    graph.add_sequential_step(r1, 1, ());
    graph.add_sequential_step(r2, 101, ());

    let branchial = BranchialGraph::from_evolution_at_step(&graph, 1);

    assert_eq!(branchial.node_count(), 2);
    assert_eq!(branchial.edge_count(), 0); // no common ancestor
    assert!(!branchial.is_fully_connected());
    assert_eq!(branchial.connected_components(), 2);

    let (nodes, matrix) = branchial.adjacency_matrix();
    assert_eq!(nodes.len(), 2);
    assert!(!matrix[0][1]);
    assert!(!matrix[1][0]);
}

// ---------------------------------------------------------------------------
// Multiway node/branch identity
// ---------------------------------------------------------------------------

#[test]
fn node_and_branch_id_display() {
    let bid = BranchId(7);
    assert_eq!(format!("{bid}"), "B7");

    let nid = MultiwayNodeId::new(BranchId(3), 5);
    assert_eq!(format!("{nid}"), "B3@5");
}

// ---------------------------------------------------------------------------
// End-to-end: BFS -> branchial -> curvature pipeline
// ---------------------------------------------------------------------------

#[test]
fn end_to_end_bfs_to_curvature_pipeline() {
    // Binary branching BFS -> extract branchial foliation -> compute curvature
    let graph: MultiwayEvolutionGraph<i32, ()> = run_multiway_bfs(
        0_i32,
        |&n| vec![(2 * n + 1, (), 0), (2 * n + 2, (), 1)],
        2, // 2 steps deep
        100,
    );

    // Foliation
    let foliation = extract_branchial_foliation(&graph);
    assert_eq!(foliation.len(), 3); // steps 0, 1, 2

    // Summary
    let summary = BranchialSummary::from_foliation(&foliation);
    assert_eq!(summary.max_parallel_branches, 4); // 4 nodes at depth 2
    assert_eq!(summary.peak_branching_step, 2);

    // Curvature foliation
    let curv_foliation = OllivierFoliation::from_evolution(&graph);
    assert_eq!(curv_foliation.curvatures.len(), 3);

    // Step 0: 1 node -> flat
    assert!(curv_foliation.curvatures[0].is_flat());

    // Step 2: 4 nodes fully connected (all share root) -> positive curvature
    let curv_step2 = &curv_foliation.curvatures[2];
    assert_eq!(curv_step2.dimension(), 4);
    assert!(
        curv_step2.scalar_curvature() > 0.0,
        "K4 branchial graph should have positive scalar curvature"
    );

    // Profile has correct length
    let profile = curv_foliation.irreducibility_profile();
    assert_eq!(profile.len(), 3);
}
