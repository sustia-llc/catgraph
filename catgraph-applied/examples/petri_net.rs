//! Petri net API demonstration.
//!
//! Models chemical reactions as Petri nets, shows firing semantics,
//! reachability analysis, composition, and cospan bridge.

use catgraph::cospan::Cospan;
use catgraph_applied::petri_net::{Marking, PetriNet, Transition};
use rust_decimal::Decimal;

/// Shorthand for `Decimal::from(n)`.
fn d(n: i64) -> Decimal {
    Decimal::from(n)
}

// ============================================================================
// Construction
// ============================================================================

fn construction() {
    println!("=== Construction ===\n");

    let net: PetriNet<&str> = PetriNet::new(
        vec!["H2", "O2", "H2O"],
        vec![Transition::new(vec![(0, d(2)), (1, d(1))], vec![(2, d(2))])],
    );
    println!("places:      {}", net.place_count());
    println!("transitions: {}", net.transition_count());
    println!("source places: {:?}", net.source_places());
    println!("sink places:   {:?}", net.sink_places());
    println!("arc H2->t0:  {}", net.arc_weight_pre(0, 0));
    println!("arc O2->t0:  {}", net.arc_weight_pre(1, 0));
    println!("arc t0->H2O: {}", net.arc_weight_post(2, 0));
    println!();
}

// ============================================================================
// Firing
// ============================================================================

fn firing() {
    println!("=== Firing ===\n");

    let net: PetriNet<&str> = PetriNet::new(
        vec!["H2", "O2", "H2O"],
        vec![Transition::new(vec![(0, d(2)), (1, d(1))], vec![(2, d(2))])],
    );
    let m0 = Marking::from_vec(vec![(0, d(4)), (1, d(2))]);
    println!("m0: H2={}, O2={}, H2O={}", m0.get(0), m0.get(1), m0.get(2));
    println!("enabled: {:?}", net.enabled(&m0));

    let m1 = net.fire(0, &m0).unwrap();
    println!("m1: H2={}, O2={}, H2O={}", m1.get(0), m1.get(1), m1.get(2));

    let m2 = net.fire(0, &m1).unwrap();
    println!("m2: H2={}, O2={}, H2O={}", m2.get(0), m2.get(1), m2.get(2));
    println!("enabled: {:?}  (reaction complete)", net.enabled(&m2));
    println!();
}

// ============================================================================
// Reachability
// ============================================================================

fn reachability() {
    println!("=== Reachability ===\n");

    let net: PetriNet<&str> = PetriNet::new(
        vec!["empty", "full"],
        vec![
            Transition::new(vec![(0, d(1))], vec![(1, d(1))]),
            Transition::new(vec![(1, d(1))], vec![(0, d(1))]),
        ],
    );
    let m0 = Marking::from_vec(vec![(0, d(2))]);
    let reachable = net.reachable(&m0, 10);
    println!("buffer size 2: {} reachable markings", reachable.len());

    let full = Marking::from_vec(vec![(1, d(2))]);
    println!("can reach full buffer: {}", net.can_reach(&m0, &full, 10));

    let overflow = Marking::from_vec(vec![(1, d(3))]);
    println!(
        "can reach overflow:    {}",
        net.can_reach(&m0, &overflow, 10)
    );
    println!();
}

// ============================================================================
// Composition
// ============================================================================

fn composition() {
    println!("=== Composition ===\n");

    let step1: PetriNet<char> = PetriNet::new(
        vec!['A', 'B'],
        vec![Transition::new(vec![(0, d(1))], vec![(1, d(1))])],
    );
    let step2: PetriNet<char> = PetriNet::new(
        vec!['B', 'C'],
        vec![Transition::new(vec![(0, d(1))], vec![(1, d(1))])],
    );

    let parallel = step1.parallel(&step2);
    println!(
        "parallel: {} places, {} transitions",
        parallel.place_count(),
        parallel.transition_count()
    );

    let sequential = step1.sequential(&step2).unwrap();
    println!(
        "sequential: {} places, {} transitions (B merged)",
        sequential.place_count(),
        sequential.transition_count()
    );

    let m0 = Marking::from_vec(vec![(0, d(1))]);
    let target = Marking::from_vec(vec![(2, d(1))]);
    println!(
        "A -> C reachable: {}",
        sequential.can_reach(&m0, &target, 5)
    );
    println!();
}

// ============================================================================
// Cospan Bridge
// ============================================================================

fn cospan_bridge() {
    println!("=== Cospan Bridge ===\n");

    let cospan: Cospan<char> = Cospan::new(vec![0, 1, 1, 1], vec![2, 2], vec!['N', 'H', 'A']);
    let net = PetriNet::from_cospan(&cospan);
    println!(
        "from_cospan: {} places, {} transitions",
        net.place_count(),
        net.transition_count()
    );
    println!("pre N(0):  {}", net.arc_weight_pre(0, 0));
    println!("pre H(1):  {}", net.arc_weight_pre(1, 0));
    println!("post A(2): {}", net.arc_weight_post(2, 0));

    let back = net.transition_as_cospan(0);
    println!("roundtrip middle: {:?}", back.middle());
    println!(
        "roundtrip left len:  {} (original: {})",
        back.left_to_middle().len(),
        cospan.left_to_middle().len()
    );
    println!(
        "roundtrip right len: {} (original: {})",
        back.right_to_middle().len(),
        cospan.right_to_middle().len()
    );
    println!();
}

fn main() {
    construction();
    firing();
    reachability();
    composition();
    cospan_bridge();
}
