use super::{
    VerificationCandidate, VerificationContext, VerificationNeed, VerificationRejection,
    VerificationTarget,
};
use crate::candidate_generation::ask_witness_verification_step;
use crate::plan_repair::RepairPlanCandidate;
use worldwake_core::{RepairKind, VerificationProviderKind};

pub fn try_build(
    need: &VerificationNeed,
    ctx: &VerificationContext<'_>,
) -> Result<VerificationCandidate, VerificationRejection> {
    let subject = match need {
        VerificationNeed::StaleEntityBelief { subject, .. } => *subject,
        VerificationNeed::StaleInstitutionalClaim { .. }
        | VerificationNeed::OverdueExpectationAtPlace { .. } => {
            return Err(VerificationRejection::BreachClassMismatch);
        }
    };
    let ask_witness_def_id = ask_witness_action_def_id(ctx.action_defs)
        .ok_or(VerificationRejection::PayloadValidationFailed)?;

    let mut witnesses = ctx.belief_view.entities_at(ctx.effective_place);
    witnesses.sort_unstable();
    witnesses.dedup();
    let (witness, step) = witnesses
        .into_iter()
        .filter(|witness| *witness != ctx.actor)
        .filter(|witness| ctx.belief_view.effective_place(*witness) == Some(ctx.effective_place))
        .find_map(|witness| {
            ask_witness_verification_step(
                ctx.belief_view,
                ctx.actor,
                witness,
                subject,
                ask_witness_def_id,
            )
            .map(|step| (witness, step))
        })
        .ok_or(VerificationRejection::NoLawfulLocalTarget)?;

    Ok(VerificationCandidate {
        provider_kind: VerificationProviderKind::AskWitness,
        target: VerificationTarget::Witness(witness),
        repair_candidate: RepairPlanCandidate {
            kind: RepairKind::InsertVerification,
            provider: ctx.broken_link.provider,
            fact: ctx.broken_link.fact,
            step,
            reusable_suffix_index: None,
        },
        source_belief: None,
    })
}

fn ask_witness_action_def_id(
    action_defs: &worldwake_sim::ActionDefRegistry,
) -> Option<worldwake_core::ActionDefId> {
    action_defs
        .iter()
        .find(|def| def.name == "ask_witness")
        .map(|def| def.id)
}
