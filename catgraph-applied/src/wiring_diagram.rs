//! Wiring diagram operad built on named cospans.
//!
//! A [`WiringDiagram`] wraps a [`NamedCospan`] where the left (domain) boundary
//! nodes sit on named **inner circles** (sub-boxes) and the right (codomain)
//! boundary nodes sit on a single **outer circle**. Wires are typed by `Lambda`;
//! ports are identified by direction ([`Dir`]) and circle/position names.
//!
//! The key operation is **operadic substitution** via the [`Operadic`] trait:
//! replacing an inner circle with another wiring diagram by composing the
//! underlying named cospans (matching outer ports of the substituted diagram
//! to the inner ports of the host diagram).
//!
//! Also supports boundary mutation (add/delete/connect/rename ports), orientation
//! flipping, and functorial mapping over wire types.
//!
//! See also `examples/wiring_diagram.rs`.

use either::Either::{Left, Right};

use catgraph::errors::CatgraphError;
use catgraph::operadic::Operadic;

use {
    catgraph::{
        category::Composable,
        monoidal::Monoidal,
        monoidal::SymmetricMonoidalMorphism,
        named_cospan::NamedCospan,
        utils::{necessary_permutation, remove_multiple},
    },
    either::Either,
    std::fmt::Debug,
};

/// Port direction on a wiring diagram circle: inward, outward, or undirected.
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub enum Dir {
    In,
    Out,
    Undirected,
}

impl Dir {
    /// Reverse the direction (`In` <-> `Out`); `Undirected` is unchanged.
    #[must_use]
    pub fn flipped(self) -> Self {
        match self {
            Self::In => Self::Out,
            Self::Out => Self::In,
            Self::Undirected => Self::Undirected,
        }
    }
}

type Pair<T> = (T, T);
type EitherPair<T, U> = Either<Pair<T>, Pair<U>>;

/// A wiring diagram in the operad of wiring diagrams.
///
/// Internally a [`NamedCospan`] whose left boundary nodes sit on named inner
/// circles (sub-boxes) and whose right boundary nodes sit on a single outer circle.
/// Wire labels are typed by `Lambda`. Inner-circle ports are identified by
/// `(Dir, InterCircle, IntraCircle)`, outer-circle ports by `(Dir, IntraCircle)`.
///
/// Implements [`Operadic<InterCircle>`] for substitution, [`Composable`] for
/// sequential wiring, and [`Monoidal`] for parallel composition.
#[derive(Clone)]
#[repr(transparent)]
pub struct WiringDiagram<
    Lambda: Eq + Copy + Debug,
    InterCircle: Eq + Clone,
    IntraCircle: Eq + Clone,
>(NamedCospan<Lambda, (Dir, InterCircle, IntraCircle), (Dir, IntraCircle)>);

impl<Lambda, InterCircle, IntraCircle> WiringDiagram<Lambda, InterCircle, IntraCircle>
where
    Lambda: Eq + Copy + Debug,
    InterCircle: Eq + Clone,
    IntraCircle: Eq + Clone,
{
    /// Wrap a [`NamedCospan`] as a wiring diagram (zero-cost newtype construction).
    #[must_use]
    pub fn new(
        inside: NamedCospan<Lambda, (Dir, InterCircle, IntraCircle), (Dir, IntraCircle)>,
    ) -> Self {
        Self(inside)
    }

    /// Access the underlying [`NamedCospan`].
    #[must_use]
    pub fn inner(
        &self,
    ) -> &NamedCospan<Lambda, (Dir, InterCircle, IntraCircle), (Dir, IntraCircle)> {
        &self.0
    }

    /// Rename a boundary node. No-op with a warning if the old name is not found.
    pub fn change_boundary_node_name(
        &mut self,
        name_pair: EitherPair<(Dir, InterCircle, IntraCircle), (Dir, IntraCircle)>,
    ) {
        self.0.change_boundary_node_name(name_pair);
    }

    /// Flip `In` ↔ `Out` on all ports of the specified side (left = inner circles, right = outer).
    pub fn toggle_orientation(&mut self, of_left_side: bool) {
        let toggler = if of_left_side {
            Left(|z: &mut (Dir, InterCircle, IntraCircle)| {
                z.0 = z.0.flipped();
            })
        } else {
            Right(|z: &mut (Dir, IntraCircle)| {
                z.0 = z.0.flipped();
            })
        };
        self.0.change_boundary_node_names(toggler);
    }

    /// Add a new boundary node connected to a fresh, isolated middle node of the given type.
    pub fn add_boundary_node_unconnected(
        &mut self,
        type_: Lambda,
        new_name: Either<(Dir, InterCircle, IntraCircle), (Dir, IntraCircle)>,
    ) {
        let _ = self.0.add_boundary_node_unknown_target(type_, new_name);
    }

    /// Merge the middle nodes behind two boundary nodes (by name).
    ///
    /// No-op if either name is missing or the two middle nodes have different labels.
    pub fn connect_pair(
        &mut self,
        node_1: Either<(Dir, InterCircle, IntraCircle), (Dir, IntraCircle)>,
        node_2: Either<(Dir, InterCircle, IntraCircle), (Dir, IntraCircle)>,
    ) {
        self.0.connect_pair(node_1, node_2);
    }

    /// Delete a boundary node by name. No-op with a warning if not found.
    pub fn delete_boundary_node(
        &mut self,
        which_node: Either<(Dir, InterCircle, IntraCircle), (Dir, IntraCircle)>,
    ) {
        self.0.delete_boundary_node_by_name(which_node);
    }

    /// Apply `f` to every wire label (middle set element), producing a new diagram
    /// with mapped types. Port names and structure are preserved.
    pub fn map<F, Mu>(&self, f: F) -> WiringDiagram<Mu, InterCircle, IntraCircle>
    where
        F: Fn(Lambda) -> Mu,
        Mu: Sized + Eq + Copy + Debug,
    {
        WiringDiagram::new(self.0.map(f))
    }
}

impl<Lambda, InterCircle, IntraCircle> Operadic<InterCircle>
    for WiringDiagram<Lambda, InterCircle, IntraCircle>
where
    Lambda: Eq + Copy + Debug,
    InterCircle: Eq + Clone + Send + Sync,
    IntraCircle: Eq + Copy + Send + Sync,
{
    fn operadic_substitution(
        &mut self,
        which_circle: InterCircle,
        mut internal_other: Self,
    ) -> Result<(), CatgraphError> {
        let found_nodes: Vec<_> = NamedCospan::find_nodes_by_name_predicate(
            &self.0,
            |z| z.1 == which_circle,
            |_| false,
            false,
        )
        .iter()
        .filter_map(|x| x.left())
        .collect();

        let mut self_inner_interface_unaffected = self.0.domain();
        remove_multiple(&mut self_inner_interface_unaffected, found_nodes.clone());
        let mut self_inner_names_unaffected = self.0.left_names().clone();
        remove_multiple(&mut self_inner_names_unaffected, found_nodes);

        internal_other.0.monoidal(NamedCospan::identity(
            &self_inner_interface_unaffected,
            &self_inner_names_unaffected,
            |left_name| (left_name.clone(), (left_name.0.flipped(), left_name.2)),
        ));

        let p = necessary_permutation(
            internal_other.0.right_names(),
            &self
                .0
                .left_names()
                .iter()
                .map(|z| (z.0.flipped(), z.2))
                .collect::<Vec<_>>(),
        )
        .map_err(|message| CatgraphError::Operadic { message })?;
        internal_other.0.permute_side(&p, true);

        self.0 = internal_other
            .0
            .compose(&self.0)
            .map_err(|z| CatgraphError::Operadic {
                message: format!("{z:?}"),
            })?;
        Ok(())
    }
}

impl<Lambda, InterCircle, IntraCircle> Composable<Vec<Lambda>>
    for WiringDiagram<Lambda, InterCircle, IntraCircle>
where
    Lambda: Sized + Eq + Copy + Debug,
    InterCircle: Eq + Clone,
    IntraCircle: Eq + Clone,
{
    fn compose(&self, other: &Self) -> Result<Self, CatgraphError> {
        Ok(Self(self.0.compose(&other.0)?))
    }

    fn domain(&self) -> Vec<Lambda> {
        self.0.domain()
    }

    fn codomain(&self) -> Vec<Lambda> {
        self.0.codomain()
    }
}

impl<Lambda, InterCircle, IntraCircle> Monoidal for WiringDiagram<Lambda, InterCircle, IntraCircle>
where
    Lambda: Sized + Eq + Copy + Debug,
    InterCircle: Eq + Clone,
    IntraCircle: Eq + Clone,
{
    fn monoidal(&mut self, other: Self) {
        self.0.monoidal(other.0);
    }
}

#[cfg(test)]
mod test {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn no_input_example() {
        use super::{Dir, WiringDiagram};
        use catgraph::named_cospan::NamedCospan;
        use either::Right;
        let unchanged_right_names = vec![
            (Dir::In, 0),
            (Dir::Out, 1),
            (Dir::In, 2),
            (Dir::Out, 3),
            (Dir::Out, 4),
        ];
        let mut example: WiringDiagram<_, (), _> = WiringDiagram::new(NamedCospan::new(
            vec![],
            vec![0, 1, 2, 2, 0],
            vec![true, true, false],
            vec![],
            unchanged_right_names.clone(),
        ));
        assert_eq!(*example.0.right_names(), unchanged_right_names);
        example.change_boundary_node_name(Right(((Dir::In, 0), (Dir::Out, 0))));
        let changed_names = example.0.right_names();
        assert_eq!(changed_names[0], (Dir::Out, 0));
        assert_eq!(changed_names[1..], unchanged_right_names[1..]);
    }

    #[allow(clippy::items_after_statements, clippy::too_many_lines)]
    #[test]
    fn operadic() {
        use super::{Dir, WiringDiagram};
        use catgraph::assert_ok;
        use catgraph::category::Composable;
        use catgraph::monoidal::SymmetricMonoidalMorphism;
        use catgraph::named_cospan::NamedCospan;
        use catgraph::operadic::Operadic;
        use permutations::Permutation;

        type WireName = usize;
        type CircleName = i32;
        let inner_right_names: Vec<(_, WireName)> = vec![
            (Dir::In, 0),
            (Dir::Out, 1),
            (Dir::In, 2),
            (Dir::Out, 3),
            (Dir::Out, 4),
        ];
        let mut outer_left_names: Vec<(_, CircleName, _)> = inner_right_names
            .iter()
            .map(|(orient, name)| (orient.flipped(), 0, *name))
            .collect();
        outer_left_names.push((Dir::Undirected, 1, 500));

        /*
        inner circle has no further inner circles
        it has 5 ports on the outside
        0 and 4 are connected to a common middle with type true
        2 and 3 are connected to a common middle with type false
        1 is connected to a middle with type true
        0 and 2 are oriented in to the boundary
        their names are just the numbers and orientations
        */
        let example_inner: WiringDiagram<_, CircleName, WireName> =
            WiringDiagram::new(NamedCospan::new(
                vec![],
                vec![0, 1, 2, 2, 0],
                vec![true, true, false],
                vec![],
                inner_right_names,
            ));
        /*
        outer circle has 2 inner circles
        the first has 5 ports for the outer of previous to connect to
        0, 1 and 4 are connected to a common middle with type true
            and that is connected to the only port on the very outer circle
        2 and 3 are connected to a common middle with type false
        the second has 1 port which is undirected and labelled 500 and of type false
        */
        let mut example_outer: WiringDiagram<_, _, _> = WiringDiagram::new(NamedCospan::new(
            vec![0, 0, 1, 1, 0, 1],
            vec![0],
            vec![true, false],
            outer_left_names.clone(),
            vec![(Dir::Out, 0)],
        ));
        /*
        permuting the domain of outer doesn't matter because the
        wires will be matched up by name not by index
        */
        use rand::seq::SliceRandom;
        let mut rng = StdRng::seed_from_u64(5001);
        let mut y: Vec<usize> = (0..6).collect();
        y.shuffle(&mut rng);
        let p = Permutation::try_from(&y).unwrap();
        example_outer.0.permute_side(&p, false);
        example_outer.0.assert_valid_nohash(false);
        assert_eq!(*example_outer.0.left_names(), p.permute(&outer_left_names));
        assert_eq!(*example_outer.0.right_names(), vec![(Dir::Out, 0)]);
        assert_eq!(
            example_outer.0.domain(),
            p.permute(&[true, true, false, false, true, false])
        );
        assert_eq!(*example_outer.0.codomain(), vec![true]);

        let op_subbed = example_outer.operadic_substitution(0, example_inner);
        assert_ok!(op_subbed);
        example_outer.0.assert_valid_nohash(false);
        assert_eq!(
            *example_outer.0.left_names(),
            vec![(Dir::Undirected, 1, 500)]
        );
        assert_eq!(*example_outer.0.domain(), vec![false]);
        assert_eq!(*example_outer.0.right_names(), vec![(Dir::Out, 0)]);
        assert_eq!(*example_outer.0.codomain(), vec![true]);
    }

    #[allow(clippy::items_after_statements, clippy::too_many_lines)]
    #[test]
    fn operadic_multiple() {
        use super::{Dir, WiringDiagram};
        use catgraph::assert_err;
        use catgraph::assert_ok;
        use catgraph::category::Composable;
        use catgraph::monoidal::SymmetricMonoidalMorphism;
        use catgraph::named_cospan::NamedCospan;
        use catgraph::operadic::Operadic;
        use either::Either::{Left, Right};
        use permutations::Permutation;

        type WireName = char;
        type CircleName = i32;

        let outer_label = |c: WireName| (Dir::Undirected, c);
        let outer_right_names: Vec<(_, WireName)> = ['a', 'b', 'c', 'd', 'e']
            .into_iter()
            .map(outer_label)
            .collect();

        let inner_label = |(w, c): (WireName, CircleName)| (Dir::Undirected, c, w);
        let outer_left_names: Vec<(_, CircleName, WireName)> = [
            ('r', 1),
            ('s', 1),
            ('t', 1),
            ('u', 2),
            ('v', 2),
            ('w', 3),
            ('x', 3),
            ('y', 3),
            ('z', 3),
        ]
        .into_iter()
        .map(inner_label)
        .collect();

        /*
        outer circle has 3 inner circles
        the first has 3 ports
            they are named r,s,t and connected to 2,1,3 in the middle
        the second has 2 ports
            they are named u,v and are connected to 4 and 3
        the third has 3 ports
            they are named w,x,y,z and are connected to 1,4,5,6
        there are no colors to the wires
        */
        let mut example_outer: WiringDiagram<_, _, _> = WiringDiagram::new(NamedCospan::new(
            vec![1, 0, 2, 3, 2, 0, 3, 4, 5],
            vec![0, 1, 2, 4, 5],
            vec![(); 6],
            outer_left_names.clone(),
            outer_right_names.clone(),
        ));
        /*
        permuting the domain of outer doesn't matter because the
        wires will be matched up by name not by index
        */
        use rand::seq::SliceRandom;
        let mut rng = StdRng::seed_from_u64(5002);
        let mut y: Vec<usize> = (0..9).collect();
        y.shuffle(&mut rng);
        let p1 = Permutation::try_from(&y).unwrap();
        example_outer.0.permute_side(&p1, false);
        example_outer.0.assert_valid_nohash(false);

        let mut y: Vec<usize> = (0..5).collect();
        y.shuffle(&mut rng);
        let p2 = Permutation::try_from(&y).unwrap();
        example_outer.0.permute_side(&p2, true);
        example_outer.0.assert_valid_nohash(false);

        assert_eq!(*example_outer.0.left_names(), p1.permute(&outer_left_names));
        assert_eq!(
            *example_outer.0.right_names(),
            p2.permute(&outer_right_names)
        );
        assert_eq!(example_outer.0.domain(), vec![(); 9]);
        assert_eq!(example_outer.0.codomain(), vec![(); 5]);

        let inner_1_right_names: Vec<(_, WireName)> = outer_left_names
            .iter()
            .filter_map(|(in_out, circle_name, wire_name)| {
                if *circle_name == 1 {
                    Some((in_out.flipped(), *wire_name))
                } else {
                    None
                }
            })
            .collect();
        let inner_1_left_names: Vec<(Dir, CircleName, WireName)> = vec![];

        /*
        first inner circle gets substituted for the r,s,t circle
        t goes to a unconnected middle
        r and s go to the same middle
        there are no internal circles
        */
        let example_inner_1: WiringDiagram<_, _, _> = WiringDiagram::new(NamedCospan::new(
            vec![],
            vec![1, 1, 0],
            vec![(), ()],
            inner_1_left_names,
            inner_1_right_names,
        ));

        let subbed = example_outer.operadic_substitution(1, example_inner_1);
        assert_ok!(subbed);

        example_outer.0.assert_valid_nohash(false);
        let expected_left_names = [
            (Dir::Undirected, 2, 'u'),
            (Dir::Undirected, 2, 'v'),
            (Dir::Undirected, 3, 'w'),
            (Dir::Undirected, 3, 'x'),
            (Dir::Undirected, 3, 'y'),
            (Dir::Undirected, 3, 'z'),
        ];
        let mut obs_left_names = example_outer.0.left_names().clone();
        obs_left_names.sort();
        assert_eq!(obs_left_names, expected_left_names.to_vec());
        assert_eq!(*example_outer.0.domain(), vec![(); 9 - 3]);
        let expected_right_names = [
            (Dir::Undirected, 'a'),
            (Dir::Undirected, 'b'),
            (Dir::Undirected, 'c'),
            (Dir::Undirected, 'd'),
            (Dir::Undirected, 'e'),
        ];
        let mut obs_right_names = example_outer.0.right_names().clone();
        obs_right_names.sort();
        assert_eq!(obs_right_names, expected_right_names.to_vec());
        assert_eq!(*example_outer.0.codomain(), vec![(); 5]);

        assert!(
            example_outer
                .0
                .map_to_same(Right(outer_label('a')), Right(outer_label('b')))
        );
        assert!(
            example_outer
                .0
                .map_to_same(Right(outer_label('a')), Left(inner_label(('w', 3))))
        );
        assert!(
            !example_outer
                .0
                .map_to_same(Right(outer_label('a')), Right(outer_label('c')))
        );
        assert!(
            example_outer
                .0
                .map_to_same(Right(outer_label('c')), Left(inner_label(('v', 2))))
        );
        assert!(
            !example_outer
                .0
                .map_to_same(Right(outer_label('a')), Right(outer_label('d')))
        );
        assert!(
            !example_outer
                .0
                .map_to_same(Right(outer_label('a')), Right(outer_label('e')))
        );
        assert!(
            !example_outer
                .0
                .map_to_same(Right(outer_label('c')), Right(outer_label('d')))
        );
        assert!(
            !example_outer
                .0
                .map_to_same(Right(outer_label('c')), Right(outer_label('e')))
        );
        assert!(
            !example_outer
                .0
                .map_to_same(Right(outer_label('d')), Right(outer_label('e')))
        );
        assert!(
            example_outer
                .0
                .map_to_same(Right(outer_label('d')), Left(inner_label(('y', 3))))
        );
        assert!(
            example_outer
                .0
                .map_to_same(Right(outer_label('e')), Left(inner_label(('z', 3))))
        );
        assert!(
            example_outer
                .0
                .map_to_same(Left(inner_label(('u', 2))), Left(inner_label(('x', 3))))
        );
        assert!(
            !example_outer
                .0
                .map_to_same(Left(inner_label(('u', 2))), Right(outer_label('b')))
        );
        assert!(
            !example_outer
                .0
                .map_to_same(Left(inner_label(('u', 2))), Right(outer_label('c')))
        );
        assert!(
            !example_outer
                .0
                .map_to_same(Left(inner_label(('u', 2))), Right(outer_label('d')))
        );
        assert!(
            !example_outer
                .0
                .map_to_same(Left(inner_label(('u', 2))), Right(outer_label('e')))
        );

        let inner_2_right_names: Vec<_> = outer_left_names
            .iter()
            .filter_map(|(in_out, circle_name, wire_name)| {
                if *circle_name == 2 {
                    Some((in_out.flipped(), *wire_name))
                } else {
                    None
                }
            })
            .collect();
        let inner_2_left_names = vec![inner_label(('q', 4))];

        /*
        first inner circle gets substituted for the u,v circle
        u and v go to the same middle
        that middle connects to a inner circle with port name q
        */
        let make_example_inner_2 = || {
            WiringDiagram::new(NamedCospan::new(
                vec![0],
                vec![0, 0],
                vec![()],
                inner_2_left_names.clone(),
                inner_2_right_names.clone(),
            ))
        };

        let subbed = example_outer.operadic_substitution(1, make_example_inner_2());
        assert_err!(subbed);
        let subbed = example_outer.operadic_substitution(3, make_example_inner_2());
        assert_err!(subbed);
        let subbed = example_outer.operadic_substitution(5, make_example_inner_2());
        assert_err!(subbed);
        let subbed = example_outer.operadic_substitution(2, make_example_inner_2());
        assert_ok!(subbed);

        example_outer.0.assert_valid_nohash(false);
        let expected_left_names = [
            (Dir::Undirected, 3, 'w'),
            (Dir::Undirected, 3, 'x'),
            (Dir::Undirected, 3, 'y'),
            (Dir::Undirected, 3, 'z'),
            (Dir::Undirected, 4, 'q'),
        ];
        let mut obs_left_names = example_outer.0.left_names().clone();
        obs_left_names.sort();
        assert_eq!(obs_left_names, expected_left_names.to_vec());
        assert_eq!(*example_outer.0.domain(), vec![(); 9 - 3 - 2 + 1]);
        let expected_right_names = [
            (Dir::Undirected, 'a'),
            (Dir::Undirected, 'b'),
            (Dir::Undirected, 'c'),
            (Dir::Undirected, 'd'),
            (Dir::Undirected, 'e'),
        ];
        let mut obs_right_names = example_outer.0.right_names().clone();
        obs_right_names.sort();
        assert_eq!(obs_right_names, expected_right_names.to_vec());
        assert_eq!(*example_outer.0.codomain(), vec![(); 5]);

        assert!(
            example_outer
                .0
                .map_to_same(Right(outer_label('a')), Right(outer_label('b')))
        );
        assert!(
            example_outer
                .0
                .map_to_same(Right(outer_label('a')), Left(inner_label(('w', 3))))
        );
        assert!(
            !example_outer
                .0
                .map_to_same(Right(outer_label('a')), Right(outer_label('c')))
        );
        assert!(
            example_outer
                .0
                .map_to_same(Right(outer_label('c')), Left(inner_label(('q', 4))))
        );
        assert!(
            !example_outer
                .0
                .map_to_same(Right(outer_label('a')), Right(outer_label('d')))
        );
        assert!(
            !example_outer
                .0
                .map_to_same(Right(outer_label('a')), Right(outer_label('e')))
        );
        assert!(
            !example_outer
                .0
                .map_to_same(Right(outer_label('c')), Right(outer_label('d')))
        );
        assert!(
            !example_outer
                .0
                .map_to_same(Right(outer_label('c')), Right(outer_label('e')))
        );
        assert!(
            !example_outer
                .0
                .map_to_same(Right(outer_label('d')), Right(outer_label('e')))
        );
        assert!(
            example_outer
                .0
                .map_to_same(Right(outer_label('d')), Left(inner_label(('y', 3))))
        );
        assert!(
            example_outer
                .0
                .map_to_same(Right(outer_label('e')), Left(inner_label(('z', 3))))
        );
        assert!(
            example_outer
                .0
                .map_to_same(Left(inner_label(('q', 4))), Left(inner_label(('x', 3))))
        );
        assert!(
            !example_outer
                .0
                .map_to_same(Left(inner_label(('q', 4))), Right(outer_label('b')))
        );
        assert!(
            example_outer
                .0
                .map_to_same(Left(inner_label(('q', 4))), Right(outer_label('c')))
        );
        assert!(
            !example_outer
                .0
                .map_to_same(Left(inner_label(('q', 4))), Right(outer_label('d')))
        );
        assert!(
            !example_outer
                .0
                .map_to_same(Left(inner_label(('q', 4))), Right(outer_label('e')))
        );
    }

    /// Helper: build a simple `WiringDiagram` with no inner circles,
    /// 3 middle nodes (labels true/true/false), and 3 right-side boundary nodes.
    fn simple_diagram() -> super::WiringDiagram<bool, (), usize> {
        use super::{Dir, WiringDiagram};
        use catgraph::named_cospan::NamedCospan;
        // middle nodes: 0=true, 1=true, 2=false
        // right boundary: node 0 -> middle 0, node 1 -> middle 1, node 2 -> middle 2
        WiringDiagram::new(NamedCospan::new(
            vec![],
            vec![0, 1, 2],
            vec![true, true, false],
            vec![],
            vec![(Dir::In, 0), (Dir::Out, 1), (Dir::In, 2)],
        ))
    }

    #[test]
    fn add_boundary_node_unconnected_right() {
        use super::Dir;
        use either::Either::Right;
        let mut wd = simple_diagram();
        assert_eq!(wd.0.right_names().len(), 3);
        // Add a new unconnected boundary node on the right side
        wd.add_boundary_node_unconnected(true, Right((Dir::Out, 99)));
        assert_eq!(wd.0.right_names().len(), 4);
        assert!(wd.0.right_names().contains(&(Dir::Out, 99)));
        wd.0.assert_valid_nohash(false);
    }

    #[test]
    fn add_boundary_node_unconnected_left() {
        use super::Dir;
        use either::Either::Left;
        let mut wd = simple_diagram();
        assert_eq!(wd.0.left_names().len(), 0);
        // Add a new unconnected boundary node on the left (inner) side
        wd.add_boundary_node_unconnected(false, Left((Dir::In, (), 42)));
        assert_eq!(wd.0.left_names().len(), 1);
        assert_eq!(wd.0.left_names()[0], (Dir::In, (), 42));
        wd.0.assert_valid_nohash(false);
    }

    #[test]
    fn delete_boundary_node_right() {
        use super::Dir;
        use either::Either::Right;
        let mut wd = simple_diagram();
        assert_eq!(wd.0.right_names().len(), 3);
        assert!(wd.0.right_names().contains(&(Dir::Out, 1)));
        // Delete the boundary node named (Dir::Out, 1)
        wd.delete_boundary_node(Right((Dir::Out, 1)));
        assert_eq!(wd.0.right_names().len(), 2);
        assert!(!wd.0.right_names().contains(&(Dir::Out, 1)));
        wd.0.assert_valid_nohash(false);
    }

    #[test]
    fn delete_boundary_node_left() {
        use super::Dir;
        use either::Either::Left;
        let mut wd = simple_diagram();
        // First add a left-side node so we can delete it
        wd.add_boundary_node_unconnected(true, Left((Dir::In, (), 7)));
        assert_eq!(wd.0.left_names().len(), 1);
        wd.delete_boundary_node(Left((Dir::In, (), 7)));
        assert_eq!(wd.0.left_names().len(), 0);
        wd.0.assert_valid_nohash(false);
    }

    #[test]
    fn delete_nonexistent_boundary_node_is_noop() {
        use super::Dir;
        use either::Either::Right;
        let mut wd = simple_diagram();
        let names_before = wd.0.right_names().clone();
        // Deleting a node that doesn't exist should be a no-op (just a warning)
        wd.delete_boundary_node(Right((Dir::Out, 999)));
        assert_eq!(*wd.0.right_names(), names_before);
    }

    #[test]
    fn connect_pair_same_type() {
        use super::Dir;
        use either::Either::Right;
        let mut wd = simple_diagram();
        // Nodes 0 and 1 both have type true but map to different middles (0 and 1)
        assert!(!wd.0.map_to_same(Right((Dir::In, 0)), Right((Dir::Out, 1))));
        // Connect them — they share the same label (true), so this should succeed
        wd.connect_pair(Right((Dir::In, 0)), Right((Dir::Out, 1)));
        assert!(wd.0.map_to_same(Right((Dir::In, 0)), Right((Dir::Out, 1))));
        // Node 2 (type false) should still be separate
        assert!(!wd.0.map_to_same(Right((Dir::In, 0)), Right((Dir::In, 2))));
    }

    #[test]
    fn connect_pair_different_type_is_noop() {
        use super::Dir;
        use either::Either::Right;
        let mut wd = simple_diagram();
        // Nodes 0 (type true) and 2 (type false) have different labels
        assert!(!wd.0.map_to_same(Right((Dir::In, 0)), Right((Dir::In, 2))));
        // Connecting nodes with different labels should be a no-op
        wd.connect_pair(Right((Dir::In, 0)), Right((Dir::In, 2)));
        assert!(!wd.0.map_to_same(Right((Dir::In, 0)), Right((Dir::In, 2))));
    }

    #[test]
    fn connect_pair_nonexistent_is_noop() {
        use super::Dir;
        use either::Either::Right;
        let mut wd = simple_diagram();
        let names_before = wd.0.right_names().clone();
        // One node doesn't exist — should be a no-op
        wd.connect_pair(Right((Dir::In, 0)), Right((Dir::Out, 999)));
        assert_eq!(*wd.0.right_names(), names_before);
    }

    #[test]
    fn change_boundary_node_name_right() {
        use super::Dir;
        use either::Either::Right;
        let mut wd = simple_diagram();
        assert!(wd.0.right_names().contains(&(Dir::In, 0)));
        assert!(!wd.0.right_names().contains(&(Dir::Out, 50)));
        // Rename (Dir::In, 0) -> (Dir::Out, 50)
        wd.change_boundary_node_name(Right(((Dir::In, 0), (Dir::Out, 50))));
        assert!(!wd.0.right_names().contains(&(Dir::In, 0)));
        assert!(wd.0.right_names().contains(&(Dir::Out, 50)));
        wd.0.assert_valid_nohash(false);
    }

    #[test]
    fn change_boundary_node_name_left() {
        use super::Dir;
        use super::WiringDiagram;
        use catgraph::named_cospan::NamedCospan;
        use either::Either::Left;
        // Build a diagram with left-side nodes
        let mut wd: WiringDiagram<bool, i32, usize> = WiringDiagram::new(NamedCospan::new(
            vec![0, 1],
            vec![0],
            vec![true, false],
            vec![(Dir::In, 1, 10), (Dir::Out, 2, 20)],
            vec![(Dir::In, 0)],
        ));
        assert!(wd.0.left_names().contains(&(Dir::In, 1, 10)));
        // Rename (Dir::In, 1, 10) -> (Dir::Undirected, 1, 10)
        wd.change_boundary_node_name(Left(((Dir::In, 1, 10), (Dir::Undirected, 1, 10))));
        assert!(!wd.0.left_names().contains(&(Dir::In, 1, 10)));
        assert!(wd.0.left_names().contains(&(Dir::Undirected, 1, 10)));
        wd.0.assert_valid_nohash(false);
    }

    #[test]
    fn change_boundary_node_name_nonexistent_is_noop() {
        use super::Dir;
        use either::Either::Right;
        let mut wd = simple_diagram();
        let names_before = wd.0.right_names().clone();
        // Rename a node that doesn't exist — should be a no-op
        wd.change_boundary_node_name(Right(((Dir::Out, 999), (Dir::In, 888))));
        assert_eq!(*wd.0.right_names(), names_before);
    }

    #[test]
    fn toggle_orientation_right() {
        use super::Dir;
        let mut wd = simple_diagram();
        // Before: In, Out, In
        assert_eq!(wd.0.right_names()[0].0, Dir::In);
        assert_eq!(wd.0.right_names()[1].0, Dir::Out);
        assert_eq!(wd.0.right_names()[2].0, Dir::In);
        // Toggle right side (of_left_side = false)
        wd.toggle_orientation(false);
        // After: Out, In, Out
        assert_eq!(wd.0.right_names()[0].0, Dir::Out);
        assert_eq!(wd.0.right_names()[1].0, Dir::In);
        assert_eq!(wd.0.right_names()[2].0, Dir::Out);
        wd.0.assert_valid_nohash(false);
    }

    #[test]
    fn toggle_orientation_left() {
        use super::Dir;
        use super::WiringDiagram;
        use catgraph::named_cospan::NamedCospan;
        let mut wd: WiringDiagram<bool, i32, usize> = WiringDiagram::new(NamedCospan::new(
            vec![0, 1],
            vec![],
            vec![true, false],
            vec![(Dir::In, 1, 10), (Dir::Out, 2, 20)],
            vec![],
        ));
        assert_eq!(wd.0.left_names()[0].0, Dir::In);
        assert_eq!(wd.0.left_names()[1].0, Dir::Out);
        // Toggle left side (of_left_side = true)
        wd.toggle_orientation(true);
        assert_eq!(wd.0.left_names()[0].0, Dir::Out);
        assert_eq!(wd.0.left_names()[1].0, Dir::In);
        wd.0.assert_valid_nohash(false);
    }

    #[test]
    fn toggle_orientation_undirected_stays() {
        use super::Dir;
        use super::WiringDiagram;
        use catgraph::named_cospan::NamedCospan;
        let mut wd: WiringDiagram<bool, (), usize> = WiringDiagram::new(NamedCospan::new(
            vec![],
            vec![0],
            vec![true],
            vec![],
            vec![(Dir::Undirected, 0)],
        ));
        assert_eq!(wd.0.right_names()[0].0, Dir::Undirected);
        wd.toggle_orientation(false);
        // Undirected stays Undirected after toggle
        assert_eq!(wd.0.right_names()[0].0, Dir::Undirected);
    }

    #[test]
    fn toggle_orientation_double_toggle_restores() {
        let mut wd = simple_diagram();
        let names_before = wd.0.right_names().clone();
        wd.toggle_orientation(false);
        wd.toggle_orientation(false);
        // Double toggle should restore original orientations
        assert_eq!(*wd.0.right_names(), names_before);
    }

    // ── Composable + Monoidal trait tests ──

    /// Build two WDs with matching codomain→domain types and compose them.
    /// The first has inner→outer going through 2 middle nodes (true, false).
    /// The second has inner→outer going through 2 middle nodes (true, false).
    #[test]
    fn compose_two_compatible_wiring_diagrams() {
        use super::{Dir, WiringDiagram};
        use catgraph::category::Composable;
        use catgraph::named_cospan::NamedCospan;

        // WD1: domain [true, false], codomain [true, false]
        // Left ports on circle 0, right ports unadorned.
        let wd1: WiringDiagram<bool, i32, usize> = WiringDiagram::new(NamedCospan::new(
            vec![0, 1],        // left → middle
            vec![0, 1],        // right → middle
            vec![true, false], // middle types
            vec![(Dir::In, 0, 10), (Dir::Out, 0, 11)],
            vec![(Dir::Out, 0), (Dir::In, 1)],
        ));

        // WD2: domain [true, false], codomain [true]
        // Both domain ports map to a single middle node of type true.
        let wd2: WiringDiagram<bool, i32, usize> = WiringDiagram::new(NamedCospan::new(
            vec![0, 1],
            vec![0],
            vec![true, false],
            vec![(Dir::In, 1, 20), (Dir::Out, 1, 21)],
            vec![(Dir::Out, 0)],
        ));

        assert_eq!(wd1.codomain(), vec![true, false]);
        assert_eq!(wd2.domain(), vec![true, false]);

        let composed = wd1.compose(&wd2);
        assert!(
            composed.is_ok(),
            "compose of type-compatible WDs must succeed"
        );
        let composed = composed.unwrap();
        // Composed domain comes from wd1's left side, codomain from wd2's right side.
        assert_eq!(composed.domain(), vec![true, false]);
        assert_eq!(composed.codomain(), vec![true]);
    }

    /// Composing two WDs whose codomain/domain types mismatch yields an error.
    #[test]
    fn compose_type_mismatch_fails() {
        use super::{Dir, WiringDiagram};
        use catgraph::category::Composable;
        use catgraph::named_cospan::NamedCospan;

        // WD1 codomain: [true, false]
        let wd1: WiringDiagram<bool, i32, usize> = WiringDiagram::new(NamedCospan::new(
            vec![0],
            vec![0, 1],
            vec![true, false],
            vec![(Dir::In, 0, 10)],
            vec![(Dir::Out, 0), (Dir::In, 1)],
        ));

        // WD2 domain: [false, false] — does not match WD1 codomain
        let wd2: WiringDiagram<bool, i32, usize> = WiringDiagram::new(NamedCospan::new(
            vec![0, 0],
            vec![0],
            vec![false],
            vec![(Dir::In, 1, 20), (Dir::Out, 1, 21)],
            vec![(Dir::Out, 0)],
        ));

        let result = wd1.compose(&wd2);
        assert!(result.is_err(), "compose with mismatched types must fail");
    }

    /// Tensor product of two WDs concatenates domain and codomain.
    #[test]
    fn monoidal_product_of_wiring_diagrams() {
        use super::{Dir, WiringDiagram};
        use catgraph::category::Composable;
        use catgraph::monoidal::Monoidal;
        use catgraph::named_cospan::NamedCospan;

        // WD_A: domain [true], codomain [true]
        let wd_a: WiringDiagram<bool, i32, usize> = WiringDiagram::new(NamedCospan::new(
            vec![0],
            vec![0],
            vec![true],
            vec![(Dir::In, 0, 10)],
            vec![(Dir::Out, 0)],
        ));

        // WD_B: domain [false], codomain [false]
        let wd_b: WiringDiagram<bool, i32, usize> = WiringDiagram::new(NamedCospan::new(
            vec![0],
            vec![0],
            vec![false],
            vec![(Dir::In, 1, 20)],
            vec![(Dir::Out, 1)],
        ));

        let mut combined = wd_a;
        combined.monoidal(wd_b);

        // Tensor product concatenates domains and codomains.
        assert_eq!(combined.domain(), vec![true, false]);
        assert_eq!(combined.codomain(), vec![true, false]);
    }

    /// Regression: `Operadic` works with a non-`Copy` `InterCircle`.
    /// Mirrors the `operadic` test fixture but parameterizes `CircleName` by
    /// `String` rather than `i32`, exercising the loosened `InterCircle: Clone`
    /// bound (catgraph `find_nodes_by_name_predicate` /
    /// `NamedCospan::identity` Clone-bound widening).
    #[allow(clippy::items_after_statements)]
    #[test]
    fn operadic_with_clone_only_intercircle() {
        use super::{Dir, WiringDiagram};
        use catgraph::assert_ok;
        use catgraph::named_cospan::NamedCospan;
        use catgraph::operadic::Operadic;

        type WireName = usize;
        // String is Clone but not Copy.
        type CircleName = String;

        let inner_right_names: Vec<(Dir, WireName)> = vec![
            (Dir::In, 0),
            (Dir::Out, 1),
            (Dir::In, 2),
            (Dir::Out, 3),
            (Dir::Out, 4),
        ];
        let circle_a: CircleName = "alpha".to_owned();
        let circle_b: CircleName = "beta".to_owned();
        let mut outer_left_names: Vec<(Dir, CircleName, WireName)> = inner_right_names
            .iter()
            .map(|(orient, name)| (orient.flipped(), circle_a.clone(), *name))
            .collect();
        outer_left_names.push((Dir::Undirected, circle_b.clone(), 500));

        let example_inner: WiringDiagram<_, CircleName, WireName> =
            WiringDiagram::new(NamedCospan::new(
                vec![],
                vec![0, 1, 2, 2, 0],
                vec![true, true, false],
                vec![],
                inner_right_names,
            ));
        let mut example_outer: WiringDiagram<_, _, _> = WiringDiagram::new(NamedCospan::new(
            vec![0, 0, 1, 1, 0, 1],
            vec![0],
            vec![true, false],
            outer_left_names,
            vec![(Dir::Out, 0)],
        ));
        let op_subbed = example_outer.operadic_substitution(circle_a, example_inner);
        assert_ok!(op_subbed);
        assert_eq!(
            *example_outer.0.left_names(),
            vec![(Dir::Undirected, circle_b, 500)]
        );
    }
}
