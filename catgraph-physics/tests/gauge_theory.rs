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
    assert!(lattice.set_state(&[1, 2], graph));

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
    assert!(lattice.set_state(&[2], initial));

    assert!(lattice.apply_rewrite(&[2], 0));
    assert_eq!(lattice.step_count(), 1);

    // The ternary edge should have been replaced by two binary edges
    let state = lattice.get_state(&[2]).unwrap();
    assert_eq!(state.edge_count(), 2);
}

// ---------------------------------------------------------------------------
// set_state / record_transition validation
// ---------------------------------------------------------------------------

#[test]
fn set_state_rejects_out_of_bounds_site() {
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([3, 4], HypergraphRewriteGroup::new(1), vec![]);

    let graph = Hypergraph::from_edges(vec![vec![0, 1]]);

    // Last in-bounds site on each axis is accepted.
    assert!(
        lattice.set_state(&[2, 3], graph.clone()),
        "set_state(&[2, 3]) on a [3, 4] lattice: got false, expected true"
    );

    // Out of bounds on either axis is rejected, and nothing is inserted.
    for site in [[3, 0], [0, 4], [3, 4], [7, 9]] {
        assert!(
            !lattice.set_state(&site, graph.clone()),
            "set_state(&{site:?}) on a [3, 4] lattice: got true, expected false"
        );
        assert!(
            lattice.get_state(&site).is_none(),
            "set_state(&{site:?}) inserted a state; expected none"
        );
    }

    assert_eq!(
        lattice.site_count(),
        1,
        "site_count after 1 accepted and 4 rejected set_state calls: expected 1"
    );
}

#[test]
fn record_transition_rejects_bad_sites_and_holonomies() {
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([2, 2], HypergraphRewriteGroup::new(1), vec![]);

    assert!(
        lattice.record_transition(&[0, 0], &[1, 0], 2.0),
        "record_transition with in-bounds sites and holonomy 2.0: got false, expected true"
    );

    // Out-of-bounds source, target, or both. Both directions of each pair are
    // rejected, so the two-site loop over them has a holonomy only if a
    // rejected call inserted a link.
    for (from, to) in [([2, 0], [0, 0]), ([0, 0], [0, 2]), ([9, 9], [9, 9])] {
        assert!(
            !lattice.record_transition(&from, &to, 2.0),
            "record_transition({from:?} -> {to:?}) on a [2, 2] lattice: got true, expected false"
        );
        assert!(
            !lattice.record_transition(&to, &from, 2.0),
            "record_transition({to:?} -> {from:?}) on a [2, 2] lattice: got true, expected false"
        );
        assert_eq!(
            lattice.wilson_loop(&[&from, &to]),
            None,
            "loop {from:?} -> {to:?} over rejected links: expected None"
        );
    }

    // Non-finite or non-positive holonomies on in-bounds sites. The reverse
    // link is recorded first, so the two-site loop closes if a rejected call
    // inserted the forward link.
    assert!(
        lattice.record_transition(&[1, 1], &[0, 1], 1.0),
        "record_transition([1, 1] -> [0, 1]) with holonomy 1.0: got false, expected true"
    );
    for h in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -1.5] {
        assert!(
            !lattice.record_transition(&[0, 1], &[1, 1], h),
            "record_transition with holonomy {h}: got true, expected false"
        );
        assert_eq!(
            lattice.wilson_loop(&[&[0, 1], &[1, 1]]),
            None,
            "loop [0,1] -> [1,1] after a rejected holonomy {h}: expected None"
        );
    }

    assert!(
        lattice.record_transition(&[1, 0], &[0, 0], 0.5),
        "record_transition with holonomy 0.5: got false, expected true"
    );
    assert_eq!(
        lattice.wilson_loop(&[&[0, 0], &[1, 0]]),
        Some(1.0),
        "loop [0,0] -> [1,0] over 2.0 and 0.5: expected Some(1.0)"
    );
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
    let holonomy = lattice
        .wilson_loop(&[&[0, 0], &[1, 0], &[1, 1], &[0, 1]])
        .expect("invariant: all four links of the plaquette were just recorded");
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

    let holonomy = lattice
        .wilson_loop(&[&[0, 0], &[1, 0]])
        .expect("invariant: both links of the two-site loop were just recorded");
    assert!(
        (holonomy - 6.0).abs() < 1e-10,
        "expected 2.0 * 3.0 = 6.0, got {holonomy}"
    );
}

#[test]
fn wilson_loop_missing_link_has_no_holonomy() {
    // A loop with an unrecorded link has no holonomy at all: the missing
    // link is not read as 1.0.
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([3, 3], HypergraphRewriteGroup::new(3), vec![]);

    lattice.record_transition(&[0, 0], &[1, 0], 3.0);
    // [1,0] -> [0,0] has no recorded transition

    assert_eq!(
        lattice.wilson_loop(&[&[0, 0], &[1, 0]]),
        None,
        "loop with one unrecorded link: expected None, the h=1.0 reading would give Some(3.0)"
    );
    assert_eq!(
        lattice.is_causally_invariant(&[&[0, 0], &[1, 0]]),
        None,
        "loop with one unrecorded link: expected None"
    );
    assert_eq!(
        lattice.plaquette_action(&[&[0, 0], &[1, 0]]),
        None,
        "loop with one unrecorded link: expected None"
    );

    // Closing the loop with a link that makes the product 1.0 turns the
    // verdict from "unknown" into "invariant".
    lattice.record_transition(&[1, 0], &[0, 0], 1.0 / 3.0);
    assert_eq!(
        lattice.is_causally_invariant(&[&[0, 0], &[1, 0]]),
        Some(true),
        "loop over 3.0 and 1/3: expected Some(true)"
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
    assert!(lattice.set_state(&[2], initial));

    assert!(lattice.apply_rewrite(&[2], 0));
    assert_eq!(lattice.step_count(), 1);

    // No self-loop transition recorded — the single-site loop has no holonomy
    let s = [2];
    assert_eq!(
        lattice.wilson_loop(&[&s]),
        None,
        "apply_rewrite should record no transition; expected None for the single-site loop"
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
    assert_eq!(
        lattice.is_causally_invariant(&path),
        Some(true),
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
    assert_eq!(
        lattice.is_causally_invariant(&path),
        Some(false),
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
    assert!(lattice.set_state(&[2], initial));

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
    assert_eq!(lattice.wilson_loop(&path), Some(1.0));
}

#[test]
fn wilson_loop_no_transitions() {
    let lattice: HypergraphLattice<2> =
        HypergraphLattice::new([4, 4], HypergraphRewriteGroup::new(3), vec![]);

    // Path over sites with no recorded transitions -> no holonomy
    let s0 = [0, 0];
    let s1 = [1, 0];
    let s2 = [1, 1];
    let s3 = [0, 1];
    let path: Vec<&[usize; 2]> = vec![&s0, &s1, &s2, &s3];

    assert_eq!(lattice.wilson_loop(&path), None);
}

#[test]
fn is_causally_invariant_trivial() {
    let lattice: HypergraphLattice<1> =
        HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![]);

    // Single-site loop with no recorded self-link -> no verdict
    let s0 = [2];
    let path: Vec<&[usize; 1]> = vec![&s0];
    assert_eq!(lattice.is_causally_invariant(&path), None);
}

// ---------------------------------------------------------------------------
// find_wilson_loops: the max_length bound, the D range, the link requirement
// ---------------------------------------------------------------------------

/// Records every axis-aligned nearest-neighbour link of a `dims` lattice with
/// holonomy `h` forward and `1.0 / h` back, so every elementary plaquette is
/// closed.
fn record_all_links<const D: usize>(lattice: &mut HypergraphLattice<D>, dims: [usize; D], h: f64) {
    let total: usize = dims.iter().product();
    for flat in 0..total {
        let mut site = [0usize; D];
        let mut rest = flat;
        for axis in (0..D).rev() {
            site[axis] = rest % dims[axis];
            rest /= dims[axis];
        }
        for axis in 0..D {
            if site[axis] + 1 < dims[axis] {
                let mut next = site;
                next[axis] += 1;
                assert!(lattice.record_transition(&site, &next, h));
                assert!(lattice.record_transition(&next, &site, 1.0 / h));
            }
        }
    }
}

#[test]
fn find_wilson_loops_2d() {
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([3, 3], HypergraphRewriteGroup::new(2), vec![]);
    record_all_links(&mut lattice, [3, 3], 1.0);

    lattice.find_wilson_loops(4);

    // A 3x3 grid has (3-1)*(3-1) = 4 elementary plaquettes
    let loops = lattice.recorded_loops();
    assert_eq!(loops.len(), 4, "3x3 lattice should have 4 plaquettes");

    for (sites, holonomy) in loops {
        assert_eq!(*holonomy, 1.0);
        assert_eq!(sites.len(), 4, "each plaquette has 4 corners");
    }
}

#[test]
fn find_wilson_loops_honors_max_length() {
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([3, 3], HypergraphRewriteGroup::new(2), vec![]);
    record_all_links(&mut lattice, [3, 3], 1.0);

    // A bound below the elementary plaquette length admits nothing.
    for bound in [0usize, 1, 2, 3] {
        lattice.find_wilson_loops(bound);
        assert_eq!(
            lattice.recorded_loops().len(),
            0,
            "find_wilson_loops({bound}) on a fully linked 3x3 lattice: expected 0 loops, \
             the unbounded reading records 4"
        );
    }

    // At and above the plaquette length, all four plaquettes are admitted.
    for bound in [4usize, 5, 12] {
        lattice.find_wilson_loops(bound);
        assert_eq!(
            lattice.recorded_loops().len(),
            4,
            "find_wilson_loops({bound}) on a fully linked 3x3 lattice: expected 4 loops"
        );
    }
}

#[test]
fn find_wilson_loops_1d_records_nothing() {
    let mut lattice: HypergraphLattice<1> =
        HypergraphLattice::new([4], HypergraphRewriteGroup::new(2), vec![]);
    record_all_links(&mut lattice, [4], 1.0);

    lattice.find_wilson_loops(4);
    assert_eq!(
        lattice.recorded_loops().len(),
        0,
        "1D lattice has no coordinate plane: expected 0 loops"
    );
    assert_eq!(
        lattice.is_globally_causally_invariant(),
        None,
        "1D lattice records no loops: expected None, the vacuous reading gives Some(true)"
    );
}

#[test]
fn find_wilson_loops_3d_covers_every_coordinate_plane() {
    // Elementary-plaquette count of a lattice [n_0..n_{D-1}]:
    //   sum over axis pairs i<j of (n_i - 1)(n_j - 1) * product of n_k, k != i,j.
    // 2x2x2: three planes, each (2-1)(2-1)*2 = 2, so 6.
    let mut lattice: HypergraphLattice<3> =
        HypergraphLattice::new([2, 2, 2], HypergraphRewriteGroup::new(2), vec![]);
    record_all_links(&mut lattice, [2, 2, 2], 1.0);

    lattice.find_wilson_loops(4);
    assert_eq!(
        lattice.recorded_loops().len(),
        6,
        "2x2x2 lattice: expected 6 plaquettes (2 per coordinate plane), \
         the D==2-only reading records 0"
    );

    // 3x3x3: three planes, each (3-1)(3-1)*3 = 12, so 36.
    let mut big: HypergraphLattice<3> =
        HypergraphLattice::new([3, 3, 3], HypergraphRewriteGroup::new(2), vec![]);
    record_all_links(&mut big, [3, 3, 3], 1.0);
    big.find_wilson_loops(4);
    assert_eq!(
        big.recorded_loops().len(),
        36,
        "3x3x3 lattice: expected 36 plaquettes, the D==2-only reading records 0"
    );

    // 2x2x2x2: six axis pairs, each (2-1)(2-1)*2*2 = 4, so 24.
    let mut four: HypergraphLattice<4> =
        HypergraphLattice::new([2, 2, 2, 2], HypergraphRewriteGroup::new(2), vec![]);
    record_all_links(&mut four, [2, 2, 2, 2], 1.0);
    four.find_wilson_loops(4);
    assert_eq!(
        four.recorded_loops().len(),
        24,
        "2x2x2x2 lattice: expected 24 plaquettes (4 per axis pair, 6 pairs), \
         the D==2-only reading records 0"
    );

    // Every recorded plaquette differs from its base site in exactly two axes,
    // and the axis pairs so moved are the three coordinate planes.
    let mut planes: Vec<(usize, usize)> = Vec::new();
    for (sites, _) in big.recorded_loops() {
        assert_eq!(sites.len(), 4, "each plaquette has 4 corners");
        let moved: Vec<usize> = (0..3)
            .filter(|&axis| sites.iter().any(|s| s[axis] != sites[0][axis]))
            .collect();
        assert_eq!(
            moved.len(),
            2,
            "plaquette {sites:?} moves in {} axes, expected 2",
            moved.len()
        );
        planes.push((moved[0], moved[1]));
    }
    planes.sort_unstable();
    planes.dedup();
    assert_eq!(
        planes,
        vec![(0, 1), (0, 2), (1, 2)],
        "3x3x3 plaquette axis pairs: expected the three coordinate planes, \
         a fixed-pair enumeration records only [(0, 1)]"
    );
}

#[test]
fn find_wilson_loops_indexes_each_axis_by_its_own_dimension() {
    // Elementary-plaquette count of [2, 3, 4]:
    //   (2-1)(3-1)*4 + (2-1)(4-1)*3 + (3-1)(4-1)*2 = 8 + 9 + 12 = 29.
    let mut lattice: HypergraphLattice<3> =
        HypergraphLattice::new([2, 3, 4], HypergraphRewriteGroup::new(2), vec![]);
    record_all_links(&mut lattice, [2, 3, 4], 1.0);

    lattice.find_wilson_loops(4);
    assert_eq!(
        lattice.recorded_loops().len(),
        29,
        "[2, 3, 4] lattice: expected 29 plaquettes; a dimensions[0] odometer bound \
         records 16, a plane test reading dimensions[i] for both corners records 15"
    );

    // Elementary-plaquette count of [4, 3, 2]:
    //   (4-1)(3-1)*2 + (4-1)(2-1)*3 + (3-1)(2-1)*4 = 12 + 9 + 8 = 29.
    let mut reversed: HypergraphLattice<3> =
        HypergraphLattice::new([4, 3, 2], HypergraphRewriteGroup::new(2), vec![]);
    record_all_links(&mut reversed, [4, 3, 2], 1.0);

    reversed.find_wilson_loops(4);
    assert_eq!(
        reversed.recorded_loops().len(),
        29,
        "[4, 3, 2] lattice: expected 29 plaquettes; a plane test reading \
         dimensions[j] for both corners records 15"
    );
}

#[test]
fn find_wilson_loops_3d_is_not_vacuously_invariant() {
    // A 3D lattice of holonomy-1.0 links, with the four forward links of the
    // xy plaquette at [0, 0, 0] overwritten with holonomy 2.0, so that
    // plaquette has holonomy 16.0.
    let mut lattice: HypergraphLattice<3> =
        HypergraphLattice::new([2, 2, 2], HypergraphRewriteGroup::new(2), vec![]);
    record_all_links(&mut lattice, [2, 2, 2], 1.0);
    for (from, to) in [
        ([0usize, 0, 0], [1usize, 0, 0]),
        ([1, 0, 0], [1, 1, 0]),
        ([1, 1, 0], [0, 1, 0]),
        ([0, 1, 0], [0, 0, 0]),
    ] {
        assert!(lattice.record_transition(&from, &to, 2.0));
    }

    lattice.find_wilson_loops(4);
    assert_eq!(
        lattice.is_globally_causally_invariant(),
        Some(false),
        "2x2x2 lattice with a curved xy plaquette (holonomy 16.0): expected Some(false); \
         a D==2-only enumeration records no loops here"
    );
}

#[test]
fn average_holonomy_divides_the_sum_by_the_loop_count() {
    // A 2x3 lattice of holonomy-1.0 links carries two plaquettes; the four
    // links of the plaquette at [0, 0] are overwritten with holonomy 2.0, so
    // the two holonomies are 16.0 and 1.0.
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([2, 3], HypergraphRewriteGroup::new(2), vec![]);
    record_all_links(&mut lattice, [2, 3], 1.0);
    for (from, to) in [
        ([0usize, 0], [1usize, 0]),
        ([1, 0], [1, 1]),
        ([1, 1], [0, 1]),
        ([0, 1], [0, 0]),
    ] {
        assert!(lattice.record_transition(&from, &to, 2.0));
    }

    lattice.find_wilson_loops(4);
    assert_eq!(
        lattice.recorded_loops().len(),
        2,
        "2x3 lattice: expected 2 plaquettes"
    );
    assert_eq!(
        lattice.average_holonomy(),
        Some(8.5),
        "holonomies 16.0 and 1.0: expected Some(8.5); an undivided sum reads \
         Some(17.0), an unconditional empty reading reads None"
    );
}

#[test]
fn find_wilson_loops_skips_plaquettes_with_unrecorded_links() {
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([3, 3], HypergraphRewriteGroup::new(2), vec![]);

    // No links at all: nothing to record.
    lattice.find_wilson_loops(4);
    assert_eq!(
        lattice.recorded_loops().len(),
        0,
        "linkless 3x3 lattice: expected 0 plaquettes, the h=1.0 reading records 4"
    );
    assert_eq!(
        lattice.is_globally_causally_invariant(),
        None,
        "linkless 3x3 lattice: expected None"
    );

    // Close exactly one plaquette, at base site [0, 0].
    for (from, to) in [
        ([0usize, 0], [1usize, 0]),
        ([1, 0], [1, 1]),
        ([1, 1], [0, 1]),
        ([0, 1], [0, 0]),
    ] {
        assert!(lattice.record_transition(&from, &to, 1.0));
    }

    lattice.find_wilson_loops(4);
    let loops = lattice.recorded_loops();
    assert_eq!(
        loops.len(),
        1,
        "3x3 lattice with one closed plaquette: expected 1 plaquette, \
         the h=1.0 reading records 4"
    );
    assert_eq!(
        loops[0].0,
        vec![vec![0, 0], vec![1, 0], vec![1, 1], vec![0, 1]],
        "the recorded plaquette should be the closed one"
    );
    assert_eq!(
        lattice.is_globally_causally_invariant(),
        Some(true),
        "one closed plaquette of holonomy 1.0: expected Some(true)"
    );
}
