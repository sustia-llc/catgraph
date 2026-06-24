//! Span composition (pullback) benchmarks.
//!
//! Measures `Span::<()>::compose` at increasing boundary sizes.
//! Pullback composition is the dual of pushout (cospan) composition.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use catgraph::category::Composable;
use catgraph::span::Span;

/// Build two composable `Span<()>` of boundary size `n`.
///
/// Both spans have `left = right = vec![(); n]` and
/// `middle = (0..n).map(|i| (i, i)).collect()`.
/// The right labels of the first match the left labels of the second
/// (all `()`), so composition is valid.
fn make_composable_pair(n: usize) -> (Span<()>, Span<()>) {
    let labels = vec![(); n];
    let middle: Vec<(usize, usize)> = (0..n).map(|i| (i, i)).collect();
    let a = Span::new(labels.clone(), labels.clone(), middle.clone());
    let b = Span::new(labels.clone(), labels, middle);
    (a, b)
}

fn bench_pullback(c: &mut Criterion) {
    let mut group = c.benchmark_group("pullback_compose");

    for size in [4, 16, 64, 256, 1024] {
        let (a, b) = make_composable_pair(size);

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |bencher, _| {
            bencher.iter(|| a.compose(&b).unwrap());
        });
    }

    group.finish();
}

criterion_group!(benches, bench_pullback);
criterion_main!(benches);
