//! Abstract content of a free-prop expression, and equality on it.
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
//! invariant. [`Content`] is that value, [`content_eq`] is that equality, and
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
//!   part by comparing a complete invariant.
//! - Together they give **Lemma 4.1** (§4.2): `e =_SMC e′` **iff**
//!   `C(e) = C(e′)`, stated color-generically over an arbitrary `Λ`.
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
//! it is word-aware for any `Λ`; [`content_of_colored`] additionally pins the
//! colors an expression alone cannot determine. With `Color = ()` the comparison
//! of *letters* degenerates to a no-op — there is only one letter — but the
//! distinction between a typed and an untyped node does not: `Some(())` and
//! `None` still differ, so the like-with-like caveat on [`content_eq`] binds
//! monochromatic callers exactly as much as colored ones.

use std::collections::VecDeque;

use catgraph::errors::CatgraphError;

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
/// Deliberately not derived: structural equality of the fields is
/// *representation* equality, which is finer than content equality. Content
/// equality is [`content_eq`]; the hashable form is [`canonical_key`].
///
/// # Construction invariant
///
/// The fields are private and the accessors read-only. Every value's *underlying
/// uncolored* cospan is monogamous directed acyclic — the image characterization
/// of BGKSZ Thm 3.12 — and every node is either incident to a hyperedge or
/// anchored on both feet (a wire no generator touches runs from the input foot
/// straight to the output foot). The tentacle typing is additionally
/// *type-respecting* exactly when the expression was word-well-formed: over a
/// nontrivial `Λ`, `content_of` accepts an ill-colored `Compose` and returns a
/// cospan whose typing no Λ-typed expression realizes.
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

// ---------------------------------------------------------------- domain check

/// Whether every `Compose` in `expr` joins a target arity to a matching source
/// arity **and** no `Braid` or `Tensor` width overflows `usize` — exactly
/// [`content_of`]'s precondition, and so a test for membership in its domain.
/// Terms built through [`Free`](crate::prop::Free) always pass; `PropExpr`'s
/// variants are public, so a hand-built or deserialized tree may not.
/// [`PropExpr::arities_fit`] is the overflow clause on its own, for callers that
/// need to separate the two failure modes.
///
/// `O(n)` for the overflow clause; the composability clause re-reads each node's
/// own arities, which are `O(height)` for a `Compose` spine and proportional to
/// the subterm for a `Tensor`.
#[must_use]
pub fn is_arity_well_formed<G: PropSignature>(expr: &PropExpr<G>) -> bool {
    // Order matters: once every width fits, `source`/`target` are exact and the
    // composability test below compares real arities rather than sentinels.
    expr.arities_fit() && composes(expr)
}

/// The composability half of [`is_arity_well_formed`]: every `Compose` joins a
/// target arity to a matching source arity. Only meaningful once
/// [`PropExpr::arities_fit`] holds — two independently saturated arities compare
/// equal, so on an overflowing tree this can report a match that is not one.
fn composes<G: PropSignature>(expr: &PropExpr<G>) -> bool {
    match expr {
        PropExpr::Identity(_) | PropExpr::Braid(_, _) | PropExpr::Generator(_) => true,
        PropExpr::Compose(f, g) => f.target() == g.source() && composes(f) && composes(g),
        PropExpr::Tensor(f, g) => composes(f) && composes(g),
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
            // The width sizes two vectors, so a saturating `usize::MAX` would be
            // a `usize::MAX`-element `collect` rather than a rejectable sentinel.
            let width = m
                .checked_add(*n)
                .expect("invariant: content_of's domain excludes a braid width that overflows usize (is_arity_well_formed)");
            let nodes: Vec<usize> = (0..width).map(|_| b.fresh()).collect();
            // σ_{m,n} : u ⊗ v → v ⊗ u — the anchor is the block swap.
            let mut output = Vec::with_capacity(width);
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
/// `expr` is **arity**-well-formed. Word-well-formedness
/// ([`colored::check`](super::super::colored::check)) is *not* required: where
/// that check would fail — a `Compose` whose two sides agree on wire counts but
/// not on colors — the glued wire takes its producer's declared color, which is a
/// content invariant like any other.
///
/// # Panics
///
/// Panics if `expr` is not arity-well-formed: some `Compose` joins a target arity
/// to a different source arity, **or** some `Braid` / `Tensor` width sums past
/// `usize::MAX`. Terms built through [`Free`](crate::prop::Free) cannot hit this.
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
    // then targets), then output anchor — the invariant renumbering is
    // `canonical_key`'s.
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

    // Type each node from its incident tentacle. The producer pass runs second so
    // it wins where an ill-colored `Compose` glued two disagreeing declarations.
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
            // hyperedge", so this checks the coverage invariant.
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
/// tentacle touches. Such a node has in-degree and out-degree 0, so monogamy
/// (BGKSZ Def 3.6) forces it into `in(G) ∩ out(G)` — anchored on *both* feet —
/// and its color is the source word's letter at its input coordinate, which a
/// [`ColoredExpr`] supplies. The boundary *words* are therefore recoverable from
/// the returned content (read `node_colors` along `input` / `output`), so
/// [`content_eq`] on two values of this function decides colored SMC-equality,
/// parallelism included. That holds of a value whose source word covers its
/// arity, which is every value [`ColoredExpr::new`] can build; a serde-built
/// shorter word leaves the uncovered coordinates untyped.
///
/// # Panics
///
/// Panics exactly when [`content_of`] does: the wrapped expression is not
/// arity-well-formed. Unreachable through [`ColoredExpr::new`], which runs
/// [`colored::check`](super::super::colored::check), but reachable through the
/// serde trust boundary on [`ColoredExpr`]. A `source_word` shorter than the
/// expression's source arity does *not* panic here.
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

// ------------------------------------------------------- construction by parts

/// Rebuild a [`Content`] from raw parts, **validating** the image
/// characterization of BGKSZ Thm 3.12 before handing one back.
///
/// # What is checked
///
/// 1. `node_colors` has one entry per node, and every index in `edges`, `input`
///    and `output` is in range.
/// 2. Each edge's tentacle counts agree with its label's declared words, **and
///    each tentacle's node carries the color that word declares at that
///    position**. A node no tentacle touches is unconstrained here: its color is
///    boundary data, not a derived quantity.
/// 3. **Monogamy** (BGKSZ Def 3.6), in full: no node has two producers or two
///    consumers; the **anchor legs are mono** — no node occupies two `input`
///    coordinates, and none two `output` coordinates (occupying one of each is
///    legal and is exactly `id₁`); and a node is producerless *iff* it is
///    input-anchored, consumerless *iff* it is output-anchored.
/// 4. **Acyclicity** (BGKSZ Def 3.9), by a Kahn sweep over the hyperedges.
///
/// (3) and (4) together are Thm 3.12's image characterization, so a value that
/// passes is one `⟦·⟧` could have produced. Colors beyond the tentacle-declared
/// ones are carried through as given.
///
/// # Errors
///
/// [`CatgraphError::Presentation`], naming the first violated clause and the
/// node or edge that violates it.
pub(super) fn from_parts<G: PropSignature>(
    node_count: usize,
    node_colors: Vec<Option<G::Color>>,
    edges: Vec<ContentEdge<G>>,
    input: Vec<usize>,
    output: Vec<usize>,
) -> Result<Content<G>, CatgraphError> {
    fn reject<T>(message: String) -> Result<T, CatgraphError> {
        Err(CatgraphError::Presentation { message })
    }

    if node_colors.len() != node_count {
        return reject(format!(
            "content parts: {} colors for {node_count} nodes",
            node_colors.len()
        ));
    }

    let mut producer: Vec<Option<usize>> = vec![None; node_count];
    let mut consumer: Vec<Option<usize>> = vec![None; node_count];
    for (index, edge) in edges.iter().enumerate() {
        if edge.sources.len() != edge.label.source_word().len()
            || edge.targets.len() != edge.label.target_word().len()
        {
            return reject(format!(
                "content parts: edge {index} carries {}/{} tentacles but its label declares {}/{}",
                edge.sources.len(),
                edge.targets.len(),
                edge.label.source_word().len(),
                edge.label.target_word().len()
            ));
        }
        let (source_word, target_word) = (edge.label.source_word(), edge.label.target_word());
        for (position, &x) in edge.sources.iter().enumerate() {
            if x >= node_count {
                return reject(format!("content parts: edge {index} sources node {x}"));
            }
            if consumer[x].is_some() {
                return reject(format!(
                    "content parts: monogamy — node {x} has two consumers"
                ));
            }
            // A caller-supplied color that disagrees with the label's word is a
            // content no `⟦·⟧` could have produced.
            if node_colors[x].as_ref() != Some(&source_word[position]) {
                return reject(format!(
                    "content parts: node {x} carries color {:?} but edge {index} declares {:?} at \
                     source position {position}",
                    node_colors[x], source_word[position]
                ));
            }
            consumer[x] = Some(index);
        }
        for (position, &x) in edge.targets.iter().enumerate() {
            if x >= node_count {
                return reject(format!("content parts: edge {index} targets node {x}"));
            }
            if producer[x].is_some() {
                return reject(format!(
                    "content parts: monogamy — node {x} has two producers"
                ));
            }
            if node_colors[x].as_ref() != Some(&target_word[position]) {
                return reject(format!(
                    "content parts: node {x} carries color {:?} but edge {index} declares {:?} at \
                     target position {position}",
                    node_colors[x], target_word[position]
                ));
            }
            producer[x] = Some(index);
        }
    }

    // Each anchor leg is mono (BGKSZ Def 3.6), so neither may repeat a node;
    // occupying one input *and* one output coordinate stays legal (`id₁`), so the
    // two sweeps are separate.
    let mut in_anchored = vec![false; node_count];
    for &x in &input {
        if x >= node_count {
            return reject(format!("content parts: input anchors node {x}"));
        }
        if in_anchored[x] {
            return reject(format!(
                "content parts: monogamy — node {x} occupies two input coordinates, so the input \
                 anchor is not mono"
            ));
        }
        in_anchored[x] = true;
    }
    let mut out_anchored = vec![false; node_count];
    for &x in &output {
        if x >= node_count {
            return reject(format!("content parts: output anchors node {x}"));
        }
        if out_anchored[x] {
            return reject(format!(
                "content parts: monogamy — node {x} occupies two output coordinates, so the \
                 output anchor is not mono"
            ));
        }
        out_anchored[x] = true;
    }

    for x in 0..node_count {
        if producer[x].is_none() != in_anchored[x] {
            return reject(format!(
                "content parts: monogamy — node {x} is producerless={} but input-anchored={}",
                producer[x].is_none(),
                in_anchored[x]
            ));
        }
        if consumer[x].is_none() != out_anchored[x] {
            return reject(format!(
                "content parts: monogamy — node {x} is consumerless={} but output-anchored={}",
                consumer[x].is_none(),
                out_anchored[x]
            ));
        }
    }

    // Kahn over hyperedges: `e → f` when a target of `e` is a source of `f`.
    let mut waiting: Vec<usize> = edges
        .iter()
        .map(|e| e.sources.iter().filter(|&&x| producer[x].is_some()).count())
        .collect();
    let mut queue: VecDeque<usize> = (0..edges.len()).filter(|&e| waiting[e] == 0).collect();
    let mut settled = 0usize;
    while let Some(e) = queue.pop_front() {
        settled += 1;
        for &x in &edges[e].targets {
            if let Some(f) = consumer[x] {
                waiting[f] -= 1;
                if waiting[f] == 0 {
                    queue.push_back(f);
                }
            }
        }
    }
    if settled != edges.len() {
        return reject(format!(
            "content parts: acyclicity — {} of {} hyperedges lie on a directed cycle",
            edges.len() - settled,
            edges.len()
        ));
    }

    Ok(Content {
        node_count,
        node_colors,
        edges,
        input,
        output,
    })
}

// ---------------------------------------------------------------- incidence

/// Per-node incidence, read once and shared by both algorithms. Monogamy (BGKSZ
/// Def 3.6) makes `producer` / `consumer` single-valued: an interior node has
/// exactly one of each, a boundary node none on its anchored side.
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct EdgeRecord<G> {
    label: G,
    sources: Vec<usize>,
    targets: Vec<usize>,
}

/// A closed component's canonical serialization: the lexicographic minimum over
/// its seed choices, and a **complete iso invariant** of the component.
///
/// Carries no colors, and does not need to: every node of a closed component has
/// a producer — a producerless node has in-degree 0, so monogamy (BGKSZ Def 3.6)
/// puts it on the input foot, and an anchored node is not in a closed component —
/// and [`content_of`] defines a node's color to be its producer's declared letter,
/// so the records below already determine every color here.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
/// choices of seed edge. Seeding fixes one edge as number 0 and the propagation
/// numbers the rest, so each seed yields one serialization and the minimum over
/// seeds depends only on the component's iso class; two components are isomorphic
/// iff their minima agree.
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
/// A cospan iso under both feet splits into two independent halves — an iso of
/// the boundary-attached parts, and a bijection of the closed components with a
/// component iso at each — and neither half searches.
///
/// 1. **Anchored forcing.** The feet are fixed, so the anchors force node images
///    pointwise: `a.input[i] ↦ b.input[i]`, likewise on the output.
/// 2. **Monogamy propagation.** A forced node pair forces its producer and its
///    consumer edge pair — each unique by monogamy (BGKSZ Def 3.6) — and a forced
///    edge pair forces every tentacle pair, since tentacle positions are
///    invariants. A conflict at any step (label, arity, color, anchor
///    coordinates, or an already-taken image) refutes the iso outright; if
///    nothing conflicts, the partial map built is the unique candidate iso of the
///    anchored halves.
/// 3. **Closed components, by invariant rather than by search.** The private
///    `minimal_block` is a *complete* iso invariant of a closed component, so the
///    required bijection exists iff the two sorted multisets agree. Colors need
///    no separate check: every closed-component node has a producer, and its
///    color is that producer's declared letter.
///
/// # Cost
///
/// Polynomial, with no backtracking. For a content with `n` nodes and `e`
/// hyperedges, steps 1–2 are linear — each node and edge is forced at most once.
/// Step 3 serializes each closed component `K` once per seed, each pass
/// allocating a numbering sized to the whole content, so a component costs
/// `O(|E_K| · (n + e))` — over all components `O(e · (n + e))`, plus the block
/// sort. With nothing closed the call is `O(n + e)`.
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

/// `Vec<Option<C>>` in a shape a self-describing format can read back.
///
/// `Option<C>` is **not** round-trippable through JSON when `C` itself
/// serializes as `null`, which is the monochromatic case `Color = ()`:
/// `Some(())` and `None` both write `null`, and `null` reads back as `None`. The
/// slot is therefore tagged explicitly instead of leaning on `Option`'s
/// representation.
#[cfg(feature = "serde")]
mod color_slots {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// A node's color slot, tagged so `Untyped` and a null-serializing letter
    /// cannot collide.
    #[derive(Serialize, Deserialize)]
    enum Slot<C> {
        /// A wire no generator touches: no producer, hence no declared letter
        /// (see [`content_of`](super::content_of)).
        Untyped,
        /// The letter its producer declares.
        Typed(C),
    }

    pub(super) fn serialize<C, S>(colors: &[Option<C>], serializer: S) -> Result<S::Ok, S::Error>
    where
        C: Serialize,
        S: Serializer,
    {
        serializer.collect_seq(colors.iter().map(|slot| match slot {
            Some(color) => Slot::Typed(color),
            None => Slot::Untyped,
        }))
    }

    pub(super) fn deserialize<'de, C, D>(deserializer: D) -> Result<Vec<Option<C>>, D::Error>
    where
        C: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        Ok(Vec::<Slot<C>>::deserialize(deserializer)?
            .into_iter()
            .map(|slot| match slot {
                Slot::Untyped => None,
                Slot::Typed(color) => Some(color),
            })
            .collect())
    }
}

/// A hashable canonical form of a [`Content`], with
/// `canonical_key(a) == canonical_key(b)` **iff** `content_eq(&a, &b)`. Use it to
/// key contents in a `HashMap` / `HashSet`.
///
/// # Not `Ord`
///
/// [`PropSignature::Color`] is not required to be `Ord`, and the key records
/// colors, so there is no total order to derive. `Eq + Hash` is the whole
/// contract; a store that needs a sort order has to impose one itself.
///
/// # Serde (feature `serde`)
///
/// `Serialize` / `Deserialize` round-trip the canonical form. **It is a key, not
/// a term.** Deserialization reconstructs the fields directly and does not re-run
/// [`canonical_key`], so a hand-crafted document can be a key of no content at
/// all — a colors vector of the wrong length for its `node_count`, an anchor
/// naming a node past the end, closed blocks in an order no seed minimization
/// produces. Such a value is still a usable `HashMap` key, but it is not
/// `canonical_key(c)` for any `c`, so the `iff` above does not hold of it.
/// Round-tripping a value this crate produced is always sound, and is the
/// contract.
///
/// The serialized shape is the private field layout, so it is **not** a stable
/// wire format across crate versions and embeds no version tag. The per-node
/// color slots are written as an explicit `Untyped` / `Typed(c)` tag rather than
/// as `Option<C>`, which is lossy in JSON for a color that serializes as `null`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "G: serde::Serialize, G::Color: serde::Serialize",
        deserialize = "G: serde::Deserialize<'de>, G::Color: serde::Deserialize<'de>"
    ))
)]
pub struct ContentKey<G: PropSignature> {
    /// Nodes of the boundary-attached part, canonically numbered.
    node_count: usize,
    /// Colors of those nodes, in canonical order.
    #[cfg_attr(feature = "serde", serde(with = "color_slots"))]
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
/// The boundary-attached part is numbered by the anchored propagation
/// [`content_eq`] uses: nodes are discovered from the input anchor in coordinate
/// order, then the output anchor, then breadth-first through incident edges taken
/// in the fixed `[consumer, producer]` order, each edge numbering its tentacles in
/// tentacle order. Every ingredient of that walk is a content invariant, so
/// isomorphic contents produce identical numberings, and two equal numberings
/// *are* an iso under both feet. The closed part is the same complete invariant
/// [`content_eq`] compares — each component's minimum-over-seeds serialization,
/// sorted.
///
/// # Cost
///
/// The same bound as [`content_eq`]: linear in the content for the
/// boundary-attached part, `O(|E_K| · (n + e))` per closed component `K`, plus
/// the block sort.
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

// ------------------------------------------------- canonical representative

/// Rebuild a content from its canonical key: the anchored part under the key's
/// numbering, then one closed component per block, node-disjointly offset in the
/// key's sorted block order. Closed nodes take their producer's declared letter.
fn content_of_key<G: PropSignature>(key: &ContentKey<G>) -> Content<G> {
    let mut node_count = key.node_count;
    let mut node_colors = key.colors.clone();
    let mut edges: Vec<ContentEdge<G>> = key
        .edges
        .iter()
        .map(|record| ContentEdge {
            label: record.label.clone(),
            sources: record.sources.clone(),
            targets: record.targets.clone(),
        })
        .collect();

    for block in &key.closed {
        let base = node_count;
        node_count += block.node_count;
        node_colors.resize(node_count, None);
        for record in &block.edges {
            let edge = ContentEdge {
                label: record.label.clone(),
                sources: record.sources.iter().map(|&x| x + base).collect(),
                targets: record.targets.iter().map(|&x| x + base).collect(),
            };
            for (position, &x) in edge.targets.iter().enumerate() {
                node_colors[x] = Some(edge.label.target_word()[position].clone());
            }
            edges.push(edge);
        }
    }

    Content {
        node_count,
        node_colors,
        edges,
        input: key.input.clone(),
        output: key.output.clone(),
    }
}

/// The canonical representative of `c`'s iso class: the same cospan, relabeled
/// onto [`canonical_key`]'s numbering.
///
/// It factors through [`canonical_key`] — literally `content_of_key ∘
/// canonical_key` — and that key is a *complete* invariant, so `content_eq(a, b)`
/// implies the two representatives are equal field for field. The result is
/// isomorphic to `c` under both feet, so monogamy, acyclicity and the boundary
/// conditions of BGKSZ Thm 3.12 transport across unchanged. It does *not*
/// preserve the node indices.
pub(super) fn canonical_content<G: PropSignature>(c: &Content<G>) -> Content<G> {
    let canonical = content_of_key(&canonical_key(c));
    debug_assert!(
        content_eq(&canonical, c),
        "invariant: relabeling by the canonical key is a cospan iso under both feet"
    );
    canonical
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

    /// Hand-built violations of each clause of the BGKSZ Thm 3.12 image
    /// characterization; every one is expected to be rejected.
    #[test]
    fn from_parts_rejects_every_way_the_image_characterization_can_fail() {
        fn edge(label: Sfg, sources: Vec<usize>, targets: Vec<usize>) -> ContentEdge<Sfg> {
            ContentEdge {
                label,
                sources,
                targets,
            }
        }
        fn rejects(
            needle: &str,
            node_count: usize,
            node_colors: Vec<Option<()>>,
            edges: Vec<ContentEdge<Sfg>>,
            input: Vec<usize>,
            output: Vec<usize>,
        ) {
            match from_parts(node_count, node_colors, edges, input, output) {
                Err(CatgraphError::Presentation { message }) => {
                    assert!(message.contains(needle), "want {needle:?}, got: {message}");
                }
                other => panic!("expected a rejection mentioning {needle:?}, got {other:?}"),
            }
        }

        // Clause 1 — one color per node, and every index in range.
        rejects("colors for", 1, vec![], vec![], vec![0], vec![0]);
        rejects(
            "sources node 5",
            1,
            vec![Some(())],
            vec![edge(SfgGenerator::Discard, vec![5], vec![])],
            vec![],
            vec![],
        );

        // Clause 2 — tentacle counts against the label's declared words.
        rejects(
            "tentacles",
            2,
            vec![Some(()); 2],
            vec![edge(SfgGenerator::Copy, vec![0], vec![1])], // Copy is 1 → 2
            vec![0],
            vec![1],
        );
        // …and the colors those words declare. `Discard : 1 → 0` types its
        // source node `Some(())`, so an untyped one is a content `⟦·⟧` never
        // produces.
        rejects(
            "declares",
            1,
            vec![None],
            vec![edge(SfgGenerator::Discard, vec![0], vec![])],
            vec![0],
            vec![],
        );

        // Clause 3 — monogamy, Def 3.6 in full.
        rejects(
            "two producers",
            1,
            vec![Some(())],
            vec![
                edge(SfgGenerator::Zero, vec![], vec![0]),
                edge(SfgGenerator::Zero, vec![], vec![0]),
            ],
            vec![],
            vec![0],
        );
        rejects(
            "two consumers",
            1,
            vec![Some(())],
            vec![
                edge(SfgGenerator::Discard, vec![0], vec![]),
                edge(SfgGenerator::Discard, vec![0], vec![]),
            ],
            vec![0],
            vec![],
        );
        // The anchor legs are mono: neither may repeat a node. This is the
        // shape a repeated boundary coordinate would smuggle in.
        rejects(
            "two input coordinates",
            1,
            vec![None],
            vec![],
            vec![0, 0],
            vec![0],
        );
        rejects(
            "two output coordinates",
            1,
            vec![None],
            vec![],
            vec![0],
            vec![0, 0],
        );
        // The boundary biconditional: producerless *iff* input-anchored.
        rejects(
            "producerless",
            1,
            vec![Some(())],
            vec![edge(SfgGenerator::Discard, vec![0], vec![])],
            vec![],
            vec![],
        );

        // Clause 4 — acyclicity. `Copy ; Add` wired back on itself: the Kahn
        // sweep settles neither hyperedge.
        rejects(
            "acyclicity",
            3,
            vec![Some(()); 3],
            vec![
                edge(SfgGenerator::Copy, vec![0], vec![1, 2]),
                edge(SfgGenerator::Add, vec![1, 2], vec![0]),
            ],
            vec![],
            vec![],
        );

        // The accepting shape: `id₁`, one node in an input *and* an output
        // coordinate. Legal — that is precisely a wire no generator touches.
        let identity = from_parts::<Sfg>(1, vec![Some(())], vec![], vec![0], vec![0])
            .expect("id₁ is monogamous, acyclic, and mono on each anchor leg");
        assert!(content_eq(
            &identity,
            &content_of_colored(
                &ColoredExpr::<Sfg>::new(vec![()], PropExpr::Identity(1)).expect("id₁ at •")
            )
        ));
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
