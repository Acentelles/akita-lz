use super::{ProductSum, ReducedScanGroup};
use crate::setup_contribution::GroupScanPartition;
use akita_algebra::{cfg_try_fold_reduce, CyclotomicRing};
use akita_error::AkitaError;
#[cfg(feature = "parallel")]
use jolt_field::solinas::parallel::{IntoParallelIterator, ParallelIterator};
use jolt_field::{ExtField, Field, MulBaseUnreduced, Unreduced};
use std::ops::Range;

/// The ratio-one, single-functional case. Segment boundaries and role slices
/// are checked once per span; each ring retains the original D, B, A and
/// group accumulation order. No equality between different queries is used.
pub(super) fn evaluate<F, E, A, const D: usize>(
    setup: &[CyclotomicRing<F, D>],
    required: usize,
    functional: &[E],
    groups: &[ReducedScanGroup<'_, E>],
    partitions: &[GroupScanPartition<E>],
) -> Result<E, AkitaError>
where
    F: Field,
    E: ExtField<F> + MulBaseUnreduced<F>,
    A: ProductSum<E>,
{
    if groups.len() != partitions.len()
        || functional.len() != D
        || groups
            .iter()
            .any(|group| group.role_ratios != [1; 3] || group.role_classes != [0; 3])
    {
        return Err(AkitaError::InvalidProof);
    }
    let setup = setup.get(..required).ok_or(AkitaError::InvalidProof)?;
    let job_rings = crate::setup_contribution::segments::SETUP_SCAN_JOB_RINGS;
    cfg_try_fold_reduce!(
        0..required.div_ceil(job_rings),
        E::zero,
        |total, job| {
            let lo = job.checked_mul(job_rings).ok_or(AkitaError::InvalidProof)?;
            let hi = lo
                .checked_add(job_rings)
                .ok_or(AkitaError::InvalidProof)?
                .min(required);
            let rings = setup.get(lo..hi).ok_or(AkitaError::InvalidProof)?;
            let mut sums = Vec::new();
            sums.try_reserve_exact(rings.len()).map_err(|_| {
                AkitaError::InvalidSetup("setup segment scratch allocation failed".into())
            })?;
            sums.resize_with(rings.len(), A::zero);
            for (group, partition) in groups.iter().zip(partitions) {
                let first = partition
                    .segments
                    .partition_point(|segment| segment.hi <= lo);
                for segment in partition.segments.iter().skip(first) {
                    if segment.lo >= hi {
                        break;
                    }
                    let start = segment.lo.max(lo);
                    let end = segment.hi.min(hi);
                    let local_start = start.checked_sub(lo).ok_or(AkitaError::InvalidProof)?;
                    let local_end = end.checked_sub(lo).ok_or(AkitaError::InvalidProof)?;
                    let targets = sums
                        .get_mut(local_start..local_end)
                        .ok_or(AkitaError::InvalidProof)?;
                    if segment.has_d {
                        add_span(
                            targets,
                            start..end,
                            segment.d_start_abs,
                            group.e,
                            segment.d_weight,
                        )?;
                    }
                    if segment.has_b {
                        for term in segment.b_terms.iter() {
                            let local_start = start
                                .checked_sub(segment.b_start_abs)
                                .and_then(|offset| term.logical_start.checked_add(offset))
                                .ok_or(AkitaError::InvalidProof)?;
                            let local_end = local_start
                                .checked_add(targets.len())
                                .ok_or(AkitaError::InvalidProof)?;
                            add_span(targets, local_start..local_end, 0, group.t, term.row_weight)?;
                        }
                    }
                    if segment.has_a {
                        add_span(
                            targets,
                            start..end,
                            segment.a_start_abs,
                            group.z,
                            segment.a_row_weight,
                        )?;
                    }
                }
            }
            let mut rows = Vec::new();
            let mut weights = Vec::new();
            rows.try_reserve_exact(rings.len()).map_err(|_| {
                AkitaError::InvalidSetup("setup segment row allocation failed".into())
            })?;
            weights.try_reserve_exact(rings.len()).map_err(|_| {
                AkitaError::InvalidSetup("setup segment weight allocation failed".into())
            })?;
            for (ring, sum) in rings.iter().zip(sums) {
                let weight = sum.finish();
                if !weight.is_zero() {
                    rows.push(ring.coefficients().as_slice());
                    weights.push(weight);
                }
            }
            Ok(total + E::weighted_dot_base_rows(&rows, &weights, functional))
        },
        |lhs, rhs| Ok(lhs + rhs)
    )
}

fn add_span<E: Field + Unreduced, A: ProductSum<E>>(
    output: &mut [A],
    range: Range<usize>,
    origin: usize,
    values: &[E],
    scale: E,
) -> Result<(), AkitaError> {
    let start = range
        .start
        .checked_sub(origin)
        .ok_or(AkitaError::InvalidProof)?;
    let end = range
        .end
        .checked_sub(origin)
        .ok_or(AkitaError::InvalidProof)?;
    let values = values.get(start..end).ok_or(AkitaError::InvalidProof)?;
    if values.len() != output.len() {
        return Err(AkitaError::InvalidProof);
    }
    for (sum, &value) in output.iter_mut().zip(values) {
        sum.add_product(scale, value);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::CanonicalProductSum;
    use super::*;
    use crate::setup_contribution::GroupSetupSegment;
    use jolt_field::{Prime32Offset99, Ring};
    use std::sync::Arc;

    type F = Prime32Offset99;

    #[test]
    fn shared_segments_cross_jobs_and_sum_overlapping_roles() {
        let n = 2051;
        let setup = vec![CyclotomicRing::from_coefficients([F::from_u64(2), F::from_u64(3)]); n];
        let e = vec![F::from_u64(5); n];
        let z = vec![F::from_u64(7); n];
        let groups = [ReducedScanGroup {
            e: &e,
            t: &[],
            z: &z,
            role_ratios: [1; 3],
            role_classes: [0; 3],
        }];
        let partitions = [GroupScanPartition {
            required: n,
            segments: vec![GroupSetupSegment {
                lo: 1,
                hi: n - 1,
                has_d: true,
                d_start_abs: 0,
                d_weight: F::from_u64(11),
                has_b: false,
                b_start_abs: 0,
                b_terms: Arc::from([]),
                has_a: true,
                a_start_abs: 0,
                a_row_weight: F::from_u64(13),
            }],
        }];
        let actual = evaluate::<F, F, CanonicalProductSum<F>, 2>(
            &setup,
            n,
            &[F::from_u64(17), F::from_u64(19)],
            &groups,
            &partitions,
        )
        .unwrap();
        // Each active ring contributes (2*17+3*19)*(5*11+7*13).
        assert_eq!(actual, F::from_u64((n as u64 - 2) * 91 * 146));
    }

    #[test]
    fn checked_span_rejects_underflow_and_short_source() {
        let mut output = [
            CanonicalProductSum::<F>::zero(),
            CanonicalProductSum::<F>::zero(),
        ];
        let values = [F::from_u64(2), F::from_u64(3)];
        assert!(add_span(&mut output, 0..2, 1, &values, F::from_u64(1)).is_err());
        assert!(add_span(&mut output, 1..3, 0, &values, F::from_u64(1)).is_err());
    }
}
