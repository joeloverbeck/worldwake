//! Authoritative blocked-intent memory stored on agents.

use crate::{
    ActionDefId, CommodityKind, Component, EntityId, GoalKey, Permille, Quantity, Tick,
    UniqueItemKind,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct BlockerKey {
    pub goal_key: GoalKey,
    pub place: Option<EntityId>,
    pub target: Option<EntityId>,
    pub action_def: Option<ActionDefId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockerDiagnostic {
    pub action_def: ActionDefId,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockedIntentMemory {
    pub intents: BTreeMap<BlockerKey, BlockedIntent>,
}

impl BlockedIntentMemory {
    pub fn is_blocked(
        &self,
        goal_key: &GoalKey,
        place: Option<EntityId>,
        target: Option<EntityId>,
        action_def: Option<ActionDefId>,
        current_tick: Tick,
    ) -> bool {
        let global_query = place.is_none() && target.is_none() && action_def.is_none();
        self.intents.values().any(|intent| {
            intent.blocker_key.goal_key == *goal_key
                && intent.expires_tick > current_tick
                && intent.blocks_goal_generation()
                && (global_query || matches_scope(&intent.blocker_key, place, target, action_def))
        })
    }

    pub fn is_blocked_for_search(
        &self,
        goal_key: &GoalKey,
        place: Option<EntityId>,
        target: Option<EntityId>,
        action_def: Option<ActionDefId>,
        current_tick: Tick,
    ) -> bool {
        self.find_blocked_for_search(goal_key, place, target, action_def, current_tick)
            .is_some()
    }

    /// Like `is_blocked_for_search` but returns the matching `BlockedIntent`
    /// reference so callers can inspect the `blocking_fact` for trace recording.
    pub fn find_blocked_for_search(
        &self,
        goal_key: &GoalKey,
        place: Option<EntityId>,
        target: Option<EntityId>,
        action_def: Option<ActionDefId>,
        current_tick: Tick,
    ) -> Option<&BlockedIntent> {
        self.intents.values().find(|intent| {
            intent.blocker_key.goal_key == *goal_key
                && intent.expires_tick > current_tick
                && matches_scope(&intent.blocker_key, place, target, action_def)
        })
    }

    pub fn record(&mut self, intent: BlockedIntent) {
        self.intents.insert(intent.blocker_key, intent);
    }

    pub fn expire(&mut self, current_tick: Tick) {
        self.intents
            .retain(|_, intent| intent.expires_tick > current_tick);
    }

    pub fn sweep_cleared(&mut self, mut is_cleared: impl FnMut(&BlockedIntent) -> bool) {
        self.intents.retain(|_, intent| !is_cleared(intent));
    }

    pub fn clear_for(&mut self, key: &BlockerKey) {
        self.intents.remove(key);
    }

    pub fn clear_all_for_goal(&mut self, goal_key: &GoalKey) {
        self.intents.retain(|k, _| k.goal_key != *goal_key);
    }
}

fn matches_scope(
    blocker: &BlockerKey,
    query_place: Option<EntityId>,
    query_target: Option<EntityId>,
    query_action: Option<ActionDefId>,
) -> bool {
    // Goal-scoped blocker (place=None, target=None, action=None) matches everything
    if blocker.place.is_none() && blocker.target.is_none() && blocker.action_def.is_none() {
        return true;
    }
    // Place must match if blocker has one
    if let Some(blocker_place) = blocker.place
        && query_place != Some(blocker_place)
    {
        return false;
    }
    // Target must match if blocker has one
    if let Some(blocker_target) = blocker.target
        && query_target != Some(blocker_target)
    {
        return false;
    }
    // Action must match if blocker has one
    if let Some(blocker_action) = blocker.action_def
        && query_action != Some(blocker_action)
    {
        return false;
    }
    true
}

impl Component for BlockedIntentMemory {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BlockerClearingCondition {
    CommodityAvailabilityChanged {
        commodity: CommodityKind,
        place: EntityId,
    },
    InventoryChanged {
        commodity: CommodityKind,
    },
    UniqueItemAcquired {
        kind: UniqueItemKind,
    },
    PathDiscovered {
        destination: EntityId,
    },
    EntityReappeared {
        entity: EntityId,
    },
    DangerReduced {
        place: EntityId,
    },
    ContentionChanged {
        facility: EntityId,
    },
    TtlOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ClearingBaseline {
    CommodityQuantity { quantity: Quantity },
    InventoryQuantity { quantity: Quantity },
    UniqueItemCount(u32),
    PathKnown(bool),
    EntityBelieved(bool),
    DangerLevel(Permille),
    ContentionPosition(Option<u32>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockedIntent {
    pub blocker_key: BlockerKey,
    pub blocking_fact: BlockingFact,
    pub diagnostic_context: Option<BlockerDiagnostic>,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
    pub clearing_condition: BlockerClearingCondition,
    pub baseline_snapshot: Option<ClearingBaseline>,
}

impl BlockedIntent {
    #[must_use]
    pub const fn blocks_goal_generation(&self) -> bool {
        !matches!(
            self.blocking_fact,
            BlockingFact::ExclusiveFacilityUnavailable | BlockingFact::SourceDepleted
        )
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
    /// Frame patience exhausted for this goal at this place/target.
    PatienceExhausted,
    /// A critical frame assumption failed (target dead, route severed).
    AssumptionFailed,
    /// Seller staffed a market but no trade occurred during the presence cycle.
    NoBuyer,
}

#[cfg(test)]
mod tests {
    use super::{
        BlockedIntent, BlockedIntentMemory, BlockerClearingCondition, BlockerDiagnostic,
        BlockerKey, BlockingFact, ClearingBaseline,
    };
    use crate::{
        ActionDefId, CommodityKind, GoalKind, Quantity, Tick, UniqueItemKind,
        test_utils::{entity_id, sample_blocked_intent, sample_blocker_key, sample_goal_key},
        traits::Component,
    };
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn assert_copy_value_bounds<T: Copy + Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn make_intent(key: BlockerKey, fact: BlockingFact, expires: Tick) -> BlockedIntent {
        BlockedIntent {
            blocker_key: key,
            blocking_fact: fact,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: expires,
            clearing_condition: BlockerClearingCondition::TtlOnly,
            baseline_snapshot: None,
        }
    }

    #[test]
    fn blocked_intent_types_satisfy_required_bounds() {
        assert_component_bounds::<BlockedIntentMemory>();
        assert_value_bounds::<BlockedIntentMemory>();
        assert_copy_value_bounds::<BlockedIntent>();
        assert_copy_value_bounds::<BlockerClearingCondition>();
        assert_copy_value_bounds::<ClearingBaseline>();
        assert_value_bounds::<BlockingFact>();
        assert_value_bounds::<BlockerKey>();
        assert_value_bounds::<BlockerDiagnostic>();
    }

    #[test]
    fn blocker_clearing_condition_and_baseline_satisfy_required_bounds() {
        assert_copy_value_bounds::<BlockerClearingCondition>();
        assert_copy_value_bounds::<ClearingBaseline>();
    }

    #[test]
    fn blocked_intent_memory_defaults_empty() {
        let memory = BlockedIntentMemory::default();
        assert!(memory.intents.is_empty());
    }

    #[test]
    fn is_blocked_matches_only_live_entries_for_goal_key() {
        let key = sample_goal_key();
        let stale_key = crate::GoalKey::from(GoalKind::Sleep);
        let mut memory = BlockedIntentMemory::default();

        // Global blocker for key — blocks goal generation
        let blocker1 = make_intent(
            BlockerKey {
                goal_key: key,
                place: None,
                target: None,
                action_def: None,
            },
            BlockingFact::NoKnownPath,
            Tick(10),
        );
        // Global blocker for stale_key
        let blocker2 = make_intent(
            BlockerKey {
                goal_key: stale_key,
                place: None,
                target: None,
                action_def: None,
            },
            BlockingFact::DangerTooHigh,
            Tick(20),
        );
        memory.record(blocker1);
        memory.record(blocker2);

        // Live at tick 9
        assert!(memory.is_blocked(&key, None, None, None, Tick(9)));
        // Expired at tick 10
        assert!(!memory.is_blocked(&key, None, None, None, Tick(10)));
        // Expired at tick 20
        assert!(!memory.is_blocked(&stale_key, None, None, None, Tick(20)));

        memory.expire(Tick(10));
        assert_eq!(memory.intents.len(), 1);
    }

    #[test]
    fn source_depleted_does_not_block_goal_generation() {
        let key = sample_goal_key();
        let mut memory = BlockedIntentMemory::default();
        memory.record(make_intent(
            BlockerKey {
                goal_key: key,
                place: Some(entity_id(2, 0)),
                target: Some(entity_id(4, 0)),
                action_def: None,
            },
            BlockingFact::SourceDepleted,
            Tick(20),
        ));

        // SourceDepleted does not block goal generation (blocks_goal_generation returns false)
        assert!(!memory.is_blocked(
            &key,
            Some(entity_id(2, 0)),
            Some(entity_id(4, 0)),
            None,
            Tick(9)
        ));
    }

    #[test]
    fn record_replaces_existing_entry_for_same_compound_key() {
        let bk = sample_blocker_key();
        let original = sample_blocked_intent();
        let replacement = BlockedIntent {
            blocker_key: bk,
            blocking_fact: BlockingFact::MissingTool(UniqueItemKind::SimpleTool),
            diagnostic_context: None,
            observed_tick: Tick(11),
            expires_tick: Tick(19),
            clearing_condition: BlockerClearingCondition::TtlOnly,
            baseline_snapshot: None,
        };
        let mut memory = BlockedIntentMemory::default();
        memory.record(original);
        memory.record(replacement);

        assert_eq!(memory.intents.len(), 1);
        assert_eq!(memory.intents[&bk], replacement);
    }

    #[test]
    fn record_preserves_different_place_for_same_goal() {
        let key = sample_goal_key();
        let place_a = entity_id(10, 0);
        let place_b = entity_id(11, 0);
        let bk_a = BlockerKey {
            goal_key: key,
            place: Some(place_a),
            target: None,
            action_def: None,
        };
        let bk_b = BlockerKey {
            goal_key: key,
            place: Some(place_b),
            target: None,
            action_def: None,
        };
        let mut memory = BlockedIntentMemory::default();
        memory.record(make_intent(bk_a, BlockingFact::SourceDepleted, Tick(20)));
        memory.record(make_intent(bk_b, BlockingFact::SourceDepleted, Tick(25)));

        assert_eq!(memory.intents.len(), 2);
    }

    #[test]
    fn expire_removes_entries_at_or_before_current_tick() {
        let key = sample_goal_key();
        let other = crate::GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: crate::CommodityPurpose::SelfConsume,
        });
        let bk1 = BlockerKey {
            goal_key: key,
            place: None,
            target: None,
            action_def: None,
        };
        let bk2 = BlockerKey {
            goal_key: other,
            place: None,
            target: None,
            action_def: None,
        };
        let mut memory = BlockedIntentMemory::default();
        memory.record(make_intent(bk1, BlockingFact::NoKnownPath, Tick(14)));
        memory.record(make_intent(bk2, BlockingFact::Unknown, Tick(15)));

        memory.expire(Tick(14));

        assert_eq!(memory.intents.len(), 1);
        assert!(memory.intents.contains_key(&bk2));
    }

    #[test]
    fn clear_for_removes_matching_blocker_key() {
        let bk = sample_blocker_key();
        let other_bk = BlockerKey {
            goal_key: crate::GoalKey::from(GoalKind::ReduceDanger),
            place: None,
            target: None,
            action_def: None,
        };
        let mut memory = BlockedIntentMemory::default();
        memory.record(sample_blocked_intent());
        memory.record(make_intent(
            other_bk,
            BlockingFact::CombatTooRisky,
            Tick(30),
        ));

        memory.clear_for(&bk);

        assert_eq!(memory.intents.len(), 1);
        assert!(memory.intents.contains_key(&other_bk));
    }

    #[test]
    fn clear_all_for_goal_removes_all_entries_for_goal() {
        let key = sample_goal_key();
        let place_a = entity_id(10, 0);
        let place_b = entity_id(11, 0);
        let other_goal = crate::GoalKey::from(GoalKind::ReduceDanger);
        let mut memory = BlockedIntentMemory::default();

        memory.record(make_intent(
            BlockerKey {
                goal_key: key,
                place: Some(place_a),
                target: None,
                action_def: None,
            },
            BlockingFact::SourceDepleted,
            Tick(20),
        ));
        memory.record(make_intent(
            BlockerKey {
                goal_key: key,
                place: Some(place_b),
                target: None,
                action_def: None,
            },
            BlockingFact::SourceDepleted,
            Tick(25),
        ));
        memory.record(make_intent(
            BlockerKey {
                goal_key: other_goal,
                place: None,
                target: None,
                action_def: None,
            },
            BlockingFact::CombatTooRisky,
            Tick(30),
        ));

        memory.clear_all_for_goal(&key);

        assert_eq!(memory.intents.len(), 1);
        assert_eq!(
            memory.intents.values().next().unwrap().blocker_key.goal_key,
            other_goal
        );
    }

    #[test]
    fn blocked_intent_memory_roundtrips_through_bincode() {
        let mut memory = BlockedIntentMemory::default();
        let mut intent = sample_blocked_intent();
        intent.clearing_condition = BlockerClearingCondition::CommodityAvailabilityChanged {
            commodity: CommodityKind::Bread,
            place: entity_id(12, 0),
        };
        intent.baseline_snapshot = Some(ClearingBaseline::CommodityQuantity {
            quantity: Quantity(7),
        });
        memory.record(intent);

        let bytes = bincode::serialize(&memory).unwrap();
        let roundtrip: BlockedIntentMemory = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, memory);
    }

    #[test]
    fn exclusive_facility_blockers_do_not_block_goal_generation() {
        let key = sample_goal_key();
        let mut memory = BlockedIntentMemory::default();
        memory.record(make_intent(
            BlockerKey {
                goal_key: key,
                place: Some(entity_id(2, 0)),
                target: Some(entity_id(4, 0)),
                action_def: Some(ActionDefId(9)),
            },
            BlockingFact::ExclusiveFacilityUnavailable,
            Tick(30),
        ));

        assert!(!memory.is_blocked(
            &key,
            Some(entity_id(2, 0)),
            Some(entity_id(4, 0)),
            Some(ActionDefId(9)),
            Tick(11)
        ));
    }

    #[test]
    fn is_blocked_for_search_ignores_blocks_goal_generation_gate() {
        let key = sample_goal_key();
        let place = entity_id(2, 0);
        let mut memory = BlockedIntentMemory::default();
        memory.record(make_intent(
            BlockerKey {
                goal_key: key,
                place: Some(place),
                target: None,
                action_def: None,
            },
            BlockingFact::SourceDepleted,
            Tick(20),
        ));

        // is_blocked returns false (SourceDepleted doesn't block goal generation)
        assert!(!memory.is_blocked(&key, Some(place), None, None, Tick(9)));
        // is_blocked_for_search returns true (no blocks_goal_generation gate)
        assert!(memory.is_blocked_for_search(&key, Some(place), None, None, Tick(9)));
    }

    #[test]
    fn global_blocker_matches_any_place_query() {
        let key = sample_goal_key();
        let mut memory = BlockedIntentMemory::default();
        memory.record(make_intent(
            BlockerKey {
                goal_key: key,
                place: None,
                target: None,
                action_def: None,
            },
            BlockingFact::NoKnownPath,
            Tick(20),
        ));

        // Global blocker matches query with a specific place
        assert!(memory.is_blocked(&key, Some(entity_id(5, 0)), None, None, Tick(9)));
        // Global blocker matches query with no place
        assert!(memory.is_blocked(&key, None, None, None, Tick(9)));
    }

    #[test]
    fn place_scoped_blocker_does_not_match_different_place() {
        let key = sample_goal_key();
        let place_a = entity_id(10, 0);
        let place_b = entity_id(11, 0);
        let mut memory = BlockedIntentMemory::default();
        memory.record(make_intent(
            BlockerKey {
                goal_key: key,
                place: Some(place_a),
                target: None,
                action_def: None,
            },
            BlockingFact::NoKnownPath,
            Tick(20),
        ));

        // Matches at place_a
        assert!(memory.is_blocked(&key, Some(place_a), None, None, Tick(9)));
        // Does NOT match at place_b
        assert!(!memory.is_blocked(&key, Some(place_b), None, None, Tick(9)));
    }

    #[test]
    fn place_scoped_goal_blocking_fact_matches_global_query() {
        let key = sample_goal_key();
        let place_a = entity_id(10, 0);
        let mut memory = BlockedIntentMemory::default();
        memory.record(make_intent(
            BlockerKey {
                goal_key: key,
                place: Some(place_a),
                target: None,
                action_def: None,
            },
            BlockingFact::NoKnownPath,
            Tick(20),
        ));

        // Global query (None place) matches any goal-generation-blocking fact
        // regardless of key scope — candidate generation uses global queries
        assert!(memory.is_blocked(&key, None, None, None, Tick(9)));
    }

    #[test]
    fn place_scoped_non_goal_blocking_fact_does_not_match_global_query() {
        let key = sample_goal_key();
        let place_a = entity_id(10, 0);
        let mut memory = BlockedIntentMemory::default();
        memory.record(make_intent(
            BlockerKey {
                goal_key: key,
                place: Some(place_a),
                target: None,
                action_def: None,
            },
            BlockingFact::SourceDepleted,
            Tick(20),
        ));

        // SourceDepleted does not block goal generation, so global query does not match
        assert!(!memory.is_blocked(&key, None, None, None, Tick(9)));
    }

    #[test]
    fn patience_exhausted_blocks_goal_generation() {
        let intent = BlockedIntent {
            blocker_key: sample_blocker_key(),
            blocking_fact: BlockingFact::PatienceExhausted,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(100),
            clearing_condition: BlockerClearingCondition::TtlOnly,
            baseline_snapshot: None,
        };
        assert!(intent.blocks_goal_generation());
    }

    #[test]
    fn assumption_failed_blocks_goal_generation() {
        let intent = BlockedIntent {
            blocker_key: sample_blocker_key(),
            blocking_fact: BlockingFact::AssumptionFailed,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(100),
            clearing_condition: BlockerClearingCondition::TtlOnly,
            baseline_snapshot: None,
        };
        assert!(intent.blocks_goal_generation());
    }

    #[test]
    fn sweep_cleared_removes_matching_entries() {
        let retained_key = BlockerKey {
            goal_key: crate::GoalKey::from(GoalKind::ReduceDanger),
            place: None,
            target: None,
            action_def: None,
        };
        let mut memory = BlockedIntentMemory::default();
        memory.record(sample_blocked_intent());
        memory.record(make_intent(retained_key, BlockingFact::Unknown, Tick(50)));

        memory.sweep_cleared(|intent| {
            matches!(
                intent.clearing_condition,
                BlockerClearingCondition::CommodityAvailabilityChanged { .. }
            )
        });

        assert_eq!(memory.intents.len(), 1);
        assert!(memory.intents.contains_key(&retained_key));
    }

    #[test]
    fn sweep_cleared_retains_non_matching_entries() {
        let mut memory = BlockedIntentMemory::default();
        memory.record(make_intent(
            sample_blocker_key(),
            BlockingFact::NoKnownPath,
            Tick(50),
        ));

        memory.sweep_cleared(|_| false);

        assert_eq!(memory.intents.len(), 1);
    }

    #[test]
    fn pursuit_target_gone_blocker_scoped_to_target_and_place() {
        let target = entity_id(2, 0);
        let place_a = entity_id(10, 0);
        let place_b = entity_id(11, 0);
        let goal = GoalKind::RaidTarget { target };
        let key = crate::GoalKey::from(goal);

        let mut memory = BlockedIntentMemory::default();
        memory.record(make_intent(
            BlockerKey {
                goal_key: key,
                place: Some(place_a),
                target: Some(target),
                action_def: None,
            },
            BlockingFact::TargetGone,
            Tick(50),
        ));

        // Blocked at place_a for this target.
        assert!(memory.is_blocked(&key, Some(place_a), Some(target), None, Tick(5)));
        // NOT blocked at place_b — pursuit to a different believed place is allowed.
        assert!(!memory.is_blocked(&key, Some(place_b), Some(target), None, Tick(5)));
    }
}
