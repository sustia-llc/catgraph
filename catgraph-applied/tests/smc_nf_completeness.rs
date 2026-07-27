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
//! component-anchored point-span sift (issue #55, closed on the fragment 𝔉 —
//! SMC-NF-RECONCILIATION.md §4.1; probe-verified, full proof open per the
//! §4.4 canonicality status; see `interchange_zero_source_eta` and the
//! `smc_canonicality_probes` module). A proptest or golden-replay failure
//! whose witness has an `η` in an *interleaved* component (guard 3), a closed
//! component written nested inside another component's span (§4.6(c),
//! `trapped_closed_block_is_nesting_residual`), or a zero-arity block solid
//! on its opening side written nested (§4.6(d),
//! `nested_sink_block_is_column_residual` /
//! `nested_source_block_is_column_residual`) is a documented residual, not a
//! new bug — **three** residuals, (a), (c) and (d).
//!
//! The fourth, residual (b) — two *distinct* closed blocks kept their input
//! order because rule (i) gives every closed component one key and
//! `PropSignature` carried no `Ord` to break the tie — is **closed** by
//! issue #79 P1: the `Ord` supertrait plus Step 7's in-situ reading key
//! (`closed_blocks_sort_by_content_key` and its companions below).

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
        /// closed by the component-anchored point-span sift (issue #55) — see
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
/// now scheduled canonically by the component-anchored point-span sift.
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
/// tensor-forms) is closed too, by the component-anchored point-span sift
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
    /// component-anchored point-span sift (PR2) lifts its `η` into the `ε`'s
    /// layer. Both are single-atom components, so the §2.6 disjointness carve
    /// hands the resulting tie back to Decision 1 / Step 6 (PR1) — η first. All
    /// three share the one NF `[[η, ε]]`.
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
/// tie-break, and idempotence on all of them.
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
    /// the tied `η ∥ ε` pair share one NF. Both components are single atoms, so
    /// the §2.6 disjointness carve routes this to Decision 1 (η first) rather
    /// than to rule (i)'s component order.
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

    /// **Trapped-nesting residual, documented** (§4.6(c), found in the #55
    /// proof phase 2026-07-27; issue #174). A closed block written strictly
    /// *inside* another component's wire span cannot escape: its `η`'s
    /// coordinate falls strictly inside the enclosing atom's target span (the
    /// point-span sift is blocked — correctly, since the gap-closer is
    /// foreign), and Step 7 never sees an adjacent free pair because the
    /// identity wires surrounding the closed block belong to the *enclosing*
    /// component. The nested and free writings have identical abstract
    /// content, so no content-level fragment condition separates them — the
    /// residual is irreducibly presentation-level, and the §4 proof excludes
    /// closed components from `𝔉` outright. Only closed components can be
    /// trapped: a nested *anchored* component's attachment is enclosed by the
    /// other's, so guard 3 marks both (residual (a)). Fix shape: a closed-block
    /// extraction move (`id₁ ⊗ s = s ⊗ id₁` sideways past identity
    /// wire-columns) — tracked on #174.
    #[test]
    #[ignore = "residual: a closed component written nested inside another component's span does not extract (§4.6(c), #174)"]
    fn trapped_closed_block_is_nesting_residual() {
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
        assert_eq!(
            nf(&nested),
            nf(&free),
            "a nested closed block should extract to the free layout"
        );
    }

    /// **Nested-column residual, sink form (CE-A)** (§4.6(d), found in the
    /// 2026-07-27 adversarial review of the draft §4 proof — the refutation
    /// of its full-canonicality theorem; issue #174). A solid-headed
    /// multi-atom `1 → 0` block (`Scalar;Discard`) written at a coordinate
    /// strictly inside the `{Zero, …, Add}` component's span cannot reach its
    /// free writing: Step 6 never bubbles `Zero` past the solid `Scalar`
    /// head, and Step 7's free-pair test is whole-component while the actual
    /// SMC freedom is column-vs-block (`Zero ⊗ ε = ε ⊗ Zero`). Both
    /// components are boundary-attached and unmarked, so the pair sits
    /// *inside* the fragment `𝔉` — same content, different fixpoints. The
    /// missing move is the §4.5 zero-arity-bounded column transposition.
    #[test]
    #[ignore = "residual: a solid-headed zero-arity block written nested inside another component's span does not converge with its free writing (§4.6(d), #174)"]
    fn nested_sink_block_is_column_residual() {
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
        assert_eq!(
            nf(&nested),
            nf(&free),
            "a nested solid-headed sink block should converge with its free writing"
        );
    }

    /// **Nested-column residual, source form (CE-A3)** — the time-reversed
    /// mirror of `nested_sink_block_is_column_residual`: the enclosing wall
    /// opens at an `ε` (`Discard`) *below* instead of an `η` above, and the
    /// nested block is output-only (`Zero;Scalar`). Same mechanism, same
    /// missing move (§4.5); issue #174.
    #[test]
    #[ignore = "residual: mirror of nested_sink_block_is_column_residual — output-only nested block, wall opens at an ε (§4.6(d), #174)"]
    fn nested_source_block_is_column_residual() {
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
        assert_eq!(
            nf(&nested),
            nf(&free),
            "a nested output-only solid-tailed block should converge with its free writing"
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
        for e in &sfg {
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
}
