//! `CospanAlgebra` API demonstration (Fong-Spivak §2.1).
//!
//! Shows `PartitionAlgebra` and `NameAlgebra` as lax monoidal functors
//! from `Cospan_Λ` to a target category.

use catgraph::{
    category::{Composable, ComposableMutating, HasIdentity},
    compact_closed::cup_single,
    cospan::Cospan,
    cospan_algebra::{CospanAlgebra, NameAlgebra, PartitionAlgebra, cospan_to_frobenius},
    frobenius::{FrobeniusMorphism, FrobeniusOperation},
};

type FM = FrobeniusMorphism<char, String>;

fn main() {
    println!("=== PartitionAlgebra (Example 2.3) ===\n");
    println!("Part(x) = Cospan([], x) — partitions of x into labeled groups.\n");

    let part = PartitionAlgebra;

    // Unit: the distinguished element of Part([]) = Cospan([], [])
    let u: Cospan<char> = part.unit();
    println!(
        "Unit:     domain={:?}, codomain={:?}",
        u.domain(),
        u.codomain()
    );

    // An element of Part([a, b]): a partition [] → [a, b]
    let element = Cospan::new(vec![], vec![0, 1], vec!['a', 'b']);
    println!(
        "Element:  domain={:?}, codomain={:?}, middle={:?}",
        element.domain(),
        element.codomain(),
        element.middle()
    );

    // Functorial action: map through an identity cospan preserves the element
    let id_ab = Cospan::<char>::identity(&vec!['a', 'b']);
    let mapped = part.map_cospan(&id_ab, &element).unwrap();
    println!(
        "map(id):  domain={:?}, codomain={:?}  (preserved)",
        mapped.domain(),
        mapped.codomain()
    );

    // Functorial action: map through a merge cospan
    // Merge: [a, a] → [a] (both domain nodes map to same middle)
    let merge_elem = Cospan::new(vec![], vec![0, 0], vec!['a', 'a']);
    let merge = Cospan::new(vec![0, 0], vec![0], vec!['a']);
    let merged = part.map_cospan(&merge, &merge_elem).unwrap();
    println!(
        "map(μ):   domain={:?}, codomain={:?}  (merged to single group)",
        merged.domain(),
        merged.codomain()
    );

    // Lax monoidal: combine two elements via tensor product
    let a_elem = Cospan::new(vec![], vec![0], vec!['a']);
    let b_elem = Cospan::new(vec![], vec![0], vec!['b']);
    let combined = part.lax_monoidal(&a_elem, &b_elem);
    println!(
        "a ⊕ b:    domain={:?}, codomain={:?}  (monoidal product)",
        combined.domain(),
        combined.codomain()
    );

    // Functoriality check: map(c1 ; c2) = map(c1) then map(c2)
    let c1 = Cospan::<char>::identity(&vec!['a']);
    let c2 = Cospan::<char>::identity(&vec!['a']);
    let composed = c1.compose(&c2).unwrap();
    let elem_a = Cospan::new(vec![], vec![0], vec!['a']);
    let via_composed = part.map_cospan(&composed, &elem_a).unwrap();
    let via_sequential = {
        let step1 = part.map_cospan(&c1, &elem_a).unwrap();
        part.map_cospan(&c2, &step1).unwrap()
    };
    println!(
        "\nFunctoriality: map(c1;c2) codomain={:?}, map(c2)(map(c1)) codomain={:?}",
        via_composed.codomain(),
        via_sequential.codomain()
    );

    println!("\n=== NameAlgebra (Prop 3.2) ===\n");
    println!("A_H(x) = H(I, P(x)) — named morphisms via compact closed structure.\n");

    let name_alg = NameAlgebra::<String>::new();

    // Unit: identity on []
    let nu: FM = name_alg.unit();
    println!(
        "Unit:     domain={:?}, codomain={:?}",
        nu.domain(),
        nu.codomain()
    );

    // An element of A_H([a]): a name [] → [a] (using the Frobenius unit η)
    let eta: FM = FrobeniusOperation::Unit('a').into();
    println!(
        "η('a'):   domain={:?}, codomain={:?}",
        eta.domain(),
        eta.codomain()
    );

    // Functorial action through identity preserves the element
    let id_a = Cospan::<char>::identity(&vec!['a']);
    let mapped_eta = name_alg.map_cospan(&id_a, &eta).unwrap();
    println!(
        "map(id):  domain={:?}, codomain={:?}  (preserved)",
        mapped_eta.domain(),
        mapped_eta.codomain()
    );

    // Map a cup [] → [a,a] through a merge cospan [a,a] → [a]
    let cup_a: FM = cup_single('a');
    let merge_a = Cospan::new(vec![0, 0], vec![0], vec!['a']);
    let mapped_cup = name_alg.map_cospan(&merge_a, &cup_a).unwrap();
    println!(
        "map(μ, cup): domain={:?}, codomain={:?}  (cup composed with merge)",
        mapped_cup.domain(),
        mapped_cup.codomain()
    );

    // Lax monoidal: tensor product of named morphisms
    let eta_b: FM = FrobeniusOperation::Unit('b').into();
    let tensor = name_alg.lax_monoidal(&eta, &eta_b);
    println!(
        "η(a)⊕η(b): domain={:?}, codomain={:?}  (monoidal product)",
        tensor.domain(),
        tensor.codomain()
    );

    println!("\n=== cospan_to_frobenius (Lemma 3.6 / Prop 3.8) ===\n");
    println!("Decomposes cospans into Frobenius generators via epi-mono factorization.\n");

    // Identity cospan → identity morphism
    let id_cospan = Cospan::<char>::identity(&vec!['a', 'b']);
    let id_morph: FM = cospan_to_frobenius(&id_cospan).unwrap();
    println!(
        "id([a,b]):  {:?} → {:?}",
        id_morph.domain(),
        id_morph.codomain()
    );

    // Merge cospan → multiplication generator
    let merge_cospan = Cospan::new(vec![0, 0], vec![0], vec!['a']);
    let merge_morph: FM = cospan_to_frobenius(&merge_cospan).unwrap();
    println!(
        "merge:      {:?} → {:?}  (multiplication)",
        merge_morph.domain(),
        merge_morph.codomain()
    );

    // Split cospan → comultiplication generator
    let split_cospan = Cospan::new(vec![0], vec![0, 0], vec!['a']);
    let split_morph: FM = cospan_to_frobenius(&split_cospan).unwrap();
    println!(
        "split:      {:?} → {:?}  (comultiplication)",
        split_morph.domain(),
        split_morph.codomain()
    );

    // Creation cospan → unit generator
    let create_cospan = Cospan::new(vec![], vec![0], vec!['a']);
    let create_morph: FM = cospan_to_frobenius(&create_cospan).unwrap();
    println!(
        "create:     {:?} → {:?}  (unit η)",
        create_morph.domain(),
        create_morph.codomain()
    );

    // Destruction cospan → counit generator
    let destroy_cospan = Cospan::new(vec![0], vec![], vec!['a']);
    let destroy_morph: FM = cospan_to_frobenius(&destroy_cospan).unwrap();
    println!(
        "destroy:    {:?} → {:?}  (counit ε)",
        destroy_morph.domain(),
        destroy_morph.codomain()
    );
}
