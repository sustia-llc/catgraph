//! Phase DL-2 Agent A — `Para(SetMonoidal, SetActegory)` composition tests.
//!
//! CDL §3.1. These tests exercise the body landed in Phase DL-2 for the
//! concrete monoidal category `(Set, ×, 1)` acting on `Set` by Cartesian
//! product. They are the regression harness for:
//!
//! - Sequential composition `(P, f) ; (Q, g) = (Q ⊗ P, h)` with
//!   `h((q, p), x) = g((q, f((p, x))))`.
//! - Left and right unit laws against the unit object `()` and the
//!   identity-on-action `id : 1 × X → X` (`λ((), x). x`).
//! - Reparameterization pre-composition, including the diagonal
//!   `Δ : P → (P, P)` for weight tying (CDL Theorem G.10).
//!
//! Each test consolidates several related assertions in one function
//! (per project TDD convention — quality over quantity).

#![allow(clippy::float_cmp)]

use catgraph_dl::para::{
    Actegory, MonoidalCategory, ParaMorphism, Reparameterization, SetActegory, SetMonoidal,
};

/// **Left unit law** — composing with the unit-parameter identity on the
/// left collapses to the right operand.
///
/// Construct `id : (1, λ((), x). x) : X → X` and compose with
/// `(Q, g) : X → Z`. The composite has parameter `(Q, ())` (since the
/// `SetMonoidal` tensor is the tuple) and the result of evaluating it
/// matches `g((q, x))` — i.e. the original `g` applied directly. Also
/// asserts that for several `(q, x)` inputs the values match exactly.
///
/// Bonus assertion: the `SetMonoidal::left_unitor` collapses
/// `((), x) ↦ x`, which is the underlying action of the identity used here.
#[test]
fn left_unit_law_collapses_to_right_operand() {
    let mono = SetMonoidal::new();
    let acteg = SetActegory::new();

    // SetMonoidal sanity: unit() == (), left_unitor((), x) == x.
    assert_eq!(mono.unit(), ());
    assert_eq!(mono.left_unitor::<i64>(((), 7_i64)), 7_i64);

    // SetActegory sanity: act(p, x) == (p, x).
    assert_eq!(acteg.act(2_u32, 5_u32), (2_u32, 5_u32));

    // Identity Para morphism on X = i64 with parameter object 1 = ().
    let id_para: ParaMorphism<SetMonoidal, SetActegory, (), _> =
        ParaMorphism::new((), |((), x): ((), i64)| x);

    // (Q, g) : X → Z with Q = i64, g((q, x)) = q + x.
    let g_para: ParaMorphism<SetMonoidal, SetActegory, i64, _> =
        ParaMorphism::new(10_i64, |(q, x): (i64, i64)| q + x);

    // (Q, g) ∘ id_para — id is on the left of the composition pipeline,
    // so `id_para.compose(g_para)` produces (Q ⊗ 1, h) which on Set is
    // (i64, ()) parameter and h(((q, ()), x)) = g((q, id((),x))) = q + x.
    let composite = id_para.compose::<i64, _, i64, i64, i64>(g_para);
    assert_eq!(composite.parameter, (10_i64, ()));

    for (q, x) in [(10_i64, 5_i64), (10, -3), (10, 0), (10, 100)] {
        // The composite forces parameter = (10, ()); apply uses that, so we
        // reconstruct an action evaluation through the stored closure.
        let z: i64 = (composite.action)(((q, ()), x));
        assert_eq!(z, q + x, "left unit failed for (q, x) = ({q}, {x})");
    }
}

/// **Right unit law** — composing with the unit-parameter identity on the
/// right collapses to the left operand.
///
/// Construct `(P, f) : X → Y` and compose with
/// `id : (1, λ((), y). y) : Y → Y`. The composite has parameter
/// `((), P)` and produces `f((p, x))` directly.
#[test]
fn right_unit_law_collapses_to_left_operand() {
    let mono = SetMonoidal::new();

    // SetMonoidal right_unitor sanity: (a, ()) ↦ a.
    assert_eq!(mono.right_unitor::<i64>((42_i64, ())), 42_i64);

    // (P, f) : X → Y with P = u32, f((p, x)) = p as i64 + x.
    let f_para: ParaMorphism<SetMonoidal, SetActegory, u32, _> =
        ParaMorphism::new(7_u32, |(p, x): (u32, i64)| i64::from(p) + x);

    // id_Y on Y = i64 with parameter 1 = ().
    let id_y: ParaMorphism<SetMonoidal, SetActegory, (), _> =
        ParaMorphism::new((), |((), y): ((), i64)| y);

    let composite = f_para.compose::<(), _, i64, i64, i64>(id_y);
    assert_eq!(composite.parameter, ((), 7_u32));

    for (p, x) in [(7_u32, 5_i64), (7, -1), (7, 0), (7, 100)] {
        let z: i64 = (composite.action)((((), p), x));
        assert_eq!(
            z,
            i64::from(p) + x,
            "right unit failed for (p, x) = ({p}, {x})"
        );
    }
}

/// **Sequential composition correctness** — small numeric example from the
/// task spec.
///
/// `(P = 2, f((p, x)) = p + x) : X → Y` composed with
/// `(Q = 3, g((q, y)) = q * y) : Y → Z`. For input `x = 5`:
///
/// - `f((2, 5)) = 7`, then `g((3, 7)) = 21`.
/// - The composed parameter is `(Q, P) = (3, 2)`.
/// - The composed action evaluates to `21` on input `((3, 2), 5)`.
///
/// Asserts (a) the parameter pair, (b) the value match, (c) several other
/// inputs produce the expected closed-form `q * (p + x)`.
#[test]
fn sequential_composition_numeric_smoke() {
    let f_para: ParaMorphism<SetMonoidal, SetActegory, i64, _> =
        ParaMorphism::new(2_i64, |(p, x): (i64, i64)| p + x);

    let g_para: ParaMorphism<SetMonoidal, SetActegory, i64, _> =
        ParaMorphism::new(3_i64, |(q, y): (i64, i64)| q * y);

    let composite = f_para.compose::<i64, _, i64, i64, i64>(g_para);
    assert_eq!(composite.parameter, (3_i64, 2_i64));

    // Headline: q * (p + x) for (q, p, x) = (3, 2, 5) is 3 * 7 = 21.
    assert_eq!((composite.action)(((3_i64, 2_i64), 5_i64)), 21_i64);

    // Sweep small grid — composite should equal q * (p + x) at every
    // (q, p, x). Note: composite.parameter is fixed to (3, 2); we vary
    // the closure argument independently to test the action-as-pure-fn.
    for (q, p, x) in [
        (3_i64, 2_i64, 0_i64),
        (3, 2, 1),
        (3, 2, -4),
        (3, 2, 100),
        (1, 0, 5),
        (-1, 5, 5),
    ] {
        let z = (composite.action)(((q, p), x));
        assert_eq!(z, q * (p + x), "composite failed at (q,p,x)=({q},{p},{x})");
    }
}

/// **Reparameterization triangle (weight tying via the diagonal)** —
/// CDL Theorem G.10.
///
/// Given a 1-morphism `(P × P, f) : X → Y` with `f(((p1, p2), x)) =
/// p1 * x + p2`, applying the diagonal `Δ : P → (P, P), Δ(p) = (p, p)` as
/// a 2-morphism produces the weight-tied 1-morphism `(P, f')` with
/// `f'((p, x)) = f(((p, p), x)) = p * x + p`.
///
/// Asserts:
/// 1. The reparameterised morphism has parameter `P = i64` (caller-
///    supplied, here `5`).
/// 2. On a sweep of `x` values, `f'((5, x)) == 5*x + 5`.
/// 3. The "untied" baseline `f(((5, 5), x))` equals the "tied" value,
///    confirming the diagonal really does collapse the two slots
///    pointwise — i.e. the triangle commutes.
#[test]
fn reparameterization_diagonal_implements_weight_tying() {
    // Untied morphism `(P × P, f) : X → Y` where X = Y = i64.
    let untied: ParaMorphism<SetMonoidal, SetActegory, (i64, i64), _> =
        ParaMorphism::new((5_i64, 5_i64), |((p1, p2), x): ((i64, i64), i64)| {
            p1 * x + p2
        });

    // Diagonal Δ : i64 → (i64, i64), p ↦ (p, p). This is the 2-morphism
    // `(P, f') ⇒ (P × P, f)` in CDL §3.1 — note the *direction*: Δ goes
    // from the new (collapsed) parameter to the old (paired) one.
    let diagonal: Reparameterization<SetMonoidal, _> = Reparameterization::new(|p: i64| (p, p));

    let tied = diagonal.apply::<SetActegory, i64, (i64, i64), _, i64, i64>(5_i64, untied);

    assert_eq!(tied.parameter, 5_i64);

    // Sweep — compare against both the closed form and the explicit
    // baseline (which exercises the un-collapsed action separately).
    let baseline = |(pp, x): ((i64, i64), i64)| pp.0 * x + pp.1;
    for x in [-5_i64, -1, 0, 1, 2, 7, 100] {
        let tied_value: i64 = (tied.action)((5_i64, x));
        let baseline_value = baseline(((5_i64, 5_i64), x));
        assert_eq!(
            tied_value, baseline_value,
            "weight-tying triangle failed at x = {x}"
        );
        assert_eq!(
            tied_value,
            5 * x + 5,
            "weight-tying closed-form failed at x = {x}"
        );
    }
}

/// **Coherence isomorphism `μ` directly** — sanity check on
/// [`SetActegory::compose_action`].
///
/// `μ : Q ▶ (P ▶ X) → (Q ⊗ P) ▶ X` should be the tuple re-association
/// `(q, (p, x)) ↦ ((q, p), x)`. Confirms the actegory body wired to the
/// `SetMonoidal` tensor matches what `ParaMorphism::compose` relies on.
#[test]
fn set_actegory_compose_action_reassociates_tuple() {
    let acteg = SetActegory::new();

    // act(p, x) == (p, x) — Cartesian-product action on Set.
    assert_eq!(acteg.act(7_u32, 13_i64), (7_u32, 13_i64));

    // compose_action(q, p, x) == ((q, p), x) — μ as exact tuple
    // re-association in Set.
    assert_eq!(
        acteg.compose_action(2_u32, 3_u32, 5_i64),
        ((2_u32, 3_u32), 5_i64)
    );

    // Cross-check against the SetMonoidal associator on the parameter:
    // the (q, p) slot of compose_action's output equals SetMonoidal's
    // tensor_objects(q, p).
    let mono = SetMonoidal::new();
    let ((q_seen, p_seen), _) = acteg.compose_action(2_u32, 3_u32, 5_i64);
    assert_eq!(mono.tensor_objects(q_seen, p_seen), (2_u32, 3_u32));
}
