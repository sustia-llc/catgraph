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
interchange, braid naturality, and the symmetry axiom `σ² = id`. The aim is
"SMC-equal iff same `StringDiagram`". The *soundness* direction — equal NFs
imply SMC-equal expressions — holds unconditionally, since every rewrite the
pipeline applies is SMC-sound (§4.3). The *canonicality* direction —
SMC-equal expressions reach the same `StringDiagram` — is probe-verified on
the fragment `𝔉` with three documented residuals (a fourth, the Ord-less
closed↔closed order, was closed by the #79 P1 content key); a full proof is
open (§4.4 status).

A `StringDiagram` is a sequence of `Layer`s `L_0 ; L_1 ; … ; L_{k-1}`; each
`Layer` is a left-to-right tensor of `Atom`s (`Identity(n)`, `Braid(m,n)`,
`Generator(g)`). Lowering (`lower` / `pad_and_zip`) turns the expression tree
into a one-atom-per-layer diagram; the canonicalization steps in §3 then drive
it to normal form. The post-`nf` invariants are listed on the `StringDiagram`
type: no `Identity(0)`; no `Braid(m,n)` with `m+n > 2`; no `Braid(0,_)` /
`Braid(_,0)`; no two adjacent all-identity layers; **no two adjacent
`Identity` atoms within a layer** (intra-layer fusion, `coalesce` (a)); **no
pure-identity layer while any non-identity layer remains** (one survives as
the arity carrier only in an all-identity diagram, `coalesce` (b)); every
`Braid` in the leading (input-side) layers; no mixed braid+generator layer;
**every maximal run of braid layers is the canonical bubble-sort schedule of
its underlying permutation** (§2.2, Step 3(b)); every generator in
its earliest admissible layer (positive-source by covering span, zero-source by
the component-anchored point span); within every layer, no adjacent
strictly-commuting pair ordered against the Step 6 order — `scalar < η < ε` at a
single-atom tie, `closed < input-anchored < output-only` otherwise (§2.6); and no
adjacent *free* pair of connected components ordered against that same component
order (Step 7, §2.6). (The three bolded clauses were implicit until the §4
review, which found the draft rigidity argument silently using them —
2026-07-27.)

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

`nf` runs the nine steps of §3 in a fixpoint loop, exiting when a full pass
leaves the diagram unchanged. Termination is by a **lexicographic measure** on
the tuple

```
(crossings,
 mixed_layer_count,
 wide_braid_count,
 braid_position_sum,
 generator_position_sum,
 layer_count,
 block_inversion_count,
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
| `block_inversion_count` | `reorder_component_blocks` (Step 7, §2.6) | one transposition flips every position pair between the two blocks' runs; the pass changes no layer's membership and rewrites no atom, so every earlier component is invariant under it |
| `tied_inversion_count` | `reorder_tied_zero_arity` (Step 6, §2.5 + §2.6) | trailing component; `try_unitor_merge`, the zero-source sift and Step 7 may *raise* it while strictly shrinking an earlier component (`layer_count` / `generator_position_sum` / `block_inversion_count`) — see below |

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

**`block_inversion_count`, precisely.** Summed over layers, the number of
*pairs* `p < q` within a layer whose two components form a §2.6 *free pair* — at
least one multi-atom, neither interleaved, at most one attaching each boundary —
inverted under the lexicographic pair (rule-(i) key, in-situ reading key). The
reading key (#79 P1) breaks the closed↔closed tie: the component's atoms read
layer-by-layer, left-to-right, each as (kind tag, widths, generator by the
`Ord` bound on `G`) — offset-independent, so it is invariant under the very
swaps it licenses; equal readings mean identical blocks, and identical blocks
are never swapped (the transposition would be invisible). A Step-7
transposition flips every such pair between the two runs (at least one, since
the two are adjacent in some layer) and leaves every other pair's relative
order untouched. The comparator is stable across the swap: boundary freedom
means neither block occupies a wire on a boundary the other attaches to, so
the swap moves no boundary wire and `keys`, `sizes`, `interleaved`, the
attachment flags and both blocks' readings are all invariant under it.

Step 6 leaves every earlier component fixed: it never moves an atom across a
layer boundary, never rewrites an atom, and never changes a layer's membership,
so `crossings`, `mixed_layer_count`, `wide_braid_count`, the two position sums
(both layer-index sums) and `layer_count` are all invariant under it.
`block_inversion_count` too: a Step-6 swap changes the relative order of exactly
the pair it swaps, and either that pair is two *single-atom* components — which
`block_inversion_count` excludes by the free-pair condition — or it is already
decided by the rule-(i) component order (`tie_sorts_before`), so the swap lowers
the block count or leaves it alone. Never raises it.

Three steps can raise `tied_inversion_count`. `try_unitor_merge` — its case 1
prepends an ε ahead of the absorbed layer's atoms (possibly past an η) and its
case 4 appends an η after them (possibly behind an ε); case 3's η-prepend is
order-canonical — but it does so only while
strictly shrinking `layer_count`, so the tuple still drops lexicographically and
Step 6 repairs the ordering on the same fixpoint pass. The zero-source
point-span sift can likewise land an `η` beside an `ε` in the earlier layer, but
only while strictly shrinking `generator_position_sum`, an earlier component
still — same argument, same repair. So can a Step-7 block move, while strictly
shrinking `block_inversion_count` — which is why Step 7 is staged *ahead* of
Step 6 in the loop.

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

Equal classes among *distinct* scalars sort by the `Ord` bound on `G`
(#79 P1; ascending, so the generator order is the scalar order); equal
generators never swap. No shipped signature has a `0 → 0` generator
(Mat(R)/SFG scalars are `1 → 1`; `FrobeniusOr`'s η/ε are `0 → 1` / `1 → 0`),
so the tie-break is exercised today only by the test signatures
(`tied_scalars_sort_by_generator_order` and friends) — behaviorally inert for
every shipped signature, which is why the CC baselines did not move.

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

Both sides of the carve are *meant to be* functions of the abstract content —
with two caveats recorded in §4: component membership and size are content
only modulo the fused-identity coarsening that the sift and Step 6 read
(§4.1's honesty note), and well-definedness of the normal form on all of `𝔉`
is exactly what residual §4.6(d) leaves open pending the §4.5 column move. The
carve is applied in two places, consistently: `component_slot` picks the sift's
insertion slot inside the run of slots the coordinate admits — for output-only
and closed components; an *input-anchored* `η` takes the leftmost admissible
slot without consulting it (its coordinate is already pinned by the component's
anchored layout), with Step 6 repairing any tied adjacency on the same pass —
and Step 6's comparator (`tie_sorts_before`) decides tied adjacencies. Because a
strictly-commuting swap moves an atom with source width 0 past one with target
width 0, it changes no other atom's wire coordinates — the component analysis is
invariant under Step 6, which is why the two passes agree and do not oscillate
(the sift moves atoms only up a layer, Step 6 only within a layer).

**Interleave guard (guard 3).** Rule (i)'s order only makes sense when the
components' attached coordinates on a boundary are disjoint intervals ordered by
least coordinate. When one component's attachment interleaves another's on the
same boundary, block transposition is not braid-free and the rule-(i) slot is
ill-defined. Such components are marked and left alone: their `η`s are not
sifted, and Step 6 falls back to the §2.5 class order for them. Canonicality
on the **fragment `𝔉` of §4.1** — every component clear (unmarked) *and*
boundary-attached — is **probe-verified but not proven**: the 2026-07-27
proof phase first added the closed-component exclusion (the trapped-nesting
residual, §4.6(c)) and then had its draft theorem refuted in review by a
residual *inside* `𝔉` (§4.6(d)); see §4.4 for the full status. The residuals
are strictly narrower than the pre-PR2 gap, which covered *every* mid-layer
`η`.

**The block pass (Step 7, `reorder_component_blocks`).** Rule (i) states an order
between whole components, and the two atom-level moves — the single-atom sift (up
one layer) and Step 6's within-layer reorder — cannot realize it: transposing two
*multi-atom* blocks is a coupled multi-layer move. Step 7 makes that move.

It transposes an **adjacent free pair** of components. A pair `{C1, C2}` is
*free* when (a) at least one is multi-atom — single ∥ single pairs stay with the
disjointness carve above — (b) neither is interleaved, and (c) at most one
occupies a wire on the input boundary and at most one on the output boundary.
Condition (c) is exactly what makes the transposition an equality rather than a
conjugation: reading the blocks as morphisms, `B1 ⊗ B2 = σ ; (B2 ⊗ B1) ; σ′` with
`σ = σ_{w1_in, w2_in}` and `σ′ = σ_{w2_out, w1_out}`, and both braids degenerate
to identities precisely when one of each pair of boundary widths is `0`. Read on
the abstract content instead: the swap moves no wire between components and,
under (c), fixes both boundary orderings, so it is the same anchored port graph.
By class the free pairs are therefore `closed ∥ anything` and
`input-only ∥ output-only`; a component touching *both* boundaries is pinned
against everything except a closed one.

A pair is *adjacent* when, in every layer holding both, each one's atoms form a
contiguous run and `C1`'s sits immediately left of `C2`'s — no third component
between them anywhere. In a layer holding only one there is nothing to swap: a
component's layer set is an interval, so the absent one contributes zero wire
width at the adjoining boundary and the present one's coordinates do not move.

**Fused identities.** `merge_adjacent_identities` fuses an `Identity` across
component boundaries, and the union-find then joins those components *through*
the fused atom. Step 7 therefore analyses a refinement in which every
`Identity(n)` is split into `n × Identity(1)` — free, since
`Identity(a+b) = Identity(a) ⊗ Identity(b)` — transposes there, and re-fuses on
the way out. The refinement is local to Step 7; §2.3's sift and §2.5's Step 6
keep reading the unrefined form. A component carrying a `Braid` is never
transposed: braid placement belongs to §2.1's pass, and keeping the two off each
other's atoms is what stops them oscillating.

**Closed↔closed order (resolved 2026-07-27, #79 P1).** All closed components
share one rule-(i) key; historically Step 7 — like Step 6 — never swapped
equal keys, so two *distinct* closed blocks kept their input order (the
block-level reading of §2.5's then-stable scalar order; residual (b) on issue
#174). The `Ord` bound `PropSignature` now carries closes it: at an equal-key
adjacency Step 7 compares the two blocks' **in-situ readings**
(layer-by-layer, left-to-right, each atom as kind tag / widths / generator by
`Ord` — offset-independent, hence invariant under the pass's own swaps) and
sorts ascending. Equal readings are identical blocks, for which the
transposition is invisible — so no finer-grain residual is reintroduced.
`component_slot`'s equal-key walk uses the same comparator (on its unrefined
analysis; the §4.1 coarsening caveat applies there as it does for keys).
Witness: `closed_blocks_sort_by_content_key` (formerly the `#[ignore]`d
`closed_closed_order_is_ord_less_residual`) plus the
`three_closed_blocks_converge_in_reading_key_order` probe family in
`tests/smc_nf_completeness.rs`.

**Downstream effect.** Closing the block-order gap moved scalar centrality —
`(η;ε) ⊗ μ = μ ⊗ (η;ε)` in the SCFM fragment — out of catgraph-syntax's
congruence-closure gap and into Layer 1: the closed `η;ε` block now sorts
leftmost on both sides, so `eq_mod` decides it without consulting `E_frob`. The
`complete_where_congruence_closure_is_not` test in
`catgraph-syntax/tests/cospan_complete_functor.rs` was re-pointed at a harder
witness accordingly.

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
| 7 | `reorder_component_blocks` | transpose adjacent *free* component blocks (`closed ∥ anything`, `input-only ∥ output-only`) into rule-(i) order, over an identity-split refinement (§2.6) |
| 6 | `reorder_tied_zero_arity` | within-layer bubble reorder of strictly-commuting zero-arity atoms — `scalar < η < ε < solid` at single-atom ties (§2.5), component order otherwise (§2.6) |

(Step 7 is staged *ahead* of Step 6 in the loop — a block move can land an `η`
beside an `ε`, and Step 6 repairs that on the same pass.)

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
| Free component blocks commute `B1⊗B2=B2⊗B1` (both connecting braids `σ_{0,n}=id` at block level) | JS-I Ch 1 §4 Thm 1.2 p. 71 (bifunctoriality) + Ch 2 §1 axiom (S) p. 73 (degenerate case) | Step 7 `reorder_component_blocks` (§2.6); `smc_canonicality_probes::block_transposition_*` |

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
  assignment* is **probe-verified** canonical on the fragment `𝔉` (§4.1:
  components clear and boundary-attached; full proof open — §4.4 status,
  residual §4.6(d)), via the component-anchored point-span sift (§2.3 + §2.6,
  issue #55 PR2; the former issue #14 follow-up gap). Tensor- and compose-forms of the same morphism
  converge (`ε ⊗ η` = `ε ; η`, `F ⊗ η ⊗ G` = `(F ⊗ G) ; (id₁ ⊗ η ⊗ id₁)`,
  `(μ;!) ; (η;Δ)` = `(μ;!) ⊗ (η;Δ)`); verified by `interchange_zero_source_eta`,
  the `zero_arity_order` tests, and the `smc_canonicality_probes` module in
  `tests/smc_nf_completeness.rs`.
- **Block order** — transposing two **multi-atom blocks** is covered by Step 7
  (§2.6) on the free pairs (`closed ∥ anything`, `input-only ∥ output-only`), so
  the tensor-form transpositions converge too: `(μ;!) ⊗ (η;Δ)` = `(η;Δ) ⊗ (μ;!)`
  = `(μ;!) ; (η;Δ)`, and `(η;!) ⊗ s` = `s ⊗ (η;!)`. Verified by
  `smc_canonicality_probes::block_transposition_converges` and
  `block_transposition_crosses_fused_identity_padding`.
- **Documented residuals**, the three open ones in §4.6 (details in
  §2.6/§4.5): (a) an `η` whose component's boundary attachment
  **interleaves** another's is not sifted, and its component is not
  transposed (guard 3); (c) a closed component **written strictly inside**
  another component's wire span does not extract (found in the §4 proof
  phase, 2026-07-27) — tracked as
  `trapped_closed_block_is_nesting_residual` (`#[ignore]`); (d) a nested
  **zero-arity block solid on its opening side** (solid-headed sink /
  solid-tailed source) — *inside* `𝔉`, the refutation of the draft §4
  theorem — tracked as `nested_sink_block_is_column_residual` /
  `nested_source_block_is_column_residual` (`#[ignore]`). All on issue #174.
  Residual **(b)** — Ord-less closed↔closed order — was **closed 2026-07-27
  by #79 P1** (§2.6's reading key; witness renamed
  `closed_blocks_sort_by_content_key`, un-ignored).

## §4 Abstract content and canonicality status

> Added 2026-07-27 (issue #55, the proof phase; owner call: proof-first with
> the honest fragment), and **rewritten the same day** after a two-round
> adversarial review of the first draft: drafting found the trapped-closed
> residual (§4.6(c)); review then **refuted the draft's full-canonicality
> theorem outright** with the probe-verified CE-A family (§4.6(d)) — the
> draft's enclosure argument had a non-exhaustive case split (§4.4). Owner
> call (second fork, same day): land the surviving core with a candid status
> section, no theorem. What stands: the content function and its equivalence
> to SMC-equality (Lemma 4.1, **color-generic** over an arbitrary color set
> `Λ` so the issue-#79 word-generalized engine inherits it), soundness of the
> pipeline (Lemma 4.2), the fragment/marking machinery (§4.1), and the
> DPO-substrate specification for the issue-#57 knowledge-base spike (§4.7).
> External anchors: Bonchi–Gadducci–Kissinger–Sobociński–Zanasi (**BGKSZ**,
> arXiv:1602.06771v2, *Rewriting modulo symmetric monoidal structure*),
> Milosavljević–Piedeleu–Zanasi (**MPZ**, CALCO 2023, *String Diagram
> Rewriting Modulo Commutative (Co)Monoid Structure*, VoR of
> arXiv:2204.04274), and Lafont (*Towards an algebraic theory of Boolean
> circuits*, JPAA 184 (2003) 257–310). All locators verified against the
> private papers cache 2026-07-27 (spec-review pass on PR #176).

### §4.1 Setting, content invariants, and the fragment 𝔉

Fix an arbitrary set of **colors** `Λ`. A signature assigns each generator
`g ∈ G` a source word `s(g) ∈ Λ*` and a target word `t(g) ∈ Λ*`; expressions,
identities `id_w` (`w ∈ Λ*`), braids `σ_{u,v}`, `;` and `⊗` are as in §1, with
arities in `Λ*` and word concatenation for `⊗`. The shipped `PropSignature` is
the monochromatic instance `Λ = {•}` (words collapse to their lengths); nothing
below ever uses `|Λ| = 1`, which is what makes the section #79-stable. Write
`e =_SMC e′` for equality in the free symmetric strict monoidal category
(equivalently the free colored prop) on the signature.

**Abstract content.** Following BGKSZ §3, interpret an arity-well-formed
expression `e : n → m` as a **cospan of Λ-typed directed hypergraphs**
`n → H ← m`: nodes are wires (typed by `Λ`), hyperedges are generator
occurrences with ordered, type-respecting source and target tentacles, and the
two anchoring maps embed the boundary words. (BGKSZ's own "Σ-typed" refers to
*hyperedge* labelling over the signature; the node-typing by `Λ` here is this
document's lift.) The interpretation — BGKSZ's `⟦·⟧`, the coproduct injection
of their §3, whose faithfulness is Prop 3.4 — sends a generator to the
single-hyperedge cospan, an
identity to a discrete bijective cospan, a braid to a discrete cospan with the
permuted anchor, `;` to pushout gluing over the shared foot, and `⊗` to
disjoint union with concatenated anchors. Define the **content** `C(e)` as this
cospan up to isomorphism *under both feet* (iso on the carrier `H` commuting
with the anchors, identity on `n` and `m`) — which is nothing exotic: it is
the standard hom-set equivalence of `FTerm_Σ` itself, where the feet are
objects of the prop and cannot move. The word "anchored" only names its
useful consequence: every boundary *coordinate* is a content invariant. By BGKSZ Thm 3.12 the cospans that arise are exactly the
**monogamous** (Def 3.6: anchors mono; interior nodes have in/out-degree
exactly 1, boundary nodes 0 on their anchored side) **directed acyclic**
(Def 3.9) ones.

**Derived invariants** (all functions of `C(e)`, since cospan iso is the
identity on the feet):

- **Components.** Connected components of the underlying hypergraph of
  `C(e)`. (At the diagram level this matches §2.6's union-find *over the
  identity-split refinement*: `Identity` and `Braid` atoms carry wires and
  belong to the component of their wires; the empty-interval guard implements
  "a zero-arity hyperedge is connected exactly through its non-empty side".
  One honesty note: only Step 7 pre-splits fused identities before analysing —
  the sift (Step 4(c)) and Step 6 read the *unrefined* analysis, a coarsening
  of the content components in which a fused `Identity` can join two content
  components. No witness is known where the coarsening alone changes an `nf`
  outcome; recorded as an open hygiene item on issue #174.)
- **Boundary attachment.** For a component `K`, the coordinate sets
  `in(K) ⊆ {0..|n|}` and `out(K) ⊆ {0..|m|}` of anchored nodes, and the
  rule-(i) key of §2.6 (class `closed < input-anchored < output-only`, least
  attached coordinate).
- **Owner words and clearness.** On each boundary read the **owner word**: the
  component owning each coordinate, left to right, with adjacent repeats
  collapsed to runs. A component is **marked** (guard 3,
  `mark_interleaved`) if it occurs in two distinct runs of either boundary's
  owner word, or lies between two occurrences of a component that does; it is
  **clear** otherwise. Note this marks both genuine alternation (`a b a b`)
  and nested attachment (`a b b a` — both `a` and `b` marked), matching the
  shipped `mark_interleaved` exactly.

**The fragment.** `𝔉` consists of the arity-well-formed expressions `e` such
that in `C(e)`:

1. every component is **clear** (no marking on either boundary), and
2. every component **touches a boundary** (no closed components).

Both conditions are content invariants, so membership in `𝔉` is itself a
property of the SMC class. Condition 2 is the 2026-07-27 correction to the
pre-proof claim ("the non-interleaved fragment"): closed components admit a
genuinely presentation-dependent pathology — §4.6, residual (c) — that no
content-level condition can carve around, because the nested and un-nested
writings of a closed block have *identical* content.

### §4.2 Content decides SMC-equality (color-generically)

**Lemma 4.1.** For arity-well-formed `e, e′ : n → m`:
`e =_SMC e′` **iff** `C(e) = C(e′)`.

*Proof.* (⇒) BGKSZ define `⟦·⟧` on the free prop `S_Σ`, whose arrows *are*
SMC-classes, so well-definedness on classes is part of their construction
(functoriality of the coproduct injection `⟦·⟧ : S_Σ → FTerm_Σ`, BGKSZ §3).
(⇐) is faithfulness, BGKSZ **Prop 3.4** (proved in their Appendix A from
properties of coproducts of PROPs). BGKSZ **Thm 3.12** identifies the image:
`n → H ← m` is in the image of `⟦·⟧` iff it is monogamous directed acyclic —
so `C` is a bijection between SMC-classes and anchored monogamous directed
acyclic cospans up to iso.

*Color-genericity.* BGKSZ state the section for a one-sorted signature (note
their own "Σ-typed" is hyperedge labelling, not node coloring). The `Λ`-typed
lift is verbatim: typed hypergraphs, their pushouts and coproducts are
computed sortwise; Lemma 3.11's convex-subgraph factorization and Thm 3.12's
induction on the number of hyperedges never inspect the node sort; Prop 3.4's
coproduct argument is sort-blind. (The multi-sorted case is developed at
length in the Bonchi et al. *String Diagram Rewrite Theory* journal series —
not in the papers cache, so cited here as a pointer, not an anchor; the
verbatim-lift argument above stands on its own.) No step of the arguments
below mentions monochromaticity either. ∎

By Lemma 4.1, proving "`nf` is a function of content" on some class of
diagrams *is* proving canonicality there: SMC-equal expressions have equal
content, hence equal NF. That reduction frames everything in §4.4 — including
what failed.

### §4.3 `nf` preserves content

**Lemma 4.2.** Every rewrite the §3 pipeline applies preserves `C`, and the
readback `from_string_diagram(nf(e))` (compose the layers, tensor each
layer's atoms) is SMC-equal to `e`.

*Proof.* Each step is an instance of an SMC axiom — the §3 paper coverage
matrix lists the axiom and anchor per step — and `C` is SMC-invariant by
Lemma 4.1(⇒). Lowering (`lower` / `pad_and_zip`) reads off the expression
tree and changes nothing up to associativity/unitor/interchange instances. ∎

Lemma 4.2 already gives the **unconditional direction** of canonicality, on
*every* diagram, fragment or not:
`nf(e) = nf(e′) ⇒ e =_SMC from_string_diagram(nf(e)) =_SMC e′`. Everything
open below concerns only the converse.

### §4.4 Canonicality status (2026-07-27)

The first draft of this section proved a rigidity theorem ("an
invariant-satisfying diagram is uniquely determined by its content on `𝔉`")
and derived full canonicality on `𝔉`. Adversarial review **refuted it** —
§4.6(d)'s CE-A family is a probe-verified SMC-equal pair *inside* `𝔉` whose
NFs differ — and the failure traces to a non-exhaustive case split in the
draft's central geometric lemma. This subsection records exactly what stands.

**Proven, unconditionally (any diagram).**

- *Soundness*: NF-equality implies SMC-equality (Lemma 4.2's readback), and
  `C(nf(e)) = C(e)`.
- *Termination*: the §2.4 lexicographic measure.

**Proven, within-layer (issue #55 PR1).** At single-atom tied adjacencies the
Step-6 order is canonical — `nf(ε ⊗ η) = nf(η ⊗ ε)`, stable under context
(§2.5). One verification caveat from review: the tied-run comparator mixes
component keys with the class order, and its merge-monotonicity across ≥ 3
components is verified only empirically — review attempted three realizing
configurations and all converged; none is known to be realizable in `𝔉`.

**Facts that survived adversarial review** (safe to build on):

- *Positional monotonicity* — braid-free layers never cross wires, so wire
  positions are consistent across the generator suffix.
- *Marking is content-level* — §4.1's owner-word marking is exactly
  `mark_interleaved`, including the nested pattern `a b b a` (both marked),
  and the owner word is determined by the anchors and components alone.
- *Key-distinctness* — distinct anchored components have distinct rule-(i)
  keys (each boundary coordinate has exactly one owner, so least coordinates
  differ).
- *The braid prefix is a function of its permutation* — Step 3(b)'s canonical
  bubble-sort word (§2.2) is deterministic in the underlying permutation.

**Verified by probes** (the canonicality gate of record,
`smc_canonicality_probes`): the §2.6 families — the two-component
counterexample with its full three-member family, the atomic `η ∥ ε`
writings, the mid-layer `η` interchange pair, closed-block placements and
transpositions including across fused identity padding — plus idempotence on
every witness.

**Refuted / open: full canonicality on `𝔉`.** The draft's *enclosure lemma*
claimed that foreign matter strictly inside a component's walled region is
closed or marked, with the two fragment conditions as the only escape
hatches. The case split missed a third: the wall itself can **open** at an
`η` (above) or `ε` (below) of the *enclosing* component, letting a nested,
boundary-attached block reach a boundary coordinate *outside* the encloser's
attachment interval — unmarked, not closed, yet still stuck (§4.6(d)).
Consequences of the same hole, for the next attempt: the draft's
forced-layer formula (positive-source at `1 + max` producer layer) fails
whenever a foreign zero-width point survives inside a consumed span — in
CE-A's fixpoint `Add` sits at layer 2 with its only producer at layer 0 —
and an `η`'s blocking gap-closer *can* be foreign inside `𝔉`. Review also
recorded three independent hygiene gaps so they are not re-trodden: the
draft's `ldepth`/gap-closer notions presupposed a layout (they were not
content-intrinsic as claimed); the input permutation π was read off a suffix
whose input order it was itself meant to determine (repair direction: run
the layer induction **bottom-up** from the output boundary, which "no braid
layer follows a generator layer" pins); and the §1 invariant list lacked
three clauses the draft used silently (now added to §1 and the
`StringDiagram` doc: intra-layer identity fusion, no pure-identity layer
beside a non-identity layer, canonical braid runs).

### §4.5 The missing move and the repair paths

The freedom `nf` fails to canonicalize is precisely `X ⊗ B = B ⊗ X` where
`B` is a **multi-atom zero-arity-bounded block** (`0 → 0`, `n → 0`, or
`0 → n` read as a block) and `X` is a **column** — a single atom or a bundle
of identity wires belonging to a *larger* component. Neither pass can make
the move: Step 6 compares only adjacent *atoms*, and a solid block-head
never strictly commutes with anything; Step 7 transposes only whole
*components*, and the column's component (which extends past the block on
both sides) is never free against the block under condition (c). A
single-atom `η` does escape via Step 6 when the neighbouring block is
η-headed — the convergences review confirmed — which is exactly why only
nestings **solid on the opening side** (solid-headed sink blocks,
solid-tailed source blocks) witness the residual.

Two repair paths, tracked on issue #174:

1. **Narrowed fragment** (content-level, proof-only): add to `𝔉` the
   condition "no input-attached component contains an `η`, and no
   output-attached component contains an `ε`" — walls never open, restoring
   the enclosure dichotomy. Sufficiency is **unproven**, and the condition
   excludes most useful SFG diagrams (anything mixing boundary attachment
   with internal `η`/`ε`), so this path trades nearly all of the theorem's
   value for its truth.
2. **Pipeline generalization** (the strong fix): extend Step 7's
   transposition from whole components to maximal zero-arity-bounded
   *columns*, subsuming both §4.6(c)'s extraction move and §4.6(d), and
   plausibly restoring rigidity on the original `𝔉` (the enclosure argument
   then needs no third case — an opened wall is exactly a transposable
   column boundary). Needs its own termination-measure component, probe
   extensions, and a pin re-measure: a PR of its own, not a docs change.

### §4.6 The residuals (three open; (b) closed 2026-07-27)

Every diagram still normalizes soundly and terminates; what is limited is
uniqueness. Residuals (a) and (c) sit *outside* `𝔉`; residual (d) sits
**inside** it, which is what refuted the draft theorem (§4.4):

- **(a) Marked (interleaved) components** — guard 3 leaves a marked
  component's `η`s unsifted and its blocks untransposed; rule (i)'s slot is
  ill-defined there because block transposition is not braid-free. Tracked on
  issue #174 (residual 2).
- **(b) Closed↔closed order — CLOSED (2026-07-27, #79 P1).** All closed
  components share the rule-(i) key `(closed, 0)`; distinct closed blocks
  formerly kept their presentation order for want of a content-derived
  tie-break. The `Ord` bound on `G` (#79 P1) supplies it: Step 7 sorts
  equal-key closed blocks by their in-situ readings (§2.6), and equal
  readings are identical blocks. Witness un-ignored and renamed
  `closed_blocks_sort_by_content_key`. Issue #174 (residual 1, closed).
- **(c) Trapped nested closed blocks** *(found in this proof phase,
  2026-07-27; the reason condition 2 is in `𝔉`).* A closed component written
  strictly inside another component's wire span cannot escape: its `η`'s
  coordinate falls strictly inside the enclosing atom's target span (sift
  blocked — correctly, since the closing atom is foreign and no
  own-component gap-closer exists), and Step 7 never sees an adjacent free pair
  because the surrounding identity wires belong to the *enclosing* component,
  so the closed block's run is never adjacent to a whole-component run.
  Probe-verified witness (`trapped_closed_block_is_nesting_residual`,
  `#[ignore]`):

  ```
  nf( Copy ; (id₁ ⊗ Zero ⊗ id₁) ; (id₁ ⊗ Discard ⊗ id₁) ; Add )
    ≠ nf( (Zero ; Discard) ⊗ (Copy ; Add) )
  ```

  though the two are SMC-equal (bifunctoriality with `id₀`; both sides have
  the same content — one closed loop, one through-component). The nested and
  free writings have identical content, so *no content-level fragment
  condition can include the free writing and exclude the nested one* — the
  residual is irreducibly presentation-level, which is why `𝔉` excludes
  closed components. A caution from review: the draft claimed "only closed
  components can be trapped" (a nested boundary-attached component being
  marked by guard 3) — **refuted**; enclosure of a component's *wires* does
  not imply enclosure of its *attachment* when the encloser's wall opens at
  its own `η`/`ε`, and that is residual (d). What does hold: the d = 2
  collision trackers cannot see this trap (it needs a producer above, `η`
  and `ε` inside, and a consumer below — expression depth ≥ 3), so the
  pinned baselines carry no contribution from it. Fix shape: the §4.5
  column-transposition generalization subsumes the earlier extraction-move
  sketch (`id₁ ⊗ s = s ⊗ id₁` sideways past identity wire-columns); tracked
  on #174.

  For diagrams whose closed components are all written *un-nested*, closed
  blocks sort leftmost as a class (rule (i)) and are placed canonically among
  themselves up to residual (b) — probe-verified by
  `closed_block_placement_converges`, `block_transposition_converges`, and
  `block_transposition_crosses_fused_identity_padding` (no proof claim,
  pending §4.5).

- **(d) Nested zero-arity blocks, solid on the opening side** *(found in adversarial
  review, 2026-07-27 — the refutation of the draft theorem; inside `𝔉`).*
  A multi-atom `n → 0` (or dually `0 → n`) block written at a coordinate
  strictly inside another component's span, with a **solid** atom on the side
  facing the enclosing wall's opening (head of a sink block, *tail* of a
  source block), converges with none of its free writings. Probe-verified witnesses
  (`nested_sink_block_is_column_residual`,
  `nested_source_block_is_column_residual`, both `#[ignore]`):

  ```
  nf( (Zero ⊗ s ⊗ id₁) ; (id₁ ⊗ Discard ⊗ id₁) ; Add )
    ≠ nf( (s ; Discard) ⊗ ((Zero ⊗ id₁) ; Add) )          (CE-A)

  nf( Copy ; (id₁ ⊗ Zero ⊗ id₁) ; (Discard ⊗ s ⊗ id₁) )
    ≠ nf( (Zero ; s) ⊗ (Copy ; (Discard ⊗ id₁)) )          (CE-A3)
  ```

  (`s` a solid `1 → 1` generator.) Both components are boundary-attached and
  unmarked, so the pair sits inside `𝔉` — same content, different fixpoints.
  Mechanism and fix: §4.5 (Step 6 cannot bubble past the solid head; Step 7's
  free-pair test is whole-component while the actual freedom is
  column-vs-block). An η-headed nested block *does* converge (Step 6 walks
  the single `η` out), which is why this shape needs the solid head. Same
  d ≥ 3 invisibility to the collision pins as (c). Tracked on #174.

### §4.7 The content function as the #57 DPO substrate

The content cospans of §4.1 are precisely the objects BGKSZ's **§5**
(*Rewriting modulo SMC*) rewrites: DPO rewriting of hypergraph cospans with
**convex** matchings (their Def 5.4 convex matching, Def 5.5 convex DPO step)
implements rewriting modulo SMC structure — the adequacy theorem is their
**Thm 5.6**, resting on Lemma 3.11's factorization together with Thm 3.12's
image characterization. (Their §4 / Thm 4.6 is the *larger* Frobenius
substrate — unrestricted DPO over all of `FTerm_Σ ≅ S_Σ + Frob` — the
comparison point for a Frobenius-aware layer, not the SMC fragment used
here.) MPZ extend the correspondence when the
signature carries a chosen commutative (co)monoid structure — **Def 7**
(right-monogamy) relaxes Def 3.6, **Thm 21** (`S_Σ + CMon ≅
RMACsp_D(Hyp_Σ)`) is the analogue of Thm 3.12, and **Thm 28** the rewriting
correspondence — which is the direction a Frobenius/`E_frob`-aware layer
would take. A #57 knowledge base would therefore: represent terms by their
content (this section's `C`), rewrite by convex DPO on content, and use `nf`
as the canonical *readback* from content to a layered term — and the
well-definedness of that readback is exactly the open §4.4 canonicality
question: probe-verified on `𝔉`, proven only in the NF-equal ⇒ SMC-equal
direction, pending the §4.5 column move for the rest. What #57
would add over the §2.4 pipeline is rewriting modulo *user equations* on the
same substrate; what it inherits from this section is that SMC-coherence
never needs rewriting at all — it is quotiented away by `C` itself.
