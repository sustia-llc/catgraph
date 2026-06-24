# catgraph-physics

Wolfram-physics extensions for [catgraph](../catgraph/): hypergraph DPO rewriting, multiway evolution tracking, gauge theory, and branchial spectral analysis.

Part of the [catgraph workspace](https://github.com/tsondru/catgraph).

## Modules

| Module | Purpose |
|--------|---------|
| `hypergraph/` | Hypergraph DPO rewriting, evolution tracking, categorical span/cospan bridges, lattice gauge theory |
| `multiway/` | Generic multiway (non-deterministic) evolution graphs, branchial foliation, Ollivier-Ricci curvature, Wasserstein transport |
| `multiway/branchial_spectrum.rs` | Graph Laplacian eigendecomposition: algebraic connectivity (λ₂), spectral gap, Fiedler vector, spectral clustering |
| `multiway/branchial_analysis.rs` | Graph algorithms via rustworkx-core: greedy coloring, k-core decomposition, articulation points |

## Dependencies

- `catgraph` — core F&S types (`Composable`, `Cospan`, `Span`)
- `nalgebra` — dense spectral analysis (`SymmetricEigen` on the branchial Laplacian)
- `petgraph` + `rustworkx-core` — graph algorithms

## Build

```sh
cargo test -p catgraph-physics
cargo clippy -p catgraph-physics -- -W clippy::pedantic
cargo bench -p catgraph-physics --bench wasserstein_bench
```

## WASM support (v0.2.2+)

`[features] parallel` (default-on) is a pass-through of `catgraph/parallel`.
This crate has no direct rayon call sites yet; the feature wires the
upstream toggle through so `--no-default-features` produces a
single-threaded catgraph dep transitively. Both WASI sub-targets build
clean:

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
