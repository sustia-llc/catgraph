//! Exhaustive permutation enumeration for braiding sweeps.
//!
//! The `#258` braiding contract is decided by *exhaustive* sweeps over `Sₙ`, and
//! every such sweep needs the same generator. It had drifted into four private
//! copies — two in `catgraph-applied/tests` (`braiding_cross_carrier.rs`,
//! `prop.rs`) and two more added by `catgraph/tests` at #286 — so it lives here
//! now, the workspace's designated dedup crate (#33).
//!
//! Two entry points, because the call sites want different things:
//!
//! - [`all_perm_indices`] yields raw `Vec<usize>` one-line notations, which sort
//!   and dedup (`permutations::Permutation` is neither `Ord` nor `Hash`), so a
//!   caller can pin *distinctness* of the enumeration itself;
//! - [`all_perms`] yields [`Permutation`] values ready to feed a constructor.

use permutations::Permutation;

/// Every permutation of `0..n` in one-line notation, each exactly once.
///
/// Prefix-swap (Heap-style) recursion: `n!` vectors of length `n`, so this is
/// for small `n` only — the workspace's sweeps run `n ≤ 4`.
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
/// [`all_perm_indices`] with each one-line notation converted; see there for the
/// enumeration contract.
///
/// # Panics
///
/// Never, on the enumeration this produces: every vector is a bijection of
/// `0..n` by construction, which is exactly `Permutation::try_from`'s
/// precondition.
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

    /// The enumeration is a helper, so "exhaustive" is a claim about *it*, not a
    /// fact the sweeps that consume it can check. Pin count and distinctness
    /// here: a generator that silently yielded fewer would leave every downstream
    /// sweep covering less than its name says while still passing.
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

            // The `Permutation` view must agree case for case.
            assert_eq!(all_perms(n).len(), expected);
        }
    }

    /// `all_perms` must preserve the one-line notation, not transpose it: the
    /// two views have to name the *same* permutation or a sweep reading through
    /// `all_perms` would pin the inverse convention.
    #[test]
    fn permutation_view_matches_the_indices() {
        for n in [3usize, 4] {
            for v in all_perm_indices(n) {
                let p = Permutation::try_from(v.clone()).expect("bijection");
                assert_eq!(
                    (0..n).map(|i| p.apply(i)).collect::<Vec<_>>(),
                    v,
                    "Permutation::apply must reproduce the one-line notation"
                );
            }
        }
    }
}
