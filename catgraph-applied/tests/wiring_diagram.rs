//! Integration tests for `WiringDiagram` using only the public API.
//!
//! `WiringDiagram` is `#[repr(transparent)]` over `NamedCospan`, but the inner
//! field is private.  From here we can only observe behavior through:
//!   - `operadic_substitution` (success / failure / side-effects on self)
//!   - `map` (returns a new WD we can substitute into)
//!   - mutation methods (add, delete, connect, rename, toggle)
//!
//! Every test therefore asserts **behavioral** properties, not internal state.

mod common;

use catgraph::{
    assert_err, assert_ok, category::Composable, monoidal::Monoidal, named_cospan::NamedCospan,
    operadic::Operadic,
};
use catgraph_applied::wiring_diagram::{Dir, WiringDiagram};
use either::Either::{Left, Right};

// ---------------------------------------------------------------------------
// Type aliases matching the unit-test conventions
// ---------------------------------------------------------------------------

type CircleName = i32;
type WireName = usize;

// ---------------------------------------------------------------------------
// Builders
// ---------------------------------------------------------------------------

/// A no-input WD: zero inner circles, three outer boundary nodes.
///
/// Middle nodes: 0 = true, 1 = true, 2 = false.
/// Right (outer) boundary: (In,0)→mid0, (Out,1)→mid1, (In,2)→mid2.
fn leaf_diagram() -> WiringDiagram<bool, CircleName, WireName> {
    WiringDiagram::new(NamedCospan::new(
        vec![],
        vec![0, 1, 2],
        vec![true, true, false],
        vec![],
        vec![(Dir::In, 0), (Dir::Out, 1), (Dir::In, 2)],
    ))
}

/// An outer WD with one inner circle (circle 0) whose interface matches
/// `leaf_diagram`'s outer boundary.
///
/// Left (inner) ports mirror the leaf's right ports with flipped orientation
/// (because substitution requires the inner circle of the outer to match the
/// external circle of the inner, with orientation reversal).
///
/// Right (outer) boundary: single node (Out,0) of type true.
/// All inner ports of type true map to the same middle node, and the type-false
/// port maps to a second middle node — matching the unit test pattern.
fn one_hole_outer() -> WiringDiagram<bool, CircleName, WireName> {
    WiringDiagram::new(NamedCospan::new(
        // left → middle  (inner circle 0 ports)
        vec![0, 0, 1],
        // right → middle (outer boundary)
        vec![0],
        // middle types
        vec![true, false],
        // left names: orientation-flipped mirror of leaf_diagram's right names
        vec![
            (Dir::Out, 0, 0), // mirrors (In, 0)
            (Dir::In, 0, 1),  // mirrors (Out, 1)
            (Dir::Out, 0, 2), // mirrors (In, 2)
        ],
        // right (outer) names
        vec![(Dir::Out, 0)],
    ))
}

/// An outer WD with two inner circles (circle 1 and circle 2), untyped labels.
///
/// Circle 1 has 2 ports (matching `unit_leaf_2`).
/// Circle 2 has 1 port.
/// Right (outer) has 2 ports.
///
/// IMPORTANT: wire names (`IntraCircle`) must be globally unique across all inner
/// circles because `operadic_substitution` drops the `InterCircle` when building
/// the permutation target — see line 213 of `wiring_diagram.rs`.
fn two_hole_outer() -> WiringDiagram<(), CircleName, WireName> {
    WiringDiagram::new(NamedCospan::new(
        // left → middle
        vec![0, 1, 2],
        // right → middle
        vec![0, 2],
        // middle
        vec![(), (), ()],
        // left names: flipped mirror of inner circles' outer boundaries
        // Wire names 10,11 for circle 1; wire name 20 for circle 2 — globally unique.
        vec![
            (Dir::Out, 1, 10), // circle 1, mirrors (In, 10) on leaf
            (Dir::In, 1, 11),  // circle 1, mirrors (Out, 11) on leaf
            (Dir::Out, 2, 20), // circle 2, mirrors (In, 20) on leaf
        ],
        // right (outer) names
        vec![(Dir::Out, 50), (Dir::In, 51)],
    ))
}

/// A leaf WD with 2 outer ports matching circle 1 of `two_hole_outer`.
///
/// Right names must be the orientation-flipped mirror of the outer's circle 1 left names.
fn leaf_for_circle_1() -> WiringDiagram<(), CircleName, WireName> {
    WiringDiagram::new(NamedCospan::new(
        vec![],
        vec![0, 1],
        vec![(), ()],
        vec![],
        // Flipped from outer's (Dir::Out,1,10) and (Dir::In,1,11)
        vec![(Dir::In, 10), (Dir::Out, 11)],
    ))
}

/// A leaf WD with 1 outer port matching circle 2 of `two_hole_outer`.
fn leaf_for_circle_2() -> WiringDiagram<(), CircleName, WireName> {
    WiringDiagram::new(NamedCospan::new(
        vec![],
        vec![0],
        vec![()],
        vec![],
        // Flipped from outer's (Dir::Out,2,20)
        vec![(Dir::In, 20)],
    ))
}

// ===========================================================================
// 1. Construct a WD and use it in operadic substitution
// ===========================================================================

#[test]
fn construct_leaf_and_substitute_into_outer() {
    let mut outer = one_hole_outer();
    let inner = leaf_diagram();
    let result = outer.operadic_substitution(0, inner);
    assert_ok!(result);
}

// ===========================================================================
// 2. Basic operadic substitution between compatible WDs
// ===========================================================================

#[test]
fn basic_operadic_substitution_succeeds() {
    let mut outer = one_hole_outer();
    let inner = leaf_diagram();
    let result = outer.operadic_substitution(0, inner);
    assert!(result.is_ok(), "compatible substitution must succeed");
}

// ===========================================================================
// 3. Type mismatch fails operadic substitution
// ===========================================================================

#[test]
fn type_mismatch_fails_substitution() {
    // Build an outer expecting circle 0 with ports of type true/true/false,
    // but provide an inner whose outer boundary has different types.
    let mismatched_inner: WiringDiagram<bool, CircleName, WireName> =
        WiringDiagram::new(NamedCospan::new(
            vec![],
            vec![0, 1, 2],
            // All false — does not match the outer's expectation (true, true, false)
            vec![false, false, false],
            vec![],
            vec![(Dir::In, 0), (Dir::Out, 1), (Dir::In, 2)],
        ));
    let mut outer = one_hole_outer();
    let result = outer.operadic_substitution(0, mismatched_inner);
    assert_err!(result);
}

// ===========================================================================
// 4. Wrong circle name fails substitution
// ===========================================================================

#[test]
fn wrong_circle_name_fails_substitution() {
    let mut outer = one_hole_outer();
    let inner = leaf_diagram();
    // Outer has circle 0 only — substituting into circle 99 should fail
    // because no ports are found, leading to a permutation error.
    let result = outer.operadic_substitution(99, inner);
    assert_err!(result);
}

// ===========================================================================
// 5. Sequential substitution into multiple circles
// ===========================================================================

#[test]
fn sequential_substitution_into_two_circles() {
    let mut outer = two_hole_outer();
    let leaf_a = leaf_for_circle_1();
    let leaf_b = leaf_for_circle_2();

    // Substitute circle 1 first
    let r1 = outer.operadic_substitution(1, leaf_a);
    assert_ok!(r1);

    // Substitute circle 2 second
    let r2 = outer.operadic_substitution(2, leaf_b);
    assert_ok!(r2);
}

// ===========================================================================
// 6. Add boundary node then substitute
// ===========================================================================

#[test]
fn add_boundary_node_then_substitute() {
    // Use the one_hole_outer + leaf_diagram pair.
    let mut outer = one_hole_outer();

    // Add an unconnected left-side (inner circle) boundary node on a
    // *different* circle (circle 5, which doesn't exist yet).
    // This should not affect substitution into circle 0.
    outer.add_boundary_node_unconnected(true, Left((Dir::In, 5, 100)));

    let inner = leaf_diagram();
    let result = outer.operadic_substitution(0, inner);
    assert_ok!(result);
}

// ===========================================================================
// 7. Connect pair then substitute
// ===========================================================================

#[test]
fn connect_pair_then_substitute() {
    let mut outer = two_hole_outer();

    // Connect two outer boundary nodes (they both have type ()).
    // Before: they may map to different middle nodes.
    // After connect_pair: they share a middle node.
    outer.connect_pair(Right((Dir::Out, 50)), Right((Dir::In, 51)));

    // Substitution into circle 1 should still succeed — connect_pair only
    // affects middle node topology, not the inner circle interface.
    let leaf = leaf_for_circle_1();
    let result = outer.operadic_substitution(1, leaf);
    assert_ok!(result);
}

// ===========================================================================
// 8. Delete boundary then substitute
// ===========================================================================

#[test]
fn delete_outer_boundary_then_substitute() {
    let mut outer = two_hole_outer();

    // Delete one of the right (outer) boundary nodes.
    outer.delete_boundary_node(Right((Dir::In, 51)));

    // The inner circle interface is unchanged, so substitution should still work.
    let leaf = leaf_for_circle_1();
    let result = outer.operadic_substitution(1, leaf);
    assert_ok!(result);
}

// ===========================================================================
// 9. Toggle orientation then substitute
// ===========================================================================

#[test]
fn toggle_left_orientation_then_substitute() {
    // The operadic substitution matches inner-circle ports by name including
    // orientation.  The inner leaf's outer boundary has orientations that are
    // the *flip* of the outer's inner-circle ports.  If we toggle the outer's
    // left (inner) side, the orientations no longer match and substitution
    // should fail.
    let mut outer = one_hole_outer();
    outer.toggle_orientation(true); // flip left-side orientations

    let inner = leaf_diagram();
    let result = outer.operadic_substitution(0, inner);
    // After toggling, the name-based permutation lookup should fail because
    // the flipped names no longer align with the inner's right names.
    assert_err!(result);
}

// ===========================================================================
// 10. Map preserves substitutability
// ===========================================================================

#[test]
fn map_preserves_substitutability() {
    // Map both outer and inner with the same function, then substitute.
    let outer_bool = one_hole_outer();
    let inner_bool = leaf_diagram();

    // Map bool -> u8: true -> 1, false -> 0
    let to_u8 = |b: bool| u8::from(b);
    let mut outer_u8 = outer_bool.map(to_u8);
    let inner_u8 = inner_bool.map(to_u8);

    let result = outer_u8.operadic_substitution(0, inner_u8);
    assert_ok!(result);
}

// ===========================================================================
// 11. Chain of mutations then substitute
// ===========================================================================

#[test]
fn chain_of_mutations_then_substitute() {
    let mut outer = two_hole_outer();

    // Add an unconnected right-side node
    outer.add_boundary_node_unconnected((), Right((Dir::Out, 77)));

    // Connect that new node with an existing outer boundary node
    outer.connect_pair(Right((Dir::Out, 50)), Right((Dir::Out, 77)));

    // Rename an inner-circle port (circle 2, wire 20) to a different wire name
    // but keep the same circle — this should not affect circle 1 substitution.
    outer.change_boundary_node_name(Left(((Dir::Out, 2, 20), (Dir::Out, 2, 25))));

    // Substitute circle 1 — should succeed despite all mutations.
    let leaf = leaf_for_circle_1();
    let result = outer.operadic_substitution(1, leaf);
    assert_ok!(result);
}

// ===========================================================================
// 12. Substitution preserves unaffected circles
// ===========================================================================

#[test]
fn substitution_preserves_unaffected_circles() {
    let mut outer = two_hole_outer();

    // Substitute only circle 1.
    let leaf = leaf_for_circle_1();
    let r1 = outer.operadic_substitution(1, leaf);
    assert_ok!(r1);

    // Circle 2 should still be substitutable — it was not touched.
    let leaf2 = leaf_for_circle_2();
    let r2 = outer.operadic_substitution(2, leaf2);
    assert_ok!(r2);

    // Circle 1 is now gone — substituting into it again should fail.
    let another_leaf = leaf_for_circle_1();
    let r3 = outer.operadic_substitution(1, another_leaf);
    assert_err!(r3);
}

// ===========================================================================
// 13. Empty / identity substitution: leaf into leaf
// ===========================================================================

#[test]
fn substitution_into_empty_circle_set() {
    // A leaf diagram has no inner circles.  Substituting into any circle name
    // should fail because there are no matching ports.
    let mut leaf = leaf_diagram();
    let another_leaf = leaf_diagram();
    let result = leaf.operadic_substitution(0, another_leaf);
    assert_err!(result);
}

// ===========================================================================
// 14. Delete inner-circle port then substitution fails
// ===========================================================================

#[test]
fn delete_inner_port_breaks_substitution() {
    let mut outer = one_hole_outer();

    // Delete one of the left (inner circle 0) ports.
    outer.delete_boundary_node(Left((Dir::Out, 0, 0)));

    // Now the inner circle's interface has 2 ports instead of 3,
    // but the leaf still has 3 outer ports.  The permutation lookup
    // should fail because the names don't match up.
    let inner = leaf_diagram();
    let result = outer.operadic_substitution(0, inner);
    assert_err!(result);
}

// ===========================================================================
// 15. Sequential composition via Composable trait
// ===========================================================================

/// Build two leaf WDs whose codomain/domain types match and compose them.
#[test]
fn sequential_compose_basic() {
    // WD_A: no inner circles, domain = [], codomain = [(), ()]
    let wd_a: WiringDiagram<(), CircleName, WireName> = WiringDiagram::new(NamedCospan::new(
        vec![],
        vec![0, 1],
        vec![(), ()],
        vec![],
        vec![(Dir::Out, 10), (Dir::In, 11)],
    ));

    // WD_B: domain = [(), ()], codomain = [()]
    let wd_b: WiringDiagram<(), CircleName, WireName> = WiringDiagram::new(NamedCospan::new(
        vec![0, 0],
        vec![0],
        vec![()],
        vec![(Dir::In, 1, 10), (Dir::Out, 1, 11)],
        vec![(Dir::Out, 50)],
    ));

    assert_eq!(wd_a.codomain(), vec![(), ()]);
    assert_eq!(wd_b.domain(), vec![(), ()]);

    let composed = wd_a.compose(&wd_b);
    assert!(composed.is_ok(), "compose of compatible WDs must succeed");
    let composed = composed.unwrap();
    assert_eq!(composed.domain(), Vec::<()>::new());
    assert_eq!(composed.codomain(), vec![()]);
}

// ===========================================================================
// 16. Identity composition is neutral
// ===========================================================================

/// Composing with an identity named cospan preserves domain and codomain.
#[test]
fn compose_identity_is_neutral() {
    let wd: WiringDiagram<(), CircleName, WireName> = WiringDiagram::new(NamedCospan::new(
        vec![0],
        vec![0, 1],
        vec![(), ()],
        vec![(Dir::In, 0, 10)],
        vec![(Dir::Out, 20), (Dir::In, 21)],
    ));

    // Build an identity WD whose domain/codomain = wd.codomain() = [(), ()].
    // Port names must differ from wd's right names (names are independent).
    let id_wd: WiringDiagram<(), CircleName, WireName> = WiringDiagram::new(NamedCospan::identity(
        &[(), ()],
        &[(Dir::Out, 20), (Dir::In, 21)],
        |n| ((n.0, 99 as CircleName, n.1), n),
    ));

    assert_eq!(id_wd.domain(), vec![(), ()]);
    assert_eq!(id_wd.codomain(), vec![(), ()]);

    let result = wd.compose(&id_wd);
    assert!(result.is_ok(), "compose with identity must succeed");
    let composed = result.unwrap();
    // Domain comes from wd, codomain comes from id_wd (which equals wd's codomain).
    assert_eq!(composed.domain(), wd.domain());
    assert_eq!(composed.codomain(), wd.codomain());
}

// ===========================================================================
// 17. Monoidal tensor of two diagrams
// ===========================================================================

/// Parallel composition concatenates domain and codomain type lists.
#[test]
fn monoidal_tensor_of_two_diagrams() {
    let wd_a: WiringDiagram<(), CircleName, WireName> = WiringDiagram::new(NamedCospan::new(
        vec![0],
        vec![0],
        vec![()],
        vec![(Dir::In, 0, 10)],
        vec![(Dir::Out, 50)],
    ));

    let wd_b: WiringDiagram<(), CircleName, WireName> = WiringDiagram::new(NamedCospan::new(
        vec![0, 1],
        vec![0],
        vec![(), ()],
        vec![(Dir::In, 1, 20), (Dir::Out, 1, 21)],
        vec![(Dir::In, 51)],
    ));

    let dom_a = wd_a.domain();
    let dom_b = wd_b.domain();
    let cod_a = wd_a.codomain();
    let cod_b = wd_b.codomain();

    let mut combined = wd_a;
    combined.monoidal(wd_b);

    let expected_domain: Vec<()> = dom_a.into_iter().chain(dom_b).collect();
    let expected_codomain: Vec<()> = cod_a.into_iter().chain(cod_b).collect();
    assert_eq!(combined.domain(), expected_domain);
    assert_eq!(combined.codomain(), expected_codomain);
}
