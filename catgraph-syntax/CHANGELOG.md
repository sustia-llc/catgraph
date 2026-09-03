# Changelog — catgraph-syntax

All notable changes to this crate are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); semver per
[SemVer 2.0.0](https://semver.org/spec/v2.0.0.html). Versioning is
workspace-wide: this crate's versions track the repo's `v0.x` tags.

## [Unreleased]

## [workspace-v0.17.0] - 2026-09-03

### Changed

- Rustdoc reduced to contract statements and stale section-name
  cross-references repointed; this CHANGELOG is one bullet per change
  ([#365](https://github.com/sustia-llc/catgraph/issues/365)).
- `docs/ANCHORS.md` lists `Wire`, `WireCount`, `PairSwap`,
  `FrobeniusEquation` and `MAX_NESTING_DEPTH` on the row of the module
  defining each; its header names the enumeration command
  ([#407](https://github.com/sustia-llc/catgraph/pull/407)).

### Fixed — tests

- `FrobeniusOr::Delta`/`Eta` print/parse pinned at the implicit sort (token
  round-trip at both the bare and colour-annotated form, plus a
  presentation-file golden) — `Epsilon` already had this coverage
  ([#316](https://github.com/sustia-llc/catgraph/issues/316)).
- Each of `scfm_equations`' nine Def 2.5 equations pinned to a `d=2`
  `MatKron` image (shared across the slots the algebra makes coincide) and
  its own concrete syntax; `to_mat_kron` and `CospanFunctor` pinned to agree
  per palette colour on all nine, and `to_mat_kron`'s braid pinned at the
  reversed mixed-colour boundary (`[B, A] ↦ braiding(3, 2)`)
  ([#317](https://github.com/sustia-llc/catgraph/issues/317)).

## [workspace-v0.16.0] - 2026-08-24

### Documentation

- `frobenius` module docs, `FrobeniusOr<G>`, `hypergraph_presentation`,
  `lib.rs`, and README no longer claim "free hypergraph category" for the
  crate's own artifacts — only as a citation of F&S 2019 Thm 3.14 about
  `Cospan_Λ` ([#277](https://github.com/sustia-llc/catgraph/issues/277)).

## [workspace-v0.14.0] - 2026-08-16

### Documentation

- `examples/workflow_dedup.rs` documents that `ContentKey` is persistable
  under catgraph-applied's off-by-default `serde` feature, with its caveats:
  the key is not a term, the serialized shape is not a stable wire format,
  and `ContentKey` remains not `Ord`
  ([#264](https://github.com/sustia-llc/catgraph/issues/264), following
  [#255](https://github.com/sustia-llc/catgraph/issues/255)).

## [workspace-v0.11.0] - 2026-08-10

### Changed

- `rand` no longer appears in this crate's lib dependency graph;
  catgraph-applied's `E1::random` now takes a caller-supplied generator
  ([#239](https://github.com/sustia-llc/catgraph/issues/239), changed in
  catgraph-applied).
- **BREAKING:** the Arrow algebra (`Arrow`, `Compose`, `Split`, `Id`, `Lift`,
  `First`, `Second`, `Fanout`, `ArrowBuilder`, `arrow`) is now defined in
  `src/arrow_seam.rs` rather than re-exported from `deep_causality_haft`;
  paths and shapes are unchanged but the types are no longer haft's, and
  haft's `left`/`right`/`choice`/`fanin` methods are not carried. Derived
  from `deep_causality_haft` 0.4.2's Arrow module (MIT, attributed in the
  file's license header). This crate's runtime dependency set is now
  catgraph + catgraph-applied + thiserror (+ opt-in serde)
  ([#222](https://github.com/sustia-llc/catgraph/issues/222)).
  - New `tests/arrow_laws.rs` pins the category/strength/fanout laws and
    builder-denotation agreement.

## [workspace-v0.10.0] - 2026-08-09

### Fixed

- Browser-wasm lib builds (`--target wasm32-unknown-unknown`) no longer fail
  in `getrandom`; catgraph-applied's `rand` edge now defaults to no features
  ([#232](https://github.com/sustia-llc/catgraph/issues/232), fixed in
  catgraph-applied).

## [workspace-v0.8.0] - 2026-08-03

### Added

- `examples/workflow_dedup.rs` — the first colored example, a role-typed
  workflow over `Λ = {Author, Reviewer, Editor}` exercising `ColoredExpr`,
  per-role `FrobeniusOr` spiders, colored presentation files,
  `hypergraph_presentation`, and the applied `rewrite` surface
  ([#214](https://github.com/sustia-llc/catgraph/issues/214) W1).
  `examples/README.md` documents the seams: the `RoleId(usize) ↔ Color`
  mapping is caller-side, there is no diagram→magnitude seam, and `eq_mod`
  is not transitive ([#189](https://github.com/sustia-llc/catgraph/issues/189)),
  so class use needs connected components.

## [workspace-v0.6.0] - 2026-08-02

### Fixed

- `eval`'s `Braid` arm checks and saturates an overflowing `m + n` before
  reserving, reporting `SyntaxError::WireCount` instead of panicking (debug)
  or wrapping into `rotate_left(mid > len)` (release); `take_exact` now
  detects a shortfall before reserving rather than draining until dry
  ([#196](https://github.com/sustia-llc/catgraph/issues/196)).

## [workspace-v0.5.0] - 2026-07-30

### Changed

- **BREAKING:** the textual surface is Λ-colored: `GeneratorSyntax` clause 2
  additionally reserves `:` and `@` and the word `->`; `SfgGenerator<R>`'s
  `Scalar` token moves from `scalar:<r>` to `scalar_<r>`;
  `impl GeneratorSyntax for FrobeniusOr<G>` ungates from
  `G: GeneratorSyntax<Color = ()>` to `G: GeneratorSyntax` with
  `G::Color: ColorSyntax + Ord`; `print_presentation`/`parse_presentation`
  gain a `G::Color: ColorSyntax` bound and emit declarations
  ([#79](https://github.com/sustia-llc/catgraph/issues/79) P3b).
- **BREAKING:** the Frobenius layer is Λ-colored: `FrobeniusOr<G>` becomes
  `Mu(G::Color)` / `Eta(G::Color)` / `Delta(G::Color)` / `Epsilon(G::Color)`
  / `User(G)`; builders (`spider`, `cup`, `cap`, `scfm_equations`) take
  their colour first; `hypergraph_presentation` seeds the nine SCFM
  equations at every palette colour; `to_mat_kron(expr, source_word, dims)`
  takes an interface word and a per-colour dimension function, and
  `SyntaxError::DimensionOverflow`'s fields change from
  `{ dim, exponent }` to `{ product, factor }`; `to_cospan(expr,
  source_word)` returns `Cospan<G::Color>` and `CospanFunctor` gains a
  `ColoredCompleteFunctor<FrobeniusOr<G>>` impl; `GeneratorSyntax for
  FrobeniusOr<G>` keeps its `Color = ()` gate
  ([#79](https://github.com/sustia-llc/catgraph/issues/79) P3a).
- **BREAKING:** `FrobeniusOr<G>`'s `PropSignature`/`GeneratorSyntax` impls
  require `G: PropSignature<Color = ()>`, pending the colored spiders above
  ([#79](https://github.com/sustia-llc/catgraph/issues/79) P1).
- The crown-jewel completeness-registry test in
  `tests/cospan_complete_functor.rs` migrates to a counit-law witness, since
  catgraph-applied's component-granular NF now lets plain congruence
  closure prove the original CC-gap pair
  ([#55](https://github.com/sustia-llc/catgraph/issues/55) PR2).

### Documentation

- The monochromatic scope note retires from `src/lib.rs`, README, and
  `src/frobenius.rs`; `docs/ANCHORS.md` gains rows for `text::ColorSyntax`
  (Def 3.9) and `cospan_functor::{to_cospan, CospanFunctor}` (Prop 3.8 /
  Thm 3.14); the "arity check" sites now say boundary-*word* equality
  ([#79](https://github.com/sustia-llc/catgraph/issues/79) P3b).
- `SfgModel`'s overflow note in `src/eval.rs` cites
  `catgraph_applied::rig`'s module docs for the per-rig-family overflow
  policy and names `catgraph_applied::rig::Checked` as the opt-in detection
  story ([#88](https://github.com/sustia-llc/catgraph/issues/88)).

### Added

- `text::ColorSyntax` (`print_color` / `parse_color` / `implicit`); spiders
  print and parse as `mu@A` / `eta@A` / `delta@A` / `epsilon@A`;
  presentation files accept a generator declaration line
  `TOKEN : COLOR* -> COLOR*`, validated against `G` but never constructing
  the signature; `tests/colored_text.rs` covers the round trip
  ([#79](https://github.com/sustia-llc/catgraph/issues/79) P3b).
- `Checked<i64>` overflow regression in `tests/eval.rs`: a poisoned
  `SfgModel<Checked<i64>>` output round-trips through `scalar_⊥`
  ([#88](https://github.com/sustia-llc/catgraph/issues/88)).

## [workspace-v0.4.0] - 2026-07-25

### Changed

- Paper-audit citation reconciliation: the "spider" vocabulary is
  re-attributed from F&S 2019 §2.2 to Seven Sketches Def 6.54 / Thm 6.55,
  and `MatKron(R)` is marked an *extension of* F&S 2019 Ex 2.16 rather than
  its stated content (`docs/ANCHORS.md`, `frobenius.rs`).

### Documentation

- `arrow_seam`'s `Free`/`FreeWitness` exclusion note is refreshed for haft
  0.4.2: still no `Hash` and no serde, so `PropExpr<G>` stays the term type
  ([#93](https://github.com/sustia-llc/catgraph/issues/93)).

### Added

- `depth` module: `term_depth` (iterative), `guard_term_depth`, and
  `MAX_TERM_DEPTH` (= the parser's `MAX_NESTING_DEPTH`, 256); `eval`,
  `to_mat_kron`, and `to_cospan` pre-flight the term's structural depth and
  return `SyntaxError::RecursionLimit` / `CatgraphError::RecursionLimit`
  instead of risking a stack overflow
  ([#99](https://github.com/sustia-llc/catgraph/issues/99)).
- Optional `serde` feature, off by default: forwards to
  `catgraph-applied/serde` and derives `Serialize`/`Deserialize` on
  `FrobeniusOr<G>` ([#81](https://github.com/sustia-llc/catgraph/issues/81)).
- `cospan_functor::CospanFunctor` — a complete decision functor
  (`CompleteFunctor<FrobeniusOr<G>>`, `Target = CospanCanon<()>`) for the
  User-free spider fragment, mapping into the free monochromatic cospan
  category and canonicalising up to apex isomorphism (F&S 2019 Prop 3.8);
  the second entry in the completeness registry after `Mat(R)`/Thm 5.60
  ([#80](https://github.com/sustia-llc/catgraph/issues/80)).

### Changed

- `SyntaxError` gains a `RecursionLimit { depth, limit }` variant, additive
  under `#[non_exhaustive]`
  ([#99](https://github.com/sustia-llc/catgraph/issues/99)).

## [0.3.0] - 2026-07-11

The crate's first release: a textual generator/relation presentation
surface for hypergraph-category morphisms over catgraph-applied's
`PropExpr` / `Free` / presentation engine
([#5](https://github.com/sustia-llc/catgraph/issues/5)).

### Added

- Workspace member; `SyntaxError` (thiserror); `arrow_seam` re-exporting
  `deep_causality_haft`; the structural, total pretty-printer
  (`text::{GeneratorSyntax, Pretty, print}`); `docs/ANCHORS.md`
  ([#82](https://github.com/sustia-llc/catgraph/pull/82)).
- Hand-rolled recursive-descent `text::parse` building exclusively through
  `Free::*`; Unicode `⊗` as an input synonym for `*`; bounded nesting depth
  (`MAX_NESTING_DEPTH`); presentation files, one `lhs = rhs` per line
  (Def 5.33); `GeneratorSyntax` for `SfgGenerator<R>`
  ([#86](https://github.com/sustia-llc/catgraph/pull/86)).
- `eval::ArrowModel` and `eval` — O(n) streaming interpreter with no
  `Clone` bound on wire values; `SfgModel<R>` (R-linear signal-flow-graph
  semantics); `SyntaxError::{WireCount, ModelArity}`; the enum becomes
  `#[non_exhaustive]`; basis-row cross-check against `MatrixNFFunctor`
  ([#87](https://github.com/sustia-llc/catgraph/pull/87)).
- `frobenius::FrobeniusOr<G>` (μ/η/δ/ε adjoined to a user signature);
  `lift_user`; `spider`/`cup`/`cap`; `scfm_equations` (the nine Def 2.5
  SCFM equations); `hypergraph_presentation`; `to_mat_kron` into
  `MatKron(R)` (sound per Prop 3.8, not a `CompleteFunctor`); cell-count
  overflow guards; `SyntaxError::{NonFrobenius, DimensionOverflow}`
  ([#89](https://github.com/sustia-llc/catgraph/pull/89)).
- `traced::Traced<A, G>` pairing an executable `Arrow` with the term it
  denotes; sealed `Wires<V>` / `WireCount`; `traced_generator`,
  `traced_id`, `traced_braid_1_1`, `then` (`>>>`), `par` (`***`); the
  coherence law `eval(t.term(), &m, in.flatten()) == Ok(t.run(in).flatten())`
  ([#90](https://github.com/sustia-llc/catgraph/pull/90)).

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.17.0...HEAD
[workspace-v0.17.0]: https://github.com/sustia-llc/catgraph/compare/v0.16.0...v0.17.0
[workspace-v0.16.0]: https://github.com/sustia-llc/catgraph/compare/v0.15.0...v0.16.0
[workspace-v0.14.0]: https://github.com/sustia-llc/catgraph/compare/v0.13.0...v0.14.0
[workspace-v0.11.0]: https://github.com/sustia-llc/catgraph/compare/v0.10.0...v0.11.0
[workspace-v0.10.0]: https://github.com/sustia-llc/catgraph/compare/v0.9.0...v0.10.0
[workspace-v0.8.0]: https://github.com/sustia-llc/catgraph/compare/v0.7.0...v0.8.0
[workspace-v0.6.0]: https://github.com/sustia-llc/catgraph/compare/v0.5.0...v0.6.0
[workspace-v0.5.0]: https://github.com/sustia-llc/catgraph/compare/v0.4.0...v0.5.0
[workspace-v0.4.0]: https://github.com/sustia-llc/catgraph/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/sustia-llc/catgraph/compare/v0.2.1...v0.3.0
