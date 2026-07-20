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

`deep_causality_num` / `deep_causality_haft` pinned `=0.4.1` (haft 0.4.1 ships the
post-0.4.0 categorical machinery — PROP `SymMonoidal`, `ArrowTerm`, native
`NaturalTransformation`, `Cofree` — re-evaluation #93 resolved 2026-07-19:
`Free`/`Cofree` adopted as catgraph-dl's carriers; `ArrowTerm` vs `PropExpr` and
`Category`/`Kleisli` vs `eval` assessed no-adopt; `SymMonoidal` decided-no —
cartesian, not a Frobenius substrate).
`catgraph-applied` + `catgraph-magnitude` depend on `deep_causality_num` (`Zero`/`One`
only); `catgraph-dl` uses `haft`'s `HKT`/`Functor` witnesses as its endofunctor
substrate (EndoFunctor→haft migration landed, #12) and now **uses** `num`'s
root `Zero`/`One` in the `F64Module` R-module actegory (`src/para/module_actegory.rs`,
#36 first bullet landed — the direct-sum monoidal category `(FinReal, ⊕, R⁰)`;
umbrella #36 stays open for hyperdoctrine/vector-bundle/lazy surfaces);
`catgraph` (core) + `catgraph-physics` are DC-free.

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
  (Frobenius/hypergraph layer); haft Arrow via the `arrow_seam` (design: #5)

Paper PDFs are **not** kept in-tree (arXiv licensing does not grant
redistribution for all anchors); fetch papers via the arXiv links in each
crate's `docs/`.

## Rules (the only ones)

1. **The paper is the spec.** Theorems move/stay intact — no re-derivation.
2. **`Rig` is a semiring** (catgraph-native). Never swap it for a `deep_causality_num`
   `Ring` — DC's lowest ring requires `Sub`; `BoolRig` / `Tropical` have none.
   Re-source only `Zero` / `One` from `deep_causality_num`.
3. **Integer SNF / `Z(BigInt)` / Storjohann / Newman stay custom** (DC has no integer-ring SNF).
4. **Every change is green** `cargo test --workspace` + clippy before merge.

Work is tracked as GitHub issues. Contributing: see [`CONTRIBUTING.md`](CONTRIBUTING.md).

> **Status:** crate migration complete — the five proven crates (core / applied /
> magnitude / physics / dl) landed on the thin DC substrate (Phases 0–5, merged).
> Phase 6 (`catgraph-syntax`, the Arrow presentation frontend, #5) is
> **complete** (S1–S5 merged 2026-07-11): S1 printer, S2 parser + presentation
> files, S3 interpreter (ArrowModel/eval/SfgModel), S4 Frobenius layer
> (FrobeniusOr/spiders/E_frob/to_mat_kron), S5 Traced typed builder over the
> haft Arrow seam. Post-milestone follow-ups on #5 (#79/#80/#81); other open
> follow-ups + audit/README reconciliation tracked as GitHub issues (e.g. #7).
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
> audit-doc tallies self-consistent. Substantive gaps live as issues
> (#124 physics ANCHORS note, #126 Prop 5.56 `mat_to_sfg`, #117 Joyal–Street
> sourcing).
