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
//! # Wire bundles stream through a cursor
//!
//! A morphism `m → n` consumes a bundle of `m` values and produces `n`. The
//! interpreter validates the top-level input length against `expr.source()`
//! **once**, then threads the values through the term as a single
//! [`vec::IntoIter`] cursor: each node draws exactly its source arity off the
//! cursor and pushes its outputs onward. Tensor needs no split point (the two
//! factors draw in turn from the shared cursor), and no node re-walks the
//! subtree to recompute an arity — so evaluation is linear in the term size even
//! for flat composition/tensor chains. A shape violation surfaces as a
//! [`SyntaxError::WireCount`] rather than a panic. The braiding
//! [`PropExpr::Braid(m, n)`](catgraph_applied::prop::PropExpr::Braid)
//! block-rotates the bundle `[a | b]` (with `|a| = m`, `|b| = n`) to `[b | a]`
//! via `rotate_left(m)`, matching the block-swap permutation matrix of the
//! Thm 5.53 functor.
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
//! # Failure attribution
//!
//! A [`SyntaxError::WireCount`] always means a *term or caller* fault — the
//! top-level input length disagrees with `expr.source()`, or a
//! directly-constructed, ill-formed [`PropExpr`] feeds an interior node the
//! wrong wire count. A misbehaving model cannot
//! produce a `WireCount`; the one way a model can break the fold is by returning
//! the wrong number of outputs, and that is caught separately as
//! [`SyntaxError::ModelArity`].
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

use std::vec;

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

/// Draw exactly `n` values off the cursor, or report a [`SyntaxError::WireCount`]
/// (with `context` naming the node) if it runs dry first.
fn take_exact<V>(
    cursor: &mut vec::IntoIter<V>,
    n: usize,
    context: &'static str,
) -> Result<Vec<V>, SyntaxError> {
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        match cursor.next() {
            Some(v) => out.push(v),
            None => {
                return Err(SyntaxError::WireCount {
                    expected: n,
                    actual: out.len(),
                    context,
                });
            }
        }
    }
    Ok(out)
}

/// Evaluate `expr` on an `input` wire bundle under `model`.
///
/// The engine is structural: `Identity(n)` passes its `n` wires through,
/// `Braid(m, n)` block-rotates `[a | b] ↦ [b | a]`, `Compose(f, g)` pipes `f`'s
/// output into `g`, `Tensor(f, g)` lets `f` then `g` draw from the shared wire
/// cursor and concatenates their outputs, and `Generator(g)` delegates to
/// [`ArrowModel::apply_generator`]. The input length is validated against
/// `expr.source()` once, up front; from there the values stream through a single
/// cursor, so the interpreter is linear in the term size and never indexes out
/// of bounds.
///
/// # Errors
///
/// - [`SyntaxError::WireCount`] if `input.len() != expr.source()` (top level, or
///   at an interior node of a directly-constructed ill-formed term).
/// - [`SyntaxError::ModelArity`] if `model` returns a bundle whose length is not
///   `generator.target()`.
/// - Any [`SyntaxError`] the model itself raises from
///   [`apply_generator`](ArrowModel::apply_generator).
///
/// Value-level failures beyond shape are the model's own: `eval` performs no
/// arithmetic, so for [`SfgModel`] the `+`/`*` on wire values inherit `R`'s
/// overflow semantics (see the [`SfgModel`] overflow note), not this function's.
pub fn eval<G, M>(
    expr: &PropExpr<G>,
    model: &M,
    input: Vec<M::Value>,
) -> Result<Vec<M::Value>, SyntaxError>
where
    G: PropSignature,
    M: ArrowModel<G>,
{
    // The single `source()` walk — O(term size), paid once. From here every
    // node draws off `cursor` in O(1) per wire, so evaluation is linear overall.
    if input.len() != expr.source() {
        return Err(SyntaxError::WireCount {
            expected: expr.source(),
            actual: input.len(),
            context: node_kind(expr),
        });
    }
    let mut cursor = input.into_iter();
    eval_stream(expr, model, &mut cursor)
}

/// Streaming core: consume exactly `expr.source()` values off `cursor` and
/// return the `expr.target()` outputs. The shared cursor is what lets `Tensor`
/// avoid computing a split point.
fn eval_stream<G, M>(
    expr: &PropExpr<G>,
    model: &M,
    cursor: &mut vec::IntoIter<M::Value>,
) -> Result<Vec<M::Value>, SyntaxError>
where
    G: PropSignature,
    M: ArrowModel<G>,
{
    match expr {
        // `id_n`: the n wires pass through untouched.
        PropExpr::Identity(n) => take_exact(cursor, *n, "id"),
        // `σ_{m,n}`: block-rotate `[a | b]` (|a| = m, |b| = n) to `[b | a]`.
        PropExpr::Braid(m, n) => {
            let mut wires = take_exact(cursor, m + n, "braid")?;
            wires.rotate_left(*m);
            Ok(wires)
        }
        // Delegate to the model, then check it returned the promised arity.
        PropExpr::Generator(g) => {
            let inputs = take_exact(cursor, g.source(), "generator")?;
            let outputs = model.apply_generator(g, inputs)?;
            if outputs.len() != g.target() {
                return Err(SyntaxError::ModelArity {
                    generator: format!("{g:?}"),
                    expected: g.target(),
                    actual: outputs.len(),
                });
            }
            Ok(outputs)
        }
        // `f ; g`: pipe f's output bundle into g. A leftover means f produced
        // more wires than g consumed (`f.target() > g.source()`) — ill-formed.
        PropExpr::Compose(f, g) => {
            let mid = eval_stream(f, model, cursor)?;
            let produced = mid.len();
            let mut mid_cursor = mid.into_iter();
            let out = eval_stream(g, model, &mut mid_cursor)?;
            if mid_cursor.next().is_some() {
                return Err(SyntaxError::WireCount {
                    expected: g.source(),
                    actual: produced,
                    context: "compose",
                });
            }
            Ok(out)
        }
        // `f ⊗ g`: f then g draw from the shared cursor; concatenate the outputs.
        PropExpr::Tensor(f, g) => {
            let mut out = eval_stream(f, model, cursor)?;
            let right = eval_stream(g, model, cursor)?;
            out.extend(right);
            Ok(out)
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
///
/// # Overflow
///
/// The value-level arithmetic (`Add`'s `+`, `Scalar`'s `*`) is just `R`'s own
/// [`Add`](std::ops::Add) / [`Mul`](std::ops::Mul). For a primitive integer rig
/// like `i64` that is Rust's profile-dependent behaviour: a debug build panics
/// on overflow, a release build wraps. This is *identical* to the arithmetic in
/// [`MatrixNFFunctor`](catgraph_applied::prop::presentation::functorial::MatrixNFFunctor)
/// / [`MatR`](catgraph_applied::mat::MatR) (same `Rig` `+`/`*`), which is why the
/// basis-row cross-check verifies **convention parity** with the matrix functor,
/// not overflow safety — both sides compute the same value and would overflow
/// together. Callers wanting total value semantics should instantiate a rig with
/// wrapping or checked arithmetic.
pub struct SfgModel<R: Rig> {
    _phantom: std::marker::PhantomData<R>,
}

impl<R: Rig> SfgModel<R> {
    /// Construct the SFG model.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
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
/// keeps the model total — **no panic on a wrong-length bundle** even when
/// [`apply_generator`](ArrowModel::apply_generator) is called directly. This is
/// a shape guarantee only: the *value-level* arithmetic the model then performs
/// inherits `R`'s overflow semantics (see the [`SfgModel`] overflow note).
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
