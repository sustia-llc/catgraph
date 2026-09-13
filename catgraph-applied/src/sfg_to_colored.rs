//! A signal-flow graph as a Λ-colored morphism.
//!
//! [`sfg_to_colored_expr`] pins the monochromatic source word onto an SFG's
//! underlying expression and pairs them into the [`ColoredExpr`] the rewrite
//! engine's entry points take. `SFG_R` is single-sorted — the [`SfgGenerator`]
//! palette is the one letter `()` and its interfaces are [`mono_word`]s — so
//! this is a representation change (the newtype unwrapped, plus the color word
//! spelled over the domain), not a functor. No paper anchor exists for it; it
//! is a crate extension, in the manner of
//! [`cost_of`](crate::prop::presentation::rewrite::cost_of).

use catgraph::errors::CatgraphError;

use crate::prop::colored::ColoredExpr;
use crate::prop::mono_word;
use crate::rig::Rig;
use crate::sfg::{SfgGenerator, SignalFlowGraph};

/// Bridge a signal-flow graph into the Λ-colored world.
///
/// Pairs `sfg`'s underlying expression with the [`mono_word`] over its domain
/// arity, producing the `ColoredExpr<SfgGenerator<R>>` the rewrite engine's
/// entry points take —
/// [`optimize`](crate::prop::presentation::rewrite::optimize) and
/// [`match_sites_of`](crate::prop::presentation::rewrite::match_sites_of)
/// among them. The expression is carried across unchanged: `colored.expr()`
/// is `sfg.as_prop_expr()`, and `colored.source_word()` is the mono word over
/// `sfg.domain()`.
///
/// # Errors
///
/// Propagates [`ColoredExpr::new`]'s — i.e.
/// [`check`](crate::prop::colored::check)'s — errors verbatim: a word-length
/// mismatch or a color mismatch. Neither can occur for a graph built through
/// `SignalFlowGraph`'s own constructors and combinators, which enforce arity
/// at construction; both are reachable through
/// [`from_prop_expr`](SignalFlowGraph::from_prop_expr)'s documented
/// no-validation path, when it wraps an arity-ill-formed tree.
pub fn sfg_to_colored_expr<R: Rig + std::fmt::Debug + Eq + std::hash::Hash + Ord + 'static>(
    sfg: &SignalFlowGraph<R>,
) -> Result<ColoredExpr<SfgGenerator<R>>, CatgraphError> {
    ColoredExpr::new(
        mono_word(sfg.domain()).into_owned(),
        sfg.as_prop_expr().clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mat::MatR;
    use crate::mat_to_sfg::mat_to_sfg;
    use crate::rig::{BoolRig, F64Rig, Tropical, UnitInterval};

    /// The rig bound every fixture in this module spells once.
    trait SweepRig: Rig + std::fmt::Debug + Eq + std::hash::Hash + Ord + 'static {}
    impl<R> SweepRig for R where R: Rig + std::fmt::Debug + Eq + std::hash::Hash + Ord + 'static {}

    /// A 2×3 zero/one matrix — `domain 2 ≠ codomain 3`, so the two boundary
    /// sides of the bridge cannot stand in for each other.
    fn two_by_three<R: SweepRig>() -> MatR<R> {
        MatR::new(
            2,
            3,
            vec![
                vec![R::one(), R::zero(), R::one()],
                vec![R::zero(), R::one(), R::zero()],
            ],
        )
        .expect("invariant: the 2×3 fixture is rectangular")
    }

    /// A 1→1 term touching all five `SfgGenerator` variants and all five
    /// `PropExpr` variants: `zero ; scalar(one) ; copy ; σ₁,₁ ; add ; discard`,
    /// tensored with `id₁`.
    fn all_variants<R: SweepRig>() -> SignalFlowGraph<R> {
        let spine = SignalFlowGraph::<R>::zero()
            .compose(&SignalFlowGraph::<R>::scalar(R::one()))
            .and_then(|g| g.compose(&SignalFlowGraph::<R>::copy()))
            .and_then(|g| g.compose(&SignalFlowGraph::<R>::braid_1_1()))
            .and_then(|g| g.compose(&SignalFlowGraph::<R>::add()))
            .and_then(|g| g.compose(&SignalFlowGraph::<R>::discard()))
            .expect("invariant: every step's arities line up (0→1→1→2→2→1→0)");
        spine.tensor(&SignalFlowGraph::<R>::identity(1))
    }

    /// The conversion's own three claims: it succeeds, the expression is
    /// carried across unchanged, and the source word is the mono word over the
    /// domain — not the codomain.
    fn assert_bridge<R: SweepRig>(sfg: &SignalFlowGraph<R>) {
        let colored = sfg_to_colored_expr(sfg)
            .unwrap_or_else(|error| panic!("the bridge accepts the SFG it is handed: {error:?}"));
        assert_eq!(
            colored.expr(),
            sfg.as_prop_expr(),
            "expr carried across: observed {:?}, expected {:?}",
            colored.expr(),
            sfg.as_prop_expr()
        );
        assert_eq!(
            colored.source_word().to_vec(),
            vec![(); sfg.domain()],
            "source word: observed {:?}, expected the mono word over domain = {}",
            colored.source_word(),
            sfg.domain()
        );
    }

    #[test]
    fn bridge_carries_the_expression_and_pins_the_domain_word() {
        // The house rig sweep: the four rigs canonical.rs ranges its SFG
        // claims over.
        assert_bridge::<BoolRig>(&mat_to_sfg(&two_by_three::<BoolRig>()).expect(
            "invariant: mat_to_sfg is arity-safe for a MatR built through its own constructors",
        ));
        assert_bridge::<BoolRig>(&all_variants::<BoolRig>());
        assert_bridge::<UnitInterval>(&mat_to_sfg(&two_by_three::<UnitInterval>()).expect(
            "invariant: mat_to_sfg is arity-safe for a MatR built through its own constructors",
        ));
        assert_bridge::<UnitInterval>(&all_variants::<UnitInterval>());
        assert_bridge::<Tropical>(&mat_to_sfg(&two_by_three::<Tropical>()).expect(
            "invariant: mat_to_sfg is arity-safe for a MatR built through its own constructors",
        ));
        assert_bridge::<Tropical>(&all_variants::<Tropical>());
        assert_bridge::<F64Rig>(&mat_to_sfg(&two_by_three::<F64Rig>()).expect(
            "invariant: mat_to_sfg is arity-safe for a MatR built through its own constructors",
        ));
        assert_bridge::<F64Rig>(&all_variants::<F64Rig>());
    }
}
