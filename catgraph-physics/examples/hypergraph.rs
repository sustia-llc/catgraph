//! Hypergraph DPO rewriting and evolution tracking.
//!
//! Demonstrates hypergraph construction, pattern matching,
//! Double-Pushout (DPO) rewrite rules, deterministic evolution,
//! and the cospan chain bridge for category-theoretic composition.

use catgraph::category::Composable;
use catgraph_physics::hypergraph::{Hypergraph, HypergraphEvolution, RewriteRule};

// ============================================================================
// Construction
// ============================================================================

fn construction() {
    println!("=== Construction ===\n");

    let graph = Hypergraph::from_edges(vec![vec![0, 1, 2], vec![2, 3], vec![3, 4, 5]]);

    println!("vertices:  {}", graph.vertex_count());
    println!("edges:     {}", graph.edge_count());
    println!();
}

// ============================================================================
// Pattern Matching
// ============================================================================

fn pattern_matching() {
    println!("=== Pattern Matching ===\n");

    let graph = Hypergraph::from_edges(vec![vec![0, 1, 2], vec![3, 4, 5]]);

    let rule = RewriteRule::wolfram_a_to_bb();
    println!("rule: {rule}");
    println!("left arity:   {}", rule.left_arity());
    println!("right arity:  {}", rule.right_arity());
    println!("variables:    {}", rule.num_variables());

    let matches = rule.find_matches(&graph);
    println!("matches found: {}", matches.len());
    println!();
}

// ============================================================================
// DPO Rewriting
// ============================================================================

fn dpo_rewriting() {
    println!("=== DPO Rewriting ===\n");

    let mut graph = Hypergraph::from_edges(vec![vec![0, 1, 2]]);
    let rule = RewriteRule::wolfram_a_to_bb();

    println!(
        "before: {} vertices, {} edges",
        graph.vertex_count(),
        graph.edge_count()
    );

    let matches = rule.find_matches(&graph);
    let mut next_id = graph.vertex_count();
    let new_vars = rule.apply(&mut graph, &matches[0], &mut next_id);

    println!(
        "after:  {} vertices, {} edges",
        graph.vertex_count(),
        graph.edge_count()
    );
    println!("new variables created: {}", new_vars.len());

    // Edge-split rule: inserts a midpoint vertex
    let mut graph2 = Hypergraph::from_edges(vec![vec![0, 1]]);
    let split = RewriteRule::edge_split();
    println!("\nrule: {split}");

    let matches2 = split.find_matches(&graph2);
    let mut next_id2 = graph2.vertex_count();
    let new2 = split.apply(&mut graph2, &matches2[0], &mut next_id2);

    println!(
        "split {{0,1}}: {} edges, new vertex IDs: {:?}",
        graph2.edge_count(),
        new2
    );
    println!();
}

// ============================================================================
// Evolution Tracking
// ============================================================================

fn evolution() {
    println!("=== Evolution Tracking ===\n");

    // edge_split chains: {x,y} -> {x,z},{z,y} produces new binary edges
    let initial = Hypergraph::from_edges(vec![vec![0, 1]]);
    let rules = vec![RewriteRule::edge_split()];

    let evo = HypergraphEvolution::run(&initial, &rules, 3);
    println!("steps:  {}", evo.max_step());
    println!("nodes:  {}", evo.node_count());
    println!("leaves: {}", evo.leaves().len());

    // Show state at each step along the deterministic path
    for step in 0..=evo.max_step() {
        let ids = evo.nodes_at_step(step);
        if let Some(&id) = ids.first() {
            let node = evo.get_node(id).unwrap();
            println!(
                "  step {step}: {} vertices, {} edges",
                node.state.vertex_count(),
                node.state.edge_count()
            );
        }
    }
    println!();
}

// ============================================================================
// Cospan Bridge
// ============================================================================

fn cospan_bridge() {
    println!("=== Cospan Bridge ===\n");

    let initial = Hypergraph::from_edges(vec![vec![0, 1]]);
    let rules = vec![RewriteRule::edge_split()];
    let evo = HypergraphEvolution::run(&initial, &rules, 3);

    let chain = evo.to_cospan_chain();
    println!("cospan chain length: {}", chain.len());

    // Show domain/codomain sizes for each cospan in the chain
    for (i, c) in chain.iter().enumerate() {
        println!(
            "  cospan {i}: domain {:?} -> codomain {:?}, middle len {}",
            c.domain(),
            c.codomain(),
            c.middle().len()
        );
    }

    // Compose the first two cospans if the chain is long enough
    if chain.len() >= 2 {
        let composed = chain[0].compose(&chain[1]);
        match composed {
            Ok(c) => println!("\ncomposed [0]+[1]: middle len {}", c.middle().len()),
            Err(e) => println!("\ncomposition failed: {e}"),
        }
    }
    println!();
}

fn main() {
    construction();
    pattern_matching();
    dpo_rewriting();
    evolution();
    cospan_bridge();
}
