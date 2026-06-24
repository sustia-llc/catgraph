//! Tests for `snf::band` (Storjohann Phase 1 — Lemmas 7.3, 7.4).

use catgraph_magnitude::snf::band::{band_reduction, compute_upper_bandwidth, matmul_mod};

#[test]
fn bandwidth_of_identity_is_one() {
    // Diagonal-only: bandwidth = 1.
    let m = vec![vec![1, 0, 0], vec![0, 1, 0], vec![0, 0, 1]];
    assert_eq!(compute_upper_bandwidth(&m, 7), 1);
}

#[test]
fn bandwidth_of_full_upper_triangular_is_n() {
    // Upper triangular n×n with all entries non-zero: bandwidth = n.
    let m = vec![vec![1, 2, 3], vec![0, 4, 5], vec![0, 0, 6]];
    assert_eq!(compute_upper_bandwidth(&m, 7), 3);
}

#[test]
fn band_reduction_halves_bandwidth() {
    // Nontrivial 4x4 matrix mod 36 with bandwidth 4; one reduction step → bandwidth ≤ 3.
    let m = vec![
        vec![1, 2, 3, 4],
        vec![5, 6, 7, 8],
        vec![9, 10, 11, 12],
        vec![13, 14, 15, 16],
    ];
    let n = 36;
    let b = compute_upper_bandwidth(&m, n);
    let (m_new, u_step, v_step, b_new) = band_reduction(&m, b, 0, n);
    assert!(
        b_new <= b.div_ceil(2) + 1,
        "bandwidth should at least halve (got b={b}, b_new={b_new})"
    );
    assert_eq!(m_new.len(), m.len());
    // Unimodular invariant: U · M · V ≡ M_new (mod n). Layered defense — Task 15
    // exercises this again at the chain-integration level, but locking the
    // contract at the band-reduction layer catches regressions early.
    let um = matmul_mod(&u_step, &m, n);
    let umv = matmul_mod(&um, &v_step, n);
    assert_eq!(umv, m_new, "unimodular invariant U @ M @ V == M_new failed");
}
