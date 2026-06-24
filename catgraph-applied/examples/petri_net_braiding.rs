//! Worked example: symmetric-monoidal braiding on a Petri net.
//!
//! Builds two single-place nets, tensors them into a 2-transition Petri
//! net, applies a codomain transposition, and prints the before/after
//! codomain ordering. Shipped in catgraph-applied v0.3.1.

use catgraph::category::Composable;
use catgraph::monoidal::{Monoidal, SymmetricMonoidalMorphism};
use catgraph_applied::petri_net::{PetriNet, Transition};
use permutations::Permutation;
use rust_decimal::Decimal;

fn main() {
    let left = PetriNet::new(
        vec!['x'],
        vec![Transition::new(vec![], vec![(0, Decimal::ONE)])],
    );
    let right = PetriNet::new(
        vec!['y'],
        vec![Transition::new(vec![], vec![(0, Decimal::ONE)])],
    );

    let mut tensor = left;
    tensor.monoidal(right);
    println!("before braiding: codomain = {:?}", tensor.codomain());

    let swap = Permutation::transposition(2, 0, 1);
    tensor.permute_side(&swap, true);
    println!("after  braiding: codomain = {:?}", tensor.codomain());
}
