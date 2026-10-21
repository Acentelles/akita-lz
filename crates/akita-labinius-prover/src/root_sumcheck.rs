//! Dense product sumcheck for the succinct LaBinius root.
//!
//! Round `i` binds table-index bit `i`. Caller code must bind the statement
//! and obtain the terminal polynomial openings separately.

use akita_error::AkitaError;
use akita_labinius_verifier::channel::ClearChannel;
use akita_labinius_verifier::root_sumcheck::{
    bind_root_sumcheck_instance, product_shape, root_instance, RootSumcheckInstance,
};
use akita_sumcheck::{FusedRounds, SumcheckKernel};
use jolt_algebra::{CanonicalDecode, CanonicalEncoding, ExtField, Field};
use jolt_poly::UnivariatePoly;
use jolt_sumcheck::{prove_single, ProverRoundContext};

/// Product instance for `sum_x Y(x) * K_Y(x)` of two field tables over the
/// challenge field: a table of field elements and its public weights. The
/// root reduction uses it for the prime left opening, which has no alphabet
/// term.
///
/// For `N = 2^mu`, peak owned storage is `2*N` field elements plus constant
/// interpolation workspace. In-place folding retains the initial capacities.
#[derive(Debug)]
pub struct ProductSumcheck<F: Field> {
    y: Vec<F>,
    ky: Vec<F>,
    rounds: usize,
    next_round: usize,
    claim: F,
}

impl<F: Field> ProductSumcheck<F> {
    /// Construct a product instance from equally sized nonempty Boolean tables.
    pub fn new(y: Vec<F>, ky: Vec<F>, claimed_sum: F) -> Result<Self, AkitaError> {
        if !y.len().is_power_of_two() || y.len() != ky.len() {
            return Err(AkitaError::InvalidInput(
                "invalid product sumcheck geometry".into(),
            ));
        }
        let rounds = y.len().trailing_zeros() as usize;
        product_shape(rounds)?;
        Ok(Self {
            y,
            ky,
            rounds,
            next_round: 0,
            claim: claimed_sum,
        })
    }

    /// Return `(Y~(rho'), K_Y~(rho'))` once every challenge has been bound.
    pub fn final_evaluations(&self) -> Option<(F, F)> {
        if self.next_round != self.rounds {
            return None;
        }
        Some((*self.y.first()?, *self.ky.first()?))
    }
}

impl<F: Field> SumcheckKernel<F> for ProductSumcheck<F> {
    fn num_rounds(&self) -> usize {
        Self::num_rounds(self)
    }

    fn round_polynomial(
        &mut self,
        round: usize,
        claim: F,
    ) -> Result<UnivariatePoly<F>, AkitaError> {
        Ok(self.compute_round_univariate(round, claim))
    }

    fn bind_challenge(&mut self, round: usize, challenge: F) -> Result<(), AkitaError> {
        self.ingest_challenge(round, challenge);
        Ok(())
    }
}

impl<F: Field> ProductSumcheck<F> {
    /// Number of rounds, one per table variable.
    pub fn num_rounds(&self) -> usize {
        self.rounds
    }

    /// The instance's claimed sum.
    pub fn input_claim(&self) -> F {
        self.claim
    }

    /// The round polynomial of the next unbound variable.
    pub fn compute_round_univariate(
        &mut self,
        _round: usize,
        _previous_claim: F,
    ) -> UnivariatePoly<F> {
        let mut values = [F::zero(); 3];
        for (y, ky) in self.y.chunks_exact(2).zip(self.ky.chunks_exact(2)) {
            for (node, value) in values.iter_mut().enumerate() {
                let t = F::from_u64(node as u64);
                *value += (y[0] + t * (y[1] - y[0])) * (ky[0] + t * (ky[1] - ky[0]));
            }
        }
        UnivariatePoly::from_evals(&values)
    }

    /// Bind the next unbound variable to `challenge`.
    pub fn ingest_challenge(&mut self, _round: usize, challenge: F) {
        if self.next_round >= self.rounds {
            return;
        }
        for table in [&mut self.y, &mut self.ky] {
            let half = table.len() / 2;
            for index in 0..half {
                table[index] =
                    table[2 * index] + challenge * (table[2 * index + 1] - table[2 * index]);
            }
            table.truncate(half);
        }
        self.next_round += 1;
    }
}

/// Bind the product instance identity and prove its rounds on the channel.
/// The returned claim still requires the terminal check and table opening.
pub fn prove_product_rounds<F, E, C>(
    instance: &mut ProductSumcheck<E>,
    channel: &mut C,
    invocation: u32,
) -> Result<(Vec<E>, E), AkitaError>
where
    F: Field + CanonicalEncoding,
    E: ExtField<F> + CanonicalDecode,
    C: ProverRoundContext<E, Error = AkitaError> + ClearChannel,
{
    let num_vars = instance.num_rounds();
    let shape = product_shape(num_vars)?;
    bind_root_sumcheck_instance(channel, RootSumcheckInstance::Product, invocation, num_vars)?;
    let sumcheck = root_instance(shape, invocation, instance.input_claim());
    let reduction =
        prove_single::<F, E, AkitaError, C>(&mut FusedRounds(instance), &sumcheck, channel)?;
    Ok((reduction.challenges, reduction.output_claim))
}
