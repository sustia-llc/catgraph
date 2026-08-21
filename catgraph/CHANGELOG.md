# Changelog

All notable changes to `catgraph` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the crate adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed — BREAKING

- **`frobenius::frobenius_to_cospan` is now a re-export of
  `cospan_algebra::frobenius_to_cospan`**
  ([#336](https://github.com/sustia-llc/catgraph/issues/336)) — one function
  where the G1
  merge briefly had two. #283 and #284 landed the Prop 3.8 semantics map
  independently, on branches each reviewed against `96cfea7`, so neither
  reviewer could see the other. The `cospan_algebra` body survives (the deeper
  review record: eight rounds, the bubble semantics, the incomparability
  analysis) and the `frobenius::operations` body is deleted, along with its
  private `operation_to_cospan`. Every existing import — `tests/frobenius_axioms.rs`,
  `tests/rayon_parallel.rs`, `tests/compact_closed.rs`, `tests/common/mod.rs`,
  `tests/hypergraph_functor.rs` — compiles unchanged.

  Two things a `pub use` could not bridge, so they **change for callers of the
  `frobenius::` path**:

  - **Bounds narrow.** The surviving function requires `Lambda: Send + Sync` and
    `BlackBoxLabel: Send + Sync` (it recurses through
    `special_frobenius_morphism`, which needs them); the retired one did not.
    A `Copy + Eq + Debug` label that is not `Send` no longer compiles here.
  - **Error variant changes.** An `UnSpecifiedBox` is now rejected with
    `CatgraphError::Interpret`, where the `frobenius::` path returned
    `CatgraphError::Composition`. The two messages were merged rather than one
    dropped — it now names both the generator (`UnSpecifiedBox`, which
    `tests/frobenius_axioms.rs` asserts) and the arities (`N in, M out`) — and
    all three facts are pinned in
    `frobenius::to_cospan_pin::black_boxes_are_rejected_by_both`.

  The two docstrings were merged into the survivor, keeping every measured
  claim: the blockwise-tensor/pushout-composite description and the Def 2.5
  decision-procedure paragraph from #283, the Prop 3.8 licensing and the
  incomparable-on-scalars analysis with both witnesses from #284. Both test sets
  stay; neither is weakened. The README's two Feature Map rows for Prop 3.8
  collapse into one.

### Added

- **`cospan_algebra::frobenius_to_cospan`** — interprets a
  `FrobeniusMorphism<Lambda, _>` in `Cospan<Lambda>` layer by layer, inverse in
  spirit to the existing `cospan_to_frobenius`
  ([#284](https://github.com/sustia-llc/catgraph/issues/284)). It errors
  (`CatgraphError::Interpret`) on an `UnSpecifiedBox`, which has no image in the
  free hypergraph category.

  Fong-Spivak Prop 3.8 is a one-to-one correspondence between special
  commutative Frobenius monoids (SCFMs) in a symmetric monoidal category `C` and
  strict symmetric monoidal functors `(Cospan, ⊕) → (C, ⊗)` — **both** of its
  directions are functors *out of* `Cospan`, so neither of them is this map.
  What Prop 3.8 licenses is the construction: the black-box-free part of
  `FrobeniusMorphism_Λ` is the free SCFM prop, `Cospan_Λ` carries an SCFM
  structure on each object (Ex 2.8), and the proposition turns that structure
  into the interpreting functor.

  This is the *observable* the compact-closed suite was missing.
  `FrobeniusMorphism`'s derived `Eq` compares layer vectors and composition only
  applies a local `two_layer_simplify`, so equal diagrams routinely differ
  syntactically and no equality on `FM` could state §3.1's propositions.
  Composing with `Cospan::canonical_form` gives a semantic one — equality up to
  apex isomorphism. ⚠ **Incomparable with SCFM on scalars — neither direction
  is exact**, both measured. `two_layer_simplify`'s rule 3 cancels a spelled
  `η;ε`, so it shares an image with `FrobeniusMorphism::identity(&vec![])`
  without being SCFM-equal to it (*not complete*), and shares SCFM-equality with
  `Spider(z, 0, 0)` — which `generator_to_cospan` builds directly as the bubble
  — without sharing its image (*not sound*). See "Known discrepancy"
  below. So the new pins catch any change to a term's boundary-to-apex
  connectivity, and to any bubble that survives the layer simplifier; the one
  thing they cannot see is a change whose only effect is to add or drop an
  adjacent `η;ε` pair, which the simplifier cancels before this function runs.

### Fixed

- **`Frobenius::basic_interpret`'s default interpreted every braiding as the
  identity** ([#284]). It built `Permutation::try_from(vec![0, 1])` — the
  identity permutation — where `σ: [z1, z2] → [z2, z1]` needs the transposition
  `[1, 0]`. Dead code today (`FrobeniusMorphism` is the only implementor and
  overrides `basic_interpret`), but it is the reference semantics any future
  implementor inherits, and `cospan_algebra::generator_to_cospan` — its
  `Cospan`-valued twin, added in this release — builds the true transposition,
  so the two disagreed.

- **`Frobenius::basic_interpret`'s default interpreted the bubble
  `Spider(z, 0, 0)` as `id_I`** ([#284]). Its `Spider` arm recursed into
  `special_frobenius_morphism(0, 0, z)`, which returns the **simplified** term —
  `two_layer_simplify`'s rule 3, the *extra-special* axiom, has already cancelled
  the `η;ε` — and `interpret_frob`ed the emptied result to `Self::identity(&[])`.
  This is the same soundness break fixed in `generator_to_cospan` — see the
  final bullet under "Known discrepancy — scalars (bubbles)" below — on the
  other side of the twin: with only the `Cospan` side repaired, the two
  disagreed at exactly this generator, falsifying `basic_interpret`'s own
  "the two must agree generator-for-generator", and any
  *special-but-not-extra-special* implementor inheriting the default (the
  reference semantics, as the braiding bullet notes) got `id_I` for a
  non-identity `0 → 0` scalar. The default now builds the bubble directly as
  `interpret_unit(z) ; interpret_counit(z)`; every other spider arity still
  recurses.

  Pinned by
  `frobenius::trait_impl::tests::basic_interpret_default_spider_zero_zero_is_the_bubble`.
  The pin needs a **`Cospan`-backed** probe implementor (`CospanBacked`), added
  in that test module beside the existing `Defaulting`: a `FrobeniusMorphism`-
  backed implementor is structurally incapable of observing the fix, because its
  carrier quotients by rule 3 and so identifies the bubble with `id_I`. Measured
  by reverting the new arm — each assertion falsified separately: apex 0 vs 1,
  scalars 0 vs 1, and both canonical-form equalities (`classes: []` vs
  `[ApexClass { label: 'a', … }]`) against the hand-built `η;ε` and against
  `frobenius_to_cospan`. Space: one generator, one label, one carrier.

- **`Frobenius::interpret_frob`'s default rejected `identity(&vec![])`**
  ([#284]). It errored on *every* block-free layer, including the block-free,
  empty-interface layer that is how `FrobeniusMorphism::identity` represents the
  identity on the empty type list. It now interprets that layer as `id_I` (the
  unit of its fold) and rejects only a block-free layer with a non-empty
  interface, matching `frobenius_to_cospan`. The two functions now cross-
  reference each other, with the reason they are not one function (`Cospan` is
  `Composable`, not the `ComposableMutating` the `Frobenius` supertraits
  require).

- **`compact_closed`'s module docs stated both zigzag identities in the wrong
  composition order** ([#284]) — `(id_X ⊗ cap_X) ; (cup_X ⊗ id_X)`, which does
  not typecheck as `id_X` (it is an endomorphism of `X ⊗ X ⊗ X`). Corrected to
  `(cup_X ⊗ id_X) ; (id_X ⊗ cap_X)` and `(id_X ⊗ cup_X) ; (cap_X ⊗ id_X)`, the
  composites `zigzag_snakes_reduce_to_the_identity` actually builds.

### Fixed — tests

- **Nothing measured that the crate's two `frobenius_to_cospan` implementations
  agreed** ([#336]). The G1 merge left `frobenius::frobenius_to_cospan` (#283)
  and `cospan_algebra::frobenius_to_cospan` (#284) both public and both
  computing the Prop 3.8 semantics map; each branch was reviewed against
  `96cfea7`, so no reviewer saw the other, and the only comparison on record was
  a 19-sample throwaway probe. `frobenius::to_cospan_pin` (a `#[cfg(test)]`
  module — the T1 algorithm walks the `pub(crate)` `layers`, so no integration
  test can express it) measured the two up to `canonical_form` over **363
  terms** before either body was removed (widened to 383 in the same PR, where
  the retired algorithm is now the oracle), and keeps measuring the survivor
  against that retired algorithm — independent on the spider route and the
  layer fold; its six generator arms, the hand-built braiding literal
  included, are byte-identical to the survivor's, so a convention error
  applied to both copies is invisible here and is held by
  `frobenius_to_cospan_agrees_with_the_cospan_generators` instead. The space:
  the ten `tests/compact_closed.rs::samples()`, the thirty-six
  `(m, n) ≤ 5` spiders including the `(0, 0)` bubble, both sides of all eleven
  Def 2.5 equations, fifteen cup / cap / name / unname terms, and 300
  pseudo-random terms of **up to 8 extension attempts** over two labels (an
  attempt whose generator fits nowhere is skipped, so the terms are shorter than
  the attempt count). The grid reaches `m = 5` because the survivor's spider arm
  recurses into `special_frobenius_morphism`, whose doubling branch is reachable
  only at even `m >= 4` — at `m <= 3` the one genuinely independent route was
  never exercised. Falsified three ways on the survivor — dropping the
  `Spider(z, 0, 0)` carve-out reddens 48 of 383, a *disconnected*
  comultiplication reddens 169 of 383, and an ill-typed braiding reddens the fold
  outright. The **space** is falsified separately, since agreement over 383
  identities would be agreement about nothing: short-circuiting the random-term
  generator leaves the differential assertion green, and a diversity floor beside
  the size assert is what reddens (7 distinct canonical forms over the random
  terms against 172 measured; 209 over all 383). One instantiation
  (`char`/`String`), and a differential claim only: both sides fold with the same
  `Cospan::compose`, so a bug in the pushout moves them together.

- **The `compact_closed` suite asserted only interfaces** ([#284]). Audit
  phase 1 measured that replacing `unname` with discard-inputs/create-outputs
  junk left 44/44 green, and that a `compose_names_direct` discarding `f̂` and
  `ĝ` for bare units did too; `assert_compose_names_equivalent`'s doc promised a
  structural cross-check its body never performed (it compared codomains). The
  suite now carries content pins routed through `frobenius_to_cospan` +
  `canonical_form`: cup/cap against a hand-built bent identity, both Eq. (13)
  snakes against `id_X`, `name`/`unname` against leg-bending computed directly on
  the cospan, Prop 3.3 against `name(f;g)`, Prop 3.4's explicit-comp helper
  against `f` itself, and the `id ⊗ cap ⊗ id` comp factor against
  `equivalence::comp_cospan`, an independently written implementation. Each was
  falsified by reverting the corresponding production line.

### Known discrepancy — scalars (bubbles)

- **Scalars are not preserved across the `Cospan` ↔ `FrobeniusMorphism`
  translation**: `cospan_to_frobenius` drops them, and `frobenius_to_cospan`
  keeps the `Spider(z, 0, 0)` bubble (see "Fixed" below) but never sees a
  spelled `η;ε`, which `two_layer_simplify` rule 3 cancels before the function
  runs ([#284]; pinned as-is by
  `cospan_algebra::tests::scalar_bubbles_are_lost_in_both_directions`, not
  endorsed). When first recorded the loss had **two** causes; the first was
  closed in this same release by [#285] (the "Fixed" entry below), and on the
  merged tree the pin is about the second alone:
  1. ~~`cospan_to_frobenius`'s identity fast path~~ — **closed by #285.** Its
     guard was `domain == codomain && left_to_middle() == right_to_middle()`,
     about the *legs*, not the arity, so it returned `identity(&domain)` and
     discarded every apex vertex neither leg reaches, at any arity. The guard
     now also requires the common leg to be a bijection onto the apex, which
     reaches every apex vertex by construction; a bubble makes
     `leg.len() < middle_len`, so `0 → 0` with any bubble falls through to the
     decomposition path like everything else. Measured on the merged tree:
     disabling the fast path entirely leaves every `cospan_algebra`,
     `equivalence` and `hypergraph_functor` test green with byte-identical
     results — it has no observable effect on this discrepancy any more.
  2. `FrobeniusLayer::two_layer_simplify` rule 3 cancels `η(z);ε(z)` — the
     **extra-special** axiom. `Cospan` is the theory of *special* commutative
     Frobenius monoids and keeps bubbles (`cospan_canon`'s module docs say so
     explicitly); `Corel` is the extra-special quotient that discards them.
     **The sole remaining cause.** The decomposition path emits one `η;ε` per
     unreached apex vertex and rule 3 eats each of them — at `0 → 0` and
     beside `id_a` alike.

  Measured on the merged tree, rule 3 disabled and nothing else: the pin goes
  RED at its first assertion (one bubble and two bubbles become distinct
  depth-2 terms), and `Cospan::new(vec![0], vec![0], vec!['a', 'b'])` — `id_a`
  beside a bubble — comes back as a depth-2 term that keeps its bubble rather
  than `identity(['a'])`. So the pin signals rule 3, and there is no longer a
  cause-1 half for it to miss. An earlier revision of this entry recorded,
  correctly for the pre-#285 tree, that "the pin signals cause 2 only" and
  called the `id_a`-beside-a-bubble assertion "the both-causes signal"; both
  statements are superseded by the guard change.

- **Fixed, and it was a soundness break, not only a scalar loss:**
  `generator_to_cospan`'s `Spider(z, 0, 0)` arm recursed into
  `special_frobenius_morphism`, which returns the **simplified** term — rule 3
  had already emptied it — so the `(0, 0)` spider interpreted to `apex 0`. It
  now builds the bubble `η;ε` directly. Pinned by
  `cospan_algebra::tests::scfm_equal_scalars_have_equal_images`, which was
  measured RED before the change (`Spider(a,0,0)` apex 0 / scalars 0 against
  `η;δ;(ε⊗ε)`'s apex 1 / scalars 1, and the same split beside `id_a`).
  This repairs one witness; it does **not** make the translation sound, since a
  spelled `η;ε` still loses its bubble to rule 3 while the SCFM-equal spider now
  keeps one.
### Added

- **`frobenius::frobenius_to_cospan`** — interpret a `FrobeniusMorphism` as the
  `Cospan` it denotes, the semantics half of the Prop 3.8 correspondence whose
  syntax half `cospan_algebra::cospan_to_frobenius` already covered
  ([#283](https://github.com/sustia-llc/catgraph/issues/283)). Each layer is the
  monoidal product of its blocks' generator cospans and the morphism is the
  pushout composite of its layers; an `UnSpecifiedBox` denotes nothing and is
  ~~rejected with a `CatgraphError::Composition` naming its arities~~ —
  **superseded within this same release by
  [#336](https://github.com/sustia-llc/catgraph/issues/336)**: this path is now a
  re-export of `cospan_algebra::frobenius_to_cospan` and rejects with
  `CatgraphError::Interpret`, whose merged message names both the generator and
  the arities. See the "Changed — BREAKING" entry at the top of `[Unreleased]`.

  This exists because `FrobeniusMorphism`'s `PartialEq` compares
  *presentations*: it separates both sides of **all eleven** Def 2.5 equations
  (measured, and pinned as a count in
  `tests/frobenius_axioms.rs::frobenius_structural_equality_decides_nothing_here`),
  so nothing in the crate could decide a Frobenius equation on that carrier.
  Composing `f` then `g` in `FrobeniusMorphism` and interpreting is *not* the
  same as interpreting each and composing in `Cospan` when the normalizer's
  unit/counit rule deletes an `η` feeding an `ε` — the extra-special axiom,
  which `Cospan` keeps as a bubble. Both halves are pinned. That is the only
  *known* divergence, not a proof there is one; see the Rule 4 fix below, which
  was a second one.

- **`tests/frobenius_axioms.rs`** (16 tests) — the Def 2.5 equations built on
  both sides and decided, for the four carriers this crate defines (#283).
  Riders for the zigzag identities and for the braiding being a genuine
  crossing: a disconnected cup and an identity braiding each leave every one of
  the nine equations intact.

  Four *named* carriers over **three** decision paths, and the file now says so:
  `Corel` is a transparent newtype whose row recomputes `Cospan`'s
  (`corel_recomputes_the_cospan_battery` measures it; the `Corel`-specific claim
  is `corel_battery_composites_stay_jointly_surjective`). Nor is it every
  implementor — `catgraph-applied`'s `PetriNet` and `DecoratedCospan` also
  implement `HypergraphCategory` and have no Def 2.5 pin anywhere.

- **`Debug` on `FrobeniusMorphism` and `FrobeniusOperation`** (#283). `PartialEq`
  on `FrobeniusMorphism` compares presentations, so a failed comparison was only
  ever reportable as `depth()`; the derive makes the whole presentation
  printable. Purely additive.

- **`tests/corel.rs::composites_induce_the_expected_partition`** — `Corel`
  composites are pinned by their whole class structure, not only by the joint
  surjectivity `Corel::new` checks and every wrong composite also satisfies (#283).

### Fixed

- **The Frobenius normalizer no longer fuses two spiders that share no wire**
  (#283). `two_layer_simplify`'s Rule 4 matched `Spider(z, m, n)` against
  `Spider(z, n, k)` with no lower bound on `n`, so `Spider(z, 2, 0)` followed by
  `Spider(z, 0, 2)` — a sink and a source, two *disconnected* components —
  collapsed into `Spider(z, 2, 2)`. Measured: the presented composite then
  interpreted to one apex class where the semantics has two, i.e. the sink's
  inputs and the source's outputs were wired together. Reachable from the public
  API (`FrobeniusOperation` and its `From` impl are both `pub`), and the whole
  workspace was green with it present. Rule 4 now requires `n >= 1`, and the
  `target_side_placement` lookup excludes zero-output blocks. ⚠ **The pin,
  `tests/frobenius_axioms.rs::spider_fusion_needs_a_wire_between_the_two_spiders`,
  covers the CONJUNCTION, not either half** — measured: deleting `&& *n1 > 0`
  alone leaves `cargo test -p catgraph` fully green, deleting the lookup filter
  alone is likewise green, and only removing both turns the pin red. The two
  are redundant defenses against one defect; cargo-mutants will score either
  single deletion as MISSED, correctly.

  Same function, same root cause: the `target_side_placement → block` lookup
  keyed zero-output blocks too, and those do not advance the placement, so
  they shared a key with the emitting block at the same placement.
  `HashMap::insert` keeps the last writer and blocks are visited in layer
  order, so the emitting block always won — the collision never displaced one;
  what the filter removes is the lookup at a placement with no emitting block
  (the trailing one, where only a zero-input next block can sit), and of those
  pairings only a spider pair `Spider(z, m, 0) ; Spider(w, 0, k)` gets past a
  rule's patterns (Rule 4's): `z1 == z2` stops it when `z ≠ w`, and when
  `z == w` only the `n >= 1` guard does. So the filter's only observable
  effect is that `n == 0` spider case — which is why it is redundant with the
  guard. Blocks with no outputs are now excluded, which also makes the
  remaining keys strictly increasing and therefore unique.

### Changed

- **`CospanAlgebraMorphism`'s `Clone` is hand-written and no longer requires
  `A: Clone`** (#283). The algebra is held behind an `Arc`, so cloning never
  clones it; the derived bound put every zero-sized algebra —
  `PartitionAlgebra` included — outside `Clone`. Strictly a loosening: every
  call that compiled before still compiles.

- **The Thm 3.14 freeness claims in `hypergraph_category.rs` are scoped to the
  deferral** ([#277](https://github.com/sustia-llc/catgraph/issues/277)). The
  trait doc and the impl banner asserted "the free hypergraph category" bare,
  while `docs/FS19-AUDIT.md` marks Thm 3.14 ❌ DEFERRED and nothing constructs
  the `Set ⇄ Hyp` adjunction. The module doc's own caveat also cited
  [#79](https://github.com/sustia-llc/catgraph/issues/79), closed since
  2026-07-27; it now cites the audit doc. Prose only.

- **The arity-only Frobenius tests are renamed to say so** (#283):
  `unitality_left` → `unitality_left_arities` and similarly for
  `counitality_left`, `associativity`, `frobenius_law` (→
  `frobenius_law_lhs_arities`, which builds one side of a different equation),
  `special_frobenius`, `zigzag_via_trait` and `frobenius_morphism_special` in
  `src/hypergraph_category.rs`, plus the H_Part trio in `tests/equivalence.rs`.
  Test names only; no API change.

  Each half needed its own mutant, because they exercise different code. All
  **19** tests in the src mod stayed green under a non-merging μ together with a
  non-splitting δ on `Cospan`'s `HypergraphCategory` impl — which the new battery
  catches on three carriers. The H_Part trio never touches that impl (it builds
  from `PartMorph::multiplication_in`/`comultiplication_in`), so its mutant is
  `multiplication_in`'s right leg `[0, 0, 0]` → `[0, 1, 0]`: the trio stays
  green, while `cospan_algebra_morphism_battery`, the bubble ledger and the
  zigzag rider all go red.

### Fixed

- **`cospan_algebra::cospan_to_frobenius` no longer collapses an all-merged
  cospan to the identity**
  ([#285](https://github.com/sustia-llc/catgraph/issues/285)). Its identity
  fast path fired on `domain == codomain && left_leg == right_leg`, which the
  single-apex cospan `m → {•} ← m` also satisfies (both legs are `[0; m]`) even
  though it is the `m→n` spider, not the identity. Measured: `[a,a] → {•} ←
  [a,a]` returned `identity` (depth 1) where the correct answer is
  `special_frobenius_morphism(2, 2, 'a')` (depth 2); `[a,a,a] → {•} ← [a,a,a]`
  returned depth 1 against the correct depth 4. The wrong answers were
  reachable for every `m = n ≥ 2` with a non-injective common leg; the guard
  now additionally requires the common leg to be a **bijection** onto the
  apex. Cospans whose apex carries a node no leg hits (`[a] → {•,•} ← [a]`)
  also satisfied the old guard, but their *output is unchanged*: they now take
  the general route, which emits `η;ε` for the spare node, and
  `two_layer_simplify` rule 3 cancels it, so the answer is still
  `identity(['a'])` — only the code path moved. Whether that scalar should
  survive is the extra-special question the "Known discrepancy" entry above
  leaves undecided pending rule 3. Every cospan the fast path used to answer
  correctly — the genuine identities, permuted ones such as
  `[1,0] → {a,b} ← [1,0]` included — still short-circuits.

  Behaviour change, not an API change: `cospan_to_frobenius`,
  `CospanToFrobeniusFunctor::map_mor`, and `NameAlgebra::map_cospan` all return
  different (correct) morphisms for the affected cospans. Callers comparing
  `FrobeniusMorphism` presentations against previously-recorded values for
  those inputs will see a difference.

### Added

- Content-level regression pins for the `Cospan → Frobenius` functor ([#285]).
  The `ctf_*` suite compared only `domain()` / `codomain()`, so it could not
  see connectivity: an implementation mapping every `m → n` cospan to
  `special_frobenius_morphism(m, n, z)` (label read off the cospan) passed
  every uniform-label test in it — measured against the old file, 7 of the 10
  `ctf_*` tests stayed green and only the three with mixed boundary labels went
  red, for the labels rather than the wiring. New and strengthened tests
  compare whole morphisms via `FrobeniusMorphism: PartialEq`:
  `ctf_single_apex_cospan_is_the_spider` (the 5×5 grid `(m,n) ∈ {0,…,4}²`),
  `ctf_disconnected_cospan_is_the_tensor_not_a_spider` (including a
  uniform-label witness whose boundary is byte-identical to the spider it must
  not equal), `ctf_functoriality_composition_content`, and content assertions
  on `F(id)`, `F(η)`, `F(ε)`, `F(μ)`, `F(δ)`. Because
  `special_frobenius_morphism` is not independent of the code under test
  (`from_decomposition` builds each surjection block with it),
  `ctf_single_apex_cospan_round_trips_up_to_canonical_form` round-trips the 24
  non-bubble grid cells through `frobenius_to_cospan` + `canonical_form` and
  asserts the `(0,0)` bubble loss separately.
- Leg-by-leg Prop 4.6 and Lemma 4.9 witnesses ([#285]). The Prop 4.6 proptests
  ran on a uniform-label generator with an injective leg, under which a wrong
  leg is invisible; `arb_mixed_part_element` now emits two apex classes sharing
  the label `'a'`, so `right_to_middle` carries information the codomain does
  not. The Lemma 4.9 witnesses moved off identity morphisms onto `μ`, `δ` and
  `μ ; δ`.
- `tests/common::assert_frobenius_eq_msg` / `frobenius_shape` — `FrobeniusMorphism`
  has `PartialEq` but no `Debug`, so `assert_eq!` is unavailable; these report
  `depth`/`domain`/`codomain` on failure, and the assertion also renders each
  side's `frobenius_to_cospan(..).canonical_form()`, since the three shape
  fields cannot tell a connectivity-only regression from a fixture drift.

## [workspace-v0.15.0] - 2026-08-16

### Changed — BREAKING

- **`SymmetricMonoidalMorphism::from_permutation(p, types, types_as_on_domain: bool)`
  is replaced by two named methods**
  ([#258](https://github.com/sustia-llc/catgraph/issues/258)):
  `from_permutation_on_domain(p, types)` and
  `from_permutation_on_codomain(p, types)`. The `bool` leaves the trait
  entirely.

  Both build the same wiring — domain wire `i` to codomain wire `p.apply(i)` —
  and differ only in which boundary `types` labels: `on_domain` gives
  `domain() == types` and `codomain()[k] == types[p.inv().apply(k)]`;
  `on_codomain` gives `codomain() == types` and `domain()[i] == types[p.apply(i)]`.

  Migration is mechanical: `from_permutation(p, t, true)` →
  `from_permutation_on_domain(p, t)`, `from_permutation(p, t, false)` →
  `from_permutation_on_codomain(p, t)`.

  The reason for two names rather than a documented flag is that a `bool` is a
  parameter on which an implementation can be silently inverted. Three of the
  workspace's implementations had drifted onto three different conventions, and
  no test could see it: within one carrier an inverted braiding is still a
  perfectly consistent braiding, and nothing exercised one permutation across
  two carriers. Two names make the wrong direction unrepresentable at the call
  site, and force every implementation to state which direction it realizes.

- **`SymmetricMonoidalDiscreteMorphism::from_permutation` loses its `bool` and
  keeps one constructor** ([#258]). Its object is a bare cardinality, so there
  is no label to place on either boundary and the sibling trait's two
  constructors would collapse to the same function of the same two arguments.
  `Decomposition::from_permutation(p, n)` is the whole surface.

- **`Span::from_permutation_on_codomain` realizes `p`, where the old
  `types_as_on_domain = false` branch realized `p⁻¹`** ([#258]). This is a
  behaviour change, not a rename. The old body built `left: p.inv().permute(types)`
  with apex pairs `(p.apply(idx), idx)`, wiring domain `j` to codomain
  `p.inv().apply(j)` — disagreeing with `Cospan` on both the wiring and the
  domain object for every non-involutive `p`. On `rotation_left(3, 1)` over
  `['a','b','c']` it reported a domain of `['c','a','b']` where `Cospan` reports
  `['b','c','a']`. The `types_as_on_domain = true` branch was always correct.
  The two in-crate tests covering the wrong branch asserted only `codomain()`,
  `domain().len()` and label agreement across the apex, every one of which an
  inverted wiring satisfies; they now pin the full contract.

  The cached identity flags moved with the wiring and for the same reason —
  they describe the apex leg maps, not the label vectors — so
  `is_left_identity()` / `is_right_identity()` on the result are swapped
  relative to the old branch.

- **`CospanAlgebraMorphism`'s permutation constructors were not braidings at
  all, and now are** ([#258]). Two independent defects, neither observable
  before this release because nothing in the workspace exercised this impl:
  it was reachable only through the generic trait, and every caller of the
  trait used a cospan or matrix carrier.

  1. The labels were inverted: the non-`types` side was `p.permute(types)`
     where the cospan family uses `p.inv().permute(types)`.
  2. The structural cospan was built over a `2n`-vertex apex (`domain ++
     codomain`) with a *bijective* right leg, so no domain wire shared an apex
     vertex with any codomain wire and the element was the all-singletons
     partition. A braiding must merge domain wire `i` with codomain wire
     `p.apply(i)` on one vertex, as `identity_in` already does with
     `(0..n) ++ (0..n)` over an `n`-vertex apex.

  The clinching evidence for (2) is that
  `from_permutation(Permutation::identity(n), ..)` did not equal `identity(..)`,
  which no symmetric monoidal category permits. Both are fixed; the apex is now
  `n` vertices and the right leg degenerates to `identity_in`'s exactly when `p`
  is the identity.

- **`NamedCospan::from_permutation_extra_data` splits the same way and is now
  fallible**: `from_permutation_extra_data_on_domain` /
  `..._on_codomain`, each returning `Result<Self, CatgraphError>` ([#258]).
  The old body reached `Cospan::from_permutation` through an `.unwrap()` in
  production code. There is no precondition making that unreachable — `p`,
  `types` and `prenames` are independent caller arguments — so it propagates
  with `?` rather than asserting an invariant it does not have. The
  `assert_eq!(types.len(), prenames.len())` became a
  `CatgraphError::CompositionSizeMismatch` for the same reason.

- **Arity mismatch is an `Err` on every permutation constructor, not a panic**
  ([#258]). The trait's `# Errors` clause always promised this, and no
  implementation honoured it: `Cospan` used `assert_eq!`, while `Span` and
  `FrobeniusMorphism` had no check at all and indexed out of bounds inside the
  `permutations` crate. `Corel`, `DecoratedCospan`, `PetriNet` and
  `NamedCospan` inherit the fix by delegation.

- **`NamedCospan`'s `SymmetricMonoidalMorphism` constructors still fail
  unconditionally**, now naming the matching replacement
  (`from_permutation_extra_data_on_domain` / `..._on_codomain`) rather than the
  retired single method ([#258]). Port names are not derivable from `types`, so
  there is no honest value to return; the split does not make it satisfiable.

- **`permute_side` splices `β(p)` on the codomain and `β(p⁻¹)` on the domain on
  every carrier; `Cospan`, `Corel`, `NamedCospan`, `Span`, `FrobeniusMorphism`,
  `CospanAlgebraMorphism` and `Decomposition` all realized the inverse before**
  ([#258]). The constructors' split left `permute_side` — the other method of
  the same trait — carrying exactly the defect the split removed, and the
  measurement is one line:

  | carrier | `identity(types).permute_side(rotation_left(3,1), true)` |
  |---|---|
  | `Cospan` / `Corel` / `NamedCospan` / `DecoratedCospan` (before) | `[2, 0, 1]` = `β(p⁻¹)` |
  | `FrobeniusMorphism` (before) | `β(p⁻¹)` |
  | `MatR` / `MatKron` / `PropExpr` (unchanged) | `[1, 2, 0]` = `β(p)` |

  The contract is now stated once, on the trait's `permute_side` rustdoc where
  every implementor sees it, in the same shape the constructors already use:
  **the wire at slot `i` of the permuted side moves to slot `p.apply(i)`**, i.e.
  `self ; from_permutation_on_domain(p, &self.codomain())` on the codomain and
  `from_permutation_on_codomain(p.inv(), &self.domain()) ; self` on the domain.
  The matrix carriers were already right and did not move.

  ⚠ **The two sides are not symmetric, and the domain side is the half that is
  easy to get wrong.** Pre-composition puts the braiding's *codomain* against
  `self`, so the braiding routes the **new** domain slot `j` to the **old** slot
  `β(j)` — which forces `β = p⁻¹`. The check that sees a swap here is
  conjugation: `β(p⁻¹) ; id ; β(p) == β(p⁻¹ ; p) == id`, where the symmetric
  reading gives `β(p²)`.

  **Migration.** Callers that were passing `p` and want the old behaviour pass
  `p.inv()`. Callers pairing `permute_side` with
  `catgraph::utils::necessary_permutation` need `p.inv()`, because that helper
  answers "which old slot supplies each new slot" while `permute_side` asks
  "where does the wire at slot `i` go" — `catgraph-applied`'s
  `WiringDiagram::operadic_substitution` is the one production call site in the
  workspace and was adapted this way. Single-sorted carriers (`MatR`,
  `MatKron`, `PropExpr`) are unaffected either way.

  On `Decomposition` (the discrete trait) the same change reads `p ∘ f` on the
  codomain and `f ∘ p⁻¹` on the domain, where it used to be `p⁻¹ ∘ f` and
  `f ∘ p`. Its own test doc said it was "matching `Cospan::permute_side`
  semantics", which is why it moved with `Cospan` rather than being left
  behind — the design record for #258 warned that the split had to cover both
  traits "or the same foot-gun survives one trait over".

  `PetriNet::permute_side` is **not** part of this change: it permutes
  `self.transitions`, so its `p` is sized by the transition count rather than by
  a boundary arity, and it is documented as such.

### Fixed

- **`Span::permute_side` was incoherent with itself and produced an invalid
  span** ([#258]). A `Span` keeps its wiring in `middle` (apex pairs) and its
  labels in `left`/`right` (words) — the opposite shape from `Cospan`, whose
  legs *are* the wiring. The body moved each apex pair by `p.apply` **and**
  permuted the word by `p`; those two must be mutually inverse. So
  `Span::identity(&['a','b','c']).permute_side(&rotation_left(3,1), true)` gave
  domain `['a','b','c']`, codomain `['b','c','a']` and pairs
  `[(0,1),(1,2),(2,0)]` — wiring domain `'a'` to a codomain slot labelled `'c'`,
  which `Span::assert_valid` rejects ("left and right linked … but their lambda
  types didn't match").

  ⚠ This was **release-silent**: `assert_valid` is `debug_assert!`-only since
  workstream A, so a release build accepted the malformed span. The word is now
  permuted by `p.inv()`, which both restores the invariant and lands `Span` on
  the contract above.

- **`CospanAlgebraMorphism::permute_side` left `self.element` stale** ([#258]).
  It permuted the two label words and nothing else. The element is the morphism
  — it is what carries the wiring — so after `permute_side(p, true)` the value
  advertised a permuted codomain word while still pairing domain wire `i` with
  the *old* codomain slot, and `compose` then fed that stale element through
  `comp_cospan` built from the *new* labels and produced a wrong composite. It
  now pushes the element through the relabelling cospan
  `(X ⊕ Y) → (X' ⊕ Y')` — apex `X ⊕ Y`, identity left leg — exactly as
  `Monoidal::monoidal` uses an interchange cospan, and gains the same defensive
  length no-op `MatR` and `PropExpr` have.

- **`CospanAlgebraMorphism`'s permutation constructor was O(n²)** ([#258]).
  `p.inv()` was already hoisted; the cost was `p_inv.permute(&(0..n).collect())`
  *inside* the per-index closure — a `Vec` allocation and a full permute for
  every index, to read one element out of the result. Replaced by a direct
  `p_inv.apply(k)`, making the constructor O(n).

  (An earlier draft of this entry blamed a missing hoist of `p.inv()`. That was
  wrong — the hoist was already there on `main`, and the expression it quoted as
  the defect was the *new* code. Corrected here rather than silently, because a
  maintainer following the old wording would go hunting a bug that never
  existed.)

- **A documentation claim that inverted the whole convention** ([#258]).
  `PropExpr::from_permutation`'s rustdoc stated that generic code calling
  `M::from_permutation(p, types, true)` got "`p` on `PropExpr`/`MatR` and `p⁻¹`
  on `Cospan`/`Corel`". That is false. It read the `p.inv()` in `Cospan`'s
  builder as the realized permutation, but that `p.inv()` is the cospan's right
  *leg index vector*; connectivity runs through the apex, so domain `i` meets
  codomain `k` when `left[i] == right[k]`, i.e. when `k == p.apply(i)`. The
  inversion in the leg is what keeps the wiring un-inverted. Every carrier in
  the workspace realizes `p`. The claim is corrected in place and the
  correction recorded, because it is an easy mistake to re-make.

### Added

- **A cross-carrier braiding test** (`catgraph-applied/tests/braiding_cross_carrier.rs`,
  [#258]) — the deliverable the issue asks for and the thing whose absence let
  three conventions coexist. It drives every permutation of `n = 3` and `n = 4`
  through both constructors of every implementation of both traits, plus
  `Decomposition` via `from_decomposition`. No assertion compares two carriers
  to each other (a symmetric drift would keep that green); each is compared
  against a reference computed directly from `p`, and that reference is itself
  pinned against hand-written values — the cospan's legs and labels, and
  `MatR`'s entries written out in full.

- **`permute_side` coverage in the same file** ([#258]) — its absence is why the
  inverted convention survived the constructor sweep. Four tests, same
  discipline: a hand anchor (`Cospan` legs, `Span` pairs, `MatR` entries,
  `Decomposition`'s permutation part, all written out for
  `rotation_left(3, 1)`); `permute_side` on an identity matched against the
  constructor it must equal, on both sides, for every permutation of `n = 3` and
  `n = 4`; the composite law `β(q).permute_side(p, true) == β(q ; p)` over every
  *ordered pair* of permutations at both arities, which the identity sweep
  cannot see; and the conjugation law, which is the only assertion that
  separates the domain rule from the codomain rule.

  `NamedCospan` is included — the risk there is names desynchronising from
  legs — and `PetriNet` is excluded, for the reason its constructors are
  ([#272](https://github.com/sustia-llc/catgraph/issues/272)).

## [workspace-v0.14.0] - 2026-08-16

### Fixed — from the #256/#261 code review

- **`CospanCanon::from_parts` no longer sizes its occurrence tally from an
  unvalidated `dom_len`/`cod_len`.** `from_parts` is the *reload* constructor,
  so those lengths arrive from a store or a wire format and are untrusted. A
  corrupt `dom_len` of `1 << 40` satisfied the sortedness and ascending checks
  with a single in-range class and then allocated an 8 TB tally — an allocation
  abort, not a catchable `Err`. A cardinality comparison
  (`sum(preimage.len()) == boundary_len`) decides the same question without
  allocating and now runs first, reporting the new
  `CatgraphError::CanonPreimageCardinalityMismatch`.
  `CanonPreimageNotAPartition` remains the finer report for the case the
  cardinality check cannot catch — the right *number* of members distributed
  wrongly — and is pinned by two new fixtures so it stays reachable.
- **`Span::new` reports `CatgraphError::ConstructionMiddlePairOutOfBounds`
  instead of reusing `ConstructionIndexOutOfBounds`.** The two describe
  different elements: a cospan's legs are vectors of indices, so
  `(leg, position)` locates the bad entry inside the named leg; a span's legs
  are derived from one shared middle-pair list, so the bad element is a *pair*.
  Reusing the cospan variant mis-located it for any consumer using the fields
  as documented, and emitted the same `position` for the domain and codomain
  failures of a single pair.
- **CI now runs `cargo test -p catgraph --lib --release`.** The three
  `#[cfg(not(debug_assertions))]` tests that pin this release's central claim —
  that `new_unchecked` is `debug_assert!`-only and so accepts what `new`
  refuses — are compiled out of the debug workspace run, and the only release
  job covered `catgraph-magnitude`. Without this lane the `Span::assert_valid`
  bug fixed below would have shipped green.

### Changed — BREAKING

- **`Cospan::new` and `Span::new` are now validated constructors returning
  `Result<Self, CatgraphError>`; the previous infallible bodies moved to
  `Cospan::new_unchecked` / `Span::new_unchecked`**
  ([#256](https://github.com/sustia-llc/catgraph/issues/256)). Until now the
  structural invariants were checked by `debug_assert!` only, so a **release**
  build accepted a cospan whose leg pointed outside its apex and deferred the
  failure to whatever indexed it later — a composition, a downstream store's
  canonicalisation, or nothing at all. `new` now performs those same checks
  unconditionally, in every profile:
  - `Cospan::new` — every `left` and every `right` entry must be `< middle.len()`.
  - `Span::new` — every middle pair's `.0` must be `< left.len()` and its `.1`
    `< right.len()`, **and** the two labels it names must agree
    (`left[pair.0] == right[pair.1]`).

  This is the crate's existing convention, not a new one: `Rel::new` /
  `Rel::new_unchecked` and `Corel::new` / `Corel::new_unchecked` already split
  this way, with the **checked** constructor owning the plain name. Shipping a
  `try_new` beside an infallible `new` would have left two opposite idioms for
  the same thing on types one layer apart.

  **Migration.** Data that is correct *by construction* — composition results,
  `identity`/`unit`/`counit`/`multiplication`/`comultiplication`, permutation
  builders, monoidal products, anything rebuilt from an already-valid value —
  should call `new_unchecked`, which is exactly the old `new` and costs nothing
  in release. Reserve `new` for data crossing a trust boundary: a store, a wire
  format, a parser, a user. In tests, `.unwrap()` is the expected fix.

- **`NamedCospan::new` is now a validated constructor returning
  `Result<Self, CatgraphError>`; the previous infallible body moved to the new
  `NamedCospan::new_unchecked`**
  ([#256](https://github.com/sustia-llc/catgraph/issues/256)). This was the last
  public constructor in core that accepted a leg map pointing outside its apex
  in a **release** build: it took raw `Vec<MiddleIndex>` legs — a trust boundary
  by the same criterion as `Cospan::new` — but built its inner cospan with
  `Cospan::new_unchecked`. `new` now validates both structural invariants
  unconditionally, in every profile:
  - **one name per port** — `left_names.len() == left.len()` and
    `right_names.len() == right.len()`. These were `assert!`s, so they aborted
    the process in every profile; they are now errors. A constructor that
    returns `Result` for one precondition and panics for another is only
    half-checked, and a name-count mismatch is precisely the corruption a
    caller reconstructing a named cospan from stored columns hits.
  - **leg bounds** — delegated to `Cospan::new` rather than re-implemented, so
    the check has one home and one error shape.

  The name counts are checked before the leg bounds, and the domain side before
  the codomain side, so the reported failure is the first one in that order.

  **Migration.** Same rule as `Cospan`/`Span` above: `new_unchecked` for data
  correct by construction, `new` for data crossing a trust boundary, `.unwrap()`
  in tests. `NamedCospan::empty` moved to `new_unchecked` and is unchanged
  behaviourally. Note `new_unchecked` is *uniformly* `debug_assert!`-only —
  including the name counts, which the old `new` enforced with a hard `assert!`.
  Keeping a release panic for one of its two invariants would have made the
  `_unchecked` suffix mean two different things on one constructor and broken
  the zero-release-cost contract `Cospan::new_unchecked` documents.

- **`CatgraphError` is `#[non_exhaustive]`.** Downstream `match`es must carry a
  wildcard arm; after this release a new variant is no longer a breaking
  change. Core was the last crate in the workspace whose error enum was not
  marked — `catgraph-syntax::SyntaxError` and `catgraph-dl::DepthError` already
  were.

### Added

- **`CatgraphError::ConstructionIndexOutOfBounds { leg, position, target,
  target_len }`** — a **cospan** leg entry targets an index outside the apex it
  must land in. Raised by `Cospan::new`, and through it by `NamedCospan::new`.
  `leg` is the new `errors::BoundaryLeg` (`Domain` / `Codomain`, rendered as
  `domain`/`codomain`), `position` is the entry's index within that leg,
  `target` the out-of-range value, `target_len` the apex size. The vocabulary
  deliberately matches what the downstream `catgraph-surreal` store already
  reports for a corrupt leg, so the store can retire its parallel spelling.
- **`CatgraphError::ConstructionMiddlePairOutOfBounds { leg, pair_position,
  target, target_len }`** — a **span** middle pair names a boundary index
  outside the boundary set it must land in. Raised by `Span::new`.

  ⚠ **A span does not raise `ConstructionIndexOutOfBounds`**, and matching only
  that variant will silently route every out-of-bounds span pair into your
  wildcard arm. The two describe different elements: a cospan's legs are vectors
  of indices, so `(leg, position)` locates the bad entry inside the named leg; a
  span's legs are derived from one shared middle-pair list, so the bad element is
  a *pair* and `pair_position` indexes that list. `leg` still says which half of
  the pair was out of range, and hence which boundary `target_len` measures.
- **`CatgraphError::ConstructionLabelMismatch { position, left_index,
  right_index, left_label, right_label }`** — a `Span` middle pair links a
  domain element to a codomain element carrying a different label. `left_label`
  / `right_label` are the `Debug` renderings of the two labels.
- **`CatgraphError::ConstructionNameCountMismatch { leg, boundary_len,
  name_count }`** — a `NamedCospan` was handed a port-name list whose length
  does not match the boundary it names. `leg` reuses `BoundaryLeg`,
  `boundary_len` is `left.len()` / `right.len()`, `name_count` the number of
  names supplied for it. Raised by `NamedCospan::new`.
- **`errors::BoundaryLeg`** — `Domain` / `Codomain`, with `as_str()` and
  `Display`. Not `#[non_exhaustive]`: a span or cospan has exactly two legs, so
  matching it exhaustively is safe.
- **`CospanCanon` round-trips: `CospanCanon::from_parts`,
  `CospanCanon::to_cospan`, and `ApexClass::new`**
  ([#261](https://github.com/sustia-llc/catgraph/issues/261)). Purely
  **additive** — no existing signature changes. Until now the only way to obtain
  a canonical form was `Cospan::canonical_form()`, so a consumer could read one
  (`classes()`, since #254), persist it, and log it, but never *reload* it: to
  compare against a stored form after a restart it had to keep the originating
  cospan and re-run `canonical_form()`, which is the work the canonical form
  exists to save.
  - `ApexClass::new(label, dom_preimage, cod_preimage) -> Self` — assembles one
    class signature. Infallible and **non-validating**: neither documented
    invariant is a property of a class in isolation.
  - `CospanCanon::from_parts(dom_len, cod_len, classes) -> Result<Self,
    CatgraphError>` — re-establishes all three: `classes` sorted under
    `ApexClass`'s `Ord`, each preimage strictly ascending, and the preimages
    partitioning `0..dom_len` / `0..cod_len` (the "each leg is a function"
    property). `CospanCanon`'s `Eq`/`Hash` decide apex isomorphism *because of*
    those invariants, so a constructor that skipped them would hand back a value
    unequal to the `canonical_form()` of every cospan.
  - `CospanCanon::to_cospan(&self) -> Cospan<Lambda>` — rebuilds a witnessing
    cospan: apex = classes in canonical order, `left[i]` = the class whose
    `dom_preimage` contains `i`, `right[k]` likewise. Scalars (bubbles) are
    placed in the apex although no leg reaches them, so `k` bubbles round-trip
    as `k`. The apex comes back in canonical order, which is generally *not*
    `structurally_equal` to the originating cospan — that difference is
    precisely the apex labelling the form forgets.

  **Rejects, does not repair.** Nothing sorts the input into shape. The intended
  input is reloaded data, where silently repairing a malformed value hides
  corruption exactly where the caller most needs to hear about it — same posture
  as `Cospan::new` above.

  **The round trip is the oracle, not a convenience.** The three invariants are
  not merely necessary but *sufficient* to rebuild a witness, so
  `c.canonical_form().to_cospan().canonical_form() == c.canonical_form()` is a
  real property test for the validation rather than a hand-checked list.
- **Five `CatgraphError` variants for canonical-form construction**, all raised
  by `CospanCanon::from_parts` and all reusing `BoundaryLeg`:
  `CanonClassesNotSorted { position }`;
  `CanonPreimageNotAscending { leg, class_position, position }`;
  `CanonPreimageOutOfBounds { leg, class_position, position, index,
  boundary_len }`;
  `CanonPreimageCardinalityMismatch { leg, total, boundary_len }`; and
  `CanonPreimageNotAPartition { leg, index, occurrences, boundary_len }`, where
  `occurrences` is `0` for an unclaimed boundary index and `>= 2` for one
  claimed by several classes.

  ⚠ **`CanonPreimageCardinalityMismatch` is the commonest failure on a corrupt
  reload**, not `CanonPreimageNotAPartition` — a caller matching only the latter
  will miss both "too few members" and "too many members". The partition tally
  runs only once the totals agree, so it reports the narrower fault: the right
  *number* of preimage members distributed wrongly.

  `CanonPreimageOutOfBounds` is deliberately not `ConstructionIndexOutOfBounds`:
  that one points the other way (a leg entry overshoots the apex), and locating
  this one needs the class's position as well as the position within the
  preimage.

### Fixed

- **`Span::assert_valid` no longer panics in release.** Its label-agreement
  check indexes both boundaries, and the check was computed into a `let`
  *before* being handed to `debug_assert!` — so the indexing ran in every
  profile and an out-of-bounds middle pair produced a bare `index out of
  bounds` panic in release, which is neither what the method's name promises
  nor what `new_unchecked` documents. Both `assert_valid` methods now write
  every check inside its `debug_assert!`, so they compile away entirely in
  release. Debug behaviour is unchanged, including the order in which the
  invariants are reported (bounds before labels, so a debug build still names
  the specific invariant rather than index-panicking).

## [workspace-v0.12.0] - 2026-08-15

### Added

- **`cospan_canon::ApexClass<Λ>` and `CospanCanon::classes`** — a read surface
  for the data that actually discriminates a canonical form
  ([#254](https://github.com/sustia-llc/catgraph/issues/254)). `CospanCanon`
  already derived `Eq`/`Hash` and worked as a key, but its apex signatures were
  private and the accessors stopped at `dom_len`/`cod_len`/`scalar_count`/
  `apex_len`, so a consumer could compare two canonical forms in-process and
  nothing else. `classes()` now returns `&[ApexClass<Λ>]`, and each class
  exposes `label()`, `dom_preimage()`, `cod_preimage()` (both preimages sorted
  ascending — a documented invariant callers may rely on), and `is_scalar()`
  for the both-empty bubble case. A canonical form can therefore be inspected,
  written out, re-encoded into another representation, or logged, not only
  compared. No serde: the core crate carries none, and this is a read surface,
  not a wire format.
  - **Read-only, deliberately: this is not a round trip.** There is no
    `CospanCanon::from_parts` and no `ApexClass::new`, so a form that has been
    written out cannot be read back into a `CospanCanon` — a consumer wanting
    to compare against stored data after a restart still keeps the originating
    `Cospan` and re-runs `canonical_form()`. A public constructor cannot be a
    plain `new`: the type's `Eq`/`Hash` decide cospan isomorphism only because
    the private construction path guarantees sortedness of the classes, of each
    preimage, and that the preimages partition the two boundaries. Whether the
    type should round-trip, and under which of those validations, is
    [#261](https://github.com/sustia-llc/catgraph/issues/261).
  - **The canonical form itself is unchanged** — its `Eq`, its `Hash`, and its
    sort order are all bit-identical to before, so persisted comparisons remain
    valid and this is *not* a format change. `ApexClass` replaces an anonymous
    `(Λ, Vec<usize>, Vec<usize>)` tuple in a **private** field; its fields are
    declared in that same order and it derives the same traits, so the derived
    `Ord` driving the canonicalising `classes.sort()` — the sort that makes the
    value invariant under apex relabelling — is exactly the tuple's
    lexicographic order. A test pins this against an in-test rebuild of the
    tuple vector sorted under tuple `Ord`, and pins the field-by-field
    tie-break directly.
  - `scalar_count` is rewritten as a filter on `is_scalar()`; behaviour,
    signature, and docs are unchanged, as are `CospanCanon`'s derives and its
    four existing accessors.

## [workspace-v0.10.0] - 2026-08-09

### Changed

- **`cospan::test::permutatation_manual` uses a literal payload instead of
  `rand::random()`** ([#232](https://github.com/sustia-llc/catgraph/issues/232)).
  Test-only; no public API or behaviour is affected. The five booleans are
  arbitrary payload the test's own assertions carry through, so entropy bought
  nothing — a visible mixed literal replaces the RNG outright and keeps the
  middle-label comparison discriminating by construction. The trigger:
  `rand::random()` needs `rand`'s `thread_rng` feature, which the slimmed
  workspace declaration no longer enables — in physics-free builds; a build
  graph containing `catgraph-physics` unifies rand's defaults back on (see
  `catgraph-applied`'s #232 entry), which is why the literal, not a feature
  bump, is the right fix. `rand` remains a dev-dependency only, now carrying
  `std`/`std_rng` on its own dev edge since the workspace entry is featureless.

## [workspace-v0.9.0] - 2026-08-04

### Changed

- **`MorphismSystem`'s topological sort is now the crate's own, and the
  `ultragraph` dependency is gone**
  ([#220](https://github.com/sustia-llc/catgraph/issues/220), D2 of the
  [#218](https://github.com/sustia-llc/catgraph/issues/218) dependency
  streamlining). Exactly one thing was consumed from that crate —
  `topological_sort` — behind the acyclicity check in
  `add_definition_composite` and the resolution order in `fill_black_boxes`.
  It is replaced by a private Kahn's-algorithm pass in
  `frobenius/morphism_system.rs`, so the crate now depends on no graph crate at
  all. `union-find` (already present for the cospan/corel pushouts) is unchanged
  and moves to a workspace dependency, now that `catgraph-applied` uses it too.
  - Behaviour is unchanged at both call sites: a cycle still yields
    `CatgraphError::Interpret`, and a valid topological order still places each
    parent before its children. The specific order among equally-valid ones may
    differ from the previous implementation's — it always could, since the label
    ids come from `HashMap` iteration.
  - The private `resolve_order` drops its `Result` wrapper: Kahn's only failure
    mode is a cycle, which the `Option` already reports, so the dead
    "graph construction failed" error path is gone. No public signature changes
    — `ultragraph::GraphError` never appeared in one.
  - `toposort` is covered directly for chains, diamonds, isolated nodes,
    duplicate edges, two-node cycles, self-loops, and a cycle sitting beside an
    acyclic component.

## [workspace-v0.6.0] - 2026-08-02

### Changed

- **`equivalence::comp_cospan`'s index arithmetic is documented as bounded, and
  its one unbounded sum is spelled against that bound**
  ([#196](https://github.com/sustia-llc/catgraph/issues/196)). The four index
  sums the issue's inventory names (`m + n`, `m + n + k`) cannot overflow:
  `middle` is built first, and a `Vec` holding `m + n + k` elements is proof
  that the sum fits. The one sum *not* covered by that argument — the left leg's
  own length `m + 2n + k`, which the inventory does not list — is now written as
  `middle.len().saturating_add(n)`, a capacity hint whose bound is visible
  rather than assumed. No saturating sentinel is introduced: unlike a `PropExpr`
  arity, these are lengths of slices the caller already holds.

## [workspace-v0.4.0] - 2026-07-25

### Changed

- **Hardened the core-crate rayon determinism guards ([#48](https://github.com/sustia-llc/catgraph/issues/48))** —
  extended the parallel-vs-sequential equivalence discipline already applied in
  `catgraph-applied` to the core crate's two rayon sites, and to a third,
  previously-untested applied site. The pre-existing `tests/rayon_equivalence.rs`
  guards were upgraded from set-shape / depth-only checks to exact assertions:
  `NamedCospan::find_nodes_by_name_predicate` now compares against an in-test
  hand-rolled sequential reference with exact ordered-`Vec` equality (below and
  above its size threshold), plus the `at_most_one=true` short-circuit and
  no-match cases. `FrobeniusMorphism` / `FrobeniusLayer` `hflip` gains direct
  `#[cfg(test)]` unit tests in `src/frobenius/operations.rs` (reachable in-module:
  `FrobeniusLayer::hflip` is module-private, `FrobeniusMorphism::hflip` is
  `pub(crate)`) asserting sequential-reference equality and the `hflip ∘ hflip ==
  id` involution on layers wide enough that rayon actually subdivides
  (`with_min_len(m)` only splits at length ≥ 2·m), plus public-API determinism
  guards through `special_frobenius_morphism` and
  `cospan_algebra::cospan_to_frobenius`. In `catgraph-applied`,
  `LinearCombination::linear_combine` — a second `CondIterator` dispatch point
  that had no equivalence coverage — gains threshold-straddling par-vs-seq tests
  (including a non-injective combiner that forces coefficient collisions). All
  guards run under both the default and `--no-default-features` builds.

- **Paper-audit citation reconciliation (Phase 1, PRs #112/#113)** — verified the
  FS19 (Hypergraph Categories) anchors against the cached paper and fixed drifted
  citations: `Thm 1.2 / Thm 4.13` isomorphism-vs-equivalence phrasing in README /
  rustdoc, `FS19-AUDIT.md` internal count drift, Lemma 4.3 "io" (cross-label)
  qualifier, and `RelabelingFunctor` re-cited as the single-map component of
  Prop 2.1 / Cor 3.13. `operadic.rs` grounded in its `1305.0297` anchor and FS18
  (`1803.05316`) declared a secondary core anchor. `tests/spider_theorem.rs`
  upgraded from shape-only to full structural-equality assertions.

### Added

- **`scripts/check_audit_counts.py` CI guard (#111)** — checks the hand-maintained
  audit-doc tallies (summary arithmetic, headline percentages, per-section emoji
  counts, `(N tests)` citations) for self-consistency; wired into CI for
  `FS19-AUDIT.md` (Phase 1), then extended to `FS18-AUDIT.md` (Phase 2) and
  `BV25-AUDIT.md` (Phase 3).
- `CatgraphError::RecursionLimit { depth, limit }` — shared term-interpreter
  recursion-guard error, so `catgraph-syntax` interpreters whose error type is
  fixed to `CatgraphError` (e.g. a `CompleteFunctor`) report the same shape as
  its `SyntaxError::RecursionLimit`
  ([#99](https://github.com/sustia-llc/catgraph/issues/99)).
- `cospan_canon` — `CospanCanon<Λ>` and `Cospan::canonical_form`, a decidable
  (hashable, `Eq`) invariant for parallel cospans up to apex isomorphism.
  Records each apex vertex's `(label, sorted dom preimage, sorted cod preimage)`
  as a sorted multiset, so **scalars** (apex-only bubbles) are counted rather
  than collapsed — the *special* (not extra-special) semantics. Enables the
  complete Cospan-valued decision functor in `catgraph-syntax`
  ([#80](https://github.com/sustia-llc/catgraph/issues/80), F&S 2019 Prop 3.8).

> **Reconciliation note
> ([#158](https://github.com/sustia-llc/catgraph/issues/158)).** Workspace tags
> `v0.1.1`, `v0.2.0`, `v0.2.1`, and `v0.3.0` (2026-07-02 → 2026-07-11) were cut
> without per-crate sections here; this crate's changes across them are recorded
> only in git history (`git log v0.1.0..v0.3.0 -- catgraph/`) and the
> workspace-level release record. Backfill was deferred out of the v0.4.0
> release (owner, 2026-07-25) and resolved as this note (#158, option 2).
> Separately, `v0.5.0` deliberately rolled no section here — the crate had no
> changes at that tag.

## [workspace-v0.1.0] - 2026-07-01

First monorepo release: workspace-wide tag `v0.1.0` (supersedes the pre-reboot
crate-scoped version lineage below). The coalition semantic-layer handoff to
downstream koalisi.

The reboot workspace is being assembled phase by phase toward `0.1.0`. This crate
— the strict implementation of Fong & Spivak, *Hypergraph Categories* (2019) —
is carried intact from prior work into a fresh five-crate workspace built on a
thin [DeepCausality](https://github.com/deepcausality-rs/deep_causality) algebraic
substrate (numeric backends kept optional).

### Added

- `Cospan<Λ>` with pushout composition (union-find, O(n·α(n))); `Span<Λ>` and
  `Rel<Λ>` via pullback (the dual); `Corel<Λ>` — jointly-surjective cospans, the
  dual of `Rel` (FS 2018 Ex 6.64).
- `NamedCospan<Λ, L, R>` — port-labeled cospans for wiring-style composition.
- `Monoidal`, `SymmetricMonoidalMorphism`, `GenericMonoidalMorphism` — tensor
  product and permutation-based braiding.
- `FrobeniusMorphism` + `MorphismSystem` (Def 2.5); `HypergraphCategory` and
  `HypergraphFunctor` (§2.3, Eq 12).
- Self-dual compact closed structure — cup/cap, name/unname, `compose_names_direct`
  (Props 3.1–3.4, zigzag identities Eq 13).
- `CospanAlgebra` with `PartitionAlgebra` (Ex 2.3, Prop 4.6 initiality) and
  `NameAlgebra` (§4.1).
- The §4 equivalence `Hyp_OF(Λ) ≅ Lax(Cospan_Λ, Set)` — Theorem 1.2 in its per-Λ form
  (= Thm 4.13), with Lemmas 4.3 / 4.9 and `CospanToFrobeniusFunctor` (Prop 3.8).
- `MorphismSystem` dependency-graph acyclicity (`add_definition_composite`) and
  bottom-up resolution order (`fill_black_boxes`) run on the zero-dependency
  `ultragraph` graph substrate (DeepCausality) via `topological_sort`. `parallel`
  (default-on) feature for rayon at hot call sites. `--no-default-features` yields
  a slim, single-threaded WASI-compatible build.

### Changed

- Graph substrate moved from `rustworkx-core`/`petgraph` to the zero-dependency
  `ultragraph` (DeepCausality) for `MorphismSystem` dependency resolution, dropping
  the `rustworkx-core` → `ndarray` + `serde` transitive chain from this crate. The
  `rustworkx` feature is removed (no slim-vs-full split remains). The speculative
  `Cospan::to_graph` / `NamedCospan::to_graph` petgraph exports — which had no
  in-crate consumers — were removed; they will be reintroduced shaped to a real
  consumer if one materializes.

### Notes

- Test posture: 517 (default and `--no-default-features` now identical — removing
  the `rustworkx` feature collapsed the prior split). Zero `unsafe`.
- Permanently-deferred paper items (cross-Λ functoriality, strictification,
  §3.3 io/ff factorization, the global Grothendieck form, LinRel examples) are
  catalogued in [`docs/FS19-AUDIT.md`](docs/FS19-AUDIT.md).

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.15.0...HEAD
[workspace-v0.15.0]: https://github.com/sustia-llc/catgraph/compare/v0.14.0...v0.15.0
[workspace-v0.14.0]: https://github.com/sustia-llc/catgraph/compare/v0.13.0...v0.14.0
[workspace-v0.12.0]: https://github.com/sustia-llc/catgraph/compare/v0.11.0...v0.12.0
[workspace-v0.10.0]: https://github.com/sustia-llc/catgraph/compare/v0.9.0...v0.10.0
[workspace-v0.9.0]: https://github.com/sustia-llc/catgraph/compare/v0.8.0...v0.9.0
[workspace-v0.6.0]: https://github.com/sustia-llc/catgraph/compare/v0.5.0...v0.6.0
[workspace-v0.4.0]: https://github.com/sustia-llc/catgraph/compare/v0.1.0...v0.4.0
[workspace-v0.1.0]: https://github.com/sustia-llc/catgraph/releases/tag/v0.1.0
