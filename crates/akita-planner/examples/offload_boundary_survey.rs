//! Plan and independently audit one cell of the setup-offload boundary survey.
//!
//! Arguments: <fp32|fp64|fp128> <dense|onehot> <chunks> <levels> <nv> <output-dir>.
//! Uses the existing recursive objective and root/first-child search domain;
//! zero offloads remain eligible. No automatic root threshold is applied.

use akita_config::{
    policy_of,
    proof_optimized::{fp128, fp32, fp64},
    CommitmentConfig,
};
use akita_params::{
    active_setup_field_len, ChunkedWitnessCfg, CommittedGroupBatchProfile, GroupCommitPhaseParams,
    PolynomialGroupLayout, ScheduleLookupKey,
};
use akita_planner::{find_schedule, RecursiveSetupSearchPolicy, SelectionPolicyId};
use akita_schedules::{
    planner_support::first_direct_setup_capacity_for_schedule, ValidatedScheduleCatalog,
};
use std::{env, fs, path::Path, time::Instant};

fn run<Cfg: CommitmentConfig>(
    label: &str,
    chunks: usize,
    levels: usize,
    nv: usize,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut policy = policy_of::<Cfg>();
    policy.selection_policy =
        SelectionPolicyId::MinPaddedSetupEnvelopeThenFirstDirectThenExactProofAndWorkV6;
    policy.recursive_setup_search_policy = RecursiveSetupSearchPolicy::RootAndFirstChildV1;
    policy.witness_chunk = ChunkedWitnessCfg {
        num_chunks: chunks,
        num_activated_levels: levels,
    };
    let key = ScheduleLookupKey::single(PolynomialGroupLayout::singleton(nv));
    let started = Instant::now();
    let planned = find_schedule(
        &key,
        Cfg::committed_source_contract()?,
        &[],
        &policy,
        Cfg::ring_challenge_config,
    )?;
    let planning_secs = started.elapsed().as_secs_f64();
    let schedule = planned.schedule;
    let offload_levels = schedule
        .recursive_folds
        .iter()
        .enumerate()
        .filter(|(_, fold)| fold.params.setup_prefix().is_some())
        .map(|(producer_level, _)| producer_level.to_string())
        .collect::<Vec<_>>();
    let offloads = offload_levels.len();
    if offloads != planned.estimate.selected_offload_edges || offloads > 2 {
        return Err("selected offload count disagrees with canonical topology".into());
    }
    let root_setup = active_setup_field_len(&schedule.root.params, &key.opening_layout()?)?;
    let first_direct = first_direct_setup_capacity_for_schedule(&schedule, &key.opening_layout()?)?;
    let setup = planned.estimate.estimated_num_setup_field_elements;
    let proof = planned.estimate.estimated_proof_payload_bytes()?;
    let root_witness = schedule.root.input_witness_len;
    let folds = schedule.num_fold_levels();
    let profiles = CommittedGroupBatchProfile {
        final_group: GroupCommitPhaseParams::try_from_params(
            key.final_group,
            &schedule.root.params,
        )?,
        precommitteds: vec![],
    };
    let family = format!("{label}_w{chunks}r{levels}");
    let catalog = ValidatedScheduleCatalog::try_new(
        family.clone(),
        [(profiles, schedule)],
        &policy,
        Cfg::ring_challenge_config,
    )?;
    fs::create_dir_all(output)?;
    let destination = output.join(format!("{family}_nv{nv}.aks"));
    fs::write(&destination, catalog.to_artifact_bytes()?)?;
    println!("{label}\t{chunks}\t{levels}\t{nv}\t{offloads}\t{}\t{root_setup}\t{first_direct}\t{setup}\t{proof}\t{root_witness}\t{folds}\t{planning_secs:.6}\t{}",
        offload_levels.join(","), destination.display());
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 6 {
        return Err(
            "expected: <fp32|fp64|fp128> <dense|onehot> <chunks> <levels> <nv> <output-dir>".into(),
        );
    }
    let chunks = args[2].parse()?;
    let levels = args[3].parse()?;
    let nv = args[4].parse()?;
    let output = Path::new(&args[5]);
    match (args[0].as_str(), args[1].as_str()) {
        ("fp32", "dense") => run::<fp32::Dense>("fp32_dense", chunks, levels, nv, output),
        ("fp32", "onehot") => run::<fp32::OneHot>("fp32_onehot", chunks, levels, nv, output),
        ("fp64", "dense") => run::<fp64::Dense>("fp64_dense", chunks, levels, nv, output),
        ("fp64", "onehot") => run::<fp64::OneHot>("fp64_onehot", chunks, levels, nv, output),
        ("fp128", "dense") => run::<fp128::Dense>("fp128_dense", chunks, levels, nv, output),
        ("fp128", "onehot") => run::<fp128::OneHot>("fp128_onehot", chunks, levels, nv, output),
        _ => Err("unknown field/source pair".into()),
    }
}
