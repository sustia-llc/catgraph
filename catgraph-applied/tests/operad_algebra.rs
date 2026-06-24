//! Tests for `catgraph_applied::operad_algebra` — F&S *Seven Sketches*
//! §6.5 Def 6.99 (an algebra for an operad `O` is a functor `F : O → Set`).

use catgraph::category::HasIdentity;
use catgraph::errors::CatgraphError;
use catgraph::named_cospan::NamedCospan;
use catgraph_applied::e1_operad::E1;
use catgraph_applied::operad_algebra::{CircAlgebra, OperadAlgebra, check_substitution_preserved};
use catgraph_applied::wiring_diagram::{Dir, WiringDiagram};

/// Trivial algebra: every operation maps to the identity function on `()`.
/// Used only as a smoke test of the trait shape.
struct IdAlgebra;

impl OperadAlgebra<E1, usize> for IdAlgebra {
    type Element = ();
    fn evaluate(
        &self,
        _op: &E1,
        _inputs: &[Self::Element],
    ) -> Result<Self::Element, CatgraphError> {
        Ok(())
    }
}

#[test]
fn identity_algebra_evaluates() {
    let op = E1::identity(&());
    IdAlgebra
        .evaluate(&op, &[()])
        .expect("identity algebra evaluates without error");
}

/// Borrowed verbatim from `wiring_diagram::tests::operadic` (inner sub-box
/// with 5 outer ports, no further inner circles).
fn make_inner_5_port() -> WiringDiagram<bool, i32, usize> {
    let inner_right_names: Vec<(Dir, usize)> = vec![
        (Dir::In, 0),
        (Dir::Out, 1),
        (Dir::In, 2),
        (Dir::Out, 3),
        (Dir::Out, 4),
    ];
    WiringDiagram::new(NamedCospan::new(
        vec![],
        vec![0, 1, 2, 2, 0],
        vec![true, true, false],
        vec![],
        inner_right_names,
    ))
}

/// Borrowed from the same test: outer diagram has 2 inner circles; circle 0
/// exposes 5 ports mirroring the inner's outer shape, circle 1 has 1 port.
/// The outer circle itself has a single port.
fn make_outer_two_inner_circles() -> WiringDiagram<bool, i32, usize> {
    let outer_left_names: Vec<(Dir, i32, usize)> = vec![
        (Dir::Out, 0, 0),
        (Dir::In, 0, 1),
        (Dir::Out, 0, 2),
        (Dir::In, 0, 3),
        (Dir::In, 0, 4),
        (Dir::Undirected, 1, 500),
    ];
    WiringDiagram::new(NamedCospan::new(
        vec![0, 0, 1, 1, 0, 1],
        vec![0],
        vec![true, false],
        outer_left_names,
        vec![(Dir::Out, 0)],
    ))
}

#[test]
fn circ_algebra_returns_outer_port_count() {
    let outer = make_outer_two_inner_circles();
    // Outer circle has exactly one port; algebra should see 1.
    let r = CircAlgebra.evaluate(&outer, &[]).unwrap();
    assert_eq!(r, 1);
}

#[test]
fn circ_algebra_commutes_with_substitution() {
    let outer = make_outer_two_inner_circles();
    let inner = make_inner_5_port();
    check_substitution_preserved(&CircAlgebra, outer, 0_i32, inner, &[])
        .expect("outer-port count is stable under operadic substitution");
}
