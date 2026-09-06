#![cfg(feature = "f64-rig")]
//! nalgebra bridge for [`MatR<F64Rig>`](crate::mat::MatR) — feature `f64-rig`.
//!
//! [`mat_to_nalgebra`] and [`mat_from_nalgebra`] convert between
//! [`MatR<F64Rig>`] and `nalgebra::DMatrix<f64>`; [`determinant`] and
//! [`try_inverse`] are the square-matrix determinant and inverse; [`solve`]
//! solves `a ; x = b` by LU; [`rank`] is the SVD numerical rank; [`one_norm`]
//! and [`frobenius_norm`] are the 1-norm and the Frobenius norm. Each
//! exists for `F64Rig` alone and not for rigs such as `BoolRig` or `Tropical`.
//! The rig-generic diagonal sum is [`MatR::trace`](crate::mat::MatR::trace).

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
/// Never: the `(rows, cols)` shape is derived from the source `DMatrix`, so
/// `MatR::new`'s shape validation always succeeds.
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

/// The determinant of a square matrix, or `None` if the matrix is non-square.
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

/// The solution of `a ; x = b` by LU with partial pivoting; entries carry
/// floating-point rounding.
///
/// Returns `None` if `a` is non-square, if `a.rows() != b.rows()`, or if the
/// decomposition meets a zero pivot and `b` has at least one column. A 0×0 `a`
/// yields the `0 × b.cols()` solution without running the decomposition.
#[must_use]
pub fn solve(a: &MatR<F64Rig>, b: &MatR<F64Rig>) -> Option<MatR<F64Rig>> {
    if a.rows() != a.cols() || a.rows() != b.rows() {
        return None;
    }
    if a.rows() == 0 {
        return Some(MatR::<F64Rig>::zero_matrix(0, b.cols()));
    }
    mat_to_nalgebra(a)
        .lu()
        .solve(&mat_to_nalgebra(b))
        .map(|x| mat_from_nalgebra(&x))
}

/// The number of singular values strictly greater than `eps`.
///
/// A matrix with `rows == 0` or `cols == 0` has rank 0, reported without
/// running the decomposition.
///
/// # Panics
///
/// If `eps < 0.0` and neither dimension is 0.
#[must_use]
pub fn rank(m: &MatR<F64Rig>, eps: f64) -> usize {
    if m.rows() == 0 || m.cols() == 0 {
        return 0;
    }
    mat_to_nalgebra(m).rank(eps)
}

/// The 1-norm: the largest absolute column sum. 0.0 on an empty shape.
#[must_use]
pub fn one_norm(m: &MatR<F64Rig>) -> f64 {
    mat_to_nalgebra(m).one_norm()
}

/// The Frobenius norm: the square root of the sum of the squared entries.
/// 0.0 on an empty shape.
#[must_use]
pub fn frobenius_norm(m: &MatR<F64Rig>) -> f64 {
    mat_to_nalgebra(m).norm()
}
