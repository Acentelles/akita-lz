//! L2 terminal quotient certificate (experimental, `l2-terminal-quotient`).
//!
//! Replaces the exact negacyclic terminal A-row product by a disclosed
//! quotient: for every row `i` the prover sends `Q_i` (exactly `D - 1`
//! canonical coefficients) after the `z` payload, the verifier then draws
//! `r_1, r_2` and checks
//!
//! `sum_j A_ij(r) z_j(r) - H_i(r) = (r^D + 1) Q_i(r)` at both points,
//!
//! where `H_i` is the sparse-challenge combination of the terminal `t` rows
//! and `A_ij` is read from the setup's own public matrix. A false ring row
//! passes one point with probability at most `(2D - 2) / p`; see the lever's
//! SOUNDNESS note. All other terminal checks are unchanged.

use akita_error::AkitaError;
use akita_params::TerminalFoldParams;
use akita_types::GrindingReplay;
use jolt_field::{CanonicalEncoding, Field, Ring};

/// The received quotient, the two evaluation points drawn after it, and the
/// setup coefficient view the A rows are read from.
pub(crate) struct TerminalQuotient<'a, F> {
    pub(crate) setup: &'a [F],
    pub(crate) coefficients: Vec<F>,
    pub(crate) points: [F; 2],
}

/// Receive the quotient (after the `z` payload) and draw the two points.
pub(crate) fn receive_terminal_quotient<'a, F>(
    grinding: &mut akita_types::VerifierGrinding<'_, '_>,
    level: u32,
    scheduled: &TerminalFoldParams,
    setup: &'a [F],
) -> Result<TerminalQuotient<'a, F>, AkitaError>
where
    F: Field + CanonicalEncoding,
{
    let quotient_len = akita_types::terminal_quotient_len(scheduled.d_a())?;
    let count = scheduled
        .inner
        .matrix
        .output_rank()
        .checked_mul(quotient_len)
        .ok_or(AkitaError::InvalidProof)?;
    let coefficients = akita_transcript::receive_field_group::<F>(
        grinding.state_mut(),
        akita_types::FoldSite::TerminalQuotient { level }.id()?,
        count,
    )?;
    let first = akita_transcript::verifier_field_challenge::<F>(grinding.state_mut())?;
    let second = akita_transcript::verifier_field_challenge::<F>(grinding.state_mut())?;
    Ok(TerminalQuotient {
        setup,
        coefficients,
        points: [first, second],
    })
}

/// `[1, r, ..., r^(D-1)]`.
pub(super) fn power_row<F: Field>(point: F, d: usize) -> Vec<F> {
    let mut powers = Vec::with_capacity(d);
    let mut power = F::one();
    for _ in 0..d {
        powers.push(power);
        power *= point;
    }
    powers
}

/// Check every terminal A row against its quotient at both points.
///
/// `rows` are the two evaluations of each `H_i`, computed from the bound
/// terminal messages and challenges; `z` contains the centered response rings.
pub(crate) fn check_rows<F, const D: usize>(
    quotient: &TerminalQuotient<'_, F>,
    rows: &[[F; 2]],
    z: &[[i16; D]],
) -> Result<(), AkitaError>
where
    F: Field + Ring,
{
    let quotient_len = akita_types::terminal_quotient_len(D)?;
    let num_rows = rows.len();
    if quotient.coefficients.len()
        != num_rows
            .checked_mul(quotient_len)
            .ok_or(AkitaError::InvalidProof)?
    {
        return Err(AkitaError::InvalidProof);
    }
    let num_cols = z.len();
    let row_len = num_cols.checked_mul(D).ok_or(AkitaError::InvalidProof)?;
    let prefix = num_rows
        .checked_mul(row_len)
        .ok_or(AkitaError::InvalidProof)?;
    let setup = quotient.setup.get(..prefix).ok_or_else(|| {
        AkitaError::InvalidSetup("terminal quotient setup view is shorter than the A prefix".into())
    })?;
    let z_fields: Vec<F> = z
        .iter()
        .flat_map(|ring| ring.iter().map(|&value| F::from_i64(i64::from(value))))
        .collect();
    for (point_index, &point) in quotient.points.iter().enumerate() {
        let powers = power_row(point, D);
        let last = powers.last().copied().ok_or(AkitaError::InvalidProof)?;
        let shift = last * point + F::one();
        let z_evals: Vec<F> = z_fields
            .chunks_exact(D)
            .map(|ring| F::dot_product(ring, &powers))
            .collect();
        for (row_index, (rhs, row_quotient)) in rows
            .iter()
            .zip(quotient.coefficients.chunks_exact(quotient_len))
            .enumerate()
        {
            let start = row_index
                .checked_mul(row_len)
                .ok_or(AkitaError::InvalidProof)?;
            let row = setup
                .get(start..start + row_len)
                .ok_or(AkitaError::InvalidProof)?;
            let product = if num_cols == 0 {
                F::zero()
            } else {
                row.chunks_exact(D)
                    .zip(&z_evals)
                    .fold(F::zero(), |acc, (a, z_eval)| {
                        acc + F::dot_product(a, &powers) * *z_eval
                    })
            };
            let h = *rhs.get(point_index).ok_or(AkitaError::InvalidProof)?;
            let q = F::dot_product(row_quotient, &powers);
            if product - h != shift * q {
                return Err(AkitaError::InvalidProof);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "quotient_tests.rs"]
mod tests;
