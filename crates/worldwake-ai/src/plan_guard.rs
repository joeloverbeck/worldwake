use serde::{Deserialize, Serialize};
use worldwake_core::{
    BeliefClaimKey, CausalLink, CommodityKind, EntityId, EventTag, ObservationPredicate, Permille,
    Quantity, StatePredicate, Tick,
};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PlanGuard {
    pub required_facts: Vec<RequiredFact>,
    pub min_confidence: Permille,
    pub invalidators: Vec<Invalidator>,
    #[serde(default)]
    pub causal_links: Vec<CausalLink>,
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
    use serde::Serialize;
    use worldwake_core::{
        BeliefClaimKey, CausalLink, CausalProvider, CommodityKind, EntityBeliefAspect, EntityId,
        EventTag, EvidenceKind, ExpectationKindTag, ObservationPredicate, Permille, PlanningFact,
        Quantity, StatePredicate, Tick,
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

    #[derive(Serialize)]
    struct LegacyPlanGuard {
        required_facts: Vec<RequiredFact>,
        min_confidence: Permille,
        invalidators: Vec<Invalidator>,
    }

    fn sample_causal_link() -> CausalLink {
        CausalLink {
            provider: CausalProvider::PriorStep { step_index: 1 },
            fact: PlanningFact::TargetPresent {
                target: entity(8),
                at_place: entity(9),
            },
            consumer_step_index: 2,
            source_tick: Tick(3),
            confidence: Permille::new(650).unwrap(),
        }
    }

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
            causal_links: vec![sample_causal_link()],
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

    #[test]
    fn plan_guard_causal_links_default_to_empty_via_serde() {
        let legacy_guard = LegacyPlanGuard {
            required_facts: vec![RequiredFact::RouteKnown {
                from: entity(1),
                to: entity(2),
            }],
            min_confidence: Permille::new(500).unwrap(),
            invalidators: vec![Invalidator::TargetMoved { target: entity(3) }],
        };
        let encoded = ron::to_string(&legacy_guard).unwrap();

        let guard: PlanGuard = ron::from_str(&encoded).unwrap();

        assert_eq!(guard.required_facts, legacy_guard.required_facts);
        assert_eq!(guard.min_confidence, legacy_guard.min_confidence);
        assert_eq!(guard.invalidators, legacy_guard.invalidators);
        assert!(guard.causal_links.is_empty());
    }

    #[test]
    fn plan_guard_causal_links_roundtrip_through_bincode() {
        let guard = PlanGuard {
            required_facts: vec![RequiredFact::ResourceAccess {
                resource: entity(10),
                agent_holds_permission: true,
            }],
            min_confidence: Permille::new(800).unwrap(),
            invalidators: vec![Invalidator::NewBlockerRecorded {
                baseline_tick: Tick(5),
            }],
            causal_links: vec![sample_causal_link()],
        };

        let bytes = bincode::serialize(&guard).unwrap();
        let restored: PlanGuard = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored, guard);
    }
}
