//! Fallible prover kernels driven by `jolt-sumcheck`'s single-instance driver.
//!
//! Every method that does work returns `Result`, so a stale session, an
//! invalid round progression, or a failed internal invariant reaches the caller
//! as an [`AkitaError`] instead of aborting the prover. The public shape of an
//! instance (input claim, degree bound, equality point) lives in the caller's
//! `jolt_sumcheck::SingleInstance`, not in the kernel.
//!
//! [`EqFactoredSumcheckKernel`] is for sumchecks whose round polynomial factors
//! as `s(X) = eq(τ, X) · q(X)`; the kernel returns `q` with its constant term
//! omitted, and [`crate::FusedEqFactoredRounds`] recovers it from the running
//! claim and the instance's `τ`.

use akita_error::AkitaError;
use jolt_field::Field;
use jolt_poly::{OmittedConstantPoly, UnivariatePoly};

/// A standard sumcheck kernel, driven through [`crate::FusedRounds`].
pub trait SumcheckKernel<E: Field> {
    /// Number of rounds (variables bound by the sumcheck).
    fn num_rounds(&self) -> usize;

    /// The round polynomial `g_round(X)`, with `g(0) + g(1) = claim`.
    fn round_polynomial(&mut self, round: usize, claim: E)
        -> Result<UnivariatePoly<E>, AkitaError>;

    /// Binds the current variable to `challenge`.
    fn bind_challenge(&mut self, round: usize, challenge: E) -> Result<(), AkitaError>;

    /// End-of-protocol hook after the last challenge is bound.
    fn finish(&mut self) -> Result<(), AkitaError> {
        Ok(())
    }
}

/// A normalized equality-factored sumcheck kernel, driven through
/// [`crate::FusedEqFactoredRounds`].
///
/// In round `j` the round polynomial is `eq(τ_j, X) · q_j(X)`, and the claim is
/// normalized: equality factors of earlier rounds are absent from it.
pub trait EqFactoredSumcheckKernel<E: Field> {
    /// Number of rounds (variables bound by the sumcheck).
    fn num_rounds(&self) -> usize;

    /// `q_round` without its constant term, from the normalized running claim.
    fn round_polynomial(
        &mut self,
        round: usize,
        claim: E,
    ) -> Result<OmittedConstantPoly<E>, AkitaError>;

    /// Binds the current variable to `challenge`.
    fn bind_challenge(&mut self, round: usize, challenge: E) -> Result<(), AkitaError>;

    /// End-of-protocol hook after the last challenge is bound.
    fn finish(&mut self) -> Result<(), AkitaError> {
        Ok(())
    }
}
