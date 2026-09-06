#![cfg(feature = "f64-fast")]
//! f64 SVD rank as an oracle for the SNF rank recovery (#168).
//!
//! Both arms range over `k ∈ 0..=`[`MAX_DEGREE`] and every grade `ℓ` of every
//! fixture in [`fixtures`]: the 3-point and 5-point hand line spaces, plus
//! [`Lcg`]-seeded random connected graph metrics (random spanning tree plus
//! extra edges, integer edge weights in `{1, 2, 3}`, shortest-path distances)
//! for each seed in [`SEEDS`] and each `n ∈ 3..=5`, indexed at chain depth
//! [`MAX_DEGREE`].
//!
//! - [`homology_rank_matches_svd_rank_nullity`]:
//!   `magnitude_homology_rank` over `F64Rig` and over `Z` equals
//!   `cols(∂_k) − rank_svd(∂_k) − rank_svd(∂_{k+1})`, with each rank bound
//!   asserted before the subtraction.
//! - [`snf_rank_matches_svd_rank_at_each_prime`]: on each `∂_k` with both
//!   dimensions non-zero, the nonzero-diagonal count of
//!   `snf::smith_normal_form(∂_k, p)` at each prime of [`PRIMES`] equals
//!   `rank_svd(∂_k)`.
//!
//! [`rank_svd`] counts singular values above
//! `σ_max · max(rows, cols) · f64::EPSILON` and asserts the gap on every
//! non-empty matrix it is called on: each kept singular value at least
//! [`KEEP_FLOOR`], each dropped one at most [`DROP_CEILING`].

use catgraph_applied::lawvere_metric::LawvereMetricSpace;
use catgraph_applied::mat::MatR;
use catgraph_applied::rig::F64Rig;
use catgraph_magnitude::Z;
use catgraph_magnitude::chain_complex::{
    ChainIndex, IntegerLikeRig, boundary_matrix, magnitude_homology_rank,
};
use catgraph_magnitude::snf::smith_normal_form;
use catgraph_testutil::Lcg;
use nalgebra::DMatrix;
use nalgebra::linalg::SVD;

/// Chain-index depth, and the top `k` both arms range over.
const MAX_DEGREE: usize = 3;

/// The three rank-recovery primes of `chain_complex::homology`, as literals:
/// the Mersenne `2^31 − 1`, the secondary cross-check, the tertiary fallback.
/// The `#[cfg(test)]` tests in `catgraph-magnitude/src/chain_complex/homology.rs`
/// assert each entry of `RANK_RECOVERY_PRIMES`, directly or as a product with
/// another entry, against a literal.
const PRIMES: [i64; 3] = [2_147_483_647, 2_147_483_629, 2_147_483_587];

/// Seeds driving the random connected graph metrics.
const SEEDS: [u64; 1] = [20_260_905];

/// Cap on the endpoint-pair draws `random_connected_space` spends placing its
/// `n / 2` extra edges.
const REDRAW_BUDGET: usize = 10_000;

/// Lower bound on the singular values `rank_svd` keeps.
const KEEP_FLOOR: f64 = 1e-6;

/// Upper bound on the singular values `rank_svd` drops.
const DROP_CEILING: f64 = 1e-9;

/// Floor on the number of `(fixture, ℓ, k)` cells, `k ∈ 0..=MAX_DEGREE`, whose
/// `∂_k` carries a non-zero entry.
const NONZERO_CELL_FLOOR: usize = 26;

/// Floor on the maximum `rank_svd(∂_k)` over those same cells.
const MAX_RANK_FLOOR: usize = 18;

/// Floor on the number of `(fixture, ℓ, k)` cells, `k ∈ 0..=MAX_DEGREE`, whose
/// `∂_k` has both dimensions non-zero.
const BOTH_DIMS_CELL_FLOOR: usize = 31;

/// Floor on how many of those cells carry `rank_svd(∂_k) < min(rows, cols)`.
const RANK_DEFICIENT_CELL_FLOOR: usize = 10;

// ---------------------------------------------------------------------------
// SVD rank oracle
// ---------------------------------------------------------------------------

/// Numerical rank of `m` from its singular values, 0 on an empty matrix.
///
/// Counts singular values strictly above `σ_max · max(rows, cols) · f64::EPSILON`
/// and asserts that every kept value is `≥ KEEP_FLOOR` and every dropped value
/// `≤ DROP_CEILING`, naming the offending value, the shape and `cell`.
fn rank_svd(m: &MatR<F64Rig>, cell: &str) -> usize {
    let rows = m.rows();
    let cols = m.cols();
    if rows == 0 || cols == 0 {
        return 0;
    }
    let dm = DMatrix::from_fn(rows, cols, |i, j| m.entries()[i][j].0);
    let svd = SVD::new_unordered(dm, false, false);
    let sigmas: Vec<f64> = svd.singular_values.iter().copied().collect();
    let sigma_max = sigmas.iter().fold(0.0_f64, |a, &b| a.max(b));
    let dim = f64::from(u32::try_from(rows.max(cols)).expect("fixture dimensions fit in u32"));
    let eps = sigma_max * dim * f64::EPSILON;
    let rank = sigmas.iter().filter(|&&s| s > eps).count();
    for &s in &sigmas {
        if s > eps {
            assert!(
                s >= KEEP_FLOOR,
                "kept singular value {s} < {KEEP_FLOOR} on {rows}x{cols} matrix at {cell} \
                 (eps = {eps}, sigma_max = {sigma_max})"
            );
        } else {
            assert!(
                s <= DROP_CEILING,
                "dropped singular value {s} > {DROP_CEILING} on {rows}x{cols} matrix at {cell} \
                 (eps = {eps}, sigma_max = {sigma_max})"
            );
        }
    }
    rank
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// 3-point line: `d(0,1) = d(1,2) = 1`, `d(0,2) = 2`.
fn line3() -> LawvereMetricSpace<usize> {
    LawvereMetricSpace::from_distance_fn(3, |a, b| {
        let table = [[0.0, 1.0, 2.0], [1.0, 0.0, 1.0], [2.0, 1.0, 0.0]];
        table[a][b]
    })
}

/// 5-point line: `d(i, j) = |i − j|`.
fn line5() -> LawvereMetricSpace<usize> {
    LawvereMetricSpace::from_distance_fn(5, |a, b| {
        let ai = i64::try_from(a).expect("index below 5 fits in i64");
        let bi = i64::try_from(b).expect("index below 5 fits in i64");
        // The difference is in `0..5`; `f64::from` on the `u32` cast is exact.
        f64::from(u32::try_from((ai - bi).abs()).expect("difference below 5 fits in u32"))
    })
}

/// Shortest-path metric of a random connected weighted graph on `n` nodes.
///
/// `Lcg::new(seed)` draws a spanning tree (each node `i ≥ 1` attaches to a
/// uniform earlier node), then `n / 2` extra edges: endpoint pairs are drawn
/// uniformly and a pair that repeats an endpoint or an already-present edge is
/// discarded and redrawn, so each of the `n / 2` edges is distinct from the
/// tree and from the others. Redrawing is capped at [`REDRAW_BUDGET`] draws;
/// exhausting the budget panics. Every edge weight is drawn from `{1, 2, 3}`.
/// Distances are the Floyd–Warshall closure of that weighting.
fn random_connected_space(seed: u64, n: usize) -> LawvereMetricSpace<usize> {
    let mut rng = Lcg::new(seed);
    let mut w = vec![vec![f64::INFINITY; n]; n];
    for (i, row) in w.iter_mut().enumerate() {
        row[i] = 0.0;
    }
    let weight = |rng: &mut Lcg| match rng.next_usize(1, 3) {
        1 => 1.0,
        2 => 2.0,
        _ => 3.0,
    };
    let set_edge = |w: &mut Vec<Vec<f64>>, a: usize, b: usize, x: f64| {
        w[a][b] = x;
        w[b][a] = x;
    };
    for i in 1..n {
        let parent = rng.next_usize(0, i - 1);
        let x = weight(&mut rng);
        set_edge(&mut w, i, parent, x);
    }
    let mut extra = 0_usize;
    let mut draws = 0_usize;
    while extra < n / 2 {
        if draws >= REDRAW_BUDGET {
            panic!(
                "random_connected_space(seed={seed}, n={n}): redraw budget {REDRAW_BUDGET} \
                 exhausted with {extra} extra edges added of {} wanted",
                n / 2
            );
        }
        draws += 1;
        let u = rng.next_usize(0, n - 1);
        let v = rng.next_usize(0, n - 1);
        if u == v || w[u][v].is_finite() {
            continue;
        }
        let x = weight(&mut rng);
        set_edge(&mut w, u, v, x);
        extra += 1;
    }
    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                let via = w[i][k] + w[k][j];
                if via < w[i][j] {
                    w[i][j] = via;
                }
            }
        }
    }
    LawvereMetricSpace::from_distance_fn(n, move |a, b| w[a][b])
}

/// The labelled fixture family both arms range over.
fn fixtures() -> Vec<(String, LawvereMetricSpace<usize>)> {
    let mut out = vec![("line3".to_owned(), line3()), ("line5".to_owned(), line5())];
    for seed in SEEDS {
        for n in 3..=5 {
            out.push((
                format!("rand(seed={seed},n={n})"),
                random_connected_space(seed, n),
            ));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Arm (a): homology rank against SVD rank-nullity
// ---------------------------------------------------------------------------

#[test]
fn homology_rank_matches_svd_rank_nullity() {
    let mut nonzero_cells = 0_usize;
    let mut max_rank = 0_usize;
    for (label, space) in fixtures() {
        let idx = ChainIndex::new(&space, MAX_DEGREE);
        for &ell in idx.grades() {
            for k in 0..=MAX_DEGREE {
                let cell = format!("{label} k={k} ell={ell}");
                let bk = boundary_matrix::<F64Rig>(&idx, &space, k, ell)
                    .unwrap_or_else(|e| panic!("boundary_matrix at {cell}: {e:?}"));
                let bk1 = boundary_matrix::<F64Rig>(&idx, &space, k + 1, ell)
                    .unwrap_or_else(|e| panic!("boundary_matrix at {cell} (k+1): {e:?}"));
                let cols_k = bk.cols();
                let rank_k = rank_svd(&bk, &cell);
                let rank_k1 = rank_svd(&bk1, &cell);
                assert!(
                    rank_k <= cols_k,
                    "rank_svd(∂_k) = {rank_k} exceeds cols(∂_k) = {cols_k} at {cell}"
                );
                let kernel_dim = cols_k - rank_k;
                assert!(
                    rank_k1 <= kernel_dim,
                    "rank_svd(∂_(k+1)) = {rank_k1} exceeds dim ker(∂_k) = {kernel_dim} at {cell}"
                );
                let expected = kernel_dim - rank_k1;
                let via_f64 = magnitude_homology_rank::<F64Rig>(&idx, &space, k, ell)
                    .unwrap_or_else(|e| {
                        panic!("magnitude_homology_rank::<F64Rig> at {cell}: {e:?}")
                    });
                assert_eq!(
                    via_f64, expected,
                    "magnitude_homology_rank::<F64Rig> at {cell}: observed {via_f64}, expected \
                     {expected} = cols {cols_k} − rank {rank_k} − rank {rank_k1}"
                );
                let via_z = magnitude_homology_rank::<Z>(&idx, &space, k, ell)
                    .unwrap_or_else(|e| panic!("magnitude_homology_rank::<Z> at {cell}: {e:?}"));
                assert_eq!(
                    via_z, expected,
                    "magnitude_homology_rank::<Z> at {cell}: observed {via_z}, expected \
                     {expected} = cols {cols_k} − rank {rank_k} − rank {rank_k1}"
                );
                if bk.entries().iter().flatten().any(|q| q.0 != 0.0) {
                    nonzero_cells += 1;
                }
                max_rank = max_rank.max(rank_k);
            }
        }
    }
    assert!(
        nonzero_cells >= NONZERO_CELL_FLOOR,
        "cells whose ∂_k carries a non-zero entry: observed {nonzero_cells}, expected at least \
         {NONZERO_CELL_FLOOR}"
    );
    assert!(
        max_rank >= MAX_RANK_FLOOR,
        "maximum rank_svd(∂_k) over the family: observed {max_rank}, expected at least \
         {MAX_RANK_FLOOR}"
    );
}

// ---------------------------------------------------------------------------
// Arm (b): SNF rank mod p against SVD rank
// ---------------------------------------------------------------------------

#[test]
fn snf_rank_matches_svd_rank_at_each_prime() {
    let mut both_dims_cells = 0_usize;
    let mut rank_deficient_cells = 0_usize;
    for (label, space) in fixtures() {
        let idx = ChainIndex::new(&space, MAX_DEGREE);
        for &ell in idx.grades() {
            for k in 0..=MAX_DEGREE {
                let cell = format!("{label} k={k} ell={ell}");
                let bk = boundary_matrix::<F64Rig>(&idx, &space, k, ell)
                    .unwrap_or_else(|e| panic!("boundary_matrix at {cell}: {e:?}"));
                if bk.rows() == 0 || bk.cols() == 0 {
                    continue;
                }
                both_dims_cells += 1;
                let expected = rank_svd(&bk, &cell);
                if expected < bk.rows().min(bk.cols()) {
                    rank_deficient_cells += 1;
                }
                let a: Vec<Vec<i64>> = bk
                    .entries()
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|q| q.to_i64().expect("boundary entries are ±1 or 0"))
                            .collect()
                    })
                    .collect();
                let dim_min = bk.rows().min(bk.cols());
                for p in PRIMES {
                    let (_u, _v, s) = smith_normal_form(&a, p)
                        .unwrap_or_else(|e| panic!("smith_normal_form at {cell}, p={p}: {e:?}"));
                    let observed = (0..dim_min).filter(|&i| s[i][i].rem_euclid(p) != 0).count();
                    assert_eq!(
                        observed,
                        expected,
                        "SNF rank at {cell}, p={p}: observed {observed}, expected {expected} \
                         (SVD rank of the {}x{} boundary matrix)",
                        bk.rows(),
                        bk.cols()
                    );
                }
            }
        }
    }
    assert!(
        both_dims_cells >= BOTH_DIMS_CELL_FLOOR,
        "cells whose ∂_k has both dimensions non-zero: observed {both_dims_cells}, expected at \
         least {BOTH_DIMS_CELL_FLOOR}"
    );
    assert!(
        rank_deficient_cells >= RANK_DEFICIENT_CELL_FLOOR,
        "cells with rank_svd(∂_k) < min(rows, cols): observed {rank_deficient_cells}, expected at \
         least {RANK_DEFICIENT_CELL_FLOOR}"
    );
}
