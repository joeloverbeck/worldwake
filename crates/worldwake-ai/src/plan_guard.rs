use serde::{Deserialize, Serialize};
use worldwake_core::{
    BeliefClaimKey, CommodityKind, EntityId, EventTag, ObservationPredicate, Permille, Quantity,
    StatePredicate, Tick,
};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PlanGuard {
    pub required_facts: Vec<RequiredFact>,
    pub min_confidence: Permille,
    pub invalidators: Vec<Invalidator>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum RequiredFact {
    TargetPresent {
        target: EntityId,
        at_place: EntityId,
    },
    CommodityAvailable {
        place: EntityId,
        kind: CommodityKind,
        min_quantity: Quantity,
    },
    RouteKnown {
        from: EntityId,
        to: EntityId,
    },
    ResourceAccess {
        resource: EntityId,
        agent_holds_permission: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Invalidator {
    BeliefStatusChange {
        claim: BeliefClaimKey,
    },
    TargetMoved {
        target: EntityId,
    },
    CommodityDepleted {
        place: EntityId,
        kind: CommodityKind,
    },
    NewBlockerRecorded {
        baseline_tick: Tick,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PlanExpectation {
    pub kind: ExpectationKind,
    pub observe_by: Option<Tick>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ExpectationKind {
    Immediate { event_tag: EventTag },
    State { predicate: StatePredicate },
    Informed { observation: ObservationPredicate },
    Regression { predicate: StatePredicate },
}

#[cfg(test)]
mod tests {
    use super::{ExpectationKind, Invalidator, PlanExpectation, PlanGuard, RequiredFact};
    use worldwake_core::{
        BeliefClaimKey, CommodityKind, EntityBeliefAspect, EntityId, EventTag, EvidenceKind,
        ExpectationKindTag, ObservationPredicate, Permille, Quantity, StatePredicate, Tick,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn claim_key(slot: u32) -> BeliefClaimKey {
        BeliefClaimKey {
            subject: entity(slot),
            aspect: EntityBeliefAspect::Location,
        }
    }

    fn assert_value_bounds<T: Clone + std::fmt::Debug + Eq + PartialEq>() {}

    #[test]
    fn plan_guard_runtime_types_satisfy_required_bounds() {
        assert_value_bounds::<PlanGuard>();
        assert_value_bounds::<PlanExpectation>();
        assert_value_bounds::<RequiredFact>();
        assert_value_bounds::<Invalidator>();
        assert_value_bounds::<ExpectationKind>();
    }

    #[test]
    fn plan_guard_runtime_types_support_stated_derives() {
        let guard = PlanGuard {
            required_facts: vec![
                RequiredFact::TargetPresent {
                    target: entity(1),
                    at_place: entity(2),
                },
                RequiredFact::CommodityAvailable {
                    place: entity(3),
                    kind: CommodityKind::Bread,
                    min_quantity: Quantity(2),
                },
            ],
            min_confidence: Permille::new(700).unwrap(),
            invalidators: vec![
                Invalidator::BeliefStatusChange {
                    claim: claim_key(4),
                },
                Invalidator::NewBlockerRecorded {
                    baseline_tick: Tick(9),
                },
            ],
        };
        let expectation = PlanExpectation {
            kind: ExpectationKind::Informed {
                observation: ObservationPredicate::EvidencePerceived {
                    kind: EvidenceKind::MovementTrace {
                        entity: entity(5),
                        departed_from: entity(6),
                        direction: entity(7),
                        observed_at: Tick(8),
                    },
                    place: entity(6),
                },
            },
            observe_by: Some(Tick(12)),
        };

        assert_eq!(guard.clone(), guard);
        assert_eq!(expectation.clone(), expectation);
        assert_eq!(
            ExpectationKind::Immediate {
                event_tag: EventTag::ExpectationMismatch,
            },
            ExpectationKind::Immediate {
                event_tag: EventTag::ExpectationMismatch,
            }
        );
        assert_eq!(ExpectationKindTag::Immediate, ExpectationKindTag::Immediate);
        assert_eq!(
            ExpectationKind::State {
                predicate: StatePredicate::ActorHoldsCommodity {
                    kind: CommodityKind::Firewood,
                    min_quantity: Quantity(4),
                },
            },
            ExpectationKind::State {
                predicate: StatePredicate::ActorHoldsCommodity {
                    kind: CommodityKind::Firewood,
                    min_quantity: Quantity(4),
                },
            }
        );
    }
}
