# catgraph

Category-theoretic graph structures in Rust, anchored to the source papers:
a strict implementation of Fong & Spivak, *Hypergraph Categories* (2019), with
applied-CT, magnitude, Wolfram-physics, and categorical-deep-learning extensions.

> **Status:** active reboot in progress. Crates land phase-by-phase on a thin
> [DeepCausality](https://github.com/deepcausality-rs/deep_causality) algebraic
> substrate (`deep_causality_num` / `deep_causality_haft`), with `nalgebra` kept
> optional and numeric-only.

## Workspace

| Crate | Paper anchor |
|---|---|
| `catgraph` | Fong & Spivak 2019 — *Hypergraph Categories* |
| `catgraph-applied` | Fong & Spivak 2018 — *Seven Sketches in Compositionality* |
| `catgraph-magnitude` | Bradley–Vigneaux 2025; Leinster 2008/2013/2017 |
| `catgraph-physics` | Wolfram-physics extensions (DPO rewriting, multiway, branchial) |
| `catgraph-dl` | Gavranović et al., ICML 2024 — *Categorical Deep Learning* |

## Build

```sh
cargo build  --workspace
cargo test   --workspace
cargo clippy --workspace --all-targets -- -W clippy::pedantic
```

## License

MIT — see [`LICENSE`](LICENSE).
