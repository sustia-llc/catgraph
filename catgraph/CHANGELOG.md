# Changelog

All notable changes to `catgraph` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the crate adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [workspace-v0.10.0] - 2026-08-09

### Changed

- **`cospan::test::permutatation_manual` uses a literal payload instead of
  `rand::random()`** ([#232](https://github.com/sustia-llc/catgraph/issues/232)).
  Test-only; no public API or behaviour is affected. The five booleans are
  arbitrary payload the test's own assertions carry through, so entropy bought
  nothing — a visible mixed literal replaces the RNG outright and keeps the
  middle-label comparison discriminating by construction. The trigger:
  `rand::random()` needs `rand`'s `thread_rng` feature, which the slimmed
  workspace declaration no longer enables — in physics-free builds; a build
  graph containing `catgraph-physics` unifies rand's defaults back on (see
  `catgraph-applied`'s #232 entry), which is why the literal, not a feature
  bump, is the right fix. `rand` remains a dev-dependency only, now carrying
  `std`/`std_rng` on its own dev edge since the workspace entry is featureless.

## [workspace-v0.9.0] - 2026-08-04

### Changed

- **`MorphismSystem`'s topological sort is now the crate's own, and the
  `ultragraph` dependency is gone**
  ([#220](https://github.com/sustia-llc/catgraph/issues/220), D2 of the
  [#218](https://github.com/sustia-llc/catgraph/issues/218) dependency
  streamlining). Exactly one thing was consumed from that crate —
  `topological_sort` — behind the acyclicity check in
  `add_definition_composite` and the resolution order in `fill_black_boxes`.
  It is replaced by a private Kahn's-algorithm pass in
  `frobenius/morphism_system.rs`, so the crate now depends on no graph crate at
  all. `union-find` (already present for the cospan/corel pushouts) is unchanged
  and moves to a workspace dependency, now that `catgraph-applied` uses it too.
  - Behaviour is unchanged at both call sites: a cycle still yields
    `CatgraphError::Interpret`, and a valid topological order still places each
    parent before its children. The specific order among equally-valid ones may
    differ from the previous implementation's — it always could, since the label
    ids come from `HashMap` iteration.
  - The private `resolve_order` drops its `Result` wrapper: Kahn's only failure
    mode is a cycle, which the `Option` already reports, so the dead
    "graph construction failed" error path is gone. No public signature changes
    — `ultragraph::GraphError` never appeared in one.
  - `toposort` is covered directly for chains, diamonds, isolated nodes,
    duplicate edges, two-node cycles, self-loops, and a cycle sitting beside an
    acyclic component.

## [workspace-v0.6.0] - 2026-08-02

### Changed

- **`equivalence::comp_cospan`'s index arithmetic is documented as bounded, and
  its one unbounded sum is spelled against that bound**
  ([#196](https://github.com/sustia-llc/catgraph/issues/196)). The four index
  sums the issue's inventory names (`m + n`, `m + n + k`) cannot overflow:
  `middle` is built first, and a `Vec` holding `m + n + k` elements is proof
  that the sum fits. The one sum *not* covered by that argument — the left leg's
  own length `m + 2n + k`, which the inventory does not list — is now written as
  `middle.len().saturating_add(n)`, a capacity hint whose bound is visible
  rather than assumed. No saturating sentinel is introduced: unlike a `PropExpr`
  arity, these are lengths of slices the caller already holds.

## [workspace-v0.4.0] - 2026-07-25

### Changed

- **Hardened the core-crate rayon determinism guards ([#48](https://github.com/sustia-llc/catgraph/issues/48))** —
  extended the parallel-vs-sequential equivalence discipline already applied in
  `catgraph-applied` to the core crate's two rayon sites, and to a third,
  previously-untested applied site. The pre-existing `tests/rayon_equivalence.rs`
  guards were upgraded from set-shape / depth-only checks to exact assertions:
  `NamedCospan::find_nodes_by_name_predicate` now compares against an in-test
  hand-rolled sequential reference with exact ordered-`Vec` equality (below and
  above its size threshold), plus the `at_most_one=true` short-circuit and
  no-match cases. `FrobeniusMorphism` / `FrobeniusLayer` `hflip` gains direct
  `#[cfg(test)]` unit tests in `src/frobenius/operations.rs` (reachable in-module:
  `FrobeniusLayer::hflip` is module-private, `FrobeniusMorphism::hflip` is
  `pub(crate)`) asserting sequential-reference equality and the `hflip ∘ hflip ==
  id` involution on layers wide enough that rayon actually subdivides
  (`with_min_len(m)` only splits at length ≥ 2·m), plus public-API determinism
  guards through `special_frobenius_morphism` and
  `cospan_algebra::cospan_to_frobenius`. In `catgraph-applied`,
  `LinearCombination::linear_combine` — a second `CondIterator` dispatch point
  that had no equivalence coverage — gains threshold-straddling par-vs-seq tests
  (including a non-injective combiner that forces coefficient collisions). All
  guards run under both the default and `--no-default-features` builds.

- **Paper-audit citation reconciliation (Phase 1, PRs #112/#113)** — verified the
  FS19 (Hypergraph Categories) anchors against the cached paper and fixed drifted
  citations: `Thm 1.2 / Thm 4.13` isomorphism-vs-equivalence phrasing in README /
  rustdoc, `FS19-AUDIT.md` internal count drift, Lemma 4.3 "io" (cross-label)
  qualifier, and `RelabelingFunctor` re-cited as the single-map component of
  Prop 2.1 / Cor 3.13. `operadic.rs` grounded in its `1305.0297` anchor and FS18
  (`1803.05316`) declared a secondary core anchor. `tests/spider_theorem.rs`
  upgraded from shape-only to full structural-equality assertions.

### Added

- **`scripts/check_audit_counts.py` CI guard (#111)** — checks the hand-maintained
  audit-doc tallies (summary arithmetic, headline percentages, per-section emoji
  counts, `(N tests)` citations) for self-consistency; wired into CI for
  `FS19-AUDIT.md` (Phase 1), then extended to `FS18-AUDIT.md` (Phase 2) and
  `BV25-AUDIT.md` (Phase 3).
- `CatgraphError::RecursionLimit { depth, limit }` — shared term-interpreter
  recursion-guard error, so `catgraph-syntax` interpreters whose error type is
  fixed to `CatgraphError` (e.g. a `CompleteFunctor`) report the same shape as
  its `SyntaxError::RecursionLimit`
  ([#99](https://github.com/sustia-llc/catgraph/issues/99)).
- `cospan_canon` — `CospanCanon<Λ>` and `Cospan::canonical_form`, a decidable
  (hashable, `Eq`) invariant for parallel cospans up to apex isomorphism.
  Records each apex vertex's `(label, sorted dom preimage, sorted cod preimage)`
  as a sorted multiset, so **scalars** (apex-only bubbles) are counted rather
  than collapsed — the *special* (not extra-special) semantics. Enables the
  complete Cospan-valued decision functor in `catgraph-syntax`
  ([#80](https://github.com/sustia-llc/catgraph/issues/80), F&S 2019 Prop 3.8).

> **Reconciliation note
> ([#158](https://github.com/sustia-llc/catgraph/issues/158)).** Workspace tags
> `v0.1.1`, `v0.2.0`, `v0.2.1`, and `v0.3.0` (2026-07-02 → 2026-07-11) were cut
> without per-crate sections here; this crate's changes across them are recorded
> only in git history (`git log v0.1.0..v0.3.0 -- catgraph/`) and the
> workspace-level release record. Backfill was deferred out of the v0.4.0
> release (owner, 2026-07-25) and resolved as this note (#158, option 2).
> Separately, `v0.5.0` deliberately rolled no section here — the crate had no
> changes at that tag.

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
- The §4 equivalence `Hyp_OF(Λ) ≅ Lax(Cospan_Λ, Set)` — Theorem 1.2 in its per-Λ form
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

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.11.0...HEAD
[workspace-v0.10.0]: https://github.com/sustia-llc/catgraph/compare/v0.9.0...v0.10.0
[workspace-v0.9.0]: https://github.com/sustia-llc/catgraph/compare/v0.8.0...v0.9.0
[workspace-v0.6.0]: https://github.com/sustia-llc/catgraph/compare/v0.5.0...v0.6.0
[workspace-v0.4.0]: https://github.com/sustia-llc/catgraph/compare/v0.1.0...v0.4.0
[workspace-v0.1.0]: https://github.com/sustia-llc/catgraph/releases/tag/v0.1.0
