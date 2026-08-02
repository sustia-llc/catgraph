//! Graphical linear algebra — F&S 2018 Thm 5.60: `Mat(R)` has a presentation
//! over the SFG generators plus 18 equations (cocomonoid + monoid + bialgebra,
//! plus scalar interactions). The functor `S: SFG_R → Mat(R)` is full and
//! faithful on this presentation.
//!
//! This module provides:
//!
//! - [`matr_presentation`] — builds the 18-equation presentation.
//! - [`FaithfulnessReport`] — result of a bounded-enumeration faithfulness check.
//! - [`verify_sfg_to_mat_is_full_and_faithful`] — enumerates bounded SFG
//!   expressions, buckets them by matrix image under S, and counts the
//!   `eq_mod`-connected components each bucket splits into.

use catgraph::errors::CatgraphError;

use crate::{
    prop::{Free, PropExpr, presentation::Presentation},
    rig::Rig,
    sfg::{SfgGenerator, SignalFlowGraph},
    sfg_to_mat::sfg_to_mat,
};

/// Build the F&S Thm 5.60 presentation of `Mat(R)`.
///
/// Equations: 10 structural (A1-A3, B1-B3, C1-C4) plus 8 scalar-parameterized
/// (D1-D8). D2 (`r_one = id`) and D8 (`r_zero = ε ; η`) are emitted once; D3/D4/
/// D5/D6 are instantiated for each `a ∈ rig_samples`; D1 (`r_a ; r_b = r_{a*b}`)
/// and D7 (scalar addition `Δ ; (r_a ⊗ r_b) ; μ = r_{a+b}`) additionally iterate
/// `b ∈ rig_samples`.
///
/// This is the full 18-relation set of BE15 Theorem 2 / F&S Thm 5.60 (p.170):
/// the rig structure of `R` is recovered by D1 (multiplication, BE15 (11)),
/// D7 (addition, BE15 (12)), D2 (unit, BE15 (13)), and D8 (zero, BE15 (14)).
///
/// # Errors
///
/// Returns [`CatgraphError::CompositionSizeMismatch`] or
/// [`CatgraphError::Composition`] if `add_equation`'s boundary-word check
/// rejects an equation — should not happen with the hardcoded monochromatic
/// forms below, but surfaced in case of future maintenance bugs.
///
/// # Panics
///
/// Panics via `.expect(...)` if one of the hardcoded compose calls below fails
/// arity validation. This indicates a maintenance bug in this function, not a
/// caller error — the panic message documents the internal inconsistency.
pub fn matr_presentation<R>(
    rig_samples: &[R],
) -> Result<Presentation<SfgGenerator<R>>, CatgraphError>
where
    R: Rig + std::fmt::Debug + Eq + std::hash::Hash + Ord + 'static,
{
    // Short alias for the generator-parameterised PropExpr type used below.
    type E<R> = PropExpr<SfgGenerator<R>>;

    let mut p = Presentation::<SfgGenerator<R>>::new();

    // Short aliases for building PropExpr terms.
    let copy = || Free::<SfgGenerator<R>>::generator(SfgGenerator::Copy);
    let discard = || Free::<SfgGenerator<R>>::generator(SfgGenerator::Discard);
    let add = || Free::<SfgGenerator<R>>::generator(SfgGenerator::Add);
    let zero_gen = || Free::<SfgGenerator<R>>::generator(SfgGenerator::Zero);
    let scalar = |a: R| Free::<SfgGenerator<R>>::generator(SfgGenerator::Scalar(a));
    let id_n = |n: usize| Free::<SfgGenerator<R>>::identity(n);
    let braid_11 = || Free::<SfgGenerator<R>>::braid(1, 1);
    let tensor = |f: E<R>, g: E<R>| Free::<SfgGenerator<R>>::tensor(f, g);

    // Free::compose returns Result; .expect() used here because any failure
    // indicates a bug in this hardcoded equation set, not a caller error.
    let cmp = |f: E<R>, g: E<R>| {
        Free::<SfgGenerator<R>>::compose(f, g).expect("matr_presentation internal arity bug")
    };

    // A1. Δ ; (Δ ⊗ id_1) = Δ ; (id_1 ⊗ Δ)
    p.add_equation(
        cmp(copy(), tensor(copy(), id_n(1))),
        cmp(copy(), tensor(id_n(1), copy())),
    )?;

    // A2. Δ ; σ = Δ
    p.add_equation(cmp(copy(), braid_11()), copy())?;

    // A3. Δ ; (id_1 ⊗ ε) = id_1
    p.add_equation(cmp(copy(), tensor(id_n(1), discard())), id_n(1))?;

    // B1. (μ ⊗ id_1) ; μ = (id_1 ⊗ μ) ; μ
    p.add_equation(
        cmp(tensor(add(), id_n(1)), add()),
        cmp(tensor(id_n(1), add()), add()),
    )?;

    // B2. σ ; μ = μ
    p.add_equation(cmp(braid_11(), add()), add())?;

    // B3. (id_1 ⊗ η) ; μ = id_1
    p.add_equation(cmp(tensor(id_n(1), zero_gen()), add()), id_n(1))?;

    // C1. μ ; Δ = (Δ ⊗ Δ) ; (id_1 ⊗ σ ⊗ id_1) ; (μ ⊗ μ)
    //   LHS:  μ ; Δ is 2→2.
    //   RHS:  (Δ ⊗ Δ) is 2→4; (id_1 ⊗ σ ⊗ id_1) is 4→4 (middle σ swaps pos 1,2);
    //         (μ ⊗ μ) is 4→2. Total: 2→4→4→2.
    let middle_swap = tensor(tensor(id_n(1), braid_11()), id_n(1)); // 4→4
    p.add_equation(
        cmp(add(), copy()),
        cmp(
            cmp(tensor(copy(), copy()), middle_swap),
            tensor(add(), add()),
        ),
    )?;

    // C2. η ; Δ = η ⊗ η
    p.add_equation(cmp(zero_gen(), copy()), tensor(zero_gen(), zero_gen()))?;

    // C3. μ ; ε = ε ⊗ ε
    p.add_equation(cmp(add(), discard()), tensor(discard(), discard()))?;

    // C4. η ; ε = id_0
    p.add_equation(cmp(zero_gen(), discard()), id_n(0))?;

    // D-group scalar equations: D2 and D8 are independent of rig_samples (the
    // unit scalar is always R::one(), the zero scalar always R::zero()); D3-D6
    // need a single sample `a`; D1 and D7 need two samples `a, b` (D1 with their
    // precomputed product, D7 with their precomputed sum). We emit D2 and D8
    // first (outside the loop), then iterate the single-sample equations, then
    // close with the double-sample D1/D7 in the inner loop. The out-of-paper-
    // order of D1 (last by emission) vs its paper position (first in §5.4) is
    // deliberate for enumeration efficiency — emitting the most-instantiated
    // equations at the end of the presentation's equation list.

    // D2. r_{one} = id_1  (BE15 (13))
    p.add_equation(scalar(R::one()), id_n(1))?;

    // D8. r_{zero} = ε ; η  (BE15 (14)) — scalar multiplication by 0 is the
    // discard-then-zero morphism 1→0→1.
    p.add_equation(scalar(R::zero()), cmp(discard(), zero_gen()))?;

    // D1 / D3 / D4 / D5 / D6 / D7 iterate over rig_samples.
    for a in rig_samples {
        // D3. r_a ; Δ = Δ ; (r_a ⊗ r_a)
        p.add_equation(
            cmp(scalar(a.clone()), copy()),
            cmp(copy(), tensor(scalar(a.clone()), scalar(a.clone()))),
        )?;

        // D4. (r_a ⊗ r_a) ; μ = μ ; r_a
        p.add_equation(
            cmp(tensor(scalar(a.clone()), scalar(a.clone())), add()),
            cmp(add(), scalar(a.clone())),
        )?;

        // D5. r_a ; ε = ε
        p.add_equation(cmp(scalar(a.clone()), discard()), discard())?;

        // D6. η ; r_a = η
        p.add_equation(cmp(zero_gen(), scalar(a.clone())), zero_gen())?;

        // D1 / D7 iterate b as well.
        for b in rig_samples {
            // D1. r_a ; r_b = r_{a*b}
            p.add_equation(
                cmp(scalar(a.clone()), scalar(b.clone())),
                scalar(a.clone() * b.clone()),
            )?;

            // D7. Δ ; (r_a ⊗ r_b) ; μ = r_{a+b} — scalar addition (BE15 (12)):
            // copy the input, scale the two copies by a and b, then add.
            p.add_equation(
                cmp(
                    cmp(copy(), tensor(scalar(a.clone()), scalar(b.clone()))),
                    add(),
                ),
                scalar(a.clone() + b.clone()),
            )?;
        }
    }

    Ok(p)
}

/// Disjoint-set forest over `0..n`, with path halving and union by size.
///
/// Used by [`verify_sfg_to_mat_is_full_and_faithful`] to take the connected
/// components of the `eq_mod`-equality graph inside one matrix bucket (#189).
struct UnionFind {
    parent: Vec<usize>,
    size: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            size: vec![1; n],
        }
    }

    fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            // Path halving: point x at its grandparent, then step.
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }

    fn same(&mut self, a: usize, b: usize) -> bool {
        self.find(a) == self.find(b)
    }

    fn union(&mut self, a: usize, b: usize) {
        let (mut ra, mut rb) = (self.find(a), self.find(b));
        if ra == rb {
            return;
        }
        if self.size[ra] < self.size[rb] {
            std::mem::swap(&mut ra, &mut rb);
        }
        self.parent[rb] = ra;
        self.size[ra] += self.size[rb];
    }
}

/// Faithfulness-check report for `S: SFG_R → Mat(R)` on a size-bounded sample.
#[derive(Debug, Clone)]
pub struct FaithfulnessReport<R: Rig + std::fmt::Debug + Eq + std::hash::Hash + Ord + 'static> {
    pub size_bound: usize,
    pub expressions_checked: usize,
    /// Summed over matrix buckets: the bucket's number of `eq_mod`-connected
    /// components, minus one. Zero iff every bucket is a single component, i.e.
    /// iff the default engine proves every equality the matrix functor asserts.
    pub collisions_under_s: usize,
    /// Pairs `(a, b)` where `a` and `b` lie in **distinct** `eq_mod`-connected
    /// components under `matr_presentation` but `sfg_to_mat(a) ==
    /// sfg_to_mat(b)`. Empty iff `S` is faithful on the enumerated fragment.
    pub witnesses: Vec<(SignalFlowGraph<R>, SignalFlowGraph<R>)>,
}

/// Enumerate SFG expressions whose `PropExpr` depth is at most `size_bound`,
/// bucket them by matrix image under `S`, and count how far each bucket falls
/// short of being a single `matr_presentation(rig_samples)`-equivalence class.
///
/// # How a bucket is partitioned (#189)
///
/// Inside a bucket, build the graph whose vertices are the bucket's expressions
/// and whose edges are the pairs [`Presentation::eq_mod`] decides `Some(true)`,
/// and take its **connected components**. A bucket with `k` components
/// contributes `k − 1` to `collisions_under_s`, and adjacent-pair witnesses
/// between the component representatives (least bucket index per component).
///
/// This used to be a greedy scan — each expression joined the first class whose
/// *representative* it matched. That agrees with components only when the
/// relation is transitive, and `eq_mod` is not: `Scalar(false)` ~
/// `Discard ; Zero` ~ `Discard ⊗ Zero` while `Scalar(false)` ≁
/// `Discard ⊗ Zero` (#189 measured 10 490 ordered violating triples on a
/// 120-expression pool of parallel `1 → 1` arrows, zero `None` verdicts — see
/// [`Presentation::eq_mod`]'s own note). Over a non-transitive relation a greedy
/// class count is a statistic of enumeration order, not a function of the
/// relation. Components are the transitive closure of the same edge set, so the
/// count *is* a function of the relation, and growing the relation can only add
/// edges and therefore only merge components — the counts move monotonically,
/// which is the property the baselines in `tests/graphical_linalg.rs` are read
/// under.
///
/// Components are coarser than the greedy partition, or equal to it — every
/// greedy class sits inside one component — so the switch could only lower and
/// in fact lowered every pinned baseline: BoolRig 952 → **748**, UnitInterval
/// 1397 → **1114**, Tropical 1974 → **1594**, F64Rig 1969 → **1590** (depth 2,
/// release).
///
/// Cost: all-pairs per bucket is `O(k²)` `eq_mod` calls in the worst case, but a
/// pair already inside one component is skipped — exactly, since an intra-component
/// edge cannot change the partition — which is what keeps it affordable. Depth-2
/// bucket sizes top out at 682 (BoolRig) / 1602 (Tropical, F64Rig), for an
/// all-pairs ceiling of 1.6M–4.5M calls per rig; the skip brings the BoolRig
/// depth-2 run to ~2 min against the greedy scan's ~11 s.
///
/// # Enumeration strategy
///
/// Expressions are built bottom-up by depth:
/// - Depth 0: `id_0..=id_4`, primitive generators, `braid(1,1)`, plus
///   `Scalar(a)` instantiated for each `a ∈ rig_samples`.
/// - Depths `1..=size_bound`: close under `Compose(f, g)` (arity-compatible)
///   and `Tensor(f, g)`, bounded by `total_arity ≤ 4` to keep the enumeration
///   finite. After each depth, expressions are deduplicated by structural
///   `Debug` key.
///
/// This is exponential in `size_bound`. Recommended: `size_bound ∈ {2, 3, 4}`.
///
/// # Errors
///
/// Returns [`CatgraphError::Presentation`] or [`CatgraphError::SfgFunctor`] if
/// normalization or `sfg_to_mat` fails on any enumerated expression, and
/// propagates [`matr_presentation`]'s errors from building the presentation.
pub fn verify_sfg_to_mat_is_full_and_faithful<R>(
    size_bound: usize,
    rig_samples: &[R],
) -> Result<FaithfulnessReport<R>, CatgraphError>
where
    R: Rig + std::fmt::Debug + Eq + std::hash::Hash + Ord + 'static,
{
    let presentation = matr_presentation(rig_samples)?;

    // Enumerate expressions up to depth = size_bound.
    let expressions = enumerate_sfg_expressions::<R>(size_bound, rig_samples);

    // Primary bucket = matrix image under S. Within each bucket, two
    // expressions are a faithfulness-violation witness iff they are NOT
    // equal modulo the presentation. Within-bucket partitioning uses
    // Presentation::eq_mod (which routes through the default
    // CongruenceClosure engine).
    let mut by_matrix: std::collections::HashMap<String, Vec<SignalFlowGraph<R>>> =
        std::collections::HashMap::new();
    for expr in &expressions {
        let m = sfg_to_mat(expr)?;
        let matrix_key = format!("{}×{} {:?}", m.rows(), m.cols(), m.entries());
        by_matrix.entry(matrix_key).or_default().push(expr.clone());
    }

    let mut collisions = 0usize;
    let mut witnesses: Vec<(SignalFlowGraph<R>, SignalFlowGraph<R>)> = Vec::new();

    for bucket in by_matrix.values() {
        if bucket.len() < 2 {
            continue;
        }

        // Partition the bucket into the CONNECTED COMPONENTS of the graph whose
        // vertices are the bucket's expressions and whose edges are the pairs
        // `eq_mod` decides `Some(true)` (#189). This is all-pairs by
        // construction, not a scan against class representatives: `eq_mod` is
        // *not* transitive (measured witness triple — `Scalar(false)` ~
        // `Discard ; Zero` ~ `Discard ⊗ Zero` while `Scalar(false)` ≁
        // `Discard ⊗ Zero`), so a representative scan returns a statistic of
        // enumeration order rather than a function of the relation. Components
        // are the transitive closure of exactly the same edge set, so the count
        // *is* a function of the relation, and enlarging the relation can only
        // merge components — the monotonicity the pins are read under.
        //
        // eq_mod returns Option<bool>:
        //   Some(true)  → equal in presentation (edge — union the endpoints)
        //   Some(false) → decisively distinct (no edge)
        //   None        → CC depth bound hit without decision (no edge; this is
        //                 the conservative choice — reporting a possible
        //                 witness rather than silently collapsing).
        let mut uf = UnionFind::new(bucket.len());
        for i in 0..bucket.len() {
            for j in (i + 1)..bucket.len() {
                // Skipping a pair already in one component is exact, not an
                // approximation: an edge inside a component cannot change the
                // component partition. It is also what keeps the pass
                // affordable — the O(k²) pair space collapses to roughly the
                // edges that actually merge something.
                if uf.same(i, j) {
                    continue;
                }
                if let Some(true) =
                    presentation.eq_mod(bucket[i].as_prop_expr(), bucket[j].as_prop_expr())?
                {
                    uf.union(i, j);
                }
            }
        }

        // One representative per component — the least bucket index in it, so
        // the witness list is deterministic given the enumeration and does not
        // depend on which endpoint union-by-size happened to keep as root.
        let mut seen_roots = std::collections::HashSet::new();
        let mut reps: Vec<&SignalFlowGraph<R>> = Vec::new();
        for (i, expr) in bucket.iter().enumerate() {
            if seen_roots.insert(uf.find(i)) {
                reps.push(expr);
            }
        }

        if reps.len() > 1 {
            // Faithfulness violation: multiple presentation-equivalence
            // components share a matrix image. Record adjacent-pair witnesses
            // between the component representatives to keep the report size
            // bounded.
            for w in reps.windows(2) {
                collisions += 1;
                witnesses.push((w[0].clone(), w[1].clone()));
            }
        }
    }

    Ok(FaithfulnessReport {
        size_bound,
        expressions_checked: expressions.len(),
        collisions_under_s: collisions,
        witnesses,
    })
}

/// Enumerate SFG expressions with `PropExpr` depth ≤ `size_bound` and
/// arity bounded so the enumeration is finite.
fn enumerate_sfg_expressions<R>(size_bound: usize, rig_samples: &[R]) -> Vec<SignalFlowGraph<R>>
where
    R: Rig + std::fmt::Debug + Eq + std::hash::Hash + Ord + 'static,
{
    let mut expressions: Vec<SignalFlowGraph<R>> = Vec::new();

    // Depth 0: atomic expressions.
    let max_arity = 4;
    for n in 0..=max_arity {
        expressions.push(SignalFlowGraph::<R>::identity(n));
    }
    expressions.push(SignalFlowGraph::<R>::copy());
    expressions.push(SignalFlowGraph::<R>::discard());
    expressions.push(SignalFlowGraph::<R>::add());
    expressions.push(SignalFlowGraph::<R>::zero());
    expressions.push(SignalFlowGraph::<R>::braid_1_1());
    for r in rig_samples {
        expressions.push(SignalFlowGraph::<R>::scalar(r.clone()));
    }

    // Depths 1..=size_bound: close under compose + tensor, with arity cap.
    for _ in 1..=size_bound {
        let mut new_exprs: Vec<SignalFlowGraph<R>> = Vec::new();
        for f in &expressions {
            for g in &expressions {
                if let Ok(c) = f.compose(g)
                    && total_arity(&c) <= max_arity
                {
                    new_exprs.push(c);
                }
                let t = f.tensor(g);
                if total_arity(&t) <= max_arity {
                    new_exprs.push(t);
                }
            }
        }
        expressions.extend(new_exprs);

        // Deduplicate by structural Debug key to prevent combinatorial explosion.
        let mut seen = std::collections::HashSet::new();
        expressions.retain(|e| seen.insert(format!("{:?}", e.as_prop_expr())));
    }

    expressions
}

fn total_arity<R>(sfg: &SignalFlowGraph<R>) -> usize
where
    R: Rig + std::fmt::Debug + Eq + std::hash::Hash + Ord + 'static,
{
    sfg.domain().max(sfg.codomain())
}
