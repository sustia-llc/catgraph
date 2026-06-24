//! Wiring diagram operad API demonstration.
//!
//! Shows constructing wiring diagrams from NamedCospans, boundary
//! manipulation (add/delete/connect/rename nodes), orientation toggling,
//! label mapping, and operadic substitution.

use catgraph::named_cospan::NamedCospan;
use catgraph::operadic::Operadic;
use catgraph_applied::wiring_diagram::{Dir, WiringDiagram};
use either::Either::{Left, Right};

// ============================================================================
// Building a basic wiring diagram
// ============================================================================

fn basic_construction() {
    println!("=== Basic Construction ===\n");

    // A wiring diagram wraps a NamedCospan where:
    //   Left port names  = (Dir, InterCircle, IntraCircle) — inner circle ports
    //   Right port names = (Dir, IntraCircle)              — outer circle ports
    //
    // This diagram has no inner circles (empty left side)
    // and 3 ports on the outer circle connected to 2 middle nodes.
    let wd: WiringDiagram<bool, (), usize> = WiringDiagram::new(NamedCospan::new(
        vec![],                                          // left -> middle (no inner circles)
        vec![0, 1, 0],                                   // right -> middle
        vec![true, false],                               // middle node types
        vec![],                                          // left names (empty)
        vec![(Dir::In, 0), (Dir::Out, 1), (Dir::In, 2)], // right names
    ));

    println!("no-inner-circle diagram:");
    println!("  middle types = [true, false]");
    println!("  right ports  = In(0), Out(1), In(2)");
    println!("  ports 0 and 2 share middle node 0 (type true)");
    println!("  port 1 connects to middle node 1 (type false)");

    // A diagram with inner circles
    let wd2: WiringDiagram<char, i32, usize> = WiringDiagram::new(NamedCospan::new(
        vec![0, 0, 1],  // left -> middle
        vec![0, 1],     // right -> middle
        vec!['a', 'b'], // middle node types
        vec![
            (Dir::In, 1, 10),  // inner circle 1, port 10
            (Dir::Out, 1, 20), // inner circle 1, port 20
            (Dir::In, 2, 30),  // inner circle 2, port 30
        ],
        vec![
            (Dir::Out, 0), // outer port 0
            (Dir::In, 1),  // outer port 1
        ],
    ));

    println!("\ndiagram with 2 inner circles:");
    println!("  inner circle 1: ports 10(In), 20(Out) -> both to middle 'a'");
    println!("  inner circle 2: port 30(In) -> middle 'b'");
    println!("  outer: port 0(Out) -> middle 'a', port 1(In) -> middle 'b'");

    // Demonstrate that Dir has In, Out, and Undirected variants
    let _ = wd;
    let _ = wd2;
    println!(
        "\nDir variants: {:?}, {:?}, {:?}",
        Dir::In,
        Dir::Out,
        Dir::Undirected
    );
    println!("Dir::In.flipped()  = {:?}", Dir::In.flipped());
    println!("Dir::Out.flipped() = {:?}", Dir::Out.flipped());
    println!(
        "Dir::Undirected.flipped() = {:?}",
        Dir::Undirected.flipped()
    );

    println!();
}

// ============================================================================
// Boundary manipulation
// ============================================================================

fn boundary_manipulation() {
    println!("=== Boundary Manipulation ===\n");

    // Start with a simple diagram
    let mut wd: WiringDiagram<bool, (), usize> = WiringDiagram::new(NamedCospan::new(
        vec![],
        vec![0, 1, 2],
        vec![true, true, false],
        vec![],
        vec![(Dir::In, 0), (Dir::Out, 1), (Dir::In, 2)],
    ));

    // Add an unconnected boundary node on the right (outer) side
    println!("before add: 3 right ports");
    wd.add_boundary_node_unconnected(true, Right((Dir::Out, 99)));
    println!("after add:  4 right ports (added Out(99), type=true)");

    // Add a node on the left (inner) side
    wd.add_boundary_node_unconnected(false, Left((Dir::In, (), 42)));
    println!("added inner port: In((), 42), type=false");

    // Connect two boundary nodes that share the same middle type
    // Ports 0 and 1 both have type true but are on different middle nodes
    println!("\nbefore connect: ports In(0) and Out(1) on separate middles");
    wd.connect_pair(Right((Dir::In, 0)), Right((Dir::Out, 1)));
    println!("after connect:  ports In(0) and Out(1) now share a middle node");

    // Connect nodes with different types is a no-op
    println!("\nconnecting In(0)(true) and In(2)(false) = no-op (type mismatch)");
    wd.connect_pair(Right((Dir::In, 0)), Right((Dir::In, 2)));

    // Delete a boundary node
    println!("\nbefore delete: right ports include Out(99)");
    wd.delete_boundary_node(Right((Dir::Out, 99)));
    println!("after delete:  Out(99) removed");

    // Rename a boundary node
    wd.change_boundary_node_name(Right(((Dir::In, 2), (Dir::Out, 50))));
    println!("\nrenamed In(2) -> Out(50)");

    println!();
}

// ============================================================================
// Orientation toggling
// ============================================================================

fn toggle_orientation() {
    println!("=== Orientation Toggling ===\n");

    let mut wd: WiringDiagram<bool, i32, usize> = WiringDiagram::new(NamedCospan::new(
        vec![0, 1],
        vec![0],
        vec![true, false],
        vec![(Dir::In, 1, 10), (Dir::Out, 2, 20)],
        vec![(Dir::In, 0)],
    ));

    println!("before toggle:");
    println!("  left  = In(1,10), Out(2,20)");
    println!("  right = In(0)");

    // Toggle left (inner) side orientations
    wd.toggle_orientation(true);
    println!("\nafter toggle left:");
    println!("  left  = Out(1,10), In(2,20)  (In<->Out flipped)");

    // Toggle right (outer) side orientations
    wd.toggle_orientation(false);
    println!("\nafter toggle right:");
    println!("  right = Out(0)  (In flipped to Out)");

    // Undirected stays unchanged through toggle
    let mut wd2: WiringDiagram<bool, (), usize> = WiringDiagram::new(NamedCospan::new(
        vec![],
        vec![0],
        vec![true],
        vec![],
        vec![(Dir::Undirected, 0)],
    ));
    wd2.toggle_orientation(false);
    println!("\nUndirected after toggle  = Undirected (unchanged)");

    // Double toggle restores original
    wd.toggle_orientation(true);
    wd.toggle_orientation(true);
    println!("double toggle            = original restored");

    println!();
}

// ============================================================================
// Label mapping
// ============================================================================

fn label_mapping() {
    println!("=== Label Mapping ===\n");

    let wd: WiringDiagram<bool, (), usize> = WiringDiagram::new(NamedCospan::new(
        vec![],
        vec![0, 1],
        vec![true, false],
        vec![],
        vec![(Dir::In, 0), (Dir::Out, 1)],
    ));

    // Map labels from bool to &str
    let mapped: WiringDiagram<&str, (), usize> = wd.map(|b| if b { "high" } else { "low" });
    println!("mapped bool -> &str:");
    println!("  true  -> \"high\"");
    println!("  false -> \"low\"");
    println!("  (structural connections preserved, only labels change)");

    // Map to numeric
    let numeric: WiringDiagram<i32, (), usize> = wd.map(|b| if b { 1 } else { 0 });
    let _ = mapped;
    let _ = numeric;
    println!("mapped bool -> i32: true->1, false->0");

    println!();
}

// ============================================================================
// Operadic substitution
// ============================================================================

fn operadic_substitution() {
    println!("=== Operadic Substitution ===\n");

    type CircleName = i32;
    type WireName = usize;

    // Build outer diagram with one inner circle (circle 0) and one outer port.
    // Inner circle 0 has 3 ports: In(0), Out(1), In(2)
    // All connected to 2 middle nodes: ports 0,1 -> middle 0 (true), port 2 -> middle 1 (false)
    // Outer has 1 port: Out(0) -> middle 0 (true)
    let inner_right_names: Vec<(Dir, WireName)> = vec![(Dir::In, 0), (Dir::Out, 1), (Dir::In, 2)];
    let outer_left_names: Vec<(Dir, CircleName, WireName)> = inner_right_names
        .iter()
        .map(|(orient, name)| (orient.flipped(), 0, *name))
        .collect();

    let mut outer: WiringDiagram<bool, CircleName, WireName> =
        WiringDiagram::new(NamedCospan::new(
            vec![0, 0, 1],     // left -> middle
            vec![0],           // right -> middle
            vec![true, false], // middle types
            outer_left_names,
            vec![(Dir::Out, 0)],
        ));

    println!("outer diagram:");
    println!("  inner circle 0: 3 ports -> 2 middles");
    println!("  outer: 1 port Out(0) -> middle 'true'");

    // Build inner diagram (no further inner circles)
    // 3 ports on outer matching the outer's inner circle:
    //   In(0) -> middle 0 (true), Out(1) -> middle 0 (true), In(2) -> middle 1 (false)
    let inner: WiringDiagram<bool, CircleName, WireName> = WiringDiagram::new(NamedCospan::new(
        vec![],
        vec![0, 0, 1],
        vec![true, false],
        vec![],
        inner_right_names,
    ));

    println!("inner diagram:");
    println!("  no inner circles");
    println!("  3 outer ports matching outer's circle 0");

    // Substitute inner into circle 0 of outer
    let result = outer.operadic_substitution(0, inner);
    println!("\nsubstitution result = {:?}", result.is_ok());
    println!("after substitution:");
    println!("  inner circles remaining = 0 (circle 0 was replaced)");
    println!("  outer port Out(0) still present");

    // Failed substitution: non-existent circle
    let inner2: WiringDiagram<bool, CircleName, WireName> =
        WiringDiagram::new(NamedCospan::new(vec![], vec![], vec![], vec![], vec![]));
    let mut target: WiringDiagram<bool, CircleName, WireName> =
        WiringDiagram::new(NamedCospan::new(
            vec![0],
            vec![0],
            vec![true],
            vec![(Dir::In, 1, 0)],
            vec![(Dir::Out, 0)],
        ));
    // Substituting circle 99 which doesn't exist
    let result2 = target.operadic_substitution(99, inner2);
    println!("\nnon-existent circle sub  = {:?}", result2.is_ok());

    println!();
}

// ============================================================================
// Multi-circle operadic substitution
// ============================================================================

fn multi_circle_substitution() {
    println!("=== Multi-Circle Operadic Substitution ===\n");

    type CircleName = i32;
    type WireName = char;

    // Outer diagram with 2 inner circles and 3 outer ports
    let outer_left_names: Vec<(Dir, CircleName, WireName)> = vec![
        (Dir::Undirected, 1, 'r'),
        (Dir::Undirected, 1, 's'),
        (Dir::Undirected, 2, 'u'),
        (Dir::Undirected, 2, 'v'),
    ];
    let outer_right_names: Vec<(Dir, WireName)> = vec![
        (Dir::Undirected, 'a'),
        (Dir::Undirected, 'b'),
        (Dir::Undirected, 'c'),
    ];

    let mut outer: WiringDiagram<(), CircleName, WireName> = WiringDiagram::new(NamedCospan::new(
        vec![0, 1, 1, 2], // left -> middle
        vec![0, 1, 2],    // right -> middle
        vec![(); 3],      // untyped middle nodes
        outer_left_names,
        outer_right_names,
    ));

    println!("outer: 2 inner circles, 3 outer ports");
    println!("  circle 1: ports r,s");
    println!("  circle 2: ports u,v");
    println!("  outer: ports a,b,c");

    // First substitution: replace circle 1
    let inner1: WiringDiagram<(), CircleName, WireName> = WiringDiagram::new(NamedCospan::new(
        vec![],
        vec![0, 0],
        vec![()],
        vec![],
        vec![(Dir::Undirected, 'r'), (Dir::Undirected, 's')],
    ));

    let result1 = outer.operadic_substitution(1, inner1);
    println!("\nsubstitute into circle 1 = {:?}", result1.is_ok());
    println!("  circle 1 removed, its ports r,s now connected through inner");

    // Second substitution: replace circle 2
    let inner2: WiringDiagram<(), CircleName, WireName> = WiringDiagram::new(NamedCospan::new(
        vec![],
        vec![0, 1],
        vec![(); 2],
        vec![],
        vec![(Dir::Undirected, 'u'), (Dir::Undirected, 'v')],
    ));

    let result2 = outer.operadic_substitution(2, inner2);
    println!("substitute into circle 2 = {:?}", result2.is_ok());
    println!("  all inner circles resolved, only outer ports remain");

    println!();
}

fn main() {
    basic_construction();
    boundary_manipulation();
    toggle_orientation();
    label_mapping();
    operadic_substitution();
    multi_circle_substitution();
}
