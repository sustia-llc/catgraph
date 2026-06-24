//! Multiway evolution with branchial foliation and Ollivier-Ricci curvature.
//!
//! Demonstrates generic multiway BFS exploration, branchial graph
//! extraction at each time step, and discrete curvature computation
//! via Wasserstein-1 optimal transport on the branchial structure.

use catgraph_physics::multiway::{
    BranchialGraph, DiscreteCurvature, OllivierRicciCurvature, extract_branchial_foliation,
    run_multiway_bfs,
};

// ============================================================================
// Branching System Definition
// ============================================================================

/// Successor function: each string state branches into two descendants.
///
/// "A"  -> [("AA", "dup", 0), ("AB", "mut", 1)]
/// "AA" -> [("AAA", "dup", 0), ("AAB", "mut", 1)]
/// etc.
///
/// States longer than 4 characters are terminal (no successors).
fn successors(state: &String) -> Vec<(String, String, usize)> {
    if state.len() > 4 {
        return vec![];
    }
    vec![
        (format!("{state}A"), "dup".to_string(), 0),
        (format!("{state}B"), "mut".to_string(), 1),
    ]
}

// ============================================================================
// Multiway BFS
// ============================================================================

fn multiway_bfs() {
    println!("=== Multiway BFS ===\n");

    let graph = run_multiway_bfs("A".to_string(), successors, 4, 64);

    let stats = graph.statistics();
    println!("nodes:    {}", stats.total_nodes);
    println!("edges:    {}", stats.total_edges);
    println!("branches: {}", stats.max_branches);
    println!("depth:    {}", stats.max_depth);
    println!("forks:    {}", stats.fork_count);
    println!("leaves:   {}", stats.leaf_count);
    println!();
}

// ============================================================================
// Branchial Foliation
// ============================================================================

fn branchial_foliation() {
    println!("=== Branchial Foliation ===\n");

    let graph = run_multiway_bfs("A".to_string(), successors, 4, 64);
    let foliation = extract_branchial_foliation(&graph);

    println!("time slices: {}", foliation.len());
    for bg in &foliation {
        println!(
            "  step {}: {} nodes, {} edges, connected: {}",
            bg.step,
            bg.node_count(),
            bg.edge_count(),
            bg.is_fully_connected()
        );
    }
    println!();
}

// ============================================================================
// Branchial Graph Details
// ============================================================================

fn branchial_details() {
    println!("=== Branchial Graph Details ===\n");

    let graph = run_multiway_bfs("A".to_string(), successors, 3, 32);
    let bg = BranchialGraph::from_evolution_at_step(&graph, 2);

    println!("step 2 branchial:");
    println!("  nodes:      {}", bg.node_count());
    println!("  edges:      {}", bg.edge_count());
    println!("  connected:  {}", bg.is_fully_connected());
    println!("  components: {}", bg.connected_components());

    let (_, matrix) = bg.adjacency_matrix();
    println!("  adjacency matrix ({}x{}):", matrix.len(), matrix.len());
    for row in &matrix {
        let bits: String = row.iter().map(|&b| if b { '1' } else { '0' }).collect();
        println!("    [{bits}]");
    }
    println!();
}

// ============================================================================
// Ollivier-Ricci Curvature
// ============================================================================

fn curvature() {
    println!("=== Ollivier-Ricci Curvature ===\n");

    let graph = run_multiway_bfs("A".to_string(), successors, 4, 64);

    // Compute curvature at each time step with >= 2 nodes
    for step in 0..=graph.max_step() {
        let orc = OllivierRicciCurvature::from_evolution_at_step(&graph, step);
        let scalar = orc.scalar_curvature();
        let flat = orc.is_flat();
        let complexity = orc.branchial_complexity();
        println!("  step {step}: scalar={scalar:+.4}, flat={flat}, complexity={complexity:.4}");
    }
    println!();
}

fn main() {
    multiway_bfs();
    branchial_foliation();
    branchial_details();
    curvature();
}
