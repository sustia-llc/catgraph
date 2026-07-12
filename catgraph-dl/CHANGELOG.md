# Changelog — catgraph-dl

All notable changes to this crate are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); semver per
[SemVer 2.0.0](https://semver.org/spec/v2.0.0.html).

> **Lineage note:** pre-reboot version links below (`catgraph-dl-v0.x` tags)
> point at the private predecessor repo `tsondru/catgraph` and will not
> resolve publicly; they are kept as an honest record of the crate's history.
> In-tree paper PDFs mentioned in historical entries were removed from the
> tree on 2026-07-10 (arXiv licensing); fetch papers from the arXiv links in
> `docs/`.

## [Unreleased]

### Added

- **`F64Module` R-module actegory — first non-`(Set, ×, 1)` `MonoidalCategory`
  ([#36](https://github.com/sustia-llc/catgraph/issues/36), first bullet).**
  Lands the direct-sum monoidal category `(FinReal, ⊕, R⁰)` of
  finite-dimensional real modules and its self-action, and **activates the
  reserved `deep_causality_num` dependency** (previously deps-only): the crate
  now imports num's root `Zero` / `One`. The umbrella #36 stays open for the
  still-deferred non-Set surfaces (hyperdoctrine / vector-bundle / fibration
  actegories; lazy final-coalgebra unrolling).
  - **`para::F64Module`** (`src/para/module_actegory.rs`) — a
    finite-dimensional real module `Rⁿ` over the scalar ring `R = f64`,
    `Vec<f64>`-backed, the free `R`-module on `n` generators. Carries genuine
    `R`-module structure: `zeros` (additive identity `0 ∈ Rⁿ`, each entry
    `<f64 as Zero>::zero()`), `basis` (standard basis `eᵢ` with
    `<f64 as One>::one()` at position `i`), `add` (dimension-guarded, `Option`),
    `scale` (`r · v`), and `direct_sum` (`⊕`, coordinate concatenation
    `Rᵐ ⊕ Rⁿ = Rᵐ⁺ⁿ`). Plus `zero_dim` (the monoidal unit `R⁰`).
  - **`para::DirectSum<A, B>`** — the object-level tensor carrier `A ⊕ B`, a
    dedicated newtype (**not** the `(Set, ×, 1)` tuple), with
    `DirectSum::<F64Module, F64Module>::flatten` realising `V ⊕ W` as one
    concatenated module.
  - **`para::F64Monoidal`** — the `MonoidalCategory` `(FinReal, ⊕, R⁰)`. The
    first non-`(Set, ×, 1)` instance: **hand-written** `MonoidalCategory` impl
    with `Tensor<A, B> = DirectSum<A, B>` and `Unit = ()` (the one-element
    module `R⁰`); it does **not** opt into `SetCategoryDefaults`. Kind markers
    **`para::F64Object`** / **`para::F64Morphism`**. Zero-sized — the base ring
    `f64` is a compile-time type, so the `&self` runtime-payload slot stays
    reserved for a runtime-ring instance (documented).
  - **`para::F64Actegory`** — the self-action `▶ = ⊕` of `F64Monoidal` on
    itself (`Actegory<F64Monoidal>`), with `act(p, x) = p ⊕ x`, the
    multiplicator `µ : Q ▶ (P ▶ X) → (Q ⊗ P) ▶ X` as the exact `DirectSum`
    re-association; the concrete concatenation is realised by
    `DirectSum::flatten` on the action result.
  - **Paper anchors (verified against [arXiv:2402.15332v2](https://arxiv.org/abs/2402.15332)).** The formal
    actegory definition is **Definition E.2** (`η_X : I ▶ X ≅ X`,
    `µ_{M,N} : (M ⊗ N) ▶ X ≅ M ▶ (N ▶ X)`, pentagonator Eq. 7 + unitors Eq. 8),
    **not** §3.1 (which is the main-body `Para` section). The self-action is
    **Example E.4**. The monoidal product is the **direct sum `⊕`**, not the
    tensor `⊗_R`: **Example G.3** forms `Para(Smooth)` over the *cartesian*
    structure of real vector spaces, and for finite-dimensional modules the
    categorical product is the biproduct `Rᵐ × Rⁿ ≅ Rᵐ⁺ⁿ = Rᵐ ⊕ Rⁿ`. The
    tensor `⊗_R` is a different (closed) monoidal structure (unit `R¹`), not the
    parameter-concatenation gradient-based-learning `Para` uses.
  - **Law tests** — `tests/module_actegory_laws.rs` (deterministic + proptest):
    Mac Lane pentagon / triangle / unitor coherence on `DirectSum` via
    `common::assert_monoidal_coherence` (generic over `MonoidalCategory` since
    [#65](https://github.com/sustia-llc/catgraph/issues/65); the initially-added
    `DirectSum`-specific `assert_direct_sum_coherence` was folded into it);
    the `R`-module axioms (`v + 0 = v`, `1 · v = v`, `0 · v = 0`,
    basis coherence) via `common::assert_f64_module_axioms` — the identities
    where `Zero` / `One` are load-bearing; and the concrete `⊕`-monoid laws
    (dimensions add, `R⁰` unit, associativity, `flatten`/`act` agreement) via
    `common::assert_direct_sum_monoid`. Float honesty: the identity asserts
    (`1 · v`, `0 · v`, `v + 0`) hold under `f64` `PartialEq` (which identifies
    `±0.0`) for finite inputs — signed-zero bit patterns are **not** preserved
    (`0.0 · (-1.0) = -0.0`; `-0.0 + 0.0 = +0.0`; cf. #58); samples use the
    NaN-free `finite_f64` strategy.
- **Coalgebra-direction unroller equivalence tests ([#64](https://github.com/sustia-llc/catgraph/issues/64)).**
  `tests/architecture_unrollers.rs` gains three `CofreeCmnd`-equivalence tests
  (+3 proptest lifts) for `UnfoldingRnn`/`MealyCell`/`MooreCell`: each bounded
  unroll equals the walk of a `CofreeCmnd<OptionWitness, O>` stream prefix
  unfolded from the same seed — witnessing the wrapper as the finite prefix of
  the unique coalgebra hom into the terminal `(O × −)`-coalgebra (CDL Remark
  H.6, App I.3/I.4/I.5). Test-only; no API change.
- **`tests/THEOREM_MAP.md` law-test → paper-anchor registry ([#70](https://github.com/sustia-llc/catgraph/issues/70), Part 1).**
  The correctness spine: every law test mapped to its paper anchor (the paper is
  the proof layer; Kani deferred). New law tests update their row in the same PR.
- **Full monad-algebra-hom certification recipe test ([#67](https://github.com/sustia-llc/catgraph/issues/67)).**
  `monad_algebra_laws::full_monad_algebra_hom_certification_recipe` exercises the
  three-part recipe end to end — source algebra lawful ∧ target algebra lawful ∧
  hom square (CDL Def 2.3 + Def 2.5) — positively (abs-value hom) and against
  three negatives each failing exactly one part. A recipe paragraph was added to
  the `MonadAlgebraHom` rustdoc; no `verify_all` convenience (minimal-ceremony).
  Test/doc-only; no API change.

### Documentation

- **`FreeMnd`/`CofreeCmnd` kept native vs haft 0.4.0's `Free`/`FreeWitness`
  ([#76](https://github.com/sustia-llc/catgraph/issues/76)).** Decision recorded
  in the `free_monad` module doc: the native carriers stay (haft `Free` ships no
  `Eq`/`Debug`, has no `CofreeCmnd` twin, and the minimal `pure`/`roll`/`new`
  surface is deliberate); the seam does not re-export haft's `Free`. No API change.
- **Ergonomics-batch verdicts ([#42](https://github.com/sustia-llc/catgraph/issues/42)).**
  Three "small ergonomics" items triaged; all resolve to documentation, no
  API change:
  - `tie_weights`'s `P: Clone` bound documented as **semantic, not
    incidental** — `Clone` is the Rust witness of the comonoid comultiplication
    `δ(p) = (p, p)` in `(Set, ×, 1)` (CDL Theorem G.10); a `P` that cannot be
    duplicated has no diagonal comonoid, so relaxing the bound would change
    what `tie_weights` means. The canonical statement lives on the `Comonoid`
    trait (the bound's home); `tie_weights` links to it, and the audit doc's
    forward-look item is annotated resolved-won't-do (`Arc<T>` is cheaply
    `Clone`, so the motivating heap-shared case was already served).
  - `#[doc(hidden)]` on `private::Sealed` verified **already structural** — the
    `private` module lives inside the private `monoidal_category` module, so
    `para::private::Sealed` is unreachable; the only public path is the
    deliberate `para::Sealed` re-export, which downstream must name for the
    dual-impl soft-seal opt-in. No change.
  - Documented dual-impl pattern chosen over a `#[derive(SetCategoryDefaults)]`
    proc-macro (two impl lines don't justify a macro crate — alpha /
    minimal-ceremony posture); the concrete opt-in snippet (imports included,
    mirroring the compile-checked doctest) is now inlined in the README `para`
    section, and the design-history note in `monoidal_category.rs` cross-links
    the re-affirmed rejection of option (β). Stale "sub-trait" wording for
    `SetCategoryDefaults` in `lib.rs`/`README.md` corrected to "opt-in marker
    trait".

### Changed

- **DC substrate pins bumped `=0.3.3` → `=0.4.0` ([#69](https://github.com/sustia-llc/catgraph/issues/69)).**
  `deep_causality_haft` 0.4.0 / `deep_causality_num` 0.4.0 (DeepCausality
  v0.14.0, released 2026-07-08). Version-only for this crate: the consumed
  seam surface was re-verified against the released crate sources — the
  `crate::endofunctor` re-export root paths, `HKT`/`Functor`/`Pure`/`Monad`
  (+provided `join`)/`NaturalIso` signatures, the `iso::test_support` helper
  path, the `Satisfies<NoConstraint>` blanket, `Either`, `OptionWitness`, and
  num's root `{Zero, One}` are all unchanged. haft 0.4.0 still ships no
  `Pointed`/`NaturalTransformation` (the upstream proposal remains
  [#62](https://github.com/sustia-llc/catgraph/issues/62)). Workspace fallback
  git rev updated to the 0.4.0 release commit `3280cb844`.

### Changed — BREAKING

- **`MonoidalCategory::tensor_morphisms` added ([#65](https://github.com/sustia-llc/catgraph/issues/65)).**
  The `MonoidalCategory` trait gains a required applying-form morphism-tensor
  method `fn tensor_morphisms<A, B, C, D>(&self, Self::Tensor<A, B>,
  impl FnMut(A) -> C, impl FnMut(B) -> D) -> Self::Tensor<C, D>` (CDL §3.1 — the
  morphism map of `⊗`). Breaking for any external implementor (no generic
  default body is possible; the two in-tree impls — the `SetCategoryDefaults`
  blanket `(a, b) ↦ (f(a), g(b))` and the direct `F64Monoidal` impl
  `DirectSum(a, b) ↦ DirectSum(f(a), g(b))` — supply it). With the `α ⊗ id` /
  `id ⊗ α` legs now expressible on the trait surface, the two per-instance
  coherence checkers collapse into one generic
  `common::assert_monoidal_coherence<M: MonoidalCategory>` exercising both the
  `(Set, ×, 1)` tuple and `DirectSum` carriers, and the "spelled manually"
  per-instance caveats are dropped.
- **EndoFunctor→haft migration ([#12](https://github.com/sustia-llc/catgraph/issues/12)).**
  The hand-rolled `EndoFunctor` trait (a GAT `type Apply<X>` plus `fmap`) is
  removed in favour of `deep_causality_haft` v0.3.3's `HKT` (object map
  `HKT::Type<X>`) + `Functor<F>` (morphism map `fmap`) witnesses. The witnesses
  `ListEndo<A>`, `TreeEndo<A>`, and `GroupActionEndo<G>` now `impl HKT + Functor<Self>`
  with `type Constraint = NoConstraint`; `FreeMnd<F, Z>` / `CofreeCmnd<F, Z>` and
  the `FAlgebraHom` / `FCoalgebraHom` verifiers are bounded `F: EndoWitness` (the
  new supertrait alias — see Added), and the recursive `Roll` / `tail` payloads
  project through `F::Type<…>`.
  - **Removed public paths** (all five cease to exist — alpha posture, no
    deprecation shim): `catgraph_dl::EndoFunctor`,
    `catgraph_dl::endofunctor::EndoFunctor`, `catgraph_dl::algebra::EndoFunctor`,
    `catgraph_dl::free_monad::EndoFunctor`, and
    `catgraph_dl::free_monad::free_mnd::EndoFunctor` (`free_mnd` is a `pub mod`
    and carried its own `pub use`).
  - **New public paths**: `catgraph_dl::{HKT, Functor, EndoWitness, NoConstraint, Satisfies, Either}`
    re-exported at the crate root and through `crate::endofunctor` (the single
    import seam), plus `{HKT, Functor, EndoWitness}` through `algebra` and
    `free_monad`.
- `TreeEndo`'s `A + (−)²` sum is now `deep_causality_haft::Either` (was the
  external `either` crate). The `either` dependency is dropped from
  `catgraph-dl` (the workspace entry stays — `catgraph` core / applied still use
  it).

### Added

- **Machine-checked coherence laws + property-based verification
  ([#40](https://github.com/sustia-llc/catgraph/issues/40)).** Discharges the
  two deferred surfaces "machine-checked `MonadAlgebraHom` coherence laws" and
  "property-based exhaustive testing of `verify_commutes` / `FreeMnd`-equivalence".
  - **`Monad` seam extension** (`src/endofunctor.rs`): the single import seam now
    also re-exports haft's `Monad`; `GroupActionEndo<G>` gains an
    `impl Monad<Self>` — the writer monad over the monoid `G`
    (`bind((g, x), f) = { let (g2, y) = f(x); (g · g2, y) }`, so
    `join((g1, (g2, x))) = (g1 · g2, x)` = the μ documented in
    `monad_algebra.rs`). Monad laws discharged by the `Group` contract
    (CDL Def 2.1 / Ex 2.2).
  - **Monad-algebra coherence verifiers** (`src/algebra/monad_algebra.rs`), all
    sample-based (mirroring `FAlgebraHom::verify_commutes`'s caller-sampled
    honesty; `η = ` haft's `Pure`, `μ = ` haft's provided `Monad::join`):
    `MonadAlgebra::verify_unit_law` (`a ∘ η = id`) /
    `MonadAlgebra::verify_assoc_law` (`a ∘ M(a) = a ∘ μ`), both CDL Def 2.3,
    and `MonadAlgebraHom::verify_unit_coherence` (`M(f) ∘ η_A = η_B ∘ f` —
    η-naturality, CDL Def 1.5 applied to `η`) /
    `MonadAlgebraHom::verify_mult_coherence` (`f ∘ a ∘ M(a) = f ∘ a ∘ μ_A` —
    Def 2.3's associativity post-composed with `f`). The two hom-side checks
    hold for *any* `f` between lawful algebras of a lawful monad and cannot
    reject a non-homomorphism — the discriminating condition stays
    `verify_commutes` (CDL Def 2.5); documented as a ⚠️ scope note on
    `MonadAlgebraHom`. Law-tested in `tests/monad_algebra_laws.rs`: positive
    deterministic + proptest over the `Z2` action on `Vec<f64>`, negative
    unlawful-algebra cases, a non-hom boundary demonstration, and exhaustive
    writer-monad laws over a non-abelian test-local `S3` pinning the
    `g1 · g2` accumulation order.
  - **Pentagon / triangle coherence** for the monoidal surface. The
    `MonoidalCategory` trait rustdoc gains the Mac Lane pentagon + triangle
    equations as implementor obligations (previously absent); the `(Set, ×, 1)`
    blanket bodies are machine-checked against `SetMonoidal` and a fresh
    downstream-style ZST in `tests/monoidal_coherence_laws.rs`, driven by a new
    witness-generic `assert_monoidal_coherence` helper in `tests/common/mod.rs`
    (the `α ⊗ id` / `id ⊗ α` legs spelled manually — the trait currently has no
    morphism-tensor operation; adding one is
    [#65](https://github.com/sustia-llc/catgraph/issues/65)). Mac Lane; CDL §3.1.
  - **Proptest coverage for `verify_commutes` + `FreeMnd`-equivalence.**
    `tests/algebra_homomorphisms.rs` proptests the abs-value equivariance square
    (holds for all samples) and the projection failure (fails for all `x[0] ≠ 0`
    under `g = true`); `tests/architecture_unrollers.rs` proptests the list- and
    tree-direction `FreeMnd`-equivalence over generated inputs (bounded `Vec<i64>`
    ≤ 16, bounded `BinaryTree<u8>`), reusing hoisted module-level walk helpers.
- **`NaturalTransformation` / `Pointed` / `Container` first-class surfaces
  ([#41](https://github.com/sustia-llc/catgraph/issues/41)).** The three
  surfaces cg-dl previously documented as deferred obligations are now shipped,
  built on the haft witness substrate:
  - **`natural::NaturalTransformation<F, G>`** (`src/natural.rs`) — the
    component family `α_X : F(X) → G(X)` of a natural transformation `α : F ⇒ G`
    (Gavranović et al., ICML 2024, Def 1.5), a static method on a zero-sized
    witness. Adapter witnesses **`natural::IsoForward`** / **`natural::IsoBackward`**
    lift any haft `NaturalIso<F, G>` to its two directions (separate types
    because a pair of blanket impls would overlap).
  - **`natural::Pointed`** — blanket marker for a pointed endofunctor `(F, σ)`
    with `σ = ` haft's `Pure` (CDL Def B.3). `GroupActionEndo<G>` gains a
    `Pure<Self>` impl (`σ(x) = (G::identity(), x)`, the writer-functor point) as
    the crate's own inhabitant; haft witnesses reachable through the seam
    (e.g. `OptionWitness`) are also pointed via their upstream `Pure` impls and
    are law-tested alongside. `ListEndo` / `TreeEndo` are documented
    non-instances — the former's only natural point (constant `None`)
    trivialises every pointed-algebra; the latter's diagonal point is natural
    but not representable under `Pure`'s no-`Clone` signature.
  - **`container::Container`** (`src/container.rs`) — the shape/position
    presentation of a polynomial endofunctor `⟦S ◁ P⟧(X) = Σ_{s} X^{P(s)}`
    (Abbott–Altenkirch–Ghani 2003, via CDL), finitary (`Vec`-of-contents)
    presentation. Instances for `ListEndo<A>`, `TreeEndo<A>`, `GroupActionEndo<G>`
    live next to each witness definition.
  - **Seam extension** (`src/endofunctor.rs`): the single import seam now also
    re-exports haft's `Pure`, `NaturalIso`, `OptionWitness`, and the public
    natural-iso law helpers `assert_natural_iso_round_trip` /
    `assert_natural_iso_naturality` (reachable only via `iso::test_support`).
  - **Law tests** — `tests/natural_pointed_laws.rs` (NT naturality via iso
    adapters over a genuine `ListEndo<()> ≅ OptionWitness` iso, plus a
    hand-written non-iso `ListEndo<i32> ⇒ ListEndo<i64>` NT, plus `Pointed`
    σ-naturality) and `tests/container_laws.rs` (round-trip, arity coherence
    including recompose rejection, `fmap` coherence over every witness shape),
    driven by new witness-generic helpers in `tests/common/mod.rs`.
  - The upstream proposal to add `Pointed` / `NaturalTransformation` to haft
    itself is tracked as [#62](https://github.com/sustia-llc/catgraph/issues/62).
- **`EndoWitness`** (`src/endofunctor.rs`) — a blanket-implemented supertrait
  alias `HKT<Constraint = NoConstraint> + Functor<Self>` packaging "endofunctor
  on Set". Restores the type-level invariant the old fused `EndoFunctor` trait
  carried: a bare `HKT` bound would admit an fmap-less carrier, whereas
  `EndoWitness` requires both the object map and the morphism map. All carriers
  (`FreeMnd` / `CofreeCmnd` / the F-(co)algebra verifiers) bound on it;
  witnesses never name it (blanket impl).
- Functor-law tests (identity + composition) for all three witnesses, in a new
  `tests/functor_laws.rs`, driven by a single witness-generic
  `assert_functor_laws<F: EndoWitness>` helper in `tests/common/mod.rs` (proptest
  strategies for `ListEndo` / `TreeEndo`, sample values for `GroupActionEndo`).
  These reify the previously documentation-only law obligations (haft `Functor`
  law docs; Gavranović et al., ICML 2024). The four byte-identical trivial
  unit-projection test witnesses were collapsed into one generic
  `common::UnitEndo<Tag>`.

### Changed

- `deep_causality_num` reservation re-anchored from #12 to
  [#36](https://github.com/sustia-llc/catgraph/issues/36) (R-module / `F64Module`
  surfaces need `Zero` / `One`); it remains deps-only. `deep_causality_haft` is
  now in use, not deps-only.

## [0.4.1] - 2026-05-10

Patch release applying review findings on top of v0.4.0. Strictly additive; no API break; no behaviour change.

### Fixed

- CHANGELOG footer was missing the `[0.4.0]` link reference and `[Unreleased]` comparator still pointed at `catgraph-dl-v0.3.1`. Added `[0.4.1]` + `[0.4.0]` link refs; updated `[Unreleased]` to compare against `catgraph-dl-v0.4.1`.

### Changed

- `tests/coalition_consumption_simulation.rs:23` module-level rustdoc updated from stale 4-param `tie_weights::<P, _, X, Y>(...)` example to 5-param explicit form matching the v0.4.0-widened signature.
- `tests/coalition_consumption_simulation.rs:28-44` rationale comment block updated: v0.3.x "SetActegory-bound" language replaced with v0.4.0 "test uses SetActegory as a conservative caller choice; v0.4.0+ permits any `Actegory<SetMonoidal>`" framing.
- `src/lib.rs` "Deferred surfaces" bullet removed: `tie_weights` actegory-generalisation is the headline of v0.4.0, no longer deferred.
- `docs/2402.15332v2-AUDIT.md` row 115 (`Reparameterization::apply`) gains v0.4.0 widening note.
- `docs/2402.15332v2-AUDIT.md` row 116 (`ParaMorphism::compose`) gains v0.4.0 widening note.
- `docs/2402.15332v2-AUDIT.md` row 117 (Theorem G.10) pinned: comonoid lives in `M`, NOT in `C` (explicit paper-fidelity statement).
- `src/para/monoidal_category.rs:43-50` rustdoc citation tightened to clarify the static-vs-runtime dispatch distinction.

### Verification

- `cargo test -p catgraph-dl`: 42 tests + 3 doctests + 5 ignored, all green.
- `cargo clippy -p catgraph-dl --all-targets -- -W clippy::pedantic`: zero new warnings.
- `cargo doc -p catgraph-dl --no-deps`: zero warnings.

## [0.4.0] - 2026-05-10

Minor release closing four architectural items from the v0.4.0 forward-look. Headline: `tie_weights` actegory-genericity widening enabling downstream consumers to consume `tie_weights` against their own `Actegory<SetMonoidal>` ZSTs directly. Strictly additive for inference-relying callers; explicit-turbofish callers move from 4-parameter to 5-parameter on `tie_weights`.

### Added

- **`§1.1`** `tie_weights<C: Actegory<SetMonoidal>>` widening at `src/para/comonoid.rs`. v0.3.1's `SetActegory`-bound signature is generalised to `C: Actegory<SetMonoidal>` at the leftmost generic position. Body untouched (the diagonal collapse delegates to `Reparameterization::apply`, which is itself widened — see below).
- **`§1.1`** `Reparameterization::apply<C, PNew, POld, F, X, Y>` widening at `src/para/reparameterization.rs`. Threads `C: Actegory<SetMonoidal>` through the input morphism + return type.
- **`§1.1`** `ParaMorphism::compose<C, P, F>` widening at `src/para/morphism.rs` (impl-block-level generic + return type).
- **`§1.2`** HKT `&self` rationale validation paragraph at `src/para/monoidal_category.rs` "## Why methods take `&self`" + `src/para/actegory.rs` "## Why methods take `&self`". Cites coalition v0.5.0 as the validating consumer.
- **`§1.2`** `docs/AUDIT-CHECKPOINT-v0.4.0.md` — three concrete predicates for the coalition v0.5.0 post-shipping `&self` audit + verdict skeleton (ratify / open follow-up).
- **`§1.4`** `pub mod private { pub trait Sealed {} }` soft-seal at `src/para/monoidal_category.rs`. `SetCategoryDefaults: private::Sealed + Sized` bound forces downstream commitment-signal via dual impl (`impl Sealed for X` + `impl SetCategoryDefaults for X`). `Sealed` re-exported at `catgraph_dl::para::Sealed` for downstream access.
- **`§1.6`** Pin-bump chain forward-planning section at `CLAUDE.md` documenting the v0.13.0 → v0.13.5 trajectory + coalition's pin trajectory.
- "## Audit checkpoints" section at `CLAUDE.md` registering the v0.4.0 audit-checkpoint as OPEN pending coalition v0.5.0 review.
- Workspace `CLAUDE.md` umbrella tag table gains `v0.13.5` row.

### Changed

- `Cargo.toml` version `0.3.1 → 0.4.0`.
- `SetCategoryDefaults` trait bound widened from `: Sized {}` to `: private::Sealed + Sized {}` — downstream that already opted in at v0.3.1 (none known) must add `impl Sealed for X` alongside the existing `impl SetCategoryDefaults for X {}`. Zero-impact migration: no external downstream consumer exists at v0.4.0 design time; coalition v0.5.0 (slot 2) is the first.
- `SetMonoidal` (in-tree consumer) adopts the dual-impl: `impl private::Sealed for SetMonoidal {}` + the existing `impl SetCategoryDefaults for SetMonoidal {}`.
- `tests/coalition_consumption_simulation.rs` turbofish updated: `tie_weights::<i64, _, i64, i64>(...)` → `tie_weights::<SetActegory, i64, _, i64, i64>(...)` (5-parameter explicit form). Same for `tests/comonoid_laws.rs` and `tests/para_composition.rs` (the `apply` turbofish in para_composition is 6-param post-widening). Pointwise-identical test behaviour.
- `docs/2402.15332v2-AUDIT.md` Theorem G.10 row + `SetCategoryDefaults` row updated to reflect v0.4.0 deltas.
- `src/para/comonoid.rs` module-level rustdoc example turbofish updated from stale 4-param `tie_weights::<P, _, X, Y>` to 5-param `tie_weights::<C, P, _, X, Y>` (B.4 code-quality ride-along M-3).
- `src/para/morphism.rs` module-level `clippy::type_complexity` rationale updated from `SetActegory` → `C` (B.3 code-quality ride-along Minor #1).
- `src/para/reparameterization.rs` adopts `use super::actegory::Actegory;` (B.2 code-quality ride-along Minor #1) and the `X, Y` rustdoc wording polish to "carrier types in the category `C` acts on" (B.2 Minor #2).

### Migration

- **Inference-relying callers** (`let tied = tie_weights(p, untied);`): no change. The widened signature still infers to `SetActegory` when the input `ParaMorphism`'s second type parameter is `SetActegory`.
- **Explicit-turbofish callers** (`tie_weights::<P, F, X, Y>(...)`): add `SetActegory` (or your actegory) at the leftmost position: `tie_weights::<SetActegory, P, F, X, Y>(...)`. Three in-tree test callers updated.
- **`SetCategoryDefaults` adopters** (none known external): add `impl Sealed for X {}` alongside the existing impl.

### Verification

- `cargo test -p catgraph-dl`: 42 tests + 3 doctests + 5 ignored, all green at v0.3.1 counts.
- `cargo clippy -p catgraph-dl --all-targets -- -W clippy::pedantic`: zero new warnings.
- `cargo doc -p catgraph-dl --no-deps`: zero warnings.
- Examples-coverage audit: cg-dl v0.4.0 ships one new public item (`pub trait Sealed`); gap rationale: "sealing trait with no consumer-facing example pattern."
- Examples-coverage baseline: full-surface cross-walk against `catgraph-dl/examples/*.rs` records 10+ items uncovered with 4 example-file recommendations folded to the v0.5.0+ forward-look.

## [0.3.1] - 2026-05-06

Patch release covering 8 Important findings + ride-along Minors from the
first cg-dl post-shipping multi-reviewer pass. Strictly additive on the
v0.3.0 surface — no API break; behaviour unchanged for all v0.3.0 callers;
the only bound widening is `SetCategoryDefaults: Sized` which is satisfied
by every shipping caller (`SetMonoidal` and the doctest's `MyMonoidal` are
unit structs).

### Important — Rust language correctness

- **Surface b hardening.** `para::SetCategoryDefaults`
  trait now carries a `: Sized` supertrait bound (was: empty bound list)
  and a new `## ⚠️ Conflict-guard caveat` rustdoc section explaining that
  implementing `SetCategoryDefaults` commits the type to the canonical
  `(Set, ×, 1)` `MonoidalCategory` body via the blanket impl, so a
  downstream user who *also* writes a hand-rolled `impl MonoidalCategory
  for MyType { ... }` will hit a `conflicting implementations` compile
  error. The convention is "don't combine the two": opt into
  `SetCategoryDefaults` for `(Set, ×, 1)` flavour, or write
  `impl MonoidalCategory` directly for non-`(Set, ×, 1)` flavour. The
  `: Sized` bound is satisfied by every shipping caller (every ZST is
  `Sized`); a downstream attempt at `impl SetCategoryDefaults for &'a [u8]`
  will fail at the bound site rather than silently picking up the blanket.

### Important — Documentation clarity

- `src/lib.rs` "Scope" header + "Out of scope" bullets refreshed to cite
  v0.2.0 + v0.3.0 surfaces, with a forward-looking "Deferred surfaces"
  bulleted list.
- `comonoid.rs` "Consumption pathway" rustdoc cross-linked from the
  in-tree integration test + the project-level Para-vs-Quantale layering
  note.
- `para::SetCategoryDefaults` doctest extended from 2 method assertions
  to all 5 (`tensor_objects`, `unit`, `associate`, `left_unitor`,
  `right_unitor`). Downstream users opening `cargo doc` now see a
  runnable demonstration that the blanket impl handles the coherence
  isomorphisms correctly, not just the trivial pair operations.

### Minor — ride-along

- `src/para/actegory.rs` now carries a "Why methods take `&self`"
  cross-link to the `monoidal_category` module's full rationale section.
- Audit doc `2402.15332v2-AUDIT.md` "## 3. Out of scope" header
  relabelled from `(v0.2.x)` to `(v0.3.x)`.
- Module-level `monoidal_category.rs` "Implementation note (option
  (γ-ii))" paragraph trimmed to a one-line pointer at the trait-level
  home; the discussion lives at exactly one canonical site.
- Both this v0.3.1 entry and the v0.3.0 entry now use the
  Keep-a-Changelog `[X.Y.Z] - YYYY-MM-DD` format with hyphen-space
  separators.

### Architectural — folded forward to v0.4.0

The post-shipping pass surfaced architectural findings folded into the
v0.4.0 forward-look:

- **`tie_weights` actegory generalisation.** v0.3.x `tie_weights` is
  hardcoded to `ParaMorphism<SetMonoidal, SetActegory, …>`; v0.4.0+
  relaxes to `<C: Actegory<SetMonoidal>>`.
- **HKT `&self` runtime-payload audit.** When a non-trivial actegory
  carrying runtime data lands, the future-proofing slot should be
  validated as load-bearing.
- **`tie_weights` `P: Clone` bound.** May become too strict for
  substitution-grammar cases where parameters are heap-shared `Arc<...>`.
- **External-user coherence footgun rustdoc.** Tighten coherence-error
  note on `SetCategoryDefaults` for downstream consumers combining the
  opt-in marker with a hand-rolled `MonoidalCategory` impl.

### Test counts

- 42 unit + integration tests (unchanged from v0.3.0).
- 3 doctests + 5 ignored (unchanged from v0.3.0; the `SetCategoryDefaults`
  doctest body grew but the doctest count is unchanged).
- 0 clippy pedantic warnings (`cargo clippy --lib --tests -- -W clippy::pedantic`).
- 0 doc warnings (`cargo doc --no-deps`).

## [0.3.0] - 2026-05-06 - Phase DL-3 flagged surfaces + Hopf evidence note

Phase DL-3: three flagged surfaces from the v0.2.0 Phase DL-2 Agent A-E
notes plus a Hopf-fibration evidence-note update. Strictly additive on the
v0.2.0 surface — no API break externally observable; the only internal
change is that `impl MonoidalCategory for SetMonoidal { ... }` (v0.2.0) was
hoisted into a blanket impl on the new `SetCategoryDefaults` opt-in marker
trait. Behaviour pointwise identical for `SetMonoidal`.

This is the first cg-dl release through the full multi-reviewer pass
(language correctness, project-mechanics, paper-audit).

### Added

- `para::SetCategoryDefaults` (Surface b) — opt-in marker trait carrying a
  blanket `impl<T: SetCategoryDefaults> MonoidalCategory for T` with the
  five canonical `(Set, ×, 1)`-flavoured method bodies. Downstream
  `(Set, ×, 1)`-flavoured ZSTs (e.g., a future `MyMonoidal: SetCategoryDefaults`)
  now get `MonoidalCategory` for free with an empty trait body, instead of
  reproducing `tensor_objects` / `unit` / `associate` / `left_unitor` /
  `right_unitor` pointwise. Re-exported at `catgraph_dl::para::SetCategoryDefaults`.
- `tests/coalition_consumption_simulation.rs` (Surface c) — integration
  test simulating the future `catgraph-coalition` v0.4.0 caller. Defines
  a local `MockQuantale: Actegory<SetMonoidal>` ZST playing the role of
  v0.4.0's actegory, and exercises `tie_weights::<i64, _, i64, i64>(3, untied)`
  end-to-end. Cross-validates that `MockQuantale::act` and `SetActegory::act`
  agree pointwise on the Cartesian-action shape, demonstrating the
  consumption pathway is structure-agnostic.

### Documentation

- **Paper anchor in-tree** — `docs/2402.15332v2.pdf` (Gavranović-Lessard-
  Dudzik et al., ICML 2024, 539 KB).
- **Merged primer** `docs/2402.15332v2-SUMMARY.md` (1295 lines) — Part I
  transcript-vs-paper comparison (flags Hopf-fibration / carry / pathfinding
  / synthetic-vs-analytic / multi-sorted-syntax as transcript-only) +
  Part II faithful paper rendering with ⚠️ CAREFUL cross-checking caveats
  on Appendix H.1 / H.3 worked-example arithmetic. Sourced from two
  pre-existing companion artefacts and merged for full in-tree
  self-containment.
- **Paper-coverage audit** `docs/2402.15332v2-AUDIT.md` — 76 audited items
  across §1-§4 + Appendices A/B/C/H/I/J/K. Headline at v0.2.0 baseline:
  **84% DONE on the implementable surface** (47 DONE / 6 PARTIAL /
  3 DEFERRED out of 56 implementable; 20 N/A motivational). Mirrors
  `BV25-AUDIT.md` + `GORARD23-AUDIT.md` structure; v0.3.0 deltas section
  populated by the post-shipping multi-reviewer pass.
- **`MonoidalCategory` `&self` rationale** (Surface a) — new module-level
  rustdoc paragraph in `src/para/monoidal_category.rs` headed "Why methods
  take `&self`" explaining the deliberate divergence from HAFT v0.3.1's
  static-dispatch convention. The `&self` slot is reserved for DL-3+
  instances over richer monoidal categories (R-module, hyperdoctrine,
  vector-bundle) that will carry runtime data — freezing the trait at
  static methods today would force a breaking change later.
- **`tie_weights` consumption pathway** (Surface c) — three new
  module-level rustdoc paragraphs in `src/para/comonoid.rs` covering
  (i) the layering invariant ("Para is upstream of Quantale";
  `catgraph-coalition` v0.4.0 imports `Actegory`, never re-defines it),
  (ii) what the v0.4.0 call site will look like end-to-end,
  (iii) what the actegory ▶ widening means for `tie_weights` in non-
  `(Set, ×, 1)` actegories.
- **Hopf-fibration evidence-note update** (Surface 2.4) — `src/hopf_fibration/mod.rs`
  + `src/lib.rs` ⚠️ CAREFUL section + `CLAUDE.md` ⚠️ CAREFUL section
  updated with the 2026-05-06 evidence note: the Filter Equivariants
  follow-up paper ([arXiv:2507.08796v1](https://arxiv.org/abs/2507.08796),
  Lewis-Ghani-Dudzik-Perivolaropoulos-Pascanu-Veličković, July 2025) §6
  explicitly puts ripple-carry addition **outside** the FE framework. As of
  2026-05-06 no Hopf-fibration / carry-operation preprint exists; the
  `hopf_fibration` private namespace stub is therefore kept reserved with
  no public API. The cited `[DvGPV24]` reference in FE is the existing
  Dudzik-von Glehn-Pascanu-Veličković *Asynchronous algorithmic alignment
  with cocycles* (LoG 2024) already cited in CDL §3.2, not a new Hopf
  paper.

### Changed (internal — no externally observable behaviour change)

- `para::SetMonoidal` now opts into the new `SetCategoryDefaults` blanket
  via an empty `impl SetCategoryDefaults for SetMonoidal {}` rather than
  carrying its own `impl MonoidalCategory for SetMonoidal { ... }`. The
  blanket supplies pointwise-identical method bodies (Cartesian product +
  tuple re-association). All v0.2.0 tests + doctests continue to pass
  without modification — this is purely a code-organisation hoist, not a
  behaviour change.

### Notes — Phase DL-3 design rationale

- **Surface (b) path taken: option (γ-ii) blanket impl on opt-in marker
  trait.** Original design-doc option (γ-i) (sub-trait with default-method
  bodies inherited by supertrait impls) does not type-check on stable
  Rust: a sub-trait cannot override a supertrait's method bodies. (γ-ii)
  is the closest stable-Rust equivalent and is functionally a renamed
  option (α) (marker trait + blanket impl) from the design doc. The
  `SetCategoryDefaults` name better signals the "(Set, ×, 1)-flavoured
  defaults" intent than a generic `Marker` would.
- **Surface (c) integration-test scope.** v0.2.0's `tie_weights` signature
  is hardcoded to `ParaMorphism<SetMonoidal, SetActegory, …>` — the
  actegory parameter is **not yet generic**. The simulation therefore
  defines `MockQuantale` to demonstrate the actegory-impl shape v0.4.0
  will write, and uses `SetActegory` for the actual `tie_weights` call.
  Cross-validates that both actegories agree pointwise on the Cartesian
  action shape, demonstrating the consumption pathway is structure-
  agnostic. **Surfacing an Architectural finding for v0.4.0 planning:**
  generalising `tie_weights` to take an `<C: Actegory<SetMonoidal>>`
  parameter is a v0.4.0-time design decision; coalition v0.4.0 will need
  either (a) a cg-dl-side `tie_weights` generalisation or (b) a coalition-
  side wrapper around `SetActegory`. Captured for the v0.3.0 multi-reviewer
  pass to confirm.
- **Hopf-fibration body remains DEFERRED** with **option (i) keep-stub**
  locked at design phase. Three options were considered: (i) keep, (ii)
  remove, (iii) promote to `forward_pointers/`. Rationale for (i): the
  stub is a useful research-direction-tracking signal in a Dudzik-flavoured
  workspace; FE §6 evidence confirms no preprint exists yet, so the
  namespace remains reserved.

### Test counts

- 42 unit + integration tests (v0.2.0: 41; +1 `tie_weights_consumption_pathway_simulation`).
- 3 doctests + 5 ignored (v0.2.0: 2 + 5; +1 `SetCategoryDefaults` doctest).
- 0 clippy pedantic warnings (`cargo clippy --lib --tests -- -W clippy::pedantic`).
- 0 doc warnings (`cargo doc --no-deps`).

## [0.2.0] — 2026-05-02 — Phase DL-2 bodies

Phase DL-2: trait surfaces from v0.1.0 widened with bodies. Bodies cover
the `Para(SetMonoidal, SetActegory)` 2-category, the `Comonoid` weight-
tying surface, F-(co)algebra homomorphisms with the GDL recovery test,
recursive `FreeMnd` / `CofreeCmnd` with `List` / `Tree` bijections, and
the five architecture unrollers with `FreeMnd`-equivalence verification.

**Public-surface widening (semver minor bump from v0.1.0):**
- `MonoidalCategory` trait: gained `Unit`, `Tensor<A, B>` GATs and 5
  coherence methods.
- `Actegory<M>` trait: gained `ActionResult<P, X>` GAT, `act`,
  `compose_action`.
- `Comonoid` trait: gained `comultiply`, `counit` methods (parameterised
  over `M: MonoidalCategory`).
- New trait `crate::endofunctor::EndoFunctor` with `Apply<X>` GAT and
  `fmap<X, Y, G: Fn(X) -> Y>`.
- New trait `algebra::Group` + `algebra::Z2Group` instance.
- New types: `para::SetMonoidal`, `para::SetActegory`, `para::SetObject`,
  `para::SetMorphism`, `para::DiagonalComonoid`, `algebra::FAlgebraHom`,
  `algebra::FCoalgebraHom`, `algebra::MonadAlgebraHom`,
  `algebra::GroupActionEndo<G>`, `free_monad::list_endo::ListEndo<A>`,
  `free_monad::tree_endo::TreeEndo<A>`,
  `free_monad::tree_endo::BinaryTree<A>`.
- New constructors / converters: `para::tie_weights`,
  `ParaMorphism::compose`, `ParaMorphism::apply`,
  `Reparameterization::apply`, `FoldingRnn::unroll`,
  `RecursiveNn::unroll`, `UnfoldingRnn::unroll_to_vec`,
  `MealyCell::run`, `MooreCell::run`,
  `vec_to_free_mnd` / `free_mnd_to_vec`,
  `tree_to_free_mnd` / `free_mnd_to_tree`.

### Added — Phase DL-2 Agent E (architecture unrollers + FreeMnd-equivalence)

- `architectures::FoldingRnn::unroll(cell, inputs: Vec<A>) -> S` —
  CDL Remark 2.13 / Example 2.12. Right-fold semantics: the unique
  algebra homomorphism `(P, List(A)) → S` from the initial algebra of
  the free monad on `1 + A × −` into the cell's algebra. Implemented
  via `inputs.into_iter().rev().fold(cell_0(p), step)` so the
  rightmost CDL element is the innermost call (Haskell `foldr`
  convention). Closure bounds: `P: Clone, Cell0: Fn(P) -> S, Cell1:
  Fn((P, A, S)) -> S`.
- `architectures::RecursiveNn::unroll(cell, tree: BinaryTree<A>) -> S`
  — CDL Example J.3. Post-order tree walk: `Leaf(_)` discharges
  through `cell_0(p)`; `Node(l, r)` recurses into both subtrees and
  combines via `cell_1((p, a, l_val, r_val))`. The four-arg `cell_1`
  shape (`(P, A, S, S)`) needs an `A` payload at internal-node
  combination — `BinaryTree::Node` carries no node payload, so the
  unroller takes the leftmost-leaf payload by convention. Tests use
  payload-agnostic cells so this choice is unobservable.
- `architectures::UnfoldingRnn::unroll_to_vec(cell, initial_state, depth) -> Vec<O>`
  — CDL Example J.2. Bounded-depth coalgebra unfolding: produce
  `depth` outputs by repeatedly applying `(cell_o, cell_n)` to advance
  the state. Infinite (lazy) unrolling is deferred to DL-3+ (would
  need `Lazy`/`Thunk` carrier or `tokio_stream::Stream`).
- `architectures::MealyCell::run<Step>(cell, initial_state, inputs: Vec<I>) -> Vec<O>`
  — CDL Example J.4. Mealy stream-process: thread state left-to-right
  through inputs, collect per-step outputs. The two-stage closure
  shape (`Cell: Fn((P, S)) -> Step`, `Step: FnOnce(I) -> (O, S)`) is
  the standard Rust workaround for "function returning closure" — the
  outer cell produces a fresh per-step closure each call.
- `architectures::MooreCell::run(cell, initial_state, inputs: Vec<I>) -> Vec<O>`
  — CDL Example I.5. Moore output-then-step: at each step, emit
  `cell_o(p, s)` *before* consuming the next input `i` and advancing
  via `cell_n((p, s, i))`. The first emitted output is from the
  initial state — distinguishes Moore from Mealy.
- `tests/architecture_unrollers.rs` — 10 acceptance tests:
  `FoldingRnn` sum-with-bias and length-counter, `RecursiveNn`
  three-tree post-order combine, `UnfoldingRnn` counter-unroll
  including `depth = 0`, `MealyCell` passthrough and stateful
  counter, `MooreCell` output-then-step (concrete `[0, 2, 4]` case),
  GDL recovery via `Z2`-invariant absolute-value folding (with a
  non-invariant discriminator), and **two `FreeMnd`-equivalence
  tests** — one each for the list and tree directions — proving
  `unroll(cell, x) == unroll_via_free_mnd(cell, to_free_mnd(x))` for
  several samples. The latter is the reification of the central CDL
  claim that the architecture unroller IS the unique algebra
  homomorphism from the initial algebra of the free monad.

### Notes — Phase DL-2 Agent E

- Closure bounds use the named-generic form (`Cell0: Fn(P) -> S, …`)
  rather than `impl Fn` to maximise downstream flexibility — consistent
  with the `EndoFunctor::fmap` named-generic decision (per
  `crate::endofunctor::EndoFunctor` module docs).
- The `RecursiveNn` unroller's leftmost-leaf convention for `cell_1`'s
  `A` argument is a Phase DL-2 implementation choice. CDL §J.3
  describes the recursive-NN architecture in terms of the algebraic
  shape `A + (−)²` where `A` is an *attribute* of internal nodes; our
  `BinaryTree::Node` is payload-free. Two semver-equivalent options
  for DL-3+: (a) widen `BinaryTree::Node` to carry an `A` payload, or
  (b) re-express the cell-1 closure as `Fn((P, S, S)) -> S` (3-arg)
  and drop the `A` from the internal-node side. The current 4-arg
  shape is what the DL-1 scaffold smoke test fixed; preserving it is
  source-compat.
- `UnfoldingRnn::unroll_to_vec` is *bounded*; truly-infinite final-
  coalgebra semantics is deferred. This is documented inline; no
  `unroll_into_iter` was added because the tests don't need lazy
  semantics yet.
- The two `FreeMnd`-equivalence tests are caller-sampled, not
  exhaustive (same caveat as Agent D's `verify_commutes`). Property-
  based testing is deferred to DL-3+ when generators for arbitrary
  `BinaryTree<u8>` and bounded `Vec<i64>` are in place.

### Added — Phase DL-2 Agent D (F-algebra/coalgebra homomorphisms + GDL recovery)

- `algebra::FAlgebraHom<F, A, B, FromS, ToS, MapS>` — F-algebra
  homomorphism (CDL Definition 2.5). Caller-driven `verify_commutes`
  evaluates the square `f ∘ a = b ∘ F(f)` on a sample `fa: F(A)`.
- `algebra::FCoalgebraHom<F, A, B, FromS, ToS, MapS>` — F-coalgebra
  homomorphism with the dual square `F(f) ∘ a = b ∘ f` (CDL Definition
  B.2 dual).
- `algebra::MonadAlgebraHom<M, A, B, FromS, ToS, MapS>` — monad-algebra
  homomorphism wrapping `FAlgebraHom`. Phase DL-2 machine-checks only
  the F-algebra square; the additional monad-unit and multiplication
  coherence is a **documented obligation** (CDL Definition 2.3) — see
  the doc comment on `MonadAlgebraHom::new`.
- `algebra::EndoFunctor` trait — `Apply<X>` GAT + `fmap`. **Local to
  `algebra/`**; Agent C is defining a sibling trait of the same shape
  in `free_monad/`. The
  `// TODO Phase DL-2 cleanup: deduplicate EndoFunctor with free_monad
  after both agents land` comment in `algebra/group_action.rs` documents
  the reconciliation contract for the parent agent.
- `algebra::Group` trait + `algebra::Z2Group` (cyclic group of order 2 —
  XOR-on-`bool`) + `algebra::GroupActionEndo<G>` (the endofunctor
  `F(X) = G × X`). CDL Example 2.4.
- `tests/algebra_homomorphisms.rs` — 5 acceptance tests covering
  identity-as-hom, non-equivariant projection failure, **CDL Example 2.6
  Geometric-Deep-Learning recovery** (absolute value as a `Z2`-invariant
  F-algebra homomorphism between the negation action and the trivial
  action), coalgebra-hom identity smoke (with non-commuting structure-
  map mismatch), and `MonadAlgebraHom` construction smoke.

### Notes — Phase DL-2 Agent D

- The GDL recovery test pairs the source algebra `(Vec<f64>, negation)`
  with the target `(Vec<f64>, trivial)`. An F-algebra homomorphism in
  this setting is exactly a `Z2`-**invariant** map. `|·|` satisfies it;
  `vec![x[0]]` does not — `verify_commutes` distinguishes the two.
- `verify_commutes` is **caller-sampled, not exhaustive** — the
  acceptance tests sweep a small representative grid (both group
  elements; positive, negative, and zero coordinates; the empty
  vector). Property-based testing is deferred to DL-3+ when enough
  algebraic infrastructure exists for a useful generator.
- `MonadAlgebraHom`'s monad-coherence laws (`M(f) ∘ η_A = η_B ∘ f`,
  associativity with `μ`) are **caller-attested**. A `Monad` trait
  carrying `η`/`μ` and the corresponding verifiers lands in DL-3+.

### Added — Phase DL-2 Agent C (recursive `FreeMnd`/`CofreeCmnd` + List/Tree bijections)

- `free_monad::EndoFunctor` — GAT-based functor trait. The object map is
  `type Apply<X>` (same GAT-as-HKT pattern Agent A used for
  `MonoidalCategory::Tensor`); the morphism map is
  `fn fmap<X, Y, G: Fn(X) -> Y>(fx: Self::Apply<X>, f: G) -> Self::Apply<Y>`.
  Functor laws (identity, composition) are documented as caller obligations
  rather than runtime-checked. **Local to `free_monad/`** — Agent D's
  `algebra/group_action.rs` defines a sibling trait of the same shape
  pending parent-agent reconciliation.
- `free_monad::FreeMnd<F, Z>` — recursive enum body. Two constructors:
  `Pure(Z)` (unit / terminator) and `Roll(Box<F::Apply<FreeMnd<F, Z>>>)`
  (functorial node). The `Box` indirection through the GAT projection
  compiled cleanly under Rust 1.95 + Edition 2024 — no workaround needed.
  Hand-rolled `Clone`/`Debug`/`PartialEq`/`Eq` impls with explicit
  `where F::Apply<Self>: Clone/Debug/...` bounds (the `derive` macros
  emit bounds the trait-resolution machinery can't always discharge
  through GAT projection). A `Default`-bounded `new()` retains
  source-compat with the DL-1 scaffold smoke test.
- `free_monad::CofreeCmnd<F, Z>` — recursive struct body. Fields
  `head: Z` and `tail: Box<F::Apply<CofreeCmnd<F, Z>>>`. Same manual
  trait-impl pattern as `FreeMnd`. Constructor `new(head, tail)` boxes
  the tail.
- `free_monad::list_endo::ListEndo<A>` — concrete `EndoFunctor` for
  `1 + A × −` with `Apply<X> = Option<(A, X)>`. CDL Example B.19.
- `free_monad::list_endo::vec_to_free_mnd` /
  `free_monad::list_endo::free_mnd_to_vec` — bijection witnesses for
  `FreeMnd<ListEndo<A>, Z> ≅ (Vec<A>, Z)`. Destruction walks
  iteratively (loop, not recursion) to keep stack usage bounded on long
  inputs.
- `free_monad::tree_endo::TreeEndo<A>` — concrete `EndoFunctor` for
  `A + (−)²` with `Apply<X> = Either<A, (X, X)>` (using the workspace
  `either` crate). CDL Example B.20.
- `free_monad::tree_endo::BinaryTree<A>` — explicit carrier for binary
  trees with leaves in `A`. `Leaf(A)` / `Node(Box, Box)` constructors.
- `free_monad::tree_endo::tree_to_free_mnd` /
  `free_monad::tree_endo::free_mnd_to_tree` — bijection witnesses for
  `BinaryTree<A> ≅ FreeMnd<TreeEndo<A>, Infallible>`. The `Infallible`
  terminator (stable proxy for the never type `!`) statically forbids
  `Pure`-shaped leaves; all leaves come through the `Left(a)` summand.
- `tests/free_monad_bijections.rs` — 5 acceptance tests:
  proptest-driven `Vec<u32>` round-trip (64 cases per direction),
  empty-list-to-`Pure(())` corner case, hand-built cons-cell tower for
  `[1, 2]`, three hand-built `BinaryTree` instances (leaf, single
  internal node, depth-3) round-tripping via `FreeMnd<TreeEndo,
  Infallible>`, and a `CofreeCmnd<TrivialEndo, u32>` smoke test
  confirming the GAT-bounded recursive struct constructs and clones.
- `Cargo.toml` — added `either.workspace = true` to dependencies (the
  workspace pin is `1.15`, already used by `catgraph` and
  `catgraph-applied`).

### Notes — Phase DL-2 Agent C

- Bijection helpers (`vec_to_free_mnd` etc.) are *not* re-exported at
  the crate root — they live at the qualified paths
  `catgraph_dl::free_monad::list_endo::*` and
  `catgraph_dl::free_monad::tree_endo::*`. The crate-root surface stays
  focused on the categorical primitives `FreeMnd`, `CofreeCmnd`, and
  `EndoFunctor`.
- `tests/scaffold_smoke.rs` updated for the new `EndoFunctor`-bounded
  type constructors. Endofunctor placeholders for `StreamEndo`,
  `MealyEndo`, `GroupActionEndo` get trivial `Apply<X> = ()` impls
  locally in the test file — semantics are exercised in
  `tests/free_monad_bijections.rs`.
- `EndoFunctor` is currently defined twice (here and in
  `algebra::group_action`, Agent D). Reconciliation is a follow-up
  parent-agent commit per the TODO note in
  `algebra/group_action.rs`.

### Added — Phase DL-2 Agent B (Comonoid coherence laws + diagonal weight-tying)

- `para::Comonoid<M>` — trait widened from a marker-only scaffold to a
  uniform-structure trait carrying value-level methods
  `comultiply : P → P ⊗ P` and `counit : P → I`. Generic over the carrier
  `P` (with the per-method bounds the implementor needs); the parameter
  category `M: MonoidalCategory` threads in Agent A's `Tensor<A, B>` and
  `Unit` GATs.
- `para::DiagonalComonoid<M>` — zero-sized witness for the canonical
  comonoid in `(Set, ×, 1)`. Implements `Comonoid<SetMonoidal>` with
  `δ(p) = (p.clone(), p)` and `ε(p) = ()`. Coassociativity, left counit,
  and right counit are exact equalities (not "up to iso"), per CDL §3.1.
- `para::tie_weights` — consumer-facing weight-tying API. Takes a
  `ParaMorphism<SetMonoidal, SetActegory, (P, P), F>` and returns a
  `ParaMorphism<…, P, impl Fn((P, X)) -> Y>` whose action is
  `λ(p, x). f(((p, p), x))`. Targeted by `catgraph-coalition` v0.4.0.
- `tests/comonoid_laws.rs` — 6 acceptance tests: coassociativity smoke
  (`bool`), proptest-driven coassociativity on `i32` and `String`,
  proptest left counit and right counit laws on `i32`, and the headline
  end-to-end weight-tying smoke (`(P × P, p1 + p2 + x)` tied at `p = 3`,
  `x = 5` ↦ `11`).

### Notes — Phase DL-2 Agent B

- Comonoid laws are exercised against `SetMonoidal::associate` /
  `left_unitor` / `right_unitor` (Agent A's coherence isomorphisms),
  closing the loop between the two agents' surfaces.
- `tie_weights` is implemented in terms of `Reparameterization::apply`
  rather than calling `Comonoid::comultiply` directly — the
  `Reparameterization` carrier wants a `Fn(PNew) -> POld` closure, not a
  method invocation borrowing `&self`. The trait body and the helper are
  semantically equivalent on `SetMonoidal`; a future DL-3+ enrichment
  can refactor in either direction without breaking the public surface.

### Added — Phase DL-2 Agent A (Para 2-category composition + Actegory action body)

- `para::SetMonoidal` — concrete monoidal category `(Set, ×, 1)`. Object-
  level tensor projects to Rust tuple `(A, B)`; unit projects to `()`.
  Coherence isomorphisms (associator, left/right unitor) implemented as
  exact tuple re-associations (CDL §3.1 default).
- `para::SetActegory` — concrete `M`-actegory of `SetMonoidal` acting on
  `Set` by Cartesian product. `▶ : (P, X) ↦ (P, X)`; coherence
  `μ : Q ▶ (P ▶ X) → (Q ⊗ P) ▶ X` is the canonical tuple re-association
  `(q, (p, x)) ↦ ((q, p), x)`.
- `para::SetObject`, `para::SetMorphism` — kind-of-objects/morphisms
  markers for `SetMonoidal`. Type-level witnesses; not instantiated at
  runtime.
- `MonoidalCategory` trait widened with `Unit` / `Tensor<A, B>` GATs and
  the `tensor_objects` / `unit` / `associate` / `left_unitor` /
  `right_unitor` methods. Required for the Phase DL-2 body but
  semver-acceptable since DL-1 (v0.1.0) was unreleased.
- `Actegory<M>` trait widened with `ActionResult<P, X>` GAT and the
  `act` / `compose_action` methods. Same semver caveat as
  `MonoidalCategory`.
- `ParaMorphism::compose` — sequential composition body for
  `Para(SetMonoidal, SetActegory)`. Returns
  `ParaMorphism<…, (Q, P), impl Fn(((Q, P), X)) -> Z>` whose action
  composes via `h((q, p), x) = g((q, f((p, x))))`.
- `ParaMorphism::apply` — convenience evaluator for
  `f((self.parameter.clone(), x))`.
- `Reparameterization::apply` — pre-composition for
  `Para(SetMonoidal, SetActegory)`. Given `r : P' → P`, produces a new
  `ParaMorphism<…, P', impl Fn((P', X)) -> Y>` whose action substitutes
  `r(p')` for `p` in the original `f`. Diagonal `Δ : P → (P, P)`
  specialises to weight tying.
- `tests/para_composition.rs` — 5 acceptance tests covering left unit,
  right unit, sequential composition (numeric), reparameterization
  triangle (diagonal weight tying), and direct sanity of
  `SetActegory::compose_action`'s μ.

### Notes — Phase DL-2 Agent A

- Closure convention frozen: `Fn((P, X)) -> Y` (tuple-as-single-argument)
  matches the `architectures::*` scaffold.
- Bodies restricted to `M = (Set, ×, 1)` per CDL default. Other monoidal
  categories deferred to DL-3+.
- HKT-like behaviour for the action result encoded via Generic
  Associated Types (`ActionResult<P, X>`) — same shape as the
  `deep_causality_haft` GAT-witness pattern.

## [0.1.0] — 2026-05-02 — Phase DL-1 scaffold

Initial scaffold release. Types-only surface; bodies land in Phase DL-2.

### Added

- `para::Para<M, C>` — type-level handle for the 2-category of parametric
  morphisms (CDL §3.1).
- `para::ParaMorphism<M, C, P, F>` — 1-morphism `(P, f : P ▶ X → Y)`.
- `para::Reparameterization<M, R>` — 2-morphism `(P, f) ⇒ (P', f')` via
  `r : P' → P`.
- `para::Comonoid` — trait surface for weight-tying-as-comonoid (CDL
  Theorem G.10).
- `para::MonoidalCategory` and `para::Actegory<M>` — trait surfaces for
  the parameter category and its action.
- `algebra::FAlgebra<F, A, S>` — F-algebra `(A, a : F(A) → A)` (CDL
  Definition 2.8).
- `algebra::FCoalgebra<F, A, S>` — F-coalgebra `(A, a : A → F(A))` (CDL
  Definition B.2).
- `algebra::MonadAlgebra<M, A, S>` — monad algebra `(A, a : M(A) → A)`
  with unit + associativity coherence (CDL Definition 2.3).
- `free_monad::FreeMnd<F, Z>` — type witness for
  `FreeMnd(F)(Z) = Fix(X ↦ F(X) + Z)` (CDL Proposition B.18).
- `free_monad::CofreeCmnd<F, Z>` — cofree-comonad dual.
- `architectures::FoldingRnn` — algebra of `Para(1 + A × −)`.
- `architectures::UnfoldingRnn` — coalgebra of `Para(O × −)`.
- `architectures::RecursiveNn` — algebra of `Para(A + (−)²)`.
- `architectures::MealyCell` — coalgebra of `Para(I → O × −)`.
- `architectures::MooreCell` — coalgebra of `Para(O × (I → −))`.
- Re-exports of the Tier 3 enrichment substrate from `catgraph-applied`:
  `Rig`, `UnitInterval`, `Tropical`, `F64Rig`, `BoolRig`,
  `EnrichedCategory`, `HomMap`, `LawvereMetricSpace`.
- `tests/scaffold_smoke.rs` — structural guard for the v0.1.0 public surface.

### Notes

- Private `hopf_fibration` module reserved for Andrew Dudzik's
  pre-publication carry-operation conjecture; not exposed.
- This release is intentionally body-less. Composition operators,
  coherence verification, and the algebra-homomorphism unroller arrive in
  Phase DL-2 with the `catgraph-coalition` v0.4.0 enriched-actegory body.

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.2.1...HEAD
[0.4.1]: https://github.com/tsondru/catgraph/releases/tag/catgraph-dl-v0.4.1
[0.4.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-dl-v0.4.0
[0.3.1]: https://github.com/tsondru/catgraph/releases/tag/catgraph-dl-v0.3.1
[0.3.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-dl-v0.3.0
[0.2.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-dl-v0.2.0
[0.1.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-dl-v0.1.0
