<!-- markdownlint-disable MD024 -->
<!-- MD024 (no-duplicate-heading) disabled: Keep a Changelog intentionally
     reuses `### Added`, `### Changed`, `### Fixed`, etc. across releases. -->
# Changelog

All notable changes to this crate are documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [workspace-v0.24.0] - 2026-09-22

### Changed — BREAKING

- `PropSignature` has `catgraph::CanonicalEncode` as a supertrait; the
  `MatchSite` content fingerprint in `prop::presentation::rewrite` is
  `canonical_fingerprint`. `SfgGenerator<R>: PropSignature` needs
  `R: CanonicalEncode`, so that bound is on `SignalFlowGraph`, `SfgSignature`,
  `copy_n` / `discard_n` / `add_n` / `zero_n`, `sfg_to_mat`, `mat_to_sfg`,
  `sfg_to_colored_expr`, the `graphical_linalg` functions,
  `FaithfulnessReport` and `MatrixNFFunctor`
  ([#469](https://github.com/sustia-llc/catgraph/pull/469)).

### Added

- `CanonicalEncode` for `Marking` (entries sorted by place, `Decimal`
  normalised), `SfgGenerator<R>`, `BoolRig`, `UnitInterval`, `Tropical`,
  `F64Rig`, `Checked<T>`, `Z`
  ([#469](https://github.com/sustia-llc/catgraph/pull/469)).
- `Marking` serde behind `serde`; `Deserialize` rejects an explicit zero token
  count ([#471](https://github.com/sustia-llc/catgraph/pull/471)).

### Changed

- Sections before `workspace-v0.17.0` moved out of this file; the file before
  the move is at tag `v0.23.0`
  ([#462](https://github.com/sustia-llc/catgraph/pull/462)).

## [workspace-v0.23.0] - 2026-09-13

### Added

- `sfg_to_colored_expr` — a `SignalFlowGraph<R>` as the
  `ColoredExpr<SfgGenerator<R>>` the rewrite engine's entry points take; a
  representation change carrying the underlying expression across unchanged
  ([#460](https://github.com/sustia-llc/catgraph/pull/460)).

## [workspace-v0.21.0] - 2026-09-08

### Changed — BREAKING

- `UnitInterval::new` accepts `{0} ∪ [UNIT_INTERVAL_FLOOR, 1]` with
  `UNIT_INTERVAL_FLOOR = 1e-9`, rejecting `0 < value < 1e-9` as
  `RigAxiomViolation`; `UnitInterval::from_rig_value` validates `[0, 1]`
  without the floor, for values produced by rig arithmetic
  ([#451](https://github.com/sustia-llc/catgraph/pull/451)).

## [workspace-v0.20.0] - 2026-09-08

### Changed — BREAKING

- `RewriteRule::new`, `apply_at`, `replay` and the entry points' well-formedness
  screens return `CatgraphError::Rewrite(RewriteRejection)` instead of
  `Presentation`, with new message text; the readback check keeps
  `Presentation` ([#447](https://github.com/sustia-llc/catgraph/pull/447)).

### Added

- `BrauerMorphism::terms` — coefficient, δ power and pairs per term
  ([#446](https://github.com/sustia-llc/catgraph/pull/446)).

### Added — tests

- `content_equality_corpus::interleave_mode_corpus_is_closed_by_content`, the
  `#[ignore]`d #183 interleave tier (745 / 0 / 745 / 745)
  ([#438](https://github.com/sustia-llc/catgraph/pull/438)).
- `non_crossing_parallel_arms_two_through_lines` at Hom(18, 18)
  ([#439](https://github.com/sustia-llc/catgraph/pull/439)).
- `monoidal_tensor` asserts the tensored terms in both operand orders
  ([#446](https://github.com/sustia-llc/catgraph/pull/446)).

### Docs

- `docs/SMC-NF-RECONCILIATION.md` routes the termination proof to #186 and the
  residual-(a) trackers to #57
  ([#438](https://github.com/sustia-llc/catgraph/pull/438)).

## [workspace-v0.19.0] - 2026-09-07

### Added

- `MatR::trace` and `MatKron::trace` (`Option<R>`, `None` off the square);
  `mat_f64::{solve, rank, one_norm, frobenius_norm}` under `f64-rig`;
  `examples/mat_operations.rs` §4 calls each
  ([#425](https://github.com/sustia-llc/catgraph/pull/425)).

### Fixed

- `smc_nf::nf` expands every wide braid of a braid-only layer: `hexagon_expand`
  fires on any layer whose atoms are all `Identity` or `Braid` with at least
  one wide braid, emitting each braid's bricks in source order and the
  residual atoms as one layer; a layer holding several braids no longer
  reaches the fixpoint unexpanded, so the normal form of every expression
  whose lowering holds such a layer changes; `docs/SMC-NF-RECONCILIATION.md`
  §2.2 states the rule ([#435](https://github.com/sustia-llc/catgraph/pull/435)).

### Changed

- `RewriteOutcome` rustdoc states the equal-cost tie rule: the first reached
  is kept ([#432](https://github.com/sustia-llc/catgraph/pull/432)).
- `decorated_cospan.rs` and `temperley_lieb.rs` rustdoc read `==` for `Cospan`
  equality; `tests/common/mod.rs` loses `cospan_eq`
  ([#430](https://github.com/sustia-llc/catgraph/pull/430)).
- `mat_f64` module doc, README and `docs/FS18-AUDIT.md` name the new `mat_f64`
  operations ([#425](https://github.com/sustia-llc/catgraph/pull/425));
  `docs/FS18-AUDIT.md`'s Thm 6.55 row reads the lifted spider-theorem
  exclusions ([#423](https://github.com/sustia-llc/catgraph/pull/423)).
- `tests/braiding_cross_carrier.rs` calls `Span::assert_valid()` without
  parameters ([#424](https://github.com/sustia-llc/catgraph/pull/424)).
- Hand-maintained test counts in `tests/graphical_linalg.rs` and
  `tests/rayon_equivalence.rs` are cut or dated
  ([#431](https://github.com/sustia-llc/catgraph/pull/431)).

### Added — tests

- `temperley_lieb.rs`: `monoidal` on a non-square Brauer factor in both orders
  ([#430](https://github.com/sustia-llc/catgraph/pull/430)).
- `tests/mat.rs` and `tests/mat_f64.rs`: the `trace`, `solve`, `rank` and norm
  pins ([#425](https://github.com/sustia-llc/catgraph/pull/425)).
- `tests/rewrite.rs`: a two-hop return path is not convex, `RewriteStep::rule`,
  one unit of fuel per application, the equal-cost tie, `replay` rejecting an
  out-of-range, repeated or mislabeled assignment
  ([#432](https://github.com/sustia-llc/catgraph/pull/432)).
- `tests/presentation.rs`: rewritten-term pins for SMC rules 1, 4, 6 and 7,
  `eq_mod` undecided when one side alone hits the depth bound,
  `eq_mod_functorial_colored` on a one-word mismatch,
  `PresentedProp::presentation`
  ([#433](https://github.com/sustia-llc/catgraph/pull/433)).
- `tests/congruence_closure.rs`: atom-canonical preference, congruence through
  an atom-free class on either child, re-filing under the second child's root,
  the propagate/refine loop past a merging refinement
  ([#434](https://github.com/sustia-llc/catgraph/pull/434)).
- `tests/smc_nf_regression.rs`: `TestSig::S` (`0 → 0`); two, three and
  mixed wide-braid layers expand, the wide braid on either side of a swap,
  equal adjacent scalars reach a fixpoint. `tests/pass_disjointness_probes.rs`:
  Step 7 transposes a boundary-free block pair through
  `nf_without_column_pass`, a column move spanning two layers, a four-layer
  scalar/copy diagram terminates
  ([#435](https://github.com/sustia-llc/catgraph/pull/435)).

## [workspace-v0.18.0] - 2026-09-05

### Added

- `tests/canonical.rs`: `S` against a basis-vector evaluator on a
  depth-bounded corpus over four rigs, the `compose` / `tensor` /
  `permute_side` squares, the Prop 5.56 round-trip, the Def 5.2 / 5.25
  arities, `compose` on `DecoratedCospan` and `PetriNet` against the partition
  reference, `zero_matrix` on `Tropical`
  ([#414](https://github.com/sustia-llc/catgraph/pull/414)).

### Changed

- `tests/hypergraph_laws.rs` points at `catgraph/tests/canonical.rs`
  ([#410](https://github.com/sustia-llc/catgraph/pull/410)).
- README, `docs/FS18-AUDIT.md` and `tests/enriched.rs` cite test files without
  a count ([#412](https://github.com/sustia-llc/catgraph/pull/412)).
- README gains §Canonical test
  ([#418](https://github.com/sustia-llc/catgraph/pull/418)).

### Removed

- `tests/prop.rs`, `tests/sfg_to_mat.rs`, `tests/mat_to_sfg_roundtrip.rs`,
  `examples/sfg_to_mat.rs` and `examples/petri_net_braiding.rs`, subsumed by
  `tests/canonical.rs` ([#414](https://github.com/sustia-llc/catgraph/pull/414)).

## [workspace-v0.17.0] - 2026-09-03

### Changed — BREAKING

- `PetriNet` carries declared boundary legs: `new` and `new_unchecked` take
  `left` and `right` place-index legs after `transitions` (`new` rejects an
  entry at or beyond `places.len()`); `from_cospan` stores the cospan's legs
  and emits each transition's `pre`/`post` sorted by place index; `parallel`
  concatenates legs with `other`'s shifted, `sequential` keeps `self`'s
  domain leg and remaps `other`'s codomain leg; `Composable::compose` goes
  through the decorated cospan and needs `Lambda: 'static`; `permute_side`
  permutes the named side's leg and is a no-op when `p.len()` is not that
  leg's length. `PetriDecoration`'s apex is `PetriApex { n, transitions }`,
  whose `combine` shifts the second operand's place indices by the first's
  `n` ([#275](https://github.com/sustia-llc/catgraph/issues/275)).

### Changed

- `DecoratedCospan<Lambda, D>`'s `Clone` and `Debug` are hand-written over
  `Lambda: Eq + Copy + Debug, D: Decoration`, matching `PartialEq`; `Debug`
  renders the struct name and the two field names
  ([#348](https://github.com/sustia-llc/catgraph/issues/348)).
- `enriched` module doc cites F&S §2.3 Def 2.46
  ([#407](https://github.com/sustia-llc/catgraph/pull/407)).
- `MatR::permute_side` takes its two `matmul` results with `.expect`, whose
  message names the length guard checked above, instead of silently discarding
  an `Err` ([#298](https://github.com/sustia-llc/catgraph/issues/298)).
- Rustdoc reduced to contract statements; this CHANGELOG rewritten to one
  bullet per change ([#365](https://github.com/sustia-llc/catgraph/issues/365)).
- `smc_nf`: `apply_braid_layer_to_perm` takes a private identity-or-swap layer
  type, losing its `_` match arm; `apply_block_transposition` loses the
  adjacency `debug_assert_eq!`, and it and `blocks_are_adjacent` both go
  through one adjacency-checked `adjacent_runs`; `reorder_zero_arity_columns`
  computes its boundary snapshot only under `debug_assertions`
  ([#303](https://github.com/sustia-llc/catgraph/issues/303)).

### Fixed

- `E1::operadic_substitution` keeps `sub_intervals` sorted for every inner
  arity and slot: images occupy the substituted slot's position, and a nullary
  inner removes the slot in place instead of `swap_remove`
  ([#360](https://github.com/sustia-llc/catgraph/issues/360)).

### Fixed — tests

- `tests/hypergraph_laws.rs`: the eleven Def 2.5 equations and the two snakes
  on `DecoratedCospan` and `PetriNet`, and a Def 2.12 generator partition
  table the six `HypergraphCategory` implementors are compared against;
  `PetriNet` enters `braiding_cross_carrier.rs` on both constructors and on
  `permute_side` ([#275](https://github.com/sustia-llc/catgraph/issues/275)).
- `verify_rig_axioms` call sites for `Z` (`rig.rs`) and, in `catgraph-dl`,
  `Dual` ([#292](https://github.com/sustia-llc/catgraph/issues/292)).
- `MatKron` law pins swept over `n` and over `BoolRig`; the `n = 0` and
  coassociativity doc claims scoped to what is asserted (`mat_kron.rs`)
  ([#278](https://github.com/sustia-llc/catgraph/issues/278)).
- `PerfectMatching::non_crossing` against a boundary-walk planarity decision
  and a Catalan count, `Pair::contains` against `min`/`max` normalisation,
  and `monoidal` against side-by-side placement (`temperley_lieb.rs`)
  ([#294](https://github.com/sustia-llc/catgraph/issues/294)).
- Hand-derived dyadic oracles for `E1` and `E2` `operadic_substitution`
  (`e1_operad.rs`, `e2_operad.rs`)
  ([#295](https://github.com/sustia-llc/catgraph/issues/295)).
- `smc_nf_differential_sweep.rs` records a dated pass of the five
  `--ignored` corpus sweeps and gains
  `braid_mode_smoke_prefix_of_the_published_corpus`
  ([#300](https://github.com/sustia-llc/catgraph/issues/300)).
- `smc_nf`'s Step 7 block-transposition pass:
  `block_transposition_needs_adjacency_in_every_shared_layer` pins a fixture
  where a braid-carrying component separates two free components' runs in 2
  of their 3 shared layers, so `blocks_are_adjacent` must decline the pair — a targeted
  release-mode oracle on the rewritten diagram, replacing reliance on the
  debug-only `reorder_component_blocks` assertion
  ([#373](https://github.com/sustia-llc/catgraph/issues/373)).
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

---

> 📄 **Sections before `workspace-v0.17.0` are archived.** Every section from
> `[workspace-v0.16.0]` down was moved out of this file verbatim on 2026-09-21
> and is held outside this repository. The file as it stood before the move
> is at tag
> [`v0.23.0`](https://github.com/sustia-llc/catgraph/blob/v0.23.0/catgraph-applied/CHANGELOG.md).

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.25.0...HEAD
[workspace-v0.24.0]: https://github.com/sustia-llc/catgraph/compare/v0.23.0...v0.24.0
[workspace-v0.23.0]: https://github.com/sustia-llc/catgraph/compare/v0.22.0...v0.23.0
[workspace-v0.21.0]: https://github.com/sustia-llc/catgraph/compare/v0.20.0...v0.21.0
[workspace-v0.20.0]: https://github.com/sustia-llc/catgraph/compare/v0.19.1...v0.20.0
[workspace-v0.19.0]: https://github.com/sustia-llc/catgraph/compare/v0.18.0...v0.19.0
[workspace-v0.18.0]: https://github.com/sustia-llc/catgraph/compare/v0.17.0...v0.18.0
[workspace-v0.17.0]: https://github.com/sustia-llc/catgraph/compare/v0.16.0...v0.17.0
