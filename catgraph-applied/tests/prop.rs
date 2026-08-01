//! Tests for `catgraph_applied::prop` — F&S *Seven Sketches* §5.2 Def 5.2
//! (props) and Def 5.25 (free prop on a signature).

use catgraph::category::{Composable, HasIdentity};
use catgraph::monoidal::{Monoidal, SymmetricMonoidalMorphism};
use catgraph_applied::prop::{Free, PropExpr, PropSignature, mono_word};
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
}

#[test]
fn from_permutation_validates_length() {
    let p = Permutation::transposition(2, 0, 1);
    let types: Vec<()> = vec![(); 3]; // len mismatch on purpose
    let r: Result<PropExpr<Sig>, _> =
        <PropExpr<Sig> as SymmetricMonoidalMorphism<()>>::from_permutation(p, &types, true);
    assert!(r.is_err());
}
