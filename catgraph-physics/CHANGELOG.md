# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/spec/v2.0.0.html).

> Pre-reboot version links (`catgraph-physics-v0.x`) point at the private
> predecessor repo `tsondru/catgraph` and do not resolve publicly.

## [Unreleased]

## [workspace-v0.18.0] - 2026-09-05

### Added

- `tests/canonical.rs`: causal invariance on the `A→BB` and `collapse`
  fixtures with `CausalGraph::compare` against a brute-force permutation
  search; `wasserstein_1` against exhaustive optima and as identity / symmetry
  proptests; the Petersen edge curvature; the `to_petgraph` parallel-edge and
  dangling-edge contracts
  ([#417](https://github.com/sustia-llc/catgraph/pull/417)).

### Changed

- README gains §Canonical test; `docs/ANCHORS.md` points at
  `tests/canonical.rs` ([#418](https://github.com/sustia-llc/catgraph/pull/418)).

### Removed

- `tests/hypergraph_rewriting.rs`, `tests/wasserstein_exact.rs` and
  `tests/ollivier_ricci_exact_transport.rs`, subsumed by `tests/canonical.rs`
  ([#417](https://github.com/sustia-llc/catgraph/pull/417)).

## [workspace-v0.17.0] - 2026-09-03

### Changed — BREAKING

- `hypergraph::causal_graph`: `EdgeId`, `EventId`, `CausalEvent`,
  `CausalGraph` (one vertex per update event, an edge when an event consumes
  an instance another produced) and `CausalGraph::compare` reporting
  `CausalComparison::{Isomorphic, NotIsomorphic, Undecided}`;
  `HypergraphEvolution` mints an `EdgeId` per edge and exposes
  `edge_identities`, `event`, `causal_graph` and `causal_graph_between`.
  Holonomy compares the causal graphs two branches induce from their common
  ancestor: `WilsonLoop::path` runs ancestor → tip → ancestor,
  `WilsonLoop::holonomy` is `1.0` or `0.0`, and
  `CausalInvarianceResult::is_invariant` keys on no threshold.
  `RewriteRule::apply_effect` returns the removed and appended host-edge
  slots and the minted vertex IDs
  ([#325](https://github.com/sustia-llc/catgraph/issues/325)).
- `HypergraphLattice`: `find_wilson_loops` honours `max_length` and
  enumerates every coordinate plane for `D >= 2`; `wilson_loop`,
  `is_causally_invariant`, `plaquette_action`,
  `is_globally_causally_invariant` and `average_holonomy` return `Option`
  (`None` on a missing link or no recorded loop); `set_state` and
  `record_transition` return `bool` and reject out-of-bounds sites and
  non-finite or non-positive holonomies
  ([#326](https://github.com/sustia-llc/catgraph/issues/326)).
- `RewriteSpan::new` is replaced by `try_new -> Result<Self, RewriteSpanError>`
  (kernel vertices mapped on both sides, images present, maps injective) and
  `to_span` returns `Result<Span<u32>, RewriteSpanError>` instead of dropping
  unmapped kernel vertices; `RewriteRule::num_variables` counts distinct
  variables over `L ∪ R`; `RewriteSpanError` and `SpanSide` are re-exported
  from `catgraph_physics::hypergraph`
  ([#327](https://github.com/sustia-llc/catgraph/issues/327)).

### Fixed

- `wasserstein_1` is exact: successive-shortest-path min-cost flow replaces
  the transportation simplex; an input whose couplings all cost
  `f64::INFINITY` returns `f64::INFINITY` where it returned `NaN`
  ([#387](https://github.com/sustia-llc/catgraph/issues/387)).
- `OllivierRicciCurvature::from_branchial` counts a pair listed more than
  once, in either orientation, as one undirected edge and drops self-loops
  ([#388](https://github.com/sustia-llc/catgraph/issues/388)).

### Added — tests

- Causal-invariance pins on cospan fixtures with exact merge groups,
  `find_merges` group contents, `composites_agree` on hand-built cospans,
  exact `Display` renderings of `EvolutionStatistics` and
  `CausalInvarianceResult`, and holonomy `0.0` for out-of-range and
  non-descendant branch endpoints
  ([#325](https://github.com/sustia-llc/catgraph/issues/325)).
- `tests/wasserstein_exact.rs`: seeded instances against contingency-table
  enumeration and the permutation minimum;
  `tests/ollivier_ricci_exact_transport.rs`: every Petersen edge at
  `κ = -1/3` and union-support agreement on the topology fixtures
  ([#387](https://github.com/sustia-llc/catgraph/issues/387)).

### Changed

- `gauge` and `ollivier_ricci` rustdoc drop "(uncached)"; `curvature` drops
  the manifold-embedding backend and `manifold-curvature` feature mentions
  ([#407](https://github.com/sustia-llc/catgraph/pull/407)).
- `multiway::ollivier_ricci::all_pairs_bfs` compiles on every feature lane and
  backs new unit pins: all-pairs distance summaries and differentials over the
  seven seeded topology fixtures (`multiway::test_topologies`), the rayon
  all-pairs sweep above its threshold, and a rayon-versus-sequential
  `multiway_betweenness` comparison
  ([#329](https://github.com/sustia-llc/catgraph/issues/329)).
- `multiway::branchial_analysis` rustdoc reduced to contract statements
  ([#330](https://github.com/sustia-llc/catgraph/issues/330)).
- This CHANGELOG rewritten to one line per change; rationale lives in the PRs.
- `tests/catgraph_bridge.rs` contiguity test runs `edge_split` for three steps
  and asserts adjacent cospans composable; the `len() >= 2` guard is removed
  ([#328](https://github.com/sustia-llc/catgraph/issues/328)).

## [workspace-v0.14.0] - 2026-08-16

### Added

- `MultiwayEvolutionGraph::to_petgraph` — directed export, nodes in
  `(step, branch_id)` order, dangling edges dropped
  ([#161](https://github.com/sustia-llc/catgraph/issues/161)).
- `multiway_betweenness(graph, normalized)` — Brandes, endpoints excluded;
  rayon from 50 nodes with `parallel` on, scores then not bit-reproducible (#161).
- `multiway_katz(graph, alpha, max_iter, tol)` — L2-normalised Katz;
  `Some(empty)` on an empty graph, `None` on non-convergence (#161).
  Eigenvector centrality is not shipped: a step-graded DAG's adjacency is
  nilpotent, ρ = 0.
- Seeded rustworkx-core topology fixtures for `BranchialGraph` tests and
  `benches/branchial_bench.rs` (`rustworkx` + `spectral` features); rustworkx-core
  is now also a dev-dependency ([#163](https://github.com/sustia-llc/catgraph/issues/163)).

### Changed

- `RewriteSpan::to_span` builds through `Span::new_unchecked`; label agreement
  is a documented, `debug_assert!`-only precondition
  ([#256](https://github.com/sustia-llc/catgraph/issues/256)).
- `OllivierRicciCurvature::from_branchial` takes all-pairs distances from
  `rustworkx_core::shortest_path::distance_matrix` via crate-internal
  `branchial_distance_matrix`; metric, `0.0` diagonal and `f64::INFINITY`
  sentinel unchanged; rayon from 300 nodes with `parallel` on; queue BFS kept
  under `--no-default-features` ([#162](https://github.com/sustia-llc/catgraph/issues/162)).
- Dependency comments repointed from #10 to `README.md` "Dependencies" (#161).

## [workspace-v0.9.0] - 2026-08-04

### Added

- `BranchialSpectrum::eigenvalue_zero_tolerance`
  ([#166](https://github.com/sustia-llc/catgraph/issues/166)).

### Changed

- `multiway::branchial_spectrum` (#166): eigenvalue zero test is relative,
  `100·ε·n·max(λ_max, 1)`; eigenvector reordering in place; k-means seeds by
  farthest-first, empty clusters reseeded, movement-tolerance stop;
  `spectral_gap` returns `0.0` on an empty spectrum. Normalised Laplacians
  deferred to [#223](https://github.com/sustia-llc/catgraph/issues/223).

## [workspace-v0.4.0] - 2026-07-25

### Added

- `docs/ANCHORS.md` provenance note
  ([#124](https://github.com/sustia-llc/catgraph/issues/124)).

### Changed

- `benches/wasserstein_bench.rs` uses `catgraph_testutil::Lcg::with_increment(42, 1)`;
  stream byte-identical ([#33](https://github.com/sustia-llc/catgraph/issues/33)).
- `nalgebra` behind default-on `spectral` feature; `--no-default-features`
  drops `BranchialSpectrum` ([#43](https://github.com/sustia-llc/catgraph/issues/43)).
- Unanchored attributions hedged in `branchial_spectrum.rs`, `gauge.rs`,
  `rewrite_rule.rs`, `ollivier_ricci.rs` (#124).
- Paper-audit phase 4: `evolution_graph.rs` header corrected against Gorard
  2301.04690 (irreducibility gloss, Z′ line, BV25 Prop 3.10 index restriction).

> Workspace tags `v0.1.1`–`v0.3.0` have no per-crate sections here
> ([#158](https://github.com/sustia-llc/catgraph/issues/158)); `v0.5.0` had no
> physics changes.

## [workspace-v0.1.0] - 2026-07-01

First monorepo release.

### Fixed

- Rustdoc links in `multiway/evolution_graph.rs` and `multiway/branchial_spectrum.rs`.

## [0.3.0] - 2026-04-28

### Added

- `interval` — `DiscreteInterval`, `ParallelIntervals` with `total_complexity`
  / `max_complexity` (ported from `irreducible`).
- `temporal_cospan_chain` — `TemporalComplex`, `to_cospan_chain`,
  `compose_cospan_chain`; `StokesError` → `TemporalComplexError`.
- `trace` — `StepTrace`, `TraceAnalysis`, `RepeatDetection`, `analyze_trace`,
  `detect_repeats`, `is_irreducible`.

## [0.2.2] - 2026-04-19

### Added

- Default-on `parallel` feature (pass-through to `catgraph/parallel`);
  `examples/wasi_smoke_physics.rs`.

### Changed

- `catgraph` dep `default-features = false`.

## [0.2.1] - 2026-04-17

### Changed

- `multiway/evolution_graph.rs` module header extended (functor framing). Doc-only.

## [0.2.0] - 2026-04-13

### Added

- `multiway/branchial_spectrum.rs` — `BranchialSpectrum` (λ₂, spectral gap,
  Fiedler vector, components, spectral clustering).
- `multiway/branchial_analysis.rs` — `BranchialGraph::to_petgraph`,
  `branchial_coloring`, `branchial_core_numbers`, `branchial_articulation_points`.
- `benches/wasserstein_bench.rs`.

### Dependencies

- `nalgebra 0.34`, `nalgebra-sparse 0.11`, `petgraph 0.8`, `rustworkx-core 0.17`; dev `criterion 0.8`.

## [0.1.0] - 2026-04-12

### Added

- Initial release: `hypergraph/` (`Hypergraph`, `RewriteRule`,
  `HypergraphEvolution`, `HypergraphLattice`, `rewrite_span`,
  `evolution_cospan`, `multiway_cospan`); `multiway/`
  (`MultiwayEvolutionGraph`, `BranchialGraph`, `OllivierRicciCurvature`,
  `wasserstein_1`); `record_transition(from, to, holonomy)`;
  `ConfluenceDiamond`, `confluence_diamonds`, `parallel_independent_events`,
  `events_commute`.

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.18.0...HEAD
[workspace-v0.18.0]: https://github.com/sustia-llc/catgraph/compare/v0.17.0...v0.18.0
[workspace-v0.17.0]: https://github.com/sustia-llc/catgraph/compare/v0.16.0...v0.17.0
[workspace-v0.14.0]: https://github.com/sustia-llc/catgraph/compare/v0.13.0...v0.14.0
[workspace-v0.9.0]: https://github.com/sustia-llc/catgraph/compare/v0.8.0...v0.9.0
[workspace-v0.4.0]: https://github.com/sustia-llc/catgraph/compare/v0.1.0...v0.4.0
[workspace-v0.1.0]: https://github.com/sustia-llc/catgraph/releases/tag/v0.1.0
[0.2.2]: https://github.com/tsondru/catgraph/releases/tag/catgraph-physics-v0.2.2
[0.2.1]: https://github.com/tsondru/catgraph/releases/tag/catgraph-physics-v0.2.1
[0.2.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-physics-v0.2.0
[0.1.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-physics-v0.1.0
