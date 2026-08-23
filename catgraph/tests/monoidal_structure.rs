//! Integration tests for monoidal category properties using only the public API.
//!
//! Covers tensor associativity, tensor unit, permutation cospan composition,
//! symmetric braiding involutivity, span tensor product, and `permute_side`.

mod common;
use common::{assert_cospan_eq_msg as assert_cospan_eq, assert_cospan_shape, cospan_wiring};

use catgraph::{
    category::{Composable, HasIdentity},
    cospan::Cospan,
    monoidal::{Monoidal, SymmetricMonoidalMorphism},
    span::Span,
};
use catgraph_testutil::all_perms;
use permutations::Permutation;

/// Build a small non-trivial cospan: domain `[a,b]`, codomain `[b,c]`,
/// middle `[a,b,c]` with left=`[0,1]`, right=`[1,2]`.
fn sample_cospan_abc() -> Cospan<char> {
    Cospan::new(vec![0, 1], vec![1, 2], vec!['a', 'b', 'c']).unwrap()
}

/// Build a second non-trivial cospan: domain `[x]`, codomain `[x,y]`,
/// middle `[x,y]` with left=`[0]`, right=`[0,1]`.
fn sample_cospan_xy() -> Cospan<char> {
    Cospan::new(vec![0], vec![0, 1], vec!['x', 'y']).unwrap()
}

/// Build a third small cospan: domain `[p,q]`, codomain `[p]`,
/// middle `[p,q]` with left=`[0,1]`, right=`[0]`.
fn sample_cospan_pq() -> Cospan<char> {
    Cospan::new(vec![0, 1], vec![0], vec!['p', 'q']).unwrap()
}

// ---------------------------------------------------------------------------
// 1. Tensor associativity: (f ⊗ g) ⊗ h  vs  f ⊗ (g ⊗ h)
// ---------------------------------------------------------------------------

#[test]
fn tensor_associativity_cospan() {
    let f = sample_cospan_abc();
    let g = sample_cospan_xy();
    let h = sample_cospan_pq();

    // (f ⊗ g) ⊗ h
    let mut fg = f.clone();
    fg.monoidal(g.clone());
    let mut fg_h = fg.clone();
    fg_h.monoidal(h.clone());

    // f ⊗ (g ⊗ h)
    let mut gh = g.clone();
    gh.monoidal(h.clone());
    let mut f_gh = f.clone();
    f_gh.monoidal(gh);

    // Monoidal product on cospans concatenates boundaries and shifts middle
    // indices, so associativity holds on the nose (not just up to iso).
    assert_cospan_shape(&fg_h, &f_gh, "tensor associativity");

    // Stronger: exact structural equality.
    assert_cospan_eq(&fg_h, &f_gh, "tensor associativity (exact)");
}

// ---------------------------------------------------------------------------
// 2. Tensor with empty (monoidal unit)
// ---------------------------------------------------------------------------

#[test]
fn tensor_unit_cospan() {
    let f = sample_cospan_abc();
    let unit = Cospan::<char>::empty();

    // f ⊗ empty == f
    let mut f_unit = f.clone();
    f_unit.monoidal(unit.clone());
    assert_cospan_eq(&f_unit, &f, "f tensor empty");

    // empty ⊗ f == f
    let mut unit_f = unit;
    unit_f.monoidal(f.clone());
    assert_cospan_eq(&unit_f, &f, "empty tensor f");
}

/// The tensor of two identities is an identity on both legs, and a tensor with
/// a non-identity operand is not — on the leg that operand fails.
///
/// Until [#289](https://github.com/sustia-llc/catgraph/issues/289) `Cospan`
/// cached the answer, and `Monoidal::monoidal` maintained the cache with
/// `is_left_id &= other.is_left_id` and its `is_right_id` mirror — one
/// expression per leg. Deleting the cache deleted both, and nothing else in the
/// suite asserted what they encoded: the only other `monoidal` identity check,
/// `cospan.rs`'s `monoidal_product_validity`, was an `assert_valid(false, true)`
/// that the signature change reduced to `assert_valid()`, which no longer looks
/// at identities at all. So this pin is not new coverage of new behaviour — it
/// is coverage that left with the flags.
///
/// **What this ranges over.** Four tensors on `char` labels at apex sizes 1–3:
/// identity ⊗ identity, identity ⊗ (right-failing), (right-failing) ⊗
/// identity, and (left-failing) ⊗ identity. That covers **both operands of each
/// deleted `&=`** and **both legs**, which is the axis that mattered — the two
/// `&=`s were separate expressions and `monoidal` treats `self` and `other`
/// differently. It does **not** sweep apex sizes past 3, does not use a label
/// type whose `Eq` is coarser than identity, and pins the whole triple only on
/// the first tensor (the tests above cover triples in general).
///
/// Two measured falsifications, because one perturbation alone would not tell
/// which half of the test is doing work:
///
/// 1. Deleting `other.left.iter_mut().for_each(|v| *v += middle_shift);` from
///    `Cospan::monoidal` reddens **1 of this file's 7 tests — only this one**,
///    on the first tensor: `left == [0, 0, 1]` where `[0, 1, 2]` is expected
///    (`is_left_identity()` is `false` for that value too). Notably
///    `tensor_associativity_cospan` and `tensor_unit_cospan` stay **green**:
///    associativity compares two tensors that suffer the bug identically, and
///    both unit fixtures have `middle_shift == 0` or no `other` entries to
///    shift. So nothing else in the file sees a lost shift.
/// 2. Dropping `represents_id` from the private `leg_is_identity` reddens
///    **4 of the 7**, this one at its **last** assertion — the `[1, 0, 2]`
///    domain leg, which is the right length and out of order. That is the
///    assertion the three mixed fixtures exist for: the tuple `assert_eq!`s
///    alone say nothing about identity-ness.
#[test]
fn tensor_of_identities_is_an_identity() {
    let id_a = Cospan::<char>::identity(&vec!['a']);
    let id_bc = Cospan::<char>::identity(&vec!['b', 'c']);

    let mut both = id_a.clone();
    both.monoidal(id_bc);
    assert_eq!(
        (both.left_to_middle(), both.right_to_middle(), both.middle()),
        (&[0, 1, 2][..], &[0, 1, 2][..], &['a', 'b', 'c'][..]),
        "the shifted legs must cover the concatenated apex in order"
    );
    assert!(
        both.is_left_identity() && both.is_right_identity(),
        "id_a ⊗ id_bc is the identity on ['a', 'b', 'c']"
    );

    // A codomain leg that is neither long enough nor in order, so the tensor
    // must lose the right-hand answer and keep the left-hand one.
    let right_fails = Cospan::<char>::new(vec![0, 1], vec![1], vec!['p', 'q']).unwrap();
    assert!(right_fails.is_left_identity() && !right_fails.is_right_identity());

    let mut id_then_bad = id_a.clone();
    id_then_bad.monoidal(right_fails.clone());
    assert_eq!(
        (id_then_bad.left_to_middle(), id_then_bad.right_to_middle()),
        (&[0, 1, 2][..], &[0, 2][..])
    );
    assert!(
        id_then_bad.is_left_identity() && !id_then_bad.is_right_identity(),
        "`other`'s failing codomain leg must lose the tensor's right answer"
    );

    let mut bad_then_id = right_fails;
    bad_then_id.monoidal(id_a.clone());
    assert_eq!(
        (bad_then_id.left_to_middle(), bad_then_id.right_to_middle()),
        (&[0, 1, 2][..], &[1, 2][..])
    );
    assert!(
        bad_then_id.is_left_identity() && !bad_then_id.is_right_identity(),
        "the same operand on the left of the tensor: same answer, other arm"
    );

    // And the mirror on the domain leg, so a hypothetical `is_left_identity`
    // that ignored `self` could not pass.
    let left_fails = Cospan::<char>::new(vec![1, 0], vec![0, 1], vec!['p', 'q']).unwrap();
    let mut bad_left = left_fails;
    bad_left.monoidal(id_a);
    assert_eq!(bad_left.left_to_middle(), &[1, 0, 2]);
    assert!(
        !bad_left.is_left_identity() && bad_left.is_right_identity(),
        "`self`'s out-of-order domain leg must lose the tensor's left answer"
    );
}

// ---------------------------------------------------------------------------
// 3. Permutation cospan compose: β(p₁) ; β(p₂) == β(p₁ ; p₂)
// ---------------------------------------------------------------------------

/// `β(p₁) ; β(p₂) == β(p₁ ; p₂)`, over **all 36 ordered pairs of `S₃`** with
/// **distinct** labels.
///
/// ⚠ This test used to run one pair over the uniform word `['a','a','a']`
/// (#286). Uniform labels make `domain()` and `codomain()` constant in the
/// permutation, so both word assertions held for *any* `p₁`, `p₂` — including a
/// compose that realized `p₂ ; p₁` — and the only other assertion was
/// `middle.len() >= 3`. Distinct labels force the middle word, so `c2` has to be
/// built on `c1`'s actual codomain, and the wiring is compared against
/// `(0..n).map(|i| (p1 * p2).apply(i))` computed from the two permutations
/// directly rather than against a third call to the constructor under test —
/// which would cancel a symmetric drift.
///
/// # The space this claim ranges over
///
/// `S₃ × S₃` (36 pairs), `Cospan<char>`, `from_permutation_on_domain` only.
/// `n = 4` and the `on_codomain` constructor are covered in
/// `tests/braiding_core_pins.rs`, not here.
#[test]
fn permutation_cospan_compose() {
    let types: Vec<char> = vec!['a', 'b', 'c'];
    let perms = all_perms(3);
    assert_eq!(perms.len(), 6, "S3 has 6 elements");

    let mut checked = 0usize;
    for p1 in &perms {
        for p2 in &perms {
            // c1 : types → p1.inv().permute(types); c2 must start where it ends.
            let mid: Vec<char> = p1.inv().permute(&types);
            let c1 = Cospan::from_permutation_on_domain(p1.clone(), &types).unwrap();
            let c2 = Cospan::from_permutation_on_domain(p2.clone(), &mid).unwrap();
            assert!(
                c1.composable(&c2).is_ok(),
                "c1;c2 must be composable; p1={p1:?} p2={p2:?}"
            );

            let composed = c1.compose(&c2).expect("compose should succeed");
            let p12 = p1 * p2;

            assert_eq!(
                cospan_wiring(&composed),
                (0..3).map(|i| p12.apply(i)).collect::<Vec<_>>(),
                "composite wiring must be i ↦ p2(p1(i)); p1={p1:?} p2={p2:?}"
            );
            assert_eq!(
                composed.domain(),
                types,
                "composite domain; p1={p1:?} p2={p2:?}"
            );
            assert_eq!(
                composed.codomain(),
                p12.inv().permute(&types),
                "composite codomain; p1={p1:?} p2={p2:?}"
            );

            // A braiding merges each domain wire with exactly one codomain
            // wire, so the pushout has exactly `n` apex vertices — not merely
            // "at least n", which the pre-#286 assertion settled for.
            assert_eq!(
                composed.middle().len(),
                types.len(),
                "composite apex; p1={p1:?} p2={p2:?}"
            );

            composed.assert_valid();
            checked += 1;
        }
    }
    assert_eq!(checked, 36, "all 36 ordered S3 pairs must have run");
}

// ---------------------------------------------------------------------------
// 4. Symmetric braiding: swap composed with itself yields identity
// ---------------------------------------------------------------------------

#[test]
fn symmetric_braiding_involutive() {
    // Use uniform labels so the swap cospan is self-composable
    // (codomain labels match domain labels regardless of permutation).
    let types: Vec<char> = vec!['a', 'a'];

    // The swap permutation on 2 elements: (0 1).
    let swap = Permutation::transposition(2, 0, 1);
    let sigma = Cospan::from_permutation_on_domain(swap.clone(), &types).unwrap();

    // sigma ; sigma should give identity (the braiding is an involution).
    assert!(
        sigma.composable(&sigma).is_ok(),
        "swap should be self-composable"
    );
    let sigma_sq = sigma.compose(&sigma).expect("compose should succeed");

    // The identity cospan for comparison.
    let id = Cospan::<char>::identity(&types);

    // Domain and codomain must be the original types.
    assert_eq!(sigma_sq.domain(), id.domain(), "domain");
    assert_eq!(sigma_sq.codomain(), id.codomain(), "codomain");

    // The swap^2 cospan after pushout simplification should have the
    // same domain-to-middle and codomain-to-middle connectivity as identity:
    // each domain wire i connects to the same middle node as codomain wire i.
    for i in 0..types.len() {
        assert_eq!(
            sigma_sq.left_to_middle()[i],
            sigma_sq.right_to_middle()[i],
            "wire {i} should connect domain and codomain to the same middle node"
        );
    }

    sigma_sq.assert_valid();
}

// ---------------------------------------------------------------------------
// 5. Span tensor: verify monoidal product combines middle_pairs correctly
// ---------------------------------------------------------------------------

#[test]
fn span_tensor_combines_middle_pairs() {
    // Span s1: left=['a','b'], right=['a','b'], middle=[(0,0),(1,1)] (identity)
    let s1 = Span::<char>::identity(&vec!['a', 'b']);
    // Span s2: left=['c'], right=['c'], middle=[(0,0)] (identity on single wire)
    let s2 = Span::<char>::identity(&vec!['c']);

    let mut product = s1.clone();
    product.monoidal(s2.clone());

    // Domain and codomain are concatenated.
    assert_eq!(product.left(), &['a', 'b', 'c'], "tensor left");
    assert_eq!(product.right(), &['a', 'b', 'c'], "tensor right");

    // Middle pairs: s1 has [(0,0),(1,1)], s2 has [(0,0)].
    // After tensor, s2's pair is shifted to (0+2, 0+2) = (2,2).
    assert_eq!(
        product.middle_pairs(),
        &[(0, 0), (1, 1), (2, 2)],
        "tensor middle_pairs"
    );

    // A non-identity span to tensor with.
    // s3: left=['x','y'], right=['y','x'], middle=[(0,1),(1,0)] (swap relation).
    let s3 = Span::new(vec!['x', 'y'], vec!['y', 'x'], vec![(0, 1), (1, 0)]).unwrap();

    let mut s1_s3 = s1;
    s1_s3.monoidal(s3);

    assert_eq!(
        s1_s3.left(),
        &['a', 'b', 'x', 'y'],
        "non-trivial tensor left"
    );
    assert_eq!(
        s1_s3.right(),
        &['a', 'b', 'y', 'x'],
        "non-trivial tensor right"
    );
    // s1 middle: [(0,0),(1,1)], s3 middle shifted by (2,2): [(2,3),(3,2)].
    assert_eq!(
        s1_s3.middle_pairs(),
        &[(0, 0), (1, 1), (2, 3), (3, 2)],
        "non-trivial tensor middle_pairs"
    );
}

// ---------------------------------------------------------------------------
// 6. permute_side domain: permuting the domain of a cospan reorders the
//    left boundary.
// ---------------------------------------------------------------------------

#[test]
fn permute_side_reorders_domain() {
    // Start with identity cospan on ['a','b','c'].
    let types = vec!['a', 'b', 'c'];
    let mut c = Cospan::<char>::identity(&types);

    // Before permutation: left_to_middle = [0,1,2], domain = ['a','b','c'].
    assert_eq!(c.domain(), vec!['a', 'b', 'c']);
    assert_eq!(c.left_to_middle(), &[0, 1, 2]);

    // Apply rotation_left(3,1) to the domain side (of_codomain = false).
    // rotation_left(3,1) sends 0->1, 1->2, 2->0.
    let rot = Permutation::rotation_left(3, 1);
    c.permute_side(&rot, false);

    // #258: `permute_side` moves the wire at slot `i` to slot `p.apply(i)`, so
    // 'a' lands at slot 1, 'b' at 2 and 'c' at 0 — the leg vector becomes
    // `old ∘ p.inv()`, i.e. `[2, 0, 1]`, and the word `['c', 'a', 'b']`.
    //
    // ⚠ Before #258 the whole check here was `assert_ne!(left_to_middle,
    // [0,1,2])` plus a length assertion, both of which hold for *either*
    // direction and for `p.inv()` as readily as for `p`. That is why this file
    // stayed green while `Cospan` sat on the inverted convention.
    assert_eq!(
        c.left_to_middle(),
        &[2, 0, 1],
        "left leg is old ∘ p.inv(); {:?} would be the inverted convention",
        [1, 2, 0]
    );
    assert_eq!(
        c.domain(),
        vec!['c', 'a', 'b'],
        "the wire at slot i moves to slot p(i)"
    );
    assert_eq!(c.domain(), rot.inv().permute(&types));
    assert!(
        !c.is_left_identity(),
        "the permuted left leg is no longer the identity"
    );

    // Codomain should be untouched.
    assert_eq!(c.codomain(), vec!['a', 'b', 'c'], "codomain unchanged");
    assert_eq!(c.right_to_middle(), &[0, 1, 2], "right leg unchanged");
    assert!(
        c.is_right_identity(),
        "the untouched right leg is still the identity"
    );

    // The cospan should still be valid.
    c.assert_valid();
}
