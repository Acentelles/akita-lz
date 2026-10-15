use super::*;

pub(crate) struct RecursiveCandidateViews {
    pub(crate) terminal: Vec<CommittedGroupParams>,
    pub(crate) folds: Vec<(CommittedGroupParams, usize)>,
}

/// Derive the terminal and fold views of one EvaluationTrace search together.
///
/// Both views use the same split candidates and materialized matrices, but
/// retain their distinct admission rules: terminal construction may use a
/// non-contracting successor, while an emitted fold must contract.
pub(crate) fn derive_recursive_candidate_views(
    request: RecursiveCandidateRequest<'_>,
    fold_policy: FoldCandidatePolicy,
    relation_domain: RelationSearchDomain,
) -> Result<RecursiveCandidateViews, AkitaError> {
    if request.opening.is_coefficient_packing() {
        return Err(AkitaError::InvalidSetup(
            "combined terminal/fold search requires EvaluationTrace".into(),
        ));
    }
    Ok(RecursiveCandidateViews {
        terminal: derive_terminal_candidates(request)?,
        folds: derive_fold_candidates(
            request,
            RecursiveFoldWork::direct(relation_domain),
            fold_policy,
        )?,
    })
}
