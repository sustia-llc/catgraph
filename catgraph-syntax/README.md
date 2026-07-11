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

Delivered incrementally, one phase per branch. **S1–S2 ship the round-trip
textual surface (printer + parser + presentation files); S3 adds the
interpreter.**

| Phase | Contents |
|---|---|
| **S1** | workspace member, crate docs, `SyntaxError`, `arrow_seam` (haft seam), the structural pretty-printer (`GeneratorSyntax`, `Pretty`, `print`) |
| **S2** | recursive-descent parser (`parse`) + presentation print/parse + the `SfgGenerator<R>` `GeneratorSyntax` impl (round-trip law-tested) |
| **S3** | interpreter (`ArrowModel`, `eval`, `SfgModel`) — the executable term-action, cross-checked against the Thm 5.53 matrix functor |
| S4 | Frobenius layer (`FrobeniusOr`, spiders, SCFM equations, hypergraph presentation) |
| S5 | typed `Traced` builder over the haft Arrow seam (cuttable) |

### Printer + parser (S1–S2)

`print(&expr)` / `Pretty(&expr)` render a `PropExpr<G>` to ASCII concrete
syntax, and `parse(&text)` reads it back. The grammar is
`expr := term (';' term)*`,
`term := factor (('⊗' | '*') factor)*`,
`factor := id(n) | braid(m,n) | GENERATOR | '(' expr ')'`: composition `;` is
the loosest operator, tensor binds tighter, both are left-associative, and
parentheses are emitted only where the tree structure requires them to reparse
identically. The Unicode tensor `⊗` is an **input** synonym for `*`; output is
ASCII.

The printer is **structural and total** (it renders a term exactly as written
and never normalizes); the parser builds exclusively through the `Free` smart
constructors, so every parse is arity-sound by construction and the round-trip
law `parse(&print(e)) == Ok(e)` holds structurally. Lexical/structural failures
are `SyntaxError::Parse { offset, .. }`; arity failures pass through as
`SyntaxError::Catgraph`. Parenthesis-nesting depth is bounded
(`MAX_NESTING_DEPTH`) so untrusted input cannot overflow the stack — the bound
is also the round-trip law's one caveat: a term whose *printed* form nests
deeper (a right-fold of more than `MAX_NESTING_DEPTH` compositions prints one
paren per level) is rejected on reparse, so print-then-parse pipelines should
left-fold machine-built chains or treat the bound as a format limit.

Presentation files (Def 5.33) are one `lhs = rhs` equation per line:
`print_presentation` / `parse_presentation`.

### Interpreter (S3)

`eval(&expr, &model, input)` runs a `PropExpr<G>` as a wire-bundle transformer:
an `ArrowModel<G>` supplies the action on generators, and the engine folds it
through identity, braiding, composition, and tensor. Wire bundles are
`Vec<Value>` sized by arity; `eval` never requires `Value: Clone` (duplication
is a *model* concern — the Fanout diagonal is not a Frobenius `δ`). `SfgModel<R>`
is the worked example: the R-linear semantics of the signal-flow-graph
signature, which under `eval` computes the row-vector action `x ↦ x · S(e)` of
the Thm 5.53 matrix functor. *Shape* violations surface as
`SyntaxError::WireCount` (bad input length) or `SyntaxError::ModelArity` (a model
returning the wrong number of outputs), never a panic. *Value* arithmetic is a
separate matter: for `SfgModel<i64>` the `+`/`*` inherit `i64`'s overflow
behaviour (debug-panic / release-wrap), exactly as the matrix functor does.

```rust
use catgraph_applied::prop::presentation::functorial::{CompleteFunctor, MatrixNFFunctor};
use catgraph_applied::sfg::SfgGenerator;
use catgraph_syntax::eval::{eval, SfgModel};
use catgraph_syntax::text::parse;

// Parse a signal-flow-graph term — copy then add, which doubles its input.
let e = parse::<SfgGenerator<i64>>("copy ; add").unwrap();

let model = SfgModel::<i64>::new();
assert_eq!(eval(&e, &model, vec![21]), Ok(vec![42]));

// Cross-check against the Thm 5.53 matrix functor: feeding the standard basis
// row e_i reproduces row i of the matrix (Def 5.50 row-vector action). Here
// copy ; add is 1 -> 1, so the basis row [1] IS the whole (only) row.
let matrix = MatrixNFFunctor::<i64>::new().apply(&e).unwrap();
assert_eq!(eval(&e, &model, vec![1]), Ok(matrix.entries()[0].clone()));
```

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
