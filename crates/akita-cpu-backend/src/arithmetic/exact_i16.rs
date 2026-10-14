use super::CpuPreparedSetup;
use crate::sources::block_digits::{DigitShape, I16Rows};
use crate::sources::packed_digits::PackedSignedDigitView;
use akita_algebra::ring::cyclotomic::BalancedDecomposePow2Params;
use akita_algebra::CyclotomicRing;
use akita_error::AkitaError;
use akita_params::balanced_signed_digit_abs_bound;
use akita_types::{NttCacheKey, NttTransformDomain};
#[allow(unused_imports)]
use jolt_field::solinas::parallel::*;
use jolt_field::{CanonicalEncoding, Field};
use std::array::from_fn;
use std::ops::Range;

#[tracing::instrument(skip_all, name = "dense_commit_rows_exact_i16")]
pub(super) fn commit_rows_i16<F, S, const D: usize>(
    prepared: &CpuPreparedSetup<F>,
    n_a: usize,
    row_width: usize,
    source: &S,
    block_ranges: &[Range<usize>],
    shape: DigitShape,
) -> Result<Vec<Vec<CyclotomicRing<F, D>>>, AkitaError>
where
    F: Field + CanonicalEncoding,
    S: I16Rows<D>,
{
    let rhs_abs_bound = balanced_signed_digit_abs_bound(shape.log_basis)
        .ok_or_else(|| AkitaError::InvalidSetup("invalid signed digit basis".into()))?;
    prepared.with_shared_ntt::<D, _>(
        NttCacheKey::from_matrix_shape(
            D,
            n_a,
            row_width,
            NttTransformDomain::ExactNegacyclicI16 {
                width: row_width,
                rhs_abs_bound,
            },
        )?,
        |ntt| {
            let fill = |scratch: &mut S::I16Scratch, rings: &Range<usize>| {
                let rhs = source.i16_rows(rings.clone(), shape, scratch)?;
                ntt.mat_vec_i16::<F>(shape.log_basis, n_a, rhs)
            };
            let scratch = || source.i16_scratch(shape, row_width / shape.num_digits);
            #[cfg(feature = "parallel")]
            {
                cfg_iter!(block_ranges).map_init(scratch, fill).collect()
            }
            #[cfg(not(feature = "parallel"))]
            {
                let mut rhs = scratch();
                block_ranges
                    .iter()
                    .map(|rings| fill(&mut rhs, rings))
                    .collect()
            }
        },
    )
}

pub(super) fn recursive_packed_witness_commit_rows<F: Field + CanonicalEncoding, const D: usize>(
    prepared: &CpuPreparedSetup<F>,
    digits: PackedSignedDigitView<'_>,
    n_rows: usize,
    num_positions_per_block: usize,
    num_live_blocks: usize,
    num_digits_inner: usize,
    log_basis_inner: u32,
) -> Result<Vec<Vec<CyclotomicRing<F, D>>>, AkitaError> {
    let row_width = num_positions_per_block
        .checked_mul(num_digits_inner)
        .ok_or_else(|| AkitaError::InvalidSetup("recursive A width overflow".into()))?;
    let rhs_abs_bound = balanced_signed_digit_abs_bound(log_basis_inner)
        .ok_or_else(|| AkitaError::InvalidSetup("invalid signed digit basis".into()))?;
    let ring_elems = digits.len() / D;
    let params = BalancedDecomposePow2Params::new(num_digits_inner, log_basis_inner);
    prepared.with_shared_ntt::<D, _>(
        NttCacheKey::from_matrix_shape(
            D,
            n_rows,
            row_width,
            NttTransformDomain::ExactNegacyclicI16 {
                width: row_width,
                rhs_abs_bound,
            },
        )?,
        |ntt| {
            cfg_into_iter!(0..num_live_blocks)
                .map(|block_index| {
                    let start_ring = block_index * num_positions_per_block;
                    let live = (ring_elems - start_ring).min(num_positions_per_block);
                    let block = digits.decode_rings::<D>(start_ring, live)?;
                    let mut rhs = vec![[0i16; D]; row_width];
                    if num_digits_inner == 1 {
                        for (dst, src) in rhs.iter_mut().zip(&block) {
                            *dst = from_fn(|k| i16::from(src[k]));
                        }
                    } else {
                        for (ring_idx, digit) in block.iter().enumerate() {
                            let ring = CyclotomicRing::from_coefficients(from_fn(|k| {
                                F::from_i8(digit[k])
                            }));
                            let start = ring_idx * num_digits_inner;
                            ring.balanced_decompose_pow2_i16_into(
                                &mut rhs[start..start + num_digits_inner],
                                &params,
                            );
                        }
                    }
                    ntt.mat_vec_i16::<F>(log_basis_inner, n_rows, &rhs)
                })
                .collect()
        },
    )
}
