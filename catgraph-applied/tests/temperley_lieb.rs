use catgraph::category::{Composable, HasIdentity};
use catgraph::monoidal::Monoidal;
use catgraph_applied::temperley_lieb::BrauerMorphism;

/// Composing a TL generator with the identity (on either side) returns the generator.
#[test]
fn generator_identity_composition() {
    let n = 4;
    let e_i = BrauerMorphism::<i64>::temperley_lieb_gens(n);
    let id = BrauerMorphism::<i64>::identity(&n);
    for ei in &e_i {
        let left = ei.compose(&id).expect("compose(e_i, id) failed");
        let right = id.compose(ei).expect("compose(id, e_i) failed");
        assert_eq!(&left, ei, "e_i * id should equal e_i");
        assert_eq!(&right, ei, "id * e_i should equal e_i");
    }
}

/// Composing a chain `e_0` * `e_1` * `e_2` * `e_3` in n=5 succeeds with correct domain/codomain.
#[test]
fn long_chain() {
    let n = 5;
    let e_i = BrauerMorphism::<i64>::temperley_lieb_gens(n);
    let chain = e_i[0]
        .compose(&e_i[1])
        .and_then(|z| z.compose(&e_i[2]))
        .and_then(|z| z.compose(&e_i[3]))
        .expect("long chain composition failed");
    assert_eq!(chain.domain(), n);
    assert_eq!(chain.codomain(), n);
}

/// `e_i` * `e_i` produces a result (loop absorption), and it differs from `e_i` itself
/// because the composition introduces a delta factor (delta power increments by 1).
#[test]
fn tl_idempotent_absorbs_loop() {
    let n = 4;
    let e_i = BrauerMorphism::<i64>::temperley_lieb_gens(n);
    for ei in &e_i {
        let squared = ei.compose(ei).expect("e_i * e_i failed");
        assert_eq!(squared.domain(), n);
        assert_eq!(squared.codomain(), n);
        // e_i^2 = delta * e_i, so with i64 coefficients the result differs from e_i
        // because delta is tracked as a symbolic power, not a concrete scalar.
        assert_ne!(
            &squared, ei,
            "e_i^2 should differ from e_i (has delta factor)"
        );
    }
}

/// `s_i` * `s_i` = identity for all symmetric group generators.
#[test]
fn symmetric_involution() {
    let n = 4;
    let s_i = BrauerMorphism::<i64>::symmetric_alg_gens(n);
    let id = BrauerMorphism::<i64>::identity(&n);
    for si in &s_i {
        let squared = si.compose(si).expect("s_i * s_i failed");
        assert_eq!(squared, id, "s_i * s_i should be the identity");
    }
}

/// `e_i` * `s_i` = `e_i` and `s_i` * `e_i` = `e_i` (mixed absorption).
#[test]
fn mixed_absorption() {
    let n = 4;
    let e_i = BrauerMorphism::<i64>::temperley_lieb_gens(n);
    let s_i = BrauerMorphism::<i64>::symmetric_alg_gens(n);
    for idx in 0..n - 1 {
        let es = e_i[idx].compose(&s_i[idx]).expect("e_i * s_i failed");
        let se = s_i[idx].compose(&e_i[idx]).expect("s_i * e_i failed");
        assert_eq!(es, e_i[idx], "e_i * s_i should equal e_i");
        assert_eq!(se, e_i[idx], "s_i * e_i should equal e_i");
    }
}

/// Tensor product of `e_0` (n=3) with identity (size 2) has domain=5, codomain=5.
#[test]
fn monoidal_tensor() {
    let e_i = BrauerMorphism::<i64>::temperley_lieb_gens(3);
    let id2 = BrauerMorphism::<i64>::identity(&2);
    let mut tensored = e_i[0].clone();
    tensored.monoidal(id2);
    assert_eq!(tensored.domain(), 5, "tensor domain should be 3+2=5");
    assert_eq!(tensored.codomain(), 5, "tensor codomain should be 3+2=5");
}

/// TL generators are self-adjoint: `e_i^dagger` = `e_i` (with identity as the conjugate for i64).
#[test]
fn dagger_self_adjoint() {
    let n = 4;
    let e_i = BrauerMorphism::<i64>::temperley_lieb_gens(n);
    for ei in &e_i {
        let dag = ei.dagger(|z| z);
        assert_eq!(
            &dag, ei,
            "e_i should be self-adjoint under trivial conjugation"
        );
    }
}

/// `simplify()` is callable as a method and removes zero-coefficient terms.
#[test]
fn simplify_method() {
    // delta_polynomial(&[0, 0, 1]) represents 0 + 0*delta + 1*delta^2
    // After simplify, the zero-coefficient terms should be removed.
    let mut poly = BrauerMorphism::<i64>::delta_polynomial(&[0, 0, 1]);
    poly.simplify();
    // Verify the polynomial is still well-formed after simplification.
    assert_eq!(poly.domain(), 0);
    assert_eq!(poly.codomain(), 0);
    // A polynomial with only the delta^2 term should differ from the zero polynomial.
    let zero_poly = BrauerMorphism::<i64>::delta_polynomial(&[0]);
    let mut simplified_zero = zero_poly.clone();
    simplified_zero.simplify();
    assert_ne!(
        poly, simplified_zero,
        "nonzero polynomial should differ from zero after simplify"
    );
}

/// Identity composed with itself gives identity.
#[test]
fn identity_self_compose() {
    let n = 5;
    let id = BrauerMorphism::<i64>::identity(&n);
    let id_squared = id.compose(&id).expect("id * id failed");
    assert_eq!(
        id_squared, id,
        "identity composed with itself should be identity"
    );
}

/// Braid relation: `s_i` * s_{i+1} * `s_i` = s_{i+1} * `s_i` * s_{i+1} (Yang-Baxter).
#[test]
fn braid_relation() {
    let n = 5;
    let s_i = BrauerMorphism::<i64>::symmetric_alg_gens(n);
    for i in 0..n - 2 {
        let lhs = s_i[i]
            .compose(&s_i[i + 1])
            .and_then(|z| z.compose(&s_i[i]))
            .expect("s_i * s_{i+1} * s_i failed");
        let rhs = s_i[i + 1]
            .compose(&s_i[i])
            .and_then(|z| z.compose(&s_i[i + 1]))
            .expect("s_{i+1} * s_i * s_{i+1} failed");
        assert_eq!(lhs, rhs, "braid relation should hold for i={i}");
    }
}
