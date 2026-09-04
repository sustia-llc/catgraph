//! The applied claim, end to end.
//!
//! `S : SFG_R → Mat(R)` (F&S 2018 Thm 5.53, `src/sfg_to_mat.rs`) equals a
//! basis-vector evaluator written in this file, on every term of a
//! depth-bounded corpus over the five `SfgGenerator` variants, over `BoolRig`,
//! `UnitInterval`, `Tropical` and `F64Rig`; `S` commutes with `compose`,
//! `tensor` and `permute_side`; `mat_to_sfg` round-trips through `S`
//! (Prop 5.56); `PropExpr` reports the Def 5.2 / 5.25 arities and its
//! `from_permutation` realizes the permutation `MatR::permutation_matrix`
//! names; `compose` on `DecoratedCospan` and on `PetriNet` equals a union-find
//! partition reference computed from the operand wirings; and
//! `MatR::zero_matrix` and `MatKron::zero_matrix` fill `R::zero()` on a rig
//! whose zero is not `0.0`.
//!
//! # Input space
//!
//! The term corpus is the closure of the atom set (`Identity(0..=2)`,
//! `Braid(1, 1)`, `Braid(1, 2)`, `Braid(2, 1)`, the five generators, two
//! `Scalar` values per rig) under two rounds of `compose` / `tensor`, each
//! round dropping the terms whose source or target exceeds `ARITY_CAP` before
//! the next round reads them, so the second round composes and tensors only
//! in-cap first-round terms; its size is asserted below per rig, its variant
//! coverage at `F64Rig`.
//! The `compose` and `tensor` squares range over the ordered pairs of the
//! one-round pool, not of that corpus; the `permute_side` square runs every
//! element of `S₃` and `S₄` on both sides of one `F64Rig` witness. The wiring
//! corpus behind the composition rows is every wiring with domain, codomain and
//! apex each at most 2, at one wire type, carried by one `Decoration` on the
//! `DecoratedCospan` side and by transition-free nets on the `PetriNet` side.
//! The round-trip rows are the F&S Eq 5.57 / Ex 5.58 matrices, the
//! empty-dimension shapes, and a 128-case proptest per rig over matrices up to
//! 4×4. `zero_matrix` is asserted at one 2×3 shape and the two degenerate ones.
//!
//! # References
//!
//! `S` is compared against `evaluate`, which sends a term to the `R`-linear map
//! it denotes — copy duplicates, add sums, scalar multiplies — and reads the
//! matrix off the images of the basis vectors; it shares no code with
//! `sfg_to_mat`, which maps generators to matrices and folds with `matmul` and
//! `block_diagonal`. The composition rows compare against
//! `catgraph_testutil::wiring::CospanWiring::pushout`, `Vec<usize>` union-find
//! with no catgraph edge. The `from_permutation` rows use
//! `MatR::permutation_matrix` through `S` as the oracle, and the generator
//! table and round-trip rows are hand-written F&S values.
//!
//! # Reach
//!
//! `Prop`, `SfgSignature` and `PetriDecoration` are marker types whose single
//! field is private, so an integration test names them only in a type
//! position; `PetriDecoration` is reached as `DecoratedCospan`'s decoration
//! parameter and the other two are not reached. `Sealed` (`integer.rs:44`) is
//! declared inside `pub(crate) mod private`, so an integration test cannot name
//! it. The `F64Rig` corpus reaches all five `PropExpr` variants and all five
//! `SfgGenerator` variants (asserted in `corpus_reaches_every_variant`), but
//! only at the two rig values each rig's atom list carries, so a defect that
//! needs a third scalar value is outside it. Both functor squares read `S` of
//! the same two subterms on either side, so what they touch is the arm's
//! operand order and its choice of `matmul` over `block_diagonal`, not the
//! matrix arithmetic itself — that is what the evaluator row is for.
//!
//! # covers:
//!
//! `BoolRig` `DecoratedCospan` `Decoration` `F64Rig` `Free` `MatKron` `MatR`
//! `One` `PetriApex` `PetriDecoration` `PetriNet` `PropExpr` `PropSignature`
//! `Rig` `SfgGenerator` `SignalFlowGraph` `Transition` `Tropical`
//! `UnitInterval` `Zero`
//!
//! # not-covered:
//!
//! `Atom` `BaseChange` `BrauerMorphism` `Checked` `CheckedOps` `CircAlgebra`
//! `ColoredCompleteFunctor` `ColoredExpr` `CompleteFunctor` `CongruenceClosure`
//! `Content` `ContentEdge` `ContentKey` `Dir` `E1` `E1ToE2` `E2`
//! `EnrichedCategory` `FaithfulnessReport` `FragmentStatus` `HasArity`
//! `HomMap` `HyperedgeIndex` `Hypergraph` `HypergraphError`
//! `LawvereMetricSpace` `Layer` `LinearCombination` `Marking` `MatchSite`
//! `MatrixNFFunctor` `Node` `NormalizeEngine` `NormalizeResult`
//! `OperadAlgebra` `OperadFunctor` `Pair` `Presentation` `PresentedProp` `Prop`
//! `RewriteOutcome` `RewriteRule` `RewriteStep` `Sealed` `SfgSignature`
//! `StringDiagram` `TermId` `VertexIndex` `WiringDiagram` `Z` `ZAlgebra`

use std::borrow::Cow;
use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;

use catgraph::cospan::Cospan;
use catgraph::{
    category::{Composable, HasIdentity},
    monoidal::{Monoidal, SymmetricMonoidalMorphism},
};
use catgraph_applied::{
    decorated_cospan::{DecoratedCospan, Decoration},
    mat::MatR,
    mat_kron::MatKron,
    mat_to_sfg::mat_to_sfg,
    petri_net::{PetriApex, PetriDecoration, PetriNet, Transition},
    prop::{Free, PropExpr, PropSignature, mono_word},
    rig::{BoolRig, F64Rig, One, Rig, Tropical, UnitInterval, Zero},
    sfg::{SfgGenerator, SignalFlowGraph, copy_n},
    sfg_to_mat::sfg_to_mat,
};
use catgraph_testutil::{all_perm_indices, wiring::CospanWiring};
use permutations::Permutation;
use proptest::prelude::*;
use rust_decimal::Decimal;

/// The wire type every cospan-side claim is built on.
const Z: char = 'z';

/// The largest source or target arity a corpus term may have.
const ARITY_CAP: usize = 3;

/// How many rounds of `compose` / `tensor` the term corpus is closed under.
const TERM_DEPTH: usize = 2;

/// The bound every rig in this file satisfies, spelled once.
trait TestRig: Rig + Debug + Eq + Hash + Ord + 'static {}
impl<R> TestRig for R where R: Rig + Debug + Eq + Hash + Ord + 'static {}

/// An `SFG_R` term.
type Term<R> = PropExpr<SfgGenerator<R>>;

// ---------------------------------------------------------------------------
// The reference: a term's denotation as an R-linear map
// ---------------------------------------------------------------------------

/// The `R`-linear map a term denotes, applied to one input vector.
///
/// Written from F&S Def 5.45's reading of the generators — copy duplicates a
/// signal, discard drops it, add sums two, zero emits `R::zero()`, `Scalar(r)`
/// multiplies by `r` — with `Braid(m, n)` moving the top `m` wires past the
/// bottom `n`, `Compose` feeding the left output into the right input, and
/// `Tensor` splitting the input at the left half's source arity.
///
/// # Panics
///
/// Panics when `input.len()` is not `expr.source()`, which the corpus builder
/// makes unreachable.
fn evaluate<R: TestRig>(expr: &Term<R>, input: &[R]) -> Vec<R> {
    assert_eq!(
        input.len(),
        expr.source(),
        "evaluate: input width {} does not match the term's source arity {}",
        input.len(),
        expr.source()
    );
    match expr {
        PropExpr::Identity(_) => input.to_vec(),
        PropExpr::Braid(m, _) => {
            let mut out = input[*m..].to_vec();
            out.extend_from_slice(&input[..*m]);
            out
        }
        PropExpr::Generator(g) => match g {
            SfgGenerator::Copy => vec![input[0].clone(), input[0].clone()],
            SfgGenerator::Discard => Vec::new(),
            SfgGenerator::Add => vec![input[0].clone() + input[1].clone()],
            SfgGenerator::Zero => vec![R::zero()],
            SfgGenerator::Scalar(r) => vec![input[0].clone() * r.clone()],
        },
        PropExpr::Compose(f, g) => evaluate(g, &evaluate(f, input)),
        PropExpr::Tensor(f, g) => {
            let split = f.source();
            let mut out = evaluate(f, &input[..split]);
            out.extend(evaluate(g, &input[split..]));
            out
        }
    }
}

/// The matrix of a term, read off the images of the basis vectors.
///
/// Row `i` is `evaluate(expr, e_i)`; the term's denotation is `R`-linear, so
/// that row is the `i`th row of the matrix `S` must produce.
fn matrix_by_evaluation<R: TestRig>(expr: &Term<R>) -> MatR<R> {
    let rows = expr.source();
    let cols = expr.target();
    let entries: Vec<Vec<R>> = (0..rows)
        .map(|i| {
            let mut basis = vec![R::zero(); rows];
            basis[i] = R::one();
            evaluate(expr, &basis)
        })
        .collect();
    MatR::new(rows, cols, entries)
        .expect("evaluate returns target()-wide rows on every basis input")
}

// ---------------------------------------------------------------------------
// The term corpus
// ---------------------------------------------------------------------------

/// The atoms every corpus is grown from: the identities up to width 2, the
/// three braids up to width 3 — including the two asymmetric ones, whose
/// matrices are each other's transpose — the four constant generators, and one
/// `Scalar` per value in `scalars`.
fn atoms<R: TestRig>(scalars: &[R]) -> Vec<Term<R>> {
    let mut out = vec![
        Free::identity(0),
        Free::identity(1),
        Free::identity(2),
        Free::braid(1, 1),
        Free::braid(1, 2),
        Free::braid(2, 1),
        Free::generator(SfgGenerator::Copy),
        Free::generator(SfgGenerator::Discard),
        Free::generator(SfgGenerator::Add),
        Free::generator(SfgGenerator::Zero),
    ];
    out.extend(
        scalars
            .iter()
            .map(|r| Free::generator(SfgGenerator::Scalar(r.clone()))),
    );
    out
}

/// Whether a term's two boundary arities are both within [`ARITY_CAP`].
fn within_cap<R: TestRig>(expr: &Term<R>) -> bool {
    expr.source() <= ARITY_CAP && expr.target() <= ARITY_CAP
}

/// One round of closure: `terms` plus every in-cap `compose` of a composable
/// ordered pair and every in-cap `tensor` of an ordered pair.
///
/// Insertion order is the enumeration order, so the corpus is the same vector
/// on every run.
fn grow<R: TestRig>(terms: &[Term<R>]) -> Vec<Term<R>> {
    let mut seen: HashSet<Term<R>> = terms.iter().cloned().collect();
    let mut out = terms.to_vec();
    for f in terms {
        for g in terms {
            if f.target() == g.source() {
                let composed = Free::compose(f.clone(), g.clone())
                    .expect("the boundary arities were just checked equal");
                if within_cap(&composed) && seen.insert(composed.clone()) {
                    out.push(composed);
                }
            }
            let tensored = Free::tensor(f.clone(), g.clone());
            if within_cap(&tensored) && seen.insert(tensored.clone()) {
                out.push(tensored);
            }
        }
    }
    out
}

/// The corpus: [`TERM_DEPTH`] rounds of [`grow`] over [`atoms`].
fn term_corpus<R: TestRig>(scalars: &[R]) -> Vec<Term<R>> {
    let mut corpus: Vec<Term<R>> = atoms(scalars)
        .into_iter()
        .filter(within_cap)
        .collect::<Vec<_>>();
    for _ in 0..TERM_DEPTH {
        corpus = grow(&corpus);
    }
    corpus
}

/// The corpus one round shallower — the operand pool the two functor squares
/// range over, so every square instance is itself a corpus term.
fn operand_pool<R: TestRig>(scalars: &[R]) -> Vec<Term<R>> {
    let mut corpus: Vec<Term<R>> = atoms(scalars).into_iter().filter(within_cap).collect();
    for _ in 0..TERM_DEPTH - 1 {
        corpus = grow(&corpus);
    }
    corpus
}

/// The two scalars each rig's atom list carries.
fn bool_scalars() -> Vec<BoolRig> {
    vec![BoolRig(true), BoolRig(false)]
}

fn unit_scalars() -> Vec<UnitInterval> {
    vec![
        UnitInterval::new(0.5).expect("0.5 is in [0, 1]"),
        UnitInterval::new(1.0).expect("1.0 is in [0, 1]"),
    ]
}

fn tropical_scalars() -> Vec<Tropical> {
    vec![Tropical(2.0), Tropical::zero()]
}

fn f64_scalars() -> Vec<F64Rig> {
    vec![F64Rig(3.0), F64Rig(0.0)]
}

// ---------------------------------------------------------------------------
// Thm 5.53: S equals the basis-vector evaluator
// ---------------------------------------------------------------------------

/// `S(t)` against `matrix_by_evaluation(t)` on every corpus term; returns the
/// number of terms checked.
fn s_agrees_with_evaluator<R: TestRig>(rig: &str, scalars: &[R]) -> usize {
    let corpus = term_corpus(scalars);
    for term in &corpus {
        let observed = sfg_to_mat(&SignalFlowGraph::from_prop_expr(term.clone()))
            .expect("corpus term is well-formed");
        let expected = matrix_by_evaluation(term);
        assert_eq!(
            observed, expected,
            "{rig}: S disagrees with the basis-vector evaluator on {term:?}\n  observed \
             {observed:?}\n  expected {expected:?}"
        );
    }
    corpus.len()
}

/// The Thm 5.53 claim on the four shipped rigs, with the corpus size pinned so
/// a corpus that silently shrank is a failure and not a weaker sweep.
#[test]
fn s_functor_equals_the_basis_vector_evaluator_on_four_rigs() {
    assert_eq!(
        (
            s_agrees_with_evaluator("BoolRig", &bool_scalars()),
            s_agrees_with_evaluator("UnitInterval", &unit_scalars()),
            s_agrees_with_evaluator("Tropical", &tropical_scalars()),
            s_agrees_with_evaluator("F64Rig", &f64_scalars()),
        ),
        (CORPUS_SIZE, CORPUS_SIZE, CORPUS_SIZE, CORPUS_SIZE),
        "the term corpus size moved"
    );
}

/// The size of `term_corpus` at [`TERM_DEPTH`] and [`ARITY_CAP`]. A term's
/// shape does not depend on the rig, and each rig's atom list carries two
/// distinct `Scalar` values, so the four corpora are the same size.
const CORPUS_SIZE: usize = 13330;

/// The size of `operand_pool`, the source of the functor-square operands.
const POOL_SIZE: usize = 144;

/// Every `PropExpr` variant and every `SfgGenerator` variant occurs somewhere
/// in the corpus, so the sweep above is not silently a sweep over a fragment.
#[test]
fn corpus_reaches_every_variant() {
    let corpus = term_corpus(&f64_scalars());
    let mut seen = [false; 5];
    let mut generators = [false; 5];
    fn walk(expr: &Term<F64Rig>, seen: &mut [bool; 5], generators: &mut [bool; 5]) {
        match expr {
            PropExpr::Identity(_) => seen[0] = true,
            PropExpr::Braid(..) => seen[1] = true,
            PropExpr::Generator(g) => {
                seen[2] = true;
                generators[match g {
                    SfgGenerator::Copy => 0,
                    SfgGenerator::Discard => 1,
                    SfgGenerator::Add => 2,
                    SfgGenerator::Zero => 3,
                    SfgGenerator::Scalar(_) => 4,
                }] = true;
            }
            PropExpr::Compose(f, g) => {
                seen[3] = true;
                walk(f, seen, generators);
                walk(g, seen, generators);
            }
            PropExpr::Tensor(f, g) => {
                seen[4] = true;
                walk(f, seen, generators);
                walk(g, seen, generators);
            }
        }
    }
    for term in &corpus {
        walk(term, &mut seen, &mut generators);
    }
    assert_eq!(
        (seen, generators),
        ([true; 5], [true; 5]),
        "the corpus misses a PropExpr or SfgGenerator variant; the flags are in \
         Identity/Braid/Generator/Compose/Tensor and Copy/Discard/Add/Zero/Scalar order"
    );
}

/// Eq 5.52's generator table, hand-written from page 165.
#[test]
fn eq_5_52_generator_table() {
    let scalar =
        sfg_to_mat(&SignalFlowGraph::<F64Rig>::scalar(F64Rig(3.5))).expect("scalar is well-formed");
    assert_eq!(
        (scalar.rows(), scalar.cols(), scalar.entries()[0][0]),
        (1, 1, F64Rig(3.5)),
        "Scalar(r) : 1 → 1 is the 1×1 matrix [[r]]"
    );

    let add = sfg_to_mat(&SignalFlowGraph::<F64Rig>::add()).expect("add is well-formed");
    assert_eq!(
        (add.rows(), add.cols(), add.entries().to_vec()),
        (2, 1, vec![vec![F64Rig(1.0)], vec![F64Rig(1.0)]]),
        "Add : 2 → 1 is the 2×1 all-ones matrix"
    );

    let zero = sfg_to_mat(&SignalFlowGraph::<F64Rig>::zero()).expect("zero is well-formed");
    assert_eq!(
        (zero.rows(), zero.cols(), zero.entries().to_vec()),
        (0, 1, Vec::new()),
        "Zero : 0 → 1 is the empty 0×1 matrix"
    );

    let copy = sfg_to_mat(&SignalFlowGraph::<F64Rig>::copy()).expect("copy is well-formed");
    assert_eq!(
        (copy.rows(), copy.cols(), copy.entries().to_vec()),
        (1, 2, vec![vec![F64Rig(1.0), F64Rig(1.0)]]),
        "Copy : 1 → 2 is the 1×2 all-ones matrix"
    );

    let discard =
        sfg_to_mat(&SignalFlowGraph::<F64Rig>::discard()).expect("discard is well-formed");
    assert_eq!(
        (discard.rows(), discard.cols(), discard.entries().to_vec()),
        (1, 0, vec![Vec::new()]),
        "Discard : 1 → 0 is the empty 1×0 matrix"
    );

    // The identity and the two-wire braid, on two rigs.
    assert_eq!(
        sfg_to_mat(&SignalFlowGraph::<F64Rig>::identity(2)).expect("id is well-formed"),
        MatR::<F64Rig>::identity(2)
    );
    assert_eq!(
        sfg_to_mat(&SignalFlowGraph::<BoolRig>::identity(3)).expect("id is well-formed"),
        MatR::<BoolRig>::identity(3)
    );
    let braid = sfg_to_mat(&SignalFlowGraph::<F64Rig>::braid_1_1()).expect("braid is well-formed");
    assert_eq!(
        braid.entries().to_vec(),
        vec![
            vec![F64Rig(0.0), F64Rig(1.0)],
            vec![F64Rig(1.0), F64Rig(0.0)]
        ],
        "S(σ_{{1,1}}) is the 2×2 swap"
    );

    // `copy_n(3)` is derived, not primitive: `S` of it is the 1×3 all-ones row.
    let copy3 = sfg_to_mat(&copy_n::<BoolRig>(3).expect("copy_n is arity-safe"))
        .expect("copy_n(3) is well-formed");
    assert_eq!(
        (copy3.rows(), copy3.cols(), copy3.entries().to_vec()),
        (1, 3, vec![vec![BoolRig(true); 3]]),
        "S(copy_n(3)) is the 1×3 all-ones row"
    );

    // Two composites at their literal values: `copy ; add` amplifies by two on
    // `F64Rig`, and a scalar chain multiplies on `UnitInterval`.
    let amplify = SignalFlowGraph::<F64Rig>::copy()
        .compose(&SignalFlowGraph::<F64Rig>::add())
        .expect("copy : 1 → 2 meets add : 2 → 1");
    let amplify = sfg_to_mat(&amplify).expect("the composite is well-formed");
    assert_eq!(
        (amplify.rows(), amplify.cols(), amplify.entries()[0][0]),
        (1, 1, F64Rig(2.0)),
        "S(copy ; add) is the 1×1 matrix [[1·1 + 1·1]]"
    );
    let half = UnitInterval::new(0.5).expect("0.5 is in [0, 1]");
    let three_fifths = UnitInterval::new(0.6).expect("0.6 is in [0, 1]");
    let chain = SignalFlowGraph::<UnitInterval>::scalar(half)
        .compose(&SignalFlowGraph::<UnitInterval>::scalar(three_fifths))
        .expect("both factors are 1 → 1");
    let chain = sfg_to_mat(&chain).expect("the composite is well-formed");
    assert_eq!(
        chain.entries()[0][0],
        UnitInterval::new(0.3).expect("0.3 is in [0, 1]"),
        "S(scalar(0.5) ; scalar(0.6)) is [[0.5 · 0.6]]"
    );

    // `S(copy ⊗ add)` on `Tropical`: (1+2) → (2+1), so a 3×3 matrix.
    let spread = SignalFlowGraph::<Tropical>::copy().tensor(&SignalFlowGraph::<Tropical>::add());
    let spread = sfg_to_mat(&spread).expect("the tensor is well-formed");
    assert_eq!((spread.rows(), spread.cols()), (3, 3));

    // Copy ; (discard ⊗ discard) is 1 → 0, so its matrix is 1×0.
    let collapsed = SignalFlowGraph::<F64Rig>::copy()
        .compose(
            &SignalFlowGraph::<F64Rig>::discard().tensor(&SignalFlowGraph::<F64Rig>::discard()),
        )
        .expect("copy : 1 → 2 meets discard ⊗ discard : 2 → 0");
    let collapsed = sfg_to_mat(&collapsed).expect("the composite is well-formed");
    assert_eq!((collapsed.rows(), collapsed.cols()), (1, 0));
}

// ---------------------------------------------------------------------------
// The three functor squares
// ---------------------------------------------------------------------------

/// `S(f ; g) == S(f) · S(g)` on every composable ordered pair of the operand
/// pool; returns the pair count.
fn compose_square<R: TestRig>(rig: &str, scalars: &[R]) -> usize {
    let pool = operand_pool(scalars);
    let mut pairs = 0usize;
    for f in &pool {
        for g in &pool {
            if f.target() != g.source() {
                continue;
            }
            let composed = Free::compose(f.clone(), g.clone())
                .expect("the boundary arities were just checked equal");
            let observed = sfg_to_mat(&SignalFlowGraph::from_prop_expr(composed))
                .expect("the composite is well-formed");
            let lhs =
                sfg_to_mat(&SignalFlowGraph::from_prop_expr(f.clone())).expect("f is well-formed");
            let rhs =
                sfg_to_mat(&SignalFlowGraph::from_prop_expr(g.clone())).expect("g is well-formed");
            let expected = lhs
                .matmul(&rhs)
                .expect("S preserves arities, so the shapes meet");
            assert_eq!(
                observed, expected,
                "{rig}: S(f ; g) != S(f) · S(g)\n  f = {f:?}\n  g = {g:?}\n  observed \
                 {observed:?}\n  expected {expected:?}"
            );
            pairs += 1;
        }
    }
    pairs
}

/// `S(f ⊗ g) == S(f) ⊕ S(g)` on every ordered pair of the operand pool;
/// returns the pair count.
fn tensor_square<R: TestRig>(rig: &str, scalars: &[R]) -> usize {
    let pool = operand_pool(scalars);
    let mut pairs = 0usize;
    for f in &pool {
        for g in &pool {
            let tensored = Free::tensor(f.clone(), g.clone());
            let observed = sfg_to_mat(&SignalFlowGraph::from_prop_expr(tensored))
                .expect("the tensor is well-formed");
            let lhs =
                sfg_to_mat(&SignalFlowGraph::from_prop_expr(f.clone())).expect("f is well-formed");
            let rhs =
                sfg_to_mat(&SignalFlowGraph::from_prop_expr(g.clone())).expect("g is well-formed");
            let expected = lhs.block_diagonal(&rhs);
            assert_eq!(
                observed, expected,
                "{rig}: S(f ⊗ g) != S(f) ⊕ S(g)\n  f = {f:?}\n  g = {g:?}\n  observed \
                 {observed:?}\n  expected {expected:?}"
            );
            pairs += 1;
        }
    }
    pairs
}

/// The `compose` and `tensor` squares on the four rigs, with the pool and pair
/// counts pinned.
#[test]
fn s_functor_commutes_with_compose_and_tensor() {
    assert_eq!(
        operand_pool(&f64_scalars()).len(),
        POOL_SIZE,
        "the operand pool size moved"
    );
    assert_eq!(
        (
            compose_square("BoolRig", &bool_scalars()),
            compose_square("UnitInterval", &unit_scalars()),
            compose_square("Tropical", &tropical_scalars()),
            compose_square("F64Rig", &f64_scalars()),
        ),
        (
            COMPOSABLE_PAIRS,
            COMPOSABLE_PAIRS,
            COMPOSABLE_PAIRS,
            COMPOSABLE_PAIRS
        ),
        "the composable-pair count moved"
    );
    let all_pairs = POOL_SIZE * POOL_SIZE;
    assert_eq!(
        (
            tensor_square("BoolRig", &bool_scalars()),
            tensor_square("UnitInterval", &unit_scalars()),
            tensor_square("Tropical", &tropical_scalars()),
            tensor_square("F64Rig", &f64_scalars()),
        ),
        (all_pairs, all_pairs, all_pairs, all_pairs),
        "the tensor-pair count moved"
    );
}

/// The number of composable ordered pairs in the operand pool.
const COMPOSABLE_PAIRS: usize = 5990;

/// An `n → n` signal flow graph (`n ≥ 2`) used as the witness `f` in the
/// `permute_side` square:
///
/// ```text
/// (Copy ⊗ id_{n-1}) ; (id_1 ⊗ Add ⊗ id_{n-2}) ; (Scalar(2) ⊗ id_{n-2} ⊗ Scalar(3))
/// ```
///
/// Its matrix is upper triangular with diagonal `2, 1, …, 1, 3` and a single
/// off-diagonal `1` at `[0][1]`. Both properties are load-bearing:
///
/// - **Not symmetric** (`[0][1] = 1` but `[1][0] = 0`). A symmetric `f` — and
///   the identity is the worst case — can satisfy `M · P = P^T · M` for
///   permutations that an inverted convention would swap.
/// - **Invertible** (triangular, non-zero diagonal). Then `M · P` determines
///   `P` and `P^T · M` determines `P^T`, so every case in the sweep
///   discriminates on both sides.
fn discriminating_sfg(n: usize) -> SignalFlowGraph<F64Rig> {
    assert!(n >= 2, "the construction needs at least two wires");
    let copy = SignalFlowGraph::<F64Rig>::copy().tensor(&SignalFlowGraph::identity(n - 1));
    let add = SignalFlowGraph::<F64Rig>::identity(1)
        .tensor(&SignalFlowGraph::add())
        .tensor(&SignalFlowGraph::identity(n - 2));
    let scale = SignalFlowGraph::<F64Rig>::scalar(F64Rig(2.0))
        .tensor(&SignalFlowGraph::identity(n - 2))
        .tensor(&SignalFlowGraph::scalar(F64Rig(3.0)));
    copy.compose(&add)
        .expect("Copy ⊗ id_{n-1} : n → n+1 meets id_1 ⊗ Add ⊗ id_{n-2} : n+1 → n")
        .compose(&scale)
        .expect("both factors are n → n")
}

/// Run the `S`-functor square at width `n` for every permutation and both
/// sides; returns the number of cases checked.
fn check_permute_side_square(n: usize) -> usize {
    let f = discriminating_sfg(n);
    let base = sfg_to_mat(&f).expect("the witness SFG is well-formed");
    assert_eq!(base.rows(), n);
    assert_eq!(base.cols(), n);
    assert_ne!(
        base.entries()[0][1],
        base.entries()[1][0],
        "the witness matrix must be non-symmetric or it cannot discriminate"
    );

    let mut checked = 0usize;
    for v in all_perm_indices(n) {
        let p = Permutation::try_from(v).expect("all_perm_indices yields valid permutations");
        for of_codomain in [false, true] {
            let mut expr = f.as_prop_expr().clone();
            expr.permute_side(&p, of_codomain);
            // Braids are endomorphisms, so the arities are untouched.
            assert_eq!(expr.source(), n);
            assert_eq!(expr.target(), n);

            let lhs = sfg_to_mat(&SignalFlowGraph::from_prop_expr(expr))
                .expect("the permuted expression is well-formed");
            let mut rhs = base.clone();
            rhs.permute_side(&p, of_codomain);

            assert_eq!(
                lhs, rhs,
                "S-functor square failed for {p:?} (of_codomain = {of_codomain})\n  observed \
                 {lhs:?}\n  expected {rhs:?}"
            );
            checked += 1;
        }
    }
    checked
}

/// `S(f.permute_side(p, side)) == S(f).permute_side(p, side)` for every element
/// of `S₃` and `S₄` on both sides, and the structural counterpart naming which
/// braiding lands on which side.
#[test]
fn s_functor_commutes_with_permute_side() {
    // 6 × 2 and 24 × 2: the assertion that pins the direction on both sides.
    assert_eq!(check_permute_side_square(3), 12);
    assert_eq!(check_permute_side_square(4), 48);

    // The structural counterpart. A 3-cycle is not its own inverse, so the two
    // spliced networks are genuinely different terms.
    let base: PropExpr<Sig> = Free::identity(3);
    let p = Permutation::try_from(vec![1, 2, 0]).expect("valid permutation");
    let types: Vec<()> = vec![(); 3];
    let braid = |q: Permutation| -> PropExpr<Sig> {
        <PropExpr<Sig> as SymmetricMonoidalMorphism<()>>::from_permutation_on_domain(q, &types)
            .expect("length matches")
    };
    let forward = braid(p.clone());
    let inverse = braid(p.inv());
    assert_ne!(
        forward, inverse,
        "a 3-cycle discriminates the two directions"
    );

    let mut cod = base.clone();
    cod.permute_side(&p, /* of_codomain = */ true);
    assert_eq!(
        cod,
        PropExpr::Compose(Box::new(base.clone()), Box::new(forward)),
        "codomain side postcomposes from_permutation_on_domain(p)"
    );

    let mut dom = base.clone();
    dom.permute_side(&p, /* of_codomain = */ false);
    assert_eq!(
        dom,
        PropExpr::Compose(Box::new(inverse), Box::new(base.clone())),
        "domain side precomposes from_permutation_on_domain(p.inv()) — P^T = P^-1"
    );

    // Neither branch may hand back `self`, nor splice the empty braid.
    for of_codomain in [false, true] {
        let mut e = base.clone();
        e.permute_side(&p, of_codomain);
        assert_ne!(e, base, "permute_side must change the expression");
        let (l, r) = match &e {
            PropExpr::Compose(l, r) => (l.as_ref(), r.as_ref()),
            other => panic!("expected a Compose, got {other:?}"),
        };
        assert_ne!(*l, PropExpr::Braid(0, 3));
        assert_ne!(*r, PropExpr::Braid(0, 3));
    }

    // Defensive no-op: the trait signature is non-fallible, so a length
    // mismatch must not panic and must not half-apply anything.
    let too_short = Permutation::transposition(2, 0, 1); // len 2 vs 3 wires
    for of_codomain in [false, true] {
        let mut e = base.clone();
        e.permute_side(&too_short, of_codomain);
        assert_eq!(e, base);
    }
    let mul: PropExpr<Sig> = Free::generator(Sig::Mul);
    let mut cod_mul = mul.clone();
    cod_mul.permute_side(&too_short, /* of_codomain = */ true);
    assert_eq!(cod_mul, mul, "len 2 does not match the 1-wire codomain");

    // …and on a term whose two arities differ, the domain side accepts the
    // matching length and leaves both arities alone.
    let mut dom_mul: PropExpr<Sig> = Free::generator(Sig::Mul);
    dom_mul.permute_side(&too_short, /* of_codomain = */ false);
    assert_eq!((dom_mul.source(), dom_mul.target()), (2, 1));
    let mut swapped: PropExpr<Sig> = Free::identity(2);
    swapped.permute_side(&too_short, /* of_codomain = */ true);
    assert_eq!((swapped.source(), swapped.target()), (2, 2));
}

// ---------------------------------------------------------------------------
// Prop 5.56: mat_to_sfg round-trips through S
// ---------------------------------------------------------------------------

/// `S(mat_to_sfg(M)) == M`, plus the domain/codomain arities `rows → cols`.
fn assert_roundtrip<R: TestRig>(m: &MatR<R>) {
    let g = mat_to_sfg(m).expect("mat_to_sfg is arity-safe for well-formed MatR");
    assert_eq!(g.domain(), m.rows(), "domain arity = rows");
    assert_eq!(g.codomain(), m.cols(), "codomain arity = cols");
    let observed = sfg_to_mat(&g).expect("sfg_to_mat succeeds on the constructed SFG");
    assert_eq!(
        observed, *m,
        "S(mat_to_sfg(M)) must equal M\n  observed {observed:?}\n  expected {m:?}"
    );
    // The realization also passes the independent evaluator, so a `mat_to_sfg`
    // and an `sfg_to_mat` that drifted together would still be caught.
    let by_evaluation = matrix_by_evaluation(g.as_prop_expr());
    assert_eq!(
        by_evaluation, *m,
        "the basis-vector evaluator disagrees with M on mat_to_sfg(M)\n  observed \
         {by_evaluation:?}\n  expected {m:?}"
    );
}

/// Build a `MatR<F64Rig>` from `f64` rows.
fn matf(entries: &[&[f64]]) -> MatR<F64Rig> {
    let rows = entries.len();
    let cols = entries.first().map_or(0, |r| r.len());
    let data = entries
        .iter()
        .map(|r| r.iter().map(|&x| F64Rig(x)).collect())
        .collect();
    MatR::new(rows, cols, data).expect("rectangular fixture")
}

/// The Prop 5.56 pins: the Eq 5.57 2×2 template, Exercise 5.58's three
/// matrices, the empty-dimension shapes, and the `Tropical` rig zero.
#[test]
fn prop_5_56_roundtrip_on_the_paper_matrices() {
    // Eq 5.57's generic 2×2 template [[a, b], [c, d]].
    assert_roundtrip(&matf(&[&[2.0, 3.0], &[5.0, 7.0]]));

    // Exercise 5.58's three matrices.
    assert_roundtrip(&matf(&[&[0.0], &[1.0], &[2.0]]));
    assert_roundtrip(&MatR::<F64Rig>::zero_matrix(2, 2));
    assert_roundtrip(&matf(&[&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]]));

    // Empty dimensions degenerate naturally.
    assert_roundtrip(&MatR::<F64Rig>::new(0, 0, vec![]).expect("0×0 fixture"));
    assert_roundtrip(
        &MatR::<F64Rig>::new(3, 0, vec![vec![], vec![], vec![]]).expect("3×0 fixture"),
    );
    assert_roundtrip(&MatR::<F64Rig>::new(0, 3, vec![]).expect("0×3 fixture"));
    assert_roundtrip(&matf(&[&[42.0]]));

    // `Tropical::zero()` (`+∞`) at 1×1, at 2×2 all-zero, and beside
    // `Tropical::one()` and `Tropical(3.0)` in a mixed 2×2.
    let zero = Tropical::zero();
    assert_roundtrip(&MatR::new(1, 1, vec![vec![zero]]).expect("1×1 fixture"));
    assert_roundtrip(&MatR::<Tropical>::zero_matrix(2, 2));
    assert_roundtrip(
        &MatR::new(
            2,
            2,
            vec![vec![zero, Tropical::one()], vec![Tropical(3.0), zero]],
        )
        .expect("2×2 fixture"),
    );
}

/// Generate a `MatR<R>` up to 4×4 from a per-rig entry strategy.
fn arb_matrix<R, S>(entry: S) -> impl Strategy<Value = MatR<R>>
where
    R: Rig + Debug + 'static,
    S: Strategy<Value = R> + Clone + 'static,
{
    (0usize..=4, 0usize..=4).prop_flat_map(move |(rows, cols)| {
        proptest::collection::vec(
            proptest::collection::vec(entry.clone(), cols..=cols),
            rows..=rows,
        )
        .prop_map(move |data| MatR::new(rows, cols, data).expect("rectangular by construction"))
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, ..ProptestConfig::default() })]

    #[test]
    fn prop_5_56_roundtrip_bool(m in arb_matrix::<BoolRig, _>(any::<bool>().prop_map(BoolRig))) {
        assert_roundtrip(&m);
    }

    /// Tropical (min, +) over the finite dyadic values `0.0, 1.0, 2.0, 3.0`
    /// and the rig zero `Tropical::zero()` (`+∞`) — round-trips exactly since
    /// each entry passes through a single path (tropical-mul by one = `+0.0`,
    /// tropical-add with zero = `min(x, +∞) = x`).
    #[test]
    fn prop_5_56_roundtrip_tropical(
        m in arb_matrix::<Tropical, _>(prop::sample::select(vec![
            Tropical(0.0),
            Tropical(1.0),
            Tropical(2.0),
            Tropical(3.0),
            Tropical::zero(),
        ]))
    ) {
        assert_roundtrip(&m);
    }

    /// Unit interval (max, ·) over dyadic values in `[0, 1]` — round-trips
    /// exactly (mul by one = `·1.0`, add with zero = `max(x, 0.0) = x`).
    #[test]
    fn prop_5_56_roundtrip_unit_interval(
        m in arb_matrix::<UnitInterval, _>(
            prop::sample::select(vec![0.0f64, 0.25, 0.5, 0.75, 1.0])
                .prop_map(|x| UnitInterval::new(x).expect("dyadic in [0,1]"))
        )
    ) {
        assert_roundtrip(&m);
    }

    /// F64Rig over a bounded finite range — round-trips exactly because each
    /// entry's single path is `x · 1.0` summed with `0.0` terms (both exact in
    /// IEEE-754 for finite `x`).
    #[test]
    fn prop_5_56_roundtrip_f64(m in arb_matrix::<F64Rig, _>((-100.0f64..100.0).prop_map(F64Rig))) {
        assert_roundtrip(&m);
    }
}

// ---------------------------------------------------------------------------
// Def 5.2 / 5.25: the free prop's arities, on a second signature
// ---------------------------------------------------------------------------

/// A two-generator signature: `Mul : 2 → 1` and `Unit : 0 → 1`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
enum Sig {
    Mul,
    Unit,
}

impl PropSignature for Sig {
    type Color = ();

    fn source_word(&self) -> Cow<'_, [()]> {
        mono_word(self.source())
    }
    fn target_word(&self) -> Cow<'_, [()]> {
        mono_word(self.target())
    }
    fn source(&self) -> usize {
        match self {
            Sig::Mul => 2,
            Sig::Unit => 0,
        }
    }
    fn target(&self) -> usize {
        match self {
            Sig::Mul | Sig::Unit => 1,
        }
    }
}

/// The Def 5.2 / 5.25 arity contract: a generator carries its declared arity,
/// identity and braid are endomorphisms, composition checks the shared
/// boundary, tensor sums both sides, and the `Composable` / `Monoidal` /
/// `HasIdentity` trait routes agree with the `Free` constructors.
#[test]
fn free_prop_arities_follow_the_signature() {
    let mul: PropExpr<Sig> = Free::generator(Sig::Mul);
    assert_eq!((mul.source(), mul.target()), (2, 1));
    let unit: PropExpr<Sig> = Free::generator(Sig::Unit);
    assert_eq!((unit.source(), unit.target()), (0, 1));

    let id: PropExpr<Sig> = Free::identity(3);
    assert_eq!((id.source(), id.target()), (3, 3));
    let braid: PropExpr<Sig> = Free::braid(2, 3);
    assert_eq!((braid.source(), braid.target()), (5, 5));

    // Composition checks the shared boundary: `Mul ; Mul` has 1 != 2.
    assert!(Free::compose(mul.clone(), mul.clone()).is_err());
    let sequenced =
        Free::compose(mul.clone(), Free::identity(1)).expect("1 → 1 meets Mul's codomain");
    assert_eq!((sequenced.source(), sequenced.target()), (2, 1));

    // Tensor sums both sides.
    let tensored = Free::tensor(mul.clone(), unit.clone());
    assert_eq!((tensored.source(), tensored.target()), (2, 2));

    // The trait routes agree with the `Free` constructors.
    let obj: Vec<()> = vec![(); 3];
    let via_trait: PropExpr<Sig> = <PropExpr<Sig> as HasIdentity<Vec<()>>>::identity(&obj);
    assert_eq!(via_trait, Free::<Sig>::identity(3));
    assert!(mul.compose(&mul).is_err());
    let via_composable = mul
        .compose(&Free::identity(1))
        .expect("1 → 1 meets Mul's codomain");
    assert_eq!(
        (via_composable.domain(), via_composable.codomain()),
        (vec![(); 2], vec![(); 1])
    );
    let mut in_place: PropExpr<Sig> = Free::generator(Sig::Mul);
    in_place.monoidal(Free::generator(Sig::Unit));
    assert_eq!((in_place.source(), in_place.target()), (2, 2));
    assert_eq!(in_place, tensored);

    // `permute_side` leaves both arities alone on an endomorphism and on a
    // term whose two sides differ.
    let swap = Permutation::transposition(2, 0, 1);
    let mut endo: PropExpr<Sig> = Free::identity(2);
    endo.permute_side(&swap, /* of_codomain = */ true);
    assert_eq!((endo.source(), endo.target()), (2, 2));
    let mut non_endo: PropExpr<Sig> = Free::generator(Sig::Mul);
    non_endo.permute_side(&swap, /* of_codomain = */ false);
    assert_eq!((non_endo.source(), non_endo.target()), (2, 1));

    // `from_permutation` validates the length against the type word.
    let three: Vec<()> = vec![(); 3];
    assert!(
        <PropExpr<Sig> as SymmetricMonoidalMorphism<()>>::from_permutation_on_domain(
            swap.clone(),
            &three
        )
        .is_err()
    );
    assert!(
        <PropExpr<Sig> as SymmetricMonoidalMorphism<()>>::from_permutation_on_codomain(
            swap, &three
        )
        .is_err()
    );
}

/// #180: the arity fold saturates instead of overflowing. Raw variant
/// construction is documented-legal, so `Braid(usize::MAX, 1)` and a `Tensor`
/// of two huge-arity halves are both reachable; an unchecked `+` would panic in
/// a debug build and wrap to a small, spuriously valid arity in release.
#[test]
fn arity_fold_saturates_instead_of_overflowing() {
    let b: PropExpr<Sig> = PropExpr::Braid(usize::MAX, 1);
    assert_eq!((b.source(), b.target()), (usize::MAX, usize::MAX));

    let wide = Free::tensor(PropExpr::<Sig>::Braid(usize::MAX, 0), PropExpr::Braid(1, 0));
    assert_eq!((wide.source(), wide.target()), (usize::MAX, usize::MAX));

    let nested = Free::tensor(
        Free::tensor(
            PropExpr::<Sig>::Identity(usize::MAX),
            PropExpr::Identity(usize::MAX),
        ),
        PropExpr::Identity(1),
    );
    assert_eq!((nested.source(), nested.target()), (usize::MAX, usize::MAX));

    // The saturated arity is still just an arity: composition against a real
    // term reports a mismatch rather than being accepted.
    assert!(Free::compose(b, Free::<Sig>::identity(2)).is_err());
}

// ---------------------------------------------------------------------------
// #252: from_permutation realizes the permutation
// ---------------------------------------------------------------------------

/// Build the braiding for `p` over the SFG signature and push it through `S`.
///
/// Also pins the two conventions the rustdoc states: the arities are `n → n`,
/// and the two `from_permutation` constructors coincide on this single-sorted
/// carrier (objects are `Vec<()>`, so there is no label to place on either
/// side).
fn realized_matrix(p: &Permutation) -> MatR<F64Rig> {
    type Sfg = PropExpr<SfgGenerator<F64Rig>>;
    let types: Vec<()> = vec![(); p.len()];
    let expr =
        <Sfg as SymmetricMonoidalMorphism<()>>::from_permutation_on_domain(p.clone(), &types)
            .expect("length matches");
    let on_cod =
        <Sfg as SymmetricMonoidalMorphism<()>>::from_permutation_on_codomain(p.clone(), &types)
            .expect("length matches");
    assert_eq!(
        expr, on_cod,
        "single-sorted carrier: the two constructors must coincide"
    );
    assert_eq!(expr.source(), p.len(), "braiding is an endomorphism of n");
    assert_eq!(expr.target(), p.len(), "braiding is an endomorphism of n");
    sfg_to_mat(&SignalFlowGraph::from_prop_expr(expr)).expect("the braiding is well-formed")
}

/// Assert the oracle for one permutation.
fn assert_realizes(p: &Permutation) {
    let observed = realized_matrix(p);
    let expected = MatR::<F64Rig>::permutation_matrix(p);
    assert_eq!(
        observed, expected,
        "from_permutation must realize {p:?}\n  observed {observed:?}\n  expected {expected:?}"
    );
}

/// `S(from_permutation(p)) == MatR::permutation_matrix(p)` on every element of
/// `S₃` and `S₄`, on the named shapes, and on the identity at four widths.
#[test]
fn from_permutation_realizes_every_permutation_of_three_and_four_wires() {
    // No swaps to perform, at every width — including the two degenerate ones.
    for n in [0usize, 1, 2, 5] {
        let types: Vec<()> = vec![(); n];
        let e: PropExpr<Sig> =
            <PropExpr<Sig> as SymmetricMonoidalMorphism<()>>::from_permutation_on_domain(
                Permutation::identity(n),
                &types,
            )
            .expect("length matches");
        assert_eq!(e, PropExpr::Identity(n), "identity perm on {n} wires");
    }
    for n in [0usize, 1, 3] {
        assert_realizes(&Permutation::identity(n));
    }

    // Named shapes: a single transposition at two widths, the full reversal on
    // 4 wires (the longest sort word), and a 3-cycle inside n = 4 — neither an
    // involution nor a single cycle, so the inverse convention would differ.
    assert_realizes(&Permutation::transposition(2, 0, 1));
    assert_realizes(&Permutation::transposition(4, 1, 3));
    assert_realizes(&Permutation::try_from(vec![3, 2, 1, 0]).expect("valid permutation"));
    let three_cycle = Permutation::try_from(vec![1, 2, 0, 3]).expect("valid permutation");
    assert_realizes(&three_cycle);
    assert_ne!(
        MatR::<F64Rig>::permutation_matrix(&three_cycle),
        MatR::<F64Rig>::permutation_matrix(&three_cycle.inv()),
        "the oracle pins the direction rather than accepting either convention"
    );

    // 6 + 24 cases: the assertion that pins faithfulness.
    for (n, expected) in [(3usize, 6usize), (4, 24)] {
        let perms = all_perm_indices(n);
        // `all_perm_indices` is a test helper, so "every permutation" is a
        // claim about it: pin `n!` and distinctness here too.
        assert_eq!(
            perms.len(),
            expected,
            "all_perm_indices({n}) must yield {n}!"
        );
        let mut distinct = perms.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(
            distinct.len(),
            expected,
            "all_perm_indices({n}) must not repeat"
        );
        for v in perms {
            assert_realizes(&Permutation::try_from(v).expect("valid permutation"));
        }
    }

    // A correct-length non-identity permutation is not `Identity(n)`, and the
    // 2-wire case is exactly the one braid layer composed onto the `id_2` seed.
    let types2: Vec<()> = vec![(); 2];
    let swap: PropExpr<Sig> =
        <PropExpr<Sig> as SymmetricMonoidalMorphism<()>>::from_permutation_on_domain(
            Permutation::transposition(2, 0, 1),
            &types2,
        )
        .expect("length matches");
    assert_ne!(swap, PropExpr::Identity(2));
    let expected = Free::compose(
        Free::identity(2),
        Free::tensor(
            Free::tensor(Free::identity(0), Free::braid(1, 1)),
            Free::identity(0),
        ),
    )
    .expect("both factors are 2 → 2");
    assert_eq!(swap, expected);

    let types4: Vec<()> = vec![(); 4];
    let cycle: PropExpr<Sig> =
        <PropExpr<Sig> as SymmetricMonoidalMorphism<()>>::from_permutation_on_domain(
            Permutation::try_from(vec![1, 2, 0, 3]).expect("valid permutation"),
            &types4,
        )
        .expect("length matches");
    assert_ne!(cycle, PropExpr::Identity(4));
    assert_eq!((cycle.source(), cycle.target()), (4, 4));
}

// ---------------------------------------------------------------------------
// #290: compose against the union-find partition reference
// ---------------------------------------------------------------------------

/// Every index word of length `len` into `slots` positions.
fn index_words(slots: usize, len: usize) -> Vec<Vec<usize>> {
    let mut words = vec![Vec::new()];
    for _ in 0..len {
        let mut next = Vec::new();
        for word in &words {
            for target in 0..slots {
                let mut extended = word.clone();
                extended.push(target);
                next.push(extended);
            }
        }
        words = next;
    }
    words
}

/// Every `(apex, dom leg, cod leg)` shape with domain, codomain and apex each
/// at most 2, at the single wire type [`Z`].
fn wiring_corpus() -> Vec<CospanWiring<char>> {
    let mut out = Vec::new();
    for apex in 0..=2usize {
        for dom in 0..=2usize {
            for cod in 0..=2usize {
                for left in index_words(apex, dom) {
                    for right in index_words(apex, cod) {
                        out.push(
                            CospanWiring::new(vec![Z; apex], left.clone(), right)
                                .expect("every corpus leg entry is below the apex size"),
                        );
                    }
                }
            }
        }
    }
    out
}

/// A flat `usize`-valued decoration: empty is `0`, combine is `+`, and
/// pushforward is the identity.
#[derive(Debug)]
struct Counter;

impl Decoration for Counter {
    type Apex = usize;

    fn empty(_n: usize) -> Self::Apex {
        0
    }

    fn combine(a: Self::Apex, b: Self::Apex) -> Self::Apex {
        a + b
    }

    fn pushforward(d: Self::Apex, _quotient: &[usize]) -> Self::Apex {
        d
    }
}

/// The wiring of a decorated cospan, read through the underlying cospan.
fn decorated_wiring(d: &DecoratedCospan<char, Counter>) -> CospanWiring<char> {
    CospanWiring::new(
        d.cospan.middle().to_vec(),
        d.cospan.left_to_middle().to_vec(),
        d.cospan.right_to_middle().to_vec(),
    )
    .expect("invariant: a Cospan's legs are in bounds of its apex")
}

/// The wiring of a Petri net, read through its places and boundary legs.
fn petri_wiring(p: &PetriNet<char>) -> CospanWiring<char> {
    CospanWiring::new(
        p.places().to_vec(),
        p.left_to_place().to_vec(),
        p.right_to_place().to_vec(),
    )
    .expect("invariant: PetriNet::new checks both legs against the places")
}

/// The corpus size, so a corpus that silently shrank is a failure and not a
/// weaker sweep.
#[test]
fn wiring_corpus_has_the_expected_size() {
    let corpus = wiring_corpus();
    // apex 0: only the 0 → 0 shape; apex 1: 9 (dom, cod) shapes, one leg each;
    // apex 2: sum over dom, cod ≤ 2 of 2^dom · 2^cod = 7 · 7.
    assert_eq!(corpus.len(), 1 + 9 + 49);
    assert!(
        corpus.iter().any(|w| w.dom() == [1, 0]),
        "the corpus lost the non-monotone domain leg"
    );
}

/// `DecoratedCospan::compose` on every composable ordered pair of the corpus
/// against the union-find pushout reference, and the laxator on the decoration.
#[test]
fn decorated_cospan_compose_agrees_with_the_partition_reference() {
    let corpus = wiring_corpus();
    let build = |w: &CospanWiring<char>| -> DecoratedCospan<char, Counter> {
        DecoratedCospan::new(
            Cospan::new(w.dom().to_vec(), w.cod().to_vec(), w.apex().to_vec())
                .expect("the corpus legs are in bounds of the apex"),
            1,
        )
    };
    let mut pairs = 0usize;
    for f in &corpus {
        for g in &corpus {
            if f.cod().len() != g.dom().len() {
                continue;
            }
            pairs += 1;
            let composite = build(f)
                .compose(&build(g))
                .expect("the boundary words were just checked equal");
            let observed = decorated_wiring(&composite).signature();
            let expected = f
                .pushout(g)
                .expect("the boundary widths were just checked equal")
                .signature();
            assert_eq!(
                observed, expected,
                "DecoratedCospan::compose disagrees with the partition reference\n  f = \
                 {f:?}\n  g = {g:?}\n  observed {observed:?}\n  expected {expected:?}"
            );
            // The laxator runs on every composite: `1 + 1`, pushed forward by
            // the identity.
            assert_eq!(composite.decoration, 2);
        }
    }
    assert_eq!(pairs, 1371, "the composable-pair count moved");
}

/// `PetriNet::compose` on every composable ordered pair of the corpus against
/// the same reference.
#[test]
fn petri_net_compose_agrees_with_the_partition_reference() {
    let corpus = wiring_corpus();
    let build = |w: &CospanWiring<char>| -> PetriNet<char> {
        PetriNet::new(
            w.apex().to_vec(),
            Vec::new(),
            w.dom().to_vec(),
            w.cod().to_vec(),
        )
        .expect("the corpus legs are in bounds of the places")
    };
    let mut pairs = 0usize;
    for f in &corpus {
        for g in &corpus {
            if f.cod().len() != g.dom().len() {
                continue;
            }
            pairs += 1;
            let composite = build(f)
                .compose(&build(g))
                .expect("the boundary words were just checked equal");
            let observed = petri_wiring(&composite).signature();
            let expected = f
                .pushout(g)
                .expect("the boundary widths were just checked equal")
                .signature();
            assert_eq!(
                observed, expected,
                "PetriNet::compose disagrees with the partition reference\n  f = {f:?}\n  g = \
                 {g:?}\n  observed {observed:?}\n  expected {expected:?}"
            );
        }
    }
    assert_eq!(pairs, 1371, "the composable-pair count moved");
}

/// The Petri decoration rides the same pushout: composing `mu` against `delta`
/// glues both apex places into one class, and each net's transition arcs are
/// relabeled onto it.
#[test]
fn petri_net_compose_relabels_transition_arcs_onto_the_pushout_classes() {
    let one = Decimal::ONE;
    let two = Decimal::TWO;
    // mu: two domain wires and one codomain wire, both on the single place.
    let mu = PetriNet::new(
        vec![Z],
        vec![Transition::new(vec![(0, one)], vec![(0, two)])],
        vec![0, 0],
        vec![0],
    )
    .expect("the legs index the single place");
    // delta: one domain wire and two codomain wires, on its own single place.
    let delta = PetriNet::new(
        vec![Z],
        vec![Transition::new(vec![(0, two)], vec![(0, one)])],
        vec![0],
        vec![0, 0],
    )
    .expect("the legs index the single place");

    // `to_decorated_cospan` exposes the net's places and transitions verbatim,
    // and `from_decorated_cospan` reads them back.
    let bridged: DecoratedCospan<char, PetriDecoration<char>> = mu.to_decorated_cospan();
    assert_eq!(
        bridged.decoration,
        PetriApex {
            n: 1,
            transitions: vec![Transition::new(vec![(0, one)], vec![(0, two)])],
        }
    );
    let back = PetriNet::from_decorated_cospan(bridged);
    assert_eq!(
        (
            back.places(),
            back.transitions(),
            back.left_to_place(),
            back.right_to_place()
        ),
        (
            mu.places(),
            mu.transitions(),
            mu.left_to_place(),
            mu.right_to_place()
        )
    );

    let composite = mu
        .compose(&delta)
        .expect("mu's codomain meets delta's domain");
    assert_eq!(
        composite.places().len(),
        1,
        "the two apex places glue into one class"
    );
    // `combine` shifted delta's arcs to place 1, and the pushforward mapped
    // both places onto class 0.
    assert_eq!(
        composite.transitions(),
        [
            Transition::new(vec![(0, one)], vec![(0, two)]),
            Transition::new(vec![(0, two)], vec![(0, one)]),
        ],
        "the composite's arcs are both on the merged class"
    );
    assert_eq!(
        (composite.left_to_place(), composite.right_to_place()),
        (&[0, 0][..], &[0, 0][..])
    );
}

// ---------------------------------------------------------------------------
// #371: zero_matrix fills the rig's zero
// ---------------------------------------------------------------------------

/// `zero_matrix` fills `R::zero()`, on a rig whose zero is not `0.0`.
///
/// `Tropical::zero()` is `+∞` and `Tropical::one()` is `0.0`, so a constructor
/// filling `one()` — or a literal `0.0` — is a different matrix here while it
/// would be indistinguishable on `F64Rig`.
#[test]
fn zero_matrix_entries_are_the_rig_zero_on_tropical() {
    let expected = Tropical::zero();
    assert_eq!(expected, Tropical(f64::INFINITY), "Tropical's zero is +∞");
    assert_ne!(expected, Tropical::one(), "Tropical's zero is not its one");

    let mat = MatR::<Tropical>::zero_matrix(2, 3);
    assert_eq!((mat.rows(), mat.cols()), (2, 3));
    assert_eq!(
        mat.entries().to_vec(),
        vec![vec![expected; 3]; 2],
        "MatR::zero_matrix must fill Tropical::zero() (+∞), observed {:?}",
        mat.entries()
    );

    let kron = MatKron::<Tropical>::zero_matrix(2, 3);
    assert_eq!((kron.rows(), kron.cols()), (2, 3));
    assert_eq!(
        kron.entries().to_vec(),
        vec![vec![expected; 3]; 2],
        "MatKron::zero_matrix must fill Tropical::zero() (+∞), observed {:?}",
        kron.entries()
    );

    // The degenerate shapes carry no entries at all.
    assert!(MatR::<Tropical>::zero_matrix(0, 3).entries().is_empty());
    assert_eq!(
        MatR::<Tropical>::zero_matrix(3, 0).entries().to_vec(),
        vec![Vec::new(); 3]
    );
}
