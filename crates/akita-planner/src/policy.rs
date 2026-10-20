//! Offline interpretation of catalog-bound planner policy data.

use akita_error::AkitaError;
use akita_params::MAX_I16_LOG_BASIS;
use akita_schedules::{DecompositionParams, PlannerPolicy};

/// Coefficient source whose A-matrix decomposition basis is being selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InnerBasisSource {
    /// Raw coefficients that may benefit from a full basis sweep.
    RawCoefficients { log_bound: u32 },
    /// Unit one-hot coefficients already represented by one exact digit.
    UnitOneHot,
    /// A recursive witness already represented as balanced digits.
    BalancedDigits { log_basis: u32 },
}

impl InnerBasisSource {
    /// Inclusive A/source basis domain for this coefficient shape.
    pub fn search_range(self, policy: &PlannerPolicy) -> Result<(u32, u32), AkitaError> {
        let (min, max) = policy.inner_basis_range;
        match self {
            Self::RawCoefficients { log_bound } => Ok((min, max.min(log_bound.max(min)))),
            Self::UnitOneHot => Ok((min, min)),
            Self::BalancedDigits { log_basis }
                if (1..=MAX_I16_LOG_BASIS).contains(&log_basis) =>
            {
                Ok((log_basis, log_basis))
            }
            Self::BalancedDigits { log_basis } => Err(AkitaError::InvalidSetup(format!(
                "recursive digit basis {log_basis} is outside the supported range [1, {MAX_I16_LOG_BASIS}]"
            ))),
        }
    }

    /// Exact source digit depth for a selected A basis.
    pub fn num_digits_inner(
        self,
        decomposition: DecompositionParams,
        selected_log_basis: u32,
    ) -> Result<usize, AkitaError> {
        match self {
            Self::RawCoefficients { log_bound } => Ok(akita_params::sis::num_digits_inner_for_bound(
                DecompositionParams {
                    log_basis: selected_log_basis,
                    ..decomposition
                },
                log_bound,
            )),
            Self::UnitOneHot => Ok(1),
            Self::BalancedDigits { log_basis } if log_basis == selected_log_basis => Ok(1),
            Self::BalancedDigits { log_basis } => Err(AkitaError::InvalidSetup(format!(
                "balanced source basis {log_basis} cannot be re-decomposed at basis {selected_log_basis}"
            ))),
        }
    }
}

pub(crate) fn log_basis_search_range_at_level(policy: &PlannerPolicy, level: usize) -> (u32, u32) {
    let (configured_min, max) = policy.opening_basis_range;
    if level == 0 {
        let root = policy.root_opening_log_basis.unwrap_or(configured_min);
        (root, root)
    } else {
        (configured_min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_override_is_validated_and_both_options_bind_identity() {
        let mut policy = akita_config::policy_of::<akita_config::proof_optimized::fp128::Dense>(
            akita_params::ChunkedWitnessCfg::default_non_chunked(),
        );
        assert!(!policy.monotone_opening_basis);
        let default_digest = akita_schedules::policy_digest(&policy);
        assert_eq!(log_basis_search_range_at_level(&policy, 0), (3, 3));
        assert_eq!(log_basis_search_range_at_level(&policy, 1), (3, 6));
        policy.root_opening_log_basis = Some(5);
        akita_schedules::validate_policy(&policy).unwrap();
        assert_eq!(log_basis_search_range_at_level(&policy, 0), (5, 5));
        assert_eq!(log_basis_search_range_at_level(&policy, 1), (3, 6));
        let pinned = akita_schedules::policy_digest(&policy);
        assert_ne!(default_digest, pinned);
        policy.monotone_opening_basis = true;
        let monotone = akita_schedules::policy_digest(&policy);
        assert_ne!(pinned, monotone);
        policy.root_opening_log_basis = None;
        assert_ne!(default_digest, akita_schedules::policy_digest(&policy));
        policy.monotone_opening_basis = false;
        assert_eq!(default_digest, akita_schedules::policy_digest(&policy));
        policy.root_opening_log_basis = Some(7);
        assert!(akita_schedules::validate_policy(&policy).is_err());
    }
}
