//! Bounded congruence-closure decision procedure for [`super::Presentation`]-modulo equality.
//!
//! Given a term graph over [`PropExpr<G>`] and a seed set of equations,
//! computes the smallest congruence relation containing the seed, then
//! answers `are_equal` queries by union-find root comparison.
//!
//! Based on the Downey-Sethi-Tarjan 1980 algorithm using a signature table
//! indexed by canonical child-class IDs. Correct for finitely-presented
//! equational theories without binders. This engine is **not** full
//! Knuth-Bendix completion with critical-pair discovery — it seeds a term graph
//! with the user's equations as-is, then propagates congruence through
//! `Compose` / `Tensor`. On the 18 F&S Thm 5.60 equations, which present Mat(R)
//! (Baez-Erbele 2015 for fields; Wadsley–Woods arXiv:1505.00048 for commutative
//! rigs, cf. BE15 §6), congruence closure with this seed decides Mat(R)-equality
//! on SFG expressions.
//!
//! # Complexity
//!
//! Per `are_equal` query: term insertion `O(|a| + |b|)` expected, assuming `O(1)`
//! hash operations on the term / signature tables; congruence propagation
//! amortized `O(n · α(n))` total across all merges, `α` the inverse-Ackermann
//! function.
//!
//! # References
//!
//! * P. J. Downey, R. Sethi, R. E. Tarjan. *Variations on the Common
//!   Subexpression Problem*. J. ACM 27(4), 1980.
//! * J. Baez, J. Erbele. *Categories in Control*. Theory and Applications
//!   of Categories 30, 2015. (Theorem 2 is field-only; §6 attributes the
//!   commutative-rig generalization to Wadsley–Woods.)
//! * S. Wadsley, N. Woods. *PROPs for Linear Systems*. arXiv:1505.00048,
//!   2015. (Mat(k) is the PROP for bicommutative bimonoids over any
//!   commutative rig k — the completeness result for the rigs used here.)

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use super::super::{PropExpr, PropSignature};

/// Kind ordinal for atom-canonical preference (lowest wins). Kinds 3 and 4
/// (`Compose`, `Tensor`) are the composites.
fn node_kind<G: PropSignature>(node: &Node<G>) -> u8 {
    match node {
        Node::Identity(_) => 0,
        Node::Braid(_, _) => 1,
        Node::Generator(_) => 2,
        Node::Compose(_, _) => 3,
        Node::Tensor(_, _) => 4,
    }
}

/// Lift an atom [`Node`] (`Identity`, `Braid`, or `Generator`) to the equivalent
/// [`PropExpr`]. Panics on composite kinds.
fn atom_node_to_expr<G: PropSignature>(node: Node<G>) -> PropExpr<G> {
    match node {
        Node::Identity(n) => PropExpr::Identity(n),
        Node::Braid(m, n) => PropExpr::Braid(m, n),
        Node::Generator(g) => PropExpr::Generator(g),
        Node::Compose(_, _) | Node::Tensor(_, _) => {
            unreachable!("atom_node_to_expr called on composite node")
        }
    }
}

/// Internal term ID — dense index into the term graph. `pub` only under the
/// `internal-bench` feature; private otherwise.
#[cfg(feature = "internal-bench")]
pub type TermId = usize;
#[cfg(not(feature = "internal-bench"))]
type TermId = usize;

/// Tag distinguishing function-symbol constructor variants for congruence
/// propagation. Atoms (`Identity`, `Braid`, `Generator`) never propagate
/// congruence, so only these two tags occur as signature-table keys or in the
/// `uses` index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Tag {
    Compose,
    Tensor,
}

/// A term-graph node. `pub` only under the `internal-bench` feature; private
/// otherwise.
#[cfg(feature = "internal-bench")]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Node<G>
where
    G: Clone + PartialEq + Eq + Hash,
{
    Identity(usize),
    Braid(usize, usize),
    Generator(G),
    Compose(TermId, TermId),
    Tensor(TermId, TermId),
}

#[cfg(not(feature = "internal-bench"))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Node<G>
where
    G: Clone + PartialEq + Eq + Hash,
{
    Identity(usize),
    Braid(usize, usize),
    Generator(G),
    Compose(TermId, TermId),
    Tensor(TermId, TermId),
}

/// Congruence-closure engine seeded with a fixed set of equations, answering
/// [`Self::are_equal`] queries over [`PropExpr<G>`]. Equality is **modulo the
/// seeded equations only** — associativity, unitality, interchange, braiding
/// naturality, and other SMC axioms are *not* assumed unless explicitly seeded;
/// an SMC-aware decision procedure needs the 18 F&S Thm 5.60 equations
/// pre-seeded (Baez-Erbele 2015 for fields, Wadsley–Woods arXiv:1505.00048 for
/// commutative rigs, cf. BE15 §6).
pub struct CongruenceClosure<G>
where
    G: PropSignature,
{
    /// Canonical term-graph lookup: structural `Node` → `TermId`. Ensures
    /// structurally-identical sub-terms share a single ID on insertion.
    nodes: HashMap<Node<G>, TermId>,
    /// Inverse map for each fresh `TermId`: the `Node` it was created from.
    reverse: Vec<Node<G>>,
    /// Union-find parent pointers; `parent[i] == i` iff `i` is a class root.
    parent: Vec<TermId>,
    /// Per-class uses list: for each class root `c`, every function-symbol node
    /// `f(a, b)` with `find(a) == c` or `find(b) == c`, as
    /// `(term_id, other_arg_id, constructor_tag)`. Entries may become stale
    /// (refer to non-root IDs) after subsequent merges — re-canonicalized on use.
    uses: Vec<Vec<(TermId, TermId, Tag)>>,
    /// Signature table keyed on `(Tag, find(arg_a), find(arg_b))`, mapping to the
    /// canonical representative of the corresponding congruence class.
    signatures: HashMap<(Tag, TermId, TermId), TermId>,
    /// LIFO worklist of pending `(losing_root, winning_root)` pairs awaiting
    /// propagation.
    pending: Vec<(TermId, TermId)>,
}

impl<G> CongruenceClosure<G>
where
    G: PropSignature,
{
    /// Build a new engine seeded with the given equations: each equation's LHS
    /// and RHS are inserted into the term graph and their classes merged, then
    /// congruence is propagated to fixpoint interleaved with post-merge SMC
    /// normalization.
    #[must_use]
    pub fn new(equations: &[(PropExpr<G>, PropExpr<G>)]) -> Self {
        let mut engine = Self {
            nodes: HashMap::new(),
            reverse: Vec::new(),
            parent: Vec::new(),
            uses: Vec::new(),
            signatures: HashMap::new(),
            pending: Vec::new(),
        };
        let mut seed_pairs = Vec::with_capacity(equations.len());
        for (lhs, rhs) in equations {
            let l = engine.add_term(lhs);
            let r = engine.add_term(rhs);
            seed_pairs.push((l, r));
        }
        for (l, r) in seed_pairs {
            engine.merge(l, r);
        }
        engine.propagate_fixpoint();
        engine
    }

    /// Test equality of two terms modulo the seeded equations. May extend the
    /// term graph with previously unseen sub-terms; after any such extension
    /// congruence is re-propagated, so the verdict stays consistent with the
    /// seeded theory.
    #[must_use]
    pub fn are_equal(&mut self, a: &PropExpr<G>, b: &PropExpr<G>) -> bool {
        let a_id = self.add_term(a);
        let b_id = self.add_term(b);
        self.propagate_fixpoint();
        self.find(a_id) == self.find(b_id)
    }

    /// Returns the preferred atom (lowest kind, then smallest [`TermId`]) of
    /// `id`'s union-find class, or `None` if the class holds no atom member.
    /// Available only under the `internal-bench` feature.
    ///
    /// **NOT public API.** May be removed or change semantics in any release
    /// without a `SemVer` guarantee.
    #[cfg(feature = "internal-bench")]
    pub fn atom_canonical_for_bench(&mut self, id: TermId) -> Option<Node<G>> {
        self.atom_canonical(id)
    }

    /// Hash-conses a [`PropExpr<G>`] into the engine's term graph and returns
    /// the resulting [`TermId`]. Available only under the `internal-bench`
    /// feature.
    ///
    /// **NOT public API.** Same `SemVer` non-guarantee as
    /// [`Self::atom_canonical_for_bench`].
    #[cfg(feature = "internal-bench")]
    pub fn add_term_for_bench(&mut self, expr: &PropExpr<G>) -> TermId {
        self.add_term(expr)
    }

    /// Add a term to the graph, returning its ID. Structural hash-cons:
    /// identical `Node` shapes share an ID. Function-symbol nodes
    /// (`Compose` / `Tensor`) additionally probe the signature table against the
    /// class-roots of their children — a congruent existing node causes a merge.
    /// Recurses on children.
    fn add_term(&mut self, expr: &PropExpr<G>) -> TermId {
        let node = match expr {
            PropExpr::Identity(n) => Node::Identity(*n),
            PropExpr::Braid(m, n) => Node::Braid(*m, *n),
            PropExpr::Generator(g) => Node::Generator(g.clone()),
            PropExpr::Compose(f, g) => {
                let f_id = self.add_term(f);
                let g_id = self.add_term(g);
                Node::Compose(f_id, g_id)
            }
            PropExpr::Tensor(f, g) => {
                let f_id = self.add_term(f);
                let g_id = self.add_term(g);
                Node::Tensor(f_id, g_id)
            }
        };
        if let Some(&id) = self.nodes.get(&node) {
            return id;
        }
        let id = self.reverse.len();
        self.parent.push(id);
        self.uses.push(Vec::new());
        self.reverse.push(node.clone());
        self.nodes.insert(node.clone(), id);

        // Register uses and probe signature table for function-symbol nodes.
        match node {
            Node::Compose(a, b) => self.install_function_node(id, a, b, Tag::Compose),
            Node::Tensor(a, b) => self.install_function_node(id, a, b, Tag::Tensor),
            _ => {}
        }
        id
    }

    /// Register a freshly-inserted function node in its children's uses
    /// lists and in the signature table. If the signature collides with an
    /// existing class representative, enqueue a merge.
    //
    // `ra`/`rb` and `ra_post`/`rb_post` pair the pre- and post-merge class roots.
    #[allow(clippy::similar_names)]
    fn install_function_node(&mut self, id: TermId, a: TermId, b: TermId, tag: Tag) {
        let ra = self.find(a);
        let rb = self.find(b);
        self.uses[ra].push((id, b, tag));
        if ra != rb {
            self.uses[rb].push((id, a, tag));
        }
        if let Some(existing) = self.signatures.insert((tag, ra, rb), id) {
            // Signature collision: `existing` already represents the congruence
            // class of `(tag, ra, rb)`. `merge`'s link direction is
            // implementation-defined, so re-canonicalize key and value via `find`.
            self.merge(id, existing);
            let (ra_post, rb_post, root_post) = (self.find(a), self.find(b), self.find(existing));
            self.signatures.insert((tag, ra_post, rb_post), root_post);
        }
    }

    /// Union-find root with path halving.
    fn find(&mut self, mut id: TermId) -> TermId {
        while self.parent[id] != id {
            let next = self.parent[id];
            self.parent[id] = self.parent[next]; // path halving
            id = next;
        }
        id
    }

    /// Merge two classes. If they are already unified this is a no-op.
    /// Otherwise the first argument's root is linked to the second argument's
    /// root — ordering is determined by the caller, not by ID comparison — and
    /// the pair is queued for propagation via [`Self::propagate`], which re-files
    /// uses from the losing root into the winning root's list.
    fn merge(&mut self, a: TermId, b: TermId) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            return;
        }
        // Link ra's root onto rb's root. Record the losing-side root
        // so propagation knows which uses list to walk.
        self.parent[ra] = rb;
        self.pending.push((ra, rb));
    }

    /// Drive congruence propagation interleaved with post-merge SMC
    /// normalization to fixpoint. Each iteration does a full [`Self::propagate`]
    /// drain followed by a [`Self::smc_refine`] pass; the loop stops when
    /// `smc_refine` reports no new merges.
    ///
    /// # Termination
    ///
    /// Each effective [`Self::smc_refine`] pass strictly decreases the number of
    /// equivalence classes, which is bounded below by 1, so the loop terminates
    /// after finitely many iterations; `SAFETY_BOUND` is a defense-in-depth cap.
    fn propagate_fixpoint(&mut self) {
        const SAFETY_BOUND: usize = 64;
        for _ in 0..SAFETY_BOUND {
            self.propagate();
            if !self.smc_refine() {
                return;
            }
        }
        // Safety bound exhausted — finish pending propagation and return.
        self.propagate();
    }

    /// Post-merge SMC refinement pass. For each currently-existing term,
    /// rebuilds its [`PropExpr`] using *atom-canonical* substitutions (see
    /// [`Self::atom_canonical`]) at every sub-term position whose class contains
    /// an atom, runs [`smc_nf::nf`] on the result, folds back via
    /// [`smc_nf::from_string_diagram`], and merges the NF into the term's class
    /// if it differs. Returns `true` iff any new merge was performed.
    fn smc_refine(&mut self) -> bool {
        let term_count = self.reverse.len();
        let mut pairs: Vec<(TermId, PropExpr<G>)> = Vec::with_capacity(term_count);

        for id in 0..term_count {
            let canon_expr = self.term_to_canonical_expr(id);
            let nf_sd = super::smc_nf::nf(&canon_expr);
            let nf_expr = super::smc_nf::from_string_diagram(&nf_sd);
            if nf_expr != canon_expr {
                pairs.push((id, nf_expr));
            }
        }

        let mut progress = false;
        for (id, nf_expr) in pairs {
            let new_id = self.add_term(&nf_expr);
            // `add_term` may already have merged `new_id` via signature collision.
            if self.find(id) != self.find(new_id) {
                self.merge(id, new_id);
                progress = true;
            }
        }

        progress
    }

    /// Rebuild a [`PropExpr`] for `id`, substituting an *atom-canonical*
    /// representative (see [`Self::atom_canonical`]) at every sub-term position
    /// whose class contains any atom — including composite (`Compose`/`Tensor`)
    /// positions, since a composite may share a class with an atom. Output size
    /// is bounded by the input term's size: substitution only ever shrinks the
    /// tree, and recursion happens only when the class holds no atom, descending
    /// into strictly-smaller child [`TermId`]s.
    fn term_to_canonical_expr(&mut self, id: TermId) -> PropExpr<G> {
        // Atom-for-anything substitution first; recurse only when the class has
        // no atom representative.
        if let Some(atom_node) = self.atom_canonical(id) {
            return atom_node_to_expr(atom_node);
        }
        let node = self.reverse[id].clone();
        match node {
            Node::Identity(n) => PropExpr::Identity(n),
            Node::Braid(m, n) => PropExpr::Braid(m, n),
            Node::Generator(g) => PropExpr::Generator(g),
            Node::Compose(a, b) => {
                let a_expr = self.term_to_canonical_expr(a);
                let b_expr = self.term_to_canonical_expr(b);
                PropExpr::Compose(Box::new(a_expr), Box::new(b_expr))
            }
            Node::Tensor(a, b) => {
                let a_expr = self.term_to_canonical_expr(a);
                let b_expr = self.term_to_canonical_expr(b);
                PropExpr::Tensor(Box::new(a_expr), Box::new(b_expr))
            }
        }
    }

    /// Scan `id`'s union-find class for any atom member (`Identity`, `Braid`, or
    /// `Generator`). Returns the preferred atom (lowest kind, then smallest
    /// [`TermId`]) if one exists, else `None`; composite members (`Compose`,
    /// `Tensor`) are ignored.
    fn atom_canonical(&mut self, id: TermId) -> Option<Node<G>> {
        let root = self.find(id);
        let mut best: Option<(u8, TermId)> = None;
        for candidate in 0..self.reverse.len() {
            let kind = node_kind(&self.reverse[candidate]);
            if kind >= 3 {
                continue; // Compose / Tensor — not an atom.
            }
            if self.find(candidate) != root {
                continue;
            }
            let key = (kind, candidate);
            if best.is_none_or(|b| key < b) {
                best = Some(key);
            }
        }
        best.map(|(_, idx)| self.reverse[idx].clone())
    }

    /// Drain the pending worklist, re-probing the signature table for
    /// every function node in each losing-class's uses list. If a signature
    /// now collides with an existing class representative, merge those and
    /// enqueue again; otherwise update the table with the new canonical
    /// signature. Terminates because each effective merge reduces the
    /// number of equivalence classes by 1.
    fn propagate(&mut self) {
        while let Some((losing_root, _winning_root)) = self.pending.pop() {
            let losing_uses = std::mem::take(&mut self.uses[losing_root]);
            for (term, _other, tag) in losing_uses {
                // The `other` component may be stale, so read `term`'s literal
                // children from `reverse` and re-canonicalize them via `find`.
                let (Node::Compose(a, b) | Node::Tensor(a, b)) = self.reverse[term] else {
                    unreachable!(
                        "non-function node in uses list (Generator/Identity/Braid never register uses)"
                    )
                };
                let ra = self.find(a);
                let rb = self.find(b);
                let key = (tag, ra, rb);

                match self.signatures.get(&key).copied() {
                    Some(canonical) if self.find(canonical) != self.find(term) => {
                        self.merge(term, canonical);
                    }
                    Some(_) => {
                        // Already canonical for this signature; nothing to do.
                    }
                    None => {
                        self.signatures.insert(key, term);
                    }
                }

                // Re-file the use under the winning root of each child.
                let root_a = self.find(a);
                let root_b = self.find(b);
                self.uses[root_a].push((term, b, tag));
                if root_a != root_b {
                    self.uses[root_b].push((term, a, tag));
                }
            }
        }
    }
}
