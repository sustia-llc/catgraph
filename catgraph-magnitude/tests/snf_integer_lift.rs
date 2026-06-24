//! §1.10 `smith_normal_form_integer` — multi-prime CRT integer SNF lift.
//!
//! Tests verify the lifted diagonal invariants against canonical fixtures:
//! Wikipedia 3×3 (diag(2, 2, 156)) + identity + zero + rectangular 2×3.
//!
//! Runtime: each test invokes `select_primes_for_bound`, which allocates a
//! 256 MB `primal::Sieve::new(1 << 31)` (~9 s per test in debug, ~0.8 s in
//! release). Four tests run sequentially in ~36 s debug + build time.

#![allow(
    clippy::needless_range_loop,
    reason = "Storjohann §1.1 + §3 conventions: indexed diagonal access on the (U, V, S) triple matches the paper notation"
)]

use catgraph_magnitude::snf::smith_normal_form_integer;

#[test]
fn snf_integer_wikipedia_3x3() {
    // Wikipedia integer SNF over ℤ: diag(2, 2, 156).
    // <https://en.wikipedia.org/wiki/Smith_normal_form#Example>
    let a = vec![vec![2, 4, 4], vec![-6, 6, 12], vec![10, 4, 16]];
    let (_u, _v, s) = smith_normal_form_integer(&a).unwrap();
    assert_eq!(s[0][0], 2);
    assert_eq!(s[1][1], 2);
    assert_eq!(s[2][2], 156);
}

#[test]
fn snf_integer_identity_3x3() {
    let a = vec![vec![1, 0, 0], vec![0, 1, 0], vec![0, 0, 1]];
    let (_u, _v, s) = smith_normal_form_integer(&a).unwrap();
    for i in 0..3 {
        assert_eq!(s[i][i], 1, "S[{i}][{i}] should be 1 for identity matrix");
    }
}

#[test]
fn snf_integer_zero_matrix_3x3() {
    let a = vec![vec![0, 0, 0]; 3];
    let (_u, _v, s) = smith_normal_form_integer(&a).unwrap();
    for i in 0..3 {
        assert_eq!(s[i][i], 0, "S[{i}][{i}] should be 0 for zero matrix");
    }
}

#[test]
fn snf_integer_rectangular_2x3() {
    // 2×3 matrix; rank-2 case.
    let a = vec![vec![1, 2, 3], vec![4, 5, 6]];
    let (_u, _v, s) = smith_normal_form_integer(&a).unwrap();
    assert_eq!(s.len(), 2);
    assert_eq!(s[0].len(), 3);
    // Invariant factors satisfy s[0][0] | s[1][1].
    if s[1][1] != 0 {
        assert_eq!(s[1][1] % s[0][0], 0, "divisibility chain s[0][0] | s[1][1]");
    }
}
