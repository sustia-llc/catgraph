//! Cospan composition (pushout) benchmarks.
//!
//! Measures `Cospan::<()>::compose` at increasing boundary sizes.
//! The pushout uses union-find internally (O(n * α(n))).
//!
//! # Flamegraph
//!
//! ```sh
//! cargo bench --bench pushout -- --profile-time=10
//! # or with cargo-flamegraph:
//! cargo flamegraph --bench pushout -- --bench
//! ```

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use catgraph::category::Composable;
use catgraph::cospan::Cospan;

/// Build two composable `Cospan<()>` of boundary size `n`.
///
/// Both cospans have `middle = vec![(); n]` and `left = right = (0..n).collect()`.
/// The codomain labels of the first match the domain labels of the second
/// (all `()`), so composition is valid.
fn make_composable_pair(n: usize) -> (Cospan<()>, Cospan<()>) {
    let indices: Vec<usize> = (0..n).collect();
    let middle = vec![(); n];
    let a = Cospan::new(indices.clone(), indices.clone(), middle.clone());
    let b = Cospan::new(indices.clone(), indices, middle);
    (a, b)
}

fn bench_pushout(c: &mut Criterion) {
    let mut group = c.benchmark_group("pushout_compose");

    for size in [4, 16, 64, 256, 1024] {
        let (a, b) = make_composable_pair(size);

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |bencher, _| {
            bencher.iter(|| a.compose(&b).unwrap());
        });
    }

    group.finish();
}

criterion_group!(benches, bench_pushout);
criterion_main!(benches);
