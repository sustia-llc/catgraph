#![cfg(feature = "f64-rig")]
//! Integration tests for the `mat_f64` nalgebra bridge (feature `f64-rig`).

use catgraph_applied::{
    mat::MatR,
    mat_f64::{determinant, mat_from_nalgebra, mat_to_nalgebra, try_inverse},
    rig::F64Rig,
};

#[test]
fn roundtrip_2x2_preserves_entries() {
    let m = MatR::<F64Rig>::new(
        2,
        2,
        vec![
            vec![F64Rig(1.0), F64Rig(2.0)],
            vec![F64Rig(3.0), F64Rig(4.0)],
        ],
    )
    .unwrap();
    let dm = mat_to_nalgebra(&m);
    let back = mat_from_nalgebra(&dm);
    assert_eq!(back, m);
}

#[test]
fn roundtrip_3x2_non_square_preserves_entries() {
    let m = MatR::<F64Rig>::new(
        3,
        2,
        vec![
            vec![F64Rig(1.0), F64Rig(2.0)],
            vec![F64Rig(3.0), F64Rig(4.0)],
            vec![F64Rig(5.0), F64Rig(6.0)],
        ],
    )
    .unwrap();
    let dm = mat_to_nalgebra(&m);
    assert_eq!(dm.nrows(), 3);
    assert_eq!(dm.ncols(), 2);
    let back = mat_from_nalgebra(&dm);
    assert_eq!(back, m);
}

#[test]
fn determinant_of_identity_is_1() {
    let i3 = MatR::<F64Rig>::identity(3);
    let det = determinant(&i3).expect("3x3 square");
    assert!((det - 1.0).abs() < 1e-12);
}

#[test]
fn determinant_of_non_square_is_none() {
    let m = MatR::<F64Rig>::new(2, 3, vec![vec![F64Rig(0.0); 3], vec![F64Rig(0.0); 3]]).unwrap();
    assert!(determinant(&m).is_none());
}

#[test]
fn try_inverse_of_identity_is_identity() {
    let i3 = MatR::<F64Rig>::identity(3);
    let inv = try_inverse(&i3).expect("identity is invertible");
    assert_eq!(inv, i3);
}

#[test]
fn try_inverse_of_singular_is_none() {
    let zero_mat = MatR::<F64Rig>::zero_matrix(2, 2);
    assert!(try_inverse(&zero_mat).is_none());
}

#[test]
fn try_inverse_of_non_square_is_none() {
    let m = MatR::<F64Rig>::new(2, 3, vec![vec![F64Rig(0.0); 3], vec![F64Rig(0.0); 3]]).unwrap();
    assert!(try_inverse(&m).is_none());
}

#[test]
fn inverse_matmul_original_is_identity() {
    let m = MatR::<F64Rig>::new(
        2,
        2,
        vec![
            vec![F64Rig(2.0), F64Rig(1.0)],
            vec![F64Rig(1.0), F64Rig(1.0)],
        ],
    )
    .unwrap();
    let inv = try_inverse(&m).expect("non-singular 2x2");
    let product = m.matmul(&inv).expect("composable");

    // Should be identity within floating-point tolerance
    let i2 = MatR::<F64Rig>::identity(2);
    let dm_product = mat_to_nalgebra(&product);
    let dm_i2 = mat_to_nalgebra(&i2);
    for i in 0..2 {
        for j in 0..2 {
            assert!((dm_product[(i, j)] - dm_i2[(i, j)]).abs() < 1e-10);
        }
    }
}
