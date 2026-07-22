//! Back-compat re-export shim for the multi-prime CRT integer SNF lift.
//!
//! The `#35` clarity refactor split this module into
//! [`crate::snf::crt`] (prime selection + CRT reconstruction) and
//! [`crate::snf::integer`] (Hadamard bound + integer-SNF composition). The
//! original `snf::crt_lift::*` paths are preserved here so downstream
//! callers keep compiling unchanged.

pub use crate::snf::crt::{crt_reconstruct_signed, select_primes_for_bound};
pub use crate::snf::integer::{
    hadamard_bound, hadamard_bound_integer, hadamard_bound_matr, smith_normal_form_integer,
};
