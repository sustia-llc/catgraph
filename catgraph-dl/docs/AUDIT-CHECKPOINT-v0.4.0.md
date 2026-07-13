# Audit checkpoint — the `MonoidalCategory` / `Actegory` `&self` rationale

**Status (2026-07-13): OPEN — forward-only, no in-tree validator yet.** The
`&self` receiver on the [`MonoidalCategory`](../src/para/monoidal_category.rs) and
[`Actegory<M>`](../src/para/actegory.rs) methods is a deliberate design decision
whose validating consumer is not yet in-tree. This doc records the criteria that
would ratify it and the current standing.

> Provenance: predicates first declared 2026-05-10 in the pre-reboot crate (then
> versioned `cg-dl v0.4.0`). The reboot dropped per-crate version lineage in
> favour of the GitHub-issue model (#7); this file is retained and refreshed as
> the live audit record. The original filename is kept so external references
> still resolve.

## Background

Both traits take `&self` on every method — a deliberate divergence from the
`deep_causality_haft` static-dispatch (associated-fn-on-ZST-witness) convention.
The rationale lives in the trait rustdoc ("## Why methods take `&self`" in
`para/monoidal_category.rs` and `para/actegory.rs`): a future instance that
carries **runtime data** — an `R`-module actegory holding its base ring, a
`QuantaleActegory` holding a Tropical/UnitInterval quantale, a
hyperdoctrine/vector-bundle surface — would need to read `self.{field}`, and
freezing the trait at static methods now would force a breaking change then.

## Current in-tree status (post-#36)

Every shipped instance is a **zero-sized type**, so the `&self` slot is
unobservable in the crate:

| Instance | Kind | Carries runtime data? | Reads `self.{field}`? |
|---|---|---|---|
| `SetMonoidal` | `(Set, ×, 1)` | no (ZST) | no |
| `SetActegory` | self-action of `SetMonoidal` | no (ZST) | no |
| `F64Monoidal` (#36) | `(FinReal, ⊕, R⁰)` — **non-cartesian** | no (ZST) | no |
| `F64Actegory` (#36) | self-action of `F64Monoidal` | no (ZST) | no |

**#36 did not validate the `&self` rationale.** `F64Monoidal` is a genuinely new
kind of instance — the first non-`(Set, ×, 1)` `MonoidalCategory`, exercising the
trait's *object/tensor generality* (direct sum, not product) — but it is still a
ZST whose method bodies ignore the receiver (the runtime data lives in the
**objects** it manipulates, `F64Module(Vec<f64>)`, not in `self`). So the trait's
`&self` slot remains forward-only after #36.

The rationale's designated validators — a `QuantaleActegory` /
`UnitIntervalQ` / `TropicalQ` / `QuantaleDefault` actegory carrying quantale
runtime data — are **out of tree**: they belong to the coalition/enriched-magnitude
consumer track (the `koalisi` / `catgraph-coalition` line), a separate repository,
not `catgraph`. Until such an instance lands somewhere this crate can observe, the
audit cannot fire on real code.

## Audit criteria (fire when a runtime-data-carrying instance exists)

For a first non-ZST `impl MonoidalCategory` / `impl Actegory<M>`:

1. **Substantive `&self` read** — does any method body read `&self.{field}`
   substantively (not a `let _ = self;` placeholder)?
2. **Source-compat regression** — would removing `&self` (switching to a static
   `fn act<P, X>(parameter: P, x: X) -> …`) break that consumer's source-compat?
3. **Caller ergonomics** — at a `tie_weights::<ThatActegory, …>(p, untied)` call
   site, does any caller need `&self` access to the actegory instance?

## Verdict

- **Ratify** (any criterion → Yes): the `&self` slot is validated; keep the trait
  surface and close this audit.
- **Reconsider** (all → No, i.e. still forward-only): open a design discussion on
  whether to switch to static methods (a breaking change) or to document the slot
  as deliberate forward-proofing with no current consumer.

As of 2026-07-13 the answer is "no validator in tree" — the decision **stands**
on the forward-proofing rationale; revisit when a runtime-carrying instance
(quantale actegory, hyperdoctrine, vector bundle) first lands in a crate this repo
builds.

## Cross-references

- Live rationale: the "## Why methods take `&self`" sections in
  `para/monoidal_category.rs` and `para/actegory.rs`.
- `F64Module` R-module actegory (#36) — the non-cartesian ZST instance that
  exercises tensor generality but not the `&self` slot.
- Out-of-tree validators: the coalition/enriched-magnitude track
  (`QuantaleActegory` family), referenced in `para/comonoid.rs`.
- `deep_causality_haft` static-dispatch convention: the `causality:hkt-type-system`
  skill (causality plugin).
