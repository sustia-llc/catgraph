use catgraph::category::{Composable, HasIdentity};
use catgraph::monoidal::Monoidal;
use catgraph_applied::temperley_lieb::{BrauerMorphism, Pair};

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

/// Run `f`, replacing a panic raised inside it with one naming `what` and
/// carrying the original payload text.
fn labelled<R>(what: &str, f: impl FnOnce() -> R) -> R {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(value) => value,
        Err(payload) => {
            let text = if let Some(s) = payload.downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "non-string panic payload".to_string()
            };
            panic!("{what}: {text}");
        }
    }
}

/// A morphism's `terms()` as `(coefficient, δ power, arcs)`, ordered by
/// `(δ power, arcs)` so the expected lists below can be written down.
///
/// Checks on the way past that each arc slice already arrives in the canonical
/// form those lists are written in — every [`Pair`] ascending, the slice
/// sorted — so the ordering is pinned rather than assumed.
fn sorted_terms(what: &str, m: &BrauerMorphism<i64>) -> Vec<(i64, usize, Vec<Pair>)> {
    let mut out: Vec<(i64, usize, Vec<Pair>)> = m
        .terms()
        .map(|(coeff, delta_pow, arcs)| {
            let mut canonical: Vec<Pair> = arcs.iter().map(Pair::sort).collect();
            canonical.sort();
            assert_eq!(
                arcs,
                canonical.as_slice(),
                "{what}: terms() handed back {arcs:?}, whose canonical form is {canonical:?}"
            );
            (coeff, delta_pow, canonical)
        })
        .collect();
    out.sort_by(|a, b| (a.1, &a.2).cmp(&(b.1, &b.2)));
    out
}

/// `monoidal` places its right operand beside its left, read off `terms()`
/// rather than off the arities alone.
///
/// **Point convention.** A morphism in Hom(n, m) numbers its domain points
/// `0..n` and its codomain points `n..n+m`. Placing `a ∈ Hom(n1, m1)` left of
/// `b ∈ Hom(n2, m2)` renumbers both onto `0 .. n1+n2+m1+m2`: `a`'s domain
/// points stay put and its codomain points move past `b`'s domain block
/// (`v + n2`); `b`'s domain points move past `a`'s domain block (`v + n1`) and
/// its codomain points past `a`'s domain and codomain blocks (`v + n1 + m1`).
/// δ powers add, since side-by-side placement closes no loop.
///
/// **What this ranges over.** `e_0` of Hom(3, 3) against `id_2`, in both
/// orders, and against the three-term `δ⁰·0 + δ·1 + δ²·1` of Hom(0, 0), in both
/// orders. The δ cases carry the zero-coefficient term `simplify` would drop.
#[test]
fn monoidal_tensor() {
    // e_0 of Hom(3, 3) is (0,1) (2,5) (3,4): a cup on the domain, a cap on the
    // codomain, one through-line. id_2 of Hom(2, 2) is (0,2) (1,3).
    let e_0 = BrauerMorphism::<i64>::temperley_lieb_gens(3)[0].clone();
    assert_eq!(
        sorted_terms("e_0 of 3", &e_0),
        vec![(1, 0, vec![Pair(0, 1), Pair(2, 5), Pair(3, 4)])],
        "the e_0 fixture"
    );

    let mut left = e_0.clone();
    labelled("e_0 ⊗ id_2", || {
        left.monoidal(BrauerMorphism::<i64>::identity(&2));
    });
    assert_eq!(left.domain(), 5, "e_0 ⊗ id_2: domain should be 3+2=5");
    assert_eq!(left.codomain(), 5, "e_0 ⊗ id_2: codomain should be 3+2=5");
    assert_eq!(
        sorted_terms("e_0 ⊗ id_2", &left),
        vec![(
            1,
            0,
            vec![Pair(0, 1), Pair(2, 7), Pair(3, 8), Pair(4, 9), Pair(5, 6)]
        )],
        "e_0 ⊗ id_2: e_0's cap moves to (5,6) and id_2's four points to 3, 4, 8, 9"
    );

    let mut right = BrauerMorphism::<i64>::identity(&2);
    labelled("id_2 ⊗ e_0", || right.monoidal(e_0.clone()));
    assert_eq!(right.domain(), 5, "id_2 ⊗ e_0: domain should be 2+3=5");
    assert_eq!(right.codomain(), 5, "id_2 ⊗ e_0: codomain should be 2+3=5");
    assert_eq!(
        sorted_terms("id_2 ⊗ e_0", &right),
        vec![(
            1,
            0,
            vec![Pair(0, 5), Pair(1, 6), Pair(2, 3), Pair(4, 9), Pair(7, 8)]
        )],
        "id_2 ⊗ e_0: id_2's codomain moves to 5, 6 and e_0's cap to (7,8)"
    );

    // Hom(0, 0) contributes no points, so both orders leave e_0's arcs alone
    // and only the δ powers move. `BrauerMorphism` carries no `Add`, so the
    // multi-term operand is this polynomial rather than a sum of generators.
    let delta_poly = BrauerMorphism::<i64>::delta_polynomial(&[0, 1, 1]);
    let e_0_arcs = vec![Pair(0, 1), Pair(2, 5), Pair(3, 4)];
    let want = vec![
        (0, 0, e_0_arcs.clone()),
        (1, 1, e_0_arcs.clone()),
        (1, 2, e_0_arcs),
    ];

    let mut poly_on_the_right = e_0.clone();
    labelled("e_0 ⊗ (δ + δ²)", || {
        poly_on_the_right.monoidal(delta_poly.clone());
    });
    assert_eq!(
        poly_on_the_right.domain(),
        3,
        "e_0 ⊗ (δ + δ²): domain should be 3+0=3"
    );
    assert_eq!(
        poly_on_the_right.codomain(),
        3,
        "e_0 ⊗ (δ + δ²): codomain should be 3+0=3"
    );
    let got = sorted_terms("e_0 ⊗ (δ + δ²)", &poly_on_the_right);
    assert_eq!(
        got.len(),
        3,
        "e_0 ⊗ (δ + δ²) has {} terms, not the 3 the polynomial's δ powers keep apart: {got:?}",
        got.len()
    );
    assert_eq!(got, want, "e_0 ⊗ (δ + δ²): terms");

    let mut poly_on_the_left = delta_poly;
    labelled("(δ + δ²) ⊗ e_0", || poly_on_the_left.monoidal(e_0));
    assert_eq!(
        poly_on_the_left.domain(),
        3,
        "(δ + δ²) ⊗ e_0: domain should be 0+3=3"
    );
    assert_eq!(
        poly_on_the_left.codomain(),
        3,
        "(δ + δ²) ⊗ e_0: codomain should be 0+3=3"
    );
    assert_eq!(
        sorted_terms("(δ + δ²) ⊗ e_0", &poly_on_the_left),
        want,
        "(δ + δ²) ⊗ e_0: terms"
    );
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
