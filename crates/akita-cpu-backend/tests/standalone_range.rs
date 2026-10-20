//! Public standalone Stage-1 proof-stream roundtrips.
use akita_cpu_backend::standalone::DigitRangeProver;
use akita_params::{
    ChallengeFieldOrder, DigitRangePlan, FlatBooleanDomain, GrindingPlan, GrindingRun,
    GrindingSite, SumcheckProtocol,
};
use akita_types::{DigitRangeEqualityPoint, ProverGrinding, VerifierGrinding};
use akita_verifier::Stage1Verifier;
use jolt_field::{One, Prime128OffsetA7F7 as F, Ring};

fn grinding_plan(plan: DigitRangePlan, rounds: u32) -> GrindingPlan {
    let order = ChallengeFieldOrder::from_full_capacity(128).unwrap();
    let mut runs = Vec::new();
    let products = plan.product_stage_arities().len() as u32;
    for stage in 0..=products {
        for round in 0..rounds {
            runs.push(
                GrindingRun::proof_of_work(
                    GrindingSite::SumcheckRound {
                        protocol: SumcheckProtocol::Stage1,
                        level: 0,
                        stage,
                        round,
                    },
                    1,
                    order,
                )
                .unwrap(),
            );
        }
        if stage < products {
            runs.push(
                GrindingRun::proof_of_work(
                    GrindingSite::Stage1InterstageBatch { level: 0, stage },
                    1,
                    order,
                )
                .unwrap(),
            );
        }
    }
    GrindingPlan::new(runs, order).unwrap()
}

#[test]
fn standalone_range_roundtrip_and_tampering() {
    for basis in [4, 8, 16, 32, 64] {
        let plan = DigitRangePlan::new(basis).unwrap();
        let domain = FlatBooleanDomain::new(6, 3).unwrap();
        let equality = DigitRangeEqualityPoint::from_column_then_ring_challenges(
            &[F::from_u64(3), F::from_u64(5), F::from_u64(7)],
            2,
            1,
        )
        .unwrap();
        let half = (basis / 2) as i8;
        let digits = vec![-half, half - 1, -1, 0, 1, -half];
        let grinding = grinding_plan(plan, 3);
        let state = akita_transcript::new_prover_channel(b"standalone-range", b"test").unwrap();
        let mut prover = ProverGrinding::new(state, &grinding);
        let output = DigitRangeProver::new(digits.clone().into(), plan, domain, equality.clone())
            .unwrap()
            .prove::<F>(&mut prover, 0)
            .unwrap();
        let proof = prover.finish().unwrap();
        let verify = |bytes: &[u8]| {
            let state =
                akita_transcript::new_verifier_channel(b"standalone-range", b"test", bytes)?;
            let mut verifier = VerifierGrinding::new(state, &grinding);
            let result =
                Stage1Verifier::new(equality.clone(), plan).verify::<F>(&mut verifier, None, 0)?;
            verifier.finish()?;
            Ok::<_, akita_error::AkitaError>(result)
        };
        let verified = verify(&proof).unwrap();
        assert_eq!(verified.point, output.point);
        assert_eq!(
            verified.range_image_evaluation,
            output.range_image_evaluation
        );
        // Independently evaluate the padded range-image MLE at the returned point.
        let mut images: Vec<F> = digits
            .iter()
            .map(|&d| F::from_i64(i64::from(d) * (i64::from(d) + 1)))
            .collect();
        images.resize(8, F::from_u64(0));
        for &r in &output.point {
            images = images
                .chunks_exact(2)
                .map(|pair| pair[0] * (F::one() - r) + pair[1] * r)
                .collect();
        }
        assert_eq!(images, vec![output.range_image_evaluation]);
        let mut tampered = proof.clone();
        *tampered.last_mut().unwrap() ^= 1;
        assert!(verify(&tampered).is_err());
        assert!(verify(&proof[..proof.len() - 1]).is_err());
        for bad in [half, -half - 1] {
            let mut invalid = digits.clone();
            invalid[0] = bad;
            assert!(DigitRangeProver::new(invalid.into(), plan, domain, equality.clone()).is_err());
        }
    }
}
