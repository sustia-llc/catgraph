# catgraph-syntax examples

Runnable consumer stories for the frozen S1–S5 textual surface (#5). Each is
`cargo run`-able and CI `check`-gated. They consume the public API only.

| Example | One-line pitch |
|---|---|
| [`programmatic_construction.rs`](programmatic_construction.rs) | Build a non-trivial SFG morphism twice — raw `Free::*` plumbing vs. one line of parsed text — and prove they are the *same* value (structural `Eq`) with identical `eval`. The term language replaces combinator plumbing. |
| [`assembly_composition.rs`](assembly_composition.rs) | The koalisi-shaped coalition-assembly shape: keep a library of named fragments authored as term strings, then wire them together incrementally with `Free::compose`/`Free::tensor`. The parser as a *construction API*, not a one-shot reader. |
| [`frobenius_wiring.rs`](frobenius_wiring.rs) | Physics-adjacent cospan wiring: express the compact-closed snake identity and spider fusion with S4 `spider`/`cup`/`cap`, checked semantically against `MatKron<i64>` via `to_mat_kron`. |
| [`workflow_dedup.rs`](workflow_dedup.rs) | **The colored one.** A role-typed workflow over `Λ = {Author, Reviewer, Editor}`: typed steps (`merge : R E -> E`), per-role spider wiring, a presentation file with `TOKEN : COLOR* -> COLOR*` declarations and `mu@R` tokens, `canonical_key` dedup across writings of the same process, and bounded convex-DPO optimization (`cost_of` / `optimize`, #214). |

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

## Standing caveats (leaned on by `frobenius_wiring.rs`)

- **#15 — soundness, not completeness.** `Presentation::eq_mod` returning
  `Ok(Some(true))` is a *proof* of equality; `None` / `Ok(Some(false))` is **not**
  a disproof (the engine is sound but syntactically incomplete by design).
  Complete decisions come only via `eq_mod_functorial` + `MatrixNFFunctor`
  (Thm 5.60). Any example asserting an equality through `eq_mod` relies only on
  the `Some(true)` direction.
- **`to_mat_kron` is a checker, not a decision.** It is a *sound semantic
  checker* (Prop 3.8), not a `CompleteFunctor`, and `User(g)` leaves are outside
  its domain (`NonFrobenius`). The three examples above it stay single-colour
  (`Λ = {•}`) for readability; `workflow_dedup.rs` is the colored one, and the
  layer itself has been Λ-colored end to end since #79, in the calculus and in
  the text alike.

## Colours, roles, and what does *not* connect (`workflow_dedup.rs`)

- **`RoleId(usize) ↔ Color` is a caller-side mapping, by design.** A downstream
  coalition indexes members by `RoleId`; `PropSignature::Color` is a different
  notion and catgraph ships no converter. There is **no diagram→magnitude
  seam** — no crate depends on `catgraph-syntax`, and `catgraph-magnitude`
  imports nothing from `prop::`. The process signal (dedup key, cost, an
  optimized representative) rides *beside* a coalition's value signal.
- **Dedup seams to carry upstream.** Compare *like with like* — a `content_of`
  content and a `content_of_colored` one differ on any wire no generator
  touches, so a dedup table uses one entry point throughout. `ContentKey` is
  deliberately **not `Ord`** (`Color` need not be), so buckets are unordered;
  never depend on iteration order. Serde **does not re-run** `check`, so a
  `ColoredExpr` rebuilt from a document is trusted rather than validated.
- **`eq_mod` is not transitive** ([#189]). Every verdict is sound for the pair
  asked about, but they do not compose: a caller that wants equivalence
  *classes* must take connected components of the `Some(true)` graph.
- **The optimizer is not a decider.** `rewrite::optimize` returns *best found
  under fuel* — no termination, confluence, or canonicality claim. `eq_mod` and
  `eq_colored` remain the deciders and are untouched by it.

[#189]: https://github.com/sustia-llc/catgraph/issues/189
