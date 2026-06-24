# Bradley-Vigneaux 2025 Coverage Audit (catgraph-magnitude v0.3.0)

> **Paper:** Bradley & Vigneaux, *Magnitude of Language Models* ([arXiv:2501.06662v2](https://arxiv.org/abs/2501.06662), Jan 2025)
> **Library:** `catgraph-magnitude` v0.3.0 (workspace member of `catgraph` v0.12.2+)
> **Date:** 2026-04-25 (v0.1.0 initial audit); updated 2026-04-28 for v0.1.1 (validated metric, replay constructor, BFS cap); updated 2026-05-04 for v0.2.0 Phase 6F (chain-sum Möbius via Leinster 2013 Prop 2.1.3 + (co)weighting primitives + scatteredness predicate; new §3.14 magnitude-homology row added as DEFERRED to v0.3.0); updated 2026-05-09 for v0.3.0 Phase E close (deltas section appended at branch HEAD `98ba05e`, pre-Phase F tag); **updated 2026-05-10 for v0.3.0 Phase F documentation closure + workspace umbrella `v0.13.3` release commit (`541f943` — catgraph-applied v0.5.5 dual-tag at the same SHA per release rule 3): BV 2025 Prop 3.14 magnitude-homology Euler-characteristic identity SHIPPED via `chain_complex` + `snf::*` Storjohann port + `magnitude_homology_rank<Q>` + `euler_char_identity_at` path C analytical-bound acceptance gate; §3.14 row promoted ⏭️ DEFERRED → ✅ DONE; §3.6 `mobius_function_via_chains_exact` surface row STRUCK from v0.3.0 + deferred to v0.4.0 §1.17 paper-anchor expansion. Pre-tag verification clean (release rule 4): `cargo test --workspace --release` 1284/0; clippy pedantic clean; `cargo doc --workspace --no-deps` zero warnings (workspace-wide rustdoc cleanup ride-along at `64ba64c` cleared 9 pre-existing warnings).**
> **Method:** read all sections; cross-checked each numbered definition / proposition / remark / equation against `catgraph-magnitude` source and the enrichment substrate re-exported from `catgraph-applied` v0.5.5
>
> **Note on scope:** BV 2025 is anchored in §2 (enriched categories of language models) and §3 (magnitude via Tsallis q-entropy). Only §2 Defs/Eqs that materialize as runtime types and §3 Props that constitute the v0.1.x acceptance gate are tracked here. Categorical foundations (§1, §4) live in `catgraph-applied`'s `enriched.rs` + `lawvere_metric.rs` and are audited by [`catgraph-applied/docs/FS18-AUDIT.md`](../../catgraph-applied/docs/FS18-AUDIT.md).
>
> **Relationship to catgraph core audit:** The Lawvere metric / `-ln π` embedding (§2.5–2.7) is implemented in `catgraph-applied::lawvere_metric` and consumed here via `WeightedCospan::into_metric_space`. See [`catgraph/docs/FS19-AUDIT.md`](../../catgraph/docs/FS19-AUDIT.md) for the cospan substrate underlying `WeightedCospan`.
>
> **Companion paper:** Bradley, Terilla, Vlassopoulos, *An Enriched Category Theory of Language* (BTV 2021, arXiv:2106.07890) — extension to substitution-grammar enrichment. Operationalised in [`catgraph-coalition/docs/BTV21-AUDIT.md`](https://github.com/tsondru/catgraph-coalition/blob/main/docs/BTV21-AUDIT.md) for v0.3.0+.

**Status legend:**
- ✅ DONE — implemented and tested
- ⚠️ PARTIAL — implementation exists but does not fully exhibit the paper's structure
- ⏭️ DEFERRED — planned for a later release (v0.2.0 chain-sum Möbius, etc.)
- ➖ N/A — theoretical / motivational, no implementation expected
- 🔗 IN APPLIED — implemented in `catgraph-applied` (re-exported by this crate); noted for completeness

## Summary

| Section | DONE | PARTIAL | DEFERRED | N/A | IN APPLIED | Total |
|---|---|---|---|---|---|---|
| §1.1 (co)weighting primitives | 1 | 0 | 0 | 0 | 0 | 1 |
| §2 LM as enriched category | 4 | 0 | 0 | 2 | 3 | 9 |
| §2.1 scatteredness predicate | 1 | 0 | 0 | 0 | 0 | 1 |
| §3 Magnitude via Tsallis | 6 | 0 | 1 | 1 | 0 | 8 |
| §3.5 Möbius / chain-sum | 2 | 0 | 0 | 0 | 0 | 2 |
| §3.14 Magnitude homology | 1 | 0 | 0 | 0 | 0 | 1 |
| §4 Bounds + asymptotics | 3 | 0 | 0 | 2 | 0 | 5 |
| **TOTAL** | **18** | **0** | **1** | **5** | **3** | **27** |

**Headline numbers (as of catgraph-magnitude v0.3.0):**
- **67% DONE / 0% PARTIAL / 4% DEFERRED / 19% N/A / 11% IN APPLIED**
- Of the 27 audited items, 3 are already in `catgraph-applied` (enrichment substrate), 5 are N/A (motivational), leaving **19 implementable items** of which **18 are DONE, 0 PARTIAL, 1 DEFERRED**.
- Of implementable items: **95% DONE / 0% PARTIAL / 5% DEFERRED**
- The one remaining deferred item is the §3 Tsallis-side optimization stash (in-section). v0.3.0 closes the v0.2.0 §3.14 deferral via the magnitude-homology / chain-complex / Storjohann SNF / Euler-char-identity stack. **The v0.3.0 design doc §3.6 surface row `mobius_function_via_chains_exact<Q: Ring>` was STRUCK from v0.3.0 and folded forward to v0.4.0 §1.17** — the paper-faithful destination requires anchoring a NEW paper (Leinster 2008 finite-category Möbius) outside the crate's BV/LS/Leinster-2013 anchor surface; user-flagged trigger is catgraph-coalition v0.5.0 (slot 4) integer-exact Möbius use cases.

---

## Per-section detail

### §1.1 (co)weighting primitives (foundational, anchored at Leinster 2013 §1.1)

| Item | Status | Location | Notes |
|---|---|---|---|
| §1.1 Def 1.1.1 + Lemma 1.1.2 + Lemma 1.1.4: weighting / coweighting | ✅ | `magnitude::weighting`, `magnitude::coweighting` | v0.2.0. Both solve `ζ · w = u_I` (resp. `v · ζ = u_J^T`) by Gaussian-Jordan elimination on the augmented system. By Lemma 1.1.2, `Σⱼ w(j) = Σᵢ v(i) = magnitude`. By Lemma 1.1.4, on invertible ζ, `w(j) = Σᵢ μ(j, i)` (row-sum of `μ = ζ⁻¹`). Verified in `tests/weighting_coweighting.rs` (6/6 tests including Lemma 1.1.2 + Lemma 1.1.4 numerical residuals). Bound: `Q: Ring + Div + From<f64>` matches `mobius_function`. |

### §2 Language models as enriched categories

| Item | Status | Location | Notes |
|---|---|---|---|
| §2.1 Vocabulary alphabet `A`, length cap `N` | ➖ | — | parameter, no struct |
| §2.5 Lawvere metric space `([0,∞], ≥, +, 0)` | 🔗 | `catgraph-applied::lawvere_metric::LawvereMetricSpace` | Triangle-inequality verifier + `-ln π` embedding (Lawvere 1973). Re-exported from this crate. |
| §2.6 V-enriched category over Lawvere quantale | 🔗 | `catgraph-applied::enriched::EnrichedCategory<V>` | v0.5.1; v0.5.4 adds `LawvereMetricSpace<T>` diagonal default `Tropical::one()`. |
| §2.7 `HomMap<O, V>` finite realization | 🔗 | `catgraph-applied::enriched::HomMap` | Finite materialization used by `LmCategory::magnitude`. |
| §2.8 Probability cospan over alphabet | ✅ | `weighted_cospan::WeightedCospan<Λ, Q>` | v0.1.0 newtype over `catgraph::Cospan<Λ>` carrying per-edge rig weights. Type aliases `ProbCospan<Λ>` / `TropCospan<Λ>`. |
| §2.9 Probability → distance via `-ln π` | ✅ | `WeightedCospan::into_metric_space` (`Q = UnitInterval`) | v0.1.0 specialization. v0.1.1 adds `into_validated_metric_space` with full O(n³) triangle-inequality scan. |
| §2.10–2.17 Prefix-extension semantics | ✅ | `lm_category::LmCategory` | Materialized BYO-LM transition table. Forward BFS multiplies edge probabilities along directed paths; `d(x, y) = -ln π(y\|x)` recorded per Eq 6. |
| Identity axiom `d(x, x) = 0` | ✅ | `LmCategory::magnitude` (internal) | Enforced before Möbius inversion. v0.5.4 `LawvereMetricSpace::hom` diagonal default also returns `Tropical::one()` at `a == b`. |
| §2.17 Acyclicity hypothesis | ✅ | `LmCategory::add_transition` (v0.1.1) | v0.1.1 rejects non-trivial self-loops (`from == to && prob > 0.0`) at insert time. Cycle-via-path forbidden by BV 2025 §3 acyclicity hypothesis but not detected ahead of `magnitude(t)` (BFS cap surfaces it). |

### §2.1 Scatteredness (anchored at Leinster 2013 Def 2.1.2; convergence precondition for chain-sum Möbius)

| Item | Status | Location | Notes |
|---|---|---|---|
| Leinster Def 2.1.2: `is_scattered(space)` ⇔ `d(a,b) > log(#A−1)` for all distinct `a, b` | ✅ | `magnitude::is_scattered` | v0.2.0. Vacuous for `n ≤ 1`; unset (`+∞`) distances auto-pass. Used by `mobius_chains::mobius_function_via_chains` as a precondition before the truncated von-Neumann series accumulator runs. |

### §3 Magnitude via Tsallis q-entropy

| Item | Status | Location | Notes |
|---|---|---|---|
| Eq 4 / Def: Tsallis q-entropy `H_t(p) = (1 - Σ pᵢᵗ) / (t-1)` | ✅ | `magnitude::tsallis_entropy(p, t)` | Shannon special case at `\|t-1\| < TSALLIS_SHANNON_EPS = 1e-6` returns `-Σ pᵢ ln pᵢ` directly (avoids `0/0` cancellation around `t = 1`). |
| Rem 3.11 Shannon recovery as `t → 1` | ✅ | `magnitude::tsallis_entropy` + acceptance test | Acceptance residual `~6.46e-10` by central FD `h = 1e-4` on 4-state LM. |
| Prop 3.10 closed form `Mag(tM) = (t-1)·Σ H_t(p_x) + #(T(⊥))` | ✅ | `tests/bv_2025_acceptance.rs` | v0.1.0 acceptance residual `0e0` (exact `f64`) at `t ∈ {0.5, 1.5, 2.0, 5.0}` on hand-computed 4-state tree (`A = {a}, N = 1`; `#T(⊥) = 2`). |
| §3.4 Acyclicity hypothesis (tree-shaped prefix poset) | ✅ | `LmCategory` runtime contract | v0.1.0 fixture rebuilt from cyclic to 4-state acyclic prefix poset during 6A.3 implementation; documented in `current-plan.md` Phase 6A. |
| §3.5 Eq 7 magnitude as Möbius sum `Mag = Σᵢⱼ μ[i][j]` | ✅ | `magnitude::magnitude::<Q>(space, t)` | Builds t-scaled zeta, Möbius-inverts, sums every entry. Algebraic surface `Q: Ring + Div + From<f64>`. |
| §3.5 Möbius inversion `ζ·μ = I` | ✅ | `magnitude::mobius_function::<Q>(space)` | Gaussian elimination on `[ζ \| I]` augmented matrix. `Err(CatgraphError::Composition)` on singular zeta. v0.1.0 limit ~1000 states (O(n³)). |
| §3.5 Chain-sum Möbius (Leinster 2013 Prop 2.1.3) | ✅ | `mobius_chains::mobius_function_via_chains` | v0.2.0. Implemented as the von-Neumann series `μ = Σ (−1)ᵏ Mᵏ` with `M = ζ − I` (algebraically identical to Prop 2.1.3's chain-sum-of-ζ-products by Mᵏ[a][b] = chain-sum at length k). O(K · n³) matrix-power accumulation with adaptive K = ⌈log(τ) / log(r)⌉, τ = 1e-13, capped at K_MAX = 200. Bound `Q: Ring + From<f64>` (no `Div` needed). Acceptance: chain-sum agrees with v0.1.x `mobius_function` to 1e-9 on hand-built 4-state + proptest n=2-5. Returns `Err` on non-scattered or near-boundary (r ≥ 0.94) input — caller falls back to `mobius_function`. |
| §3.6 Numerical scoping `TSALLIS_SHANNON_EPS = 1e-6` | ✅ | `lib.rs` | Public constant; threshold for special-case branch and lower bound on Rem 3.11 finite-difference step. |

### §3.14 Magnitude homology Euler-characteristic identity

| Item | Status | Location | Notes |
|---|---|---|---|
| BV 2025 Prop 3.14 (page 21): `Mag(tM) = Σ_ℓ q^ℓ · Σ_{k≥0} (−1)ᵏ · rank(H_{k,ℓ}(M))`, `q = e^(−t)` | ✅ | `chain_complex::euler_char_identity_at` + `chain_complex::magnitude_homology_rank` + `snf::*` (Storjohann port). **NOTE (v0.3.1):** the numerical comparator is `crate::magnitude::magnitude` (matrix-inverse Möbius), NOT `mobius_chains::mobius_chains_graded` — the latter is a per-grade chain-count diagnostic without `q^ℓ` weighting. | **v0.3.0.** Headline closing result of BV 2025; mirror of Leinster–Shulman 2017 Theorem 3.5 / Cor 7.15 (the metric-space specialisation directly used here; the LS 2021 statement is the parametric `Q((q^ℝ))` formal-series analogue). Implementation t-pre-scales the space by `t` so the weight `q^ℓ_orig = e^(−t · ℓ_orig)` collapses to `e^(−ℓ_scaled)` in pre-scaled coordinates. Implementation: (a) Leinster–Shulman 2017 §2 chain complex via `Chain` + `enumerate_chains` DFS + `ChainIndex` (length-graded basis materialisation) + `boundary_matrix<Q>` (alternating face map drop-one-vertex); (b) length-grading by `ℓ = Σ d(a_{j−1}, a_j)` via `ChainIndex::grades()` per LS 2017 §2; (c) integer Smith normal form for `rank(H_{k,ℓ})` via custom Storjohann §7 port (`snf::{zmod, echelon, band}` + `phase_1_to_bidiagonal` + `diagonal_to_smith` + `bidiagonal_to_smith` fused 9-step + top-level `smith_normal_form`) over `MatR<Q>` with single-prime + 2-prime cross-check rank recovery (Mersenne `2^31 − 1` primary). **Algorithmic reference + dev-only oracle:** [`events555/modularsnf`](https://github.com/events555/modularsnf) at SHA `d62535e` (Apache-2.0); workspace stays ndarray-free (option (c) custom port over `MatR<Q>` per design doc §2.4, NOT a runtime dep). **Acceptance gate (v0.3.0 path C):** `euler_char_identity_at(space, t, max_degree)` returns `(via_homology, via_magnitude)` agreeing within an analytical residual bound `\|Δ\| ≤ n · r^(max_deg+1) / (1−r) + 1e-9` where `r = (n−1) · exp(−d_min_scaled)`. Tests: 5 fixtures pass at branch HEAD `98ba05e` (release suite 35.5s). See v0.3.0 deltas section for implementation deltas + path-C ratification rationale (replaced the originally-locked `1e-9` absolute tolerance which was unattainable on slow-converging fixtures at locked `max_degree`). |

### §4 Bounds + asymptotics

| Item | Status | Location | Notes |
|---|---|---|---|
| Eq 4.3 bounds `#T(⊥) ≤ Mag(tM) ≤ #ob(M)` for `t ≥ 1` | ✅ | `tests/lm_category.rs` proptest | Bounds proptest on randomly generated forward-chain LMs (size 2–4). |
| Asymptotic `Mag(t·M) → #T(⊥)` as `t → ∞` (deterministic) | ✅ | `examples/lm_magnitude.rs` | Asserted at `t = 1e6`. v0.1.0 caveat documented: the `t → ∞` limit equals `#T(⊥)` only for fully-deterministic LMs (all-Dirac rows); for non-degenerate rows it is `#T(⊥) + #{non-degenerate non-terminal states}`. |
| Monotonicity of `Mag` in `t` for `t ≥ 1` | ✅ | `examples/lm_magnitude.rs` | Property (C) asserted. |
| §4 Information-theoretic interpretation | ➖ | — | motivational; covered narratively in `README.md`. |
| §4 Connection to BTV 2021 grammar enrichment | ➖ | — | Forward-pointer; operationalised in `catgraph-coalition` v0.3.0+. |

---

## Acceptance-gate residuals

| Test | Target | Observed (v0.1.1) |
|---|---|---|
| Prop 3.10 closed form | `Mag(tM) = (t-1)·Σ H_t(p_x) + #(T(⊥))` to `~1e-9` | **`0e0`** (exact `f64`) at `t ∈ {0.5, 1.5, 2.0, 5.0}` |
| Rem 3.11 Shannon recovery | `d/dt Mag\|_{t=1} = Σ H(p_x)` to `~1e-6` | **`6.46e-10`** by central FD `h = 1e-4` |
| Tsallis-Shannon worst @ `δt = 1e-3` | `< 5e-3` (Tsallis branch) | **`1.226e-3`** |
| Tsallis-Shannon worst @ `δt < 1e-6` | exact zero (special-case branch) | **`0`** (exact) |

Both v0.1.x acceptance verifications live in `tests/bv_2025_acceptance.rs` and pass at every release.

---

## Performance baseline (v0.1.x)

`mag_lm/<N>` criterion median wall-clock (optimized, `--quick`, acyclic forward-chain LM at `t = 2.0`):

| N | Median |
|---|---|
| 10 | ~30 µs |
| 100 | ~11 ms |
| 1000 | ~11 s |

O(n³) Gaussian elimination dominates above n ≈ 100. 1000-state is the practical limit for the v0.1.x dense-matrix Möbius implementation; v0.2.0 chain-sum Möbius would lift this for sparse / `Tropical` regimes.

---

## Out of scope (v0.2.x)

- **`Tropical`-valued / `BoolRig`-valued magnitude.** Per Leinster 2013 §1.3 Examples 1.3.1, the scalar rig `k` is **determined by the enriching V**, not free at the call site: `V = [0,∞] ⇒ k = ℝ` (the v0.1.x and v0.2.0 setting), `V = FinSet ⇒ k = ℚ`, `V = 2 (Boolean) ⇒ k = ℤ`, `V = FDVect ⇒ k = ℚ`. There is no paper-aligned notion of magnitude valued in `Tropical` or `BoolRig` for our V = Lawvere `[0,∞]` setting. An earlier internal spec's framing (`SignTwist: Rig` trait + `negate_at_parity` for `Tropical` / `BoolRig`) was a misattribution that doesn't trace back to BV 2025, Leinster 2013, or Leinster–Shulman 2017. The §1.4 final-paragraph "formal magnitude in `R(A)`" path (free k-algebra containing a weighting + coweighting) is the only paper-aligned route to magnitude across rigs without a fixed k; it is deferred indefinitely until a downstream consumer surfaces a concrete need.
- **Agent transport** (SurrealDB `RELATE`, tokio live-queries) — see [`catgraph-coalition`](https://github.com/tsondru/catgraph-coalition) (Phase 6B+ external sibling).
- **BTV 2021 substitution-grammar enrichment** — operationalised in `catgraph-coalition` v0.3.0+; see `BTV21-AUDIT.md` in that repo.
- **Yoneda copresheaves for role discovery** — designed as a parallel-track post-v0.2.0 feature; independent of the runtime layer.
- ~~**BV 2025 Prop 3.14 magnitude-homology Euler-characteristic identity** — deferred to v0.3.0~~ — **SHIPPED in v0.3.0** via the magnitude-homology / chain-complex / Storjohann SNF / Euler-char-identity stack. See §3.14 row above + v0.3.0 deltas section.
- **`mobius_function_via_chains_exact<Q: Ring>`** — **shipped at v0.4.0**. Originally planned as a v0.3.0 surface alongside the magnitude-homology stack, struck on 2026-05-09 after a spec-tension surfaced (the `Q: Ring` bound is incompatible with mirroring v0.2.0's body, which requires `Q: Ring + From<f64>`). Paper-faithful `Q: Ring + Integer` (renamed `Q: Ring + ZAlgebra` at v0.5.0) required a new paper anchor (Leinster 2008 finite-category Möbius), a new input type (`PosetCategory<NodeId>`), and a Z-ring substrate.

---

## v0.1.1 deltas (2026-04-28 H.4)

Five additive items closing soundness / pre-flight gaps from a deep review. BV 2025 acceptance residuals unchanged (`0e0` and `~6.46e-10`).

| Item | Spec | Audit row affected |
|---|---|---|
| `LmCategory::add_transition` returns `Result<(), CatgraphError>` (BREAKING) | Reject non-trivial self-loops + invalid probabilities at insert time | §2.17 acyclicity row |
| `LmCategory::from_transition_log<I, S, T>` replay constructor | Rebuild from append-only event log; fail-fast on invalid entries | (new — replays §2.10–2.17 transition stream) |
| `WeightedCospan::into_validated_metric_space` (`Q = UnitInterval`) | `-ln π` embedding + full O(n³) triangle-inequality verifier | §2.9 row |
| `LmCategory::magnitude` BFS frontier cap (`n*n`) | Defense-in-depth against non-acyclic input; `CatgraphError::Composition` on overflow | §2.17 / §3.4 acyclicity rows |
| `LmCategory::magnitude` `debug_assert!(t > 0.0)` entry guard | Documentary check; doesn't fix `Tropical(±∞)` semiring-zero confusion (root cause per H.3 verdict #4) | §3.5 Möbius row |

---

## v0.2.0 deltas (2026-05-04 Phase 6F)

Three new public surfaces + two audit row additions + rewritten Out-of-scope first bullet. Strictly additive; no v0.1.x API break; BV 2025 Prop 3.10 + Rem 3.11 acceptance residuals unchanged (`0e0` and `~6.46e-10`).

| Item | Spec | Audit row affected |
|---|---|---|
| `magnitude::weighting::<Q: Ring + Div + From<f64>>(space) -> Result<Vec<Q>>` | Leinster 2013 §1.1 Def 1.1.1; solves `ζ · w = u_I` via Gaussian-Jordan elimination on the augmented `[ζ \| u_I]` system. By Lemma 1.1.4, equals row-sum of `μ = ζ⁻¹` when ζ is invertible. | NEW §1.1 row |
| `magnitude::coweighting::<Q: Ring + Div + From<f64>>(space) -> Result<Vec<Q>>` | Symmetric primitive; solves `v · ζ = u_J^T` via the transposed augmented system. By Lemma 1.1.2, `Σⱼ w(j) = Σᵢ v(i) = magnitude`. | NEW §1.1 row |
| `magnitude::is_scattered(space) -> bool` | Leinster Def 2.1.2 predicate `d(a,b) > log(#A−1)`; convergence precondition for chain-sum Möbius. Vacuous for `n ≤ 1`. | NEW §2.1 row |
| `mobius_chains::mobius_function_via_chains::<Q: Ring + From<f64>>(space) -> Result<MatR<Q>>` | Leinster 2013 Prop 2.1.3 chain-sum formula realized as the von-Neumann series `μ = Σ (−1)ᵏ Mᵏ`, `M = ζ − I`. O(K · n³) with adaptive `K = ⌈log(τ)/log(r)⌉`, `τ = 1e-13`, capped at `K_MAX = 200`. Returns `Err` on non-scattered input or near-boundary `r ≥ 0.94` (caller falls back to `mobius_function`). | §3.5 row (now ✅ DONE) |
| (audit) NEW §3.14 row | BV 2025 Prop 3.14 magnitude-homology Euler-characteristic identity; deferred to v0.3.0 with own design phase. See §3.14 row above. | NEW §3.14 row |
| (audit) Out-of-scope rewrite | Rejects the original Phase 6A.6 spec's `Tropical`/`BoolRig` magnitude framing as a misattribution; cites Leinster §1.3 Examples 1.3.1 (V picks k) and §1.4 (formal magnitude in `R(A)`) as the only paper-aligned alternatives for non-Ring V's. | Out of scope (first bullet) |

### Why

An earlier internal spec called for `mobius_function_via_chains<Q: Rig>` with a `SignTwist` trait providing `negate_at_parity` for `Tropical` / `BoolRig`. Re-reading Leinster 2013 §1, §2, §1.4 + BV 2025 §3 against the spec surfaced five corrections:

1. The chain-sum is `Σ (−1)ᵏ · ζ-product` (Prop 2.1.3), not `Σ (−1)ᵏ · #chains`.
2. The convergence condition is **scatteredness** (Def 2.1.2: `d(a,b) > log(#A−1)`), not "acyclic poset."
3. `(−1)ᵏ` requires `Neg`, i.e. `Q: Ring`. There is no "tropical sign convention" anywhere in the cited literature.
4. The rig `k` is determined by V (§1.3 Ex 1.3.1) — not free at the call site. For our `V = [0,∞]`, `k = ℝ` always. `Tropical`-valued / `BoolRig`-valued magnitude isn't a thing in BV 2025 or Leinster 2013.
5. The spec's `BaseChange<Tropical>` recipe (cited twice) doesn't exist as a trait or function in `catgraph-applied` v0.5.4 or any sibling; it was invented by the spec author and never grounded.

The shipped v0.2.0 surface is paper-faithful: `Q: Ring` (not `Q: Rig`); chain-sum-of-ζ-products (not chain-counts); scattered (not acyclic); no `SignTwist`. The chain-sum body is the von-Neumann series — algebraically identical to matrix inversion, polynomial-cost, and converges absolutely under scatteredness.

BV 2025 Prop 3.14 magnitude-homology Euler-characteristic identity is the BV 2025 paper's headline closing result that v0.1.x audit missed. NEW §3.14 row added; deferred to v0.3.0 (own design phase + Leinster–Shulman 2017 §2 read pass + integer Smith normal form decision; see <https://github.com/events555/modularsnf> as a candidate reference).

---

## v0.3.0 deltas (Phases A–F — magnitude homology Euler-characteristic identity SHIPPED 2026-05-10)

**Phase E acceptance gate ✅ shipped 2026-05-09 at branch HEAD `98ba05e` (41 commits ahead of `main`, pushed; pre-Phase F tag); Phase F documentation closure ✅ shipped 2026-05-09 at `83b4c33` + 2026-05-10 at `541f943` (Cargo.toml bump + workspace doc closure + dual-tag `catgraph-applied-v0.5.5` + `catgraph-magnitude-v0.3.0` + workspace umbrella `v0.13.3`).** v0.3.0 closes the **headline §3.14 deferral** that v0.2.0 added. The crate's primary deferred audit row (BV 2025 Prop 3.14 magnitude-homology Euler-characteristic identity) flips ⏭️ DEFERRED → ✅ DONE. Workspace umbrella target: **v0.13.3** (catgraph-applied v0.5.5 dual-tag at the same release commit per release rule 3).

| Item | Spec | Audit row affected |
|---|---|---|
| `chain_complex::Chain` + `chain_complex::enumerate_chains` (DFS over `LawvereMetricSpace<NodeId>`) | Length-graded simple-chain enumeration `(a₀, …, a_k)` with `a_{j−1} ≠ a_j`; grade `ℓ = Σ d(a_{j−1}, a_j)`. Per Leinster–Shulman 2017 §2. Phase B Tasks 6–9. | NEW §3.14 row (now ✅ DONE) |
| `chain_complex::ChainIndex` + `boundary_matrix<Q>` | Materialised `(k, ℓ)`-bucketed chain index with `chains_at(k, ℓ)`; alternating-sum drop-one-vertex face map yields the LS 2017 §2 boundary `∂_k: C_{k,ℓ} → C_{k−1,ℓ}`. Phase B Tasks 10–11. | NEW §3.14 row |
| `snf::{zmod, echelon, band}` + `phase_1_to_bidiagonal` (Phase 1 band reduction) | Custom Storjohann §7 port over `MatR<Q>`: integer arithmetic over `Z/p` Mersenne (`2^31 − 1`) primary; cache-friendly band reduction; Lemma 7.4 echelon / band invariants verified per-step. Phase C Tasks 12–15. **Algorithmic reference**: `events555/modularsnf` (Apache-2.0) at SHA `d62535e`; **dev-only oracle** gated by `modularsnf-oracle` feature flag (NOT a runtime dep — workspace stays ndarray-free). | NEW §3.14 row |
| `snf::diagonal_to_smith` + `snf::bidiagonal_to_smith` (fused 9-step Storjohann §7.12) + top-level `smith_normal_form` | Phase 2 (diagonalisation) + Phase 3 (Smith form) end-to-end. **Re-decomposition mid-phase** (2026-05-08): the original Tasks 16/17/18 were re-shaped as Tasks 16′/17′/18′ to match the upstream `events555/modularsnf::snf::smith_from_upper_2_banded` 9-step fused pipeline (the bidiag→Smith logic is fused, not separable, in the modularsnf reference). Phase D Tasks 16′–18′. | NEW §3.14 row |
| `snf::verify_snf_invariants` + Wikipedia 3×3 fixture | Pre-flight invariant verifier — confirms the SNF interior is sound (no unimodularity panics on Wikipedia 3×3 retrofit). Phase E pre-flight Task 20.5. | NEW §3.14 row |
| `chain_complex::magnitude_homology_rank<Q>` | `rank(H_{k,ℓ}(M))` via SNF over `Z/p` (single-prime + 2-prime cross-check rank recovery; multi-prime CRT for full integer SNF lift deferred to v0.4.0 §1.10). Phase E Task 21. | NEW §3.14 row |
| `mobius_chains::mobius_chains_graded<Q>` + `magnitude::is_mobius_invertible_at` | Numerical Prop 3.14 path (length-graded chain-sum `μ` per Leinster 2013 Prop 2.1.3 + LS 2017 §2 grading) + ergonomic Möbius-existence oracle (Leinster 2013 Prop 2.4.17 threshold check). Phase E Task 22. | NEW §3.14 row + cross-link to v0.2.0 §3.5 row (chain-sum Möbius now extended to graded) |
| **`chain_complex::euler_char_identity_at(space, t, max_degree)`** + 5-fixture acceptance gate | **Headline acceptance**: returns `(via_homology, via_magnitude)` at the requested `t` and chain-length cutoff. Compares the structural path (`Σ_ℓ e^(−tℓ) · Σ_k (−1)ᵏ · rank(H_{k,ℓ}(M))`) against the numerical path (`Σ_ℓ e^(−tℓ) · Σ_k (−1)ᵏ · entry-sum(Mᵏ_at_ℓ)`). Inline `prev_rank` cache absorbs v0.4.0 forward-look §1.15 (boundary-matrix recomputation across consecutive `k` iterations) at the call site; ~2× SNF speedup on slow-converging fixtures. Phase E Task 23. | NEW §3.14 row (✅ DONE; see Acceptance-gate residuals below) |
| Path C analytical-bound acceptance assertion | Replaced originally-locked `\|Δ\| < 1e-9` absolute tolerance after a 2026-05-08 first attempt surfaced a plan-level calibration bug (locked `1e-9` unattainable on slow-converging fixtures at locked `max_degree`). User-ratified path C: `\|Δ\| ≤ analytical_residual_bound(n, t · d_min_original, max_degree) + 1e-9` where `bound = n · r^(max_degree + 1) / (1 − r)`, `r = (n − 1) · exp(−d_min_scaled)`. Tight upper bound on omitted-`k > max_degree` chain contribution. Tests Prop 3.14 modulo provable finite-truncation residual (the conservative-but-true regime). | §3.14 row + new Acceptance-gate residuals row |
| (audit) §3.6 surface-row deferral | The v0.3.0 design doc §3.6 surface row `mobius_function_via_chains_exact<Q: Ring>` was STRUCK from v0.3.0 on 2026-05-09 after a spec-tension surfaced (the `Q: Ring` bound is incompatible with mirroring v0.2.0's body, which requires `Q: Ring + From<f64>`). Paper-faithful `Q: Ring + Integer` requires anchoring a NEW paper (Leinster 2008 finite-category Möbius) outside the crate's BV/LS/Leinster-2013 anchor surface, plus carving a new input type (`PosetCategory<NodeId>`), plus adding a Z-ring substrate. Folded forward to v0.4.0 forward-look §1.17. | Out-of-scope second bullet (NEW); §3.5 Möbius / chain-sum row count unchanged. |
| (audit) Status legend example refresh | The status-legend "v0.2.0 chain-sum Möbius" example in line 17 was outdated post v0.2.0 ship; v0.3.0 deltas section provides the canonical current example (the §3.6 deferral). Status legend left as-is (purely illustrative). | Status legend (notes) |
| (substrate) catgraph-applied v0.5.5 dual-tag | 8 mutable `MatR` methods (`row_swap` / `col_swap` / `row_scale` / `col_scale` / `row_add_scaled` / `col_add_scaled` / `entry_mut` / `entries_mut`) + 3 `LawvereMetricSpace` accessors (`size` / `objects` / `from_distance_fn`) + `impl From<i64> for F64Rig` ride-along. Substrate for the Storjohann port; ships at the same release commit per release rule 3. | Cross-crate substrate (catgraph-applied audit doc handles this row). |

### Acceptance-gate residuals (v0.3.0 Phase E)

| Test | Target | Observed (v0.3.0, release build) |
|---|---|---|
| Prop 3.14 fixture 1 (4state-scattered, `n=4`, `t=2.0`, `max_deg=4`) | `\|Δ\| ≤ 2.12e-6 + 1e-9` (path C analytical bound) | **`1.90e-6`** (89% of bound; tight) |
| Prop 3.14 fixture 2 (3point-line, `n=3`, `t=3.0`, `max_deg=4`) | `\|Δ\| ≤ 3.26e-5 + 1e-9` | **`5.02e-6`** (15% of bound; comfortable) |
| Prop 3.14 fixture 3 (5point-path, `n=5`, `t=2.5`, `max_deg=4`) | `\|Δ\| ≤ 2.84e-2 + 1e-9` | **`3.13e-4`** (1% of bound; very loose; alternating-sum cancellation) |
| Prop 3.14 fixture 4 (random 4point, `n=4`, `t=3.0`, `max_deg=3`) | `\|Δ\| ≤ 2.34e-3 + 1e-9` | **`1.36e-5`** (0.6% of bound; very loose) |
| Prop 3.14 fixture 5 (2point, `n=2`, `t=4.0`, `max_deg=2`) | `\|Δ\| ≤ 1.25e-5 + 1e-9` | **`1.21e-5`** (96% of bound; tight) |

**Bound is mathematically tight on fixtures 1 + 5** (chain count `n · (n − 1)^k` saturates the `H_k` rank); **loose on 3 + 4** where alternating-sum cancellation reduces the actual residual by ~100×. Per Phase E acceptance intent ("Prop 3.14 holds modulo provable truncation residual"), conservative-but-true is the desired regime. Release suite 35.5s (fixture 3 finishes in ~28s single-test mode, vs prior 300s timeout pre-`prev_rank`-cache).

### Why path C (analytical bound) over A (per-fixture engineered tolerance) or B (re-pick fixtures)

- **Path A** (per-fixture engineered tolerance) — would have set `tol = 1.5 × observed_residual` per fixture; mathematically arbitrary; tests "the residual hasn't drifted" rather than "Prop 3.14 holds modulo truncation."
- **Path B** (re-pick fixtures) — would have replaced slow-converging fixtures (3, 4) with faster ones (e.g., uniform-distance fixtures where the geometric ratio is small); mathematically valid but artificially restricts the test surface to "fixtures the implementation handles well."
- **Path C** (analytical residual bound) — math-faithful: tests the BV 2025 / LS 2017 identity modulo the provable upper bound on the omitted-`k > max_degree` chain contribution. Tight on fixtures 1 + 5 (saturates the bound); loose on 3 + 4 (alternating-sum cancellation reduces the actual residual). Conservative-but-true.

### Workspace test counts at v0.3.0 (release SHA `541f943`)

86 integration + lib unit tests + 5 doctests across 18 sets (catgraph + catgraph-applied + catgraph-magnitude); 1284 passes / 0 failures workspace-wide on the v0.3.0 release commit per release rule 4 verification. Clippy pedantic clean workspace-wide. `cargo doc --workspace --no-deps` zero warnings — the workspace-wide rustdoc cleanup ride-along at `64ba64c` cleared 9 pre-existing warnings (3 in `catgraph`, 3 in `catgraph-applied`, 2 in `catgraph-physics`, plus 1 introduced by the v0.5.5 ride-along `EnrichedCategory::objects` rustdoc).

### Architectural findings folded forward to v0.4.0

17 items in the v0.4.0 forward-look as of 2026-05-09:

- §1.1–§1.4 — SNF-interior perf items (`matmul_mod` cache; `n_big × n_big` padding; Phase 1 boundary on `Vec<Vec<i64>>`; `phase_1_to_bidiagonal` co-location)
- §1.5 — Storjohann §7.12 paper-faithful bidiag→diag isolation (Phase D re-decomposition deferral)
- §1.6 — SNF private-helper duplication across `snf::band` + `snf::diagonal`
- §1.7 — `snf::diagonal` file-size growth split
- §1.8 — Storjohann §7.10 / §7.11 chain-rule oracle helpers
- §1.9 — Phase D Tasks 19–20 (Storjohann fixture lift + `modularsnf-oracle` proptest) deferral
- §1.10 — Multi-prime CRT for full integer SNF lift (currently single-prime + 2-prime cross-check)
- §1.11 — `verify_snf_invariants` scalability (`det_mod` cofactor expansion; matmul intermediate alloc)
- §1.12 — `chain_complex.rs` file-size growth (305 → 419 → 534 LOC across Phase E)
- §1.13 — Kahan summation for `mobius_chains_graded` f64 stability (forward-look)
- §1.14 — `ChainIndex` reuse (`_with_index` overload pattern)
- §1.15 — `boundary_matrix` recomputation across consecutive `k` iterations (partially absorbed inline at `euler_char_identity_at` via `prev_rank` cache; residual concern for direct external `magnitude_homology_rank` consumers)
- §1.16 — `scale_lawvere_space` per-call `O(n²)` `HashMap` rebuild (Mag(tM)-curve consumer would amplify)
- §1.17 — **`mobius_function_via_chains_exact<Q: Ring + Integer>` paper anchor + input-type expansion** (struck Task 24 fold-forward; user-flagged trigger: catgraph-coalition v0.5.0 integer-exact Möbius use cases)

Phase G (post-shipping multi-reviewer pass per release rule 7 canonical triumvirate non-substitutable + additive deep paper-audit reviewer) is the next checkpoint; Phase G findings will trigger v0.3.1 patch + v0.4.0 forward-look check.

### Why v0.3.0 closes BV 2025

The crate's anchored claim — that BV 2025 Prop 3.14 (magnitude as the Euler characteristic of magnitude homology) holds in code — is now backed by a dual-path numerical-vs-structural acceptance gate on 5 fixtures with mathematically-justified tolerances (path C analytical bound). Both paths agree to within the provable finite-truncation residual; the crate's audit doc reflects this with the §3.14 row promotion. v0.3.0 advances the implementable-DONE percentage from 89% (v0.2.0: 17/19) to 95% (v0.3.0: 18/19); the remaining deferred item (§3 Tsallis-side optimization stash) is performance-oriented, not paper-coverage-oriented.

---

## v0.4.0 §1.17 Leinster 2008 Cor 1.5 paper-audit

> **Paper:** Leinster, *The Euler characteristic of a category* ([arXiv:math/0610260v1](https://arxiv.org/abs/math/0610260), 8 Oct 2006 — published Documenta Math 2008).
> **In-tree at:** [`catgraph-magnitude/docs/Leinster-0610260v1.pdf`](Leinster-0610260v1.pdf) (~263 KB; cold-read primary anchor per release rule 7 hybrid-paper-audit workflow).
> **Method:** end-to-end re-read of §1 (Möbius inversion, pages 4-9); cross-walked Cor 1.5 (page 6), Prop 2.10 (referenced in §1 Def 1.1 commentary on Möbius vs graph Möbius), and Ex 1.11(c) (terminal object weighting) against shipped code at HEAD `a9b94ad`.
> **Anchor surface added 2026-05-13** (post-shipping Phase G paper-audit reviewer pass; 4th-seat additive per release rule 7).

### Paper-to-code cross-walk

| Paper item | Page | Crate surface | Status |
|---|---|---|---|
| Def 1.1 (incidence algebra `R(𝔸)`, zeta `ζ_𝔸`, Möbius `μ_𝔸 = ζ⁻¹`) | p. 4 | `PosetCategory::{zeta_matrix, zeta}` (`u64` arrow counts; `Q::from_i64` lift to ring at use site) | ✅ DONE |
| Ex 1.2(a) (poset Möbius by induction, `μ(a,a) = 1`, `μ(a,c) = -Σ_{a≤b<c} μ(a,b)`) | p. 4 | `verify_mobius_recursion` checks the equivalent `μ · ζ = I` algebraically | ✅ DONE |
| Ex 1.2(c) (`𝔻^inj_N`, `ζ(a,b) = C(b,a)`) | p. 4 | `tests/mobius_chains_exact.rs::cor_1_5_n_path_inj_2` fixture (3-object slice) | ✅ DONE |
| Lemma 1.3 (equivalent conditions: idempotents-are-identities ⇔ endomorphisms-are-automorphisms ⇔ circuits-are-isomorphisms) | p. 5 | `PosetCategory::from_arrow_counts` enforces (b) ∧ (c) directly: diagonal `ζ[i][i] ∈ {0, 1}` rules out non-identity endos at the level of arrow counts; `has_cycle` DFS rules out non-identity circuits. Skeletality is implicit (object list is the index) | ✅ DONE |
| Thm 1.4 (Möbius inversion exists on finite skeletal 𝔸 with identity-only idempotents; closed form with `\|Aut(a_i)\|` denominators) | p. 5 | The `\|Aut\|` denominators degenerate to 1 under the stronger Cor 1.5 hypothesis (identity-only endomorphisms ⇒ trivial automorphism group at every object). The implementation specialises to this case | 🔗 SPECIALISED to Cor 1.5 |
| **Cor 1.5** (finite skeletal 𝔸 with identity-only endomorphisms ⇒ `μ(a,b) = Σ_{n≥0} (−1)^n · #{nondeg n-paths a → b} ∈ ℤ`) | **p. 6** | **`mobius_chains::mobius_function_via_chains_exact<N, Q>` over `Q: Ring + Integer`** | ✅ DONE |
| Cor 1.5 termination (implicit: nondeg path count vanishes once `n > #(non-identity arrows on any chain from a to b)`) | p. 6 | Algorithm early-terminates when `M^k` becomes the zero matrix (the `all_zero` flag in the loop body); bounded above by `k = n = \|objects\|` per the `for _k in 1..=n` ceiling | ✅ DONE |
| Phil Hall classical formula (Prop 3.8.5 of Stanley; Prop 6 of [R]) on a poset | p. 6 (commentary) | `tests/mobius_chains_exact.rs::cor_1_5_chain_3_linear_poset` recovers Phil Hall on the 3-chain | ✅ DONE |
| Thm 1.6 (epi-mono factorisation generalisation) | p. 6 | Not implemented — out of scope at v0.4.0 (would require factorisation-system substrate). Forward-look candidate | ➖ N/A at v0.4.0 |
| Ex 1.7 (𝔽_N — sets + injections + surjections, `μ(1,2) = -5/2`) | p. 6 | Not implemented (rational-valued; non-Cor-1.5; needs Thm 1.6 substrate) | ➖ N/A at v0.4.0 |
| Def 1.10 (weighting `Σ_b ζ(a,b) k^b = 1`) | p. 8 | `magnitude::weighting<Q>` over the metric-magnitude path; the Cor 1.5 column `μ(-, ⊤)` recovers it on terminal-object categories | 🔗 IN v0.2.0 (different ring) |
| Ex 1.11(c) (terminal object ⊤ ⇒ `δ(-, ⊤)` is a weighting) | p. 8 | `tests/mobius_chains_exact.rs::cor_1_5_terminal_object_recursion` exercises a 4-object terminal-⊤ fixture via `verify_mobius_recursion` | ✅ DONE |

### Algorithm verification (Cor 1.5 mathematical statement)

**Paper:** `μ(a, b) = Σ_{n≥0} (−1)^n · #{nondegenerate n-paths from a to b}`.

**Implementation** (`mobius_chains.rs:444-518`): `μ = Σ_{k=0}^K (−1)^k M^k`, where `M = ζ − I`.

The two are equivalent **as matrices** because `(M^k)[i][j]` counts walks of length `k` in the non-identity arrow graph (composition law of matrix multiplication on adjacency matrices), and "walks in the non-identity arrow graph" is exactly Leinster's "nondegenerate paths" — Leinster's `nondegenerate` (no `f_i` is an identity, §1 paragraph after Lemma 1.3) coincides with "no edge in the path is a diagonal-zeta self-loop" because the diagonal of `M = ζ − I` is forced to `0` on the identity-only-endo restriction. The `k = 0` term contributes the identity matrix (empty paths exist only at `a = b`). Conclusion: the matrix-power realisation is the entry-wise Cor 1.5 formula, paper-faithful.

### Prop 2.10 (nilpotency / circuit-freeness) audit

The Leinster 2008 paper has multiple "Prop 2.10" candidates depending on edition/numbering. The implementation's rustdoc cites "Prop 2.10" as the termination guarantee — that on circuit-free 𝔸, `M^k = 0` for `k ≥ |𝔸|`. **The actual paper anchor for this termination claim is Cor 1.5 itself** (the nondegenerate-path count is bounded by the longest chain in the non-identity arrow graph, which is `≤ |𝔸| − 1` on a circuit-free DAG; this is implicit in the paper's framing where the sum `Σ_{n≥0}` collapses to a finite sum on circuit-free 𝔸). The paper does not need a separate Prop 2.10 — the termination falls out of nondegeneracy + circuit-freeness directly. The implementation's "Prop 2.10" rustdoc citation is therefore **slightly misattributed** but mathematically correct; see Important I-1 below.

### 5 fixture paper-faithfulness

| Fixture | Paper anchor | μ-value verification | Status |
|---|---|---|---|
| `cor_1_5_chain_3_linear_poset` | Phil Hall classical Möbius (p. 6 commentary) — chain 0 < 1 < 2 has `μ(i,j) = (-1)^{j-i}` for `j-i ∈ {0,1}`, else `0` | Direct entry-by-entry assertion of all 6 upper-triangular entries (`μ[0][0]=1`, `μ[0][1]=-1`, `μ[0][2]=0`, `μ[1][1]=1`, `μ[1][2]=-1`, `μ[2][2]=1`) plus lower-triangle zeros | ✅ Paper-exact |
| `cor_1_5_diamond_lattice_2_squared` | Rota classical Möbius on Boolean lattice 2² (Cor 1.5 + Ex 1.2(a)): `μ(⊥,⊤) = (-1)^|⊤|` | Direct entry assertions: `μ(⊥,⊤) = 1`, `μ(⊥,atom) = -1` for both atoms, `μ(atom,⊤) = -1`, identity diagonal | ✅ Paper-exact |
| `cor_1_5_n_path_inj_2` | Ex 1.2(c) `𝔻^inj_N`, 3-object slice `ζ(a,b) = C(b,a)` | Closed-form would be `μ(a,b) = (−1)^{b−a} · C(b,a)` (Ex 1.2(c) for injections); test uses `verify_mobius_recursion` (algebraic μ · ζ = I check). **No entry-by-entry assertion** — see Architectural A-1 below | ✅ Algebraic-only |
| `cor_1_5_terminal_object_recursion` | Ex 1.11(c) — terminal-⊤ ⇒ `δ(-,⊤)` is a weighting; column `μ(-,⊤)` recovers the weights | `verify_mobius_recursion` algebraic check | ✅ Algebraic-only |
| `cor_1_5_single_object` | Trivial: 1-object 𝔸, ζ = (1), μ = (1) | Direct: `μ[0][0] = 1` | ✅ Paper-exact |

`verify_mobius_recursion` checks only `μ · ζ = I` (left identity). On finite skeletal 𝔸 the incidence algebra `R(𝔸)` is finite-dimensional as a vector space over ℚ (and as a finitely-generated free ℤ-module on the implemented Cor 1.5 specialisation), so one-sided inverse ⇒ two-sided inverse (Def 1.1 explicit: "By finite-dimensionality, either one implies the other", p. 4). **One-sided check is algebraically sufficient** — see I-2 below for the rustdoc citation strengthening.

### Findings

**Blocking** — none. The algorithm, fixtures, and assertions are all paper-faithful at Cor 1.5; `verify_mobius_recursion` correctly invokes the finite-dimensionality argument from Def 1.1 (one-sided suffices). No blocker.

**Important**

- **I-1.** `mobius_chains.rs:401-402, 492-494` cite **"Prop 2.10"** as the termination guarantee. The paper has no Prop 2.10 in §1 (the only Prop 2.10 is in §2 / Euler characteristic context, page 10+ — about graph Euler characteristic vs category Euler characteristic, which is the wrong claim for this code). The termination is actually implicit in Cor 1.5 + circuit-freeness (the nondegenerate-path count vanishes once `n ≥ |𝔸|` on a DAG; bounded above by the longest chain length). **Recommended fix**: replace "Prop 2.10" rustdoc citations in `mobius_chains.rs` with "Cor 1.5 (implicit termination: nondegenerate path count vanishes for `n ≥ |𝔸|` on circuit-free 𝔸)" or "Lemma 1.3(c) + Cor 1.5". `poset_category.rs:7-8` and the example file's docstring inherit the same misattribution.
- **I-2.** `verify_mobius_recursion` rustdoc at `mobius_chains.rs:520-527` correctly mentions one-sided sufficiency but does not anchor the claim to **Def 1.1 (page 4)**: *"By finite-dimensionality, either one implies the other"*. Add this anchor explicitly: `"Per Leinster 2008 Def 1.1 (p. 4), in the finite-dimensional incidence algebra `R(𝔸)`, one-sided inverse implies two-sided inverse — so `μ · ζ = I` (left) is algebraically sufficient."`

**Minor**

- **M-1.** The `Q::from_i64(zeta[i][j] as i64)` cast at `mobius_chains.rs:472` + the duplicate at `:572` performs a `u64 → i64` cast that wraps silently at counts ≥ 2⁶³ (correctly documented as a Caveat in the rustdoc at `:419-424`). The fixture `cor_1_5_n_path_inj_2` uses arrow counts `{0, 1, 2, 3}`, far below the wrap boundary, so this is dormant. Folded forward to v0.4.0 forward-look §1.31 already (per the rustdoc) — no action needed beyond confirming the §1.31 entry exists in the forward-look doc.
- **M-2.** `poset_category.rs:4-9` cite "Cor 1.5 (page 6)" — page 6 is correct for arXiv:0610260v1; future versions of the paper might reflow. Consider citing the arXiv version explicitly: `"per Leinster 2008 (arXiv:0610260v1) Cor 1.5, p. 6"`.
- **M-3.** `examples/integer_mobius.rs:13-14` cites "Cor 1.5" but the docstring header (`":: μ = Σ_{n≥0} (−1)^n Mⁿ` with `M = ζ − I`") uses `Mⁿ` (Unicode superscript-n) while the source code uses `M^k` (caret-k). Both are correct; minor stylistic inconsistency. The test file at `:11-13` uses `M^k`. Pick one and propagate.
- **M-4.** `tests/mobius_chains_exact.rs::cor_1_5_diamond_lattice_2_squared` line comment at `:48` says *"Rota classical Möbius on 2² Boolean lattice"*. The Cor 1.5 fixture is the categorical Möbius, which Cor 1.5 + Ex 1.2(a) prove **equals** the Rota classical Möbius on a poset. Comment is correct as written; consider adding a forward citation: `"(Cor 1.5 specialises to Rota's classical poset Möbius via Ex 1.2(a))"`.

**Architectural** (fold to v0.4.0 forward-look §1.32+ as the v0.4.0-shipped slot — these are NOT v0.4.0 blockers)

- **A-1.** `cor_1_5_n_path_inj_2` uses **only algebraic μ · ζ = I**, no entry-by-entry assertion. Ex 1.2(c) gives a closed form `μ(a,b) = (−1)^{b−a} · C(b,a)` for injections (read off the paper at p. 4); the 3-object slice expected μ is `[[1,-2,3],[0,1,-3],[0,0,1]]`. Adding direct entry assertions would strengthen the fixture from "algebraic regression test" to "paper-exact value test". Recommend folding to v0.5.0 as a Minor enhancement (low risk, ~10 LOC added test scope).
- **A-2.** `verify_mobius_recursion` checks only `μ · ζ = I` (left). The paper proves left ⇔ right via finite-dimensionality (Def 1.1); adding a `verify_mobius_recursion_bidirectional` that checks both directions would be a defensive regression guard against future changes to `mobius_function_via_chains_exact` that might break right-identity while preserving left-identity (e.g., if the algorithm were ever specialised to lower-triangular ζ). Fold to v0.5.0 forward-look as nice-to-have.
- **A-3.** Thm 1.6 (epi-mono factorisation generalisation, p. 6) + Ex 1.7 (𝔽_N — sets + injections + surjections, μ(1,2) = -5/2) are paper-tracked but unimplemented at v0.4.0 since they require a factorisation-system substrate (likely lives in `catgraph-applied` as a future addition). Closed-form rationals like `-5/2` would also require a `Q: Ring + Integer + Div` widening, which the cleaner path is via `mobius_function::<F64Rig>` on the metric-magnitude path. Fold to v0.5.0+ forward-look (research-track).

### Audit-doc updates applied

Augmented this file (BV25-AUDIT.md) — added §"v0.4.0 §1.17 Leinster 2008 Cor 1.5 paper-audit" section (this section). **No separate `Leinster-2008-AUDIT.md` created**: Cor 1.5 §1.17 is a single integer-Möbius surface inside the magnitude-anchored crate, and the BV25 + LS2017 + Leinster-2013 + Leinster-2008 papers form a single coherent magnitude-of-categories thesis. Maintaining four sibling audit docs would inflate maintenance surface without separation-of-concerns payoff. The §1.17 row should be flipped from ⏭️ DEFERRED → ✅ DONE in the v0.4.0 Phase G CHANGELOG ride-along.

### Verdict

**APPROVE** with the I-1 + I-2 important findings as ride-along candidates for Phase G:

- I-1 (Prop 2.10 misattribution → Cor 1.5 implicit termination) is a citation precision fix touching 3 rustdoc sites + 1 example docstring. Ride-along scope: ~6 LOC of rustdoc churn.
- I-2 (`verify_mobius_recursion` Def 1.1 finite-dim anchor) is a 2-3 line rustdoc strengthening.

Both are pure documentation fixes; neither affects compiled behaviour. Algorithm, fixtures, and `μ · ζ = I` recursion are all paper-faithful at Cor 1.5; the v0.4.0 §1.17 substrate is mathematically sound and ready to ship. Minors + Architectural items fold forward per release rule 7 standard triage.
