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
> **Recovery note (2026-07-29):** the original working note is *not* lost after
> all — it survived in the author's private notes archive, outside this
> repository, and was recovered while the #57 a1 arc was in flight. A fidelity
> diff against the reconstruction above found no correction to make: the
> §2.1–§2.4/§3 numbering the citations depend on matches, every convention
> decision recorded in the note matches the one documented here, the 18-test
> inventory shipped verbatim, and the note's §2.4 three-tuple termination
> measure is an ordered subsequence of the measure this document now carries
> (the later components were appended by the passes that needed them, none
> reordered). The diff also settled the provenance of one known error: the
> Selinger "Thm 3.12 p. 17" slip originated in the working note itself, so the
> audit's correction to p. 18 below stands and was not introduced by the
> reconstruction. The paragraph above is left as written — it is the honest
> record of what was known when this document was built.
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
SMC-equal expressions reach the same `StringDiagram` — has a proven core and
a conditional bridge: **rigidity** is proven on the fragment `𝔉′` (braid-free,
every `η` placement-pinned — Theorem 4.5, §4.4, at proof-sketch density with
two flagged-open steps, a failed discharge attempt recorded), and
**canonicality via `nf`** on `𝔉′`
is *conditional* on the fixpoints being braid-free and invariant-satisfying —
a condition with a committed counterexample witness, *to be* discharged by
the filed `adjacent_column_cuts` fix (§4.4's conditional corollary). Beyond `𝔉′` it is
open and not bounded either: the `smc_canonicality_probes` suite verifies a
named set of convergences, while a differential sweep still finds divergent
SMC-equal pairs inside the larger fragment `𝔉` — on the default corpus all
of them `η` placement slack, the freedom `𝔉′` excludes (§4.6's ledger,
§4.4's status).

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
the point span at its leftmost admissible slot — admissibility is evaluated at
the `η`'s *written* coordinate, so this clause pins the slot only relative to
that coordinate; the coordinate itself is the residual freedom `ι`, §4.4);
within every layer, no adjacent
strictly-commuting pair ordered against the Step 6 order — `scalar < η < ε` at a
single-atom tie (§2.5 — since the #174 design round the tied comparator reads
nothing else); no adjacent *free* pair of connected components ordered against
rule (i)'s component order (Step 7, §2.6); and no adjacent pair of
interval-aligned *columns* whose block arities strictly commute **and whose
component keys differ** ordered against that same component order (Step 6½,
§4.5 — an equal-key pair is declined, not decided). Both transposition passes carry the
same three guards — at least one component multi-atom, neither marked, neither
braid-carrying — so an ordering violation involving a marked or braid-carrying
component is not an invariant breach. **Known deviation (2026-07-28, filed):**
the shipped Step 6½ seed test (`adjacent_column_cuts`) demands the *right*
column's whole layer presence be one contiguous run where this clause requires
only local runs, so a fixpoint can violate the column clause where a
split-presence encloser keeps the pass from seeding — witness
`cut_asymmetry_separates_smc_equal_writings_inside_f_prime`; §4.4's
conditional corollary is conditioned on exactly this. **The two transposition clauses except
*both-readings adjacencies*** (restated 2026-07-28): where the contested run
boundary is an adjacency of strictly-commuting atoms from the two components —
so Step 6 also claims the pair — the §2.5 class order wins and the component-
order clauses do not apply. Step 6 runs last in the fixpoint loop and is
idempotent, so every fixpoint is Step-6-sorted; without the exception the
clause set is jointly unsatisfiable on some contents and was violated at real
`nf` fixpoints — machine-verified witnesses in
`tests/pass_disjointness_probes.rs`, disposition in §4.4 ("pass
disjointness"). (The three bolded clauses were implicit
until the §4 review, which found the draft rigidity argument silently using them
— 2026-07-27.)

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

   **Zero-source** generators (`η : 0 → 1`) sift by the **point-span rule**
   (issue #55 PR2; coordinate-only since #174): the empty consumed span reduces to a
   single output coordinate `q` at the source cursor, so `η` slides into the
   earliest braid-free layer `j−1` iff `q` is an atom boundary there (insert
   between the adjacent atoms, e.g. the `[F, G]` boundary in the witness) or
   strictly inside one of its identities (split that identity). It is blocked
   when `q` is strictly inside a generator's output span (whose wires cannot be
   split).

   Where `q` admits **several** slots — a run of target-0 atoms all sits at that
   coordinate — every one of them denotes the same morphism, and the **leftmost**
   is taken. The component-anchored walk that used to choose among them was
   **retired in the #174 design round** (§2.6): which slot an `η` takes inside
   that run is a genuinely *free* choice, so importing rule (i)'s
   writing-dependent coordinates into it made the normal form writing-dependent —
   witnessed by CE-R1, an SMC-equal pair inside `𝔉` that the imported order
   separated. What the 2026-07-26 diagnosis note got right stands: the *cursor*
   is not the anchor either. The coordinate `q` is; the slot among equals is
   free, and free is resolved positionally, not by content the diagram does not
   pin. Target-0 sinks (`ε : 1 → 0`) have a non-empty source span and sift via
   the positive-source path. Only **braids** are excluded from the sift entirely
   (their placement is §2.1's job, and letting both passes move atoms would
   oscillate the fixpoint).

   Choice 1 leaves one residual freedom that wire order does not decide — the
   relative order of *strictly commuting* zero-arity atoms within a layer. §2.5
   fixes it at the atom level, §2.6 at the component level.

- Anchors: JS-I Ch 1 Prop 1.1 p. 66 (rectangle-cover independence); JS-I
  Ch 1 §4 Thm 1.2 p. 71 (𝔽(𝒟) freeness — interchange is proof item (f) +
  Fig 1.9; see the header note on the heading misprint); issue #14.

### §2.4 Termination measure

`nf` runs the ten steps of §3 in a fixpoint loop, exiting when a full pass
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
 column_inversion_count,
 tied_inversion_count)
```

with each step non-increasing on the whole tuple and at least one step strictly
decreasing whenever the diagram is not yet a fixpoint — **with one witnessed
exception** (2026-07-28): at a *both-readings adjacency* (§4.4) Step 6 moves a
block/column-counted pair against those counts, the affected rewriting pass
moves it back on the same iteration, and the loop exits through the whole-pass
`sd == prev` check on exact cancellation. The table below is the proof
everywhere else; the exception's status and the repair options are §4.4's
termination item:

| Component | Step that strictly decreases it | Note |
|---|---|---|
| `crossings` | `reduce_involution` (`σ;σ → id`) | `hexagon_expand` leaves it fixed (preserves the underlying permutation) |
| `mixed_layer_count` | `isolate_mixed_braid_layers` (inside `collect_braid_prefix`) | the mixed-merge refusal at `reduce_involution`'s merge site stops any step re-creating a mixed layer |
| `wide_braid_count` | `hexagon_expand` (`Braid(m,n), m+n>2 → Braid(1,1)` bricks) | ordered ahead of `braid_position_sum` so a naturality-emitted wide braid is decomposed before positions are compared |
| `braid_position_sum` | the naturality sweep (braids move input-ward) | §2.1 |
| `generator_position_sum` | `topological_layer_order` (Step 4(c)) | one generator — positive-source or zero-source `η` (issue #55) — drops exactly one layer per sift; bounded below by 0 |
| `layer_count` | `coalesce_identity_layers` / `simplify_units` | identity-only layers absorb; `Identity(0)` atoms are removed |
| `block_inversion_count` | `reorder_component_blocks` (Step 7, §2.6) | one transposition flips every position pair between the two blocks' runs; the pass changes no layer's membership and rewrites no atom, so every earlier component is invariant under it |
| `column_inversion_count` | `reorder_zero_arity_columns` (Step 6½, §4.5) | the interval-level analogue of the block count; one transposition flips every position pair between the two columns' runs, and the pass changes no layer's membership and rewrites no atom |
| `tied_inversion_count` | `reorder_tied_zero_arity` (Step 6, §2.5 + §2.6) | trailing component; `try_unitor_merge`, the zero-source sift, Step 7 and Step 6½ may *raise* it while strictly shrinking an earlier component (`layer_count` / `generator_position_sum` / `block_inversion_count` / `column_inversion_count`) — see below |

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

**`column_inversion_count`, precisely.** Summed over layers, the number of
*pairs* `p < q` within a layer whose two components pass Step 6½'s guards — at
least one multi-atom, neither marked, neither braid-carrying — have **distinct**
keys, and are inverted against the shared `component_key_order`. A Step-6½
transposition flips every such pair between the two columns' runs (at least one,
since the runs are adjacent in every layer of the interval) and leaves every
other pair's relative order untouched. The comparator is stable across the swap
for the same reason Step 7's is, arrived at differently: block-level strict
commutation makes one side of the pair zero-width at *each* of the interval's
own boundaries, so where the interval abuts a diagram boundary the swap moves no
wire there at all, and `keys`, `sizes`, `interleaved` and the attachment flags
are invariant. Since the count excludes equal-key (closed↔closed) pairs, no
reading key enters it.

Step 6½ leaves every earlier component fixed by the same argument as Step 7 — it
moves no atom across a layer boundary, rewrites no atom, and changes no layer's
membership. `block_inversion_count` in particular: a column swap changes the
relative order of exactly the two runs' components, which the block count either
excludes (not a free pair) or orders by the same `component_key_order` core, so
the block count drops or holds. Never rises.

Step 6 leaves every earlier component fixed: it never moves an atom across a
layer boundary, never rewrites an atom, and never changes a layer's membership,
so `crossings`, `mixed_layer_count`, `wide_braid_count`, the two position sums
(both layer-index sums) and `layer_count` are all invariant under it.
`block_inversion_count` and `column_inversion_count` are **not** invariant under
it in general — the question this paragraph used to leave open is settled, in
the negative (2026-07-28). A Step-6 swap changes the relative order of exactly
the pair it swaps, and that pair is either excluded from those counts outright —
two single-atom components, a marked or braid-carrying one, or an equal-key
(closed↔closed) column pair — or it is a pair the transposition passes do count.
In the second case Step 6 orders it by the **class** order while the counts
order it by rule (i), and at a both-readings adjacency (§4.4) the two orders
oppose: Step 6 *does* raise the count, machine-verified
(`tests/pass_disjointness_probes.rs`, three shapes, two with shipped
generators). What holds termination together there is not this measure but the
loop's whole-pass exit: the rewriting pass's move and Step 6's counter-move
cancel exactly, `sd == prev`, and the loop stops on the Step-6-sorted layout —
which the restated §1 clauses now ratify as the canonical one. No
non-terminating or diverging instance is known; completing the proof (or
restoring the measure by gating the rewriting passes off both-readings pairs)
is tracked on issue #174.

Four steps can raise `tied_inversion_count`, each while strictly shrinking an
*earlier* component, so the tuple still drops lexicographically and Step 6
repairs the ordering on the same fixpoint pass:

- `try_unitor_merge` — its case 1 prepends an ε ahead of the absorbed layer's
  atoms (possibly past an η) and its case 4 appends an η after them (possibly
  behind an ε); case 3's η-prepend is order-canonical. It shrinks `layer_count`.
  It cannot raise `column_inversion_count` either: it changes no component's
  key, size or marking, and the atoms it moves keep their components, so the
  only pairs whose order it changes are ones the count either excludes or was
  already counting in the same direction.
- the zero-source point-span sift, which can land an `η` beside an `ε` in the
  earlier layer. It shrinks `generator_position_sum`. It moves an atom between
  layers, so it *can* raise `column_inversion_count` — a later component, and
  Step 6½ repairs it on the same pass.
- a Step-7 block move, which shrinks `block_inversion_count`. It cannot raise
  `column_inversion_count`: it changes the relative order only of the two
  components it swaps, and both counts order those by the same
  `component_key_order` core, so a Step-7 swap lowers the column count or leaves
  it alone.
- a Step-6½ column move, which shrinks `column_inversion_count` itself.

Steps 7 and 6½ are staged *ahead* of Step 6 in the loop for exactly this
reason.

`tied_inversion_count` is read against the *Step 6 order* of §2.5 — the class
order `scalar < η < ε < solid`, then `G::cmp` at an equal class. Since the
design round retired the comparator's rule-(i) branch (§2.6) that is the whole
order: it reads the two atoms and nothing else, so it cannot change under any
pass, and the count is well-defined without the invariance argument the old
mixed comparator needed. Each swap fixes exactly the pair it swaps and can never
re-invert it, so the count drops by one per swap.

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
input-anchored. All closed components share one rule-(i) key; among themselves
Step 7 sorts them by the in-situ reading key (#79 P1 — see "Closed↔closed order
(resolved)" below), and Step 6½ declines the tie rather than deciding it. Step 6
plays no part: since the #174 retirement its comparator never consults component
class at all, so an atomic `0 → 0` scalar is ordered there by `G::cmp` as an
*atom*, not as the closed component it also happens to be.

**Where rule (i)'s coordinates may be read** (design round, 2026-07-28).
`CompKey` carries a boundary *coordinate*, and a coordinate is a function of
the morphism only where the geometry pins it. That distinction is now the
organizing rule for the whole engine:

- The two **rewriting** passes — Step 7 and Step 6½ — *verify* the pinning
  before moving anything. Each checks that its transposition is braid-free at
  both boundaries it spans (Step 7's condition (c); Step 6½'s block-level strict
  commutation), and braid-free layers never cross wires, so for the pairs those
  passes actually move, the coordinates really are geometry. They read
  `component_key_order`, and at its one tie — two closed components — Step 7
  reads the in-situ reading key on top.
- The **free** decision sites have no such guarantee, and no longer consult
  either order. The Step 4(c) `η` slot walk (`component_slot`) and the rule-(i)
  branch of Step 6's tied comparator were both **retired** in the design round.
  The `η` sift now takes the leftmost slot its coordinate admits; a tied
  adjacency is decided by §2.5's class order and `G::cmp` alone. Both sites are
  coordinate-free and component-free.

The retirement was forced, not tidy-minded. A per-pair clause reading output
coordinates was tried first (it is what CE-A3 appeared to need) and was refuted
twice: it broke **CE-R1**, an SMC-equal pair *inside* 𝔉 that converges without
it, at exactly the slot walk; and it made the comparator **intransitive** on
about 37% of sampled diagrams, which termination survives but canonicality and
the §4.4 order argument do not. The measurement that resolved it: CE-A3
converges with no clause anywhere once the coordinate keys leave the free sites
— its blocker was the slot walk and the tied comparator, not the comparator's
content. Both riders were dropped; CE-R1 and CE-R2 are committed as regression
witnesses.

**What is *and is not* claimed (settled 2026-07-28).** Steps 6½ and 7 share a
comparator, so those two cannot disagree with each other — that pairwise
statement survives every probe. The broader hope — that the class-order family
(Step 6) and the component-order family (Steps 6½/7) never prescribe
conflicting layouts — is **refuted**: both-readings adjacencies exist, with
shipped generators and inside `𝔉`, at which the two orders oppose
(`tests/pass_disjointness_probes.rs`). The resolution is a ratified carve, not
a lemma: the class order wins at such adjacencies (Step 6 runs last and every
fixpoint is Step-6-sorted), the §1 transposition clauses except them, and the
full disposition — including what the carve does *not* establish — is §4.4's
"pass disjointness" entry.

**The disjointness carve (retired 2026-07-28).** An atomic `η ∥ ε` pair is both
a §2.5 tied adjacency (η first) and a rule-(i) component transposition
(input-anchored ε first). The carve split them by component size: single-atom
pairs went to Decision 1, anything touching a multi-atom component went to rule
(i)'s component order. It exists only if both orders are live at the same site,
and after the design round they are not — the tied comparator no longer reads
component keys at all, so a tied adjacency is *always* §2.5's class order and
`G::cmp`. Nothing is carved, and the merge-monotonicity caveat the carve carried
(§4.4) retires with it. Component order survives only in Steps 6½ and 7, whose
own guards (at least one multi-atom, neither marked, neither braid-carrying)
decide what they may move.

Strict commutation still makes every Step-6 swap connectivity-preserving — one
side has source width 0 and one target width 0, so no other atom's wire
coordinates move — which is what lets the sift and Step 6 run in the same
fixpoint without oscillating (the sift moves atoms only up a layer, Step 6 only
within a layer).

**Interleave guard (guard 3).** Rule (i)'s order only makes sense when the
components' attached coordinates on a boundary are disjoint intervals ordered by
least coordinate. When one component's attachment interleaves another's on the
same boundary, block transposition is not braid-free and the rule-(i) slot is
ill-defined. Such components are marked, and **Steps 7 and 6½** transpose
neither their blocks nor their columns.

Its scope narrowed in the design round. It used to gate the `η` sift and Step
6's comparator too; both of those are now coordinate-free and component-free, so
a marked component's `η` sifts like any other and its tied adjacencies order by
class. Only the two rewriting passes still consult the marking. Measured effect
on the differential corpus: divergences on marked cases fell 888 → 23 (§4.6).
What remains blocked — a marked component's block and column transpositions — is
residual (a), witnessed by `marked_encloser_blocks_the_column_move`, whose guard
is verified decisive by ablation.

Canonicality on the **fragment `𝔉` of §4.1** — every component clear (unmarked)
*and* boundary-attached — is **probe-verified and refuted as a theorem**: the
2026-07-27 proof phase first added the closed-component exclusion (the
trapped-nesting residual, §4.6(c)), its draft theorem was refuted in review by
a residual *inside* `𝔉` (§4.6(d)), both were closed behaviourally in #174 —
(c) by the §4.5 column pass, (d)'s source form by the free-site retirement —
and the 2026-07-28 investigation then refuted rigidity on `𝔉` outright with a
three-generator `η`-slack witness that no pipeline pass addresses. What is
**proven** is Theorem 4.5 on the smaller `𝔉′` (braid-free, every `η`
placement-pinned); the divergences §4.6 records inside `𝔉` are exactly the
slack `𝔉′` excludes. See §4.4.

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
the way out. Step 6½ (§4.5) rewrites on the *same* refinement and the *same*
analysis: the two passes run back-to-back and neither rewrites an atom or moves
one between layers, so one explode/analyse/fuse round trip serves both. §2.3's
sift and §2.5's Step 6 read no component analysis at all. A component carrying a
`Braid` is never
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
Step 6½ reads the same `CompKey` core but declines this tie rather than
deciding it, so closed↔closed order has exactly one owner. (The `η` slot walk
that used to share the comparator was retired in the #174 design round.)
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
| 4(c) | `topological_layer_order` | sift each generator to its earliest admissible braid-free layer — covering-identity span for positive source, point-span rule at the leftmost admissible slot for zero-source `η` (§2.3) |
| 5 | `simplify_units` | remove `Identity(0)` atoms; drop layers emptied as a result |
| 7 | `reorder_component_blocks` | transpose adjacent *free* component blocks (`closed ∥ anything`, `input-only ∥ output-only`) into rule-(i) order, over an identity-split refinement (§2.6) |
| 6½ | `reorder_zero_arity_columns` | transpose two adjacent **interval-aligned columns** whose block arities strictly commute, over the same identity-split refinement — the move between Step 6 (atoms) and Step 7 (whole components) that residuals §4.6(c)/(d) needed (§4.5) |
| 6 | `reorder_tied_zero_arity` | within-layer bubble reorder of strictly-commuting zero-arity atoms — `scalar < η < ε < solid`, then `G::cmp` (§2.5); content-only since #174 retired its component-order branch |

(Steps 7 and 6½ are staged *ahead* of Step 6 in the loop — a block or column
move can land an `η` beside an `ε`, and Step 6 repairs that on the same pass.)

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
  components clear and boundary-attached; full proof open — §4.4 status), via
  the point-span sift at its leftmost admissible slot (§2.3; issue #55 PR2, with
  the component-anchored slot walk retired in #174). Tensor- and compose-forms of the same morphism
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
- **Column order** — transposing a zero-arity-bounded **block** against a
  **column** of a larger component is covered by Step 6½ (§4.5) on
  interval-aligned pairs whose block arities strictly commute, so nested
  writings converge with their free ones:
  `Copy ; (id₁ ⊗ Zero ⊗ id₁) ; (id₁ ⊗ Discard ⊗ id₁) ; Add`
  = `(Zero;Discard) ⊗ (Copy;Add)`, and the sink form of §4.6(d).
  **Ablation-verified** as depending on Step 6½ — the five probes that break when
  the pass is disabled: `smc_canonicality_probes::trapped_closed_block_extracts`,
  `nested_sink_block_converges_with_free_writing`,
  `column_move_crosses_a_merging_wall`,
  `column_move_crosses_a_fused_wide_identity` and `multi_nested_blocks_extract`.
  Two further probes in the same family are convergence regressions **not**
  attributable to the pass — they converge with it ablated:
  `nested_source_block_converges_with_free_writing` (CE-A3, closed by the
  free-site retirement, §4.6(d)) and
  `column_interval_is_the_adjacency_run_not_the_block_span` (scope stated on the
  probe). `interval_alignment_check_is_exercised` covers the alignment test
  itself.
- **Residual ledger** — §4.6, restated 2026-07-28. **(a)** marked components:
  their blocks and columns are still not transposed (their `η`s now sift);
  witnesses `marked_encloser_blocks_the_column_move` and
  `marked_component_eta_sifts_and_converges`. **(b)** Ord-less closed↔closed
  order: closed 2026-07-27 by #79 P1. **(c)** trapped nested closed block:
  closed by the §4.5 column pass. **(d)** nested block solid on its opening
  side: sink form closed by the column pass, source form by the §2.6 free-site
  retirement. All three former `#[ignore]`d witnesses are live regressions.
  §4.6 is a ledger of *named* residuals, **not** a bound — a differential sweep
  still finds in-`𝔉` divergences outside all four letters, most of them
  predating this work.

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
> **Superseded in part 2026-07-28 (§4.4 v2):** the candid-status posture is
> replaced by a theorem — Lemma 4.3 (column pinning), Lemma 4.4 (layout
> freedom) and Theorem 4.5 (rigidity/canonicality on the smaller fragment
> `𝔉′`) — while rigidity on `𝔉` itself is withdrawn with a witness; the
> inventory in this note predates that rewrite.
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
  The coarsening question this note used to raise has **dissolved** rather than
  been answered. It asked what to do about the two passes that merely *read* the
  analysis — the sift and Step 6 — seeing a coarser partition than Step 7, since
  `merge_adjacent_identities` can fuse an `Identity` across a component boundary
  and the union-find then joins those components through it. Refining those two
  sites was tried in the #174 design round and **refuted**: it broke CE-R1 and
  CE-R2 and bought no shipped witness. The round then retired both read sites
  outright, so no consumer of the component analysis remains outside the two
  rewriting passes — and those rewrite on the refinement, where the question
  does not arise. Nothing reads a coarse partition because nothing reads a
  partition.)
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

> **Status (2026-07-29): `C` is implemented.** Until now this section specified
> a function that existed only on paper. It now ships as
> `src/prop/presentation/content.rs` (issue #57 a1, PR #188): `content_of`
> builds the cospan above by structural recursion with union-find gluing at `;`,
> `content_eq` decides isomorphism under both feet, and `canonical_key` gives a
> hashable form satisfying `canonical_key(a) == canonical_key(b)` iff
> `content_eq(a, b)`. The `Λ`-typing is not deferred: a node takes its producer
> tentacle's declared letter (its consumer's when it has no producer), so the
> module is word-generic exactly as this section is, and `content_of_colored`
> pins the one remaining case — a wire no generator touches, which monogamy
> forces onto both feet — from a `ColoredExpr`'s source word.
>
> Verified against the §4.6 corpora: content closes **253/253** divergent pairs
> on the published default corpus and **1162/1162** in braid mode (including the
> marked residual-(a) cases and the dead-braid-prefix shapes no `nf`-level fix
> reaches), with `canonical_key` agreeing with `content_eq` on every one, and
> zero false equalities across 2000 cross-corpus negative controls — the ten
> genuinely-equal hits among them each cross-checked against `nf`.
> `tests/content_equality_corpus.rs` is the tracker; `tests/content_equality.rs`
> carries the named §4.4/§4.6 witnesses.

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

### §4.4 Canonicality status and the rigidity theorem (v2, 2026-07-28)

The first draft of this section proved a rigidity theorem ("an
invariant-satisfying diagram is uniquely determined by its content on `𝔉`")
and derived full canonicality on `𝔉`. Adversarial review **refuted it** —
§4.6(d)'s CE-A family was a probe-verified SMC-equal pair *inside* `𝔉` whose
NFs differed — and the failure traced to a non-exhaustive case split in the
draft's central geometric lemma. This is the second version, written after a
three-track investigation round (2026-07-28: an adversarial pass-disjointness
referee, a column-pinning/induction analysis, and a full characterization of
the differential sweep's in-`𝔉` divergences). It records a theorem that is
true, and it withdraws the goal that was not. The #174 design round had left
the restoration owing two obligations, and both are resolved below: **(i)
pass disjointness** — the class-order family (Step 6) and the
component-order family (Steps 6½/7) never prescribe conflicting layouts —
resolved **false as stated**, with a ratified carve; **(ii) pinned-pair
geometric determination at column granularity** — resolved as a **split
verdict** at Lemma 4.3.

Verification tiers for the claims below, stated once: the sweep totals
(253/128/23), the ablation five, and every named witness are **pinned by
in-tree tests**. The remaining figures are **dated measurements**
(2026-07-28, scratch instruments over the same seeded corpus), re-derivable
from their stated classifiers — the 121/5/2 sub-shape split (layer counts,
then per-layer generator multisets, then full-atom multisets), the Path-1
12-of-128 (the §4.5 condition on the NF's component analysis), the 66%
retention (consumer-distance proxy for layer slack), the 42-of-128 fusion
note (stored-layer component counts compared across a pair) — and the
load-bearing ones (12-of-128, 121/5/2, the `η`-free zero) were independently
re-derived during the PR's review. Classifier-sensitive figures are quoted
coarsely for exactly that reason.

**Withdrawn: rigidity on the original `𝔉`.** The restoration plan ("restore
the theorem on `𝔉`, with the column move closing the refuted case split") is
**refuted, not deferred**. Witness, three generators, both sides `nf`
fixpoints, both idempotent, both inside `𝔉`, same content:

```
nf( (Copy ; (id₁ ⊗ Discard)) ⊗ Zero ) = [Copy, Zero] ; [id₁, Discard, id₁]
nf( Copy ; (id₁ ⊗ Zero ⊗ Discard) )   = [Copy] ; [id₁, Zero, Discard]
```

The mechanism is not a missing transposition move at all — it is **`η`
placement freedom**: the point-span sift evaluates admissibility at the `η`'s
*written* wire coordinate (`point_placement`), so a `Zero` written strictly
inside `Copy`'s output span is blocked one layer down while the same `Zero`
written beside the span escapes to layer 0, and no pass moves an `η`
horizontally past a non-commuting neighbour. The mirror writing
`Copy ; (id₁ ⊗ Discard ⊗ Zero)` — one §2.5 strict commutation away —
converges with the first, pinning the mechanism to the written slot. The
sweep characterization generalizes this: **all 128 in-`𝔉` divergent pairs
(and all 253 divergent pairs of any bucket) on the default corpus — which is
braid-free by construction — reduce to this one mechanism** — an `η` whose
consumer sits two or more layers below its earliest legal layer has several
SMC-legal (layer, slot) positions, and the pipeline has no pass that
canonicalizes among them. (Beyond that corpus two further freedoms are
witnessed below: the braid prefix of a braid-carrying writing, and the
cut-asymmetry shapes the conditional corollary records.) Witnesses:
`eta_layer_slack_separates_smc_equal_writings`,
`eta_slack_mirror_writing_converges`,
`single_strict_commutation_separates_fixpoints`.

**Proven, unconditionally (any diagram).**

- *Soundness*: NF-equality implies SMC-equality (Lemma 4.2's readback), and
  `C(nf(e)) = C(e)`.
- *Termination — status corrected (2026-07-28).* The §2.4 lexicographic
  measure covers every pass application **except** Step-6 swaps at a
  *both-readings adjacency* (defined under "pass disjointness" below), where
  Step 6 provably moves a block/column-counted pair against those counts —
  the question §2.4 used to leave open is settled in the negative, by
  machine-checked witnesses. At such adjacencies the loop exits through the
  `sd == prev` whole-pass check: the rewriting pass's move and Step 6's
  counter-move cancel exactly. Termination is empirically unbroken —
  1,502 adversarially constructed SMC-equal writings across 19 conflict
  families, plus the 100 000-pair sweep, all terminate — but the measure
  argument is no longer a complete proof, and completing it (or gating the
  passes so the old measure is restored) is tracked on issue #174.

**Proven, within-layer (issue #55 PR1).** At single-atom tied adjacencies the
Step-6 order is canonical — `nf(ε ⊗ η) = nf(η ⊗ ε)`, stable under context
(§2.5). The merge-monotonicity caveat this paragraph used to carry is
**deleted**: it existed because the tied comparator mixed component keys with
the class order, the #174 design round removed the mixing, and
`tie_sorts_before` now reads the two atoms and nothing else — the caveat's
precondition is structurally gone, so there is nothing left to prove.

**Facts that survived adversarial review** (corrected 2026-07-28 — one is
narrowed, one strengthened, and two non-facts are now recorded beside them):

- *Positional monotonicity* — braid-free layers never cross wires, so wire
  positions are consistent across the generator suffix.
- *Marking is content-level — on braid-free diagrams only* (narrowed).
  `mark_interleaved` runs over the *diagram's* component partition, and a
  braid coarsens it: the braid atom joins the components of both its wires,
  so a content-clear diagram can be marked. Witness
  (`braid_coarsening_marks_content_clear_diagram`):
  `(id₁ ⊗ Discard ⊗ id₁) ; σ_{1,1}` is content-clear (three content
  components, no repeated run in either owner word) yet `fragment_status`
  marks it — the braid merges the two through-wires into one diagram
  component whose input coordinates straddle `Discard`. On braid-free
  diagrams the two partitions coincide (Lemma 4.3) and the fact stands as
  originally recorded. Consequence for §4.6: in the braid-injecting sweep
  mode, the "marked" bucket is conservative relative to content.
- *Key-distinctness* — distinct anchored components have distinct rule-(i)
  keys (each boundary coordinate has exactly one owner, so least coordinates
  differ). **Strengthened** by Lemma 4.3: for braid-free components the keys
  are content-invariants at any point in the `nf` loop, not just at the
  fixpoint.
- *The braid prefix is a function of its permutation* — Step 3(b)'s canonical
  bubble-sort word (§2.2) is deterministic in the underlying permutation.
  (The permutation itself is **not** a content invariant — a dead input
  permutation survives to the NF while an SMC-equal writing has none;
  witness `braid_prefix_is_not_content_derived`. Canonical-given-the-
  permutation is all this fact says.)
- *Not content-invariant* (recorded so the next draft does not lean on them):
  `sizes[c]` counts atoms of the identity-split refinement, so it depends on
  a component's vertical stretch — the multi-atom guards of Steps 6½/7 are
  layout conditions, not content conditions. `component_reading` records one
  `ReadingAtom::Identity` entry per refined identity atom, so it too varies
  with stretch; it is only ever consulted at the closed↔closed tie, which
  `𝔉`'s condition 2 excludes — condition 2 was doing that proof work
  silently, and is credited here.

**Lemma 4.3 (column pinning — obligation (ii), resolved as a split verdict).**
Let `D` be any diagram, `D̂` its identity-split refinement, and `(c1, c2)` a
pair selected by Step 7's or Step 6½'s search. Then (1) neither component
contains a `Braid` (both passes gate on it); (2) each is therefore *exactly* a
connected component of the content hypergraph of `C(D)`; and (3)
`touches_input`, `touches_output`, the rule-(i) keys, and hence
`component_key_order` on the pair are functions of `C(D)` alone — at any
point in the loop, not only at the fixpoint.

*Proof.* (2) In the refinement every `Identity` has width 1, so the
union-find's share-a-wire test is exactly wire incidence, and the
empty-interval guard implements "a zero-arity hyperedge connects only through
its non-empty side" — content connectivity. A chain of width-1 identities
realizes one content node across layers, so content-connected implies
diagram-connected; the converse fails only *through a braid atom* (a
`Braid(1,1)` is not a content hyperedge, and joining its two wires can merge
two content components — but the braid is then a member of the merged
component), so a braid-free component cannot have been coarsened. (3) The
analysis reads owner words off layer 0's sources and the last layer's
targets — the two feet — and cospan iso is the identity on the feet (§4.1),
so each boundary coordinate's owner, and with it `in_min`/`out_min`/class,
is a content invariant. ∎

The split verdict on obligation (ii): the **direction** of every Step-6½/7
move is content-determined (Lemma 4.3 — and the braid guard is thereby
upgraded from oscillation hygiene to a load-bearing hypothesis), but the
**decision to fire** is not (the multi-atom guard reads `sizes`, a layout
quantity). The lifted form of the obligation — owner words determine a pinned
*column* pair's order — holds exactly as far as the keys do, because Step 6½
orders columns by their components' keys and declines its one tie.

**Lemma 4.4 (layout freedom).** A braid-free layered diagram satisfying the
§1 invariants is uniquely determined by its content together with the pair
`(λ, ι)`, where `λ` assigns each generator occurrence its layer and `ι`
assigns each **source-0** occurrence its slot **in the layer's atom
sequence** (not a wire coordinate — so two source-0 occurrences at one
coordinate are distinguished, and their relative order is `ι`'s to give).

*Proof.* At each layer boundary the wire word is a sequence of content nodes.
Every atom with positive source occupies the contiguous span of its source
nodes, so the within-layer order of positive-source atoms is forced by the
wire word; identity atoms fill the gaps and are fused maximally (§1
intra-layer fusion); `Identity(0)` is absent (§1). Only a source-0 atom has
an empty span, hence a free slot. ∎

Lemma 4.4 is the frame for everything else in this subsection: **rigidity is
exactly the statement that `(λ, ι)` is pinned by content.** Note what it does
*not* need — neither of `𝔉`'s conditions. Block order, column order and
closed-block placement are consequences of `(λ, ι)`, not extra data; the
clear/boundary-attached conditions matter to the *engine's guards* (Lemma
4.3, the reading-key note above), not to the theorem.

**`ldepth` and `η`-placement slack (content-intrinsic).** For a hyperedge `h`
of `C`: `ldepth(h) = 0` if every source node of `h` is an input-boundary
node (in particular if `h` has no source tentacle); else
`1 + max { ldepth(h′) : h′ produces a source node of h }` — the longest
directed path from the input foot, a function of `C` alone. (The sift moves
generators toward the *input*, so the input-side level is the notion that
matches the engine; the first draft's output-side version presupposed a
layout and is superseded.) Let `depth(C) = 1 + max_h ldepth(h)`. For a
source-0 hyperedge `h` with a single output node `z` (the `0 → 1` shape —
see the definedness bullet below for the other shapes), define its
**ceiling**:
`ceil(h) = ldepth(k)` if some hyperedge `k` consumes `z`, and
`ceil(h) = depth(C)` if `z` is anchored at the output foot. Then:

- `h` is **layer-pinned** iff `ceil(h) = 1` — its consumer sits at content
  level 1, so `h` has zero extra legal *levels*. **Caution (delta review,
  machine-verified):** this does *not* force `λ(h) = 0` — a consumer's
  tentacle order can pin the `η`'s coordinate strictly inside a producer's
  span, and the unique invariant-satisfying realization then holds it at
  `λ = 1` (witness `layer_pinned_eta_sits_below_layer_zero`:
  `Copy ; (id₁ ⊗ η ⊗ id₁) ; (id₁ ⊗ Add)`, one component, sift blocked by
  `Add`'s `(z, v)` tentacle order). Layer-pinnedness bounds the content-level
  room (`ceil(h) − 1` extra levels; zero here); converting pinnedness into a
  *unique* `λ` is exactly what the two flagged induction steps below still
  owe. A non-pinned `h` has `ceil(h) − 1 ≥ 1` extra legal levels: that is
  the layer slack.
- `h` is **slot-pinned** iff `z`'s wire coordinate at the boundary below
  `h`'s layer is the same in every braid-free realization satisfying the
  restated §1 invariants. This is deliberately *realization-quantified*
  (adversarial review, 2026-07-28): it ranges over exactly the diagrams
  Theorem 4.5 is about, so the `ι` half of "rigidity = pinning `(λ, ι)`"
  holds by hypothesis, and the theorem's real proof content is the `λ`
  induction. (An earlier draft offered a "sufficient condition" whose key
  word — *forced* — had no definition except the definiendum; it is
  deleted rather than repaired.)
- `h` has **slack** iff it is not both. Slack is a function of `C` and the
  invariant list alone, so by Lemma 4.2 it can be read off either of two
  SMC-equal writings.
- *Definedness scope* (adversarial review): these definitions cover the
  source-0 shape every shipped signature has, `0 → 1`. A source-0 hyperedge
  with **no** output node (`0 → 0`) or **several** (`0 → n`, `n ≥ 2`) is
  conservatively deemed slack-bearing — `𝔉′` excludes it — pending a
  tuple-level treatment. (A `0 → 0` scalar's *within-class order* is already
  content via `G::cmp`, §2.5; its layer/slot theory is what is undeveloped.
  Only test signatures have such generators today.)

**The fragment of record for the theorem:**

```
𝔉′  =  { e : nf(e) is braid-free,
         and no source-0 hyperedge of C(e) has slack }
```

The first condition is deliberately **per-writing**: NF braid-freeness is
*not* a content invariant — a dead input permutation keeps its canonical
braid prefix while an SMC-equal writing has none (witness
`braid_prefix_is_not_content_derived`:
`nf(σ_{1,1} ; (ε ⊗ ε)) = [σ] ; [ε, ε] ≠ [ε, ε] = nf(ε ⊗ ε)`) — so a
content-level "`C` admits a braid-free NF" reading would make the corollaries
below false, and whether any content condition can replace it is the open
braid-freedom question at the end of this subsection. The slack condition
*is* content-level.

On the default sweep corpus (braid-free by construction), the no-slack
condition excludes every one of the 253 divergent pairs **under §4.6's
layout-level classifier** while retaining 66% of the in-`𝔉` normal forms —
it is the observed divergence mechanism, not a carve fitted around it. One
caveat keeps those figures calibration rather than hypothesis: the
layout-level classifier and the content-level definition above can disagree
on cut-asymmetry shapes (the conditional corollary's witness is
content-pinned yet layout-slack), so the corpus figures do not *prove* the
exclusion under the theorem's own definitions.

**Theorem 4.5 (rigidity on `𝔉′`).** Let `C` be an anchored monogamous
directed acyclic cospan in which every source-0 hyperedge is layer-pinned and
slot-pinned. Let `D`, `D′` be braid-free layered diagrams satisfying the §1
post-`nf` invariants (as restated 2026-07-28, with the both-readings carve),
with `C(D) = C(D′) = C`. Then `D = D′`.

*Proof sketch (top-down induction from the input foot).* By Lemma 4.4 it
suffices to pin `(λ, ι)`; `ι` is pinned by hypothesis. For `λ`, maintain "the
wire word `W_j` and layers `0..j−1` are determined by `C`": the base is the
input foot (Lemma 4.1's anchoring); for the step, let `S_j` be the unplaced
occurrences whose producers all sit in layers `< j` and whose source nodes
are contiguous in `W_j` — every member of `S_j` is in layer `j` (its source
wires pass through layer `j` as identities by monogamy, are adjacent by
contiguity, hence fused into one covering `Identity`, so the earliest-
admissible clause would lift it from any later layer) and conversely; if
`S_j` were empty with occurrences remaining, layer `j` would be a
pure-identity layer beside a non-identity layer, violating §1. Layer `j`'s
atom sequence is then Lemma 4.4's; `W_{j+1}` follows. The only step that can
fail is contiguity being *destroyed* by a source-0 occurrence inserted at a
later layer splitting a span — which is exactly `ι`, pinned by hypothesis. ∎

Two steps remain **flagged open** — and a first discharge attempt is
recorded here precisely because it *failed* in the delta review
(machine-verified), so it is not re-trodden. The attempt assumed `ceil = 1`
puts every pinned `η` at layer 0 with its output node inside its consumer's
span in `W₁`, splitting nothing; the layer-pinned caution above refutes the
premise — a pinned `η` can sit at `λ > 0`, so *(a) monotone contiguity* must
be re-proven allowing pinned-`η` insertions at layers other than 0, with one
named open sub-question: can a slot-pinned `η` of a **marked** foreign
component (whose guards disable extraction) split a span and create
`λ`-ambiguity? And *(b) the occupancy/non-emptiness step* is defective as
sketched for source-0 occurrences — `S_j`'s membership conditions (all
producers above, source nodes contiguous) are vacuous for them, so the
induction must restrict `S_j` to positive-source occurrences and place
pinned `η`s by a separate earliest-admissible-coordinate rule. What *does*
stand in that step: contents with no braid-free realization at all (a
producer emits `(u, v)` where the consumer wants `(v, u)`) leave the theorem
vacuously true, consistent with its scoping; and the statement itself
survived two full adversarial rounds without a counterexample — including a
`λ`-ambiguity attack via generator races, which the §1 column clause
blocked. At proof-sketch density these two steps are open obligations, not
glossed lemmas. On the induction's *direction*: the first
draft's π circularity arose from admitting braids into the induction; with
braid-freedom as a hypothesis it dissolves, and top-down from the input foot
is the direction that matches both `ldepth` and the sift. The bottom-up
route the first review recommended is hereby re-scoped to the braided
*extension* (its real job: the braid prefix is the §2.2 bubble-sort word of
the permutation between the input foot and the wire word the top-down
induction delivers at the prefix boundary — i.e. bottom-up determines the
prefix *last*, from data the induction supplies, instead of assuming it).
It is deliberately not attempted here.

*Corollary (canonicality on `𝔉′` — **conditional**).* For SMC-equal
`e, e′ ∈ 𝔉′` — membership already gives both NFs braid-free and their shared
content slack-free — whose normal forms **both satisfy the restated §1
invariants**, `nf(e) = nf(e′)`: equal content by Lemma 4.1,
then Theorem 4.5. Neither condition is automatic, and each failure mode has
a committed witness — both supplied by this subsection's own adversarial
review, which refuted the unconditional corollary an earlier draft stated:

- *Fixpoints can violate a non-excepted §1 clause.* The shipped
  `adjacent_column_cuts` demands the **right** column's whole layer presence
  be one contiguous run where the §1 clause (and §4.5's own column
  definition) requires only local runs, so Step 6½ never seeds when the
  enclosing component's presence is split (`[L, B, L]`), and the inverted,
  non-excepted, strictly-commuting-at-block-level pair survives to the
  fixpoint. Witness `cut_asymmetry_separates_smc_equal_writings_inside_f_prime`:
  a divergent SMC-equal pair whose content is in `𝔉′` — its `Zero` is layer-
  and slot-pinned precisely because the nested layout is *not*
  invariant-satisfying. The asymmetry is an engine defect, filed at landing;
  fixing it is what would discharge this condition, and the corollary is to
  be re-verified (and, if clean, unconditionalized) in that fix's PR.
- *NF braid-freeness is per-writing* — the `𝔉′` definition note above
  (`braid_prefix_is_not_content_derived`).

*Corollary (the `η`-free case).* If `C` has no source-0 hyperedge and
`e`, `e′` are SMC-equal writings whose NFs are **both braid-free**, then
`nf(e) = nf(e′)`. The slack hypothesis is vacuous, and on a braid-free NF
with no source-0 atom all three ordering clauses are vacuously satisfied
because all three passes are provably inert: Step 6 needs a
strictly-commuting adjacency, which needs a source-0 atom (`Identity(0)` is
gone by `simplify_units`, which runs earlier); Step 6½ needs a zero source
width at the interval top, impossible when every column has an atom with
positive source in every layer; Step 7 needs a free pair, and with no
source-0 hyperedge every node traces back to the input foot, so every
component is input-attached and no pair is free. So every such fixpoint
satisfies the restated invariants and Theorem 4.5 applies. The braid-free
conditioning is necessary — the braid-prefix witness above is exactly an
`η`-free SMC-equal pair separated by a dead braid prefix. Empirically: the
16 103 corpus pairs that are `η`-free on both sides show **zero**
divergences in any bucket, marked included. ∎

**Pass disjointness (obligation (i)): FALSE as stated — resolved by a
ratified carve, not a lemma.** The obligation read: every move is either a
free zero-width permutation (content-decided, Step 6) or a pinned
wire-carrying transposition (geometry-decided, Steps 6½/7), *and no adjacency
admits both readings*. Define: an adjacency of atoms `u ∈ C_u`, `v ∈ C_v`
(`C_u ≠ C_v`) **admits both readings** when `(u, v)` strictly commute (§2.5)
and `{C_u, C_v}` passes the guards of a rewriting pass — Step 7's (free pair,
neither braid-carrying, whole-run adjacency in every shared layer) or Step
6½'s (admissible pair, *distinct* keys, a transposable interval seeded at the
adjacency). Such adjacencies exist, with the two orders opposed —
machine-verified in `tests/pass_disjointness_probes.rs`, three shapes:

1. *Step 7 vs Step 6, test signature*: a `0 → 0` scalar beside a closed
   multi-atom block, with the generator `Ord` chosen so the reading key and
   the class order disagree. (The equal-key carve keeps Step **6½** out of
   this tie — but the #79 P1 reading key handed Step **7** exactly the tie
   6½ declines, which is the hole in the previous version of this
   subsection's carve claim.)
2. *Step 7 vs Step 6, shipped generators, inside `𝔉`*:
   `Discard ⊗ (Zero ; Scalar(true))` — the sift-proof `η` sits in the top
   layer; rule (i) wants the input-anchored `Discard` left, the class order
   wants the `η` left.
3. *Step 6½ vs Step 6, shipped generators, inside `𝔉`, at **distinct**
   keys*: `(Zero ⊗ Discard ⊗ Scalar(true)) ; Add` — both components touch
   the input boundary, so Step 7 is structurally blocked, but Step 6½ has no
   boundary-freedom guard and transposes the columns toward key order while
   Step 6 orders the same atoms `η` before `ε`.

At every such adjacency the engine cycles *inside* each pass — the rewriting
pass swaps toward component order, Step 6 swaps back toward class order — and
the loop exits through the whole-pass `sd == prev` check because the two
moves cancel exactly. Three consequences, all now recorded where they bite:
the pre-restatement §1 transposition clauses were **violated at real `nf`
fixpoints** (and on these contents the old clause set was jointly
unsatisfiable — no layout satisfies both orders); §2.4's per-step
non-increase claim was false at these adjacencies (restated there); and the
previous version of this subsection's claim that the closed↔closed carve
"keeps Step 6's class fallback from fighting Step 6½" was wrong twice over —
shape 1 routes the fight through Step 7's reading key at the very tie 6½
declines, and shape 3 fights 6½ itself at distinct keys with shipped
generators.

**The ratified resolution** (2026-07-28): the class order wins. Step 6 runs
last in the loop and is idempotent, so every observed fixpoint is
Step-6-sorted; the §1 transposition clauses are restated to except
both-readings adjacencies (see §1), which ratifies the shipped behaviour
exactly — no engine change, no pin movement. What the carve does *not* do is
prove the exactness of the cancellation that termination now visibly rests
on, nor rule out a three-way interaction that fails to cancel; no such case
was found (1,502 writings, 19 families, all watchdogged), and the engine-side
alternative — gating the rewriting passes off both-readings pairs, which
would restore the §2.4 measure outright — is filed on issue #174 as an
owner option rather than taken silently here.

**And the conflict that actually produces divergences is a different one.**
The order fights above break documented invariants, not uniqueness — every
conflict witness converges from all its writings. The divergence-producing
interaction is **Step 4(c) vs Step 6**: the sift reads the written slot of an
`η` *before* Step 6 has any say, and the class order then holds the `η` at a
blocking coordinate (in the withdrawal witness above, `ε < η` would have
freed it — which is an observation about the mechanism, not a repair: the
#174 round refuted reversed-class candidates empirically). That interaction
is exactly `η` placement slack, `ι`, the hypothesis of Theorem 4.5.

**What remains open**, exactly:

- **Canonicalizing `ι`** — by Lemma 4.4 this is the *entire* remaining gap on
  braid-free diagrams: a canonical, content-derived choice of layer and slot
  for slack `η`s. It is also the sharpest available framing for issue #57: a
  content-level rewriting engine (§4.7) represents morphisms by `C` itself
  and never chooses `ι` at all — the readback would inherit whatever `nf`
  pins, and Theorem 4.5 is precisely the statement that on `𝔉′` there is
  nothing left to choose.
- **Braid-freedom as a content condition** — NF braid-freeness is
  per-writing, settled by the braid-prefix witness. What stays open is
  whether a content condition ("`C` admits a non-crossing layered embedding
  compatible with both feet") could replace it — i.e. whether `nf` could be
  made to land braid-free exactly when `C` admits it, which would need a
  dead-permutation elimination pass (an engine change, not attempted here).
  Every empirical result above sits inside the braid-free hypothesis.
- **Completing the termination proof** (or restoring the measure by the
  engine-side gate) — above.
- **General source-(d) closure is not claimed, here or anywhere.** In-scope
  closure remains "`𝔉` ∩ interval-aligned nestings, probe-verified" (§4.5);
  the general shape routes to issue #57's content engine, and nothing in
  this subsection touches it.

### §4.5 The column move (Step 6½, implemented 2026-07-28)

The freedom `nf` used to leave uncanonicalized is precisely `X ⊗ B = B ⊗ X`
where `B` is a **multi-atom zero-arity-bounded block** (`0 → 0`, `n → 0`, or
`0 → n` read as a block) and `X` is a **column** — a single atom or a bundle
of identity wires belonging to a *larger* component. Neither existing pass
could make the move: Step 6 compares only adjacent *atoms*, and a solid
block-head never strictly commutes with anything; Step 7 transposes only whole
*components*, and the column's component (which extends past the block on
both sides) is never free against the block under condition (c). A
single-atom `η` does escape via Step 6 when the neighbouring block is
η-headed — the convergences review confirmed — which is exactly why only
nestings **solid on the opening side** (solid-headed sink blocks,
solid-tailed source blocks) witnessed the residual.

Two repair paths were recorded on issue #174. **Path 2 was taken.**

1. **Narrowed fragment** (content-level, proof-only, *not taken*): add to `𝔉`
   the condition "no input-attached component contains an `η`, and no
   output-attached component contains an `ε`" — walls never open, restoring
   the enclosure dichotomy. Sufficiency is now **refuted** (2026-07-28): 12 of
   the differential sweep's 128 in-`𝔉` divergent pairs satisfy the condition
   and diverge anyway — the actual mechanism (`η` placement slack, §4.4) is
   not confined to opened walls, so this fragment would have carried a false
   theorem. The condition also excludes most useful SFG diagrams (anything
   mixing boundary attachment with internal `η`/`ε`); the path is dead on
   both counts.
2. **Pipeline generalization** (*taken*, PR-A on issue #174):
   `reorder_zero_arity_columns` (Step 6½) extends the transposition from whole
   components to interval-aligned **columns**, subsuming both §4.6(c)'s
   extraction move and §4.6(d).

**The pass.** Over the same identity-split refinement Step 7 rewrites on, a
**column pair** is a layer interval together with, in every one of its layers, a
contiguous run of one component's atoms sitting immediately left of a contiguous
run of another's. It is **interval-aligned** when the three cuts — left of `X`,
between `X` and `B`, right of `B` — sit at the same wire coordinate read from
above and from below at every internal boundary; alignment is what makes the
interval's morphism factor as `A ⊗ X ⊗ B ⊗ C`, so the move is a two-column
tensor transposition rather than a conjugation by braids. The pair transposes
when its block arities strictly commute at the interval's own boundaries,

```
(src X = 0 ∨ src B = 0) ∧ (tgt X = 0 ∨ tgt B = 0)
```

— §2.5's criterion read at column granularity, both connecting braids again
`σ_{0,n} = id`. A column closed over the interval (`0 → 0`) commutes with every
other, which is §4.6(c)'s extraction move for free.

Direction is `component_key_order` — rule (i)'s plain `CompKey`, the same core
Step 7 reads, so the two rewriting passes cannot disagree with each other (§2.6;
the free sites read no component order at all). The pass declines the one tie
that order admits — two *closed* components — leaving closed↔closed order to
Step 7's whole-block reading key (#79 P1). What the decline does **not** buy is
keeping Step 6's class order out of the rewriting passes' way — that hope was
refuted (2026-07-28): Step 7 decides by reading key exactly the tie this pass
declines, and this pass itself meets Step 6 at *distinct* keys with shipped
generators (§4.4 "pass disjointness", shapes 1 and 3); the restated §1 clauses
carve those both-readings adjacencies to the class order. Its guards are
Step 7's: no braid-carrying component (braid
placement stays §2.1's; witness `braid_bearing_encloser_blocks_the_column_move`,
verified decisive by ablation), no marked component (residual (a) unchanged),
and at least one component multi-atom. Termination is `column_inversion_count`,
§2.4 (with the both-readings exception recorded there).

The interval is taken as the maximal run of layers over which the two
components keep the adjacent-run shape, with shorter sub-intervals containing
the seed tried longest-first if the maximal one fails alignment or commutation
— a completeness safety net, and fixing the search order is what keeps the pass
deterministic.

**What actually depends on the pass.** Ablating Step 6½ breaks exactly five
probes: `trapped_closed_block_extracts`,
`nested_sink_block_converges_with_free_writing`,
`column_move_crosses_a_merging_wall`, `column_move_crosses_a_fused_wide_identity`
and `multi_nested_blocks_extract`. It does **not** break
`nested_source_block_converges_with_free_writing` — CE-A3 was never a column
residual (below) — nor
`column_interval_is_the_adjacency_run_not_the_block_span`, which is kept as a
nested-block convergence regression with that scope stated on it. The
interval-alignment test is separately verified to do real work: instrumented
over the same 100 000-case corpus it rejected **5 780** candidate intervals, and
the delta-shrunk smallest such case is `interval_alignment_check_is_exercised`.
That count is **not** pinned by the committed sweep tracker, which does not
instrument the alignment branch: it was obtained by temporarily counting
rejections inside `column_pair_is_transposable`, and re-deriving it means redoing
that instrumentation. The probe guards the behaviour; the number is provenance
for the claim that the check is not decorative.

Scope note (2026-07-28): all five pass-dependent probes involve a `Zero` with
placement slack, so they sit **outside** Theorem 4.5's fragment `𝔉′` — the
column pass buys convergences the theorem does not cover, and the theorem
covers diagrams on which the pass is provably inert. The two are complements,
not overlaps.

The `d = 2` collision pins did not move **for the column pass**: its residuals
need expression depth ≥ 3, so the enumeration is structurally blind to them
(§4.6(c)). They did move `+1` on every rig for the free-site retirement — see
the `tests/graphical_linalg.rs` module docstring for the witness diff.

### §4.6 The residual ledger (restated 2026-07-28)

Every diagram still normalizes soundly and terminates; what is limited is
uniqueness.

**Read this section as a ledger of *named* residuals, not as a bound.** The
four lettered entries below are the freedoms that were identified, reproduced
and tracked; three are closed. What they are not is a complete inventory of
where `nf` fails to converge, and the design round established that they never
were. A seeded 100 000-case differential sweep — random SFG expressions each
paired with one sound interchange rewriting of itself — finds **253 divergent
pairs, 128 of them inside `𝔉`**, on the shipped engine. Those are not residual
(a): they are unmarked and boundary-attached. They include shapes with no
interleaving, no closed component and no nesting at all, and roughly
150 of the 253 predate every line of #174. Earlier drafts of this section wrote
"the open set is down to (a)"; that claim was wrong on every build the project
has ever shipped, and it is withdrawn. (A correction to an earlier draft of
*this* restatement, 2026-07-28: it cited "the engine reviewer's case 7079" as
a live layer-count exemplar — that case **converges** on the shipped engine;
its divergence was the review-round engine's and was closed by the free-site
retirement, not the column pass. The class of shapes it exemplified is real
and present in the 128; that particular index is not.)

**The 128 are one mechanism (characterized 2026-07-28; default corpus,
which is braid-free by construction).** An instrumented
re-run of the same corpus classified every in-`𝔉` divergent pair: all 128 —
and all 125 outside `𝔉` — are **`η` placement slack** (§4.4): a `Zero` whose
consumer sits two or more layers below its earliest legal layer has several
SMC-legal (layer, slot) positions, the point-span sift reads the written one,
and no pass canonicalizes among them. By sub-shape: 121 differ in an `η`'s
layer assignment at equal layer count, 5 in its slot within one layer, 2 in
layer count; the column pass is involved in exactly one case and decides
none; every case contains both a `Zero` and a `Discard`; the 16 103 pairs
that are `η`-free on both sides diverge **nowhere**, marked bucket included.
Two of the five slot-slack cases trace to a distinct engine asymmetry —
`adjacent_column_cuts` takes a *maximal local run* on the left column but the
component's *whole layer presence* on the right, so a fragment-symmetric,
interval-aligned, strictly-commuting pair can be declined — filed separately
rather than silently widened — and the same asymmetry is what makes §4.4's
canonicality corollary *conditional*
(`cut_asymmetry_separates_smc_equal_writings_inside_f_prime`). Excluding `η`
slack retains 66% of the in-`𝔉` corpus and calibrates Theorem 4.5's fragment
`𝔉′` (whose own definitions are content-level — the classifier here is
layout-level; §4.4's verification-tiers note records the gap).

Two measurement caveats, recorded here because this section publishes the
numbers: `fragment_status` reads the **stored** layers (deliberately — see its
docstring), and identity fusion can coarsen differently across a divergent
pair's two writings (it does in 42 of the 128), so the in-`𝔉` bucketing of a
*divergent* pair is itself mildly presentation-sensitive; and in the
braid-injecting mode the "marked" bucket is conservative relative to content —
a braid can merge components and mark a content-clear diagram (§4.4,
`braid_coarsening_marks_content_clear_diagram`) — so the residual-(a) tracker
should be read as an upper bound on content-level marking.

The honest statement of what is verified: **the `smc_canonicality_probes` suite
is the gate**, and it is a suite of named convergences, not a bound on
divergence.

Calibration, same corpus and same seed (100 000 pairs), measured against the
pre-#174 engine:

| | pre-#174 | shipped | rejected `out_min` variant |
|---|---:|---:|---:|
| divergent pairs, total | 1311 | 253 | — |
| …inside `𝔉` | 192 | 128 | 17 |
| …on marked cases | 888 | 23 | — |
| intransitive comparator triples | 0 | 0 | ≈37% of diagrams |

**The corpus, so the numbers above are reproducible.** The shipped column is
pinned by `published_divergence_figures_reproduce` in
`tests/smc_nf_differential_sweep.rs` — the design round's own driver, ported into
the tree. It is `#[ignore]`d (a 100 000-pair sweep) and run with `--ignored` when
the normal form changes; that test and this table quote each other, so they are
re-pinned together or not at all.

The corpus is 100 000 cases, each generated purely from
`splitmix64(seed ^ index)` with seed `0x9E37_79B9_7F4A_7C15`, so it is identical
across builds and any case can be re-run in isolation by index. A case is a
random SFG expression over `BoolRig` (≤ 4 layers, ≤ 7 atoms per layer, wire width
capped at 7, generators drawn from `Copy`/`Add`/`Zero`/`Discard`/`Scalar` plus
identities) paired with **one sound interchange rewriting of itself**, so every
pair is SMC-equal by construction and any NF difference is a canonicality failure
rather than a generator artifact. "Inside `𝔉`" is decided on the normal form: no
component marked by `mark_interleaved`, every component touching a boundary.

The pre-#174 column was measured the same way against that engine and is **not**
pinned by anything in-tree — it is a historical baseline, not a regression
target. And note what the tracker is not: `smc_canonicality_probes` stays the
gate of record, deciding named convergences; this sweep is a tracker, and a move
in it is a signal to diagnose rather than a failure in itself.

The `out_min` variant reaches the lowest in-`𝔉` count and is still **rejected**:
an intransitive comparator has no total order for §4.4 to build on, and it
regresses the named CE-R1. Lower is not the objective — a comparator that is a
function of the morphism is. Residual (a) is improved (888 → 23) and open.

The lettered ledger:

- **(a) Marked (interleaved) components — open, narrowed 2026-07-28.** Guard 3
  leaves a marked component's blocks and columns untransposed, because
  transposition is not braid-free there. Its `η`s *are* now sifted: the design
  round retired the guard from the sift and from Step 6's comparator, leaving it
  in Steps 7 and 6½ only, and divergences on marked cases fell 888 → 23 on the
  default corpus. Witnesses: `marked_encloser_blocks_the_column_move` (the
  guard, verified decisive by ablation) and
  `marked_component_eta_sifts_and_converges` (the widened sift surface).

  Tracked quantitatively by `published_braid_mode_figures_reproduce` in
  `tests/smc_nf_differential_sweep.rs`, which is the residual-(a) tracker: the
  default corpus produces too few marked cases to measure this residual, so that
  test sweeps a **braid-injecting** corpus where components own non-contiguous
  boundary intervals and guard 3's marking actually fires. Its figures are a
  different corpus from the calibration table above and are not comparable to
  it. Tracked on issue #174.
- **(b) Closed↔closed order — CLOSED (2026-07-27, #79 P1).** All closed
  components share the rule-(i) key `(closed, 0)`; distinct closed blocks
  formerly kept their presentation order for want of a content-derived
  tie-break. The `Ord` bound on `G` (#79 P1) supplies it: Step 7 sorts
  equal-key closed blocks by their in-situ readings (§2.6), and equal
  readings are identical blocks. Witness un-ignored and renamed
  `closed_blocks_sort_by_content_key`. Issue #174 (residual 1, closed).
- **(c) Trapped nested closed blocks — CLOSED (2026-07-28, Step 6½).**
  *(Found in the 2026-07-27 proof phase; the reason condition 2 is in `𝔉`.)*
  A closed component written strictly inside another component's wire span used
  to be inextricable: its `η`'s coordinate falls strictly inside the enclosing
  atom's target span (sift blocked — correctly, since the closing atom is
  foreign and no own-component gap-closer exists), and Step 7 never saw an
  adjacent free pair because the surrounding identity wires belong to the
  *enclosing* component, so the closed block's run was never adjacent to a
  whole-component run. The §4.5 column pass makes the move: a `0 → 0` column
  strictly commutes with the enclosing component's identity column over their
  shared interval, rule (i) sorts closed leftmost, and once the block is out the
  encloser's wires re-fuse and the blocked sifts fire. Witness un-ignored
  (`trapped_closed_block_extracts`), now a regression:

  ```
  nf( Copy ; (id₁ ⊗ Zero ⊗ id₁) ; (id₁ ⊗ Discard ⊗ id₁) ; Add )
    = nf( (Zero ; Discard) ⊗ (Copy ; Add) )
    = [Zero, Copy] ; [Discard, Add]
  ```

  The two are SMC-equal (bifunctoriality with `id₀`; both sides have the same
  content — one closed loop, one through-component), and now have the same
  normal form. The nested and free writings having identical content is why *no
  content-level fragment condition could have separated them*: the residual was
  irreducibly presentation-level, so a pipeline move — not a fragment condition
  — was the only available fix, and it is why `𝔉` still excludes closed
  components (condition 2 is now conservative rather than necessary; whether it
  can be dropped is a §4.4 proof question, not a behaviour one). A caution from
  the same review stands: the draft claimed "only closed components can be
  trapped" (a nested boundary-attached component being marked by guard 3) —
  **refuted**; enclosure of a component's *wires* does not imply enclosure of
  its *attachment* when the encloser's wall opens at its own `η`/`ε`, and that
  is residual (d). Also still true: the d = 2 collision trackers cannot see this
  trap (it needs a producer above, `η` and `ε` inside, and a consumer below —
  expression depth ≥ 3), which is why the pinned baselines did not move when the
  column pass closed it.

  For diagrams whose closed components are all written *un-nested*, closed
  blocks sort leftmost as a class (rule (i)) and are placed canonically among
  themselves — probe-verified by `closed_block_placement_converges`,
  `block_transposition_converges`, and
  `block_transposition_crosses_fused_identity_padding`.

- **(d) Nested zero-arity blocks, solid on the opening side — CLOSED
  (2026-07-28); sink form by Step 6½, source form by the free-site retirement.** *(Found in adversarial review, 2026-07-27 — the
  refutation of the draft theorem; inside `𝔉`.)* A multi-atom `n → 0` (or dually
  `0 → n`) block written at a coordinate strictly inside another component's
  span, with a **solid** atom on the side facing the enclosing wall's opening
  (head of a sink block, *tail* of a source block), used to converge with none of
  its free writings. Witnesses un-ignored
  (`nested_sink_block_converges_with_free_writing`,
  `nested_source_block_converges_with_free_writing`), now regressions:

  ```
  nf( (Zero ⊗ s ⊗ id₁) ; (id₁ ⊗ Discard ⊗ id₁) ; Add )
    = nf( (s ; Discard) ⊗ ((Zero ⊗ id₁) ; Add) )
    = [s, Zero, id₁] ; [Discard, Add]                      (CE-A)

  nf( Copy ; (id₁ ⊗ Zero ⊗ id₁) ; (Discard ⊗ s ⊗ id₁) )
    = nf( (Zero ; s) ⊗ (Copy ; (Discard ⊗ id₁)) )
    = [Zero, Copy] ; [s, Discard, id₁]                     (CE-A3)
  ```

  (`s` a solid `1 → 1` generator.) Both components are boundary-attached and
  unmarked, so the pair sat inside `𝔉` — same content, different fixpoints, which
  is what refuted the draft §4 theorem.

  **The two forms turned out to have different causes**, and the correction is
  worth recording because the first fix attempt was built on the wrong one.
  *CE-A (sink form)* is a genuine column residual: Step 6 cannot bubble past the
  solid head and Step 7's free-pair test is whole-component, while the actual
  freedom is column-vs-block. Step 6½ transposes it, and ablating the pass
  re-breaks the witness. *CE-A3 (source form)* is **not**. It was blocked by
  Step 6 refusing to bubble the nested block's `η` past the encloser's `ε`,
  because the tied comparator's rule-(i) branch ranked the two components ahead
  of the class order. Retiring that branch (§2.6) lets `η < ε` fire and the
  ordinary sift finishes; ablating Step 6½ leaves CE-A3 converging. The first
  PR-A attempt read the source form as a comparator problem and added an
  `out_min` clause for it; the clause was then refuted by CE-R1 and by
  intransitivity, and the real fix was to remove machinery rather than add it.
  An η-headed nested block converged even before all this (Step 6 walks the
  single `η` out), which is why the shape needed the solid head to bite at all.
  The pins are blind to (c) and to CE-A at `d = 2`; the `+1` re-baseline they
  did take is the free-site retirement's, i.e. CE-A3's own fix.

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
well-definedness of that readback is exactly the §4.4 canonicality question:
rigidity proven on `𝔉′` (Theorem 4.5; the `nf`-level corollary there
conditional on the filed cut-asymmetry fix), probe-verified on the §2.6 and
§4.5 families beyond it, and open in general, where the dominant remaining
freedom is a single named function — the `η` placement choice `ι` (§4.4). That is the sharpest argument
this document can offer #57: an engine that rewrites content directly never
chooses `ι` at all, and inherits `nf`'s choice only at readback. What #57
would add over the §2.4 pipeline is rewriting modulo *user equations* on the
same substrate; what it inherits from this section is that SMC-coherence
never needs rewriting at all — it is quotiented away by `C` itself.

> **Status (2026-07-29): the #57 work split, and the equality half landed.**
> The spike that sized this section separated it into two halves priced very
> differently, and only the first was taken.
>
> **(a1) Content equality — shipped.** `C` and its equality are in tree (§4.1's
> status note), and as of PR2 they are the **equality of record at the SMC
> layer**: `Presentation::eq_mod`'s congruence-closure path settles SMC
> coherence by content equality, and `ColoredExpr::eq_colored` does the same on
> the worded surface. So the sentence above ("an engine that rewrites content
> directly never chooses `ι` at all") is now literal for equality *at that
> layer*: the `η` placement choice is never consulted when deciding whether two
> writings are SMC-equal, and every pair `nf` separates for that reason is
> decided equal. There are **two decision layers**, worth stating once: content
> decides SMC coherence *exactly* (Lemma 4.1, both directions), and congruence
> closure decides *user* equations above it, with its existing
> bounded-completeness caveats.
>
> `nf` is not a third layer, and it is also not out of the decision business.
> What changed is narrow: it no longer decides the SMC layer on the
> well-formed path. It remains the canonicalizer inside
> `kb::CongruenceClosure`'s `smc_refine` fixpoint — which NF-normalizes each
> class representative and merges the class with the result, so NF quality
> still affects which user-equation classes close — and it remains the fallback
> outside content's domain (an arity-ill-formed expression, where `content_of`
> is undefined and `eq_mod` / `eq_colored` must still answer). Display and
> readback are unchanged.
>
> **(a2) Convex-DPO rewriting modulo user equations — deferred**, on the
> unchanged ground that its only consumer would be a functor-less presentation.
> Deferring costs little now that `C` exists: a2 would start from a landed
> substrate rather than from this specification.
>
> **Correction of record (2026-07-29).** An earlier #57 knowledge-base report
> claimed "Lafont proves termination for the bialgebra structure". That is
> **refuted** against the cached anchor. Lafont's strictly-monotone-interpretation
> technique (Appendix A, p. 300) is proved for his PROP **F** of functions only
> — *not* this document's fragment `𝔉`, an unrelated symbol that happens to
> collide; for the bialgebra-bearing `L(Z₂)` system he states termination as a
> **conjecture** and documents the obstruction — `ε : 1 → 0` admits no strictly
> monotone interpretation into `ℕ*⁰`. The nearest actual proof in the cached anchors is BGKSZ **Thm 6.1**:
> termination for the *non-commutative* bimonoid, via a lexicographic
> path-counting measure that handles `ε` by counting paths *to* `ε`-hyperedges
> rather than by interpreting it. The commutative case an `E_18`-style system
> would need is unproven in the anchors, which *raises* a2's estimated cost.
> The false claim never reached this document; it is corrected here because
> this section is where a future a2 attempt would start reading.
