//! Tests for `catgraph_applied::prop` — F&S *Seven Sketches* §5.2 Def 5.2
//! (props) and Def 5.25 (free prop on a signature).

use catgraph::category::{Composable, HasIdentity};
use catgraph::monoidal::{Monoidal, SymmetricMonoidalMorphism};
use catgraph_applied::mat::MatR;
use catgraph_applied::prop::{Free, PropExpr, PropSignature, mono_word};
use catgraph_applied::rig::F64Rig;
use catgraph_applied::sfg::{SfgGenerator, SignalFlowGraph};
use catgraph_applied::sfg_to_mat::sfg_to_mat;
use permutations::Permutation;
use std::borrow::Cow;

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

#[test]
fn generator_carries_declared_arity() {
    let g: PropExpr<Sig> = Free::generator(Sig::Mul);
    assert_eq!(g.source(), 2);
    assert_eq!(g.target(), 1);
}

#[test]
fn identity_has_matching_source_and_target() {
    let i: PropExpr<Sig> = Free::identity(3);
    assert_eq!(i.source(), 3);
    assert_eq!(i.target(), 3);
}

#[test]
fn braid_has_sum_arity() {
    let b: PropExpr<Sig> = Free::braid(2, 3);
    assert_eq!(b.source(), 5);
    assert_eq!(b.target(), 5);
}

#[test]
fn compose_rejects_arity_mismatch() {
    let f: PropExpr<Sig> = Free::generator(Sig::Mul); // 2 -> 1
    let g: PropExpr<Sig> = Free::generator(Sig::Mul); // 2 -> 1
    // f.target() == 1 but g.source() == 2 -> must fail
    assert!(Free::compose(f, g).is_err());
}

#[test]
fn compose_accepts_matching_arities() {
    let f: PropExpr<Sig> = Free::generator(Sig::Mul); // 2 -> 1
    let id: PropExpr<Sig> = Free::identity(1); // 1 -> 1
    let r = Free::compose(f, id).expect("arities match");
    assert_eq!(r.source(), 2);
    assert_eq!(r.target(), 1);
}

#[test]
fn tensor_sums_arities() {
    let f: PropExpr<Sig> = Free::generator(Sig::Mul); // 2 -> 1
    let u: PropExpr<Sig> = Free::generator(Sig::Unit); // 0 -> 1
    let r = Free::tensor(f, u);
    assert_eq!(r.source(), 2);
    assert_eq!(r.target(), 2);
}

/// #180: the arity fold saturates instead of overflowing. Raw variant
/// construction is documented-legal, so `Braid(usize::MAX, 1)` and a `Tensor`
/// of two huge-arity halves are both reachable; in a debug build an unchecked
/// `+` would panic here, in release it would wrap to a small, spuriously valid
/// arity. `usize::MAX` matches no real wire bundle, so the saturated value can
/// only be reported as a mismatch downstream.
#[test]
fn arity_fold_saturates_instead_of_overflowing() {
    // Braid arm, both sides.
    let b: PropExpr<Sig> = PropExpr::Braid(usize::MAX, 1);
    assert_eq!(b.source(), usize::MAX);
    assert_eq!(b.target(), usize::MAX);

    // Tensor arm: the fold over two saturating halves stays saturated.
    let wide = Free::tensor(PropExpr::<Sig>::Braid(usize::MAX, 0), PropExpr::Braid(1, 0));
    assert_eq!(wide.source(), usize::MAX);
    assert_eq!(wide.target(), usize::MAX);

    // Identity-based equivalent, and nesting one level deeper.
    let nested = Free::tensor(
        Free::tensor(
            PropExpr::<Sig>::Identity(usize::MAX),
            PropExpr::Identity(usize::MAX),
        ),
        PropExpr::Identity(1),
    );
    assert_eq!(nested.source(), usize::MAX);
    assert_eq!(nested.target(), usize::MAX);

    // The saturated arity is still just an arity: composition against a real
    // term reports a mismatch rather than being accepted.
    let id: PropExpr<Sig> = Free::identity(2);
    assert!(Free::compose(b, id).is_err());
}

#[test]
fn has_identity_trait_matches_raw_constructor() {
    let obj: Vec<()> = vec![(); 3];
    let id: PropExpr<Sig> = <PropExpr<Sig> as HasIdentity<Vec<()>>>::identity(&obj);
    assert_eq!(id.source(), 3);
    assert_eq!(id.target(), 3);
    // Equivalent to Free::identity(3) via structural equality.
    assert_eq!(id, Free::<Sig>::identity(3));
}

#[test]
fn composable_trait_respects_arity_check() {
    let f: PropExpr<Sig> = Free::generator(Sig::Mul); // 2 -> 1
    let g: PropExpr<Sig> = Free::generator(Sig::Mul); // 2 -> 1
    // Composable::compose is by-reference; arity mismatch still fails.
    assert!(f.compose(&g).is_err());
    let id: PropExpr<Sig> = Free::identity(1);
    let ok = f.compose(&id).expect("arities match");
    assert_eq!(ok.domain(), vec![(); 2]);
    assert_eq!(ok.codomain(), vec![(); 1]);
}

#[test]
fn monoidal_trait_extends_arity_in_place() {
    let mut a: PropExpr<Sig> = Free::generator(Sig::Mul); // 2 -> 1
    let b: PropExpr<Sig> = Free::generator(Sig::Unit); // 0 -> 1
    a.monoidal(b);
    assert_eq!(a.source(), 2);
    assert_eq!(a.target(), 2);
}

#[test]
fn permute_side_preserves_source_and_target() {
    // Swap two wires on the codomain of id_2 — source/target unchanged.
    let mut e: PropExpr<Sig> = Free::identity(2);
    let swap = Permutation::transposition(2, 0, 1);
    e.permute_side(&swap, /* of_codomain = */ true);
    assert_eq!(e.source(), 2);
    assert_eq!(e.target(), 2);
    // …and on the domain side of a *non*-endomorphism, where the two arities
    // genuinely differ, so a spliced braid of the wrong width would show up:
    // `Mul : 2 → 1`, permuted on its 2-wire domain.
    let mut g: PropExpr<Sig> = Free::generator(Sig::Mul);
    g.permute_side(&swap, /* of_codomain = */ false);
    assert_eq!(g.source(), 2);
    assert_eq!(g.target(), 1);
}

#[test]
fn from_permutation_validates_length() {
    let p = Permutation::transposition(2, 0, 1);
    let types: Vec<()> = vec![(); 3]; // len mismatch on purpose
    let r: Result<PropExpr<Sig>, _> =
        <PropExpr<Sig> as SymmetricMonoidalMorphism<()>>::from_permutation_on_domain(
            p.clone(),
            &types,
        );
    assert!(r.is_err());
    let r2: Result<PropExpr<Sig>, _> =
        <PropExpr<Sig> as SymmetricMonoidalMorphism<()>>::from_permutation_on_codomain(p, &types);
    assert!(r2.is_err());
}

// ---- #252: `from_permutation` faithfully realizes the permutation ----------
//
// The oracle is the functor `S` of F&S Thm 5.53: applying `sfg_to_mat` to the
// braiding network must reproduce `MatR::permutation_matrix(p)`, whose
// convention (`entries[i][p.apply(i)] = 1` — row = input wire, column = output
// wire) is the same input-indexed one `from_permutation` documents. Before
// #252 the body returned `Identity(n)` for every permutation, so every one of
// these comparisons would have collapsed to the identity matrix.

/// Build the braiding for `p` over the SFG signature and push it through `S`.
///
/// Also pins the two crate-wide conventions the rustdoc states: the arities are
/// `n → n`, and the #258 constructors coincide on this single-sorted carrier
/// (objects are `Vec<()>`, so there is no label to place on either side).
fn realized_matrix(p: &Permutation) -> MatR<F64Rig> {
    type Sfg = PropExpr<SfgGenerator<F64Rig>>;
    let types: Vec<()> = vec![(); p.len()];
    let expr =
        <Sfg as SymmetricMonoidalMorphism<()>>::from_permutation_on_domain(p.clone(), &types)
            .unwrap();
    let on_cod =
        <Sfg as SymmetricMonoidalMorphism<()>>::from_permutation_on_codomain(p.clone(), &types)
            .unwrap();
    assert_eq!(
        expr, on_cod,
        "single-sorted carrier: the two #258 constructors must coincide"
    );
    assert_eq!(expr.source(), p.len(), "braiding is an endomorphism of n");
    assert_eq!(expr.target(), p.len(), "braiding is an endomorphism of n");
    sfg_to_mat(&SignalFlowGraph::from_prop_expr(expr)).unwrap()
}

/// Assert the #252 oracle for one permutation.
fn assert_realizes(p: &Permutation) {
    assert_eq!(
        realized_matrix(p),
        MatR::<F64Rig>::permutation_matrix(p),
        "from_permutation must realize {p:?}"
    );
}

/// Every permutation of `0..n`, each exactly once (Heap-style prefix swaps).
fn all_perms(n: usize) -> Vec<Vec<usize>> {
    fn go(cur: &mut Vec<usize>, k: usize, out: &mut Vec<Vec<usize>>) {
        if k == cur.len() {
            out.push(cur.clone());
            return;
        }
        for i in k..cur.len() {
            cur.swap(k, i);
            go(cur, k + 1, out);
            cur.swap(k, i);
        }
    }
    let mut cur: Vec<usize> = (0..n).collect();
    let mut out = Vec::new();
    go(&mut cur, 0, &mut out);
    out
}

#[test]
fn from_permutation_identity_is_identity_expr() {
    // No swaps to perform, at every width — including the two degenerate ones.
    for n in [0usize, 1, 2, 5] {
        let types: Vec<()> = vec![(); n];
        let e: PropExpr<Sig> =
            <PropExpr<Sig> as SymmetricMonoidalMorphism<()>>::from_permutation_on_domain(
                Permutation::identity(n),
                &types,
            )
            .unwrap();
        assert_eq!(e, PropExpr::Identity(n), "identity perm on {n} wires");
    }
    // …and the oracle agrees it is the identity matrix.
    for n in [0usize, 1, 3] {
        assert_realizes(&Permutation::identity(n));
    }
}

#[test]
fn from_permutation_realizes_named_shapes() {
    // Single transposition.
    assert_realizes(&Permutation::transposition(2, 0, 1));
    assert_realizes(&Permutation::transposition(4, 1, 3));
    // Full reversal on 4 wires — the longest sort word (6 swaps).
    assert_realizes(&Permutation::try_from(vec![3, 2, 1, 0]).unwrap());
    // Neither an involution nor a single cycle: a 3-cycle inside n = 4, so the
    // sort word is non-trivial and the inverse convention would differ.
    let three_cycle = Permutation::try_from(vec![1, 2, 0, 3]).unwrap();
    assert_realizes(&three_cycle);
    // The inverse is a *different* matrix, so the oracle above genuinely pins
    // the direction rather than accepting either convention.
    assert_ne!(
        MatR::<F64Rig>::permutation_matrix(&three_cycle),
        MatR::<F64Rig>::permutation_matrix(&three_cycle.inv()),
    );
}

#[test]
fn from_permutation_exhaustive_oracle_n3_and_n4() {
    // 6 + 24 cases: the assertion that actually pins faithfulness.
    for (n, expected) in [(3usize, 6usize), (4, 24)] {
        let perms = all_perms(n);
        // `all_perms` is a test helper, so "exhaustive" is a claim about it, not
        // a fact. Pin `n!` and distinctness here: a helper that silently yielded
        // fewer would leave the sweep below covering less than its name says
        // while still passing.
        assert_eq!(perms.len(), expected, "all_perms({n}) must yield {n}!");
        let mut distinct = perms.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), expected, "all_perms({n}) must not repeat");

        for v in perms {
            assert_realizes(&Permutation::try_from(v).unwrap());
        }
    }
}

#[test]
fn from_permutation_is_not_identity_for_non_identity_perm() {
    // The exact #252 defect: a correct-length non-identity permutation used to
    // come back as `Identity(n)`.
    let types2: Vec<()> = vec![(); 2];
    let swap: PropExpr<Sig> =
        <PropExpr<Sig> as SymmetricMonoidalMorphism<()>>::from_permutation_on_domain(
            Permutation::transposition(2, 0, 1),
            &types2,
        )
        .unwrap();
    assert_ne!(swap, PropExpr::Identity(2));
    // One braid layer `id_0 ⊗ σ ⊗ id_0`, composed onto the `id_2` seed.
    let expected = Free::compose(
        Free::identity(2),
        Free::tensor(
            Free::tensor(Free::identity(0), Free::braid(1, 1)),
            Free::identity(0),
        ),
    )
    .unwrap();
    assert_eq!(swap, expected);

    let types4: Vec<()> = vec![(); 4];
    let cycle: PropExpr<Sig> =
        <PropExpr<Sig> as SymmetricMonoidalMorphism<()>>::from_permutation_on_domain(
            Permutation::try_from(vec![1, 2, 0, 3]).unwrap(),
            &types4,
        )
        .unwrap();
    assert_ne!(cycle, PropExpr::Identity(4));
    assert_eq!(cycle.source(), 4);
    assert_eq!(cycle.target(), 4);
}

// ---- #252 (review follow-up): `permute_side` faithfully permutes -----------
//
// The same defect class, in the sibling method of the same trait impl: the old
// body spliced `PropExpr::Braid(0, n)`, i.e. `σ_{0,n}`, which is the identity
// on `n` wires — `presentation::smc_nf`'s Step 0 rewrites `Braid(0, n)` to
// `Identity(n)` outright. So `f.permute_side(&p, side)` returned `f` unchanged
// for *every* `p`, on both sides.
//
// The oracle is the `S`-functor square. `SignalFlowGraph<R>` wraps
// `PropExpr<SfgGenerator<R>>` and `MatR` implements the same trait, with a
// faithful `permute_side` ("right-mul by P permutes columns (codomain side);
// left-mul by P^T permutes rows (domain side)"), so
//
//     sfg_to_mat(f.permute_side(p, side)) == sfg_to_mat(f).permute_side(p, side)
//
// cannot hold under an inverted convention: `MatR` supplies `P` on the codomain
// and `P^T = P^-1` on the domain, and `PropExpr` must match it on both.

/// An `n → n` signal flow graph (`n ≥ 2`) used as the witness `f`:
///
/// ```text
/// (Copy ⊗ id_{n-1}) ; (id_1 ⊗ Add ⊗ id_{n-2}) ; (Scalar(2) ⊗ id_{n-2} ⊗ Scalar(3))
/// ```
///
/// Its matrix is upper triangular with diagonal `2, 1, …, 1, 3` and a single
/// off-diagonal `1` at `[0][1]`. Both properties are load-bearing, and neither
/// is accidental:
///
/// - **Not symmetric** (`[0][1] = 1` but `[1][0] = 0`). A symmetric `f` — and
///   the identity is the worst case — can satisfy `M · P = P^T · M` for
///   permutations that an inverted convention would swap, hiding the defect.
/// - **Invertible** (triangular, non-zero diagonal). Then `M · P` determines
///   `P` and `P^T · M` determines `P^T`, so *every* case in the exhaustive
///   sweep discriminates, on both sides, rather than only the lucky ones.
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
    for v in all_perms(n) {
        let p = Permutation::try_from(v).expect("all_perms yields valid permutations");
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
                "S-functor square failed for {p:?} (of_codomain = {of_codomain})"
            );
            checked += 1;
        }
    }
    checked
}

#[test]
fn permute_side_functor_square_exhaustive_n3_and_n4() {
    // 6 × 2 and 24 × 2: the assertion that pins the direction on both sides.
    assert_eq!(check_permute_side_square(3), 12);
    assert_eq!(check_permute_side_square(4), 48);
}

#[test]
fn permute_side_splices_p_on_the_codomain_and_p_inv_on_the_domain() {
    // The structural counterpart of the functor square, spelling out which
    // braiding lands on which side. A 3-cycle is not its own inverse, so the
    // two spliced networks are genuinely different terms.
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
}

#[test]
fn permute_side_changes_the_expression_for_a_non_identity_perm() {
    // The exact defect: `Braid(0, n)` is `Identity(n)`, so both branches used
    // to hand back `self`. This is the assertion whose absence let it live.
    let base: PropExpr<Sig> = Free::identity(3);
    let p = Permutation::try_from(vec![1, 2, 0]).expect("valid permutation");
    for of_codomain in [false, true] {
        let mut e = base.clone();
        e.permute_side(&p, of_codomain);
        assert_ne!(e, base, "permute_side must change the expression");
        // Nor may the spliced half be the empty braid that used to stand in.
        let (l, r) = match &e {
            PropExpr::Compose(l, r) => (l.as_ref(), r.as_ref()),
            other => panic!("expected a Compose, got {other:?}"),
        };
        assert_ne!(*l, PropExpr::Braid(0, 3));
        assert_ne!(*r, PropExpr::Braid(0, 3));
    }
}

#[test]
fn permute_side_length_mismatch_leaves_self_unchanged() {
    // Defensive no-op: the trait signature is non-fallible, so a caller bug
    // must not panic and must not half-apply anything.
    let base: PropExpr<Sig> = Free::identity(3);
    let too_short = Permutation::transposition(2, 0, 1); // len 2 vs 3 wires
    for of_codomain in [false, true] {
        let mut e = base.clone();
        e.permute_side(&too_short, of_codomain);
        assert_eq!(e, base);
    }
    // Also on a non-endomorphism, where the two sides have different arities:
    // `Mul : 2 → 1` accepts a length-2 perm on its domain but not its codomain.
    let mul: PropExpr<Sig> = Free::generator(Sig::Mul);
    let mut cod = mul.clone();
    cod.permute_side(&too_short, /* of_codomain = */ true);
    assert_eq!(cod, mul, "len 2 does not match the 1-wire codomain");
}
