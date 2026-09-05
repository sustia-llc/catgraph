//! Tests for `snf::band` (Storjohann Phase 1 — Lemmas 7.3, 7.4).

mod common;

use catgraph_magnitude::snf::band::{band_reduction, compute_upper_bandwidth, matmul_mod};
use common::snf_invariants::assert_unimodular;

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
    // Recomputed from the returned matrix, not from the returned `b_new`.
    let b_rec = compute_upper_bandwidth(&m_new, n);
    assert!(
        b_rec <= b.div_ceil(2) + 1,
        "bandwidth should at least halve: observed b_rec={b_rec}, expected <= {} (b={b})",
        b.div_ceil(2) + 1
    );
    assert!(
        b_rec <= b_new,
        "self-reported bandwidth is not an upper bound: observed b_rec={b_rec}, b_new={b_new}"
    );
    assert!(
        b_new <= b.div_ceil(2) + 1,
        "bandwidth should at least halve: observed b_new={b_new}, expected <= {} (b={b})",
        b.div_ceil(2) + 1
    );
    assert_eq!(m_new.len(), m.len());
    // `U_step` and `V_step` are unimodular over Z/36.
    assert_unimodular(&u_step, n, "U_step");
    assert_unimodular(&v_step, n, "V_step");
    // Unimodular invariant: U · M · V ≡ M_new (mod n). Layered defense — the
    // chain-integration tests exercise this again at a higher level, but locking
    // the contract at the band-reduction layer catches regressions early.
    let um = matmul_mod(&u_step, &m, n);
    let umv = matmul_mod(&um, &v_step, n);
    assert_eq!(umv, m_new, "unimodular invariant U @ M @ V == M_new failed");
}
