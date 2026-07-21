//! The realization `mat_to_sfg` — F&S 2018 Prop 5.56 (converse of Thm 5.53).
//!
//! Thm 5.53 ships the functor `S : SFG_R → Mat(R)` (see
//! [`crate::sfg_to_mat`]). Prop 5.56 states that `S` is **full/surjective**:
//! *every* matrix `M ∈ Mat(R)` is realized by some signal-flow graph. This
//! module implements the constructive converse `mat_to_sfg`, so that
//! `sfg_to_mat(mat_to_sfg(M)) == M` for every `M`.
//!
//! ## The four-layer construction (Prop 5.56 proof sketch + Exercise 5.59)
//!
//! For an `m × n` matrix `M` (m inputs, n outputs), the realization is the
//! composite `L1 ; L2 ; L3 ; L4` of four monoidal-product layers:
//!
//! - **L1 — copy/discard** `m → m·n`: each of the `m` inputs is fanned out to
//!   `n` copies via [`copy_n(n)`](crate::sfg::copy_n) (for `n = 0` this is
//!   `discard`). After L1, wire index `i·n + j` is the `j`-th copy of input `i`
//!   (**input-major** grouping).
//! - **L2 — scalars** `m·n → m·n`: wire `(i, j)` is scaled by the icon
//!   `Scalar(M(i, j))`, iterated in input-major order.
//! - **L3 — swaps/identities** `m·n → m·n`: the permutation regrouping
//!   input-major `i·n + j` to **output-major** `j·m + i`, realized as a
//!   bubble-sort network of adjacent transpositions (`id ⊗ σ ⊗ id`).
//! - **L4 — add/zero** `m·n → n`: each output `j` sums its `m` incoming wires
//!   via [`add_n(m)`](crate::sfg::add_n) (for `m = 0` this is `zero`).
//!
//! Key idea: there is exactly one path from input `i` to output `j`, and it
//! carries exactly one scalar icon `M(i, j)`, so `S` of the composite is `M`.
//!
//! Matrix-dimension convention (F&S Def 5.50 + Remark 5.49, matching
//! [`crate::sfg_to_mat`]): a morphism `m → n` is an `m × n` matrix — row index
//! = input wire, column index = output wire.
//!
//! ## Degenerate shapes
//!
//! The general four-layer composite degenerates naturally for every empty
//! dimension — no shape needs special-casing:
//!
//! - `m = 0` (`0 × n`): L1/L2/L3 collapse to `id(0)`; L4 is `n` copies of
//!   `zero` (= [`zero_n(n)`](crate::sfg::zero_n)), realizing the `0 × n` matrix.
//! - `n = 0` (`m × 0`): L1 is `m` copies of `discard` (=
//!   [`discard_n(m)`](crate::sfg::discard_n)); L2/L3/L4 collapse to `id(0)`.
//! - `0 × 0`: every layer is `id(0)`, so the composite is `id(0)`.

use catgraph::errors::CatgraphError;

use crate::{
    mat::MatR,
    rig::Rig,
    sfg::{SignalFlowGraph, add_n, copy_n},
};

/// Realize a matrix as a signal-flow graph (F&S Prop 5.56).
///
/// The returned SFG `g` satisfies `sfg_to_mat(g) == *m` and has
/// `g.domain() == m.rows()`, `g.codomain() == m.cols()`.
///
/// See the [module docs](self) for the four-layer construction.
///
/// # Errors
///
/// Returns [`CatgraphError::CompositionSizeMismatch`] only if the internal
/// layer arities fail to line up — a construction invariant that holds for all
/// well-formed [`MatR`], so this cannot occur for values obtained through the
/// safe [`MatR`] constructors. The `Result` mirrors [`copy_n`]/[`add_n`].
pub fn mat_to_sfg<R: Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static>(
    m: &MatR<R>,
) -> Result<SignalFlowGraph<R>, CatgraphError> {
    let rows = m.rows(); // m — input wires
    let cols = m.cols(); // n — output wires

    // Layer 1: fan each input out to `cols` copies. `copy_n(cols)` : 1 → cols
    // (`copy_n(0) = discard`). Composite: m → m·n.
    let mut layer1 = SignalFlowGraph::<R>::identity(0);
    for _ in 0..rows {
        let fan = copy_n::<R>(cols)?;
        layer1 = layer1.tensor(&fan);
    }

    // Layer 2: scale wire (i, j) by M(i, j), in input-major order (wire index
    // i·n + j). Empty when m·n = 0 (the fold seed id(0) carries through).
    let mut layer2 = SignalFlowGraph::<R>::identity(0);
    for row in m.entries() {
        for entry in row {
            let icon = SignalFlowGraph::<R>::scalar(entry.clone());
            layer2 = layer2.tensor(&icon);
        }
    }

    // Layer 3: regroup input-major (i·n + j) → output-major (j·m + i).
    let width = rows * cols;
    let mut perm = vec![0usize; width];
    for i in 0..rows {
        for j in 0..cols {
            perm[i * cols + j] = j * rows + i;
        }
    }
    let layer3 = permutation_sfg::<R>(&perm)?;

    // Layer 4: sum the `rows` wires feeding each output. `add_n(rows)` : rows → 1
    // (`add_n(0) = zero`). Composite: m·n → n.
    let mut layer4 = SignalFlowGraph::<R>::identity(0);
    for _ in 0..cols {
        let sum = add_n::<R>(rows)?;
        layer4 = layer4.tensor(&sum);
    }

    layer1.compose(&layer2)?.compose(&layer3)?.compose(&layer4)
}

/// Realize a permutation of `perm.len()` wires as an SFG built from adjacent
/// transpositions (`id_a ⊗ σ_{1,1} ⊗ id_b`).
///
/// Convention: a wire entering at position `p` exits at position `perm[p]`.
/// Under `S`, this is the permutation matrix with `entries[p][perm[p]] = 1`
/// (input row `p` → output column `perm[p]`), consistent with `braid_matrix`
/// in [`crate::sfg_to_mat`].
///
/// The network is produced by a bubble sort of `perm` into ascending order:
/// each adjacent swap during the sort becomes one braid layer, emitted
/// input-side first. `O(k²)` braid layers for `k = perm.len()`; the identity
/// is returned when `perm` is already sorted (including `k ≤ 1`).
///
/// # Errors
///
/// Returns [`CatgraphError::CompositionSizeMismatch`] only on an internal
/// arity bug; the layer widths are constructed to always line up.
fn permutation_sfg<R: Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static>(
    perm: &[usize],
) -> Result<SignalFlowGraph<R>, CatgraphError> {
    let k = perm.len();
    let mut arr = perm.to_vec();
    let mut g = SignalFlowGraph::<R>::identity(k);
    // Bubble sort: swapping physical positions t, t+1 is a braid layer applied
    // in input→output order, so the wire at position p is routed to perm[p].
    for pass in 0..k {
        for t in 0..k.saturating_sub(pass + 1) {
            if arr[t] > arr[t + 1] {
                arr.swap(t, t + 1);
                let layer = SignalFlowGraph::<R>::identity(t)
                    .tensor(&SignalFlowGraph::<R>::braid_1_1())
                    .tensor(&SignalFlowGraph::<R>::identity(k - t - 2));
                g = g.compose(&layer)?;
            }
        }
    }
    Ok(g)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rig::F64Rig;
    use crate::sfg_to_mat::sfg_to_mat;

    /// Build a `MatR<F64Rig>` from `f64` rows for terse test fixtures.
    fn matf(entries: &[&[f64]]) -> MatR<F64Rig> {
        let rows = entries.len();
        let cols = entries.first().map_or(0, |r| r.len());
        let data = entries
            .iter()
            .map(|r| r.iter().map(|&x| F64Rig(x)).collect())
            .collect();
        MatR::new(rows, cols, data).unwrap()
    }

    #[test]
    fn permutation_sfg_identity_when_sorted() {
        let g = permutation_sfg::<F64Rig>(&[0, 1, 2]).unwrap();
        let mat = sfg_to_mat(&g).unwrap();
        assert_eq!(mat, MatR::<F64Rig>::identity(3));
    }

    #[test]
    fn permutation_sfg_realizes_perm() {
        // input p → output perm[p]; entries[p][perm[p]] = 1.
        let perm = [2usize, 0, 1];
        let g = permutation_sfg::<F64Rig>(&perm).unwrap();
        let mat = sfg_to_mat(&g).unwrap();
        for (p, &q) in perm.iter().enumerate() {
            for col in 0..3 {
                let expect = if col == q { F64Rig(1.0) } else { F64Rig(0.0) };
                assert_eq!(mat.entries()[p][col], expect, "row {p} col {col}");
            }
        }
    }

    #[test]
    fn roundtrip_1x1() {
        let m = matf(&[&[7.0]]);
        let g = mat_to_sfg(&m).unwrap();
        assert_eq!(g.domain(), 1);
        assert_eq!(g.codomain(), 1);
        assert_eq!(sfg_to_mat(&g).unwrap(), m);
    }

    #[test]
    fn roundtrip_2x2() {
        let m = matf(&[&[1.0, 2.0], &[3.0, 4.0]]);
        let g = mat_to_sfg(&m).unwrap();
        assert_eq!(g.domain(), 2);
        assert_eq!(g.codomain(), 2);
        assert_eq!(sfg_to_mat(&g).unwrap(), m);
    }
}
