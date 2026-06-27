//! Cospan API demonstration with category traits.
//!
//! Shows construction, identity morphisms, composition via pushout,
//! monoidal (tensor) product, permutation morphisms, graph conversion,
//! and how category traits (`HasIdentity`, Composable) work through Cospan.

use catgraph::category::{Composable, HasIdentity};
use catgraph::cospan::Cospan;
use catgraph::monoidal::{Monoidal, SymmetricMonoidalMorphism};
use either::Either::{Left, Right};
use permutations::Permutation;

// ============================================================================
// Construction and Accessors
// ============================================================================

fn construction() {
    println!("=== Construction and Accessors ===\n");

    // A cospan with 2 left (domain) nodes, 1 right (codomain) node, 3 middle nodes.
    // left[0] -> middle[0], left[1] -> middle[1]
    // right[0] -> middle[2]
    let c = Cospan::new(vec![0, 1], vec![2], vec!['a', 'b', 'c']);
    println!("left_to_middle   = {:?}", c.left_to_middle());
    println!("right_to_middle  = {:?}", c.right_to_middle());
    println!("middle           = {:?}", c.middle());
    println!("is_left_identity = {}", c.is_left_identity());
    println!("is_right_identity= {}", c.is_right_identity());

    // Empty cospan
    let empty: Cospan<char> = Cospan::empty();
    println!("\nempty cospan: is_empty = {}", empty.is_empty());

    // Both domain nodes map to the same middle node (wire merging)
    let merged = Cospan::new(vec![0, 0], vec![0], vec!['x']);
    println!(
        "\nmerged: left_to_middle = {:?}, middle = {:?}",
        merged.left_to_middle(),
        merged.middle()
    );
    println!();
}

// ============================================================================
// Category Traits: HasIdentity and Composable
// ============================================================================

fn category_traits() {
    println!("=== Category Traits ===\n");

    // HasIdentity: identity morphism on a type vector
    let types = vec!['a', 'b', 'c'];
    let id = Cospan::identity(&types);
    println!("identity on ['a','b','c']:");
    println!("  left_to_middle  = {:?}", id.left_to_middle());
    println!("  right_to_middle = {:?}", id.right_to_middle());
    println!("  middle          = {:?}", id.middle());
    println!("  is_left_identity  = {}", id.is_left_identity());
    println!("  is_right_identity = {}", id.is_right_identity());

    // Composable: domain and codomain
    println!("\n  domain   = {:?}", id.domain());
    println!("  codomain = {:?}", id.codomain());

    // composable check
    let f = Cospan::new(vec![0, 1], vec![0, 1], vec!['a', 'b']);
    let g = Cospan::new(vec![0, 1], vec![0], vec!['a', 'b']);
    println!(
        "\nf.domain = {:?}, f.codomain = {:?}",
        f.domain(),
        f.codomain()
    );
    println!(
        "g.domain = {:?}, g.codomain = {:?}",
        g.domain(),
        g.codomain()
    );
    println!("f.composable(&g) = {:?}", f.composable(&g));
    println!();
}

// ============================================================================
// Composition (Pushout)
// ============================================================================

fn composition() {
    println!("=== Composition (Pushout) ===\n");

    // f: {a,b} -> {a,b}  (identity-like)
    // g: {a,b} -> {a}    (merge two into one)
    let f = Cospan::new(vec![0, 1], vec![0, 1], vec!['a', 'b']);
    let g = Cospan::new(vec![0, 0], vec![0], vec!['a']);

    // f.codomain() = ['a','b'], g.domain() = ['a','a'] — mismatch!
    println!(
        "f.codomain = {:?}, g.domain = {:?}",
        f.codomain(),
        g.domain()
    );
    println!("f.compose(&g) = {:?}", f.compose(&g));

    // Composable pair: matching interface
    let h = Cospan::new(vec![0, 1], vec![0, 1], vec!['a', 'b']);
    let k = Cospan::new(vec![0, 1], vec![0], vec!['a', 'b']);
    println!(
        "\nh.codomain = {:?}, k.domain = {:?}",
        h.codomain(),
        k.domain()
    );
    let composed = h.compose(&k).unwrap();
    println!("h.compose(&k):");
    println!("  left_to_middle  = {:?}", composed.left_to_middle());
    println!("  right_to_middle = {:?}", composed.right_to_middle());
    println!("  middle          = {:?}", composed.middle());

    // Identity law: id.compose(&f) == f (structurally)
    let types = vec!['a', 'b'];
    let id = Cospan::identity(&types);
    let id_then_k = id.compose(&k).unwrap();
    println!("\nid.compose(&k):");
    println!("  left_to_middle  = {:?}", id_then_k.left_to_middle());
    println!("  right_to_middle = {:?}", id_then_k.right_to_middle());
    println!("  middle          = {:?}", id_then_k.middle());
    println!();
}

// ============================================================================
// Monoidal Product (Tensor)
// ============================================================================

fn monoidal_product() {
    println!("=== Monoidal Product (Tensor) ===\n");

    let mut a = Cospan::new(vec![0], vec![0], vec!['x']);
    let b = Cospan::new(vec![0], vec![0], vec!['y']);

    println!("a: middle = {:?}, domain = {:?}", a.middle(), a.domain());
    println!("b: middle = {:?}, domain = {:?}", b.middle(), b.domain());

    a.monoidal(b);
    println!("\nafter a.monoidal(b):");
    println!("  left_to_middle  = {:?}", a.left_to_middle());
    println!("  right_to_middle = {:?}", a.right_to_middle());
    println!("  middle          = {:?}", a.middle());
    println!("  domain          = {:?}", a.domain());
    println!("  codomain        = {:?}", a.codomain());
    println!();
}

// ============================================================================
// Permutation Morphisms
// ============================================================================

fn permutation_morphisms() {
    println!("=== Permutation Morphisms ===\n");

    // Swap permutation on 2 elements: (0 1) -> (1 0)
    let swap = Permutation::rotation_left(2, 1);
    let types = ['a', 'b'];

    // types_as_on_domain = true: domain has given order, codomain is permuted
    let perm_cospan = Cospan::from_permutation(swap, &types, true).unwrap();
    println!("swap permutation (types on domain):");
    println!("  domain   = {:?}", perm_cospan.domain());
    println!("  codomain = {:?}", perm_cospan.codomain());
    println!("  left_to_middle  = {:?}", perm_cospan.left_to_middle());
    println!("  right_to_middle = {:?}", perm_cospan.right_to_middle());
    println!("  is_left_identity  = {}", perm_cospan.is_left_identity());
    println!("  is_right_identity = {}", perm_cospan.is_right_identity());
    println!();
}

// ============================================================================
// Mutation: Add/Delete/Connect Boundary Nodes
// ============================================================================

fn mutation() {
    println!("=== Mutation ===\n");

    let mut c = Cospan::new(vec![0, 1], vec![1], vec!['a', 'b']);
    println!(
        "initial: left = {:?}, right = {:?}, middle = {:?}",
        c.left_to_middle(),
        c.right_to_middle(),
        c.middle()
    );

    // Add a left boundary node pointing to existing middle[0]
    let idx = c.add_boundary_node_known_target(Left(0));
    println!("\nadd_boundary_node_known_target(Left(0)) -> {idx:?}");
    println!("  left = {:?}", c.left_to_middle());

    // Add a right boundary node with a new middle node labeled 'c'
    let idx = c.add_boundary_node_unknown_target(Right('c'));
    println!("\nadd_boundary_node_unknown_target(Right('c')) -> {idx:?}");
    println!("  right  = {:?}", c.right_to_middle());
    println!("  middle = {:?}", c.middle());

    // Connect two boundary nodes (merge their middle targets)
    println!(
        "\nmap_to_same(Left(0), Left(2)) = {}",
        c.map_to_same(Left(0), Left(2))
    );
    c.connect_pair(Left(0), Right(0));
    println!("after connect_pair(Left(0), Right(0)):");
    println!("  left   = {:?}", c.left_to_middle());
    println!("  right  = {:?}", c.right_to_middle());
    println!("  middle = {:?}", c.middle());

    // Delete a boundary node
    c.delete_boundary_node(Left(0));
    println!("\nafter delete_boundary_node(Left(0)):");
    println!("  left = {:?}", c.left_to_middle());
    println!();
}

// ============================================================================
// Map
// ============================================================================

fn map_demo() {
    println!("=== Map ===\n");

    let c = Cospan::new(vec![0, 1], vec![0], vec!['a', 'b']);

    // Map lambda labels
    let mapped = c.map(|ch| ch.to_ascii_uppercase());
    println!("original middle  = {:?}", c.middle());
    println!("mapped middle    = {:?}", mapped.middle());
    println!();
}

fn main() {
    construction();
    category_traits();
    composition();
    monoidal_product();
    permutation_morphisms();
    mutation();
    map_demo();
}
