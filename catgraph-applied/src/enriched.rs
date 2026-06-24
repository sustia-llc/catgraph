//! V-enriched categories — hom-objects live in a monoidal category V
//! (typically a [`Rig`]). Pedagogical references: F&S Seven Sketches §1.1, §2.4;
//! CTFP Ch 28.
//!
//! The `V`-enriched refinement of an ordinary category replaces
//! `Hom(a, b): Set` with `Hom(a, b): V` for a chosen monoidal category V.
//! catgraph-applied's [`EnrichedCategory<V>`] targets V = a [`Rig`]: the rig's `·`
//! is the monoidal composition, the rig's `1` is the identity hom, and the
//! rig's `0` represents "no hom" (absorbing zero).
//!
//! # Phase 6 role
//!
//! This trait is the catgraph-side enrichment substrate for the Phase 6
//! `catgraph-magnitude` sibling repo. BTV 2021 (arXiv:2106.07890) enriches
//! language categories over `[0,1]` (via [`crate::rig::UnitInterval`]); BV 2025
//! (arXiv:2501.06662) computes magnitude over [`crate::rig::Tropical`]-enriched
//! categories via the `-ln π` embedding provided by
//! [`crate::rig::BaseChange<UnitInterval> for Tropical`].

use std::collections::HashMap;
use std::hash::Hash;

use crate::rig::Rig;

/// A V-enriched category over the rig V.
///
/// # Semantics
///
/// - [`hom`](Self::hom) returns the hom-value between two objects. By
///   convention, `V::zero()` signals "no morphism" — `zero` is the rig's
///   absorbing element, which under [`compose_hom`](Self::compose_hom)
///   propagates to "no composite".
/// - [`id_hom`](Self::id_hom) defaults to `V::one()`, the rig's
///   multiplicative unit.
/// - [`compose_hom`](Self::compose_hom) defaults to `hom(a, b) · hom(b, c)`;
///   implementations may override for specialised semantics (e.g. over
///   [`crate::rig::Tropical`] this coincides with real addition of
///   distances — shortest-path semantics).
///
/// # Object safety
///
/// This trait IS object-safe. Callers may use `Box<dyn EnrichedCategory<V, Object = T>>`
/// (specifying both the `V: Rig` parameter and the `Object` associated type at the
/// `dyn` site). The associated type constraint is required because trait objects
/// erase the concrete `Self`, so `Self::Object` needs a binding to be nameable:
///
/// ```rust,ignore
/// use catgraph_applied::enriched::{EnrichedCategory, HomMap};
/// use catgraph_applied::rig::Tropical;
///
/// let boxed: Box<dyn EnrichedCategory<Tropical, Object = char>>
///     = Box::new(HomMap::new(vec!['a', 'b']));
/// let _d = boxed.hom(&'a', &'b');
/// ```
///
/// This is important for Phase 6 `catgraph-magnitude` consumers that may hold
/// heterogeneous collections of enriched categories.
pub trait EnrichedCategory<V: Rig> {
    /// Objects of the enriched category.
    type Object: Clone + Eq + Hash;

    /// The hom-value between two objects. `V::zero()` signals "no morphism".
    fn hom(&self, a: &Self::Object, b: &Self::Object) -> V;

    /// Identity hom — must equal `V::one()`. Default impl returns `V::one()`.
    fn id_hom(&self, _a: &Self::Object) -> V {
        V::one()
    }

    /// Composition hom — by default, `hom(a, b) · hom(b, c)`. Implementations
    /// may override for specialised semantics (e.g. min-plus = shortest path).
    fn compose_hom(&self, a: &Self::Object, b: &Self::Object, c: &Self::Object) -> V {
        self.hom(a, b) * self.hom(b, c)
    }

    /// Iterator over all objects.
    fn objects(&self) -> Box<dyn Iterator<Item = Self::Object> + '_>;
}

/// A concrete finite enriched category backed by an explicit hom-table.
///
/// Useful for testing and small-finite enrichment cases. Objects are stored
/// in a `Vec<O>` (insertion-ordered); homs are stored in a
/// `HashMap<(O, O), V>` with unset entries defaulting to `V::zero()`.
#[derive(Debug, Clone)]
pub struct HomMap<O, V>
where
    O: Clone + Eq + Hash,
    V: Rig,
{
    objects: Vec<O>,
    homs: HashMap<(O, O), V>,
}

impl<O, V> HomMap<O, V>
where
    O: Clone + Eq + Hash,
    V: Rig,
{
    /// Construct an empty `HomMap` over a fixed object list. All hom-values
    /// start at `V::zero()`; use [`set_hom`](Self::set_hom) to populate.
    #[must_use]
    pub fn new(objects: Vec<O>) -> Self {
        Self {
            objects,
            homs: HashMap::new(),
        }
    }

    /// Set the hom-value between `a` and `b` (overwriting any prior value).
    pub fn set_hom(&mut self, a: O, b: O, v: V) {
        self.homs.insert((a, b), v);
    }
}

impl<O, V> EnrichedCategory<V> for HomMap<O, V>
where
    O: Clone + Eq + Hash + 'static,
    V: Rig + 'static,
{
    type Object = O;

    fn hom(&self, a: &Self::Object, b: &Self::Object) -> V {
        self.homs
            .get(&(a.clone(), b.clone()))
            .cloned()
            .unwrap_or_else(V::zero)
    }

    fn objects(&self) -> Box<dyn Iterator<Item = Self::Object> + '_> {
        Box::new(self.objects.iter().cloned())
    }
}
