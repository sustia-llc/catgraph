# catgraph-syntax

A textual generator/relation presentation surface for hypergraph-category
morphisms (issue [#5](https://github.com/sustia-llc/catgraph/issues/5), the
Phase 6 milestone). Morphisms are terms of the free prop over a signature —
[`catgraph-applied`](../catgraph-applied)'s
`PropExpr<G>` / `Free` engine — and this crate adds the *textual* layer above
that engine. It never re-derives the term AST or the decision procedures.

Anchors: Fong & Spivak 2018, *Seven Sketches in Compositionality* (Def
5.25/5.30/5.33, Thm 5.60) and 2019, *Hypergraph Categories* (the Frobenius
layer). Item-by-item map in [`docs/ANCHORS.md`](docs/ANCHORS.md).

## Scope

Delivered incrementally, one phase per branch. **S1 (this phase) ships the
printer only.**

| Phase | Contents |
|---|---|
| **S1** | workspace member, crate docs, `SyntaxError`, `arrow_seam` (haft seam), the structural pretty-printer (`GeneratorSyntax`, `Pretty`, `print`) |
| S2 | recursive-descent parser + presentation print/parse + `GeneratorSyntax` impls (round-trip law-tested) |
| S3 | interpreter (`ArrowModel`, `eval`, `SfgModel`) |
| S4 | Frobenius layer (`FrobeniusOr`, spiders, SCFM equations, hypergraph presentation) |
| S5 | typed `Traced` builder over the haft Arrow seam (cuttable) |

### Printer (S1)

`print(&expr)` / `Pretty(&expr)` render a `PropExpr<G>` to ASCII concrete
syntax. The grammar is `expr := term (';' term)*`,
`term := factor ('*' factor)*`, `factor := id(n) | braid(m,n) | GENERATOR |
'(' expr ')'`: composition `;` is the loosest operator, tensor `*` binds
tighter, both are left-associative, and parentheses are emitted only where the
tree structure requires them to reparse identically.

The printer is **structural and total**: it renders a term exactly as written
and never normalizes.

## Two standing disclaimers

1. **The [#15](https://github.com/sustia-llc/catgraph/issues/15) completeness
   boundary.** Applied's congruence-closure decision (`Presentation::eq_mod`)
   is sound but syntactically incomplete by design — a non-`Some(true)` result
   is *not* a disproof. Complete decisions come only through the functorial
   route (`eq_mod_functorial` + a `CompleteFunctor`), which today means Mat(R)
   (`MatrixNFFunctor`, Seven Sketches Thm 5.60). Nothing here promotes an
   incomplete `None` into a decision.

2. **Monochromatic-fragment scope.** The future Frobenius layer presents the
   single-sort free hypergraph category — the object palette is `Λ = {•}`, one
   wire colour. F&S 2019 Thm 3.14's full **colored** generality is out of scope
   here and tracked as [#79](https://github.com/sustia-llc/catgraph/issues/79).

## haft seam

`deep_causality_haft` is consumed through the single file
[`src/arrow_seam.rs`](src/arrow_seam.rs) — the only file naming haft, following
catgraph-dl's `src/endofunctor.rs` precedent
([#12](https://github.com/sustia-llc/catgraph/issues/12)). Its Arrow re-exports
are live public API now; the Arrow surface itself is first exercised by the S5
`Traced` builder.
