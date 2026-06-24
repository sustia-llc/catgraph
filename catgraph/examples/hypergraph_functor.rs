//! `HypergraphFunctor` API demonstration.
//!
//! Shows how `RelabelingFunctor` maps cospans between different label types
//! while preserving all categorical structure (Fong-Spivak §2.3, Eq. 12).

use catgraph::{
    category::{Composable, ComposableMutating, HasIdentity},
    cospan::Cospan,
    frobenius::FrobeniusMorphism,
    hypergraph_category::HypergraphCategory,
    hypergraph_functor::{CospanToFrobeniusFunctor, HypergraphFunctor, RelabelingFunctor},
    monoidal::Monoidal,
};

fn main() {
    println!("=== HypergraphFunctor: Relabeling ===\n");

    // Create a functor that relabels char → u32
    let functor = RelabelingFunctor::new(|c: char| c as u32);

    // --- Object mapping ---
    println!("Object mapping:");
    println!("  'a' → {}", functor.map_ob('a'));
    println!("  'z' → {}", functor.map_ob('z'));

    // --- Morphism mapping: identity ---
    println!("\nIdentity preservation:");
    let id = Cospan::<char>::identity(&vec!['a', 'b']);
    let mapped_id = functor.map_mor(&id).unwrap();
    println!("  id(['a','b'])   domain = {:?}", id.domain());
    println!("  F(id)           domain = {:?}", mapped_id.domain());

    // --- Frobenius generators ---
    println!("\nFrobenius generator preservation:");

    let eta = Cospan::<char>::unit('x');
    let mapped_eta = functor.map_mor(&eta).unwrap();
    println!("  η('x'):    [] → {:?}", eta.codomain());
    println!(
        "  F(η('x')): [] → {:?}  (= η({}))",
        mapped_eta.codomain(),
        functor.map_ob('x')
    );

    let mu = Cospan::<char>::multiplication('x');
    let mapped_mu = functor.map_mor(&mu).unwrap();
    println!("  μ('x'):    {:?} → {:?}", mu.domain(), mu.codomain());
    println!(
        "  F(μ('x')): {:?} → {:?}",
        mapped_mu.domain(),
        mapped_mu.codomain()
    );

    // --- Functoriality: composition ---
    println!("\nFunctoriality (composition):");
    let delta = Cospan::<char>::comultiplication('a');
    let mu_a = Cospan::<char>::multiplication('a');
    let composed = delta.compose(&mu_a).unwrap();
    let mapped_composed = functor.map_mor(&composed).unwrap();
    let mapped_delta = functor.map_mor(&delta).unwrap();
    let mapped_mu_a = functor.map_mor(&mu_a).unwrap();
    let compose_mapped = mapped_delta.compose(&mapped_mu_a).unwrap();
    println!("  F(δ;μ) middle     = {:?}", mapped_composed.middle());
    println!("  F(δ);F(μ) middle  = {:?}", compose_mapped.middle());
    println!(
        "  Equal? {}",
        mapped_composed.middle() == compose_mapped.middle()
            && mapped_composed.left_to_middle() == compose_mapped.left_to_middle()
            && mapped_composed.right_to_middle() == compose_mapped.right_to_middle()
    );

    // --- Monoidal preservation ---
    println!("\nMonoidal preservation:");
    let g = Cospan::<char>::unit('a');
    let h = Cospan::<char>::counit('b');
    let mut tensor = g.clone();
    tensor.monoidal(h.clone());
    let mapped_tensor = functor.map_mor(&tensor).unwrap();
    let mut mapped_parts = functor.map_mor(&g).unwrap();
    mapped_parts.monoidal(functor.map_mor(&h).unwrap());
    println!("  F(g⊗h) domain   = {:?}", mapped_tensor.domain());
    println!("  F(g)⊗F(h) domain = {:?}", mapped_parts.domain());

    println!("\n=== CospanToFrobeniusFunctor: Cospan → FrobeniusMorphism ===\n");

    let ctf = CospanToFrobeniusFunctor::<String>::new();

    // Object mapping is identity
    println!("Object mapping (identity):");
    println!("  'a' → '{}'", ctf.map_ob('a'));

    // Generators
    println!("\nFrobenius generator preservation:");
    let eta_cospan = Cospan::<char>::unit('a');
    let mapped_eta: FrobeniusMorphism<char, String> = ctf.map_mor(&eta_cospan).unwrap();
    println!("  η('a'): [] → {:?}  (cospan)", eta_cospan.codomain());
    println!(
        "  F(η):   [] → {:?}  (frobenius morphism)",
        mapped_eta.codomain()
    );

    let mu_cospan = Cospan::<char>::multiplication('a');
    let mapped_mu_c: FrobeniusMorphism<char, String> = ctf.map_mor(&mu_cospan).unwrap();
    println!(
        "  μ('a'): {:?} → {:?}  (cospan)",
        mu_cospan.domain(),
        mu_cospan.codomain()
    );
    println!(
        "  F(μ):   {:?} → {:?}  (frobenius morphism)",
        mapped_mu_c.domain(),
        mapped_mu_c.codomain()
    );

    // Functoriality
    println!("\nFunctoriality:");
    let delta_cospan = Cospan::<char>::comultiplication('a');
    let composed_c = delta_cospan.compose(&mu_cospan).unwrap();
    let mapped_composed_c: FrobeniusMorphism<char, String> = ctf.map_mor(&composed_c).unwrap();
    println!(
        "  F(δ;μ): {:?} → {:?}",
        mapped_composed_c.domain(),
        mapped_composed_c.codomain()
    );
}
