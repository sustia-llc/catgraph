//! Wasserstein-1 (earth mover's distance) solver.
//!
//! Computes W₁(μ, ν) = min Σ `T_ij` * `d_ij` subject to transport plan
//! constraints, where T is a coupling with marginals μ and ν. This is the
//! optimal transport cost under a given ground metric.
//!
//! Input space: finite non-negative μ and ν of equal total mass over a
//! non-negative cost matrix; either marginal may carry zero-mass entries.
//!
//! Used internally by the Ollivier-Ricci curvature backend to compute
//! transport distances between neighbor distributions on branchial graphs.

/// Numerical tolerance for floating-point comparisons.
const EPS: f64 = 1e-12;

/// Compute the Wasserstein-1 distance between two discrete distributions.
///
/// # Arguments
///
/// * `mu` - Source distribution (non-negative, sums to total mass); entries
///   may be zero.
/// * `nu` - Target distribution (non-negative, sums to same total mass as
///   `mu`); entries may be zero.
/// * `distance` - Pairwise distance matrix; `distance[i][j]` is the ground
///   metric cost of transporting one unit from support point `i` to `j`.
///   Must be `mu.len()` x `nu.len()`.
///
/// # Returns
///
/// The optimal transport cost W₁(μ, ν), or `f64::INFINITY` when no coupling
/// of finite cost exists.
///
/// # Panics
///
/// Panics if:
/// - `mu` or `nu` is empty
/// - `distance` dimensions don't match `mu.len()` x `nu.len()`
/// - Total masses of `mu` and `nu` differ by more than `1e-9`
/// - Any entry in `mu`, `nu`, or `distance` is negative
#[must_use]
#[allow(clippy::similar_names)]
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

    // Transportation problem as a min-cost flow: a source feeding one node per
    // `mu` entry, complete bipartite arcs carrying the ground metric, one node
    // per `nu` entry draining to a sink.
    let source = m + n;
    let sink = m + n + 1;
    let mut network = Network::new(m + n + 2);
    for (row, &supply) in mu.iter().enumerate() {
        network.add_arc(source, row, supply, 0.0);
    }
    for (col, &demand) in nu.iter().enumerate() {
        network.add_arc(m + col, sink, demand, 0.0);
    }
    for (row, costs) in distance.iter().enumerate() {
        for (col, &cost) in costs.iter().enumerate() {
            network.add_arc(row, m + col, sum_mu, cost);
        }
    }

    let (moved, cost) = network.min_cost_flow(source, sink);
    if moved < sum_mu.min(sum_nu) - 1e-9 {
        return f64::INFINITY;
    }
    cost
}

/// One directed residual arc; `arcs[e ^ 1]` is its reverse.
struct Arc {
    /// Head of the arc.
    to: usize,
    /// Remaining capacity.
    cap: f64,
    /// Cost per unit of flow; negative on a reverse arc.
    cost: f64,
}

/// Residual network for successive-shortest-path min-cost flow.
struct Network {
    /// Arc pool; forward and reverse arcs are adjacent, so `e ^ 1` pairs them.
    arcs: Vec<Arc>,
    /// Arc indices leaving each node.
    adj: Vec<Vec<usize>>,
}

impl Network {
    /// An empty network on `nodes` nodes.
    fn new(nodes: usize) -> Self {
        Self {
            arcs: Vec::new(),
            adj: vec![Vec::new(); nodes],
        }
    }

    /// Add a forward arc of capacity `cap` and cost `cost`, together with its
    /// zero-capacity reverse.
    fn add_arc(&mut self, from: usize, to: usize, cap: f64, cost: f64) {
        self.adj[from].push(self.arcs.len());
        self.arcs.push(Arc { to, cap, cost });
        self.adj[to].push(self.arcs.len());
        self.arcs.push(Arc {
            to: from,
            cap: 0.0,
            cost: -cost,
        });
    }

    /// Route a maximum flow from `source` to `sink` at minimum cost, returning
    /// `(flow, cost)`.
    ///
    /// Each round sends flow along a cheapest residual path, found by Dijkstra
    /// on reduced costs under node potentials.
    fn min_cost_flow(&mut self, source: usize, sink: usize) -> (f64, f64) {
        let nodes = self.adj.len();
        let mut potential = vec![0.0_f64; nodes];
        let mut total_flow = 0.0_f64;
        let mut total_cost = 0.0_f64;

        loop {
            let (dist, parent) = self.cheapest_paths(source, &potential);
            if !dist[sink].is_finite() {
                return (total_flow, total_cost);
            }
            for (pot, &d) in potential.iter_mut().zip(&dist) {
                if d.is_finite() {
                    *pot += d;
                }
            }

            let mut push = f64::INFINITY;
            let mut node = sink;
            while node != source {
                let arc = parent[node];
                push = push.min(self.arcs[arc].cap);
                node = self.arcs[arc ^ 1].to;
            }
            assert!(
                push > EPS,
                "augmenting path bottleneck {push} <= {EPS}, expected > {EPS}"
            );

            let mut node = sink;
            while node != source {
                let arc = parent[node];
                self.arcs[arc].cap -= push;
                self.arcs[arc ^ 1].cap += push;
                total_cost = self.arcs[arc].cost.mul_add(push, total_cost);
                node = self.arcs[arc ^ 1].to;
            }
            total_flow += push;
        }
    }

    /// Dijkstra over residual arcs with positive capacity, on reduced costs
    /// `cost + potential[tail] - potential[head]`.
    ///
    /// Returns the reduced-cost distance from `source` to each node and, for
    /// each reached node, the arc it was reached by.
    fn cheapest_paths(&self, source: usize, potential: &[f64]) -> (Vec<f64>, Vec<usize>) {
        let nodes = self.adj.len();
        let mut dist = vec![f64::INFINITY; nodes];
        let mut parent = vec![usize::MAX; nodes];
        let mut settled = vec![false; nodes];
        dist[source] = 0.0;

        loop {
            let mut best = f64::INFINITY;
            let mut next = usize::MAX;
            for (node, &d) in dist.iter().enumerate() {
                if !settled[node] && d < best {
                    best = d;
                    next = node;
                }
            }
            if next == usize::MAX {
                return (dist, parent);
            }
            settled[next] = true;

            for &arc_idx in &self.adj[next] {
                let arc = &self.arcs[arc_idx];
                if arc.cap <= EPS || settled[arc.to] {
                    continue;
                }
                let reduced = arc.cost + potential[next] - potential[arc.to];
                let candidate = best + reduced;
                if candidate < dist[arc.to] {
                    dist[arc.to] = candidate;
                    parent[arc.to] = arc_idx;
                }
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

    /// The 3x4 instance recorded on issue #387: masses in twelfths, integer
    /// costs, optimum 49/12 by contingency-table enumeration over the tables
    /// with margins `[2,5,5]` and `[3,4,4,1]`.
    #[test]
    fn w1_three_by_four_rational_masses_is_forty_nine_twelfths() {
        let mu = vec![2.0 / 12.0, 5.0 / 12.0, 5.0 / 12.0];
        let nu = vec![3.0 / 12.0, 4.0 / 12.0, 4.0 / 12.0, 1.0 / 12.0];
        let distance = vec![
            vec![7.0, 5.0, 2.0, 9.0],
            vec![7.0, 4.0, 9.0, 7.0],
            vec![2.0, 7.0, 8.0, 7.0],
        ];

        let w1 = wasserstein_1(&mu, &nu, &distance);
        let want = 49.0 / 12.0;
        assert!(
            (w1 - want).abs() < 1e-9,
            "W1 on the #387 3x4 instance: got {w1}, want {want}"
        );
    }

    /// Padding the #387 3x4 instance with a zero-mass row and a zero-mass
    /// column leaves the optimum at 49/12: a support point carrying no mass
    /// admits no transport.
    #[test]
    fn w1_zero_mass_padding_leaves_the_optimum() {
        let mu = vec![2.0 / 12.0, 0.0, 5.0 / 12.0, 5.0 / 12.0];
        let nu = vec![3.0 / 12.0, 4.0 / 12.0, 0.0, 4.0 / 12.0, 1.0 / 12.0];
        let distance = vec![
            vec![7.0, 5.0, 1.0, 2.0, 9.0],
            vec![1.0, 1.0, 0.0, 1.0, 1.0],
            vec![7.0, 4.0, 1.0, 9.0, 7.0],
            vec![2.0, 7.0, 1.0, 8.0, 7.0],
        ];

        let w1 = wasserstein_1(&mu, &nu, &distance);
        let want = 49.0 / 12.0;
        assert!(
            (w1 - want).abs() < 1e-9,
            "W1 on the zero-padded #387 3x4 instance: got {w1}, want {want}"
        );
    }

    /// Disjoint supports whose only cross costs are `f64::INFINITY`: no
    /// coupling of finite cost exists, so W₁ is infinite.
    #[test]
    fn w1_unreachable_supports_is_infinite() {
        let mu = vec![1.0, 0.0];
        let nu = vec![0.0, 1.0];
        let distance = vec![vec![0.0, f64::INFINITY], vec![f64::INFINITY, 0.0]];

        let w1 = wasserstein_1(&mu, &nu, &distance);
        assert!(
            w1.is_infinite() && w1 > 0.0,
            "W1 across an infinite ground cost should be +inf, got {w1}"
        );
    }
}
