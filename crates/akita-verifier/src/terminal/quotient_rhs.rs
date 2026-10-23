//! Evaluate sparse negacyclic products without materializing their coefficients.

use akita_algebra::CyclotomicRing;
use akita_challenges::SparseChallenge;
use akita_error::AkitaError;
use jolt_field::{Field, Ring};

/// For `w_j = (X^j c mod (X^D + 1))(r)`, use
/// `w_0 = c(r)` and `w_(j+1) = r*w_j - (r^D + 1)*c_(D-1-j)`.
/// Thus `dot(t, w)` evaluates the reduced product even at `r = 0`.
pub(super) fn evaluate<F, const D: usize>(
    t: &[CyclotomicRing<F, D>],
    challenges: &[SparseChallenge],
    num_rows: usize,
    points: [F; 2],
) -> Result<Vec<[F; 2]>, AkitaError>
where
    F: Field + Ring,
{
    if D == 0
        || t.len()
            != num_rows
                .checked_mul(challenges.len())
                .ok_or(AkitaError::InvalidProof)?
    {
        return Err(AkitaError::InvalidProof);
    }
    // The caller's authenticated terminal schedule bounds num_rows. Preserve
    // the empty-row behavior without passing zero to chunks_exact.
    if num_rows == 0 {
        return Ok(Vec::new());
    }
    for challenge in challenges {
        challenge.validate::<D>()?;
    }
    let mut result = vec![[F::zero(); 2]; num_rows];
    let mut corrections = vec![F::zero(); D];
    let mut weights = Vec::with_capacity(D);
    for (point_index, point) in points.into_iter().enumerate() {
        let powers = super::quotient::power_row(point, D);
        let shift = powers.last().copied().ok_or(AkitaError::InvalidProof)? * point + F::one();
        for (challenge, block) in challenges.iter().zip(t.chunks_exact(num_rows)) {
            corrections.fill(F::zero());
            let mut weight = F::zero();
            for (&position, &coefficient) in challenge.positions.iter().zip(&challenge.coeffs) {
                let position = usize::try_from(position).map_err(|_| AkitaError::InvalidProof)?;
                let coefficient = F::from_i64(i64::from(coefficient));
                let power = powers.get(position).ok_or(AkitaError::InvalidProof)?;
                weight += *power * coefficient;
                *corrections
                    .get_mut(position)
                    .ok_or(AkitaError::InvalidProof)? = shift * coefficient;
            }
            weights.clear();
            weights.push(weight);
            for correction in corrections.iter().skip(1).rev() {
                weight = point * weight - *correction;
                weights.push(weight);
            }
            for (evaluations, row) in result.iter_mut().zip(block) {
                *evaluations
                    .get_mut(point_index)
                    .ok_or(AkitaError::InvalidProof)? +=
                    F::dot_product(row.coefficients(), &weights);
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
#[path = "quotient_rhs_tests.rs"]
mod tests;
