# catgraph-dl — theorem map (law-test → paper anchor)

The correctness spine for `catgraph-dl` (issue
[#70](https://github.com/sustia-llc/catgraph/issues/70), Part 1). Every law test
in this crate is a Rust *witness* of a theorem/definition in the source papers;
**the paper is the proof layer** (we do not adopt the Lean/Kani toolchain — that
tradeoff is recorded on #70). This file is the tag registry linking each witness
to its anchor, mirroring DeepCausality's `THEOREM_MAP.md` tag→proof traceability.

Primary anchor: Gavranović et al., *Categorical Deep Learning*, ICML 2024
([arXiv:2402.15332v2](https://arxiv.org/abs/2402.15332)) — cited `CDL <kind> <n.m>`.
Appendices run **A–J only** (App C = GDL examples, not containers; there is no
App K). Container laws anchor to Abbott–Altenkirch–Ghani 2003 by author. Monoidal
coherence additionally cites Mac Lane, *CWM*.

Convention: when adding or moving a law test, add/adjust its row here in the same
PR (this is the #70 discipline). Style: reference `<file>::<test_fn>`.

## Functor / natural-transformation / pointed laws

| Witness | Anchor | Law |
|---|---|---|
| `functor_laws::list_endo_functor_laws`, `::tree_endo_functor_laws`, `::group_action_endo_functor_laws` | CDL Def 1.4 | Functor identity + composition |
| `natural_pointed_laws::*_natural_transformation*` | CDL Def 1.5 | Naturality square `transform ∘ F(f) == G(f) ∘ transform` |
| `natural_pointed_laws::*_pointed_sigma_naturality` | CDL Def B.3 | σ-naturality `fmap(pure(x), f) == pure(f(x))` |
| `natural_pointed_laws::*_iso_*` (via `iso::test_support`) | CDL Def 1.5 | `NaturalIso` round-trip + naturality |
| `common::assert_functor_laws` / `::assert_natural_transformation_naturality` / `::assert_pointed_naturality` | (helpers) | witness-generic drivers for the above |

## Container laws

| Witness | Anchor | Law |
|---|---|---|
| `container_laws::{list_endo,tree_endo,group_action_endo}_container_laws` | Abbott–Altenkirch–Ghani 2003 (via CDL) | Shape/arity round-trip + `fmap` coherence |
| `common::assert_container_laws` | (helper) | witness-generic container-law driver |

## Monoidal / actegory / module laws

| Witness | Anchor | Law |
|---|---|---|
| `monoidal_coherence_laws::monoidal_coherence_{deterministic,proptest}` | Mac Lane CWM; CDL §3.1 | Pentagon + triangle (Set/tuple carrier), via the generic driver |
| `module_actegory_laws::direct_sum_coherence_*` | CDL Def E.2 / Example G.3 | Pentagon + triangle on `DirectSum`, via the **same** generic driver |
| `module_actegory_laws::f64_module_axioms_*` | CDL Def E.2 / Example G.3 | R-module axioms (`Zero`/`One`) |
| `module_actegory_laws::direct_sum_monoid_*` | CDL Example E.4 / G.3 | `(FinReal, ⊕, R⁰)` monoid laws |
| `module_actegory_laws::actegory_action_and_multiplicator` | CDL Def E.2 | Actegory action + multiplicator |
| `common::assert_monoidal_coherence` | (helper) | **one** generic pentagon/triangle driver over `MonoidalCategory`; the `α ⊗ id` / `id ⊗ α` legs go through `MonoidalCategory::tensor_morphisms` (issue #65), so it serves both the tuple and `DirectSum` carriers |
| `common::assert_f64_module_axioms` / `::assert_direct_sum_monoid` | (helpers) | drivers for the module-axiom / `⊕`-monoid rows |

## Eilenberg–Moore / (co)algebra laws

| Witness | Anchor | Law |
|---|---|---|
| `monad_algebra_laws::negation_monad_algebra_unit_and_assoc_laws` | CDL Def 2.3 | EM-algebra unit + assoc |
| `monad_algebra_laws::unlawful_structure_maps_fail_the_algebra_laws` | CDL Def 2.3 | negative: unlawful maps fail |
| `monad_algebra_laws::abs_hom_unit_and_mult_coherence` | CDL Def 2.5 (ambient) | hom-side coherence probes (η-naturality, source-assoc∘f) |
| `monad_algebra_laws::hom_coherence_verifiers_pass_for_a_non_homomorphism` | CDL Def 2.5 | boundary: coherence verifiers do **not** discriminate a hom |
| `monad_algebra_laws::full_monad_algebra_hom_certification_recipe` | CDL Def 2.3 + Def 2.5; Mac Lane CWM VI.2 | end-to-end hom certification: source lawful ∧ target lawful ∧ square (each negative fails exactly one part) |
| `monad_algebra_laws::writer_monad_laws_pinned_by_non_abelian_s3` | CDL Def 2.1 / Ex 2.2 | writer-monad `join` order (non-abelian S3) |
| `monad_algebra_laws::monad_algebra_coherence_holds_proptest` | CDL Def 2.3 / 2.5 | proptest lift of the four verifiers |
| `algebra_homomorphisms::identity_is_an_f_algebra_homomorphism` | CDL Def 2.5 | F-algebra hom square (identity) |
| `algebra_homomorphisms::absolute_value_is_z2_equivariant_homomorphism` (+ proptest) | CDL Def 2.5 / Example 2.6 | Z2-equivariant hom square |
| `algebra_homomorphisms::non_equivariant_projection_fails_commuting_square` (+ proptest) | CDL Def 2.5 | negative: non-equivariant map fails the square |
| `algebra_homomorphisms::coalgebra_hom_identity_smoke` | CDL Def B.2 | F-coalgebra hom square (identity) |
| `algebra_homomorphisms::monad_algebra_hom_construction_and_commuting_square` | CDL Def 2.5 | monad-algebra hom square |

## Free-monad / cofree-comonad (un)rollers — architecture as (co)algebra

| Witness | Anchor | Law |
|---|---|---|
| `architecture_unrollers::folding_rnn_equivalent_to_free_mnd_unroller` (+ proptest) | CDL Remark 2.13 / Prop B.18 | RNN unroll = unique algebra hom from initial `FreeMnd(1 + A × −)` |
| `architecture_unrollers::recursive_nn_equivalent_to_free_mnd_unroller` (+ proptest) | CDL Remark 2.13 / Prop B.18 | tree unroll = unique algebra hom (tree direction) |
| `architecture_unrollers::unfolding_rnn_equivalent_to_cofree_cmnd_unroller` (+ proptest) | **CDL Remark H.6** / App I.3 | UnfoldingRnn unroll = finite prefix of unique coalgebra hom into `Stream(O)` |
| `architecture_unrollers::mealy_cell_equivalent_to_cofree_cmnd_unroller` (+ proptest) | **CDL Remark H.6** / App I.4 | Mealy run = input-driven `Cofree<OptionWitness, O>` prefix walk |
| `architecture_unrollers::moore_cell_equivalent_to_cofree_cmnd_unroller` (+ proptest) | **CDL Remark H.6** / App I.5 | Moore run = output-then-step `Cofree<OptionWitness, O>` prefix walk |
| `architecture_unrollers::unfolding_rnn_unroll_iter_agrees_and_is_lazy` (+ proptest) | **CDL Remark H.6** / App I.3 | `UnfoldingRnn::unroll_iter` (lazy #36): `.take(n)` prefix = `unroll_to_vec` = `Cofree` walk; laziness witness |
| `architecture_unrollers::mealy_cell_run_iter_agrees` (+ proptest) | **CDL Remark H.6** / App I.4 | `MealyCell::run_iter` (lazy #36): full/prefix consumption = `run` = `Cofree` walk |
| `architecture_unrollers::moore_cell_run_iter_agrees` (+ proptest) | **CDL Remark H.6** / App I.5 | `MooreCell::run_iter` (lazy #36): output-then-step consumption = `run` = `Cofree` walk |
| `architecture_unrollers::gdl_recovery_via_z2_invariant_folding` | CDL Example 2.6 | GDL recovery: Z2-invariant fold |
| `free_monad_bijections::*` | CDL Example B.19/B.20, Prop B.18 | haft `Free`/`Cofree` ↔ concrete-carrier bijections |

> **Anchor correction (#64):** the coalgebra-direction dual of Remark 2.13 is
> **Remark H.6** ("streams are a terminal object in the category of
> `(O × −)`-coalgebras"), and the three coalgebra wrappers live in **App I**
> (I.3/I.4/I.5). The #64 issue body's "Remark 2.13 dual / App B + App J" was
> imprecise; this registry carries the verified anchors.

## Comonoid / weight-tying / para-composition laws

| Witness | Anchor | Law |
|---|---|---|
| `comonoid_laws::diagonal_*` | CDL Thm G.10 | diagonal comonoid: coassoc + left/right counit |
| `comonoid_laws::tie_weights_end_to_end_diagonal_smoke` | CDL Thm G.10 | weight tying = diagonal reparameterization |
| `para_composition::{left,right}_unit_law_*` | CDL Thm G.10 / §3 | Para composition unit laws |
| `para_composition::reparameterization_diagonal_implements_weight_tying` | CDL Thm G.10 | reparam diagonal = weight tying |
| `para_composition::set_actegory_compose_action_reassociates_tuple` | CDL §3 | Set-actegory action associativity |

## Smoke / construction coverage

| Witness | Anchor | Law |
|---|---|---|
| `scaffold_smoke::*_constructs` | CDL Example 2.x / B.x | construction smoke (types inhabit) |
| `coalition_consumption_simulation::tie_weights_consumption_pathway_simulation` | CDL Thm G.10 | weight-tying consumption pathway (BTV21-adjacent) |
