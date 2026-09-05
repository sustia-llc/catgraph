# Changelog

All notable changes to this crate are documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). This
crate is **unpublished** (`publish = false`, dev-only): it cuts no releases, so
entries accumulate under `[Unreleased]` as a change record rather than a
version history.

## [Unreleased]

### Changed

- Rustdoc in `src/` states what each item does over what input space; this
  CHANGELOG is one bullet per change
  ([#365](https://github.com/sustia-llc/catgraph/issues/365)).

### Added

- `wiring`: `Leg`, `Wiring::shift_concat`, `CospanWiring::pushout` /
  `signature` / `to_wiring`, `PartitionSignature` — index-wiring references
  for tensor and pushout claims
  ([#410](https://github.com/sustia-llc/catgraph/pull/410)).
- `all_perms` + `all_perm_indices`, re-exported at the crate root: exhaustive
  `Sₙ` enumeration by prefix swaps, in two index-aligned views —
  `all_perms(n)[k]` is the permutation whose one-line notation is
  `all_perm_indices(n)[k]`
  ([#286](https://github.com/sustia-llc/catgraph/issues/286)).
- `permutations` dependency, required by `all_perms`; this crate has no
  `catgraph` edge ([#286](https://github.com/sustia-llc/catgraph/issues/286)).
- `approx_rel` (re-exported at the crate root) + `assert_approx_rel!`:
  relative-plus-absolute float comparison,
  `|a − b| <= max(abs, rel · max(|a|, |b|))`, with NaN close to nothing and
  matching infinities close; the macro reports both tolerances and the residual
  on failure ([#169](https://github.com/sustia-llc/catgraph/issues/169)).
- `strategy` module — `wide_range_f64` (bit-pattern-uniform finite floats,
  subnormals, the `-1e6..1e6` band, and explicit edge cases, at weights
  3/1/2/1) and `near_cancellation_pair` (`(a, a · (1 + δ))` with `|δ|`
  log-uniform on `[1e-17, 1e-8)`, filtered to finite and `a != b`)
  ([#169](https://github.com/sustia-llc/catgraph/issues/169)).
- `proptest` dependency, required by the strategies
  ([#169](https://github.com/sustia-llc/catgraph/issues/169)).
- Initial crate: deterministic `Lcg` (Knuth MMIX multiplier) for seeded
  test/bench fixtures — `new(seed)` (standard increment),
  `with_increment(seed, increment)` (the `catgraph-physics` wasserstein bench's
  increment `1`), `next_f64`, `next_usize`; seed preparation such as `| 1`
  stays at call sites. Replaces the inline copies in catgraph-magnitude and
  catgraph-physics with a byte-identical stream, pinned by a golden-value unit
  test over `to_bits` ([#33](https://github.com/sustia-llc/catgraph/issues/33)).
