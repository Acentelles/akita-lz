use super::*;
use jolt_field::{One, Prime128OffsetA7F7 as F, Zero};

fn challenge(positions: Vec<u32>, coeffs: Vec<i8>) -> SparseChallenge {
    SparseChallenge {
        positions: positions.into(),
        coeffs: coeffs.into(),
    }
}

// Independent schoolbook ring reduction followed by Horner evaluation.
fn reference<const D: usize>(
    t: &[CyclotomicRing<F, D>],
    challenges: &[SparseChallenge],
    rows: usize,
    points: [F; 2],
) -> Vec<[F; 2]> {
    (0..rows)
        .map(|row| {
            let mut coefficients = [F::zero(); D];
            for (block, challenge) in challenges.iter().enumerate() {
                for (&position, &coefficient) in challenge.positions.iter().zip(&challenge.coeffs) {
                    for (j, value) in t[block * rows + row].coefficients().iter().enumerate() {
                        let exponent = position as usize + j;
                        let product = *value * F::from_i64(i64::from(coefficient));
                        if exponent < D {
                            coefficients[exponent] += product;
                        } else {
                            coefficients[exponent - D] -= product;
                        }
                    }
                }
            }
            points.map(|point| {
                coefficients
                    .iter()
                    .rev()
                    .fold(F::zero(), |acc, c| acc * point + *c)
            })
        })
        .collect()
}

fn differential<const D: usize>() {
    let challenges = vec![
        challenge(vec![(D - 1) as u32, 0, 1], vec![-2, 1, 2]),
        challenge(vec![0, (D - 1) as u32], vec![i8::MIN, i8::MAX]),
        challenge(Vec::new(), Vec::new()),
    ];
    for seed in 0..8 {
        let t: Vec<_> = (0..9)
            .map(|block| {
                CyclotomicRing::<F, D>::from_coefficients(std::array::from_fn(|j| {
                    F::from_i64((j * 11 + block * 17 + seed * 5) as i64 - 101)
                }))
            })
            .collect();
        for points in [
            [F::zero(), F::one()],
            [-F::one(), F::from_u64(17)],
            [F::from_u64(31 + seed as u64), F::from_u64(31 + seed as u64)],
        ] {
            assert_eq!(
                evaluate(&t, &challenges, 3, points).unwrap(),
                reference(&t, &challenges, 3, points)
            );
        }
    }
}

#[test]
fn transpose_matches_negacyclic_products_at_small_and_terminal_dimensions() {
    differential::<4>();
    differential::<64>();
}

#[test]
fn transpose_rejects_malformed_challenges_and_shapes() {
    let t = vec![CyclotomicRing::<F, 4>::zero(); 3];
    for bad in [
        challenge(vec![1, 1], vec![1, -1]),
        challenge(vec![4], vec![1]),
        challenge(vec![0], vec![0]),
        challenge(vec![0, 1], vec![1]),
    ] {
        assert!(evaluate(&t, &[bad], 3, [F::zero(), F::one()]).is_err());
    }
    let good = challenge(vec![0], vec![1]);
    assert!(evaluate(
        &t[..2],
        std::slice::from_ref(&good),
        3,
        [F::zero(), F::one()]
    )
    .is_err());
    assert!(evaluate(&t, &[good], usize::MAX, [F::zero(), F::one()]).is_err());
    assert!(evaluate(&t, &[], 0, [F::zero(), F::one()]).is_err());
}

#[test]
fn transpose_empty_products_are_zero() {
    let empty: Vec<CyclotomicRing<F, 4>> = Vec::new();
    assert_eq!(
        evaluate(&empty, &[], 3, [F::zero(), F::one()]).unwrap(),
        vec![[F::zero(); 2]; 3]
    );
    assert!(evaluate(&empty, &[], 0, [F::zero(), F::one()])
        .unwrap()
        .is_empty());
}

#[test]
fn transpose_feeds_the_existing_quotient_check() {
    const D: usize = 4;
    let setup = vec![
        F::from_u64(3),
        F::from_u64(5),
        F::from_u64(7),
        F::from_u64(11),
    ];
    let z = [[1i16, 0, 0, 0]];
    let t = vec![CyclotomicRing::<F, D>::from_coefficients([
        setup[0], setup[1], setup[2], setup[3],
    ])];
    let challenges = [challenge(vec![0], vec![1])];
    let quotient = super::super::quotient::TerminalQuotient {
        setup: &setup,
        coefficients: vec![F::zero(); D - 1],
        points: [F::zero(), F::from_u64(13)],
    };
    let rhs = evaluate(&t, &challenges, 1, quotient.points).unwrap();
    super::super::quotient::check_rows(&quotient, &rhs, &z).unwrap();
    let mut changed = t;
    changed[0] += CyclotomicRing::from_coefficients([F::one(), F::zero(), F::zero(), F::zero()]);
    let rhs = evaluate(&changed, &challenges, 1, quotient.points).unwrap();
    assert!(super::super::quotient::check_rows(&quotient, &rhs, &z).is_err());
}
