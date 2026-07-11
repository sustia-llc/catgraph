# catgraph

Category-theoretic graph structures in Rust, anchored to the source papers:
a strict implementation of Fong & Spivak, *Hypergraph Categories* (2019), with
applied-CT, magnitude, Wolfram-physics, and categorical-deep-learning extensions.

> **Status:** the five proven crates (core / applied / magnitude / physics / dl)
> have landed on a thin
> [DeepCausality](https://github.com/deepcausality-rs/deep_causality) algebraic
> substrate (`deep_causality_num` / `deep_causality_haft`), with `nalgebra` kept
> optional and numeric-only. Versioning is workspace-wide (tags v0.1.0 → v0.2.1)
> and work is tracked as GitHub issues. Phase 6 (`catgraph-syntax`, the Arrow
> presentation frontend, #5): the S1–S5 milestone surface is **complete**
> (printer, parser + presentation files, interpreter, Frobenius layer, Traced
> typed builder); post-milestone follow-ups are tracked on #5 (#79/#80/#81).

## Workspace

| Crate | Paper anchor |
|---|---|
| `catgraph` | Fong & Spivak 2019 — *Hypergraph Categories* |
| `catgraph-applied` | Fong & Spivak 2018 — *Seven Sketches in Compositionality* |
| `catgraph-magnitude` | Bradley–Vigneaux 2025; Leinster 2008/2013/2017 |
| `catgraph-physics` | Wolfram-physics extensions (DPO rewriting, multiway, branchial) |
| `catgraph-dl` | Gavranović et al., ICML 2024 — *Categorical Deep Learning* |
| `catgraph-syntax` | F&S 2018 Ch. 5 (props/presentations) + F&S 2019 (Frobenius layer); term language over `catgraph-applied`'s NF engine |

## Build

```sh
cargo build  --workspace
cargo test   --workspace
cargo clippy --workspace --all-targets -- -D warnings          # the CI gate (default lints)
cargo clippy --workspace --all-targets -- -W clippy::pedantic  # advisory local pass (non-gating)
```

## License

MIT — see [`LICENSE`](LICENSE).
