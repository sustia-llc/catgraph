//! Comonoid coherence laws + weight-tying smoke.
//!
//! CDL Theorem G.10. These tests exercise the `DiagonalComonoid` body for
//! the concrete monoidal category `(Set, ×, 1)` (i.e. [`SetMonoidal`]) and
//! the consumer-facing `tie_weights` helper used by
//! `catgraph-coalition`.
//!
//! ## Laws verified
//!
//! Let `δ = comultiply`, `ε = counit`, `α/λ/ρ` be the monoidal coherence
//! isomorphisms, and `id_P` be the identity on `P`.
//!
//! - **Coassociativity** — `α ∘ (δ ⊗ id_P) ∘ δ = (id_P ⊗ δ) ∘ δ`. In
//!   `(Set, ×, 1)` both sides equal the triple `(p, (p, p))` after
//!   threading through the associator. The first form before `α` produces
//!   `((p, p), p)`.
//! - **Left counit** — `λ ∘ (ε ⊗ id_P) ∘ δ = id_P`. With `δ(p) = (p, p)`
//!   and `ε(p) = ()`, the chain is `p ↦ (p, p) ↦ ((), p) ↦ p` (the final
//!   step is `λ`, the left unitor).
//! - **Right counit** — `ρ ∘ (id_P ⊗ ε) ∘ δ = id_P`. Symmetric:
//!   `p ↦ (p, p) ↦ (p, ()) ↦ p` via `ρ`.
//!
//! ## End-to-end
//!
//! `tie_weights` applied to `(P × P, λ((p1, p2), x). p1 + p2 + x)` with
//! `parameter_tied = 3` and input `x = 5` yields `3 + 3 + 5 = 11`.
//!
//! Each test consolidates several related assertions in one function (per
//! project TDD convention — quality over quantity).

#![allow(clippy::float_cmp)]

use catgraph_dl::para::{
    Comonoid, DiagonalComonoid, MonoidalCategory, ParaMorphism, SetActegory, SetMonoidal,
    tie_weights,
};
use proptest::prelude::*;

/// **Coassociativity** — `α ∘ (δ ⊗ id_P) ∘ δ = (id_P ⊗ δ) ∘ δ`.
///
/// In `(Set, ×, 1)`, with `δ(p) = (p, p)`:
///
/// - Pre-associator side: `(δ ⊗ id_P) ∘ δ : p ↦ (p, p) ↦ ((p, p), p)`.
/// - Right side: `(id_P ⊗ δ) ∘ δ : p ↦ (p, p) ↦ (p, (p, p))`.
/// - Applying `α : ((p, p), p) → (p, (p, p))` to the first equates them.
///
/// Proptest sweeps `i32` and `String` carriers. Also verifies a small
/// hand-picked `bool` case as a smoke check.
#[test]
fn diagonal_coassociativity_smoke() {
    let comonoid = DiagonalComonoid::<SetMonoidal>::new();
    let mono = SetMonoidal::new();

    // Hand smoke on bool — DiagonalComonoid is generic in `P: Clone`.
    let p = true;
    let dp = comonoid.comultiply(p);
    let left_branch = (comonoid.comultiply(dp.0), dp.1);
    let right_branch = (dp.0, comonoid.comultiply(dp.1));
    let left_after_alpha = mono.associate::<bool, bool, bool>(left_branch);
    assert_eq!(
        left_after_alpha, right_branch,
        "coassociativity failed on bool smoke"
    );
}

proptest! {
    /// Coassociativity on `i32`. Property: for every `p: i32`,
    /// `α((δ ⊗ id_P) ∘ δ (p)) == (id_P ⊗ δ) ∘ δ (p)`.
    #[test]
    fn diagonal_coassociativity_i32(p in any::<i32>()) {
        let comonoid = DiagonalComonoid::<SetMonoidal>::new();
        let mono = SetMonoidal::new();

        // (δ ⊗ id_P) ∘ δ (p) = ((p, p), p) — first build δ(p), then duplicate the left slot.
        let dp = comonoid.comultiply(p);
        let left_branch = (comonoid.comultiply(dp.0), dp.1);

        // (id_P ⊗ δ) ∘ δ (p) = (p, (p, p)) — duplicate the right slot.
        let dp_again = comonoid.comultiply(p);
        let right_branch = (dp_again.0, comonoid.comultiply(dp_again.1));

        // The two sides agree after applying the associator.
        let left_after_alpha = mono.associate::<i32, i32, i32>(left_branch);
        prop_assert_eq!(left_after_alpha, right_branch);
    }

    /// Coassociativity on `String` — exercises a non-`Copy` `Clone` carrier.
    /// Restricted to length ≤ 32 to keep test budget bounded.
    #[test]
    fn diagonal_coassociativity_string(p in "\\PC{0,32}") {
        let comonoid = DiagonalComonoid::<SetMonoidal>::new();
        let mono = SetMonoidal::new();

        let dp = comonoid.comultiply(p.clone());
        let left_branch = (comonoid.comultiply(dp.0), dp.1);

        let dp_again = comonoid.comultiply(p);
        let right_branch = (dp_again.0, comonoid.comultiply(dp_again.1));

        let left_after_alpha = mono.associate::<String, String, String>(left_branch);
        prop_assert_eq!(left_after_alpha, right_branch);
    }

    /// **Left counit** — `λ ∘ (ε ⊗ id_P) ∘ δ = id_P`.
    ///
    /// In `(Set, ×, 1)` the chain on input `p` is
    /// `p ↦ (p, p) ↦ ((), p) ↦ p` (final step is the left unitor).
    /// Property: for every `p: i32`, the round-trip is `p`. Also asserts
    /// the intermediate `((), p)` shape and that `ε` produces `()`.
    #[test]
    fn diagonal_left_counit_law(p in any::<i32>()) {
        let comonoid = DiagonalComonoid::<SetMonoidal>::new();
        let mono = SetMonoidal::new();

        // δ(p) = (p, p).
        let (left, right) = comonoid.comultiply(p);

        // (ε ⊗ id_P) — apply ε to the left slot.
        let after_eps_left: ((), i32) = (comonoid.counit(left), right);
        prop_assert_eq!(after_eps_left, ((), p));

        // λ : ((), p) → p.
        let recovered = mono.left_unitor::<i32>(after_eps_left);
        prop_assert_eq!(recovered, p);
    }

    /// **Right counit** — `ρ ∘ (id_P ⊗ ε) ∘ δ = id_P`.
    ///
    /// Symmetric to the left counit law. Chain: `p ↦ (p, p) ↦ (p, ()) ↦ p`.
    #[test]
    fn diagonal_right_counit_law(p in any::<i32>()) {
        let comonoid = DiagonalComonoid::<SetMonoidal>::new();
        let mono = SetMonoidal::new();

        let (left, right) = comonoid.comultiply(p);

        // (id_P ⊗ ε) — apply ε to the right slot.
        let after_eps_right: (i32, ()) = (left, comonoid.counit(right));
        prop_assert_eq!(after_eps_right, (p, ()));

        // ρ : (p, ()) → p.
        let recovered = mono.right_unitor::<i32>(after_eps_right);
        prop_assert_eq!(recovered, p);
    }
}

/// **End-to-end weight tying** — `tie_weights` collapses a paired-parameter
/// `Para` 1-morphism via the diagonal.
///
/// Per the task spec: starting with `(P × P, λ((p1, p2), x). p1 + p2 + x)`,
/// applying `tie_weights` with `parameter_tied = 3` and running on `x = 5`
/// produces `3 + 3 + 5 = 11`.
///
/// Asserts:
/// 1. The headline numeric case `(p, x) = (3, 5) ↦ 11`.
/// 2. The tied morphism's `parameter` field equals the supplied value.
/// 3. A small sweep — for several `(p, x)` pairs, the tied action returns
///    `2*p + x` (the closed form of `p + p + x`), confirming the diagonal
///    really did duplicate `p` into both slots.
/// 4. Cross-check against the untied action evaluated with `(p, p)` —
///    semantically equal.
#[test]
fn tie_weights_end_to_end_diagonal_smoke() {
    // Untied morphism `(P × P, f) : X → Y` with `X = Y = i64`,
    // `f(((p1, p2), x)) = p1 + p2 + x`.
    let untied: ParaMorphism<SetMonoidal, SetActegory, (i64, i64), _> =
        ParaMorphism::new((0_i64, 0_i64), |((p1, p2), x): ((i64, i64), i64)| {
            p1 + p2 + x
        });

    let tied = tie_weights::<SetActegory, i64, _, i64, i64>(3_i64, untied);

    // Headline.
    assert_eq!(tied.parameter, 3_i64);
    let headline: i64 = (tied.action)((3_i64, 5_i64));
    assert_eq!(headline, 11_i64, "tied action should yield 3 + 3 + 5 = 11");

    // Sweep.
    let baseline = |((p1, p2), x): ((i64, i64), i64)| p1 + p2 + x;
    for (p, x) in [(3_i64, 5_i64), (3, 0), (3, -5), (0, 0), (-7, 4), (100, 100)] {
        let tied_value: i64 = (tied.action)((p, x));
        let baseline_value = baseline(((p, p), x));
        assert_eq!(
            tied_value, baseline_value,
            "weight tying mismatch at (p, x) = ({p}, {x})"
        );
        assert_eq!(
            tied_value,
            2 * p + x,
            "tied closed-form (2p + x) failed at (p, x) = ({p}, {x})"
        );
    }
}
