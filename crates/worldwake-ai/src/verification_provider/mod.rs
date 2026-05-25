use crate::plan_repair::RepairPlanCandidate;
use worldwake_core::{
    BeliefRef, EntityBeliefAspect, EntityId, ExpectationId, RecordTopic, VerificationProviderKind,
};
use worldwake_sim::PerAgentBeliefView;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerificationNeed {
    StaleEntityBelief {
        subject: EntityId,
        aspect: EntityBeliefAspect,
    },
    StaleInstitutionalClaim {
        record_topic: RecordTopic,
    },
    OverdueExpectationAtPlace {
        expectation: ExpectationId,
        place: EntityId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationCandidate {
    pub provider_kind: VerificationProviderKind,
    pub target: VerificationTarget,
    pub repair_candidate: RepairPlanCandidate,
    pub source_belief: Option<BeliefRef>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum VerificationTarget {
    Witness(EntityId),
    Record(EntityId),
    Place(EntityId),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum VerificationRejection {
    BreachClassMismatch,
    NoLawfulLocalTarget,
    PayloadValidationFailed,
    RecentlyFailedAtTarget,
}

pub struct VerificationContext<'a> {
    pub actor: EntityId,
    pub belief_view: &'a PerAgentBeliefView<'a>,
    pub effective_place: EntityId,
}
