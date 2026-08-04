# Changelog

All notable changes to this crate are documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). This
crate is **unpublished** (`publish = false`, dev-only): it cuts no releases, so
entries accumulate under `[Unreleased]` as a change record rather than a
version history.

## [Unreleased]

### Added

- **`approx_rel` + `assert_approx_rel!`**
  ([#169](https://github.com/sustia-llc/catgraph/issues/169)): relative-plus-
  absolute float comparison, `|a − b| <= max(abs, rel · max(|a|, |b|))`. The
  workspace previously compared floats against scattered absolute epsilons, so
  an assertion's tolerance intent was unreadable and the bound silently changed
  meaning with the operands' magnitude. NaN is never close to anything
  (including itself); matching infinities are. The macro reports both
  tolerances and the residual on failure.
- **`strategy` module: `wide_range_f64` + `near_cancellation_pair`**
  ([#169](https://github.com/sustia-llc/catgraph/issues/169)): shared `proptest`
  float strategies, replacing a workspace whose only *named* float strategy was
  `finite_f64 = -1e6..1e6` plus a dozen inline ranges of the same
  uniform-magnitude shape.

  `wide_range_f64` mixes bit-pattern-uniform finite floats (weight 3, for the
  huge exponents), a dedicated subnormal branch (weight 1, a random mantissa
  with the exponent zeroed), the `-1e6..1e6` band (weight 2), and explicit edge
  cases (weight 1). `near_cancellation_pair` yields `(a, a·(1 + δ))` with `|δ|`
  drawn **log-uniformly** from `[1e-17, 1e-8)`, filtered to `a != b`.

  Every coverage claim is backed by a test that samples and counts, because the
  first draft's claims did not survive measurement: the subnormal regime was
  represented by two literal edge values (the bit-pattern branch supplies ~2 per
  20k draws, since only `2⁻¹¹` of finite patterns are subnormal), a linearly
  sampled `δ` put ~99.99% of its mass in the top decade so the deep-cancellation
  end was unreachable, and ~5% of pairs came back with `a == b` (from `±0.0` and
  from subnormal `a`, where `a·δ` falls below half an ULP).
- **`proptest` dependency**
  ([#169](https://github.com/sustia-llc/catgraph/issues/169)) — required by the
  strategies above. Free in practice: this crate is dev-only and unpublished, it
  never appears in a published crate's `[dependencies]` (#33), and every member
  that dev-deps it already dev-deps `proptest`.

- **Initial crate** ([#33](https://github.com/sustia-llc/catgraph/issues/33)):
  shared deterministic `Lcg` (Knuth MMIX multiplier) for seeded test/bench
  fixtures — `new(seed)` (standard increment), `with_increment(seed, increment)`
  (preserves the wasserstein bench's historical increment `1`), `next_f64`,
  `next_usize`. Replaces the seven drifted inline copies across
  catgraph-magnitude and catgraph-physics; every call-site stream is
  byte-identical (seed prep like `| 1` stays at call sites; a golden-value
  unit test pins the three stream variants via `to_bits`).
