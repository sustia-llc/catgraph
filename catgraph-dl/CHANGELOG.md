# Changelog — catgraph-dl

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [workspace-v0.24.0] - 2026-09-22

### Changed

- Sections before `workspace-v0.17.0` moved out of this file; the file before
  the move is at tag `v0.23.0`
  ([#462](https://github.com/sustia-llc/catgraph/pull/462)).

## [workspace-v0.21.0] - 2026-09-08

### Added

- `Dual<f64>::is_finite` and `RModule<Dual<f64>>::is_finite` under `ad`
  ([#448](https://github.com/sustia-llc/catgraph/pull/448)).

## [workspace-v0.20.0] - 2026-09-08

### Added

- `RModule<f64>::is_finite` ([#444](https://github.com/sustia-llc/catgraph/pull/444)).

### Docs

- The fold surface cites Def A.2 initiality; Remark 2.13 stays on the list
  instances, Ex 2.14 on the tree ones
  ([#443](https://github.com/sustia-llc/catgraph/pull/443)).

## [workspace-v0.19.0] - 2026-09-07

### Added — tests

- `tests/ad_module_laws.rs`: nested `Dual<Dual<T>>` pins; `src/para/dual.rs`
  documents the nesting
  ([#429](https://github.com/sustia-llc/catgraph/pull/429)).

## [workspace-v0.18.0] - 2026-09-05

### Added

- `Functor<FreeWitness<F>>` and `Functor<CofreeWitness<F>>` for
  `F: Container` ([#416](https://github.com/sustia-llc/catgraph/pull/416)).
- `tests/canonical.rs`: the functor laws on every shipped `HKT` witness, the
  container laws on the four `Container` implementors, the B.19 / B.20
  bijections, the five architectures against `Free` / `Cofree` walkers, and
  `unroll_iter` / `run_iter` call counters
  ([#416](https://github.com/sustia-llc/catgraph/pull/416)).

### Changed

- README, `tests/THEOREM_MAP.md`, `docs/2402.15332v2-AUDIT.md`,
  `examples/README.md` and the `hkt.rs` / `depth.rs` / `group_action.rs`
  comments point at `tests/canonical.rs`; the `leftmost_leaf` rustdoc
  sentence on payload-agnostic cells is cut
  ([#416](https://github.com/sustia-llc/catgraph/pull/416)).
- README gains §Canonical test
  ([#418](https://github.com/sustia-llc/catgraph/pull/418)).

### Removed

- `tests/functor_laws.rs`, `tests/container_laws.rs`,
  `tests/architecture_unrollers.rs`, `tests/free_monad_bijections.rs` and
  `examples/architecture_unrollers.rs` with its `ci.yml` loop entry, subsumed
  by `tests/canonical.rs` ([#416](https://github.com/sustia-llc/catgraph/pull/416)).

## [workspace-v0.17.0] - 2026-09-03

### Changed

- The `ignore` doctest fences in `architectures/{folding_rnn, mealy_cell,
  moore_cell, unfolding_rnn}` and `free_monad/list_endo` compile and run as
  doctests ([#407](https://github.com/sustia-llc/catgraph/pull/407)).
- `docs/2402.15332v2-SUMMARY.md` line-audited against the paper: appendix
  range A–J, the free-monad formula at Proposition B.18, Definition G.1,
  Remark H.6 and Example H.8
  ([#406](https://github.com/sustia-llc/catgraph/pull/406)).
- `Dual` gains `verify_rig_axioms` call sites (`para/dual.rs`)
  ([#292](https://github.com/sustia-llc/catgraph/issues/292)).
- Rustdoc reduced to contract statements; this CHANGELOG rewritten to one
  bullet per change. `architectures` module doc no longer describes
  `RecursiveNn::unroll` as fallible (it has returned `S` since v0.15.0).
- `Free::fold` and `Cofree::unfold` rustdoc state position order; the
  evaluation-order clauses are gone
  ([#310](https://github.com/sustia-llc/catgraph/issues/310)).
- The list-bijection tests compare `Free` values structurally: the backward
  proptest leg asserts `f2 == f1`, and the cons-cell tower is compared to its
  canonical encoding ([#311](https://github.com/sustia-llc/catgraph/issues/311)).
- `free_mnd_to_vec` gains a `#[should_panic]` pin on a bare
  `Free::suspend(None)` reaching the panic branch with no `Pure` terminator
  above it ([#312](https://github.com/sustia-llc/catgraph/issues/312)).
- `ListEndo` joins the `Debug`-twin, `size_of` and deep-spine pins: a
  `#[derive(Debug)]` twin at `u8` and `f64` payloads and at both bottoms,
  `==`/`!=` through `Free<ListEndo<_>, _>`, a `size_of` relation and 64-bit
  byte rows, and a `DEEP` cons tower in the #200 pin, whose prose now
  enumerates operations per carrier. The twin macro grew the
  width-and-precision forms and a second value pair for every spec-carrying
  renderer arm ([#313](https://github.com/sustia-llc/catgraph/issues/313)).

---

> 📄 **Sections before `workspace-v0.17.0` are archived.** Every section from
> `[workspace-v0.15.0]` down was moved out of this file verbatim on 2026-09-21
> and is held outside this repository. The file as it stood before the move
> is at tag
> [`v0.23.0`](https://github.com/sustia-llc/catgraph/blob/v0.23.0/catgraph-dl/CHANGELOG.md).

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.25.0...HEAD
[workspace-v0.24.0]: https://github.com/sustia-llc/catgraph/compare/v0.23.0...v0.24.0
[workspace-v0.21.0]: https://github.com/sustia-llc/catgraph/compare/v0.20.0...v0.21.0
[workspace-v0.20.0]: https://github.com/sustia-llc/catgraph/compare/v0.19.1...v0.20.0
[workspace-v0.19.0]: https://github.com/sustia-llc/catgraph/compare/v0.18.0...v0.19.0
[workspace-v0.18.0]: https://github.com/sustia-llc/catgraph/compare/v0.17.0...v0.18.0
[workspace-v0.17.0]: https://github.com/sustia-llc/catgraph/compare/v0.16.0...v0.17.0
