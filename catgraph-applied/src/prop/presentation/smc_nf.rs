//! SMC string-diagram normal form (Layer 1 of the normalizer).
//!
//! # Role
//!
//! This module provides, in place of the plain `apply_smc_rules` pre-pass, a
//! **Joyal-Street string-diagram normal form**: a total function
//! [`PropExpr`] → [`StringDiagram`] such that two expressions are SMC-equal
//! iff their NF values are structurally equal. The sibling
//! [`super::kb::CongruenceClosure`] (Layer 2) then operates on NF-normalized
//! terms and handles user-equation congruence without needing to know about
//! SMC axioms.
//!
//! # Algorithm (6 steps + empty-braid normalization)
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
//! - **Step 4(c)** — `topological_layer_order`: sift each non-identity-source
//!   generator up to its earliest braid-free layer (interchange scheduling).
//!   Paper: JS-I Ch 1 §4 Thm 1.2 p.71 (bifunctoriality / interchange); issue #14.
//! - **Step 5** — `simplify_units`: remove `Identity(0)` atoms.
//!   Paper: JS-I Ch 1 §1 p.57; Selinger Table 2 p.10.

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
/// - All `Atom::Braid` atoms appear in the leading (input-side) layers; no
///   generator layer is followed by a braid layer.
/// - No layer contains both a `Braid` and a `Generator` atom (mixed layers are
///   split by `isolate_mixed_braid_layers` and never re-created).
/// - Every `Generator` atom with non-zero source arity occupies its earliest
///   admissible layer: no generator's consumed wires all pass through
///   `Identity` atoms in the preceding braid-free layer (Step 4(c)).
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
/// Canonicality claim: for any two SMC-equal expressions `a`, `b` (i.e., equal
/// in the free symmetric monoidal category on `G`), `nf(&a) == nf(&b)`. The
/// converse holds by construction since `nf` applies only SMC-sound rewrites.
///
/// Known exception: a **zero-source generator** (`source == 0`, e.g. `η : 0 → 1`)
/// sitting mid-layer is not always scheduled canonically — `topological_layer_order`
/// skips source-0 atoms (their consumed span is empty, so the earliest-layer rule
/// is positionally ambiguous) and `try_unitor_merge` only absorbs the 2-atom
/// boundary pattern. So e.g. `nf(F ⊗ η ⊗ G)` and `nf((F ⊗ G) ; (id₁ ⊗ η ⊗ id₁))`
/// can differ. Generators with `source > 0` are unaffected. Tracked as the
/// Watch-item in `tests/smc_nf_completeness.rs` (issue #14 follow-up).
pub fn nf<G: PropSignature>(expr: &PropExpr<G>) -> StringDiagram<G> {
    let mut sd = lower(expr);
    // Fixpoint loop, terminating by the lexicographic measure
    // (crossings, mixed_layer_count, wide_braid_count, braid_position_sum,
    //  generator_position_sum, layer_count):
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
    // - `topological_layer_order` shrinks generator_position_sum;
    // - `coalesce_identity_layers`/`simplify_units` shrink layer_count.
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
    // Reversed sequence applied to identity reproduces σ_{m,n}. The reversal is
    // relative to `permutation_sfg`'s input→output emission order in
    // `crate::mat_to_sfg`: the string-diagram normalization pipeline applies
    // these layers in the opposite direction.
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

    // Canonical decomposition of perm via bubble sort (same algorithm as
    // hexagon_expand's decompose_braid).
    let mut sorted = perm.clone();
    let mut swaps: Vec<usize> = Vec::new();
    loop {
        let mut swapped = false;
        for i in 0..total - 1 {
            if sorted[i] > sorted[i + 1] {
                sorted.swap(i, i + 1);
                swaps.push(i);
                swapped = true;
            }
        }
        if !swapped {
            break;
        }
    }
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

/// **Step 4(c)**: sift each `Atom::Generator` up to its earliest admissible
/// layer.
///
/// Greedy fixpoint: for every `Generator` `g` at layer `j` (`j ≥ 1`) whose
/// consumed wire span at the `j−1 ; j` boundary is covered entirely by
/// `Identity` atoms of a **braid-free** layer `j−1`, move `g` up into layer
/// `j−1` (splitting the covering `Identity` into pre/post pieces around it) and
/// leave `Identity(atom_target(g))` behind in its old slot. Iterate until no
/// generator can move.
///
/// Soundness: this is bifunctoriality / interchange — JS-I Ch 1 §4 Thm 1.2
/// p.71, `(id ⊗ g) ; (h ⊗ id) = h ⊗ g`, generalized to the identity context
/// `(id_pre ⊗ g ⊗ id_post) ; (id_pre ⊗ id_g.target ⊗ id_post ⊗ …)`. Moving `g`
/// earlier through a column of identities preserves the morphism. The pass is
/// the topological-layer-order canonicalization of issue #14: it forces the
/// C2-gap witnesses (same morphism, independent atoms scheduled into different
/// layers) onto a single earliest-schedule form. Paper: JS-I Ch 1 §4 Thm 1.2
/// p.71.
///
/// Termination: each move strictly decreases the sum of the layer indices of
/// `Generator` atoms (one generator drops by one layer; no other generator or
/// braid moves), bounded below by zero.
///
/// Limitations / deliberate guards:
/// - **Zero-arity-source generators are skipped** (`atom_source(g) == 0`, e.g.
///   `η : 0 → 1`). Their consumed span is empty, so a covering identity is
///   found at every boundary position — sifting them is sound but positionally
///   ambiguous, and would race the `try_unitor_merge` zero-arity watch-item
///   (see `tests/smc_nf_completeness.rs` header). Target-0 sinks (`ε : 1 → 0`)
///   have a non-empty source span and sift normally.
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
    /// Index of the covering `Identity` atom within layer `j − 1`.
    k: usize,
    /// Slack identity width preceding the generator inside atom `k`.
    pre: usize,
    /// Slack identity width following the generator inside atom `k`.
    post: usize,
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
            // Skip zero-arity-source generators (η); their span is empty.
            if matches!(atom, Atom::Generator(_))
                && s > 0
                && let Some((k, p, n)) = covering_identity(prev, src_pos, s)
            {
                return Some(Sift {
                    j,
                    idx,
                    k,
                    pre: src_pos - p,
                    post: (p + n) - (src_pos + s),
                });
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

/// Apply a located [`Sift`]: insert the generator into layer `j − 1` (splitting
/// the covering identity) and leave `Identity(target)` in layer `j`.
fn apply_sift<G: PropSignature>(sd: &mut StringDiagram<G>, sift: &Sift) {
    let Sift {
        j,
        idx,
        k,
        pre,
        post,
    } = *sift;
    let g = sd.layers[j].atoms[idx].clone();
    let target = atom_target(&g);

    // Rebuild layer j−1 with the covering identity split into `Identity(pre)`,
    // the generator, `Identity(post)`; zero-width pieces are suppressed.
    let prev_atoms = std::mem::take(&mut sd.layers[j - 1].atoms);
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
