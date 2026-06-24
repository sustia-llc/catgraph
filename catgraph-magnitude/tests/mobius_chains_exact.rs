//! §1.17 Leinster 2008 Cor 1.5 integer-exact Möbius inversion.
//!
//! Five paper-faithful fixtures + the `μ · ζ = I` recursion verifier.
//!
//! Paper anchor: Leinster 2008 *The Euler characteristic of a category*
//! (arXiv:0610260) §1.4 Cor 1.5 (page 6) + the proof of Prop 2.10 (p. 13).
//!
//! For finite skeletal 𝔸 with identity-only endomorphisms, the Möbius function
//! takes integer values:
//!
//! ```text
//! μ(a, b) = Σ_{n≥0} (−1)^n · #{nondegenerate n-paths from a to b}
//! ```
//!
//! Realised via `μ = Σ_{k=0}^K (−1)^k Mᵏ` where `M = ζ − I`. Per the proof of Prop 2.10 (p. 13)
//! the series terminates by `k = |objects|`; the algorithm early-terminates
//! when `Mᵏ` becomes the zero matrix.

use catgraph_magnitude::Z;
use catgraph_magnitude::mobius_chains::{
    mobius_function_via_chains_exact, verify_mobius_recursion,
};
use catgraph_magnitude::poset_category::PosetCategory;

#[test]
fn cor_1_5_chain_3_linear_poset() {
    // 3-chain 0 ≤ 1 ≤ 2. Phil Hall: μ = [[1, -1, 0], [0, 1, -1], [0, 0, 1]].
    let cat = PosetCategory::<u32>::from_partial_order(vec![0, 1, 2], |a, b| a <= b);
    let mu = mobius_function_via_chains_exact::<u32, Z>(&cat).unwrap();
    assert_eq!(mu.entries()[0][0], Z::from(1_i64));
    assert_eq!(mu.entries()[0][1], Z::from(-1_i64));
    assert_eq!(mu.entries()[0][2], Z::from(0_i64));
    assert_eq!(mu.entries()[1][1], Z::from(1_i64));
    assert_eq!(mu.entries()[1][2], Z::from(-1_i64));
    assert_eq!(mu.entries()[2][2], Z::from(1_i64));
    // Row 1, col 0 and row 2, col 0/1: strictly lower triangle of μ on an
    // upper-triangular ζ is zero.
    assert_eq!(mu.entries()[1][0], Z::from(0_i64));
    assert_eq!(mu.entries()[2][0], Z::from(0_i64));
    assert_eq!(mu.entries()[2][1], Z::from(0_i64));
    // FORWARD-LOOK §2.12 cross-check: closed-form Phil Hall asserted entry-by-entry
    // above; recursion verifier confirms μ · ζ = ζ · μ = I per Leinster 2008 Def 1.1.
    verify_mobius_recursion(&cat, &mu).expect("μ · ζ = ζ · μ = I on the 3-object linear chain");
}

#[test]
fn cor_1_5_diamond_lattice_2_squared() {
    // Boolean lattice 2²: 0=⊥, 1={a}, 2={b}, 3=⊤. Ordering: a ⊆ b ⇔ a & b == a.
    let cat = PosetCategory::<u32>::from_partial_order(vec![0, 1, 2, 3], |a, b| (*a & *b) == *a);
    let mu = mobius_function_via_chains_exact::<u32, Z>(&cat).unwrap();
    // Rota classical Möbius on 2² Boolean lattice:
    //   μ(⊥, ⊤) = (-1)^2 = 1.
    //   μ(⊥, atom) = -1 for atoms {a} and {b}.
    //   μ(atom, ⊤) = -1.
    assert_eq!(mu.entries()[0][3], Z::from(1_i64));
    assert_eq!(mu.entries()[0][1], Z::from(-1_i64));
    assert_eq!(mu.entries()[0][2], Z::from(-1_i64));
    assert_eq!(mu.entries()[1][3], Z::from(-1_i64));
    assert_eq!(mu.entries()[2][3], Z::from(-1_i64));
    // Diagonal = identity.
    assert_eq!(mu.entries()[0][0], Z::from(1_i64));
    assert_eq!(mu.entries()[3][3], Z::from(1_i64));
}

#[test]
fn cor_1_5_n_path_inj_2() {
    // 𝔻^inj_2: order-preserving injections {1,...,a} → {1,...,b}.
    // ζ(a, b) = C(b, a) (binomial coefficient).
    //
    // 3-object slice (a, b ∈ {1, 2, 3}):
    //   ζ(1, 1) = 1 (identity), ζ(1, 2) = C(2,1) = 2, ζ(1, 3) = C(3,1) = 3.
    //   ζ(2, 2) = 1, ζ(2, 3) = C(3,2) = 3.
    //   ζ(3, 3) = 1.
    //
    // Algebraic check via `verify_mobius_recursion` — μ · ζ = I exactly.
    let cat = PosetCategory::<u32>::from_arrow_counts(
        vec![1, 2, 3],
        vec![vec![1, 2, 3], vec![0, 1, 3], vec![0, 0, 1]],
    )
    .expect("upper-triangular ζ with unit diagonal is circuit-free");
    let mu = mobius_function_via_chains_exact::<u32, Z>(&cat).unwrap();
    verify_mobius_recursion(&cat, &mu).expect("μ · ζ ≠ I on n_path_inj_2 fixture");
}

#[test]
fn cor_1_5_terminal_object_recursion() {
    // Ex 1.11(c): 𝔸 has terminal object ⊤ ⇒ δ(-, ⊤) is a weighting on 𝔸.
    //
    // Concrete fixture: 4-object poset where only ⊤ = 3 has non-identity
    // predecessors. Closure: `(a ≤ b ∧ b == 3) ∨ a == b` (operator precedence:
    // `&&` binds tighter than `||`).
    let cat = PosetCategory::<u32>::from_partial_order(vec![0, 1, 2, 3], |a, b| {
        (a <= b && *b == 3) || a == b
    });
    let mu = mobius_function_via_chains_exact::<u32, Z>(&cat).unwrap();
    verify_mobius_recursion(&cat, &mu).expect("μ · ζ ≠ I on terminal-object fixture");
}

#[test]
fn cor_1_5_single_object() {
    // 1-object category: ζ = (1), μ = (1).
    let cat = PosetCategory::<u32>::from_arrow_counts(vec![42], vec![vec![1]])
        .expect("1×1 identity-only ζ is circuit-free");
    let mu = mobius_function_via_chains_exact::<u32, Z>(&cat).unwrap();
    assert_eq!(mu.entries()[0][0], Z::from(1_i64));
}
