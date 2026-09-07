//! Integration tests for the gauge theory module.
//!
//! Tests structure constants, plaquette/total action functions, and
//! `HypergraphLattice` construction, state management, DPO rewriting, matrix
//! link variables, path-ordered loop holonomies, Wilson loops, flatness, and
//! gauge transformation by vertex conjugation.

#![cfg(feature = "gauge")]
#![allow(clippy::float_cmp)]

use std::collections::HashMap;

use catgraph_physics::hypergraph::{
    GaugeGroup, Hypergraph, HypergraphLattice, HypergraphRewriteGroup, RewriteRule,
    plaquette_action, total_action,
};
use catgraph_testutil::Lcg;
use nalgebra::DMatrix;

// ---------------------------------------------------------------------------
// Link-variable helpers
// ---------------------------------------------------------------------------

/// The 1 × 1 link variable carrying `value`.
fn link1(value: f64) -> DMatrix<f64> {
    DMatrix::from_element(1, 1, value)
}

/// The 2 × 2 matrix with rows `[a, b]` and `[c, d]`.
fn m2(a: f64, b: f64, c: f64, d: f64) -> DMatrix<f64> {
    DMatrix::from_row_slice(2, 2, &[a, b, c, d])
}

/// The matrix as a row-major nested list.
fn rows(matrix: &DMatrix<f64>) -> Vec<Vec<f64>> {
    matrix
        .row_iter()
        .map(|row| row.iter().copied().collect())
        .collect()
}

/// Largest absolute entrywise difference between two matrices of one shape.
fn max_diff(left: &DMatrix<f64>, right: &DMatrix<f64>) -> f64 {
    (left - right)
        .iter()
        .fold(0.0_f64, |acc, d| acc.max(d.abs()))
}

/// A seeded `dim` × `dim` matrix with entries in `-2.0..2.0`, redrawn until
/// `try_inverse` is `Some`.
fn seeded_invertible(rng: &mut Lcg, dim: usize) -> DMatrix<f64> {
    for _ in 0..64 {
        let entries: Vec<f64> = (0..dim * dim)
            .map(|_| 4.0_f64.mul_add(rng.next_f64(), -2.0))
            .collect();
        let candidate = DMatrix::from_row_slice(dim, dim, &entries);
        if candidate.clone().try_inverse().is_some() {
            return candidate;
        }
    }
    panic!("no invertible draw at dim {dim} in 64 attempts");
}

/// The `[usize; D]` path over a recorded loop's site cycle.
fn corners_2d(sites: &[Vec<usize>]) -> Vec<[usize; 2]> {
    sites.iter().map(|s| [s[0], s[1]]).collect()
}

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
        HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![], 1);

    assert_eq!(lattice.dimensions(), &[5]);
    assert_eq!(lattice.group().num_rules(), 3);
    assert_eq!(lattice.step_count(), 0);
    assert_eq!(lattice.site_count(), 0); // no states populated yet
    assert_eq!(lattice.link_dim(), 1);
}

#[test]
fn lattice_2d_construction() {
    let lattice: HypergraphLattice<2> =
        HypergraphLattice::new([4, 4], HypergraphRewriteGroup::new(2), vec![], 3);

    assert_eq!(lattice.dimensions(), &[4, 4]);
    assert_eq!(lattice.group().num_rules(), 2);
    assert_eq!(lattice.site_count(), 0);
    assert_eq!(
        lattice.link_dim(),
        3,
        "link_dim() = {}, expected the 3 passed to new",
        lattice.link_dim()
    );
}

/// `new` rejects `link_dim` 0 by panicking.
#[test]
#[should_panic(expected = "link_dim must be positive")]
fn lattice_new_rejects_link_dim_zero() {
    let _: HypergraphLattice<1> =
        HypergraphLattice::new([2], HypergraphRewriteGroup::new(1), vec![], 0);
}

// ---------------------------------------------------------------------------
// State management
// ---------------------------------------------------------------------------

#[test]
fn set_and_get_state() {
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([4, 4], HypergraphRewriteGroup::new(3), vec![], 1);

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
        HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![rule], 1);

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
        HypergraphLattice::new([3, 4], HypergraphRewriteGroup::new(1), vec![], 1);

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
fn record_transition_rejects_out_of_bounds_sites() {
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([2, 2], HypergraphRewriteGroup::new(1), vec![], 1);

    assert!(
        lattice.record_transition(&[0, 0], &[1, 0], link1(2.0)),
        "record_transition with in-bounds sites and link 2.0: got false, expected true"
    );

    // Out-of-bounds source, target, or both. Both directions of each pair are
    // rejected, so the two-site loop over them has a holonomy only if a
    // rejected call inserted a link.
    for (from, to) in [([2, 0], [0, 0]), ([0, 0], [0, 2]), ([9, 9], [9, 9])] {
        assert!(
            !lattice.record_transition(&from, &to, link1(2.0)),
            "record_transition({from:?} -> {to:?}) on a [2, 2] lattice: got true, expected false"
        );
        assert!(
            !lattice.record_transition(&to, &from, link1(2.0)),
            "record_transition({to:?} -> {from:?}) on a [2, 2] lattice: got true, expected false"
        );
        assert_eq!(
            lattice.wilson_loop(&[&from, &to]),
            None,
            "loop {from:?} -> {to:?} over rejected links: expected None"
        );
    }

    assert!(
        lattice.record_transition(&[1, 0], &[0, 0], link1(0.5)),
        "record_transition with link 0.5: got false, expected true"
    );
    assert_eq!(
        lattice.wilson_loop(&[&[0, 0], &[1, 0]]),
        Some(1.0),
        "loop [0,0] -> [1,0] over 2.0 and 0.5: expected Some(1.0)"
    );
}

/// Pin (e): the admissibility contract on a link and on a gauge field.
#[test]
fn record_transition_rejects_inadmissible_link_matrices() {
    let mut lattice: HypergraphLattice<1> =
        HypergraphLattice::new([2], HypergraphRewriteGroup::new(1), vec![], 2);

    let wrong_shape = DMatrix::from_element(1, 1, 1.0);
    assert!(
        !lattice.record_transition(&[0], &[1], wrong_shape),
        "record_transition of a 1x1 link on a link_dim 2 lattice: got true, expected false"
    );
    assert!(
        lattice.link(&[0], &[1]).is_none(),
        "link(0, 1) after a rejected 1x1 link: got Some, expected None"
    );

    let not_finite = m2(1.0, f64::NAN, 0.0, 1.0);
    assert!(
        !lattice.record_transition(&[0], &[1], not_finite),
        "record_transition of [[1, NAN], [0, 1]]: got true, expected false"
    );

    let singular = m2(1.0, 2.0, 2.0, 4.0);
    assert!(
        !lattice.record_transition(&[0], &[1], singular),
        "record_transition of the singular [[1, 2], [2, 4]] (determinant 0): \
         got true, expected false"
    );
    assert!(
        lattice.link(&[0], &[1]).is_none(),
        "link(0, 1) after three rejected links: got Some, expected None"
    );

    let admissible = m2(1.0, 2.0, 3.0, 4.0);
    assert!(
        lattice.record_transition(&[0], &[1], admissible.clone()),
        "record_transition of [[1, 2], [3, 4]] (determinant -2): got false, expected true"
    );
    assert_eq!(
        lattice.link(&[0], &[1]),
        Some(&admissible),
        "link(0, 1) after the accepted link: expected [[1, 2], [3, 4]]"
    );

    // A gauge field with a singular value is rejected whole, and no link moves.
    let mut singular_field: HashMap<Vec<usize>, DMatrix<f64>> = HashMap::new();
    singular_field.insert(vec![0], m2(1.0, 2.0, 2.0, 4.0));
    assert!(
        !lattice.gauge_transform(&singular_field),
        "gauge_transform with the singular g_0 = [[1, 2], [2, 4]]: got true, expected false"
    );
    assert_eq!(
        lattice.link(&[0], &[1]),
        Some(&admissible),
        "link(0, 1) after a rejected gauge_transform: expected the unchanged [[1, 2], [3, 4]]"
    );

    // A gauge field of the wrong shape is rejected the same way.
    let mut wrong_shape_field: HashMap<Vec<usize>, DMatrix<f64>> = HashMap::new();
    wrong_shape_field.insert(vec![1], DMatrix::from_element(1, 1, 2.0));
    assert!(
        !lattice.gauge_transform(&wrong_shape_field),
        "gauge_transform with a 1x1 g_1 on a link_dim 2 lattice: got true, expected false"
    );
    assert_eq!(
        lattice.link(&[0], &[1]),
        Some(&admissible),
        "link(0, 1) after a rejected gauge_transform: expected the unchanged [[1, 2], [3, 4]]"
    );
}

// ---------------------------------------------------------------------------
// Pins (a), (b), (c), (f), (g) — link_dim 1 scalars, ordering, conjugation
// ---------------------------------------------------------------------------

/// Pin (a): at `link_dim` 1 the Wilson value is the scalar link product.
#[test]
fn wilson_loop_at_link_dim_one_is_the_scalar_product() {
    let mut lattice: HypergraphLattice<1> =
        HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![], 1);

    assert!(lattice.record_transition(&[0], &[1], link1(2.0)));
    assert!(lattice.record_transition(&[1], &[0], link1(0.5)));

    let flat = lattice.wilson_loop(&[&[0], &[1]]);
    assert_eq!(
        flat,
        Some(1.0),
        "links 2.0 and 0.5 at link_dim 1: wilson_loop([0, 1]) = {flat:?}, expected Some(1.0)"
    );
    let verdict = lattice.is_causally_invariant(&[&[0], &[1]]);
    assert_eq!(
        verdict,
        Some(true),
        "links 2.0 and 0.5: is_causally_invariant([0, 1]) = {verdict:?}, expected Some(true)"
    );

    assert!(lattice.record_transition(&[1], &[0], link1(0.25)));
    let curved = lattice.wilson_loop(&[&[0], &[1]]);
    assert_eq!(
        curved,
        Some(0.5),
        "links 2.0 and 0.25 at link_dim 1: wilson_loop([0, 1]) = {curved:?}, expected Some(0.5)"
    );
    let curved_verdict = lattice.is_causally_invariant(&[&[0], &[1]]);
    assert_eq!(
        curved_verdict,
        Some(false),
        "links 2.0 and 0.25: is_causally_invariant([0, 1]) = {curved_verdict:?}, \
         expected Some(false)"
    );
}

/// The `[3]` line of pin (b): `A` on 0→1, `B` on 1→2, `C` on 2→0.
fn ordering_line() -> (
    HypergraphLattice<1>,
    DMatrix<f64>,
    DMatrix<f64>,
    DMatrix<f64>,
) {
    let mut lattice: HypergraphLattice<1> =
        HypergraphLattice::new([3], HypergraphRewriteGroup::new(1), vec![], 2);
    let a = m2(1.0, 1.0, 0.0, 1.0);
    let b = m2(1.0, 0.0, 1.0, 1.0);
    let c = m2(2.0, 0.0, 0.0, 1.0);
    assert!(lattice.record_transition(&[0], &[1], a.clone()));
    assert!(lattice.record_transition(&[1], &[2], b.clone()));
    assert!(lattice.record_transition(&[2], &[0], c.clone()));
    (lattice, a, b, c)
}

/// Pin (b): `loop_holonomy` is the ordered product `U_k · … · U_1`, not its
/// reverse, and rotating the base point conjugates it.
#[test]
fn loop_holonomy_is_path_ordered() {
    let (lattice, a, b, c) = ordering_line();

    let holonomy = lattice
        .loop_holonomy(&[&[0], &[1], &[2]])
        .expect("invariant: all three links of the [0, 1, 2] cycle were just recorded");
    let expected = m2(2.0, 2.0, 1.0, 2.0);
    assert!(
        max_diff(&holonomy, &expected) < 1e-12,
        "loop_holonomy([0, 1, 2]) = {:?}, expected C·B·A = [[2, 2], [1, 2]]; \
         max entry difference {}",
        rows(&holonomy),
        max_diff(&holonomy, &expected)
    );
    assert_eq!(
        holonomy.trace(),
        4.0,
        "trace of C·B·A = {}, expected 4",
        holonomy.trace()
    );
    let wilson = lattice.wilson_loop(&[&[0], &[1], &[2]]);
    assert_eq!(
        wilson,
        Some(2.0),
        "wilson_loop([0, 1, 2]) = {wilson:?}, expected Some(2.0) = trace 4 / link_dim 2"
    );

    // The reversed product A·B·C is a different matrix of trace 5, so the
    // reversed reading would report a Wilson value of 2.5.
    let reversed = &a * &b * &c;
    let reversed_expected = m2(4.0, 1.0, 2.0, 1.0);
    assert!(
        max_diff(&reversed, &reversed_expected) < 1e-12,
        "A·B·C = {:?}, expected [[4, 1], [2, 1]]; max entry difference {}",
        rows(&reversed),
        max_diff(&reversed, &reversed_expected)
    );
    assert_eq!(
        reversed.trace(),
        5.0,
        "trace of A·B·C = {}, expected 5, which is Wilson 2.5",
        reversed.trace()
    );

    // Rotating the base point to site 1 gives A·C·B, the conjugate of C·B·A
    // by A, with the same trace.
    let rotated = lattice
        .loop_holonomy(&[&[1], &[2], &[0]])
        .expect("invariant: all three links of the [1, 2, 0] cycle were just recorded");
    let rotated_expected = m2(3.0, 1.0, 1.0, 1.0);
    assert!(
        max_diff(&rotated, &rotated_expected) < 1e-12,
        "loop_holonomy([1, 2, 0]) = {:?}, expected A·C·B = [[3, 1], [1, 1]]; \
         max entry difference {}",
        rows(&rotated),
        max_diff(&rotated, &rotated_expected)
    );
    assert_eq!(
        rotated.trace(),
        4.0,
        "trace of A·C·B = {}, expected 4",
        rotated.trace()
    );
    let a_inverse = a
        .clone()
        .try_inverse()
        .expect("invariant: A = [[1, 1], [0, 1]] has determinant 1");
    let conjugate = &a * &holonomy * &a_inverse;
    assert!(
        max_diff(&rotated, &conjugate) < 1e-12,
        "loop_holonomy([1, 2, 0]) = {:?}, expected A·(C·B·A)·A⁻¹ = {:?}; \
         max entry difference {}",
        rows(&rotated),
        rows(&conjugate),
        max_diff(&rotated, &conjugate)
    );
}

/// Pin (c): `gauge_transform` conjugates each link by its endpoints and leaves
/// the Wilson value fixed while moving the holonomy.
#[test]
fn gauge_transform_conjugates_links_and_fixes_the_wilson_value() {
    let (mut lattice, _a, _b, _c) = ordering_line();

    let before = lattice
        .loop_holonomy(&[&[0], &[1], &[2]])
        .expect("invariant: all three links of the [0, 1, 2] cycle were just recorded");
    assert_eq!(
        lattice.wilson_loop(&[&[0], &[1], &[2]]),
        Some(2.0),
        "wilson_loop before the gauge transformation: expected Some(2.0)"
    );
    assert_eq!(
        before[(0, 1)],
        2.0,
        "entry (0, 1) of C·B·A = {}, expected 2",
        before[(0, 1)]
    );

    let mut g: HashMap<Vec<usize>, DMatrix<f64>> = HashMap::new();
    g.insert(vec![0], m2(1.0, 2.0, 0.0, 1.0));
    g.insert(vec![1], m2(3.0, 0.0, 0.0, 1.0));
    // Site 2 is absent from g and transforms by the identity.
    assert!(
        lattice.gauge_transform(&g),
        "gauge_transform with invertible g_0 and g_1: got false, expected true"
    );

    for (from, to, expected, name) in [
        (0usize, 1usize, m2(3.0, -3.0, 0.0, 1.0), "g_1·A·g_0⁻¹"),
        (
            1,
            2,
            m2(1.0 / 3.0, 0.0, 1.0 / 3.0, 1.0),
            "g_2·B·g_1⁻¹ with g_2 = I",
        ),
        (2, 0, m2(2.0, 2.0, 0.0, 1.0), "g_0·C·g_2⁻¹ with g_2 = I"),
    ] {
        let link = lattice
            .link(&[from], &[to])
            .expect("invariant: the three links were recorded and gauge_transform kept the keys");
        assert!(
            max_diff(link, &expected) < 1e-12,
            "link({from}, {to}) after the gauge transformation = {:?}, \
             expected {name} = {:?}; max entry difference {}",
            rows(link),
            rows(&expected),
            max_diff(link, &expected)
        );
    }

    let after = lattice
        .loop_holonomy(&[&[0], &[1], &[2]])
        .expect("invariant: gauge_transform keeps the link key set");
    let after_expected = m2(4.0, -2.0, 1.0, 0.0);
    assert!(
        max_diff(&after, &after_expected) < 1e-12,
        "loop_holonomy([0, 1, 2]) after the gauge transformation = {:?}, \
         expected g_0·(C·B·A)·g_0⁻¹ = [[4, -2], [1, 0]]; max entry difference {}",
        rows(&after),
        max_diff(&after, &after_expected)
    );
    assert!(
        (after[(0, 1)] - (-2.0)).abs() < 1e-12,
        "entry (0, 1) after the gauge transformation = {}, expected -2 (it was 2 before)",
        after[(0, 1)]
    );

    let wilson_after = lattice
        .wilson_loop(&[&[0], &[1], &[2]])
        .expect("invariant: gauge_transform keeps the link key set");
    assert!(
        (wilson_after - 2.0).abs() < 1e-12,
        "wilson_loop after the gauge transformation = {wilson_after}, expected 2.0 as before"
    );
}

/// Pin (f): a Wilson value of 1.0 does not make a loop flat.
#[test]
fn wilson_value_one_is_not_flatness() {
    let mut lattice: HypergraphLattice<1> =
        HypergraphLattice::new([2], HypergraphRewriteGroup::new(1), vec![], 2);

    assert!(lattice.record_transition(&[0], &[1], m2(1.0, 1.0, 0.0, 1.0)));
    assert!(lattice.record_transition(&[1], &[0], m2(1.0, 0.0, 0.0, 1.0)));

    let wilson = lattice.wilson_loop(&[&[0], &[1]]);
    assert_eq!(
        wilson,
        Some(1.0),
        "shear [[1, 1], [0, 1]] then I: wilson_loop([0, 1]) = {wilson:?}, \
         expected Some(1.0) = trace 2 / link_dim 2"
    );
    let verdict = lattice.is_causally_invariant(&[&[0], &[1]]);
    assert_eq!(
        verdict,
        Some(false),
        "shear [[1, 1], [0, 1]] then I: is_causally_invariant([0, 1]) = {verdict:?}, \
         expected Some(false); the |wilson - 1| reading gives Some(true)"
    );
}

/// `is_flat` reads its `eps`: a holonomy whose largest entrywise deviation
/// from the identity is `1e-3` is flat at `1e-2` and not flat at `1e-4`, and
/// `is_causally_invariant` reads the same holonomy as not flat at its own
/// `1e-6`.
#[test]
fn is_flat_reads_its_eps() {
    let mut lattice: HypergraphLattice<1> =
        HypergraphLattice::new([2], HypergraphRewriteGroup::new(1), vec![], 2);

    // Holonomy of [0, 1] = I · [[1, 1e-3], [0, 1]], so the largest entrywise
    // deviation from the identity is 1e-3.
    assert!(lattice.record_transition(&[0], &[1], m2(1.0, 1e-3, 0.0, 1.0)));
    assert!(lattice.record_transition(&[1], &[0], m2(1.0, 0.0, 0.0, 1.0)));

    let path: [&[usize; 1]; 2] = [&[0], &[1]];

    let loose = lattice.is_flat(&path, 1e-2);
    assert_eq!(
        loose,
        Some(true),
        "deviation 1e-3: is_flat([0, 1], 1e-2) = {loose:?}, expected Some(true); \
         an is_flat that ignores eps and reads 1e-6 gives Some(false)"
    );

    let tight = lattice.is_flat(&path, 1e-4);
    assert_eq!(
        tight,
        Some(false),
        "deviation 1e-3: is_flat([0, 1], 1e-4) = {tight:?}, expected Some(false)"
    );

    let verdict = lattice.is_causally_invariant(&path);
    assert_eq!(
        verdict,
        Some(false),
        "deviation 1e-3: is_causally_invariant([0, 1]) = {verdict:?}, expected Some(false) \
         at its fixed 1e-6"
    );
}

/// Pin (g): an empty path has the identity holonomy.
#[test]
fn empty_path_holonomy_is_the_identity() {
    let lattice: HypergraphLattice<1> =
        HypergraphLattice::new([2], HypergraphRewriteGroup::new(1), vec![], 2);

    let path: Vec<&[usize; 1]> = vec![];
    let holonomy = lattice
        .loop_holonomy(&path)
        .expect("invariant: the empty cycle traverses no link, so nothing can be missing");
    let identity = DMatrix::<f64>::identity(2, 2);
    assert!(
        max_diff(&holonomy, &identity) < 1e-12,
        "loop_holonomy([]) at link_dim 2 = {:?}, expected I2; max entry difference {}",
        rows(&holonomy),
        max_diff(&holonomy, &identity)
    );
    assert_eq!(
        lattice.wilson_loop(&path),
        Some(1.0),
        "wilson_loop([]) at link_dim 2: expected Some(1.0) = trace 2 / link_dim 2"
    );
}

// ---------------------------------------------------------------------------
// record_transition and Wilson loop inter-site holonomy
// ---------------------------------------------------------------------------

#[test]
fn record_transition_populates_wilson_loop() {
    // Record inter-site transitions with known link variables and verify
    // that wilson_loop traverses them correctly.
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([3, 3], HypergraphRewriteGroup::new(3), vec![], 1);

    lattice.record_transition(&[0, 0], &[1, 0], link1(2.0));
    lattice.record_transition(&[1, 0], &[1, 1], link1(0.5));
    lattice.record_transition(&[1, 1], &[0, 1], link1(2.0));
    lattice.record_transition(&[0, 1], &[0, 0], link1(0.5));

    // Wilson loop around the plaquette: 2.0 * 0.5 * 2.0 * 0.5 = 1.0
    let holonomy = lattice
        .wilson_loop(&[&[0, 0], &[1, 0], &[1, 1], &[0, 1]])
        .expect("invariant: all four links of the plaquette were just recorded");
    assert!(
        (holonomy - 1.0).abs() < 1e-10,
        "closed plaquette should have Wilson value 1.0, got {holonomy}"
    );
}

#[test]
fn wilson_loop_non_trivial_holonomy() {
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([3, 3], HypergraphRewriteGroup::new(3), vec![], 1);

    // Non-unit holonomy loop
    lattice.record_transition(&[0, 0], &[1, 0], link1(2.0));
    lattice.record_transition(&[1, 0], &[0, 0], link1(3.0));

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
    // link is not read as the identity.
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([3, 3], HypergraphRewriteGroup::new(3), vec![], 1);

    lattice.record_transition(&[0, 0], &[1, 0], link1(3.0));
    // [1,0] -> [0,0] has no recorded transition

    assert_eq!(
        lattice.wilson_loop(&[&[0, 0], &[1, 0]]),
        None,
        "loop with one unrecorded link: expected None, the identity reading would give Some(3.0)"
    );
    assert_eq!(
        lattice.loop_holonomy(&[&[0, 0], &[1, 0]]),
        None,
        "loop with one unrecorded link: expected None"
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
    lattice.record_transition(&[1, 0], &[0, 0], link1(1.0 / 3.0));
    assert_eq!(
        lattice.is_causally_invariant(&[&[0, 0], &[1, 0]]),
        Some(true),
        "loop over 3.0 and 1/3: expected Some(true)"
    );
}

#[test]
fn apply_rewrite_does_not_record_transition() {
    // After the self-loop bug fix, apply_rewrite no longer inserts
    // a (site, site) transition, so a single-site Wilson loop has no
    // holonomy at all.
    let rule = RewriteRule::wolfram_a_to_bb();
    let mut lattice: HypergraphLattice<1> =
        HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![rule], 1);

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
    // A closed plaquette where all link variables multiply to the identity
    // should be detected as causally invariant.
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([3, 3], HypergraphRewriteGroup::new(3), vec![], 1);

    lattice.record_transition(&[0, 0], &[1, 0], link1(2.0));
    lattice.record_transition(&[1, 0], &[1, 1], link1(0.5));
    lattice.record_transition(&[1, 1], &[0, 1], link1(2.0));
    lattice.record_transition(&[0, 1], &[0, 0], link1(0.5));

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
        HypergraphLattice::new([3, 3], HypergraphRewriteGroup::new(3), vec![], 1);

    lattice.record_transition(&[0, 0], &[1, 0], link1(2.0));
    lattice.record_transition(&[1, 0], &[1, 1], link1(2.0));
    lattice.record_transition(&[1, 1], &[0, 1], link1(2.0));
    lattice.record_transition(&[0, 1], &[0, 0], link1(2.0));

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
        HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![rule], 1);

    // Place a binary edge -- won't match the A→BB rule
    let initial = Hypergraph::from_edges(vec![vec![0, 1]]);
    assert!(lattice.set_state(&[2], initial));

    assert!(!lattice.apply_rewrite(&[2], 0));
    assert_eq!(lattice.step_count(), 0);
}

#[test]
fn apply_rewrite_invalid_site() {
    let mut lattice: HypergraphLattice<1> =
        HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![], 1);

    // Site [5] is out of bounds for dimension size 5 (valid: 0..4)
    assert!(!lattice.apply_rewrite(&[5], 0));
    assert_eq!(lattice.step_count(), 0);
}

#[test]
fn apply_rewrite_invalid_rule() {
    let mut lattice: HypergraphLattice<1> =
        HypergraphLattice::new([5], HypergraphRewriteGroup::new(2), vec![], 1);

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
        HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![], 1);

    let path: Vec<&[usize; 1]> = vec![];
    assert_eq!(lattice.wilson_loop(&path), Some(1.0));
}

#[test]
fn wilson_loop_no_transitions() {
    let lattice: HypergraphLattice<2> =
        HypergraphLattice::new([4, 4], HypergraphRewriteGroup::new(3), vec![], 1);

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
        HypergraphLattice::new([5], HypergraphRewriteGroup::new(3), vec![], 1);

    // Single-site loop with no recorded self-link -> no verdict
    let s0 = [2];
    let path: Vec<&[usize; 1]> = vec![&s0];
    assert_eq!(lattice.is_causally_invariant(&path), None);
}

// ---------------------------------------------------------------------------
// find_wilson_loops: the max_length bound, the D range, the link requirement
// ---------------------------------------------------------------------------

/// Records every axis-aligned nearest-neighbour link of a `dims` lattice with
/// the 1 × 1 link `h` forward and `1.0 / h` back, so every elementary
/// plaquette is closed.
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
                assert!(lattice.record_transition(&site, &next, link1(h)));
                assert!(lattice.record_transition(&next, &site, link1(1.0 / h)));
            }
        }
    }
}

#[test]
fn find_wilson_loops_2d() {
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([3, 3], HypergraphRewriteGroup::new(2), vec![], 1);
    record_all_links(&mut lattice, [3, 3], 1.0);

    lattice.find_wilson_loops(4);

    // A 3x3 grid has (3-1)*(3-1) = 4 elementary plaquettes
    let loops = lattice.recorded_loops();
    assert_eq!(loops.len(), 4, "3x3 lattice should have 4 plaquettes");

    for (sites, wilson) in loops {
        assert_eq!(*wilson, 1.0);
        assert_eq!(sites.len(), 4, "each plaquette has 4 corners");
    }
}

#[test]
fn find_wilson_loops_honors_max_length() {
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([3, 3], HypergraphRewriteGroup::new(2), vec![], 1);
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
        HypergraphLattice::new([4], HypergraphRewriteGroup::new(2), vec![], 1);
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
        HypergraphLattice::new([2, 2, 2], HypergraphRewriteGroup::new(2), vec![], 1);
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
        HypergraphLattice::new([3, 3, 3], HypergraphRewriteGroup::new(2), vec![], 1);
    record_all_links(&mut big, [3, 3, 3], 1.0);
    big.find_wilson_loops(4);
    assert_eq!(
        big.recorded_loops().len(),
        36,
        "3x3x3 lattice: expected 36 plaquettes, the D==2-only reading records 0"
    );

    // 2x2x2x2: six axis pairs, each (2-1)(2-1)*2*2 = 4, so 24.
    let mut four: HypergraphLattice<4> =
        HypergraphLattice::new([2, 2, 2, 2], HypergraphRewriteGroup::new(2), vec![], 1);
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
        HypergraphLattice::new([2, 3, 4], HypergraphRewriteGroup::new(2), vec![], 1);
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
        HypergraphLattice::new([4, 3, 2], HypergraphRewriteGroup::new(2), vec![], 1);
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
    // A 3D lattice of identity links, with the four forward links of the
    // xy plaquette at [0, 0, 0] overwritten with the 1x1 link 2.0, so that
    // plaquette has holonomy 16.0.
    let mut lattice: HypergraphLattice<3> =
        HypergraphLattice::new([2, 2, 2], HypergraphRewriteGroup::new(2), vec![], 1);
    record_all_links(&mut lattice, [2, 2, 2], 1.0);
    for (from, to) in [
        ([0usize, 0, 0], [1usize, 0, 0]),
        ([1, 0, 0], [1, 1, 0]),
        ([1, 1, 0], [0, 1, 0]),
        ([0, 1, 0], [0, 0, 0]),
    ] {
        assert!(lattice.record_transition(&from, &to, link1(2.0)));
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
    // A 2x3 lattice of identity links carries two plaquettes; the four
    // links of the plaquette at [0, 0] are overwritten with the 1x1 link 2.0,
    // so the two Wilson values are 16.0 and 1.0.
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([2, 3], HypergraphRewriteGroup::new(2), vec![], 1);
    record_all_links(&mut lattice, [2, 3], 1.0);
    for (from, to) in [
        ([0usize, 0], [1usize, 0]),
        ([1, 0], [1, 1]),
        ([1, 1], [0, 1]),
        ([0, 1], [0, 0]),
    ] {
        assert!(lattice.record_transition(&from, &to, link1(2.0)));
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
        "Wilson values 16.0 and 1.0: expected Some(8.5); an undivided sum reads \
         Some(17.0), an unconditional empty reading reads None"
    );
}

#[test]
fn find_wilson_loops_skips_plaquettes_with_unrecorded_links() {
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([3, 3], HypergraphRewriteGroup::new(2), vec![], 1);

    // No links at all: nothing to record.
    lattice.find_wilson_loops(4);
    assert_eq!(
        lattice.recorded_loops().len(),
        0,
        "linkless 3x3 lattice: expected 0 plaquettes, the identity reading records 4"
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
        assert!(lattice.record_transition(&from, &to, link1(1.0)));
    }

    lattice.find_wilson_loops(4);
    let loops = lattice.recorded_loops();
    assert_eq!(
        loops.len(),
        1,
        "3x3 lattice with one closed plaquette: expected 1 plaquette, \
         the identity reading records 4"
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

// ---------------------------------------------------------------------------
// Pins (d), (h) — the plaquette surface at matrix link_dim
// ---------------------------------------------------------------------------

/// Pin (h): the global verdict, the mean Wilson value and the total action on
/// a single matrix plaquette.
#[test]
fn global_verdict_separates_flatness_from_the_wilson_value() {
    let corners = [[0usize, 0], [1, 0], [1, 1], [0, 1]];
    let links = [
        ([0usize, 0], [1usize, 0]),
        ([1, 0], [1, 1]),
        ([1, 1], [0, 1]),
        ([0, 1], [0, 0]),
    ];

    let mut sheared: HypergraphLattice<2> =
        HypergraphLattice::new([2, 2], HypergraphRewriteGroup::new(1), vec![], 2);
    for (index, (from, to)) in links.iter().enumerate() {
        let link = if index == 0 {
            m2(1.0, 1.0, 0.0, 1.0)
        } else {
            DMatrix::<f64>::identity(2, 2)
        };
        assert!(sheared.record_transition(from, to, link));
    }
    sheared.find_wilson_loops(4);
    assert_eq!(
        sheared.recorded_loops().len(),
        1,
        "[2, 2] lattice: expected 1 plaquette, got {}",
        sheared.recorded_loops().len()
    );
    assert_eq!(
        sheared.recorded_loops()[0].0,
        corners.iter().map(|s| s.to_vec()).collect::<Vec<_>>(),
        "the recorded plaquette should be the only one of a [2, 2] lattice"
    );

    let verdict = sheared.is_globally_causally_invariant();
    assert_eq!(
        verdict,
        Some(false),
        "one shear link and three identities: is_globally_causally_invariant() = {verdict:?}, \
         expected Some(false); the |wilson - 1| reading gives Some(true)"
    );
    let mean = sheared.average_holonomy();
    assert_eq!(
        mean,
        Some(1.0),
        "one shear link and three identities: average_holonomy() = {mean:?}, \
         expected Some(1.0) = trace 2 / link_dim 2"
    );
    let action = sheared.total_plaquette_action();
    assert_eq!(
        action, 0.0,
        "one shear link and three identities: total_plaquette_action() = {action}, \
         expected 0.0, the action of Wilson value 1.0"
    );

    let mut flat: HypergraphLattice<2> =
        HypergraphLattice::new([2, 2], HypergraphRewriteGroup::new(1), vec![], 2);
    for (from, to) in &links {
        assert!(flat.record_transition(from, to, DMatrix::<f64>::identity(2, 2)));
    }
    flat.find_wilson_loops(4);
    let flat_verdict = flat.is_globally_causally_invariant();
    assert_eq!(
        flat_verdict,
        Some(true),
        "identity on every link: is_globally_causally_invariant() = {flat_verdict:?}, \
         expected Some(true)"
    );
}

/// `gauge_transform` recomputes the recorded Wilson values from their site
/// cycles: a link overwritten after `find_wilson_loops` leaves the stored
/// value stale, and an empty gauge field moves `average_holonomy` to the
/// cycle's current value.
#[test]
fn gauge_transform_recomputes_the_recorded_wilson_values() {
    let links = [
        ([0usize, 0], [1usize, 0]),
        ([1, 0], [1, 1]),
        ([1, 1], [0, 1]),
        ([0, 1], [0, 0]),
    ];

    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([2, 2], HypergraphRewriteGroup::new(1), vec![], 2);
    for (from, to) in &links {
        assert!(lattice.record_transition(from, to, DMatrix::<f64>::identity(2, 2)));
    }
    lattice.find_wilson_loops(4);
    assert_eq!(
        lattice.average_holonomy(),
        Some(1.0),
        "identity on every link: average_holonomy() = {:?}, expected Some(1.0)",
        lattice.average_holonomy()
    );

    // Overwrite one link with 3·I. `record_transition` does not touch the
    // recorded loops, so the stored Wilson value is now stale at 1.0 while the
    // cycle's true value is trace(3·I) / 2 = 3.0.
    assert!(lattice.record_transition(&[0, 0], &[1, 0], m2(3.0, 0.0, 0.0, 3.0)));
    assert_eq!(
        lattice.average_holonomy(),
        Some(1.0),
        "after the overwrite and before gauge_transform: average_holonomy() = {:?}, \
         expected the stale Some(1.0)",
        lattice.average_holonomy()
    );

    // An empty gauge field is the identity at every site, so it changes no
    // link — the only thing it can change is the recorded Wilson values.
    let identity_field: HashMap<Vec<usize>, DMatrix<f64>> = HashMap::new();
    assert!(
        lattice.gauge_transform(&identity_field),
        "gauge_transform with an empty g: got false, expected true"
    );
    assert_eq!(
        lattice.link(&[0, 0], &[1, 0]),
        Some(&m2(3.0, 0.0, 0.0, 3.0)),
        "an empty g leaves every link where it was"
    );
    assert_eq!(
        lattice.average_holonomy(),
        Some(3.0),
        "after gauge_transform: average_holonomy() = {:?}, expected Some(3.0) = \
         trace(3·I) / link_dim 2; a gauge_transform that skips the recompute reads Some(1.0)",
        lattice.average_holonomy()
    );
    assert_eq!(
        lattice.total_plaquette_action(),
        0.0,
        "Wilson value 3.0 is above 1.0, so the plaquette action is 0.0, got {}",
        lattice.total_plaquette_action()
    );
}

/// Pin (d): over a seeded GL(2) and GL(3) link field on a `[4, 4]` lattice,
/// `gauge_transform` fixes every recorded Wilson value while moving every
/// recorded plaquette's holonomy matrix.
#[test]
fn gauge_transform_fixes_every_recorded_wilson_value() {
    for (dim, seed) in [(2usize, 160_u64), (3, 1_600_003)] {
        let mut rng = Lcg::new(seed);
        let mut lattice: HypergraphLattice<2> =
            HypergraphLattice::new([4, 4], HypergraphRewriteGroup::new(1), vec![], dim);

        for x in 0..4usize {
            for y in 0..4usize {
                for axis in 0..2usize {
                    let mut next = [x, y];
                    if next[axis] + 1 >= 4 {
                        continue;
                    }
                    next[axis] += 1;
                    assert!(
                        lattice.record_transition(&[x, y], &next, seeded_invertible(&mut rng, dim)),
                        "seeded link {:?} -> {next:?} at link_dim {dim}: got false, expected true",
                        [x, y]
                    );
                    assert!(
                        lattice.record_transition(&next, &[x, y], seeded_invertible(&mut rng, dim)),
                        "seeded link {next:?} -> {:?} at link_dim {dim}: got false, expected true",
                        [x, y]
                    );
                }
            }
        }

        lattice.find_wilson_loops(4);
        assert_eq!(
            lattice.recorded_loops().len(),
            9,
            "[4, 4] lattice at link_dim {dim}: recorded {} plaquettes, expected 9",
            lattice.recorded_loops().len()
        );

        let before: Vec<f64> = lattice.recorded_loops().iter().map(|(_, w)| *w).collect();
        let corners: Vec<Vec<[usize; 2]>> = lattice
            .recorded_loops()
            .iter()
            .map(|(sites, _)| corners_2d(sites))
            .collect();
        let holonomies_before: Vec<DMatrix<f64>> = corners
            .iter()
            .map(|corner| {
                let path: Vec<&[usize; 2]> = corner.iter().collect();
                lattice
                    .loop_holonomy(&path)
                    .expect("invariant: a recorded plaquette's four links are all recorded")
            })
            .collect();

        let mut g: HashMap<Vec<usize>, DMatrix<f64>> = HashMap::new();
        for x in 0..4usize {
            for y in 0..4usize {
                g.insert(vec![x, y], seeded_invertible(&mut rng, dim));
            }
        }
        assert!(
            lattice.gauge_transform(&g),
            "gauge_transform with a seeded invertible g at link_dim {dim}: \
             got false, expected true"
        );

        let after: Vec<f64> = lattice.recorded_loops().iter().map(|(_, w)| *w).collect();
        let worst = before
            .iter()
            .zip(&after)
            .map(|(b, a)| (a - b).abs())
            .fold(0.0_f64, f64::max);
        assert!(
            worst < 1e-9,
            "link_dim {dim}: the largest Wilson move under gauge_transform is {worst}, \
             expected below 1e-9; before = {before:?}, after = {after:?}"
        );

        let moves: Vec<f64> = corners
            .iter()
            .zip(&holonomies_before)
            .map(|(corner, holonomy_before)| {
                let path: Vec<&[usize; 2]> = corner.iter().collect();
                let holonomy_after = lattice
                    .loop_holonomy(&path)
                    .expect("invariant: gauge_transform keeps the link key set");
                max_diff(holonomy_before, &holonomy_after)
            })
            .collect();
        let (least, least_move) = moves.iter().enumerate().fold(
            (0usize, f64::INFINITY),
            |(index, smallest), (i, &moved)| {
                if moved < smallest {
                    (i, moved)
                } else {
                    (index, smallest)
                }
            },
        );
        assert!(
            least_move > 1e-6,
            "link_dim {dim}: the smallest holonomy move under gauge_transform is {least_move}, \
             on the plaquette at {:?}, expected more than 1e-6 on each of the {} recorded \
             plaquettes; moves = {moves:?}",
            corners[least],
            moves.len()
        );
    }
}
