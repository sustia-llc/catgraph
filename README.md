# catgraph

Category-theoretic graph structures in Rust, anchored to the source papers:
a strict implementation of Fong & Spivak, *Hypergraph Categories* (2019), with
applied-CT, magnitude, Wolfram-physics, and categorical-deep-learning extensions.

> **Status:** the five proven crates (core / applied / magnitude / physics / dl)
> have landed. The algebraic substrate is entirely catgraph's own (#218,
> completed at #222) — the `Rig` semiring and its `Zero` / `One` identities
> live in `catgraph-applied`, the endofunctor / `Free` / `Cofree` substrate in
> `catgraph-dl`, and the value-level Arrow algebra in `catgraph-syntax` (the
> latter two derived from `deep_causality_haft` 0.4.2, MIT; notice in
> `THIRD-PARTY.md` and the defining files) — with zero external algebraic
> dependencies and `nalgebra`
> kept optional and numeric-only. Versioning is workspace-wide (tags v0.1.0 → v0.12.0)
> and work is tracked as GitHub issues. Phase 6 (`catgraph-syntax`, the Arrow
> presentation frontend, #5): the S1–S5 milestone surface is **complete**
> (printer, parser + presentation files, interpreter, Frobenius layer, Traced
> typed builder), and the post-milestone follow-ups have all shipped — #80
> (Cospan-valued complete functor) and #81 (serde) at v0.4.0, #79 (Λ-colored
> props) completed at v0.5.0.

## Workspace

| Crate | Paper anchor |
|---|---|
| `catgraph` | Fong & Spivak 2019 — *Hypergraph Categories*; secondary: F&S 2018 (Thm 6.55 spider tests, Ex 6.64 `Corel`) |
| `catgraph-applied` | Fong & Spivak 2018 — *Seven Sketches in Compositionality* |
| `catgraph-magnitude` | Bradley–Vigneaux 2025; Leinster 2008/2013/2017 |
| `catgraph-physics` | Wolfram-physics extensions (DPO rewriting, multiway, branchial) |
| `catgraph-dl` | Gavranović et al., ICML 2024 — *Categorical Deep Learning* |
| `catgraph-syntax` | F&S 2018 Ch. 5 (props/presentations) + F&S 2019 (Frobenius layer); term language over `catgraph-applied`'s NF engine |
| `catgraph-testutil` (dev-only, unpublished; `[dev-dependencies]` only, #33) | — |

## Build

```sh
cargo build  --workspace
cargo test   --workspace
cargo clippy --workspace --all-targets -- -D warnings          # the CI gate (default lints)
cargo clippy --workspace --all-targets -- -W clippy::pedantic  # advisory local pass (non-gating)
```

## License

MIT — see [`LICENSE`](LICENSE).
