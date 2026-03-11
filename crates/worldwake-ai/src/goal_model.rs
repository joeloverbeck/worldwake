use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use worldwake_core::{EntityId, GoalKey};

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum GoalPriorityClass {
    Background,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GroundedGoal {
    pub key: GoalKey,
    pub priority_class: GoalPriorityClass,
    pub motive_score: u32,
    pub evidence_entities: BTreeSet<EntityId>,
    pub evidence_places: BTreeSet<EntityId>,
}

#[cfg(test)]
mod tests {
    use super::{GoalPriorityClass, GroundedGoal};
    use crate::{CommodityPurpose, GoalKey, GoalKind};
    use serde::{de::DeserializeOwned, Serialize};
    use std::{collections::BTreeSet, fmt::Debug};
    use worldwake_core::{test_utils::entity_id, CommodityKind};

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn goal_priority_class_satisfies_required_bounds() {
        assert_value_bounds::<GoalPriorityClass>();
        assert!(GoalPriorityClass::Critical > GoalPriorityClass::High);
        assert!(GoalPriorityClass::High > GoalPriorityClass::Medium);
        assert!(GoalPriorityClass::Medium > GoalPriorityClass::Low);
        assert!(GoalPriorityClass::Low > GoalPriorityClass::Background);
    }

    #[test]
    fn grounded_goal_satisfies_required_bounds() {
        assert_value_bounds::<GroundedGoal>();
    }

    #[test]
    fn crate_re_exports_the_canonical_shared_goal_identity() {
        let kind = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::Treatment,
        };
        let key = GoalKey::from(kind);

        assert_eq!(key.kind, kind);
        assert_eq!(key.commodity, Some(CommodityKind::Water));
    }

    #[test]
    fn grounded_goal_roundtrips_through_bincode() {
        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::Heal {
                target: entity_id(7, 1),
            }),
            priority_class: GoalPriorityClass::High,
            motive_score: 900,
            evidence_entities: BTreeSet::from([entity_id(3, 0), entity_id(3, 1)]),
            evidence_places: BTreeSet::from([entity_id(10, 0)]),
        };

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: GroundedGoal = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }
}
