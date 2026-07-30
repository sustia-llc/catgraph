//! Gradient descent as a `Para` reparameterization, with exact forward-mode
//! derivatives (Gavranović et al., ICML 2024, CDL §3.1 + Example G.3).
//!
//! Requires the off-by-default `ad` feature
//! ([#74](https://github.com/sustia-llc/catgraph/issues/74)):
//!
//! ```text
//! cargo run -p catgraph-dl --features ad --example gradient_descent_para
//! ```
//!
//! ## The categorical reading
//!
//! A `Para` 1-morphism `(P, f) : X → Y` is a parameter object `P` together with
//! an action `f : P ▶ X → Y` (CDL §3.1). Here `P` is `F64Module` — a genuine
//! `R`-module of parameters — `X = Y = f64`, and `f` is a linear model
//! `f((w, x)) = w₀·x + w₁`. CDL §3.1 notes that this 2-category "is one of the
//! key components in the categorical picture of gradient-based learning
//! (Cruttwell et al., 2022)", and Example G.3 is the `Para(Smooth)` instance
//! over real vector spaces that reading lives in.
//!
//! A **2-morphism** of `Para` is a reparameterization `r : P' → P` (CDL §3.1:
//! "the 2-morphisms in Para capture *reparameterisations* between parametric
//! functions"), and [`Reparameterization::apply`] pre-composes it into the
//! parameter slot: `f'((p', x)) = f((r(p'), x))`. One gradient-descent step is
//! exactly such an `r` — the endo-reparameterization
//! `r(w) = w ⊖ lr · ∇L(w)` — so the trained model is literally the original
//! model reparameterized by the update map. That is what step 3 below builds.
//!
//! ## Where the derivatives come from
//!
//! `∇L` is computed by **forward-mode AD**, not by a hand-written derivative:
//! [`gradient`] seeds one coordinate at a time with `du = 1` and reads the `ε`
//! channel of the result. The loss is written **once**, generically over the
//! scalar `S`, and runs unchanged at `S = f64` (to report the loss) and at
//! `S = Dual<f64>` (to differentiate it) — which is the whole point of the #74
//! PR1 genericization of `RModule<S>`.
//!
//! ## What is asserted
//!
//! The target is a least-squares fit to exactly-linear data, so the optimum is
//! known in closed form: `(w₀, w₁) = (2, -1)` with loss `0`. The example checks
//! that forward-mode AD reproduces the analytic gradient, that one step through
//! the `Reparameterization` surface agrees with the direct update, that the
//! loss decreases strictly at every iteration, and that the run converges to
//! the known optimum.

use core::ops::{Add, Mul, Sub};

use catgraph_dl::para::ad::{Dual, DualF64Module, gradient, seed};
use catgraph_dl::para::{
    F64Module, ParaMorphism, RModule, Reparameterization, SetActegory, SetMonoidal,
};

/// Exactly-linear samples `(x, y)` from `y = 2x - 1`, so the least-squares
/// optimum is `(2, -1)` with residual zero — a simple quadratic target with a
/// known minimum.
const SAMPLES: [(f64, f64); 4] = [(0.0, -1.0), (1.0, 1.0), (2.0, 3.0), (3.0, 5.0)];

/// The learning rate. The loss Hessian here is `2·[[14, 6], [6, 4]]`, whose
/// largest eigenvalue is ≈ 33.6, so gradient descent is stable and monotone for
/// `lr < 2/33.6 ≈ 0.0595`.
const LEARNING_RATE: f64 = 0.02;

const STEPS: usize = 500;

/// The model action `f((w, x)) = w₀·x + w₁`, written generically in the scalar
/// so it can be evaluated at `f64` *and* at `Dual<f64>`.
fn predict<S>(params: &RModule<S>, x: S) -> S
where
    S: Clone + Add<Output = S> + Mul<Output = S>,
{
    let w = params.as_slice();
    w[0].clone() * x + w[1].clone()
}

/// Total squared error `L(w) = Σⱼ (f((w, xⱼ)) - yⱼ)²`, generic in the scalar.
///
/// Returns `None` on an empty dataset — folding without a `Zero` bound keeps
/// the scalar requirements down to exactly the arithmetic actually used.
fn squared_error<S>(params: &RModule<S>, samples: &[(S, S)]) -> Option<S>
where
    S: Clone + Add<Output = S> + Mul<Output = S> + Sub<Output = S>,
{
    samples
        .iter()
        .map(|(x, y)| {
            let residual = predict(params, x.clone()) - y.clone();
            residual.clone() * residual
        })
        .reduce(|a, b| a + b)
}

/// The dataset lifted to constant duals — the inputs carry no derivative, only
/// the parameters do.
fn dual_samples() -> Vec<(Dual<f64>, Dual<f64>)> {
    SAMPLES
        .iter()
        .map(|&(x, y)| (Dual::constant(x), Dual::constant(y)))
        .collect()
}

/// `L` at real scalars — the reported loss.
fn loss_at(params: &F64Module) -> f64 {
    squared_error(params, &SAMPLES).expect("invariant: SAMPLES is non-empty")
}

/// `∇L(w)` by forward-mode AD: one pass per parameter coordinate.
fn loss_gradient(params: &F64Module) -> F64Module {
    let data = dual_samples();
    gradient(params, |dual_params: &DualF64Module| {
        squared_error(dual_params, &data).expect("invariant: SAMPLES is non-empty")
    })
}

/// One gradient-descent step `w ↦ w ⊖ lr · ∇L(w)`, expressed in the `R`-module
/// structure of the parameter object (`scale` then `add`).
fn descent_step(params: &F64Module) -> F64Module {
    let update = loss_gradient(params).scale(-LEARNING_RATE);
    params
        .add(&update)
        .expect("invariant: the gradient has the same dimension as the parameters")
}

fn main() {
    // ---- 1. The Para 1-morphism ------------------------------------------
    //
    // (P, f) : X → Y with P = F64Module (the parameter module), X = Y = f64,
    // and f((w, x)) = w₀·x + w₁. CDL §3.1.
    let initial = F64Module::new(vec![0.0, 0.0]);
    let model: ParaMorphism<SetMonoidal, SetActegory, F64Module, _> =
        ParaMorphism::new(initial.clone(), |(w, x): (F64Module, f64)| -> f64 {
            predict(&w, x)
        });
    assert_eq!(model.parameter, initial);
    assert_eq!((model.action)((F64Module::new(vec![2.0, -1.0]), 3.0)), 5.0);
    println!("para: (P = F64Module, f((w, x)) = w0*x + w1), f((2,-1), 3) = 5");

    // ---- 2. Forward-mode AD reproduces the analytic gradient -------------
    //
    // L(w) = Σ (w₀·xⱼ + w₁ - yⱼ)², so
    //   ∂L/∂w₀ = Σ 2·xⱼ·(w₀·xⱼ + w₁ - yⱼ),
    //   ∂L/∂w₁ = Σ 2·(w₀·xⱼ + w₁ - yⱼ).
    // At w = (0, 0) the residuals are -yⱼ, giving
    //   ∂L/∂w₀ = -2·Σ xⱼyⱼ = -2·22 = -44   and   ∂L/∂w₁ = -2·Σ yⱼ = -2·8 = -16.
    let g0 = loss_gradient(&initial);
    assert_eq!(g0.as_slice(), &[-44.0, -16.0], "∇L(0,0) by forward-mode AD");

    // The same numbers, computed the long way round from the definition.
    let analytic: Vec<f64> = {
        let residuals: Vec<f64> = SAMPLES
            .iter()
            .map(|&(x, y)| predict(&initial, x) - y)
            .collect();
        let d0 = residuals
            .iter()
            .zip(SAMPLES)
            .map(|(r, (x, _))| 2.0 * x * r)
            .sum();
        let d1 = residuals.iter().map(|r| 2.0 * r).sum();
        vec![d0, d1]
    };
    assert_eq!(
        g0.as_slice(),
        analytic.as_slice(),
        "AD matches the analytic gradient"
    );

    // Seeding marks exactly one independent variable.
    let seeded = seed(&initial, 0).expect("0 < dim so the seed is in range");
    assert_eq!(
        seeded.as_slice()[0].derivative(),
        1.0,
        "coordinate 0 is the variable"
    );
    assert_eq!(
        seeded.as_slice()[1].derivative(),
        0.0,
        "coordinate 1 is constant"
    );
    println!("forward-mode: ∇L(0,0) = [-44, -16], matches the analytic partials");

    // ---- 3. One step, as a Para 2-morphism -------------------------------
    //
    // A reparameterization r : P' → P pre-composes the parameter slot. Taking
    // r = one descent step makes the stepped model literally "the model
    // reparameterized by the update map": f'((w, x)) = f((r(w), x)).
    let step: Reparameterization<SetMonoidal, _> =
        Reparameterization::new(|w: F64Module| descent_step(&w));
    let stepped_model =
        step.apply::<SetActegory, F64Module, F64Module, _, f64, f64>(initial.clone(), model);

    // Evaluating the reparameterized morphism at the *old* parameter gives the
    // model with the *new* parameters — same thing the direct update computes.
    let after_one = descent_step(&initial);
    for x in [-2.0_f64, 0.0, 1.5, 4.0] {
        assert_eq!(
            (stepped_model.action)((initial.clone(), x)),
            predict(&after_one, x),
            "the 2-morphism agrees with the direct update at x = {x}"
        );
    }
    assert!(
        loss_at(&after_one) < loss_at(&initial),
        "one reparameterization step decreases the loss"
    );
    println!(
        "2-morphism: r(w) = w - lr*∇L(w); loss {:.4} → {:.4} after one step",
        loss_at(&initial),
        loss_at(&after_one)
    );

    // ---- 4. Descend, and check strict decrease + convergence -------------
    let mut params = initial.clone();
    let mut loss = loss_at(&params);
    let initial_loss = loss;

    for k in 0..STEPS {
        let next = descent_step(&params);
        let next_loss = loss_at(&next);
        assert!(
            next_loss < loss,
            "loss strictly decreases at step {k}: {next_loss} !< {loss}"
        );
        params = next;
        loss = next_loss;
    }

    // The optimum is exact: y = 2x - 1 fits the data with zero residual.
    let optimum = F64Module::new(vec![2.0, -1.0]);
    assert_eq!(loss_at(&optimum), 0.0, "the known optimum has zero loss");

    let gap: Vec<f64> = params
        .as_slice()
        .iter()
        .zip(optimum.as_slice())
        .map(|(p, o)| (p - o).abs())
        .collect();
    assert!(
        gap.iter().all(|d| *d < 1e-6),
        "converged to the known optimum (2, -1); gap = {gap:?}"
    );
    assert!(
        loss < initial_loss * 1e-9,
        "the loss collapsed by nine orders of magnitude: {initial_loss} → {loss}"
    );

    // At the optimum the gradient vanishes — the fixed point of the
    // reparameterization is the trained model.
    let g_star = loss_gradient(&optimum);
    assert_eq!(g_star.as_slice(), &[0.0, 0.0], "∇L vanishes at the optimum");

    println!(
        "descent: {STEPS} steps, loss {initial_loss:.4} → {loss:.3e}, w = [{:.9}, {:.9}]",
        params.as_slice()[0],
        params.as_slice()[1]
    );
    println!("gradient_descent_para: all assertions passed");
}
