# Changelog

All notable changes to this crate are documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). This
crate is **unpublished** (`publish = false`, dev-only): it cuts no releases, so
entries accumulate under `[Unreleased]` as a change record rather than a
version history.

## [Unreleased]

### Added

- **`all_perms` + `all_perm_indices`**
  ([#286](https://github.com/sustia-llc/catgraph/issues/286)): exhaustive `Sₙ`
  enumeration by prefix swaps, the generator every #258 braiding sweep runs on.
  It existed as two private copies on `main` — `catgraph-applied`'s
  `tests/braiding_cross_carrier.rs` and `tests/prop.rs` — and #286's new core
  sweeps needed it too; rather than write it a third and fourth time it moved
  here, which is exactly the duplication class #33 opened this crate for.

  Two entry points because the call sites want different things:
  `all_perm_indices` yields raw `Vec<usize>` one-line notations, which sort and
  dedup (`permutations::Permutation` is neither `Ord` nor `Hash`) so a caller can
  pin distinctness of the enumeration itself; `all_perms` yields `Permutation`
  values ready to feed a constructor. This crate's own tests pin `n!` and
  distinctness for `n ≤ 5`, that every entry is a bijection of `0..n`, and that
  the two views are **index-aligned** — `all_perms(n)[k]` is the permutation
  whose one-line notation is `all_perm_indices(n)[k]`, asserted through
  `all_perms` itself. Alignment is the only contract a caller mixing the views
  depends on; a drift in `all_perms` cannot flip a consuming sweep's convention
  (each sweep derives its reference from the same `p` it feeds the constructor,
  and `Sₙ` is closed under inversion), so this pin is the only place such a
  drift is visible — falsified by mapping `all_perms` through `.inv()`: RED at
  `all_perms(3)[3]`, `[2, 0, 1]` where `all_perm_indices(3)[3]` is `[1, 2, 0]`
  (16 passed, 1 failed), with all 66 test binaries of
  `cargo test -p catgraph -p catgraph-applied --tests` staying green.
- **`permutations` dependency**
  ([#286](https://github.com/sustia-llc/catgraph/issues/286)) — required by
  `all_perms` above, and free on the same grounds as `proptest` below: dev-only
  and unpublished, never in a published crate's `[dependencies]` (#33). This
  crate has no `catgraph` edge and must not grow one — its dedup targets are its
  dev-dependants.
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
