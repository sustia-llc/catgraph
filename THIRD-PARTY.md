# Third-party notices

## deep_causality_haft 0.4.2 (MIT)

Portions of this workspace are derived from
[`deep_causality_haft`](https://crates.io/crates/deep_causality_haft) 0.4.2
(part of the [DeepCausality](https://github.com/deepcausality-rs/deep_causality)
project), used under the MIT license:

> Copyright (c) 2023 - 2026. The DeepCausality Authors.
>
> Permission is hereby granted, free of charge, to any person obtaining a copy
> of this software and associated documentation files (the "Software"), to
> deal in the Software without restriction, including without limitation the
> rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
> sell copies of the Software, and to permit persons to whom the Software is
> furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in
> all copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
> IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
> FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
> AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
> LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
> FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
> IN THE SOFTWARE.

## What is derived, precisely

**The API shape and the design**, not the current implementations. What
originated with haft and survives here: the carrier shapes (`Pure | Suspend`
with the box inside the functor hole; `head :< tail`), the witness tower and its
GAT-based HKT emulation, the `fold` / `unfold` / `new` / `head` / `tail` /
`into_parts` signatures, and the opt-in capability-trait mechanism that works
around `E0275`.

**The bodies are catgraph's own**, and increasingly so. `catgraph-syntax`'s
Arrow algebra has *defined* rather than re-exported its surface since
[#222](https://github.com/sustia-llc/catgraph/issues/222). In `catgraph-dl`,
[#200](https://github.com/sustia-llc/catgraph/issues/200) replaced every
recursive walk with an explicit worklist, moved `Free`/`BinaryTree` behind a
private representation, made the capability traits shape-level, and reshaped
`TreeView::Node`. The notice is retained because the derivation is real and MIT
requires it to travel with derived work — **not** because any
`deep_causality_*` crate is a dependency. None is, anywhere in the graph, and
CI enforces that.

The derived files (each also carries the notice in its license header; the
substrate was brought in-tree at #222). Provenance note:
0.4.2 is the crates.io release, tag `deep_causality_haft-v0.4.2`, commit
`aeff6549e` in the DeepCausality repository — its `main` branch read 0.4.1 at
divestment time (deepcausality-rs/deep_causality#720).

- `catgraph-dl/src/endofunctor/hkt.rs`
- `catgraph-dl/src/endofunctor/either.rs`
- `catgraph-dl/src/endofunctor/capability.rs`
- `catgraph-dl/src/endofunctor/natural_iso.rs`
- `catgraph-dl/src/endofunctor/option_witness.rs`
- `catgraph-dl/src/free_monad/free.rs`
- `catgraph-dl/src/free_monad/cofree.rs`
- `catgraph-syntax/src/arrow_seam.rs`
