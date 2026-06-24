//! Top-level SNF integration tests — hand-computed canonical fixtures.

#![allow(
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    reason = "single-char bindings (n, a, u, v, s) match Storjohann §1.1 + §3 textbook conventions; the (i, j) index pair is needed simultaneously to assert specific cells of S and to verify the U @ A @ V ≡ S product entry-by-entry"
)]

mod common;

use catgraph_magnitude::snf::band::matmul_mod;
use catgraph_magnitude::snf::smith_normal_form;
use catgraph_magnitude::snf::zmod::gcd_raw;

#[test]
fn snf_3x3_storjohann_section_3_example() {
    // Fixture from modularsnf README Quick Start:
    //   A = [[2, 4, 0], [6, 8, 3], [0, 3, 9]] mod 36
    let a = vec![vec![2, 4, 0], vec![6, 8, 3], vec![0, 3, 9]];
    let n = 36;
    let (u, v, s) = smith_normal_form(&a, n).unwrap();

    // Verify U @ A @ V ≡ S (mod n).
    let ua = matmul_mod(&u, &a, n);
    let uav = matmul_mod(&ua, &v, n);
    for (i, urow) in uav.iter().enumerate() {
        for (j, &val) in urow.iter().enumerate() {
            assert_eq!(val, s[i][j], "U @ A @ V ≢ S at ({i}, {j})");
        }
    }

    // S is diagonal.
    for (i, row) in s.iter().enumerate() {
        for (j, &val) in row.iter().enumerate() {
            if i != j {
                assert_eq!(val, 0, "S[{i}][{j}] = {val} non-zero");
            }
        }
    }

    // Diagonal divides chain on gcd(s_i, n).
    let d0 = gcd_raw(s[0][0], n);
    let d1 = gcd_raw(s[1][1], n);
    let d2 = gcd_raw(s[2][2], n);
    assert_eq!(d1 % d0, 0);
    assert_eq!(d2 % d1, 0);

    common::snf_invariants::verify_snf_invariants(&u, &v, &s, &a, n);
}

#[test]
fn snf_rejects_invalid_modulus() {
    let a = vec![vec![1, 0], vec![0, 1]];
    assert!(smith_normal_form(&a, 0).is_err());
    assert!(smith_normal_form(&a, -3).is_err());
}

#[test]
fn snf_rejects_non_rectangular() {
    let a = vec![vec![1, 0, 0], vec![0, 1]];
    assert!(smith_normal_form(&a, 7).is_err());
}

#[test]
fn snf_handles_empty() {
    let a: Vec<Vec<i64>> = Vec::new();
    let n = 7;
    let (u, v, s) = smith_normal_form(&a, n).unwrap();
    assert!(u.is_empty());
    assert!(v.is_empty());
    assert!(s.is_empty());

    common::snf_invariants::verify_snf_invariants(&u, &v, &s, &a, n);
}

#[test]
fn snf_handles_zero_columns_with_nonzero_rows() {
    // 3 rows × 0 cols — exercise the (rows > 0, cols == 0) branch of the
    // empty-edge-case early-return at snf::mod::smith_normal_form.
    let a: Vec<Vec<i64>> = vec![Vec::new(); 3];
    let n = 7;
    let (u, v, s) = smith_normal_form(&a, n).unwrap();
    // U is I_3 (3×3 identity), V is I_0 (empty), S is the 3×0 input.
    assert_eq!(u.len(), 3);
    for (i, row) in u.iter().enumerate() {
        assert_eq!(row.len(), 3);
        for (j, &val) in row.iter().enumerate() {
            assert_eq!(
                val,
                i64::from(i == j),
                "U[{i}][{j}] should be {} got {val}",
                i64::from(i == j)
            );
        }
    }
    assert!(v.is_empty());
    assert_eq!(s.len(), 3);
    for row in &s {
        assert!(row.is_empty(), "S row should be 0-length");
    }

    common::snf_invariants::verify_snf_invariants(&u, &v, &s, &a, n);
}

#[test]
fn snf_2x3_rectangular_identity() {
    // Rectangular 2×3 fixture exercising the s_dim = max(rows, cols) padding
    // + crop-back path. Identity-shaped over Z/7: pad to 3×3, run pipeline,
    // crop back to 2×3.
    let a = vec![vec![1, 0, 0], vec![0, 1, 0]];
    let n = 7;
    let (u, v, s) = smith_normal_form(&a, n).unwrap();

    // Output shapes: U is (rows, rows) = 2×2; V is (cols, cols) = 3×3;
    // S is (rows, cols) = 2×3.
    assert_eq!(u.len(), 2);
    for row in &u {
        assert_eq!(row.len(), 2);
    }
    assert_eq!(v.len(), 3);
    for row in &v {
        assert_eq!(row.len(), 3);
    }
    assert_eq!(s.len(), 2);
    for row in &s {
        assert_eq!(row.len(), 3);
    }

    // U · A · V ≡ S (mod n).
    let ua = matmul_mod(&u, &a, n);
    let uav = matmul_mod(&ua, &v, n);
    for (i, urow) in uav.iter().enumerate() {
        for (j, &val) in urow.iter().enumerate() {
            assert_eq!(val, s[i][j], "U @ A @ V ≢ S at ({i}, {j})");
        }
    }

    common::snf_invariants::verify_snf_invariants(&u, &v, &s, &a, n);
}
