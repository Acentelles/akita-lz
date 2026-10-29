use super::super::*;
use akita_algebra::cfg_try_fold_reduce;

#[expect(
    clippy::too_many_arguments,
    reason = "matches the existing setup scan boundary"
)]
pub(super) fn evaluate_base_ring_direct<F, E, const BASE_D: usize>(
    group: &SetupContributionGroupPlan<E>,
    setup_view: &RingMatrixView<'_, F, BASE_D>,
    weights: &DirectScanWeights<E>,
    partition: &GroupScanPartition<E>,
    base_pows: &[E],
    d_weights: &[E],
    a_projection: &RoleProjection<E>,
    b_projection: &RoleProjection<E>,
    d_projection: &RoleProjection<E>,
    d_rows: usize,
    d_physical_cols: usize,
) -> Result<E, AkitaError>
where
    F: Field,
    E: ExtField<F> + MulBaseUnreduced<F>,
{
    let setup_flat = setup_view.as_slice();
    let (e_eq_slice, t_eq_slice, z_eq_slice) = (&weights.e[..], &weights.t[..], &weights.z[..]);
    if partition.required > setup_flat.len() {
        return Err(AkitaError::InvalidSetup(
            "shared matrix is too small for selected verifier layout".into(),
        ));
    }
    if d_weights.len() != d_rows
        || group.d_col_range().end > d_physical_cols
        || group.d_col_range().len() != e_eq_slice.len()
    {
        return Err(AkitaError::InvalidSetup(
            "cached setup scan geometry is malformed".into(),
        ));
    }

    cfg_try_fold_reduce!(
        partition.segments.as_slice(),
        E::zero,
        |acc, segment| {
            if !(a_projection.is_identity()
                && b_projection.is_identity()
                && d_projection.is_identity())
            {
                return dispatch_segment_roles!(segment, Ok(acc), |HAS_D, HAS_B, HAS_A| {
                    base_ring_segment_inner_sum_typed::<F, E, BASE_D, HAS_D, HAS_B, HAS_A>(
                        segment.lo..segment.hi,
                        setup_flat,
                        base_pows,
                        segment,
                        e_eq_slice,
                        t_eq_slice,
                        z_eq_slice,
                        d_projection,
                        b_projection,
                        a_projection,
                    )
                    .map(|term| acc + term)
                });
            }
            if !(segment.has_a || segment.has_b || segment.has_d) {
                return Ok(acc);
            }
            let setup = setup_flat
                .get(segment.lo..segment.hi)
                .ok_or(AkitaError::InvalidProof)?;
            let mut row_weights = vec![E::zero(); setup.len()];
            segment.accumulate_weight_tiles(
                segment.lo..segment.hi,
                &mut row_weights,
                0,
                [z_eq_slice, t_eq_slice, e_eq_slice],
                [a_projection, b_projection, d_projection],
            )?;
            let mut rows = Vec::with_capacity(setup.len());
            let mut nonzero_weights = Vec::with_capacity(setup.len());
            for (ring, weight) in setup.iter().zip(row_weights) {
                if !weight.is_zero() {
                    rows.push(ring.coefficients().as_slice());
                    nonzero_weights.push(weight);
                }
            }
            Ok(acc + E::weighted_dot_base_rows(&rows, &nonzero_weights, base_pows))
        },
        |lhs, rhs| Ok(lhs + rhs)
    )
}
