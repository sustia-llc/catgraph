# catgraph-dl examples

Runnable, self-checking walkthroughs of the crate's public surface, anchored to
Gavranović et al., *Categorical Deep Learning*, ICML 2024
([arXiv:2402.15332](https://arxiv.org/abs/2402.15332)). Each `main` ends by
asserting its results, so `cargo run --example <name>` doubles as a smoke check.

| Example | Demonstrates | CDL anchor |
|---|---|---|
| [`para_walkthrough`](para_walkthrough.rs) | `Para` 1-morphisms: build, sequential `compose`, `Reparameterization` | §3.1 |
| [`weight_tying`](weight_tying.rs) | the diagonal `Comonoid` and `tie_weights` (shared parameters) | Thm G.10 / §3.1 |
| [`free_monad_basics`](free_monad_basics.rs) | `Free`/`Cofree` construction, list/tree bijections, an `FAlgebra` catamorphism | App B |
| [`architecture_unrollers`](architecture_unrollers.rs) | `FoldingRnn`/`RecursiveNn`/`UnfoldingRnn`/`MealyCell`/`MooreCell` as (co)algebra unrollers; GDL invariance | App I & J, Ex 2.6 |

```sh
cargo run -p catgraph-dl --example para_walkthrough
cargo run -p catgraph-dl --example weight_tying
cargo run -p catgraph-dl --example free_monad_basics
cargo run -p catgraph-dl --example architecture_unrollers
```

## Scope

These examples exercise the **public API** only; they do not touch crate
internals. The law-level guarantees (functor/comonoid/monad-algebra coherence,
`FreeMnd`/`CofreeCmnd` equivalence, GDL equivariance) are the job of the
`tests/` suite — the examples show *usage*, the tests prove *correctness*.

Non-goal: the enriched-magnitude coalition layer (`coalition_*`) is a separate
consumer concern, not illustrated here.
