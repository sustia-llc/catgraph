//! Worked example: F-decorated cospans with F(n) = edge set on n vertices.
//!
//! Minimal form of the textbook's Circ example (§6.4 Ex 6.79–6.86): each
//! decoration is a set of undirected edges between the apex vertices,
//! representing a "circuit" built from resistors. Both parallel (monoidal
//! product) and series (composition with pushforward) compositions are
//! demonstrated; series composition coequalizes the shared boundary
//! vertex and pushes edge endpoints forward through the quotient via
//! `D::pushforward`.

use catgraph::category::Composable;
use catgraph::cospan::Cospan;
use catgraph::monoidal::Monoidal;
use catgraph_applied::decorated_cospan::{DecoratedCospan, Decoration};

#[derive(Clone, Debug, PartialEq)]
struct EdgeSet {
    n: usize,
    edges: Vec<(usize, usize)>,
}

struct Circuit;

impl Decoration for Circuit {
    type Apex = EdgeSet;
    fn empty(n: usize) -> EdgeSet {
        EdgeSet { n, edges: vec![] }
    }
    fn combine(a: EdgeSet, b: EdgeSet) -> EdgeSet {
        let shift = a.n;
        let mut edges = a.edges;
        edges.extend(b.edges.into_iter().map(|(u, v)| (u + shift, v + shift)));
        EdgeSet {
            n: a.n + b.n,
            edges,
        }
    }
    fn pushforward(d: EdgeSet, quotient: &[usize]) -> EdgeSet {
        let new_n = quotient.iter().copied().max().map_or(0, |m| m + 1);
        EdgeSet {
            n: new_n,
            edges: d
                .edges
                .into_iter()
                .map(|(u, v)| (quotient[u], quotient[v]))
                .collect(),
        }
    }
}

fn main() {
    // Parallel composition — no pushforward needed (apex is disjoint union).
    let c1 = Cospan::<usize>::new(vec![0], vec![1], vec![0, 0]);
    let circ1 = DecoratedCospan::<usize, Circuit>::new(
        c1,
        EdgeSet {
            n: 2,
            edges: vec![(0, 1)],
        },
    );
    let c2 = Cospan::<usize>::new(vec![0], vec![1], vec![0, 0]);
    let circ2 = DecoratedCospan::<usize, Circuit>::new(
        c2,
        EdgeSet {
            n: 2,
            edges: vec![(0, 1)],
        },
    );
    let mut parallel = circ1;
    parallel.monoidal(circ2);
    println!(
        "parallel R1 || R1: {} edges over {} apex vertices",
        parallel.decoration.edges.len(),
        parallel.cospan.middle().len()
    );

    // Series composition — pushforward glues the shared boundary vertex.
    let c3 = Cospan::<usize>::new(vec![0], vec![1], vec![0, 0]);
    let circ3 = DecoratedCospan::<usize, Circuit>::new(
        c3,
        EdgeSet {
            n: 2,
            edges: vec![(0, 1)],
        },
    );
    let c4 = Cospan::<usize>::new(vec![0], vec![1], vec![0, 0]);
    let circ4 = DecoratedCospan::<usize, Circuit>::new(
        c4,
        EdgeSet {
            n: 2,
            edges: vec![(0, 1)],
        },
    );
    let series = circ3.compose(&circ4).expect("series composition");
    println!(
        "series R1 -> R1: {} edges over {} apex vertices",
        series.decoration.edges.len(),
        series.cospan.middle().len()
    );
    assert_eq!(series.decoration.edges.len(), 2);
    assert_eq!(series.cospan.middle().len(), 3);
}
