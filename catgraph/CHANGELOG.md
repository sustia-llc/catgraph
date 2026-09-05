# Changelog

All notable changes to `catgraph` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the crate adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [workspace-v0.18.0] - 2026-09-05

### Added

- `tests/canonical.rs`: the eleven Def 2.5 equations, the Def 2.12 generator
  table, both zigzags and strict left/right unitality on the
  `HypergraphCategory` implementors, decided by `CospanCanon`; `compose`
  against a union-find partition reference; the tensor wiring law on the
  public `Monoidal` implementors
  ([#410](https://github.com/sustia-llc/catgraph/pull/410)).
- `Decomposition::to_finset_morphism`; `GenericMonoidalMorphism::append_layer`
  is `pub` ([#410](https://github.com/sustia-llc/catgraph/pull/410)).

### Added — tooling

- `scripts/check_canonical_tests.py`, its `ci.yml` row and CLAUDE.md rule 7:
  every `pub struct|enum|trait|type` under a published crate's `src` is named
  in its `tests/canonical.rs` header's `covers:` or `not-covered:` list
  ([#410](https://github.com/sustia-llc/catgraph/pull/410)).

### Changed

- `GenericMonoidalMorphism::append_layer` and `FrobeniusMorphism::append_layer`
  keep the popped layer on the type-mismatch `Err`; the `tests/canonical.rs`
  `GenericMonoidalMorphism` tensor row runs at unequal depths
  ([#414](https://github.com/sustia-llc/catgraph/pull/414)).
- `tests/common/mod.rs` and `tests/compact_closed.rs` cite test files without a
  count ([#412](https://github.com/sustia-llc/catgraph/pull/412)).
- README §Testing describes `tests/canonical.rs`
  ([#418](https://github.com/sustia-llc/catgraph/pull/418)).

### Removed

- `tests/frobenius_axioms.rs`, subsumed by `tests/canonical.rs`
  ([#410](https://github.com/sustia-llc/catgraph/pull/410)).

## [workspace-v0.17.0] - 2026-09-03

### Changed

- Rustdoc in `src/` states what each item does over what input space; this
  CHANGELOG is one bullet per change
  ([#404](https://github.com/sustia-llc/catgraph/pull/404)).
- `SymmetricMonoidalMorphism::permute_side` rustdoc drops its `PetriNet`
  known-deviation section
  ([#275](https://github.com/sustia-llc/catgraph/issues/275)).
- `equivalence` module doc cites F&S §4 Theorem 4.13 alone
  ([#407](https://github.com/sustia-llc/catgraph/pull/407)).

### Fixed — tests

- `tests/corel_quotient.rs`'s six `MEASURED` emitters each print a leading
  newline, so a multi-threaded `--nocapture` log cannot merge one into
  libtest's `... ok`
  ([#293](https://github.com/sustia-llc/catgraph/issues/293)).

### Added — tooling

- `scripts/check_measured_claims.py`, wired into CI: a measured figure cited in
  prose must equal the fact the emitting test printed, cited by an HTML comment
  placed immediately after the number. 21 citation sites over 6 keys carry
  markers, in this file and in `tests/corel_quotient.rs`. Perturbation results
  and figures inside assertion messages carry no marker
  ([#293](https://github.com/sustia-llc/catgraph/issues/293)).

## [workspace-v0.16.0] - 2026-08-24

### Changed — BREAKING (#351: `Corel::compose` restricts to the outer boundary)

- `Corel`'s `Composable::compose` performs F&S 2018 (*Seven Sketches*)
  Example 4.61 fn. 2 step (iii) — restriction to `A ⊔ C` — on top of the
  pushout, so it no longer returns values `Corel::new` would reject
  ([#351](https://github.com/sustia-llc/catgraph/issues/351)). Smallest
  witness: `Cospan::new(vec![], vec![0], vec!['m'])` composed with
  `Cospan::new(vec![0], vec![], vec!['m'])` was
  `left=[] right=[] middle=['m']` (`is_jointly_surjective() == false`,
  `scalar_count() == 1`) and is now the empty corelation.
- That witness is `Corel::unit('m') ; Corel::counit('m')`, so the fixed
  composite `η ; ε == id_I` is the extra-special axiom (Baez–Erbele 2015,
  *Categories in Control*, arXiv:1405.6881 §2, p. 11), pinned as
  `tests/corel_quotient.rs::extra_special_axiom_unit_then_counit_is_id_i`.
  Nothing in-tree proves the Baez–Erbele identification; it is a match of
  descriptions.
- **What breaks:** a consumer that composes corelations and counts apex
  vertices, reads `as_cospan().middle()`, or hashes/compares the underlying
  `Cospan` sees a smaller apex wherever a composition merged two boundary-only
  vertices. Measured over the exhaustive corpus in `tests/corel_quotient.rs`
  (every cospan with apex ≤ 3, boundary ≤ 2, one wire type): of **4 803**
  composable pairs of genuine corelations, **2 154** had a raw pushout that was
  not jointly surjective.
- **What breaks, continued:** the relation on `domain ⊔ codomain` is unchanged,
  its encoding is not. `Corel::equivalence_classes()` lays flat indices out as
  `0..dom_len` │ `dom_len..dom_len + mid_len` │ `dom_len + mid_len..`, so a
  smaller apex shifts every codomain flat index down; `Corel::merges(a, b)`,
  `Corel::is_identity_partition` and `equivalence_classes().len()` all change on
  an affected composite. Measured on `a : 1 → {a,a} ← 2` then
  `b : 2 → {a,a} ← 1` (`compose_shifts_the_flat_index_layout`): raw pushout
  classes `[{0,1,3}, {2}]`, `len 2`, `merges(0, 3) == true`,
  `is_identity_partition() == false`; new composite `[{0,1,2}]`, `len 1`,
  `merges(0, 3) == false` (`merges(0, 2) == true`),
  `is_identity_partition() == true`.
- ⚠ `Corel::refines` matches classes by flat index across two values and
  silently skips elements not found in the other's classes, so with composites
  now carrying smaller apexes it can skip the entire boundary and return `true`.
  Pre-existing, not fixed here.

### Added (#351)

- `Corel::from_cospan_dropping_bubbles(Cospan<Lambda>) -> Corel<Lambda>` — the
  total map `Cospan → Corel`: every apex vertex neither leg reaches is dropped
  and both legs are reindexed onto the survivors, which keep their relative
  order and their labels. `domain()` and `codomain()` are untouched, the image
  satisfies `is_jointly_surjective()` and
  `canonical_form().scalar_count() == 0`, and the map is idempotent and the
  identity on an already jointly-surjective input. Written over
  `left_to_middle` / `right_to_middle` / `middle` rather than through
  `CospanCanon`, which would need `Lambda: Ord + Hash` and return a canonical
  witness rather than this one
  ([#351](https://github.com/sustia-llc/catgraph/issues/351)).
- ⚠ Not the `canonical_form` bubble drop: `Cospan::canonical_form` keeps
  scalars, and `classes.retain(|c| !c.is_scalar())` inside it is the deliberate
  mutant [#343](https://github.com/sustia-llc/catgraph/issues/343) runs against
  `tests/property_laws.rs::canonical_form_decides_apex_isomorphism`.

### Tests (#351)

- `tests/corel_quotient.rs` (new, 9 tests) over an exhaustive corpus: every
  cospan with apex ≤ 3, domain ≤ 2, codomain ≤ 2 over one wire type — **228**
  cospans, **139** bubble-carrying (**166** bubble vertices), **25 616** ordered
  arity-matching pairs of which **6 896** grow a bubble the operands did not
  carry. `corpus_is_not_vacuous` measures those first and asserts non-zero the
  three that can independently be zero.
- Step (iii)'s scope is pinned by a boundary-only flat-index reading: the
  composite induces the same partition on `domain ⊔ codomain` as the raw
  pushout, and the quotient the same one as its input. The `domain()` /
  `codomain()` assertions beside it are weak on a one-wire-type corpus;
  `quotient_keeps_surviving_labels_in_their_original_order` uses heterogeneous
  witnesses.
- `q(a ; b)` and `q(a) ; q(b)` agree up to apex isomorphism on all 25 616 pairs
  and differ structurally on **1 488**, recorded by
  `quotient_functoriality_is_not_structural`: **0** of the 1 488 have two
  already-jointly-surjective operands, and **1 488 of 1 488** are pairs where
  `perform_pushout`'s identity fast path fires on one side of the quotient and
  not the other (**6 975** pairs flip in total). Smallest witness:
  `a = ([], [0], ['a','a'])`, `b = ([1], [0,1], ['a','a'])`.
- `tests/corel.rs::compose_preserves_joint_surjectivity` renamed to
  `compose_of_unfold_then_fold_is_jointly_surjective`: it asserts one input pair
  (`f : 1 → {a} ← 2`, `g : 2 → {a} ← 1`), and the universal reading is pinned
  over 4 803 pairs by `corel_quotient.rs::compose_result_is_always_a_corelation`.
  Measured: with the fix reverted the renamed test stays green.
- **Falsification.** Six perturbations of `src/corel.rs`, measured against the
  nine tests in `tests/corel_quotient.rs` and reverted.
  (1) Restoring `map(Self::new_unchecked)` reddens **6 of 9**; functoriality
  goes to **6 896** mismatching pairs and structural mismatches to **8 204**, of
  which **2 154** have two jointly-surjective operands.
  (2) Filtering the apex without reindexing the legs reddens **6 of 9**, in
  `Cospan`'s bounds check.
  (3) Reindexing in-bounds but in reverse order reddens **2 of 9** — a
  consistently renumbered apex is unobservable on a single-label corpus, which
  is why `quotient_keeps_surviving_labels_in_their_original_order` uses
  heterogeneous labels.
  (4) Reversing the left-leg vector of the value `Corel::compose` returns
  reddens **4 of 9**; `quotient_is_total_and_lands_in_corel` stays green,
  never calling `compose`.
  (5) The same reversal inside `from_cospan_dropping_bubbles` reddens
  **6 of 9**, `quotient_is_total_and_lands_in_corel` and
  `compose_result_is_always_a_corelation` failing at the boundary-partition
  assertion.
  (6) Restricting **only when the pushout leaves exactly one bubble** — an
  order-dependent rule, which is the shape of defect an associativity pin
  exists to catch — reddens **4 of 9**, with
  `new_composition_is_associative_up_to_apex_isomorphism` failing on **192** of
  **14 473**<!--m:assoc.triples--> triples.
- The new composition's own category law is pinned
  (`new_composition_is_associative_up_to_apex_isomorphism`). It does not: over
  the **14 473**<!--m:assoc.triples--> composable triples of
  corelations the apex ≤ 2 corpus
  offers, **0**<!--m:assoc.iso_mismatches--> differ up to apex isomorphism,
  **456**<!--m:assoc.structural_mismatches--> differ structurally, and
  **5 048**<!--m:assoc.restriction_fired--> have step (iii) firing somewhere —
  that last count is asserted
  non-zero, so the sweep cannot pass by being about the raw pushout. Those
  **456**<!--m:assoc.structural_mismatches--> are
  the same `perform_pushout` apex-numbering artefact recorded above and not a
  second phenomenon, and the **correlate** of that is asserted rather than left
  in prose: all **456**<!--m:assoc.structural_mismatches--> of
  **456**<!--m:assoc.structural_mismatches--> carry an identity fast-path
  asymmetry between the two
  associations (**120**<!--m:assoc.outer_asymmetry_only--> of them at the outer
  composition only — printed, not
  asserted, since that split moves with the corpus). ⚠ Necessary, not
  sufficient, and therefore not a proof of the diagnosis:
  **7 828**<!--m:assoc.any_asymmetry--> of the
  **14 473**<!--m:assoc.triples--> triples carry the asymmetry while only
  **456**<!--m:assoc.structural_mismatches--> mismatch, so a second cause
  confined to asymmetric triples would satisfy the assertion unchanged. The
  pre-#351 pushout has 0 mismatches up to apex isomorphism and **512**
  structural ones on the same corpus; the apex ≤ 3 corpus was also measured
  (**261 625** triples, 0 mismatches up to apex isomorphism) and the suite runs
  the smaller one.

### Changed — prose corrections forced by this change (#351)

- `Corel` is not a transparent newtype for composition, and the in-tree prose
  saying it was is corrected in `src/corel.rs`, `tests/frobenius_axioms.rs`,
  `docs/FS19-AUDIT.md`, and the test name
  `corel_hypergraph_category.rs::left_unitality_via_cospan_delegation`, renamed
  `left_unitality_arities`. No count or completeness is claimed: `rg -i delegat`
  found six of the seven sites and misses the seventh
  (`corel_battery_composites_stay_jointly_surjective`, whose rationale #351
  inverts).
- `tests/frobenius_axioms.rs::corel_recomputes_the_cospan_battery`'s docstring
  now states what the test pins — `Corel` and `Cospan` agree on these eleven
  equations — and records that #351's `Composable::compose` override left it
  green.
- `catgraph-applied/docs/FS18-AUDIT.md`'s Ex 4.61 row is repointed from
  `catgraph::span::Rel` to `catgraph::corel`. The `🔗 IN CORE` marker is
  unchanged, so `scripts/check_audit_counts.py`'s tallies do not move.

### Changed — BREAKING (#350: `FrobeniusMorphism` is the *special* theory)

- `two_layer_simplify`'s rule 3 is deleted
  ([#350](https://github.com/sustia-llc/catgraph/issues/350)). It cancelled
  `Unit(z)` feeding directly into `Counit(z)` — the extra-special axiom
  `ε ∘ η = id_I`, not among the nine equations of F&S 2019 Def 2.5.
- **What breaks:** `FrobeniusMorphism::compose` on an `η` meeting an `ε`
  returns a two-layer term (`depth() == 2`) that the derived `Eq` separates from
  `FrobeniusMorphism::identity(&vec![])`; `special_frobenius_morphism(0, 0, z)`
  returns that same term; `cospan_to_frobenius` keeps an apex vertex neither leg
  reaches, so `Cospan::new(vec![], vec![], vec!['a'])` maps to `η;ε` and
  `Cospan::new(vec![0], vec![0], vec!['a','b'])` to a depth-2 term instead of
  `identity(['a'])`; and `frobenius_to_cospan` of a spelled `η;ε` is the bubble
  (apex 1, one scalar class) instead of the empty cospan. `catgraph-surreal`
  mirrors `Cospan::canonical_form`, which rule 3 never touched, so no stored row
  moves.
- Rules 1, 2 and 4 keep their numbers, so every "Rule 4" reference still points
  at spider fusion; the gap is documented in `two_layer_simplify`'s rustdoc.
- The `(0, 0)` carve-outs in `generator_to_cospan` and
  `Frobenius::basic_interpret` are measured redundant — letting `(0, 0)` recurse
  leaves `cargo test -p catgraph` green — and are kept so the bubble does not
  depend on the layer simplifier. Dropping `generator_to_cospan`'s carve-out
  used to redden **48 of 383** terms in `frobenius::to_cospan_pin` and reddens
  **0** now.
- Eleven terms in `to_cospan_pin`'s 383-term space denote different cospans
  (`random_11`, `_30`, `_63`, `_118`, `_124`, `_140`, `_159`, `_217`, `_230`,
  `_263`, `_271`), each gaining one or more scalar `ApexClass`. Distinct
  canonical forms go **172 → 175** over the 300 random terms and **209 → 212**
  over the whole space — a net change in distinct forms, not a count of terms
  that moved. The count of random terms whose image has a `0 → 0` boundary is
  unchanged at **46** (4 distinct scalar-shaped forms before, 6 now); the count
  carrying at least one scalar class goes **64 → 71**.

### Tests (#350)

- Six pins re-pointed, none deleted:
  `frobenius::operations::test::test_unit_counit_cancel` →
  `test_unit_counit_does_not_cancel` (absorbing
  `test_unit_counit_no_cancel_different_labels`);
  `cospan_algebra::tests::scalar_bubbles_are_lost_in_both_directions` →
  `scalar_bubbles_survive_in_both_directions`;
  `tests/frobenius_axioms.rs::frobenius_scalar_loop_is_erased_before_interpretation`
  → `frobenius_scalar_loop_survives_to_interpretation`;
  `tests/frobenius_laws.rs::unit_counit_scalar` pins depth 2;
  `tests/equivalence.rs::lemma_4_9_cospan_to_name_on_a_non_identity_morphism`
  moves from depth 3 to 4; and
  `tests/hypergraph_functor.rs::ctf_single_apex_cospan_round_trips_up_to_canonical_form`
  lifted its `(0,0)` exclusion, so all 25 single-apex cospans round-trip.
- Three pins that had become vacuous are sharpened:
  `test_unit_counit_cancel_via_compose` → `test_unit_counit_scalar_survives_compose`
  with a depth assertion;
  `cospan_algebra::tests::cospan_to_frobenius_unhit_apex_node_is_total` →
  `cospan_to_frobenius_unhit_apex_node_keeps_the_bubble`, which reverting the
  #285 guard now reddens (depth 1 against 2);
  `tests/spider_theorem.rs::spider_0_0_via_eta_epsilon` compares two two-layer
  terms with no assertion changed.
- `cospan_frobenius_cospan_round_trips` gained a bare bubble and `id_a` beside a
  bubble.
- **Falsification.** Restoring rule 3 reddens nine of the ten tests above; the
  tenth, `spider_0_0_via_eta_epsilon`, builds both sides the same way and cannot
  redden.
- `tests/spider_theorem.rs` keeps its `m = n = 0` and component-closing
  exclusions; lifting them is
  [#353](https://github.com/sustia-llc/catgraph/issues/353).

### Tests (#343: the `CospanCanon` iff proptest now reaches the bubble dimension)

- `arb_cospan_and_perturbation` gained a `BubbleOp` arm that adds or drops a
  bubble vertex, so `canonical_form_decides_apex_isomorphism` reaches
  `exists_apex_iso`'s size guard and the form's `apex_len` dimension
  ([#343](https://github.com/sustia-llc/catgraph/issues/343)). Test-and-docs
  only.
- The drop arm's reindexing is checked in the generator against an independent
  reference. Falsified: `if *m >= middle.len()` in place of `if *m > victim`
  leaves the suite green without that check (bubble-separated corpus 72 → 61)
  and reddens three tests with it.
- Falsified: the bubble-drop mutant `classes.retain(|c| !c.is_scalar())` before
  `classes.sort()` in `CospanCanon::canonical_form` reddens
  `canonical_form_decides_apex_isomorphism`. Before this arm it left all 19
  tests in `tests/property_laws.rs` green; it now fails 1 of 20 there and 14
  workspace-wide rather than 13.
- `perturbation_generator_reaches_bubble_edits` asserts both size directions
  fire (57 grew, 40 shrank, 159 unchanged of 256 deterministic samples) and the
  load-bearing quantity directly: **72** of 256 pairs the oracle calls
  non-isomorphic while `dom_len`, `cod_len` and every non-bubble class agree,
  **59** of them with a non-bubble class present. Falsified: restricting the
  bubble arm to empty-boundary cospans drives 59 to 0 while the grew/shrank
  assertion stays green.
- The `iff` test's docstring states the corpus: 133 isomorphic and 123
  non-isomorphic pairs of 256; apex size differs in 97, `scalar_count` in 93,
  and in **72** of the 123 only the bubble classes separate the forms. One
  residual stays excluded: a pair differing only by an in-place relabelling of
  one apex vertex is never generated.
  `perturbation_generator_reaches_isomorphic_and_non_isomorphic_pairs` now
  asserts the discriminating subcorpus — non-isomorphic at equal apex size —
  which fell **64 → 26**; neutering the rewire arm reddens it at `0 of 93`.
- `arb_cospan_and_perturbation` yields
  `Perturbed { a, b, rewire_only, rewired }`, where `rewire_only` withholds the
  bubble arm, and the guard asserts the exact pair `(51, 26)`. Falsified:
  neutering the rewire arm takes it to `(0, 0)`. ⚠ A fourth arm added later must
  be withheld from `rewire_only` too.

### Tests (#288: `tests/spider_theorem.rs` widened to the theorem it cites)

- Thm 6.55 is pinned at the semantics — on the image under
  `frobenius_to_cospan`, canonicalised by `Cospan::canonical_form` — over a
  generated corpus of 2105 terms built from η, ε, μ, δ, σ and `id` alone,
  yielding 1280 connected diagrams, each asserted to have exactly one apex
  vertex, no scalar class, preimages covering `0..m` and `0..n`, and to equal
  the canonical form of `special_frobenius_morphism(m, n, z)`
  ([#288](https://github.com/sustia-llc/catgraph/issues/288)). Test-and-docs
  only.
- The five hand-built term-level tests are kept with docstrings saying what they
  pin: each recipe is the one `special_frobenius_morphism` follows at that
  arity, so they are a builder-shape pin. `FrobeniusMorphism`'s derived `Eq` is
  syntactic up to `two_layer_simplify`'s rules, so a term-level assertion cannot
  be widened to arbitrary connected diagrams — measured on `d6c7bd5`,
  `(δ ⊗ id);(id ⊗ μ)`, `σ;μ;δ`, `(μ ⊗ id);(δ ⊗ id);(id ⊗ μ)`,
  `(δ ⊗ id);(id ⊗ σ);(μ ⊗ id)`, `(η ⊗ id);μ` and the left-comb `4 → 1` are all
  connected and all structurally ≠ their spider.
- The corpus's interior-waist bias is measured and answered by the 16-term
  `wide_waist_family` and the 1488-term `wide_waist_permutation_family`, which
  sweeps every permutation of the `2m` middle wires of a δ-fan folded into a
  μ-fan at `m ∈ {2, 3}`, `n ∈ {2, 3}` (992 connected, 496 disconnected). The
  connected interior-waist histogram is `{None: 45, 1: 205, 2: 23, 3: 619,
  4: 388}`, all three buckets census-pinned with a floor on the wide one;
  `permutations` / `transposition_word` are themselves checked (`n!` distinct
  outputs, each word realising its target).
- Connectivity is decided by a disjoint-set over the recipe, never read off the
  oracle, and the 716 disconnected recipes are asserted to denote exactly their
  own component count. A σ spans two components only if its sides are still
  distinct when the recipe ends (121 of the 132 counted at braid time survive).
- Excluded by design: `m == n == 0`; any recipe that closes a component (62
  corpus terms, on the special vs extra-special line #350 decided — lifting them
  is [#353](https://github.com/sustia-llc/catgraph/issues/353)); and the empty
  term, produced 47 times, which is census-pinned and ranged over by no claim
  test. Both claim tests assert `scalar_count() == 0` on every term they range
  over.
- Eleven of the twelve counts in `the_corpus_is_the_space_these_pins_claim` are
  properties of the generator alone; the twelfth, the number of distinct
  disconnected canonical forms, runs through `frobenius_to_cospan` and moves
  with production code (203 → 220 under the comultiplication perturbation).
- **Falsification.** A disconnected `Comultiplication` arm: 1165 of 1280
  connected and 380 of 716 disconnected terms disagree. A merging
  `SymmetricBraiding` arm, restricted to same-label σ: 397 of 716 disconnected
  recipes disagree, the connected arm green. Dropping
  `wide_waist_permutation_family`: `MIN_CONNECTED` reddens at 288 of 617 and the
  wide bucket falls to 38. Mirroring `special_frobenius_morphism`'s odd-`m`
  branch to `id ⊗ sfm(m-1, 1)` reddens two of the five term-level tests and
  leaves the semantic pin green.

### Changed — BREAKING (#289: the checked boundary-node mutators)

- `Cospan` has no cached identity flags: the private `is_left_id` /
  `is_right_id` fields are deleted and `Cospan::is_left_identity()` /
  `is_right_identity()` keep their signatures and return
  `leg.len() == middle.len() && represents_id(leg)`, `O(leg)` per call and exact
  in both directions ([#289](https://github.com/sustia-llc/catgraph/issues/289)).
  An answer can move either way: at `0.15.0` every composite with at least one
  apex vertex reported `false` on both legs, so
  `Cospan::identity(&vec!['a']).compose(&Cospan::identity(&vec!['a']))` reads
  `true` here and `false` there; and `Cospan::identity(&vec!['a'])` followed by
  `add_boundary_node_unknown_target(Right('b'))` read `true` there and reads
  `false` here.
- `Cospan::assert_valid` loses both `bool` parameters — the signature is
  `assert_valid(&self)` — and `NamedCospan::assert_valid` /
  `assert_valid_nohash` lose the `check_id: bool` they forwarded. The two bounds
  `debug_assert!`s are unchanged and still compile away in release.
- `Cospan`'s derived `Debug` output loses two fields, so anything logging,
  snapshotting or diffing a formatted cospan sees a different string:
  `Cospan { left: [0], right: [0, 1], middle: ['a', 'b'], is_left_id: false,
  is_right_id: true }` at `0.15.0` against
  `Cospan { left: [0], right: [0, 1], middle: ['a', 'b'] }` now. It carries into
  `Corel` and `catgraph-applied`'s `DecoratedCospan`, whose `Debug` is derived
  over one; `NamedCospan` derives only `Clone`.
- `Span` is untouched: it keeps its two flags, which carry no boundary-length
  conjunct and are not read by `Span::compose`, and keeps
  `assert_valid(&self, bool, bool)`
  ([#345](https://github.com/sustia-llc/catgraph/issues/345)).
- `Cospan::compose` is a function of `(left, right, middle)` alone:
  `perform_pushout` derives the identity predicate itself instead of taking the
  cached flags, so `identity(&f.domain()) ; f` is `f` on the nose for every `f`.
  Measured on `f = Cospan::new(vec![1, 0], vec![0, 1], vec!['a', 'b'])`: apex
  `['a', 'b']` now, `['b', 'a']` under the union-find numbering.
  `canonical_form` is equal across the change. Pinned by
  `tests/compose_identity_arms.rs`, whose five tests redden when the
  `left_leg_id` arm is disabled — including that when both legs are the identity
  the composite keeps the **right** operand's apex labels, observable only for a
  `Lambda` whose `Eq` is coarser than identity.
- The `add_boundary_node` family is checked and the raw path is `*_unchecked`:
  `Cospan::add_boundary_node` and `add_boundary_node_known_target` return
  `Result<Either<LeftIndex, RightIndex>, CatgraphError>`, raising
  `ConstructionIndexOutOfBounds` and leaving the cospan untouched on `Err`;
  `Cospan::add_boundary_node_unchecked` is new;
  `add_boundary_node_unknown_target` keeps its infallible signature;
  `NamedCospan::add_boundary_node` and both its `_known_target` /
  `_unknown_target` wrappers return the same `Result` beside the new
  `NamedCospan::add_boundary_node_unchecked`; `NamedCospan::add_middle` returns
  the new `MiddleIndex` it used to discard; and `Span::add_boundary_node` keeps
  its infallible signature and gains no `_unchecked` sibling, its argument being
  a label rather than an index.
- `CatgraphError::ConstructionDuplicatePortName { leg, existing_position }` is a
  new variant (additive under `#[non_exhaustive]`). It does not carry the name,
  which is bounded only by `Eq`.
- `finset::from_cycle` validates its cycle in every build profile and before any
  recursion: elements must be `< n` and pairwise distinct. Previously
  `from_cycle(3, &[7])` returned the identity and `from_cycle(3, &[0, 1, 0])`
  returned a permutation that is not the documented cycle.
- `utils::remove_multiple` deduplicates and bounds-checks its index list, so
  `to_remove = [3, 3]` removes index 3 once instead of also deleting the element
  that had been at 4.

### Fixed (#289)

- `Cospan`'s identity accessors cannot go stale, there being nothing left to go
  stale ([#289](https://github.com/sustia-llc/catgraph/issues/289)). Four
  writers were responsible for the flags and each failed differently:
  `add_boundary_node`'s `Left(idx)` arms tested `leg.len() - 1 == tgt_idx`;
  its `Right(label)` arms updated only the flag of the leg they push to while
  growing the apex (inherited by `NamedCospan`); `delete_boundary_node` tested
  `z == leg.len() - 1`; and `connect_pair` updated nothing (reachable through
  `WiringDiagram::connect_pair` in `catgraph-applied`). A stale `true` was a
  wrong composition while `perform_pushout` fast-pathed on the flags. Pinned in
  `tests/checked_mutators.rs`, where
  `cospan_identity_accessors_need_the_leg_to_cover_the_whole_apex` names the
  `leg.len() == middle.len()` conjunct all four dropped and 9 of that file's 27
  tests redden when it is deleted from `leg_is_identity`.
- `Cospan::connect_pair` merges the two ports in every argument order
  ([#289](https://github.com/sustia-llc/catgraph/issues/289)). Its leg remap
  wrote node 1's old apex index after `swap_remove` had moved that vertex into
  node 2's slot, so when node 1's vertex was the last apex index both legs
  received `middle.len()` and the ports were never merged. Measured before the
  fix: `Cospan::new(vec![0, 1], vec![0, 1], vec!['a', 'a'])` →
  `connect_pair(Left(1), Left(0))` gave `left = [1, 0], right = [1, 0]` over a
  1-vertex apex with `map_to_same` false. Reachable through
  `NamedCospan::connect_pair` and `WiringDiagram::connect_pair`. Pinned in
  `tests/checked_mutators.rs` on both surfaces.
- `Cospan::assert_valid` no longer rejects valid cospans
  ([#289](https://github.com/sustia-llc/catgraph/issues/289)): its strong arm
  compared `represents_id(leg)` against the cached flag without the
  `leg.len() == middle.len()` conjunct, so
  `Cospan::new(vec![0], vec![0, 1], vec!['a', 'b'])` tripped
  `assert_valid(true, _)` in debug.
- `Span::add_middle` bounds-checks the pair before reading the labels
  ([#289](https://github.com/sustia-llc/catgraph/issues/289)), returning
  `ConstructionMiddlePairOutOfBounds` where `add_middle((usize::MAX, 0))` used
  to panic with a bare slice message in every profile. The reported
  `pair_position` is the position the pair would have taken.
- `Cospan::delete_boundary_node`, `Cospan::map_to_same`, `Cospan::connect_pair`
  and `NamedCospan::delete_boundary_node` carry `# Panics` sections and messages
  naming the out-of-bounds index and the boundary size, in every build profile
  ([#289](https://github.com/sustia-llc/catgraph/issues/289)). The empty-leg
  case is why the checks are explicit: `delete_boundary_node` read
  `leg.len() - 1` first, which underflowed.

### Added (#289)

- `Cospan` derives `PartialEq` and `Eq` — additive. `==` compares
  `(left, right, middle)` field for field, which is the whole of the value now
  that the identity flags are gone. `Cospan::structurally_equal` stays,
  undeprecated, as a named alias. `==` is as coarse as `Lambda`'s `Eq`, which
  `Cospan` never requires to be identity (see
  `tests/compose_identity_arms.rs::both_legs_identity_keeps_the_right_operands_labels`),
  and finer than equality of cospans as morphisms, `CospanCanon` remaining the
  semantic comparison
  ([#289](https://github.com/sustia-llc/catgraph/issues/289)).
- `catgraph-applied`'s `DecoratedCospan` gains a hand-written `PartialEq` and no
  `Eq` in the same window; see that crate's CHANGELOG.

### Changed — BREAKING

- `frobenius::frobenius_to_cospan` is now a re-export of
  `cospan_algebra::frobenius_to_cospan`, and the `frobenius::operations` body is
  deleted along with its private `operation_to_cospan`
  ([#336](https://github.com/sustia-llc/catgraph/issues/336)). Every existing
  import compiles unchanged. Two things the `pub use` could not bridge change
  for callers of the `frobenius::` path: the bounds narrow to require
  `Lambda: Send + Sync` and `BlackBoxLabel: Send + Sync`, and an
  `UnSpecifiedBox` is rejected with `CatgraphError::Interpret` rather than
  `CatgraphError::Composition`, its merged message naming both the generator and
  the arities (`N in, M out`). All three facts are pinned in
  `frobenius::to_cospan_pin::black_boxes_are_rejected_by_both`. The README's two
  Feature Map rows for Prop 3.8 collapse into one.

### Added

- `cospan_algebra::frobenius_to_cospan` — interprets a
  `FrobeniusMorphism<Lambda, _>` in `Cospan<Lambda>` layer by layer, inverse in
  spirit to `cospan_to_frobenius`, and errors with `CatgraphError::Interpret` on
  an `UnSpecifiedBox`
  ([#284](https://github.com/sustia-llc/catgraph/issues/284)). F&S 2019 Prop 3.8
  is a one-to-one correspondence between SCFMs in a symmetric monoidal `C` and
  strict symmetric monoidal functors `(Cospan, ⊕) → (C, ⊗)`, both directions of
  which are functors *out of* `Cospan`, so neither is this map; what it licenses
  is the construction, `Cospan_Λ` carrying an SCFM structure on each object
  (Ex 2.8). Composing with `Cospan::canonical_form` gives equality up to apex
  isomorphism, which no equality on `FrobeniusMorphism` could state.

### Fixed

- `Frobenius::basic_interpret`'s default interpreted every braiding as the
  identity ([#284](https://github.com/sustia-llc/catgraph/issues/284)): it built
  `Permutation::try_from(vec![0, 1])` where `σ: [z1, z2] → [z2, z1]` needs the
  transposition `[1, 0]`.
- `Frobenius::basic_interpret`'s default interpreted the bubble
  `Spider(z, 0, 0)` as `id_I`
  ([#284](https://github.com/sustia-llc/catgraph/issues/284)). It now builds the
  bubble directly as `interpret_unit(z) ; interpret_counit(z)`; every other
  spider arity still recurses. Pinned by
  `frobenius::trait_impl::tests::basic_interpret_default_spider_zero_zero_is_the_bubble`
  on the `Cospan`-backed probe implementor `CospanBacked`, added beside
  `Defaulting`. Space: one generator, one label, one carrier.
- `cospan_algebra::generator_to_cospan`'s `Spider(z, 0, 0)` arm recursed into
  `special_frobenius_morphism`, whose simplified term interpreted to apex 0; it
  now builds the bubble `η;ε` directly
  ([#284](https://github.com/sustia-llc/catgraph/issues/284)). Pinned by
  `cospan_algebra::tests::scfm_equal_scalars_have_equal_images`, measured red
  before the change (`Spider(a,0,0)` apex 0 / scalars 0 against `η;δ;(ε⊗ε)`'s
  apex 1 / scalars 1, and the same split beside `id_a`).
- `Frobenius::interpret_frob`'s default rejected `identity(&vec![])`
  ([#284](https://github.com/sustia-llc/catgraph/issues/284)). It now interprets
  a block-free, empty-interface layer as `id_I` and rejects only a block-free
  layer with a non-empty interface, matching `frobenius_to_cospan`.
- `compact_closed`'s module docs stated both zigzag identities in the wrong
  composition order ([#284](https://github.com/sustia-llc/catgraph/issues/284)),
  corrected to `(cup_X ⊗ id_X) ; (id_X ⊗ cap_X)` and
  `(id_X ⊗ cup_X) ; (cap_X ⊗ id_X)`.

### Fixed — tests

- `tests/property_laws.rs` asserted a production predicate against its own
  definition, generated only identity left legs, and never randomised an apex
  ([#287](https://github.com/sustia-llc/catgraph/issues/287)). Test-only.
  `rel_equivalence_iff_rst` compared `is_equivalence_rel()` with the production
  body spelled out — measured: three `return true` stubs left it green — and is
  replaced by `rel_predicates_match_a_direct_pair_set_oracle` and
  `rel_predicates_decided_exhaustively_on_small_carriers`, which decide all
  seven `Rel` predicates over every relation on carriers of size 1, 2 and 3 (530
  in all) and cross-check acceptance totals against published enumerations
  (reflexive/irreflexive `2^(n²−n)`, symmetric `2^(n(n+1)/2)`, antisymmetric
  `2ⁿ·3^(n(n−1)/2)`, transitive [A006905](https://oeis.org/A006905) `2, 13, 171`,
  equivalence Bell(n) `1, 2, 5`, partial order
  [A001035](https://oeis.org/A001035) `1, 3, 19`);
  `rel_composites_require_homogeneity` pins the `is_homogeneous() &&` screen.
  New `arb_label_preserving_leg` allows non-identity left legs (139 of 256
  samples for `g`, 138 for `h`), pinned by
  `composability_generators_emit_label_aware_non_identity_left_legs`. New
  `canonical_form_decides_apex_isomorphism` asserts equality of canonical forms
  **is** apex isomorphism against a brute-force search over `S_apex`, with
  `perturbation_generator_reaches_isomorphic_and_non_isomorphic_pairs` pinning
  that both sides are reached (192 isomorphic, 64 non-isomorphic of 256) and
  `a_single_rewire_changes_the_form_unless_it_is_a_relabelling` adding the
  negative.
- `rel_from_selector`'s "same relation for a given mask" was a structural claim
  no test asserted ([#287](https://github.com/sustia-llc/catgraph/issues/287)
  follow-up). The strategies build through a named `rel_from_bools`, and
  `rel_from_selector_matches_its_definition` checks, for every mask over
  `n ∈ {1, 2, 3}` (530 relations), that `rel_from_bools` and `rel_from_mask`
  agree and that `rel_from_mask` matches a reference written from the row-major
  definition. Falsified three ways: a column-major flat index reddens only the
  definition check; reversing `rel_from_mask`'s bit order reddens the parity
  check; reversing `rel_from_bools`'s reddens the parity check alone. `n = 4` is
  drawn but not enumerated.
- The #258 braiding contract was pinned only downstream, and the only core
  *integration* test naming permutation composition was vacuous
  ([#286](https://github.com/sustia-llc/catgraph/issues/286)). Measured:
  inverting the braiding direction in `CospanAlgebraMorphism`'s two constructors
  left `cargo test -p catgraph` fully green while reddening 4 tests in
  `catgraph-applied/tests/braiding_cross_carrier.rs`. New
  `tests/braiding_core_pins.rs` lifts three rows into core —
  `CospanAlgebraMorphism`, `FrobeniusMorphism`'s wiring, and `NamedCospan`'s
  port-name direction — with a hand-written canonical anchor, exhaustive sweeps
  over all `6 + 24 = 30` permutations at `n ∈ {3, 4}` with distinct labels, a
  `permute_side` identity/conjugation sweep, all 36 ordered `S₃` pairs for
  `β(p₁) ; β(p₂) == β(p₁ ; p₂)`, and the arity-mismatch and
  `NamedCospan`-refusal rows. `tests/monoidal_structure.rs::permutation_cospan_compose`
  was rewritten to run all 36 ordered `S₃` pairs over `['a','b','c']` and compare
  the composite's wiring. The exhaustive-permutation generator (`all_perms` /
  `all_perm_indices`) landed in `catgraph-testutil`, adding a
  `[dev-dependencies]` edge on this crate and retiring two copies in
  `catgraph-applied/tests`; the `cospan_wiring` extractor moved to
  `tests/common/mod.rs`. Falsified six ways: the `CospanAlgebraMorphism`
  constructor flip reddens 4 of the 5 new tests and nothing else in core;
  dropping `.inv()` from `FrobeniusMorphism::from_permutation_on_codomain`
  reddens the Frobenius wiring row (also caught by two lib tests); reading
  `FrobeniusMorphism::permute_side`'s domain branch symmetrically reddens the
  conjugation row (also caught by three lib tests); permuting `NamedCospan`'s
  port names by `p` reddens only the new file; using `p.apply` in
  `CospanAlgebraMorphism::permute_side`'s domain branch likewise; and flipping
  `Cospan`'s two constructors reddens the rewritten
  `permutation_cospan_compose` (also caught by 4 pre-existing lib tests), a
  vacuity repair whose pre-#286 version was green under the identical mutation.
  Space: `n ∈ {3, 4}` only, `PartitionAlgebra` and `char` the only algebra and
  label type, `()` the only black-box label, `S₃ × S₃` for composition, and no
  `permute_side` row starting from a non-identity morphism.
- Nothing measured that the crate's two `frobenius_to_cospan` implementations
  agreed ([#336](https://github.com/sustia-llc/catgraph/issues/336)).
  `frobenius::to_cospan_pin` (a `#[cfg(test)]` module — the retired algorithm
  walks the `pub(crate)` `layers`) measured the two up to `canonical_form` over
  383 terms before either body was removed and keeps measuring the survivor
  against the retired one. The space: the ten
  `tests/compact_closed.rs::samples()`, the thirty-six `(m, n) ≤ 5` spiders
  including the `(0, 0)` bubble, both sides of all eleven Def 2.5 equations,
  fifteen cup / cap / name / unname terms, and 300 pseudo-random terms of up to
  8 extension attempts over two labels, at one instantiation
  (`char`/`String`). The grid reaches `m = 5` because
  `special_frobenius_morphism`'s doubling branch is reachable only at even
  `m >= 4`. Falsified on the survivor: dropping the `Spider(z, 0, 0)` carve-out
  reddens 0 of 383 (48 before #350), a disconnected comultiplication 169 of 383,
  and an ill-typed braiding the fold outright. The space is falsified
  separately — short-circuiting the random-term generator leaves the
  differential assertion green and the diversity floor reddens (7 distinct
  canonical forms against 175 measured; 212 over all 383). Differential only:
  both sides fold with the same `Cospan::compose`.
- The `compact_closed` suite asserted only interfaces
  ([#284](https://github.com/sustia-llc/catgraph/issues/284)); measured,
  replacing `unname` with discard-inputs/create-outputs junk left 44/44 green.
  It now carries content pins routed through `frobenius_to_cospan` +
  `canonical_form`: cup/cap against a hand-built bent identity, both Eq. (13)
  snakes against `id_X`, `name`/`unname` against leg-bending computed on the
  cospan, Prop 3.3 against `name(f;g)`, Prop 3.4's explicit-comp helper against
  `f`, and the `id ⊗ cap ⊗ id` comp factor against `equivalence::comp_cospan`.
  Each was falsified by reverting the corresponding production line.

### Known discrepancy — scalars (bubbles) — **CLOSED within this same release by [#350](https://github.com/sustia-llc/catgraph/issues/350)**

- Scalars were not preserved across the `Cospan` ↔ `FrobeniusMorphism`
  translation: `cospan_to_frobenius`'s identity fast path discarded every apex
  vertex neither leg reaches, and `two_layer_simplify`'s rule 3 cancelled a
  spelled `η;ε` before `frobenius_to_cospan` ran. The first cause was closed by
  [#285](https://github.com/sustia-llc/catgraph/issues/285) and the second by
  [#350](https://github.com/sustia-llc/catgraph/issues/350), both in this
  release; the pin is
  `cospan_algebra::tests::scalar_bubbles_survive_in_both_directions`.

### Added

- `frobenius::frobenius_to_cospan` — interpret a `FrobeniusMorphism` as the
  `Cospan` it denotes, the semantics half of the Prop 3.8 correspondence
  ([#283](https://github.com/sustia-llc/catgraph/issues/283)). Superseded within
  this same release by
  [#336](https://github.com/sustia-llc/catgraph/issues/336): this path is a
  re-export of `cospan_algebra::frobenius_to_cospan`.
- `tests/frobenius_axioms.rs` (16 tests) — the Def 2.5 equations built on both
  sides and decided for four named carriers over three decision paths, with
  riders for the zigzag identities and for the braiding being a genuine crossing
  ([#283](https://github.com/sustia-llc/catgraph/issues/283)). Not every
  implementor: `catgraph-applied`'s `PetriNet` and `DecoratedCospan` also
  implement `HypergraphCategory` and have no Def 2.5 pin.
- `Debug` on `FrobeniusMorphism` and `FrobeniusOperation` — purely additive
  ([#283](https://github.com/sustia-llc/catgraph/issues/283)).
- `tests/corel.rs::composites_induce_the_expected_partition` — `Corel`
  composites pinned by their whole class structure
  ([#283](https://github.com/sustia-llc/catgraph/issues/283)).

### Fixed

- The Frobenius normalizer no longer fuses two spiders that share no wire
  ([#283](https://github.com/sustia-llc/catgraph/issues/283)). Rule 4 matched
  `Spider(z, m, n)` against `Spider(z, n, k)` with no lower bound on `n`, so
  `Spider(z, 2, 0)` followed by `Spider(z, 0, 2)` collapsed into
  `Spider(z, 2, 2)`, wiring two disconnected components together. Rule 4 now
  requires `n >= 1` and the `target_side_placement` lookup excludes zero-output
  blocks. ⚠ The pin,
  `tests/frobenius_axioms.rs::spider_fusion_needs_a_wire_between_the_two_spiders`,
  covers the conjunction: measured, deleting either defence alone leaves
  `cargo test -p catgraph` green.

### Changed

- `CospanAlgebraMorphism`'s `Clone` is hand-written and no longer requires
  `A: Clone`, the algebra being held behind an `Arc`
  ([#283](https://github.com/sustia-llc/catgraph/issues/283)). Strictly a
  loosening.
- The Thm 3.14 freeness claims in `hypergraph_category.rs` are scoped to the
  deferral recorded in `docs/FS19-AUDIT.md`
  ([#277](https://github.com/sustia-llc/catgraph/issues/277)). Prose only.
- The arity-only Frobenius tests are renamed to say so
  ([#283](https://github.com/sustia-llc/catgraph/issues/283)):
  `unitality_left` → `unitality_left_arities`, and similarly `counitality_left`,
  `associativity`, `frobenius_law` (→ `frobenius_law_lhs_arities`),
  `special_frobenius`, `zigzag_via_trait` and `frobenius_morphism_special` in
  `src/hypergraph_category.rs`, plus the H_Part trio in `tests/equivalence.rs`.
  Test names only. All 19 tests in the src mod stay green under a non-merging μ
  with a non-splitting δ on `Cospan`'s `HypergraphCategory` impl; the H_Part
  trio's own mutant is `multiplication_in`'s right leg `[0, 0, 0]` → `[0, 1, 0]`,
  under which the trio stays green while
  `cospan_algebra_morphism_battery`, the bubble ledger and the zigzag rider go
  red.
- clippy 1.98 compatibility
  ([#340](https://github.com/sustia-llc/catgraph/issues/340)): the six
  `#[allow(clippy::from_iter_instead_of_collect)]` in `span.rs` named a removed
  lint, and the `HashSet::from_iter` calls are now `.collect()`. No behaviour
  change.

### Fixed

- `cospan_algebra::cospan_to_frobenius` no longer collapses an all-merged cospan
  to the identity ([#285](https://github.com/sustia-llc/catgraph/issues/285)).
  Its fast path fired on `domain == codomain && left_leg == right_leg`, which
  the single-apex cospan `m → {•} ← m` also satisfies. Measured:
  `[a,a] → {•} ← [a,a]` returned depth 1 where the correct answer is
  `special_frobenius_morphism(2, 2, 'a')` (depth 2), and `[a,a,a] → {•} ← [a,a,a]`
  depth 1 against depth 4. The guard now also requires the common leg to be a
  bijection onto the apex, so `[a] → {•,•} ← [a]` takes the general route and —
  with rule 3 deleted at #350 — comes back as a depth-2 term rather than
  `identity(['a'])`. Behaviour change, not an API change: `cospan_to_frobenius`,
  `CospanToFrobeniusFunctor::map_mor` and `NameAlgebra::map_cospan` return
  different morphisms for the affected cospans.

### Added

- Content-level regression pins for the `Cospan → Frobenius` functor
  ([#285](https://github.com/sustia-llc/catgraph/issues/285)). The `ctf_*` suite
  compared only `domain()` / `codomain()` — measured, an implementation mapping
  every `m → n` cospan to `special_frobenius_morphism(m, n, z)` kept 7 of the 10
  `ctf_*` tests green. New and strengthened tests compare whole morphisms:
  `ctf_single_apex_cospan_is_the_spider` (the 5×5 grid),
  `ctf_disconnected_cospan_is_the_tensor_not_a_spider`,
  `ctf_functoriality_composition_content`, content assertions on `F(id)`, `F(η)`,
  `F(ε)`, `F(μ)`, `F(δ)`, and
  `ctf_single_apex_cospan_round_trips_up_to_canonical_form`, which ranges over
  all 25 grid cells.
- Leg-by-leg Prop 4.6 and Lemma 4.9 witnesses
  ([#285](https://github.com/sustia-llc/catgraph/issues/285)):
  `arb_mixed_part_element` emits two apex classes sharing the label `'a'`, so
  `right_to_middle` carries information the codomain does not, and the Lemma 4.9
  witnesses moved off identity morphisms onto `μ`, `δ` and `μ ; δ`.
- `tests/common::assert_frobenius_eq_msg` / `frobenius_shape` — `assert_eq!` is
  unavailable on `FrobeniusMorphism`, which has `PartialEq` but no `Debug`;
  these report `depth`/`domain`/`codomain` and each side's
  `frobenius_to_cospan(..).canonical_form()`
  ([#285](https://github.com/sustia-llc/catgraph/issues/285)).

## [workspace-v0.15.0] - 2026-08-16

### Changed — BREAKING

- `SymmetricMonoidalMorphism::from_permutation(p, types, types_as_on_domain: bool)`
  is replaced by `from_permutation_on_domain(p, types)` and
  `from_permutation_on_codomain(p, types)`
  ([#258](https://github.com/sustia-llc/catgraph/issues/258)). Both build the
  wiring domain wire `i` → codomain wire `p.apply(i)` and differ only in which
  boundary `types` labels: `on_domain` gives `domain() == types` and
  `codomain()[k] == types[p.inv().apply(k)]`; `on_codomain` gives
  `codomain() == types` and `domain()[i] == types[p.apply(i)]`. Migration is
  mechanical.
- `SymmetricMonoidalDiscreteMorphism::from_permutation` loses its `bool` and
  keeps one constructor, `Decomposition::from_permutation(p, n)`
  ([#258](https://github.com/sustia-llc/catgraph/issues/258)).
- `Span::from_permutation_on_codomain` realizes `p`, where the old
  `types_as_on_domain = false` branch realized `p⁻¹`
  ([#258](https://github.com/sustia-llc/catgraph/issues/258)). On
  `rotation_left(3, 1)` over `['a','b','c']` it reported a domain of
  `['c','a','b']` where `Cospan` reports `['b','c','a']`. The cached identity
  flags on the result are swapped relative to the old branch.
- `CospanAlgebraMorphism`'s permutation constructors were not braidings and now
  are ([#258](https://github.com/sustia-llc/catgraph/issues/258)): the labels
  were inverted (`p.permute(types)` where the cospan family uses
  `p.inv().permute(types)`), and the structural cospan was built over a
  `2n`-vertex apex with a bijective right leg, giving the all-singletons
  partition. The apex is now `n` vertices and the right leg degenerates to
  `identity_in`'s exactly when `p` is the identity.
- `NamedCospan::from_permutation_extra_data` splits the same way and is now
  fallible: `from_permutation_extra_data_on_domain` / `..._on_codomain`, each
  returning `Result<Self, CatgraphError>`, replacing an `.unwrap()` in
  production code and an `assert_eq!(types.len(), prenames.len())`
  ([#258](https://github.com/sustia-llc/catgraph/issues/258)).
- Arity mismatch is an `Err` on every permutation constructor, not a panic
  ([#258](https://github.com/sustia-llc/catgraph/issues/258)). `Cospan` used
  `assert_eq!`; `Span` and `FrobeniusMorphism` had no check and indexed out of
  bounds inside the `permutations` crate. `Corel`, `DecoratedCospan`, `PetriNet`
  and `NamedCospan` inherit the fix by delegation.
- `NamedCospan`'s `SymmetricMonoidalMorphism` constructors still fail
  unconditionally, now naming
  `from_permutation_extra_data_on_domain` / `..._on_codomain` as the replacement
  ([#258](https://github.com/sustia-llc/catgraph/issues/258)).
- `permute_side` splices `β(p)` on the codomain and `β(p⁻¹)` on the domain on
  every carrier; `Cospan`, `Corel`, `NamedCospan`, `Span`, `FrobeniusMorphism`,
  `CospanAlgebraMorphism` and `Decomposition` all realized the inverse before
  ([#258](https://github.com/sustia-llc/catgraph/issues/258)). Measured on
  `identity(types).permute_side(rotation_left(3,1), true)`:

  | carrier | result |
  |---|---|
  | `Cospan` / `Corel` / `NamedCospan` / `DecoratedCospan` (before) | `[2, 0, 1]` = `β(p⁻¹)` |
  | `FrobeniusMorphism` (before) | `β(p⁻¹)` |
  | `MatR` / `MatKron` / `PropExpr` (unchanged) | `[1, 2, 0]` = `β(p)` |

  The contract is stated on the trait's `permute_side` rustdoc: the wire at slot
  `i` of the permuted side moves to slot `p.apply(i)`, i.e.
  `self ; from_permutation_on_domain(p, &self.codomain())` on the codomain and
  `from_permutation_on_codomain(p.inv(), &self.domain()) ; self` on the domain.
  **Migration:** callers passing `p` and wanting the old behaviour pass
  `p.inv()`; callers pairing `permute_side` with
  `catgraph::utils::necessary_permutation` need `p.inv()`, as
  `catgraph-applied`'s `WiringDiagram::operadic_substitution` was adapted.
  Single-sorted carriers are unaffected. On `Decomposition` the change reads
  `p ∘ f` on the codomain and `f ∘ p⁻¹` on the domain.
  `PetriNet::permute_side` is not part of this change: its `p` is sized by the
  transition count.

### Fixed

- `Span::permute_side` was incoherent with itself and produced an invalid span
  ([#258](https://github.com/sustia-llc/catgraph/issues/258)). It moved each
  apex pair by `p.apply` **and** permuted the word by `p`, which must be
  mutually inverse, so
  `Span::identity(&['a','b','c']).permute_side(&rotation_left(3,1), true)` wired
  domain `'a'` to a codomain slot labelled `'c'`. Release-silent, `assert_valid`
  being `debug_assert!`-only. The word is now permuted by `p.inv()`.
- `CospanAlgebraMorphism::permute_side` left `self.element` stale
  ([#258](https://github.com/sustia-llc/catgraph/issues/258)), permuting only
  the two label words, so `compose` fed a stale element through `comp_cospan`
  built from the new labels. It now pushes the element through the relabelling
  cospan `(X ⊕ Y) → (X' ⊕ Y')` and gains the defensive length no-op `MatR` and
  `PropExpr` have.
- `CospanAlgebraMorphism`'s permutation constructor was O(n²)
  ([#258](https://github.com/sustia-llc/catgraph/issues/258)):
  `p_inv.permute(&(0..n).collect())` ran inside the per-index closure, and is
  replaced by a direct `p_inv.apply(k)`.
- `PropExpr::from_permutation`'s rustdoc claimed generic code got `p` on
  `PropExpr`/`MatR` and `p⁻¹` on `Cospan`/`Corel`
  ([#258](https://github.com/sustia-llc/catgraph/issues/258)). Every carrier in
  the workspace realizes `p`; the `p.inv()` in `Cospan`'s builder is the right
  leg index vector, and that inversion is what keeps the wiring un-inverted.

### Added

- A cross-carrier braiding test
  (`catgraph-applied/tests/braiding_cross_carrier.rs`,
  [#258](https://github.com/sustia-llc/catgraph/issues/258)) driving every
  permutation of `n = 3` and `n = 4` through both constructors of every
  implementation of both traits, plus `Decomposition` via `from_decomposition`.
  No assertion compares two carriers to each other; each is compared against a
  reference computed from `p`, itself pinned against hand-written values.
- `permute_side` coverage in the same file
  ([#258](https://github.com/sustia-llc/catgraph/issues/258)): a hand anchor for
  `rotation_left(3, 1)`, `permute_side` on an identity matched against the
  constructor it must equal on both sides for every permutation of `n = 3` and
  `n = 4`, the composite law `β(q).permute_side(p, true) == β(q ; p)` over every
  ordered pair at both arities, and the conjugation law. `NamedCospan` is
  included and `PetriNet` excluded until v0.17.0
  ([#272](https://github.com/sustia-llc/catgraph/issues/272),
  [#275](https://github.com/sustia-llc/catgraph/issues/275)).

## [workspace-v0.14.0] - 2026-08-16

### Fixed — from the #256/#261 code review

- `CospanCanon::from_parts` no longer sizes its occurrence tally from an
  unvalidated `dom_len`/`cod_len`: a corrupt `dom_len` of `1 << 40` passed the
  sortedness and ascending checks and then allocated an 8 TB tally. A
  cardinality comparison runs first, reporting the new
  `CatgraphError::CanonPreimageCardinalityMismatch`;
  `CanonPreimageNotAPartition` remains the finer report and is pinned by two new
  fixtures.
- `Span::new` reports `CatgraphError::ConstructionMiddlePairOutOfBounds` instead
  of reusing `ConstructionIndexOutOfBounds`, whose `(leg, position)` fields
  mis-locate a span's offending middle pair.
- CI runs `cargo test -p catgraph --lib --release`, so the three
  `#[cfg(not(debug_assertions))]` tests pinning that `new_unchecked` is
  `debug_assert!`-only are covered.

### Changed — BREAKING

- `Cospan::new` and `Span::new` are validated constructors returning
  `Result<Self, CatgraphError>`; the previous infallible bodies moved to
  `Cospan::new_unchecked` / `Span::new_unchecked`
  ([#256](https://github.com/sustia-llc/catgraph/issues/256)). `Cospan::new`
  checks every `left` and `right` entry against `middle.len()`; `Span::new`
  checks each middle pair's `.0` against `left.len()` and `.1` against
  `right.len()`, **and** that the two labels it names agree. **Migration:**
  `new_unchecked` for data correct by construction, `new` for data crossing a
  trust boundary, `.unwrap()` in tests.
- `NamedCospan::new` is a validated constructor returning
  `Result<Self, CatgraphError>`; the previous infallible body moved to
  `NamedCospan::new_unchecked`
  ([#256](https://github.com/sustia-llc/catgraph/issues/256)). It checks one
  name per port — previously `assert!`s, so they aborted the process in every
  profile — and delegates the leg bounds to `Cospan::new`. Name counts are
  checked before leg bounds, domain before codomain. `NamedCospan::empty` moved
  to `new_unchecked`, unchanged behaviourally; `new_unchecked` is uniformly
  `debug_assert!`-only, including the name counts.
- `CatgraphError` is `#[non_exhaustive]`, so downstream `match`es must carry a
  wildcard arm and a later variant is not a breaking change.

### Added

- `CatgraphError::ConstructionIndexOutOfBounds { leg, position, target,
  target_len }` — a cospan leg entry targets an index outside the apex. Raised
  by `Cospan::new`, and through it by `NamedCospan::new`.
- `CatgraphError::ConstructionMiddlePairOutOfBounds { leg, pair_position,
  target, target_len }` — a span middle pair names a boundary index outside the
  boundary set. Raised by `Span::new`. ⚠ A span does not raise
  `ConstructionIndexOutOfBounds`: `pair_position` indexes the middle-pair list,
  and `leg` says which half of the pair was out of range.
- `CatgraphError::ConstructionLabelMismatch { position, left_index,
  right_index, left_label, right_label }` — a `Span` middle pair links a domain
  element to a codomain element carrying a different label.
- `CatgraphError::ConstructionNameCountMismatch { leg, boundary_len,
  name_count }` — a `NamedCospan` port-name list whose length does not match its
  boundary. Raised by `NamedCospan::new`.
- `errors::BoundaryLeg` — `Domain` / `Codomain`, with `as_str()` and `Display`.
  Not `#[non_exhaustive]`.
- `CospanCanon::from_parts`, `CospanCanon::to_cospan` and `ApexClass::new` —
  purely additive round-trip surface
  ([#261](https://github.com/sustia-llc/catgraph/issues/261)). `ApexClass::new`
  is infallible and non-validating; `from_parts` re-establishes all three
  invariants (classes sorted under `ApexClass`'s `Ord`, each preimage strictly
  ascending, the preimages partitioning `0..dom_len` / `0..cod_len`) and rejects
  rather than repairs; `to_cospan` rebuilds a witnessing cospan with the apex in
  canonical order, placing scalars although no leg reaches them, so `k` bubbles
  round-trip as `k`. The invariants are sufficient to rebuild a witness, so
  `c.canonical_form().to_cospan().canonical_form() == c.canonical_form()` is a
  property test for the validation.
- Five `CatgraphError` variants for canonical-form construction, all raised by
  `CospanCanon::from_parts` and all reusing `BoundaryLeg`:
  `CanonClassesNotSorted { position }`;
  `CanonPreimageNotAscending { leg, class_position, position }`;
  `CanonPreimageOutOfBounds { leg, class_position, position, index,
  boundary_len }`; `CanonPreimageCardinalityMismatch { leg, total,
  boundary_len }`; and `CanonPreimageNotAPartition { leg, index, occurrences,
  boundary_len }`, where `occurrences` is `0` for an unclaimed boundary index
  and `>= 2` for one claimed by several classes. ⚠
  `CanonPreimageCardinalityMismatch` is the commonest failure on a corrupt
  reload, the partition tally running only once the totals agree.

### Fixed

- `Span::assert_valid` no longer panics in release: its label-agreement check
  was computed into a `let` before being handed to `debug_assert!`, so the
  indexing ran in every profile. Both `assert_valid` methods now write every
  check inside its `debug_assert!`. Debug behaviour is unchanged, bounds still
  reported before labels.

## [workspace-v0.12.0] - 2026-08-15

### Added

- `cospan_canon::ApexClass<Λ>` and `CospanCanon::classes` — a read surface for
  the apex signatures that discriminate a canonical form
  ([#254](https://github.com/sustia-llc/catgraph/issues/254)). `classes()`
  returns `&[ApexClass<Λ>]`, and each class exposes `label()`,
  `dom_preimage()`, `cod_preimage()` (both sorted ascending, a documented
  invariant) and `is_scalar()`. Read-only: there is no `CospanCanon::from_parts`
  and no `ApexClass::new` in this release
  ([#261](https://github.com/sustia-llc/catgraph/issues/261)). No serde. The
  canonical form itself is unchanged — `Eq`, `Hash` and sort order are
  bit-identical, `ApexClass` replacing an anonymous
  `(Λ, Vec<usize>, Vec<usize>)` tuple in a private field with the same field
  order and derives. `scalar_count` is rewritten as a filter on `is_scalar()`
  with signature, behaviour and docs unchanged.

## [workspace-v0.10.0] - 2026-08-09

### Changed

- `cospan::test::permutatation_manual` uses a literal payload instead of
  `rand::random()` ([#232](https://github.com/sustia-llc/catgraph/issues/232)).
  Test-only. `rand::random()` needs `rand`'s `thread_rng` feature, which the
  slimmed workspace declaration no longer enables in physics-free builds. `rand`
  remains a dev-dependency only, carrying `std`/`std_rng` on its own dev edge.

## [workspace-v0.9.0] - 2026-08-04

### Changed

- `MorphismSystem`'s topological sort is the crate's own and the `ultragraph`
  dependency is gone
  ([#220](https://github.com/sustia-llc/catgraph/issues/220), D2 of
  [#218](https://github.com/sustia-llc/catgraph/issues/218)). `topological_sort`
  is replaced by a private Kahn's-algorithm pass in
  `frobenius/morphism_system.rs`, so the crate depends on no graph crate;
  `union-find` moves to a workspace dependency. Behaviour is unchanged at both
  call sites — a cycle still yields `CatgraphError::Interpret` and a valid
  topological order still places each parent before its children, though the
  order among equally-valid ones may differ. The private `resolve_order` drops
  its `Result` wrapper; no public signature changes. `toposort` is covered for
  chains, diamonds, isolated nodes, duplicate edges, two-node cycles,
  self-loops, and a cycle beside an acyclic component.

## [workspace-v0.6.0] - 2026-08-02

### Changed

- `equivalence::comp_cospan`'s index arithmetic is documented as bounded, and
  the left leg's own length `m + 2n + k` is written as
  `middle.len().saturating_add(n)`
  ([#196](https://github.com/sustia-llc/catgraph/issues/196)). No saturating
  sentinel is introduced.

## [workspace-v0.4.0] - 2026-07-25

### Changed

- Hardened the core-crate rayon determinism guards
  ([#48](https://github.com/sustia-llc/catgraph/issues/48)).
  `tests/rayon_equivalence.rs` upgraded from set-shape / depth-only checks to
  exact assertions: `NamedCospan::find_nodes_by_name_predicate` against an
  in-test sequential reference below and above its size threshold, plus the
  `at_most_one=true` short-circuit and no-match cases. `FrobeniusMorphism` /
  `FrobeniusLayer` `hflip` gains `#[cfg(test)]` unit tests in
  `src/frobenius/operations.rs` asserting sequential-reference equality and the
  `hflip ∘ hflip == id` involution on layers wide enough that rayon subdivides,
  plus public-API determinism guards through `special_frobenius_morphism` and
  `cospan_algebra::cospan_to_frobenius`. In `catgraph-applied`,
  `LinearCombination::linear_combine` gains threshold-straddling par-vs-seq
  tests. All guards run under the default and `--no-default-features` builds.
- Paper-audit citation reconciliation (Phase 1, PRs #112/#113): the FS19 anchors
  verified against the cached paper, fixing the `Thm 1.2 / Thm 4.13`
  isomorphism-vs-equivalence phrasing, `FS19-AUDIT.md` count drift, Lemma 4.3's
  "io" qualifier, and `RelabelingFunctor` re-cited as the single-map component
  of Prop 2.1 / Cor 3.13. `operadic.rs` grounded in its `1305.0297` anchor and
  FS18 (`1803.05316`) declared a secondary core anchor.
  `tests/spider_theorem.rs` upgraded to full structural-equality assertions.

### Added

- `scripts/check_audit_counts.py` CI guard (#111) — checks audit-doc tallies
  (summary arithmetic, headline percentages, per-section emoji counts,
  `(N tests)` citations) for self-consistency, for `FS19-AUDIT.md`, then
  `FS18-AUDIT.md` and `BV25-AUDIT.md`.
- `CatgraphError::RecursionLimit { depth, limit }` — shared term-interpreter
  recursion-guard error, so `catgraph-syntax` interpreters whose error type is
  fixed to `CatgraphError` report the same shape
  ([#99](https://github.com/sustia-llc/catgraph/issues/99)).
- `cospan_canon` — `CospanCanon<Λ>` and `Cospan::canonical_form`, a decidable
  (hashable, `Eq`) invariant for parallel cospans up to apex isomorphism,
  recording each apex vertex's `(label, sorted dom preimage, sorted cod
  preimage)` as a sorted multiset so scalars are counted rather than collapsed
  ([#80](https://github.com/sustia-llc/catgraph/issues/80), F&S 2019 Prop 3.8).

> **Reconciliation note
> ([#158](https://github.com/sustia-llc/catgraph/issues/158)).** Workspace tags
> `v0.1.1`, `v0.2.0`, `v0.2.1`, and `v0.3.0` (2026-07-02 → 2026-07-11) were cut
> without per-crate sections here; this crate's changes across them are recorded
> only in git history (`git log v0.1.0..v0.3.0 -- catgraph/`) and the
> workspace-level release record. `v0.5.0` deliberately rolled no section here.

## [workspace-v0.1.0] - 2026-07-01

First monorepo release: workspace-wide tag `v0.1.0` (supersedes the pre-reboot
crate-scoped version lineage below). The coalition semantic-layer handoff to
downstream koalisi.

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
  bottom-up resolution order (`fill_black_boxes`) on the `ultragraph` graph
  substrate via `topological_sort`. `parallel` (default-on) feature for rayon at
  hot call sites; `--no-default-features` yields a slim, single-threaded
  WASI-compatible build.

### Changed

- Graph substrate moved from `rustworkx-core`/`petgraph` to `ultragraph` for
  `MorphismSystem` dependency resolution, dropping the `rustworkx-core` →
  `ndarray` + `serde` transitive chain. The `rustworkx` feature is removed. The
  speculative `Cospan::to_graph` / `NamedCospan::to_graph` petgraph exports were
  removed.

### Notes

- Test posture: 517 (default and `--no-default-features` identical). Zero
  `unsafe`.
- Permanently-deferred paper items (cross-Λ functoriality, strictification,
  §3.3 io/ff factorization, the global Grothendieck form, LinRel examples) are
  catalogued in [`docs/FS19-AUDIT.md`](docs/FS19-AUDIT.md).

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.18.0...HEAD
[workspace-v0.18.0]: https://github.com/sustia-llc/catgraph/compare/v0.17.0...v0.18.0
[workspace-v0.17.0]: https://github.com/sustia-llc/catgraph/compare/v0.16.0...v0.17.0
[workspace-v0.16.0]: https://github.com/sustia-llc/catgraph/compare/v0.15.0...v0.16.0
[workspace-v0.15.0]: https://github.com/sustia-llc/catgraph/compare/v0.14.0...v0.15.0
[workspace-v0.14.0]: https://github.com/sustia-llc/catgraph/compare/v0.13.0...v0.14.0
[workspace-v0.12.0]: https://github.com/sustia-llc/catgraph/compare/v0.11.0...v0.12.0
[workspace-v0.10.0]: https://github.com/sustia-llc/catgraph/compare/v0.9.0...v0.10.0
[workspace-v0.9.0]: https://github.com/sustia-llc/catgraph/compare/v0.8.0...v0.9.0
[workspace-v0.6.0]: https://github.com/sustia-llc/catgraph/compare/v0.5.0...v0.6.0
[workspace-v0.4.0]: https://github.com/sustia-llc/catgraph/compare/v0.1.0...v0.4.0
[workspace-v0.1.0]: https://github.com/sustia-llc/catgraph/releases/tag/v0.1.0
