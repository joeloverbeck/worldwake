use crate::{BeliefClaimKey, CommodityKind, EntityId, EvidenceKind, Quantity};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ExpectationKindTag {
    Immediate,
    State,
    Informed,
    Regression,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum StatePredicate {
    CommodityAtPlaceAtLeast {
        place: EntityId,
        kind: CommodityKind,
        quantity: Quantity,
    },
    EntityAtPlace {
        entity: EntityId,
        place: EntityId,
    },
    ActorHoldsCommodity {
        kind: CommodityKind,
        min_quantity: Quantity,
    },
    ClaimEstablished {
        claim: BeliefClaimKey,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ObservationPredicate {
    EntityPerceivedAtPlace { entity: EntityId, place: EntityId },
    EvidencePerceived { kind: EvidenceKind, place: EntityId },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum InvalidatorTag {
    BeliefStatusChange,
    TargetMoved,
    CommodityDepleted,
    NewBlockerRecorded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MismatchDetail {
    GuardInvalidator(InvalidatorTag),
    StateUnmet { predicate: StatePredicate },
    ObservationMissing { predicate: ObservationPredicate },
}

#[cfg(test)]
mod tests {
    use super::{
        ExpectationKindTag, InvalidatorTag, MismatchDetail, ObservationPredicate, StatePredicate,
    };
    use crate::{
        BeliefClaimKey, CommodityKind, DisturbanceKind, EntityBeliefAspect, EntityId, EvidenceKind,
        Permille, Quantity, Tick,
    };
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;
    use std::hash::Hash;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn assert_copy_bounds<T: Copy + Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn assert_tag_bounds<
        T: Copy + Clone + Eq + Ord + Hash + Debug + Serialize + DeserializeOwned,
    >() {
    }

    fn assert_roundtrip<T>(value: &T)
    where
        T: Clone + Eq + Debug + Serialize + DeserializeOwned,
    {
        let bytes = bincode::serialize(value).unwrap();
        let roundtrip: T = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, *value);
    }

    #[test]
    fn plan_step_guard_core_types_satisfy_required_bounds() {
        assert_tag_bounds::<ExpectationKindTag>();
        assert_tag_bounds::<InvalidatorTag>();
        assert_copy_bounds::<StatePredicate>();
        assert_copy_bounds::<ObservationPredicate>();
        assert_copy_bounds::<MismatchDetail>();
    }

    #[test]
    fn plan_step_guard_core_types_roundtrip_through_bincode() {
        let claim = BeliefClaimKey {
            subject: entity(40),
            aspect: EntityBeliefAspect::Inventory(CommodityKind::Bread),
        };
        let evidence = EvidenceKind::BloodTrail {
            from_place: entity(50),
            severity: Permille::new(300).unwrap(),
            caused_by: Some(entity(51)),
        };

        assert_roundtrip(&ExpectationKindTag::Regression);
        assert_roundtrip(&InvalidatorTag::CommodityDepleted);
        assert_roundtrip(&StatePredicate::CommodityAtPlaceAtLeast {
            place: entity(10),
            kind: CommodityKind::Firewood,
            quantity: Quantity(3),
        });
        assert_roundtrip(&StatePredicate::EntityAtPlace {
            entity: entity(11),
            place: entity(12),
        });
        assert_roundtrip(&StatePredicate::ActorHoldsCommodity {
            kind: CommodityKind::Bread,
            min_quantity: Quantity(2),
        });
        assert_roundtrip(&StatePredicate::ClaimEstablished { claim });
        assert_roundtrip(&ObservationPredicate::EntityPerceivedAtPlace {
            entity: entity(20),
            place: entity(21),
        });
        assert_roundtrip(&ObservationPredicate::EvidencePerceived {
            kind: evidence,
            place: entity(22),
        });
        assert_roundtrip(&MismatchDetail::GuardInvalidator(
            InvalidatorTag::NewBlockerRecorded,
        ));
        assert_roundtrip(&MismatchDetail::StateUnmet {
            predicate: StatePredicate::ClaimEstablished { claim },
        });
        assert_roundtrip(&MismatchDetail::ObservationMissing {
            predicate: ObservationPredicate::EvidencePerceived {
                kind: EvidenceKind::DisturbanceMarker {
                    place: entity(23),
                    kind: DisturbanceKind::ForcedEntry,
                    created_at: Tick(24),
                },
                place: entity(23),
            },
        });
    }
}
