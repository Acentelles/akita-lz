use super::*;

struct PointGenerator(u64);

impl PointGenerator {
    fn word(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn scalar(&mut self) -> F {
        loop {
            let value = u128::from(self.word()) | (u128::from(self.word()) << 64);
            if let Some(value) = F::from_u128_checked(value) {
                return value;
            }
        }
    }

    fn equality(&mut self, bits: usize) -> Vec<F> {
        let point = (0..bits).map(|_| self.scalar()).collect::<Vec<_>>();
        EqPolynomial::evals(&point).unwrap()
    }
}

// Both OFF and ON execute this same row-oracle test. The fixture drives the
// fused path, including zero/one alpha in addition to the random points.
#[test]
fn direct_scan_2048_random_points_match_row_oracle() {
    let mut rng = PointGenerator(0x5345_5455_5045_5641);
    for role_dims in [
        CommitmentRingDims { inner: 64, outer: 64, opening: 64 },
        CommitmentRingDims { inner: 512, outer: 64, opening: 64 },
        CommitmentRingDims { inner: 256, outer: 128, opening: 64 },
        CommitmentRingDims { inner: 256, outer: 64, opening: 128 },
    ] {
        for point in 0..514 {
            let mut groups = Vec::new();
            for group in 0..2 {
                groups.push(test_group_plan(
                    4 * group..4 * group + 4, 8, 8, 2, 2,
                    rng.equality(2), rng.equality(3), rng.equality(3),
                    rng.equality(1), rng.equality(1),
                ));
            }
            let mut scan = finalize_test_plan(2, 8, groups, role_dims);
            let alpha = match point {
                512 => F::zero(),
                513 => F::one(),
                _ => rng.scalar(),
            };
            let DirectScanMode::Lifted { alpha: scan_alpha, .. } = &mut scan.mode else {
                panic!("lifted fixture");
            };
            *scan_alpha = alpha;
            let size = scan.plan().required() * scan.plan().projection_geometry().base_ring_dim();
            let setup = AkitaExpandedSetup::from_trusted_seed_derived_parts_unchecked(
                AkitaSetupDescriptor {
                    max_num_vars: 0, max_num_batched_polys: 0,
                    num_field_elements: size, setup_seed: [0u8; 32].into(),
                },
                FlatMatrix::from_flat_data((0..size).map(|_| rng.scalar()).collect()),
            );
            let powers_a = scalar_powers(alpha, role_dims.d_a());
            let powers_b = scalar_powers(alpha, role_dims.d_b());
            let powers_d = scalar_powers(alpha, role_dims.d_d());
            let expected = scan.evaluate_direct_by_rows(
                &setup, &powers_a, &powers_b, &powers_d, role_dims.d_a(),
            ).unwrap();
            assert_eq!(scan.evaluate_direct(&setup).unwrap(), expected);
        }
    }
}
