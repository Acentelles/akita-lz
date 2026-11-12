use super::groups::FoldGroups;
use super::*;
use crate::test_fixtures::{
    address_oracle_fixture, address_oracle_group_params, address_oracle_precommit,
    sample_layout_lp, sample_multi_group_root_params,
};
use crate::PolynomialGroupLayout;

#[test]
fn fold_groups_reject_empty_and_misordered_topologies_without_unwinding() {
    let empty = std::panic::catch_unwind(|| FoldGroups::try_from_vec(Vec::new()));
    assert!(matches!(empty, Ok(Err(AkitaError::InvalidSetup(_)))));

    let ordinary = *sample_layout_lp().own_group();
    let mut prefix = ordinary;
    prefix.setup_natural_len = Some(64);

    let sole_prefix = std::panic::catch_unwind(|| FoldGroups::try_from_vec(vec![prefix]));
    assert!(matches!(sole_prefix, Ok(Err(AkitaError::InvalidSetup(_)))));

    let late_prefix =
        std::panic::catch_unwind(|| FoldGroups::try_from_vec(vec![ordinary, prefix, ordinary]));
    assert!(matches!(late_prefix, Ok(Err(AkitaError::InvalidSetup(_)))));
}

#[test]
fn precommitted_params_reject_frozen_matrix_dimension_mismatch() {
    let (mut lp, _) = sample_multi_group_root_params();
    let precommitted = lp.preceding_group_mut_for_test(0).unwrap();
    precommitted
        .profile
        .outer
        .matrix
        .sis_table_key
        .ring_dimension /= 2;
    let err = precommitted
        .validate()
        .expect_err("frozen B dimension must match the serialized B matrix");
    assert!(matches!(err, AkitaError::InvalidSetup(_)));
}

#[test]
fn relation_geometry_revalidates_frozen_precommitted_profiles() {
    let (mut lp, batch) = address_oracle_fixture(2);
    lp.preceding_group_mut_for_test(0)
        .unwrap()
        .profile
        .outer
        .matrix
        .sis_table_key
        .ring_dimension /= 2;
    assert!(crate::RelationWitnessGeometry::for_level(&lp, &batch, 2).is_err());
}

#[test]
fn opening_validation_rejects_changed_policy_after_valid_input() {
    let (lp, _) = sample_multi_group_root_params();
    let params = *lp.preceding_group_params(0).unwrap();
    params.validate().unwrap();

    let mut changed = params;
    changed.profile.version = 0;
    assert!(matches!(
        changed.validate(),
        Err(AkitaError::InvalidSetup(message))
            if message == "unsupported committed-group profile version 0"
    ));

    let mut changed = params;
    changed.setup_natural_len = Some(0);
    assert!(matches!(
        changed.validate(),
        Err(AkitaError::InvalidSetup(message))
            if message == "setup-prefix inner digit depth 2 does not cover the full field width (43 digits)"
    ));

    let mut changed = params;
    changed.opening.log_basis_open = 0;
    assert!(matches!(
        changed.validate(),
        Err(AkitaError::InvalidSetup(message))
            if message == "precommitted exact fold plan is missing or inconsistent"
    ));

    changed.profile.version = 0;
    assert!(matches!(
        changed.validate(),
        Err(AkitaError::InvalidSetup(message))
            if message == "unsupported committed-group profile version 0"
    ));
}

#[test]
fn opening_validation_is_consistent_across_threads() {
    use crate::{
        BlockGeometry, CommitmentSliceCount, GadgetDigits, GroupCommitPhaseParams,
        GroupOpenPhaseParams, GroupOpeningPlan, InnerCommitMatrixParams, OpeningMethod,
        OuterCommitMatrixParams, RoleParams, SisModulusProfileId, SisTableDigest,
        DEFAULT_SIS_SECURITY_POLICY,
    };
    use akita_challenges::SparseChallengeConfig;
    use std::sync::Barrier;

    let modulus_profile = SisModulusProfileId::Q128OffsetA7F7;
    let values = [
        (1024, 16, 2, 5, 26, 52, 688, 128, 0, 4),
        (512, 32, 4, 7, 10, 40, 344, 64, 10, 5),
    ]
    .map(
        |(ring, live, positions, basis, digits, a_width, b_width, subring, pm2, fold)| {
            GroupOpenPhaseParams {
                profile: GroupCommitPhaseParams {
                    version: 4,
                    group: PolynomialGroupLayout::new(14, 1),
                    blocks: BlockGeometry::new(live, positions, 8),
                    outer_slice_count: CommitmentSliceCount::EIGHT,
                    inner: RoleParams::new(
                        GadgetDigits::new(basis, digits),
                        InnerCommitMatrixParams::new_unchecked(
                            DEFAULT_SIS_SECURITY_POLICY,
                            SisTableDigest::CURRENT,
                            modulus_profile,
                            1,
                            a_width,
                            53_477_172,
                            ring,
                        ),
                    ),
                    outer: RoleParams::new(
                        GadgetDigits::new(3, 43),
                        OuterCommitMatrixParams::new_unchecked(
                            DEFAULT_SIS_SECURITY_POLICY,
                            SisTableDigest::CURRENT,
                            modulus_profile,
                            1,
                            b_width,
                            7,
                            64,
                        ),
                    ),
                },
                opening: GroupOpeningPlan {
                    opening_method: OpeningMethod::SubringCoefficientPacking {
                        challenge_subring_dimension: subring,
                    },
                    fold_challenge_config: SparseChallengeConfig {
                        count_pm1: 31,
                        count_pm2: pm2,
                    },
                    log_basis_open: 3,
                    num_digits_open: 43,
                    num_digits_fold: fold,
                },
                setup_natural_len: None,
            }
        },
    );
    let mut invalid = values[0];
    invalid.profile.version = 0;
    let start = Barrier::new(4);
    std::thread::scope(|scope| {
        for _ in 0..4 {
            scope.spawn(|| {
                start.wait();
                for _ in 0..32 {
                    for params in &values {
                        params.validate().unwrap();
                    }
                    assert!(matches!(
                        invalid.validate(),
                        Err(AkitaError::InvalidSetup(message))
                            if message == "unsupported committed-group profile version 0"
                    ));
                }
            });
        }
    });
}

#[test]
fn opening_batch_validation_checks_every_distinct_group_in_a_repeated_run() {
    let repeated_fixture = || {
        let mut lp = address_oracle_group_params(64, 64, 64, 8);
        lp.set_precommitted_groups(vec![address_oracle_precommit(128, 64, 64, 16, 1); 3])
            .unwrap();
        let layouts = lp
            .precommitted_groups()
            .iter()
            .map(|group| group.profile.group)
            .collect::<Vec<_>>();
        let batch =
            OpeningClaimsLayout::from_root_groups(&layouts, PolynomialGroupLayout::new(4, 2))
                .expect("repeated opening layout");
        (lp, batch)
    };
    let corrupt = |lp: &mut CommittedGroupParams, group_index| {
        lp.preceding_group_mut_for_test(group_index)
            .unwrap()
            .profile
            .outer
            .matrix
            .sis_table_key
            .ring_dimension /= 2;
    };

    let (lp, batch) = repeated_fixture();
    assert_eq!(
        lp.validated_groups(&batch).expect("repeated groups").len(),
        batch.num_groups()
    );
    for corrupted in [vec![0], vec![2], vec![1, 2]] {
        let (mut lp, batch) = repeated_fixture();
        for &group_index in &corrupted {
            corrupt(&mut lp, group_index);
        }
        assert!(
            lp.validate_opening_batch(&batch).is_err(),
            "corrupting groups {corrupted:?} must fail validation"
        );
    }
}
