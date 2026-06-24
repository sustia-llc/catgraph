//! Lattice gauge theory for hypergraph rewriting.
//!
//! Demonstrates gauge groups, structure constants, lattice construction
//! with DPO rewrite rules, Wilson loops for causal invariance analysis,
//! and plaquette action computation.

use catgraph_physics::hypergraph::{
    GaugeGroup, Hypergraph, HypergraphLattice, HypergraphRewriteGroup, RewriteRule,
    plaquette_action, total_action,
};

// ============================================================================
// Gauge Group
// ============================================================================

fn gauge_group() {
    println!("=== Gauge Group ===\n");

    let group = HypergraphRewriteGroup::new(2);
    println!("rules:            {}", group.num_rules());
    println!(
        "Lie algebra dim:  {}",
        HypergraphRewriteGroup::LIE_ALGEBRA_DIM
    );
    println!("abelian:          {}", HypergraphRewriteGroup::IS_ABELIAN);
    println!(
        "spacetime dim:    {}",
        HypergraphRewriteGroup::SPACETIME_DIM
    );
    println!("name:             {}", HypergraphRewriteGroup::name());
    println!("representation:   {}", group.representation_dim());
    println!();
}

// ============================================================================
// Structure Constants
// ============================================================================

fn structure_constants() {
    println!("=== Structure Constants ===\n");

    let group = HypergraphRewriteGroup::new(3);

    // Show antisymmetry: f^{abc} = -f^{bac}
    for (a, b, c) in [(0, 1, 2), (1, 0, 2), (0, 0, 1), (0, 1, 0)] {
        let f = group.structure_constant_for(a, b, c);
        println!("  f^{{{a},{b},{c}}} = {f:+.1}");
    }
    println!("\nantisymmetry check:");
    let f012 = group.structure_constant_for(0, 1, 2);
    let f102 = group.structure_constant_for(1, 0, 2);
    println!("  f^{{0,1,2}} + f^{{1,0,2}} = {}", f012 + f102);
    println!();
}

// ============================================================================
// Lattice Construction
// ============================================================================

fn lattice_construction() {
    println!("=== Lattice Construction ===\n");

    let rules = vec![RewriteRule::wolfram_a_to_bb(), RewriteRule::edge_split()];
    let group = HypergraphRewriteGroup::new(2);
    let lattice: HypergraphLattice<2> = HypergraphLattice::new([3, 3], group, rules);

    println!("dimensions: {:?}", lattice.dimensions());
    println!("rules:      {}", lattice.rules().len());
    println!("sites:      {}", lattice.site_count());
    println!("steps:      {}", lattice.step_count());
    println!();
}

// ============================================================================
// Rewriting on Lattice
// ============================================================================

fn lattice_rewriting() {
    println!("=== Rewriting on Lattice ===\n");

    let rules = vec![RewriteRule::wolfram_a_to_bb(), RewriteRule::edge_split()];
    let group = HypergraphRewriteGroup::new(2);
    let mut lattice: HypergraphLattice<2> = HypergraphLattice::new([3, 3], group, rules);

    // Set ternary edges at two sites
    lattice.set_state(&[0, 0], Hypergraph::from_edges(vec![vec![0, 1, 2]]));
    lattice.set_state(&[1, 1], Hypergraph::from_edges(vec![vec![3, 4, 5]]));

    // Apply A->BB rule (index 0) at [0,0]
    let ok = lattice.apply_rewrite(&[0, 0], 0);
    println!("rewrite at [0,0] with rule 0: {ok}");
    if let Some(state) = lattice.get_state(&[0, 0]) {
        println!("  result: {} edges", state.edge_count());
    }

    // Apply A->BB rule at [1,1]
    let ok2 = lattice.apply_rewrite(&[1, 1], 0);
    println!("rewrite at [1,1] with rule 0: {ok2}");

    // Try edge-split (rule 1) at [0,0] on its binary edges
    let ok3 = lattice.apply_rewrite(&[0, 0], 1);
    println!("rewrite at [0,0] with rule 1: {ok3}");
    if let Some(state) = lattice.get_state(&[0, 0]) {
        println!("  result: {} edges", state.edge_count());
    }

    println!("total steps: {}", lattice.step_count());
    println!();
}

// ============================================================================
// Wilson Loops and Causal Invariance
// ============================================================================

fn wilson_loops() {
    println!("=== Wilson Loops ===\n");

    let rules = vec![RewriteRule::wolfram_a_to_bb()];
    let group = HypergraphRewriteGroup::new(1);
    let mut lattice: HypergraphLattice<1> = HypergraphLattice::new([5], group, rules);

    // Set initial states and evolve
    lattice.set_state(&[1], Hypergraph::from_edges(vec![vec![0, 1, 2]]));
    lattice.set_state(&[2], Hypergraph::from_edges(vec![vec![3, 4, 5]]));
    lattice.apply_rewrite(&[1], 0);
    lattice.apply_rewrite(&[2], 0);

    // Compute Wilson loop around a path
    let path: Vec<&[usize; 1]> = vec![&[1], &[2], &[1]];
    let holonomy = lattice.wilson_loop(&path);
    let invariant = lattice.is_causally_invariant(&path);
    println!("path [1]->[2]->[1]:");
    println!("  holonomy: {holonomy:.4}");
    println!("  causally invariant: {invariant}");
    println!();
}

// ============================================================================
// Plaquette Action
// ============================================================================

fn actions() {
    println!("=== Plaquette Action ===\n");

    // Standalone plaquette action for sample holonomies
    let holonomies = [1.0, 0.8, 0.5, 0.1];
    for h in holonomies {
        let s = plaquette_action(h);
        println!("  holonomy={h:.1}  action={s:.4}");
    }

    // Total action for a set of holonomies
    let mixed = [0.9, 0.7, 1.0];
    let total = total_action(&mixed);
    println!("\ntotal action for {mixed:?}: {total:.4}");

    // Lattice-level plaquette action
    let rules = vec![RewriteRule::wolfram_a_to_bb()];
    let group = HypergraphRewriteGroup::new(1);
    let lattice: HypergraphLattice<1> = HypergraphLattice::new([5], group, rules);

    let path: Vec<&[usize; 1]> = vec![&[0], &[1]];
    let action = lattice.plaquette_action(&path);
    println!("lattice plaquette action (no transitions): {action:.4}");
    println!();
}

fn main() {
    gauge_group();
    structure_constants();
    lattice_construction();
    lattice_rewriting();
    wilson_loops();
    actions();
}
