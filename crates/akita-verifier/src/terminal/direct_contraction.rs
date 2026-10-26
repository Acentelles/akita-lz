/// `Σ_{p,d} w_p·g_d·z[p·digits + d]` for base-field position weights `w` and
/// gadget scalars `g`: the consistency fold of the decoded `z`, computed as
/// one weight-vector dot product per ring coefficient.
fn base_reduced_z<F, const D: usize>(
    position_weights: &[F],
    gadget: &[F],
    z: &[[i16; D]],
    num_positions: usize,
) -> Result<CyclotomicRing<F, D>, AkitaError>
where
    F: Field + Ring,
{
    let terms = num_positions
        .checked_mul(gadget.len())
        .ok_or(AkitaError::InvalidProof)?;
    let position_weights = position_weights.get(..num_positions).ok_or_else(|| {
        AkitaError::InvalidInput("base multiplier product index out of range".into())
    })?;
    let z = z.get(..terms).ok_or(AkitaError::InvalidProof)?;
    let weights: Vec<F> = position_weights
        .iter()
        .flat_map(|&weight| gadget.iter().map(move |&scalar| weight * scalar))
        .collect();
    #[cfg(not(feature = "terminal-contraction"))]
    let mut column = Vec::with_capacity(terms);
    Ok(CyclotomicRing::from_coefficients(std::array::from_fn(
        |coefficient| {
            #[cfg(feature = "terminal-contraction")]
            {
                F::dot_product_i16(&weights, z.iter().map(|ring| ring[coefficient]))
            }
            #[cfg(not(feature = "terminal-contraction"))]
            {
                column.clear();
                column.extend(
                    z.iter()
                        .map(|ring| F::from_i64(i64::from(ring[coefficient]))),
                );
                F::dot_product(&weights, &column)
            }
        },
    )))
}

