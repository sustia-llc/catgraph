//! Tests for SNF Phase 1 integration (echelon → band reduction → bi-diagonal).

#![allow(
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    reason = "single-char bindings (n, a, u, t, v) are Storjohann §7 textbook names and the (i, j) index pair is needed simultaneously to test specific cells of T"
)]

use catgraph_magnitude::snf::band::matmul_mod;
use catgraph_magnitude::snf::phase_1_to_bidiagonal;

#[test]
fn phase1_4x4_yields_bidiagonal() {
    let n = 36;
    let a = vec![
        vec![2, 4, 0, 1],
        vec![6, 8, 3, 5],
        vec![0, 3, 9, 7],
        vec![1, 0, 2, 4],
    ];
    let (u, t, v) = phase_1_to_bidiagonal(&a, n);
    // T is bi-diagonal (b ≤ 2): only m[i][i] and m[i][i+1] may be nonzero.
    for i in 0..4 {
        for j in 0..4 {
            if j > i + 1 || j < i {
                assert_eq!(
                    t[i][j], 0,
                    "bi-diagonal violated at ({i}, {j}): t={}",
                    t[i][j]
                );
            }
        }
    }
    // U @ A @ V ≡ T (mod n)
    let ua = matmul_mod(&u, &a, n);
    let uav = matmul_mod(&ua, &v, n);
    for i in 0..4 {
        for j in 0..4 {
            assert_eq!(uav[i][j], t[i][j], "U @ A @ V ≢ T at ({i}, {j})");
        }
    }
}

#[test]
fn phase1_zero_rows_returns_empty_triple() {
    // Edge case n_rows == 0: must return (empty, empty, empty) without
    // panicking on any sub-routine (echelon, bandwidth probe, identity init).
    let (u, t, v) = phase_1_to_bidiagonal(&[], 7);
    assert!(u.is_empty(), "U not empty for n_rows == 0");
    assert!(t.is_empty(), "T not empty for n_rows == 0");
    assert!(v.is_empty(), "V not empty for n_rows == 0");
}

#[test]
fn phase1_already_bidiagonal_short_circuits() {
    // 2×2 already-bi-diagonal input: `lemma_3_1` produces a `t0` with
    // bandwidth ≤ 2, so `while b > 2` never iterates. U_band / V_band stay
    // as `identity(2)`, and the U @ A @ V ≡ T invariant collapses to the
    // pure echelon contract. Verifies the no-iteration short-circuit path.
    let n = 12;
    let a = vec![vec![3, 5], vec![0, 4]];
    let (u, t, v) = phase_1_to_bidiagonal(&a, n);
    // T is bi-diagonal (vacuously true at 2×2; check anyway for shape parity).
    for i in 0..2 {
        for j in 0..2 {
            if j > i + 1 || j < i {
                assert_eq!(t[i][j], 0, "bi-diagonal violated at ({i}, {j})");
            }
        }
    }
    // U @ A @ V ≡ T (mod n).
    let ua = matmul_mod(&u, &a, n);
    let uav = matmul_mod(&ua, &v, n);
    for i in 0..2 {
        for j in 0..2 {
            assert_eq!(uav[i][j], t[i][j], "U @ A @ V ≢ T at ({i}, {j})");
        }
    }
}
