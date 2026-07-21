//! Signal flow graph over a rig R (F&S 2018 Def 5.45).
//!
//! `SFG_R = Free(G_R)` with the 5 primitive generators from Eq 5.52:
//!
//! - `Copy : 1 → 2` (the Δ icon)
//! - `Discard : 1 → 0` (the ! icon)
//! - `Add : 2 → 1` (the μ icon)
//! - `Zero : 0 → 1` (the η icon)
//! - `Scalar(r) : 1 → 1` for `r ∈ R`
//!
//! Higher-arity `copy_n : 1 → n` and `discard_n : n → 0` are provided as
//! **derived** free functions (iterated composition / tensor), not primitive
//! generators. This keeps [`SfgSignature`] aligned with F&S `G_R` exactly and
//! keeps the Thm 5.60 equation set defined on primitives.
//!
//! ## Trait-bound story
//!
//! [`PropSignature`] requires `Clone + PartialEq + Eq + Hash +
//! Debug`. [`SfgGenerator<R>`] derives all five uniformly and therefore
//! requires `R: Rig + Eq + Hash + Debug`. All four shipped rigs
//! ([`crate::rig::BoolRig`], [`crate::rig::UnitInterval`],
//! [`crate::rig::Tropical`], [`crate::rig::F64Rig`]) satisfy these — the three
//! `f64`-wrapping rigs provide manual `Eq + Hash` impls via `to_bits()`
//! (bit-exact except `-0.0` normalizes to `0.0` to satisfy the `Eq`/`Hash`
//! contract; NaN caveats documented on the rig module).

use catgraph::errors::CatgraphError;

use crate::{
    prop::{Free, PropExpr, PropSignature},
    rig::Rig,
};

/// The 5 primitive `G_R` generators from F&S Def 5.45 / Eq 5.52.
///
/// Parameterised over the rig `R` so that `Scalar(r)` ranges over `R`-values.
///
/// The `Eq + Hash` bounds on `R` are required (via the `PropSignature`
/// `Eq + Hash` bounds) by the congruence-closure decision procedure. All four shipped
/// rig instances — [`crate::rig::BoolRig`], [`crate::rig::UnitInterval`],
/// [`crate::rig::Tropical`], [`crate::rig::F64Rig`] — satisfy both; the three
/// `f64`-wrapping rigs provide manual `Eq + Hash` via `to_bits()` (bit-exact
/// except `-0.0` normalizes to `0.0` to satisfy the `Eq`/`Hash` contract).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SfgGenerator<R: Rig + Eq + std::hash::Hash> {
    /// `Δ : 1 → 2` — duplicate a wire.
    Copy,
    /// `! : 1 → 0` — discard a wire.
    Discard,
    /// `μ : 2 → 1` — sum two wires.
    Add,
    /// `η : 0 → 1` — emit the additive identity.
    Zero,
    /// `r · (–) : 1 → 1` — scalar multiplication by `r ∈ R`.
    Scalar(R),
}

impl<R: Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static> PropSignature for SfgGenerator<R> {
    fn source(&self) -> usize {
        match self {
            SfgGenerator::Copy | SfgGenerator::Discard | SfgGenerator::Scalar(_) => 1,
            SfgGenerator::Add => 2,
            SfgGenerator::Zero => 0,
        }
    }

    fn target(&self) -> usize {
        match self {
            SfgGenerator::Copy => 2,
            SfgGenerator::Discard => 0,
            SfgGenerator::Add | SfgGenerator::Zero | SfgGenerator::Scalar(_) => 1,
        }
    }
}

/// The `G_R` signature: a zero-sized witness that `SfgGenerator<R>` is the
/// carrier of the prop `SFG_R = Free(G_R)`.
///
/// Primarily a documentation / type-level marker; actual prop operations go
/// through [`SignalFlowGraph`] and [`Free::<SfgGenerator<R>>`].
pub struct SfgSignature<R: Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static>(
    std::marker::PhantomData<R>,
);

/// A morphism `m → n` of `SFG_R` — an arity-tracked expression tree over
/// the 5 primitive generators plus identity / braid / composition / tensor.
///
/// Equality is the structural equality inherited from [`PropExpr`]; the
/// F&S Thm 5.60 quotient (matrix equivalence of signal-flow graphs) is
/// presentation-layer work.
#[derive(Clone, Debug)]
pub struct SignalFlowGraph<R: Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static>(
    PropExpr<SfgGenerator<R>>,
);

impl<R: Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static> SignalFlowGraph<R> {
    /// `Δ : 1 → 2` — the copy generator.
    #[must_use]
    pub fn copy() -> Self {
        Self(Free::<SfgGenerator<R>>::generator(SfgGenerator::Copy))
    }

    /// `! : 1 → 0` — the discard generator.
    #[must_use]
    pub fn discard() -> Self {
        Self(Free::<SfgGenerator<R>>::generator(SfgGenerator::Discard))
    }

    /// `μ : 2 → 1` — the add generator.
    #[must_use]
    pub fn add() -> Self {
        Self(Free::<SfgGenerator<R>>::generator(SfgGenerator::Add))
    }

    /// `η : 0 → 1` — the zero generator.
    #[must_use]
    pub fn zero() -> Self {
        Self(Free::<SfgGenerator<R>>::generator(SfgGenerator::Zero))
    }

    /// `r · (–) : 1 → 1` — scalar multiplication by `r ∈ R`.
    #[must_use]
    pub fn scalar(r: R) -> Self {
        Self(Free::<SfgGenerator<R>>::generator(SfgGenerator::Scalar(r)))
    }

    /// `id_n : n → n`.
    #[must_use]
    pub fn identity(n: usize) -> Self {
        Self(Free::<SfgGenerator<R>>::identity(n))
    }

    /// Standard 2-wire swap `σ_{1,1} : 2 → 2`.
    #[must_use]
    pub fn braid_1_1() -> Self {
        Self(Free::<SfgGenerator<R>>::braid(1, 1))
    }

    /// General symmetric braid `σ_{m,n} : m+n → m+n`.
    #[must_use]
    pub fn braid(m: usize, n: usize) -> Self {
        Self(Free::<SfgGenerator<R>>::braid(m, n))
    }

    /// Sequential composition `self ; other`.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::CompositionSizeMismatch`] if
    /// `self.codomain() != other.domain()`.
    pub fn compose(&self, other: &Self) -> Result<Self, CatgraphError> {
        Free::<SfgGenerator<R>>::compose(self.0.clone(), other.0.clone()).map(Self)
    }

    /// Parallel tensor `self ⊗ other`.
    #[must_use]
    pub fn tensor(&self, other: &Self) -> Self {
        Self(Free::<SfgGenerator<R>>::tensor(
            self.0.clone(),
            other.0.clone(),
        ))
    }

    /// Borrow the underlying [`PropExpr`].
    #[must_use]
    pub fn as_prop_expr(&self) -> &PropExpr<SfgGenerator<R>> {
        &self.0
    }

    /// Wrap a `PropExpr` into a `SignalFlowGraph`.
    ///
    /// This is the inverse of [`as_prop_expr`](Self::as_prop_expr). Intended
    /// for soundness tests and for plumbing equation pairs from a
    /// [`Presentation`](crate::prop::presentation::Presentation) back through
    /// functors like `sfg_to_mat` that consume `SignalFlowGraph`.
    ///
    /// No structural validation is performed — the caller is responsible for
    /// ensuring the expression is built from `SfgGenerator<R>` primitives.
    #[must_use]
    pub fn from_prop_expr(expr: PropExpr<SfgGenerator<R>>) -> Self {
        Self(expr)
    }

    /// Domain arity (number of input wires).
    #[must_use]
    pub fn domain(&self) -> usize {
        self.0.source()
    }

    /// Codomain arity (number of output wires).
    #[must_use]
    pub fn codomain(&self) -> usize {
        self.0.target()
    }
}

/// Iterated copy: `copy_n(0) = discard`, `copy_n(1) = id`,
/// `copy_n(n) = copy ; (id ⊗ copy_n(n-1))`.
///
/// # Errors
///
/// In principle this construction is arity-safe, but it returns
/// `Result<_, CatgraphError>` to match the composition signature and to
/// surface any bugs in the recursion.
pub fn copy_n<R: Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static>(
    n: usize,
) -> Result<SignalFlowGraph<R>, CatgraphError> {
    match n {
        0 => Ok(SignalFlowGraph::<R>::discard()),
        1 => Ok(SignalFlowGraph::<R>::identity(1)),
        _ => {
            let rest = copy_n::<R>(n - 1)?;
            let id1_tensor_rest = SignalFlowGraph::<R>::identity(1).tensor(&rest);
            SignalFlowGraph::<R>::copy().compose(&id1_tensor_rest)
        }
    }
}

/// Iterated discard: `discard_n(0) = id(0)`,
/// `discard_n(n) = discard ⊗ discard_n(n-1)`.
#[must_use]
pub fn discard_n<R: Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static>(
    n: usize,
) -> SignalFlowGraph<R> {
    if n == 0 {
        SignalFlowGraph::<R>::identity(0)
    } else {
        let head = SignalFlowGraph::<R>::discard();
        let tail = discard_n::<R>(n - 1);
        head.tensor(&tail)
    }
}

/// Iterated add (dual of [`copy_n`]): `add_n(0) = zero`, `add_n(1) = id`,
/// `add_n(m) = (id ⊗ add_n(m-1)) ; add`.
///
/// A morphism `m → 1` summing all `m` inputs; `S(add_n(m))` is the all-ones
/// `m × 1` matrix (the transpose of `copy_n`'s `1 × n` all-ones matrix).
///
/// # Errors
///
/// In principle this construction is arity-safe, but it returns
/// `Result<_, CatgraphError>` to match the composition signature and to
/// surface any bugs in the recursion.
pub fn add_n<R: Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static>(
    m: usize,
) -> Result<SignalFlowGraph<R>, CatgraphError> {
    match m {
        0 => Ok(SignalFlowGraph::<R>::zero()),
        1 => Ok(SignalFlowGraph::<R>::identity(1)),
        _ => {
            let rest = add_n::<R>(m - 1)?;
            let id1_tensor_rest = SignalFlowGraph::<R>::identity(1).tensor(&rest);
            id1_tensor_rest.compose(&SignalFlowGraph::<R>::add())
        }
    }
}

/// Iterated zero (dual of [`discard_n`]): `zero_n(0) = id(0)`,
/// `zero_n(n) = zero ⊗ zero_n(n-1)`.
///
/// A morphism `0 → n` emitting the additive identity on each of `n` output
/// wires; `S(zero_n(n))` is the empty `0 × n` matrix.
#[must_use]
pub fn zero_n<R: Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static>(
    n: usize,
) -> SignalFlowGraph<R> {
    if n == 0 {
        SignalFlowGraph::<R>::identity(0)
    } else {
        let head = SignalFlowGraph::<R>::zero();
        let tail = zero_n::<R>(n - 1);
        head.tensor(&tail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rig::{F64Rig, UnitInterval};

    #[test]
    fn copy_arity_1_to_2() {
        let c = SignalFlowGraph::<F64Rig>::copy();
        assert_eq!(c.domain(), 1);
        assert_eq!(c.codomain(), 2);
    }

    #[test]
    fn discard_arity_1_to_0() {
        let d = SignalFlowGraph::<F64Rig>::discard();
        assert_eq!(d.domain(), 1);
        assert_eq!(d.codomain(), 0);
    }

    #[test]
    fn add_arity_2_to_1() {
        let a = SignalFlowGraph::<F64Rig>::add();
        assert_eq!(a.domain(), 2);
        assert_eq!(a.codomain(), 1);
    }

    #[test]
    fn zero_arity_0_to_1() {
        let z = SignalFlowGraph::<F64Rig>::zero();
        assert_eq!(z.domain(), 0);
        assert_eq!(z.codomain(), 1);
    }

    #[test]
    fn scalar_arity_1_to_1() {
        let s = SignalFlowGraph::<F64Rig>::scalar(F64Rig(2.5));
        assert_eq!(s.domain(), 1);
        assert_eq!(s.codomain(), 1);
    }

    #[test]
    fn scalar_accepts_unit_interval() {
        let p = UnitInterval::new(0.5).unwrap();
        let s = SignalFlowGraph::<UnitInterval>::scalar(p);
        assert_eq!(s.domain(), 1);
        assert_eq!(s.codomain(), 1);
    }

    #[test]
    fn compose_copy_add_is_1_to_1() {
        // copy (1→2) ; add (2→1) is 1→1.
        let composed = SignalFlowGraph::<F64Rig>::copy()
            .compose(&SignalFlowGraph::<F64Rig>::add())
            .unwrap();
        assert_eq!(composed.domain(), 1);
        assert_eq!(composed.codomain(), 1);
    }

    #[test]
    fn copy_n_3_codomain_is_3() {
        let c = copy_n::<F64Rig>(3).unwrap();
        assert_eq!(c.domain(), 1);
        assert_eq!(c.codomain(), 3);
    }

    #[test]
    fn discard_n_4_domain_is_4() {
        let d = discard_n::<F64Rig>(4);
        assert_eq!(d.domain(), 4);
        assert_eq!(d.codomain(), 0);
    }

    #[test]
    fn copy_n_zero_is_discard() {
        let c = copy_n::<F64Rig>(0).unwrap();
        // copy_n(0) returns discard, which is 1 → 0.
        assert_eq!(c.domain(), 1);
        assert_eq!(c.codomain(), 0);
    }

    #[test]
    fn copy_n_one_is_identity() {
        let c = copy_n::<F64Rig>(1).unwrap();
        // copy_n(1) returns identity(1), which is 1 → 1.
        assert_eq!(c.domain(), 1);
        assert_eq!(c.codomain(), 1);
    }

    #[test]
    fn discard_n_zero_is_identity() {
        let d = discard_n::<F64Rig>(0);
        // discard_n(0) returns identity(0), which is 0 → 0.
        assert_eq!(d.domain(), 0);
        assert_eq!(d.codomain(), 0);
    }

    #[test]
    fn discard_n_one_is_discard() {
        let d = discard_n::<F64Rig>(1);
        // discard_n(1) = discard ⊗ identity(0) = discard (via SMC quotient),
        // structurally this is just `discard`. Arity 1 → 0.
        assert_eq!(d.domain(), 1);
        assert_eq!(d.codomain(), 0);
    }

    #[test]
    fn add_n_3_domain_is_3() {
        let a = add_n::<F64Rig>(3).unwrap();
        assert_eq!(a.domain(), 3);
        assert_eq!(a.codomain(), 1);
    }

    #[test]
    fn add_n_zero_is_zero() {
        let a = add_n::<F64Rig>(0).unwrap();
        // add_n(0) returns zero, which is 0 → 1.
        assert_eq!(a.domain(), 0);
        assert_eq!(a.codomain(), 1);
    }

    #[test]
    fn add_n_one_is_identity() {
        let a = add_n::<F64Rig>(1).unwrap();
        // add_n(1) returns identity(1), which is 1 → 1.
        assert_eq!(a.domain(), 1);
        assert_eq!(a.codomain(), 1);
    }

    #[test]
    fn zero_n_4_codomain_is_4() {
        let z = zero_n::<F64Rig>(4);
        assert_eq!(z.domain(), 0);
        assert_eq!(z.codomain(), 4);
    }

    #[test]
    fn zero_n_zero_is_identity() {
        let z = zero_n::<F64Rig>(0);
        assert_eq!(z.domain(), 0);
        assert_eq!(z.codomain(), 0);
    }
}
