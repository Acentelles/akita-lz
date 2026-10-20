//! Canonical Stage-1 transcript driver shared by PCS and standalone range proofs.
use crate::backend::{
    Stage1FinalClaims, Stage1PublicTransition, Stage1RoundPolynomial, Stage1Step, Stage1Transition,
};
use akita_error::AkitaError;
use akita_params::DigitRangePlan;
use akita_serialization::AkitaSerialize;
use akita_types::{GrindingReplay, PhysicalResponsePlan};
use jolt_field::{CanonicalEncoding, ExtField, Field, Fold, Ring, Unreduced};
use jolt_poly::UnivariatePoly;

/// Interactive arithmetic boundary for the canonical Stage-1 driver.
pub trait Stage1Session<E: Field>: Sized {
    fn round_polynomial(
        &mut self,
        step: Stage1Step,
        round: usize,
        claim: E,
    ) -> Result<Stage1RoundPolynomial<E>, AkitaError>;
    fn bind_challenge(
        &mut self,
        step: Stage1Step,
        round: usize,
        challenge: E,
    ) -> Result<(), AkitaError>;
    fn public_transition(
        &mut self,
        step: Stage1Step,
    ) -> Result<Stage1PublicTransition<E>, AkitaError>;
    fn bind_batch_challenge(
        &mut self,
        transition: Stage1Transition,
        challenge: E,
    ) -> Result<(), AkitaError>;
    fn finish(self) -> Result<Stage1FinalClaims<E>, AkitaError>;
}

/// Public claims produced by Stage 1. Proof messages live in the Spongefish stream.
/// The caller must bind the range-image evaluation to its committed witness.
pub struct Stage1ProveOutput<E: Field> {
    /// Final sumcheck point in digit-domain coordinate order.
    pub point: Vec<E>,
    /// Evaluation of the multilinear extension of digit * (digit + 1).
    pub range_image_evaluation: E,
    /// Physical response norm and virtual evaluations, when requested.
    pub physical_l2: Option<(u128, Vec<E>)>,
}

/// Run the same Stage-1 schedule used by PCS proving on an arithmetic session.
/// The caller supplies the matching grinding plan and finishes the proof channel.
pub fn prove_stage1<F, E, S>(
    mut session: S,
    plan: DigitRangePlan,
    mut equality_coordinates: Vec<E>,
    physical_plan: Option<&PhysicalResponsePlan>,
    grinding: &mut akita_types::ProverGrinding<'_>,
    level: u32,
) -> Result<Stage1ProveOutput<E>, AkitaError>
where
    F: Field + CanonicalEncoding + AkitaSerialize,
    E: ExtField<F> + Unreduced + Fold + Ring + AkitaSerialize,
    S: Stage1Session<E>,
{
    let product_count = plan.product_stage_arities().len();
    let rounds = equality_coordinates.len();
    let mut claim = E::zero();
    for stage_index in 0..product_count {
        let shape = plan
            .stage_shape(rounds, stage_index)
            .ok_or_else(|| AkitaError::Internal("stage 1 product stage shape is missing".into()))?;
        let step = crate::backend::Stage1Step::Product(stage_index);
        let stage = u32::try_from(stage_index)
            .map_err(|_| AkitaError::Internal("stage 1 product stage index exceeds u32".into()))?;
        let mut kernel = Stage1EqSumcheck {
            session: &mut session,
            step,
            equality_point: &equality_coordinates,
            claim,
            degree: shape.sumcheck_proof.1,
            next_round: 0,
        };
        let mut channel = akita_types::GrindingSumcheckProver::<F, E>::new(
            grinding,
            akita_params::SumcheckProtocol::Stage1,
            level,
            stage,
        );
        let (challenges, _output_claim) = akita_sumcheck::prove_eq_factored_sumcheck::<F, E, _, _>(
            &mut kernel,
            &mut channel,
            akita_sumcheck::SumcheckShape::new(rounds, shape.sumcheck_proof.1)?,
            0,
        )?;
        let crate::backend::Stage1PublicTransition::ProductChildClaims(mut child_claims) =
            session.public_transition(step)?
        else {
            return Err(AkitaError::Internal(
                "stage 1 product transition did not return child claims".into(),
            ));
        };
        if child_claims.len() != shape.child_claims {
            return Err(AkitaError::Internal(
                "stage 1 product child claim count differs from stage shape".into(),
            ));
        }
        akita_types::stage1_child_claims::<F, E, _>(grinding, level, stage, &mut child_claims)?;
        let gamma = grinding.grinded_ext_challenge::<F, E>(
            akita_params::GrindingSite::Stage1InterstageBatch { level, stage },
        )?;
        let weights = plan.interstage_batch_weights(gamma, child_claims.len());
        claim = plan.batch_claims(&weights, &child_claims)?;
        session.bind_batch_challenge(
            crate::backend::Stage1Transition::ProductBatch(stage_index),
            gamma,
        )?;
        equality_coordinates = challenges;
    }

    let final_point;
    let (range_image_evaluation, norm_proof) = if physical_plan.is_some() {
        let step = crate::backend::Stage1Step::FusedRangeNorm;
        let crate::backend::Stage1PublicTransition::PhysicalL2Claims {
            response_l2_sq,
            mut subclaims,
        } = session.public_transition(step)?
        else {
            return Err(AkitaError::Internal(
                "stage 1 fused range-norm transition did not return physical L2 claims".into(),
            ));
        };
        let physical = physical_plan.as_ref().ok_or_else(|| {
            AkitaError::Internal("stage 1 fused range-norm execution has no physical plan".into())
        })?;
        if subclaims.len()
            != physical.shape().subclaim_count().ok_or_else(|| {
                AkitaError::Internal("stage 1 physical L2 subclaim count overflow".into())
            })?
        {
            return Err(AkitaError::Internal(
                "stage 1 physical L2 subclaim count differs from plan".into(),
            ));
        }
        akita_types::l2_prefix::<F, E, _>(grinding, level, response_l2_sq, &mut subclaims)?;
        let norm_claim = if subclaims.is_empty() {
            E::from_u128(response_l2_sq)
        } else {
            let gamma = grinding.grinded_ext_challenge::<F, E>(
                akita_params::GrindingSite::L2SubclaimBatch { level },
            )?;
            session
                .bind_batch_challenge(crate::backend::Stage1Transition::L2SubclaimBatch, gamma)?;
            let mut power = E::one();
            subclaims.iter().fold(E::zero(), |sum, &subclaim| {
                let next = sum + power * subclaim;
                power *= gamma;
                next
            })
        };
        let merge = grinding
            .grinded_ext_challenge::<F, E>(akita_params::GrindingSite::L2NormMerge { level })?;
        session.bind_batch_challenge(crate::backend::Stage1Transition::RangeNormMerge, merge)?;
        claim += merge * norm_claim;
        let mut kernel = Stage1StandardSumcheck {
            session: &mut session,
            step,
            claim,
            rounds,
            degree: plan.leaf_degree() + 1,
            next_round: 0,
        };
        let mut channel = akita_types::GrindingSumcheckProver::<F, E>::new(
            grinding,
            akita_params::SumcheckProtocol::PhysicalL2,
            level,
            0,
        );
        let (point, output_claim) = akita_sumcheck::prove_sumcheck::<F, E, _, _>(
            &mut kernel,
            &mut channel,
            akita_sumcheck::SumcheckShape::new(rounds, plan.leaf_degree() + 1)?,
            0,
        )?;
        claim = output_claim;
        final_point = point;
        let crate::backend::Stage1PublicTransition::Final {
            range_image_evaluation,
            mut virtual_evaluations,
        } = session.public_transition(step)?
        else {
            return Err(AkitaError::Internal(
                "stage 1 fused range-norm transition did not return final evaluations".into(),
            ));
        };
        if virtual_evaluations.len() != physical.shape().virtual_evaluation_count() {
            return Err(AkitaError::Internal(
                "stage 1 physical L2 virtual evaluation count differs from plan".into(),
            ));
        }
        akita_types::l2_virtual_evaluations::<F, E, _>(grinding, level, &mut virtual_evaluations)?;
        (
            range_image_evaluation,
            Some((response_l2_sq, virtual_evaluations)),
        )
    } else {
        let step = crate::backend::Stage1Step::RangeLeaf;
        let stage = u32::try_from(product_count)
            .map_err(|_| AkitaError::Internal("stage 1 leaf stage index exceeds u32".into()))?;
        let mut kernel = Stage1EqSumcheck {
            session: &mut session,
            step,
            equality_point: &equality_coordinates,
            claim,
            degree: plan.leaf_degree(),
            next_round: 0,
        };
        let mut channel = akita_types::GrindingSumcheckProver::<F, E>::new(
            grinding,
            akita_params::SumcheckProtocol::Stage1,
            level,
            stage,
        );
        let (point, output_claim) = akita_sumcheck::prove_eq_factored_sumcheck::<F, E, _, _>(
            &mut kernel,
            &mut channel,
            akita_sumcheck::SumcheckShape::new(rounds, plan.leaf_degree())?,
            0,
        )?;
        claim = output_claim;
        final_point = point;
        let crate::backend::Stage1PublicTransition::Final {
            range_image_evaluation,
            virtual_evaluations,
        } = session.public_transition(step)?
        else {
            return Err(AkitaError::Internal(
                "stage 1 range-leaf transition did not return final evaluations".into(),
            ));
        };
        if !virtual_evaluations.is_empty() {
            return Err(AkitaError::Internal(
                "stage 1 range-leaf transition returned physical L2 virtual evaluations".into(),
            ));
        }
        (range_image_evaluation, None)
    };
    let final_claims = session.finish()?;
    if final_claims.final_claim() != claim || final_claims.point() != final_point {
        return Err(AkitaError::Internal(
            "stage 1 backend final claim or point differs from sumcheck output".into(),
        ));
    }
    let stage1_point = final_claims.point().to_vec();
    akita_types::stage1_range_image::<F, E, _>(
        grinding,
        level,
        u32::try_from(product_count).map_err(|_| {
            AkitaError::Internal("stage 1 range-image stage index exceeds u32".into())
        })?,
        range_image_evaluation,
    )?;
    Ok(Stage1ProveOutput {
        point: stage1_point,
        range_image_evaluation,
        physical_l2: norm_proof,
    })
}

struct Stage1EqSumcheck<'a, E: Field, S: Stage1Session<E>> {
    session: &'a mut S,
    step: crate::backend::Stage1Step,
    equality_point: &'a [E],
    claim: E,
    degree: usize,
    next_round: usize,
}

impl<E: Field, S: Stage1Session<E>> akita_sumcheck::EqFactoredSumcheckKernel<E>
    for Stage1EqSumcheck<'_, E, S>
{
    fn num_rounds(&self) -> usize {
        self.equality_point.len()
    }
    fn degree_bound(&self) -> usize {
        self.degree
    }
    fn input_claim(&self) -> E {
        self.claim
    }
    fn current_tau(&self) -> E {
        self.equality_point[self.next_round]
    }
    fn round_polynomial(
        &mut self,
        round: usize,
        claim: E,
    ) -> Result<jolt_poly::OmittedConstantPoly<E>, AkitaError> {
        if round != self.next_round {
            return Err(AkitaError::Internal(
                "stage 1 equality-factored round index differs from adapter state".into(),
            ));
        }
        let polynomial = self.session.round_polynomial(self.step, round, claim)?;
        let crate::backend::Stage1RoundPolynomial::EqFactored(polynomial) = polynomial else {
            return Err(AkitaError::Internal(
                "stage 1 equality-factored backend returned a standard polynomial".into(),
            ));
        };
        if polynomial.degree() != self.degree {
            return Err(AkitaError::Internal(
                "stage 1 equality-factored polynomial degree differs from plan".into(),
            ));
        }
        Ok(polynomial)
    }
    fn bind_challenge(&mut self, round: usize, challenge: E) -> Result<(), AkitaError> {
        if round != self.next_round {
            return Err(AkitaError::Internal(
                "stage 1 equality-factored bind round differs from adapter state".into(),
            ));
        }
        self.session.bind_challenge(self.step, round, challenge)?;
        self.next_round += 1;
        Ok(())
    }
    fn finish(&mut self) -> Result<(), AkitaError> {
        if self.next_round != self.equality_point.len() {
            return Err(AkitaError::Internal(
                "stage 1 equality-factored sumcheck finished before all rounds were bound".into(),
            ));
        }
        Ok(())
    }
}

struct Stage1StandardSumcheck<'a, E: Field, S: Stage1Session<E>> {
    session: &'a mut S,
    step: crate::backend::Stage1Step,
    claim: E,
    rounds: usize,
    degree: usize,
    next_round: usize,
}

impl<E: Field, S: Stage1Session<E>> akita_sumcheck::SumcheckKernel<E>
    for Stage1StandardSumcheck<'_, E, S>
{
    fn num_rounds(&self) -> usize {
        self.rounds
    }
    fn degree_bound(&self) -> usize {
        self.degree
    }
    fn input_claim(&self) -> E {
        self.claim
    }
    fn round_polynomial(
        &mut self,
        round: usize,
        claim: E,
    ) -> Result<UnivariatePoly<E>, AkitaError> {
        if round != self.next_round {
            return Err(AkitaError::Internal(
                "stage 1 standard round index differs from adapter state".into(),
            ));
        }
        let polynomial = self.session.round_polynomial(self.step, round, claim)?;
        let crate::backend::Stage1RoundPolynomial::Standard(polynomial) = polynomial else {
            return Err(AkitaError::Internal(
                "stage 1 standard backend returned an equality-factored polynomial".into(),
            ));
        };
        if polynomial.coefficients().len() != self.degree + 1 {
            return Err(AkitaError::Internal(
                "stage 1 standard polynomial coefficient count differs from plan".into(),
            ));
        }
        Ok(polynomial)
    }
    fn bind_challenge(&mut self, round: usize, challenge: E) -> Result<(), AkitaError> {
        if round != self.next_round {
            return Err(AkitaError::Internal(
                "stage 1 standard bind round differs from adapter state".into(),
            ));
        }
        self.session.bind_challenge(self.step, round, challenge)?;
        self.next_round += 1;
        Ok(())
    }
    fn finish(&mut self) -> Result<(), AkitaError> {
        if self.next_round != self.rounds {
            return Err(AkitaError::Internal(
                "stage 1 standard sumcheck finished before all rounds were bound".into(),
            ));
        }
        Ok(())
    }
}
