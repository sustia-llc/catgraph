# Changelog

All notable changes to this crate are documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Lineage note:** pre-reboot version links below (`catgraph-physics-v0.x`
> tags) point at the private predecessor repo `tsondru/catgraph` and will not
> resolve publicly; they are kept as an honest record of the crate's history.

## [Unreleased]

## [workspace-v0.14.0] - 2026-08-16

### Changed

- **`RewriteSpan::to_span` documents its label-agreement precondition**, and
  builds its span with the new `Span::new_unchecked`
  ([#256](https://github.com/sustia-llc/catgraph/issues/256)). **Its observable
  release behaviour is unchanged.** `RewriteSpan`'s fields are public, so
  `left_map` and `right_map` may send a kernel vertex to different vertex IDs —
  the span's labels *are* vertex IDs, so that is a genuine label-agreement
  violation the type permits. It stays `debug_assert!`-only rather than becoming
  a release check, which is what `new_unchecked` expresses. Every `RewriteSpan`
  this crate builds satisfies the invariant (`RewriteRule::to_rewrite_span` uses
  identity morphisms). Callers wanting the check should validate through
  `Span::new` themselves.

  An earlier draft of this entry claimed the method "no longer index-panics in a
  release build". That was wrong and the code review caught it: both pair
  components come from `left_index` / `right_index`, whose value ranges are
  exactly `0..left_verts.len()` and `0..right_verts.len()`, and the label
  vectors have those same lengths — so the bounds were never violated here. The
  release-mode indexing bug fixed in core's `Span::assert_valid` was
  unreachable through this path; only the label check, which is debug-only,
  could ever have fired.

- **`OllivierRicciCurvature::from_branchial` gets its all-pairs distances from
  rustworkx-core** ([#162](https://github.com/sustia-llc/catgraph/issues/162),
  `rustworkx` feature, **no new dependencies**). The hand-rolled `all_pairs_bfs`
  queue sweep is replaced by `rustworkx_core::shortest_path::distance_matrix`
  over the existing `BranchialGraph::to_petgraph` shim, wrapped in a new
  crate-internal `branchial_distance_matrix`. Behaviour is unchanged: the
  unweighted hop metric, the `0.0` diagonal, and `f64::INFINITY` for an
  unreachable pair are all preserved — `f64::INFINITY` is passed as rustworkx's
  `null_value`, so the sentinel `from_branchial` already skipped on keeps its
  meaning. Branchial graphs are dense by construction (nodes are joined when
  they share *any* ancestor), which is the regime rustworkx's `FixedBitSet`
  frontier is built for.

  - **The rayon threshold is gated, exactly as #161's was.** `distance_matrix`
    takes a `parallel_threshold` and branches straight onto `into_par_iter()`
    once the node count reaches it, so it is the crate's *second* path into
    rayon. It gets its own `APSP_PARALLEL_THRESHOLD`: 300 (rustworkx's own
    recommended default) with the `parallel` feature on, `usize::MAX` with it
    off. Ungated, a `--no-default-features --features rustworkx` build — the
    single-threaded / WASI case — would spawn rayon work from inside a
    curvature computation, and on a no-threads wasm target rayon's global pool
    init fails at *runtime*, only for graphs past the threshold. (Mechanism
    differs from #161's: `distance_matrix` branches on a plain `if`, not
    `rayon_cond::CondIterator`. The hazard is identical.)
  - **Unlike betweenness, this one *is* bit-reproducible on the parallel
    path.** `distance_matrix` fills disjoint rows — each per-source BFS writes
    only into its own row view, and every value written is an integer-valued
    hop counter rather than an accumulated sum. There is no shared f64 buffer
    whose summation order could vary with scheduling, which is precisely what
    makes the Brandes accumulation non-reproducible.
  - **No `ndarray` in the signature or in `Cargo.toml`.** `distance_matrix`
    returns an `ndarray` 2-D array; it is copied to `Vec<Vec<f64>>` at the
    boundary. `ndarray` is already present transitively beneath rustworkx-core,
    but *declaring* it would version-lock this crate to rustworkx-core's choice
    and break the slim/WASM story that one feature gate drops the whole
    rustworkx-core → petgraph + ndarray chain. The copy is `O(n²)` against an
    `O(n³/64)` sweep, so it does not change the shape of the cost.
  - **`--no-default-features` keeps working.** `ollivier_ricci` is not itself
    gated, so the old queue BFS is retained as the slim-build fallback under
    `#[cfg(not(feature = "rustworkx"))]`. The two paths agree on the metric,
    and the module's curvature tests — now pinned to *exact* values rather
    than signs — run on both.
  - **Adjacency-list construction stays.** #162 suggested it could be deleted
    along with the BFS; it cannot. `adj` also carries the uniform neighbour
    distributions `μ_x` that the Wasserstein step integrates against.
  - Not done, and deliberately out of scope: memoizing `to_petgraph` across an
    analysis pass. It is a real observation in #162 but a separate design
    change, and this pass touches only the distance sweep.

### Added

- **Multiway DAG centrality**
  ([#161](https://github.com/sustia-llc/catgraph/issues/161), `rustworkx`
  feature, no new dependencies) — in `multiway::branchial_analysis`:
  - **`MultiwayEvolutionGraph::to_petgraph`** — the directed sibling of
    `BranchialGraph::to_petgraph`, returning `(DiGraph<MultiwayNodeId, ()>,
    Vec<MultiwayNodeId>)` where `order[i]` is the node at `NodeIndex::new(i)`.
    Node order is **canonical `(step, branch_id)` ascending**, not `HashMap`
    order: the evolution graph stores nodes in a map, so an unsorted export
    would give a different index assignment on every run and move every
    index-keyed score with it.
  - **`multiway_betweenness(graph, normalized)`** — branching-junction load
    via Brandes, endpoints excluded, parallelised through rustworkx-core's
    `CondIterator` at or above 50 nodes **when the `parallel` feature is on**.
    This is the crate's first direct rayon call site, so the threshold is
    `usize::MAX` under `--no-default-features`: an ungated `CondIterator`
    would spawn rayon work on a single-threaded or WASI build and fail at
    *runtime* on a no-threads wasm target, and only for graphs large enough
    to cross the threshold. ⚠ The parallel path is **not bit-reproducible** —
    rustworkx accumulates per-source partials from rayon workers, so f64
    summation order varies run to run; the node *ordering* is canonical
    either way. Build without `parallel` for pinnable scores.
  - **`multiway_katz(graph, alpha, max_iter, tol)`** — α-damped inbound path
    count, L2-normalized. Returns `Option`, so rustworkx-core's
    non-convergence / zero-norm case is surfaced rather than collapsed to
    zeros or an empty map — except for an **empty** graph, which scores
    `Some(empty)` to match `multiway_betweenness` (rustworkx's convergence
    test is `0.0 < 0.0` there, so it would otherwise report a
    non-convergence that never happened).
  - ⚠ **The DAG grading that makes Katz terminate is a convention, not an
    invariant.** `add_merge_edge` validates neither endpoint nor step
    ordering, so a same-step or backward edge is constructible — and its own
    documented workflow permits one, since `add_fork` leaves every step-`t+1`
    sibling active. Such an edge makes the adjacency non-nilpotent; if
    `ρ(A) ≥ 1/alpha` the iteration diverges and returns `None`. On a graph
    carrying merge edges, read `None` as "α too large" at least as readily as
    "needs more iterations". Relatedly, `to_petgraph` **drops** edges whose
    endpoints are not registered nodes, so on such a graph the centralities
    are computed over a different topology than `edge_count()` reports; both
    behaviours are now documented at the API.
  - **Substitution: Katz, not eigenvector centrality.** The issue asked for
    betweenness *and eigenvector* centrality. **Eigenvector centrality is
    undefined on a multiway evolution graph** and is deliberately not
    shipped. The graph is a step-graded DAG, so its adjacency is nilpotent
    and its spectral radius is 0 — there is no dominant eigenvector for power
    iteration to find. rustworkx-core's `eigenvector_centrality` returns
    `Ok(None)` at its defaults; forced to terminate, it reports a *sink
    indicator*, scoring the root of a star fork at ≈ 0 — the exact branching
    junction the issue wanted measured. Katz's `+ β` term makes the iteration
    terminate on a nilpotent adjacency and floors a source at β instead. All
    three behaviours are pinned by
    `katz_floors_the_root_where_eigenvector_centrality_degenerates`. The
    rationale is also in `multiway_katz`'s rustdoc, where a future reader
    will actually meet it.
  - Betweenness is verified against a hand-computed diamond (fork + merge),
    both raw and normalized.

- **Widened test / bench topologies via seeded rustworkx-core generators**
  ([#163](https://github.com/sustia-llc/catgraph/issues/163)) —
  `rustworkx-core` is now also a **dev-dependency**; the published dependency
  tree is unchanged (it was already an optional lib dep behind the default-on
  `rustworkx` feature).
  - **The insertion point is `BranchialGraph`, not `MultiwayEvolutionGraph`.**
    The issue proposed generating topologies at the evolution-graph level, but
    that is not constructible: `MultiwayEvolutionGraph`'s fields are private
    and its only builders are `add_root` / `add_fork` /
    `add_sequential_step`, so every branchial cross-section it can produce is
    a Kₙ. `BranchialGraph`'s `nodes` / `edges` are public, so a generated
    `UnGraph` translates into one directly. The existing evolution-level
    `arb_branched_graph` strategy is unchanged; the generated topologies are
    siblings of it. The three pre-existing proptests are still served, because
    what they assert (λ₂ positivity, zero-eigenvalue multiplicity, proper
    coloring) are *branchial* properties.
  - Seven pinned fixtures in `tests/branchial_analysis.rs`: sparse and dense
    Erdős–Rényi, Barabási–Albert, 3-regular, path, Petersen, and a 4×6 grid —
    covering disconnected, connected-but-incomplete, regular, and
    heavy-tailed-degree shapes that Kₙ never reaches. A non-property test
    (`generated_topologies_escape_the_complete_graph_regime`) pins that the
    set is actually wider, so the new proptests cannot silently degenerate
    back to complete graphs.
  - New `benches/branchial_bench.rs` (`required-features = ["rustworkx",
    "spectral"]`) with fixtures for `branchial_analysis` (coloring, k-core,
    articulation points at n = 100 and n = 1000) and `branchial_spectrum`
    (n = 100/200/300 — the dense `SymmetricEigen` is Θ(n³), so it stops short
    of 1000). Fixtures only; no performance claim is made.
  - **Determinism**: every generator seed is a pinned literal, and the
    generators supply *topology only*. Numeric fixtures stay on
    `catgraph-testutil`'s LCG, whose stream is byte-identity-pinned (#33);
    nothing LCG-derived changed.

### Changed

- **Dependency comments repointed off the closed issue #10**
  ([#161](https://github.com/sustia-llc/catgraph/issues/161)) — the workspace
  `Cargo.toml` `rustworkx-core` line and this crate's `rustworkx` /
  `spectral` feature comments cited #10 (the feature-gate work, long since
  closed) as if it were the standing rationale. They now point at
  `catgraph-physics/README.md`, "Dependencies", which is the live one.

## [workspace-v0.9.0] - 2026-08-04

### Added

- **`BranchialSpectrum::eigenvalue_zero_tolerance`**
  ([#166](https://github.com/sustia-llc/catgraph/issues/166)) — the
  spectrum-scaled tolerance now used by `connected_components` and
  `spectral_gap`, exposed so callers can see (and reproduce) the zero test.

### Changed

- **`multiway::branchial_spectrum` hygiene pass**
  ([#166](https://github.com/sustia-llc/catgraph/issues/166), `spectral`
  feature, no new dependencies):
  - **Degrees hoisted out of the Laplacian cell closure** — a readability
    change with **no complexity or work difference**. #166 reported the old
    inline `(0..n).filter(…).count()` as an accidental O(n³) construction; that
    is **incorrect**, and the correction is recorded on the issue. `from_fn`
    evaluates its closure once per cell and the scan sat behind `if i == j`, so
    it ran on the n diagonal cells only — Θ(n²), exactly what the hoisted loop
    costs. The old form was merely easy to misread as O(n³).
  - **Eigenvector reordering applies the sort permutation in place** (cycle
    walk over `swap_columns`) instead of rebuilding the whole n×n matrix cell
    by cell into a second allocation.
  - **Eigenvalue zero detection is relative, not absolute.** The fixed
    `EIGENVALUE_ZERO_THRESHOLD = 1e-10` is replaced by
    `100 · ε · n · max(λ_max, 1)`, mirroring the `O(ε · n · ‖L‖₂)`
    backward-error bound of a symmetric eigensolver. A fixed absolute
    threshold does not grow with the solver's own error, so large or
    weight-scaled graphs could misclassify zero eigenvalues and hence
    **miscount connected components**. The `max(λ_max, 1)` floor keeps an
    edgeless graph (every eigenvalue zero) counting every node as its own
    component.
  - **k-means seeding, empty clusters, and convergence.** Seeds now come from
    farthest-first traversal (the deterministic analogue of k-means++) rather
    than the first `k` pairwise-distinct points, which could place every
    centroid inside a single dense blob; clusters that go empty are reseeded
    onto the point farthest from its own centroid instead of being parked at a
    zero centroid — so all `k` clusters stay populated whenever the embedding
    has at least `k` distinct points (with coincident points, k-means cannot
    fill `k` clusters at all, and the rustdoc says so); and the sweep now stops
    on a centroid-movement tolerance as well as on the 100-iteration cap.
    Clustering remains fully deterministic — no RNG, all ties broken toward the
    lowest index.
  - `fiedler_vector`'s copy is now documented, with `spectrum.eigenvectors
    .column(1)` named as the zero-copy alternative.
  - `spectral_gap` gained an explicit empty-spectrum guard. The relative
    tolerance is zero for an empty spectrum (there is no λ_max to scale
    against), so the `λ_max < tolerance` test could no longer catch that case
    and the ratio would have evaluated to `0/0 = NaN` where the old absolute
    threshold returned `0.0`.

  Normalized Laplacians (`L_sym` / `L_rw`), listed as optional in #166, are
  **deferred** to [#223](https://github.com/sustia-llc/catgraph/issues/223) —
  they add public API and want their own test matrix.

## [workspace-v0.4.0] - 2026-07-25

### Added

- **`docs/ANCHORS.md` provenance note**
  ([#124](https://github.com/sustia-llc/catgraph/issues/124)): the crate is
  **inspiration-anchored**, not theorem-anchored — the note maps each
  attribution site to its source and cache status: [Gor23]
  (arXiv:2301.04690, cached, ✅ verified in Phase 4) vs the uncached (†)
  attributions [Gor20a]/[Gor20b]/[Oll09]/[Vil03]/[EPS73]. README + root
  CLAUDE.md paper-anchor list link it.

### Changed

- **`benches/wasserstein_bench.rs` inline LCG replaced by
  `catgraph-testutil::Lcg`**
  ([#33](https://github.com/sustia-llc/catgraph/issues/33) item 2) — the bench's
  local LCG closure moves to the shared, dev-only `catgraph-testutil` crate. The
  random stream is **byte-identical**: this site historically used the
  non-standard increment `1` (not the MMIX increment the magnitude sites use),
  preserved via `Lcg::with_increment(42, 1)`. No behavior change.
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

> **Reconciliation note
> ([#158](https://github.com/sustia-llc/catgraph/issues/158)).** Workspace tags
> `v0.1.1`, `v0.2.0`, `v0.2.1`, and `v0.3.0` (2026-07-02 → 2026-07-11) were cut
> without per-crate sections here; this crate's changes across them are recorded
> only in git history (`git log v0.1.0..v0.3.0 -- catgraph-physics/`) and the
> workspace-level release record. Backfill was deferred out of the v0.4.0
> release (owner, 2026-07-25) and resolved as this note (#158, option 2).
> Separately, `v0.5.0` deliberately rolled no section here — the crate had no
> changes at that tag.

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

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.14.0...HEAD
[workspace-v0.14.0]: https://github.com/sustia-llc/catgraph/compare/v0.13.0...v0.14.0
[workspace-v0.9.0]: https://github.com/sustia-llc/catgraph/compare/v0.8.0...v0.9.0
[workspace-v0.4.0]: https://github.com/sustia-llc/catgraph/compare/v0.1.0...v0.4.0
[workspace-v0.1.0]: https://github.com/sustia-llc/catgraph/releases/tag/v0.1.0
[0.2.2]: https://github.com/tsondru/catgraph/releases/tag/catgraph-physics-v0.2.2
[0.2.1]: https://github.com/tsondru/catgraph/releases/tag/catgraph-physics-v0.2.1
[0.2.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-physics-v0.2.0
[0.1.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-physics-v0.1.0
