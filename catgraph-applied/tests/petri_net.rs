//! Integration tests for `PetriNet`: chemical reactions, reachability, composition, cospan roundtrip.

use catgraph::category::Composable;
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
        vec![],
        vec![],
    )
    .unwrap();
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
        vec![],
        vec![],
    )
    .unwrap();
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
        vec![],
        vec![],
    )
    .unwrap();
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
        vec![],
        vec![],
    )
    .unwrap();
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
        vec![],
        vec![],
    )
    .unwrap();
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
        vec![0],
        vec![1],
    )
    .unwrap();
    let step2: PetriNet<char> = PetriNet::new(
        vec!['B', 'C'],
        vec![Transition::new(vec![(0, d(1))], vec![(1, d(1))])],
        vec![0],
        vec![1],
    )
    .unwrap();
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
        vec![0],
        vec![1],
    )
    .unwrap();
    let b: PetriNet<char> = PetriNet::new(
        vec!['x', 'y'],
        vec![Transition::new(vec![(0, d(1))], vec![(1, d(1))])],
        vec![0],
        vec![1],
    )
    .unwrap();
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
    let cospan: Cospan<char> =
        Cospan::new(vec![0, 1, 1, 1], vec![2, 2], vec!['N', 'H', 'A']).unwrap();
    let net = PetriNet::from_cospan(&cospan);
    // `from_cospan` stores both legs verbatim, so the net's boundary is the
    // cospan's boundary rather than a re-expansion of the arc weights.
    assert_eq!(net.left_to_place(), cospan.left_to_middle());
    assert_eq!(net.right_to_place(), cospan.right_to_middle());
    assert_eq!(net.domain(), cospan.domain());
    assert_eq!(net.codomain(), cospan.codomain());

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
mod braiding {
    use catgraph::category::Composable;
    use catgraph::monoidal::{Monoidal, SymmetricMonoidalMorphism};
    use catgraph_applied::petri_net::{PetriNet, Transition};
    use permutations::Permutation;
    use rust_decimal::Decimal;

    /// Places `['a','b','c']`, two transitions, domain leg `[0, 1]` (arity 2)
    /// and codomain leg `[0, 1, 2]` (arity 3).
    ///
    /// The three lengths — 2 domain slots, 3 codomain slots, 2 transitions —
    /// are deliberately not all equal, so a permutation sized for one of them
    /// is the wrong size for the others.
    fn asymmetric_net() -> PetriNet<char> {
        let t0 = Transition::new(vec![(0, Decimal::ONE)], vec![(1, Decimal::ONE)]);
        let t1 = Transition::new(vec![(1, Decimal::ONE)], vec![(2, Decimal::ONE)]);
        PetriNet::new(vec!['a', 'b', 'c'], vec![t0, t1], vec![0, 1], vec![0, 1, 2]).unwrap()
    }

    #[test]
    fn identity_permutation_is_a_no_op_on_either_side() {
        let original = asymmetric_net();
        for (of_codomain, arity) in [(false, 2usize), (true, 3)] {
            let mut net = original.clone();
            net.permute_side(&Permutation::identity(arity), of_codomain);
            assert_eq!(net.places(), original.places());
            assert_eq!(net.transitions(), original.transitions());
            assert_eq!(net.left_to_place(), original.left_to_place());
            assert_eq!(net.right_to_place(), original.right_to_place());
        }
    }

    /// `permute_side` moves the wire at slot `i` of the named side to slot
    /// `p.apply(i)`, leaving the other leg, the places and the transitions
    /// alone.
    ///
    /// Values written out for `p = rotation_left(3, 1)` on the codomain and
    /// `p = transposition(2, 0, 1)` on the domain of [`asymmetric_net`].
    #[test]
    fn permute_side_moves_the_named_leg_only() {
        let original = asymmetric_net();

        let mut net = original.clone();
        net.permute_side(&Permutation::rotation_left(3, 1), true);
        assert_eq!(net.right_to_place(), &[2, 0, 1]);
        assert_eq!(net.codomain(), vec!['c', 'a', 'b']);
        assert_eq!(net.left_to_place(), &[0, 1], "domain leg untouched");
        assert_eq!(net.domain(), vec!['a', 'b']);
        assert_eq!(net.transitions(), original.transitions());
        assert_eq!(net.places(), original.places());

        let mut net = original.clone();
        net.permute_side(&Permutation::transposition(2, 0, 1), false);
        assert_eq!(net.left_to_place(), &[1, 0]);
        assert_eq!(net.domain(), vec!['b', 'a']);
        assert_eq!(net.right_to_place(), &[0, 1, 2], "codomain leg untouched");
        assert_eq!(net.transitions(), original.transitions());
        assert_eq!(net.places(), original.places());
    }

    /// #302 — a permutation whose length is not the permuted side's arity
    /// leaves the whole net alone.
    ///
    /// **What this ranges over.** One net, and three mismatch sources at
    /// `p.len()` 2 and 3: the opposite leg's arity, the transition count (the
    /// pre-#272 sizing), and a length matching neither. Both flag values are
    /// driven. It does not sweep arities beyond `{2, 3}`.
    ///
    /// The final block is what stops the no-op assertions from passing for
    /// free: the same `p`, at the length the permuted side does have, changes
    /// that leg.
    #[test]
    fn mismatched_permutation_length_is_a_no_op() {
        let original = asymmetric_net();
        assert_eq!(original.transitions().len(), 2, "fixture: 2 transitions");

        // `p.len() == 2` is the domain arity and the transition count, and is
        // neither on the codomain side.
        let p2 = Permutation::transposition(2, 0, 1);
        let mut net = original.clone();
        net.permute_side(&p2, true);
        assert_eq!(
            net.right_to_place(),
            original.right_to_place(),
            "p.len() 2 != codomain arity 3: no-op"
        );
        assert_eq!(net.left_to_place(), original.left_to_place());
        assert_eq!(
            net.transitions(),
            original.transitions(),
            "p sized to the transition count must not reorder transitions"
        );

        // `p.len() == 3` is the codomain arity, and is not the domain arity.
        let p3 = Permutation::rotation_left(3, 1);
        let mut net = original.clone();
        net.permute_side(&p3, false);
        assert_eq!(
            net.left_to_place(),
            original.left_to_place(),
            "p.len() 3 != domain arity 2: no-op"
        );
        assert_eq!(net.right_to_place(), original.right_to_place());
        assert_eq!(net.transitions(), original.transitions());

        // A length matching no arity of this net, on both sides.
        let p5 = Permutation::rotation_left(5, 2);
        for of_codomain in [false, true] {
            let mut net = original.clone();
            net.permute_side(&p5, of_codomain);
            assert_eq!(net.left_to_place(), original.left_to_place());
            assert_eq!(net.right_to_place(), original.right_to_place());
            assert_eq!(net.transitions(), original.transitions());
        }

        // Not vacuous: at the permuted side's own arity, each `p` moves it.
        let mut net = original.clone();
        net.permute_side(&p2, false);
        assert_eq!(net.left_to_place(), &[1, 0]);
        let mut net = original.clone();
        net.permute_side(&p3, true);
        assert_eq!(net.right_to_place(), &[2, 0, 1]);
    }

    #[test]
    fn permuting_the_codomain_twice_by_an_involution_restores_it() {
        let original = asymmetric_net();
        let mut net = original.clone();
        let swap = Permutation::transposition(3, 0, 1);
        net.permute_side(&swap, true);
        net.permute_side(&swap, true);
        assert_eq!(net.right_to_place(), original.right_to_place());
        assert_eq!(net.places(), original.places());
        assert_eq!(net.transitions(), original.transitions());
    }

    #[test]
    fn naturality_on_tensor_codomain() {
        // net1 ⊗ net2 followed by codomain-swap yields net2 ⊗ net1's codomain.
        let mut net1 = PetriNet::new(
            vec!['x'],
            vec![Transition::new(vec![], vec![(0, Decimal::ONE)])],
            vec![],
            vec![0],
        )
        .unwrap();
        let net2 = PetriNet::new(
            vec!['y'],
            vec![Transition::new(vec![], vec![(0, Decimal::ONE)])],
            vec![],
            vec![0],
        )
        .unwrap();

        let mut reverse = net2.clone();
        reverse.monoidal(net1.clone());

        net1.monoidal(net2);
        assert_eq!(net1.codomain(), vec!['x', 'y']);
        let swap = Permutation::transposition(2, 0, 1);
        net1.permute_side(&swap, true);

        assert_eq!(net1.codomain(), vec!['y', 'x']);
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
// The two boundary invariants #272's reading makes maintained rather than
// derived
// ============================================================================

/// `compose` yields the first operand's domain and the second's codomain, and
/// `monoidal` concatenates both boundary words.
///
/// **What this ranges over.** Two fixtures — a two-place net composed with a
/// two-place net over a one-wire interface, and the same two tensored — on one
/// `Lambda` (`char`). It does not sweep arities, and it says nothing about the
/// composite's transitions.
#[test]
fn compose_and_monoidal_agree_with_the_stored_boundary() {
    use catgraph::monoidal::Monoidal;

    let f: PetriNet<char> = PetriNet::new(
        vec!['A', 'B'],
        vec![Transition::new(vec![(0, d(1))], vec![(1, d(1))])],
        vec![0],
        vec![1],
    )
    .unwrap();
    let g: PetriNet<char> = PetriNet::new(
        vec!['B', 'C'],
        vec![Transition::new(vec![(0, d(1))], vec![(1, d(1))])],
        vec![0],
        vec![1],
    )
    .unwrap();

    let composed = f.compose(&g).expect("['B'] matches ['B']");
    assert_eq!(composed.domain(), f.domain());
    assert_eq!(composed.domain(), vec!['A']);
    assert_eq!(composed.codomain(), g.codomain());
    assert_eq!(composed.codomain(), vec!['C']);

    let mut tensored = f.clone();
    tensored.monoidal(g.clone());
    assert_eq!(tensored.domain(), vec!['A', 'B']);
    assert_eq!(tensored.codomain(), vec!['B', 'C']);

    // A mismatched interface is an error, not a silent splice.
    assert!(g.compose(&f).is_err(), "['C'] does not match ['A']");
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
