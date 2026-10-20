use super::*;
use jolt_poly::UnivariatePoly;
struct BackendStage1Session<
    'a,
    F: Field + CanonicalEncoding,
    E: Field,
    B: crate::backend::OpaqueStage1Kernel<F, E>,
> {
    backend: &'a B,
    handle: B::Stage1SessionHandle,
}
impl<F: Field + CanonicalEncoding, E: Field, B: crate::backend::OpaqueStage1Kernel<F, E>>
    crate::stage1::Stage1Session<E> for BackendStage1Session<'_, F, E, B>
{
    fn round_polynomial(
        &mut self,
        step: crate::backend::Stage1Step,
        round: usize,
        claim: E,
    ) -> Result<crate::backend::Stage1RoundPolynomial<E>, AkitaError> {
        self.backend
            .stage1_round_polynomial(&mut self.handle, step, round, claim)
    }
    fn bind_challenge(
        &mut self,
        step: crate::backend::Stage1Step,
        round: usize,
        challenge: E,
    ) -> Result<(), AkitaError> {
        self.backend
            .bind_stage1_challenge(&mut self.handle, step, round, challenge)
    }
    fn public_transition(
        &mut self,
        step: crate::backend::Stage1Step,
    ) -> Result<crate::backend::Stage1PublicTransition<E>, AkitaError> {
        self.backend
            .stage1_public_transition(&mut self.handle, step)
    }
    fn bind_batch_challenge(
        &mut self,
        transition: crate::backend::Stage1Transition,
        challenge: E,
    ) -> Result<(), AkitaError> {
        self.backend
            .bind_stage1_batch_challenge(&mut self.handle, transition, challenge)
    }
    fn finish(self) -> Result<crate::backend::Stage1FinalClaims<E>, AkitaError> {
        self.backend.finish_stage1(self.handle)
    }
}

pub(super) fn prove_stage1<F, E, B>(
    ctx: &crate::backend::OperationCtx<'_, F, B>,
    grinding: &mut akita_types::ProverGrinding<'_>,
    level: u32,
    rs: &mut RingSwitchOutput<E, B::RelationHandle>,
    lp: &CommittedGroupParams,
    plan: &RelationRangeImagePlan,
) -> Result<Stage1ProveOutput<E>, AkitaError>
where
    F: Field + CanonicalEncoding + akita_serialization::AkitaSerialize + Send + Sync + 'static,
    E: ExtField<F> + Unreduced + Fold + Ring + AkitaSerialize + Send + Sync + 'static,
    B: crate::backend::OpaqueStage1Kernel<F, E>,
{
    let _sumcheck_span = tracing::info_span!("stage1_sumcheck").entered();
    let domain = plan.digit_witness_domain();
    if plan.relation_address_geometry() != rs.relation_address_geometry
        || domain.live_len() != rs.witness_len
        || plan.digit_range_plan().basis() != rs.b
    {
        return Err(AkitaError::Internal(
            "ring-switch output disagrees with the relation/range-image plan".into(),
        ));
    }
    let digit_range_equality_col_bits = rs
        .tau0
        .len()
        .checked_sub(rs.digit_range_equality_low_variable_count)
        .ok_or_else(|| AkitaError::Internal("digit-range equality width overflow".into()))?;
    let equality_point = DigitRangeEqualityPoint::from_column_then_ring_challenges(
        &rs.tau0,
        digit_range_equality_col_bits,
        rs.digit_range_equality_low_variable_count,
    )?;
    let physical_plan = PhysicalResponsePlan::new(lp, plan)?;
    let equality_coordinates = equality_point.clone().into_coordinates();
    let stage1_plan = crate::backend::ValidatedStage1Plan::new(
        plan.digit_range_plan(),
        domain,
        equality_point,
        physical_plan.clone(),
        rs.witness_len,
    );
    let session_handle = crate::backend::OpaqueStage1Kernel::begin_stage1(
        ctx.backend(),
        &rs.relation_handle,
        &stage1_plan,
    )?;
    let crate::stage1::Stage1ProveOutput {
        point: stage1_point,
        range_image_evaluation,
        physical_l2: norm_proof,
    } = crate::stage1::prove_stage1::<F, E, _>(
        BackendStage1Session {
            backend: ctx.backend(),
            handle: session_handle,
        },
        plan.digit_range_plan(),
        equality_coordinates,
        physical_plan.as_ref(),
        grinding,
        level,
    )?;
    let physical_l2 = match physical_plan {
        Some(physical_plan) => {
            let (response_l2_sq, virtual_evaluations) = norm_proof.as_ref().ok_or_else(|| {
                AkitaError::Internal("stage 1 physical L2 replay has no norm proof".into())
            })?;
            let InnerCommitSecurityRoute::L2 {
                response_l2_sq_cap, ..
            } = lp.inner().matrix.security_route()
            else {
                return Err(AkitaError::Internal(
                    "physical L2 plan disagrees with the A security route".into(),
                ));
            };
            if *response_l2_sq > response_l2_sq_cap {
                return Err(AkitaError::InvalidInput(
                    "folded response exceeds the scheduled L2 cap".into(),
                ));
            }
            Some(PhysicalL2ProverReplay {
                plan: physical_plan,
                point: stage1_point.clone(),
                virtual_evaluations: virtual_evaluations.clone(),
                batching: Vec::new(),
                claim: E::zero(),
            })
        }
        None => {
            if norm_proof.is_some() {
                return Err(AkitaError::Internal(
                    "L-infinity route produced an L2 norm proof".into(),
                ));
            }
            None
        }
    };
    Ok(Stage1ProveOutput {
        point: stage1_point,
        range_image_evaluation,
        physical_l2,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn prove_stage2<F, E, B>(
    ctx: &crate::backend::OperationCtx<'_, F, B>,
    level: usize,
    grinding: &mut akita_types::ProverGrinding<'_>,
    batching_coeff: E,
    rs: RingSwitchOutput<E, B::RelationHandle>,
    stage1_point: &[E],
    range_image_evaluation: E,
    relation_claim: E,
    binary_batching: E,
    physical_l2: Option<PhysicalL2ProverReplay<E>>,
    linear_terms: Stage2OpeningDescription<'_, E>,
    trace_opening_claim: E,
    plan: &RelationRangeImagePlan,
    relation: crate::backend::RelationWeightRequest<'_, F, E>,
) -> Result<Stage2ProveOutput<E>, AkitaError>
where
    F: Field + CanonicalEncoding + akita_serialization::AkitaSerialize,
    E: ExtField<F> + Unreduced + Fold + Ring + AkitaSerialize,
    B: crate::backend::OpaqueStage2Kernel<F, E>,
{
    let _sumcheck_span = tracing::info_span!("stage2_sumcheck").entered();
    let domain = plan.digit_witness_domain();
    let geometry = rs.relation_address_geometry;
    let live_relation_lane_count = geometry.live_relation_lane_count();
    let relation_lane_variable_count = geometry.relation_lane_variable_count();
    let relation_coefficient_variable_count = geometry.relation_coefficient_variable_count();
    if plan.relation_address_geometry() != geometry
        || domain.live_len() != rs.witness_len
        || plan.digit_range_plan().basis() != rs.b
    {
        return Err(AkitaError::Internal(
            "ring-switch output disagrees with the relation/range-image plan".into(),
        ));
    }
    let domain_len = domain.domain_len();
    let physical_l2_claim = physical_l2.as_ref().map_or_else(E::zero, |norm| norm.claim);
    let relation_plan = crate::backend::ValidatedRelationSessionPlan::new(
        batching_coeff,
        stage1_point,
        range_image_evaluation,
        plan.digit_range_plan().basis(),
        relation,
        live_relation_lane_count,
        relation_lane_variable_count,
        relation_coefficient_variable_count,
        relation_claim,
        physical_l2_claim,
        linear_terms,
        trace_opening_claim,
        domain_len,
        domain.live_len(),
        physical_l2
            .as_ref()
            .map(|norm| crate::backend::PhysicalL2WeightRequest {
                plan: &norm.plan,
                point: &norm.point,
                batching: &norm.batching,
            }),
        binary_batching,
    );
    let mut session_handle = crate::backend::OpaqueStage2Kernel::begin_stage2(
        ctx.backend(),
        rs.relation_handle,
        relation_plan,
    )?;
    let level =
        u32::try_from(level).map_err(|_| AkitaError::Internal("fold level exceeds u32".into()))?;
    let claim =
        crate::backend::OpaqueStage2Kernel::stage2_input_claim(ctx.backend(), &session_handle)?;
    let num_rounds =
        crate::backend::OpaqueStage2Kernel::stage2_num_rounds(ctx.backend(), &session_handle)?;
    if num_rounds != domain.num_vars()
        || claim
            != batching_coeff * range_image_evaluation
                + relation_claim
                + trace_opening_claim
                + physical_l2_claim
    {
        return Err(AkitaError::Internal(
            "stage 2 backend round count or input claim differs from relation plan".into(),
        ));
    }
    let mut kernel = Stage2Sumcheck::<F, E, B> {
        backend: ctx.backend(),
        session: &mut session_handle,
        claim,
        rounds: num_rounds,
        field: std::marker::PhantomData,
    };
    let mut channel = akita_types::GrindingSumcheckProver::<F, E>::new(
        grinding,
        akita_params::SumcheckProtocol::Stage2,
        level,
        0,
    );
    let (sumcheck_challenges, claim) = akita_sumcheck::prove_sumcheck::<F, E, _, _>(
        &mut kernel,
        &mut channel,
        akita_sumcheck::SumcheckShape::new(num_rounds, 3)?,
        0,
    )?;
    let final_output =
        crate::backend::OpaqueStage2Kernel::finish_stage2(ctx.backend(), session_handle)?;
    if claim != final_output.final_claim() {
        return Err(AkitaError::Internal(
            "stage 2 backend final claim differs from sumcheck output".into(),
        ));
    }
    Ok(Stage2ProveOutput {
        challenges: sumcheck_challenges,
        witness_evaluation: final_output.witness_evaluation(),
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn prove_stage3<F, E, B>(
    backend: &B,
    session: &B::ProofSessionHandle,
    level: usize,
    setup_contribution_mode: SetupContributionMode,
    expanded: &akita_types::AkitaSetupDescriptor,
    prefix_slots: &SetupPrefixProverRegistry<F, B::CommitmentHandle>,
    lp: &CommittedGroupParams,
    next_level_params: &CommittedGroupParams,
    instance: &RingRelationInstance<F>,
    tau1: &[E],
    alpha: E,
    sumcheck_challenges: &[E],
    relation_address_geometry: akita_params::RelationAddressGeometry,
    grinding: &mut akita_types::ProverGrinding<'_>,
) -> Result<Option<Stage3ProveOutput<E>>, AkitaError>
where
    F: Field + CanonicalEncoding + akita_serialization::AkitaSerialize,
    E: FpExtEncoding<F>
        + Ring
        + ExtField<F>
        + AkitaSerialize
        + jolt_field::Unreduced
        + jolt_field::MulBaseUnreduced<F>,
    B: crate::backend::OpaqueStage3Kernel<F, E> + crate::backend::ProverHandleFamily<F, E>,
{
    match setup_contribution_mode {
        SetupContributionMode::Recursive => {
            let _stage3_span = tracing::info_span!(
                "stage3_sumcheck",
                level,
                stage2_rounds = sumcheck_challenges.len(),
                d_a = lp.d_a(),
            )
            .entered();
            let level = u32::try_from(level)
                .map_err(|_| AkitaError::Internal("fold level exceeds u32".into()))?;
            let prefix_id = next_level_params
                .setup_prefix()
                .and_then(|group| group.slot_id())
                .ok_or_else(|| AkitaError::Internal("Stage 3 requires a setup prefix".into()))?;
            let slot = prefix_slots.get(&prefix_id).ok_or_else(|| {
                AkitaError::InvalidSetup("planned setup prefix is missing".into())
            })?;
            let prefix_len = slot
                .public
                .id
                .natural_len
                .checked_next_power_of_two()
                .ok_or_else(|| AkitaError::InvalidSetup("setup prefix length overflow".into()))?;
            if slot.public.id != prefix_id || prefix_len > expanded.num_field_elements {
                return Err(AkitaError::InvalidSetup(
                    "invalid Stage 3 setup prefix".into(),
                ));
            }
            let (setup_product_claim, mut session) =
                backend.begin_stage3(crate::backend::Stage3Request {
                    session,
                    level,
                    prefix: &slot.public.id,
                    parameters: lp,
                    next_parameters: next_level_params,
                    relation: instance,
                    tau1,
                    alpha,
                    stage2_challenges: sumcheck_challenges,
                    address_geometry: relation_address_geometry,
                })?;
            let mut encoded_slot = Vec::new();
            slot.public
                .id
                .serialize_compressed(&mut encoded_slot)
                .map_err(|error| {
                    AkitaError::Internal(format!("setup-prefix slot serialization failed: {error}"))
                })?;
            akita_types::stage3_public_slot(grinding, level, &encoded_slot)?;
            akita_types::stage3_claim::<F, E, _>(grinding, level, setup_product_claim)?;
            let mut kernel = Stage3Sumcheck {
                backend,
                session: &mut session,
                claim: setup_product_claim,
                rounds: prefix_len.trailing_zeros() as usize,
                next_round: 0,
                field: std::marker::PhantomData,
            };
            let mut channel = akita_types::GrindingSumcheckProver::<F, E>::new(
                grinding,
                akita_params::SumcheckProtocol::Stage3,
                level,
                0,
            );
            let (setup_prefix_point, _) = akita_sumcheck::prove_sumcheck::<F, E, _, _>(
                &mut kernel,
                &mut channel,
                akita_sumcheck::SumcheckShape::new(
                    prefix_len.trailing_zeros() as usize,
                    akita_params::SETUP_SUMCHECK_DEGREE,
                )?,
                0,
            )?;
            let setup_prefix_eval = backend.finish_stage3(session)?;
            akita_types::stage3_prefix_eval::<F, E, _>(grinding, level, setup_prefix_eval)?;
            Ok(Some(Stage3ProveOutput {
                setup_prefix_eval,
                setup_prefix_point,
            }))
        }
        SetupContributionMode::Direct => Ok(None),
    }
}

struct Stage3Sumcheck<'a, F: Field, E: Field, B: crate::backend::OpaqueStage3Kernel<F, E>> {
    backend: &'a B,
    session: &'a mut B::Stage3SessionHandle,
    claim: E,
    rounds: usize,
    next_round: usize,
    field: std::marker::PhantomData<F>,
}

impl<F: Field, E: Field, B: crate::backend::OpaqueStage3Kernel<F, E>>
    akita_sumcheck::SumcheckKernel<E> for Stage3Sumcheck<'_, F, E, B>
{
    fn num_rounds(&self) -> usize {
        self.rounds
    }
    fn degree_bound(&self) -> usize {
        akita_params::SETUP_SUMCHECK_DEGREE
    }
    fn input_claim(&self) -> E {
        self.claim
    }
    fn round_polynomial(
        &mut self,
        round: usize,
        claim: E,
    ) -> Result<UnivariatePoly<E>, AkitaError> {
        self.backend
            .stage3_round_polynomial(self.session, round, claim)
    }
    fn bind_challenge(&mut self, round: usize, challenge: E) -> Result<(), AkitaError> {
        self.backend
            .bind_stage3_challenge(self.session, round, challenge)?;
        self.next_round += 1;
        Ok(())
    }
    fn finish(&mut self) -> Result<(), AkitaError> {
        if self.next_round != self.rounds {
            return Err(AkitaError::Internal(
                "stage 3 sumcheck finished before all rounds were bound".into(),
            ));
        }
        Ok(())
    }
}

struct Stage2Sumcheck<
    'a,
    F: Field + CanonicalEncoding,
    E: Field,
    B: crate::backend::OpaqueStage2Kernel<F, E>,
> {
    backend: &'a B,
    session: &'a mut B::Stage2SessionHandle,
    claim: E,
    rounds: usize,
    field: std::marker::PhantomData<F>,
}
impl<F: Field + CanonicalEncoding, E: Field, B: crate::backend::OpaqueStage2Kernel<F, E>>
    akita_sumcheck::SumcheckKernel<E> for Stage2Sumcheck<'_, F, E, B>
{
    fn num_rounds(&self) -> usize {
        self.rounds
    }
    fn degree_bound(&self) -> usize {
        3
    }
    fn input_claim(&self) -> E {
        self.claim
    }
    fn round_polynomial(
        &mut self,
        round: usize,
        claim: E,
    ) -> Result<UnivariatePoly<E>, AkitaError> {
        self.backend
            .stage2_round_polynomial(self.session, round, claim)
    }
    fn bind_challenge(&mut self, round: usize, challenge: E) -> Result<(), AkitaError> {
        self.backend
            .bind_stage2_challenge(self.session, round, challenge)
    }
    fn finish(&mut self) -> Result<(), AkitaError> {
        Ok(())
    }
}
