# catgraph-dl

Categorical Deep Learning substrate for the [catgraph](https://github.com/tsondru/catgraph) workspace. Anchored to:

> Bruno Gavranović, Paul Lessard, Andrew Dudzik, Tamara von Glehn, João G.M. Araújo, Petar Veličković.
> *Categorical Deep Learning is an Algebraic Theory of All Architectures.*
> ICML 2024 — [arXiv:2402.15332v2](https://arxiv.org/abs/2402.15332)

## What this crate provides

A Rust expression of the central CDL constructions, available to other workspace members and downstream crates:

- **`para`** — the 2-category `Para(M, C)` of parametric morphisms (CDL §3.1). Objects of an `M`-actegory `C`; 1-morphisms `(P, f : P ▶ X → Y)`; 2-morphisms = reparameterizations `r : P' → P`. Sequential composition: `(Q ⊗ P, h)`.
- **`algebra`** — F-algebras `(A, a : F(A) → A)`, F-coalgebras (dual), monad algebras (CDL Definitions 2.3, 2.8, B.2). Group-action monad algebras recover GDL equivariant maps.
- **`free_monad`** — explicit `FreeMnd(F)(Z) = Fix(X ↦ F(X) + Z)` and the cofree-comonad dual (CDL Proposition B.18).
- **`architectures`** — five typed wrappers for the (co)algebra-as-architecture catalogue (CDL Appendix I):
  - Folding RNN — `Para(1 + A × −)` algebra
  - Unfolding RNN — `Para(O × −)` coalgebra
  - Recursive NN — `Para(A + (−)²)` algebra
  - Full RNN (Mealy) — `Para(I → O × −)` coalgebra
  - Moore Machine NN — `Para(O × (I → −))` coalgebra
- **`hopf_fibration`** — *private* namespace stub for Dudzik's carry-operation conjecture. ⚠️ Pre-publication research; not in CDL ICML 2024.

## Status — v0.4.1

Patch release applying review findings + an examples-coverage baseline on top of v0.4.0. Strictly additive; no API break.

### v0.4.0 — `tie_weights` actegory-genericity

`tie_weights` widened to `C: Actegory<SetMonoidal>` so downstream callers with their own `Actegory<SetMonoidal>` ZSTs can consume the API directly. This is a non-source-breaking generalisation for inference-relying callers; explicit-turbofish callers move from 4-parameter to 5-parameter on `tie_weights`. Companion changes:

- `SetCategoryDefaults` soft-sealed via `private::Sealed` (prevents accidental out-of-crate impls).
- HKT `&self` audit-checkpoint declared at `docs/AUDIT-CHECKPOINT-v0.4.0.md`.

### v0.3.x — three flagged surfaces

v0.3.0 + v0.3.1 closed three flagged surfaces from the v0.2.0 review pass:

- **(a) `MonoidalCategory` `&self` rationale** — module-level rustdoc explaining the deliberate divergence from static-dispatch convention; the `&self` slot is reserved for instances over richer monoidal categories (R-module, hyperdoctrine, vector-bundle) that will carry runtime data.
- **(b) `SetCategoryDefaults` opt-in marker trait** — blanket `impl<T: SetCategoryDefaults> MonoidalCategory for T` carrying the five canonical `(Set, ×, 1)`-flavoured method bodies. Downstream `(Set, ×, 1)`-flavoured ZSTs opt in via empty `impl SetCategoryDefaults for MyType {}` and get `MonoidalCategory` for free without reproducing the bodies.
- **(c) `tie_weights` consumption pathway** — module-level rustdoc in `comonoid.rs` documenting the downstream caller's pathway + `tests/coalition_consumption_simulation.rs` integration test simulating the consumer via a local `MockQuantale: Actegory<SetMonoidal>` ZST.

**Hopf-fibration evidence-note update**: the published Filter Equivariants follow-up paper ([arXiv:2507.08796v1](https://arxiv.org/abs/2507.08796), Lewis-Ghani-Dudzik-Perivolaropoulos-Pascanu-Veličković, July 2025) §6 explicitly puts ripple-carry addition **outside** the FE framework — as of 2026-05-06 no Hopf-fibration / carry-operation preprint exists. The private `hopf_fibration` namespace stub stays reserved with no public API.

Test counts: 42 unit/integration tests + 3 doctests + 5 ignored. Clippy pedantic clean. `cargo doc --no-deps` clean. **84% paper-coverage** on the implementable surface per `docs/2402.15332v2-AUDIT.md`.

### Documentation artefacts

- [`docs/2402.15332v2.pdf`](docs/2402.15332v2.pdf) — the CDL paper itself (539 KB).
- [`docs/2402.15332v2-SUMMARY.md`](docs/2402.15332v2-SUMMARY.md) — merged primer (Part I transcript-vs-paper comparison + Part II faithful paper rendering with ⚠️ CAREFUL cross-checking caveats on Appendix H.1 / H.3 worked-example arithmetic).
- [`docs/2402.15332v2-AUDIT.md`](docs/2402.15332v2-AUDIT.md) — paper-coverage audit, 76 audited items, **84% DONE on the implementable surface** (47 DONE / 6 PARTIAL / 3 DEFERRED out of 56 implementable; 20 N/A motivational).
- [`docs/AUDIT-CHECKPOINT-v0.4.0.md`](docs/AUDIT-CHECKPOINT-v0.4.0.md) — HKT `&self` audit checkpoint.

### v0.4.0 public surface (added since v0.3.x)

- `tie_weights<P, A, X, M, C: Actegory<SetMonoidal>>(...)` — actegory-genericity for downstream consumers with their own `Actegory<SetMonoidal>` ZSTs.
- `SetCategoryDefaults: Sealed` — soft-seal via `private::Sealed`.

### v0.3.0 public surface (added since v0.2.0)

- `para::SetCategoryDefaults` — opt-in marker trait + blanket impl for `(Set, ×, 1)`-flavoured ZSTs.
- `tests/coalition_consumption_simulation.rs` — downstream consumption-pathway simulation test.

### v0.2.0 public surface (added since v0.1.0)

- `para::SetMonoidal`, `para::SetActegory`, `para::SetObject`, `para::SetMorphism`
- `para::DiagonalComonoid`, `para::tie_weights`
- `algebra::FAlgebraHom`, `algebra::FCoalgebraHom`, `algebra::MonadAlgebraHom` (each with caller-driven `verify_commutes`)
- `algebra::Group`, `algebra::Z2Group`, `algebra::GroupActionEndo<G>`
- `free_monad::list_endo::ListEndo<A>` + `vec_to_free_mnd` / `free_mnd_to_vec`
- `free_monad::tree_endo::TreeEndo<A>` + `BinaryTree<A>` + `tree_to_free_mnd` / `free_mnd_to_tree`
- Canonical `crate::endofunctor::EndoFunctor` trait at crate root
- `MonoidalCategory` / `Actegory<M>` / `Comonoid<M>` traits widened with their full method bodies and GAT shapes (`Tensor<A, B>`, `Unit`, `ActionResult<P, X>`)
- `ParaMorphism::compose`, `ParaMorphism::apply`, `Reparameterization::apply`

### v0.1.0 — scaffold

Public types declared and constructible; bodies were body-less. The scaffold smoke test (`tests/scaffold_smoke.rs`) confirmed the public API surface compiled and all five architecture wrappers + F-(co)algebra newtypes + free-monad type witnesses instantiated.

## Relationship to other workspace members

- **`catgraph-applied`** provides `Rig` and `EnrichedCategory<V>`. `catgraph-dl::para::Actegory<M, C>` is the 2-categorical refinement: `Rig` gives elements; `Actegory` gives morphisms and the coherence witness `μ : Q ⊗ (P ▶ X) → (Q ⊗ P) ▶ X`.
- **`catgraph-magnitude`** is orthogonal — magnitude is a scalar invariant (Möbius sum over a `Ring`-enriched category); `Para` is the 2-category of parametric morphisms. A future bridge to Para-over-Rig actegory-enriched magnitude is deferred.
- **`catgraph-physics`** — `evolution_cospan` is a *deterministic projection* of a Para F-algebra trajectory; `FreeMnd(F)` specialises to cospan chains when `F` is the cospan-step endofunctor. Cross-reference only.

## Out of scope for v0.4.x

- Other monoidal categories beyond `(Set, ×, 1)` — a future minor. The current `Para` body specialises to `M = SetMonoidal`; an `R`-module instance, a hyperdoctrine instance, etc. land later.
- Truly-infinite final-coalgebra semantics (`UnfoldingRnn::unroll_to_vec` is *bounded* — needs a `Lazy` / `Thunk` carrier or `tokio_stream::Stream` to lift to streams).
- Property-based exhaustive testing of `verify_commutes` and `FreeMnd`-equivalence (current tests are caller-sampled).
- `MonadAlgebraHom` machine-checked monad-coherence laws (`M(f) ∘ η_A = η_B ∘ f`, associativity with `μ`) — currently caller-attested.
- The Hopf-fibration / carry-operation construction (private stub only; pre-publication research). Filter Equivariants follow-up paper §6 confirms no preprint exists as of 2026-05-06.
- Symbiogenesis, Levin bioelectric, active inference (deferred to a future external sibling crate — ambitious tier of the proposal).

## v0.5.0 forward-look

Headline candidate items for v0.5.0:

1. **Example files closure** — four new example files (`para_walkthrough`, `weight_tying`, `free_monad_basics`, `architecture_unrollers`) closing the v0.4.0 examples-coverage baseline (24 UNCOVERED items).
2. **Lazy stream-coalgebra `UnfoldingRnn` carrier** — `Lazy` / `Thunk` lifting per CDL §B.20.
3. **Non-`(Set, ×, 1)` `MonoidalCategory` instance** — R-module is the natural first target per CDL §3.1.

## Provenance caveat — Hopf fibration

The `hopf_fibration` module reserves namespace for a transcript-only conjecture by Andrew Dudzik (DeepMind discussion of CDL): that modular arithmetic with carry is a non-trivial `S¹`-fibration of `S³ → S²` rather than a product `S¹ × S²`, motivating richer-than-diagonal `Para` 2-morphisms. **This is not a result of the published CDL ICML 2024 paper.** Treat as pre-publication research.

**2026-05-06 evidence update**: the most recent published Dudzik-co-authored work, *Filter Equivariant Functions* ([arXiv:2507.08796v1](https://arxiv.org/abs/2507.08796), July 2025), §6 explicitly puts ripple-carry addition **outside** the FE framework. As of 2026-05-06 no Hopf-fibration / carry-operation preprint exists; the private `hopf_fibration` namespace stub stays reserved with no public API.

## Build

```sh
cargo build -p catgraph-dl
cargo test -p catgraph-dl
cargo clippy -p catgraph-dl -- -W clippy::pedantic
```

## License

MIT — same as the rest of the catgraph workspace.
