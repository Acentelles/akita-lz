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
    assert!(
        small.schedule().root.params.inner().digits.num_digits
            < bounded.schedule().root.params.inner().digits.num_digits
    );
}

#[test]
fn small_dense_bound_and_planned_digits() {
    check::<3>();
    check::<4>();
    check::<5>();
    check::<6>();
}
