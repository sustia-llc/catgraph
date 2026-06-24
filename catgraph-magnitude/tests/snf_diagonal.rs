//! Tests for `snf::diagonal` Prop 7.7 (diagonal → Smith via D&C).

#![allow(
    clippy::many_single_char_names,
    reason = "Storjohann textbook conventions: n (modulus), d (input diagonal), u/v/s (SNF triple), g (gcd) match the public API and § 7.7 paper notation"
)]

mod common;

use catgraph_magnitude::snf::band::matmul_mod;
use catgraph_magnitude::snf::diagonal::diagonal_to_smith;
use catgraph_magnitude::snf::zmod::gcd_raw;

#[test]
fn diagonal_to_smith_2x2_with_gcd_2() {
    // diag(6, 4) over Z/12: gcd(6, 4, 12) = 2; expected Smith form has gcd-chain (2, ?).
    let n = 12;
    let d = vec![vec![6, 0], vec![0, 4]];
    let (u, v, s) = diagonal_to_smith(&d, n);
    assert_eq!(s[0][1], 0);
    assert_eq!(s[1][0], 0);
    let g0 = gcd_raw(s[0][0], n);
    let g1 = gcd_raw(s[1][1], n);
    assert_eq!(g0, 2);
    assert_eq!(g1 % g0, 0);
    let ud = matmul_mod(&u, &d, n);
    let udv = matmul_mod(&ud, &v, n);
    for i in 0..2 {
        for j in 0..2 {
            assert_eq!(udv[i][j], s[i][j]);
        }
    }

    common::snf_invariants::verify_snf_invariants(&u, &v, &s, &d, n);
}

#[test]
fn diagonal_to_smith_already_satisfies_chain() {
    // diag(2, 6, 12) over Z/24 — already a divisibility chain (2 | 6 | 12).
    let n = 24;
    let d = vec![vec![2, 0, 0], vec![0, 6, 0], vec![0, 0, 12]];
    let (u, v, s) = diagonal_to_smith(&d, n);
    let g0 = gcd_raw(s[0][0], n);
    let g1 = gcd_raw(s[1][1], n);
    let g2 = gcd_raw(s[2][2], n);
    assert_eq!(g1 % g0, 0);
    assert_eq!(g2 % g1, 0);

    common::snf_invariants::verify_snf_invariants(&u, &v, &s, &d, n);
}

#[test]
fn diagonal_to_smith_zero_input() {
    let n = 12;
    let d = vec![vec![0, 0], vec![0, 0]];
    let (u, v, s) = diagonal_to_smith(&d, n);
    assert_eq!(s[0][0], 0);
    assert_eq!(s[1][1], 0);
    let ud = matmul_mod(&u, &d, n);
    let udv = matmul_mod(&ud, &v, n);
    for i in 0..2 {
        for j in 0..2 {
            assert_eq!(udv[i][j], s[i][j]);
        }
    }

    common::snf_invariants::verify_snf_invariants(&u, &v, &s, &d, n);
}

#[test]
fn diagonal_to_smith_singleton() {
    let n = 12;
    let d = vec![vec![5]];
    let (u, v, s) = diagonal_to_smith(&d, n);
    assert_eq!(s[0][0], 5);

    common::snf_invariants::verify_snf_invariants(&u, &v, &s, &d, n);
}

#[test]
fn bidiagonal_to_smith_2x2() {
    // Bi-diagonal:
    //   [3, 5]
    //   [0, 7]
    let n = 12;
    let bd = vec![vec![3, 5], vec![0, 7]];
    let (u, v, s) = catgraph_magnitude::snf::diagonal::bidiagonal_to_smith_for_testing(&bd, n);
    // S is diagonal.
    assert_eq!(s[0][1], 0);
    assert_eq!(s[1][0], 0);
    // U @ BD @ V ≡ S (mod n).
    let ubd = matmul_mod(&u, &bd, n);
    let ubdv = matmul_mod(&ubd, &v, n);
    for (i, urow) in ubdv.iter().enumerate() {
        for (j, &val) in urow.iter().enumerate() {
            assert_eq!(val, s[i][j]);
        }
    }

    common::snf_invariants::verify_snf_invariants(&u, &v, &s, &bd, n);
}

#[test]
fn bidiagonal_to_smith_3x3_chain() {
    // 3×3 upper bi-diagonal:
    //   [2, 4, 0]
    //   [0, 6, 3]
    //   [0, 0, 9]
    let n = 36;
    let bd = vec![vec![2, 4, 0], vec![0, 6, 3], vec![0, 0, 9]];
    let (u, v, s) = catgraph_magnitude::snf::diagonal::bidiagonal_to_smith_for_testing(&bd, n);
    // Diagonal.
    for (i, row) in s.iter().enumerate() {
        for (j, &val) in row.iter().enumerate() {
            if i != j {
                assert_eq!(val, 0, "off-diag at ({i},{j})");
            }
        }
    }
    // Transform invariant.
    let ubd = matmul_mod(&u, &bd, n);
    let ubdv = matmul_mod(&ubd, &v, n);
    for (i, urow) in ubdv.iter().enumerate() {
        for (j, &val) in urow.iter().enumerate() {
            assert_eq!(val, s[i][j]);
        }
    }
    // Divisibility chain.
    let g0 = gcd_raw(s[0][0], n);
    let g1 = gcd_raw(s[1][1], n);
    let g2 = gcd_raw(s[2][2], n);
    assert_eq!(g1 % g0, 0);
    assert_eq!(g2 % g1, 0);

    common::snf_invariants::verify_snf_invariants(&u, &v, &s, &bd, n);
}

#[test]
fn diagonal_to_smith_4x4_rank_permutation_branch() {
    // diag(2, 0, 3, 0) over Z/12. After power-of-two padding (already 4×4),
    // the bottom-up D&C builds 4 → 2 → 1 blocks. The mid-level merge produces
    // an SNF block where rank(B1) = 1 < t = 1 may force the rank-permutation
    // step inside `merge_raw` (the `if !is_zero(&b2) && r_b1 < t` branch).
    // Crafted to exercise the branch at lines 282-323 that the 2×2 / 3×3
    // fixtures skip; the assertion stays at the SNF invariants
    // (diagonal + chain + transform identity), which are robust to whichever
    // branch the merge takes.
    let n = 12;
    let d = vec![
        vec![2, 0, 0, 0],
        vec![0, 0, 0, 0],
        vec![0, 0, 3, 0],
        vec![0, 0, 0, 0],
    ];
    let (u, v, s) = diagonal_to_smith(&d, n);

    // S is diagonal.
    for (i, row) in s.iter().enumerate() {
        for (j, &val) in row.iter().enumerate() {
            if i != j {
                assert_eq!(val, 0, "off-diagonal entry at ({i}, {j}) is {val}");
            }
        }
    }

    // Divisibility chain on gcd(s_i, n).
    let mut prev = 1i64;
    for (i, row) in s.iter().enumerate() {
        let g = gcd_raw(row[i], n);
        if g != 0 {
            assert_eq!(g % prev, 0, "chain violation at i={i}: gcd={g} prev={prev}");
            prev = g;
        }
    }

    // U @ D @ V ≡ S (mod n).
    let ud = matmul_mod(&u, &d, n);
    let udv = matmul_mod(&ud, &v, n);
    for (i, urow) in udv.iter().enumerate() {
        for (j, &val) in urow.iter().enumerate() {
            assert_eq!(val, s[i][j], "transform mismatch at ({i}, {j})");
        }
    }

    common::snf_invariants::verify_snf_invariants(&u, &v, &s, &d, n);
}
