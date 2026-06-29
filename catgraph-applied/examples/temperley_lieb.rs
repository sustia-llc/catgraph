//! Temperley-Lieb and Brauer algebra API demonstration.
//!
//! Shows TL and symmetric group generators, identity morphisms,
//! composition, dagger (involution), tensor product (monoidal),
//! simplification, and the braid relation (Yang-Baxter equation).
//!
//! Build with `cargo run --example temperley_lieb -p catgraph-applied`.

use catgraph::category::{Composable, HasIdentity};
use catgraph::monoidal::Monoidal;
use catgraph_applied::temperley_lieb::BrauerMorphism;

// ============================================================================
// Generators
// ============================================================================

fn generators() {
    println!("=== Generators ===\n");

    let n = 4;
    let e_i = BrauerMorphism::<i64>::temperley_lieb_gens(n);
    println!("Temperley-Lieb generators e_0..e_{} for n={n}:", n - 2);
    for (idx, ei) in e_i.iter().enumerate() {
        println!(
            "  e_{idx}: domain={}, codomain={}",
            ei.domain(),
            ei.codomain()
        );
    }

    let s_i = BrauerMorphism::<i64>::symmetric_alg_gens(n);
    println!("\nSymmetric group generators s_0..s_{} for n={n}:", n - 2);
    for (idx, si) in s_i.iter().enumerate() {
        println!(
            "  s_{idx}: domain={}, codomain={}",
            si.domain(),
            si.codomain()
        );
    }
    println!();
}

// ============================================================================
// Identity
// ============================================================================

fn identity() {
    println!("=== Identity ===\n");

    let n = 4;
    let id = BrauerMorphism::<i64>::identity(&n);
    println!(
        "identity({n}): domain={}, codomain={}",
        id.domain(),
        id.codomain()
    );

    // Identity composed with itself gives identity.
    let id_squared = id.compose(&id).expect("id * id failed");
    println!("id * id == id: {}", id_squared == id);
    println!();
}

// ============================================================================
// Composition
// ============================================================================

fn composition() {
    println!("=== Composition ===\n");

    let n = 5;
    let e_i = BrauerMorphism::<i64>::temperley_lieb_gens(n);
    let id = BrauerMorphism::<i64>::identity(&n);

    // Identity is left and right unit for composition.
    let left = e_i[0].compose(&id).expect("e_0 * id failed");
    let right = id.compose(&e_i[0]).expect("id * e_0 failed");
    println!("e_0 * id == e_0: {}", left == e_i[0]);
    println!("id * e_0 == e_0: {}", right == e_i[0]);

    // Chain composition: e_0 * e_1 * e_2
    let chain = e_i[0]
        .compose(&e_i[1])
        .and_then(|z| z.compose(&e_i[2]))
        .expect("chain composition failed");
    println!(
        "e_0 * e_1 * e_2: domain={}, codomain={}",
        chain.domain(),
        chain.codomain(),
    );

    // e_i^2 = delta * e_i (differs due to delta power tracking).
    let e0_squared = e_i[0].compose(&e_i[0]).expect("e_0^2 failed");
    println!(
        "e_0^2 == e_0: {} (differs because e_0^2 = delta * e_0)",
        e0_squared == e_i[0]
    );

    // Symmetric generator is an involution: s_i^2 = identity.
    let s_i = BrauerMorphism::<i64>::symmetric_alg_gens(n);
    let s0_squared = s_i[0].compose(&s_i[0]).expect("s_0^2 failed");
    println!("s_0^2 == id: {} (involution)", s0_squared == id);
    println!();
}

// ============================================================================
// Dagger (Involution)
// ============================================================================

fn dagger() {
    println!("=== Dagger (Involution) ===\n");

    let n = 4;
    let e_i = BrauerMorphism::<i64>::temperley_lieb_gens(n);
    let s_i = BrauerMorphism::<i64>::symmetric_alg_gens(n);

    // TL generators are self-adjoint under trivial conjugation.
    let e0_dag = e_i[0].dagger(|z| z);
    println!("e_0 == e_0^dagger (self-adjoint): {}", e0_dag == e_i[0]);

    // Symmetric generators are also self-adjoint (they are transpositions).
    let s0_dag = s_i[0].dagger(|z| z);
    println!("s_0 == s_0^dagger (self-adjoint): {}", s0_dag == s_i[0]);

    // Dagger swaps domain and codomain (visible when they differ, but
    // for generators domain == codomain, so the swap is invisible).
    let e0_dag = e_i[0].dagger(|z| z);
    println!(
        "dagger preserves domain/codomain: domain={}, codomain={}",
        e0_dag.domain(),
        e0_dag.codomain(),
    );
    println!();
}

// ============================================================================
// Tensor Product (Monoidal)
// ============================================================================

fn tensor_product() {
    println!("=== Tensor Product (Monoidal) ===\n");

    let e_i = BrauerMorphism::<i64>::temperley_lieb_gens(3);
    let id2 = BrauerMorphism::<i64>::identity(&2);

    // e_0 (n=3) tensor identity (n=2) gives a morphism on 5 strands.
    let mut tensored = e_i[0].clone();
    tensored.monoidal(id2.clone());
    println!(
        "e_0 (3) tensor id (2): domain={}, codomain={}",
        tensored.domain(),
        tensored.codomain(),
    );

    // Tensor two identities.
    let id3 = BrauerMorphism::<i64>::identity(&3);
    let mut id_tensor = id3.clone();
    id_tensor.monoidal(id2);
    let id5 = BrauerMorphism::<i64>::identity(&5);
    println!("id(3) tensor id(2) == id(5): {}", id_tensor == id5);
    println!();
}

// ============================================================================
// Simplification and Delta Polynomial
// ============================================================================

fn simplification() {
    println!("=== Simplification & Delta Polynomial ===\n");

    // delta_polynomial creates morphisms in Hom(0,0) — the polynomial ring T[delta].
    // [0, 0, 1] represents 0 + 0*delta + 1*delta^2
    let mut poly = BrauerMorphism::<i64>::delta_polynomial(&[0, 0, 1]);
    println!(
        "delta^2 polynomial: domain={}, codomain={}",
        poly.domain(),
        poly.codomain(),
    );

    // Simplify removes zero-coefficient terms.
    poly.simplify();

    // Compare with zero polynomial.
    let mut zero_poly = BrauerMorphism::<i64>::delta_polynomial(&[0]);
    zero_poly.simplify();
    println!("delta^2 != zero after simplify: {}", poly != zero_poly);
    println!();
}

// ============================================================================
// Braid Relation (Yang-Baxter)
// ============================================================================

fn braid_relation() {
    println!("=== Braid Relation (Yang-Baxter) ===\n");

    let n = 5;
    let s_i = BrauerMorphism::<i64>::symmetric_alg_gens(n);

    // s_i * s_{i+1} * s_i = s_{i+1} * s_i * s_{i+1}
    for i in 0..n - 2 {
        let lhs = s_i[i]
            .compose(&s_i[i + 1])
            .and_then(|z| z.compose(&s_i[i]))
            .expect("lhs failed");
        let rhs = s_i[i + 1]
            .compose(&s_i[i])
            .and_then(|z| z.compose(&s_i[i + 1]))
            .expect("rhs failed");
        println!(
            "s_{i} * s_{} * s_{i} == s_{} * s_{i} * s_{}: {}",
            i + 1,
            i + 1,
            i + 1,
            lhs == rhs,
        );
    }
    println!();
}

// ============================================================================
// Mixed Absorption
// ============================================================================

fn mixed_absorption() {
    println!("=== Mixed Absorption ===\n");

    let n = 4;
    let e_i = BrauerMorphism::<i64>::temperley_lieb_gens(n);
    let s_i = BrauerMorphism::<i64>::symmetric_alg_gens(n);

    // e_i * s_i = e_i and s_i * e_i = e_i
    for idx in 0..n - 1 {
        let es = e_i[idx].compose(&s_i[idx]).expect("e_i * s_i failed");
        let se = s_i[idx].compose(&e_i[idx]).expect("s_i * e_i failed");
        println!(
            "e_{idx} * s_{idx} == e_{idx}: {},  s_{idx} * e_{idx} == e_{idx}: {}",
            es == e_i[idx],
            se == e_i[idx],
        );
    }
    println!();
}

fn main() {
    generators();
    identity();
    composition();
    dagger();
    tensor_product();
    simplification();
    braid_relation();
    mixed_absorption();
}
