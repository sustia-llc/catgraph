//! Finite set morphism demonstration.
//!
//! Shows `FinSetMorphism` (Vec<usize>, usize) as composable maps,
//! OrderPresSurj/OrderPresInj for order-preserving surjections and injections,
//! Decomposition for epi-mono factorization, permutations, and the `from_cycle` helper.

use catgraph::category::{Composable, HasIdentity};
use catgraph::finset::{Decomposition, FinSetMorphism, OrderPresInj, OrderPresSurj, from_cycle};
use catgraph::monoidal::{Monoidal, SymmetricMonoidalDiscreteMorphism};
use permutations::Permutation;

// ============================================================================
// FinSetMorphism basics
// ============================================================================

fn finset_morphism_basics() {
    println!("=== FinSetMorphism Basics ===\n");

    // A morphism {0,1,2} -> {0,1,2,3}: maps 0->1, 1->3, 2->0
    // The second element is the "extra" codomain beyond the max image value.
    let f: FinSetMorphism = (vec![1, 3, 0], 0);
    println!("f = {f:?}");
    println!("  domain={}, codomain={}", f.domain(), f.codomain());

    // Identity on 4 elements
    let id: FinSetMorphism = FinSetMorphism::identity(&4);
    println!("id(4) = {id:?}");
    println!("  domain={}, codomain={}", id.domain(), id.codomain());

    // Composition: f ; id should equal f
    let f_id = f.compose(&id).unwrap();
    println!("f ; id = {f_id:?}");
    println!("  same as f = {}", f_id == f);

    // A surjection: {0,1,2} -> {0,1} via 0->0, 1->0, 2->1
    let surj: FinSetMorphism = (vec![0, 0, 1], 0);
    println!("\nsurj = {surj:?}");
    println!("  domain={}, codomain={}", surj.domain(), surj.codomain());

    // Compose surj with an injection: {0,1} -> {0,1,2} via 0->0, 1->2
    let inj: FinSetMorphism = (vec![0, 2], 0);
    let composed = surj.compose(&inj).unwrap();
    println!("surj ; inj = {composed:?}");
    println!(
        "  domain={}, codomain={}",
        composed.domain(),
        composed.codomain()
    );
    println!();
}

// ============================================================================
// Monoidal Structure on FinSetMorphism
// ============================================================================

fn finset_monoidal() {
    println!("=== FinSetMorphism Monoidal ===\n");

    let f: FinSetMorphism = (vec![1, 0], 0); // {0,1} -> {0,1}
    let g: FinSetMorphism = (vec![0], 0); // {0} -> {0}

    let mut tensor = f.clone();
    tensor.monoidal(g.clone());
    println!("f = {f:?}, g = {g:?}");
    println!("f ⊗ g = {tensor:?}");
    println!(
        "  domain={}, codomain={}",
        tensor.domain(),
        tensor.codomain()
    );
    println!();
}

// ============================================================================
// Order-Preserving Surjections
// ============================================================================

fn order_preserving_surjections() {
    println!("=== Order-Preserving Surjections ===\n");

    // Identity surjection on 3 elements: each fiber has cardinality 1
    let id_surj = OrderPresSurj::identity(&3);
    println!(
        "id(3): domain={}, codomain={}",
        id_surj.domain(),
        id_surj.codomain()
    );
    println!(
        "  preimage_cardinalities={:?}",
        id_surj.preimage_cardinalities()
    );

    // Surjection [0,0,1,1,1,2] -> [0,1,2] with fibers of size [2,3,1]
    // Represented as preimage_card_minus_1 = [1, 2, 0]
    let surj = OrderPresSurj::from([1, 2, 0]);
    println!(
        "surj: domain={}, codomain={}, preimage_cardinalities={:?}",
        surj.domain(),
        surj.codomain(),
        surj.preimage_cardinalities()
    );

    // Compose two surjections
    let s1 = OrderPresSurj::from([1, 0]); // domain=3, codomain=2
    let s2 = OrderPresSurj::from([1]); // domain=3, codomain=1
    let composed = s1.compose(&s2).unwrap();
    println!(
        "s1(dom={},cod={}) ; s2(dom={},cod={}) = composed(dom={},cod={})",
        s1.domain(),
        s1.codomain(),
        s2.domain(),
        s2.codomain(),
        composed.domain(),
        composed.codomain()
    );

    // Monoidal product
    let mut tensor = OrderPresSurj::from([0, 0]);
    tensor.monoidal(OrderPresSurj::from([1]));
    println!(
        "monoidal: domain={}, codomain={}, preimage_cardinalities={:?}",
        tensor.domain(),
        tensor.codomain(),
        tensor.preimage_cardinalities()
    );
    println!();
}

// ============================================================================
// Order-Preserving Injections
// ============================================================================

fn order_preserving_injections() {
    println!("=== Order-Preserving Injections ===\n");

    // Identity injection on 3 elements
    let id_inj = OrderPresInj::identity(&3);
    println!(
        "id(3): domain={}, codomain={}, iden_unit_counts={:?}",
        id_inj.domain(),
        id_inj.codomain(),
        id_inj.iden_unit_counts()
    );

    // Injection {0,1} -> {0,1,2,3}: maps 0->1, 1->2 (skip 0, gap at end)
    // From FinSetMorphism representation
    let inj_morphism: FinSetMorphism = (vec![1, 2], 1);
    let inj = OrderPresInj::try_from(inj_morphism);
    match &inj {
        Ok(i) => println!(
            "inj [1,2]+1: domain={}, codomain={}, counts={:?}",
            i.domain(),
            i.codomain(),
            i.iden_unit_counts()
        ),
        Err(e) => println!("error: {e}"),
    }

    // Non-order-preserving injection should fail
    let bad: FinSetMorphism = (vec![2, 0], 0);
    let bad_result = OrderPresInj::try_from(bad);
    println!("non-order-preserving: is_err={}", bad_result.is_err());
    println!();
}

// ============================================================================
// Permutations and from_cycle
// ============================================================================

fn permutations_demo() {
    println!("=== Permutations ===\n");

    // Identity permutation
    let id = Permutation::identity(4);
    println!("id(4): apply(0)={}, apply(3)={}", id.apply(0), id.apply(3));

    // Transposition: swap elements 1 and 3
    let swap = Permutation::transposition(4, 1, 3);
    println!(
        "swap(1,3): apply(0)={}, apply(1)={}, apply(2)={}, apply(3)={}",
        swap.apply(0),
        swap.apply(1),
        swap.apply(2),
        swap.apply(3)
    );

    // Build a permutation from a cycle: (0 2 3) in S_4
    let cyclic = from_cycle(4, &[0, 2, 3]);
    println!(
        "cycle(0,2,3): apply(0)={}, apply(1)={}, apply(2)={}, apply(3)={}",
        cyclic.apply(0),
        cyclic.apply(1),
        cyclic.apply(2),
        cyclic.apply(3)
    );

    // Inverse
    let p = Permutation::rotation_left(4, 1);
    let p_inv = p.inv();
    let composed = &p * &p_inv;
    let is_id = composed == Permutation::identity(4);
    println!("rotation * inv = identity: {is_id}");

    // Single-element cycle gives identity
    let trivial = from_cycle(3, &[1]);
    let trivial_is_id = trivial == Permutation::identity(3);
    println!("from_cycle(3, [1]) is identity: {trivial_is_id}");
    println!();
}

// ============================================================================
// Epi-Mono Factorization (Decomposition)
// ============================================================================

fn epi_mono_factorization() {
    println!("=== Epi-Mono Factorization ===\n");

    // Decompose a general morphism into: permutation ; surjection ; injection
    // f: {0,1,2,3} -> {0,1,2,3,4} given by 0->3, 1->1, 2->1, 3->4
    let f: FinSetMorphism = (vec![3, 1, 1, 4], 0);
    let decomp = Decomposition::try_from(f.clone());
    match decomp {
        Ok(d) => {
            let (perm, surj, inj) = d.get_parts();
            println!("f = {f:?}");
            println!("  domain={}, codomain={}", d.domain(), d.codomain());
            println!("  permutation part length: {}", perm.len());
            println!(
                "  surjection: preimage_cardinalities={:?}",
                surj.preimage_cardinalities()
            );
            println!("  injection: iden_unit_counts={:?}", inj.iden_unit_counts());
        }
        Err(e) => println!("decomposition error: {e}"),
    }

    // Identity decomposition
    let id_decomp = Decomposition::identity(&3);
    let (id_perm, id_surj, id_inj) = id_decomp.get_parts();
    println!(
        "\nid(3): perm=identity({}), surj_cards={:?}, inj_counts={:?}",
        id_perm.len(),
        id_surj.preimage_cardinalities(),
        id_inj.iden_unit_counts()
    );

    // Monoidal product of decompositions
    let d1 = Decomposition::identity(&2);
    let d2 = Decomposition::identity(&3);
    let mut tensor = d1;
    tensor.monoidal(d2);
    println!(
        "id(2) ⊗ id(3): domain={}, codomain={}",
        tensor.domain(),
        tensor.codomain()
    );

    // Composition of decompositions
    let da = Decomposition::identity(&3);
    let db = Decomposition::identity(&3);
    let composed = da.compose(&db).unwrap();
    println!(
        "id(3) ; id(3): domain={}, codomain={}",
        composed.domain(),
        composed.codomain()
    );

    // SymmetricMonoidalDiscreteMorphism: from_permutation
    let p = Permutation::transposition(3, 0, 2);
    let perm_decomp = Decomposition::from_permutation(p, 3, true);
    println!(
        "\nfrom_permutation swap(0,2): domain={}, codomain={}",
        perm_decomp.domain(),
        perm_decomp.codomain()
    );
    println!();
}

fn main() {
    finset_morphism_basics();
    finset_monoidal();
    order_preserving_surjections();
    order_preserving_injections();
    permutations_demo();
    epi_mono_factorization();
}
