# catgraph-syntax — paper anchors

Every `pub` declaration of this crate is mapped below — to a theorem/definition
in a paper, or to a crate-local rationale ("—") where no anchor exists (PDFs are
not kept in-tree; fetch from arXiv). Coverage is established by matching the
table against `rg -o '^pub (struct|enum|type|const|trait|fn)' catgraph-syntax/src`,
run by hand; one row may cover several declarations of the module that defines
them.

- **Seven Sketches** — Fong & Spivak 2018, *Seven Sketches in Compositionality*:
  [arXiv:1803.05316v3](https://arxiv.org/abs/1803.05316)
- **Hypergraph Categories** — Fong & Spivak 2019:
  [arXiv:1806.08304v3](https://arxiv.org/abs/1806.08304)

Status legend: **live** = shipped this phase; **planned** = arrives in a named
later phase.

| Public item | Paper | Anchor | Status |
|---|---|---|---|
| `text::GeneratorSyntax` | Seven Sketches | Def 5.25 (prop signature / `Free(G)`) — the lexical layer over a generator set | live (S1) |
| `text::ColorSyntax` | Hypergraph Categories | Def 3.9 (an *objectwise-free* structure is a set `Λ` with a monoid isomorphism `List(Λ) ≅ Ob(C)`) — the lexical layer over `Λ`, one token per letter | live (#79 P3b) |
| `text::print::Pretty` | Seven Sketches | Def 5.30 (a `G`-generated prop expression) — concrete syntax of a free-prop term | live (S1) |
| `text::print::print` | Seven Sketches | Def 5.30 | live (S1) |
| `errors::SyntaxError` | — | crate-local error surface; `Catgraph(..)` passes through applied's arity failures | live (S1) |
| `arrow_seam` (owned Arrow algebra) | Hughes 2000 (*Generalising Monads to Arrows*; lineage citation, not a theorem anchor) | #12 single-seam precedent (catgraph-dl's `endofunctor` module); crate-owned definitions since #222, derived from `deep_causality_haft` 0.4.2 (MIT, attributed in the module header; re-exports of that crate before #222; the ⊕ half — `left`/`right`/`choice`/`fanin` — deliberately not carried over); {`Arrow`,`Compose`,`Split`,`Id`,`Lift`} consumed by S5 `Traced`; {`arrow`,`ArrowBuilder`,`First`,`Second`,`Fanout`,`ThenFn`} live public surface | live (S1 as re-export, owned at #222); all ten haft-parity names law-tested (`tests/arrow_laws.rs`, #222), plus the owned 11th public name `ThenFn` (pub type, #222) exercised there via `then_fn` |
| `text::parse::{parse, MAX_NESTING_DEPTH}` (parser) | Seven Sketches | Def 5.30 (concrete syntax of a `G`-generated prop expression, parsed); `MAX_NESTING_DEPTH` (256) is the crate-local cap on parenthesis nesting `parse` accepts, one level per open parenthesis | live (S2) |
| `text::presentation::{print_presentation, parse_presentation}` | Seven Sketches + Hypergraph Categories | Def 5.33 (presentation = boundary-matched equation pairs — *words* since #79 P2); the declaration line `g : A B -> C` writes the generator's Def 3.9 source/target words, checked against `G` | live (S2); declarations since #79 P3b |
| `sfg_syntax` (`GeneratorSyntax for SfgGenerator<R>`) | Seven Sketches | Def 5.45 / Eq 5.52 (the `G_R` demo signature's token scheme) | live (S2); scalar token `scalar_<r>` since #79 P3b |
| `eval::ArrowModel` | Seven Sketches | Def 5.25 (a semantics = the generator action extended along the free prop) | live (S3) |
| `eval::eval` | Seven Sketches | Def 5.25 (executable term-action); Thm 5.53 / 5.60 (agrees with the Mat(R) functor); Def 5.50 / Remark 5.49 (row-vector convention — basis row `i` = matrix row `i`) | live (S3) |
| `eval::SfgModel` | Seven Sketches | Def 5.45 / Eq 5.52 (R-linear Σ_SFG action); Thm 5.53 (matches `S : SFG_R → Mat(R)`) | live (S3) |
| `frobenius::{FrobeniusOr, lift_user}` | Hypergraph Categories | Def 2.5 (the SCFM generators μ/η/δ/ε as a sum over `G`); Def 2.12 (a hypergraph category equips **each object** with a Frobenius structure — hence the `Λ` payload on the four spider variants) | live (S4); Λ-colored since #79 P3a, colour-annotated tokens (`mu@A`) since P3b |
| `frobenius::{spider, cup, cap}` | Seven Sketches + Hypergraph Categories | "spider" vocabulary + fusion: Seven Sketches Def 6.54 / Thm 6.55 (§6.3.1 — F&S 2019 never uses the word); SCFM axioms: F&S 2019 Def 2.5 §2.2, instantiated at the builder's colour; cospan model: Ex 2.8 (unique apex-1 cospan), transported to each `l ∈ Λ` by Ex 3.12 | live (S4); colour parameter since #79 P3a |
| `frobenius::{scfm_equations, FrobeniusEquation}` | Hypergraph Categories | Def 2.5 (the **nine** equations, per Ex 2.8's count), at one colour; `FrobeniusEquation<G>` is the `(lhs, rhs)` pair over `FrobeniusOr<G>` those equations take | live (S4); per-colour since #79 P3a |
| `frobenius::hypergraph_presentation` | Seven Sketches + Hypergraph Categories | Def 5.33 (presentation) seeded with `E_frob` = Def 2.5's nine equations **per palette colour**, which suffices by Lemma 3.10 (a Frobenius structure per `l ∈ Λ` induces a unique hypergraph structure on `List(Λ)`; Ex 2.9 describes the induced structure) | live (S4); per-colour since #79 P3a |
| `frobenius::to_mat_kron` | Hypergraph Categories | Prop 3.8 (SCFM = strict SM functor `Cospan → C`, the sound checker); Thm 3.14 (`Cospan_Λ` is the free hypergraph category on `Λ`, so the per-colour Hadamard SCFMs extend uniquely); *extension of* Ex 2.16 (FdVect-with-chosen-basis is a hypergraph category [Kis15]) from a field to an arbitrary rig — `MatKron(R)`, the Hadamard SCFM target | live (S4); worded, per-colour dims since #79 P3a |
| `cospan_functor::{to_cospan, CospanFunctor}` | Hypergraph Categories | Prop 3.8 (an SCFM in `(C, ⊗)` is exactly a **strict** symmetric monoidal functor `(Cospan, ⊕) → (C, ⊗)`, so `Cospan` is the theory of SCFMs — the completeness route for the User-free fragment); Thm 3.14 (`Cospan_Λ` is the free hypergraph category on the set `Λ`, an adjunction `Cospan_ ⊣ Ob : Hyp → Set`) with Ex 3.12 / Lemma 3.10 transporting Ex 2.8's generators to each `l ∈ Λ` | live (S4/[#80](https://github.com/sustia-llc/catgraph/issues/80)); worded + colored since #79 P3a |
| `traced::{Wires, Wire, WireCount}` | — | the arity-preserving bridge between typed pair bundles and the `Vec<V>` interpreter world (`flatten`/`unflatten`/`COUNT`); `Wire<V>` is the one-wire atom, `WireCount` the sealed `V`-free arity of a bundle shape | live (S5) |
| `traced::Traced` | — | Hughes 2000 arrow lineage (*Generalising Monads to Arrows*) — the executable-arrow / denoted-term pairing; coherence law `eval(term, m, in.flatten()) == Ok(run(in).flatten())`, inductive: generator constructors *establish* it (caller's value contract), `then`/`par`/`traced_id`/`traced_braid_1_1` *preserve* it | live (S5) |
| `traced::{traced_generator, traced_id, traced_braid_1_1, then, par, PairSwap}` | — | Hughes 2000 arrow combinators (`arr`/`id`/`>>>`/`***`); `fanout` (`&&&`) rejected: Fanout ≠ Frobenius δ; `PairSwap<V>` names `traced_braid_1_1`'s concrete arrow type | live (S5) |
| `depth::{term_depth, guard_term_depth, MAX_TERM_DEPTH}` | — | crate-local recursion guard for the term interpreters ([#99](https://github.com/sustia-llc/catgraph/issues/99)): iterative structural-depth pre-flight shared by `eval` / `to_mat_kron` / `to_cospan`; `MAX_TERM_DEPTH` = the parser's `MAX_NESTING_DEPTH` (256) | live (v0.4.0, #99) |
