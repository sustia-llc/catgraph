# Changelog — catgraph-dl

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/spec/v2.0.0.html).

> Pre-reboot version links (`catgraph-dl-v0.x`) point at the private
> predecessor repo `tsondru/catgraph` and do not resolve publicly. Paper PDFs
> named in historical entries were removed from the tree on 2026-07-10.

## [Unreleased]

### Changed

- Rustdoc reduced to contract statements; this CHANGELOG rewritten to one
  bullet per change. `architectures` module doc no longer describes
  `RecursiveNn::unroll` as fallible (it has returned `S` since v0.15.0).
- `Free::fold` and `Cofree::unfold` rustdoc state position order; the
  evaluation-order clauses are gone
  ([#310](https://github.com/sustia-llc/catgraph/issues/310)).
- The list-bijection tests compare `Free` values structurally: the backward
  proptest leg asserts `f2 == f1`, and the cons-cell tower is compared to its
  canonical encoding ([#311](https://github.com/sustia-llc/catgraph/issues/311)).

## [workspace-v0.15.0] - 2026-08-16

### Changed

- **BREAKING:** every carrier walk is an explicit heap worklist — `Free::fold`,
  `Cofree::unfold`, the tree bijections, `RecursiveNn::unroll`, the
  carriers' `Drop`/`PartialEq`/`Debug` and `BinaryTree`'s `Clone`
  ([#200](https://github.com/sustia-llc/catgraph/issues/200)):
  - `Free` and `BinaryTree` are structs over a private cell; their shapes are
    the new `FreeView` (`Pure`/`Suspend`) and `TreeView` (`Leaf`/`Node`),
    reached via `pure`/`suspend`/`leaf`/`node`, `into_view`, `as_view`,
    `from_view` / `From<…View>`.
  - `Free::fold`, `Cofree::unfold` and the carriers' `PartialEq`/`Debug` gain a
    `Container` bound; `OptionWitness` gains a `Container` impl.
  - `tree_to_free_mnd`, `free_mnd_to_tree`, `RecursiveNn::unroll` return
    plain values again (no `Result`).
  - `EqFunctor::eq_type` → `eq_shape`; `DebugFunctor::fmt_type` →
    `fmt_shape(fa, f, contents)`; `Container::Shape` loses its
    `PartialEq + Debug` bound; `Container` gains `contents`.
  - A borrowed payload must be declared before the carrier holding it
    (hand-written `Drop`, `error[E0597]` otherwise).
  - A `DebugFunctor` that renders a recursion slot outside the `Formatter`
    it was handed is a `fmt::Error`, not a truncated rendering.
  - `Debug` carries `{:#?}`, precision and width; fill, alignment,
    sign/zero-pad and debug-hex flags render as if absent.
  - `Free::fold` now allocates (`2·n + 2` on lists, `2·(L − 1) + 2` on trees).
  - Sizes: `Free` and `BinaryTree` equal their views; `Cofree` is one word
    larger than its cell (`Cofree<OptionWitness, u32>` 16 → 24,
    `Cofree<TreeEndo<u8>, f64>` 24 → 32).
- `TreeView::Node` holds one boxed pair (`Node(Box<(BinaryTree, BinaryTree)>)`):
  `L − 1` allocations per tree, `BinaryTree<A>` 24 → 16 bytes; `Debug` output
  unchanged (`Node(<left>, <right>)`). Breaking for by-hand matches on the
  variant (#200).
- `depth` is opt-in: `MAX_TREE_DEPTH`, `guard_*`, `tree_depth`,
  `free_mnd_depth`, `DepthError` stay published; no crate entry calls them.

## [workspace-v0.11.0] - 2026-08-10

### Changed

- `rand` leaves this crate's lib graph
  ([#239](https://github.com/sustia-llc/catgraph/issues/239), via catgraph-applied).
- **BREAKING:** the endofunctor/carrier substrate is crate-owned;
  `deep_causality_haft` removed
  ([#222](https://github.com/sustia-llc/catgraph/issues/222)). `Satisfies` /
  `NoConstraint` gone (`EndoWitness = HKT + Functor<Self>`); the 15 re-exported
  names change nominal identity; not carried: `Free::{bind, map, lift, pure}`
  (inherent), `Cofree::{map, extend, extract, duplicate}`, carrier `Clone`,
  `CofreeWitness: Functor`, `OptionWitness: Monad`, carrier `Eq`. Trait and
  carrier shapes derive from haft 0.4.2 (MIT, attributed in each file).
  Unconditional external dependency set is now `thiserror` alone.

## [workspace-v0.10.0] - 2026-08-09

### Added

- Optional `serde` feature: derives on `RModule<S>`, `DirectSum<A, B>` and
  (with `ad`) `Dual<T>` ([#230](https://github.com/sustia-llc/catgraph/issues/230)).
  Deserialization checks nothing beyond the constructors; non-finite floats
  serialize as JSON `null` and do not load back.
- `depth` and `errors` modules: `MAX_TREE_DEPTH = 256`, `tree_depth` /
  `free_mnd_depth`, `guard_tree_depth` / `guard_free_mnd_depth`,
  `DepthError::TreeDepthExceeded { depth, limit }` (`#[non_exhaustive]`,
  also re-exported at the crate root); `thiserror` dependency
  ([#231](https://github.com/sustia-llc/catgraph/issues/231)).

### Changed

- **BREAKING:** `tree_to_free_mnd`, `free_mnd_to_tree`, `RecursiveNn::unroll`
  return `Result<_, DepthError>` (#231; reverted in v0.15.0).
- `benches/free_cofree_shapes.rs` caterpillar axis is
  `SPINE_LEAVES = [16, 64, MAX_TREE_DEPTH]`.

### Fixed

- Browser-wasm lib builds no longer fail in `getrandom`
  ([#232](https://github.com/sustia-llc/catgraph/issues/232), via catgraph-applied).

## [workspace-v0.9.0] - 2026-08-04

### Added

- `para::dual::Dual<T>` is crate-owned: `new`/`constant`/`variable`/`value`/
  `derivative`, `Add`/`Sub`/`Mul`/`Neg`/`Div`/`AddAssign`/`MulAssign`,
  `Mul<T>`, `Zero`/`One`; re-exported at `para::ad::Dual`. Not carried from
  the replaced type: `Sum`, `Product`, `FromPrimitive`, `Display`, `Default`,
  the analytic-scalar marker traits
  ([#221](https://github.com/sustia-llc/catgraph/issues/221)).

### Changed

- `deep_causality_num` and `deep_causality_num_dual` removed; `Zero`/`One`
  resolve to `catgraph_applied::rig` and are re-exported at the crate root;
  `ad = []` declares no dependency
  ([#219](https://github.com/sustia-llc/catgraph/issues/219), #221).

## [workspace-v0.6.0] - 2026-08-02

### Added

- `benches/free_cofree_shapes.rs` — construction, `fold`, `unfold` and the
  lazy iterators across list, balanced-tree and caterpillar shapes, with a
  counting allocator ([#156](https://github.com/sustia-llc/catgraph/issues/156)).

## [workspace-v0.5.0] - 2026-07-30

### Added

- Off-by-default `ad` feature: `para::ad` with `Dual` (then from
  `deep_causality_num_dual =0.1.4`), `DualF64Module`, `seed`, `gradient`;
  `tests/ad_module_laws.rs`; example `gradient_descent_para`; CI `ad` lane
  ([#74](https://github.com/sustia-llc/catgraph/issues/74)).

### Changed

- R-module actegory generic in the scalar: `RModule<S>`, `RObject<S>`,
  `RMorphism<S>`, `RMonoidal<S>`, `RActegory<S>` with per-method bounds,
  re-exported from `para`; `DirectSum::flatten` generic over
  `DirectSum<RModule<S>, RModule<S>>`; `F64*` become aliases. Breaking: the
  four former unit structs lose bare-value construction and their `Debug`
  names change (#74).

## [workspace-v0.4.0] - 2026-07-25

### Changed

- `deep_causality_haft` / `deep_causality_num` pins `=0.3.3` → `=0.4.0`
  ([#69](https://github.com/sustia-llc/catgraph/issues/69)), then haft
  `=0.4.1` → `=0.4.2`; carrier `Clone` unadopted
  ([#154](https://github.com/sustia-llc/catgraph/issues/154)).
- **BREAKING:** `free_monad` adopts haft's `Free`/`Cofree`: `FreeMnd` /
  `CofreeCmnd` → `Free` / `Cofree` (with `FreeWitness` / `CofreeWitness`,
  `EqFunctor` / `DebugFunctor` re-exported at the crate root), variants
  `Pure`/`Suspend` with the box inside the functor hole, private `Cofree`
  fields, no carrier `Clone`, opt-in `Eq`/`Debug` via `EqFunctor` /
  `DebugFunctor` (`ListEndo` / `TreeEndo` implement both); gains `Free::fold`
  and `Cofree::unfold` ([#93](https://github.com/sustia-llc/catgraph/issues/93)).
- Paper-audit phase 5: "Appendix K" → J; THEOREM_MAP functor-laws row Def
  1.5 → 1.4; `container.rs` cites §4 "New Horizons"; unroller catalogue
  cites Ex J.1–J.5 with Remark 2.13 / Remark H.6.
- **BREAKING:** `MonoidalCategory::tensor_morphisms` required
  ([#65](https://github.com/sustia-llc/catgraph/issues/65)).
- **BREAKING:** `EndoFunctor` replaced by haft `HKT` + `Functor` witnesses:
  the five `EndoFunctor` paths (`catgraph_dl::`, `endofunctor::`, `algebra::`,
  `free_monad::`, `free_monad::free_mnd::`) removed; `catgraph_dl::{HKT,
  Functor, EndoWitness, NoConstraint, Satisfies, Either}` added at the crate
  root and through `endofunctor`, `{HKT, Functor, EndoWitness}` through
  `algebra` and `free_monad`; `either` dependency dropped
  ([#12](https://github.com/sustia-llc/catgraph/issues/12)).
- `deep_causality_num` reservation re-anchored to #36.

### Added

- `UnfoldingRnn::unroll_iter`, `MealyCell::run_iter`, `MooreCell::run_iter`
  — lazy iterators; poisoned after a caught panic
  ([#36](https://github.com/sustia-llc/catgraph/issues/36)).
- `examples/`: `para_walkthrough`, `weight_tying`, `free_monad_basics`,
  `architecture_unrollers`, run in CI
  ([#34](https://github.com/sustia-llc/catgraph/issues/34)).
- `F64Module` R-module actegory `(FinReal, ⊕, R⁰)`: `F64Module`,
  `DirectSum<A, B>`, `F64Monoidal`, `F64Actegory`, `F64Object`,
  `F64Morphism`; `tests/module_actegory_laws.rs`; `deep_causality_num`
  moves from deps-only to used (#36).
- Coalgebra-direction unroller equivalence tests against
  `Cofree<OptionWitness, O>` ([#64](https://github.com/sustia-llc/catgraph/issues/64)).
- `tests/THEOREM_MAP.md` ([#70](https://github.com/sustia-llc/catgraph/issues/70)).
- `full_monad_algebra_hom_certification_recipe` test
  ([#67](https://github.com/sustia-llc/catgraph/issues/67)).
- `endofunctor` re-exports `Monad`; `GroupActionEndo<G>: Monad` (writer
  monad); `MonadAlgebra::verify_unit_law` / `verify_assoc_law`,
  `MonadAlgebraHom::verify_unit_coherence` / `verify_mult_coherence`;
  pentagon/triangle equations on `MonoidalCategory`;
  `tests/monoidal_coherence_laws.rs`, `tests/monad_algebra_laws.rs`, proptests
  for `verify_commutes` and `FreeMnd`-equivalence
  ([#40](https://github.com/sustia-llc/catgraph/issues/40)).
- `endofunctor` re-exports `Pure`, `NaturalIso`, `OptionWitness`,
  `assert_natural_iso_round_trip` / `assert_natural_iso_naturality`;
  `natural::NaturalTransformation`, `natural::IsoForward` / `IsoBackward`,
  `natural::Pointed` (`GroupActionEndo: Pure`), `container::Container` with
  `ListEndo` / `TreeEndo` / `GroupActionEndo` instances;
  `tests/natural_pointed_laws.rs`, `tests/container_laws.rs`
  ([#41](https://github.com/sustia-llc/catgraph/issues/41)).
- `EndoWitness` supertrait alias; `tests/functor_laws.rs`.

### Documentation

- `FreeMnd`/`CofreeCmnd` kept native ([#76](https://github.com/sustia-llc/catgraph/issues/76));
  `tie_weights`' `P: Clone` bound and the `SetCategoryDefaults` dual-impl
  pattern documented ([#42](https://github.com/sustia-llc/catgraph/issues/42)).

> Workspace tags `v0.1.0`–`v0.3.0` have no sections here
> ([#158](https://github.com/sustia-llc/catgraph/issues/158)). The `[0.4.1]` /
> `[0.4.0]` headings below are pre-reboot crate-local versions.

## [0.4.1] - 2026-05-10

### Fixed

- CHANGELOG link references.

### Changed

- Test and doc text updated for the v0.4.0 `tie_weights` signature.

## [0.4.0] - 2026-05-10

### Added

- `tie_weights`, `Reparameterization::apply`, `ParaMorphism::compose`
  generic over `C: Actegory<SetMonoidal>` (explicit turbofish callers add
  `C` at the leftmost position).
- `para::Sealed`: `SetCategoryDefaults: private::Sealed + Sized` (dual-impl
  opt-in).
- `docs/AUDIT-CHECKPOINT-v0.4.0.md`.

## [0.3.1] - 2026-05-06

### Changed

- `SetCategoryDefaults: Sized`; coherence-conflict caveat documented;
  `SetCategoryDefaults` doctest covers all five methods.

## [0.3.0] - 2026-05-06

### Added

- `para::SetCategoryDefaults` opt-in blanket `MonoidalCategory`;
  `SetMonoidal` opts in.
- `tests/coalition_consumption_simulation.rs`.
- `docs/2402.15332v2-SUMMARY.md`, `docs/2402.15332v2-AUDIT.md`; Hopf-fibration
  evidence note (no preprint as of 2026-05-06).

## [0.2.0] - 2026-05-02

### Added

- `MonoidalCategory` gains `Unit`, `Tensor<A, B>`, `tensor_objects`, `unit`,
  `associate`, `left_unitor`, `right_unitor`; `Actegory<M>` gains
  `ActionResult<P, X>`, `act`, `compose_action`; `Comonoid` gains
  `comultiply`, `counit`.
- `para::SetMonoidal`, `SetActegory`, `SetObject`, `SetMorphism`,
  `DiagonalComonoid`, `tie_weights`, `ParaMorphism::compose` / `apply`,
  `Reparameterization::apply`.
- `algebra::EndoFunctor`, `Group`, `Z2Group`, `GroupActionEndo<G>`,
  `FAlgebraHom`, `FCoalgebraHom`, `MonadAlgebraHom` with `verify_commutes`.
- `free_monad::FreeMnd` / `CofreeCmnd` bodies, `ListEndo<A>`, `TreeEndo<A>`,
  `BinaryTree<A>`, `vec_to_free_mnd` / `free_mnd_to_vec`,
  `tree_to_free_mnd` / `free_mnd_to_tree`; `either` dependency.
- `FoldingRnn::unroll`, `RecursiveNn::unroll`, `UnfoldingRnn::unroll_to_vec`,
  `MealyCell::run`, `MooreCell::run`.
- Tests: `para_composition`, `comonoid_laws`, `algebra_homomorphisms`,
  `free_monad_bijections`, `architecture_unrollers`.

## [0.1.0] - 2026-05-02

### Added

- Scaffold: `para::{Para, ParaMorphism, Reparameterization, Comonoid,
  MonoidalCategory, Actegory}`, `algebra::{FAlgebra, FCoalgebra,
  MonadAlgebra}`, `free_monad::{FreeMnd, CofreeCmnd}`, the five
  `architectures` cells, re-exports of `Rig`, `UnitInterval`, `Tropical`,
  `F64Rig`, `BoolRig`, `EnrichedCategory`, `HomMap`, `LawvereMetricSpace`;
  private `hopf_fibration` stub.

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.16.0...HEAD
[workspace-v0.15.0]: https://github.com/sustia-llc/catgraph/compare/v0.14.0...v0.15.0
[workspace-v0.11.0]: https://github.com/sustia-llc/catgraph/compare/v0.10.0...v0.11.0
[workspace-v0.10.0]: https://github.com/sustia-llc/catgraph/compare/v0.9.0...v0.10.0
[workspace-v0.9.0]: https://github.com/sustia-llc/catgraph/compare/v0.8.0...v0.9.0
[workspace-v0.6.0]: https://github.com/sustia-llc/catgraph/compare/v0.5.0...v0.6.0
[workspace-v0.5.0]: https://github.com/sustia-llc/catgraph/compare/v0.4.0...v0.5.0
[workspace-v0.4.0]: https://github.com/sustia-llc/catgraph/compare/v0.2.1...v0.4.0
[0.4.1]: https://github.com/tsondru/catgraph/releases/tag/catgraph-dl-v0.4.1
[0.4.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-dl-v0.4.0
[0.3.1]: https://github.com/tsondru/catgraph/releases/tag/catgraph-dl-v0.3.1
[0.3.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-dl-v0.3.0
[0.2.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-dl-v0.2.0
[0.1.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-dl-v0.1.0
