use super::*;
use jolt_field::{Ring, Zero};

fn check<const BITS: u32>()
where
    fp128::DenseSmall<BITS>: CommitmentConfig<Field = F, ExtField = F>,
{
    init_rayon_pool();
    run_on_large_stack(|| {
        if BITS >= 5 {
            let scheme = load_workspace_scheme::<fp128::DenseSmall<BITS>>(
                ChunkedWitnessCfg::default_non_chunked(),
            )
            .unwrap();
            let row = scheme
                .schedules()
                .resolve_key(&akita_params::ScheduleLookupKey::single(
                    akita_params::PolynomialGroupLayout::singleton(19),
                ))
                .unwrap();
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
        prove_verify_dense_roundtrip_with_evals::<fp128::DenseSmall<BITS>>(
            ChunkedWitnessCfg::default_non_chunked(),
            if BITS >= 5 { &[14, 19] } else { &[14] },
            b"completeness/fp128_dense_small",
            |nv, _| {
                let mut values: Vec<F> = (0..1usize << nv)
                    .map(|i| F::from_u64((i % (1 << (BITS - 1))) as u64))
                    .collect();
                values[0] = -F::from_u64(1 << (BITS - 1));
                values
            },
        );
        let scheme = load_workspace_scheme::<fp128::DenseSmall<BITS>>(
            ChunkedWitnessCfg::default_non_chunked(),
        )
        .unwrap();
        let setup = scheme.setup_prover(14, 1).unwrap();
        let backend = CpuBackend::new(setup.expanded.clone()).unwrap();
        for outside in [
            F::from_u64(1 << (BITS - 1)),
            -F::from_u64((1 << (BITS - 1)) + 1),
        ] {
            let mut values = vec![F::zero(); 1 << 14];
            values[17] = outside;
            let poly = akita_cpu_backend::DensePoly::from_field_evals(14, &values).unwrap();
            let source = backend.import_source(vec![poly]).unwrap();
            let error = backend
                .commit(
                    scheme.schedules(),
                    &source,
                    akita_cpu_backend::GroupContext::scheduler_without_precommitted_groups(),
                )
                .map(|_| ())
                .expect_err("outside declared signed bound");
            assert!(matches!(error, akita_error::AkitaError::InvalidInput(_)));
        }
    });
}

#[test]
fn small_dense_roundtrip_and_signed_bounds() {
    check::<4>();
    check::<5>();
    check::<6>();
}
