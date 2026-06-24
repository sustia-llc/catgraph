//! The functor `S: SFG_R → Mat(R)` — F&S 2018 Thm 5.53.
//!
//! Structural recursion on `PropExpr<SfgGenerator<R>>`, mapping generators per
//! Eq 5.52 (page 165):
//!
//! - `Copy    : 1 → 2` → `1×2` matrix `[[one, one]]`
//! - `Discard : 1 → 0` → `1×0` empty matrix
//! - `Add     : 2 → 1` → `2×1` matrix `[[one], [one]]`
//! - `Zero    : 0 → 1` → `0×1` empty matrix
//! - `Scalar(r) : 1 → 1` → `1×1` matrix `[[r]]`
//! - `Identity(n)` → `MatR::identity(n)`
//! - `Braid(m, n)` → `(m+n) × (m+n)` block-swap matrix (top `m` wires to
//!   bottom, bottom `n` wires to top)
//! - `Compose(f, g)` → `S(f).matmul(&S(g))`
//! - `Tensor(f, g)` → `S(f).block_diagonal(&S(g))`
//!
//! Matrix-dimension convention (F&S Def 5.50 + Remark 5.49): a morphism
//! `m → n` is represented as an `m × n` matrix; composition is row-major
//! matmul corresponding to `(v ; A)`.

use catgraph::errors::CatgraphError;

use crate::{
    mat::MatR,
    prop::PropExpr,
    rig::Rig,
    sfg::{SfgGenerator, SignalFlowGraph},
};

/// Apply the functor `S` to a signal flow graph.
///
/// # Errors
///
/// Returns [`CatgraphError::SfgFunctor`] (wrapping an inner
/// [`CatgraphError::CompositionSizeMismatch`]) if the underlying
/// [`PropExpr`] is ill-formed. For values built through the safe
/// [`SignalFlowGraph`] constructors this cannot occur; the error arm exists
/// to surface misuse via direct `PropExpr` construction.
pub fn sfg_to_mat<R: Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static>(
    sfg: &SignalFlowGraph<R>,
) -> Result<MatR<R>, CatgraphError> {
    sfg_to_mat_inner(sfg.as_prop_expr())
}

fn sfg_to_mat_inner<R: Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static>(
    expr: &PropExpr<SfgGenerator<R>>,
) -> Result<MatR<R>, CatgraphError> {
    match expr {
        PropExpr::Identity(n) => Ok(MatR::<R>::identity(*n)),

        PropExpr::Braid(m, n) => Ok(braid_matrix::<R>(*m, *n)),

        PropExpr::Generator(g) => Ok(generator_matrix::<R>(g)),

        PropExpr::Compose(f, g) => {
            let fm = sfg_to_mat_inner(f)?;
            let gm = sfg_to_mat_inner(g)?;
            fm.matmul(&gm).map_err(|e| CatgraphError::SfgFunctor {
                message: format!("matmul failure in S(f;g): {e}"),
            })
        }

        PropExpr::Tensor(f, g) => {
            let fm = sfg_to_mat_inner(f)?;
            let gm = sfg_to_mat_inner(g)?;
            Ok(fm.block_diagonal(&gm))
        }
    }
}

/// Eq 5.52 generator-to-matrix table.
fn generator_matrix<R: Rig + Eq + std::hash::Hash>(g: &SfgGenerator<R>) -> MatR<R> {
    match g {
        // Copy : 1 → 2 → 1×2 [[one, one]]
        SfgGenerator::Copy => {
            MatR::<R>::new(1, 2, vec![vec![R::one(), R::one()]]).expect("1×2 shape is correct")
        }

        // Discard : 1 → 0 → 1×0 (1 row, 0 cols — one empty inner vec)
        SfgGenerator::Discard => MatR::<R>::new(1, 0, vec![vec![]]).expect("1×0 shape is correct"),

        // Add : 2 → 1 → 2×1 [[one], [one]]
        SfgGenerator::Add => MatR::<R>::new(2, 1, vec![vec![R::one()], vec![R::one()]])
            .expect("2×1 shape is correct"),

        // Zero : 0 → 1 → 0×1 (0 rows, 1 col — no inner vecs)
        SfgGenerator::Zero => MatR::<R>::new(0, 1, vec![]).expect("0×1 shape is correct"),

        // Scalar(r) : 1 → 1 → 1×1 [[r]]
        SfgGenerator::Scalar(r) => {
            MatR::<R>::new(1, 1, vec![vec![r.clone()]]).expect("1×1 shape is correct")
        }
    }
}

/// Braid matrix for `σ_{m, n} : m+n → m+n`.
///
/// The braiding swaps the top `m` wires with the bottom `n`. As an
/// `(m+n) × (m+n)` permutation matrix:
///
/// - input at row `i` with `i < m` → output column `n + i`
/// - input at row `m + j` with `j < n` → output column `j`
fn braid_matrix<R: Rig>(m: usize, n: usize) -> MatR<R> {
    let dim = m + n;
    let mut entries = vec![vec![R::zero(); dim]; dim];
    for i in 0..m {
        entries[i][n + i] = R::one();
    }
    for j in 0..n {
        entries[m + j][j] = R::one();
    }
    MatR::<R>::new(dim, dim, entries).expect("braid matrix has (m+n)×(m+n) shape")
}
