//! Arrow-algebra laws for the owned [`arrow_seam`](catgraph_syntax::arrow_seam)
//! surface ([#222](https://github.com/sustia-llc/catgraph/issues/222)).
//!
//! The S5 coherence suite (`tests/traced.rs`) exercises composition and the
//! tensor only *through* [`Traced`](catgraph_syntax::traced::Traced), where every
//! check also drags in a term, a model and the interpreter — so an arrow-side
//! regression would surface there as a coherence failure, if at all. Owning the
//! algebra means owning its laws directly:
//!
//! - **Identity** — `id >>> f == f == f >>> id` (`Id` is the unit of `Compose`).
//! - **Associativity** — `(f >>> g) >>> h == f >>> (g >>> h)`.
//! - **Bifunctoriality of `***`** — the interchange law
//!   `(f *** g) >>> (h *** k) == (f >>> h) *** (g >>> k)`, with `id *** id == id`
//!   as its unit case.
//! - Plus the derived surface (`first`/`second`/`fanout`) against its defining
//!   equations, and the builder against the combinators it hides.
//!
//! Arrows are functions, so each law is checked by *extensionality* over a fixed
//! sample set: the pipelines are affine over `i64` (`+`/`*`/negate), so a handful
//! of points — including the sign boundaries and `0` — pins every composite. The
//! samples stay small so no product overflows along a chain.

use catgraph_syntax::arrow_seam::{Arrow, ArrowBuilder, Fanout, First, Id, Lift, Second, arrow};

/// The extensionality sample: sign boundaries, zero, and two magnitudes.
const SAMPLES: [i64; 5] = [-7, -1, 0, 3, 11];

/// The sample pairs for the two-wire (`***`) laws — the `SAMPLES` points paired
/// with a rotation of themselves. `SAMPLES` has no repeated values, so the two
/// components of every pair genuinely differ (a fixed point like `(0, 0)`
/// would blind the component-swap-sensitive laws).
fn sample_pairs() -> impl Iterator<Item = (i64, i64)> {
    SAMPLES.into_iter().zip(SAMPLES.into_iter().cycle().skip(1))
}

fn inc() -> impl Arrow<In = i64, Out = i64> {
    Lift::new(|x: i64| x + 1)
}

fn dbl() -> impl Arrow<In = i64, Out = i64> {
    Lift::new(|x: i64| x * 2)
}

fn neg() -> impl Arrow<In = i64, Out = i64> {
    Lift::new(|x: i64| -x)
}

#[test]
fn id_is_the_unit_of_compose_on_both_sides() {
    let bare = inc();
    let left = Id::new().compose(inc());
    let right = inc().compose(Id::new());

    for x in SAMPLES {
        let expected = bare.run(x);
        assert_eq!(left.run(x), expected, "id >>> f differs at {x}");
        assert_eq!(right.run(x), expected, "f >>> id differs at {x}");
    }
}

#[test]
fn compose_is_associative() {
    let left_nested = inc().compose(dbl()).compose(neg());
    let right_nested = inc().compose(dbl().compose(neg()));

    for x in SAMPLES {
        assert_eq!(
            left_nested.run(x),
            right_nested.run(x),
            "(f >>> g) >>> h differs from f >>> (g >>> h) at {x}"
        );
    }
}

#[test]
fn split_is_bifunctorial() {
    // Interchange: tensoring then composing == composing then tensoring.
    let tensor_then_compose = inc().split(dbl()).compose(neg().split(inc()));
    let compose_then_tensor = inc().compose(neg()).split(dbl().compose(inc()));
    // Unit: the tensor of identities is the identity on the pair.
    let id_tensor_id = Id::new().split(Id::new());

    for (a, b) in sample_pairs() {
        assert_eq!(
            tensor_then_compose.run((a, b)),
            compose_then_tensor.run((a, b)),
            "interchange fails at ({a}, {b})"
        );
        assert_eq!(
            id_tensor_id.run((a, b)),
            (a, b),
            "id *** id moved ({a}, {b})"
        );
    }
}

#[test]
fn strength_and_fanout_match_their_defining_equations() {
    // `first`/`second` (the provided methods) are the tensor with an identity
    // on the other side.
    let first = inc().first();
    let first_as_split = inc().split(Id::new());
    let second = inc().second();
    let second_as_split = Id::new().split(inc());
    // `fanout` duplicates the input into both branches.
    let fanout = inc().fanout(dbl());

    for (a, b) in sample_pairs() {
        assert_eq!(first.run((a, b)), first_as_split.run((a, b)));
        assert_eq!(second.run((b, a)), second_as_split.run((b, a)));
        assert_eq!(fanout.run(a), (a + 1, a * 2), "fanout differs at {a}");
    }

    // The explicit constructors agree with the combinator methods.
    let first_direct = First::new(inc());
    let second_direct = Second::new(inc());
    let fanout_direct = Fanout::new(inc(), dbl());
    for (a, b) in sample_pairs() {
        assert_eq!(first_direct.run((a, b)), first.run((a, b)));
        assert_eq!(second_direct.run((b, a)), second.run((b, a)));
        assert_eq!(fanout_direct.run(a), fanout.run(a));
    }
}

#[test]
fn builder_chains_denote_the_combinators_they_hide() {
    let built = arrow(|x: i64| x + 1).then_fn(|x| x * 2).build();
    let hand = inc().compose(dbl());
    let wrapped = ArrowBuilder::new(inc()).then(dbl());
    let paired = arrow(|x: i64| x + 1).par(neg()).build();
    // `fanout` on a builder feeds the CHAIN'S ORIGINAL input to the second
    // arrow (bound `G: Arrow<In = S::In>`), pairing the chain's output with
    // `g(original)` — not `g(current stage output)`.
    let fanned = arrow(|x: i64| x + 1)
        .then_fn(|x| x * 2)
        .fanout(neg())
        .build();

    for (a, b) in sample_pairs() {
        assert_eq!(built.run(a), hand.run(a), "then_fn chain differs at {a}");
        assert_eq!(wrapped.run(a), hand.run(a), "then chain differs at {a}");
        assert_eq!(
            paired.run((a, b)),
            (a + 1, -b),
            "par chain differs at ({a}, {b})"
        );
        assert_eq!(
            fanned.run(a),
            ((a + 1) * 2, -a),
            "builder fanout must feed the chain's original input at {a}"
        );
    }
}
