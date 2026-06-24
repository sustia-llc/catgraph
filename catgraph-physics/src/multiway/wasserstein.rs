//! Wasserstein-1 (earth mover's distance) solver.
//!
//! Computes W₁(μ, ν) = min Σ `T_ij` * `d_ij` subject to transport plan
//! constraints, where T is a coupling with marginals μ and ν. This is the
//! optimal transport cost under a given ground metric.
//!
//! Used internally by the Ollivier-Ricci curvature backend to compute
//! transport distances between neighbor distributions on branchial graphs.
//!
//! # Algorithm
//!
//! Uses the transportation simplex (MODI / modified distribution method):
//! 1. Find initial basic feasible solution via north-west corner rule
//! 2. Iteratively improve by finding entering variables with negative reduced cost
//! 3. Pivot along cycles in the basis graph until optimal
//!
//! Scales to ~1000 nodes without performance cliffs (O(n^3) typical case).

/// Numerical tolerance for floating-point comparisons in the simplex method.
const EPS: f64 = 1e-12;

/// Maximum number of simplex pivots before declaring non-convergence.
/// For an m*n problem the theoretical bound is O(m*n) pivots;
/// we use a generous multiplier to handle degenerate cycling.
const MAX_PIVOTS: usize = 100_000;

/// Compute the Wasserstein-1 distance between two discrete distributions.
///
/// # Arguments
///
/// * `mu` - Source distribution (non-negative, sums to total mass)
/// * `nu` - Target distribution (non-negative, sums to same total mass as `mu`)
/// * `distance` - Pairwise distance matrix; `distance[i][j]` is the ground
///   metric cost of transporting one unit from support point `i` to `j`.
///   Must be `mu.len()` x `nu.len()`.
///
/// # Returns
///
/// The optimal transport cost W₁(μ, ν).
///
/// # Panics
///
/// Panics if:
/// - `mu` or `nu` is empty
/// - `distance` dimensions don't match `mu.len()` x `nu.len()`
/// - Total masses of `mu` and `nu` differ by more than `1e-9`
/// - Any entry in `mu`, `nu`, or `distance` is negative
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::too_many_lines
)]
pub fn wasserstein_1(mu: &[f64], nu: &[f64], distance: &[Vec<f64>]) -> f64 {
    let m = mu.len();
    let n = nu.len();

    // --- Validate inputs ---
    assert!(!mu.is_empty(), "mu must be non-empty");
    assert!(!nu.is_empty(), "nu must be non-empty");
    assert_eq!(distance.len(), m, "distance must have mu.len() rows");
    for (idx, row) in distance.iter().enumerate() {
        assert_eq!(row.len(), n, "distance[{idx}] must have nu.len() columns");
    }
    assert!(
        mu.iter().all(|&x| x >= 0.0),
        "mu entries must be non-negative"
    );
    assert!(
        nu.iter().all(|&x| x >= 0.0),
        "nu entries must be non-negative"
    );
    for row in distance {
        assert!(
            row.iter().all(|&x| x >= 0.0),
            "distance entries must be non-negative"
        );
    }

    let sum_mu: f64 = mu.iter().sum();
    let sum_nu: f64 = nu.iter().sum();
    assert!(
        (sum_mu - sum_nu).abs() < 1e-9,
        "Total masses must be equal: sum(mu)={sum_mu}, sum(nu)={sum_nu}"
    );

    // Trivial case: zero total mass
    if sum_mu < EPS {
        return 0.0;
    }

    // --- North-west corner rule: initial basic feasible solution ---
    // The basis has exactly m + n - 1 cells.
    let mut supply = mu.to_vec();
    let mut demand = nu.to_vec();

    // Basis representation: for each basic cell (i,j), store the flow.
    // is_basic[i][j] tracks membership.
    let mut flow = vec![vec![0.0_f64; n]; m];
    let mut is_basic = vec![vec![false; n]; m];

    let mut i = 0;
    let mut j = 0;
    while i < m && j < n {
        let amount = supply[i].min(demand[j]);
        flow[i][j] = amount;
        is_basic[i][j] = true;
        supply[i] -= amount;
        demand[j] -= amount;
        if supply[i] < EPS {
            i += 1;
        }
        if demand[j] < EPS {
            j += 1;
        }
    }

    // Ensure we have exactly m + n - 1 basic variables.
    // Degenerate cases may produce fewer; add zero-flow basics.
    let basic_count: usize = is_basic
        .iter()
        .flat_map(|row| row.iter())
        .filter(|&&b| b)
        .count();
    if basic_count < m + n - 1 {
        add_degenerate_basics(&mut is_basic, m + n - 1 - basic_count);
    }

    // --- MODI (u-v method) simplex iterations ---
    let mut u = vec![0.0_f64; m];
    let mut v = vec![0.0_f64; n];

    for _ in 0..MAX_PIVOTS {
        // Step 1: Compute dual variables u, v from basis cells.
        // For basic cell (i,j): u[i] + v[j] = cost[i][j].
        // Fix u[0] = 0, then BFS/propagate.
        if !compute_duals(&is_basic, distance, &mut u, &mut v) {
            // Disconnected basis tree -- shouldn't happen with correct
            // degenerate handling, but add more basics and retry.
            add_degenerate_basics(&mut is_basic, 1);
            continue;
        }

        // Step 2: Find the non-basic cell with the most negative reduced cost.
        // Reduced cost: c_bar[i][j] = cost[i][j] - u[i] - v[j].
        let mut entering = None;
        let mut best_rc = -EPS;
        for ii in 0..m {
            for jj in 0..n {
                if !is_basic[ii][jj] {
                    let rc = distance[ii][jj] - u[ii] - v[jj];
                    if rc < best_rc {
                        best_rc = rc;
                        entering = Some((ii, jj));
                    }
                }
            }
        }

        // If no negative reduced cost, the current solution is optimal.
        let Some((ei, ej)) = entering else {
            break;
        };

        // Step 3: Find the cycle and pivot.
        // Adding (ei, ej) to the basis creates exactly one cycle.
        // We find it, determine the leaving variable, and update flows.
        let Some(cycle) = find_cycle(&is_basic, ei, ej, m, n) else {
            // Degenerate: couldn't find cycle. Mark as basic and continue.
            is_basic[ei][ej] = true;
            // Trim excess basics if needed
            let bc: usize = is_basic
                .iter()
                .flat_map(|row| row.iter())
                .filter(|&&b| b)
                .count();
            if bc > m + n - 1 {
                trim_basics(&mut is_basic, &flow, bc - (m + n - 1));
            }
            continue;
        };

        // The cycle alternates +/- adjustments.
        // Minimum flow on "-" cells determines the pivot amount.
        let theta = cycle
            .iter()
            .skip(1)
            .step_by(2)
            .map(|&(ci, cj)| flow[ci][cj])
            .fold(f64::INFINITY, f64::min);

        // Update flows along the cycle.
        for (step, &(ci, cj)) in cycle.iter().enumerate() {
            if step % 2 == 0 {
                flow[ci][cj] += theta;
            } else {
                flow[ci][cj] -= theta;
            }
        }

        // The entering variable becomes basic.
        is_basic[ei][ej] = true;

        // The leaving variable is the first "-" cell that hit zero.
        // (Skip the entering cell at index 0.)
        let mut left = false;
        for (step, &(ci, cj)) in cycle.iter().enumerate() {
            if step % 2 == 1 && flow[ci][cj] < EPS && (ci, cj) != (ei, ej) {
                is_basic[ci][cj] = false;
                flow[ci][cj] = 0.0;
                left = true;
                break;
            }
        }
        if !left {
            // Degenerate pivot (theta ~ 0); remove any zero-flow "-" cell.
            for (step, &(ci, cj)) in cycle.iter().enumerate() {
                if step % 2 == 1 && (ci, cj) != (ei, ej) {
                    is_basic[ci][cj] = false;
                    flow[ci][cj] = 0.0;
                    break;
                }
            }
        }
    }

    // Compute total cost.
    let mut total = 0.0;
    for ii in 0..m {
        for jj in 0..n {
            total = distance[ii][jj].mul_add(flow[ii][jj], total);
        }
    }
    total
}

/// Compute dual variables u, v via BFS on the basis tree.
///
/// Sets `u[0] = 0` and propagates through all basic cells.
/// Returns `false` if the basis graph is disconnected (indicates a bug
/// in the degenerate-variable handling).
#[allow(clippy::many_single_char_names)]
fn compute_duals(is_basic: &[Vec<bool>], cost: &[Vec<f64>], u: &mut [f64], v: &mut [f64]) -> bool {
    let m = u.len();
    let n = v.len();

    let mut u_set = vec![false; m];
    let mut v_set = vec![false; n];

    u[0] = 0.0;
    u_set[0] = true;

    // Iterative propagation -- at most m+n rounds.
    let mut changed = true;
    let mut rounds = 0;
    while changed && rounds < m + n {
        changed = false;
        rounds += 1;
        for row in 0..m {
            for col in 0..n {
                if !is_basic[row][col] {
                    continue;
                }
                if u_set[row] && !v_set[col] {
                    v[col] = cost[row][col] - u[row];
                    v_set[col] = true;
                    changed = true;
                } else if v_set[col] && !u_set[row] {
                    u[row] = cost[row][col] - v[col];
                    u_set[row] = true;
                    changed = true;
                }
            }
        }
    }

    // Check all variables were set.
    u_set.iter().all(|&s| s) && v_set.iter().all(|&s| s)
}

/// Find the unique cycle created by adding non-basic cell `(ei, ej)` to the
/// basis.
///
/// Returns the cycle as a list of (row, col) pairs starting with `(ei, ej)`.
/// Odd-indexed cells are on the "-" side of the pivot (flow decreases).
/// Returns `None` if no cycle is found (degenerate basis).
fn find_cycle(
    is_basic: &[Vec<bool>],
    ei: usize,
    ej: usize,
    rows: usize,
    cols: usize,
) -> Option<Vec<(usize, usize)>> {
    // DFS-based cycle finder.
    // From (ei, ej), alternate between row and column moves through basic cells.
    // "horizontal" = true means we're looking for another basic in the same row.
    let mut path = vec![(ei, ej)];
    if dfs_cycle(is_basic, &mut path, ei, ej, true, rows, cols) {
        Some(path)
    } else {
        None
    }
}

/// Recursive DFS to find a stepping-stone cycle.
///
/// `horizontal` indicates whether the next move should scan the current
/// row (true) or column (false) for another basic cell.
fn dfs_cycle(
    is_basic: &[Vec<bool>],
    path: &mut Vec<(usize, usize)>,
    target_row: usize,
    target_col: usize,
    horizontal: bool,
    rows: usize,
    cols: usize,
) -> bool {
    let &(cur_row, cur_col) = path.last().expect("path must be non-empty");

    if horizontal {
        // Scan current row for basic cells in columns != cur_col.
        for col in 0..cols {
            if col == cur_col || !is_basic[cur_row][col] {
                continue;
            }
            // Check if this closes the cycle.
            if col == target_col && path.len() >= 3 {
                path.push((cur_row, col));
                return true;
            }
            // Check we haven't visited this column already.
            if path.iter().any(|&(_, pc)| pc == col) {
                continue;
            }
            path.push((cur_row, col));
            if dfs_cycle(is_basic, path, target_row, target_col, false, rows, cols) {
                return true;
            }
            path.pop();
        }
    } else {
        // Scan current column for basic cells in rows != cur_row.
        for row in 0..rows {
            if row == cur_row || !is_basic[row][cur_col] {
                continue;
            }
            // Check if this closes the cycle.
            if row == target_row && path.len() >= 3 {
                path.push((row, cur_col));
                return true;
            }
            // Check we haven't visited this row already.
            if path.iter().any(|&(pr, _)| pr == row) {
                continue;
            }
            path.push((row, cur_col));
            if dfs_cycle(is_basic, path, target_row, target_col, true, rows, cols) {
                return true;
            }
            path.pop();
        }
    }

    false
}

/// Add degenerate basic variables (zero-flow) to make the basis tree
/// connected with exactly m + n - 1 entries.
fn add_degenerate_basics(is_basic: &mut [Vec<bool>], mut needed: usize) {
    for row in is_basic.iter_mut() {
        if needed == 0 {
            break;
        }
        for cell in row.iter_mut() {
            if needed == 0 {
                break;
            }
            if !*cell {
                *cell = true;
                needed -= 1;
            }
        }
    }
}

/// Remove excess basic variables (prefer zero-flow cells).
fn trim_basics(is_basic: &mut [Vec<bool>], flow: &[Vec<f64>], mut excess: usize) {
    for (row_idx, row) in is_basic.iter_mut().enumerate() {
        if excess == 0 {
            break;
        }
        for (col_idx, cell) in row.iter_mut().enumerate() {
            if excess == 0 {
                break;
            }
            if *cell && flow[row_idx][col_idx] < EPS {
                *cell = false;
                excess -= 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// W₁(μ, μ) = 0 for any distribution μ.
    /// Using uniform distribution on 3 points.
    #[test]
    fn w1_identical_distributions_is_zero() {
        let mu = vec![1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0];
        let distance = vec![
            vec![0.0, 1.0, 2.0],
            vec![1.0, 0.0, 1.0],
            vec![2.0, 1.0, 0.0],
        ];

        let w1 = wasserstein_1(&mu, &mu, &distance);
        assert!(
            w1.abs() < 1e-9,
            "W1 of identical distributions should be 0, got {w1}"
        );
    }

    /// W₁(μ, ν) = W₁(ν, μ) -- symmetry property.
    /// Using Dirac masses at different points.
    #[test]
    fn w1_symmetry() {
        let mu = vec![1.0, 0.0, 0.0];
        let nu = vec![0.0, 1.0, 0.0];
        let distance = vec![
            vec![0.0, 2.0, 5.0],
            vec![2.0, 0.0, 3.0],
            vec![5.0, 3.0, 0.0],
        ];

        let w1_forward = wasserstein_1(&mu, &nu, &distance);
        let w1_reverse = wasserstein_1(&nu, &mu, &distance);

        assert!(
            (w1_forward - w1_reverse).abs() < 1e-9,
            "W1 should be symmetric: forward={w1_forward}, reverse={w1_reverse}"
        );
    }

    /// W₁ of two Dirac deltas equals the distance between their supports.
    /// μ = δ₀, ν = δ₂, d(0,2) = 3 -> W₁ = 3.
    #[test]
    fn w1_dirac_masses_equals_distance() {
        let mu = vec![1.0, 0.0, 0.0];
        let nu = vec![0.0, 0.0, 1.0];
        let distance = vec![
            vec![0.0, 1.0, 3.0],
            vec![1.0, 0.0, 2.0],
            vec![3.0, 2.0, 0.0],
        ];

        let w1 = wasserstein_1(&mu, &nu, &distance);
        assert!(
            (w1 - 3.0).abs() < 1e-9,
            "W1 of Dirac masses should equal distance=3, got {w1}"
        );
    }

    /// Triangle inequality: W₁(μ, ρ) <= W₁(μ, ν) + W₁(ν, ρ).
    #[test]
    #[allow(clippy::similar_names)] // w_mu_nu / w_nu_rho / w_mu_rho form a standard triangle
    fn w1_triangle_inequality() {
        let mu = vec![0.5, 0.3, 0.2];
        let nu = vec![0.2, 0.5, 0.3];
        let rho = vec![0.1, 0.1, 0.8];
        let distance = vec![
            vec![0.0, 1.0, 2.0],
            vec![1.0, 0.0, 1.0],
            vec![2.0, 1.0, 0.0],
        ];

        let w_mu_nu = wasserstein_1(&mu, &nu, &distance);
        let w_nu_rho = wasserstein_1(&nu, &rho, &distance);
        let w_mu_rho = wasserstein_1(&mu, &rho, &distance);

        assert!(
            w_mu_rho <= w_mu_nu + w_nu_rho + 1e-9,
            "Triangle inequality violated: W(mu,rho)={w_mu_rho} > W(mu,nu)+W(nu,rho)={}",
            w_mu_nu + w_nu_rho
        );
    }

    /// Stress test: 100 nodes with disjoint support distributions.
    /// First 50 nodes hold all μ mass, last 50 hold all ν mass.
    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn stress_test_100_nodes() {
        let n = 100;
        let dist: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| (f64::from(i) - f64::from(j)).abs())
                    .collect()
            })
            .collect();

        let mu: Vec<f64> = (0..n)
            .map(|i| if i < 50 { 1.0 / 50.0 } else { 0.0 })
            .collect();
        let nu: Vec<f64> = (0..n)
            .map(|i| if i >= 50 { 1.0 / 50.0 } else { 0.0 })
            .collect();

        let result = wasserstein_1(&mu, &nu, &dist);
        assert!(result > 0.0, "W1 should be positive for disjoint supports");
        assert!(result.is_finite(), "W1 should be finite");
    }

    /// Uniform [0.5, 0.5] vs [1.0, 0.0] at distance 1 -> W₁ = 0.5.
    /// Must move 0.5 units from point 1 to point 0, costing 0.5 * 1 = 0.5.
    #[test]
    fn w1_uniform_to_skewed() {
        let mu = vec![0.5, 0.5];
        let nu = vec![1.0, 0.0];
        let distance = vec![vec![0.0, 1.0], vec![1.0, 0.0]];

        let w1 = wasserstein_1(&mu, &nu, &distance);
        assert!(
            (w1 - 0.5).abs() < 1e-9,
            "W1 of uniform vs skewed should be 0.5, got {w1}"
        );
    }
}
