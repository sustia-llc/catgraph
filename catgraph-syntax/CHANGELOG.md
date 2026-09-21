# Changelog — catgraph-syntax

All notable changes to this crate are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); semver per
[SemVer 2.0.0](https://semver.org/spec/v2.0.0.html). Versioning is
workspace-wide: this crate's versions track the repo's `v0.x` tags.

## [Unreleased]

## [workspace-v0.19.0] - 2026-09-07

### Changed — tests

- `tests/persistence.rs` decides its premise in-test with `eq_mod` on the
  axiom-free presentation; `tests/printer_golden.rs` drops the definitional
  `pretty_adapter_agrees_with_print`
  ([#422](https://github.com/sustia-llc/catgraph/pull/422)).

## [workspace-v0.18.0] - 2026-09-05

### Added

- `tests/canonical.rs`: the print / parse round trip, `eval` against
  `MatrixNFFunctor`, `CospanFunctor` on the nine `E_frob` equations, the
  `MatKron` golden, the `lift_user` pin and the serde variant list
  ([#415](https://github.com/sustia-llc/catgraph/pull/415)).

### Changed

- `lift_user` returns `SyntaxError::RecursionLimit` past `MAX_TERM_DEPTH`; the
  `RecursionLimit` rustdoc names it
  ([#415](https://github.com/sustia-llc/catgraph/pull/415)).
- README gains §Canonical test
  ([#418](https://github.com/sustia-llc/catgraph/pull/418)).

### Removed

- `tests/cospan_complete_functor.rs`, `tests/eval.rs` and
  `tests/serde_roundtrip.rs`, subsumed by `tests/canonical.rs`
  ([#415](https://github.com/sustia-llc/catgraph/pull/415)).

## [workspace-v0.17.0] - 2026-09-03

### Changed

- Rustdoc reduced to contract statements and stale section-name
  cross-references repointed; this CHANGELOG is one bullet per change
  ([#365](https://github.com/sustia-llc/catgraph/issues/365)).
- `docs/ANCHORS.md` lists `Wire`, `WireCount`, `PairSwap`,
  `FrobeniusEquation` and `MAX_NESTING_DEPTH` on the row of the module
  defining each; its header names the enumeration command
  ([#407](https://github.com/sustia-llc/catgraph/pull/407)).

### Fixed — tests

- `FrobeniusOr::Delta`/`Eta` print/parse pinned at the implicit sort (token
  round-trip at both the bare and colour-annotated form, plus a
  presentation-file golden) — `Epsilon` already had this coverage
  ([#316](https://github.com/sustia-llc/catgraph/issues/316)).
- Each of `scfm_equations`' nine Def 2.5 equations pinned to a `d=2`
  `MatKron` image (shared across the slots the algebra makes coincide) and
  its own concrete syntax; `to_mat_kron` and `CospanFunctor` pinned to agree
  per palette colour on all nine, and `to_mat_kron`'s braid pinned at the
  reversed mixed-colour boundary (`[B, A] ↦ braiding(3, 2)`)
  ([#317](https://github.com/sustia-llc/catgraph/issues/317)).

---

> 📄 **Sections before `workspace-v0.17.0` are archived.** Every section from
> `[workspace-v0.16.0]` down was moved out of this file verbatim on 2026-09-21
> and is held outside this repository. The file as it stood before the move
> is at tag
> [`v0.23.0`](https://github.com/sustia-llc/catgraph/blob/v0.23.0/catgraph-syntax/CHANGELOG.md).

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.23.0...HEAD
[workspace-v0.19.0]: https://github.com/sustia-llc/catgraph/compare/v0.18.0...v0.19.0
[workspace-v0.18.0]: https://github.com/sustia-llc/catgraph/compare/v0.17.0...v0.18.0
[workspace-v0.17.0]: https://github.com/sustia-llc/catgraph/compare/v0.16.0...v0.17.0
