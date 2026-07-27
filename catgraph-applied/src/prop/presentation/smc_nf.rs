//! SMC string-diagram normal form (Layer 1 of the normalizer).
//!
//! # Role
//!
//! This module provides, in place of the plain `apply_smc_rules` pre-pass, a
//! **Joyal-Street string-diagram normal form**: a total function
//! [`PropExpr`] → [`StringDiagram`] aiming at "SMC-equal iff NF-equal". The
//! ⇐ direction holds unconditionally (every rewrite is SMC-sound); the ⇒
//! direction is **probe-verified on the fragment `𝔉`** with four documented
//! residuals — a full proof is open (`docs/SMC-NF-RECONCILIATION.md` §4.4
//! canonicality status; residual scope on [`nf`]). The sibling
//! [`super::kb::CongruenceClosure`] (Layer 2) then operates on NF-normalized
//! terms and handles user-equation congruence without needing to know about
//! SMC axioms.
//!
//! # Algorithm (8 steps + empty-braid normalization)
//!
//! - **Step 0** — `normalize_empty_braids`: `Braid(0, n) → Identity(n)`.
//! - **Step 1** — `hexagon_expand`: `σ_{m,n}` (m+n > 2) → bricks of `Braid(1,1)`.
//!   Paper: JS-Braided Prop 2.1 / (B2) p.33–34; JS-I Ch 2 Thm 2.3 p.81.
//! - **Step 2** — `reduce_involution`: `σ;σ → id_2`.
//!   Paper: JS-I Ch 2 axiom (S) p.73; JS-Braided (S) p.21; Selinger §3.5 p.17.
//! - **Step 3** — `collect_braid_prefix`: push braids to input-side layers.
//!   Paper: JS-II §1.2 α-anchor; JS-Braided p.36 "box slides through crossing".
//! - **Step 4** — `coalesce_identity_layers`: identity-only layers absorb.
//!   Paper: JS-I Ch 1 Prop 1.1 p.66; JS-II Thm 1.1.3 p.4.
//! - **Step 4(c)** — `topological_layer_order`: sift each generator up to its
//!   earliest braid-free layer (interchange scheduling; zero-source `η` by the
//!   component-anchored point-span rule, issue #55).
//!   Paper: JS-I Ch 1 §4 Thm 1.2 p.71 (bifunctoriality / interchange); issue #14.
//! - **Step 5** — `simplify_units`: remove `Identity(0)` atoms.
//!   Paper: JS-I Ch 1 §1 p.57; Selinger Table 2 p.10.
//! - **Step 7** — `reorder_component_blocks`: transpose adjacent *free* component
//!   blocks (closed ∥ anything, input-only ∥ output-only) into rule-(i) order.
//!   Paper: JS-I Ch 1 §4 Thm 1.2 p.71 with JS-I Ch 2 §1 axiom (S) p.73 in its
//!   degenerate `σ_{0,n} = id` form; issue #55 rule (i).
//! - **Step 6** — `reorder_tied_zero_arity`: within-layer canonical order for
//!   strictly-commuting zero-arity atoms (scalar < η < ε < solid at single-atom
//!   ties, component order otherwise).
//!   Paper: JS-I Ch 1 §4 Thm 1.2 p.71 (bifunctoriality) with a 0-arity edge;
//!   issue #55 Decision 1 + rule (i).
//!
//! (Step 7 is staged *before* Step 6 in the fixpoint loop: a block move can land
//! an `η` beside an `ε`, and Step 6 repairs that on the same pass.)

use super::super::{PropExpr, PropSignature};

/// A single horizontal slice of a string diagram. Within a layer, atoms are
/// tensored left-to-right; the layer's `Vec<Atom>` preserves source tensor
/// order (see `docs/SMC-NF-RECONCILIATION.md` §2.3).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Layer<G: PropSignature> {
    pub atoms: Vec<Atom<G>>,
}

/// A primitive building block of a [`Layer`]. Every `PropExpr` leaf lowers to
/// exactly one `Atom`; composite `PropExpr` nodes combine layers sequentially
/// ([`PropExpr::Compose`]) or in parallel ([`PropExpr::Tensor`]).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Atom<G: PropSignature> {
    /// Invertible wire bundle of width `n`. `Identity(0)` is the tensor unit
    /// and is eliminated by the internal `simplify_units` step.
    Identity(usize),
    /// Braid generator `σ_{m,n}: m+n → n+m`. The `Braid(1,1)` case is
    /// irreducible; wider braids are decomposed by the internal
    /// `hexagon_expand` step.
    Braid(usize, usize),
    /// A primitive from the signature.
    Generator(G),
}

/// Layered string-diagram representation: `L_0 ; L_1 ; ... ; L_{k-1}` where
/// each `L_i : Layer<G>` is a parallel tensor product of atoms.
///
/// After `nf`, the following invariants hold:
/// - No `Atom::Identity(0)` anywhere.
/// - No `Atom::Braid(m,n)` with `m+n > 2` (all hexagon-expanded to `Braid(1,1)`).
/// - No `Atom::Braid(0, _)` or `Atom::Braid(_, 0)` (normalized to `Identity`).
/// - No two adjacent layers both consisting entirely of `Identity` atoms.
/// - No two adjacent `Identity` atoms within a layer (intra-layer fusion,
///   `coalesce_identity_layers` (a) / `merge_adjacent_identities`).
/// - No pure-identity layer while any non-identity layer remains (one
///   survives as the arity carrier only in an all-identity diagram).
/// - Every maximal run of braid layers is the canonical bubble-sort schedule
///   of its underlying permutation (`canonicalize_braid_runs`, Step 3(b)).
/// - All `Atom::Braid` atoms appear in the leading (input-side) layers; no
///   generator layer is followed by a braid layer.
/// - No layer contains both a `Braid` and a `Generator` atom (mixed layers are
///   split by `isolate_mixed_braid_layers` and never re-created).
/// - Every `Generator` atom occupies its earliest admissible layer: no
///   positive-source generator's consumed wires all pass through `Identity`
///   atoms in the preceding braid-free layer, and no zero-source generator
///   (`η`) in a non-interleaved component can slide across into that layer at
///   its component-anchored coordinate (Step 4(c)).
/// - Within every layer, no adjacent strictly-commuting pair is ordered against
///   the Step 6 order: `scalar < η < ε` at a single-atom tie, and
///   `closed < input-anchored < output-only` (by least attached boundary
///   coordinate) when either atom belongs to a multi-atom component.
/// - No adjacent *free* pair of connected components (Step 7: at most one
///   attaching each boundary, at least one multi-atom, neither interleaved) is
///   ordered against that same component order.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct StringDiagram<G: PropSignature> {
    pub layers: Vec<Layer<G>>,
}

// -------------------------------------------------------------------------
// Public entry point
// -------------------------------------------------------------------------

/// Normalize `expr` to a canonical [`StringDiagram`].
///
/// Totality: `nf` is defined on every arity-well-formed `PropExpr<G>` and
/// always terminates (see the termination measure in `docs/SMC-NF-RECONCILIATION.md` §2.4).
///
/// Canonicality: the *soundness* direction — `nf(&a) == nf(&b)` implies `a`
/// and `b` are SMC-equal (equal in the free symmetric monoidal category on
/// `G`) — holds unconditionally, since `nf` applies only SMC-sound rewrites.
/// The *canonicality* direction — SMC-equal expressions get equal NFs — is
/// probe-verified on the fragment 𝔉, not proven (docs §4.4).
///
/// Known exception (issue #55): both halves of the former zero-arity gap are
/// closed — **probe-verified, not proven** — on the fragment 𝔉
/// (`docs/SMC-NF-RECONCILIATION.md` §4.1: every connected component clear of
/// guard 3's marking *and* touching a boundary; the closed-component
/// exclusion was added 2026-07-27, §4.6(c), and the draft §4 proof was
/// refuted the same day by a residual *inside* 𝔉, §4.6(d) — see the §4.4
/// canonicality status).
/// - **Within-layer order** — Step 6 (`reorder_tied_zero_arity`, PR1) puts every
///   tied run in canonical order, so `nf(ε ⊗ η) == nf(η ⊗ ε)`.
/// - **Layer assignment** — a zero-source generator (`source == 0`, e.g.
///   `η : 0 → 1`) is scheduled by the component-anchored point-span sift (PR2,
///   rule (i)): its slot in the earlier layer is derived from its connected
///   component's boundary attachment, never from the incidental source cursor.
///   So `nf(F ⊗ η ⊗ G) == nf((F ⊗ G) ; (id₁ ⊗ η ⊗ id₁))`,
///   `nf(ε ; η) == nf(η ⊗ ε) == nf(ε ⊗ η)`, and the two-component witness
///   `nf((μ;!) ; (η;Δ)) == nf((μ;!) ⊗ (η;Δ))`.
/// - **Block order** — Step 7 (`reorder_component_blocks`) transposes whole
///   *free* component blocks into the same rule-(i) order, so the tensor-form
///   transpositions close too: `nf((μ;!) ⊗ (η;Δ)) == nf((η;Δ) ⊗ (μ;!))`, and a
///   closed block sorts leftmost regardless of where it was written.
///
/// **Residual scope**, four cases. The *interleave guard*: when a component's
/// boundary attachment interleaves another component's on the same boundary,
/// block transposition is not braid-free and rule (i)'s slot is ill-defined, so
/// such an `η` is not sifted, Step 6 falls back to the Decision-1 class order
/// for it, and Step 7 leaves it alone. The *equal-key* case: rule (i) gives
/// all closed components one key, so two distinct closed blocks keep their input
/// order — the block-level reading of Step 6's stable-among-scalars, and the
/// same no-`Ord`-on-`G` limitation. The *trapped-nesting* case (found in
/// the #55 proof phase, 2026-07-27): a closed component written strictly inside
/// another component's wire span does not extract — its `η` is blocked by the
/// enclosing atom's target span and Step 7 never sees an adjacent free pair;
/// the nested and free writings have identical abstract content, so this
/// residual is irreducibly presentation-level (issue #174). And the
/// *nested-column* case (found in adversarial review the same day, *inside*
/// 𝔉): a multi-atom zero-arity block, solid on the side facing its enclosing
/// wall's opening (solid-headed sink / solid-tailed source), written nested
/// inside another component's span converges with none of its free writings — the
/// missing move is a zero-arity-bounded *column* transposition, between what
/// Step 6 (atoms) and Step 7 (whole components) can do; see
/// `docs/SMC-NF-RECONCILIATION.md` §4.5. All four are strictly narrower than
/// the pre-PR2 gap, which covered *every* mid-layer `η`. See
/// `docs/SMC-NF-RECONCILIATION.md` §2.6 (the component-order anchor and its
/// carve against §2.5), §4 (content function + canonicality status), and the
/// `smc_canonicality_probes` module in `tests/smc_nf_completeness.rs`.
/// Generators with `source > 0` were never affected by the *zero-arity* gap,
/// though a nested zero-arity block can distort their layers too (§4.4).
pub fn nf<G: PropSignature>(expr: &PropExpr<G>) -> StringDiagram<G> {
    let mut sd = lower(expr);
    // Fixpoint loop, terminating by the lexicographic measure
    // (crossings, mixed_layer_count, wide_braid_count, braid_position_sum,
    //  generator_position_sum, layer_count, block_inversion_count,
    //  tied_inversion_count):
    // - `reduce_involution` shrinks crossings (σ;σ → id); `hexagon_expand` leaves
    //   crossings fixed (it preserves the underlying permutation);
    // - `isolate_mixed_braid_layers` (inside `collect_braid_prefix`) strictly
    //   shrinks mixed_layer_count, and the mixed-merge refusal at
    //   `reduce_involution`'s merge site keeps anything from re-creating a mixed
    //   layer;
    // - `hexagon_expand` strictly shrinks wide_braid_count (`Braid(m,n)`, m+n>2 →
    //   `Braid(1,1)` bricks), placed ahead of braid_position_sum so a wide braid
    //   emitted by the naturality sweep is decomposed on the next pass before
    //   braid positions are compared (at the fixpoint check no wide braid remains);
    // - the naturality sweep shrinks braid_position_sum (braids move input-ward);
    // - `topological_layer_order` shrinks generator_position_sum (each sift,
    //   including a zero-source `η` component-anchored point-span sift, drops
    //   exactly one generator by one layer and moves nothing else — issue #55);
    // - `coalesce_identity_layers`/`simplify_units` shrink layer_count;
    // - `reorder_component_blocks` (Step 7) shrinks block_inversion_count — each
    //   transposition flips every position pair between the two blocks' runs and
    //   leaves all other pairs' relative order alone. It changes no layer's
    //   membership and rewrites no atom, so every earlier component is invariant
    //   under it. It may *raise* tied_inversion_count (a block move can land an
    //   η beside an ε), but that is the later component and Step 6 repairs it on
    //   the same pass;
    // - `reorder_tied_zero_arity` (Step 6) shrinks tied_inversion_count — each
    //   swap flips exactly one class-inverted pair and leaves every other pair's
    //   relative order alone. `try_unitor_merge` can *raise* it (its case 1
    //   prepends an ε ahead of the absorbed layer's atoms, possibly past an η;
    //   its case 4 appends an η after them, possibly behind an ε; case 3's
    //   η-prepend is order-canonical), but only while strictly shrinking
    //   layer_count, an earlier component — so the tuple still drops
    //   lexicographically and Step 6 repairs the ordering on the same pass.
    //   The point-span sift can likewise land an `η` beside an `ε` and raise
    //   tied_inversion_count, but only while shrinking generator_position_sum —
    //   an earlier component still — and Step 6 repairs it on the same pass.
    //   Step 6 itself is
    //   within-layer only: it moves no atom between layers and rewrites no atom,
    //   so every earlier component is untouched by it — block_inversion_count
    //   included: a single-single tied swap involves only single-atom components,
    //   which the block count excludes by its condition (a), and a tied swap
    //   touching a multi-atom component already fires on the rule-(i) component
    //   order (`tie_sorts_before`), so it can only lower the block count or leave
    //   it alone.
    // See `docs/SMC-NF-RECONCILIATION.md` §2.4.
    loop {
        let prev = sd.clone();
        sd = normalize_empty_braids(sd);
        sd = hexagon_expand(sd);
        sd = reduce_involution(sd);
        sd = collect_braid_prefix(sd);
        sd = coalesce_identity_layers(sd);
        sd = topological_layer_order(sd);
        sd = simplify_units(sd);
        sd = reorder_component_blocks(sd);
        sd = reorder_tied_zero_arity(sd);
        if sd == prev {
            break;
        }
    }
    sd
}

/// Fold a [`StringDiagram`] back into a right-associated [`PropExpr`].
///
/// Inverse of the lowering construction (within SMC coherence): a single layer
/// becomes a right-associated tensor of its atoms; a layer sequence becomes a
/// right-associated compose of the per-layer tensor expressions.
///
/// Used by Layer 2 to feed NF-normalized terms into the CC engine.
///
/// # Panics
///
/// Never — the internal `expect` calls are invariant-guarded by the empty-list
/// early returns. The `expect` messages exist for developer visibility if the
/// invariant is ever violated by a future refactor.
#[must_use]
pub fn from_string_diagram<G: PropSignature>(sd: &StringDiagram<G>) -> PropExpr<G> {
    if sd.layers.is_empty() {
        return PropExpr::Identity(0);
    }
    let mut layer_exprs = sd.layers.iter().map(layer_to_expr::<G>);
    let first = layer_exprs.next().expect("non-empty layers checked above");
    layer_exprs.fold(first, |acc, next| {
        PropExpr::Compose(Box::new(acc), Box::new(next))
    })
}

/// Convert a single layer's atoms into a right-associated tensor expression.
fn layer_to_expr<G: PropSignature>(layer: &Layer<G>) -> PropExpr<G> {
    if layer.atoms.is_empty() {
        return PropExpr::Identity(0);
    }
    let mut iter = layer.atoms.iter().rev().map(atom_to_expr::<G>);
    let last = iter.next().expect("non-empty atoms checked above");
    iter.fold(last, |acc, atom_expr| {
        PropExpr::Tensor(Box::new(atom_expr), Box::new(acc))
    })
}

fn atom_to_expr<G: PropSignature>(atom: &Atom<G>) -> PropExpr<G> {
    match atom {
        Atom::Identity(n) => PropExpr::Identity(*n),
        Atom::Braid(m, n) => PropExpr::Braid(*m, *n),
        Atom::Generator(g) => PropExpr::Generator(g.clone()),
    }
}

// -------------------------------------------------------------------------
// Lowering: PropExpr → StringDiagram (unoptimized)
// -------------------------------------------------------------------------

/// Lower a `PropExpr` into a one-atom-per-layer `StringDiagram`.
///
/// Paper: JS-I Ch 1 §3 p.66 rectangle-cover; JS-I Ch 2 Prop 2.1 p.78 layering
/// of abstract diagrams.
fn lower<G: PropSignature>(expr: &PropExpr<G>) -> StringDiagram<G> {
    match expr {
        PropExpr::Identity(0) => StringDiagram { layers: Vec::new() },
        PropExpr::Identity(n) => StringDiagram {
            layers: vec![Layer {
                atoms: vec![Atom::Identity(*n)],
            }],
        },
        PropExpr::Braid(m, n) => StringDiagram {
            layers: vec![Layer {
                atoms: vec![Atom::Braid(*m, *n)],
            }],
        },
        PropExpr::Generator(g) => StringDiagram {
            layers: vec![Layer {
                atoms: vec![Atom::Generator(g.clone())],
            }],
        },
        PropExpr::Compose(a, b) => {
            // Sequential composition = layer concatenation. JS-I Prop 1.1.
            let mut la = lower(a);
            la.layers.extend(lower(b).layers);
            la
        }
        PropExpr::Tensor(a, b) => {
            // Parallel composition = layer-wise side-by-side merge, padding the
            // shorter side with Identity atoms so layer counts match.
            // Paper: JS-I Ch 1 §4 p.69–70.
            pad_and_zip(lower(a), lower(b))
        }
    }
}

/// Pad two diagrams to equal layer count (inserting `Identity(arity)` layers
/// on the shorter side) and concatenate atom-sequences layer-wise.
fn pad_and_zip<G: PropSignature>(
    mut a: StringDiagram<G>,
    mut b: StringDiagram<G>,
) -> StringDiagram<G> {
    let a_last_arity = trailing_arity(&a);
    let b_last_arity = trailing_arity(&b);
    while a.layers.len() < b.layers.len() {
        a.layers.push(Layer {
            atoms: vec![Atom::Identity(a_last_arity)],
        });
    }
    while b.layers.len() < a.layers.len() {
        b.layers.push(Layer {
            atoms: vec![Atom::Identity(b_last_arity)],
        });
    }
    let layers: Vec<Layer<G>> = a
        .layers
        .into_iter()
        .zip(b.layers)
        .map(|(mut la, lb)| {
            la.atoms.extend(lb.atoms);
            // Merge adjacent identities eagerly so distinct lowering paths
            // (e.g., tensor associator variants) converge on the same
            // atom-boundary structure before canonicalization runs. Without
            // this, `pad_and_zip` can produce `[Identity(k)]` on one branch
            // while the mirror branch produces `[Identity(1), Identity(1),
            // ...]` at the same wire positions, causing Step 2's
            // `try_column_merge` to fire asymmetrically.
            Layer {
                atoms: merge_adjacent_identities(la.atoms),
            }
        })
        .collect();
    StringDiagram { layers }
}

/// Output arity of a diagram (target wire count). Used by `pad_and_zip` to
/// pick the right `Identity(arity)` for padding layers.
fn trailing_arity<G: PropSignature>(sd: &StringDiagram<G>) -> usize {
    sd.layers
        .last()
        .map_or(0, |layer| layer.atoms.iter().map(atom_target).sum())
}

fn atom_source<G: PropSignature>(a: &Atom<G>) -> usize {
    match a {
        Atom::Identity(n) => *n,
        Atom::Braid(m, n) => m + n,
        Atom::Generator(g) => g.source(),
    }
}

fn atom_target<G: PropSignature>(a: &Atom<G>) -> usize {
    match a {
        Atom::Identity(n) => *n,
        Atom::Braid(m, n) => m + n,
        Atom::Generator(g) => g.target(),
    }
}

/// True if `atoms` holds at least one `Braid`.
fn layer_has_braid<G: PropSignature>(atoms: &[Atom<G>]) -> bool {
    atoms.iter().any(|a| matches!(a, Atom::Braid(_, _)))
}

/// True if `atoms` holds both a `Braid` and a `Generator` — a "mixed" layer,
/// the shape `isolate_mixed_braid_layers` splits and `reduce_involution` refuses
/// to (re-)create.
fn is_mixed_layer<G: PropSignature>(atoms: &[Atom<G>]) -> bool {
    layer_has_braid(atoms) && atoms.iter().any(|a| matches!(a, Atom::Generator(_)))
}

// -------------------------------------------------------------------------
// Canonicalization steps
// -------------------------------------------------------------------------

/// **Step 0**: normalize `Braid(0, n) → Identity(n)` and `Braid(m, 0) → Identity(m)`.
///
/// Paper anchor: trivial consequence of JS-I Ch 2 axiom (S) p.73 on
/// degenerate block structure. Runs before [`hexagon_expand`] so that step
/// never recurses on an already-identity braid.
fn normalize_empty_braids<G: PropSignature>(sd: StringDiagram<G>) -> StringDiagram<G> {
    let layers: Vec<Layer<G>> = sd
        .layers
        .into_iter()
        .map(|layer| Layer {
            atoms: layer
                .atoms
                .into_iter()
                .map(|a| match a {
                    Atom::Braid(0, n) => Atom::Identity(n),
                    Atom::Braid(m, 0) => Atom::Identity(m),
                    other => other,
                })
                .collect(),
        })
        .collect();
    StringDiagram { layers }
}

/// **Step 1**: hexagon-expand every `Atom::Braid(m, n)` with `m+n > 2` into
/// a layered sequence of `Atom::Braid(1, 1)` bricks.
///
/// Algorithm: `σ_{m,n}` is the permutation
///   `π = [m, m+1, ..., m+n-1, 0, 1, ..., m-1]`
/// (output position k holds wire from input position `π[k]`). Bubble-sort
/// `π` back to `[0, 1, ..., m+n-1]` to obtain a sequence of adjacent
/// transpositions; the reversed sequence applied to the identity produces
/// `π`, which is exactly the decomposition of `σ_{m,n}`.
///
/// Each emitted layer has the form `[Identity(i), Braid(1,1), Identity(m+n-i-2)]`
/// (with the leading/trailing identities suppressed when their width is 0).
///
/// A layer qualifies when its only non-`Identity` atom is a single wide
/// `Braid(m,n)` (`m+n > 2`) — including identity-padded layers such as
/// `[Identity(p), Braid(2,1), Identity(s)]`. Wide braids reach this shape not
/// only from lowering but from `try_naturality_swap`'s `σ_{s_a,s_b}` emission
/// and from `isolate_mixed_braid_layers`; expanding them in place is what keeps
/// the "no `Braid(m,n)` with `m+n > 2`" invariant true. The surrounding
/// prefix/suffix identity widths are re-applied to each emitted brick layer.
///
/// Paper anchor: JS-Braided Prop 2.1 / axiom (B2) p.33–34
/// (`c_{U⊗V, W} = (c_{U,W} ⊗ 1_V) ∘ (1_U ⊗ c_{V,W})`); JS-I Ch 2 Thm 2.3 p.81
/// via the `S_n` presentation. Direction convention: always expand, never
/// collapse (see `docs/SMC-NF-RECONCILIATION.md` §2.2).
fn hexagon_expand<G: PropSignature>(sd: StringDiagram<G>) -> StringDiagram<G> {
    let mut new_layers: Vec<Layer<G>> = Vec::with_capacity(sd.layers.len());
    for layer in sd.layers {
        if let Some((prefix, (m, n), suffix)) = wide_braid_in_identity_padding(&layer.atoms) {
            for brick in decompose_braid::<G>(m, n) {
                new_layers.push(pad_braid_layer(prefix, brick.atoms, suffix));
            }
        } else {
            new_layers.push(layer);
        }
    }
    StringDiagram { layers: new_layers }
}

/// If `atoms` is entirely `Identity` except for exactly one wide `Braid(m,n)`
/// (`m+n > 2`), return `(prefix_width, (m, n), suffix_width)` where the widths
/// are the total wire widths before/after the braid. Any generator, any second
/// braid, or a non-wide braid disqualifies (returns `None`).
fn wide_braid_in_identity_padding<G: PropSignature>(
    atoms: &[Atom<G>],
) -> Option<(usize, (usize, usize), usize)> {
    let mut braid: Option<(usize, usize)> = None;
    let mut braid_idx = 0;
    for (i, a) in atoms.iter().enumerate() {
        match a {
            Atom::Identity(_) => {}
            Atom::Braid(m, n) if m + n > 2 && braid.is_none() => {
                braid = Some((*m, *n));
                braid_idx = i;
            }
            // Second braid, non-wide braid, or generator disqualifies.
            _ => return None,
        }
    }
    let (m, n) = braid?;
    let prefix: usize = atoms[..braid_idx].iter().map(atom_source).sum();
    let suffix: usize = atoms[braid_idx + 1..].iter().map(atom_source).sum();
    Some((prefix, (m, n), suffix))
}

/// Re-apply surrounding identity padding to a brick layer emitted by
/// [`decompose_braid`], fusing adjacent identities.
fn pad_braid_layer<G: PropSignature>(
    prefix: usize,
    brick: Vec<Atom<G>>,
    suffix: usize,
) -> Layer<G> {
    let mut out: Vec<Atom<G>> = Vec::with_capacity(brick.len() + 2);
    if prefix > 0 {
        out.push(Atom::Identity(prefix));
    }
    out.extend(brick);
    if suffix > 0 {
        out.push(Atom::Identity(suffix));
    }
    Layer {
        atoms: merge_adjacent_identities(out),
    }
}

/// Decompose `σ_{m, n}` (with `m + n > 2`) into a sequence of single-
/// transposition layers.
fn decompose_braid<G: PropSignature>(m: usize, n: usize) -> Vec<Layer<G>> {
    let total = m + n;
    // σ_{m,n} takes input [a_0..a_{m-1}, b_0..b_{n-1}] to output [b_0..b_{n-1},
    // a_0..a_{m-1}]. Output position k holds input position perm[k]:
    //   perm[0..n]      = m..m+n  (b-block from input positions m..m+n)
    //   perm[n..n+m]    = 0..m    (a-block from input positions 0..m)
    let perm: Vec<usize> = (0..n).map(|k| m + k).chain(0..m).collect();
    // Bubble-sort perm to [0..total] via the shared `adjacent_swaps` core,
    // recording the swap positions.
    let mut swaps = super::super::adjacent_swaps(&perm);
    // Reversed sequence applied to identity reproduces σ_{m,n}: sorting the
    // OUTPUT-indexed perm above yields the word that UNDOES the braid, and
    // reversing a word of self-inverse adjacent transpositions inverts the
    // composite — giving the word that BUILDS it. (A permutation-convention
    // fact, not a pipeline-direction difference: `permutation_sfg` in
    // `crate::mat_to_sfg` needs no reversal because its perm is INPUT-indexed —
    // wire `p` exits at `perm[p]` — and both consumers compose layers in the
    // same forward order.)
    swaps.reverse();
    swaps
        .into_iter()
        .map(|i| Layer {
            atoms: braid_at_position::<G>(i, total),
        })
        .collect()
}

/// Construct a layer containing a single `Braid(1,1)` at wire position `i`
/// (swapping wires `i` and `i+1`), padded with leading/trailing `Identity`
/// atoms as needed to cover `n_total` wires.
fn braid_at_position<G: PropSignature>(i: usize, n_total: usize) -> Vec<Atom<G>> {
    let mut atoms: Vec<Atom<G>> = Vec::new();
    if i > 0 {
        atoms.push(Atom::Identity(i));
    }
    atoms.push(Atom::Braid(1, 1));
    if i + 2 < n_total {
        atoms.push(Atom::Identity(n_total - i - 2));
    }
    atoms
}

/// **Step 2**: column-wise compose of adjacent layers when every column pair
/// reduces to a single atom. Handles three cases per column:
///
/// - `Identity(n) ; Identity(n)  →  Identity(n)` — trivial
/// - `Identity(n) ; X  →  X` with `X.source == n` — left identity-compose
/// - `X ; Identity(n)  →  X` with `X.target == n` — right identity-compose
/// - `Braid(m, n) ; Braid(n, m)  →  Identity(m+n)` — symmetric involution (S)
///
/// The generalization beyond bare involution is load-bearing: the
/// faithfulness tests' blocking case `(σ ⊗ id) ; (σ ⊗ id) = id_3` only
/// closes when each column reduces independently (σ;σ=id in column 0;
/// id;id=id in column 1), which is this step's job.
///
/// If any column-pair fails to reduce to a single atom, the layer pair is
/// left intact and normalization proceeds via later steps. A σ;σ band trapped
/// alongside a `generator ; generator` chain in a mixed layer — which no
/// whole-layer merge can reach — is instead handled downstream by
/// `isolate_mixed_braid_layers` → naturality sweep → braid-run canonicalization
/// (see [`collect_braid_prefix`]), which slides the braids to the leading
/// layers where the ordinary involution column-merge cancels them.
///
/// Paper anchor: JS-I Ch 2 §1 axiom (S) p.73 (the σ;σ case); JS-Braided (S)
/// p.21; Selinger §3.5 p.17. Interchange-over-identity is JS-I §4 Thm 1.2
/// p.71 (bifunctoriality) specialized to identity-containing contexts.
fn reduce_involution<G: PropSignature>(sd: StringDiagram<G>) -> StringDiagram<G> {
    let mut layers = sd.layers;
    let mut i = 0;
    while i + 1 < layers.len() {
        let merged = try_column_merge(&layers[i], &layers[i + 1])
            .or_else(|| try_unitor_merge(&layers[i], &layers[i + 1]))
            // Never (re-)create a mixed braid+generator layer: `collect_braid_prefix`'s
            // `isolate_mixed_braid_layers` splits those, and re-forming one would
            // re-trap a braid alongside an independent generator and stall the
            // fixpoint. Both `try_column_merge` and `try_unitor_merge` can otherwise
            // produce one, so the guard lives here at the shared merge site.
            .filter(|m| !is_mixed_layer(&m.atoms));
        if let Some(merged) = merged {
            layers[i] = merged;
            layers.remove(i + 1);
            // Don't advance — the new merged layer might merge with the next.
        } else {
            i += 1;
        }
    }
    StringDiagram { layers }
}

/// Merge two adjacent layers when one side has a zero-arity "sink" or "source"
/// atom that absorbs into the parallel structure of the other side.
///
/// Handles four mirror cases derived from bifunctoriality with a 0-arity edge:
///
/// - `[X, Identity(k)] ; L2` with `X.target == 0`, `L2.source == k`
///   → `[X, L2.atoms...]` (sink-left pattern, ε-on-left)
/// - `[Identity(k), X] ; L2` with `X.target == 0`, `L2.source == k`
///   → `[L2.atoms..., X]` (sink-right pattern)
/// - `L1 ; [X, Identity(k)]` with `X.source == 0`, `L1.target == k`
///   → `[X, L1.atoms...]` (source-left pattern, η-on-left)
/// - `L1 ; [Identity(k), X]` with `X.source == 0`, `L1.target == k`
///   → `[L1.atoms..., X]` (source-right pattern)
///
/// Derivation (sink-left case): `(X ⊗ id_k) ; L2 = (X ⊗ id_k) ; (id_0 ⊗ L2)
/// = (X ; id_0) ⊗ (id_k ; L2) = X ⊗ L2` by left-unitor on `id_0 ⊗ L2` and
/// bifunctoriality. Symmetrically for the other three cases.
///
/// Paper anchor: JS-I Ch 1 §1 (`id_0` as ⊗-unit); JS-I Ch 1 §4 Thm 1.2 p.71
/// (bifunctoriality).
fn try_unitor_merge<G: PropSignature>(l1: &Layer<G>, l2: &Layer<G>) -> Option<Layer<G>> {
    let l1_target: usize = l1.atoms.iter().map(atom_target).sum();
    let l2_source: usize = l2.atoms.iter().map(atom_source).sum();
    if l1_target != l2_source {
        return None;
    }

    // Case: L1 has exactly two atoms: [X, Identity(k)] with X.target == 0.
    if let [x, Atom::Identity(k)] = l1.atoms.as_slice()
        && atom_target(x) == 0
        && *k == l2_source
    {
        let mut atoms: Vec<Atom<G>> = vec![x.clone()];
        atoms.extend(l2.atoms.iter().cloned());
        return Some(Layer { atoms });
    }
    // Case: L1 = [Identity(k), X] with X.target == 0.
    if let [Atom::Identity(k), x] = l1.atoms.as_slice()
        && atom_target(x) == 0
        && *k == l2_source
    {
        let mut atoms: Vec<Atom<G>> = l2.atoms.clone();
        atoms.push(x.clone());
        return Some(Layer { atoms });
    }
    // Case: L2 = [X, Identity(k)] with X.source == 0.
    // Derivation: `L1 ; (X ⊗ id_k) = (id_0 ⊗ L1) ; (X ⊗ id_k)
    //            = (id_0 ; X) ⊗ (L1 ; id_k) = X ⊗ L1`.
    // So `X` is PREPENDED before L1's atoms (it occupies the leading fresh
    // wires that X introduces on the left).
    if let [x, Atom::Identity(k)] = l2.atoms.as_slice()
        && atom_source(x) == 0
        && *k == l1_target
    {
        let mut atoms: Vec<Atom<G>> = vec![x.clone()];
        atoms.extend(l1.atoms.iter().cloned());
        return Some(Layer { atoms });
    }
    // Case: L2 = [Identity(k), X] with X.source == 0.
    // Derivation: `L1 ; (id_k ⊗ X) = (L1 ⊗ id_0) ; (id_k ⊗ X)
    //            = (L1 ; id_k) ⊗ (id_0 ; X) = L1 ⊗ X`.
    // So `X` appends after L1's atoms (not prepends).
    if let [Atom::Identity(k), x] = l2.atoms.as_slice()
        && atom_source(x) == 0
        && *k == l1_target
    {
        let mut atoms: Vec<Atom<G>> = l1.atoms.clone();
        atoms.push(x.clone());
        return Some(Layer { atoms });
    }
    None
}

/// Try to compose two adjacent layers column-wise into a single layer. First
/// refines atom boundaries to a common grid (splitting `Identity` atoms as
/// needed), then composes column-wise. Returns `None` if the layers have
/// incompatible total widths or any column pair fails to reduce to a single
/// atom.
///
/// Boundary refinement is load-bearing: different `PropExpr` factorizations
/// of the same morphism can produce adjacent layers with the same wire
/// widths but different atom-boundary splits (e.g., `[Identity(2), F]`
/// alongside `[F, Identity(2)]`). Without refinement, Step 2 would fire
/// asymmetrically across different lowering paths, breaking tensor-associator
/// and bifunctoriality canonicalization.
///
/// Mixed braid+generator results are rejected by the caller
/// ([`reduce_involution`]'s merge site), not here.
fn try_column_merge<G: PropSignature>(l1: &Layer<G>, l2: &Layer<G>) -> Option<Layer<G>> {
    let (l1_split, l2_split) = refine_to_common_boundaries(l1, l2)?;
    if l1_split.atoms.len() != l2_split.atoms.len() {
        return None;
    }
    let mut merged_atoms: Vec<Atom<G>> = Vec::with_capacity(l1_split.atoms.len());
    for (a1, a2) in l1_split.atoms.iter().zip(l2_split.atoms.iter()) {
        // Composability: a1.target must equal a2.source for the column to
        // sequentially compose.
        if atom_target(a1) != atom_source(a2) {
            return None;
        }
        let merged = match (a1, a2) {
            // Widths matched via composability check above; either side works.
            (Atom::Identity(_), Atom::Identity(_) | _) => a2.clone(),
            (_, Atom::Identity(_)) => a1.clone(),
            (Atom::Braid(m, n), Atom::Braid(m2, n2)) if *m == *n2 && *n == *m2 => {
                // Symmetric involution: σ_{m,n} ; σ_{n,m} = id_{m+n}.
                Atom::Identity(m + n)
            }
            _ => return None,
        };
        merged_atoms.push(merged);
    }
    // The merged atoms may have adjacent identities from the split; fuse them.
    Some(Layer {
        atoms: merge_adjacent_identities(merged_atoms),
    })
}

/// Refine two adjacent layers to a common atom-boundary grid. `Identity`
/// atoms are split at interior boundaries as needed; non-`Identity` atoms
/// must already coincide with the grid (else the layers are structurally
/// incompatible and `None` is returned).
fn refine_to_common_boundaries<G: PropSignature>(
    l1: &Layer<G>,
    l2: &Layer<G>,
) -> Option<(Layer<G>, Layer<G>)> {
    let l1_target_total: usize = l1.atoms.iter().map(atom_target).sum();
    let l2_source_total: usize = l2.atoms.iter().map(atom_source).sum();
    if l1_target_total != l2_source_total {
        return None;
    }
    let total = l1_target_total;

    // Collect atom boundaries on each side.
    let l1_boundaries: Vec<usize> = wire_boundaries(&l1.atoms, /*use_target=*/ true);
    let l2_boundaries: Vec<usize> = wire_boundaries(&l2.atoms, /*use_target=*/ false);

    // Union of boundaries, sorted unique.
    let mut common: Vec<usize> = Vec::with_capacity(l1_boundaries.len() + l2_boundaries.len());
    common.extend(l1_boundaries.iter().copied());
    common.extend(l2_boundaries.iter().copied());
    common.sort_unstable();
    common.dedup();

    // Boundaries must cover [0, total]; both endpoints are included from the
    // two boundary vectors (0 and `total`).
    debug_assert_eq!(common.first().copied(), Some(0));
    debug_assert_eq!(common.last().copied(), Some(total));

    let l1_refined = split_atoms_at_boundaries(&l1.atoms, &common, /*use_target=*/ true)?;
    let l2_refined = split_atoms_at_boundaries(&l2.atoms, &common, /*use_target=*/ false)?;
    Some((Layer { atoms: l1_refined }, Layer { atoms: l2_refined }))
}

/// Return `[0, w_0, w_0+w_1, ..., total]` — cumulative wire positions of
/// atom boundaries using the requested width dimension (target or source).
fn wire_boundaries<G: PropSignature>(atoms: &[Atom<G>], use_target: bool) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::with_capacity(atoms.len() + 1);
    out.push(0);
    let mut cursor = 0;
    for a in atoms {
        cursor += if use_target {
            atom_target(a)
        } else {
            atom_source(a)
        };
        out.push(cursor);
    }
    out
}

/// Split each atom at any interior `common` boundaries. Identity atoms can be
/// split freely; non-identity atoms must coincide with the boundary grid
/// (else `None`).
fn split_atoms_at_boundaries<G: PropSignature>(
    atoms: &[Atom<G>],
    common: &[usize],
    use_target: bool,
) -> Option<Vec<Atom<G>>> {
    let mut out: Vec<Atom<G>> = Vec::new();
    let mut cursor = 0;
    let mut ci = 0; // index into common; common[0] == 0 always
    // Skip the starting 0 — it's always present.
    while ci < common.len() && common[ci] == 0 {
        ci += 1;
    }
    for atom in atoms {
        let w = if use_target {
            atom_target(atom)
        } else {
            atom_source(atom)
        };
        let end = cursor + w;
        // Collect interior split points: common boundaries strictly between
        // `cursor` and `end`.
        let mut splits: Vec<usize> = Vec::new();
        while ci < common.len() && common[ci] < end {
            splits.push(common[ci]);
            ci += 1;
        }
        // Skip the boundary at `end` itself (it's always present).
        if ci < common.len() && common[ci] == end {
            ci += 1;
        }
        if splits.is_empty() {
            out.push(atom.clone());
        } else if matches!(atom, Atom::Identity(_)) {
            // Split identity into pieces of widths delineated by `splits`.
            let mut prev = cursor;
            for sp in &splits {
                out.push(Atom::Identity(sp - prev));
                prev = *sp;
            }
            out.push(Atom::Identity(end - prev));
        } else {
            // Non-identity atom with interior split required — cannot refine.
            return None;
        }
        cursor = end;
    }
    Some(out)
}

/// **Step 3**: push all `Atom::Braid` atoms to the leading (input-side)
/// layers via braid-past-generator naturality, then canonicalize each run
/// of pure-braid layers to the bubble-sort decomposition of its underlying
/// permutation.
///
/// Three sub-rules, applied in order:
///
/// (0) **Mixed-layer braid isolation** — any layer holding both a `Braid` and
/// a `Generator` factors into two layers, braid part first, generator part
/// second, via bifunctoriality: per column `Braid → (Braid, Identity)`,
/// `Generator g → (Identity(g.source), g)`, `Identity(n) → (Identity(n),
/// Identity(n))`. This exposes the braid in a braid-only layer so sub-rules
/// (a)/(b) can act on it — without it, a braid co-resident with an unrelated
/// generator (e.g. `[σ, F]`) is stuck forever, since `is_braid_only_layer`
/// rejects the layer. Paper: JS-I Ch 1 §4 Thm 1.2 p.71 (bifunctoriality
/// factorization); braids-to-input direction is JS-II §1.2 α-anchor.
///
/// (a) **Naturality sweep** — for an adjacent pair `L_gen ; L_braid` where
/// `L_braid` contains a `Braid(1,1)` covering wires `[p, p+1]` that align
/// with the target boundary between two 1-wire-target atoms `X, Y` in
/// `L_gen`, rewrite to `L_braid' ; L_gen'` where `L_gen'` has `X` and `Y`
/// swapped and `L_braid'` uses the braid arities dictated by the atoms'
/// source widths (`σ_{X.source, Y.source}`). Paper: JS-Braided p.36
/// "box slides through crossing"; JS-II p.5 functoriality of α ↦ ⟨α⟩.
///
/// (b) **Run canonicalization** — each maximal sequence of pure-braid layers
/// computes an underlying permutation in `S_n`; replace the sequence with
/// the canonical bubble-sort decomposition of that permutation. Paper:
/// JS-Braided Cor 2.6 p.44 (equality of braids via underlying permutation);
/// JS-I Ch 2 §1 + Ch 3 p.84 (`S_n` reduced-word canonicality).
///
/// Direction convention: braids to input (see `docs/SMC-NF-RECONCILIATION.md` §2.1).
fn collect_braid_prefix<G: PropSignature>(sd: StringDiagram<G>) -> StringDiagram<G> {
    let mut layers = sd.layers;
    // Sub-rule (0): isolate braids out of mixed (braid+generator) layers.
    layers = isolate_mixed_braid_layers(layers);
    // Pre-split braid runs into single-braid canonical layers. `reduce_involution`
    // fuses adjacent independent braid layers into one multi-braid layer, which
    // the single-braid naturality sweep cannot slide; splitting first lets the
    // sweep move each braid past the generator layers.
    layers = canonicalize_braid_runs(layers);
    // Sub-rule (a): naturality sweep — iterate until no more swaps.
    let mut changed = true;
    let mut swapped_any = false;
    while changed {
        changed = false;
        let mut i = 0;
        while i + 1 < layers.len() {
            if is_braid_only_layer(&layers[i + 1])
                && !is_braid_only_layer(&layers[i])
                && let Some((new_braid, new_gen)) = try_naturality_swap(&layers[i], &layers[i + 1])
            {
                layers[i] = new_braid;
                layers[i + 1] = new_gen;
                changed = true;
                swapped_any = true;
            }
            i += 1;
        }
    }
    // Sub-rule (b): re-canonicalize the runs the sweep disturbed. If the sweep
    // made no swap, the pre-split above already left every run canonical.
    if swapped_any {
        layers = canonicalize_braid_runs(layers);
    }
    StringDiagram { layers }
}

/// Split every layer that holds both a `Braid` and a `Generator` into a
/// braid-part layer followed by a generator-part layer (see
/// [`collect_braid_prefix`] sub-rule (0)). Layers without that mix pass through
/// unchanged. Soundness: bifunctoriality, JS-I Ch 1 §4 Thm 1.2 p.71 — each
/// column composes back to its original atom (`Braid ; Identity = Braid`,
/// `Identity ; g = g`, `Identity ; Identity = Identity`).
fn isolate_mixed_braid_layers<G: PropSignature>(layers: Vec<Layer<G>>) -> Vec<Layer<G>> {
    let mut out: Vec<Layer<G>> = Vec::with_capacity(layers.len());
    for layer in layers {
        if is_mixed_layer(&layer.atoms) {
            let mut upper: Vec<Atom<G>> = Vec::with_capacity(layer.atoms.len());
            let mut lower: Vec<Atom<G>> = Vec::with_capacity(layer.atoms.len());
            for a in &layer.atoms {
                match a {
                    Atom::Braid(m, n) => {
                        upper.push(Atom::Braid(*m, *n));
                        lower.push(Atom::Identity(m + n));
                    }
                    Atom::Generator(g) => {
                        upper.push(Atom::Identity(g.source()));
                        lower.push(Atom::Generator(g.clone()));
                    }
                    Atom::Identity(k) => {
                        upper.push(Atom::Identity(*k));
                        lower.push(Atom::Identity(*k));
                    }
                }
            }
            out.push(Layer {
                atoms: merge_adjacent_identities(upper),
            });
            out.push(Layer {
                atoms: merge_adjacent_identities(lower),
            });
        } else {
            out.push(layer);
        }
    }
    out
}

/// A layer is "braid-only" if every atom is `Identity` or `Braid`, and at
/// least one atom is a `Braid` (otherwise it's a pure identity layer, handled
/// by Step 4).
fn is_braid_only_layer<G: PropSignature>(layer: &Layer<G>) -> bool {
    layer_has_braid(&layer.atoms)
        && layer
            .atoms
            .iter()
            .all(|a| matches!(a, Atom::Identity(_) | Atom::Braid(_, _)))
}

/// Split each `Identity(n)` atom into `n × Identity(1)`, leaving all other
/// atoms unchanged. Used to refine a gen layer so the naturality sweep can
/// cross an individual wire of a wide identity.
fn explode_identities<G: PropSignature>(atoms: &[Atom<G>]) -> Vec<Atom<G>> {
    let mut out: Vec<Atom<G>> = Vec::with_capacity(atoms.len());
    for a in atoms {
        if let Atom::Identity(n) = a {
            out.extend(std::iter::repeat_n(Atom::Identity(1), *n));
        } else {
            out.push(a.clone());
        }
    }
    out
}

/// Attempt a single naturality swap: `L_gen ; L_braid → L_braid' ; L_gen'`.
/// Returns the rewritten pair or `None` if no simple swap applies (e.g.,
/// braid position doesn't align with an atom boundary).
///
/// Identity-width refinement: `L_gen`'s `Identity(n)` atoms are first exploded
/// to `n × Identity(1)`, so the crossed pair need only have 1-wire *target*
/// after refinement — this lets a braid slide past a wide `Identity(n>1)` or a
/// pure-identity cover, not just two width-1 generators. The resulting gen
/// layer is re-fused with `merge_adjacent_identities`.
fn try_naturality_swap<G: PropSignature>(
    gen_layer: &Layer<G>,
    braid_layer: &Layer<G>,
) -> Option<(Layer<G>, Layer<G>)> {
    // Only a single-braid layer can slide as a whole: this rewrite replaces the
    // entire braid layer with the swapped gen layer, so a second braid would be
    // dropped. Multi-braid layers are normally eliminated before the sweep —
    // `canonicalize_braid_runs` (run at the top of `collect_braid_prefix`) splits
    // `Braid(1,1)` runs into single-braid layers, and `hexagon_expand` decomposes
    // wide braids in identity padding. This guard is the safety net for any
    // residual multi-braid layer.
    if braid_layer
        .atoms
        .iter()
        .filter(|a| matches!(a, Atom::Braid(_, _)))
        .count()
        > 1
    {
        return None;
    }
    // Locate the first Braid atom in braid_layer and its wire position.
    let mut braid_wire_pos: Option<usize> = None;
    let mut wire_cursor = 0;
    for atom in &braid_layer.atoms {
        match atom {
            Atom::Identity(n) => wire_cursor += n,
            Atom::Braid(1, 1) => {
                braid_wire_pos = Some(wire_cursor);
                break;
            }
            // Wider braids should have been decomposed by Step 1; bail out.
            _ => return None,
        }
    }
    let braid_wire_pos = braid_wire_pos?;

    // Refine gen-layer identities to unit width so the braid can cross a wide
    // identity or a pure-identity cover, not only two 1-target atoms — but only
    // allocate the exploded copy when a wide identity is actually present.
    // Otherwise scan the layer as-is (the common path, incl. every no-swap exit).
    let exploded;
    let gen_atoms: &[Atom<G>] = if gen_layer
        .atoms
        .iter()
        .any(|a| matches!(a, Atom::Identity(n) if *n > 1))
    {
        exploded = explode_identities(&gen_layer.atoms);
        &exploded
    } else {
        &gen_layer.atoms
    };

    // Find atoms at target-wire positions [braid_wire_pos, braid_wire_pos + 1].
    // Both must be 1-wire-target atoms for a clean swap.
    let mut cumulative_target = 0;
    let mut swap_idx: Option<usize> = None;
    for (i, atom) in gen_atoms.iter().enumerate() {
        if cumulative_target == braid_wire_pos && atom_target(atom) == 1 {
            if let Some(next) = gen_atoms.get(i + 1)
                && atom_target(next) == 1
            {
                swap_idx = Some(i);
            }
            break;
        }
        cumulative_target += atom_target(atom);
        if cumulative_target > braid_wire_pos {
            break;
        }
    }
    let idx = swap_idx?;

    // Build new braid_layer: σ_{s_a, s_b} at input-side wire position equal
    // to the source-side prefix width before atom `idx`.
    let atom_a = &gen_atoms[idx];
    let atom_b = &gen_atoms[idx + 1];
    let s_a = atom_source(atom_a);
    let s_b = atom_source(atom_b);
    let prefix_src: usize = gen_atoms[..idx].iter().map(atom_source).sum();
    let suffix_src: usize = gen_atoms[idx + 2..].iter().map(atom_source).sum();

    let mut new_braid_atoms: Vec<Atom<G>> = Vec::new();
    if prefix_src > 0 {
        new_braid_atoms.push(Atom::Identity(prefix_src));
    }
    new_braid_atoms.push(Atom::Braid(s_a, s_b));
    if suffix_src > 0 {
        new_braid_atoms.push(Atom::Identity(suffix_src));
    }

    // Build new gen_layer: same atoms, but with positions idx and idx+1
    // swapped, then re-fuse any exploded identities.
    let mut new_gen_atoms = gen_atoms.to_vec();
    new_gen_atoms.swap(idx, idx + 1);
    let new_gen_atoms = merge_adjacent_identities(new_gen_atoms);

    Some((
        Layer {
            atoms: new_braid_atoms,
        },
        Layer {
            atoms: new_gen_atoms,
        },
    ))
}

/// Canonicalize each maximal run of consecutive pure-braid (or pure-identity)
/// layers to its bubble-sort decomposition. Identity-only layers contribute
/// no swap to the underlying permutation but are absorbed into the run.
fn canonicalize_braid_runs<G: PropSignature>(layers: Vec<Layer<G>>) -> Vec<Layer<G>> {
    let mut out: Vec<Layer<G>> = Vec::new();
    let mut run: Vec<Layer<G>> = Vec::new();

    for layer in layers {
        let all_ident_or_braid = !layer.atoms.is_empty()
            && layer
                .atoms
                .iter()
                .all(|a| matches!(a, Atom::Identity(_) | Atom::Braid(1, 1)));
        if all_ident_or_braid {
            run.push(layer);
        } else {
            if !run.is_empty() {
                out.extend(canonicalize_run(&std::mem::take(&mut run)));
            }
            out.push(layer);
        }
    }
    if !run.is_empty() {
        out.extend(canonicalize_run(&run));
    }
    out
}

fn canonicalize_run<G: PropSignature>(run: &[Layer<G>]) -> Vec<Layer<G>> {
    if run.is_empty() {
        return Vec::new();
    }
    // Total wire width = source width of first layer in the run.
    let total: usize = run[0].atoms.iter().map(atom_source).sum();
    if total < 2 {
        // No possible crossings; just pass through a single identity layer.
        return vec![run[0].clone()];
    }

    // Apply each layer's swaps to compute the underlying permutation.
    let mut perm: Vec<usize> = (0..total).collect();
    for layer in run {
        apply_braid_layer_to_perm(layer, &mut perm);
    }

    // Canonical decomposition of perm via the shared `adjacent_swaps` core
    // (the same one behind `decompose_braid`), reversed for the same
    // undo-word → build-word reason documented there.
    let mut swaps = super::super::adjacent_swaps(&perm);
    swaps.reverse();

    if swaps.is_empty() {
        // Identity permutation — emit a single identity layer preserving width.
        vec![Layer {
            atoms: vec![Atom::Identity(total)],
        }]
    } else {
        swaps
            .into_iter()
            .map(|i| Layer {
                atoms: braid_at_position::<G>(i, total),
            })
            .collect()
    }
}

/// Update `perm` in place by applying each `Braid(1,1)` in `layer` as an
/// adjacent-wire swap. Identity atoms advance the wire cursor without
/// modifying the permutation.
fn apply_braid_layer_to_perm<G: PropSignature>(layer: &Layer<G>, perm: &mut [usize]) {
    let mut wire_pos = 0;
    for atom in &layer.atoms {
        match atom {
            Atom::Identity(n) => wire_pos += n,
            Atom::Braid(1, 1) => {
                perm.swap(wire_pos, wire_pos + 1);
                wire_pos += 2;
            }
            _ => {
                // Non-brick braid or generator shouldn't appear here;
                // canonicalize_braid_runs filters these.
                debug_assert!(false, "apply_braid_layer_to_perm: unexpected atom");
            }
        }
    }
}

/// **Step 4**: two sub-rules, both anchored to JS-I Ch 1 Prop 1.1 (rectangle-
/// cover independence).
///
/// (a) **Intra-layer identity merging** — adjacent `Atom::Identity(m)` and
/// `Atom::Identity(n)` within the same layer fuse to `Atom::Identity(m+n)`.
/// This is Rule 9 generalized (n-ary identity flattening).
///
/// (b) **Inter-layer identity absorption** — a layer consisting entirely of
/// `Atom::Identity` atoms absorbs into either neighbor when a non-identity
/// layer exists. If every layer is identity-only, retain one as the canonical
/// arity carrier (the NF of `Identity(n)` is a single identity layer, not
/// the empty diagram which is reserved for `Identity(0)`).
///
/// Paper anchor: JS-I Ch 1 Prop 1.1 p.66; JS-II Thm 1.1.3 p.4 (planar surgery).
fn coalesce_identity_layers<G: PropSignature>(sd: StringDiagram<G>) -> StringDiagram<G> {
    // Sub-rule (a): merge adjacent identity atoms within each layer.
    let layers: Vec<Layer<G>> = sd
        .layers
        .into_iter()
        .map(|layer| Layer {
            atoms: merge_adjacent_identities(layer.atoms),
        })
        .collect();

    // Sub-rule (b): drop pure-identity layers when at least one non-identity
    // layer remains.
    let has_non_identity = layers.iter().any(|l| !is_identity_only(l));
    if has_non_identity {
        let layers: Vec<Layer<G>> = layers
            .into_iter()
            .filter(|l| !is_identity_only(l))
            .collect();
        StringDiagram { layers }
    } else if layers.len() >= 2 {
        // All layers identity-only: keep just the first as canonical.
        let first = layers.into_iter().next().expect("len >= 2 checked");
        StringDiagram {
            layers: vec![first],
        }
    } else {
        StringDiagram { layers }
    }
}

fn is_identity_only<G: PropSignature>(layer: &Layer<G>) -> bool {
    !layer.atoms.is_empty() && layer.atoms.iter().all(|a| matches!(a, Atom::Identity(_)))
}

fn merge_adjacent_identities<G: PropSignature>(atoms: Vec<Atom<G>>) -> Vec<Atom<G>> {
    let mut out: Vec<Atom<G>> = Vec::with_capacity(atoms.len());
    for atom in atoms {
        if let Atom::Identity(n) = atom
            && let Some(Atom::Identity(prev_n)) = out.last_mut()
        {
            *prev_n += n;
            continue;
        }
        out.push(atom);
    }
    out
}

// -------------------------------------------------------------------------
// Connected components — the rule-(i) component-order anchor (issue #55)
// -------------------------------------------------------------------------

/// Boundary attachment of a connected component: the key rule (i) orders
/// components by (`docs/SMC-NF-RECONCILIATION.md` §2.6).
///
/// The order is `closed < input-anchored < output-only`, each anchored class
/// ordered by its least attached boundary coordinate. All closed components
/// share the key `(0, 0)` and so compare equal — "stable among themselves",
/// the block-level reading of Step 6's scalars-leftmost.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct CompKey {
    /// `0` = closed (attaches to neither boundary), `1` = input-anchored
    /// (attaches to the input boundary, possibly to both), `2` = output-only.
    class: u8,
    /// Least attached coordinate on the anchoring boundary; `0` when closed.
    coord: usize,
}

/// Connected-component decomposition of a [`StringDiagram`] with each
/// component's rule-(i) key.
///
/// Two atoms are connected when a wire leaving one at a layer boundary is a
/// wire entering the other at the same boundary and coordinate. `Identity` and
/// `Braid` atoms carry wires and so belong to components like any generator.
struct Components {
    /// `ids[j][i]` — component index of the atom at layer `j`, position `i`.
    ids: Vec<Vec<usize>>,
    /// Per-component rule-(i) key.
    keys: Vec<CompKey>,
    /// Per-component atom count. `1` marks the single-atom case that the
    /// §2.6 disjointness carve hands back to Decision 1 / Step 6.
    sizes: Vec<usize>,
    /// Guard 3: components whose attached coordinates on a boundary are not a
    /// contiguous interval, or into whose span another component intrudes.
    /// Rule (i)'s slot is ill-defined for these, so they are left alone.
    interleaved: Vec<bool>,
    /// Does the component occupy a wire on the diagram's **input** boundary?
    /// (`class == 1` also covers components that touch *both* boundaries, so
    /// the flags — not the class — are what Step 7's boundary-freedom test reads.)
    touches_input: Vec<bool>,
    /// Does the component occupy a wire on the diagram's **output** boundary?
    touches_output: Vec<bool>,
}

/// Half-open wire intervals occupied by each atom on one side of a boundary.
fn atom_intervals<G: PropSignature>(atoms: &[Atom<G>], use_target: bool) -> Vec<(usize, usize)> {
    let mut out = Vec::with_capacity(atoms.len());
    let mut p = 0;
    for a in atoms {
        let w = if use_target {
            atom_target(a)
        } else {
            atom_source(a)
        };
        out.push((p, p + w));
        p += w;
    }
    out
}

fn uf_find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}

fn uf_union(parent: &mut [usize], a: usize, b: usize) {
    let (ra, rb) = (uf_find(parent, a), uf_find(parent, b));
    if ra != rb {
        parent[ra.max(rb)] = ra.min(rb);
    }
}

/// Component of each wire on a boundary, read left to right.
fn wire_owners(intervals: &[(usize, usize)], comp_of: &[usize], offset: usize) -> Vec<usize> {
    let mut out = Vec::new();
    for (i, (s, e)) in intervals.iter().enumerate() {
        for _ in *s..*e {
            out.push(comp_of[offset + i]);
        }
    }
    out
}

/// Mark every component that breaks the "attached coordinates are disjoint
/// intervals ordered by least coordinate" condition on this boundary — either
/// because its own wires are split by another component, or because it sits
/// inside another component's span.
fn mark_interleaved(owners: &[usize], interleaved: &mut [bool]) {
    let mut runs: Vec<usize> = Vec::new();
    for &c in owners {
        if runs.last() != Some(&c) {
            runs.push(c);
        }
    }
    for (i, first) in runs.iter().enumerate() {
        for (k, second) in runs.iter().enumerate().skip(i + 1) {
            if first == second {
                for &m in &runs[i..=k] {
                    interleaved[m] = true;
                }
            }
        }
    }
}

/// Union-find the atoms of `sd` into connected components and classify each by
/// its boundary attachment (issue #55 rule (i), owner decision 2026-07-26).
fn analyze_components<G: PropSignature>(sd: &StringDiagram<G>) -> Components {
    let mut offsets = Vec::with_capacity(sd.layers.len() + 1);
    let mut total = 0;
    for l in &sd.layers {
        offsets.push(total);
        total += l.atoms.len();
    }
    offsets.push(total);

    let mut parent: Vec<usize> = (0..total).collect();
    // Two atoms connect when their wire intervals at a shared boundary share a
    // wire. Zero-width intervals (an ε's target, an η's source) contain no wire
    // and so connect to nothing — a zero-arity atom is joined only through its
    // non-empty side. The emptiness guard is load-bearing: without it the usual
    // two-sided overlap test reports a spurious hit for an empty interval that
    // merely *touches* a neighbour, which makes a zero-arity atom's component
    // depend on where it sits in its layer. Step 6 permutes exactly such atoms,
    // so its comparator would change under its own swaps and the `nf` fixpoint
    // would oscillate instead of terminating.
    for j in 1..sd.layers.len() {
        let ups = atom_intervals(&sd.layers[j - 1].atoms, /*use_target=*/ true);
        let downs = atom_intervals(&sd.layers[j].atoms, /*use_target=*/ false);
        let (mut a, mut b) = (0, 0);
        while a < ups.len() && b < downs.len() {
            let (a_start, a_end) = ups[a];
            let (b_start, b_end) = downs[b];
            let shares_a_wire =
                a_start < a_end && b_start < b_end && a_start < b_end && b_start < a_end;
            if shares_a_wire {
                uf_union(&mut parent, offsets[j - 1] + a, offsets[j] + b);
            }
            if a_end <= b_end {
                a += 1;
            } else {
                b += 1;
            }
        }
    }

    // Compact the union-find roots into dense component indices.
    let roots: Vec<usize> = (0..total).map(|x| uf_find(&mut parent, x)).collect();
    let mut label = vec![usize::MAX; total];
    let mut ncomp = 0;
    for &r in &roots {
        if label[r] == usize::MAX {
            label[r] = ncomp;
            ncomp += 1;
        }
    }
    let comp_of: Vec<usize> = roots.iter().map(|&r| label[r]).collect();

    let mut sizes = vec![0usize; ncomp];
    for &c in &comp_of {
        sizes[c] += 1;
    }

    let mut in_min = vec![usize::MAX; ncomp];
    let mut out_min = vec![usize::MAX; ncomp];
    let mut interleaved = vec![false; ncomp];
    if let Some(first) = sd.layers.first() {
        let owners = wire_owners(
            &atom_intervals(&first.atoms, /*use_target=*/ false),
            &comp_of,
            offsets[0],
        );
        for (w, &c) in owners.iter().enumerate() {
            in_min[c] = in_min[c].min(w);
        }
        mark_interleaved(&owners, &mut interleaved);
    }
    if let Some(last) = sd.layers.last() {
        let offset = offsets[sd.layers.len() - 1];
        let owners = wire_owners(
            &atom_intervals(&last.atoms, /*use_target=*/ true),
            &comp_of,
            offset,
        );
        for (w, &c) in owners.iter().enumerate() {
            out_min[c] = out_min[c].min(w);
        }
        mark_interleaved(&owners, &mut interleaved);
    }

    let keys: Vec<CompKey> = (0..ncomp)
        .map(|c| {
            if in_min[c] != usize::MAX {
                CompKey {
                    class: 1,
                    coord: in_min[c],
                }
            } else if out_min[c] != usize::MAX {
                CompKey {
                    class: 2,
                    coord: out_min[c],
                }
            } else {
                CompKey { class: 0, coord: 0 }
            }
        })
        .collect();

    let ids: Vec<Vec<usize>> = sd
        .layers
        .iter()
        .enumerate()
        .map(|(j, l)| comp_of[offsets[j]..offsets[j] + l.atoms.len()].to_vec())
        .collect();

    let touches_input: Vec<bool> = in_min.iter().map(|&v| v != usize::MAX).collect();
    let touches_output: Vec<bool> = out_min.iter().map(|&v| v != usize::MAX).collect();

    Components {
        ids,
        keys,
        sizes,
        interleaved,
        touches_input,
        touches_output,
    }
}

/// **Step 4(c)**: sift each `Atom::Generator` up to its earliest admissible
/// layer.
///
/// Greedy fixpoint: for every `Generator` `g` at layer `j` (`j ≥ 1`), move `g`
/// up into a **braid-free** layer `j−1` when its footprint there is admissible,
/// leaving `Identity(atom_target(g))` behind in its old slot. Iterate until no
/// generator can move. Two footprints:
/// - **Positive source** (`atom_source(g) > 0`): the whole consumed span at the
///   `j−1 ; j` boundary must lie inside one covering `Identity`
///   (`covering_identity`), which is split into pre/post pieces around `g`.
/// - **Zero source** (`η : 0 → n`): the empty span reduces to a single point at
///   the source cursor; `g` slides in iff that coordinate is an atom boundary
///   of layer `j−1` or strictly inside one of its identities
///   (`point_placement`), never strictly inside a generator's output span.
///   Where the coordinate leaves a *choice* of slot — a run of target-0 atoms
///   all sit at it — the slot is picked by [`component_slot`] from the
///   components' boundary attachment, **not** from the incidental cursor
///   (issue #55 rule (i); the cursor is presentation-dependent, which is the
///   defect the PR2-v2 re-cut fixes).
///
/// Soundness: this is bifunctoriality / interchange — JS-I Ch 1 §4 Thm 1.2
/// p.71, `(id ⊗ g) ; (h ⊗ id) = h ⊗ g`, generalized to the identity context
/// `(id_pre ⊗ g ⊗ id_post) ; (id_pre ⊗ id_g.target ⊗ id_post ⊗ …)`. Moving `g`
/// earlier through a column of identities (or, for `η`, across an atom boundary
/// where it shares no wire with either neighbour) preserves the morphism. The
/// pass is the topological-layer-order canonicalization of issue #14: it forces
/// the C2-gap witnesses (same morphism, independent atoms scheduled into
/// different layers) onto a single earliest-schedule form. Paper: JS-I Ch 1 §4
/// Thm 1.2 p.71.
///
/// Termination: each move strictly decreases the sum of the layer indices of
/// `Generator` atoms (one generator — positive- or zero-source — drops by one
/// layer; no other generator or braid moves), bounded below by zero.
///
/// Limitations / deliberate guards:
/// - **Zero-source `η` uses the component-anchored point-span rule, not a
///   covering-identity scan.** Its consumed span is empty, so the earliest
///   placement is fixed by the single output coordinate together with its
///   component's key. It is blocked when that coordinate is strictly inside a
///   generator's output span, and skipped entirely when its component is
///   *interleaved* (guard 3, `Components::interleaved`) — there rule (i)'s slot
///   is ill-defined and the pre-PR2 behaviour (no sift) stands. This subsumes
///   the `try_unitor_merge` zero-arity source patterns (both agree on the
///   2-atom boundary shape) without racing it (both moves only ever decrease
///   the `nf` fixpoint measure). Target-0 sinks (`ε : 1 → 0`) have a non-empty
///   source span and sift via the positive-source path.
/// - **Braids never sift and generators never sift into a braid-bearing layer.**
///   Braid placement is `collect_braid_prefix`'s responsibility (braids stay in
///   the leading layers); this guard keeps the two passes from oscillating in
///   the `nf` fixpoint.
fn topological_layer_order<G: PropSignature>(mut sd: StringDiagram<G>) -> StringDiagram<G> {
    // Precondition: adjacent identities are already fused, so a covering identity
    // region is always a single `Identity` atom (simplifies span lookup and
    // splitting). Established by `coalesce_identity_layers`, which runs
    // immediately before this in `nf`; `apply_sift` re-fuses both layers it
    // touches, so the property is maintained across the loop.
    debug_assert!(
        sd.layers.iter().all(|l| {
            l.atoms
                .windows(2)
                .all(|w| !matches!((&w[0], &w[1]), (Atom::Identity(_), Atom::Identity(_))))
        }),
        "topological_layer_order expects merged adjacent identities (run coalesce_identity_layers first)"
    );
    // Resume the scan just above the last sift: a sift at layer j only opens new
    // opportunities at layers j-1 and j+1, so restarting from layer 1 is wasteful.
    let mut start = 1;
    while let Some(sift) = find_sift(&sd, start) {
        let j = sift.j;
        apply_sift(&mut sd, &sift);
        start = (j - 1).max(1);
    }
    sd
}

/// A single applicable sift-up move located by [`find_sift`].
struct Sift {
    /// Layer of the generator being moved (`≥ 1`).
    j: usize,
    /// Index of the generator atom within layer `j`.
    idx: usize,
    /// Where the generator lands in layer `j − 1`.
    place: Placement,
}

/// How a sifted generator is inserted into the preceding (braid-free) layer.
#[derive(Clone, Copy)]
enum Placement {
    /// Split the covering `Identity` atom at index `k` into `Identity(pre)`,
    /// the generator, `Identity(post)` (zero-width pieces suppressed). Used for
    /// positive-source generators — whose consumed span lies inside one identity
    /// — and for a zero-source generator (`η`) whose output point falls
    /// *strictly* inside an identity (`pre > 0` and `post > 0`).
    SplitIdentity { k: usize, pre: usize, post: usize },
    /// Insert the (zero-source) generator at atom-boundary index `at`, between
    /// atoms `at − 1` and `at` (no split; `at == len` appends). Used when the
    /// output point sits on an atom boundary — e.g. between two generators'
    /// target spans, as in `[F, G]` — so there is no covering identity to split.
    /// This is the case the mid-layer-`η` witness needs (issue #55).
    Boundary { at: usize },
}

/// Locate the first generator (at layer `≥ start`) that can sift one layer
/// earlier. Scans layers top-to-bottom, left-to-right within a layer; returns
/// `None` at fixpoint. Callers pass `start = 1` initially; after a sift at
/// layer `j`, resuming from `max(1, j-1)` is complete because every layer below
/// `j-1` is unchanged and was already non-siftable.
fn find_sift<G: PropSignature>(sd: &StringDiagram<G>, start: usize) -> Option<Sift> {
    // Component analysis is only needed by the zero-source (η) branch, and most
    // diagrams have no η at all — build it on first use.
    let mut comps: Option<Components> = None;
    for j in start.max(1)..sd.layers.len() {
        let prev = &sd.layers[j - 1];
        // Guard: never sift into a layer that carries a braid.
        if layer_has_braid(&prev.atoms) {
            continue;
        }
        let cur = &sd.layers[j];
        // Source-side cursor: the generator's consumed span is [src_pos, src_pos + s).
        let mut src_pos = 0;
        for (idx, atom) in cur.atoms.iter().enumerate() {
            let s = atom_source(atom);
            if matches!(atom, Atom::Generator(_)) {
                if s > 0 {
                    // Positive-source generator: its whole consumed span must lie
                    // inside a single covering `Identity` of the previous layer.
                    if let Some((k, p, n)) = covering_identity(prev, src_pos, s) {
                        return Some(Sift {
                            j,
                            idx,
                            place: Placement::SplitIdentity {
                                k,
                                pre: src_pos - p,
                                post: (p + n) - (src_pos + s),
                            },
                        });
                    }
                } else {
                    // Zero-source generator (η): empty consumed span, so its
                    // placement is fixed by the wire coordinate `src_pos`
                    // together with its component's rule-(i) key (issue #55).
                    let c = comps.get_or_insert_with(|| analyze_components(sd));
                    if let Some(place) = eta_placement(prev, src_pos, c, j, idx) {
                        return Some(Sift { j, idx, place });
                    }
                }
            }
            src_pos += s;
        }
    }
    None
}

/// Find the `Identity` atom in `layer` whose target span `[p, p + n)` contains
/// `[start, start + width)`. Returns `(index, p, n)`. Assumes adjacent
/// identities are already merged, so at most one such atom exists.
fn covering_identity<G: PropSignature>(
    layer: &Layer<G>,
    start: usize,
    width: usize,
) -> Option<(usize, usize, usize)> {
    let mut p = 0;
    for (k, atom) in layer.atoms.iter().enumerate() {
        let w = atom_target(atom);
        if let Atom::Identity(n) = atom
            && p <= start
            && start + width <= p + n
        {
            return Some((k, p, *n));
        }
        p += w;
    }
    None
}

/// Placement for a **zero-source** generator (`η`) whose single output point
/// sits at wire coordinate `q` on the `j−1 ; j` boundary. Two admissible cases:
///
/// 1. `q` *strictly* inside an `Identity(n)` atom's target span `[p, p+n)`
///    (`p < q < p+n`) → split that identity ([`Placement::SplitIdentity`]).
/// 2. `q` on an atom boundary (the layer start, the layer end, or a cumulative
///    boundary between two atoms — including the two edges of an identity)
///    → insert between the adjacent atoms ([`Placement::Boundary`], no split).
///
/// Blocked (returns `None`) when `q` falls *strictly* inside a non-`Identity`
/// atom's target span: a `Generator`'s output wires (`prev` is braid-free, so
/// the only non-identity atoms are generators) cannot be split. Soundness is
/// the same interchange law as the positive-source sift — moving `η` earlier
/// through a column of identities (or across an atom boundary, where it shares
/// no wire with either neighbour) preserves the morphism. JS-I Ch 1 §4 Thm 1.2
/// p.71.
///
/// Case 2 returns the **leftmost** matching boundary. When a run of target-0
/// atoms (`ε`s) makes the coordinate repeat, that is only the *start* of the
/// admissible range; [`component_slot`] then picks the actual slot inside it.
fn point_placement<G: PropSignature>(prev: &Layer<G>, q: usize) -> Option<Placement> {
    let mut p = 0; // cumulative target position = boundary before atom `k`
    for (k, atom) in prev.atoms.iter().enumerate() {
        if p == q {
            return Some(Placement::Boundary { at: k });
        }
        let n = atom_target(atom);
        if p < q && q < p + n {
            // Strictly inside atom `k`.
            return match atom {
                Atom::Identity(_) => Some(Placement::SplitIdentity {
                    k,
                    pre: q - p,
                    post: (p + n) - q,
                }),
                // Strictly inside a generator's output span: cannot split.
                _ => None,
            };
        }
        p += n;
    }
    // `q` at (or past) the total target width: boundary at the very end.
    (p == q).then_some(Placement::Boundary {
        at: prev.atoms.len(),
    })
}

/// The component-anchored placement of a zero-source generator (`η`) — issue
/// #55 rule (i), owner decision 2026-07-26 (diagnosis-note addendum);
/// `docs/SMC-NF-RECONCILIATION.md` §2.6.
///
/// [`point_placement`] fixes the *coordinate*; when several slots realize that
/// coordinate (a run of target-0 atoms sits at it) rule (i) fixes *which*.
/// Three cases, by the class of the `η`'s own component:
///
/// - **Input-anchored** — its coordinate is already pinned by the component's
///   anchored layout, so the leftmost slot (the plain point-span behaviour)
///   stands.
/// - **Output-only or closed** — the slot comes from [`component_slot`]:
///   closed blocks leftmost, then input-anchored blocks, then output-only
///   blocks by least attached output coordinate.
/// - **Interleaved** (guard 3) — block transposition is not braid-free there
///   and the rule-(i) slot is ill-defined, so the `η` is not sifted at all.
fn eta_placement<G: PropSignature>(
    prev: &Layer<G>,
    q: usize,
    comps: &Components,
    j: usize,
    idx: usize,
) -> Option<Placement> {
    let eta_comp = comps.ids[j][idx];
    match point_placement(prev, q)? {
        // A strict-interior split has exactly one slot — no freedom to anchor.
        split @ Placement::SplitIdentity { .. } => Some(split),
        Placement::Boundary { at: lo } => {
            if comps.keys[eta_comp].class == 1 {
                return Some(Placement::Boundary { at: lo });
            }
            if comps.interleaved[eta_comp] {
                return None;
            }
            Some(Placement::Boundary {
                at: component_slot(prev, lo, comps, j - 1, eta_comp),
            })
        }
    }
}

/// Pick the rule-(i) slot inside the admissible range that starts at `lo`.
///
/// The range runs from `lo` over the following target-0 atoms: they all sit at
/// the same wire coordinate, so inserting the `η` anywhere among them denotes
/// the same morphism. The `η` moves right past every atom whose component sorts
/// **before** its own, and stops at the first that does not.
///
/// The §2.6 disjointness carve applies here: a tie between two *single-atom*
/// components is a strict-commutation freedom governed by Decision 1 / Step 6
/// (η first), so the `η` stops rather than passing such an atom.
fn component_slot<G: PropSignature>(
    prev: &Layer<G>,
    lo: usize,
    comps: &Components,
    prev_layer: usize,
    eta_comp: usize,
) -> usize {
    let key = comps.keys[eta_comp];
    let eta_is_single = comps.sizes[eta_comp] == 1;
    let mut at = lo;
    while at < prev.atoms.len() && atom_target(&prev.atoms[at]) == 0 {
        let c = comps.ids[prev_layer][at];
        if (eta_is_single && comps.sizes[c] == 1) || comps.interleaved[c] {
            break;
        }
        if comps.keys[c] >= key {
            break;
        }
        at += 1;
    }
    at
}

/// Apply a located [`Sift`]: insert the generator into layer `j − 1` (splitting
/// a covering identity or inserting at an atom boundary, per the [`Placement`])
/// and leave `Identity(target)` in layer `j`.
fn apply_sift<G: PropSignature>(sd: &mut StringDiagram<G>, sift: &Sift) {
    let Sift { j, idx, place } = sift;
    let (j, idx) = (*j, *idx);
    let g = sd.layers[j].atoms[idx].clone();
    let target = atom_target(&g);

    // Rebuild layer j−1 with the generator inserted per its placement.
    let prev_atoms = std::mem::take(&mut sd.layers[j - 1].atoms);
    let new_prev = match *place {
        Placement::SplitIdentity { k, pre, post } => {
            // Split the covering identity into `Identity(pre)`, the generator,
            // `Identity(post)`; zero-width pieces are suppressed.
            let mut new_prev = Vec::with_capacity(prev_atoms.len() + 2);
            for (kk, atom) in prev_atoms.into_iter().enumerate() {
                if kk == k {
                    if pre > 0 {
                        new_prev.push(Atom::Identity(pre));
                    }
                    new_prev.push(g.clone());
                    if post > 0 {
                        new_prev.push(Atom::Identity(post));
                    }
                } else {
                    new_prev.push(atom);
                }
            }
            new_prev
        }
        Placement::Boundary { at } => {
            // Insert the generator between atoms `at−1` and `at` (append if
            // `at == len`); `Vec::insert` handles both.
            let mut new_prev = prev_atoms;
            new_prev.insert(at, g.clone());
            new_prev
        }
    };
    sd.layers[j - 1].atoms = merge_adjacent_identities(new_prev);

    // Layer j keeps an identity of the generator's target width in its old slot;
    // fuse it with any neighbouring identities.
    sd.layers[j].atoms[idx] = Atom::Identity(target);
    let cur_atoms = std::mem::take(&mut sd.layers[j].atoms);
    sd.layers[j].atoms = merge_adjacent_identities(cur_atoms);
}

/// **Step 5**: remove any `Atom::Identity(0)` atoms within a layer; drop
/// layers that become empty as a result.
///
/// Paper anchor: JS-I Ch 1 §1 p.57 (Identity(0) is the ⊗ unit); Selinger
/// Table 2 p.10 ("unit is zero wires").
fn simplify_units<G: PropSignature>(sd: StringDiagram<G>) -> StringDiagram<G> {
    let layers: Vec<Layer<G>> = sd
        .layers
        .into_iter()
        .map(|layer| Layer {
            atoms: layer
                .atoms
                .into_iter()
                .filter(|a| !matches!(a, Atom::Identity(0)))
                .collect(),
        })
        .filter(|layer| !layer.atoms.is_empty())
        .collect();
    StringDiagram { layers }
}

// -------------------------------------------------------------------------
// Step 7 — block-level component transposition (issue #55 rule (i))
// -------------------------------------------------------------------------

/// **Step 7**: transpose adjacent **free component blocks** into rule-(i) order.
///
/// Step 6 reorders single *atoms* within a layer. Rule (i) states an order
/// between whole connected components, and transposing two multi-atom blocks is
/// a coupled multi-layer move that no single-atom pass can make — the residual
/// the PR2 diagnosis note named (`docs/SMC-NF-RECONCILIATION.md` §2.6). This
/// pass makes it.
///
/// # Free pair
///
/// Two components `C1`, `C2` may be transposed when
///
/// - **(a)** at least one is multi-atom — single ∥ single pairs are the §2.6
///   disjointness carve's territory and stay with Decision 1 / Step 6;
/// - **(b)** neither is interleaved (guard 3);
/// - **(c)** *boundary freedom*: at most one of the two occupies a wire on the
///   input boundary, and at most one occupies a wire on the output boundary.
///
/// (c) is exactly the condition that makes the transposition an equality rather
/// than a conjugation by braids. Writing `B1`, `B2` for the two blocks read as
/// morphisms, `B1 ⊗ B2 = σ ; (B2 ⊗ B1) ; σ′` with `σ = σ_{w1_in, w2_in}` and
/// `σ′ = σ_{w2_out, w1_out}`; both degenerate to identities precisely when one
/// of each pair of boundary widths is `0`. Equivalently, at the level of the
/// diagram's abstract content (anchored port graph): the swap moves no wire
/// between components and, under (c), leaves both boundary orderings fixed, so
/// it is the *same* morphism. By class, the free pairs are therefore
/// `closed ∥ anything` and `input-only ∥ output-only`; a component touching
/// *both* boundaries is pinned against everything except a closed one.
///
/// # Canonical order and adjacency
///
/// Rule (i)'s `CompKey` ascending — `closed(0) < input-anchored(1, coord) <
/// output-only(2, coord)`. **Equal keys never swap**, so the order among closed
/// blocks is stable, the block-level reading of Step 6's scalars-leftmost and
/// the same no-`Ord` caveat (`closed_closed_order_is_ord_less_residual`).
///
/// A pair is *adjacent* when, in **every** layer where both appear, each one's
/// atoms form a contiguous run and `C1`'s run sits immediately left of `C2`'s —
/// so no third component's atoms separate them anywhere. In a layer where only
/// one appears there is nothing to swap: a component's layer set is an interval,
/// so the absent one contributes zero wire width at the adjoining boundary and
/// the present one's wire coordinates do not move.
///
/// # Fused identities (the pre-split)
///
/// `merge_adjacent_identities` fuses an `Identity` across component boundaries,
/// and `analyze_components`' union-find then joins those components *through*
/// the fused atom. The pass therefore works on a refinement in which every
/// `Identity(n)` is split into `n × Identity(1)` — free, since
/// `Identity(a+b) = Identity(a) ⊗ Identity(b)` — analyses components there,
/// transposes, and re-fuses on the way out (the next fixpoint pass re-runs
/// `coalesce_identity_layers` regardless). The refinement is local to this pass;
/// Step 4(c) and Step 6 keep reading the unrefined form. If a split still leaves
/// a third component's atoms between `C1` and `C2`, the pair is simply not
/// adjacent and is skipped.
///
/// # Guards
///
/// A component carrying a `Braid` atom is never transposed: braid placement is
/// `collect_braid_prefix`'s business and `canonicalize_braid_runs` recomputes
/// braid runs from the underlying permutation, so keeping the two passes off
/// each other's atoms is what stops them oscillating.
///
/// # Termination
///
/// Strictly decreases `block_inversion_count` — summed over layers, the number
/// of position pairs `p < q` whose components form a free pair (a)+(b)+(c) with
/// inverted keys. A transposition flips every such pair between the two runs and
/// leaves every other pair's relative order alone, so the count drops by at least
/// one; and because (c) holds, neither block contributes wires at a boundary the
/// other attaches to, so all of `keys`, `sizes`, `interleaved` and the
/// attachment flags are invariant under the swap. `block_inversion_count` sits
/// between `layer_count` and `tied_inversion_count` in the `nf` measure
/// (`docs/SMC-NF-RECONCILIATION.md` §2.4).
///
/// Paper anchor: JS-I Ch 1 §4 Thm 1.2 p.71 (bifunctoriality) plus JS-I Ch 2 §1
/// axiom (S) p.73 in its degenerate `σ_{0,n} = id` form — the block-level
/// reading of the same two laws Step 6 uses at the atom level.
fn reorder_component_blocks<G: PropSignature>(sd: StringDiagram<G>) -> StringDiagram<G> {
    // A transposition needs two atoms side by side somewhere.
    if sd.layers.iter().all(|l| l.atoms.len() < 2) {
        return sd;
    }
    let mut refined = StringDiagram {
        layers: sd
            .layers
            .iter()
            .map(|l| Layer {
                atoms: explode_identities(&l.atoms),
            })
            .collect(),
    };
    let comps = analyze_components(&refined);
    let has_braid = braid_bearing_components(&refined, &comps);
    let mut ids = comps.ids.clone();
    let mut swapped = false;
    while let Some((c1, c2)) = find_block_transposition(&comps, &ids, &has_braid) {
        apply_block_transposition(&mut refined, &mut ids, c1, c2);
        swapped = true;
    }
    if !swapped {
        return sd;
    }
    debug_assert!(
        block_wiring_is_consistent(&refined, &ids),
        "reorder_component_blocks broke the per-wire component correspondence"
    );
    StringDiagram {
        layers: refined
            .layers
            .into_iter()
            .map(|l| Layer {
                atoms: merge_adjacent_identities(l.atoms),
            })
            .collect(),
    }
}

/// Which components hold a `Braid` atom (Step 7's braid guard).
fn braid_bearing_components<G: PropSignature>(
    sd: &StringDiagram<G>,
    comps: &Components,
) -> Vec<bool> {
    let mut out = vec![false; comps.keys.len()];
    for (row, layer) in comps.ids.iter().zip(sd.layers.iter()) {
        for (&c, atom) in row.iter().zip(layer.atoms.iter()) {
            if matches!(atom, Atom::Braid(_, _)) {
                out[c] = true;
            }
        }
    }
    out
}

/// Positions of component `c` in one layer's id row, as a half-open range.
/// `None` when `c` is absent from the layer **or** its positions are not
/// contiguous — Step 7 treats both as "not transposable here", and distinguishes
/// them via [`layer_holds`].
fn contiguous_run(row: &[usize], c: usize) -> Option<(usize, usize)> {
    let first = row.iter().position(|&x| x == c)?;
    let last = row
        .iter()
        .rposition(|&x| x == c)
        .expect("a first position implies a last one");
    row[first..=last]
        .iter()
        .all(|&x| x == c)
        .then_some((first, last + 1))
}

fn layer_holds(row: &[usize], c: usize) -> bool {
    row.contains(&c)
}

/// Step 7 conditions (a)–(c): is `{c1, c2}` a *free pair*?
fn block_pair_is_free(comps: &Components, c1: usize, c2: usize) -> bool {
    c1 != c2
        && (comps.sizes[c1] > 1 || comps.sizes[c2] > 1)
        && !comps.interleaved[c1]
        && !comps.interleaved[c2]
        && !(comps.touches_input[c1] && comps.touches_input[c2])
        && !(comps.touches_output[c1] && comps.touches_output[c2])
}

/// `c1`'s run immediately left of `c2`'s in every layer holding both.
fn blocks_are_adjacent(ids: &[Vec<usize>], c1: usize, c2: usize) -> bool {
    for row in ids {
        if !layer_holds(row, c1) || !layer_holds(row, c2) {
            continue;
        }
        let (Some((_, b1)), Some((a2, _))) = (contiguous_run(row, c1), contiguous_run(row, c2))
        else {
            return false;
        };
        if b1 != a2 {
            return false;
        }
    }
    true
}

/// The first adjacent free pair ordered against rule (i), scanning layers
/// top-to-bottom and positions left-to-right.
fn find_block_transposition(
    comps: &Components,
    ids: &[Vec<usize>],
    has_braid: &[bool],
) -> Option<(usize, usize)> {
    for row in ids {
        for w in row.windows(2) {
            let (c1, c2) = (w[0], w[1]);
            if c1 == c2
                || comps.keys[c2] >= comps.keys[c1]
                || has_braid[c1]
                || has_braid[c2]
                || !block_pair_is_free(comps, c1, c2)
                || !blocks_are_adjacent(ids, c1, c2)
            {
                continue;
            }
            return Some((c1, c2));
        }
    }
    None
}

/// Swap `c1`'s and `c2`'s runs in every layer holding both, keeping each run's
/// internal order (and the parallel `ids` rows) intact.
fn apply_block_transposition<G: PropSignature>(
    sd: &mut StringDiagram<G>,
    ids: &mut [Vec<usize>],
    c1: usize,
    c2: usize,
) {
    for (row, layer) in ids.iter_mut().zip(sd.layers.iter_mut()) {
        if !layer_holds(row, c1) || !layer_holds(row, c2) {
            continue;
        }
        if let (Some((a1, b1)), Some((a2, b2))) = (contiguous_run(row, c1), contiguous_run(row, c2))
        {
            debug_assert_eq!(b1, a2, "block transposition on a non-adjacent pair");
            layer.atoms[a1..b2].rotate_left(b1 - a1);
            row[a1..b2].rotate_left(b1 - a1);
        }
    }
}

/// Every wire still joins the same two components: the per-wire component
/// sequence read from layer `j`'s targets must equal the one read from layer
/// `j + 1`'s sources. Debug-only invariant check for [`reorder_component_blocks`].
fn block_wiring_is_consistent<G: PropSignature>(sd: &StringDiagram<G>, ids: &[Vec<usize>]) -> bool {
    (1..sd.layers.len()).all(|j| {
        let up = wire_owners(
            &atom_intervals(&sd.layers[j - 1].atoms, /*use_target=*/ true),
            &ids[j - 1],
            0,
        );
        let down = wire_owners(
            &atom_intervals(&sd.layers[j].atoms, /*use_target=*/ false),
            &ids[j],
            0,
        );
        up == down
    })
}

/// **Step 6**: canonical within-layer order for strictly-commuting zero-arity
/// atoms.
///
/// # Strict commutation criterion
///
/// Adjacent atoms `A`, `B` in a layer commute **strictly** — both connecting
/// braids are `σ_{0,n} = id`, so `A ⊗ B` and `B ⊗ A` are the same morphism on
/// the nose rather than merely up to symmetry — iff
///
/// ```text
/// (src A == 0 ∨ src B == 0) ∧ (tgt A == 0 ∨ tgt B == 0)
/// ```
///
/// Consequences:
/// - η's (`src = 0`, `tgt > 0`) never commute with each other, and ε's
///   (`src > 0`, `tgt = 0`) never commute with each other — their relative order
///   is coordinate-determined by disjoint non-empty wire spans.
/// - An η and an ε *do* commute at a tied adjacency.
/// - A `0 → 0` scalar commutes with **every** atom.
/// - A solid atom (`src > 0 ∧ tgt > 0`, which includes `Identity(n>0)` and every
///   `Braid`) never strictly commutes with an η or an ε.
///
/// # Canonical order: η before ε, scalars leftmost
///
/// Class order (issue #55 Decision 1, design of record 2026-07-25,
/// `.claude/docs/2026-07-25-55-tensor-order-canonicalization.md`):
///
/// ```text
/// scalar (0→0)  <  η (0→n)  <  ε (n→0)  <  solid
/// ```
///
/// # The disjointness carve (rule (i), 2026-07-26)
///
/// A tied adjacency between atoms of two *multi-atom* components is not only a
/// strict-commutation freedom, it is a **component transposition** — and those
/// are governed by rule (i)'s component-order anchor, not by the class order
/// (`docs/SMC-NF-RECONCILIATION.md` §2.6). The carve splits the two cleanly:
///
/// - both atoms in **single-atom** components → Decision 1 class order above;
/// - otherwise → component order `closed < input-anchored < output-only`, each
///   anchored class by least attached boundary coordinate, with the class order
///   as the tie-break when the two keys coincide (same component, or two closed
///   blocks);
/// - either component **interleaved** (guard 3) → fall back to the class order,
///   which is the pre-rule-(i) behaviour.
///
/// The pass is a within-layer **bubble reorder**: repeatedly swap an adjacent
/// pair `(A, B)` iff `A` and `B` strictly commute *and* `B` sorts before `A`.
/// This is the greedy (lex-least) normal form of the layer's word in the trace
/// monoid generated by strict commutation — one-directional, so it terminates by
/// the inversion count (each swap fixes exactly the pair it swaps and can never
/// re-invert it) and is confluent by the usual sorting argument.
///
/// Equal keys never swap, so the order **among scalars is stable** (input
/// order preserved), as is the order among closed blocks. The design note
/// records "scalars sorted by generator order" for generality, but
/// `PropSignature` carries no `Ord` bound and no shipped signature has a
/// `0 → 0` generator (Mat(R)/SFG scalars are `1 → 1`; `FrobeniusOr`'s η/ε are
/// `0 → 1` / `1 → 0`), so stable-among-scalars is the deliberate reading here;
/// a total scalar order awaits a signature that actually exercises it.
///
/// The pass is per-layer: it moves no atom across a layer boundary, rewrites no
/// atom, and is idempotent. Strict commutation also makes every swap
/// **connectivity-preserving** — one side of the pair has source width 0 and one
/// has target width 0, so no other atom's wire coordinates move — which is why
/// the component analysis stays valid across the whole pass (the per-atom
/// component ids are permuted alongside the atoms).
///
/// Paper anchor: JS-I Ch 1 §4 Thm 1.2 p.71 (bifunctoriality) specialized to a
/// 0-arity edge — the same `id_0`-unitor derivation [`try_unitor_merge`] uses.
fn reorder_tied_zero_arity<G: PropSignature>(mut sd: StringDiagram<G>) -> StringDiagram<G> {
    // No tied adjacency anywhere ⇒ nothing to reorder, and no reason to pay for
    // the component analysis (the common case: diagrams with no 0-arity atom).
    if !sd
        .layers
        .iter()
        .any(|l| l.atoms.windows(2).any(|w| strictly_commute(&w[0], &w[1])))
    {
        return sd;
    }
    let comps = analyze_components(&sd);
    for (j, layer) in sd.layers.iter_mut().enumerate() {
        let mut ids = comps.ids[j].clone();
        let mut swapped_any = false;
        loop {
            let mut changed = false;
            for i in 0..layer.atoms.len().saturating_sub(1) {
                if strictly_commute(&layer.atoms[i], &layer.atoms[i + 1])
                    && tie_sorts_before(
                        &comps,
                        (&layer.atoms[i + 1], ids[i + 1]),
                        (&layer.atoms[i], ids[i]),
                    )
                {
                    layer.atoms.swap(i, i + 1);
                    ids.swap(i, i + 1);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
            swapped_any = true;
        }
        if swapped_any {
            // A `0 → 0` scalar bubbling out from between two `Identity` atoms
            // leaves them adjacent; re-fuse so `topological_layer_order`'s
            // merged-identity precondition still holds on the next fixpoint pass.
            // (Unreachable with today's signatures — no shipped `G` has a 0→0
            // generator — but cheap and keeps the invariant unconditional.)
            layer.atoms = merge_adjacent_identities(std::mem::take(&mut layer.atoms));
        }
    }
    sd
}

/// The strict-commutation criterion of [`reorder_tied_zero_arity`]: `a ⊗ b` and
/// `b ⊗ a` denote the same morphism because both connecting braids degenerate
/// to `σ_{0,n} = id`.
fn strictly_commute<G: PropSignature>(a: &Atom<G>, b: &Atom<G>) -> bool {
    (atom_source(a) == 0 || atom_source(b) == 0) && (atom_target(a) == 0 || atom_target(b) == 0)
}

/// Does `b` sort strictly before `a` at a tied adjacency? Implements the §2.6
/// disjointness carve documented on [`reorder_tied_zero_arity`]: single-atom
/// pairs (and pairs touching an interleaved component) by the Decision-1 class
/// order, everything else by the rule-(i) component order with the class order
/// as tie-break.
///
/// Each argument is the atom paired with its component index.
fn tie_sorts_before<G: PropSignature>(
    comps: &Components,
    (b, cb): (&Atom<G>, usize),
    (a, ca): (&Atom<G>, usize),
) -> bool {
    let by_component = (comps.sizes[ca] > 1 || comps.sizes[cb] > 1)
        && !comps.interleaved[ca]
        && !comps.interleaved[cb];
    if by_component && comps.keys[cb] != comps.keys[ca] {
        return comps.keys[cb] < comps.keys[ca];
    }
    zero_arity_class(b) < zero_arity_class(a)
}

/// Rank of `a` in the canonical class order `scalar < η < ε < solid` used by
/// [`reorder_tied_zero_arity`].
fn zero_arity_class<G: PropSignature>(a: &Atom<G>) -> u8 {
    match (atom_source(a), atom_target(a)) {
        (0, 0) => 0, // scalar
        (0, _) => 1, // η (source)
        (_, 0) => 2, // ε (sink)
        _ => 3,      // solid
    }
}

// -------------------------------------------------------------------------
// C0 inline smoke tests: lowering + nf entry point work end-to-end.
// The paper-cited regression tests live in tests/smc_nf_regression.rs.
// -------------------------------------------------------------------------

#[cfg(test)]
mod c0_smoke {
    use super::*;
    use crate::prop::PropExpr;

    /// Minimal signature for smoke tests. Not re-exported — tests in
    /// `tests/smc_nf_regression.rs` define their own per-paper `TestSig`.
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    enum Sig {
        F,
    }
    impl PropSignature for Sig {
        fn source(&self) -> usize {
            1
        }
        fn target(&self) -> usize {
            1
        }
    }

    #[test]
    fn nf_of_identity_0_is_empty_diagram() {
        let e: PropExpr<Sig> = PropExpr::Identity(0);
        assert_eq!(nf(&e), StringDiagram { layers: Vec::new() });
    }

    #[test]
    fn nf_of_generator_is_single_atom_layer() {
        let e: PropExpr<Sig> = PropExpr::Generator(Sig::F);
        let expected = StringDiagram {
            layers: vec![Layer {
                atoms: vec![Atom::Generator(Sig::F)],
            }],
        };
        assert_eq!(nf(&e), expected);
    }

    #[test]
    fn nf_of_identity_n_is_single_identity_layer() {
        let e: PropExpr<Sig> = PropExpr::Identity(3);
        let expected = StringDiagram {
            layers: vec![Layer {
                atoms: vec![Atom::Identity(3)],
            }],
        };
        assert_eq!(nf(&e), expected);
    }

    #[test]
    fn nf_of_compose_concatenates_layers() {
        // f ; f : 1 → 1, lowered to two single-atom layers.
        let f = PropExpr::Generator(Sig::F);
        let e: PropExpr<Sig> = PropExpr::Compose(Box::new(f.clone()), Box::new(f));
        let nf_sd = nf(&e);
        // Without Steps 1–5 implemented yet, we don't assert canonical form —
        // only that the structure is two layers, each with one Generator atom.
        assert_eq!(nf_sd.layers.len(), 2);
        assert_eq!(nf_sd.layers[0].atoms.len(), 1);
        assert_eq!(nf_sd.layers[1].atoms.len(), 1);
    }

    #[test]
    fn nf_of_tensor_merges_layer_atoms() {
        // f ⊗ f : 2 → 2, lowered to one layer with two atoms.
        let f = PropExpr::Generator(Sig::F);
        let e: PropExpr<Sig> = PropExpr::Tensor(Box::new(f.clone()), Box::new(f));
        let nf_sd = nf(&e);
        assert_eq!(nf_sd.layers.len(), 1);
        assert_eq!(nf_sd.layers[0].atoms.len(), 2);
    }
}
