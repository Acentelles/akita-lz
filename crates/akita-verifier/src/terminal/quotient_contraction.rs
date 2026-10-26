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
    #[cfg(not(feature = "terminal-contraction"))]
    let z_fields: Vec<F> = z
        .iter()
        .flat_map(|ring| ring.iter().map(|&value| F::from_i64(i64::from(value))))
        .collect();
    for (point_index, &point) in quotient.points.iter().enumerate() {
        let powers = power_row(point, D);
        let last = powers.last().copied().ok_or(AkitaError::InvalidProof)?;
        let shift = last * point + F::one();
        #[cfg(not(feature = "terminal-contraction"))]
        let z_evals: Vec<F> = z_fields
            .chunks_exact(D)
            .map(|ring| F::dot_product(ring, &powers))
            .collect();
        #[cfg(feature = "terminal-contraction")]
        let z_evals: Vec<F> = z
            .iter()
            .map(|ring| F::dot_product_i16(&powers, ring.iter().copied()))
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
                #[cfg(feature = "terminal-contraction")]
                {
                    let rings: Vec<&[F]> = row.chunks_exact(D).collect();
                    F::weighted_dot_product(&rings, &z_evals, &powers)
                }
                #[cfg(not(feature = "terminal-contraction"))]
                {
                    row.chunks_exact(D)
                        .zip(&z_evals)
                        .fold(F::zero(), |acc, (a, z_eval)| {
                            acc + F::dot_product(a, &powers) * *z_eval
                        })
                }
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

