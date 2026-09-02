//! Worked example: symmetric-monoidal braiding on a Petri net.
//!
//! Builds two single-place nets, tensors them into a 2-place Petri net, and
//! permutes each side of the declared boundary in turn, printing the domain
//! and codomain words before and after.

use catgraph::category::Composable;
use catgraph::monoidal::{Monoidal, SymmetricMonoidalMorphism};
use catgraph_applied::petri_net::{PetriNet, Transition};
use permutations::Permutation;
use rust_decimal::Decimal;

fn main() {
    let left = PetriNet::new(
        vec!['x'],
        vec![Transition::new(
            vec![(0, Decimal::ONE)],
            vec![(0, Decimal::ONE)],
        )],
        vec![0],
        vec![0],
    )
    .unwrap();
    let right = PetriNet::new(
        vec!['y'],
        vec![Transition::new(
            vec![(0, Decimal::ONE)],
            vec![(0, Decimal::ONE)],
        )],
        vec![0],
        vec![0],
    )
    .unwrap();

    let mut tensor = left;
    tensor.monoidal(right);
    println!(
        "tensor:            {:?} -> {:?}",
        tensor.domain(),
        tensor.codomain()
    );

    let swap = Permutation::transposition(2, 0, 1);

    let mut braided = tensor.clone();
    braided.permute_side(&swap, true);
    println!(
        "codomain braiding: {:?} -> {:?}",
        braided.domain(),
        braided.codomain()
    );

    let mut braided = tensor.clone();
    braided.permute_side(&swap, false);
    println!(
        "domain braiding:   {:?} -> {:?}",
        braided.domain(),
        braided.codomain()
    );

    // A permutation whose length is not the permuted side's arity is a no-op.
    let mut untouched = tensor.clone();
    untouched.permute_side(&Permutation::rotation_left(3, 1), true);
    println!(
        "arity mismatch:    {:?} -> {:?}",
        untouched.domain(),
        untouched.codomain()
    );
}
