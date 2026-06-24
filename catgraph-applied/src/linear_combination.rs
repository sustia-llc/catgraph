//! Formal linear combinations over a coefficient ring.
//!
//! [`LinearCombination<Coeffs, Target>`] represents an element of the free module R\[T\]:
//! a sparse map from basis elements `T` to coefficients in a ring `R`. Supports:
//!
//! - **Additive structure**: `Add`, `Sub`, `Neg` (pointwise on coefficients)
//! - **Scalar multiplication**: `Mul<Coeffs>`, `MulAssign<Coeffs>`
//! - **Convolution**: `Mul<Self>` when `Target: Mul<Output = Target>` — the product
//!   of two formal sums, distributing over basis multiplication (parallelized
//!   via rayon above a fixed `PARALLEL_MUL_THRESHOLD` (32 terms))
//! - **Generalized convolution**: [`linear_combine`](LinearCombination::linear_combine)
//!   over heterogeneous basis types
//! - **Functorial maps**: [`inj_linearly_extend`](LinearCombination::inj_linearly_extend)
//!   and [`linearly_extend`](LinearCombination::linearly_extend) for basis change
//!
//! Used internally by [`BrauerMorphism`](crate::temperley_lieb::BrauerMorphism) for
//! Brauer algebra arithmetic and available as a standalone module for any algebraic
//! computation over formal sums.
//!
//! See also `examples/linear_combination.rs`.

use {
    // `num::{One, Zero}` is intentional here: `LinearCombination` is the private
    // coefficient substrate for `temperley_lieb`, whose `num::Complex<i32>`
    // coefficients cannot implement `deep_causality_num::{One, Zero}` (DC's
    // `Complex` requires a `RealField`, which `i32` is not). Kept on the `num`
    // side of the Phase-2 re-substrate; on the P2 grep-guard exception list.
    num::{One, Zero},
    std::{
        collections::HashMap,
        fmt::Debug,
        hash::Hash,
        ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
    },
};

#[cfg(feature = "parallel")]
use rayon_cond::CondIterator;

/// Threshold gating the parallel arm of [`CondIterator`] in `Mul::mul` and
/// [`LinearCombination::linear_combine`] when the `parallel` feature is
/// enabled. Below this size the serial iterator is faster due to rayon
/// worker-setup overhead.
// HashMap `into_par_iter()` is not `IndexedParallelIterator`, so the adaptive
// `with_min_len` pattern (used in catgraph core) doesn't apply here —
// `rayon_cond::CondIterator` provides the compile/runtime parallel↔serial
// toggle instead. See `~/.claude/summaries/rayon-summary-0.md` for the
// rustworkx-core precedent; pattern established in Phase W.0 (2026-04-19).
#[cfg(feature = "parallel")]
const PARALLEL_MUL_THRESHOLD: usize = 32;

/// A formal linear combination: a sparse map from basis elements to coefficients.
///
/// Represents `Σ c_i · t_i` where `c_i: Coeffs` and `t_i: Target`. Zero-coefficient
/// terms may be present until [`simplify`](Self::simplify) is called.
#[repr(transparent)]
#[derive(PartialEq, Eq, Debug, Default, Clone)]
pub struct LinearCombination<Coeffs, Target: Eq + Hash>(HashMap<Target, Coeffs>);

impl<Coeffs, Target: Eq + Hash> FromIterator<(Target, Coeffs)>
    for LinearCombination<Coeffs, Target>
{
    fn from_iter<T: IntoIterator<Item = (Target, Coeffs)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<Coeffs, Target: Eq + Hash> Add for LinearCombination<Coeffs, Target>
where
    Coeffs: Copy + AddAssign,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        let mut new_map = self.0;
        for (k, v) in rhs.0 {
            new_map
                .entry(k)
                .and_modify(|self_val: &mut Coeffs| *self_val += v)
                .or_insert(v);
        }
        Self(new_map)
    }
}

impl<Coeffs, Target: Eq + Hash> AddAssign for LinearCombination<Coeffs, Target>
where
    Coeffs: Copy + AddAssign,
{
    fn add_assign(&mut self, rhs: Self) {
        for (k, v) in rhs.0 {
            self.0
                .entry(k)
                .and_modify(|self_val: &mut Coeffs| *self_val += v)
                .or_insert(v);
        }
    }
}

impl<Coeffs, Target: Eq + Hash> Sub for LinearCombination<Coeffs, Target>
where
    Coeffs: Copy + SubAssign + Neg<Output = Coeffs>,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        let mut new_map = self.0;
        for (k, v) in rhs.0 {
            new_map
                .entry(k)
                .and_modify(|self_val: &mut Coeffs| *self_val -= v)
                .or_insert(-v);
        }
        Self(new_map)
    }
}

impl<Coeffs, Target: Eq + Hash> Neg for LinearCombination<Coeffs, Target>
where
    Coeffs: Copy + Neg<Output = Coeffs>,
{
    type Output = Self;

    fn neg(self) -> Self {
        let mut new_map = self.0;
        for val in new_map.values_mut() {
            *val = -*val;
        }
        Self(new_map)
    }
}

impl<Coeffs, Target: Eq + Hash> Mul<Coeffs> for LinearCombination<Coeffs, Target>
where
    Coeffs: Copy + MulAssign,
{
    type Output = Self;

    fn mul(self, rhs: Coeffs) -> Self {
        let mut new_map = self.0;
        for val in new_map.values_mut() {
            *val *= rhs;
        }
        Self(new_map)
    }
}

impl<Coeffs, Target: Eq + Hash + Clone> Mul for LinearCombination<Coeffs, Target>
where
    Coeffs: Copy + AddAssign + Mul<Output = Coeffs> + MulAssign + One + Send + Sync,
    Target: Mul<Output = Target> + Send + Sync,
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        let rhs_vec: Vec<_> = rhs.0.iter().collect();
        let process = |(k1, c_k1): (Target, Coeffs)| -> Self {
            let mut partial = Self(HashMap::new());
            for (k2, c_k2) in &rhs_vec {
                partial += Self::singleton(k1.clone() * (*k2).clone()) * (c_k1 * (**c_k2));
            }
            partial
        };

        #[cfg(feature = "parallel")]
        let partial_results: Vec<Self> = {
            let enable_parallel =
                self.0.len() >= PARALLEL_MUL_THRESHOLD && rhs.0.len() >= PARALLEL_MUL_THRESHOLD;
            CondIterator::new(self.0, enable_parallel)
                .map(process)
                .collect()
        };
        #[cfg(not(feature = "parallel"))]
        let partial_results: Vec<Self> = self.0.into_iter().map(process).collect();

        partial_results
            .into_iter()
            .fold(Self(HashMap::new()), |acc, x| acc + x)
    }
}

/*
This would be a conflicting implementation of Mul for two LinearCombination's
We like to choose the Target type so that it is a nice basis
which when multiplied doesn't produce a complicated linear combination
but instead just some Target again
For that reason, we choose the simpler implementation of Mul
instead of this more general one
*/
/*
impl<Coeffs: Copy, Target: Eq + Hash + Clone> Mul for LinearCombination<Coeffs, Target>
where
    Coeffs: AddAssign + Mul<Output = Coeffs> + MulAssign + One,
    Target: Mul<Output = LinearCombination<Coeffs,Target>>,
{
    /*
    multiply two formal sums provided the target has a multiplication operation
    that produces formal sums (usually singletons but does not have to be)
    */
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        let mut ret_val = Self(HashMap::new());
        for (k1, c_k1) in self.0 {
            for (k2, c_k2) in &rhs.0 {
                ret_val += (k1.clone() * k2.clone()) * (c_k1 * (*c_k2));
            }
        }
        ret_val
    }
}
*/

impl<Coeffs, Target: Eq + Hash> MulAssign<Coeffs> for LinearCombination<Coeffs, Target>
where
    Coeffs: Copy + MulAssign,
{
    fn mul_assign(&mut self, rhs: Coeffs) {
        for val in self.0.values_mut() {
            *val *= rhs;
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
impl<Coeffs, Target: Eq + Hash> LinearCombination<Coeffs, Target> {
    /// Generalized convolution over heterogeneous basis types.
    ///
    /// Combines `self ∈ R[T]` with `rhs ∈ R[U]` to produce an element of `R[V]`,
    /// using `combiner: T × U → V` as the "multiplication" on basis elements.
    /// Parallelized via rayon when both operands exceed `PARALLEL_MUL_THRESHOLD` (32 terms).
    pub fn linear_combine<U, V, F>(
        &self,
        rhs: LinearCombination<Coeffs, U>,
        combiner: F,
    ) -> LinearCombination<Coeffs, V>
    where
        Coeffs: Copy + AddAssign + Mul<Output = Coeffs> + MulAssign + One + Send + Sync,
        Target: Eq + Hash + Clone + Send + Sync,
        U: Eq + Hash + Clone + Send + Sync,
        V: Eq + Hash + Send,
        F: Fn(Target, U) -> V + Sync,
    {
        let self_vec: Vec<_> = self.0.iter().collect();
        let rhs_vec: Vec<_> = rhs.0.iter().collect();
        let process = |(k1, c_k1): (&Target, &Coeffs)| -> LinearCombination<Coeffs, V> {
            let mut partial = LinearCombination(HashMap::new());
            for (k2, c_k2) in &rhs_vec {
                partial += LinearCombination::singleton(combiner(k1.clone(), (*k2).clone()))
                    * (*c_k1 * (**c_k2));
            }
            partial
        };

        #[cfg(feature = "parallel")]
        let partial_results: Vec<LinearCombination<Coeffs, V>> = {
            let enable_parallel =
                self.0.len() >= PARALLEL_MUL_THRESHOLD && rhs.0.len() >= PARALLEL_MUL_THRESHOLD;
            CondIterator::new(self_vec, enable_parallel)
                .map(process)
                .collect()
        };
        #[cfg(not(feature = "parallel"))]
        let partial_results: Vec<LinearCombination<Coeffs, V>> =
            self_vec.into_iter().map(process).collect();

        partial_results
            .into_iter()
            .fold(LinearCombination(HashMap::new()), |acc, x| acc + x)
    }

    /// Apply a ring endomorphism to every coefficient (e.g. conjugation, negation).
    pub fn change_coeffs<F>(&mut self, coeff_changer: F)
    where
        Coeffs: Copy,
        F: Fn(Coeffs) -> Coeffs,
    {
        for val in self.0.values_mut() {
            *val = coeff_changer(*val);
        }
    }

    /// True if every basis element satisfies the predicate (ignoring coefficients).
    pub fn all_terms_satisfy<F>(&self, term_predicate: F) -> bool
    where
        F: Fn(&Target) -> bool,
    {
        self.0.keys().all(term_predicate)
    }
}

impl<Coeffs, Target: Eq + Hash> LinearCombination<Coeffs, Target>
where
    Coeffs: One,
{
    /// A single basis element with coefficient one: the Dirac delta `1 · t`.
    pub fn singleton(t: Target) -> Self {
        Self([(t, <_>::one())].into())
    }
}

impl<Coeffs: Zero, Target: Eq + Hash> LinearCombination<Coeffs, Target> {
    /// Remove all terms with zero coefficient.
    pub fn simplify(&mut self) {
        self.0.retain(|_, v| !v.is_zero());
    }
}

impl<Coeffs, Target: Clone + Eq + Hash> LinearCombination<Coeffs, Target> {
    /// Extend an injective map `T1 → T2` to a linear map `R[T1] → R[T2]`.
    ///
    /// Each basis element is mapped through `injection` and its coefficient
    /// is preserved. Panics if the function is not actually injective (i.e. two
    /// distinct basis elements map to the same target).
    ///
    /// # Panics
    ///
    /// Panics if `injection` is not actually injective — i.e. maps distinct basis elements
    /// to the same target.
    pub fn inj_linearly_extend<Target2: Eq + Hash, F>(
        &self,
        injection: F,
    ) -> LinearCombination<Coeffs, Target2>
    where
        F: Fn(Target) -> Target2,
        Coeffs: Copy,
    {
        let mut new_map = HashMap::with_capacity(self.0.len());
        for (k, v) in &self.0 {
            let new_key = injection(k.clone());
            let old_val = new_map.insert(new_key, *v);
            assert_eq!(
                old_val.map(|_| 0),
                None,
                "The function called injection should have been injective"
            );
        }
        LinearCombination(new_map)
    }

    /// Extend a (possibly non-injective) map `T1 → T2` to a linear map `R[T1] → R[T2]`.
    ///
    /// When distinct basis elements collide under `f`, their coefficients are summed.
    /// This is the free module functorial action for arbitrary set maps.
    pub fn linearly_extend<Target2: Eq + Hash, F>(&self, f: F) -> LinearCombination<Coeffs, Target2>
    where
        F: Fn(Target) -> Target2,
        Coeffs: Copy + Add<Output = Coeffs>,
    {
        let mut new_map = HashMap::with_capacity(self.0.len());
        for (k, v) in &self.0 {
            let new_key = f(k.clone());
            new_map
                .entry(new_key)
                .and_modify(|old| *old = *old + *v)
                .or_insert(*v);
        }
        LinearCombination(new_map)
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn adding() {
        use super::LinearCombination;
        let one_a = LinearCombination::singleton("a".to_string());
        let two_b = LinearCombination::singleton("b".to_string()) * 2;
        let one_a_plus_two_b = one_a.clone() + two_b.clone();
        let two_b_plus_one_a = two_b + one_a;
        assert_eq!(one_a_plus_two_b, two_b_plus_one_a);
        let mut zeroed = one_a_plus_two_b - two_b_plus_one_a;
        zeroed.simplify();
        assert!(zeroed.0.is_empty());
    }

    /// Multiplication of two linear combinations where the target type
    /// supports Mul (here we use i32 as both coefficient and target).
    /// (2*1 + 3*2) * (1*1 + 1*2) should yield:
    ///   2*1*1 + 2*1*2 + 3*2*1 + 3*2*2
    /// = 2*1   + 2*2   + 3*2   + 3*4
    /// = 2*1   + 5*2   + 3*4
    #[test]
    fn multiplication() {
        use super::LinearCombination;
        // lhs = 2*1 + 3*2  (coeff * target, both i64)
        let lhs: LinearCombination<i64, i64> =
            LinearCombination::singleton(1) * 2 + LinearCombination::singleton(2) * 3;
        // rhs = 1*1 + 1*2
        let rhs: LinearCombination<i64, i64> =
            LinearCombination::singleton(1) + LinearCombination::singleton(2);
        let product = lhs * rhs;
        // Expected: target 1 => 2*1=2, target 2 => 2*1+3*1=5, target 4 => 3*1=3
        assert_eq!(product.0.get(&1), Some(&2));
        assert_eq!(product.0.get(&2), Some(&5));
        assert_eq!(product.0.get(&4), Some(&3));
        assert_eq!(product.0.len(), 3);
    }

    /// `linear_combine` generalizes multiplication by taking a combiner
    /// function instead of requiring Mul on the target type.
    /// Combine string targets via concatenation.
    #[test]
    fn linear_combine() {
        use super::LinearCombination;
        // lhs = 2"x" + 3"y"
        let lhs: LinearCombination<i64, String> = LinearCombination::singleton("x".into()) * 2
            + LinearCombination::singleton("y".into()) * 3;
        // rhs = 1"a" + 4"b"
        let rhs: LinearCombination<i64, String> =
            LinearCombination::singleton("a".into()) + LinearCombination::singleton("b".into()) * 4;
        // combiner: concatenate the two strings
        let result = lhs.linear_combine(rhs, |s1, s2| format!("{s1}{s2}"));
        // Expected terms:
        //   "xa" => 2*1=2, "xb" => 2*4=8, "ya" => 3*1=3, "yb" => 3*4=12
        assert_eq!(result.0.get("xa"), Some(&2));
        assert_eq!(result.0.get("xb"), Some(&8));
        assert_eq!(result.0.get("ya"), Some(&3));
        assert_eq!(result.0.get("yb"), Some(&12));
        assert_eq!(result.0.len(), 4);
    }

    /// `linearly_extend` applies a (possibly non-injective) function
    /// T1 -> T2 and induces a map R[T1] -> R[T2], merging coefficients
    /// when different keys map to the same target.
    #[test]
    fn linearly_extend() {
        use super::LinearCombination;
        // lc = 2*1 + 3*2 + 5*3
        let lc: LinearCombination<i64, i64> = LinearCombination::singleton(1) * 2
            + LinearCombination::singleton(2) * 3
            + LinearCombination::singleton(3) * 5;
        // Map each target to its parity (mod 2). This is non-injective:
        //   1 -> 1, 2 -> 0, 3 -> 1
        // So target 1 gets coeff 2+5=7, target 0 gets coeff 3.
        let result = lc.linearly_extend(|x| x % 2);
        assert_eq!(result.0.get(&1), Some(&7));
        assert_eq!(result.0.get(&0), Some(&3));
        assert_eq!(result.0.len(), 2);
    }

    /// `inj_linearly_extend` applies an injective function T1 -> T2
    /// and induces a map R[T1] -> R[T2]. Panics if the function is
    /// not actually injective.
    #[test]
    fn inj_linearly_extend_ok() {
        use super::LinearCombination;
        // lc = 2*"a" + 5*"b"
        let lc: LinearCombination<i64, String> = LinearCombination::singleton("a".into()) * 2
            + LinearCombination::singleton("b".into()) * 5;
        // Injective map: prefix with "prefix_"
        let result = lc.inj_linearly_extend(|s| format!("prefix_{s}"));
        assert_eq!(result.0.get("prefix_a"), Some(&2));
        assert_eq!(result.0.get("prefix_b"), Some(&5));
        assert_eq!(result.0.len(), 2);
    }

    /// `inj_linearly_extend` should panic when the function is not injective.
    #[test]
    #[should_panic(expected = "injective")]
    fn inj_linearly_extend_panics_on_collision() {
        use super::LinearCombination;
        // lc = 1*"a" + 1*"b"
        let lc: LinearCombination<i64, String> =
            LinearCombination::singleton("a".into()) + LinearCombination::singleton("b".into());
        // Non-injective: maps everything to the same value
        let _ = lc.inj_linearly_extend(|_| "same".to_string());
    }

    /// `simplify` removes terms whose coefficient is zero.
    #[test]
    #[allow(clippy::erasing_op)] // intentional `* 0` exercises zero-coefficient simplify
    fn zero_coefficient_cleanup() {
        use super::LinearCombination;
        // lc = 5*"a" + 0*"b" + 3*"c"
        let mut lc: LinearCombination<i64, String> = LinearCombination::singleton("a".into()) * 5
            + LinearCombination::singleton("b".into()) * 0
            + LinearCombination::singleton("c".into()) * 3;
        // Before simplify, "b" is present with coefficient 0
        assert!(lc.0.contains_key("b"));
        lc.simplify();
        // After simplify, "b" is gone
        assert!(!lc.0.contains_key("b"));
        assert_eq!(lc.0.get("a"), Some(&5));
        assert_eq!(lc.0.get("c"), Some(&3));
        assert_eq!(lc.0.len(), 2);
    }

    /// Subtraction that produces zero coefficients, then simplify.
    #[test]
    fn subtraction_then_simplify() {
        use super::LinearCombination;
        let lc1: LinearCombination<i64, String> = LinearCombination::singleton("x".into()) * 7
            + LinearCombination::singleton("y".into()) * 3;
        let lc2: LinearCombination<i64, String> = LinearCombination::singleton("x".into()) * 7
            + LinearCombination::singleton("y".into()) * 1;
        let mut diff = lc1 - lc2;
        // "x" coefficient should be 0, "y" should be 2
        assert_eq!(diff.0.get("x"), Some(&0));
        diff.simplify();
        assert!(!diff.0.contains_key("x"));
        assert_eq!(diff.0.get("y"), Some(&2));
        assert_eq!(diff.0.len(), 1);
    }

    /// `change_coeffs` applies a function to every coefficient.
    #[test]
    fn change_coeffs() {
        use super::LinearCombination;
        let mut lc: LinearCombination<i64, String> = LinearCombination::singleton("a".into()) * 3
            + LinearCombination::singleton("b".into()) * 5;
        lc.change_coeffs(|c| c * c);
        assert_eq!(lc.0.get("a"), Some(&9));
        assert_eq!(lc.0.get("b"), Some(&25));
    }

    /// `all_terms_satisfy` checks a predicate on all target keys.
    #[test]
    fn all_terms_satisfy() {
        use super::LinearCombination;
        let lc: LinearCombination<i64, i64> = LinearCombination::singleton(2) * 1
            + LinearCombination::singleton(4) * 1
            + LinearCombination::singleton(6) * 1;
        assert!(lc.all_terms_satisfy(|t| t % 2 == 0));
        assert!(!lc.all_terms_satisfy(|t| *t > 3));
    }

    /// Negation flips the sign of every coefficient.
    #[test]
    fn negation() {
        use super::LinearCombination;
        let lc: LinearCombination<i64, String> = LinearCombination::singleton("a".into()) * 3
            + LinearCombination::singleton("b".into()) * -2;
        let neg = -lc;
        assert_eq!(neg.0.get("a"), Some(&-3));
        assert_eq!(neg.0.get("b"), Some(&2));
    }

    /// `MulAssign` scales all coefficients in place.
    #[test]
    fn mul_assign_scalar() {
        use super::LinearCombination;
        let mut lc: LinearCombination<i64, String> = LinearCombination::singleton("a".into()) * 3
            + LinearCombination::singleton("b".into()) * 5;
        lc *= 4;
        assert_eq!(lc.0.get("a"), Some(&12));
        assert_eq!(lc.0.get("b"), Some(&20));
    }

    /// `AddAssign` merges terms from another linear combination.
    #[test]
    fn add_assign() {
        use super::LinearCombination;
        let mut lc: LinearCombination<i64, String> = LinearCombination::singleton("a".into()) * 2;
        lc += LinearCombination::singleton("a".into()) * 3
            + LinearCombination::singleton("b".into()) * 7;
        assert_eq!(lc.0.get("a"), Some(&5));
        assert_eq!(lc.0.get("b"), Some(&7));
    }

    /// `FromIterator` collects (Target, Coeffs) pairs into a `LinearCombination`.
    #[test]
    fn from_iterator() {
        use super::LinearCombination;
        let lc: LinearCombination<i64, String> = vec![("a".to_string(), 2), ("b".to_string(), 5)]
            .into_iter()
            .collect();
        assert_eq!(lc.0.get("a"), Some(&2));
        assert_eq!(lc.0.get("b"), Some(&5));
        assert_eq!(lc.0.len(), 2);
    }
}
