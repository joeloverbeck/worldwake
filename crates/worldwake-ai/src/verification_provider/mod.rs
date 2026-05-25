use crate::plan_repair::RepairPlanCandidate;
use serde::{Deserialize, Serialize};
use worldwake_core::{
    BeliefRef, CausalLink, EntityBeliefAspect, EntityId, ExpectationId, RecordTopic,
    VerificationProviderKind,
};
use worldwake_sim::{ActionDefRegistry, GoalBeliefView};

pub mod ask_witness_provider;
pub mod consult_record_provider;
pub mod search_place_provider;

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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum VerificationRejection {
    BreachClassMismatch,
    NoLawfulLocalTarget,
    PayloadValidationFailed,
    RecentlyFailedAtTarget,
}

pub struct VerificationContext<'a> {
    pub actor: EntityId,
    pub belief_view: &'a dyn GoalBeliefView,
    pub effective_place: EntityId,
    pub broken_link: CausalLink,
    pub action_defs: &'a ActionDefRegistry,
}

pub const PROVIDER_ITERATION_ORDER: [VerificationProviderKind; 3] = [
    VerificationProviderKind::AskWitness,
    VerificationProviderKind::ConsultRecord,
    VerificationProviderKind::SearchPlace,
];

pub fn try_build_verification_candidate(
    provider: VerificationProviderKind,
    need: &VerificationNeed,
    ctx: &VerificationContext<'_>,
) -> Result<VerificationCandidate, VerificationRejection> {
    match provider {
        VerificationProviderKind::AskWitness => ask_witness_provider::try_build(need, ctx),
        VerificationProviderKind::ConsultRecord => consult_record_provider::try_build(need, ctx),
        VerificationProviderKind::SearchPlace => search_place_provider::try_build(need, ctx),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldwake_core::{AgentBeliefStore, EntityId, PlanningFact, Quantity, Tick};

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn context<'a>(
        view: &'a dyn GoalBeliefView,
        action_defs: &'a worldwake_sim::ActionDefRegistry,
    ) -> VerificationContext<'a> {
        VerificationContext {
            actor: entity(1),
            belief_view: view,
            effective_place: entity(2),
            broken_link: CausalLink {
                provider: worldwake_core::CausalProvider::Record {
                    record_entity: entity(3),
                    topic: RecordTopic::RouteSafety,
                },
                fact: PlanningFact::CommodityAvailable {
                    place: entity(2),
                    kind: worldwake_core::CommodityKind::Bread,
                    min_quantity: Quantity(1),
                },
                consumer_step_index: 0,
                source_tick: Tick(0),
                confidence: worldwake_core::Permille::new_unchecked(1000),
            },
            action_defs,
        }
    }

    #[test]
    fn provider_iteration_order_is_deterministic() {
        assert_eq!(
            PROVIDER_ITERATION_ORDER,
            [
                VerificationProviderKind::AskWitness,
                VerificationProviderKind::ConsultRecord,
                VerificationProviderKind::SearchPlace,
            ]
        );
    }

    #[test]
    fn registry_routes_search_place_placeholder_as_breach_class_mismatch() {
        let world = worldwake_core::World::new(worldwake_core::Topology::new()).unwrap();
        let store = AgentBeliefStore::new();
        let view = worldwake_sim::PerAgentBeliefView::new(entity(1), &world, &store);
        let action_defs = worldwake_sim::ActionDefRegistry::new();
        let ctx = context(&view, &action_defs);
        let need = VerificationNeed::StaleInstitutionalClaim {
            record_topic: RecordTopic::RouteSafety,
        };

        assert_eq!(
            try_build_verification_candidate(VerificationProviderKind::SearchPlace, &need, &ctx),
            Err(VerificationRejection::BreachClassMismatch)
        );
    }
}
