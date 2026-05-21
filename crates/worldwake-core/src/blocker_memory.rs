//! Authoritative blocker memory stored on agents.

use crate::{
    ActionDefId, AffordanceKey, BlockerScope, CommodityKind, Component, EntityId, EventId, GoalKey,
    Permille, Quantity, RouteSegment, Tick, UniqueItemKind,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
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
pub struct BlockerMemory {
    pub intents: BTreeMap<BlockerScope, Blocker>,
}

impl BlockerMemory {
    pub fn is_blocked(&self, scope: &BlockerScope, current_tick: Tick) -> bool {
        self.intents.values().any(|intent| {
            intent.expires_tick > current_tick
                && intent.blocks_goal_generation()
                && matches_scope(&intent.scope, scope)
        })
    }

    pub fn is_blocked_for_search(&self, scope: &BlockerScope, current_tick: Tick) -> bool {
        self.find_blocked_for_search(scope, current_tick).is_some()
    }

    /// Like `is_blocked_for_search` but returns the matching `Blocker`
    /// reference so callers can inspect the `blocking_fact` for trace recording.
    pub fn find_blocked_for_search(
        &self,
        scope: &BlockerScope,
        current_tick: Tick,
    ) -> Option<&Blocker> {
        self.intents.values().find(|intent| {
            intent.expires_tick > current_tick && matches_scope(&intent.scope, scope)
        })
    }

    pub fn record(&mut self, intent: Blocker) {
        self.intents.insert(intent.scope, intent);
    }

    pub fn expire(&mut self, current_tick: Tick) {
        self.intents
            .retain(|_, intent| intent.expires_tick > current_tick);
    }

    pub fn sweep_cleared(&mut self, mut is_cleared: impl FnMut(&Blocker) -> bool) {
        self.intents.retain(|_, intent| !is_cleared(intent));
    }

    pub fn clear_for(&mut self, scope: &BlockerScope) {
        self.intents.remove(scope);
    }

    pub fn clear_all_for_goal(&mut self, goal_key: &GoalKey) {
        self.intents.retain(|scope, _| match scope {
            BlockerScope::Exact(key) => key.goal_key != *goal_key,
            BlockerScope::RouteSegment(_) | BlockerScope::Counterparty(_) => true,
        });
    }

    pub fn route_segment_blocked(
        &self,
        from: EntityId,
        to: EntityId,
        current_tick: Tick,
    ) -> Option<&Blocker> {
        self.find_blocked_for_search(
            &BlockerScope::RouteSegment(RouteSegment::new(from, to)),
            current_tick,
        )
    }

    pub fn counterparty_blocked(&self, other: EntityId, current_tick: Tick) -> Option<&Blocker> {
        self.find_blocked_for_search(&BlockerScope::Counterparty(other), current_tick)
    }

    pub fn any_blocker_on_path(&self, path: &[EntityId], current_tick: Tick) -> Option<&Blocker> {
        path.windows(2).find_map(|pair| {
            let [from, to] = pair else {
                return None;
            };
            self.route_segment_blocked(*from, *to, current_tick)
        })
    }
}

fn matches_exact_scope(blocker: &BlockerKey, query: &BlockerKey) -> bool {
    // Goal-scoped blocker (place=None, target=None, action=None) matches everything
    if blocker.place.is_none() && blocker.target.is_none() && blocker.action_def.is_none() {
        return blocker.goal_key == query.goal_key;
    }
    if blocker.goal_key != query.goal_key {
        return false;
    }
    // Place must match if blocker has one
    if let Some(blocker_place) = blocker.place
        && query.place.is_some()
        && query.place != Some(blocker_place)
    {
        return false;
    }
    // Target must match if blocker has one
    if let Some(blocker_target) = blocker.target
        && query.target != Some(blocker_target)
    {
        return false;
    }
    // Action must match if blocker has one
    if let Some(blocker_action) = blocker.action_def
        && query.action_def != Some(blocker_action)
    {
        return false;
    }
    true
}

fn matches_scope(blocker: &BlockerScope, query: &BlockerScope) -> bool {
    match (blocker, query) {
        (BlockerScope::Exact(blocker), BlockerScope::Exact(query)) => {
            matches_exact_scope(blocker, query)
        }
        (BlockerScope::RouteSegment(blocker), BlockerScope::RouteSegment(query)) => {
            blocker == query
        }
        (BlockerScope::Counterparty(blocker), BlockerScope::Counterparty(query)) => {
            blocker == query
        }
        _ => false,
    }
}

impl Component for BlockerMemory {}

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
    RouteRetraversedSafely(RouteSegment),
    CounterpartyAccepted(EntityId),
    TtlOnly,
}

impl BlockerClearingCondition {
    #[must_use]
    pub const fn for_scope_and_fact(
        scope: BlockerScope,
        fact: BlockingFact,
        fallback: Self,
    ) -> Self {
        match (scope, fact) {
            (
                BlockerScope::RouteSegment(segment),
                BlockingFact::DangerTooHigh | BlockingFact::CombatTooRisky,
            ) => Self::RouteRetraversedSafely(segment),
            (
                BlockerScope::Counterparty(counterparty),
                BlockingFact::PatienceExhausted | BlockingFact::NoBuyer,
            ) => Self::CounterpartyAccepted(counterparty),
            _ => fallback,
        }
    }
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
pub struct Blocker {
    pub scope: BlockerScope,
    pub blocking_fact: BlockingFact,
    pub diagnostic_context: Option<BlockerDiagnostic>,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
    pub clearing_condition: BlockerClearingCondition,
    pub baseline_snapshot: Option<ClearingBaseline>,
    pub source_event: Option<EventId>,
}

impl Blocker {
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
    ReservationConflict {
        affordance: AffordanceKey,
        contention_event: Option<EventId>,
    },
    ExclusiveFacilityUnavailable,
    MissingTool(UniqueItemKind),
    MissingInput(CommodityKind),
    TargetGone,
    DangerTooHigh,
    CombatTooRisky,
    /// Frame patience exhausted for this goal at this place/target.
    PatienceExhausted,
    /// Seller staffed a market but no trade occurred during the presence cycle.
    NoBuyer,
}

#[cfg(test)]
mod tests {
    use super::{
        Blocker, BlockerClearingCondition, BlockerDiagnostic, BlockerKey, BlockerMemory,
        BlockingFact, ClearingBaseline,
    };
    use crate::{
        AcquisitionQuantity, ActionDefId, AffordanceKey, BlockerScope, CommodityKind, EventId,
        GoalKind, Quantity, RouteSegment, Tick, UniqueItemKind,
        test_utils::{entity_id, sample_blocker, sample_blocker_key, sample_goal_key},
        traits::Component,
    };
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn assert_copy_value_bounds<T: Copy + Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn make_intent(key: BlockerKey, fact: BlockingFact, expires: Tick) -> Blocker {
        Blocker {
            scope: key.into(),
            blocking_fact: fact,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: expires,
            clearing_condition: BlockerClearingCondition::TtlOnly,
            baseline_snapshot: None,
            source_event: Some(EventId(1)),
        }
    }

    #[test]
    fn blocker_types_satisfy_required_bounds() {
        assert_component_bounds::<BlockerMemory>();
        assert_value_bounds::<BlockerMemory>();
        assert_copy_value_bounds::<Blocker>();
        assert_copy_value_bounds::<BlockerClearingCondition>();
        assert_copy_value_bounds::<ClearingBaseline>();
        assert_copy_value_bounds::<BlockingFact>();
        assert_copy_value_bounds::<BlockerScope>();
        assert_copy_value_bounds::<RouteSegment>();
        assert_value_bounds::<BlockerKey>();
        assert_value_bounds::<BlockerDiagnostic>();
    }

    #[test]
    fn blocker_clearing_condition_and_baseline_satisfy_required_bounds() {
        assert_copy_value_bounds::<BlockerClearingCondition>();
        assert_copy_value_bounds::<ClearingBaseline>();
    }

    #[test]
    fn scope_aware_clearing_condition_selection_is_deterministic() {
        let segment = RouteSegment::new(entity_id(1, 0), entity_id(2, 0));
        let counterparty = entity_id(3, 0);

        assert_eq!(
            BlockerClearingCondition::for_scope_and_fact(
                BlockerScope::RouteSegment(segment),
                BlockingFact::DangerTooHigh,
                BlockerClearingCondition::TtlOnly,
            ),
            BlockerClearingCondition::RouteRetraversedSafely(segment)
        );
        assert_eq!(
            BlockerClearingCondition::for_scope_and_fact(
                BlockerScope::RouteSegment(segment),
                BlockingFact::CombatTooRisky,
                BlockerClearingCondition::TtlOnly,
            ),
            BlockerClearingCondition::RouteRetraversedSafely(segment)
        );
        assert_eq!(
            BlockerClearingCondition::for_scope_and_fact(
                BlockerScope::Counterparty(counterparty),
                BlockingFact::NoBuyer,
                BlockerClearingCondition::TtlOnly,
            ),
            BlockerClearingCondition::CounterpartyAccepted(counterparty)
        );
        assert_eq!(
            BlockerClearingCondition::for_scope_and_fact(
                BlockerScope::Counterparty(counterparty),
                BlockingFact::PatienceExhausted,
                BlockerClearingCondition::TtlOnly,
            ),
            BlockerClearingCondition::CounterpartyAccepted(counterparty)
        );
        assert_eq!(
            BlockerClearingCondition::for_scope_and_fact(
                BlockerScope::exact(sample_goal_key(), None, None, None),
                BlockingFact::DangerTooHigh,
                BlockerClearingCondition::TtlOnly,
            ),
            BlockerClearingCondition::TtlOnly
        );
    }

    #[test]
    fn blocker_memory_defaults_empty() {
        let memory = BlockerMemory::default();
        assert!(memory.intents.is_empty());
    }

    #[test]
    fn is_blocked_matches_only_live_entries_for_goal_key() {
        let key = sample_goal_key();
        let stale_key = crate::GoalKey::from(GoalKind::Sleep);
        let mut memory = BlockerMemory::default();

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
        assert!(memory.is_blocked(&BlockerScope::exact(key, None, None, None), Tick(9)));
        // Expired at tick 10
        assert!(!memory.is_blocked(&BlockerScope::exact(key, None, None, None), Tick(10)));
        // Expired at tick 20
        assert!(!memory.is_blocked(&BlockerScope::exact(stale_key, None, None, None), Tick(20)));

        memory.expire(Tick(10));
        assert_eq!(memory.intents.len(), 1);
    }

    #[test]
    fn source_depleted_does_not_block_goal_generation() {
        let key = sample_goal_key();
        let mut memory = BlockerMemory::default();
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
            &BlockerScope::exact(key, Some(entity_id(2, 0)), Some(entity_id(4, 0)), None),
            Tick(9),
        ));
    }

    #[test]
    fn record_replaces_existing_entry_for_same_compound_key() {
        let bk = sample_blocker_key();
        let original = sample_blocker();
        let replacement = Blocker {
            scope: bk.into(),
            blocking_fact: BlockingFact::MissingTool(UniqueItemKind::SimpleTool),
            diagnostic_context: None,
            observed_tick: Tick(11),
            expires_tick: Tick(19),
            clearing_condition: BlockerClearingCondition::TtlOnly,
            baseline_snapshot: None,
            source_event: Some(EventId(2)),
        };
        let mut memory = BlockerMemory::default();
        memory.record(original);
        memory.record(replacement);

        assert_eq!(memory.intents.len(), 1);
        assert_eq!(memory.intents[&bk.into()], replacement);
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
        let mut memory = BlockerMemory::default();
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
            quantity: AcquisitionQuantity::single(),
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
        let mut memory = BlockerMemory::default();
        memory.record(make_intent(bk1, BlockingFact::NoKnownPath, Tick(14)));
        memory.record(make_intent(bk2, BlockingFact::NoKnownPath, Tick(15)));

        memory.expire(Tick(14));

        assert_eq!(memory.intents.len(), 1);
        assert!(memory.intents.contains_key(&bk2.into()));
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
        let mut memory = BlockerMemory::default();
        memory.record(sample_blocker());
        memory.record(make_intent(
            other_bk,
            BlockingFact::CombatTooRisky,
            Tick(30),
        ));

        memory.clear_for(&bk.into());

        assert_eq!(memory.intents.len(), 1);
        assert!(memory.intents.contains_key(&other_bk.into()));
    }

    #[test]
    fn clear_all_for_goal_removes_all_entries_for_goal() {
        let key = sample_goal_key();
        let place_a = entity_id(10, 0);
        let place_b = entity_id(11, 0);
        let other_goal = crate::GoalKey::from(GoalKind::ReduceDanger);
        let mut memory = BlockerMemory::default();

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
            memory
                .intents
                .values()
                .next()
                .unwrap()
                .scope
                .exact_goal_key()
                .unwrap(),
            other_goal
        );
    }

    #[test]
    fn blocker_memory_roundtrips_through_bincode() {
        let mut memory = BlockerMemory::default();
        let mut intent = sample_blocker();
        intent.clearing_condition = BlockerClearingCondition::CommodityAvailabilityChanged {
            commodity: CommodityKind::Bread,
            place: entity_id(12, 0),
        };
        intent.baseline_snapshot = Some(ClearingBaseline::CommodityQuantity {
            quantity: Quantity(7),
        });
        memory.record(intent);

        let bytes = bincode::serialize(&memory).unwrap();
        let roundtrip: BlockerMemory = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, memory);
    }

    #[test]
    fn blocker_memory_preserves_explicit_absent_source_event() {
        let mut memory = BlockerMemory::default();
        let mut intent = sample_blocker();
        intent.source_event = None;
        memory.record(intent);

        let bytes = bincode::serialize(&memory).unwrap();
        let roundtrip: BlockerMemory = bincode::deserialize(&bytes).unwrap();

        assert_eq!(
            roundtrip.intents.values().next().unwrap().source_event,
            None
        );
    }

    #[test]
    fn blocker_memory_roundtrips_non_exact_scope_entries() {
        let mut memory = BlockerMemory::default();
        let route_scope =
            BlockerScope::RouteSegment(RouteSegment::new(entity_id(10, 0), entity_id(11, 0)));
        let counterparty_scope = BlockerScope::Counterparty(entity_id(12, 0));
        memory.record(Blocker {
            scope: route_scope,
            blocking_fact: BlockingFact::NoKnownPath,
            diagnostic_context: None,
            observed_tick: Tick(2),
            expires_tick: Tick(20),
            clearing_condition: BlockerClearingCondition::RouteRetraversedSafely(
                RouteSegment::new(entity_id(10, 0), entity_id(11, 0)),
            ),
            baseline_snapshot: None,
            source_event: Some(EventId(7)),
        });
        memory.record(Blocker {
            scope: counterparty_scope,
            blocking_fact: BlockingFact::NoBuyer,
            diagnostic_context: None,
            observed_tick: Tick(3),
            expires_tick: Tick(30),
            clearing_condition: BlockerClearingCondition::CounterpartyAccepted(entity_id(12, 0)),
            baseline_snapshot: None,
            source_event: Some(EventId(8)),
        });

        let bytes = bincode::serialize(&memory).unwrap();
        let roundtrip: BlockerMemory = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, memory);
        assert_eq!(
            roundtrip.intents[&route_scope].source_event,
            Some(EventId(7))
        );
        assert_eq!(
            roundtrip.intents[&counterparty_scope].source_event,
            Some(EventId(8))
        );
    }

    #[test]
    fn reservation_conflict_blocking_fact_roundtrips_with_affordance_and_event() {
        let fact = BlockingFact::ReservationConflict {
            affordance: AffordanceKey {
                facility: entity_id(44, 0),
                action: ActionDefId(7),
            },
            contention_event: Some(EventId(99)),
        };

        let bytes = bincode::serialize(&fact).unwrap();
        let roundtrip: BlockingFact = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, fact);
    }

    #[test]
    fn exclusive_facility_blockers_do_not_block_goal_generation() {
        let key = sample_goal_key();
        let mut memory = BlockerMemory::default();
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
            &BlockerScope::exact(
                key,
                Some(entity_id(2, 0)),
                Some(entity_id(4, 0)),
                Some(ActionDefId(9)),
            ),
            Tick(11),
        ));
    }

    #[test]
    fn is_blocked_for_search_ignores_blocks_goal_generation_gate() {
        let key = sample_goal_key();
        let place = entity_id(2, 0);
        let mut memory = BlockerMemory::default();
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
        assert!(!memory.is_blocked(&BlockerScope::exact(key, Some(place), None, None), Tick(9)));
        // is_blocked_for_search returns true (no blocks_goal_generation gate)
        assert!(
            memory
                .is_blocked_for_search(&BlockerScope::exact(key, Some(place), None, None), Tick(9))
        );
    }

    #[test]
    fn route_segment_blocked_matches_canonical_segment() {
        let from = entity_id(20, 0);
        let to = entity_id(21, 0);
        let mut memory = BlockerMemory::default();
        memory.record(Blocker {
            scope: BlockerScope::RouteSegment(RouteSegment::new(from, to)),
            blocking_fact: BlockingFact::NoKnownPath,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(10),
            clearing_condition: BlockerClearingCondition::TtlOnly,
            baseline_snapshot: None,
            source_event: Some(EventId(4)),
        });

        assert!(memory.route_segment_blocked(to, from, Tick(5)).is_some());
        assert!(memory.route_segment_blocked(from, to, Tick(10)).is_none());
    }

    #[test]
    fn counterparty_blocked_matches_counterparty_scope_only() {
        let counterparty = entity_id(30, 0);
        let mut memory = BlockerMemory::default();
        memory.record(Blocker {
            scope: BlockerScope::Counterparty(counterparty),
            blocking_fact: BlockingFact::NoBuyer,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(10),
            clearing_condition: BlockerClearingCondition::TtlOnly,
            baseline_snapshot: None,
            source_event: Some(EventId(5)),
        });

        assert!(memory.counterparty_blocked(counterparty, Tick(5)).is_some());
        assert!(
            memory
                .counterparty_blocked(entity_id(31, 0), Tick(5))
                .is_none()
        );
    }

    #[test]
    fn any_blocker_on_path_returns_first_blocked_segment() {
        let a = entity_id(40, 0);
        let b = entity_id(41, 0);
        let c = entity_id(42, 0);
        let mut memory = BlockerMemory::default();
        memory.record(Blocker {
            scope: BlockerScope::RouteSegment(RouteSegment::new(b, c)),
            blocking_fact: BlockingFact::DangerTooHigh,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(10),
            clearing_condition: BlockerClearingCondition::TtlOnly,
            baseline_snapshot: None,
            source_event: Some(EventId(6)),
        });

        assert_eq!(
            memory
                .any_blocker_on_path(&[a, b, c], Tick(5))
                .map(|blocker| blocker.scope),
            Some(BlockerScope::RouteSegment(RouteSegment::new(b, c)))
        );
        assert!(memory.any_blocker_on_path(&[a, b], Tick(5)).is_none());
    }

    #[test]
    fn global_blocker_matches_any_place_query() {
        let key = sample_goal_key();
        let mut memory = BlockerMemory::default();
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
        assert!(memory.is_blocked(
            &BlockerScope::exact(key, Some(entity_id(5, 0)), None, None),
            Tick(9),
        ));
        // Global blocker matches query with no place
        assert!(memory.is_blocked(&BlockerScope::exact(key, None, None, None), Tick(9)));
    }

    #[test]
    fn place_scoped_blocker_does_not_match_different_place() {
        let key = sample_goal_key();
        let place_a = entity_id(10, 0);
        let place_b = entity_id(11, 0);
        let mut memory = BlockerMemory::default();
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
        assert!(memory.is_blocked(
            &BlockerScope::exact(key, Some(place_a), None, None),
            Tick(9)
        ));
        // Does NOT match at place_b
        assert!(!memory.is_blocked(
            &BlockerScope::exact(key, Some(place_b), None, None),
            Tick(9)
        ));
    }

    #[test]
    fn place_scoped_goal_blocking_fact_matches_global_query() {
        let key = sample_goal_key();
        let place_a = entity_id(10, 0);
        let mut memory = BlockerMemory::default();
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
        assert!(memory.is_blocked(&BlockerScope::exact(key, None, None, None), Tick(9)));
    }

    #[test]
    fn place_scoped_non_goal_blocking_fact_does_not_match_global_query() {
        let key = sample_goal_key();
        let place_a = entity_id(10, 0);
        let mut memory = BlockerMemory::default();
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
        assert!(!memory.is_blocked(&BlockerScope::exact(key, None, None, None), Tick(9)));
    }

    #[test]
    fn patience_exhausted_blocks_goal_generation() {
        let intent = Blocker {
            scope: sample_blocker_key().into(),
            blocking_fact: BlockingFact::PatienceExhausted,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(100),
            clearing_condition: BlockerClearingCondition::TtlOnly,
            baseline_snapshot: None,
            source_event: Some(EventId(3)),
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
        let mut memory = BlockerMemory::default();
        memory.record(sample_blocker());
        memory.record(make_intent(
            retained_key,
            BlockingFact::NoKnownPath,
            Tick(50),
        ));

        memory.sweep_cleared(|intent| {
            matches!(
                intent.clearing_condition,
                BlockerClearingCondition::CommodityAvailabilityChanged { .. }
            )
        });

        assert_eq!(memory.intents.len(), 1);
        assert!(memory.intents.contains_key(&retained_key.into()));
    }

    #[test]
    fn sweep_cleared_retains_non_matching_entries() {
        let mut memory = BlockerMemory::default();
        memory.record(make_intent(
            sample_blocker_key(),
            BlockingFact::NoKnownPath,
            Tick(50),
        ));

        memory.sweep_cleared(|_| false);

        assert_eq!(memory.intents.len(), 1);
    }

    #[test]
    fn sweep_cleared_removes_route_retraversed_safely_blockers() {
        let segment = RouteSegment::new(entity_id(50, 0), entity_id(51, 0));
        let retained_segment = RouteSegment::new(entity_id(52, 0), entity_id(53, 0));
        let mut memory = BlockerMemory::default();
        memory.record(Blocker {
            scope: BlockerScope::RouteSegment(segment),
            blocking_fact: BlockingFact::DangerTooHigh,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(20),
            clearing_condition: BlockerClearingCondition::RouteRetraversedSafely(segment),
            baseline_snapshot: None,
            source_event: Some(EventId(9)),
        });
        memory.record(Blocker {
            scope: BlockerScope::RouteSegment(retained_segment),
            blocking_fact: BlockingFact::DangerTooHigh,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(20),
            clearing_condition: BlockerClearingCondition::RouteRetraversedSafely(retained_segment),
            baseline_snapshot: None,
            source_event: Some(EventId(10)),
        });

        memory.sweep_cleared(|blocker| {
            matches!(
                blocker.clearing_condition,
                BlockerClearingCondition::RouteRetraversedSafely(cleared) if cleared == segment
            )
        });

        assert_eq!(memory.intents.len(), 1);
        assert!(
            memory
                .intents
                .contains_key(&BlockerScope::RouteSegment(retained_segment))
        );
    }

    #[test]
    fn sweep_cleared_removes_counterparty_accepted_blockers() {
        let counterparty = entity_id(60, 0);
        let retained_counterparty = entity_id(61, 0);
        let mut memory = BlockerMemory::default();
        memory.record(Blocker {
            scope: BlockerScope::Counterparty(counterparty),
            blocking_fact: BlockingFact::NoBuyer,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(20),
            clearing_condition: BlockerClearingCondition::CounterpartyAccepted(counterparty),
            baseline_snapshot: None,
            source_event: Some(EventId(11)),
        });
        memory.record(Blocker {
            scope: BlockerScope::Counterparty(retained_counterparty),
            blocking_fact: BlockingFact::NoBuyer,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(20),
            clearing_condition: BlockerClearingCondition::CounterpartyAccepted(
                retained_counterparty,
            ),
            baseline_snapshot: None,
            source_event: Some(EventId(12)),
        });

        memory.sweep_cleared(|blocker| {
            matches!(
                blocker.clearing_condition,
                BlockerClearingCondition::CounterpartyAccepted(cleared)
                    if cleared == counterparty
            )
        });

        assert_eq!(memory.intents.len(), 1);
        assert!(
            memory
                .intents
                .contains_key(&BlockerScope::Counterparty(retained_counterparty))
        );
    }

    #[test]
    fn pursuit_target_gone_blocker_scoped_to_target_and_place() {
        let target = entity_id(2, 0);
        let place_a = entity_id(10, 0);
        let place_b = entity_id(11, 0);
        let goal = GoalKind::RaidTarget { target };
        let key = crate::GoalKey::from(goal);

        let mut memory = BlockerMemory::default();
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
        assert!(memory.is_blocked(
            &BlockerScope::exact(key, Some(place_a), Some(target), None),
            Tick(5)
        ));
        // NOT blocked at place_b — pursuit to a different believed place is allowed.
        assert!(!memory.is_blocked(
            &BlockerScope::exact(key, Some(place_b), Some(target), None),
            Tick(5)
        ));
    }
}
