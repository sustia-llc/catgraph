//! Integration tests for `PetriNet`: chemical reactions, reachability, composition, cospan roundtrip.

use catgraph::cospan::Cospan;
use catgraph_applied::petri_net::{Marking, PetriNet, Transition};
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Shorthand for `Decimal::from(n)`.
fn d(n: i64) -> Decimal {
    Decimal::from(n)
}

// ---------------------------------------------------------------------------
// Chemical reactions
// ---------------------------------------------------------------------------

#[test]
fn combustion_h2_o2_h2o() {
    let net: PetriNet<&str> = PetriNet::new(
        vec!["H2", "O2", "H2O"],
        vec![Transition::new(vec![(0, d(2)), (1, d(1))], vec![(2, d(2))])],
    );
    let m0 = Marking::from_vec(vec![(0, d(4)), (1, d(2))]);
    let m1 = net.fire(0, &m0).unwrap();
    let m2 = net.fire(0, &m1).unwrap();
    assert_eq!(m2.get(0), Decimal::ZERO);
    assert_eq!(m2.get(1), Decimal::ZERO);
    assert_eq!(m2.get(2), d(4));
    assert!(net.enabled(&m2).is_empty());
}

#[test]
fn two_step_synthesis() {
    let net: PetriNet<&str> = PetriNet::new(
        vec!["A", "B", "C", "D", "E"],
        vec![
            Transition::new(vec![(0, d(1)), (1, d(1))], vec![(2, d(1))]),
            Transition::new(vec![(2, d(1)), (3, d(1))], vec![(4, d(1))]),
        ],
    );
    let m0 = Marking::from_vec(vec![(0, d(1)), (1, d(1)), (3, d(1))]);
    assert_eq!(net.enabled(&m0), vec![0]);
    let m1 = net.fire(0, &m0).unwrap();
    assert_eq!(m1.get(2), d(1));
    assert_eq!(net.enabled(&m1), vec![1]);
    let m2 = net.fire(1, &m1).unwrap();
    assert_eq!(m2.get(4), d(1));
}

#[test]
fn haber_process_stoichiometry() {
    let net: PetriNet<&str> = PetriNet::new(
        vec!["N2", "H2", "NH3"],
        vec![Transition::new(vec![(0, d(1)), (1, d(3))], vec![(2, d(2))])],
    );
    let m0 = Marking::from_vec(vec![(0, d(1)), (1, d(3))]);
    let m1 = net.fire(0, &m0).unwrap();
    assert_eq!(m1.get(2), d(2));
    assert!(net.enabled(&m1).is_empty());
}

// ---------------------------------------------------------------------------
// Reachability
// ---------------------------------------------------------------------------

#[test]
fn producer_consumer_bounded_buffer() {
    let net: PetriNet<&str> = PetriNet::new(
        vec!["empty", "full"],
        vec![
            Transition::new(vec![(0, d(1))], vec![(1, d(1))]),
            Transition::new(vec![(1, d(1))], vec![(0, d(1))]),
        ],
    );
    let m0 = Marking::from_vec(vec![(0, d(3))]);
    let reachable = net.reachable(&m0, 10);
    assert_eq!(reachable.len(), 4);
    assert!(net.can_reach(&m0, &Marking::from_vec(vec![(1, d(3))]), 10));
    assert!(!net.can_reach(&m0, &Marking::from_vec(vec![(0, d(4))]), 10));
}

#[test]
fn deadlock_detection() {
    let net: PetriNet<&str> = PetriNet::new(
        vec!["fork0", "fork1", "think0", "think1", "eat0", "eat1"],
        vec![
            Transition::new(vec![(2, d(1)), (0, d(1)), (1, d(1))], vec![(4, d(1))]),
            Transition::new(vec![(4, d(1))], vec![(2, d(1)), (0, d(1)), (1, d(1))]),
            Transition::new(vec![(3, d(1)), (0, d(1)), (1, d(1))], vec![(5, d(1))]),
            Transition::new(vec![(5, d(1))], vec![(3, d(1)), (0, d(1)), (1, d(1))]),
        ],
    );
    let m0 = Marking::from_vec(vec![(0, d(1)), (1, d(1)), (2, d(1)), (3, d(1))]);
    let eating0 = Marking::from_vec(vec![(3, d(1)), (4, d(1))]);
    assert!(net.can_reach(&m0, &eating0, 5));
}

// ---------------------------------------------------------------------------
// Composition
// ---------------------------------------------------------------------------

#[test]
fn sequential_pipeline() {
    let step1: PetriNet<char> = PetriNet::new(
        vec!['A', 'B'],
        vec![Transition::new(vec![(0, d(1))], vec![(1, d(1))])],
    );
    let step2: PetriNet<char> = PetriNet::new(
        vec!['B', 'C'],
        vec![Transition::new(vec![(0, d(1))], vec![(1, d(1))])],
    );
    let pipeline = step1.sequential(&step2).unwrap();
    assert_eq!(pipeline.place_count(), 3);
    let m0 = Marking::from_vec(vec![(0, d(1))]);
    let target = Marking::from_vec(vec![(2, d(1))]);
    assert!(pipeline.can_reach(&m0, &target, 5));
}

#[test]
fn parallel_independence() {
    let a: PetriNet<char> = PetriNet::new(
        vec!['a', 'b'],
        vec![Transition::new(vec![(0, d(1))], vec![(1, d(1))])],
    );
    let b: PetriNet<char> = PetriNet::new(
        vec!['x', 'y'],
        vec![Transition::new(vec![(0, d(1))], vec![(1, d(1))])],
    );
    let combined = a.parallel(&b);
    let m0 = Marking::from_vec(vec![(0, d(1))]);
    let m1 = combined.fire(0, &m0).unwrap();
    assert_eq!(m1.get(1), d(1));
    assert_eq!(m1.get(2), Decimal::ZERO);
}

// ---------------------------------------------------------------------------
// Cospan roundtrip
// ---------------------------------------------------------------------------

#[test]
fn cospan_roundtrip_preserves_structure() {
    let cospan: Cospan<char> = Cospan::new(vec![0, 1, 1, 1], vec![2, 2], vec!['N', 'H', 'A']);
    let net = PetriNet::from_cospan(&cospan);
    let back = net.transition_as_cospan(0);
    assert_eq!(back.middle(), cospan.middle());
    let mut left_counts_orig: HashMap<usize, usize> = HashMap::new();
    for &idx in cospan.left_to_middle() {
        *left_counts_orig.entry(idx).or_insert(0) += 1;
    }
    let mut left_counts_back: HashMap<usize, usize> = HashMap::new();
    for &idx in back.left_to_middle() {
        *left_counts_back.entry(idx).or_insert(0) += 1;
    }
    assert_eq!(left_counts_orig, left_counts_back);
}

// ============================================================================
// direct PetriNet::permute_side tests
// ============================================================================

#[cfg(test)]
mod v0_3_1_braiding {
    use catgraph::category::Composable;
    use catgraph::monoidal::{Monoidal, SymmetricMonoidalMorphism};
    use catgraph_applied::petri_net::{PetriNet, Transition};
    use permutations::Permutation;
    use rust_decimal::Decimal;

    fn two_transition_net() -> PetriNet<char> {
        // Two transitions on a 2-place net, each with one pre and one post arc.
        let places = vec!['a', 'b'];
        let t0 = Transition::new(vec![(0, Decimal::ONE)], vec![(1, Decimal::ONE)]);
        let t1 = Transition::new(vec![(1, Decimal::ONE)], vec![(0, Decimal::ONE)]);
        PetriNet::new(places, vec![t0, t1])
    }

    #[test]
    fn t3_1_identity_permutation_is_no_op() {
        let original = two_transition_net();
        let mut net = original.clone();
        net.permute_side(&Permutation::identity(net.transitions().len()), false);
        assert_eq!(net.places(), original.places());
        assert_eq!(net.transitions(), original.transitions());
    }

    #[test]
    fn t3_2_transposition_swaps_transition_order() {
        let original = two_transition_net();
        let mut net = original.clone();

        let swap = Permutation::transposition(2, 0, 1);
        net.permute_side(&swap, true);

        // Transitions permuted
        assert_eq!(net.transitions().len(), 2);
        assert_eq!(net.transitions()[0], original.transitions()[1]);
        assert_eq!(net.transitions()[1], original.transitions()[0]);
        // Places unchanged
        assert_eq!(net.places(), original.places());
        // Codomain sequence reflects the swap
        assert_ne!(
            net.codomain(),
            original.codomain(),
            "codomain must observe the braiding"
        );
    }

    #[test]
    fn t3_3_involution() {
        let original = two_transition_net();
        let mut net = original.clone();
        let swap = Permutation::transposition(2, 0, 1);
        net.permute_side(&swap, true);
        net.permute_side(&swap, true);
        assert_eq!(net.places(), original.places());
        assert_eq!(net.transitions(), original.transitions());
    }

    #[test]
    fn t3_4_naturality_on_tensor_codomain() {
        // net1 ⊗ net2 followed by codomain-swap yields net2 ⊗ net1 codomain.
        let mut net1 = PetriNet::new(
            vec!['x'],
            vec![Transition::new(vec![], vec![(0, Decimal::ONE)])],
        );
        let net2 = PetriNet::new(
            vec!['y'],
            vec![Transition::new(vec![], vec![(0, Decimal::ONE)])],
        );

        let mut reverse = net2.clone();
        reverse.monoidal(net1.clone());

        net1.monoidal(net2);
        let swap = Permutation::transposition(2, 0, 1);
        net1.permute_side(&swap, true);

        assert_eq!(
            net1.codomain(),
            reverse.codomain(),
            "swap on (net1 ⊗ net2).codomain equals (net2 ⊗ net1).codomain"
        );
        assert_eq!(
            net1.domain(),
            reverse.domain(),
            "swap on (net1 ⊗ net2).domain equals (net2 ⊗ net1).domain"
        );
    }
}

// ============================================================================
// Transition::relabel arc dedup tests
// ============================================================================

#[cfg(test)]
mod v0_3_1_arc_dedup {
    use catgraph_applied::petri_net::Transition;
    use rust_decimal::Decimal;

    #[test]
    fn t4_1_quotient_collapses_pre_arcs_with_summed_weights() {
        // Pre-arcs [(0, 1), (1, 2)]. Quotient [0, 0] maps both to place 0.
        // After relabel+dedup, pre should be [(0, 3)].
        let pre = vec![(0usize, Decimal::ONE), (1usize, Decimal::TWO)];
        let t = Transition::new(pre, vec![]);
        let relabelled = t.relabel(&[0, 0]);
        assert_eq!(relabelled.pre(), &[(0usize, Decimal::from(3))]);
        assert_eq!(relabelled.post(), &[] as &[(usize, Decimal)]);
    }

    #[test]
    fn t4_2_distinct_places_not_merged() {
        // Quotient [0, 1] is identity on a 2-place apex — no dedup happens.
        let pre = vec![(0usize, Decimal::ONE), (1usize, Decimal::TWO)];
        let t = Transition::new(pre.clone(), vec![]);
        let relabelled = t.relabel(&[0, 1]);
        assert_eq!(relabelled.pre(), &pre[..]);
    }

    #[test]
    fn t4_3_pre_and_post_separate_self_loop_preserved() {
        // Transition has pre = [(0, 1)] and post = [(0, 1)].
        // Quotient is identity. Pre and post stay separate (self-loop).
        let t = Transition::new(vec![(0usize, Decimal::ONE)], vec![(0usize, Decimal::ONE)]);
        let relabelled = t.relabel(&[0]);
        assert_eq!(relabelled.pre(), &[(0usize, Decimal::ONE)]);
        assert_eq!(relabelled.post(), &[(0usize, Decimal::ONE)]);
    }

    #[test]
    fn t4_4_order_independence() {
        // Two arcs collapsing to the same place, starting in different orders,
        // produce the same canonical merged form.
        let q = &[0, 0];
        let a = Transition::new(vec![(0usize, Decimal::ONE), (1usize, Decimal::TWO)], vec![])
            .relabel(q);
        let b = Transition::new(vec![(1usize, Decimal::TWO), (0usize, Decimal::ONE)], vec![])
            .relabel(q);
        assert_eq!(a.pre(), b.pre());
    }
}
