//! Integration tests for `MatR<R>` over concrete rigs (`F64Rig`, `BoolRig`,
//! `Tropical`). Exercises identity / matmul / block-diag / permutation
//! correctness and the categorical interchange law.

use catgraph::{category::Composable, errors::CatgraphError};
use catgraph_applied::{
    mat::MatR,
    rig::{BoolRig, F64Rig, Tropical},
};

// ---- Identity composition is a no-op ----

#[test]
fn identity_composition_is_noop_f64() {
    let a = MatR::<F64Rig>::new(
        2,
        3,
        vec![
            vec![F64Rig(1.0), F64Rig(2.0), F64Rig(3.0)],
            vec![F64Rig(4.0), F64Rig(5.0), F64Rig(6.0)],
        ],
    )
    .unwrap();
    let ia = MatR::<F64Rig>::identity(2).matmul(&a).unwrap();
    let ai = a.matmul(&MatR::<F64Rig>::identity(3)).unwrap();
    assert_eq!(ia, a);
    assert_eq!(ai, a);
}

#[test]
fn identity_composition_is_noop_bool() {
    let a = MatR::<BoolRig>::new(1, 2, vec![vec![BoolRig(true), BoolRig(false)]]).unwrap();
    let ia = MatR::<BoolRig>::identity(1).matmul(&a).unwrap();
    assert_eq!(ia, a);
}

#[test]
fn size_mismatch_rejected() {
    let a = MatR::<F64Rig>::identity(3);
    let b = MatR::<F64Rig>::identity(2);
    assert!(matches!(
        a.matmul(&b),
        Err(CatgraphError::CompositionSizeMismatch { .. })
    ));
}

// ---- Matmul associativity ----

#[test]
fn matmul_associativity_f64() {
    let a = MatR::<F64Rig>::new(1, 2, vec![vec![F64Rig(1.0), F64Rig(2.0)]]).unwrap();
    let b = MatR::<F64Rig>::new(
        2,
        2,
        vec![
            vec![F64Rig(1.0), F64Rig(0.0)],
            vec![F64Rig(0.0), F64Rig(1.0)],
        ],
    )
    .unwrap();
    let c = MatR::<F64Rig>::new(2, 1, vec![vec![F64Rig(3.0)], vec![F64Rig(4.0)]]).unwrap();

    let ab_c = a.matmul(&b).unwrap().matmul(&c).unwrap();
    let a_bc = a.matmul(&b.matmul(&c).unwrap()).unwrap();
    assert_eq!(ab_c, a_bc);
}

// ---- Block-diagonal shape ----

#[test]
fn block_diagonal_shape() {
    let a = MatR::<F64Rig>::identity(2);
    let b = MatR::<F64Rig>::identity(3);
    let ab = a.block_diagonal(&b);
    assert_eq!(ab.rows(), 5);
    assert_eq!(ab.cols(), 5);
    // Off-diagonal blocks should all be zero.
    assert_eq!(ab.entries()[0][3], F64Rig(0.0));
    assert_eq!(ab.entries()[2][1], F64Rig(0.0));
}

// ---- Interchange law for monoidal category ----
// (A ⊕ B) ; (C ⊕ D) = (A ; C) ⊕ (B ; D)

#[test]
fn interchange_law_block_diag_and_matmul() {
    let a = MatR::<F64Rig>::new(1, 2, vec![vec![F64Rig(1.0), F64Rig(2.0)]]).unwrap();
    let b = MatR::<F64Rig>::new(1, 2, vec![vec![F64Rig(3.0), F64Rig(4.0)]]).unwrap();
    let c = MatR::<F64Rig>::new(2, 1, vec![vec![F64Rig(5.0)], vec![F64Rig(6.0)]]).unwrap();
    let d = MatR::<F64Rig>::new(2, 1, vec![vec![F64Rig(7.0)], vec![F64Rig(8.0)]]).unwrap();

    let left = a.block_diagonal(&b).matmul(&c.block_diagonal(&d)).unwrap();
    let right = a.matmul(&c).unwrap().block_diagonal(&b.matmul(&d).unwrap());
    assert_eq!(left, right);
}

// ---- Permutation matrix: identity + swap² = id ----

#[test]
fn permutation_identity() {
    let p = permutations::Permutation::identity(3);
    let m = MatR::<F64Rig>::permutation_matrix(&p);
    let i = MatR::<F64Rig>::identity(3);
    assert_eq!(m, i);
}

#[test]
fn permutation_swap_squared_is_identity() {
    let swap = permutations::Permutation::transposition(3, 0, 1);
    let m = MatR::<F64Rig>::permutation_matrix(&swap);
    let mm = m.matmul(&m).unwrap();
    assert_eq!(mm, MatR::<F64Rig>::identity(3));
}

// ---- Tropical smoke test: matmul behaves like shortest-path ----

#[test]
fn tropical_matmul_is_shortest_path_like() {
    // (min, +): identity = [[0, ∞], [∞, 0]].
    // Multiplication computes one step of Floyd-Warshall / APSP.
    let m = MatR::<Tropical>::new(
        2,
        2,
        vec![
            vec![Tropical(0.0), Tropical(3.0)],
            vec![Tropical(f64::INFINITY), Tropical(0.0)],
        ],
    )
    .unwrap();
    let mm = m.matmul(&m).unwrap();
    // (mm)[0][0] = min(0+0, 3+∞) = 0
    // (mm)[0][1] = min(0+3, 3+0) = 3
    assert!((mm.entries()[0][0].0 - 0.0).abs() < 1e-9);
    assert!((mm.entries()[0][1].0 - 3.0).abs() < 1e-9);
}

// ---- Domain / codomain conventions ----

#[test]
fn domain_equals_rows() {
    let m = MatR::<F64Rig>::new(2, 3, vec![vec![F64Rig(0.0); 3], vec![F64Rig(0.0); 3]]).unwrap();
    assert_eq!(m.domain().len(), 2);
    assert_eq!(m.codomain().len(), 3);
}
