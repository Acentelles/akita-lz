//! Caller-selected small committed-source widths and generated digit depth.
use akita_config::{proof_optimized::fp128, CommitmentConfig, TrustedScheduleCatalog};
use akita_params::{ChunkedWitnessCfg, PolynomialGroupLayout, ScheduleLookupKey};

fn catalog<C: CommitmentConfig>() -> TrustedScheduleCatalog<C> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../artifacts/schedules")
        .join(format!("{}.aks", C::schedule_family_name()));
    TrustedScheduleCatalog::from_artifact_bytes(
        &std::fs::read(path).unwrap(),
        ChunkedWitnessCfg::default_non_chunked(),
    )
    .unwrap()
}

fn check<const BITS: u32>()
where
    fp128::DenseSmall<BITS>: CommitmentConfig,
{
    assert_eq!(fp128::DenseSmall::<BITS>::LOG_COMMIT_BOUND, BITS);
    assert_eq!(
        fp128::DenseSmall::<BITS>::schedule_family_name(),
        format!("fp128_dense_small_{BITS}")
    );
    assert_eq!(
        fp128::DenseSmall::<BITS>::committed_source_contract()
            .unwrap()
            .declared_bounds(),
        (Some(1 << (BITS - 1)), Some((1 << (BITS - 1)) - 1))
    );
    assert_eq!(
        fp128::DenseSmall::<BITS>::RING_DIMENSION_SCHEDULE,
        fp128::DenseBounded::RING_DIMENSION_SCHEDULE
    );
    assert_eq!(
        fp128::DenseSmall::<BITS>::decomposition().log_open_bound,
        Some(128)
    );
    let key = ScheduleLookupKey::single(PolynomialGroupLayout::singleton(14));
    let small = catalog::<fp128::DenseSmall<BITS>>();
    let bounded = catalog::<fp128::DenseBounded>();
    let small = small.resolve_key(&key).unwrap();
    let bounded = bounded.resolve_key(&key).unwrap();
    let small_digits = small.schedule().root.params.inner().digits.num_digits;
    let bounded_digits = bounded.schedule().root.params.inner().digits.num_digits;
    eprintln!(
        "signed width {BITS}, num_vars 14: {small_digits} inner digits \
         (DenseBounded: {bounded_digits})"
    );
    assert!(small_digits < bounded_digits);
}

#[test]
fn small_dense_bound_and_planned_digits() {
    check::<3>();
    check::<4>();
    check::<5>();
    check::<6>();
}

fn ckks_catalog<const BITS: u32>()
where
    fp128::DenseSmall<BITS>: CommitmentConfig,
{
    let catalog = catalog::<fp128::DenseSmall<BITS>>();
    let mut decreases = false;
    for arity in 13..=29 {
        let key = ScheduleLookupKey::single(PolynomialGroupLayout::singleton(arity));
        let row = catalog.resolve_key(&key).unwrap();
        let schedule = row.schedule();
        assert_eq!(schedule.root.params.inner().digits.num_digits, 1);
        assert_eq!(schedule.root.params.open().digits.log_basis, 5);
        let mut previous = 5;
        for step in &schedule.recursive_folds {
            assert_eq!(step.params.inner().digits.log_basis, previous);
            assert_eq!(step.params.inner().digits.num_digits, 1);
            let open = step.params.open().digits.log_basis;
            decreases |= open < previous;
            previous = open;
        }
        assert_eq!(schedule.terminal.inner.digits.log_basis, previous);
        decreases |= schedule.terminal.fold.log_basis < previous;
    }
    assert!(
        decreases,
        "catalog must exercise a decreasing opening basis"
    );
}

#[test]
fn ckks_catalogs_cover_all_arities_with_free_recursive_bases() {
    ckks_catalog::<5>();
    ckks_catalog::<6>();
}

#[test]
fn small_dense_collision_buckets_reflect_challenge_mass_not_source_width() {
    use akita_params::sis::{rounded_up_role_a_inf_norm, FoldChallengeNorms};
    let policy =
        akita_config::policy_of::<fp128::DenseSmall<5>>(ChunkedWitnessCfg::default_non_chunked());
    // Baseline arity-14 K=5 and K=6 selected different packing subrings.
    for (challenge_dimension, mass, expected) in [(64, 51, 835_380), (128, 31, 507_780)] {
        let challenge = fp128::DenseSmall::<5>::ring_challenge_config(challenge_dimension).unwrap();
        assert_eq!(FoldChallengeNorms::new(&challenge).l1_norm, mass);
        assert_eq!(
            rounded_up_role_a_inf_norm(
                policy.sis_security_policy,
                policy.sis_table_digest,
                policy.sis_modulus_profile,
                256,
                3,
                &challenge,
                4,
                1,
            ),
            Some(expected),
        );
        assert_eq!(expected, 4 * mass * 4095);
    }
}
