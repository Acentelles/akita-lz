use super::*;
use jolt_field::{One, Prime128OffsetA7F7};

type F = Prime128OffsetA7F7;

#[test]
fn sparse_windows_preserve_wraps_and_distinct_values() {
    const D: usize = 64;
    for position in 0..D {
        let challenges: Vec<_> = [i8::MIN, 33, -31, i8::MAX]
            .into_iter()
            .map(|coefficient| SparseChallenge {
                positions: vec![position as u32, ((position + 1) % D) as u32].into(),
                coeffs: vec![coefficient, -1].into(),
            })
            .collect();
        let values: Vec<_> = (0..challenges.len())
            .map(|index| {
                CyclotomicRing::<F, D>::from_coefficients(std::array::from_fn(|k| {
                    F::from_i64((index as i64 + 1) * (k as i64 - 17))
                }))
            })
            .collect();
        let expected = challenges.iter().zip(&values).fold(
            CyclotomicRing::zero(),
            |sum, (challenge, value)| {
                let mut dense = CyclotomicRing::<F, D>::zero();
                for (&position, &coefficient) in challenge.positions.iter().zip(&challenge.coeffs) {
                    dense.coeffs[position as usize] = F::from_i64(i64::from(coefficient));
                }
                sum + dense * *value
            },
        );
        assert_eq!(
            sparse_challenge_dot(&challenges, values.iter()),
            Ok(expected)
        );
    }
}

#[test]
fn sparse_windows_preserve_degenerate_shapes_and_error_precedence() {
    let empty = SparseChallenge {
        positions: Default::default(),
        coeffs: Default::default(),
    };
    let zero = CyclotomicRing::<F, 0>::zero();
    assert_eq!(
        sparse_challenge_dot(&[empty], [&zero].into_iter()),
        Ok(zero)
    );
    let challenge = SparseChallenge {
        positions: vec![0].into(),
        coeffs: vec![i8::MIN].into(),
    };
    let expected = challenge.validate::<0>().unwrap_err();
    assert_eq!(
        sparse_challenge_dot(std::slice::from_ref(&challenge), [&zero].into_iter()),
        Err(expected)
    );
    let one = CyclotomicRing::<F, 1>::from_coefficients([-F::one()]);
    assert_eq!(
        sparse_challenge_dot(&[challenge], [&one].into_iter()),
        Ok(CyclotomicRing::from_coefficients([F::from_u64(128)]))
    );
    let malformed = SparseChallenge {
        positions: vec![0].into(),
        coeffs: vec![0].into(),
    };
    assert_eq!(
        sparse_challenge_dot::<F, 1>(&[malformed], std::iter::empty()),
        Err(AkitaError::InvalidProof)
    );
}
