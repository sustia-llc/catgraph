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
the component-anchored point span); within every layer, no adjacent
strictly-commuting pair ordered against the Step 6 order — `scalar < η < ε` at a
single-atom tie, `closed < input-anchored < output-only` otherwise (§2.6); and no
adjacent *free* pair of connected components ordered against that same component
order (Step 7, §2.6).

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
with inverted rule-(i) keys. A Step-7 transposition flips every such pair between
the two runs (at least one, since the two are adjacent in some layer) and leaves
every other pair's relative order untouched. The comparator is stable across the
swap: boundary freedom means neither block occupies a wire on a boundary the
other attaches to, so the swap moves no boundary wire and `keys`, `sizes`,
`interleaved` and the attachment flags are all invariant under it.

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
claimed and proven on the **fragment `𝔉` of §4.1** — every component clear
(unmarked) *and* boundary-attached; the closed-component exclusion was added
2026-07-27 when the proof phase found the trapped-nesting residual, §4.6(c).
The residuals are strictly narrower than the pre-PR2 gap, which covered
*every* mid-layer `η`.

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

**Residual, second kind (equal keys).** All closed components share one rule-(i)
key, and Step 7 — like Step 6 — never swaps equal keys, so two *distinct* closed
blocks keep their input order. That is the block-level reading of §2.5's
stable-among-scalars and the same limitation for the same reason: sorting them
needs a content-derived total order on components, which bottoms out in an `Ord`
bound on `G` that `PropSignature` does not carry. Recorded as
`closed_closed_order_is_ord_less_residual` (`#[ignore]`) in
`tests/smc_nf_completeness.rs`, beside the now-converging
`block_transposition_converges`.

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
  assignment* is canonical on the **fragment `𝔉`** (§4.1: components clear and
  boundary-attached; proven in §4), via the component-anchored point-span sift
  (§2.3 + §2.6, issue #55 PR2; the former issue #14 follow-up gap). Tensor- and compose-forms of the same morphism
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
- **Documented residuals**, all three in §2.6/§4.6 and all narrower than the
  pre-PR2 gap: (a) an `η` whose component's boundary attachment **interleaves**
  another's is not sifted, and its component is not transposed (guard 3);
  (b) two distinct **closed** blocks share one rule-(i) key, so neither Step 6
  nor Step 7 swaps them — the same no-`Ord`-on-`G` limitation as scalar order,
  tracked as `closed_closed_order_is_ord_less_residual` (`#[ignore]`); (c) a
  closed component **written strictly inside** another component's wire span
  does not extract (found in the §4 proof phase, 2026-07-27) — tracked as
  `trapped_closed_block_is_nesting_residual` (`#[ignore]`), issue #174.

## §4 Canonicality on the anchored fragment (the proof)

> Added 2026-07-27 (issue #55, the proof phase; owner call: proof-first with
> the honest fragment). This section proves that `nf` computes a *function of
> the diagram's abstract content* on the fragment defined in §4.1 — canonicality
> outright, strictly stronger than confluence of the §3 rewrite pipeline. It is
> stated **color-generically** over an arbitrary color set `Λ` so that the
> issue-#79 word-generalized engine inherits it unchanged, and it doubles as
> the DPO-substrate specification for the issue-#57 knowledge-base spike
> (§4.7). External anchors: Bonchi–Gadducci–Kissinger–Sobociński–Zanasi
> (**BGKSZ**, arXiv:1602.06771v2, *Rewriting modulo symmetric monoidal
> structure*), Milosavljević–Piedeleu–Zanasi (**MPZ**, CALCO 2023, *String
> Diagram Rewriting Modulo Commutative (Co)Monoid Structure*, VoR of
> arXiv:2204.04274), and Lafont (*Towards an algebraic theory of Boolean
> circuits*, JPAA 184 (2003) 257–310). All locators verified against the
> private papers cache 2026-07-27.

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
two anchoring maps embed the boundary words. The interpretation — BGKSZ's
`⟦·⟧`, their Prop 3.4 — sends a generator to the single-hyperedge cospan, an
identity to a discrete bijective cospan, a braid to a discrete cospan with the
permuted anchor, `;` to pushout gluing over the shared foot, and `⊗` to
disjoint union with concatenated anchors. Define the **content** `C(e)` as this
cospan up to isomorphism *under both feet* (iso on the carrier `H` commuting
with the anchors, identity on `n` and `m`). "Anchored" is load-bearing:
because the feet are held pointwise fixed, every boundary *coordinate* is a
content invariant. By BGKSZ Thm 3.12 the cospans that arise are exactly the
**monogamous** (Def 3.6: anchors mono; interior nodes have in/out-degree
exactly 1, boundary nodes 0 on their anchored side) **directed acyclic**
(Def 3.9) ones.

**Derived invariants** (all functions of `C(e)`, since cospan iso is the
identity on the feet):

- **Components.** Connected components of the underlying hypergraph of
  `C(e)`. (At the diagram level this matches §2.6's union-find: `Identity` and
  `Braid` atoms carry wires and belong to the component of their wires; the
  empty-interval guard implements "a zero-arity hyperedge is connected exactly
  through its non-empty side".)
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

*Color-genericity.* BGKSZ state the section for a one-sorted signature. The
`Λ`-typed lift is verbatim: typed hypergraphs, their pushouts and coproducts
are computed sortwise; Lemma 3.11's convex-subgraph factorization and
Thm 3.12's induction on the number of hyperedges never inspect the node sort;
Prop 3.4's coproduct argument is sort-blind. No step of the proofs below
mentions monochromaticity either. ∎

By Lemma 4.1, proving "`nf` is a function of content on `𝔉`" *is* proving
canonicality on `𝔉`: SMC-equal expressions have equal content, hence equal NF.

### §4.3 `nf` preserves content

**Lemma 4.2.** Every rewrite the §3 pipeline applies preserves `C`, and the
readback of `nf(e)` (compose its layers, tensor each layer's atoms) is
SMC-equal to `e`.

*Proof.* Each step is an instance of an SMC axiom — the §3 paper coverage
matrix lists the axiom and anchor per step — and `C` is SMC-invariant by
Lemma 4.1(⇒). Lowering (`lower` / `pad_and_zip`) reads off the expression
tree and changes nothing up to associativity/unitor/interchange instances. ∎

### §4.4 Rigidity: the invariants pin the diagram

The §1 invariant list (the post-`nf` clauses on `StringDiagram`) is what the
fixpoint loop guarantees. Rigidity says the list leaves no freedom on `𝔉`:

**Theorem 4.3 (rigidity).** Let `D`, `D′` be layered diagrams satisfying the
§1 post-`nf` invariants, with `C(D) = C(D′) ∈ 𝔉`. Then `D = D′`.

Throughout, split `D = B ; S` into the **braid prefix** `B` (the leading
braid-bearing layers) and the **suffix** `S` (braid-free): the invariants "all
braids in leading layers" and "no mixed layers" force this shape.

**(4.4.1) Positional monotonicity.** In a braid-free layer, each atom occupies
a contiguous source interval and a contiguous target interval, and the atom
order induces the same linear order on both boundaries. Consequently wires
never cross in `S`: if two wires `w < w′` coexist at two boundaries, their
order agrees. (Immediate from §2.1's source-order convention; this is the
planarity that lets us speak of a wire's *position* consistently.)

**(4.4.2) Enclosure.** Say foreign matter `x` (a wire, or a zero-width
atom-point, of component `K′ ≠ K`) is **enclosed** by `K` at a boundary of `S`
when `K`-wires `w_l < x < w_r` flank it there. *Claim: in `𝔉`, enclosed
foreign matter does not exist.* Trace the flanking wires: upward, a flank
either persists (an identity wire) or terminates at a `K`-atom, whose own
source wires (ordered around the region by monotonicity) continue the wall —
except when that atom has no sources on the relevant side (an `η` of `K`),
where the wall opens; dually downward (an `ε` of `K` opens the wall below).
By monotonicity `x`'s component can never cross a wall wire, and it cannot
pass through a `K`-atom (sharing a wire with one would mean `K′ = K`). Two
exhaustive outcomes:

- `K′` reaches an outer boundary through an opening whose *both* sides reach
  that boundary flanked by `K`-wires — then the owner word reads
  `… K … K′ … K …`, `K` occurs in two runs, and both components are marked:
  excluded from `𝔉` by clearness. (This includes the nested-attachment
  pattern `a b b a`.)
- `K′` never reaches an outer boundary — it is walled in above and below
  (positions strictly inside an atom's span at the boundary adjacent to that
  atom are that atom's own wires, by monogamy, so entry/exit through a wall
  atom's span is impossible for foreign wires): `K′` is **closed**, excluded
  from `𝔉` by condition 2. ∎

Enclosure is the load-bearing geometric fact, and both fragment conditions
are exactly its two escape hatches — which is why `𝔉` is the natural
statement and why dropping condition 2 is not a wording matter (§4.6(c)).

**(4.4.3) Layers are forced.** Define, on content, the **longest-path depth**
of a hyperedge (`ldepth(h) = 0` if every source node of `h` is
input-boundary-anchored or `h` has no sources and is unblocked all the way;
else `1 + max` over producers/gap-closers as below). Precisely, in any
invariant diagram with content in `𝔉`:

- *Positive-source atoms.* An atom `g` with `s(g) ≠ ε` sits at suffix layer
  `1 + max{layer(p) : p a producer of g}` (layer `0` when all sources are
  boundary wires). "≥" is well-formedness (a producer sits strictly above its
  consumer). "≤": suppose every producer of `g` sits at layer `≤ j−2`. Then
  every wire of `g`'s source span passes layer `j−1` inside `Identity` atoms;
  by enclosure no zero-width foreign point sits strictly inside the span, and
  no positive-width atom does (it would own span wires, i.e. be a producer at
  `j−1`); with adjacent identities fused (§1 invariant) one `Identity` covers
  the span — the sift invariant ("no positive-source generator's consumed
  wires all pass through `Identity` atoms in the preceding braid-free layer")
  is violated. So some producer sits at `j−1`. By induction on layers, forced.
- *Zero-source atoms (`η`).* At fixpoint an `η` at layer `j ≥ 1` is blocked at
  `j−1`, which by the §2.3 point-span rule means its output coordinate falls
  strictly inside some atom `p`'s target span there. By enclosure `p` belongs
  to the `η`'s **own component** (a foreign `p` would enclose the `η`'s
  wires). So `layer(η) = layer(p) + 1` for its content-determined
  *gap-closer* `p` — the deepest own-component atom whose target span strictly
  contains the `η`'s wire gap — or `0` when no atom closes the gap. Forced.

**(4.4.4) Within-layer order is forced.** Positions in a layer are the atom
order (widths sum). Atoms consuming existing wires sit at their consumed
span's position — forced by monotonicity from the layers above. What remains
are zero-source coordinates and tied runs:

- An `η` whose component is input-anchored inherits its coordinate from its
  consumers' wiring relative to already-positioned wires (its output tentacle
  order at the consumer fixes its gap) — content.
- An `η`-headed **output-only** block carries no wiring constraint against
  the input-anchored part ("free-floating"); its slot is rule (i)'s: ordered
  by the component's least attached *output* coordinate — content
  (`component_slot`).
- Within a tied run (§2.5 strict commutation), the comparator is total on
  `𝔉`: distinct anchored components have distinct keys (each boundary
  coordinate belongs to exactly one component, so least coordinates differ);
  a single-atom tie inside one component or across equal-key blocks falls to
  the Decision-1 class order `η < ε` (two `η`s or two `ε`s never strictly
  commute); and `𝔉` has no closed components, hence no `0→0` scalars and no
  equal-key closed pair — the §2.5/§2.6 stability caveats are vacuous here.
- Identity padding is forced: the wires passing through a layer are
  determined, and maximal fusion (§1) fixes their grouping.

**(4.4.5) The prefix is forced.** By 4.4.3–4.4.4 the suffix `S` is determined
by content up to its input-boundary order; reading each top wire's origin
against the anchored input coordinates of `C` gives one permutation `π` —
content-determined. The prefix `B` realizes `π` as the canonical reduced word
in layered `Braid(1,1)` bricks (§2.2, Step 3(b) invariants: bubble-sort word
of the underlying permutation, JS-I Ch 2 Thm 2.3 p. 81) — unique. `D = B ; S`
is therefore determined by `C(D)`; likewise `D′`, so `D = D′`. ∎

### §4.5 The canonicality theorem

**Theorem 4.4 (canonicality on `𝔉`).** For `e, e′ ∈ 𝔉`:

```
nf(e) = nf(e′)   ⟺   e =_SMC e′
```

*Proof.* (⇐) `nf` terminates (§2.4) and its output satisfies the §1
invariants; by Lemma 4.2 `C(nf(e)) = C(e)`, and `C(e) = C(e′)` by Lemma 4.1.
Theorem 4.3 applies (membership in `𝔉` is a content property): `nf(e)` and
`nf(e′)` are invariant diagrams with the same content in `𝔉`, hence equal.
(⇒) Lemma 4.2's readback: `e =_SMC nf(e) = nf(e′) =_SMC e′`. ∎

Equivalently: on `𝔉`, `nf(e)` *is* the unique invariant-satisfying diagram
with content `C(e)` — a normal form defined by its universal description, not
by the pipeline that happens to compute it.

**Canonicality vs. confluence.** The classical route to uniqueness —
termination plus local confluence (Newman; cf. Lafont, Appendix A
*Rewriting*, §A.2 *Termination* p. 299, and the Lemma 15 equivalences,
for exactly this method applied to circuit presentations) — would require
joining every critical pair of the nine-step pipeline. Theorem 4.4 is
stronger and independent of the rewrite strategy: *any* reduction to an
invariant-satisfying diagram lands on the same object. The §2.4 lexicographic
measure plays the termination role of Lafont's §A.2; rigidity replaces the
confluence half.

### §4.6 Beyond 𝔉: closed components, three residuals

Diagrams outside `𝔉` still normalize soundly and terminate; what weakens is
uniqueness. The three residuals, in decreasing severity of the freedom left:

- **(a) Marked (interleaved) components** — guard 3 leaves a marked
  component's `η`s unsifted and its blocks untransposed; rule (i)'s slot is
  ill-defined there because block transposition is not braid-free. The §4.4
  induction stops at the first marked component. Tracked on issue #174
  (residual 2).
- **(b) Closed↔closed order** — all closed components share the rule-(i) key
  `(closed, 0)`; distinct closed blocks keep their presentation order (no
  content-derived tie-break without an `Ord` on `G`). Witness:
  `closed_closed_order_is_ord_less_residual` (`#[ignore]`). Retired for free
  by #79's stable-generator-key design input. Issue #174 (residual 1).
- **(c) Trapped nested closed blocks** *(found in this proof phase,
  2026-07-27; the reason condition 2 is in `𝔉`).* A closed component written
  strictly inside another component's wire span cannot escape: its `η`'s
  coordinate falls strictly inside the enclosing atom's target span (sift
  blocked — correctly, §4.4.3's gap-closer analysis does not apply since the
  closing atom is foreign), and Step 7 never sees an adjacent free pair
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
  residual is irreducibly presentation-level, and the honest theorem excludes
  closed components from `𝔉` altogether. Two mitigating structure facts:
  **only closed components can be trapped** — a nested component that touches
  a boundary has its attachment enclosed by the surrounding component's, so
  both are marked by guard 3 and already sit in residual (a); and the d = 2
  collision trackers cannot see the trap (a nested closed block needs a
  producer above, `η` and `ε` inside, and a consumer below — expression depth
  ≥ 3), so the pinned baselines carry no contribution from it. Fix shape (a
  candidate PR3, tracked on #174): an *extraction move* sliding a closed
  block sideways past identity wire-columns (`id₁ ⊗ s = s ⊗ id₁`, the
  degenerate symmetry), after which the existing sift and Step 7 finish; it
  needs its own measure component and a re-measure of the pins.

  For diagrams whose closed components are all written *un-nested* (every
  closed block's atoms adjacent only to identity wires of no component or to
  whole-block runs), the §4.4 argument extends: closed blocks sort leftmost
  as a class (rule (i)), are placed canonically among themselves up to
  residual (b), and Theorem 4.4 holds relative to that closed-block order.
  The `smc_canonicality_probes` exercise exactly this extension
  (`closed_block_placement_converges`, `block_transposition_converges`,
  `block_transposition_crosses_fused_identity_padding`).

### §4.7 The content function as the #57 DPO substrate

The content cospans of §4.1 are precisely the objects BGKSZ's §4 rewrites:
DPO rewriting of (Λ-typed) hypergraphs with **convex** matchings implements
rewriting modulo SMC structure (their Thm on convexity via Lemma 3.11 and
Thm 3.12's factorization), and MPZ extend the correspondence when the
signature carries a chosen commutative (co)monoid structure — **Def 7**
(right-monogamy) relaxes Def 3.6, **Thm 21** (`S_Σ + CMon ≅
RMACsp_D(Hyp_Σ)`) is the analogue of Thm 3.12, and **Thm 28** the rewriting
correspondence — which is the direction a Frobenius/`E_frob`-aware layer
would take. A #57 knowledge base would therefore: represent terms by their
content (this section's `C`), rewrite by convex DPO on content, and use `nf`
as the canonical *readback* from content to a layered term — Theorem 4.4 is
exactly the statement that the readback is well defined on `𝔉`. What #57
would add over the §2.4 pipeline is rewriting modulo *user equations* on the
same substrate; what it inherits from this section is that SMC-coherence
never needs rewriting at all — it is quotiented away by `C` itself.
