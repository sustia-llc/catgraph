//! Canonical string-diagram **display**, by readback from abstract content.
//!
//! ```text
//! canonical_display(e)  =  nf(expr_of_content(content_of(e)))
//! ```
//!
//! [`expr_of_content`] writes a layered expression out of a content cospan in one
//! deterministic sweep, and the **unchanged** [`nf`] then normalizes it.
//! Canonicality is by construction rather than by a uniqueness theorem about
//! `nf`: the sweep reads only the canonical relabeling of the content, which
//! factors through the complete invariant
//! [`canonical_key`](super::content::canonical_key), so SMC-equal expressions —
//! equal content by Lemma 4.1 (`docs/SMC-NF-RECONCILIATION.md` §4.2) — reach
//! *identical* expressions before `nf` is even called. This is **display only**:
//! `nf` is untouched, and neither
//! [`Presentation::eq_mod`](super::Presentation::eq_mod) nor the
//! congruence-closure engine consults this module.
//!
//! # The sweep
//!
//! [`expr_of_content`] is Theorem 4.5's induction (§4.4) run as an algorithm:
//! layers are built top-down from the input foot, maintaining the wire word, and
//! each hyperedge is placed at the earliest layer where its source nodes are all
//! present and contiguous in tentacle order. The schedule is `ldepth`-**guided**,
//! not `λ = ldepth` — a non-contiguous source span, a gather pass or a source-0
//! insertion pass each push occupants down a layer — because `λ` and `ι` are
//! chosen **together**, an `η`'s slot changing the wire adjacency that fixes an
//! anchored consumer's earliest admissible layer.
//!
//! Source-0 hyperedges (`η`) are placed by the private `demand` helper. Where a
//! coordinate is **pinned** — by the consumer's tentacle order, or by the output
//! coordinates the wire reaches downstream — the `η` goes in the layer
//! immediately above that consumer. Where neither pins one the coordinate is a
//! positional **convention** (`free_slot`) and such `η` are emitted one per pass,
//! so a consumer waiting on two unpinned `η`s sits two or more layers below the
//! first of them. Ties among coordinate-equal `η`s break by reachable output
//! coordinate, then tentacle position, then the canonical numbering: global data,
//! never a per-pair comparator.
//!
//! Both the pinning rules and the `λ`/`ι` coupling above are stated for the
//! `0 → 1` source-0 shape, the only one §4.4's `ldepth`/`ceil` definitions cover;
//! `demand` correspondingly reads only the *first* target of a `0 → n` hyperedge
//! and lets the remaining wires be routed by braids.
//!
//! Closed components — the part of the content no anchor reaches — are emitted
//! as `0 → 0` blocks to the left of the anchored part, in the canonical key's
//! sorted block order.
//!
//! # Crossings
//!
//! `Braid` atoms are forced whenever a consumer wants `(v, u)` from a producer
//! emitting `(u, v)`: such a content has no braid-free layered writing at all,
//! which is how §4.4 records that Theorem 4.5 is vacuous there. `nf` cannot
//! delete a dead braid, so a crossing the sweep inserts *without* that excuse is
//! permanent, and the convention above can cause one — a wire the content does
//! not place has to cross back out if the content later pins it elsewhere.

use std::collections::VecDeque;

use super::super::{PropExpr, PropSignature};
use super::content::{Content, canonical_content, content_of};
use super::smc_nf::{StringDiagram, nf};

// ---------------------------------------------------------------- public API

/// Write a content cospan back out as a layered [`PropExpr`].
///
/// The result is a **function of the content's iso class under both feet**: if
/// [`content_eq`](super::content::content_eq) holds of two contents, this returns
/// the very same expression for both, which is why the sweep reads the canonical
/// relabeling rather than the presentation-dependent raw node indices. The result
/// is arity-well-formed — [`nf`]'s precondition.
///
/// # The round trip
///
/// The intended property is `content_eq(&content_of(&expr_of_content(c)), c)` —
/// recovery of the content up to cospan iso under both feet, the only sense
/// available since the readback renumbers nodes. It is **not** proven here;
/// `tests/canonical_display_corpus.rs` checks it over the frozen corpora. For a
/// content from [`content_of_colored`](super::content::content_of_colored) the
/// round trip returns the *uncolored* reading of the same cospan, since the color
/// of a wire no generator touches is data the boundary word carries and the
/// expression does not.
///
/// # Totality
///
/// Total on every value the two content constructors can produce, including the
/// shapes §4.4 leaves outside `𝔉′`: `0 → 0` scalars, `0 → n` sources with
/// `n ≥ 2`, closed components, contents with no hyperedges at all, and contents
/// with **no** braid-free realization. It allocates and never panics.
///
/// # Cost
///
/// For a content with `n` nodes, `e` hyperedges and a wire word of width `w`:
/// worst case `O(e² · w + e · w²)`, plus `O(n²)` for the `reach` relaxation.
///
/// - the sweep runs at most `2e + 4` passes, and each rescans every unplaced
///   hyperedge against the word by linear search (`O(e · w)` per pass);
/// - a gather emits up to `O(w²)` adjacent transpositions, and at most `e` of
///   them plus one final permutation occur;
/// - `reach` relaxes over the DAG without a topological pre-order, so it costs
///   `O(n)` passes over `O(n)` nodes.
#[must_use]
pub fn expr_of_content<G: PropSignature>(c: &Content<G>) -> PropExpr<G> {
    let canonical = canonical_content(c);
    let incidence = incidence(&canonical);
    let (anchored, closed) = split(&canonical, &incidence);

    let mut parts: Vec<PropExpr<G>> = closed
        .iter()
        .map(|component| layout(&canonical, &incidence, component, &[], &[]))
        .collect();
    parts.push(layout(
        &canonical,
        &incidence,
        &anchored,
        canonical.input(),
        canonical.output(),
    ));

    let expr = tensor_all(parts);
    debug_assert!(
        super::content::is_arity_well_formed(&expr),
        "invariant: every emitted layer covers the whole wire word, so the \
         layer composition is arity-well-formed"
    );
    expr
}

/// The canonical display of `e`: its normal form *after* canonical readback.
///
/// Two SMC-equal expressions always get the same display, on every diagram —
/// including the families `nf` alone separates (§4.6's ledger). The mechanism is
/// composition, not a stronger `nf`: content decides SMC-equality exactly
/// (Lemma 4.1) and [`expr_of_content`] is a function of the content's iso class,
/// so the two calls to `nf` see literally the same expression.
///
/// `canonical_display(e)` and `nf(e)` **may differ**, and where they do neither
/// is wrong — they are the normal forms of two SMC-equal writings. On `𝔉′`
/// (§4.4: braid-free normal form, no source-0 hyperedge with slack) they are
/// expected to agree, by §4.4's canonicality corollary.
///
/// # Panics
///
/// Panics where [`content_of`] does: `e` is not arity-well-formed — some
/// `Compose` joins a target arity to a different source arity, or some `Braid` /
/// `Tensor` width sums past `usize::MAX`. Ask
/// [`is_arity_well_formed`](super::content::is_arity_well_formed) first if the
/// tree is hand-built or deserialized.
#[must_use]
pub fn canonical_display<G: PropSignature>(e: &PropExpr<G>) -> StringDiagram<G> {
    nf(&expr_of_content(&content_of(e)))
}

// ---------------------------------------------------------------- incidence

/// Per-node incidence of the canonical content, in the form the sweep reads it.
/// Monogamy (BGKSZ Def 3.6) makes each field single-valued; the `is_none` guards
/// below keep the sweep total rather than trusting that at runtime.
struct Incidence {
    /// The edge producing the node, if any.
    producer: Vec<Option<usize>>,
    /// `(edge, tentacle position)` consuming the node, if any.
    consumer: Vec<Option<(usize, usize)>>,
    /// The node's coordinate in the output anchor, if it has one.
    out_coord: Vec<Option<usize>>,
    /// The least output coordinate reachable downstream of the node, or
    /// [`usize::MAX`] where the whole downstream cone dies inside the diagram.
    ///
    /// This is the readback's planarity handle. Braid-free layers never cross
    /// wires (§4.4's positional monotonicity), so in a braid-free realization the
    /// wires of a boundary are ordered *weakly* consistently with the output
    /// coordinates they reach: `w` left of `w′` gives `reach(w) ≤ reach(w′)`. The
    /// strict form is false — two source tentacles of one hyperedge reach the
    /// identical set of outputs — so the comparison is `>=`, inserting before the
    /// first wire that does not strictly precede, and a real tie falls through to
    /// the tie key or to [`free_slot`]'s convention.
    reach: Vec<usize>,
}

impl Incidence {
    /// The edges incident to `node`, in the fixed `[consumer, producer]` order
    /// the canonical numbering also uses.
    fn incident(&self, node: usize) -> [Option<usize>; 2] {
        [self.consumer[node].map(|(e, _)| e), self.producer[node]]
    }
}

fn incidence<G: PropSignature>(c: &Content<G>) -> Incidence {
    let n = c.node_count();
    let mut inc = Incidence {
        producer: vec![None; n],
        consumer: vec![None; n],
        out_coord: vec![None; n],
        reach: vec![usize::MAX; n],
    };
    for (index, edge) in c.edges().iter().enumerate() {
        for (position, &x) in edge.sources.iter().enumerate() {
            if inc.consumer[x].is_none() {
                inc.consumer[x] = Some((index, position));
            }
        }
        for &x in &edge.targets {
            if inc.producer[x].is_none() {
                inc.producer[x] = Some(index);
            }
        }
    }
    for (coordinate, &x) in c.output().iter().enumerate() {
        if inc.out_coord[x].is_none() {
            inc.out_coord[x] = Some(coordinate);
        }
    }

    // `reach` by relaxation against the flow. The content is acyclic (BGKSZ
    // Thm 3.12), so one pass per node suffices and the loop exits early on the
    // usual shapes.
    for x in 0..n {
        if let Some(coordinate) = inc.out_coord[x] {
            inc.reach[x] = coordinate;
        }
    }
    for _ in 0..=n {
        let mut changed = false;
        for x in 0..n {
            if inc.out_coord[x].is_some() {
                continue;
            }
            let Some((k, _)) = inc.consumer[x] else {
                continue;
            };
            let least = c.edges()[k]
                .targets
                .iter()
                .map(|&y| inc.reach[y])
                .min()
                .unwrap_or(usize::MAX);
            if least < inc.reach[x] {
                inc.reach[x] = least;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    inc
}

/// Split the edges into the boundary-attached ones and the closed components,
/// the same two halves [`content_eq`](super::content::content_eq) decides
/// separately. Both come out in canonical order: `canonical_content` lays the
/// anchored edges out in the key's discovery order and each closed block's edges
/// consecutively in sorted block order, so index order reproduces it.
fn split<G: PropSignature>(c: &Content<G>, inc: &Incidence) -> (Vec<usize>, Vec<Vec<usize>>) {
    let mut seen_node = vec![false; c.node_count()];
    let mut seen_edge = vec![false; c.edges().len()];
    let mut queue: VecDeque<usize> = VecDeque::new();
    for &x in c.input().iter().chain(c.output().iter()) {
        if !seen_node[x] {
            seen_node[x] = true;
            queue.push_back(x);
        }
    }
    while let Some(x) = queue.pop_front() {
        for e in inc.incident(x).into_iter().flatten() {
            if seen_edge[e] {
                continue;
            }
            seen_edge[e] = true;
            let edge = &c.edges()[e];
            for &y in edge.sources.iter().chain(edge.targets.iter()) {
                if !seen_node[y] {
                    seen_node[y] = true;
                    queue.push_back(y);
                }
            }
        }
    }

    let anchored: Vec<usize> = (0..c.edges().len()).filter(|&e| seen_edge[e]).collect();

    let mut component = vec![usize::MAX; c.edges().len()];
    let mut components: Vec<Vec<usize>> = Vec::new();
    for start in 0..c.edges().len() {
        if seen_edge[start] || component[start] != usize::MAX {
            continue;
        }
        let id = components.len();
        component[start] = id;
        let mut stack = vec![start];
        let mut members = Vec::new();
        while let Some(e) = stack.pop() {
            members.push(e);
            let edge = &c.edges()[e];
            for &x in edge.sources.iter().chain(edge.targets.iter()) {
                for f in inc.incident(x).into_iter().flatten() {
                    if !seen_edge[f] && component[f] == usize::MAX {
                        component[f] = id;
                        stack.push(f);
                    }
                }
            }
        }
        members.sort_unstable();
        components.push(members);
    }

    (anchored, components)
}

// ---------------------------------------------------------------- the sweep

/// Where in `word` the node sits, if it is there at all.
///
/// Returns the *first* occurrence, and the sweep relies on there being only one:
/// a node occupying two coordinates of one anchor would put two wires in the word
/// for one content node, and the layout would silently drop one of them rather
/// than fail. The legs of the cospan are mono (BGKSZ Def 3.6, Thm 3.12), so that
/// cannot happen for any [`Content`] the constructors produce.
fn position(word: &[usize], node: usize) -> Option<usize> {
    word.iter().position(|&x| x == node)
}

/// The start of `nodes` as a contiguous run of `word`, in that order.
fn contiguous_at(word: &[usize], nodes: &[usize]) -> Option<usize> {
    let head = *nodes.first()?;
    let start = position(word, head)?;
    let end = start.checked_add(nodes.len())?;
    (end <= word.len() && word[start..end] == *nodes).then_some(start)
}

/// Right-associated tensor of `parts`, with tensor units dropped.
fn tensor_all<G: PropSignature>(parts: Vec<PropExpr<G>>) -> PropExpr<G> {
    let mut iter = parts
        .into_iter()
        .filter(|p| !matches!(p, PropExpr::Identity(0)))
        .rev();
    let Some(last) = iter.next() else {
        return PropExpr::Identity(0);
    };
    iter.fold(last, |acc, part| {
        PropExpr::Tensor(Box::new(part), Box::new(acc))
    })
}

/// Left-associated composition of `layers` — the shape
/// [`from_string_diagram`](super::smc_nf::from_string_diagram) also produces.
fn compose_all<G: PropSignature>(layers: Vec<PropExpr<G>>, width: usize) -> PropExpr<G> {
    let mut iter = layers.into_iter();
    let Some(first) = iter.next() else {
        return PropExpr::Identity(width);
    };
    iter.fold(first, |acc, layer| {
        PropExpr::Compose(Box::new(acc), Box::new(layer))
    })
}

/// One adjacent transposition at `at`, padded to `width`.
fn swap_layer<G: PropSignature>(at: usize, width: usize) -> PropExpr<G> {
    tensor_all(vec![
        PropExpr::Identity(at),
        PropExpr::Braid(1, 1),
        PropExpr::Identity(width.saturating_sub(at + 2)),
    ])
}

/// Bubble `word` into `target` by adjacent transpositions, emitting one layer per
/// swap. Selection order settles coordinate 0 first, then 1, …, so the emitted
/// word is a function of the two sequences alone.
fn bubble_to<G: PropSignature>(word: &mut [usize], target: &[usize]) -> Vec<PropExpr<G>> {
    let mut layers = Vec::new();
    if word.len() != target.len() {
        return layers;
    }
    let width = word.len();
    for (i, &wanted) in target.iter().enumerate() {
        let Some(mut j) = (i..width).find(|&j| word[j] == wanted) else {
            continue;
        };
        while j > i {
            word.swap(j - 1, j);
            layers.push(swap_layer(j - 1, width));
            j -= 1;
        }
    }
    layers
}

/// The word `word` becomes once `nodes` are gathered into one contiguous run,
/// in tentacle order, at the position of their leftmost member.
fn gather_target(word: &[usize], nodes: &[usize]) -> Option<Vec<usize>> {
    let mut leftmost = usize::MAX;
    for &x in nodes {
        leftmost = leftmost.min(position(word, x)?);
    }
    let before = word[..leftmost]
        .iter()
        .filter(|x| !nodes.contains(x))
        .count();
    let rest: Vec<usize> = word
        .iter()
        .copied()
        .filter(|x| !nodes.contains(x))
        .collect();
    let mut target = Vec::with_capacity(word.len());
    target.extend_from_slice(&rest[..before]);
    target.extend_from_slice(nodes);
    target.extend_from_slice(&rest[before..]);
    Some(target)
}

/// The coordinate a source-0 wire takes when nothing in the content pins one:
/// the far side of the wire word, out of the way of every wire already placed.
///
/// This is the positional convention §2.3's slot rule is, and it has to be one:
/// where the two competing writings give the `η` an identical content attachment
/// record, no content lookup can separate the candidates. The scope of the choice
/// is recorded in `docs/SMC-NF-RECONCILIATION.md` §4.4's "Canonicalizing `ι`"
/// item. Several `η` landing on *one* coordinate are still emitted left to right
/// in tie order; what the far side governs is the free-coordinate side alone.
const fn free_slot(width: usize) -> usize {
    width
}

/// Where a source-0 hyperedge's first output wire goes, and how coordinate-ties
/// among several of them break.
///
/// Two rules, tried in that order, and a convention behind them:
///
/// 1. **The consumer's tentacle order.** If the consumer already has one of its
///    other source wires on the word, the `η`'s wire must land beside it —
///    Lemma 4.4's "positive-source atoms occupy the contiguous span of their
///    source nodes".
/// 2. **The output coordinates reached downstream** (`Incidence::reach`). A
///    braid-free boundary orders its wires by the outputs they reach, so
///    inserting before the first live wire that exits at or after this one's own
///    least reachable coordinate keeps a planar content planar.
/// 3. Otherwise the slot is genuinely **free**, this returns `None`, and the
///    caller falls back to [`free_slot`]'s convention.
///
/// The returned tie is the secondary sort key for `η`s that land on the same
/// coordinate in the same layer — reachable output coordinate first, then
/// tentacle position. Both are content data under the canonical numbering, so the
/// order is global, never a per-pair comparison.
///
/// # Scope: the `0 → 1` shape
///
/// This reads `targets.first()` and nothing else, so for a `0 → n (n ≥ 2)`
/// hyperedge it places the whole output block by where the *first* wire is wanted
/// and leaves the rest to the gather pass's braids, matching §4.4, whose
/// `ldepth`/`ceil` definitions cover the `0 → 1` shape and whose definedness
/// bullet defers `0 → 0` and `0 → n`.
fn demand<G: PropSignature>(
    c: &Content<G>,
    inc: &Incidence,
    word: &[usize],
    placed: &[bool],
    edge: usize,
) -> Option<(usize, (usize, usize))> {
    let z = *c.edges()[edge].targets.first()?;
    let mut tie = (inc.reach[z], usize::MAX);

    if let Some((k, tentacle)) = inc.consumer[z]
        && !placed[k]
    {
        tie.1 = tentacle;
        let sources = &c.edges()[k].sources;
        for i in (0..tentacle).rev() {
            if let Some(p) = position(word, sources[i]) {
                return Some((p + 1, tie));
            }
        }
        for &node in sources.iter().skip(tentacle + 1) {
            if let Some(p) = position(word, node) {
                return Some((p, tie));
            }
        }
    }

    if tie.0 == usize::MAX {
        return None;
    }
    let at = word
        .iter()
        .position(|&x| inc.reach[x] != usize::MAX && inc.reach[x] >= tie.0)
        .unwrap_or(word.len());
    Some((at, tie))
}

/// Build one part of the diagram: the layers placing `edges`, taking the wire
/// word from `input` to `output`.
///
/// The anchored part is called with the content's two anchors; each closed
/// component is called with empty ones, and so comes out as a `0 → 0` block.
fn layout<G: PropSignature>(
    c: &Content<G>,
    inc: &Incidence,
    edges: &[usize],
    input: &[usize],
    output: &[usize],
) -> PropExpr<G> {
    let mut word: Vec<usize> = input.to_vec();
    let mut layers: Vec<PropExpr<G>> = Vec::new();
    let mut remaining: Vec<usize> = edges.to_vec();
    let mut placed = vec![true; c.edges().len()];
    for &e in edges {
        placed[e] = false;
    }

    // Every pass either places at least one edge or gathers one edge's sources,
    // and a gather makes that edge ready, so twice the edge count plus slack
    // bounds the loop. The budget is a guard, not a reachable exit, and breaks
    // rather than panics to keep the display path total.
    let mut budget = 2 * remaining.len() + 4;

    while !remaining.is_empty() && budget > 0 {
        budget -= 1;

        // (1) Everything whose source nodes are present and contiguous in
        //     tentacle order goes now — the earliest-admissible clause, which
        //     realizes `ldepth` wherever contiguity does not intervene.
        let mut ready: Vec<(usize, usize)> = remaining
            .iter()
            .filter(|&&e| !c.edges()[e].sources.is_empty())
            .filter_map(|&e| contiguous_at(&word, &c.edges()[e].sources).map(|p| (p, e)))
            .collect();
        if !ready.is_empty() {
            ready.sort_unstable();
            layers.push(place_generators(c, &mut word, &ready));
            for (_, e) in ready {
                placed[e] = true;
            }
            remaining.retain(|e| !placed[*e]);
            continue;
        }

        // (2) Sources all present but out of order: no braid-free layer can take
        //     this boundary to the next one, so cross the wires. Smallest
        //     canonical index first, so the choice is content data.
        let gatherable = remaining.iter().copied().find(|&e| {
            let sources = &c.edges()[e].sources;
            !sources.is_empty() && sources.iter().all(|x| word.contains(x))
        });
        if let Some(e) = gatherable
            && let Some(target) = gather_target(&word, &c.edges()[e].sources)
            && target != word
        {
            layers.extend(bubble_to(&mut word, &target));
            continue;
        }

        // (3) Nothing can proceed on the wires present, so the missing wires
        //     are source-0 outputs: emit them, `λ` and `ι` chosen together.
        let mut inserts: Vec<(usize, (usize, usize), usize)> = Vec::new();
        for &e in &remaining {
            if !c.edges()[e].sources.is_empty() {
                continue;
            }
            if let Some((at, tie)) = demand(c, inc, &word, &placed, e) {
                inserts.push((at, tie, e));
            }
        }
        if inserts.is_empty() {
            // Nothing in the content pins a coordinate for any of them. Emit
            // exactly one — the canonically first — and go round again: the wire
            // it adds is what lets the next one be pinned by its consumer's
            // tentacle order rather than by convention.
            if let Some(e) = remaining
                .iter()
                .copied()
                .find(|&e| c.edges()[e].sources.is_empty())
            {
                inserts.push((free_slot(word.len()), (usize::MAX, usize::MAX), e));
            }
        }
        if inserts.is_empty() {
            break;
        }
        inserts.sort_unstable();
        layers.push(place_sources(c, &mut word, &inserts));
        for (_, _, e) in inserts {
            placed[e] = true;
        }
        remaining.retain(|e| !placed[*e]);
    }

    debug_assert!(
        remaining.is_empty(),
        "invariant: acyclicity leaves some hyperedge placeable on every pass"
    );

    if word != output {
        let target = output.to_vec();
        layers.extend(bubble_to(&mut word, &target));
    }

    compose_all(layers, input.len())
}

/// One layer placing every ready positive-source hyperedge, with identities
/// filling the gaps. `ready` is sorted by span start and the spans are disjoint,
/// since a node has one consumer; the `at < cursor` skip below states that
/// disjointness defensively — overlapping spans would violate monogamy (BGKSZ
/// Def 3.6) and the skip drops the hyperedge rather than panicking.
fn place_generators<G: PropSignature>(
    c: &Content<G>,
    word: &mut Vec<usize>,
    ready: &[(usize, usize)],
) -> PropExpr<G> {
    let mut atoms: Vec<PropExpr<G>> = Vec::new();
    let mut next: Vec<usize> = Vec::with_capacity(word.len());
    let mut cursor = 0usize;
    for &(at, e) in ready {
        if at < cursor {
            continue;
        }
        if at > cursor {
            atoms.push(PropExpr::Identity(at - cursor));
            next.extend_from_slice(&word[cursor..at]);
        }
        let edge = &c.edges()[e];
        atoms.push(PropExpr::Generator(edge.label.clone()));
        next.extend_from_slice(&edge.targets);
        cursor = at + edge.sources.len();
    }
    if cursor < word.len() {
        atoms.push(PropExpr::Identity(word.len() - cursor));
        next.extend_from_slice(&word[cursor..]);
    }
    *word = next;
    tensor_all(atoms)
}

/// One layer placing source-0 hyperedges at their chosen coordinates.
/// `inserts` is sorted by `(coordinate, tie, edge)`.
fn place_sources<G: PropSignature>(
    c: &Content<G>,
    word: &mut Vec<usize>,
    inserts: &[(usize, (usize, usize), usize)],
) -> PropExpr<G> {
    let mut atoms: Vec<PropExpr<G>> = Vec::new();
    let mut next: Vec<usize> = Vec::with_capacity(word.len());
    let mut cursor = 0usize;
    for &(at, _, e) in inserts {
        let at = at.min(word.len());
        if at > cursor {
            atoms.push(PropExpr::Identity(at - cursor));
            next.extend_from_slice(&word[cursor..at]);
            cursor = at;
        }
        let edge = &c.edges()[e];
        atoms.push(PropExpr::Generator(edge.label.clone()));
        next.extend_from_slice(&edge.targets);
    }
    if cursor < word.len() {
        atoms.push(PropExpr::Identity(word.len() - cursor));
        next.extend_from_slice(&word[cursor..]);
    }
    *word = next;
    tensor_all(atoms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prop::Free;
    use crate::prop::presentation::content::content_eq;
    use crate::prop::presentation::smc_nf::Atom;
    use crate::rig::BoolRig;
    use crate::sfg::SfgGenerator;

    type Sfg = SfgGenerator<BoolRig>;
    type E = PropExpr<Sfg>;

    fn prim(g: Sfg) -> E {
        Free::<Sfg>::generator(g)
    }

    fn readback(e: &E) -> E {
        expr_of_content(&content_of(e))
    }

    /// The proof obligation, at unit scale: readback preserves content.
    fn round_trips(e: &E) {
        let c = content_of(e);
        assert!(
            content_eq(&content_of(&readback(e)), &c),
            "readback changed the content of {e:?}"
        );
    }

    fn has_braid(sd: &StringDiagram<Sfg>) -> bool {
        sd.layers
            .iter()
            .any(|l| l.atoms.iter().any(|a| matches!(a, Atom::Braid(_, _))))
    }

    #[test]
    fn identity_reads_back_as_itself() {
        assert_eq!(readback(&PropExpr::Identity(3)), PropExpr::Identity(3));
        assert_eq!(readback(&PropExpr::Identity(0)), PropExpr::Identity(0));
    }

    #[test]
    fn a_braid_is_emitted_because_the_content_demands_it() {
        // The anchors are permuted, so no braid-free writing exists.
        let e = PropExpr::<Sfg>::Braid(1, 1);
        assert_eq!(readback(&e), PropExpr::Braid(1, 1));
        round_trips(&e);
    }

    #[test]
    fn a_single_generator_reads_back_alone() {
        let e = prim(SfgGenerator::Copy);
        assert_eq!(readback(&e), PropExpr::Generator(SfgGenerator::Copy));
        round_trips(&e);
    }

    #[test]
    fn a_chain_reads_back_as_layers() {
        let e = Free::compose(prim(SfgGenerator::Copy), prim(SfgGenerator::Add))
            .expect("Copy : 1 → 2 ; Add : 2 → 1");
        assert_eq!(
            readback(&e),
            PropExpr::Compose(
                Box::new(PropExpr::Generator(SfgGenerator::Copy)),
                Box::new(PropExpr::Generator(SfgGenerator::Add)),
            )
        );
        round_trips(&e);
    }

    #[test]
    fn an_eta_lands_at_the_coordinate_its_consumer_wants() {
        // Add consumes (input wire, Zero's wire) in that tentacle order, so the
        // η must be written to the *right* of the through-wire.
        let e = Free::compose(
            Free::tensor(PropExpr::Identity(1), prim(SfgGenerator::Zero)),
            prim(SfgGenerator::Add),
        )
        .expect("(id₁ ⊗ Zero) : 1 → 2 ; Add : 2 → 1");
        let back = readback(&e);
        round_trips(&e);
        assert!(!has_braid(&nf(&back)), "no crossing is needed here");
        assert_eq!(canonical_display(&e), nf(&e));

        // The claim is about *where* the η goes, so assert the layout, not just
        // the agreement: layer 0 carries the η at slot 1, to the right of the
        // through-wire, which is the coordinate `Add`'s tentacle order forces.
        let display = canonical_display(&e);
        assert_eq!(
            display.layers[0].atoms,
            vec![Atom::Identity(1), Atom::Generator(SfgGenerator::Zero),]
        );
        assert_eq!(
            display.layers[1].atoms,
            vec![Atom::Generator(SfgGenerator::Add)]
        );
    }

    #[test]
    fn a_closed_component_is_emitted_as_a_left_block() {
        // (Zero ; Discard) is reachable from neither foot.
        let closed = Free::compose(prim(SfgGenerator::Zero), prim(SfgGenerator::Discard))
            .expect("Zero : 0 → 1 ; Discard : 1 → 0");
        let e = Free::tensor(PropExpr::Identity(1), closed);
        let back = readback(&e);
        round_trips(&e);
        assert_eq!(
            back,
            PropExpr::Tensor(
                Box::new(PropExpr::Compose(
                    Box::new(PropExpr::Generator(SfgGenerator::Zero)),
                    Box::new(PropExpr::Generator(SfgGenerator::Discard)),
                )),
                Box::new(PropExpr::Identity(1)),
            )
        );
    }

    #[test]
    fn two_isomorphic_closed_components_both_survive() {
        let loop_ = Free::compose(prim(SfgGenerator::Zero), prim(SfgGenerator::Discard))
            .expect("Zero : 0 → 1 ; Discard : 1 → 0");
        let e = Free::tensor(loop_.clone(), loop_);
        round_trips(&e);
        assert_eq!(content_of(&readback(&e)).edges().len(), 4);
    }

    #[test]
    fn the_written_layer_of_an_eta_does_not_reach_the_display() {
        // §4.4's withdrawal witness: two SMC-equal writings whose normal forms
        // differ. Their displays do not.
        let left = Free::tensor(
            Free::compose(
                prim(SfgGenerator::Copy),
                Free::tensor(PropExpr::Identity(1), prim(SfgGenerator::Discard)),
            )
            .expect("Copy : 1 → 2 ; (id₁ ⊗ Discard) : 2 → 1"),
            prim(SfgGenerator::Zero),
        );
        let right = Free::compose(
            prim(SfgGenerator::Copy),
            Free::tensor(
                PropExpr::Identity(1),
                Free::tensor(prim(SfgGenerator::Zero), prim(SfgGenerator::Discard)),
            ),
        )
        .expect("Copy : 1 → 2 ; (id₁ ⊗ Zero ⊗ Discard) : 2 → 2");
        assert_ne!(nf(&left), nf(&right), "the §4.4 witness still separates");
        assert_eq!(canonical_display(&left), canonical_display(&right));
    }

    #[test]
    fn a_layer_pinned_eta_below_layer_zero_round_trips() {
        // §4.4's layer-pinnedness caution: `Add`'s tentacle order holds the η
        // at λ ≥ 1 even though `ceil = 1`.
        let e = Free::compose(
            Free::compose(
                prim(SfgGenerator::Copy),
                Free::tensor(
                    PropExpr::Identity(1),
                    Free::tensor(prim(SfgGenerator::Zero), PropExpr::Identity(1)),
                ),
            )
            .expect("Copy : 1 → 2 ; (id₁ ⊗ Zero ⊗ id₁) : 2 → 3"),
            Free::tensor(PropExpr::Identity(1), prim(SfgGenerator::Add)),
        )
        .expect("(…) : 1 → 3 ; (id₁ ⊗ Add) : 3 → 2");
        round_trips(&e);
        assert_eq!(canonical_display(&e), nf(&e));

        // The point of the case is the layer, so assert it: the η is *not* in
        // layer 0 — `Copy` is — and sits in layer 1 strictly inside `Copy`'s
        // output span, which is what holds it below layer 0. The whole layer is
        // asserted, not just the η's atom index, so the "inside the span" claim
        // is pinned rather than implied by the identity to its left.
        let display = canonical_display(&e);
        assert_eq!(
            display.layers[0].atoms,
            vec![Atom::Generator(SfgGenerator::Copy)]
        );
        assert_eq!(
            display.layers[1].atoms,
            vec![
                Atom::Identity(1),
                Atom::Generator(SfgGenerator::Zero),
                Atom::Identity(1),
            ]
        );
    }

    #[test]
    fn an_output_anchored_eta_lands_at_its_boundary_coordinate() {
        // No consumer pins the η: its coordinate comes from the output anchor,
        // so the two writings must place it on opposite sides of the wire.
        let eta = Atom::Generator(SfgGenerator::Zero);

        let e = Free::tensor(prim(SfgGenerator::Zero), PropExpr::Identity(1));
        round_trips(&e);
        assert_eq!(canonical_display(&e), nf(&e));
        assert_eq!(
            canonical_display(&e).layers[0].atoms,
            vec![eta.clone(), Atom::Identity(1)],
            "output coordinate 0 puts the η left of the wire"
        );

        let e = Free::tensor(PropExpr::Identity(1), prim(SfgGenerator::Zero));
        round_trips(&e);
        assert_eq!(canonical_display(&e), nf(&e));
        assert_eq!(
            canonical_display(&e).layers[0].atoms,
            vec![Atom::Identity(1), eta],
            "output coordinate 1 puts it right of the wire"
        );
    }

    #[test]
    fn a_content_with_no_braid_free_writing_gets_its_crossing() {
        // Copy emits (u, v); the swap makes Add consume (v, u).
        let e = Free::compose(
            Free::compose(prim(SfgGenerator::Copy), PropExpr::Braid(1, 1))
                .expect("Copy : 1 → 2 ; σ : 2 → 2"),
            prim(SfgGenerator::Add),
        )
        .expect("(Copy ; σ) : 1 → 2 ; Add : 2 → 1");
        round_trips(&e);
        assert!(has_braid(&canonical_display(&e)));
        assert_eq!(canonical_display(&e), nf(&e));
    }

    #[test]
    fn a_dead_braid_prefix_does_not_survive_the_readback() {
        // §4.4's `braid_prefix_is_not_content_derived`: `nf` keeps the dead
        // permutation, the display cannot — content does not record it.
        let discard = prim(SfgGenerator::Discard);
        let pair = Free::tensor(discard.clone(), discard);
        let braided = Free::compose(PropExpr::Braid(1, 1), pair.clone()).expect("σ ; (ε ⊗ ε)");
        assert_ne!(nf(&braided), nf(&pair), "the §4.4 witness still separates");
        assert_eq!(canonical_display(&braided), canonical_display(&pair));
        assert!(!has_braid(&canonical_display(&braided)));
    }

    /// A **layer-pinned** `η` whose content still admits two
    /// invariant-satisfying braid-free realizations, at `λ = 0` and `λ = 1`: one
    /// content, two realizable layers, so there need not be a forced layer. The
    /// `η`'s wire sits at coordinate 2 of `W₁` under `nf` and coordinate 1 of
    /// `W₂` in the display, so the content is slot-slack and outside `𝔉′` —
    /// neither a counterexample to Theorem 4.5 nor to §4.4's induction step (a),
    /// both of which hypothesize slot-pinnedness.
    #[test]
    fn a_layer_pinned_eta_can_still_take_two_layers() {
        let e = Free::compose(
            Free::compose(
                prim(SfgGenerator::Copy),
                Free::tensor(
                    Free::tensor(PropExpr::Identity(1), prim(SfgGenerator::Discard)),
                    prim(SfgGenerator::Zero),
                ),
            )
            .expect("Copy : 1 → 2 ; (id₁ ⊗ Discard ⊗ Zero) : 2 → 2"),
            prim(SfgGenerator::Add),
        )
        .expect("(…) : 1 → 2 ; Add : 2 → 1");

        let written = nf(&e);
        let displayed = canonical_display(&e);
        assert_ne!(written, displayed);
        // Both are `nf` fixpoints of SMC-equal writings, both braid-free.
        assert_eq!(nf(&readback(&e)), displayed);
        assert!(!has_braid(&written) && !has_braid(&displayed));
        // The `η` sits in layer 0 in one and layer 1 in the other.
        let eta = Atom::Generator(SfgGenerator::Zero);
        assert!(written.layers[0].atoms.contains(&eta));
        assert!(displayed.layers[1].atoms.contains(&eta));
    }

    #[test]
    fn an_empty_content_reads_back_as_the_tensor_unit() {
        let e = Free::<Sfg>::tensor(PropExpr::Identity(0), PropExpr::Identity(0));
        assert_eq!(readback(&e), PropExpr::Identity(0));
        round_trips(&e);
    }

    #[test]
    fn smc_equal_writings_of_a_wide_diagram_agree() {
        // Interchange: (f ⊗ id) ; (id ⊗ g) vs (id ⊗ g) ; (f ⊗ id).
        let f = prim(SfgGenerator::Copy);
        let g = prim(SfgGenerator::Discard);
        let left = Free::compose(
            Free::tensor(f.clone(), PropExpr::Identity(1)),
            Free::tensor(PropExpr::Identity(2), g.clone()),
        )
        .expect("(Copy ⊗ id₁) : 2 → 3 ; (id₂ ⊗ Discard) : 3 → 2");
        let right = Free::compose(
            Free::tensor(PropExpr::Identity(1), g),
            Free::tensor(f, PropExpr::Identity(0)),
        )
        .expect("(id₁ ⊗ Discard) : 2 → 1 ; (Copy ⊗ id₀) : 1 → 2");
        assert!(content_eq(&content_of(&left), &content_of(&right)));
        assert_eq!(canonical_display(&left), canonical_display(&right));
    }

    /// The source-0 shapes §4.4's definedness bullet leaves outside `𝔉′` — a
    /// `0 → 0` scalar and a `0 → n` with `n ≥ 2` — over a signature declared here
    /// because no shipped one has either.
    mod exotic_shapes {
        use super::*;

        #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        enum Exotic {
            /// `0 → 0`.
            Nil,
            /// `0 → 2`.
            Fork,
            /// `2 → 0`.
            Join,
            /// `1 → 1`.
            Wire,
        }

        impl PropSignature for Exotic {
            type Color = ();
            fn source_word(&self) -> std::borrow::Cow<'_, [()]> {
                std::borrow::Cow::Owned(vec![(); self.source()])
            }
            fn target_word(&self) -> std::borrow::Cow<'_, [()]> {
                std::borrow::Cow::Owned(vec![(); self.target()])
            }
            fn source(&self) -> usize {
                match self {
                    Exotic::Nil | Exotic::Fork => 0,
                    Exotic::Join => 2,
                    Exotic::Wire => 1,
                }
            }
            fn target(&self) -> usize {
                match self {
                    Exotic::Nil | Exotic::Join => 0,
                    Exotic::Fork => 2,
                    Exotic::Wire => 1,
                }
            }
        }

        type X = PropExpr<Exotic>;

        fn round_trips(e: &X) {
            let c = content_of(e);
            assert!(
                content_eq(&content_of(&expr_of_content(&c)), &c),
                "readback changed the content of {e:?}"
            );
        }

        #[test]
        fn a_zero_by_zero_scalar_round_trips() {
            let e = X::Generator(Exotic::Nil);
            round_trips(&e);
            assert_eq!(expr_of_content(&content_of(&e)), X::Generator(Exotic::Nil));

            // Two of them, and a wire beside: three components, one anchored.
            let two = PropExpr::Tensor(
                Box::new(X::Generator(Exotic::Nil)),
                Box::new(PropExpr::Tensor(
                    Box::new(X::Generator(Exotic::Wire)),
                    Box::new(X::Generator(Exotic::Nil)),
                )),
            );
            round_trips(&two);
        }

        #[test]
        fn a_source_zero_hyperedge_with_two_outputs_round_trips() {
            round_trips(&X::Generator(Exotic::Fork));
            round_trips(&PropExpr::Compose(
                Box::new(X::Generator(Exotic::Fork)),
                Box::new(X::Generator(Exotic::Join)),
            ));
        }

        #[test]
        fn a_split_fork_round_trips() {
            // `Join` takes the through-wire and the fork's first output; the
            // second leaves by the output foot.
            let e = PropExpr::Compose(
                Box::new(PropExpr::Tensor(
                    Box::new(X::Generator(Exotic::Wire)),
                    Box::new(X::Generator(Exotic::Fork)),
                )),
                Box::new(PropExpr::Tensor(
                    Box::new(X::Generator(Exotic::Join)),
                    Box::new(X::Generator(Exotic::Wire)),
                )),
            );
            round_trips(&e);
        }

        #[test]
        fn a_fork_consumed_in_reverse_round_trips() {
            // `Join` wants `(f₁, f₀)` — no braid-free writing exists.
            let e = PropExpr::Compose(
                Box::new(PropExpr::Compose(
                    Box::new(X::Generator(Exotic::Fork)),
                    Box::new(PropExpr::Braid(1, 1)),
                )),
                Box::new(X::Generator(Exotic::Join)),
            );
            round_trips(&e);
        }
    }
}
