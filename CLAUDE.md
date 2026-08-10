# catgraph

Category-theoretic graph structures in Rust — strict Fong & Spivak,
*Hypergraph Categories* (2019), plus applied / magnitude / physics / DL extensions.

## Build & test

```sh
cargo build  --workspace
cargo test   --workspace                                  # every change: green before merge
cargo clippy --workspace --all-targets -- -D warnings     # the CI gate (default lints)
cargo clippy --workspace --all-targets -- -W clippy::pedantic   # advisory local pass (non-gating)
cargo fmt    --all --check
```

## Crate graph (dependency order)

```
catgraph (F&S core) ─▶ catgraph-applied ─▶ catgraph-magnitude
        └─▶ catgraph-physics              ├─▶ catgraph-dl
                                          └─▶ catgraph-syntax
```

`catgraph-testutil` is a seventh workspace member: a **dev-only, unpublished**
(`publish = false`) crate of shared test/bench helpers (currently a deterministic
LCG), pulled in only via `[dev-dependencies]` — never a published crate's
`[dependencies]` (#33).

**Zero external algebraic dependencies** (#218, completed at #222): the whole
substrate is catgraph-owned. catgraph-dl defines the endofunctor witness tower
and the `Free`/`Cofree` carriers (`src/endofunctor/`, `src/free_monad/`);
catgraph-syntax defines the value-level Arrow algebra (`src/arrow_seam.rs`).
The owned surfaces keep *carrier/combinator shape parity* with the
`deep_causality_haft` 0.4.2 code they replaced (MIT, attributed in the
defining files' license headers; 0.4.2 = tag `deep_causality_haft-v0.4.2`,
commit `aeff6549e` — DC `main` read 0.4.1, DC#720), so construction and match
sites compile unchanged — but #222 is BREAKING in both crates: the constraint
slot (`Satisfies`/`NoConstraint`) is gone, the carrier/witness method surface
is the consumed-only set, and haft's provided `⊕` arrow methods are not
carried (each crate's CHANGELOG enumerates its cuts). The #93 no-adopt
verdicts (`ArrowTerm` vs `PropExpr`, `Category`/`Kleisli` vs `eval`,
`SymMonoidal` — cartesian, not a Frobenius substrate) are pin-independent and
stand. No `deep_causality_*` crate remains anywhere in the graph
(`rg deep_causality Cargo.lock */Cargo.toml` stays empty; CI-guarded).

**Streamlining landed (#218):** the graph-crate edge retired in **#220** (D2 —
toposort/connectivity in-tree; see catgraph's CHANGELOG);
`deep_causality_num` retired in **#219** (D1) —
`Zero`/`One` are catgraph's own, defined in `catgraph-applied/src/rig.rs` next
to the native `Rig` and re-exported by magnitude and dl. They back the
`RModule<S>` R-module actegory too (`F64Module = RModule<f64>`;
`src/para/module_actegory.rs`, #36 first bullet — the direct-sum monoidal
category `(FinReal, ⊕, R⁰)`; umbrella #36 stays open for
hyperdoctrine/vector-bundle/lazy surfaces). `deep_causality_num_dual` retired in
**#221** (D3), forced by the same change: the orphan rule forbids implementing a
catgraph-owned `Zero`/`One` for a foreign `Dual`, so forward-mode `Dual<T>` moved
to `catgraph-dl/src/para/dual.rs`. catgraph-dl's off-by-default `ad` feature
(#74 PR2) now adds **no dependency at all**. `deep_causality_haft` retired in
**#222** (D4): the dl endofunctor/carrier substrate and the syntax Arrow
algebra moved in-tree at shape parity; `catgraph-applied`'s published
randomness edge slimmed to `rand_core` in the same window (#239,
`E1::random` over `catgraph_applied::Rng`, `rand` dev-only + CI-guarded).
The #218 streamlining arc is complete.

## Paper anchors

- **catgraph** — Fong & Spivak 2019 (*Hypergraph Categories*); secondary: F&S 2018
  (*Seven Sketches*) for Thm 6.55 spider tests + Ex 6.64 `Corel`
- **catgraph-applied** — Fong & Spivak 2018 (*Seven Sketches in Compositionality*)
- **catgraph-magnitude** — Bradley–Vigneaux 2025; Leinster 2008 / 2013 / 2017
- **catgraph-dl** — Gavranović et al., ICML 2024 (*Categorical Deep Learning*)
- **catgraph-physics** — Gorard 2023 (*A functorial perspective on
  (multi)computational irreducibility*); inspiration-anchored, not
  theorem-anchored — provenance in `catgraph-physics/docs/ANCHORS.md`
- **catgraph-syntax** — F&S 2018 Ch. 5 (props, presentations, Thm 5.60) + F&S 2019
  (Frobenius/hypergraph layer); owned Arrow algebra via the `arrow_seam`
  (design: #5; crate-owned since #222)

Paper PDFs are **not** kept in-tree (arXiv licensing does not grant
redistribution for all anchors); fetch papers via the arXiv links in each
crate's `docs/`.

## Rules (the only ones)

1. **The paper is the spec.** Theorems move/stay intact — no re-derivation.
2. **`Rig` is a semiring, and the whole rig substrate is ours.** `Rig`, `Zero`,
   and `One` are all catgraph-native, in `catgraph-applied/src/rig.rs` (#219).
   Never swap `Rig` for an external `Ring` — the lowest ring in the numeric
   crates requires `Sub`; `BoolRig` / `Tropical` have none.
3. **Integer SNF / `Z(BigInt)` / Storjohann / Newman stay custom** (`num` supplies
   the BigInt, not the algorithm).
4. **Every change is green** `cargo test --workspace` + clippy before merge.

Work is tracked as GitHub issues. Contributing: see [`CONTRIBUTING.md`](CONTRIBUTING.md).

> **Status:** crate migration complete — the five proven crates (core / applied /
> magnitude / physics / dl) landed on the then-current thin DC substrate
> (Phases 0–5, merged; that substrate has since been divested entirely, #218/#222).
> Phase 6 (`catgraph-syntax`, the Arrow presentation frontend, #5) is
> **complete** (S1–S5 merged 2026-07-11): S1 printer, S2 parser + presentation
> files, S3 interpreter (ArrowModel/eval/SfgModel), S4 Frobenius layer
> (FrobeniusOr/spiders/E_frob/to_mat_kron), S5 Traced typed builder over the
> Arrow seam (crate-owned since #222). The post-milestone follow-ups #79/#80/#81 have ALL shipped
> (#80/#81 at v0.4.0, #79 completed at v0.5.0); other open follow-ups +
> audit/README reconciliation tracked as GitHub issues (e.g. #7).
>
> **Paper-audit (papers-vs-implementation citation sweep), ALL phases 1–7
> complete (2026-07-19):** core (#112/#113), applied (#118/#119 — Thm 5.60
> presentation completed to the paper's 18 equations "E_18"; Mat(R) completeness
> attribution corrected to Baez–Erbele for fields + Wadsley–Woods for commutative
> rigs), magnitude (#120/#122 — BV25/Leinster/LS reconciliation + BV25-AUDIT
> recount), physics (#125 — inverted Gorard irreducibility gloss fixed;
> provenance follow-up #124), dl (#128 — phantom "Appendix K", Def 1.4/1.5 swap,
> fabricated section name), and syntax (#127 — spider vocabulary re-anchored to
> FS18 Def 6.54/Thm 6.55; MatKron marked an extension of Ex 2.16) are merged.
> A CI guard (`scripts/check_audit_counts.py`) keeps the FS19/FS18/BV25
> audit-doc tallies self-consistent. Follow-ups resolved 2026-07-19: #117
> (Selinger/JS sourcing — all four papers cached, every SMC-NF anchor
> verified, (†) marks retired) and #124 (physics `docs/ANCHORS.md`).
> The last substantive gap closed 2026-07-21: #126 (Prop 5.56 `mat_to_sfg`
> realization, PR #137) — the audit arc is fully resolved.
