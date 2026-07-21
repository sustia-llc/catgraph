# Bradley-Terilla-Vlassopoulos 2021 Coverage Audit

> **Paper:** Bradley, Terilla & Vlassopoulos, *An Enriched Category Theory of
> Language: From Syntax to Semantics* ([arXiv:2106.07890v2](https://arxiv.org/abs/2106.07890), 18 Nov 2021)
> **Library:** `catgraph-magnitude` (workspace member of `catgraph`)
> **Provenance:** ported 2026-07-21 from the archived `tsondru/catgraph-coalition`
> `docs/BTV21-AUDIT.md`, then re-expressed against what `catgraph-magnitude`
> actually shipped in #19–#23 (Yoneda / determinism / semantic clustering /
> coalition) plus the corpus-MLE constructor landing on branch
> `feat/53-btv21-salvage` (#53). The legacy audit tracked a coalition crate that
> no longer exists; every status row here is re-verified against the migrated
> tree, and every paper claim against the cached v2 text — not transcribed.
> **Companion:** [`BV25-AUDIT.md`](BV25-AUDIT.md) — Bradley–Vigneaux 2025 magnitude
> audit; BTV 2021 is BV25's syntax/semantics precursor (the `-ln π` embedding and
> `[0,1]` enrichment are shared).
> **Tracking issue:** [#53](https://github.com/sustia-llc/catgraph/issues/53)
> (salvage the un-re-expressed BTV21 surfaces).
>
> **Section map** (paper's own numbering — the legacy audit drifted here; see the
> corrections at the end): §1 Introduction (§1.1 Compositionality, §1.2 Why
> category theory, §1.3 Constructions in the unenriched setting); §2 Enriched
> category theory (§2.1 Categories enriched over [0,1], §2.2 The syntax category
> L); §3 Enriched copresheaves (§3.1 [0,1]-enriched Yoneda, §3.2 The semantic
> category L̂); §4 Enriched products and coproducts in L̂ (§4.1–§4.4); §5 A metric
> space interpretation (§5.1 Tropical module structure); §6 Conclusion.
>
> **Scope note:** the legacy crate was the *coalition* host; that role moved to the
> private `koalisi` repo. The magnitude-relevant BTV21 surfaces (the Yoneda
> semantic embedding, the `[0,1]` internal hom, the corpus estimator) are
> re-expressed here first-class; the syntax-side production layer and the weighted
> (co)limit / semi-tropical-module algebra are tracked as deferred work
> (§3.2 / §4 / §5.1 rows below), not ported verbatim.

**Status legend:**
- ✅ DONE — implemented and tested in `catgraph-magnitude`
- ⚠️ PARTIAL — implementation exists but does not fully exhibit the paper's structure
- ⏭️ DEFERRED — tracked for a later milestone / research track (issue linked)
- ➖ N/A — motivational, proof-layer, or deliberately not re-expressed (with reason)
- 🔗 IN APPLIED — lives in `catgraph-applied` (enrichment / rig substrate), noted for completeness

## Summary

| Section | DONE | PARTIAL | DEFERRED | N/A | IN APPLIED | Total |
|---|---|---|---|---|---|---|
| §1 Introduction | 0 | 0 | 1 | 2 | 0 | 3 |
| §2.1 Categories enriched over [0,1] | 1 | 0 | 0 | 0 | 2 | 3 |
| §2.2 The syntax category L | 3 | 0 | 0 | 0 | 0 | 3 |
| §3 Enriched copresheaves | 4 | 0 | 0 | 2 | 0 | 6 |
| §3.1 The [0,1]-enriched Yoneda lemma | 1 | 0 | 0 | 2 | 0 | 3 |
| §3.2 The semantic category L̂ | 2 | 0 | 1 | 0 | 0 | 3 |
| §4 Weighted products and coproducts | 0 | 0 | 4 | 0 | 0 | 4 |
| §5 A metric space interpretation | 2 | 0 | 1 | 0 | 1 | 4 |
| §5.1 Tropical module structure | 0 | 0 | 1 | 0 | 1 | 2 |
| §6 Conclusion | 0 | 0 | 0 | 1 | 0 | 1 |
| **TOTAL** | **13** | **0** | **8** | **7** | **4** | **32** |

**Headline numbers:**
- **41% DONE / 0% PARTIAL / 25% DEFERRED / 22% N/A / 13% IN APPLIED**
- Of the 32 audited items: 4 live in `catgraph-applied` (the enrichment / Tropical
  substrate re-exported here), 7 are N/A (5 motivational or proof-layer, 2 abstract
  enriched-functor setup whose concrete instance is the representable copresheaf).
  That leaves **21 implementable items**, of which **13 are DONE and 8 DEFERRED**.
- The 8 deferrals are all *algebra above the semantics gate*: the unenriched
  (co)limit constructions (§1.3, one item), the weighted (co)limit calculus
  (§4, four items), the M̂ weighted (co)product (§5), the semi-tropical module
  action (§5.1, Thm 6), and the L̂-as-a-category syntax-side layer (§3.2).
  The §1.3/§4 (co)limits + the Thm 6 module action are folded into
  [#36](https://github.com/sustia-llc/catgraph/issues/36); the L̂ syntax layer is
  [#53](https://github.com/sustia-llc/catgraph/issues/53) item 2 (research track).
- The **load-bearing** syntax-category surface (Def 4 + Eq 8) and the **semantic**
  surface (the Yoneda embedding + the L̂ / M̂ hom) are DONE: the syntax→semantics
  arc that gives the paper its title ships end-to-end.

---

## Per-section detail

### §1 Introduction

Motivation for modelling language as a `[0,1]`-enriched category and passing to
copresheaves for semantics. The unenriched constructions (§1.3) are re-run in the
enriched setting downstream (§4), so nothing implementable lands *here*.

| Item | Status | Location | Notes |
|---|---|---|---|
| §1.1 Compositionality — algebraic + distributional (Harris 1954) picture of meaning | ➖ | — | Pedagogical framing; the crate encodes both operationally without recapitulating the philosophy. |
| §1.2 Why category theory — thin subtext category, unenriched Yoneda `x ↦ hom(x,−)` | ➖ | — | Motivational; the *enriched* Yoneda embedding that carries the actual content ships at §3.1 / §3.2 (`yoneda`). |
| §1.3 Constructions in the unenriched setting — coproduct / product / cartesian-closed internal hom | ⏭️ | — | The enriched analogues are §4; deferred with §4 into [#36](https://github.com/sustia-llc/catgraph/issues/36). |

### §2.1 Categories enriched over [0,1]

Defs 1–3 (commutative monoidal preorder, V-enriched category, closed structure)
and Lemma 1 (the unit interval is closed, internal hom = truncated division
`[a,b] = min{1, b/a}`, Eq 6–7). The abstract enrichment machinery lives in
`catgraph-applied`; the `[0,1]` internal hom is exercised directly here.

| Item | Status | Location | Notes |
|---|---|---|---|
| Def 1 — commutative monoidal preorder `(V, ≤, ⊗, 1)` (Eq 2 monotonicity) | 🔗 | `catgraph-applied::enriched` | `[0,1]` is the `UnitInterval` instance; `[0,∞]` the `Tropical` instance. |
| Def 2 — V-enriched category, identity `1 ≤ C(x,x)` (Eq 3) + lax composition `C(y,z) ⊗ C(x,y) ≤ C(x,z)` (Eq 4) | 🔗 | `catgraph-applied::{enriched::EnrichedCategory, lawvere_metric::LawvereMetricSpace}` | Re-exported and consumed by `LmCategory`; diagonal defaults to `Tropical::one()`. |
| Def 3 + Lemma 1 + Eq 5–7 — closed structure, internal hom `[a,b] = min{1, b/a}` on `[0,1]` | ✅ | `yoneda::semantic_hom` | The truncated-division internal hom is applied pointwise inside the L̂ hom `inf_c min{1, g(c)/f(c)}`; verified in `yoneda` unit tests. |

### §2.2 The syntax category L

**Load-bearing.** Def 4: `L(x,y) := π(y\|x)`, the probability that `y` extends
`x` (`0` if `x` is not a subtext of `y`). Eq 8: the chain-rule *equality*
`π(z\|y)·π(y\|x) = π(z\|x)` — the paper's structural claim (satisfies Eq 3/4 *with
equalities*, verified at v2 lines 504–510).

| Item | Status | Location | Notes |
|---|---|---|---|
| Def 4 — `L(x,y) = π(y\|x)` syntax category (BYO-LM transition table) | ✅ | `lm_category::LmCategory`, `weighted_cospan::WeightedCospan` | The transition table *is* `L(x,y)`; `WeightedCospan` carries per-edge `[0,1]` weights. |
| Def 4 — corpus-MLE constructor (prefix-state MLE `π(p·t\|p) = N(p·t)/N(p)`, full-history prefix trie — *not* an order-1 bigram model) | ✅ | `lm_category::LmCategory::from_traces` | **Lands in this change** ([#53](https://github.com/sustia-llc/catgraph/issues/53) item 1): closes the "where do the probabilities come from" gap for real text fixtures. Anti-target from the legacy round-3 verdict carried: BTV21 has *no* production-rule taxonomy — do not invent a terminal/non-terminal split. |
| Eq 8 — chain-rule equality `π(z\|y)·π(y\|x) = π(z\|x)` | ✅ | `lm_category::LmCategory::from_traces` (prefix-tree tables) | Holds by construction **on the tree-shaped tables `from_traces` produces** (unique path ⇒ `-ln` additivity is exact, and the count MLE telescopes: `N(z)/N(y)·N(y)/N(x) = N(z)/N(x)`). On general BYO-LM tables `enriched_space` computes a *max-probability-path* relaxation (its rustdoc's DAG-with-rejoin caveat), so Eq 8 is a bound, not an equality, there. |

### §3 Enriched copresheaves

Def 5 (enriched functor, Eq 9), Def 6 (enriched copresheaf `f: C → V`), Eq 10
(the end defining the functor category), Lemma 2 + Eq 11 (the copresheaf hom
`Ĉ(f,g) = inf_c min{1, gc/fc}`), Lemma 3 + Def 7 (representable copresheaf
`h^x := C(x,−)`). The representable copresheaf and its hom are the shipped
semantic surface; the general enriched-functor / end setup is abstract.

| Item | Status | Location | Notes |
|---|---|---|---|
| Def 5 — enriched functor `C → D` (Eq 9) | ➖ | — | Abstract setup; the only instance the crate needs is the representable copresheaf (Lemma 3), realized below. |
| Def 6 — enriched copresheaf `f: C → V` | ✅ | `yoneda::Copresheaf` | A materialized row of the LM hom-space, read in probability form via `π = exp(−d)`. |
| Eq 10 — end / enriched functor category `D^C` | ➖ | — | Theoretical; collapses to Lemma 2 for `V = [0,1]`. |
| Lemma 2 + Eq 11 — copresheaf hom `Ĉ(f,g) = inf_c min{1, gc/fc}` | ✅ | `yoneda::semantic_hom` / `semantic_distance` | The asymmetric BTV hom — **pinned, not re-derived** (private crate rule). `inf` over shared contexts; vacuous case returns `1.0`. |
| Lemma 3 — `h^x := C(x,−)` is a `[0,1]`-functor | ✅ | `yoneda::Copresheaf` (construction) | Every row of the LM space is a representable copresheaf. |
| Def 7 — representable copresheaf | ✅ | `lm_category::LmCategory::{yoneda, yoneda_all}` (#19) | `x ↦ L(x,−)`; the batch form powers `semantic` clustering. |

### §3.1 The [0,1]-enriched Yoneda lemma

Theorem 1 (`Ĉ(h^x, f) = f(x)`) and Corollaries 1–2 (`C(y,x) = Ĉ(h^x, h^y)`;
`x ↦ h^x` embeds `C^op` into `Ĉ`). The embedding is the shipped artifact; the
lemma and Cor 1 are the identities that justify it.

| Item | Status | Location | Notes |
|---|---|---|---|
| Theorem 1 — enriched Yoneda `Ĉ(h^x, f) = f(x)` | ➖ | — | Proof-layer identity; not a runtime surface. |
| Cor 1 — `C(y,x) = Ĉ(h^x, h^y)` | ➖ | — | Proof-layer identity; consumed implicitly by the `semantic_hom` inf. |
| Cor 2 — `x ↦ h^x` embeds `C^op` into `Ĉ` (the Yoneda embedding = meaning-of-x) | ✅ | `yoneda` (#19), `semantic` (#21) | "The meaning of a text *is* its representable copresheaf." The embedding + its consumer (semantic ranking / clustering) both ship. |

### §3.2 The semantic category L̂

Def 8: `L̂ := [0,1]^L`, the `[0,1]`-category of enriched copresheaves on `L`.
Eq 12: `h^x(c) = π(c\|x)` if `x ≤ c`, else `0`. The copresheaves and their hom
ship; the L̂-as-a-first-class-category syntax-side layer is research-track.

| Item | Status | Location | Notes |
|---|---|---|---|
| Def 8 — L̂ semantic hom (the `[0,1]`-category structure on copresheaves) | ✅ | `yoneda::semantic_hom` / `semantic_distance`, `semantic` (#21) | The hom object between meanings ships; `semantic` groups whole texts by it. |
| Eq 12 — representable copresheaf `h^x(c) = π(c\|x)` on the principal ideal of `x` | ✅ | `yoneda::Copresheaf` (a row of the LM space) | Support = extensions of `x`; unreachable contexts map to `0` (infinite distance). |
| Def 8 — L̂ as a first-class category object + syntax-side layer (`GrammarPort` / L̂ hook) | ⏭️ | — | [#53](https://github.com/sustia-llc/catgraph/issues/53) item 2 (research track). Carries the ratified anti-target: no production-rule taxonomy. |

### §4 Weighted products and coproducts

§4.1 Def 9 (weighted limit/colimit, Eq 13–16); §4.2 Def 10 + Eq 17–19 + Lemma 4
(weighted product `min{fc/w₁, gc/w₂, 1}`); §4.3 Def 11 + Theorem 3 + Eq 20
(weighted coproduct `max{w₁·fc, w₂·gc}`); §4.4 Def 12 + Lemma 5 + Def 13 +
Theorem 4 + Eq 21 (enriched implication `[f,g]`, `x ⇒ y`). None re-expressed in
`catgraph-magnitude`; the weighted (co)limit calculus is folded into
[#36](https://github.com/sustia-llc/catgraph/issues/36).

| Item | Status | Location | Notes |
|---|---|---|---|
| §4.1 Def 9 — V-weighted limit / colimit (Eq 13–16) | ⏭️ | — | [#36](https://github.com/sustia-llc/catgraph/issues/36). |
| §4.2 Def 10 + Eq 17–19 + Lemma 4 — weighted product on representables `min{fc/w₁, gc/w₂, 1}` | ⏭️ | — | [#36](https://github.com/sustia-llc/catgraph/issues/36). (The legacy cited a "Theorem 2" here — no such theorem exists; see corrections.) |
| §4.3 Def 11 + Theorem 3 + Eq 20 — weighted coproduct `max{w₁·fc, w₂·gc}` | ⏭️ | — | [#36](https://github.com/sustia-llc/catgraph/issues/36). |
| §4.4 Def 12 + Lemma 5 + Def 13 + Theorem 4 + Eq 21 — enriched implication `[f,g]`, `x ⇒ y` | ⏭️ | — | Research track; the entailment surface has no consumer yet. |

### §5 A metric space interpretation

`−ln: [0,1] → [0,∞]` is an iso of commutative monoidal preorders; the syntax
category `L` maps to a generalized (Lawvere) metric space `M`, `M(x,y) = −ln
L(x,y)`, internal hom `[a,b] = max{b−a, 0}`. Theorem 5 gives the weighted
(co)product in `M̂`. This is the bridge into BV25 magnitude.

| Item | Status | Location | Notes |
|---|---|---|---|
| `−ln` iso `[0,1] ≅ [0,∞]` + Tropical `(min,+)` rig | 🔗 | `catgraph-applied::{rig::Tropical, lawvere_metric}` | The iso and the semi-tropical rig live in `applied`; re-exported here. |
| Generalized metric space `M(x,y) = −ln π`, triangle inequality (both inequalities are equalities, inherited from Eq 8) | ✅ | `weighted_cospan::WeightedCospan::{into_metric_space, into_validated_metric_space}`, `lm_category::LmCategory::enriched_space` | `into_validated_metric_space` runs the full `O(n³)` triangle-inequality scan (BTV 2021 §5). |
| `M̂` copresheaf distance `d̂(f,g) = sup_c max{gc − fc, 0}` | ✅ | `yoneda::semantic_distance` | The `−ln` image of the L̂ hom; `semantic_distance_sym` is the labelled symmetric wrapper for clustering. |
| Theorem 5 — weighted product/coproduct in `M̂` (`max{fc−w₁, gc−w₂, 0}` / `min{fc+w₁, gc+w₂}`) | ⏭️ | — | [#36](https://github.com/sustia-llc/catgraph/issues/36); the `−ln` image of the §4 constructions. |

### §5.1 Tropical module structure

Def 14: the tropical semi-ring `((−∞,∞], ⊕=min, ⊙=+)`; the sub-semi-ring `[0,∞]`
is the semi-tropical semi-ring. Theorem 6: the coproduct (trivial weights) makes
`M̂` a commutative monoid, and `(s ⊙ f)(x) = f(x) + s` makes it a semi-tropical
module.

| Item | Status | Location | Notes |
|---|---|---|---|
| Def 14 — tropical / semi-tropical semi-ring `([0,∞], min, +)` | 🔗 | `catgraph-applied::rig::Tropical` | `Tropical::zero() = +∞` (⊕-identity), `Tropical::one() = 0` (⊙-identity). |
| Theorem 6 — `M̂` as a semi-tropical module, action `(s ⊙ f)(x) = f(x) + s` | ⏭️ | — | [#36](https://github.com/sustia-llc/catgraph/issues/36). The non-Cartesian module action is the natural home for a token-carrying `FreeMonoidQ`; the §5.1 "ReLU networks compute tropical rational maps" remark is inspiration, not a theorem to implement. |

### §6 Conclusion

§6.1 names future directions (studying LMs via representable copresheaves,
operations on trained-LM parameters, gender-neutral pronoun via coproduct of
representables, knowledge graphs from internal homs, the density-operator approach
[BV20], and the tropical-module ↔ ReLU-network bridge).

| Item | Status | Location | Notes |
|---|---|---|---|
| §6.1 Applications and future directions (six topics) | ➖ | — | Out of scope / downstream. The representable-copresheaf-as-meaning direction is the one already realized in-crate (`yoneda` / `semantic`); the rest are research or `koalisi`/`catgraph-dl` concerns. |

---

## Legacy corrections and dropped bookkeeping

Re-expressing the archived `catgraph-coalition` audit surfaced these drifts and
scope changes:

1. **Section drift — Def 4 / Eq 8.** The legacy header and §2.2 body cited "§3 Def
   4" and "§3 Eq 8". Both are in **§2.2** (paper v2 lines 493–510); Def 8 (the
   semantic category L̂) is the §3.2 result. Corrected throughout.
2. **Phantom "Theorem 2".** The legacy §4.2 row cited "Definition 10 + Theorem 2"
   for the weighted product. **There is no Theorem 2 in the paper** — the weighted
   product is Def 10 + Eq 17–19 (+ Lemma 4). The only numbered §4 theorems are
   Theorem 3 (coproduct) and Theorem 4 (implication). Corrected.
3. **Phantom "§1.4" (already fixed upstream).** `weighted_cospan.rs:12` once cited
   a non-existent "§1.4" for the `−ln` embedding; PR #120 re-anchored it to
   **§5**, matching the paper (the `−ln` metric interpretation is §5, v2 lines
   1260–1264). This audit cites §5 accordingly.
4. **Coalition module names replaced.** Every legacy surface pointer
   (`enriched::corpus`, `LanguageSignature`, `PortPair`, `EnrichedCoalition`,
   `build_lawvere_space`) named the archived crate. Rows now point at the shipped
   magnitude surfaces (`lm_category`, `weighted_cospan`, `yoneda`, `semantic`,
   `coalition`) verified in the migrated tree.
5. **Release-ladder bookkeeping dropped.** The legacy tracked status as a
   `v0.2.x → v0.3.0 → v0.4.0 → v0.5.0 → v0.6.0` coalition-release ladder and a
   per-task "v0.3.0 plan-mapping" table. Those version gates are meaningless for
   `catgraph-magnitude`; status is now DONE / DEFERRED against real issues.
6. **Ratified verdict carried (anti-target).** The one load-bearing round-3
   verdict — *BTV21 has no production-rule taxonomy; do not invent a parser-like
   terminal/non-terminal split* — is carried on the §2.2 and §3.2 rows. The rest
   of the legacy's "§16.2" design-doc verdicts were coalition-internal and dropped.
7. **Surfaces re-homed, not ported.** `FreeMonoidQ<Token>` / the §5.1 Theorem 6
   module action → folded into [#36](https://github.com/sustia-llc/catgraph/issues/36);
   `magnitude_history` (temporal trajectory) → `koalisi`. Marked deferred / out of
   scope with links rather than carried as coalition rows.

## Cross-references

- BTV 2021 paper: [arXiv:2106.07890v2](https://arxiv.org/abs/2106.07890) (PDF not kept in-tree).
- Companion magnitude audit: [`BV25-AUDIT.md`](BV25-AUDIT.md).
- Enrichment / Lawvere-metric substrate audit: [`catgraph-applied/docs/FS18-AUDIT.md`](../../catgraph-applied/docs/FS18-AUDIT.md).
- Cospan substrate audit: [`catgraph/docs/FS19-AUDIT.md`](../../catgraph/docs/FS19-AUDIT.md).
- Salvage tracking: [#53](https://github.com/sustia-llc/catgraph/issues/53); folded algebra: [#36](https://github.com/sustia-llc/catgraph/issues/36).
