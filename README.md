# catgraph

Category-theoretic graph structures in Rust, anchored to the source papers:
a strict implementation of Fong & Spivak, *Hypergraph Categories* (2019), with
applied-CT, magnitude, Wolfram-physics, and categorical-deep-learning extensions.

> **Status:** the five proven crates (core / applied / magnitude / physics / dl)
> have landed. The algebraic substrate is entirely catgraph's own (#218,
> completed at #222) — the `Rig` semiring and its `Zero` / `One` identities
> live in `catgraph-applied`, the endofunctor / `Free` / `Cofree` substrate in
> `catgraph-dl`, and the value-level Arrow algebra in `catgraph-syntax`. No
> `deep_causality_*` crate is a dependency anywhere (CI-guarded); the API shape
> of the latter two originated as a port from `deep_causality_haft` 0.4.2, whose
> MIT notice is therefore retained in `THIRD-PARTY.md` and the defining files.
> Zero external algebraic dependencies, with `nalgebra`
> kept optional and numeric-only. Versioning is workspace-wide (tags v0.1.0 → v0.17.0)
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

## Canonical tests

Each published crate carries one canonical integration test,
`<crate>/tests/canonical.rs`: the crate's headline claim, end to end, against
a reference not derived from the implementation. Its header names every
`pub struct|enum|trait|type` under `src` as `covers:` or `not-covered:`, and
`python3 scripts/check_canonical_tests.py` fails CI when a
`pub struct|enum|trait|type` under `src` is in neither list
([`CLAUDE.md`](CLAUDE.md) rule 7).

| File | Claim |
|---|---|
| [`catgraph/tests/canonical.rs`](catgraph/tests/canonical.rs) | Every `HypergraphCategory` implementor in `catgraph/src` satisfies the eleven Def 2.5 equations, the Def 2.12 generator table and both zigzags, decided by `CospanCanon` equality over generator words of length ≤ 3 and every permutation of ≤ 4 wires; `compose` on each equals a union-find partition reference computed from the operand wirings; `wiring(f ⊗ g) == wiring(f) ++ shift(wiring(g))` on every public `Monoidal` implementor in `catgraph/src`. |
| [`catgraph-applied/tests/canonical.rs`](catgraph-applied/tests/canonical.rs) | `S : SFG_R → Mat(R)` (F&S 2018 Thm 5.53) equals a basis-vector evaluator on every term of a depth-bounded corpus over the five `SfgGenerator` variants, over `BoolRig`, `UnitInterval`, `Tropical` and `F64Rig`; `S` commutes with `compose`, `tensor` and `permute_side`; `mat_to_sfg` round-trips through `S` (Prop 5.56); `compose` on `DecoratedCospan` and on `PetriNet` equals the partition reference. |
| [`catgraph-magnitude/tests/canonical.rs`](catgraph-magnitude/tests/canonical.rs) | The four acceptance gates of its README from one file: BV 2025 Prop 3.10's closed form against `LmCategory::magnitude`, and `Mag(2M)` against the hand-computed `2.48`; Rem 3.11 Shannon recovery by central finite difference; Leinster 2013 Prop 2.1.3 chain-sum Möbius against the Gaussian-elimination Möbius; BV 2025 Prop 3.14's magnitude-homology Euler-characteristic identity within an analytical truncation bound. |
| [`catgraph-dl/tests/canonical.rs`](catgraph-dl/tests/canonical.rs) | Every shipped `HKT` witness satisfies the CDL Def 1.4 functor laws, and each witness that presents as a container satisfies the container laws at one sample per constructor of its shape set; the CDL Example B.19 / B.20 bijections round-trip in both directions; each of `FoldingRnn`, `RecursiveNn`, `UnfoldingRnn`, `MealyCell` and `MooreCell` unrolls to what a `Free` / `Cofree` walker computes from the same cell; `unroll_iter` and the two `run_iter`s call their cells exactly once per pulled item. |
| [`catgraph-physics/tests/canonical.rs`](catgraph-physics/tests/canonical.rs) | Inspiration-anchored: `RewriteRule::wolfram_a_to_bb` reaches isomorphic states whose root-based causal graphs compare `Isomorphic`, with holonomy `1.0` on every Wilson loop, `RewriteRule::collapse` has a pair comparing `NotIsomorphic` with holonomy `0.0`, and on every causal-graph shape the two fixtures produce and on hand-built pairs `CausalGraph::compare` agrees with a brute-force permutation search; `run_multiway_bfs` → `BranchialGraph` → `OllivierRicciCurvature` via `wasserstein_1` reproduces hand-computed curvature on a K₄ branchial slice, `wasserstein_1` agrees with exhaustive transport optima on three seeded families, and `to_petgraph` keeps parallel edges and drops an edge with an unregistered endpoint. |
| [`catgraph-syntax/tests/canonical.rs`](catgraph-syntax/tests/canonical.rs) | `parse(print(t)) == t` on a proptest corpus and on a right fold at the parser's deepest accepted nesting; `eval` reproduces `MatrixNFFunctor` row by row and both agree with a hand-written generator table (F&S 2018 Thm 5.53 / 5.60); `CospanFunctor` decides the nine `E_frob` equations equal and separates `braid(1,1)` from `id(2)` (F&S 2019 Prop 3.8); `lift_user` refuses a term past `MAX_TERM_DEPTH`; every `FrobeniusOr` variant survives a JSON round trip. |

## Build

```sh
cargo build  --workspace
cargo test   --workspace
cargo clippy --workspace --all-targets -- -D warnings          # the CI gate (default lints)
cargo clippy --workspace --all-targets -- -W clippy::pedantic  # advisory local pass (non-gating)
```

## License

MIT — see [`LICENSE`](LICENSE).
