//! Forward-mode automatic differentiation over the scalar-generic module stack
//! — **feature `ad`**, off by default
//! ([#74](https://github.com/sustia-llc/catgraph/issues/74)).
//!
//! ## What this module is, and what it is not
//!
//! [`RModule<S>`](super::RModule) became generic in its scalar ring in #74 PR1.
//! This module supplies one more `S`: [`Dual<T>`], the forward-mode dual number
//! `a + b·ε` with `ε² = 0`. Evaluating a function at
//! `Dual::variable(x₀) = x₀ + 1·ε` returns `f(x₀)` in the real part and `f'(x₀)`
//! in the `ε` part, exact to machine precision — the chain rule falls out of
//! `Dual`'s arithmetic impls.
//!
//! `Dual` drops in with **no adapter**: it implements catgraph's own
//! [`Zero`](catgraph_applied::rig::Zero) / [`One`](catgraph_applied::rig::One)
//! plus `Add` / `Mul` and derives `Clone` — exactly the per-method bound set
//! PR1's `RModule<S>` signatures ask for. Nothing here re-implements the module
//! structure; it parameterises it.
//!
//! **Honesty note on anchors.** Dual numbers are *not* a CDL construction — the
//! paper's differentiation content is a citation to the gradient-based-learning
//! literature, not a definition it develops. So this module claims no anchor of
//! its own: the paper anchors stay where they belong, on the module/actegory
//! layer it plugs into ([`RModule`] /
//! [`RMonoidal`](super::RMonoidal) — CDL
//! Definition E.2 / Example E.4 / Example G.3). The `examples/gradient_descent_para.rs`
//! walkthrough is where the CDL §3.1 `Para` reading of a gradient step is spelled
//! out.
//!
//! ## Where `Dual` lives
//!
//! [`Dual`] is defined in the sibling `para::dual` module and re-exported here,
//! so `para::ad` stays the single public entry point for the whole feature —
//! the path `catgraph_dl::para::ad::Dual` is unchanged from when the type came
//! from `deep_causality_num_dual` (#221). There is no longer an upstream crate
//! to seam against: the `ad` feature adds no dependency at all.

pub use super::dual::Dual;

use super::module_actegory::{F64Module, RModule};

/// A finite-dimensional module whose scalars carry a derivative channel —
/// `RModule<Dual<f64>>`, the `ad` counterpart of [`F64Module`].
///
/// Every `RModule` operation is available unchanged; the coordinates simply
/// propagate derivatives alongside values.
pub type DualF64Module = RModule<Dual<f64>>;

/// Lift a real module to the dual scalars, seeding coordinate `i` as **the**
/// independent variable.
///
/// Coordinate `i` becomes `Dual::variable(vᵢ) = vᵢ + 1·ε`; every other
/// coordinate becomes `Dual::constant(vⱼ) = vⱼ + 0·ε`. Evaluating any function
/// of the result therefore returns its partial derivative `∂/∂vᵢ` in the `ε`
/// channel — this is the one seeding step forward-mode AD is built on.
///
/// Returns `None` when `i` is out of range (`i >= params.dim()`), matching
/// [`RModule::basis`](super::RModule::basis)'s guard.
#[must_use]
pub fn seed(params: &F64Module, i: usize) -> Option<DualF64Module> {
    if i >= params.dim() {
        return None;
    }
    Some(RModule::new(
        params
            .as_slice()
            .iter()
            .enumerate()
            .map(|(j, &v)| {
                if j == i {
                    Dual::variable(v)
                } else {
                    Dual::constant(v)
                }
            })
            .collect(),
    ))
}

/// The gradient `∇loss(params)`, by forward-mode AD — one evaluation pass per
/// coordinate.
///
/// For each coordinate `i`, [`seed`]s `params` at `i` and reads the `ε` channel
/// of `loss`'s result, which is `∂loss/∂paramsᵢ` by the chain rule. The returned
/// module has the same dimension as `params`.
///
/// Forward mode costs one pass per input coordinate, so this is `params.dim()`
/// evaluations of `loss` — the right complexity for the low-dimensional
/// parameter spaces the examples use, and deliberately *not* the reverse-mode
/// (backpropagation) trade-off.
#[must_use]
pub fn gradient(
    params: &F64Module,
    mut loss: impl FnMut(&DualF64Module) -> Dual<f64>,
) -> F64Module {
    let partials = (0..params.dim())
        .map(|i| {
            let seeded =
                seed(params, i).expect("invariant: i < params.dim() so the seed is in range");
            loss(&seeded).derivative()
        })
        .collect();
    F64Module::new(partials)
}
