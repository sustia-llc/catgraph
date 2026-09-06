//! Integration tests for `MatR<R>` over concrete rigs (`F64Rig`, `BoolRig`,
//! `Tropical`). Exercises identity / matmul / block-diag / permutation
//! correctness, `permute_side` and its length guard, the categorical
//! interchange law, and `MatR::trace` with its `MatKron::trace` delegate.

use catgraph::{category::Composable, errors::CatgraphError, monoidal::SymmetricMonoidalMorphism};
use catgraph_applied::{
    mat::MatR,
    mat_kron::MatKron,
    rig::{BoolRig, F64Rig, Rig, Tropical},
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

// ---- permute_side: each side, and the length guard ----

/// `permute_side` for each value of `of_codomain` and each length outcome of
/// the `p.len() != expected` guard, on non-square matrices.
///
/// Both direction pins use the 3-cycle `pc`, whose inverse differs from it, so
/// each separates `β(p)` from `β(p⁻¹)`; the shape is transposed for the domain
/// case to give that side arity 3. A 2-element permutation cannot make that
/// separation — it is its own inverse.
///
/// The two no-op cases feed the permutation that is *valid on the opposite
/// side* of the 2 × 3 — `pc` has length 3 (its codomain arity) and `pd`
/// length 2 (its domain arity) — so a mismatch is rejected by length alone,
/// never because the permutation was an identity.
#[test]
fn permute_side_permutes_the_matching_side_and_no_ops_on_length_mismatch() {
    let wide = || {
        MatR::<F64Rig>::new(
            2,
            3,
            vec![
                vec![F64Rig(1.0), F64Rig(2.0), F64Rig(3.0)],
                vec![F64Rig(4.0), F64Rig(5.0), F64Rig(6.0)],
            ],
        )
        .unwrap()
    };
    let tall = || {
        MatR::<F64Rig>::new(
            3,
            2,
            vec![
                vec![F64Rig(1.0), F64Rig(2.0)],
                vec![F64Rig(3.0), F64Rig(4.0)],
                vec![F64Rig(5.0), F64Rig(6.0)],
            ],
        )
        .unwrap()
    };
    // apply = [1, 2, 0]
    let pc = permutations::Permutation::rotation_left(3, 1);
    // apply = [1, 0]
    let pd = permutations::Permutation::transposition(2, 0, 1);

    let mut codomain = wide();
    codomain.permute_side(&pc, true);
    assert_eq!(
        codomain,
        MatR::<F64Rig>::new(
            2,
            3,
            vec![
                vec![F64Rig(3.0), F64Rig(1.0), F64Rig(2.0)],
                vec![F64Rig(6.0), F64Rig(4.0), F64Rig(5.0)],
            ],
        )
        .unwrap(),
        "columns permuted by pc: column p.apply(b) receives column b"
    );

    let mut domain = tall();
    domain.permute_side(&pc, false);
    assert_eq!(
        domain,
        MatR::<F64Rig>::new(
            3,
            2,
            vec![
                vec![F64Rig(5.0), F64Rig(6.0)],
                vec![F64Rig(1.0), F64Rig(2.0)],
                vec![F64Rig(3.0), F64Rig(4.0)],
            ],
        )
        .unwrap(),
        "rows permuted by pc: row p.apply(i) receives row i"
    );

    let mut wrong_on_codomain = wide();
    wrong_on_codomain.permute_side(&pd, true);
    assert_eq!(
        wrong_on_codomain,
        wide(),
        "pd has length 2, the codomain arity is 3: guarded no-op"
    );

    let mut wrong_on_domain = wide();
    wrong_on_domain.permute_side(&pc, false);
    assert_eq!(
        wrong_on_domain,
        wide(),
        "pc has length 3, the domain arity is 2: guarded no-op"
    );
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

// ---- trace: the rig diagonal sum ----

/// Check `MatR::trace` against each `(label, matrix, expected)` case,
/// reporting every mismatch with the value it measured.
fn check_traces<R: Rig + std::fmt::Debug>(cases: &[(&str, MatR<R>, Option<R>)]) {
    let mut mismatches = Vec::new();
    for (label, m, expected) in cases {
        let measured = m.trace();
        if measured != *expected {
            mismatches.push(format!(
                "{label}: expected {expected:?}, measured {measured:?}"
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "trace mismatches: {}",
        mismatches.join("; ")
    );
}

/// A wrong `expected` (`[[7]]` → 8) makes `check_traces` panic with the
/// measured value.
#[test]
#[should_panic(expected = "[[7]]: expected Some(F64Rig(8.0)), measured Some(F64Rig(7.0))")]
fn check_traces_reports_a_mismatch() {
    check_traces(&[(
        "[[7]]",
        MatR::<F64Rig>::new(1, 1, vec![vec![F64Rig(7.0)]]).unwrap(),
        Some(F64Rig(8.0)),
    )]);
}

/// `[[1,2],[3,4]]` → 1 + 4 = 5, separating the diagonal sum from the
/// all-entries sum (10) and from the constant `zero` (0); the 2×3 → `None`;
/// the 0×0 → `Some(0)`, the empty rig sum.
#[test]
fn trace_f64_diagonal_sum_shape_guard_and_empty() {
    check_traces(&[
        (
            "[[1,2],[3,4]]",
            MatR::<F64Rig>::new(
                2,
                2,
                vec![
                    vec![F64Rig(1.0), F64Rig(2.0)],
                    vec![F64Rig(3.0), F64Rig(4.0)],
                ],
            )
            .unwrap(),
            Some(F64Rig(5.0)),
        ),
        (
            "2x3 zeros",
            MatR::<F64Rig>::new(2, 3, vec![vec![F64Rig(0.0); 3], vec![F64Rig(0.0); 3]]).unwrap(),
            None,
        ),
        (
            "0x0",
            MatR::<F64Rig>::new(0, 0, vec![]).unwrap(),
            Some(F64Rig(0.0)),
        ),
    ]);
}

/// Tropical `(min, +)`: `[[3,1],[2,5]]` → `min(3, 5) = 3`, which the
/// all-entries minimum (1) does not equal. The 0×0 trace is `Tropical::zero()`
/// = `+∞`, not the real 0.
#[test]
fn trace_tropical_is_the_diagonal_min_and_empty_is_infinity() {
    check_traces(&[
        (
            "[[3,1],[2,5]]",
            MatR::<Tropical>::new(
                2,
                2,
                vec![
                    vec![Tropical(3.0), Tropical(1.0)],
                    vec![Tropical(2.0), Tropical(5.0)],
                ],
            )
            .unwrap(),
            Some(Tropical(3.0)),
        ),
        (
            "0x0",
            MatR::<Tropical>::new(0, 0, vec![]).unwrap(),
            Some(Tropical(f64::INFINITY)),
        ),
    ]);
}

/// Boolean `(∨, ∧)`: an all-false diagonal under a true off-diagonal traces to
/// `false`; a diagonal carrying one `true` traces to `true`.
#[test]
fn trace_bool_is_the_diagonal_disjunction() {
    check_traces(&[
        (
            "[[F,T],[T,F]]",
            MatR::<BoolRig>::new(
                2,
                2,
                vec![
                    vec![BoolRig(false), BoolRig(true)],
                    vec![BoolRig(true), BoolRig(false)],
                ],
            )
            .unwrap(),
            Some(BoolRig(false)),
        ),
        (
            "[[T,T],[F,F]]",
            MatR::<BoolRig>::new(
                2,
                2,
                vec![
                    vec![BoolRig(true), BoolRig(true)],
                    vec![BoolRig(false), BoolRig(false)],
                ],
            )
            .unwrap(),
            Some(BoolRig(true)),
        ),
    ]);
}

/// `MatKron::trace` reports its inner `MatR`'s diagonal sum: 1 + 4 = 5.
#[test]
fn mat_kron_trace_delegates_to_the_inner_mat() {
    let inner = MatR::<F64Rig>::new(
        2,
        2,
        vec![
            vec![F64Rig(1.0), F64Rig(2.0)],
            vec![F64Rig(3.0), F64Rig(4.0)],
        ],
    )
    .unwrap();
    let measured = MatKron::from_mat(inner).trace();
    assert_eq!(
        measured,
        Some(F64Rig(5.0)),
        "MatKron::trace of [[1,2],[3,4]]: expected Some(F64Rig(5.0)), measured {measured:?}"
    );
}

// ---- Domain / codomain conventions ----

#[test]
fn domain_equals_rows() {
    let m = MatR::<F64Rig>::new(2, 3, vec![vec![F64Rig(0.0); 3], vec![F64Rig(0.0); 3]]).unwrap();
    assert_eq!(m.domain().len(), 2);
    assert_eq!(m.codomain().len(), 3);
}
