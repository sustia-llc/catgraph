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

Delivered incrementally, one phase per branch. The full **S1–S5** surface is
now shipped: **S1–S2 give the round-trip textual surface (printer + parser +
presentation files); S3 adds the interpreter; S4 adds the Frobenius layer; S5
adds the typed builder** — so the
[#5](https://github.com/sustia-llc/catgraph/issues/5) milestone *surface* is
complete. Post-milestone follow-ups (multi-sorted props
[#79](https://github.com/sustia-llc/catgraph/issues/79), the Cospan-valued
complete-functor spike [#80](https://github.com/sustia-llc/catgraph/issues/80),
serde on `PropExpr` [#81](https://github.com/sustia-llc/catgraph/issues/81))
stay tracked on #5.

| Phase | Contents |
|---|---|
| **S1** | workspace member, crate docs, `SyntaxError`, `arrow_seam` (haft seam), the structural pretty-printer (`GeneratorSyntax`, `Pretty`, `print`) |
| **S2** | recursive-descent parser (`parse`) + presentation print/parse + the `SfgGenerator<R>` `GeneratorSyntax` impl (round-trip law-tested) |
| **S3** | interpreter (`ArrowModel`, `eval`, `SfgModel`) — the executable term-action, cross-checked against the Thm 5.53 matrix functor |
| **S4** | Frobenius layer (`FrobeniusOr`, spiders, the nine SCFM equations, `hypergraph_presentation`, the sound `to_mat_kron` checker) |
| **S5** | typed `Traced` builder over the haft Arrow seam (`Wires`, paired combinators) — one value that both *runs* and *denotes a term* |

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

### Frobenius layer (S4)

`FrobeniusOr<G>` adjoins the four special-commutative-Frobenius generators
(`Mu` `2→1`, `Eta` `0→1`, `Delta` `1→2`, `Epsilon` `1→0`) to a user signature
`G` as a **sum type** — `FrobeniusOr<G>` is itself a `PropSignature`, so
`PropExpr<FrobeniusOr<G>>` reuses the whole engine (NF, presentation, `eq_mod`,
parser/printer, `eval`) with no new AST. This presents the *monochromatic* free
hypergraph category (F&S 2019, `Λ = {•}`).

- **Spiders.** `spider(m, n)` collapses `m` legs to one wire via a μ-comb then
  expands to `n` via a δ-comb (`spider(0,0) = η;ε`; `spider(1,1) = id(1)`).
  `cup()` (`0→2 = η;δ`) and `cap()` (`2→0 = μ;ε`) match `MatKron::cup`/`cap`.
- **`scfm_equations()`** — the **nine** Def 2.5 equations (Ex 2.8: "the nine
  equations in Definition 2.5"; the design note's "ten" was corrected against
  the paper). `hypergraph_presentation(user_eqs)` seeds a `Presentation` with the
  lifted user equations plus these nine (`E_frob`).
- **`to_mat_kron(expr, dim)`** — a **sound** semantic checker (Prop 3.8): the
  Hadamard SCFM on `R^dim` induces the strict SM functor from `Cospan` (the free
  monochromatic hypergraph category, Thm 3.14) into `MatKron(R)`, so a proven
  equality maps to equal matrices. A wire `•` maps to `R^dim`, so a `k`-wire
  interface maps to dimension `dim^k` (use `dim ≥ 2`; `dim = 1` degenerates).
  `User(g)` generators are out of the functor's domain
  (`SyntaxError::NonFrobenius`); a `dim^k` overflow is
  `SyntaxError::DimensionOverflow`. `to_mat_kron` is **not** registered as a
  `CompleteFunctor` — no completeness theorem is claimed for `E_frob` (the
  Cospan-valued complete-functor spike is
  [#80](https://github.com/sustia-llc/catgraph/issues/80)), so equal images
  witness equality *in `MatKron(R)`*, and the #15 boundary below still governs
  `eq_mod` over `E_frob` (it overlaps, so `None` is an expected non-disproof).

### Typed builder (S5)

A `Traced<A, G>` carries a morphism as **both** an executable haft `Arrow` `A`
**and** the `PropExpr<G>` term it denotes, so one value can be *run* (via the
arrow) and *reasoned about* (print it, parse over it, `eval` it under any
`ArrowModel`, normalize it, feed it to the presentation engine). It is the typed
track of the Arrow bridge: the S3 interpreter works over flat `Vec<V>` wire
bundles, while haft arrows speak in nested pairs, and `Wires<V>` is the lawful
bridge — `Wire<V>` is one wire, `()` is zero, `(L, R)` is `L` then `R`, and
`flatten`/`unflatten` (with the compile-time `WireCount::COUNT`) move any pair-tree
to/from the canonical `Vec<V>`.
Because `Split`'s input is a bare `(In1, In2)` pair, every tensor of bundles is
automatically a `Wires` bundle.

- **The pairing invariant.** The fields are private; the term's arities stay in
  sync with the arrow's interface types *only* because the sole way to build a
  `Traced` is through the paired combinators, each of which advances arrow and
  term together. There is no constructor from parts, and `Wires` / `WireCount`
  are **sealed** (the three bundle shapes are their only inhabitants), so no
  downstream impl can make a bundle's `COUNT` disagree with its `flatten` length —
  which is why the invariant genuinely cannot be violated from outside the module.
- **Combinators.** `traced_generator(g, arrow)` is the one fallible constructor
  (it checks the arrow's `WireCount::COUNT` against `g.source()`/`g.target()` — a
  *structural* check; value-level agreement with the model is the caller's
  contract). `traced_id`, `traced_braid_1_1` (the single-wire swap), `then`
  (`>>>`), and `par` (`***`) are all **infallible**: `then`'s type equality
  `A::Out = B::In` plus the (sealing-guaranteed) sync invariant make
  `Free::compose` arity-safe at compile time — the payoff of the typed track over
  the interpreter's runtime arity check.
- **The coherence law (the S5 milestone).**
  `eval(t.term(), &model, input.flatten()) == Ok(t.run(input).flatten())`, stated
  inductively: the generator constructors *establish* it for a model whose actions
  agree with the paired arrows (arity checked structurally by `traced_generator`,
  value agreement the caller's contract), and `then`/`par`/`traced_id`/`traced_braid_1_1`
  *preserve* it — so the conditionality is confined to the generator base case and
  every structural combinator is unconditionally sound. Law-tested over every
  combinator with proptest-random input *values* (shapes are type-level).
- **Three deliberate omissions.** General `braid(m, n)` (would need type-level
  rebracketing of nested pairs); `fanout`/`&&&` (**rejected** so the arrow cannot
  copy a wire no term generator copied — Fanout ≠ Frobenius δ); and spider arrows
  (haft `Arrow` has no Frobenius structure). The `traced` module docs are the
  canonical statement of each rejection.

## Two standing disclaimers

1. **The [#15](https://github.com/sustia-llc/catgraph/issues/15) completeness
   boundary.** Applied's congruence-closure decision (`Presentation::eq_mod`)
   is sound but syntactically incomplete by design — a non-`Some(true)` result
   is *not* a disproof. Complete decisions come only through the functorial
   route (`eq_mod_functorial` + a `CompleteFunctor`), which today means Mat(R)
   (`MatrixNFFunctor`, Seven Sketches Thm 5.60). Nothing here promotes an
   incomplete `None` into a decision.

2. **Monochromatic-fragment scope.** The Frobenius layer (S4, live) presents the
   single-sort free hypergraph category — the object palette is `Λ = {•}`, one
   wire colour, one spider family. F&S 2019 Thm 3.14's full **colored** generality
   is out of scope here and tracked as
   [#79](https://github.com/sustia-llc/catgraph/issues/79).

## haft seam

`deep_causality_haft` is consumed through the single file
[`src/arrow_seam.rs`](src/arrow_seam.rs) — the only file naming haft, following
catgraph-dl's `src/endofunctor.rs` precedent
([#12](https://github.com/sustia-llc/catgraph/issues/12)). Its Arrow re-exports
are live public API; the Arrow surface itself is first exercised by the S5
`Traced` builder ([typed builder](#typed-builder-s5) above).
