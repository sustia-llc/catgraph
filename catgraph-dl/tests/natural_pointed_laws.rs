//! Law acceptance tests for `NaturalTransformation` and `Pointed` (issue #41).
//!
//! Covers three surfaces:
//!
//! - The blanket [`Pointed`] instances — the crate's own
//!   `GroupActionEndo<Z2Group>` and haft's `OptionWitness` (pointed via its
//!   upstream `Pure`, reachable through the seam) — σ-naturality
//!   `fmap(pure(x), f) == pure(f(x))` (CDL Def B.3).
//! - The iso adapters [`IsoForward`] / [`IsoBackward`] over a genuine
//!   cross-witness [`NaturalIso`]: `ListEndo<()>` (`Option<((), X)>`) ≅
//!   `OptionWitness` (`Option<X>`). Both NT directions are exercised, and
//!   haft's own natural-iso law helpers (re-exported through the crate seam)
//!   confirm the underlying iso.
//! - A hand-written [`NaturalTransformation`] that is **not** iso-derived —
//!   `ListEndo<i32> ⇒ ListEndo<i64>` widening the shape label — to show the
//!   trait stands alone (Gavranović et al., ICML 2024, Def 1.5).

mod common;

use common::{assert_natural_transformation_naturality, assert_pointed_naturality};

use catgraph_dl::algebra::{GroupActionEndo, Z2Group};
use catgraph_dl::endofunctor::{
    NaturalIso, OptionWitness, assert_natural_iso_naturality, assert_natural_iso_round_trip,
};
use catgraph_dl::free_monad::list_endo::ListEndo;
use catgraph_dl::{IsoBackward, IsoForward, NaturalTransformation, NoConstraint, Satisfies};

/// A genuine cross-witness natural isomorphism `ListEndo<()> ≅ OptionWitness`:
/// `Option<((), X)> ≅ Option<X>` — the unit label carries no information, so
/// pairing with `()` and dropping it are mutually inverse and natural in `X`.
struct ListUnitOptionIso;

impl NaturalIso<ListEndo<()>, OptionWitness> for ListUnitOptionIso {
    fn to_target<T>(fa: Option<((), T)>) -> Option<T>
    where
        T: Satisfies<NoConstraint>,
    {
        fa.map(|((), x)| x)
    }

    fn to_source<T>(ga: Option<T>) -> Option<((), T)>
    where
        T: Satisfies<NoConstraint>,
    {
        ga.map(|x| ((), x))
    }
}

/// A hand-written, non-iso-derived natural transformation `ListEndo<i32> ⇒
/// ListEndo<i64>`: widen the shape label `i32 → i64`, leave the recursive slot
/// untouched. Natural because the widening acts on the shape only, commuting
/// with `fmap` (which acts on the slot only).
struct ListWiden;

impl NaturalTransformation<ListEndo<i32>, ListEndo<i64>> for ListWiden {
    fn transform<T>(fa: Option<(i32, T)>) -> Option<(i64, T)> {
        fa.map(|(a, x)| (i64::from(a), x))
    }
}

#[test]
fn group_action_endo_pointed_sigma_naturality() {
    // GroupActionEndo<Z2Group> is the crate's own Pointed inhabitant;
    // check σ-naturality across a spread of samples including a wrapping edge.
    for x in [0_i32, 5, -7, 42, i32::MAX] {
        assert_pointed_naturality::<GroupActionEndo<Z2Group>>(x);
    }
}

#[test]
fn option_witness_pointed_sigma_naturality() {
    // haft's OptionWitness reaches Pointed through the same blanket impl via
    // its upstream Pure (`pure(x) = Some(x)`); law-check it like any other
    // inhabitant so every publicly reachable Pointed witness is covered.
    for x in [0_i32, 5, -7, i32::MAX] {
        assert_pointed_naturality::<OptionWitness>(x);
    }
}

#[test]
fn iso_adapters_natural_transformation_both_directions() {
    // Forward: ListEndo<()> ⇒ OptionWitness (to_target leg).
    assert_natural_transformation_naturality::<
        IsoForward<ListUnitOptionIso>,
        ListEndo<()>,
        OptionWitness,
    >(Some(((), 5)));
    assert_natural_transformation_naturality::<
        IsoForward<ListUnitOptionIso>,
        ListEndo<()>,
        OptionWitness,
    >(None);

    // Backward: OptionWitness ⇒ ListEndo<()> (to_source leg).
    assert_natural_transformation_naturality::<
        IsoBackward<ListUnitOptionIso>,
        OptionWitness,
        ListEndo<()>,
    >(Some(7));
    assert_natural_transformation_naturality::<
        IsoBackward<ListUnitOptionIso>,
        OptionWitness,
        ListEndo<()>,
    >(None);
}

#[test]
fn list_unit_option_iso_haft_laws_via_seam() {
    // haft's own round-trip + naturality helpers, reached through the crate
    // seam's re-exports — proving the seam surfaces them correctly. Independent
    // `fa` / `ga` inputs so a non-bijective witness cannot slip through.
    assert_natural_iso_round_trip::<ListUnitOptionIso, ListEndo<()>, OptionWitness, i32>(
        Some(((), 3)),
        Some(9),
    );
    assert_natural_iso_round_trip::<ListUnitOptionIso, ListEndo<()>, OptionWitness, i32>(
        None, None,
    );
    assert_natural_iso_naturality::<ListUnitOptionIso, ListEndo<()>, OptionWitness, i32, i32, _>(
        Some(((), 4)),
        |v: i32| v.wrapping_add(3),
    );
}

#[test]
fn list_widen_standalone_natural_transformation() {
    // A non-iso-derived NT still satisfies naturality through the same helper.
    assert_natural_transformation_naturality::<ListWiden, ListEndo<i32>, ListEndo<i64>>(Some((
        10, 5,
    )));
    assert_natural_transformation_naturality::<ListWiden, ListEndo<i32>, ListEndo<i64>>(None);
}
