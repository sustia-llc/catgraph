# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [workspace-v0.25.0] - 2026-09-23

### Changed — BREAKING

- `TemporalComplex` keeps the input intervals in order: `step_counts`,
  `num_intervals` and `to_cospan_chain` are per input interval;
  `time_points` / `num_time_steps` are the endpoints in input order with only
  consecutive equal points merged; `is_conserved` and `is_contiguous` are
  false on a gap, an overlap or an out-of-order interval, `is_monotonic` on an
  out-of-order interval; `total_complexity` is `last.end − first.start`
  ([#476](https://github.com/sustia-llc/catgraph/pull/476)).

## [workspace-v0.24.0] - 2026-09-22

### Changed — BREAKING

- `MultiwayEvolutionGraph` node fingerprints, `Hypergraph::fingerprint` and
  `Hyperedge::fingerprint` are `canonical_fingerprint` values; the multiway
  state bound `S: Hash` is `S: CanonicalEncode` on the fingerprinting impl
  block, `run_multiway_bfs`, and the `branchial` / `ollivier_ricci` functions
  that take a `MultiwayEvolutionGraph`
  ([#469](https://github.com/sustia-llc/catgraph/pull/469)).
- `trace::is_irreducible` → `trace::is_contiguous_without_repeats`,
  `TraceAnalysis::is_irreducible` → `TraceAnalysis::is_contiguous_without_repeats`,
  `Display` label `Contiguous without repeats:`; no alias
  ([#472](https://github.com/sustia-llc/catgraph/pull/472)).

### Added

- `serde` feature: derives on `BranchId`, `MultiwayNodeId`,
  `MultiwayEdgeKind`, `MultiwayEdge<T>`, `MultiwayNode<S>`, and serde on
  `MultiwayEvolutionGraph` through `MultiwayParts` (`to_parts` /
  `from_parts`, rejecting malformed parts as `MultiwayPartsError`);
  `MultiwayEdge` / `MultiwayNode` derive `PartialEq, Eq`; new dependency
  `thiserror` ([#471](https://github.com/sustia-llc/catgraph/pull/471)).
- `HypergraphEvolution::rules()`
  ([#472](https://github.com/sustia-llc/catgraph/pull/472)).

### Fixed

- `add_sequential_step` from a node whose `(branch, step + 1)` id is taken
  allocates a fresh branch with a `Fork` edge instead of overwriting the node
  ([#471](https://github.com/sustia-llc/catgraph/pull/471)).
- `RewriteRule::find_matches` returns every match of a multi-edge left-hand
  side (was: at most one per first-edge candidate); `matches[0]` is unchanged
  ([#474](https://github.com/sustia-llc/catgraph/pull/474)).

### Changed

- Sections before `workspace-v0.17.0` moved out of this file; the file before
  the move is at tag `v0.23.0`
  ([#462](https://github.com/sustia-llc/catgraph/pull/462)).

## [workspace-v0.22.0] - 2026-09-08

### Added

- `LinkVariable` trait with impls for `DMatrix<f64>`, `Rotation3<f64>`,
  `UnitQuaternion<f64>` and `Isometry3<f64>`;
  `HypergraphLattice<const D: usize, L: LinkVariable = DMatrix<f64>>`; the
  Wilson value is the defining-representation trace over its dimension, and
  typed admissibility gates on the carrier invariant within
  `TYPED_LINK_TOL = 1e-9`
  ([#453](https://github.com/sustia-llc/catgraph/pull/453)).

## [workspace-v0.21.0] - 2026-09-08

### Changed

- `wasserstein_1` measures mass against relative thresholds — `REL_MASS_TOL`,
  `REL_CAP_FLOOR`, `REL_SHORTFALL` as fractions of `max(Σμ, Σν)` — returns
  `0.0` only on exactly-zero totals, and rejects a non-finite total
  ([#449](https://github.com/sustia-llc/catgraph/pull/449)).

## [workspace-v0.20.0] - 2026-09-08

### Added — tests

- `tests/wasserstein_props.rs`: metric axioms, one-homogeneity and the
  sorted-CDF oracle on line instances; the absolute-constant bands pinned
  ([#445](https://github.com/sustia-llc/catgraph/pull/445)).

## [workspace-v0.19.1] - 2026-09-07

### Changed

- `Hypergraph::compare -> CausalComparison` (new) and `is_isomorphic_to` are
  exact up to `CausalGraph::MAX_SEARCH_STEPS`: a typed incidence digraph goes
  through the colour refinement and backtracking search now shared with
  `CausalGraph::compare` in a private `hypergraph/isomorphism.rs`, nodes
  placed in an arc-connected order. `Hypergraph::fingerprint` is invariant
  under vertex relabelling and edge reordering; `find_merges` partitions each
  fingerprint bucket into isomorphism classes and `find_wilson_loops` closes
  loops within a class, so relabelled isomorphic states merge: the `collapse`
  fixture reports 3 groups and 33 loops, and `{{0,1,2}}` under
  `wolfram_a_to_bb` + `edge_split` to depth 3 reports 16 loops and is not
  causally invariant ([#437](https://github.com/sustia-llc/catgraph/pull/437)).
- `CausalComparison` covers both comparisons; README names the
  `Hypergraph::compare` arm
  ([#437](https://github.com/sustia-llc/catgraph/pull/437)).

### Added — tests

- `compare_settles_the_pairs_the_prefilter_leaves_open`,
  `equal_fingerprints_do_not_imply_isomorphism`,
  `fingerprint_is_invariant_and_separating`,
  `compare_decides_vertex_transitive_inputs`,
  `merges_and_loops_range_over_isomorphism_classes_not_buckets`,
  `eight_cycle_siblings_open_one_group_and_one_loop`,
  `multiway_fixtures_merge_their_isomorphic_states`; the canonical
  `hypergraph_compare_agrees_with_brute_force_permutation_isomorphism` arm
  over a corpus of at most 8 vertices
  ([#437](https://github.com/sustia-llc/catgraph/pull/437)).

## [workspace-v0.19.0] - 2026-09-07

### Changed — BREAKING

- `HypergraphLattice<D>` links are `DMatrix<f64>` of side `link_dim`, behind a
  default-on `gauge` feature (`dep:nalgebra`): `new(dimensions, group, rules,
  link_dim)` (panics at `link_dim` 0), `record_transition(from, to,
  DMatrix<f64>) -> bool`; `link`, `link_dim`, `loop_holonomy` (`U_k · … ·
  U_1`), `is_flat(path, eps)` and `gauge_transform` are new; `wilson_loop` is
  `tr(H) / link_dim`; `hypergraph::gauge`, `tests/gauge_theory.rs`, the
  canonical gauge arm and `examples/gauge.rs` are feature-gated
  ([#426](https://github.com/sustia-llc/catgraph/pull/426)).

### Changed

- `docs/ANCHORS.md` gains `[Wil74]` and the `gauge.rs` provenance row; README
  gains the `gauge` feature row
  ([#426](https://github.com/sustia-llc/catgraph/pull/426)).

### Added — tests

- `tests/gauge_theory.rs`: `is_flat` at a second `eps` and at the boundary,
  `gauge_transform` endpoint keys and an inadmissible value, an overflowing
  link product, a repeated-site holonomy
  ([#429](https://github.com/sustia-llc/catgraph/pull/429)).

### Added — tooling

- `ci.yml`: a physics `--no-default-features --features gauge` clippy lane
  ([#429](https://github.com/sustia-llc/catgraph/pull/429)).

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

---

> 📄 **Sections before `workspace-v0.17.0` are archived.** Every section from
> `[workspace-v0.14.0]` down was moved out of this file verbatim on 2026-09-21
> and is held outside this repository. The file as it stood before the move
> is at tag
> [`v0.23.0`](https://github.com/sustia-llc/catgraph/blob/v0.23.0/catgraph-physics/CHANGELOG.md).

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.25.0...HEAD
[workspace-v0.25.0]: https://github.com/sustia-llc/catgraph/compare/v0.24.0...v0.25.0
[workspace-v0.24.0]: https://github.com/sustia-llc/catgraph/compare/v0.23.0...v0.24.0
[workspace-v0.22.0]: https://github.com/sustia-llc/catgraph/compare/v0.21.0...v0.22.0
[workspace-v0.21.0]: https://github.com/sustia-llc/catgraph/compare/v0.20.0...v0.21.0
[workspace-v0.20.0]: https://github.com/sustia-llc/catgraph/compare/v0.19.1...v0.20.0
[workspace-v0.19.1]: https://github.com/sustia-llc/catgraph/compare/v0.19.0...v0.19.1
[workspace-v0.19.0]: https://github.com/sustia-llc/catgraph/compare/v0.18.0...v0.19.0
[workspace-v0.18.0]: https://github.com/sustia-llc/catgraph/compare/v0.17.0...v0.18.0
[workspace-v0.17.0]: https://github.com/sustia-llc/catgraph/compare/v0.16.0...v0.17.0
