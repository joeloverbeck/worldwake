//! Authoritative blocked-intent memory stored on agents.

use crate::{ActionDefId, CommodityKind, Component, EntityId, GoalKey, Tick, UniqueItemKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockedIntentMemory {
    pub intents: Vec<BlockedIntent>,
}

impl BlockedIntentMemory {
    pub fn is_blocked(&self, key: &GoalKey, current_tick: Tick) -> bool {
        self.intents
            .iter()
            .any(|intent| {
                intent.goal_key == *key
                    && intent.expires_tick > current_tick
                    && intent.blocks_goal_generation()
            })
    }

    pub fn record(&mut self, intent: BlockedIntent) {
        self.intents
            .retain(|existing| existing.goal_key != intent.goal_key);
        self.intents.push(intent);
    }

    pub fn expire(&mut self, current_tick: Tick) {
        self.intents
            .retain(|intent| intent.expires_tick > current_tick);
    }

    pub fn clear_for(&mut self, key: &GoalKey) {
        self.intents.retain(|intent| intent.goal_key != *key);
    }
}

impl Component for BlockedIntentMemory {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockedIntent {
    pub goal_key: GoalKey,
    pub blocking_fact: BlockingFact,
    pub related_entity: Option<EntityId>,
    pub related_place: Option<EntityId>,
    pub related_action: Option<ActionDefId>,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
}

impl BlockedIntent {
    #[must_use]
    pub const fn blocks_goal_generation(&self) -> bool {
        !matches!(self.blocking_fact, BlockingFact::ExclusiveFacilityUnavailable)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BlockingFact {
    NoKnownPath,
    NoKnownSeller,
    SellerOutOfStock,
    TooExpensive,
    SourceDepleted,
    WorkstationBusy,
    ReservationConflict,
    ExclusiveFacilityUnavailable,
    MissingTool(UniqueItemKind),
    MissingInput(CommodityKind),
    TargetGone,
    DangerTooHigh,
    CombatTooRisky,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::{BlockedIntent, BlockedIntentMemory, BlockingFact};
    use crate::{
        test_utils::{entity_id, sample_blocked_intent, sample_goal_key},
        traits::Component,
        ActionDefId, CommodityKind, GoalKind, Tick, UniqueItemKind,
    };
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn blocked_intent_types_satisfy_required_bounds() {
        assert_component_bounds::<BlockedIntentMemory>();
        assert_value_bounds::<BlockedIntentMemory>();
        assert_value_bounds::<BlockedIntent>();
        assert_value_bounds::<BlockingFact>();
    }

    #[test]
    fn blocked_intent_memory_defaults_empty() {
        assert_eq!(BlockedIntentMemory::default().intents, Vec::new());
    }

    #[test]
    fn is_blocked_matches_only_live_entries_for_goal_key() {
        let key = sample_goal_key();
        let stale_key = crate::GoalKey::from(GoalKind::Sleep);
        let mut memory = BlockedIntentMemory {
            intents: vec![
                BlockedIntent {
                    expires_tick: Tick(10),
                    ..sample_blocked_intent()
                },
                BlockedIntent {
                    goal_key: stale_key,
                    blocking_fact: BlockingFact::DangerTooHigh,
                    related_entity: None,
                    related_place: None,
                    related_action: None,
                    observed_tick: Tick(8),
                    expires_tick: Tick(20),
                },
            ],
        };

        assert!(memory.is_blocked(&key, Tick(9)));
        assert!(!memory.is_blocked(&key, Tick(10)));
        assert!(!memory.is_blocked(&stale_key, Tick(20)));

        memory.expire(Tick(10));
        assert_eq!(memory.intents.len(), 1);
    }

    #[test]
    fn record_replaces_existing_entry_for_same_goal_key() {
        let key = sample_goal_key();
        let original = sample_blocked_intent();
        let replacement = BlockedIntent {
            goal_key: key,
            blocking_fact: BlockingFact::MissingTool(UniqueItemKind::SimpleTool),
            related_entity: Some(entity_id(7, 0)),
            related_place: Some(entity_id(3, 0)),
            related_action: Some(ActionDefId(44)),
            observed_tick: Tick(11),
            expires_tick: Tick(19),
        };
        let mut memory = BlockedIntentMemory {
            intents: vec![original],
        };

        memory.record(replacement);

        assert_eq!(memory.intents, vec![replacement]);
    }

    #[test]
    fn expire_removes_entries_at_or_before_current_tick() {
        let key = sample_goal_key();
        let other = crate::GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: crate::CommodityPurpose::Treatment,
        });
        let mut memory = BlockedIntentMemory {
            intents: vec![
                BlockedIntent {
                    goal_key: key,
                    expires_tick: Tick(14),
                    ..sample_blocked_intent()
                },
                BlockedIntent {
                    goal_key: other,
                    blocking_fact: BlockingFact::Unknown,
                    related_entity: None,
                    related_place: None,
                    related_action: None,
                    observed_tick: Tick(9),
                    expires_tick: Tick(15),
                },
            ],
        };

        memory.expire(Tick(14));

        assert_eq!(memory.intents.len(), 1);
        assert_eq!(memory.intents[0].goal_key, other);
    }

    #[test]
    fn clear_for_removes_all_matching_entries() {
        let key = sample_goal_key();
        let other = crate::GoalKey::from(GoalKind::ReduceDanger);
        let mut memory = BlockedIntentMemory {
            intents: vec![
                sample_blocked_intent(),
                BlockedIntent {
                    goal_key: key,
                    blocking_fact: BlockingFact::TargetGone,
                    related_entity: Some(entity_id(10, 1)),
                    related_place: None,
                    related_action: None,
                    observed_tick: Tick(8),
                    expires_tick: Tick(17),
                },
                BlockedIntent {
                    goal_key: other,
                    blocking_fact: BlockingFact::CombatTooRisky,
                    related_entity: None,
                    related_place: Some(entity_id(12, 0)),
                    related_action: None,
                    observed_tick: Tick(6),
                    expires_tick: Tick(30),
                },
            ],
        };

        memory.clear_for(&key);

        assert_eq!(memory.intents.len(), 1);
        assert_eq!(memory.intents[0].goal_key, other);
    }

    #[test]
    fn blocked_intent_memory_roundtrips_through_bincode() {
        let memory = BlockedIntentMemory {
            intents: vec![sample_blocked_intent()],
        };

        let bytes = bincode::serialize(&memory).unwrap();
        let roundtrip: BlockedIntentMemory = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, memory);
    }

    #[test]
    fn exclusive_facility_blockers_do_not_block_goal_generation() {
        let key = sample_goal_key();
        let memory = BlockedIntentMemory {
            intents: vec![BlockedIntent {
                goal_key: key,
                blocking_fact: BlockingFact::ExclusiveFacilityUnavailable,
                related_entity: Some(entity_id(4, 0)),
                related_place: Some(entity_id(2, 0)),
                related_action: Some(ActionDefId(9)),
                observed_tick: Tick(10),
                expires_tick: Tick(30),
            }],
        };

        assert!(!memory.is_blocked(&key, Tick(11)));
    }
}
