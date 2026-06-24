# catgraph

Category-theoretic graph structures in Rust — strict Fong & Spivak,
*Hypergraph Categories* (2019), plus applied / magnitude / physics / DL extensions.

## Build & test

```sh
cargo build  --workspace
cargo test   --workspace          # every change: green before merge
cargo clippy --workspace --all-targets -- -W clippy::pedantic
cargo fmt    --all --check
```

## Crate graph (dependency order)

```
catgraph (F&S core) ─▶ catgraph-applied ─▶ catgraph-magnitude
        └─▶ catgraph-physics              └─▶ catgraph-dl
```

`deep_causality_num` / `deep_causality_haft` pinned `=0.3.3` (fallback git rev `b1aba1e`).
Adopted incrementally from Phase 2; not all crates depend on them yet.

## Paper anchors

- **catgraph** — Fong & Spivak 2019 (*Hypergraph Categories*)
- **catgraph-applied** — Fong & Spivak 2018 (*Seven Sketches in Compositionality*)
- **catgraph-magnitude** — Bradley–Vigneaux 2025; Leinster 2008 / 2013 / 2017
- **catgraph-dl** — Gavranović et al., ICML 2024 (*Categorical Deep Learning*)

## Rules (the only ones)

1. **The paper is the spec.** Theorems move/stay intact — no re-derivation.
2. **`Rig` is a semiring** (catgraph-native). Never swap it for a `deep_causality_num`
   `Ring` — DC's lowest ring requires `Sub`; `BoolRig` / `Tropical` have none.
   Re-source only `Zero` / `One` from `deep_causality_num`.
3. **Integer SNF / `Z(BigInt)` / Storjohann / Newman stay custom** (DC has no integer-ring SNF).
4. **Every change is green** `cargo test --workspace` + clippy before merge.

Work is tracked as GitHub issues. Contributing: see [`CONTRIBUTING.md`](CONTRIBUTING.md).

> **Status:** active reboot. The proven crates land phase-by-phase; this skeleton is Phase 0.
