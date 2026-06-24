//! Compact closed structure demonstration (Fong-Spivak §3.1).
//!
//! Shows cup/cap morphisms, zigzag identities, tensor-ordered variants,
//! and the name bijection (Prop 3.2) with composition-via-names (Prop 3.3).

use catgraph::{
    category::{ComposableMutating, HasIdentity},
    compact_closed::{
        cap, cap_single, cap_tensor, compose_names, cup, cup_single, cup_tensor, name, unname,
    },
    frobenius::{FrobeniusMorphism, FrobeniusOperation},
    monoidal::Monoidal,
};

type FM = FrobeniusMorphism<char, String>;

fn main() {
    println!("=== Cup/Cap: Per-Type (Paired Ordering) ===\n");

    // Single-type cup and cap
    let cup_a: FM = cup_single('a');
    let cap_a: FM = cap_single('a');
    println!(
        "cup('a') = η;δ:  {:?} → {:?}",
        cup_a.domain(),
        cup_a.codomain()
    );
    println!(
        "cap('a') = μ;ε:  {:?} → {:?}",
        cap_a.domain(),
        cap_a.codomain()
    );

    // Multi-type: paired ordering [a,a,b,b]
    let cup_ab: FM = cup(&['a', 'b']);
    let cap_ab: FM = cap(&['a', 'b']);
    println!(
        "\ncup([a,b]):  {:?} → {:?}  (paired)",
        cup_ab.domain(),
        cup_ab.codomain()
    );
    println!(
        "cap([a,b]):  {:?} → {:?}  (paired)",
        cap_ab.domain(),
        cap_ab.codomain()
    );

    // Empty types: identity on []
    let cup_empty: FM = cup(&[]);
    println!(
        "cup([]):     {:?} → {:?}  (identity)",
        cup_empty.domain(),
        cup_empty.codomain()
    );

    // --- Zigzag identity (snake lemma) ---
    println!("\n=== Zigzag Identities ===\n");
    println!("(cup ⊗ id) ; (id ⊗ cap) = id\n");

    let z = 'x';
    let mut right_half: FM = cup_single(z);
    right_half.monoidal(FrobeniusMorphism::identity(&vec![z]));
    let mut left_half: FM = FrobeniusMorphism::identity(&vec![z]);
    left_half.monoidal(cap_single(z));
    let mut snake = right_half;
    snake.compose(left_half).expect("interfaces match");
    println!(
        "Right snake: {:?} → {:?}  (should be [x] → [x])",
        snake.domain(),
        snake.codomain()
    );

    // Left snake: (id ⊗ cup) ; (cap ⊗ id) = id
    let mut left_cup: FM = FrobeniusMorphism::identity(&vec![z]);
    left_cup.monoidal(cup_single(z));
    let mut cap_id: FM = cap_single(z);
    cap_id.monoidal(FrobeniusMorphism::identity(&vec![z]));
    let mut left_snake = left_cup;
    left_snake.compose(cap_id).expect("interfaces match");
    println!(
        "Left snake:  {:?} → {:?}  (should be [x] → [x])",
        left_snake.domain(),
        left_snake.codomain()
    );

    println!("\n=== Tensor-Ordered Cup/Cap ===\n");
    println!("Tensor ordering: X⊗X = [z₁,z₂,...,z₁,z₂,...]\n");

    let cup_t: FM = cup_tensor(&['a', 'b']);
    let cap_t: FM = cap_tensor(&['a', 'b']);
    println!(
        "cup_tensor([a,b]):  {:?} → {:?}",
        cup_t.domain(),
        cup_t.codomain()
    );
    println!(
        "cap_tensor([a,b]):  {:?} → {:?}",
        cap_t.domain(),
        cap_t.codomain()
    );

    // Three types
    let cup_t3: FM = cup_tensor(&['x', 'y', 'z']);
    println!(
        "cup_tensor([x,y,z]): {:?} → {:?}",
        cup_t3.domain(),
        cup_t3.codomain()
    );

    // cup_tensor ; cap_tensor roundtrip
    let mut dim: FM = cup_tensor(&['a', 'b']);
    dim.compose(cap_tensor(&['a', 'b'])).expect("X⊗X interface");
    println!(
        "\ncup;cap roundtrip:  {:?} → {:?}  (dimension morphism)",
        dim.domain(),
        dim.codomain()
    );

    println!("\n=== Name Bijection (Prop 3.2) ===\n");
    println!("H(X,Y) ≅ H(I, X⊗Y)  via  name/unname\n");

    // Name of identity: id_X ↦ cup_X : I → X⊗X
    let id_a: FM = FrobeniusMorphism::identity(&vec!['a']);
    let named_id = name(&id_a).unwrap();
    println!(
        "name(id([a])):   {:?} → {:?}  (= cup)",
        named_id.domain(),
        named_id.codomain()
    );

    // Multi-type identity
    let id_ab: FM = FrobeniusMorphism::identity(&vec!['a', 'b']);
    let named_id_ab = name(&id_ab).unwrap();
    println!(
        "name(id([a,b])): {:?} → {:?}",
        named_id_ab.domain(),
        named_id_ab.codomain()
    );

    // Name of unit: η: [] → [a] has name η itself (domain is already I)
    let unit: FM = FrobeniusOperation::Unit('a').into();
    let named_unit = name(&unit).unwrap();
    println!(
        "name(η('a')):    {:?} → {:?}",
        named_unit.domain(),
        named_unit.codomain()
    );

    // Unname roundtrip: unname(name(f)) should recover f's domain/codomain
    let recovered = unname(&named_id, 1).unwrap();
    println!(
        "\nunname(name(id)): {:?} → {:?}  (roundtrip)",
        recovered.domain(),
        recovered.codomain()
    );

    let recovered_ab = unname(&named_id_ab, 2).unwrap();
    println!(
        "unname(name(id[a,b])): {:?} → {:?}  (roundtrip)",
        recovered_ab.domain(),
        recovered_ab.codomain()
    );

    println!("\n=== Composition via Names (Prop 3.3) ===\n");
    println!("(f̂ ⊗ ĝ) ; comp = (f;g)^\n");

    // Compose names of two identities
    let f_hat = name(&id_a).unwrap();
    let g_hat = name(&id_a).unwrap();
    let composed = compose_names(&f_hat, &g_hat, 1, 1).unwrap();
    println!(
        "compose_names(id^, id^): {:?} → {:?}  (= name(id;id) = name(id))",
        composed.domain(),
        composed.codomain()
    );
}
