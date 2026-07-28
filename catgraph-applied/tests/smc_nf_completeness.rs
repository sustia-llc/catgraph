//! Layer 1 NF completeness verification.
//!
//! Three complementary checks:
//!
//! 1. **SMC-axiom closure** — for each Mac Lane / Joyal-Street axiom,
//!    `nf(lhs) == nf(rhs)` over bounded-random inputs (proptest).
//! 2. **Idempotence** — `nf(e) = nf(nf-round-trip(e))` i.e. applying `nf`
//!    after building a `PropExpr` from the NF reaches the same fixpoint.
//!    Approximated by running `nf` on the result expression-ified (can't
//!    literally re-run `nf` because we don't have a `StringDiagram →
//!    PropExpr` unparser; we instead do an in-Rust check that
//!    canonicalization is stable under re-invocation of the individual
//!    steps).
//! 3. **Phase A golden-replay** — for every witness pair in the Phase A
//!    corpus at `/tmp/v052_witnesses_<rig>_2.json`, `nf(lhs) == nf(rhs)`.
//!    All Phase A witnesses are matrix-equal under `sfg_to_mat`, so any
//!    non-equal NF is a C1 bug.
//!
//! The golden-replay test is gated behind `#[ignore]` because the corpora
//! are ~3 MB each and only exist on the developer's machine; release-gate
//! reviewers run it manually with `--ignored`.
//!
//! **Zero-arity atoms.** `try_unitor_merge` absorbs the 2-atom sink/source
//! pattern (`[X, Identity(k)]` and three mirrors); mid-layer zero-source `η`
//! deeper in a layer is scheduled by the `topological_layer_order`
//! point-span sift (issue #55, closed on the fragment 𝔉 —
//! SMC-NF-RECONCILIATION.md §4.1; probe-verified, full proof open per the
//! §4.4 canonicality status; see `interchange_zero_source_eta` and the
//! `smc_canonicality_probes` module). A proptest or golden-replay failure
//! whose witness has an `η` in an *interleaved* component (guard 3) is the one
//! documented residual left, **(a)** — see
//! `marked_encloser_blocks_the_column_move`.
//!
//! Three of the four *named* residuals are closed — and "named" is the load
//! bearing word: §4.6 is a ledger, not a bound. A differential sweep still
//! finds SMC-equal pairs inside `𝔉` whose normal forms differ, outside all four
//! letters and mostly predating this machinery, so this module is the
//! canonicality gate by virtue of the convergences it *names*, not by any claim
//! about what is left. **(b)** — two *distinct* closed
//! blocks kept their input order because rule (i) gives every closed component
//! one key and `PropSignature` carried no `Ord` to break the tie — closed by
//! issue #79 P1: the `Ord` supertrait plus Step 7's in-situ reading key
//! (`closed_blocks_sort_by_content_key` and its companions below). **(c)** (a
//! closed component written nested inside another component's span) and
//! **(d)** (a zero-arity block solid on its opening side, written nested) are
//! closed by the Step 6½ zero-arity column pass (issue #174); their former
//! `#[ignore]`d witnesses — `trapped_closed_block_extracts`,
//! `nested_sink_block_converges_with_free_writing`,
//! `nested_source_block_converges_with_free_writing` — are live regressions below.

use catgraph_applied::prop::presentation::smc_nf::{from_string_diagram, nf};
use catgraph_applied::prop::{PropExpr, PropSignature, mono_word};
use proptest::prelude::*;
use std::borrow::Cow;

/// Test signature. `Sc(u8)` is a `0 → 0` **scalar** — the shape no *shipped*
/// signature has (Mat(R)/SFG scalars are `1 → 1`, `FrobeniusOr`'s η/ε are
/// `0 → 1` / `1 → 0`), and the only one that exercises Step 6's `G::cmp`
/// tie-break at an equal zero-arity class (issue #79 P1). Its `u8` payload
/// gives the derived `Ord` something to order.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum TestSig {
    F,      // 1 → 1
    G,      // 1 → 1
    Eps,    // 1 → 0 (sink)
    Eta,    // 0 → 1 (source)
    Eps2,   // 2 → 0 (wide sink)
    Eta2,   // 0 → 2 (wide source)
    Sc(u8), // 0 → 0 (scalar)
}

impl PropSignature for TestSig {
    type Color = ();

    fn source_word(&self) -> Cow<'_, [()]> {
        mono_word(self.source())
    }
    fn target_word(&self) -> Cow<'_, [()]> {
        mono_word(self.target())
    }
    fn source(&self) -> usize {
        match self {
            TestSig::F | TestSig::G | TestSig::Eps => 1,
            TestSig::Eta | TestSig::Eta2 | TestSig::Sc(_) => 0,
            TestSig::Eps2 => 2,
        }
    }
    fn target(&self) -> usize {
        match self {
            TestSig::F | TestSig::G | TestSig::Eta => 1,
            TestSig::Eps | TestSig::Eps2 | TestSig::Sc(_) => 0,
            TestSig::Eta2 => 2,
        }
    }
}

// ============================================================================
// 1. SMC-axiom closure (proptest)
// ============================================================================

mod axiom_closure {
    use super::*;

    /// Generate a `PropExpr<TestSig>` of bounded depth. Wire arities are
    /// kept low (≤ 4) to keep the enumeration tractable.
    fn arb_expr() -> impl Strategy<Value = PropExpr<TestSig>> {
        let leaf = prop_oneof![
            (1u32..=3u32).prop_map(|n| PropExpr::Identity(n as usize)),
            Just(PropExpr::Braid(1, 1)),
            Just(PropExpr::Generator(TestSig::F)),
            Just(PropExpr::Generator(TestSig::G)),
        ];
        // Recursive strategy with depth limit 3 — keeps each test case fast.
        leaf.prop_recursive(3, 32, 4, |inner| {
            prop_oneof![
                // Compose: arity compatibility enforced via try/retry below.
                (inner.clone(), inner.clone()).prop_filter_map("Compose arity match", |(a, b)| {
                    if a.target() == b.source() {
                        Some(PropExpr::Compose(Box::new(a), Box::new(b)))
                    } else {
                        None
                    }
                }),
                // Tensor: always well-typed.
                (inner.clone(), inner)
                    .prop_map(|(a, b)| { PropExpr::Tensor(Box::new(a), Box::new(b)) }),
            ]
        })
    }

    proptest! {
        // Three-way arity compatibility (`a.target() == b.source()` AND
        // `b.target() == c.source()` for the compose_associator test) is
        // rejected aggressively by `arb_expr`'s bounded-arity leaf set.
        // Bump `max_global_rejects` from the default 1024 → 16_384 so the
        // test is stable even when the generator happens to produce a bad
        // batch of incompatible arities.
        #![proptest_config(ProptestConfig { cases: 64, max_global_rejects: 16_384, .. ProptestConfig::default() })]

        /// Associativity of compose: `(f ; g) ; h  =  f ; (g ; h)`.
        /// JS-I Ch 1 Prop 1.1.
        #[test]
        fn compose_associator(
            a in arb_expr(),
            b in arb_expr(),
            c in arb_expr(),
        ) {
            prop_assume!(a.target() == b.source());
            prop_assume!(b.target() == c.source());
            let lhs = PropExpr::Compose(
                Box::new(PropExpr::Compose(Box::new(a.clone()), Box::new(b.clone()))),
                Box::new(c.clone()),
            );
            let rhs = PropExpr::Compose(
                Box::new(a),
                Box::new(PropExpr::Compose(Box::new(b), Box::new(c))),
            );
            prop_assert_eq!(nf(&lhs), nf(&rhs));
        }

        /// Associativity of tensor: `(f ⊗ g) ⊗ h  =  f ⊗ (g ⊗ h)`.
        /// JS-I Ch 1 §4.
        #[test]
        fn tensor_associator(
            a in arb_expr(),
            b in arb_expr(),
            c in arb_expr(),
        ) {
            let lhs = PropExpr::Tensor(
                Box::new(PropExpr::Tensor(Box::new(a.clone()), Box::new(b.clone()))),
                Box::new(c.clone()),
            );
            let rhs = PropExpr::Tensor(
                Box::new(a),
                Box::new(PropExpr::Tensor(Box::new(b), Box::new(c))),
            );
            prop_assert_eq!(nf(&lhs), nf(&rhs));
        }

        /// Left / right identity for compose: `id ; f = f  =  f ; id`.
        #[test]
        fn compose_unitors(f in arb_expr()) {
            let id_src = PropExpr::<TestSig>::Identity(f.source());
            let id_tgt = PropExpr::<TestSig>::Identity(f.target());
            let left = PropExpr::Compose(Box::new(id_src), Box::new(f.clone()));
            let right = PropExpr::Compose(Box::new(f.clone()), Box::new(id_tgt));
            prop_assert_eq!(nf(&left), nf(&f));
            prop_assert_eq!(nf(&right), nf(&f));
        }

        /// Tensor unitors: `id_0 ⊗ f = f = f ⊗ id_0`. JS-I Ch 1 §1.
        #[test]
        fn tensor_unitors(f in arb_expr()) {
            let id0 = PropExpr::<TestSig>::Identity(0);
            let left = PropExpr::Tensor(Box::new(id0.clone()), Box::new(f.clone()));
            let right = PropExpr::Tensor(Box::new(f.clone()), Box::new(id0));
            prop_assert_eq!(nf(&left), nf(&f));
            prop_assert_eq!(nf(&right), nf(&f));
        }

        /// Bifunctoriality / interchange: `(f ⊗ g) ; (h ⊗ k) = (f ; h) ⊗ (g ; k)`
        /// when arities align. JS-I Ch 1 §4 Thm 1.2 p.71.
        ///
        /// **C2 gap (2026-04-23) — closed (issue #14).** Two coordinated
        /// mechanisms canonicalize both sides onto one normal form:
        ///
        /// - **Generator scheduling** — `topological_layer_order` (Step 4(c))
        ///   sifts each non-identity-source generator to its earliest admissible
        ///   (braid-free) layer, so schedulings like `[id_2, F]; [F, id_1, F];
        ///   [id_2, F]` vs `[F, id_1, F]; [id_2, F]; [id_2, F]` converge.
        /// - **Braids in mixed layers** — a `Braid(1,1)` co-resident with an
        ///   unrelated generator (e.g. `[σ, F]`) is freed by
        ///   `isolate_mixed_braid_layers` (bifunctoriality factorization into a
        ///   braid-only layer + a generator layer) so `collect_braid_prefix`'s
        ///   naturality sweep — now identity-width-refined — can slide it to the
        ///   leading layers; `try_column_merge` never re-creates a mixed layer.
        ///
        /// See the `issue_14_topological_layer_order` regressions in
        /// `smc_nf_regression.rs`.
        ///
        /// The follow-up mid-layer **zero-source** case (`η : 0 → 1`) is now also
        /// closed by the point-span sift (issue #55) — see
        /// `interchange_zero_source_eta` below. This proptest's `arb_expr` emits
        /// only `F`, `G : 1 → 1` plus braids/identities, so it exercises the
        /// positive-source scheduling directly.
        #[test]
        fn interchange(
            f in arb_expr(),
            g in arb_expr(),
            h in arb_expr(),
            k in arb_expr(),
        ) {
            prop_assume!(f.target() == h.source());
            prop_assume!(g.target() == k.source());
            let lhs = PropExpr::Compose(
                Box::new(PropExpr::Tensor(Box::new(f.clone()), Box::new(g.clone()))),
                Box::new(PropExpr::Tensor(Box::new(h.clone()), Box::new(k.clone()))),
            );
            let rhs = PropExpr::Tensor(
                Box::new(PropExpr::Compose(Box::new(f), Box::new(h))),
                Box::new(PropExpr::Compose(Box::new(g), Box::new(k))),
            );
            prop_assert_eq!(nf(&lhs), nf(&rhs));
        }

        /// Idempotence via a full `nf` re-run on a fresh `PropExpr`. The
        /// expression-based re-run sidesteps the absence of a `StringDiagram
        /// → PropExpr` unparser.
        #[test]
        fn idempotence_on_compose(a in arb_expr(), b in arb_expr()) {
            prop_assume!(a.target() == b.source());
            let e = PropExpr::Compose(Box::new(a), Box::new(b));
            let once = nf(&e);
            let twice = nf(&e);
            prop_assert_eq!(once, twice);
        }
    }
}

// ============================================================================
// 2. Known-edge-case proptest regression
// ============================================================================

/// Standalone regression for the `try_unitor_merge` 2-atom sink/source
/// pattern. If proptest ever produces a counterexample outside this pattern
/// shape, the existing `try_unitor_merge` will need extending.
///
/// Pattern being exercised: `(ε ⊗ id_k) ; L2` and three mirrors.
#[test]
fn known_edge_case_unitor_merge_two_atom_pattern() {
    let eps = PropExpr::Generator(TestSig::Eps); // 1 → 0
    let eta = PropExpr::Generator(TestSig::Eta); // 0 → 1
    let f: PropExpr<TestSig> = PropExpr::Generator(TestSig::F);

    // ε on left + identity bridge + next layer.
    let lhs_a = PropExpr::Compose(
        Box::new(PropExpr::Tensor(
            Box::new(eps.clone()),
            Box::new(PropExpr::Identity(1)),
        )),
        Box::new(f.clone()),
    );
    let rhs_a = PropExpr::Tensor(Box::new(eps), Box::new(f.clone()));
    assert_eq!(nf(&lhs_a), nf(&rhs_a), "ε-sink-left absorption");

    // η on right + identity bridge + previous layer.
    let lhs_b = PropExpr::Compose(
        Box::new(f.clone()),
        Box::new(PropExpr::Tensor(
            Box::new(PropExpr::Identity(1)),
            Box::new(eta.clone()),
        )),
    );
    let rhs_b = PropExpr::Tensor(Box::new(f), Box::new(eta));
    assert_eq!(nf(&lhs_b), nf(&rhs_b), "η-source-right absorption");
}

/// Closed (issue #55): a mid-layer **zero-source** generator (`η : 0 → 1`) is
/// now scheduled canonically by the point-span sift.
/// `F ⊗ η ⊗ G` and `(F ⊗ G) ; (id₁ ⊗ η ⊗ id₁)` are SMC-equal (both are
/// `[F(in0), η-fresh, G(in1)]`); `topological_layer_order` slides `η` into the
/// earlier layer at its point coordinate (the boundary between `F`'s and `G`'s
/// target spans), so both normalize to `[[F, η, G]]`.
#[test]
fn interchange_zero_source_eta() {
    let f: PropExpr<TestSig> = PropExpr::Generator(TestSig::F);
    let g: PropExpr<TestSig> = PropExpr::Generator(TestSig::G);
    let eta: PropExpr<TestSig> = PropExpr::Generator(TestSig::Eta); // 0 → 1

    // F ⊗ η ⊗ G  :  2 → 3
    let lhs = PropExpr::Tensor(
        Box::new(f.clone()),
        Box::new(PropExpr::Tensor(Box::new(eta.clone()), Box::new(g.clone()))),
    );
    // (F ⊗ G) ; (id₁ ⊗ η ⊗ id₁)  :  2 → 3
    let rhs = PropExpr::Compose(
        Box::new(PropExpr::Tensor(Box::new(f), Box::new(g))),
        Box::new(PropExpr::Tensor(
            Box::new(PropExpr::Identity(1)),
            Box::new(PropExpr::Tensor(
                Box::new(eta),
                Box::new(PropExpr::Identity(1)),
            )),
        )),
    );
    assert_eq!(nf(&lhs), nf(&rhs));
}

// ============================================================================
// 3. Within-layer zero-arity order (issue #55 PR1, Step 6)
// ============================================================================

/// Step 6 (`reorder_tied_zero_arity`) canonicalizes the within-layer order of
/// strictly-commuting zero-arity atoms to `scalar < η < ε < solid` — issue #55
/// Decision 1 (η-before-ε), design of record 2026-07-25.
///
/// `TestSig::Eps` (`1 → 0`) is the ε-class witness (SFG's `Discard`);
/// `TestSig::Eta` (`0 → 1`) is the η-class witness (SFG's `Zero`).
///
/// The **layer-assignment** half of #55 (`ε ; η` compose-forms vs `ε ⊗ η`
/// tensor-forms) is closed too, by the point-span sift
/// (PR2) — see `interchange_zero_source_eta` above and
/// `compose_form_converges_with_tensor_forms` below.
mod zero_arity_order {
    use super::*;
    use catgraph_applied::prop::presentation::smc_nf::{Atom, Layer, StringDiagram};

    fn eps() -> PropExpr<TestSig> {
        PropExpr::Generator(TestSig::Eps)
    }
    fn eta() -> PropExpr<TestSig> {
        PropExpr::Generator(TestSig::Eta)
    }

    /// The #55 witness pair: `ε ⊗ η` and `η ⊗ ε` are the same morphism `1 → 1`
    /// (both connecting braids are `σ_{0,n} = id`) and now share one NF, with
    /// the η first.
    #[test]
    fn tied_eta_eps_pair_converges_eta_first() {
        let lhs = PropExpr::Tensor(Box::new(eps()), Box::new(eta())); // Tensor(Discard, Zero)
        let rhs = PropExpr::Tensor(Box::new(eta()), Box::new(eps())); // Tensor(Zero, Discard)
        assert_eq!(nf(&lhs), nf(&rhs), "#55 within-layer split must close");

        let expected = StringDiagram {
            layers: vec![Layer {
                atoms: vec![Atom::Generator(TestSig::Eta), Atom::Generator(TestSig::Eps)],
            }],
        };
        assert_eq!(nf(&lhs), expected, "canonical order is η before ε");
    }

    /// The full #55 witness class converges: the **compose**-form `ε ; η`
    /// (`1 → 0 → 1`) is the same morphism `1 → 1` as either tensor-form, and the
    /// point-span sift (PR2) lifts its `η` into the `ε`'s layer. The resulting
    /// tie is decided by Decision 1 / Step 6 (PR1) — η first — as every tie now
    /// is: since #174 the tied comparator reads the two atoms' classes and
    /// nothing else, so no carve is needed to route this pair. All three share
    /// the one NF `[[η, ε]]`.
    #[test]
    fn compose_form_converges_with_tensor_forms() {
        let compose_form = PropExpr::Compose(Box::new(eps()), Box::new(eta()));
        let tensor_eta_first = PropExpr::Tensor(Box::new(eta()), Box::new(eps()));
        let tensor_eps_first = PropExpr::Tensor(Box::new(eps()), Box::new(eta()));

        let expected = StringDiagram {
            layers: vec![Layer {
                atoms: vec![Atom::Generator(TestSig::Eta), Atom::Generator(TestSig::Eps)],
            }],
        };
        assert_eq!(nf(&compose_form), expected, "ε ; η must sift to [[η, ε]]");
        assert_eq!(
            nf(&compose_form),
            nf(&tensor_eta_first),
            "#55 layer-assignment half must close (η ⊗ ε)"
        );
        assert_eq!(
            nf(&compose_form),
            nf(&tensor_eps_first),
            "#55 layer-assignment half must close (ε ⊗ η)"
        );
    }

    /// An η bubbles left past a whole run of ε's: `ε ⊗ ε' ⊗ η` → `η ⊗ ε ⊗ ε'`.
    /// This is the case that forces the termination measure to count *all*
    /// class-inverted pairs, not just adjacent ones — the first swap trades one
    /// adjacent inversion for another.
    #[test]
    fn eta_bubbles_left_past_an_eps_run() {
        let e = PropExpr::Tensor(
            Box::new(eps()),
            Box::new(PropExpr::Tensor(
                Box::new(PropExpr::Generator(TestSig::Eps2)),
                Box::new(eta()),
            )),
        );
        let expected = StringDiagram {
            layers: vec![Layer {
                atoms: vec![
                    Atom::Generator(TestSig::Eta),
                    Atom::Generator(TestSig::Eps),
                    Atom::Generator(TestSig::Eps2),
                ],
            }],
        };
        assert_eq!(nf(&e), expected);
    }

    /// η's never strictly commute with η's (their targets are both non-empty),
    /// so two distinct-arity η's keep their relative order — and the two
    /// orderings stay distinct morphisms.
    #[test]
    fn eta_eta_relative_order_is_preserved() {
        let a = PropExpr::Tensor(
            Box::new(eta()),
            Box::new(PropExpr::Generator(TestSig::Eta2)),
        );
        let b = PropExpr::Tensor(
            Box::new(PropExpr::Generator(TestSig::Eta2)),
            Box::new(eta()),
        );
        let atoms = |sd: &StringDiagram<TestSig>| sd.layers[0].atoms.clone();
        assert_eq!(
            atoms(&nf(&a)),
            vec![
                Atom::Generator(TestSig::Eta),
                Atom::Generator(TestSig::Eta2)
            ]
        );
        assert_eq!(
            atoms(&nf(&b)),
            vec![
                Atom::Generator(TestSig::Eta2),
                Atom::Generator(TestSig::Eta)
            ]
        );
        assert_ne!(nf(&a), nf(&b), "η ⊗ η' and η' ⊗ η are different morphisms");
    }

    /// Likewise for ε's: their sources are both non-empty, so they never
    /// strictly commute and their relative order is coordinate-determined.
    #[test]
    fn eps_eps_relative_order_is_preserved() {
        let a = PropExpr::Tensor(
            Box::new(eps()),
            Box::new(PropExpr::Generator(TestSig::Eps2)),
        );
        let b = PropExpr::Tensor(
            Box::new(PropExpr::Generator(TestSig::Eps2)),
            Box::new(eps()),
        );
        let atoms = |sd: &StringDiagram<TestSig>| sd.layers[0].atoms.clone();
        assert_eq!(
            atoms(&nf(&a)),
            vec![
                Atom::Generator(TestSig::Eps),
                Atom::Generator(TestSig::Eps2)
            ]
        );
        assert_eq!(
            atoms(&nf(&b)),
            vec![
                Atom::Generator(TestSig::Eps2),
                Atom::Generator(TestSig::Eps)
            ]
        );
        assert_ne!(nf(&a), nf(&b), "ε ⊗ ε' and ε' ⊗ ε are different morphisms");
    }

    /// A solid atom (`src > 0 ∧ tgt > 0`) blocks η mobility: `Identity(1)` does
    /// not strictly commute with η (both targets are non-empty), so the η must
    /// stay to the right of it. Moving it would permute the output wires.
    #[test]
    fn solid_atom_blocks_eta_mobility() {
        let e = PropExpr::Tensor(
            Box::new(eps()),
            Box::new(PropExpr::Tensor(
                Box::new(PropExpr::Identity(1)),
                Box::new(eta()),
            )),
        );
        let expected = StringDiagram {
            layers: vec![Layer {
                atoms: vec![
                    Atom::Generator(TestSig::Eps),
                    Atom::Identity(1),
                    Atom::Generator(TestSig::Eta),
                ],
            }],
        };
        assert_eq!(nf(&e), expected, "η must not cross the Identity(1)");
    }

    /// Idempotence on the #55 witnesses: re-running `nf` on the expression
    /// rebuilt from the NF reaches the same fixpoint.
    #[test]
    fn idempotent_on_zero_arity_witnesses() {
        let witnesses: Vec<PropExpr<TestSig>> = vec![
            PropExpr::Tensor(Box::new(eps()), Box::new(eta())),
            PropExpr::Tensor(Box::new(eta()), Box::new(eps())),
            PropExpr::Compose(Box::new(eps()), Box::new(eta())),
            PropExpr::Tensor(
                Box::new(eps()),
                Box::new(PropExpr::Tensor(
                    Box::new(PropExpr::Generator(TestSig::Eps2)),
                    Box::new(eta()),
                )),
            ),
            PropExpr::Tensor(
                Box::new(eps()),
                Box::new(PropExpr::Tensor(
                    Box::new(PropExpr::Identity(1)),
                    Box::new(eta()),
                )),
            ),
        ];
        for e in &witnesses {
            let once = nf(e);
            let twice = nf(&from_string_diagram(&once));
            assert_eq!(once, twice, "nf not idempotent on {e:?}");
        }
    }
}

// ============================================================================
// 4. Pure-SMC canonicality probes (issue #55 PR2)
// ============================================================================

/// **Unconfoundable canonicality metric.** Every pair here is SMC-equal by
/// construction, so `nf(lhs) == nf(rhs)` is a direct test of the normal form —
/// unlike `collisions_under_s`, which measures NF *plus* bounded-depth
/// E_18 congruence against matrix ground truth and so moves in either direction
/// when a sound NF change redistributes which equation-mediated identifications
/// succeed (diagnosis note 2026-07-26 §1, "metric lesson"). Read the collision
/// pins alongside these probes, never alone.
///
/// Coverage: the two-component counterexample that forced the PR2 re-cut, the
/// PR1 atomic witnesses, the mid-layer `η` interchange pair, closed-block
/// placement, the Step-7 block transpositions (including across fused identity
/// padding and among closed blocks, issue #79 P1), Step 6's `G::cmp` scalar
/// tie-break, the Step-6½ column transpositions (the three former nesting
/// residuals plus a merging wall, a short adjacency interval and a multi-level
/// nesting, issue #174) with the marked-encloser guard that survives them, and
/// idempotence on all of them.
mod smc_canonicality_probes {
    use super::*;
    use catgraph_applied::prop::presentation::smc_nf::{Atom, Layer, StringDiagram};
    use catgraph_applied::rig::BoolRig;
    use catgraph_applied::sfg::SfgGenerator;

    type Sfg = SfgGenerator<BoolRig>;

    fn prim(x: Sfg) -> PropExpr<Sfg> {
        PropExpr::Generator(x)
    }
    fn seq<G: PropSignature>(a: PropExpr<G>, b: PropExpr<G>) -> PropExpr<G> {
        PropExpr::Compose(Box::new(a), Box::new(b))
    }
    fn par<G: PropSignature>(a: PropExpr<G>, b: PropExpr<G>) -> PropExpr<G> {
        PropExpr::Tensor(Box::new(a), Box::new(b))
    }

    /// `A = μ ; ! : 2 → 0` — the input-anchored two-atom block.
    fn sink_block() -> PropExpr<Sfg> {
        seq(prim(SfgGenerator::Add), prim(SfgGenerator::Discard))
    }
    /// `B = η ; Δ : 0 → 2` — the output-only two-atom block.
    fn source_block() -> PropExpr<Sfg> {
        seq(prim(SfgGenerator::Zero), prim(SfgGenerator::Copy))
    }
    /// `η ; ! : 0 → 0` — a closed two-atom block.
    fn closed_block() -> PropExpr<Sfg> {
        seq(prim(SfgGenerator::Zero), prim(SfgGenerator::Discard))
    }
    /// `η ; s ; ! : 0 → 0` — a *different* closed block, for the closed↔closed
    /// reading-key order (`closed_blocks_sort_by_content_key`).
    fn long_closed_block() -> PropExpr<Sfg> {
        seq(closed_via_scalar(), prim(SfgGenerator::Discard))
    }
    /// `η ; s′ ; ! : 0 → 0` — a *third* closed block, differing from
    /// `long_closed_block` only in which scalar it carries, so the two are
    /// separated by the reading key's last resort: `G::cmp` on the generator.
    fn other_long_closed_block() -> PropExpr<Sfg> {
        seq(
            seq(prim(SfgGenerator::Zero), scalar_other()),
            prim(SfgGenerator::Discard),
        )
    }
    fn closed_via_scalar() -> PropExpr<Sfg> {
        seq(prim(SfgGenerator::Zero), scalar())
    }
    /// A **solid** `1 → 1` generator — `SfgGenerator::Scalar` is scalar
    /// *multiplication*, not a `0 → 0` closed atom, so it touches both
    /// boundaries and pairs freely only with a closed block.
    fn scalar() -> PropExpr<Sfg> {
        prim(SfgGenerator::Scalar(BoolRig(true)))
    }
    fn scalar_other() -> PropExpr<Sfg> {
        prim(SfgGenerator::Scalar(BoolRig(false)))
    }
    /// Left-associated tensor of a non-empty list — the shape the design
    /// round's fuzz corpus emits, kept so CE-R1/CE-R2 read as layers.
    fn tens(xs: Vec<PropExpr<Sfg>>) -> PropExpr<Sfg> {
        xs.into_iter().reduce(par).expect("non-empty factor list")
    }

    /// **The decisive counterexample** (diagnosis note 2026-07-26 §2). With
    /// `A = μ;! : 2 → 0` and `B = η;Δ : 0 → 2`, bifunctoriality with `id₀` gives
    /// `A ; B = A ⊗ B : 2 → 2`. The parked point-span sift anchored `B`'s `η` at
    /// its incidental source cursor and block-transposed the two components in
    /// the compose-form; the component anchor (rule (i)) puts the
    /// input-anchored block left in both, converging on the tensor-form layout
    /// `pad_and_zip` already produces.
    #[test]
    fn two_component_counterexample_converges_to_tensor_layout() {
        let compose_form = seq(sink_block(), source_block());
        let tensor_form = par(sink_block(), source_block());

        let expected = StringDiagram {
            layers: vec![
                Layer {
                    atoms: vec![
                        Atom::Generator(SfgGenerator::Add),
                        Atom::Generator(SfgGenerator::Zero),
                    ],
                },
                Layer {
                    atoms: vec![
                        Atom::Generator(SfgGenerator::Discard),
                        Atom::Generator(SfgGenerator::Copy),
                    ],
                },
            ],
        };
        assert_eq!(
            nf(&compose_form),
            nf(&tensor_form),
            "(μ;!);(η;Δ) and (μ;!)⊗(η;Δ) are the same morphism"
        );
        assert_eq!(
            nf(&compose_form),
            expected,
            "canonical layout is the tensor zip, input-anchored block left"
        );
    }

    /// The PR1 atomic witnesses, re-asserted as a probe set: all three forms of
    /// the tied `η ∥ ε` pair share one NF, in Decision 1's order (η first).
    /// Before #174 this pair needed the §2.6 disjointness carve to keep it away
    /// from rule (i)'s component order; the carve is retired and the tied
    /// comparator reads only the two atoms, so Decision 1 applies unconditionally.
    #[test]
    fn atomic_eta_eps_witnesses_converge() {
        let eps: PropExpr<TestSig> = PropExpr::Generator(TestSig::Eps);
        let eta: PropExpr<TestSig> = PropExpr::Generator(TestSig::Eta);
        let forms = [
            seq(eps.clone(), eta.clone()),
            par(eta.clone(), eps.clone()),
            par(eps, eta),
        ];
        let expected = StringDiagram {
            layers: vec![Layer {
                atoms: vec![Atom::Generator(TestSig::Eta), Atom::Generator(TestSig::Eps)],
            }],
        };
        for form in &forms {
            assert_eq!(nf(form), expected, "atomic witness diverged: {form:?}");
        }
    }

    /// The mid-layer `η` interchange pair: `F ⊗ η ⊗ G` against
    /// `(F ⊗ G) ; (id₁ ⊗ η ⊗ id₁)`. The `η`'s component is output-only and
    /// non-interleaved, so the sift slides it into the earlier layer at the
    /// `F | G` atom boundary.
    #[test]
    fn mid_layer_eta_interchange_converges() {
        let f: PropExpr<TestSig> = PropExpr::Generator(TestSig::F);
        let g: PropExpr<TestSig> = PropExpr::Generator(TestSig::G);
        let eta: PropExpr<TestSig> = PropExpr::Generator(TestSig::Eta);

        let tensor_form = par(f.clone(), par(eta.clone(), g.clone()));
        let compose_form = seq(
            par(f, g),
            par(PropExpr::Identity(1), par(eta, PropExpr::Identity(1))),
        );
        let expected = StringDiagram {
            layers: vec![Layer {
                atoms: vec![
                    Atom::Generator(TestSig::F),
                    Atom::Generator(TestSig::Eta),
                    Atom::Generator(TestSig::G),
                ],
            }],
        };
        assert_eq!(nf(&tensor_form), nf(&compose_form));
        assert_eq!(nf(&tensor_form), expected);
    }

    /// A **closed** (`0 → 0`) two-atom block placed beside a solid generator:
    /// the compose-form and the tensor-form of each placement converge. The
    /// closed block's `η` is scheduled by rule (i)'s closed branch — leftmost
    /// within the slots its coordinate admits.
    ///
    /// The block-level companion — `(η;!) ⊗ s` against `s ⊗ (η;!)` — is
    /// `block_transposition_converges`.
    #[test]
    fn closed_block_placement_converges() {
        // Left placement: (η;!) ⊗ s  ==  (η ⊗ id₁) ; (! ⊗ s).
        let left_tensor = par(closed_block(), scalar());
        let left_compose = seq(
            par(prim(SfgGenerator::Zero), PropExpr::Identity(1)),
            par(prim(SfgGenerator::Discard), scalar()),
        );
        assert_eq!(
            nf(&left_tensor),
            nf(&left_compose),
            "closed block on the left"
        );

        // Right placement: s ⊗ (η;!)  ==  (id₁ ⊗ η) ; (s ⊗ !).
        let right_tensor = par(scalar(), closed_block());
        let right_compose = seq(
            par(PropExpr::Identity(1), prim(SfgGenerator::Zero)),
            par(scalar(), prim(SfgGenerator::Discard)),
        );
        assert_eq!(
            nf(&right_tensor),
            nf(&right_compose),
            "closed block on the right"
        );
    }

    /// **Block transposition converges** — Step 7 (`reorder_component_blocks`).
    /// Rule (i) orders whole components, and until Step 7 the pipeline's only
    /// moves were the single-atom sift (up one layer) and Step 6's within-layer
    /// reorder, neither of which can perform the coupled multi-layer block move
    /// the diagnosis note (§2) identified. Step 7 performs it, so the tensor-form
    /// transpositions converge too.
    ///
    /// Two families:
    /// - `A = μ;! : 2 → 0` (input-only) against `B = η;Δ : 0 → 2` (output-only):
    ///   the **full three-member family** — both tensor orders and the compose
    ///   form — lands on the one rule-(i) layout;
    /// - a closed block against a solid `1 → 1` generator. The solid one touches
    ///   *both* boundaries, and `closed ∥ both` is still a free pair (a closed
    ///   component occupies no boundary wire at all), so the closed block sorts
    ///   leftmost from either writing.
    #[test]
    fn block_transposition_converges() {
        let expected = StringDiagram {
            layers: vec![
                Layer {
                    atoms: vec![
                        Atom::Generator(SfgGenerator::Add),
                        Atom::Generator(SfgGenerator::Zero),
                    ],
                },
                Layer {
                    atoms: vec![
                        Atom::Generator(SfgGenerator::Discard),
                        Atom::Generator(SfgGenerator::Copy),
                    ],
                },
            ],
        };
        for form in [
            par(sink_block(), source_block()),
            par(source_block(), sink_block()),
            seq(sink_block(), source_block()),
        ] {
            assert_eq!(nf(&form), expected, "block-order family diverged: {form:?}");
        }

        let closed_left = nf(&par(closed_block(), scalar()));
        assert_eq!(
            nf(&par(scalar(), closed_block())),
            closed_left,
            "a closed block sorts leftmost against a solid generator"
        );
        assert_eq!(
            closed_left.layers[0].atoms[0],
            Atom::Generator(SfgGenerator::Zero),
            "…and leftmost means the closed block's η opens layer 0"
        );
    }

    /// Fused identity padding. `(s ⊗ s′) ⊗ (η;!)` pads the one-layer `s ⊗ s′`
    /// with a single `Identity(2)` spanning **both** solid components, and the
    /// plain union-find joins those two through it. Step 7 pre-splits every
    /// `Identity(n)` at wire boundaries before analysing — free, since
    /// `Identity(a+b) = Identity(a) ⊗ Identity(b)` — which keeps them apart and
    /// lets the closed block bubble left past each in turn; the identity re-fuses
    /// on the way out.
    #[test]
    fn block_transposition_crosses_fused_identity_padding() {
        let solids = par(scalar(), scalar_other());
        let expected = StringDiagram {
            layers: vec![
                Layer {
                    atoms: vec![
                        Atom::Generator(SfgGenerator::Zero),
                        Atom::Generator(SfgGenerator::Scalar(BoolRig(true))),
                        Atom::Generator(SfgGenerator::Scalar(BoolRig(false))),
                    ],
                },
                Layer {
                    atoms: vec![Atom::Generator(SfgGenerator::Discard), Atom::Identity(2)],
                },
            ],
        };
        assert_eq!(nf(&par(solids.clone(), closed_block())), expected);
        assert_eq!(nf(&par(closed_block(), solids)), expected);
    }

    /// **Closed↔closed order, by content** (issue #79 P1 — formerly residual
    /// (b)). Rule (i) gives every closed component the same key `(closed, 0)`,
    /// so until P1 two *distinct* closed blocks kept whichever order they were
    /// written in. Breaking that tie needs a content-derived total order on
    /// components, which bottoms out in an `Ord` bound on `G`; `PropSignature`
    /// now carries one (Decision 2), and Step 7 compares tied blocks by their
    /// **in-situ reading** — layer by layer, left to right, atoms mapped to
    /// (kind, widths, generator) and compared lexicographically. The reading
    /// names no wire coordinate, so it survives the very swap it licenses.
    #[test]
    fn closed_blocks_sort_by_content_key() {
        assert_eq!(
            nf(&par(closed_block(), long_closed_block())),
            nf(&par(long_closed_block(), closed_block())),
            "two closed blocks must agree under transposition"
        );
    }

    /// Three *distinct* closed blocks: all six writings converge on one NF, and
    /// the blocks land in reading-key order. The readings share their first
    /// atom (`η`) and separate at the second — `!` before `s·(–)` by atom kind
    /// and variant, then `s′ = false` before `s = true` by `G::cmp` — so this
    /// exercises every level of the key, generator order included.
    #[test]
    fn three_closed_blocks_converge_in_reading_key_order() {
        let blocks = [
            closed_block(),
            long_closed_block(),
            other_long_closed_block(),
        ];
        let permutations = [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ];
        let expected = StringDiagram {
            layers: vec![
                Layer {
                    atoms: vec![
                        Atom::Generator(SfgGenerator::Zero),
                        Atom::Generator(SfgGenerator::Zero),
                        Atom::Generator(SfgGenerator::Zero),
                    ],
                },
                Layer {
                    atoms: vec![
                        Atom::Generator(SfgGenerator::Discard),
                        Atom::Generator(SfgGenerator::Scalar(BoolRig(false))),
                        Atom::Generator(SfgGenerator::Scalar(BoolRig(true))),
                    ],
                },
                Layer {
                    atoms: vec![
                        Atom::Generator(SfgGenerator::Discard),
                        Atom::Generator(SfgGenerator::Discard),
                    ],
                },
            ],
        };
        for p in permutations {
            let form = par(
                blocks[p[0]].clone(),
                par(blocks[p[1]].clone(), blocks[p[2]].clone()),
            );
            assert_eq!(nf(&form), expected, "closed-block writing diverged: {p:?}");
        }
    }

    /// Two *identical* closed blocks. Their readings are equal, so Step 7 makes
    /// no swap — and none is needed: equal readings mean the blocks are the
    /// same, and transposing them is invisible. The tensor and compose writings
    /// (`A ⊗ A` and `A ; A`, the same `0 → 0` morphism) converge on the
    /// two-copy layout.
    #[test]
    fn identical_closed_blocks_converge_trivially() {
        let expected = StringDiagram {
            layers: vec![
                Layer {
                    atoms: vec![
                        Atom::Generator(SfgGenerator::Zero),
                        Atom::Generator(SfgGenerator::Zero),
                    ],
                },
                Layer {
                    atoms: vec![
                        Atom::Generator(SfgGenerator::Discard),
                        Atom::Generator(SfgGenerator::Discard),
                    ],
                },
            ],
        };
        assert_eq!(nf(&par(closed_block(), closed_block())), expected);
        assert_eq!(nf(&seq(closed_block(), closed_block())), expected);
    }

    /// Closed↔closed ordering across the fused identity padding of
    /// `block_transposition_crosses_fused_identity_padding`: two solid `1 → 1`
    /// generators padded by a single `Identity(2)`, with two distinct closed
    /// blocks to place. Step 7's pre-split keeps the two solids apart, and the
    /// reading key then orders the closed blocks the same way from every
    /// writing.
    #[test]
    fn closed_block_order_crosses_fused_identity_padding() {
        let solids = || par(scalar(), scalar_other());
        let baseline = nf(&par(solids(), par(closed_block(), long_closed_block())));
        for form in [
            par(solids(), par(long_closed_block(), closed_block())),
            par(par(closed_block(), long_closed_block()), solids()),
            par(par(long_closed_block(), closed_block()), solids()),
            par(closed_block(), par(solids(), long_closed_block())),
            par(long_closed_block(), par(solids(), closed_block())),
        ] {
            assert_eq!(
                nf(&form),
                baseline,
                "closed blocks across fused padding diverged: {form:?}"
            );
        }
        assert_eq!(
            baseline.layers[0].atoms[0],
            Atom::Generator(SfgGenerator::Zero),
            "a closed block still opens layer 0"
        );
    }

    /// **Step 6's `G::cmp` scalar tie-break** (issue #79 P1). A `0 → 0` scalar
    /// strictly commutes with every atom, so two of them at an equal zero-arity
    /// class are the one case the Decision-1 class order cannot separate. They
    /// now sort ascending by `G::cmp`. No shipped signature has a `0 → 0`
    /// generator — hence `TestSig::Sc`, and hence the baseline-inertness of the
    /// change.
    #[test]
    fn tied_scalars_sort_by_generator_order() {
        let sc = |k: u8| PropExpr::Generator(TestSig::Sc(k));
        let expected = StringDiagram {
            layers: vec![Layer {
                atoms: vec![
                    Atom::Generator(TestSig::Sc(1)),
                    Atom::Generator(TestSig::Sc(2)),
                ],
            }],
        };
        assert_eq!(nf(&par(sc(1), sc(2))), expected);
        assert_eq!(nf(&par(sc(2), sc(1))), expected);
    }

    /// The three-scalar companion: every writing of `s₁ ⊗ s₂ ⊗ s₃` converges on
    /// the ascending order. Two scalars need one swap; three need the bubble
    /// pass to reach the lex-least word of the trace monoid.
    #[test]
    fn three_tied_scalars_converge_in_generator_order() {
        let sc = |k: u8| PropExpr::Generator(TestSig::Sc(k));
        let expected = StringDiagram {
            layers: vec![Layer {
                atoms: vec![
                    Atom::Generator(TestSig::Sc(1)),
                    Atom::Generator(TestSig::Sc(2)),
                    Atom::Generator(TestSig::Sc(3)),
                ],
            }],
        };
        for p in [
            [1u8, 2, 3],
            [1, 3, 2],
            [2, 1, 3],
            [2, 3, 1],
            [3, 1, 2],
            [3, 2, 1],
        ] {
            let form = par(sc(p[0]), par(sc(p[1]), sc(p[2])));
            assert_eq!(nf(&form), expected, "scalar writing diverged: {p:?}");
        }
    }

    /// **Trapped nesting extracts** (§4.6(c), closed by the Step 6½ column pass,
    /// issue #174). A closed block written strictly *inside* another
    /// component's wire span used to be stuck: its `η`'s coordinate falls
    /// strictly inside the enclosing atom's target span, so the point-span sift
    /// is blocked (correctly — the gap-closer is foreign), and Step 7 never sees
    /// an adjacent free pair because the identity wires surrounding the closed
    /// block belong to the *enclosing* component, whose run is therefore not
    /// contiguous.
    ///
    /// Step 6½ makes the missing move. The closed block is a `0 → 0` column, so
    /// it strictly commutes with the enclosing component's identity column over
    /// their shared layer interval; rule (i) sorts closed leftmost, the block
    /// transposes out, the two identity wires re-fuse, and both the `η` and the
    /// consumer below can then sift. The nested and free writings have identical
    /// abstract content, and now identical normal forms.
    #[test]
    fn trapped_closed_block_extracts() {
        // Copy ; (id₁ ⊗ Zero ⊗ id₁) ; (id₁ ⊗ Discard ⊗ id₁) ; Add — the closed
        // {Zero, Discard} loop written between Copy's two output wires.
        let id1 = || PropExpr::Identity(1);
        let nested = seq(
            seq(
                seq(
                    prim(SfgGenerator::Copy),
                    par(id1(), par(prim(SfgGenerator::Zero), id1())),
                ),
                par(id1(), par(prim(SfgGenerator::Discard), id1())),
            ),
            prim(SfgGenerator::Add),
        );
        // The same content written free: (Zero;Discard) ⊗ (Copy;Add).
        let free = par(
            closed_block(),
            seq(prim(SfgGenerator::Copy), prim(SfgGenerator::Add)),
        );
        let expected = StringDiagram {
            layers: vec![
                Layer {
                    atoms: vec![
                        Atom::Generator(SfgGenerator::Zero),
                        Atom::Generator(SfgGenerator::Copy),
                    ],
                },
                Layer {
                    atoms: vec![
                        Atom::Generator(SfgGenerator::Discard),
                        Atom::Generator(SfgGenerator::Add),
                    ],
                },
            ],
        };
        assert_eq!(
            nf(&nested),
            nf(&free),
            "a nested closed block extracts to the free layout"
        );
        assert_eq!(
            nf(&nested),
            expected,
            "…and the free layout is the closed block leftmost, rule (i)"
        );
    }

    /// **Nested column, sink form (CE-A)** (§4.6(d), closed by the Step 6½
    /// column pass, issue #174). A solid-headed multi-atom `1 → 0` block
    /// (`Scalar;Discard`) written at a coordinate strictly inside the
    /// `{Zero, …, Add}` component's span used to reach none of its free
    /// writings: Step 6 never bubbles `Zero` past the solid `Scalar` head, and
    /// Step 7's free-pair test is whole-component while the actual SMC freedom
    /// is column-vs-block (`Zero ⊗ ε = ε ⊗ Zero`). Both components are
    /// boundary-attached and unmarked, so the pair sits *inside* the fragment
    /// `𝔉` — which is why this pair refuted the draft §4 theorem in the
    /// 2026-07-27 adversarial review.
    ///
    /// Step 6½ makes exactly that column move: the sink block is `1 → 0` and the
    /// enclosing component's neighbouring column is `0 → 1`, so their block
    /// arities strictly commute over the shared interval, and both components
    /// attach the input boundary — the sink block at coordinate 0 — so rule (i)
    /// puts it left. With the swap made, the enclosing component's two identity
    /// wires become adjacent, fuse, and `Add` sifts up. Unlike the source form
    /// below, this one really does need the column pass.
    #[test]
    fn nested_sink_block_converges_with_free_writing() {
        let id1 = || PropExpr::Identity(1);
        // (Zero ⊗ Scalar ⊗ id₁) ; (id₁ ⊗ Discard ⊗ id₁) ; Add : 2 → 1
        let nested = seq(
            seq(
                par(prim(SfgGenerator::Zero), par(scalar(), id1())),
                par(id1(), par(prim(SfgGenerator::Discard), id1())),
            ),
            prim(SfgGenerator::Add),
        );
        // (Scalar;Discard) ⊗ ((Zero ⊗ id₁) ; Add) : 2 → 1
        let free = par(
            seq(scalar(), prim(SfgGenerator::Discard)),
            seq(
                par(prim(SfgGenerator::Zero), id1()),
                prim(SfgGenerator::Add),
            ),
        );
        let expected = StringDiagram {
            layers: vec![
                Layer {
                    atoms: vec![
                        Atom::Generator(SfgGenerator::Scalar(BoolRig(true))),
                        Atom::Generator(SfgGenerator::Zero),
                        Atom::Identity(1),
                    ],
                },
                Layer {
                    atoms: vec![
                        Atom::Generator(SfgGenerator::Discard),
                        Atom::Generator(SfgGenerator::Add),
                    ],
                },
            ],
        };
        assert_eq!(
            nf(&nested),
            nf(&free),
            "a nested solid-headed sink block converges with its free writing"
        );
        assert_eq!(
            nf(&nested),
            expected,
            "…on the layout its input coordinates pin: the sink block leftmost"
        );
    }

    /// **Nested source block converges (CE-A3)** — the time-reversed mirror of
    /// `nested_sink_block_converges_with_free_writing`: the enclosing wall opens
    /// at an `ε` (`Discard`) *below* instead of an `η` above, and the nested
    /// block is output-only (`Zero;Scalar`).
    ///
    /// **Not a column residual after all** (issue #174 design round correction,
    /// 2026-07-28). §4.6(d) filed this as the mirror of the sink form, needing
    /// the same column move. It is not: what blocked it was Step 6 refusing to
    /// bubble the nested block's `η` past the encloser's `ε` in a single layer,
    /// because the tied comparator's rule-(i) branch ranked the two *components*
    /// ahead of the Decision-1 class order. Retiring that branch — the free-site
    /// retirement of §2.6 — lets `η < ε` fire, the `η` reaches an atom boundary,
    /// and the ordinary point-span sift finishes the job. The column pass is not
    /// involved. Kept as a regression on the retirement, not on Step 6½.
    #[test]
    fn nested_source_block_converges_with_free_writing() {
        let id1 = || PropExpr::Identity(1);
        // Copy ; (id₁ ⊗ Zero ⊗ id₁) ; (Discard ⊗ Scalar ⊗ id₁) : 1 → 2
        let nested = seq(
            seq(
                prim(SfgGenerator::Copy),
                par(id1(), par(prim(SfgGenerator::Zero), id1())),
            ),
            par(prim(SfgGenerator::Discard), par(scalar(), id1())),
        );
        // (Zero;Scalar) ⊗ (Copy ; (Discard ⊗ id₁)) : 1 → 2
        let free = par(
            seq(prim(SfgGenerator::Zero), scalar()),
            seq(
                prim(SfgGenerator::Copy),
                par(prim(SfgGenerator::Discard), id1()),
            ),
        );
        let expected = StringDiagram {
            layers: vec![
                Layer {
                    atoms: vec![
                        Atom::Generator(SfgGenerator::Zero),
                        Atom::Generator(SfgGenerator::Copy),
                    ],
                },
                Layer {
                    atoms: vec![
                        Atom::Generator(SfgGenerator::Scalar(BoolRig(true))),
                        Atom::Generator(SfgGenerator::Discard),
                        Atom::Identity(1),
                    ],
                },
            ],
        };
        assert_eq!(
            nf(&nested),
            nf(&free),
            "a nested output-only solid-tailed block converges with its free writing"
        );
        assert_eq!(
            nf(&nested),
            expected,
            "…on the layout its output coordinates pin: the source block leftmost"
        );
    }

    // ------------------------------------------------------------------
    // Step 6½ column pass — interval shapes and the surviving guard
    // ------------------------------------------------------------------

    /// `Copy ; (id₁ ⊗ Zero ⊗ id₁) ; (Copy ⊗ Discard ⊗ id₁) ; (Add ⊗ id₁) ; Add`
    /// — the enclosing wall **merges inside the block's span**, so the column
    /// left of the closed block is one identity wire in the upper layer and a
    /// `Copy` in the lower one. The pass does not require the columns to have
    /// the same shape, only that their three cuts sit at the same wire
    /// coordinate read from above and from below at every internal boundary;
    /// they do, so the block still extracts.
    #[test]
    fn column_move_crosses_a_merging_wall() {
        let id1 = || PropExpr::Identity(1);
        let nested = seq(
            seq(
                seq(
                    seq(
                        prim(SfgGenerator::Copy),
                        par(id1(), par(prim(SfgGenerator::Zero), id1())),
                    ),
                    par(
                        prim(SfgGenerator::Copy),
                        par(prim(SfgGenerator::Discard), id1()),
                    ),
                ),
                par(prim(SfgGenerator::Add), id1()),
            ),
            prim(SfgGenerator::Add),
        );
        let free = par(
            closed_block(),
            seq(
                seq(
                    seq(
                        prim(SfgGenerator::Copy),
                        par(prim(SfgGenerator::Copy), id1()),
                    ),
                    par(prim(SfgGenerator::Add), id1()),
                ),
                prim(SfgGenerator::Add),
            ),
        );
        assert_eq!(
            nf(&nested),
            nf(&free),
            "merging wall blocked the column move"
        );
    }

    /// A nested block whose **span exceeds its adjacency run**:
    /// `Copy ; (id₁ ⊗ η ⊗ id₁) ; (! ⊗ s ⊗ id₁) ; (! ⊗ id₁)` traps the three-atom
    /// closed block `η ; s ; !` inside the `Copy`-component's span, and the
    /// encloser sits immediately left of it in only the first two of its three
    /// layers — in the third the encloser's left wire has already died into its
    /// own `!`, so the block's `!` opens that layer with nothing beside it.
    ///
    /// **Honest scope.** This converges, but *not* because of Step 6½: ablating
    /// the pass leaves it converging, because the encloser's early `!` opens
    /// enough room for the ordinary sift to place the block's `η` on its own.
    /// It is kept as a convergence regression over the nested-block family — the
    /// shape whose interval arithmetic is most likely to be broken by a future
    /// change — not as a witness that the adjacency-run interval is load-bearing.
    /// No pass-dependent witness for that sub-case was found; the interval logic
    /// is exercised end-to-end by the other column probes, all five of which do
    /// fail when Step 6½ is ablated.
    #[test]
    fn column_interval_is_the_adjacency_run_not_the_block_span() {
        let id1 = || PropExpr::Identity(1);
        let nested = seq(
            seq(
                seq(
                    prim(SfgGenerator::Copy),
                    par(id1(), par(prim(SfgGenerator::Zero), id1())),
                ),
                par(prim(SfgGenerator::Discard), par(scalar(), id1())),
            ),
            par(prim(SfgGenerator::Discard), id1()),
        );
        let free = par(
            seq(closed_via_scalar(), prim(SfgGenerator::Discard)),
            seq(
                prim(SfgGenerator::Copy),
                par(prim(SfgGenerator::Discard), id1()),
            ),
        );
        assert_eq!(
            nf(&nested),
            nf(&free),
            "short adjacency interval was missed"
        );
    }

    /// **The interval-alignment check does real work.** Step 6½ only transposes
    /// a pair whose three cuts sit at the same wire coordinate read from above
    /// and from below at every internal boundary; without that test the "swap"
    /// would not be a tensor transposition at all. The check is not decorative:
    /// instrumenting it over the design round's 100 000-case differential corpus
    /// counted **5 780** candidate intervals rejected on alignment alone.
    ///
    /// This is the smallest such case, delta-shrunk from corpus case 8 — a
    /// four-layer diagram whose components weave enough that a widened
    /// adjacency run offers intervals the cuts do not line up across. It is
    /// asserted the only way a probe can: the expression and a sound interchange
    /// rewriting of it (the trailing `!` pulled into its own layer) reach the
    /// same normal form, and that form is a fixpoint.
    #[test]
    fn interval_alignment_check_is_exercised() {
        let id = PropExpr::Identity;
        let l0 = par(
            par(prim(SfgGenerator::Copy), prim(SfgGenerator::Discard)),
            prim(SfgGenerator::Zero),
        );
        let l1 = par(id(2), prim(SfgGenerator::Discard));
        let l2 = tens(vec![
            prim(SfgGenerator::Zero),
            scalar(),
            prim(SfgGenerator::Zero),
            id(1),
        ]);
        let l3 = tens(vec![
            id(1),
            prim(SfgGenerator::Copy),
            prim(SfgGenerator::Copy),
            prim(SfgGenerator::Zero),
            prim(SfgGenerator::Discard),
        ]);
        let a = seq(seq(seq(l0.clone(), l1.clone()), l2.clone()), l3);
        // Same morphism: the trailing `!` consumes its wire in a layer of its own.
        let l3_split = seq(
            par(id(3), prim(SfgGenerator::Discard)),
            tens(vec![
                id(1),
                prim(SfgGenerator::Copy),
                prim(SfgGenerator::Copy),
                prim(SfgGenerator::Zero),
            ]),
        );
        let b = seq(seq(seq(l0, l1), l2), l3_split);
        assert_eq!(nf(&a), nf(&b), "alignment-exercising pair diverged");
        let once = nf(&a);
        assert_eq!(once, nf(&from_string_diagram(&once)), "not idempotent");
    }

    /// **Multi-nested**: a closed block nested inside a sink block that is
    /// itself nested inside a larger component. Both the fully-free writing and
    /// the half-free one (inner block extracted, outer block still nested)
    /// converge with the doubly-nested writing, so the pass composes across
    /// nesting depth rather than only unwrapping one level per fixpoint.
    #[test]
    fn multi_nested_blocks_extract() {
        let id1 = || PropExpr::Identity(1);
        // A 1 → 1 component carrying its own nested closed block.
        let boxed = || {
            seq(
                seq(
                    seq(
                        prim(SfgGenerator::Copy),
                        par(id1(), par(prim(SfgGenerator::Zero), id1())),
                    ),
                    par(id1(), par(prim(SfgGenerator::Discard), id1())),
                ),
                prim(SfgGenerator::Add),
            )
        };
        // (Zero ⊗ boxed ⊗ id₁) ; (id₁ ⊗ Discard ⊗ id₁) ; Add — CE-A's shape with
        // the solid head replaced by `boxed`.
        let nested = seq(
            seq(
                par(prim(SfgGenerator::Zero), par(boxed(), id1())),
                par(id1(), par(prim(SfgGenerator::Discard), id1())),
            ),
            prim(SfgGenerator::Add),
        );
        let half_free = par(
            seq(boxed(), prim(SfgGenerator::Discard)),
            seq(
                par(prim(SfgGenerator::Zero), id1()),
                prim(SfgGenerator::Add),
            ),
        );
        let free = par(
            closed_block(),
            par(
                seq(
                    seq(prim(SfgGenerator::Copy), prim(SfgGenerator::Add)),
                    prim(SfgGenerator::Discard),
                ),
                seq(
                    par(prim(SfgGenerator::Zero), id1()),
                    prim(SfgGenerator::Add),
                ),
            ),
        );
        assert_eq!(nf(&nested), nf(&half_free), "outer nesting did not extract");
        assert_eq!(nf(&nested), nf(&free), "inner nesting did not extract");
    }

    /// **The widened sift surface** (issue #174 design round). Guard 3 used to
    /// stop the `η` sift outright for a marked component; since the free-site
    /// retirement the sift is purely coordinate-driven and marking no longer
    /// enters it. This is the round's flagged risk, so it gets a witness.
    ///
    /// The shape has to satisfy three conditions at once for the old guard to
    /// have been reachable, which is narrower than it looks. The `η`'s component
    /// must be **marked**; it must **not** be input-anchored, because the old
    /// `eta_placement` returned the leftmost slot for `class == 1` *before* it
    /// ever consulted `interleaved`; and its output coordinate must land on an
    /// atom boundary of the preceding layer, or `point_placement` declines for
    /// plain geometry and the guard is never asked.
    ///
    /// `Copy ; (s ⊗ s′) ; (id₁ ⊗ η ⊗ id₁)` meets all three. `Copy` joins the two
    /// scalars into one component, so the output owner word reads `A Z A` and
    /// `mark_interleaved` marks both; `Z = {η}` touches only the output boundary,
    /// so it is `class == 2`; and the `η` sits at the `s | s′` atom boundary.
    /// Under the old guard that `η` never moved and this pair diverged — verified
    /// by ablation, restoring `interleaved` at the sift flips the assertion — and
    /// it now sifts, so the two writings of the same morphism converge.
    #[test]
    fn marked_component_eta_sifts_and_converges() {
        let id1 = || PropExpr::Identity(1);
        // Copy ; (s ⊗ s′) ; (id₁ ⊗ η ⊗ id₁) : 1 → 3
        let late = seq(
            seq(prim(SfgGenerator::Copy), par(scalar(), scalar_other())),
            par(id1(), par(prim(SfgGenerator::Zero), id1())),
        );
        // Copy ; (s ⊗ η ⊗ s′) : 1 → 3 — same morphism, `η` written in one layer.
        let early = seq(
            prim(SfgGenerator::Copy),
            par(scalar(), par(prim(SfgGenerator::Zero), scalar_other())),
        );
        assert_eq!(
            nf(&late),
            nf(&early),
            "a marked component's η must still schedule canonically now that \
             guard 3 no longer blocks the sift"
        );
        for e in [&late, &early] {
            let once = nf(e);
            assert_eq!(once, nf(&from_string_diagram(&once)), "not idempotent");
        }
    }

    /// **Braid guard** (Step 6½). The trapped-closed nesting with the enclosing
    /// component carrying a `Braid`: braid placement belongs to Step 3, and
    /// `canonicalize_braid_runs` recomputes braid runs from the underlying
    /// permutation, so letting Step 6½ move a braid-bearing component's atoms
    /// would put the two passes on each other's territory. The guard declines
    /// the pair, and the nested writing does not reach the free one.
    #[test]
    fn braid_bearing_encloser_blocks_the_column_move() {
        let id1 = || PropExpr::Identity(1);
        // σ ; (id₁ ⊗ η ⊗ id₁) ; (id₁ ⊗ ! ⊗ id₁) ; μ : 2 → 1
        let nested = seq(
            seq(
                seq(
                    PropExpr::Braid(1, 1),
                    par(id1(), par(prim(SfgGenerator::Zero), id1())),
                ),
                par(id1(), par(prim(SfgGenerator::Discard), id1())),
            ),
            prim(SfgGenerator::Add),
        );
        let free = par(
            closed_block(),
            seq(PropExpr::Braid(1, 1), prim(SfgGenerator::Add)),
        );
        assert_ne!(
            nf(&nested),
            nf(&free),
            "the braid guard is meant to leave a braid-bearing component's \
             columns alone; if this converges, Step 3 and Step 6½ now share \
             atoms and the oscillation argument needs rechecking"
        );
    }

    /// **The refinement boundary** (Steps 7 and 6½). The enclosing component
    /// presents a fused `Identity(2)` — one atom, two wires — immediately left
    /// of the nested closed block, so the unrefined union-find would join the
    /// two components through it and see no transposable pair at all. Both
    /// passes rewrite on the identity-split refinement, where the `Identity(2)`
    /// is two `Identity(1)`s and the block's column is genuinely adjacent.
    #[test]
    fn column_move_crosses_a_fused_wide_identity() {
        let id1 = || PropExpr::Identity(1);
        let id2 = || PropExpr::Identity(2);
        // Copy ; (Copy ⊗ id₁) ; (id₂ ⊗ η ⊗ id₁) ; (id₂ ⊗ ! ⊗ id₁) ; (μ ⊗ id₁) ; μ
        let nested = seq(
            seq(
                seq(
                    seq(
                        seq(
                            prim(SfgGenerator::Copy),
                            par(prim(SfgGenerator::Copy), id1()),
                        ),
                        par(id2(), par(prim(SfgGenerator::Zero), id1())),
                    ),
                    par(id2(), par(prim(SfgGenerator::Discard), id1())),
                ),
                par(prim(SfgGenerator::Add), id1()),
            ),
            prim(SfgGenerator::Add),
        );
        let free = par(
            closed_block(),
            seq(
                seq(
                    seq(
                        prim(SfgGenerator::Copy),
                        par(prim(SfgGenerator::Copy), id1()),
                    ),
                    par(prim(SfgGenerator::Add), id1()),
                ),
                prim(SfgGenerator::Add),
            ),
        );
        assert_eq!(
            nf(&nested),
            nf(&free),
            "a fused Identity(2) wall blocked the column move"
        );
    }

    // ------------------------------------------------------------------
    // CE-R1 / CE-R2 — the design round's refutation witnesses
    // ------------------------------------------------------------------

    /// **CE-R1** — the pair that refuted the shared-output-boundary clause
    /// (issue #174 design round, 2026-07-28). `B` is `A` with one interchange
    /// split on the third factor (`X ; (η ⊗ id₂) = η ⊗ X` at wire position 0
    /// under `id₁ ⊗ —`), so the two are SMC-equal by bifunctoriality alone.
    ///
    /// It converges on the pre-#174 engine and on the shipped one, and it
    /// **diverged** under both of the riders the first PR-A attempt carried: the
    /// `out_min` comparator clause broke it at the `η`-slot walk, and the
    /// analysis-refinement adoption broke it independently. Both riders were
    /// retired; this witness is what stops either coming back unnoticed. Its
    /// components are unmarked and boundary-attached, so the pair sits *inside*
    /// the fragment 𝔉 — a divergence here would be a genuine canonicality
    /// regression, not a documented residual.
    #[test]
    fn ce_r1_interchange_split_converges() {
        let l1 = tens(vec![
            prim(SfgGenerator::Copy),
            scalar_other(),
            prim(SfgGenerator::Zero),
        ]);
        let l2 = tens(vec![
            scalar_other(),
            scalar_other(),
            prim(SfgGenerator::Copy),
            prim(SfgGenerator::Copy),
            prim(SfgGenerator::Zero),
        ]);
        let l3 = tens(vec![
            PropExpr::Identity(1),
            prim(SfgGenerator::Zero),
            prim(SfgGenerator::Discard),
            prim(SfgGenerator::Discard),
            prim(SfgGenerator::Add),
            prim(SfgGenerator::Add),
        ]);
        let a = seq(seq(l1.clone(), l2.clone()), l3);
        let l3b = par(
            PropExpr::Identity(1),
            seq(
                tens(vec![
                    prim(SfgGenerator::Discard),
                    prim(SfgGenerator::Discard),
                    prim(SfgGenerator::Add),
                    prim(SfgGenerator::Add),
                ]),
                par(prim(SfgGenerator::Zero), PropExpr::Identity(2)),
            ),
        );
        let b = seq(l1, seq(l2, l3b));
        assert_eq!(nf(&a), nf(&b), "CE-R1 diverged");
        let once = nf(&a);
        assert_eq!(
            once,
            nf(&from_string_diagram(&once)),
            "CE-R1 not idempotent"
        );
    }

    /// **CE-R2** — the refinement rider's own regression guard (issue #174
    /// design round). `B` is `A` with the leading scalar slid out through
    /// identity padding, again SMC-equal by interchange.
    ///
    /// This one sits *outside* 𝔉 — it has a closed component — so its
    /// convergence is not covered by any fragment claim, and the probe suite is
    /// the only thing recording it. It converged on the pre-#174 engine, broke
    /// under the `analyze_components_refined` adoption specifically (the
    /// comparator clause did not touch it), and converges again now that the
    /// adoption is reverted. It is here so a future re-attempt at refining the
    /// read-only analyses has to answer for it.
    #[test]
    fn ce_r2_identity_padding_converges() {
        let a = seq(
            tens(vec![
                scalar_other(),
                prim(SfgGenerator::Copy),
                prim(SfgGenerator::Zero),
            ]),
            tens(vec![
                scalar_other(),
                scalar(),
                prim(SfgGenerator::Zero),
                scalar(),
                prim(SfgGenerator::Discard),
            ]),
        );
        let b = seq(
            seq(
                seq(
                    par(scalar_other(), PropExpr::Identity(1)),
                    tens(vec![
                        PropExpr::Identity(1),
                        prim(SfgGenerator::Copy),
                        prim(SfgGenerator::Zero),
                    ]),
                ),
                tens(vec![
                    PropExpr::Identity(1),
                    scalar(),
                    prim(SfgGenerator::Zero),
                    scalar(),
                    prim(SfgGenerator::Discard),
                ]),
            ),
            par(scalar_other(), PropExpr::Identity(3)),
        );
        assert_eq!(nf(&a), nf(&b), "CE-R2 diverged");
        let once = nf(&a);
        assert_eq!(
            once,
            nf(&from_string_diagram(&once)),
            "CE-R2 not idempotent"
        );
    }

    /// **Step 6½'s interleave guard is load-bearing** (§4.6(a), guard 3). The
    /// same trapped-closed nesting as `trapped_closed_block_extracts`, but the
    /// enclosing component is **marked**: it owns input coordinates 0 and 2 with
    /// a foreign `Discard` at coordinate 1, so its owner-word runs are `A E A`
    /// and `mark_interleaved` marks it.
    ///
    /// Two things make this probe worth its `assert_ne!`. First, marking is the
    /// *only* difference from the converging witness, so the divergence is
    /// attributable to the guard rather than to geometry. Second — and this is
    /// what the #174 review asked for — the guard is verified **decisive**, not
    /// merely present: deleting the `!comps.interleaved[..]` conjuncts from
    /// `column_pair_is_admissible` makes these two writings converge, so every
    /// other admissibility test and the geometry both pass here. That ablation
    /// is the standing argument that this probe is not inert.
    ///
    /// Since the #174 design round the guard survives in Steps 6½ and 7 only:
    /// Step 4(c)'s `η` sift no longer consults it, so a marked component's `η`
    /// *does* now sift (see `marked_component_eta_sifts_and_converges`). What
    /// stays blocked is the block/column transposition, which is what keeps
    /// residual (a) open.
    #[test]
    fn marked_encloser_blocks_the_column_move() {
        let id1 = || PropExpr::Identity(1);
        let nested = seq(
            seq(
                seq(
                    par(id1(), par(prim(SfgGenerator::Discard), id1())),
                    par(id1(), par(prim(SfgGenerator::Zero), id1())),
                ),
                par(id1(), par(prim(SfgGenerator::Discard), id1())),
            ),
            prim(SfgGenerator::Add),
        );
        let free = par(
            closed_block(),
            seq(
                par(id1(), par(prim(SfgGenerator::Discard), id1())),
                prim(SfgGenerator::Add),
            ),
        );
        assert_ne!(
            nf(&nested),
            nf(&free),
            "residual (a) closed unannounced — guard 3 is meant to leave a \
             marked component's blocks alone; if this now converges the §4.6 \
             residual set and the guard's rationale both need revisiting"
        );
    }

    /// Idempotence on every probe witness: re-running `nf` on the expression
    /// rebuilt from the NF reaches the same fixpoint.
    #[test]
    fn probe_witnesses_are_idempotent() {
        let sfg: Vec<PropExpr<Sfg>> = vec![
            seq(sink_block(), source_block()),
            par(sink_block(), source_block()),
            par(source_block(), sink_block()),
            par(closed_block(), scalar()),
            par(scalar(), closed_block()),
            par(par(scalar(), scalar_other()), closed_block()),
            par(closed_block(), par(scalar(), scalar_other())),
            par(closed_block(), long_closed_block()),
            par(long_closed_block(), closed_block()),
            par(
                closed_block(),
                par(long_closed_block(), other_long_closed_block()),
            ),
            par(
                other_long_closed_block(),
                par(long_closed_block(), closed_block()),
            ),
            par(closed_block(), closed_block()),
            seq(closed_block(), closed_block()),
            par(
                par(scalar(), scalar_other()),
                par(closed_block(), long_closed_block()),
            ),
            par(
                par(long_closed_block(), closed_block()),
                par(scalar(), scalar_other()),
            ),
            seq(
                par(prim(SfgGenerator::Zero), PropExpr::Identity(1)),
                par(prim(SfgGenerator::Discard), scalar()),
            ),
        ];
        // The Step 6½ witnesses (issue #174): the three former residuals, the
        // merging wall, the short adjacency interval, the multi-nesting, and the
        // marked encloser that stays put.
        let id1 = || PropExpr::Identity(1);
        let boxed = || {
            seq(
                seq(
                    seq(
                        prim(SfgGenerator::Copy),
                        par(id1(), par(prim(SfgGenerator::Zero), id1())),
                    ),
                    par(id1(), par(prim(SfgGenerator::Discard), id1())),
                ),
                prim(SfgGenerator::Add),
            )
        };
        let columns: Vec<PropExpr<Sfg>> = vec![
            // trapped closed block, nested and free
            boxed(),
            par(
                closed_block(),
                seq(prim(SfgGenerator::Copy), prim(SfgGenerator::Add)),
            ),
            // CE-A: nested solid-headed sink block
            seq(
                seq(
                    par(prim(SfgGenerator::Zero), par(scalar(), id1())),
                    par(id1(), par(prim(SfgGenerator::Discard), id1())),
                ),
                prim(SfgGenerator::Add),
            ),
            // CE-A3: nested output-only solid-tailed block
            seq(
                seq(
                    prim(SfgGenerator::Copy),
                    par(id1(), par(prim(SfgGenerator::Zero), id1())),
                ),
                par(prim(SfgGenerator::Discard), par(scalar(), id1())),
            ),
            // merging wall
            seq(
                seq(
                    seq(
                        seq(
                            prim(SfgGenerator::Copy),
                            par(id1(), par(prim(SfgGenerator::Zero), id1())),
                        ),
                        par(
                            prim(SfgGenerator::Copy),
                            par(prim(SfgGenerator::Discard), id1()),
                        ),
                    ),
                    par(prim(SfgGenerator::Add), id1()),
                ),
                prim(SfgGenerator::Add),
            ),
            // adjacency run shorter than the block span — nested and free
            seq(
                seq(
                    seq(
                        prim(SfgGenerator::Copy),
                        par(id1(), par(prim(SfgGenerator::Zero), id1())),
                    ),
                    par(prim(SfgGenerator::Discard), par(scalar(), id1())),
                ),
                par(prim(SfgGenerator::Discard), id1()),
            ),
            par(
                seq(closed_via_scalar(), prim(SfgGenerator::Discard)),
                seq(
                    prim(SfgGenerator::Copy),
                    par(prim(SfgGenerator::Discard), id1()),
                ),
            ),
            // multi-nesting, doubly nested and half free
            seq(
                seq(
                    par(prim(SfgGenerator::Zero), par(boxed(), id1())),
                    par(id1(), par(prim(SfgGenerator::Discard), id1())),
                ),
                prim(SfgGenerator::Add),
            ),
            par(
                seq(boxed(), prim(SfgGenerator::Discard)),
                seq(
                    par(prim(SfgGenerator::Zero), id1()),
                    prim(SfgGenerator::Add),
                ),
            ),
            // marked encloser (Step 6½'s guard leaves it alone — still a fixpoint)
            seq(
                seq(
                    seq(
                        par(id1(), par(prim(SfgGenerator::Discard), id1())),
                        par(id1(), par(prim(SfgGenerator::Zero), id1())),
                    ),
                    par(id1(), par(prim(SfgGenerator::Discard), id1())),
                ),
                prim(SfgGenerator::Add),
            ),
            // …and its free writing, which the guard keeps it apart from.
            par(
                closed_block(),
                seq(
                    par(id1(), par(prim(SfgGenerator::Discard), id1())),
                    prim(SfgGenerator::Add),
                ),
            ),
            // The *free* writings of every converging column witness — the
            // fixpoint side of each pair, which the corpus used to omit.
            // CE-A free.
            par(
                seq(scalar(), prim(SfgGenerator::Discard)),
                seq(
                    par(prim(SfgGenerator::Zero), id1()),
                    prim(SfgGenerator::Add),
                ),
            ),
            // CE-A3 free.
            par(
                seq(prim(SfgGenerator::Zero), scalar()),
                seq(
                    prim(SfgGenerator::Copy),
                    par(prim(SfgGenerator::Discard), id1()),
                ),
            ),
            // Multi-nested, fully free.
            par(
                closed_block(),
                par(
                    seq(
                        seq(prim(SfgGenerator::Copy), prim(SfgGenerator::Add)),
                        prim(SfgGenerator::Discard),
                    ),
                    seq(
                        par(prim(SfgGenerator::Zero), id1()),
                        prim(SfgGenerator::Add),
                    ),
                ),
            ),
            // Merging wall, free.
            par(
                closed_block(),
                seq(
                    seq(
                        seq(
                            prim(SfgGenerator::Copy),
                            par(prim(SfgGenerator::Copy), id1()),
                        ),
                        par(prim(SfgGenerator::Add), id1()),
                    ),
                    prim(SfgGenerator::Add),
                ),
            ),
            // Fused wide identity, nested and free.
            seq(
                seq(
                    seq(
                        seq(
                            seq(
                                prim(SfgGenerator::Copy),
                                par(prim(SfgGenerator::Copy), id1()),
                            ),
                            par(PropExpr::Identity(2), par(prim(SfgGenerator::Zero), id1())),
                        ),
                        par(
                            PropExpr::Identity(2),
                            par(prim(SfgGenerator::Discard), id1()),
                        ),
                    ),
                    par(prim(SfgGenerator::Add), id1()),
                ),
                prim(SfgGenerator::Add),
            ),
            // Braid-guarded encloser, nested and free.
            seq(
                seq(
                    seq(
                        PropExpr::Braid(1, 1),
                        par(id1(), par(prim(SfgGenerator::Zero), id1())),
                    ),
                    par(id1(), par(prim(SfgGenerator::Discard), id1())),
                ),
                prim(SfgGenerator::Add),
            ),
            par(
                closed_block(),
                seq(PropExpr::Braid(1, 1), prim(SfgGenerator::Add)),
            ),
        ];
        for e in sfg.iter().chain(columns.iter()) {
            let once = nf(e);
            let twice = nf(&from_string_diagram(&once));
            assert_eq!(once, twice, "nf not idempotent on {e:?}");
        }

        let f: PropExpr<TestSig> = PropExpr::Generator(TestSig::F);
        let g: PropExpr<TestSig> = PropExpr::Generator(TestSig::G);
        let eta: PropExpr<TestSig> = PropExpr::Generator(TestSig::Eta);
        let eps: PropExpr<TestSig> = PropExpr::Generator(TestSig::Eps);
        let sc = |k: u8| PropExpr::Generator(TestSig::Sc(k));
        let test_sig: Vec<PropExpr<TestSig>> = vec![
            seq(eps.clone(), eta.clone()),
            par(eta.clone(), eps),
            par(f.clone(), par(eta.clone(), g.clone())),
            seq(
                par(f, g),
                par(PropExpr::Identity(1), par(eta, PropExpr::Identity(1))),
            ),
            par(sc(2), sc(1)),
            par(sc(3), par(sc(1), sc(2))),
        ];
        for e in &test_sig {
            let once = nf(e);
            let twice = nf(&from_string_diagram(&once));
            assert_eq!(once, twice, "nf not idempotent on {e:?}");
        }
    }

    /// **Which witnesses Step 6½ is actually load-bearing for.**
    ///
    /// §4.5's "What actually depends on the pass" and the CHANGELOG's
    /// residual-(c)/(d) entry both make a *counted* claim about the column pass,
    /// and until this probe existed the only way to check it was to comment the
    /// pass out by hand — which is how the two came to disagree (§4.5 said five
    /// witnesses, the CHANGELOG said four).
    ///
    /// Each column-family pair runs through `nf` and through
    /// `nf_without_column_pass` (the `internal-ablation` hook), pinning the
    /// split: five pairs converge **only** with the pass, two converge either
    /// way. The `assert_ne!` half is the load-bearing one — it fails the day the
    /// column pass stops mattering for a witness that is documented as needing
    /// it, whether because the pass regressed or because some other pass grew to
    /// subsume it. Either way the prose in §4.5 would have gone stale silently.
    ///
    /// The two `needs_pass == false` rows are the attribution correction of
    /// record: CE-A3 was filed as a column residual and is not one (it converges
    /// by the free-site retirement), and the adjacency-interval probe is a
    /// nested-block convergence regression rather than a column witness.
    ///
    /// The pairs are rebuilt here rather than shared with the named tests above,
    /// which keeps this module append-only; folding them into shared
    /// `*_pair()` helpers is worth doing once #174's doc-staleness edits land.
    #[cfg(feature = "internal-probes")]
    mod column_pass_ablation {
        use super::*;
        use catgraph_applied::prop::presentation::smc_nf::nf_without_column_pass;

        fn id1() -> PropExpr<Sfg> {
            PropExpr::Identity(1)
        }

        /// `(name, nested writing, free writing, does Step 6½ decide it?)`
        fn column_family() -> Vec<(&'static str, PropExpr<Sfg>, PropExpr<Sfg>, bool)> {
            let boxed = || {
                seq(
                    seq(
                        seq(
                            prim(SfgGenerator::Copy),
                            par(id1(), par(prim(SfgGenerator::Zero), id1())),
                        ),
                        par(id1(), par(prim(SfgGenerator::Discard), id1())),
                    ),
                    prim(SfgGenerator::Add),
                )
            };
            let copy_add_tail = || {
                seq(
                    seq(
                        seq(
                            prim(SfgGenerator::Copy),
                            par(prim(SfgGenerator::Copy), id1()),
                        ),
                        par(prim(SfgGenerator::Add), id1()),
                    ),
                    prim(SfgGenerator::Add),
                )
            };
            vec![
                (
                    "trapped_closed_block_extracts",
                    boxed(),
                    par(
                        closed_block(),
                        seq(prim(SfgGenerator::Copy), prim(SfgGenerator::Add)),
                    ),
                    true,
                ),
                (
                    "nested_sink_block_converges_with_free_writing",
                    seq(
                        seq(
                            par(prim(SfgGenerator::Zero), par(scalar(), id1())),
                            par(id1(), par(prim(SfgGenerator::Discard), id1())),
                        ),
                        prim(SfgGenerator::Add),
                    ),
                    par(
                        seq(scalar(), prim(SfgGenerator::Discard)),
                        seq(
                            par(prim(SfgGenerator::Zero), id1()),
                            prim(SfgGenerator::Add),
                        ),
                    ),
                    true,
                ),
                (
                    "column_move_crosses_a_merging_wall",
                    seq(
                        seq(
                            seq(
                                seq(
                                    prim(SfgGenerator::Copy),
                                    par(id1(), par(prim(SfgGenerator::Zero), id1())),
                                ),
                                par(
                                    prim(SfgGenerator::Copy),
                                    par(prim(SfgGenerator::Discard), id1()),
                                ),
                            ),
                            par(prim(SfgGenerator::Add), id1()),
                        ),
                        prim(SfgGenerator::Add),
                    ),
                    par(closed_block(), copy_add_tail()),
                    true,
                ),
                (
                    "column_move_crosses_a_fused_wide_identity",
                    seq(
                        seq(
                            seq(
                                seq(
                                    seq(
                                        prim(SfgGenerator::Copy),
                                        par(prim(SfgGenerator::Copy), id1()),
                                    ),
                                    par(
                                        PropExpr::Identity(2),
                                        par(prim(SfgGenerator::Zero), id1()),
                                    ),
                                ),
                                par(
                                    PropExpr::Identity(2),
                                    par(prim(SfgGenerator::Discard), id1()),
                                ),
                            ),
                            par(prim(SfgGenerator::Add), id1()),
                        ),
                        prim(SfgGenerator::Add),
                    ),
                    par(closed_block(), copy_add_tail()),
                    true,
                ),
                (
                    "multi_nested_blocks_extract",
                    seq(
                        seq(
                            par(prim(SfgGenerator::Zero), par(boxed(), id1())),
                            par(id1(), par(prim(SfgGenerator::Discard), id1())),
                        ),
                        prim(SfgGenerator::Add),
                    ),
                    par(
                        closed_block(),
                        par(
                            seq(
                                seq(prim(SfgGenerator::Copy), prim(SfgGenerator::Add)),
                                prim(SfgGenerator::Discard),
                            ),
                            seq(
                                par(prim(SfgGenerator::Zero), id1()),
                                prim(SfgGenerator::Add),
                            ),
                        ),
                    ),
                    true,
                ),
                (
                    "nested_source_block_converges_with_free_writing",
                    seq(
                        seq(
                            prim(SfgGenerator::Copy),
                            par(id1(), par(prim(SfgGenerator::Zero), id1())),
                        ),
                        par(prim(SfgGenerator::Discard), par(scalar(), id1())),
                    ),
                    par(
                        seq(prim(SfgGenerator::Zero), scalar()),
                        seq(
                            prim(SfgGenerator::Copy),
                            par(prim(SfgGenerator::Discard), id1()),
                        ),
                    ),
                    false,
                ),
                (
                    "column_interval_is_the_adjacency_run_not_the_block_span",
                    seq(
                        seq(
                            seq(
                                prim(SfgGenerator::Copy),
                                par(id1(), par(prim(SfgGenerator::Zero), id1())),
                            ),
                            par(prim(SfgGenerator::Discard), par(scalar(), id1())),
                        ),
                        par(prim(SfgGenerator::Discard), id1()),
                    ),
                    par(
                        seq(closed_via_scalar(), prim(SfgGenerator::Discard)),
                        seq(
                            prim(SfgGenerator::Copy),
                            par(prim(SfgGenerator::Discard), id1()),
                        ),
                    ),
                    false,
                ),
            ]
        }

        #[test]
        fn column_pass_decides_exactly_the_five_documented_witnesses() {
            let mut decided_by_the_pass = Vec::new();
            for (name, nested, free, needs_pass) in column_family() {
                assert_eq!(
                    nf(&nested),
                    nf(&free),
                    "{name}: the shipped engine must converge this pair"
                );
                let ablated_converges =
                    nf_without_column_pass(&nested) == nf_without_column_pass(&free);
                if !ablated_converges {
                    decided_by_the_pass.push(name);
                }
                assert_eq!(
                    ablated_converges,
                    !needs_pass,
                    "{name}: Step 6½ attribution changed — the witness is documented as \
                     {} the column pass, but ablating the pass leaves it {}. Update \
                     §4.5's \"What actually depends on the pass\" and the CHANGELOG \
                     count together with this table.",
                    if needs_pass { "needing" } else { "NOT needing" },
                    if ablated_converges {
                        "converging"
                    } else {
                        "diverging"
                    },
                );
            }
            assert_eq!(
                decided_by_the_pass.len(),
                5,
                "§4.5 and the CHANGELOG both quote a count of witnesses the column \
                 pass decides; measured {decided_by_the_pass:?}"
            );
        }
    }
}
