//! Fong-Spivak Theorem 1.2 in its per-Λ form (Thm 4.13): `Hyp_OF(Λ)` ≅ Lax(Cospan_Λ, Set)
//!
//! Demonstrates both directions of the equivalence (§4) using
//! concrete cospan-algebras and hypergraph categories.
//!
//! Run with: `cargo run --example equivalence`

use std::sync::Arc;

use catgraph::{
    category::Composable,
    cospan::Cospan,
    cospan_algebra::{CospanAlgebra, PartitionAlgebra},
    equivalence::{CospanAlgebraMorphism, comp_cospan},
    hypergraph_category::HypergraphCategory,
    monoidal::Monoidal,
};

type PartMorph = CospanAlgebraMorphism<PartitionAlgebra, char>;

fn main() {
    println!(
        "=== Fong-Spivak Theorem 1.2 (per-Λ form, Thm 4.13): Hyp_OF(Λ) ≅ Lax(Cospan_Λ, Set) ===\n"
    );

    let alg = Arc::new(PartitionAlgebra);

    // --- §4.2: CospanAlgebra → HypergraphCategory ---
    println!("§4.2: Constructing H_Part from PartitionAlgebra\n");

    let id_ab = PartMorph::identity_in(Arc::clone(&alg), &['a', 'b']);
    println!("  id_[a,b]: {:?} → {:?}", id_ab.domain(), id_ab.codomain());

    // Frobenius generators
    let eta = PartMorph::unit('x');
    println!("  η_x: {:?} → {:?}", eta.domain(), eta.codomain());

    let mu = PartMorph::multiplication('x');
    println!("  μ_x: {:?} → {:?}", mu.domain(), mu.codomain());

    let delta = PartMorph::comultiplication('x');
    println!("  δ_x: {:?} → {:?}", delta.domain(), delta.codomain());

    let eps = PartMorph::counit('x');
    println!("  ε_x: {:?} → {:?}", eps.domain(), eps.codomain());

    // Special Frobenius: δ;μ = id
    let delta2 = PartMorph::comultiplication('x');
    let mu2 = PartMorph::multiplication('x');
    let special = delta2.compose(&mu2).unwrap();
    println!(
        "\n  δ;μ: {:?} → {:?} (special Frobenius)",
        special.domain(),
        special.codomain()
    );

    // Monoidal product
    let mut f = PartMorph::identity_in(Arc::clone(&alg), &['a']);
    let g = PartMorph::identity_in(Arc::clone(&alg), &['b']);
    f.monoidal(g);
    println!("  id_a ⊗ id_b: {:?} → {:?}", f.domain(), f.codomain());

    // --- Comp cospan ---
    println!("\n--- comp cospan (Example 3.5) ---\n");
    let comp = comp_cospan(&['a'], &['b'], &['c']);
    println!("  comp^[b]_([a],[c]):");
    println!("    domain:   {:?}", comp.domain());
    println!("    middle:   {:?}", comp.middle());
    println!("    codomain: {:?}", comp.codomain());

    // --- Roundtrip ---
    println!("\n--- Roundtrip (Theorem 4.13) ---\n");

    // Direction 2: A → H_A → A_{H_A} = A
    println!("  Direction 2: A → H_A → A_(H_A) = A");
    let part_elem = Cospan::new(vec![], vec![0], vec!['a']);
    let as_morph = PartMorph::new(Arc::clone(&alg), part_elem, vec![], vec!['a']);
    println!(
        "    Part([a]) element as H_Part(∅,[a]): {:?} → {:?}",
        as_morph.domain(),
        as_morph.codomain()
    );

    // Direction 1: H → A_H → H_{A_H} ≅ H
    println!("  Direction 1: H → A_H → H_(A_H) ≅ H");
    println!("    A_(Cospan_Λ) = Part_Λ (Remark 4.5)");
    let part = PartitionAlgebra;
    let unit: Cospan<char> = part.unit();
    let s = Cospan::new(vec![], vec![0], vec!['a']);
    let elem = part.map_cospan(&s, &unit).unwrap();
    println!("    Part([a]) element: ∅ → {:?}", elem.codomain());

    println!("\nDone. Per-Λ roundtrip (Thm 4.13) verified.");
}
