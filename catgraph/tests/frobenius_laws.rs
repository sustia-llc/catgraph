//! Integration tests for Frobenius algebra axioms using only the public API.
//!
//! Tests cover identity laws, braiding involution, spider fusion, unit-counit
//! scalar loops, monoidal products, composition associativity, permutation
//! morphisms, and the `from_decomposition` roundtrip.

use catgraph::{
    category::{ComposableMutating, HasIdentity},
    finset::Decomposition,
    frobenius::{
        FrobeniusMorphism, FrobeniusOperation, from_decomposition, special_frobenius_morphism,
    },
    monoidal::Monoidal,
};

// ---------------------------------------------------------------------------
// Type aliases for readability
// ---------------------------------------------------------------------------

/// Morphisms with char-labelled wires and String black-box labels.
type FM = FrobeniusMorphism<char, String>;

// ---------------------------------------------------------------------------
// 1. Identity compose identity: id;id == id
// ---------------------------------------------------------------------------

#[test]
fn identity_compose_identity() {
    let types = vec!['a', 'b', 'c'];
    let id: FM = FrobeniusMorphism::identity(&types);

    let mut id_twice = id.clone();
    id_twice.compose(id.clone()).unwrap();

    assert_eq!(id_twice.domain(), types);
    assert_eq!(id_twice.codomain(), types);
    assert!(id_twice == id, "id;id should equal id");

    // Also check single-wire identity
    let single = vec!['x'];
    let id_single: FM = FrobeniusMorphism::identity(&single);
    let mut id_single_twice = id_single.clone();
    id_single_twice.compose(id_single.clone()).unwrap();
    assert!(id_single_twice == id_single);

    // Empty identity: identity on the empty tensor product
    let empty: Vec<char> = vec![];
    let id_empty: FM = FrobeniusMorphism::identity(&empty);
    let mut id_empty_twice = id_empty.clone();
    id_empty_twice.compose(id_empty.clone()).unwrap();
    assert!(id_empty_twice == id_empty);
}

// ---------------------------------------------------------------------------
// 2. Braiding involution: sigma(a,b);sigma(b,a) == id(a,b)
// ---------------------------------------------------------------------------

#[test]
fn braiding_involution() {
    // sigma(a,b) swaps wires: domain [a,b] -> codomain [b,a]
    let sigma_ab: FM = FrobeniusOperation::SymmetricBraiding('a', 'b').into();
    assert_eq!(sigma_ab.domain(), vec!['a', 'b']);
    assert_eq!(sigma_ab.codomain(), vec!['b', 'a']);

    // sigma(b,a) swaps back: domain [b,a] -> codomain [a,b]
    let sigma_ba: FM = FrobeniusOperation::SymmetricBraiding('b', 'a').into();
    assert_eq!(sigma_ba.domain(), vec!['b', 'a']);
    assert_eq!(sigma_ba.codomain(), vec!['a', 'b']);

    // Compose: sigma(a,b);sigma(b,a) should behave as identity on [a,b]
    let mut composed = sigma_ab.clone();
    composed.compose(sigma_ba).unwrap();
    assert_eq!(composed.domain(), vec!['a', 'b']);
    assert_eq!(composed.codomain(), vec!['a', 'b']);

    let id_ab: FM = FrobeniusMorphism::identity(&vec!['a', 'b']);
    // The two-layer simplification rule collapses braiding pairs into identities,
    // so the composed morphism should be structurally equal to the identity.
    assert!(
        composed == id_ab,
        "sigma(a,b);sigma(b,a) should equal id(a,b)"
    );
}

// ---------------------------------------------------------------------------
// 3. Spider fusion: Spider(z,m,n);Spider(z,n,k) == Spider(z,m,k)
// ---------------------------------------------------------------------------

#[test]
fn spider_fusion() {
    // Spider(z,2,3) composed with Spider(z,3,1) should fuse to Spider(z,2,1)
    let spider_23: FM = FrobeniusOperation::Spider('z', 2, 3).into();
    let spider_31: FM = FrobeniusOperation::Spider('z', 3, 1).into();

    assert_eq!(spider_23.domain(), vec!['z', 'z']);
    assert_eq!(spider_23.codomain(), vec!['z', 'z', 'z']);
    assert_eq!(spider_31.domain(), vec!['z', 'z', 'z']);
    assert_eq!(spider_31.codomain(), vec!['z']);

    let mut fused = spider_23;
    fused.compose(spider_31).unwrap();
    assert_eq!(fused.domain(), vec!['z', 'z']);
    assert_eq!(fused.codomain(), vec!['z']);

    // The fused morphism should be equivalent to Spider(z,2,1).
    // Note: Spider(z,2,1) and Multiplication(z) are distinct enum variants,
    // so structural equality is checked against a Spider, not Multiplication.
    let expected_fused: FM = FrobeniusOperation::Spider('z', 2, 1).into();
    assert!(
        fused == expected_fused,
        "Spider(z,2,3);Spider(z,3,1) should fuse to Spider(z,2,1)"
    );

    // Also verify Spider(z,1,3);Spider(z,3,4) -> Spider(z,1,4)
    let spider_13: FM = FrobeniusOperation::Spider('z', 1, 3).into();
    let spider_34: FM = FrobeniusOperation::Spider('z', 3, 4).into();
    let mut fused_2 = spider_13;
    fused_2.compose(spider_34).unwrap();
    assert_eq!(fused_2.domain(), vec!['z']);
    assert_eq!(fused_2.codomain(), vec!['z', 'z', 'z', 'z']);

    // Compare with special_frobenius_morphism(1,4,z) which builds the same thing
    let expected: FM = special_frobenius_morphism(1, 4, 'z');
    assert_eq!(fused_2.domain(), expected.domain());
    assert_eq!(fused_2.codomain(), expected.codomain());
}

// ---------------------------------------------------------------------------
// 4. Unit-counit scalar: Unit(z);Counit(z) produces a scalar (empty morphism)
// ---------------------------------------------------------------------------

#[test]
fn unit_counit_scalar() {
    let unit: FM = FrobeniusOperation::Unit('z').into();
    let counit: FM = FrobeniusOperation::Counit('z').into();

    assert_eq!(unit.domain(), Vec::<char>::new());
    assert_eq!(unit.codomain(), vec!['z']);
    assert_eq!(counit.domain(), vec!['z']);
    assert_eq!(counit.codomain(), Vec::<char>::new());

    // Composing unit then counit: the scalar loop
    let mut scalar = unit;
    scalar.compose(counit).unwrap();

    // Result has empty domain and empty codomain
    assert_eq!(scalar.domain(), Vec::<char>::new());
    assert_eq!(scalar.codomain(), Vec::<char>::new());
    // The unit-counit cancellation rule removes both blocks. A single
    // (vacuous) identity layer is retained to preserve the empty interface.
    assert!(
        scalar.depth() <= 1,
        "Scalar loop should simplify to at most depth 1, got {}",
        scalar.depth()
    );

    // Also check via special_frobenius_morphism(0,0,z) which builds the same thing
    let scalar_via_factory: FM = special_frobenius_morphism(0, 0, 'z');
    assert_eq!(scalar_via_factory.domain(), Vec::<char>::new());
    assert_eq!(scalar_via_factory.codomain(), Vec::<char>::new());
}

// ---------------------------------------------------------------------------
// 5. Composition associativity: (f;g);h == f;(g;h)
// ---------------------------------------------------------------------------

#[test]
fn composition_associativity() {
    // f: Comultiplication 'a' -> domain ['a'], codomain ['a','a']
    let f: FM = FrobeniusOperation::Comultiplication('a').into();
    // g: id(a) tensor counit(a), i.e. identity on first wire, counit on second
    let mut g: FM = FrobeniusOperation::Identity('a').into();
    g.monoidal(FrobeniusOperation::Counit('a').into());
    // h: Identity('a')
    let h: FM = FrobeniusOperation::Identity('a').into();

    // (f;g);h
    let mut fg = f.clone();
    fg.compose(g.clone()).unwrap();
    let mut fg_h = fg.clone();
    fg_h.compose(h.clone()).unwrap();

    // f;(g;h)
    let mut gh = g;
    gh.compose(h).unwrap();
    let mut f_gh = f;
    f_gh.compose(gh).unwrap();

    assert_eq!(fg_h.domain(), f_gh.domain());
    assert_eq!(fg_h.codomain(), f_gh.codomain());
    assert!(fg_h == f_gh, "Composition should be associative");
}

// ---------------------------------------------------------------------------
// 6. Monoidal product: tensor of two morphisms has combined domain/codomain
// ---------------------------------------------------------------------------

#[test]
fn monoidal_product_domains() {
    // Multiplication(a): [a,a] -> [a]
    let mul_a: FM = FrobeniusOperation::Multiplication('a').into();
    // Comultiplication(b): [b] -> [b,b]
    let comul_b: FM = FrobeniusOperation::Comultiplication('b').into();

    let mut tensor = mul_a.clone();
    tensor.monoidal(comul_b.clone());

    // Domain is concatenation: [a,a,b]
    assert_eq!(tensor.domain(), vec!['a', 'a', 'b']);
    // Codomain is concatenation: [a,b,b]
    assert_eq!(tensor.codomain(), vec!['a', 'b', 'b']);

    // Tensor with identity preserves the morphism's effect
    let id_c: FM = FrobeniusMorphism::identity(&vec!['c']);
    let mut mul_with_id = mul_a.clone();
    mul_with_id.monoidal(id_c);
    assert_eq!(mul_with_id.domain(), vec!['a', 'a', 'c']);
    assert_eq!(mul_with_id.codomain(), vec!['a', 'c']);

    // Tensor with empty morphism is a no-op
    let empty: FM = FrobeniusMorphism::new();
    let mut mul_with_empty = mul_a.clone();
    mul_with_empty.monoidal(empty);
    assert_eq!(mul_with_empty.domain(), mul_a.domain());
    assert_eq!(mul_with_empty.codomain(), mul_a.codomain());
}

// ---------------------------------------------------------------------------
// 7. from_decomposition roundtrip: create from Decomposition, verify types
// ---------------------------------------------------------------------------

#[test]
fn from_decomposition_roundtrip() {
    // The finite set map {0->0, 1->0, 2->1} from {0,1,2} to {0,1,2}
    // where element 2 of the codomain is not hit (leftover = 1).
    let map = vec![0_usize, 0, 1];
    let decomp = Decomposition::try_from((map, 1_usize)).unwrap();

    // Source types: each element in domain gets a label based on where it maps.
    // map[0]=0, map[1]=0, map[2]=1, so source_types[i] = codomain_types[map[i]]
    let source_types = vec!['x', 'x', 'y'];
    let codomain_types = vec!['x', 'y', 'q'];

    let morphism: FrobeniusMorphism<char, String> =
        from_decomposition(decomp, &source_types, &codomain_types).unwrap();

    assert_eq!(morphism.domain(), source_types);
    assert_eq!(morphism.codomain(), codomain_types);

    // Identity decomposition should produce identity morphism
    let id_decomp = Decomposition::try_from((vec![0_usize, 1, 2], 0_usize)).unwrap();
    let id_types = vec!['a', 'b', 'c'];
    let id_morphism: FrobeniusMorphism<char, String> =
        from_decomposition(id_decomp, &id_types, &id_types).unwrap();
    let expected_id: FrobeniusMorphism<char, String> = FrobeniusMorphism::identity(&id_types);
    assert_eq!(id_morphism.domain(), expected_id.domain());
    assert_eq!(id_morphism.codomain(), expected_id.codomain());
    assert!(id_morphism == expected_id);
}

// ---------------------------------------------------------------------------
// 8. special_frobenius_morphism domain/codomain consistency
// ---------------------------------------------------------------------------

#[test]
fn special_frobenius_morphism_consistency() {
    // Verify domain/codomain for the basic generators
    let unit: FM = special_frobenius_morphism(0, 1, 'z');
    assert_eq!(unit.domain(), Vec::<char>::new());
    assert_eq!(unit.codomain(), vec!['z']);

    let counit: FM = special_frobenius_morphism(1, 0, 'z');
    assert_eq!(counit.domain(), vec!['z']);
    assert_eq!(counit.codomain(), Vec::<char>::new());

    let mul: FM = special_frobenius_morphism(2, 1, 'z');
    assert_eq!(mul.domain(), vec!['z', 'z']);
    assert_eq!(mul.codomain(), vec!['z']);

    let comul: FM = special_frobenius_morphism(1, 2, 'z');
    assert_eq!(comul.domain(), vec!['z']);
    assert_eq!(comul.codomain(), vec!['z', 'z']);

    let id: FM = special_frobenius_morphism(1, 1, 'z');
    assert_eq!(id.domain(), vec!['z']);
    assert_eq!(id.codomain(), vec!['z']);
    assert_eq!(id.depth(), 1);

    // Larger spiders: domain is m copies, codomain is n copies
    for (m, n) in [(3, 2), (4, 1), (2, 5), (0, 3), (3, 0), (5, 5)] {
        let spider: FM = special_frobenius_morphism(m, n, 'w');
        assert_eq!(
            spider.domain(),
            vec!['w'; m],
            "Spider({m},{n}) domain should be {m} copies of 'w'"
        );
        assert_eq!(
            spider.codomain(),
            vec!['w'; n],
            "Spider({m},{n}) codomain should be {n} copies of 'w'"
        );
    }

    // Composing two spiders built by the factory should be composable
    // when codomain of first matches domain of second
    let s_3_2: FM = special_frobenius_morphism(3, 2, 'q');
    let s_2_4: FM = special_frobenius_morphism(2, 4, 'q');
    let mut composed = s_3_2;
    composed.compose(s_2_4).unwrap();
    assert_eq!(composed.domain(), vec!['q', 'q', 'q']);
    assert_eq!(composed.codomain(), vec!['q', 'q', 'q', 'q']);
}
