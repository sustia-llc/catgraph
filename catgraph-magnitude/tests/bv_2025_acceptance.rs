//! BV 2025 acceptance gate for `catgraph-magnitude`.
//!
//! Two paper-anchored numerical tests on a hand-computed 3-state LM:
//!
//! 1. **Thm 3.10 closed form.** `Mag(tM) = (t − 1) · Σ_{x ∉ T(⊥)} H_t(p_x) +
//!    #(T(⊥))` matches the Möbius-sum value from
//!    [`LmCategory::magnitude`] at `t ∈ {0.5, 1.5, 2.0, 5.0}` (avoiding
//!    `t = 1` exactly because of the Shannon special case).
//! 2. **Cor 3.14 Shannon recovery.** `d/dt Mag(tM)|_{t=1} = Σ_{x ∉ T(⊥)}
//!    H(p_x)` verified by central finite difference with `h = 1e-4`.
//!
//! ## Hand-fixture — `A = {a}`, `N = 1`
//!
//! Following BV 2025 §2.15, with `A = {a}` (single non-terminal token) and
//! cutoff `N = 1`, `L^{≤1}` has exactly four objects:
//!
//! ```text
//! States: ["s0", "s0a", "s0t", "s0at"]
//!         (= ⊥, ⊥a, ⊥†, ⊥a†)
//! Terminating: {"s0t", "s0at"}  — strings ending in †, ⇒ #T(⊥) = 2
//!
//! Transitions (rows are next-token distributions p_x):
//!   s0  → s0a  with π = 0.6      (next token = a)
//!   s0  → s0t  with π = 0.4      (next token = †)
//!   s0a → s0at with π = 1.0      (forced: |s0a| = N = 1, only † extends)
//!   s0t, s0at: terminal, no outgoing transitions.
//! ```
//!
//! Each non-terminating row sums **exactly to 1** — the BV 2025 §2 hypothesis
//! that `p_x: A ∪ {†} → [0, 1]` is a true probability mass function. There
//! is no implicit terminal mass here because every successor is in `ob(M)`.
//!
//! ## On the BV 2025 entropy convention
//!
//! Per BV 2025 Eq (10) ↔ (11), the inner sum `Σ_a p_x(a)^t` runs over the
//! direct children `L_x^{(1)}` of `x` *inside the truncated category* `L^{≤N}`.
//! For tree-shaped `L^{≤N}` with all children present, this equals
//! `Σ_{a ∈ A ∪ {†}} p_x(a)^t` since every `xa ∈ ob(L)`. We compute the sum
//! directly from `transitions[x].values()` — no implicit `†` term is added,
//! because in this fixture every non-terminating row is already normalized.
//!
//! ## Hand-computed Mag at `t = 2` (sanity reference)
//!
//! `Mag(2M) = 4 − (0.36 + 0.16 + 0.36) − (1) = 2.48` via Eq (10).
//! Equivalently: `(t − 1)·[H_2(p_s0) + H_2(p_s0a)] + #T(⊥) = 1·(0.48 + 0) + 2 = 2.48`.

// `usize → f64` casts on small-state-count test fixtures are precision-safe.
#![allow(clippy::cast_precision_loss)]

use catgraph_magnitude::lm_category::LmCategory;
use catgraph_magnitude::magnitude::tsallis_entropy;

/// Build the 4-state hand fixture from the module docs (`A={a}`, `N=1`).
fn build_bv_lm() -> LmCategory {
    let mut m = LmCategory::new(vec!["s0".into(), "s0a".into(), "s0t".into(), "s0at".into()]);
    m.mark_terminating("s0t");
    m.mark_terminating("s0at");
    // p_s0 = (a: 0.6, †: 0.4) — recorded as transitions to s0a and s0t.
    m.add_transition("s0", "s0a", 0.6).unwrap();
    m.add_transition("s0", "s0t", 0.4).unwrap();
    // p_s0a = (†: 1.0) — only legal extension is termination.
    m.add_transition("s0a", "s0at", 1.0).unwrap();
    m
}

/// Closed-form RHS of Thm 3.10's Tsallis sum: `Σ_{x ∉ T(⊥)} H_t(p_x)`.
///
/// Per BV 2025 Eq (10) the inner sum is over the direct children of `x`
/// inside the truncated category, equivalently the recorded transition
/// values when every successor is in `ob(M)`.
fn tsallis_sum(m: &LmCategory, t: f64) -> f64 {
    m.objects()
        .iter()
        .filter(|x| !m.terminating().contains(*x))
        .map(|x| {
            let probs: Vec<f64> = m
                .transitions()
                .get(x)
                .map(|r| r.values().copied().collect())
                .unwrap_or_default();
            tsallis_entropy(&probs, t)
        })
        .sum()
}

/// Shannon-entropy variant of [`tsallis_sum`] — used by the Cor 3.14 test.
fn shannon_sum(m: &LmCategory) -> f64 {
    m.objects()
        .iter()
        .filter(|x| !m.terminating().contains(*x))
        .map(|x| {
            let probs: Vec<f64> = m
                .transitions()
                .get(x)
                .map(|r| r.values().copied().collect())
                .unwrap_or_default();
            // tsallis_entropy at t=1 hits the Shannon special-case branch
            // via TSALLIS_SHANNON_EPS.
            tsallis_entropy(&probs, 1.0)
        })
        .sum()
}

#[test]
fn bv_2025_thm_3_10_closed_form() {
    let m = build_bv_lm();
    let mut max_residual: f64 = 0.0;
    for &t in &[0.5_f64, 1.5, 2.0, 5.0] {
        let lhs = m.magnitude(t).expect("zeta_t should be invertible");
        let rhs = (t - 1.0) * tsallis_sum(&m, t) + (m.terminating().len() as f64);
        let residual = (lhs - rhs).abs();
        max_residual = max_residual.max(residual);
        assert!(
            residual < 1e-9,
            "Thm 3.10 failed at t={t}: lhs={lhs}, rhs={rhs}, residual={residual}"
        );
    }
    // Surface the worst-case residual in test output via panic-free path.
    eprintln!("BV 2025 Thm 3.10: max |lhs − rhs| over 4 t-values = {max_residual:e}");
}

#[test]
fn bv_2025_cor_3_14_shannon_recovery() {
    let m = build_bv_lm();
    // h = 1e-4 > TSALLIS_SHANNON_EPS = 1e-6 so both f(1±h) hit the
    // Tsallis branch.
    let h = 1e-4_f64;
    let mag_plus = m.magnitude(1.0 + h).expect("zeta_t invertible at 1+h");
    let mag_minus = m.magnitude(1.0 - h).expect("zeta_t invertible at 1-h");
    let lhs = (mag_plus - mag_minus) / (2.0 * h);
    let rhs = shannon_sum(&m);
    let residual = (lhs - rhs).abs();
    assert!(
        residual < 1e-6,
        "Cor 3.14 failed: lhs={lhs}, rhs={rhs}, residual={residual}"
    );
    eprintln!("BV 2025 Cor 3.14: |fd − shannon| = {residual:e}");
}
