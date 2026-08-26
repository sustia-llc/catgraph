//! Forward-mode automatic differentiation (feature `ad`, no dependency):
//! [`Dual<T>`] `a + b·ε`, `ε² = 0`, as a scalar for [`RModule<S>`](super::RModule).
//! Evaluating at `Dual::variable(x₀)` yields `f(x₀)` and `f'(x₀)`. Not a CDL
//! construction; `examples/gradient_descent_para.rs` gives the `Para` reading.

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
