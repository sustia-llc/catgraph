<!-- markdownlint-disable MD024 -->
<!-- MD024 (no-duplicate-heading) disabled: Keep a Changelog intentionally
     reuses `### Added`, `### Changed`, `### Fixed`, etc. across releases. -->
# Changelog

All notable changes to this crate are documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Lineage note:** pre-reboot version links below (`catgraph-applied-v0.x`
> tags) point at the private predecessor repo `tsondru/catgraph` and will not
> resolve publicly; they are kept as an honest record of the crate's history.

## [Unreleased]

### Changed

- `MatR::permute_side` takes its two `matmul` results with `.expect`, whose
  message names the length guard checked above, instead of silently discarding
  an `Err` ([#298](https://github.com/sustia-llc/catgraph/issues/298)).
- Rustdoc reduced to contract statements; this CHANGELOG rewritten to one
  bullet per change ([#365](https://github.com/sustia-llc/catgraph/issues/365)).

### Fixed

- `E1::operadic_substitution` keeps `sub_intervals` sorted for every inner
  arity and slot: images occupy the substituted slot's position, and a nullary
  inner removes the slot in place instead of `swap_remove`
  ([#360](https://github.com/sustia-llc/catgraph/issues/360)).

### Fixed — tests

- `mat_to_sfg` round-trip at the Tropical rig zero: `prop_5_56_tropical_rig_zero`
  pins `Tropical::zero()` (`+∞`) at 1×1, 2×2 all-zero and mixed with finite
  entries; `roundtrip_tropical` samples that zero alongside `0.0, 1.0, 2.0, 3.0`
  ([#301](https://github.com/sustia-llc/catgraph/issues/301)).
- `PropExpr` serde: each variant round-trips on its own, `Braid`'s widths under
  an asymmetric `σ_{1,2}`, with wildcard-free matches making a sixth variant a
  compile error in the test file; `sample_term` composes a `Braid` and its
  every-variant claim is read off the term; the `RewriteOutcome` accessor census
  covers `into_best`
  ([#299](https://github.com/sustia-llc/catgraph/issues/299)).
- `MatR::permute_side` on non-square `MatR<F64Rig>`: entry pins for each value
  of `of_codomain` under a 3-cycle, and unchanged-matrix pins where the
  permutation's length matches the opposite side's arity
  ([#298](https://github.com/sustia-llc/catgraph/issues/298)).
- `Presentation` depth-bound contract: `eq_mod` pinned at `Ok(None)` on both
  engines (Structural with `A = A;A` at depth 4; CC at depth 0) next to
  `Some(true)`/`Some(false)` on pairs that converge; `normalize` pinned at
  depths 0, 1 and 2 with `expr` written out; the two `A = B, B = A` tests now
  assert `converged`, `expr` and `steps_taken`. No production change
  ([#297](https://github.com/sustia-llc/catgraph/issues/297)).
- `mat_f64::determinant`: value pins at n = 0, 1, 2, 3, 4 (sign, singular,
  block-diagonal); the rustdoc's "via nalgebra's LU decomposition" claim
  removed, and the same claim in `examples/mat_operations.rs`; CI
  gains an `f64-rig` test + clippy lane
  ([#296](https://github.com/sustia-llc/catgraph/issues/296)).
- `LinearCombination`'s `Mul` parallel arm (above `PARALLEL_MUL_THRESHOLD`,
  32 terms on both operands; production call site `BrauerMorphism::compose`)
  gains value oracles against a nested-loop `HashMap` reference that calls no
  `LinearCombination` arithmetic
  ([#293](https://github.com/sustia-llc/catgraph/issues/293)):
  - `rayon_equivalence::mul_matches_sequential_reference_across_dispatch_states`
    — all four cells of the `self.len() >= 32 && rhs.len() >= 32` truth table
    (16 × 16, 40 × 40, 40 × 16, 16 × 40), each checking its own discriminating
    power first. Basis collisions are measured per case, not assumed:
    256<!--m:mul.16x16.term_pairs--> term pairs give
    97<!--m:mul.16x16.distinct_products--> distinct products with a top
    multiplicity of 6<!--m:mul.16x16.max_multiplicity-->, and
    1600<!--m:mul.40x40.term_pairs--> pairs give
    517<!--m:mul.40x40.distinct_products--> products with a top multiplicity of
    12<!--m:mul.40x40.max_multiplicity--> (each mixed cell:
    287<!--m:mul.40x16.distinct_products--> products). The test fails loudly if
    a case turns out to have no collisions.
  - `rayon_equivalence::mul_on_a_non_commutative_basis_keeps_operand_order` —
    a free-monoid `Word` basis under concatenation at 40 × 40.
  - `rayon_parallel::linear_combination_above_threshold` — its
    `assert_ne!(…, default())` became a full comparison against the same
    reference on the 64 × 64 `LinearCombination<i64, i32>` fixture. The absorbing class is
    127<!--m:mul_absorbing.pairs_at_zero--> of the
    4096<!--m:mul_absorbing.term_pairs--> term pairs, summing to
    2143<!--m:mul_absorbing.coeff_at_zero--> at basis `0`; since every
    coefficient is positive, exactly
    0<!--m:mul_absorbing.zero_coefficient_terms--> product terms carry a zero
    coefficient, so `simplify` is pinned here as the identity on this input and
    this fixture does **not** cover zero-coefficient removal after a collision.

## [workspace-v0.16.0] - 2026-08-24

### Docs

- `docs/FS18-AUDIT.md` Thm 6.55 rows no longer claim core's
  `tests/spider_theorem.rs` asserts structural equality against canonical
  spiders; they state the builder-shape tests plus the theorem pinned over a
  generated corpus, with a `(9 tests)` citation guarded by
  `scripts/check_audit_counts.py`
  ([#288](https://github.com/sustia-llc/catgraph/issues/288)).

### Changed — BREAKING

- `WiringDiagram::add_boundary_node_unconnected` returns
  `Result<(), CatgraphError>` — `ConstructionDuplicatePortName` (carrying the
  leg and position of the existing port) on a duplicate port name, diagram
  untouched on `Err`
  ([#289](https://github.com/sustia-llc/catgraph/issues/289)).
- `WiringDiagram::connect_pair` merges the two ports in every argument order
  (signature unchanged; inherited from `Cospan::connect_pair`) (#289).
- `NamedCospan::assert_valid` / `assert_valid_nohash` take no argument (core
  #289); this crate's calls updated. `wd.inner().cospan().is_left_identity()`
  and its codomain mirror are computed from the legs on every call (#289).

### Changed

- clippy 1.98: `E1::random` pairs `sub_ints` via `as_chunks::<2>()`;
  `tests/mat_mutable_api.rs` zeroes rows via `fill(F64Rig(0.0))`. No behaviour
  change ([#340](https://github.com/sustia-llc/catgraph/issues/340)).

### Added

- `DecoratedCospan` implements `PartialEq` (hand-written, bounded on
  `D: Decoration` only; not `Eq`) (#289).

### Fixed — tests

- The `all_perms` helper duplicated in `tests/braiding_cross_carrier.rs` and
  `tests/prop.rs` moved to `catgraph-testutil` as `all_perms` /
  `all_perm_indices` ([#286](https://github.com/sustia-llc/catgraph/issues/286)).

## [workspace-v0.15.0] - 2026-08-16

### Changed — BREAKING

- `SymmetricMonoidalMorphism::from_permutation` splits into
  `from_permutation_on_domain` / `from_permutation_on_codomain` on `MatR`,
  `MatKron`, `PropExpr`, `DecoratedCospan` and `PetriNet`; the single-sorted
  `MatR`, `MatKron`, `PropExpr` return the same morphism from both
  ([#258](https://github.com/sustia-llc/catgraph/issues/258)).
- `MatR` and `MatKron` return `Err` when `p.len() != types.len()`;
  `DecoratedCospan` and `PetriNet` inherit `Cospan`'s `Err` (#258).
- `DecoratedCospan::permute_side` follows `Cospan`'s new convention. `MatR`,
  `MatKron`, `PropExpr` (codomain `self · P`, domain `Pᵀ · self`) and
  `PetriNet::permute_side` do not move (#258).
- `WiringDiagram::operadic_substitution` passes `p.inv()` to `permute_side`;
  its behaviour is unchanged (#258).

### Fixed

- `PropExpr::from_permutation` rustdoc no longer claims two opposing
  permutation conventions in the workspace (#258).

### Added

- `tests/braiding_cross_carrier.rs`: every permutation at n = 3, 4, both
  constructors, every carrier against a hand-anchored reference;
  `NamedCospan` is asserted to refuse the constructors; `PetriNet`'s braiding
  is lossy (empty boundaries) and is checked as arity/apex only (#258).

## [workspace-v0.14.0] - 2026-08-16

### Changed — BREAKING

- `PetriNet::new` returns `Result<Self, CatgraphError>` (`CatgraphError::PetriNet`
  on an arc referencing a place index ≥ `places.len()`); the old body is
  `PetriNet::new_unchecked`
  ([#256](https://github.com/sustia-llc/catgraph/issues/256)).
- `NamedCospan::new` returns `Result` (core #256); this crate's tests, examples
  and benches gained `.unwrap()`. No change to this crate's API (#256).

### Added

- `FaithfulnessReport::matrix_buckets` (`|S(enumerated)|`);
  `FaithfulnessReport` is `#[non_exhaustive]`
  ([#167](https://github.com/sustia-llc/catgraph/issues/167)).

### Fixed

- `verify_sfg_to_mat_is_full_and_faithful` buckets by
  `(rows, cols, Vec<Vec<R>>)` instead of a `Debug` string, so `F64Rig(-0.0)`
  and `F64Rig(0.0)` share a bucket, and a NaN-imaged expression lands in a
  bucket of one; the depth-2 baselines (748 / 1114 / 1594 / 1590) are unmoved;
  pinned by `signed_zero_is_one_matrix_bucket_not_two` (#167).

## [workspace-v0.13.0] - 2026-08-15

### Added

- `RewriteStep` and `RewriteOutcome<G>` derive `Serialize`/`Deserialize`
  behind the `serde` feature; `replay` re-validates every step;
  `RewriteRule<G>` gains no derive; `initial_cost`, `best_cost`,
  `states_explored`, `fuel_exhausted` and `best` are not validated;
  `RewriteOutcome` carries no start state; neither type is a stable wire
  format (`tests/serde_roundtrip.rs`)
  ([#249](https://github.com/sustia-llc/catgraph/issues/249)).
- `prop::presentation::content::ContentKey<G>` derives
  `Serialize`/`Deserialize` behind `serde`; `Option` colors are written as an
  explicit `Untyped` / `Typed(c)` tag; not a stable wire format
  ([#255](https://github.com/sustia-llc/catgraph/issues/255)).
- The `serde` feature comment in `Cargo.toml` enumerates the full derived
  surface (`ColoredExpr` was missing).

## [workspace-v0.12.0] - 2026-08-15

### Added

- `prop::presentation::rewrite` match-site surface: `match_sites(…, limit)`
  (`limit` truncates, never ranks), `apply_at` (a stale site and a non-convex
  assignment are two distinct `CatgraphError::Presentation`s),
  `MatchSite::{matched_edges, matched_nodes, into_step}` (no serde impl), and
  the expression-level `match_sites_of` / `rewrite_at`, which return `Err` on
  a malformed term, never an empty `Vec`
  ([#250](https://github.com/sustia-llc/catgraph/issues/250)).
- `Presentation::rewrite_depth()` accessor; rebuild a stored presentation via
  `with_depth(depth)` then `set_engine(engine)`.

### Fixed

- `PropExpr::from_permutation` realizes the permutation (was
  `Identity(n)`): one `Identity(t) ⊗ Braid(1, 1) ⊗ Identity(n-t-2)` layer per
  `adjacent_swaps` swap; oracle
  `sfg_to_mat(from_permutation(p)) == MatR::permutation_matrix(&p)` over every
  permutation of n = 3, 4
  ([#252](https://github.com/sustia-llc/catgraph/issues/252)).
- `PropExpr::permute_side` permutes the requested side (was splicing
  `Braid(0, n)`, the identity): codomain postcomposes `from_permutation(p)`,
  domain precomposes `from_permutation(p.inv())`; `O(n²)` braid layers per
  call (#252, arity magnitude stays the caller's obligation per
  [#197](https://github.com/sustia-llc/catgraph/issues/197)).

## [workspace-v0.11.0] - 2026-08-10

### Changed

- `E1::random(cur_arity, rng: &mut impl Rng)` takes any `rand_core 0.10`
  generator (engines must be on the 0.10 line); `[dependencies]` carry
  `rand_core` instead of `rand` (`rand` dev-only workspace-wide, guarded by
  `scripts/check_rand_dev_only.py`); `catgraph_applied::{Rng, TryRng}` are
  re-exported, so rand_core's major version is public API (a 0.11 adoption is
  breaking); `rand 0.10` `RngExt` call sites compile unchanged; the seeded
  draw stream is not bit-identical to before
  ([#239](https://github.com/sustia-llc/catgraph/issues/239)).

## [workspace-v0.10.0] - 2026-08-09

### Fixed

- Browser-wasm lib builds no longer fail in `getrandom`: the workspace `rand`
  entry drops its default features and this crate's lib edge carries none;
  `cargo check --lib -p <crate> --target wasm32-unknown-unknown` passes for
  applied, magnitude, dl and syntax (`--all-targets`/`--tests` still reach
  `getrandom` through `proptest`); downstream lib graphs also shed
  `chacha20`. Any graph containing `catgraph-physics`
  (`rustworkx-core`) re-enables them (CI lane
  [#233](https://github.com/sustia-llc/catgraph/issues/233)); consumers enable
  `thread_rng`/`sys_rng` themselves; browser builds should use
  `--no-default-features` (`parallel` cannot spawn threads there)
  ([#232](https://github.com/sustia-llc/catgraph/issues/232)).

## [workspace-v0.9.0] - 2026-08-04

### Added

- `rig::Zero` and `rig::One`, catgraph-owned, implemented for every primitive
  integer and float ([#219](https://github.com/sustia-llc/catgraph/issues/219)).

### Changed

- `deep_causality_num` dropped from this crate. **BREAKING for downstream
  scalars:** implement `catgraph_applied::rig::{Zero, One}` instead of
  `deep_causality_num::{Zero, One}`. `tests/rig_dc_substrate.rs` renamed
  `tests/rig_identity_substrate.rs` (#219).
- `temperley_lieb` composition connectivity is a `union-find`
  `QuickUnionUf<UnionBySize>` pass; the `ultragraph` dependency is gone;
  `<ExtendedPerfectMatching as Mul>::mul` drops its `.expect()`
  ([#220](https://github.com/sustia-llc/catgraph/issues/220)).

## [workspace-v0.8.0] - 2026-08-03

### Added

- `prop::presentation::rewrite`: `cost_of`, `RewriteRule::new` (rejects
  non-parallel or ill-formed sides, an edge-free lhs, a non-mono left
  interface), `optimize` (best-first over `canonical_key`, convex DPO per
  BGKSZ [arXiv:1602.06771](https://arxiv.org/abs/1602.06771) Thm 5.6,
  fuel-bounded, no termination or confluence claim; the best state's readback
  is re-checked via `ColoredExpr::new` + `content_eq`, `Err` on a lost
  readback), `replay`, `RewriteOutcome`; every entry re-validates its
  `ColoredExpr` inputs on arity and words
  ([#214](https://github.com/sustia-llc/catgraph/issues/214),
  [#57](https://github.com/sustia-llc/catgraph/issues/57) a2).

### Changed

- Internal: `prop::presentation::content` gains a `pub(super)` `from_parts`
  constructor that checks ranges, tentacle counts and colors, monogamy and
  acyclicity (#214).

## [workspace-v0.6.0] - 2026-08-02

### Added

- Interleave-biased third tier of the SMC-NF differential sweep,
  `published_interleave_mode_figures_reproduce` (`internal-probes`,
  `#[ignore]`d, 100 000 pairs, seed `0x94D0_49BB_1331_11EB`), pinned
  745 / 0 / 745 ([#183](https://github.com/sustia-llc/catgraph/issues/183)).

### Changed

- `verify_sfg_to_mat_is_full_and_faithful` partitions each matrix bucket into
  connected components of the `Some(true)` graph (union-find) instead of greedy
  classes; depth-2 baselines BoolRig 952 → 748, UnitInterval 1397 → 1114,
  Tropical 1974 → 1594, F64Rig 1969 → 1590; `Presentation::eq_mod` rustdoc
  records its non-transitivity
  ([#189](https://github.com/sustia-llc/catgraph/issues/189)).

### Removed

- `functor::cc_incompleteness_count::{bool, f64rig}/2` bench groups (≈120 s /
  ≈129 s per call after #189); the `#[ignore]`d `cc_completeness_tracking_*`
  trackers carry the counts (#189).

### Fixed

- Step 6½ column cuts are symmetric: `adjacent_column_cuts` →
  `adjacent_column_cuts_at`, both columns maximal local runs, widening scans
  leftmost-first via `cuts_meet` (pinned by
  `column_widening_picks_the_interval_aligned_adjacency`); ablation table
  5 → 7 (`column_pass_decides_exactly_the_seven_documented_witnesses`); new
  probe `split_presence_both_readings_pair_is_newly_seeded`; F1 witness renamed
  `split_presence_nesting_converges_with_free_writing` (`assert_eq!`).
  Re-pins: sweep 253 / 128 / 23 → 183 / 93 / 23 (default), 1162 / 634 / 237 →
  1153 / 630 / 237 (braid), smoke prefix 16 → 14; `content_equality_corpus`
  183/183 and 1153/1153; `canonical_display_corpus` churn 5 334 → 4 585 and
  `layer_pinned_agree` 39 991 → 40 075 (default), 19 098 → 18 977 and
  32 270 → 32 287 (braid); CC pins unmoved. §4.4's canonicality corollary on
  `𝔉′` is unconditional (truncation lemma)
  ([#185](https://github.com/sustia-llc/catgraph/issues/185)).
- Wire-count sums saturate at `usize::MAX` in `prop::colored::check` /
  `infer` and `PropExpr::source` / `target`, reported as
  `CompositionSizeMismatch`
  ([#180](https://github.com/sustia-llc/catgraph/issues/180)).
- The deeper passes reject an overflowing arity: new
  `PropExpr::checked_arities` / `arities_fit`; `content::is_arity_well_formed`
  is `false` on an overflowing `Braid`/`Tensor`; `smc_nf::nf` documents
  `# Panics`; `Presentation::add_equation` reports `CompositionSizeMismatch`
  on a `usize::MAX` LHS arity; `sfg_to_mat` reports `SfgFunctor` on an
  overflowing braid width; `Presentation::eq_mod` screens the overflow class
  ahead of either engine — `Ok(Some(true))` on identical trees, `Ok(None)`
  otherwise — and `ColoredExpr::eq_colored` falls back to structural equality
  there (both previously panicked). A literal huge arity stays the
  caller's obligation (#197)
  ([#196](https://github.com/sustia-llc/catgraph/issues/196)).

## [workspace-v0.5.0] - 2026-07-30

### Changed

- **BREAKING (behavioral):** under `NormalizeEngine::CongruenceClosure`,
  `Presentation::eq_mod`'s SMC layer is decided by
  `content_eq(content_of(a), content_of(b))` instead of `nf(a) == nf(b)`,
  gated on `content::is_arity_well_formed`; `ColoredExpr::eq_colored` decides
  colored SMC-equality via `content_of_colored`; `NormalizeEngine::Structural`
  untouched. CC baselines BoolRig 980 → 952, UnitInterval 1433 → 1397,
  Tropical 2018 → 1974, F64Rig 2013 → 1969 (#57 a1 PR2;
  [#173](https://github.com/sustia-llc/catgraph/issues/173) stays open).
- **BREAKING:** `Presentation::add_equation` checks boundary-word equality
  over an inferred common source word; rejection is
  `CatgraphError::CompositionSizeMismatch` (lengths) or
  `CatgraphError::Composition` (colors) instead of
  `CatgraphError::Presentation`; hand-built ill-composed trees are rejected
  even at `Color = ()`; `Deserialize` still does not re-run the check
  ([#79](https://github.com/sustia-llc/catgraph/issues/79) P2).
- **BREAKING:** `PropSignature` gains `type Color: Clone + Eq + Hash + Debug`
  and required `source_word` / `target_word` (`Cow<'_, [Self::Color]>`); `source()` /
  `target()` are provided; supertraits gain `Ord`; `mono_word` helper;
  `SfgGenerator<R>` requires `R: Ord`; `UnitInterval`, `Tropical`, `F64Rig`
  gain `-0.0`-normalized `total_cmp` total orders; `Checked<T>` derives
  `PartialOrd + Ord` (#79 P1).
- SMC NF residual (b) closed: closed↔closed blocks sort by the lexicographic
  in-situ reading; Step 6 sorts equal-class `0→0` scalars by generator order;
  witness renamed `closed_closed_order_is_ord_less_residual` →
  `closed_blocks_sort_by_content_key` (#79 P1,
  [#174](https://github.com/sustia-llc/catgraph/issues/174)).
- SMC NF Step 7 `reorder_component_blocks` orders wire-disjoint component
  blocks closed < input-anchored < output-only
  ([#55](https://github.com/sustia-llc/catgraph/issues/55) PR2). The
  component-anchored `η` slot derivation from the same PR was retired within
  this release (#174).
- `smc_canonicality_probes` test module — the canonicality gate of record;
  scalar centrality is an NF theorem. CC depth-2 baselines re-pinned BoolRig
  972 → 979, UnitInterval 1400 → 1432, Tropical 1930 → 2017, F64Rig
  1925 → 2012; `scripts/check_audit_counts.py` scans prose pin sites against
  `BASELINE_*_D2` (#55 PR2, #173).
- SMC NF Step 6 `reorder_tied_zero_arity`: strictly-commuting adjacent atoms
  sort `scalar (0→0) < η < ε < solid`; termination measure gains
  `tied_inversion_count`; CC depth-2 baselines re-pinned BoolRig 1142 → 972,
  UnitInterval 1634 → 1400, Tropical 2234 → 1930, F64Rig 2229 → 1925
  (#55 PR1).

### Added

- `prop::presentation::display`: `canonical_display(e) =
  nf(expr_of_content(content_of(e)))` and `expr_of_content`; `nf` untouched;
  measured in `tests/canonical_display_corpus.rs` (`internal-probes`); unit
  witness `display::tests::a_layer_pinned_eta_can_still_take_two_layers`
  ([#187](https://github.com/sustia-llc/catgraph/issues/187) PR1).
- Display-convergence witnesses in `tests/smc_nf_completeness.rs`
  (`display_converges_on_the_eta_slack_writings`,
  `nf_separates_where_the_display_converges`,
  `display_converges_on_both_beyond_eta_witnesses`); the `nf`-level
  `assert_ne!` witnesses keep their names; `SMC-NF-RECONCILIATION.md`
  §4.4 / §4.6 / §4.7 notes dated 2026-07-30 (#187 PR2).
- `prop::presentation::content::is_arity_well_formed`, public.
- `prop::presentation::content`: `content_of`, `content_eq`, `canonical_key`,
  `content_of_colored` — the `SMC-NF-RECONCILIATION.md` §4.1 content function;
  `tests/content_equality_corpus.rs` scores 253/253 (default) and
  1 162/1 162 (braid) (#57 a1 PR1).
- `ColoredCompleteFunctor` + `Presentation::eq_mod_functorial_colored`
  (parallelism checked before images are compared) (#79 P3a).
- `prop::colored`: `check(expr, input_word) -> Result<target_word>` and
  `ColoredExpr<G>` with `eq_colored`; serde derives behind `serde`
  (`Deserialize` does not re-run `check`) (#79 P1).
- `SMC-NF-RECONCILIATION.md` §4: abstract content, Lemma 4.1 (content decides
  SMC-equality), Lemma 4.2 (`nf` preserves content); §1 gains three invariant
  clauses (#55 proof phase).
- `Checked<T>` poison-on-overflow rig wrapper (`Value(T) | Poison`; `⊥` fully
  absorbing, `⊥ × 0 = ⊥`; `is_poisoned()`; `verify_rig_axioms` fails with
  exactly `"absorbing zero"` on a poisoned sample; `Display`/`FromStr` read
  `⊥` as one atom; no serde)
  ([#88](https://github.com/sustia-llc/catgraph/issues/88)).
- `CheckedOps` trait (`checked_add` / `checked_mul`) for the twelve primitive
  integer types (#88).
- SMC NF Step 6½ `reorder_zero_arity_columns`: interval-aligned column
  transposition between Step 6 and Step 7; termination measure gains
  `column_inversion_count` (#174).
- `internal-probes` feature (opt-in, test-only, not public API):
  `smc_nf::nf_without_column_pass`, `smc_nf::fragment_status`;
  `tests/smc_nf_differential_sweep.rs` (seed `0x9E37_79B9_7F4A_7C15`) pins
  253 / 128 / 23 (default) and 1162 / 634 / 237 (braid); CI runs the
  5 000-pair smoke tier (#174).

### Fixed

- `SMC-NF-RECONCILIATION.md` (2026-07-29): §4.1 / §4.7 status notes; the
  "Lafont proves termination for the bialgebra structure" claim refuted
  (Lafont states a conjecture; nearest proof BGKSZ Thm 6.1).
- SMC NF Theorem 4.5 on `𝔉′` (§4.4): Lemma 4.3 column pinning, Lemma 4.4
  layout freedom, two induction steps flagged open
  (`layer_pinned_eta_sits_below_layer_zero`); rigidity on the original `𝔉`
  withdrawn (all 253 divergences are `η` placement slack); pass-disjointness
  obligation resolved FALSE-as-stated and the §1 clauses restated with the
  both-readings carve (`tests/pass_disjointness_probes.rs`); §4.5 Path 1
  refuted; §4.6 case-7079 exemplar retracted; `eq_colored` / `smc_nf`
  canonicality claims re-scoped to `𝔉′` (#174 PR-B).
- SMC NF residuals (c) and (d) closed (column pass; (d)'s source form by
  retiring Step 6's tied comparator branch); witnesses un-ignored and renamed
  `trapped_closed_block_extracts`,
  `nested_sink_block_converges_with_free_writing`,
  `nested_source_block_converges_with_free_writing` (#174).
- Free decision sites no longer read component order: `component_slot`
  deleted, `tie_sorts_before`'s rule-(i) branch removed, Guard 3 no longer
  gates the sift; sweep divergences 1311 → 253 (in-`𝔉` 192 → 128, marked
  888 → 23); CE-R1 / CE-R2 witnesses (#174).
- Depth-2 pins re-baselined +1: BoolRig 979 → 980, UnitInterval 1432 → 1433,
  Tropical 2017 → 2018, F64Rig 2012 → 2013 (#174).

### Documentation

- `rig` module docs carry the workspace overflow policy (per-rig-family
  matrix; saturating arithmetic rejected); README design entry (#88).

## [workspace-v0.4.0] - 2026-07-25

### Changed

- E1 test RNG seeds are documented `const SEED: u64` values with `SEED + 1`
  offsets ([#141](https://github.com/sustia-llc/catgraph/issues/141)
  follow-up, PR #146).
- `LinearCombination::linear_combine` gains par-vs-seq equivalence coverage in
  `tests/rayon_equivalence.rs`
  ([#48](https://github.com/sustia-llc/catgraph/issues/48)).
- `pub(crate)` `crate::prop::adjacent_swaps` bubble-sort core shared by
  `mat_to_sfg`'s `permutation_sfg` and `smc_nf`'s `decompose_braid` /
  `canonicalize_run` ([#138](https://github.com/sustia-llc/catgraph/issues/138)).
- `functor_bench`: `cc_incompleteness_count::bool/3` dropped (one `d=3` call
  exceeds 590 s); `bool/2` and `f64rig/2` run at `sample_size(10)`
  ([#59](https://github.com/sustia-llc/catgraph/issues/59)).
- Every Selinger (cited as arXiv:0908.3347), JS-I, JS-II and JS-Braided
  anchor in `docs/SMC-NF-RECONCILIATION.md` verified; all (†)/(‡) marks
  retired; Selinger Thm 3.12 is p. 18, not p. 17 (doc +
  `tests/smc_nf_regression.rs`); JS-I's two "Theorem 1.2" headings noted;
  the JS-Braided precursor report (Macquarie 860081) cached
  ([#117](https://github.com/sustia-llc/catgraph/issues/117)).
- `docs/FS18-AUDIT.md` §5.2 gains Ex 5.7 (Corel) and Ex 5.8 (Rel), both IN
  CORE; summary `[27,3,3,12,16] of 61 → [27,3,3,12,18] of 63` (audit Phase 7).
- Thm 5.60 presentation completed to E_18: D7 scalar addition and D8 zero
  scalar added; `E_17 → E_18` renamed workspace-wide; depth-2 baselines
  BoolRig 1301 → 1142, UnitInterval 1856 → 1634, Tropical 2526 → 2234, F64Rig
  `2770..=2790` → `2468..=2488`; `prop_presentation_nf` BoolRig expansion count
  23 → 28 ([#114](https://github.com/sustia-llc/catgraph/issues/114)).
- FS18-AUDIT summary recount: TOTAL 26/2/2/15/15 of 60 → 27/3/3/12/16 of 61;
  citation fixes (F&S 2019 §2.6 → §2.3 / §3.1, BTV 2021 §1.4 → §5,
  `Ring + ZAlgebra` bound) (paper-audit Phase 2).
- #15 resolved: `Presentation::eq_mod_functorial` with `MatrixNFFunctor` is
  the terminal decision procedure for Mat(R); KB completion demoted to a spike
  (#57); `cc_completeness_tracking_*` re-baselined BoolRig 1301, UnitInterval
  1856, Tropical 2526 exact, F64 jitter band `2770..=2790` (#58).

### Fixed

- `E1::random` resamples until adjacent sorted coordinates are separated by
  more than `2·F32_EPSILON`, so construction is infallible; signature
  generalized from `&mut ThreadRng` to `&mut impl RngExt` (#141).
- Signed-zero `Eq`/`Hash` contract on `UnitInterval`, `Tropical`, `F64Rig`:
  `Hash` normalizes `-0.0` to `0.0`; the F64Rig depth-2 CC diagnostic
  re-baselined from `2468..=2488` to an exact `2229`
  ([#58](https://github.com/sustia-llc/catgraph/issues/58)).
- `try_unitor_merge` prepends a zero-source `X`; `hexagon_expand` decomposes
  wide braids in identity-padded layers; the braid+generator merge guard moved
  to `reduce_involution`; the #14 interchange proptest is un-ignored;
  mid-layer zero-source (η) scheduling stays an ignored known-gap test (#14).

### Added

- `mat_to_sfg` — FS18 Prop 5.56 realization; `sfg_to_mat(mat_to_sfg(M)) == M`
  in `tests/mat_to_sfg_roundtrip.rs`; FS18-AUDIT §5.3 `6/1 → 7/0`
  ([#126](https://github.com/sustia-llc/catgraph/issues/126)).
- `add_n` / `zero_n` SFG helpers (#126).
- Optional `serde` feature: `Serialize`/`Deserialize` on `PropExpr<G>`,
  `Presentation<G>`, `PresentedProp<G>`, `NormalizeEngine`,
  `NormalizeResult<G>`, `SfgGenerator<R>`; off by default; `Presentation`
  deserialization does not re-run `add_equation`'s check; CI
  `--features serde` lane ([#81](https://github.com/sustia-llc/catgraph/issues/81)).
- SMC normal form: `topological_layer_order` (Step 4(c)); mixed-layer braid
  isolation in `collect_braid_prefix`; identity-width-refined naturality
  sweep (#14).

> Reconciliation note ([#158](https://github.com/sustia-llc/catgraph/issues/158)):
> workspace tags `v0.1.1`, `v0.2.0`, `v0.2.1`, `v0.3.0` have no per-crate
> sections here; see `git log v0.1.0..v0.3.0 -- catgraph-applied/`.

## [workspace-v0.1.0] - 2026-07-01

### Added

- `hypergraph` module: `Hypergraph<V, HE>` CRUD container replacing
  yamafaktory `hypergraph` v4.2.0 for koalisi — never-reused monotonic
  `VertexIndex` / `HyperedgeIndex`, ordered hyperedges with duplicates, no-op
  updates return `Ok`, infallible clears, bounds `Copy + Eq + Debug`, no
  serde, `Copy` weights returned by value; `add_hyperedge` is idempotent and
  returns the smallest matching index; `remove_vertex` cascades;
  `reverse_hyperedge`, `join_hyperedges` (keeps the first edge's weight),
  `contract_hyperedge_vertices` (collapses adjacent `target` runs),
  `hyperedge_vertices`, counts and sorted iteration accessors; and
  `hyperedge_as_cospan(idx)` (identity cospan over the member list);
  re-exported at the crate root as
  `catgraph_applied::{Hypergraph, HypergraphError, VertexIndex, HyperedgeIndex}`;
  `examples/agent_hypergraph.rs` (#23).

## [0.6.0] - 2026-05-13

Co-released with catgraph-magnitude v0.5.0 at workspace v0.14.0.

### Added

- `tests/zalgebra_axioms.rs` — `from_i64` ring-homomorphism axioms.
- Crate-root re-export `pub use integer::ZAlgebra` (`catgraph_applied::ZAlgebra`).

### Changed (BREAKING)

- `Integer` trait renamed `ZAlgebra` (`catgraph_applied::ZAlgebra` or
  `catgraph_applied::integer::ZAlgebra`).
- `ZAlgebra` is sealed via `private::Sealed`; `Z(BigInt)` is the only
  implementation.

## [0.5.6] - 2026-05-13

Co-released with catgraph-magnitude v0.4.0 and catgraph v0.13.0 at workspace
v0.13.8.

### Added

- `Integer` trait: `Rig + Neg + Sub` with a `from_i64` lifting constructor.
- `Z(BigInt)` newtype, `Integer + Ring`.
- `rustworkx` feature flag (default-on) gating `rustworkx-core`;
  `--no-default-features` removes the `temperley_lieb` module.

## [0.5.5] - 2026-05-10

Substrate release for catgraph-magnitude v0.3.0 (workspace v0.13.3).

### Added

- Mutable `MatR<Q>` API: `row_swap`, `scale_row`, `add_scaled_row`,
  `col_swap`, `scale_col`, `add_scaled_col`, `entries_mut`, `entry_mut`.
- `LawvereMetricSpace::size()` and `LawvereMetricSpace::objects()`.
- `LawvereMetricSpace::<usize>::from_distance_fn(n, f)`.
- `impl From<i64> for F64Rig`.

### Changed

- rustdoc: links to the private `PARALLEL_MUL_THRESHOLD` replaced by the
  literal (32); redundant explicit link targets on `MonoidalMorphism`
  (`temperley_lieb.rs`) and `EnrichedCategory::objects` (`lawvere_metric.rs`)
  removed.

## [0.5.4] - 2026-04-28

Co-released with catgraph v0.12.2 and catgraph-magnitude v0.1.1.

### Added

- `LawvereMetricSpace::from_distances` (last-write-wins on duplicate keys).
- `EnrichedCategory::hom` for `LawvereMetricSpace<T>` returns
  `Tropical::one()` on an unset diagonal.
- `tests/decorated_cospan.rs`
  `t2_3_decorated_cospan_pushforward_through_quotient`; the previous
  `t2_3_petri_decoration_*` test renamed `t2_4_*`.
- `tests/wiring_diagram::operadic_with_clone_only_intercircle`.

### Changed

- `Operadic for WiringDiagram` bound `InterCircle: Eq + Copy + Send + Sync`
  loosened to `Eq + Clone + Send + Sync` (rides catgraph v0.12.2).

## [0.5.3] - 2026-04-25

### Added

- `Neg`, `Sub`, `Div`, `From<f64>` on `F64Rig` (for catgraph-magnitude
  v0.1.0's `mobius_function::<F64Rig>`).

## [0.5.2] - 2026-04-24

### Added

- `src/prop/presentation/smc_nf.rs` — Joyal-Street string-diagram normal
  form: `smc_nf::nf(e) -> StringDiagram<G>`, `from_string_diagram`;
  `tests/smc_nf_regression.rs`, `tests/smc_nf_completeness.rs`.
- `src/prop/presentation/functorial.rs` — `CompleteFunctor<G>` +
  `MatrixNFFunctor<R>`.
- `Presentation::eq_mod_functorial<F: CompleteFunctor<G>>`; always
  `Ok(Some(_))`.
- `kb::CongruenceClosure` atom-canonical refinement (`propagate_fixpoint` +
  `smc_refine`, `SAFETY_BOUND = 64`); BoolRig d=2 collisions 2574 → 1433.
- `tests/functorial.rs`.

### Changed

- `Presentation::eq_mod` (CC branch) short-circuits on
  `smc_nf::nf(a) == smc_nf::nf(b)`.
- The 12 `thm_5_60_faithful_*` tests renamed `cc_completeness_tracking_*`;
  still `#[ignore]`d.

### Fixed

- `install_function_node` re-canonicalizes the signature-table key via
  `find` after a merge.
- Docstrings: "9 fixed SMC-canonical-form rules"; `triangle_inequality_holds`
  comment; `from_string_diagram` `# Panics`.
- `smc_nf_completeness::compose_associator` `max_global_rejects`
  1024 → 16 384.

## [0.5.1] - 2026-04-22

### Added

- `src/prop/presentation/kb.rs` — congruence-closure decision procedure
  (Downey-Sethi-Tarjan 1980); `tests/kb.rs`.
- `Presentation::with_engine(NormalizeEngine)` / `set_engine`, for `eq_mod`
  only: `NormalizeEngine::Structural` (may return `None`) and
  `NormalizeEngine::CongruenceClosure` (default).
- SMC Rule 9: `Identity(m) ⊗ Identity(n) → Identity(m+n)`.
- `src/enriched.rs` — `EnrichedCategory<V: Rig>` + `HomMap<O, V>`.
- `src/lawvere_metric.rs` — `LawvereMetricSpace<T>` over `Tropical`;
  `EnrichedCategory<Tropical>` impl.

### Changed

- **BREAKING:** `Presentation::normalize` returns
  `Result<NormalizeResult<G>, CatgraphError>` (`.expr`, `.converged`,
  `.steps_taken`).
- **BREAKING:** `Presentation::eq_mod` returns
  `Result<Option<bool>, CatgraphError>` (`None` = depth bound hit).
- **BREAKING:** `PropSignature` requires `Eq + Hash`.
- **BREAKING:** `UnitInterval`, `Tropical`, `F64Rig` gain manual `Eq + Hash`
  via `f64::to_bits()`.

### Fixed

- `verify_sfg_to_mat_is_full_and_faithful` routes through
  `Presentation::eq_mod`.

## [0.5.0] - 2026-04-21

### Added

- `src/rig.rs` — `Rig` (F&S Def 5.36) blanket over
  `num_traits::{Zero, One}` + `Add` + `Mul`; `BoolRig`, `UnitInterval`,
  `Tropical`, `F64Rig`; `BaseChange<UnitInterval>` for `Tropical`;
  `verify_rig_axioms` (`CatgraphError::RigAxiomViolation`).
- `src/prop/presentation.rs` — `Presentation<G>` (Def 5.33): `add_equation`,
  `normalize`, `eq_mod`, `with_depth`; 8-rule SMC canonical form; default
  depth 32.
- `src/sfg.rs` — `SignalFlowGraph<R>` (Def 5.45); `copy_n` / `discard_n`.
- `src/mat.rs` — `MatR<R>` (Def 5.50; `m → n` is `m × n`); `Composable`,
  `Monoidal`, `SymmetricMonoidalMorphism`, `block_diagonal`.
- `src/sfg_to_mat.rs` — `sfg_to_mat` functor `S: SFG_R → Mat(R)` (Thm 5.53).
- `src/graphical_linalg.rs` — `matr_presentation<R>` (16 Thm 5.60 equations);
  `verify_sfg_to_mat_is_full_and_faithful<R>`.
- `src/mat_f64.rs` (feature `f64-rig`) — `mat_to_nalgebra` /
  `mat_from_nalgebra`, `determinant`, `try_inverse`.
- Examples `rig_showcase`, `sfg_to_mat`.

### Changed

- `src/prop.rs` → `src/prop/mod.rs`.
- `PropSignature: Eq` relaxed to `PartialEq`.
- catgraph dep bumped to v0.12.0.

### Features

- `f64-rig` (opt-in, off by default) — enables `mat_f64` and a transitive
  `nalgebra` dep.

### Known limitations

- The 12 `thm_5_60_faithful_*` tests in `tests/graphical_linalg.rs` are
  `#[ignore]`d; FS18-AUDIT §5.4 Thm 5.60 is PARTIAL.

## [0.4.0] - 2026-04-20

### Added

- `prop` module (Def 5.2, 5.25): `PropSignature`, `PropExpr<G>`,
  `Free::{identity, braid, generator, compose, tensor}`; `Composable<Vec<()>>`,
  `HasIdentity<Vec<()>>`, `Monoidal`, `SymmetricMonoidalMorphism<()>`;
  equality is structural.
- `operad_algebra` module (Def 6.99): `OperadAlgebra<O, Input>`,
  `CircAlgebra` (Ex 6.100), `check_substitution_preserved`.
- `operad_functor` module (Rough Def 6.98): `OperadFunctor<O1, O2, Input>`,
  `E1ToE2` with `check_substitution_preserved`.
- Accessors `E1::arity`, `E1::sub_intervals`, `E2::arity_of`,
  `E2::sub_circles`; `Clone` on `E1` and `E2<Name: Clone>`.
- Examples `free_prop`, `operad_algebra_circ`, `operad_functor_e1_to_e2`;
  tests `prop.rs`, `operad_algebra.rs`, `operad_functor.rs`.

### Requires

- catgraph v0.11.4.

## [0.3.3] - 2026-04-19

### Added

- `[features] default = ["parallel"]`, `parallel = ["dep:rayon",
  "dep:rayon-cond", "catgraph/parallel"]`; compiles for
  `wasm32-wasip1-threads` and `wasm32-wasip1 --no-default-features`.
- `examples/wasi_smoke_applied.rs`.

### Changed

- `rayon` and `rayon-cond` are optional behind `parallel`; the `catgraph` dep
  is `default-features = false`.
- `linear_combination::{Mul::mul, linear_combine}` and
  `BrauerMorphism::non_crossing` `CondIterator` paths are gated
  `#[cfg(feature = "parallel")]` with a plain-iterator fallback.
- `tests/rayon_equivalence.rs`'s three direct `CondIterator` tests are gated
  behind `parallel`.

## [0.3.2] - 2026-04-19

### Changed

- `linear_combination::{Mul::mul, linear_combine}` and
  `BrauerMorphism::non_crossing` use `rayon_cond::CondIterator`;
  `PARALLEL_MUL_THRESHOLD = 32` and `PARALLEL_COMBINATIONS_THRESHOLD = 8`
  unchanged.

### Added

- `rayon-cond = "0.4"` as a direct dependency.
- `tests/rayon_equivalence.rs` exercises both `CondIterator` arms.

## [0.3.1] - 2026-04-18

### Added

- `DecoratedCospan::compose` invokes `D::pushforward` through the pushout
  quotient (F&S Def 6.75 / Thm 6.77; requires catgraph v0.11.3's
  `Cospan::compose_with_quotient`).
- Direct `PetriNet::permute_side` over the transition sequence.
- `Transition::relabel` arc dedup with summed `Decimal` multiplicities.
- `examples/petri_net_braiding.rs`; `tests/decorated_cospan.rs`;
  `tests/petri_net.rs` (+8).

### Changed

- `examples/decorated_cospan_circuit.rs` extended with series composition;
  its `NOTE:` caveat block removed.
- FS18-AUDIT Ex 6.79–6.86 row PARTIAL → DONE.

## [0.3.0] - 2026-04-17

### Added

- `DecoratedCospan<Lambda, D>` + `Decoration` trait (F&S Def 6.75, Thm 6.77).
- `PetriDecoration<Lambda>`.
- `HypergraphCategory<Lambda>` for `DecoratedCospan<Lambda, D>` and
  `PetriNet<Lambda>`.
- `examples/decorated_cospan_circuit.rs`; `Trivial` decoration.

### Known limitations (closed in 0.3.1)

- `DecoratedCospan::compose` did not invoke `D::pushforward`;
  `PetriNet::permute_side` discarded leg permutations; `Transition::relabel`
  produced duplicate arcs.

## [0.2.0] - 2026-04-17

### Added

- `docs/FS18-AUDIT.md` (56 items, Chapters 4–6); cross-reconciliation with
  `catgraph/docs/FS19-AUDIT.md`.

## [0.1.0] - 2026-04-14

### Added

- Initial release: `linear_combination`, `wiring_diagram`, `petri_net`,
  `temperley_lieb`, `e1_operad`, `e2_operad` extracted from `catgraph` core;
  Criterion bench `rayon_thresholds`.

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.16.0...HEAD
[workspace-v0.16.0]: https://github.com/sustia-llc/catgraph/compare/v0.15.0...v0.16.0
[workspace-v0.15.0]: https://github.com/sustia-llc/catgraph/compare/v0.14.0...v0.15.0
[workspace-v0.14.0]: https://github.com/sustia-llc/catgraph/compare/v0.13.0...v0.14.0
[workspace-v0.13.0]: https://github.com/sustia-llc/catgraph/compare/v0.12.0...v0.13.0
[workspace-v0.12.0]: https://github.com/sustia-llc/catgraph/compare/v0.11.0...v0.12.0
[workspace-v0.11.0]: https://github.com/sustia-llc/catgraph/compare/v0.10.0...v0.11.0
[workspace-v0.10.0]: https://github.com/sustia-llc/catgraph/compare/v0.9.0...v0.10.0
[workspace-v0.9.0]: https://github.com/sustia-llc/catgraph/compare/v0.8.0...v0.9.0
[workspace-v0.8.0]: https://github.com/sustia-llc/catgraph/compare/v0.7.0...v0.8.0
[workspace-v0.6.0]: https://github.com/sustia-llc/catgraph/compare/v0.5.0...v0.6.0
[workspace-v0.5.0]: https://github.com/sustia-llc/catgraph/compare/v0.4.0...v0.5.0
[workspace-v0.4.0]: https://github.com/sustia-llc/catgraph/compare/v0.1.0...v0.4.0
[workspace-v0.1.0]: https://github.com/sustia-llc/catgraph/releases/tag/v0.1.0
[0.6.0]: https://github.com/tsondru/catgraph/compare/catgraph-applied-v0.5.6...catgraph-applied-v0.6.0
[0.5.6]: https://github.com/tsondru/catgraph/compare/catgraph-applied-v0.5.5...catgraph-applied-v0.5.6
[0.5.5]: https://github.com/tsondru/catgraph/compare/catgraph-applied-v0.5.4...catgraph-applied-v0.5.5
[0.5.4]: https://github.com/tsondru/catgraph/compare/catgraph-applied-v0.5.3...catgraph-applied-v0.5.4
[0.5.3]: https://github.com/tsondru/catgraph/compare/catgraph-applied-v0.5.2...catgraph-applied-v0.5.3
[0.5.2]: https://github.com/tsondru/catgraph/compare/catgraph-applied-v0.5.1...catgraph-applied-v0.5.2
[0.5.1]: https://github.com/tsondru/catgraph/compare/catgraph-applied-v0.5.0...catgraph-applied-v0.5.1
[0.5.0]: https://github.com/tsondru/catgraph/compare/catgraph-applied-v0.4.0...catgraph-applied-v0.5.0
[0.4.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-applied-v0.4.0
[0.3.3]: https://github.com/tsondru/catgraph/releases/tag/catgraph-applied-v0.3.3
[0.3.2]: https://github.com/tsondru/catgraph/releases/tag/catgraph-applied-v0.3.2
[0.3.1]: https://github.com/tsondru/catgraph/releases/tag/catgraph-applied-v0.3.1
[0.3.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-applied-v0.3.0
[0.2.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-applied-v0.2.0
[0.1.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-applied-v0.1.0
