//! Exhaustive enumeration of `Sₙ`, in two views.
//!
//! - [`all_perm_indices`] yields raw `Vec<usize>` one-line notations, which
//!   sort and dedup (`permutations::Permutation` is neither `Ord` nor `Hash`);
//! - [`all_perms`] yields [`Permutation`] values.
//!
//! The two views are index-aligned: `all_perms(n)[k]` is the permutation whose
//! one-line notation is `all_perm_indices(n)[k]`.

use permutations::Permutation;

/// Every permutation of `0..n` in one-line notation, each exactly once.
///
/// Prefix-swap recursion yielding `n!` vectors of length `n`, so for small `n`.
///
/// # Examples
///
/// ```
/// use catgraph_testutil::all_perm_indices;
///
/// let ps = all_perm_indices(3);
/// assert_eq!(ps.len(), 6);
/// assert!(ps.contains(&vec![0, 1, 2]));
/// assert!(ps.contains(&vec![2, 0, 1]));
/// ```
#[must_use]
pub fn all_perm_indices(n: usize) -> Vec<Vec<usize>> {
    fn go(cur: &mut Vec<usize>, k: usize, out: &mut Vec<Vec<usize>>) {
        if k == cur.len() {
            out.push(cur.clone());
            return;
        }
        for i in k..cur.len() {
            cur.swap(k, i);
            go(cur, k + 1, out);
            cur.swap(k, i);
        }
    }
    let mut cur: Vec<usize> = (0..n).collect();
    let mut out = Vec::new();
    go(&mut cur, 0, &mut out);
    out
}

/// Every permutation of `0..n` as a [`Permutation`], each exactly once.
///
/// [`all_perm_indices`] with each one-line notation converted.
///
/// # Panics
///
/// Does not panic: every vector [`all_perm_indices`] yields is a bijection of
/// `0..n` by construction, which is `Permutation::try_from`'s precondition.
///
/// # Examples
///
/// ```
/// use catgraph_testutil::all_perms;
///
/// assert_eq!(all_perms(4).len(), 24);
/// assert_eq!(all_perms(0).len(), 1, "S₀ is the trivial group, not empty");
/// ```
#[must_use]
pub fn all_perms(n: usize) -> Vec<Permutation> {
    all_perm_indices(n)
        .into_iter()
        .map(|v| Permutation::try_from(v).expect("a prefix-swap enumeration yields bijections"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `n ∈ 0..=5`: `n!` one-line notations, pairwise distinct, each of length
    /// `n` and a bijection of `0..n`, with `all_perms(n)` the same length.
    #[test]
    fn counts_are_factorial_and_distinct() {
        for (n, expected) in [(0usize, 1usize), (1, 1), (2, 2), (3, 6), (4, 24), (5, 120)] {
            let perms = all_perm_indices(n);
            assert_eq!(
                perms.len(),
                expected,
                "all_perm_indices({n}) must yield {n}!"
            );

            let mut distinct = perms.clone();
            distinct.sort_unstable();
            distinct.dedup();
            assert_eq!(
                distinct.len(),
                expected,
                "all_perm_indices({n}) must not repeat"
            );

            for v in &perms {
                assert_eq!(v.len(), n, "every one-line notation has length {n}");
                let mut sorted = v.clone();
                sorted.sort_unstable();
                assert_eq!(
                    sorted,
                    (0..n).collect::<Vec<_>>(),
                    "every entry is a bijection of 0..{n}"
                );
            }

            assert_eq!(all_perms(n).len(), expected);
        }
    }

    /// `n ∈ {3, 4}`: `all_perms(n)[k]` applied to `0..n` equals
    /// `all_perm_indices(n)[k]`, over every `k`.
    #[test]
    fn permutation_view_matches_the_indices() {
        for n in [3usize, 4] {
            let perms = all_perms(n);
            let indices = all_perm_indices(n);
            assert_eq!(
                perms.len(),
                indices.len(),
                "the two views must have the same length at n={n}"
            );
            for (k, (p, v)) in perms.iter().zip(&indices).enumerate() {
                assert_eq!(
                    &(0..n).map(|i| p.apply(i)).collect::<Vec<_>>(),
                    v,
                    "all_perms({n})[{k}] must be the permutation whose one-line notation is all_perm_indices({n})[{k}]"
                );
            }
        }
    }
}
