//! Concrete references to the state that gives a goal motive weight.

use serde::{Deserialize, Serialize};

use crate::{
    goal::OpportunityKey,
    ids::{EntityId, Tick},
    needs::HomeostaticNeedId,
    violation::ViolationId,
    wounds::WoundId,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MotiveSource {
    NeedPressure { need: HomeostaticNeedId },
    Pain { wound: WoundId },
    OfficeDuty { office: EntityId },
    Loyalty { other: EntityId },
    Greed { opportunity: OpportunityKey },
    Shame { reputation_record: EntityId },
    Revenge { violation: ViolationId },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MotiveSourceRef {
    pub source: MotiveSource,
    pub introduced_tick: Tick,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AcquisitionQuantity, CommodityKind, CommodityPurpose, GoalKey, GoalKind};

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn sample_opportunity() -> OpportunityKey {
        OpportunityKey {
            goal_key: GoalKey::new(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
            anchor: crate::OpportunityAnchor::Place(entity(10)),
        }
    }

    fn roundtrip<T>(value: &T) -> T
    where
        T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
    {
        let bytes = bincode::serialize(value).unwrap();
        let roundtrip = bincode::deserialize(&bytes).unwrap();
        assert_eq!(value, &roundtrip);
        assert_eq!(bytes, bincode::serialize(&roundtrip).unwrap());
        roundtrip
    }

    #[test]
    fn motive_source_roundtrips_through_bincode() {
        let variants = [
            MotiveSource::NeedPressure {
                need: HomeostaticNeedId::Hunger,
            },
            MotiveSource::Pain { wound: WoundId(1) },
            MotiveSource::OfficeDuty { office: entity(2) },
            MotiveSource::Loyalty { other: entity(3) },
            MotiveSource::Greed {
                opportunity: sample_opportunity(),
            },
            MotiveSource::Shame {
                reputation_record: entity(4),
            },
            MotiveSource::Revenge {
                violation: ViolationId(5),
            },
        ];

        for variant in variants {
            let decoded = roundtrip(&variant);
            match decoded {
                MotiveSource::NeedPressure { .. }
                | MotiveSource::Pain { .. }
                | MotiveSource::OfficeDuty { .. }
                | MotiveSource::Loyalty { .. }
                | MotiveSource::Greed { .. }
                | MotiveSource::Shame { .. }
                | MotiveSource::Revenge { .. } => {}
            }
        }
    }

    #[test]
    fn motive_source_ref_roundtrips_through_bincode() {
        let source_ref = MotiveSourceRef {
            source: MotiveSource::Revenge {
                violation: ViolationId(12),
            },
            introduced_tick: Tick(34),
        };

        roundtrip(&source_ref);
    }
}
