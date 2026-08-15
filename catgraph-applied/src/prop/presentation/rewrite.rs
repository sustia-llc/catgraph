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
//! # The policy and the primitives it is built from
//!
//! [`optimize`] is *one* policy — descend on [`cost_of`], stop on fuel. The two
//! things it does underneath are public in their own right
//! ([#250](https://github.com/sustia-llc/catgraph/issues/250)): [`match_sites`]
//! enumerates every convex match of a rule, preferring none, and [`apply_at`]
//! fires one chosen site, comparing nothing. A caller with a different objective
//! — a neutral walk, a search that must move through a *dearer* writing to reach
//! a cheaper one, an externally scored choice — writes it over that pair rather
//! than reshaping the search. [`MatchSite::into_step`] turns the sites it picked
//! into ordinary [`RewriteStep`]s, so such a walk is [`replay`]-able like any
//! other trace. [`match_sites_of`] / [`rewrite_at`] are the same pair over a
//! [`ColoredExpr`], for a single named step; a multi-step search stays on
//! [`Content`] and pays the readback once at the end.
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
//! The colored conclusion leans on the color-generic lifts of Lemma 3.11 /
//! Thm 3.12 recorded in the Lemma 4.1 proof
//! (`docs/SMC-NF-RECONCILIATION.md` §4.2), which realize Thm 5.6's
//! factorization contexts as colored terms; color matching strictly refining
//! matches supplies the rest.
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
//! - **No spider transparency.** `catgraph-syntax`'s `FrobeniusOr`-style
//!   spiders participate as **opaque generators**; their SCFM equations may be
//!   supplied as ordinary rules, which is sound but weaker than treating them as
//!   structure. BGKSZ **Thm 4.6** — rewriting modulo a chosen special Frobenius
//!   structure as *unrestricted* DPO over Frobenius termgraphs, where spiders
//!   become vertex identifications — is a **different substrate** and is
//!   explicitly out of scope here, as is MPZ23's commutative-(co)monoid middle
//!   case.
//!
//! # Layering
//!
//! `cost_of` and the engine both read [`Content`], so both are functions of the
//! morphism's SMC-iso class (Lemma 4.1) rather than of its writing. Neither is
//! invariant under the *user* equations — that difference is the whole
//! optimization signal.

use std::cmp::Reverse;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::hash::{Hash, Hasher};

use catgraph::errors::CatgraphError;

use super::super::PropSignature;
use super::super::colored::{ColoredExpr, check};
use super::content::{
    Content, ContentEdge, canonical_key, content_eq, content_of_colored, from_parts,
    is_arity_well_formed,
};
use super::display::expr_of_content;

// -------------------------------------------------------- the trust boundary

/// Re-validate a caller-supplied [`ColoredExpr`] from scratch — arity first,
/// then the *words* — naming it `what` in any rejection.
///
/// Every value [`ColoredExpr::new`] builds is word-well-formed by construction,
/// because that constructor runs [`check`]. Its **serde path does not**:
/// deserialization reconstructs the three fields directly, and the trust
/// boundary documented there prescribes exactly this remedy — "when ingesting
/// untrusted documents, re-validate by rebuilding". So each public entry point
/// of this module re-derives the target word from the expression and the
/// declared source word, and requires it to be the one stored.
///
/// An arity screen alone would not do. It admits a forged document whose
/// `Compose` joins matching *widths* with mismatched *colors* — a term no
/// [`ColoredExpr::new`] can produce — and the whole colored engine then reads
/// node colors that no `⟦·⟧` ever assigned, including through the
/// `content::from_parts` rebuild. The arity clause stays in front of the word
/// clause anyway, because an arity-ill-formed tree is the one that would make
/// [`content_of_colored`] *panic*, and it earns the sharper message.
///
/// Once per input, at entry — never per search state.
fn revalidate<G: PropSignature>(what: &str, expr: &ColoredExpr<G>) -> Result<(), CatgraphError> {
    if !is_arity_well_formed(expr.expr()) {
        return Err(CatgraphError::Presentation {
            message: format!("{what} is not arity-well-formed (#196 screen included)"),
        });
    }
    let derived =
        check(expr.expr(), expr.source_word()).map_err(|error| CatgraphError::Presentation {
            message: format!("{what} is not word-well-formed: {error}"),
        })?;
    if derived != expr.target_word() {
        return Err(CatgraphError::Presentation {
            message: format!(
                "{what} declares a target word its expression does not produce: stored \
                 {:?}, derived {derived:?}",
                expr.target_word()
            ),
        });
    }
    Ok(())
}

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
    /// 2. **Well-formed on both sides, re-derived rather than trusted.** Arity
    ///    first ([`is_arity_well_formed`], which subsumes the
    ///    [#196](https://github.com/sustia-llc/catgraph/issues/196) overflow
    ///    screen), then the words: [`check`] is re-run against the declared
    ///    source word and the target word it derives must equal the stored one
    ///    (the private `revalidate`). Both clauses are unreachable through
    ///    [`ColoredExpr::new`](crate::prop::colored::ColoredExpr::new) and both
    ///    are reachable across its serde trust boundary. Outside the arity
    ///    domain [`content_of_colored`] would panic rather than answer; inside
    ///    it, an un-rechecked *word* would let a rule whose two sides are not
    ///    the colored morphisms they claim to be match on label equality alone.
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
            revalidate(&format!("rewrite rule: the {side}"), expr)?;
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

/// A structural hash of `content` **as a representation** — not up to iso.
///
/// This is deliberately *not* [`canonical_key`]. That key is a complete
/// invariant of the SMC-iso class, so it is stable under exactly the renumbering
/// this has to detect: `apply_match` renumbers the surviving nodes densely and
/// appends the rhs's hyperedges, and two different states can be iso-equal while
/// their index spaces disagree. A [`MatchSite`] is a tuple of *indices*, so what
/// it has to be pinned to is the writing, not the class.
///
/// Hashed, in order: the node count, the node colors, the hyperedge count, then
/// each hyperedge's `(label, sources, targets)` **in index order**, then the
/// input and output anchors. Every one of those moves under a renumbering that
/// changes which edge an index names, which is the property the check needs.
///
/// It is a hash, so it is a screen and not a proof: distinct contents may
/// collide with probability ~2⁻⁶⁴, and a collision degrades exactly to the
/// pre-fingerprint behaviour — the convex-match re-derivation `match_at` runs
/// next, which is what previously carried the whole check.
fn fingerprint<G: PropSignature>(content: &Content<G>) -> u64 {
    let mut hasher = DefaultHasher::new();
    content.node_count().hash(&mut hasher);
    content.node_colors().hash(&mut hasher);
    content.edges().len().hash(&mut hasher);
    for edge in content.edges() {
        edge.label.hash(&mut hasher);
        edge.sources.hash(&mut hasher);
        edge.targets.hash(&mut hasher);
    }
    content.input().hash(&mut hasher);
    content.output().hash(&mut hasher);
    hasher.finish()
}

/// One convex match: the target node and edge each `lhs` item maps to.
///
/// Sites are produced by [`match_sites`] and consumed by [`apply_at`] — together
/// the **neutral** primitive pair [`optimize`] is built from: enumeration, and
/// apply-at-a-chosen-site, with no cost descent and no fuel
/// ([#250](https://github.com/sustia-llc/catgraph/issues/250)). A caller wanting
/// some policy other than "descend on cost" writes it over these two; `optimize`
/// keeps its own policy and is not routed through them.
///
/// # The edge assignment is what a site *is*
///
/// [`Self::matched_nodes`] is forced by [`Self::matched_edges`]: tentacle
/// positions are content invariants, so binding a `lhs` hyperedge to a target one
/// binds every node it touches. That is why [`Self::into_step`] loses nothing in
/// handing the site to a [`RewriteStep`], which records hyperedges alone, and why
/// [`apply_at`] re-derives the node images from the edges rather than trusting
/// the ones stored here.
///
/// # A site names its content, and is rejected anywhere else
///
/// Both index vectors are the *target's* own indices, so a site is meaningful
/// only against the content it was enumerated from — and a **stale** site is the
/// dangerous case, not the obvious one. `apply_match` renumbers the surviving
/// nodes densely and appends the rhs's hyperedges, so after one step the old
/// indices name *different* edges; in a repetitive content they can still happen
/// to form a perfectly convex match there. Re-deriving the assignment alone
/// would accept such a site and rewrite the wrong place in silence — `A ; A ; A`
/// under `A ⇒ B` is the two-line demonstration.
///
/// So a site also carries a private **fingerprint** of the content it came from
/// (the private `fingerprint`: a structural hash of that content as a
/// *representation*, not up to iso), and [`apply_at`] checks it first. The two
/// rejections are kept apart deliberately: a site from another content is a
/// stale or foreign *site*, while a site whose content is right but whose
/// assignment is not convex for this rule is a bad *pairing* — different
/// diagnoses, and a caller has to be able to tell them apart. The correct loop
/// re-enumerates after every apply.
///
/// The fingerprint is *not* carried into a step. [`Self::into_step`] drops it,
/// because a [`RewriteStep`] is index-based by design and [`replay`] re-derives
/// every step against the running content it has actually reached — which is
/// also why the site the private `match_at` builds is stamped with the content
/// it was just validated against rather than with any earlier one.
///
/// # Equality
///
/// Two sites enumerated from the same content compare exactly as their index
/// vectors do; sites from different contents differ on the fingerprint too,
/// which is the same statement as "a site is meaningful only against its own
/// content".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchSite {
    nodes: Vec<usize>,
    edges: Vec<usize>,
    /// The private `fingerprint` of the content this site was enumerated — or,
    /// through the private `match_at`, validated — against.
    content: u64,
}

impl MatchSite {
    /// The target hyperedges this site occupies, indexed by `lhs` hyperedge:
    /// position `i` is the image of `lhs` hyperedge `i`.
    ///
    /// The same vector [`RewriteStep::matched_edges`] carries, in the same
    /// **`lhs`-hyperedge index order** — *not* the matcher's traversal order,
    /// which is [`RewriteRule`]'s internal connectivity-first permutation of the
    /// same edges. The two are different sequences, so correlating position `i`
    /// here with the `i`-th edge the matcher visited reads the wrong `lhs` edge.
    #[must_use]
    pub fn matched_edges(&self) -> &[usize] {
        &self.edges
    }

    /// The target nodes this site occupies, indexed by `lhs` node.
    ///
    /// Determined by [`Self::matched_edges`] (see the type docs), so this is a
    /// convenience for reading *where* a site sits — which wires it would glue
    /// along — not an independent degree of freedom.
    #[must_use]
    pub fn matched_nodes(&self) -> &[usize] {
        &self.nodes
    }

    /// Record this site as a trace step firing rule index `rule`.
    ///
    /// The bridge from the neutral surface to the trace surface: the result is an
    /// ordinary [`RewriteStep`], so a caller driving its own search with
    /// [`match_sites`] / [`apply_at`] can hand the sequence it chose to
    /// [`replay`] and have every step re-validated. `rule` indexes the `rules`
    /// slice the replay will be given — a step binds a rule *index*, not a rule
    /// identity, so the two must line up.
    ///
    /// The site's content fingerprint is **dropped** here, and a
    /// [`RewriteStep`] carries none. That is the design, not an omission: a step
    /// is index-based against the *running* content of a replay, which is a
    /// different content at every position, so pinning it to the one content the
    /// site was enumerated from would reject legal traces. [`replay`] re-derives
    /// each step's assignment against the state it has actually reached.
    #[must_use]
    pub fn into_step(self, rule: usize) -> RewriteStep {
        RewriteStep {
            rule,
            matched_edges: self.edges,
        }
    }
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
    /// The private `fingerprint` of `target`, computed once here and stamped
    /// onto every [`MatchSite`] this matcher produces.
    target_fingerprint: u64,
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
            target_fingerprint: fingerprint(target),
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
    fn finish(&self) -> Option<MatchSite> {
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
        //
        // Provably unreachable, and kept as defense. An interior lhs node is
        // neither input- nor output-anchored there, so by the lhs's own
        // boundary biconditional it has a producer *and* a consumer inside the
        // lhs; both are matched, so its image has a producer and a consumer
        // among the matched target hyperedges, and the target's biconditional
        // then says it is anchored on neither foot. One sweep, against a
        // failure mode — a deleted boundary node — that is worth not having.
        for (v, &w) in nodes.iter().enumerate() {
            if self.interior[v] && self.anchored[w] {
                return None;
            }
        }
        // Stamped with the fingerprint of the content the match was just
        // established against — the target of *this* matcher, whether it was
        // built by `match_sites`'s enumeration or by `match_at`'s re-derivation
        // of a caller-supplied assignment.
        is_convex(self.target, &self.consumer, &edges).then_some(MatchSite {
            nodes,
            edges,
            content: self.target_fingerprint,
        })
    }

    /// Depth-first extension over `order`, collecting at most `limit` matches.
    fn collect(&mut self, depth: usize, order: &[usize], limit: usize, out: &mut Vec<MatchSite>) {
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
/// The **enumeration** half of the neutral primitive pair
/// ([#250](https://github.com/sustia-llc/catgraph/issues/250)); [`apply_at`] is
/// the other. Neutral in the exact sense that no site is preferred: every convex
/// match is returned, cheaper or dearer or neither, with no cost functional
/// consulted and no fuel spent. [`optimize`] is the cost-descending policy built
/// on top; this is what it is built *from*, and a caller wanting some other
/// policy — enumerate, score by its own criterion, apply one — writes it here
/// instead of reaching into the search.
///
/// `limit` truncates: enumeration stops as soon as `limit` sites are in hand, so
/// which sites come back is the matcher's own order and not a ranking. `limit`
/// of `0` returns nothing.
///
/// [`match_sites_of`] is the same enumeration one level up, over a
/// [`ColoredExpr`] rather than its content.
///
/// # Cost
///
/// Backtracking over hyperedges, pruned by label equality, tentacle-position
/// forcing and node colors. Worst case **exponential in `|lhs|`** — the
/// small-rule assumption is explicit: rules are expected to be a handful of
/// generators, the size the equations of a presentation actually are.
#[must_use]
pub fn match_sites<G: PropSignature>(
    target: &Content<G>,
    rule: &RewriteRule<G>,
    limit: usize,
) -> Vec<MatchSite> {
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
/// re-checks the whole image characterization of Thm 3.12 — so a step that
/// would leave it surfaces as an error here rather than as a corrupt content
/// downstream. The rebuild runs in both build profiles, because a corrupt
/// content is not something to discover later.
///
/// # Why the rebuild cannot fail on a convex match
///
/// **Acyclicity.** Suppose the rebuilt content had a directed cycle. The cycle
/// alternates between hyperedges that came from the rhs and hyperedges of the
/// surviving context, so take any maximal *context* segment of it. That segment
/// starts at a class containing the image of an lhs **output** node — its
/// producer inside the target was matched, since the mono interface forces
/// every lhs output node to have an lhs producer, so what the segment leaves by
/// is a context consumer — and it ends, dually, at a class containing the image
/// of an lhs **input** node, whose consumer was matched. That is a directed
/// context path leaving the image at one matched hyperedge and re-entering at
/// another: exactly the BGKSZ Def 3.10 convexity violation `is_convex` rejects,
/// so no such match reaches this function.
///
/// **Monogamy.** A glued class holds at most three nodes — one rhs node, one
/// lhs-input image, one lhs-output image — because the rhs anchors are mono,
/// the lhs interface is mono ([`RewriteRule::new`] clause 4) and the match is
/// injective, so two nodes of the same kind could only be glued through a
/// coordinate collision that mono-ness rules out. Of those three, the rhs node
/// is anchored on both rhs feet (that is how it got glued twice) and so has
/// neither producer nor consumer inside the rhs; the lhs-input image has no
/// surviving consumer, its lhs consumer having been deleted; the lhs-output
/// image has no surviving producer. So the class's producer is the lhs-input
/// image's — a context hyperedge, or none, in which case the class is
/// input-anchored — and its consumer is the lhs-output image's. One each, which
/// is the condition. Every node outside a glued class keeps the degrees it had.
///
/// # Errors
///
/// [`CatgraphError::Presentation`] from the rebuild. [`replay`] propagates it,
/// since there the step came from a caller-supplied trace. [`optimize`]'s
/// handling is **profile-dependent** — see its `# Errors`.
fn apply_match<G: PropSignature>(
    target: &Content<G>,
    rule: &RewriteRule<G>,
    found: &MatchSite,
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

/// Apply `rule` at `site` in `target`, and return the rewritten content.
///
/// The **apply-at-a-chosen-site** half of the neutral primitive pair
/// ([#250](https://github.com/sustia-llc/catgraph/issues/250)); [`match_sites`]
/// is the other. It fires wherever the caller points it: no cost is compared, no
/// fuel is spent, and a site that *raises* the cost is applied as readily as one
/// that lowers it. [`optimize`]'s cost descent is a policy over this, not a
/// property of it.
///
/// # The site is re-validated, not trusted — in two independent stages
///
/// A [`MatchSite`] is an ordinary value: it can be enumerated against one content
/// and handed to another, kept across an apply that renumbered everything, or
/// paired with a rule it never matched. Two checks answer those separately, and
/// the order matters.
///
/// 1. **Is this the site's content at all?** The site's fingerprint (see
///    [`MatchSite`]) must equal `target`'s. Re-deriving the assignment would
///    *not* settle this: this function renumbers the surviving nodes and appends
///    the rhs, so a site kept across one apply names different hyperedges
///    afterwards and, in a repetitive content, can still form a convex match
///    there. Without this stage such a site is applied at the wrong location in
///    silence — the failure the stage exists to make impossible.
/// 2. **Is this assignment a convex match of *this rule* here?** The hyperedge
///    assignment is then re-run through every condition [`match_sites`] enforces
///    — label agreement, injectivity, positional tentacle binding, node colors,
///    no deleted boundary node, and BGKSZ Def 3.10 convexity — before any gluing
///    happens, and the node images the step uses are the ones that re-derivation
///    produces rather than the ones the value carries.
///
/// This is the same trust boundary [`replay`] applies to a caller-supplied
/// [`RewriteStep`] (stage 2 of it; a step deliberately carries no fingerprint,
/// see [`MatchSite::into_step`]), and it is the point of the primitive: an
/// unchecked apply is what the caller would have had to write themselves.
///
/// # Errors
///
/// Two **distinct** rejections, reported with different messages because they are
/// different diagnoses and the fix differs:
///
/// - [`CatgraphError::Presentation`] if `site` was enumerated from a *different
///   content* than `target` — a foreign site, or a stale one held across an
///   apply. The site's indices name other hyperedges here; the fix is to
///   re-enumerate against the content in hand. This says nothing about whether
///   the assignment would have been convex.
/// - [`CatgraphError::Presentation`] if `site` belongs to `target` but its
///   hyperedges are not a convex match **of `rule`** there — typically the wrong
///   rule. The fix is a different rule, or a different site.
/// - [`CatgraphError::Presentation`] from the rebuild, which re-establishes the
///   BGKSZ Thm 3.12 image characterization. Unreachable on a convex match, for
///   the reasons the private `apply_match` argues.
pub fn apply_at<G: PropSignature>(
    target: &Content<G>,
    rule: &RewriteRule<G>,
    site: &MatchSite,
) -> Result<Content<G>, CatgraphError> {
    if site.content != fingerprint(target) {
        return Err(CatgraphError::Presentation {
            message: "rewrite at a site: the site was enumerated from a different content, so \
                      its hyperedge indices do not name the same hyperedges here — re-enumerate \
                      against this content (a site does not survive an apply)"
                .to_string(),
        });
    }
    let found = match_at(target, rule, &site.edges).ok_or_else(|| CatgraphError::Presentation {
        message: "rewrite at a site: the site's hyperedges do not describe a convex match of \
                  this rule in this content"
            .to_string(),
    })?;
    apply_match(target, rule, &found)
}

// ---------------------------------------------------------------- the search

/// One applied step of a [`RewriteOutcome`]'s trace: which rule fired, and which
/// hyperedges of the state it fired on.
///
/// The edge indices are the state's own, in **`lhs`-hyperedge index order**
/// (position `i` is the image of `lhs` hyperedge `i`) — *not* the matcher's
/// traversal order, which is [`RewriteRule`]'s internal connectivity-first
/// permutation of the same edges. That makes the trace a **witness** rather than
/// a summary: replaying it with [`replay`] re-derives the state it describes.
///
/// [`optimize`] is not the only source of steps: [`MatchSite::into_step`] turns a
/// site a caller chose itself into one, so a search driven over [`match_sites`]
/// and [`apply_at`] produces traces of the same kind, replayable the same way
/// ([#250](https://github.com/sustia-llc/catgraph/issues/250)).
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
    /// The cheapest representative found — read back from its content, then
    /// re-checked through
    /// [`ColoredExpr::new`](crate::prop::colored::ColoredExpr::new) *and*
    /// against that state's own content (see the private `readback`).
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
/// **This is one policy, not the surface.** Cost descent and a fuel budget are
/// choices made here; [`match_sites`] and [`apply_at`] are the neutral primitives
/// underneath, and a caller wanting a different policy drives those directly
/// ([#250](https://github.com/sustia-llc/catgraph/issues/250)) rather than
/// reshaping this function.
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
/// can confirm independently (soundly, in the `Some(true)` direction —
/// [#15](https://github.com/sustia-llc/catgraph/issues/15)).
///
/// # Errors
///
/// - [`CatgraphError::Presentation`] if `start` is not well-formed — arity or
///   words, the private `revalidate` — reachable only across [`ColoredExpr`]'s
///   serde trust boundary, and screened here because [`content_of_colored`]
///   would panic instead of answering, and because an unchecked word would put
///   node colors no `⟦·⟧` assigned in front of the colored matcher.
/// - [`CatgraphError::Presentation`] if the readback of the best state does not
///   re-check as a colored morphism, or does not carry that state's content —
///   an engine-invariant failure rather than a user error.
///
/// A step whose rebuild fails is not an error of this function, and what
/// becomes of it **depends on the build profile**: with `debug_assertions` on
/// — the test profile, deliberately — the failure trips an assertion and the
/// search dies loudly; with them off the match is *skipped* and the search
/// continues with the states it already has. Both are unreachable on a convex
/// match, for the reasons `apply_match` argues.
pub fn optimize<G: PropSignature>(
    start: &ColoredExpr<G>,
    rules: &[RewriteRule<G>],
    fuel: usize,
    per_gen: impl Fn(&G) -> u64,
) -> Result<RewriteOutcome<G>, CatgraphError> {
    revalidate("rewrite search: the starting morphism", start)?;

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
            let found = match_sites(&states[index].content, rule, fuel_left.saturating_add(1));
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
/// describe a legal derivation is rejected rather than trusted. The validation
/// is relative to `rules` **as given** — the trace binds rule *indices*, not
/// rule identities; a different rules slice may replay to a different, still
/// legal endpoint.
///
/// # Errors
///
/// [`CatgraphError::Presentation`] if `start` is not well-formed (arity or
/// words — the private `revalidate`), if a step names a rule outside `rules`,
/// if its recorded hyperedges do not form a convex match of that rule, or if
/// the readback does not re-check.
pub fn replay<G: PropSignature>(
    start: &ColoredExpr<G>,
    rules: &[RewriteRule<G>],
    steps: &[RewriteStep],
) -> Result<ColoredExpr<G>, CatgraphError> {
    revalidate("rewrite replay: the starting morphism", start)?;
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

// ------------------------------------------------ the expr-level site surface

/// Every convex match of `rule`'s left-hand side in `expr`, up to `limit`.
///
/// [`match_sites`] one level up: the expression's content is computed, the
/// enumeration runs there, and the sites index *that* content — which is the
/// content [`rewrite_at`] recomputes, so the pair composes. The sites are
/// fingerprinted with it too (see [`MatchSite`]), so [`rewrite_at`] takes them
/// and refuses one enumerated from a different expression, or held across an
/// earlier rewrite.
///
/// # Prefer the content level for a multi-step search
///
/// This wrapper and [`rewrite_at`] each pay a `content_of_colored` on the way in,
/// and `rewrite_at` a readback on the way out; a search that takes `n` steps
/// through them pays `n` of each. The content level pays neither: hold a
/// [`Content`], loop [`match_sites`] and [`apply_at`] over it, and read back
/// **once** at the end — [`optimize`] is written exactly that way. That asymmetry
/// is why both levels exist: this one for a single named step from and to a
/// [`ColoredExpr`], the content one for search.
///
/// # Cost
///
/// [`match_sites`]'s, unchanged: backtracking over hyperedges, pruned by label
/// equality, tentacle-position forcing and node colors, worst case
/// **exponential in `|lhs|`** under an explicit small-rule assumption — rules are
/// expected to be a handful of generators, the size the equations of a
/// presentation actually are.
///
/// # Errors
///
/// [`CatgraphError::Presentation`] if `expr` is not well-formed — arity or
/// words, the private `revalidate`, under the same message [`rewrite_at`] uses,
/// because it is the same screen. Reachable only across [`ColoredExpr`]'s
/// documented serde trust boundary, and screened here rather than left to panic:
/// this is the **first** step of the enumerate-then-apply pair, so a malformed
/// term would abort before `rewrite_at`'s own screen was ever reached.
///
/// The malformed case is an `Err`, never an empty `Vec` — "this term is not a
/// term" and "this term has no sites" are different answers, and returning the
/// second for the first is a silent wrong answer.
pub fn match_sites_of<G: PropSignature>(
    expr: &ColoredExpr<G>,
    rule: &RewriteRule<G>,
    limit: usize,
) -> Result<Vec<MatchSite>, CatgraphError> {
    revalidate("rewrite site enumeration: the morphism", expr)?;
    Ok(match_sites(&content_of_colored(expr), rule, limit))
}

/// Apply `rule` at `site` in `expr`, and read the result back as a colored
/// morphism.
///
/// [`apply_at`] one level up, with the same two-stage re-validation: `site`'s
/// fingerprint must be `expr`'s content's, and its assignment is then re-derived
/// there before anything is glued — so a site enumerated from some other
/// morphism, or held across an earlier rewrite, is rejected rather than
/// reinterpreted. The output goes through the module's usual two-check readback
/// — re-checked as a colored morphism, then compared back against the content it
/// was read off.
///
/// Pair it with [`match_sites_of`]; for more than one step, see that function's
/// note on staying at the content level and paying the readback once.
///
/// # Errors
///
/// - [`CatgraphError::Presentation`] if `expr` is not well-formed — arity or
///   words, the private `revalidate` — reachable only across [`ColoredExpr`]'s
///   serde trust boundary.
/// - [`CatgraphError::Presentation`] if `site` was enumerated from a different
///   content than `expr`'s — a foreign or stale site. [`apply_at`]'s first
///   rejection, with its message.
/// - [`CatgraphError::Presentation`] if `site` belongs to `expr`'s content but is
///   not a convex match of `rule` there. [`apply_at`]'s second rejection, kept
///   distinct from the first because the two call for different fixes.
/// - [`CatgraphError::Presentation`] if the readback does not re-check as a
///   colored morphism, or does not carry the rewritten content — an
///   engine-invariant failure rather than a user error.
pub fn rewrite_at<G: PropSignature>(
    expr: &ColoredExpr<G>,
    rule: &RewriteRule<G>,
    site: &MatchSite,
) -> Result<ColoredExpr<G>, CatgraphError> {
    revalidate("rewrite at a site: the morphism", expr)?;
    let rewritten = apply_at(&content_of_colored(expr), rule, site)?;
    readback(expr.source_word(), &rewritten)
}

/// Rebuild the match a recorded hyperedge assignment describes, re-running every
/// condition [`match_sites`] enforces.
///
/// The [`MatchSite`] this returns is stamped with `target`'s fingerprint — the
/// content the assignment was just validated against, not any content it may
/// have been recorded from. That is what keeps [`replay`] working: a step's
/// assignment is *meant* to be read against the running content at that
/// position, so the site built from it belongs to that content by construction.
fn match_at<G: PropSignature>(
    target: &Content<G>,
    rule: &RewriteRule<G>,
    assignment: &[usize],
) -> Option<MatchSite> {
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

/// Read a content back out as a colored morphism, and check that it is one.
///
/// # The output validation
///
/// Two checks, and together they are the engine's output validation:
///
/// 1. [`ColoredExpr::new`] re-runs [`check`], so a readback that did not
///    realize the boundary words is rejected rather than returned;
/// 2. the content of that colored morphism is compared against the state the
///    readback came from, with [`content_eq`] — cospan iso under both feet, the
///    only sense available, since node indices are presentation-dependent.
///
/// (2) is what discharges a dependency at runtime. [`expr_of_content`] is total
/// on everything the content constructors produce, but its round-trip property
/// — `content_eq(content_of(expr_of_content(c)), c)` — is **corpus-verified,
/// not proven** (see [`expr_of_content`]'s own docs: 200 000 contents per mode
/// in `tests/canonical_display_corpus.rs`). Without (2) the engine would be
/// resting a per-step soundness claim on an unproven premise; with it, a
/// readback that lost the content is an error at the call site instead.
///
/// # Errors
///
/// [`CatgraphError::Presentation`] if either check fails. Both are
/// engine-invariant failures, not user errors.
fn readback<G: PropSignature>(
    source_word: &[G::Color],
    content: &Content<G>,
) -> Result<ColoredExpr<G>, CatgraphError> {
    let read = ColoredExpr::new(source_word.to_vec(), expr_of_content(content))?;
    if !content_eq(&content_of_colored(&read), content) {
        return Err(CatgraphError::Presentation {
            message: "rewrite readback: the expression read off the best state does not carry \
                      that state's content"
                .to_string(),
        });
    }
    Ok(read)
}
