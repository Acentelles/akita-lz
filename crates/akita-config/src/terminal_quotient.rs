//! Terminal quotient certificate config adapter (feature `terminal-quotient`).
//!
//! `TerminalQuotientConfig<Cfg>` selects the terminal quotient certificate at
//! runtime, as `RecursiveCommitmentConfig` selects recursive setup offloading:
//! it delegates every parameter to `Cfg` and overrides only
//! [`CommitmentConfig::terminal_quotient`]. With it the prover sends the
//! quotient of every terminal A-row product by `X^D + 1`, the verifier checks
//! each row at two base-field points instead of computing the exact ring
//! product, and the proof stream uses its own protocol identifier, grinding-plan
//! entry and byte model. The adapter reuses `Cfg`'s schedule family; the
//! trusted rows are not re-planned for the quotient bytes. Only the 128-bit
//! base field is admitted: other fields refuse to prove and to verify.

use crate::CommitmentConfig;
use akita_challenges::SparseChallengeConfig;
use akita_error::AkitaError;
use akita_params::{ChunkedWitnessCfg, DecompositionParams, SisModulusProfileId};
use std::marker::PhantomData;

/// Config adapter that selects the terminal quotient certificate.
#[derive(Clone, Copy, Debug, Default)]
pub struct TerminalQuotientConfig<Cfg>(PhantomData<Cfg>);

impl<Cfg: CommitmentConfig> CommitmentConfig for TerminalQuotientConfig<Cfg> {
    type Field = Cfg::Field;
    type ExtField = Cfg::ExtField;

    const EXT_DEGREE: usize = Cfg::EXT_DEGREE;
    const RING_DIMENSION_SCHEDULE: akita_schedules::RingDimensionSchedule =
        Cfg::RING_DIMENSION_SCHEDULE;

    fn decomposition() -> DecompositionParams {
        Cfg::decomposition()
    }

    fn ring_challenge_config(d: usize) -> Result<SparseChallengeConfig, AkitaError> {
        Cfg::ring_challenge_config(d)
    }

    fn sis_modulus_profile() -> SisModulusProfileId {
        Cfg::sis_modulus_profile()
    }

    fn validate_sis_modulus_profile() -> Result<(), AkitaError> {
        Cfg::validate_sis_modulus_profile()
    }

    fn opening_basis_range() -> (u32, u32) {
        Cfg::opening_basis_range()
    }

    fn inner_basis_range() -> (u32, u32) {
        Cfg::inner_basis_range()
    }

    fn committed_source_class() -> akita_params::sis::CommittedSourceClass {
        Cfg::committed_source_class()
    }

    fn committed_source_contract() -> Result<akita_params::sis::CommittedSourceContract, AkitaError>
    {
        Cfg::committed_source_contract()
    }

    fn chunked_witness_cfg() -> ChunkedWitnessCfg {
        Cfg::chunked_witness_cfg()
    }

    fn recursive_setup_planning() -> bool {
        Cfg::recursive_setup_planning()
    }

    fn selection_policy() -> akita_schedules::SelectionPolicyId {
        Cfg::selection_policy()
    }

    fn schedule_family_name() -> &'static str {
        Cfg::schedule_family_name()
    }

    fn terminal_quotient() -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof_optimized::{fp128, fp32};
    use crate::{policy_of, RecursiveCommitmentConfig};

    fn assert_delegates<Cfg: CommitmentConfig>() {
        type Selected<C> = TerminalQuotientConfig<C>;
        assert!(Selected::<Cfg>::terminal_quotient());
        assert!(!Cfg::terminal_quotient());
        assert_eq!(
            Selected::<Cfg>::schedule_family_name(),
            Cfg::schedule_family_name()
        );
        assert_eq!(Selected::<Cfg>::EXT_DEGREE, Cfg::EXT_DEGREE);
        assert_eq!(
            Selected::<Cfg>::sis_modulus_profile(),
            Cfg::sis_modulus_profile()
        );
        assert_eq!(
            Selected::<Cfg>::recursive_setup_planning(),
            Cfg::recursive_setup_planning()
        );
        // The schedule policy, and so the trusted family it binds, is the base's.
        assert_eq!(policy_of::<Selected<Cfg>>(), policy_of::<Cfg>());
    }

    #[test]
    fn adapter_selects_only_the_terminal_quotient() {
        assert_delegates::<fp128::Dense>();
        assert_delegates::<fp128::OneHot>();
        assert_delegates::<fp32::Dense>();
        assert_delegates::<RecursiveCommitmentConfig<fp128::OneHot>>();
    }
}
