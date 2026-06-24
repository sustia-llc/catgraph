# Fong-Spivak Coverage Audit (catgraph — strict F&S 2019 core)

> **Paper:** Fong & Spivak, *Hypergraph Categories* (arXiv:1806.08304v3, 18 Jan 2019)
> **Library:** `catgraph` (strict F&S 2019 core)
> **Method:** re-read paper, cross-check each numbered item against source; spot-check all ✅/⚠️ location claims; run the test suite (520 passed / 0 failed / 0 ignored).
>
> **Scope.** This crate is the strict F&S 2019 surface only. `Corel<Λ>` (the dual of `Rel`,
> FS 2018 Ex 6.64) lives here for substrate-locality and is therefore not represented in the
> tables below; its dedicated tests ([`tests/corel.rs`](../tests/corel.rs),
> [`tests/corel_hypergraph_category.rs`](../tests/corel_hypergraph_category.rs)) pass against
> the FS 2019 `HypergraphCategory` trait surface. Applied-CT (FS 2018), magnitude,
> Wolfram-physics, and CDL extensions live in the sibling crates `catgraph-applied`,
> `catgraph-magnitude`, `catgraph-physics`, and `catgraph-dl`. The FS 2019 core (`cospan`,
> `span`, `named_cospan`, `frobenius/`, `compact_closed`, `cospan_algebra`,
> `hypergraph_category`, `hypergraph_functor`, `equivalence`, `finset`, `monoidal`) is fully intact.
>
> **⚠️ Reboot note (2026-06-24).** Written pre-reboot; the version stamps (Phase 0.5, catgraph v0.10.x–v0.12.0, the test-count `520 passed`, the Release-history references) predate the reboot to `sustia-llc/catgraph` (workspace pinned `0.1.0`, GitHub-issue tracked). Coverage content is current: Phase 2 (#1) realized **Ex 2.16** in sibling `catgraph-applied::mat_kron` (+ partial `trace`), reflected in the tables below. Holistic version reconciliation is tracked in #7.

**Status legend:**
- ✅ DONE — implemented and tested
- ⚠️ PARTIAL — implementation exists but is incomplete, implicit, or doesn't fully exhibit the paper's structure
- ❌ MISSING — not implemented in catgraph
- ➖ N/A — theoretical / motivational, no implementation expected

## Summary

| Section | DONE | PARTIAL | MISSING | N/A | Total |
|---|---|---|---|---|---|
| §1 Introduction | 6 | 2 | 0 | 1 | 9 |
| §2.1 Cospan-algebras | 3 | 2 | 2 | 0 | 7 |
| §2.2 Frobenius monoids | 3 | 2 | 2 | 1 | 8 |
| §2.3 Hypergraph categories | 4 | 6 | 1 | 2 | 13 |
| §2.4 Critiques | 0 | 0 | 2 | 0 | 2 |
| §2.5 Operads | 0 | 0 | 0 | 1 | 1 |
| §3.1 Compact closed | 7 | 0 | 0 | 0 | 7 |
| §3.2 Free hypergraph cats | 4 | 4 | 3 | 2 | 13 |
| §3.3 io/ff factorization | 0 | 0 | 6 | 0 | 6 |
| §3.4 Strictification | 0 | 1 | 3 | 0 | 4 |
| §4.1 H → A direction | 5 | 2 | 2 | 1 | 10 |
| §4.2 A → H direction | 5 | 1 | 1 | 1 | 8 |
| §4.3 The equivalence | 1 | 1 | 2 | 1 | 5 |
| **TOTAL** | **38** | **21** | **24** | **10** | **93** |

**Headline numbers (after Phase 0.5):**
- **41% DONE / 23% PARTIAL / 26% MISSING / 11% N/A**
- Of the 24 missing items: 6 are §3.3 (explicitly deferred), 5 are LinRel/non-strict examples (deferred; Ex 2.16 FdVect is no longer here — realized in `catgraph-applied::mat_kron`, Phase 2 #1), 3 are §3.4 strictification (deferred), and most of the remainder are cross-Λ functoriality items that require parametric-Λ machinery beyond catgraph's type system.
- **Phase 0.5 closed 5 gaps + 1 bonus**: Prop 3.4 (explicit comp cospan form), Prop 4.6 (initiality proptest), compose_names_direct, Lemma 4.3 (A_F natural transformation), Lemma 4.9 (F_α io functor), plus a bug fix to `two_layer_simplify` that let `permutation_automatic` come out of `#[ignore]`.

## Per-section detail

### §1 Introduction (motivation + main theorems)

| Item | Status | catgraph location | Notes |
|---|---|---|---|
| Eq 1: 3-box wiring example | ✅ | examples + named_cospan.rs | running example |
| Eq 2: Frobenius generator decomposition | ✅ | frobenius/operations.rs | from_decomposition |
| Eq 3: alternative wiring | ➖ | — | visual variant of Eq 2 |
| Eq 4: cospan A→N←B for the running example | ✅ | named_cospan.rs::new | core type |
| Eq 5: hierarchy of category types (cat → mon → traced → hyper) | ⚠️ | — | implicit; no explicit `TracedMonoidalCategory` layer in core (CLAUDE.md says "OK because hypergraph subsumes it"). Sibling `catgraph-applied::trace` provides a concrete partial trace over the Kronecker HC (Phase 2 #1). |
| Eq 6: operadic substitution as a compositional view | ✅ | operadic.rs (trait) | impl currently in wiring_diagram.rs — **Phase 3 note:** `wiring_diagram.rs` moves to `catgraph-applied`; the `Operadic` *trait* stays in catgraph core |
| Eq 7: labeled cospan diagram (m → p ← n) | ✅ | cospan.rs | core type |
| Eq 8: Hyp_OF(Λ) ≅ Lax(Cospan_Λ, Set) | ✅ | equivalence.rs | morphism direction via CospanAlgebraMorphism + roundtrip tests |
| Thm 1.1: every hypergraph cat ≅ OF (coherence) | ⚠️ | — | catgraph works inside Cospan_Λ which IS objectwise-free, but never proves the general equivalence Hyp ≃ Hyp_OF |
| Thm 1.2: Hyp_OF ≅ ∫ Lax(Cospan_Λ, Set) | ⚠️ | equivalence.rs | per-Λ version verified (Thm 4.13); the Grothendieck-construction global form (Thm 4.16) is implicit |

### §2.1 Cospans and cospan-algebras

| Item | Status | catgraph location | Notes |
|---|---|---|---|
| Cospan_Λ category with pushout composition | ✅ | cospan.rs | union-find pushout |
| Eq 9: Cospan_f functoriality square | ❌ | — | no explicit Cospan_- functor between different Λ's |
| Prop 2.1: Cospan_- : Set_List → Cat is a functor | ❌ | — | the cross-Λ functoriality is not implemented |
| Def 2.2: cospan-algebra (lax sym mon functor a: Cospan_Λ → Set) | ✅ | cospan_algebra.rs | CospanAlgebra trait |
| Def 2.2: morphism of cospan-algebras (relabeling f + nat trans α) | ⚠️ | — | the relabeling f: Λ → List(Λ') part is missing; α part exists implicitly via Lemma 4.9 (also missing) |
| Def 2.2: Cospan-Alg category (objects + morphisms) | ⚠️ | — | trait exists; no explicit "category of cospan-algebras" type with composition |
| Ex 2.3: PartitionAlgebra Part_Λ | ✅ | cospan_algebra.rs::PartitionAlgebra | |
| Prop 2.4: Cospan-Alg ≅ ∫ Lax(Cospan_Λ, Set) (Grothendieck) | ⚠️ | — | the right-hand side IS what catgraph tests against, but the left-hand global category is never constructed |

### §2.2 Special commutative Frobenius monoids

| Item | Status | catgraph location | Notes |
|---|---|---|---|
| Def 2.5: special commutative Frobenius monoid (μ, η, δ, ε + 9 axioms) | ✅ | frobenius/operations.rs::FrobeniusOperation | |
| Frobenius axioms verification | ✅ | tests/frobenius_laws.rs | 8 tests: `identity_compose_identity`, `braiding_involution`, `spider_fusion`, `unit_counit_scalar`, `composition_associativity`, `monoidal_product_domains`, `from_decomposition_roundtrip`, `special_frobenius_morphism_consistency` |
| Ex 2.6: canonical Frobenius on monoidal unit I | ⚠️ | — | implicit (every type provides identity-as-unit); not an explicit constructor |
| Ex 2.7: 1-object SMC = monoid case (Frobenius on I = invertible scalar) | ➖ | — | algebraic remark, not a constructable example |
| Ex 2.8: Frobenius on object 1 in Cospan | ✅ | hypergraph_category.rs (`impl HypergraphCategory for Cospan<Lambda>`) | |
| Ex 2.9: any object in Cospan(C) for C with finite colimits | ⚠️ | — | catgraph only does Cospan_Λ on FinSet_Λ, not general Cospan(C) |
| Ex 2.10: additive Frobenius on ℝ in LinRel | ❌ | — | LinRel deferred per FONG-SPIVAK-XREF |
| Ex 2.11: multiplicative Frobenius on ℝ in LinRel (different) | ❌ | — | LinRel deferred |

### §2.3 Hypergraph categories

| Item | Status | catgraph location | Notes |
|---|---|---|---|
| Def 2.12: hypergraph category | ✅ | hypergraph_category.rs::HypergraphCategory | trait with η, ε, μ, δ + cup, cap |
| Eq 11: monoidal compatibility (4 equations) | ⚠️ | — | implicit in cospan structure; not a separate verification |
| Unit coherence axiom η_I = id_I = ε_I | ⚠️ | — | implicit; relies on Prop 2.18 (strict case) |
| Eq 12: hypergraph functor (F, φ) | ✅ | hypergraph_functor.rs::HypergraphFunctor | trait |
| Hyp 1-category | ⚠️ | — | trait exists but no explicit "category of hypergraph cats" type |
| Hyp 2-category (with monoidal nat trans as 2-morphisms) | ❌ | — | catgraph is not 2-categorical |
| Remark 2.13: every nat trans is invertible | ➖ | — | theoretical observation |
| Ex 2.14: Cospan(C) for C with colimits as hypergraph cat | ⚠️ | — | only Cospan_Λ on FinSet_Λ, not general Cospan(C) |
| Ex 2.15: Span(C) when C^op has limits | ⚠️ | span.rs | only Span on FinSet_Λ |
| Ex 2.15: Rel as hypergraph cat | ⚠️ | span.rs::Rel | Rel exists; HypergraphCategory impl missing |
| Ex 2.16: FdVect with chosen basis as hypergraph cat | ✅ | sibling `catgraph-applied::mat_kron` | Kronecker/Hadamard genuine HC over a rig (Phase 2 #1); Hadamard SCFM μ/δ/η/ε as inherent generators, speciality δ;μ=id (n=2,3,5). Realized in the applied crate, not core. |
| Remark 2.17: unit coherence is a NEW axiom vs older defs | ➖ | — | theoretical |
| Prop 2.18: strict case ⟹ unit coherence automatic | ✅ | — | catgraph relies on this implicitly (cospans are strict) |
| Ex 2.19: non-strict counterexample requiring unit coherence | ❌ | — | not implemented |

### §2.4 Critiques

| Item | Status | catgraph location | Notes |
|---|---|---|---|
| Ex 2.20: hypergraph structures don't extend along equivalences (LinRel_2) | ❌ | — | LinRel deferred |
| Ex 2.21: ff+ess.surj functor not necessarily hypergraph equivalence | ❌ | — | LinRel deferred |

### §2.5 A word on operads

(motivational discussion, no theorems)

**Phase 3 reconciliation note:** catgraph's concrete operadic machinery — `wiring_diagram.rs`, `e1_operad.rs`, `e2_operad.rs`, `temperley_lieb.rs`, `linear_combination.rs`, `petri_net.rs` — is **not** part of the F&S 2019 paper's numbered content. These modules will relocate to `catgraph-applied` (workspace sibling). The core `Operadic` trait (`operadic.rs`) stays in catgraph as the abstract interface. See "Reconciliation with catgraph-applied" section below.

### §3.1 Self-dual compact closed

| Item | Status | catgraph location | Notes |
|---|---|---|---|
| Compact closed category definition (cup, cap, zigzag) | ✅ | compact_closed.rs (cup/cap functions) | not a separate trait; provided as helper functions |
| Eq 13: zigzag identities | ✅ | tests/compact_closed.rs (33 tests) | tested |
| Prop 3.1: every hypergraph cat is self-dual compact closed (cup_X := η; δ, cap_X := μ; ε) | ✅ | compact_closed.rs::cup, cap | exact formula |
| Prop 3.2: bijection C(X,Y) ≅ C(I, X⊗Y) (name) | ✅ | compact_closed.rs::name, unname | |
| Eq 14: comp^Y_{X,Z} morphism (id_X ⊗ cap_Y ⊗ id_Z) | ✅ | equivalence.rs::comp_cospan | |
| Prop 3.3: (f̂ ⊗ ĝ) ; comp^Y_{X,Z} = (f;g)^ | ✅ | compact_closed.rs::compose_names_direct + compose_names_via_unname | direct impl matches Prop 3.3 literally; via-unname kept as reference; equivalence tested in tests/compact_closed.rs |
| Prop 3.4: (id_X ⊕ f̂) ; comp^X_{∅,Y} = f | ✅ | tests/compact_closed.rs::prop_3_4_* | literal form tested (id_X ⊗ f̂) ; (cap_X ⊗ id_Y) for 5 witnesses including identity, mult, comult, unit;comult |
| Ex 3.5: comp in Cospan_Λ | ✅ | equivalence.rs::comp_cospan + tests | the literal cospan picture matches |

### §3.2 Free hypergraph categories

| Item | Status | catgraph location | Notes |
|---|---|---|---|
| Lemma 3.6: Cospan generated by μ, η, δ, ε, σ, id | ✅ | finset.rs::Decomposition + frobenius::from_decomposition | epi-mono factorization |
| Ex 3.7: building Eq 4 cospan from generators | ⚠️ | — | example exists for similar cospans but not literally Eq 4 |
| Prop 3.8: Cospan ≃ theory of special commutative Frobenius monoids | ✅ | cospan_algebra.rs::cospan_to_frobenius + hypergraph_functor.rs::CospanToFrobeniusFunctor | |
| Def 3.9: OF(Λ) structure (List(Λ) ≅ Ob(C)) | ⚠️ | — | implicit; List(Λ) ≅ Ob(C) is just how catgraph encodes objects |
| Lemma 3.10: assigning Frobenius per l ∈ Λ uniquely determines hypergraph structure | ⚠️ | — | implicit in CospanToFrobeniusFunctor |
| Remark 3.11: explicit construction of μ_l | ➖ | — | construction detail |
| Ex 3.12: Cospan_Λ as hypergraph cat | ✅ | hypergraph_category.rs (Cospan impl) | |
| Cor 3.13: Cospan_- : Set_List → Hyp_OF as a functor | ❌ | — | the cross-Λ functor not implemented |
| Thm 3.14: Cospan_Λ is FREE hypergraph cat over Λ (Set ⇄ Hyp adjunction with unit/counit/triangles) | ❌ | — | DEFERRED — universal property API |
| Cor 3.15: Set_List ⇄ Hyp_OF refinement | ❌ | — | not implemented |
| Prop 3.16: counit Frob_{Cospan_Λ} is identity | ⚠️ | — | implicit in CospanToFrobeniusFunctor design; not a separate test |
| Remark 3.17: combined adjunctions diagram | ➖ | — | summary diagram |

### §3.3 Factoring hypergraph functors (io/ff)

| Item | Status | catgraph location | Notes |
|---|---|---|---|
| Eq 19: H_F construction | ❌ | — | DEFERRED (low priority per FONG-SPIVAK-XREF) |
| Remark 3.18: orthogonal factorization system on Hyp | ❌ | — | DEFERRED |
| Lemma 3.19: i_1 = Frob_1 | ❌ | — | DEFERRED |
| Prop 3.20: Gens : Hyp_OF → Set_List is split Grothendieck fibration | ❌ | — | DEFERRED |
| Eq 21: Hyp_OF(f) construction | ❌ | — | DEFERRED |
| Cor 3.21: Hyp_OF ≃ ∫ Hyp_OF(Λ) | ❌ | — | DEFERRED |

### §3.4 Strictification (Coherence theorem)

| Item | Status | catgraph location | Notes |
|---|---|---|---|
| Hyp_OF as full sub-2-cat of Hyp | ❌ | — | not 2-categorical |
| Thm 3.22: U: Hyp_OF → Hyp is a 2-equivalence (the coherence theorem) | ⚠️ | — | implicit; catgraph works in OF case only and never proves the equivalence |
| Eq 22: pre-parenthesized product P([x,y]) | ❌ | — | strictification construction |
| Eq 23: Str: Hyp → Hyp_OF 2-functor | ❌ | — | not implemented |

### §4.1 Hypergraph cats → cospan-algebras

| Item | Status | catgraph location | Notes |
|---|---|---|---|
| Eq 24: Hyp_OF(Λ) ≅ Lax(Cospan_Λ, Set) | ✅ | equivalence.rs | tested via roundtrip |
| Prop 4.1: A_- : Hyp_OF(Λ) → Lax(Cospan_Λ, Set) is a functor | ⚠️ | cospan_algebra.rs::NameAlgebra | NameAlgebra is the analog *for one fixed H = Cospan*, not a functor over varying H |
| Eq 25: A_H(-) := H(I, Frob(-)) | ✅ | NameAlgebra::map_cospan | implemented as the lax monoidal functor itself |
| Lemma 4.2: A_H is lax monoidal functor | ✅ | NameAlgebra trait impl | implicit in trait impl |
| Eq 26: laxator γ definition (γ: 1 → A_H(∅) and γ_{X,Y}: A_H(X) × A_H(Y) → A_H(X⊕Y)) | ✅ | NameAlgebra::lax_monoidal | |
| Lemma 4.3: A_F natural transformation construction (from F: H → H' to α: A_H → A_H') | ✅ | cospan_algebra.rs::functor_induced_algebra_map | free function lifting any `HypergraphFunctor` to a cospan-algebra morphism; tests in tests/cospan_algebra.rs verify naturality/monoidality/unit for RelabelingFunctor and CospanToFrobeniusFunctor |
| Eq 27/28: naturality square + α_X definition | ✅ | cospan_algebra.rs::functor_induced_algebra_map + lemma_4_3_* tests | naturality verified via assert_eq chains |
| Remark 4.4: extension to Hyp (non-OF case) via Frob_H'(-) | ➖ | — | not needed since catgraph only does OF |
| Remark 4.5: A_{Cospan_Λ} = Part_Λ | ✅ | implicit | the roundtrip test verifies this for Part |
| Prop 4.6: Part_Λ is initial cospan-algebra | ✅ | tests/cospan_algebra.rs::prop_4_6_* | proptest verifies unit preservation, naturality, and monoidal coherence of the unique map α_x(c) = A.map_cospan(c, A.unit()) for PartitionAlgebra → PartitionAlgebra and PartitionAlgebra → NameAlgebra |

### §4.2 Cospan-algebras → hypergraph cats

| Item | Status | catgraph location | Notes |
|---|---|---|---|
| Prop 4.7: H_- : Lax(Cospan_Λ, Set) → Hyp_OF(Λ) functor | ✅ | equivalence.rs::CospanAlgebraMorphism | the H_A construction |
| Lemma 4.8: H_A definition (objects = List(Λ), morphisms X→Y = A(X⊕Y)) | ✅ | equivalence.rs::CospanAlgebraMorphism | with full trait stack |
| Eq 31: Ob(H_A) := List(Λ), H_A(X,Y) := A(X⊕Y) | ✅ | equivalence.rs (domain/codomain/element fields) | |
| Eq 32: composition formula via comp^Y_{X,Z} | ✅ | equivalence.rs::compose | |
| Eq 33: six required cospans (id, braiding, μ, η, δ, ε) as ∅ → X⊕Y morphisms | ✅ | equivalence.rs identity_in/multiplication_in/comultiplication_in/unit_in/counit_in + from_permutation | |
| Lemma 4.9: F_α io functor from morphism α: A→B | ✅ | equivalence.rs::functor_from_algebra_morphism | free function lifting a monoidal natural transformation α: A → B to F_α: H_A → H_B by pointwise application; tests in tests/equivalence.rs verify identity and composition preservation for both F_id and the non-trivial α = cospan_to_frobenius case |
| Remark 4.10: hypergraph cats absorb special morphisms into operations | ➖ | — | conceptual remark |
| Lemma 4.11: Frob(c) = name(c) for Part case | ⚠️ | — | implicit; not a separate test |
| Cor 4.12: Frob_A(c) = A(name(c))(γ) | ⚠️ | — | implicit |

### §4.3 The equivalence

| Item | Status | catgraph location | Notes |
|---|---|---|---|
| Thm 4.13: Hyp_OF(Λ) ≃ Lax(Cospan_Λ, Set) | ✅ | equivalence.rs + tests/equivalence.rs | roundtrip verified for PartitionAlgebra and NameAlgebra (17 tests) |
| Naturality in Λ | ❌ | — | naturality across varying Λ not verified |
| Ex 4.14: LinRel' ≃ LinRel (specific worked example with the rectification ν(R) = (-a, b)) | ❌ | — | LinRel deferred |
| Remark 4.15: generalization Hyp^io_{H/} ≅ Lax(H, Set) | ➖ | — | theoretical remark |
| Thm 4.16: Hyp_OF ≅ Cospan-Alg (the global form, not per-Λ) | ⚠️ | — | implicit; catgraph proves the per-Λ Thm 4.13 but not the Grothendieck-construction global form which packages everything via naturality |

## Missing items — revisit list

Flat bulleted index of every ❌ MISSING entry across the per-section tables above, grouped by paper section so it can be revisited in a single pass. Each bullet declares its **disposition class** at the end of the line:

- **[paper-acknowledged]** — the paper itself flags this as ancillary; no obligation to ship
- **[different-target-category]** — would require LinRel / FdVect / 2-categorical machinery beyond the FS19 Cospan_Λ / FrobeniusMorphism surface
- **[cross-Λ-parametric]** — would require parametric-Λ machinery beyond Rust's current type system (functoriality of `Cospan_-` over varying Λ)
- **[universal-property-API]** — Set ⇄ Hyp adjunction surface not exposed in the type system

### §1 Introduction

(no ❌ items)

### §2.1 Cospans and cospan-algebras

- Eq 9: `Cospan_f` functoriality square — **[cross-Λ-parametric]**
- Prop 2.1: `Cospan_- : Set_List → Cat` is a functor — **[cross-Λ-parametric]**

### §2.2 Special commutative Frobenius monoids

- Ex 2.10: additive Frobenius on ℝ in LinRel — **[different-target-category]**
- Ex 2.11: multiplicative Frobenius on ℝ in LinRel — **[different-target-category]**

### §2.3 Hypergraph categories

- Hyp 2-category (with monoidal nat trans as 2-morphisms) — **[different-target-category]** (1-categorical vs 2-categorical)
- Ex 2.19: non-strict counterexample requiring unit coherence — **[different-target-category]**

*(Ex 2.16 FdVect-with-chosen-basis is no longer missing — realized in sibling `catgraph-applied::mat_kron`, Phase 2 #1.)*

### §2.4 Critiques

- Ex 2.20: hypergraph structures don't extend along equivalences (LinRel_2) — **[different-target-category]**
- Ex 2.21: ff+ess.surj functor not necessarily a hypergraph equivalence — **[different-target-category]**

### §2.5 A word on operads

(motivational section; no ❌ items)

### §3.1 Self-dual compact closed

(no ❌ items)

### §3.2 Free hypergraph categories

- Cor 3.13: `Cospan_- : Set_List → Hyp_OF` as a functor — **[cross-Λ-parametric]**
- Thm 3.14: `Cospan_Λ` is the FREE hypergraph cat over Λ (Set ⇄ Hyp adjunction) — **[universal-property-API]**
- Cor 3.15: `Set_List ⇄ Hyp_OF` refinement — **[cross-Λ-parametric]**

### §3.3 Factoring hypergraph functors (io/ff)

- Eq 19: `H_F` construction — **[paper-acknowledged]** (fibration-specific)
- Remark 3.18: orthogonal factorization system on Hyp — **[paper-acknowledged]**
- Lemma 3.19: `i_1 = Frob_1` — **[paper-acknowledged]**
- Prop 3.20: `Gens : Hyp_OF → Set_List` is a split Grothendieck fibration — **[paper-acknowledged]**
- Eq 21: `Hyp_OF(f)` construction — **[paper-acknowledged]**
- Cor 3.21: `Hyp_OF ≃ ∫ Hyp_OF(Λ)` — **[paper-acknowledged]** (entire §3.3 is fibration analysis, not needed for Thm 1.2)

### §3.4 Strictification (Coherence theorem)

- `Hyp_OF` as full sub-2-cat of `Hyp` — **[different-target-category]** (2-categorical)
- Eq 22: pre-parenthesized product `P([x,y])` — **[different-target-category]**
- Eq 23: `Str: Hyp → Hyp_OF` 2-functor — **[different-target-category]**

### §4.1 Hypergraph cats → cospan-algebras

(no ❌ items)

### §4.2 Cospan-algebras → hypergraph cats

(no ❌ items)

### §4.3 The equivalence

- Naturality in Λ — **[cross-Λ-parametric]**
- Ex 4.14: LinRel' ≃ LinRel (specific worked example with the rectification ν(R) = (-a, b)) — **[different-target-category]**

### Disposition summary

| Disposition class | Count | Resolution |
|---|---|---|
| paper-acknowledged | 6 (all of §3.3) | Stay missing. Paper says explicitly: this is fibration analysis, not needed for Thm 1.2. |
| different-target-category | 10 (§2.2 LinRel, §2.3 Hyp 2-cat + Ex 2.19, §2.4 LinRel, §3.4 strictification, §4.3 LinRel) | Stay missing in catgraph core. Would require LinRel / 2-categorical machinery — a different crate's job. (Ex 2.16 FdVect was here; now realized in sibling `catgraph-applied::mat_kron`, Phase 2 #1.) |
| cross-Λ-parametric | 5 (§2.1 Eq 9 + Prop 2.1, §3.2 Cor 3.13 + Cor 3.15, §4.3 naturality in Λ) | Stay missing until Rust's type system grows parametric-over-Lambda traits, OR catgraph adds runtime-dispatched cross-Λ machinery (out of scope for the slim baseline). |
| universal-property-API | 1 (§3.2 Thm 3.14) | Could be added without new machinery — but only if a concrete consumer surfaces the need. Currently no consumer demand. |

**Net assessment of the table above:** of 22 ❌ items, **21 are structurally outside catgraph core's scope** (different category / parametric Λ / paper-acknowledged fibration). The single discretionary item is **Thm 3.14 universal-property API**, which is dormant by choice, not by mathematical obstacle.

## Critical findings

### Phase 0.5 completed (2026-04-11)

All five items listed in the previous audit have been closed for catgraph v0.10.4,
and remain closed in v0.10.5 and v0.10.6:

1. **Lemma 4.3** — ✅ `cospan_algebra::functor_induced_algebra_map` lifts any hypergraph functor to a cospan-algebra morphism; `tests/cospan_algebra.rs` verifies naturality, monoidality, and unit preservation for both `RelabelingFunctor` and `CospanToFrobeniusFunctor`.

2. **Lemma 4.9** — ✅ `equivalence::functor_from_algebra_morphism` lifts a monoidal natural transformation `α: A → B` to `F_α: H_A → H_B`; `tests/equivalence.rs` verifies identity and composition preservation for both `F_id` and the non-trivial `α = cospan_to_frobenius` case.

3. **Prop 3.4** — ✅ `tests/compact_closed.rs::prop_3_4_*` verifies the literal `(id_X ⊗ f̂) ; (cap_X ⊗ id_Y) = f` formula for 5 witnesses (identity, multiplication, comultiplication, unit;comult), building the comp cospan explicitly rather than going through `unname`.

4. **Prop 4.6 initiality test** — ✅ `tests/cospan_algebra.rs::prop_4_6_*` proptest verifies unit preservation, naturality, and monoidal coherence of the unique map `α_x(c) = A.map_cospan(c, A.unit())` for PartitionAlgebra self-map and PartitionAlgebra → NameAlgebra.

5. **`compose_names_direct`** — ✅ `compact_closed::compose_names_direct` implements Prop 3.3's literal formula `(f̂ ⊗ ĝ) ; comp^Y_{X,Z}`. `compose_names_via_unname` is preserved as a reference implementation. `tests/compact_closed.rs::compose_names_direct_*` verify equivalence on 5 concrete witnesses.

### Bonus: two_layer_simplify bug fix

The `#[ignore]`'d `permutation_automatic` test in `frobenius/operations.rs` was uncovered during Phase 0.5. Root cause: Rule 2 (braiding cancellation) in `FrobeniusLayer::two_layer_simplify` used `[Identity(b2), Identity(a2)]` for the next-layer replacement, preserving the layer's *input* types but flipping its *output* types — breaking the coupling with adjacent layers. Fix: swap to `[Identity(a2), Identity(b2)]` so `next_layer.right_type` is preserved. Stress test (`from_permutation_compose_probe`, 270 random witnesses over n∈[2,10]) now passes.

### Items intentionally deferred

(See "Items to keep deferred" below.)

### Items to keep deferred (paper-acknowledged or low-impact)

- §3.3 io/ff factorization (entire section, 6 items) — paper itself notes this is for fibration analysis; not needed for Thm 1.2
- §3.4 strictification (Thm 3.22) — implicit since catgraph works in OF case; making it explicit requires 2-category machinery
- Thm 3.14 universal property — paper-deferred
- LinRel examples (2.10, 2.11, 2.20, 2.21, 4.14) — paper-deferred
- Cross-Λ functoriality (Prop 2.1, Cor 3.13, Cor 3.15) — would require parametric Λ machinery beyond catgraph's current type system

### Items that are implicit / "morally correct" but not explicit

These are correct but could be made explicit for catgraph-as-paper-implementation pedagogy:

1. **Eq 11 monoidal compatibility** — implicit in cospan structure
2. **Unit coherence axiom** — implicit via Prop 2.18 (strict case)
3. **Prop 2.18** — relied on but not stated/tested
4. **Lemma 3.10** — implicit in CospanToFrobeniusFunctor
5. **Prop 3.16** — implicit in design
6. **Lemma 4.11, Cor 4.12** — implicit; would benefit from test assertions matching the paper's exact formulas

### The "compose_names" shortcut (audit finding confirmed, resolved 2026-04-11)

Historical note — this was the original finding that drove Gap 5 in Phase 0.5:

`compact_closed.rs::compose_names` used to be a "shortcut" that went back to morphisms via `unname → compose → name`. Mathematically equivalent to Prop 3.3's `(f̂ ⊗ ĝ) ; comp^Y_{X,Z}` formula, but didn't exhibit it structurally — defeating the §3.1 demonstration that you can compose at the name level without going back to morphisms.

**Resolution:** `compose_names_direct` now implements Prop 3.3 literally. `compose_names` is an alias pointing at `compose_names_direct` (the canonical form). `compose_names_via_unname` is preserved as the reference implementation, and `tests/compact_closed.rs::compose_names_direct_*` verify that both produce identical results on 5 concrete witnesses.

## What does "Theorem 1.2 is implemented" actually mean for catgraph?

**catgraph v0.13.0 implements Theorem 1.2 in its per-Λ form (which is Thm 4.13)**, with full bidirectional functoriality (Lemmas 4.3 and 4.9), all six structural cospans of §4.2, and Props 3.1–3.4 on compact closed structure, using `PartitionAlgebra` and `NameAlgebra` as worked examples. This has been the case since v0.10.4 (Phase 0.5); no v0.10.5/v0.10.6/v0.11.0/v0.12.0/v0.13.0 change has moved coverage in either direction.

**catgraph does NOT implement:**
- The global Grothendieck-construction form (Thm 4.16) — `Hyp_OF ≅ Cospan-Alg` as 1-categories with naturality across Λ
- The 2-categorical version (Thm 1.1) — the strictification result that reduces general Hyp to Hyp_OF
- Cross-Λ functoriality (Prop 2.1, Cor 3.13, Cor 3.15, Thm 3.14, §3.3 io/ff factorization, LinRel examples)

These require either the Grothendieck construction or parametric-Λ machinery beyond catgraph's current type system, and are permanently deferred.

## Reconciliation with catgraph-applied (Phase 3 and beyond)

catgraph core (v0.11.0 target) tracks the Fong-Spivak 2019 *Hypergraph Categories* paper strictly. A second, complementary applied-CT track is planned for the workspace sibling `catgraph-applied`, aligning with a different (TBD) F&S applied category theory paper whose audit docs will be added when that work begins.

### Current catgraph modules that are **not** part of F&S 2019 numbered content

These stay in catgraph until Phase 3, then relocate to `catgraph-applied`. They do not appear in the audit tables above because the 2019 paper does not cover them. The audit's coverage percentages are unaffected by the move.

| Module | Nature | Paper relationship | Phase 3 destination |
|---|---|---|---|
| `wiring_diagram.rs` | Operadic substitution impl | Referenced by Eq 6 as an illustration, but the F&S paper does not develop wiring diagrams formally | `catgraph-applied` |
| `e1_operad.rs` | 1D little-intervals operad | Not in F&S 2019 (§2.5 is motivational on operads but makes no formal claim) | `catgraph-applied` |
| `e2_operad.rs` | 2D little-disks operad | Not in F&S 2019 | `catgraph-applied` |
| `temperley_lieb.rs` | Temperley-Lieb category | Not in F&S 2019 (related to Jones/TL representation theory) | `catgraph-applied` |
| `linear_combination.rs` | Formal linear combinations | Not in F&S 2019; used only by `temperley_lieb.rs` | `catgraph-applied` |
| `petri_net.rs` | Petri net primitives | Not in F&S 2019; connects to applied CT papers on concurrency | `catgraph-applied` |

The **`Operadic` trait** (`src/operadic.rs`) is the abstract interface referenced by Eq 6 and stays in catgraph core. Only the concrete implementations move.

### catgraph-applied audit (Phase 5 — completed 2026-04-16)

The applied-CT audit is anchored to Fong & Spivak, *Seven Sketches in Compositionality* (arXiv:1803.05316v3, 2018). See [`catgraph-applied/docs/FS18-AUDIT.md`](../../catgraph-applied/docs/FS18-AUDIT.md) for the full section-by-section audit covering:

- **§6.5 Operads** — `Operadic` trait (core), `WiringDiagram` (Cospan operad, Ex 6.94), `E1`/`E2` operads (Rough Def 6.91)
- **§6.4 Decorated cospans** — `PetriNet` cospan bridge (Def 6.75 specialized)
- **§6.3 Hypergraph categories** — `BrauerMorphism` / Temperley-Lieb (spider-adjacent)
- **§5.2–5.4 Props, signal flow** — `LinearCombination` (rig infrastructure); props/SFG/Mat(R) not yet implemented
- **§4.4–4.5 Monoidal / compact closed** — inherited from catgraph core

The audit also includes a **cross-paper reconciliation** mapping every module in all three workspace crates (catgraph, catgraph-applied, catgraph-physics) to its paper provenance in [FS19] (this document's paper), [FS18] (Seven Sketches), or neither (Wolfram model, representation theory, etc.).

### What catgraph-applied inherits from the F&S 2019 core

Every item in catgraph-applied will be built on top of catgraph's F&S 2019 primitives:

- `Cospan<Lambda>` pushout composition — underlies any applied monoidal composition
- `Frobenius` generators — underlies operadic composition of SMCs via Prop 3.8
- `HypergraphCategory` trait — is the target category for wiring diagrams, Petri nets, and operadic morphisms
- Compact closed structure (§3.1) — cup/cap used in string-diagram rewriting (TL, wiring)
- `HypergraphFunctor` — the target for applied-CT semantic functors (e.g., Petri → Mark, wiring → StochMatrix)

No duplication of F&S primitives in catgraph-applied — it depends on catgraph.

Release history for this crate is in [`../CHANGELOG.md`](../CHANGELOG.md); infrastructure consumed by the applied-CT crate is cross-linked there.

## Canonical release-notes paragraph (durable; applies to every release since v0.11.0)

"catgraph implements Theorem 1.2 in its per-Λ form (Thm 4.13), with full bidirectional functoriality (Lemmas 4.3 and 4.9), all six structural cospans of §4.2, and Propositions 3.1–3.4 on compact closed structure directly verified. The global Grothendieck-construction form (Thm 4.16), the 2-categorical strictification (Thm 1.1), §3.3 io/ff factorization, and cross-Λ functoriality are intentionally deferred as requiring machinery beyond catgraph's current scope. Non-F&S modules live in sibling crates: Gorard's functorial irreducibility framework in `irreducible`, Wolfram-physics extensions (hypergraph DPO rewriting, gauge theory, multiway evolution, branchial analysis) in `catgraph-physics`, and applied-CT extensions (Petri nets, wiring diagrams, E_n operads, Temperley-Lieb, props, rigs, SFGs, MatR, enrichment) in `catgraph-applied`."
