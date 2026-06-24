//! Integration tests for gauge theory module.
//!
//! Tests structure constants, plaquette/total action functions,
//! and `HypergraphLattice` construction, state management, DPO
//! rewriting with holonomy, Wilson loops, and causal invariance.

#![allow(clippy::float_cmp)]

use catgraph_physics::hypergraph::{
    GaugeGroup, Hypergraph, HypergraphLattice, HypergraphRewriteGroup, RewriteRule,
    plaquette_action, total_action,
};

// ---------------------------------------------------------------------------
// Structure constants
// ---------------------------------------------------------------------------

#[test]
fn structure_constant_antisymmetric() {
    let group = HypergraphRewriteGroup::new(4);

    // f^{abc} = -f^{bac} when c coincides with a or b.
    // The simplified model uses sign(b > a) for c == a and sign(a > b)
    // for c == b, giving antisymmetry in those branches.
    for (a, b, c) in [
        (0, 1, 0),
        (0, 1, 1),
        (1, 2, 1),
        (1, 2, 2),
        (0, 3, 0),
        (0, 3, 3),
    ] {
        let forward = group.structure_constant_for(a, b, c);
        let swapped = group.structure_constant_for(b, a, c);
        assert!(
            (forward + swapped).abs() < 1e-12,
            "f^{{{a},{b},{c}}} = {forward}, f^{{{b},{a},{c}}} = {swapped}; sum should be 0"
        );
    }

    // When all three indices are distinct the simplified model returns 1.0
    // for both orderings (non-antisymmetric -- acknowledged simplification).
    assert_eq!(group.structure_constant_for(0, 1, 2), 1.0);
    assert_eq!(group.structure_constant_for(1, 0, 2), 1.0);
}

#[test]
fn structure_constant_zero_when_equal() {
    let group = HypergraphRewriteGroup::new(4);

    // f^{aac} = 0 for all a, c
    for a in 0..4 {
        for c in 0..4 {
            assert_eq!(
                group.structure_constant_for(a, a, c),
                0.0,
                "f^{{{a},{a},{c}}} should be 0"
            );
        }
    }
}

#[test]
fn structure_constant_out_of_range() {
    let group = HypergraphRewriteGroup::new(3);

    // Any index >= num_rules yields 0
    assert_eq!(group.structure_constant_for(3, 0, 1), 0.0);
    assert_eq!(group.structure_constant_for(0, 3, 1), 0.0);
    assert_eq!(group.structure_constant_for(0, 1, 3), 0.0);
    assert_eq!(group.structure_constant_for(5, 5, 5), 0.0);
}

#[test]
fn trait_constants_correct() {
    assert_eq!(HypergraphRewriteGroup::LIE_ALGEBRA_DIM, 3);
    let is_abelian = HypergraphRewriteGroup::IS_ABELIAN;
    assert!(!is_abelian);
    assert_eq!(HypergraphRewriteGroup::SPACETIME_DIM, 1);
    assert_eq!(HypergraphRewriteGroup::name(), "HypergraphRewrite");
}

// ---------------------------------------------------------------------------
// Plaquette and total action
// ---------------------------------------------------------------------------

#[test]
fn plaquette_action_flat() {
    assert!((plaquette_action(1.0)).abs() < 1e-12);
}

#[test]
fn plaquette_action_curved() {
    let action = plaquette_action(0.5);
    assert!(action > 0.0);
    // -ln(0.5) = ln(2) ≈ 0.6931
    assert!((action - 2.0_f64.ln()).abs() < 1e-12);
}

#[test]
fn plaquette_action_zero_holonomy() {
    assert!(plaquette_action(0.0).is_infinite());
}

#[test]
fn total_action_sums() {
    let expected = 2.0 * plaquette_action(0.5);
    let actual = total_action(&[1.0, 0.5, 0.5]);
    assert!(
        (actual - expected).abs() < 1e-12,
        "total_action([1.0, 0.5, 0.5]) = {actual}, expected {expected}"
    );
}

// ---------------------------------------------------------------------------
// HypergraphLattice construction
// ---------------------------------------------------------------------------

#[test]
fn lattice_1d_construction() {
    let lattice: HypergraphLattice<1> =
        HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![]);

    assert_eq!(lattice.dimensions(), &[5]);
    assert_eq!(lattice.group().num_rules(), 3);
    assert_eq!(lattice.step_count(), 0);
    assert_eq!(lattice.site_count(), 0); // no states populated yet
}

#[test]
fn lattice_2d_construction() {
    let lattice: HypergraphLattice<2> =
        HypergraphLattice::new([4, 4], HypergraphRewriteGroup::new(2), vec![]);

    assert_eq!(lattice.dimensions(), &[4, 4]);
    assert_eq!(lattice.group().num_rules(), 2);
    assert_eq!(lattice.site_count(), 0);
}

// ---------------------------------------------------------------------------
// State management
// ---------------------------------------------------------------------------

#[test]
fn set_and_get_state() {
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([4, 4], HypergraphRewriteGroup::new(3), vec![]);

    let graph = Hypergraph::from_edges(vec![vec![0, 1, 2], vec![2, 3]]);
    lattice.set_state(&[1, 2], graph);

    let retrieved = lattice.get_state(&[1, 2]);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().vertex_count(), 4);

    let retrieved2 = lattice.get_state(&[1, 2]);
    assert_eq!(retrieved2.unwrap().edge_count(), 2);

    // Unoccupied site returns None
    assert!(lattice.get_state(&[0, 0]).is_none());
}

// ---------------------------------------------------------------------------
// apply_rewrite with DPO rewriting
// ---------------------------------------------------------------------------

#[test]
fn apply_rewrite_dpo_splits_ternary_edge() {
    let rule = RewriteRule::wolfram_a_to_bb();
    let mut lattice: HypergraphLattice<1> =
        HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![rule]);

    // Place a ternary edge at site [2]
    let initial = Hypergraph::from_edges(vec![vec![0, 1, 2]]);
    lattice.set_state(&[2], initial);

    assert!(lattice.apply_rewrite(&[2], 0));
    assert_eq!(lattice.step_count(), 1);

    // The ternary edge should have been replaced by two binary edges
    let state = lattice.get_state(&[2]).unwrap();
    assert_eq!(state.edge_count(), 2);
}

// ---------------------------------------------------------------------------
// record_transition and Wilson loop inter-site holonomy
// ---------------------------------------------------------------------------

#[test]
fn record_transition_populates_wilson_loop() {
    // Record inter-site transitions with known holonomies and verify
    // that wilson_loop traverses them correctly.
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([3, 3], HypergraphRewriteGroup::new(3), vec![]);

    lattice.record_transition(&[0, 0], &[1, 0], 2.0);
    lattice.record_transition(&[1, 0], &[1, 1], 0.5);
    lattice.record_transition(&[1, 1], &[0, 1], 2.0);
    lattice.record_transition(&[0, 1], &[0, 0], 0.5);

    // Wilson loop around the plaquette: 2.0 * 0.5 * 2.0 * 0.5 = 1.0
    let holonomy = lattice.wilson_loop(&[&[0, 0], &[1, 0], &[1, 1], &[0, 1]]);
    assert!(
        (holonomy - 1.0).abs() < 1e-10,
        "closed plaquette should have holonomy 1.0, got {holonomy}"
    );
}

#[test]
fn wilson_loop_non_trivial_holonomy() {
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([3, 3], HypergraphRewriteGroup::new(3), vec![]);

    // Non-unit holonomy loop
    lattice.record_transition(&[0, 0], &[1, 0], 2.0);
    lattice.record_transition(&[1, 0], &[0, 0], 3.0);

    let holonomy = lattice.wilson_loop(&[&[0, 0], &[1, 0]]);
    assert!(
        (holonomy - 6.0).abs() < 1e-10,
        "expected 2.0 * 3.0 = 6.0, got {holonomy}"
    );
}

#[test]
fn wilson_loop_partial_links_default_to_unit() {
    // When only some links on a path have recorded transitions,
    // the missing links default to holonomy 1.0.
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([3, 3], HypergraphRewriteGroup::new(3), vec![]);

    lattice.record_transition(&[0, 0], &[1, 0], 3.0);
    // [1,0] → [0,0] has no recorded transition — defaults to 1.0

    let holonomy = lattice.wilson_loop(&[&[0, 0], &[1, 0]]);
    assert!(
        (holonomy - 3.0).abs() < 1e-10,
        "expected 3.0 * 1.0 = 3.0, got {holonomy}"
    );
}

#[test]
fn apply_rewrite_does_not_record_transition() {
    // After the self-loop bug fix, apply_rewrite no longer inserts
    // a (site, site) transition. A single-site Wilson loop should
    // return 1.0 (no transition recorded).
    let rule = RewriteRule::wolfram_a_to_bb();
    let mut lattice: HypergraphLattice<1> =
        HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![rule]);

    let initial = Hypergraph::from_edges(vec![vec![0, 1, 2]]);
    lattice.set_state(&[2], initial);

    assert!(lattice.apply_rewrite(&[2], 0));
    assert_eq!(lattice.step_count(), 1);

    // No self-loop transition recorded — single-site loop is trivial
    let s = [2];
    let holonomy = lattice.wilson_loop(&[&s]);
    assert!(
        (holonomy - 1.0).abs() < 1e-12,
        "apply_rewrite should not record transitions; single-site holonomy should be 1.0, got {holonomy}"
    );
}

#[test]
fn is_causally_invariant_with_flat_plaquette() {
    // A closed plaquette where all link holonomies multiply to 1.0
    // should be detected as causally invariant.
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([3, 3], HypergraphRewriteGroup::new(3), vec![]);

    lattice.record_transition(&[0, 0], &[1, 0], 2.0);
    lattice.record_transition(&[1, 0], &[1, 1], 0.5);
    lattice.record_transition(&[1, 1], &[0, 1], 2.0);
    lattice.record_transition(&[0, 1], &[0, 0], 0.5);

    let path: Vec<&[usize; 2]> = vec![&[0, 0], &[1, 0], &[1, 1], &[0, 1]];
    assert!(
        lattice.is_causally_invariant(&path),
        "plaquette with holonomy product 1.0 should be causally invariant"
    );
}

#[test]
fn is_not_causally_invariant_with_curved_plaquette() {
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([3, 3], HypergraphRewriteGroup::new(3), vec![]);

    lattice.record_transition(&[0, 0], &[1, 0], 2.0);
    lattice.record_transition(&[1, 0], &[1, 1], 2.0);
    lattice.record_transition(&[1, 1], &[0, 1], 2.0);
    lattice.record_transition(&[0, 1], &[0, 0], 2.0);

    let path: Vec<&[usize; 2]> = vec![&[0, 0], &[1, 0], &[1, 1], &[0, 1]];
    assert!(
        !lattice.is_causally_invariant(&path),
        "plaquette with holonomy product 16.0 should NOT be causally invariant"
    );
}

#[test]
fn apply_rewrite_no_match_returns_false() {
    let rule = RewriteRule::wolfram_a_to_bb(); // expects ternary edge
    let mut lattice: HypergraphLattice<1> =
        HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![rule]);

    // Place a binary edge -- won't match the A→BB rule
    let initial = Hypergraph::from_edges(vec![vec![0, 1]]);
    lattice.set_state(&[2], initial);

    assert!(!lattice.apply_rewrite(&[2], 0));
    assert_eq!(lattice.step_count(), 0);
}

#[test]
fn apply_rewrite_invalid_site() {
    let mut lattice: HypergraphLattice<1> =
        HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![]);

    // Site [5] is out of bounds for dimension size 5 (valid: 0..4)
    assert!(!lattice.apply_rewrite(&[5], 0));
    assert_eq!(lattice.step_count(), 0);
}

#[test]
fn apply_rewrite_invalid_rule() {
    let mut lattice: HypergraphLattice<1> =
        HypergraphLattice::new([5], HypergraphRewriteGroup::new(2), vec![]);

    // No rules at all -- any rule index should fail
    assert!(!lattice.apply_rewrite(&[1], 0));
    assert_eq!(lattice.step_count(), 0);
}

// ---------------------------------------------------------------------------
// Wilson loops and causal invariance
// ---------------------------------------------------------------------------

#[test]
fn wilson_loop_empty_path() {
    let lattice: HypergraphLattice<1> =
        HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![]);

    let path: Vec<&[usize; 1]> = vec![];
    assert_eq!(lattice.wilson_loop(&path), 1.0);
}

#[test]
fn wilson_loop_no_transitions() {
    let lattice: HypergraphLattice<2> =
        HypergraphLattice::new([4, 4], HypergraphRewriteGroup::new(3), vec![]);

    // Path over sites with no recorded transitions -> holonomy = 1.0
    let s0 = [0, 0];
    let s1 = [1, 0];
    let s2 = [1, 1];
    let s3 = [0, 1];
    let path: Vec<&[usize; 2]> = vec![&s0, &s1, &s2, &s3];

    assert_eq!(lattice.wilson_loop(&path), 1.0);
}

#[test]
fn is_causally_invariant_trivial() {
    let lattice: HypergraphLattice<1> =
        HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![]);

    // Trivial path (single site loop) with no transitions -> invariant
    let s0 = [2];
    let path: Vec<&[usize; 1]> = vec![&s0];
    assert!(lattice.is_causally_invariant(&path));
}

#[test]
fn find_wilson_loops_2d() {
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([3, 3], HypergraphRewriteGroup::new(2), vec![]);

    lattice.find_wilson_loops(4);

    // A 3x3 grid has (3-1)*(3-1) = 4 elementary plaquettes
    let loops = lattice.recorded_loops();
    assert_eq!(loops.len(), 4, "3x3 lattice should have 4 plaquettes");

    // All holonomies should be 1.0 (no transitions recorded)
    for (sites, holonomy) in loops {
        assert_eq!(*holonomy, 1.0);
        assert_eq!(sites.len(), 4, "each plaquette has 4 corners");
    }
}
