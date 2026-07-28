# Changelog — catgraph-syntax

All notable changes to this crate are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); semver per
[SemVer 2.0.0](https://semver.org/spec/v2.0.0.html). Versioning is
workspace-wide: this crate's versions track the repo's `v0.x` tags.

## [Unreleased]

### Changed

- **BREAKING: the Frobenius layer is Λ-colored**
  ([#79](https://github.com/sustia-llc/catgraph/issues/79) P3a). FS19 Def 2.12
  equips *each object* with a Frobenius structure, so the spider family is
  per-colour and the four variants gain a `Λ` payload:
  - `FrobeniusOr<G>` becomes `Mu(G::Color)` / `Eta(G::Color)` /
    `Delta(G::Color)` / `Epsilon(G::Color)` / `User(G)`, with
    `G: PropSignature` now a bound on the enum itself and `type Color =
    G::Color` (colour-transparent). Mono callers write `Mu(())`. `Ord` /
    `PartialOrd` are hand-written under `where G::Color: Ord` — `Color`'s own
    bounds are only `Clone + Eq + Hash + Debug`, and that same bound rides the
    `PropSignature` impl and every Frobenius-facing signature.
  - Builders take their colour first: `spider(c, m, n)`, `cup(c)`, `cap(c)`,
    `scfm_equations(c)`. `hypergraph_presentation(colors, user_eqs)` seeds the
    nine SCFM equations at **every** palette colour (FS19 Lemma 3.10 / Ex 2.9:
    a structure per `l ∈ Λ` induces the whole hypergraph structure). The P1
    `Color = ()` gates are lifted from every genuinely colour-generic bound
    (`lift_user` included).
  - `to_mat_kron(expr, source_word, dims)` takes an interface word and a
    **per-colour** dimension function; a wire of colour `c` maps to
    `R^dims(c)` and a word to the product of its letters' dimensions. Colours
    flow top-down as in applied's `prop::colored::check`, so a spider fed the
    wrong colour is rejected (`CatgraphError::Composition`) and a wrong-length
    word is `CompositionSizeMismatch`. The overflow guard moves from `dim^k`
    exponents to checked word products, and `SyntaxError::DimensionOverflow`'s
    fields change from `{ dim, exponent }` to `{ product, factor }` — with
    per-colour dimensions there is no single `dim` to report, so the variant
    now names the multiplication that overflowed. The
    "usize-overflow rejection ≠ OOM protection" posture is unchanged.
  - `to_cospan(expr, source_word)` returns `Cospan<G::Color>`: apex vertices
    carry the colours that flow through them (FS19 Ex 3.12 transports Ex 2.8's
    generators to each `l ∈ Λ`). An arity mismatch now surfaces from the word
    pass as `CompositionSizeMismatch` rather than from the pushout as
    `Composition`. `CospanFunctor` keeps its monochromatic
    `CompleteFunctor<FrobeniusOr<G>>` impl (`Target = CospanCanon<()>`) and
    gains a `ColoredCompleteFunctor<FrobeniusOr<G>>` impl with
    `Target = CospanCanon<G::Color>` for `G::Color: Copy + Ord`, anchored on
    Thm 3.14 (`Cospan_Λ` is the free hypergraph category on `Λ`).
  - `GeneratorSyntax for FrobeniusOr<G>` **keeps** its `Color = ()` gate: the
    colour-annotated token grammar (`mu@A`, generator declarations) is P3b.
    The crate's scope note narrows accordingly — the calculus is colored, the
    text is not.
- **BREAKING: `FrobeniusOr<G>` is monochromatic-gated pending colored
  spiders** ([#79](https://github.com/sustia-llc/catgraph/issues/79) P1):
  following catgraph-applied's Λ-colored `PropSignature` extension, the
  `PropSignature`/`GeneratorSyntax` impls for `FrobeniusOr<G>` (and ~20
  Frobenius-facing generic bounds in `frobenius.rs`/`cospan_functor.rs`)
  require `G: PropSignature<Color = ()>` — the spider variants
  `Mu/Eta/Delta/Epsilon` have no per-color shape yet (that is P3's ratified
  change: they gain a `Λ` payload, per FS19's per-object Frobenius
  structure), so the crate's monochromatic scope note now holds by
  construction rather than by convention. `FrobeniusOr` derives
  `PartialOrd + Ord`; signature impls migrate per the applied-crate
  migration note (`type Color = ()` + `mono_word`-backed word methods).
- **Scalar centrality became an NF theorem — the #80 CC-gap witness migrated**
  ([#55](https://github.com/sustia-llc/catgraph/issues/55) PR2): the
  crown-jewel completeness-registry test `(η;ε)⊗μ = μ⊗(η;ε)` in
  `tests/cospan_complete_functor.rs` documented a CC *gap* (plain congruence
  closure returned non-`Some(true)`; only `CospanFunctor` decided it). After
  catgraph-applied's component-anchored NF (rule (i) + Step 7), closed
  components sort leftmost and plain CC now proves the pair — an unplanned
  external confirmation that the NF change does real work. The test is
  migrated to a **counit-law witness** (a pair the NF still cannot decide, so
  it keeps demonstrating the functor's strictly-finer decision power); an
  in-test doc comment explains the migration.

### Documentation

- **`SfgModel`'s overflow note now cites the policy of record**
  ([#88](https://github.com/sustia-llc/catgraph/issues/88)): the inherited-from-`R`
  explanation in `src/eval.rs` points at `catgraph_applied::rig`'s module docs
  for the per-rig-family matrix and names `catgraph_applied::rig::Checked` as
  the opt-in detection story — a `⊥` in `eval`'s output vector is exactly the
  failure the basis-row cross-check cannot catch, since with a plain `i64` the
  interpreter and the matrix functor wrap identically and agree on the wrong
  answer. The existing convention-parity explanation is unchanged; docs only.

### Added

- **`Checked<i64>` overflow regression in `tests/eval.rs`**
  ([#88](https://github.com/sustia-llc/catgraph/issues/88)): the issue's
  reproducer `"scalar:4000000000 ; scalar:4000000000"` parsed as
  `PropExpr<SfgGenerator<Checked<i64>>>` and evaluated under
  `SfgModel<Checked<i64>>` yields a poisoned output (with plain `i64` this is
  the invisible release-build wrap), plus a `scalar:⊥` print → parse round-trip
  proving the poison sentinel stays a single lexical atom under the
  `GeneratorSyntax` token contract.

## [workspace-v0.4.0] - 2026-07-25

### Changed

- **Paper-audit citation reconciliation (Phase 6)** — verified every FS18/FS19
  anchor in `docs/ANCHORS.md`, `src/**`, README, and examples against the
  cached papers. Two fixes: (1) the "spider" vocabulary was mis-attributed to
  F&S 2019 §2.2 — the word never appears in that paper; the name + fusion
  theorem are Seven Sketches Def 6.54 / Thm 6.55 (§6.3.1), while F&S 2019
  §2.2/Def 2.5 keeps the SCFM-axiom anchor and Ex 2.8 the apex-1 cospan model
  (`docs/ANCHORS.md`, `frobenius.rs::spider`); (2) `MatKron(R)` was presented
  as F&S 2019 Ex 2.16's content — the example states only that
  FdVect-with-chosen-basis is a hypergraph category \[Kis15\]; the arbitrary-rig
  Kronecker target is now marked an *extension of* Ex 2.16 (`docs/ANCHORS.md`,
  `frobenius.rs`; the released [0.3.0] "(Ex 2.16)" wording below is historical).
  Re-verified as correct: the nine-equation count (FS19 Ex 2.8 + FS18 Def
  6.52's own nine), basis-ROW/Rem 5.49, Prop 3.8 "special", Thm 3.14, Defs
  5.25/5.30/5.33/5.45/5.50, Thm 5.53/5.60, and `frobenius.rs`'s left-only
  unitality/counitality note (PDF p.10 figure check).

### Documentation

- **`arrow_seam` `Free`/`FreeWitness` exclusion rationale refreshed for haft
  0.4.2** ([#93](https://github.com/sustia-llc/catgraph/issues/93)): haft's free
  monad now has opt-in `Eq`/`Debug`/`Clone` via the `EqFunctor`/`DebugFunctor`/
  `CloneFunctor` capability traits (no longer "opaque by design"; `CloneFunctor`
  arrived in 0.4.2), but still **no `Hash`** and no serde. `Hash` is the
  load-bearing gap: the congruence-closure engine needs `Eq + Hash` on terms, so
  `PropExpr<G>` stays the term type and **the exclusion is unchanged by `Clone`
  having arrived**. No code change; catgraph-syntax does not adopt haft's `Free`
  (that adoption landed in catgraph-dl per #93).

### Added

- **`depth` module — recursion guard for the term interpreters**
  ([#99](https://github.com/sustia-llc/catgraph/issues/99)): `term_depth`
  (iterative, overflow-free), `guard_term_depth`, and `MAX_TERM_DEPTH` (= the
  parser's `MAX_NESTING_DEPTH`, 256). `eval`, `to_mat_kron`, and the Cospan
  functor's `to_cospan` now pre-flight the term's structural depth and return a
  catchable error (`SyntaxError::RecursionLimit`; the `CatgraphError`-typed
  `to_cospan` reports the shared `CatgraphError::RecursionLimit`, one shape across
  all interpreters) instead of risking a stack overflow on
  an unbounded programmatically-built term. The limit is set below the
  *interpreters'* own recursion-overflow point (their frames — Kronecker
  products, cospan pushouts — are heavy), not merely to bound depth. `print`
  stays infallible and documents the same exposure with a `term_depth` pre-check.
  (The recursive `Drop` of a deep `PropExpr` remains an upstream `catgraph-applied`
  concern, out of scope here.)
- **Optional `serde` feature** ([#81](https://github.com/sustia-llc/catgraph/issues/81),
  syntax complement): forwards to `catgraph-applied/serde` and derives
  `Serialize`/`Deserialize` on `FrobeniusOr<G>`, so a full syntax term
  `PropExpr<FrobeniusOr<G>>` round-trips through serde — the machine analogue of
  the textual parser/printer. Off by default.
- **`cospan_functor` — a complete decision functor for the pure-spider
  fragment** ([#80](https://github.com/sustia-llc/catgraph/issues/80)):
  `CospanFunctor` implements `catgraph-applied`'s `CompleteFunctor<FrobeniusOr<G>>`
  by mapping the **User-free** spider fragment into the free monochromatic
  cospan category and canonicalising up to apex isomorphism (F&S 2019
  Prop 3.8 — `(Cospan, ⊕)` is the theory of *special* commutative Frobenius
  monoids). `Target = CospanCanon<()>`. This is the **second entry in the
  completeness registry** after `Mat(R)`/Thm 5.60: `Presentation::eq_mod_functorial`
  now gives a definite decision for `E_frob` where the congruence-closure
  `eq_mod` is only sound-incomplete (#15). Scalars are **kept** (the closed
  bubble `η;ε` is distinct from `id₀`), so `Cospan` — not the extra-special
  `Corel` — is the target; over an idempotent rig the functor is strictly finer
  than `to_mat_kron`. `User` generators lie outside the fragment
  (`CatgraphError::Presentation`); colored/multi-sorted generality is
  [#79](https://github.com/sustia-llc/catgraph/issues/79). Relies on the new
  `catgraph::cospan_canon` canonical form.

### Changed

- `SyntaxError` gains a `RecursionLimit { depth, limit }` variant (#99;
  additive — the enum is `#[non_exhaustive]`).

## [0.3.0] - 2026-07-11

The crate's first release — the complete S1–S5 milestone surface of
[#5](https://github.com/sustia-llc/catgraph/issues/5) (design approved
2026-07-09; every phase review-gated). A textual generator/relation
presentation surface for hypergraph-category morphisms over
`catgraph-applied`'s `PropExpr` / `Free` / presentation engine.

### Added

- **S1 — skeleton + printer**
  ([#82](https://github.com/sustia-llc/catgraph/pull/82)): workspace member;
  `SyntaxError` (thiserror); `arrow_seam` (the single file naming
  `deep_causality_haft`, per the catgraph-dl `endofunctor.rs` precedent,
  [#12](https://github.com/sustia-llc/catgraph/issues/12)); the structural,
  total pretty-printer (`text::{GeneratorSyntax, Pretty, print}`) — never
  normalizes, minimal parentheses (Seven Sketches Def 5.25/5.30);
  `docs/ANCHORS.md` (public-item → theorem map).
- **S2 — parser + presentation files**
  ([#86](https://github.com/sustia-llc/catgraph/pull/86)): hand-rolled
  recursive-descent `text::parse` (zero deps) building exclusively through
  `Free::*`, so every parse is arity-sound by construction; Unicode `⊗` as an
  input synonym for `*`; bounded nesting depth (`MAX_NESTING_DEPTH`) on
  untrusted input; structured keyword arguments (`,` is a reserved
  delimiter); presentation files, one `lhs = rhs` per line (Def 5.33) —
  the format carries the equation list only; `GeneratorSyntax` for
  `SfgGenerator<R>` (`scalar:<r>` tokens, round-trip conditions documented).
  Round-trip law `parse(&print(e)) == Ok(e)` proptested (structural, up to
  the depth bound).
- **S3 — interpreter**
  ([#87](https://github.com/sustia-llc/catgraph/pull/87)): `eval::ArrowModel`
  (a semantics = the generator action) and `eval` — **O(n) streaming** (each
  subterm consumes exactly its wires from a cursor) with **no `Clone` bound
  on wire values** (duplication is a model concern: the Fanout ≠ Frobenius-δ
  discipline); `SfgModel<R>` (R-linear Σ_SFG semantics; value arithmetic
  inherits `R`'s overflow semantics — documented, see
  [#88](https://github.com/sustia-llc/catgraph/issues/88));
  `SyntaxError::{WireCount, ModelArity}`; the enum is now `#[non_exhaustive]`.
  Milestone law: the basis-**row** cross-check against `MatrixNFFunctor`
  (Thm 5.53/5.60; row-vector convention per Def 5.50/Remark 5.49). The
  README example is compile-tested (doctest include).
- **S4 — Frobenius layer**
  ([#89](https://github.com/sustia-llc/catgraph/pull/89)): `frobenius::FrobeniusOr<G>`
  (μ/η/δ/ε adjoined to a user signature as a sum type — every engine surface
  works over `PropExpr<FrobeniusOr<G>>` unchanged); `lift_user`; iterative
  `spider(m, n)` / `cup` / `cap`; `scfm_equations()` — the **nine** Def 2.5
  SCFM equations (F&S 2019; the paper's own count, Ex 2.8 — an earlier design
  note said "ten"); `hypergraph_presentation` (user theory ⊎ E_frob, the
  [#15](https://github.com/sustia-llc/catgraph/issues/15) boundary restated at
  point of use); `to_mat_kron` into `MatKron(R)` (Ex 2.16) — **sound** per
  Prop 3.8, deliberately not a `CompleteFunctor` (the Cospan-valued spike is
  [#80](https://github.com/sustia-llc/catgraph/issues/80)); complete
  cell-count overflow guards (`dim^(src+tgt)` checked before every
  constructor); `SyntaxError::{NonFrobenius, DimensionOverflow}`.
  Monochromatic fragment `Λ = {•}` (colored props are
  [#79](https://github.com/sustia-llc/catgraph/issues/79)).
- **S5 — Traced typed builder**
  ([#90](https://github.com/sustia-llc/catgraph/pull/90)): `traced::Traced<A, G>`
  pairs an executable haft `Arrow` with the term it denotes; **sealed**
  `Wires<V>` / `WireCount` bridge (`()` / `Wire<V>` / `(L, R)` pair-trees —
  sealing is a soundness precondition for the combinators' infallibility);
  paired combinators `traced_generator` (the sole fallible constructor),
  `traced_id`, `traced_braid_1_1`, `then` (`>>>`), `par` (`***`) — `then`/`par`
  are **infallible** (type-level interface agreement); `fanout` rejected
  type-level, general `braid(m, n)` and spider arrows deliberately omitted.
  The coherence law `eval(t.term(), &m, in.flatten()) == Ok(t.run(in).flatten())`
  is stated inductively (generators establish it for value-agreeing models;
  combinators preserve it) and tested per combinator. Hughes 2000 cited as
  lineage.

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.4.0...HEAD
[workspace-v0.4.0]: https://github.com/sustia-llc/catgraph/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/sustia-llc/catgraph/compare/v0.2.1...v0.3.0
