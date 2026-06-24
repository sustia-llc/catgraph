# `AUDIT-CHECKPOINT-v0.4.0` — HKT `&self` rationale audit

**Status:** Audit predicates declared 2026-05-10 alongside cg-dl v0.4.0; **audit fires at coalition v0.5.0 post-shipping review**.

## Background

cg-dl's [`MonoidalCategory`](../src/para/monoidal_category.rs) and [`Actegory<M>`](../src/para/actegory.rs) traits take `&self` on every method as a deliberate divergence from the HAFT v0.3.1 static-dispatch convention. The rustdoc at `src/para/monoidal_category.rs` "## Why methods take `&self`" + `src/para/actegory.rs` "## Why methods take `&self`" explains the rationale: future R-module / hyperdoctrine / vector-bundle / `QuantaleActegory` instances will carry runtime data, and freezing the trait at static methods would force a breaking change later.

The shipped instances at v0.4.0 are ZSTs (`SetMonoidal`, `SetActegory`), so the `&self` slot is unobservable in-tree. The rationale empirically requires a first downstream consumer that genuinely carries runtime data. v0.4.0 commits to the rationale and cites `catgraph-coalition` v0.5.0 as the validating consumer; this audit fires at coalition v0.5.0 post-shipping review and either ratifies the choice or opens a follow-up to consider static dispatch.

## Audit criteria

For each of the three coalition v0.5.0 actegory impls — `impl Actegory<SetMonoidal> for UnitIntervalQ`, `impl Actegory<SetMonoidal> for TropicalQ`, `impl Actegory<SetMonoidal> for QuantaleDefault` — and each of the three coalition v0.5.0 monoidal-category impls (via `SetCategoryDefaults` blanket, so this trivially holds if the soft-seal is honoured):

### Criterion 1 — substantive `&self` read

For each method body in coalition v0.5.0, does it **read `&self.{field}` substantively** (not just `let _ = self;` placeholder)?

- `Actegory::act` body: reads `&self.{field}`?
- `Actegory::compose_action` body: reads `&self.{field}`?
- (For `SetCategoryDefaults` consumers): does any blanket-derived `MonoidalCategory` method body read `&self.{field}` substantively?

### Criterion 2 — source-compat regression

Would removing `&self` from the trait surface (switching to `fn act<P, X>(parameter: P, x: X) -> Self::ActionResult<P, X>` static method) **break coalition v0.5.0 source-compat**?

- Yes → `&self` rationale empirically validated; coalition v0.5.0 has a substantive use.
- No → `&self` rationale is forward-only; no current consumer; revisit at v0.5.0+ design.

### Criterion 3 — caller ergonomics

At coalition v0.5.0's `tie_weights::<UnitIntervalQ, …>(p, untied)` call site (or analogous): does any caller need `&self` access to the actegory instance to compute the parameter or the morphism body?

- Yes → caller-side `&self` use validates the trait surface.
- No → caller never names the actegory instance; `&self` is purely a definition-site convenience.

## Audit verdict (post-coalition-v0.5.0-shipping)

Three outcomes possible:

1. **Ratify (Criterion 1 or 2 or 3 → Yes):** `&self` rationale validated; v0.3.x rustdoc + v0.4.0 rationale paragraph stay; close audit.
2. **Open follow-up — consider static dispatch:** all three criteria → No; `&self` is forward-only with no concrete consumer; open a v0.5.0+ design discussion on whether to switch to static methods (breaking change) or document the slot as forward-future-proofing only.
3. **Open follow-up — refine rustdoc:** mixed — the rationale needs sharpening; e.g., one criterion validates and the other two don't. Refine the rustdoc to cite the specific validator.

## Cross-references

- v0.4.0 design doc §2.2 surface (b) — where this audit was scoped.
- v0.4.0 forward-look §1.2 — original audit-checkpoint articulation.
- Coalition v0.5.0 design doc (to be written after cg-dl v0.4.0 ships) — should explicitly link this doc as the audit target.
- HAFT static-dispatch convention reference: `causality:hkt-type-system` skill (rebozo plugin v1.2.0).
