# Seven Sketches Coverage Audit (catgraph-applied)

> **Paper:** Fong & Spivak, *Seven Sketches in Compositionality: An Invitation to Applied Category Theory* (arXiv:1803.05316v3, 12 Oct 2018)
> **Library:** catgraph-applied (workspace member of the `sustia-llc/catgraph` workspace)
> **Date:** originally authored 2026-04 (pre-reboot); version framing reconciled 2026-07-03 to the phase/issue model per #7
> **Method:** read all 334 pages of the textbook, cross-checked each numbered definition/theorem/example against catgraph-applied source and catgraph core
>
> **Note on scope:** *Seven Sketches* is a 334-page textbook covering seven topics in applied CT. Only **Chapters 4, 5, and 6** contain formal content relevant to catgraph-applied's modules. Chapters 1–3 (orders, enrichment, databases) and Chapter 7 (toposes) establish foundational CT that catgraph core already provides or that is outside catgraph's scope entirely.
>
> **Relationship to catgraph core audit:** The core catgraph crate tracks Fong & Spivak's *Hypergraph Categories* (arXiv:1806.08304v3, 2019) — the research paper that formalizes the §6.3 content into a full equivalence theorem. See [`catgraph/docs/FS19-AUDIT.md`](../../catgraph/docs/FS19-AUDIT.md) for the core audit. This audit covers the *textbook* content that goes beyond that paper: decorated cospans, operads and their algebras, props, signal flow, and wiring diagrams for monoidal/compact-closed/hypergraph categories.
>
> **Provenance.** This audit was authored pre-reboot; its version framing has been reconciled to the reboot's phase/issue model per #7 (the per-crate version lineage and release-history ceremony are retired — work is tracked by reboot phases and GitHub issues/PRs against `sustia-llc/catgraph`). The *coverage content* below was spot-checked current post-reboot: Phase 2 (#1/#6) re-substrated `Rig` onto `deep_causality_num` and added `mat_kron` (Ex 2.16) + `trace`, reflected inline.

**Status legend:**
- ✅ DONE — implemented and tested
- ⚠️ PARTIAL — implementation exists but is incomplete or doesn't fully exhibit the paper's structure
- ❌ MISSING — not implemented in catgraph-applied (or catgraph core)
- ➖ N/A — theoretical / motivational / pedagogical, no implementation expected
- 🔗 IN CORE — implemented in catgraph core (not catgraph-applied); noted for completeness

## Summary

| Chapter/Section | DONE | PARTIAL | MISSING | N/A | IN CORE | Total |
|---|---|---|---|---|---|---|
| §4.4 Categorification + monoidal cats | 5 | 0 | 0 | 2 | 2 | 9 |
| §4.5 Compact closed categories | 0 | 0 | 0 | 2 | 3 | 5 |
| §5.2 Props and presentations | 4 | 0 | 0 | 3 | 0 | 7 |
| §5.3 Signal flow graphs | 5 | 0 | 0 | 1 | 0 | 6 |
| §5.4 Graphical linear algebra | 1 | 0 | 1 | 1 | 0 | 3 |
| §6.2 Colimits and connection | 0 | 0 | 0 | 2 | 4 | 6 |
| §6.3 Hypergraph categories | 2 | 0 | 0 | 2 | 6 | 10 |
| §6.4 Decorated cospans | 4 | 0 | 1 | 1 | 0 | 6 |
| §6.5 Operads and their algebras | 5 | 2 | 0 | 1 | 0 | 8 |
| **TOTAL** | **26** | **2** | **2** | **15** | **15** | **60** |

**Headline numbers:**
- **43% DONE / 3% PARTIAL / 3% MISSING / 25% N/A / 25% IN CORE**
- Of the 60 audited items, 15 are already in catgraph core (the research paper's content), 15 are N/A (pedagogical), leaving **30 implementable items** of which **26 are DONE, 2 PARTIAL, 2 MISSING**.
- Of implementable items: **87% DONE / 7% PARTIAL / 7% MISSING**
- §5.4 Thm 5.60 is closed via the opt-in `Presentation::eq_mod_functorial<MatrixNFFunctor<R>>` semantic engine — a complete decision procedure for `Free(Σ_SFG)/⟨E_{17}⟩ ≅ Mat(R)` by Baez-Erbele 2015. The default syntactic (CC) engine remains incomplete by design; see the §5.4 Thm 5.60 row for the measured Option-A improvement and the Mat(R) completeness resolution (resolved: functorial-terminal (#15); KB feasibility spike #57).
- §4.4 carries 3 enriched-category rows (EnrichedCategory, HomMap, LawvereMetricSpace) and the congruence-closure decision procedure as the default `eq_mod` backend.
- SFG_R, Mat(R), the `sfg_to_mat` functor, `Presentation`, Thm 5.60, and Corel are all closed; §6.3 Ex 6.64 Corel is closed in catgraph core.

---

## Per-section detail

### §4.4 Categorification (pp. 132–139)

| Item | Status | Location | Notes |
|---|---|---|---|
| Rough Def 4.45: symmetric monoidal category | 🔗 | catgraph::monoidal | `Monoidal` + `SymmetricMonoidalMorphism` traits |
| Remark 4.46: strict SMC (Mac Lane coherence) | 🔗 | catgraph core design | catgraph works in the strict case |
| Remark 4.47: non-rough definition reference | ➖ | — | theoretical pointer |
| Ex 4.49: (Set, {1}, ×) monoidal structure | ➖ | — | motivational example |
| Ex 4.50: wiring diagram for monoidal composition | ✅ | catgraph-applied::wiring_diagram | `WiringDiagram` implements `Composable` + `Monoidal` for exactly this diagram interpretation |
| Rough Def 4.51: V-category (enriched in SMC) | ✅ | catgraph-applied::enriched | See enriched-category rows below. |
| V-enriched category | ✅ | catgraph-applied::enriched::EnrichedCategory | Trait over `V: Rig`. F&S §1.1, §2.4; CTFP Ch 28. |
| Lawvere metric space | ✅ | catgraph-applied::lawvere_metric::LawvereMetricSpace | Concrete impl over `Tropical`. Triangle-inequality verifier + `-ln π` embedding from `UnitInterval`. |
| HomMap finite realization | ✅ | catgraph-applied::enriched::HomMap | Concrete trait realization. Used for testing + the catgraph-magnitude LmCategory construction (sibling crate, Phase 3). |

### §4.5 Profunctors form a compact closed category (pp. 139–146)

| Item | Status | Location | Notes |
|---|---|---|---|
| Def 4.58: dual, unit η, counit ε, snake equations | 🔗 | catgraph::compact_closed | cup/cap functions, zigzag tests |
| Prop 4.60: compact closed ⟹ monoidal closed | 🔗 | catgraph core (implicit) | catgraph relies on this via Prop 6.66 |
| Ex 4.61: Corel as compact closed category | 🔗 | catgraph::span::Rel | `Rel` exists; corelation structure implicit |
| Thm 4.63: Prof_V is compact closed | ➖ | — | theoretical; profunctor categories not implemented |
| Ex 4.66: snake equations for Prof_V | ➖ | — | theoretical verification |

### §5.2 Props and presentations (pp. 149–158)

| Item | Status | Location | Notes |
|---|---|---|---|
| Def 5.2: prop (symmetric strict monoidal category, Ob = ℕ) | ✅ | catgraph-applied::prop | `PropExpr<G>` arity-tracked expression tree with `Composable<Vec<()>>`, `Monoidal`, `HasIdentity`, `SymmetricMonoidalMorphism<()>` impls. |
| Def 5.11: prop functor | ➖ | — | definition only (operadic analogue available as `OperadFunctor` for Rough Def 6.98) |
| Def 5.13: (m,n)-port graph | ⚠️ | catgraph-applied::petri_net | `PetriNet` is a bipartite graph with typed ports; not literally a port graph but structurally adjacent. `WiringDiagram` inner/outer circles are closer. |
| Def 5.25: free prop on a signature Free(G) | ✅ | catgraph-applied::prop::Free | `Free<G>::{identity, braid, generator, compose, tensor}` smart constructors on `PropExpr<G>`, arity-checked at construction time. SMC-axiom quotient (`Presentation::normalize` with 9-rule canonical form). |
| Def 5.30: G-generated prop expressions | ✅ | catgraph-applied::prop::PropExpr + prop::presentation | `PropExpr<G>` realises the syntactic layer (Identity/Braid/Generator/Compose/Tensor); `Presentation::normalize` applies the 9-rule SMC canonical form (interchange, unitors, braiding naturality, identity-coherence of ⊗). Note: the quotient uses bounded structural rewriting; the congruence-closure backend is the default `eq_mod` engine, and Knuth-Bendix completion of the SMC-coherence rewriting is the #57 feasibility spike (Mat(R) completeness resolved functorial-terminal, #15). |
| Rough Def 5.33: presentation (G, s, t, E) for a prop | ✅ | catgraph-applied::prop::presentation::Presentation | `Presentation<G>` with `add_equation`, `normalize`, `eq_mod`, `with_depth`. 9-rule SMC canonical form applied first; user-supplied equations then applied left-to-right. |
| Remark 5.34: universal property of presentations | ➖ | — | theoretical |
| Prop 5.29: universal property of Free(G) | ➖ | — | theoretical |

### §5.3 Simplified signal flow graphs (pp. 159–168)

| Item | Status | Location | Notes |
|---|---|---|---|
| Def 5.36: rig (semiring) | ✅ | catgraph-applied::rig | `Rig` trait (blanket impl over `deep_causality_num::{Zero,One}` + Add + Mul; re-substrated from the `num` crate in Phase 2 #1 — `Rig` stays a native semiring, never a DC `Ring`) + 4 concrete instances: `BoolRig` (∨,∧), `UnitInterval` ([0,1] Viterbi), `Tropical` ([0,∞], min, +), `F64Rig`. `verify_rig_axioms` + `BaseChange<UnitInterval>` for `Tropical`. |
| Def 5.45: SFG_R = Free(G_R) (signal flow graphs as free prop) | ✅ | catgraph-applied::sfg | `SignalFlowGraph<R>` with 5 primitive generators from Eq 5.52 (Copy 1→2, Discard 1→0, Add 2→1, Zero 0→1, Scalar(r) 1→1) plus derived `copy_n`/`discard_n`. |
| Def 5.50: Mat(R) prop of R-matrices | ✅ | catgraph-applied::mat | `MatR<R>` pure-rig matrix prop. F&S convention: morphism m→n is m×n matrix. Composable/Monoidal/SymmetricMonoidalMorphism over any `Rig`; block_diagonal tensor. `mat_f64` nalgebra bridge behind opt-in `f64-rig` feature. |
| Thm 5.53: prop functor S: SFG_R → Mat(R) | ✅ | catgraph-applied::sfg_to_mat | `sfg_to_mat` structural recursion over `PropExpr<SfgGenerator<R>>`; generator table matches Eq 5.52 exactly. Functoriality (S(f∘g) = S(f)·S(g), S(f⊗g) = S(f)⊕S(g)) verified on all 4 rigs via 13 integration tests. |
| Prop 5.54: matrix S(g) describes input→output amplification | ✅ | catgraph-applied::sfg_to_mat (implicit) | Implicitly verified by Thm 5.53 functoriality tests; the generator matrices are exact per Eq 5.52. No standalone test. |
| Eq 5.52: generator → matrix table (copy, discard, add, zero, scalar) | ✅ | catgraph-applied::sfg_to_mat + tests/sfg_to_mat.rs | All 5 generator matrices verified in integration tests across BoolRig, UnitInterval, Tropical, F64Rig. |

### §5.4 Graphical linear algebra (pp. 168–178)

| Item | Status | Location | Notes |
|---|---|---|---|
| Thm 5.60: presentation of Mat(R) from Frobenius + rig equations | ✅ | catgraph-applied::graphical_linalg + prop::presentation::functorial | **DONE** via the semantic Functorial engine. `matr_presentation<R>` builds all 16 equations from F&S p.170 (Groups A cocomonoid, B monoid, C bialgebra, D scalar — D1/D3/D4/D5/D6 instantiated for `rig_samples`). Decision procedure: `Presentation::eq_mod_functorial(a, b, &MatrixNFFunctor::<R>::new())` — complete by F&S Thm 5.53 + Baez-Erbele 2015 (the isomorphism `Free(Σ_SFG)/⟨E_{17}⟩ ≅ Mat(R)` is realised by `sfg_to_mat`). **Syntactic (CC) decision remains incomplete by design** — Option A (atom-canonical `smc_refine` in `kb::CongruenceClosure`) plus the post-#14 layer-ordering NF reduce BoolRig d=2 collisions 2574 → 1433 → 1301 (~49% total) but cannot reach zero; the terminal Mat(R) decision path is the Functorial engine (resolved: functorial-terminal, #15; KB feasibility spike #57). The 12 integration tests in `tests/graphical_linalg.rs` were renamed `cc_completeness_tracking_*` (from `thm_5_60_faithful_*`) to reflect that they diagnose CC incompleteness vs the matrix ground truth — they are NOT Thm 5.60 verification (Baez-Erbele proved that abstractly). They stay `#[ignore]`'d as a diagnostic, not a release gate. See the CHANGELOG entry. |
| Def 5.65: monoid object in SMC (commutative monoid axioms) | ❌ | — | catgraph has `FrobeniusOperation` (monoid + comonoid) but no standalone `MonoidObject` in general SMC; deferred |
| Thm 5.87: hypergraph category from linear relations | ➖ | — | LinRel deferred (same as core audit) |

### §6.2 Colimits and connection (pp. 184–196)

| Item | Status | Location | Notes |
|---|---|---|---|
| Def 6.1: initial object | ➖ | — | pedagogical; catgraph uses ∅ as monoidal unit |
| Def 6.11: coproduct | ➖ | — | pedagogical; catgraph monoidal product is coproduct on FinSet |
| Def 6.19: pushout | 🔗 | catgraph::cospan | union-find pushout composition |
| Prop 6.32: finite colimits ⟺ initial + pushouts | 🔗 | catgraph core (implicit) | FinSet has both |
| Thm 6.37: colimit formula as equivalence classes | 🔗 | catgraph::cospan | pushout via union-find is exactly this formula |
| Def 6.43 + 6.45: cospan, Cospan_C category | 🔗 | catgraph::cospan | `Cospan<Lambda>` with pushout composition |

### §6.3 Hypergraph categories (pp. 197–203)

| Item | Status | Location | Notes |
|---|---|---|---|
| Def 6.52: Frobenius structure (μ, η, δ, ε + 9 axioms) | 🔗 | catgraph::frobenius | `FrobeniusOperation`, 8 axiom tests |
| Def 6.54: spider s_{m,n} | 🔗 | catgraph::frobenius | `from_decomposition` constructs spiders from generators |
| Thm 6.55: spider theorem (connected diagrams = spiders) | 🔗 ✅ | catgraph::frobenius + `tests/spider_theorem.rs` | Explicit tests in catgraph core — 5 tests covering s_{2,2}, s_{3,1}, s_{1,3}, s_{0,0} and connected-diagram shape via `special_frobenius_morphism` constructor |
| Thm 6.58: free prop on Frobenius ≅ Cospan_FinSet | 🔗 | catgraph::cospan_algebra + hypergraph_functor | `CospanToFrobeniusFunctor` (Prop 3.8 in the research paper) |
| Def 6.60: hypergraph category | 🔗 | catgraph::hypergraph_category | `HypergraphCategory` trait |
| Ex 6.61: Cospan_C is a hypergraph category | 🔗 | catgraph::hypergraph_category | `impl HypergraphCategory for Cospan<Lambda>` |
| Ex 6.64: Corel is a hypergraph category | ✅ | catgraph::corel | `Corel<Lambda>` type with full `HypergraphCategory<Lambda>` impl in catgraph core. See [catgraph/CHANGELOG.md](../../catgraph/CHANGELOG.md) for the coarsen-and-compose semantics. |
| Prop 6.66: hypergraph cats are self-dual compact closed | 🔗 | catgraph::compact_closed | cup/cap from η;δ and μ;ε |
| Temperley-Lieb as diagrammatic SMC (spider-theorem adjacent) | ✅ | catgraph-applied::temperley_lieb | `BrauerMorphism` composition via connected components + loop counting; TL generators e_i; Frobenius-law-adjacent relations tested (e_i² = δ·e_i, Jones relations) |

### §6.4 Decorated cospans (pp. 203–211)

| Item | Status | Location | Notes |
|---|---|---|---|
| Rough Def 6.68: symmetric monoidal functor (F, φ) | ➖ | — | theoretical; catgraph uses `HypergraphFunctor` |
| Def 6.75: F-decorated cospan | ✅ | catgraph-applied::decorated_cospan | `Decoration` trait + generic `DecoratedCospan<Lambda, D>` struct. `PetriDecoration` specializes to Petri nets; `Circuit` example specializes to EdgeSet on apex vertices. |
| Thm 6.77: Cospan_F is a hypergraph category | ✅ | catgraph-applied::decorated_cospan + petri_net | `impl HypergraphCategory<Lambda> for DecoratedCospan<Lambda, D>` realizes the theorem generically (any `D: Decoration`). `impl HypergraphCategory<Lambda> for PetriNet<Lambda>` specializes via `from_cospan`. |
| Ex 6.79–6.86: Circ functor, decorated cospan composition for circuits | ✅ | catgraph-applied::decorated_cospan + examples/decorated_cospan_circuit.rs | Parallel and series composition both demonstrated; series composition uses `Cospan::compose_with_quotient` + `D::pushforward` to coequalize the shared boundary vertex. |
| Ex 6.88: closed circuits via η;x;ε composition | ❌ | — | no closed-circuit construction |
| Petri net cospan bridge (pre/post arc weights as left/right legs) | ✅ | catgraph-applied::petri_net | `from_cospan`, `transition_as_cospan` — multiplicity-weighted cospan bridge. `fire`, `enabled`, `reachable` for state-space exploration. |

### §6.5 Operads and their algebras (pp. 211–218)

| Item | Status | Location | Notes |
|---|---|---|---|
| Rough Def 6.91: operad (types, operations, substitution ∘_i, identities) | ✅ | catgraph::operadic + catgraph-applied::e1_operad, e2_operad | `Operadic` trait in core defines substitution with identity/associativity laws. E₁ and E₂ implement concrete operads with validated substitution. |
| Ex 6.93: Set operad (functions as operations) | ➖ | — | motivational example |
| Ex 6.94: Cospan operad (cospans as operations, substitution by pushout) | ✅ | catgraph-applied::wiring_diagram | `WiringDiagram` implements `Operadic` with cospan-pushout substitution. This IS the Cospan operad specialized to named cospans with inner/outer circles. |
| Eq 6.95: wiring diagram as cospan operation | ✅ | catgraph-applied::wiring_diagram | the `Operadic::substitute` implementation literally performs this: replace an inner circle with a sub-diagram, connecting ports by name |
| Def 6.97: operad O_C underlying any SMC C | ⚠️ | catgraph::operadic (trait) | the `Operadic` trait captures the abstract interface, but there is no generic construction that takes an arbitrary SMC and produces its underlying operad |
| Rough Def 6.98: operad functor | ✅ | catgraph-applied::operad_functor | `OperadFunctor<O1, O2, Input>` trait plus concrete `E1ToE2` packaging the canonical little-intervals-into-little-disks inclusion. Literal geometric functoriality is verified by `E1ToE2::check_substitution_preserved` (comparing disks by centre/radius within f32 tolerance, modulo naming); a generic arity-level shadow helper covers any functor. |
| Def 6.99: operad algebra (F: O → Set) | ✅ | catgraph-applied::operad_algebra | Single-sorted `OperadAlgebra<O, Input>` trait generic over any `Operadic<Input>` operad; concrete `CircAlgebra` implementing F&S Ex 6.100 for `WiringDiagram` (carrier = outer-port count, verifying Ex 6.100's invariance under substitution). |
| Prop 6.101: Cospan-algebras ≅ hypergraph props | ⚠️ | catgraph::cospan_algebra + equivalence + catgraph-applied::operad_algebra | the per-Λ version (Thm 4.13 in the research paper) is verified in catgraph core. The operadic side of the equivalence (Cospan-*algebras* in the operad sense) is expressible as `OperadAlgebra<WiringDiagram, _>` instances; the `≅` itself remains a test-only consolidation task. |

---

## Critical findings

### What catgraph-applied implements well

1. **Operadic substitution (§6.5)** — `WiringDiagram` faithfully implements the Cospan operad (Ex 6.94) with name-matched port substitution. This is the textbook's primary concrete operad example. E₁ and E₂ operads demonstrate the abstract definition (Rough Def 6.91) with geometric substitution (affine rescaling).

2. **Temperley-Lieb / Brauer algebra (§6.3 adjacent)** — `BrauerMorphism` implements the diagrammatic category of perfect matchings with composition via connected components and closed-loop counting. TL generators and Jones relations are tested. This goes beyond the textbook (which mentions Frobenius diagrams and spiders but not TL specifically) into representation-theoretic territory.

3. **Petri net cospan bridge (§6.4 specialized)** — `PetriNet` implements a specific decorated cospan with multiplicity-weighted arc structure, BFS reachability, and parallel/sequential composition. The cospan bridge (`from_cospan` / `transition_as_cospan`) is well-tested.

4. **Linear combinations (§5.3 infrastructure)** — `LinearCombination<Coeffs, Target>` provides the free R-module over a basis set, with convolution product, functorial pushforward, and scalar operations. This is the algebraic infrastructure that the textbook presupposes (rigs, rings) but doesn't package as a standalone construct.

### Major gaps

1. ~~**Props and presentations (§5.2)**~~ — ✅ **CLOSED.** `Prop` type and `Free(G)` in `catgraph-applied::prop`; `Presentation<G>` with 9-rule SMC canonical form (`prop::presentation`). Def 5.30 and Def 5.33 both DONE.

2. ~~**Signal flow graphs and Mat(R) (§5.3–5.4)**~~ — ✅ **CLOSED.** `SignalFlowGraph<R>` (Def 5.45), `MatR<R>` (Def 5.50), and `sfg_to_mat` functor (Thm 5.53). Thm 5.60 closed via the opt-in `Presentation::eq_mod_functorial<MatrixNFFunctor<R>>` semantic engine — complete by Baez-Erbele 2015. The syntactic (CC) engine remains incomplete by design: CC is the default `eq_mod` backend (closing the overlapping-user-equation branch), and Option A + the Layer-1 Joyal-Street NF (through the #14 layer-ordering pass) cut BoolRig d=2 collisions 2574 → 1433 → 1301 (~49% total), but reaching zero is unnecessary — the terminal Mat(R) decision path is the Functorial engine (resolved: functorial-terminal, #15; KB feasibility spike #57). The 12 `cc_completeness_tracking_*` tests (renamed from `thm_5_60_faithful_*` to reflect what they measure) remain `#[ignore]`'d as diagnostic; the four depth-2 tests are now bounded regression trackers at the post-#14 baselines.

3. ~~**General decorated cospans (§6.4)**~~ — ✅ **CLOSED.** `Decoration` trait + `DecoratedCospan<Lambda, D>` in `catgraph-applied::decorated_cospan`. `PetriDecoration` specializes to Petri nets; `Circuit` EdgeSet example specializes to resistor circuits. `HypergraphCategory<Lambda>` realized generically (Thm 6.77). `D::pushforward` wired through `Composable::compose` via `Cospan::compose_with_quotient`; direct `PetriNet::permute_side` added.

4. ~~**Operad algebras (§6.5 Def 6.99)**~~ — ✅ **CLOSED.** Single-sorted `OperadAlgebra<O, Input>` trait in `catgraph-applied::operad_algebra`; `CircAlgebra` implementing F&S Ex 6.100 for `WiringDiagram`. Prop 6.101 (Cospan-algebras ≅ hypergraph props) — the operadic side of the equivalence is expressible; a test-only consolidation of the `≅` remains.

5. ~~**Operad functors (§6.5 Rough Def 6.98)**~~ — ✅ **CLOSED.** `OperadFunctor<O1, O2, Input>` trait with concrete `E1ToE2` packaging the canonical little-intervals-into-little-disks inclusion; literal geometric functoriality verified by comparing E₂ disk positions modulo naming.

6. ~~**Corel as hypergraph category (§6.3 Ex 6.64)**~~ — ✅ **CLOSED in catgraph core.** `Corel<Lambda>` with `HypergraphCategory<Lambda>` impl in catgraph core.

### Items intentionally deferred

- **Ch 1–3** (orders, enrichment, databases): foundational CT already in catgraph core or out of scope
- **Ch 7** (toposes, sheaves, logic): out of scope for catgraph
- **LinRel examples** (Ex 6.65, Thm 5.87): deferred per core audit decision
- **Profunctor categories** (Thm 4.63): enriched profunctors are out of catgraph's scope

### Items that are implicit / "morally present" but not explicit

1. **Thm 6.55 (spider theorem)** — ✅ **CLOSED in catgraph core.** `tests/spider_theorem.rs` asserts shape equality between connected Frobenius diagrams and the canonical spiders produced by `special_frobenius_morphism(m, n, z)`.

2. **Def 6.97 (operad underlying an SMC)** — the `Operadic` trait captures the interface but the generic *construction* that derives an operad from any SMC is not automated.

3. **Prop 6.101 (Cospan-algebras ≅ hypergraph props)** — the per-Λ equivalence (Thm 4.13 in the research paper) is verified in catgraph core. Restating it in operadic language would be a test-only task.

---

## Inheritance from catgraph core

catgraph-applied builds on catgraph's F&S 2019 primitives. The following textbook items are **already implemented in catgraph core** and available to catgraph-applied modules:

| Textbook item | catgraph core location | catgraph-applied usage |
|---|---|---|
| Def 6.19: pushout composition | `cospan.rs` (union-find) | `WiringDiagram::substitute`, `PetriNet::from_cospan` |
| Def 6.43 + 6.45: Cospan_C | `cospan.rs`, `named_cospan.rs` | `WiringDiagram` wraps `NamedCospan` |
| Def 6.52: Frobenius structure | `frobenius/operations.rs` | `BrauerMorphism` TL generators; `WiringDiagram` operadic structure |
| Def 6.60: hypergraph category | `hypergraph_category.rs` | ✅ `impl HypergraphCategory<Lambda> for PetriNet<Lambda>`; generic `impl` for `DecoratedCospan<Lambda, D>` alongside |
| Prop 6.66: self-dual compact closed | `compact_closed.rs` | `BrauerMorphism::dagger` uses compact closed structure |
| Thm 6.58: Cospan ≅ free Frobenius | `cospan_algebra.rs` | foundation for operadic substitution |
| Rough Def 4.45: SMC | `monoidal.rs` | `WiringDiagram`, `BrauerMorphism` implement `Monoidal` |

No duplication of F&S primitives in catgraph-applied — it depends on catgraph.

---

## Roadmap

### Tier 1 — ✅ shipped (catgraph core + catgraph-applied)

| Gap | Textbook ref | Status | Location |
|---|---|---|---|
| ~~Spider theorem explicit test~~ | Thm 6.55 | ✅ | `catgraph/tests/spider_theorem.rs` |
| ~~`DecoratedCospan<F>` generic type~~ | Def 6.75, Thm 6.77 | ✅ | `catgraph-applied/src/decorated_cospan.rs` |
| ~~`HypergraphCategory` impl for `PetriNet`~~ | Def 6.60 via Thm 6.77 | ✅ | `catgraph-applied/src/petri_net.rs` |

### Tier 1.1 — ✅ shipped (catgraph core + catgraph-applied)

| Gap | Source | Status | Location |
|---|---|---|---|
| ~~`Cospan::compose_with_quotient` upstream API~~ | Task 4 self-review | ✅ | `catgraph/src/cospan.rs` |
| ~~`DecoratedCospan::compose` invokes `D::pushforward`~~ | Task 4 | ✅ | `catgraph-applied/src/decorated_cospan.rs` |
| ~~`PetriNet::SymmetricMonoidalMorphism` braiding semantics~~ | Task 8 | ✅ | `catgraph-applied/src/petri_net.rs` |
| ~~`Transition::relabel` arc deduplication~~ | Task 7 | ✅ | `catgraph-applied/src/petri_net.rs` |

### Tier 2 — ✅ shipped (catgraph-applied)

| Gap | Textbook ref | Status | Location |
|---|---|---|---|
| ~~`Prop` type + `Free(G)` construction~~ | Def 5.2, Def 5.25 | ✅ | `catgraph-applied/src/prop.rs` |
| ~~`OperadAlgebra` type (F: O → Set) + Ex 6.100 Circ~~ | Def 6.99, Ex 6.100 | ✅ | `catgraph-applied/src/operad_algebra.rs` |
| ~~`OperadFunctor` type + canonical `E₁ ↪ E₂`~~ | Rough Def 6.98 | ✅ | `catgraph-applied/src/operad_functor.rs` |

### Tier 3 — ✅ shipped (catgraph core + catgraph-applied)

| Gap | Textbook ref | Status | Location |
|---|---|---|---|
| ~~Signal flow graphs (SFG_R)~~ | Def 5.45 | ✅ | `catgraph-applied/src/sfg.rs` |
| ~~Mat(R) prop + functorial semantics~~ | Def 5.50, Thm 5.53 | ✅ | `catgraph-applied/src/mat.rs` + `sfg_to_mat.rs` |
| ~~Presentation type (G, s, t, E)~~ | Def 5.33 | ✅ | `catgraph-applied/src/prop/presentation/mod.rs` |
| ~~Graphical linear algebra (Thm 5.60)~~ | §5.4.1 | ✅ | Closed via `Presentation::eq_mod_functorial<MatrixNFFunctor<R>>` — complete by Baez-Erbele 2015. 16-equation presentation + Functorial engine in `catgraph-applied/src/prop/presentation/functorial.rs`. |
| ~~Corel `HypergraphCategory` impl~~ | Ex 6.64 | ✅ | `catgraph/src/corel.rs` |

### Tier 3.1 — enrichment + CC follow-ups

| Item | Textbook ref | Notes |
|---|---|---|
| ~~`EnrichedCategory` + Lawvere metric (`UnitInterval` hom-sets)~~ | §1.3–1.4, §2.4 pedagogical anchor | ✅ **DONE.** `EnrichedCategory<V>` trait + `HomMap<O, V>` + `LawvereMetricSpace<T>` over `Tropical` with triangle-inequality verifier + `-ln π` embedding from `UnitInterval`. Unblocks the `catgraph-magnitude` sibling crate. |
| Congruence-closure `eq_mod` backend | §5.2 Def 5.33 | ✅ **DONE.** `prop::presentation::kb::CongruenceClosure` (DST 1980 signature-table variant) + `NormalizeEngine` selector on `Presentation`. Decides equality for finitely-presented equational theories without binders; closes the overlapping-user-equation branch of the Thm 5.60 faithfulness problem. |
| ~~Thm 5.60 faithfulness enumeration (SMC string-diagram normal form)~~ | §5.4.1 | ✅ **DONE** via semantic route. Layer-1 Joyal-Street NF in `prop::presentation::smc_nf` (used as the CC engine's short-circuit). `MatrixNFFunctor` (complete by Baez-Erbele 2015) is the **terminal** semantic decision path (resolved: functorial-terminal, #15; KB feasibility spike #57). Syntactic CC stays incomplete by design; the 12 renamed `cc_completeness_tracking_*` tests stay `#[ignore]`'d as diagnostic (the four depth-2 ones re-baselined as bounded regression trackers), not a release gate — equality on the Mat(R) presentation is decidable operationally via `eq_mod_functorial`. |

---

## Resolved decision: Mat(R) syntactic completeness (#15 — functorial-terminal)

See [`../CHANGELOG.md`](../CHANGELOG.md) for this crate's per-change scope and [`../../catgraph/CHANGELOG.md`](../../catgraph/CHANGELOG.md) for the cross-crate core infrastructure (e.g. `Cospan::compose_with_quotient`, which unblocked the decorated-cospan pushforward wiring, and `Corel<Lambda>` + its `HypergraphCategory` impl closing Ex 6.64).

The `MatrixNFFunctor` semantic engine already decides Mat(R) equality completely (Baez-Erbele 2015). The remaining question — whether to *also* make the syntactic CC engine complete — was a decision point tracked as **#15**, with two branches (kept here as the record of what was decided between):

- **(A)** Knuth-Bendix completion of the 17 Thm 5.60 equations modulo SMC coherence (would close the `cc_completeness_tracking_*` tests under CC and retire `smc_refine`), or
- **(B)** declare `MatrixNFFunctor` terminal for Mat(R) and leave the syntactic engine as-is.

**Decision (resolved): (B) functorial-terminal.** `Presentation::eq_mod_functorial` with `MatrixNFFunctor` is the terminal, complete decision procedure for Mat(R) (F&S Thm 5.53 / Baez-Erbele 2015). The syntactic CC engine is **incomplete by design**: it stays the default `eq_mod` backend for its role (overlapping-user-equation congruence), but it is not required to be complete for Mat(R). Knuth-Bendix completion is demoted to a time-boxed feasibility spike (**#57**), relevant only for a future non-Mat(R) presentation that lacks a semantic functor. The `cc_completeness_tracking_*` depth-2 tests were re-baselined as regression trackers at the post-#14 collision counts (BoolRig 1301, UnitInterval 1856, Tropical 2526 pinned exactly two-sided; F64 ~2777, float-nondeterministic so tracked as a jitter band `2770..=2790`, #58).

The related C2 interchange gap (**#14**) is now **closed**: `topological_layer_order` (Step 4(c)) plus mixed-layer braid isolation and identity-width-refined naturality canonicalize interchange, and the `interchange` proptest is un-ignored and gating. One narrow follow-up remains — mid-layer zero-source (η) scheduling — recorded as an ignored known-gap test.

---

## Cross-paper reconciliation: both F&S papers × all three workspace crates

This section maps every catgraph workspace module to its paper provenance (or lack thereof). Two papers are tracked:

- **[FS19]** = Fong & Spivak, *Hypergraph Categories* (arXiv:1806.08304v3, 2019) — tracked by [`catgraph/docs/FS19-AUDIT.md`](../../catgraph/docs/FS19-AUDIT.md)
- **[FS18]** = Fong & Spivak, *Seven Sketches in Compositionality* (arXiv:1803.05316v3, 2018) — tracked by this document

### catgraph core (Phase 1) — all modules anchored to [FS19]

| Module | [FS19] ref | [FS18] ref | Notes |
|---|---|---|---|
| `cospan.rs` | §1 Eq 7, §2.1 | §6.2.5 Def 6.43–6.45 | pushout composition via union-find |
| `span.rs` / `Rel` | §2.3 Ex 2.15 | §5.2 Ex 5.8 (Rel prop) | pullback composition, relation algebra |
| `named_cospan.rs` | §1 Eq 4 | — | port-labeled cospans (catgraph extension) |
| `frobenius/` | §2.2 Def 2.5 | §6.3.1 Def 6.52 | Frobenius monoid generators + 9 axioms |
| `compact_closed.rs` | §3.1 Props 3.1–3.4 | §4.5.1 Def 4.58, Prop 6.66 | cup/cap, name/unname, compose_names |
| `cospan_algebra.rs` | §2.1 Def 2.2, §4.1 | — | PartitionAlgebra, NameAlgebra, functor lifting |
| `hypergraph_category.rs` | §2.3 Def 2.12 | §6.3.3 Def 6.60 | HypergraphCategory trait |
| `hypergraph_functor.rs` | §2.3 Eq 12, §3.2 Prop 3.8 | §6.3 Thm 6.58 | HypergraphFunctor, CospanToFrobeniusFunctor |
| `equivalence.rs` | §4 Thm 4.13 (= Thm 1.2) | — | CospanAlgebraMorphism, roundtrip |
| `monoidal.rs` | implicit | §4.4.3 Rough Def 4.45 | Monoidal, SymmetricMonoidalMorphism |
| `operadic.rs` (trait only) | §2.5 (motivational) | §6.5 Rough Def 6.91 | Operadic trait; concrete impls in catgraph-applied |
| `category.rs` | implicit | §3.2 Def 3.6 (pedagogical) | HasIdentity, Composable |
| `finset.rs` | §3.2 Lemma 3.6 | — | Permutation, Decomposition, epi-mono factorization |

### catgraph-applied (Phase 2) — mixed provenance

| Module | [FS19] ref | [FS18] ref | Neither paper | Notes |
|---|---|---|---|---|
| `wiring_diagram.rs` | §2.5 Eq 6 (illustration) | §6.5 Ex 6.94 (Cospan operad), §4.4.2 wiring diagrams, §6.3.2 | — | Operadic substitution on named cospans. The *Operadic* trait is anchored to [FS18] §6.5; the wiring diagram interpretation is anchored to [FS18] §6.3.2 + §4.4.2. [FS19] only references wiring diagrams illustratively in §2.5. |
| `petri_net.rs` | — | §6.4 Def 6.75 (decorated cospan, specialized) | Baez-Pollard [BP17], Baez-Fong-Pollard [BFP16] | cospan bridge, fire/enable/reachable, parallel/sequential composition. The textbook cites [BFP16, BP17] as further reading for Petri nets as decorated cospans. The formal Petri-net-as-SMC treatment is from those papers, not from [FS18] or [FS19]. |
| `temperley_lieb.rs` | — | §6.3 (spider-adjacent) | Jones [Jon83], Kauffman [Kau87], Brauer [Bra37] | Brauer/TL algebra via perfect matchings, Jones relations, dagger. The textbook's Frobenius/spider material (§6.3) is the *context* for TL, but TL itself (non-crossing matchings, Jones polynomial, representation theory) is from the knot theory / representation theory literature, not from either F&S paper. |
| `linear_combination.rs` | — | §5.3.1 (rig infrastructure) | — | Free R-module R[T]. Provides the coefficient algebra that [FS18] §5.3 presupposes. Not a formal item in either paper — it's algebraic infrastructure. |
| `e1_operad.rs` | — | §6.5 Rough Def 6.91 | May [May72], Boardman-Vogt [BV73] | Little-intervals operad. [FS18] §6.5 defines operads abstractly; the *specific* E₁ operad is from the algebraic topology literature. |
| `e2_operad.rs` | — | §6.5 Rough Def 6.91 | May [May72], Boardman-Vogt [BV73] | Little-disks operad. Same: abstract operad definition from [FS18], specific E₂ construction from homotopy theory. |
| `rig.rs` | — | §5.3.1 Def 5.36 | deep_causality_num (blanket) | `Rig` trait + BoolRig, UnitInterval, Tropical, F64Rig. `Zero`/`One` re-sourced from `deep_causality_num` in Phase 2 (#1). |
| `mat_kron.rs` | §2.3 Ex 2.16 | — | Kissinger 2015 (FdVect HC) | `MatKron<R>` Kronecker-tensor **genuine hypergraph category** over a rig; Hadamard SCFM (μ/δ/η/ε) as inherent generators on native `Monoidal`/`Composable`/`SymmetricMonoidalMorphism`; speciality δ;μ=id (n=2,3,5). Phase 2 (#1). |
| `trace.rs` | §2.6 | — | — | Partial trace `Tr_X(f)` built from the `mat_kron` cup/cap generators (strict Kronecker; no associators). Phase 2 (#1). |
| `prop/presentation/mod.rs` | — | §5.2 Def 5.33 | — | `Presentation<G>` with 9-rule SMC canonical form + user equations; `NormalizeEngine` selector (Structural / CongruenceClosure) + `eq_mod_functorial<F>` method. File split out of a single `prop/presentation.rs` when the CC backend landed. |
| `sfg.rs` | — | §5.3 Def 5.45 | — | `SignalFlowGraph<R>` free prop on G_R generators. |
| `mat.rs` | — | §5.3 Def 5.50 | — | `MatR<R>` pure-rig matrix prop. |
| `sfg_to_mat.rs` | — | §5.3 Thm 5.53 | — | `sfg_to_mat` functor S: SFG_R → Mat(R). |
| `graphical_linalg.rs` | — | §5.4 Thm 5.60 | — | `matr_presentation<R>` 16-equation presentation. Thm 5.60 closed via the semantic Functorial engine; see `prop/presentation/functorial.rs`. |
| `prop/presentation/smc_nf.rs` | — | §5.2 Def 5.2/5.25 (SMC coherence) | Joyal-Street 1991 Part I, Selinger 2011 | Layer 1 string-diagram normal form — canonicalizes PropExpr up to SMC coherence (associator, unitors, interchange, braid naturality, σ²=id). |
| `prop/presentation/functorial.rs` | — | §5.4 Thm 5.60 (decision) | Baez-Erbele 2015 | `CompleteFunctor<G>` trait + `MatrixNFFunctor<R>` concrete instance wrapping `sfg_to_mat` as a complete-by-theorem decision procedure for Mat(R). |
| `mat_f64.rs` (feature `f64-rig`) | — | §5.3 Def 5.50 bridge | nalgebra | `mat_to_nalgebra`/`mat_from_nalgebra` + det + inverse for F64Rig. |
| `enriched.rs` | — | §1.1, §2.4, Rough Def 4.51 | CTFP Ch 28 | `EnrichedCategory<V: Rig>` trait + `HomMap<O, V>` finite realization. Object-safe for the `catgraph-magnitude` LmCategory. |
| `lawvere_metric.rs` | — | §1.3–1.4 pedagogical anchor | Lawvere 1973, CTFP §28.5 | `LawvereMetricSpace<T>` over `Tropical` + triangle-inequality verifier + `-ln π` embedding from `UnitInterval`. |
| `prop/presentation/kb.rs` | — | §5.2 Def 5.33 (CC backend) | Downey-Sethi-Tarjan 1980 | Congruence-closure decision procedure (signature-table variant) — default `eq_mod` backend via `NormalizeEngine::CongruenceClosure`. |

### catgraph-physics (Phase 4) — no F&S provenance

| Module | [FS19] ref | [FS18] ref | Actual provenance | Notes |
|---|---|---|---|---|
| `hypergraph/hypergraph.rs` | — | — | Wolfram [Wol20] | Typed hypergraph with source/target semantics |
| `hypergraph/rewrite_rule.rs` | — | — | Gorard [Gor20], Ehrig [EPS73] (DPO) | Double-pushout rewriting on hypergraphs |
| `hypergraph/evolution.rs` | — | — | Wolfram [Wol20], Gorard [Gor20] | Hypergraph evolution, BFS, causal invariance |
| `hypergraph/gauge.rs` | — | — | Gorard [Gor21] | Lattice gauge theory on hypergraph substrates |
| `hypergraph/evolution_cospan.rs` | uses `Cospan<Λ>` | — | catgraph bridge design | Cospan chain from evolution steps |
| `hypergraph/rewrite_span.rs` | uses `Span<Λ>` | — | catgraph bridge design | Span representation of rewrite rules |
| `hypergraph/multiway_cospan.rs` | uses `Cospan<Λ>` | — | catgraph bridge design | Multiway cospans |
| `multiway/evolution_graph.rs` | — | — | Wolfram [Wol20] | MultiwayEvolutionGraph, confluence diamonds |
| `multiway/branchial.rs` | — | — | Wolfram [Wol20], Gorard [Gor20] | BranchialGraph (per-step cross-sections) |
| `multiway/curvature.rs` | — | — | Ollivier [Oll09] | Ollivier-Ricci curvature on graphs |
| `multiway/wasserstein.rs` | — | — | Villani [Vil03] | Wasserstein-1 optimal transport |
| `multiway/branchial_spectrum.rs` | — | — | spectral graph theory | Laplacian eigendecomposition (nalgebra) |
| `multiway/branchial_analysis.rs` | — | — | rustworkx-core algorithms | Coloring, k-core, articulation points |

**catgraph-physics uses catgraph core types** (`Composable`, `Cospan`, `Span`) as categorical bridges, but its mathematical content is entirely from the Wolfram model / discrete differential geometry literature — neither F&S paper.

### Features not in either paper

| catgraph-applied module | Feature | Paper provenance |
|---|---|---|
| `temperley_lieb.rs` | Brauer algebra (perfect matchings with crossings) | Brauer [1937], not F&S |
| `temperley_lieb.rs` | Jones relations (e_i² = δ·e_i, far commutativity, braid) | Jones [1983], not F&S |
| `temperley_lieb.rs` | `LinearCombination` over Brauer diagrams | representation theory, not F&S |
| `temperley_lieb.rs` | `dagger` (adjoint / vertical reflection) | dagger-category structure, not F&S |
| `temperley_lieb.rs` | `non_crossing` detection | TL-specific, not F&S |
| `e1_operad.rs` | `go_to_monoid` homomorphism | algebraic topology, not F&S |
| `e1_operad.rs` | `coalesce_boxes` (inverse substitution) | catgraph design |
| `e2_operad.rs` | `from_e1_config` (E₁ → E₂ embedding) | standard embedding, not F&S |
| `petri_net.rs` | BFS reachability analysis | Petri net theory [Murata89], not F&S |
| `petri_net.rs` | Weighted arcs (`Decimal`) | quantitative Petri nets, not F&S |
| `linear_combination.rs` | Convolution product `Mul<Self>` | ring theory infrastructure |
| `linear_combination.rs` | `linearly_extend` / `inj_linearly_extend` | functorial pushforward |
| `wiring_diagram.rs` | Directed ports (`Dir::In`, `Dir::Out`, `Dir::Undirected`) | catgraph design extension |

### Overlap between papers

The following items appear in both [FS18] and [FS19]. The core audit ([FS19]) is authoritative for these; [FS18] covers them pedagogically:

| Topic | [FS19] section | [FS18] section | Tracked in |
|---|---|---|---|
| Frobenius monoid definition | §2.2 Def 2.5 | §6.3.1 Def 6.52 | core audit |
| Hypergraph category definition | §2.3 Def 2.12 | §6.3.3 Def 6.60 | core audit |
| Cospan_C as hypergraph category | §2.3 Ex 2.14 | §6.3 Ex 6.61 | core audit |
| Cospan pushout composition | §1 | §6.2.5 Def 6.43–6.45 | core audit |
| Self-dual compact closed | §3.1 Prop 3.1 | §6.3 Prop 6.66 | core audit |
| Cospan ≅ free Frobenius | §3.2 Prop 3.8 | §6.3 Thm 6.58 | core audit |
| Operads (motivational) | §2.5 | §6.5 Rough Def 6.91 | this audit (formal) / core audit (motivational) |
| SMC definition | implicit | §4.4.3 Rough Def 4.45 | core audit (implicit) |

For all overlapping items, the [FS19] research paper provides the rigorous version. The [FS18] textbook provides the pedagogical introduction. catgraph core implements the [FS19] versions; this audit does not re-count them.

---

## Enrichment extension point: catgraph-magnitude + koalisi

catgraph-applied stays in the `Set`-enriched (ordinary) categorical world:
all hom-sets are plain Rust collections. The *enriched* refinement — where
hom-objects live in a monoidal base `V` (e.g. `[0,1]`, `[0,∞]`, a tropical
semiring, or a more general rig) — is implemented one level up:

- **Compute layer:** [`catgraph-magnitude`](../../catgraph-magnitude), the
  sibling workspace crate (Phase 3). Provides `Rig`/`Ring`,
  `WeightedCospan<Q>`, `LawvereMetricSpace<T>`, `LmCategory`, and the
  `magnitude(t)` functional. Anchored to [BV25] (arXiv:2501.06662v2).
- **Application layer:** the coalition layer lives in the downstream private
  repo **koalisi** (sustia-llc/koalisi), not in this workspace. It provides
  the generic-payload `Coalition<T>` (weighted-edge message graph + `mag(t)`
  agent-graph view) and the `EnrichedCoalition<L>` BTV21 enriched layer.
  Anchored to [BTV21] (arXiv:2106.07890v2) + [BV25]. koalisi consumes the K1
  `Hypergraph` container from this crate as its backend (sustia-llc/koalisi#4).

The relevant catgraph-applied surface that the enriched layer consumes —
`WiringDiagram` + `Operadic` + `LawvereMetricSpace` + `EnrichedCategory` —
is audited above as part of the unenriched [FS18] coverage. The
`Operadic for WiringDiagram` `InterCircle: Clone` widening and the
`LawvereMetricSpace::from_distances` + `hom` diagonal default were added to
support the downstream koalisi coalition layer.
