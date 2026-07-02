# Changelog

All notable changes to `catgraph-magnitude` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [workspace-v0.2.0] - 2026-07-02

Incremental coalition magnitude for the decision hot path (#31, PR #32).

### Added

- **`CoalitionEvaluator`** (`coalition_eval` module) — caches a base coalition
  `S` (closed coupling table, skeletal `t`-scaled Möbius inverse, weighting /
  coweighting) so per-candidate `Mag(S ∪ {x})` queries skip the O(m³) fresh
  closure and, on the fast path, the O(k³) inversion: an O(m²) closure border
  plus the bordered-Schur update `Mag′ = Mag + (1−p)(1−q)/s`. Near-singular
  borders (`|s|` within `SCHUR_SLOW_FALLBACK_TOL`) and candidates that improve
  interior couplings or merge skeletal classes fall back to a slow path that
  re-skeletalizes the bordered table (still skipping the fresh closure).
  ~6×/6×/4.4× per 8-candidate sweep at m = 4/8/16 vs two fresh
  `coalition_value` calls per candidate.
- **`coalition_value_delta(agents, couplings, members, candidate)`** — one-shot
  `(Mag(S), Mag(S ∪ {x}))` pair at the pinned `t = 1` arm.
- **`INCREMENTAL_REL_TOL`** (re-exported at the crate root) — the #31-amendment
  numerical contract: base value bit-identical to fresh, incremental values
  within 1e-9 relative, rank-order identity over candidate sweeps. The leave
  path stays fresh (max-product closures do not downdate).

### Changed

- Internal (no public-surface change): one shared validation / scaling / ζ-kernel
  code path (`build_coupling_category`, `scaled_space`,
  `zeta_from_scaled_distance`) now backs both fresh and incremental evaluation,
  keeping the two routes in lockstep by construction.

## [workspace-v0.1.0] - 2026-07-01

First monorepo release: workspace-wide tag `v0.1.0` (supersedes the pre-reboot
crate-scoped version lineage below). The coalition semantic-layer handoff to
downstream koalisi (#19–#23).

### Added

- **Stable consumer entry point `coalition_value`** (`coalition` module, #23).
  `coalition_value(agents, couplings, members) -> Result<f64, CatgraphError>` =
  `coalition_magnitude_from_couplings(agents, couplings, members, 1.0)` — the
  stability-contracted scalar downstream decision policies call (koalisi #5's
  `MagnitudePolicy`, A/B'd against tira/aif's `−G`). Semantics =
  effective-member diversity (skeletalized magnitude); `t = 1` is the pinned
  canonical arm (#22 pins it — the `t`-sweep is an experiment axis of the
  downstream A/B harness, not a knob on this API). Re-exported at the crate
  root. Errors inherited verbatim from `coalition_magnitude_from_couplings`.
- `tests/coalition_consumer.rs` (#23) — the cross-crate **K1 → K2** consumer
  path exercised end to end: `catgraph_applied::Hypergraph` coalition members →
  `VertexIndex`→agent-index mapping → couplings → `coalition_value`. Pins the
  chain fixture (`a→b 0.7, b→c 0.5 ⇒ Mag(1) = 1.8`), the
  `coalition_value == coalition_magnitude_from_couplings(.., 1.0)` identity, the
  dedup-before-magnitude contract (duplicate members rejected until deduped),
  and the skeletalization seam (mutual-`1.0` pair ⇒ `Mag = 1.0`).
- **Enriched-coalition magnitude surface** (`coalition` module, #22; gemini-spec
  §IV.5). Reads a coalition as a **cospan-weighted subgraph of an enriched
  category** — agents = objects, inter-agent couplings = `UnitInterval` (`[0,1]`)
  hom-objects (BTV 2021, arXiv:2106.07890), coalition diversity = `Mag(tA|members)`
  via the BV 2025 §3.5 Eq 7 Möbius sum (arXiv:2501.06662; Thm 3.10's Tsallis
  closed form is the acyclic tree-poset special case — coalitions may be cyclic,
  which Eq 7 handles). `Coalition<O>` wraps a `WeightedCospan<O, UnitInterval>`
  over the members and stores a **derived, immutable** skeletal
  `LawvereMetricSpace` built once at construction. `Coalition::from_enriched`
  applies:
  - **restrict-then-close** — restrict to member homs first, then max-product
    transitive closure through **member nodes only** (dense Bellman–Ford, `m−1`
    rounds; exact for weights `≤ 1` since the optimal path is simple and cycles
    never improve). Coupling mediated through a non-member does **not** count.
    The closure makes composition `A(i,j)·A(j,k) ≤ A(i,k)` hold, so the triangle
    inequality holds by construction under the `−ln` lift.
  - **skeletalization** — members with `A(x,y)=A(y,x)=1.0` (distance `0` both
    ways) are quotiented on the **closed** table (Kolmogorov quotient; magnitude
    is skeleton-invariant, Leinster 2008 / 2013). This removes the singular-ζ
    "identical rows" degeneracy that would otherwise make a perfectly-coupled
    coalition error at every `t`; other singular configurations still return
    `Err`. `effective_members()` reports the skeleton size and `member_classes()`
    the per-member class index; the full member cospan is retained for the
    boundary story.

  `coalition_magnitude(coalition, t)` reads the cached skeletal space (no
  per-call allocation) and calls `magnitude::<F64Rig>` — `t = 1` is the
  canonical arm (its Shannon tie is the derivative `d/dt Mag|_{t=1}=ΣH(p_x)`,
  BV 2025 §3.14 Cor, not the `t=1` value), `t = 2` a collision proxy, `t → ∞` a
  cardinality-like limit. `coalition_magnitude_from_couplings(agents, couplings,
  members, t)` is the plain-data entry point — validates member indices first,
  then coupling indices, rejects self-coupling triples `(i,i,_)` (the identity
  axiom fixes the diagonal), validates probs ∈ `[0,1]` via `UnitInterval::new` —
  the seed of C3's stable `coalition_value` (#23). Hand-computed acceptance
  tests: chain (`A(a,c)=0.35`, cross-checks `LmCategory::magnitude` to 1e-9),
  diamond (`A(a,d)=max(0.30, 0.36)=0.36`, hand-derived `Mag(1)=1.90` via
  back-substitution on the upper-triangular ζ), restrict-before-close pin, cyclic
  couplings (`Mag(1)=4/3`), skeletalization (mutual-1.0 pair → 1 effective agent,
  `Mag=1`; 1.0 three-cycle collapses via the closed table; two clones + one ≡ the
  2-member coalition; asymmetric-1.0 not merged), singleton (`Mag=1` at any `t`),
  construction errors (empty / unknown / duplicate member; self-coupling), and
  `t ≥ 1` monotonicity bounds. New worked example
  `examples/coalition_magnitude.rs` (5-agent table, two overlapping coalitions,
  restrict-then-close `∞` demo, self-asserting). Re-exported at the crate root:
  `Coalition`, `coalition_magnitude`, `coalition_magnitude_from_couplings`.
  No new dependencies.

- **Semantic comparison / clustering over the Yoneda embedding** (`semantic`
  module, #21). Consumer layer over `yoneda` (#19) that ranks and groups whole
  texts by their meanings (Bradley–Terilla–Vlassopoulos 2021, arXiv:2106.07890;
  Lemma 2 Eq 11 hom / §5 asymmetry). Adds `LmCategory::yoneda_all()` — the full
  Yoneda image (one `Copresheaf` per object) from a **single**
  `enriched_space()` pass rather than `n` per-object rebuilds. Adds
  `k_nearest_from` / `k_nearest_to` — the `k` nearest meanings to a query in
  **both** directions of the asymmetric `semantic_distance` (BTV keep the
  Lawvere generalized metric, so "query's nearest" ≠ "nearest to query"); `∞`
  distances are rankable (sort last via `f64::total_cmp`, `NodeId` tie-break),
  self is excluded, `k > len` returns all. Adds `cluster_semantic_sym` —
  single-linkage threshold clustering (connected components where
  `semantic_distance_sym(a, b) <= epsilon`) via plain union-find, O(n²)
  distance evaluations; labelled a **symmetric convenience** over the
  non-canonical `semantic_distance_sym` (mutually-unreachable meanings sit at
  `∞` and never merge). Deterministic output (members ascending, clusters by
  smallest member). New worked example `examples/semantic_comparison.rs`
  (bidirectional nearest-meaning ranking + ≥2 nontrivial clusters, with
  assertions). Re-exported at the crate root: `k_nearest_from`, `k_nearest_to`,
  `cluster_semantic_sym`. No new dependencies.

- **BTV 2021 Yoneda semantic embedding** (`yoneda` module, #19). `LmCategory::yoneda(name)`
  returns the representable copresheaf `L(x, −)` in probability form as a `Copresheaf`
  (`base` / `extension_to` / `distance_to` / `support` / `extensions`, `π = exp(−d)`) —
  meaning-as-distribution over continuations (Bradley–Terilla–Vlassopoulos 2021,
  arXiv:2106.07890). Adds the **asymmetric** semantic internal hom
  `semantic_hom(a, b) = inf_c min{1, b(c)/a(c)}` (BTV 2021 Lemma 2 Eq 11; internal hom
  Eq 6) and `semantic_distance(a, b) = −ln semantic_hom(a, b)` (§5; kept asymmetric per
  BTV "symmetry not required"), plus a non-canonical symmetric `semantic_distance_sym`.
  The shared `LmCategory::enriched_space()` builder was extracted out of `magnitude()`
  (no behaviour change; BV 2025 acceptance tests pass unchanged). Re-exported at the
  crate root: `Copresheaf`, `semantic_hom`, `semantic_distance`, `semantic_distance_sym`.

- **`LmCategory::deterministic_transition_rank()`** (`determinism` module, #20). The rank
  of the first magnitude homology `MH₁` at grade `ℓ = 0`. Since the LS 2017 interior-only
  boundary gives `∂_1 = 0` (so `MH₁(ℓ) = C₁(ℓ) / im ∂₂`), this counts the *covering*
  deterministic transitions — `π = 1` forced continuations / memorisation — of the LM
  transition graph. A structural invariant (BV 2025 / Leinster–Shulman 2017), **not** a
  coherence or hallucination detector (the earlier MH₁-as-obstruction framing was
  falsified and dropped). Reuses `chain_complex::{ChainIndex, magnitude_homology_rank}`;
  no new dependencies.

## [0.5.0] - 2026-05-13

Co-releases with **catgraph-applied v0.6.0** at workspace umbrella `v0.14.0`.
Primary change: consumer-side migration from the `Integer` trait to `ZAlgebra`
(renamed in cg-applied v0.6.0; see that crate's CHANGELOG for the full Bourbaki
*Algèbre* Ch. I §8 — ℤ as initial object of the category of unital rings — rationale). Three design fold-ins shipped
alongside (see Added below).

### Added

- **Closed-form Möbius cross-check fixture:** `cor_1_5_chain_3_linear_poset`
  test fixture extended with a closed-form Phil Hall Möbius cross-check.
  `verify_mobius_recursion` at the fixture tail cross-verifies the integer-exact
  chain-sum against the analytic `[[1,-1,0],[0,1,-1],[0,0,1]]` matrix (Leinster
  2008 Cor 1.5).

- **`verify_mobius_recursion` bidirectional widening:** now checks BOTH
  `μ · ζ = I` (right inverse) and `ζ · μ = I` (left inverse) on every fixture,
  providing a runtime asymmetry guard for the Möbius implementation. Leinster
  2008 Def 1.1 (p. 4) two-sided inverse anchor added to the function's rustdoc.
  Function signature unchanged; internal change only.

- **`modularsnf-oracle` proptest grid extension:** grid widened from `n=2` only
  to `n ∈ {2, 3, 4}`. Three parallel proptest functions
  (`snf_mod_p_rank_agrees_with_modularsnf_2x2`, `_3x3`, `_4x4`); 768 cases
  under `--features modularsnf-oracle` (up from 256). The `n=4` case exercises
  non-trivial rank-recovery and Newman 1972 chain-rebalance interactions at
  4×4 scale.

### Changed

- **`Integer` → `ZAlgebra` migration:** `catgraph_applied::Integer` re-export
  renamed to `catgraph_applied::ZAlgebra` via cg-applied v0.6.0. Downstream code
  using `use catgraph_magnitude::Integer` must migrate to
  `use catgraph_magnitude::ZAlgebra`. See cg-applied v0.6.0 CHANGELOG for the
  full rationale (Bourbaki *Algèbre* Ch. I §8 — ℤ as initial object of the category of unital rings) and migration guide.

- **Trait bounds updated:** `mobius_function_via_chains_exact<N, Q: Ring +
  ZAlgebra>` (was `Q: Ring + Integer`); `verify_mobius_recursion<N, Q: Ring +
  ZAlgebra + Debug>`; internal `matmul_q` helper bound updated accordingly.

- **`modularsnf` dev-dep portability:** converted from machine-local path dep to
  git dep (`{ git = "https://github.com/events555/modularsnf", rev = "d62535e",
  optional = true }`). Enables the `modularsnf-oracle` feature on any developer
  machine and in CI without a local checkout of the `modularsnf` repo.

- **Scope header version-stamps stripped:** `src/lib.rs` `## Scope (v0.3.0)` →
  `## Scope`; subsection `## Algebraic scoping (v0.3.0)` → `## Algebraic
  scoping`. Version stamps in doc comments drift silently across releases; the
  crate version in `Cargo.toml` is the authoritative version indicator.

### Fixed

- **I-5 (citation role labels):** `mobius_chains.rs` rustdoc clarifies the
  distinct roles of Cor 1.5 and Prop 2.10 in Leinster 2008. Cor 1.5 (page 6)
  anchors the integer Möbius formula `μ = Σ (-1)^k M^k`; Prop 2.10 (§1.2)
  anchors the termination bound on circuit-free 𝔸. They are complementary,
  not substitutes.

- **I-6 (Def 1.1 rustdoc anchor):** Leinster 2008 Def 1.1 (p. 4) anchor
  explicitly added to `verify_mobius_recursion` rustdoc, documenting the
  two-sided inverse property `μ · ζ = I` AND `ζ · μ = I`.

### Migration

Downstream code consuming `catgraph_magnitude::Integer` (re-exported from
`catgraph_applied`) must update the import:

```rust
// v0.4.0 (OLD)
use catgraph_magnitude::Integer;
fn foo<Q: Ring + Integer>(...) { ... }

// v0.5.0 (NEW)
use catgraph_magnitude::ZAlgebra;
fn foo<Q: Ring + ZAlgebra>(...) { ... }
```

The trait is otherwise identical in structure; only the name changed. See
cg-applied v0.6.0 CHANGELOG for the `private::Sealed` supertrait addition that
accompanies the rename (prevents accidental out-of-crate impls; behaviour of
existing `impl Integer for Z` sites updated to `impl ZAlgebra for Z`).

Examples-coverage + benches-coverage baselines for v0.5.0 land at the release boundary (first minor bump for this crate's reviewer workflow).

## [0.4.0] - 2026-05-13

### Added

- **§1.17 Leinster 2008 Cor 1.5 integer-exact Möbius via chain enumeration**
  (T16-T17). New module `poset_category` with `PosetCategory<NodeId>` input
  type (`from_partial_order` + `from_arrow_counts` with circuit-free DFS
  validation). New `mobius_chains::mobius_function_via_chains_exact<N, Q: Ring
  + Integer>` realising `μ = Σ_{k=0}^K (-1)^k M^k` (M = ζ - I, K =
  |objects|) with early-termination on zero matrix. New
  `mobius_chains::verify_mobius_recursion<N, Q>` checking μ · ζ = I. Paper
  anchor: `docs/Leinster-0610260v1.pdf` §1.4 Cor 1.5 page 6.
- **§1.10 multi-prime CRT for full integer SNF lift** (T11-T14 from Session 2;
  integrated). `snf::crt_lift::smith_normal_form_integer` returns
  integer-exact invariants via Hadamard bound (T11) + prime selection (T12) +
  per-prime SNF + sign-symmetric CRT reconstruction (T13) + integer chain
  rebalance per Newman 1972 §1.4 Thm II.9 (T14, O(2^r) subset enumeration
  acceptable for r ≤ 20).
- **§1.18 pseudo-metric `is_finite_in` gate** (T6 from Session 2):
  `Chain::is_finite_in<NodeId>` widened to accept Leinster–Shulman 2017
  pseudo-metric spaces (`d(a, b) = 0` for distinct points permitted).
- **§1.20 `smith_normal_form_matr<R: IntegerLikeRig>` round-trip API**
  (T10 from Session 2).
- **§1.21 `IntegerLikeRig` trait** (T9 from Session 2) — parameterises
  rank-recovery surface over generic `IntegerLikeRig` instead of concrete
  `F64Rig`.
- **`integer_mobius.rs` example** (T20) — §1.17 Cor 1.5 demo with Phil Hall
  / Rota / terminal-object fixtures.
- **`prop_3_14_acceptance.rs` example** (T19) — BV 2025 Prop 3.14 5-fixture
  path-C demo with regression-detection `exit(1)` on any margin violation.
- **Examples-coverage baseline** — first walk against the crate's full public
  surface; tracked alongside the release.
- **§1.10 `modularsnf-oracle` Cargo feature** (T15 from Session 2) —
  dev-only cross-validation against external `modularsnf` Rust crate;
  activates `dep:modularsnf` + `dep:ndarray`.

### Changed

- **§1.19 rename:** `mobius_chains::mobius_chains_graded` →
  `chain_count_signed_graded` (T7 from Session 2) — clarifies role as
  per-grade signed chain count diagnostic, NOT the numerical path used by
  `euler_char_identity_at`.
- **§1.12 split:** `chain_complex.rs` →
  `chain_complex/{mod.rs, homology.rs}` (T8 from Session 2) — separates
  type definitions from algorithm implementations.
- **`mock_coalition.rs` refresh** (T18) — adds Prop 3.14 +
  `magnitude_homology_rank` panels demonstrating the v0.3.0 surface beyond
  the v0.1.x baseline.

### Internal

- **R4 nalgebra workspace hoist** (T2 from Session 1) — nalgebra promoted to
  `[workspace.dependencies]` since 2+ crates consume it.
- **R5 rustworkx feature gate propagation** (T22-T23) — `catgraph`,
  `catgraph-applied`, and `catgraph-magnitude` all gain a default-on
  `rustworkx` feature; `--no-default-features` produces a genuinely slim
  build with no rustworkx-core / ndarray / petgraph in cg-mag's compile
  graph.
- **2026-05-13 fixture_3 debug-mode guard** —
  `tests/euler_char_identity.rs::fixture_3_5point_path_t_2_5` carries
  `#[cfg_attr(debug_assertions, ignore)]` (30s release / 15+ min debug under
  SNF + chain enumeration; CI runs in release and exercises it; local
  debug-mode runs auto-skip cleanly).
- **Pre-existing `tests/z_substrate.rs:7` `clippy::doc_markdown` warning**
  fixed (Session 2 T5 deferred finding).

### Refactored

- **`catgraph-applied` substrate bump** v0.5.5 → v0.5.6 — adds `Integer`
  trait + `Z(BigInt)` newtype (T3 + T4 from Session 1).

## [0.3.1] - 2026-05-10

Phase G post-shipping multi-reviewer pass per workspace `CLAUDE.md` release
rule 7. Strictly additive on v0.3.0; no API break.

**Reviewer substitution flag (release rule 7 case (b)):**
`superpowers:code-reviewer` was unavailable in the current environment;
substituted with `feature-dev:code-reviewer` per the cg-dl v0.3.0 + v0.3.1
precedent. Other three reviewer seats ran as designed (`rust-v2:rust-dev-v2`,
`rust-v2:rust-practical`, `general-purpose` deep paper-audit briefed with
BV 2025 + Leinster–Shulman 2017 + Leinster 2013 PDFs).

### Important fixes

- **`snf_rank_over_zp` → `Result<usize, CatgraphError>`** (`chain_complex.rs`;
  Phase G code-quality I-2 + rust-dev-v2 I-1, duplicate). Was a `panic!` in
  a `Result`-returning call chain (`magnitude_homology_rank` →
  `snf_rank_with_cross_check` → `snf_rank_over_zp`). v0.3.1 propagates via
  `?` so a future regression in `smith_normal_form` (e.g. tightened modulus
  precondition) returns `Err` instead of aborting the process. The
  function's invariants (positive prime `p`, rectangular `a`) hold by
  construction in v0.3.0, so this is defensive.
- **`boundary_matrix<Q>` rustdoc clarifies generic-vs-mono coupling**
  (`chain_complex.rs`; rust-dev-v2 I-2). The public surface is generic in
  `Q: Rig + From<i64>`, but the rank-recovery path
  ([`magnitude_homology_rank`], [`euler_char_identity_at`]) silently coerces
  to `F64Rig` via the private type alias. v0.3.1 documents this explicitly
  + renames the alias `Q` → `RankQ` to remove the future-confusion trap.
- **`mobius_chains_graded` rustdoc reconciled** (`mobius_chains.rs`; paper-
  audit I-1). v0.3.0 rustdoc claimed it was "the numerical path of the BV
  2025 Prop 3.14 acceptance gate" but `euler_char_identity_at` does NOT use
  it; the numerical path is `magnitude::magnitude` (matrix-inverse Möbius).
  v0.3.1 demotes the function's role to "per-grade chain-count diagnostic"
  with explicit cross-link to the acceptance-gate flow + a paper-faithful
  alternative (multiply by `q^ℓ`, sum) folded forward to v0.4.0 §1.19.
- **`is_mobius_invertible_at` citation corrected** (`magnitude.rs`; paper-
  audit I-2). v0.3.0 cited "Leinster 2013 Prop 2.4.17"; the actual
  threshold the function checks (`t > log(n − 1)`) is the §2.1 scatteredness
  threshold (Def 2.1.2 + Prop 2.1.3 chain-sum convergence). v0.3.1 fixes
  the citation; behaviour unchanged.
- **12 cg-magnitude source files reformatted via `cargo fmt`** (rust-
  practical I-1). The release rule 4 verification checklist did not include
  `cargo fmt --check`; v0.3.1 cleans the 12 files. The workspace CLAUDE.md
  release rule 4 update to add `cargo fmt --check` is a follow-up
  (architectural-tier item; deferred to a future workspace-doc patch).
- **`catgraph-magnitude/CLAUDE.md` header refreshed** to v0.3.1 with the
  new `chain_complex` + `snf` + `mobius_chains_graded` + `is_mobius_invertible_at`
  scope entries (rust-practical I-2). v0.3.0's CLAUDE.md still described
  only the v0.2.0 surface.

### Minor ride-alongs

- `chain_complex.rs::ChainIndex::grades` rustdoc — round-trip invariant
  (bucketise / reconstruct via `tolerance`) documented (code-quality M-3).
- `chain_complex.rs::Chain::is_finite_in` rustdoc — pseudo-metric
  caveat (LS 2017 Example 2.9) documented; reactivation condition
  (v0.4.0 forward-look §1.18) flagged (paper M-1).
- `chain_complex.rs::euler_char_identity_at` rustdoc — `q^ℓ ↔ e^(−ℓ_scaled)`
  weight equivalence under t-prescaling explicitly stated; LS 2017
  Theorem 3.5 / Cor 7.15 cross-link added (paper M-2 + M-3).
- `snf/diagonal.rs::merge_scalars` — unimodularity proof comment
  `det(V) = 1·(1+q) − q·1 = 1 (mod n)` added (code-quality M-1).
- `snf/diagonal.rs::is_zero` → `is_snf_block_zero` rename (rust-dev-v2 M-4)
  with caller-contract docstring; protects against future reuse on
  non-SNF blocks.
- `snf/diagonal.rs::chain_matmul_left` — replaced `factors[0]` indexing
  + `debug_assert!` with `split_first().expect(...)` for unified
  dev/release behaviour (rust-dev-v2 M-5).
- `snf/diagonal.rs::bidiag_step5_to_8_gcd_chain` — performance note on
  the inline `stab` `O(n)` exhaustive search added (code-quality M-2).
- Workspace `CLAUDE.md` Members table: `catgraph-dl v0.3.0` → `v0.3.1`
  (was a pre-existing oversight from v0.13.2; rust-practical M-1).
- Workspace `CLAUDE.md` Sibling repos catgraph-coalition pin-bump
  prerequisite: `v0.13.2` → `v0.13.3` (rust-practical M-2).
- `BV25-AUDIT.md` §3.14 row: weight equivalence `q^ℓ ↔ e^(−ℓ_scaled)`
  explicitly stated; numerical-comparator clarification cross-linked
  (paper M-3).

### Architectural items folded forward

The 7 architectural findings across the 4 reviewers are deferred to a future
release (SNF backend `Vec<Vec<i64>>` vs `MatR<R>` reconciliation; `RANK_RECOVERY_PRIMES`
parameterisation; `chain_complex::RankQ` generic widening; pseudo-metric
chain enumeration; `mobius_chains_graded` chain-sum-graded reconciliation;
rustworkx-core transitive dep audit; `cargo audit` / `cargo deny` integration).
These are NOT v0.3.1 scope.



Phase E (magnitude-homology Euler-characteristic identity) shipped. Closes
the v0.2.0 §3.14 deferral via the chain-complex / Storjohann SNF /
Euler-char-identity stack. Dual-tagged with **catgraph-applied v0.5.5** at
the same release commit per workspace `CLAUDE.md` release rule 3 (target
workspace umbrella **v0.13.3**).

This release ships the **headline acceptance gate** for the crate's
primary anchor paper: BV 2025 Prop 3.14
`Mag(tM) = Σ_ℓ e^(−tℓ) · Σ_k (−1)ᵏ · rank(H_{k,ℓ}(M))` is verified
end-to-end on 5 fixtures via a dual-path numerical-vs-structural
comparison with a mathematically-justified analytical residual bound.

Strictly additive on v0.2.x; v0.2.0 chain-sum equivalence + Prop 3.10 +
Rem 3.11 acceptance residuals unchanged.

### Added

- **`chain_complex` module** (Leinster–Shulman 2017 §2; Phase B Tasks 6–11):
  - `Chain` simple-chain newtype `(a₀, …, a_k)` with `a_{j−1} ≠ a_j`.
  - `enumerate_chains` DFS over `LawvereMetricSpace<NodeId>` returning all
    simple chains up to a caller-supplied length cutoff.
  - `ChainIndex` materialised `(k, ℓ)`-bucketed index over enumerated
    chains; `grades()` and `chains_at(k, ℓ)` per LS 2017 §2 grading by
    `ℓ = Σ d(a_{j−1}, a_j)`.
  - `boundary_matrix<Q>(idx, k, ℓ)` — alternating-sum drop-one-vertex face
    map yielding the LS 2017 §2 boundary `∂_k: C_{k,ℓ} → C_{k−1,ℓ}` as a
    `MatR<Q>`. Bound `Q: Rig + From<i64>`.
  - `magnitude_homology_rank<Q>(idx, k, ℓ)` — `rank(H_{k,ℓ}(M))` via SNF
    over `Z/p` with single-prime + 2-prime cross-check rank recovery
    (Mersenne `2^31 − 1` primary). Multi-prime CRT for full integer SNF
    lift deferred to v0.4.0 §1.10.
  - `euler_char_identity_at(space, t, max_degree) -> Result<(f64, f64), _>`
    — **headline acceptance gate**. Returns `(via_homology, via_magnitude)`
    at the requested `t` and chain-length cutoff. Compares the structural
    path (Σ_ℓ e^(−tℓ) · Σ_k (−1)ᵏ · rank(H_{k,ℓ}(M))) against the
    numerical path (entry-sum of `(-1)^k Mᵏ` per LS 2017 §2 grading).
    Inline `prev_rank` cache absorbs v0.4.0 forward-look §1.15 (boundary
    matrix recomputation across consecutive `k` iterations) at the call
    site; ~2× SNF speedup on slow-converging fixtures.

- **`snf` subsystem** (custom Storjohann §7 port over `MatR<Q>`; Phase C
  Tasks 12–15 + Phase D Tasks 16′–18′ + Phase E pre-flight Task 20.5):
  - `snf::zmod` — `Z/p` modular helpers (`posmod`, `mulmod_safe`,
    `gcdex`).
  - `snf::echelon` — row-echelon form over `Z/p` (Lemma 7.4).
  - `snf::band` — Phase 1 band reduction.
  - `snf::phase_1_to_bidiagonal` — Phase 1 entry: `(MatR<Q>, n) ↦ (U, B,
    V)` upper-2-banded form.
  - `snf::diagonal_to_smith` — Storjohann §7.7 diagonal-to-Smith via GCD
    chain.
  - `snf::bidiagonal_to_smith` — Storjohann §7.12 fused 9-step pipeline
    end-to-end (port of `events555/modularsnf::snf::smith_from_upper_2_banded`).
  - `snf::smith_normal_form` — top-level entry composing Phase 1 + Phase
    2 + Phase 3.
  - `snf::verify_snf_invariants` — pre-flight invariant verifier (`UAV =
    diag(s_i)` + chain-divisibility); confirms SNF interior soundness
    (no unimodularity panics on Wikipedia 3×3 retrofit).

- **`mobius_chains::mobius_chains_graded<Q>`** (Phase E Task 22) —
  numerical Prop 3.14 path: length-graded chain-sum `μ` per Leinster
  2013 Prop 2.1.3 + LS 2017 §2 grading. Bound `Q: Ring + From<i64>`.

- **`magnitude::is_mobius_invertible_at(space, t) -> bool`** (Phase E
  Task 22) — ergonomic Möbius-existence oracle per Leinster 2013 Prop
  2.4.17 threshold check.

- **5-fixture path C analytical-bound acceptance suite** at
  `tests/euler_char_identity.rs`. Verifies BV 2025 Prop 3.14 across:
  - 4state-scattered (`n=4`, `t=2.0`, `max_deg=4`)
  - 3point-line (`n=3`, `t=3.0`, `max_deg=4`)
  - 5point-path (`n=5`, `t=2.5`, `max_deg=4`)
  - Random 4point (`n=4`, `t=3.0`, `max_deg=3`)
  - 2point (`n=2`, `t=4.0`, `max_deg=2`)

### Acceptance gate (v0.3.0)

Four BV 2025 / Leinster 2013 / LS 2017 verifications must pass at any
v0.3.x tag (v0.1.x + v0.2.0 + new):

1. **BV 2025 Prop 3.10 closed form** — unchanged from v0.1.0, `0e0` (exact `f64`).
2. **BV 2025 Rem 3.11 Shannon recovery** — unchanged from v0.1.0, `~6.46e-10`.
3. **Leinster 2013 Prop 2.1.3 chain-sum equivalence** — unchanged from v0.2.0, `< 1e-9`.
4. **BV 2025 Prop 3.14 magnitude-homology Euler-char identity (NEW)** — `(via_homology, via_magnitude)` from `chain_complex::euler_char_identity_at` agree within an analytical residual bound `|Δ| ≤ n · r^(max_deg+1) / (1−r) + 1e-9` where `r = (n−1) · exp(−d_min_scaled)`. Tight on fixtures 1+5 (chain count saturates the bound); loose on 3+4 where alternating-sum cancellation reduces the actual residual ~100×. Conservative-but-true; tests Prop 3.14 modulo provable finite-truncation residual rather than the (unattainable at locked `max_degree`) absolute `1e-9` claim.

### Path C ratification (math-level decision)

User picked **path C** (analytical residual bound) on 2026-05-09 over
path A (per-fixture engineered tolerance) and path B (re-pick fixtures)
after a 2026-05-08 first attempt surfaced a plan-level calibration bug:
the originally-locked `1e-9` absolute tolerance was unattainable on
slow-converging fixtures at locked `max_degree`. Path C tests the
BV 2025 / LS 2017 identity modulo the provable upper bound on the
omitted-`k > max_degree` chain contribution. See
[`docs/BV25-AUDIT.md`](docs/BV25-AUDIT.md) v0.3.0 deltas section "Why path
C" for the full rationale.

### Substrate

- Depends on **catgraph-applied v0.5.5** mutable `MatR` API (8 mutator
  methods + `LawvereMetricSpace` accessors + `impl From<i64> for F64Rig`).
  Dual-tagged at the same release commit per release rule 3.
- Algorithmic reference: [`events555/modularsnf`](https://github.com/events555/modularsnf)
  at SHA `d62535e` (Apache-2.0). **Dev-only oracle** gated by the
  `modularsnf-oracle` feature flag; **NOT** a runtime dep — workspace
  stays ndarray-free per design doc §2.4 option (c) custom Storjohann
  port over `MatR<Q>`.

### Anchor papers added in-tree

- [`docs/2501.06662v2.pdf`](docs/2501.06662v2.pdf) — BV 2025 (already in-tree from v0.1.x).
- [`docs/1711.00802v4.pdf`](docs/1711.00802v4.pdf) — Leinster–Shulman 2017 (already in-tree from v0.2.0; promoted from forward reference to active anchor at v0.3.0).
- Storjohann 2000 §7 — algorithmic reference for the SNF backend; not in-tree (open-access via author's institutional repository); `events555/modularsnf` is the working reference.

### Performance baseline (v0.3.0)

- 5-fixture acceptance suite finishes in **35.5s** release-mode (single-threaded).
- Fixture 3 (5point-path, `n=5`, slowest converger) finishes in **~28s** single-test mode (vs ~300s pre-`prev_rank`-cache).
- `mag_lm/<N>` v0.1.0 baseline unchanged.
- SNF over `MatR<Q>` at `n ≤ ~50` (typical chain-complex differential size for `n ≤ 5`-vertex fixtures at `max_deg ≤ 4`): sub-second per matrix.

### Out of scope (v0.3.x)

- **`mobius_function_via_chains_exact<Q: Ring>`** (design doc §3.6) was
  STRUCK from v0.3.0 on 2026-05-09 after a spec-tension surfaced (the
  `Q: Ring` bound is incompatible with mirroring v0.2.0's body, which
  requires `Q: Ring + From<f64>`). Paper-faithful `Q: Ring + Integer`
  requires anchoring a NEW paper (Leinster 2008 finite-category Möbius)
  outside the crate's BV/LS/Leinster-2013 anchor surface, plus carving
  a new input type (`PosetCategory<NodeId>`), plus adding a Z-ring
  substrate. Folded forward to v0.4.0 forward-look §1.17. **User-flagged
  trigger**: catgraph-coalition v0.5.0 (slot 4) integer-exact Möbius
  use cases.
- **Multi-prime CRT for full integer SNF lift** — currently single-prime
  + 2-prime cross-check rank recovery (Mersenne `2^31 − 1` primary);
  multi-prime CRT deferred to v0.4.0 §1.10.
- **All v0.2.x out-of-scope items** carry forward unchanged.

### Workspace test counts at v0.3.0

86 integration + lib unit tests + 5 doctests across 18 sets
(catgraph + catgraph-applied + catgraph-magnitude). Clippy pedantic
clean workspace-wide. `cargo doc --workspace --no-deps` clean.

### v0.4.0 forward-look

17 architectural items consolidated for a future release. Headline items:
SNF interior perf (§1.1–§1.4); Storjohann §7.12
paper-faithful bidiag→diag isolation (§1.5); SNF private-helper
duplication (§1.6); `chain_complex.rs` file-size split (§1.12);
multi-prime CRT for full integer SNF lift (§1.10); `boundary_matrix`
recomputation cache pattern generalisation (§1.15); `scale_lawvere_space`
allocation cost (§1.16); **`mobius_function_via_chains_exact<Q: Ring +
Integer>` paper-anchor + input-type expansion (§1.17)** — newest 2026-05-09
from struck Task 24 fold-forward.

v0.4.0 stays dormant until ANY trigger fires (Phase E benchmarks show
SNF interior on hot path; downstream consumer surfaces with non-trivially
sized matrices; **catgraph-coalition v0.5.0 design phase crystallises a
concrete integer-exact Möbius use case**; research goal sharpens for
catgraph-coalition-dl; user opens for research-driven reasons).

### Why this release closes the BV 2025 audit-doc deferral

The crate's anchored claim — that BV 2025 Prop 3.14 (magnitude as the
Euler characteristic of magnitude homology) holds in code — is now backed
by a dual-path numerical-vs-structural acceptance gate on 5 fixtures with
mathematically-justified tolerances. v0.3.0 advances the implementable-DONE
percentage from 89% (v0.2.0: 17/19) to 95% (v0.3.0: 18/19); the remaining
deferred item (§3 Tsallis-side optimization stash) is performance-oriented,
not paper-coverage-oriented.

## [0.2.1] - 2026-05-04

Phase 6F post-shipping three-reviewer patch pass per workspace `CLAUDE.md`
release rule 7. Reviewers: `superpowers:code-reviewer` (general code
quality), `causality:causality-theory` (Leinster 2013 paper-fidelity),
`rust-v2:rust-dev-v2` (Rust idioms / trait bounds / perf).

**Verdict: GO for v0.2.0 as shipped — zero Blocking findings.** v0.2.1
is the additive patch bundling 11 Important + 6 Minor findings before
v0.3.0 magnitude-homology design phase opens. Strictly additive; v0.2.0
API unchanged; BV 2025 Prop 3.10 + Rem 3.11 acceptance residuals
unchanged (`0e0` and `~6.46e-10`). v0.2.0 chain-sum equivalence
acceptance residual unchanged (`< 1e-9`).

### Added

- `magnitude::scatteredness_witness(space) -> Option<((NodeId, NodeId), f64, f64)>`
  — diagnostic companion to `is_scattered`. Returns the first violator
  pair `((a, b), d(a, b), log(#A − 1))` if the space is not scattered, or
  `None` if scattered. v0.3.0 substrate hook (per the three-reviewer
  Rust A-2): the chain complex will use violator pairs as boundary-map
  kernel generators. Includes a doctest verifying the contract on a
  4-state non-scattered fixture.

- One new test in `tests/mobius_chains.rs`: `boundary_near_non_scattered_returns_err_on_chain_sum`
  — fixture at `d = 1.05 < log(3) ≈ 1.0986` (boundary-near, not below by
  much). Verifies `is_scattered`'s strict `>` in Def 2.1.2 has no
  off-by-epsilon issue at the boundary (per Causality Minor #4).

### Fixed

- **`tsallis_entropy` precondition guard.** Added
  `debug_assert!(t > 0.0)` mirroring `LmCategory::magnitude`'s v0.1.1
  documentary check. At `t < 0`, `0.0_f64.powf(t)` returns `+∞` and
  pollutes the sum with non-finite values (per Rust I-2). Function does
  not return `Result` — release-mode callers with `t ≤ 0` get the
  documented NaN/`+∞` pollution. Hot path stays branch-free.

- **`weighting` / `coweighting` ergonomics.** Replaced
  `nth(n).expect(...)` after `into_iter()` with `swap_remove(n)` per row
  (per Code I-1). No behavior change; reads safer to a maintainer
  scanning for unwrap-class concerns. `# Panics` section dropped (no
  longer applicable).

- **`mobius_chains.rs` `let _ = &mut m;` dead-code line removed.** `m`
  doesn't mutate after construction (per Code I-2 + Rust Minor #1).
  Changed `let mut m: Vec<Vec<Q>>` to `let m: Vec<Vec<Q>>`; dropped the
  `let _ = &mut m;` incantation. Cleaner read.

- **`mobius_chains.rs` `r == 0.0` else-branch comment.** Old comment
  ("K=1 step suffices and contributes nothing") was misleading — the
  loop *does* iterate at K=1, computing `(-1)·M = 0`. New comment
  documents the discrete-topology case (`r == 0` ⇒ all off-diagonal
  `d = +∞` ⇒ `M = 0` ⇒ `μ = I`) and explains why the K=1 path is fine
  (per Code I-3). Behavior unchanged.

### Documented (no behavior change)

- **`mobius_chains.rs` operator-norm wording correction.** Old docstring
  claimed "‖M‖ < 1 operator-norm condition." Replaced with the actual
  per-entry geometric bound `|μ_{A,k}(a,b)| ≤ ((n − 1)·e^(−ε))ᵏ` from
  Leinster Prop 2.1.3 proof (page 11), with the row-sum bound
  `‖M‖_∞ ≤ (n − 1)·e^(−ε) < 1` dominating `ρ(M)` as the absolute-
  convergence justification (per Causality I-1).

- **`mobius_chains.rs` truncation-residual `n` factor.** Annotated the
  `n · rᴷ⁺¹ / (1 − r)` bound as defensively-padded over Leinster's tight
  per-entry `rᴷ⁺¹ / (1 − r)` (per Causality I-2). Behavior unchanged
  (the cap is conservative, which is fine).

- **`mobius_chains.rs` `Q: Ring + From<f64>` bound clarification.** Old
  docstring claimed forward-compat to "any future `Ring`-rig." Tightened
  to acknowledge that the implementation's `is_zero()` short-circuits in
  `matmul` + the `r == 0.0` branch assume `Q::is_zero()` matches
  `f64 == 0.0` semantics (which `F64Rig` provides; `Tropical`'s
  `is_zero` is `+∞`). v0.3.0 magnitude-homology will either widen the
  bound semantically or carve `mobius_function_via_chains_exact<Q: Ring>`
  (per Rust I-1, I-7; v0.3.0 design doc §3.6).

- **`mobius_chains.rs` `# Errors` block fallback hints.** Added
  caller-side fallback bullets for both `Err` paths pointing at
  `magnitude::mobius_function` (per Rust I-4).

- **`weighting` / `coweighting` Lemma citations.** `weighting` doc now
  spells out `μ(j, i)` indexing convention for Lemma 1.1.4 (per
  Causality Minor #1). `coweighting` doc now cites Leinster 2013 §1.1
  last paragraph "weightings and coweightings are essentially the same"
  on symmetric ζ (per Causality Minor #2).

- **`tsallis_entropy` `#[inline]` attribute.** Hot-path branch in BV 2025
  Prop 3.10 evaluator (per Rust I-6).

- **`m_k.clone()` vs double-buffering note** added inline in
  `mobius_function_via_chains` matrix-power loop, deferring to v0.3.0
  magnitude-homology design (per the user-flagged carry-forward of
  Rust Minor #4 from the v0.2.1 skip list to the v0.3.0 design doc).

### Internal refactoring (no public API change)

- **Private `materialize_objects(space) -> Vec<NodeId>` helper** in
  `magnitude.rs`. Replaces 6 duplicated FQN dispatch sites of
  `<LawvereMetricSpace<NodeId> as crate::EnrichedCategory<crate::Tropical>>::objects(space).collect()`
  across `mobius_function`, `magnitude`, `weighting`, `coweighting`,
  `is_scattered`, and `mobius_chains::mobius_function_via_chains` (per
  Code Minor #1 + Rust Minor #3).

- **Module-level `#![allow(clippy::needless_range_loop)]`** in
  `magnitude.rs` replaces 6 per-site annotations. Single rationale
  comment in module docs (per Rust Minor #6).

- **Standardized error messages** across `mobius_function`,
  `weighting`, `coweighting`. Now use the consistent prefix
  `"zeta matrix is singular at column {col} (X solve)"` for X ∈
  {weighting, coweighting} (per Code Minor #2).

### Skipped — surfaced explicitly

Per workspace `CLAUDE.md` rule "NEVER silently skip … reviewer-agent
finding. Apply every finding … If you judge a finding is not worth
applying, STOP and surface it to the user with your reasoning so they
decide." User-ratified skips:

- **Replace bool sign-tracking with `signum: i32` flip** (Code Minor #3) —
  pure aesthetic; bool toggle is just as readable.
- **`Q::from((-d.0).exp())` shared helper** (Rust Minor #4) — 4 sites,
  inline form readable, over-DRY.
- **`m_k.clone()` vs double-buffering** (Rust Minor #4 carry-forward) —
  user-flagged carry-forward to v0.3.0 design doc §3.9 instead of skip.
  Inline note added in `mobius_function_via_chains` matrix-power loop.
- **`test_case` dev-dep** (Rust Minor #5) — dev-dep churn unwelcome.
- **`weighting_coweighting.rs` test docstring inconsistency**
  (Code Minor #4) — trivial, folded into the standardize-error-prefix
  pass instead.

### Architectural — consolidated for v0.3.0

All Architectural findings consolidated into the v0.3.0 design:

- BV 2025 §3.13 length grading + Leinster–Shulman §6 Euler char identity
  (Causality A-1, A-2).
- Leinster 2013 Prop 2.4.17 Möbius-invertibility-at-`t` oracle
  (Causality A-3).
- `MatR<Q>` mutable API (`row_swap`, `scale_row`, `add_scaled_row`) for
  SNF — affects catgraph-applied v0.5.5 too (Rust A-1).
- `is_scattered` returning `bool` won't carry magnitude-homology;
  `scatteredness_witness` (Rust A-2) — closed by v0.2.1 Important #10.
- `mobius_function_via_chains_exact<Q: Ring>` no-`From<f64>` variant
  (Rust A-3).
- Pervasive `Vec<Vec<Q>>` representation duplicates across magnitude.rs +
  mobius_chains.rs; SNF will magnify (Code Architectural).
- `is_scattered` NaN-distance silent classification defense (Code
  Architectural).
- `m_k.clone()` vs double-buffering (Rust Minor #4 carry-forward).

### v0.3.0 SNF dependency reference

User-flagged 2026-05-04: <https://github.com/events555/modularsnf/tree/main/crates/modularsnf>
as a candidate dep for `rank(H_{k,ℓ}(M))` computation in BV 2025
Prop 3.14. Local clone planned. Decision deferred to v0.3.0 design phase
(see v0.3.0 design doc §2.4).

## [0.2.0] - 2026-05-04

Phase 6F — chain-sum Möbius via Leinster 2013 Prop 2.1.3, plus the
paper-foundational (co)weighting primitives that v0.1.x bypassed.
Strictly additive; v0.1.x API unchanged; BV 2025 Prop 3.10 + Rem 3.11
acceptance residuals unchanged (`0e0` and `~6.46e-10`).

This release is the **paper-faithful redesign** of the earlier
v0.2.0 spec. Re-reading Leinster 2013 §1, §2, §1.4 + BV 2025 §3
against the spec surfaced five corrections. The
earlier `SignTwist: Rig` trait + `Tropical`/`BoolRig` magnitude
framing was a misattribution that doesn't trace back to BV 2025,
Leinster 2013, or Leinster–Shulman 2017; v0.2.0 ships the actual
Prop 2.1.3 chain-sum (over `Q: Ring`, scattered-space precondition).

Anchor papers added in-tree at `catgraph-magnitude/docs/`:
- `Leinster-1012.5857v3.pdf` — Leinster, *The magnitude of metric spaces* (2013).
- `1711.00802v4.pdf` — Leinster & Shulman, *Magnitude homology* (2017/2021). Forward reference for v0.3.0.
- `1606.00095v2.pdf` — Leinster & Meckes (2016) survey.
- `2201.11363v3.pdf` — Gimperlein, Goffeng, Louca, *The magnitude and spectral geometry* (2025). Downstream of Leinster 2013.

### Added

- `magnitude::weighting::<Q: Ring + Div + From<f64>>(space) -> Result<Vec<Q>, CatgraphError>`
  — Leinster 2013 §1.1 Def 1.1.1. Solves `ζ · w = u_I` (all-ones RHS)
  via Gaussian-Jordan elimination on the augmented `[ζ | u_I]` system.
  By Leinster Lemma 1.1.4, `w(j) = Σᵢ μ(j, i)` (row-sum of `μ = ζ⁻¹`)
  when ζ is invertible. Foundational primitive that v0.1.x bypassed in
  favour of the more restrictive matrix-inversion path.

- `magnitude::coweighting::<Q: Ring + Div + From<f64>>(space) -> Result<Vec<Q>, CatgraphError>`
  — symmetric primitive; solves `v · ζ = u_J^T` via the transposed
  augmented system. By Lemma 1.1.2, `Σⱼ w(j) = Σᵢ v(i) = magnitude`.

- `magnitude::is_scattered(space) -> bool` — Leinster 2013 Def 2.1.2
  predicate `d(a, b) > log(#A − 1)` for all distinct `a, b`. Vacuous
  for `n ≤ 1`; unset (`+∞`) distances auto-pass. Convergence
  precondition for the chain-sum Möbius formula.

- `mobius_chains` module + `mobius_chains::mobius_function_via_chains::<Q: Ring + From<f64>>(space) -> Result<MatR<Q>, CatgraphError>`
  — Leinster 2013 Prop 2.1.3 chain-sum formula
  `μ(a, b) = Σ_{k≥0} (−1)ᵏ · Σ_{a=a₀≠…≠a_k=b} ζ(a₀,a₁) · … · ζ(a_{k−1},a_k)`.
  Realized as the von-Neumann series `μ = Σ (−1)ᵏ Mᵏ` with `M = ζ − I`
  (algebraically identical to the chain-sum-of-ζ-products by
  `Mᵏ[a][b] = Σ chain-products of length k`; the diagonal-zero of M
  enforces the simple-chain `a_{j-1} ≠ a_j` constraint automatically).
  O(K · n³) matrix-power accumulation with adaptive truncation depth
  `K = ⌈log(τ) / log(r)⌉` where `r = (n − 1) · e^(−ε)` is the
  geometric ratio (`τ = 1e-13`, capped at `K_MAX = 200`). Returns
  `Err(CatgraphError::Composition)` on non-scattered input or
  near-boundary `r ≥ 0.94` regime — caller falls back to
  `magnitude::mobius_function::<Q>` (which inverts ζ directly without
  truncation).

- 13 new tests across two integration test files
  (`tests/weighting_coweighting.rs` 6/6, `tests/mobius_chains.rs` 6/6,
  + 1 v0.1.1-carryover sanity case). Acceptance highlights:
  - Lemma 1.1.2 verification (`Σw == Σv == magnitude`) on uniform 4-state space.
  - Lemma 1.1.4 verification (`w(j) == Σᵢ μ(j, i)`) on invertible ζ.
  - Symmetric-ζ `weighting == coweighting` agreement.
  - Chain-sum vs matrix-inversion agreement to `1e-9` on hand-built
    4-state scattered fixture + proptest n=2-5, slack ∈ [0.5, 3.0].

### Acceptance gate (v0.2.0)

Three verifications must pass at any v0.2.x tag:

1. **BV 2025 Prop 3.10 closed form** — `Mag(tM) = (t−1)·Σ H_t(p_x) + #(T(⊥))` to `0e0` (exact f64) on hand-computed 4-state LM at `t ∈ {0.5, 1.5, 2.0, 5.0}`.
2. **BV 2025 Rem 3.11 Shannon recovery** — `d/dt Mag|_{t=1} = Σ H(p_x)` by central FD (`h = 1e-4`) to `~6.46e-10`.
3. **Leinster 2013 Prop 2.1.3 chain-sum equivalence** — `mobius_function_via_chains::<F64Rig>(scattered_space) ≈ mobius_function::<F64Rig>(scattered_space)` to `1e-9`.

### Algebraic scoping (v0.2.0)

Two Möbius paths ship with distinct trait bounds:

- **Field-fast path** — `mobius_function::<Q: Ring + Div + From<f64>>` (v0.1.x). Gaussian elimination on `[ζ | I]`; requires invertible ζ; works on any space.
- **Chain-sum path** — `mobius_function_via_chains::<Q: Ring + From<f64>>` (v0.2.0). Von-Neumann series; requires scattered input; doesn't need `Div`.

Among the workspace's four concrete rigs, only `F64Rig` satisfies either bound in v0.2.0; the wider `Q: Ring + From<f64>` is forward-compat for any future `Ring`-rig.

**Out of scope: `Tropical`-valued / `BoolRig`-valued magnitude.** Per Leinster 2013 §1.3 Examples 1.3.1, the scalar rig `k` is determined by V (V = `[0,∞]` ⇒ k = ℝ). See `docs/BV25-AUDIT.md` §"Out of scope (v0.2.x)" for the full citation chain rejecting the original Phase 6A.6 spec's `Tropical`/`BoolRig` framing.

### Performance baseline (v0.2.0)

Chain-sum is O(K · n³) where K is the adaptive truncation depth. For the typical scattered regime (`r ≤ 0.5`), `K ~ 30` so the chain-sum path costs ~30× more than `mobius_function`'s single O(n³) inversion. Use `mobius_function` as the default for performance; `mobius_function_via_chains` is the algebraically-clean reference path for Prop 2.1.3 verification and any future `Ring`-rig that doesn't admit cheap inversion.

`mag_lm/<N>` v0.1.0 baseline unchanged: `N = 10` ~30 µs / `N = 100` ~11 ms / `N = 1000` ~11 s. v0.2.0 ships no new criterion bench; chain-sum performance is bounded above by `K · mobius_function` cost.

### Dependencies

Unchanged from v0.1.1: `catgraph` (path), `catgraph-applied` (path), `num` (workspace), `proptest`+`criterion` (dev). No tokio, no serde, no rayon.

### Why this release reframes the earlier spec

An earlier internal spec called for `mobius_function_via_chains<Q: Rig>` with a `SignTwist` trait providing `negate_at_parity` for `Tropical` / `BoolRig`. Re-reading Leinster 2013 §1, §2, §1.4 + BV 2025 §3 against the spec surfaced five corrections:

1. The chain-sum is `Σ (−1)ᵏ · ζ-product` (Prop 2.1.3), not `Σ (−1)ᵏ · #chains`.
2. The convergence condition is **scatteredness** (Def 2.1.2), not "acyclic poset."
3. `(−1)ᵏ` requires `Neg`, i.e. `Q: Ring` (not `Q: Rig`).
4. The rig `k` is determined by V (§1.3 Ex 1.3.1) — `Tropical`/`BoolRig` magnitude isn't a thing in our setting.
5. The spec's `BaseChange<Tropical>` recipe doesn't exist in any sibling crate; it was invented and never grounded.

The shipped v0.2.0 surface is paper-faithful: `Q: Ring`; chain-sum-of-ζ-products; scattered; no `SignTwist`. The chain-sum body is the von-Neumann series — algebraically identical to matrix inversion, polynomial-cost, and converges absolutely under scatteredness.

### Roadmap forward — v0.3.0 (deferred)

**BV 2025 Prop 3.14 magnitude-homology Euler-characteristic identity** — `Mag(tM) = Σ_ℓ e^(−tℓ) · Σ_{k≥0} (−1)ᵏ · rank(H_{k,ℓ}(M))`. Headline closing result of BV 2025 that v0.1.x audit missed; deferred to v0.3.0 with own design phase. Requires Leinster–Shulman 2017 §2 chain complex + integer Smith normal form. SNF reference candidate: <https://github.com/events555/modularsnf/tree/main/crates/modularsnf>.

## [0.1.1] - 2026-04-28

Additive patch closing five soundness and pre-flight items surfaced
during a deep review. Co-released with catgraph v0.12.2 +
catgraph-applied v0.5.4 at the same workspace SHA.

The BV 2025 Prop 3.10 + Rem 3.11 acceptance gate residuals are unchanged
(`0e0` and `~6.46e-10` respectively).

### Breaking

- `LmCategory::add_transition` signature changed from `fn(&mut self, &str,
  &str, f64)` to `fn(&mut self, &str, &str, f64) -> Result<(),
  CatgraphError>`. The previous `debug_assert!` on `prob ∈ [0, 1]` and
  state membership are now release-mode `Err` returns; non-trivial
  self-loops (`from == to && prob > 0.0`) — forbidden by BV 2025 §3
  acyclicity hypothesis — are also rejected. Existing callers must append
  `.unwrap()` (test/example/bench fixtures) or `?` (library code).
  Justified for a v0.1.x patch by the absence of any external published
  consumer at this point in the workspace timeline; all known callers
  (3 examples, 2 test files, 1 bench) are updated in this same release.

### Added

- `LmCategory::from_transition_log<I, S, T>(objects, terminating, log) ->
  Result<Self, CatgraphError>` — replay constructor that reconstructs an
  `LmCategory` from an append-only sequence of `(from, to, prob)` triples.
  Designed for the upcoming Phase 6C `magnitude_history` and
  catgraph-surreal `EventLogStore::replay` callers. Validation is
  delegated to `add_transition`, so an invalid entry fails-fast with
  `CatgraphError::Composition`.
- `WeightedCospan::into_validated_metric_space() -> Result<LawvereMetricSpace<NodeId>,
  CatgraphError>` — `Q = UnitInterval` specialization that lifts the
  weighted cospan via `-ln π` AND validates the triangle inequality
  before returning. Returns `Err(CatgraphError::Composition)` on the
  first triple violating `d(x, z) ≤ d(x, y) + d(y, z)`. The
  tree-additivity equality fast path (BV 2025 §2.15 prefix-extension
  semantics) is documented as a v0.2.0+ optimization; v0.1.1 ships the
  full O(n³) scan for correctness.
- `LmCategory::magnitude` — `frontier_steps_remaining = n*n` BFS cap
  (S1.1 defense-in-depth from H.3 verdict #2) and `debug_assert!(t > 0.0)`
  entry guard (S1.4 from H.3 verdict #4). The BFS cap returns
  `CatgraphError::Composition` if exhausted; the `t > 0` check is
  debug-only since `add_transition` already enforces `prob ∈ [0, 1]`,
  making malformed BFS inputs reachable only through future direct-mutation
  callers.
- Five new unit tests in `tests/lm_category.rs` exercising the new error
  paths (`add_transition_*_errors`, `from_transition_log_*`) and two new
  unit tests in `tests/weighted_cospan.rs`
  (`into_validated_metric_space_*_v0_1_1`).

### Why

These items unblock Phase 6C's `EnrichedCoalition::magnitude_history`
replay-from-event-log path and harden the public API surface against the
S1.1/S1.2/S1.4 soundness gaps documented in the 2026-04-27 deep review.
Per H.3 verdict #4, S1.4's root cause is **not** `t < 0` but the
`Tropical(+∞)` vs `Tropical(-∞)` semiring-zero confusion at unset entries;
v0.1.1 ships the documentary `debug_assert` and leaves the
catgraph-applied `LawvereMetricSpace::distance` `+∞` convention intact
(verified at v0.5.4 audit time).

## [0.1.0] - 2026-04-25

First publishable release. Anchored to BV 2025 (Bradley & Vigneaux,
*Magnitude of Language Models*, arXiv:2501.06662v2).

### Added

- Phase 6A.5 criterion bench (`benches/magnitude_bench.rs`) — three
  `mag_lm/<N>` benches (N = 10, 100, 1000) on acyclic forward-chain LMs at
  `t = 2.0`. Baseline median wall-clock (optimized, `--quick`):
  `mag_lm/10` ~30 µs, `mag_lm/100` ~11 ms, `mag_lm/1000` ~11 s.
  O(n³) Gaussian elimination dominates — 1000-state is the practical limit
  for the v0.1.0 dense-matrix Möbius implementation.

- Phase 6A.4 `examples/lm_magnitude.rs` — BV 2025 magnitude bounds
  demonstration on two contrasting LMs (deterministic 3-state, uniform
  5-state). Prints `Mag(tM)` at `t ∈ {0.5, 1.0, 2.0, 10.0, 1e6}` with
  Prop 3.10 closed-form comparison. Asserts four properties from BV 2025
  p.4 for `t ≥ 1`: (A) lower bound `≥ #T(⊥)`, (B) upper bound `≤ #ob(M)`,
  (C) monotone non-decreasing in `t`, (D) `Mag(1e6·M) ∈ [#T(⊥), #ob(M)]`.
  Verifies closed form = Möbius sum to `< 1e-9` at `t ∈ {0.5, 2.0, 10.0}`.
  Note: the `t → ∞` limit equals `#T(⊥)` only for fully-deterministic LMs
  (all-Dirac rows); for non-degenerate rows it is
  `#T(⊥) + #{non-degenerate non-terminal states}`.

- Phase 6A.4 `examples/tsallis_shannon.rs` — Tsallis-to-Shannon recovery
  (BV 2025 Rem 3.11) over 50 seeded random distributions (size 2–5) at
  `δt ∈ {1e-2, …, 1e-7}`. Asserts exact zero error within the
  `TSALLIS_SHANNON_EPS = 1e-6` special-case branch; asserts worst error
  `< 5e-3` at `δt = 1e-3`. Uses a minimal deterministic PCG-64-style LCG —
  same as `tests/lm_category.rs`. No `rand` dev-dep.

- Phase 6A.4 `examples/mock_coalition.rs` — 5-agent
  `WeightedCospan<&str, UnitInterval>` + 3-agent `LmCategory` diversity
  demo without any transport deps. Builds the 5-agent interaction graph
  (including a cycle), prints the Lawvere distance matrix, highlights
  `d(alice, bob) = -ln 0.7` and `d(alice, carol) = ∞` (no transitive
  closure in `into_metric_space`). Builds an acyclic 3-agent prefix-poset
  sub-coalition and prints four magnitude-derived indicators (`Mag(1.0)`,
  `Mag(2.0)`, `Mag(1e6)`, Shannon FD). Asserts BV 2025 p.4 bounds at
  `t = 2.0` and that `Mag(1e6·M) ∈ [#T(⊥), #ob(M)]`. Demonstrates the
  `WeightedCospan`/`LmCategory` API split (cyclic vs. acyclic view) before
  Phase 6B wires in `catgraph-coalition` transport.

- Phase 6A.4 `README.md` — replaced Phase 6A.0 stub with a v0.1.0-quality
  landing page. Includes: quickstart code snippet, two-point acceptance
  gate, full API surface table, algebraic + numerical scoping sections,
  three example descriptions, and roadmap.

- Phase 6A.4 rustdoc audit — fixed 3 pre-existing doc warnings: broken
  intra-doc link `catgraph::Cospan` (replaced with plain text), redundant
  explicit target in `ring.rs`, redundant explicit target in
  `lm_category.rs`. Zero doc warnings on `cargo doc`.

- Phase 6A.3 `magnitude::<Q>(space, t)` — magnitude
  `Mag(tM) = Σᵢⱼ μ_t[i][j]` of a Lawvere metric space at scale `t` via
  Möbius sum (BV 2025 §3.5, Eq 7). Builds a t-scaled copy of the input
  space, Möbius-inverts the resulting zeta matrix, and sums every entry.
  Same algebraic surface as `mobius_function`: `Q: Ring + Div + From<f64>`
  (only `F64Rig` qualifies in v0.1.0).

- Phase 6A.3 `LmCategory` — materialized language-model transition table
  per BV 2025 §3. Public API: `new`, `add_transition`, `mark_terminating`,
  `objects`, `terminating`, `transitions`, `magnitude(t)`. The `magnitude`
  method lifts the transition table into a `LawvereMetricSpace<NodeId>` via
  the prefix-extension semantics of BV 2025 §2.10–2.17: a forward BFS from
  each source state multiplies edge probabilities along every directed path,
  recording `d(x, y) = -ln π(y|x)` where `π(y|x)` is the product of
  intermediate transitions (Eq 6). Identity axiom `d(x, x) = 0` is
  enforced internally. The transition graph must be acyclic for magnitude to
  match Prop 3.10's closed form.

- Phase 6A.3 BV 2025 acceptance gate (`tests/bv_2025_acceptance.rs`):
  - **Prop 3.10 closed form** `Mag(tM) = (t−1)·Σ H_t(p_x) + #(T(⊥))`
    verified to `0e0` (exact `f64`) at `t ∈ {0.5, 1.5, 2.0, 5.0}` on a
    hand-computed 4-state LM (`A = {a}, N = 1`; states `⊥, ⊥a, ⊥†, ⊥a†`;
    `#T(⊥) = 2`).
  - **Rem 3.11 Shannon recovery** `d/dt Mag|_{t=1} = Σ H(p_x)` verified by
    central finite difference `(f(1+h) − f(1−h))/(2h)` with `h = 1e-4`.
    Observed residual `~6.46e-10`.

- Phase 6A.3 `LmCategory` unit tests (`tests/lm_category.rs`): empty-LM
  baseline (`Mag = n` for the identity zeta), round-trip on
  `add_transition` / `mark_terminating`, smoke test on the same 4-state
  tree fixture, and a BV 2025 Eq 4.3 bounds proptest
  (`#T(⊥) ≤ Mag(tM) ≤ #ob(M)` for `t ≥ 1`) on randomly generated
  forward-chain LMs of size 2–4.

- Phase 6A.2 `tsallis_entropy(p, t)` — Tsallis q-entropy
  `H_t(p) = (1 − Σ pᵢᵗ) / (t − 1)` with Shannon-recovery special case at
  `|t − 1| < TSALLIS_SHANNON_EPS = 1e-6`. The special-case branch returns
  `-Σ pᵢ ln pᵢ` directly, avoiding catastrophic cancellation in the `0/0`
  regime around `t = 1`. The Rem 3.11 finite-difference step `h` MUST stay
  above the threshold so both `f(1±h)` evaluate the Tsallis branch.

- Phase 6A.2 `mobius_function::<Q>(space)` — Möbius inversion `ζ · μ = I`
  via Gaussian elimination on an `n × 2n` augmented matrix `[ζ | I]`. Bound
  `Q: Ring + Div + From<f64>` — a (commutative) field for v0.1.0; only
  `F64Rig` qualifies among the workspace's four concrete rigs. Returns
  `Err(CatgraphError::Composition)` when zeta is singular. The chain-sum
  variant `mobius_function_via_chains<Q: Rig>` per Leinster-Shulman is
  deferred to v0.2.0.

- Tests: 4 proptest arms (Shannon recovery within ε threshold,
  Tsallis-to-Shannon limit on normalized distributions, μ·ζ=I on random
  Lawvere metric spaces) + 3 spot checks (basic Tsallis values, all-∞
  singular zeta, all-zero singular zeta).

- Re-exports: `MatR` (from `catgraph-applied`), `CatgraphError` (from
  `catgraph::errors`).

- Phase 6A.0 scaffold: workspace member, `Cargo.toml`, `lib.rs` with module
  stubs + re-exports of the Tier 3 enrichment substrate from
  `catgraph-applied` v0.5.x (`Rig`, `UnitInterval`, `Tropical`, `F64Rig`,
  `BoolRig`, `EnrichedCategory`, `HomMap`, `LawvereMetricSpace`).

- `Ring` super-trait over `Rig` with blanket impl over `Neg + Sub`. Required
  by Möbius inversion.

- `TSALLIS_SHANNON_EPS = 1e-6` public constant — Shannon special-case
  threshold for `tsallis_entropy` and lower bound for the Rem 3.11
  finite-difference step.

- Phase 6A.1 `WeightedCospan<Λ, Q>` newtype wrapper over
  `catgraph::Cospan<Λ>` carrying per-edge weights in a rig `Q`. Public API:
  `from_cospan_uniform`, `from_cospan_with_weights`, `weight`, `set_weight`,
  `as_cospan`. Absent entries return `Q::zero()`. Type aliases
  `ProbCospan<Λ>` (= `WeightedCospan<Λ, UnitInterval>`) and
  `TropCospan<Λ>` (= `WeightedCospan<Λ, Tropical>`). Specialized
  `into_metric_space` on `WeightedCospan<Λ, UnitInterval>` lifts to a
  `LawvereMetricSpace<NodeId>` via the `-ln π` embedding (Lawvere 1973).
  Tests: 2 proptest arms (round-trip + `set_weight` idempotence on
  `Q = F64Rig`) + 3 spot checks (metric-space embedding on `Q = UnitInterval`,
  absent-edge zero on `Q = Tropical`, per-pair `from_cospan_with_weights`).

### Acceptance gate

Both BV 2025 verifications pass at v0.1.0:

- **Prop 3.10 closed form** — `Mag(tM) = (t−1)·Σ H_t(p_x) + #(T(⊥))`
  verified to **0e0** (exact `f64`) on a 4-state hand-computed LM
  at `t ∈ {0.5, 1.5, 2.0, 5.0}`.
- **Rem 3.11 Shannon recovery** — `d/dt Mag|_{t=1} = Σ H(p_x)` by central
  finite difference (`h = 1e-4`) verified to **6.46e-10** on the same
  fixture.

### Numerical scoping

- `TSALLIS_SHANNON_EPS = 1e-6` — threshold below which `tsallis_entropy`
  returns `-Σ pᵢ ln pᵢ` directly to avoid catastrophic cancellation.
- Tsallis-Shannon worst-case recovery error: `0` (exact) at
  `δt < TSALLIS_SHANNON_EPS` (special-case branch); `< 5e-3` at
  `δt = 1e-3` (Tsallis branch).

### Performance baseline

`mag_lm/<N>` (criterion median wall-clock, optimized, `--quick`):

- `N = 10`: ~30 µs
- `N = 100`: ~11 ms
- `N = 1000`: ~11 s

### Dependencies

- `catgraph = "0.12"` (path dep during development; crates.io strips path on publish)
- `catgraph-applied = "0.5"` (requires v0.5.3+ for `F64Rig` ring + field ops)
- `num` (workspace dep)
- `proptest`, `criterion` (dev only)
- No tokio, no serde, no rayon

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.2.0...HEAD
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
