use super::*;

macro_rules! dispatch_slot {
    ($slot:expr, $num_rows:expr, $num_cols:expr, $func:ident $(, $arg:expr)*) => {{
        let nr: usize = $num_rows;
        let nc: usize = $num_cols;
        if let Some(base) = $slot.q32_base() {
            let neg = base.negacyclic().ok_or_else(|| {
                    AkitaError::InvalidSetup("negacyclic NTT domain not prepared".into())
                })?;
                let rows: Vec<&[_]> = (0..nr).map(|i| &neg[i * nc..(i + 1) * nc]).collect();
            $func(&rows, $($arg,)* base.params())
        } else if let Some(base) = $slot.q64_base() {
            let neg = base.negacyclic().ok_or_else(|| {
                    AkitaError::InvalidSetup("negacyclic NTT domain not prepared".into())
                })?;
                let rows: Vec<&[_]> = (0..nr).map(|i| &neg[i * nc..(i + 1) * nc]).collect();
            $func(&rows, $($arg,)* base.params())
        } else if let Some(base) = $slot.q128_base() {
            let neg = base.negacyclic().ok_or_else(|| {
                    AkitaError::InvalidSetup("negacyclic NTT domain not prepared".into())
                })?;
                let rows: Vec<&[_]> = (0..nr).map(|i| &neg[i * nc..(i + 1) * nc]).collect();
            $func(&rows, $($arg,)* base.params())
        } else {
                return Err(AkitaError::InvalidSetup(
                    "signed-i8 matvec requires a base-profile NTT cache".into(),
                ));
        }
    }};
}

pub(crate) fn mat_vec_mul_ntt_i8<F: Field + CanonicalEncoding, const D: usize>(
    slot: &PreparedNttCache<D>,
    num_rows: usize,
    num_cols: usize,
    blocks: &[&[CyclotomicRing<F, D>]],
    num_digits: usize,
    log_basis: u32,
) -> Result<Vec<Vec<CyclotomicRing<F, D>>>, AkitaError> {
    validate_i8_log_basis(log_basis)?;
    let params = BalancedDecomposePow2Params::new(num_digits, log_basis);
    let basis = crate::sources::poly_helpers::SignedDigitBasis::new(log_basis)?;
    let shape = DigitShape {
        num_digits,
        log_basis,
    };
    blocks
        .iter()
        .map(|coeffs| {
            let source = LiveRings {
                coeffs,
                params: &params,
                basis,
            };
            mat_vec_mul_ntt_blocks_i8(
                slot,
                num_rows,
                num_cols,
                &source,
                std::slice::from_ref(&(0..coeffs.len())),
                shape,
                true,
            )?
            .pop()
            .ok_or_else(|| {
                AkitaError::Internal("single-block commitment matvec returned no batch".into())
            })
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn mat_vec_mul_ntt_blocks_i8<
    F: Field + CanonicalEncoding,
    S: I8Rows<D>,
    const D: usize,
>(
    slot: &PreparedNttCache<D>,
    num_rows: usize,
    num_cols: usize,
    source: &S,
    blocks: &[Range<usize>],
    shape: DigitShape,
    check_zero: bool,
) -> Result<Vec<Vec<CyclotomicRing<F, D>>>, AkitaError> {
    validate_i8_log_basis(shape.log_basis)?;
    if !check_zero && num_rows == 1 {
        return Ok(dispatch_slot!(
            slot,
            1usize,
            num_cols,
            mat_vec_mul_blocks_i8_single_row_with_params,
            source,
            blocks,
            shape
        )?
        .into_iter()
        .map(|ring| vec![ring])
        .collect());
    }
    macro_rules! run {
        ($zero:literal) => {{
            fn driver<
                F: Field + CanonicalEncoding,
                S: I8Rows<D>,
                W: PrimeWidth,
                const K: usize,
                const D: usize,
            >(
                matrix: &[&[CyclotomicCrtNtt<W, K, D>]],
                source: &S,
                blocks: &[Range<usize>],
                shape: DigitShape,
                params: &CrtNttParamSet<W, K, D>,
            ) -> Result<Vec<Vec<CyclotomicRing<F, D>>>, AkitaError> {
                mat_vec_mul_blocks_i8_with_params_impl::<F, S, W, K, D, $zero>(
                    matrix,
                    source,
                    blocks,
                    shape,
                    "i8 matvec CRT capacity cannot fit a single term",
                    params,
                )
            }
            dispatch_slot!(slot, num_rows, num_cols, driver, source, blocks, shape)
        }};
    }
    if check_zero {
        run!(true)
    } else {
        run!(false)
    }
}

/// Column-tiled A*x across multiple blocks of pre-decomposed i8 digit planes.
///
/// This uses one inner digit per coefficient. It skips the `CyclotomicRing -> i8 digit plane`
/// decomposition entirely because the caller already holds each coefficient as a
/// balanced digit plane for a validated `log_basis <= 8`.
#[tracing::instrument(skip_all, name = "mat_vec_mul_ntt_digits_i8")]
pub fn mat_vec_mul_ntt_digits_i8<F: Field + CanonicalEncoding, const D: usize>(
    slot: &PreparedNttCache<D>,
    num_rows: usize,
    num_cols: usize,
    blocks: &[&[[i8; D]]],
    log_basis: u32,
) -> Result<Vec<Vec<CyclotomicRing<F, D>>>, AkitaError> {
    validate_i8_log_basis(log_basis)?;
    for block in blocks {
        validate_digit_rows_for_log_basis(
            block,
            num_cols.min(block.len()),
            log_basis,
            "for predecomposed digit mat-vec",
        )?;
    }
    dispatch_slot!(
        slot,
        num_rows,
        num_cols,
        mat_vec_mul_digits_i8_with_params,
        blocks,
        log_basis
    )
}

/// Predecomposed mat-vec over a source that decodes one commitment block at a
/// time. This keeps the full byte witness out of memory while sharing one NTT
/// dispatch and one digit-LUT policy across all blocks.
pub(crate) fn mat_vec_mul_ntt_packed_digits_i8<
    F: Field + CanonicalEncoding,
    Decode,
    const D: usize,
>(
    slot: &PreparedNttCache<D>,
    num_rows: usize,
    row_width: usize,
    num_live_blocks: usize,
    decode_block: &Decode,
    log_basis: u32,
) -> Result<Vec<Vec<CyclotomicRing<F, D>>>, AkitaError>
where
    Decode: Fn(usize) -> Result<Vec<[i8; D]>, AkitaError> + Sync,
{
    validate_i8_log_basis(log_basis)?;
    dispatch_slot!(
        slot,
        num_rows,
        row_width,
        mat_vec_mul_packed_digits_i8_with_params,
        num_live_blocks,
        row_width,
        decode_block,
        log_basis
    )
}

/// Fold-major (block) direct-signed-i8 variant for recursive witnesses.
///
/// The block/column layout and output shape match
/// [`mat_vec_mul_ntt_digits_i8`], but this path does not assume the rows are
/// balanced gadget digits: it is the `num_digits_inner = 1` commit path for a
/// recursive witness whose extension-field tensor base-lift packing can push
/// coefficients past the balanced range. Coefficients too large for the CRT
/// lift are rejected as `AkitaError` rather than panicking.
#[tracing::instrument(skip_all, name = "mat_vec_mul_ntt_raw_digits_i8")]
#[cfg(test)]
pub(crate) fn mat_vec_mul_ntt_raw_digits_i8<F: Field + CanonicalEncoding, const D: usize>(
    slot: &PreparedNttCache<D>,
    num_rows: usize,
    num_cols: usize,
    blocks: &[&[[i8; D]]],
) -> Result<Vec<Vec<CyclotomicRing<F, D>>>, AkitaError> {
    dispatch_slot!(
        slot,
        num_rows,
        num_cols,
        mat_vec_mul_raw_digits_i8_with_params,
        blocks
    )
}

/// Raw signed counterpart of [`mat_vec_mul_ntt_packed_digits_i8`].
pub(crate) fn mat_vec_mul_ntt_packed_raw_i8<F: Field + CanonicalEncoding, Decode, const D: usize>(
    slot: &PreparedNttCache<D>,
    num_rows: usize,
    row_width: usize,
    num_live_blocks: usize,
    rhs_bound: u64,
    decode_block: &Decode,
) -> Result<Vec<Vec<CyclotomicRing<F, D>>>, AkitaError>
where
    Decode: Fn(usize) -> Result<Vec<[i8; D]>, AkitaError> + Sync,
{
    dispatch_slot!(
        slot,
        num_rows,
        row_width,
        mat_vec_mul_packed_raw_i8_with_params,
        num_live_blocks,
        row_width,
        rhs_bound,
        decode_block
    )
}
