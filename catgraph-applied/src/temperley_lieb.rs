//! Temperley-Lieb and Brauer algebra morphisms via perfect matchings.
//!
//! A [`BrauerMorphism<T>`] is a formal linear combination of Brauer diagrams —
//! perfect matchings on `source + target` points — with coefficients in a ring `T`.
//! Composition multiplies diagrams by stacking them vertically and connecting
//! matched points through a `petgraph` connectivity check, accumulating powers
//! of the loop parameter δ for each closed loop.
//!
//! The non-crossing subset forms the **Temperley-Lieb subalgebra**: diagrams where
//! no arcs cross on either the source or target side, and through-lines are
//! monotonically increasing. The `is_def_tl` flag tracks this property.
//!
//! ## Generators
//!
//! - [`BrauerMorphism::temperley_lieb_gens`] — the TL generators `e_1, …, e_{n-1}`
//!   (cup-cap pairs in Hom(n, n))
//! - [`BrauerMorphism::symmetric_alg_gens`] — the symmetric group generators
//!   `s_1, …, s_{n-1}` (transpositions in Hom(n, n))
//!
//! Implements [`Composable`], [`Monoidal`], [`HasIdentity`], and
//! [`MonoidalMorphism`].
//!
//! See also `examples/temperley_lieb.rs` for the braid relation and generator usage.

use catgraph::errors::CatgraphError;

use {
    crate::linear_combination::LinearCombination,
    catgraph::{
        category::{Composable, HasIdentity},
        monoidal::{Monoidal, MonoidalMorphism},
    },
    itertools::Itertools,
    num::{One, Zero},
    std::{
        collections::HashSet,
        fmt::Debug,
        hash::Hash,
        ops::{Add, AddAssign, Mul, MulAssign},
    },
};

#[cfg(feature = "parallel")]
use rayon_cond::CondIterator;

/// Threshold gating the parallel arm of [`CondIterator`] in
/// [`BrauerMorphism::non_crossing`] per-side combinations check when the
/// `parallel` feature is enabled. Combinations grow as `n * (n - 1) / 2`, so
/// a source-line count of 8 yields 28 pairs.
// TODO(rayon-threshold): remeasure via `benches/rayon_thresholds.rs`. The
// current value of 8 is flagged by the 2026-04-10 gleaner2 audit as likely
// too low — the parallel arm's per-worker setup cost may dominate for small
// pair counts. Run `cargo bench -p catgraph-applied --bench rayon_thresholds`
// and adjust. See `~/.claude/summaries/rayon-summary-0.md` for the
// rustworkx-core `CondIterator` precedent adopted in Phase W.0.
#[cfg(feature = "parallel")]
const PARALLEL_COMBINATIONS_THRESHOLD: usize = 8;

/// An ordered pair of point indices, representing a matched arc in a Brauer diagram.
///
/// Points `0..source` lie on the domain (top) side and `source..source+target` on
/// the codomain (bottom) side. A pair connecting two domain points is a "cup",
/// two codomain points a "cap", and one from each side a "through-line".
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct Pair(pub usize, pub usize);

impl Pair {
    /// Iterate over both point indices of the pair.
    pub fn iter(&self) -> impl Iterator<Item = usize> {
        [self.0, self.1].into_iter()
    }

    /// Apply `f` to both point indices, returning a new pair.
    #[must_use]
    pub fn map(&self, f: impl Fn(usize) -> usize) -> Self {
        Self(f(self.0), f(self.1))
    }

    /// True if the predicate holds for both elements.
    pub fn all(&self, f: impl Fn(usize) -> bool) -> bool {
        f(self.0) && f(self.1)
    }

    /// True if the predicate holds for at least one element.
    pub fn any(&self, f: impl Fn(usize) -> bool) -> bool {
        f(self.0) || f(self.1)
    }

    fn flip_upside_down(&self, source: usize, target: usize) -> Self {
        self.map(|v| if v < source { v + target } else { v - source })
    }

    /// Return this pair with elements in ascending order.
    #[must_use]
    pub const fn sort(&self) -> Self {
        Self::sorted(self.0, self.1)
    }

    /// Construct a pair with the smaller element first.
    #[must_use]
    pub const fn sorted(x: usize, y: usize) -> Self {
        if x < y { Self(x, y) } else { Self(y, x) }
    }

    /// True if `x` lies strictly between the two point indices (used for crossing detection).
    #[must_use]
    pub const fn contains(&self, x: usize) -> bool {
        (x < self.0 && x > self.1) || (x < self.1 && x > self.0)
    }
}

impl From<(usize, usize)> for Pair {
    fn from(value: (usize, usize)) -> Self {
        Self(value.0, value.1)
    }
}

#[derive(PartialEq, Eq, Clone, Hash, Debug)]
struct PerfectMatching {
    pairs: Vec<Pair>,
}

impl FromIterator<Pair> for PerfectMatching {
    fn from_iter<T: IntoIterator<Item = Pair>>(pair_prime: T) -> Self {
        let pairs: Vec<Pair> = pair_prime.into_iter().collect();
        let max_expected = pairs.len() * 2;
        let seen: HashSet<_> = pairs
            .iter()
            .flat_map(|x| {
                assert!(x.all(|x| x < max_expected));
                x.iter()
            })
            .collect();
        assert_eq!(seen.len(), max_expected);
        let mut ret_val = Self { pairs };

        ret_val.canonicalize();
        ret_val
    }
}

impl From<Vec<Pair>> for PerfectMatching {
    fn from(value: Vec<Pair>) -> Self {
        Self::from_iter(value)
    }
}

impl PerfectMatching {
    fn canonicalize(&mut self) {
        for Pair(p, q) in &mut self.pairs {
            if *p > *q {
                std::mem::swap(p, q);
            }
        }

        self.pairs.sort();
    }

    fn flip_upside_down(&self, source: usize, target: usize) -> Self {
        self.pairs
            .iter()
            .map(|x| x.flip_upside_down(source, target))
            .collect()
    }

    fn non_crossing(&self, source: usize, _target: usize) -> bool {
        let source_lines: Vec<_> = self
            .pairs
            .iter()
            .filter(|p| p.all(|x| x < source))
            .copied()
            .collect();

        // Check for crossings in source lines. `combinations(2)` is a lazy
        // itertools iterator — collect to `Vec` so `CondIterator::new` can
        // dispatch to either `into_par_iter` (rayon) or `into_iter` (std).
        let source_combos: Vec<Vec<Pair>> = source_lines.iter().copied().combinations(2).collect();
        let has_crossing = |cur_item: Vec<Pair>| -> bool {
            let first_block = cur_item[0];
            let second_block = cur_item[1];
            first_block.contains(second_block.0) != first_block.contains(second_block.1)
        };
        #[cfg(feature = "parallel")]
        let source_has_crossing = CondIterator::new(
            source_combos,
            source_lines.len() >= PARALLEL_COMBINATIONS_THRESHOLD,
        )
        .any(has_crossing);
        #[cfg(not(feature = "parallel"))]
        let source_has_crossing = source_combos.into_iter().any(has_crossing);
        if source_has_crossing {
            return false;
        }

        // no crossing lines can use these indices because they are blocked by a line connecting
        //      two source points
        let mut no_through_lines_idx: HashSet<_> = source_lines
            .iter()
            .flat_map(|Pair(x, y)| (1 + x.min(y))..*x.max(y))
            .collect();

        // the lines connecting two points both on target side
        let target_lines: Vec<_> = self
            .pairs
            .iter()
            .filter(|p| p.all(|x| x >= source))
            .copied()
            .collect();

        // Check for crossings in target lines (same `CondIterator` pattern,
        // reusing the `has_crossing` predicate defined above).
        let target_combos: Vec<Vec<Pair>> = target_lines.iter().copied().combinations(2).collect();
        #[cfg(feature = "parallel")]
        let target_has_crossing = CondIterator::new(
            target_combos,
            target_lines.len() >= PARALLEL_COMBINATIONS_THRESHOLD,
        )
        .any(has_crossing);
        #[cfg(not(feature = "parallel"))]
        let target_has_crossing = target_combos.into_iter().any(has_crossing);
        if target_has_crossing {
            return false;
        }

        // no crossing lines can use these indices because they are blocked by a line connecting
        // two target points

        no_through_lines_idx.extend(
            target_lines
                .iter()
                .flat_map(|Pair(x, y)| (1 + x.min(y))..*x.max(y)),
        );

        // now check that those crossing lines don't use those indices that were stated to be forbidden
        #[allow(clippy::redundant_closure_for_method_calls)]
        let through_lines = self
            .pairs
            .iter()
            .filter(|Pair(z, w)| (*z < source && *w >= source) || (*w < source && *z >= source))
            .map(|p| p.sort());

        if through_lines
            .clone()
            .any(|p| p.any(|x| no_through_lines_idx.contains(&x)))
        {
            return false;
        }

        // the induced map from the through_lines is monotonically increasing
        through_lines.map(|Pair(_, w)| w).is_sorted()
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct ExtendedPerfectMatching((usize, usize, usize, PerfectMatching));

impl Mul for ExtendedPerfectMatching {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        // Diagram composition resolves the glued endpoint matching + closed-loop
        // (δ-power) count via an `ultragraph` connected-components pass; see
        // [`connectivity::resolve`]. The graph is well-formed by construction, so
        // the `ultragraph` operations cannot fail.
        connectivity::resolve(&self, &rhs).expect(
            "invariant: ultragraph composition on a well-formed Brauer diagram cannot fail",
        )
    }
}

/// A morphism in the Brauer algebra: Hom(source, target).
///
/// Internally a [`LinearCombination`] over `(delta_power, PerfectMatching)` pairs.
/// Each term represents a Brauer diagram scaled by `δ^k` where `k` tracks closed
/// loops accumulated during composition. The coefficient ring `T` must support
/// addition, multiplication, and the constants 0 and 1.
///
/// Use [`temperley_lieb_gens`](Self::temperley_lieb_gens) and
/// [`symmetric_alg_gens`](Self::symmetric_alg_gens) to obtain standard generators,
/// then compose/tensor them to build morphisms.
pub struct BrauerMorphism<T>
where
    T: Add<Output = T> + Zero + One + Copy,
{
    /// The formal sum of `(delta_power, diagram)` terms with coefficients in `T`.
    diagram: LinearCombination<T, (usize, PerfectMatching)>,
    /// Number of domain (top) points.
    source: usize,
    /// Number of codomain (bottom) points.
    target: usize,
    /// True if all terms are known to be non-crossing (Temperley-Lieb subalgebra).
    is_def_tl: bool,
}

impl<T> PartialEq for BrauerMorphism<T>
where
    T: Add<Output = T> + Zero + One + Copy + Eq,
{
    fn eq(&self, other: &Self) -> bool {
        self.diagram == other.diagram && self.source == other.source && self.target == other.target
    }
}

impl<T> Clone for BrauerMorphism<T>
where
    T: Add<Output = T> + Zero + One + Copy,
{
    fn clone(&self) -> Self {
        Self {
            diagram: self.diagram.clone(),
            source: self.source,
            target: self.target,
            is_def_tl: self.is_def_tl,
        }
    }
}

impl<T> Debug for BrauerMorphism<T>
where
    T: Add<Output = T> + Zero + One + Copy + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrauerMorphism")
            .field("diagram", &self.diagram)
            .field("source", &self.source)
            .field("target", &self.target)
            .field("is_def_tl", &self.is_def_tl)
            .finish()
    }
}

impl<T> HasIdentity<usize> for BrauerMorphism<T>
where
    T: Add<Output = T> + Zero + One + Copy,
{
    fn identity(on_this: &usize) -> Self {
        let matching: PerfectMatching = (0..*on_this).map(|x| Pair(x, x + on_this)).collect();
        Self {
            diagram: LinearCombination::singleton((0, matching)),
            source: *on_this,
            target: *on_this,
            is_def_tl: true,
        }
    }
}

impl<T> Composable<usize> for BrauerMorphism<T>
where
    T: Add<Output = T> + Zero + One + Copy + AddAssign + Mul<Output = T> + MulAssign + Send + Sync,
{
    fn compose(&self, other: &Self) -> Result<Self, CatgraphError> {
        self.composable(other)?;
        let extended_diagram_self = self.diagram.inj_linearly_extend(|(delta_pow, diagram)| {
            ExtendedPerfectMatching((self.domain(), self.codomain(), delta_pow, diagram))
        });
        let extended_diagram_other = other.diagram.inj_linearly_extend(|(delta_pow, diagram)| {
            ExtendedPerfectMatching((other.domain(), other.codomain(), delta_pow, diagram))
        });
        let extended_diagram_product = extended_diagram_self * extended_diagram_other;
        let diagram_product =
            extended_diagram_product.linearly_extend(|extended| (extended.0.2, extended.0.3));
        Ok(Self {
            diagram: diagram_product,
            source: self.domain(),
            target: other.codomain(),
            is_def_tl: self.is_def_tl && other.is_def_tl,
        })
    }

    fn domain(&self) -> usize {
        self.source
    }

    fn codomain(&self) -> usize {
        self.target
    }
}

impl<T> Monoidal for BrauerMorphism<T>
where
    T: Add<Output = T> + Zero + One + Copy + AddAssign + Mul<Output = T> + MulAssign + Send + Sync,
{
    fn monoidal(&mut self, other: Self) {
        let old_domain = self.domain();
        let old_codomain = self.codomain();
        let other_domain = other.domain();
        self.source += other_domain;
        self.target += other.codomain();
        let new_domain = self.domain();
        self.is_def_tl &= other.is_def_tl;
        let shift_pairs =
            |diagram: &PerfectMatching, if_above: usize, shift_amount: usize| -> Vec<Pair> {
                diagram
                    .pairs
                    .iter()
                    .map(|p| p.map(|v| if v >= if_above { v + shift_amount } else { v }))
                    .collect()
            };
        self.diagram = self.diagram.linear_combine(
            other.diagram,
            |(delta_pow1, matching_1), (delta_pow2, matching2)| {
                let mut combined = shift_pairs(&matching_1, old_domain, other_domain);
                let other_shifted = shift_pairs(&matching2, 0, old_domain);
                let other_reshifted: Vec<Pair> = other_shifted
                    .iter()
                    .map(|p| p.map(|v| if v >= new_domain { v + old_codomain } else { v }))
                    .collect();
                combined.extend(other_reshifted);
                let new_matching: PerfectMatching = combined.into();
                (delta_pow1 + delta_pow2, new_matching)
            },
        );
    }
}

impl<T> MonoidalMorphism<usize> for BrauerMorphism<T> where
    T: Add<Output = T> + Zero + One + Copy + AddAssign + Mul<Output = T> + MulAssign + Send + Sync
{
}

impl<T> BrauerMorphism<T>
where
    T: Add<Output = T> + Zero + One + Copy + AddAssign + Mul<Output = T> + MulAssign,
{
    /// The Temperley-Lieb generators e\_1, …, e\_{n-1} in Hom(n, n).
    ///
    /// Generator `e_i` pairs domain point `i` with `i+1` (cup) and codomain
    /// point `i` with `i+1` (cap), with all other points connected straight across.
    #[must_use]
    pub fn temperley_lieb_gens(n: usize) -> Vec<Self> {
        (0..n - 1)
            .map(|i| {
                let e_i_matching: PerfectMatching = (0..n)
                    .map(|j| {
                        (if j == i {
                            (i, i + 1)
                        } else if j == i + 1 {
                            (i + n, i + 1 + n)
                        } else {
                            (j, j + n)
                        })
                        .into()
                    })
                    .collect();
                Self {
                    diagram: LinearCombination::singleton((0, e_i_matching)),
                    source: n,
                    target: n,
                    is_def_tl: true,
                }
            })
            .collect()
    }

    /// The symmetric group generators s\_1, …, s\_{n-1} in Hom(n, n).
    ///
    /// Generator `s_i` crosses positions `i` and `i+1` (a transposition),
    /// matching the rest straight across. These generate the full Brauer algebra
    /// together with the Temperley-Lieb generators.
    #[must_use]
    pub fn symmetric_alg_gens(n: usize) -> Vec<Self> {
        (0..(n - 1))
            .map(|i| {
                let e_i_matching: PerfectMatching = (0..n)
                    .map(|j| {
                        (if j == i {
                            (i, i + n + 1)
                        } else if j == i + 1 {
                            (i + 1, i + n)
                        } else {
                            (j, j + n)
                        })
                        .into()
                    })
                    .collect();
                Self {
                    diagram: LinearCombination::singleton((0, e_i_matching)),
                    source: n,
                    target: n,
                    is_def_tl: false,
                }
            })
            .collect()
    }

    /// Construct a polynomial in δ as a scalar morphism in Hom(0, 0).
    ///
    /// `coeffs[i]` is the coefficient of δ^i. This represents a closed
    /// diagram (no external points) — the "ground ring" element.
    pub fn delta_polynomial(coeffs: &[T]) -> Self {
        let zeroth_coeff = *coeffs.first().unwrap_or(&T::zero());
        let empty_matching = PerfectMatching { pairs: vec![] };
        let mut diagram = LinearCombination::singleton((0, empty_matching));
        diagram *= zeroth_coeff;
        for (idx, cur_coeff) in coeffs.iter().enumerate().skip(1) {
            let empty_matching = PerfectMatching { pairs: vec![] };
            let mut cur_diagram = LinearCombination::singleton((idx, empty_matching));
            cur_diagram *= *cur_coeff;
            diagram += cur_diagram;
        }
        Self {
            diagram,
            source: 0,
            target: 0,
            is_def_tl: true,
        }
    }

    /// Dagger (adjoint): reflect each diagram vertically (swap source ↔ target
    /// sides) and apply `num_dagger` to every coefficient.
    ///
    /// For the standard involution, pass `|x| x` (or conjugation for complex coefficients).
    #[must_use]
    pub fn dagger<F>(&self, num_dagger: F) -> Self
    where
        F: Fn(T) -> T,
    {
        let mut diagram = self
            .diagram
            .inj_linearly_extend(|(d, m)| (d, m.flip_upside_down(self.source, self.target)));
        diagram.change_coeffs(num_dagger);
        Self {
            diagram,
            source: self.target,
            target: self.source,
            is_def_tl: self.is_def_tl,
        }
    }

    /// Check and cache whether all terms are non-crossing (Temperley-Lieb).
    ///
    /// Iterates over every diagram term and verifies the non-crossing property.
    /// No-op if the flag is already set. Call this after constructing a morphism
    /// from raw diagrams if you need the TL guarantee for downstream optimizations.
    pub fn set_is_tl(&mut self) {
        if self.is_def_tl {
            return;
        }
        self.is_def_tl = self
            .diagram
            .all_terms_satisfy(|(_, p)| p.non_crossing(self.source, self.target));
    }
}

impl<T> BrauerMorphism<T>
where
    T: Add<Output = T> + Zero + One + Copy + Eq,
{
    /// Remove all terms with zero coefficient.
    pub fn simplify(&mut self) {
        self.diagram.simplify();
    }
}

#[cfg(test)]
mod test {
    use std::ops::{AddAssign, MulAssign};

    use catgraph::errors::CatgraphError;

    use super::BrauerMorphism;
    use either::Either;
    use num::{One, Zero};

    fn test_helper<T: Eq + AddAssign + MulAssign + Copy + One + Zero + Send + Sync>(
        e_i: &[BrauerMorphism<T>],
        s_i: &[BrauerMorphism<T>],
        prod_these: &[Either<usize, usize>],
        delta_poly_coeffs: &[T],
    ) -> Result<BrauerMorphism<T>, CatgraphError> {
        fn get_generator<T: Clone>(l_gens: &[T], r_gens: &[T], which: Either<usize, usize>) -> T {
            use catgraph::utils::EitherExt;
            which.join(|n| l_gens[n].clone(), |n| r_gens[n].clone())
        }
        use catgraph::{category::Composable, monoidal::Monoidal};
        assert!(!prod_these.is_empty());
        let prod_these_0 = get_generator(e_i, s_i, prod_these[0]);
        let mut delta_poly = BrauerMorphism::delta_polynomial(delta_poly_coeffs);
        delta_poly.simplify();
        if prod_these.len() == 1 {
            let mut full_prod = prod_these_0;
            full_prod.monoidal(delta_poly);
            return Ok(full_prod);
        }
        let prod_these_1 = get_generator(e_i, s_i, prod_these[1]);
        let mut full_prod = prod_these_0.compose(&prod_these_1);
        for cur_idx in prod_these.iter().skip(2) {
            let cur = get_generator(e_i, s_i, *cur_idx);
            full_prod = full_prod.and_then(|z| z.compose(&cur));
        }
        match full_prod {
            Ok(mut t) => {
                t.monoidal(delta_poly);
                Ok(t)
            }
            Err(e) => Err(e),
        }
    }

    #[test]
    fn t_l_relations() {
        use catgraph::{category::Composable, utils::test_asserter};
        use either::Either::Left;
        use num::Complex;
        let e_i = BrauerMorphism::<Complex<i32>>::temperley_lieb_gens(5);
        let delta_coeffs: [Complex<i32>; 2] = [<_>::zero(), <_>::one()];
        for idx in 0..e_i.len() {
            assert!(e_i[idx].is_def_tl);
            let e_i_dag = e_i[idx].dagger(|z| z.conj());
            assert!(
                e_i[idx] == e_i_dag,
                "{:?} vs {:?} when checking self adjointness of e_i",
                e_i[idx],
                e_i_dag
            );
            let e_ie_i = e_i[idx].compose(&e_i[idx]);
            let deltae_i = test_helper(&e_i, &[], &[Left(idx)], &delta_coeffs);
            test_asserter(
                e_ie_i,
                deltae_i,
                |j, k| j.is_def_tl && k.is_def_tl,
                "e_i e_i = delta e_i",
            );
            if idx < e_i.len() - 1 {
                let prod_iji = e_i[idx]
                    .compose(&e_i[idx + 1])
                    .and_then(|z| z.compose(&e_i[idx]));
                test_asserter(
                    prod_iji,
                    Ok(e_i[idx].clone()),
                    |j, k| j.is_def_tl && k.is_def_tl,
                    "e_i e_(i+1) e_i = e_i",
                );
            }
            if idx > 1 {
                let prod_iji = e_i[idx]
                    .compose(&e_i[idx - 1])
                    .and_then(|z| z.compose(&e_i[idx]));
                test_asserter(
                    prod_iji,
                    Ok(e_i[idx].clone()),
                    |j, k| j.is_def_tl && k.is_def_tl,
                    "e_i e_(i-1) e_i = e_i",
                );
            }
            for jdx in idx + 2..e_i.len() {
                let prod_ij = e_i[idx].compose(&e_i[jdx]);
                let prod_ji = e_i[jdx].compose(&e_i[idx]);
                test_asserter(
                    prod_ij,
                    prod_ji,
                    |j, k| j.is_def_tl && k.is_def_tl,
                    "e_i e_j = e_j e_i",
                );
            }
        }
    }

    #[test]
    fn wiki_example() {
        use super::BrauerMorphism;
        use catgraph::{category::Composable, monoidal::Monoidal};
        use num::Complex;
        let e_i = BrauerMorphism::<Complex<i32>>::temperley_lieb_gens(5);
        let zero_complex = Complex::<i32>::zero();
        let one_complex = Complex::<i32>::one();
        let prod_1432 = e_i[0]
            .compose(&e_i[3])
            .and_then(|z| z.compose(&e_i[2]))
            .and_then(|z| z.compose(&e_i[1]));
        let prod_243 = e_i[1].compose(&e_i[3]).and_then(|z| z.compose(&e_i[2]));
        let prod_143243 = e_i[0]
            .compose(&e_i[3])
            .and_then(|z| z.compose(&e_i[2]))
            .and_then(|z| z.compose(&e_i[1]))
            .and_then(|z| z.compose(&e_i[3]))
            .and_then(|z| z.compose(&e_i[2]));
        let observed = prod_1432.and_then(|z| match prod_243 {
            Ok(real_prod_243) => z.compose(&real_prod_243),
            Err(e) => Err(e),
        });
        let mut expected =
            BrauerMorphism::<Complex<i32>>::delta_polynomial(&[zero_complex, one_complex]);
        expected.simplify();
        match (observed, prod_143243) {
            (Ok(real_obs), Ok(exp_wo_delta)) => {
                assert!(real_obs.is_def_tl);
                expected.monoidal(exp_wo_delta);
                assert!(expected.is_def_tl);
                assert!(PartialEq::eq(&real_obs, &expected));
            }
            _ => {
                panic!(
                    "Error in composition when checking (e_1 e_4 e_3 e_2) (e_2 e_4 e_3) = delta e_1 e_4 e_3 e_2 e_4 e_3"
                )
            }
        }
    }

    #[test]
    fn sym_relations() {
        use super::BrauerMorphism;
        use catgraph::{
            category::{Composable, HasIdentity},
            utils::test_asserter,
        };
        use either::Either::Right;
        use num::Complex;
        let n = 7;
        let s_i = BrauerMorphism::<Complex<i32>>::symmetric_alg_gens(n);
        let one_poly_coeffs = [Complex::<i32>::one()];
        let identity = BrauerMorphism::<Complex<i32>>::identity(&n);
        for idx in 0..n - 1 {
            assert!(!s_i[idx].is_def_tl);
            let s_i_dag = s_i[idx].dagger(|z| z.conj());
            assert!(
                PartialEq::eq(&s_i[idx], &s_i_dag),
                "{:?} vs {:?} when checking self adjointness of s_i",
                s_i[idx],
                s_i_dag
            );
            let s_is_i = s_i[idx].compose(&s_i[idx]);
            test_asserter(
                s_is_i,
                Ok(identity.clone()),
                |j, k| !j.is_def_tl && k.is_def_tl,
                "s_i s_i = 1",
            );
            if idx < n - 2 {
                let s_is_js_i = test_helper(
                    &[],
                    &s_i,
                    &[Right(idx), Right(idx + 1), Right(idx)],
                    &one_poly_coeffs,
                );
                let s_js_is_j = test_helper(
                    &[],
                    &s_i,
                    &[Right(idx + 1), Right(idx), Right(idx + 1)],
                    &one_poly_coeffs,
                );
                test_asserter(
                    s_is_js_i,
                    s_js_is_j,
                    |j, k| !j.is_def_tl && !k.is_def_tl,
                    "s_i s_(i+1) s_i = s_(i+1) s_i s_(i+1)",
                );
            }
            if idx > 1 {
                let s_is_js_i = test_helper(
                    &[],
                    &s_i,
                    &[Right(idx), Right(idx - 1), Right(idx)],
                    &one_poly_coeffs,
                );
                let s_js_is_j = test_helper(
                    &[],
                    &s_i,
                    &[Right(idx - 1), Right(idx), Right(idx - 1)],
                    &one_poly_coeffs,
                );
                test_asserter(
                    s_is_js_i,
                    s_js_is_j,
                    |j, k| !j.is_def_tl && !k.is_def_tl,
                    "s_i s_(i-1) s_i = s_(i-1) s_i s_(i-1)",
                );
            }
            for jdx in idx + 2..s_i.len() {
                let prod_ij = s_i[idx].compose(&s_i[jdx]);
                let prod_ji = s_i[jdx].compose(&s_i[idx]);
                test_asserter(
                    prod_ij,
                    prod_ji,
                    |j, k| !j.is_def_tl && !k.is_def_tl,
                    "s_i s_j = s_j s_i",
                );
            }
        }
    }

    #[test]
    fn tangle_relations() {
        use super::BrauerMorphism;
        use catgraph::{category::Composable, utils::test_asserter};
        use either::Either::{Left, Right};
        use num::Complex;
        let n = 7;
        let s_i = BrauerMorphism::<Complex<i32>>::symmetric_alg_gens(n);
        let e_i = BrauerMorphism::<Complex<i32>>::temperley_lieb_gens(n);
        let one_poly_coeffs = [Complex::<i32>::one()];
        for idx in 0..n - 1 {
            let e_is_i = e_i[idx].compose(&s_i[idx]);
            let s_ie_i: Result<BrauerMorphism<Complex<i32>>, CatgraphError> =
                s_i[idx].compose(&e_i[idx]);
            test_asserter(
                e_is_i,
                Ok(e_i[idx].clone()),
                |j, k| !j.is_def_tl && k.is_def_tl,
                "e_i s_i = e_i",
            );
            test_asserter(
                s_ie_i,
                Ok(e_i[idx].clone()),
                |j, k| !j.is_def_tl && k.is_def_tl,
                "s_i e_i = e_i",
            );
            if idx < n - 2 {
                let s_is_je_i = test_helper(
                    &e_i,
                    &s_i,
                    &[Right(idx), Right(idx + 1), Left(idx)],
                    &one_poly_coeffs,
                );
                let e_je_i = test_helper(&e_i, &s_i, &[Left(idx + 1), Left(idx)], &one_poly_coeffs);
                test_asserter(
                    s_is_je_i,
                    e_je_i,
                    |j, k| !j.is_def_tl && k.is_def_tl,
                    "s_i s_(i+1) e_i = e_(i+1) e_i",
                );
                let e_is_je_i = test_helper(
                    &e_i,
                    &s_i,
                    &[Left(idx), Right(idx + 1), Left(idx)],
                    &one_poly_coeffs,
                );
                test_asserter(
                    e_is_je_i,
                    Ok(e_i[idx].clone()),
                    |j, k| !j.is_def_tl && k.is_def_tl,
                    "e_i s_(i+1) e_i = e_i",
                );
            }
            if idx > 1 {
                let s_is_je_i = test_helper(
                    &e_i,
                    &s_i,
                    &[Right(idx), Right(idx - 1), Left(idx)],
                    &one_poly_coeffs,
                );
                let e_je_i = test_helper(&e_i, &s_i, &[Left(idx - 1), Left(idx)], &one_poly_coeffs);
                test_asserter(
                    s_is_je_i,
                    e_je_i,
                    |j, k| !j.is_def_tl && k.is_def_tl,
                    "s_i s_(i-1) e_i = e_(i-1) e_i",
                );
                let e_is_je_i = test_helper(
                    &e_i,
                    &s_i,
                    &[Left(idx), Right(idx - 1), Left(idx)],
                    &one_poly_coeffs,
                );
                test_asserter(
                    e_is_je_i,
                    Ok(e_i[idx].clone()),
                    |j, k| !j.is_def_tl && k.is_def_tl,
                    "e_i s_(i-1) e_i = e_i",
                );
            }
            #[allow(clippy::needless_range_loop)]
            for jdx in idx + 2..s_i.len() {
                let prod_ij = s_i[idx].compose(&e_i[jdx]);
                let prod_ji = e_i[jdx].compose(&s_i[idx]);
                test_asserter(
                    prod_ij,
                    prod_ji,
                    |j, k| !j.is_def_tl && !k.is_def_tl,
                    "s_i e_j = e_j s_i",
                );
            }
        }
    }
}

/// `ultragraph`-backed resolution of the connectivity core of
/// `<ExtendedPerfectMatching as Mul>::mul`. Composition glues two Brauer diagrams
/// and reads off (a) the endpoint matching of the result and (b) the number of
/// closed loops (the δ-power increment) — both from the connected components of
/// the glued diagram graph.
///
/// `ultragraph` is a directed graph, so each undirected diagram arc is added in
/// both directions; under that construction `strongly_connected_components()`
/// equals the undirected component count, and same-component endpoints are the
/// matched pairs. Components are labelled once (O(V+E)) and the matching resolved
/// by O(1) component-id comparison.
mod connectivity {
    use super::{ExtendedPerfectMatching, Pair, PerfectMatching};
    use std::collections::HashSet;
    use ultragraph::*;

    /// The frozen composite graph + the scalar bookkeeping [`resolve`] needs.
    struct Composite {
        g: UltraGraph<()>,
        node_idcs: Vec<Option<usize>>,
        self_dom: usize,
        self_cod: usize,
        rhs_cod: usize,
        self_delta_pow: usize,
        rhs_delta_pow: usize,
    }

    /// Build + freeze the composite graph of `lhs ∘ rhs`, each undirected diagram
    /// arc added bidirectionally so SCC count == undirected component count.
    fn build_frozen(
        lhs: &ExtendedPerfectMatching,
        rhs: &ExtendedPerfectMatching,
    ) -> Result<Composite, GraphError> {
        let (self_dom, self_cod, self_delta_pow) = (lhs.0.0, lhs.0.1, lhs.0.2);
        let self_pairs = &lhs.0.3.pairs;
        let (rhs_dom, rhs_cod, rhs_delta_pow) = (rhs.0.0, rhs.0.1, rhs.0.2);
        let rhs_pairs = &rhs.0.3.pairs;
        assert_eq!(rhs_dom, self_cod, "composition dom/cod mismatch");

        let mut g: UltraGraph<()> =
            UltraGraph::with_capacity(self_dom + self_cod + rhs_cod, None);
        let mut node_idcs: Vec<Option<usize>> = vec![None; self_dom + self_cod + rhs_cod];

        // self diagram: a fresh node per endpoint, one bidirectional edge per pair.
        for &Pair(p, q) in self_pairs {
            let p_loc = g.add_node(())?;
            node_idcs[p] = Some(p_loc);
            let q_loc = g.add_node(())?;
            node_idcs[q] = Some(q_loc);
            add_undirected(&mut g, p_loc, q_loc)?;
        }
        // rhs diagram: reuse the glued domain nodes (p < rhs_dom), fresh codomain.
        for &Pair(p, q) in rhs_pairs {
            let p_loc = resolve_rhs_node(&mut g, &mut node_idcs, p, rhs_dom, self_dom)?;
            let q_loc = resolve_rhs_node(&mut g, &mut node_idcs, q, rhs_dom, self_dom)?;
            add_undirected(&mut g, p_loc, q_loc)?;
        }

        // "freeze = compile": algorithms run only on the frozen CSR graph.
        g.freeze();
        Ok(Composite {
            g,
            node_idcs,
            self_dom,
            self_cod,
            rhs_cod,
            self_delta_pow,
            rhs_delta_pow,
        })
    }

    /// Resolve `lhs ∘ rhs`: label components with one SCC pass, then match
    /// endpoints by O(1) component-id comparison. The δ-power increment is the
    /// component count less the matched-pair count.
    pub(super) fn resolve(
        lhs: &ExtendedPerfectMatching,
        rhs: &ExtendedPerfectMatching,
    ) -> Result<ExtendedPerfectMatching, GraphError> {
        let c = build_frozen(lhs, rhs)?;

        let sccs = c.g.strongly_connected_components()?;
        let mut component_of: Vec<usize> = vec![usize::MAX; c.g.number_nodes()];
        for (cid, comp) in sccs.iter().enumerate() {
            for &node in comp {
                component_of[node] = cid;
            }
        }

        let endpoints = c.self_dom + c.rhs_cod;
        let mut endpoints_done: HashSet<usize> = HashSet::with_capacity(endpoints);
        let mut final_matching: Vec<Pair> = Vec::with_capacity(endpoints / 2);
        #[allow(clippy::needless_range_loop)]
        for i in 0..endpoints {
            if endpoints_done.contains(&i) {
                continue;
            }
            let i_cid = component_of[endpoint_node(&c.node_idcs, i, c.self_dom, c.self_cod)];
            for j in (i + 1)..endpoints {
                let j_cid = component_of[endpoint_node(&c.node_idcs, j, c.self_dom, c.self_cod)];
                if i_cid == j_cid {
                    final_matching.push(Pair(i, j));
                    endpoints_done.insert(i);
                    endpoints_done.insert(j);
                    break;
                }
            }
        }

        let new_delta_power = sccs.len() + c.self_delta_pow + c.rhs_delta_pow - (endpoints / 2);
        Ok(ExtendedPerfectMatching((
            c.self_dom,
            c.rhs_cod,
            new_delta_power,
            PerfectMatching { pairs: final_matching },
        )))
    }

    /// Add an undirected edge as a directed pair (both orientations).
    fn add_undirected(g: &mut UltraGraph<()>, a: usize, b: usize) -> Result<(), GraphError> {
        g.add_edge(a, b, ())?;
        g.add_edge(b, a, ())?;
        Ok(())
    }

    /// Resolve (or lazily create) the node index for an rhs endpoint, reusing the
    /// glued self-codomain node when the rhs point is a domain point.
    fn resolve_rhs_node(
        g: &mut UltraGraph<()>,
        node_idcs: &mut [Option<usize>],
        p: usize,
        rhs_dom: usize,
        self_dom: usize,
    ) -> Result<usize, GraphError> {
        if p >= rhs_dom {
            let loc = g.add_node(())?;
            node_idcs[p + self_dom] = Some(loc);
            Ok(loc)
        } else {
            Ok(node_idcs[p + self_dom]
                .expect("invariant: glued rhs-domain node was populated by the self diagram"))
        }
    }

    /// Map a composite-diagram endpoint id to its graph node index.
    fn endpoint_node(node_idcs: &[Option<usize>], i: usize, self_dom: usize, self_cod: usize) -> usize {
        node_idcs[if i < self_dom { i } else { i + self_cod }]
            .expect("invariant: endpoint node index populated during graph construction")
    }
}
