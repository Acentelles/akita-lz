//! Manual reproducible measurements, using report-only catalogs under /tmp.
use super::*;
use akita_pcs::AkitaCommitmentScheme;
use jolt_field::Ring;
use std::time::Instant;
use tracing_subscriber::prelude::*;

#[derive(Clone)]
struct LegacySmall<const K: u32>;
impl<const K: u32> CommitmentConfig for LegacySmall<K>
where
    fp128::DenseSmall<K>: CommitmentConfig<Field = F, ExtField = F>,
{
    type Field = F;
    type ExtField = F;
    const RING_DIMENSION_SCHEDULE: akita_schedules::RingDimensionSchedule =
        <fp128::DenseSmall<K> as CommitmentConfig>::RING_DIMENSION_SCHEDULE;
    fn schedule_family_name() -> &'static str {
        fp128::DenseSmall::<K>::schedule_family_name()
    }
    fn decomposition() -> akita_params::DecompositionParams {
        fp128::DenseSmall::<K>::decomposition()
    }
    fn sis_modulus_profile() -> akita_params::SisModulusProfileId {
        fp128::DenseSmall::<K>::sis_modulus_profile()
    }
    fn ring_challenge_config(
        d: usize,
    ) -> Result<akita_challenges::SparseChallengeConfig, akita_error::AkitaError> {
        fp128::DenseSmall::<K>::ring_challenge_config(d)
    }
    fn opening_basis_range() -> (u32, u32) {
        (3, 6)
    }
    fn inner_basis_range() -> (u32, u32) {
        fp128::DenseSmall::<K>::inner_basis_range()
    }
    fn committed_source_class() -> akita_params::sis::CommittedSourceClass {
        fp128::DenseSmall::<K>::committed_source_class()
    }
}

fn measure<C: CommitmentConfig<Field = F, ExtField = F>>(k: u32, variant: u32) {
    const NV: usize = 19;
    const LABEL: &[u8] = b"root-basis-measurement";
    let path = format!("/tmp/akita-root-basis-report/k{k}_v{variant}.aks");
    let scheme = AkitaCommitmentScheme::<C>::from_schedule_artifact(
        &std::fs::read(path).unwrap(),
        ChunkedWitnessCfg::default_non_chunked(),
    )
    .unwrap();
    let key =
        akita_params::ScheduleLookupKey::single(akita_params::PolynomialGroupLayout::singleton(NV));
    let row = scheme.schedules().resolve_key(&key).unwrap();
    if variant == 3 {
        let schedule = row.schedule();
        let bases: Vec<_> = std::iter::once(schedule.root.params.open().digits.log_basis)
            .chain(
                schedule
                    .recursive_folds
                    .iter()
                    .map(|step| step.params.open().digits.log_basis),
            )
            .chain(std::iter::once(schedule.terminal.fold.log_basis))
            .collect();
        assert!(bases.windows(2).any(|pair| pair[1] < pair[0]));
    }
    // All variants use the same 5-bit chunks. K=5 centers them by subtracting 16.
    let values: Vec<F> = (0..1usize << NV)
        .map(|i| {
            let value = F::from_u64(((i * 17 + i / 31) % 32) as u64);
            if k == 5 {
                value - F::from_u64(16)
            } else {
                value
            }
        })
        .collect();
    let point = random_point(NV, 0xcafe_0000);
    let evaluation = dense_opening_lagrange(&values, &point);
    let poly = akita_cpu_backend::DensePoly::from_field_evals(NV, &values).unwrap();
    let setup = scheme.setup_prover(NV, 1).unwrap();
    let backend = CpuBackend::new(setup.expanded.clone()).unwrap();
    let verifier = scheme
        .verifier(scheme.setup_verifier(&setup).unwrap())
        .unwrap();
    let source = backend.import_source(vec![poly]).unwrap();
    let started = Instant::now();
    let output = backend
        .commit(
            scheme.schedules(),
            &source,
            akita_cpu_backend::GroupContext::scheduler_without_precommitted_groups(),
        )
        .unwrap();
    let commit_s = started.elapsed().as_secs_f64();
    let commitment = output.committed_group;
    let opening = prove_input::<C>(
        &point,
        &[evaluation],
        &commitment,
        output.private_handle,
        scheme.schedules(),
    );
    let started = Instant::now();
    let proof = scheme
        .batched_prove(&setup, opening, &backend, LABEL, BasisMode::Lagrange)
        .unwrap();
    let prove_s = started.elapsed().as_secs_f64();
    let started = Instant::now();
    verifier
        .batched_verify(
            &proof,
            LABEL,
            verify_input::<C>(&point, &[evaluation], &commitment, scheme.schedules()),
            BasisMode::Lagrange,
        )
        .unwrap();
    let verify_s = started.elapsed().as_secs_f64();
    println!("MEASURE k={k} variant=V{variant} bytes={} commit_s={commit_s:.6} prove_s={prove_s:.6} verify_s={verify_s:.6}", proof.len());
}

#[test]
#[ignore = "manual arity-19 performance measurement with temporary variant catalogs"]
fn small_dense_measurement() {
    let k: u32 = std::env::var("AKITA_MEASURE_K").unwrap().parse().unwrap();
    let variant: u32 = std::env::var("AKITA_MEASURE_VARIANT")
        .unwrap()
        .parse()
        .unwrap();
    let trace = format!("/tmp/akita-root-basis-report/k{k}_v{variant}.json");
    let (layer, guard) = tracing_chrome::ChromeLayerBuilder::new()
        .file(trace)
        .include_args(false)
        .include_locations(false)
        .build();
    tracing_subscriber::registry()
        .with(layer.with_filter(tracing_subscriber::filter::LevelFilter::INFO))
        .init();
    init_rayon_pool();
    run_on_large_stack(move || match (k, variant) {
        (0, 0) => measure::<fp128::Dense>(k, variant),
        (5, 1) => measure::<LegacySmall<5>>(k, variant),
        (6, 1) => measure::<LegacySmall<6>>(k, variant),
        (5, 3) => measure::<fp128::DenseSmall<5>>(k, variant),
        (6, 3) => measure::<fp128::DenseSmall<6>>(k, variant),
        _ => panic!("unsupported measurement variant"),
    });
    drop(guard);
}
