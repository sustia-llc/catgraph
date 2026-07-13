# Changelog

All notable changes to `catgraph` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the crate adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `cospan_canon` — `CospanCanon<Λ>` and `Cospan::canonical_form`, a decidable
  (hashable, `Eq`) invariant for parallel cospans up to apex isomorphism.
  Records each apex vertex's `(label, sorted dom preimage, sorted cod preimage)`
  as a sorted multiset, so **scalars** (apex-only bubbles) are counted rather
  than collapsed — the *special* (not extra-special) semantics. Enables the
  complete Cospan-valued decision functor in `catgraph-syntax`
  ([#80](https://github.com/sustia-llc/catgraph/issues/80), F&S 2019 Prop 3.8).

## [workspace-v0.1.0] - 2026-07-01

First monorepo release: workspace-wide tag `v0.1.0` (supersedes the pre-reboot
crate-scoped version lineage below). The coalition semantic-layer handoff to
downstream koalisi.

The reboot workspace is being assembled phase by phase toward `0.1.0`. This crate
— the strict implementation of Fong & Spivak, *Hypergraph Categories* (2019) —
is carried intact from prior work into a fresh five-crate workspace built on a
thin [DeepCausality](https://github.com/deepcausality-rs/deep_causality) algebraic
substrate (numeric backends kept optional).

### Added

- `Cospan<Λ>` with pushout composition (union-find, O(n·α(n))); `Span<Λ>` and
  `Rel<Λ>` via pullback (the dual); `Corel<Λ>` — jointly-surjective cospans, the
  dual of `Rel` (FS 2018 Ex 6.64).
- `NamedCospan<Λ, L, R>` — port-labeled cospans for wiring-style composition.
- `Monoidal`, `SymmetricMonoidalMorphism`, `GenericMonoidalMorphism` — tensor
  product and permutation-based braiding.
- `FrobeniusMorphism` + `MorphismSystem` (Def 2.5); `HypergraphCategory` and
  `HypergraphFunctor` (§2.3, Eq 12).
- Self-dual compact closed structure — cup/cap, name/unname, `compose_names_direct`
  (Props 3.1–3.4, zigzag identities Eq 13).
- `CospanAlgebra` with `PartitionAlgebra` (Ex 2.3, Prop 4.6 initiality) and
  `NameAlgebra` (§4.1).
- The §4 equivalence `Hyp_OF ≅ Cospan-Alg` — Theorem 1.2 in its per-Λ form
  (= Thm 4.13), with Lemmas 4.3 / 4.9 and `CospanToFrobeniusFunctor` (Prop 3.8).
- `MorphismSystem` dependency-graph acyclicity (`add_definition_composite`) and
  bottom-up resolution order (`fill_black_boxes`) run on the zero-dependency
  `ultragraph` graph substrate (DeepCausality) via `topological_sort`. `parallel`
  (default-on) feature for rayon at hot call sites. `--no-default-features` yields
  a slim, single-threaded WASI-compatible build.

### Changed

- Graph substrate moved from `rustworkx-core`/`petgraph` to the zero-dependency
  `ultragraph` (DeepCausality) for `MorphismSystem` dependency resolution, dropping
  the `rustworkx-core` → `ndarray` + `serde` transitive chain from this crate. The
  `rustworkx` feature is removed (no slim-vs-full split remains). The speculative
  `Cospan::to_graph` / `NamedCospan::to_graph` petgraph exports — which had no
  in-crate consumers — were removed; they will be reintroduced shaped to a real
  consumer if one materializes.

### Notes

- Test posture: 517 (default and `--no-default-features` now identical — removing
  the `rustworkx` feature collapsed the prior split). Zero `unsafe`.
- Permanently-deferred paper items (cross-Λ functoriality, strictification,
  §3.3 io/ff factorization, the global Grothendieck form, LinRel examples) are
  catalogued in [`docs/FS19-AUDIT.md`](docs/FS19-AUDIT.md).

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.1.0...HEAD
[workspace-v0.1.0]: https://github.com/sustia-llc/catgraph/releases/tag/v0.1.0
