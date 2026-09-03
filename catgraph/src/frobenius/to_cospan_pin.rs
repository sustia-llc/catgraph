//! Differential pin for `frobenius_to_cospan`.
//!
//! [`reference_to_cospan`] is a second reading of the same semantics map; the
//! two are measured equal up to
//! [`canonical_form`](crate::cospan_canon::CospanCanon) over 383 terms. The
//! independence is partial: the spider route (one apex vertex built directly,
//! versus a recursion into `special_frobenius_morphism`) and the layer fold
//! differ; the six arms other than `Spider` and `UnSpecifiedBox` — the
//! hand-built braiding literal included — are byte-identical to the survivor's,
//! so a convention error applied to both copies alike is not visible here.
//!
//! The module sits inside the crate because the reference algorithm walks
//! `FrobeniusMorphism::layers`, which is `pub(crate)`.
//!
//! # Falsification
//!
//! Perturbing `cospan_algebra::generator_to_cospan` takes
//! [`the_two_frobenius_to_cospan_agree_over_the_wide_space`] red two ways:
//!
//! | perturbation | result |
//! |---|---|
//! | braiding right leg `vec![1, 0]` → `vec![0, 1]` | red at `random_5`: the ill-typed braiding makes the layer fold fail outright (`'a'` vs `'b'` at a common interface) |
//! | delete the `Spider(z, 0, 0)` carve-out, i.e. recurse | **0 of 383** — not red |
//! | `Comultiplication(z)` → the *disconnected* `Cospan::new_unchecked(vec![0], vec![0, 1], vec![z, z])` | red, **169 of 383** terms disagree — `delta`: survivor `apex=2` vs reference `apex=1` |
//!
//! [`black_boxes_are_rejected_by_both`] goes red when the survivor's variant is
//! switched to `CatgraphError::Composition`.
//!
//! The space is falsified separately: short-circuiting [`random_generator`] to
//! `None` leaves 300 identities that agree perfectly, so the differential
//! assertion stays green and the [`MIN_RANDOM_DISTINCT`] floor is what catches
//! it — **7 distinct canonical forms over the 300 random terms, against the 175
//! measured here**.

use crate::{
    category::{Composable, ComposableMutating, HasIdentity},
    compact_closed::{cap, cup, name, unname},
    cospan::Cospan,
    cospan_canon::CospanCanon,
    errors::CatgraphError,
    frobenius::{FrobeniusMorphism, FrobeniusOperation},
    monoidal::Monoidal,
};
use rand::{RngExt, SeedableRng, rngs::StdRng};
use std::collections::HashSet;
use std::fmt::Debug;

// The surviving implementation, reached through the `frobenius::` re-export,
// so that path is exercised rather than bypassed.
use super::frobenius_to_cospan as survivor_to_cospan;

type FM = FrobeniusMorphism<char, String>;

/// The wire labels the random terms are built on. Two of them, so the braiding
/// is a real crossing of *distinct* types and not an accidental identity.
const LABELS: [char; 2] = ['a', 'b'];

/// How many random terms [`space`] contributes.
const RANDOM_TERMS: usize = 300;

/// The exact size of [`space`]: eighty-three hand-built terms plus
/// [`RANDOM_TERMS`]. Asserted, so the space cannot shrink silently.
const SPACE_SIZE: usize = 83 + RANDOM_TERMS;

/// Floor on the number of **distinct canonical forms** among the random terms.
///
/// `SPACE_SIZE` guards the space's *count*; this guards its *content*.
/// Measured on the pinned seed: **175** distinct canonical forms over the 300
/// random terms. The floor sits below the measured value so an incidental
/// reshuffle of the draw does not red the pin, while a collapse cannot pass.
const MIN_RANDOM_DISTINCT: usize = 150;

/// Floor on the number of distinct canonical forms over the **whole** space.
///
/// Measured on the pinned seed: **212**. This one additionally notices a
/// hand-built block (the spider grid, the Def 2.5 battery, the compact-closed
/// terms) degenerating without its length changing.
const MIN_TOTAL_DISTINCT: usize = 180;

// ---------------------------------------------------------------------------
// The reference implementation: the retired G1-T1 body
// ---------------------------------------------------------------------------

/// Interpret one generator as the cospan it denotes — the **G1-T1** reading.
///
/// This is the retired `frobenius::operations::operation_to_cospan`, kept as an
/// independent oracle. It differs from the surviving
/// [`cospan_algebra::generator_to_cospan`](crate::cospan_algebra) in two places
/// that matter, which is what makes the comparison worth running:
///
/// - `Spider(z, m, n)` is built **directly** as a one-vertex apex with `m`
///   domain wires and `n` codomain wires on it. The survivor instead recurses
///   into [`special_frobenius_morphism`](crate::frobenius::special_frobenius_morphism)
///   for every `(m, n) != (0, 0)` — a generator decomposition, then a fold. The
///   two routes share no code.
/// - `UnSpecifiedBox` is rejected with [`CatgraphError::Composition`]; the
///   survivor rejects it with [`CatgraphError::Interpret`]. No term in [`space`]
///   carries a black box, so the wide pin never reaches this arm; the divergence
///   is pinned separately in [`black_boxes_are_rejected_by_both`].
fn reference_generator<Lambda, BlackBoxLabel>(
    op: &FrobeniusOperation<Lambda, BlackBoxLabel>,
) -> Result<Cospan<Lambda>, CatgraphError>
where
    Lambda: Eq + Copy + Debug,
    BlackBoxLabel: Eq + Clone,
{
    use crate::hypergraph_category::HypergraphCategory;

    Ok(match op {
        FrobeniusOperation::Unit(z) => Cospan::unit(*z),
        FrobeniusOperation::Counit(z) => Cospan::counit(*z),
        FrobeniusOperation::Multiplication(z) => Cospan::multiplication(*z),
        FrobeniusOperation::Comultiplication(z) => Cospan::comultiplication(*z),
        FrobeniusOperation::Identity(z) => Cospan::identity(&vec![*z]),
        // σ: [z, w] → [w, z]. Two apex vertices; the right leg reads them back
        // in the other order.
        FrobeniusOperation::SymmetricBraiding(z, w) => {
            Cospan::new_unchecked(vec![0, 1], vec![1, 0], vec![*z, *w])
        }
        // One apex vertex, every leg entry `0` — including at `(0, 0)`, where
        // that is the bubble.
        FrobeniusOperation::Spider(z, m, n) => {
            Cospan::new_unchecked(vec![0; *m], vec![0; *n], vec![*z])
        }
        FrobeniusOperation::UnSpecifiedBox(_, srcs, tgts) => {
            return Err(CatgraphError::Composition {
                message: format!(
                    "reference_generator: an UnSpecifiedBox on {} → {} wires has no cospan \
                     interpretation",
                    srcs.len(),
                    tgts.len()
                ),
            });
        }
    })
}

/// Interpret a `FrobeniusMorphism` as a `Cospan` — the **G1-T1** reading.
///
/// The retired `frobenius::operations::frobenius_to_cospan`: every layer is the
/// monoidal product of its blocks' generator cospans in block order, seeded from
/// [`Cospan::empty`] rather than from the identity on the morphism's domain, and
/// the layers are composed by pushout in order. A layerless morphism is the
/// empty cospan.
fn reference_to_cospan<Lambda, BlackBoxLabel>(
    morphism: &FrobeniusMorphism<Lambda, BlackBoxLabel>,
) -> Result<Cospan<Lambda>, CatgraphError>
where
    Lambda: Eq + Copy + Debug,
    BlackBoxLabel: Eq + Clone,
{
    let mut answer: Option<Cospan<Lambda>> = None;
    for layer in &morphism.layers {
        let mut current = Cospan::<Lambda>::empty();
        for block in &layer.blocks {
            current.monoidal(reference_generator(&block.op)?);
        }
        answer = Some(match answer {
            None => current,
            Some(previous) => Composable::compose(&previous, &current)?,
        });
    }
    Ok(answer.unwrap_or_else(Cospan::empty))
}

// ---------------------------------------------------------------------------
// The space
// ---------------------------------------------------------------------------

/// A one-line digest of a canonical form, for failure messages.
fn digest(c: &CospanCanon<char>) -> String {
    format!(
        "{}→{} apex={} scalars={}",
        c.dom_len(),
        c.cod_len(),
        c.apex_len(),
        c.scalar_count()
    )
}

/// The ten terms `tests/compact_closed.rs::samples()` ranges over.
///
/// Deliberately duplicated rather than shared: that file is an integration test
/// and cannot be imported from `src`. `compact_closed_samples_match_the_integration_file`
/// is not a thing this module can assert — the drift risk is accepted and named.
fn compact_closed_samples() -> Vec<(String, FM)> {
    let braid: FM = FrobeniusOperation::SymmetricBraiding('a', 'b').into();

    let mut delta_mu: FM = FrobeniusOperation::Comultiplication('a').into();
    delta_mu
        .compose(FrobeniusOperation::Multiplication('a').into())
        .expect("invariant: δ;μ interfaces match by construction");

    let mut mu_delta: FM = FrobeniusOperation::Multiplication('a').into();
    mu_delta
        .compose(FrobeniusOperation::Comultiplication('a').into())
        .expect("invariant: μ;δ interfaces match by construction");

    vec![
        ("id_a".to_string(), FM::identity(&vec!['a'])),
        ("id_ab".to_string(), FM::identity(&vec!['a', 'b'])),
        (
            "mu".to_string(),
            FrobeniusOperation::Multiplication('a').into(),
        ),
        (
            "delta".to_string(),
            FrobeniusOperation::Comultiplication('a').into(),
        ),
        ("eta".to_string(), FrobeniusOperation::Unit('a').into()),
        (
            "epsilon".to_string(),
            FrobeniusOperation::Counit('a').into(),
        ),
        ("braid_ab".to_string(), braid),
        ("delta_then_mu".to_string(), delta_mu),
        ("mu_then_delta".to_string(), mu_delta),
        (
            "spider_2_3".to_string(),
            FrobeniusOperation::Spider('a', 2, 3).into(),
        ),
    ]
}

/// The `(m, n) ≤ 5` spider grid — thirty-six terms, including the `(0, 0)`
/// bubble.
///
/// The bound is `5`, not `3`, for one reason: the survivor's spider arm recurses
/// into
/// [`special_frobenius_morphism`](crate::frobenius::special_frobenius_morphism),
/// whose `m.is_multiple_of(2)` doubling branch is reachable only at even
/// `m >= 4`. At `m <= 3` the one route through which the two implementations are
/// genuinely independent never exercises that branch, so the grid covers
/// `(4, n)` and `(5, n)` too. The random terms do not reach there —
/// [`random_generator`]'s spider arm caps both arities at `3`.
fn spider_grid() -> Vec<(String, FM)> {
    let mut out = Vec::new();
    for m in 0..=5usize {
        for n in 0..=5usize {
            out.push((
                format!("spider_{m}_{n}"),
                FrobeniusOperation::Spider('a', m, n).into(),
            ));
        }
    }
    out
}

/// Both sides of each of the eleven Def 2.5 equations, built exactly as
/// `tests/canonical.rs::equations` builds them on this carrier.
fn def_2_5_battery() -> Vec<(String, FM)> {
    const Z: char = 'z';
    let eta = || -> FM { FrobeniusOperation::Unit(Z).into() };
    let eps = || -> FM { FrobeniusOperation::Counit(Z).into() };
    let mu = || -> FM { FrobeniusOperation::Multiplication(Z).into() };
    let delta = || -> FM { FrobeniusOperation::Comultiplication(Z).into() };
    let id = || -> FM { FM::identity(&vec![Z]) };
    let sigma = || -> FM { FrobeniusOperation::SymmetricBraiding(Z, Z).into() };

    let par = |a: &FM, b: &FM| -> FM {
        let mut answer = a.clone();
        answer.monoidal(b.clone());
        answer
    };
    let seq = |a: &FM, b: &FM| -> FM {
        let mut answer = a.clone();
        ComposableMutating::compose(&mut answer, b.clone())
            .expect("invariant: the Def 2.5 composites are type-correct by hand");
        answer
    };

    let table: Vec<(&str, FM, FM)> = vec![
        (
            "assoc",
            seq(&par(&mu(), &id()), &mu()),
            seq(&par(&id(), &mu()), &mu()),
        ),
        ("left_unit", seq(&par(&eta(), &id()), &mu()), id()),
        ("right_unit", seq(&par(&id(), &eta()), &mu()), id()),
        ("comm", seq(&sigma(), &mu()), mu()),
        (
            "coassoc",
            seq(&delta(), &par(&delta(), &id())),
            seq(&delta(), &par(&id(), &delta())),
        ),
        ("left_counit", seq(&delta(), &par(&eps(), &id())), id()),
        ("right_counit", seq(&delta(), &par(&id(), &eps())), id()),
        ("cocomm", seq(&delta(), &sigma()), delta()),
        (
            "frob_left",
            seq(&par(&delta(), &id()), &par(&id(), &mu())),
            seq(&mu(), &delta()),
        ),
        (
            "frob_right",
            seq(&par(&id(), &delta()), &par(&mu(), &id())),
            seq(&mu(), &delta()),
        ),
        ("special", seq(&delta(), &mu()), id()),
    ];

    let mut out = Vec::new();
    for (label, lhs, rhs) in table {
        out.push((format!("{label}_lhs"), lhs));
        out.push((format!("{label}_rhs"), rhs));
    }
    out
}

/// Cup, cap, name and unname terms — the compact-closed shapes, whose images
/// carry bent legs no generator cospan has.
fn compact_closed_terms() -> Vec<(String, FM)> {
    let mut out: Vec<(String, FM)> = vec![
        ("cup_empty".to_string(), cup(&[])),
        ("cup_a".to_string(), cup(&['a'])),
        ("cup_ab".to_string(), cup(&['a', 'b'])),
        ("cup_aa".to_string(), cup(&['a', 'a'])),
        ("cap_empty".to_string(), cap(&[])),
        ("cap_a".to_string(), cap(&['a'])),
        ("cap_ab".to_string(), cap(&['a', 'b'])),
        ("cap_aa".to_string(), cap(&['a', 'a'])),
    ];

    let named: Vec<(&str, FM)> = vec![
        ("id_a", FM::identity(&vec!['a'])),
        ("id_ab", FM::identity(&vec!['a', 'b'])),
        ("mu", FrobeniusOperation::Multiplication('a').into()),
        ("eta", FrobeniusOperation::Unit('a').into()),
        ("eps", FrobeniusOperation::Counit('a').into()),
    ];
    for (label, f) in &named {
        out.push((
            format!("name_{label}"),
            name(f).expect("invariant: name is total on black-box-free terms"),
        ));
    }
    for (label, f, x_len) in [
        ("id_a", FM::identity(&vec!['a']), 1usize),
        ("mu", FrobeniusOperation::Multiplication('a').into(), 2usize),
    ] {
        let g = name(&f).expect("invariant: name is total on black-box-free terms");
        out.push((
            format!("unname_name_{label}"),
            unname(&g, x_len).expect("invariant: unname inverts name at the declared arity"),
        ));
    }
    out
}

/// One random extension step: pick a generator that fits somewhere in `cod` and
/// return `(position, wires consumed, the generator as a morphism)`.
///
/// Returns `None` when the drawn generator has no legal position, which the
/// caller treats as a skipped step — that keeps the draw uniform over generator
/// *kinds* rather than over (kind, position) pairs, and costs only term length.
/// [`MIN_RANDOM_DISTINCT`] is what keeps a regression in these guards from
/// silently emptying the random half of the space.
fn random_generator(rng: &mut StdRng, cod: &[char]) -> Option<(usize, usize, FM)> {
    let n = cod.len();
    // Positions where two adjacent wires carry the same label.
    let equal_pairs: Vec<usize> = (0..n.saturating_sub(1))
        .filter(|&i| cod[i] == cod[i + 1])
        .collect();

    match rng.random_range(0..7u8) {
        // η: consumes nothing, so any gap is a legal position.
        0 => {
            let i = rng.random_range(0..=n);
            let z = LABELS[rng.random_range(0..LABELS.len())];
            Some((i, 0, FrobeniusOperation::Unit(z).into()))
        }
        1 if n >= 1 => {
            let i = rng.random_range(0..n);
            Some((i, 1, FrobeniusOperation::Counit(cod[i]).into()))
        }
        2 if !equal_pairs.is_empty() => {
            let i = equal_pairs[rng.random_range(0..equal_pairs.len())];
            Some((i, 2, FrobeniusOperation::Multiplication(cod[i]).into()))
        }
        3 if n >= 1 => {
            let i = rng.random_range(0..n);
            Some((i, 1, FrobeniusOperation::Comultiplication(cod[i]).into()))
        }
        4 if n >= 2 => {
            let i = rng.random_range(0..n - 1);
            Some((
                i,
                2,
                FrobeniusOperation::SymmetricBraiding(cod[i], cod[i + 1]).into(),
            ))
        }
        5 if n >= 1 => {
            let i = rng.random_range(0..n);
            Some((i, 1, FrobeniusOperation::Identity(cod[i]).into()))
        }
        // A spider on a run of equal labels: `m` in `0..=3` inputs (all of the
        // same label), `k` in `0..=3` outputs.
        6 => {
            let i = rng.random_range(0..=n);
            let z = if i < n {
                cod[i]
            } else {
                LABELS[rng.random_range(0..LABELS.len())]
            };
            let run = cod[i..].iter().take_while(|&&c| c == z).count().min(3);
            let m = rng.random_range(0..=run);
            let k = rng.random_range(0..=3usize);
            Some((i, m, FrobeniusOperation::Spider(z, m, k).into()))
        }
        _ => None,
    }
}

/// A random black-box-free `FrobeniusMorphism` over `LABELS`.
///
/// Grown by repeatedly composing `id ⊗ generator ⊗ id` onto the running term,
/// so every intermediate is type-correct by construction and the term passes
/// through `two_layer_simplify` exactly as a caller-built term would.
fn random_term(rng: &mut StdRng, steps: usize) -> FM {
    let width = rng.random_range(0..3usize);
    let start: Vec<char> = (0..width)
        .map(|_| LABELS[rng.random_range(0..LABELS.len())])
        .collect();
    let mut term: FM = FM::identity(&start);

    for _ in 0..steps {
        let cod = ComposableMutating::codomain(&term);
        let Some((i, consumed, generator)) = random_generator(rng, &cod) else {
            continue;
        };
        let mut layer: FM = FM::identity(&cod[..i].to_vec());
        layer.monoidal(generator);
        layer.monoidal(FM::identity(&cod[i + consumed..].to_vec()));
        ComposableMutating::compose(&mut term, layer)
            .expect("invariant: the layer was built on the term's own codomain");
    }
    term
}

/// The whole space this module's differential pin ranges over.
fn space() -> Vec<(String, FM)> {
    let mut out = compact_closed_samples();
    out.extend(spider_grid());
    out.extend(def_2_5_battery());
    out.extend(compact_closed_terms());

    let mut rng = StdRng::seed_from_u64(0x0336_0001);
    for k in 0..RANDOM_TERMS {
        let steps = 1 + (k % 8);
        out.push((format!("random_{k}"), random_term(&mut rng, steps)));
    }
    out
}

// ---------------------------------------------------------------------------
// The pin
// ---------------------------------------------------------------------------

/// Compares `cospan_algebra::frobenius_to_cospan`, reached through the
/// `frobenius::` re-export, against [`reference_to_cospan`] up to
/// `canonical_form`.
///
/// **Fixture (383 terms):** the ten `tests/compact_closed.rs::samples()` terms;
/// the thirty-six `(m, n) ≤ 5` spiders including the `(0, 0)` bubble; both
/// sides of all eleven Def 2.5 equations (22 terms); fifteen cup / cap / name /
/// unname terms; and 300 pseudo-random terms of up to 8 extension attempts over
/// two labels, seeded at `0x0336_0001`. All at `Lambda = char`,
/// `BlackBoxLabel = String` — one instantiation. No term here carries a black
/// box; that arm is [`black_boxes_are_rejected_by_both`].
///
/// **Expected:** no survivor/oracle mismatch, `terms.len() == SPACE_SIZE`, and
/// the two distinct-form floors. Measured on the pinned seed: 212 distinct
/// canonical forms over the 383 terms and 175 over the 300 random ones; 46 of
/// the random terms have `0 → 0` images, spread over 6 distinct scalar-shaped
/// forms.
///
/// **What it cannot see:** both sides fold with the same `Cospan::compose`,
/// `Monoidal` and `HypergraphCategory` generator cospans, so this is a
/// *differential* claim about the two interpretation functions, not an absolute
/// one about the cospan machinery underneath them. The oracle's six arms other
/// than `Spider` and `UnSpecifiedBox` are byte-identical to the survivor's, the
/// braiding's hand-written
/// `Cospan::new_unchecked(vec![0, 1], vec![1, 0], vec![z, w])` literal
/// included, so a convention error applied to both copies is not something this
/// pin can compare away. Measured with the right leg flipped to `[0, 1]` in
/// **both** copies: this test goes red at `random_5` through the fold's label
/// check, while the same pin restricted to the spider grid plus the Def 2.5
/// battery (58 terms) has 0 mismatches.
#[test]
fn the_two_frobenius_to_cospan_agree_over_the_wide_space() {
    let terms = space();
    assert_eq!(
        terms.len(),
        SPACE_SIZE,
        "the differential space changed size without SPACE_SIZE following it"
    );

    let mut mismatches: Vec<String> = Vec::new();
    let mut all_images: HashSet<CospanCanon<char>> = HashSet::new();
    let mut random_images: HashSet<CospanCanon<char>> = HashSet::new();
    for (label, term) in &terms {
        let survivor = survivor_to_cospan(term)
            .unwrap_or_else(|e| panic!("{label}: the survivor rejected a black-box-free term: {e}"))
            .canonical_form();
        let reference = reference_to_cospan(term)
            .unwrap_or_else(|e| {
                panic!("{label}: the reference rejected a black-box-free term: {e}")
            })
            .canonical_form();
        all_images.insert(survivor.clone());
        if label.starts_with("random_") {
            random_images.insert(survivor.clone());
        }
        if survivor != reference {
            mismatches.push(format!(
                "  {label}: survivor {} vs G1-T1 reference {}\n    survivor classes:  {:?}\n    \
                 reference classes: {:?}",
                digest(&survivor),
                digest(&reference),
                survivor.classes(),
                reference.classes(),
            ));
        }
    }

    assert!(
        mismatches.is_empty(),
        "{} of {} terms disagree between the surviving frobenius_to_cospan and the retired \
         G1-T1 algorithm:\n{}",
        mismatches.len(),
        terms.len(),
        mismatches.join("\n"),
    );

    // The space's *content*, not just its count. Agreement over 383 copies of
    // the identity would be agreement about nothing.
    assert!(
        random_images.len() >= MIN_RANDOM_DISTINCT,
        "the random half of the space collapsed: {} distinct canonical forms over {} random \
         terms, floor {} (measured 175 on this tree; 172 when this pin was written, before \
         #350 stopped cancelling η;ε)",
        random_images.len(),
        RANDOM_TERMS,
        MIN_RANDOM_DISTINCT,
    );
    assert!(
        all_images.len() >= MIN_TOTAL_DISTINCT,
        "the space collapsed: {} distinct canonical forms over {} terms, floor {} (measured 212 \
         on this tree; 209 when this pin was written, before #350 stopped cancelling η;ε)",
        all_images.len(),
        terms.len(),
        MIN_TOTAL_DISTINCT,
    );
}

/// Fixture: one `UnSpecifiedBox`, `1 → 2` wires, at `char`/`String`, through
/// both readings.
///
/// Expected: the reference returns [`CatgraphError::Composition`] and the
/// survivor [`CatgraphError::Interpret`], whose message names both the
/// generator and the arities as `N in, M out`. It says nothing about a black
/// box nested inside a larger term.
#[test]
fn black_boxes_are_rejected_by_both() {
    let boxed: FM =
        FrobeniusOperation::UnSpecifiedBox("f".to_string(), vec!['a'], vec!['b', 'b']).into();

    let reference = reference_to_cospan(&boxed)
        .expect_err("the G1-T1 reference gives a black box no interpretation");
    assert!(
        matches!(reference, CatgraphError::Composition { .. }),
        "G1-T1 rejected with Composition; got {reference:?}"
    );

    let survivor = survivor_to_cospan(&boxed).expect_err("a black box denotes nothing");
    assert!(
        matches!(survivor, CatgraphError::Interpret { .. }),
        "the survivor's documented variant is Interpret; got {survivor:?}"
    );
    let rendered = format!("{survivor}");
    assert!(
        rendered.contains("UnSpecifiedBox"),
        "the merged wording keeps G1-T1's name for the generator; got: {rendered}"
    );
    assert!(
        rendered.contains("1 in, 2 out"),
        "the merged wording keeps G1-T2's arity rendering; got: {rendered}"
    );
}
