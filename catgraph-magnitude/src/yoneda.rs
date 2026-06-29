//! [`Copresheaf`] — the BTV 2021 Yoneda embedding `x ↦ L(x, −)`.
//!
//! Bradley–Terilla–Vlassopoulos 2021 (*An enriched category theory of language*)
//! models a language model as a category `L` enriched over the unit interval:
//! objects are texts, and the hom-object `L(x, y) = π(y | x) ∈ [0, 1]` is the
//! probability that `y` is an extension of `x`. The **representable copresheaf**
//! `L(x, −)` — the function sending each text `y` to `π(y | x)` — *is* the
//! meaning of `x`: its distribution of extension-probabilities over every other
//! text. This is the computable form of relational, reference-free meaning
//! ("meaning is context-change potential").
//!
//! # Relationship to [`LmCategory`]
//!
//! [`LmCategory::enriched_space`](crate::lm_category::LmCategory::enriched_space)
//! already materializes the full hom-matrix as a [`LawvereMetricSpace`] under the
//! `d(x, y) = −ln π(y | x)` embedding (BTV 2021 §5; the isomorphism
//! `[0, 1] ≅ [0, ∞]^op`). A copresheaf is one **row** of that space, read back in
//! probability form via `π = exp(−d)`. So [`LmCategory::yoneda`] introduces no new
//! traversal — it reuses the same space [`magnitude`](crate::lm_category::LmCategory::magnitude)
//! consumes.
//!
//! # Semantic distance (BTV 2021)
//!
//! The hom between two copresheaves `f, g` in the semantic category `L̂` is the
//! **asymmetric** end (BTV 2021 Lemma 2, Eq 11):
//!
//! ```text
//! L̂(f, g) = inf_c [f(c), g(c)] = inf_c min{1, g(c) / f(c)}   ∈ [0, 1]
//! ```
//!
//! with the unit-interval internal hom `[a, b] = min{1, b/a}` (Lemma 1, Eq 6;
//! `b / 0 ≥ 1`). In the metric view (§5, internal hom = truncated subtraction)
//! this is `d̂(f, g) = −ln L̂(f, g) = sup_c max{0, G(c) − F(c)}`. BTV keep the
//! asymmetric Lawvere generalized metric ("symmetry is not required"), so
//! [`semantic_distance`] is asymmetric; [`semantic_distance_sym`] is a labelled,
//! non-canonical symmetric convenience.

use crate::lm_category::LmCategory;
use crate::weighted_cospan::NodeId;
use crate::{CatgraphError, LawvereMetricSpace, Tropical};

/// The Yoneda image of one text: its representable copresheaf `L(x, −)`.
///
/// Stored in **probability form**: `extensions[y] = L(x, y) = π(y | x) ∈ [0, 1]`,
/// indexed by `NodeId` (position in [`LmCategory::objects`]), with
/// `extensions[x] = 1.0` (identity, `π(x | x) = 1`) and `0.0` for texts
/// unreachable from `x`. The metric form `d(x, y) = −ln L(x, y) ∈ [0, ∞]` is
/// available via [`Copresheaf::distance_to`].
#[derive(Clone, Debug, PartialEq)]
pub struct Copresheaf {
    base: NodeId,
    extensions: Vec<f64>,
}

impl Copresheaf {
    /// The basepoint text `x` whose meaning this copresheaf represents.
    #[must_use]
    pub fn base(&self) -> NodeId {
        self.base
    }

    /// `L(x, y) = π(y | x)` as an extension probability in `[0, 1]`. Out-of-range
    /// `y` (not an object of the source category) reads as `0.0`.
    #[must_use]
    pub fn extension_to(&self, y: NodeId) -> f64 {
        self.extensions.get(y).copied().unwrap_or(0.0)
    }

    /// `d(x, y) = −ln L(x, y) ∈ [0, ∞]` (BTV 2021 §5). Returns `f64::INFINITY`
    /// for unreachable (or out-of-range) `y`.
    #[must_use]
    pub fn distance_to(&self, y: NodeId) -> f64 {
        let p = self.extension_to(y);
        if p > 0.0 { -p.ln() } else { f64::INFINITY }
    }

    /// The forward extension-support of `x`: objects `y` with `L(x, y) > 0`.
    pub fn support(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.extensions
            .iter()
            .enumerate()
            .filter_map(|(y, &p)| (p > 0.0).then_some(y))
    }

    /// The raw extension vector `[L(x, 0), L(x, 1), …]` in object order.
    #[must_use]
    pub fn extensions(&self) -> &[f64] {
        &self.extensions
    }
}

impl LmCategory {
    /// Yoneda embedding of a single text: its representable copresheaf `L(x, −)`
    /// (BTV 2021). The "meaning" of `name` as a distribution of extension
    /// probabilities over every object.
    ///
    /// # Errors
    ///
    /// - [`CatgraphError::Composition`] if `name` is not an object of this
    ///   category, or propagated from
    ///   [`enriched_space`](Self::enriched_space) (BFS-cap on malformed input).
    pub fn yoneda(&self, name: &str) -> Result<Copresheaf, CatgraphError> {
        let base = self
            .objects()
            .iter()
            .position(|o| o == name)
            .ok_or_else(|| CatgraphError::Composition {
                message: format!("yoneda: {name:?} is not an object of this LmCategory"),
            })?;
        let space = self.enriched_space()?;
        Ok(copresheaf_from_space(&space, base))
    }
}

/// Read row `x` of an enriched space as a copresheaf (probability form,
/// `π = exp(−d)`). Unset distances are `Tropical(+∞)` ⇒ `0.0`.
///
/// # Panics
///
/// Panics if `x` is not an object of `space` (`x >= space.size()`); an
/// out-of-range row has no meaning as a copresheaf. [`LmCategory::yoneda`]
/// always passes a validated `base`, so this fires only on direct misuse.
#[must_use]
pub fn copresheaf_from_space(space: &LawvereMetricSpace<NodeId>, x: NodeId) -> Copresheaf {
    let n = space.size();
    assert!(
        x < n,
        "copresheaf_from_space: object index {x} out of range (space has {n} objects)"
    );
    let extensions = (0..n)
        .map(|y| {
            let Tropical(d) = space.distance(&x, &y); // d(x, y) = −ln π(y | x)
            // π = exp(−d); unreachable distances are +∞ ⇒ exp(−∞) = 0.0
            // (matches the inline convention in `magnitude` / `mobius_chains`).
            (-d).exp()
        })
        .collect();
    Copresheaf {
        base: x,
        extensions,
    }
}

/// Asymmetric semantic hom `L̂(a, b) = inf_c min{1, b(c) / a(c)} ∈ [0, 1]`
/// between two meanings (BTV 2021 Lemma 2, Eq 11; internal hom Eq 6).
///
/// `1.0` ⇒ `b`'s meaning everywhere extends `a`'s (`b(c) ≥ a(c)` for all `c`);
/// `0.0` ⇒ some context `a` reaches that `b` cannot. **Asymmetric** in general.
/// Computed in probability form, so the `b / 0 ≥ 1` convention (Eq 6) needs no
/// `∞` handling: an `a(c) = 0` term contributes the monoidal unit `1.0`.
///
/// # Panics
///
/// Panics if `a` and `b` are copresheaves over different [`LmCategory`]
/// instances (mismatched object counts) — the infimum over contexts is only
/// defined when both share one object indexing. A silent `zip`-truncation
/// would otherwise return a mathematically wrong (too-large) hom in release.
#[must_use]
pub fn semantic_hom(a: &Copresheaf, b: &Copresheaf) -> f64 {
    assert_eq!(
        a.extensions.len(),
        b.extensions.len(),
        "semantic_hom: copresheaves must share one LmCategory's object indexing"
    );
    a.extensions
        .iter()
        .zip(&b.extensions)
        .map(|(&fc, &gc)| {
            if fc <= 0.0 {
                1.0 // b / 0 ≥ 1  ⇒  [f c, g c] = 1  (BTV Eq 6)
            } else {
                (gc / fc).min(1.0) // truncated division
            }
        })
        .fold(1.0_f64, f64::min) // inf over objects; vacuous (empty) = 1.0
}

/// Asymmetric semantic **distance** `d̂(a, b) = −ln L̂(a, b) ∈ [0, ∞]`
/// (BTV 2021 §5; `[0, ∞]` truncated-subtraction hom). `0.0` ⇒ `b` everywhere
/// extends `a`; `∞` ⇒ `a` reaches a context `b` cannot. This is the
/// BTV-canonical object — keep it asymmetric; for a symmetric metric use
/// [`semantic_distance_sym`].
#[must_use]
pub fn semantic_distance(a: &Copresheaf, b: &Copresheaf) -> f64 {
    let hom = semantic_hom(a, b);
    if hom > 0.0 { -hom.ln() } else { f64::INFINITY }
}

/// Symmetric semantic distance `max(d̂(a, b), d̂(b, a))`. **Derived, not the BTV
/// enriched hom** — provided for callers needing a symmetric metric (e.g.
/// clustering). BTV 2021 §5 keeps the asymmetric generalized metric; prefer
/// [`semantic_distance`] for paper-faithful work.
#[must_use]
pub fn semantic_distance_sym(a: &Copresheaf, b: &Copresheaf) -> f64 {
    semantic_distance(a, b).max(semantic_distance(b, a))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Toy autoregressive LM over `⊥ → {a, b} → c`:
    ///   π(a|⊥)=0.6, π(b|⊥)=0.4 (terminal mass 0); π(c|a)=1.0; b terminal.
    fn toy() -> LmCategory {
        let mut m = LmCategory::new(vec!["⊥".into(), "a".into(), "b".into(), "c".into()]);
        m.add_transition("⊥", "a", 0.6).unwrap();
        m.add_transition("⊥", "b", 0.4).unwrap();
        m.add_transition("a", "c", 1.0).unwrap();
        m.mark_terminating("b");
        m.mark_terminating("c");
        m
    }

    const EPS: f64 = 1e-12;

    #[test]
    fn copresheaf_identity_and_extension_probs() {
        let m = toy();
        let bot = m.yoneda("⊥").unwrap();
        // Identity axiom: L(⊥, ⊥) = π(⊥|⊥) = 1.
        assert!((bot.extension_to(0) - 1.0).abs() < EPS);
        // Direct extensions: π(a|⊥)=0.6, π(b|⊥)=0.4.
        assert!((bot.extension_to(1) - 0.6).abs() < EPS);
        assert!((bot.extension_to(2) - 0.4).abs() < EPS);
        // Two-step extension π(c|⊥) = π(a|⊥)·π(c|a) = 0.6·1.0 = 0.6.
        assert!((bot.extension_to(3) - 0.6).abs() < EPS);
    }

    #[test]
    fn unreachable_text_has_zero_extension_and_infinite_distance() {
        let m = toy();
        let a = m.yoneda("a").unwrap();
        // ⊥ is not an extension of `a` (no a→⊥ path): L(a, ⊥) = 0.
        assert_eq!(a.extension_to(0), 0.0);
        assert!(a.distance_to(0).is_infinite());
        // `a` extends to itself (1.0) and to c (1.0).
        assert!((a.extension_to(1) - 1.0).abs() < EPS);
        assert!((a.extension_to(3) - 1.0).abs() < EPS);
        let support: Vec<NodeId> = a.support().collect();
        assert_eq!(support, vec![1, 3]);
    }

    #[test]
    fn distance_to_matches_neg_log_extension() {
        let m = toy();
        let bot = m.yoneda("⊥").unwrap();
        assert!((bot.distance_to(1) - (-0.6_f64.ln())).abs() < EPS);
        assert_eq!(bot.distance_to(0), 0.0); // -ln 1
    }

    #[test]
    fn semantic_hom_is_asymmetric() {
        let m = toy();
        let bot = m.yoneda("⊥").unwrap(); // reaches ⊥,a,b,c
        let a = m.yoneda("a").unwrap(); // reaches a,c

        // L̂(a, ⊥): for every c, is ⊥'s extension ≥ a's? a reaches a(1.0),c(1.0);
        // ⊥ reaches a(0.6),c(0.6) — both < 1.0 ⇒ inf gives 0.6.
        let h_a_bot = semantic_hom(&a, &bot);
        assert!((h_a_bot - 0.6).abs() < EPS);

        // L̂(⊥, a): ⊥ reaches ⊥(1.0) but a does NOT (a→⊥ absent) ⇒ term 0 ⇒ inf 0.
        let h_bot_a = semantic_hom(&bot, &a);
        assert!(h_bot_a.abs() < EPS);

        // Asymmetric.
        assert!((h_a_bot - h_bot_a).abs() > 0.1);
    }

    #[test]
    fn semantic_distance_and_symmetric_variant() {
        let m = toy();
        let bot = m.yoneda("⊥").unwrap();
        let a = m.yoneda("a").unwrap();

        // d̂(a, ⊥) = -ln 0.6 (finite); d̂(⊥, a) = -ln 0 = ∞.
        assert!((semantic_distance(&a, &bot) - (-0.6_f64.ln())).abs() < EPS);
        assert!(semantic_distance(&bot, &a).is_infinite());

        // Self-distance is 0 (a copresheaf everywhere extends itself).
        assert_eq!(semantic_distance(&a, &a), 0.0);

        // Symmetric variant = max of both directions = ∞ here.
        assert!(semantic_distance_sym(&bot, &a).is_infinite());
        assert!(semantic_distance_sym(&a, &bot).is_infinite());
    }
}
