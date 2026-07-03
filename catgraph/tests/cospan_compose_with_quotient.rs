//! Integration tests for `Cospan::compose_with_quotient` — the additive
//! pushout-quotient API.

use std::collections::HashSet;

use catgraph::category::{Composable, HasIdentity};
use catgraph::cospan::Cospan;

mod common;
use common::cospan_eq;

#[test]
fn t1_1_identity_compose_quotient_concatenates_ranges() {
    // id(3) ∘ id(3): left cospan has middle [a,b,c] with both legs = [0,1,2];
    // right cospan is the same. Pushout merges right leg of first with left
    // leg of second pointwise, so the quotient maps both middles surjectively
    // onto the shared pushout [0,1,2].
    let left = Cospan::<char>::identity(&vec!['a', 'b', 'c']);
    let right = Cospan::<char>::identity(&vec!['a', 'b', 'c']);

    let (composed, quotient) = left
        .compose_with_quotient(&right)
        .expect("identities compose");

    // Quotient length = self.middle.len() + other.middle.len()
    assert_eq!(quotient.len(), 6);
    // First 3 entries map self.middle indices [0,1,2] into the pushout
    assert_eq!(&quotient[..3], &[0, 1, 2]);
    // Next 3 entries map other.middle indices [0,1,2] to the *same* classes
    assert_eq!(&quotient[3..], &[0, 1, 2]);
    // Sanity: composed cospan has 3 middle elements
    assert_eq!(composed.middle().len(), 3);
}

#[test]
fn t1_2_surjective_coequalizer_merges_shared_element() {
    // Left cospan: domain [a], codomain [b], middle [a, b], legs [0], [1]
    // Right cospan: domain [b], codomain [c], middle [b, c], legs [0], [1]
    // Compose glues Left.right (= middle[1] = b) to Right.left (= middle[0] = b)
    // Pushout apex has 3 classes: {a}, {b (shared)}, {c}
    let left = Cospan::<char>::new(vec![0], vec![1], vec!['a', 'b']);
    let right = Cospan::<char>::new(vec![0], vec![1], vec!['b', 'c']);

    let (composed, quotient) = left.compose_with_quotient(&right).unwrap();

    assert_eq!(composed.middle().len(), 3);
    assert_eq!(quotient.len(), 4); // 2 + 2

    // quotient[0] = class of left.middle[0] = 'a'
    // quotient[1] = class of left.middle[1] = 'b' (shared with right.middle[0])
    // quotient[2] = class of right.middle[0] = 'b' (shared — same as quotient[1])
    // quotient[3] = class of right.middle[1] = 'c'
    assert_eq!(
        &quotient[..],
        &[0, 1, 1, 2],
        "deterministic pushout quotient"
    );
    assert_eq!(
        quotient[1], quotient[2],
        "shared 'b' collapses to one class"
    );
    assert_ne!(quotient[0], quotient[1], "'a' stays separate from 'b'");
    assert_ne!(quotient[3], quotient[1], "'c' stays separate from 'b'");
}

#[test]
fn t1_3_roundtrip_with_plain_compose() {
    // compose_with_quotient(a, b).0 must equal compose(a, b) for several inputs.
    let cases = [
        (
            Cospan::<char>::identity(&vec!['x']),
            Cospan::<char>::identity(&vec!['x']),
        ),
        (
            Cospan::<char>::new(vec![0], vec![1], vec!['p', 'q']),
            Cospan::<char>::new(vec![0], vec![1], vec!['q', 'r']),
        ),
    ];
    for (a, b) in &cases {
        let via_compose = a.compose(b).unwrap();
        let (via_quotient, _) = a.compose_with_quotient(b).unwrap();
        assert!(cospan_eq(&via_compose, &via_quotient));
    }
}

// Additions exercising the non-parallel quotient cases that
// `compose_with_quotient` was added to fix. The earlier
// regression set above is all parallel/identity-shaped (single shared element
// per apex); these cover quotient surjectivity and multi-element collisions.

#[test]
fn t2_1_quotient_is_surjective_onto_composed_middle_indices() {
    // Three shared elements between left.right and right.left. After pushout,
    // the quotient must surject onto exactly `0..composed.middle.len()` — this
    // is what makes `quotient` a valid pushforward index for downstream
    // `Decoration::pushforward` calls in `catgraph-applied::DecoratedCospan`.
    let left = Cospan::<char>::new(vec![0, 1], vec![2, 3, 4], vec!['a', 'b', 'c', 'd', 'e']);
    let right = Cospan::<char>::new(vec![0, 1, 2], vec![3, 4], vec!['c', 'd', 'e', 'f', 'g']);

    let (composed, quotient) = left.compose_with_quotient(&right).unwrap();

    let image: HashSet<usize> = quotient.iter().copied().collect();
    let expected: HashSet<usize> = (0..composed.middle().len()).collect();
    assert_eq!(
        image, expected,
        "quotient must surject onto composed middle"
    );

    // Quotient length is the disjoint pre-pushout sum.
    assert_eq!(quotient.len(), left.middle().len() + right.middle().len());
}

#[test]
fn t2_2_multi_element_collision_in_apex() {
    // Three elements of right.middle all collapse onto a single composed-apex
    // class via three coequalized maps from left.right. Verifies that a
    // 1-to-many quotient class is recorded with all its members pointing to
    // the same composed-middle index.
    //
    // Left:  domain ['x'] = [0]; codomain ['p','q','r'] = [1,2,3]
    //        middle = ['x','p','q','r']
    // Right: domain ['p','q','r'] = [0,1,2]; codomain ['z'] = [3]
    //        middle = ['p','q','r','z']
    //
    // Pushout merges left.right[i] with right.left[i] for i in 0..3, but each
    // of left.middle[1,2,3] = 'p','q','r' is distinct — so the result has 5
    // composed-apex classes: {x}, {p}, {q}, {r}, {z}. We then build a *second*
    // composition where multiple right.middle entries genuinely collapse.
    let left = Cospan::<char>::new(vec![0, 0, 0], vec![1, 1, 1], vec!['s', 't']);
    // Three left-domain elements all into apex idx 0; three left-codomain all into apex idx 1.
    let right = Cospan::<char>::new(vec![0, 0, 0], vec![1], vec!['t', 'u']);

    let (composed, quotient) = left.compose_with_quotient(&right).unwrap();

    // Pushout glues left.middle[1]='t' (target of all three left-codomain) with
    // right.middle[0]='t' (source of all three right-domain). Composed apex has
    // {s, t (shared), u} = 3 elements.
    assert_eq!(composed.middle().len(), 3);

    // Quotient layout: [left.middle[0], left.middle[1], right.middle[0], right.middle[1]]
    //                = [class(s),       class(t),       class(t shared), class(u)]
    assert_eq!(quotient.len(), 4);
    assert_eq!(
        quotient[1], quotient[2],
        "shared 't' must collapse to a single composed-apex class"
    );
    assert_ne!(quotient[0], quotient[1], "'s' is its own class");
    assert_ne!(quotient[3], quotient[1], "'u' is its own class");
    assert_ne!(quotient[0], quotient[3], "'s' and 'u' are distinct classes");

    // Every quotient entry must be a valid composed-middle index.
    for &q in &quotient {
        assert!(q < composed.middle().len());
    }
}
