# Changelog

All notable changes to `catgraph` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the crate adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [workspace-v0.24.0] - 2026-09-22

### Added

- `CanonicalEncode` (canonical byte encoding; impls for the integer types,
  `usize`/`isize` as 64-bit, `bool`, `char`, `str`, `String`, `[T]`, `Vec`,
  `[T; N]`, `VecDeque`, `Option`, `()`, tuples of arity 1–6, `&T`, `Box<T>`,
  `BTreeSet`, `BTreeMap`) and `canonical_fingerprint` (the first 8 bytes of
  BLAKE3 over the encoding, little-endian); new dependency `blake3`
  ([#469](https://github.com/sustia-llc/catgraph/pull/469)).

### Changed

- Sections before `workspace-v0.17.0` moved out of this file; the file before
  the move is at tag `v0.23.0`
  ([#462](https://github.com/sustia-llc/catgraph/pull/462)).

## [workspace-v0.20.0] - 2026-09-08

### Changed — BREAKING

- `CatgraphError::Rewrite(RewriteRejection)` carries the rewrite module's
  user-attributable rejections; `RewriteRejection` (`#[non_exhaustive]`:
  `SidesNotParallel`, `IllFormed`, `EmptyLhs`, `LhsInterfaceNotMono`,
  `StaleSite`, `NotAMatch`, `UnknownRule`) with `RewriteBoundary` and
  `RewriteSide`. Rejections that surfaced as `Presentation` from
  `catgraph-applied`'s rewrite entry points surface as `Rewrite`, with new
  message text ([#447](https://github.com/sustia-llc/catgraph/pull/447)).

### Added — tests

- `frobenius::to_cospan_pin`'s oracle derives its braiding arm through
  `from_permutation_on_domain`; the space gains `σ_{a,a}` and `σ_{a,a} ; σ_{a,a}`
  ([#440](https://github.com/sustia-llc/catgraph/pull/440)).

## [workspace-v0.19.0] - 2026-09-07

### Changed — BREAKING

- `Span::is_left_identity()` / `is_right_identity()` are computed on each call
  as the boundary-length conjunct with `represents_id` on the leg; the cached
  `is_left_id` / `is_right_id` flags are gone and `Span::assert_valid()` takes
  no parameters ([#424](https://github.com/sustia-llc/catgraph/pull/424)).
- `Cospan::structurally_equal` deleted; `==` is the same predicate
  ([#430](https://github.com/sustia-llc/catgraph/pull/430)).

### Changed

- The eight `.unwrap()`s in `Rel` (`union`, `intersection`, the four
  predicates) are `.expect` naming the precondition each `# Panics` states;
  `Rel::complement`'s `# Errors` names the grid-cell condition `add_middle`
  checks ([#430](https://github.com/sustia-llc/catgraph/pull/430)).
- `tests/common/mod.rs` loses `cospan_eq` / `assert_cospan_eq` /
  `assert_cospan_eq_msg`; callers use `==` / `assert_eq!`
  ([#430](https://github.com/sustia-llc/catgraph/pull/430)).
- `tests/spider_theorem.rs` lifts the `m == n == 0` and component-closing
  exclusions; both claim tests assert `scalar_count()` and `apex_len()` on
  every corpus term ([#423](https://github.com/sustia-llc/catgraph/pull/423)).
- Hand-maintained test counts in `src/hypergraph_category.rs` and five test
  files are cut or dated by the commit that measured them
  ([#431](https://github.com/sustia-llc/catgraph/pull/431)).

### Added — tests

- `tests/checked_mutators.rs`: `Span` identity pins over the boundary length,
  recomputation, the permutation constructors and six mutators
  ([#424](https://github.com/sustia-llc/catgraph/pull/424)).
- `Rel` set-law, dedup, boundary-guard and `complement`-`Err` pins
  ([#430](https://github.com/sustia-llc/catgraph/pull/430)).

## [workspace-v0.18.0] - 2026-09-05

### Added

- `tests/canonical.rs`: the eleven Def 2.5 equations, the Def 2.12 generator
  table, both zigzags and strict left/right unitality on the
  `HypergraphCategory` implementors, decided by `CospanCanon`; `compose`
  against a union-find partition reference; the tensor wiring law on the
  public `Monoidal` implementors
  ([#410](https://github.com/sustia-llc/catgraph/pull/410)).
- `Decomposition::to_finset_morphism`; `GenericMonoidalMorphism::append_layer`
  is `pub` ([#410](https://github.com/sustia-llc/catgraph/pull/410)).

### Added — tooling

- `scripts/check_canonical_tests.py`, its `ci.yml` row and CLAUDE.md rule 7:
  every `pub struct|enum|trait|type` under a published crate's `src` is named
  in its `tests/canonical.rs` header's `covers:` or `not-covered:` list
  ([#410](https://github.com/sustia-llc/catgraph/pull/410)).

### Changed

- `GenericMonoidalMorphism::append_layer` and `FrobeniusMorphism::append_layer`
  keep the popped layer on the type-mismatch `Err`; the `tests/canonical.rs`
  `GenericMonoidalMorphism` tensor row runs at unequal depths
  ([#414](https://github.com/sustia-llc/catgraph/pull/414)).
- `tests/common/mod.rs` and `tests/compact_closed.rs` cite test files without a
  count ([#412](https://github.com/sustia-llc/catgraph/pull/412)).
- README §Testing describes `tests/canonical.rs`
  ([#418](https://github.com/sustia-llc/catgraph/pull/418)).

### Removed

- `tests/frobenius_axioms.rs`, subsumed by `tests/canonical.rs`
  ([#410](https://github.com/sustia-llc/catgraph/pull/410)).

## [workspace-v0.17.0] - 2026-09-03

### Changed

- Rustdoc in `src/` states what each item does over what input space; this
  CHANGELOG is one bullet per change
  ([#404](https://github.com/sustia-llc/catgraph/pull/404)).
- `SymmetricMonoidalMorphism::permute_side` rustdoc drops its `PetriNet`
  known-deviation section
  ([#275](https://github.com/sustia-llc/catgraph/issues/275)).
- `equivalence` module doc cites F&S §4 Theorem 4.13 alone
  ([#407](https://github.com/sustia-llc/catgraph/pull/407)).

### Fixed — tests

- `tests/corel_quotient.rs`'s six `MEASURED` emitters each print a leading
  newline, so a multi-threaded `--nocapture` log cannot merge one into
  libtest's `... ok`
  ([#293](https://github.com/sustia-llc/catgraph/issues/293)).

### Added — tooling

- `scripts/check_measured_claims.py`, wired into CI: a measured figure cited in
  prose must equal the fact the emitting test printed, cited by an HTML comment
  placed immediately after the number. 21 citation sites over 6 keys carry
  markers, in this file and in `tests/corel_quotient.rs`. Perturbation results
  and figures inside assertion messages carry no marker
  ([#293](https://github.com/sustia-llc/catgraph/issues/293)).

---

> 📄 **Sections before `workspace-v0.17.0` are archived.** Every section from
> `[workspace-v0.16.0]` down was moved out of this file verbatim on 2026-09-21
> and is held outside this repository. The file as it stood before the move
> is at tag
> [`v0.23.0`](https://github.com/sustia-llc/catgraph/blob/v0.23.0/catgraph/CHANGELOG.md).

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.24.0...HEAD
[workspace-v0.24.0]: https://github.com/sustia-llc/catgraph/compare/v0.23.0...v0.24.0
[workspace-v0.20.0]: https://github.com/sustia-llc/catgraph/compare/v0.19.1...v0.20.0
[workspace-v0.19.0]: https://github.com/sustia-llc/catgraph/compare/v0.18.0...v0.19.0
[workspace-v0.18.0]: https://github.com/sustia-llc/catgraph/compare/v0.17.0...v0.18.0
[workspace-v0.17.0]: https://github.com/sustia-llc/catgraph/compare/v0.16.0...v0.17.0
