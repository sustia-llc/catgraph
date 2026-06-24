//! Span and Rel (relation algebra) API demonstration.
//!
//! Shows Span construction, identity, composition (pullback), dagger,
//! monoidal product, and the Rel wrapper with reflexive/symmetric/
//! transitive/antisymmetric checks, subsumption, union, intersection,
//! complement, equivalence relations, and partial orders.

use catgraph::category::{Composable, HasIdentity};
use catgraph::monoidal::Monoidal;
use catgraph::span::{Rel, Span};

// ============================================================================
// Span Construction and Accessors
// ============================================================================

fn construction() {
    println!("=== Span Construction and Accessors ===\n");

    // Span<Lambda>: left = domain labels, right = codomain labels,
    // middle = pairs (left_index, right_index) representing the source set.
    // Each pair (i, j) requires left[i] == right[j] (types must match).
    let left = vec!['a', 'b', 'a'];
    let right = vec!['a', 'b'];
    let middle = vec![(0, 0), (1, 1), (2, 0)];
    let s = Span::new(left, right, middle);

    println!("left (domain)    = {:?}", s.left());
    println!("right (codomain) = {:?}", s.right());
    println!("middle_pairs     = {:?}", s.middle_pairs());
    println!("middle_to_left   = {:?}", s.middle_to_left());
    println!("middle_to_right  = {:?}", s.middle_to_right());
    println!("is_left_identity = {}", s.is_left_identity());
    println!("is_right_identity= {}", s.is_right_identity());
    println!("is_jointly_injective = {}", s.is_jointly_injective());
    println!();
}

// ============================================================================
// Span Identity
// ============================================================================

fn identity() {
    println!("=== Span Identity ===\n");

    let types = vec!['x', 'y', 'z'];
    let id = Span::identity(&types);

    println!("identity on ['x','y','z']:");
    println!("  domain          = {:?}", id.domain());
    println!("  codomain        = {:?}", id.codomain());
    println!("  middle_pairs    = {:?}", id.middle_pairs());
    println!("  is_left_identity  = {}", id.is_left_identity());
    println!("  is_right_identity = {}", id.is_right_identity());
    println!("  jointly_injective = {}", id.is_jointly_injective());
    println!();
}

// ============================================================================
// Composition (Pullback)
// ============================================================================

fn composition() {
    println!("=== Composition (Pullback) ===\n");

    // f: {a,b} -> {a,b} with identity-like middle
    let f = Span::new(vec!['a', 'b'], vec!['a', 'b'], vec![(0, 0), (1, 1)]);
    // g: {a,b} -> {a,b} with middle mapping both source elements to index 0
    // Pair (0,0): left[0]='a' == right[0]='a', pair (1,1): left[1]='b' == right[1]='b'
    let g = Span::new(vec!['a', 'b'], vec!['a', 'b'], vec![(0, 0), (1, 1)]);

    println!(
        "f.domain = {:?}, f.codomain = {:?}",
        f.domain(),
        f.codomain()
    );
    println!(
        "g.domain = {:?}, g.codomain = {:?}",
        g.domain(),
        g.codomain()
    );
    println!("f.composable(&g) = {:?}", f.composable(&g));

    let fg = f.compose(&g).unwrap();
    println!("\nf.compose(&g):");
    println!("  domain       = {:?}", fg.domain());
    println!("  codomain     = {:?}", fg.codomain());
    println!("  middle_pairs = {:?}", fg.middle_pairs());

    // Identity law: id.compose(&g) should give same result as g
    let id = Span::identity(&vec!['a', 'b']);
    let id_g = id.compose(&g).unwrap();
    println!("\nid.compose(&g):");
    println!("  domain       = {:?}", id_g.domain());
    println!("  codomain     = {:?}", id_g.codomain());
    println!("  middle_pairs = {:?}", id_g.middle_pairs());
    println!();
}

// ============================================================================
// Dagger (Transpose)
// ============================================================================

fn dagger() {
    println!("=== Dagger (Transpose) ===\n");

    // Pairs (0,1) and (1,0): left[0]='a'=right[1], left[1]='b'=right[0]
    let s = Span::new(vec!['a', 'b'], vec!['b', 'a'], vec![(0, 1), (1, 0)]);
    let d = s.dagger();

    println!(
        "original: domain = {:?}, codomain = {:?}, pairs = {:?}",
        s.domain(),
        s.codomain(),
        s.middle_pairs()
    );
    println!(
        "dagger:   domain = {:?}, codomain = {:?}, pairs = {:?}",
        d.domain(),
        d.codomain(),
        d.middle_pairs()
    );
    println!();
}

// ============================================================================
// Monoidal Product (Tensor)
// ============================================================================

fn monoidal_product() {
    println!("=== Monoidal Product (Tensor) ===\n");

    let mut a = Span::new(vec!['a'], vec!['a'], vec![(0, 0)]);
    let b = Span::new(vec!['b'], vec!['b'], vec![(0, 0)]);

    println!(
        "a: domain = {:?}, codomain = {:?}",
        a.domain(),
        a.codomain()
    );
    println!(
        "b: domain = {:?}, codomain = {:?}",
        b.domain(),
        b.codomain()
    );

    a.monoidal(b);
    println!("\nafter a.monoidal(b):");
    println!("  domain       = {:?}", a.domain());
    println!("  codomain     = {:?}", a.codomain());
    println!("  middle_pairs = {:?}", a.middle_pairs());
    println!();
}

// ============================================================================
// Map
// ============================================================================

fn map_labels() {
    println!("=== Map Labels ===\n");

    let s = Span::new(vec!['a', 'b'], vec!['a', 'b'], vec![(0, 0), (1, 1)]);
    let mapped = s.map(|ch| ch.to_ascii_uppercase());

    println!("original: left = {:?}, right = {:?}", s.left(), s.right());
    println!(
        "mapped:   left = {:?}, right = {:?}",
        mapped.left(),
        mapped.right()
    );
    println!();
}

// ============================================================================
// Rel: Construction and Properties
// ============================================================================

fn rel_construction() {
    println!("=== Rel Construction ===\n");

    // A jointly-injective span can become a Rel (all types must match across pairs)
    let span = Span::new(vec!['a', 'a'], vec!['a', 'a'], vec![(0, 0), (1, 1)]);
    println!(
        "span.is_jointly_injective = {}",
        span.is_jointly_injective()
    );

    let rel = Rel::new(span).unwrap();
    println!("Rel created successfully");
    println!("  domain   = {:?}", rel.as_span().left());
    println!("  codomain = {:?}", rel.as_span().right());

    // A non-injective span (duplicate pairs) cannot be a Rel
    let non_inj = Span::new(vec!['a', 'a'], vec!['a', 'a'], vec![(0, 0), (0, 0)]);
    println!(
        "\nnon-injective span: is_jointly_injective = {}",
        non_inj.is_jointly_injective()
    );
    println!("Rel::new result = {:?}", Rel::new(non_inj).err());
    println!();
}

// ============================================================================
// Rel: Equivalence Relation
// ============================================================================

fn rel_equivalence() {
    println!("=== Rel: Equivalence Relation ===\n");

    // Full equivalence on {a,a}: every pair present (universal relation)
    // All types are the same so any pair is valid.
    let universal = Rel::new_unchecked(Span::new(
        vec!['a', 'a'],
        vec!['a', 'a'],
        vec![(0, 0), (0, 1), (1, 0), (1, 1)],
    ));
    println!("universal relation on 2 elements:");
    println!("  is_reflexive   = {}", universal.is_reflexive());
    println!("  is_symmetric   = {}", universal.is_symmetric());
    println!("  is_transitive  = {}", universal.is_transitive());
    println!("  is_equivalence = {}", universal.is_equivalence_rel());

    // Identity relation: only diagonal pairs
    let identity_rel = Rel::identity(&vec!['a', 'a', 'a']);
    println!("\nidentity relation on 3 elements:");
    println!("  is_reflexive      = {}", identity_rel.is_reflexive());
    println!("  is_symmetric      = {}", identity_rel.is_symmetric());
    println!("  is_antisymmetric  = {}", identity_rel.is_antisymmetric());
    println!(
        "  is_equivalence    = {}",
        identity_rel.is_equivalence_rel()
    );
    println!("  is_partial_order  = {}", identity_rel.is_partial_order());
    println!();
}

// ============================================================================
// Rel: Partial Order
// ============================================================================

fn rel_partial_order() {
    println!("=== Rel: Partial Order ===\n");

    // <= on 3 elements represented as: (0,0),(0,1),(0,2),(1,1),(1,2),(2,2)
    // All elements have the same type so every pair is valid.
    let leq = Rel::new_unchecked(Span::new(
        vec!['a', 'a', 'a'],
        vec!['a', 'a', 'a'],
        vec![(0, 0), (0, 1), (0, 2), (1, 1), (1, 2), (2, 2)],
    ));
    println!("leq relation on 3 elements (total order):");
    println!("  is_reflexive     = {}", leq.is_reflexive());
    println!("  is_antisymmetric = {}", leq.is_antisymmetric());
    println!("  is_transitive    = {}", leq.is_transitive());
    println!("  is_partial_order = {}", leq.is_partial_order());
    println!("  is_equivalence   = {}", leq.is_equivalence_rel());
    println!();
}

// ============================================================================
// Rel: Set Operations
// ============================================================================

fn rel_set_operations() {
    println!("=== Rel: Set Operations ===\n");

    // Uniform types allow any (i, j) pair
    let r1 = Rel::new_unchecked(Span::new(
        vec!['a', 'a'],
        vec!['a', 'a'],
        vec![(0, 0), (0, 1)],
    ));
    let r2 = Rel::new_unchecked(Span::new(
        vec!['a', 'a'],
        vec!['a', 'a'],
        vec![(0, 0), (1, 1)],
    ));

    println!("r1 pairs = {:?}", r1.as_span().middle_pairs());
    println!("r2 pairs = {:?}", r2.as_span().middle_pairs());

    // Subsumption
    println!("\nr1.subsumes(&r2) = {:?}", r1.subsumes(&r2));
    println!("r2.subsumes(&r1) = {:?}", r2.subsumes(&r1));

    // Union
    let u = r1.union(&r2).unwrap();
    println!("\nunion pairs = {:?}", u.as_span().middle_pairs());

    // Intersection
    let i = r1.intersection(&r2).unwrap();
    println!("intersection pairs = {:?}", i.as_span().middle_pairs());

    // Complement
    let comp = r1.complement().unwrap();
    println!("complement of r1 = {:?}", comp.as_span().middle_pairs());
    println!();
}

// ============================================================================
// Rel: Composition
// ============================================================================

fn rel_composition() {
    println!("=== Rel: Composition ===\n");

    // Compose two relations: relational composition (exists z. (x,z) in R and (z,y) in S)
    let r = Rel::new_unchecked(Span::new(
        vec!['a', 'a'],
        vec!['a', 'a'],
        vec![(0, 0), (0, 1)],
    ));
    let s = Rel::new_unchecked(Span::new(
        vec!['a', 'a'],
        vec!['a', 'a'],
        vec![(0, 0), (1, 0)],
    ));

    println!("r pairs = {:?}", r.as_span().middle_pairs());
    println!("s pairs = {:?}", s.as_span().middle_pairs());

    let rs = r.compose(&s).unwrap();
    println!("r.compose(&s) pairs = {:?}", rs.as_span().middle_pairs());
    println!();
}

fn main() {
    construction();
    identity();
    composition();
    dagger();
    monoidal_product();
    map_labels();
    rel_construction();
    rel_equivalence();
    rel_partial_order();
    rel_set_operations();
    rel_composition();
}
