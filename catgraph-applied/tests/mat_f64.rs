#![cfg(feature = "f64-rig")]
//! Integration tests for the `mat_f64` nalgebra bridge (feature `f64-rig`).

use catgraph_applied::{
    mat::MatR,
    mat_f64::{
        determinant, frobenius_norm, mat_from_nalgebra, mat_to_nalgebra, one_norm, rank, solve,
        try_inverse,
    },
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

// ---- solve: a ; x = b by LU ----

/// Entrywise `|got − want| < 1e-12` over two matrices of equal shape,
/// collecting every mismatch as `"(i, j): expected …, measured …"`.
fn entry_mismatches(got: &MatR<F64Rig>, want: &MatR<F64Rig>) -> Vec<String> {
    let mut mismatches = Vec::new();
    for i in 0..want.rows() {
        for j in 0..want.cols() {
            let g = got.entries()[i][j].0;
            let w = want.entries()[i][j].0;
            let diff = (g - w).abs();
            if diff.is_nan() || diff >= 1e-12 {
                mismatches.push(format!("({i}, {j}): expected {w}, measured {g}"));
            }
        }
    }
    mismatches
}

/// Check `solve(a, b)` entry-for-entry against `expected` within `1e-12`, then
/// re-check the defining equation `a ; x = b` on the returned `x`.
fn check_solve(label: &str, a: &MatR<F64Rig>, b: &MatR<F64Rig>, expected: &MatR<F64Rig>) {
    let x = solve(a, b).unwrap_or_else(|| panic!("{label}: expected Some(x), measured None"));
    assert_eq!(
        (x.rows(), x.cols()),
        (expected.rows(), expected.cols()),
        "{label}: shape of x"
    );
    let value = entry_mismatches(&x, expected);
    assert!(
        value.is_empty(),
        "{label}: solve mismatches: {}",
        value.join("; ")
    );
    let residual = a.matmul(&x).expect("a is square with a.cols == x.rows");
    let back = entry_mismatches(&residual, b);
    assert!(
        back.is_empty(),
        "{label}: a ; x mismatches b at: {}",
        back.join("; ")
    );
}

/// `a = [[2,1],[1,1]]`: `b = [[3],[2]]` → `x = [[1],[1]]`;
/// `b = [[3,0],[2,1]]` → `x = [[1,-1],[1,2]]` (column 2 solves `a ; x = [0,1]`).
#[test]
fn solve_2x2_single_and_multi_rhs() {
    let a = mat(&[&[2.0, 1.0], &[1.0, 1.0]]);
    check_solve(
        "a=[[2,1],[1,1]] b=[[3],[2]]",
        &a,
        &mat(&[&[3.0], &[2.0]]),
        &mat(&[&[1.0], &[1.0]]),
    );
    check_solve(
        "a=[[2,1],[1,1]] b=[[3,0],[2,1]]",
        &a,
        &mat(&[&[3.0, 0.0], &[2.0, 1.0]]),
        &mat(&[&[1.0, -1.0], &[1.0, 2.0]]),
    );
}

/// `None` on a singular `a` (`[[1,2],[2,4]]`, row 1 = 2·row 0), on a
/// non-square `a` (2×3), and on `a.rows() != b.rows()` (2×2 against a 3×1).
#[test]
fn solve_is_none_on_singular_non_square_and_row_mismatch() {
    let singular = solve(&mat(&[&[1.0, 2.0], &[2.0, 4.0]]), &mat(&[&[1.0], &[1.0]]));
    assert!(
        singular.is_none(),
        "singular a=[[1,2],[2,4]]: expected None, measured {singular:?}"
    );

    let wide = MatR::<F64Rig>::new(2, 3, vec![vec![F64Rig(1.0); 3], vec![F64Rig(1.0); 3]])
        .expect("2x3 fixture");
    let non_square = solve(&wide, &mat(&[&[1.0], &[1.0]]));
    assert!(
        non_square.is_none(),
        "non-square a (2x3): expected None, measured {non_square:?}"
    );

    let row_mismatch = solve(
        &mat(&[&[2.0, 1.0], &[1.0, 1.0]]),
        &mat(&[&[1.0], &[1.0], &[1.0]]),
    );
    assert!(
        row_mismatch.is_none(),
        "a 2x2 with b 3x1: expected None, measured {row_mismatch:?}"
    );
}

/// A 0×0 `a` returns the empty solution of the `b`'s width, for a 0×2 `b` and
/// for a 0×0 one.
#[test]
fn solve_0x0_returns_the_empty_solution() {
    let a = MatR::<F64Rig>::new(0, 0, vec![]).expect("0x0 fixture");
    for (label, b, expected) in [
        (
            "0x2 rhs",
            MatR::<F64Rig>::new(0, 2, vec![]).expect("0x2 fixture"),
            Some((0, 2)),
        ),
        (
            "0x0 rhs",
            MatR::<F64Rig>::new(0, 0, vec![]).expect("0x0 fixture"),
            Some((0, 0)),
        ),
    ] {
        let x = solve(&a, &b);
        let shape = x.as_ref().map(|m| (m.rows(), m.cols()));
        assert_eq!(
            shape, expected,
            "solve(0x0, {label}): expected {expected:?}, measured {shape:?}"
        );
    }
}

// ---- rank: the SVD singular-value count ----

/// Check `rank(m, 1e-10)` against each `(label, matrix, expected)` case,
/// reporting every mismatch with the value it measured.
fn check_ranks(cases: &[(&str, MatR<F64Rig>, usize)]) {
    let mut mismatches = Vec::new();
    for (label, m, expected) in cases {
        let measured = rank(m, 1e-10);
        if measured != *expected {
            mismatches.push(format!("{label}: expected {expected}, measured {measured}"));
        }
    }
    assert!(
        mismatches.is_empty(),
        "rank mismatches: {}",
        mismatches.join("; ")
    );
}

/// Rank-deficient and full-rank squares: `[[1,2],[2,4]]` (row 1 = 2·row 0) → 1;
/// `[[1,2,3],[4,5,6],[7,8,9]]` (row 2 = 2·row 1 − row 0) → 2; I₃ → 3; the
/// 2×2 zero matrix → 0. The three deficient cases fall below `min(rows, cols)`;
/// I₃ meets it.
#[test]
fn rank_square_values() {
    check_ranks(&[
        ("[[1,2],[2,4]]", mat(&[&[1.0, 2.0], &[2.0, 4.0]]), 1),
        (
            "[[1,2,3],[4,5,6],[7,8,9]]",
            mat(&[&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0], &[7.0, 8.0, 9.0]]),
            2,
        ),
        ("I3", MatR::<F64Rig>::identity(3), 3),
        ("zero 2x2", MatR::<F64Rig>::zero_matrix(2, 2), 0),
    ]);
}

/// A 2×3 of full row rank → 2; the empty shapes 0×0, 0×3 and 3×0 → 0.
#[test]
fn rank_non_square_and_empty_shapes() {
    check_ranks(&[
        (
            "2x3 [[1,0,0],[0,1,0]]",
            mat(&[&[1.0, 0.0, 0.0], &[0.0, 1.0, 0.0]]),
            2,
        ),
        ("0x0", MatR::<F64Rig>::new(0, 0, vec![]).expect("0x0"), 0),
        ("0x3", MatR::<F64Rig>::new(0, 3, vec![]).expect("0x3"), 0),
        (
            "3x0",
            MatR::<F64Rig>::new(3, 0, vec![vec![], vec![], vec![]]).expect("3x0"),
            0,
        ),
    ]);
}

/// A negative `eps` on a non-empty shape reaches nalgebra's assertion.
#[test]
#[should_panic(expected = "epsilon must be non-negative")]
fn rank_negative_eps_panics() {
    let _ = rank(&mat(&[&[1.0]]), -1.0);
}

// ---- norms ----

/// Check `one_norm` and `frobenius_norm` against hand-computed values within
/// `1e-12`, reporting every mismatch with the value it measured.
fn check_norms(label: &str, m: &MatR<F64Rig>, expected_one: f64, expected_frobenius: f64) {
    let mut mismatches = Vec::new();
    for (name, measured, expected) in [
        ("one_norm", one_norm(m), expected_one),
        ("frobenius_norm", frobenius_norm(m), expected_frobenius),
    ] {
        let diff = (measured - expected).abs();
        if diff.is_nan() || diff >= 1e-12 {
            mismatches.push(format!("{name}: expected {expected}, measured {measured}"));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{label}: norm mismatches: {}",
        mismatches.join("; ")
    );
}

/// `[[1,-2,3],[4,5,-6],[7,8,9]]`: absolute column sums 12, 15, 18 → `one_norm`
/// 18, which the largest absolute row sum (24) does not equal;
/// `1+4+9+16+25+36+49+64+81 = 285` → `frobenius_norm` `√285`, which `one_norm`
/// does not equal.
#[test]
fn norms_3x3_values() {
    check_norms(
        "[[1,-2,3],[4,5,-6],[7,8,9]]",
        &mat(&[&[1.0, -2.0, 3.0], &[4.0, 5.0, -6.0], &[7.0, 8.0, 9.0]]),
        18.0,
        285f64.sqrt(),
    );
}

/// Non-square 2×3 `[[1,-2,3],[4,5,-6]]`: absolute column sums 5, 7, 9 →
/// `one_norm` 9; `1+4+9+16+25+36 = 91` → `frobenius_norm` `√91`. Each of the
/// three empty shapes has both norms 0.
#[test]
fn norms_non_square_and_empty() {
    check_norms(
        "2x3 [[1,-2,3],[4,5,-6]]",
        &mat(&[&[1.0, -2.0, 3.0], &[4.0, 5.0, -6.0]]),
        9.0,
        91f64.sqrt(),
    );
    check_norms("0x0", &mat(&[]), 0.0, 0.0);
    check_norms(
        "0x3",
        &MatR::<F64Rig>::new(0, 3, vec![]).expect("0x3"),
        0.0,
        0.0,
    );
    check_norms(
        "3x0",
        &MatR::<F64Rig>::new(3, 0, vec![vec![], vec![], vec![]]).expect("3x0"),
        0.0,
        0.0,
    );
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
