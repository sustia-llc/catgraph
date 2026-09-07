//! Lattice gauge theory for hypergraph rewriting.
//!
//! Demonstrates gauge groups, structure constants, lattice construction
//! with DPO rewrite rules, Wilson loops for causal invariance analysis,
//! plaquette action computation, and a GL(2) link field under a gauge
//! transformation.

use std::collections::HashMap;

use catgraph_physics::hypergraph::{
    GaugeGroup, Hypergraph, HypergraphLattice, HypergraphRewriteGroup, RewriteRule,
    plaquette_action, total_action,
};
use nalgebra::DMatrix;

/// The 1 × 1 link variable carrying `value`.
fn link1(value: f64) -> DMatrix<f64> {
    DMatrix::from_element(1, 1, value)
}

/// The 2 × 2 matrix with rows `[a, b]` and `[c, d]`.
fn m2(a: f64, b: f64, c: f64, d: f64) -> DMatrix<f64> {
    DMatrix::from_row_slice(2, 2, &[a, b, c, d])
}

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
    let lattice: HypergraphLattice<2> = HypergraphLattice::new([3, 3], group, rules, 1);

    println!("dimensions: {:?}", lattice.dimensions());
    println!("link dim:   {}", lattice.link_dim());
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
    let mut lattice: HypergraphLattice<2> = HypergraphLattice::new([3, 3], group, rules, 1);

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
    let mut lattice: HypergraphLattice<1> = HypergraphLattice::new([5], group, rules, 1);

    // Set initial states and evolve
    lattice.set_state(&[1], Hypergraph::from_edges(vec![vec![0, 1, 2]]));
    lattice.set_state(&[2], Hypergraph::from_edges(vec![vec![3, 4, 5]]));
    lattice.apply_rewrite(&[1], 0);
    lattice.apply_rewrite(&[2], 0);

    // Wilson loop over a path whose links have no recorded transition
    let path: Vec<&[usize; 1]> = vec![&[1], &[2]];
    println!("path [1]->[2]->[1] with no links recorded:");
    println!("  holonomy: {:?}", lattice.wilson_loop(&path));
    println!(
        "  causally invariant: {:?}",
        lattice.is_causally_invariant(&path)
    );

    // Record both links of the loop, then measure it
    lattice.record_transition(&[1], &[2], link1(2.0));
    lattice.record_transition(&[2], &[1], link1(0.5));
    println!("path [1]->[2]->[1] with both links recorded:");
    match lattice.wilson_loop(&path) {
        Some(h) => println!("  holonomy: {h:.4}"),
        None => println!("  holonomy: none"),
    }
    println!(
        "  causally invariant: {:?}",
        lattice.is_causally_invariant(&path)
    );
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
    let mut lattice: HypergraphLattice<1> = HypergraphLattice::new([5], group, rules, 1);

    let path: Vec<&[usize; 1]> = vec![&[0], &[1]];
    println!(
        "lattice plaquette action (no transitions): {:?}",
        lattice.plaquette_action(&path)
    );

    lattice.record_transition(&[0], &[1], link1(0.5));
    lattice.record_transition(&[1], &[0], link1(1.0));
    match lattice.plaquette_action(&path) {
        Some(a) => println!("lattice plaquette action (holonomy 0.5): {a:.4}"),
        None => println!("lattice plaquette action (holonomy 0.5): none"),
    }
    println!();
}

// ============================================================================
// GL(2) Link Variables
// ============================================================================

fn gl2_holonomy() {
    println!("=== GL(2) Link Variables ===\n");

    let group = HypergraphRewriteGroup::new(1);
    let mut lattice: HypergraphLattice<1> = HypergraphLattice::new([3], group, vec![], 2);

    let a = m2(1.0, 1.0, 0.0, 1.0);
    let b = m2(1.0, 0.0, 1.0, 1.0);
    let c = m2(2.0, 0.0, 0.0, 1.0);
    lattice.record_transition(&[0], &[1], a);
    lattice.record_transition(&[1], &[2], b);
    lattice.record_transition(&[2], &[0], c);

    let path: Vec<&[usize; 1]> = vec![&[0], &[1], &[2]];
    match lattice.loop_holonomy(&path) {
        Some(h) => println!("holonomy C*B*A over [0]->[1]->[2]->[0]:\n{h}"),
        None => println!("holonomy over [0]->[1]->[2]->[0]: none"),
    }
    println!("wilson value:  {:?}", lattice.wilson_loop(&path));
    println!("flat at 1e-6:  {:?}", lattice.is_flat(&path, 1e-6));

    // Rotating the base point conjugates the holonomy and keeps the trace.
    let rotated: Vec<&[usize; 1]> = vec![&[1], &[2], &[0]];
    match lattice.loop_holonomy(&rotated) {
        Some(h) => println!("holonomy A*C*B over [1]->[2]->[0]->[1]:\n{h}"),
        None => println!("holonomy over [1]->[2]->[0]->[1]: none"),
    }
    println!("wilson value:  {:?}", lattice.wilson_loop(&rotated));

    // A gauge transformation moves the links and fixes the Wilson value.
    let mut g: HashMap<Vec<usize>, DMatrix<f64>> = HashMap::new();
    g.insert(vec![0], m2(1.0, 2.0, 0.0, 1.0));
    g.insert(vec![1], m2(3.0, 0.0, 0.0, 1.0));
    println!("gauge_transform: {}", lattice.gauge_transform(&g));
    match lattice.loop_holonomy(&path) {
        Some(h) => println!("holonomy after conjugation by g_0:\n{h}"),
        None => println!("holonomy after conjugation by g_0: none"),
    }
    println!("wilson value:  {:?}", lattice.wilson_loop(&path));
    println!();
}

fn main() {
    gauge_group();
    structure_constants();
    lattice_construction();
    lattice_rewriting();
    wilson_loops();
    actions();
    gl2_holonomy();
}
