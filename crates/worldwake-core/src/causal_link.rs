use crate::{
    BeliefClaimKey, CommodityKind, EntityBeliefAspect, EntityId, ExpectationId, Permille, Quantity,
    Tick,
};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
pub struct CausalLink {
    pub provider: CausalProvider,
    pub fact: PlanningFact,
    pub consumer_step_index: u16,
    pub source_tick: Tick,
    pub confidence: Permille,
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
pub enum CausalProvider {
    PriorStep {
        step_index: u16,
    },
    Belief {
        claim_key: BeliefClaimKey,
    },
    Observation {
        observed_entity: EntityId,
        aspect: EntityBeliefAspect,
    },
    Record {
        record_entity: EntityId,
        topic: RecordTopic,
    },
    CarriedItem {
        item_lot: EntityId,
    },
    Expectation {
        expectation_id: ExpectationId,
    },
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
pub enum PlanningFact {
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

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
pub enum RecordTopic {
    PriceObserved { commodity: CommodityKind },
    RouteSafety,
    OfficeRule { office: EntityId },
    BountyExists,
    TestifiedAbout { subject: EntityId },
}

#[cfg(test)]
mod tests {
    use super::{CausalLink, CausalProvider, PlanningFact, RecordTopic};
    use crate::{BeliefClaimKey, CommodityKind, EntityBeliefAspect, EntityId, Permille, Quantity};
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;
    use std::hash::Hash;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn assert_copy_value_bounds<
        T: Copy + Clone + Eq + Ord + Hash + Debug + Serialize + DeserializeOwned,
    >() {
    }

    fn assert_roundtrip<T>(value: T)
    where
        T: Copy + Eq + Debug + Serialize + DeserializeOwned,
    {
        let bytes = bincode::serialize(&value).unwrap();
        let roundtrip: T = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, value);
    }

    fn claim_key() -> BeliefClaimKey {
        BeliefClaimKey {
            subject: entity(10),
            aspect: EntityBeliefAspect::Inventory(CommodityKind::Bread),
        }
    }

    #[test]
    fn causal_link_types_satisfy_required_bounds() {
        assert_copy_value_bounds::<CausalLink>();
        assert_copy_value_bounds::<CausalProvider>();
        assert_copy_value_bounds::<PlanningFact>();
        assert_copy_value_bounds::<RecordTopic>();
    }

    #[test]
    fn causal_link_roundtrips_through_bincode() {
        assert_roundtrip(CausalLink {
            provider: CausalProvider::Belief {
                claim_key: claim_key(),
            },
            fact: PlanningFact::CommodityAvailable {
                place: entity(11),
                kind: CommodityKind::Grain,
                min_quantity: Quantity(4),
            },
            consumer_step_index: 3,
            source_tick: crate::Tick(42),
            confidence: Permille::new(750).unwrap(),
        });
    }

    #[test]
    fn causal_provider_variants_roundtrip_through_bincode() {
        assert_roundtrip(CausalProvider::PriorStep { step_index: 1 });
        assert_roundtrip(CausalProvider::Belief {
            claim_key: claim_key(),
        });
        assert_roundtrip(CausalProvider::Observation {
            observed_entity: entity(12),
            aspect: EntityBeliefAspect::ResourceAvailable(CommodityKind::Water),
        });
        assert_roundtrip(CausalProvider::Record {
            record_entity: entity(13),
            topic: RecordTopic::OfficeRule { office: entity(14) },
        });
        assert_roundtrip(CausalProvider::CarriedItem {
            item_lot: entity(15),
        });
        assert_roundtrip(CausalProvider::Expectation {
            expectation_id: crate::ExpectationId(16),
        });
    }

    #[test]
    fn planning_fact_variants_roundtrip_through_bincode() {
        assert_roundtrip(PlanningFact::TargetPresent {
            target: entity(20),
            at_place: entity(21),
        });
        assert_roundtrip(PlanningFact::CommodityAvailable {
            place: entity(22),
            kind: CommodityKind::Apple,
            min_quantity: Quantity(5),
        });
        assert_roundtrip(PlanningFact::RouteKnown {
            from: entity(23),
            to: entity(24),
        });
        assert_roundtrip(PlanningFact::ResourceAccess {
            resource: entity(25),
            agent_holds_permission: true,
        });
    }

    #[test]
    fn record_topic_variants_roundtrip_through_bincode() {
        assert_roundtrip(RecordTopic::PriceObserved {
            commodity: CommodityKind::Coin,
        });
        assert_roundtrip(RecordTopic::RouteSafety);
        assert_roundtrip(RecordTopic::OfficeRule { office: entity(30) });
        assert_roundtrip(RecordTopic::BountyExists);
        assert_roundtrip(RecordTopic::TestifiedAbout {
            subject: entity(31),
        });
    }
}
