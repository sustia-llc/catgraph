//! Benchmark fixtures for `multiway::branchial_analysis` and
//! `multiway::branchial_spectrum` (issue #163).
//!
//! # Why the topologies come from rustworkx-core
//!
//! `MultiwayEvolutionGraph` is constructible only through `add_root` /
//! `add_fork` / `add_sequential_step`, so every branchial graph reachable from
//! it is a star's cross-section — i.e. Kₙ. Benchmarking the graph algorithms on
//! Kₙ alone measures one point of the (n, m) plane, and the dense point at
//! that. `BranchialGraph`'s `nodes` / `edges` are public, so a seeded
//! rustworkx-core generator can be reinterpreted as a branchial graph directly
//! and the fixtures can span sparse, scale-free, and dense shapes.
//!
//! Every seed is a pinned literal, so the fixtures are byte-identical across
//! runs. Topology only — numeric fixtures elsewhere in the workspace stay on
//! the byte-identity-pinned `catgraph-testutil` LCG (#33).
//!
//! The benches are serial-friendly: each fixture is built once, up front, and
//! the measured closures read shared immutable state only.
//!
//! ```sh
//! cargo bench -p catgraph-physics --bench branchial_bench
//! ```

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rustworkx_core::generators::{barabasi_albert_graph, gnp_random_graph};
use rustworkx_core::petgraph::graph::UnGraph;

use catgraph_physics::multiway::{
    BranchId, BranchialGraph, BranchialSpectrum, MultiwayNodeId, branchial_articulation_points,
    branchial_coloring, branchial_core_numbers,
};

/// Reinterpret a generated undirected graph as a branchial graph at step 1.
///
/// Mirrors the translation used by `tests/branchial_analysis.rs`; kept local
/// because Cargo gives benches and integration tests no shared module.
fn branchial_from_ungraph(g: &UnGraph<(), ()>) -> BranchialGraph {
    let nodes: Vec<MultiwayNodeId> = (0..g.node_count())
        .map(|i| MultiwayNodeId::new(BranchId(i), 1))
        .collect();
    let edges = g
        .edge_indices()
        .filter_map(|e| g.edge_endpoints(e))
        .map(|(a, b)| (nodes[a.index()], nodes[b.index()]))
        .collect();
    BranchialGraph {
        step: 1,
        nodes,
        edges,
    }
}

/// Sparse Erdős–Rényi G(n, p), pinned seed.
fn erdos_renyi(n: usize, p: f64, seed: u64) -> BranchialGraph {
    let g: UnGraph<(), ()> = gnp_random_graph(n, p, Some(seed), || (), || ())
        .expect("gnp_random_graph arguments are in range");
    branchial_from_ungraph(&g)
}

/// Barabási–Albert scale-free graph, pinned seed.
fn barabasi_albert(n: usize, m: usize, seed: u64) -> BranchialGraph {
    let g: UnGraph<(), ()> = barabasi_albert_graph(n, m, Some(seed), None, || (), || ())
        .expect("barabasi_albert_graph arguments are in range");
    branchial_from_ungraph(&g)
}

fn branchial_analysis_benchmark(c: &mut Criterion) {
    // (label, graph) — two sizes × two topology families.
    let fixtures = [
        ("er_n100_p0.05", erdos_renyi(100, 0.05, 0x00C0_FFEE)),
        ("ba_n100_m3", barabasi_albert(100, 3, 0x0000_5EED)),
        ("er_n1000_p0.006", erdos_renyi(1000, 0.006, 0x0BAD_C0DE)),
        ("ba_n1000_m3", barabasi_albert(1000, 3, 0x0000_D1CE)),
    ];

    let mut group = c.benchmark_group("branchial_analysis");
    for (label, graph) in &fixtures {
        group.bench_with_input(BenchmarkId::new("coloring", label), graph, |b, g| {
            b.iter(|| branchial_coloring(g));
        });
        group.bench_with_input(BenchmarkId::new("core_numbers", label), graph, |b, g| {
            b.iter(|| branchial_core_numbers(g));
        });
        group.bench_with_input(
            BenchmarkId::new("articulation_points", label),
            graph,
            |b, g| {
                b.iter(|| branchial_articulation_points(g));
            },
        );
    }
    group.finish();
}

fn branchial_spectrum_benchmark(c: &mut Criterion) {
    // `BranchialSpectrum` builds a dense n×n Laplacian and runs
    // `SymmetricEigen`, so cost is Θ(n³) and the sizes stop well short of the
    // n = 1000 used above — n = 300 already dominates the whole bench run.
    let fixtures = [
        ("er_n100_p0.05", erdos_renyi(100, 0.05, 0x00C0_FFEE)),
        ("ba_n200_m3", barabasi_albert(200, 3, 0x0000_5EED)),
        ("er_n300_p0.02", erdos_renyi(300, 0.02, 0x0BAD_C0DE)),
    ];

    let mut group = c.benchmark_group("branchial_spectrum");
    group.sample_size(20);
    for (label, graph) in &fixtures {
        group.bench_with_input(BenchmarkId::new("from_branchial", label), graph, |b, g| {
            b.iter(|| BranchialSpectrum::from_branchial(g));
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    branchial_analysis_benchmark,
    branchial_spectrum_benchmark
);
criterion_main!(benches);
