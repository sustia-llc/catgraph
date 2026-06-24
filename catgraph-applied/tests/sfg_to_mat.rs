//! Functoriality tests for `sfg_to_mat`: verifies F&S Thm 5.53 structural
//! preservation of compose / tensor / identity on all 4 concrete rigs, plus
//! the Eq 5.52 generator table.

use catgraph_applied::{
    mat::MatR,
    rig::{BoolRig, F64Rig, Tropical, UnitInterval},
    sfg::{SignalFlowGraph, copy_n},
    sfg_to_mat::sfg_to_mat,
};

// ---- Generator table (Eq 5.52) ----

#[test]
fn eq_5_52_scalar_maps_to_1x1_matrix() {
    let s = SignalFlowGraph::<F64Rig>::scalar(F64Rig(3.5));
    let m = sfg_to_mat(&s).unwrap();
    assert_eq!(m.rows(), 1);
    assert_eq!(m.cols(), 1);
    assert_eq!(m.entries()[0][0], F64Rig(3.5));
}

#[test]
fn eq_5_52_add_maps_to_2x1_ones() {
    let a = SignalFlowGraph::<F64Rig>::add();
    let m = sfg_to_mat(&a).unwrap();
    assert_eq!(m.rows(), 2);
    assert_eq!(m.cols(), 1);
    assert_eq!(m.entries()[0][0], F64Rig(1.0));
    assert_eq!(m.entries()[1][0], F64Rig(1.0));
}

#[test]
fn eq_5_52_zero_maps_to_0x1_empty() {
    let z = SignalFlowGraph::<F64Rig>::zero();
    let m = sfg_to_mat(&z).unwrap();
    assert_eq!(m.rows(), 0);
    assert_eq!(m.cols(), 1);
}

#[test]
fn eq_5_52_copy_maps_to_1x2_ones() {
    let c = SignalFlowGraph::<F64Rig>::copy();
    let m = sfg_to_mat(&c).unwrap();
    assert_eq!(m.rows(), 1);
    assert_eq!(m.cols(), 2);
    assert_eq!(m.entries()[0][0], F64Rig(1.0));
    assert_eq!(m.entries()[0][1], F64Rig(1.0));
}

#[test]
fn eq_5_52_discard_maps_to_1x0_empty() {
    let d = SignalFlowGraph::<F64Rig>::discard();
    let m = sfg_to_mat(&d).unwrap();
    assert_eq!(m.rows(), 1);
    assert_eq!(m.cols(), 0);
}

// ---- Functoriality: S(id) = identity ----

#[test]
fn s_preserves_identity_2_f64() {
    let id2 = SignalFlowGraph::<F64Rig>::identity(2);
    let m = sfg_to_mat(&id2).unwrap();
    assert_eq!(m, MatR::<F64Rig>::identity(2));
}

#[test]
fn s_preserves_identity_3_bool() {
    let id3 = SignalFlowGraph::<BoolRig>::identity(3);
    let m = sfg_to_mat(&id3).unwrap();
    assert_eq!(m, MatR::<BoolRig>::identity(3));
}

// ---- Functoriality: S(f ; g) = S(f) * S(g) ----

#[test]
fn s_preserves_compose_copy_then_add_f64() {
    // copy(1→2) ; add(2→1) should map to matmul of their matrices.
    let c = SignalFlowGraph::<F64Rig>::copy();
    let a = SignalFlowGraph::<F64Rig>::add();
    let composed = c.compose(&a).unwrap();

    let lhs = sfg_to_mat(&composed).unwrap();
    let rhs = sfg_to_mat(&c)
        .unwrap()
        .matmul(&sfg_to_mat(&a).unwrap())
        .unwrap();
    assert_eq!(lhs, rhs);

    // Also check the concrete value: 1×2 × 2×1 = 1×1 [(1·1+1·1)] = [2].
    assert_eq!(lhs.rows(), 1);
    assert_eq!(lhs.cols(), 1);
    assert_eq!(lhs.entries()[0][0], F64Rig(2.0));
}

#[test]
fn s_preserves_compose_scalar_chain_unit_interval() {
    let a = SignalFlowGraph::<UnitInterval>::scalar(UnitInterval::new(0.5).unwrap());
    let b = SignalFlowGraph::<UnitInterval>::scalar(UnitInterval::new(0.6).unwrap());
    let composed = a.compose(&b).unwrap();

    let lhs = sfg_to_mat(&composed).unwrap();
    let rhs = sfg_to_mat(&a)
        .unwrap()
        .matmul(&sfg_to_mat(&b).unwrap())
        .unwrap();
    assert_eq!(lhs, rhs);

    // Concrete: scalar(0.5); scalar(0.6) → matrix 0.5 * 0.6 = 0.3.
    assert_eq!(lhs.entries()[0][0], UnitInterval::new(0.3).unwrap());
}

// ---- Functoriality: S(f ⊗ g) = S(f) ⊕ S(g) ----

#[test]
fn s_preserves_tensor_copy_add_tropical() {
    let c = SignalFlowGraph::<Tropical>::copy();
    let a = SignalFlowGraph::<Tropical>::add();
    let tensored = c.tensor(&a);

    let lhs = sfg_to_mat(&tensored).unwrap();
    let rhs = sfg_to_mat(&c)
        .unwrap()
        .block_diagonal(&sfg_to_mat(&a).unwrap());
    assert_eq!(lhs, rhs);

    // Shape: copy(1→2) ⊗ add(2→1) = (1+2)→(2+1) = 3→3 ⇒ 3×3 matrix.
    assert_eq!(lhs.rows(), 3);
    assert_eq!(lhs.cols(), 3);
}

// ---- Functoriality: S(Braid(m, n)) = block-swap matrix ----

#[test]
fn s_braid_1_1_is_2x2_swap_f64() {
    let b = SignalFlowGraph::<F64Rig>::braid_1_1();
    let m = sfg_to_mat(&b).unwrap();
    assert_eq!(m.rows(), 2);
    assert_eq!(m.cols(), 2);
    // Swap matrix: [[0, 1], [1, 0]]
    assert_eq!(m.entries()[0][0], F64Rig(0.0));
    assert_eq!(m.entries()[0][1], F64Rig(1.0));
    assert_eq!(m.entries()[1][0], F64Rig(1.0));
    assert_eq!(m.entries()[1][1], F64Rig(0.0));
}

// ---- Copy-then-discard is a 1×0 empty matrix (derived identity) ----

#[test]
fn copy_then_discard_both_wires_is_empty() {
    // copy ; (discard ⊗ discard) collapses to a 1→0 morphism,
    // whose matrix is 1×0 empty.
    let c = SignalFlowGraph::<F64Rig>::copy();
    let dd = SignalFlowGraph::<F64Rig>::discard().tensor(&SignalFlowGraph::<F64Rig>::discard());
    let composed = c.compose(&dd).unwrap();
    let m = sfg_to_mat(&composed).unwrap();
    assert_eq!(m.rows(), 1);
    assert_eq!(m.cols(), 0);
}

// ---- copy_n smoke test on BoolRig ----

#[test]
fn copy_n_3_matrix_bool() {
    let c = copy_n::<BoolRig>(3).unwrap();
    let m = sfg_to_mat(&c).unwrap();
    assert_eq!(m.rows(), 1);
    assert_eq!(m.cols(), 3);
    // All entries should be BoolRig(true) == BoolRig::one()
    for j in 0..3 {
        assert_eq!(m.entries()[0][j], BoolRig(true));
    }
}
