//! Weight tying via the diagonal comonoid (Gavranović et al., ICML 2024,
//! CDL Theorem G.10 / §3.1).
//!
//! *Weight tying* — reusing one parameter tensor in several places of a network
//! — is, categorically, pre-composition by the **diagonal** of the comonoid
//! structure every object of `(Set, ×, 1)` carries: `δ : P → P × P`, `p ↦ (p, p)`.
//! The [`tie_weights`] helper packages this: given a 1-morphism whose parameter
//! is a *pair* `P × P`, it returns the 1-morphism with a single shared `P`.
//!
//! This example first exhibits the comonoid laws that make `δ` well-behaved, then
//! ties the weights of a concrete two-parameter morphism.
//!
//! Run with `cargo run -p catgraph-dl --example weight_tying`.

use catgraph_dl::para::{
    Comonoid, DiagonalComonoid, MonoidalCategory, ParaMorphism, SetActegory, SetMonoidal,
    tie_weights,
};

fn main() {
    let comonoid = DiagonalComonoid::<SetMonoidal>::new();
    let mono = SetMonoidal::new();

    // ---- The diagonal comonoid `(δ, ε)` ---------------------------------
    //
    // Comultiplication δ duplicates; counit ε discards.
    assert_eq!(comonoid.comultiply(9_i32), (9, 9));
    assert_eq!(comonoid.counit(9_i32), ());
    println!("comonoid: δ(p) = (p, p), ε(p) = ()");

    // Coassociativity: α ∘ (δ ⊗ id) ∘ δ = (id ⊗ δ) ∘ δ. Both collapse to the
    // triple `(p, (p, p))` once the associator re-brackets the left branch.
    let p = 4_i32;
    let dp = comonoid.comultiply(p);
    let left_branch = (comonoid.comultiply(dp.0), dp.1); // ((p, p), p)
    let right_branch = (dp.0, comonoid.comultiply(dp.1)); // (p, (p, p))
    assert_eq!(mono.associate::<i32, i32, i32>(left_branch), right_branch);

    // Counit laws: λ ∘ (ε ⊗ id) ∘ δ = id = ρ ∘ (id ⊗ ε) ∘ δ.
    let (l, r) = comonoid.comultiply(p);
    assert_eq!(mono.left_unitor::<i32>((comonoid.counit(l), r)), p);
    assert_eq!(mono.right_unitor::<i32>((l, comonoid.counit(r))), p);
    println!("comonoid laws: coassociativity + left/right counit hold");

    // ---- Tie the weights of a two-parameter morphism --------------------
    //
    // Untied: `(P × P, f) : X → Y` with f(((p1, p2), x)) = p1 + p2 + x — two
    // *independent* parameter slots. `tie_weights` shares one `P` across both.
    let untied: ParaMorphism<SetMonoidal, SetActegory, (i64, i64), _> =
        ParaMorphism::new((0_i64, 0_i64), |((p1, p2), x): ((i64, i64), i64)| {
            p1 + p2 + x
        });

    let tied = tie_weights::<SetActegory, i64, _, i64, i64>(3_i64, untied);

    // The tied morphism has a single parameter; the shared p flows to both slots,
    // so f'((p, x)) = p + p + x = 2p + x.
    assert_eq!(tied.parameter, 3_i64);
    assert_eq!((tied.action)((3_i64, 5_i64)), 11); // 3 + 3 + 5
    for (p, x) in [(3_i64, 5_i64), (0, 0), (-7, 4), (100, 100)] {
        assert_eq!((tied.action)((p, x)), 2 * p + x);
    }
    println!("tie_weights: (P×P, f) → (P, f'), f'((3, 5)) = 3 + 3 + 5 = 11");

    println!("weight_tying: all assertions passed");
}
