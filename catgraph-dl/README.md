# catgraph-dl

Categorical Deep Learning substrate for the [catgraph](https://github.com/sustia-llc/catgraph) workspace. Anchored to:

> Bruno Gavranović, Paul Lessard, Andrew Dudzik, Tamara von Glehn, João G.M. Araújo, Petar Veličković.
> *Categorical Deep Learning is an Algebraic Theory of All Architectures.*
> ICML 2024 — [arXiv:2402.15332v2](https://arxiv.org/abs/2402.15332)

The crate is a Rust expression of the central CDL constructions — the `Para`
2-category, F-(co)algebras and monad algebras, free/cofree recursion, and the
(co)algebra-as-architecture catalogue — available to other workspace members
and downstream crates. It is types plus (co)algebra wrappers over `(Set, ×, 1)`
by default; other monoidal categories are admitted by the trait surface but not
yet instantiated (see [Deferred surfaces](#deferred-surfaces)).

## Public surface

Five public modules plus one private namespace stub. Every item below is
re-exported from its module root.

### `para` — the 2-category `Para(M, C)` (CDL §3.1)

Objects of an `M`-actegory `C`; 1-morphisms `(P ∈ M, f : P ▶ X → Y)`;
2-morphisms are reparameterizations `r : P' → P`. Sequential composition yields
`(Q ⊗ P, h)`.

- **`MonoidalCategory`** — GAT-based trait for the parameter category `M`
  (associated `Object`, `Morphism`, `Unit`, `Tensor<A, B>`). The concrete
  `(Set, ×, 1)` instance is the zero-sized **`SetMonoidal`**, with kind markers
  **`SetObject`** / **`SetMorphism`** and the **`MonoidalTag<M>`** phantom
  witness.
- **`SetCategoryDefaults`** — opt-in marker trait (soft-sealed via **`Sealed`**)
  carrying a blanket `impl MonoidalCategory` with the five canonical
  `(Set, ×, 1)` method bodies. A downstream `(Set, ×, 1)`-flavoured ZST opts in
  with empty `impl Sealed` + `impl SetCategoryDefaults` and gets
  `MonoidalCategory` for free; `SetMonoidal` itself uses this path.
- **`Actegory<M>`** + **`SetActegory`** — the action `▶ : M × C → C` and its
  coherence witness `μ : Q ⊗ (P ▶ X) → (Q ⊗ P) ▶ X`.
- **`Comonoid<M>`** + **`DiagonalComonoid`** — the diagonal `Δ : P → (P, P)`.
- **`tie_weights`** — ties the parameter of a `Para(SetMonoidal, C)` 1-morphism
  through the diagonal comonoid, generic over any `C: Actegory<SetMonoidal>` so
  downstream callers with their own `Actegory<SetMonoidal>` ZSTs consume it
  directly.
- **`Para`** / **`ParaMorphism`** (with `compose`, `apply`) and
  **`Reparameterization`** (with `apply`).

### `algebra` — F-(co)algebras and monad algebras (CDL §2)

- **`FAlgebra<F>`** `(A, a : F(A) → A)`, **`FCoalgebra<F>`** (dual), and
  **`MonadAlgebra<M>`** (CDL Definitions 2.3, 2.8, B.2).
- Homomorphism wrappers **`FAlgebraHom`** / **`FCoalgebraHom`** /
  **`MonadAlgebraHom`**, each with a caller-attested `verify_commutes`.
- **`Group`**, **`Z2Group`**, **`GroupActionEndo<G>`** — group-action monad
  algebras recover GDL equivariant maps as monad-algebra homomorphisms (CDL §2.1
  Ex 2.6).

### `free_monad` — free and cofree recursion (CDL Proposition B.18)

- **`FreeMnd<F, Z>`** — explicit `FreeMnd(F)(Z) = Fix(X ↦ F(X) + Z)`, plus the
  cofree-comonad dual **`CofreeCmnd`**.
- **`ListEndo<A>`** with `vec_to_free_mnd` / `free_mnd_to_vec` — the list
  bijection witness (CDL Example B.19).
- **`TreeEndo<A>`** + the **`BinaryTree<A>`** carrier with `tree_to_free_mnd` /
  `free_mnd_to_tree` — the tree bijection witness (CDL Example B.20).

### `architectures` — (co)algebra-as-architecture catalogue (CDL Appendix I / J / K)

Five typed wrappers, each shipping a `FreeMnd`-equivalence test (CDL Remark 2.13):

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
  (F-algebras and homomorphisms) and `free_monad` (recursive `FreeMnd` /
  `CofreeCmnd`). Replaces the former hand-rolled `EndoFunctor` trait
  ([#12](https://github.com/sustia-llc/catgraph/issues/12)); every shipped
  witness uses `NoConstraint`.

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
`src/endofunctor.rs`, the single seam. `deep_causality_num` stays carried
**deps-only**, reserved for the R-module / `F64Module` surfaces
([#36](https://github.com/sustia-llc/catgraph/issues/36)) which need `Zero` /
`One`; it is not referenced anywhere in the crate (`rg deep_causality_num src`
returns no hits — post-#12 the doc mentions went too). Rig `Zero` / `One` are
inherited transitively from `catgraph-applied`'s `Rig`.

## Dependencies

- `catgraph` — core Fong & Spivak types.
- `catgraph-applied` — the `Rig` + `EnrichedCategory` substrate (crate-graph
  position: `catgraph-applied` → `catgraph-dl`).
- `deep_causality_haft` — the `HKT` / `Functor` endofunctor witnesses (#12) and
  the `Either` sum used by `TreeEndo`. (The former dependency on the `either`
  crate was dropped — `Either` now comes from haft.)
- `deep_causality_num` — **deps-only**, reserved for #36 (see [Status](#status)).
- dev: `proptest`.

## Deferred surfaces

Held until a downstream consumer surfaces a concrete need. Re-anchored to a
GitHub issue where one exists, otherwise plainly deferred.

- **Non-`(Set, ×, 1)` `MonoidalCategory` instances** — R-module actegory,
  hyperdoctrine, vector-bundle. The trait surface admits them; concrete
  instances are deferred —
  [#36](https://github.com/sustia-llc/catgraph/issues/36). The
  `SetCategoryDefaults` sub-trait closes the boilerplate gap for
  `(Set, ×, 1)`-flavoured ZSTs only.
- **Truly-infinite final-coalgebra semantics** for `UnfoldingRnn` — the current
  `unroll_to_vec` is *bounded*; a lazy / `Iterator` / `tokio_stream::Stream`
  carrier is deferred — [#36](https://github.com/sustia-llc/catgraph/issues/36).
- **`examples/` closure** — the crate ships no examples; the four planned
  example files closing the pre-reboot examples-coverage baseline are tracked
  in [#34](https://github.com/sustia-llc/catgraph/issues/34).
- **Property-based exhaustive testing** of `verify_commutes` and
  `FreeMnd`-equivalence — current tests are caller-sampled.
- **Machine-checked `MonadAlgebraHom` coherence laws** (`M(f) ∘ η_A = η_B ∘ f`,
  associativity with `μ`) — currently caller-attested.
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

- [`docs/2402.15332v2.pdf`](docs/2402.15332v2.pdf) — the CDL paper itself.
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
