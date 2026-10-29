use akita_error::AkitaError;
use jolt_field::Field;

use crate::eq_poly::EqPolynomial;

// The public entry point checked length and little-endian domain order.
// A tile binds the four low variables. Higher variables keep their order.
pub(super) fn evaluate<E: Field>(evals: &[E], point: &[E]) -> Result<E, AkitaError> {
    const LOW_BITS: usize = 4;
    if point.len() <= LOW_BITS {
        return Ok(super::multilinear_eval_ref(evals, point));
    }
    let (low, high) = point.split_at(LOW_BITS);
    let weights = EqPolynomial::evals(low)?;
    Ok(contract(evals, high, &weights))
}

// No input-sized table: recursion retains one scalar per high variable.
// The existing Fp128 guest dot kernel batches each tile's field readout.
fn contract<E: Field>(evals: &[E], high: &[E], weights: &[E]) -> E {
    match high.split_last() {
        None => E::dot_product(evals, weights),
        Some((&r, rest)) => {
            let (lo, hi) = evals.split_at(evals.len() / 2);
            let lo = contract(lo, rest, weights);
            let hi = contract(hi, rest, weights);
            lo + r * (hi - lo)
        }
    }
}
