//! `Para(SetMonoidal, SetActegory)` — building, composing, and reparameterizing
//! parametric 1-morphisms (Gavranović et al., ICML 2024, CDL §3.1).
//!
//! A `Para` 1-morphism `(P, f) : X → Y` bundles a **parameter object** `P` with
//! an action `f : P ▶ X → Y`. Over `(Set, ×, 1)` the action `▶` is the Cartesian
//! product, so `f` is just a closure `(P, X) → Y`. This example walks the three
//! core operations:
//!
//! 1. constructing 1-morphisms with [`ParaMorphism::new`],
//! 2. sequential composition `(P, f) ; (Q, g) = (Q ⊗ P, h)` via
//!    [`ParaMorphism::compose`], and
//! 3. reparameterization (pre-composition in the parameter) via
//!    [`Reparameterization`].
//!
//! Run with `cargo run -p catgraph-dl --example para_walkthrough`.

use catgraph_dl::para::{
    Actegory, MonoidalCategory, ParaMorphism, Reparameterization, SetActegory, SetMonoidal,
};

fn main() {
    // The monoidal base `(Set, ×, 1)` and its self-action on Set.
    let mono = SetMonoidal::new();
    let acteg = SetActegory::new();

    // The unit object is `()`; the action is the tuple `(p, x)`.
    assert_eq!(mono.unit(), ());
    assert_eq!(acteg.act(2_u32, 5_u32), (2_u32, 5_u32));
    println!("base: unit = (), act(p, x) = (p, x)");

    // ---- 1. Build two 1-morphisms ---------------------------------------
    //
    // (P = 2, f((p, x)) = p + x) : X → Y   — an affine "add the parameter".
    let f: ParaMorphism<SetMonoidal, SetActegory, i64, _> =
        ParaMorphism::new(2_i64, |(p, x): (i64, i64)| p + x);
    // (Q = 3, g((q, y)) = q * y) : Y → Z   — a "scale by the parameter".
    let g: ParaMorphism<SetMonoidal, SetActegory, i64, _> =
        ParaMorphism::new(3_i64, |(q, y): (i64, i64)| q * y);

    // ---- 2. Compose sequentially ----------------------------------------
    //
    // (P, f) ; (Q, g) = (Q ⊗ P, h) with h((q, p), x) = g((q, f((p, x)))).
    // The composite parameter is the *pair* (Q, P) = (3, 2).
    let composite = f.compose::<i64, _, i64, i64, i64>(g);
    assert_eq!(composite.parameter, (3_i64, 2_i64));

    // On input x = 5: f((2, 5)) = 7, then g((3, 7)) = 21 = 3 * (2 + 5).
    let y = (composite.action)(((3_i64, 2_i64), 5_i64));
    assert_eq!(y, 21);
    println!(
        "compose: (Q⊗P, h) parameter = {:?}, h((3,2), 5) = {y}",
        composite.parameter
    );

    // The action is a pure function of its argument: q * (p + x) everywhere.
    for (q, p, x) in [(3_i64, 2_i64, 0_i64), (3, 2, -4), (1, 0, 5), (-1, 5, 5)] {
        assert_eq!((composite.action)(((q, p), x)), q * (p + x));
    }

    // ---- 3. Reparameterize ----------------------------------------------
    //
    // A reparameterization pre-composes the parameter. Here the diagonal
    // Δ : P → (P, P), p ↦ (p, p) collapses a *paired* parameter into one — the
    // weight-tying 2-morphism (CDL Theorem G.10). Note the direction: Δ maps the
    // new (collapsed) parameter to the old (paired) one.
    let untied: ParaMorphism<SetMonoidal, SetActegory, (i64, i64), _> =
        ParaMorphism::new((5_i64, 5_i64), |((p1, p2), x): ((i64, i64), i64)| {
            p1 * x + p2
        });
    let diagonal: Reparameterization<SetMonoidal, _> = Reparameterization::new(|p: i64| (p, p));
    let tied = diagonal.apply::<SetActegory, i64, (i64, i64), _, i64, i64>(5_i64, untied);

    assert_eq!(tied.parameter, 5_i64);
    for x in [-5_i64, 0, 2, 100] {
        // f'((p, x)) = f(((p, p), x)) = p*x + p.
        assert_eq!((tied.action)((5_i64, x)), 5 * x + 5);
    }
    println!("reparameterize: diagonal tied (P×P) → P, f'((5, x)) = 5x + 5");

    println!("para_walkthrough: all assertions passed");
}
