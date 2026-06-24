//! F&S *Seven Sketches* Ex 6.100 worked demo:
//! `Circ : WiringDiagram → Set` via outer-port counts.
//!
//! Builds an outer wiring diagram with two inner circles, plugs a 5-port
//! sub-diagram into circle 0, and prints `Circ(op)` before and after the
//! substitution to witness that the outer-port count is invariant under
//! inner-circle substitution.

use catgraph::named_cospan::NamedCospan;
use catgraph::operadic::Operadic;
use catgraph_applied::operad_algebra::{CircAlgebra, OperadAlgebra};
use catgraph_applied::wiring_diagram::{Dir, WiringDiagram};

fn main() {
    // Inner sub-box: 5 outer ports, no further inner circles.
    let inner_right_names: Vec<(Dir, usize)> = vec![
        (Dir::In, 0),
        (Dir::Out, 1),
        (Dir::In, 2),
        (Dir::Out, 3),
        (Dir::Out, 4),
    ];
    let inner: WiringDiagram<bool, i32, usize> = WiringDiagram::new(NamedCospan::new(
        vec![],
        vec![0, 1, 2, 2, 0],
        vec![true, true, false],
        vec![],
        inner_right_names,
    ));

    // Outer host: 2 inner circles; circle 0 has 5 ports that match the
    // inner's outer ports; circle 1 has 1 port. Exactly one outer port.
    let outer_left_names: Vec<(Dir, i32, usize)> = vec![
        (Dir::Out, 0, 0),
        (Dir::In, 0, 1),
        (Dir::Out, 0, 2),
        (Dir::In, 0, 3),
        (Dir::In, 0, 4),
        (Dir::Undirected, 1, 500),
    ];
    let mut outer: WiringDiagram<bool, i32, usize> = WiringDiagram::new(NamedCospan::new(
        vec![0, 0, 1, 1, 0, 1],
        vec![0],
        vec![true, false],
        outer_left_names,
        vec![(Dir::Out, 0)],
    ));

    let before = CircAlgebra.evaluate(&outer, &[]).expect("evaluate before");
    println!("Circ(outer) before substitution = {before}");

    outer
        .operadic_substitution(0_i32, inner)
        .expect("substitute circle 0");

    let after = CircAlgebra.evaluate(&outer, &[]).expect("evaluate after");
    println!("Circ(outer) after  substitution = {after}");
    assert_eq!(
        before, after,
        "Ex 6.100: outer-port count is invariant under substitution",
    );
}
