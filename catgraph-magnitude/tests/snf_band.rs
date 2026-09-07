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

#[test]
fn band_reduction_odd_bandwidth_halves() {
    let m = vec![
        vec![1, 2, 3, 4, 5],
        vec![0, 6, 7, 8, 9],
        vec![0, 0, 10, 11, 13],
        vec![0, 0, 0, 14, 15],
        vec![0, 0, 0, 0, 17],
    ];
    let n = 36;
    let b = compute_upper_bandwidth(&m, n);
    assert_eq!(b, 5, "fixture bandwidth: observed b={b}, expected 5");
    let (m_new, u_step, v_step, b_new) = band_reduction(&m, b, 0, n);
    assert_eq!(
        b_new,
        b / 2 + 1,
        "documented return: observed b_new={b_new}, expected {} (b={b})",
        b / 2 + 1
    );
    let b_rec = compute_upper_bandwidth(&m_new, n);
    assert!(
        b_rec <= b / 2 + 1,
        "bandwidth should at least halve: observed b_rec={b_rec}, expected <= {} (b={b})",
        b / 2 + 1
    );
    assert!(
        b_rec <= b_new,
        "self-reported bandwidth is not an upper bound: observed b_rec={b_rec}, b_new={b_new}"
    );
    assert_unimodular(&u_step, n, "U_step");
    assert_unimodular(&v_step, n, "V_step");
    let um = matmul_mod(&u_step, &m, n);
    let umv = matmul_mod(&um, &v_step, n);
    assert_eq!(umv, m_new, "unimodular invariant U @ M @ V == M_new failed");
}

/// `band_reduction` at `t_param = 4` on a 10 × 10 `b = 5` fixture: the reduced
/// matrix's recomputed bandwidth is at most the returned `b_new`, and the
/// unimodular invariant holds. The inner shift loop runs
/// `⌈(rows − t_param − (i + 1)·s1) / s2⌉` times per outer step, and one step
/// fewer per outer step leaves this fixture at bandwidth 4.
#[test]
fn band_reduction_needs_every_shift_step() {
    let m = vec![
        vec![12, 4, 29, 23, 22, 0, 0, 0, 0, 0],
        vec![0, 35, 22, 7, 12, 11, 0, 0, 0, 0],
        vec![0, 0, 10, 35, 3, 23, 18, 0, 0, 0],
        vec![0, 0, 0, 11, 22, 20, 24, 20, 0, 0],
        vec![0, 0, 0, 0, 9, 27, 19, 35, 1, 0],
        vec![0, 0, 0, 0, 0, 10, 30, 26, 7, 25],
        vec![0, 0, 0, 0, 0, 0, 9, 33, 11, 12],
        vec![0, 0, 0, 0, 0, 0, 0, 16, 18, 9],
        vec![0, 0, 0, 0, 0, 0, 0, 0, 16, 13],
        vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 29],
    ];
    let n = 36;
    let t_param = 4;
    let b = compute_upper_bandwidth(&m, n);
    assert_eq!(b, 5, "fixture bandwidth: observed b={b}, expected 5");

    let (m_new, u_step, v_step, b_new) = band_reduction(&m, b, t_param, n);
    assert_eq!(
        b_new,
        b / 2 + 1,
        "documented return: observed b_new={b_new}, expected {} (b={b})",
        b / 2 + 1
    );

    let b_rec = compute_upper_bandwidth(&m_new, n);
    assert!(
        b_rec <= b_new,
        "observed b_rec={b_rec}, expected <= b_new={b_new}; a shift loop one \
         step short per outer step reads 4 here"
    );

    let um = matmul_mod(&u_step, &m, n);
    let umv = matmul_mod(&um, &v_step, n);
    assert_eq!(umv, m_new, "unimodular invariant U @ M @ V == M_new failed");
}
