use crate::planner_ops::PlannerOpKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use worldwake_core::{EntityId, GoalKey, GoalKind};

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum GoalKindTag {
    ConsumeOwnedCommodity,
    AcquireCommodity,
    Sleep,
    Relieve,
    Wash,
    ReduceDanger,
    Heal,
    ProduceCommodity,
    SellCommodity,
    RestockCommodity,
    MoveCargo,
    LootCorpse,
    BuryCorpse,
}

pub trait GoalKindPlannerExt {
    fn goal_kind_tag(&self) -> GoalKindTag;
    fn relevant_op_kinds(&self) -> &'static [PlannerOpKind];
}

const CONSUME_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Consume,
    PlannerOpKind::Travel,
    PlannerOpKind::Trade,
    PlannerOpKind::Harvest,
    PlannerOpKind::Craft,
    PlannerOpKind::MoveCargo,
];
const ACQUIRE_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Trade,
    PlannerOpKind::Harvest,
    PlannerOpKind::Craft,
    PlannerOpKind::MoveCargo,
];
const SLEEP_OPS: &[PlannerOpKind] = &[PlannerOpKind::Sleep, PlannerOpKind::Travel];
const RELIEVE_OPS: &[PlannerOpKind] = &[PlannerOpKind::Relieve, PlannerOpKind::Travel];
const WASH_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Wash,
    PlannerOpKind::Travel,
    PlannerOpKind::MoveCargo,
];
const REDUCE_DANGER_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Attack,
    PlannerOpKind::Defend,
    PlannerOpKind::Heal,
];
const HEAL_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Heal,
    PlannerOpKind::Trade,
    PlannerOpKind::Craft,
];
const PRODUCE_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Craft,
    PlannerOpKind::MoveCargo,
];
const SELL_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Trade,
    PlannerOpKind::MoveCargo,
];
const RESTOCK_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Trade,
    PlannerOpKind::Harvest,
    PlannerOpKind::Craft,
    PlannerOpKind::MoveCargo,
];
const MOVE_CARGO_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::MoveCargo];
const LOOT_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::Loot];
const NO_OPS: &[PlannerOpKind] = &[];

impl GoalKindPlannerExt for GoalKind {
    fn goal_kind_tag(&self) -> GoalKindTag {
        match self {
            GoalKind::ConsumeOwnedCommodity { .. } => GoalKindTag::ConsumeOwnedCommodity,
            GoalKind::AcquireCommodity { .. } => GoalKindTag::AcquireCommodity,
            GoalKind::Sleep => GoalKindTag::Sleep,
            GoalKind::Relieve => GoalKindTag::Relieve,
            GoalKind::Wash => GoalKindTag::Wash,
            GoalKind::ReduceDanger => GoalKindTag::ReduceDanger,
            GoalKind::Heal { .. } => GoalKindTag::Heal,
            GoalKind::ProduceCommodity { .. } => GoalKindTag::ProduceCommodity,
            GoalKind::SellCommodity { .. } => GoalKindTag::SellCommodity,
            GoalKind::RestockCommodity { .. } => GoalKindTag::RestockCommodity,
            GoalKind::MoveCargo { .. } => GoalKindTag::MoveCargo,
            GoalKind::LootCorpse { .. } => GoalKindTag::LootCorpse,
            GoalKind::BuryCorpse { .. } => GoalKindTag::BuryCorpse,
        }
    }

    fn relevant_op_kinds(&self) -> &'static [PlannerOpKind] {
        match self {
            GoalKind::ConsumeOwnedCommodity { .. } => CONSUME_OPS,
            GoalKind::AcquireCommodity { .. } => ACQUIRE_OPS,
            GoalKind::Sleep => SLEEP_OPS,
            GoalKind::Relieve => RELIEVE_OPS,
            GoalKind::Wash => WASH_OPS,
            GoalKind::ReduceDanger => REDUCE_DANGER_OPS,
            GoalKind::Heal { .. } => HEAL_OPS,
            GoalKind::ProduceCommodity { .. } => PRODUCE_OPS,
            GoalKind::SellCommodity { .. } => SELL_OPS,
            GoalKind::RestockCommodity { .. } => RESTOCK_OPS,
            GoalKind::MoveCargo { .. } => MOVE_CARGO_OPS,
            GoalKind::LootCorpse { .. } => LOOT_OPS,
            GoalKind::BuryCorpse { .. } => NO_OPS,
        }
    }
}

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
    pub evidence_entities: BTreeSet<EntityId>,
    pub evidence_places: BTreeSet<EntityId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RankedGoal {
    pub grounded: GroundedGoal,
    pub priority_class: GoalPriorityClass,
    pub motive_score: u32,
}

#[cfg(test)]
mod tests {
    use super::{GoalKindPlannerExt, GoalKindTag, GoalPriorityClass, GroundedGoal, RankedGoal};
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
        assert_value_bounds::<RankedGoal>();
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
            evidence_entities: BTreeSet::from([entity_id(3, 0), entity_id(3, 1)]),
            evidence_places: BTreeSet::from([entity_id(10, 0)]),
        };

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: GroundedGoal = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }

    #[test]
    fn ranked_goal_roundtrips_through_bincode() {
        let goal = RankedGoal {
            grounded: GroundedGoal {
                key: GoalKey::from(GoalKind::Heal {
                    target: entity_id(7, 1),
                }),
                evidence_entities: BTreeSet::from([entity_id(3, 0), entity_id(3, 1)]),
                evidence_places: BTreeSet::from([entity_id(10, 0)]),
            },
            priority_class: GoalPriorityClass::High,
            motive_score: 900,
        };

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: RankedGoal = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }

    #[test]
    fn goal_kind_tag_tracks_goal_families_without_payload_identity() {
        assert_eq!(
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: CommodityPurpose::Treatment,
            }
            .goal_kind_tag(),
            GoalKindTag::AcquireCommodity
        );
        assert_eq!(
            GoalKind::BuryCorpse {
                corpse: entity_id(1, 0),
                burial_site: entity_id(2, 0),
            }
            .goal_kind_tag(),
            GoalKindTag::BuryCorpse
        );
    }

    #[test]
    fn consume_goal_relevant_ops_include_consumption_and_access_paths() {
        let goal = GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Bread,
        };

        assert!(goal
            .relevant_op_kinds()
            .contains(&crate::PlannerOpKind::Consume));
        assert!(goal
            .relevant_op_kinds()
            .contains(&crate::PlannerOpKind::Travel));
        assert!(!goal
            .relevant_op_kinds()
            .contains(&crate::PlannerOpKind::Attack));
    }

    #[test]
    fn reduce_danger_goal_relevant_ops_include_defense_leaf_options() {
        let goal = GoalKind::ReduceDanger;

        assert!(goal
            .relevant_op_kinds()
            .contains(&crate::PlannerOpKind::Travel));
        assert!(goal
            .relevant_op_kinds()
            .contains(&crate::PlannerOpKind::Attack));
        assert!(goal
            .relevant_op_kinds()
            .contains(&crate::PlannerOpKind::Defend));
        assert!(goal
            .relevant_op_kinds()
            .contains(&crate::PlannerOpKind::Heal));
    }

    #[test]
    fn restock_goal_relevant_ops_include_trade_production_and_cargo() {
        let goal = GoalKind::RestockCommodity {
            commodity: CommodityKind::Bread,
        };

        assert!(goal
            .relevant_op_kinds()
            .contains(&crate::PlannerOpKind::Travel));
        assert!(goal
            .relevant_op_kinds()
            .contains(&crate::PlannerOpKind::Trade));
        assert!(goal
            .relevant_op_kinds()
            .contains(&crate::PlannerOpKind::Harvest));
        assert!(goal
            .relevant_op_kinds()
            .contains(&crate::PlannerOpKind::Craft));
        assert!(goal
            .relevant_op_kinds()
            .contains(&crate::PlannerOpKind::MoveCargo));
    }

    #[test]
    fn bury_goal_has_no_relevant_ops_until_action_family_exists() {
        let goal = GoalKind::BuryCorpse {
            corpse: entity_id(1, 0),
            burial_site: entity_id(2, 0),
        };

        assert!(goal.relevant_op_kinds().is_empty());
    }
}
