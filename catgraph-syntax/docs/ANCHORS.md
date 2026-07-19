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
| `arrow_seam` (haft re-exports) | — | #12 single-seam precedent (catgraph-dl `src/endofunctor.rs`); {`Arrow`,`Compose`,`Split`,`Id`,`Lift`} consumed by S5 `Traced`; {`arrow`,`ArrowBuilder`,`First`,`Second`,`Fanout`} reserved surface | live re-export (S1); {consumed, reserved} split (S5) |
| `text::parse::parse` (parser) | Seven Sketches | Def 5.30 (concrete syntax of a `G`-generated prop expression, parsed) | live (S2) |
| `text::presentation::{print_presentation, parse_presentation}` | Seven Sketches | Def 5.33 (presentation = arity-matched equation pairs) | live (S2) |
| `sfg_syntax` (`GeneratorSyntax for SfgGenerator<R>`) | Seven Sketches | Def 5.45 / Eq 5.52 (the `G_R` demo signature's token scheme) | live (S2) |
| `eval::ArrowModel` | Seven Sketches | Def 5.25 (a semantics = the generator action extended along the free prop) | live (S3) |
| `eval::eval` | Seven Sketches | Def 5.25 (executable term-action); Thm 5.53 / 5.60 (agrees with the Mat(R) functor); Def 5.50 / Remark 5.49 (row-vector convention — basis row `i` = matrix row `i`) | live (S3) |
| `eval::SfgModel` | Seven Sketches | Def 5.45 / Eq 5.52 (R-linear Σ_SFG action); Thm 5.53 (matches `S : SFG_R → Mat(R)`) | live (S3) |
| `frobenius::FrobeniusOr` | Hypergraph Categories | Def 2.5 (the SCFM generators μ/η/δ/ε as a sum over `G`); Def 2.12 (hypergraph category) | live (S4) |
| `frobenius::{spider, cup, cap}` | Seven Sketches + Hypergraph Categories | "spider" vocabulary + fusion: Seven Sketches Def 6.54 / Thm 6.55 (§6.3.1 — F&S 2019 never uses the word); SCFM axioms: F&S 2019 Def 2.5 §2.2 (monochromatic, Λ = {•}); cospan model: Ex 2.8 (unique apex-1 cospan) | live (S4) |
| `frobenius::scfm_equations` | Hypergraph Categories | Def 2.5 (the **nine** equations, per Ex 2.8's count) | live (S4) |
| `frobenius::hypergraph_presentation` | Seven Sketches + Hypergraph Categories | Def 5.33 (presentation) seeded with `E_frob` = Def 2.5's nine equations | live (S4) |
| `frobenius::to_mat_kron` | Hypergraph Categories | Prop 3.8 (SCFM = strict SM functor `Cospan → C`, the sound checker); Thm 3.14 (`Cospan` is the free monochromatic hypergraph category); *extension of* Ex 2.16 (FdVect-with-chosen-basis is a hypergraph category [Kis15]) from a field to an arbitrary rig — `MatKron(R)`, the Hadamard SCFM target | live (S4) |
| `traced::Wires` | — | the arity-preserving bridge between typed pair bundles and the `Vec<V>` interpreter world (`flatten`/`unflatten`/`COUNT`) | live (S5) |
| `traced::Traced` | — | Hughes 2000 arrow lineage (*Generalising Monads to Arrows*) — the executable-arrow / denoted-term pairing; coherence law `eval(term, m, in.flatten()) == Ok(run(in).flatten())`, inductive: generator constructors *establish* it (caller's value contract), `then`/`par`/`traced_id`/`traced_braid_1_1` *preserve* it | live (S5) |
| `traced::{traced_generator, traced_id, traced_braid_1_1, then, par}` | — | Hughes 2000 arrow combinators (`arr`/`id`/`>>>`/`***`); `fanout` (`&&&`) rejected: Fanout ≠ Frobenius δ | live (S5) |
