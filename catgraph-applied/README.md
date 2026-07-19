# catgraph-applied

Applied category theory extensions for [catgraph](../catgraph). Anchored to [Fong & Spivak, *Seven Sketches in Compositionality* (arXiv:1803.05316v3, 2018)](https://arxiv.org/abs/1803.05316), Chapters 4–6.

## Overview

This crate packages applied-CT modules that build on catgraph's strict Fong-Spivak 2019 core but are not part of the 2019 paper's numbered content. It is the applied-CT complement to the F&S core crate.

## Modules

| Module | Purpose |
|---|---|
| `decorated_cospan` | Generic `Decoration` trait + `DecoratedCospan<Lambda, D>` realizing F&S Def 6.75 + Thm 6.77 |
| `wiring_diagram` | Operadic substitution built on named cospans |
| `petri_net` | Place/transition nets with cospan bridge, firing, reachability, parallel/sequential composition, `HypergraphCategory` impl, `PetriDecoration` bridge to `DecoratedCospan` |
| `temperley_lieb` | Temperley-Lieb / Brauer algebra via perfect matchings |
| `linear_combination` | Formal linear combinations over a coefficient ring |
| `e1_operad` | Little-intervals operad (E₁) |
| `e2_operad` | Little-disks operad (E₂) |
| `prop` | Symmetric strict monoidal categories with `Ob = ℕ` and the free prop `Free(G)` on a signature (F&S Def 5.2, Def 5.25); `Presentation<G>` with 9-rule SMC quotient (Def 5.33) |
| `operad_algebra` | Single-sorted operad algebras `F : O → Set` with concrete `CircAlgebra` for `WiringDiagram` (F&S Def 6.99, Ex 6.100) |
| `operad_functor` | Functors between operads with the canonical `E₁ ↪ E₂` inclusion (F&S Rough Def 6.98) |
| `rig` | `Rig` trait (semiring) + `BoolRig`, `UnitInterval`, `Tropical`, `F64Rig` instances (F&S Def 5.36) |
| `sfg` | `SignalFlowGraph<R>` — free prop on signal-flow generators (F&S Def 5.45) |
| `mat` | `MatR<R>` — pure-rig matrix prop over any `Rig` R (F&S Def 5.50) |
| `mat_kron` | `MatKron<R>` — Kronecker-tensor matrix prop: a genuine hypergraph category over a rig with Hadamard SCFM (η/ε/μ/δ) as inherent generators; speciality δ;μ = id (F&S 2019 *Hypergraph Categories* Ex 2.16, §2.3) |
| `trace` | Partial trace `Tr_X(f)` for `MatKron<R>`, built from its compact-closed cup/cap generators (strict Kronecker; no associators) (F&S 2019 §3.1) |
| `sfg_to_mat` | `sfg_to_mat` functor `S: SFG_R → Mat(R)` (F&S Thm 5.53) |
| `graphical_linalg` | `matr_presentation<R>` — 18-equation Thm 5.60 presentation of Mat(R) (F&S §5.4; closed via the Functorial engine — see `prop::presentation::functorial`) |
| `mat_f64` (feature `f64-rig`) | nalgebra bridge for `MatR<F64Rig>`: determinant, inverse, `DMatrix` roundtrip |
| `prop::presentation::kb` | Congruence-closure decision procedure (DST 1980 signature-table variant) — the default `eq_mod` backend, with an atom-canonical `smc_refine` fixpoint (BoolRig d=2 collisions 2574 → 1433 → 1301 post-#14 → 1142 post-E_18) |
| `prop::presentation::smc_nf` | Layer 1 Joyal-Street string-diagram normal form — canonicalizes `PropExpr` up to SMC coherence (associator, unitors, interchange, braid naturality, σ²=id) (JS 1991 Part I, Selinger 2011) |
| `prop::presentation::functorial` | `CompleteFunctor<G>` trait + `MatrixNFFunctor<R>` — opt-in semantic decision engine for prop-equality via `Presentation::eq_mod_functorial`. Complete by theorem for Mat(R) (F&S Thm 5.60; proof via Baez-Erbele 2015 for fields, Wadsley–Woods arXiv:1505.00048 for commutative rigs, cf. BE15 §6) |
| `enriched` | `EnrichedCategory<V>` trait + `HomMap<O, V>` finite realization (F&S §1.1, §2.4) |
| `lawvere_metric` | `LawvereMetricSpace<T>` over `Tropical` — triangle-inequality verifier + `-ln π` embedding from `UnitInterval` (Lawvere 1973) |
| `integer` | `ZAlgebra` trait (sealed) — `Rig` extension with `Neg + Sub + from_i64` for rings carrying integer-exact arithmetic. Canonical ring-homomorphism ℤ → R (Bourbaki *Algèbre* Ch. I §8). Renamed from `Integer`; sealed via `private::Sealed` so only `Z(BigInt)` and `i64` carry it |
| `z` | `Z(BigInt)` newtype — arbitrary-precision `ZAlgebra + Ring` instance using `num-bigint`. Substrate for integer-exact Möbius and multi-prime CRT SNF lift in catgraph-magnitude |
| `hypergraph` | `Hypergraph<V, HE>` — zero-dependency CRUD hypergraph container (K1 backend for the downstream koalisi coalition layer — private repo, sustia-llc/koalisi#4). Stable never-reused `VertexIndex`/`HyperedgeIndex`, ordered duplicate-allowing hyperedge lists, `Copy` weights by value; idempotent `add_hyperedge` (smallest matching index), cascading `remove_vertex`, `join`/`reverse`/`contract`; `hyperedge_as_cospan` categorical view = the identity cospan over the member index list (`Cospan<VertexIndex>`; a composition handle within applied, not the magnitude consumer path — that path is `get_hyperedge_vertices` → couplings → `coalition_value`, dedup first). Deliberate yamafaktory v4.2.0 divergences: no-op updates return `Ok` (makes `try_join_coalition` idempotency true), infallible clears, bounds relaxed to `Copy + Eq + Debug`, no serde (#23; workspace tag v0.1.0) |

### Design notes

- **`ZAlgebra` seal.** `ZAlgebra` (`Rig + Neg + Sub + from_i64`) captures
  ℤ-algebra structure — any ring R is canonically a ℤ-algebra via the unique
  ring-hom ℤ → R. It is sealed via `private::Sealed`, so only the in-crate
  types `Z(BigInt)` and `i64` carry it; `F64Rig` *cannot* implement it (floats
  lack injectivity of `from_i64`), which the seal enforces.
- **Ring/field bound placement.** `F64Rig` carries `Neg`/`Sub`/`Div`/`From<f64>`
  (the ring + field operations that `catgraph-magnitude`'s `mobius_function`
  Gaussian elimination needs). That bound stays off `Rig` itself — only `F64Rig`
  carries it — so rigs without subtraction (`BoolRig`, `Tropical`) remain valid
  `Rig` instances.
- **`MatR<Q>` in-place mutators** (`row_swap`, `scale_row`, `add_scaled_row`,
  and the column duals, plus `entries_mut`/`entry_mut`) are the substrate for
  the Storjohann SNF port in `catgraph-magnitude`.
- **Thm 5.60 test naming.** The 12 integration tests in `tests/graphical_linalg.rs`
  are named `cc_completeness_tracking_*` (not `thm_5_60_faithful_*`): they measure
  the default CC engine's syntactic incompleteness vs the matrix ground truth, not
  Thm 5.60 itself (F&S Thm 5.60 proves that abstractly — via Baez-Erbele 2015 for
  fields, Wadsley–Woods arXiv:1505.00048 for commutative rigs). They stay
  `#[ignore]`'d as a diagnostic.

## Dependency on catgraph

Every module depends on catgraph's public API:

- `Cospan`, `NamedCospan`, `Span`, `Rel` — pushout/pullback composition
- `Frobenius` generators — operadic composition of SMCs (Prop 3.8)
- `HypergraphCategory` trait — target for semantic functors
- `Operadic` trait — abstract substitution interface (concrete impls live here)
- `compact_closed` cup/cap — string-diagram rewriting (TL, wiring)

## Paper alignment

See [`docs/FS18-AUDIT.md`](docs/FS18-AUDIT.md) for the section-by-section Seven Sketches coverage audit (Chapters 4–6, 63 items tracked; 82% of implementable items DONE, §5.4 Thm 5.60 closed via Functorial engine). Cross-linked from [`../catgraph/docs/FS19-AUDIT.md`](../catgraph/docs/FS19-AUDIT.md) "Reconciliation" section.

## Changelog

See [`CHANGELOG.md`](CHANGELOG.md) for release history.

## Build

```sh
cargo test -p catgraph-applied
cargo clippy -p catgraph-applied -- -W clippy::pedantic
```

## WASM support

`[features] parallel` (default-on) gates the `rayon` + `rayon-cond`
dependencies and the four `CondIterator` call sites in
`linear_combination::Mul::mul`, `linear_combination::linear_combine`, and
`temperley_lieb::BrauerMorphism::non_crossing` (source + target sides).
Disable with `--no-default-features` for single-threaded WASI hosts.

```sh
cargo build --lib -p catgraph-applied --target wasm32-wasip1-threads
cargo build --lib -p catgraph-applied --target wasm32-wasip1 --no-default-features
```

See `examples/wasi_smoke_applied.rs` for a minimal `LinearCombination`
multiplication smoke test exercising the `CondIterator` parallel arm.

See `examples/agent_hypergraph.rs` for a worked agent-coalition registry over
the K1 `Hypergraph` — the full coalition lifecycle (member read, join with the
no-op re-join divergence, leave, merge, dissolve, agent-removal cascade, index
stability) plus the `hyperedge_as_cospan` categorical view. Run with
`cargo run -p catgraph-applied --example agent_hypergraph`.

## License

MIT.
