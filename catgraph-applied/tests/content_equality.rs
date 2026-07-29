//! Content equality on the committed divergence witnesses
//! ([#57](https://github.com/sustia-llc/catgraph/issues/57), a1).
//!
//! `docs/SMC-NF-RECONCILIATION.md` §4.4 / §4.6 name the SMC-equal pairs whose
//! normal forms differ, and §4.6 names the ones a pass has since closed. Lemma
//! 4.1 (§4.2) says content decides SMC-equality on all of them alike, so every
//! witness below is a two-sided check: `content_eq` must agree with the *SMC*
//! verdict whatever `nf` does with the pair.
//!
//! The three negatives are the other half of the claim. In particular
//! `Copy ; Add` against `Copy ; σ ; Add` must come out **unequal**:
//! cocommutativity is one of the 18 Thm 5.60 *user* equations, and `C` quotients
//! by SMC coherence and nothing more.
//!
//! Also checked here: `canonical_key`'s `iff` on the same pairs, content
//! preservation under `nf` (§4.3 Lemma 4.2), and the Λ-colored path on a
//! two-color fixture.

use std::borrow::Cow;

use catgraph_applied::prop::colored::ColoredExpr;
use catgraph_applied::prop::presentation::content::{
    canonical_key, content_eq, content_of, content_of_colored,
};
use catgraph_applied::prop::presentation::smc_nf::{from_string_diagram, nf};
use catgraph_applied::prop::{Free, PropExpr, PropSignature};
use catgraph_applied::rig::BoolRig;
use catgraph_applied::sfg::SfgGenerator;

type Sfg = SfgGenerator<BoolRig>;
type E = PropExpr<Sfg>;

fn prim(g: Sfg) -> E {
    PropExpr::Generator(g)
}
fn id1() -> E {
    PropExpr::Identity(1)
}
fn seq(a: E, b: E) -> E {
    PropExpr::Compose(Box::new(a), Box::new(b))
}
fn par(a: E, b: E) -> E {
    PropExpr::Tensor(Box::new(a), Box::new(b))
}
fn strue() -> E {
    prim(SfgGenerator::Scalar(BoolRig(true)))
}
fn sfalse() -> E {
    prim(SfgGenerator::Scalar(BoolRig(false)))
}

/// Assert `content_eq` and `canonical_key` both return the *SMC* verdict, and
/// that each side's content survives a round trip through the normal form.
fn witness(name: &str, a: &E, b: &E, smc_equal: bool) {
    let (ca, cb) = (content_of(a), content_of(b));

    assert_eq!(
        content_eq(&ca, &cb),
        smc_equal,
        "{name}: content_eq disagrees with the SMC verdict (nf agrees: {})",
        nf(a) == nf(b)
    );
    assert_eq!(
        canonical_key(&ca) == canonical_key(&cb),
        smc_equal,
        "{name}: canonical_key disagrees with content_eq"
    );

    // §4.3 Lemma 4.2: every pipeline rewrite preserves content, so the readback
    // of the normal form has the same content as the expression itself.
    for (side, e) in [("lhs", a), ("rhs", b)] {
        let readback = from_string_diagram(&nf(e));
        assert!(
            content_eq(&content_of(e), &content_of(&readback)),
            "{name} ({side}): nf did not preserve content"
        );
    }
}

// ---- §4.4 / §4.6 witnesses --------------------------------------------------

/// The withdrawal witness of §4.4: `η` placement slack, the single mechanism
/// behind all 253 divergences and the reason rigidity on `𝔉` was withdrawn.
#[test]
fn eta_layer_slack() {
    witness(
        "eta-layer-slack",
        &par(
            seq(
                prim(SfgGenerator::Copy),
                par(id1(), prim(SfgGenerator::Discard)),
            ),
            prim(SfgGenerator::Zero),
        ),
        &seq(
            prim(SfgGenerator::Copy),
            par(
                par(id1(), prim(SfgGenerator::Zero)),
                prim(SfgGenerator::Discard),
            ),
        ),
        true,
    );
}

/// §4.4 F2: a braid prefix that the diagram carries but the content does not —
/// NF braid-freeness is per-writing, so no NF-level fix reaches this pair.
#[test]
fn dead_braid_prefix() {
    witness(
        "dead-braid-prefix",
        &seq(
            PropExpr::Braid(1, 1),
            par(prim(SfgGenerator::Discard), prim(SfgGenerator::Discard)),
        ),
        &par(prim(SfgGenerator::Discard), prim(SfgGenerator::Discard)),
        true,
    );
}

/// §4.4 F1 / [#185](https://github.com/sustia-llc/catgraph/issues/185): the
/// `adjacent_column_cuts` right-column asymmetry, inside `𝔉′`.
#[test]
fn cut_asymmetry() {
    witness(
        "cut-asymmetry",
        &seq(
            seq(
                prim(SfgGenerator::Copy),
                par(par(id1(), prim(SfgGenerator::Zero)), sfalse()),
            ),
            par(par(id1(), strue()), prim(SfgGenerator::Discard)),
        ),
        &par(
            seq(
                seq(prim(SfgGenerator::Copy), par(id1(), sfalse())),
                par(id1(), prim(SfgGenerator::Discard)),
            ),
            seq(prim(SfgGenerator::Zero), strue()),
        ),
        true,
    );
}

/// §4.6(c), the trapped closed block — closed by the Step 6½ column pass, so
/// `nf` converges here too. Content must agree with the fix, not merely with the
/// gap.
#[test]
fn trapped_closed_block() {
    witness(
        "trapped-closed",
        &seq(
            seq(
                seq(
                    prim(SfgGenerator::Copy),
                    par(par(id1(), prim(SfgGenerator::Zero)), id1()),
                ),
                par(par(id1(), prim(SfgGenerator::Discard)), id1()),
            ),
            prim(SfgGenerator::Add),
        ),
        &par(
            seq(prim(SfgGenerator::Zero), prim(SfgGenerator::Discard)),
            seq(prim(SfgGenerator::Copy), prim(SfgGenerator::Add)),
        ),
        true,
    );
}

/// §4.6(d), the CE-A nested sink form — the family that refuted the first
/// rigidity draft, closed behaviourally by Step 6½.
#[test]
fn ce_a_nested_sink() {
    witness(
        "CE-A nested sink",
        &seq(
            seq(
                par(par(prim(SfgGenerator::Zero), strue()), id1()),
                par(par(id1(), prim(SfgGenerator::Discard)), id1()),
            ),
            prim(SfgGenerator::Add),
        ),
        &par(
            seq(strue(), prim(SfgGenerator::Discard)),
            seq(
                par(prim(SfgGenerator::Zero), id1()),
                prim(SfgGenerator::Add),
            ),
        ),
        true,
    );
}

/// The [#55](https://github.com/sustia-llc/catgraph/issues/55) diagnosis pair:
/// two zero-arity atoms in either tensor order.
#[test]
fn zero_arity_tensor_order() {
    witness(
        "zero-arity tensor order",
        &par(prim(SfgGenerator::Discard), prim(SfgGenerator::Zero)),
        &par(prim(SfgGenerator::Zero), prim(SfgGenerator::Discard)),
        true,
    );
}

// ---- negatives --------------------------------------------------------------

/// Different morphisms, different content.
#[test]
fn negative_distinct_shapes() {
    witness(
        "Copy ; Add vs id₁",
        &seq(prim(SfgGenerator::Copy), prim(SfgGenerator::Add)),
        &id1(),
        false,
    );
}

/// Labels separate: the two scalars differ only in their `BoolRig` payload.
#[test]
fn negative_distinct_scalars() {
    witness(
        "scalar true vs false",
        &seq(
            prim(SfgGenerator::Zero),
            seq(strue(), prim(SfgGenerator::Discard)),
        ),
        &seq(
            prim(SfgGenerator::Zero),
            seq(sfalse(), prim(SfgGenerator::Discard)),
        ),
        false,
    );
}

/// **The layer boundary.** Same edge multiset, different wiring: `Copy ; Add`
/// against `Copy ; σ ; Add`. They are equal in the prop *presented* by the 18
/// Thm 5.60 equations (cocommutativity, A2/B2) and unequal in the **free** prop.
/// `C` decides the free-prop question, so this must be unequal — anything else
/// would mean content had absorbed a user equation.
#[test]
fn negative_free_prop_cocommutativity_is_a_user_equation() {
    witness(
        "Copy ; Add vs Copy ; σ ; Add",
        &seq(prim(SfgGenerator::Copy), prim(SfgGenerator::Add)),
        &seq(
            prim(SfgGenerator::Copy),
            seq(PropExpr::Braid(1, 1), prim(SfgGenerator::Add)),
        ),
        false,
    );
}

// ---- closed components at scale ---------------------------------------------

/// A closed component: `Zero ; Scalar(r) ; Discard`, `0 → 0`.
fn bubble(r: bool) -> E {
    seq(
        seq(
            prim(SfgGenerator::Zero),
            prim(SfgGenerator::Scalar(BoolRig(r))),
        ),
        prim(SfgGenerator::Discard),
    )
}

/// `k` bubbles tensored, all carrying `true` except the one at `odd`.
fn bubbles(k: usize, odd: Option<usize>) -> E {
    (0..k)
        .map(|i| bubble(Some(i) != odd))
        .reduce(par)
        .expect("k > 0")
}

/// **Backtracking regression.** `k` pairwise-isomorphic closed components used to
/// drive the closed-component match factorially — a near-miss at `k = 12` took
/// ~52 s. Closed components are now compared by a complete invariant instead of
/// being matched against each other, so both decisions are immediate.
///
/// The generous time bound is a floor-through-the-floor guard, not a benchmark:
/// the work here is microseconds, and the behavior it rules out was measured in
/// tens of seconds.
#[test]
fn many_isomorphic_closed_components_do_not_backtrack() {
    const K: usize = 12;
    let budget = std::time::Duration::from_secs(10);

    // Equal: the same 12 bubbles, the odd one written first against last. Two
    // `0 → 0` blocks always commute, so these are SMC-equal.
    let start = std::time::Instant::now();
    let (ca, cb) = (
        content_of(&bubbles(K, Some(0))),
        content_of(&bubbles(K, Some(K - 1))),
    );
    assert!(content_eq(&ca, &cb));
    assert_eq!(canonical_key(&ca), canonical_key(&cb));
    assert!(
        start.elapsed() < budget,
        "equal case took {:?}",
        start.elapsed()
    );

    // Near miss: 12 identical bubbles against 11 identical plus one differing
    // scalar. This is the shape that maximized the old search.
    let start = std::time::Instant::now();
    let (cc, cd) = (
        content_of(&bubbles(K, None)),
        content_of(&bubbles(K, Some(K - 1))),
    );
    assert!(!content_eq(&cc, &cd));
    assert_ne!(canonical_key(&cc), canonical_key(&cd));
    assert!(
        start.elapsed() < budget,
        "near-miss case took {:?}",
        start.elapsed()
    );
}

// ---- serde trust boundary ---------------------------------------------------

/// `ColoredExpr`'s deserialization does not re-run `colored::check`, so a
/// hand-crafted document can carry a source word shorter than the expression's
/// arity. `content_of_colored` must leave the uncovered coordinates untyped
/// rather than index past the word.
#[cfg(feature = "serde")]
#[test]
fn colored_content_survives_a_short_source_word_from_serde() {
    // `BoolRig` carries no serde derives; the payload is irrelevant here.
    type SerdeSfg = SfgGenerator<i64>;

    let good = ColoredExpr::<SerdeSfg>::new(vec![()], PropExpr::Identity(1))
        .expect("id₁ is well-formed at one wire");
    let mut document = serde_json::to_value(&good).expect("serialize");
    document["source_word"] = serde_json::json!([]);
    let short: ColoredExpr<SerdeSfg> =
        serde_json::from_value(document).expect("deserialization skips the check");

    let content = content_of_colored(&short);
    assert_eq!(
        content.node_colors(),
        &[None],
        "the uncovered coordinate stays untyped"
    );
}

// ---- Λ-colored path ---------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Wire {
    A,
    B,
}

const W_A: &[Wire] = &[Wire::A];
const W_B: &[Wire] = &[Wire::B];

/// `F : A → B` and `G : B → A`, the two-color fixture of
/// `tests/colored_presentation.rs`, narrowed to what a content needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Two {
    F,
    G,
}

impl PropSignature for Two {
    type Color = Wire;

    fn source_word(&self) -> Cow<'_, [Wire]> {
        Cow::Borrowed(match self {
            Two::F => W_A,
            Two::G => W_B,
        })
    }

    fn target_word(&self) -> Cow<'_, [Wire]> {
        Cow::Borrowed(match self {
            Two::F => W_B,
            Two::G => W_A,
        })
    }
}

fn colored(word: &[Wire], expr: PropExpr<Two>) -> ColoredExpr<Two> {
    ColoredExpr::new(word.to_vec(), expr).expect("word-well-formed fixture")
}

/// Generator tentacle words type the nodes they touch, for any `Λ` — no colored
/// entry point required.
#[test]
fn colored_generator_tentacles_type_their_nodes() {
    let c = content_of(&Free::<Two>::generator(Two::F));
    assert_eq!(c.node_colors(), &[Some(Wire::A), Some(Wire::B)]);
}

/// A node's color is its **producer**'s declared letter, so the typing stays a
/// content invariant even where `colored::check` would reject the expression —
/// `F ; F` glues F's target `B` to F's source `A`. Two SMC-equal writings of that
/// ill-colored term must still agree.
#[test]
fn colored_typing_survives_an_ill_colored_compose() {
    let f = || Free::<Two>::generator(Two::F);
    // Arity-well-formed (1 = 1); `colored::check` rejects it, `content_of` does not.
    let direct = Free::compose(f(), f()).expect("arity check accepts F ; F");
    let padded = Free::compose(
        Free::compose(f(), Free::<Two>::identity(1)).expect("F ; id₁"),
        f(),
    )
    .expect("(F ; id₁) ; F");

    let (ca, cb) = (content_of(&direct), content_of(&padded));
    assert_eq!(
        ca.node_colors(),
        &[Some(Wire::A), Some(Wire::B), Some(Wire::B)],
        "the glued wire takes its producer's letter, not its consumer's"
    );
    assert!(content_eq(&ca, &cb));
    assert_eq!(canonical_key(&ca), canonical_key(&cb));
}

/// A colored entry point types every node, so the boundary words are readable
/// off the content.
#[test]
fn colored_content_is_fully_typed() {
    // (F ⊗ id₁) : A B → B B — the identity wire is the one `content_of` alone
    // cannot type.
    let expr = Free::tensor(Free::<Two>::generator(Two::F), Free::<Two>::identity(1));
    let positional = content_of(&expr);
    assert!(
        positional.node_colors().iter().any(Option::is_none),
        "the identity wire has no color the expression determines"
    );

    let c = content_of_colored(&colored(&[Wire::A, Wire::B], expr));
    assert!(c.node_colors().iter().all(Option::is_some));

    let source: Vec<Wire> = c
        .input()
        .iter()
        .map(|&x| c.node_colors()[x].expect("fully typed"))
        .collect();
    let target: Vec<Wire> = c
        .output()
        .iter()
        .map(|&x| c.node_colors()[x].expect("fully typed"))
        .collect();
    assert_eq!(source, vec![Wire::A, Wire::B]);
    assert_eq!(target, vec![Wire::B, Wire::B]);
}

/// Interchange over a two-color signature: `F ⊗ G` against its split, both at
/// `A B`. Content-equal and key-equal, with every node typed.
#[test]
fn colored_interchange_split_has_equal_content() {
    let f = || Free::<Two>::generator(Two::F);
    let g = || Free::<Two>::generator(Two::G);

    let tensored = Free::tensor(f(), g());
    let split = Free::compose(
        Free::tensor(f(), Free::<Two>::identity(1)),
        Free::tensor(Free::<Two>::identity(1), g()),
    )
    .expect("(F ⊗ id) : A B → B B ; (id ⊗ G) : B B → B A");

    let word = [Wire::A, Wire::B];
    let (ca, cb) = (
        content_of_colored(&colored(&word, tensored)),
        content_of_colored(&colored(&word, split)),
    );
    assert!(content_eq(&ca, &cb));
    assert_eq!(canonical_key(&ca), canonical_key(&cb));
}

/// Word-awareness bites: one expression, two source words. The positional
/// contents coincide; the colored ones must not.
#[test]
fn colored_content_separates_source_words() {
    let expr = || Free::<Two>::identity(1);
    assert!(content_eq(&content_of(&expr()), &content_of(&expr())));

    let at_a = content_of_colored(&colored(W_A, expr()));
    let at_b = content_of_colored(&colored(W_B, expr()));
    assert!(
        !content_eq(&at_a, &at_b),
        "id₁ at A and id₁ at B are different morphisms"
    );
    assert_ne!(canonical_key(&at_a), canonical_key(&at_b));
}

/// Braid involution over two colors: `σ ; σ = id₂` at `A B`, with the colors
/// flowing through the braids.
#[test]
fn colored_braid_involution_has_equal_content() {
    let braid = || Free::<Two>::braid(1, 1);
    let twice = Free::compose(braid(), braid()).expect("2 → 2 ; 2 → 2");

    let word = [Wire::A, Wire::B];
    let (ca, cb) = (
        content_of_colored(&colored(&word, Free::<Two>::identity(2))),
        content_of_colored(&colored(&word, twice)),
    );
    assert!(content_eq(&ca, &cb));
    assert_eq!(canonical_key(&ca), canonical_key(&cb));
}
