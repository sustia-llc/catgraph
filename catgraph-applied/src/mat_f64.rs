#![cfg(feature = "f64-rig")]
//! nalgebra bridge for [`MatR<F64Rig>`](crate::mat::MatR) — feature `f64-rig`.
//!
//! Provides:
//! - `mat_to_nalgebra` / `mat_from_nalgebra`: bidirectional conversion between
//!   [`MatR<F64Rig>`] and `nalgebra::DMatrix<f64>`.
//! - `determinant`: square-matrix determinant via nalgebra.
//! - `try_inverse`: square-matrix inverse via nalgebra, returning None for
//!   singular matrices.
//!
//! These operations are field-specific — they require R to be a field (has
//! subtraction and division). For arbitrary rigs (`BoolRig`, `Tropical`, user
//! semirings without subtraction), these functions are not available;
//! `MatR<R>` pure-rig `matmul` / `block_diagonal` remain available for any
//! `R: Rig`.

use nalgebra::DMatrix;

use crate::{mat::MatR, rig::F64Rig};

/// Convert `MatR<F64Rig>` to `nalgebra::DMatrix<f64>`.
///
/// Empty-dimension matrices (`rows == 0` or `cols == 0`) roundtrip to nalgebra's
/// empty `DMatrix` equivalents.
#[must_use]
pub fn mat_to_nalgebra(m: &MatR<F64Rig>) -> DMatrix<f64> {
    DMatrix::from_fn(m.rows(), m.cols(), |i, j| m.entries()[i][j].0)
}

/// Convert `nalgebra::DMatrix<f64>` to `MatR<F64Rig>`.
///
/// # Panics
///
/// Never panics in practice — the `(rows, cols)` shape is derived directly
/// from the source `DMatrix`, so `MatR::new`'s shape validation always
/// succeeds. The `.expect` is defensive against future refactors.
#[must_use]
pub fn mat_from_nalgebra(m: &DMatrix<f64>) -> MatR<F64Rig> {
    let rows = m.nrows();
    let cols = m.ncols();
    let mut entries = Vec::with_capacity(rows);
    for i in 0..rows {
        let mut row = Vec::with_capacity(cols);
        for j in 0..cols {
            row.push(F64Rig(m[(i, j)]));
        }
        entries.push(row);
    }
    MatR::<F64Rig>::new(rows, cols, entries).expect("shape derived from source DMatrix")
}

/// Determinant of a square matrix via nalgebra's LU decomposition.
///
/// Returns `None` if the matrix is non-square.
#[must_use]
pub fn determinant(m: &MatR<F64Rig>) -> Option<f64> {
    if m.rows() != m.cols() {
        return None;
    }
    Some(mat_to_nalgebra(m).determinant())
}

/// Compute the matrix inverse via nalgebra, returning None for singular or
/// non-square matrices.
#[must_use]
pub fn try_inverse(m: &MatR<F64Rig>) -> Option<MatR<F64Rig>> {
    if m.rows() != m.cols() {
        return None;
    }
    mat_to_nalgebra(m)
        .try_inverse()
        .map(|inv| mat_from_nalgebra(&inv))
}
