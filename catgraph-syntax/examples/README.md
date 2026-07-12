# catgraph-syntax examples

Runnable consumer stories for the frozen S1–S5 textual surface (#5). Each is
`cargo run`-able and CI `check`-gated. They consume the public API only.

| Example | One-line pitch |
|---|---|
| [`programmatic_construction.rs`](programmatic_construction.rs) | Build a non-trivial SFG morphism twice — raw `Free::*` plumbing vs. one line of parsed text — and prove they are the *same* value (structural `Eq`) with identical `eval`. The term language replaces combinator plumbing. |
| [`assembly_composition.rs`](assembly_composition.rs) | The koalisi-shaped coalition-assembly shape: keep a library of named fragments authored as term strings, then wire them together incrementally with `Free::compose`/`Free::tensor`. The parser as a *construction API*, not a one-shot reader. |
| [`frobenius_wiring.rs`](frobenius_wiring.rs) | Physics-adjacent cospan wiring: express the compact-closed snake identity and spider fusion with S4 `spider`/`cup`/`cap`, checked semantically against `MatKron<i64>` via `to_mat_kron`. |

Run one with, e.g.:

```sh
cargo run -p catgraph-syntax --example programmatic_construction
```

The persistence round-trip lives as an integration test, not an example:
[`../tests/persistence.rs`](../tests/persistence.rs) — persist a presentation +
terms to text, reload, and assert decision-procedure equivalence (the pre-serde
answer to #73 / #81).

## Why there is no magnitude example

`catgraph-magnitude` computes **invariants of** structures (Euler characteristic,
magnitude, diversity); it does not *author* term presentations. It benefits from
this surface only *indirectly* — an easier way to construct the enriched
structures whose invariants it then measures — so there is deliberately no
magnitude example here. Authoring is the syntax layer's job; measuring is
magnitude's, and the two do not meet at the term level.

## Two standing disclaimers (leaned on by `frobenius_wiring.rs`)

- **#15 — soundness, not completeness.** `Presentation::eq_mod` returning
  `Ok(Some(true))` is a *proof* of equality; `None` / `Ok(Some(false))` is **not**
  a disproof (the engine is sound but syntactically incomplete by design).
  Complete decisions come only via `eq_mod_functorial` + `MatrixNFFunctor`
  (Thm 5.60). Any example asserting an equality through `eq_mod` relies only on
  the `Some(true)` direction.
- **Monochromatic S4 scope.** The Frobenius layer is single-colour (`Λ = {•}`,
  one spider family). `to_mat_kron` is a *sound semantic checker* (Prop 3.8), not
  a `CompleteFunctor`, and `User(g)` leaves are outside its domain
  (`NonFrobenius`). Fully colored props are tracked as #79.
