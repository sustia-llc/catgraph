//! `agent_hypergraph` — the K1 CRUD [`Hypergraph`] as an agent-coalition
//! registry, walked through the full coalition lifecycle.
//!
//! This is the worked companion to `catgraph_applied::hypergraph` (#23): an
//! agent registry realized as `Hypergraph<&str, u8>` where **vertices are
//! agents** (weight = a small capability/priority tag) and **hyperedges are
//! coalitions** (an ordered, duplicate-allowing member list, weight = a
//! coalition tag). It exercises exactly the operations the downstream koalisi
//! coalition layer (sustia-llc/koalisi#4) calls, in the shape of the K1
//! lifecycle:
//!
//! - **member read** — `get_hyperedge_vertices` (owned, for read-modify-write)
//!   and the borrowing `hyperedge_vertices`;
//! - **join** — `update_hyperedge_vertices` with an appended member, including
//!   the documented divergence: a no-op re-join of an already-present agent
//!   returns `Ok` (yamafaktory would error);
//! - **leave** — a filtered `update_hyperedge_vertices`;
//! - **merge** — `join_hyperedges` (the first edge's weight survives);
//! - **dissolve** — `remove_hyperedge`;
//! - **agent-removal cascade** — `remove_vertex` (a sole-vertex coalition
//!   disappears; a multi-member coalition is filtered);
//! - **index stability** — removed indices error forever; fresh adds get fresh
//!   indices (never reused);
//! - **categorical view** — `hyperedge_as_cospan` = the identity cospan over
//!   the member index list.
//!
//! This example depends on **catgraph-applied only** (plus its own catgraph
//! dependency) — it deliberately does *not* touch catgraph-magnitude: the
//! magnitude consumer path (`coalition_value`) lives one crate downstream and
//! is exercised by `catgraph-magnitude/tests/coalition_consumer.rs`.
//!
//! Run: `cargo run -p catgraph-applied --example agent_hypergraph`

use catgraph_applied::{Hypergraph, VertexIndex};

fn main() {
    println!("=== agent_hypergraph — coalition lifecycle over the K1 Hypergraph ===\n");

    // Registry: six agents (weight = a small capability/priority tag).
    let mut g: Hypergraph<&str, u8> = Hypergraph::new();
    let alice = g.add_vertex("alice");
    let bob = g.add_vertex("bob");
    let carol = g.add_vertex("carol");
    let dave = g.add_vertex("dave");
    let erin = g.add_vertex("erin");
    let frank = g.add_vertex("frank");
    // Fresh, monotonic indices in add order: alice..frank = v0..v5.
    assert_eq!(
        [alice, bob, carol, dave, erin, frank],
        [
            VertexIndex(0),
            VertexIndex(1),
            VertexIndex(2),
            VertexIndex(3),
            VertexIndex(4),
            VertexIndex(5),
        ]
    );
    println!("Registered 6 agents: alice..frank at indices v0..v5.");

    // Two coalitions as hyperedges, plus a sole-member coalition and a
    // frank-containing multi-member coalition to drive the cascade later.
    let c_a = g.add_hyperedge(vec![alice, bob, carol], 10).unwrap();
    let c_b = g.add_hyperedge(vec![dave, erin], 20).unwrap();
    let c_solo = g.add_hyperedge(vec![frank], 30).unwrap();
    let c_multi = g.add_hyperedge(vec![alice, frank, bob], 40).unwrap();
    println!(
        "Formed coalitions: A{:?}=alice,bob,carol | B{:?}=dave,erin | \
         solo{:?}=frank | multi{:?}=alice,frank,bob\n",
        c_a.get(),
        c_b.get(),
        c_solo.get(),
        c_multi.get()
    );

    // ---- member read: owned (read-modify-write) vs borrowing accessor -------
    println!("§ member read");
    let owned = g.get_hyperedge_vertices(c_a).unwrap();
    let borrowed = g.hyperedge_vertices(c_a).unwrap();
    assert_eq!(owned.as_slice(), borrowed);
    assert_eq!(owned, vec![alice, bob, carol]);
    println!("  A members (owned == borrowed) = {owned:?}\n");

    // ---- join: append a member; then the no-op re-join returns Ok -----------
    println!("§ join (carol joins B, then re-joins — the yamafaktory divergence)");
    let mut with_carol = g.get_hyperedge_vertices(c_b).unwrap();
    if !with_carol.contains(&carol) {
        with_carol.push(carol);
    }
    g.update_hyperedge_vertices(c_b, with_carol).unwrap();
    assert_eq!(
        g.get_hyperedge_vertices(c_b).unwrap(),
        vec![dave, erin, carol]
    );

    // Re-join carol (already a member): the appended list is UNCHANGED, and the
    // update still returns Ok — the divergence that makes re-join idempotent.
    // yamafaktory v4.2.0 would return Err(...Unchanged) on an unchanged list.
    let current = g.get_hyperedge_vertices(c_b).unwrap();
    let mut rejoin = current.clone();
    if !rejoin.contains(&carol) {
        rejoin.push(carol); // no-op: carol already present
    }
    assert_eq!(rejoin, current, "re-join leaves the member list unchanged");
    assert!(
        g.update_hyperedge_vertices(c_b, rejoin).is_ok(),
        "unchanged-list update must return Ok (re-join idempotency)"
    );
    println!(
        "  B members after join + no-op re-join = {:?}\n",
        g.get_hyperedge_vertices(c_b).unwrap()
    );

    // ---- leave: filtered update ---------------------------------------------
    println!("§ leave (dave leaves B)");
    let remaining: Vec<VertexIndex> = g
        .get_hyperedge_vertices(c_b)
        .unwrap()
        .into_iter()
        .filter(|&v| v != dave)
        .collect();
    g.update_hyperedge_vertices(c_b, remaining).unwrap();
    assert_eq!(g.get_hyperedge_vertices(c_b).unwrap(), vec![erin, carol]);
    println!(
        "  B members after dave leaves = {:?}\n",
        g.get_hyperedge_vertices(c_b).unwrap()
    );

    // ---- merge: join_hyperedges — first edge's weight survives ---------------
    println!("§ merge (A ⊔ B → A; A's weight survives)");
    let w_a_before = g.get_hyperedge_weight(c_a).unwrap();
    let merged = g.join_hyperedges(&[c_a, c_b]).unwrap();
    assert_eq!(merged, c_a, "the joined edge is the FIRST edge");
    // A's list = original A ++ B's list, in argument order (duplicates kept).
    assert_eq!(
        g.get_hyperedge_vertices(c_a).unwrap(),
        vec![alice, bob, carol, erin, carol]
    );
    assert_eq!(
        g.get_hyperedge_weight(c_a).unwrap(),
        w_a_before,
        "the FIRST edge's weight is retained; B's weight is discarded"
    );
    assert!(
        g.get_hyperedge_vertices(c_b).is_err(),
        "B was consumed by the merge"
    );
    println!(
        "  merged A members = {:?}, weight = {} (B's weight 20 discarded)\n",
        g.get_hyperedge_vertices(c_a).unwrap(),
        g.get_hyperedge_weight(c_a).unwrap()
    );

    // ---- dissolve: remove_hyperedge -----------------------------------------
    println!("§ dissolve (remove coalition A)");
    g.remove_hyperedge(c_a).unwrap();
    assert!(g.get_hyperedge_vertices(c_a).is_err());
    println!("  A dissolved; its index errors from here on.\n");

    // ---- agent-removal cascade: sole-vertex edge gone; multi-member filtered -
    println!("§ agent removal cascade (frank leaves the registry entirely)");
    g.remove_vertex(frank).unwrap();
    assert!(
        g.get_hyperedge_vertices(c_solo).is_err(),
        "the sole-vertex coalition {frank} disappears"
    );
    assert_eq!(
        g.get_hyperedge_vertices(c_multi).unwrap(),
        vec![alice, bob],
        "the multi-member coalition is filtered (frank removed, order preserved)"
    );
    println!(
        "  solo coalition removed; multi coalition filtered to {:?}\n",
        g.get_hyperedge_vertices(c_multi).unwrap()
    );

    // ---- index stability: removed indices error forever; fresh adds are fresh
    println!("§ index stability (never-reused indices)");
    assert!(
        g.get_vertex_weight(frank).is_err(),
        "frank's removed index errors forever"
    );
    let grace = g.add_vertex("grace");
    assert_eq!(
        grace,
        VertexIndex(6),
        "a fresh add gets index v6 — NOT frank's reclaimed v5"
    );
    assert_ne!(grace, frank);
    assert!(
        g.get_hyperedge_vertices(c_a).is_err(),
        "the dissolved coalition A's index also errors forever"
    );
    println!(
        "  frank (v5) errors forever; grace gets fresh v{}.\n",
        grace.get()
    );

    // ---- categorical view: hyperedge_as_cospan = identity over member indices
    println!("§ categorical view (hyperedge_as_cospan on the surviving multi)");
    let cospan = g.hyperedge_as_cospan(c_multi).unwrap();
    assert_eq!(
        cospan.middle(),
        &[alice, bob],
        "the middle is the ordered member INDEX list (identities, not weights)"
    );
    assert!(cospan.is_left_identity(), "left leg is the identity 0..k");
    assert!(cospan.is_right_identity(), "right leg is the identity 0..k");
    println!(
        "  cospan middle = {:?}, both legs identity ⇒ identity cospan.\n",
        cospan.middle()
    );

    println!("All agent_hypergraph assertions hold.");
}
