//! Leinster 2008 Cor 1.5 integer-exact Möbius — three-fixture demo.
//!
//! Runnable companion to `tests/mobius_chains_exact.rs`. Demonstrates the
//! `mobius_function_via_chains_exact<N, Q>` driver over
//! `Q = Z` (`BigInt` newtype): three paper-faithful fixtures
//! covering the canonical classical Möbius computations + the recursion
//! verifier `verify_mobius_recursion`.
//!
//! ## Paper anchor
//!
//! - **Leinster 2008** *The Euler characteristic of a category*
//!   (arXiv:0610260) §1.4 Cor 1.5 — integer Möbius via the chain-sum series
//!   `μ = Σ_{n≥0} (−1)^n M^n` with `M = ζ − I`.
//! - **Leinster 2008** Cor 1.5 implicit termination — the nondegenerate-path
//!   count vanishes for `k ≥ |𝔸|` on circuit-free 𝔸, so the series terminates
//!   by `k = |objects|`; the algorithm early-terminates when `M^k = 0`.
//! - **Leinster 2008** Ex 1.11(c) — a finite category with terminal object ⊤
//!   admits `δ(-, ⊤)` as a weighting; the column `μ(-, ⊤)` recovers the
//!   weights and `μ · ζ = I` confirms invertibility.
//!
//! The three fixtures recover the textbook closed forms:
//!
//! 1. **Linear poset 0 < 1 < 2** (Philip Hall classical Möbius on a chain):
//!    `μ(i, j) = (−1)^{j−i}` for `0 ≤ j − i ≤ 1`, else `0`.
//! 2. **Boolean lattice 2²** (Gian-Carlo Rota classical Möbius on the
//!    subset lattice): `μ(⊥, ⊤) = (−1)^|⊤| = 1`.
//! 3. **Terminal-object recursion** (Leinster 2008 Ex 1.11(c)): a 4-object
//!    poset with terminal ⊤ where only `μ · ζ = I` is checked algebraically.

use catgraph_magnitude::Z;
use catgraph_magnitude::mobius_chains::{
    mobius_function_via_chains_exact, verify_mobius_recursion,
};
use catgraph_magnitude::poset_category::PosetCategory;

fn main() {
    println!("Leinster 2008 Cor 1.5 integer-exact Möbius — 3-fixture demo\n");

    // (1) Linear poset 0 < 1 < 2 — Philip Hall classical Möbius on a chain.
    {
        let cat = PosetCategory::<u32>::from_partial_order(vec![0, 1, 2], |a, b| a <= b);
        let mu = mobius_function_via_chains_exact::<u32, Z>(&cat)
            .expect("μ via chain-sum on 3-chain succeeds (circuit-free, identity-only endo)");
        println!("Fixture 1: linear poset 0 < 1 < 2 (Philip Hall)");
        println!("  μ matrix:");
        for (i, row) in mu.entries().iter().enumerate() {
            println!("    row {i}: {row:?}");
        }
        verify_mobius_recursion(&cat, &mu).expect("μ · ζ = I on the 3-chain (Cor 1.5 recursion)");
        println!("  μ · ζ = I (verified)");
    }

    // (2) Boolean lattice 2² — Rota classical Möbius on the subset lattice.
    //     0 = ⊥, 1 = {a}, 2 = {b}, 3 = ⊤; ordering a ⊆ b ⇔ a & b == a.
    {
        let cat =
            PosetCategory::<u32>::from_partial_order(vec![0, 1, 2, 3], |a, b| (*a & *b) == *a);
        let mu = mobius_function_via_chains_exact::<u32, Z>(&cat)
            .expect("μ via chain-sum on 2² Boolean lattice succeeds");
        println!("\nFixture 2: 2² Boolean lattice (Rota)");
        println!(
            "  μ(⊥, ⊤) = {:?}  (expected 1; = (−1)^|⊤| = (−1)^2)",
            mu.entries()[0][3]
        );
        println!(
            "  μ(⊥, atom) = {:?}, {:?}  (expected −1, −1)",
            mu.entries()[0][1],
            mu.entries()[0][2]
        );
        verify_mobius_recursion(&cat, &mu).expect("μ · ζ = I on the 2² Boolean lattice");
        println!("  μ · ζ = I (verified)");
    }

    // (3) Terminal-object recursion — Leinster 2008 Ex 1.11(c).
    //     4-object poset where only ⊤ = 3 has non-identity predecessors.
    //     Predicate: (a ≤ b ∧ b == 3) ∨ a == b. Parentheses are required —
    //     `&&` binds tighter than `||`, but explicit grouping documents intent.
    {
        let cat = PosetCategory::<u32>::from_partial_order(vec![0, 1, 2, 3], |a, b| {
            (a <= b && *b == 3) || a == b
        });
        let mu = mobius_function_via_chains_exact::<u32, Z>(&cat)
            .expect("μ via chain-sum on terminal-object fixture succeeds");
        verify_mobius_recursion(&cat, &mu)
            .expect("μ · ζ = I on terminal-object fixture (Leinster 2008 Ex 1.11(c))");
        println!("\nFixture 3: terminal-object recursion (Leinster 2008 Ex 1.11(c))");
        println!("  μ · ζ = I (verified)");
    }

    println!("\nAll 3 fixtures verified.");
}
