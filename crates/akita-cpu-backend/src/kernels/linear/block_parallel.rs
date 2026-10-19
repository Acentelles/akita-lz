use super::*;

pub(super) struct I8ColumnScratch<W: PrimeWidth, const K: usize, const D: usize> {
    rhs: [[MontCoeff<W>; D]; K],
    lazy_dot: [[MontCoeff<W>; D]; I32_LAZY_DOT_BATCH],
}

impl<W: PrimeWidth, const K: usize, const D: usize> I8ColumnScratch<W, K, D> {
    pub(super) fn new() -> Self {
        Self {
            rhs: [[MontCoeff::from_raw(W::default()); D]; K],
            lazy_dot: [[MontCoeff::from_raw(W::default()); D]; I32_LAZY_DOT_BATCH],
        }
    }
}

pub(super) fn accumulate_i8_columns<
    W: PrimeWidth,
    const K: usize,
    const D: usize,
    const CHECK_ZERO: bool,
>(
    accs: &mut [CyclotomicCrtNtt<W, K, D>],
    ntt_mat: &[&[CyclotomicCrtNtt<W, K, D>]],
    column_start: usize,
    digits: &[[i8; D]],
    params: &CrtNttParamSet<W, K, D>,
    lut: &DigitMontLut<W, K>,
    scratch: &mut I8ColumnScratch<W, K, D>,
) {
    let batch_size = params.pointwise_dot_batch_size();
    if batch_size == 1 {
        for (offset, digit) in digits.iter().enumerate() {
            if CHECK_ZERO && is_zero_plane(digit) {
                continue;
            }
            CyclotomicCrtNtt::add_assign_col_pointwise_mul_i8_multi_with_lut_scratch(
                accs,
                ntt_mat,
                column_start + offset,
                digit,
                params,
                lut,
                &mut scratch.rhs,
            );
        }
        return;
    }

    for batch_start in (0..digits.len()).step_by(batch_size) {
        let batch_end = (batch_start + batch_size).min(digits.len());
        let batch = &digits[batch_start..batch_end];
        if CHECK_ZERO && batch.iter().any(is_zero_plane) {
            for (offset, digit) in batch.iter().enumerate() {
                if is_zero_plane(digit) {
                    continue;
                }
                CyclotomicCrtNtt::add_assign_col_pointwise_mul_i8_multi_with_lut_scratch(
                    accs,
                    ntt_mat,
                    column_start + batch_start + offset,
                    digit,
                    params,
                    lut,
                    &mut scratch.rhs,
                );
            }
            continue;
        }

        if batch.len() == 1 {
            CyclotomicCrtNtt::add_assign_col_pointwise_mul_i8_multi_with_lut_scratch(
                accs,
                ntt_mat,
                column_start + batch_start,
                &batch[0],
                params,
                lut,
                &mut scratch.rhs,
            );
            continue;
        }

        CyclotomicCrtNtt::add_assign_col_pointwise_dot_i8_multi_with_lut_scratch(
            accs,
            ntt_mat,
            column_start + batch_start,
            batch,
            params,
            lut,
            &mut scratch.lazy_dot,
        );
    }
}

/// Block-parallel fast path for small `n_a` and many blocks.
pub(super) fn mat_vec_mul_digits_i8_block_parallel<
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
    digit_bound: u64,
    params: &CrtNttParamSet<W, K, D>,
) -> Result<Vec<Vec<CyclotomicRing<F, D>>>, AkitaError> {
    let width = blocks
        .iter()
        .map(|b| b.len() * shape.num_digits)
        .max()
        .unwrap_or(0);
    mat_vec_mul_digits_i8_block_parallel_chunked::<F, S, W, K, D, CHECK_ZERO>(
        ntt_mat,
        source,
        blocks,
        shape,
        width,
        width.max(1),
        digit_bound,
        false,
        params,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn mat_vec_mul_digits_i8_block_parallel_chunked<
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
    inner_width: usize,
    chunk_width: usize,
    digit_bound: u64,
    raw: bool,
    params: &CrtNttParamSet<W, K, D>,
) -> Result<Vec<Vec<CyclotomicRing<F, D>>>, AkitaError> {
    let n_a = ntt_mat.len();
    let lut =
        DigitMontLut::<W, K>::new_with_digit_bound(params, if raw { 128 } else { digit_bound });
    // A ring batch keeps full lazy-dot groups intact without tying decode
    // scratch to the CRT chunk width.
    let decode_rings = params.pointwise_dot_batch_size();
    let init = || {
        (
            source.i8_scratch(shape, decode_rings),
            I8ColumnScratch::new(),
            vec![CyclotomicCrtNtt::<W, K, D>::zero(); n_a],
        )
    };
    let apply = |(digits, scratch, accs): &mut (
        S::I8Scratch,
        I8ColumnScratch<W, K, D>,
        Vec<CyclotomicCrtNtt<W, K, D>>,
    ),
                 (block_index, block): (usize, &Range<usize>)| {
        let live_width = (block.len() * shape.num_digits).min(inner_width);
        let chunk_width = if raw {
            let rhs_bound = i8_block_abs_bound(
                source,
                block_index,
                block,
                shape,
                inner_width,
                decode_rings,
                digits,
            )?;
            safe_crt_chunk_width::<F, W, K, D>(params, inner_width, rhs_bound)
                .ok_or_else(|| AkitaError::InvalidInput(
                    "raw i8 recursive-witness coefficients exceed the CRT lift range for these parameters".into(),
                ))?
        } else {
            chunk_width
        };
        let mut out = vec![CyclotomicRing::<F, D>::zero(); n_a];
        for start in (0..live_width).step_by(chunk_width) {
            let end = (start + chunk_width).min(live_width);
            accs.fill(CyclotomicCrtNtt::zero());
            let mut column = start;
            while column < end {
                let ring_start = column / shape.num_digits;
                let ring_end = (ring_start + decode_rings).min(end.div_ceil(shape.num_digits));
                let batch_end = (ring_end * shape.num_digits).min(end);
                let offset = column % shape.num_digits;
                let rows = source.i8_rows(
                    block_index,
                    block.start,
                    ring_start..ring_end,
                    shape,
                    digits,
                )?;
                accumulate_i8_columns::<W, K, D, CHECK_ZERO>(
                    accs,
                    ntt_mat,
                    column,
                    &rows[offset..offset + batch_end - column],
                    params,
                    &lut,
                    scratch,
                );
                column = batch_end;
            }
            for (dst, acc) in out.iter_mut().zip(accs.iter()) {
                *dst += acc.to_ring(params);
            }
        }
        Ok(out)
    };
    #[cfg(feature = "parallel")]
    {
        blocks
            .par_iter()
            .enumerate()
            .map_init(init, apply)
            .collect()
    }
    #[cfg(not(feature = "parallel"))]
    {
        let mut scratch = init();
        blocks
            .iter()
            .enumerate()
            .map(|block| apply(&mut scratch, block))
            .collect()
    }
}

pub(super) fn mat_vec_mul_blocks_i8_single_row_with_params<
    F: Field + CanonicalEncoding,
    S: I8Rows<D>,
    W: PrimeWidth,
    const K: usize,
    const D: usize,
>(
    ntt_mat: &[&[CyclotomicCrtNtt<W, K, D>]],
    source: &S,
    blocks: &[Range<usize>],
    shape: DigitShape,
    params: &CrtNttParamSet<W, K, D>,
) -> Result<Vec<CyclotomicRing<F, D>>, AkitaError> {
    let mat_width = ntt_mat.first().map_or(0, |row| row.len());
    let max_data_width = blocks
        .iter()
        .map(|b| b.len().saturating_mul(shape.num_digits))
        .max()
        .unwrap_or(0);
    let inner_width = mat_width.min(max_data_width);
    if inner_width == 0 || blocks.is_empty() {
        return Ok(vec![CyclotomicRing::zero(); blocks.len()]);
    }
    let digit_bound = source.digit_abs_bound(shape);
    let safe_width = safe_crt_chunk_width::<F, W, K, D>(params, inner_width, digit_bound)
        .ok_or_else(|| {
            AkitaError::InvalidSetup("single-row i8 CRT capacity cannot fit a single term".into())
        })?;
    let chunk_width = capacity_safe_i8_chunk_width(safe_width, inner_width, shape.num_digits);
    if inner_width <= safe_width && inner_width == max_data_width
        || blocks.len() >= DENSE_I8_BLOCK_PARALLEL_MIN_BLOCKS
    {
        return Ok(
            mat_vec_mul_digits_i8_block_parallel_chunked::<F, S, W, K, D, false>(
                ntt_mat,
                source,
                blocks,
                shape,
                inner_width,
                chunk_width,
                digit_bound,
                false,
                params,
            )?
            .into_iter()
            .map(|mut row| row.remove(0))
            .collect(),
        );
    }
    let lut = DigitMontLut::<W, K>::new_with_digit_bound(params, digit_bound);
    let decode_rings = params.pointwise_dot_batch_size();
    blocks
        .iter()
        .enumerate()
        .map(|(block_index, block)| {
            let live_width = inner_width.min(block.len() * shape.num_digits);
            let (out, _, _) = cfg_try_fold_reduce!(
                0..live_width.div_ceil(chunk_width),
                || (
                    CyclotomicRing::<F, D>::zero(),
                    source.i8_scratch(shape, decode_rings),
                    I8ColumnScratch::new()
                ),
                |(mut out, mut digits, mut scratch), chunk_idx| {
                    let start = chunk_idx * chunk_width;
                    let end = (start + chunk_width).min(live_width);
                    let mut acc = CyclotomicCrtNtt::<W, K, D>::zero();
                    let mut column = start;
                    while column < end {
                        let ring_start = column / shape.num_digits;
                        let ring_end =
                            (ring_start + decode_rings).min(end.div_ceil(shape.num_digits));
                        let batch_end = (ring_end * shape.num_digits).min(end);
                        let offset = column % shape.num_digits;
                        let rows = source.i8_rows(
                            block_index,
                            block.start,
                            ring_start..ring_end,
                            shape,
                            &mut digits,
                        )?;
                        accumulate_i8_columns::<W, K, D, false>(
                            std::slice::from_mut(&mut acc),
                            ntt_mat,
                            column,
                            &rows[offset..offset + batch_end - column],
                            params,
                            &lut,
                            &mut scratch,
                        );
                        column = batch_end;
                    }
                    out += acc.to_ring(params);
                    Ok::<_, AkitaError>((out, digits, scratch))
                },
                |(mut a, digits, scratch), (b, _, _)| {
                    a += b;
                    Ok((a, digits, scratch))
                }
            )?;
            Ok(out)
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn i8_block_abs_bound<S: I8Rows<D>, const D: usize>(
    source: &S,
    block_index: usize,
    block: &Range<usize>,
    shape: DigitShape,
    inner_width: usize,
    decode_rings: usize,
    scratch: &mut S::I8Scratch,
) -> Result<u64, AkitaError> {
    let live_width = (block.len() * shape.num_digits).min(inner_width);
    let mut bound = 0;
    for ring_start in (0..live_width.div_ceil(shape.num_digits)).step_by(decode_rings) {
        let ring_end = (ring_start + decode_rings).min(live_width.div_ceil(shape.num_digits));
        let rows = source.i8_rows(
            block_index,
            block.start,
            ring_start..ring_end,
            shape,
            scratch,
        )?;
        let width = (live_width - ring_start * shape.num_digits).min(rows.len());
        for coefficient in rows[..width].iter().flatten() {
            bound = bound.max(u64::from(coefficient.unsigned_abs()));
        }
    }
    Ok(bound)
}
