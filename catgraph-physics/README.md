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
| `multiway/branchial_analysis.rs` | Graph algorithms via rustworkx-core: greedy coloring, k-core decomposition, articulation points; betweenness and Katz centrality on multiway evolution graphs |

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

`[features] parallel` (default-on) is a pass-through of `catgraph/parallel`.
This crate has no direct rayon call sites yet; the feature wires the
upstream toggle through so `--no-default-features` produces a
single-threaded catgraph dep transitively. `--no-default-features` also
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
