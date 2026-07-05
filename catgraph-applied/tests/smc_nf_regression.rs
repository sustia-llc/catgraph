//! Paper-cited SMC-coherence regression tests for the Layer 1 NF.
//!
//! Source: reconciled from the 4 dpcs notes (JS-I, JS-II, Selinger, JS-Braided).
//!
//! Each test's docstring cites the primary paper + section/theorem. When an
//! equation is justified by multiple papers, the primary citation is chosen
//! per the reconciliation's "paper coverage matrix" (§3 step table).

use catgraph_applied::prop::presentation::smc_nf::{Atom, StringDiagram, nf};
use catgraph_applied::prop::{PropExpr, PropSignature};

/// Shared test signature covering all four papers' tested arities:
/// `F, G : 1 → 1`, `H : 2 → 1`, `Eps : 1 → 0`, `Eta : 0 → 1`. `H` exercises
/// multi-wire generators in interchange/braiding patterns.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum TestSig {
    F,
    G,
    H,
    Eps,
    Eta,
}

impl PropSignature for TestSig {
    fn source(&self) -> usize {
        match self {
            TestSig::F | TestSig::G | TestSig::Eps => 1,
            TestSig::H => 2,
            TestSig::Eta => 0,
        }
    }
    fn target(&self) -> usize {
        match self {
            TestSig::F | TestSig::G | TestSig::H | TestSig::Eta => 1,
            TestSig::Eps => 0,
        }
    }
}

/// True if any layer holds a `Braid(m,n)` with `m+n > 2` (a wide braid the NF
/// invariant forbids).
fn has_wide_braid(sd: &StringDiagram<TestSig>) -> bool {
    sd.layers.iter().any(|l| {
        l.atoms
            .iter()
            .any(|a| matches!(a, Atom::Braid(m, n) if m + n > 2))
    })
}

/// True if any layer holds both a `Braid` and a `Generator` (a mixed layer the
/// NF invariant forbids).
fn has_mixed_layer(sd: &StringDiagram<TestSig>) -> bool {
    sd.layers.iter().any(|l| {
        l.atoms.iter().any(|a| matches!(a, Atom::Braid(_, _)))
            && l.atoms.iter().any(|a| matches!(a, Atom::Generator(_)))
    })
}

// ============================================================================
// Joyal-Street Part I (1991) — Geometry of Tensor Calculus I
// ============================================================================

mod joyal_street_part_i_regression {
    use super::*;
    use PropExpr::{Braid, Compose, Generator, Identity, Tensor};

    /// JS-I Ch 1 Prop 1.1 (p. 66): rectangle-cover independence —
    /// `v(Γ) = v(Γ[u,b]) ∘ v(Γ[a,u])`. Strict associativity of `;` is
    /// absorbed into `Vec<Layer<G>>` concatenation.
    #[test]
    fn ch1_prop_1_1_compose_associativity() {
        let f: PropExpr<TestSig> = Generator(TestSig::F);
        let g: PropExpr<TestSig> = Generator(TestSig::G);
        let h: PropExpr<TestSig> = Generator(TestSig::F); // 1 → 1 (reuse)
        let lhs = Compose(
            Box::new(Compose(Box::new(f.clone()), Box::new(g.clone()))),
            Box::new(h.clone()),
        );
        let rhs = Compose(Box::new(f), Box::new(Compose(Box::new(g), Box::new(h))));
        assert_eq!(nf(&lhs), nf(&rhs));
    }

    /// JS-I Ch 1 §3 p. 65 (invertible diagram `v(Γ) = id`) + Prop 1.1 p. 66:
    /// an Identity layer absorbs into either neighbour under composition.
    /// Covers both left and right unitors for `;`.
    #[test]
    fn ch1_invertible_left_right_unitor() {
        let f: PropExpr<TestSig> = Generator(TestSig::F);
        let lhs_left = Compose(Box::new(Identity(1)), Box::new(f.clone()));
        let lhs_right = Compose(Box::new(f.clone()), Box::new(Identity(1)));
        assert_eq!(nf(&lhs_left), nf(&f));
        assert_eq!(nf(&lhs_right), nf(&f));
    }

    /// JS-I Ch 1 §4 Thm 1.2 (p. 71): `𝔽(𝒟)` is free on 𝒟, so bifunctoriality
    /// of `⊗` holds: `(f ⊗ g) ; (h ⊗ k) = (f ; h) ⊗ (g ; k)`. The NF's
    /// pad-and-zip construction (paper §4 p. 69–70) produces identical
    /// `Vec<Layer<G>>` for both sides.
    ///
    /// **This is the equation the plain `apply_smc_rules` bottom-up
    /// rewriter could not canonicalize** — the motivating failure mode for
    /// the Joyal-Street rewrite.
    #[test]
    fn ch1_thm_1_2_s4_interchange() {
        let f: PropExpr<TestSig> = Generator(TestSig::F);
        let g: PropExpr<TestSig> = Generator(TestSig::G);
        let h: PropExpr<TestSig> = Generator(TestSig::F); // 1 → 1 (reuse)
        let k: PropExpr<TestSig> = Generator(TestSig::G); // 1 → 1 (reuse)
        let lhs = Compose(
            Box::new(Tensor(Box::new(f.clone()), Box::new(g.clone()))),
            Box::new(Tensor(Box::new(h.clone()), Box::new(k.clone()))),
        );
        let rhs = Tensor(
            Box::new(Compose(Box::new(f), Box::new(h))),
            Box::new(Compose(Box::new(g), Box::new(k))),
        );
        assert_eq!(nf(&lhs), nf(&rhs));
    }

    /// JS-I Ch 1 §1 p. 57 (strict monoidal unit): `Identity(0)` is the unit
    /// for `⊗`, so `id_0 ⊗ f = f = f ⊗ id_0` up to the strict-skeleton
    /// identification the paper makes on p. 58 (bracket-cliques).
    #[test]
    fn ch1_s1_strict_unit() {
        let f: PropExpr<TestSig> = Generator(TestSig::F);
        let lhs_left = Tensor(Box::new(Identity(0)), Box::new(f.clone()));
        let lhs_right = Tensor(Box::new(f.clone()), Box::new(Identity(0)));
        assert_eq!(nf(&lhs_left), nf(&f));
        assert_eq!(nf(&lhs_right), nf(&f));
    }

    /// JS-I Ch 2 §1 axiom (S) p. 73: `c_{B,A} ∘ c_{A,B} = 1_{A⊗B}`.
    /// Symmetry is self-inverse. `reduce_involution` must collapse
    /// `Braid(1,1) ; Braid(1,1)` to `Identity(2)`.
    #[test]
    fn ch2_s1_axiom_s_braid_involution() {
        let lhs: PropExpr<TestSig> = Compose(Box::new(Braid(1, 1)), Box::new(Braid(1, 1)));
        let rhs: PropExpr<TestSig> = Identity(2);
        assert_eq!(nf(&lhs), nf(&rhs));
    }

    /// JS-I Ch 2 Thm 2.2 (p. 79): diagram isomorphism implies equal value
    /// up to induced dom/cod permutations. Concretely — braid naturality
    /// `σ_{1,1} ; (g ⊗ f) = (f ⊗ g) ; σ_{1,1}` for `f, g : 1→1`.
    /// Both sides are abstract-diagram-isomorphic; the NF must put the
    /// braid in a canonical position (anchored form per Cor 2.3 p. 80).
    #[test]
    fn ch2_thm_2_2_braid_naturality() {
        let f: PropExpr<TestSig> = Generator(TestSig::F);
        let g: PropExpr<TestSig> = Generator(TestSig::G);
        let lhs = Compose(
            Box::new(Braid(1, 1)),
            Box::new(Tensor(Box::new(g.clone()), Box::new(f.clone()))),
        );
        let rhs = Compose(
            Box::new(Tensor(Box::new(f), Box::new(g))),
            Box::new(Braid(1, 1)),
        );
        assert_eq!(nf(&lhs), nf(&rhs));
    }

    /// JS-I Ch 2 Thm 2.3 (p. 81): `𝔽_s(𝒟)` is free symmetric on 𝒟.
    /// Consequence: `σ_{2,1} = (id_1 ⊗ σ_{1,1}) ; (σ_{1,1} ⊗ id_1)`.
    ///
    /// `σ_{2,1}` takes `[a, b, c]` (m=2 block `[a,b]`, n=1 block `[c]`)
    /// to `[c, a, b]` (n-block first). Trace of the RHS:
    ///
    /// - `(id_1 ⊗ σ_{1,1})` on `[a, b, c]` → `[a, c, b]` (id passes a; σ swaps b↔c).
    /// - `(σ_{1,1} ⊗ id_1)` on `[a, c, b]` → `[c, a, b]` (σ swaps a↔c; id passes b).
    ///
    /// — matches `σ_{2,1}` ✓.
    ///
    /// Paper-anchor note: this is JS-Braided axiom (B2), `c_{U⊗V, W} =
    /// (σ_{U,W} ⊗ 1_V) ∘ (1_U ⊗ σ_{V,W})`, with the composition in
    /// Rust's `;` (forward) direction; (B1) would give the mirror
    /// `σ_{1,2}` decomposition.
    #[test]
    fn ch2_thm_2_3_symmetry_on_larger_tensors() {
        let lhs: PropExpr<TestSig> = Braid(2, 1);
        let rhs: PropExpr<TestSig> = Compose(
            // inner 1 (runs first): id_1 ⊗ σ_{1,1}
            Box::new(Tensor(Box::new(Identity(1)), Box::new(Braid(1, 1)))),
            // inner 2 (runs second): σ_{1,1} ⊗ id_1
            Box::new(Tensor(Box::new(Braid(1, 1)), Box::new(Identity(1)))),
        );
        assert_eq!(nf(&lhs), nf(&rhs));
    }

    /// JS-I Ch 1 §4 Thm 1.2 (p. 71) reassoc-then-interchange motivating case:
    /// the rewriter needs to re-associate a
    /// Tensor tree to *expose* the bifunctoriality equation, then apply
    /// interchange. The plain pre-pass could only do one of those at a
    /// time; the Joyal-Street NF closes both in one shot because
    /// `pad_and_zip` is associative and interchange is structural.
    ///
    /// Setup: ε : 1→0 discarded alongside a braid-and-identity tensor.
    /// - LHS = `ε ⊗ (σ_{1,1} ⊗ id_1)` (deeply right-assoc)
    /// - RHS = `(ε ⊗ id_3) ; (σ_{1,1} ⊗ id_1)` (compose form)
    #[test]
    fn ch1_thm_1_2_s4_reassoc_exposes_interchange() {
        let eps: PropExpr<TestSig> = Generator(TestSig::Eps);
        let braid_plus_id = Tensor(Box::new(Braid(1, 1)), Box::new(Identity(1)));
        let lhs = Tensor(Box::new(eps.clone()), Box::new(braid_plus_id.clone()));
        let rhs = Compose(
            Box::new(Tensor(Box::new(eps), Box::new(Identity(3)))),
            Box::new(braid_plus_id),
        );
        assert_eq!(nf(&lhs), nf(&rhs));
    }
}

// ============================================================================
// Joyal-Street Part II — surgery / conjugation patterns
// ============================================================================

mod joyal_street_ii_regression {
    use super::*;
    use PropExpr::{Braid, Compose, Generator, Identity, Tensor};

    /// JS-II Thm 1.2.2 (3D-deformation) + Thm 1.2.3 (surgery, p. 6–7):
    /// `σ ; (f ⊗ id_1) ; σ = id_1 ⊗ f` via surgery — replace the
    /// `σ ; σ` sandwich with `id_2` inside a rectangle.
    ///
    /// Closure of this pattern is exactly the round-1 adversarial
    /// finding (the braid-conjugation gap) — captured cleanly by `nf`
    /// applying involution + naturality in tandem.
    #[test]
    fn braid_sandwich_is_identity_tensor() {
        let f: PropExpr<TestSig> = Generator(TestSig::F);
        let id_1: PropExpr<TestSig> = Identity(1);
        let sigma_11: PropExpr<TestSig> = Braid(1, 1);

        let lhs = Compose(
            Box::new(Compose(
                Box::new(sigma_11.clone()),
                Box::new(Tensor(Box::new(f.clone()), Box::new(id_1.clone()))),
            )),
            Box::new(sigma_11),
        );
        let rhs = Tensor(Box::new(id_1), Box::new(f));
        assert_eq!(nf(&lhs), nf(&rhs));
    }

    /// JS-II Thm 1.1.2 (planar deformation, p. 3–4): two progressive
    /// plane diagrams that differ only by adding an empty horizontal slice
    /// have equal value. `id_1 ; f ; id_1 = f`.
    #[test]
    fn planar_identity_layer_coalesce() {
        let f: PropExpr<TestSig> = Generator(TestSig::F);
        let id_1: PropExpr<TestSig> = Identity(1);
        let lhs = Compose(
            Box::new(Compose(Box::new(id_1.clone()), Box::new(f.clone()))),
            Box::new(id_1),
        );
        assert_eq!(nf(&lhs), nf(&f));
    }
}

// ============================================================================
// Selinger — Survey of Graphical Languages (2011)
// ============================================================================

mod selinger_2011_regression {
    use super::*;
    use PropExpr::{Braid, Compose, Generator, Identity, Tensor};

    /// Selinger §2 Technicalities (p. 7) + §3 Monoidal signatures (p. 12):
    /// "the labels can be viewed as formal symbols." Generators are opaque
    /// in the free category — swapping two distinct generators across the
    /// same wire structure must NOT normalize them to the same form.
    #[test]
    fn smc_generators_are_uninterpreted_black_boxes() {
        let sd_f: PropExpr<TestSig> = Generator(TestSig::F);
        let sd_g: PropExpr<TestSig> = Generator(TestSig::G);
        assert_ne!(
            nf(&sd_f),
            nf(&sd_g),
            "Selinger §2: formal symbols are distinguished in the free category",
        );
    }

    /// Selinger §3.5 Thm 3.12 (p. 17) vs §3.3 Thm 3.7: in a **braided** MC
    /// two crossings of the same two wires are NOT equal to the identity
    /// (§3.3 diagram p. 15 middle: `c_{A,B} ∘ c_{B,A} ≠ id_{A⊗B}`). In an
    /// **SMC** they ARE equal (§3.5 p. 17). This test specifically
    /// exercises the SMC-strengthening over braided.
    #[test]
    fn smc_two_crossings_cancel_but_braided_would_not() {
        let double_crossing: PropExpr<TestSig> =
            Compose(Box::new(Braid(1, 1)), Box::new(Braid(1, 1)));
        let id_2: PropExpr<TestSig> = Identity(2);
        assert_eq!(
            nf(&double_crossing),
            nf(&id_2),
            "Selinger §3.5: SMC self-inverse braid; Thm 3.12 coherence witnessing",
        );
    }

    /// Selinger Table 2 (p. 10) interchange law:
    /// `(id ⊗ g) ∘ (f ⊗ id) = (f ⊗ id) ∘ (id ⊗ g)` — quoted on p. 10 as
    /// a consequence of bifunctoriality. With `f, g : 1→1`, this is the
    /// 2-wire independent-parallel-generators interchange. The NF must
    /// identify both sides.
    #[test]
    fn smc_bifunctoriality_interchange() {
        let f = || Generator::<TestSig>(TestSig::F);
        let g = || Generator::<TestSig>(TestSig::G);
        let lhs = Compose(
            Box::new(Tensor(Box::new(f()), Box::new(Identity(1)))),
            Box::new(Tensor(Box::new(Identity(1)), Box::new(g()))),
        );
        let rhs = Compose(
            Box::new(Tensor(Box::new(Identity(1)), Box::new(g()))),
            Box::new(Tensor(Box::new(f()), Box::new(Identity(1)))),
        );
        assert_eq!(
            nf(&lhs),
            nf(&rhs),
            "Selinger Table 2 (p. 10): bifunctoriality consequence, holds in every MC",
        );
    }
}

// ============================================================================
// Joyal-Street — Braided Tensor Categories (1993) — symmetric quotient
// ============================================================================

mod joyal_street_braided_regression {
    use super::*;
    use PropExpr::{Braid, Compose, Generator, Identity, Tensor};

    /// JS-Braided Example 2.1 (A1) p. 35: the Artin 3-strand relation
    /// `s_i s_{i+1} s_i = s_{i+1} s_i s_{i+1}` — Reidemeister III in the
    /// isotopy picture. Holds in the braid group `𝔅_3` directly, and thus
    /// in the symmetric quotient `𝔖_3`.
    #[test]
    fn test_yang_baxter() {
        let lhs: PropExpr<TestSig> = Compose(
            Box::new(Compose(
                Box::new(Tensor(Box::new(Braid(1, 1)), Box::new(Identity(1)))),
                Box::new(Tensor(Box::new(Identity(1)), Box::new(Braid(1, 1)))),
            )),
            Box::new(Tensor(Box::new(Braid(1, 1)), Box::new(Identity(1)))),
        );
        let rhs: PropExpr<TestSig> = Compose(
            Box::new(Compose(
                Box::new(Tensor(Box::new(Identity(1)), Box::new(Braid(1, 1)))),
                Box::new(Tensor(Box::new(Braid(1, 1)), Box::new(Identity(1)))),
            )),
            Box::new(Tensor(Box::new(Identity(1)), Box::new(Braid(1, 1)))),
        );
        assert_eq!(nf(&lhs), nf(&rhs));
    }

    /// JS-Braided axiom (B2), Def 2.1 p. 33: `c_{U⊗V, W} = (σ_{U,W} ⊗ 1_V)
    /// ∘ (1_U ⊗ σ_{V,W})` — the source-side m+n splitting of a braid. In
    /// forward composition: `σ_{2,1} = (id_1 ⊗ σ_{1,1}) ; (σ_{1,1} ⊗ id_1)`.
    /// Traces to `[c, a, b]` on input `[a, b, c]` ✓.
    /// (Paper-anchor note corrected to (B2) per the reconciliation.)
    #[test]
    fn test_hexagon_sigma_on_tensor() {
        let lhs: PropExpr<TestSig> = Braid(2, 1);
        let rhs: PropExpr<TestSig> = Compose(
            // inner 1 (runs first): id_1 ⊗ σ_{1,1}
            Box::new(Tensor(Box::new(Identity(1)), Box::new(Braid(1, 1)))),
            // inner 2 (runs second): σ_{1,1} ⊗ id_1
            Box::new(Tensor(Box::new(Braid(1, 1)), Box::new(Identity(1)))),
        );
        assert_eq!(nf(&lhs), nf(&rhs));
    }

    /// JS-Braided (A1) / Example 2.1: non-trivial 3-strand permutation
    /// exercise — verify that a sequence of two different `Braid(1,1)`
    /// placements produces a diagram structurally distinct from `id_3`
    /// (i.e., is not normalized away to the identity), AND that its
    /// round-trip through the inverse sequence IS `id_3` (two applications
    /// of (S)).
    #[test]
    fn test_braid_interaction_with_identity() {
        let forward: PropExpr<TestSig> = Compose(
            Box::new(Tensor(Box::new(Identity(1)), Box::new(Braid(1, 1)))),
            Box::new(Tensor(Box::new(Braid(1, 1)), Box::new(Identity(1)))),
        );
        let id_3: PropExpr<TestSig> = Identity(3);
        assert_ne!(nf(&forward), nf(&id_3));

        let backward: PropExpr<TestSig> = Compose(
            Box::new(Tensor(Box::new(Braid(1, 1)), Box::new(Identity(1)))),
            Box::new(Tensor(Box::new(Identity(1)), Box::new(Braid(1, 1)))),
        );
        let round_trip: PropExpr<TestSig> = Compose(Box::new(forward), Box::new(backward));
        assert_eq!(nf(&round_trip), nf(&id_3));
    }

    /// JS-Braided Example 6.1 (p. 66): "symmetric tensor categories are
    /// balanced." Exactly the statement that squares of transpositions
    /// collapse. `(σ_{1,1} ; σ_{1,1}) ⊗ id_1 = id_3` via (S) then
    /// `coalesce_identity_layers`.
    #[test]
    fn test_symmetric_collapse_3_strands() {
        let two_swaps: PropExpr<TestSig> = Compose(Box::new(Braid(1, 1)), Box::new(Braid(1, 1)));
        let with_id = Tensor(Box::new(two_swaps), Box::new(Identity(1)));
        let id_3: PropExpr<TestSig> = Identity(3);
        assert_eq!(nf(&with_id), nf(&id_3));
    }

    /// JS-Braided p. 36 picture — mirror of `ch2_thm_2_2_braid_naturality`:
    /// `(f ⊗ g) ; σ_{1,1} = σ_{1,1} ; (g ⊗ f)`. Confirms naturality in the
    /// other direction (generators on the left of the braid).
    #[test]
    fn test_braid_naturality_right() {
        let f = || Generator::<TestSig>(TestSig::F);
        let g = || Generator::<TestSig>(TestSig::G);
        let lhs = Compose(
            Box::new(Tensor(Box::new(f()), Box::new(g()))),
            Box::new(Braid(1, 1)),
        );
        let rhs = Compose(
            Box::new(Braid(1, 1)),
            Box::new(Tensor(Box::new(g()), Box::new(f()))),
        );
        assert_eq!(nf(&lhs), nf(&rhs));
    }
}

// ============================================================================
// Issue #14 — topological-layer-order (Step 4(c)) and its σ;σ-band companion
// ============================================================================

mod issue_14_topological_layer_order {
    use super::*;
    use PropExpr::{Braid, Compose, Generator, Identity, Tensor};

    /// Right-associate a tensor of atoms into a single layer expression.
    fn layer(atoms: Vec<PropExpr<TestSig>>) -> PropExpr<TestSig> {
        let mut it = atoms.into_iter().rev();
        let last = it.next().expect("non-empty layer");
        it.fold(last, |acc, x| Tensor(Box::new(x), Box::new(acc)))
    }
    /// Compose a sequence of layers top-to-bottom.
    fn seq(layers: Vec<PropExpr<TestSig>>) -> PropExpr<TestSig> {
        let mut it = layers.into_iter();
        let first = it.next().expect("non-empty sequence");
        it.fold(first, |a, b| Compose(Box::new(a), Box::new(b)))
    }

    /// JS-I Ch 1 §4 Thm 1.2 p. 71 (bifunctoriality / interchange), issue #14
    /// C2 scheduling witness: the same morphism with independent atoms placed
    /// into different layer schedulings must reach one normal form. Exact
    /// example from the C2 gap note:
    ///   `[id_2, F] ; [F, id_1, F] ; [id_2, F]`  ==
    ///   `[F, id_1, F] ; [id_2, F] ; [id_2, F]`.
    /// Closed by `topological_layer_order` sifting each generator to its
    /// earliest admissible layer.
    #[test]
    fn c2_scheduling_witness_converges() {
        let f = || Generator(TestSig::F);
        let lhs = seq(vec![
            layer(vec![Identity(2), f()]),
            layer(vec![f(), Identity(1), f()]),
            layer(vec![Identity(2), f()]),
        ]);
        let rhs = seq(vec![
            layer(vec![f(), Identity(1), f()]),
            layer(vec![Identity(2), f()]),
            layer(vec![Identity(2), f()]),
        ]);
        assert_eq!(nf(&lhs), nf(&rhs));
    }

    /// JS-I Ch 1 §4 Thm 1.2 p. 71: a target-0 sink (`ε : 1 → 0`) has a
    /// non-empty source span, so it sifts up like any other generator. Here
    /// `[id_1, F] ; [ε, id_1]` schedules `ε` and `F` into one layer, matching
    /// the parallel `ε ⊗ F`. (Zero-*source* generators such as `η` are the
    /// skipped case and are not exercised by this signature.)
    #[test]
    fn target_zero_sink_sifts_up() {
        let lhs = seq(vec![
            layer(vec![Identity(1), Generator(TestSig::F)]),
            layer(vec![Generator(TestSig::Eps), Identity(1)]),
        ]);
        let rhs = layer(vec![Generator(TestSig::Eps), Generator(TestSig::F)]);
        assert_eq!(nf(&lhs), nf(&rhs));
    }

    /// JS-Braided p. 36 / issue #14 guard: a braid-bearing layer blocks
    /// sifting — `[σ] ; [F, id_1]` must keep `F` in its layer (the leading
    /// braid stays isolated; the generator does not move into the crossing).
    /// The diagram is already in normal form, so both layers survive.
    #[test]
    fn braid_layer_blocks_sift() {
        let sd = seq(vec![
            Braid(1, 1),
            layer(vec![Generator(TestSig::F), Identity(1)]),
        ]);
        let nfd = nf(&sd);
        assert_eq!(nfd.layers.len(), 2, "braid guard must keep both layers");
    }

    /// JS-I Ch 2 §1 axiom (S) p. 73 over an independent 2-wire band: two σ's
    /// trapped alongside a `generator ; generator` chain still cancel.
    /// `[F, σ] ; [G, σ]  ==  (F ; G) ⊗ id_2` — the wire-0 `F;G` dependency
    /// blocks any whole-layer merge, so the σ's are freed by
    /// `isolate_mixed_braid_layers` into braid-only layers, slid together by the
    /// naturality sweep, and cancelled by the ordinary braid-run canonicalization
    /// / involution (bifunctoriality, JS-I §4 Thm 1.2 p. 71).
    #[test]
    fn aligned_braid_band_cancels_through_generators() {
        let lhs = seq(vec![
            layer(vec![Generator(TestSig::F), Braid(1, 1)]),
            layer(vec![Generator(TestSig::G), Braid(1, 1)]),
        ]);
        let rhs = layer(vec![
            Compose(
                Box::new(Generator(TestSig::F)),
                Box::new(Generator(TestSig::G)),
            ),
            Identity(2),
        ]);
        assert_eq!(nf(&lhs), nf(&rhs));
    }
}

// ============================================================================
// Review-driven soundness / invariant regressions
// ============================================================================

mod review_soundness_fixes {
    use super::{PropExpr, TestSig, has_mixed_layer, has_wide_braid, nf};
    use PropExpr::{Braid, Compose, Generator, Identity, Tensor};

    fn b<T>(x: T) -> Box<T> {
        Box::new(x)
    }

    /// JS-I Ch 1 §1 (`id_0` ⊗-unit) + §4 Thm 1.2 p. 71 (bifunctoriality),
    /// `try_unitor_merge` source-left case: `L1 ; (X ⊗ id_k)` with `X.source == 0`
    /// factors as `X ⊗ L1` — `X` is PREPENDED. Left-unitor `id ; f = f` with a
    /// zero-source `η : 0 → 1` on the left of the trailing layer must land `η`
    /// ahead of the passed-through wire, matching the parallel form.
    #[test]
    fn unitor_merge_source_left_prepends() {
        let eta = || Generator(TestSig::Eta);
        let lhs = Compose(b(Identity(1)), b(Tensor(b(eta()), b(Identity(1)))));
        let rhs = Tensor(b(eta()), b(Identity(1)));
        assert_eq!(nf(&lhs), nf(&rhs));
    }

    /// The prepend fix is load-bearing for *distinctness*: `σ ; (η ⊗ id₂)` places
    /// the fresh `η` wire on the left, `σ ; (id₂ ⊗ η)` on the right — genuinely
    /// different morphisms (JS-I §4). Before the fix, case 3 appended `X` in both
    /// and the two collided. Their NFs must differ.
    #[test]
    fn unitor_merge_zero_source_position_is_significant() {
        let eta = || Generator(TestSig::Eta);
        let left = Compose(b(Braid(1, 1)), b(Tensor(b(eta()), b(Identity(2)))));
        let right = Compose(b(Braid(1, 1)), b(Tensor(b(Identity(2)), b(eta()))));
        assert_ne!(nf(&left), nf(&right));
    }

    /// JS-Braided (B2) p. 33–34 / JS-I Ch 2 Thm 2.3 p. 81: a wide braid produced
    /// mid-normalization must still hexagon-expand. Here `σ_{1,1}` slides past
    /// `H : 2 → 1` in the naturality sweep, emitting `σ_{2,1}` inside an
    /// identity-padded layer; the generalized `hexagon_expand` must decompose it
    /// so the result carries no wide braid and no mixed layer.
    #[test]
    fn wide_braid_from_naturality_expands() {
        // (G ⊗ (H ⊗ F)) ; (id₁ ⊗ σ)   with A=H:2→1, B=F, C=G.
        let expr = Compose(
            b(Tensor(
                b(Generator(TestSig::G)),
                b(Tensor(b(Generator(TestSig::H)), b(Generator(TestSig::F)))),
            )),
            b(Tensor(b(Identity(1)), b(Braid(1, 1)))),
        );
        let out = nf(&expr);
        assert!(
            !has_wide_braid(&out),
            "wide braid must be expanded: {out:?}"
        );
        assert!(!has_mixed_layer(&out), "no mixed layer: {out:?}");
    }

    /// A wide braid inside an identity-padded layer (`F ⊗ σ_{2,1}` isolates to
    /// `[Id1, σ_{2,1}]`) must expand to the bubble-brick form of `σ_{2,1}`.
    /// Equals the explicit brick decomposition `(id₁ ⊗ σ₁₁) ; (σ₁₁ ⊗ id₁)`
    /// tensored/composed into the same morphism, and carries no wide braid.
    #[test]
    fn wide_braid_in_padded_layer_expands() {
        let lhs = Tensor(b(Generator(TestSig::F)), b(Braid(2, 1)));
        let brick_sigma_21 = Compose(
            b(Tensor(b(Identity(1)), b(Braid(1, 1)))),
            b(Tensor(b(Braid(1, 1)), b(Identity(1)))),
        );
        let rhs = Compose(
            b(Tensor(b(Generator(TestSig::F)), b(Identity(3)))),
            b(Tensor(b(Identity(1)), b(brick_sigma_21))),
        );
        assert_eq!(nf(&lhs), nf(&rhs));
        assert!(!has_wide_braid(&nf(&lhs)), "wide braid must be expanded");
    }

    /// The bare identity-padded singleton `id₁ ⊗ σ_{2,1}` must also expand.
    #[test]
    fn wide_braid_identity_padded_singleton_expands() {
        let out = nf(&Tensor(b(Identity(1)), b(Braid(2, 1))));
        assert!(
            !has_wide_braid(&out),
            "wide braid must be expanded: {out:?}"
        );
    }

    /// F3: `try_unitor_merge` could re-create a mixed braid+generator layer; the
    /// mixed-merge refusal at `reduce_involution`'s merge site prevents it.
    /// `σ ; (id₂ ⊗ η)` normalizes to a fixpoint with no mixed layer.
    #[test]
    fn unitor_merge_never_leaves_mixed_layer() {
        let expr = Compose(
            b(Braid(1, 1)),
            b(Tensor(b(Identity(2)), b(Generator(TestSig::Eta)))),
        );
        let out = nf(&expr);
        assert!(!has_mixed_layer(&out), "no mixed layer: {out:?}");
    }
}
