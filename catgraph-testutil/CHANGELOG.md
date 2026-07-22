# Changelog

All notable changes to this crate are documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). This
crate is **unpublished** (`publish = false`, dev-only): it cuts no releases, so
entries accumulate under `[Unreleased]` as a change record rather than a
version history.

## [Unreleased]

### Added

- **Initial crate** ([#33](https://github.com/sustia-llc/catgraph/issues/33)):
  shared deterministic `Lcg` (Knuth MMIX multiplier) for seeded test/bench
  fixtures — `new(seed)` (standard increment), `with_increment(seed, increment)`
  (preserves the wasserstein bench's historical increment `1`), `next_f64`,
  `next_usize`. Replaces the seven drifted inline copies across
  catgraph-magnitude and catgraph-physics; every call-site stream is
  byte-identical (seed prep like `| 1` stays at call sites; a golden-value
  unit test pins the three stream variants via `to_bits`).
