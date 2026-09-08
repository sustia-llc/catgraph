//! `wasserstein_1` metric axioms and mass behaviour on adversarial marginals
//! ([#225](https://github.com/sustia-llc/catgraph/issues/225), item 3).
//!
//! # What this is
//!
//! `src/multiway/wasserstein.rs`'s inline tests assert identity, symmetry,
//! Dirac-to-Dirac, the triangle inequality and two rational instances on benign
//! inputs — masses in twelfths, integer costs, totals of 1. This file asserts
//! the same axioms over a generated corpus of *line instances*: support points
//! drawn from the full `f64` exponent range, marginals carrying zeros and
//! entries spanning many decades of weight, and total masses drawn
//! log-uniformly across the band the solver admits.
//!
//! The ground metric is `d(i, j) = |xᵢ − xⱼ|` on sorted support points, so the
//! cost matrix is the distance matrix of a line rather than an arbitrary
//! non-negative matrix, and the transport optimum has the closed form
//! `Σₖ |Σ_{i≤k}(μᵢ − νᵢ)| · (x_{k+1} − xₖ)` — an oracle independent of the
//! min-cost-flow solver.
//!
//! # The three absolute constants that bound the input space
//!
//! `wasserstein.rs` measures mass against absolute thresholds, so its behaviour
//! is not scale-free:
//!
//! - `EPS = 1e-12` as a **total**-mass floor: a total below it returns `0.0`.
//! - `EPS = 1e-12` as a **per-arc capacity** floor: Dijkstra skips any residual
//!   arc of capacity `<= EPS`, so a marginal entry at or below `1e-12` is never
//!   routed.
//! - the mass-balance assertion `|Σμ − Σν| < 1e-9`: absolute, so at total mass
//!   `T` it is satisfiable only while the floating-point summation error of
//!   `Σν`, of order `n · ulp(T)`, stays below `1e-9`.
//!
//! The capacity floor has a consequence the inline tests do not reach:
//! `capacity_floor_breaks_the_triangle_inequality_by_the_mass_it_drops` builds
//! a two-point instance whose dropped `1e-13` entry makes `W(μ, ν)` come back
//! `0.0` against a non-zero closed form, and the triangle inequality fail by
//! exactly that entry's transport cost.
//! `one_homogeneity_low_edge_is_the_capacity_floor` and the two
//! `a_one_ulp_mass_imbalance_*` pins scan the decades of total mass on two
//! fixed instances and pin which of them are homogeneous, which the floor
//! zeroes out, and where the balance assertion starts rejecting.
//!
//! # The generated corpus
//!
//! Instances are narrowed to the regime the axioms survive, and the narrowing
//! is the finding:
//!
//! - every marginal entry is exactly zero or above `MIN_ENTRY = 1e-9` — entries
//!   under the capacity floor are snapped to zero before balancing;
//! - the two marginals of a pair sum to **bitwise** equal totals, not merely to
//!   within the assertion's `1e-9`;
//! - every non-zero partial CDF difference `Σ_{i≤k}(μᵢ − νᵢ)` — the mass the
//!   optimal coupling moves across gap `k` — exceeds `CUM_FRACTION = 1e-6` of
//!   the total, since below that it is comparable to `n · ulp(total)`, the
//!   rounding of the two totals themselves;
//! - total masses run over `10^MASS_EXP_LO ..= 10^MASS_EXP_HI`, and
//!   `total_mass × support_span` is a normal `f64`.
//!
//! Support points still range over the full `f64` exponent range within that,
//! and `census_of_the_generated_corpus` pins the widest largest-to-smallest
//! entry ratio the corpus reaches.
//!
//! The generated-corpus properties are asserted at relative tolerance
//! `REL = 1e-9` with an absolute floor
//! `ABS_FRACTION × total_mass × support_span` — the accumulated rounding bound
//! for a sum of at most `2n` products of that size. The census also counts how
//! often the relative half is the binding one, so the floor cannot make them
//! vacuous unnoticed. `w1_of_a_marginal_with_itself_is_zero` asserts exact
//! equality instead, and the decade scans compare at `REL` alone.

use catgraph_physics::multiway::wasserstein_1;
use catgraph_testutil::{approx_rel, strategy::wide_range_f64};
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;

/// Lower decade exponent of the generated total mass, inclusive.
const MASS_EXP_LO: i32 = -6;
/// Upper decade exponent of the generated total mass, inclusive.
const MASS_EXP_HI: i32 = 5;

/// Smallest non-zero marginal entry the generator emits; entries at or below it
/// are snapped to zero.
const MIN_ENTRY: f64 = 1e-9;

/// Smallest non-zero partial CDF difference the generator emits, as a fraction
/// of the total mass.
const CUM_FRACTION: f64 = 1e-6;

/// Relative tolerance every property below is asserted at.
const REL: f64 = 1e-9;

/// Absolute tolerance as a fraction of `total_mass × support_span`.
const ABS_FRACTION: f64 = 1e-14;

/// Number of raw draws the corpus census takes.
const CENSUS_DRAWS: usize = 4_000;

/// Largest-to-smallest marginal-entry ratio the census requires the corpus to
/// reach.
const MIN_WIDEST_RATIO: f64 = 1e12;

// ---------------------------------------------------------------------------
// instance construction
// ---------------------------------------------------------------------------

/// A line instance: sorted support points, the ground metric they induce, and
/// three marginals of the same total mass over them.
#[derive(Debug, Clone)]
struct Instance {
    /// Support points, ascending and distinct.
    x: Vec<f64>,
    /// `d[i][j] = |x[i] − x[j]|`.
    d: Vec<Vec<f64>>,
    /// `[μ, ν, ρ]`, each summing to bitwise the same total.
    m: [Vec<f64>; 3],
    /// The drawn total mass.
    scale: f64,
    /// `x[n − 1] − x[0]`.
    span: f64,
}

impl Instance {
    /// The absolute tolerance for this instance: `ABS_FRACTION × scale × span`.
    fn abs_tol(&self) -> f64 {
        ABS_FRACTION * self.scale * self.span
    }
}

/// One unnormalised weight: the unit band, an exact zero, or a magnitude in
/// `[1e-15, 1e0)` whose decade is sampled, so each decade carries equal weight.
fn weight_entry() -> impl Strategy<Value = f64> {
    prop_oneof![
        4 => 0.0f64..1.0,
        1 => Just(0.0f64),
        3 => (1.0f64..10.0, 1i32..=15).prop_map(|(m, e)| m * 10f64.powi(-e)),
    ]
}

/// A total mass whose decade is drawn uniformly from
/// `[MASS_EXP_LO, MASS_EXP_HI]`, so each decade of the band carries equal
/// weight.
fn mass_scale() -> impl Strategy<Value = f64> {
    (1.0f64..10.0, MASS_EXP_LO..=MASS_EXP_HI).prop_map(|(m, e)| m * 10f64.powi(e))
}

/// The unfiltered draw an [`Instance`] is built from: raw support points, three
/// weight vectors, and a total mass.
type RawDraw = (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, f64);

/// A raw draw of `2..=6` support points, three weight vectors of the same
/// length, and a total mass.
fn raw_draw() -> impl Strategy<Value = RawDraw> {
    (2usize..=6).prop_flat_map(|n| {
        (
            prop::collection::vec(wide_range_f64(), n),
            prop::collection::vec(weight_entry(), n),
            prop::collection::vec(weight_entry(), n),
            prop::collection::vec(weight_entry(), n),
            mass_scale(),
        )
    })
}

/// `w` normalised to total mass `scale`, with entries at or below [`MIN_ENTRY`]
/// snapped to zero.
///
/// `None` when the weights sum to zero, when the scaling leaves the finite
/// range, or when the snapping empties the marginal.
fn snapped(w: &[f64], scale: f64) -> Option<Vec<f64>> {
    let s: f64 = w.iter().sum();
    if !s.is_finite() || s <= 0.0 {
        return None;
    }
    let v: Vec<f64> = w
        .iter()
        .map(|e| {
            let m = e / s * scale;
            if m <= MIN_ENTRY { 0.0 } else { m }
        })
        .collect();
    let total: f64 = v.iter().sum();
    if !total.is_finite() || total <= 0.0 {
        return None;
    }
    Some(v)
}

/// `v` rescaled so its entries sum to **exactly** `target`, the residual being
/// folded into the largest entry and the fold repeated until the sum lands.
///
/// Bitwise equality rather than the solver's `1e-9` slack: an imbalance of `δ`
/// caps the max flow at `min(Σμ, Σν)`, and the closed form assumes it does not,
/// so `δ` would enter the oracle comparison as a relative error of
/// `δ / (transported mass)` with no bound of its own.
///
/// `None` when the correction drives the fold entry to [`MIN_ENTRY`] or below,
/// or when the sum does not land on `target` within eight folds.
fn rebalanced(v: &[f64], target: f64) -> Option<Vec<f64>> {
    let s: f64 = v.iter().sum();
    if !s.is_finite() || s <= 0.0 {
        return None;
    }
    let k = target / s;
    let mut out: Vec<f64> = v.iter().map(|e| e * k).collect();
    let idx = (0..out.len()).max_by(|&p, &q| out[p].total_cmp(&out[q]))?;
    for _ in 0..8 {
        let sum: f64 = out.iter().sum();
        if sum == target {
            break;
        }
        out[idx] += target - sum;
    }
    if !out[idx].is_finite() || out[idx] <= MIN_ENTRY {
        return None;
    }
    if out.iter().sum::<f64>() != target {
        return None;
    }
    Some(out)
}

/// Whether every partial sum `Σ_{i≤k}(aᵢ − bᵢ)` — the mass the optimal coupling
/// moves across gap `k` — is either zero or above both [`MIN_ENTRY`] and
/// `CUM_FRACTION × total`.
///
/// At or below [`MIN_ENTRY`] the whole of it can be stranded by the per-arc
/// capacity floor without the `1e-9` shortfall guard firing, since the guard's
/// threshold is [`MIN_ENTRY`]. At or below `CUM_FRACTION × total` it is
/// comparable to `n · ulp(total)`, the rounding of the two totals themselves:
/// bitwise-equal sums still stand for exact values that far apart, and the
/// closed form charges the difference to the transported mass.
fn routable(a: &[f64], b: &[f64], total: f64) -> bool {
    let floor = MIN_ENTRY.max(CUM_FRACTION * total);
    let mut cum = 0.0_f64;
    for k in 0..a.len() - 1 {
        cum += a[k] - b[k];
        if cum != 0.0 && cum.abs() <= floor {
            return false;
        }
    }
    true
}

/// The instance a raw draw denotes, or `None` when the draw is inadmissible:
/// fewer than two distinct support points, a support span or a cost scale
/// outside the normal `f64` range, a marginal emptied by the [`MIN_ENTRY`]
/// snap, marginals that cannot be balanced to a common total, or a pair of them
/// that is not [`routable`].
fn build(raw: &RawDraw) -> Option<Instance> {
    let (points, a, b, c, scale) = raw;

    let mut x = points.clone();
    x.sort_by(f64::total_cmp);
    x.dedup();
    let n = x.len();
    if n < 2 {
        return None;
    }
    let span = x[n - 1] - x[0];
    let cost_scale = scale * span;
    if !cost_scale.is_finite() || cost_scale < f64::MIN_POSITIVE {
        return None;
    }

    let mu = snapped(&a[..n], *scale)?;
    let target: f64 = mu.iter().sum();
    let nu = rebalanced(&snapped(&b[..n], *scale)?, target)?;
    let rho = rebalanced(&snapped(&c[..n], *scale)?, target)?;
    if !routable(&mu, &nu, target) || !routable(&nu, &rho, target) || !routable(&mu, &rho, target) {
        return None;
    }

    let d: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..n).map(|j| (x[i] - x[j]).abs()).collect())
        .collect();

    Some(Instance {
        x,
        d,
        m: [mu, nu, rho],
        scale: *scale,
        span,
    })
}

/// Admissible line instances.
fn instance() -> impl Strategy<Value = Instance> {
    raw_draw().prop_filter_map("admissible line instance", |raw| build(&raw))
}

// ---------------------------------------------------------------------------
// oracles
// ---------------------------------------------------------------------------

/// `dᵀ`.
fn transpose(d: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let rows = d.len();
    let cols = d[0].len();
    (0..cols)
        .map(|j| (0..rows).map(|i| d[i][j]).collect())
        .collect()
}

/// `Σₖ |Σ_{i≤k}(μᵢ − νᵢ)| · (x_{k+1} − xₖ)` over ascending support points: the
/// closed form of `W₁` under the ground metric `|xᵢ − xⱼ|`.
fn cdf_w1(x: &[f64], mu: &[f64], nu: &[f64]) -> f64 {
    let mut cum = 0.0_f64;
    let mut total = 0.0_f64;
    for k in 0..x.len() - 1 {
        cum += mu[k] - nu[k];
        total = cum.abs().mul_add(x[k + 1] - x[k], total);
    }
    total
}

/// The relative gap between `a` and `b`, `0.0` when both are zero.
fn rel_gap(a: f64, b: f64) -> f64 {
    let scale = a.abs().max(b.abs());
    if scale == 0.0 {
        0.0
    } else {
        (a - b).abs() / scale
    }
}

// ---------------------------------------------------------------------------
// properties
// ---------------------------------------------------------------------------

proptest! {
    /// `W(μ, μ) = 0` exactly, for each of the three marginals.
    #[test]
    fn w1_of_a_marginal_with_itself_is_zero(inst in instance()) {
        for (which, mu) in inst.m.iter().enumerate() {
            let w = wasserstein_1(mu, mu, &inst.d);
            prop_assert!(
                w == 0.0,
                "W(m[{which}], m[{which}]) = {w:e}, expected exactly 0 \
                 (scale = {:e}, support = {:?})",
                inst.scale,
                inst.x
            );
        }
    }

    /// `W(μ, ν)` under `d` and `W(ν, μ)` under `dᵀ` agree. A line metric is
    /// symmetric, so `dᵀ` holds the same values as `d` and what varies between
    /// the two calls is the argument order.
    #[test]
    fn w1_is_symmetric_under_transposing_the_cost_matrix(inst in instance()) {
        let dt = transpose(&inst.d);
        let [mu, nu, _] = &inst.m;
        let forward = wasserstein_1(mu, nu, &inst.d);
        let reverse = wasserstein_1(nu, mu, &dt);
        prop_assert!(
            approx_rel(forward, reverse, REL, inst.abs_tol()),
            "asymmetric: forward = {forward:e}, reverse = {reverse:e} \
             (relative gap = {:e}, allowed {REL:e} relative or {:e} absolute)",
            rel_gap(forward, reverse),
            inst.abs_tol()
        );
    }

    /// `W(c·δᵢ, c·δⱼ) = c · d(i, j)`, for the instance's total mass `c`.
    #[test]
    fn w1_of_dirac_pairs_is_the_scaled_ground_distance(
        inst in instance(),
        i in any::<prop::sample::Index>(),
        j in any::<prop::sample::Index>(),
    ) {
        let n = inst.x.len();
        let (i, j) = (i.index(n), j.index(n));
        let want = inst.scale * inst.d[i][j];
        prop_assume!(want.is_finite());

        let mut mu = vec![0.0; n];
        let mut nu = vec![0.0; n];
        mu[i] = inst.scale;
        nu[j] = inst.scale;

        let w = wasserstein_1(&mu, &nu, &inst.d);
        prop_assert!(
            approx_rel(w, want, REL, inst.abs_tol()),
            "W(delta_{i}, delta_{j}) = {w:e}, expected {want:e} \
             (relative gap = {:e}, allowed {REL:e} relative or {:e} absolute, \
             scale = {:e}, d = {:e})",
            rel_gap(w, want),
            inst.abs_tol(),
            inst.scale,
            inst.d[i][j]
        );
    }

    /// `W(μ, ρ) ≤ W(μ, ν) + W(ν, ρ)`.
    #[test]
    #[allow(clippy::similar_names)]
    fn w1_satisfies_the_triangle_inequality(inst in instance()) {
        let [mu, nu, rho] = &inst.m;
        let w_mu_nu = wasserstein_1(mu, nu, &inst.d);
        let w_nu_rho = wasserstein_1(nu, rho, &inst.d);
        let w_mu_rho = wasserstein_1(mu, rho, &inst.d);
        let bound = w_mu_nu + w_nu_rho;
        let slack = (REL * bound.abs()).max(inst.abs_tol());
        prop_assert!(
            w_mu_rho <= bound + slack,
            "triangle violated: W(mu,rho) = {w_mu_rho:e} > \
             W(mu,nu) + W(nu,rho) = {bound:e} (excess = {:e}, allowed = {slack:e})",
            w_mu_rho - bound
        );
    }

    /// `W₁` equals the sorted-CDF integral.
    #[test]
    fn w1_on_a_line_matches_the_sorted_cdf_integral(inst in instance()) {
        let [mu, nu, _] = &inst.m;
        let w = wasserstein_1(mu, nu, &inst.d);
        let want = cdf_w1(&inst.x, mu, nu);
        prop_assert!(
            approx_rel(w, want, REL, inst.abs_tol()),
            "solver {w:e} vs CDF integral {want:e} \
             (relative gap = {:e}, allowed {REL:e} relative or {:e} absolute, \
             scale = {:e}, support = {:?})",
            rel_gap(w, want),
            inst.abs_tol(),
            inst.scale,
            inst.x
        );
    }

    /// `W(cμ, cν) = c · W(μ, ν)`, for `c` leaving the scaled total inside the
    /// generated mass band and the scaled marginals inside the balance
    /// assertion.
    #[test]
    fn w1_is_one_homogeneous_in_mass(inst in instance(), c in mass_scale()) {
        let [mu, nu, _] = &inst.m;
        let total: f64 = mu.iter().sum();
        let scaled_total = c * total;
        prop_assume!(scaled_total >= 10f64.powi(MASS_EXP_LO));
        prop_assume!(scaled_total <= 10f64.powi(MASS_EXP_HI));
        prop_assume!(mu.iter().chain(nu).all(|&e| e == 0.0 || e * c > MIN_ENTRY));

        let cmu: Vec<f64> = mu.iter().map(|v| v * c).collect();
        let cnu: Vec<f64> = nu.iter().map(|v| v * c).collect();
        let sum_cmu: f64 = cmu.iter().sum();
        let sum_cnu: f64 = cnu.iter().sum();
        prop_assume!((sum_cmu - sum_cnu).abs() < 1e-9);

        let base = wasserstein_1(mu, nu, &inst.d);
        let want = c * base;
        prop_assume!(want.is_finite());

        let scaled = wasserstein_1(&cmu, &cnu, &inst.d);
        let abs_tol = ABS_FRACTION * scaled_total * inst.span;
        prop_assert!(
            approx_rel(scaled, want, REL, abs_tol),
            "W(c*mu, c*nu) = {scaled:e}, expected c*W(mu,nu) = {want:e} \
             (c = {c:e}, relative gap = {:e}, allowed {REL:e} relative or \
             {abs_tol:e} absolute)",
            rel_gap(scaled, want)
        );
    }
}

// ---------------------------------------------------------------------------
// censuses
// ---------------------------------------------------------------------------

/// `count` values drawn from `strategy` with a fixed seed.
fn sample<T>(strategy: &impl Strategy<Value = T>, count: usize) -> Vec<T> {
    let mut runner = TestRunner::deterministic();
    (0..count)
        .map(|_| {
            strategy
                .new_tree(&mut runner)
                .expect("invariant: raw_draw has no failing generation path")
                .current()
        })
        .collect()
}

/// Over `CENSUS_DRAWS` fixed-seed raw draws: how many `build` admits, how many
/// admitted instances trip the solver's total-mass early return or come back
/// non-finite, how many are decided by the relative half of the tolerance
/// rather than the absolute floor, the widest largest-to-smallest entry ratio
/// reached, and the largest relative gap against the CDF oracle.
#[test]
fn census_of_the_generated_corpus() {
    let raws = sample(&raw_draw(), CENSUS_DRAWS);

    let mut rejected = 0_usize;
    let mut admitted = 0_usize;
    let mut sub_eps_totals = 0_usize;
    let mut non_finite = 0_usize;
    let mut relative_binding = 0_usize;
    let mut widest_ratio = 0.0_f64;
    let mut worst_gap = 0.0_f64;
    let mut worst_at: Option<(Vec<f64>, f64, f64)> = None;

    for raw in &raws {
        let Some(inst) = build(raw) else {
            rejected += 1;
            continue;
        };
        admitted += 1;

        let [mu, nu, _] = &inst.m;
        if mu.iter().sum::<f64>() < 1e-12 {
            sub_eps_totals += 1;
        }
        for marginal in &inst.m {
            let positive: Vec<f64> = marginal.iter().copied().filter(|&e| e > 0.0).collect();
            let smallest = positive.iter().copied().fold(f64::INFINITY, f64::min);
            let largest = positive.iter().copied().fold(0.0_f64, f64::max);
            if smallest.is_finite() && smallest > 0.0 {
                widest_ratio = widest_ratio.max(largest / smallest);
            }
        }

        let w = wasserstein_1(mu, nu, &inst.d);
        if !w.is_finite() {
            non_finite += 1;
        }
        let want = cdf_w1(&inst.x, mu, nu);
        if REL * w.abs().max(want.abs()) > inst.abs_tol() {
            relative_binding += 1;
        }
        let gap = rel_gap(w, want);
        if gap > worst_gap {
            worst_gap = gap;
            worst_at = Some((inst.x.clone(), w, want));
        }
    }

    assert_eq!(
        rejected + admitted,
        CENSUS_DRAWS,
        "census lost a draw: {rejected} rejected + {admitted} admitted"
    );
    assert!(
        admitted > CENSUS_DRAWS / 2,
        "only {admitted} of {CENSUS_DRAWS} draws were admissible, expected \
         more than {}",
        CENSUS_DRAWS / 2
    );
    assert_eq!(
        non_finite, 0,
        "{non_finite} of {admitted} admitted instances returned a non-finite \
         W1 over a finite cost matrix, expected 0"
    );
    assert_eq!(
        sub_eps_totals, 0,
        "{sub_eps_totals} of {admitted} admitted instances have a total mass \
         below the solver's EPS = 1e-12 total-mass early return, expected 0"
    );
    assert!(
        relative_binding * 20 > admitted * 17,
        "the relative half of the tolerance decides only {relative_binding} of \
         {admitted} instances, expected more than {}; below that the absolute \
         floor is carrying the property",
        admitted * 17 / 20
    );
    assert!(
        widest_ratio >= MIN_WIDEST_RATIO,
        "the widest largest-to-smallest entry ratio in the corpus is \
         {widest_ratio:e}, expected at least {MIN_WIDEST_RATIO:e} — the \
         marginals are not reaching the near-degenerate regime"
    );
    assert!(
        worst_gap <= REL,
        "largest relative gap against the CDF integral over {admitted} \
         instances is {worst_gap:e}, expected at most {REL:e}; worst case \
         {worst_at:?}"
    );
}

/// How one decade of total mass classifies for a fixed pair of marginals.
#[derive(Debug, PartialEq)]
enum Decade {
    /// `W(cμ, cν)` agreed with `c · W(μ, ν)` to [`REL`].
    Homogeneous,
    /// `W(cμ, cν)` came back `0.0` against a non-zero `c · W(μ, ν)`.
    Zeroed,
    /// Neither, carrying the observed and expected values.
    Mismatched(f64, f64),
}

/// Classify the decades `10^lo ..= 10^hi` of total mass for `(mu, nu)` under
/// the ground metric `d`, returning `(exponent, verdict)` per decade.
///
/// Every decade calls the solver; a decade whose scaled marginals trip one of
/// its input assertions propagates that panic rather than being classified.
fn scan_decades(mu: &[f64], nu: &[f64], d: &[Vec<f64>], lo: i32, hi: i32) -> Vec<(i32, Decade)> {
    let base = wasserstein_1(mu, nu, d);
    (lo..=hi)
        .map(|e| {
            let c = 10f64.powi(e);
            let cmu: Vec<f64> = mu.iter().map(|v| v * c).collect();
            let cnu: Vec<f64> = nu.iter().map(|v| v * c).collect();
            let got = wasserstein_1(&cmu, &cnu, d);
            let want = c * base;
            let verdict = if approx_rel(got, want, REL, 0.0) {
                Decade::Homogeneous
            } else if got == 0.0 && want != 0.0 {
                Decade::Zeroed
            } else {
                Decade::Mismatched(got, want)
            };
            (e, verdict)
        })
        .collect()
}

/// The one-ulp-imbalanced marginals the mass-balance pins scan, and the ground
/// metric they live on: `ν`'s total is one ulp above `μ`'s, the tightest
/// non-exact balance a unit total admits.
fn one_ulp_imbalanced_fixture() -> (Vec<f64>, Vec<f64>, Vec<Vec<f64>>) {
    let x: Vec<f64> = vec![-1.0, 0.0, 2.5, 7.0];
    let d: Vec<Vec<f64>> = (0..x.len())
        .map(|i| (0..x.len()).map(|j| (x[i] - x[j]).abs()).collect())
        .collect();
    let mu = vec![0.5, 0.25, 0.125, 0.125];
    let nu = vec![0.125, 0.125, 0.25, 0.5 + f64::EPSILON];
    (mu, nu, d)
}

/// The exponents in `scan` whose verdict is `want`.
fn decades_with(scan: &[(i32, Decade)], want: &Decade) -> Vec<i32> {
    scan.iter()
        .filter(|(_, v)| v == want)
        .map(|&(e, _)| e)
        .collect()
}

/// On dyadic marginals — whose totals stay bit-equal under any rescaling, so
/// the mass-balance assertion does not fire anywhere in the scan — the low edge
/// of one-homogeneity is the per-arc capacity floor: below `1e-11` total mass
/// every entry falls at or below `EPS = 1e-12` and `W` comes back `0.0`.
#[test]
fn one_homogeneity_low_edge_is_the_capacity_floor() {
    let x: Vec<f64> = vec![-1.0, 0.0, 2.5, 7.0];
    let d: Vec<Vec<f64>> = (0..x.len())
        .map(|i| (0..x.len()).map(|j| (x[i] - x[j]).abs()).collect())
        .collect();
    let mu = vec![0.5, 0.25, 0.125, 0.125];
    let nu = vec![0.125, 0.125, 0.25, 0.5];

    let scan = scan_decades(&mu, &nu, &d, -18, 18);
    let mismatched: Vec<_> = scan
        .iter()
        .filter(|(_, v)| matches!(v, Decade::Mismatched(_, _)))
        .collect();
    assert!(
        mismatched.is_empty(),
        "decades that are neither homogeneous nor zeroed: {mismatched:?}"
    );

    let holds = decades_with(&scan, &Decade::Homogeneous);
    let zeroed = decades_with(&scan, &Decade::Zeroed);

    assert_eq!(
        zeroed,
        (-18..=-12).collect::<Vec<_>>(),
        "expected the capacity floor to zero decades -18..=-12, got {zeroed:?}"
    );
    assert_eq!(
        holds,
        (-11..=18).collect::<Vec<_>>(),
        "expected homogeneity on decades -11..=18, got {holds:?}"
    );
}

/// A one-ulp mass imbalance is still accepted at a total mass of `1e6`, and
/// one-homogeneity holds from the capacity floor up to it.
#[test]
fn a_one_ulp_mass_imbalance_is_accepted_up_to_a_total_mass_of_1e6() {
    let (mu, nu, d) = one_ulp_imbalanced_fixture();
    let residual = nu.iter().sum::<f64>() - mu.iter().sum::<f64>();
    assert!(
        residual > 0.0 && residual < 1e-15,
        "the fixture must carry a one-ulp imbalance, got {residual:e}"
    );

    let scan = scan_decades(&mu, &nu, &d, -18, 6);
    let mismatched: Vec<_> = scan
        .iter()
        .filter(|(_, v)| matches!(v, Decade::Mismatched(_, _)))
        .collect();
    assert!(
        mismatched.is_empty(),
        "decades that are neither homogeneous nor zeroed: {mismatched:?}"
    );

    let holds = decades_with(&scan, &Decade::Homogeneous);
    let zeroed = decades_with(&scan, &Decade::Zeroed);
    assert_eq!(
        zeroed,
        (-18..=-12).collect::<Vec<_>>(),
        "expected the capacity floor to zero decades -18..=-12, got {zeroed:?}"
    );
    assert_eq!(
        holds,
        (-11..=6).collect::<Vec<_>>(),
        "expected homogeneity on decades -11..=6 at a residual of \
         {residual:e}, got {holds:?}"
    );
}

/// The `|Σμ − Σν| < 1e-9` assertion is absolute, so the one-ulp imbalance
/// accepted at a total mass of `1e6` is rejected one decade higher.
#[test]
#[should_panic(expected = "Total masses must be equal")]
fn a_one_ulp_mass_imbalance_is_rejected_at_a_total_mass_of_1e7() {
    let (mu, nu, d) = one_ulp_imbalanced_fixture();
    let c = 1e7;
    let cmu: Vec<f64> = mu.iter().map(|v| v * c).collect();
    let cnu: Vec<f64> = nu.iter().map(|v| v * c).collect();
    let _ = wasserstein_1(&cmu, &cnu, &d);
}

/// A two-point instance whose `ν` carries `1e-13` at the far support point:
/// that entry sits at or below the solver's per-arc capacity floor, so it is
/// never routed, `W(μ, ν)` comes back `0.0` against the closed form's `1e-13`,
/// and `W(μ, ρ) ≤ W(μ, ν) + W(ν, ρ)` fails by that entry's transport cost.
#[test]
#[allow(clippy::similar_names)]
fn capacity_floor_breaks_the_triangle_inequality_by_the_mass_it_drops() {
    let x: Vec<f64> = vec![0.0, 1.0];
    let d = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
    let dropped = 1e-13;

    let mu = vec![1.0, 0.0];
    let nu = vec![1.0 - dropped, dropped];
    let rho = vec![0.0, 1.0];

    let w_mu_nu = wasserstein_1(&mu, &nu, &d);
    let w_nu_rho = wasserstein_1(&nu, &rho, &d);
    let w_mu_rho = wasserstein_1(&mu, &rho, &d);
    let oracle_mu_nu = cdf_w1(&x, &mu, &nu);

    assert!(
        oracle_mu_nu > 0.0,
        "the closed form must see the dropped entry: got {oracle_mu_nu:e}, \
         expected a positive value near {dropped:e}"
    );
    assert!(
        w_mu_nu == 0.0,
        "W(mu, nu) = {w_mu_nu:e}, expected exactly 0 — the {dropped:e} entry \
         is at or below the capacity floor and is never routed, while the \
         closed form gives {oracle_mu_nu:e}"
    );

    let excess = w_mu_rho - (w_mu_nu + w_nu_rho);
    assert!(
        excess > 0.0,
        "expected the triangle inequality to fail: W(mu,rho) = {w_mu_rho:e}, \
         W(mu,nu) + W(nu,rho) = {:e}, excess = {excess:e}",
        w_mu_nu + w_nu_rho
    );
    assert!(
        approx_rel(excess, oracle_mu_nu, 1e-9, 0.0),
        "the excess should be the dropped mass's transport cost: \
         excess = {excess:e}, |mu_0 - nu_0| * d(0,1) = {oracle_mu_nu:e} \
         (relative gap = {:e})",
        rel_gap(excess, oracle_mu_nu)
    );
}
