//! Hadamard bound calculator: `H(A) = ∏_i ||a_i||_2`.
//! Used by the multi-prime CRT lift to size the prime product.

use catgraph_magnitude::snf::crt_lift::hadamard_bound;

#[test]
fn hadamard_3x3_unit_matrix() {
    let a = vec![vec![1, 0, 0], vec![0, 1, 0], vec![0, 0, 1]];
    // H(I) = 1 · 1 · 1 = 1.
    assert_eq!(hadamard_bound(&a).unwrap(), 1);
}

#[test]
fn hadamard_2x2_simple() {
    let a = vec![vec![3, 4], vec![5, 12]];
    // ||row0||_2 = 5; ||row1||_2 = 13; H = 65.
    // Using ⌈ floor of sqrt-of-sum-of-squares-product ⌉ for safety.
    let h = hadamard_bound(&a).unwrap();
    assert!(
        (65..=67).contains(&h),
        "hadamard bound {h} not within [65, 67]"
    );
}

#[test]
fn hadamard_overflow_errors() {
    // Three entries of i64::MAX in a single row overflow the i128 row-sum:
    //   (2^63 - 1)² ≈ 2^126; 3 · 2^126 > i128::MAX = 2^127 - 1.
    // (Two entries fit in i128 — task notes accept either i128 row-sum
    // overflow or downstream f64-bound overflow as valid failure modes; we
    // pick the i128 row-sum path with the smallest fixture that actually
    // trips it.)
    let a = vec![vec![i64::MAX, i64::MAX, i64::MAX]];
    assert!(hadamard_bound(&a).is_err());
}
