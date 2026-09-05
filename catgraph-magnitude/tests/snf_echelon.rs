//! Tests for `snf::echelon` (Storjohann Lemma 3.1).

#![allow(
    clippy::cast_possible_truncation,
    reason = "sum % i128::from(n) ∈ (-n, n) ⊂ [i64::MIN, i64::MAX] for n: i64; the cast to i64 is exact"
)]

mod common;

use catgraph_magnitude::snf::echelon::{index1_reduce_on_columns, lemma_3_1};
use catgraph_magnitude::snf::zmod::posmod;
use common::snf_invariants::assert_unimodular;

#[test]
fn echelon_identity_is_unchanged() {
    let n = 7;
    let a = vec![vec![1, 0, 0], vec![0, 1, 0], vec![0, 0, 1]];
    let (u, t, rank) = lemma_3_1(&a, n);
    // U is identity; T is identity; rank = 3.
    for i in 0..3 {
        for j in 0..3 {
            assert_eq!(u[i][j], i64::from(i == j));
            assert_eq!(t[i][j], i64::from(i == j));
        }
    }
    assert_eq!(rank, 3);
}

#[test]
fn echelon_simple_2x2_yields_upper_triangular() {
    let n = 12;
    let a = vec![vec![3, 5], vec![6, 4]];
    let (u, t, rank) = lemma_3_1(&a, n);
    // U @ A == T (mod n), and T is upper triangular.
    for i in 0..2 {
        for j in 0..2 {
            let mut sum: i128 = 0;
            for k in 0..2 {
                sum += i128::from(u[i][k]) * i128::from(a[k][j]);
            }
            assert_eq!(posmod((sum % i128::from(n)) as i64, n), t[i][j]);
        }
    }
    // T is upper triangular: t[1][0] == 0
    assert_eq!(t[1][0], 0);
    // `U` is unimodular over Z/12 (echelon.rs contract on the returned `U`).
    assert_unimodular(&u, n, "U");
    // Both columns of A produce a pivot: the k = 0 elimination leaves
    // t[0][0] = gcd(3, 6, 12) = 3 and the k = 1 pass finds t[1][1] = 6, both
    // nonzero mod 12.
    assert_eq!(rank, 2, "observed rank {rank}, expected 2");
}

#[test]
fn echelon_rank_deficient_2x2_separates_rank_from_column_count() {
    let n = 12;
    let a = vec![vec![3, 5], vec![6, 10]];
    let (_u, t, rank) = lemma_3_1(&a, n);
    assert_eq!(rank, 1, "observed rank {rank}, expected 1 (n_cols = 2)");
    assert_eq!(
        t,
        vec![vec![3, 5], vec![0, 0]],
        "observed T {t:?}, expected [[3, 5], [0, 0]]"
    );
}

#[test]
fn index1_reduce_smoke_test_diagonal() {
    // 3x3 with non-zero off-diagonals in the upper triangle.
    let n = 12;
    let a = vec![vec![3, 5, 7], vec![0, 6, 4], vec![0, 0, 9]];
    let (u, t) = index1_reduce_on_columns(&a, 3, n);
    // U·A ≡ T (mod n)
    for i in 0..3 {
        for j in 0..3 {
            let mut sum: i128 = 0;
            for k in 0..3 {
                sum += i128::from(u[i][k]) * i128::from(a[k][j]);
            }
            assert_eq!(posmod((sum % i128::from(n)) as i64, n), t[i][j]);
        }
    }
    // `U` is unimodular over Z/12.
    assert_unimodular(&u, n, "U");
    // Reduced `T`: the strictly-upper part of the leading 3 × 3 block is
    // reduced modulo the associates gcd(t[j][j], 12) of the diagonal —
    // gcd(6, 12) = 6 for column 1 (5 mod 6 = 5, unchanged) and
    // gcd(9, 12) = 3 for column 2 (7 mod 3 = 1, 4 mod 3 = 1).
    let t_expected = vec![vec![3, 5, 1], vec![0, 6, 1], vec![0, 0, 9]];
    assert_eq!(t, t_expected, "observed T {t:?}, expected {t_expected:?}");
}
