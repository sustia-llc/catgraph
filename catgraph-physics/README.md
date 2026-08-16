# catgraph-physics

Wolfram-physics extensions for [catgraph](../catgraph/): hypergraph DPO rewriting, multiway evolution tracking, gauge theory, and branchial spectral analysis.

Part of the [catgraph workspace](https://github.com/sustia-llc/catgraph).
Paper provenance (this crate is inspiration-anchored, not theorem-anchored):
[`docs/ANCHORS.md`](docs/ANCHORS.md).

## Modules

| Module | Purpose |
|--------|---------|
| `hypergraph/` | Hypergraph DPO rewriting, evolution tracking, categorical span/cospan bridges, lattice gauge theory |
| `multiway/` | Generic multiway (non-deterministic) evolution graphs, branchial foliation, Ollivier-Ricci curvature, Wasserstein transport |
| `multiway/branchial_spectrum.rs` | Graph Laplacian eigendecomposition: algebraic connectivity (λ₂), spectral gap, Fiedler vector, spectral clustering |
| `multiway/branchial_analysis.rs` | Graph algorithms via rustworkx-core: greedy coloring, k-core decomposition, articulation points; betweenness and Katz centrality on multiway evolution graphs. Also an all-pairs shortest-path pass, but that one is **`pub(crate)`** — it exists to feed `ollivier_ricci`, and is not re-exported from `multiway` |

## Dependencies

- `catgraph` — core F&S types (`Composable`, `Cospan`, `Span`)
- `nalgebra` — dense spectral analysis (`SymmetricEigen` on the branchial
  Laplacian), gated behind the default-on `spectral` feature (gates
  `multiway::branchial_spectrum`; opt out with `--no-default-features` to drop
  the nalgebra stack for slim / WASM builds).
- `petgraph` + `rustworkx-core` — graph algorithms, gated behind the default-on
  `rustworkx` feature (gates `multiway::branchial_analysis`; opt out with
  `--no-default-features` to drop the `rustworkx-core` → `petgraph` chain).
  Retained deliberately, and not tracked for replacement: greedy coloring,
  k-core, and the Brandes betweenness sweep are not the kind of short,
  self-contained pass the toposort and connectivity sites turned out to be
  (#220). This bullet is the live rationale for the dependency; the
  `Cargo.toml` comments point here rather than at the closed issue that
  originally added the gate.

  Since #162 the gate also selects the all-pairs shortest-path sweep inside
  `OllivierRicciCurvature::from_branchial`: rustworkx-core's `distance_matrix`
  when the feature is on, a hand-rolled queue BFS in `ollivier_ricci.rs` when it
  is off. Same unweighted hop metric either way — the curvature values do not
  depend on the feature, and the crate's exact-valued curvature tests run on
  both paths. The `Vec<Vec<f64>>` at that boundary is a deliberate copy out of
  rustworkx's `ndarray` return: `ndarray` stays undeclared here so it can stay
  inside the one chain this one gate drops.

  `rustworkx-core` is *additionally* a `[dev-dependencies]` entry (#163), for
  its seeded topology generators in proptest and bench fixtures. That edge is
  dev-only and does not widen the published dependency tree.

## Build

```sh
cargo test -p catgraph-physics
cargo clippy -p catgraph-physics -- -W clippy::pedantic
cargo bench -p catgraph-physics --bench wasserstein_bench
cargo bench -p catgraph-physics --bench branchial_bench
```

## WASM support

`[features] parallel` (default-on) is a pass-through of `catgraph/parallel`,
and it wires the upstream toggle through so `--no-default-features` produces a
single-threaded catgraph dep transitively.

Since #161 this crate also hands work to rayon, at **two sites**, both inside
rustworkx-core and both gated the same way — with `parallel` off the threshold
is `usize::MAX`, so no node count reaches it and the sweep is always
sequential:

| site | mechanism | threshold | bit-reproducible on the parallel path? |
|------|-----------|----------:|----------------------------------------|
| `multiway_betweenness` (#161) | `CondIterator` over the Brandes sweep | 50 | **No** — partials accumulate into a shared buffer, so f64 summation order varies |
| `OllivierRicciCurvature::from_branchial` (#162) | `distance_matrix`'s `into_par_iter()` over rows | 300 | **Yes** — disjoint rows, integer hop counts, nothing accumulated |

The gate matters for more than tidiness: on a no-threads wasm target an ungated
rayon path fails at *runtime* on rayon's global pool init, and only for graphs
large enough to cross the threshold. Build without `parallel` if you need
pinnable betweenness scores; the distances are pinnable either way.

`--no-default-features` also
drops the `rustworkx` feature (the `rustworkx-core` → `petgraph` chain) and
the `spectral` feature (the nalgebra stack behind `BranchialSpectrum`),
which is what makes the plain `wasm32-wasip1` build slim. Both WASI
sub-targets build clean:

```sh
cargo build --lib -p catgraph-physics --target wasm32-wasip1-threads
cargo build --lib -p catgraph-physics --target wasm32-wasip1 --no-default-features
```

See `examples/wasi_smoke_physics.rs` for a minimal hypergraph-construction
smoke test.

## Changelog

See [`CHANGELOG.md`](CHANGELOG.md) for release history.

## License

MIT — see [LICENSE](../LICENSE).
