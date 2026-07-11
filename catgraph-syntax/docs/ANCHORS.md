# catgraph-syntax — paper anchors

Every public item maps to a theorem/definition in a paper (PDFs are not kept
in-tree; fetch from arXiv):

- **Seven Sketches** — Fong & Spivak 2018, *Seven Sketches in Compositionality*:
  [arXiv:1803.05316v3](https://arxiv.org/abs/1803.05316)
- **Hypergraph Categories** — Fong & Spivak 2019:
  [arXiv:1806.08304v3](https://arxiv.org/abs/1806.08304)

Status legend: **live** = shipped this phase; **planned** = arrives in a named
later phase.

| Public item | Paper | Anchor | Status |
|---|---|---|---|
| `text::GeneratorSyntax` | Seven Sketches | Def 5.25 (prop signature / `Free(G)`) — the lexical layer over a generator set | live (S1) |
| `text::print::Pretty` | Seven Sketches | Def 5.30 (a `G`-generated prop expression) — concrete syntax of a free-prop term | live (S1) |
| `text::print::print` | Seven Sketches | Def 5.30 | live (S1) |
| `errors::SyntaxError` | — | crate-local error surface; `Catgraph(..)` passes through applied's arity failures | live (S1) |
| `arrow_seam` (haft re-exports) | — | #12 single-seam precedent (catgraph-dl `src/endofunctor.rs`); Arrow surface exercised from S5 (`Traced`) | live re-export (S1); exercised S5 |
| `text::parse::parse` (parser) | Seven Sketches | Def 5.30 (concrete syntax of a `G`-generated prop expression, parsed) | live (S2) |
| `text::presentation::{print_presentation, parse_presentation}` | Seven Sketches | Def 5.33 (presentation = arity-matched equation pairs) | live (S2) |
| `sfg_syntax` (`GeneratorSyntax for SfgGenerator<R>`) | Seven Sketches | Def 5.45 / Eq 5.52 (the `G_R` demo signature's token scheme) | live (S2) |
| interpreter (`ArrowModel`, `eval`) | Seven Sketches | Def 5.25; Thm 5.60 (Mat(R) NF) | planned (S3) |
| Frobenius layer (`FrobeniusOr`, spiders) | Hypergraph Categories | Def 2.5/2.12, Prop 3.8, Thm 3.14 (monochromatic fragment, Λ = {•}) | planned (S4) |
| `Traced` typed builder | — | Hughes 2000 arrow lineage; Fanout ≠ Frobenius δ | planned (S5) |
