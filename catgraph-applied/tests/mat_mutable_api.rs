//! Tests for the v0.5.5 mutable `MatR<Q>` API (substrate for cg-mag v0.3.0 SNF).
//!
//! All `assert_eq!` here compare exact-bit-pattern `f64` values written and
//! immediately read back (no arithmetic in between); strict equality is correct
//! by construction, so the file-level `clippy::float_cmp` allow is appropriate.

#![allow(clippy::float_cmp)]

use catgraph_applied::mat::MatR;
use catgraph_applied::rig::F64Rig;

#[test]
fn entry_mut_round_trip() {
    let mut m = MatR::<F64Rig>::identity(3);
    *m.entry_mut(1, 2).unwrap() = F64Rig(7.0);
    assert_eq!(m.entries()[1][2].0, 7.0);
}

#[test]
fn entry_mut_out_of_bounds_returns_none() {
    let mut m = MatR::<F64Rig>::identity(3);
    assert!(m.entry_mut(3, 0).is_none());
    assert!(m.entry_mut(0, 3).is_none());
}

#[test]
fn entries_mut_lets_us_zero_a_row() {
    let mut m = MatR::<F64Rig>::identity(3);
    for cell in &mut m.entries_mut()[1] {
        *cell = F64Rig(0.0);
    }
    assert_eq!(m.entries()[1].iter().map(|c| c.0).sum::<f64>(), 0.0);
}

#[test]
fn row_swap_exchanges_rows() {
    let mut m = MatR::<F64Rig>::new(
        2,
        2,
        vec![
            vec![F64Rig(1.0), F64Rig(2.0)],
            vec![F64Rig(3.0), F64Rig(4.0)],
        ],
    )
    .unwrap();
    m.row_swap(0, 1).unwrap();
    assert_eq!(m.entries()[0][0].0, 3.0);
    assert_eq!(m.entries()[0][1].0, 4.0);
    assert_eq!(m.entries()[1][0].0, 1.0);
    assert_eq!(m.entries()[1][1].0, 2.0);
}

#[test]
fn scale_row_multiplies_all_entries() {
    let mut m = MatR::<F64Rig>::new(
        2,
        2,
        vec![
            vec![F64Rig(1.0), F64Rig(2.0)],
            vec![F64Rig(3.0), F64Rig(4.0)],
        ],
    )
    .unwrap();
    m.scale_row(0, &F64Rig(5.0)).unwrap();
    assert_eq!(m.entries()[0][0].0, 5.0);
    assert_eq!(m.entries()[0][1].0, 10.0);
    assert_eq!(m.entries()[1][0].0, 3.0);
}

#[test]
fn add_scaled_row_dst_plus_factor_times_src() {
    // dst row receives dst + factor * src; src row unchanged.
    let mut m = MatR::<F64Rig>::new(
        2,
        2,
        vec![
            vec![F64Rig(1.0), F64Rig(2.0)],
            vec![F64Rig(3.0), F64Rig(4.0)],
        ],
    )
    .unwrap();
    m.add_scaled_row(/*dst=*/ 0, /*src=*/ 1, &F64Rig(2.0))
        .unwrap();
    // row 0: (1, 2) + 2 * (3, 4) = (7, 10)
    assert_eq!(m.entries()[0][0].0, 7.0);
    assert_eq!(m.entries()[0][1].0, 10.0);
    // row 1 unchanged
    assert_eq!(m.entries()[1][0].0, 3.0);
    assert_eq!(m.entries()[1][1].0, 4.0);
}

#[test]
fn add_scaled_row_self_index_returns_error() {
    // dst == src is rejected (would need a clone; Storjohann's pattern always
    // has dst != src).
    let mut m = MatR::<F64Rig>::identity(3);
    let err = m.add_scaled_row(0, 0, &F64Rig(1.0)).unwrap_err();
    assert!(
        err.to_string().contains("dst == src"),
        "unexpected message: {err}"
    );
}

#[test]
fn row_swap_out_of_bounds_returns_error() {
    let mut m = MatR::<F64Rig>::identity(3);
    assert!(m.row_swap(3, 0).is_err());
    assert!(m.row_swap(0, 3).is_err());
}

#[test]
fn scale_row_out_of_bounds_returns_error() {
    let mut m = MatR::<F64Rig>::identity(3);
    assert!(m.scale_row(3, &F64Rig(2.0)).is_err());
}

#[test]
fn add_scaled_row_out_of_bounds_returns_error() {
    let mut m = MatR::<F64Rig>::identity(3);
    assert!(m.add_scaled_row(3, 0, &F64Rig(2.0)).is_err());
    assert!(m.add_scaled_row(0, 3, &F64Rig(2.0)).is_err());
}

#[test]
fn col_swap_exchanges_columns() {
    let mut m = MatR::<F64Rig>::new(
        2,
        2,
        vec![
            vec![F64Rig(1.0), F64Rig(2.0)],
            vec![F64Rig(3.0), F64Rig(4.0)],
        ],
    )
    .unwrap();
    m.col_swap(0, 1).unwrap();
    assert_eq!(m.entries()[0][0].0, 2.0);
    assert_eq!(m.entries()[0][1].0, 1.0);
    assert_eq!(m.entries()[1][0].0, 4.0);
    assert_eq!(m.entries()[1][1].0, 3.0);
}

#[test]
fn scale_col_multiplies_column() {
    let mut m = MatR::<F64Rig>::new(
        2,
        2,
        vec![
            vec![F64Rig(1.0), F64Rig(2.0)],
            vec![F64Rig(3.0), F64Rig(4.0)],
        ],
    )
    .unwrap();
    m.scale_col(1, &F64Rig(5.0)).unwrap();
    assert_eq!(m.entries()[0][0].0, 1.0);
    assert_eq!(m.entries()[0][1].0, 10.0);
    assert_eq!(m.entries()[1][1].0, 20.0);
}

#[test]
fn add_scaled_col_dst_plus_factor_times_src() {
    let mut m = MatR::<F64Rig>::new(
        2,
        2,
        vec![
            vec![F64Rig(1.0), F64Rig(2.0)],
            vec![F64Rig(3.0), F64Rig(4.0)],
        ],
    )
    .unwrap();
    m.add_scaled_col(/*dst=*/ 0, /*src=*/ 1, &F64Rig(2.0))
        .unwrap();
    // col 0: (1, 3) + 2 * (2, 4) = (5, 11)
    assert_eq!(m.entries()[0][0].0, 5.0);
    assert_eq!(m.entries()[1][0].0, 11.0);
    // col 1 unchanged
    assert_eq!(m.entries()[0][1].0, 2.0);
    assert_eq!(m.entries()[1][1].0, 4.0);
}

#[test]
fn col_swap_out_of_bounds_returns_error() {
    let mut m = MatR::<F64Rig>::identity(3);
    assert!(m.col_swap(3, 0).is_err());
    assert!(m.col_swap(0, 3).is_err());
}

#[test]
fn scale_col_out_of_bounds_returns_error() {
    let mut m = MatR::<F64Rig>::identity(3);
    assert!(m.scale_col(3, &F64Rig(2.0)).is_err());
}

#[test]
fn add_scaled_col_self_index_returns_error() {
    let mut m = MatR::<F64Rig>::identity(3);
    let err = m.add_scaled_col(0, 0, &F64Rig(1.0)).unwrap_err();
    assert!(
        err.to_string().contains("dst == src"),
        "unexpected message: {err}"
    );
}

#[test]
fn add_scaled_col_out_of_bounds_returns_error() {
    let mut m = MatR::<F64Rig>::identity(3);
    assert!(m.add_scaled_col(3, 0, &F64Rig(2.0)).is_err());
    assert!(m.add_scaled_col(0, 3, &F64Rig(2.0)).is_err());
}
