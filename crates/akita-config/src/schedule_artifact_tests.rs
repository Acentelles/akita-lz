use akita_error::AkitaError;
use akita_params::{FoldSchedule, GroupOpenPhaseParams};

use crate::proof_optimized::fp128;
use crate::{policy_of, CommitmentConfig, RecursiveCommitmentConfig};

pub(crate) fn mutated_row_admission_error<Cfg: CommitmentConfig>(
    row: &akita_schedules::ResolvedScheduleRow,
    mutate: impl FnOnce(&mut FoldSchedule),
    chunking: akita_params::ChunkedWitnessCfg,
) -> AkitaError {
    let profiles = row.profiles().clone();
    let mut schedule = row.schedule().clone();
    mutate(&mut schedule);
    akita_schedules::ResolvedScheduleRow::try_new(profiles, schedule, &policy_of::<Cfg>(chunking))
        .expect_err("noncanonical row must fail at admission")
}

fn row_with_setup_prefix<Cfg: CommitmentConfig>(
    predicate: impl Fn(&GroupOpenPhaseParams) -> bool,
    chunking: akita_params::ChunkedWitnessCfg,
) -> akita_schedules::ResolvedScheduleRow {
    crate::test_support::workspace_schedule_catalog::<Cfg>(chunking)
        .expect("workspace schedule catalog")
        .rows()
        .find(|row| {
            row.schedule()
                .recursive_folds
                .iter()
                .any(|fold| fold.params.setup_prefix().is_some_and(&predicate))
        })
        .cloned()
        .expect("catalog row with matching setup prefix")
}

#[test]
fn row_admission_audits_recursive_setup_prefix_policy() {
    type RecursiveOneHot = RecursiveCommitmentConfig<fp128::OneHot>;

    let row = row_with_setup_prefix::<RecursiveOneHot>(
        |_| true,
        akita_params::ChunkedWitnessCfg::default_non_chunked(),
    );
    let prefix_index = row
        .schedule()
        .recursive_folds
        .iter()
        .position(|fold| fold.params.setup_prefix().is_some())
        .expect("recursive setup-prefix fold");
    let error = mutated_row_admission_error::<RecursiveOneHot>(
        &row,
        |schedule| {
            let fold = schedule
                .recursive_folds
                .get_mut(prefix_index)
                .expect("recursive setup-prefix fold");
            let mut prefix = *fold.params.setup_prefix().expect("setup-prefix group");
            prefix.opening.num_digits_open -= 1;
            fold.params
                .set_setup_prefix(Some(prefix))
                .expect("valid prefix topology");
        },
        akita_params::ChunkedWitnessCfg::default_non_chunked(),
    );
    assert!(
        error.to_string().contains(&format!(
            "recursive fold {prefix_index} setup prefix: opening digit depth is not canonical"
        )),
        "unexpected setup-prefix policy error: {error}"
    );
}

#[test]
fn dense_recursive_artifacts_cover_benchmark_sizes_and_offload_setup() {
    use crate::proof_optimized::{fp32, fp64};
    use crate::recursive_commitment::RecursiveScheduleConfig;
    use akita_params::PolynomialGroupLayout;
    use akita_params::ScheduleLookupKey;

    fn check<Cfg: RecursiveScheduleConfig>(sizes: &[usize]) {
        type Recursive<C> = RecursiveCommitmentConfig<C>;
        let catalog = crate::test_support::workspace_schedule_catalog::<Recursive<Cfg>>(
            akita_params::ChunkedWitnessCfg::default_non_chunked(),
        )
        .expect("dense recursive artifact admitted under current policy");
        assert_eq!(catalog.rows().count(), sizes.len());
        let mut offloaded = 0;
        for &num_vars in sizes {
            let key = ScheduleLookupKey::single(PolynomialGroupLayout::singleton(num_vars));
            let row = catalog.resolve_key(&key).expect("benchmark size present");
            if row
                .schedule()
                .recursive_folds
                .iter()
                .any(|fold| fold.params.setup_prefix().is_some())
            {
                offloaded += 1;
            }
        }
        assert!(
            offloaded > 0,
            "dense recursive family must actually offload setup"
        );
    }

    check::<fp32::Dense>(&[20, 22, 24, 26, 28, 30]);
    check::<fp64::Dense>(&[21, 23, 25, 27, 29]);
    check::<fp128::Dense>(&[20, 22, 24, 26, 28]);
}

#[test]
fn row_admission_pins_terminal_z_wire_parameters_to_the_cap() {
    use crate::proof_optimized::fp32;

    fn check<Cfg: CommitmentConfig>(linf: bool) {
        let row = crate::test_support::workspace_schedule_catalog::<Cfg>()
            .expect("workspace schedule catalog")
            .rows()
            .find(|row| {
                row.schedule().terminal.response_shape.layout.groups[0]
                    .z_linf_cap
                    .is_some()
                    == linf
            })
            .cloned()
            .expect("row with the requested terminal route");
        let group = row.schedule().terminal.response_shape.layout.groups[0];
        group
            .validate_canonical_z_wire()
            .expect("generated terminal wire parameters are canonical");

        let inflate: fn(&mut FoldSchedule) = |schedule| {
            let group = &mut schedule.terminal.response_shape.layout.groups[0];
            group.z_payload_bytes = group.z_payload_bytes.checked_mul(128).unwrap();
        };
        let widen: fn(&mut FoldSchedule) = |schedule| {
            schedule.terminal.response_shape.layout.groups[0].z_rice_low_bits += 1;
        };
        for mutate in [inflate, widen] {
            let error = mutated_row_admission_error::<Cfg>(&row, mutate);
            assert!(
                error.to_string().contains("not canonical"),
                "unexpected admission error: {error}"
            );
        }
    }

    check::<fp128::OneHot>(false);
    check::<fp32::OneHot>(true);
}
