//! SMC string-diagram normal form (Layer 1 of the normalizer).
//!
//! # Role
//!
//! This module provides, in place of the plain `apply_smc_rules` pre-pass, a
//! **Joyal-Street string-diagram normal form**: a total function
//! [`PropExpr`] → [`StringDiagram`] aiming at "SMC-equal iff NF-equal". The
//! ⇐ direction holds unconditionally (every rewrite is SMC-sound); the ⇒
//! direction is **not proven, and not bounded**: the `smc_canonicality_probes`
//! suite verifies a named set of convergences, while a differential sweep still
//! finds SMC-equal pairs inside the fragment `𝔉` whose normal forms differ
//! (`docs/SMC-NF-RECONCILIATION.md` §4.6's ledger and §4.4's status; residual
//! scope on [`nf`]). Of the four *named* residuals, three are closed: the
//! closed↔closed equal-key case in #79 P1 with the in-situ reading key (see the
//! private `component_reading` helper), the trapped nesting and the sink form
//! of the nested-column case in #174 with the Step 6½ column pass (see the
//! private `reorder_zero_arity_columns` helper), and its source form with the
//! same issue's free-site retirement. The sibling
//! [`super::kb::CongruenceClosure`] (Layer 2) then operates on NF-normalized
//! terms and handles user-equation congruence without needing to know about
//! SMC axioms.
//!
//! # Algorithm (9 steps + empty-braid normalization)
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
//!   point-span rule at the leftmost slot its coordinate admits — the
//!   component-anchored slot walk was retired in #174).
//!   Paper: JS-I Ch 1 §4 Thm 1.2 p.71 (bifunctoriality / interchange); issue #14.
//! - **Step 5** — `simplify_units`: remove `Identity(0)` atoms.
//!   Paper: JS-I Ch 1 §1 p.57; Selinger Table 2 p.10.
//! - **Step 7** — `reorder_component_blocks`: transpose adjacent *free* component
//!   blocks (closed ∥ anything, input-only ∥ output-only) into rule-(i) order.
//!   Paper: JS-I Ch 1 §4 Thm 1.2 p.71 with JS-I Ch 2 §1 axiom (S) p.73 in its
//!   degenerate `σ_{0,n} = id` form; issue #55 rule (i).
//! - **Step 6½** — `reorder_zero_arity_columns`: transpose two adjacent
//!   interval-aligned *columns* whose block arities strictly commute — the move
//!   between Step 6 (atoms) and Step 7 (whole components).
//!   Paper: the same two laws as Step 7, read at column granularity; issue #174.
//! - **Step 6** — `reorder_tied_zero_arity`: within-layer canonical order for
//!   strictly-commuting zero-arity atoms (`scalar < η < ε < solid`, then
//!   `G::cmp`; content-only since #174 retired its component-order branch).
//!   Paper: JS-I Ch 1 §4 Thm 1.2 p.71 (bifunctoriality) with a 0-arity edge;
//!   issue #55 Decision 1 + rule (i).
//!
//! (Steps 7 and 6½ are staged *before* Step 6 in the fixpoint loop: a block or
//! column move can land an `η` beside an `ε`, and Step 6 repairs that on the
//! same pass.)

use super::super::{PropExpr, PropSignature};
use std::cmp::Ordering;

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
///   (`η`) can slide across into that layer at its wire coordinate (Step 4(c)).
///   Marking does not enter this clause — it stopped gating the sift in #174.
/// - Within every layer, no adjacent strictly-commuting pair is ordered against
///   the Step 6 order `scalar < η < ε < solid`, then `G::cmp` at an equal
///   class. That order reads the two atoms alone; the component-order branch it
///   used to open with was retired in #174.
/// - No adjacent *free* pair of connected components is ordered against rule
///   (i)'s component order, extended at the one tie that order admits (two
///   closed blocks) by the in-situ reading key (the private `component_reading`
///   helper). Step 7's free pair: at most one component attaching each
///   boundary, at least one multi-atom, neither interleaved, neither
///   braid-carrying.
/// - No adjacent pair of interval-aligned *columns* whose block arities
///   strictly commute is ordered against that same component order (Step 6½,
///   over the same refinement). Its guards: at least one multi-atom, neither
///   interleaved, neither braid-carrying, and the two keys distinct — an
///   equal-key (closed↔closed) pair is declined, not decided.
///
/// Both transposition clauses are conditional on those guards, so a marked or
/// braid-carrying component ordered "wrongly" is not an invariant breach.
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
/// guard 3's marking *and* touching a boundary; the closed-component exclusion
/// was added 2026-07-27 for the trapped-nesting residual, and the draft §4
/// proof was refuted the same day by a residual *inside* 𝔉. Both of those are
/// closed behaviourally as of #174 — the trapped nesting by the Step 6½ column
/// pass, the refuting family's source form by the free-site retirement — but
/// the proof gap the refutation opened stays open; see the §4.4 status).
/// - **Within-layer order** — Step 6 (`reorder_tied_zero_arity`, PR1) puts every
///   tied run in canonical order, so `nf(ε ⊗ η) == nf(η ⊗ ε)`.
/// - **Layer assignment** — a zero-source generator (`source == 0`, e.g.
///   `η : 0 → 1`) is scheduled by the point-span sift (PR2): its slot in the
///   earlier layer is the leftmost one its output coordinate admits. The
///   component-anchored slot walk that used to refine that choice was retired in
///   #174 — it made the normal form depend on the writing (`CE-R1`).
///   So `nf(F ⊗ η ⊗ G) == nf((F ⊗ G) ; (id₁ ⊗ η ⊗ id₁))`,
///   `nf(ε ; η) == nf(η ⊗ ε) == nf(ε ⊗ η)`, and the two-component witness
///   `nf((μ;!) ; (η;Δ)) == nf((μ;!) ⊗ (η;Δ))`.
/// - **Block order** — Step 7 (`reorder_component_blocks`) transposes whole
///   *free* component blocks into the same rule-(i) order, so the tensor-form
///   transpositions close too: `nf((μ;!) ⊗ (η;Δ)) == nf((η;Δ) ⊗ (μ;!))`, and a
///   closed block sorts leftmost regardless of where it was written. Two
///   *distinct* closed blocks share rule (i)'s one closed key and are separated
///   by the in-situ reading key (`component_reading`, #79 P1), so they too
///   converge from either writing.
///
/// - **Column order** — Step 6½ (`reorder_zero_arity_columns`, issue #174)
///   transposes a zero-arity-bounded *block* against a *column* of a larger
///   component, the move between what Step 6 (adjacent atoms) and Step 7 (whole
///   components) can do. So a block written **nested** inside another
///   component's wire span extracts to its free writing:
///   `nf(Copy ; (id₁ ⊗ η ⊗ id₁) ; (id₁ ⊗ ε ⊗ id₁) ; μ) == nf((η;ε) ⊗ (Copy;μ))`,
///   and likewise for a nested block solid on the side facing its encloser's
///   opening (the two cases that were residuals (c) and (d)).
///
/// **Residual scope.** The one *named* residual left is the interleave guard:
/// when a component's boundary attachment interleaves another's on the same
/// boundary, transposition is not braid-free, so Steps 7 and 6½ leave its blocks
/// and columns alone. (Its `η`s do sift, and its tied adjacencies do order —
/// #174 retired the guard from those two sites.)
///
/// That is a named residual, **not a bound**. A differential sweep over 100 000
/// random SFG expressions, each paired with a sound interchange rewriting of
/// itself, still finds 128 divergent pairs *inside* the fragment 𝔉 — unmarked,
/// boundary-attached, and mostly predating this machinery. `nf` remains sound on
/// every one of them; what is open is uniqueness. See
/// `docs/SMC-NF-RECONCILIATION.md` §2.6 (where rule (i)'s coordinates may be
/// read), §4.5 (the column move), §4.6 (the residual ledger and the measured
/// landscape), and the `smc_canonicality_probes` module in
/// `tests/smc_nf_completeness.rs`, which is the gate of record.
pub fn nf<G: PropSignature>(expr: &PropExpr<G>) -> StringDiagram<G> {
    let mut sd = lower(expr);
    // Fixpoint loop, terminating by the lexicographic measure
    // (crossings, mixed_layer_count, wide_braid_count, braid_position_sum,
    //  generator_position_sum, layer_count, block_inversion_count,
    //  column_inversion_count, tied_inversion_count):
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
    //   including a zero-source `η` point-span sift, drops
    //   exactly one generator by one layer and moves nothing else — issue #55);
    // - `coalesce_identity_layers`/`simplify_units` shrink layer_count;
    // - `reorder_component_blocks` (Step 7) shrinks block_inversion_count — each
    //   transposition flips every position pair between the two blocks' runs and
    //   leaves all other pairs' relative order alone. The count is taken against
    //   the lex order (rule-(i) key, in-situ reading key): the reading key
    //   records no wire coordinates, so — like the keys — it is invariant under
    //   the swap, and the same all-pairs argument covers the equal-key
    //   closed↔closed swap the reading key enables (#79 P1). It changes no
    //   layer's membership and rewrites no atom, so every earlier component is
    //   invariant under it. It may *raise* tied_inversion_count (a block move
    //   can land an η beside an ε), but that is the later component and Step 6
    //   repairs it on the same pass;
    // - `reorder_zero_arity_columns` (Step 6½) shrinks column_inversion_count —
    //   the interval-level analogue of block_inversion_count, counted over the
    //   pairs whose two components pass the pass's guards and have distinct keys.
    //   Each transposition flips every position pair between the two columns'
    //   runs and leaves all other pairs alone; the comparator core is
    //   `component_key_order`, and because one side of the pair is zero-width at
    //   each of the interval's own boundaries, the swap moves no boundary wire —
    //   so keys, sizes, markings and attachment flags are invariant under it.
    //   It changes no layer's membership and rewrites no atom, so every earlier
    //   component is invariant (block_inversion_count included: its free pairs
    //   are ordered by the same core, so a column swap can only lower it or
    //   leave it alone). Like Step 7 it may *raise* tied_inversion_count, the
    //   later component, which Step 6 repairs on the same pass;
    // - `reorder_tied_zero_arity` (Step 6) shrinks tied_inversion_count — each
    //   swap flips exactly one order-inverted pair and leaves every other pair's
    //   relative order alone. The order is `tie_sorts_before`'s: the Decision-1
    //   class order, then (two 0→0 scalar generators, the only atoms that can
    //   strictly commute at an equal class) `G::cmp`. Since #174 retired its
    //   rule-(i) branch, that order reads the two atoms and nothing else, so it
    //   cannot change under any pass and the count is well-defined without an
    //   invariance argument. Step 6 is within-layer only — it moves no atom
    //   between layers and rewrites no atom — so every earlier component is
    //   untouched, block_inversion_count and column_inversion_count included:
    //   a tied swap changes the relative order of exactly the pair it swaps, and
    //   both of those counts either exclude that pair (single∥single, marked,
    //   braid-carrying, or equal-key) or already order it the same way.
    //
    // Three steps can *raise* a later component while strictly shrinking an
    // earlier one, so the tuple still drops lexicographically and the later
    // pass repairs the ordering on the same fixpoint pass:
    // - `try_unitor_merge` raises tied_inversion_count (its case 1 prepends an ε
    //   ahead of the absorbed layer's atoms, possibly past an η; its case 4
    //   appends an η after them, possibly behind an ε; case 3's η-prepend is
    //   order-canonical) while shrinking layer_count. It changes no component's
    //   key, size or marking, so it cannot raise column_inversion_count.
    // - the point-span sift can land an `η` beside an `ε` (raising
    //   tied_inversion_count) and moves an atom between layers (so it *can*
    //   raise column_inversion_count) while shrinking generator_position_sum.
    // - a Step-7 block move raises tied_inversion_count while shrinking
    //   block_inversion_count; it cannot raise column_inversion_count, since
    //   both counts order the swapped pair by the same `component_key_order`.
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
        sd = reorder_blocks_and_columns(sd);
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
/// ordered by its least attached boundary coordinate.
///
/// Two *distinct* anchored components can never share a key — a least attached
/// coordinate is a wire owned by exactly one of them — so equal keys mean both
/// components are closed, all of which share `(0, 0)`. That one tie is broken
/// by the in-situ reading key ([`component_reading`], #79 P1): closed blocks
/// order by content, not by the order they were written in.
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

/// One atom's contribution to a component's **in-situ reading key**.
///
/// The derived order is the intended one: `Identity < Braid < Generator` by
/// variant tag, then widths numerically, then the generator by
/// `PropSignature: Ord` (issue #79 P1, Decision 2). No wire coordinate appears
/// — see [`component_reading`] for why that matters.
///
/// A component carrying a `Braid` is never transposed by Step 7 (the braid
/// guard), so the `Braid` arm exists only to keep the comparator total.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum ReadingAtom<'a, G: PropSignature> {
    Identity(usize),
    Braid(usize, usize),
    Generator(&'a G),
}

/// The **in-situ reading** of component `c`: its atoms in layer order,
/// left-to-right within each layer, mapped to [`ReadingAtom`] keys.
///
/// This is the content-derived total order that separates two closed
/// components, which rule (i)'s [`CompKey`] cannot (issue #79 P1, Decision 2).
/// Compared as a `Vec`, i.e. lexicographically with a prefix sorting before a
/// longer sequence.
///
/// **Offset-independence is the load-bearing property.** The reading records
/// only what the atoms *are*, never where they sit, so it is invariant under
/// Step 7's own transpositions — which permute whole runs and rewrite nothing.
/// A position-sensitive key would change under the swap it just licensed and
/// the `nf` fixpoint would oscillate instead of terminating (the same failure
/// mode `analyze_components`' emptiness guard avoids for Step 6). Equal
/// readings therefore mean the two blocks are identical, and transposing them
/// is invisible.
///
/// `ids` is the caller's per-layer component-id rows, which must be the ones
/// analysed from `sd`. Step 7 is the only caller, and it passes the refined
/// pair it rewrites on — the unrefined call site this note used to name
/// (`component_slot`) was retired in #174.
fn component_reading<'a, G: PropSignature>(
    sd: &'a StringDiagram<G>,
    ids: &[Vec<usize>],
    c: usize,
) -> Vec<ReadingAtom<'a, G>> {
    let mut out = Vec::new();
    for (row, layer) in ids.iter().zip(sd.layers.iter()) {
        for (&owner, atom) in row.iter().zip(layer.atoms.iter()) {
            if owner == c {
                out.push(match atom {
                    Atom::Identity(n) => ReadingAtom::Identity(*n),
                    Atom::Braid(m, n) => ReadingAtom::Braid(*m, *n),
                    Atom::Generator(g) => ReadingAtom::Generator(g),
                });
            }
        }
    }
    out
}

/// Order between two components *before* the closed↔closed reading-key
/// tie-break: rule (i)'s [`CompKey`], nothing else.
///
/// **Where rule (i)'s coordinates are allowed to be read** (issue #174 design
/// round, 2026-07-28). `CompKey` carries a boundary *coordinate*, and a
/// coordinate is only a function of the morphism where the geometry pins it.
/// The two **rewriting** passes — Step 7 and Step 6½ — verify that pinning
/// before they move anything: each checks that the transposition is braid-free
/// at both boundaries it spans, so the pair's order there really is geometry,
/// not a choice. The **free** decision sites do not have that guarantee, which
/// is why they no longer consult this order at all: the Step 4(c) `η` slot walk
/// and Step 6's tied comparator were retired in the same round (`component_slot`
/// and `tie_sorts_before`'s rule-(i) branch), leaving those sites
/// coordinate-free. See `docs/SMC-NF-RECONCILIATION.md` §2.6.
///
/// `Ordering::Equal` means both components are closed, the one tie rule (i)
/// admits; [`block_sorts_before`] resolves it by content.
fn component_key_order(comps: &Components, b: usize, a: usize) -> Ordering {
    comps.keys[b].cmp(&comps.keys[a])
}

/// Does component `b` sort strictly before component `a` in the block order —
/// [`component_key_order`], then, at the one tie that order admits (two closed
/// components), the in-situ [`component_reading`]?
///
/// Read by [`find_block_transposition`] (Step 7); Step 6½'s
/// [`find_column_transposition`] reads its [`component_key_order`] core. Those
/// two rewriting passes therefore cannot disagree with each other. They are the
/// only component-order consumers left — no free decision site reads either.
fn block_sorts_before<G: PropSignature>(
    sd: &StringDiagram<G>,
    comps: &Components,
    ids: &[Vec<usize>],
    b: usize,
    a: usize,
) -> bool {
    match component_key_order(comps, b, a) {
        Ordering::Less => true,
        Ordering::Greater => false,
        Ordering::Equal => component_reading(sd, ids, b) < component_reading(sd, ids, a),
    }
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
///   all sit at it — the **leftmost** is taken. The component-anchored walk
///   that used to choose among them was retired in #174: the slot is a genuinely
///   free choice, and rule (i)'s coordinates are writing-dependent there, so
///   consulting them made the normal form writing-dependent too (`CE-R1`).
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
/// - **Zero-source `η` uses the point-span rule, not a covering-identity scan.**
///   Its consumed span is empty, so the earliest placement is fixed by the
///   single output coordinate. It is blocked only when that coordinate is
///   strictly inside a generator's output span. Marking no longer enters: guard
///   3 stopped gating this sift in #174, so a marked component's `η` schedules
///   like any other (witness `marked_component_eta_sifts_and_converges`). This
///   subsumes the `try_unitor_merge` zero-arity source patterns (both agree on
///   the 2-atom boundary shape) without racing it (both moves only ever decrease
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
                    // placement is fixed by the wire coordinate `src_pos` alone —
                    // the leftmost slot that realizes it. The component-anchored
                    // slot walk that used to refine this choice was retired in
                    // the #174 design round (§2.6): the `η`'s slot among a run of
                    // target-0 atoms is a *free* choice, and rule (i)'s
                    // coordinates are not writing-invariant there.
                    if let Some(place) = point_placement(prev, src_pos) {
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
/// Case 2 returns the **leftmost** matching boundary, and since #174 that is
/// also the slot used: when a run of target-0 atoms (`ε`s) makes the coordinate
/// repeat, every slot in the run denotes the same morphism, and the leftmost is
/// taken rather than chosen by component order.
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
/// output-only(2, coord)` — with the **in-situ reading key** breaking the one
/// tie that key admits. Two distinct *anchored* components can never share a
/// key (a least attached coordinate is a wire owned by exactly one of them), so
/// equal keys mean both blocks are closed, and [`component_reading`] then
/// orders them by content: layer by layer, left to right within each layer,
/// compared lexicographically (issue #79 P1). The reading names no wire
/// coordinate, so it is invariant under this pass's own swaps; equal readings
/// mean the two blocks are identical and the transposition is invisible, so no
/// swap is made and none is needed. The whole comparator is
/// [`block_sorts_before`]; Step 6½ shares its [`component_key_order`] core.
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
/// of position pairs `p < q` whose components form a free pair (a)+(b)+(c) and
/// are inverted against the lex order (rule-(i) key, reading key). A
/// transposition flips every such pair between the two runs and leaves every
/// other pair's relative order alone, so the count drops by at least one; and
/// because (c) holds, neither block contributes wires at a boundary the other
/// attaches to, so all of `keys`, `sizes`, `interleaved` and the attachment
/// flags are invariant under the swap. The reading keys are invariant too — the
/// swap permutes whole runs and rewrites no atom, and the reading depends on no
/// position — so the same all-pairs argument carries the equal-key
/// closed↔closed swap. `block_inversion_count` sits
/// between `layer_count` and `tied_inversion_count` in the `nf` measure
/// (`docs/SMC-NF-RECONCILIATION.md` §2.4).
///
/// Paper anchor: JS-I Ch 1 §4 Thm 1.2 p.71 (bifunctoriality) plus JS-I Ch 2 §1
/// axiom (S) p.73 in its degenerate `σ_{0,n} = id` form — the block-level
/// reading of the same two laws Step 6 uses at the atom level.
fn reorder_component_blocks<G: PropSignature>(
    refined: &mut StringDiagram<G>,
    ids: &mut [Vec<usize>],
    comps: &Components,
    has_braid: &[bool],
) -> bool {
    let mut swapped = false;
    while let Some((c1, c2)) = find_block_transposition(refined, comps, ids, has_braid) {
        apply_block_transposition(refined, ids, c1, c2);
        swapped = true;
    }
    debug_assert!(
        !swapped || block_wiring_is_consistent(refined, ids),
        "reorder_component_blocks broke the per-wire component correspondence"
    );
    swapped
}

/// Steps 7 and 6½, sharing one identity-split refinement and one component
/// analysis.
///
/// The two passes run back-to-back in the `nf` loop and would otherwise build
/// the same refinement and the same union-find twice per iteration. They can
/// share both: neither rewrites an atom or moves one between layers, so
/// connectivity — and with it every field of [`Components`] — is invariant
/// across Step 7's swaps, and Step 6½ can keep reading the analysis Step 7 was
/// handed. (`ids` is permuted alongside the atoms, so the per-atom mapping stays
/// in step.) Re-fusing after Step 7 only to re-explode for Step 6½ would be a
/// round trip through the identical atom sequence, so the fuse happens once, on
/// the way out.
///
/// Ordering is Step 7 first: a block move can expose a column pair, and the
/// column pass runs to fixpoint after it.
fn reorder_blocks_and_columns<G: PropSignature>(sd: StringDiagram<G>) -> StringDiagram<G> {
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

    let blocks_moved = reorder_component_blocks(&mut refined, &mut ids, &comps, &has_braid);
    let columns_moved = reorder_zero_arity_columns(&mut refined, &mut ids, &comps, &has_braid);
    if !blocks_moved && !columns_moved {
        return sd;
    }
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

/// The first adjacent free pair ordered against the block order, scanning
/// layers top-to-bottom and positions left-to-right.
///
/// The order is [`block_sorts_before`]: rule (i)'s key, then the in-situ
/// reading key at the one tie it admits. The key comparison prunes first (it is
/// two integers); a reading is only ever built for a pair that has already
/// passed every guard and tied on its key — i.e. two adjacent free closed
/// blocks.
fn find_block_transposition<G: PropSignature>(
    sd: &StringDiagram<G>,
    comps: &Components,
    ids: &[Vec<usize>],
    has_braid: &[bool],
) -> Option<(usize, usize)> {
    for row in ids {
        for w in row.windows(2) {
            let (c1, c2) = (w[0], w[1]);
            // The raw-key prune is literally `component_key_order`'s body — it
            // exists only to skip the expensive guards on a pair that cannot be
            // inverted. Keep the two in step: if the order ever grows a clause,
            // this line has to grow it too or the prune starts lying.
            if c1 == c2
                || comps.keys[c2] > comps.keys[c1]
                || has_braid[c1]
                || has_braid[c2]
                || !block_pair_is_free(comps, c1, c2)
                || !blocks_are_adjacent(ids, c1, c2)
                || !block_sorts_before(sd, comps, ids, c2, c1)
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

// -------------------------------------------------------------------------
// Step 6½ — zero-arity column transposition (issue #174)
// -------------------------------------------------------------------------

/// The atom-index cuts of one column pair in one layer: `X` occupies
/// `[left, mid)`, `B` occupies `[mid, right)`.
type ColumnCuts = (usize, usize, usize);

/// A located column transposition: the layer interval `[j0, j0 + cuts.len())`
/// and its per-layer cuts.
struct ColumnSwap {
    j0: usize,
    cuts: Vec<ColumnCuts>,
}

/// **Step 6½**: transpose two adjacent **interval-aligned columns** whose block
/// arities strictly commute.
///
/// Step 6 lifts strict commutation to adjacent *atoms*; Step 7 lifts component
/// order to whole *components*. The move strictly between them — the one
/// residuals §4.6(c) and §4.6(d) both need — is a transposition of two
/// **columns**: `X ⊗ B = B ⊗ X` where `B` is a zero-arity-bounded block and `X`
/// is a column of a *larger* component, so Step 7's boundary-freedom test (a
/// whole-component test) never licenses it (`docs/SMC-NF-RECONCILIATION.md`
/// §4.5). This pass makes it.
///
/// # Columns and interval alignment
///
/// Like Step 7, the pass works over the identity-split refinement (every
/// `Identity(n)` split into `n × Identity(1)`, free since
/// `Identity(a+b) = Identity(a) ⊗ Identity(b)`), re-fusing on the way out.
///
/// A **column pair** is a layer interval `[j0, j1)` together with, in every one
/// of its layers, a contiguous run of `c1`'s atoms sitting immediately left of a
/// contiguous run of `c2`'s — `c1`'s run taken maximal, `c2`'s required to be
/// its whole presence in the layer. The pair is **interval-aligned** when all
/// three cuts (left of `X`, between `X` and `B`, right of `B`) sit at the same
/// wire coordinate read from above and from below at every internal boundary.
/// Alignment is what makes the interval's morphism factor as `A ⊗ X ⊗ B ⊗ C`,
/// so the move is a two-column tensor transposition rather than a conjugation.
///
/// # Block-level strict commutation
///
/// With `src`/`tgt` the columns' wire widths at the interval's own top and
/// bottom boundaries,
///
/// ```text
/// (src X == 0 ∨ src B == 0) ∧ (tgt X == 0 ∨ tgt B == 0)
/// ```
///
/// — Step 6's criterion read at column granularity. Both connecting braids are
/// then `σ_{0,n} = id`, so `X ⊗ B` and `B ⊗ X` are the same morphism on the
/// nose. A column closed over the interval (`0 → 0`) commutes with every other,
/// which is residual (c)'s extraction move for free. Because at each of the two
/// boundaries one side contributes no wire, the swap moves no wire coordinate
/// there — so when the interval abuts a diagram boundary, that boundary's owner
/// word is unchanged and every component key, size, marking and attachment flag
/// survives the swap.
///
/// # Canonical direction
///
/// [`component_key_order`] — the same core Step 7 and Step 6 read, so the three
/// passes cannot disagree. The pass declines the one tie that order admits (two
/// *closed* components): closed↔closed order is Step 7's whole-block reading key
/// (#79 P1), and leaving the tie there keeps Step 6's class-order fallback from
/// fighting this pass over the same adjacency.
///
/// # Guards
///
/// - a braid-carrying component is never transposed — braid placement stays
///   `collect_braid_prefix`'s (Step 3), and keeping the passes off each other's
///   atoms is what stops them oscillating;
/// - a marked (interleaved) component is never transposed — rule (i)'s slot is
///   ill-defined there, so residual (a) is unchanged;
/// - at least one of the two components is multi-atom: a single ∥ single pair is
///   the §2.6 disjointness carve's territory and stays with Decision 1 / Step 6.
///
/// # Termination
///
/// Strictly decreases `column_inversion_count` — summed over layers, the number
/// of position pairs `p < q` whose two components pass the guards above, have
/// distinct keys, and are inverted against [`component_key_order`]. A
/// transposition flips every such pair between the two runs (at least one, since
/// they are adjacent in every layer of the interval) and leaves every other
/// pair's relative order alone; the comparator itself is invariant under the
/// swap by the boundary argument above. The pass changes no layer's membership
/// and rewrites no atom, so every earlier measure component — including
/// `block_inversion_count`, whose free pairs are ordered by the same core — is
/// invariant or lowered. It may *raise* `tied_inversion_count`, the later
/// component, which Step 6 repairs on the same pass.
/// `column_inversion_count` sits between `block_inversion_count` and
/// `tied_inversion_count` (`docs/SMC-NF-RECONCILIATION.md` §2.4).
///
/// Paper anchor: JS-I Ch 1 §4 Thm 1.2 p.71 (bifunctoriality) plus JS-I Ch 2 §1
/// axiom (S) p.73 in its degenerate `σ_{0,n} = id` form — the column-level
/// reading of the same two laws Steps 6 and 7 cite.
fn reorder_zero_arity_columns<G: PropSignature>(
    refined: &mut StringDiagram<G>,
    ids: &mut [Vec<usize>],
    comps: &Components,
    has_braid: &[bool],
) -> bool {
    // The two diagram boundaries the swaps must leave alone. `block_wiring_is_consistent`
    // checks only *internal* boundaries — it would pass a swap that reordered the
    // diagram's own input or output wires, which is exactly what strict commutation
    // at the interval's ends is there to prevent. Capture them before any move.
    let before = boundary_owner_words(refined, ids);
    let mut swapped = false;
    while let Some(swap) = find_column_transposition(refined, comps, ids, has_braid) {
        apply_column_transposition(refined, ids, &swap);
        swapped = true;
    }
    debug_assert!(
        !swapped || block_wiring_is_consistent(refined, ids),
        "reorder_zero_arity_columns broke the per-wire component correspondence"
    );
    debug_assert!(
        !swapped || boundary_owner_words(refined, ids) == before,
        "reorder_zero_arity_columns moved a wire on a diagram boundary"
    );
    swapped
}

/// The per-wire component owner sequences on the diagram's **own** two
/// boundaries: layer 0 read on its sources, the last layer read on its targets.
/// Both are part of the morphism, so no transposition may change either.
fn boundary_owner_words<G: PropSignature>(
    sd: &StringDiagram<G>,
    ids: &[Vec<usize>],
) -> (Vec<usize>, Vec<usize>) {
    let first = sd.layers.first().map_or_else(Vec::new, |l| {
        wire_owners(&atom_intervals(&l.atoms, /*use_target=*/ false), &ids[0], 0)
    });
    let last = sd.layers.last().map_or_else(Vec::new, |l| {
        wire_owners(
            &atom_intervals(&l.atoms, /*use_target=*/ true),
            &ids[sd.layers.len() - 1],
            0,
        )
    });
    (first, last)
}

/// The maximal run of `c` in `row` ending at position `p` (inclusive), as the
/// half-open range `(start, p + 1)`. Callers have already checked `row[p] == c`.
fn run_ending_at(row: &[usize], p: usize, c: usize) -> (usize, usize) {
    let mut start = p;
    while start > 0 && row[start - 1] == c {
        start -= 1;
    }
    (start, p + 1)
}

/// The one layer's cuts for the pair `(c1, c2)`: `c2`'s whole presence in the
/// layer must be one contiguous run with `c1`'s atoms immediately left of it.
/// `None` when the layer does not hold that shape.
fn adjacent_column_cuts(row: &[usize], c1: usize, c2: usize) -> Option<ColumnCuts> {
    let (mid, right) = contiguous_run(row, c2)?;
    if mid == 0 || row[mid - 1] != c1 {
        return None;
    }
    let (left, _) = run_ending_at(row, mid - 1, c1);
    Some((left, mid, right))
}

/// One layer's cumulative wire coordinates: `src[k]` / `tgt[k]` are the total
/// source / target width of `atoms[..k]`, so a cut at atom index `k` sits at
/// wire coordinate `src[k]` from below and `tgt[k]` from above. Both have length
/// `atoms.len() + 1`.
///
/// [`find_column_transposition`] reads six of these per candidate interval
/// boundary inside an `O(layers²)` sub-interval search, so they are tabulated
/// once per search rather than summed per lookup.
struct LayerPrefixes {
    src: Vec<usize>,
    tgt: Vec<usize>,
}

fn layer_prefixes<G: PropSignature>(sd: &StringDiagram<G>) -> Vec<LayerPrefixes> {
    sd.layers
        .iter()
        .map(|l| {
            let mut src = Vec::with_capacity(l.atoms.len() + 1);
            let mut tgt = Vec::with_capacity(l.atoms.len() + 1);
            let (mut s, mut t) = (0, 0);
            src.push(0);
            tgt.push(0);
            for a in &l.atoms {
                s += atom_source(a);
                t += atom_target(a);
                src.push(s);
                tgt.push(t);
            }
            LayerPrefixes { src, tgt }
        })
        .collect()
}

/// Is this interval's column pair interval-aligned *and* strictly commuting?
/// See [`reorder_zero_arity_columns`] for both conditions.
fn column_pair_is_transposable(prefixes: &[LayerPrefixes], j0: usize, cuts: &[ColumnCuts]) -> bool {
    let (top_left, top_mid, top_right) = cuts[0];
    let top = &prefixes[j0];
    let src_x = top.src[top_mid] - top.src[top_left];
    let src_b = top.src[top_right] - top.src[top_mid];
    let last = cuts.len() - 1;
    let (bot_left, bot_mid, bot_right) = cuts[last];
    let bottom = &prefixes[j0 + last];
    let tgt_x = bottom.tgt[bot_mid] - bottom.tgt[bot_left];
    let tgt_b = bottom.tgt[bot_right] - bottom.tgt[bot_mid];
    if !((src_x == 0 || src_b == 0) && (tgt_x == 0 || tgt_b == 0)) {
        return false;
    }
    // Interval alignment: every cut is the same wire coordinate read from above
    // and from below at every internal boundary.
    (1..cuts.len()).all(|t| {
        let up = &prefixes[j0 + t - 1];
        let down = &prefixes[j0 + t];
        let (au, mu, bu) = cuts[t - 1];
        let (ad, md, bd) = cuts[t];
        up.tgt[au] == down.src[ad] && up.tgt[mu] == down.src[md] && up.tgt[bu] == down.src[bd]
    })
}

/// The guards of [`reorder_zero_arity_columns`], on the component pair alone.
fn column_pair_is_admissible(comps: &Components, has_braid: &[bool], c1: usize, c2: usize) -> bool {
    c1 != c2
        && (comps.sizes[c1] > 1 || comps.sizes[c2] > 1)
        && !comps.interleaved[c1]
        && !comps.interleaved[c2]
        && !has_braid[c1]
        && !has_braid[c2]
}

/// The first column transposition ordered against [`component_key_order`],
/// scanning layers top-to-bottom and positions left-to-right.
///
/// For each seeding adjacency the pass takes the **maximal** run of layers over
/// which the two components keep the same adjacent-run shape, then tries the
/// sub-intervals containing the seed longest-first, leftmost-first — a longer
/// interval carries more of the block across, and fixing the search order is
/// what keeps the pass deterministic.
fn find_column_transposition<G: PropSignature>(
    sd: &StringDiagram<G>,
    comps: &Components,
    ids: &[Vec<usize>],
    has_braid: &[bool],
) -> Option<ColumnSwap> {
    let prefixes = layer_prefixes(sd);
    for (j, row) in ids.iter().enumerate() {
        for p in 0..row.len().saturating_sub(1) {
            let (c1, c2) = (row[p], row[p + 1]);
            // `!= Less` also declines `Equal` — two closed components — which is
            // the tie carve: closed↔closed order belongs to Step 7's whole-block
            // reading key, and leaving it there is what keeps Step 6's
            // class-order fallback from undoing this pass over the same
            // adjacency.
            if !column_pair_is_admissible(comps, has_braid, c1, c2)
                || component_key_order(comps, c2, c1) != Ordering::Less
            {
                continue;
            }
            // Widen to the maximal layer run holding the same adjacent shape.
            // The seed adjacency alone does not guarantee it: `c2` may also sit
            // elsewhere in this very layer, and then it has no single run here.
            let Some(seed) = adjacent_column_cuts(row, c1, c2) else {
                continue;
            };
            let mut all = vec![seed];
            let mut lo = j;
            while lo > 0
                && let Some(cuts) = adjacent_column_cuts(&ids[lo - 1], c1, c2)
            {
                all.insert(0, cuts);
                lo -= 1;
            }
            while lo + all.len() < ids.len()
                && let Some(cuts) = adjacent_column_cuts(&ids[lo + all.len()], c1, c2)
            {
                all.push(cuts);
            }
            for len in (1..=all.len()).rev() {
                for j0 in lo..=(lo + all.len() - len) {
                    if !(j0..j0 + len).contains(&j) {
                        continue;
                    }
                    let cuts = &all[j0 - lo..j0 - lo + len];
                    if column_pair_is_transposable(&prefixes, j0, cuts) {
                        return Some(ColumnSwap {
                            j0,
                            cuts: cuts.to_vec(),
                        });
                    }
                }
            }
        }
    }
    None
}

/// Swap the two columns' runs in every layer of the interval, keeping each
/// run's internal order (and the parallel `ids` rows) intact.
fn apply_column_transposition<G: PropSignature>(
    sd: &mut StringDiagram<G>,
    ids: &mut [Vec<usize>],
    swap: &ColumnSwap,
) {
    for (t, &(left, mid, right)) in swap.cuts.iter().enumerate() {
        let j = swap.j0 + t;
        sd.layers[j].atoms[left..right].rotate_left(mid - left);
        ids[j][left..right].rotate_left(mid - left);
    }
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
/// # The disjointness carve, retired (issue #174 design round, 2026-07-28)
///
/// This pass used to open with rule (i): a tied adjacency between atoms of two
/// *multi-atom* components was read as a **component transposition** and ordered
/// by the component anchor, with the class order only as a fallback. The carve
/// that split the two freedoms by component size is gone, and with it the
/// component analysis this pass used to build — a tied adjacency is now always
/// the class order above, then `G::cmp`.
///
/// The reason is that a tied adjacency is a *free* choice: nothing in the
/// diagram pins which of the two commuting atoms comes first, so importing
/// rule (i)'s boundary coordinates made the normal form depend on how the
/// morphism was written. That is CE-R1 (`ce_r1_interchange_split_converges`),
/// an SMC-equal pair inside 𝔉 that the imported order separated. Component
/// order survives in the two *rewriting* passes, Steps 7 and 6½, which verify
/// braid-freedom at the boundaries they span before moving anything
/// (`docs/SMC-NF-RECONCILIATION.md` §2.6). Retiring the branch also retires the
/// §4.4 merge-monotonicity caveat, which existed only because this comparator
/// mixed the two orders.
///
/// The pass is a within-layer **bubble reorder**: repeatedly swap an adjacent
/// pair `(A, B)` iff `A` and `B` strictly commute *and* `B` sorts before `A`.
/// This is the greedy (lex-least) normal form of the layer's word in the trace
/// monoid generated by strict commutation — one-directional, so it terminates by
/// the inversion count (each swap fixes exactly the pair it swaps and can never
/// re-invert it) and is confluent by the usual sorting argument.
///
/// Equal *classes* among scalars swap: two
/// `0 → 0` generators sort ascending by `G::cmp`, the design note's "scalars
/// sorted by generator order", available since `PropSignature` gained its `Ord`
/// supertrait (issue #79 P1, Decision 2). Two equal generators still never
/// swap — `Ord`'s law makes that the same atom. No shipped signature has a
/// `0 → 0` generator (Mat(R)/SFG scalars are `1 → 1`; `FrobeniusOr`'s η/ε are
/// `0 → 1` / `1 → 0`), so the tie-break is inert on today's baselines and
/// exercised only by test signatures; it retires the caveat structurally rather
/// than waiting on a signature to exercise it.
///
/// The pass is per-layer: it moves no atom across a layer boundary, rewrites no
/// atom, and is idempotent. Strict commutation also makes every swap
/// **connectivity-preserving** — one side of the pair has source width 0 and one
/// has target width 0, so no other atom's wire coordinates move — which is what
/// keeps it from disturbing Steps 7 and 6½'s component analysis, even though it
/// no longer reads one itself.
///
/// Paper anchor: JS-I Ch 1 §4 Thm 1.2 p.71 (bifunctoriality) specialized to a
/// 0-arity edge — the same `id_0`-unitor derivation [`try_unitor_merge`] uses.
fn reorder_tied_zero_arity<G: PropSignature>(mut sd: StringDiagram<G>) -> StringDiagram<G> {
    // No tied adjacency anywhere ⇒ nothing to reorder (the common case: diagrams
    // with no 0-arity atom).
    if !sd
        .layers
        .iter()
        .any(|l| l.atoms.windows(2).any(|w| strictly_commute(&w[0], &w[1])))
    {
        return sd;
    }
    for layer in &mut sd.layers {
        let mut swapped_any = false;
        loop {
            let mut changed = false;
            for i in 0..layer.atoms.len().saturating_sub(1) {
                if strictly_commute(&layer.atoms[i], &layer.atoms[i + 1])
                    && tie_sorts_before(&layer.atoms[i + 1], &layer.atoms[i])
                {
                    layer.atoms.swap(i, i + 1);
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

/// Does `b` sort strictly before `a` at a tied adjacency? The Decision-1 class
/// order `scalar < η < ε < solid`, then — at an equal class, reachable only for
/// a `0 → 0` scalar pair — `G::cmp` (issue #79 P1).
///
/// **Content-only, by construction.** Both keys are read off the two atoms
/// alone: no wire coordinate, no component identity, nothing about where the
/// pair sits. The rule-(i) branch this comparator used to open with — component
/// keys whenever either component was multi-atom — was retired in the #174
/// design round (`docs/SMC-NF-RECONCILIATION.md` §2.6). A tied adjacency is a
/// *free* choice between two orders that the diagram does not pin, and rule
/// (i)'s coordinates are writing-dependent there, so importing them made the
/// normal form depend on how the morphism was written. Retiring the branch also
/// retires the §2.6 disjointness carve at this site and the §4.4
/// merge-monotonicity caveat, which existed only because the two orders mixed.
///
/// The `G::cmp` tie-break is reachable only for a `0 → 0` scalar pair. Equal
/// classes among strictly-commuting atoms leave no other option: two η's never
/// strictly commute with each other, nor do two ε's, nor two solids, and
/// `Identity(0)` — the only non-generator `0 → 0` atom, `Braid(0,0)` having
/// been normalized to it — is removed by `simplify_units` before this pass
/// runs.
fn tie_sorts_before<G: PropSignature>(b: &Atom<G>, a: &Atom<G>) -> bool {
    let (class_b, class_a) = (zero_arity_class(b), zero_arity_class(a));
    if class_b != class_a {
        return class_b < class_a;
    }
    match (b, a) {
        (Atom::Generator(gb), Atom::Generator(ga)) if class_b == 0 => gb < ga,
        _ => false,
    }
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
    #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
    enum Sig {
        F,
    }
    impl PropSignature for Sig {
        type Color = ();

        fn source_word(&self) -> std::borrow::Cow<'_, [()]> {
            crate::prop::mono_word(self.source())
        }
        fn target_word(&self) -> std::borrow::Cow<'_, [()]> {
            crate::prop::mono_word(self.target())
        }
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
