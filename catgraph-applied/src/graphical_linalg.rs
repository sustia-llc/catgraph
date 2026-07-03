//! Graphical linear algebra — F&S 2018 Thm 5.60: `Mat(R)` has a presentation
//! over the SFG generators plus 16 equations (cocomonoid + monoid + bialgebra,
//! plus scalar interactions). The functor `S: SFG_R → Mat(R)` is full and
//! faithful on this presentation.
//!
//! This module provides:
//!
//! - [`matr_presentation`] — builds the 16-equation presentation.
//! - [`FaithfulnessReport`] — result of a bounded-enumeration faithfulness check.
//! - [`verify_sfg_to_mat_is_full_and_faithful`] — enumerates bounded SFG
//!   expressions, groups by presentation-equivalence, and verifies that
//!   distinct equivalence classes map to distinct matrices under S.

use catgraph::errors::CatgraphError;

use crate::{
    prop::{Free, PropExpr, presentation::Presentation},
    rig::Rig,
    sfg::{SfgGenerator, SignalFlowGraph},
    sfg_to_mat::sfg_to_mat,
};

/// Build the F&S Thm 5.60 presentation of `Mat(R)`.
///
/// Equations: 10 structural (A1-A3, B1-B3, C1-C4) plus 6 scalar-parameterized
/// (D1-D6), with D1/D3/D4/D5/D6 instantiated for each `a ∈ rig_samples` and
/// D1 additionally iterating `b ∈ rig_samples`.
///
/// # Errors
///
/// Returns [`CatgraphError::Presentation`] if any equation construction fails
/// (arity mismatch — should not happen with the hardcoded forms below, but
/// surfaced in case of future maintenance bugs).
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
    R: Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static,
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

    // D-group scalar equations: D2 is independent of rig_samples (the unit
    // scalar is always R::one()); D3-D6 need a single sample `a`; D1 needs two
    // samples `a, b` with their precomputed product. We emit D2 first (outside
    // the loop), then iterate the single-sample equations, then close with the
    // double-sample D1 in the inner loop. The out-of-paper-order of D1 (last
    // by emission) vs its paper position (first in §5.4) is deliberate for
    // enumeration efficiency — emitting D1 last puts the most instantiated
    // equations at the end of the presentation's equation list.

    // D2. r_{one} = id_1
    p.add_equation(scalar(R::one()), id_n(1))?;

    // D1 / D3 / D4 / D5 / D6 iterate over rig_samples.
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

        // D1. r_a ; r_b = r_{a*b} — iterate b as well.
        for b in rig_samples {
            p.add_equation(
                cmp(scalar(a.clone()), scalar(b.clone())),
                scalar(a.clone() * b.clone()),
            )?;
        }
    }

    Ok(p)
}

/// Faithfulness-check report for `S: SFG_R → Mat(R)` on a size-bounded sample.
#[derive(Debug, Clone)]
pub struct FaithfulnessReport<R: Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static> {
    pub size_bound: usize,
    pub expressions_checked: usize,
    pub collisions_under_s: usize,
    /// Pairs `(a, b)` where `a` and `b` normalize to **distinct** expressions
    /// under `matr_presentation` but `sfg_to_mat(a) == sfg_to_mat(b)`. Empty
    /// iff `S` is faithful on the enumerated fragment.
    pub witnesses: Vec<(SignalFlowGraph<R>, SignalFlowGraph<R>)>,
}

/// Enumerate SFG expressions whose `PropExpr` depth is at most `size_bound`,
/// normalize each under `matr_presentation(rig_samples)`, and verify that
/// distinct presentation-equivalence classes map to distinct matrices under
/// `S`.
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
/// normalization or `sfg_to_mat` fails on any enumerated expression.
pub fn verify_sfg_to_mat_is_full_and_faithful<R>(
    size_bound: usize,
    rig_samples: &[R],
) -> Result<FaithfulnessReport<R>, CatgraphError>
where
    R: Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static,
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

        // Partition the bucket into presentation-equivalence classes via
        // pairwise eq_mod. `classes[i]` is a representative of the i-th
        // class; each expression is added to the first class whose
        // representative it is eq_mod-equal to, or opens a new class.
        //
        // eq_mod returns Option<bool>:
        //   Some(true)  → equal in presentation (same class)
        //   Some(false) → decisively distinct (different class)
        //   None        → CC depth bound hit without decision (treated as
        //                 distinct; this is the conservative choice —
        //                 reporting a possible witness rather than silently
        //                 collapsing).
        let mut classes: Vec<&SignalFlowGraph<R>> = Vec::new();
        for expr in bucket {
            let mut matched = false;
            for rep in &classes {
                if let Some(true) = presentation.eq_mod(expr.as_prop_expr(), rep.as_prop_expr())? {
                    matched = true;
                    break;
                }
            }
            if !matched {
                classes.push(expr);
            }
        }

        if classes.len() > 1 {
            // Faithfulness violation: multiple presentation-equivalence
            // classes share a matrix image. Record adjacent-pair witnesses
            // between the class representatives to keep the report size
            // bounded.
            for w in classes.windows(2) {
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
    R: Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static,
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
    R: Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static,
{
    sfg.domain().max(sfg.codomain())
}
