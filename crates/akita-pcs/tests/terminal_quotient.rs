//! Terminal quotient certificate: runtime selection and its boundaries.
//!
//! The feature compiles the certificate in; `TerminalQuotientConfig<Cfg>`
//! selects it. Every test here proves and verifies in one build, so a
//! rejection is attributed to the one thing that differs between the prover's
//! and the verifier's configuration, or to the one change made to the bytes.
//!
//! `unselected_fp128_proof_fixture` runs in every build. With
//! `AKITA_TQ_FIXTURE_OUT` set it writes the unselected proof bytes, the
//! grinding-plan digest and the instance-descriptor bytes, so a feature-off
//! build and a feature-on build can be compared byte for byte.

#![allow(missing_docs)]

use akita_config::proof_optimized::fp128;
use akita_cpu_backend::{CpuBackend, DensePoly, GroupContext};
use akita_params::{lagrange_weights, BasisMode, OpeningClaimsLayout};
use akita_pcs::AkitaCommitmentScheme;
use akita_prover::SelectedProverOpeningData;
use akita_types::{GroupBatchStatement, OpeningClaims, PolynomialGroupClaims};
use jolt_field::{Ring, Zero};

type F = fp128::Field;
type Base = fp128::Dense;
const NV: usize = 14;
const LABEL: &[u8] = b"terminal-quotient/selection";

fn on_large_stack(body: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(body)
        .unwrap()
        .join()
        .unwrap();
}

fn evals() -> Vec<F> {
    (0..(1u64 << NV))
        .map(|index| F::from_u64(7 * index + 13))
        .collect()
}

fn point() -> Vec<F> {
    (0..NV)
        .map(|index| F::from_u64((index + 2) as u64))
        .collect()
}

fn opening() -> F {
    let weights = lagrange_weights(&point()).unwrap();
    evals()
        .iter()
        .zip(weights)
        .fold(F::zero(), |sum, (&value, weight)| sum + value * weight)
}

/// Commit and prove the fixed dense polynomial under configuration `$cfg`,
/// with the setup `$setup` and backend `$backend`. Evaluates to
/// `(scheme, commitment, selection, proof)`.
macro_rules! commit_and_prove {
    ($cfg:ty, $setup:expr, $backend:expr) => {{
        let scheme = AkitaCommitmentScheme::<$cfg>::new(
            akita_config::test_support::workspace_schedule_catalog::<$cfg>().unwrap(),
        );
        let poly = DensePoly::<F>::from_field_evals(NV, &evals()).unwrap();
        let output = $backend
            .commit(
                scheme.schedules(),
                &$backend.import_source(vec![poly]).unwrap(),
                GroupContext::scheduler_without_precommitted_groups(),
            )
            .unwrap();
        let commitment = output.committed_group;
        let group =
            PolynomialGroupClaims::new(point(), vec![opening()], commitment.clone()).unwrap();
        let prover_data = SelectedProverOpeningData::from_committed_claims::<$cfg>(
            OpeningClaims::from_groups(vec![group]).unwrap(),
            vec![output.private_handle],
            scheme.schedules(),
        )
        .unwrap();
        let selection = prover_data.selection();
        let proof =
            scheme.batched_prove($setup, prover_data, &$backend, LABEL, BasisMode::Lagrange);
        (scheme, commitment, selection, proof)
    }};
}

/// Verify `$proof` under configuration `$cfg` against the fixed statement.
macro_rules! verify_under {
    ($cfg:ty, $setup:expr, $commitment:expr, $selection:expr, $proof:expr) => {{
        let scheme = AkitaCommitmentScheme::<$cfg>::new(
            akita_config::test_support::workspace_schedule_catalog::<$cfg>().unwrap(),
        );
        let group = PolynomialGroupClaims::new(point(), vec![opening()], $commitment).unwrap();
        let claims = OpeningClaims::from_groups(vec![group]).unwrap();
        let statement = GroupBatchStatement::new($selection, claims).unwrap();
        scheme
            .verifier(scheme.setup_verifier($setup).unwrap())
            .and_then(|verifier| {
                verifier.batched_verify($proof, LABEL, statement, BasisMode::Lagrange)
            })
    }};
}

/// The unselected fp128 proof, its grinding-plan digest and its instance
/// descriptor. With `AKITA_TQ_FIXTURE_OUT` set they are written there, for a
/// byte comparison between a build without the feature and one with it.
#[test]
fn unselected_fp128_proof_fixture() {
    on_large_stack(|| {
        let base = AkitaCommitmentScheme::<Base>::new(
            akita_config::test_support::workspace_schedule_catalog::<Base>().unwrap(),
        );
        let setup = base.setup_prover(NV, 1).unwrap();
        let backend = CpuBackend::new(setup.expanded.clone()).unwrap();
        let (scheme, commitment, selection, proof) = commit_and_prove!(Base, &setup, backend);
        let proof = proof.unwrap();
        verify_under!(Base, &setup, &commitment, selection, &proof).unwrap();
        // A second honest run gives the same bytes (deterministic prover).
        let (_, _, _, again) = commit_and_prove!(Base, &setup, backend);
        assert_eq!(again.unwrap(), proof);

        let resolved = scheme.schedules().resolve_selection(selection).unwrap();
        let (plan, descriptor) = akita_config::transcript_instance_descriptor::<F, Base>(
            setup.expanded.descriptor(),
            &OpeningClaimsLayout::new(NV, 1).unwrap(),
            selection,
            resolved.schedule(),
            BasisMode::Lagrange,
        )
        .unwrap();
        assert!(!plan.runs().iter().any(|run| matches!(
            run.site(),
            akita_params::GrindingSite::TerminalQuotientPair { .. }
        )));
        if let Ok(path) = std::env::var("AKITA_TQ_FIXTURE_OUT") {
            let digest = plan.digest().unwrap();
            let mut out = Vec::new();
            for part in [proof.as_slice(), digest.as_slice(), descriptor.as_slice()] {
                out.extend_from_slice(&(part.len() as u64).to_le_bytes());
                out.extend_from_slice(part);
            }
            std::fs::write(path, out).unwrap();
        }
    });
}

#[cfg(feature = "terminal-quotient")]
mod selected {
    use super::*;
    use akita_config::proof_optimized::{fp32, fp64};
    use akita_config::{CommitmentConfig, TerminalQuotientConfig};
    use akita_error::AkitaError;
    use akita_params::GrindingSite;
    use jolt_field::{CanonicalBytes, ExtField};

    type Selected = TerminalQuotientConfig<Base>;

    fn quotient_pairs(plan: &akita_params::GrindingPlan) -> usize {
        plan.runs()
            .iter()
            .filter(|run| matches!(run.site(), GrindingSite::TerminalQuotientPair { .. }))
            .count()
    }

    /// The selected certificate proves and verifies, carries its plan entry,
    /// and is rejected by the base configuration, and the base proof by it:
    /// a selection mismatch between prover and verifier fails, whichever
    /// side selects. Each version's terminal grammar fed to the other also
    /// fails.
    #[test]
    fn selection_is_runtime_and_mismatches_reject() {
        on_large_stack(|| {
            let base = AkitaCommitmentScheme::<Base>::new(
                akita_config::test_support::workspace_schedule_catalog::<Base>().unwrap(),
            );
            let setup = base.setup_prover(NV, 1).unwrap();
            let backend = CpuBackend::new(setup.expanded.clone()).unwrap();
            let (_, commitment, selection, base_proof) = commit_and_prove!(Base, &setup, backend);
            let base_proof = base_proof.unwrap();
            let (scheme, _, selected_selection, selected_proof) =
                commit_and_prove!(Selected, &setup, backend);
            let selected_proof = selected_proof.unwrap();
            // Same schedule row: the adapter reuses the base schedule family.
            assert_eq!(selected_selection, selection);
            assert_ne!(selected_proof, base_proof);

            // Each proof verifies under its own selection.
            verify_under!(Base, &setup, &commitment, selection, &base_proof).unwrap();
            verify_under!(Selected, &setup, &commitment, selection, &selected_proof).unwrap();

            // Mismatched selection: rejected in both directions.
            assert!(verify_under!(Base, &setup, &commitment, selection, &selected_proof).is_err());
            assert!(verify_under!(Selected, &setup, &commitment, selection, &base_proof).is_err());

            // The selected plan carries exactly one pair entry; the base none.
            let resolved = scheme.schedules().resolve_selection(selection).unwrap();
            let layout = OpeningClaimsLayout::new(NV, 1).unwrap();
            let (selected_plan, _) = akita_config::transcript_instance_descriptor::<F, Selected>(
                setup.expanded.descriptor(),
                &layout,
                selection,
                resolved.schedule(),
                BasisMode::Lagrange,
            )
            .unwrap();
            let (base_plan, _) = akita_config::transcript_instance_descriptor::<F, Base>(
                setup.expanded.descriptor(),
                &layout,
                selection,
                resolved.schedule(),
                BasisMode::Lagrange,
            )
            .unwrap();
            assert_eq!(quotient_pairs(&selected_plan), 1);
            assert_eq!(quotient_pairs(&base_plan), 0);
            assert_ne!(selected_plan.digest().unwrap(), base_plan.digest().unwrap());

            // Grammar: the quotient group, `n_a (D - 1)` field atoms, is the
            // last message of a selected proof and absent from a base proof.
            let terminal = &resolved.schedule().terminal;
            let quotient_bytes = terminal.inner.matrix.output_rank()
                * (terminal.d_a() - 1)
                * <F as CanonicalBytes>::NUM_BYTES;
            let stripped = &selected_proof[..selected_proof.len() - quotient_bytes];
            assert!(verify_under!(Selected, &setup, &commitment, selection, stripped).is_err());
            assert!(verify_under!(Base, &setup, &commitment, selection, stripped).is_err());
            let mut padded = base_proof.clone();
            padded.resize(base_proof.len() + quotient_bytes, 0);
            assert!(verify_under!(Selected, &setup, &commitment, selection, &padded).is_err());
            assert!(verify_under!(Base, &setup, &commitment, selection, &padded).is_err());
        });
    }

    fn assert_refused(error: &AkitaError) {
        assert!(
            matches!(error, AkitaError::UnsupportedSchedule(message)
                if message.starts_with("terminal quotient certificate is not admitted")),
            "{error:?}"
        );
    }

    /// Selecting the certificate over a 32- or 64-bit production field is
    /// refused by the prover and by the verifier (before any proof byte); the
    /// same rows without the selection keep working (`akita_small_field_e2e`).
    macro_rules! dense_refusal {
        ($name:ident, $base:ty, $nv:expr) => {
            #[test]
            fn $name() {
                on_large_stack(|| {
                    type Cfg = TerminalQuotientConfig<$base>;
                    type SF = <Cfg as CommitmentConfig>::Field;
                    type E = <Cfg as CommitmentConfig>::ExtField;
                    const SNV: usize = $nv;
                    let scheme = AkitaCommitmentScheme::<Cfg>::new(
                        akita_config::test_support::workspace_schedule_catalog::<Cfg>().unwrap(),
                    );
                    let setup = scheme.setup_prover(SNV, 1).unwrap();
                    let backend = CpuBackend::new(setup.expanded.clone()).unwrap();
                    let evals: Vec<SF> = (0..(1u64 << SNV))
                        .map(|index| SF::from_u64(7 * index + 13))
                        .collect();
                    let poly = DensePoly::<SF>::from_field_evals(SNV, &evals).unwrap();
                    let output = backend
                        .commit(
                            scheme.schedules(),
                            &backend.import_source(vec![poly]).unwrap(),
                            GroupContext::scheduler_without_precommitted_groups(),
                        )
                        .unwrap();
                    let commitment = output.committed_group;
                    let point: Vec<E> = (0..SNV)
                        .map(|index| E::from_u64((index + 2) as u64))
                        .collect();
                    let weights = lagrange_weights(&point).unwrap();
                    let opening = evals
                        .iter()
                        .zip(weights)
                        .fold(E::zero(), |sum, (&value, weight)| {
                            sum + weight * E::lift_base(value)
                        });
                    let group = PolynomialGroupClaims::new(
                        point.clone(),
                        vec![opening],
                        commitment.clone(),
                    )
                    .unwrap();
                    let prover_data = SelectedProverOpeningData::from_committed_claims::<Cfg>(
                        OpeningClaims::from_groups(vec![group]).unwrap(),
                        vec![output.private_handle],
                        scheme.schedules(),
                    )
                    .unwrap();
                    let selection = prover_data.selection();
                    let error = scheme
                        .batched_prove(&setup, prover_data, &backend, LABEL, BasisMode::Lagrange)
                        .expect_err("the prover must refuse the certificate");
                    assert_refused(&error);

                    let group =
                        PolynomialGroupClaims::new(point, vec![opening], &commitment).unwrap();
                    let claims = OpeningClaims::from_groups(vec![group]).unwrap();
                    let statement = GroupBatchStatement::new(selection, claims).unwrap();
                    let error = scheme
                        .verifier(scheme.setup_verifier(&setup).unwrap())
                        .and_then(|verifier| {
                            verifier.batched_verify(&[], LABEL, statement, BasisMode::Lagrange)
                        })
                        .expect_err("the verifier must refuse the certificate");
                    assert_refused(&error);
                });
            }
        };
    }

    dense_refusal!(fp32_dense_selection_is_refused, fp32::Dense, 20);
    dense_refusal!(fp64_dense_selection_is_refused, fp64::Dense, 20);
}
