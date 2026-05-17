use crate::{
    BeliefClaimKey, BlockerReason, BlockerScope, CommodityKind, Component, EntityId, EventId,
    HomeostaticNeedId, MethodSchemaId, OmissionReason, Tick,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Discrepancy {
    /// A previously believed fact has aged out and should be reverified.
    BeliefStale,
    /// New evidence directly contradicts a previously believed claim.
    BeliefContradicted,
    /// A committed concrete source failed while the broader goal may still hold.
    SourceInvalidated,
    /// Planning assumed a state that should never have been treated as valid.
    ImproperPlanningState,
    /// The agent lacks a needed observation to continue safely.
    MissingObservation,
    /// The world no longer supports the believed legal or institutional binding.
    NoLegalBinding,
    /// A needed counterparty exists but is unwilling or unavailable to cooperate.
    NoWillingCounterparty,
    /// The agent does not currently know a route needed for the plan.
    RouteUnknown,
    /// Search terminated before finding a viable plan within budget.
    SearchBudgetExhausted,
    /// Execution partially committed before the plan drifted out from under it.
    PartialExecutionDrift,
    /// A `NeedSafeUntilTick` assumption breached: the projected tick at
    /// which the named need crosses its high threshold fell before the
    /// plan-completion tick the assumption was guarding.
    NeedHorizonExceeded {
        need: HomeostaticNeedId,
        projected_breach_tick: Tick,
    },
    /// The agent could not revalidate against an entity that perception had
    /// dropped from the belief store under the given salience-budget reason.
    Omission(OmissionReason),
    /// A social artifact known to the planner cannot currently be acted on.
    ArtifactNotActionable {
        artifact: EntityId,
        reason: BlockerReason,
    },
    /// A method-level pursuit pattern failed before or during subgoal execution.
    MethodFailure(MethodFailureContext),
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
pub struct MethodFailureContext {
    pub method_id: MethodSchemaId,
    pub kind: MethodFailureKind,
    pub subgoal_index: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
pub enum MethodFailureKind {
    PreconditionLost,
    SubgoalUnachievable,
    ArtifactNotProduced,
    ClaimDenied,
    Timeout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiscrepancyEntry {
    pub scope: BlockerScope,
    pub discrepancy: Discrepancy,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
    pub clearing_condition: DiscrepancyClearing,
    pub source_event: EventId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DiscrepancyClearing {
    TtlExpiry,
    ReobservationOf {
        target: EntityId,
    },
    BeliefUpdate {
        claim_key: BeliefClaimKey,
    },
    CommodityAvailabilityChanged {
        commodity: CommodityKind,
        place: EntityId,
    },
    WorldStructureChange,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiscrepancyMemory {
    pub entries: BTreeMap<BlockerScope, DiscrepancyEntry>,
}

impl DiscrepancyMemory {
    pub fn record(&mut self, entry: DiscrepancyEntry) {
        self.entries.insert(entry.scope, entry);
    }

    pub fn expire(&mut self, current_tick: Tick) {
        self.entries
            .retain(|_, entry| entry.expires_tick > current_tick);
    }

    #[must_use]
    pub fn is_suppressed(&self, scope: &BlockerScope, current_tick: Tick) -> bool {
        self.entries
            .get(scope)
            .is_some_and(|entry| entry.expires_tick > current_tick)
    }

    pub fn clear_for(&mut self, scope: &BlockerScope) {
        self.entries.remove(scope);
    }

    pub fn clear_by_condition(&mut self, mut predicate: impl FnMut(&DiscrepancyEntry) -> bool) {
        self.entries.retain(|_, entry| !predicate(entry));
    }
}

impl Component for DiscrepancyMemory {}

#[cfg(test)]
mod tests {
    use super::{
        Discrepancy, DiscrepancyClearing, DiscrepancyEntry, DiscrepancyMemory,
        MethodFailureContext, MethodFailureKind,
    };
    use crate::{
        ActionDefId, BeliefClaimKey, BlockerKey, BlockerReason, BlockerScope, CommodityKind,
        EntityBeliefAspect, EntityId, EventId, GoalKind, MethodSchemaId, OmissionReason,
        RouteSegment, SaliencePolicy, Tick,
        test_utils::{entity_id, sample_goal_key},
        traits::Component,
    };
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn assert_copy_value_bounds<T: Copy + Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn blocker_key() -> BlockerKey {
        BlockerKey {
            goal_key: sample_goal_key(),
            place: Some(entity_id(3, 0)),
            target: Some(entity_id(4, 0)),
            action_def: Some(ActionDefId(5)),
        }
    }

    fn discrepancy_entry(
        key: BlockerKey,
        discrepancy: Discrepancy,
        expires_tick: Tick,
        clearing_condition: DiscrepancyClearing,
    ) -> DiscrepancyEntry {
        DiscrepancyEntry {
            scope: key.into(),
            discrepancy,
            observed_tick: Tick(9),
            expires_tick,
            clearing_condition,
            source_event: EventId(1),
        }
    }

    #[test]
    fn discrepancy_types_satisfy_required_bounds() {
        assert_component_bounds::<DiscrepancyMemory>();
        assert_value_bounds::<DiscrepancyMemory>();
        assert_copy_value_bounds::<Discrepancy>();
        assert_copy_value_bounds::<DiscrepancyEntry>();
        assert_copy_value_bounds::<DiscrepancyClearing>();
        assert_copy_value_bounds::<MethodFailureContext>();
        assert_copy_value_bounds::<MethodFailureKind>();
    }

    #[test]
    fn discrepancy_roundtrips_through_bincode() {
        let discrepancy = Discrepancy::SourceInvalidated;

        let bytes = bincode::serialize(&discrepancy).unwrap();
        let roundtrip: Discrepancy = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, discrepancy);
    }

    #[test]
    fn discrepancy_need_horizon_exceeded_roundtrips_through_bincode() {
        let discrepancy = Discrepancy::NeedHorizonExceeded {
            need: crate::HomeostaticNeedId::Hunger,
            projected_breach_tick: Tick(50),
        };

        let bytes = bincode::serialize(&discrepancy).unwrap();
        let roundtrip: Discrepancy = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, discrepancy);
    }

    #[test]
    fn discrepancy_omission_roundtrips_through_bincode() {
        let discrepancy = Discrepancy::Omission(OmissionReason::OverBudget {
            budget: 5,
            candidates_seen: 10,
        });

        let bytes = bincode::serialize(&discrepancy).unwrap();
        let roundtrip: Discrepancy = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, discrepancy);
    }

    #[test]
    fn discrepancy_artifact_not_actionable_roundtrips_through_bincode() {
        let discrepancy = Discrepancy::ArtifactNotActionable {
            artifact: entity_id(42, 0),
            reason: BlockerReason::LegalEffectExpired,
        };

        let bytes = bincode::serialize(&discrepancy).unwrap();
        let roundtrip: Discrepancy = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, discrepancy);
    }

    #[test]
    fn discrepancy_method_failure_roundtrips_through_bincode() {
        let discrepancy = Discrepancy::MethodFailure(MethodFailureContext {
            method_id: MethodSchemaId(7),
            kind: MethodFailureKind::SubgoalUnachievable,
            subgoal_index: Some(2),
        });

        let bytes = bincode::serialize(&discrepancy).unwrap();
        let roundtrip: Discrepancy = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, discrepancy);
    }

    #[test]
    fn discrepancy_omission_orders_after_existing_variants() {
        let need_horizon = Discrepancy::NeedHorizonExceeded {
            need: crate::HomeostaticNeedId::Hunger,
            projected_breach_tick: Tick(50),
        };
        let omission = Discrepancy::Omission(OmissionReason::SalienceBelowFloor {
            policy: SaliencePolicy::PriorityWithNeedBoost,
        });

        assert!(omission > need_horizon);
    }

    #[test]
    fn discrepancy_entry_roundtrips_through_bincode() {
        let entry = discrepancy_entry(
            blocker_key(),
            Discrepancy::RouteUnknown,
            Tick(19),
            DiscrepancyClearing::ReobservationOf {
                target: entity_id(4, 0),
            },
        );

        let bytes = bincode::serialize(&entry).unwrap();
        let roundtrip: DiscrepancyEntry = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, entry);
    }

    #[test]
    fn discrepancy_memory_roundtrips_non_exact_scope_entries() {
        let route_scope =
            BlockerScope::RouteSegment(RouteSegment::new(entity_id(10, 0), entity_id(11, 0)));
        let counterparty_scope = BlockerScope::Counterparty(entity_id(12, 0));
        let mut memory = DiscrepancyMemory::default();
        memory.record(DiscrepancyEntry {
            scope: route_scope,
            discrepancy: Discrepancy::RouteUnknown,
            observed_tick: Tick(2),
            expires_tick: Tick(20),
            clearing_condition: DiscrepancyClearing::WorldStructureChange,
            source_event: EventId(7),
        });
        memory.record(DiscrepancyEntry {
            scope: counterparty_scope,
            discrepancy: Discrepancy::NoWillingCounterparty,
            observed_tick: Tick(3),
            expires_tick: Tick(30),
            clearing_condition: DiscrepancyClearing::TtlExpiry,
            source_event: EventId(8),
        });

        let bytes = bincode::serialize(&memory).unwrap();
        let roundtrip: DiscrepancyMemory = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, memory);
        assert!(roundtrip.is_suppressed(&route_scope, Tick(10)));
        assert!(roundtrip.is_suppressed(&counterparty_scope, Tick(10)));
        assert_eq!(roundtrip.entries[&route_scope].source_event, EventId(7));
        assert_eq!(
            roundtrip.entries[&counterparty_scope].source_event,
            EventId(8)
        );
    }

    #[test]
    fn discrepancy_clearing_roundtrips_through_bincode() {
        let clearing = DiscrepancyClearing::BeliefUpdate {
            claim_key: BeliefClaimKey {
                subject: EntityId {
                    slot: 8,
                    generation: 0,
                },
                aspect: EntityBeliefAspect::Inventory(CommodityKind::Bread),
            },
        };

        let bytes = bincode::serialize(&clearing).unwrap();
        let roundtrip: DiscrepancyClearing = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, clearing);
    }

    #[test]
    fn discrepancy_memory_record_and_expire_prunes_stale_entries() {
        let key = blocker_key();
        let other_key = BlockerKey {
            goal_key: crate::GoalKey::from(GoalKind::Sleep),
            place: None,
            target: None,
            action_def: None,
        };
        let mut memory = DiscrepancyMemory::default();
        memory.record(discrepancy_entry(
            key,
            Discrepancy::BeliefStale,
            Tick(10),
            DiscrepancyClearing::TtlExpiry,
        ));
        memory.record(discrepancy_entry(
            other_key,
            Discrepancy::ImproperPlanningState,
            Tick(11),
            DiscrepancyClearing::WorldStructureChange,
        ));

        assert!(memory.is_suppressed(&key.into(), Tick(9)));
        assert!(!memory.is_suppressed(&key.into(), Tick(10)));

        memory.expire(Tick(10));

        assert_eq!(memory.entries.len(), 1);
        assert!(memory.entries.contains_key(&other_key.into()));
    }

    #[test]
    fn discrepancy_memory_clear_for_removes_matching_entry() {
        let key = blocker_key();
        let mut memory = DiscrepancyMemory::default();
        memory.record(discrepancy_entry(
            key,
            Discrepancy::BeliefContradicted,
            Tick(20),
            DiscrepancyClearing::TtlExpiry,
        ));

        memory.clear_for(&key.into());

        assert!(memory.entries.is_empty());
    }

    #[test]
    fn discrepancy_memory_clear_by_condition_matches_reobservation_target() {
        let clear_target = entity_id(10, 0);
        let retained_target = entity_id(11, 0);
        let mut memory = DiscrepancyMemory::default();

        let mut clear_key = blocker_key();
        clear_key.target = Some(clear_target);
        memory.record(discrepancy_entry(
            clear_key,
            Discrepancy::MissingObservation,
            Tick(20),
            DiscrepancyClearing::ReobservationOf {
                target: clear_target,
            },
        ));

        let mut retained_key = blocker_key();
        retained_key.goal_key = crate::GoalKey::from(GoalKind::Sleep);
        retained_key.target = Some(retained_target);
        memory.record(discrepancy_entry(
            retained_key,
            Discrepancy::MissingObservation,
            Tick(20),
            DiscrepancyClearing::ReobservationOf {
                target: retained_target,
            },
        ));

        memory.clear_by_condition(|entry| {
            matches!(
                entry.clearing_condition,
                DiscrepancyClearing::ReobservationOf { target } if target == clear_target
            )
        });

        assert!(!memory.entries.contains_key(&clear_key.into()));
        assert!(memory.entries.contains_key(&retained_key.into()));
    }

    #[test]
    fn discrepancy_memory_clear_by_condition_matches_belief_update_key() {
        let clear_claim_key = BeliefClaimKey {
            subject: entity_id(20, 0),
            aspect: EntityBeliefAspect::Alive,
        };
        let retained_claim_key = BeliefClaimKey {
            subject: entity_id(21, 0),
            aspect: EntityBeliefAspect::Location,
        };
        let mut memory = DiscrepancyMemory::default();

        memory.record(discrepancy_entry(
            blocker_key(),
            Discrepancy::BeliefContradicted,
            Tick(20),
            DiscrepancyClearing::BeliefUpdate {
                claim_key: clear_claim_key,
            },
        ));

        let mut other_key = blocker_key();
        other_key.goal_key = crate::GoalKey::from(GoalKind::Sleep);
        memory.record(discrepancy_entry(
            other_key,
            Discrepancy::BeliefContradicted,
            Tick(20),
            DiscrepancyClearing::BeliefUpdate {
                claim_key: retained_claim_key,
            },
        ));

        memory.clear_by_condition(|entry| {
            matches!(
                entry.clearing_condition,
                DiscrepancyClearing::BeliefUpdate { claim_key } if claim_key == clear_claim_key
            )
        });

        assert_eq!(memory.entries.len(), 1);
        assert!(memory.entries.contains_key(&other_key.into()));
    }

    #[test]
    fn discrepancy_memory_clear_by_condition_matches_commodity_availability_change() {
        let clear_place = entity_id(30, 0);
        let retained_place = entity_id(31, 0);
        let mut memory = DiscrepancyMemory::default();

        memory.record(discrepancy_entry(
            blocker_key(),
            Discrepancy::BeliefContradicted,
            Tick(20),
            DiscrepancyClearing::CommodityAvailabilityChanged {
                commodity: CommodityKind::Apple,
                place: clear_place,
            },
        ));

        let mut other_key = blocker_key();
        other_key.goal_key = crate::GoalKey::from(GoalKind::Sleep);
        memory.record(discrepancy_entry(
            other_key,
            Discrepancy::BeliefContradicted,
            Tick(20),
            DiscrepancyClearing::CommodityAvailabilityChanged {
                commodity: CommodityKind::Apple,
                place: retained_place,
            },
        ));

        memory.clear_by_condition(|entry| {
            matches!(
                entry.clearing_condition,
                DiscrepancyClearing::CommodityAvailabilityChanged { commodity, place }
                    if commodity == CommodityKind::Apple && place == clear_place
            )
        });

        assert_eq!(memory.entries.len(), 1);
        assert!(memory.entries.contains_key(&other_key.into()));
    }
}
