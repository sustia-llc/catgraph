//! Abstract content of a free-prop expression, and equality on it
//! ([#57](https://github.com/sustia-llc/catgraph/issues/57), a1).
//!
//! # What the content is
//!
//! `docs/SMC-NF-RECONCILIATION.md` **§4.1** interprets an arity-well-formed
//! expression `e : n → m` as a **cospan of Λ-typed directed hypergraphs**
//! `n → H ← m`, following
//! Bonchi–Gadducci–Kissinger–Sobociński–Zanasi (**BGKSZ**, arXiv:1602.06771v2,
//! *Rewriting modulo symmetric monoidal structure*) §3: nodes are wires (typed
//! by the color alphabet `Λ` — [`PropSignature::Color`]), hyperedges are
//! generator occurrences with ordered, type-respecting source and target
//! tentacles, and the two anchoring maps embed the boundary words. That
//! interpretation is BGKSZ's `⟦·⟧`; the **content** `C(e)` is its value up to
//! isomorphism *under both feet* — an iso of the carrier `H` commuting with the
//! anchors, identity on `n` and `m`, so every boundary *coordinate* is a content
//! invariant.
//!
//! [`Content`] is that value, [`content_eq`] is that equality, and
//! [`canonical_key`] is a hashable normal form for it.
//!
//! # The anchors
//!
//! - **BGKSZ Prop 3.4** — `⟦·⟧` is faithful, so content separates distinct
//!   SMC-classes.
//! - **BGKSZ Thm 3.12** — the cospans in the image of `⟦·⟧` are exactly the
//!   **monogamous** (Def 3.6) **directed acyclic** (Def 3.9) ones: every interior
//!   node has in- and out-degree exactly 1, and a boundary node has degree 0 on
//!   its anchored side. That is what lets [`content_eq`] avoid search entirely —
//!   the boundary-attached part is decided by forced propagation, and the closed
//!   part by comparing a complete invariant rather than by matching components
//!   against each other.
//! - Together they give **Lemma 4.1** (§4.2): `e =_SMC e′` **iff**
//!   `C(e) = C(e′)`. Stated color-generically there, over an arbitrary `Λ`, so
//!   the [#79](https://github.com/sustia-llc/catgraph/issues/79) worded surface
//!   inherits it.
//!
//! # Why this module exists
//!
//! §4.7 specifies content as the substrate a #57 knowledge base would rewrite:
//! represent terms by `C`, rewrite by convex DPO on content, and use
//! [`smc_nf::nf`](super::smc_nf::nf) as the canonical *readback*. This module is
//! the first half of that — content and its equality — with rewriting-modulo-user
//! -equations deferred. The immediate payoff is stated in §4.7: an engine that
//! decides equality on content never chooses `η` placement (`ι`, the single named
//! canonicality gap of §4.4) at all, so content equality is complete on every
//! divergence class the normal form leaves open.
//!
//! # Layering: this is SMC-equality, not equality modulo user equations
//!
//! `C` quotients by SMC coherence and **nothing else**. `Copy ; Add` and
//! `Copy ; σ ; Add` have different content and are correctly *unequal* here —
//! cocommutativity is one of the 18 Thm 5.60 *user* equations, and those stay
//! with [`Presentation::eq_mod`](super::Presentation::eq_mod)'s congruence
//! closure above this layer.
//!
//! # Words (Λ-colored signatures)
//!
//! Nodes carry a color. [`content_of`] reads it off the generator tentacle
//! *words* ([`PropSignature::source_word`] / [`PropSignature::target_word`]), so
//! it is word-aware for any `Λ` without any positional engine being touched —
//! the same layering #79 used, where the diagram layer stayed positional.
//! [`content_of_colored`] additionally pins the colors an expression alone cannot
//! determine (see [`content_of_colored`] for which those are, and why there are
//! no others). With `Color = ()` the comparison of *letters* degenerates to a
//! no-op — there is only one letter — but the distinction between a typed and an
//! untyped node does not: `Some(())` and `None` still differ, so a monochromatic
//! signature is not the same as one with colors deleted, and the like-with-like
//! caveat on [`content_eq`] binds monochromatic callers exactly as much as
//! colored ones.

use std::collections::VecDeque;

use super::super::colored::ColoredExpr;
use super::super::{PropExpr, PropSignature};

// ---------------------------------------------------------------- the content

/// One hyperedge of a content cospan: a generator occurrence with its ordered
/// source and target tentacles, as indices into the node set.
///
/// Tentacle *positions* are content invariants — an iso under both feet matches
/// `sources[i]` against `sources[i]`, never against `sources[j]`.
#[derive(Clone, Debug)]
pub struct ContentEdge<G: PropSignature> {
    /// The generator occurring at this hyperedge.
    pub label: G,
    /// Nodes consumed, in source-word order.
    pub sources: Vec<usize>,
    /// Nodes produced, in target-word order.
    pub targets: Vec<usize>,
}

/// The content `C(e)`: the anchored cospan `n → H ← m` of §4.1.
///
/// Nodes are `0..node_count`; `input` and `output` are the two anchor maps,
/// listing the node at each boundary coordinate. Node *indices* are an artifact
/// of the expression that produced the value and carry no meaning across
/// contents — [`content_eq`] never reads them structurally, and
/// [`canonical_key`] renumbers them invariantly.
///
/// # No `PartialEq`
///
/// Deliberately not derived. Structural equality of the fields would be
/// *representation* equality, which is finer than content equality and would be
/// the wrong default for a type whose whole point is the quotient. Content
/// equality is [`content_eq`]; the hashable form is [`canonical_key`].
///
/// # Construction invariant
///
/// The fields are private, so every value comes from [`content_of`] /
/// [`content_of_colored`], and the *underlying uncolored* cospan is therefore
/// monogamous directed acyclic — the image characterization of BGKSZ Thm 3.12.
/// Both algorithms below rely on that, and on its consequence that every node is
/// either incident to a hyperedge or anchored on both feet (a wire no generator
/// touches runs from the input foot straight to the output foot). The accessors
/// are read-only, so no caller can break either.
///
/// The tentacle typing is additionally *type-respecting* — putting the value in
/// the image of the Λ-typed `⟦·⟧` — exactly when the expression was
/// word-well-formed. Over a nontrivial `Λ` that is a real restriction, not a
/// tautology: `content_of` accepts an ill-colored `Compose` and returns a
/// perfectly good uncolored cospan whose typing no Λ-typed expression realizes.
#[derive(Clone, Debug)]
pub struct Content<G: PropSignature> {
    node_count: usize,
    node_colors: Vec<Option<G::Color>>,
    edges: Vec<ContentEdge<G>>,
    input: Vec<usize>,
    output: Vec<usize>,
}

impl<G: PropSignature> Content<G> {
    /// Number of nodes (wires).
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.node_count
    }

    /// The `Λ`-typing of each node, indexed by node.
    ///
    /// `None` marks a node whose color the expression does not determine — see
    /// [`content_of_colored`], which pins exactly those.
    #[must_use]
    pub fn node_colors(&self) -> &[Option<G::Color>] {
        &self.node_colors
    }

    /// The hyperedges: one per generator occurrence.
    #[must_use]
    pub fn edges(&self) -> &[ContentEdge<G>] {
        &self.edges
    }

    /// The input anchor: the node at each source coordinate.
    #[must_use]
    pub fn input(&self) -> &[usize] {
        &self.input
    }

    /// The output anchor: the node at each target coordinate.
    #[must_use]
    pub fn output(&self) -> &[usize] {
        &self.output
    }
}

// ---------------------------------------------------------------- construction

/// Union-find over wires, accumulating hyperedges as the expression is walked.
struct Builder<G: PropSignature> {
    parent: Vec<usize>,
    edges: Vec<ContentEdge<G>>,
}

/// A sub-expression's two open boundaries, as node indices.
struct Piece {
    input: Vec<usize>,
    output: Vec<usize>,
}

impl<G: PropSignature> Builder<G> {
    fn fresh(&mut self) -> usize {
        self.parent.push(self.parent.len());
        self.parent.len() - 1
    }

    fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }

    /// Glue two wires.
    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent[ra] = rb;
        }
    }
}

/// `⟦·⟧` by structural recursion: generators become single-hyperedge cospans,
/// identities and braids discrete cospans (the braid with the permuted anchor),
/// `;` glues over the shared foot, and `⊗` is disjoint union with concatenated
/// anchors (§4.1).
fn build<G: PropSignature>(e: &PropExpr<G>, b: &mut Builder<G>) -> Piece {
    match e {
        PropExpr::Identity(n) => {
            let nodes: Vec<usize> = (0..*n).map(|_| b.fresh()).collect();
            Piece {
                input: nodes.clone(),
                output: nodes,
            }
        }
        PropExpr::Braid(m, n) => {
            let nodes: Vec<usize> = (0..m + n).map(|_| b.fresh()).collect();
            // σ_{m,n} : u ⊗ v → v ⊗ u — the anchor is the block swap.
            let mut output = Vec::with_capacity(m + n);
            output.extend_from_slice(&nodes[*m..]);
            output.extend_from_slice(&nodes[..*m]);
            Piece {
                input: nodes,
                output,
            }
        }
        PropExpr::Generator(g) => {
            let sources: Vec<usize> = (0..g.source_word().len()).map(|_| b.fresh()).collect();
            let targets: Vec<usize> = (0..g.target_word().len()).map(|_| b.fresh()).collect();
            b.edges.push(ContentEdge {
                label: g.clone(),
                sources: sources.clone(),
                targets: targets.clone(),
            });
            Piece {
                input: sources,
                output: targets,
            }
        }
        PropExpr::Compose(x, y) => {
            let px = build(x, b);
            let py = build(y, b);
            assert_eq!(
                px.output.len(),
                py.input.len(),
                "invariant: content_of requires an arity-well-formed expression, \
                 but a Compose joins {} wires to {}",
                px.output.len(),
                py.input.len()
            );
            for (a, c) in px.output.iter().zip(py.input.iter()) {
                b.union(*a, *c);
            }
            Piece {
                input: px.input,
                output: py.output,
            }
        }
        PropExpr::Tensor(x, y) => {
            let mut px = build(x, b);
            let py = build(y, b);
            px.input.extend(py.input);
            px.output.extend(py.output);
            px
        }
    }
}

/// Densely renumber union-find classes in first-touch order.
struct Renumber<'a> {
    root_of: &'a [usize],
    dense: Vec<usize>,
    next: usize,
}

impl Renumber<'_> {
    fn get(&mut self, x: usize) -> usize {
        let root = self.root_of[x];
        if self.dense[root] == usize::MAX {
            self.dense[root] = self.next;
            self.next += 1;
        }
        self.dense[root]
    }
}

/// Compute the content `C(e)` of §4.1.
///
/// # Node colors
///
/// A node is typed by the tentacle incident to it: its **producer**'s
/// [`target_word`](PropSignature::target_word) letter, or — for a node with no
/// producer — its consumer's [`source_word`](PropSignature::source_word) letter.
/// Monogamy (BGKSZ Def 3.6) makes each of those single-valued, so the typing is a
/// function of the content and not of the writing. That leaves exactly one kind of
/// node untyped, one no generator touches at all; [`content_of_colored`] pins
/// those.
///
/// # Precondition
///
/// `expr` is **arity**-well-formed, the same precondition
/// [`nf`](super::smc_nf::nf) carries; like `nf`, this function does not return a
/// `Result`. It does *not* require word-well-formedness
/// ([`colored::check`](super::super::colored::check)): where that check would
/// fail — a `Compose` whose two sides agree on wire counts but not on colors —
/// the glued wire takes its producer's declared color, which is a content
/// invariant like any other, so nothing below degrades.
///
/// # Panics
///
/// Panics if `expr` is not arity-well-formed, i.e. some `Compose` joins a target
/// arity to a different source arity. `PropExpr`'s variants are public, so such a
/// tree is constructible by hand; terms built through
/// [`Free`](crate::prop::Free) cannot hit this.
///
/// **This is a stricter contract than the equality API above it.**
/// [`Presentation::eq_mod`](super::Presentation::eq_mod) is total on such a tree
/// today — it answers rather than panics — so any future wiring of content into
/// `eq_mod` has to decide the policy deliberately: either gate the content path
/// behind an arity check and fall back to `nf`, or give `content_of` a fallible
/// sibling returning [`CatgraphError`](catgraph::errors::CatgraphError).
#[must_use]
pub fn content_of<G: PropSignature>(expr: &PropExpr<G>) -> Content<G> {
    let mut b = Builder {
        parent: Vec::new(),
        edges: Vec::new(),
    };
    let piece = build(expr, &mut b);

    let n = b.parent.len();
    let root_of: Vec<usize> = (0..n).map(|i| b.find(i)).collect();
    let mut renumber = Renumber {
        root_of: &root_of,
        dense: vec![usize::MAX; n],
        next: 0,
    };

    // First-touch order: input anchor, then edges in construction order (sources
    // then targets), then output anchor. Presentation-dependent by design — the
    // invariant renumbering is `canonical_key`'s.
    let input: Vec<usize> = piece.input.iter().map(|&x| renumber.get(x)).collect();
    let edges: Vec<ContentEdge<G>> = b
        .edges
        .iter()
        .map(|e| ContentEdge {
            label: e.label.clone(),
            sources: e.sources.iter().map(|&x| renumber.get(x)).collect(),
            targets: e.targets.iter().map(|&x| renumber.get(x)).collect(),
        })
        .collect();
    let output: Vec<usize> = piece.output.iter().map(|&x| renumber.get(x)).collect();

    // Type each node from its incident tentacle. Both passes are single-valued by
    // monogamy, and the producer pass runs second so it wins where an
    // ill-colored `Compose` glued two disagreeing declarations.
    let mut node_colors: Vec<Option<G::Color>> = vec![None; renumber.next];
    for edge in &edges {
        for (position, &x) in edge.sources.iter().enumerate() {
            node_colors[x] = Some(edge.label.source_word()[position].clone());
        }
    }
    for edge in &edges {
        for (position, &x) in edge.targets.iter().enumerate() {
            node_colors[x] = Some(edge.label.target_word()[position].clone());
        }
    }

    debug_assert!(
        {
            // `node_colors[x].is_some()` is now exactly "x is incident to some
            // hyperedge", so this is the coverage invariant both algorithms below
            // rely on: no node escapes into neither the anchors nor the edges.
            let mut anchored = vec![false; renumber.next];
            for &x in &input {
                anchored[x] = true;
            }
            (0..renumber.next).all(|x| node_colors[x].is_some() || anchored[x])
        },
        "invariant: an edge-free wire runs from the input foot to the output \
         foot, so it is anchored"
    );

    Content {
        node_count: renumber.next,
        node_colors,
        edges,
        input,
        output,
    }
}

/// Compute the content of a **colored** morphism, typing every node — save on the
/// serde-built exception in `# Panics` below.
///
/// The expression alone leaves exactly one kind of node untyped: one no generator
/// tentacle touches. Such a node has in-degree and out-degree 0 by construction,
/// so monogamy (BGKSZ Def 3.6) forces it into `in(G) ∩ out(G)` — anchored on
/// *both* feet — and its color is therefore the source word's letter at its input
/// coordinate, which a [`ColoredExpr`] supplies. Every other node already carries
/// the color its incident tentacle declared.
///
/// Consequently the boundary *words* are recoverable from the returned content
/// (read `node_colors` along `input` / `output`), so [`content_eq`] on two values
/// of this function decides colored SMC-equality — parallelism included — rather
/// than only the positional part. Both statements are about a value whose source
/// word covers its arity, which is every value [`ColoredExpr::new`] can build;
/// the serde exception below leaves the uncovered coordinates untyped, and a word
/// read off such a content is correspondingly partial.
///
/// # Panics
///
/// Panics exactly when [`content_of`] does: the wrapped expression is not
/// arity-well-formed. That is unreachable through [`ColoredExpr::new`], which
/// runs [`colored::check`](super::super::colored::check) — but it *is* reachable
/// through the documented serde trust boundary on [`ColoredExpr`], whose
/// deserialization reconstructs the fields without re-running that check.
///
/// The other shape that boundary admits — a `source_word` shorter than the
/// expression's source arity — does not panic here: the uncovered coordinates
/// simply keep their `None`, and the result is as meaningful as the document that
/// produced it.
#[must_use]
pub fn content_of_colored<G: PropSignature>(expr: &ColoredExpr<G>) -> Content<G> {
    let mut content = content_of(expr.expr());
    let anchors = content.input.clone();
    for (position, node) in anchors.into_iter().enumerate() {
        if content.node_colors[node].is_none() {
            // `.get`, not `[]`: a serde-built `ColoredExpr` can carry a source
            // word shorter than the expression's arity.
            content.node_colors[node] = expr.source_word().get(position).cloned();
        }
    }
    debug_assert!(
        expr.source_word().len() < content.input.len()
            || content.node_colors.iter().all(Option::is_some),
        "invariant: an untyped node has degree 0 (monogamy), so it is \
         input-anchored and a source word covering the arity types it"
    );
    content
}

// ---------------------------------------------------------------- incidence

/// Per-node incidence, read once and shared by both algorithms.
///
/// Monogamy (BGKSZ Def 3.6) is what makes `producer` / `consumer` single-valued:
/// an interior node has exactly one of each, a boundary node none on its anchored
/// side. It holds by construction for every [`Content`] (Thm 3.12).
struct Profile {
    /// `(edge, tentacle position)` producing the node, if any.
    producer: Vec<Option<(usize, usize)>>,
    /// `(edge, tentacle position)` consuming the node, if any.
    consumer: Vec<Option<(usize, usize)>>,
    /// The node's coordinates in the input anchor (a node may occupy several).
    in_anchor: Vec<Vec<usize>>,
    /// The node's coordinates in the output anchor.
    out_anchor: Vec<Vec<usize>>,
}

fn profile<G: PropSignature>(c: &Content<G>) -> Profile {
    let n = c.node_count;
    let mut p = Profile {
        producer: vec![None; n],
        consumer: vec![None; n],
        in_anchor: vec![Vec::new(); n],
        out_anchor: vec![Vec::new(); n],
    };
    for (index, edge) in c.edges.iter().enumerate() {
        for (position, &x) in edge.sources.iter().enumerate() {
            debug_assert!(p.consumer[x].is_none(), "monogamy: two consumers of {x}");
            p.consumer[x] = Some((index, position));
        }
        for (position, &x) in edge.targets.iter().enumerate() {
            debug_assert!(p.producer[x].is_none(), "monogamy: two producers of {x}");
            p.producer[x] = Some((index, position));
        }
    }
    for (position, &x) in c.input.iter().enumerate() {
        p.in_anchor[x].push(position);
    }
    for (position, &x) in c.output.iter().enumerate() {
        p.out_anchor[x].push(position);
    }
    p
}

/// The incidences of a node, in the fixed order both algorithms read them.
fn incidences(p: &Profile, node: usize) -> [Option<(usize, usize)>; 2] {
    [p.consumer[node], p.producer[node]]
}

// ------------------------------------------- closed components, canonically

/// One hyperedge under a canonical numbering.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct EdgeRecord<G> {
    label: G,
    sources: Vec<usize>,
    targets: Vec<usize>,
}

/// A closed component's canonical serialization: the lexicographic minimum over
/// its seed choices, and a **complete iso invariant** of the component.
///
/// Carries no colors, and does not need to. Every node of a closed component has
/// a *producer*: a node with no producer has in-degree 0, so monogamy (BGKSZ
/// Def 3.6) puts it on the input foot, and an anchored node is by definition not
/// in a closed component. Since [`content_of`] defines a node's color to be its
/// producer's declared letter, the records below already determine every color
/// here. (The boundary-attached part does record colors, because a wire no
/// generator touches has no producer and takes the boundary word's letter
/// instead.)
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct ClosedBlock<G> {
    node_count: usize,
    edges: Vec<EdgeRecord<G>>,
}

/// A canonical numbering under construction: nodes and edges in discovery order.
struct Numbering {
    node: Vec<Option<usize>>,
    edge: Vec<Option<usize>>,
    next_node: usize,
    order: Vec<usize>,
    queue: VecDeque<usize>,
}

impl Numbering {
    fn new(nodes: usize, edges: usize) -> Self {
        Self {
            node: vec![None; nodes],
            edge: vec![None; edges],
            next_node: 0,
            order: Vec::new(),
            queue: VecDeque::new(),
        }
    }

    /// Number `node` if new, and queue it for expansion.
    fn visit_node(&mut self, node: usize) {
        if self.node[node].is_none() {
            self.node[node] = Some(self.next_node);
            self.next_node += 1;
            self.queue.push_back(node);
        }
    }

    /// Number `edge` if new, and its tentacle nodes in tentacle order.
    fn visit_edge<G: PropSignature>(&mut self, c: &Content<G>, edge: usize) {
        if self.edge[edge].is_some() {
            return;
        }
        self.edge[edge] = Some(self.order.len());
        self.order.push(edge);
        let e = &c.edges[edge];
        for &x in e.sources.iter().chain(e.targets.iter()) {
            self.visit_node(x);
        }
    }

    /// Expand the queue: every incident edge of every queued node, in the fixed
    /// `[consumer, producer]` order.
    fn run<G: PropSignature>(&mut self, c: &Content<G>, p: &Profile) {
        while let Some(x) = self.queue.pop_front() {
            for (e, _) in incidences(p, x).into_iter().flatten() {
                self.visit_edge(c, e);
            }
        }
    }

    /// The canonical index of `node`.
    fn node_id(&self, node: usize) -> usize {
        self.node[node].expect("invariant: every reached node is numbered")
    }

    fn records<G: PropSignature>(&self, c: &Content<G>) -> Vec<EdgeRecord<G>> {
        self.order
            .iter()
            .map(|&e| {
                let edge = &c.edges[e];
                EdgeRecord {
                    label: edge.label.clone(),
                    sources: edge.sources.iter().map(|&x| self.node_id(x)).collect(),
                    targets: edge.targets.iter().map(|&x| self.node_id(x)).collect(),
                }
            })
            .collect()
    }
}

/// Group the edges `matched` does not cover into their connected components —
/// the closed components, which no anchor reaches.
fn closed_components<G: PropSignature>(
    c: &Content<G>,
    p: &Profile,
    matched: &[Option<usize>],
) -> Vec<Vec<usize>> {
    let mut component = vec![usize::MAX; c.edges.len()];
    let mut components: Vec<Vec<usize>> = Vec::new();
    for start in 0..c.edges.len() {
        if matched[start].is_some() || component[start] != usize::MAX {
            continue;
        }
        let id = components.len();
        component[start] = id;
        let mut stack = vec![start];
        let mut members = Vec::new();
        while let Some(e) = stack.pop() {
            members.push(e);
            let edge = &c.edges[e];
            for &x in edge.sources.iter().chain(edge.targets.iter()) {
                for (f, _) in incidences(p, x).into_iter().flatten() {
                    if component[f] == usize::MAX {
                        component[f] = id;
                        stack.push(f);
                    }
                }
            }
        }
        members.sort_unstable();
        components.push(members);
    }
    components
}

/// The lexicographic minimum serialization of one closed component, over its
/// choices of seed edge.
///
/// Seeding fixes one edge as number 0 and the propagation numbers the rest, so
/// each seed yields one serialization; the minimum over seeds depends only on the
/// component's iso class. Two components are isomorphic iff their minima agree:
/// `⇐` because the minimum is invariant, `⇒` because equal serializations exhibit
/// the bijection.
fn minimal_block<G: PropSignature>(
    c: &Content<G>,
    p: &Profile,
    component: &[usize],
) -> ClosedBlock<G> {
    debug_assert!(
        component
            .iter()
            .flat_map(|&e| c.edges[e].sources.iter().chain(c.edges[e].targets.iter()))
            .all(|&x| match p.producer[x] {
                Some((e, position)) => {
                    let word = c.edges[e].label.target_word();
                    c.node_colors[x].as_ref() == Some(&word[position])
                }
                None => false,
            }),
        "invariant: every closed-component node has a producer (a producerless \
         node is input-anchored, hence not closed) and carries that producer's \
         declared letter — which is what lets ClosedBlock drop colors"
    );
    component
        .iter()
        .map(|&seed| {
            let mut numbering = Numbering::new(c.node_count, c.edges.len());
            numbering.visit_edge(c, seed);
            numbering.run(c, p);
            ClosedBlock {
                node_count: numbering.next_node,
                edges: numbering.records(c),
            }
        })
        .min()
        .expect("invariant: a closed component has at least one edge")
}

/// Every closed component of `c`, canonically serialized and sorted — the
/// complete invariant of the closed part, shared by [`content_eq`] and
/// [`canonical_key`].
fn closed_blocks<G: PropSignature>(
    c: &Content<G>,
    p: &Profile,
    matched: &[Option<usize>],
) -> Vec<ClosedBlock<G>> {
    let mut blocks: Vec<ClosedBlock<G>> = closed_components(c, p, matched)
        .iter()
        .map(|component| minimal_block(c, p, component))
        .collect();
    blocks.sort_unstable();
    blocks
}

// ---------------------------------------------------------------- equality

/// A partial iso being extended, together with the two profiles it reads.
struct Matcher<'a, G: PropSignature> {
    a: &'a Content<G>,
    b: &'a Content<G>,
    pa: &'a Profile,
    pb: &'a Profile,
    node_map: Vec<Option<usize>>,
    node_rev: Vec<Option<usize>>,
    edge_map: Vec<Option<usize>>,
    edge_rev: Vec<Option<usize>>,
}

impl<G: PropSignature> Matcher<'_, G> {
    /// Force `x ↦ y`, queueing it when new. `false` on any conflict.
    fn force_node(&mut self, x: usize, y: usize, queue: &mut Vec<(usize, usize)>) -> bool {
        if let Some(mapped) = self.node_map[x] {
            return mapped == y;
        }
        if self.node_rev[y].is_some() {
            // `y` is already the image of some other node.
            return false;
        }
        if self.a.node_colors[x] != self.b.node_colors[y] {
            return false;
        }
        self.node_map[x] = Some(y);
        self.node_rev[y] = Some(x);
        queue.push((x, y));
        true
    }

    /// Force `e ↦ f` and, with it, every tentacle node pair. `false` on conflict.
    fn force_edge(&mut self, e: usize, f: usize, queue: &mut Vec<(usize, usize)>) -> bool {
        if let Some(mapped) = self.edge_map[e] {
            return mapped == f;
        }
        if self.edge_rev[f].is_some() {
            return false;
        }
        let (ea, fb) = (&self.a.edges[e], &self.b.edges[f]);
        if ea.label != fb.label
            || ea.sources.len() != fb.sources.len()
            || ea.targets.len() != fb.targets.len()
        {
            return false;
        }
        self.edge_map[e] = Some(f);
        self.edge_rev[f] = Some(e);
        for (&x, &y) in ea.sources.iter().zip(fb.sources.iter()) {
            if !self.force_node(x, y, queue) {
                return false;
            }
        }
        for (&x, &y) in ea.targets.iter().zip(fb.targets.iter()) {
            if !self.force_node(x, y, queue) {
                return false;
            }
        }
        true
    }

    /// Close the queued node pairs under their forced consequences.
    fn propagate(&mut self, queue: &mut Vec<(usize, usize)>) -> bool {
        while let Some((x, y)) = queue.pop() {
            // Boundary coordinates are content invariants, so the two nodes must
            // occupy the same ones.
            if self.pa.in_anchor[x] != self.pb.in_anchor[y]
                || self.pa.out_anchor[x] != self.pb.out_anchor[y]
            {
                return false;
            }
            for (left, right) in incidences(self.pa, x)
                .into_iter()
                .zip(incidences(self.pb, y))
            {
                match (left, right) {
                    (None, None) => {}
                    (Some((e, pe)), Some((f, pf))) => {
                        if pe != pf || !self.force_edge(e, f, queue) {
                            return false;
                        }
                    }
                    _ => return false,
                }
            }
        }
        true
    }
}

/// Decide `C(a) = C(b)`: cospan isomorphism **under both feet**.
///
/// By Lemma 4.1 (§4.2) this decides SMC-equality of the two expressions the
/// contents came from — exactly, on every diagram, fragment or not. It is *not*
/// equality modulo user equations; see the module docs on layering.
///
/// # Exactness
///
/// A cospan iso under both feet splits into two independent halves, because a
/// closed component contains no anchored node and so is coupled to nothing else:
/// an iso of the boundary-attached parts, and a bijection of the closed
/// components with a component iso at each. The procedure decides each half
/// outright, and neither half searches.
///
/// 1. **Anchored forcing.** The feet are fixed, so the anchors force node images
///    pointwise: `a.input[i] ↦ b.input[i]`, likewise on the output. No choice.
/// 2. **Monogamy propagation.** A forced node pair forces its producer and its
///    consumer edge pair — each is unique by monogamy (BGKSZ Def 3.6) — and a
///    forced edge pair forces every tentacle pair, since tentacle positions are
///    invariants. Iterating reaches every node and edge of every
///    boundary-attached component, still with no choice. A conflict at any step
///    (label, arity, color, anchor coordinates, or an already-taken image) refutes
///    the iso outright, because each step was forced. If nothing conflicts, the
///    partial map built is the unique candidate iso of the anchored halves.
/// 3. **Closed components, by invariant rather than by search.** Serializing a
///    connected closed component from each of its edges in turn and keeping the
///    lexicographic minimum (the private `minimal_block`) is a *complete* iso
///    invariant of it: equal serializations exhibit a label- and
///    tentacle-preserving bijection, and isomorphic components minimize to the
///    same one. So the required bijection exists iff the two sorted multisets
///    agree, which is a comparison, not a match. Colors need no separate check
///    here: every closed-component node has a producer, and its color is by
///    definition that producer's declared letter.
///
/// There is no comparator, no order on components, and no writing-dependent
/// coordinate anywhere in the above — the whole decision reads labels, tentacle
/// positions, anchor coordinates and colors, all of which are content invariants.
///
/// # Cost
///
/// Polynomial, with no backtracking anywhere. Take a content with `n` nodes and
/// `e` hyperedges. Steps 1–2 are linear in it: each node and edge is forced at
/// most once. Step 3 serializes each closed component `K` once per choice of
/// seed, and each of those passes allocates a numbering sized to the *whole*
/// content rather than to `K`, so a component costs `O(|E_K| · (n + e))` — over
/// all components, `O(e · (n + e))`, plus the block sort. With nothing closed the
/// call is `O(n + e)`.
///
/// Sizing the per-seed numbering to the component would drop that to
/// `O(|E_K| · (|V_K| + |E_K|))`. It is not worth doing at present content sizes
/// (tens of nodes), and is left for whenever a measurement asks for it.
///
/// # Comparing like with like
///
/// Colors are compared, so a content from [`content_of`] and one from
/// [`content_of_colored`] can differ purely in typing (`None` against `Some(c)`)
/// on a wire no generator touches. Build both sides through the same entry point.
#[must_use]
pub fn content_eq<G: PropSignature>(a: &Content<G>, b: &Content<G>) -> bool {
    if a.node_count != b.node_count
        || a.edges.len() != b.edges.len()
        || a.input.len() != b.input.len()
        || a.output.len() != b.output.len()
    {
        return false;
    }
    let (pa, pb) = (profile(a), profile(b));
    let mut m = Matcher {
        a,
        b,
        pa: &pa,
        pb: &pb,
        node_map: vec![None; a.node_count],
        node_rev: vec![None; b.node_count],
        edge_map: vec![None; a.edges.len()],
        edge_rev: vec![None; b.edges.len()],
    };
    let mut queue = Vec::new();
    for (&x, &y) in a.input.iter().zip(b.input.iter()) {
        if !m.force_node(x, y, &mut queue) {
            return false;
        }
    }
    for (&x, &y) in a.output.iter().zip(b.output.iter()) {
        if !m.force_node(x, y, &mut queue) {
            return false;
        }
    }
    if !m.propagate(&mut queue) {
        return false;
    }
    // Propagation is symmetric, so on success the matched edges of each side are
    // exactly its anchor-reachable ones; what is left over is closed.
    closed_blocks(a, &pa, &m.edge_map) == closed_blocks(b, &pb, &m.edge_rev)
}

// ---------------------------------------------------------------- canonical key

/// A hashable canonical form of a [`Content`], with
/// `canonical_key(a) == canonical_key(b)` **iff** `content_eq(&a, &b)`.
///
/// Use it to key contents in a `HashMap` / `HashSet` — the intended consumer is
/// the congruence-closure engine's structural buckets.
///
/// # Not `Ord`
///
/// [`PropSignature::Color`] is not required to be `Ord`, and the key records
/// colors, so there is no total order to derive. `Eq + Hash` is the whole
/// contract.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ContentKey<G: PropSignature> {
    /// Nodes of the boundary-attached part, canonically numbered.
    node_count: usize,
    /// Colors of those nodes, in canonical order.
    colors: Vec<Option<G::Color>>,
    /// Their hyperedges, in canonical discovery order.
    edges: Vec<EdgeRecord<G>>,
    /// Input anchor, as canonical node indices.
    input: Vec<usize>,
    /// Output anchor, as canonical node indices.
    output: Vec<usize>,
    /// The closed components, each canonically serialized, sorted.
    closed: Vec<ClosedBlock<G>>,
}

/// Compute the canonical key of a content.
///
/// # Why it is canonical
///
/// The boundary-attached part is numbered by the same anchored propagation
/// [`content_eq`] uses, made into a numbering: nodes are discovered from the
/// input anchor in coordinate order, then the output anchor, then breadth-first
/// through incident edges taken in the fixed `[consumer, producer]` order, each
/// edge numbering its tentacles in tentacle order. Every ingredient of that walk
/// — anchor coordinates, tentacle positions, labels — is a content invariant, so
/// isomorphic contents produce identical numberings. Conversely two equal
/// numberings *are* an iso under both feet, since the anchors are recorded
/// coordinate-wise.
///
/// The closed part is the same complete invariant [`content_eq`] compares — each
/// component's minimum-over-seeds serialization, sorted — which turns the
/// unordered family of closed components into a canonical sequence.
///
/// # Cost
///
/// The same bound as [`content_eq`]: linear in the content for the
/// boundary-attached part, `O(|E_K| · (n + e))` per closed component `K` — see
/// that function on why the per-seed pass is sized to the content and not to `K`
/// — plus the block sort.
#[must_use]
pub fn canonical_key<G: PropSignature>(c: &Content<G>) -> ContentKey<G> {
    let p = profile(c);

    // Boundary-attached part: anchors seed the numbering, in coordinate order.
    let mut anchored = Numbering::new(c.node_count, c.edges.len());
    for &x in c.input.iter().chain(c.output.iter()) {
        anchored.visit_node(x);
    }
    anchored.run(c, &p);

    let closed = closed_blocks(c, &p, &anchored.edge);

    let mut colors: Vec<Option<G::Color>> = vec![None; anchored.next_node];
    for (id, color) in anchored.node.iter().zip(c.node_colors.iter()) {
        if let Some(id) = id {
            colors[*id].clone_from(color);
        }
    }

    ContentKey {
        node_count: anchored.next_node,
        colors,
        edges: anchored.records(c),
        input: c.input.iter().map(|&x| anchored.node_id(x)).collect(),
        output: c.output.iter().map(|&x| anchored.node_id(x)).collect(),
        closed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prop::Free;
    use crate::rig::BoolRig;
    use crate::sfg::SfgGenerator;

    type Sfg = SfgGenerator<BoolRig>;
    type E = PropExpr<Sfg>;

    fn prim(g: Sfg) -> E {
        Free::<Sfg>::generator(g)
    }

    #[test]
    fn identity_is_a_discrete_cospan() {
        let c = content_of::<Sfg>(&PropExpr::Identity(3));
        assert_eq!(c.node_count(), 3);
        assert!(c.edges().is_empty());
        assert_eq!(c.input(), c.output());
    }

    #[test]
    fn braid_permutes_the_anchor_but_not_the_nodes() {
        let c = content_of::<Sfg>(&PropExpr::Braid(1, 2));
        assert_eq!(c.node_count(), 3);
        assert_eq!(c.input(), &[0, 1, 2]);
        assert_eq!(c.output(), &[1, 2, 0]);
    }

    #[test]
    fn compose_glues_over_the_shared_foot() {
        // Copy ; Add : 1 → 1 has two hyperedges sharing both middle wires.
        let e = Free::compose(prim(SfgGenerator::Copy), prim(SfgGenerator::Add))
            .expect("Copy : 1 → 2 ; Add : 2 → 1");
        let c = content_of(&e);
        assert_eq!(c.edges().len(), 2);
        assert_eq!(c.node_count(), 4); // in, two middles, out
        assert_eq!(c.edges()[0].targets, c.edges()[1].sources);
    }

    #[test]
    fn generator_tentacles_type_their_nodes() {
        let c = content_of(&prim(SfgGenerator::Copy));
        assert!(c.node_colors().iter().all(Option::is_some));
    }

    #[test]
    fn an_untouched_wire_has_no_color_until_a_word_supplies_one() {
        let c = content_of::<Sfg>(&PropExpr::Identity(1));
        assert_eq!(c.node_colors(), &[None]);

        let colored = ColoredExpr::<Sfg>::new(vec![()], PropExpr::Identity(1))
            .expect("id₁ is well-formed at •");
        let c = content_of_colored(&colored);
        assert_eq!(c.node_colors(), &[Some(())]);
    }

    #[test]
    #[should_panic(expected = "arity-well-formed")]
    fn arity_mismatched_compose_is_rejected_loudly() {
        // Hand-built, bypassing `Free::compose`'s check.
        let bad = PropExpr::<Sfg>::Compose(
            Box::new(PropExpr::Identity(1)),
            Box::new(PropExpr::Identity(2)),
        );
        let _ = content_of(&bad);
    }

    #[test]
    fn closed_component_key_is_seed_independent() {
        // (Zero ; Discard) ⊗ (Zero ; Discard): two identical closed components,
        // so the sorted-block key must not depend on which is seeded first.
        let loop_ = Free::compose(prim(SfgGenerator::Zero), prim(SfgGenerator::Discard))
            .expect("Zero : 0 → 1 ; Discard : 1 → 0");
        let two = Free::tensor(loop_.clone(), loop_);
        let c = content_of(&two);
        let key = canonical_key(&c);
        assert_eq!(key.closed.len(), 2);
        assert_eq!(key.closed[0], key.closed[1]);
        assert!(content_eq(&c, &c));
    }
}
