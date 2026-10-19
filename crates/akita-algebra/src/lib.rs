//! Concrete algebra backends and arithmetic building blocks.
//!
//! This module includes multilinear polynomial, eq-polynomial, and offset-eq
//! utilities.
//!
//! Concrete fields, field packing, the cyclotomic ring, NTT/CRT arithmetic,
//! and the smooth FFT live in `jolt-field`. Sparse Fiat–Shamir challenge
//! representations and samplers live in `akita-challenges`.

#![warn(missing_docs)]
#![warn(unreachable_pub)]

pub mod eq_poly;
pub mod offset_eq;
pub mod poly;
pub mod split_eq;

// Flat re-exports for convenience.
pub use eq_poly::{EqPolynomial, SplitEqEvals};
pub use jolt_field::{
    cfg_chunks, cfg_chunks_mut, cfg_fold_reduce, cfg_into_iter, cfg_iter, cfg_iter_mut, cfg_join,
};
pub use jolt_field::{AdditiveGroup, CanonicalEncoding, Field, One, PseudoMersenne, Ring, Zero};
pub use split_eq::GruenSplitEq;
