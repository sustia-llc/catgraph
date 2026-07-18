# catgraph

Strict Rust implementation of [Fong & Spivak, *Hypergraph Categories* (2019)](https://arxiv.org/abs/1806.08304).

Cospans, spans, Frobenius algebras, hypergraph categories, compact closed structure, and the Theorem 1.2 equivalence. This crate tracks the F&S 2019 paper strictly — applied-CT extras and Wolfram-physics extensions live in sibling crates. Rust 2024 edition, zero `unsafe`, criterion benchmarks.

Originally based on a fork of [Cobord/Hypergraph](https://github.com/Cobord/Hypergraph), substantially rewritten to use source/target (cospan) semantics and implement the full F&S paper.

## Component Index

| Module | Component | Purpose |
|--------|-----------|---------|
| `category.rs` | `HasIdentity`, `Composable`, `ComposableMutating` | Core composition traits |
| `cospan.rs` | `Cospan<Lambda>` | Morphisms in Cospan_Λ, pushout composition (union-find) |
| `span.rs` | `Span<Lambda>`, `Rel<Lambda>` | Pullback composition (dual), relation algebra |
| `named_cospan.rs` | `NamedCospan<Lambda, L, R>` | Port-labeled cospans for wiring-style composition |
| `monoidal.rs` | `Monoidal`, `SymmetricMonoidalMorphism`, `GenericMonoidalMorphism` | Tensor product, braiding, generic layered morphisms |
| `frobenius/` | `FrobeniusMorphism`, `MorphismSystem` | String diagram morphisms, DAG-based black-box interpretation |
| `compact_closed.rs` | `cup`, `cap`, `name`, `unname`, `compose_names_direct` | Self-dual compact closed structure (§3.1), Prop 3.3 literal form |
| `cospan_algebra.rs` | `CospanAlgebra`, `PartitionAlgebra`, `NameAlgebra`, `functor_induced_algebra_map` | Lax monoidal functors Cospan → Set (§2.1), Lemma 4.3 natural transformation (io; see Feature Map) |
| `hypergraph_category.rs` | `HypergraphCategory` | Frobenius generators η, ε, μ, δ with cup/cap (§2.3) |
| `hypergraph_functor.rs` | `HypergraphFunctor`, `RelabelingFunctor`, `CospanToFrobeniusFunctor` | Structure-preserving maps between hypergraph categories (§2.3) |
| `equivalence.rs` | `CospanAlgebraMorphism`, `comp_cospan`, `functor_from_algebra_morphism` | §4 equivalence (Thm 1.2 per-Λ form = Thm 4.13, Eq 8; statement in the Feature Map below), Lemma 4.9 io functor |
| `operadic.rs` | `Operadic` | Abstract operadic-substitution trait (Eq 6). Concrete impls live in `catgraph-applied` |
| `finset.rs` | `Permutation`, `Decomposition`, `OrderPresSurj`, `OrderPresInj` | Epi-mono factorization, order-preserving maps |

## Workspace siblings

| Crate | Purpose |
|-------|---------|
| [`catgraph-applied`](../catgraph-applied/) | Petri nets, wiring diagrams, E_n operads, Temperley-Lieb, props, signal-flow graphs — applied-CT extensions (F&S 2018) |
| [`catgraph-magnitude`](../catgraph-magnitude/) | Magnitude of enriched categories + magnitude homology (Bradley–Vigneaux 2025; Leinster) |
| [`catgraph-physics`](../catgraph-physics/) | Hypergraph DPO rewriting, multiway evolution, gauge theory, branchial spectral analysis |
| [`catgraph-dl`](../catgraph-dl/) | Categorical Deep Learning substrate (Gavranović et al., ICML 2024) |

## Fong-Spivak Feature Map

Features implementing structures from [Fong & Spivak, *Hypergraph Categories*](https://arxiv.org/abs/1806.08304). See [`docs/FS19-AUDIT.md`](docs/FS19-AUDIT.md) for the full per-section coverage audit.

| Paper Reference | Module | Summary |
|-----------------|--------|---------|
| Core (§1–2) | `cospan.rs` | `Cospan<Lambda>` — morphisms in Cospan_Λ, composition via pushout (union-find). |
| Core (§1–2) | `span.rs` | `Span<Lambda>` — dual of cospan, composition via pullback. Ex 2.15: Span/Rel. |
| Core | `category.rs` | `HasIdentity`, `Composable`, `ComposableMutating` traits for morphism composition. |
| Core | `monoidal.rs` | `Monoidal`, `SymmetricMonoidalMorphism` traits; tensor product and braiding. |
| Def 2.2 | `cospan_algebra.rs` | `CospanAlgebra` trait — lax monoidal functors Cospan_Λ → C. `PartitionAlgebra` (Ex 2.3, Prop 4.6: initial) and `NameAlgebra` (Prop 4.1). |
| Def 2.5 | `frobenius/` | `FrobeniusMorphism` — string diagram morphisms from the 4 Frobenius generators. `MorphismSystem` DAG for named composition. Ex 2.8: generators as cospans. |
| Def 2.12 | `hypergraph_category.rs` | `HypergraphCategory` trait — Frobenius generators (η, ε, μ, δ) with derived cup/cap. Prop 2.18 (strict case) implicitly satisfied. |
| Def 2.12, Eq 12 | `hypergraph_functor.rs` | `HypergraphFunctor` trait — structure-preserving maps. `RelabelingFunctor` (single-map component of Prop 2.1/Cor 3.13, Eq 9; the cross-Λ functor itself is deferred). |
| Prop 3.1–3.4 | `compact_closed.rs` | Self-dual compact closed — cup/cap (Prop 3.1), name bijection (Prop 3.2), `compose_names_direct` realising the literal Prop 3.3 formula `(f̂ ⊗ ĝ) ; comp^Y_{X,Z}`, Prop 3.4 recovery. Zigzag identities (Eq 13). |
| Lemma 4.3 | `cospan_algebra.rs` | `functor_induced_algebra_map` lifts a `HypergraphFunctor` to a cospan-algebra morphism α: A_H → A_H'. Paper states the lemma for io functors over fixed Λ; the cross-label case is a beyond-paper generalization (Eq 29 direction). |
| Lemma 4.9 | `equivalence.rs` | `functor_from_algebra_morphism` lifts a monoidal natural transformation α: A → B to the induced io hypergraph functor F_α: H_A → H_B. |
| Lemma 3.6, Prop 3.8 | `cospan_algebra.rs`, `hypergraph_functor.rs` | `cospan_to_frobenius` + `CospanToFrobeniusFunctor` — epi-mono decomposition into Frobenius generators. |
| **Thm 1.2** (per-Λ = Thm 4.13) | `equivalence.rs` | `CospanAlgebraMorphism<A>` (Lemma 4.8): cospan-algebra → hypergraph category. `comp_cospan` (Ex 3.5, Eq 32). Identity/Frobenius via Eq 33. Roundtrip: `Hyp_OF(Λ) ≅ Lax(Cospan_Λ, Set)` (Eq 8). |

**Permanently deferred** (documented in [`docs/FS19-AUDIT.md`](docs/FS19-AUDIT.md) — require parametric Λ machinery or 2-category machinery beyond catgraph's current type system):

- Cross-Λ functoriality (Prop 2.1, Cor 3.13, Cor 3.15, Thm 3.14 universal property)
- Thm 1.1 strictification / coherence (Hyp ≃ Hyp_OF)
- §3.3 io/ff factorization (Lemma 3.19, Prop 3.20, Cor 3.21)
- Thm 4.16 global Grothendieck form (per-Λ Thm 4.13 suffices)
- LinRel examples (Ex 2.10, 2.11, 2.20, 2.21, 4.14) — Ex 2.16 (FdVect) is realized in `catgraph-applied::mat_kron`

## Core: Cospans and Spans

Hyperedges connect **source sets** to **target sets** via typed middle sets:

```
    domain          middle         codomain
   [a, b]  ──left──▶ [x, y, z] ◀──right── [c, d]
```

An edge `[a,b] → [c,d]` means a→c, a→d, b→c, b→d (bipartite complete subgraph). This is distinct from path semantics where `[a,b,c,d]` means a→b→c→d.

| Type | Purpose |
|------|---------|
| `Cospan<Lambda>` | Morphisms in Cospan_Lambda. Composition via pushout (union-find, O(n·α(n))). |
| `NamedCospan<Lambda, L, R>` | Port-labeled cospans for wiring-style composition with named boundary nodes. |
| `Span<Lambda>` | Dual of cospan — composition via pullback. |
| `Rel<Lambda>` | Relations as jointly-injective spans. Full relation algebra. |
| `Corel<Lambda>` | Corelations as jointly-surjective cospans. Dual of `Rel`. Implements `HypergraphCategory` (F&S 2018 Ex 6.64). |

`Lambda` types the middle vertices — use `()` for untyped graphs.

## Examples

```bash
cargo run -p catgraph --example cospan
cargo run -p catgraph --example span
cargo run -p catgraph --example named_cospan
cargo run -p catgraph --example monoidal
cargo run -p catgraph --example finset
cargo run -p catgraph --example frobenius
cargo run -p catgraph --example hypergraph_category
cargo run -p catgraph --example compact_closed
cargo run -p catgraph --example cospan_algebra
cargo run -p catgraph --example hypergraph_functor
cargo run -p catgraph --example equivalence
cargo run -p catgraph --example corel
```

## Testing

```bash
cargo test   -p catgraph
cargo test   -p catgraph --examples
cargo clippy -p catgraph --all-targets -- -D warnings
```

## WASM support

catgraph compiles to both `wasm32-wasip1-threads` (preferred, rayon-backed
parallelism via the `wasi-threads` runtime proposal) and `wasm32-wasip1`
(single-threaded, `--no-default-features`). Browsers
(`wasm32-unknown-unknown`) are out of scope — the target audience is edge
devices running Wasmtime / Wasmer / WasmEdge / Fermyon Spin.

The `parallel` feature (default-on) gates the `rayon` dependency and the two
internal `par_iter` call sites in `find_nodes_by_name_predicate` and
`FrobeniusLayer::hflip`. Disable it with `--no-default-features` for
single-threaded WASI hosts.

```sh
cargo build --lib -p catgraph --target wasm32-wasip1-threads
cargo build --lib -p catgraph --target wasm32-wasip1 --no-default-features
```

See `examples/wasi_smoke_core.rs` for a minimal cospan-composition smoke test.

## Dependencies

- `ultragraph` — zero-dependency directed-graph substrate backing `MorphismSystem`'s dependency-graph topological sort
- `itertools` — iterator utilities
- `either` — Left/Right sum type for bipartite node types
- `permutations` — permutation type for symmetric monoidal braiding
- `union-find` — QuickUnionUf for pushout composition
- `rayon` (optional, `parallel` feature) — data parallelism with adaptive thresholds
- `log` — warning messages
- `thiserror` — structured error types
- Dev: `env_logger`, `proptest`, `rand`, `criterion`

## References

- [Fong & Spivak, *Hypergraph Categories* (2019)](https://arxiv.org/abs/1806.08304) — primary theoretical foundation
- [Fong & Spivak, *Seven Sketches in Compositionality* (2018)](https://arxiv.org/abs/1803.05316) — secondary anchor: Thm 6.55 (`tests/spider_theorem.rs`), Ex 6.64 `Corel` (`src/corel.rs`)
- [Spivak, *The Operad of Wiring Diagrams* (2013)](https://arxiv.org/abs/1305.0297) — operadic viewpoint behind `src/operadic.rs` (Eq 6)

## License

[MIT](LICENSE)
