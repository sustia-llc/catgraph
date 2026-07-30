//! `RModule<Dual<f64>>` module axioms + forward-mode derivative correctness
//! ([#74](https://github.com/sustia-llc/catgraph/issues/74) PR2).
//!
//! Runs only under `--features ad`; the default build compiles neither
//! `para::ad` nor this file.
//!
//! Two things are under test, and they are different in kind:
//!
//! 1. **The module axioms still hold when the scalar carries a derivative
//!    channel.** `Dual<f64>` is just another `S` for the #74 PR1 generic stack,
//!    so `zeros` / `basis` / `add` / `scale` / `direct_sum` must satisfy the
//!    same CDL Definition E.2 / Example G.3 identities they satisfy for `f64`.
//! 2. **The derivative channel is actually correct.** Seeding `du = 1` at
//!    coordinate `i` and evaluating a function whose analytic partial is known
//!    by hand must reproduce that partial exactly.
#![cfg(feature = "ad")]

use catgraph_dl::para::ad::{Dual, DualF64Module, gradient, seed};
use catgraph_dl::para::{F64Module, RModule};

/// Shorthand for a dual constant (`v + 0·ε`).
fn c(v: f64) -> Dual<f64> {
    Dual::constant(v)
}

/// The `R`-module axioms over the dual scalars — the `Dual<f64>` counterpart of
/// `common::assert_f64_module_axioms`.
///
/// Identities: `v + 0 = v`, `0 + v = v`, `1 · v = v`, `0 · v = 0`, the
/// dimension-mismatch guard on `add`, and basis coherence (`eᵢ` is `One` at `i`
/// and `Zero` elsewhere). CDL Definition E.2 / Example G.3.
#[test]
fn dual_module_axioms_hold() {
    let v: DualF64Module = RModule::new(vec![Dual::new(2.0, 1.0), Dual::new(-3.5, 0.0), c(7.25)]);
    let n = v.dim();
    let zero = DualF64Module::zeros(n);

    assert_eq!(
        v.add(&zero).as_ref(),
        Some(&v),
        "additive identity v + 0 = v"
    );
    assert_eq!(
        zero.add(&v).as_ref(),
        Some(&v),
        "additive identity 0 + v = v"
    );

    // `One::one()` for `Dual` is `1 + 0·ε`, i.e. `Dual::constant(1.0)`.
    assert_eq!(v.scale(c(1.0)), v, "scalar unit 1 · v = v");
    assert_eq!(v.scale(c(0.0)), zero, "scalar zero 0 · v = 0");

    let shorter = DualF64Module::zeros(n - 1);
    assert_eq!(
        v.add(&shorter),
        None,
        "addition rejects a dimension mismatch"
    );

    for i in 0..n {
        let e_i = DualF64Module::basis(n, i).expect("i < n so basis is defined");
        for (j, x) in e_i.as_slice().iter().enumerate() {
            let expected = if j == i { 1.0 } else { 0.0 };
            assert_eq!(x.value(), expected, "basis e_{i} value at coordinate {j}");
            assert_eq!(
                x.derivative(),
                0.0,
                "basis e_{i} carries no derivative at coordinate {j}"
            );
        }
    }
    assert_eq!(
        DualF64Module::basis(n, n),
        None,
        "basis rejects i == dim (out of range)"
    );
}

/// `direct_sum` concatenates dual coordinates left-block-first and is unital in
/// the zero-dimensional module — the `⊕`-monoid laws with a derivative channel
/// riding along. CDL Example E.4 / G.3.
#[test]
fn dual_direct_sum_is_a_unital_monoid() {
    let u: DualF64Module = RModule::new(vec![Dual::new(1.0, 0.5)]);
    let w: DualF64Module = RModule::new(vec![c(2.0), Dual::new(3.0, -1.0)]);
    let unit = DualF64Module::zero_dim();

    let uw = u.clone().direct_sum(w.clone());
    assert_eq!(uw.dim(), 3, "dimensions add under ⊕");
    assert_eq!(uw.as_slice()[0], Dual::new(1.0, 0.5), "left block first");
    assert_eq!(uw.as_slice()[2], Dual::new(3.0, -1.0), "right block last");

    assert_eq!(
        u.clone().direct_sum(unit.clone()),
        u,
        "right unit v ⊕ R⁰ = v"
    );
    assert_eq!(unit.direct_sum(u.clone()), u, "left unit R⁰ ⊕ v = v");
}

/// **Derivative correctness.** `seed` makes coordinate `i` the independent
/// variable and every other coordinate a constant, so the `ε` channel of any
/// function of the result is that function's partial `∂/∂vᵢ`.
#[test]
fn seed_marks_exactly_one_independent_variable() {
    let params = F64Module::new(vec![3.0, -1.0, 0.5]);
    let seeded = seed(&params, 1).expect("1 < dim so the seed is in range");

    for (j, x) in seeded.as_slice().iter().enumerate() {
        assert_eq!(
            x.value(),
            params.as_slice()[j],
            "seeding preserves the value at coordinate {j}"
        );
        let expected_du = if j == 1 { 1.0 } else { 0.0 };
        assert_eq!(x.derivative(), expected_du, "seed du at coordinate {j}");
    }

    assert_eq!(
        seed(&params, 3),
        None,
        "seed rejects i == dim (out of range)"
    );
}

/// **Derivative correctness against known analytic partials.**
///
/// `f(a, b, c) = a²·b + 3·c` has partials `∂f/∂a = 2ab`, `∂f/∂b = a²`,
/// `∂f/∂c = 3`, all exact in floating point at the chosen point, so the
/// assertions are exact equalities rather than tolerance comparisons.
#[test]
fn gradient_matches_the_analytic_partials() {
    // f(v) = v0² · v1 + 3 · v2
    let f = |v: &DualF64Module| -> Dual<f64> {
        let s = v.as_slice();
        s[0] * s[0] * s[1] + c(3.0) * s[2]
    };

    let params = F64Module::new(vec![2.0, 5.0, -4.0]);
    let grad = gradient(&params, f);

    assert_eq!(grad.dim(), 3, "the gradient has one partial per coordinate");
    assert_eq!(grad.as_slice()[0], 2.0 * 2.0 * 5.0, "∂f/∂a = 2ab = 20");
    assert_eq!(grad.as_slice()[1], 2.0 * 2.0, "∂f/∂b = a² = 4");
    assert_eq!(grad.as_slice()[2], 3.0, "∂f/∂c = 3");

    // The real part is untouched by seeding: f itself evaluates the same.
    let seeded = seed(&params, 0).expect("0 < dim");
    assert_eq!(
        f(&seeded).value(),
        2.0 * 2.0 * 5.0 + 3.0 * -4.0,
        "the value channel is the ordinary evaluation f(2,5,-4) = 8"
    );
}

/// A gradient of a quadratic vanishes exactly at its minimum — the property the
/// `gradient_descent_para` example's convergence claim rests on.
#[test]
fn gradient_vanishes_at_the_minimum_of_a_quadratic() {
    // g(v) = (v0 - 2)² + (v1 + 1)², minimised at (2, -1).
    let g = |v: &DualF64Module| -> Dual<f64> {
        let s = v.as_slice();
        let d0 = s[0] - c(2.0);
        let d1 = s[1] + c(1.0);
        d0 * d0 + d1 * d1
    };

    let at_min = F64Module::new(vec![2.0, -1.0]);
    let grad = gradient(&at_min, g);
    assert_eq!(grad.as_slice(), &[0.0, 0.0], "∇g vanishes at the minimum");

    let off_min = F64Module::new(vec![4.0, -1.0]);
    let grad_off = gradient(&off_min, g);
    assert_eq!(
        grad_off.as_slice()[0],
        4.0,
        "∂g/∂v0 = 2(v0 - 2) = 4 two units off the minimum"
    );
}
