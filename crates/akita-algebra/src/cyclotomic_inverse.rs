//! Inversion in `Z_q[X]/(X^D + 1)` through the tower of even-power subrings.
//!
//! Akita-local until `jolt-algebra`'s `CyclotomicRing` carries an inherent
//! `inverse`; the dev-protocol terminal verifier inverts a fold challenge to
//! recover an omitted block.

use jolt_algebra::{CyclotomicRing, Field};

/// Multiplicative inverse of a cyclotomic ring element.
pub trait CyclotomicInverse: Sized {
    /// Multiplicative inverse, or `None` when `self` is not a unit or `D` is
    /// not a power of two.
    ///
    /// The conjugate `self(-X)` makes `self * self(-X)` invariant under
    /// `X -> -X`, so the product has only even powers and lives in
    /// `Z_q[Y]/(Y^{D/2} + 1)` with `Y = X^2`. Recursing down to degree one
    /// leaves a single field inversion, and
    /// `self^{-1} = self(-X) * (self * self(-X))^{-1}`.
    fn inverse(&self) -> Option<Self>;
}

impl<F: Field, const D: usize> CyclotomicInverse for CyclotomicRing<F, D> {
    fn inverse(&self) -> Option<Self> {
        if !D.is_power_of_two() {
            return None;
        }
        negacyclic_inverse(&self.coeffs).map(|inverse| Self::from_slice(&inverse))
    }
}

/// Inverse in `Z_q[X]/(X^n + 1)` for a power-of-two `n = value.len()`.
fn negacyclic_inverse<F: Field>(value: &[F]) -> Option<Vec<F>> {
    if let [constant] = value {
        return constant.inverse().map(|inverse| vec![inverse]);
    }
    let conjugate: Vec<F> = value
        .iter()
        .enumerate()
        .map(|(i, coeff)| if i % 2 == 0 { *coeff } else { -*coeff })
        .collect();
    let norm: Vec<F> = negacyclic_mul(value, &conjugate)
        .into_iter()
        .step_by(2)
        .collect();
    let norm_inverse = negacyclic_inverse(&norm)?;
    let mut lifted = vec![F::zero(); value.len()];
    for (dst, src) in lifted.iter_mut().step_by(2).zip(norm_inverse) {
        *dst = src;
    }
    Some(negacyclic_mul(&lifted, &conjugate))
}

/// Schoolbook product in `Z_q[X]/(X^n + 1)` for operands of equal length `n`.
fn negacyclic_mul<F: Field>(lhs: &[F], rhs: &[F]) -> Vec<F> {
    let mut out = vec![F::zero(); lhs.len()];
    for (i, lhs_coeff) in lhs.iter().copied().enumerate() {
        if lhs_coeff.is_zero() {
            continue;
        }
        let (out_wrap, out_direct) = out.split_at_mut(i);
        let (rhs_direct, rhs_wrap) = rhs.split_at(rhs.len().saturating_sub(i));
        for (dst, rhs_coeff) in out_direct.iter_mut().zip(rhs_direct) {
            *dst += lhs_coeff * *rhs_coeff;
        }
        for (dst, rhs_coeff) in out_wrap.iter_mut().zip(rhs_wrap) {
            *dst -= lhs_coeff * *rhs_coeff;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use jolt_algebra::{One, Prime128Offset275, Prime32Offset99, Prime64Offset59, Ring};
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    type F128 = Prime128Offset275;
    type F32 = Prime32Offset99;
    /// Production 64-bit prime.
    type F64Full = Prime64Offset59;
    const D: usize = 64;

    fn check_inverse_round_trips<F: Field, const D: usize>(seed: u64) {
        let mut rng = StdRng::seed_from_u64(seed);
        for _ in 0..8 {
            let value = CyclotomicRing::<F, D>::random(&mut rng);
            let inverse = value.inverse().expect("random element is a unit");
            assert_eq!(value * inverse, CyclotomicRing::one());
        }
        assert_eq!(CyclotomicRing::<F, D>::zero().inverse(), None);
        assert_eq!(
            CyclotomicRing::<F, D>::one().inverse(),
            Some(CyclotomicRing::one())
        );
    }

    #[test]
    fn inverse_round_trips() {
        check_inverse_round_trips::<F32, 1>(0x1a01);
        check_inverse_round_trips::<F32, 128>(0x1a02);
        check_inverse_round_trips::<F64Full, 128>(0x1a03);
        check_inverse_round_trips::<F128, 64>(0x1a04);
    }

    #[test]
    fn inverse_rejects_zero_divisors_and_non_power_of_two_degrees() {
        // `q = 2^32 - 99` is `5 mod 8`, so `2` is a non-residue and
        // `i = 2^((q - 1) / 4)` squares to `-1`. Then
        // `X^D + 1 = (X^{D/2} - i)(X^{D/2} + i)`.
        let i = F32::pow2((u32::MAX as usize - 99) / 4);
        assert_eq!(i * i, -F32::one());
        let mut factor = CyclotomicRing::<F32, D>::zero();
        factor.coeffs[0] = -i;
        factor.coeffs[D / 2] = F32::one();
        assert_eq!(factor.inverse(), None);

        assert_eq!(CyclotomicRing::<F32, 3>::one().inverse(), None);
    }
}
