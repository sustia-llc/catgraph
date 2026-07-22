# Changelog

All notable changes to this crate are documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Lineage note:** pre-reboot version links below (`catgraph-physics-v0.x`
> tags) point at the private predecessor repo `tsondru/catgraph` and will not
> resolve publicly; they are kept as an honest record of the crate's history.

## [Unreleased]

### Added

- **`docs/ANCHORS.md` provenance note**
  ([#124](https://github.com/sustia-llc/catgraph/issues/124)): the crate is
  **inspiration-anchored**, not theorem-anchored — the note maps each
  attribution site to its source and cache status: [Gor23]
  (arXiv:2301.04690, cached, ✅ verified in Phase 4) vs the uncached (†)
  attributions [Gor20a]/[Gor20b]/[Oll09]/[Vil03]/[EPS73]. README + root
  CLAUDE.md paper-anchor list link it.

### Changed

- **`nalgebra` gated behind a default-on `spectral` feature**
  ([#43](https://github.com/sustia-llc/catgraph/issues/43)): the dense-Laplacian
  eigendecomposition (`multiway::branchial_spectrum::BranchialSpectrum`) and its
  `nalgebra` dependency now sit behind the default-on `spectral` feature, a
  slim-build companion to the
  [#10](https://github.com/sustia-llc/catgraph/issues/10) `rustworkx` gate. Default builds are
  unchanged. **Behavioral change for slim consumers:** `--no-default-features`
  builds no longer include `BranchialSpectrum` unless they enable `spectral`
  (e.g. `--no-default-features --features spectral`); opting out drops the whole
  nalgebra stack for slim / WASM builds.

- **Unanchored attributions reworded / hedged**
  ([#124](https://github.com/sustia-llc/catgraph/issues/124)):
  `branchial_spectrum.rs` no longer credits the λ₂/Fiedler reducibility
  proxy to "Gorard's framework" — it is a catgraph extrapolation over the
  branchial substrate (the cached paper has zero spectral/Laplacian
  content); `gauge.rs` gains a Provenance section (inspired by [Gor20a]'s
  causal-invariance-as-gauge-covariance; Wilson-loop/plaquette vocabulary =
  standard lattice gauge theory; "causal invariance ⟺ flat gauge field" is
  a catgraph interpretive gloss); `rewrite_rule.rs` credits DPO to
  Ehrig–Pfender–Schneider 1973 [EPS73]; the `ollivier_ricci.rs`
  `branchial_complexity` "max |scalar| ~1" comment is corrected to the
  definitional `κ ≤ 1` plus an explicit normalization-convention hedge
  (negative Ollivier curvature is not bounded below by −1).

- **Paper-audit citation reconciliation (Phase 4)** — verified every Gorard
  2301.04690 anchor in `src/**` against the cached paper and fixed the drifted
  claims in the `evolution_graph.rs` header: the irreducibility gloss was
  *inverted* ("irreducibility = lack of functorial exactness"; the paper says a
  multicomputationally irreducible computation is one whose map **is** a pure
  symmetric monoidal functor — it also contradicted the same header's Z′ line);
  the Z′ line now says *pure* symmetric monoidal functor; the Mamba
  state-space-model bullet is labeled analogy-only (cache-unverifiable, not a
  citation anchor); the stale "planned `catgraph-magnitude` sibling crate
  (Phase 6)" note updated (the crate shipped); the Bradley–Vigneaux magnitude
  formula gained its missing index restriction `x ∈ ob(M) \ T(⊥)` (BV25
  Prop 3.10). Adversarially re-checked as already correct: the paper-title
  quotes, the trace.rs "cannot be shortcut" gloss, and the branchial
  common-ancestor edge definition. The `branchial_spectrum.rs` λ₂/Fiedler
  attribution and the uncited `gauge.rs`/`rewrite_rule.rs`/`ollivier_ricci.rs`
  physics claims are substantive and tracked as a GitHub issue (crate-local
  ANCHORS provenance note).

## [workspace-v0.1.0] - 2026-07-01

First monorepo release: workspace-wide tag `v0.1.0` (supersedes the pre-reboot
crate-scoped version lineage below). The coalition semantic-layer handoff to
downstream koalisi.

### Fixed

- **Rustdoc warnings:**
  - `multiway/evolution_graph.rs:22` — broken link `[crate::hypergraph::evolution_cospan::to_cospan_chain]` (free-fn path that doesn't exist) repointed to `[HypergraphEvolution::to_cospan_chain](crate::hypergraph::HypergraphEvolution::to_cospan_chain)` — the method actually lives on the re-exported type.
  - `multiway/branchial_spectrum.rs:128` — public-doc link to private const `EIGENVALUE_ZERO_THRESHOLD` replaced with backtick formatting + the literal value (`1e-10`). Const stays private (internal fudge factor).
  No source changes; doc-only.

## [0.3.0] - 2026-04-28

Port of timestep machinery from `irreducible` so downstream sibling consumers
share a single source of truth (no orphaned drift, no cross-bar dep).

### Added

- `interval` module — `DiscreteInterval` (composable `[start, end] ∩ ℕ`
  intervals with mathematical and left-to-right composition) plus
  `ParallelIntervals` (tensor-product structure with `total_complexity` /
  `max_complexity` distinguishing summed from observed cost). Ported verbatim
  from `irreducible/src/interval.rs`; framing neutralized (cobordism category
  ℬ → discrete-time category) so the module is consumable outside the
  irreducibility framework.
- `temporal_cospan_chain` module — `TemporalComplex` builds a 1D simplicial
  complex from interval sequences, with conservation verification (contiguity
  + monotonicity), 1-form integration, and the bridge into composable cospan
  chains via `to_cospan_chain` / `compose_cospan_chain`. Ported from
  `irreducible/src/temporal_cospan_chain.rs`; error type renamed `StokesError`
  → `TemporalComplexError` (drops the historic stokes lineage that does not
  exist in catgraph-physics).
- `trace` module — `StepTrace` trait for execution histories that evolve in
  discrete steps, plus `analyze_trace` / `TraceAnalysis` / `RepeatDetection`
  / `detect_repeats`. Free function `is_irreducible(&impl StepTrace) -> bool`
  ties the Wolfram-irreducibility judgment (Gorard 2023) to the structural
  trace, so downstream consumers can use the trait neutrally without buying
  into the framing.

### Test count

- 137 → 172 (+35 across the three new modules: 13 + 10 + 12).

### Cross-repo follow-up (NOT in this release)

- `irreducible` v0.6.3 (separate timeline): convert
  `irreducible/src/{interval,temporal_cospan_chain,trace}.rs` to thin
  re-export shims pointing at `catgraph_physics::{interval,
  temporal_cospan_chain, trace}` to avoid three diverging copies.
- `irreducible` v0.7.0 (later): drop the deprecated local modules entirely.

### Performance candidates (bench-driven, no version target)

Deferred from prior rayon ride-along.

- `par_array_windows::<2>()` in `multiway::branchial::branchial_parallel_step_pairs` — per-pair work is cheap; bench on long foliations
- `par_array_windows::<2>()` in `hypergraph::evolution_cospan::to_cospan_chain` — per-pair work does a union-find pushout; benchable on long deterministic paths
- `walk_tree_prefix` / `walk_tree_postfix` in `multiway::evolution_graph` — compare against current recursive BFS / confluence-diamond enumeration
- rayon Producer/Consumer plumbing — reference design if `MultiwayEvolutionGraph` / `BranchialGraph` ever expose public parallel-iterator APIs

## [0.2.2] - 2026-04-19

WASM + edge-device support. Pass-through `parallel` feature (this crate has
no direct rayon call sites yet; the feature wires the upstream
`catgraph/parallel` toggle through so downstream builds with
`--no-default-features` see a single-threaded catgraph transitively).

### Added

- `[features] default = ["parallel"]` — `parallel = ["catgraph/parallel"]`.
- `examples/wasi_smoke_physics.rs` — small hypergraph construction smoke
  example.

### Changed

- `catgraph` dep now `default-features = false` so the `parallel` toggle
  propagates cleanly through this crate.

## [0.2.1] - 2026-04-17

### Changed

- Rustdoc framing pass: `src/multiway/evolution_graph.rs` module header extended with `## Time-step discretization as a functor F: C → D` and `## Per-step foliation selection` subsections. References Gorard 2023, Mamba state-space models, and BV 2025. No API changes.

## [0.2.0] - 2026-04-13

Branchial analysis toolkit — additive capabilities for `BranchialGraph`.

### Added

- `src/multiway/branchial_spectrum.rs`: `BranchialSpectrum` — graph Laplacian eigendecomposition via `SymmetricEigen`. Exposes algebraic connectivity (λ₂), spectral gap, Fiedler vector, connected-component count, spectral clustering (k-means on leading eigenvectors).
- `src/multiway/branchial_analysis.rs`: `to_petgraph()` conversion on `BranchialGraph`, plus `branchial_coloring` (greedy via rustworkx-core), `branchial_core_numbers` (k-core), `branchial_articulation_points`.
- Wasserstein DMatrix benchmark (`benches/wasserstein_bench.rs`) comparing `Vec<Vec<f64>>` vs `DMatrix<f64>` at sizes 10/50/100/200. Outcome: no performance delta — no refactor needed.

### Dependencies

- New: `nalgebra 0.34`, `nalgebra-sparse 0.11`, `petgraph 0.8`, `rustworkx-core 0.17`.
- New dev: `criterion 0.8`.

## [0.1.0] - 2026-04-12

### Added

- Initial release. Wolfram-physics extensions extracted from `catgraph` core:
  - `hypergraph/` — `Hypergraph`, `RewriteRule`, `HypergraphEvolution`, `HypergraphLattice` (gauge), categorical bridges (`rewrite_span.rs`, `evolution_cospan.rs`, `multiway_cospan.rs`).
  - `multiway/` — `MultiwayEvolutionGraph`, `BranchialGraph`, `OllivierRicciCurvature`, `wasserstein_1`.
- Gauge Wilson-loop fix: `record_transition(from, to, holonomy)` for explicit inter-site gauge links (was erroneously recording self-loops).
- Multiway APIs exposed for downstream consumers in `irreducible`: `ConfluenceDiamond`, `confluence_diamonds()`, `parallel_independent_events(node_id)`, `events_commute(a, b)`.

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.1.0...HEAD
[workspace-v0.1.0]: https://github.com/sustia-llc/catgraph/releases/tag/v0.1.0
[0.2.2]: https://github.com/tsondru/catgraph/releases/tag/catgraph-physics-v0.2.2
[0.2.1]: https://github.com/tsondru/catgraph/releases/tag/catgraph-physics-v0.2.1
[0.2.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-physics-v0.2.0
[0.1.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-physics-v0.1.0
