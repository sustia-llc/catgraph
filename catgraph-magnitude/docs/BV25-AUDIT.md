# Bradley-Vigneaux 2025 Coverage Audit

> **Paper:** Bradley & Vigneaux, *The Magnitude of Categories of Texts Enriched by Language Models* ([arXiv:2501.06662v2](https://arxiv.org/abs/2501.06662), Jan 2025)
> **Library:** `catgraph-magnitude` (workspace member of `catgraph`)
> **Anchors:** reconciled against the cached paper text + PDF (2026-07-13). BV25 uses a Documenta/TAC **shared subsection/environment counter** — cite the *environment* number (e.g. Example 2.13, Definition 2.14), not a "§2.N" subsection.
> **Method:** cross-checked each numbered definition / proposition / remark / equation against `catgraph-magnitude` source and the enrichment substrate re-exported from `catgraph-applied`.
>
> **Anchor papers** (PDFs are not kept in-tree; fetch from arXiv):
> - BV 2025 — Bradley & Vigneaux, *The Magnitude of Categories of Texts Enriched by Language Models*, [arXiv:2501.06662v2](https://arxiv.org/abs/2501.06662).
> - Leinster 2013 — Leinster, *The magnitude of metric spaces*, [arXiv:1012.5857v3](https://arxiv.org/abs/1012.5857).
> - Leinster–Shulman 2017 — *Magnitude homology of enriched categories and metric spaces*, [arXiv:1711.00802v4](https://arxiv.org/abs/1711.00802).
> - Leinster 2008 — *The Euler characteristic of a category*, [arXiv:math/0610260v1](https://arxiv.org/abs/math/0610260).
> - BTV 2021 — Bradley, Terilla, Vlassopoulos, *An Enriched Category Theory of Language*, [arXiv:2106.07890](https://arxiv.org/abs/2106.07890).
>
> **Background references** (smooth-manifold / spectral magnitude —
> not implementation anchors): Leinster–Meckes 2016, *The
> Magnitude of a Metric Space: From Category Theory to Geometric Measure Theory*,
> [arXiv:1606.00095v2](https://arxiv.org/abs/1606.00095);
> Gimperlein–Goffeng–Louca, *The Magnitude and Spectral Geometry*,
> [arXiv:2201.11363v3](https://arxiv.org/abs/2201.11363).
>
> **Note on scope:** BV 2025 is anchored in §2 (enriched categories of language models) and §3 (magnitude via Tsallis q-entropy). Only §2 Defs/Eqs that materialize as runtime types and §3 Props that constitute the BV 2025 acceptance gate are tracked here. Categorical foundations (§1, §4) live in `catgraph-applied`'s `enriched.rs` + `lawvere_metric.rs` and are audited by [`catgraph-applied/docs/FS18-AUDIT.md`](../../catgraph-applied/docs/FS18-AUDIT.md).
>
> **Relationship to catgraph core audit:** The Lawvere metric / `-ln π` embedding (§2.5–2.7) is implemented in `catgraph-applied::lawvere_metric` and consumed here via `WeightedCospan::into_metric_space`. See [`catgraph/docs/FS19-AUDIT.md`](../../catgraph/docs/FS19-AUDIT.md) for the cospan substrate underlying `WeightedCospan`.
>
> **Companion paper:** Bradley, Terilla, Vlassopoulos, *An Enriched Category Theory of Language* (BTV 2021, arXiv:2106.07890) — extension to substitution-grammar enrichment. Its Yoneda semantic embedding and `[0,1]` coalition enrichment ship in this crate (`yoneda` / `coalition`, #19–#23); the downstream coalition consumer is the private koalisi repo.

**Status legend:**
- ✅ DONE — implemented and tested
- ⚠️ PARTIAL — implementation exists but does not fully exhibit the paper's structure
- ⏭️ DEFERRED — planned for a later milestone
- ➖ N/A — theoretical / motivational, no implementation expected
- 🔗 IN APPLIED — implemented in `catgraph-applied` (re-exported by this crate); noted for completeness

## Summary

| Section | DONE | PARTIAL | DEFERRED | N/A | IN APPLIED | Total |
|---|---|---|---|---|---|---|
| §1.1 (co)weighting primitives | 1 | 0 | 0 | 0 | 0 | 1 |
| §2 LM as enriched category | 5 | 0 | 0 | 1 | 3 | 9 |
| §2.1 scatteredness predicate | 1 | 0 | 0 | 0 | 0 | 1 |
| §3 Magnitude via Tsallis | 7 | 0 | 0 | 1 | 0 | 8 |
| §3.5 Möbius / chain-sum | 2 | 0 | 0 | 0 | 0 | 2 |
| §3.14 Magnitude homology | 1 | 0 | 0 | 0 | 0 | 1 |
| §4 Bounds + asymptotics | 3 | 0 | 0 | 2 | 0 | 5 |
| **TOTAL** | **20** | **0** | **0** | **4** | **3** | **27** |

**Headline numbers:**
- **74% DONE / 0% PARTIAL / 0% DEFERRED / 15% N/A / 11% IN APPLIED**
- Of the 27 audited items, 3 are already in `catgraph-applied` (enrichment substrate), 4 are N/A (3 motivational + the §3 acyclicity standing hypothesis, whose runtime enforcement is audited at §2.17), leaving **20 implementable items** of which **20 are DONE, 0 PARTIAL, 0 DEFERRED**.
- Of implementable items: **100% DONE / 0% PARTIAL / 0% DEFERRED**
- No paper-anchored audit item remains deferred: the §3 Tsallis-side optimization stash (#37) is a performance-backlog item, out of the paper-item audit scope (only §2/§3 Defs/Props/Eqs that materialize as types or constitute the acceptance gate are tracked — see the scope note above), not a tracked deferral. The magnitude-homology / chain-complex / Storjohann SNF / Euler-char-identity stack closes the §3.14 deferral. The design-doc §3.6 surface row `mobius_function_via_chains_exact<Q: Ring>` was struck from that stack and folded into the Leinster 2008 Cor 1.5 integer-exact Möbius surface (documented below) — the paper-faithful destination requires anchoring a NEW paper (Leinster 2008 finite-category Möbius) outside the crate's BV/LS/Leinster-2013 anchor surface. Both now ship in the migrated tree.

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
| 2.13 Example: Lawvere metric space `([0,∞], ≥, +, 0)` | 🔗 | `catgraph-applied::lawvere_metric::LawvereMetricSpace` | Triangle-inequality verifier + `-ln π` embedding (Lawvere 1973). Re-exported from this crate. (BV25 uses a shared subsection/environment counter — this is Example **2.13**, not a subsection "§2.5".) |
| 2.14 Definition: V-enriched category over the monoidal preorder | 🔗 | `catgraph-applied::enriched::EnrichedCategory<V>` | `LawvereMetricSpace<T>` provides a diagonal default of `Tropical::one()`. (Definition **2.14**, not "§2.6".) |
| §2.7 `HomMap<O, V>` finite realization | 🔗 | `catgraph-applied::enriched::HomMap` | Finite materialization used by `LmCategory::magnitude`. |
| §2.8 Probability cospan over alphabet | ✅ | `weighted_cospan::WeightedCospan<Λ, Q>` | v0.1.0 newtype over `catgraph::Cospan<Λ>` carrying per-edge rig weights. Type aliases `ProbCospan<Λ>` / `TropCospan<Λ>`. |
| §2.9 Probability → distance via `-ln π` | ✅ | `WeightedCospan::into_metric_space` (`Q = UnitInterval`) | v0.1.0 specialization. v0.1.1 adds `into_validated_metric_space` with full O(n³) triangle-inequality scan. |
| §2.10–2.17 Prefix-extension semantics | ✅ | `lm_category::LmCategory` | Materialized BYO-LM transition table. Forward BFS multiplies edge probabilities along directed paths; `d(x, y) = -ln π(y\|x)` recorded per Eq 6. |
| Identity axiom `d(x, x) = 0` | ✅ | `LmCategory::magnitude` (internal) | Enforced before Möbius inversion. `LawvereMetricSpace::hom` diagonal default also returns `Tropical::one()` at `a == b`. |
| §2.17 Acyclicity hypothesis | ✅ | `LmCategory::add_transition` (v0.1.1) | v0.1.1 rejects non-trivial self-loops (`from == to && prob > 0.0`) at insert time. Cycle-via-path forbidden by BV 2025 §3 acyclicity hypothesis but not detected ahead of `magnitude(t)` (BFS cap surfaces it). |

### §2.1 Scatteredness (anchored at Leinster 2013 Def 2.1.2; convergence precondition for chain-sum Möbius)

| Item | Status | Location | Notes |
|---|---|---|---|
| Leinster Def 2.1.2: `is_scattered(space)` ⇔ `d(a,b) > log(#A−1)` for all distinct `a, b` | ✅ | `magnitude::is_scattered` | v0.2.0. Vacuous for `n ≤ 1`; unset (`+∞`) distances auto-pass. Used by `mobius_chains::mobius_function_via_chains` as a precondition before the truncated von-Neumann series accumulator runs. |

### §3 Magnitude via Tsallis q-entropy

| Item | Status | Location | Notes |
|---|---|---|---|
| Tsallis q-entropy `H_t(p) = (1 - Σ pᵢᵗ) / (t-1)` (stated **unnumbered** in BV25 — not "Eq (4)"; Eq (4) is a `π`-normalization step in the Prop 2.9 proof) | ✅ | `magnitude::tsallis_entropy(p, t)` | Shannon special case at `\|t-1\| < TSALLIS_SHANNON_EPS = 1e-6` returns `-Σ pᵢ ln pᵢ` directly (avoids `0/0` cancellation around `t = 1`). |
| Rem 3.11 Shannon recovery as `t → 1` | ✅ | `magnitude::tsallis_entropy` + acceptance test | Acceptance residual `~6.46e-10` by central FD `h = 1e-4` on 4-state LM. |
| Prop 3.10 closed form `Mag(tM) = (t-1)·Σ H_t(p_x) + #(T(⊥))` | ✅ | `tests/bv_2025_acceptance.rs` | v0.1.0 acceptance residual `0e0` (exact `f64`) at `t ∈ {0.5, 1.5, 2.0, 5.0}` on hand-computed 4-state tree (`A = {a}, N = 1`; `#T(⊥) = 2`). |
| Acyclicity hypothesis (tree-shaped prefix poset) | ➖ | `LmCategory` runtime contract | Fixture rebuilt from cyclic to a 4-state acyclic prefix poset. **Note:** acyclicity is a *prose* standing hypothesis in BV25 §3, not a numbered result — "3.4" is the *Example* that an initial object gives magnitude 1, a different statement. Marked N/A as a hypothesis (not an implementable item); its runtime enforcement is audited at the §2.17 row above. |
| §3.5 Eq 7 magnitude as Möbius sum `Mag = Σᵢⱼ μ[i][j]` | ✅ | `magnitude::magnitude::<Q>(space, t)` | Builds t-scaled zeta, Möbius-inverts, sums every entry. Algebraic surface `Q: Ring + Div + From<f64>`. |
| §3.5 Möbius inversion `ζ·μ = I` | ✅ | `magnitude::mobius_function::<Q>(space)` | Gaussian elimination on `[ζ \| I]` augmented matrix. `Err(CatgraphError::Composition)` on singular zeta. v0.1.0 limit ~1000 states (O(n³)). |
| §3.5 Chain-sum Möbius (Leinster 2013 Prop 2.1.3) | ✅ | `mobius_chains::mobius_function_via_chains` | v0.2.0. Implemented as the von-Neumann series `μ = Σ (−1)ᵏ Mᵏ` with `M = ζ − I` (algebraically identical to Prop 2.1.3's chain-sum-of-ζ-products by Mᵏ[a][b] = chain-sum at length k). O(K · n³) matrix-power accumulation with adaptive K = ⌈log(τ) / log(r)⌉, τ = 1e-13, capped at K_MAX = 200. Bound `Q: Ring + From<f64>` (no `Div` needed). Acceptance: chain-sum agrees with v0.1.x `mobius_function` to 1e-9 on hand-built 4-state + proptest n=2-5. Returns `Err` on non-scattered or near-boundary (r ≥ 0.94) input — caller falls back to `mobius_function`. |
| §3.6 Numerical scoping `TSALLIS_SHANNON_EPS = 1e-6` | ✅ | `lib.rs` | Public constant; threshold for special-case branch and lower bound on Rem 3.11 finite-difference step. |

### §3.14 Magnitude homology Euler-characteristic identity

| Item | Status | Location | Notes |
|---|---|---|---|
| BV 2025 Prop 3.14 (page 21): `Mag(tM) = Σ_ℓ q^ℓ · Σ_{k≥0} (−1)ᵏ · rank(H_{k,ℓ}(M))`, `q = e^(−t)` | ✅ | `chain_complex::euler_char_identity_at` + `chain_complex::magnitude_homology_rank` + `snf::*` (Storjohann port). **NOTE:** the numerical comparator is `crate::magnitude::magnitude` (matrix-inverse Möbius), NOT `mobius_chains::chain_count_signed_graded` — the latter is a per-grade chain-count diagnostic without `q^ℓ` weighting. | Headline closing result of BV 2025; mirror of Leinster–Shulman 2017 Theorem 3.5 / Cor 7.15 (the metric-space specialisation directly used here; the LS 2021 statement is the parametric `Q((q^ℝ))` formal-series analogue). Implementation t-pre-scales the space by `t` so the weight `q^ℓ_orig = e^(−t · ℓ_orig)` collapses to `e^(−ℓ_scaled)` in pre-scaled coordinates. Implementation: (a) Leinster–Shulman 2017 §3 (Def 3.3) chain complex via `Chain` + `enumerate_chains` DFS + `ChainIndex` (length-graded basis materialisation) + `boundary_matrix<Q>` (alternating face map drop-one-vertex); (b) length-grading by `ℓ = Σ d(a_{j−1}, a_j)` via `ChainIndex::grades()` per LS 2017 §3 (Def 3.3–3.4); (c) integer Smith normal form for `rank(H_{k,ℓ})` via custom Storjohann §7 port (`snf::{zmod, echelon, band}` + `phase_1_to_bidiagonal` + `diagonal_to_smith` + `bidiagonal_to_smith` fused 9-step + top-level `smith_normal_form`) over `MatR<Q>` with single-prime + 2-prime cross-check rank recovery (Mersenne `2^31 − 1` primary). **Algorithmic reference + dev-only oracle:** [`events555/modularsnf`](https://github.com/events555/modularsnf) (Apache-2.0); workspace stays ndarray-free (option (c) custom port over `MatR<Q>` per design doc §2.4, NOT a runtime dep). **Acceptance gate (path C):** `euler_char_identity_at(space, t, max_degree)` returns `(via_homology, via_magnitude)` agreeing within an analytical residual bound `\|Δ\| ≤ n · r^(max_deg+1) / (1−r) + 1e-9` where `r = (n−1) · exp(−d_min_scaled)`. Tests: 5 fixtures pass (release suite 35.5s). |

### §4 Bounds + asymptotics

| Item | Status | Location | Notes |
|---|---|---|---|
| Intro bounds `#T(⊥) ≤ Mag(tM) ≤ #ob(M)` for `t ≥ 1` | ✅ | `tests/lm_category.rs` proptest | Bounds proptest on randomly generated forward-chain LMs (size 2–4). **Un-numbered intro prose** (p.4), derivable from Prop 3.10 — not a numbered "Eq 4.3". |
| Asymptotic `Mag(t·M) → #T(⊥)` as `t → ∞` (deterministic) | ✅ | `examples/lm_magnitude.rs` | Asserted at `t = 1e6`. v0.1.0 caveat documented: the `t → ∞` limit equals `#T(⊥)` only for fully-deterministic LMs (all-Dirac rows); for non-degenerate rows it is `#T(⊥) + #{non-degenerate non-terminal states}`. |
| Monotonicity of `Mag` in `t` for `t ≥ 1` | ✅ | `examples/lm_magnitude.rs` | Property (C) asserted. |
| §4 Information-theoretic interpretation | ➖ | — | motivational; covered narratively in `README.md`. |
| §4 Connection to BTV 2021 grammar enrichment | ➖ | — | Forward-pointer; the BTV Yoneda embedding ships here (`yoneda`, #19), substitution-grammar enrichment is a downstream (koalisi) concern. |

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

## Performance baseline

`mag_lm/<N>` criterion median wall-clock (optimized, `--quick`, acyclic forward-chain LM at `t = 2.0`):

| N | Median |
|---|---|
| 10 | ~30 µs |
| 100 | ~11 ms |
| 1000 | ~11 s |

O(n³) Gaussian elimination dominates above n ≈ 100. 1000-state is the practical limit for the v0.1.x dense-matrix Möbius implementation; v0.2.0 chain-sum Möbius would lift this for sparse / `Tropical` regimes.

---

## Out of scope

- **`Tropical`-valued / `BoolRig`-valued magnitude.** Per Leinster 2013 §1.3 Examples 1.3.1, the scalar rig `k` is **determined by the enriching V**, not free at the call site: `V = [0,∞] ⇒ k = ℝ` (the LM-magnitude setting), `V = FinSet ⇒ k = ℚ`, `V = 2 (Boolean) ⇒ k = ℤ`, `V = FDVect ⇒ k = ℚ`. There is no paper-aligned notion of magnitude valued in `Tropical` or `BoolRig` for our V = Lawvere `[0,∞]` setting. An earlier internal spec's framing (`SignTwist: Rig` trait + `negate_at_parity` for `Tropical` / `BoolRig`) was a misattribution that doesn't trace back to BV 2025, Leinster 2013, or Leinster–Shulman 2017. The §1.4 final-paragraph "formal magnitude in `R(A)`" path (free k-algebra containing a weighting + coweighting) is the only paper-aligned route to magnitude across rigs without a fixed k; it is deferred indefinitely until a downstream consumer surfaces a concrete need.
- **Agent transport** (SurrealDB `RELATE`, tokio live-queries) — a downstream (koalisi) concern, not in this crate.
- **BTV 2021 substitution-grammar enrichment** — a downstream (koalisi) concern; the BTV Yoneda embedding itself ships here (`yoneda`, #19).
- **Yoneda copresheaves for role discovery** — now ships in-crate (`yoneda` / `semantic`, #19/#21); independent of the runtime layer.
- ~~**BV 2025 Prop 3.14 magnitude-homology Euler-characteristic identity** — deferred~~ — **SHIPPED** via the magnitude-homology / chain-complex / Storjohann SNF / Euler-char-identity stack. See the §3.14 row above.
- **`mobius_function_via_chains_exact<Q: Ring>`** — **shipped** in the migrated tree. Originally planned alongside the magnitude-homology stack, struck after a spec-tension surfaced (the `Q: Ring` bound is incompatible with mirroring the chain-sum body, which requires `Q: Ring + From<f64>`). The paper-faithful `Q: Ring + ZAlgebra` bound (originally `Q: Ring + Integer`) required a new paper anchor (Leinster 2008 finite-category Möbius), a new input type (`PosetCategory<NodeId>`), and a Z-ring substrate.

---

## Leinster 2008 Cor 1.5 paper-audit

> **Paper:** Leinster, *The Euler characteristic of a category* ([arXiv:math/0610260v1](https://arxiv.org/abs/math/0610260), 8 Oct 2006 — published Documenta Math 2008; ~263 KB PDF via arXiv, the primary cold-read anchor).
> **Method:** end-to-end re-read of §1 (Möbius inversion, pages 4-9); cross-walked Cor 1.5 (page 6), Prop 2.10 (referenced in §1 Def 1.1 commentary on Möbius vs graph Möbius), and Ex 1.11(c) (terminal object weighting) against the shipped code (now at sustia-llc/catgraph; re-verified post-reboot).

### Paper-to-code cross-walk

| Paper item | Page | Crate surface | Status |
|---|---|---|---|
| Def 1.1 (incidence algebra `R(𝔸)`, zeta `ζ_𝔸`, Möbius `μ_𝔸 = ζ⁻¹`) | p. 4 | `PosetCategory::{zeta_matrix, zeta}` (`u64` arrow counts; `Q::from_i64` lift to ring at use site) | ✅ DONE |
| Ex 1.2(a) (poset Möbius by induction, `μ(a,a) = 1`, `μ(a,c) = -Σ_{a≤b<c} μ(a,b)`) | p. 4 | `verify_mobius_recursion` checks the equivalent `μ · ζ = I` algebraically | ✅ DONE |
| Ex 1.2(c) (`𝔻^inj_N`, `ζ(a,b) = C(b,a)`) | p. 4 | `tests/mobius_chains_exact.rs::cor_1_5_n_path_inj_2` fixture (3-object slice) | ✅ DONE |
| Lemma 1.3 (equivalent conditions: idempotents-are-identities ⇔ endomorphisms-are-automorphisms ⇔ circuits-are-isomorphisms) | p. 5 | `PosetCategory::from_arrow_counts` enforces (b) ∧ (c) directly: diagonal `ζ[i][i] ∈ {0, 1}` rules out non-identity endos at the level of arrow counts; `has_cycle` DFS rules out non-identity circuits. Skeletality is implicit (object list is the index) | ✅ DONE |
| Thm 1.4 (Möbius inversion exists on finite skeletal 𝔸 with identity-only idempotents; closed form with `\|Aut(a_i)\|` denominators) | p. 5 | The `\|Aut\|` denominators degenerate to 1 under the stronger Cor 1.5 hypothesis (identity-only endomorphisms ⇒ trivial automorphism group at every object). The implementation specialises to this case | 🔗 SPECIALISED to Cor 1.5 |
| **Cor 1.5** (finite skeletal 𝔸 with identity-only endomorphisms ⇒ `μ(a,b) = Σ_{n≥0} (−1)^n · #{nondeg n-paths a → b} ∈ ℤ`) | **p. 6** | **`mobius_chains::mobius_function_via_chains_exact<N, Q>` over `Q: Ring + ZAlgebra`** | ✅ DONE |
| Cor 1.5 termination (implicit: nondeg path count vanishes once `n > #(non-identity arrows on any chain from a to b)`) | p. 6 | Algorithm early-terminates when `M^k` becomes the zero matrix (the `all_zero` flag in the loop body); bounded above by `k = n = \|objects\|` per the `for _k in 1..=n` ceiling | ✅ DONE |
| Phil Hall classical formula (Prop 3.8.5 of Stanley; Prop 6 of [R]) on a poset | p. 6 (commentary) | `tests/mobius_chains_exact.rs::cor_1_5_chain_3_linear_poset` recovers Phil Hall on the 3-chain | ✅ DONE |
| Thm 1.6 (epi-mono factorisation generalisation) | p. 6 | Not implemented — out of scope (would require factorisation-system substrate). Forward-look candidate | ➖ N/A |
| Ex 1.7 (𝔽_N — sets + injections + surjections, `μ(1,2) = -5/2`) | p. 6 | Not implemented (rational-valued; non-Cor-1.5; needs Thm 1.6 substrate) | ➖ N/A |
| Def 1.10 (weighting `Σ_b ζ(a,b) k^b = 1`) | p. 8 | `magnitude::weighting<Q>` over the metric-magnitude path; the Cor 1.5 column `μ(-, ⊤)` recovers it on terminal-object categories | 🔗 IN-CRATE (different ring) |
| Ex 1.11(c) (terminal object ⊤ ⇒ `δ(-, ⊤)` is a weighting) | p. 8 | `tests/mobius_chains_exact.rs::cor_1_5_terminal_object_recursion` exercises a 4-object terminal-⊤ fixture via `verify_mobius_recursion` | ✅ DONE |

### Algorithm verification (Cor 1.5 mathematical statement)

**Paper:** `μ(a, b) = Σ_{n≥0} (−1)^n · #{nondegenerate n-paths from a to b}`.

**Implementation** (`mobius_chains.rs:444-518`): `μ = Σ_{k=0}^K (−1)^k M^k`, where `M = ζ − I`.

The two are equivalent **as matrices** because `(M^k)[i][j]` counts walks of length `k` in the non-identity arrow graph (composition law of matrix multiplication on adjacency matrices), and "walks in the non-identity arrow graph" is exactly Leinster's "nondegenerate paths" — Leinster's `nondegenerate` (no `f_i` is an identity, §1 paragraph after Lemma 1.3) coincides with "no edge in the path is a diagonal-zeta self-loop" because the diagonal of `M = ζ − I` is forced to `0` on the identity-only-endo restriction. The `k = 0` term contributes the identity matrix (empty paths exist only at `a = b`). Conclusion: the matrix-power realisation is the entry-wise Cor 1.5 formula, paper-faithful.

### Prop 2.10 (nilpotency / circuit-freeness) audit

The Leinster 2008 paper has **exactly one** Prop 2.10 (arXiv:math/0610260v1, §2): "Let `G` be a finite circuit-free directed graph. Then `χ(F(G))` is defined and equal to `|G₀| − |G₁|`." That is a real, distinct result (the free-category Euler characteristic of a circuit-free graph), *not* the per-`𝔸` termination guarantee. An earlier revision's rustdoc cited "Prop 2.10" as the termination guarantee — that on circuit-free 𝔸, `M^k = 0` for `k ≥ |𝔸|`. **The actual paper anchor for that termination claim is Cor 1.5 itself** (the nondegenerate-path count is bounded by the longest chain in the non-identity arrow graph, which is `≤ |𝔸| − 1` on a circuit-free DAG; this is implicit in the paper's framing where the sum `Σ_{n≥0}` collapses to a finite sum on circuit-free 𝔸). The paper does not need a separate proposition for it — the termination falls out of nondegeneracy + circuit-freeness directly. The current `mobius_chains.rs` rustdoc anchors termination on Cor 1.5 (+ Lemma 1.3), so no live code cites Prop 2.10; the conclusion (termination ⇐ Cor 1.5, mathematically correct) stands. See Important I-1 below.

### 5 fixture paper-faithfulness

| Fixture | Paper anchor | μ-value verification | Status |
|---|---|---|---|
| `cor_1_5_chain_3_linear_poset` | Phil Hall classical Möbius (p. 6 commentary) — chain 0 < 1 < 2 has `μ(i,j) = (-1)^{j-i}` for `j-i ∈ {0,1}`, else `0` | Direct entry-by-entry assertion of all 6 upper-triangular entries (`μ[0][0]=1`, `μ[0][1]=-1`, `μ[0][2]=0`, `μ[1][1]=1`, `μ[1][2]=-1`, `μ[2][2]=1`) plus lower-triangle zeros | ✅ Paper-exact |
| `cor_1_5_diamond_lattice_2_squared` | Rota classical Möbius on Boolean lattice 2² (Cor 1.5 + Ex 1.2(a)): `μ(⊥,⊤) = (-1)^|⊤|` | Direct entry assertions: `μ(⊥,⊤) = 1`, `μ(⊥,atom) = -1` for both atoms, `μ(atom,⊤) = -1`, identity diagonal | ✅ Paper-exact |
| `cor_1_5_n_path_inj_2` | Ex 1.2(c) `𝔻^inj_N`, 3-object slice `ζ(a,b) = C(b,a)` | Closed-form would be `μ(a,b) = (−1)^{b−a} · C(b,a)` (Ex 1.2(c) for injections); test uses `verify_mobius_recursion` (algebraic μ · ζ = I check). **No entry-by-entry assertion** — see Architectural A-1 below | ✅ Algebraic-only |
| `cor_1_5_terminal_object_recursion` | Ex 1.11(c) — terminal-⊤ ⇒ `δ(-,⊤)` is a weighting; column `μ(-,⊤)` recovers the weights | `verify_mobius_recursion` algebraic check | ✅ Algebraic-only |
| `cor_1_5_single_object` | Trivial: 1-object 𝔸, ζ = (1), μ = (1) | Direct: `μ[0][0] = 1` | ✅ Paper-exact |

`verify_mobius_recursion` checks only `μ · ζ = I` (left identity). On finite skeletal 𝔸 the incidence algebra `R(𝔸)` is finite-dimensional as a vector space over ℚ (and as a finitely-generated free ℤ-module on the implemented Cor 1.5 specialisation), so one-sided inverse ⇒ two-sided inverse (Def 1.1 explicit: "By finite-dimensionality, either one implies the other", p. 4). **One-sided check is algebraically sufficient** — see I-2 below for the rustdoc citation strengthening.

