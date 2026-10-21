//! Test-only prover faults for soundness tests. Each fault makes the prover
//! emit a proof the honest path never produces, past its own self-checks, so
//! a test can show which verifier check rejects it.

use std::cell::Cell;

use akita_error::AkitaError;
use jolt_algebra::Field;
use jolt_poly::UnivariatePoly;
use jolt_sumcheck::ProveRounds;

thread_local! {
    /// `Some(count)` while the stage-2 fault is armed on this thread; `count`
    /// is how many stage-2 sumchecks it has perturbed.
    static STAGE2_LAST_ROUND_SHIFT: Cell<Option<usize>> = const { Cell::new(None) };
}

/// Runs `prove` with the stage-2 fault armed on this thread and returns its
/// result with the number of stage-2 sumchecks the fault perturbed.
///
/// The fault adds `X − X²` to the last stage-2 round polynomial. That keeps
/// the round's `s(0) + s(1)` and every earlier round honest, so the rounds
/// verify, but the final claim misses the stage-2 oracle by `r − r²` at the
/// last challenge `r`. The prover skips its own final-claim self-check while
/// the fault is armed.
pub fn with_stage2_last_round_shift<R>(prove: impl FnOnce() -> R) -> (R, usize) {
    STAGE2_LAST_ROUND_SHIFT.with(|fault| fault.set(Some(0)));
    let result = prove();
    let count = STAGE2_LAST_ROUND_SHIFT
        .with(|fault| fault.take())
        .unwrap_or(0);
    (result, count)
}

/// A stage-2 round member that applies the fault to its last round when the
/// fault is armed on this thread.
pub(crate) struct LastRoundShift<'m, E> {
    inner: &'m mut dyn ProveRounds<E, AkitaError>,
    armed: bool,
}

impl<'m, E> LastRoundShift<'m, E> {
    pub(crate) fn new(inner: &'m mut dyn ProveRounds<E, AkitaError>) -> Self {
        Self {
            inner,
            armed: STAGE2_LAST_ROUND_SHIFT.with(|fault| fault.get().is_some()),
        }
    }

    pub(crate) fn armed(&self) -> bool {
        self.armed
    }
}

impl<E: Field> ProveRounds<E, AkitaError> for LastRoundShift<'_, E> {
    fn num_rounds(&self) -> usize {
        self.inner.num_rounds()
    }

    fn prove_round(
        &mut self,
        bind: Option<E>,
        round: usize,
        previous_claim: E,
    ) -> Result<UnivariatePoly<E>, AkitaError> {
        let poly = self.inner.prove_round(bind, round, previous_claim)?;
        if !self.armed || round + 1 != self.inner.num_rounds() {
            return Ok(poly);
        }
        let mut coefficients = poly.into_coefficients();
        if coefficients.len() < 3 {
            coefficients.resize(3, E::zero());
        }
        coefficients[1] += E::one();
        coefficients[2] -= E::one();
        STAGE2_LAST_ROUND_SHIFT.with(|fault| fault.set(fault.get().map(|count| count + 1)));
        Ok(UnivariatePoly::new(coefficients))
    }

    fn finish_rounds(&mut self, bind: E) -> Result<(), AkitaError> {
        self.inner.finish_rounds(bind)
    }
}
