use super::PhysicalBWeightTerm;
#[cfg(test)]
use akita_error::AkitaError;
use jolt_field::Field;
#[cfg(test)]
use jolt_field::{ExtField, MulBaseUnreduced};

#[derive(Clone)]
pub(crate) struct GroupSetupSegment<E> {
    pub(super) lo: usize,
    pub(super) hi: usize,
    pub(super) has_d: bool,
    pub(super) d_start_abs: usize,
    pub(super) d_weight: E,
    pub(super) has_b: bool,
    pub(super) b_start_abs: usize,
    pub(super) b_terms: std::sync::Arc<[PhysicalBWeightTerm<E>]>,
    pub(super) has_a: bool,
    pub(super) a_start_abs: usize,
    pub(super) a_row_weight: E,
}

pub(super) enum RoleProjection<E> {
    Identity,
    Projected {
        scales: Vec<E>,
        shift: usize,
        mask: usize,
    },
}

pub(super) fn role_projection<E: Field>(
    alpha_pows: &[E],
    base_pows: &[E],
    expected_ratio: usize,
) -> Option<RoleProjection<E>> {
    let base_d = base_pows.len();
    if base_d == 0 || !alpha_pows.len().is_multiple_of(base_d) {
        return None;
    }
    let ratio = alpha_pows.len() / base_d;
    if ratio != expected_ratio {
        return None;
    }
    if ratio == 1 {
        return (alpha_pows == base_pows).then_some(RoleProjection::Identity);
    }
    let mut scales = Vec::with_capacity(ratio);
    for chunk in alpha_pows.chunks_exact(base_d) {
        let scale = *chunk.first()?;
        for (&power, &base_power) in chunk.iter().zip(base_pows) {
            if power != scale * base_power {
                return None;
            }
        }
        scales.push(scale);
    }
    Some(RoleProjection::Projected {
        scales,
        shift: ratio.trailing_zeros() as usize,
        mask: ratio.checked_sub(1)?,
    })
}

#[cfg(test)]
pub(super) fn evaluate_weighted_setup_row<Base, E>(
    row: &[Base],
    col_offset: usize,
    col_weights: &[E],
    row_weight: E,
    alpha_pows: &[E],
) -> Result<E, AkitaError>
where
    Base: Field,
    E: ExtField<Base> + MulBaseUnreduced<Base>,
{
    use super::checked_slice;

    let ring_d = alpha_pows.len();
    let mut acc = E::zero();
    for (col, &col_weight) in col_weights.iter().enumerate() {
        if col_weight.is_zero() {
            continue;
        }
        let setup_col = col_offset
            .checked_add(col)
            .ok_or_else(|| AkitaError::InvalidSetup("weighted setup column overflow".into()))?;
        let coeff_start = setup_col.checked_mul(ring_d).ok_or_else(|| {
            AkitaError::InvalidSetup("weighted setup coeff start overflow".into())
        })?;
        let coeffs = checked_slice(row, coeff_start, ring_d, "weighted setup coeffs")?;
        acc += row_weight * col_weight * MulBaseUnreduced::dot_base(alpha_pows, coeffs);
    }
    Ok(acc)
}
