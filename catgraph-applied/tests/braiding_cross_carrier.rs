//! #258 — the braiding convention, checked **across** carriers.
//!
//! Nothing in the workspace used to exercise one permutation through two
//! carriers, and that is exactly how three implementations drifted onto three
//! different conventions without a red test: within a carrier, an inverted
//! braiding is still a perfectly consistent braiding.
//!
//! The contract under test is the one
//! [`SymmetricMonoidalMorphism`](catgraph::monoidal::SymmetricMonoidalMorphism)
//! states: **both** constructors build the wiring `i ↦ p.apply(i)`, differing
//! only in which boundary the caller's `types` slice labels.
//!
//! # Why this cannot pass under a symmetric drift
//!
//! "Carrier A agrees with carrier B" would stay green if both moved the same
//! wrong way. So no assertion here compares two carriers to each other. Every
//! carrier is compared against a reference computed **directly from `p`** —
//! `(0..n).map(|i| p.apply(i))` for the wiring, `p.inv().permute(types)` for
//! the forced labels — and `hand_anchored_reference_values` additionally pins
//! that reference itself against values written out by hand, so the reference
//! cannot drift either.

use catgraph::category::{Composable, ComposableMutating, HasIdentity};
use catgraph::corel::Corel;
use catgraph::cospan::Cospan;
use catgraph::cospan_algebra::PartitionAlgebra;
use catgraph::equivalence::CospanAlgebraMorphism;
use catgraph::finset::Decomposition;
use catgraph::frobenius::{FrobeniusMorphism, from_decomposition};
use catgraph::monoidal::{SymmetricMonoidalDiscreteMorphism, SymmetricMonoidalMorphism};
use catgraph::span::Span;
use catgraph_applied::decorated_cospan::DecoratedCospan;
use catgraph_applied::mat::MatR;
use catgraph_applied::mat_kron::MatKron;
use catgraph_applied::petri_net::{PetriDecoration, PetriNet};
use catgraph_applied::prop::PropExpr;
use catgraph_applied::rig::{F64Rig, One, Rig, Zero};
use catgraph_applied::sfg::{SfgGenerator, SignalFlowGraph};
use catgraph_applied::sfg_to_mat::sfg_to_mat;
use catgraph_testutil::all_perms;
use permutations::Permutation;

type Sfg = PropExpr<SfgGenerator<F64Rig>>;

/// The reference wiring, read straight off `p`: domain wire `i` goes to
/// codomain wire `p.apply(i)`.
fn reference_wiring(p: &Permutation) -> Vec<usize> {
    (0..p.len()).map(|i| p.apply(i)).collect()
}

/// Distinct labels, so a permuted word is distinguishable from an
/// inverse-permuted one. `['A', 'B', 'C', …]`.
fn labels(n: usize) -> Vec<char> {
    (0..n)
        .map(|i| char::from(b'A' + u8::try_from(i).expect("n < 26 in these fixtures")))
        .collect()
}

// ---------------------------------------------------------------------------
// Wiring extractors — one per carrier shape
// ---------------------------------------------------------------------------

/// Domain wire `i` and codomain wire `k` meet when they land on the same apex
/// vertex, i.e. `left[i] == right[k]`.
fn cospan_wiring<L: Eq + Copy + std::fmt::Debug>(c: &Cospan<L>) -> Vec<usize> {
    let (l, r) = (c.left_to_middle(), c.right_to_middle());
    l.iter()
        .map(|li| {
            r.iter()
                .position(|rk| rk == li)
                .expect("a braiding cospan links every domain wire to a codomain wire")
        })
        .collect()
}

/// A span's apex pair `(i, k)` links domain wire `i` to codomain wire `k`.
fn span_wiring<L: Eq + Copy + std::fmt::Debug>(s: &Span<L>) -> Vec<usize> {
    let mut out = vec![usize::MAX; s.domain().len()];
    for &(i, k) in s.middle_pairs() {
        out[i] = k;
    }
    assert!(
        !out.contains(&usize::MAX),
        "every domain wire must be wired"
    );
    out
}

/// Row `i` of a permutation matrix carries its single `one` in column `p(i)`.
fn mat_wiring<R: Rig + PartialEq>(m: &MatR<R>) -> Vec<usize> {
    (0..m.rows())
        .map(|i| {
            let row = &m.entries()[i];
            let hits: Vec<usize> = (0..m.cols()).filter(|j| row[*j] == R::one()).collect();
            assert_eq!(hits.len(), 1, "row {i} of a permutation matrix has one 1");
            hits[0]
        })
        .collect()
}

/// `CospanAlgebraMorphism` over `PartitionAlgebra`: the element is a cospan
/// `[] → apex ← domain ⊕ codomain`, and its right leg is the partition. Domain
/// wire `i` meets codomain wire `k` when `right[i] == right[n + k]`.
fn cam_wiring(m: &CospanAlgebraMorphism<PartitionAlgebra, char>) -> Vec<usize> {
    let n = m.domain().len();
    let right = m.element().right_to_middle();
    assert_eq!(right.len(), 2 * n, "interface is domain ⊕ codomain");
    (0..n)
        .map(|i| {
            (0..n)
                .find(|k| right[n + k] == right[i])
                .expect("a braiding element must pair every domain wire with a codomain wire")
        })
        .collect()
}

/// A `PetriNet`'s boundary legs are the cospan
/// [`PetriNet::to_decorated_cospan`] exposes, so its wiring is read the same
/// way as any other cospan's.
fn petri_wiring(net: &PetriNet<char>) -> Vec<usize> {
    cospan_wiring(&net.to_decorated_cospan().cospan)
}

/// `PropExpr` over the SFG signature, pushed through the functor `S` of
/// F&S Thm 5.53 so it lands in the same shape as a matrix.
fn prop_wiring(e: Sfg) -> Vec<usize> {
    mat_wiring(&sfg_to_mat(&SignalFlowGraph::from_prop_expr(e)).expect("braid network is an SFG"))
}

// ---------------------------------------------------------------------------
// 1. The hand anchor — nothing here is computed by the code under test
// ---------------------------------------------------------------------------

/// `rotation_left(3, 1)` sends `0→1, 1→2, 2→0`, so `p.inv()` sends `k → (k+2)%3`.
///
/// Every number below is written out rather than derived, on both sides of the
/// carrier divide: the cospan's legs and labels, and the matrix's entries. This
/// is what stops a symmetric drift from passing the sweeps that follow — those
/// compare carriers against `reference_wiring`, and this test pins
/// `reference_wiring` itself.
#[test]
fn hand_anchored_reference_values() {
    let p = Permutation::rotation_left(3, 1);
    let types = ['A', 'B', 'C'];

    assert_eq!(reference_wiring(&p), vec![1, 2, 0]);

    // -- Cospan, `types` on the domain -------------------------------------
    let c = Cospan::<char>::from_permutation_on_domain(p.clone(), &types).unwrap();
    assert_eq!(c.domain(), vec!['A', 'B', 'C']);
    assert_eq!(c.codomain(), vec!['C', 'A', 'B']);
    assert_eq!(c.left_to_middle(), &[0, 1, 2]);
    assert_eq!(c.right_to_middle(), &[2, 0, 1]);
    assert_eq!(cospan_wiring(&c), vec![1, 2, 0]);

    // -- Cospan, `types` on the codomain -----------------------------------
    let c = Cospan::<char>::from_permutation_on_codomain(p.clone(), &types).unwrap();
    assert_eq!(c.domain(), vec!['B', 'C', 'A']);
    assert_eq!(c.codomain(), vec!['A', 'B', 'C']);
    assert_eq!(c.left_to_middle(), &[1, 2, 0]);
    assert_eq!(c.right_to_middle(), &[0, 1, 2]);
    assert_eq!(cospan_wiring(&c), vec![1, 2, 0]);

    // -- MatR: entries written out in full ---------------------------------
    let unit: Vec<()> = vec![(); 3];
    let m = MatR::<F64Rig>::from_permutation_on_domain(p.clone(), &unit).unwrap();
    let one = F64Rig::one();
    let zero = F64Rig::zero();
    assert_eq!(
        m.entries(),
        &[
            vec![zero, one, zero],
            vec![zero, zero, one],
            vec![one, zero, zero],
        ],
        "row i carries its 1 in column p(i)"
    );
    assert_eq!(mat_wiring(&m), vec![1, 2, 0]);

    // The inverse is a genuinely different matrix, so the assertions above
    // pin a direction rather than accepting either.
    let m_inv = MatR::<F64Rig>::from_permutation_on_domain(p.inv(), &unit).unwrap();
    assert_ne!(m.entries(), m_inv.entries());
}

// ---------------------------------------------------------------------------
// 2. Every carrier realizes `p`, on both constructors, exhaustively
// ---------------------------------------------------------------------------

/// The sweep #258 asks for: the same permutation through every implementation
/// of both traits, at `n = 3` and `n = 4` (`6 + 24 = 30` permutations).
///
/// Each carrier is checked against `reference_wiring(&p)` — never against
/// another carrier.
#[test]
fn every_carrier_realizes_p_on_both_constructors() {
    let mut checked = 0usize;
    for n in [3usize, 4] {
        let perms = all_perms(n);
        assert_eq!(
            perms.len(),
            (1..=n).product::<usize>(),
            "all_perms({n}) must yield n!"
        );
        let types = labels(n);
        let unit: Vec<()> = vec![(); n];

        for p in perms {
            let want = reference_wiring(&p);
            let cod_labels: Vec<char> = p.inv().permute(&types);
            let dom_labels: Vec<char> = p.permute(&types);

            // ---- SymmetricMonoidalMorphism, `types` on the domain --------
            let c = Cospan::<char>::from_permutation_on_domain(p.clone(), &types).unwrap();
            assert_eq!(cospan_wiring(&c), want, "Cospan on_domain n={n} p={p:?}");
            assert_eq!(c.domain(), types, "Cospan on_domain dom n={n} p={p:?}");
            assert_eq!(
                c.codomain(),
                cod_labels,
                "Cospan on_domain cod n={n} p={p:?}"
            );

            let s = Span::<char>::from_permutation_on_domain(p.clone(), &types).unwrap();
            assert_eq!(span_wiring(&s), want, "Span on_domain n={n} p={p:?}");
            assert_eq!(s.domain(), types, "Span on_domain dom n={n} p={p:?}");
            assert_eq!(s.codomain(), cod_labels, "Span on_domain cod n={n} p={p:?}");

            let r = Corel::<char>::from_permutation_on_domain(p.clone(), &types).unwrap();
            assert_eq!(
                cospan_wiring(r.as_cospan()),
                want,
                "Corel on_domain n={n} p={p:?}"
            );

            let f = FrobeniusMorphism::<char, ()>::from_permutation_on_domain(p.clone(), &types)
                .unwrap();
            assert_eq!(f.domain(), types, "Frobenius on_domain dom n={n} p={p:?}");
            assert_eq!(
                f.codomain(),
                cod_labels,
                "Frobenius on_domain cod n={n} p={p:?}"
            );

            let a = CospanAlgebraMorphism::<PartitionAlgebra, char>::from_permutation_on_domain(
                p.clone(),
                &types,
            )
            .unwrap();
            assert_eq!(
                cam_wiring(&a),
                want,
                "CospanAlgebra on_domain n={n} p={p:?}"
            );
            assert_eq!(
                a.domain(),
                types,
                "CospanAlgebra on_domain dom n={n} p={p:?}"
            );
            assert_eq!(
                a.codomain(),
                cod_labels,
                "CospanAlgebra on_domain cod n={n} p={p:?}"
            );

            let d = DecoratedCospan::<char, PetriDecoration<char>>::from_permutation_on_domain(
                p.clone(),
                &types,
            )
            .unwrap();
            assert_eq!(
                cospan_wiring(&d.cospan),
                want,
                "DecoratedCospan on_domain n={n} p={p:?}"
            );

            let net = PetriNet::<char>::from_permutation_on_domain(p.clone(), &types).unwrap();
            assert_eq!(petri_wiring(&net), want, "PetriNet on_domain n={n} p={p:?}");
            assert_eq!(net.places(), types, "PetriNet on_domain apex n={n} p={p:?}");
            assert!(
                net.transitions().is_empty(),
                "a pure braiding has no transitions; n={n} p={p:?}"
            );
            assert_eq!(net.domain(), types, "PetriNet on_domain dom n={n} p={p:?}");
            assert_eq!(
                net.codomain(),
                cod_labels,
                "PetriNet on_domain cod n={n} p={p:?}"
            );

            let m = MatR::<F64Rig>::from_permutation_on_domain(p.clone(), &unit).unwrap();
            assert_eq!(mat_wiring(&m), want, "MatR on_domain n={n} p={p:?}");

            let k = MatKron::<F64Rig>::from_permutation_on_domain(p.clone(), &unit).unwrap();
            assert_eq!(
                mat_wiring(k.inner()),
                want,
                "MatKron on_domain n={n} p={p:?}"
            );

            let e = Sfg::from_permutation_on_domain(p.clone(), &unit).unwrap();
            assert_eq!(prop_wiring(e), want, "PropExpr on_domain n={n} p={p:?}");

            // ---- SymmetricMonoidalMorphism, `types` on the codomain ------
            let c = Cospan::<char>::from_permutation_on_codomain(p.clone(), &types).unwrap();
            assert_eq!(cospan_wiring(&c), want, "Cospan on_codomain n={n} p={p:?}");
            assert_eq!(
                c.domain(),
                dom_labels,
                "Cospan on_codomain dom n={n} p={p:?}"
            );
            assert_eq!(c.codomain(), types, "Cospan on_codomain cod n={n} p={p:?}");

            let s = Span::<char>::from_permutation_on_codomain(p.clone(), &types).unwrap();
            assert_eq!(span_wiring(&s), want, "Span on_codomain n={n} p={p:?}");
            assert_eq!(s.domain(), dom_labels, "Span on_codomain dom n={n} p={p:?}");
            assert_eq!(s.codomain(), types, "Span on_codomain cod n={n} p={p:?}");

            let r = Corel::<char>::from_permutation_on_codomain(p.clone(), &types).unwrap();
            assert_eq!(
                cospan_wiring(r.as_cospan()),
                want,
                "Corel on_codomain n={n} p={p:?}"
            );

            let f = FrobeniusMorphism::<char, ()>::from_permutation_on_codomain(p.clone(), &types)
                .unwrap();
            assert_eq!(
                f.domain(),
                dom_labels,
                "Frobenius on_codomain dom n={n} p={p:?}"
            );
            assert_eq!(
                f.codomain(),
                types,
                "Frobenius on_codomain cod n={n} p={p:?}"
            );

            let a = CospanAlgebraMorphism::<PartitionAlgebra, char>::from_permutation_on_codomain(
                p.clone(),
                &types,
            )
            .unwrap();
            assert_eq!(
                cam_wiring(&a),
                want,
                "CospanAlgebra on_codomain n={n} p={p:?}"
            );
            assert_eq!(
                a.domain(),
                dom_labels,
                "CospanAlgebra on_codomain dom n={n} p={p:?}"
            );
            assert_eq!(
                a.codomain(),
                types,
                "CospanAlgebra on_codomain cod n={n} p={p:?}"
            );

            let d = DecoratedCospan::<char, PetriDecoration<char>>::from_permutation_on_codomain(
                p.clone(),
                &types,
            )
            .unwrap();
            assert_eq!(
                cospan_wiring(&d.cospan),
                want,
                "DecoratedCospan on_codomain n={n} p={p:?}"
            );

            let net = PetriNet::<char>::from_permutation_on_codomain(p.clone(), &types).unwrap();
            assert_eq!(
                petri_wiring(&net),
                want,
                "PetriNet on_codomain n={n} p={p:?}"
            );
            assert_eq!(
                net.places(),
                types,
                "PetriNet on_codomain apex n={n} p={p:?}"
            );
            assert!(
                net.transitions().is_empty(),
                "a pure braiding has no transitions; n={n} p={p:?}"
            );
            assert_eq!(
                net.domain(),
                dom_labels,
                "PetriNet on_codomain dom n={n} p={p:?}"
            );
            assert_eq!(
                net.codomain(),
                types,
                "PetriNet on_codomain cod n={n} p={p:?}"
            );

            let m = MatR::<F64Rig>::from_permutation_on_codomain(p.clone(), &unit).unwrap();
            assert_eq!(mat_wiring(&m), want, "MatR on_codomain n={n} p={p:?}");

            let k = MatKron::<F64Rig>::from_permutation_on_codomain(p.clone(), &unit).unwrap();
            assert_eq!(
                mat_wiring(k.inner()),
                want,
                "MatKron on_codomain n={n} p={p:?}"
            );

            let e = Sfg::from_permutation_on_codomain(p.clone(), &unit).unwrap();
            assert_eq!(prop_wiring(e), want, "PropExpr on_codomain n={n} p={p:?}");

            // ---- SymmetricMonoidalDiscreteMorphism ----------------------
            // `Decomposition`'s object is a bare cardinality, so its single
            // constructor is checked two ways: the permutation part it stores,
            // and the labelled Frobenius morphism it factors into — which is
            // the only place the discrete trait meets the labelled one.
            let decomp = Decomposition::from_permutation(p.clone(), n);
            assert_eq!(
                decomp.get_parts().0,
                &p,
                "Decomposition stores p, not p.inv(), n={n} p={p:?}"
            );
            let via_decomp: FrobeniusMorphism<char, ()> =
                from_decomposition(decomp, &types, &cod_labels).unwrap();
            assert_eq!(
                via_decomp.domain(),
                types,
                "from_decomposition dom n={n} p={p:?}"
            );
            assert_eq!(
                via_decomp.codomain(),
                cod_labels,
                "from_decomposition cod n={n} p={p:?}"
            );

            checked += 1;
        }
    }
    assert_eq!(checked, 6 + 24, "n=3 and n=4 sweeps must both have run");
}

// ---------------------------------------------------------------------------
// 3. The relabelling law that ties the two constructors together
// ---------------------------------------------------------------------------

/// `on_codomain(p, types) == on_domain(p, &p.permute(types))`.
///
/// This is a *consequence* of the contract, not a restatement of either body:
/// `p.permute(types)` is computed here, and the two sides come out of two
/// different functions. An impl that inverted exactly one of its two
/// constructors would satisfy every within-constructor assertion above and
/// fail here.
///
/// Equality is asserted on the *morphism* — domain, codomain, wiring — not on
/// the internal representation. `Cospan` deliberately keeps the identity on
/// whichever leg the caller's `types` sits behind, so the two builds agree as
/// cospans while numbering the apex differently.
#[test]
fn the_two_constructors_are_related_by_relabelling() {
    for n in [3usize, 4] {
        let types = labels(n);
        for p in all_perms(n) {
            let relabelled: Vec<char> = p.permute(&types);

            let a = Cospan::<char>::from_permutation_on_codomain(p.clone(), &types).unwrap();
            let b = Cospan::<char>::from_permutation_on_domain(p.clone(), &relabelled).unwrap();
            assert_eq!(a.domain(), b.domain(), "Cospan n={n} p={p:?}");
            assert_eq!(a.codomain(), b.codomain(), "Cospan n={n} p={p:?}");
            assert_eq!(cospan_wiring(&a), cospan_wiring(&b), "Cospan n={n} p={p:?}");

            let a = Span::<char>::from_permutation_on_codomain(p.clone(), &types).unwrap();
            let b = Span::<char>::from_permutation_on_domain(p.clone(), &relabelled).unwrap();
            assert_eq!(a.domain(), b.domain(), "Span n={n} p={p:?}");
            assert_eq!(a.codomain(), b.codomain(), "Span n={n} p={p:?}");
            assert_eq!(a.middle_pairs(), b.middle_pairs(), "Span n={n} p={p:?}");

            let a = FrobeniusMorphism::<char, ()>::from_permutation_on_codomain(p.clone(), &types)
                .unwrap();
            let b =
                FrobeniusMorphism::<char, ()>::from_permutation_on_domain(p.clone(), &relabelled)
                    .unwrap();
            assert_eq!(a.domain(), b.domain(), "Frobenius n={n} p={p:?}");
            assert_eq!(a.codomain(), b.codomain(), "Frobenius n={n} p={p:?}");

            let a = CospanAlgebraMorphism::<PartitionAlgebra, char>::from_permutation_on_codomain(
                p.clone(),
                &types,
            )
            .unwrap();
            let b = CospanAlgebraMorphism::<PartitionAlgebra, char>::from_permutation_on_domain(
                p.clone(),
                &relabelled,
            )
            .unwrap();
            assert_eq!(a.domain(), b.domain(), "CospanAlgebra n={n} p={p:?}");
            assert_eq!(a.codomain(), b.codomain(), "CospanAlgebra n={n} p={p:?}");
            assert_eq!(
                cam_wiring(&a),
                cam_wiring(&b),
                "CospanAlgebra n={n} p={p:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 4. The identity permutation must give the identity morphism
// ---------------------------------------------------------------------------

/// No symmetric monoidal category permits `braiding(id) != id`, so this holds
/// on every carrier regardless of which convention it picked — which makes it
/// the sharpest available check on `CospanAlgebraMorphism`, whose pre-#258 body
/// failed it.
#[test]
fn identity_permutation_gives_the_identity_morphism() {
    for n in [0usize, 1, 3, 4] {
        let types = labels(n);
        let id = Permutation::identity(n);

        let c = Cospan::<char>::from_permutation_on_domain(id.clone(), &types).unwrap();
        let c_id = Cospan::<char>::identity(&types);
        assert_eq!(c.left_to_middle(), c_id.left_to_middle(), "Cospan n={n}");
        assert_eq!(c.right_to_middle(), c_id.right_to_middle(), "Cospan n={n}");
        assert_eq!(c.middle(), c_id.middle(), "Cospan n={n}");

        let s = Span::<char>::from_permutation_on_domain(id.clone(), &types).unwrap();
        let s_id = Span::<char>::identity(&types);
        assert_eq!(s.middle_pairs(), s_id.middle_pairs(), "Span n={n}");
        assert_eq!(s.domain(), s_id.domain(), "Span n={n}");
        assert_eq!(s.codomain(), s_id.codomain(), "Span n={n}");

        // The one that used to fail: the element was built over a `2n`-vertex
        // apex with a bijective leg, so it was the all-singletons partition
        // rather than the `n`-vertex cup that pairs domain `i` with codomain `i`.
        let a = CospanAlgebraMorphism::<PartitionAlgebra, char>::from_permutation_on_domain(
            id.clone(),
            &types,
        )
        .unwrap();
        let a_id =
            <CospanAlgebraMorphism<PartitionAlgebra, char> as HasIdentity<Vec<char>>>::identity(
                &types,
            );
        assert_eq!(a.domain(), a_id.domain(), "CospanAlgebra n={n}");
        assert_eq!(a.codomain(), a_id.codomain(), "CospanAlgebra n={n}");
        assert_eq!(
            a.element().middle(),
            a_id.element().middle(),
            "CospanAlgebra apex must be n vertices, not 2n; n={n}"
        );
        assert_eq!(
            a.element().right_to_middle(),
            a_id.element().right_to_middle(),
            "CospanAlgebra partition must pair domain i with codomain i; n={n}"
        );

        let m = MatR::<F64Rig>::from_permutation_on_domain(id.clone(), &vec![(); n]).unwrap();
        assert_eq!(
            m.entries(),
            MatR::<F64Rig>::identity(n).entries(),
            "MatR n={n}"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. Arity mismatch is an error on every carrier, not a panic
// ---------------------------------------------------------------------------

/// The trait's `# Errors` clause promises this; before #258 `Cospan` (and with
/// it `Corel`, `DecoratedCospan`, `PetriNet` and `NamedCospan`) reached it
/// through an `assert_eq!` and panicked instead, while `Span` and
/// `FrobeniusMorphism` had no check at all and indexed out of bounds.
///
/// ⚠ This checked `on_domain` only on six of the nine carriers, so a missing
/// length check in any `on_codomain` body would have passed. Every carrier is
/// now driven through **both** constructors.
#[test]
fn arity_mismatch_is_an_error_on_every_carrier() {
    let p = Permutation::rotation_left(3, 1);
    let types = labels(4); // deliberately one longer than p
    let unit: Vec<()> = vec![(); 4];

    assert!(Cospan::<char>::from_permutation_on_domain(p.clone(), &types).is_err());
    assert!(Cospan::<char>::from_permutation_on_codomain(p.clone(), &types).is_err());
    assert!(Span::<char>::from_permutation_on_domain(p.clone(), &types).is_err());
    assert!(Span::<char>::from_permutation_on_codomain(p.clone(), &types).is_err());
    assert!(Corel::<char>::from_permutation_on_domain(p.clone(), &types).is_err());
    assert!(Corel::<char>::from_permutation_on_codomain(p.clone(), &types).is_err());
    assert!(FrobeniusMorphism::<char, ()>::from_permutation_on_domain(p.clone(), &types).is_err());
    assert!(
        FrobeniusMorphism::<char, ()>::from_permutation_on_codomain(p.clone(), &types).is_err()
    );
    assert!(
        CospanAlgebraMorphism::<PartitionAlgebra, char>::from_permutation_on_domain(
            p.clone(),
            &types
        )
        .is_err()
    );
    assert!(
        CospanAlgebraMorphism::<PartitionAlgebra, char>::from_permutation_on_codomain(
            p.clone(),
            &types
        )
        .is_err()
    );
    assert!(
        DecoratedCospan::<char, PetriDecoration<char>>::from_permutation_on_domain(
            p.clone(),
            &types
        )
        .is_err()
    );
    assert!(
        DecoratedCospan::<char, PetriDecoration<char>>::from_permutation_on_codomain(
            p.clone(),
            &types
        )
        .is_err()
    );
    assert!(PetriNet::<char>::from_permutation_on_domain(p.clone(), &types).is_err());
    assert!(PetriNet::<char>::from_permutation_on_codomain(p.clone(), &types).is_err());
    assert!(MatR::<F64Rig>::from_permutation_on_domain(p.clone(), &unit).is_err());
    assert!(MatR::<F64Rig>::from_permutation_on_codomain(p.clone(), &unit).is_err());
    assert!(MatKron::<F64Rig>::from_permutation_on_domain(p.clone(), &unit).is_err());
    assert!(MatKron::<F64Rig>::from_permutation_on_codomain(p.clone(), &unit).is_err());
    assert!(Sfg::from_permutation_on_domain(p.clone(), &unit).is_err());
    assert!(Sfg::from_permutation_on_codomain(p, &unit).is_err());
}

// ---------------------------------------------------------------------------
// 6. NamedCospan stays honest
// ---------------------------------------------------------------------------

/// `NamedCospan` cannot satisfy the contract — port names are not derivable
/// from `types` — so both constructors fail loudly and name the replacement.
#[test]
fn named_cospan_refuses_both_constructors_and_points_at_the_replacement() {
    use catgraph::named_cospan::NamedCospan;

    let p = Permutation::rotation_left(3, 1);
    let types = labels(3);

    let refusals = [
        (
            <NamedCospan<char, u8, u8> as SymmetricMonoidalMorphism<char>>::from_permutation_on_domain(
                p.clone(),
                &types,
            )
            .err(),
            "from_permutation_extra_data_on_domain",
        ),
        (
            <NamedCospan<char, u8, u8> as SymmetricMonoidalMorphism<char>>::from_permutation_on_codomain(
                p.clone(),
                &types,
            )
            .err(),
            "from_permutation_extra_data_on_codomain",
        ),
    ];
    for (err, expected) in refusals {
        let err = err.expect("NamedCospan cannot build a braiding without port names");
        let rendered = format!("{err}");
        assert!(
            rendered.contains(expected),
            "the refusal must name {expected}, got: {rendered}"
        );
    }

    // And the replacement does honour the contract, on the same fixture.
    let named = NamedCospan::<char, char, char>::from_permutation_extra_data_on_domain(
        p.clone(),
        &types,
        &types,
        |z| (z, z),
    )
    .unwrap();
    assert_eq!(named.domain(), types);
    assert_eq!(named.codomain(), p.inv().permute(&types));

    let named = NamedCospan::<char, char, char>::from_permutation_extra_data_on_codomain(
        p.clone(),
        &types,
        &types,
        |z| (z, z),
    )
    .unwrap();
    assert_eq!(named.domain(), p.permute(&types));
    assert_eq!(named.codomain(), types);

    // Its length checks are errors too, not the pre-#258 `assert_eq!`/`unwrap`.
    assert!(
        NamedCospan::<char, char, char>::from_permutation_extra_data_on_domain(
            p.clone(),
            &labels(4),
            &labels(4),
            |z| (z, z)
        )
        .is_err(),
        "p shorter than types"
    );
    assert!(
        NamedCospan::<char, char, char>::from_permutation_extra_data_on_domain(
            p,
            &types,
            &labels(4),
            |z| (z, z)
        )
        .is_err(),
        "prenames longer than types"
    );
}

// ---------------------------------------------------------------------------
// 7. `permute_side` — the hand anchor
// ---------------------------------------------------------------------------

/// `permute_side` had no cross-carrier coverage at all, which is why it kept
/// the inverted convention the constructors shed at #258: `Cospan`,
/// `FrobeniusMorphism` and `Span` spliced `β(p⁻¹)` where `MatR`, `MatKron` and
/// `PropExpr` spliced `β(p)`.
///
/// Contract under test (trait rustdoc): **the wire at slot `i` of the permuted
/// side moves to slot `p.apply(i)`**, i.e.
///
/// - `of_codomain = true`  → `self ; from_permutation_on_domain(p, cod)`
/// - `of_codomain = false` → `from_permutation_on_codomain(p.inv(), dom) ; self`
///
/// Every number below is written out rather than derived, for
/// `p = rotation_left(3, 1)` over `['A','B','C']`, on both sides and on both
/// sides of the carrier divide.
#[test]
fn hand_anchored_permute_side_values() {
    let p = Permutation::rotation_left(3, 1);
    let types = labels(3);

    // -- Cospan, codomain side: the right leg becomes old ∘ p.inv() ---------
    let mut c = Cospan::<char>::identity(&types);
    c.permute_side(&p, true);
    assert_eq!(c.left_to_middle(), &[0, 1, 2], "domain leg untouched");
    assert_eq!(c.right_to_middle(), &[2, 0, 1]);
    assert_eq!(c.domain(), vec!['A', 'B', 'C']);
    assert_eq!(c.codomain(), vec!['C', 'A', 'B']);
    assert_eq!(cospan_wiring(&c), vec![1, 2, 0], "the braiding is β(p)");

    // -- Cospan, domain side: β(p.inv()) is spliced, so the wiring inverts --
    let mut c = Cospan::<char>::identity(&types);
    c.permute_side(&p, false);
    assert_eq!(c.left_to_middle(), &[2, 0, 1]);
    assert_eq!(c.right_to_middle(), &[0, 1, 2], "codomain leg untouched");
    assert_eq!(c.domain(), vec!['C', 'A', 'B']);
    assert_eq!(c.codomain(), vec!['A', 'B', 'C']);
    assert_eq!(
        cospan_wiring(&c),
        vec![2, 0, 1],
        "precomposition splices β(p.inv())"
    );

    // -- Span: pairs move by p, words by p.inv() — the two are inverse ------
    let mut s = Span::<char>::identity(&types);
    s.permute_side(&p, true);
    assert_eq!(s.middle_pairs(), &[(0, 1), (1, 2), (2, 0)]);
    assert_eq!(s.domain(), vec!['A', 'B', 'C']);
    assert_eq!(s.codomain(), vec!['C', 'A', 'B']);
    assert_eq!(span_wiring(&s), vec![1, 2, 0]);

    let mut s = Span::<char>::identity(&types);
    s.permute_side(&p, false);
    assert_eq!(s.middle_pairs(), &[(1, 0), (2, 1), (0, 2)]);
    assert_eq!(s.domain(), vec!['C', 'A', 'B']);
    assert_eq!(s.codomain(), vec!['A', 'B', 'C']);
    assert_eq!(span_wiring(&s), vec![2, 0, 1]);

    // -- MatR: entries written out in full ---------------------------------
    let one = F64Rig::one();
    let zero = F64Rig::zero();
    let mut m = MatR::<F64Rig>::identity(3);
    m.permute_side(&p, true);
    assert_eq!(
        m.entries(),
        &[
            vec![zero, one, zero],
            vec![zero, zero, one],
            vec![one, zero, zero],
        ],
        "right-multiplying by P: row i carries its 1 in column p(i)"
    );
    assert_eq!(mat_wiring(&m), vec![1, 2, 0]);

    let mut m = MatR::<F64Rig>::identity(3);
    m.permute_side(&p, false);
    assert_eq!(
        m.entries(),
        &[
            vec![zero, zero, one],
            vec![one, zero, zero],
            vec![zero, one, zero],
        ],
        "left-multiplying by Pᵀ = P⁻¹"
    );
    assert_eq!(mat_wiring(&m), vec![2, 0, 1]);

    // -- Decomposition (the discrete trait): p on the codomain, p.inv() on --
    // -- the domain, read straight off the stored permutation part ---------
    let mut d = Decomposition::identity(&3);
    d.permute_side(&p, true);
    assert_eq!(d.get_parts().0, &p, "codomain side stores p");

    let mut d = Decomposition::identity(&3);
    d.permute_side(&p, false);
    assert_eq!(d.get_parts().0, &p.inv(), "domain side stores p.inv()");
}

// ---------------------------------------------------------------------------
// 8. `permute_side` on an identity is the constructor, exhaustively
// ---------------------------------------------------------------------------

/// The tie between the two methods, at `n = 3` and `n = 4`:
///
/// - `identity(types).permute_side(p, true)` == `from_permutation_on_domain(p, types)`
/// - `identity(types).permute_side(p, false)` == `from_permutation_on_codomain(p.inv(), types)`
///
/// Both sides of each comparison are checked against `reference_wiring`, which
/// is computed from `p` alone and anchored by hand in
/// `hand_anchored_reference_values` — never carrier against carrier.
#[test]
fn permute_side_on_an_identity_matches_the_constructors() {
    use catgraph::named_cospan::NamedCospan;

    let mut checked = 0usize;
    for n in [3usize, 4] {
        let types = labels(n);
        for p in all_perms(n) {
            let p_inv = p.inv();
            let want_cod = reference_wiring(&p);
            let want_dom = reference_wiring(&p_inv);
            let permuted: Vec<char> = p_inv.permute(&types);

            // ---- Cospan ------------------------------------------------
            let mut c = Cospan::<char>::identity(&types);
            c.permute_side(&p, true);
            assert_eq!(cospan_wiring(&c), want_cod, "Cospan cod n={n} p={p:?}");
            assert_eq!(c.domain(), types, "Cospan cod dom n={n} p={p:?}");
            assert_eq!(c.codomain(), permuted, "Cospan cod cod n={n} p={p:?}");

            let mut c = Cospan::<char>::identity(&types);
            c.permute_side(&p, false);
            assert_eq!(cospan_wiring(&c), want_dom, "Cospan dom n={n} p={p:?}");
            assert_eq!(c.domain(), permuted, "Cospan dom dom n={n} p={p:?}");
            assert_eq!(c.codomain(), types, "Cospan dom cod n={n} p={p:?}");

            // ---- Span --------------------------------------------------
            let mut s = Span::<char>::identity(&types);
            s.permute_side(&p, true);
            s.assert_valid(false, false);
            assert_eq!(span_wiring(&s), want_cod, "Span cod n={n} p={p:?}");
            assert_eq!(s.domain(), types, "Span cod dom n={n} p={p:?}");
            assert_eq!(s.codomain(), permuted, "Span cod cod n={n} p={p:?}");

            let mut s = Span::<char>::identity(&types);
            s.permute_side(&p, false);
            s.assert_valid(false, false);
            assert_eq!(span_wiring(&s), want_dom, "Span dom n={n} p={p:?}");
            assert_eq!(s.domain(), permuted, "Span dom dom n={n} p={p:?}");
            assert_eq!(s.codomain(), types, "Span dom cod n={n} p={p:?}");

            // ---- Corel -------------------------------------------------
            let mut r = Corel::<char>::identity(&types);
            r.permute_side(&p, true);
            assert_eq!(
                cospan_wiring(r.as_cospan()),
                want_cod,
                "Corel cod n={n} p={p:?}"
            );
            let mut r = Corel::<char>::identity(&types);
            r.permute_side(&p, false);
            assert_eq!(
                cospan_wiring(r.as_cospan()),
                want_dom,
                "Corel dom n={n} p={p:?}"
            );

            // ---- FrobeniusMorphism (no wiring extractor; word + equality) --
            let mut f: FrobeniusMorphism<char, ()> = FrobeniusMorphism::identity(&types);
            f.permute_side(&p, true);
            assert_eq!(f.domain(), types, "Frobenius cod dom n={n} p={p:?}");
            assert_eq!(f.codomain(), permuted, "Frobenius cod cod n={n} p={p:?}");

            let mut f: FrobeniusMorphism<char, ()> = FrobeniusMorphism::identity(&types);
            f.permute_side(&p, false);
            assert_eq!(f.domain(), permuted, "Frobenius dom dom n={n} p={p:?}");
            assert_eq!(f.codomain(), types, "Frobenius dom cod n={n} p={p:?}");

            // ---- CospanAlgebraMorphism ---------------------------------
            // The element is the morphism here, so this is the assertion that
            // catches a stale `element` behind a permuted word.
            let mut a =
                <CospanAlgebraMorphism<PartitionAlgebra, char> as HasIdentity<Vec<char>>>::identity(
                    &types,
                );
            a.permute_side(&p, true);
            assert_eq!(cam_wiring(&a), want_cod, "CospanAlgebra cod n={n} p={p:?}");
            assert_eq!(a.domain(), types, "CospanAlgebra cod dom n={n} p={p:?}");
            assert_eq!(
                a.codomain(),
                permuted,
                "CospanAlgebra cod cod n={n} p={p:?}"
            );

            let mut a =
                <CospanAlgebraMorphism<PartitionAlgebra, char> as HasIdentity<Vec<char>>>::identity(
                    &types,
                );
            a.permute_side(&p, false);
            assert_eq!(cam_wiring(&a), want_dom, "CospanAlgebra dom n={n} p={p:?}");
            assert_eq!(a.domain(), permuted, "CospanAlgebra dom dom n={n} p={p:?}");
            assert_eq!(a.codomain(), types, "CospanAlgebra dom cod n={n} p={p:?}");

            // ---- DecoratedCospan ---------------------------------------
            let mut d: DecoratedCospan<char, PetriDecoration<char>> =
                DecoratedCospan::identity(&types);
            d.permute_side(&p, true);
            assert_eq!(
                cospan_wiring(&d.cospan),
                want_cod,
                "DecoratedCospan cod n={n} p={p:?}"
            );
            let mut d: DecoratedCospan<char, PetriDecoration<char>> =
                DecoratedCospan::identity(&types);
            d.permute_side(&p, false);
            assert_eq!(
                cospan_wiring(&d.cospan),
                want_dom,
                "DecoratedCospan dom n={n} p={p:?}"
            );

            // ---- PetriNet ----------------------------------------------
            // The identity net carries one relay transition, which the
            // braiding must leave alone while it moves the named leg.
            let base: PetriNet<char> = PetriNet::identity(&types);
            let relay = base.transitions().to_vec();

            let mut net = base.clone();
            net.permute_side(&p, true);
            assert_eq!(petri_wiring(&net), want_cod, "PetriNet cod n={n} p={p:?}");
            assert_eq!(net.domain(), types, "PetriNet cod dom n={n} p={p:?}");
            assert_eq!(net.codomain(), permuted, "PetriNet cod cod n={n} p={p:?}");
            assert_eq!(net.transitions(), relay, "PetriNet cod transitions n={n}");

            let mut net = base.clone();
            net.permute_side(&p, false);
            assert_eq!(petri_wiring(&net), want_dom, "PetriNet dom n={n} p={p:?}");
            assert_eq!(net.domain(), permuted, "PetriNet dom dom n={n} p={p:?}");
            assert_eq!(net.codomain(), types, "PetriNet dom cod n={n} p={p:?}");
            assert_eq!(net.transitions(), relay, "PetriNet dom transitions n={n}");

            // ---- NamedCospan: the port names must travel with the wires --
            let mut nc = NamedCospan::<char, char, char>::identity(&types, &types, |z| (z, z));
            nc.permute_side(&p, true);
            assert_eq!(nc.domain(), types, "NamedCospan cod dom n={n} p={p:?}");
            assert_eq!(nc.codomain(), permuted, "NamedCospan cod cod n={n} p={p:?}");
            assert_eq!(
                nc.right_names(),
                &permuted,
                "NamedCospan right names must move with the right leg"
            );
            assert_eq!(nc.left_names(), &types, "NamedCospan left names untouched");

            let mut nc = NamedCospan::<char, char, char>::identity(&types, &types, |z| (z, z));
            nc.permute_side(&p, false);
            assert_eq!(nc.domain(), permuted, "NamedCospan dom dom n={n} p={p:?}");
            assert_eq!(nc.codomain(), types, "NamedCospan dom cod n={n} p={p:?}");
            assert_eq!(
                nc.left_names(),
                &permuted,
                "NamedCospan left names must move with the left leg"
            );
            assert_eq!(
                nc.right_names(),
                &types,
                "NamedCospan right names untouched"
            );

            // ---- MatR / MatKron / PropExpr -----------------------------
            let mut m = MatR::<F64Rig>::identity(n);
            m.permute_side(&p, true);
            assert_eq!(mat_wiring(&m), want_cod, "MatR cod n={n} p={p:?}");
            let mut m = MatR::<F64Rig>::identity(n);
            m.permute_side(&p, false);
            assert_eq!(mat_wiring(&m), want_dom, "MatR dom n={n} p={p:?}");

            let mut k = MatKron::<F64Rig>::identity(n);
            k.permute_side(&p, true);
            assert_eq!(mat_wiring(k.inner()), want_cod, "MatKron cod n={n} p={p:?}");
            let mut k = MatKron::<F64Rig>::identity(n);
            k.permute_side(&p, false);
            assert_eq!(mat_wiring(k.inner()), want_dom, "MatKron dom n={n} p={p:?}");

            let mut e: Sfg = PropExpr::Identity(n);
            e.permute_side(&p, true);
            assert_eq!(prop_wiring(e), want_cod, "PropExpr cod n={n} p={p:?}");
            let mut e: Sfg = PropExpr::Identity(n);
            e.permute_side(&p, false);
            assert_eq!(prop_wiring(e), want_dom, "PropExpr dom n={n} p={p:?}");

            // ---- Decomposition (the discrete trait) --------------------
            let mut dec = Decomposition::identity(&n);
            dec.permute_side(&p, true);
            assert_eq!(dec.get_parts().0, &p, "Decomposition cod n={n} p={p:?}");
            let mut dec = Decomposition::identity(&n);
            dec.permute_side(&p, false);
            assert_eq!(dec.get_parts().0, &p_inv, "Decomposition dom n={n} p={p:?}");

            checked += 1;
        }
    }
    assert_eq!(checked, 6 + 24, "n=3 and n=4 sweeps must both have run");
}

// ---------------------------------------------------------------------------
// 9. `permute_side` on a *non*-identity morphism
// ---------------------------------------------------------------------------

/// The identity sweep above cannot separate "splices the right braiding" from
/// "rebuilds a braiding from scratch", because on an identity the two coincide.
/// This one starts from `β(q)` for a non-identity `q` and checks the composite
/// law, with the expected permutation computed from `q` and `p` directly:
///
/// - `β(q).permute_side(p, true)  == β(q ; p)` — `(q * p).apply(i) == p(q(i))`
/// - `β(q).permute_side(p, false) == β(p⁻¹ ; q)`
///
/// Every ordered pair `(q, p)` at `n = 3` and `n = 4`.
#[test]
fn permute_side_composes_the_braidings() {
    let mut checked = 0usize;
    for n in [3usize, 4] {
        let types = labels(n);
        let perms = all_perms(n);
        for q in &perms {
            for p in &perms {
                let want_cod = reference_wiring(&(q * p));
                let want_dom = reference_wiring(&(&p.inv() * q));

                let mut c = Cospan::<char>::from_permutation_on_domain(q.clone(), &types).unwrap();
                c.permute_side(p, true);
                assert_eq!(
                    cospan_wiring(&c),
                    want_cod,
                    "Cospan cod n={n} q={q:?} p={p:?}"
                );
                let mut c =
                    Cospan::<char>::from_permutation_on_codomain(q.clone(), &types).unwrap();
                c.permute_side(p, false);
                assert_eq!(
                    cospan_wiring(&c),
                    want_dom,
                    "Cospan dom n={n} q={q:?} p={p:?}"
                );

                let mut s = Span::<char>::from_permutation_on_domain(q.clone(), &types).unwrap();
                s.permute_side(p, true);
                s.assert_valid(false, false);
                assert_eq!(span_wiring(&s), want_cod, "Span cod n={n} q={q:?} p={p:?}");
                let mut s = Span::<char>::from_permutation_on_codomain(q.clone(), &types).unwrap();
                s.permute_side(p, false);
                s.assert_valid(false, false);
                assert_eq!(span_wiring(&s), want_dom, "Span dom n={n} q={q:?} p={p:?}");

                let mut a =
                    CospanAlgebraMorphism::<PartitionAlgebra, char>::from_permutation_on_domain(
                        q.clone(),
                        &types,
                    )
                    .unwrap();
                a.permute_side(p, true);
                assert_eq!(
                    cam_wiring(&a),
                    want_cod,
                    "CospanAlgebra cod n={n} q={q:?} p={p:?}"
                );
                let mut a =
                    CospanAlgebraMorphism::<PartitionAlgebra, char>::from_permutation_on_codomain(
                        q.clone(),
                        &types,
                    )
                    .unwrap();
                a.permute_side(p, false);
                assert_eq!(
                    cam_wiring(&a),
                    want_dom,
                    "CospanAlgebra dom n={n} q={q:?} p={p:?}"
                );

                let unit: Vec<()> = vec![(); n];
                let mut m = MatR::<F64Rig>::from_permutation_on_domain(q.clone(), &unit).unwrap();
                m.permute_side(p, true);
                assert_eq!(mat_wiring(&m), want_cod, "MatR cod n={n} q={q:?} p={p:?}");
                let mut m = MatR::<F64Rig>::from_permutation_on_domain(q.clone(), &unit).unwrap();
                m.permute_side(p, false);
                assert_eq!(mat_wiring(&m), want_dom, "MatR dom n={n} q={q:?} p={p:?}");

                let mut e = Sfg::from_permutation_on_domain(q.clone(), &unit).unwrap();
                e.permute_side(p, true);
                assert_eq!(
                    prop_wiring(e),
                    want_cod,
                    "PropExpr cod n={n} q={q:?} p={p:?}"
                );
                let mut e = Sfg::from_permutation_on_domain(q.clone(), &unit).unwrap();
                e.permute_side(p, false);
                assert_eq!(
                    prop_wiring(e),
                    want_dom,
                    "PropExpr dom n={n} q={q:?} p={p:?}"
                );

                checked += 1;
            }
        }
    }
    assert_eq!(checked, 6 * 6 + 24 * 24, "both pair sweeps must have run");
}

// ---------------------------------------------------------------------------
// 10. Conjugation — the check that separates the domain rule from the codomain
// ---------------------------------------------------------------------------

/// `permute_side(p, false)` splices `β(p⁻¹)`, **not** `β(p)`. The consequence
/// is that permuting *both* sides of an identity by the same `p` gives an
/// identity back — `β(p⁻¹) ; id ; β(p) == β(p⁻¹ ; p) == id` — on the relabelled
/// word.
///
/// This is the assertion the domain-side derivation lives or dies on. Under the
/// symmetric reading (`β(p)` on both) the result is `β(p²)`, whose wiring is
/// `p²` and whose domain and codomain words disagree for any `p` of order > 2.
#[test]
fn permute_side_conjugation_returns_the_identity() {
    for n in [3usize, 4] {
        let types = labels(n);
        let identity_wiring: Vec<usize> = (0..n).collect();
        for p in all_perms(n) {
            let permuted: Vec<char> = p.inv().permute(&types);

            let mut c = Cospan::<char>::identity(&types);
            c.permute_side(&p, true);
            c.permute_side(&p, false);
            assert_eq!(cospan_wiring(&c), identity_wiring, "Cospan n={n} p={p:?}");
            assert_eq!(c.domain(), permuted, "Cospan n={n} p={p:?}");
            assert_eq!(c.codomain(), permuted, "Cospan n={n} p={p:?}");

            let mut s = Span::<char>::identity(&types);
            s.permute_side(&p, true);
            s.permute_side(&p, false);
            assert_eq!(span_wiring(&s), identity_wiring, "Span n={n} p={p:?}");
            assert_eq!(s.domain(), permuted, "Span n={n} p={p:?}");
            assert_eq!(s.codomain(), permuted, "Span n={n} p={p:?}");

            let mut a =
                <CospanAlgebraMorphism<PartitionAlgebra, char> as HasIdentity<Vec<char>>>::identity(
                    &types,
                );
            a.permute_side(&p, true);
            a.permute_side(&p, false);
            assert_eq!(
                cam_wiring(&a),
                identity_wiring,
                "CospanAlgebra n={n} p={p:?}"
            );
            assert_eq!(a.domain(), permuted, "CospanAlgebra n={n} p={p:?}");
            assert_eq!(a.codomain(), permuted, "CospanAlgebra n={n} p={p:?}");

            let mut m = MatR::<F64Rig>::identity(n);
            m.permute_side(&p, true);
            m.permute_side(&p, false);
            assert_eq!(
                m.entries(),
                MatR::<F64Rig>::identity(n).entries(),
                "MatR n={n} p={p:?}"
            );

            let mut e: Sfg = PropExpr::Identity(n);
            e.permute_side(&p, true);
            e.permute_side(&p, false);
            assert_eq!(prop_wiring(e), identity_wiring, "PropExpr n={n} p={p:?}");

            let mut f: FrobeniusMorphism<char, ()> = FrobeniusMorphism::identity(&types);
            f.permute_side(&p, true);
            f.permute_side(&p, false);
            assert_eq!(f.domain(), permuted, "Frobenius n={n} p={p:?}");
            assert_eq!(f.codomain(), permuted, "Frobenius n={n} p={p:?}");
            assert!(
                f == FrobeniusMorphism::<char, ()>::identity(&permuted),
                "Frobenius conjugation must simplify to the identity; n={n} p={p:?}"
            );

            let mut dec = Decomposition::identity(&n);
            dec.permute_side(&p, true);
            dec.permute_side(&p, false);
            assert_eq!(
                dec.get_parts().0,
                &Permutation::identity(n),
                "Decomposition n={n} p={p:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 11. `permute_side` composes with the morphism it permutes
// ---------------------------------------------------------------------------

/// The stale-`element` regression in `CospanAlgebraMorphism`, stated as the
/// property it actually broke: after `permute_side(p, true)` the morphism must
/// still compose correctly, and the composite is what a *hand-built*
/// composition with the same braiding gives.
///
/// This reaches past the interface words — the old body permuted those and
/// nothing else, so every word-level assertion above it stayed green.
#[test]
fn permute_side_agrees_with_explicit_composition() {
    for n in [3usize, 4] {
        let types = labels(n);
        for p in all_perms(n) {
            let braid =
                CospanAlgebraMorphism::<PartitionAlgebra, char>::from_permutation_on_domain(
                    p.clone(),
                    &types,
                )
                .unwrap();

            let mut permuted = <CospanAlgebraMorphism<PartitionAlgebra, char> as HasIdentity<
                Vec<char>,
            >>::identity(&types);
            permuted.permute_side(&p, true);

            let composed = <CospanAlgebraMorphism<PartitionAlgebra, char> as HasIdentity<
                Vec<char>,
            >>::identity(&types)
            .compose(&braid)
            .expect("the braiding is built on the identity's codomain");

            assert_eq!(permuted.domain(), composed.domain(), "n={n} p={p:?}");
            assert_eq!(permuted.codomain(), composed.codomain(), "n={n} p={p:?}");
            assert_eq!(
                cam_wiring(&permuted),
                cam_wiring(&composed),
                "permute_side(p, true) must equal `self ; β(p)`; n={n} p={p:?}"
            );

            // Composing the permuted morphism with `β(p.inv())` must return an
            // identity — impossible if `element` were stale, because the stale
            // element pairs domain `i` with the *old* codomain slot.
            let back = CospanAlgebraMorphism::<PartitionAlgebra, char>::from_permutation_on_domain(
                p.inv(),
                &permuted.codomain(),
            )
            .unwrap();
            let round = permuted
                .compose(&back)
                .expect("β(p.inv()) is built on the permuted codomain");
            assert_eq!(
                cam_wiring(&round),
                (0..n).collect::<Vec<_>>(),
                "self ; β(p) ; β(p.inv()) must be the identity; n={n} p={p:?}"
            );
            assert_eq!(round.domain(), types, "n={n} p={p:?}");
            assert_eq!(round.codomain(), types, "n={n} p={p:?}");
        }
    }
}
