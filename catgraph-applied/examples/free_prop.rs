//! F&S *Seven Sketches* §5.2 worked demo — `Free(G)`.
//!
//! Build the free prop on a signature `G = {Mul : 2 → 1, Unit : 0 → 1}` —
//! a binary multiplication and a constant — then form the morphism
//! `(Unit ⊗ id₁) ; Mul : 1 → 1` to exhibit compose + tensor + arity
//! tracking.

use catgraph_applied::prop::{Free, PropSignature};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum Sig {
    Mul,
    Unit,
}

impl PropSignature for Sig {
    fn source(&self) -> usize {
        match self {
            Sig::Mul => 2,
            Sig::Unit => 0,
        }
    }
    fn target(&self) -> usize {
        match self {
            Sig::Mul | Sig::Unit => 1,
        }
    }
}

fn main() {
    let unit = Free::<Sig>::generator(Sig::Unit); // 0 → 1
    let id1 = Free::<Sig>::identity(1); //           1 → 1
    let left = Free::tensor(unit, id1); //           1 → 2
    let mul = Free::<Sig>::generator(Sig::Mul); //   2 → 1
    let composed = Free::compose(left, mul).expect("(Unit ⊗ id₁) ; Mul arities match");
    println!(
        "Free(G) morphism (Unit ⊗ id₁) ; Mul has source = {} and target = {}",
        composed.source(),
        composed.target(),
    );
    assert_eq!(composed.source(), 1);
    assert_eq!(composed.target(), 1);
}
