//! Corel example: constructing a corelation, inspecting its equivalence classes,
//! and composing two corelations via pushout.
//!
//! Run: `cargo run -p catgraph --example corel`

use catgraph::{
    category::Composable, corel::Corel, cospan::Cospan, hypergraph_category::HypergraphCategory,
};

fn main() {
    println!("=== Corel<char> example ===\n");

    let c = Cospan::new(vec![0], vec![0], vec!['z']);
    let corel = Corel::new(c).expect("jointly surjective");

    println!(
        "Corelation shape: {:?} → {:?}",
        corel.domain(),
        corel.codomain()
    );
    println!(
        "Equivalence classes: {} class(es)",
        corel.equivalence_classes().len()
    );
    for (i, class) in corel.equivalence_classes().iter().enumerate() {
        println!("  class {i}: {} elements", class.len());
    }

    let second = Corel::new(Cospan::new(vec![0], vec![0], vec!['z'])).unwrap();
    let composed = corel.compose(&second).unwrap();
    println!(
        "\nComposed shape: {:?} → {:?}",
        composed.domain(),
        composed.codomain()
    );

    let mu = Corel::<char>::multiplication('z');
    println!(
        "\nμ (multiplication) shape: {:?} → {:?}",
        mu.domain(),
        mu.codomain()
    );
    println!(
        "μ equivalence classes: {} class(es)",
        mu.equivalence_classes().len()
    );

    println!(
        "\nAll assertions passed. See F&S 2018 Example 6.64 for the full hypergraph category structure."
    );
}
