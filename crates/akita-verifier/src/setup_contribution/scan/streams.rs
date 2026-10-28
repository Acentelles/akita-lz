use super::super::{DirectScanWeights, GroupSetupSegment, RoleProjection};
use akita_error::AkitaError;
use jolt_field::Field;
use std::ops::Range;

impl<E: Field> GroupSetupSegment<E> {
    #[inline(always)]
    pub(super) fn accumulate_lane_streams(
        &self,
        range: Range<usize>,
        weights: &mut [E],
        weight_start: usize,
        direct: &DirectScanWeights<E>,
        projections: &[RoleProjection<E>; 3],
    ) -> Result<(), AkitaError> {
        let len = range.end.checked_sub(range.start).ok_or(AkitaError::InvalidProof)?;
        if range.start < self.lo || range.end > self.hi {
            return Err(AkitaError::InvalidProof);
        }
        let end = weight_start.checked_add(len).ok_or(AkitaError::InvalidProof)?;
        let output = weights.get_mut(weight_start..end).ok_or(AkitaError::InvalidProof)?;
        let [a, b, d] = projections;
        if self.has_d {
            d.accumulate_role(output, range.start, self.d_start_abs, self.d_weight, &direct.e)?;
        }
        if self.has_b {
            for term in self.b_terms.iter() {
                let values = direct.t.get(term.logical_start..).ok_or(AkitaError::InvalidProof)?;
                b.accumulate_role(output, range.start, self.b_start_abs, term.row_weight, values)?;
            }
        }
        if self.has_a {
            a.accumulate_role(output, range.start, self.a_start_abs, self.a_row_weight, &direct.z)?;
        }
        Ok(())
    }
}

impl<E: Field> RoleProjection<E> {
    // Distributivity permits scaling the row weight once per lane, including
    // each overlapping B term. No challenge value or zero support is cached.
    #[inline(always)]
    fn accumulate_role(
        &self,
        output: &mut [E],
        base_start: usize,
        role_start: usize,
        row_weight: E,
        values: &[E],
    ) -> Result<(), AkitaError> {
        if output.is_empty() {
            return Ok(());
        }
        match self {
            Self::Identity => {
                let start = base_start.checked_sub(role_start).ok_or(AkitaError::InvalidProof)?;
                let end = start.checked_add(output.len()).ok_or(AkitaError::InvalidProof)?;
                let values = values.get(start..end).ok_or(AkitaError::InvalidProof)?;
                for (slot, value) in output.iter_mut().zip(values) {
                    *slot += row_weight * *value;
                }
            }
            Self::Projected { scales, shift, mask } => {
                let ratio = scales.len();
                if !ratio.is_power_of_two()
                    || *shift != ratio.trailing_zeros() as usize
                    || *mask != ratio - 1
                {
                    return Err(AkitaError::InvalidProof);
                }
                // A job may start or finish inside a projection block.
                for offset in 0..ratio.min(output.len()) {
                    let base = base_start.checked_add(offset).ok_or(AkitaError::InvalidProof)?;
                    let start = (base >> *shift).checked_sub(role_start).ok_or(AkitaError::InvalidProof)?;
                    let count = (output.len() - 1 - offset) / ratio + 1;
                    let end = start.checked_add(count).ok_or(AkitaError::InvalidProof)?;
                    let values = values.get(start..end).ok_or(AkitaError::InvalidProof)?;
                    let scale = scales.get(base & *mask).ok_or(AkitaError::InvalidProof)?;
                    let scaled_row = row_weight * *scale;
                    for (slot, value) in output.iter_mut().skip(offset).step_by(ratio).zip(values) {
                        *slot += scaled_row * *value;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::{role_projection, PhysicalBWeightTerm};
    use akita_algebra::ring::scalar_powers;
    use jolt_field::{One, Prime128OffsetA7F7, Ring, Zero};
    use std::sync::Arc;

    type F = Prime128OffsetA7F7;

    #[test]
    fn lane_streams_cover_partial_blocks_and_overlapping_roles() {
        let alpha = F::from_u64(7);
        let base = scalar_powers(alpha, 64);
        for ratio in [1, 2, 4, 8] {
            let powers = scalar_powers(alpha, 64 * ratio);
            let projection = || role_projection(&powers, &base, ratio).unwrap();
            for active in 0..8 {
                let direct = DirectScanWeights {
                    e: (0..16).map(|i| F::from_u64(11 + i)).collect(),
                    t: (0..32).map(|i| F::from_u64(31 + i)).collect(),
                    z: (0..16).map(|i| F::from_u64(71 + i)).collect(),
                };
                let segment = GroupSetupSegment {
                    lo: ratio, hi: 9 * ratio,
                    has_d: active & 1 != 0, d_start_abs: 1, d_weight: F::from_u64(3),
                    has_b: active & 2 != 0, b_start_abs: 1,
                    b_terms: Arc::from([
                        PhysicalBWeightTerm { logical_start: 0, row_weight: F::from_u64(5) },
                        PhysicalBWeightTerm { logical_start: 8, row_weight: F::from_u64(13) },
                    ]),
                    has_a: active & 4 != 0, a_start_abs: 1, a_row_weight: F::from_u64(17),
                };
                for begin in ratio..2 * ratio + 1 {
                    for end in 7 * ratio..9 * ratio + 1 {
                        let mut got = vec![F::from_u64(19); end - begin + 2];
                        segment.accumulate_lane_streams(begin..end, &mut got, 1, &direct,
                            &[projection(), projection(), projection()]).unwrap();
                        assert_eq!(got[0], F::from_u64(19));
                        assert_eq!(*got.last().unwrap(), F::from_u64(19));
                        for (offset, actual) in got[1..got.len()-1].iter().enumerate() {
                            let physical = begin + offset;
                            let logical = physical / ratio - 1;
                            let mut expected = F::zero();
                            if active & 1 != 0 { expected += F::from_u64(3) * direct.e[logical]; }
                            if active & 2 != 0 {
                                expected += F::from_u64(5) * direct.t[logical]
                                    + F::from_u64(13) * direct.t[8 + logical];
                            }
                            if active & 4 != 0 { expected += F::from_u64(17) * direct.z[logical]; }
                            expected *= powers[64 * (physical % ratio)];
                            assert_eq!(*actual, F::from_u64(19) + expected);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn lane_streams_reject_invalid_projection_and_short_values() {
        let values = vec![F::one(); 4];
        let mut output = vec![F::zero(); 8];
        for projection in [
            RoleProjection::Projected { scales: vec![], shift: 0, mask: 0 },
            RoleProjection::Projected { scales: vec![F::one(); 3], shift: 1, mask: 2 },
            RoleProjection::Projected { scales: vec![F::one(); 2], shift: 0, mask: 1 },
            RoleProjection::Projected { scales: vec![F::one(); 2], shift: 1, mask: 0 },
        ] {
            assert!(projection.accumulate_role(&mut output, 0, 0, F::one(), &values).is_err());
        }
        assert!(RoleProjection::Identity.accumulate_role(&mut output, 0, 0, F::zero(), &values).is_err());
        assert!(RoleProjection::Identity.accumulate_role(&mut output, 0, 1, F::one(), &values).is_err());
        let projection = RoleProjection::Projected { scales: vec![F::one(); 2], shift: 1, mask: 1 };
        assert!(projection.accumulate_role(&mut output, 1, 0, F::one(), &values).is_err());
        assert!(projection.accumulate_role(&mut output, usize::MAX, 0, F::one(), &values).is_err());
    }
}
