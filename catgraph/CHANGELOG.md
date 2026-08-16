# Changelog

All notable changes to `catgraph` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the crate adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed — BREAKING

- **`Cospan::new` and `Span::new` are now validated constructors returning
  `Result<Self, CatgraphError>`; the previous infallible bodies moved to
  `Cospan::new_unchecked` / `Span::new_unchecked`**
  ([#256](https://github.com/sustia-llc/catgraph/issues/256)). Until now the
  structural invariants were checked by `debug_assert!` only, so a **release**
  build accepted a cospan whose leg pointed outside its apex and deferred the
  failure to whatever indexed it later — a composition, a downstream store's
  canonicalisation, or nothing at all. `new` now performs those same checks
  unconditionally, in every profile:
  - `Cospan::new` — every `left` and every `right` entry must be `< middle.len()`.
  - `Span::new` — every middle pair's `.0` must be `< left.len()` and its `.1`
    `< right.len()`, **and** the two labels it names must agree
    (`left[pair.0] == right[pair.1]`).

  This is the crate's existing convention, not a new one: `Rel::new` /
  `Rel::new_unchecked` and `Corel::new` / `Corel::new_unchecked` already split
  this way, with the **checked** constructor owning the plain name. Shipping a
  `try_new` beside an infallible `new` would have left two opposite idioms for
  the same thing on types one layer apart.

  **Migration.** Data that is correct *by construction* — composition results,
  `identity`/`unit`/`counit`/`multiplication`/`comultiplication`, permutation
  builders, monoidal products, anything rebuilt from an already-valid value —
  should call `new_unchecked`, which is exactly the old `new` and costs nothing
  in release. Reserve `new` for data crossing a trust boundary: a store, a wire
  format, a parser, a user. In tests, `.unwrap()` is the expected fix.

- **`NamedCospan::new` is now a validated constructor returning
  `Result<Self, CatgraphError>`; the previous infallible body moved to the new
  `NamedCospan::new_unchecked`**
  ([#256](https://github.com/sustia-llc/catgraph/issues/256)). This was the last
  public constructor in core that accepted a leg map pointing outside its apex
  in a **release** build: it took raw `Vec<MiddleIndex>` legs — a trust boundary
  by the same criterion as `Cospan::new` — but built its inner cospan with
  `Cospan::new_unchecked`. `new` now validates both structural invariants
  unconditionally, in every profile:
  - **one name per port** — `left_names.len() == left.len()` and
    `right_names.len() == right.len()`. These were `assert!`s, so they aborted
    the process in every profile; they are now errors. A constructor that
    returns `Result` for one precondition and panics for another is only
    half-checked, and a name-count mismatch is precisely the corruption a
    caller reconstructing a named cospan from stored columns hits.
  - **leg bounds** — delegated to `Cospan::new` rather than re-implemented, so
    the check has one home and one error shape.

  The name counts are checked before the leg bounds, and the domain side before
  the codomain side, so the reported failure is the first one in that order.

  **Migration.** Same rule as `Cospan`/`Span` above: `new_unchecked` for data
  correct by construction, `new` for data crossing a trust boundary, `.unwrap()`
  in tests. `NamedCospan::empty` moved to `new_unchecked` and is unchanged
  behaviourally. Note `new_unchecked` is *uniformly* `debug_assert!`-only —
  including the name counts, which the old `new` enforced with a hard `assert!`.
  Keeping a release panic for one of its two invariants would have made the
  `_unchecked` suffix mean two different things on one constructor and broken
  the zero-release-cost contract `Cospan::new_unchecked` documents.

- **`CatgraphError` is `#[non_exhaustive]`.** Downstream `match`es must carry a
  wildcard arm; after this release a new variant is no longer a breaking
  change. Core was the last crate in the workspace whose error enum was not
  marked — `catgraph-syntax::SyntaxError` and `catgraph-dl::DepthError` already
  were.

### Added

- **`CatgraphError::ConstructionIndexOutOfBounds { leg, position, target,
  target_len }`** — a cospan or span leg entry targets an index outside the set
  it must land in. `leg` is the new `errors::BoundaryLeg` (`Domain` /
  `Codomain`, rendered as `domain`/`codomain`), `position` is the entry's index
  within that leg, `target` the out-of-range value, `target_len` the size of
  the set it had to land in. The vocabulary deliberately matches what the
  downstream `catgraph-surreal` store already reports for a corrupt leg, so the
  store can retire its parallel spelling.
- **`CatgraphError::ConstructionLabelMismatch { position, left_index,
  right_index, left_label, right_label }`** — a `Span` middle pair links a
  domain element to a codomain element carrying a different label. `left_label`
  / `right_label` are the `Debug` renderings of the two labels.
- **`CatgraphError::ConstructionNameCountMismatch { leg, boundary_len,
  name_count }`** — a `NamedCospan` was handed a port-name list whose length
  does not match the boundary it names. `leg` reuses `BoundaryLeg`,
  `boundary_len` is `left.len()` / `right.len()`, `name_count` the number of
  names supplied for it. Raised by `NamedCospan::new`.
- **`errors::BoundaryLeg`** — `Domain` / `Codomain`, with `as_str()` and
  `Display`. Not `#[non_exhaustive]`: a span or cospan has exactly two legs, so
  matching it exhaustively is safe.
- **`CospanCanon` round-trips: `CospanCanon::from_parts`,
  `CospanCanon::to_cospan`, and `ApexClass::new`**
  ([#261](https://github.com/sustia-llc/catgraph/issues/261)). Purely
  **additive** — no existing signature changes. Until now the only way to obtain
  a canonical form was `Cospan::canonical_form()`, so a consumer could read one
  (`classes()`, since #254), persist it, and log it, but never *reload* it: to
  compare against a stored form after a restart it had to keep the originating
  cospan and re-run `canonical_form()`, which is the work the canonical form
  exists to save.
  - `ApexClass::new(label, dom_preimage, cod_preimage) -> Self` — assembles one
    class signature. Infallible and **non-validating**: neither documented
    invariant is a property of a class in isolation.
  - `CospanCanon::from_parts(dom_len, cod_len, classes) -> Result<Self,
    CatgraphError>` — re-establishes all three: `classes` sorted under
    `ApexClass`'s `Ord`, each preimage strictly ascending, and the preimages
    partitioning `0..dom_len` / `0..cod_len` (the "each leg is a function"
    property). `CospanCanon`'s `Eq`/`Hash` decide apex isomorphism *because of*
    those invariants, so a constructor that skipped them would hand back a value
    unequal to the `canonical_form()` of every cospan.
  - `CospanCanon::to_cospan(&self) -> Cospan<Lambda>` — rebuilds a witnessing
    cospan: apex = classes in canonical order, `left[i]` = the class whose
    `dom_preimage` contains `i`, `right[k]` likewise. Scalars (bubbles) are
    placed in the apex although no leg reaches them, so `k` bubbles round-trip
    as `k`. The apex comes back in canonical order, which is generally *not*
    `structurally_equal` to the originating cospan — that difference is
    precisely the apex labelling the form forgets.

  **Rejects, does not repair.** Nothing sorts the input into shape. The intended
  input is reloaded data, where silently repairing a malformed value hides
  corruption exactly where the caller most needs to hear about it — same posture
  as `Cospan::new` above.

  **The round trip is the oracle, not a convenience.** The three invariants are
  not merely necessary but *sufficient* to rebuild a witness, so
  `c.canonical_form().to_cospan().canonical_form() == c.canonical_form()` is a
  real property test for the validation rather than a hand-checked list.
- **Four `CatgraphError` variants for canonical-form construction**, all raised
  by `CospanCanon::from_parts` and all reusing `BoundaryLeg`:
  `CanonClassesNotSorted { position }`;
  `CanonPreimageNotAscending { leg, class_position, position }`;
  `CanonPreimageOutOfBounds { leg, class_position, position, index,
  boundary_len }`; and
  `CanonPreimageNotAPartition { leg, index, occurrences, boundary_len }`, where
  `occurrences` is `0` for an unclaimed boundary index and `>= 2` for one
  claimed by several classes. `CanonPreimageOutOfBounds` is deliberately not
  `ConstructionIndexOutOfBounds`: that one points the other way (a leg entry
  overshoots the apex), and locating this one needs the class's position as well
  as the position within the preimage.

### Fixed

- **`Span::assert_valid` no longer panics in release.** Its label-agreement
  check indexes both boundaries, and the check was computed into a `let`
  *before* being handed to `debug_assert!` — so the indexing ran in every
  profile and an out-of-bounds middle pair produced a bare `index out of
  bounds` panic in release, which is neither what the method's name promises
  nor what `new_unchecked` documents. Both `assert_valid` methods now write
  every check inside its `debug_assert!`, so they compile away entirely in
  release. Debug behaviour is unchanged, including the order in which the
  invariants are reported (bounds before labels, so a debug build still names
  the specific invariant rather than index-panicking).

## [workspace-v0.12.0] - 2026-08-15

### Added

- **`cospan_canon::ApexClass<Λ>` and `CospanCanon::classes`** — a read surface
  for the data that actually discriminates a canonical form
  ([#254](https://github.com/sustia-llc/catgraph/issues/254)). `CospanCanon`
  already derived `Eq`/`Hash` and worked as a key, but its apex signatures were
  private and the accessors stopped at `dom_len`/`cod_len`/`scalar_count`/
  `apex_len`, so a consumer could compare two canonical forms in-process and
  nothing else. `classes()` now returns `&[ApexClass<Λ>]`, and each class
  exposes `label()`, `dom_preimage()`, `cod_preimage()` (both preimages sorted
  ascending — a documented invariant callers may rely on), and `is_scalar()`
  for the both-empty bubble case. A canonical form can therefore be inspected,
  written out, re-encoded into another representation, or logged, not only
  compared. No serde: the core crate carries none, and this is a read surface,
  not a wire format.
  - **Read-only, deliberately: this is not a round trip.** There is no
    `CospanCanon::from_parts` and no `ApexClass::new`, so a form that has been
    written out cannot be read back into a `CospanCanon` — a consumer wanting
    to compare against stored data after a restart still keeps the originating
    `Cospan` and re-runs `canonical_form()`. A public constructor cannot be a
    plain `new`: the type's `Eq`/`Hash` decide cospan isomorphism only because
    the private construction path guarantees sortedness of the classes, of each
    preimage, and that the preimages partition the two boundaries. Whether the
    type should round-trip, and under which of those validations, is
    [#261](https://github.com/sustia-llc/catgraph/issues/261).
  - **The canonical form itself is unchanged** — its `Eq`, its `Hash`, and its
    sort order are all bit-identical to before, so persisted comparisons remain
    valid and this is *not* a format change. `ApexClass` replaces an anonymous
    `(Λ, Vec<usize>, Vec<usize>)` tuple in a **private** field; its fields are
    declared in that same order and it derives the same traits, so the derived
    `Ord` driving the canonicalising `classes.sort()` — the sort that makes the
    value invariant under apex relabelling — is exactly the tuple's
    lexicographic order. A test pins this against an in-test rebuild of the
    tuple vector sorted under tuple `Ord`, and pins the field-by-field
    tie-break directly.
  - `scalar_count` is rewritten as a filter on `is_scalar()`; behaviour,
    signature, and docs are unchanged, as are `CospanCanon`'s derives and its
    four existing accessors.

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

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.13.0...HEAD
[workspace-v0.12.0]: https://github.com/sustia-llc/catgraph/compare/v0.11.0...v0.12.0
[workspace-v0.10.0]: https://github.com/sustia-llc/catgraph/compare/v0.9.0...v0.10.0
[workspace-v0.9.0]: https://github.com/sustia-llc/catgraph/compare/v0.8.0...v0.9.0
[workspace-v0.6.0]: https://github.com/sustia-llc/catgraph/compare/v0.5.0...v0.6.0
[workspace-v0.4.0]: https://github.com/sustia-llc/catgraph/compare/v0.1.0...v0.4.0
[workspace-v0.1.0]: https://github.com/sustia-llc/catgraph/releases/tag/v0.1.0
