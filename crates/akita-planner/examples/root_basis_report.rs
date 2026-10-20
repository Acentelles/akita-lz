//! Report-only root-basis sweep. Artifacts are restricted to a temporary directory.
use akita_config::{policy_of, proof_optimized::fp128, CommitmentConfig};
use akita_params::{
    ChunkedWitnessCfg, CommittedGroupBatchProfile, FoldSchedule, GroupCommitPhaseParams,
    PolynomialGroupLayout, ScheduleLookupKey,
};
use akita_planner::find_schedule;
use akita_schedules::{expanded_schedule_proof_estimate_bytes, ValidatedScheduleCatalog};
use std::{error::Error, path::Path, time::Instant};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn report<C: CommitmentConfig>(arity: usize, variant: u32, k: u32) -> Result<()> {
    let key = ScheduleLookupKey::single(PolynomialGroupLayout::singleton(arity));
    let mut policy = policy_of::<C>(ChunkedWitnessCfg::default_non_chunked());
    policy.root_opening_log_basis = None;
    policy.monotone_opening_basis = variant == 0 || variant == 1 || variant == 4;
    let roots: &[u32] = match variant {
        3 | 4 => &[5],
        5 => &[3, 4, 5, 6],
        _ => &[3],
    };
    let started = Instant::now();
    let mut best: Option<(usize, FoldSchedule, akita_planner::PlannerPolicy)> = None;
    for &root in roots {
        policy.root_opening_log_basis = (root != 3).then_some(root);
        let schedule = match find_schedule(
            &key,
            C::committed_source_contract()?,
            &[],
            &policy,
            C::ring_challenge_config,
        ) {
            Ok(planned) => planned.schedule,
            Err(akita_error::AkitaError::UnsupportedSchedule(_)) if variant == 5 => continue,
            Err(error) => return Err(error.into()),
        };
        let bytes = expanded_schedule_proof_estimate_bytes(&key, &schedule, &policy)?;
        if best.as_ref().is_none_or(|(old, _, _)| bytes < *old) {
            best = Some((bytes, schedule, policy));
        }
    }
    let seconds = started.elapsed().as_secs_f64();
    let (bytes, schedule, policy) = best.ok_or("no root candidate")?;
    let layout = &schedule.terminal.response_shape.layout;
    let e = layout.e_field_elems() * 16;
    let t = layout.t_field_elems() * 16;
    let z = akita_params::terminal_response_planner_bytes(
        128,
        &schedule.terminal.response_shape,
        schedule.terminal.response_l2_sq_cap(),
    )? - e
        - t
        - 4 * layout.groups.len();
    let other = bytes.checked_sub(z + e + t).ok_or("invalid byte split")?;
    let levels = std::iter::once(&schedule.root.params)
        .chain(schedule.recursive_folds.iter().map(|step| &step.params))
        .map(|p| {
            format!(
                "({},{})/{},{},{},{}",
                p.inner().digits.log_basis,
                p.open().digits.log_basis,
                p.inner().digits.num_digits,
                p.open().digits.num_digits,
                p.outer().digits.num_digits,
                p.num_digits_fold()
            )
        })
        .chain(std::iter::once(format!(
            "({},{})/{},-,-,{}T",
            schedule.terminal.inner.digits.log_basis,
            schedule.terminal.fold.log_basis,
            schedule.terminal.inner.digits.num_digits,
            schedule.terminal.fold.num_digits
        )))
        .collect::<Vec<_>>()
        .join("; ");
    println!("| {k} | V{variant} | {arity} | {bytes} | {z} | {e} | {t} | {other} | {seconds:.3} | {levels} |");
    if arity == 19 && [0, 1, 3].contains(&variant) {
        let dir = std::env::var("AKITA_REPORT_DIR")
            .unwrap_or_else(|_| "/tmp/akita-root-basis-report".into());
        let dir = Path::new(&dir);
        if !dir.starts_with("/tmp") {
            return Err("report artifacts must be under /tmp".into());
        }
        std::fs::create_dir_all(dir)?;
        let profile = CommittedGroupBatchProfile {
            final_group: GroupCommitPhaseParams::try_from_params(
                key.final_group,
                &schedule.root.params,
            )?,
            precommitteds: vec![],
        };
        let catalog = ValidatedScheduleCatalog::try_new(
            C::schedule_family_name(),
            [(profile, schedule)],
            &policy,
            C::ring_challenge_config,
        )?;
        std::fs::write(
            dir.join(format!("k{k}_v{variant}.aks")),
            catalog.to_artifact_bytes()?,
        )?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = std::env::args()
        .skip(1)
        .map(|arg| arg.parse::<u32>())
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let k = *args
        .first()
        .ok_or("usage: root_basis_report K VARIANT ARITY")?;
    let variant = *args.get(1).ok_or("missing variant")?;
    let arity = *args.get(2).ok_or("missing arity")? as usize;
    let arities: Vec<usize> = if arity == 0 {
        (13..=29).collect()
    } else {
        vec![arity]
    };
    let started = Instant::now();
    for arity in arities {
        if variant == 0 || k == 0 {
            report::<fp128::Dense>(arity, variant, 0)?;
        } else {
            match k {
                5 => report::<fp128::DenseSmall<5>>(arity, variant, k)?,
                6 => report::<fp128::DenseSmall<6>>(arity, variant, k)?,
                _ => return Err("K must be 0, 5 or 6".into()),
            }
        }
    }
    if arity == 0 {
        println!("CATALOG_SECONDS={:.3}", started.elapsed().as_secs_f64());
    }
    Ok(())
}
