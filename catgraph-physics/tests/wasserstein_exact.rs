//! `wasserstein_1` against exact optima computed by exhaustive enumeration
//! (issue #387).
//!
//! Two reference oracles, each exact on the family it covers:
//!
//! - **Contingency-table enumeration** — every non-negative integer table with
//!   the given row and column margins, in units of 1/12. Optimal transport
//!   over rational margins has an integral optimum at that denominator, so the
//!   minimum over these tables is W₁ exactly.
//! - **Permutation minimum** — for uniform 1/k margins on a k x k cost matrix,
//!   the transportation polytope is the Birkhoff polytope, whose vertices are
//!   the permutation matrices, so the minimum over permutations is W₁ exactly.
//!
//! Both families are drawn from the seeded `catgraph-testutil` LCG. The
//! rational-margin family reaches zero-mass margins; the uniform family does
//! not, and is covered at zero mass by the padded variant.

use catgraph_physics::multiway::wasserstein_1;
use catgraph_testutil::Lcg;

/// Rows of the rational-margin family.
const ROWS: usize = 3;
/// Columns of the rational-margin family.
const COLS: usize = 4;
/// Margin denominator: masses are integer multiples of 1/12.
const DENOM: u32 = 12;

/// A disagreeing rational-margin case: trial index, row margins, column
/// margins, cost matrix, solver value, exact value.
type RationalCase = (usize, [u32; ROWS], [u32; COLS], Vec<Vec<f64>>, f64, f64);

/// Minimum cost over every non-negative integer table with row margins `r`
/// and column margins `c`, in units of 1/`DENOM`.
fn exact_by_contingency_tables(r: &[u32; ROWS], c: &[u32; COLS], cost: &[Vec<f64>]) -> f64 {
    fn walk(
        row: usize,
        col: usize,
        table: &mut [[u32; COLS]; ROWS],
        r: &[u32; ROWS],
        c: &[u32; COLS],
        cost: &[Vec<f64>],
        best: &mut f64,
    ) {
        if row == ROWS {
            for (j, &want) in c.iter().enumerate() {
                let got: u32 = (0..ROWS).map(|i| table[i][j]).sum();
                if got != want {
                    return;
                }
            }
            let total: f64 = (0..ROWS)
                .flat_map(|i| (0..COLS).map(move |j| (i, j)))
                .map(|(i, j)| f64::from(table[i][j]) * cost[i][j])
                .sum();
            *best = best.min(total);
            return;
        }
        let placed: u32 = (0..col).map(|j| table[row][j]).sum();
        let remaining = r[row] - placed;
        if col == COLS - 1 {
            table[row][col] = remaining;
            walk(row + 1, 0, table, r, c, cost, best);
            table[row][col] = 0;
            return;
        }
        for amount in 0..=remaining {
            table[row][col] = amount;
            walk(row, col + 1, table, r, c, cost, best);
        }
        table[row][col] = 0;
    }

    let mut best = f64::INFINITY;
    let mut table = [[0_u32; COLS]; ROWS];
    walk(0, 0, &mut table, r, c, cost, &mut best);
    best / f64::from(DENOM)
}

/// Minimum cost over the k! permutation couplings of uniform 1/k margins.
#[allow(clippy::cast_precision_loss)]
fn exact_by_permutations(cost: &[Vec<f64>]) -> f64 {
    fn walk(depth: usize, perm: &mut Vec<usize>, cost: &[Vec<f64>], best: &mut f64) {
        if depth == 1 {
            let total: f64 = perm.iter().enumerate().map(|(i, &p)| cost[i][p]).sum();
            *best = best.min(total);
            return;
        }
        walk(depth - 1, perm, cost, best);
        for i in 0..depth - 1 {
            if depth.is_multiple_of(2) {
                perm.swap(i, depth - 1);
            } else {
                perm.swap(0, depth - 1);
            }
            walk(depth - 1, perm, cost, best);
        }
    }

    let k = cost.len();
    let mut perm: Vec<usize> = (0..k).collect();
    let mut best = f64::INFINITY;
    walk(k, &mut perm, cost, &mut best);
    best / k as f64
}

/// Integer cost matrix with entries in `0..=9`.
fn random_costs(rng: &mut Lcg, rows: usize, cols: usize) -> Vec<Vec<f64>> {
    (0..rows)
        .map(|_| {
            (0..cols)
                .map(|_| {
                    let digit = u32::try_from(rng.next_usize(0, 9))
                        .expect("invariant: next_usize(0, 9) returns at most 9");
                    f64::from(digit)
                })
                .collect()
        })
        .collect()
}

/// 300 seeded 3x4 instances with margins in twelfths — zero margins included,
/// so both marginals carry zero-mass support points — agree with the
/// contingency-table optimum to 1e-9.
#[test]
fn matches_contingency_table_optimum_on_rational_margins() {
    let mut rng = Lcg::new(0x0387_0001);
    let trials = 300;
    let mut zero_margin_cases = 0;
    let mut worst = 0.0_f64;
    let mut first_bad: Option<RationalCase> = None;

    for trial in 0..trials {
        let mut r = [0_u32; ROWS];
        let mut c = [0_u32; COLS];
        for _ in 0..DENOM {
            r[rng.next_usize(0, ROWS - 1)] += 1;
            c[rng.next_usize(0, COLS - 1)] += 1;
        }
        if r.contains(&0) || c.contains(&0) {
            zero_margin_cases += 1;
        }
        let cost = random_costs(&mut rng, ROWS, COLS);
        let mu: Vec<f64> = r.iter().map(|&x| f64::from(x) / f64::from(DENOM)).collect();
        let nu: Vec<f64> = c.iter().map(|&x| f64::from(x) / f64::from(DENOM)).collect();

        let got = wasserstein_1(&mu, &nu, &cost);
        let want = exact_by_contingency_tables(&r, &c, &cost);
        let delta = (got - want).abs();
        if delta > worst {
            worst = delta;
        }
        if delta > 1e-9 && first_bad.is_none() {
            first_bad = Some((trial, r, c, cost, got, want));
        }
    }

    assert!(
        first_bad.is_none(),
        "solver disagrees with the contingency-table optimum on {trials} 3x4 instances \
         (worst |delta| {worst}); first: {first_bad:?}"
    );
    assert!(
        zero_margin_cases > 0,
        "the family must reach zero-mass margins, got {zero_margin_cases} of {trials}"
    );
}

/// 400 seeded uniform k x k instances for each k in 2..=6 agree with the
/// permutation minimum to 1e-9.
#[test]
#[allow(clippy::cast_precision_loss)]
fn matches_permutation_minimum_on_uniform_margins() {
    let mut rng = Lcg::new(0x0387_0002);
    for k in 2..=6_usize {
        let trials = 400;
        let mut worst = 0.0_f64;
        let mut first_bad: Option<(usize, Vec<Vec<f64>>, f64, f64)> = None;

        for trial in 0..trials {
            let cost = random_costs(&mut rng, k, k);
            let mu = vec![1.0 / k as f64; k];
            let got = wasserstein_1(&mu, &mu, &cost);
            let want = exact_by_permutations(&cost);
            let delta = (got - want).abs();
            if delta > worst {
                worst = delta;
            }
            if delta > 1e-9 && first_bad.is_none() {
                first_bad = Some((trial, cost, got, want));
            }
        }

        assert!(
            first_bad.is_none(),
            "k={k}: solver disagrees with the permutation minimum on {trials} instances \
             (worst |delta| {worst}); first: {first_bad:?}"
        );
    }
}

/// 300 seeded 3x4 rational-margin instances embedded in a 6x8 support with
/// zero-mass rows and columns interleaved — the shape `edge_ollivier_ricci`
/// builds from a union of two neighbourhoods — still agree with the
/// contingency-table optimum of the embedded instance to 1e-9.
#[test]
fn zero_mass_padding_leaves_the_contingency_table_optimum() {
    let mut rng = Lcg::new(0x0387_0003);
    let trials = 300;
    let mut worst = 0.0_f64;
    let mut first_bad: Option<RationalCase> = None;

    for trial in 0..trials {
        let mut r = [0_u32; ROWS];
        let mut c = [0_u32; COLS];
        for _ in 0..DENOM {
            r[rng.next_usize(0, ROWS - 1)] += 1;
            c[rng.next_usize(0, COLS - 1)] += 1;
        }
        let core = random_costs(&mut rng, ROWS, COLS);

        // Even support positions carry the embedded instance; odd positions
        // carry no mass on either marginal.
        let mut cost = random_costs(&mut rng, 2 * ROWS, 2 * COLS);
        for (i, row) in core.iter().enumerate() {
            for (j, &value) in row.iter().enumerate() {
                cost[2 * i][2 * j] = value;
            }
        }
        let mut mu = vec![0.0; 2 * ROWS];
        let mut nu = vec![0.0; 2 * COLS];
        for (i, &mass) in r.iter().enumerate() {
            mu[2 * i] = f64::from(mass) / f64::from(DENOM);
        }
        for (j, &mass) in c.iter().enumerate() {
            nu[2 * j] = f64::from(mass) / f64::from(DENOM);
        }

        let got = wasserstein_1(&mu, &nu, &cost);
        let want = exact_by_contingency_tables(&r, &c, &core);
        let delta = (got - want).abs();
        if delta > worst {
            worst = delta;
        }
        if delta > 1e-9 && first_bad.is_none() {
            first_bad = Some((trial, r, c, cost, got, want));
        }
    }

    assert!(
        first_bad.is_none(),
        "zero-padded solver disagrees with the contingency-table optimum on {trials} \
         instances (worst |delta| {worst}); first: {first_bad:?}"
    );
}
