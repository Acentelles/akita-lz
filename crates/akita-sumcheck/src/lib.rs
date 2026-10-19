//! Akita's sumcheck kernels and their adapters to `jolt-sumcheck`'s
//! single-instance driver.
//!
//! This crate owns only protocol-independent sumcheck machinery. Akita-specific
//! stage provers, verifier instances, and two-round-prefix skip proofs stay in
//! the PCS protocol crate until their role-specific APIs are split.

mod accum;
mod compact_fold;
mod fused;
mod shape;
mod traits;

pub use akita_algebra::poly::{fold_evals_in_place, multilinear_eval};

pub use accum::reduce_signed_accum;
pub use compact_fold::CompactPairFoldLut;
pub use fused::{FusedEqFactoredRounds, FusedRounds};
pub use shape::SumcheckShape;
pub use traits::{
    EqFactoredSumcheckInstanceProver, EqFactoredSumcheckKernel, SumcheckInstanceProver,
    SumcheckKernel,
};
