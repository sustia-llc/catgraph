//! Interpreter for free-prop terms (Phase S3).
//!
//! [`eval`] runs a [`PropExpr<G>`](catgraph_applied::prop::PropExpr) as a
//! wire-bundle transformer: given a model that says how each *generator* acts on
//! a bundle of values, it folds that action through the term's structure
//! (identity, braiding, composition, tensor). This is the term-action of Seven
//! Sketches Def 5.25 made executable — the free prop `Free(G)` is the syntax,
//! an [`ArrowModel`] is a semantics, and `eval` is the unique prop-functor
//! extending the model along the generators.
//!
//! # Wire bundles are `Vec<Value>`, sized by arity
//!
//! A morphism `m → n` consumes a bundle of `m` values and produces `n`. The
//! interpreter tracks that invariant explicitly: [`eval`] checks the input
//! length against `expr.source()` at every node, so a mismatch (a directly
//! constructed ill-formed term, or a misbehaving model) surfaces as a
//! [`SyntaxError::WireCount`] rather than a panic. The braiding
//! [`PropExpr::Braid(m, n)`](catgraph_applied::prop::PropExpr::Braid)
//! block-rotates the bundle `[a | b]` (with `|a| = m`, `|b| = n`) to `[b | a]`,
//! matching the block-swap permutation matrix of the Thm 5.53 functor.
//!
//! # No `Clone` on the wire values
//!
//! [`eval`] never requires `Value: Clone`. Duplicating a wire is a *model*
//! concern, not an engine one: the only way to fan a value out is for a
//! generator to do it (the SFG [`Copy`](catgraph_applied::sfg::SfgGenerator::Copy)
//! below returns `[x.clone(), x]`). This keeps the interpreter honest about the
//! distinction between the Cartesian diagonal (`Fanout`, copy-is-free in `Set`)
//! and a Frobenius comultiplication `δ` (a structure map a model must *supply*)
//! — the interpreter grants neither for free; a model that wants copying must
//! carry the `Clone` itself. [`SfgModel`] is exactly such a model.
//!
//! # Depth
//!
//! [`eval`] recurses once per tree level with no depth bound, matching the
//! structural recursion of `PropExpr` itself (and the [printer](mod@crate::text::print)).
//! Pathologically deep terms (tens of thousands of nested nodes) can exhaust the
//! stack; this is an engine-wide property of the term representation, not
//! specific to the interpreter. The [S2 parser](mod@crate::text::parse) bounds input
//! nesting explicitly (untrusted text).
//!
//! # Relation to the matrix functor (Def 5.50 / Remark 5.49)
//!
//! For the SFG signature, [`eval`] under [`SfgModel`] computes exactly the
//! **row-vector action** `x ↦ x · S(e)` of the Thm 5.53 matrix functor `S`,
//! where a morphism `m → n` is the `m × n` matrix (rows = domain arity). Feeding
//! the `i`-th standard basis vector therefore reproduces **row `i`** of the
//! matrix — the law the S3 cross-check proptest pins.

use std::marker::PhantomData;

use catgraph_applied::prop::{PropExpr, PropSignature};
use catgraph_applied::rig::Rig;
use catgraph_applied::sfg::SfgGenerator;

use crate::errors::SyntaxError;

/// A semantics for a prop signature: how each generator acts on a bundle of
/// wire values.
///
/// An `ArrowModel` supplies the *only* non-structural piece [`eval`] needs — the
/// action on generators. Identity, braiding, composition, and tensor are fixed
/// by the free-prop structure and handled by the engine.
///
/// # Contract
///
/// [`apply_generator`](ArrowModel::apply_generator) **must** return exactly
/// `generator.target()` values when given exactly `generator.source()` inputs.
/// [`eval`] enforces both halves: it hands the generator exactly its source
/// arity, and rejects a return bundle of the wrong length with
/// [`SyntaxError::ModelArity`]. A model is free to be partial in *value* (return
/// an [`Err`]) but not in *shape*.
pub trait ArrowModel<G: PropSignature> {
    /// The type carried on each wire.
    type Value;

    /// Apply the model's action for `generator` to its `inputs` bundle.
    ///
    /// # Errors
    ///
    /// Returns a [`SyntaxError`] if the model's action fails. Implementations
    /// should return exactly `generator.target()` values; a wrong count is
    /// reported by [`eval`] as [`SyntaxError::ModelArity`].
    fn apply_generator(
        &self,
        generator: &G,
        inputs: Vec<Self::Value>,
    ) -> Result<Vec<Self::Value>, SyntaxError>;
}

/// Name the kind of node a wire-count mismatch was detected at, for diagnostics.
fn node_kind<G: PropSignature>(expr: &PropExpr<G>) -> &'static str {
    match expr {
        PropExpr::Identity(_) => "id",
        PropExpr::Braid(_, _) => "braid",
        PropExpr::Generator(_) => "generator",
        PropExpr::Compose(_, _) => "compose",
        PropExpr::Tensor(_, _) => "tensor",
    }
}

/// Evaluate `expr` on an `input` wire bundle under `model`.
///
/// The engine is structural: `Identity(n)` passes its `n` wires through,
/// `Braid(m, n)` block-rotates `[a | b] ↦ [b | a]`, `Compose(f, g)` pipes `f`'s
/// output into `g`, `Tensor(f, g)` splits the input at `f.source()` and
/// concatenates the two sub-results, and `Generator(g)` delegates to
/// [`ArrowModel::apply_generator`]. The input length is checked against
/// `expr.source()` at every node, so the recursion never indexes out of bounds.
///
/// # Errors
///
/// - [`SyntaxError::WireCount`] if `input.len() != expr.source()` (top level, or
///   at an interior node of a directly-constructed ill-formed term).
/// - [`SyntaxError::ModelArity`] if `model` returns a bundle whose length is not
///   `generator.target()`.
/// - Any [`SyntaxError`] the model itself raises from
///   [`apply_generator`](ArrowModel::apply_generator).
pub fn eval<G, M>(
    expr: &PropExpr<G>,
    model: &M,
    input: Vec<M::Value>,
) -> Result<Vec<M::Value>, SyntaxError>
where
    G: PropSignature,
    M: ArrowModel<G>,
{
    if input.len() != expr.source() {
        return Err(SyntaxError::WireCount {
            expected: expr.source(),
            actual: input.len(),
            context: node_kind(expr),
        });
    }
    match expr {
        // `id_n`: the n wires pass through untouched.
        PropExpr::Identity(_) => Ok(input),
        // `σ_{m,n}`: block-rotate `[a | b]` (|a| = m, |b| = n) to `[b | a]`.
        PropExpr::Braid(m, _) => {
            let mut a = input;
            let b = a.split_off(*m);
            let mut out = b;
            out.extend(a);
            Ok(out)
        }
        // Delegate to the model, then check it returned the promised arity.
        PropExpr::Generator(g) => {
            let outputs = model.apply_generator(g, input)?;
            if outputs.len() != g.target() {
                return Err(SyntaxError::ModelArity {
                    generator: format!("{g:?}"),
                    expected: g.target(),
                    actual: outputs.len(),
                });
            }
            Ok(outputs)
        }
        // `f ; g`: pipe f's output bundle into g.
        PropExpr::Compose(f, g) => {
            let mid = eval(f, model, input)?;
            eval(g, model, mid)
        }
        // `f ⊗ g`: split at f's source arity, evaluate both, concatenate.
        PropExpr::Tensor(f, g) => {
            let mut left_in = input;
            let right_in = left_in.split_off(f.source());
            let mut left_out = eval(f, model, left_in)?;
            let right_out = eval(g, model, right_in)?;
            left_out.extend(right_out);
            Ok(left_out)
        }
    }
}

/// The R-linear semantics of the signal-flow-graph signature: an
/// [`ArrowModel`] for [`SfgGenerator<R>`] with `Value = R` (Seven Sketches
/// Def 5.45 / Eq 5.52).
///
/// Under [`eval`] this computes the row-vector action `x ↦ x · S(e)` of the
/// Thm 5.53 matrix functor `S` (see the module docs). The generator actions are:
///
/// - [`Copy`](SfgGenerator::Copy) `1 → 2`: `x ↦ [x, x]` (the `Clone` lives
///   here, in the model — never in [`eval`]);
/// - [`Discard`](SfgGenerator::Discard) `1 → 0`: `x ↦ []`;
/// - [`Add`](SfgGenerator::Add) `2 → 1`: `[a, b] ↦ [a + b]`;
/// - [`Zero`](SfgGenerator::Zero) `0 → 1`: `[] ↦ [R::zero()]`;
/// - [`Scalar(r)`](SfgGenerator::Scalar) `1 → 1`: `x ↦ [x · r]`.
///
/// # Scalar multiplication order
///
/// `Scalar(r)` multiplies the **wire value on the left, the scalar on the
/// right** (`x * r`), matching the row-vector convention `x · [[r]]` and the
/// left-to-right factor order of [`MatR::matmul`](catgraph_applied::mat::MatR)
/// (`entries[i][b] * entries[b][c]`). For the shipped rigs (all commutative)
/// `x * r == r * x`, so the cross-check with the matrix functor is insensitive
/// to the choice; the order is fixed deliberately for non-commutative rigs.
pub struct SfgModel<R: Rig> {
    _phantom: PhantomData<R>,
}

impl<R: Rig> SfgModel<R> {
    /// Construct the SFG model.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<R: Rig> Default for SfgModel<R> {
    fn default() -> Self {
        Self::new()
    }
}

/// Consume exactly `N` inputs, or report a [`SyntaxError::WireCount`].
///
/// [`eval`] already guarantees the bundle length equals the generator's source
/// arity before delegating, so `N` always matches; the fallible conversion
/// keeps the model total (no panic) even when `apply_generator` is called
/// directly with a wrong-length bundle.
fn take_inputs<const N: usize, R>(inputs: Vec<R>) -> Result<[R; N], SyntaxError> {
    let actual = inputs.len();
    inputs.try_into().map_err(|_| SyntaxError::WireCount {
        expected: N,
        actual,
        context: "SfgModel generator input arity",
    })
}

impl<R> ArrowModel<SfgGenerator<R>> for SfgModel<R>
where
    R: Rig + core::fmt::Debug + Eq + std::hash::Hash + 'static,
{
    type Value = R;

    fn apply_generator(
        &self,
        generator: &SfgGenerator<R>,
        inputs: Vec<R>,
    ) -> Result<Vec<R>, SyntaxError> {
        match generator {
            SfgGenerator::Copy => {
                let [x] = take_inputs::<1, R>(inputs)?;
                Ok(vec![x.clone(), x])
            }
            SfgGenerator::Discard => {
                let [_x] = take_inputs::<1, R>(inputs)?;
                Ok(vec![])
            }
            SfgGenerator::Add => {
                let [a, b] = take_inputs::<2, R>(inputs)?;
                Ok(vec![a + b])
            }
            SfgGenerator::Zero => {
                let [] = take_inputs::<0, R>(inputs)?;
                Ok(vec![R::zero()])
            }
            SfgGenerator::Scalar(r) => {
                let [x] = take_inputs::<1, R>(inputs)?;
                Ok(vec![x * r.clone()])
            }
        }
    }
}
