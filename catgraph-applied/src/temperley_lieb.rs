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
// TODO(rayon-threshold, #37): remeasure via `benches/rayon_thresholds.rs`. The
// current value of 8 was flagged by a pre-reboot audit as likely too low —
// the parallel arm's per-worker setup cost may dominate for small pair
// counts. Run `cargo bench -p catgraph-applied --bench rayon_thresholds`
// and adjust.
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
        // (δ-power) count via a union-find connected-components pass; see
        // [`connectivity::resolve`], which is total.
        connectivity::resolve(&self, &rhs)
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

    use super::{BrauerMorphism, Pair, PerfectMatching};
    use catgraph_testutil::Lcg;
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

    // ---------------------------------------------------------------------
    // #294 — value oracles for `PerfectMatching::non_crossing` and for
    // `Monoidal::monoidal` at a non-zero shift.
    //
    // These live beside the implementation rather than in
    // `tests/temperley_lieb.rs` because the only public reader of `is_def_tl`
    // is the `Debug` impl, and the pair set of a diagram has no public reader
    // at all. A child module reads the fields; the integration suite would
    // need a new accessor to say anything at all.
    // ---------------------------------------------------------------------

    /// `Pair::contains` against strict betweenness, over every `(a, b, x)` drawn
    /// from `0..6` — 216 cases, both argument orders and `a == b` included.
    ///
    /// `contains` is public and its contract is order-agnostic, but no fixture
    /// reaching it through `non_crossing` supplies a descending pair: deleting
    /// the `(x < self.0 && x > self.1)` disjunct leaves the crate suite green.
    /// This asserts the contract directly. The reference normalises with
    /// `min`/`max` instead of case-splitting on the argument order, so it
    /// shares no branch with production.
    #[test]
    fn pair_contains_is_strict_betweenness_in_either_order() {
        for a in 0..6usize {
            for b in 0..6usize {
                for x in 0..6usize {
                    let want = x > a.min(b) && x < a.max(b);
                    assert_eq!(Pair(a, b).contains(x), want, "Pair({a}, {b}).contains({x})");
                }
            }
        }
    }

    /// Planarity of a Brauer diagram, decided by walking its boundary.
    ///
    /// Draw Hom(`source`, `target`) in a rectangle with the domain points left
    /// to right along the top edge and the codomain points left to right along
    /// the bottom. Walking the rectangle's boundary once visits the domain in
    /// ascending label order and then the codomain in *descending* label order;
    /// `walk_position` is that walk. Arcs drawn inside the rectangle avoid each
    /// other exactly when the walk reads as a balanced parenthesis string, so
    /// cancelling adjacent partners off a stack empties it iff the diagram is
    /// planar.
    ///
    /// This is a different algorithm from the one under test, not a paraphrase
    /// of it. [`PerfectMatching::non_crossing`] splits planarity into four
    /// special cases — cup/cup interleaving, cap/cap interleaving, through-lines
    /// landing on an index some cup or cap blocks, and monotonicity of the
    /// through-line map — and never builds the boundary order at all.
    fn boundary_walk_planar(pairs: &[Pair], source: usize, target: usize) -> bool {
        let points = source + target;
        let walk_position = |v: usize| {
            if v < source {
                v
            } else {
                source + (points - 1 - v)
            }
        };
        let mut partner = vec![usize::MAX; points];
        for p in pairs {
            let (a, b) = (walk_position(p.0), walk_position(p.1));
            partner[a] = b;
            partner[b] = a;
        }
        let mut stack: Vec<usize> = Vec::with_capacity(points);
        for i in 0..points {
            if stack.last().is_some_and(|&top| partner[top] == i) {
                stack.pop();
            } else {
                stack.push(i);
            }
        }
        stack.is_empty()
    }

    /// Every perfect matching on the point set `0..2k`.
    fn all_matchings(k: usize) -> Vec<Vec<Pair>> {
        fn go(remaining: &[usize], acc: &mut Vec<Pair>, out: &mut Vec<Vec<Pair>>) {
            let Some((first, rest)) = remaining.split_first() else {
                out.push(acc.clone());
                return;
            };
            for (i, &other) in rest.iter().enumerate() {
                acc.push(Pair(*first, other));
                let mut narrowed: Vec<usize> = rest.to_vec();
                narrowed.remove(i);
                go(&narrowed, acc, out);
                acc.pop();
            }
        }
        let points: Vec<usize> = (0..2 * k).collect();
        let mut out = Vec::new();
        go(&points, &mut Vec::new(), &mut out);
        out
    }

    /// `(2k - 1)!!`, the number of perfect matchings on `2k` points.
    fn perfect_matching_count(k: usize) -> usize {
        (1..=k).map(|i| 2 * i - 1).product()
    }

    /// `C_k`, from the Segner recurrence — the number of planar perfect
    /// matchings of `2k` points arranged on a circle.
    fn catalan(k: usize) -> usize {
        let mut c = vec![0usize; k + 1];
        c[0] = 1;
        for i in 1..=k {
            for j in 0..i {
                c[i] += c[j] * c[i - 1 - j];
            }
        }
        c[k]
    }

    /// `non_crossing` against the boundary walk on every perfect matching of up
    /// to ten points, at every domain/codomain split of those points.
    ///
    /// Two oracles, because a pointwise check is only as good as its reference:
    ///
    /// * pointwise — production against [`boundary_walk_planar`];
    /// * by count — a split `source + target = 2k` only relabels which of the
    ///   `2k` boundary positions is which, so exactly `C_k` of the matchings are
    ///   planar at *every* split, whatever the split. Neither constant can
    ///   satisfy that for `k >= 2`, and neither can a `Pair::contains` that has
    ///   stopped discriminating.
    ///
    /// Ten points is the ceiling of the exhaustive enumeration. It never reaches
    /// the `parallel` arms, which need eight arcs on one side and so at least
    /// sixteen domain points; `non_crossing_parallel_arms_public_api` and
    /// `non_crossing_parallel_arms_seeded` cover those.
    #[test]
    fn non_crossing_exhaustive_to_ten_points() {
        for k in 1..=5 {
            let matchings: Vec<PerfectMatching> = all_matchings(k)
                .into_iter()
                .map(PerfectMatching::from)
                .collect();
            // "every matching" is the claim the two oracles below range over, so
            // pin the enumeration itself: distinct, and all of them.
            let distinct: std::collections::BTreeSet<&Vec<Pair>> =
                matchings.iter().map(|m| &m.pairs).collect();
            assert_eq!(
                distinct.len(),
                perfect_matching_count(k),
                "the enumeration yielded {} distinct matchings on {} points, \
                 not (2k-1)!! = {}",
                distinct.len(),
                2 * k,
                perfect_matching_count(k)
            );
            let expected_planar = catalan(k);
            for source in 0..=2 * k {
                let target = 2 * k - source;
                let mut planar = 0usize;
                for m in &matchings {
                    let got = m.non_crossing(source, target);
                    let want = boundary_walk_planar(&m.pairs, source, target);
                    assert_eq!(
                        got, want,
                        "non_crossing said {got} and the boundary walk said {want} \
                         for {:?} in Hom({source}, {target})",
                        m.pairs
                    );
                    planar += usize::from(got);
                }
                assert_eq!(
                    planar,
                    expected_planar,
                    "{planar} of the perfect matchings on {} points passed non_crossing \
                     in Hom({source}, {target}), but exactly C_{k} = {expected_planar} \
                     of them are planar",
                    2 * k
                );
            }
        }
    }

    /// The number of arcs with both endpoints on the domain side, then on the
    /// codomain side — the two counts `non_crossing` tests against
    /// `PARALLEL_COMBINATIONS_THRESHOLD`.
    #[cfg(feature = "parallel")]
    fn same_side_arc_counts(pairs: &[Pair], source: usize) -> (usize, usize) {
        (
            pairs.iter().filter(|q| q.all(|x| x < source)).count(),
            pairs.iter().filter(|q| q.all(|x| x >= source)).count(),
        )
    }

    /// Assert both same-side arc counts reach the parallel threshold.
    ///
    /// The counts are the operands each `CondIterator` site's dispatch
    /// condition is applied to, not the condition itself. Changing both sites'
    /// comparison so neither arm dispatches leaves the crate suite green.
    ///
    /// A no-op without the feature, where neither site exists.
    fn assert_parallel_arms_dispatch(pairs: &[Pair], source: usize, what: &str) {
        #[cfg(feature = "parallel")]
        {
            let (domain_arcs, codomain_arcs) = same_side_arc_counts(pairs, source);
            let threshold = super::PARALLEL_COMBINATIONS_THRESHOLD;
            assert!(
                domain_arcs >= threshold && codomain_arcs >= threshold,
                "{what} has {domain_arcs} domain-side and {codomain_arcs} codomain-side arcs, \
                 below the parallel threshold of {threshold}"
            );
        }
        #[cfg(not(feature = "parallel"))]
        {
            let _ = (pairs, source, what);
        }
    }

    /// Collect a morphism's `(delta power, pairs)` basis, sorted.
    ///
    /// `LinearCombination` keeps its map private and exposes no iterator, so
    /// this borrows `all_terms_satisfy`. The predicate is constantly true
    /// because that method short-circuits on false.
    fn terms(m: &BrauerMorphism<i64>) -> Vec<(usize, Vec<Pair>)> {
        let collected = std::cell::RefCell::new(Vec::new());
        m.diagram
            .all_terms_satisfy(|(delta_pow, matching): &(usize, PerfectMatching)| {
                collected
                    .borrow_mut()
                    .push((*delta_pow, matching.pairs.clone()));
                true
            });
        let mut out = collected.into_inner();
        out.sort();
        out
    }

    /// Both parallel arms of `non_crossing`, in both polarities, over diagrams
    /// built with nothing but the public API.
    ///
    /// Eight same-side arcs is the threshold, so the domain needs sixteen
    /// points. One generator has a single cup; composition accumulates them, and
    /// `e_0 ; e_2 ; … ; e_14` in Hom(16, 16) cups every domain point in pairs and
    /// caps every codomain point in pairs — eight arcs a side, exactly at the
    /// threshold.
    ///
    /// Getting `non_crossing` to run at all takes one more step: `set_is_tl`
    /// early-returns while `is_def_tl` is true, and that product's flag is true.
    /// `s_0 ; s_0` is the identity diagram, so composing with it leaves the
    /// geometry bit-identical while `&&`-ing the flag down to false.
    ///
    /// The three cases put the two arms through all four of their outcomes:
    /// the domain arm finds a crossing (`domain_crossing`) and does not
    /// (`flag_cleared`, `codomain_crossing`); the codomain arm finds one
    /// (`codomain_crossing`) and does not (`flag_cleared`).
    #[test]
    fn non_crossing_parallel_arms_public_api() {
        use catgraph::category::Composable;
        let n = 16;
        let e = BrauerMorphism::<i64>::temperley_lieb_gens(n);
        let s = BrauerMorphism::<i64>::symmetric_alg_gens(n);
        let mut product = e[0].clone();
        for k in 1..8 {
            product = product
                .compose(&e[2 * k])
                .expect("e_0 ; e_2 ; … ; e_14 in Hom(16, 16)");
        }
        assert!(
            product.is_def_tl,
            "a product of TL generators is flagged TL by construction, which is \
             why set_is_tl needs the flag knocked down before it will compute"
        );

        let flag_cleared = product
            .compose(&s[0])
            .and_then(|z| z.compose(&s[0]))
            .expect("product ; s_0 ; s_0");
        let domain_crossing = s[1].compose(&product).expect("s_1 ; product");
        let codomain_crossing = product.compose(&s[1]).expect("product ; s_1");

        assert_eq!(
            terms(&flag_cleared),
            terms(&product),
            "s_0 ; s_0 is the identity, so this must be the same diagram as the product"
        );

        for (what, morphism, expected) in [
            ("product ; s_0 ; s_0", &flag_cleared, true),
            ("s_1 ; product", &domain_crossing, false),
            ("product ; s_1", &codomain_crossing, false),
        ] {
            let mut subject = morphism.clone();
            assert!(
                !subject.is_def_tl,
                "{what}: composing with a symmetric generator must clear the flag, \
                 or set_is_tl early-returns and non_crossing never runs"
            );
            for (_, pairs) in terms(&subject) {
                assert_parallel_arms_dispatch(&pairs, subject.source, what);
                assert_eq!(
                    boundary_walk_planar(&pairs, subject.source, subject.target),
                    expected,
                    "{what}: the boundary walk disagrees with the case's stated planarity"
                );
            }
            subject.set_is_tl();
            assert_eq!(
                subject.is_def_tl, expected,
                "{what}: set_is_tl computed {} where the geometry is planar = {expected}",
                subject.is_def_tl
            );
        }
    }

    /// A matching of `points`, drawn by repeatedly pairing two of them at
    /// random. Planar only by accident.
    fn seeded_matching_on(points: &[usize], rng: &mut Lcg, out: &mut Vec<Pair>) {
        let mut pool: Vec<usize> = points.to_vec();
        while !pool.is_empty() {
            let a = pool.swap_remove(rng.next_usize(0, pool.len() - 1));
            let b = pool.swap_remove(rng.next_usize(0, pool.len() - 1));
            out.push(Pair::sorted(a, b));
        }
    }

    /// A planar matching of `points`, which must be given in boundary order —
    /// either direction, since reversing a boundary preserves planarity. Pair
    /// the first point with one at an odd offset, then recurse on the inside and
    /// the outside: the standard planar decomposition.
    fn seeded_planar_matching_on(points: &[usize], rng: &mut Lcg, out: &mut Vec<Pair>) {
        if points.is_empty() {
            return;
        }
        let partner_slot = 2 * rng.next_usize(0, points.len() / 2 - 1) + 1;
        out.push(Pair::sorted(points[0], points[partner_slot]));
        seeded_planar_matching_on(&points[1..partner_slot], rng, out);
        seeded_planar_matching_on(&points[partner_slot + 1..], rng, out);
    }

    /// Seeded diagrams that keep both parallel arms dispatching, checked
    /// against the boundary walk.
    ///
    /// Reaching the arm takes eight arcs on a side. At `source = target = 16`
    /// that consumes the domain entirely, so those diagrams have no
    /// through-lines and exercise the two combination arms alone. At
    /// `source = target = 17` one domain point and one codomain point are left
    /// over and pair with each other, so the *blocking* branch runs under the
    /// parallel arms too — emptying either side's blocking set reddens these
    /// fixtures. The monotonicity branch runs and cannot discriminate: one
    /// through-line is always sorted, and reaching two while keeping eight
    /// same-side arcs per side needs `source = target >= 18`. Each side is
    /// drawn either planar by construction or unconstrained, independently of
    /// the other.
    #[test]
    fn non_crossing_parallel_arms_seeded() {
        let mut rng = Lcg::new(0x0002_945e_ed00_0001);
        for _ in 0..64 {
            for (domain_planar, codomain_planar) in
                [(false, false), (false, true), (true, false), (true, true)]
            {
                for (source, target) in [(16usize, 16usize), (17, 17)] {
                    let mut pairs: Vec<Pair> = Vec::new();
                    let mut domain_points: Vec<usize> = (0..source).collect();
                    let mut codomain_points: Vec<usize> = (source..source + target).collect();
                    if source % 2 == 1 {
                        let d = domain_points.remove(rng.next_usize(0, domain_points.len() - 1));
                        let c =
                            codomain_points.remove(rng.next_usize(0, codomain_points.len() - 1));
                        pairs.push(Pair::sorted(d, c));
                    }
                    for (planar, block) in [
                        (domain_planar, &domain_points),
                        (codomain_planar, &codomain_points),
                    ] {
                        if planar {
                            seeded_planar_matching_on(block, &mut rng, &mut pairs);
                        } else {
                            seeded_matching_on(block, &mut rng, &mut pairs);
                        }
                    }
                    let matching = PerfectMatching::from(pairs);
                    let what = format!(
                        "Hom({source}, {target}) with domain_planar={domain_planar} \
                         codomain_planar={codomain_planar}"
                    );
                    assert_parallel_arms_dispatch(&matching.pairs, source, &what);
                    let got = matching.non_crossing(source, target);
                    let want = boundary_walk_planar(&matching.pairs, source, target);
                    if domain_planar && codomain_planar && source % 2 == 0 {
                        // Two planar blocks occupy disjoint stretches of the
                        // boundary and there is no through-line to bridge them,
                        // so nothing can interleave. Pins the planar generator,
                        // and keeps a planar sample in the corpus.
                        assert!(want, "must be planar by construction — {what}");
                    }
                    assert_eq!(
                        got, want,
                        "non_crossing said {got} and the boundary walk said {want} \
                         for {:?} — {what}",
                        matching.pairs
                    );
                }
            }
        }
    }

    /// The pair set of `a ⊗ b`, read off the side-by-side placement.
    ///
    /// Tensoring draws `a` to the left of `b`, so the result's domain is `a`'s
    /// domain block followed by `b`'s and its codomain is `a`'s codomain block
    /// followed by `b`'s. On the flat point ids `0 .. n1 + n2 + m1 + m2` the two
    /// factors therefore embed as
    ///
    /// * `a`: a domain point `v` stays put; a codomain point (`v >= n1`) moves
    ///   past `b`'s domain block, to `v + n2`;
    /// * `b`: a domain point moves past `a`'s domain block, to `v + n1`; a
    ///   codomain point (`v >= n2`) moves past `a`'s domain *and* codomain
    ///   blocks, to `v + n1 + m1`.
    ///
    /// `monoidal` arrives at the same place by a different route — it shifts
    /// `a` once and `b` twice, the second time against the *new* domain width.
    fn tensor_reference(a: &[Pair], n1: usize, m1: usize, b: &[Pair], n2: usize) -> Vec<Pair> {
        let mut out: Vec<Pair> = a
            .iter()
            .map(|p| p.map(|v| if v < n1 { v } else { v + n2 }))
            .chain(
                b.iter()
                    .map(|p| p.map(|v| if v < n2 { v + n1 } else { v + n1 + m1 })),
            )
            .map(|p| p.sort())
            .collect();
        out.sort();
        out
    }

    /// `monoidal` against the side-by-side placement, on the pair set rather
    /// than on the arities.
    ///
    /// The arity assertions in `tests/temperley_lieb.rs::monoidal_tensor` cannot
    /// see a mislabelling: a shift that destroys the matching trips
    /// `PerfectMatching::from_iter`'s bijection assertion and the test panics
    /// before reaching them, and a shift that preserves it leaves the arities
    /// alone. This asserts the labels.
    #[test]
    fn monoidal_pair_set_at_nonzero_shift() {
        use catgraph::{
            category::{Composable, HasIdentity},
            monoidal::Monoidal,
        };
        use std::collections::BTreeSet;

        // Hand-derived from the placement, before running anything: e_0 in
        // Hom(3, 3) is (0,1) (2,5) (3,4) and id_2 in Hom(2, 2) is (0,2) (1,3).
        // Putting id_2 to the right of e_0 gives Hom(5, 5), whose domain block
        // is 0..5 and codomain block 5..10. e_0's domain points 0, 1, 2 stay;
        // its codomain points 3, 4, 5 shift past id_2's two domain points to
        // 5, 6, 7. id_2's domain points 0, 1 shift past e_0's three domain
        // points to 3, 4; its codomain points 2, 3 shift past e_0's three
        // domain and three codomain points to 8, 9. So (0,1) stays, (2,5) →
        // (2,7), (3,4) → (5,6), (0,2) → (3,8), (1,3) → (4,9).
        let mut tensored = BrauerMorphism::<i64>::temperley_lieb_gens(3)[0].clone();
        tensored.monoidal(BrauerMorphism::<i64>::identity(&2));
        assert_eq!(tensored.domain(), 5);
        assert_eq!(tensored.codomain(), 5);
        assert_eq!(
            terms(&tensored),
            vec![(
                0,
                vec![Pair(0, 1), Pair(2, 7), Pair(3, 8), Pair(4, 9), Pair(5, 6)]
            )],
            "e_0 (3) ⊗ id_2 should place id_2's four points at 3, 4, 8, 9"
        );

        let tl2 = BrauerMorphism::<i64>::temperley_lieb_gens(2);
        let factors: Vec<(&str, BrauerMorphism<i64>)> = vec![
            ("id_1", BrauerMorphism::identity(&1)),
            ("id_3", BrauerMorphism::identity(&3)),
            (
                "e_0 of 3",
                BrauerMorphism::temperley_lieb_gens(3)[0].clone(),
            ),
            (
                "e_1 of 4",
                BrauerMorphism::temperley_lieb_gens(4)[1].clone(),
            ),
            ("s_0 of 3", BrauerMorphism::symmetric_alg_gens(3)[0].clone()),
            ("s_2 of 4", BrauerMorphism::symmetric_alg_gens(4)[2].clone()),
            (
                "e_0 e_0 of 2",
                tl2[0].compose(&tl2[0]).expect("e_0 ; e_0 in Hom(2, 2)"),
            ),
            (
                "delta + delta^2",
                BrauerMorphism::delta_polynomial(&[0, 1, 1]),
            ),
        ];
        for (a_name, a) in &factors {
            for (b_name, b) in &factors {
                let mut got = a.clone();
                got.monoidal(b.clone());
                let what = format!("{a_name} ⊗ {b_name}");
                assert_eq!(got.domain(), a.domain() + b.domain(), "{what}: domain");
                assert_eq!(
                    got.codomain(),
                    a.codomain() + b.codomain(),
                    "{what}: codomain"
                );
                // Side by side introduces no closed loop, so the delta powers add.
                let mut want: BTreeSet<(usize, Vec<Pair>)> = BTreeSet::new();
                for (a_delta, a_pairs) in terms(a) {
                    for (b_delta, b_pairs) in terms(b) {
                        want.insert((
                            a_delta + b_delta,
                            tensor_reference(
                                &a_pairs,
                                a.domain(),
                                a.codomain(),
                                &b_pairs,
                                b.domain(),
                            ),
                        ));
                    }
                }
                // A basis element is a set: two `(delta power, matching)` keys
                // that coincide sum their coefficients and appear once.
                let have: BTreeSet<(usize, Vec<Pair>)> = terms(&got).into_iter().collect();
                assert_eq!(have, want, "{what}: basis");
            }
        }
    }
}

/// Union-find resolution of the connectivity core of
/// `<ExtendedPerfectMatching as Mul>::mul`. Composition glues two Brauer diagrams
/// and reads off (a) the endpoint matching of the result and (b) the number of
/// closed loops (the δ-power increment) — both from the connected components of
/// the glued diagram graph.
///
/// Diagram arcs are undirected, which is exactly what a disjoint-set structure
/// models, so each arc is one `union` and connectivity needs no graph object at
/// all. The point set is the flat range `0 .. self_dom + self_cod + rhs_cod`:
/// `lhs`'s points keep their own ids, and `rhs`'s point `p` is `p + self_dom`,
/// which lands rhs's domain exactly on lhs's codomain — that offset *is* the
/// gluing. Components are labelled once (near-linear), then the matching falls
/// out of O(1) root comparison: two endpoints are matched iff they share a root.
mod connectivity {
    use super::{ExtendedPerfectMatching, Pair, PerfectMatching};
    use std::collections::HashSet;
    use union_find::{QuickUnionUf, UnionBySize, UnionFind};

    /// Resolve `lhs ∘ rhs`.
    ///
    /// Total: a perfect matching cannot fail to have components, so there is no
    /// error path to report.
    pub(super) fn resolve(
        lhs: &ExtendedPerfectMatching,
        rhs: &ExtendedPerfectMatching,
    ) -> ExtendedPerfectMatching {
        let (self_dom, self_cod, self_delta_pow) = (lhs.0.0, lhs.0.1, lhs.0.2);
        let (rhs_dom, rhs_cod, rhs_delta_pow) = (rhs.0.0, rhs.0.1, rhs.0.2);
        assert_eq!(rhs_dom, self_cod, "composition dom/cod mismatch");

        // Both diagrams are perfect matchings, so every point in the flat range is
        // covered by exactly one arc and no point is left unvisited.
        let points = self_dom + self_cod + rhs_cod;
        let mut uf: QuickUnionUf<UnionBySize> = QuickUnionUf::new(points);
        for &Pair(p, q) in &lhs.0.3.pairs {
            uf.union(p, q);
        }
        for &Pair(p, q) in &rhs.0.3.pairs {
            uf.union(p + self_dom, q + self_dom);
        }

        // One pass to label, so the matching search below is pure comparison.
        let root_of: Vec<usize> = (0..points).map(|i| uf.find(i)).collect();
        let components = root_of.iter().collect::<HashSet<_>>().len();

        // The composite's own boundary: lhs's domain, then rhs's codomain. The
        // glued interior (lhs's codomain) is skipped, which is the `+ self_cod`.
        let endpoints = self_dom + rhs_cod;
        let node_of = |i: usize| if i < self_dom { i } else { i + self_cod };

        let mut endpoints_done: HashSet<usize> = HashSet::with_capacity(endpoints);
        let mut final_matching: Vec<Pair> = Vec::with_capacity(endpoints / 2);
        for i in 0..endpoints {
            if endpoints_done.contains(&i) {
                continue;
            }
            let i_root = root_of[node_of(i)];
            for j in (i + 1)..endpoints {
                if root_of[node_of(j)] == i_root {
                    final_matching.push(Pair(i, j));
                    endpoints_done.insert(i);
                    endpoints_done.insert(j);
                    break;
                }
            }
        }

        // Components that did not consume a boundary pair are the closed loops.
        let new_delta_power = components + self_delta_pow + rhs_delta_pow - (endpoints / 2);
        ExtendedPerfectMatching((
            self_dom,
            rhs_cod,
            new_delta_power,
            PerfectMatching {
                pairs: final_matching,
            },
        ))
    }
}
