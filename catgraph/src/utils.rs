use {
    either::Either::{self, Left, Right},
    permutations::Permutation,
    std::{collections::HashSet, fmt::Debug},
};

pub fn is_unique<T: Eq + std::hash::Hash>(s: &[T]) -> bool {
    let mut uniq = HashSet::with_capacity(s.len());
    s.iter().all(|cur| uniq.insert(cur))
}

pub trait EitherExt<T, U> {
    fn bimap<V, W>(self, f1: impl Fn(T) -> V, f2: impl Fn(U) -> W) -> impl EitherExt<V, W>;
    fn join<V>(self, f1: impl Fn(T) -> V, f2: impl Fn(U) -> V) -> V;
}

impl<T, U> EitherExt<T, U> for Either<T, U> {
    #[allow(refining_impl_trait)]
    fn bimap<V, W>(self, f1: impl Fn(T) -> V, f2: impl Fn(U) -> W) -> Either<V, W> {
        match self {
            Left(t) => Left(f1(t)),
            Right(u) => Right(f2(u)),
        }
    }

    fn join<V>(self, f1: impl Fn(T) -> V, f2: impl Fn(U) -> V) -> V {
        match self {
            Left(t) => f1(t),
            Right(u) => f2(u),
        }
    }
}

pub fn represents_id(it: impl Iterator<Item = usize>) -> bool {
    (0..).zip(it).all(|(l, r)| l == r)
}

pub fn remove_multiple<T>(me: &mut Vec<T>, mut to_remove: Vec<usize>) {
    to_remove.sort_unstable();
    to_remove.reverse();
    for r in to_remove {
        me.remove(r);
    }
}

/// Compute the permutation that reorders `side_1` to match `side_2`.
///
/// # Errors
///
/// Returns `Err` if the slices differ in length, contain mismatched elements,
/// or have duplicates that prevent a unique permutation.
pub fn necessary_permutation<T: Eq>(side_1: &[T], side_2: &[T]) -> Result<Permutation, String> {
    let n1 = side_1.len();
    let n2 = side_2.len();
    if n1 != n2 {
        return Err(format!(
            "No permutation can take side 1 to side 2 because the lengths {n1} and {n2} don't match"
        ));
    }
    let mut trial_perm = Vec::<usize>::with_capacity(n1);
    for cur in side_1 {
        let Some(idx) = side_2.iter().position(|t| *t == *cur) else {
            return Err("No permutation can take side 1 to side 2 \
            because an item in side 1 was not in side 2"
                .to_string());
        };
        trial_perm.push(idx);
    }
    Permutation::try_from(trial_perm)
        .map_err(|_| {
            "No permutation can take side 1 to side 2\n\
            because there were multiple in side 1 that were equal \
            and so mapped to the same index in side 2"
                .to_string()
        })
        .map(|e| e.inv())
}

fn perm_decompose(p: &Permutation) -> Vec<(usize, usize)> {
    if p.len() <= 1 {
        return vec![];
    }
    let mut seen = vec![false; p.len()];
    let mut answer = Vec::with_capacity(p.len() - 1);
    for i in 0..p.len() {
        if !seen[i] {
            seen[i] = true;
            let mut j = p.apply(i);
            let mut j_before = i;
            while j != i {
                answer.push((j_before, j));
                seen[j] = true;
                j_before = j;
                j = p.apply(j_before);
            }
        }
    }
    answer
}

pub fn in_place_permute<T>(me: &mut [T], p: &Permutation) {
    let transpositions = perm_decompose(p);
    for (p, q) in transpositions {
        me.swap(p, q);
    }
}

/// Build a random permutation on `n` points by composing `max_depth`
/// random transpositions.
///
/// # Panics
///
/// Panics if `n == 0` (empty uniform distribution).
#[cfg(test)]
pub fn rand_perm(n: usize, max_depth: usize, rng: &mut impl rand::Rng) -> Permutation {
    use rand::{distr::Uniform, prelude::Distribution};
    let between = Uniform::try_from(0..n).unwrap();
    let mut answer = Permutation::identity(n);
    for _ in 0..max_depth {
        let i = between.sample(rng);
        let j = between.sample(rng);
        answer = answer * Permutation::transposition(n, i, j);
    }
    answer
}

pub trait ResultExt<T, E> {
    /// Combine two `Result` values into a tuple, short-circuiting on the first `Err`.
    ///
    /// # Errors
    ///
    /// Returns the first `Err` encountered from either `self` or `other`.
    fn zip<U>(self, other: Result<U, E>) -> Result<(T, U), E>;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn zip<U>(self, other: Result<U, E>) -> Result<(T, U), E> {
        match (self, other) {
            (Ok(a), Ok(b)) => Ok((a, b)),
            (Err(e), _) | (_, Err(e)) => Err(e),
        }
    }
}

/// Check that two label sequences match element-wise.
///
/// # Errors
///
/// Returns `Err` if the iterators differ in length or any pair of labels is unequal.
pub fn same_labels_check<
    Lambda: Eq + Debug,
    L: ExactSizeIterator + Iterator<Item = Lambda>,
    R: ExactSizeIterator + Iterator<Item = Lambda>,
>(
    l: L,
    r: R,
) -> Result<(), String> {
    if l.len() != r.len() {
        return Err("Mismatch in cardinalities of common interface".to_string());
    }
    let Some((w1, w2)) = l.zip(r).find(|(a, b)| a != b) else {
        return Ok(());
    };
    Err(format!(
        "Mismatch in labels of common interface. At some index there was {w1:?} vs {w2:?}"
    ))
}

/// Assert that two `Result`s match both by equality and by an auxiliary
/// predicate. Used in unit tests within this crate and in downstream crates
/// to compare morphisms that lack `PartialEq` for the full structure.
///
/// # Panics
///
/// Panics if either `observed` or `expected` is `Err`, or if the auxiliary
/// predicate fails, or if the unwrapped `Ok` values differ.
pub fn test_asserter<T, U, F>(
    observed: Result<T, U>,
    expected: Result<T, U>,
    aux_test: F,
    equation_str: &str,
) where
    F: Fn(&T, &T) -> bool,
    T: Debug + PartialEq,
{
    let Ok((real_observed, real_expected)) = observed.zip(expected) else {
        panic!("Error on one of observed/expected sides when checking {equation_str:?}")
    };

    assert!(aux_test(&real_observed, &real_expected));
    assert!(
        real_observed == real_expected,
        "{real_observed:?} vs {real_expected:?} when checking {equation_str:?}"
    );
}

#[macro_export]
macro_rules! assert_ok {
    ( $x:expr ) => {
        match $x {
            std::result::Result::Ok(v) => v,
            std::result::Result::Err(e) => {
                panic!("Error calling {}: {:?}", stringify!($x), e);
            }
        }
    };
}

#[macro_export]
macro_rules! assert_err {
    ( $x:expr ) => {
        match $x {
            std::result::Result::Err(v) => v,
            std::result::Result::Ok(e) => {
                panic!("No error calling {}: {:?}", stringify!($x), e);
            }
        }
    };
}

#[cfg(test)]
mod test {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn nec_permutation() {
        use crate::utils::{necessary_permutation, rand_perm};
        use rand::{distr::Uniform, prelude::Distribution};
        let n_max = 10;
        let between = Uniform::<usize>::try_from(2usize..n_max).unwrap();
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..10 {
            let n = between.sample(&mut rng);
            let set = (0..n).map(|i| format!("{i}")).collect::<Vec<String>>();
            let p1 = rand_perm(n, n * n / 4, &mut rng);
            let permuted_set = p1.permute(&set);
            let found_perm = necessary_permutation(&set, &permuted_set);
            assert_eq!(found_perm, Ok(p1));
        }
    }

    #[test]
    fn perm_decompose() {
        use crate::utils::{perm_decompose, rand_perm};
        use permutations::Permutation;
        use rand::{distr::Uniform, prelude::Distribution};
        let n_max = 10;

        let between = Uniform::<usize>::try_from(2usize..n_max).unwrap();
        let mut rng = StdRng::seed_from_u64(123);
        for _ in 0..10 {
            let n = between.sample(&mut rng);
            let p1 = rand_perm(n, n * n / 4, &mut rng);
            let cycle_prod = perm_decompose(&p1);
            let obs_p1 = cycle_prod
                .iter()
                .fold(Permutation::identity(n), |acc, (p, q)| {
                    Permutation::transposition(n, *p, *q) * acc
                });
            assert_eq!(p1, obs_p1);
        }
    }

    #[test]
    fn in_place_permuting() {
        use crate::utils::{in_place_permute, rand_perm};
        use rand::{distr::Uniform, prelude::Distribution};
        let n_max = 10;
        let between = Uniform::<usize>::try_from(2usize..n_max).unwrap();
        let mut rng = StdRng::seed_from_u64(456);
        for _ in 0..10 {
            let n = between.sample(&mut rng);
            let mut set = (0..n).map(|i| format!("{i}")).collect::<Vec<String>>();
            let p1 = rand_perm(n, n * n / 4, &mut rng);
            in_place_permute(&mut set, &p1);
            for (idx, cur) in set.iter().enumerate() {
                assert_eq!(*cur, format!("{}", p1.apply(idx)));
            }
            in_place_permute(&mut set, &p1.inv());
            for (idx, cur) in set.iter().enumerate() {
                assert_eq!(*cur, format!("{idx}"));
            }
        }
    }
}
