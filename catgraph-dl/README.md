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

Seven public modules plus one private namespace stub. Every item below is
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
- **`F64Module` R-module actegory** — the first **non-`(Set, ×, 1)`**
  `MonoidalCategory` / `Actegory` instance (issue #36). **`F64Monoidal`** is the
  monoidal category `(FinReal, ⊕, R⁰)` of finite-dimensional real modules under
  **direct sum**; **`F64Actegory`** is its self-action `▶ = ⊕`. The object-level
  tensor is the dedicated **`DirectSum<A, B>`** carrier (not the tuple), so
  `F64Monoidal` is a genuine non-`Set` instance with a hand-written
  `MonoidalCategory` impl — it does *not* opt into `SetCategoryDefaults`. The
  object carrier **`F64Module`** (`Vec<f64>`-backed `Rⁿ`) carries genuine
  `R`-module structure (`zeros` / `basis` / `add` / `scale` / `direct_sum`),
  which is where the reserved `deep_causality_num` `Zero` / `One` finally
  activate. Kind markers **`F64Object`** / **`F64Morphism`**. Monoidal product =
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

### `algebra` — F-(co)algebras and monad algebras (CDL §2)

- **`FAlgebra<F>`** `(A, a : F(A) → A)`, **`FCoalgebra<F>`** (dual), and
  **`MonadAlgebra<M>`** (CDL Definitions 2.3, 2.8, B.2). `MonadAlgebra` carries
  machine-checked monad-law verifiers **`verify_unit_law`** /
  **`verify_assoc_law`** (`η = ` haft's `Pure`, `μ = ` haft's `Monad::join`).
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
  `deep_causality_haft` 0.4.1 carriers, adopted per
  [#93](https://github.com/sustia-llc/catgraph/issues/93) (the box sits inside
  the functor hole: `Suspend(F::Type<Box>)`).
- **`ListEndo<A>`** with `vec_to_free_mnd` / `free_mnd_to_vec` — the list
  bijection witness (CDL Example B.19).
- **`TreeEndo<A>`** + the **`BinaryTree<A>`** carrier with `tree_to_free_mnd` /
  `free_mnd_to_tree` — the tree bijection witness (CDL Example B.20).

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

### `endofunctor` — the shared functor substrate

- **`HKT` / `Functor`** — `deep_causality_haft`'s GAT-based witness traits
  (object map `HKT::Type<X>`, morphism map `Functor::fmap`), re-exported through
  `crate::endofunctor` as the single import seam and shared by `algebra`
  (F-algebras and homomorphisms) and `free_monad` (the recursive `Free` /
  `Cofree` carriers — `deep_causality_haft` 0.4.1, adopted per
  [#93](https://github.com/sustia-llc/catgraph/issues/93)). Replaces the former
  hand-rolled `EndoFunctor` trait
  ([#12](https://github.com/sustia-llc/catgraph/issues/12)); every shipped
  witness uses `NoConstraint`.

### `natural` — natural transformations and pointed endofunctors (CDL Def 1.5 / B.3)

- **`NaturalTransformation<F, G>`** — the component family `α_X : F(X) → G(X)`
  of a natural transformation `α : F ⇒ G`, a static method on a zero-sized
  witness (matching the `Functor::fmap` dispatch style), with the naturality
  law `transform(F::fmap(fa, h)) == G::fmap(transform(fa), h)` as the
  implementor obligation.
- **`IsoForward`** / **`IsoBackward`** — adapter witnesses turning any haft
  `NaturalIso<F, G>` into its two natural transformations (`F ⇒ G` and `G ⇒ F`);
  separate types because the two directions would otherwise be overlapping
  blanket impls.
- **`Pointed`** — blanket marker for a pointed endofunctor `(F, σ)` with
  `σ = ` haft's `Pure` (the natural transformation `id ⇒ F`). `GroupActionEndo<G>`
  is the crate's own inhabitant (`σ(x) = (e, x)`, the writer-functor point);
  haft witnesses reachable through the seam (e.g. `OptionWitness`) are also
  pointed via their upstream `Pure` impls. `ListEndo` / `TreeEndo` ship no
  point — the former's only natural point (constant `None`) trivialises every
  pointed-algebra, the latter's diagonal point is natural but not representable
  under `Pure`'s no-`Clone` signature (see `src/natural.rs`).

### `container` — polynomial-functor shape/position presentation (Abbott–Altenkirch–Ghani 2003, via CDL)

- **`Container`** — equips an endofunctor witness with a `Shape` set, a per-shape
  `arity`, and a `decompose` / `recompose` pair witnessing
  `F(X) ≅ Σ_{s} X^{arity(s)}` in the finitary (`Vec`-of-contents) presentation.
  `ListEndo<A>` (`Shape = Option<A>`), `TreeEndo<A>` (`Shape = Either<A, ()>`),
  and `GroupActionEndo<G>` (`Shape = G`) are the shipped instances; the
  round-trip, arity-coherence, and `fmap`-coherence laws are machine-checked.

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

Phase 5 (`catgraph-dl`) is merged. The endofunctor layer now runs on
`deep_causality_haft`'s `HKT` / `Functor` witnesses (EndoFunctor→haft migration
landed, [#12](https://github.com/sustia-llc/catgraph/issues/12)) — imported in
`src/endofunctor.rs`, the single seam. `deep_causality_num` is now **in use**
([#36](https://github.com/sustia-llc/catgraph/issues/36)): `src/para/module_actegory.rs`
imports num's root `Zero` / `One` for the `F64Module` R-module actegory (the
ring identities filling the zero vector and marking the standard basis). Only
`Zero` / `One` are used — the DC substrate stays thin. Rig `Zero` / `One` are
still inherited transitively from `catgraph-applied`'s `Rig`.

## Dependencies

- `catgraph` — core Fong & Spivak types.
- `catgraph-applied` — the `Rig` + `EnrichedCategory` substrate (crate-graph
  position: `catgraph-applied` → `catgraph-dl`).
- `deep_causality_haft` — the `HKT` / `Functor` endofunctor witnesses (#12) and
  the `Either` sum used by `TreeEndo`. (The former dependency on the `either`
  crate was dropped — `Either` now comes from haft.)
- `deep_causality_num` — root `Zero` / `One` for the `F64Module` R-module
  actegory (#36; see [Status](#status)).
- dev: `proptest`.

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
- **Truly-infinite final-coalgebra semantics** for `UnfoldingRnn` — the current
  `unroll_to_vec` is *bounded*; a lazy / `Iterator` / `tokio_stream::Stream`
  carrier is deferred — [#36](https://github.com/sustia-llc/catgraph/issues/36).
- **`examples/` closure** — the crate ships no examples; the four planned
  example files closing the pre-reboot examples-coverage baseline are tracked
  in [#34](https://github.com/sustia-llc/catgraph/issues/34).
- ~~**Property-based exhaustive testing** of `verify_commutes` and
  `FreeMnd`-equivalence~~ — **shipped** ([#40](https://github.com/sustia-llc/catgraph/issues/40)).
  `tests/algebra_homomorphisms.rs` proptests the abs-value equivariance square
  (positive) and the projection failure (negative);
  `tests/architecture_unrollers.rs` proptests the list- and tree-direction
  `FreeMnd`-equivalence over generated inputs (the coalgebra-direction
  equivalence tests remain open —
  [#64](https://github.com/sustia-llc/catgraph/issues/64)). The individual
  `verify_commutes` entry points stay caller-sampled by design (the domain is
  not enumerable).
- **Upstream haft adoption of `Pointed` / `NaturalTransformation`** — the
  local `natural` traits stand in until the proposal to add them to
  `deep_causality_haft` itself lands —
  [#62](https://github.com/sustia-llc/catgraph/issues/62). On adoption they
  become seam re-exports.
- ~~**Machine-checked `MonadAlgebraHom` coherence laws** (`M(f) ∘ η_A = η_B ∘ f`,
  associativity with `μ`)~~ — **shipped**
  ([#40](https://github.com/sustia-llc/catgraph/issues/40)).
  `MonadAlgebra::verify_unit_law` / `verify_assoc_law` and
  `MonadAlgebraHom::verify_unit_coherence` / `verify_mult_coherence`, built on
  haft's `Monad` (`η = Pure`, `μ = join`) and law-tested in
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

## Build

```sh
cargo build  -p catgraph-dl
cargo test   -p catgraph-dl
cargo clippy -p catgraph-dl -- -W clippy::pedantic
```

## License

MIT — same as the rest of the catgraph workspace.
