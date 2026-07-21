use itertools::Itertools;

use crate::errors::CatgraphError;

use {
    crate::{
        category::{ComposableMutating, HasIdentity},
        finset::Decomposition,
        monoidal::{
            GenericMonoidalMorphism, GenericMonoidalMorphismLayer, Monoidal,
            MonoidalMutatingMorphism, SymmetricMonoidalMorphism,
        },
        utils::in_place_permute,
    },
    permutations::Permutation,
    std::{collections::HashMap, convert::identity, fmt::Debug},
};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

use super::morphism_system::Contains;

/// Threshold for parallelizing block mutations in Frobenius layers when the
/// `parallel` feature is enabled.
#[cfg(feature = "parallel")]
const PARALLEL_BLOCK_THRESHOLD: usize = 64;

/// A single generator of a Frobenius algebra, typed by `Lambda`.
///
/// Six standard generators plus `Spider(z, m, n)` (m-to-n special Frobenius morphism)
/// and `UnSpecifiedBox` for opaque black-box operations.
#[allow(clippy::module_name_repetitions)]
#[derive(PartialEq, Eq, Clone)]
pub enum FrobeniusOperation<Lambda: Eq + Copy, BlackBoxLabel: Eq + Clone> {
    /// η: \[\] → \[z\] — the unit (creation).
    Unit(Lambda),
    /// μ: \[z, z\] → \[z\] — the multiplication (merge).
    Multiplication(Lambda),
    /// δ: \[z\] → \[z, z\] — the comultiplication (split).
    Comultiplication(Lambda),
    /// ε: \[z\] → \[\] — the counit (destruction).
    Counit(Lambda),
    /// id: \[z\] → \[z\] — identity wire.
    Identity(Lambda),
    /// σ: [z₁, z₂] → [z₂, z₁] — symmetric braiding (wire crossing).
    SymmetricBraiding(Lambda, Lambda),
    /// Special Frobenius morphism: m inputs to n outputs of type z.
    Spider(Lambda, usize, usize),
    /// Opaque black box with labeled source and target types.
    UnSpecifiedBox(BlackBoxLabel, Vec<Lambda>, Vec<Lambda>),
}

impl<Lambda, BlackBoxLabel> FrobeniusOperation<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy,
    BlackBoxLabel: Eq + Clone,
{
    fn source_size(&self) -> usize {
        /*
        how many wires incoming
        */
        match self {
            Self::Unit(_) => 0,
            Self::Comultiplication(_) | Self::Counit(_) | Self::Identity(_) => 1,
            Self::Multiplication(_) | Self::SymmetricBraiding(_, _) => 2,
            Self::UnSpecifiedBox(_, srcs, _) => srcs.len(),
            Self::Spider(_, d, _) => *d,
        }
    }

    fn target_size(&self) -> usize {
        /*
        how many wires outgoing
        */
        match self {
            Self::Counit(_) => 0,
            Self::Unit(_) | Self::Multiplication(_) | Self::Identity(_) => 1,
            Self::Comultiplication(_) | Self::SymmetricBraiding(_, _) => 2,
            Self::UnSpecifiedBox(_, _, tgts) => tgts.len(),
            Self::Spider(_, _, d) => *d,
        }
    }

    fn source_types(&self) -> Vec<Lambda> {
        /*
        labels of the wires incoming
        */
        match self {
            Self::Unit(_) => vec![],
            Self::Multiplication(z) => vec![*z, *z],
            Self::Comultiplication(z) | Self::Counit(z) | Self::Identity(z) => vec![*z],
            Self::SymmetricBraiding(z, w) => vec![*z, *w],
            Self::UnSpecifiedBox(_, srcs, _) => srcs.clone(),
            Self::Spider(z, d, _) => vec![*z; *d],
        }
    }

    fn target_types(&self) -> Vec<Lambda> {
        /*
        labels of the wires outgoing
        */
        match self {
            Self::Unit(z) | Self::Identity(z) | Self::Multiplication(z) => vec![*z],
            Self::Comultiplication(z) => vec![*z, *z],
            Self::Counit(_) => vec![],
            Self::SymmetricBraiding(z, w) => vec![*w, *z],
            Self::UnSpecifiedBox(_, _, tgts) => tgts.clone(),
            Self::Spider(z, _, d) => vec![*z; *d],
        }
    }

    fn hflip<F>(&mut self, black_box_changer: F)
    where
        F: Fn(BlackBoxLabel) -> BlackBoxLabel,
    {
        /*
        horizontal flip where the diagram is drawn left to right
        sources and targets switched
        */
        *self = match self {
            Self::Unit(z) => Self::Counit(*z),
            Self::Multiplication(z) => Self::Comultiplication(*z),
            Self::Comultiplication(z) => Self::Multiplication(*z),
            Self::Counit(z) => Self::Unit(*z),
            Self::Identity(z) => Self::Identity(*z),
            Self::SymmetricBraiding(z, w) => Self::SymmetricBraiding(*w, *z),
            Self::UnSpecifiedBox(label, srcs, tgts) => {
                Self::UnSpecifiedBox(black_box_changer(label.clone()), tgts.clone(), srcs.clone())
            }
            Self::Spider(z, d1, d2) => Self::Spider(*z, *d2, *d1),
        };
    }
}

#[derive(PartialEq, Eq, Clone)]
pub(crate) struct FrobeniusBlock<Lambda: Eq + Copy, BlackBoxLabel: Eq + Clone> {
    pub(crate) op: FrobeniusOperation<Lambda, BlackBoxLabel>,
    source_side_placement: usize,
    target_side_placement: usize,
}

impl<Lambda, BlackBoxLabel> Contains<BlackBoxLabel> for FrobeniusBlock<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy + Debug,
    BlackBoxLabel: Eq + Clone,
{
    fn contained_labels(&self) -> Vec<BlackBoxLabel> {
        match &self.op {
            FrobeniusOperation::UnSpecifiedBox(lab, _, _) => vec![lab.clone()],
            _ => vec![],
        }
    }
}

impl<Lambda, BlackBoxLabel> FrobeniusBlock<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy,
    BlackBoxLabel: Eq + Clone,
{
    fn new(
        op: FrobeniusOperation<Lambda, BlackBoxLabel>,
        source_side_placement: usize,
        target_side_placement: usize,
    ) -> Self {
        Self {
            op,
            source_side_placement,
            target_side_placement,
        }
    }

    fn source_size(&self) -> usize {
        self.op.source_size()
    }

    fn target_size(&self) -> usize {
        self.op.target_size()
    }

    fn hflip<F>(&mut self, black_box_changer: F)
    where
        F: Fn(BlackBoxLabel) -> BlackBoxLabel,
    {
        /*
        horizontal flip where the diagram is drawn left to right
        sources and targets switched
        */
        self.op.hflip(black_box_changer);
        std::mem::swap(
            &mut self.source_side_placement,
            &mut self.target_side_placement,
        );
    }

    fn is_identity(&self) -> bool {
        match self.op {
            FrobeniusOperation::Identity(_) => true,
            FrobeniusOperation::Spider(_, in_arms, out_arms) => in_arms == out_arms && in_arms == 1,
            _ => false,
        }
    }
}

#[derive(PartialEq, Eq, Clone)]
pub(crate) struct FrobeniusLayer<Lambda: Eq + Copy, BlackBoxLabel: Eq + Clone> {
    pub(crate) blocks: Vec<FrobeniusBlock<Lambda, BlackBoxLabel>>,
    pub(crate) left_type: Vec<Lambda>,
    pub(crate) right_type: Vec<Lambda>,
}

impl<Lambda, BlackBoxLabel> Contains<BlackBoxLabel> for FrobeniusLayer<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy + Debug,
    BlackBoxLabel: Eq + Clone,
{
    fn contained_labels(&self) -> Vec<BlackBoxLabel> {
        #[allow(clippy::redundant_closure_for_method_calls)]
        self.blocks
            .iter()
            .flat_map(|block| block.contained_labels())
            .collect_vec()
    }
}

impl<Lambda, BlackBoxLabel> FrobeniusLayer<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy,
    BlackBoxLabel: Eq + Clone,
{
    pub fn new() -> Self {
        Self {
            blocks: vec![],
            left_type: vec![],
            right_type: vec![],
        }
    }

    fn hflip<F>(&mut self, black_box_changer: &F)
    where
        F: Fn(BlackBoxLabel) -> BlackBoxLabel + Sync,
        Lambda: Send + Sync,
        BlackBoxLabel: Send + Sync,
    {
        /*
        horizontal flip where the diagram is drawn left to right
        sources and targets switched
        */
        // With the `parallel` feature on, `with_min_len` tells rayon's
        // LengthSplitter not to subdivide below the threshold, so small layer
        // blocks run as a single sequential task and large ones fan out across
        // workers. Without the feature (e.g. `wasm32-wasip1` single-threaded),
        // fall back to plain `iter_mut()`.
        #[cfg(feature = "parallel")]
        self.blocks
            .par_iter_mut()
            .with_min_len(PARALLEL_BLOCK_THRESHOLD)
            .for_each(|block| block.hflip(black_box_changer));
        #[cfg(not(feature = "parallel"))]
        self.blocks
            .iter_mut()
            .for_each(|block| block.hflip(black_box_changer));
        let temp = self.left_type.clone();
        self.left_type = self.right_type.clone();
        self.right_type = temp;
    }

    pub fn append_block(&mut self, op: FrobeniusOperation<Lambda, BlackBoxLabel>) {
        /*
        monoidal of this single layer morphism and op
        if the diagram is drawn left domain to right codomain
            and source and target types are read top to bottom
            this puts op at the bottom
        */
        let source_side_placement = self.left_type.len();
        let target_side_placement = self.right_type.len();
        self.left_type.extend(op.source_types());
        self.right_type.extend(op.target_types());
        self.blocks.push(FrobeniusBlock::new(
            op,
            source_side_placement,
            target_side_placement,
        ));
    }

    pub(crate) fn is_identity(&self) -> bool {
        #[allow(clippy::redundant_closure_for_method_calls)]
        self.blocks.iter().all(|cur_block| cur_block.is_identity())
    }

    /// Rebuild a layer from a list of operations, recomputing all placements
    /// and type vectors from scratch.
    fn rebuild_from_ops(ops: Vec<FrobeniusOperation<Lambda, BlackBoxLabel>>) -> Self {
        let mut layer = Self::new();
        for op in ops {
            layer.append_block(op);
        }
        layer
    }

    /// Attempt to simplify two adjacent layers using Frobenius laws.
    ///
    /// Returns `(self_is_identity, next_is_identity, mutations_occurred)`.
    ///
    /// **Rule 1 — Identity elimination**: layers consisting entirely of
    /// identity blocks are flagged so the caller can drop them.
    ///
    /// **Rule 2 — Braiding cancellation**: a `SymmetricBraiding(a, b)`
    /// followed by `SymmetricBraiding(b, a)` at matching wire positions
    /// collapses to two identity wires.
    ///
    /// **Rule 3 — Unit/Counit cancellation**: `Unit(z)` feeding directly
    /// into `Counit(z)` (a scalar loop) removes both blocks.
    ///
    /// **Rule 4 — Spider fusion**: `Spider(z, m, n)` followed by
    /// `Spider(z, n, k)` at matching wires fuses into `Spider(z, m, k)`.
    pub(crate) fn two_layer_simplify(&mut self, next_layer: &mut Self) -> (bool, bool, bool) {
        // Rule 1: identity check (no mutations needed)
        let self_id = self.is_identity();
        let next_id = next_layer.is_identity();
        if self_id || next_id {
            return (self_id, next_id, false);
        }

        // Build lookup: target_side_placement → index in self.blocks
        let mut target_pos_to_self_idx: HashMap<usize, usize> = HashMap::new();
        for (i, block) in self.blocks.iter().enumerate() {
            target_pos_to_self_idx.insert(block.target_side_placement, i);
        }

        // Track which blocks are matched for simplification.
        let mut self_matched: Vec<bool> = vec![false; self.blocks.len()];
        let mut next_matched: Vec<bool> = vec![false; next_layer.blocks.len()];

        // Replacement operations for matched blocks in self.
        // Key: self block index → replacement ops (may be empty, one, or two).
        let mut self_replacements: HashMap<usize, Vec<FrobeniusOperation<Lambda, BlackBoxLabel>>> =
            HashMap::new();
        // Replacement operations for matched blocks in next_layer.
        let mut next_replacements: HashMap<usize, Vec<FrobeniusOperation<Lambda, BlackBoxLabel>>> =
            HashMap::new();

        for (j, next_block) in next_layer.blocks.iter().enumerate() {
            if next_matched[j] {
                continue;
            }
            let src_pos = next_block.source_side_placement;
            if let Some(&i) = target_pos_to_self_idx.get(&src_pos) {
                if self_matched[i] {
                    continue;
                }
                let self_block = &self.blocks[i];
                // Wire-range matching: output of self feeds exactly into input of next
                if self_block.target_size() != next_block.source_size() {
                    continue;
                }

                // Rule 2: Braiding cancellation
                // σ(a,b) then σ(b,a) → two identity wires in each layer.
                //
                // The replacements must preserve each layer's OUTER type interfaces
                // (self.left_type and next_layer.right_type) so adjacent layers
                // above and below the simplified pair stay coupled.
                //
                // For self: SB(a1, b1) has source (a1, b1), target (b1, a1). We keep
                // self.left unchanged; self.right at these positions becomes (a1, b1)
                // instead of (b1, a1). The new interface between self and next is
                // (a1, b1) — matching next_layer.left_type's new positions.
                //
                // For next: SB(b2, a2) has source (b2, a2), target (a2, b2). We must
                // keep next.right unchanged at (a2, b2), so the replacement identities
                // must be ordered [Id(a2), Id(b2)] — NOT [Id(b2), Id(a2)]. That gives
                // next.left = (a2, b2) at those positions, which equals (a1, b1) and
                // couples consistently with self.right above.
                if let FrobeniusOperation::SymmetricBraiding(a1, b1) = &self_block.op
                    && let FrobeniusOperation::SymmetricBraiding(b2, a2) = &next_block.op
                    && a1 == a2
                    && b1 == b2
                {
                    self_replacements.insert(
                        i,
                        vec![
                            FrobeniusOperation::Identity(*a1),
                            FrobeniusOperation::Identity(*b1),
                        ],
                    );
                    next_replacements.insert(
                        j,
                        vec![
                            FrobeniusOperation::Identity(*a2),
                            FrobeniusOperation::Identity(*b2),
                        ],
                    );
                    self_matched[i] = true;
                    next_matched[j] = true;
                    continue;
                }

                // Rule 3: Unit/Counit cancellation
                // η(z) then ε(z) → both removed (scalar loop)
                if let FrobeniusOperation::Unit(z1) = &self_block.op
                    && let FrobeniusOperation::Counit(z2) = &next_block.op
                    && z1 == z2
                {
                    self_replacements.insert(i, vec![]);
                    next_replacements.insert(j, vec![]);
                    self_matched[i] = true;
                    next_matched[j] = true;
                    continue;
                }

                // Rule 4: Spider fusion
                // Spider(z, m, n) then Spider(z, n, k) → Spider(z, m, k)
                if let FrobeniusOperation::Spider(z1, m, n1) = &self_block.op
                    && let FrobeniusOperation::Spider(z2, n2, k) = &next_block.op
                    && z1 == z2
                    && n1 == n2
                {
                    self_replacements.insert(i, vec![FrobeniusOperation::Spider(*z1, *m, *k)]);
                    next_replacements.insert(j, vec![]);
                    self_matched[i] = true;
                    next_matched[j] = true;
                }
            }
        }

        if self_replacements.is_empty() && next_replacements.is_empty() {
            return (false, false, false);
        }

        // Rebuild self: iterate blocks in original order, replacing matched
        // blocks with their replacement ops and keeping unmatched blocks as-is.
        let mut self_ops: Vec<FrobeniusOperation<Lambda, BlackBoxLabel>> = Vec::new();
        for (i, block) in self.blocks.iter().enumerate() {
            if let Some(replacements) = self_replacements.get(&i) {
                self_ops.extend(replacements.iter().cloned());
            } else {
                self_ops.push(block.op.clone());
            }
        }

        let mut next_ops: Vec<FrobeniusOperation<Lambda, BlackBoxLabel>> = Vec::new();
        for (j, block) in next_layer.blocks.iter().enumerate() {
            if let Some(replacements) = next_replacements.get(&j) {
                next_ops.extend(replacements.iter().cloned());
            } else {
                next_ops.push(block.op.clone());
            }
        }

        // Rebuild both layers from the replacement op lists.
        *self = Self::rebuild_from_ops(self_ops);
        *next_layer = Self::rebuild_from_ops(next_ops);

        (self.is_identity(), next_layer.is_identity(), true)
    }
}

impl<Lambda, BlackBoxLabel> HasIdentity<Vec<Lambda>> for FrobeniusLayer<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy,
    BlackBoxLabel: Eq + Clone,
{
    fn identity(on_type: &Vec<Lambda>) -> Self {
        let mut answer = Self::new();
        for cur_type in on_type {
            answer.append_block(FrobeniusOperation::Identity(*cur_type));
        }
        answer
    }
}

impl<Lambda, BlackBoxLabel> Monoidal for FrobeniusLayer<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy,
    BlackBoxLabel: Eq + Clone,
{
    fn monoidal(&mut self, other: Self) {
        for new_op in other.blocks {
            self.append_block(new_op.op);
        }
    }
}

/// A string diagram morphism: a sequence of horizontal layers, each containing parallel generators.
///
/// Composition appends layers with automatic `two_layer_simplify` at the boundary.
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, PartialEq, Eq)]
pub struct FrobeniusMorphism<Lambda: Eq + Copy + Debug, BlackBoxLabel: Eq + Clone> {
    pub(crate) layers: Vec<FrobeniusLayer<Lambda, BlackBoxLabel>>,
}

impl<Lambda, BlackBoxLabel> Contains<BlackBoxLabel> for FrobeniusMorphism<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy + Debug,
    BlackBoxLabel: Eq + Clone,
{
    #[allow(clippy::redundant_closure_for_method_calls)]
    fn contained_labels(&self) -> Vec<BlackBoxLabel> {
        self.layers
            .iter()
            .flat_map(|layer| layer.contained_labels())
            .collect_vec()
    }
}

impl<Lambda: Eq + Copy + Debug, BlackBoxLabel: Eq + Clone>
    From<FrobeniusOperation<Lambda, BlackBoxLabel>> for FrobeniusMorphism<Lambda, BlackBoxLabel>
{
    fn from(op: FrobeniusOperation<Lambda, BlackBoxLabel>) -> Self {
        let mut answer_layer = FrobeniusLayer::new();
        answer_layer.append_block(op);
        let mut answer = Self::new();
        let _ = answer.append_layer(answer_layer);
        answer
    }
}

impl<Lambda, BlackBoxLabel> Default for FrobeniusMorphism<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy + Debug,
    BlackBoxLabel: Eq + Clone,
{
    fn default() -> Self {
        Self { layers: vec![] }
    }
}

impl<Lambda, BlackBoxLabel> FrobeniusMorphism<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy + Debug,
    BlackBoxLabel: Eq + Clone,
{
    /// Create an empty morphism with no layers.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of layers in this morphism (presentation depth, reducible via simplification).
    #[must_use]
    pub fn depth(&self) -> usize {
        self.layers.len()
    }

    pub(crate) fn append_layer(
        &mut self,
        next_layer: FrobeniusLayer<Lambda, BlackBoxLabel>,
    ) -> Result<(), CatgraphError> {
        /*
        composition with one more layer
        */
        if let Some(mut v) = self.layers.pop() {
            if v.right_type != next_layer.left_type {
                return Err(CatgraphError::Composition {
                    message: "type mismatch in frobenius morphims composition".to_string(),
                });
            }
            let mut temp_next_layer = next_layer.clone();
            let (v_id, temp_id, v_change) = v.two_layer_simplify(&mut temp_next_layer);
            if !v_id {
                if v_change && !self.layers.is_empty() {
                    /*
                    just 1 more step with the second to last layer
                    don't worry about if this exposes even more simplifications
                    with even earlier layers
                    */
                    let last_idx = self.layers.len() - 1;
                    let (_, v_now_id, _) = self.layers[last_idx].two_layer_simplify(&mut v);
                    if !v_now_id {
                        self.layers.push(v);
                    }
                } else {
                    self.layers.push(v);
                }
            } else if temp_id && self.layers.is_empty() {
                // Both layers simplified to identity and no earlier layers
                // remain. Keep v to preserve the domain/codomain interface.
                self.layers.push(v);
                return Ok(());
            }
            if !temp_id {
                self.layers.push(temp_next_layer);
            }
        } else {
            self.layers.push(next_layer);
        }
        Ok(())
    }

    pub(crate) fn hflip<F>(&mut self, black_box_changer: &F)
    where
        F: Fn(BlackBoxLabel) -> BlackBoxLabel + Sync,
        Lambda: Send + Sync,
        BlackBoxLabel: Send + Sync,
    {
        /*
        horizontal flip where the diagram is drawn left to right
        sources and targets switched
        */
        for layer in &mut self.layers {
            layer.hflip(black_box_changer);
        }
        self.layers.reverse();
    }
}

impl<Lambda, BlackBoxLabel> HasIdentity<Vec<Lambda>> for FrobeniusMorphism<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy + Debug,
    BlackBoxLabel: Eq + Clone,
{
    fn identity(on_this: &Vec<Lambda>) -> Self {
        Self {
            layers: vec![<_>::identity(on_this)],
        }
    }
}

impl<Lambda, BlackBoxLabel> Monoidal for FrobeniusMorphism<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy + Debug,
    BlackBoxLabel: Eq + Clone,
{
    #[allow(clippy::assigning_clones)]
    fn monoidal(&mut self, other: Self) {
        let self_len = self.layers.len();
        let others_len = other.layers.len();
        let mut last_other_type: Vec<_> = vec![];
        let mut last_self_type: Vec<_> = vec![];
        for (n, cur_self_layer) in self.layers.iter_mut().enumerate() {
            last_self_type = cur_self_layer.right_type.clone();
            if n < other.layers.len() {
                last_other_type = other.layers[n].right_type.clone();
                cur_self_layer.monoidal(other.layers[n].clone());
            } else {
                cur_self_layer.monoidal(<_>::identity(&last_other_type));
            }
        }
        for n in self_len..others_len {
            let mut new_layer = FrobeniusLayer::identity(&last_self_type);
            new_layer.monoidal(other.layers[n].clone());
            let _ = self.append_layer(new_layer);
        }
    }
}

impl<Lambda, BlackBoxLabel> ComposableMutating<Vec<Lambda>>
    for FrobeniusMorphism<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy + Debug,
    BlackBoxLabel: Eq + Clone,
{
    fn composable(&self, other: &Self) -> Result<(), CatgraphError> {
        if self.layers.is_empty() || other.layers.is_empty() {
            if self.layers.is_empty() && other.layers.is_empty() {
                return Ok(());
            }
            let interface = if self.layers.is_empty() {
                &other.layers[0].left_type
            } else {
                &self.layers.last().unwrap().right_type
            };
            return if interface.is_empty() {
                Ok(())
            } else {
                Err(CatgraphError::CompositionSizeMismatch {
                    expected: 0,
                    actual: interface.len(),
                })
            };
        }
        let self_interface = &self.layers.last().unwrap().right_type;
        let other_interface = &other.layers[0].left_type;
        if self_interface.len() != other_interface.len() {
            Err(CatgraphError::CompositionSizeMismatch {
                expected: self_interface.len(),
                actual: other_interface.len(),
            })
        } else if self_interface != other_interface {
            for idx in 0..self_interface.len() {
                let w1 = self_interface[idx];
                let w2 = other_interface[idx];
                if w1 != w2 {
                    return Err(CatgraphError::CompositionLabelMismatch {
                        index: idx,
                        expected: format!("{w1:?}"),
                        actual: format!("{w2:?}"),
                    });
                }
            }
            Err(CatgraphError::Composition {
                message: "Mismatch in labels of common interface at some unknown index."
                    .to_string(),
            })
        } else {
            Ok(())
        }
    }

    fn compose(&mut self, other: Self) -> Result<(), CatgraphError> {
        self.composable(&other)?;
        // composable has better error message than append_layer
        for next_layer in other.layers {
            self.append_layer(next_layer)?;
        }
        Ok(())
    }

    fn domain(&self) -> Vec<Lambda> {
        self.layers
            .first()
            .map(|x| x.left_type.clone())
            .unwrap_or_default()
    }

    fn codomain(&self) -> Vec<Lambda> {
        self.layers
            .last()
            .map(|x| x.right_type.clone())
            .unwrap_or_default()
    }
}

impl<Lambda, BlackBoxLabel> MonoidalMutatingMorphism<Vec<Lambda>>
    for FrobeniusMorphism<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy + Debug,
    BlackBoxLabel: Eq + Clone,
{
}

impl<Lambda: Eq + Copy + Debug, BlackBoxLabel: Eq + Clone>
    From<GenericMonoidalMorphismLayer<(BlackBoxLabel, Vec<Lambda>, Vec<Lambda>), Lambda>>
    for FrobeniusLayer<Lambda, BlackBoxLabel>
{
    fn from(
        value: GenericMonoidalMorphismLayer<(BlackBoxLabel, Vec<Lambda>, Vec<Lambda>), Lambda>,
    ) -> Self {
        let mut new_blocks: Vec<FrobeniusBlock<Lambda, BlackBoxLabel>> =
            Vec::with_capacity(value.blocks.len());
        let mut src_side_shift = 0;
        let mut tgt_side_shift = 0;
        for (op, dom_op, cod_op) in value.blocks {
            let dom_op_len = dom_op.len();
            let cod_op_len = cod_op.len();
            let frob_op = FrobeniusOperation::UnSpecifiedBox(op, dom_op, cod_op);
            new_blocks.push(FrobeniusBlock {
                op: frob_op,
                source_side_placement: src_side_shift,
                target_side_placement: tgt_side_shift,
            });
            src_side_shift += dom_op_len;
            tgt_side_shift += cod_op_len;
        }
        Self {
            blocks: new_blocks,
            left_type: value.left_type,
            right_type: value.right_type,
        }
    }
}

impl<Lambda: Eq + Copy + Debug, BlackBoxLabel: Eq + Clone>
    From<GenericMonoidalMorphism<(BlackBoxLabel, Vec<Lambda>, Vec<Lambda>), Lambda>>
    for FrobeniusMorphism<Lambda, BlackBoxLabel>
{
    fn from(
        value: GenericMonoidalMorphism<(BlackBoxLabel, Vec<Lambda>, Vec<Lambda>), Lambda>,
    ) -> Self {
        Self {
            layers: value
                .extract_layers()
                .into_iter()
                .map(FrobeniusLayer::from)
                .collect(),
        }
    }
}

impl<Lambda, BlackBoxLabel> SymmetricMonoidalMorphism<Lambda>
    for FrobeniusMorphism<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
{
    fn permute_side(&mut self, p: &permutations::Permutation, of_codomain: bool) {
        if of_codomain {
            assert_eq!(p.len(), self.codomain().len());
            let p_frob = Self::from_permutation(p.inv(), &self.codomain(), true).unwrap();
            self.compose(p_frob).unwrap();
        } else {
            self.hflip(&identity);
            self.permute_side(p, true);
            self.hflip(&identity);
        }
    }

    fn from_permutation(
        p: permutations::Permutation,
        types: &[Lambda],
        types_as_on_domain: bool,
    ) -> Result<Self, CatgraphError> {
        if !types_as_on_domain {
            let mut answer = Self::from_permutation(p.inv(), types, true)?;
            answer.hflip(&identity);
            return Ok(answer);
        }

        if p == Permutation::identity(p.len()) {
            return Ok(Self::identity(&types.to_vec()));
        }
        let mut types_now = types.to_vec();
        let mut p_remaining = p.clone();
        let mut first_layer = Self::new();
        for idx in (0..p_remaining.len() - 1).step_by(2) {
            let idx_goes = p_remaining.apply(idx);
            let jdx_goes = p_remaining.apply(idx + 1);
            if idx_goes > jdx_goes {
                let cur_swap = Permutation::transposition(p_remaining.len(), idx, idx + 1);
                first_layer.monoidal(
                    FrobeniusOperation::SymmetricBraiding(types_now[idx], types_now[idx + 1])
                        .into(),
                );
                in_place_permute(&mut types_now, &cur_swap);
                p_remaining = cur_swap * p_remaining;
            } else {
                first_layer.monoidal(FrobeniusOperation::Identity(types_now[idx]).into());
                first_layer.monoidal(FrobeniusOperation::Identity(types_now[idx + 1]).into());
            }
        }
        if p_remaining.len() % 2 == 1 {
            first_layer
                .monoidal(FrobeniusOperation::Identity(types_now[p_remaining.len() - 1]).into());
        }
        let mut second_layer: Self = FrobeniusOperation::Identity(types_now[0]).into();
        for idx in (1..p_remaining.len() - 1).step_by(2) {
            let idx_goes = p_remaining.apply(idx);
            let jdx_goes = p_remaining.apply(idx + 1);
            if idx_goes > jdx_goes {
                let cur_swap = Permutation::transposition(p_remaining.len(), idx, idx + 1);
                second_layer.monoidal(
                    FrobeniusOperation::SymmetricBraiding(types_now[idx], types_now[idx + 1])
                        .into(),
                );
                in_place_permute(&mut types_now, &cur_swap);
                p_remaining = cur_swap * p_remaining;
            } else {
                second_layer.monoidal(FrobeniusOperation::Identity(types_now[idx]).into());
                second_layer.monoidal(FrobeniusOperation::Identity(types_now[idx + 1]).into());
            }
        }
        if p_remaining.len().is_multiple_of(2) {
            second_layer
                .monoidal(FrobeniusOperation::Identity(types_now[p_remaining.len() - 1]).into());
        }
        first_layer.compose(second_layer).unwrap();
        let remaining = Self::from_permutation(p_remaining, &types_now, true)?;
        first_layer.compose(remaining).unwrap();
        assert_eq!(first_layer.domain(), types);
        let mut types_after_all_p = types.to_vec();
        in_place_permute(&mut types_after_all_p, &p.inv());
        assert_eq!(first_layer.codomain(), types_after_all_p);
        Ok(first_layer)
    }
}

/// Build the special Frobenius (spider) morphism with `m` inputs and `n` outputs of `wire_type`.
///
/// Base cases map to the six generators; larger arities decompose recursively
/// via binary tree of multiplications/comultiplications.
pub fn special_frobenius_morphism<
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
>(
    m: usize,
    n: usize,
    wire_type: Lambda,
) -> FrobeniusMorphism<Lambda, BlackBoxLabel> {
    match (m, n) {
        (2, 1) => FrobeniusOperation::Multiplication(wire_type).into(),
        (1, 2) => FrobeniusOperation::Comultiplication(wire_type).into(),
        (1, 0) => FrobeniusOperation::Counit(wire_type).into(),
        (0, 1) => FrobeniusOperation::Unit(wire_type).into(),
        (1, 1) => FrobeniusOperation::Identity(wire_type).into(),
        _ => {
            if m < n {
                let mut x = special_frobenius_morphism(n, m, wire_type);
                x.hflip(&identity);
                x
            } else if n != 1 {
                let mut x = special_frobenius_morphism(m, 1, wire_type);
                let y = special_frobenius_morphism(1, n, wire_type);
                let _ = x.compose(y);
                x
            } else if m.is_multiple_of(2) {
                let mut answer = special_frobenius_morphism(m / 2, 1, wire_type);
                answer.monoidal(answer.clone());
                let _ = answer.compose(FrobeniusOperation::Multiplication(wire_type).into());
                answer
            } else {
                let mut answer = special_frobenius_morphism(m - 1, 1, wire_type);
                answer.monoidal(FrobeniusOperation::Identity(wire_type).into());
                let _ = answer.compose(FrobeniusOperation::Multiplication(wire_type).into());
                answer
            }
        }
    }
}

/// Build a `FrobeniusMorphism` from an epi-mono `Decomposition` of a finite set map.
///
/// The decomposition is realized as: permutation, then surjection (spider merges),
/// then injection (identities interleaved with units).
///
/// # Errors
///
/// - Decomposition is incompatible with source/target types.
///
/// # Panics
///
/// - Panics if all types in a homogeneous block are not equal (internal invariant).
#[allow(clippy::needless_pass_by_value)]
pub fn from_decomposition<Lambda, BlackBoxLabel>(
    v: Decomposition,
    source_types: &[Lambda],
    target_types: &[Lambda],
) -> Result<FrobeniusMorphism<Lambda, BlackBoxLabel>, CatgraphError>
where
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
{
    let (perm_part, surj_part, inj_part) = v.get_parts();
    let mut answer = FrobeniusMorphism::from_permutation(perm_part.clone(), source_types, true)?;

    let mut surj_part_frob = FrobeniusMorphism::<Lambda, BlackBoxLabel>::new();
    let mut after_perm_number = 0;
    #[allow(clippy::unused_enumerate_index)]
    for c in &surj_part.preimage_cardinalities() {
        let after_perm_types = &answer.codomain()[after_perm_number..after_perm_number + c];
        assert!(after_perm_types.iter().all(|l| *l == after_perm_types[0]));
        let cur_part = special_frobenius_morphism::<_, BlackBoxLabel>(*c, 1, after_perm_types[0]);
        surj_part_frob.monoidal(cur_part);
        after_perm_number += c;
    }

    let mut inj_part_frob = FrobeniusMorphism::<Lambda, BlackBoxLabel>::new();
    let mut target_number = 0;
    for (n, c) in inj_part.iden_unit_counts().iter().enumerate() {
        if n % 2 == 0 {
            let cur_iden_type = target_types[target_number..target_number + c].to_vec();
            inj_part_frob.monoidal(FrobeniusMorphism::identity(&cur_iden_type));
            target_number += c;
        } else {
            for idx in 0..*c {
                inj_part_frob
                    .monoidal(FrobeniusOperation::Unit(target_types[target_number + idx]).into());
            }
            target_number += c;
        }
    }

    assert!(
        answer.compose(surj_part_frob).is_ok(),
        "The provided source and target types did not line up for the given decomposed finite set map"
    );
    assert!(
        answer.compose(inj_part_frob).is_ok(),
        "The provided source and target types did not line up for the given decomposed finite set map"
    );
    Ok(answer)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::category::ComposableMutating;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn rand_spiders() {
        use rand::{distr::Uniform, prelude::Distribution};
        let between = Uniform::try_from(0..5).unwrap();
        let mut rng = StdRng::seed_from_u64(3001);
        for _ in 0..10 {
            let m = between.sample(&mut rng);
            let n = between.sample(&mut rng);
            let rand_spider: FrobeniusMorphism<(), ()> = special_frobenius_morphism(m, n, ());
            let exp_source_type = vec![(); m];
            let exp_target_type = vec![(); n];
            assert_eq!(exp_source_type, rand_spider.domain());
            assert_eq!(exp_target_type, rand_spider.codomain());
        }
        let between = Uniform::try_from(128..255).unwrap();
        let mut rng = StdRng::seed_from_u64(3002);
        for _ in 0..5 {
            let m = between.sample(&mut rng);
            let n = between.sample(&mut rng);
            let rand_spider: FrobeniusMorphism<(), ()> = special_frobenius_morphism(m, n, ());
            let exp_source_type = vec![(); m];
            let exp_target_type = vec![(); n];
            assert_eq!(exp_source_type, rand_spider.domain());
            assert_eq!(exp_target_type, rand_spider.codomain());
            assert!(
                rand_spider.depth() <= 4 * 8,
                "Depth of {} to {} was {} instead of {}",
                m,
                n,
                rand_spider.depth(),
                4 * 8
            );
        }
    }

    #[test]
    fn basic_spiders() {
        let counit_spider: FrobeniusMorphism<(), ()> = special_frobenius_morphism(1, 0, ());
        let exp_counit_spider: FrobeniusMorphism<_, _> = FrobeniusOperation::Counit(()).into();
        assert!(exp_counit_spider == counit_spider);
        assert_eq!(counit_spider.depth(), 1);

        let comul_spider: FrobeniusMorphism<(), ()> = special_frobenius_morphism(1, 2, ());
        let exp_comul_spider: FrobeniusMorphism<_, _> =
            FrobeniusOperation::Comultiplication(()).into();
        assert!(exp_comul_spider == comul_spider);
        assert_eq!(comul_spider.depth(), 1);

        let mul_spider: FrobeniusMorphism<(), ()> = special_frobenius_morphism(2, 1, ());
        let exp_mul_spider: FrobeniusMorphism<_, _> = FrobeniusOperation::Multiplication(()).into();
        assert!(exp_mul_spider == mul_spider);
        assert_eq!(mul_spider.depth(), 1);

        let unit_spider: FrobeniusMorphism<(), ()> = special_frobenius_morphism(0, 1, ());
        let exp_unit_spider: FrobeniusMorphism<_, _> = FrobeniusOperation::Unit(()).into();
        assert!(exp_unit_spider == unit_spider);
        assert_eq!(unit_spider.depth(), 1);

        let id_spider: FrobeniusMorphism<(), ()> = special_frobenius_morphism(1, 1, ());
        let exp_id_spider: FrobeniusMorphism<_, _> = FrobeniusOperation::Identity(()).into();
        assert!(exp_id_spider == id_spider);
        assert_eq!(id_spider.depth(), 1);
    }

    #[allow(clippy::items_after_statements)]
    #[test]
    fn basic_typed_spiders() {
        let counit_spider: FrobeniusMorphism<bool, ()> = special_frobenius_morphism(1, 0, true);
        let exp_counit_spider: FrobeniusMorphism<_, _> = FrobeniusOperation::Counit(true).into();
        assert!(exp_counit_spider == counit_spider);

        let comul_spider: FrobeniusMorphism<bool, ()> = special_frobenius_morphism(1, 2, false);
        let exp_comul_spider: FrobeniusMorphism<_, _> =
            FrobeniusOperation::Comultiplication(false).into();
        assert!(exp_comul_spider == comul_spider);

        #[derive(PartialEq, Eq, Clone, Copy, Debug)]
        enum Color {
            Red,
            Green,
            Blue,
        }
        let mul_spider: FrobeniusMorphism<Color, ()> = special_frobenius_morphism(2, 1, Color::Red);
        let exp_mul_spider: FrobeniusMorphism<_, _> =
            FrobeniusOperation::Multiplication(Color::Red).into();
        assert!(exp_mul_spider == mul_spider);
        let exp_mul_spider: FrobeniusMorphism<_, _> =
            FrobeniusOperation::Multiplication(Color::Green).into();
        assert!(exp_mul_spider != mul_spider);

        let unit_spider: FrobeniusMorphism<Color, ()> =
            special_frobenius_morphism(0, 1, Color::Blue);
        let exp_unit_spider: FrobeniusMorphism<_, _> = FrobeniusOperation::Unit(Color::Blue).into();
        assert!(exp_unit_spider == unit_spider);

        let id_spider: FrobeniusMorphism<Color, ()> =
            special_frobenius_morphism(1, 1, Color::Green);
        let exp_id_spider: FrobeniusMorphism<_, _> =
            FrobeniusOperation::Identity(Color::Green).into();
        assert!(exp_id_spider == id_spider);
        let exp_id_spider: FrobeniusMorphism<_, _> =
            FrobeniusOperation::Identity(Color::Blue).into();
        assert!(exp_id_spider != id_spider);

        let zero_zero_spider: FrobeniusMorphism<Color, ()> =
            special_frobenius_morphism(0, 0, Color::Green);
        let mut exp_zero_zero_spider: FrobeniusMorphism<_, _> =
            FrobeniusOperation::Unit(Color::Green).into();
        let composition_worked =
            exp_zero_zero_spider.compose(FrobeniusOperation::Counit(Color::Green).into());
        #[allow(clippy::assertions_on_constants)]
        if composition_worked.is_ok() {
            assert!(exp_zero_zero_spider == zero_zero_spider);
        } else {
            assert!(false, "Unit and counit do compose");
        }
    }

    #[test]
    fn permutation_automatic() {
        use crate::{
            monoidal::SymmetricMonoidalMorphism,
            utils::{in_place_permute, rand_perm},
        };
        use rand::{distr::Uniform, prelude::Distribution};
        let n_max = 10;
        let between = Uniform::<usize>::try_from(2..n_max).unwrap();
        let mut rng = StdRng::seed_from_u64(3003);
        let my_n = between.sample(&mut rng);
        let types_as_on_source = true;
        let domain_types = (0..my_n).map(|idx| idx + 100).collect::<Vec<usize>>();
        let p1 = rand_perm(my_n, my_n * 2, &mut rng);
        let frob_p1 = FrobeniusMorphism::<usize, ()>::from_permutation(
            p1.clone(),
            &domain_types,
            types_as_on_source,
        )
        .unwrap();
        let mut frob_prod = frob_p1.clone();
        assert_eq!(frob_prod.domain(), domain_types);
        let mut types_after_this_layer = domain_types.clone();
        in_place_permute(&mut types_after_this_layer, &p1.inv());
        assert_eq!(frob_prod.codomain(), types_after_this_layer);
        let p2 = rand_perm(my_n, my_n * 2, &mut rng);
        let frob_p2 = FrobeniusMorphism::from_permutation(
            p2.clone(),
            &frob_p1.codomain(),
            types_as_on_source,
        )
        .unwrap();
        frob_prod.compose(frob_p2).unwrap();
        in_place_permute(&mut types_after_this_layer, &p2.inv());
        assert_eq!(frob_prod.domain(), domain_types);
        assert_eq!(frob_prod.codomain(), types_after_this_layer);
        // Exercise the types_as_on_source = false branch.
        // from_permutation(p3, T, false) produces domain = p3(T), codomain = T.
        // We build such a morphism independently and verify its invariants, then
        // use its own domain to chain onto frob_prod through a tailored bridge.
        let p3 = rand_perm(my_n, my_n * 2, &mut rng);
        let codomain_of_prod = frob_prod.codomain().clone();
        let frob_p3_targeting_t = FrobeniusMorphism::<usize, ()>::from_permutation(
            p3.clone(),
            &codomain_of_prod,
            false, // types_as_on_source = false → codomain = codomain_of_prod
        )
        .unwrap();
        let mut expected_p3_domain = codomain_of_prod.clone();
        in_place_permute(&mut expected_p3_domain, &p3);
        assert_eq!(frob_p3_targeting_t.domain(), expected_p3_domain);
        assert_eq!(frob_p3_targeting_t.codomain(), codomain_of_prod);
        // To compose frob_prod with a permutation of codomain_of_prod, use the
        // types_as_on_source = true variant (domain = codomain_of_prod, codomain = p3.inv(...)).
        let frob_p3 =
            FrobeniusMorphism::<usize, ()>::from_permutation(p3.clone(), &codomain_of_prod, true)
                .unwrap();
        assert_eq!(frob_p3.domain(), codomain_of_prod);
        frob_prod.compose(frob_p3).unwrap();
        let mut expected_codomain = codomain_of_prod.clone();
        in_place_permute(&mut expected_codomain, &p3.inv());
        assert_eq!(frob_prod.domain(), domain_types);
        assert_eq!(frob_prod.codomain(), expected_codomain);
        #[allow(clippy::match_same_arms, clippy::match_like_matches_macro)]
        let all_swaps = frob_prod.layers.iter().all(|layer| {
            layer.blocks.iter().all(|block| match block.op {
                FrobeniusOperation::SymmetricBraiding(_, _) => true,
                FrobeniusOperation::Identity(_) => true,
                _ => false,
            })
        });
        assert!(all_swaps);
    }

    #[test]
    fn from_permutation_invariants_probe() {
        use crate::{
            monoidal::SymmetricMonoidalMorphism,
            utils::{in_place_permute, rand_perm},
        };
        let mut rng = StdRng::seed_from_u64(3003);
        for n in 2..=10 {
            let types: Vec<usize> = (100..100 + n).collect();
            for trial in 0..30 {
                let p = rand_perm(n, n * 2, &mut rng);
                let fp_dom: FrobeniusMorphism<usize, ()> =
                    FrobeniusMorphism::from_permutation(p.clone(), &types, true).unwrap();
                let mut expected_cod = types.clone();
                in_place_permute(&mut expected_cod, &p.inv());
                assert_eq!(fp_dom.domain(), types, "n={n} trial {trial} TRUE dom");
                assert_eq!(
                    fp_dom.codomain(),
                    expected_cod,
                    "n={n} trial {trial} TRUE cod"
                );

                let fp_cod: FrobeniusMorphism<usize, ()> =
                    FrobeniusMorphism::from_permutation(p.clone(), &types, false).unwrap();
                let mut expected_dom = types.clone();
                in_place_permute(&mut expected_dom, &p);
                assert_eq!(fp_cod.codomain(), types, "n={n} trial {trial} FALSE cod");
                assert_eq!(
                    fp_cod.domain(),
                    expected_dom,
                    "n={n} trial {trial} FALSE dom"
                );
            }
        }
    }

    #[test]
    fn from_permutation_compose_probe() {
        use crate::{
            category::ComposableMutating,
            monoidal::SymmetricMonoidalMorphism,
            utils::{in_place_permute, rand_perm},
        };
        let mut rng = StdRng::seed_from_u64(3003);
        for n in 2..=10 {
            for trial in 0..30 {
                let types: Vec<usize> = (100..100 + n).collect();
                let p1 = rand_perm(n, n * 2, &mut rng);
                let p2 = rand_perm(n, n * 2, &mut rng);
                let mut f1: FrobeniusMorphism<usize, ()> =
                    FrobeniusMorphism::from_permutation(p1.clone(), &types, true).unwrap();
                let mut mid = types.clone();
                in_place_permute(&mut mid, &p1.inv());
                let f2: FrobeniusMorphism<usize, ()> =
                    FrobeniusMorphism::from_permutation(p2.clone(), &mid, true).unwrap();
                f1.compose(f2)
                    .unwrap_or_else(|e| panic!("n={n} trial {trial} compose: {e:?}"));
                let mut end = mid.clone();
                in_place_permute(&mut end, &p2.inv());
                assert_eq!(f1.domain(), types, "n={n} trial {trial} end dom");
                assert_eq!(f1.codomain(), end, "n={n} trial {trial} end cod");
            }
        }
    }

    #[test]
    fn decomposition_automatic() {
        use crate::finset::Decomposition;
        use rand::{distr::Uniform, prelude::Distribution};
        let in_max = 20;
        let out_max = 20;
        let mut rng = StdRng::seed_from_u64(3004);
        let between = Uniform::<usize>::try_from(2..in_max).unwrap();
        let in_ = between.sample(&mut rng);
        let between = Uniform::<usize>::try_from(2..out_max).unwrap();
        let out_ = between.sample(&mut rng);
        let cur_test = (0..in_)
            .map(|_| {
                Uniform::<usize>::try_from(0..out_)
                    .unwrap()
                    .sample(&mut rng)
            })
            .collect::<Vec<usize>>();
        let domain_types = (0..in_)
            .map(|idx| cur_test[idx] + 100)
            .collect::<Vec<usize>>();
        let mut codomain_types = (0..out_).map(|idx| idx + 40).collect::<Vec<usize>>();
        for (idx, idx_goes) in cur_test.iter().enumerate() {
            codomain_types[*idx_goes] = domain_types[idx];
        }
        let cur_res = Decomposition::try_from((cur_test.clone(), 0));
        #[allow(clippy::assertions_on_constants)]
        if let Ok(cur_decomp) = cur_res {
            let _x: FrobeniusMorphism<_, ()> =
                from_decomposition(cur_decomp, &domain_types, &codomain_types).unwrap();
        } else {
            assert!(false, "All maps of finite sets decompose");
        }
    }

    /// Algebraic verification of `FrobeniusMorphism::permute_side`.
    ///
    /// Reference: `Cospan::permute_side` uses `in_place_permute` directly
    /// on the leg arrays, which is known correct (tested in `wiring_diagram`).
    /// Here we verify that `FrobeniusMorphism` matches the same contract:
    ///   - `permute_side(p, true)` → codomain becomes `p.permute(old_codomain)`
    ///   - `permute_side(p, false)` → domain becomes `p.permute(old_domain)`
    #[test]
    fn frobenius_permute_side_codomain_with_swap() {
        use crate::category::HasIdentity;
        use crate::monoidal::SymmetricMonoidalMorphism;
        use permutations::Permutation;

        let types = vec!['a', 'b'];
        let mut morph: FrobeniusMorphism<char, ()> = FrobeniusMorphism::identity(&types);
        let swap = Permutation::transposition(2, 0, 1);
        morph.permute_side(&swap, true);
        assert_eq!(morph.domain(), vec!['a', 'b']);
        assert_eq!(morph.codomain(), swap.permute(&types));
    }

    #[test]
    fn frobenius_permute_side_domain_with_swap() {
        use crate::category::HasIdentity;
        use crate::monoidal::SymmetricMonoidalMorphism;
        use permutations::Permutation;

        let types = vec!['a', 'b'];
        let mut morph: FrobeniusMorphism<char, ()> = FrobeniusMorphism::identity(&types);
        let swap = Permutation::transposition(2, 0, 1);
        morph.permute_side(&swap, false);
        assert_eq!(morph.domain(), swap.permute(&types));
        assert_eq!(morph.codomain(), vec!['a', 'b']);
    }

    /// Non-involution (3-cycle) catches p vs `p.inv()` confusion.
    #[test]
    fn frobenius_permute_side_codomain_rotation() {
        use crate::category::HasIdentity;
        use crate::monoidal::SymmetricMonoidalMorphism;
        use permutations::Permutation;

        let types = vec!['a', 'b', 'c'];
        let rotation = Permutation::rotation_left(3, 1); // [1, 2, 0]
        let mut morph: FrobeniusMorphism<char, ()> = FrobeniusMorphism::identity(&types);
        morph.permute_side(&rotation, true);
        assert_eq!(morph.domain(), vec!['a', 'b', 'c']);
        // p.permute([a,b,c])[i] = [a,b,c][p(i)] → [b, c, a]
        assert_eq!(morph.codomain(), rotation.permute(&types));
    }

    #[test]
    fn frobenius_permute_side_domain_rotation() {
        use crate::category::HasIdentity;
        use crate::monoidal::SymmetricMonoidalMorphism;
        use permutations::Permutation;

        let types = vec!['a', 'b', 'c'];
        let rotation = Permutation::rotation_left(3, 1);
        let mut morph: FrobeniusMorphism<char, ()> = FrobeniusMorphism::identity(&types);
        morph.permute_side(&rotation, false);
        assert_eq!(morph.domain(), rotation.permute(&types));
        assert_eq!(morph.codomain(), vec!['a', 'b', 'c']);
    }

    /// Verify with random non-identity permutations on various sizes.
    #[test]
    fn frobenius_permute_side_random() {
        use crate::category::HasIdentity;
        use crate::monoidal::SymmetricMonoidalMorphism;
        use crate::utils::rand_perm;
        use rand::{distr::Uniform, prelude::Distribution};

        let mut rng = StdRng::seed_from_u64(3005);
        for _ in 0..20 {
            let n = Uniform::<usize>::try_from(2..8).unwrap().sample(&mut rng);
            let types: Vec<usize> = (0..n).map(|i| i + 100).collect();
            let p = rand_perm(n, n * 2, &mut rng);

            // codomain case
            let mut morph_cod: FrobeniusMorphism<usize, ()> = FrobeniusMorphism::identity(&types);
            morph_cod.permute_side(&p, true);
            assert_eq!(
                morph_cod.domain(),
                types,
                "domain unchanged after cod permute"
            );
            assert_eq!(
                morph_cod.codomain(),
                p.permute(&types),
                "codomain = p.permute(types) for n={n}, p={p:?}"
            );

            // domain case
            let mut morph_dom: FrobeniusMorphism<usize, ()> = FrobeniusMorphism::identity(&types);
            morph_dom.permute_side(&p, false);
            assert_eq!(
                morph_dom.domain(),
                p.permute(&types),
                "domain = p.permute(types) for n={n}, p={p:?}"
            );
            assert_eq!(
                morph_dom.codomain(),
                types,
                "codomain unchanged after dom permute"
            );
        }
    }

    /// Composing two morphisms around a permutation: verify type-correctness.
    /// f : [a,b,c] → [a,b,c], then `permute_side(p`, true) gives f' : [a,b,c] → p.permute([a,b,c]).
    /// g = `from_permutation(p.inv()`, p.permute(types), true) should compose with f'.
    #[test]
    fn frobenius_permute_side_compose_roundtrip() {
        use crate::category::HasIdentity;
        use crate::monoidal::SymmetricMonoidalMorphism;
        use permutations::Permutation;

        let types = vec![10, 20, 30, 40];
        let p = Permutation::rotation_left(4, 1);

        let mut morph: FrobeniusMorphism<i32, ()> = FrobeniusMorphism::identity(&types);
        morph.permute_side(&p, true);
        let permuted_cod = morph.codomain();
        assert_eq!(permuted_cod, p.permute(&types));

        // Build the inverse permutation morphism to compose back
        let inv_morph: FrobeniusMorphism<i32, ()> =
            FrobeniusMorphism::from_permutation(p.clone(), &permuted_cod, true).unwrap();
        assert_eq!(inv_morph.domain(), permuted_cod);
        let compose_result = morph.compose(inv_morph);
        assert!(
            compose_result.is_ok(),
            "composition after permute_side should type-check"
        );
    }

    /// Verify that `permute_side` on a non-identity morphism (spider) works correctly.
    #[test]
    fn frobenius_permute_side_on_spider() {
        use crate::monoidal::SymmetricMonoidalMorphism;
        use permutations::Permutation;

        // Spider: 3 inputs → 2 outputs, all type ()
        let mut spider: FrobeniusMorphism<(), ()> = special_frobenius_morphism(3, 2, ());
        let original_domain = spider.domain();
        let original_codomain = spider.codomain();
        assert_eq!(original_domain, vec![(); 3]);
        assert_eq!(original_codomain, vec![(); 2]);

        let p_dom = Permutation::rotation_left(3, 1);
        spider.permute_side(&p_dom, false);
        // For uniform types, permutation doesn't change the types themselves
        assert_eq!(spider.domain(), vec![(); 3]);
        assert_eq!(spider.codomain(), vec![(); 2]);

        let p_cod = Permutation::transposition(2, 0, 1);
        spider.permute_side(&p_cod, true);
        assert_eq!(spider.domain(), vec![(); 3]);
        assert_eq!(spider.codomain(), vec![(); 2]);
    }

    // ── two_layer_simplify tests ──

    #[test]
    fn test_identity_layers_simplify() {
        // Both layers are identity → (true, true, false)
        let mut layer1: FrobeniusLayer<char, ()> = FrobeniusLayer::new();
        layer1.append_block(FrobeniusOperation::Identity('a'));
        layer1.append_block(FrobeniusOperation::Identity('b'));

        let mut layer2: FrobeniusLayer<char, ()> = FrobeniusLayer::new();
        layer2.append_block(FrobeniusOperation::Identity('a'));
        layer2.append_block(FrobeniusOperation::Identity('b'));

        let result = layer1.two_layer_simplify(&mut layer2);
        assert_eq!(result, (true, true, false));

        // Only self is identity → (true, false, false)
        let mut id_layer: FrobeniusLayer<char, ()> = FrobeniusLayer::new();
        id_layer.append_block(FrobeniusOperation::Identity('x'));

        let mut non_id_layer: FrobeniusLayer<char, ()> = FrobeniusLayer::new();
        non_id_layer.append_block(FrobeniusOperation::Comultiplication('x'));

        let result = id_layer.two_layer_simplify(&mut non_id_layer);
        assert_eq!(result, (true, false, false));

        // Only next is identity → (false, true, false)
        let mut non_id_layer2: FrobeniusLayer<char, ()> = FrobeniusLayer::new();
        non_id_layer2.append_block(FrobeniusOperation::Multiplication('y'));

        let mut id_layer2: FrobeniusLayer<char, ()> = FrobeniusLayer::new();
        id_layer2.append_block(FrobeniusOperation::Identity('y'));

        let result = non_id_layer2.two_layer_simplify(&mut id_layer2);
        assert_eq!(result, (false, true, false));
    }

    #[test]
    fn test_braiding_self_inverse() {
        // σ(a,b) then σ(b,a) should cancel to identities in both layers.
        //
        // Original interfaces: layer1 is (a,b)→(b,a), layer2 is (b,a)→(a,b).
        // The external type interface of the composed pair is (a,b)→(a,b).
        // Both simplified layers must realise this, so each becomes identity
        // on [a,b] — in particular layer2.right_type must be preserved as [a,b].
        let mut layer1: FrobeniusLayer<char, ()> = FrobeniusLayer::new();
        layer1.append_block(FrobeniusOperation::SymmetricBraiding('a', 'b'));

        let mut layer2: FrobeniusLayer<char, ()> = FrobeniusLayer::new();
        layer2.append_block(FrobeniusOperation::SymmetricBraiding('b', 'a'));

        let (self_id, next_id, mutations) = layer1.two_layer_simplify(&mut layer2);

        assert!(mutations, "braiding cancellation should mutate");
        assert!(self_id, "self should become identity after braiding cancel");
        assert!(next_id, "next should become identity after braiding cancel");

        assert!(layer1.is_identity());
        assert!(layer2.is_identity());

        // Outer interfaces preserved: layer1.left and layer2.right both [a,b].
        // Middle interface (layer1.right, layer2.left) is [a,b], coupled.
        assert_eq!(layer1.left_type, vec!['a', 'b']);
        assert_eq!(layer1.right_type, vec!['a', 'b']);
        assert_eq!(layer2.left_type, vec!['a', 'b']);
        assert_eq!(layer2.right_type, vec!['a', 'b']);
    }

    #[test]
    fn test_braiding_no_cancel_different_types() {
        // σ(a,b) then σ(a,b) should NOT cancel (not inverse)
        let mut layer1: FrobeniusLayer<char, ()> = FrobeniusLayer::new();
        layer1.append_block(FrobeniusOperation::SymmetricBraiding('a', 'b'));

        let mut layer2: FrobeniusLayer<char, ()> = FrobeniusLayer::new();
        layer2.append_block(FrobeniusOperation::SymmetricBraiding('a', 'b'));

        let (_, _, mutations) = layer1.two_layer_simplify(&mut layer2);
        assert!(!mutations, "non-inverse braidings should not cancel");
    }

    #[test]
    fn test_unit_counit_cancel() {
        // Unit(z) then Counit(z) → both removed (scalar loop)
        let mut layer1: FrobeniusLayer<char, ()> = FrobeniusLayer::new();
        layer1.append_block(FrobeniusOperation::Unit('z'));

        let mut layer2: FrobeniusLayer<char, ()> = FrobeniusLayer::new();
        layer2.append_block(FrobeniusOperation::Counit('z'));

        let (_, _, mutations) = layer1.two_layer_simplify(&mut layer2);
        assert!(mutations, "unit-counit should cancel");

        // Both layers should now be empty
        assert!(
            layer1.blocks.is_empty(),
            "self should have no blocks after unit-counit cancel"
        );
        assert!(
            layer2.blocks.is_empty(),
            "next should have no blocks after unit-counit cancel"
        );
        assert!(layer1.left_type.is_empty());
        assert!(layer1.right_type.is_empty());
        assert!(layer2.left_type.is_empty());
        assert!(layer2.right_type.is_empty());
    }

    #[test]
    fn test_unit_counit_no_cancel_different_labels() {
        // Unit('a') then Counit('b') → no cancellation (different labels)
        let mut layer1: FrobeniusLayer<char, ()> = FrobeniusLayer::new();
        layer1.append_block(FrobeniusOperation::Unit('a'));

        let mut layer2: FrobeniusLayer<char, ()> = FrobeniusLayer::new();
        layer2.append_block(FrobeniusOperation::Counit('b'));

        let (_, _, mutations) = layer1.two_layer_simplify(&mut layer2);
        assert!(!mutations, "unit(a) and counit(b) should not cancel");
    }

    #[test]
    fn test_spider_fusion() {
        // Spider(z, 1, 2) then Spider(z, 2, 1) → Spider(z, 1, 1) which is identity
        let mut layer1: FrobeniusLayer<char, ()> = FrobeniusLayer::new();
        layer1.append_block(FrobeniusOperation::Spider('z', 1, 2));

        let mut layer2: FrobeniusLayer<char, ()> = FrobeniusLayer::new();
        layer2.append_block(FrobeniusOperation::Spider('z', 2, 1));

        let (self_id, _, mutations) = layer1.two_layer_simplify(&mut layer2);
        assert!(mutations, "spider fusion should mutate");

        // self should now contain Spider(z, 1, 1) which is_identity
        assert!(self_id, "Spider(z,1,1) should be identity");
        assert!(
            layer1.blocks.len() == 1,
            "expected 1 block, got {}",
            layer1.blocks.len()
        );
        assert!(
            matches!(layer1.blocks[0].op, FrobeniusOperation::Spider('z', 1, 1)),
            "expected Spider(z,1,1)"
        );

        // next_layer should be empty (the second spider was consumed)
        assert!(layer2.blocks.is_empty());
    }

    #[test]
    fn test_spider_fusion_general() {
        // Spider(z, 3, 2) then Spider(z, 2, 4) → Spider(z, 3, 4)
        let mut layer1: FrobeniusLayer<char, ()> = FrobeniusLayer::new();
        layer1.append_block(FrobeniusOperation::Spider('z', 3, 2));

        let mut layer2: FrobeniusLayer<char, ()> = FrobeniusLayer::new();
        layer2.append_block(FrobeniusOperation::Spider('z', 2, 4));

        let (_, _, mutations) = layer1.two_layer_simplify(&mut layer2);
        assert!(mutations, "spider fusion should mutate");

        assert!(
            layer1.blocks.len() == 1,
            "expected 1 block, got {}",
            layer1.blocks.len()
        );
        assert!(
            matches!(layer1.blocks[0].op, FrobeniusOperation::Spider('z', 3, 4)),
            "expected Spider(z,3,4)"
        );
        assert!(layer2.blocks.is_empty());

        // Verify types
        assert_eq!(layer1.left_type, vec!['z'; 3]);
        assert_eq!(layer1.right_type, vec!['z'; 4]);
    }

    #[test]
    fn test_spider_no_fusion_different_labels() {
        // Spider('a', 1, 2) then Spider('b', 2, 1) → no fusion (different labels)
        let mut layer1: FrobeniusLayer<char, ()> = FrobeniusLayer::new();
        layer1.append_block(FrobeniusOperation::Spider('a', 1, 2));

        let mut layer2: FrobeniusLayer<char, ()> = FrobeniusLayer::new();
        layer2.append_block(FrobeniusOperation::Spider('b', 2, 1));

        let (_, _, mutations) = layer1.two_layer_simplify(&mut layer2);
        assert!(!mutations, "spiders with different labels should not fuse");
    }

    /// Integration test: composing σ(a,b) ; σ(b,a) through `FrobeniusMorphism::compose`
    /// should simplify to identity (reducing depth).
    #[test]
    fn test_braiding_cancel_via_compose() {
        let braiding1: FrobeniusMorphism<char, ()> =
            FrobeniusOperation::SymmetricBraiding('a', 'b').into();
        let braiding2: FrobeniusMorphism<char, ()> =
            FrobeniusOperation::SymmetricBraiding('b', 'a').into();

        let mut composed = braiding1;
        composed.compose(braiding2).unwrap();

        // After simplification, the composed morphism should be equivalent
        // to identity on ['a', 'b']
        assert_eq!(composed.domain(), vec!['a', 'b']);
        assert_eq!(composed.codomain(), vec!['a', 'b']);
        // The braiding layers should have cancelled, reducing depth
        assert!(
            composed.depth() <= 1,
            "braiding self-inverse should simplify depth to at most 1, got {}",
            composed.depth()
        );
    }

    /// Integration test: Unit then Counit through compose should produce
    /// an empty (scalar) morphism.
    #[test]
    fn test_unit_counit_cancel_via_compose() {
        let unit: FrobeniusMorphism<char, ()> = FrobeniusOperation::Unit('z').into();
        let counit: FrobeniusMorphism<char, ()> = FrobeniusOperation::Counit('z').into();

        let mut composed = unit;
        composed.compose(counit).unwrap();

        assert_eq!(composed.domain(), Vec::<char>::new());
        assert_eq!(composed.codomain(), Vec::<char>::new());
    }

    // ── hflip determinism / involution (rayon-site guard, #48) ──
    //
    // `FrobeniusLayer::hflip` fans block flips across rayon workers via
    // `par_iter_mut().with_min_len(PARALLEL_BLOCK_THRESHOLD)` (threshold 64,
    // `parallel` feature). Each block's flip touches only itself, so the
    // parallel fan-out must equal a plain sequential loop. These tests build
    // layers straddling the 64-block threshold and pin that equivalence against
    // an in-test sequential reference, plus the hflip involution: `hflip ∘ hflip
    // == id` whenever the block-label changer is involutive, and
    // `std::convert::identity` is. `FrobeniusLayer`/`FrobeniusMorphism` derive
    // only `Clone, PartialEq, Eq` (no `Debug`), so equality is asserted with
    // `assert!(a == b, ..)` rather than `assert_eq!`.

    /// A single layer of `n` heterogeneous blocks, so `hflip` actually
    /// transforms every block (Unit↔Counit, Mul↔Comul, braiding swap, spider
    /// arity swap) instead of mapping identities to themselves.
    fn wide_frobenius_layer(n: usize) -> FrobeniusLayer<i32, ()> {
        let mut layer = FrobeniusLayer::new();
        for i in 0..n {
            let z = i32::try_from(i).expect("fixture index fits in i32");
            match i % 4 {
                0 => layer.append_block(FrobeniusOperation::Unit(z)),
                1 => layer.append_block(FrobeniusOperation::Multiplication(z)),
                2 => layer.append_block(FrobeniusOperation::SymmetricBraiding(z, z + 1)),
                _ => layer.append_block(FrobeniusOperation::Spider(z, 2, 3)),
            }
        }
        layer
    }

    /// Sequential reference for `FrobeniusLayer::hflip`: flip each block in a
    /// plain loop and swap the layer's left/right type interfaces — the same
    /// per-block work as the method, without the rayon fan-out.
    fn hflip_layer_reference(layer: &FrobeniusLayer<i32, ()>) -> FrobeniusLayer<i32, ()> {
        let mut blocks = layer.blocks.clone();
        for block in &mut blocks {
            block.hflip(std::convert::identity);
        }
        FrobeniusLayer {
            blocks,
            left_type: layer.right_type.clone(),
            right_type: layer.left_type.clone(),
        }
    }

    #[test]
    fn frobenius_layer_hflip_matches_sequential_reference() {
        // Below threshold (64): the method runs its single-task / sequential arm.
        let small = wide_frobenius_layer(40);
        let mut small_flipped = small.clone();
        small_flipped.hflip(&std::convert::identity);
        assert!(
            small_flipped == hflip_layer_reference(&small),
            "hflip below the 64-block threshold must match the sequential reference"
        );

        // Above threshold (64): `par_iter_mut` fans out across workers.
        let large = wide_frobenius_layer(200);
        let mut large_flipped = large.clone();
        large_flipped.hflip(&std::convert::identity);
        assert!(
            large_flipped == hflip_layer_reference(&large),
            "parallel hflip above the 64-block threshold must match the sequential reference"
        );
    }

    #[test]
    fn frobenius_layer_hflip_is_involution() {
        for n in [40_usize, 200] {
            let original = wide_frobenius_layer(n);
            let mut twice = original.clone();
            twice.hflip(&std::convert::identity);
            twice.hflip(&std::convert::identity);
            assert!(
                twice == original,
                "hflip∘hflip must be the identity at n={n} blocks"
            );
        }
    }

    #[test]
    fn frobenius_layer_hflip_deterministic_across_runs() {
        let build = || {
            let mut layer = wide_frobenius_layer(200);
            layer.hflip(&std::convert::identity);
            layer
        };
        assert!(
            build() == build(),
            "parallel hflip must be run-to-run stable above the threshold"
        );
    }

    #[test]
    fn frobenius_morphism_hflip_involution_and_determinism() {
        // `special_frobenius_morphism(100, 1, _)` builds a deep, multi-layer
        // morphism whose wider layers cross the 64-block threshold; `hflip`
        // reverses the layer order and flips each layer.
        let base: FrobeniusMorphism<char, ()> = special_frobenius_morphism(100, 1, 'a');

        let mut twice = base.clone();
        twice.hflip(&std::convert::identity);
        twice.hflip(&std::convert::identity);
        assert!(twice == base, "morphism hflip∘hflip must be the identity");

        let mut once_a = base.clone();
        once_a.hflip(&std::convert::identity);
        let mut once_b: FrobeniusMorphism<char, ()> = special_frobenius_morphism(100, 1, 'a');
        once_b.hflip(&std::convert::identity);
        assert!(once_a == once_b, "morphism hflip must be run-to-run stable");

        // Sanity: a single flip of a non-self-dual (100→1) morphism is not a no-op.
        assert!(once_a != base);
    }
}
