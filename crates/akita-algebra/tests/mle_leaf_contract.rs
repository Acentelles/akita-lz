use akita_algebra::poly::multilinear_eval;
use jolt_field::{Prime128OffsetA7F7, Ring, Zero};

type F = Prime128OffsetA7F7;

#[test]
fn mle_leaf_multiaffine_contract() {
    for bits in 0..=10 {
        for seed in 0..16u64 {
            let point: Vec<F> = (0..bits).map(|i| F::from_u64(seed + i as u64)).collect();
            // Product of affine factors: independent polynomial ground truth.
            let table: Vec<F> = (0..1usize << bits)
                .map(|index| {
                    (0..bits).fold(F::from_u64(7), |acc, i| {
                        acc * F::from_u64(3 + i as u64 + 2 * ((index >> i) & 1) as u64)
                    })
                })
                .collect();
            let expected = point
                .iter()
                .enumerate()
                .fold(F::from_u64(7), |acc, (i, r)| {
                    acc * (F::from_u64(3 + i as u64) + F::from_u64(2) * *r)
                });
            assert_eq!(multilinear_eval(&table, &point).unwrap(), expected);
        }
    }
}

#[test]
fn mle_leaf_rejects_non_domains() {
    assert!(multilinear_eval::<F>(&[], &[]).is_err());
    assert!(multilinear_eval(&[F::zero(); 3], &[F::zero(); 2]).is_err());
    assert!(multilinear_eval(&[F::zero(); 32], &[F::zero(); 4]).is_err());
    assert!(multilinear_eval(&[F::zero()], &[F::zero(); usize::BITS as usize]).is_err());
}
