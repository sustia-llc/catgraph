//! V-enriched categories — hom-objects live in a monoidal category V
//! (F&S *Seven Sketches* §1.1, §2.4).
//!
//! The V-enriched refinement of an ordinary category replaces `Hom(a, b): Set`
//! with `Hom(a, b): V`. [`EnrichedCategory<V>`] takes V to be a [`Rig`]: the
//! rig's `·` is the monoidal composition, its `1` is the identity hom, and its
//! absorbing `0` represents "no hom".

use std::collections::HashMap;
use std::hash::Hash;

use crate::rig::Rig;

/// A V-enriched category over the rig V.
///
/// [`hom`](Self::hom) returns the hom-value between two objects, with
/// `V::zero()` signalling "no morphism"; being the rig's absorbing element it
/// propagates to "no composite" under [`compose_hom`](Self::compose_hom), which
/// defaults to `hom(a, b) · hom(b, c)`. [`id_hom`](Self::id_hom) defaults to
/// `V::one()`. Implementations may override either for specialised semantics.
///
/// The trait is object-safe: `Box<dyn EnrichedCategory<V, Object = T>>` names
/// both the `V: Rig` parameter and the `Object` associated type at the `dyn`
/// site.
///
/// ```rust,ignore
/// use catgraph_applied::enriched::{EnrichedCategory, HomMap};
/// use catgraph_applied::rig::Tropical;
///
/// let boxed: Box<dyn EnrichedCategory<Tropical, Object = char>>
///     = Box::new(HomMap::new(vec!['a', 'b']));
/// let _d = boxed.hom(&'a', &'b');
/// ```
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

/// A finite enriched category backed by an explicit hom-table: objects in an
/// insertion-ordered `Vec<O>`, homs in a `HashMap<(O, O), V>` whose unset
/// entries read as `V::zero()`.
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
