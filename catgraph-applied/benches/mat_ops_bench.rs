//! Bench groups for `MatR<R>` + the mutable row/column API +
//! `MatR::block_diagonal`.
//!
//! Paper anchors:
//! - **FS18 §5.3 Def 5.50** — `MatR<R>` as the pure-rig matrix prop.
//!   Fong-Spivak, *Seven Sketches in Compositionality* (arXiv:1803.05316v3).
//! - **Storjohann 2000 §7** — the mutable row/column API is the perf
//!   substrate for the Smith Normal Form pipeline in
//!   `catgraph-magnitude` (`snf::{band, bidiagonal, mod}`).
//!
//! The foundational `mat_ops` bench file that the cg-mag
//! SNF benches reference to isolate the SNF-internal cost from
//! the underlying matrix-op cost.
//!
//! ## Bench-size bracket
//!
//! - **`matmul` groups: `n ∈ {8, 16, 32, 64, 128}`.** n=128 covers the
//!   LAPACK-vs-pure-Rust crossover for f64 dense matmul, which sits at ~n=64. The
//!   earlier bracket `{8, 16, 32, 64}` topped out exactly at the
//!   crossover, missing the regime where a hypothetical future LAPACK
//!   bridge would unambiguously win on FFI-overhead. n=128 adds one data
//!   point above the inflection so a future regression in either regime
//!   is visible. The matmul body is `Rig::clone`-bound at the cell level
//!   (`MatR::matmul` triple loop with `R::clone()` per `mul` per cell);
//!   the `BoolRig` clone-contrast variant isolates the Copy-vs-Clone cost.
//!   The `n=128` f64 measurement serves as the **pre-LAPACK regression
//!   baseline** for future `mat_f64` LAPACK-bridge work. When that
//!   bridge lands, this measurement becomes the "before" half of the
//!   speedup ratio.
//!
//! - **`mutable_ops` + `block_diagonal` groups: `n ∈ {8, 16, 32}`.** These
//!   characterise a different perf regime — in-place state transitions on
//!   the mutable API (cg-mag SNF substrate) and tensor structure
//!   cost (`sfg_to_mat` Tensor-node hot path) respectively. Neither hits
//!   the LAPACK crossover, so the smaller bracket is sufficient.
//!
//! ## Reproducibility
//!
//! Fixtures are built from `StdRng::seed_from_u64(SEED)` with a single
//! file-level seed (`SEED = 0xCAFE_BABE_DEAD_BEEF`). Every bench iteration
//! receives a `clone()` of the prebuilt fixture; the RNG is consumed
//! once at fixture-construction time, never inside `bencher.iter`.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

use catgraph_applied::mat::MatR;
use catgraph_applied::rig::{BoolRig, F64Rig};

/// Single file-level seed. Documented in the module rustdoc so a future
/// reader knows that adding new bench groups should reuse it (or thread
/// a per-group seed if needed for independence).
const SEED: u64 = 0xCAFE_BABE_DEAD_BEEF;

// ---------------------------------------------------------------------------
// Fixture builders
// ---------------------------------------------------------------------------

/// Build a dense `n × n` `MatR<F64Rig>` with entries sampled uniformly
/// from `[0.0, 1.0)`. The sampling range is bounded so the matmul result
/// stays well inside f64 representable range (no overflow risk through
/// the full `n=128` bracket: max entry product ~`128 * 1.0 * 1.0 = 128`).
fn fixture_matr_f64_random(n: usize, rng: &mut StdRng) -> MatR<F64Rig> {
    let entries: Vec<Vec<F64Rig>> = (0..n)
        .map(|_| (0..n).map(|_| F64Rig(rng.random_range(0.0..1.0))).collect())
        .collect();
    MatR::new(n, n, entries).expect("fixture: well-formed rectangular MatR<F64Rig>")
}

/// Build a dense `n × n` `MatR<BoolRig>` with entries sampled with
/// `Bernoulli(0.5)`. Per the design doc clone-cost-contrast rationale:
/// `bool` is `Copy`, so the matmul inner loop sees zero clone allocation
/// per cell — contrast against `F64Rig` which is also `Copy` but routes
/// through `Rig::clone()` calls at the trait level.
fn fixture_matr_bool_random(n: usize, rng: &mut StdRng) -> MatR<BoolRig> {
    let entries: Vec<Vec<BoolRig>> = (0..n)
        .map(|_| {
            (0..n)
                .map(|_| BoolRig(rng.random_range(0.0..1.0) < 0.5))
                .collect()
        })
        .collect();
    MatR::new(n, n, entries).expect("fixture: well-formed rectangular MatR<BoolRig>")
}

// ---------------------------------------------------------------------------
// Group 1 — `mat_ops::matmul` (LAPACK-crossover-characterising)
// ---------------------------------------------------------------------------

fn bench_matmul_f64(c: &mut Criterion) {
    let mut group = c.benchmark_group("mat_ops::matmul::f64");
    let mut rng = StdRng::seed_from_u64(SEED);

    for &n in &[8usize, 16, 32, 64, 128] {
        let a = fixture_matr_f64_random(n, &mut rng);
        let b = fixture_matr_f64_random(n, &mut rng);

        // Throughput in elements: an n×n matmul touches n² output cells
        // each computed via an n-length dot product. Reporting per-element
        // gives a comparable rate across the bracket; per-FMA would
        // require `n³` and obscures the cache-locality story.
        group.throughput(Throughput::Elements((n * n) as u64));

        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |bencher, _| {
            bencher.iter(|| {
                // Explicit `drop` makes the anti-elision intent
                // structural rather than relying on heap-allocation
                // side-effects of `Vec<Vec<R>>` to prevent compiler
                // folding (Specialist M-3 latent-elision hazard).
                drop(black_box(black_box(&a).matmul(black_box(&b))));
            });
        });
    }
    group.finish();
}

fn bench_matmul_bool(c: &mut Criterion) {
    let mut group = c.benchmark_group("mat_ops::matmul::bool");
    let mut rng = StdRng::seed_from_u64(SEED);

    for &n in &[8usize, 16, 32, 64, 128] {
        let a = fixture_matr_bool_random(n, &mut rng);
        let b = fixture_matr_bool_random(n, &mut rng);

        group.throughput(Throughput::Elements((n * n) as u64));

        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |bencher, _| {
            bencher.iter(|| {
                drop(black_box(black_box(&a).matmul(black_box(&b))));
            });
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Group 2 — `mat_ops::mutable_ops` (in-place API; cg-mag SNF substrate)
// ---------------------------------------------------------------------------

/// Run the fixed Storjohann-style sequence on `m`:
///
/// ```text
/// row_swap(0, 1)
/// scale_row(0, 2.0)
/// add_scaled_row(0, 1, 3.0)
/// col_swap(0, 1)
/// scale_col(0, 2.0)
/// add_scaled_col(0, 1, 3.0)
/// ```
///
/// repeated 4 times. Per the design doc: "`row_swap` + `scale_row` +
/// `add_scaled_row` + col-side duals → repeat 4 times". Each iteration
/// touches O(n) cells; the 4-fold repeat amortises criterion's
/// measurement-overhead constant.
fn run_mutable_ops_sequence(m: &mut MatR<F64Rig>) {
    let two = F64Rig(2.0);
    let three = F64Rig(3.0);
    for _ in 0..4 {
        m.row_swap(0, 1).expect("row_swap in-bounds");
        m.scale_row(0, &two).expect("scale_row in-bounds");
        m.add_scaled_row(0, 1, &three)
            .expect("add_scaled_row in-bounds, dst != src");
        m.col_swap(0, 1).expect("col_swap in-bounds");
        m.scale_col(0, &two).expect("scale_col in-bounds");
        m.add_scaled_col(0, 1, &three)
            .expect("add_scaled_col in-bounds, dst != src");
    }
}

fn bench_mutable_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("mat_ops::mutable_ops");
    let mut rng = StdRng::seed_from_u64(SEED);

    for &n in &[8usize, 16, 32] {
        let fixture = fixture_matr_f64_random(n, &mut rng);

        // The 4-fold loop touches 6 ops × 4 repeats × n cells = 24n
        // cells per iteration. Reporting `Elements(24 * n)` gives an
        // accurate absolute throughput in criterion's HTML report
        // (Specialist M-2 — earlier `Elements(n)` understated the
        // work by 24× and required consumer-side multiplication).
        group.throughput(Throughput::Elements((24 * n) as u64));

        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |bencher, _| {
            bencher.iter(|| {
                // Clone the fixture inside the iter so each iteration
                // starts from the same state. The clone cost is included
                // in the measurement but is O(n²) and the same across
                // the bracket, so it does not distort the n-axis trend.
                let mut m = fixture.clone();
                run_mutable_ops_sequence(black_box(&mut m));
                black_box(m);
            });
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Group 3 — `mat_ops::block_diagonal` (sfg_to_mat Tensor-node hot path)
// ---------------------------------------------------------------------------

fn bench_block_diagonal(c: &mut Criterion) {
    let mut group = c.benchmark_group("mat_ops::block_diagonal");
    let mut rng = StdRng::seed_from_u64(SEED);

    for &n in &[8usize, 16, 32] {
        let a = fixture_matr_f64_random(n, &mut rng);
        let b = fixture_matr_f64_random(n, &mut rng);

        // The output is `2n × 2n`, but the work is dominated by the two
        // `n × n` copies into the diagonal blocks — Elements(2n²) is the
        // natural unit (no multiplies, just clones).
        group.throughput(Throughput::Elements((2 * n * n) as u64));

        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |bencher, _| {
            bencher.iter(|| {
                // `block_diagonal` returns `Self` (not `Result`); using
                // `drop` instead of `let _ =` removes the "eliding an
                // error" reading and resolves the asymmetry with the
                // Result-returning matmul call sites above (CQ Minor-2 +
                // Specialist M-3 merged).
                drop(black_box(MatR::block_diagonal(
                    black_box(&a),
                    black_box(&b),
                )));
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_matmul_f64,
    bench_matmul_bool,
    bench_mutable_ops,
    bench_block_diagonal
);
criterion_main!(benches);
