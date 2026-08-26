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

/// Build a `MatR<F64Rig>` from row slices; `&[]` yields the 0×0 matrix.
fn mat(rows: &[&[f64]]) -> MatR<F64Rig> {
    let r = rows.len();
    let c = rows.first().map_or(0, |row| row.len());
    let entries = rows
        .iter()
        .map(|row| row.iter().map(|&x| F64Rig(x)).collect())
        .collect();
    MatR::<F64Rig>::new(r, c, entries).expect("fixture rows are rectangular")
}

/// Check `determinant` against each `(label, matrix, expected)` case within
/// `1e-12`, reporting every mismatch with the value it measured.
fn check_determinants(cases: &[(&str, MatR<F64Rig>, f64)]) {
    let mut mismatches = Vec::new();
    for (label, m, expected) in cases {
        let det = determinant(m).expect("fixture is square");
        let diff = (det - expected).abs();
        if diff.is_nan() || diff >= 1e-12 {
            mismatches.push(format!("{label}: expected {expected}, measured {det}"));
        }
    }
    assert!(
        mismatches.is_empty(),
        "determinant mismatches: {}",
        mismatches.join("; ")
    );
}

/// A wrong `expected` (`[[7]]` → 8) makes `check_determinants` panic with the
/// measured value.
#[test]
#[should_panic(expected = "[[7]]: expected 8, measured 7")]
fn check_determinants_reports_a_mismatch() {
    check_determinants(&[("[[7]]", mat(&[&[7.0]]), 8.0)]);
}

/// `[[7]]` → 7; `[[-7]]` → −7; `[[1,2,0],[0,3,4],[5,0,6]]` → 58; I₃ with
/// rows 0 and 1 swapped → −1.
#[test]
fn determinant_1x1_and_3x3_values() {
    check_determinants(&[
        ("[[7]]", mat(&[&[7.0]]), 7.0),
        ("[[-7]]", mat(&[&[-7.0]]), -7.0),
        (
            "[[1,2,0],[0,3,4],[5,0,6]]",
            mat(&[&[1.0, 2.0, 0.0], &[0.0, 3.0, 4.0], &[5.0, 0.0, 6.0]]),
            58.0,
        ),
        (
            "I3 with rows 0,1 swapped",
            mat(&[&[0.0, 1.0, 0.0], &[1.0, 0.0, 0.0], &[0.0, 0.0, 1.0]]),
            -1.0,
        ),
    ]);
}

/// `diag(2, 3)` → 6; `[[1, 2], [2, 4]]` → 0; `[[0, 1], [1, 0]]` → −1.
#[test]
fn determinant_2x2_values() {
    check_determinants(&[
        ("diag(2,3)", mat(&[&[2.0, 0.0], &[0.0, 3.0]]), 6.0),
        ("[[1,2],[2,4]]", mat(&[&[1.0, 2.0], &[2.0, 4.0]]), 0.0),
        ("[[0,1],[1,0]]", mat(&[&[0.0, 1.0], &[1.0, 0.0]]), -1.0),
    ]);
}

/// `diag([[1,2],[3,4]], [[5,6],[7,8]])` → 4; I₄ with rows 0 and 1 swapped
/// → −1; `[[1,2,3,4],[2,4,6,8],[0,1,0,1],[1,0,1,0]]` (row 1 = 2·row 0) → 0.
#[test]
fn determinant_4x4_values() {
    check_determinants(&[
        (
            "block diag 2x2 ⊕ 2x2",
            mat(&[
                &[1.0, 2.0, 0.0, 0.0],
                &[3.0, 4.0, 0.0, 0.0],
                &[0.0, 0.0, 5.0, 6.0],
                &[0.0, 0.0, 7.0, 8.0],
            ]),
            4.0,
        ),
        (
            "I4 with rows 0,1 swapped",
            mat(&[
                &[0.0, 1.0, 0.0, 0.0],
                &[1.0, 0.0, 0.0, 0.0],
                &[0.0, 0.0, 1.0, 0.0],
                &[0.0, 0.0, 0.0, 1.0],
            ]),
            -1.0,
        ),
        (
            "row1 = 2*row0",
            mat(&[
                &[1.0, 2.0, 3.0, 4.0],
                &[2.0, 4.0, 6.0, 8.0],
                &[0.0, 1.0, 0.0, 1.0],
                &[1.0, 0.0, 1.0, 0.0],
            ]),
            0.0,
        ),
    ]);
}

/// The 0×0 matrix → 1.
#[test]
fn determinant_0x0_is_1() {
    check_determinants(&[("0x0", mat(&[]), 1.0)]);
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
