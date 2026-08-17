//! Container-law acceptance tests for the four shipped endofunctor witnesses
//! (issue #41).
//!
//! Round-trip, arity coherence (including recompose rejection of a length ≠
//! arity), `fmap` coherence and borrow coherence (Abbott–Altenkirch–Ghani 2003,
//! via CDL) for every shape of `ListEndo` (`None` / `Some`), `TreeEndo`
//! (`Left` leaf / `Right` node), `GroupActionEndo` (both `Z2` group elements)
//! and `OptionWitness` (`None` / `Some`). Each feeds per-witness sample values
//! into the single generic [`common::assert_container_laws`] helper.

mod common;

use common::assert_container_laws;

use catgraph_dl::Either;
use catgraph_dl::algebra::{GroupActionEndo, Z2Group};
use catgraph_dl::endofunctor::OptionWitness;
use catgraph_dl::free_monad::list_endo::ListEndo;
use catgraph_dl::free_monad::tree_endo::TreeEndo;

#[test]
fn list_endo_container_laws() {
    // `1 + A × −`: `None` shape (arity 0) and `Some` shape (arity 1).
    assert_container_laws::<ListEndo<i32>>(None);
    assert_container_laws::<ListEndo<i32>>(Some((7, 42)));
}

#[test]
fn tree_endo_container_laws() {
    // `A + (−)²`: `Left` leaf shape (arity 0) and `Right` node shape (arity 2).
    assert_container_laws::<TreeEndo<i32>>(Either::Left(9));
    assert_container_laws::<TreeEndo<i32>>(Either::Right((3, 4)));
}

#[test]
fn group_action_endo_container_laws() {
    // `G × −`: single shape per group element, all arity 1.
    assert_container_laws::<GroupActionEndo<Z2Group>>((Z2Group(false), 5));
    assert_container_laws::<GroupActionEndo<Z2Group>>((Z2Group(true), -5));
}

/// `1 + −`, the stock witness — added in the v0.14.0 window and load-bearing
/// from the day it landed: `Cofree<OptionWitness, O>::unfold` routes through
/// this `decompose`/`recompose` pair, and the carrier's `==` / `{:?}` through
/// `contents`. It shipped with no law coverage at all while the README listed
/// it among the machine-checked instances.
#[test]
fn option_witness_container_laws() {
    // `Shape = bool`: `false` is the unit summand (arity 0), `true` the
    // singleton (arity 1 — the one recursive slot).
    assert_container_laws::<OptionWitness>(None);
    assert_container_laws::<OptionWitness>(Some(7));
}
