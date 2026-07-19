//! Bounded congruence-closure decision procedure for [`super::Presentation`]-modulo equality.
//!
//! Given a term graph over [`PropExpr<G>`] and a seed set of equations,
//! computes the smallest congruence relation containing the seed, then
//! answers `are_equal` queries by union-find root comparison.
//!
//! Based on the Downey-Sethi-Tarjan 1980 algorithm using a signature-table
//! indexed by canonical child-class IDs. Correct for finitely-presented
//! equational theories without binders; the 18 F&S Thm 5.60 equations present
//! Mat(R) (proof via Baez-Erbele 2015 for fields; the commutative-rig case —
//! which covers all four rigs instantiated in this crate — is Wadsley–Woods
//! arXiv:1505.00048, cf. BE15 §6).
//!
//! This engine is **not** full Knuth-Bendix completion with critical-pair
//! discovery — it seeds a term graph with the user's equations as-is, then
//! propagates congruence through `Compose` / `Tensor`. For a confluent,
//! terminating rewrite system, this is a decision procedure. For the 18
//! Thm 5.60 equations specifically, F&S Thm 5.60 presents Mat(R) (proof via
//! Baez-Erbele 2015 for fields, Wadsley–Woods arXiv:1505.00048 for commutative
//! rigs, cf. BE15 §6), so congruence closure with this seed decides
//! Mat(R)-equality on SFG expressions.
//!
//! # Algorithm sketch
//!
//! The term graph hash-conses every sub-term to a `TermId`. A signature
//! table keyed on `(Tag, root_class(arg_a), root_class(arg_b))` records the
//! canonical representative of each function-node congruence class. On
//! `add_term` we probe this table: if a match exists, the new term is
//! immediately merged into the existing class. On `merge` we walk the uses
//! lists of the smaller class, re-probing each function node's signature
//! against the (now updated) table — any collision produces another merge,
//! propagating congruence to fixpoint.
//!
//! # Complexity
//!
//! Per `are_equal` query:
//! - Term insertion: `O(|a| + |b|)` expected, assuming `O(1)` hash
//!   operations on the term / signature tables.
//! - Congruence propagation: amortized `O(n · α(n))` total across all
//!   merges, where `α` is the inverse-Ackermann function (from union-find
//!   with path halving).
//!
//! With a sorted-pair signature representation (as in the original DST
//! paper) the `α(n)` bound extends to per-insertion; we trade that for
//! the average-case simplicity of a hash table.
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

/// Kind ordinal for atom-canonical preference (lowest wins). Used by
/// [`CongruenceClosure::atom_canonical`] to bias toward identity/braid
/// atoms when surfacing a class's atom representative. Kinds 3 and 4
/// (`Compose`, `Tensor`) correspond to composites and are skipped by the
/// scan.
fn node_kind<G: PropSignature>(node: &Node<G>) -> u8 {
    match node {
        Node::Identity(_) => 0,
        Node::Braid(_, _) => 1,
        Node::Generator(_) => 2,
        Node::Compose(_, _) => 3,
        Node::Tensor(_, _) => 4,
    }
}

/// Lift an atom [`Node`] (one of `Identity`, `Braid`, `Generator`) to the
/// equivalent [`PropExpr`]. Panics on composite kinds — callers guarantee
/// atom-only input by filtering in [`CongruenceClosure::atom_canonical`].
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

/// Internal term ID — dense index into the term graph.
///
/// Type-aliased to `pub` only under the `internal-bench` feature flag (see
/// [`CongruenceClosure::atom_canonical_for_bench`]); private in default
/// builds (zero overhead, no public surface delta).
#[cfg(feature = "internal-bench")]
pub type TermId = usize;
#[cfg(not(feature = "internal-bench"))]
type TermId = usize;

/// Tag distinguishing function-symbol constructor variants for congruence
/// propagation. Atoms (`Identity`, `Braid`, `Generator`) never propagate
/// congruence — only `Compose` and `Tensor` do — so only these two tags
/// occur as signature-table keys or in the `uses` index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Tag {
    Compose,
    Tensor,
}

/// A term-graph node. The `Generator(G)` variant constrains `G: Eq + Hash`
/// via the unconditional derives below; all other variants have `usize` or
/// `TermId` children, so derivation works uniformly once `G` satisfies the
/// required bounds.
///
/// Widened to `pub` only under the `internal-bench` feature flag (see
/// [`CongruenceClosure::atom_canonical_for_bench`]); private in default
/// builds (zero overhead, no public surface delta).
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

/// Congruence-closure engine seeded with a fixed set of equations.
///
/// After construction the engine is ready to answer [`Self::are_equal`]
/// queries. Queries may extend the term graph with previously unseen
/// sub-terms; the engine re-probes the signature table on insertion, so
/// query results remain consistent with the seeded equations.
///
/// Equality is **modulo the seeded equations only** — associativity,
/// unitality, interchange, braiding naturality, and other SMC axioms are
/// *not* assumed unless explicitly seeded. Callers that need an SMC-aware
/// decision procedure should pre-seed the 18 Thm 5.60 equations (F&S Thm 5.60;
/// proof via Baez-Erbele 2015 for fields, Wadsley–Woods arXiv:1505.00048 for
/// commutative rigs, cf. BE15 §6).
pub struct CongruenceClosure<G>
where
    G: PropSignature,
{
    /// Canonical term-graph lookup: structural `Node` → `TermId`. Ensures
    /// structurally-identical sub-terms share a single ID on insertion.
    nodes: HashMap<Node<G>, TermId>,
    /// Inverse map for each fresh `TermId`: the `Node` it was created from.
    /// Read by `propagate` to re-canonicalize a function node's children
    /// via `find` after a merge has potentially invalidated the IDs
    /// recorded in the `uses` list.
    reverse: Vec<Node<G>>,
    /// Union-find parent pointers; `parent[i] == i` iff `i` is a class root.
    parent: Vec<TermId>,
    /// Per-class uses list: for each class root `c`, records every
    /// function-symbol node `f(a, b)` with `find(a) == c` or `find(b) == c`,
    /// as `(term_id, other_arg_id, constructor_tag)`. Scanned during merge
    /// propagation to re-probe signatures. Entries may become stale (refer
    /// to non-root IDs) after subsequent merges — we re-canonicalize on use.
    uses: Vec<Vec<(TermId, TermId, Tag)>>,
    /// Signature table keyed on `(Tag, find(arg_a), find(arg_b))`, mapping
    /// to the canonical representative of the corresponding congruence
    /// class. New function nodes probe this table on insertion.
    signatures: HashMap<(Tag, TermId, TermId), TermId>,
    /// LIFO worklist (stack) of pending `(ra, rb)` root pairs awaiting
    /// propagation. DST terminates under any worklist order, so a stack
    /// via `Vec::pop` is fine.
    pending: Vec<(TermId, TermId)>,
}

impl<G> CongruenceClosure<G>
where
    G: PropSignature,
{
    /// Build a new engine seeded with the given equations.
    ///
    /// Each equation's LHS and RHS are inserted into the term graph and
    /// their classes merged. Congruence is then propagated to fixpoint,
    /// interleaved with post-merge SMC normalization (the internal
    /// `propagate_fixpoint` method).
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

    /// Test equality of two terms modulo the seeded equations.
    ///
    /// May extend the term graph with previously unseen sub-terms; after
    /// any such extension, congruence is re-propagated so the returned
    /// verdict is consistent with the seeded theory.
    #[must_use]
    pub fn are_equal(&mut self, a: &PropExpr<G>, b: &PropExpr<G>) -> bool {
        let a_id = self.add_term(a);
        let b_id = self.add_term(b);
        self.propagate_fixpoint();
        self.find(a_id) == self.find(b_id)
    }

    /// Feature-gated public shim exposing the private `atom_canonical`
    /// selection logic for benchmark fixtures only. Available exclusively
    /// under the `internal-bench` feature flag.
    ///
    /// **NOT public API.** This shim exists solely to bench the O(n²)/n³
    /// atom-canonical work + provide quantitative signal for the KB-vs-Functorial
    /// decision (resolved: functorial-terminal, issue #15; KB feasibility spike
    /// #57).
    /// Downstream consumers MUST NOT enable `internal-bench` in production
    /// builds; the method may be removed or its semantics changed in any
    /// release without `SemVer` guarantee.
    ///
    /// Returns the preferred atom (lowest-kind, smallest [`TermId`]) of `id`'s
    /// union-find class, or `None` if the class contains no atom members.
    /// The underlying selection picks the lowest-`Node::kind_rank` member of
    /// the class (atoms preferred over compound function nodes), breaking
    /// ties by smallest `TermId`.
    ///
    /// Under `--features internal-bench`, [`TermId`] (an alias for `usize`)
    /// and [`Node`] are widened to `pub` so this shim's signature is
    /// expressible across the crate boundary. Pair with
    /// [`Self::add_term_for_bench`] to insert a [`PropExpr<G>`] into the
    /// term graph and obtain its [`TermId`] in one call.
    #[cfg(feature = "internal-bench")]
    pub fn atom_canonical_for_bench(&mut self, id: TermId) -> Option<Node<G>> {
        self.atom_canonical(id)
    }

    /// Feature-gated public shim that hash-conses a [`PropExpr<G>`] into the
    /// engine's term graph and returns the resulting [`TermId`]. Pairs with
    /// [`Self::atom_canonical_for_bench`] for the
    /// `presentation::atom_canonical_n*` bench group.
    ///
    /// **NOT public API.** Same `SemVer` non-guarantee as
    /// [`Self::atom_canonical_for_bench`].
    #[cfg(feature = "internal-bench")]
    pub fn add_term_for_bench(&mut self, expr: &PropExpr<G>) -> TermId {
        self.add_term(expr)
    }

    /// Add a term to the graph, returning its ID.
    ///
    /// Structural hash-cons: identical `Node` shapes share an ID. For
    /// function-symbol nodes (`Compose` / `Tensor`) we additionally probe
    /// the signature table against the class-roots of the children — if a
    /// congruent function node already exists, the new node is merged with
    /// it. Recurses on children.
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
    // `ra`/`rb`/`ra_post`/`rb_post` are an intentional paired naming for the
    // pre-merge and post-merge versions of the children's class roots;
    // renaming away the similarity harms readability.
    #[allow(clippy::similar_names)]
    fn install_function_node(&mut self, id: TermId, a: TermId, b: TermId, tag: Tag) {
        let ra = self.find(a);
        let rb = self.find(b);
        self.uses[ra].push((id, b, tag));
        if ra != rb {
            self.uses[rb].push((id, a, tag));
        }
        if let Some(existing) = self.signatures.insert((tag, ra, rb), id) {
            // Signature collision: `existing` already represents the
            // congruence class of (tag, ra, rb). Merge the two, then
            // store the *post-merge* canonical root — `merge` links one
            // root onto the other but the direction is implementation-
            // defined, so we must re-canonicalize via `find`.
            //
            // Defensive: re-canonicalize the key via `find(a) / find(b)`
            // in addition to the value. Today `merge(id, existing)` only
            // unions `{id, existing}` — it cannot shift the children's
            // roots `ra`/`rb`, so the key is invariant. Recomputing it
            // anyway means any future refactor that moves merges into
            // `install_function_node` (or that reorders the recursion)
            // cannot silently leak a stale signature key past this point.
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
    /// Otherwise the first argument's root is linked to the second
    /// argument's root — ordering is determined by the caller, not by
    /// ID comparison — and the pair is queued for propagation via
    /// [`Self::propagate`].
    ///
    /// We don't union-by-rank here because per-class uses lists
    /// dominate cost and aren't tied to the root choice; `propagate`
    /// is responsible for re-filing uses from the losing root into
    /// the winning root's list.
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
    /// normalization to fixpoint.
    ///
    /// Each iteration does a full [`Self::propagate`] drain (congruence
    /// closure) followed by a [`Self::smc_refine`] pass (SMC string-diagram
    /// normal form applied to each term under class-canonical substitution).
    /// The loop terminates when `smc_refine` reports no new merges.
    ///
    /// # Why this is needed
    ///
    /// User equations may merge classes whose members are SMC-atoms (e.g.,
    /// `Scalar(R::one())` and `Identity(1)`). After such a merge, terms like
    /// `Tensor(Identity(3), Scalar(1))` are congruent to
    /// `Tensor(Identity(3), Identity(1))` — but the latter is not congruent
    /// to `Identity(4)` without the infinite family of SMC Rule 9 rewrites
    /// (`Identity(m) ⊗ Identity(n) = Identity(m+n)`). Calling [`smc_nf::nf`]
    /// on each term using class-canonical subterms surfaces exactly these
    /// post-merge opportunities without seeding an infinite equation set.
    ///
    /// # Termination
    ///
    /// Each effective [`Self::smc_refine`] pass strictly decreases the number
    /// of equivalence classes (every merge added reduces class count by 1);
    /// the class count is bounded below by 1, so the loop terminates after
    /// finitely many iterations. The `SAFETY_BOUND` is a defense-in-depth
    /// guard — in practice never reached on the 18 Thm 5.60 equations.
    fn propagate_fixpoint(&mut self) {
        const SAFETY_BOUND: usize = 64;
        for _ in 0..SAFETY_BOUND {
            self.propagate();
            if !self.smc_refine() {
                return;
            }
        }
        // Safety bound exhausted — finish pending propagation and return.
        // An infinite loop here would indicate a bug in smc_refine (it
        // should strictly reduce class count per iteration).
        self.propagate();
    }

    /// Post-merge SMC refinement pass.
    ///
    /// For each currently-existing term, rebuild its [`PropExpr`] using
    /// *atom-canonical* substitutions (see [`Self::atom_canonical`]) at
    /// every sub-term position whose class contains any atom, run
    /// [`smc_nf::nf`] on the result, fold back via
    /// [`smc_nf::from_string_diagram`], and merge the NF into the term's
    /// class if it differs.
    ///
    /// Returns `true` iff any new merge was performed. Callers drive a
    /// fixpoint loop over this and [`Self::propagate`] via
    /// [`Self::propagate_fixpoint`].
    ///
    /// # Why atom-only substitution
    ///
    /// The natural alternative — substitute each child's full class-canonical
    /// representative — produces exponentially large `PropExpr` trees when
    /// composite canonicals themselves reference other composite classes,
    /// which then trigger stack overflow in the downstream [`Self::add_term`]
    /// recursion. Atom-only substitution stays O(|term|) and captures the
    /// essential post-merge SMC refinement case: atomic equivalences like
    /// `Scalar(R::one()) ≡ Identity(1)` (D2 in the 18 F&S Thm 5.60 equations)
    /// that enable SMC Rule 9 (`Identity(m) ⊗ Identity(n) → Identity(m+n)`)
    /// to fire on the enclosing `Tensor`.
    ///
    /// Composite-↔-composite equivalences are handled by plain congruence
    /// propagation on the term graph via [`Self::propagate`]; they don't
    /// need re-NF passes.
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
            // `add_term` may have auto-merged `new_id` into an existing class
            // via signature-table collision; check the post-insertion classes
            // to decide whether we need to enqueue an additional merge.
            if self.find(id) != self.find(new_id) {
                self.merge(id, new_id);
                progress = true;
            }
        }

        progress
    }

    /// Rebuild a [`PropExpr`] for `id`, substituting an *atom-canonical*
    /// representative (see [`Self::atom_canonical`]) at every sub-term
    /// position whose class contains any atom — including at composite
    /// (`Compose`/`Tensor`) positions, because a composite like
    /// `Compose(Discard, Zero)` may belong to a class that also contains an
    /// atom like `Scalar(R::zero())` (a direct equation in Thm 5.60). Such
    /// atom-for-composite substitution is the key mechanism for exposing
    /// post-merge SMC Rule 9 opportunities on the enclosing `Tensor`/`Compose`.
    ///
    /// # Size bound
    ///
    /// Substitution only ever *shrinks* the tree — an atom replaces a
    /// composite. Recursion happens only when no atom is available in the
    /// class, and then descends into strictly-smaller child [`TermId`]s.
    /// Output size is bounded by the input term's size, avoiding the
    /// composite-canonical exponential blow-up.
    fn term_to_canonical_expr(&mut self, id: TermId) -> PropExpr<G> {
        // Try atom-for-anything substitution first. This covers both
        // atom-kind nodes (natural case) and composite nodes whose class
        // also contains an atom (D2-style equations). Only recurse into
        // child Compose/Tensor structure when no atom representative
        // exists for this class.
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

    /// Scan `id`'s union-find class for any atom member (`Identity`, `Braid`,
    /// or `Generator`). Returns the preferred atom (lowest-kind, smallest
    /// [`TermId`]) if one exists, else `None`. Composite members (`Compose`,
    /// `Tensor`) are ignored — see [`Self::term_to_canonical_expr`] for
    /// rationale.
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
            // Take the losing root's uses list; after the merge, any uses
            // of its members properly belong to the winning root's list.
            // We re-probe each use's signature against the current root
            // classes and re-file into the winner's list.
            let losing_uses = std::mem::take(&mut self.uses[losing_root]);
            for (term, _other, tag) in losing_uses {
                // `term` is a `Compose(a, b)` / `Tensor(a, b)` node. Re-read
                // its literal children directly from `reverse` — the `other`
                // component of the uses tuple may reference a non-root ID by
                // the time propagation reaches us, so we re-canonicalize via
                // `find` rather than trust it.
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
                        // Fresh collision — merge the two classes.
                        self.merge(term, canonical);
                    }
                    Some(_) => {
                        // Already canonical for this signature; nothing to do.
                    }
                    None => {
                        // Fresh signature; register `term` as canonical.
                        self.signatures.insert(key, term);
                    }
                }

                // Re-file the use under the winning root of each child so
                // later merges involving this node can still find it.
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
