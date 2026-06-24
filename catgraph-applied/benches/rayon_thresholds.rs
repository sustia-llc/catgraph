//! Rayon threshold validation benchmarks.
//!
//! Each group tests sizes below, at, and above the rayon parallel threshold
//! to verify that the threshold is correctly placed.
//!
//! | Module                | Parallelized Operation      | Threshold |
//! |-----------------------|-----------------------------|-----------|
//! | `linear_combination`  | `Mul` impl, `linear_combine`| 32 terms  |
//! | `temperley_lieb`      | `non_crossing` checks       | 8 elements|
//! | `named_cospan`        | `find_nodes_by_name_predicate`| 256 elements |
//! | `frobenius/operations` | `hflip` block mutations    | 64 blocks |

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use catgraph::category::{Composable, ComposableMutating, HasIdentity};
use catgraph::frobenius::FrobeniusMorphism;
use catgraph::named_cospan::NamedCospan;
use catgraph_applied::linear_combination::LinearCombination;
use catgraph_applied::temperley_lieb::BrauerMorphism;

// ---------------------------------------------------------------------------
// 1. LinearCombination Mul (threshold: 32)
// ---------------------------------------------------------------------------

/// Build a `LinearCombination<i64, i64>` with `n` distinct terms.
fn make_linear_combination(n: usize) -> LinearCombination<i64, i64> {
    (0..n as i64).map(|i| (i, 1i64)).collect()
}

fn bench_linear_combination_mul(c: &mut Criterion) {
    let mut group = c.benchmark_group("linear_combination_mul");

    for size in [16, 32, 64, 128] {
        let a = make_linear_combination(size);
        let b = make_linear_combination(size);

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |bencher, _| {
            bencher.iter(|| {
                let _ = a.clone() * b.clone();
            });
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// 2. Temperley-Lieb / Brauer compose (threshold: 8)
// ---------------------------------------------------------------------------

fn bench_temperley_lieb_compose(c: &mut Criterion) {
    let mut group = c.benchmark_group("temperley_lieb_compose");

    for size in [4, 8, 16, 32] {
        // Generate TL e_i generators and symmetric s_i generators at size n.
        let e_gens = BrauerMorphism::<i64>::temperley_lieb_gens(size);
        let s_gens = BrauerMorphism::<i64>::symmetric_alg_gens(size);

        // Compose e_0 with s_0 (both are Hom(n, n)).
        let e = &e_gens[0];
        let s = &s_gens[0];

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |bencher, _| {
            bencher.iter(|| e.compose(s).unwrap());
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// 3. NamedCospan find_nodes_by_name_predicate (threshold: 256)
// ---------------------------------------------------------------------------

fn bench_named_cospan_predicate(c: &mut Criterion) {
    let mut group = c.benchmark_group("named_cospan_predicate");

    for size in [128, 256, 512] {
        // Build a NamedCospan<(), u32, u32> with `size` left ports and 1 right port.
        let indices: Vec<usize> = (0..size).collect();
        let middle = vec![(); size];
        let left_names: Vec<u32> = (0..size as u32).collect();
        let right_names: Vec<u32> = vec![0u32];
        // right leg maps the single right port to middle index 0.
        let right_indices = vec![0usize];

        let named: NamedCospan<(), u32, u32> =
            NamedCospan::new(indices, right_indices, middle, left_names, right_names);

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |bencher, _| {
            bencher
                .iter(|| named.find_nodes_by_name_predicate(|n| n % 2 == 0, |n| n % 2 == 0, false));
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// 4. Frobenius compose / simplify (threshold: 64 blocks for hflip)
// ---------------------------------------------------------------------------

/// Build a `FrobeniusMorphism<(), ()>` with `n` identity blocks (single layer),
/// then compose two such morphisms to trigger `two_layer_simplify` internally.
fn make_identity_morphism(n: usize) -> FrobeniusMorphism<(), ()> {
    FrobeniusMorphism::identity(&vec![(); n])
}

fn bench_frobenius_compose(c: &mut Criterion) {
    let mut group = c.benchmark_group("frobenius_compose");

    for size in [32, 64, 128] {
        let a = make_identity_morphism(size);
        let b = make_identity_morphism(size);

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |bencher, _| {
            bencher.iter(|| {
                let mut lhs = a.clone();
                lhs.compose(b.clone()).unwrap();
                lhs
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_linear_combination_mul,
    bench_temperley_lieb_compose,
    bench_named_cospan_predicate,
    bench_frobenius_compose
);
criterion_main!(benches);
