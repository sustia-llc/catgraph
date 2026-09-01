# catgraph

Category-theoretic graph structures in Rust — strict Fong & Spivak,
*Hypergraph Categories* (2019), plus applied / magnitude / physics / DL extensions.

## Build & test

```sh
cargo nextest run <scope> --no-fail-fast                  # suites; cargo test only for -- --nocapture guard logs
cargo clippy <scope> --all-targets -- -D warnings         # the CI gate, on every feature lane (.github/workflows/ci.yml)
cargo fmt    --all --check
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
python3 scripts/check_version_refs.py ; scripts/check_rand_dev_only.py ; scripts/check_audit_counts.py <audit docs> ; scripts/check_measured_claims.py <nocapture log>
```

`<scope>` while iterating = the touched crate **and its dependents** (core or
testutil → `--workspace`; applied → `-p catgraph-applied -p catgraph-dl
-p catgraph-magnitude -p catgraph-syntax`; dl / magnitude / physics / syntax →
`-p <crate>`); `--workspace` once before the PR. Never `--workspace` on a wasm
target. Each gate its own shell call, output to a file, never piped.

## Crate graph (dependency order)

```
catgraph (F&S core) ─▶ catgraph-applied ─▶ catgraph-magnitude
        └─▶ catgraph-physics              ├─▶ catgraph-dl
                                          └─▶ catgraph-syntax
```

`catgraph-testutil` is a seventh workspace member: a **dev-only, unpublished**
(`publish = false`) crate of shared test/bench helpers (a deterministic LCG for
seeded fixtures, relative float comparison, shared proptest float strategies,
and exhaustive permutation enumeration), pulled in only via
`[dev-dependencies]` — never a published crate's `[dependencies]` (#33).

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
4. **Every change is green** — the gate list above, on the scope above, before merge.
5. **No per-PR CHANGELOG edits.** `CHANGELOG.md` is written at tag time from the
   merged PR titles, one line each, or by a dedicated prose PR. A PR body is the
   one record of a change; the commit body states what changed and nothing more.
   Rustdoc states **what** over **what input space** — no why, no history, no
   hand-maintained counts; a universal names the command that establishes it.
6. **A test is a claim.** New pins are falsified (revert the change, the test
   goes red, restore) with observed vs expected in the failure message; a
   docstring never quantifies more than the assertions do. Existing tests are
   not retro-audited — the 2026-09 triage folded that queue.

Work is tracked as GitHub issues (`taskmap.md` §Triage in `.claude/docs/` is
the live order). Contributing: see [`CONTRIBUTING.md`](CONTRIBUTING.md).
History (crate migration, DC divestment #218–#222, paper audit #112–#128) is
in git and each crate's CHANGELOG.
