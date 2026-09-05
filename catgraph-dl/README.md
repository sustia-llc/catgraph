# catgraph-dl

Categorical Deep Learning substrate for the [catgraph](https://github.com/sustia-llc/catgraph) workspace. Anchored to:

> Bruno Gavranović, Paul Lessard, Andrew Dudzik, Tamara von Glehn, João G.M. Araújo, Petar Veličković.
> *Categorical Deep Learning is an Algebraic Theory of All Architectures.*
> ICML 2024 — [arXiv:2402.15332v2](https://arxiv.org/abs/2402.15332)

The crate is a Rust expression of the central CDL constructions — the `Para`
2-category, F-(co)algebras and monad algebras, free/cofree recursion, and the
(co)algebra-as-architecture catalogue — available to other workspace members
and downstream crates. It is types plus (co)algebra wrappers over `(Set, ×, 1)`
by default, plus the first non-Set instance: the R-module actegory
`(FinReal, ⊕, R⁰)` (`F64Monoidal` / `F64Actegory`, issue #36); the remaining
monoidal categories are admitted by the trait surface but not yet instantiated
(see [Deferred surfaces](#deferred-surfaces)).

## Public surface

Nine public modules plus one private namespace stub. Every item below is
re-exported from its module root.

### `para` — the 2-category `Para(M, C)` (CDL §3.1)

Objects of an `M`-actegory `C`; 1-morphisms `(P ∈ M, f : P ▶ X → Y)`;
2-morphisms are reparameterizations `r : P' → P`. Sequential composition yields
`(Q ⊗ P, h)`.

- **`MonoidalCategory`** — GAT-based trait for the parameter category `M`
  (associated `Object`, `Morphism`, `Unit`, `Tensor<A, B>`). The trait rustdoc
  now carries the Mac Lane **pentagon** and **triangle** coherence equations as
  implementor obligations; for the `(Set, ×, 1)` blanket they are
  machine-checked (against `SetMonoidal` and a downstream-style ZST) in
  `tests/monoidal_coherence_laws.rs` (issue #40). The concrete `(Set, ×, 1)`
  instance is the zero-sized **`SetMonoidal`**, with kind markers
  **`SetObject`** / **`SetMorphism`** and the **`MonoidalTag<M>`** phantom
  witness.
- **`SetCategoryDefaults`** — opt-in marker trait (soft-sealed via **`Sealed`**)
  carrying a blanket `impl MonoidalCategory` with the five canonical
  `(Set, ×, 1)` method bodies; `SetMonoidal` itself uses this path. A
  downstream `(Set, ×, 1)`-flavoured ZST opts in with the dual-impl pattern
  (mirroring the compile-checked doctest on `SetCategoryDefaults`):

  ```rust
  use catgraph_dl::para::{MonoidalCategory, Sealed, SetCategoryDefaults};

  #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
  struct MyMonoidal;

  // Dual-impl soft-seal: Sealed (commitment to (Set, ×, 1)) first, then
  // SetCategoryDefaults — the blanket MonoidalCategory impl comes for free.
  impl Sealed for MyMonoidal {}
  impl SetCategoryDefaults for MyMonoidal {}
  ```

  The documented dual-impl pattern was chosen over a
  `#[derive(SetCategoryDefaults)]` proc-macro — two impl lines don't justify a
  separate macro crate
  ([#42](https://github.com/sustia-llc/catgraph/issues/42) decision).
- **`Actegory<M>`** + **`SetActegory`** — the action `▶ : M × C → C` and its
  coherence witness `μ : Q ⊗ (P ▶ X) → (Q ⊗ P) ▶ X`.
- **`RModule<S>` R-module actegory** — the first **non-`(Set, ×, 1)`**
  `MonoidalCategory` / `Actegory` instance (issue #36). The stack is generic in
  the scalar ring `S`, with `F64Module = RModule<f64>` (and `F64Monoidal` /
  `F64Actegory` / `F64Object` / `F64Morphism` likewise aliases of the `R*`
  types) as the default instantiation used throughout the crate.
  **`RMonoidal<S>`** is the monoidal category `(FinReal, ⊕, R⁰)` of
  finite-dimensional modules under
  **direct sum**; **`RActegory<S>`** is its self-action `▶ = ⊕`. The object-level
  tensor is the dedicated **`DirectSum<A, B>`** carrier (not the tuple), so
  `RMonoidal` is a genuine non-`Set` instance with a hand-written
  `MonoidalCategory` impl — it does *not* opt into `SetCategoryDefaults`. The
  object carrier **`RModule<S>`** (`Vec<S>`-backed `Sⁿ`) carries genuine
  `R`-module structure (`zeros` / `basis` / `add` / `scale` / `direct_sum`)
  under minimal per-method bounds,
  which is where catgraph-applied's `rig::Zero` / `rig::One` carry the ring
  identities. Kind markers **`RObject<S>`** / **`RMorphism<S>`**. Monoidal product =
  **direct sum `⊕`**, not tensor `⊗_R`: CDL Example G.3 forms `Para(Smooth)`
  over the *cartesian* structure of real vector spaces, whose finite-dimensional
  biproduct is `Rᵐ × Rⁿ ≅ Rᵐ⁺ⁿ`. Anchors: CDL Definition E.2 (actegory), Example
  E.4 (self-action), Example G.3.
- **`Comonoid<M>`** + **`DiagonalComonoid`** — the diagonal `Δ : P → (P, P)`.
- **`tie_weights`** — ties the parameter of a `Para(SetMonoidal, C)` 1-morphism
  through the diagonal comonoid, generic over any `C: Actegory<SetMonoidal>` so
  downstream callers with their own `Actegory<SetMonoidal>` ZSTs consume it
  directly.
- **`Para`** / **`ParaMorphism`** (with `compose`, `apply`) and
  **`Reparameterization`** (with `apply`).
- **`para::ad`** — a feature-gated *submodule* (not re-exported into `para`),
  present only under `--features ad`: `Dual<f64>` as a scalar for `RModule<S>`,
  the alias `DualF64Module`, and the `seed` / `gradient` forward-mode helpers.
  See [Features](#features).

### `algebra` — F-(co)algebras and monad algebras (CDL §2)

- **`FAlgebra<F>`** `(A, a : F(A) → A)`, **`FCoalgebra<F>`** (dual), and
  **`MonadAlgebra<M>`** (CDL Definitions 2.3, 2.8, B.2). `MonadAlgebra` carries
  machine-checked monad-law verifiers **`verify_unit_law`** /
  **`verify_assoc_law`** (`η = ` the substrate's `Pure`, `μ = Monad::join`).
- Homomorphism wrappers **`FAlgebraHom`** / **`FCoalgebraHom`** /
  **`MonadAlgebraHom`**, each with a caller-sampled `verify_commutes`;
  `MonadAlgebraHom` additionally carries the unit/multiplication coherence
  verifiers **`verify_unit_coherence`** (η-naturality, CDL Def 1.5 applied to
  `η`) / **`verify_mult_coherence`** (Def 2.3's associativity post-composed
  with `f`), machine-checked against samples in `tests/monad_algebra_laws.rs`.
  Note the two coherence verifiers probe the ambient monad/algebra structure —
  they hold for *any* `f` and cannot reject a non-homomorphism; the
  discriminating hom condition is `verify_commutes` (CDL Def 2.5). See the ⚠️
  scope note on `MonadAlgebraHom`.
- **`Group`**, **`Z2Group`**, **`GroupActionEndo<G>`** — group-action monad
  algebras recover GDL equivariant maps as monad-algebra homomorphisms (CDL §2.1
  Ex 2.6).

### `free_monad` — free and cofree recursion (CDL Proposition B.18)

- **`Free<F, Z>`** — realises the paper's `FreeMnd(F)(Z) = Fix(X ↦ F(X) + Z)`
  (CDL Def B.8), plus the cofree-comonad dual **`Cofree`**. Both are
  crate-owned carriers
  ([#222](https://github.com/sustia-llc/catgraph/issues/222)), keeping shape
  parity with the haft carriers adopted at
  [#93](https://github.com/sustia-llc/catgraph/issues/93) (the box sits inside
  the functor hole: `Suspend(F::Type<Box>)`). Since
  [#200](https://github.com/sustia-llc/catgraph/issues/200) `Free` holds that
  shape behind a private cell and hands it out as **`FreeView`** — build with
  `Free::pure` / `Free::suspend`, read with `into_view()` / `as_view()`.
- **`ListEndo<A>`** with `vec_to_free_mnd` / `free_mnd_to_vec` — the list
  bijection witness (CDL Example B.19).
- **`TreeEndo<A>`** + the **`BinaryTree<A>`** carrier (shape behind
  **`TreeView`**, same reshape) with `tree_to_free_mnd` / `free_mnd_to_tree` —
  the tree bijection witness (CDL Example B.20). `TreeView::Node` holds its two
  subtrees as **one boxed pair**, so this carrier costs one `Box` per internal
  *node* rather than per hole — half the allocations, and a private cell that
  costs nothing (`BinaryTree<u8>` = `TreeView<u8>` = 16 B on x86-64). `Free` and
  `Cofree` keep the per-hole placement, which their generic witness forces.
  Both bijection helpers are **infallible at any depth**: their walks are explicit heap worklists, as are `Free::fold`,
  `Cofree::unfold`, the carriers' `Drop`/`PartialEq`/`Debug`, and `BinaryTree`'s
  `Clone` (#200). Between
  [#231](https://github.com/sustia-llc/catgraph/issues/231) and #200 they were
  fallible, pre-flighting the `depth` guard and returning `DepthError` rather
  than risking a stack overflow.
- **Two consequences of the #200 reshape worth knowing before you hit them.**
  (a) A carrier's `Debug` lays each cell out through a scratch buffer, so it
  carries the caller's `alternate`, `precision` and `width` down to every
  payload but **drops** fill, alignment, the sign/zero-pad flags and
  `{:x?}` / `{:X?}` — stable Rust cannot build a `Formatter` from another's
  options. Under a dropped flag a carrier renders as if it were absent, where a
  `#[derive(Debug)]` type of the same shape would honour it. (b) The manual
  `Drop` tightens dropck: a carrier over a **borrowed** payload must be declared
  *after* what it borrows, or you get `error[E0597]`. Both are written up in the
  `free_monad` module docs, with runnable examples.

### `depth` / `errors` — an opt-in recursion guard for *callers* (#231, #200)

- **`tree_depth` / `free_mnd_depth`** — *iterative* depth measures (explicit
  worklist), so measuring an arbitrarily deep carrier never overflows.
- **`MAX_TREE_DEPTH = 256`** and **`guard_tree_depth` / `guard_free_mnd_depth`**
  — a ready-made ceiling and its checks, with
  **`DepthError::TreeDepthExceeded { depth, limit }`** the rejection. Equal to
  `catgraph-syntax`'s `MAX_TERM_DEPTH`
  ([#99](https://github.com/sustia-llc/catgraph/issues/99)), so the workspace has
  one recursion ceiling by convention.
- Engineering, not a CDL surface — and **opt-in since #200**: no entry in this
  crate calls the guard, because no walk in this crate recurses. It is published
  for a caller whose *own* code walks these carriers recursively (a hand-written
  `match` walk, a `fold` algebra that recurses, a serializer). The `guard_*`
  helpers borrow, so a rejected value stays yours.

### `architectures` — (co)algebra-as-architecture catalogue (CDL Appendix I / J)

Five typed wrappers. The two algebra-direction wrappers (`FoldingRnn`,
`RecursiveNn`) ship `FreeMnd`-equivalence tests — deterministic + proptest —
reifying CDL Remark 2.13; the three coalgebra-direction wrappers have
behavioural tests only, with final-coalgebra equivalence tracked in
[#64](https://github.com/sustia-llc/catgraph/issues/64):

| Type | Construction |
|------|--------------|
| `FoldingRnn` | `Para(1 + A × −)` algebra |
| `UnfoldingRnn` | `Para(O × −)` coalgebra |
| `RecursiveNn` | `Para(A + (−)²)` algebra |
| `MealyCell` (full RNN) | `Para(I → O × −)` coalgebra |
| `MooreCell` | `Para(O × (I → −))` coalgebra |

All five unrollers are infallible. `RecursiveNn::unroll` was the only
depth-recursive one and therefore the only fallible one between #231 and #200;
its walk is an explicit heap worklist now. The other four are folds and
`from_fn` state machines and never recursed.

### `endofunctor` — the shared functor substrate

- **`HKT` / `Functor`** — crate-owned GAT-based witness traits (object map
  `HKT::Type<X>`, morphism map `Functor::fmap`), defined in
  `crate::endofunctor` and shared by `algebra` (F-algebras and homomorphisms)
  and `free_monad` (the recursive `Free` / `Cofree` carriers). History: the
  hand-rolled `EndoFunctor` trait was replaced by haft's witnesses
  ([#12](https://github.com/sustia-llc/catgraph/issues/12)), which the crate
  then owned outright
  ([#222](https://github.com/sustia-llc/catgraph/issues/222)). The ambient
  category is `Set` by construction, so the owned traits carry no constraint
  machinery at all.

### `natural` — natural transformations and pointed endofunctors (CDL Def 1.5 / B.3)

- **`NaturalTransformation<F, G>`** — the component family `α_X : F(X) → G(X)`
  of a natural transformation `α : F ⇒ G`, a static method on a zero-sized
  witness (matching the `Functor::fmap` dispatch style), with the naturality
  law `transform(F::fmap(fa, h)) == G::fmap(transform(fa), h)` as the
  implementor obligation.
- **`IsoForward`** / **`IsoBackward`** — adapter witnesses turning any
  `NaturalIso<F, G>` into its two natural transformations (`F ⇒ G` and `G ⇒ F`);
  separate types because the two directions would otherwise be overlapping
  blanket impls.
- **`Pointed`** — blanket marker for a pointed endofunctor `(F, σ)` with
  `σ = ` the substrate's `Pure` (the natural transformation `id ⇒ F`).
  `GroupActionEndo<G>`
  is the crate's own inhabitant (`σ(x) = (e, x)`, the writer-functor point);
  seam witnesses (e.g. `OptionWitness`) are also
  pointed via their `Pure` impls. `ListEndo` / `TreeEndo` ship no
  point — the former's only natural point (constant `None`) trivialises every
  pointed-algebra, the latter's diagonal point is natural but not representable
  under `Pure`'s no-`Clone` signature (see `src/natural.rs`).

### `container` — polynomial-functor shape/position presentation (Abbott–Altenkirch–Ghani 2003, via CDL)

- **`Container`** — equips an endofunctor witness with a `Shape` set, a per-shape
  `arity`, a `decompose` / `recompose` pair witnessing
  `F(X) ≅ Σ_{s} X^{arity(s)}` in the finitary (`Vec`-of-contents) presentation,
  and `contents`, the borrowing half of `decompose`.
  `ListEndo<A>` (`Shape = Option<A>`), `TreeEndo<A>` (`Shape = Either<A, ()>`),
  `GroupActionEndo<G>` (`Shape = G`) and `OptionWitness` (`Shape = bool`) are the
  shipped instances; the round-trip, arity-coherence, `fmap`-coherence and
  borrow-coherence laws are machine-checked.
- It is also load-bearing for the carriers: `Free::fold`, `Cofree::unfold` and
  the carriers' `==` / `{:?}` bound on it, because pulling a generic witness's
  recursion slots out and putting results back is exactly what an
  explicit-worklist walk needs and what `fmap` alone cannot do
  ([#200](https://github.com/sustia-llc/catgraph/issues/200)).

### `hopf_fibration` (private)

Namespace stub for Dudzik's carry-operation conjecture. Pre-publication research,
not part of CDL ICML 2024, and not part of the public surface. See
[Provenance caveat](#provenance-caveat--hopf-fibration).

## Substrate re-exports

For a single import path, the Tier-3 enrichment substrate is re-exported from
`catgraph-applied`: `Rig`, `UnitInterval`, `Tropical`, `F64Rig`, `BoolRig`,
`EnrichedCategory`, `HomMap`, `LawvereMetricSpace`.

## Relationship to other workspace members

- **`catgraph-applied`** provides `Rig` and `EnrichedCategory<V>`.
  `catgraph-dl::para::Actegory<M, C>` is the 2-categorical refinement: `Rig`
  gives elements; `Actegory` gives morphisms and the coherence witness
  `μ : Q ⊗ (P ▶ X) → (Q ⊗ P) ▶ X`.
- **`catgraph-magnitude`** is orthogonal — magnitude is a scalar invariant
  (Möbius sum over a `Ring`-enriched category); `Para` is the 2-category of
  parametric morphisms. A `Para`-over-`Rig` actegory-enriched magnitude bridge
  is deferred.
- **`catgraph-physics`** — `evolution_cospan` is a *deterministic projection* of
  a `Para` F-algebra trajectory; `FreeMnd(F)` specialises to cospan chains when
  `F` is the cospan-step endofunctor. Cross-reference only; no code shared.

## Status

Phase 5 (`catgraph-dl`) is merged. The endofunctor layer runs on crate-owned
`HKT` / `Functor` witnesses in `crate::endofunctor` — the lineage is
hand-rolled `EndoFunctor` → haft witnesses
([#12](https://github.com/sustia-llc/catgraph/issues/12)) → crate-owned
([#222](https://github.com/sustia-llc/catgraph/issues/222)). The `RModule<S>` R-module actegory
([#36](https://github.com/sustia-llc/catgraph/issues/36)) takes its ring
identities — filling the zero vector and marking the standard basis — from
`catgraph_applied::rig::{Zero, One}`, the same pair the `Rig` bound is written
against ([#219](https://github.com/sustia-llc/catgraph/issues/219)); they used
to come from `deep_causality_num`.

## Dependencies

- `catgraph` — core Fong & Spivak types.
- `catgraph-applied` — the `Rig` + `EnrichedCategory` substrate (crate-graph
  position: `catgraph-applied` → `catgraph-dl`).
- `thiserror` — the workspace error-derive pattern, for `errors::DepthError`
  ([#231](https://github.com/sustia-llc/catgraph/issues/231)). Already in the
  tree via core's `CatgraphError`, applied's `HypergraphError`, and syntax's
  `SyntaxError`, so it adds no lockfile entry.
- `serde` — **optional**, off by default, behind the `serde` feature (#230).
- dev: `proptest`, `criterion`, `serde_json` (the round-trip driver for
  `tests/serde_roundtrip.rs`), and the workspace's unpublished
  `catgraph-testutil` (deterministic bench LCG).

That is the whole list — `thiserror` is the crate's only unconditional
non-catgraph `[dependencies]` entry, and the `ad` feature adds nothing to it.
`serde` is the one feature that adds a direct edge, which
is why it stays off by default (note serde itself already appears in the
default build transitively, under `catgraph-applied → rust_decimal` — the
feature adds this crate's *own* edge plus `serde_derive`, not serde's first
appearance). The
`Zero` / `One` behind
`RModule<S>` came from `deep_causality_num` until #219, the `Dual<T>` behind
`ad` came from `deep_causality_num_dual` until #221, and the
`HKT` / `Functor` / `Either` / `Free` / `Cofree` substrate came from
`deep_causality_haft` until #222; all are catgraph's own now. No
`deep_causality_*` crate remains anywhere in this crate's dependency graph.

## Deferred surfaces

Held until a downstream consumer surfaces a concrete need. Re-anchored to a
GitHub issue where one exists, otherwise plainly deferred.

- **Non-`(Set, ×, 1)` `MonoidalCategory` instances** — the R-module actegory
  (`F64Monoidal` / `F64Actegory` / `F64Module`) is **shipped** (the first bullet
  of [#36](https://github.com/sustia-llc/catgraph/issues/36)); the
  hyperdoctrine, vector-bundle, and fibration actegories remain deferred and
  keep #36 open. The `SetCategoryDefaults` opt-in marker trait closes the
  boilerplate gap for `(Set, ×, 1)`-flavoured ZSTs only; non-`Set` instances
  hand-write their `MonoidalCategory` impl as `F64Monoidal` does.
- ~~**Truly-infinite final-coalgebra semantics** for `UnfoldingRnn`~~ —
  **shipped** ([#36](https://github.com/sustia-llc/catgraph/issues/36)):
  `UnfoldingRnn::unroll_iter` is a genuinely infinite pull-based `Iterator`
  carrier (lazy `MealyCell::run_iter` / `MooreCell::run_iter` siblings);
  `unroll_to_vec` stays the bounded eager surface. A
  `tokio_stream::Stream` adapter remains unbuilt by design (no async deps).
- ~~**`examples/` closure**~~ — **shipped**
  ([#34](https://github.com/sustia-llc/catgraph/issues/34), closed):
  self-checking examples in `examples/`, run by CI.
- ~~**Property-based exhaustive testing** of `verify_commutes` and
  `FreeMnd`-equivalence~~ — **shipped** ([#40](https://github.com/sustia-llc/catgraph/issues/40)).
  `tests/algebra_homomorphisms.rs` proptests the abs-value equivariance square
  (positive) and the projection failure (negative);
  `tests/canonical.rs` proptests the list- and tree-direction
  `FreeMnd`-equivalence over generated inputs (the coalgebra-direction
  equivalence tests remain open —
  [#64](https://github.com/sustia-llc/catgraph/issues/64)). The individual
  `verify_commutes` entry points stay caller-sampled by design (the domain is
  not enumerable).
- ~~**Machine-checked `MonadAlgebraHom` coherence laws** (`M(f) ∘ η_A = η_B ∘ f`,
  associativity with `μ`)~~ — **shipped**
  ([#40](https://github.com/sustia-llc/catgraph/issues/40)).
  `MonadAlgebra::verify_unit_law` / `verify_assoc_law` and
  `MonadAlgebraHom::verify_unit_coherence` / `verify_mult_coherence`, built on
  the substrate's `Monad` (`η = Pure`, `μ = join`) and law-tested in
  `tests/monad_algebra_laws.rs`. Verifiers are caller-sampled; construction
  still does not enforce the laws; the two hom-side coherence checks probe the
  ambient monad/algebra structure and cannot reject a non-homomorphism (the
  discriminating condition is `verify_commutes` — see the ⚠️ scope note on
  `MonadAlgebraHom`).
- **The Hopf-fibration / carry-operation construction** — private stub only;
  deferred pending a Dudzik preprint (see below).
- **Symbiogenesis, Levin bioelectric, active inference** — deferred to a future
  external sibling crate, not this one.

## Provenance caveat — Hopf fibration

The private `hopf_fibration` module reserves namespace for a transcript-only
conjecture by Andrew Dudzik (DeepMind discussion of CDL): that modular
arithmetic with carry is a non-trivial `S¹`-fibration of `S³ → S²` rather than a
product `S¹ × S²`, motivating richer-than-diagonal `Para` 2-morphisms. **This is
not a result of the published CDL ICML 2024 paper.** Treat as pre-publication
research; do not cite it as co-authored by Gavranović et al. until a preprint
exists.

The most recent published Dudzik-co-authored work, *Filter Equivariant
Functions* ([arXiv:2507.08796v1](https://arxiv.org/abs/2507.08796),
Lewis–Ghani–Dudzik–Perivolaropoulos–Pascanu–Veličković, July 2025), §6 explicitly
puts ripple-carry addition **outside** the FE framework. As of 2026-05-06 no
Hopf-fibration / carry-operation preprint exists; the private stub stays
reserved with no public API. See `src/hopf_fibration/mod.rs` for the full
evidence trail.

## Documentation artefacts

- [arXiv:2402.15332v2](https://arxiv.org/abs/2402.15332) — the CDL paper
  itself (PDF not kept in-tree; fetch from arXiv).
- [`docs/2402.15332v2-SUMMARY.md`](docs/2402.15332v2-SUMMARY.md) — a merged
  primer (transcript-vs-paper comparison + a faithful paper rendering, with
  ⚠️ CAREFUL cross-checking caveats on the Appendix H.1 / H.3 worked-example
  arithmetic).
- [`docs/2402.15332v2-AUDIT.md`](docs/2402.15332v2-AUDIT.md) — paper-coverage
  audit of the implementable surface.
- [`docs/AUDIT-CHECKPOINT-v0.4.0.md`](docs/AUDIT-CHECKPOINT-v0.4.0.md) — a
  pre-reboot HKT `&self` audit checkpoint (kept for provenance; the filename
  retains its original pre-reboot version stamp and is not renamed).

## Features

| Feature | Default | What it adds |
|---|---|---|
| `ad` | off | Forward-mode automatic differentiation ([#74](https://github.com/sustia-llc/catgraph/issues/74)). Exposes `para::ad`: the crate's own `Dual<f64>` as a scalar for the generic `RModule<S>` stack, the alias `DualF64Module = RModule<Dual<f64>>`, and the `seed` / `gradient` helpers. Adds **no dependency** ([#221](https://github.com/sustia-llc/catgraph/issues/221)). |
| `serde` | off | `Serialize`/`Deserialize` derives on the parameter carriers ([#230](https://github.com/sustia-llc/catgraph/issues/230)): `RModule<S>`, `DirectSum<A, B>`, and — under `ad` too — `Dual<T>`. Adds `serde` (derive). Mirrors `catgraph-applied`'s opt-in `serde` ([#81](https://github.com/sustia-llc/catgraph/issues/81)), which persists *terms*; this persists the parameter data they act on, for the #72/#73 persistence track. |

`ad` is **additive, not a code path**: `Dual` satisfies every `RModule<S>` method
bound (`Zero`, `One`, `Add`, `Mul`, `Clone`), so the feature adds two modules
without changing any existing behaviour. Since #221 it pulls in no crate at all,
so `cargo tree -p catgraph-dl` and `cargo tree -p catgraph-dl --features ad`
now print the same tree.

`serde` is likewise additive — derives only, no behaviour change — but unlike
`ad` it *does* add a direct edge (`serde` + `serde_derive`), so it stays off by
default; see the Dependencies note above for the pre-existing transitive serde.
Deserialization reconstructs `RModule`'s coordinate vector directly: a
document's dimension is whatever it says, nothing checks it against the
dimension a caller expected, and of the module's operations only `add` rejects
a mismatch (with `None`) — `scale`, `direct_sum`, `flatten`, and the slice
accessors operate at whatever dimension was loaded. **Check `dim()` once at
your own entry point.** The full trust-boundary statement of record — including
the untagged wire shape and the non-finite-float `null` asymmetry — is
`RModule`'s Serde rustdoc section.

```sh
cargo test -p catgraph-dl --features ad
cargo run  -p catgraph-dl --features ad --example gradient_descent_para
cargo test -p catgraph-dl --features serde
# `Dual` lives in the `ad`-gated `para::ad`, so its derives need both:
cargo test -p catgraph-dl --features "serde ad"
```

## Canonical test

[`tests/canonical.rs`](tests/canonical.rs) is the crate's canonical integration
test: every shipped `HKT` witness satisfies the CDL Def 1.4 functor laws, and
each witness that presents as a container satisfies the container laws; the
CDL Example B.19 / B.20 bijections round-trip in both directions; each of
`FoldingRnn`, `RecursiveNn`, `UnfoldingRnn`, `MealyCell` and `MooreCell`
unrolls to what a `Free` / `Cofree` walker computes from the same cell; and
`unroll_iter` and the two `run_iter`s call their cells exactly once per pulled
item. Its header names the public types and traits it ranges over and the ones
it does not ([root README](../README.md#canonical-tests)).

## Build

```sh
cargo build  -p catgraph-dl
cargo test   -p catgraph-dl
cargo clippy -p catgraph-dl -- -W clippy::pedantic
```

## License

MIT — same as the rest of the catgraph workspace.
