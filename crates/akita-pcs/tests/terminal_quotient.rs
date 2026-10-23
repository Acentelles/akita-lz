//! Terminal quotient certificate: the proof-stream version boundary, and field
//! admission with the `terminal-quotient` feature.
//!
//! The cross-version test runs in both builds. It feeds one honest proof, with
//! its own setup and statement, to a verifier that differs only in the version
//! domain, and then to the same verifier in the other version's terminal
//! grammar (the quotient group removed or added), so each rejection is
//! attributed to exactly one boundary.

#![allow(missing_docs)]

use akita_config::{proof_optimized::fp128, CommitmentConfig};
use akita_cpu_backend::{CpuBackend, DensePoly, GroupContext};
use akita_params::{lagrange_weights, BasisMode};
use akita_pcs::AkitaCommitmentScheme;
use akita_prover::SelectedProverOpeningData;
use akita_transcript::{with_proof_stream_version, ProofStreamVersion, TERMINAL_QUOTIENT};
use akita_types::{GroupBatchStatement, OpeningClaims, PolynomialGroupClaims};
use jolt_field::{CanonicalBytes, Ring, Zero};

fn on_large_stack(body: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(body)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn proof_stream_versions_reject_each_other_at_the_version_boundary() {
    on_large_stack(|| {
        type Cfg = fp128::Dense;
        type F = <Cfg as CommitmentConfig>::Field;
        const NV: usize = 14;
        const LABEL: &[u8] = b"terminal-quotient/cross-version";
        let schedules = akita_config::test_support::workspace_schedule_catalog::<Cfg>().unwrap();
        let scheme = AkitaCommitmentScheme::<Cfg>::new(schedules);
        let setup = scheme.setup_prover(NV, 1).unwrap();
        let backend = CpuBackend::new(setup.expanded.clone()).unwrap();
        let evals: Vec<F> = (0..(1u64 << NV))
            .map(|index| F::from_u64(7 * index + 13))
            .collect();
        let poly = DensePoly::<F>::from_field_evals(NV, &evals).unwrap();
        let output = backend
            .commit(
                scheme.schedules(),
                &backend.import_source(vec![poly]).unwrap(),
                GroupContext::scheduler_without_precommitted_groups(),
            )
            .unwrap();
        let commitment = output.committed_group;
        let point: Vec<F> = (0..NV)
            .map(|index| F::from_u64((index + 2) as u64))
            .collect();
        let weights = lagrange_weights(&point).unwrap();
        let opening = evals
            .iter()
            .zip(weights)
            .fold(F::zero(), |sum, (&value, weight)| sum + value * weight);
        let group =
            PolynomialGroupClaims::new(point.clone(), vec![opening], commitment.clone()).unwrap();
        let prover_data = SelectedProverOpeningData::from_committed_claims::<Cfg>(
            OpeningClaims::from_groups(vec![group]).unwrap(),
            vec![output.private_handle],
            scheme.schedules(),
        )
        .unwrap();
        let selection = prover_data.selection();
        let proof = scheme
            .batched_prove(&setup, prover_data, &backend, LABEL, BasisMode::Lagrange)
            .unwrap();
        let verifier = scheme
            .verifier(scheme.setup_verifier(&setup).unwrap())
            .unwrap();
        let verify = |bytes: &[u8], version: ProofStreamVersion| {
            let group =
                PolynomialGroupClaims::new(point.clone(), vec![opening], &commitment).unwrap();
            let claims = OpeningClaims::from_groups(vec![group]).unwrap();
            let statement = GroupBatchStatement::new(selection, claims).unwrap();
            with_proof_stream_version(version, || {
                verifier.batched_verify(bytes, LABEL, statement, BasisMode::Lagrange)
            })
        };
        let other = match ProofStreamVersion::ACTIVE {
            ProofStreamVersion::V7 => ProofStreamVersion::V7TerminalQuotient,
            ProofStreamVersion::V7TerminalQuotient => ProofStreamVersion::V7,
        };

        // Control: the proof verifies under its own version.
        verify(&proof, ProofStreamVersion::ACTIVE).expect("honest proof under its own version");

        // Version domain: the same bytes, setup, statement, grinding plan and
        // verifier code, with only the protocol identifier changed.
        assert!(verify(&proof, other).is_err());

        // Grammar: the quotient group, `n_a (D - 1)` field atoms, is the last
        // message of a certificate proof and absent from an exact-relation
        // proof. Removing it (certificate build) or appending that many atoms
        // (exact build) gives the other version's terminal shape.
        let resolved = scheme.schedules().resolve_selection(selection).unwrap();
        let terminal = &resolved.schedule().terminal;
        let quotient_bytes = terminal.inner.matrix.output_rank()
            * (terminal.d_a() - 1)
            * <F as CanonicalBytes>::NUM_BYTES;
        let other_grammar = if TERMINAL_QUOTIENT {
            proof[..proof.len() - quotient_bytes].to_vec()
        } else {
            let mut longer = proof.clone();
            longer.resize(proof.len() + quotient_bytes, 0);
            longer
        };
        assert!(verify(&other_grammar, ProofStreamVersion::ACTIVE).is_err());
        assert!(verify(&other_grammar, other).is_err());
    });
}

/// With the certificate, the production 32- and 64-bit configurations refuse
/// to prove and to verify: the grinding plan admits the quotient's base-field
/// point pair only when `(2D - 2)^2 2^128 <= |F|^2`.
#[cfg(feature = "terminal-quotient")]
mod small_field_refusal {
    use super::*;
    use akita_config::proof_optimized::{fp32, fp64};
    use akita_error::AkitaError;
    use jolt_field::ExtField;

    fn assert_refused(error: &AkitaError) {
        assert!(
            matches!(error, AkitaError::UnsupportedSchedule(message)
                if message.starts_with("terminal quotient certificate is not admitted")),
            "{error:?}"
        );
    }

    macro_rules! dense_refusal {
        ($name:ident, $cfg:ty, $nv:expr) => {
            #[test]
            fn $name() {
                on_large_stack(|| {
                    type Cfg = $cfg;
                    type F = <Cfg as CommitmentConfig>::Field;
                    type E = <Cfg as CommitmentConfig>::ExtField;
                    const NV: usize = $nv;
                    const LABEL: &[u8] = b"terminal-quotient/small-field";
                    let schedules =
                        akita_config::test_support::workspace_schedule_catalog::<Cfg>().unwrap();
                    let scheme = AkitaCommitmentScheme::<Cfg>::new(schedules);
                    let setup = scheme.setup_prover(NV, 1).unwrap();
                    let backend = CpuBackend::new(setup.expanded.clone()).unwrap();
                    let evals: Vec<F> = (0..(1u64 << NV))
                        .map(|index| F::from_u64(7 * index + 13))
                        .collect();
                    let poly = DensePoly::<F>::from_field_evals(NV, &evals).unwrap();
                    let output = backend
                        .commit(
                            scheme.schedules(),
                            &backend.import_source(vec![poly]).unwrap(),
                            GroupContext::scheduler_without_precommitted_groups(),
                        )
                        .unwrap();
                    let commitment = output.committed_group;
                    let point: Vec<E> = (0..NV)
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

    dense_refusal!(fp32_dense_refuses_to_prove_and_verify, fp32::Dense, 20);
    dense_refusal!(fp64_dense_refuses_to_prove_and_verify, fp64::Dense, 20);
}
