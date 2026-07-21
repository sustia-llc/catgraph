//! Round-trip verification for `mat_to_sfg` — F&S 2018 Prop 5.56.
//!
//! Prop 5.56 asserts that the functor `S : SFG_R → Mat(R)` (Thm 5.53) is
//! full/surjective: every matrix is realized by some signal-flow graph. The
//! shipped realization `mat_to_sfg` witnesses this constructively, so the
//! characteristic property is `S(mat_to_sfg(M)) == M`.
//!
//! Pinned cases follow the paper: the 2×2 template of Eq 5.57 and the three
//! matrices of Exercise 5.58, plus the empty-dimension edge cases. The
//! proptest sweeps the round-trip over all four shipped rigs.

use catgraph_applied::{
    mat::MatR,
    mat_to_sfg::mat_to_sfg,
    rig::{BoolRig, F64Rig, Tropical, UnitInterval},
    sfg_to_mat::sfg_to_mat,
};
use proptest::prelude::*;

/// `S(mat_to_sfg(M)) == M`, plus domain/codomain arities `m → n`.
fn assert_roundtrip<R>(m: &MatR<R>)
where
    R: catgraph_applied::rig::Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static,
{
    let g = mat_to_sfg(m).expect("mat_to_sfg is arity-safe for well-formed MatR");
    assert_eq!(g.domain(), m.rows(), "domain arity = rows");
    assert_eq!(g.codomain(), m.cols(), "codomain arity = cols");
    assert_eq!(
        sfg_to_mat(&g).expect("sfg_to_mat succeeds on constructed SFG"),
        *m,
        "S(mat_to_sfg(M)) must equal M"
    );
}

/// Build a `MatR<F64Rig>` from `f64` rows.
fn matf(entries: &[&[f64]]) -> MatR<F64Rig> {
    let rows = entries.len();
    let cols = entries.first().map_or(0, |r| r.len());
    let data = entries
        .iter()
        .map(|r| r.iter().map(|&x| F64Rig(x)).collect())
        .collect();
    MatR::new(rows, cols, data).expect("rectangular fixture")
}

// ---- Pinned: Eq 5.57 (the generic 2×2 template [[a,b],[c,d]]) ----

#[test]
fn prop_5_56_eq_5_57_two_by_two() {
    let m = matf(&[&[2.0, 3.0], &[5.0, 7.0]]);
    assert_roundtrip(&m);
}

// ---- Pinned: Exercise 5.58's three matrices ----

#[test]
fn prop_5_56_exercise_5_58_three_by_one() {
    let m = matf(&[&[0.0], &[1.0], &[2.0]]);
    assert_roundtrip(&m);
}

#[test]
fn prop_5_56_exercise_5_58_two_by_two_zero() {
    let m = MatR::<F64Rig>::zero_matrix(2, 2);
    assert_roundtrip(&m);
}

#[test]
fn prop_5_56_exercise_5_58_two_by_three() {
    let m = matf(&[&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]]);
    assert_roundtrip(&m);
}

// ---- Edge cases: empty dimensions degenerate naturally ----

#[test]
fn edge_zero_by_zero() {
    let m = MatR::<F64Rig>::new(0, 0, vec![]).unwrap();
    assert_roundtrip(&m);
}

#[test]
fn edge_three_by_zero() {
    let m = MatR::<F64Rig>::new(3, 0, vec![vec![], vec![], vec![]]).unwrap();
    assert_roundtrip(&m);
}

#[test]
fn edge_zero_by_three() {
    let m = MatR::<F64Rig>::new(0, 3, vec![]).unwrap();
    assert_roundtrip(&m);
}

#[test]
fn edge_one_by_one() {
    let m = matf(&[&[42.0]]);
    assert_roundtrip(&m);
}

// ---- Round-trip proptest over all four rigs ----

/// Generate a `MatR<R>` up to 4×4 from a per-rig entry strategy.
fn arb_matrix<R, S>(entry: S) -> impl Strategy<Value = MatR<R>>
where
    R: catgraph_applied::rig::Rig + std::fmt::Debug + 'static,
    S: Strategy<Value = R> + Clone + 'static,
{
    (0usize..=4, 0usize..=4).prop_flat_map(move |(rows, cols)| {
        proptest::collection::vec(
            proptest::collection::vec(entry.clone(), cols..=cols),
            rows..=rows,
        )
        .prop_map(move |data| MatR::new(rows, cols, data).expect("rectangular by construction"))
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, ..ProptestConfig::default() })]

    #[test]
    fn roundtrip_bool(m in arb_matrix::<BoolRig, _>(any::<bool>().prop_map(BoolRig))) {
        assert_roundtrip(&m);
    }

    /// Tropical (min, +) over finite dyadic values — round-trips exactly since
    /// each entry passes through a single path (tropical-mul by one = `+0.0`,
    /// tropical-add with zero = `min(x, +∞) = x`).
    #[test]
    fn roundtrip_tropical(
        m in arb_matrix::<Tropical, _>(
            prop::sample::select(vec![0.0f64, 1.0, 2.0, 3.0]).prop_map(Tropical)
        )
    ) {
        assert_roundtrip(&m);
    }

    /// Unit interval (max, ·) over dyadic values in `[0, 1]` — round-trips
    /// exactly (mul by one = `·1.0`, add with zero = `max(x, 0.0) = x`).
    #[test]
    fn roundtrip_unit_interval(
        m in arb_matrix::<UnitInterval, _>(
            prop::sample::select(vec![0.0f64, 0.25, 0.5, 0.75, 1.0])
                .prop_map(|x| UnitInterval::new(x).expect("dyadic in [0,1]"))
        )
    ) {
        assert_roundtrip(&m);
    }

    /// F64Rig over a bounded finite range — round-trips exactly because each
    /// entry's single path is `x · 1.0` summed with `0.0` terms (both exact in
    /// IEEE-754 for finite `x`).
    #[test]
    fn roundtrip_f64(m in arb_matrix::<F64Rig, _>((-100.0f64..100.0).prop_map(F64Rig))) {
        assert_roundtrip(&m);
    }
}
