//! Frobenius algebra and `MorphismSystem` demonstration.
//!
//! Shows `FrobeniusOperation` variants, building `FrobeniusMorphism` from operations,
//! simplification via `two_layer_simplify`, `from_permutation`, `from_decomposition`,
//! spider morphisms, and DAG-based `MorphismSystem` with `fill_black_boxes`.

use catgraph::category::{ComposableMutating, HasIdentity};
use catgraph::errors::CatgraphError;
use catgraph::frobenius::{
    Contains, FrobeniusMorphism, FrobeniusOperation, InterpretableMorphism, MorphismSystem,
    from_decomposition, special_frobenius_morphism,
};
use catgraph::monoidal::Monoidal;

// ============================================================================
// Basic Operations
// ============================================================================

fn basic_operations() {
    println!("=== Basic FrobeniusOperations ===\n");

    // The six primitive operations (with char as Lambda, () as BlackBoxLabel)
    let unit: FrobeniusOperation<char, ()> = FrobeniusOperation::Unit('a');
    let counit: FrobeniusOperation<char, ()> = FrobeniusOperation::Counit('a');
    let mul: FrobeniusOperation<char, ()> = FrobeniusOperation::Multiplication('a');
    let comul: FrobeniusOperation<char, ()> = FrobeniusOperation::Comultiplication('a');
    let id: FrobeniusOperation<char, ()> = FrobeniusOperation::Identity('a');
    let braid: FrobeniusOperation<char, ()> = FrobeniusOperation::SymmetricBraiding('a', 'b');

    // Convert each to a single-layer morphism
    let unit_m: FrobeniusMorphism<char, ()> = unit.into();
    let counit_m: FrobeniusMorphism<char, ()> = counit.into();
    let mul_m: FrobeniusMorphism<char, ()> = mul.into();
    let comul_m: FrobeniusMorphism<char, ()> = comul.into();
    let id_m: FrobeniusMorphism<char, ()> = id.into();
    let braid_m: FrobeniusMorphism<char, ()> = braid.into();

    println!(
        "unit:   depth={}, domain={:?} -> codomain={:?}",
        unit_m.depth(),
        unit_m.domain(),
        unit_m.codomain()
    );
    println!(
        "counit: depth={}, domain={:?} -> codomain={:?}",
        counit_m.depth(),
        counit_m.domain(),
        counit_m.codomain()
    );
    println!(
        "mul:    depth={}, domain={:?} -> codomain={:?}",
        mul_m.depth(),
        mul_m.domain(),
        mul_m.codomain()
    );
    println!(
        "comul:  depth={}, domain={:?} -> codomain={:?}",
        comul_m.depth(),
        comul_m.domain(),
        comul_m.codomain()
    );
    println!(
        "id:     depth={}, domain={:?} -> codomain={:?}",
        id_m.depth(),
        id_m.domain(),
        id_m.codomain()
    );
    println!(
        "braid:  depth={}, domain={:?} -> codomain={:?}",
        braid_m.depth(),
        braid_m.domain(),
        braid_m.codomain()
    );
    println!();
}

// ============================================================================
// Composition and Monoidal Product
// ============================================================================

fn composition_and_monoidal() {
    println!("=== Composition and Monoidal Product ===\n");

    // Compose: comultiplication then multiplication = identity-like (Frobenius law)
    let mut comul: FrobeniusMorphism<(), ()> = FrobeniusOperation::Comultiplication(()).into();
    let mul: FrobeniusMorphism<(), ()> = FrobeniusOperation::Multiplication(()).into();
    println!(
        "comul: domain={:?}, codomain={:?}",
        comul.domain(),
        comul.codomain()
    );
    println!(
        "mul:   domain={:?}, codomain={:?}",
        mul.domain(),
        mul.codomain()
    );
    let compose_result = comul.compose(mul);
    println!("comul ; mul: composable={}", compose_result.is_ok());
    println!("  depth={}", comul.depth());

    // Monoidal product: id ⊗ id
    let mut id1: FrobeniusMorphism<char, ()> = FrobeniusOperation::Identity('a').into();
    let id2: FrobeniusMorphism<char, ()> = FrobeniusOperation::Identity('b').into();
    id1.monoidal(id2);
    println!(
        "\nid('a') ⊗ id('b'): domain={:?}, codomain={:?}, depth={}",
        id1.domain(),
        id1.codomain(),
        id1.depth()
    );

    // Build mul ⊗ id, then compose with comul ⊗ id
    let mut mul_id: FrobeniusMorphism<(), ()> = FrobeniusOperation::Multiplication(()).into();
    mul_id.monoidal(FrobeniusOperation::Identity(()).into());
    println!(
        "mul ⊗ id: domain={:?}, codomain={:?}",
        mul_id.domain(),
        mul_id.codomain()
    );
    println!();
}

// ============================================================================
// Spider Morphisms
// ============================================================================

fn spider_morphisms() {
    println!("=== Spider Morphisms ===\n");

    // special_frobenius_morphism(m, n, type) builds the unique Spider(m,n) morphism
    // from the Frobenius generators (multiplication/comultiplication/unit/counit)

    for (m, n) in [
        (0, 1),
        (1, 0),
        (1, 1),
        (2, 1),
        (1, 2),
        (3, 1),
        (1, 3),
        (3, 2),
    ] {
        let spider: FrobeniusMorphism<(), ()> = special_frobenius_morphism(m, n, ());
        println!(
            "spider({m},{n}): depth={}, domain={:?} -> codomain={:?}",
            spider.depth(),
            spider.domain(),
            spider.codomain()
        );
    }

    // Typed spider
    let typed_spider: FrobeniusMorphism<char, ()> = special_frobenius_morphism(2, 3, 'x');
    println!(
        "\ntyped spider(2,3,'x'): domain={:?} -> codomain={:?}",
        typed_spider.domain(),
        typed_spider.codomain()
    );
    println!();
}

// ============================================================================
// Identity and Simplification
// ============================================================================

fn identity_and_simplification() {
    println!("=== Identity and Simplification ===\n");

    // Identity morphism
    let types = vec!['a', 'b', 'c'];
    let id: FrobeniusMorphism<char, ()> = FrobeniusMorphism::identity(&types);
    println!(
        "identity: depth={}, domain={:?}, codomain={:?}",
        id.depth(),
        id.domain(),
        id.codomain()
    );

    // Braiding cancellation: sigma(a,b) ; sigma(b,a) simplifies to identities
    let mut braid1: FrobeniusMorphism<char, ()> =
        FrobeniusOperation::SymmetricBraiding('a', 'b').into();
    let braid2: FrobeniusMorphism<char, ()> =
        FrobeniusOperation::SymmetricBraiding('b', 'a').into();
    let before_depth = braid1.depth();
    let _ = braid1.compose(braid2);
    println!(
        "\nsigma(a,b) ; sigma(b,a): depth before={}, after={}",
        before_depth,
        braid1.depth()
    );

    // Unit-counit cancellation: eta(z) ; epsilon(z) simplifies (scalar loop)
    let mut unit_counit: FrobeniusMorphism<(), ()> = FrobeniusOperation::Unit(()).into();
    let counit: FrobeniusMorphism<(), ()> = FrobeniusOperation::Counit(()).into();
    let _ = unit_counit.compose(counit);
    println!(
        "unit ; counit: depth={}, domain={:?}, codomain={:?}",
        unit_counit.depth(),
        unit_counit.domain(),
        unit_counit.codomain()
    );
    println!();
}

// ============================================================================
// From Permutation
// ============================================================================

fn from_permutation_demo() {
    println!("=== FrobeniusMorphism from Permutation ===\n");

    use catgraph::monoidal::SymmetricMonoidalMorphism;
    use permutations::Permutation;

    // Transposition as a Frobenius morphism
    let swap = Permutation::transposition(3, 0, 2);
    let types = ['a', 'b', 'c'];
    let perm_morph: FrobeniusMorphism<char, ()> =
        FrobeniusMorphism::from_permutation(swap, &types, true).unwrap();
    println!(
        "swap(0,2): depth={}, domain={:?} -> codomain={:?}",
        perm_morph.depth(),
        perm_morph.domain(),
        perm_morph.codomain()
    );

    // Identity permutation
    let id_perm = Permutation::identity(3);
    let id_morph: FrobeniusMorphism<char, ()> =
        FrobeniusMorphism::from_permutation(id_perm, &types, true).unwrap();
    println!(
        "identity(3): depth={}, domain={:?} -> codomain={:?}",
        id_morph.depth(),
        id_morph.domain(),
        id_morph.codomain()
    );

    // Cyclic rotation
    let rot = Permutation::rotation_left(4, 1);
    let types4 = ['a', 'b', 'c', 'd'];
    let rot_morph: FrobeniusMorphism<char, ()> =
        FrobeniusMorphism::from_permutation(rot, &types4, true).unwrap();
    println!(
        "rotate_left(1): depth={}, domain={:?} -> codomain={:?}",
        rot_morph.depth(),
        rot_morph.domain(),
        rot_morph.codomain()
    );
    println!();
}

// ============================================================================
// From Decomposition (epi-mono factorization)
// ============================================================================

fn from_decomposition_demo() {
    println!("=== FrobeniusMorphism from Decomposition ===\n");

    use catgraph::finset::{Decomposition, FinSetMorphism};

    // Decompose a finite set morphism into a Frobenius morphism
    // f: {0,1,2} -> {0,1}: maps 0->0, 1->0, 2->1 (surjection)
    let f: FinSetMorphism = (vec![0, 0, 1], 0);
    let decomp = Decomposition::try_from(f).unwrap();

    let source_types = ['a', 'a', 'b'];
    let target_types = ['a', 'b'];
    let frob: FrobeniusMorphism<char, ()> =
        from_decomposition(decomp, &source_types, &target_types).unwrap();
    println!(
        "surjection: depth={}, domain={:?} -> codomain={:?}",
        frob.depth(),
        frob.domain(),
        frob.codomain()
    );

    // Identity decomposition
    let id_decomp = Decomposition::identity(&3);
    let id_types = ['x', 'y', 'z'];
    let id_frob: FrobeniusMorphism<char, ()> =
        from_decomposition(id_decomp, &id_types, &id_types).unwrap();
    println!(
        "identity decomp: depth={}, domain={:?} -> codomain={:?}",
        id_frob.depth(),
        id_frob.domain(),
        id_frob.codomain()
    );
    println!();
}

// ============================================================================
// MorphismSystem DAG
// ============================================================================

/// A minimal container that holds references to other labels.
#[derive(Clone, Debug)]
struct SumContainer(Vec<String>);

impl Contains<String> for SumContainer {
    fn contained_labels(&self) -> Vec<String> {
        self.0.clone()
    }
}

/// A morphism that sums integer values from its resolved dependencies.
#[derive(Clone, Debug, PartialEq, Eq)]
struct SumMorphism(i32);

impl InterpretableMorphism<SumContainer, (), String> for SumMorphism {
    fn interpret<F>(
        container: &SumContainer,
        black_box_interpreter: F,
    ) -> Result<Self, CatgraphError>
    where
        F: Fn(&String, &[()], &[()]) -> Result<Self, CatgraphError>,
    {
        let mut sum = 0i32;
        for label in container.contained_labels() {
            let resolved = black_box_interpreter(&label, &[], &[])?;
            sum += resolved.0;
        }
        Ok(SumMorphism(sum))
    }
}

type SumSystem = MorphismSystem<String, (), SumContainer, SumMorphism>;

fn morphism_system_dag() {
    println!("=== MorphismSystem DAG ===\n");

    let mut sys: SumSystem = MorphismSystem::new("top".into());

    // Register leaf definitions (simple pieces)
    sys.add_definition_simple("x".into(), SumMorphism(10))
        .unwrap();
    sys.add_definition_simple("y".into(), SumMorphism(20))
        .unwrap();
    sys.add_definition_simple("z".into(), SumMorphism(30))
        .unwrap();

    println!("leaves: x=10, y=20, z=30");

    // Register a composite that depends on leaves
    sys.add_definition_composite("mid".into(), SumContainer(vec!["x".into(), "y".into()]))
        .unwrap();
    println!("mid = x + y (composite)");

    // Register the top-level composite
    sys.add_definition_composite("top".into(), SumContainer(vec!["mid".into(), "z".into()]))
        .unwrap();
    println!("top = mid + z (composite)");

    // Resolve the entire DAG
    let result = sys.fill_black_boxes(None).unwrap();
    println!("fill_black_boxes(top) = {result:?}  (expected 60)");

    // Resolve a specific target
    let mid_result = sys.fill_black_boxes(Some("mid".into())).unwrap();
    println!("fill_black_boxes(mid) = {mid_result:?}  (expected 30)");

    // Diamond dependency: two composites share a leaf
    let mut diamond: SumSystem = MorphismSystem::new("root".into());
    diamond
        .add_definition_simple("shared".into(), SumMorphism(7))
        .unwrap();
    diamond
        .add_definition_composite("left".into(), SumContainer(vec!["shared".into()]))
        .unwrap();
    diamond
        .add_definition_composite("right".into(), SumContainer(vec!["shared".into()]))
        .unwrap();
    diamond
        .add_definition_composite(
            "root".into(),
            SumContainer(vec!["left".into(), "right".into()]),
        )
        .unwrap();
    let diamond_result = diamond.fill_black_boxes(None).unwrap();
    println!("\ndiamond DAG: shared=7, root=left+right=7+7={diamond_result:?}");

    // Cycle detection
    let mut cyclic: SumSystem = MorphismSystem::new("a".into());
    cyclic
        .add_definition_composite("a".into(), SumContainer(vec!["b".into()]))
        .unwrap();
    let cycle_err = cyclic.add_definition_composite("b".into(), SumContainer(vec!["a".into()]));
    println!("cycle detection: is_err={}", cycle_err.is_err());
    println!();
}

fn main() {
    basic_operations();
    composition_and_monoidal();
    spider_morphisms();
    identity_and_simplification();
    from_permutation_demo();
    from_decomposition_demo();
    morphism_system_dag();
}
