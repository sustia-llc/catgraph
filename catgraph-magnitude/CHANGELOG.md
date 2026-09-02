# Changelog

All notable changes to `catgraph-magnitude` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Lineage note:** pre-reboot version links below (`catgraph-magnitude-v0.x`
> tags) point at the private predecessor repo `tsondru/catgraph` and will not
> resolve publicly. In-tree paper PDFs mentioned in historical entries were
> removed from the tree on 2026-07-10 (arXiv licensing); fetch papers from the
> arXiv links in `docs/`.

## [Unreleased]

## [workspace-v0.16.0] - 2026-08-24

### Changed

- `smith_from_diagonal_raw` uses `blocks.as_chunks::<2>().0` instead of
  `blocks.chunks_exact(2)`, for clippy 1.98 under `-D warnings`; no behaviour
  change ([#340](https://github.com/sustia-llc/catgraph/issues/340)).

## [workspace-v0.11.0] - 2026-08-10

### Changed

- `rand` leaves this crate's normal-dependency graph: catgraph-applied's lib
  edge now carries `rand_core` alone
  ([#239](https://github.com/sustia-llc/catgraph/issues/239), changed in
  `catgraph-applied`).

## [workspace-v0.10.0] - 2026-08-09

### Fixed

- `cargo check --lib -p catgraph-magnitude --target wasm32-unknown-unknown`
  passes: the workspace `rand` entry is slimmed to no default features
  ([#232](https://github.com/sustia-llc/catgraph/issues/232), fixed in
  `catgraph-applied`). Dev graphs still reach `getrandom` via `proptest`, and a
  build graph containing `catgraph-physics` re-enables rand's defaults by
  feature unification.

## [workspace-v0.9.0] - 2026-08-04

### Changed

- `deep_causality_num` is no longer a dependency: the rig identities come from
  catgraph-applied's `rig::{Zero, One}`, re-exported at this crate's root
  alongside `Rig`, `BoolRig`, `F64Rig`, `Tropical`, and `UnitInterval`. `num`
  stays for BigInt / ToPrimitive in the integer-exact SNF and CRT lift
  ([#219](https://github.com/sustia-llc/catgraph/issues/219), D1 of
  [#218](https://github.com/sustia-llc/catgraph/issues/218)).
- Möbius and (co)weighting test tolerances migrated to `approx_rel`; the
  4-state boundary fixture carries `MOBIUS_BOUNDARY_REL_TOL = 1e-8`
  ([#169](https://github.com/sustia-llc/catgraph/issues/169), tests only).

## [workspace-v0.7.0] - 2026-08-02

### Added

- `coalition_typed` — typed-magnitude valuation surface over the coalition
  engine, re-exported at the crate root as `ChannelCouplings`, `MixedClass`,
  `ModulatedCouplings`, `RoleFibrationProof`, `RoleGrid`, `RoleId`,
  `RoleModulation`, `RoleShares`, `modulate`, and `role_grid`, plus the
  `CoalitionEvaluator::role_shares` method. Additive: no existing signature
  changes, no new `CatgraphError` variant, `UnitInterval` stays the enrichment
  of record ([#211](https://github.com/sustia-llc/catgraph/issues/211)).

## [workspace-v0.6.0] - 2026-08-02

### Added

- `CoalitionEvaluator::value_with_report` / `value_with_report_scratch` return a
  `JoinReport` with `value()` / `base()` / `increment()` / `path()` /
  `schur_complement()` / `zero_proof()` / `is_provably_zero()`. `EvalPath`,
  `JoinReport` and `ZeroDiversityProof` are re-exported at the crate root;
  `value_with` / `value_with_scratch` are unchanged and bit-identical
  ([#153](https://github.com/sustia-llc/catgraph/issues/153)).
- `magnitude_f64` module behind the off-by-default `f64-fast` feature:
  `ZetaFactorization`, `FactorizationPath`, `ConditionReport`, and
  `magnitude_f64(space, t)`. `nalgebra` enters as an optional dependency;
  default-build results are unchanged
  ([#165](https://github.com/sustia-llc/catgraph/issues/165)).
- `tests/magnitude_f64.rs` plus two feature-gated criterion groups and a
  `--features f64-fast` CI lane (test + clippy). The chain-LM group measures
  the Gauss–Jordan fallback, since that fixture's ζ is asymmetric — pinned by a
  test — while `magnitude_f64_symmetric` times the Cholesky route
  ([#165](https://github.com/sustia-llc/catgraph/issues/165)).

## [workspace-v0.5.0] - 2026-07-30

### Added

- `examples/semantic_category.rs` — BTV 2021 Def 8 end to end, no library
  change ([#53](https://github.com/sustia-llc/catgraph/issues/53) item 2).

### Changed

- `mobius_function_via_chains_exact` and `verify_mobius_recursion` route their
  ζ-count `u64 → i64` casts through a checked `zeta_entry_to_q` helper; a count
  above `i64::MAX` returns `CatgraphError::Composition` instead of wrapping.
  Both `#[allow(clippy::cast_possible_wrap)]` sites removed; no signature
  changed ([#88](https://github.com/sustia-llc/catgraph/issues/88)).
- `docs/BTV21-AUDIT.md` §3.2 Def 8 row and summary counts resynced
  ([#53](https://github.com/sustia-llc/catgraph/issues/53) item 2).

## [workspace-v0.4.0] - 2026-07-25

### Added

- `snf::integer::hadamard_bound_matr<R: IntegerLikeRig>` and
  `snf::integer::hadamard_bound_integer` — `MatR<R>` round-trip and float-free
  Hadamard bounds beside the existing `hadamard_bound`
  ([#35](https://github.com/sustia-llc/catgraph/issues/35)).
- `EvalScratch` and `CoalitionEvaluator::value_with_scratch` — caller-owned
  buffers reused across a candidate sweep, bit-identical to `value_with`.
  `EvalScratch` is re-exported at the crate root beside `CoalitionEvaluator`
  ([#33](https://github.com/sustia-llc/catgraph/issues/33) item 1).
- `benches/magnitude_bench.rs` gains the `evaluator_rebuild` and
  `coalition_value_with` groups
  ([#33](https://github.com/sustia-llc/catgraph/issues/33)).
- `LmCategory::from_traces` — prefix-state corpus-MLE constructor for the
  BTV 2021 syntax category (arXiv:2106.07890v2 §2.2 Def 4, Eq 8)
  ([#53](https://github.com/sustia-llc/catgraph/issues/53)).
- `docs/BTV21-AUDIT.md` — BTV 2021 coverage audit, joined to the CI
  audit-count guard ([#53](https://github.com/sustia-llc/catgraph/issues/53)
  item 3).

### Changed

- `snf::smith_normal_form_integer` computes the determinantal divisors
  `D_k = gcd of all k-subset products` by an `O(r²)` dynamic program instead of
  `O(2^r)` subset enumeration; same results
  ([#35](https://github.com/sustia-llc/catgraph/issues/35)).
- `snf::crt::select_primes_for_bound` uses a baked-in const array of the 16
  largest primes below `2^31` and clamps `k_max` to the table length; the
  `primal` dependency is removed from `catgraph-magnitude`
  ([#35](https://github.com/sustia-llc/catgraph/issues/35)).
- `snf::crt_lift` split into `snf::crt` and `snf::integer`; `snf::crt_lift` is
  retained as a `pub use` shim, so all prior `snf::crt_lift::*` paths keep
  compiling ([#35](https://github.com/sustia-llc/catgraph/issues/35)).
- Inline LCG copies in the seeded test / bench / example fixtures replaced by
  the dev-only `catgraph-testutil::Lcg`; random streams are byte-identical and
  a golden-value unit test in `catgraph-testutil` pins the stream contract
  ([#33](https://github.com/sustia-llc/catgraph/issues/33) item 2).
- Paper-audit citation reconciliation across `src/**`, tests, examples, README
  and `docs/BV25-AUDIT.md` (PR #120): `Thm 3.10 → Prop 3.10`; the
  Shannon-entropy derivative `Cor 3.14 → Rem 3.11 + Eq (12)`; the
  `#T(⊥) ≤ Mag(tM) ≤ #ob(M)` bounds re-anchored to BV25's un-numbered intro
  prose; LS `Def 2.5 → Def 3.3`, `Example 2.7 → 2.9`, `§2 → §3`;
  `Prop 2.4.17 → Def 2.1.2 + Prop 2.1.3`; the phantom `§1.4` dropped from
  Leinster08 Cor 1.5.
- `docs/BV25-AUDIT.md` §2/§3 summary rows recounted, §3 acyclicity-hypothesis
  status ✅ → ➖, and five completeness rows added (Leinster 2013 Def 1.1.3;
  BV25 Prop 2.9, Prop 3.6, Cor 3.8/3.9); BV25-AUDIT wired into the
  `scripts/check_audit_counts.py` CI guard.

> **Reconciliation note
> ([#158](https://github.com/sustia-llc/catgraph/issues/158)).** Workspace tags
> `v0.2.1` and `v0.3.0` (2026-07-09 / 2026-07-11) — and `v0.1.1` (2026-07-02,
> between the two sections below) — were cut without per-crate sections here;
> this crate's changes across them are recorded only in git history
> (`git log v0.1.0..v0.3.0 -- catgraph-magnitude/`) and the workspace-level
> release record.

## [workspace-v0.2.0] - 2026-07-02

### Added

- `CoalitionEvaluator` (`coalition_eval` module) — caches a base coalition `S`
  so per-candidate `Mag(S ∪ {x})` queries take an `O(m²)` closure border plus
  the bordered-Schur update `Mag′ = Mag + (1−p)(1−q)/s`, falling back to a slow
  path on near-singular borders, interior improvement or skeletal merge
  ([#31](https://github.com/sustia-llc/catgraph/issues/31)).
- `coalition_value_delta(agents, couplings, members, candidate)` — one-shot
  `(Mag(S), Mag(S ∪ {x}))` at the pinned `t = 1` arm
  ([#31](https://github.com/sustia-llc/catgraph/issues/31)).
- `INCREMENTAL_REL_TOL`, re-exported at the crate root — base value
  bit-identical to fresh, incremental values within `1e-9` relative, rank-order
  identity over candidate sweeps. The leave path stays fresh
  ([#31](https://github.com/sustia-llc/catgraph/issues/31)).

### Changed

- One shared validation / scaling / ζ-kernel path
  (`build_coupling_category`, `scaled_space`, `zeta_from_scaled_distance`)
  backs both fresh and incremental evaluation; no public-surface change.

## [workspace-v0.1.0] - 2026-07-01

First monorepo release: workspace-wide tag `v0.1.0`, superseding the pre-reboot
crate-scoped version lineage below.

### Added

- `coalition_value(agents, couplings, members)` — the `t = 1` scalar,
  equivalent to `coalition_magnitude_from_couplings(.., 1.0)`; re-exported at
  the crate root ([#23](https://github.com/sustia-llc/catgraph/issues/23)).
- `tests/coalition_consumer.rs` — the cross-crate consumer path from
  `catgraph_applied::Hypergraph` members through `coalition_value`
  ([#23](https://github.com/sustia-llc/catgraph/issues/23)).
- `coalition` module — `Coalition<O>` over a `WeightedCospan<O, UnitInterval>`
  with restrict-then-close max-product closure and skeletalization of
  perfectly-coupled members, plus `coalition_magnitude(coalition, t)` and the
  plain-data `coalition_magnitude_from_couplings(agents, couplings, members, t)`.
  Re-exported at the crate root: `Coalition`, `coalition_magnitude`,
  `coalition_magnitude_from_couplings`. New example
  `examples/coalition_magnitude.rs`; no new dependencies
  ([#22](https://github.com/sustia-llc/catgraph/issues/22)).
- `semantic` module — `LmCategory::yoneda_all()`, `k_nearest_from`,
  `k_nearest_to`, and `cluster_semantic_sym`. Re-exported at the crate root:
  `k_nearest_from`, `k_nearest_to`, `cluster_semantic_sym`. New example
  `examples/semantic_comparison.rs`; no new dependencies
  ([#21](https://github.com/sustia-llc/catgraph/issues/21)).
- `yoneda` module — `LmCategory::yoneda(name)` returning a `Copresheaf`, the
  asymmetric `semantic_hom` / `semantic_distance` (BTV 2021 Lemma 2 Eq 11, §5)
  and the non-canonical `semantic_distance_sym`. `LmCategory::enriched_space()`
  extracted out of `magnitude()` with no behaviour change. Re-exported at the
  crate root: `Copresheaf`, `semantic_hom`, `semantic_distance`,
  `semantic_distance_sym` ([#19](https://github.com/sustia-llc/catgraph/issues/19)).
- `LmCategory::deterministic_transition_rank()` (`determinism` module) — the
  rank of `MH₁` at grade `ℓ = 0`, counting covering `π = 1` transitions; reuses
  `chain_complex::{ChainIndex, magnitude_homology_rank}`, no new dependencies
  ([#20](https://github.com/sustia-llc/catgraph/issues/20)).

## [0.5.0] - 2026-05-13

Co-releases with **catgraph-applied v0.6.0** at workspace umbrella `v0.14.0`.

### Breaking

- The `catgraph_applied::Integer` re-export is renamed to
  `catgraph_applied::ZAlgebra`: `use catgraph_magnitude::Integer` must become
  `use catgraph_magnitude::ZAlgebra`. Bounds updated to
  `mobius_function_via_chains_exact<N, Q: Ring + ZAlgebra>`,
  `verify_mobius_recursion<N, Q: Ring + ZAlgebra + Debug>`, and the internal
  `matmul_q`. The trait is otherwise unchanged in structure.

### Added

- `cor_1_5_chain_3_linear_poset` carries a closed-form Phil Hall Möbius
  cross-check against `[[1,-1,0],[0,1,-1],[0,0,1]]` (Leinster 2008 Cor 1.5).
- `verify_mobius_recursion` checks both `μ · ζ = I` and `ζ · μ = I` on every
  fixture; signature unchanged.
- The `modularsnf-oracle` proptest grid widens from `n = 2` to `n ∈ {2, 3, 4}`.

### Changed

- The `modularsnf` dev-dependency moves from a machine-local path dep to a git
  dep at `rev = "d62535e"`, `optional = true`.
- `src/lib.rs` scope headers drop their `(v0.3.0)` version stamps.

### Fixed

- `mobius_chains.rs` rustdoc separates the roles of Leinster 2008 Cor 1.5
  (integer Möbius formula) and Prop 2.10 (termination on circuit-free 𝔸).
- `verify_mobius_recursion` rustdoc carries the Leinster 2008 Def 1.1 (p. 4)
  two-sided-inverse anchor.

## [0.4.0] - 2026-05-13

### Added

- `poset_category` module with `PosetCategory<NodeId>`
  (`from_partial_order`, `from_arrow_counts` with circuit-free DFS validation),
  `mobius_chains::mobius_function_via_chains_exact<N, Q: Ring + Integer>`
  realising `μ = Σ (-1)^k M^k`, and `mobius_chains::verify_mobius_recursion`
  (Leinster 2008 Cor 1.5).
- `snf::crt_lift::smith_normal_form_integer` — integer-exact invariants via
  Hadamard bound, prime selection, per-prime SNF, sign-symmetric CRT
  reconstruction, and the Newman 1972 §1.4 Thm II.9 chain rebalance.
- `Chain::is_finite_in<NodeId>` widened to Leinster–Shulman 2017 pseudo-metric
  spaces (`d(a, b) = 0` permitted for distinct points).
- `snf::smith_normal_form_matr<R: IntegerLikeRig>` round-trip API.
- `IntegerLikeRig` trait, parameterising the rank-recovery surface over a
  generic rig instead of concrete `F64Rig`.
- `examples/integer_mobius.rs` and `examples/prop_3_14_acceptance.rs`.
- `modularsnf-oracle` Cargo feature — dev-only cross-validation, activating
  `dep:modularsnf` + `dep:ndarray`.

### Changed

- `mobius_chains::mobius_chains_graded` renamed to `chain_count_signed_graded`.
- `chain_complex.rs` split into `chain_complex/{mod.rs, homology.rs}`.
- `examples/mock_coalition.rs` gains Prop 3.14 and `magnitude_homology_rank`
  panels.
- `nalgebra` promoted to `[workspace.dependencies]`.
- `catgraph`, `catgraph-applied` and `catgraph-magnitude` gain a default-on
  `rustworkx` feature; `--no-default-features` builds without rustworkx-core,
  ndarray or petgraph in this crate's compile graph.
- `tests/euler_char_identity.rs::fixture_3_5point_path_t_2_5` carries
  `#[cfg_attr(debug_assertions, ignore)]`.
- catgraph-applied substrate bump v0.5.5 → v0.5.6, adding the `Integer` trait
  and the `Z(BigInt)` newtype.

## [0.3.1] - 2026-05-10

Strictly additive on v0.3.0; no API break.

### Fixed

- `snf_rank_over_zp` returns `Result<usize, CatgraphError>` instead of
  panicking inside a `Result`-returning call chain.
- `boundary_matrix<Q>` rustdoc records that the rank-recovery path coerces to
  `F64Rig`; the private alias `Q` is renamed `RankQ`.
- `mobius_chains_graded` rustdoc demoted to "per-grade chain-count diagnostic";
  `euler_char_identity_at`'s numerical path is `magnitude::magnitude`.
- `is_mobius_invertible_at` citation corrected from Leinster 2013 Prop 2.4.17
  to the §2.1 scatteredness threshold (Def 2.1.2 + Prop 2.1.3).
- 12 source files reformatted via `cargo fmt`.
- `catgraph-magnitude/CLAUDE.md` header refreshed to the v0.3.1 surface.

### Changed

- Rustdoc additions: `ChainIndex::grades` round-trip invariant;
  `Chain::is_finite_in` pseudo-metric caveat (LS 2017 Ex 2.9);
  `euler_char_identity_at` `q^ℓ ↔ e^(−ℓ_scaled)` equivalence with the LS 2017
  Theorem 3.5 / Cor 7.15 cross-link; `snf/diagonal.rs::merge_scalars`
  unimodularity comment; `bidiag_step5_to_8_gcd_chain` `stab` search note.
- `snf/diagonal.rs::is_zero` renamed `is_snf_block_zero` with a
  caller-contract docstring.
- `snf/diagonal.rs::chain_matmul_left` uses `split_first().expect(...)` in
  place of `factors[0]` indexing plus a `debug_assert!`.
- `BV25-AUDIT.md` §3.14 row states the `q^ℓ ↔ e^(−ℓ_scaled)` weight
  equivalence.
- Workspace `CLAUDE.md`: Members table `catgraph-dl v0.3.0` → `v0.3.1`, and the
  Sibling-repos catgraph-coalition pin-bump prerequisite `v0.13.2` → `v0.13.3`.

### Added (v0.3.0)

Strictly additive on v0.2.x. Dual-tagged with **catgraph-applied v0.5.5** at
the same release commit.

- `chain_complex` module (Leinster–Shulman 2017 §2): `Chain`,
  `enumerate_chains`, `ChainIndex` with `grades()` / `chains_at(k, ℓ)`,
  `boundary_matrix<Q: Rig + From<i64>>`, `magnitude_homology_rank<Q>` via SNF
  over `Z/p` with single-prime + 2-prime cross-check (Mersenne `2^31 − 1`
  primary), and `euler_char_identity_at(space, t, max_degree)` returning
  `(via_homology, via_magnitude)`.
- `snf` subsystem — a custom Storjohann §7 port over `MatR<Q>`: `snf::zmod`,
  `snf::echelon`, `snf::band`, `snf::phase_1_to_bidiagonal`,
  `snf::diagonal_to_smith`, `snf::bidiagonal_to_smith`,
  `snf::smith_normal_form`, and `snf::verify_snf_invariants`.
- `mobius_chains::mobius_chains_graded<Q: Ring + From<i64>>` — length-graded
  chain-sum μ (Leinster 2013 Prop 2.1.3 + LS 2017 §2 grading).
- `magnitude::is_mobius_invertible_at(space, t) -> bool`.
- `tests/euler_char_identity.rs` — 5-fixture BV 2025 Prop 3.14 acceptance suite
  compared within the analytical residual bound
  `|Δ| ≤ n · r^(max_deg+1) / (1−r) + 1e-9`, `r = (n−1) · exp(−d_min_scaled)`.

### Substrate (v0.3.0)

- Depends on **catgraph-applied v0.5.5** for the mutable `MatR` API,
  `LawvereMetricSpace` accessors, and `impl From<i64> for F64Rig`.
- Algorithmic reference:
  [`events555/modularsnf`](https://github.com/events555/modularsnf) at SHA
  `d62535e` (Apache-2.0), a dev-only oracle behind the `modularsnf-oracle`
  feature and never a runtime dependency.

## [0.2.1] - 2026-05-04

Strictly additive; v0.2.0 API unchanged.

### Added

- `magnitude::scatteredness_witness(space) -> Option<((NodeId, NodeId), f64, f64)>`
  — the first scatteredness violator pair, or `None` when scattered.
- `tests/mobius_chains.rs::boundary_near_non_scattered_returns_err_on_chain_sum`
  at `d = 1.05 < log(3)`.

### Fixed

- `tsallis_entropy` gains a `debug_assert!(t > 0.0)` precondition guard.
- `weighting` / `coweighting` use `swap_remove(n)` per row instead of
  `nth(n).expect(...)`; the `# Panics` section is dropped.
- `mobius_chains.rs` drops the dead `let _ = &mut m;` line and corrects the
  `r == 0.0` branch comment to the discrete-topology case.

### Changed

- Rustdoc corrections in `mobius_chains.rs`: the per-entry geometric bound
  `|μ_{A,k}(a,b)| ≤ ((n − 1)·e^(−ε))ᵏ` (Leinster Prop 2.1.3, p. 11) with the
  row-sum bound `‖M‖_∞ ≤ (n − 1)·e^(−ε) < 1`; the `n · rᴷ⁺¹ / (1 − r)`
  truncation residual annotated as padded over Leinster's per-entry bound; the
  `Q: Ring + From<f64>` bound scoped to rigs whose `is_zero()` matches
  `f64 == 0.0`; `# Errors` fallback hints pointing at
  `magnitude::mobius_function`.
- `weighting` rustdoc spells out the `μ(j, i)` indexing convention for
  Lemma 1.1.4; `coweighting` cites Leinster 2013 §1.1 on symmetric ζ.
- `tsallis_entropy` marked `#[inline]`.
- Private `materialize_objects(space) -> Vec<NodeId>` helper in `magnitude.rs`
  replaces six duplicated FQN dispatch sites.
- Module-level `#![allow(clippy::needless_range_loop)]` in `magnitude.rs`
  replaces six per-site annotations.
- Singular-ζ error messages standardised to
  `"zeta matrix is singular at column {col} (X solve)"`.

## [0.2.0] - 2026-05-04

Strictly additive; v0.1.x API unchanged.

### Added

- `magnitude::weighting::<Q: Ring + Div + From<f64>>(space)` — Leinster 2013
  §1.1 Def 1.1.1, solving `ζ · w = u_I` by Gaussian-Jordan elimination on
  `[ζ | u_I]`.
- `magnitude::coweighting::<Q: Ring + Div + From<f64>>(space)` — the transposed
  system `v · ζ = u_J^T`; `Σⱼ w(j) = Σᵢ v(i)` by Lemma 1.1.2.
- `magnitude::is_scattered(space) -> bool` — Leinster 2013 Def 2.1.2
  `d(a, b) > log(#A − 1)`; vacuous for `n ≤ 1`, unset `+∞` distances auto-pass.
- `mobius_chains` module with
  `mobius_function_via_chains::<Q: Ring + From<f64>>(space)` — Leinster 2013
  Prop 2.1.3 as the von-Neumann series `μ = Σ (−1)ᵏ Mᵏ`, `M = ζ − I`, with
  truncation depth `K = ⌈log(τ) / log(r)⌉` (`τ = 1e-13`, `K_MAX = 200`) and
  `Err(CatgraphError::Composition)` on non-scattered input or `r ≥ 0.94`.
- 13 tests across `tests/weighting_coweighting.rs` and `tests/mobius_chains.rs`.

### Dependencies

Unchanged from v0.1.1: `catgraph` (path), `catgraph-applied` (path), `num`
(workspace), `proptest` + `criterion` (dev). No tokio, no serde, no rayon.

## [0.1.1] - 2026-04-28

Co-released with catgraph v0.12.2 and catgraph-applied v0.5.4 at the same
workspace SHA.

### Breaking

- `LmCategory::add_transition` returns `Result<(), CatgraphError>` instead of
  `()`. The former `debug_assert!` on `prob ∈ [0, 1]` and state membership are
  release-mode `Err` returns, and a non-trivial self-loop
  (`from == to && prob > 0.0`) is rejected. Existing callers append `.unwrap()`
  or `?`.

### Added

- `LmCategory::from_transition_log<I, S, T>(objects, terminating, log)` —
  replay constructor delegating validation to `add_transition`.
- `WeightedCospan::into_validated_metric_space()` — the `Q = UnitInterval` lift
  plus a triangle-inequality scan, returning `Err` on the first violating
  triple.
- `LmCategory::magnitude` gains an `n*n` BFS frontier cap returning
  `CatgraphError::Composition` when exhausted, and a `debug_assert!(t > 0.0)`
  entry guard.
- Five tests in `tests/lm_category.rs` and two in `tests/weighted_cospan.rs`.

## [0.1.0] - 2026-04-25

First publishable release. Anchored to BV 2025 (Bradley & Vigneaux,
arXiv:2501.06662v2).

### Added

- `WeightedCospan<Λ, Q>` over `catgraph::Cospan<Λ>` with per-edge weights in a
  rig `Q`: `from_cospan_uniform`, `from_cospan_with_weights`, `weight`,
  `set_weight`, `as_cospan`, absent entries reading `Q::zero()`; type aliases
  `ProbCospan<Λ>` and `TropCospan<Λ>`; `into_metric_space` on
  `WeightedCospan<Λ, UnitInterval>` lifting via the `-ln π` embedding
  (Lawvere 1973).
- `tsallis_entropy(p, t)` — `H_t(p) = (1 − Σ pᵢᵗ) / (t − 1)` with the Shannon
  branch at `|t − 1| < TSALLIS_SHANNON_EPS`.
- `TSALLIS_SHANNON_EPS = 1e-6` public constant.
- `mobius_function::<Q: Ring + Div + From<f64>>(space)` — Möbius inversion
  `ζ · μ = I` by Gaussian elimination on `[ζ | I]`, `Err` on singular ζ.
- `magnitude::<Q>(space, t)` — `Mag(tM) = Σᵢⱼ μ_t[i][j]` (BV 2025 §3.5 Eq 7).
- `LmCategory` — materialized LM transition table with `new`, `add_transition`,
  `mark_terminating`, `objects`, `terminating`, `transitions`, `magnitude(t)`.
- `Ring` super-trait over `Rig`, blanket-impl'd over `Neg + Sub`.
- `tests/bv_2025_acceptance.rs` (Prop 3.10 closed form; Rem 3.11 Shannon
  recovery by central finite difference at `h = 1e-4`), `tests/lm_category.rs`,
  and the proptest / spot-check suites for `tsallis_entropy`,
  `mobius_function` and `WeightedCospan`.
- `benches/magnitude_bench.rs` — `mag_lm/<N>` at `N ∈ {10, 100, 1000}`.
- `examples/lm_magnitude.rs`, `examples/tsallis_shannon.rs`,
  `examples/mock_coalition.rs`, and a v0.1.0 `README.md`.
- Re-exports at the crate root: `MatR` and the Tier 3 enrichment substrate from
  `catgraph-applied` (`Rig`, `UnitInterval`, `Tropical`, `F64Rig`, `BoolRig`,
  `EnrichedCategory`, `HomMap`, `LawvereMetricSpace`), plus `CatgraphError`
  from `catgraph::errors`.

### Dependencies

- `catgraph = "0.12"` (path dep during development)
- `catgraph-applied = "0.5"` (requires v0.5.3+ for `F64Rig` ring + field ops)
- `num` (workspace dep)
- `proptest`, `criterion` (dev only)
- No tokio, no serde, no rayon

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.16.0...HEAD
[workspace-v0.16.0]: https://github.com/sustia-llc/catgraph/compare/v0.15.0...v0.16.0
[workspace-v0.11.0]: https://github.com/sustia-llc/catgraph/compare/v0.10.0...v0.11.0
[workspace-v0.10.0]: https://github.com/sustia-llc/catgraph/compare/v0.9.0...v0.10.0
[workspace-v0.9.0]: https://github.com/sustia-llc/catgraph/compare/v0.8.0...v0.9.0
[workspace-v0.7.0]: https://github.com/sustia-llc/catgraph/compare/v0.6.0...v0.7.0
[workspace-v0.6.0]: https://github.com/sustia-llc/catgraph/compare/v0.5.0...v0.6.0
[workspace-v0.5.0]: https://github.com/sustia-llc/catgraph/compare/v0.4.0...v0.5.0
[workspace-v0.4.0]: https://github.com/sustia-llc/catgraph/compare/v0.2.0...v0.4.0
[workspace-v0.2.0]: https://github.com/sustia-llc/catgraph/compare/v0.1.1...v0.2.0
[workspace-v0.1.0]: https://github.com/sustia-llc/catgraph/releases/tag/v0.1.0
[0.5.0]: https://github.com/tsondru/catgraph/compare/catgraph-magnitude-v0.4.0...catgraph-magnitude-v0.5.0
[0.4.0]: https://github.com/tsondru/catgraph/compare/catgraph-magnitude-v0.3.1...catgraph-magnitude-v0.4.0
[0.3.1]: https://github.com/tsondru/catgraph/compare/catgraph-magnitude-v0.3.0...catgraph-magnitude-v0.3.1
[0.3.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-magnitude-v0.3.0
[0.2.1]: https://github.com/tsondru/catgraph/releases/tag/catgraph-magnitude-v0.2.1
[0.2.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-magnitude-v0.2.0
[0.1.1]: https://github.com/tsondru/catgraph/releases/tag/catgraph-magnitude-v0.1.1
[0.1.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-magnitude-v0.1.0
