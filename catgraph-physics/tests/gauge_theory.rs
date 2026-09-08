//! Integration tests for the gauge theory module.
//!
//! Tests structure constants, plaquette/total action functions, and
//! `HypergraphLattice` construction, state management, DPO rewriting, matrix
//! link variables, the `Rotation3<f64>` / `UnitQuaternion<f64>` /
//! `Isometry3<f64>` link variables, path-ordered loop holonomies, Wilson
//! loops, flatness, and gauge transformation by vertex conjugation.

#![cfg(feature = "gauge")]
#![allow(clippy::float_cmp)]

use std::collections::HashMap;
use std::f64::consts::{FRAC_PI_2, PI};

use catgraph_physics::hypergraph::{
    GaugeGroup, Hypergraph, HypergraphLattice, HypergraphRewriteGroup, LinkVariable, RewriteRule,
    plaquette_action, total_action,
};
use catgraph_testutil::Lcg;
use nalgebra::{
    DMatrix, Isometry3, Matrix3, Matrix4, Quaternion, Rotation3, Translation3, UnitQuaternion,
    Vector3,
};

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
#[should_panic(expected = "link_dim 0 has no identity")]
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

/// `is_flat` reads its `eps` at the value the caller passes, and compares
/// strictly: a holonomy whose largest entrywise deviation from the identity is
/// `1e-3` is flat at `1e-2`, not flat at `1e-4`, not flat at `5e-4` — the last
/// straddling `1e-3` under a tenfold scaling of `eps` — and not flat at `1e-3`
/// itself, where the deviation equals `eps`. `is_causally_invariant` reads the
/// same holonomy as not flat at its own `1e-6`.
#[test]
fn is_flat_reads_its_eps() {
    let mut lattice: HypergraphLattice<1> =
        HypergraphLattice::new([2], HypergraphRewriteGroup::new(1), vec![], 2);

    // Holonomy of [0, 1] = I · [[1, 1e-3], [0, 1]], so the largest entrywise
    // deviation from the identity is 1e-3.
    assert!(
        lattice.record_transition(&[0], &[1], m2(1.0, 1e-3, 0.0, 1.0)),
        "record_transition([0] -> [1], [[1, 1e-3], [0, 1]]): got false, expected true"
    );
    assert!(
        lattice.record_transition(&[1], &[0], m2(1.0, 0.0, 0.0, 1.0)),
        "record_transition([1] -> [0], I2): got false, expected true"
    );

    let path: [&[usize; 1]; 2] = [&[0], &[1]];

    let straddling = lattice.is_flat(&path, 5e-4);
    assert_eq!(
        straddling,
        Some(false),
        "deviation 1e-3: is_flat([0, 1], 5e-4) = {straddling:?}, expected Some(false); \
         an is_flat comparing against 10·eps = 5e-3 gives Some(true)"
    );

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

    // The deviation is the f64 nearest 1e-3 and so is this eps, so the two are
    // the same bit pattern and the comparison is decided by its strictness.
    let boundary = lattice.is_flat(&path, 1e-3);
    assert_eq!(
        boundary,
        Some(false),
        "deviation 1e-3: is_flat([0, 1], 1e-3) = {boundary:?}, expected Some(false); \
         an is_flat comparing d.abs() <= eps gives Some(true)"
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

/// `gauge_transform` uses the keys of `g` that are endpoints of a recorded
/// link and ignores the rest. On a `[3, 3]` lattice carrying the four links of
/// the plaquette at the origin, a `g` holding the usable key `[1, 0]` next to
/// four unusable ones — `[9, 9]` off the lattice, `[0]` of arity 1,
/// `[0, 0, 0]` of arity 3, and the in-bounds arity-2 `[2, 2]` that no recorded
/// link touches — returns `true`, moves the two links incident to `[1, 0]` to
/// `g_[1,0] · U` and `U · g_[1,0]⁻¹`, leaves the other two links where they
/// were, and leaves the plaquette's Wilson value at its gauge-invariant `8.5`.
///
/// A value on an unusable key is admissibility-checked all the same: a `g`
/// whose only key is the non-endpoint `[2, 2]`, carrying a singular matrix,
/// returns `false` and moves nothing.
#[test]
fn gauge_transform_uses_endpoint_keys_and_ignores_the_rest() {
    let links = [
        ([0usize, 0], [1usize, 0]),
        ([1, 0], [1, 1]),
        ([1, 1], [0, 1]),
        ([0, 1], [0, 0]),
    ];

    // [3, 3] rather than [2, 2] so that [2, 2] is an in-bounds site of arity 2
    // that no recorded link touches.
    let mut lattice: HypergraphLattice<2> =
        HypergraphLattice::new([3, 3], HypergraphRewriteGroup::new(1), vec![], 2);
    for (from, to) in &links {
        assert!(
            lattice.record_transition(from, to, m2(2.0, 0.0, 0.0, 1.0)),
            "seeding link {from:?} -> {to:?}: got false, expected true"
        );
    }
    lattice.find_wilson_loops(4);

    let links_before: Vec<DMatrix<f64>> = links
        .iter()
        .map(|(from, to)| {
            lattice
                .link(from, to)
                .expect("invariant: the four links were just recorded")
                .clone()
        })
        .collect();
    let loops_before: Vec<(Vec<Vec<usize>>, f64)> = lattice.recorded_loops().to_vec();
    assert_eq!(
        loops_before.len(),
        1,
        "[3, 3] lattice with only the origin plaquette linked: recorded {} \
         plaquettes, expected 1",
        loops_before.len()
    );
    assert_eq!(
        loops_before[0].1, 8.5,
        "four links of [[2, 0], [0, 1]]: the plaquette's Wilson value is {}, \
         expected 8.5 = trace([[16, 0], [0, 1]]) / link_dim 2",
        loops_before[0].1
    );

    // [2, 2] is in bounds and no link's endpoint; its value is singular
    // (determinant 0), and the whole gauge field is rejected for it.
    let mut singular_on_a_non_endpoint: HashMap<Vec<usize>, DMatrix<f64>> = HashMap::new();
    singular_on_a_non_endpoint.insert(vec![2, 2], m2(1.0, 2.0, 2.0, 4.0));
    assert!(
        !lattice.gauge_transform(&singular_on_a_non_endpoint),
        "gauge_transform whose only key is the non-endpoint [2, 2] carrying the \
         singular [[1, 2], [2, 4]]: got true, expected false"
    );
    for ((from, to), before) in links.iter().zip(&links_before) {
        let after = lattice
            .link(from, to)
            .expect("invariant: a rejected gauge_transform keeps the link key set");
        assert_eq!(
            after,
            before,
            "link({from:?}, {to:?}) after the rejected gauge_transform = {:?}, \
             expected the unchanged {:?}",
            rows(after),
            rows(before)
        );
    }
    assert_eq!(
        lattice.recorded_loops(),
        loops_before.as_slice(),
        "recorded loops after the rejected gauge_transform = {:?}, expected the \
         unchanged {loops_before:?}",
        lattice.recorded_loops()
    );

    // Every value is 2 × 2 and invertible. `[1, 0]` is an endpoint of two
    // recorded links; the other four keys are no link's endpoint — `[9, 9]` is
    // off a [3, 3] lattice, `[0]` has arity 1, `[0, 0, 0]` arity 3, and
    // `[2, 2]` is in bounds and unlinked.
    let mut g: HashMap<Vec<usize>, DMatrix<f64>> = HashMap::new();
    g.insert(vec![1, 0], m2(2.0, 0.0, 0.0, 4.0));
    g.insert(vec![9, 9], m2(5.0, 0.0, 0.0, 7.0));
    g.insert(vec![0], m2(3.0, 0.0, 0.0, 3.0));
    g.insert(vec![0, 0, 0], m2(1.0, 4.0, 0.0, 1.0));
    g.insert(vec![2, 2], m2(6.0, 0.0, 0.0, 9.0));

    assert!(
        lattice.gauge_transform(&g),
        "gauge_transform with one endpoint key, an off-lattice key, two \
         wrong-arity keys and an unlinked in-bounds key: got false, expected true"
    );

    // g_[1,0] = [[2, 0], [0, 4]], so [0,0] -> [1,0] becomes g · U = [[4, 0],
    // [0, 4]] and [1,0] -> [1,1] becomes U · g⁻¹ = [[1, 0], [0, 0.25]]. The
    // other two links touch no key of g and keep U = [[2, 0], [0, 1]].
    let expected_after = [
        (m2(4.0, 0.0, 0.0, 4.0), true),
        (m2(1.0, 0.0, 0.0, 0.25), true),
        (m2(2.0, 0.0, 0.0, 1.0), false),
        (m2(2.0, 0.0, 0.0, 1.0), false),
    ];

    for (((from, to), before), (expected, moves)) in
        links.iter().zip(&links_before).zip(&expected_after)
    {
        let after = lattice
            .link(from, to)
            .expect("invariant: gauge_transform keeps the link key set");
        assert!(
            max_diff(after, expected) < 1e-12,
            "link({from:?}, {to:?}) after the transformation = {:?}, expected {:?}; \
             max entry difference {}",
            rows(after),
            rows(expected),
            max_diff(after, expected)
        );
        let travel = max_diff(after, before);
        assert_eq!(
            travel > 0.0,
            *moves,
            "link({from:?}, {to:?}) moved by {travel} (from {:?} to {:?}); \
             expected it to move: {moves}",
            rows(before),
            rows(after)
        );
    }

    let loops_after = lattice.recorded_loops();
    assert_eq!(
        loops_after,
        loops_before.as_slice(),
        "recorded loops after the transformation = {loops_after:?}, expected the \
         unchanged {loops_before:?}; conjugation fixes the Wilson value"
    );
}

/// Two `1e200 · I` links are each admissible, and their product overflows: the
/// loop holonomy is infinite on the diagonal, `wilson_loop` reads
/// `Some(f64::INFINITY)`, and `is_flat` is `Some(false)` at every finite `eps`
/// tried, `1e300` included.
#[test]
fn overflowing_link_product_reads_as_an_infinite_wilson_value() {
    let mut lattice: HypergraphLattice<1> =
        HypergraphLattice::new([2], HypergraphRewriteGroup::new(1), vec![], 2);

    let big = DMatrix::<f64>::identity(2, 2) * 1e200;
    assert!(
        lattice.record_transition(&[0], &[1], big.clone()),
        "record_transition([0] -> [1], 1e200·I): got false, expected true — \
         every entry is finite and the matrix inverts"
    );
    assert!(
        lattice.record_transition(&[1], &[0], big),
        "record_transition([1] -> [0], 1e200·I): got false, expected true"
    );

    let path: [&[usize; 1]; 2] = [&[0], &[1]];
    let holonomy = lattice
        .loop_holonomy(&path)
        .expect("invariant: both links of the [0, 1] cycle were just recorded");
    for (i, j) in [(0usize, 0usize), (1, 1)] {
        assert_eq!(
            holonomy[(i, j)],
            f64::INFINITY,
            "entry ({i}, {j}) of the 1e200·I product = {}, expected inf (1e400 overflows)",
            holonomy[(i, j)]
        );
    }
    for (i, j) in [(0usize, 1usize), (1, 0)] {
        assert_eq!(
            holonomy[(i, j)],
            0.0,
            "entry ({i}, {j}) of the 1e200·I product = {}, expected 0",
            holonomy[(i, j)]
        );
    }

    let wilson = lattice.wilson_loop(&path);
    assert_eq!(
        wilson,
        Some(f64::INFINITY),
        "wilson_loop over the overflowing product = {wilson:?}, expected \
         Some(inf) = trace(inf·I) / link_dim 2"
    );

    for eps in [1e-6, 1.0, 1e300] {
        let verdict = lattice.is_flat(&path, eps);
        assert_eq!(
            verdict,
            Some(false),
            "is_flat over the overflowing product at eps {eps:e} = {verdict:?}, \
             expected Some(false); the deviation inf is below no finite eps"
        );
    }
}

/// A path may repeat a site. A path visiting one site 1, 2 or 3 times
/// traverses that site's self-link that many times, and the cycle
/// `[0, 1, 0, 2]` multiplies site 0's two outgoing links in traversal order.
/// A one-site path at a site whose self-link is unrecorded reads `None`.
#[test]
fn holonomy_traverses_a_repeated_site_once_per_visit() {
    let mut lattice: HypergraphLattice<1> =
        HypergraphLattice::new([3], HypergraphRewriteGroup::new(1), vec![], 2);

    let u = m2(1.0, 1.0, 0.0, 1.0);
    assert!(
        lattice.record_transition(&[0], &[0], u.clone()),
        "record_transition([0] -> [0], [[1, 1], [0, 1]]): got false, expected true"
    );

    for (visits, expected) in [
        (1usize, m2(1.0, 1.0, 0.0, 1.0)),
        (2, m2(1.0, 2.0, 0.0, 1.0)),
        (3, m2(1.0, 3.0, 0.0, 1.0)),
    ] {
        let site: [usize; 1] = [0];
        let path: Vec<&[usize; 1]> = (0..visits).map(|_| &site).collect();
        let Some(holonomy) = lattice.loop_holonomy(&path) else {
            panic!(
                "loop_holonomy over {visits} visits to site 0 = None, expected \
                 Some(U^{visits}); site 0 carries a recorded self-link"
            )
        };
        assert!(
            max_diff(&holonomy, &expected) < 1e-12,
            "loop_holonomy over {visits} visits to site 0 = {:?}, expected U^{visits} \
             = {:?}; max entry difference {}",
            rows(&holonomy),
            rows(&expected),
            max_diff(&holonomy, &expected)
        );
        let wilson = lattice.wilson_loop(&path);
        assert_eq!(
            wilson,
            Some(1.0),
            "wilson_loop over {visits} visits to site 0 = {wilson:?}, expected \
             Some(1.0); U^{visits} is unipotent, so its trace is 2"
        );
    }

    let unrecorded: [&[usize; 1]; 1] = [&[1]];
    assert_eq!(
        lattice.loop_holonomy(&unrecorded),
        None,
        "site 1 carries no self-link, so its one-site path has no holonomy"
    );

    // A cycle that leaves site 0, returns, and leaves again.
    for (from, to, link, name) in [
        (0usize, 1usize, m2(1.0, 1.0, 0.0, 1.0), "A"),
        (1, 0, m2(1.0, 0.0, 1.0, 1.0), "B"),
        (0, 2, m2(2.0, 0.0, 0.0, 1.0), "C"),
        (2, 0, m2(1.0, 0.0, 0.0, 3.0), "D"),
    ] {
        assert!(
            lattice.record_transition(&[from], &[to], link),
            "record_transition([{from}] -> [{to}], {name}): got false, expected true"
        );
    }

    let revisit: [&[usize; 1]; 4] = [&[0], &[1], &[0], &[2]];
    let Some(holonomy) = lattice.loop_holonomy(&revisit) else {
        panic!(
            "loop_holonomy([0, 1, 0, 2]) = None, expected Some(D·C·B·A); all four \
             links of the cycle are recorded"
        )
    };
    let expected = m2(2.0, 2.0, 3.0, 6.0);
    assert!(
        max_diff(&holonomy, &expected) < 1e-12,
        "loop_holonomy([0, 1, 0, 2]) = {:?}, expected D·C·B·A = [[2, 2], [3, 6]]; \
         max entry difference {}",
        rows(&holonomy),
        max_diff(&holonomy, &expected)
    );
    let wilson = lattice.wilson_loop(&revisit);
    assert_eq!(
        wilson,
        Some(4.0),
        "wilson_loop([0, 1, 0, 2]) = {wilson:?}, expected Some(4.0) = trace 8 / link_dim 2"
    );
}

// ---------------------------------------------------------------------------
// Typed SO(3) / SE(3) link variables
// ---------------------------------------------------------------------------

/// The quarter-turn about the z axis.
fn rz() -> Rotation3<f64> {
    Rotation3::from_axis_angle(&Vector3::z_axis(), FRAC_PI_2)
}

/// The quarter-turn about the x axis.
fn rx() -> Rotation3<f64> {
    Rotation3::from_axis_angle(&Vector3::x_axis(), FRAC_PI_2)
}

/// The quarter-turn about the y axis.
fn ry() -> Rotation3<f64> {
    Rotation3::from_axis_angle(&Vector3::y_axis(), FRAC_PI_2)
}

/// `[[0, -1, 0], [1, 0, 0], [0, 0, 1]]`, the matrix of [`rz`].
fn rz_matrix() -> Matrix3<f64> {
    Matrix3::new(0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0)
}

/// `[[1, 0, 0], [0, 0, -1], [0, 1, 0]]`, the matrix of [`rx`].
fn rx_matrix() -> Matrix3<f64> {
    Matrix3::new(1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 1.0, 0.0)
}

/// `[[0, 0, 1], [0, 1, 0], [-1, 0, 0]]`, the matrix of [`ry`].
fn ry_matrix() -> Matrix3<f64> {
    Matrix3::new(0.0, 0.0, 1.0, 0.0, 1.0, 0.0, -1.0, 0.0, 0.0)
}

/// The quarter-turn about the z axis as a unit quaternion.
fn qz() -> UnitQuaternion<f64> {
    UnitQuaternion::from_axis_angle(&Vector3::z_axis(), FRAC_PI_2)
}

/// The rotation of `angle` about the z axis as a translation-free isometry.
fn iso_z(angle: f64) -> Isometry3<f64> {
    Isometry3::new(Vector3::zeros(), Vector3::z() * angle)
}

/// Largest absolute entrywise difference between two 3 × 3 matrices.
fn diff3(left: &Matrix3<f64>, right: &Matrix3<f64>) -> f64 {
    (left - right)
        .iter()
        .fold(0.0_f64, |acc, d| acc.max(d.abs()))
}

/// Largest absolute entrywise difference between two 4 × 4 matrices.
fn diff4(left: &Matrix4<f64>, right: &Matrix4<f64>) -> f64 {
    (left - right)
        .iter()
        .fold(0.0_f64, |acc, d| acc.max(d.abs()))
}

/// Records `link` on every hop of the four-site cycle `[0] → [1] → [2] → [3] →
/// [0]` of a `[4]` line lattice at `link_dim`, and returns that cycle's
/// flatness verdict at `1e-6`.
fn closure_is_flat<L: LinkVariable>(link: &L, link_dim: usize) -> Option<bool> {
    let mut lattice: HypergraphLattice<1, L> =
        HypergraphLattice::new([4], HypergraphRewriteGroup::new(1), vec![], link_dim);
    let hops: [([usize; 1], [usize; 1]); 4] = [([0], [1]), ([1], [2]), ([2], [3]), ([3], [0])];
    for (from, to) in hops {
        assert!(
            lattice.record_transition(&from, &to, link.clone()),
            "record_transition({from:?} -> {to:?}) = false, expected true"
        );
    }
    let path: Vec<&[usize; 1]> = vec![&[0], &[1], &[2], &[3]];
    lattice.is_flat(&path, 1e-6)
}

// --- (a) construction ------------------------------------------------------

#[test]
fn typed_lattices_carry_their_defining_representation_dimension() {
    let group = HypergraphRewriteGroup::new(1);

    let rotation: HypergraphLattice<1, Rotation3<f64>> =
        HypergraphLattice::new([3], group, vec![], 3);
    let quaternion: HypergraphLattice<1, UnitQuaternion<f64>> =
        HypergraphLattice::new([3], group, vec![], 3);
    let isometry: HypergraphLattice<1, Isometry3<f64>> =
        HypergraphLattice::new([3], group, vec![], 4);

    assert_eq!(
        rotation.link_dim(),
        3,
        "Rotation3 lattice link_dim = {}, expected 3",
        rotation.link_dim()
    );
    assert_eq!(
        quaternion.link_dim(),
        3,
        "UnitQuaternion lattice link_dim = {}, expected 3",
        quaternion.link_dim()
    );
    assert_eq!(
        isometry.link_dim(),
        4,
        "Isometry3 lattice link_dim = {}, expected 4",
        isometry.link_dim()
    );
}

#[test]
#[should_panic(expected = "link_dim 2 has no identity")]
fn rotation3_lattice_rejects_link_dim_2() {
    let _lattice: HypergraphLattice<1, Rotation3<f64>> =
        HypergraphLattice::new([3], HypergraphRewriteGroup::new(1), vec![], 2);
}

#[test]
#[should_panic(expected = "link_dim 4 has no identity")]
fn unit_quaternion_lattice_rejects_link_dim_4() {
    let _lattice: HypergraphLattice<1, UnitQuaternion<f64>> =
        HypergraphLattice::new([3], HypergraphRewriteGroup::new(1), vec![], 4);
}

#[test]
#[should_panic(expected = "link_dim 3 has no identity")]
fn isometry3_lattice_rejects_link_dim_3() {
    let _lattice: HypergraphLattice<1, Isometry3<f64>> =
        HypergraphLattice::new([3], HypergraphRewriteGroup::new(1), vec![], 3);
}

// --- (b) SO(3) path ordering ----------------------------------------------

#[test]
fn so3_loop_holonomy_is_path_ordered() {
    let group = HypergraphRewriteGroup::new(1);
    let mut lattice: HypergraphLattice<1, Rotation3<f64>> =
        HypergraphLattice::new([3], group, vec![], 3);

    assert!(lattice.record_transition(&[0], &[1], rz()), "A on [0]->[1]");
    assert!(lattice.record_transition(&[1], &[2], rx()), "B on [1]->[2]");
    assert!(lattice.record_transition(&[2], &[0], ry()), "C on [2]->[0]");

    let path: Vec<&[usize; 1]> = vec![&[0], &[1], &[2]];
    let holonomy = lattice
        .loop_holonomy(&path)
        .expect("invariant: all three links of the cycle were just recorded");
    let d = diff3(holonomy.matrix(), &rx_matrix());
    assert!(
        d < 1e-14,
        "loop_holonomy([0, 1, 2]) = {:?}, expected C·B·A = Rx = [[1,0,0],[0,0,-1],[0,1,0]]; \
         max entry difference {d}, expected < 1e-14",
        holonomy.matrix()
    );
    let wilson = lattice
        .wilson_loop(&path)
        .expect("invariant: the holonomy of this cycle exists");
    assert!(
        (wilson - 1.0 / 3.0).abs() < 1e-14,
        "wilson_loop([0, 1, 2]) = {wilson}, expected {} = trace(Rx) 1 / 3",
        1.0 / 3.0
    );
    let flat = lattice.is_flat(&path, 1e-6);
    assert_eq!(
        flat,
        Some(false),
        "is_flat([0, 1, 2], 1e-6) = {flat:?}, expected Some(false) — C·B·A is Rx, not I"
    );

    // Rotating the base point conjugates the holonomy and keeps the trace.
    let rotated: Vec<&[usize; 1]> = vec![&[1], &[2], &[0]];
    let conjugate = lattice
        .loop_holonomy(&rotated)
        .expect("invariant: the same three links carry this cycle");
    let dr = diff3(conjugate.matrix(), &ry_matrix());
    assert!(
        dr < 1e-14,
        "loop_holonomy([1, 2, 0]) = {:?}, expected A·C·B = Ry = [[0,0,1],[0,1,0],[-1,0,0]]; \
         max entry difference {dr}, expected < 1e-14",
        conjugate.matrix()
    );
    let rotated_wilson = lattice
        .wilson_loop(&rotated)
        .expect("invariant: the holonomy of this cycle exists");
    assert!(
        (rotated_wilson - 1.0 / 3.0).abs() < 1e-14,
        "wilson_loop([1, 2, 0]) = {rotated_wilson}, expected {} = trace(Ry) 1 / 3",
        1.0 / 3.0
    );

    // Assigning the same three rotations in the opposite direction around the
    // cycle gives the reversed product, which the trace separates.
    let mut reversed: HypergraphLattice<1, Rotation3<f64>> =
        HypergraphLattice::new([3], group, vec![], 3);
    assert!(
        reversed.record_transition(&[0], &[1], ry()),
        "C on [0]->[1]"
    );
    assert!(
        reversed.record_transition(&[1], &[2], rx()),
        "B on [1]->[2]"
    );
    assert!(
        reversed.record_transition(&[2], &[0], rz()),
        "A on [2]->[0]"
    );
    let backwards = reversed
        .loop_holonomy(&path)
        .expect("invariant: all three links of the cycle were just recorded");
    let expected_backwards = Matrix3::new(-1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0);
    let db = diff3(backwards.matrix(), &expected_backwards);
    assert!(
        db < 1e-14,
        "reversed loop_holonomy([0, 1, 2]) = {:?}, expected A·B·C = \
         [[-1,0,0],[0,0,1],[0,1,0]]; max entry difference {db}, expected < 1e-14",
        backwards.matrix()
    );
    let backwards_wilson = reversed
        .wilson_loop(&path)
        .expect("invariant: the holonomy of this cycle exists");
    assert!(
        (backwards_wilson + 1.0 / 3.0).abs() < 1e-14,
        "reversed wilson_loop([0, 1, 2]) = {backwards_wilson}, expected {} = trace -1 / 3",
        -1.0 / 3.0
    );
}

// --- (c) SO(3) closure -----------------------------------------------------

#[test]
fn so3_four_quarter_turns_close_the_loop() {
    let group = HypergraphRewriteGroup::new(1);
    let mut lattice: HypergraphLattice<1, Rotation3<f64>> =
        HypergraphLattice::new([4], group, vec![], 3);
    let hops: [([usize; 1], [usize; 1]); 4] = [([0], [1]), ([1], [2]), ([2], [3]), ([3], [0])];
    for (from, to) in hops {
        assert!(
            lattice.record_transition(&from, &to, rz()),
            "record_transition({from:?} -> {to:?}) = false, expected true"
        );
    }

    let path: Vec<&[usize; 1]> = vec![&[0], &[1], &[2], &[3]];
    let holonomy = lattice
        .loop_holonomy(&path)
        .expect("invariant: all four links of the cycle were just recorded");
    let d = diff3(holonomy.matrix(), &Matrix3::identity());
    assert!(
        d < 1e-12,
        "loop_holonomy of four z quarter-turns = {:?}, expected I3; \
         max entry difference {d}, expected < 1e-12",
        holonomy.matrix()
    );
    let wilson = lattice
        .wilson_loop(&path)
        .expect("invariant: the holonomy of this cycle exists");
    assert!(
        (wilson - 1.0).abs() < 1e-12,
        "wilson_loop of Rz^4 = {wilson}, expected 1.0 = trace(I3) 3 / 3"
    );
    let flat = lattice.is_flat(&path, 1e-6);
    assert_eq!(
        flat,
        Some(true),
        "is_flat of Rz^4 at 1e-6 = {flat:?}, expected Some(true)"
    );

    // Closing the cycle with the inverse quarter-turn leaves the z half-turn.
    let closing = LinkVariable::inverse(&rz()).expect("invariant: every rotation inverts");
    assert!(
        lattice.record_transition(&[3], &[0], closing),
        "record_transition([3] -> [0], Rz^-1) = false, expected true"
    );
    let curved = lattice
        .loop_holonomy(&path)
        .expect("invariant: all four links of the cycle carry a transition");
    let expected = Matrix3::new(-1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0);
    let dc = diff3(curved.matrix(), &expected);
    assert!(
        dc < 1e-12,
        "loop_holonomy with Rz^-1 closing = {:?}, expected Rz^2 = \
         [[-1,0,0],[0,-1,0],[0,0,1]]; max entry difference {dc}, expected < 1e-12",
        curved.matrix()
    );
    let curved_wilson = lattice
        .wilson_loop(&path)
        .expect("invariant: the holonomy of this cycle exists");
    assert!(
        (curved_wilson + 1.0 / 3.0).abs() < 1e-12,
        "wilson_loop of Rz^2 = {curved_wilson}, expected {} = trace -1 / 3",
        -1.0 / 3.0
    );
    let action = lattice.plaquette_action(&path);
    assert_eq!(
        action,
        Some(f64::INFINITY),
        "plaquette_action of Rz^2 = {action:?}, expected Some(inf) — its Wilson value is negative"
    );
}

// --- (d) SE(3) -------------------------------------------------------------

#[test]
fn se3_holonomy_reads_the_homogeneous_matrix() {
    let group = HypergraphRewriteGroup::new(1);
    let path: Vec<&[usize; 1]> = vec![&[0], &[1]];

    // A translation and its inverse close the loop.
    let mut closed: HypergraphLattice<1, Isometry3<f64>> =
        HypergraphLattice::new([2], group, vec![], 4);
    let shift = Isometry3::translation(1.0, 2.0, 3.0);
    assert!(
        closed.record_transition(&[0], &[1], shift),
        "record_transition([0] -> [1], T(1,2,3)) = false, expected true"
    );
    assert!(
        closed.record_transition(&[1], &[0], shift.inverse()),
        "record_transition([1] -> [0], T(1,2,3)^-1) = false, expected true"
    );
    let holonomy = closed
        .loop_holonomy(&path)
        .expect("invariant: both links of the two-site loop were just recorded");
    let d = diff4(&holonomy.to_homogeneous(), &Matrix4::identity());
    assert!(
        d < 1e-12,
        "loop_holonomy of T and T^-1 = {:?}, expected I4; \
         max entry difference {d}, expected < 1e-12",
        holonomy.to_homogeneous()
    );
    let wilson = closed
        .wilson_loop(&path)
        .expect("invariant: the holonomy of this loop exists");
    assert!(
        (wilson - 1.0).abs() < 1e-12,
        "wilson_loop of T·T^-1 = {wilson}, expected 1.0 = trace(I4) 4 / 4"
    );
    let flat = closed.is_flat(&path, 1e-6);
    assert_eq!(
        flat,
        Some(true),
        "is_flat of T·T^-1 at 1e-6 = {flat:?}, expected Some(true)"
    );

    // A surviving pure translation is Wilson-blind but not flat.
    let mut shifted: HypergraphLattice<1, Isometry3<f64>> =
        HypergraphLattice::new([2], group, vec![], 4);
    assert!(
        shifted.record_transition(&[0], &[1], Isometry3::translation(1.0, 0.0, 0.0)),
        "record_transition([0] -> [1], T(1,0,0)) = false, expected true"
    );
    assert!(
        shifted.record_transition(&[1], &[0], Isometry3::identity()),
        "record_transition([1] -> [0], identity) = false, expected true"
    );
    let shifted_wilson = shifted
        .wilson_loop(&path)
        .expect("invariant: the holonomy of this loop exists");
    assert!(
        (shifted_wilson - 1.0).abs() < 1e-12,
        "wilson_loop of T(1,0,0) = {shifted_wilson}, expected 1.0 = (trace(I3) 3 + 1) / 4"
    );
    let shifted_flat = shifted.is_flat(&path, 1e-6);
    assert_eq!(
        shifted_flat,
        Some(false),
        "is_flat of T(1,0,0) at 1e-6 = {shifted_flat:?}, expected Some(false) — \
         entry (0, 3) of the homogeneous matrix is 1"
    );

    // The rotation part alone sets the Wilson value.
    let mut quarter: HypergraphLattice<1, Isometry3<f64>> =
        HypergraphLattice::new([2], group, vec![], 4);
    assert!(
        quarter.record_transition(&[0], &[1], iso_z(FRAC_PI_2)),
        "record_transition([0] -> [1], Rz(pi/2)) = false, expected true"
    );
    assert!(
        quarter.record_transition(&[1], &[0], Isometry3::identity()),
        "record_transition([1] -> [0], identity) = false, expected true"
    );
    let quarter_wilson = quarter
        .wilson_loop(&path)
        .expect("invariant: the holonomy of this loop exists");
    assert!(
        (quarter_wilson - 0.5).abs() < 1e-12,
        "wilson_loop of the SE(3) z quarter-turn = {quarter_wilson}, \
         expected 0.5 = (trace(Rz) 1 + 1) / 4"
    );

    let mut half: HypergraphLattice<1, Isometry3<f64>> =
        HypergraphLattice::new([2], group, vec![], 4);
    assert!(
        half.record_transition(&[0], &[1], iso_z(PI)),
        "record_transition([0] -> [1], Rz(pi)) = false, expected true"
    );
    assert!(
        half.record_transition(&[1], &[0], Isometry3::identity()),
        "record_transition([1] -> [0], identity) = false, expected true"
    );
    let half_wilson = half
        .wilson_loop(&path)
        .expect("invariant: the holonomy of this loop exists");
    assert!(
        half_wilson.abs() < 1e-12,
        "wilson_loop of the SE(3) z half-turn = {half_wilson}, \
         expected 0.0 = (trace(Rz(pi)) -1 + 1) / 4"
    );
    let half_action = half.plaquette_action(&path);
    assert_eq!(
        half_action,
        Some(f64::INFINITY),
        "plaquette_action of the SE(3) z half-turn = {half_action:?}, \
         expected Some(inf) — its Wilson value is 0.0"
    );
}

// --- (e) SE(3) gauge transformation ---------------------------------------

#[test]
fn se3_gauge_transform_conjugates_the_holonomy_and_fixes_its_wilson_value() {
    let group = HypergraphRewriteGroup::new(1);
    let mut lattice: HypergraphLattice<1, Isometry3<f64>> =
        HypergraphLattice::new([2], group, vec![], 4);
    let turn = iso_z(FRAC_PI_2);
    assert!(
        lattice.record_transition(&[0], &[1], turn),
        "record_transition([0] -> [1], Rz(pi/2)) = false, expected true"
    );
    assert!(
        lattice.record_transition(&[1], &[0], Isometry3::identity()),
        "record_transition([1] -> [0], identity) = false, expected true"
    );

    let path: Vec<&[usize; 1]> = vec![&[0], &[1]];
    let before = lattice
        .wilson_loop(&path)
        .expect("invariant: both links of the two-site loop were just recorded");
    assert!(
        (before - 0.5).abs() < 1e-12,
        "wilson_loop before gauge_transform = {before}, expected 0.5 = (1 + 1) / 4"
    );
    let flat_before = lattice.is_flat(&path, 1e-6);
    assert_eq!(
        flat_before,
        Some(false),
        "is_flat before gauge_transform = {flat_before:?}, expected Some(false)"
    );

    // A gauge value with a non-finite translation is rejected outright.
    let mut bad: HashMap<Vec<usize>, Isometry3<f64>> = HashMap::new();
    bad.insert(vec![0], Isometry3::translation(f64::NAN, 0.0, 0.0));
    let rejected = lattice.gauge_transform(&bad);
    assert!(
        !rejected,
        "gauge_transform with a NaN translation = {rejected}, expected false"
    );
    let unchanged = lattice.link(&[0], &[1]);
    assert_eq!(
        unchanged,
        Some(&turn),
        "link([0] -> [1]) after the rejected gauge_transform = {unchanged:?}, \
         expected the recorded Rz(pi/2)"
    );

    let mut g: HashMap<Vec<usize>, Isometry3<f64>> = HashMap::new();
    g.insert(vec![0], Isometry3::translation(5.0, 0.0, 0.0));
    let accepted = lattice.gauge_transform(&g);
    assert!(
        accepted,
        "gauge_transform with g_0 = T(5,0,0) = {accepted}, expected true"
    );

    let holonomy = lattice
        .loop_holonomy(&path)
        .expect("invariant: gauge_transform keeps the link key set");
    let rotation_diff = diff3(
        holonomy.rotation.to_rotation_matrix().matrix(),
        &rz_matrix(),
    );
    assert!(
        rotation_diff < 1e-12,
        "rotation of g_0·U·g_0^-1 = {:?}, expected Rz(pi/2) = [[0,-1,0],[1,0,0],[0,0,1]]; \
         max entry difference {rotation_diff}, expected < 1e-12",
        holonomy.rotation.to_rotation_matrix().matrix()
    );
    let translation = holonomy.translation.vector;
    let expected_translation = Vector3::new(5.0, -5.0, 0.0);
    let translation_diff = (translation - expected_translation)
        .iter()
        .fold(0.0_f64, |acc, d| acc.max(d.abs()));
    assert!(
        translation_diff < 1e-12,
        "translation of g_0·U·g_0^-1 = {translation:?}, expected s - R s = \
         (5, 0, 0) - (0, 5, 0) = (5, -5, 0); max component difference \
         {translation_diff}, expected < 1e-12"
    );
    let after = lattice
        .wilson_loop(&path)
        .expect("invariant: gauge_transform keeps the link key set");
    assert!(
        (after - 0.5).abs() < 1e-12,
        "wilson_loop after gauge_transform = {after}, expected 0.5, unchanged from {before}"
    );
    let flat_after = lattice.is_flat(&path, 1e-6);
    assert_eq!(
        flat_after,
        Some(false),
        "is_flat after gauge_transform = {flat_after:?}, expected Some(false)"
    );
}

// --- (f) cross-carrier agreement -------------------------------------------

#[test]
fn the_three_typed_carriers_agree_on_the_same_rotations() {
    let group = HypergraphRewriteGroup::new(1);
    let path: Vec<&[usize; 1]> = vec![&[0], &[1], &[2]];

    let mut rotation: HypergraphLattice<1, Rotation3<f64>> =
        HypergraphLattice::new([3], group, vec![], 3);
    assert!(
        rotation.record_transition(&[0], &[1], rz()),
        "A on [0]->[1]"
    );
    assert!(
        rotation.record_transition(&[1], &[2], rx()),
        "B on [1]->[2]"
    );
    assert!(
        rotation.record_transition(&[2], &[0], ry()),
        "C on [2]->[0]"
    );
    let rotation_wilson = rotation
        .wilson_loop(&path)
        .expect("invariant: all three links of the cycle were just recorded");
    assert!(
        (rotation_wilson - 1.0 / 3.0).abs() < 1e-12,
        "Rotation3 wilson_loop([0, 1, 2]) = {rotation_wilson}, expected {} = trace 1 / 3",
        1.0 / 3.0
    );
    let rotation_flat = rotation.is_flat(&path, 1e-6);
    assert_eq!(
        rotation_flat,
        Some(false),
        "Rotation3 is_flat([0, 1, 2], 1e-6) = {rotation_flat:?}, expected Some(false)"
    );

    let mut quaternion: HypergraphLattice<1, UnitQuaternion<f64>> =
        HypergraphLattice::new([3], group, vec![], 3);
    assert!(
        quaternion.record_transition(
            &[0],
            &[1],
            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), FRAC_PI_2)
        ),
        "A on [0]->[1]"
    );
    assert!(
        quaternion.record_transition(
            &[1],
            &[2],
            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), FRAC_PI_2)
        ),
        "B on [1]->[2]"
    );
    assert!(
        quaternion.record_transition(
            &[2],
            &[0],
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), FRAC_PI_2)
        ),
        "C on [2]->[0]"
    );
    let quaternion_wilson = quaternion
        .wilson_loop(&path)
        .expect("invariant: all three links of the cycle were just recorded");
    assert!(
        (quaternion_wilson - 1.0 / 3.0).abs() < 1e-12,
        "UnitQuaternion wilson_loop([0, 1, 2]) = {quaternion_wilson}, expected {} = trace 1 / 3",
        1.0 / 3.0
    );
    let quaternion_flat = quaternion.is_flat(&path, 1e-6);
    assert_eq!(
        quaternion_flat,
        Some(false),
        "UnitQuaternion is_flat([0, 1, 2], 1e-6) = {quaternion_flat:?}, expected Some(false)"
    );
    let quaternion_round_trip = LinkVariable::inverse(&qz())
        .expect("invariant: every unit quaternion has a conjugate")
        .compose(&qz());
    assert_eq!(
        quaternion_round_trip,
        UnitQuaternion::identity(),
        "UnitQuaternion inverse(qz)·qz = {quaternion_round_trip:?}, \
         expected the identity quaternion"
    );

    let mut isometry: HypergraphLattice<1, Isometry3<f64>> =
        HypergraphLattice::new([3], group, vec![], 4);
    assert!(
        isometry.record_transition(&[0], &[1], iso_z(FRAC_PI_2)),
        "A on [0]->[1]"
    );
    assert!(
        isometry.record_transition(
            &[1],
            &[2],
            Isometry3::new(Vector3::zeros(), Vector3::x() * FRAC_PI_2)
        ),
        "B on [1]->[2]"
    );
    assert!(
        isometry.record_transition(
            &[2],
            &[0],
            Isometry3::new(Vector3::zeros(), Vector3::y() * FRAC_PI_2)
        ),
        "C on [2]->[0]"
    );
    let isometry_wilson = isometry
        .wilson_loop(&path)
        .expect("invariant: all three links of the cycle were just recorded");
    assert!(
        (isometry_wilson - 0.5).abs() < 1e-12,
        "Isometry3 wilson_loop([0, 1, 2]) = {isometry_wilson}, expected 0.5 = (trace 1 + 1) / 4"
    );
    let isometry_flat = isometry.is_flat(&path, 1e-6);
    assert_eq!(
        isometry_flat,
        Some(false),
        "Isometry3 is_flat([0, 1, 2], 1e-6) = {isometry_flat:?}, expected Some(false)"
    );

    // The four-quarter-turn closure is flat on every carrier.
    let rotation_closure = closure_is_flat(&rz(), 3);
    assert_eq!(
        rotation_closure,
        Some(true),
        "Rotation3 closure flatness = {rotation_closure:?}, expected Some(true)"
    );
    let quaternion_closure = closure_is_flat(&qz(), 3);
    assert_eq!(
        quaternion_closure,
        Some(true),
        "UnitQuaternion closure flatness = {quaternion_closure:?}, expected Some(true)"
    );
    let isometry_closure = closure_is_flat(&iso_z(FRAC_PI_2), 4);
    assert_eq!(
        isometry_closure,
        Some(true),
        "Isometry3 closure flatness = {isometry_closure:?}, expected Some(true)"
    );
}

// --- (g) typed admissibility ----------------------------------------------

#[test]
fn typed_links_with_non_finite_entries_are_rejected() {
    let group = HypergraphRewriteGroup::new(1);

    // The trait method rejects a dimension other than the carrier's own, and
    // rejects a non-finite rotation coordinate at the carrier's own.
    let rotation_at_4 = rz().is_admissible(4);
    assert!(
        !rotation_at_4,
        "Rotation3::is_admissible(4) = {rotation_at_4}, expected false — its \
         defining representation is 3 x 3"
    );
    let quaternion_at_4 = qz().is_admissible(4);
    assert!(
        !quaternion_at_4,
        "UnitQuaternion::is_admissible(4) = {quaternion_at_4}, expected false — its \
         defining representation is 3 x 3"
    );
    let isometry_at_3 = Isometry3::<f64>::identity().is_admissible(3);
    assert!(
        !isometry_at_3,
        "Isometry3::is_admissible(3) = {isometry_at_3}, expected false — its \
         defining representation is 4 x 4"
    );
    let nan_axis = Isometry3::new(Vector3::zeros(), Vector3::new(f64::NAN, 0.0, 0.0));
    let nan_axis_at_4 = nan_axis.is_admissible(4);
    assert!(
        !nan_axis_at_4,
        "Isometry3::is_admissible(4) on a NaN rotation coordinate = {nan_axis_at_4}, \
         expected false"
    );

    let mut rotation: HypergraphLattice<1, Rotation3<f64>> =
        HypergraphLattice::new([2], group, vec![], 3);
    let nan_rotation = Rotation3::from_matrix_unchecked(Matrix3::new(
        f64::NAN,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ));
    let rotation_rejected = rotation.record_transition(&[0], &[1], nan_rotation);
    assert!(
        !rotation_rejected,
        "record_transition of a NaN Rotation3 = {rotation_rejected}, expected false"
    );
    assert_eq!(
        rotation.link(&[0], &[1]),
        None,
        "link([0] -> [1]) after the rejected NaN Rotation3 = {:?}, expected None",
        rotation.link(&[0], &[1])
    );
    let rotation_accepted = rotation.record_transition(&[0], &[1], rz());
    assert!(
        rotation_accepted,
        "record_transition of Rz = {rotation_accepted}, expected true"
    );
    assert_eq!(
        rotation.link(&[0], &[1]),
        Some(&rz()),
        "link([0] -> [1]) after recording Rz = {:?}, expected Some(Rz)",
        rotation.link(&[0], &[1])
    );

    let mut quaternion: HypergraphLattice<1, UnitQuaternion<f64>> =
        HypergraphLattice::new([2], group, vec![], 3);
    let nan_quaternion = UnitQuaternion::new_unchecked(Quaternion::new(f64::NAN, 0.0, 0.0, 0.0));
    let quaternion_rejected = quaternion.record_transition(&[0], &[1], nan_quaternion);
    assert!(
        !quaternion_rejected,
        "record_transition of a NaN UnitQuaternion = {quaternion_rejected}, expected false"
    );
    assert_eq!(
        quaternion.link(&[0], &[1]),
        None,
        "link([0] -> [1]) after the rejected NaN UnitQuaternion = {:?}, expected None",
        quaternion.link(&[0], &[1])
    );
    let quaternion_accepted = quaternion.record_transition(&[0], &[1], qz());
    assert!(
        quaternion_accepted,
        "record_transition of the z quarter-turn quaternion = {quaternion_accepted}, \
         expected true"
    );
    assert_eq!(
        quaternion.link(&[0], &[1]),
        Some(&qz()),
        "link([0] -> [1]) after recording the quaternion = {:?}, expected Some(qz)",
        quaternion.link(&[0], &[1])
    );

    let mut isometry: HypergraphLattice<1, Isometry3<f64>> =
        HypergraphLattice::new([2], group, vec![], 4);
    let nan_isometry = Isometry3::translation(f64::NAN, 0.0, 0.0);
    let isometry_rejected = isometry.record_transition(&[0], &[1], nan_isometry);
    assert!(
        !isometry_rejected,
        "record_transition of a NaN-translation Isometry3 = {isometry_rejected}, expected false"
    );
    assert_eq!(
        isometry.link(&[0], &[1]),
        None,
        "link([0] -> [1]) after the rejected NaN Isometry3 = {:?}, expected None",
        isometry.link(&[0], &[1])
    );
    let good_isometry = Isometry3::translation(1.0, 2.0, 3.0);
    let isometry_accepted = isometry.record_transition(&[0], &[1], good_isometry);
    assert!(
        isometry_accepted,
        "record_transition of T(1,2,3) = {isometry_accepted}, expected true"
    );
    assert_eq!(
        isometry.link(&[0], &[1]),
        Some(&good_isometry),
        "link([0] -> [1]) after recording T(1,2,3) = {:?}, expected Some(T(1,2,3))",
        isometry.link(&[0], &[1])
    );
}

// --- (h) SO(3) plaquettes over a 2 x 2 lattice -----------------------------

#[test]
fn so3_plaquette_verdicts_over_a_2x2_lattice() {
    let group = HypergraphRewriteGroup::new(1);
    let mut lattice: HypergraphLattice<2, Rotation3<f64>> =
        HypergraphLattice::new([2, 2], group, vec![], 3);
    let hops: [([usize; 2], [usize; 2]); 4] = [
        ([0, 0], [1, 0]),
        ([1, 0], [1, 1]),
        ([1, 1], [0, 1]),
        ([0, 1], [0, 0]),
    ];
    for (from, to) in hops {
        assert!(
            lattice.record_transition(&from, &to, Rotation3::identity()),
            "record_transition({from:?} -> {to:?}, I3) = false, expected true"
        );
    }

    lattice.find_wilson_loops(4);
    assert_eq!(
        lattice.recorded_loops().len(),
        1,
        "recorded_loops().len() = {}, expected 1 — a 2 x 2 lattice has one plaquette",
        lattice.recorded_loops().len()
    );
    let invariant = lattice.is_globally_causally_invariant();
    assert_eq!(
        invariant,
        Some(true),
        "is_globally_causally_invariant with identity links = {invariant:?}, expected Some(true)"
    );
    let average = lattice.average_holonomy();
    assert_eq!(
        average,
        Some(1.0),
        "average_holonomy with identity links = {average:?}, \
         expected Some(1.0) = trace(I3) 3 / 3"
    );
    let action = lattice.total_plaquette_action();
    assert_eq!(
        action, 0.0,
        "total_plaquette_action with identity links = {action}, expected 0.0"
    );

    assert!(
        lattice.record_transition(&[0, 0], &[1, 0], rz()),
        "record_transition([0,0] -> [1,0], Rz) = false, expected true"
    );
    lattice.find_wilson_loops(4);
    let curved_invariant = lattice.is_globally_causally_invariant();
    assert_eq!(
        curved_invariant,
        Some(false),
        "is_globally_causally_invariant with one Rz link = {curved_invariant:?}, \
         expected Some(false)"
    );
    let curved_average = lattice
        .average_holonomy()
        .expect("invariant: one plaquette is recorded");
    assert!(
        (curved_average - 1.0 / 3.0).abs() < 1e-12,
        "average_holonomy with one Rz link = {curved_average}, expected {} = trace(Rz) 1 / 3",
        1.0 / 3.0
    );
    let curved_action = lattice.total_plaquette_action();
    assert!(
        (curved_action - 3.0_f64.ln()).abs() < 1e-12,
        "total_plaquette_action with one Rz link = {curved_action}, expected {} = -ln(1/3)",
        3.0_f64.ln()
    );
}

// --- carrier-invariant admissibility ---------------------------------------

/// The `[2]` line lattice whose two links both carry the `Rotation3` identity.
fn flat_rotation_line() -> HypergraphLattice<1, Rotation3<f64>> {
    let mut lattice: HypergraphLattice<1, Rotation3<f64>> =
        HypergraphLattice::new([2], HypergraphRewriteGroup::new(1), vec![], 3);
    let hops: [([usize; 1], [usize; 1]); 2] = [([0], [1]), ([1], [0])];
    for (from, to) in hops {
        assert!(
            lattice.record_transition(&from, &to, Rotation3::identity()),
            "record_transition({from:?} -> {to:?}, I3) = false, expected true"
        );
    }
    lattice
}

#[test]
fn typed_links_off_their_carrier_invariant_are_rejected() {
    let group = HypergraphRewriteGroup::new(1);

    // A quaternion of norm 2 is not a rotation.
    let mut quaternion: HypergraphLattice<1, UnitQuaternion<f64>> =
        HypergraphLattice::new([2], group, vec![], 3);
    let scaled = UnitQuaternion::new_unchecked(Quaternion::new(2.0, 0.0, 0.0, 0.0));
    let scaled_recorded = quaternion.record_transition(&[0], &[1], scaled);
    assert!(
        !scaled_recorded,
        "record_transition of a norm-2 UnitQuaternion = {scaled_recorded}, expected false — \
         its norm is 2, off 1 by 1, past TYPED_LINK_TOL"
    );
    let scaled_link = quaternion.link(&[0], &[1]);
    assert_eq!(
        scaled_link, None,
        "link([0] -> [1]) after the rejected norm-2 quaternion = {scaled_link:?}, expected None"
    );

    // A reflection is orthogonal but has determinant −1.
    let mut rotation: HypergraphLattice<1, Rotation3<f64>> =
        HypergraphLattice::new([2], group, vec![], 3);
    let reflection = Rotation3::from_matrix_unchecked(Matrix3::new(
        -1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0,
    ));
    let reflection_recorded = rotation.record_transition(&[0], &[1], reflection);
    assert!(
        !reflection_recorded,
        "record_transition of diag(-1, 1, 1) = {reflection_recorded}, expected false — \
         its determinant is -1, not positive"
    );

    // A non-orthogonal matrix is not a rotation either.
    let stretched =
        Rotation3::from_matrix_unchecked(Matrix3::new(2.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0));
    let stretched_recorded = rotation.record_transition(&[0], &[1], stretched);
    assert!(
        !stretched_recorded,
        "record_transition of diag(2, 1, 1) = {stretched_recorded}, expected false — \
         entry (0, 0) of R·Rᵀ - I is 3, past TYPED_LINK_TOL"
    );

    // The checked constructors stay accepted.
    let rz_recorded = rotation.record_transition(&[0], &[1], rz());
    assert!(
        rz_recorded,
        "record_transition of Rz = {rz_recorded}, expected true"
    );
    let qz_recorded = quaternion.record_transition(&[0], &[1], qz());
    assert!(
        qz_recorded,
        "record_transition of the z quarter-turn quaternion = {qz_recorded}, expected true"
    );

    // The tolerance boundary: diag(1 + e, 1, 1) has R·Rᵀ - I entry (0, 0) of
    // 2e + e², and the quaternion (1 + e, 0, 0, 0) has norm 1 + e.
    let near = Rotation3::from_matrix_unchecked(Matrix3::new(
        1.0 + 1e-10,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ));
    let near_admissible = LinkVariable::is_admissible(&near, 3);
    assert!(
        near_admissible,
        "is_admissible(diag(1 + 1e-10, 1, 1)) = {near_admissible}, expected true — \
         entry (0, 0) of R·Rᵀ - I is 2e-10, under TYPED_LINK_TOL 1e-9"
    );
    let far = Rotation3::from_matrix_unchecked(Matrix3::new(
        1.0 + 1e-9,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ));
    let far_admissible = LinkVariable::is_admissible(&far, 3);
    assert!(
        !far_admissible,
        "is_admissible(diag(1 + 1e-9, 1, 1)) = {far_admissible}, expected false — \
         entry (0, 0) of R·Rᵀ - I is 2e-9, past TYPED_LINK_TOL 1e-9"
    );
    let near_q = UnitQuaternion::new_unchecked(Quaternion::new(1.0 + 1e-10, 0.0, 0.0, 0.0));
    let near_q_admissible = LinkVariable::is_admissible(&near_q, 3);
    assert!(
        near_q_admissible,
        "is_admissible of the quaternion of norm 1 + 1e-10 = {near_q_admissible}, expected \
         true — off 1 by 1e-10, under TYPED_LINK_TOL 1e-9"
    );
    let far_q = UnitQuaternion::new_unchecked(Quaternion::new(1.0 + 1e-9, 0.0, 0.0, 0.0));
    let far_q_admissible = LinkVariable::is_admissible(&far_q, 3);
    assert!(
        !far_q_admissible,
        "is_admissible of the quaternion of norm 1 + 1e-9 = {far_q_admissible}, expected \
         false — off 1 by 1e-9, not under TYPED_LINK_TOL 1e-9"
    );
}

#[test]
fn isometry3_links_over_a_non_unit_quaternion_are_rejected() {
    let mut isometry: HypergraphLattice<1, Isometry3<f64>> =
        HypergraphLattice::new([2], HypergraphRewriteGroup::new(1), vec![], 4);

    // An isometry inherits its rotation's verdict.
    let scaled_isometry = Isometry3::from_parts(
        Translation3::new(1.0, 2.0, 3.0),
        UnitQuaternion::new_unchecked(Quaternion::new(2.0, 0.0, 0.0, 0.0)),
    );
    let scaled_isometry_recorded = isometry.record_transition(&[0], &[1], scaled_isometry);
    assert!(
        !scaled_isometry_recorded,
        "record_transition of an Isometry3 over a norm-2 quaternion = \
         {scaled_isometry_recorded}, expected false"
    );
    let scaled_link = isometry.link(&[0], &[1]);
    assert_eq!(
        scaled_link, None,
        "link([0] -> [1]) after the rejected Isometry3 = {scaled_link:?}, expected None"
    );

    // The checked constructors stay accepted.
    let iso_recorded = isometry.record_transition(&[0], &[1], iso_z(FRAC_PI_2));
    assert!(
        iso_recorded,
        "record_transition of the SE(3) z quarter-turn = {iso_recorded}, expected true"
    );
    let translation_recorded =
        isometry.record_transition(&[1], &[0], Isometry3::translation(1.0, 2.0, 3.0));
    assert!(
        translation_recorded,
        "record_transition of T(1,2,3) = {translation_recorded}, expected true"
    );
}

#[test]
fn gauge_transform_rejects_a_non_orthogonal_rotation_value() {
    let mut lattice = flat_rotation_line();
    let path: Vec<&[usize; 1]> = vec![&[0], &[1]];

    let before = lattice.wilson_loop(&path);
    assert_eq!(
        before,
        Some(1.0),
        "wilson_loop before gauge_transform = {before:?}, expected Some(1.0) = trace(I3) 3 / 3"
    );

    let mut g: HashMap<Vec<usize>, Rotation3<f64>> = HashMap::new();
    g.insert(
        vec![0],
        Rotation3::from_matrix_unchecked(Matrix3::new(2.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0)),
    );
    let accepted = lattice.gauge_transform(&g);
    assert!(
        !accepted,
        "gauge_transform with g_0 = diag(2, 1, 1) = {accepted}, expected false — \
         diag(2, 1, 1) is not orthogonal"
    );

    let link = lattice.link(&[0], &[1]);
    assert_eq!(
        link,
        Some(&Rotation3::identity()),
        "link([0] -> [1]) after the rejected gauge_transform = {link:?}, expected Some(I3)"
    );
    let after = lattice.wilson_loop(&path);
    assert_eq!(
        after,
        Some(1.0),
        "wilson_loop after the rejected gauge_transform = {after:?}, expected Some(1.0)"
    );
}

// --- gauge invariance on the SO(3) carriers --------------------------------

#[test]
fn so3_gauge_transform_conjugates_the_holonomy_and_fixes_its_wilson_value() {
    let group = HypergraphRewriteGroup::new(1);
    let mut lattice: HypergraphLattice<1, Rotation3<f64>> =
        HypergraphLattice::new([3], group, vec![], 3);
    assert!(lattice.record_transition(&[0], &[1], rz()), "A on [0]->[1]");
    assert!(lattice.record_transition(&[1], &[2], rx()), "B on [1]->[2]");
    assert!(lattice.record_transition(&[2], &[0], ry()), "C on [2]->[0]");

    let path: Vec<&[usize; 1]> = vec![&[0], &[1], &[2]];
    let before = lattice
        .wilson_loop(&path)
        .expect("invariant: all three links of the cycle were just recorded");
    assert!(
        (before - 1.0 / 3.0).abs() < 1e-12,
        "wilson_loop before gauge_transform = {before}, expected {} = trace(Rx) 1 / 3",
        1.0 / 3.0
    );

    let mut g: HashMap<Vec<usize>, Rotation3<f64>> = HashMap::new();
    g.insert(vec![0], rz());
    let accepted = lattice.gauge_transform(&g);
    assert!(
        accepted,
        "gauge_transform with g_0 = Rz = {accepted}, expected true"
    );

    let holonomy = lattice
        .loop_holonomy(&path)
        .expect("invariant: gauge_transform keeps the link key set");
    let moved = diff3(holonomy.matrix(), &rx_matrix());
    assert!(
        moved > 1e-6,
        "max entry difference between the conjugated holonomy {:?} and the \
         pre-transform Rx = {moved}, expected > 1e-6 (hand value 1, at entry \
         (0, 0): 0 against 1)",
        holonomy.matrix()
    );
    let conjugate_diff = diff3(holonomy.matrix(), &ry_matrix());
    assert!(
        conjugate_diff < 1e-12,
        "loop_holonomy([0, 1, 2]) after gauge_transform = {:?}, expected \
         Rz·Rx·Rz⁻¹ = Ry = [[0,0,1],[0,1,0],[-1,0,0]]; max entry difference \
         {conjugate_diff}, expected < 1e-12",
        holonomy.matrix()
    );
    let after = lattice
        .wilson_loop(&path)
        .expect("invariant: gauge_transform keeps the link key set");
    assert!(
        (after - 1.0 / 3.0).abs() < 1e-12,
        "wilson_loop after gauge_transform = {after}, expected {} = trace(Ry) 1 / 3, \
         unchanged from {before}",
        1.0 / 3.0
    );
}

#[test]
fn quaternion_gauge_transform_conjugates_the_holonomy_and_fixes_its_wilson_value() {
    let group = HypergraphRewriteGroup::new(1);
    let mut lattice: HypergraphLattice<1, UnitQuaternion<f64>> =
        HypergraphLattice::new([3], group, vec![], 3);
    assert!(
        lattice.record_transition(
            &[0],
            &[1],
            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), FRAC_PI_2)
        ),
        "A on [0]->[1]"
    );
    assert!(
        lattice.record_transition(
            &[1],
            &[2],
            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), FRAC_PI_2)
        ),
        "B on [1]->[2]"
    );
    assert!(
        lattice.record_transition(
            &[2],
            &[0],
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), FRAC_PI_2)
        ),
        "C on [2]->[0]"
    );

    let path: Vec<&[usize; 1]> = vec![&[0], &[1], &[2]];
    let before = lattice
        .wilson_loop(&path)
        .expect("invariant: all three links of the cycle were just recorded");
    assert!(
        (before - 1.0 / 3.0).abs() < 1e-12,
        "wilson_loop before gauge_transform = {before}, expected {} = trace 1 / 3",
        1.0 / 3.0
    );

    let mut g: HashMap<Vec<usize>, UnitQuaternion<f64>> = HashMap::new();
    g.insert(vec![0], qz());
    let accepted = lattice.gauge_transform(&g);
    assert!(
        accepted,
        "gauge_transform with g_0 = qz = {accepted}, expected true"
    );

    let holonomy = lattice
        .loop_holonomy(&path)
        .expect("invariant: gauge_transform keeps the link key set");
    let rotation = holonomy.to_rotation_matrix();
    let moved = diff3(rotation.matrix(), &rx_matrix());
    assert!(
        moved > 1e-6,
        "max entry difference between the conjugated rotation {:?} and the \
         pre-transform Rx = {moved}, expected > 1e-6 (hand value 1, at entry \
         (0, 0): 0 against 1)",
        rotation.matrix()
    );
    let conjugate_diff = diff3(rotation.matrix(), &ry_matrix());
    assert!(
        conjugate_diff < 1e-12,
        "the rotation of loop_holonomy([0, 1, 2]) after gauge_transform = {:?}, expected \
         Rz·Rx·Rz⁻¹ = Ry = [[0,0,1],[0,1,0],[-1,0,0]]; max entry difference \
         {conjugate_diff}, expected < 1e-12",
        rotation.matrix()
    );
    let after = lattice
        .wilson_loop(&path)
        .expect("invariant: gauge_transform keeps the link key set");
    assert!(
        (after - 1.0 / 3.0).abs() < 1e-12,
        "wilson_loop after gauge_transform = {after}, expected {} = trace(Ry) 1 / 3, \
         unchanged from {before}",
        1.0 / 3.0
    );
}
