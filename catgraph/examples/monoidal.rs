//! Monoidal and `SymmetricMonoidalMorphism` trait demonstration.
//!
//! Shows tensor product of cospans, braiding via permutations,
//! `permute_side`, associativity of tensor, and `GenericMonoidalMorphism`
//! composition with identity checks.

use catgraph::category::ComposableMutating;
use catgraph::category::{Composable, HasIdentity};
use catgraph::cospan::Cospan;
use catgraph::monoidal::{
    GenericMonoidalMorphism, GenericMonoidalMorphismLayer, Monoidal, SymmetricMonoidalMorphism,
};
use permutations::Permutation;

// ============================================================================
// Tensor Product (Monoidal)
// ============================================================================

fn tensor_product() {
    println!("=== Tensor Product (Monoidal) ===\n");

    // Two simple cospans: f: [a] -> [b] and g: [c] -> [d]
    let f = Cospan::new(vec![0], vec![1], vec!['a', 'b']);
    let g = Cospan::new(vec![0], vec![1], vec!['c', 'd']);

    println!("f: domain={:?}, codomain={:?}", f.domain(), f.codomain());
    println!("g: domain={:?}, codomain={:?}", g.domain(), g.codomain());

    // Tensor product: f ⊗ g
    let mut fg = f.clone();
    fg.monoidal(g.clone());
    println!(
        "f ⊗ g: domain={:?}, codomain={:?}",
        fg.domain(),
        fg.codomain()
    );

    // Identity is monoidal unit (identity on empty type)
    let id_empty: Cospan<char> = Cospan::identity(&vec![]);
    let mut f_tensored = f.clone();
    f_tensored.monoidal(id_empty);
    println!(
        "f ⊗ id_empty: domain={:?}, codomain={:?} (same as f)",
        f_tensored.domain(),
        f_tensored.codomain()
    );

    // Tensor of identities
    let id_ab = Cospan::identity(&vec!['a', 'b']);
    let id_cd = Cospan::identity(&vec!['c', 'd']);
    let mut id_tensor = id_ab;
    id_tensor.monoidal(id_cd);
    println!(
        "id_ab ⊗ id_cd: domain={:?}, codomain={:?}",
        id_tensor.domain(),
        id_tensor.codomain()
    );
    println!();
}

// ============================================================================
// Braiding via Permutations
// ============================================================================

fn braiding() {
    println!("=== Braiding via Permutations ===\n");

    let types = ['a', 'b', 'c'];

    // Transposition: swap first two elements
    let swap_01 = Permutation::transposition(3, 0, 1);
    let braiding = Cospan::from_permutation(swap_01, &types, true).unwrap();
    println!(
        "swap(0,1): domain={:?}, codomain={:?}",
        braiding.domain(),
        braiding.codomain()
    );

    // Cyclic rotation: (0 1 2) -> (1 2 0)
    let rotation = Permutation::rotation_left(3, 1);
    let rot_cospan = Cospan::from_permutation(rotation, &types, true).unwrap();
    println!(
        "rotate_left(1): domain={:?}, codomain={:?}",
        rot_cospan.domain(),
        rot_cospan.codomain()
    );

    // Identity permutation gives identity cospan
    let id_perm = Permutation::identity(3);
    let id_cospan = Cospan::from_permutation(id_perm, &types, true).unwrap();
    println!(
        "identity perm: domain={:?}, codomain={:?}",
        id_cospan.domain(),
        id_cospan.codomain()
    );

    // Composing a permutation with its inverse yields identity-like result
    let p = Permutation::transposition(3, 0, 2);
    let cospan_p = Cospan::from_permutation(p.clone(), &types, true).unwrap();
    let cospan_p_inv = Cospan::from_permutation(p.inv(), &types, false).unwrap();
    let composed = cospan_p.compose(&cospan_p_inv).unwrap();
    println!(
        "p ; p_inv: domain={:?}, codomain={:?}",
        composed.domain(),
        composed.codomain()
    );
    println!();
}

// ============================================================================
// Permute Side
// ============================================================================

fn permute_side() {
    println!("=== Permute Side ===\n");

    let types = vec!['a', 'b', 'c'];
    let mut cospan = Cospan::identity(&types);
    println!(
        "before permute: domain={:?}, codomain={:?}",
        cospan.domain(),
        cospan.codomain()
    );

    // Permute the codomain side
    let swap = Permutation::transposition(3, 0, 2);
    cospan.permute_side(&swap, true);
    println!(
        "after permute codomain swap(0,2): domain={:?}, codomain={:?}",
        cospan.domain(),
        cospan.codomain()
    );

    // Permute the domain side of a fresh cospan
    let mut cospan2 = Cospan::identity(&types);
    let rot = Permutation::rotation_left(3, 1);
    cospan2.permute_side(&rot, false);
    println!(
        "after permute domain rotate_left(1): domain={:?}, codomain={:?}",
        cospan2.domain(),
        cospan2.codomain()
    );
    println!();
}

// ============================================================================
// Tensor Associativity
// ============================================================================

fn tensor_associativity() {
    println!("=== Tensor Associativity ===\n");

    let a = Cospan::new(vec![0], vec![0], vec!['a']);
    let b = Cospan::new(vec![0], vec![0], vec!['b']);
    let c = Cospan::new(vec![0], vec![0], vec!['c']);

    // (a ⊗ b) ⊗ c
    let mut ab = a.clone();
    ab.monoidal(b.clone());
    let mut ab_c = ab;
    ab_c.monoidal(c.clone());

    // a ⊗ (b ⊗ c)
    let mut bc = b;
    bc.monoidal(c);
    let mut a_bc = a;
    a_bc.monoidal(bc);

    println!(
        "(a ⊗ b) ⊗ c: domain={:?}, codomain={:?}",
        ab_c.domain(),
        ab_c.codomain()
    );
    println!(
        "a ⊗ (b ⊗ c): domain={:?}, codomain={:?}",
        a_bc.domain(),
        a_bc.codomain()
    );

    // Domains and codomains match (strict associativity)
    println!("domains equal  = {}", ab_c.domain() == a_bc.domain());
    println!("codomains equal = {}", ab_c.codomain() == a_bc.codomain());
    println!();
}

// ============================================================================
// GenericMonoidalMorphism
// ============================================================================

#[derive(Clone, PartialEq, Eq, Debug)]
struct Wire {
    wire_type: char,
}

impl HasIdentity<char> for Wire {
    fn identity(on_this: &char) -> Self {
        Wire {
            wire_type: *on_this,
        }
    }
}

fn generic_monoidal_morphism() {
    println!("=== GenericMonoidalMorphism ===\n");

    // Identity morphism on [a, b]
    let types = vec!['a', 'b'];
    let id: GenericMonoidalMorphism<Wire, char> = GenericMonoidalMorphism::identity(&types);
    println!(
        "identity depth={}, domain={:?}, codomain={:?}",
        id.depth(),
        id.domain(),
        id.codomain()
    );

    // Build a two-layer morphism via composition
    let mut composed: GenericMonoidalMorphism<Wire, char> =
        GenericMonoidalMorphism::identity(&types);
    let layer2: GenericMonoidalMorphism<Wire, char> = GenericMonoidalMorphism::identity(&types);
    let result = composed.compose(layer2);
    println!(
        "composed depth={}, composable={}",
        composed.depth(),
        result.is_ok()
    );

    // Monoidal product of two morphisms
    let m1: GenericMonoidalMorphism<Wire, char> = GenericMonoidalMorphism::identity(&vec!['a']);
    let m2: GenericMonoidalMorphism<Wire, char> = GenericMonoidalMorphism::identity(&vec!['b']);
    let mut tensored = m1;
    tensored.monoidal(m2);
    println!(
        "tensored domain={:?}, codomain={:?}",
        tensored.domain(),
        tensored.codomain()
    );

    // Extract layers
    let morphism: GenericMonoidalMorphism<Wire, char> = GenericMonoidalMorphism::identity(&types);
    let layers = morphism.extract_layers();
    println!("extracted {} layer(s)", layers.len());

    // Layer-level operations
    let layer: GenericMonoidalMorphismLayer<Wire, char> =
        GenericMonoidalMorphismLayer::identity(&types);
    println!(
        "layer: {} block(s), left_type={:?}, right_type={:?}",
        layer.blocks.len(),
        layer.left_type,
        layer.right_type
    );
    println!();
}

fn main() {
    tensor_product();
    braiding();
    permute_side();
    tensor_associativity();
    generic_monoidal_morphism();
}
