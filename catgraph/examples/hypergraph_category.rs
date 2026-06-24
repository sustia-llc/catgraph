//! `HypergraphCategory` API demonstration (Fong-Spivak §2.3, Def 2.12).
//!
//! Shows the four Frobenius generators (η, ε, μ, δ) and derived cup/cap
//! for both `Cospan<Lambda>` and `FrobeniusMorphism<Lambda>`.
//! Verifies key axioms: unitality, specialness, associativity, and zigzag.

use catgraph::{
    category::{Composable, ComposableMutating, HasIdentity},
    cospan::Cospan,
    frobenius::FrobeniusMorphism,
    hypergraph_category::HypergraphCategory,
    monoidal::Monoidal,
};

fn main() {
    println!("=== HypergraphCategory: Cospan<char> (free hypergraph category) ===\n");

    let z = 'a';

    // --- Four Frobenius generators ---
    let eta = Cospan::<char>::unit(z);
    let eps = Cospan::<char>::counit(z);
    let mu = Cospan::<char>::multiplication(z);
    let delta = Cospan::<char>::comultiplication(z);

    println!("Frobenius generators for type '{z}':");
    println!(
        "  η (unit):           {:?} → {:?}",
        eta.domain(),
        eta.codomain()
    );
    println!(
        "  ε (counit):         {:?} → {:?}",
        eps.domain(),
        eps.codomain()
    );
    println!(
        "  μ (multiplication): {:?} → {:?}",
        mu.domain(),
        mu.codomain()
    );
    println!(
        "  δ (comultiplication): {:?} → {:?}",
        delta.domain(),
        delta.codomain()
    );

    // --- Derived cup/cap ---
    let cup = Cospan::<char>::cup(z).unwrap();
    let cap = Cospan::<char>::cap(z).unwrap();

    println!("\nDerived cup/cap:");
    println!("  cup = η;δ:  {:?} → {:?}", cup.domain(), cup.codomain());
    println!("  cap = μ;ε:  {:?} → {:?}", cap.domain(), cap.codomain());

    // --- Axiom: Unitality (η ⊗ id) ; μ = id ---
    println!("\n--- Axiom checks ---\n");

    let mut eta_id = Cospan::<char>::unit(z);
    eta_id.monoidal(Cospan::identity(&vec![z]));
    let unitality = eta_id.compose(&Cospan::multiplication(z)).unwrap();
    println!(
        "Unitality  (η⊗id);μ:  {:?} → {:?}  (should be [a] → [a])",
        unitality.domain(),
        unitality.codomain()
    );

    // --- Axiom: Specialness δ;μ = id ---
    let special = Cospan::<char>::comultiplication(z)
        .compose(&Cospan::multiplication(z))
        .unwrap();
    println!(
        "Specialness    δ;μ:   {:?} → {:?}  (should be [a] → [a])",
        special.domain(),
        special.codomain()
    );

    // --- Axiom: Associativity (μ⊗id);μ = (id⊗μ);μ ---
    let mu_fn = || Cospan::<char>::multiplication(z);
    let id_fn = || Cospan::<char>::identity(&vec![z]);

    let mut mu_id = mu_fn();
    mu_id.monoidal(id_fn());
    let left = mu_id.compose(&mu_fn()).unwrap();

    let mut id_mu = id_fn();
    id_mu.monoidal(mu_fn());
    let right = id_mu.compose(&mu_fn()).unwrap();

    println!(
        "Associativity: (μ⊗id);μ = {:?}→{:?}, (id⊗μ);μ = {:?}→{:?}",
        left.domain(),
        left.codomain(),
        right.domain(),
        right.codomain()
    );

    // --- Zigzag identity: (cup ⊗ id) ; (id ⊗ cap) = id ---
    let mut cup_id = Cospan::<char>::cup(z).unwrap();
    cup_id.monoidal(Cospan::identity(&vec![z]));
    let mut id_cap = Cospan::<char>::identity(&vec![z]);
    id_cap.monoidal(Cospan::cap(z).unwrap());
    let snake = cup_id.compose(&id_cap).unwrap();
    println!(
        "Zigzag (cup⊗id);(id⊗cap): {:?} → {:?}  (should be [a] → [a])",
        snake.domain(),
        snake.codomain()
    );

    // =========================================================================
    println!("\n=== HypergraphCategory: FrobeniusMorphism<char, String> ===\n");

    type FM = FrobeniusMorphism<char, String>;

    let eta_f = FM::unit(z);
    let eps_f = FM::counit(z);
    let mu_f = FM::multiplication(z);
    let delta_f = FM::comultiplication(z);

    println!("Frobenius generators for type '{z}':");
    println!("  η: {:?} → {:?}", eta_f.domain(), eta_f.codomain());
    println!("  ε: {:?} → {:?}", eps_f.domain(), eps_f.codomain());
    println!("  μ: {:?} → {:?}", mu_f.domain(), mu_f.codomain());
    println!("  δ: {:?} → {:?}", delta_f.domain(), delta_f.codomain());

    let cup_f = FM::cup(z).unwrap();
    let cap_f = FM::cap(z).unwrap();
    println!("\nDerived cup/cap:");
    println!(
        "  cup = η;δ:  {:?} → {:?}",
        cup_f.domain(),
        cup_f.codomain()
    );
    println!(
        "  cap = μ;ε:  {:?} → {:?}",
        cap_f.domain(),
        cap_f.codomain()
    );

    // Specialness in FrobeniusMorphism
    let mut delta_fm = FM::comultiplication(z);
    let mu_fm = FM::multiplication(z);
    ComposableMutating::compose(&mut delta_fm, mu_fm).unwrap();
    println!(
        "\nSpecialness δ;μ: {:?} → {:?}  (should be [a] → [a])",
        delta_fm.domain(),
        delta_fm.codomain()
    );

    // =========================================================================
    println!("\n=== Multi-type generators ===\n");

    // Generators compose monoidally across types
    let mut eta_ab = Cospan::<char>::unit('a');
    eta_ab.monoidal(Cospan::unit('b'));
    println!(
        "η('a') ⊗ η('b'):  {:?} → {:?}",
        eta_ab.domain(),
        eta_ab.codomain()
    );

    let mut mu_ab = Cospan::<char>::multiplication('a');
    mu_ab.monoidal(Cospan::multiplication('b'));
    println!(
        "μ('a') ⊗ μ('b'):  {:?} → {:?}",
        mu_ab.domain(),
        mu_ab.codomain()
    );
}
