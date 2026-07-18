//! Partial trace from the compact-closed structure of [`MatKron`]
//! (Fong & Spivak 2019, *Hypergraph Categories* arXiv:1806.08304v3, §3.1).
//!
//! `MatKron(R)` is a self-dual compact closed category whose Kronecker tensor
//! is **strictly** associative with a **strict** unit (dimension `1`). No
//! associators or unitors are therefore required: the partial trace over an
//! object `X` of a morphism `f : A⊗X → B⊗X` is built directly from the
//! cup/cap generators of the Hadamard SCFM as
//!
//! ```text
//! Tr_X(f) = (id_A ⊗ cup_X) ; (f ⊗ id_X) ; (id_B ⊗ cap_X)
//! ```
//!
//! This is a concrete (semantic) re-expression on the native
//! [`Composable`] / [`Monoidal`](catgraph::monoidal::Monoidal) traits, with
//! the compact-closed
//! cup/cap realized as inherent generators on [`MatKron`].

use catgraph::{category::Composable, errors::CatgraphError};

use crate::mat_kron::MatKron;
use crate::rig::Rig;

/// Partial trace over `X` of `f : A⊗X → B⊗X`, returning `Tr_X(f) : A → B`.
///
/// In the row-vector convention `f` is an `(a·x) × (b·x)` matrix and the result
/// is `a × b`. Built structurally from the compact-closed pieces:
///
/// ```text
/// Tr_X(f) = (id_A ⊗ cup_X) ; (f ⊗ id_X) ; (id_B ⊗ cap_X)
/// ```
///
/// # Errors
///
/// Returns [`CatgraphError::CompositionSizeMismatch`] if `f.rows() != a*x` or
/// `f.cols() != b*x`.
pub fn trace<R: Rig>(
    f: &MatKron<R>,
    a: usize,
    x: usize,
    b: usize,
) -> Result<MatKron<R>, CatgraphError> {
    if f.rows() != a * x {
        return Err(CatgraphError::CompositionSizeMismatch {
            expected: a * x,
            actual: f.rows(),
        });
    }
    if f.cols() != b * x {
        return Err(CatgraphError::CompositionSizeMismatch {
            expected: b * x,
            actual: f.cols(),
        });
    }

    let left = MatKron::identity(a).kron(&MatKron::cup(x));
    let mid = f.kron(&MatKron::identity(x));
    let right = MatKron::identity(b).kron(&MatKron::cap(x));

    left.compose(&mid)?.compose(&right)
}

/// Partial trace for the strict category — delegates to [`trace`].
///
/// `MatKron(R)` is strictly associative/unital, so no associator/unitor
/// rebracketing is needed and the strict trace coincides with [`trace`].
///
/// # Errors
///
/// Propagates the shape-mismatch errors of [`trace`].
pub fn trace_strict<R: Rig>(
    f: &MatKron<R>,
    a: usize,
    x: usize,
    b: usize,
) -> Result<MatKron<R>, CatgraphError> {
    trace(f, a, x, b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mat::MatR;
    use crate::rig::F64Rig;

    fn f64(v: f64) -> F64Rig {
        F64Rig(v)
    }

    // Strong correctness: the structural cup/cap construction equals the direct
    // partial-trace sum  Tr(f)[p][q] = Σ_s f[p*x + s][q*x + s].
    #[test]
    fn partial_trace_value_matches_direct_sum() {
        let (a, x, b) = (2usize, 2usize, 2usize);
        // An explicit (a*x)x(b*x) = 4x4 matrix with distinct entries.
        let mut data = vec![vec![f64(0.0); b * x]; a * x];
        let mut v = 1.0;
        for row in &mut data {
            for cell in row {
                *cell = f64(v);
                v += 1.0;
            }
        }
        let f = MatKron::from_mat(MatR::new(a * x, b * x, data).unwrap());

        let tr = trace(&f, a, x, b).unwrap();
        assert_eq!(tr.rows(), a);
        assert_eq!(tr.cols(), b);

        let fe = f.entries();
        for p in 0..a {
            for q in 0..b {
                let mut sum = f64(0.0);
                for s in 0..x {
                    sum = sum + fe[p * x + s][q * x + s];
                }
                assert_eq!(
                    tr.entries()[p][q],
                    sum,
                    "trace entry ({p},{q}) ≠ direct partial-trace sum"
                );
            }
        }
    }

    // Tr(id_{a*x}) = x · I_a.
    #[test]
    fn trace_of_identity_is_scaled_identity() {
        let (a, x) = (3usize, 2usize);
        let id = MatKron::<F64Rig>::identity(a * x);
        let tr = trace(&id, a, x, a).unwrap();

        assert_eq!(tr.rows(), a);
        assert_eq!(tr.cols(), a);
        for i in 0..a {
            for j in 0..a {
                let expected = if i == j { f64(x as f64) } else { f64(0.0) };
                assert_eq!(tr.entries()[i][j], expected, "entry ({i},{j})");
            }
        }
    }

    // Shape-mismatch input returns Err.
    #[test]
    fn shape_mismatch_is_err() {
        // f is 5x4 but a*x = 2*2 = 4 ≠ 5 rows.
        let f = MatKron::<F64Rig>::zero_matrix(5, 4);
        assert!(trace(&f, 2, 2, 2).is_err());
    }
}
