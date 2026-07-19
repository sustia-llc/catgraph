# SMC Normal-Form Reconciliation (catgraph-applied)

> **Provenance:** reconstructed 2026-07-18 from the shipped code
> (`src/prop/presentation/smc_nf.rs`) and its regression + completeness tests
> (`tests/smc_nf_regression.rs`, `tests/smc_nf_completeness.rs`) during the
> paper audit (issue #116). The original working note (a pre-publication,
> gitignored artifact "reconciled from the 4 dpcs notes") was never committed
> and is unrecoverable; eight code/test comments cite it by section number.
> This document restores those sections from the behaviour the code actually
> implements, keeping the exact numbering the citations use (§2.1–§2.4, §3) so
> the references resolve without renumbering.
>
> **Anchor caveat:** the Joyal-Street (JS-I 1991, JS-II, JS-Braided 1993) and
> Selinger (2011, *A survey of graphical languages for monoidal categories*,
> [arXiv:0908.3347](https://arxiv.org/abs/0908.3347)) anchors below are carried
> over verbatim from the code and
> test citations. None of these four papers is in the local papers cache, so
> the page/theorem numbers are **cache-unverifiable** pending issue #117
> (Selinger fetch) and the Joyal-Street fetches. Every such anchor is marked
> **(†)**. The paper is still the spec (CLAUDE.md Rule 1) — the (†) marks
> "not yet re-checked against the source", not "suspected wrong".

## §1 What the normal form is

`smc_nf::nf` is a total function `PropExpr<G> → StringDiagram<G>` that
canonicalizes a prop expression up to symmetric-monoidal-category (SMC)
coherence: associativity and unitors of `;` and `⊗`, bifunctoriality /
interchange, braid naturality, and the symmetry axiom `σ² = id`. Two
expressions that are equal in the free SMC on the signature `G` reach the same
`StringDiagram`; the converse holds by construction, since every rewrite the
pipeline applies is SMC-sound.

A `StringDiagram` is a sequence of `Layer`s `L_0 ; L_1 ; … ; L_{k-1}`; each
`Layer` is a left-to-right tensor of `Atom`s (`Identity(n)`, `Braid(m,n)`,
`Generator(g)`). Lowering (`lower` / `pad_and_zip`) turns the expression tree
into a one-atom-per-layer diagram; the canonicalization steps in §3 then drive
it to normal form. The post-`nf` invariants are listed on the `StringDiagram`
type: no `Identity(0)`; no `Braid(m,n)` with `m+n > 2`; no `Braid(0,_)` /
`Braid(_,0)`; no two adjacent all-identity layers; every `Braid` in the
leading (input-side) layers; no mixed braid+generator layer; every
positive-source generator in its earliest admissible layer.

## §2 Conventions

The four conventions below are the choices that make the normal form *unique*
rather than merely *sound* — each resolves a coherence-equivalent ambiguity in
one fixed direction.

### §2.1 Direction convention (braids move to the input side)

Composition is written forward, in Rust's `;` order: `Compose(a, b)` means
"`a` then `b`", lowered to `a`'s layers followed by `b`'s layers.

Within a layer, atoms are tensored **left-to-right and the `Vec<Atom>`
preserves source tensor order** — the leftmost atom occupies the lowest-indexed
wires. All wire-position arithmetic (`wire_boundaries`, `covering_identity`,
`braid_at_position`) reads this source-order left-to-right.

Braids are canonicalized toward the **input (leading) layers**. The naturality
sweep in `collect_braid_prefix` rewrites each adjacent pair `L_gen ; L_braid →
L_braid' ; L_gen'`, sliding a braid earlier past a generator layer
(`try_naturality_swap`). When a `Braid(1,1)` at target-wire position `[p, p+1]`
slides past two atoms `X, Y`, the emitted input-side braid is `σ_{X.source,
Y.source}` (arities taken from the atoms' *source* widths, since the braid now
sits on their inputs), and `X, Y` swap in the trailing generator layer. A
wide-braid decomposition uses the forward-`;` `(B2)` splitting, i.e.
`σ_{2,1} = (id₁ ⊗ σ_{1,1}) ; (σ_{1,1} ⊗ id₁)`; the mirror `(B1)` would give the
`σ_{1,2}` split and is *not* used.

- Anchors: JS-II §1.2 α-anchor (†); JS-Braided p. 36 "box slides through
  crossing" (†); JS-Braided Def 2.1 axiom (B2) p. 33 (†).

### §2.2 Wide-braid handling (expand only, never collapse)

A `Braid(m,n)` with `m+n > 2` ("wide braid") is always **expanded** into a
layered sequence of `Braid(1,1)` bricks by `hexagon_expand`; the normal form
never runs the reverse (bricks → wide braid). `hexagon_expand` fires on any
layer whose only non-`Identity` atom is a single wide braid — including
identity-padded layers such as `[Identity(p), Braid(2,1), Identity(s)]` — so
wide braids that appear *mid-normalization* (emitted by the naturality sweep's
`σ_{s_a,s_b}`, or exposed by `isolate_mixed_braid_layers`) are decomposed on the
next fixpoint pass. The decomposition is the bubble-sort of the braid's
underlying permutation `π = [m..m+n, 0..m]`, giving a canonical reduced word of
adjacent transpositions.

This is what keeps the "no `Braid(m,n)` with `m+n > 2`" invariant true and why
the measure in §2.4 places `wide_braid_count` ahead of the braid-position term:
a naturality-emitted wide braid is expanded before braid positions are
compared, so no wide braid survives to the fixpoint check.

- Anchors: JS-Braided Prop 2.1 / axiom (B2) p. 33–34,
  `c_{U⊗V,W} = (c_{U,W} ⊗ 1_V) ∘ (1_U ⊗ c_{V,W})` (†); JS-I Ch 2 Thm 2.3 p. 81
  via the `S_n` presentation (†).

### §2.3 Canonical order (source order within a layer, earliest layer across)

Two independent choices fix the placement of atoms:

1. **Within a layer** — atoms keep source tensor order (§2.1): the layer's
   `Vec<Atom>` is left-to-right in wire index. Distinct lowering paths of the
   same morphism are forced to the same atom-boundary structure by eager
   identity fusion (`merge_adjacent_identities` in `pad_and_zip`) and by
   boundary refinement (`refine_to_common_boundaries`) before column merges.

2. **Across layers** — every `Generator` with positive source arity is sifted
   up to its **earliest admissible (braid-free) layer** by
   `topological_layer_order` (Step 4(c)). "Admissible" means the generator's
   consumed wire span at the `j−1 ; j` boundary is fully covered by an
   `Identity` region of a braid-free layer `j−1`; the covering identity is
   split around the generator and an `Identity(target)` is left behind. This is
   the interchange (`(id ⊗ g) ; (h ⊗ id) = h ⊗ g`) canonicalization: it forces
   the issue-#14 C2 scheduling witnesses (same morphism, independent atoms
   placed in different layers) onto one earliest-schedule form.

   Two atoms are deliberately excluded from the sift: braids never sift (their
   placement is §2.1's job, and letting both passes move atoms would oscillate
   the fixpoint), and **zero-source** generators (`η : 0 → 1`) are skipped —
   their consumed span is empty, so "earliest admissible layer" is positionally
   ambiguous. That skip is the known mid-layer-`η` gap
   (`interchange_zero_source_eta_known_gap`, issue #14 follow-up). Target-0
   sinks (`ε : 1 → 0`) have a non-empty source span and sift normally.

- Anchors: JS-I Ch 1 Prop 1.1 p. 66 (rectangle-cover independence) (†); JS-I
  Ch 1 §4 Thm 1.2 p. 71 (bifunctoriality / interchange) (†); issue #14.

### §2.4 Termination measure

`nf` runs the seven steps of §3 in a fixpoint loop, exiting when a full pass
leaves the diagram unchanged. Termination is by a **lexicographic measure** on
the tuple

```
(crossings,
 mixed_layer_count,
 wide_braid_count,
 braid_position_sum,
 generator_position_sum,
 layer_count)
```

with each step non-increasing on the whole tuple and at least one step strictly
decreasing whenever the diagram is not yet a fixpoint:

| Component | Step that strictly decreases it | Note |
|---|---|---|
| `crossings` | `reduce_involution` (`σ;σ → id`) | `hexagon_expand` leaves it fixed (preserves the underlying permutation) |
| `mixed_layer_count` | `isolate_mixed_braid_layers` (inside `collect_braid_prefix`) | the mixed-merge refusal at `reduce_involution`'s merge site stops any step re-creating a mixed layer |
| `wide_braid_count` | `hexagon_expand` (`Braid(m,n), m+n>2 → Braid(1,1)` bricks) | ordered ahead of `braid_position_sum` so a naturality-emitted wide braid is decomposed before positions are compared |
| `braid_position_sum` | the naturality sweep (braids move input-ward) | §2.1 |
| `generator_position_sum` | `topological_layer_order` (Step 4(c)) | one generator drops exactly one layer per sift; bounded below by 0 |
| `layer_count` | `coalesce_identity_layers` / `simplify_units` | identity-only layers absorb; `Identity(0)` atoms are removed |

`topological_layer_order` has its own inner termination for the same reason:
each sift strictly decreases the sum of the layer indices of `Generator` atoms
(one generator drops one layer; nothing else moves), bounded below by zero.

- Anchors: JS-I Ch 1 §4 Thm 1.2 p. 71 (†); JS-I Ch 2 §1 axiom (S) p. 73 (†).

## §3 Step table and paper coverage matrix

### Step table (pipeline order, as staged in the `nf` fixpoint loop)

| Step | Function | Effect |
|---|---|---|
| 0 | `normalize_empty_braids` | `Braid(0,n) → Identity(n)`, `Braid(m,0) → Identity(m)` (runs first so Step 1 never recurses on an already-identity braid) |
| 1 | `hexagon_expand` | wide `Braid(m,n)` (`m+n>2`) → `Braid(1,1)` bricks (§2.2) |
| 2 | `reduce_involution` | column-wise adjacent-layer compose: `id;id`, `id;X`, `X;id`, and `σ_{m,n};σ_{n,m} → id_{m+n}`; also `try_unitor_merge` 0-arity sink/source absorption; mixed layers refused at the merge site |
| 3 | `collect_braid_prefix` | (0) `isolate_mixed_braid_layers`, (a) naturality sweep (braids → input, §2.1), (b) `canonicalize_braid_runs` (permutation → canonical bubble-sort word) |
| 4 | `coalesce_identity_layers` | (a) fuse adjacent `Identity` atoms in a layer; (b) drop pure-identity layers when a non-identity layer remains (keep one as arity carrier otherwise) |
| 4(c) | `topological_layer_order` | sift each positive-source generator to its earliest admissible braid-free layer (§2.3) |
| 5 | `simplify_units` | remove `Identity(0)` atoms; drop layers emptied as a result |

(`lower` / `pad_and_zip` run once before the loop: `PropExpr` → one-atom-per-
layer `StringDiagram`, padding the shorter side of a `⊗` with `Identity`
layers.)

### Paper coverage matrix

Each SMC statement the code/tests anchor, mapped to the step (§3) or the
regression test that exercises it. All external-paper anchors are (†)
cache-unverifiable (see the header caveat).

| Statement | Anchor | Step / test |
|---|---|---|
| Rectangle-cover independence `v(Γ)=v(Γ[u,b])∘v(Γ[a,u])`; `;` associativity | JS-I Ch 1 Prop 1.1 p. 66 (†) | `lower`; Step 4; `ch1_prop_1_1_compose_associativity`, `compose_associator` |
| Layering of abstract diagrams | JS-I Ch 2 Prop 2.1 p. 78 (†) | `lower` |
| `⊗` bifunctoriality / interchange `(f⊗g);(h⊗k)=(f;h)⊗(g;k)` | JS-I Ch 1 §4 Thm 1.2 p. 71 (†) | `pad_and_zip` (§4 p. 69–70), Steps 3(0)/4(c); `ch1_thm_1_2_s4_interchange`, `smc_bifunctoriality_interchange`, `interchange`, `c2_scheduling_witness_converges`, `target_zero_sink_sifts_up` |
| `;` left/right unitor; invertible diagram `v(Γ)=id` | JS-I Ch 1 §3 p. 65 + Prop 1.1 p. 66 (†) | Step 2 (`try_column_merge` identity cases); `ch1_invertible_left_right_unitor`, `compose_unitors` |
| `⊗` strict unit `id_0` (bracket-clique skeleton p. 58) | JS-I Ch 1 §1 p. 57 (†) | Step 5; `ch1_s1_strict_unit`, `tensor_unitors` |
| Symmetry axiom (S) `c_{B,A}∘c_{A,B}=1_{A⊗B}` | JS-I Ch 2 §1 axiom (S) p. 73 (†); JS-Braided (S) p. 21 (†) | Step 0, Step 2 (`σ;σ → id`); `ch2_s1_axiom_s_braid_involution`, `aligned_braid_band_cancels_through_generators` |
| Braid naturality `σ_{1,1};(g⊗f)=(f⊗g);σ_{1,1}` (anchored form, Cor 2.3 p. 80) | JS-I Ch 2 Thm 2.2 p. 79 (†) | Step 3(a); `ch2_thm_2_2_braid_naturality`, `test_braid_naturality_right` |
| Free symmetric on `𝒟`; `σ_{2,1}=(id₁⊗σ_{1,1});(σ_{1,1}⊗id₁)` | JS-I Ch 2 Thm 2.3 p. 81 (†) | Step 1; `ch2_thm_2_3_symmetry_on_larger_tensors`, `wide_braid_*` |
| Hexagon (B2) `c_{U⊗V,W}=(σ_{U,W}⊗1_V)∘(1_U⊗σ_{V,W})` | JS-Braided Def 2.1 (B2) p. 33–34 (†) | Step 1 (`decompose_braid`); `test_hexagon_sigma_on_tensor` |
| Yang-Baxter / Artin 3-strand `s_i s_{i+1} s_i = s_{i+1} s_i s_{i+1}` (Reidemeister III) | JS-Braided Example 2.1 (A1) p. 35 (†) | Step 3(b); `test_yang_baxter`, `test_braid_interaction_with_identity` |
| Reduced-word canonicality of `S_n`; braid run = underlying permutation | JS-Braided Cor 2.6 p. 44 (†); JS-I Ch 2 §1 + Ch 3 p. 84 (†) | Step 3(b) `canonicalize_braid_runs` |
| Symmetric categories are balanced (transposition squares collapse) | JS-Braided Example 6.1 p. 66 (†) | Step 2 + Step 4; `test_symmetric_collapse_3_strands` |
| Braid slides through box | JS-Braided p. 36 (†); JS-II p. 5 functoriality of `α↦⟨α⟩` (†) | Step 3(a) `try_naturality_swap`; `braid_layer_blocks_sift` |
| Braids-to-input direction | JS-II §1.2 α-anchor (†) | §2.1; Step 3(a) |
| Planar deformation `id;f;id=f` (empty slice) | JS-II Thm 1.1.2 p. 3–4 (†); Thm 1.1.3 p. 4 (†) | Step 4; `planar_identity_layer_coalesce` |
| 3D deformation + surgery `σ;(f⊗id₁);σ=id₁⊗f` | JS-II Thm 1.2.2 + Thm 1.2.3 p. 6–7 (†) | Steps 2+3 in tandem; `braid_sandwich_is_identity_tensor` |
| Generators are uninterpreted formal symbols (distinct symbols stay distinct) | Selinger §2 p. 7 + §3 p. 12 (†) | whole NF; `smc_generators_are_uninterpreted_black_boxes` |
| SMC self-inverse braid (two crossings cancel; braided would not) | Selinger §3.5 Thm 3.12 p. 17 vs §3.3 Thm 3.7 (†) | Step 2; `smc_two_crossings_cancel_but_braided_would_not` |
| Interchange law; `id_0` as unit ("zero wires") | Selinger Table 2 p. 10 (†) | Steps 2/5; `smc_bifunctoriality_interchange` |
| 0-arity sink/source absorption `L1;(X⊗id_k)=X⊗L1` etc. | JS-I Ch 1 §1 + §4 Thm 1.2 p. 71 (†) | Step 2 `try_unitor_merge`; `unitor_merge_*` |

### Coverage summary

- **SMC coherence axioms** — associativity, unitors (both products),
  bifunctoriality/interchange, strict unit: **covered** by the pipeline and the
  JS-I / Selinger regression suite.
- **Symmetry layer** — `σ² = id`, braid naturality, hexagon/`σ_{m,n}`
  decomposition, Yang-Baxter, `S_n` reduced-word canonicality: **covered** by
  Steps 0–3 and the JS-Braided / JS-II suite.
- **Known open gap** — mid-layer **zero-source** (`η : 0 → 1`) scheduling is
  not canonical (§2.3); tracked as `interchange_zero_source_eta_known_gap`
  (`#[ignore]`) and the Watch-item in `tests/smc_nf_completeness.rs`, issue #14
  follow-up.
