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
> **Anchor provenance:** the Joyal-Street (JS-I 1991, JS-II, JS-Braided 1993)
> and Selinger (2011, *A survey of graphical languages for monoidal
> categories*, [arXiv:0908.3347](https://arxiv.org/abs/0908.3347)) anchors
> below were carried over verbatim from the code and test citations, and have
> **all been verified against the private papers cache** (#117: Selinger +
> JS-II 2026-07-19 by text; JS-I + JS-Braided the same day from page images
> of the journal scans, after the Elsevier copies were placed — option (b)).
> Two findings from the verification passes, both handled:
>
> - Selinger's symmetric-coherence **Thm 3.12 sits on p. 18, not p. 17** —
>   p. 17 holds §3.5's self-inverse symmetry definition, which is the
>   substantively cited content (corrected here and in the regression
>   doc-comment).
> - JS-I prints **two theorems headed "Theorem 1.2"**: the planar-deformation
>   theorem (p. 66 — the one Selinger's Thm 3.1 cites as `[22, Thm. 1.2]`)
>   and the 𝔽(𝒟)-freeness theorem (p. 71, in Ch 1 §4 — the one cited below;
>   the paper's own p. 81 cross-references call it "Theorem 1.3", so the
>   p. 71 heading is a misprint in the original). The "Ch 1 §4 … p. 71"
>   locator used here disambiguates; interchange is its proof item (f) +
>   Fig 1.9.
>
> Every other page/theorem locator below was verified exact as written.
> Earlier interim scaffolding — (†) cache-unverifiable marks and (‡ Sel /
> ‡ MMR86) cross-check marks against Selinger's restatements and the 1986
> Macquarie precursor report — is retired; see the git history / PRs
> #133–#136 for the audit trail.

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
leading (input-side) layers; no mixed braid+generator layer; every generator in
its earliest admissible layer (positive-source by covering span, zero-source by
the component-anchored point span); and within every layer, no adjacent
strictly-commuting pair ordered against the Step 6 order — `scalar < η < ε` at a
single-atom tie, `closed < input-anchored < output-only` otherwise (§2.6).

## §2 Conventions

The six conventions below are the choices that make the normal form *unique*
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

- Anchors: JS-II §1.2 α-anchor (Remark 1.2.1 p. 6); JS-Braided p. 36 "box
  slides through crossing" (the pictorial naturality proof of `c_{m,n}`);
  JS-Braided Def 2.1 axiom (B2) p. 33 (B1 = its mirror via `c⁻¹`, noted
  right below the diagrams).

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
  `c_{U⊗V,W} = (c_{U,W} ⊗ 1_V) ∘ (1_U ⊗ c_{V,W})`; JS-I Ch 2 Thm 2.3 p. 81
  (`𝔽_s(𝒟)` free symmetric) via the `S_n` presentation.

### §2.3 Canonical order (source order within a layer, earliest layer across)

Two independent choices fix the placement of atoms:

1. **Within a layer** — atoms keep source tensor order (§2.1): the layer's
   `Vec<Atom>` is left-to-right in wire index. Distinct lowering paths of the
   same morphism are forced to the same atom-boundary structure by eager
   identity fusion (`merge_adjacent_identities` in `pad_and_zip`) and by
   boundary refinement (`refine_to_common_boundaries`) before column merges.

2. **Across layers** — every `Generator` is sifted up to its **earliest
   admissible (braid-free) layer** by `topological_layer_order` (Step 4(c)).
   For a generator with **positive source arity**, "admissible" means the
   consumed wire span at the `j−1 ; j` boundary is fully covered by an
   `Identity` region of a braid-free layer `j−1`; the covering identity is
   split around the generator and an `Identity(target)` is left behind. This is
   the interchange (`(id ⊗ g) ; (h ⊗ id) = h ⊗ g`) canonicalization: it forces
   the issue-#14 C2 scheduling witnesses (same morphism, independent atoms
   placed in different layers) onto one earliest-schedule form.

   **Zero-source** generators (`η : 0 → 1`) sift by the **component-anchored
   point-span rule** (issue #55 PR2): the empty consumed span reduces to a
   single output coordinate `q` at the source cursor, so `η` slides into the
   earliest braid-free layer `j−1` iff `q` is an atom boundary there (insert
   between the adjacent atoms, e.g. the `[F, G]` boundary in the witness) or
   strictly inside one of its identities (split that identity). It is blocked
   when `q` is strictly inside a generator's output span (whose wires cannot be
   split).

   Where `q` admits **several** slots — a run of target-0 atoms all sits at that
   coordinate — the choice is *not* made from the cursor. The cursor position of
   a zero-source atom is presentation-dependent, and anchoring on it makes the
   normal form presentation-dependent too (the counterexample recorded in the
   2026-07-26 diagnosis note §2). The slot comes instead from the **component
   order anchor**, rule (i) — see §2.6. Target-0 sinks (`ε : 1 → 0`) have a
   non-empty source span and sift via the positive-source path. Only **braids**
   are excluded from the sift entirely (their placement is §2.1's job, and
   letting both passes move atoms would oscillate the fixpoint).

   Choice 1 leaves one residual freedom that wire order does not decide — the
   relative order of *strictly commuting* zero-arity atoms within a layer. §2.5
   fixes it at the atom level, §2.6 at the component level.

- Anchors: JS-I Ch 1 Prop 1.1 p. 66 (rectangle-cover independence); JS-I
  Ch 1 §4 Thm 1.2 p. 71 (𝔽(𝒟) freeness — interchange is proof item (f) +
  Fig 1.9; see the header note on the heading misprint); issue #14.

### §2.4 Termination measure

`nf` runs the eight steps of §3 in a fixpoint loop, exiting when a full pass
leaves the diagram unchanged. Termination is by a **lexicographic measure** on
the tuple

```
(crossings,
 mixed_layer_count,
 wide_braid_count,
 braid_position_sum,
 generator_position_sum,
 layer_count,
 tied_inversion_count)
```

with each step non-increasing on the whole tuple and at least one step strictly
decreasing whenever the diagram is not yet a fixpoint:

| Component | Step that strictly decreases it | Note |
|---|---|---|
| `crossings` | `reduce_involution` (`σ;σ → id`) | `hexagon_expand` leaves it fixed (preserves the underlying permutation) |
| `mixed_layer_count` | `isolate_mixed_braid_layers` (inside `collect_braid_prefix`) | the mixed-merge refusal at `reduce_involution`'s merge site stops any step re-creating a mixed layer |
| `wide_braid_count` | `hexagon_expand` (`Braid(m,n), m+n>2 → Braid(1,1)` bricks) | ordered ahead of `braid_position_sum` so a naturality-emitted wide braid is decomposed before positions are compared |
| `braid_position_sum` | the naturality sweep (braids move input-ward) | §2.1 |
| `generator_position_sum` | `topological_layer_order` (Step 4(c)) | one generator — positive-source or zero-source `η` (issue #55) — drops exactly one layer per sift; bounded below by 0 |
| `layer_count` | `coalesce_identity_layers` / `simplify_units` | identity-only layers absorb; `Identity(0)` atoms are removed |
| `tied_inversion_count` | `reorder_tied_zero_arity` (Step 6, §2.5 + §2.6) | trailing component; `try_unitor_merge` and the zero-source sift may *raise* it while strictly shrinking an earlier component (`layer_count` / `generator_position_sum`) — see below |

`topological_layer_order` has its own inner termination for the same reason:
each sift strictly decreases the sum of the layer indices of `Generator` atoms
(one generator drops one layer; nothing else moves), bounded below by zero.

**`tied_inversion_count`, precisely.** Summed over layers, the number of *pairs*
`i < j` within a layer whose class ranks are inverted, i.e.
`class(atom_i) > class(atom_j)` for the §2.5 order `scalar < η < ε < solid`. Each
Step-6 swap flips exactly one such pair from inverted to non-inverted and leaves
every other pair's relative order untouched, so the count drops by exactly one
per swap. Counting only *adjacent* inverted pairs would **not** work: in
`[ε, ε′, η]` the ε's do not commute, so the only available swap moves the η past
`ε′`, trading the adjacent inversion `(ε′, η)` for the new adjacent inversion
`(ε, η)` — an adjacent-only count is flat there, while the all-pairs count drops
2 → 1.

Step 6 leaves every earlier component fixed: it never moves an atom across a
layer boundary, never rewrites an atom, and never changes a layer's membership,
so `crossings`, `mixed_layer_count`, `wide_braid_count`, the two position sums
(both layer-index sums) and `layer_count` are all invariant under it. Two steps
can raise `tied_inversion_count`. `try_unitor_merge` — its case 1
prepends an ε ahead of the absorbed layer's atoms (possibly past an η) and its
case 4 appends an η after them (possibly behind an ε); case 3's η-prepend is
order-canonical — but it does so only while
strictly shrinking `layer_count`, so the tuple still drops lexicographically and
Step 6 repairs the ordering on the same fixpoint pass. The zero-source
point-span sift can likewise land an `η` beside an `ε` in the earlier layer, but
only while strictly shrinking `generator_position_sum`, an earlier component
still — same argument, same repair.

`tied_inversion_count` is read against the *Step 6 order* of §2.5 + §2.6, not
the class order alone: an "inversion" is an adjacent strictly-commuting pair
that Step 6 would swap. Each swap fixes exactly the pair it swaps and can never
re-invert it (the comparator is fixed for a given diagram, and every swap is
connectivity-preserving, so the component keys it reads do not change), so the
count drops by one per swap regardless of which branch of the §2.6 carve
governs.

- Anchors: JS-I Ch 1 §4 Thm 1.2 p. 71; JS-I Ch 2 §1 axiom (S) p. 73.

### §2.5 Within-layer order of zero-arity atoms (η before ε)

§2.3's "source order within a layer" leaves one genuine freedom. Two adjacent
atoms `A`, `B` commute **strictly** — both connecting braids degenerate to
`σ_{0,n} = id`, so `A ⊗ B` and `B ⊗ A` are the same morphism on the nose — iff

```
(src A = 0 ∨ src B = 0) ∧ (tgt A = 0 ∨ tgt B = 0)
```

so η's (`0 → n`) never commute with η's and ε's (`n → 0`) never commute with
ε's (disjoint non-empty spans fix their relative order); an η and an ε commute
at a tied adjacency; a `0 → 0` scalar commutes with every atom; and a solid atom
(`src > 0 ∧ tgt > 0`, including `Identity(n>0)` and every `Braid`) never
strictly commutes with an η or an ε.

The canonical order is **η before ε, scalars leftmost** (issue #55 Decision 1,
owner call 2026-07-25):

```
scalar (0→0)  <  η (0→n)  <  ε (n→0)  <  solid
```

Step 6 (`reorder_tied_zero_arity`) realizes it as a within-layer bubble reorder:
swap an adjacent `(A, B)` iff they strictly commute and `class(B) < class(A)`.
That is the greedy (lex-least) normal form of the layer's word in the trace
monoid generated by strict commutation. Rationale: it matches the NF's uniform
"earliest admissible" principle (braids to the leading layers, generators sifted
to the earliest layer — η goes as early/left as feasible).

Equal classes never swap, so scalar order is **stable** rather than sorted:
`PropSignature` carries no `Ord` bound and no shipped signature has a `0 → 0`
generator (Mat(R)/SFG scalars are `1 → 1`; `FrobeniusOr`'s η/ε are `0 → 1` /
`1 → 0`), so a total scalar order awaits a signature that exercises it.

Stability under substitution: the rule is stated per tied adjacency at a wire
coordinate, and ambient padding only adds `Identity` atoms with `src, tgt > 0`,
which never join a tied run — so embedding a layer in a wider context preserves
its tied runs and hence its canonical order.

This closes the within-layer half of issue #55 (`nf(ε ⊗ η) = nf(η ⊗ ε)`) **at
single-atom ties**. The layer-assignment half — tensor-forms vs compose-forms
such as `ε ; η` — is also closed, by the §2.3 point-span sift together with the
component anchor of §2.6, which is where multi-atom ties are decided.

- Anchors: JS-I Ch 1 §4 Thm 1.2 p. 71 (bifunctoriality) specialized to a
  0-arity edge — the same `id_0`-unitor derivation `try_unitor_merge` uses;
  JS-I Ch 1 §1 p. 57 (`id_0` as ⊗-unit). Issue #55, design of record
  `.claude/docs/2026-07-25-55-tensor-order-canonicalization.md`.

### §2.6 Component-order anchor (rule (i))

§2.5 orders *atoms* at a tied adjacency. That is not enough to make the normal
form a function of the diagram's abstract content: a zero-source atom's wire
coordinate is presentation-dependent, so anchoring its placement on the cursor
lets two SMC-equal expressions reach different fixpoints. The decisive
counterexample: with `A = μ ; ! : 2 → 0` and `B = η ; Δ : 0 → 2`,
bifunctoriality with `id₀` gives `A ; B = A ⊗ B`, yet a cursor-anchored sift
block-transposes the two connected components in the compose-form.

Rule (i) fixes the freedom at **component** granularity. Take the connected
components of the diagram — union-find over *all* atoms (`Identity` and `Braid`
atoms carry wires and belong to components too), joining two atoms whenever a
wire leaving one at a layer boundary is a wire entering the other at the same
boundary and coordinate.

The join must be on a **shared wire**, not on touching intervals: a zero-arity
atom's interval at one boundary is empty (an `ε` has no target wire, an `η` no
source wire), and an empty interval shares no wire with the neighbour it abuts.
Getting this wrong makes a zero-arity atom's component depend on where it sits
in its layer — which is exactly what Step 6 permutes, so the comparator would
change under Step 6's own swaps and the `nf` fixpoint would oscillate rather
than terminate. (Regression:
`component_analysis_fixpoint_terminates_on_tied_pair_beside_braid` in
`tests/smc_nf_regression.rs`.) With the guard, component membership is a
function of the diagram's connectivity alone, and every strictly-commuting swap
leaves it invariant.

Classify each component by its boundary attachment and order the classes:

```
closed (touches neither boundary)  <  input-anchored  <  output-only
```

with each anchored class ordered by its **least attached boundary coordinate**
(least input coordinate for input-anchored, least output coordinate for
output-only). A component that touches *both* boundaries counts as
input-anchored. All closed components share one key and so keep their input
order — the block-level reading of §2.5's scalars-leftmost, since an atomic
`0 → 0` scalar *is* a closed component.

**The disjointness carve.** An atomic `η ∥ ε` pair is both a §2.5 tied
adjacency (η first) and a rule-(i) component transposition (input-anchored ε
first). The two freedom classes are carved apart by component size:

- **tied pairs of single-atom components** → §2.5 / Decision 1, η first;
- **anything involving a multi-atom component** → rule (i)'s component order,
  with the §2.5 class order as the tie-break when the two component keys
  coincide (the same component, or two closed blocks).

Both sides of the carve are functions of the abstract content (component
membership and size are content), so the normal form stays well defined. The
carve is applied in two places, consistently: `component_slot` picks the sift's
insertion slot inside the run of slots the coordinate admits, and Step 6's
comparator (`tie_sorts_before`) decides tied adjacencies. Because a
strictly-commuting swap moves an atom with source width 0 past one with target
width 0, it changes no other atom's wire coordinates — the component analysis is
invariant under Step 6, which is why the two passes agree and do not oscillate
(the sift moves atoms only up a layer, Step 6 only within a layer).

**Interleave guard (guard 3).** Rule (i)'s order only makes sense when the
components' attached coordinates on a boundary are disjoint intervals ordered by
least coordinate. When one component's attachment interleaves another's on the
same boundary, block transposition is not braid-free and the rule-(i) slot is
ill-defined. Such components are marked and left alone: their `η`s are not
sifted, and Step 6 falls back to the §2.5 class order for them. Canonicality is
claimed and proven on the **non-interleaved fragment**; the residual is strictly
narrower than the pre-PR2 gap, which covered *every* mid-layer `η`.

**Residual, second kind.** Rule (i) states an order between whole components,
but the pipeline's only moves are the single-atom sift (up one layer) and Step
6's within-layer reorder. Transposing two *multi-atom* blocks is neither — it is
a coupled multi-layer block move — so tensor-form block transpositions such as
`(η;!) ⊗ s` vs `s ⊗ (η;!)`, or `A ⊗ B` vs `B ⊗ A` above, still normalize apart.
Realizing them needs a block-level analogue of Step 6 and is out of PR2's scope;
it is recorded as `closed_block_transposition_is_a_documented_residual`
(`#[ignore]`) in `tests/smc_nf_completeness.rs`.

- Source: issue #55 owner decision 2026-07-26, diagnosis-note addendum
  (`2026-07-26-55-proof-phase-diagnosis.md`), refining the design of record
  `.claude/docs/2026-07-25-55-tensor-order-canonicalization.md`.
- Anchors: JS-I Ch 1 §4 Thm 1.2 p. 71 (bifunctoriality) with a 0-arity edge, as
  in §2.5; JS-I Ch 1 §1 p. 57 (`id_0` as ⊗-unit).

## §3 Step table and paper coverage matrix

### Step table (pipeline order, as staged in the `nf` fixpoint loop)

| Step | Function | Effect |
|---|---|---|
| 0 | `normalize_empty_braids` | `Braid(0,n) → Identity(n)`, `Braid(m,0) → Identity(m)` (runs first so Step 1 never recurses on an already-identity braid) |
| 1 | `hexagon_expand` | wide `Braid(m,n)` (`m+n>2`) → `Braid(1,1)` bricks (§2.2) |
| 2 | `reduce_involution` | column-wise adjacent-layer compose: `id;id`, `id;X`, `X;id`, and `σ_{m,n};σ_{n,m} → id_{m+n}`; also `try_unitor_merge` 0-arity sink/source absorption; mixed layers refused at the merge site |
| 3 | `collect_braid_prefix` | (0) `isolate_mixed_braid_layers`, (a) naturality sweep (braids → input, §2.1), (b) `canonicalize_braid_runs` (permutation → canonical bubble-sort word) |
| 4 | `coalesce_identity_layers` | (a) fuse adjacent `Identity` atoms in a layer; (b) drop pure-identity layers when a non-identity layer remains (keep one as arity carrier otherwise) |
| 4(c) | `topological_layer_order` | sift each generator to its earliest admissible braid-free layer — covering-identity span for positive source, component-anchored point-span rule for zero-source `η` (§2.3, §2.6) |
| 5 | `simplify_units` | remove `Identity(0)` atoms; drop layers emptied as a result |
| 6 | `reorder_tied_zero_arity` | within-layer bubble reorder of strictly-commuting zero-arity atoms — `scalar < η < ε < solid` at single-atom ties (§2.5), component order otherwise (§2.6) |

(`lower` / `pad_and_zip` run once before the loop: `PropExpr` → one-atom-per-
layer `StringDiagram`, padding the shorter side of a `⊗` with `Identity`
layers.)

### Paper coverage matrix

Each SMC statement the code/tests anchor, mapped to the step (§3) or the
regression test that exercises it. All external-paper anchors are
cache-verified (2026-07-19, #117 — see the header provenance note).

| Statement | Anchor | Step / test |
|---|---|---|
| Rectangle-cover independence `v(Γ)=v(Γ[u,b])∘v(Γ[a,u])`; `;` associativity | JS-I Ch 1 Prop 1.1 p. 66 | `lower`; Step 4; `ch1_prop_1_1_compose_associativity`, `compose_associator` |
| Layering of abstract diagrams | JS-I Ch 2 Prop 2.1 p. 78 | `lower` |
| `⊗` bifunctoriality / interchange `(f⊗g);(h⊗k)=(f;h)⊗(g;k)` | JS-I Ch 1 §4 Thm 1.2 p. 71 (𝔽(𝒟) freeness; interchange = proof item (f) + Fig 1.9) | `pad_and_zip` (§4 p. 69–70), Steps 3(0)/4(c); `ch1_thm_1_2_s4_interchange`, `smc_bifunctoriality_interchange`, `interchange`, `c2_scheduling_witness_converges`, `target_zero_sink_sifts_up`, `interchange_zero_source_eta`, `smc_canonicality_probes::*` |
| `;` left/right unitor; invertible diagram `v(Γ)=id` | JS-I Ch 1 §3 p. 65 + Prop 1.1 p. 66 | Step 2 (`try_column_merge` identity cases); `ch1_invertible_left_right_unitor`, `compose_unitors` |
| `⊗` strict unit `id_0` (bracket-clique skeleton p. 58) | JS-I Ch 1 §1 p. 57 | Step 5; `ch1_s1_strict_unit`, `tensor_unitors` |
| Symmetry axiom (S) `c_{B,A}∘c_{A,B}=1_{A⊗B}` | JS-I Ch 2 §1 axiom (S) p. 73; JS-Braided (S) p. 21 | Step 0, Step 2 (`σ;σ → id`); `ch2_s1_axiom_s_braid_involution`, `aligned_braid_band_cancels_through_generators` |
| Braid naturality `σ_{1,1};(g⊗f)=(f⊗g);σ_{1,1}` (anchored form, Cor 2.3 p. 80) | JS-I Ch 2 Thm 2.2 p. 79 | Step 3(a); `ch2_thm_2_2_braid_naturality`, `test_braid_naturality_right` |
| Free symmetric on `𝒟`; `σ_{2,1}=(id₁⊗σ_{1,1});(σ_{1,1}⊗id₁)` | JS-I Ch 2 Thm 2.3 p. 81 (`𝔽_s(𝒟)` free symmetric) | Step 1; `ch2_thm_2_3_symmetry_on_larger_tensors`, `wide_braid_*` |
| Hexagon (B2) `c_{U⊗V,W}=(σ_{U,W}⊗1_V)∘(1_U⊗σ_{V,W})` | JS-Braided Def 2.1 (B2) p. 33–34 (B1 = mirror via `c⁻¹`) | Step 1 (`decompose_braid`); `test_hexagon_sigma_on_tensor` |
| Yang-Baxter / Artin 3-strand `s_i s_{i+1} s_i = s_{i+1} s_i s_{i+1}` (Reidemeister III) | JS-Braided Example 2.1 (A1) p. 35; JS-I Ch 3 p. 84 (same (A1)/(A2) presentation) | Step 3(b); `test_yang_baxter`, `test_braid_interaction_with_identity` |
| Reduced-word canonicality of `S_n`; braid run = underlying permutation | JS-Braided Cor 2.6 p. 44 (underlying braid decides commutativity); JS-I Ch 2 §1 + Ch 3 p. 84 (`S_n` = `𝔹_n` + `s_i² = 1`; canonical surjection `𝔹_n → 𝕊_n`) | Step 3(b) `canonicalize_braid_runs` |
| Symmetric categories are balanced (transposition squares collapse) | JS-Braided Example 6.1 p. 66 | Step 2 + Step 4; `test_symmetric_collapse_3_strands` |
| Braid slides through box | JS-Braided p. 36 (pictorial naturality of `c_{m,n}`); JS-II p. 5 canonical iso `α↦⟨α⟩` | Step 3(a) `try_naturality_swap`; `braid_layer_blocks_sift` |
| Braids-to-input direction | JS-II §1.2 α-anchor (Remark 1.2.1 p. 6) | §2.1; Step 3(a) |
| Planar deformation `id;f;id=f` (empty slice) | JS-II Thm 1.1.2 p. 3–4; Thm 1.1.3 p. 4 | Step 4; `planar_identity_layer_coalesce` |
| 3D deformation + surgery `σ;(f⊗id₁);σ=id₁⊗f` | JS-II Thm 1.2.2 + Thm 1.2.3 p. 6–7 | Steps 2+3 in tandem; `braid_sandwich_is_identity_tensor` |
| Generators are uninterpreted formal symbols (distinct symbols stay distinct) | Selinger §2 p. 7 + §3 p. 12 | whole NF; `smc_generators_are_uninterpreted_black_boxes` |
| SMC self-inverse braid (two crossings cancel; braided would not) | Selinger §3.5 p. 17 (self-inverse def.) + Thm 3.12 p. 18 vs §3.3 Thm 3.7 p. 16 | Step 2; `smc_two_crossings_cancel_but_braided_would_not` |
| Interchange law; `id_0` as unit ("zero wires") | Selinger Table 2 p. 10 (+ interchange example below it) | Steps 2/5; `smc_bifunctoriality_interchange` |
| 0-arity sink/source absorption `L1;(X⊗id_k)=X⊗L1` etc. | JS-I Ch 1 §1 + §4 Thm 1.2 p. 71 | Step 2 `try_unitor_merge`; `unitor_merge_*` |
| Strictly-commuting zero-arity atoms `ε⊗η=η⊗ε` (both connecting braids `σ_{0,n}=id`) | JS-I Ch 1 §1 p. 57 (`id_0` unit) + Ch 1 §4 Thm 1.2 p. 71 | Step 6 `reorder_tied_zero_arity` (§2.5); `zero_arity_order::*` |

### Coverage summary

- **SMC coherence axioms** — associativity, unitors (both products),
  bifunctoriality/interchange, strict unit: **covered** by the pipeline and the
  JS-I / Selinger regression suite.
- **Symmetry layer** — `σ² = id`, braid naturality, hexagon/`σ_{m,n}`
  decomposition, Yang-Baxter, `S_n` reduced-word canonicality: **covered** by
  Steps 0–3 and the JS-Braided / JS-II suite.
- **Zero-arity within-layer order** — `scalar < η < ε < solid` at every
  single-atom tied adjacency: **covered** by Step 6 (§2.5) and the
  `zero_arity_order` tests (issue #55 PR1).
- **Zero-arity scheduling** — mid-layer **zero-source** (`η : 0 → 1`) *layer
  assignment* is canonical on the **non-interleaved fragment**, via the
  component-anchored point-span sift (§2.3 + §2.6, issue #55 PR2; the former
  issue #14 follow-up gap). Tensor- and compose-forms of the same morphism
  converge (`ε ⊗ η` = `ε ; η`, `F ⊗ η ⊗ G` = `(F ⊗ G) ; (id₁ ⊗ η ⊗ id₁)`,
  `(μ;!) ; (η;Δ)` = `(μ;!) ⊗ (η;Δ)`); verified by `interchange_zero_source_eta`,
  the `zero_arity_order` tests, and the `smc_canonicality_probes` module in
  `tests/smc_nf_completeness.rs`.
- **Documented residuals**, both in §2.6 and both narrower than the pre-PR2 gap:
  (a) an `η` whose component's boundary attachment **interleaves** another's is
  not sifted (guard 3); (b) transposing two **multi-atom blocks** needs a
  block-level analogue of Step 6 that the pipeline does not have, so
  `(η;!) ⊗ s` and `s ⊗ (η;!)` still normalize apart — tracked as
  `closed_block_transposition_is_a_documented_residual` (`#[ignore]`).
