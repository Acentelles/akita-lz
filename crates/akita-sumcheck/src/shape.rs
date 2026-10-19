//! Public round-count and degree grammar of one sumcheck.

use akita_error::{checked, AkitaError};

/// Public fixed grammar of one sumcheck invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SumcheckShape {
    num_rounds: usize,
    degree_bound: usize,
}

impl SumcheckShape {
    /// Construct a checked sumcheck grammar.
    ///
    /// # Errors
    ///
    /// Returns an error when a proof-controlled round body would have no
    /// coefficients or the shape cannot be represented safely.
    pub fn new(num_rounds: usize, degree_bound: usize) -> Result<Self, AkitaError> {
        if degree_bound == 0
            || u32::try_from(num_rounds).is_err()
            || checked::product([degree_bound, num_rounds]).is_none()
        {
            return Err(AkitaError::InvalidSetup(
                "invalid native sumcheck shape".into(),
            ));
        }
        Ok(Self {
            num_rounds,
            degree_bound,
        })
    }

    /// Number of proof rounds.
    #[must_use]
    pub const fn num_rounds(self) -> usize {
        self.num_rounds
    }

    /// Fixed extension coefficients emitted in each compressed round body.
    #[must_use]
    pub const fn degree_bound(self) -> usize {
        self.degree_bound
    }
}
