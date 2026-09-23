# Changelog

All notable changes to `catgraph-magnitude` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [workspace-v0.24.0] - 2026-09-22

### Changed

- Sections before `workspace-v0.17.0` moved out of this file; the file before
  the move is at tag `v0.23.0`
  ([#462](https://github.com/sustia-llc/catgraph/pull/462)).

## [workspace-v0.21.0] - 2026-09-08

### Changed

- `SCHUR_SLOW_FALLBACK_TOL` is `1e-7` (was `1e-12`): every border whose
  incremental value differed from fresh evaluation above
  `INCREMENTAL_REL_TOL` on the property corpus is diverted to
  `EvalPath::SlowNearSingular`
  ([#450](https://github.com/sustia-llc/catgraph/pull/450)).
- The coalition closure builds its max-product entries through
  `UnitInterval::from_rig_value`; couplings below `UNIT_INTERVAL_FLOOR`
  (re-exported) are rejected at every entry point instead of tripping the
  closure's triangle-inequality assert
  ([#451](https://github.com/sustia-llc/catgraph/pull/451)).

## [workspace-v0.20.0] - 2026-09-08

### Added — tests

- `tests/coalition_schur_guard_props.rs`: parity of `value_with` with fresh
  evaluation, the `EvalPath` census and the delta base over a generated family
  ([#441](https://github.com/sustia-llc/catgraph/pull/441)).
- `tests/magnitude.rs`: the Tsallis switch over `|t − 1| ∈ [1e-9, 1e-2)` and
  across its branch cut ([#442](https://github.com/sustia-llc/catgraph/pull/442)).

## [workspace-v0.19.1] - 2026-09-07

### Changed

- `CoalitionEvaluator::value_with` returns the cached `base_value()` as
  `EvalPath::MergeOnly` (a new variant) with the `SkeletalMerge` proof and no
  Schur complement when the candidate is a mutual-`1.0` clone of a member and
  opens no interior shortcut; a merge with an interior improvement stays
  `Slow` ([#436](https://github.com/sustia-llc/catgraph/pull/436)).

### Added — tests

- `seeded_grid_snap45_merge_only` (seed `0x153_D00D`, snap 0.45) and
  `bench_merge_only_fixture_is_merge_only`; `benches/magnitude_bench.rs` gains
  `coalition_incremental/merge_only_sweep`
  ([#436](https://github.com/sustia-llc/catgraph/pull/436)).

## [workspace-v0.19.0] - 2026-09-07

### Changed

- `snf_rank_with_cross_check` returns the larger of the two agreeing ranks on
  the tertiary-prime branch
  ([#420](https://github.com/sustia-llc/catgraph/pull/420)).
- `phase_1_to_bidiagonal` carries a `debug_assert!` that each `band_reduction`
  step lowers the bandwidth; its `# Panics` names it
  ([#419](https://github.com/sustia-llc/catgraph/pull/419)).

### Added — tests

- SNF layer: `assert_unimodular` in `tests/common/snf_invariants.rs`, echelon
  rank and `T` assertions with a rank-deficient 2×2, phase-1 unimodularity
  ([#419](https://github.com/sustia-llc/catgraph/pull/419)).
- `tests/homology_rank_svd_oracle.rs` (`f64-fast`): homology rank and
  per-prime SNF rank against an SVD rank oracle, plus the `snf_rank_with_cross_check`
  branch pins in `src/chain_complex/homology.rs`
  ([#420](https://github.com/sustia-llc/catgraph/pull/420)).
- `tests/chain_complex_basic.rs` boundary shape and entry pins;
  `tests/mobius_invertibility.rs` asserts the full graded count vector
  ([#421](https://github.com/sustia-llc/catgraph/pull/421)).
- `tests/snf_band.rs`: `band_reduction_needs_every_shift_step` at `t_param = 4`
  ([#429](https://github.com/sustia-llc/catgraph/pull/429)).
- `tests/weighted_cospan.rs`: the file-local `cospan_eq` is `prop_assert_eq!`
  ([#430](https://github.com/sustia-llc/catgraph/pull/430)).

## [workspace-v0.18.0] - 2026-09-05

### Added

- `tests/canonical.rs`: the four README acceptance gates, a wrong-μ `Err` arm
  for `verify_mobius_recursion`, and `magnitude_homology_rank` at `(2, 2)` on
  the three-point geodesic line
  ([#413](https://github.com/sustia-llc/catgraph/pull/413)).

### Changed

- The CI release lane runs `--test canonical`; README §Acceptance gates and
  the `mobius_function_via_chains` rustdoc point at `tests/canonical.rs`
  ([#413](https://github.com/sustia-llc/catgraph/pull/413)).
- `docs/BV25-AUDIT.md` cites test files without a count
  ([#412](https://github.com/sustia-llc/catgraph/pull/412)).

### Removed

- `tests/bv_2025_acceptance.rs`, `tests/mobius_chains.rs`,
  `tests/euler_char_identity.rs`, `tests/mobius_chains.proptest-regressions`
  and `examples/prop_3_14_acceptance.rs`, subsumed by `tests/canonical.rs`
  ([#413](https://github.com/sustia-llc/catgraph/pull/413)).

## [workspace-v0.17.0] - 2026-09-03

### Changed — BREAKING

- `EvalPath` is `#[non_exhaustive]` and gains `SlowNearSingular`, the path
  taken when the `SCHUR_SLOW_FALLBACK_TOL` guard diverts a near-singular
  Schur complement; `schur_complement()` is `Some` iff the path is `Fast`
  ([#307](https://github.com/sustia-llc/catgraph/issues/307)).

### Changed

- Rustdoc across `src/` states what each item does over what input space;
  this CHANGELOG is one bullet per change
  ([#403](https://github.com/sustia-llc/catgraph/pull/403)).
- `magnitude_f64` labels BV 2025 Eq (7) with its Def 3.1 / §3.5 loci;
  `mobius_chains` states the relative tolerances its tests assert;
  `docs/BTV21-AUDIT.md` deferred rows point at #405
  ([#407](https://github.com/sustia-llc/catgraph/pull/407)).
- CI `check` runs the sub-second examples, and runs
  `tests/snf_modularsnf_integer_oracle.rs` plus clippy under
  `modularsnf-oracle`; its three rank-mod-p proptests run at a second prime
  ([#305](https://github.com/sustia-llc/catgraph/issues/305),
  [#276](https://github.com/sustia-llc/catgraph/issues/276)).

### Added — tests

- A deterministic near-singular fixture; per-stratum evaluated-pair counters
  in `mag_bounds_intro`; `EvalPath` hit counters in the seeded sweeps and
  `rank_order_identity`; an argsort rank comparison on the seeded grid; a
  role-grid factorization proptest over product couplings
  ([#307](https://github.com/sustia-llc/catgraph/issues/307)).

---

> 📄 **Sections before `workspace-v0.17.0` are archived.** Every section from
> `[workspace-v0.16.0]` down was moved out of this file verbatim on 2026-09-21
> and is held outside this repository. The file as it stood before the move
> is at tag
> [`v0.23.0`](https://github.com/sustia-llc/catgraph/blob/v0.23.0/catgraph-magnitude/CHANGELOG.md).

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.24.0...HEAD
[workspace-v0.24.0]: https://github.com/sustia-llc/catgraph/compare/v0.23.0...v0.24.0
[workspace-v0.21.0]: https://github.com/sustia-llc/catgraph/compare/v0.20.0...v0.21.0
[workspace-v0.20.0]: https://github.com/sustia-llc/catgraph/compare/v0.19.1...v0.20.0
[workspace-v0.19.1]: https://github.com/sustia-llc/catgraph/compare/v0.19.0...v0.19.1
[workspace-v0.19.0]: https://github.com/sustia-llc/catgraph/compare/v0.18.0...v0.19.0
[workspace-v0.18.0]: https://github.com/sustia-llc/catgraph/compare/v0.17.0...v0.18.0
[workspace-v0.17.0]: https://github.com/sustia-llc/catgraph/compare/v0.16.0...v0.17.0
