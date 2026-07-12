//! The physics-adjacent wiring-diagram story: **cospan interconnection via the
//! S4 Frobenius layer, checked semantically against `MatKron`.**
//!
//! Spiders, cups, and caps are the special commutative Frobenius (SCFM)
//! generators of the hypergraph category `Cospan` (F&S 2019 *Hypergraph
//! Categories*). They express wiring-diagram interconnection: a spider is an
//! `m`-in/`n`-out junction where all legs carry the same value; a cup/cap bends
//! a wire, giving the compact-closed (bidirectional) structure. We build two
//! wiring identities and confirm them **semantically** by mapping each side to
//! its `MatKron<i64>` image on `R^d` via [`to_mat_kron`] and comparing.
//!
//! # Boundary: what `to_mat_kron` is, and is NOT
//!
//! [`to_mat_kron`] is a **sound semantic checker** (F&S 2019 Prop 3.8): equal
//! `MatKron` images witness equality *in the model* `MatKron(R)`. It is **not a
//! `CompleteFunctor`** — it does not decide the free theory, and it has a
//! restricted domain:
//!
//! - `User(g)` leaves are **out of its domain**: a wire decorated by a user
//!   signature generator has no canonical `MatKron` image, so [`to_mat_kron`]
//!   returns [`SyntaxError::NonFrobenius`]. We demonstrate this below.
//! - The layer is **monochromatic**: one wire colour `Λ = {•}`, one spider
//!   family. F&S 2019 Thm 3.14's fully *colored* generality (a distinct spider
//!   per colour) is out of scope here — colored props are tracked as #79.
//!
//! Run: `cargo run -p catgraph-syntax --example frobenius_wiring`

use std::error::Error;

use catgraph_applied::mat_kron::MatKron;
use catgraph_applied::prop::{Free, PropExpr, PropSignature};
use catgraph_syntax::errors::SyntaxError;
use catgraph_syntax::frobenius::{FrobeniusOr, cap, cup, spider, to_mat_kron};

/// A small demo user signature (mirroring `tests/common`'s `Sig`). Here it only
/// fixes the `G` in `FrobeniusOr<G>`; its single generator is a wire decorator
/// that sits **outside** `to_mat_kron`'s Frobenius domain — used below to show
/// the `NonFrobenius` boundary concretely.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Sig {
    /// `signal : 1 → 1` — a decorated wire, not a Frobenius generator.
    Signal,
}

impl PropSignature for Sig {
    fn source(&self) -> usize {
        match self {
            Sig::Signal => 1,
        }
    }
    fn target(&self) -> usize {
        match self {
            Sig::Signal => 1,
        }
    }
}

/// Map a Frobenius term to its `MatKron<i64>` image on `R^d` (turbofish shim,
/// the `tests/frobenius.rs` idiom).
fn mk(expr: &PropExpr<FrobeniusOr<Sig>>, d: usize) -> Result<MatKron<i64>, SyntaxError> {
    to_mat_kron::<Sig, i64>(expr, d)
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== Frobenius wiring diagrams, checked in MatKron(R) ===\n");

    // ---- (a) The compact-closed snake / zig-zag identity ---------------------
    // (cup ⊗ id) ; (id ⊗ cap) ≡ id(1): pulling a wire taut through a cup then a
    // cap is the plain identity wire. cup : 0 → 2, cap : 2 → 0.
    let id1 = Free::<FrobeniusOr<Sig>>::identity(1);
    let snake = Free::compose(
        Free::tensor(cup::<Sig>(), id1.clone()), // 0+1 = 1 → 2+1 = 3
        Free::tensor(id1, cap::<Sig>()),         // 1+2 = 3 → 1+0 = 1
    )?;
    println!(
        "snake  (cup ⊗ id) ; (id ⊗ cap) : {} → {}",
        snake.source(),
        snake.target()
    );
    for d in [2_usize, 3] {
        assert_eq!(
            mk(&snake, d)?,
            MatKron::<i64>::identity(d),
            "snake ≡ id(1) at d={d}"
        );
        println!("  d={d}: to_mat_kron(snake) == MatKron::identity({d})  ✓");
    }

    // ---- (b) Spider fusion ---------------------------------------------------
    // spider(m,k) ; spider(k,n) ≡ spider(m,n) for k ≥ 1: fusing two junctions
    // along a shared wire is one wider junction (the SCFM "all legs equal" law).
    let fused = Free::compose(spider::<Sig>(3, 1), spider::<Sig>(1, 2))?; // 3 → 1 → 2
    let direct = spider::<Sig>(3, 2);
    println!("\nfusion spider(3,1) ; spider(1,2) ≡ spider(3,2):");
    for d in [2_usize, 3] {
        assert_eq!(mk(&fused, d)?, mk(&direct, d)?, "spider fusion at d={d}");
        println!("  d={d}: to_mat_kron(fused) == to_mat_kron(direct)  ✓");
    }

    // ---- (c) The domain boundary: User(g) is NonFrobenius --------------------
    // A user-decorated wire has no canonical MatKron image. `to_mat_kron` reports
    // it rather than inventing one — this is the sound-checker's domain edge.
    let decorated = Free::generator(FrobeniusOr::User(Sig::Signal));
    match mk(&decorated, 2) {
        Err(SyntaxError::NonFrobenius { generator }) => {
            println!("\nboundary: to_mat_kron(User(Signal)) → NonFrobenius {{ {generator} }}");
            println!("  (User leaves are out of the monochromatic Frobenius domain — see #79)");
        }
        other => return Err(format!("expected NonFrobenius, got {other:?}").into()),
    }

    println!("\nEqual MatKron images witness equality in the model, not a complete decision.");
    Ok(())
}
