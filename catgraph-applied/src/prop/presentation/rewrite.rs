//! A process **cost functional** and bounded **convex-DPO rewriting** over
//! abstract content ([#214](https://github.com/sustia-llc/catgraph/issues/214)
//! W2 + W3; [#57](https://github.com/sustia-llc/catgraph/issues/57) a2).
//!
//! # An optimizer, not a decider
//!
//! Everything here searches for a *cheaper writing of the same process*. It is
//! not a decision procedure and does not try to be:
//! [`Presentation::eq_mod`](super::Presentation::eq_mod) and
//! [`ColoredExpr::eq_colored`](crate::prop::colored::ColoredExpr::eq_colored)
//! remain the deciders, and nothing in this module is consulted by either
//! ([#15](https://github.com/sustia-llc/catgraph/issues/15) stays
//! (B) functorial-terminal). The engine below is a *separate surface*: it takes
//! oriented equations as rules, applies them to the content hypergraph, and
//! returns the cheapest representative it reached within a caller-supplied fuel
//! budget, together with a replayable trace.
//!
//! # Anchors
//!
//! Bonchi–Gadducci–Kissinger–Sobociński–Zanasi, *Rewriting modulo symmetric
//! monoidal structure*, **arXiv:1602.06771** (BGKSZ) — the same paper the
//! content substrate is anchored to (Prop 3.4, Thm 3.12; see
//! [`content`](super::content)):
//!
//! - **Def 3.10** — a *convex* sub-hypergraph: one closed under directed paths
//!   between its own nodes. Every match [`optimize`] applies satisfies it.
//! - **Def 5.4 / 5.5** — a convex match and the DPO step it licenses.
//! - **Thm 5.6** — convex-DPO **adequacy** for plain SMC: for any rewriting
//!   system `R` on `S_Σ`, `d ⇒_R e` iff `Φ(⌜d⌝) ⇒convex Φ(⌜R⌝) Φ(⌜e⌝)`. This is
//!   the per-step soundness claim of record: **each step [`optimize`] applies
//!   corresponds to a rewriting step modulo SMC structure with the given
//!   rules**, so the returned representative is related to the start by the
//!   rules in the free symmetric monoidal category.
//!
//! Papers are not kept in-tree; cite by arXiv id and theorem number.
//!
//! ## The Λ-colored lift is an extension
//!
//! BGKSZ's setting is **single-sorted** — the paper has no sorts or colors. The
//! lift to a Λ-colored signature ([#79](https://github.com/sustia-llc/catgraph/issues/79))
//! is a crate **extension**, marked here in the same style as `MatKron`'s
//! extension of F&S Ex 2.16 and `catgraph-magnitude`'s typed valuation surface.
//! It is a *restricting* extension: the convexity condition is unchanged and
//! matching additionally requires node-color equality, so every colored match is
//! an uncolored match and per-step soundness is inherited rather than re-proved.
//! The precedent for stating the underlying lemma color-generically is
//! `docs/SMC-NF-RECONCILIATION.md`, whose Lemma 4.1 is written over an arbitrary
//! `Λ`.
//!
//! # What is deliberately **not** claimed
//!
//! - **No termination.** The search is bounded by fuel and by nothing else.
//!   `docs/SMC-NF-RECONCILIATION.md` §4.7 carries the correction of record
//!   (2026-07-29): Lafont's strictly-monotone-interpretation technique proves
//!   termination for the PROP **F** of functions only — for the
//!   bialgebra-bearing system he states termination as a *conjecture* and
//!   documents the obstruction (`ε : 1 → 0` admits no strictly monotone
//!   interpretation) — and the nearest actual proof in the cached anchors,
//!   BGKSZ Thm 6.1, covers the **non-commutative** bimonoid. The commutative
//!   case an `E_18`-style system needs is unproven in those anchors. Fuel is the
//!   honest bound, and [`RewriteOutcome::fuel_exhausted`] reports when it bit.
//! - **No confluence, no normal form, no canonicality.**
//!   [`RewriteOutcome::best`] is *best found under fuel*, not a representative of
//!   anything. Two starting points in the same class may return different
//!   representatives, and a larger fuel budget may return a cheaper one.
//! - **No spider transparency.** [`FrobeniusOr`]-style spiders participate as
//!   **opaque generators**; their SCFM equations may be supplied as ordinary
//!   rules, which is sound but weaker than treating them as structure. BGKSZ
//!   **Thm 4.6** — rewriting modulo a chosen special Frobenius structure as
//!   *unrestricted* DPO over Frobenius termgraphs, where spiders become vertex
//!   identifications — is a **different substrate** and is explicitly out of
//!   scope here, as is MPZ23's commutative-(co)monoid middle case.
//!
//! [`FrobeniusOr`]: https://docs.rs/catgraph-syntax
//!
//! # Layering
//!
//! `cost_of` and the engine both read [`Content`], so both are functions of the
//! morphism's SMC-iso class (Lemma 4.1) rather than of its writing. Neither is
//! invariant under the *user* equations — that difference is the whole
//! optimization signal.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};

use catgraph::errors::CatgraphError;

use super::super::PropSignature;
use super::super::colored::ColoredExpr;
use super::content::{
    Content, ContentEdge, canonical_key, content_of_colored, from_parts, is_arity_well_formed,
};
use super::display::expr_of_content;

// ---------------------------------------------------------------- W2: cost

/// The cost of a process: `per_gen` summed over the content's hyperedges.
///
/// One hyperedge is one **generator occurrence** (BGKSZ §3), so the default
/// weighting `|_| 1` counts generators. A caller that prices its own steps —
/// per-step latency, money, headcount — supplies them through `per_gen`, and
/// this crate owns none of that semantics.
///
/// # Why it is defined on content
///
/// Content quotients by SMC coherence exactly (Lemma 4.1,
/// `docs/SMC-NF-RECONCILIATION.md` §4.2), so a function of the content is a
/// function of the **morphism** rather than of the writing. That is the property
/// §4.6's recorded rejection of the `out_min` comparator demanded — "lower is
/// not the objective; a comparator that is a function of the morphism is" —
/// and it is why the objective lives here rather than on [`PropExpr`].
///
/// It is **deliberately not** invariant under the presentation's user
/// equations: `Copy ; Add` and whatever an equation rewrites it to are
/// different contents with different costs, and that difference is precisely
/// the signal [`optimize`] chases ("the same job in fewer steps").
///
/// # Extension
///
/// No paper anchor exists for a cost functional on string diagrams; this is a
/// crate **extension**, marked as such. What *is* anchored is the invariance
/// argument above.
///
/// # Saturation
///
/// The sum saturates at [`u64::MAX`] rather than overflowing, so a pathological
/// weighting cannot panic a debug build or wrap a release one.
///
/// [`PropExpr`]: crate::prop::PropExpr
///
/// # Examples
///
/// ```
/// use catgraph_applied::prop::presentation::content::content_of;
/// use catgraph_applied::prop::presentation::rewrite::cost_of;
/// use catgraph_applied::prop::Free;
/// use catgraph_applied::rig::BoolRig;
/// use catgraph_applied::sfg::SfgGenerator;
///
/// type Sfg = SfgGenerator<BoolRig>;
/// let e = Free::<Sfg>::compose(
///     Free::generator(SfgGenerator::Copy),
///     Free::generator(SfgGenerator::Add),
/// )
/// .unwrap();
/// assert_eq!(cost_of(&content_of(&e), |_| 1), 2);
/// ```
#[must_use]
pub fn cost_of<G: PropSignature>(content: &Content<G>, per_gen: impl Fn(&G) -> u64) -> u64 {
    content.edges().iter().fold(0u64, |total, edge| {
        total.saturating_add(per_gen(&edge.label))
    })
}

// ---------------------------------------------------------------- W3: rules

/// An oriented rule `lhs ⇒ rhs`, compiled to a content-level span at
/// construction.
///
/// The two sides are parallel colored morphisms; [`RewriteRule::new`] validates
/// that and the extra conditions the DPO step needs, so every value of this type
/// is applicable and no rewrite site has to re-check.
#[derive(Clone, Debug)]
pub struct RewriteRule<G: PropSignature> {
    lhs: Content<G>,
    rhs: Content<G>,
    /// Which `lhs` nodes are *interior* — deleted by a step, rather than kept as
    /// the interface the replacement is glued along.
    interior: Vec<bool>,
    /// `lhs` hyperedges in a connectivity-first order, so the backtracking
    /// matcher propagates node bindings instead of guessing independently.
    order: Vec<usize>,
}

impl<G: PropSignature> RewriteRule<G> {
    /// Compile `lhs ⇒ rhs` into a content-level span.
    ///
    /// # What is required, and why
    ///
    /// 1. **Parallel.** Equal source words and equal target words. A step
    ///    replaces one side by the other in place, so the interfaces have to
    ///    agree letter for letter, not merely in width.
    /// 2. **Arity-well-formed** on both sides
    ///    ([`is_arity_well_formed`], which subsumes the
    ///    [#196](https://github.com/sustia-llc/catgraph/issues/196) overflow
    ///    screen). Unreachable through
    ///    [`ColoredExpr::new`](crate::prop::colored::ColoredExpr::new), reachable
    ///    across its serde trust boundary — and outside that domain
    ///    [`content_of_colored`] would panic
    ///    rather than answer.
    /// 3. **A non-empty left-hand side.** An edge-free `lhs` matches everywhere
    ///    and would make the search meaningless.
    /// 4. **A mono left interface.** No node may occur twice in
    ///    `lhs.input ++ lhs.output`. Two consequences of monogamy make this the
    ///    right condition: a node occurring in both anchors is a wire no
    ///    generator touches — a rule matching a bare wire — and a repeated
    ///    coordinate would make the interface non-injective, which is exactly
    ///    where the **unique pushout complement** BGKSZ's DPO step relies on
    ///    (the discussion following Thm 4.6) stops being unique.
    ///
    /// The right-hand side carries no such restriction: it may thread wires
    /// straight through (`Copy ; Add ⇒ id₁` is the motivating shape), and the
    /// step glues the corresponding interface nodes together.
    ///
    /// # Errors
    ///
    /// [`CatgraphError::Presentation`] naming the violated condition. Never
    /// panics — an ill-formed rule is rejected here rather than at a match site.
    pub fn new(lhs: ColoredExpr<G>, rhs: ColoredExpr<G>) -> Result<Self, CatgraphError> {
        let reject = |message: String| CatgraphError::Presentation { message };

        if lhs.source_word() != rhs.source_word() {
            return Err(reject(
                "rewrite rule: the two sides declare different source words".to_string(),
            ));
        }
        if lhs.target_word() != rhs.target_word() {
            return Err(reject(
                "rewrite rule: the two sides declare different target words".to_string(),
            ));
        }
        for (side, expr) in [("lhs", &lhs), ("rhs", &rhs)] {
            if !is_arity_well_formed(expr.expr()) {
                return Err(reject(format!(
                    "rewrite rule: the {side} is not arity-well-formed (#196 screen included)"
                )));
            }
        }

        let lhs = content_of_colored(&lhs);
        let rhs = content_of_colored(&rhs);

        if lhs.edges().is_empty() {
            return Err(reject(
                "rewrite rule: the lhs has no generator occurrence, so it matches everywhere"
                    .to_string(),
            ));
        }

        let mut on_boundary = vec![false; lhs.node_count()];
        for &x in lhs.input().iter().chain(lhs.output().iter()) {
            if on_boundary[x] {
                return Err(reject(format!(
                    "rewrite rule: the lhs interface is not mono — node {x} occupies two boundary \
                     coordinates, so the pushout complement is not unique"
                )));
            }
            on_boundary[x] = true;
        }
        let interior: Vec<bool> = on_boundary.iter().map(|&b| !b).collect();
        let order = connectivity_order(&lhs);

        Ok(Self {
            lhs,
            rhs,
            interior,
            order,
        })
    }
}

/// Order `content`'s hyperedges so each one after the first shares a node with
/// an earlier one wherever the hypergraph is connected enough to allow it.
fn connectivity_order<G: PropSignature>(content: &Content<G>) -> Vec<usize> {
    let (producer, consumer) = incidence(content);
    let mut seen = vec![false; content.edges().len()];
    let mut order = Vec::with_capacity(content.edges().len());
    for seed in 0..content.edges().len() {
        if seen[seed] {
            continue;
        }
        seen[seed] = true;
        let mut queue = VecDeque::from([seed]);
        while let Some(e) = queue.pop_front() {
            order.push(e);
            let edge = &content.edges()[e];
            for &x in edge.sources.iter().chain(edge.targets.iter()) {
                for f in [producer[x], consumer[x]].into_iter().flatten() {
                    if !seen[f] {
                        seen[f] = true;
                        queue.push_back(f);
                    }
                }
            }
        }
    }
    order
}

/// Per-node `(producer, consumer)` edge indices. Single-valued by monogamy
/// (BGKSZ Def 3.6), which every [`Content`] carries by construction.
fn incidence<G: PropSignature>(content: &Content<G>) -> (Vec<Option<usize>>, Vec<Option<usize>>) {
    let mut producer = vec![None; content.node_count()];
    let mut consumer = vec![None; content.node_count()];
    for (index, edge) in content.edges().iter().enumerate() {
        for &x in &edge.sources {
            consumer[x] = Some(index);
        }
        for &x in &edge.targets {
            producer[x] = Some(index);
        }
    }
    (producer, consumer)
}

// ---------------------------------------------------------------- matching

/// One convex match: the target node and edge each `lhs` item maps to.
#[derive(Clone, Debug)]
struct Match {
    nodes: Vec<usize>,
    edges: Vec<usize>,
}

/// Whether the sub-hypergraph spanned by `image` is **convex** (BGKSZ Def 3.10):
/// every directed path between two of its hyperedges stays inside it.
///
/// The test is one sweep: seed with the image's successors that lie *outside*
/// the image, relax forward, and reject the moment an image hyperedge is reached
/// again. That is exactly "a path left the image and came back", the shape a
/// pushout complement cannot absorb. `O(n + e)`.
fn is_convex<G: PropSignature>(
    target: &Content<G>,
    consumer: &[Option<usize>],
    image: &[usize],
) -> bool {
    let mut inside = vec![false; target.edges().len()];
    for &e in image {
        inside[e] = true;
    }
    let mut seen = vec![false; target.edges().len()];
    let mut queue: VecDeque<usize> = VecDeque::new();
    // The first hop out of the image is allowed — image-to-image adjacency is
    // not a violation — so the sweep is seeded with the outside successors only.
    for &e in image {
        for &x in &target.edges()[e].targets {
            if let Some(f) = consumer[x]
                && !inside[f]
                && !seen[f]
            {
                seen[f] = true;
                queue.push_back(f);
            }
        }
    }
    while let Some(g) = queue.pop_front() {
        for &x in &target.edges()[g].targets {
            if let Some(f) = consumer[x] {
                if inside[f] {
                    return false;
                }
                if !seen[f] {
                    seen[f] = true;
                    queue.push_back(f);
                }
            }
        }
    }
    true
}

/// A partial match being extended by backtracking.
struct Matcher<'a, G: PropSignature> {
    target: &'a Content<G>,
    lhs: &'a Content<G>,
    interior: &'a [bool],
    anchored: Vec<bool>,
    consumer: Vec<Option<usize>>,
    node_map: Vec<Option<usize>>,
    node_used: Vec<bool>,
    edge_map: Vec<Option<usize>>,
    edge_used: Vec<bool>,
}

impl<'a, G: PropSignature> Matcher<'a, G> {
    fn new(target: &'a Content<G>, rule: &'a RewriteRule<G>) -> Self {
        let (_, consumer) = incidence(target);
        let mut anchored = vec![false; target.node_count()];
        for &x in target.input().iter().chain(target.output().iter()) {
            anchored[x] = true;
        }
        Self {
            target,
            lhs: &rule.lhs,
            interior: &rule.interior,
            anchored,
            consumer,
            node_map: vec![None; rule.lhs.node_count()],
            node_used: vec![false; target.node_count()],
            edge_map: vec![None; rule.lhs.edges().len()],
            edge_used: vec![false; target.edges().len()],
        }
    }

    /// Bind `v ↦ w`, recording the binding in `undo` when it is new. Colors must
    /// agree — the Λ-colored refinement of BGKSZ's matching condition — and the
    /// map stays injective.
    fn bind_node(&mut self, v: usize, w: usize, undo: &mut Vec<usize>) -> bool {
        match self.node_map[v] {
            Some(already) => already == w,
            None => {
                if self.node_used[w] || self.lhs.node_colors()[v] != self.target.node_colors()[w] {
                    return false;
                }
                self.node_map[v] = Some(w);
                self.node_used[w] = true;
                undo.push(v);
                true
            }
        }
    }

    /// Bind every tentacle of `le ↦ te`. Tentacle *positions* are content
    /// invariants, so the correspondence is positional and forced.
    fn bind_tentacles(&mut self, le: usize, te: usize, undo: &mut Vec<usize>) -> bool {
        let (lhs, target) = (self.lhs, self.target);
        let (l_edge, t_edge) = (&lhs.edges()[le], &target.edges()[te]);
        for (&v, &w) in l_edge
            .sources
            .iter()
            .zip(t_edge.sources.iter())
            .chain(l_edge.targets.iter().zip(t_edge.targets.iter()))
        {
            if !self.bind_node(v, w, undo) {
                return false;
            }
        }
        true
    }

    fn unbind(&mut self, le: usize, te: usize, undo: Vec<usize>) {
        for v in undo {
            if let Some(w) = self.node_map[v].take() {
                self.node_used[w] = false;
            }
        }
        self.edge_map[le] = None;
        self.edge_used[te] = false;
    }

    /// Turn a total binding into a match, or reject it.
    fn finish(&self) -> Option<Match> {
        let mut nodes = Vec::with_capacity(self.node_map.len());
        for slot in &self.node_map {
            nodes.push((*slot)?);
        }
        let mut edges = Vec::with_capacity(self.edge_map.len());
        for slot in &self.edge_map {
            edges.push((*slot)?);
        }
        // A deleted node must be interior to the *target* as well: an anchored
        // one is boundary data no step may consume.
        for (v, &w) in nodes.iter().enumerate() {
            if self.interior[v] && self.anchored[w] {
                return None;
            }
        }
        is_convex(self.target, &self.consumer, &edges).then_some(Match { nodes, edges })
    }

    /// Depth-first extension over `order`, collecting at most `limit` matches.
    fn collect(&mut self, depth: usize, order: &[usize], limit: usize, out: &mut Vec<Match>) {
        if out.len() >= limit {
            return;
        }
        let Some(&le) = order.get(depth) else {
            if let Some(found) = self.finish() {
                out.push(found);
            }
            return;
        };
        for te in 0..self.target.edges().len() {
            if self.edge_used[te] || self.lhs.edges()[le].label != self.target.edges()[te].label {
                continue;
            }
            self.edge_map[le] = Some(te);
            self.edge_used[te] = true;
            let mut undo = Vec::new();
            if self.bind_tentacles(le, te, &mut undo) {
                self.collect(depth + 1, order, limit, out);
            }
            self.unbind(le, te, undo);
            if out.len() >= limit {
                return;
            }
        }
    }
}

/// Every convex match of `rule`'s left-hand side in `target`, up to `limit`.
///
/// # Cost
///
/// Backtracking over hyperedges, pruned by label equality, tentacle-position
/// forcing and node colors. Worst case **exponential in `|lhs|`** — the
/// small-rule assumption is explicit: rules are expected to be a handful of
/// generators, the size the equations of a presentation actually are.
fn matches_of<G: PropSignature>(
    target: &Content<G>,
    rule: &RewriteRule<G>,
    limit: usize,
) -> Vec<Match> {
    let mut out = Vec::new();
    if limit == 0 {
        return out;
    }
    let mut matcher = Matcher::new(target, rule);
    matcher.collect(0, &rule.order, limit, &mut out);
    out
}

// ---------------------------------------------------------------- the DPO step

fn find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}

fn union(parent: &mut [usize], a: usize, b: usize) {
    let (ra, rb) = (find(parent, a), find(parent, b));
    if ra != rb {
        parent[ra] = rb;
    }
}

/// Apply one DPO step: excise the matched hyperedges and the interior nodes they
/// span, then glue `rule.rhs` in along the interface (BGKSZ Def 5.5).
///
/// The result is rebuilt through the validating `content::from_parts`, which
/// re-checks monogamy and acyclicity — so a step that would leave the image
/// characterization of Thm 3.12 surfaces as an error here rather than as a
/// corrupt content downstream. That check is *unreachable* on a convex match:
/// convexity forbids a context path from the match's outputs back to its
/// inputs, which is the only way gluing the replacement in could close a cycle,
/// and the interface conditions keep every degree at most one. It runs anyway,
/// in both build profiles, because a corrupt content is not something to
/// discover later.
///
/// # Errors
///
/// [`CatgraphError::Presentation`] from the rebuild. [`optimize`] treats it as
/// "this match is not applicable" and moves on; [`replay`] propagates it, since
/// there the step came from a caller-supplied trace.
fn apply_match<G: PropSignature>(
    target: &Content<G>,
    rule: &RewriteRule<G>,
    found: &Match,
) -> Result<Content<G>, CatgraphError> {
    let (lhs, rhs) = (&rule.lhs, &rule.rhs);
    let t_nodes = target.node_count();
    let r_nodes = rhs.node_count();

    let mut deleted_edge = vec![false; target.edges().len()];
    for &e in &found.edges {
        deleted_edge[e] = true;
    }
    let mut deleted_node = vec![false; t_nodes];
    for (v, &interior) in rule.interior.iter().enumerate() {
        if interior {
            deleted_node[found.nodes[v]] = true;
        }
    }

    // Slots `0..t_nodes` are the target's nodes; the rest are the rhs's. Gluing
    // is the identification of the rhs boundary with the matched lhs boundary,
    // coordinate for coordinate — the interfaces agree because the rule is
    // parallel.
    let mut parent: Vec<usize> = (0..t_nodes + r_nodes).collect();
    for (coordinate, &rv) in rhs.input().iter().enumerate() {
        union(
            &mut parent,
            t_nodes + rv,
            found.nodes[lhs.input()[coordinate]],
        );
    }
    for (coordinate, &rv) in rhs.output().iter().enumerate() {
        union(
            &mut parent,
            t_nodes + rv,
            found.nodes[lhs.output()[coordinate]],
        );
    }

    // Dense renumbering: surviving target nodes in index order, then the rhs's
    // own. A class never mixes a deleted node with a kept one — the lhs
    // boundary is kept by definition, and nothing else is glued.
    let mut dense = vec![usize::MAX; t_nodes + r_nodes];
    let mut node_colors: Vec<Option<G::Color>> = Vec::new();
    for (x, &deleted) in deleted_node.iter().enumerate() {
        if deleted {
            continue;
        }
        let root = find(&mut parent, x);
        if dense[root] == usize::MAX {
            dense[root] = node_colors.len();
            node_colors.push(target.node_colors()[x].clone());
        }
    }
    for v in 0..r_nodes {
        let root = find(&mut parent, t_nodes + v);
        if dense[root] == usize::MAX {
            dense[root] = node_colors.len();
            node_colors.push(rhs.node_colors()[v].clone());
        }
    }
    let slot: Vec<usize> = (0..t_nodes + r_nodes)
        .map(|s| dense[find(&mut parent, s)])
        .collect();

    debug_assert!(
        (0..r_nodes).all(|v| rhs.node_colors()[v] == node_colors[slot[t_nodes + v]]),
        "invariant: the rule is parallel and the match preserves colors, so a glued class \
         carries one color"
    );

    let mut edges: Vec<ContentEdge<G>> =
        Vec::with_capacity(target.edges().len() + rhs.edges().len());
    for (index, edge) in target.edges().iter().enumerate() {
        if deleted_edge[index] {
            continue;
        }
        edges.push(ContentEdge {
            label: edge.label.clone(),
            sources: edge.sources.iter().map(|&x| slot[x]).collect(),
            targets: edge.targets.iter().map(|&x| slot[x]).collect(),
        });
    }
    for edge in rhs.edges() {
        edges.push(ContentEdge {
            label: edge.label.clone(),
            sources: edge.sources.iter().map(|&x| slot[t_nodes + x]).collect(),
            targets: edge.targets.iter().map(|&x| slot[t_nodes + x]).collect(),
        });
    }

    let input: Vec<usize> = target.input().iter().map(|&x| slot[x]).collect();
    let output: Vec<usize> = target.output().iter().map(|&x| slot[x]).collect();
    from_parts(node_colors.len(), node_colors, edges, input, output)
}

// ---------------------------------------------------------------- the search

/// One applied step of a [`RewriteOutcome`]'s trace: which rule fired, and which
/// hyperedges of the state it fired on.
///
/// The edge indices are the state's own, in the rule's internal `lhs`-edge
/// order, which makes the trace a **witness** rather than a summary: replaying
/// it with [`replay`] re-derives the state it describes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewriteStep {
    rule: usize,
    matched_edges: Vec<usize>,
}

impl RewriteStep {
    /// Index of the fired rule in the `rules` slice the step came from.
    #[must_use]
    pub fn rule(&self) -> usize {
        self.rule
    }

    /// The hyperedges of the state this step matched.
    #[must_use]
    pub fn matched_edges(&self) -> &[usize] {
        &self.matched_edges
    }
}

/// What [`optimize`] found: the cheapest representative it reached, its cost
/// against the start's, and the trace that gets there.
#[derive(Clone, Debug)]
pub struct RewriteOutcome<G: PropSignature> {
    best: ColoredExpr<G>,
    initial_cost: u64,
    best_cost: u64,
    steps: Vec<RewriteStep>,
    fuel_exhausted: bool,
    states_explored: usize,
}

impl<G: PropSignature> RewriteOutcome<G> {
    /// The cheapest representative found — read back from its content and
    /// re-checked through
    /// [`ColoredExpr::new`](crate::prop::colored::ColoredExpr::new).
    ///
    /// Best **found under fuel**: not a normal form, not canonical, and not
    /// stable under a change of budget.
    #[must_use]
    pub fn best(&self) -> &ColoredExpr<G> {
        &self.best
    }

    /// Consume the outcome for its representative.
    #[must_use]
    pub fn into_best(self) -> ColoredExpr<G> {
        self.best
    }

    /// [`cost_of`] on the starting morphism.
    #[must_use]
    pub fn initial_cost(&self) -> u64 {
        self.initial_cost
    }

    /// [`cost_of`] on [`Self::best`]. Never above [`Self::initial_cost`] — the
    /// start is itself a candidate.
    #[must_use]
    pub fn best_cost(&self) -> u64 {
        self.best_cost
    }

    /// The steps from the start to [`Self::best`], in application order. Empty
    /// exactly when the start was already the cheapest state reached.
    #[must_use]
    pub fn steps(&self) -> &[RewriteStep] {
        &self.steps
    }

    /// Whether the budget ran out with applicable matches still unexplored. A
    /// `false` means the reachable space was exhausted **within the rules
    /// given** — still not a canonicality claim.
    #[must_use]
    pub fn fuel_exhausted(&self) -> bool {
        self.fuel_exhausted
    }

    /// How many distinct states (by [`canonical_key`]) the search saw, including
    /// the start.
    #[must_use]
    pub fn states_explored(&self) -> usize {
        self.states_explored
    }
}

/// One node of the search tree.
struct SearchState<G: PropSignature> {
    content: Content<G>,
    cost: u64,
    parent: Option<usize>,
    step: Option<RewriteStep>,
}

/// Search for a cheaper writing of `start` under `rules`, spending at most
/// `fuel` rewrite applications.
///
/// Best-first over states keyed by [`canonical_key`] — the sanctioned dedup use
/// of that key — expanding the cheapest frontier state each round and applying
/// every convex match of every rule to it. `fuel` counts **applications**, so a
/// cyclic rule set (a commutativity rule and its mirror, say) terminates against
/// the visited set rather than looping.
///
/// # What the result means
///
/// Each recorded step is a convex-DPO step, so by BGKSZ **Thm 5.6** it is a
/// rewriting step modulo SMC structure with the given rules; the returned
/// representative is therefore related to `start` by those rules in the free
/// symmetric monoidal category. Nothing stronger: see the module docs on the
/// absent termination, confluence and canonicality claims. When the rules come
/// from a [`Presentation`](super::Presentation)'s equations, this composes to
/// "`start` and [`RewriteOutcome::best`] are equal modulo that presentation" —
/// which [`Presentation::eq_mod`](super::Presentation::eq_mod), the decider,
/// confirms independently.
///
/// # Errors
///
/// - [`CatgraphError::Presentation`] if `start` is not arity-well-formed —
///   reachable only across [`ColoredExpr`]'s serde trust boundary, and screened
///   here because
///   [`content_of_colored`] would panic
///   instead of answering.
/// - [`CatgraphError::Presentation`] if the readback of the best state does not
///   re-check as a colored morphism — an engine-invariant failure rather than a
///   user error. A step whose rebuild fails is *rejected* instead: the match is
///   skipped and the search continues (see `apply_match` on why that is
///   unreachable on a convex match).
pub fn optimize<G: PropSignature>(
    start: &ColoredExpr<G>,
    rules: &[RewriteRule<G>],
    fuel: usize,
    per_gen: impl Fn(&G) -> u64,
) -> Result<RewriteOutcome<G>, CatgraphError> {
    if !is_arity_well_formed(start.expr()) {
        return Err(CatgraphError::Presentation {
            message: "rewrite search: the starting morphism is not arity-well-formed".to_string(),
        });
    }

    let root = content_of_colored(start);
    let initial_cost = cost_of(&root, &per_gen);
    let mut visited: HashMap<_, usize> = HashMap::new();
    visited.insert(canonical_key(&root), 0);
    let mut states = vec![SearchState {
        content: root,
        cost: initial_cost,
        parent: None,
        step: None,
    }];
    let mut frontier = BinaryHeap::new();
    frontier.push(Reverse((initial_cost, 0usize)));

    let mut best = 0usize;
    let mut fuel_left = fuel;
    let mut fuel_exhausted = false;

    'search: while let Some(Reverse((_, index))) = frontier.pop() {
        for (rule_index, rule) in rules.iter().enumerate() {
            // One beyond the budget, so an unaffordable match stays visible.
            let found = matches_of(&states[index].content, rule, fuel_left.saturating_add(1));
            for one in found {
                if fuel_left == 0 {
                    fuel_exhausted = true;
                    break 'search;
                }
                fuel_left -= 1;
                let rebuilt = apply_match(&states[index].content, rule, &one);
                debug_assert!(
                    rebuilt.is_ok(),
                    "invariant: a convex match rebuilds to a monogamous acyclic content"
                );
                let Ok(next) = rebuilt else {
                    // Unreachable on a convex match (see `apply_match`); the
                    // match is *rejected* rather than failing the whole search.
                    continue;
                };
                let key = canonical_key(&next);
                if visited.contains_key(&key) {
                    continue;
                }
                let cost = cost_of(&next, &per_gen);
                let slot = states.len();
                visited.insert(key, slot);
                states.push(SearchState {
                    content: next,
                    cost,
                    parent: Some(index),
                    step: Some(RewriteStep {
                        rule: rule_index,
                        matched_edges: one.edges,
                    }),
                });
                if cost < states[best].cost {
                    best = slot;
                }
                frontier.push(Reverse((cost, slot)));
            }
        }
    }

    let mut steps = Vec::new();
    let mut walk = best;
    while let Some(parent) = states[walk].parent {
        if let Some(step) = states[walk].step.clone() {
            steps.push(step);
        }
        walk = parent;
    }
    steps.reverse();

    let best_cost = states[best].cost;
    let best = readback(start.source_word(), &states[best].content)?;
    Ok(RewriteOutcome {
        best,
        initial_cost,
        best_cost,
        steps,
        fuel_exhausted,
        states_explored: visited.len(),
    })
}

/// Re-derive the state a trace describes: apply `steps` to `start` in order.
///
/// This is what makes [`RewriteOutcome::steps`] a witness. Each step is
/// re-validated as a convex match before it is applied, so a trace that does not
/// describe a legal derivation is rejected rather than trusted.
///
/// # Errors
///
/// [`CatgraphError::Presentation`] if `start` is not arity-well-formed, if a
/// step names a rule outside `rules`, if its recorded hyperedges do not form a
/// convex match of that rule, or if the readback does not re-check.
pub fn replay<G: PropSignature>(
    start: &ColoredExpr<G>,
    rules: &[RewriteRule<G>],
    steps: &[RewriteStep],
) -> Result<ColoredExpr<G>, CatgraphError> {
    if !is_arity_well_formed(start.expr()) {
        return Err(CatgraphError::Presentation {
            message: "rewrite replay: the starting morphism is not arity-well-formed".to_string(),
        });
    }
    let mut content = content_of_colored(start);
    for (position, step) in steps.iter().enumerate() {
        let rule = rules
            .get(step.rule)
            .ok_or_else(|| CatgraphError::Presentation {
                message: format!(
                    "rewrite replay: step {position} names rule {} of {}",
                    step.rule,
                    rules.len()
                ),
            })?;
        let found = match_at(&content, rule, &step.matched_edges).ok_or_else(|| {
            CatgraphError::Presentation {
                message: format!(
                    "rewrite replay: step {position} does not describe a convex match"
                ),
            }
        })?;
        content = apply_match(&content, rule, &found)?;
    }
    readback(start.source_word(), &content)
}

/// Rebuild the match a recorded hyperedge assignment describes, re-running every
/// condition [`matches_of`] enforces.
fn match_at<G: PropSignature>(
    target: &Content<G>,
    rule: &RewriteRule<G>,
    assignment: &[usize],
) -> Option<Match> {
    if assignment.len() != rule.lhs.edges().len() {
        return None;
    }
    let mut matcher = Matcher::new(target, rule);
    for &le in &rule.order {
        let te = *assignment.get(le)?;
        if te >= target.edges().len()
            || matcher.edge_used[te]
            || rule.lhs.edges()[le].label != target.edges()[te].label
        {
            return None;
        }
        matcher.edge_map[le] = Some(te);
        matcher.edge_used[te] = true;
        let mut undo = Vec::new();
        if !matcher.bind_tentacles(le, te, &mut undo) {
            return None;
        }
    }
    matcher.finish()
}

/// Read a content back out as a colored morphism.
///
/// [`expr_of_content`] is total on everything the content constructors produce,
/// and the [`ColoredExpr::new`] re-check in front of the result **is** the
/// engine's output validation: a readback that did not realize the boundary
/// words would be rejected here rather than returned.
fn readback<G: PropSignature>(
    source_word: &[G::Color],
    content: &Content<G>,
) -> Result<ColoredExpr<G>, CatgraphError> {
    ColoredExpr::new(source_word.to_vec(), expr_of_content(content))
}
