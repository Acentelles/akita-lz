use super::*;

pub(super) fn mat_vec_mul_digits_i8_with_params<
    F: Field + CanonicalEncoding,
    W: PrimeWidth,
    const K: usize,
    const D: usize,
>(
    ntt_mat: &[&[CyclotomicCrtNtt<W, K, D>]],
    blocks: &[&[[i8; D]]],
    log_basis: u32,
    params: &CrtNttParamSet<W, K, D>,
) -> Result<Vec<Vec<CyclotomicRing<F, D>>>, AkitaError> {
    let source = DigitRows {
        blocks,
        digit_abs_bound: balanced_digit_abs_bound(log_basis),
    };
    let mut start = 0;
    let ranges = blocks
        .iter()
        .map(|block| {
            let range = start..start + block.len();
            start = range.end;
            range
        })
        .collect::<Vec<_>>();
    mat_vec_mul_blocks_i8_with_params_impl::<F, _, W, K, D, true>(
        ntt_mat,
        &source,
        &ranges,
        DigitShape {
            num_digits: 1,
            log_basis,
        },
        "digit matvec CRT capacity cannot fit a single term",
        params,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn mat_vec_mul_blocks_i8_with_params_impl<
    F: Field + CanonicalEncoding,
    S: I8Rows<D>,
    W: PrimeWidth,
    const K: usize,
    const D: usize,
    const CHECK_ZERO: bool,
>(
    ntt_mat: &[&[CyclotomicCrtNtt<W, K, D>]],
    source: &S,
    blocks: &[Range<usize>],
    shape: DigitShape,
    capacity_error: &str,
    params: &CrtNttParamSet<W, K, D>,
) -> Result<Vec<Vec<CyclotomicRing<F, D>>>, AkitaError> {
    let num_live_blocks = blocks.len();
    if num_live_blocks == 0 {
        return Ok(vec![]);
    }
    let n_a = ntt_mat.len();
    let mat_width = ntt_mat.first().map_or(0, |row| row.len());
    let max_data_width = blocks
        .iter()
        .map(|b| b.len().saturating_mul(shape.num_digits))
        .max()
        .unwrap_or(0);
    let inner_width = mat_width.min(max_data_width);
    if inner_width == 0 || n_a == 0 {
        return Ok(vec![
            vec![CyclotomicRing::<F, D>::zero(); n_a];
            num_live_blocks
        ]);
    }
    let digit_bound = source.digit_abs_bound(shape);
    let safe_width = safe_crt_chunk_width::<F, W, K, D>(params, inner_width, digit_bound)
        .ok_or_else(|| AkitaError::InvalidSetup(capacity_error.into()))?;
    let chunk_width = capacity_safe_i8_chunk_width(safe_width, inner_width, shape.num_digits);
    if n_a <= DENSE_I8_BLOCK_PARALLEL_MAX_ROWS
        && num_live_blocks >= DENSE_I8_BLOCK_PARALLEL_MIN_BLOCKS
        && inner_width == max_data_width
    {
        if inner_width <= safe_width {
            return mat_vec_mul_digits_i8_block_parallel::<F, S, W, K, D, CHECK_ZERO>(
                ntt_mat,
                source,
                blocks,
                shape,
                digit_bound,
                params,
            );
        }
        return mat_vec_mul_digits_i8_block_parallel_chunked::<F, S, W, K, D, CHECK_ZERO>(
            ntt_mat,
            source,
            blocks,
            shape,
            inner_width,
            chunk_width,
            digit_bound,
            params,
        );
    }
    let lut = DigitMontLut::<W, K>::new_with_digit_bound(params, digit_bound);
    let batch = params.pointwise_dot_batch_size();
    let tile_width = aligned_i8_tile_width(
        base_tile_width::<W, K, D>(n_a),
        inner_width,
        shape.num_digits,
    );
    let max_rings = batch.max(shape.num_digits).div_ceil(shape.num_digits) + 1;
    drive_block_chunked_matvec(
        num_live_blocks,
        n_a,
        inner_width,
        safe_width,
        tile_width,
        chunk_width,
        params,
        || {
            (
                source.i8_scratch(shape, max_rings),
                vec![
                    vec![CyclotomicCrtNtt::<W, K, D>::zero(); batch.max(shape.num_digits)];
                    num_live_blocks
                ],
                vec![0usize; num_live_blocks],
            )
        },
        |accs, (scratch, transformed, live), start, end| {
            // Decode a ring-aligned batch once for each right-hand side, then
            // reuse its transforms across every matrix row.
            let batch_width = aligned_i8_tile_width(batch, end - start, shape.num_digits);
            for batch_start in (start..end).step_by(batch_width) {
                let batch_end = (batch_start + batch_width).min(end);
                for (block_idx, block) in blocks.iter().enumerate() {
                    let ring_start = batch_start / shape.num_digits;
                    if ring_start >= block.len() {
                        live[block_idx] = 0;
                        continue;
                    }
                    let ring_end = batch_end.div_ceil(shape.num_digits).min(block.len());
                    let offset = batch_start % shape.num_digits;
                    let rows = source.i8_rows(
                        block_idx,
                        block.start,
                        ring_start..ring_end,
                        shape,
                        scratch,
                    )?;
                    let n = (batch_end - batch_start).min(rows.len() - offset);
                    let digits = &rows[offset..offset + n];
                    if CHECK_ZERO && digits.iter().any(is_zero_plane) {
                        for (i, digit) in digits.iter().enumerate() {
                            if is_zero_plane(digit) {
                                continue;
                            }
                            transformed[block_idx][i].assign_i8_with_lut(digit, params, &lut);
                            for (acc, mat_row) in accs[block_idx].iter_mut().zip(ntt_mat) {
                                accumulate_pointwise_product_into(
                                    acc,
                                    &mat_row[batch_start + i],
                                    &transformed[block_idx][i],
                                    params,
                                );
                            }
                        }
                        live[block_idx] = 0;
                    } else {
                        live[block_idx] = n;
                        for (slot, digit) in transformed[block_idx].iter_mut().zip(digits) {
                            slot.assign_i8_with_lut(digit, params, &lut);
                        }
                    }
                }
                for (row_idx, mat_row) in ntt_mat.iter().enumerate() {
                    for (block_idx, (&n, digits_ntt)) in
                        live.iter().zip(transformed.iter()).enumerate()
                    {
                        for offset in (0..n).step_by(batch) {
                            let len = batch.min(n - offset);
                            accs[block_idx][row_idx].add_assign_pointwise_dot(
                                &mat_row[batch_start + offset..batch_start + offset + len],
                                &digits_ntt[offset..offset + len],
                                params,
                            );
                        }
                    }
                }
            }
            Ok(())
        },
    )
}

enum PackedI8Lift<W: PrimeWidth, const K: usize> {
    Balanced {
        log_basis: u32,
        digit_bound: u64,
        lut: DigitMontLut<W, K>,
    },
    Raw {
        rhs_bound: u64,
    },
}

impl<W: PrimeWidth, const K: usize> PackedI8Lift<W, K> {
    fn rhs_bound(&self) -> u64 {
        match self {
            Self::Balanced { digit_bound, .. } => *digit_bound,
            Self::Raw { rhs_bound } => *rhs_bound,
        }
    }

    fn range_error(&self) -> &'static str {
        match self {
            Self::Balanced { .. } => {
                "packed recursive digits exceed the CRT lift range for these parameters"
            }
            Self::Raw { .. } => {
                "raw packed recursive digits exceed the CRT lift range for these parameters"
            }
        }
    }

    fn width_error(&self) -> &'static str {
        match self {
            Self::Balanced { .. } => "packed recursive commitment block exceeds its row width",
            Self::Raw { .. } => "raw packed recursive commitment block exceeds its row width",
        }
    }
}

fn mat_vec_mul_packed_i8_with_params<
    F: Field + CanonicalEncoding,
    W: PrimeWidth,
    Decode,
    const K: usize,
    const D: usize,
>(
    ntt_mat: &[&[CyclotomicCrtNtt<W, K, D>]],
    num_live_blocks: usize,
    row_width: usize,
    decode_block: &Decode,
    params: &CrtNttParamSet<W, K, D>,
    lift: PackedI8Lift<W, K>,
) -> Result<Vec<Vec<CyclotomicRing<F, D>>>, AkitaError>
where
    Decode: Fn(usize) -> Result<Vec<[i8; D]>, AkitaError> + Sync,
{
    if num_live_blocks < DENSE_I8_BLOCK_PARALLEL_MIN_BLOCKS {
        return cfg_into_iter!(0..num_live_blocks)
            .map(|block_index| {
                let block = decode_block(block_index)?;
                match &lift {
                    PackedI8Lift::Balanced {
                        log_basis,
                        digit_bound,
                        ..
                    } => {
                        debug_assert!(digit_rows_within_digit_bound(
                            &block,
                            row_width.min(block.len()),
                            *digit_bound,
                        ));
                        let mut rows = mat_vec_mul_digits_i8_with_params(
                            ntt_mat,
                            &[block.as_slice()],
                            *log_basis,
                            params,
                        )?;
                        rows.pop().ok_or_else(|| {
                            AkitaError::Internal(
                                "single-block scaled digit matvec returned no batch".into(),
                            )
                        })
                    }
                    PackedI8Lift::Raw { .. } => {
                        let mut rows = mat_vec_mul_raw_digits_i8_with_params(
                            ntt_mat,
                            &[block.as_slice()],
                            params,
                        )?;
                        rows.pop().ok_or_else(|| {
                            AkitaError::Internal(
                                "single-block raw digit matvec returned no batch".into(),
                            )
                        })
                    }
                }
            })
            .collect();
    }

    let n_a = ntt_mat.len();
    let mat_width = ntt_mat.first().map_or(0, |row| row.len());
    let inner_width = mat_width.min(row_width);
    if inner_width == 0 || n_a == 0 {
        return Ok(vec![
            vec![CyclotomicRing::<F, D>::zero(); n_a];
            num_live_blocks
        ]);
    }
    let safe_width = safe_crt_chunk_width::<F, W, K, D>(params, inner_width, lift.rhs_bound())
        .ok_or_else(|| AkitaError::InvalidInput(lift.range_error().into()))?;

    cfg_into_iter!(0..num_live_blocks)
        .map(|block_index| {
            let block = decode_block(block_index)?;
            if block.len() > row_width {
                return Err(AkitaError::InvalidSetup(lift.width_error().into()));
            }
            let mut out = vec![CyclotomicRing::<F, D>::zero(); n_a];
            match &lift {
                PackedI8Lift::Balanced {
                    digit_bound, lut, ..
                } => {
                    debug_assert!(digit_rows_within_digit_bound(
                        &block,
                        inner_width.min(block.len()),
                        *digit_bound,
                    ));
                    let mut scratch = I8ColumnScratch::new();
                    for chunk_start in (0..block.len().min(inner_width)).step_by(safe_width) {
                        let chunk_end =
                            (chunk_start + safe_width).min(block.len().min(inner_width));
                        let mut accs = vec![CyclotomicCrtNtt::<W, K, D>::zero(); n_a];
                        accumulate_i8_columns::<W, K, D, true>(
                            &mut accs,
                            ntt_mat,
                            chunk_start,
                            &block[chunk_start..chunk_end],
                            params,
                            lut,
                            &mut scratch,
                        );
                        for (dst, acc) in out.iter_mut().zip(accs) {
                            *dst += acc.to_ring(params);
                        }
                    }
                }
                PackedI8Lift::Raw { .. } => {
                    for chunk_start in (0..block.len().min(inner_width)).step_by(safe_width) {
                        let chunk_end =
                            (chunk_start + safe_width).min(block.len().min(inner_width));
                        let mut accs = vec![CyclotomicCrtNtt::<W, K, D>::zero(); n_a];
                        for (offset, coefficients) in
                            block[chunk_start..chunk_end].iter().enumerate()
                        {
                            if is_zero_plane(coefficients) {
                                continue;
                            }
                            let column = chunk_start + offset;
                            let rhs = CyclotomicCrtNtt::from_i8_with_params(coefficients, params);
                            for (acc, matrix_row) in accs.iter_mut().zip(ntt_mat) {
                                accumulate_pointwise_product_into(
                                    acc,
                                    &matrix_row[column],
                                    &rhs,
                                    params,
                                );
                            }
                        }
                        for (dst, acc) in out.iter_mut().zip(accs) {
                            *dst += acc.to_ring(params);
                        }
                    }
                }
            }
            Ok(out)
        })
        .collect()
}

/// Block-streamed predecomposed mat-vec for a compact physical digit source.
///
/// `decode_block` materializes at most one commitment block per worker. The
/// shared digit LUT and matrix stay outside that decode boundary, preserving
/// the block-parallel kernel without retaining the full byte witness.
pub(super) fn mat_vec_mul_packed_digits_i8_with_params<
    F: Field + CanonicalEncoding,
    W: PrimeWidth,
    Decode,
    const K: usize,
    const D: usize,
>(
    ntt_mat: &[&[CyclotomicCrtNtt<W, K, D>]],
    num_live_blocks: usize,
    row_width: usize,
    decode_block: &Decode,
    log_basis: u32,
    params: &CrtNttParamSet<W, K, D>,
) -> Result<Vec<Vec<CyclotomicRing<F, D>>>, AkitaError>
where
    Decode: Fn(usize) -> Result<Vec<[i8; D]>, AkitaError> + Sync,
{
    let digit_bound = balanced_digit_abs_bound(log_basis);
    let lut = DigitMontLut::<W, K>::new_with_digit_bound(params, digit_bound);
    mat_vec_mul_packed_i8_with_params(
        ntt_mat,
        num_live_blocks,
        row_width,
        decode_block,
        params,
        PackedI8Lift::Balanced {
            log_basis,
            digit_bound,
            lut,
        },
    )
}

/// Fold-major (block) raw signed-i8 ring mat-vec for `num_digits_inner == 1`.
///
/// Mirrors [`mat_vec_mul_digits_i8_with_params`] exactly in block/column layout
/// and output shape, but treats each `[i8; D]` as a raw signed ring-coefficient
/// vector rather than a balanced gadget digit: it lifts with
/// `from_i8_with_params` (valid for any `i8`) instead of a balanced-digit LUT,
/// and sizes the CRT chunk width from the data-derived coefficient bound. This
/// is the commit path for a recursive witness whose extension-field tensor
/// base-lift packing (`pack_tensor_base_lift_i8_digits`) sums gadget digits and
/// can push coefficients past the balanced range `[-2^(log_basis-1),
/// 2^(log_basis-1))`. Degree-one fields keep the faster balanced-digit kernel.
pub(super) fn mat_vec_mul_raw_digits_i8_with_params<
    F: Field + CanonicalEncoding,
    W: PrimeWidth,
    const K: usize,
    const D: usize,
>(
    ntt_mat: &[&[CyclotomicCrtNtt<W, K, D>]],
    blocks: &[&[[i8; D]]],
    params: &CrtNttParamSet<W, K, D>,
) -> Result<Vec<Vec<CyclotomicRing<F, D>>>, AkitaError> {
    let num_live_blocks = blocks.len();
    if num_live_blocks == 0 {
        return Ok(vec![]);
    }
    let n_a = ntt_mat.len();
    let mat_width = ntt_mat.first().map_or(0, |row| row.len());
    let max_data_width = blocks.iter().map(|b| b.len()).max().unwrap_or(0);
    let inner_width = mat_width.min(max_data_width);
    if inner_width == 0 || n_a == 0 {
        return Ok(vec![
            vec![CyclotomicRing::<F, D>::zero(); n_a];
            num_live_blocks
        ]);
    }
    // Read the raw signed-i8 bound directly from the witness. It can in
    // principle be large enough that even a single CRT term cannot lift
    // exactly; reject that at this checked boundary rather than panicking.
    let rhs_bound = blocks
        .iter()
        .flat_map(|block| block.iter().take(inner_width))
        .flat_map(|row| row.iter())
        .map(|&coeff| u64::from(coeff.unsigned_abs()))
        .max()
        .unwrap_or(0);
    let safe_width = safe_crt_chunk_width::<F, W, K, D>(params, inner_width, rhs_bound)
        .ok_or_else(|| {
            AkitaError::InvalidInput(
                "raw i8 recursive-witness coefficients exceed the CRT lift range for these parameters"
                    .to_string(),
            )
        })?;
    drive_block_chunked_matvec(
        num_live_blocks,
        n_a,
        inner_width,
        safe_width,
        base_tile_width::<W, K, D>(n_a),
        safe_width,
        params,
        || (),
        |accs, _, start, end| {
            for block_idx in 0..num_live_blocks {
                let block = blocks[block_idx];
                if start >= block.len() {
                    continue;
                }
                let block_tile_end = end.min(block.len());
                let tile = &block[start..block_tile_end];
                for (i, coeff) in tile.iter().enumerate() {
                    if is_zero_plane(coeff) {
                        continue;
                    }
                    let col = start + i;
                    let ntt_d = CyclotomicCrtNtt::from_i8_with_params(coeff, params);
                    for (acc, mat_row) in accs[block_idx].iter_mut().zip(ntt_mat.iter()) {
                        accumulate_pointwise_product_into(acc, &mat_row[col], &ntt_d, params);
                    }
                }
            }
            Ok(())
        },
    )
}

/// Raw signed-i8 counterpart of
/// [`mat_vec_mul_packed_digits_i8_with_params`].
pub(super) fn mat_vec_mul_packed_raw_i8_with_params<
    F: Field + CanonicalEncoding,
    W: PrimeWidth,
    Decode,
    const K: usize,
    const D: usize,
>(
    ntt_mat: &[&[CyclotomicCrtNtt<W, K, D>]],
    num_live_blocks: usize,
    row_width: usize,
    rhs_bound: u64,
    decode_block: &Decode,
    params: &CrtNttParamSet<W, K, D>,
) -> Result<Vec<Vec<CyclotomicRing<F, D>>>, AkitaError>
where
    Decode: Fn(usize) -> Result<Vec<[i8; D]>, AkitaError> + Sync,
{
    mat_vec_mul_packed_i8_with_params(
        ntt_mat,
        num_live_blocks,
        row_width,
        decode_block,
        params,
        PackedI8Lift::Raw { rhs_bound },
    )
}
