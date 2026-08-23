# Changelog

All notable changes to `catgraph` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the crate adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Tests (#288: `tests/spider_theorem.rs` widened to the theorem it cites)

- **Thm 6.55 is now pinned at the semantics, not only at the term level**
  ([#288](https://github.com/sustia-llc/catgraph/issues/288)). The file carried
  an "any connected Frobenius diagram on `m` inputs and `n` outputs equals the
  spider `s_{m,n}`" header over five hand-built ≤ 2-layer, single-label,
  braid-free composites at `m, n ≤ 3`, each asserting *structural* equality
  against `special_frobenius_morphism`. No production code changed; this is a
  test-and-docs change.

  - **The over-claim is corrected rather than papered over.**
    `FrobeniusMorphism`'s derived `Eq` is syntactic up to `two_layer_simplify`'s
    four rewrite rules — it is not the Frobenius quotient — so a term-level
    assertion *cannot* be widened to arbitrary connected diagrams. Measured on
    `d6c7bd5`: `(δ ⊗ id);(id ⊗ μ)`, `σ;μ;δ`, `(μ ⊗ id);(δ ⊗ id);(id ⊗ μ)`,
    `(δ ⊗ id);(id ⊗ σ);(μ ⊗ id)`, `(η ⊗ id);μ` and the left-comb `4 → 1` are all
    connected and all structurally ≠ the spider at their arities. The five tests
    are kept, with docstrings that say what they actually pin: each hand-built
    recipe is the recipe `special_frobenius_morphism` itself follows at that
    arity, so they are a *builder-shape* pin.
  - **The theorem is asserted where it is true** — on the image under
    `frobenius_to_cospan`, canonicalised by `Cospan::canonical_form`. (That map
    is *not* a direction of F&S 2019 Prop 3.8, both of whose directions are
    functors *out of* `Cospan`; Prop 3.8 licenses the construction, from
    `Cospan_Λ`'s per-object SCFM structure at Ex 2.8, and the anchor belongs
    there. `cospan_algebra`'s rustdoc also records the map as neither sound nor
    complete against SCFM on scalars — which is the *cause* of the scalar-shaped
    exclusions below, not an aside.) A generated corpus of 2105 terms, built from
    η, ε, μ, δ, σ and `id` alone (never from a `Spider` block, so the builder
    never appears inside a term under test), yields 1280 connected diagrams —
    both labels × `(m, n)` in `0..=4 × 0..=4` minus `(0, 0)` × four decoration
    variants, plus two scripted wide-waist families and seeded random walks;
    recipe depth to 21; 1105 carrying a braiding — and each is asserted to have
    exactly one apex vertex, no scalar class, and preimages covering all of
    `0..m` and `0..n`, *and* to equal the canonical form of
    `special_frobenius_morphism(m, n, z)`. The oracle is independent of the
    builder; the builder is compared to it.
  - **The corpus's one structural bias is measured and answered, not left
    implicit.** A diagram's *interior waist* is the narrowest running codomain
    strictly *between* two of its blocks; a one-wire internal boundary splits it
    into two strictly smaller connected pieces, on which the conclusion follows
    by induction. A narrow *boundary* means no such thing — a `1 → 1` diagram
    that δ's out to four wires, braids and μ's back is fully non-trivial — so
    neither the domain nor the final codomain counts as a cut. The metric is
    evidence and not proof in both directions, and `Built::is_wide_waist` states
    both gaps: a diagram cut is an antichain and need not fall on a recipe layer
    boundary, and at `m = n = 1` the `s_{m,1} ; … ; s_{1,n}` phrase is satisfied
    vacuously by `id ; D ; id` — 15 of the 1030 wide terms sit at that arity,
    census-pinned so the prose bounds the reading instead of estimating it.
    The scripted connected family folds to one wire by construction at
    `m >= 2`, and before the wide families existed only 22 of the 272 connected
    terms had an interior waist ≥ 2 (a further 45 had no internal cut at all).
    Two families answer it: the 16-term `wide_waist_family`
    (comb, braided-comb and folded-comb at widths 2–4 on both labels,
    generalising three of the composites above — `wide_comb_z_2` *is*
    `(δ ⊗ id);(id ⊗ μ)`; `wide_braided_comb_z_2` is
    `(δ ⊗ id);(id ⊗ σ);(μ ⊗ id)`; `wide_folded_comb_z_2` is
    `(μ ⊗ id);(δ ⊗ id);(id ⊗ μ)`), and the 1488-term
    `wide_waist_permutation_family`, which sweeps **every** permutation of the
    `2m` middle wires of a δ-fan folded back into a μ-fan at `m ∈ {2, 3}`,
    `n ∈ {2, 3}` — narrowest internal cut `min(m, n) + 1`, the permutation
    realised as a word of adjacent σ's, and the same recipe shape landing in
    *both* arms of the differential (992 connected, 496 disconnected) according
    to the disjoint-set rather than the oracle. None of the 992 is structurally
    equal to its spider, and each has interior waist exactly `min(m, n) + 1` —
    both asserted per term, not left as prose, and `permutations` /
    `transposition_word` are themselves checked (`n!` distinct outputs; each word
    realising its target) by a new test, since a sweep that silently repeated a
    wiring would keep every other number in the file intact. The connected
    interior-waist histogram is now `{None: 45, 1: 205, 2: 23, 3: 619, 4: 388}`,
    all three buckets pinned by the census, with a floor on the wide one so it
    cannot silently leave. The 16 hand-picked shapes are guarded by *name*
    instead: measured, deleting them leaves every floor green, because the sweep
    out-supplies any count worth flooring.
  - **Connectivity is decided by the recipe, never read off the oracle.** A
    disjoint-set over the construction is carried block by block (μ unions, δ
    propagates, η starts a component, ε consumes a wire without destroying its
    component, and σ permutes wires and unions nothing). The verdict is used in
    both directions: the 716 disconnected recipes are asserted to denote exactly
    their own component count, which makes it a differential rather than an
    unchecked filter. A σ counts as spanning two components only if its two
    sides are *still* distinct when the recipe ends — resolved against the final
    disjoint-set, since a σ whose sides a later μ merges cannot see the merging
    braiding perturbation either (measured: 121 of the 132 counted at braid time
    survived that test).
  - **Excluded by design, and said so:** `m == n == 0`; any recipe that closes a
    component; and any recipe with *no* component at all. The first two sit on
    the *special* vs *extra-special* line — `two_layer_simplify` cancels `η;ε`
    while `Cospan` keeps the bubble as a genuine scalar — which this file does
    not decide; 62 corpus terms fall in that arm. The third is the empty term
    (`[] → []`, depth 0), produced 47 times by walks that started at width 0 and
    never drew an η: on it `apex_len() == components` reads `0 == 0`, true and
    vacuous, and `apex_len() > 1` is simply false, so admitting it would make the
    disconnected arm's own name wrong for 17.6% of its range. It is now counted
    in the census and pinned to be exactly that shape, and no claim test ranges
    over it. Both claim tests additionally assert `scalar_count() == 0` on every
    term they *do* range over, so a surviving scalar reddens the pin.
  - **The census says which of its numbers are the generator's and which are
    not.** Eleven of the twelve counts in
    `the_corpus_is_the_space_these_pins_claim`
    are properties of the generator alone; the twelfth, the number of distinct
    disconnected canonical forms, is computed through `frobenius_to_cospan` +
    `canonical_form` and therefore moves with the production code — measured, it
    goes 203 → 220 under the comultiplication perturbation below, and an `Err`
    from `frobenius_to_cospan` takes the census down with a panic rather than an
    assertion. The docstring says so, instead of pointing a future maintainer at
    the fixture.
  - **Falsified, measured, reverted.** Replacing `generator_to_cospan`'s
    `Comultiplication` arm with a disconnected cospan: 1165 of 1280 connected —
    all 992 connected permutation-family terms among them — and 380 of 716
    disconnected terms disagree. Making the `SymmetricBraiding` arm a merge: on
    mixed labels the layer fold rejects it outright (a merged apex cannot retype
    `[z, w] → [w, z]`), and restricted to same-label σ so every term stays
    type-correct, 397 of 716 disconnected recipes disagree while the connected arm
    stays green by rights. Dropping `wide_waist_permutation_family` (with
    `CORPUS_SIZE` followed down): `MIN_CONNECTED` reddens at 288 of 617, and the
    wide bucket falls to 38. Mirroring
    `special_frobenius_morphism`'s odd-`m` branch to `id ⊗ sfm(m-1, 1)` reddens
    two of the five term-level tests and leaves the semantic pin green — the two
    shapes are SCFM-equal, which is precisely the division of labour between the
    file's two halves.

### Changed — BREAKING (#289: the checked boundary-node mutators)

- **`Cospan` has no cached identity flags**
  ([#289](https://github.com/sustia-llc/catgraph/issues/289)). The private
  `is_left_id` / `is_right_id` fields are deleted. A `Cospan` is its
  `(left, right, middle)` triple and nothing else, and the two questions the
  cache answered are now computed on demand:

  - `Cospan::is_left_identity()` / `is_right_identity()` keep their signatures
    and return `leg.len() == middle.len() && represents_id(leg)` — `O(leg)`
    per call, and **exact in both directions**. The cached answers were not, and
    **an answer can move either way**, so a consumer that reads either accessor
    should re-check both directions rather than only the one below.

    **`false` → `true`.** The maintained flags could only ever *clear* (every
    update was an `&=`; `connect_pair` did not update at all), so a leg that
    genuinely was the identity reported `false` if it had reached that shape by
    mutation rather than by construction. **The largest class by far is
    composites**, which the old entry did not mention:
    `compose_with_quotient` mints its result through `add_middle` — which set
    both flags `false` — and then `add_boundary_node`, whose `&=` cannot undo
    that, so at `0.15.0` **every composite with at least one apex vertex
    reported `false` on both legs**, whatever it actually was.
    `perform_pushout`'s own docs said as much; the changelog did not. Measured
    on `0.15.0` and today:
    `Cospan::identity(&vec!['a']).compose(&Cospan::identity(&vec!['a']))` has
    `is_left_identity() == is_right_identity() == false` there and `true` here.
    (Empty-apex composites are the exception in both releases — with no
    `add_middle` call there was nothing to clear, so `empty ; empty` read
    `true` then too.) Three smaller routes: `identity(&['a', 'b'])`,
    `delete_boundary_node(Left(1))`, `add_boundary_node_known_target(Left(1))`
    reports a left identity again; `from_permutation_on_domain(
    Permutation::identity(n), types)` reports a **right** identity, where the
    constructor used to hard-code `is_right_id: false` for every permutation
    including the identity (its codomain mirror likewise for `is_left_id`); and
    a `permute_side` call with an identity permutation, which used to clear the
    permuted leg's flag unconditionally.

    **`true` → `false`.** The four stale-`true` defects in *Fixed* below run the
    other way, and a consumer relying on one of those answers loses it.
    Measured: `Cospan::identity(&vec!['a'])` followed by
    `add_boundary_node_unknown_target(Right('b'))` gives `([0], [0, 1],
    ['a', 'b'])`, whose `is_left_identity()` read `true` at `0.15.0` — the
    domain leg covers one of two apex vertices — and reads `false` here.
  - `Cospan::assert_valid` **loses both of its `bool` parameters** — the
    signature is now `assert_valid(&self)`. They selected two arms that
    compared a cached flag against the predicate it cached; with no cache
    those arms could only compare `leg_is_identity` with itself. The two
    bounds `debug_assert!`s are unchanged, and the method still compiles away
    entirely in release. `NamedCospan::assert_valid` and
    `NamedCospan::assert_valid_nohash` lose the `check_id: bool` they
    forwarded, for the same reason. Callers drop the arguments; there is no
    behaviour to preserve.
  - **`Cospan`'s derived `Debug` output loses two fields.** The fields were
    private, but `Debug` renders them, so anything that logs, snapshots or
    diffs a formatted cospan — `catgraph-surreal`, and any consumer with a
    `format!("{cospan:?}")` in a golden — sees a different string. Nothing
    in-tree pins it. Measured, on the same value both sides:

    ```text
    0.15.0:  Cospan { left: [0], right: [0, 1], middle: ['a', 'b'], is_left_id: false, is_right_id: true }
    now:     Cospan { left: [0], right: [0, 1], middle: ['a', 'b'] }
    ```

    That is the **final** shape: `Cospan` also gains `PartialEq` / `Eq` in this
    release (see *Added* below), and deriving those does not touch `Debug` —
    re-measured after the derive landed, byte-identical to the line above.

    It carries into every wrapper whose own `Debug` is derived and which holds
    a `Cospan`: `Corel` (a `#[repr(transparent)]` newtype over one) and
    `catgraph-applied`'s `DecoratedCospan`. `NamedCospan` derives only `Clone`,
    so nothing moves there.

  `Span` is untouched: it keeps its own two flags, which are computed
  differently (no boundary-length conjunct — see
  [#345](https://github.com/sustia-llc/catgraph/issues/345)) and are not read
  by `Span::compose`. No `Debug` surface moves there — `Span` derives only
  `Clone`. It also keeps `assert_valid(&self, bool, bool)`, so the two sibling
  types now differ in that method's arity; #345 **axis 2** owns closing that,
  and `Span::assert_valid`'s rustdoc says so.

- **`Cospan::compose` is a function of `(left, right, middle)` alone**
  ([#289](https://github.com/sustia-llc/catgraph/issues/289)).
  `compose_with_quotient` used to hand `self.is_right_id` / `other.is_left_id`
  to `perform_pushout`, which selected its two identity fast paths from them,
  so the composite depended on each operand's **mutation history** as well as
  its state: two structurally equal cospans could compose two different ways.
  `perform_pushout` derives the predicate itself, with the same private
  `leg_is_identity` the accessors use.

  **This changes results**, through one arm only. The `right_leg_id` arm — the
  one `other.is_left_id` used to select — returns field-for-field what the
  union-find body returns for the same input, so entering it or not cannot
  change an answer — confirmed by deleting that arm outright and running the
  whole workspace, **2085 tests, zero failures**. So the operands that move are
  those whose **codomain leg is the identity while the old cache said
  otherwise**, composed with a partner whose domain leg does not first-visit
  its apex in increasing order. Such a composite used to come back with its
  apex permuted and now comes back strict: `identity(&f.domain()) ; f` is `f`
  on the nose for every `f`, not only for the `f.left = [0, 1]` fixtures the
  old suite happened to use. Measured, `id ; f` for
  `f = Cospan::new(vec![1, 0], vec![0, 1], vec!['a', 'b'])`: apex `['a', 'b']`
  now, `['b', 'a']` under the union-find numbering. The correction runs both
  ways relative to `0.15.0`, because that release's cache could be stale in
  either direction: a stale `false` cost a fast path that is now taken, and a
  stale `true` — the defect class below — took one that is now refused.

  Downstream impact is limited to byte-level comparison of such composites;
  `canonical_form` was and stays equal across the change, since both answers
  are legitimate pushouts differing only in apex numbering. No in-tree
  expectation moved, which is precisely why this needed a pin of its own:
  `tests/compose_identity_arms.rs`, whose five tests all go red when the
  `left_leg_id` arm is disabled.

  One further fact the review turned up and the pins now record: when **both**
  legs are the identity, the `left_leg_id` arm is tried first, and its
  `Right(..)` representative tags mean the composite keeps the **right**
  operand's apex labels. `composable` has already forced the two apexes equal
  under `Lambda`'s `Eq`, so under every label type in this workspace there is
  nothing to observe — but `Cospan` requires only `Eq`, never that `Eq` be
  identity, so for a label carrying provenance it does not compare on the arm
  order is visible. It is unchanged by this release and is now pinned rather
  than incidental.

- **The `add_boundary_node` family is checked, and the raw path is
  `*_unchecked`** ([#289](https://github.com/sustia-llc/catgraph/issues/289)).
  The #256/#261 arc fenced the *constructors* (`new` → `Result`), but these
  mutators re-opened the same hole one call later on an already-valid value:
  `Cospan::add_boundary_node` pushed a caller-supplied apex index into the leg
  with **no check at all** — weaker than `new_unchecked`, which at least
  `debug_assert!`s it.

  - `Cospan::add_boundary_node` and
    `Cospan::add_boundary_node_known_target` now return
    `Result<Either<LeftIndex, RightIndex>, CatgraphError>`, raising
    `ConstructionIndexOutOfBounds` when an `Left(tgt_idx)` target is at or
    beyond `middle.len()`. The variant names the leg, the **position the node
    would have taken**, the target and the apex size. On `Err` the cospan is
    left exactly as it was.
  - `Cospan::add_boundary_node_unchecked` is new: the raw path, with
    `new_unchecked`'s posture (a `debug_assert!`, no release cost). The one
    in-crate caller that needs it is the pushout builder in
    `compose_with_quotient`.
  - `Cospan::add_boundary_node_unknown_target` keeps its infallible signature.
    It mints the apex vertex itself, so it has no precondition a `Result`
    could report.
  - `NamedCospan::add_boundary_node` and **both** its `_known_target` /
    `_unknown_target` wrappers return the same `Result`, and
    `NamedCospan::add_boundary_node_unchecked` is the raw path. The method used
    to mean two different things by its two invariants — a duplicate port name
    aborted the process through a bare release `assert!`, an out-of-bounds apex
    index was not checked at all. Both are now `Err`s, the name is checked
    first, and neither the name list nor the leg is written on `Err`.
  - `NamedCospan::add_middle` returns the new `MiddleIndex`, as
    `Cospan::add_middle` always has. It previously discarded it.
  - `Span::add_boundary_node` **keeps its infallible signature** and gains no
    `_unchecked` sibling. A span's legs point *out* of the apex, so this
    mutator takes a **label** (`Either<Lambda, Lambda>`), not an index: there
    is no argument to bounds-check, appending one leaves every existing middle
    pair in bounds and label-agreeing, and the identity flags are computed from
    the middle pairs alone, which the call does not touch. A `Result` here
    would have been permanently `Ok`, so every caller would write a `?` or an
    `.expect(..)` for an error that cannot occur. `Span::add_middle`, below, is
    the `Span` mutator with real preconditions, and it does return a `Result`.

    `Span`'s identity flags carry no boundary-length conjunct, so
    `identity(&['a', 'b'])` followed by `add_boundary_node(Left('c'))` still
    reports `is_left_identity() == true`. That is pinned as the *current*
    contract in `tests/checked_mutators.rs`
    (`span_identity_flag_ignores_the_boundary_length`, which asserts the
    `Cospan` shape beside it for contrast) and must be inverted when
    [#345](https://github.com/sustia-llc/catgraph/issues/345) lands; #345 is a
    flag-semantics change with no error arm, so it is not a reason to
    pre-emptively widen this return type either. Deleting `Cospan`'s cache
    widens #345 rather than closing it: the two types now differ both in the
    missing conjunct and in whether the answer is cached at all.

- **`CatgraphError::ConstructionDuplicatePortName`** is a new variant (the enum
  is `#[non_exhaustive]`, so this is additive for `match`es that already carry
  a wildcard). It carries `leg` and the `existing_position` of the port that
  already holds the name — not the name itself, since port names are only
  bounded by `Eq`.

- **`finset::from_cycle` validates its cycle**, in every build profile and
  before any recursion: every element must be `< n`, and the elements must be
  pairwise distinct. Both were previously accepted. An out-of-range element
  reached a bare `assert!(i < n && j < n)` inside `permutations 0.1.1` from a
  recursive call (and a cycle shorter than 2 short-circuited before even that,
  so `from_cycle(3, &[7])` returned the identity); a repeated element silently
  returned a permutation that is **not** the documented cycle —
  `from_cycle(3, &[0, 1, 0])` is the identity, and no 3-cycle exists on the two
  distinct elements it names. Callers passing malformed cycles now panic with a
  message naming the function and the cycle.

- **`utils::remove_multiple` deduplicates its index list**, so a repeated index
  names one element and removes it once. Previously `to_remove = [3, 3]`
  removed index 3 and then index 3 *of the shortened vector*, silently deleting
  the element that had been at 4 — or panicked with a bare slice message when 3
  had been the last index. It also bounds-checks, naming the offending index
  and the length. Every in-crate and in-workspace caller already passed
  distinct in-range indices, so no behaviour they relied on changes.

### Fixed (#289)

- **`Cospan`'s identity accessors cannot go stale**
  ([#289](https://github.com/sustia-llc/catgraph/issues/289)) — because there
  is nothing left to go stale. See *`Cospan` has no cached identity flags*
  under **Changed — BREAKING** above for the API surface; this entry records
  the defect class that motivated deleting it.

  `is_left_id` / `is_right_id` were documented to mean
  `leg.len() == middle.len() && represents_id(leg)`. **Four** writers were
  responsible for keeping that true and each failed differently — three by
  re-spelling the predicate by hand and dropping part of it, one by not
  updating at all — and three of the four are reachable through the fully
  checked API, with no `_unchecked` call and no malformed input:

  - `add_boundary_node`'s `Left(idx)` arms tested `leg.len() - 1 == tgt_idx`.
    On an identity cospan the only index that satisfies it is `middle.len()`
    itself, so the old code pushed an out-of-range entry **and kept the flag
    `true`**.
  - `add_boundary_node`'s `Right(label)` arms grow the **apex**, and updated
    only the flag of the leg they push to. The partner leg kept its length
    while the apex gained a vertex, so a legitimately-`true` flag survived on
    a leg now strictly shorter than the apex. Inherited by `NamedCospan`.
  - `delete_boundary_node` tested `z == leg.len() - 1`. Deleting the last port
    of an identity cospan shortens the leg without shrinking the apex.
  - `connect_pair` left both flags alone. A merge shrinks the apex while both
    legs keep their length, so a `true` pair survived over an apex the legs
    are now *longer* than. Reachable through `WiringDiagram::connect_pair` in
    `catgraph-applied`.

  This was not cosmetic while `perform_pushout` fast-pathed on the flags and
  sized its reindexing map from the partner's apex: a stale `true` was a wrong
  composition. Measured then, and kept here because they are the evidence the
  class mattered — none of them is reproducible on the shipped code, which has
  no flag to make stale. `identity(&['a','b']).delete(Right(1)).compose(&id_a)`
  panicked in `compose_with_quotient` at
  `left_to_pushout[*target_in_self_middle]` with `index out of bounds: the len
  is 1 but the index is 1`; `Cospan::new(vec![0], vec![0], vec!['a', 'x'])`
  composed with `Cospan::new(vec![0], vec![], vec!['a'])` +
  `unknown_target(Right('b'))` panicked the same way at
  `right_to_pushout[*target_in_other_middle]`, and with that operand's codomain
  port then deleted so nothing indexed out of range, composed *silently* to the
  apex `['a', 'x']` against a reference `['a', 'x', 'b']`; and
  `Cospan::new(vec![0, 1], vec![0, 1], vec!['a', 'a'])` →
  `connect_pair(Left(0), Left(1))` → `compose(&identity(&['a', 'a']))`
  *silently* returned `right == [0, 1]` over `['a', 'a']` against a reference
  `right == [0, 0]` over `['a']` (review R2-01).

  Pinned in `tests/checked_mutators.rs`, which now asserts legs, apexes and
  composites against hand-written expectations —
  `cospan_identity_accessors_need_the_leg_to_cover_the_whole_apex` names the
  `leg.len() == middle.len()` conjunct all four defects dropped, and reddens
  when it is deleted from the private `leg_is_identity` (9 of that file's 27
  tests do, `perform_pushout` reading the same predicate).

  **Two earlier drafts of this entry described observables the release does not
  have**, corrected here rather than quietly dropped.

  - It said `connect_pair`'s flag recompute could turn a flag **on**, so a
    composite built after a merge might come back with a different (isomorphic)
    apex order than one built before — `structurally_equal` false,
    `canonical_form` equal — and advised byte-level consumers to compare
    canonical forms. The composite never depended on the flag once
    `perform_pushout` derived the predicate, and now there is no flag at all.
    The R3-03 fixture (`Cospan::new(vec![1, 2], vec![0, 1], vec!['b','a','a'])`
    → `connect_pair(Left(0), Left(1))`, composed with
    `Cospan::new(vec![1, 0], vec![0, 1], vec!['a', 'b'])`) has exactly one
    answer, `left = [0, 0], right = [0, 1], middle = ['a', 'b']`; the
    `left = [1, 1], right = [1, 0], middle = ['b', 'a']` alternative is
    unreachable. Byte-level comparison of composites is sound for operands that
    are byte-equal.
  - It said the `add` / `delete` flags stay **conservative in the false
    direction**, so `identity(&['a', 'b'])` → `delete_boundary_node(Left(1))` →
    `add_boundary_node_known_target(Left(1))` left `is_left_identity()` `false`
    where a fresh `Cospan::new` of the same three vectors said `true`. That was
    the last thing the cache still did, and deleting it is what closed it: that
    sequence now reports `true`. It is the one shape where a consumer can see
    an accessor answer change, and it is the reason
    `Cospan::structurally_equal`'s "the flags can make structurally equal
    cospans compare unequal" caveat is gone from its docs.

- **`Cospan::connect_pair` merges the two ports in every argument order**
  ([#289](https://github.com/sustia-llc/catgraph/issues/289), found by the
  review of the flag fix above). Its leg remap wrote node 1's *old* apex index
  after `swap_remove` had moved that vertex into node 2's slot, so whenever
  node 1's vertex was the **last** apex index (and node 2's was not) both legs
  received an entry equal to `middle.len()` and the two ports were never
  merged — a pre-existing defect, silent in every profile (`connect_pair` ran
  no `assert_valid`; it does now, so the same regression would trip the bounds
  assertion in debug), and invisible to every in-tree caller because each
  passes the lower apex index first or merges away the last vertex. Reachable
  unchanged through `NamedCospan::connect_pair` and
  `WiringDiagram::connect_pair` (`catgraph-applied`): mint a port with
  `add_boundary_node_unconnected` — it lands on the last vertex — then connect
  it, passing the new port first. Measured before the fix:
  `Cospan::new(vec![0, 1], vec![0, 1], vec!['a', 'a'])` →
  `connect_pair(Left(1), Left(0))` gave `left = [1, 0], right = [1, 0]` over a
  1-vertex apex with `map_to_same(Left(0), Left(1))` false (and `Cospan::new`
  of the result refuses it with `ConstructionIndexOutOfBounds`); the same on
  the named surface gave `left = [1, 0], right = [1]`; on a 3-vertex apex
  `connect_pair(Left(2), Left(0))` gave `left = [2, 1, 0]` over 2 vertices
  (review R3-01). The remap now keeps the vertex node 1 maps to wherever the
  `swap_remove` left it, in one loop over both legs. Pinned in
  `tests/checked_mutators.rs` on the `Cospan` and `NamedCospan` surfaces,
  against hand-written legs and `map_to_same` — a rebuild of the mutated
  value cannot see a remap error.

- **`Cospan::assert_valid` no longer rejects valid cospans**
  ([#289](https://github.com/sustia-llc/catgraph/issues/289)). Its strong arm
  compared `represents_id(leg)` against the cached flag, without the
  `leg.len() == middle.len()` conjunct that defines the flag, so
  `Cospan::new(vec![0], vec![0, 1], vec!['a', 'b'])` — valid, and correctly
  *not* a left identity — tripped `assert_valid(true, _)` in debug with "The
  identity nature of the left arrow was wrong". Reachable through
  `NamedCospan::assert_valid(true)` / `assert_valid_nohash(true)`; no in-tree
  caller passed `true`. Both arms are gone with the cache they checked, and
  with them both `bool` parameters — see *`Cospan` has no cached identity
  flags* under **Changed — BREAKING** above.

- **`Span::add_middle` bounds-checks the pair before reading the labels**
  ([#289](https://github.com/sustia-llc/catgraph/issues/289)), returning
  `ConstructionMiddlePairOutOfBounds` — the variant `Span::new` already raises
  for the identical input shape. It previously reached the labels through
  `self.left[new_middle.0]` first, so `add_middle((usize::MAX, 0))` panicked
  with a bare `index out of bounds: the len is 1 but the index is
  18446744073709551615`, in every profile, from a method that already returns
  `Result`. The reported `pair_position` is the position the pair would have
  taken.

- **The remaining panicking preconditions name their invariant**, in every
  build profile: `Cospan::delete_boundary_node`, `Cospan::map_to_same`,
  `Cospan::connect_pair` and `NamedCospan::delete_boundary_node` now carry
  `# Panics` sections and messages that say which index was out of bounds and
  how large the boundary is. The empty-leg case is why the checks are explicit
  rather than left to the indexing: `delete_boundary_node` read
  `leg.len() - 1` first, which underflowed (debug panic, release wrap to
  `usize::MAX` followed by a `swap_remove` panic). Measured pre-#289 messages,
  now replaced: `attempt to subtract with overflow`; `swap_remove index (is 3)
  should be < len (is 1)`; `index out of bounds: the len is 1 but the index is
  5`.

### Added (#289)

- **`Cospan` derives `PartialEq` and `Eq`** — additive, not breaking; nothing
  that compiled before stops compiling. `==` compares `(left, right, middle)`
  field for field, which is the whole of the value now that the identity flags
  are gone. That is precisely what blocked the derive before: two cospans with
  identical triples could carry different cached flags and so compare unequal,
  for a difference no other part of the API could see.

  `Cospan::structurally_equal` **stays**, undeprecated, as a named alias for
  `==` — dropping it would break callers, Phase 6B (`catgraph-coalition`)
  snapshot-vs-expected assertions among them. New code may use either.

  Two properties of `==` worth stating, because they are easy to assume in the
  wrong direction:

  - It is **as coarse as `Lambda`'s `Eq`**, which `Cospan` never requires to be
    identity. Two `==` cospans can therefore differ observably in a field their
    labels do not compare on, and the difference can survive into a composite:
    `tests/compose_identity_arms.rs`'s
    `both_legs_identity_keeps_the_right_operands_labels` is exactly that
    fixture — its two operands are `==`, and which operand's apex the composite
    keeps is visible through the ignored field. Every `Lambda` in this
    workspace has `Eq` equal to identity, where this cannot bite.
  - It is **finer than equality of cospans as morphisms**, being apex-order
    sensitive. `cospan_canon`'s existing statement to that effect is unchanged
    by the derive: `==` is the same triple comparison `structurally_equal`
    always was, so `CospanCanon` remains the semantic comparison.

- **`catgraph-applied`'s `DecoratedCospan` gains a `PartialEq`** in the same
  window — hand-written rather than derived, and no `Eq`; see that crate's
  CHANGELOG for why the bounds differ.

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

- **`tests/property_laws.rs` asserted a production predicate against its own
  definition, generated only identity left legs, and never randomised an apex**
  ([#287](https://github.com/sustia-llc/catgraph/issues/287)). Test-only; no
  library surface changes.

  1. **`rel_equivalence_iff_rst` was self-oracled** — it compared
     `is_equivalence_rel()` with `is_reflexive() && is_symmetric() &&
     is_transitive()`, which is the production body spelled out on the other
     side of the `prop_assert_eq!`. Measured: rewriting all three predicates to
     `return true` left it **green**. Replaced by
     `rel_predicates_match_a_direct_pair_set_oracle`, which decides all seven
     `Rel` predicates against quantifiers over `middle_pairs()` that never call
     a `Rel` method, plus
     `rel_predicates_decided_exhaustively_on_small_carriers`, which does the
     same over **every** relation on carriers of size 1, 2 and 3 (530 in all)
     and cross-checks the per-predicate acceptance totals against published
     enumerations — reflexive/irreflexive `2^(n²−n)`, symmetric
     `2^(n(n+1)/2)`, antisymmetric `2ⁿ·3^(n(n−1)/2)`, transitive
     [A006905](https://oeis.org/A006905) `2, 13, 171`, equivalence Bell(n)
     `1, 2, 5`, partial order [A001035](https://oeis.org/A001035) `1, 3, 19`.
     The literature totals are what catches a bug shared by a predicate *and*
     its oracle — whenever the shared bug changes how many relations the
     predicate accepts (a shared bug under which the predicate accepts a
     different set of the same size passes both checks).
     `rel_composites_require_homogeneity` additionally pins the
     `is_homogeneous() &&` screen in both composites, which the exhaustive
     sweep cannot see (every relation it builds is homogeneous). Falsified:
     the three `return true` stubs redden all three tests; dropping the
     homogeneity screen turns `is_equivalence_rel` on a heterogeneous relation
     from `false` into an `unwrap` panic.

  2. **The composability generators emitted identity-only left legs** —
     `left_g`/`left_h` were `(0..b_size).collect()` unconditionally, so
     `cospan_associativity` was doubly quotiented outside the space it claims.
     New `arb_label_preserving_leg` sends each boundary slot to a uniformly
     chosen apex vertex *carrying that slot's label*, keeping `g.domain() ==
     f.codomain()` while allowing non-identity legs (measured: 139 of 256
     samples for `g`, 138 of 256 for `h`). Because a consuming proptest is
     blind to this — associativity holds over identity legs just as well — the
     generator is pinned by its own meta-test,
     `composability_generators_emit_label_aware_non_identity_left_legs`.
     Falsified: reverting the leg to `i ↦ i` reddens *only* the meta-test
     (0 of 256, `cospan_associativity` still green); making the leg
     label-blind reddens the meta-test and `cospan_associativity` together.

  3. **`CospanCanon`'s "equal iff isomorphic" had one hand-written apex swap
     and no random coverage** — new `canonical_form_decides_apex_isomorphism`
     asserts that equality of canonical forms **is** apex isomorphism, in both
     directions, against a brute-force search over `S_apex` (the F&S 2019 §3
     definition, not a second copy of the canonicaliser). Pairs are generated
     by a random apex permutation optionally followed by one rewired leg entry;
     `perturbation_generator_reaches_isomorphic_and_non_isomorphic_pairs` pins
     that both sides of the `iff` are reached (measured: 192 isomorphic, 64
     non-isomorphic of 256). `a_single_rewire_changes_the_form_unless_it_is_a_relabelling`
     adds the single-rewire negative *and* the case that keeps it honest — a
     rewire onto an equally-labelled vertex can be an apex transposition, which
     the form is designed to forget. Falsified: deleting `classes.sort()` from
     `canonical_form` reddens `canonical_form_decides_apex_isomorphism` and
     `a_single_rewire_changes_the_form_unless_it_is_a_relabelling` (the
     proptest shrinks to a pure apex relabelling; the meta-test never calls
     `canonical_form` and stays green); dropping the rewire arm from the
     generator leaves the `iff` test green at 256/256 isomorphic and reddens
     only the meta-test.

- **`rel_from_selector`'s "same relation for a given mask" was a structural
  claim no test asserted**
  ([#287](https://github.com/sustia-llc/catgraph/issues/287) follow-up) — the
  helper #287 introduced (`rel_from_selector`, shared by the proptest strategies
  and the exhaustive sweep) is one function, but sharing a function is not
  evidence that a bit denotes the pair the docstring says it does, nor that the
  strategies' bool-vec view and the sweep's `u32`-bitmask view agree. The
  strategies now build through a named `rel_from_bools`, and the new
  `rel_from_selector_matches_its_definition` checks, for every mask over
  `n ∈ {1, 2, 3}` (530 relations — the exhaustive sweep's whole corpus), first
  that `rel_from_bools` and `rel_from_mask` agree, then that `rel_from_mask`
  matches a reference written from the row-major definition without calling
  the helper. Falsified three ways, each reverted: a column-major flat index
  (`j*n + i`) inside the helper moves both views together, so only the
  definition check reddens (`n = 2, mask = 0b10`, `{(1, 0)}` vs `{(0, 1)}`);
  reversing `rel_from_mask`'s bit order reddens the parity check
  (`n = 2, mask = 0b1`, bool-vec `{(0, 0)}` vs bitmask `{(1, 1)}` — the
  definition check would too, behind it); reversing `rel_from_bools`'s bit
  order — the strategies' own path — reddens the parity check with the
  definition check untouched (`n = 2, mask = 0b1`, `{(1, 1)}` vs `{(0, 0)}`).
  Scope: `n = 4` is drawn by the strategies but not enumerated (2^16 masks);
  the convention is `n`-independent, so a break shows at `n = 2`, as all three
  mutations do.

- **The #258 braiding contract was pinned only downstream, and the only core
  *integration* test that named permutation composition was vacuous**
  ([#286](https://github.com/sustia-llc/catgraph/issues/286)). (Core's lib unit
  tests `cospan::test::permutation_automatic` and
  `frobenius::operations::test::from_permutation_compose_probe` name permutation
  composition too and are **not** vacuous — both use distinct labels, and
  `permutation_automatic` is one of the four lib tests that go red under the
  `Cospan` constructor flip in (6) below. The gap was in `tests/`.)

  Measured before the fix: inverting the braiding direction in
  `equivalence::CospanAlgebraMorphism`'s two constructors — `p.inv()` ⇄ `p` in
  both the forced label word and the structural cospan's right leg — left
  `cargo test -p catgraph` **fully green** while `cargo test -p catgraph-applied`
  went red on exactly 4 tests, all in `tests/braiding_cross_carrier.rs`. Three
  rows on types *defined in this crate* had no core-side pin:
  `CospanAlgebraMorphism`, `FrobeniusMorphism`'s **wiring** (the applied oracle
  checks only its `domain()`/`codomain()` words, and #258 established that a
  word can be right over an inverted wiring), and `NamedCospan`'s **port-name
  direction** — its cospan direction was already pinned transitively:
  `NamedCospan` delegates its cospan to `Cospan::from_permutation_on_*`, so
  `cospan::test::permutation_automatic` / `permutatation_manual_labelled` pin it
  (of the two `named_cospan` lib tests in (6) below, `permutatation_automatic`
  compares the composite's legs — its name assertions are over all-`()` names —
  and `permutatation_manual` only its words). Its names were pinned nowhere in
  core: permuting them by `p` instead of `p⁻¹` reddens only the new file,
  whether in `permute_side` (see (4)) or in the constructor
  (`from_permutation_extra_data_on_domain`: `['b','c','a']` vs `['c','a','b']`
  in two tests, the 289 lib tests green). A downstream test restructure would
  have zeroed core's coverage of those rows silently.

  New `tests/braiding_core_pins.rs` lifts those three rows into core: a
  hand-written anchor comparing `CospanAlgebraMorphism::from_permutation_*`'s
  `.element()` **canonically** — `CospanCanon::from_parts` built by hand from
  `ApexClass`es, so the pushout's apex numbering cannot make it pass — plus
  exhaustive sweeps over all `6 + 24 = 30` permutations at `n ∈ {3, 4}` with
  **distinct** labels, a `permute_side` identity/conjugation sweep, all 36
  ordered `S₃` pairs for `β(p₁) ; β(p₂) == β(p₁ ; p₂)`, and the arity-mismatch
  and `NamedCospan`-refusal rows. `FrobeniusMorphism` wiring is read through the
  crate's own `cospan_algebra::frobenius_to_cospan`.

  `tests/monoidal_structure.rs::permutation_cospan_compose` was rewritten in the
  same pass. It ran **one** pair of permutations over the uniform word
  `['a','a','a']`; uniform labels make `domain()` and `codomain()` constant in
  the permutation, so both word assertions held for any `p₁`, `p₂` — a compose
  realizing `p₂ ; p₁` passes — and the only other assertion was
  `middle.len() >= 3`. It now runs all 36 ordered `S₃` pairs over `['a','b','c']`,
  compares the composite's *wiring* against `(0..3).map(|i| (p1 * p2).apply(i))`
  computed from the two permutations directly, and asserts the apex is exactly
  `n` vertices rather than "at least".

  The exhaustive-permutation generator both files run on landed in
  **`catgraph-testutil`** (`all_perms` / `all_perm_indices`, and a
  `[dev-dependencies]` edge on this crate) rather than as two more private
  copies: it already existed twice in `catgraph-applied/tests`, and #33 opened
  that crate for exactly this. Both applied copies are retired. The
  `cospan_wiring` extractor — which needs a `catgraph` type, so it cannot live
  in `catgraph-testutil` (no `catgraph` edge, by design) — moved to this crate's
  existing `tests/common/mod.rs` instead of being written twice.

  **Falsified six ways.** (1) The `CospanAlgebraMorphism` constructor flip above
  now reddens 4 of the 5 tests in the new file
  (`arity_mismatch_and_named_cospan_refusal` asserts refusals, not direction,
  and stays green), and nothing else in core — the 289
  lib unit tests stay green, which is the gap restated as a measurement.
  `hand_written_reference_and_cam_element` fails first on the **codomain word**
  (`['B','C','A']` where the contract requires `['C','A','B']`), never reaching
  its canonical-form assertion; the apex-class form is what
  `permute_side_pins_on_the_core_carriers` reports, and there the *mutated
  constructor* is the expected side — `A:[0,5] B:[1,3] C:[2,4]` — against the
  contract form `A:[0,4] B:[1,5] C:[2,3]` that `permute_side` itself still
  builds. `core_carriers_realize_p_on_both_constructors` and
  `braiding_composition_over_all_s3_pairs` fail on the wiring, `[2, 0, 1]` where
  it must be `[1, 2, 0]`. (2) Dropping the `.inv()` from
  `FrobeniusMorphism::from_permutation_on_codomain` reddens the new Frobenius
  *wiring* row (`[2, 0, 1]` vs `[1, 2, 0]`) — also caught by two pre-existing
  lib tests, so a redundant catch rather than new coverage.
  (3) Reading the `FrobeniusMorphism::permute_side` domain branch symmetrically
  (`β(p)` where the contract asks for `β(p⁻¹)`) reddens the conjugation row
  (`['B','C','A']` vs `['C','A','B']`) — also redundant, three pre-existing lib
  tests see it. (4) Permuting `NamedCospan`'s port names by `p` instead of
  `p.inv()` in `permute_side` reddens **only** the new file (`['b','c','a']` vs
  `['c','a','b']`); the rest of `cargo test -p catgraph` stays green. (5) Using
  `p.apply` where `CospanAlgebraMorphism::permute_side`'s domain branch builds
  its relabelling leg with `p_inv.apply` likewise reddens **only** the new file
  (`[1, 2, 0]` vs `[2, 0, 1]`). (6) Flipping `Cospan`'s two constructors reddens
  the rewritten `permutation_cospan_compose` (`[2, 0, 1]` vs `[1, 2, 0]`) — also
  caught by **4 pre-existing lib tests** (`cospan::test::permutation_automatic`,
  `cospan::test::permutatation_manual_labelled`,
  `named_cospan::test::permutatation_automatic`,
  `named_cospan::test::permutatation_manual`), so this is a *vacuity repair*, not
  new coverage of `Cospan` itself. What it repairs is measured: the **pre-#286
  version of that same test was green under the identical mutation**, along with
  all 6 tests in its file.

  **The space these claims range over**, stated where the assertions can be
  checked against it: `n ∈ {3, 4}` only (`n ≤ 2` makes every permutation an
  involution, so a direction flip is unobservable there); `PartitionAlgebra` and
  `char` are the only algebra and label type instantiated; `()` is the only
  black-box label; the composition sweep is `S₃ × S₃` and does not extend to
  `n = 4`; and no `permute_side` row here starts from a *non*-identity morphism —
  that separation ("splices the right braiding" vs "rebuilds one from scratch")
  remains `catgraph-applied/tests/braiding_cross_carrier.rs`'s claim, not this
  crate's.

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
  layer fold; its six arms other than `Spider` and `UnSpecifiedBox`, the
  hand-built braiding literal included, are byte-identical to the survivor's,
  so a type-correct convention error applied to both copies cannot be compared
  away here (measured: flipping the braiding leg in both still reddens the pin,
  but via the fold's label check on a mixed-label σ, not a comparison; the
  same-label case is held by the `from_permutation_on_domain`-built tests in
  `tests/frobenius_axioms.rs` — two red under E, of 15 crate-wide). The space:
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

- **clippy 1.98 compatibility**
  ([#340](https://github.com/sustia-llc/catgraph/issues/340)). The six
  `#[allow(clippy::from_iter_instead_of_collect)]` in `span.rs` named a lint
  clippy 1.98 removed ("lint has proved problematic"), which `-D warnings`
  turned into `renamed_and_removed_lints` errors and failed every `main` CI
  run from #334 (2026-08-21) on; the `HashSet::from_iter` calls are now
  `.collect()`, so no allow is needed on either toolchain. No behaviour change.

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
