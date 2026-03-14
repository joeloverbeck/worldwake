use crate::{GoalKey, GoalPriorityClass, HypotheticalEntityId, PlannedPlan};
use std::collections::BTreeMap;
use worldwake_core::{
    ActionDefId, CommodityKind, EntityId, HomeostaticNeeds, Quantity, Tick, UniqueItemKind, Wound,
};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum JourneyCommitmentState {
    #[default]
    Active,
    Suspended,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum JourneyPlanRelation {
    NoCommitment,
    RefreshesCommitment,
    SuspendsCommitment,
    AbandonsCommitment,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum JourneyClearReason {
    GoalSatisfied,
    Reprioritized,
    PlanFailed,
    PatienceExhausted,
    Death,
    LostTravelPlan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JourneyRuntimeSnapshot {
    pub committed_destination: Option<EntityId>,
    pub active_plan_destination: Option<EntityId>,
    pub commitment_state: JourneyCommitmentState,
    pub established_at: Option<Tick>,
    pub last_progress_tick: Option<Tick>,
    pub remaining_travel_steps: usize,
    pub consecutive_blocked_ticks: u32,
    pub has_active_journey_travel: bool,
    pub last_clear_reason: Option<JourneyClearReason>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MaterializationBindings {
    pub hypothetical_to_authoritative: BTreeMap<HypotheticalEntityId, EntityId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QueuedFacilityIntent {
    pub goal_key: GoalKey,
    pub intended_action: ActionDefId,
}

impl MaterializationBindings {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind(&mut self, hyp: HypotheticalEntityId, auth: EntityId) {
        self.hypothetical_to_authoritative.insert(hyp, auth);
    }

    #[must_use]
    pub fn resolve(&self, hyp: HypotheticalEntityId) -> Option<EntityId> {
        self.hypothetical_to_authoritative.get(&hyp).copied()
    }

    pub fn clear(&mut self) {
        self.hypothetical_to_authoritative.clear();
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AgentDecisionRuntime {
    pub current_goal: Option<GoalKey>,
    pub current_plan: Option<PlannedPlan>,
    pub current_step_index: usize,
    pub journey_committed_goal: Option<GoalKey>,
    pub journey_committed_destination: Option<EntityId>,
    pub journey_commitment_state: JourneyCommitmentState,
    pub journey_established_at: Option<Tick>,
    pub journey_last_progress_tick: Option<Tick>,
    pub consecutive_blocked_leg_ticks: u32,
    pub last_journey_clear_reason: Option<JourneyClearReason>,
    pub step_in_flight: bool,
    pub dirty: bool,
    pub last_priority_class: Option<GoalPriorityClass>,
    pub last_effective_place: Option<EntityId>,
    pub last_needs: Option<HomeostaticNeeds>,
    pub last_wounds: Vec<Wound>,
    pub last_commodity_signature: Vec<(CommodityKind, Quantity)>,
    pub last_unique_item_signature: Vec<(UniqueItemKind, u32)>,
    pub last_facility_access_signature: Vec<(EntityId, bool, Option<ActionDefId>)>,
    pub queued_facility_intents: BTreeMap<EntityId, QueuedFacilityIntent>,
    pub materialization_bindings: MaterializationBindings,
}

impl AgentDecisionRuntime {
    #[must_use]
    pub fn has_journey_commitment(&self) -> bool {
        self.journey_committed_goal.is_some() && self.journey_committed_destination.is_some()
    }

    #[must_use]
    pub fn has_active_journey_travel(&self) -> bool {
        self.has_journey_commitment()
            && self.journey_commitment_state == JourneyCommitmentState::Active
            && self.journey_established_at.is_some()
            && self.current_plan.as_ref().is_some_and(|plan| {
                plan.has_remaining_travel_steps_from(self.current_step_index)
                    && plan.terminal_travel_destination() == self.journey_committed_destination
            })
    }

    #[must_use]
    pub fn remaining_travel_steps(&self) -> usize {
        self.current_plan.as_ref().map_or(0, |plan| {
            plan.remaining_travel_steps_from(self.current_step_index)
        })
    }

    #[must_use]
    pub fn journey_runtime_snapshot(&self) -> JourneyRuntimeSnapshot {
        JourneyRuntimeSnapshot {
            committed_destination: self.journey_committed_destination(),
            active_plan_destination: self
                .current_plan
                .as_ref()
                .and_then(PlannedPlan::terminal_travel_destination),
            commitment_state: self.journey_commitment_state,
            established_at: self.journey_established_at,
            last_progress_tick: self.journey_last_progress_tick,
            remaining_travel_steps: self.remaining_travel_steps(),
            consecutive_blocked_ticks: self.consecutive_blocked_leg_ticks,
            has_active_journey_travel: self.has_active_journey_travel(),
            last_clear_reason: self.last_journey_clear_reason,
        }
    }

    pub fn journey_committed_destination(&self) -> Option<EntityId> {
        self.has_journey_commitment()
            .then_some(self.journey_committed_destination)
            .flatten()
    }

    pub fn clear_journey_commitment(&mut self) {
        self.clear_journey_commitment_with_reason(JourneyClearReason::LostTravelPlan);
    }

    pub fn clear_journey_commitment_with_reason(&mut self, reason: JourneyClearReason) {
        let had_journey_state = self.has_journey_commitment()
            || self.journey_established_at.is_some()
            || self.journey_last_progress_tick.is_some()
            || self.consecutive_blocked_leg_ticks > 0;
        self.journey_committed_goal = None;
        self.journey_committed_destination = None;
        self.journey_commitment_state = JourneyCommitmentState::Active;
        self.journey_established_at = None;
        self.journey_last_progress_tick = None;
        self.consecutive_blocked_leg_ticks = 0;
        if had_journey_state {
            self.last_journey_clear_reason = Some(reason);
        }
    }

    #[must_use]
    pub fn classify_journey_plan_relation(&self, plan: &PlannedPlan) -> JourneyPlanRelation {
        if !self.has_journey_commitment() {
            return JourneyPlanRelation::NoCommitment;
        }

        if plan.goal == self.journey_committed_goal.unwrap_or(plan.goal)
            && plan.terminal_travel_destination() == self.journey_committed_destination
        {
            return JourneyPlanRelation::RefreshesCommitment;
        }

        if !plan.has_remaining_travel_steps_from(0) {
            return JourneyPlanRelation::SuspendsCommitment;
        }

        JourneyPlanRelation::AbandonsCommitment
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AgentDecisionRuntime, JourneyClearReason, JourneyCommitmentState, JourneyPlanRelation,
        MaterializationBindings,
    };
    use crate::{
        CommodityPurpose, GoalKey, HypotheticalEntityId, PlanTerminalKind, PlannedPlan,
        PlannedStep, PlannerOpKind, PlanningEntityRef,
    };
    use worldwake_core::ActionDefId;
    use worldwake_core::{CommodityKind, EntityId, Tick};

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn sample_step(def_id: u32, op_kind: PlannerOpKind) -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(def_id),
            targets: vec![PlanningEntityRef::Authoritative(entity(def_id + 100))],
            payload_override: None,
            op_kind,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
        }
    }

    fn sample_plan(steps: Vec<PlannedStep>) -> PlannedPlan {
        PlannedPlan::new(
            GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }),
            steps,
            PlanTerminalKind::GoalSatisfied,
        )
    }

    #[test]
    fn agent_decision_runtime_defaults_to_empty_clean_state() {
        let runtime = AgentDecisionRuntime::default();

        assert_eq!(runtime.current_goal, None);
        assert_eq!(runtime.current_plan, None);
        assert_eq!(runtime.current_step_index, 0);
        assert_eq!(runtime.journey_committed_goal, None);
        assert_eq!(runtime.journey_committed_destination, None);
        assert_eq!(
            runtime.journey_commitment_state,
            JourneyCommitmentState::Active
        );
        assert_eq!(runtime.journey_established_at, None);
        assert_eq!(runtime.journey_last_progress_tick, None);
        assert_eq!(runtime.consecutive_blocked_leg_ticks, 0);
        assert_eq!(runtime.last_journey_clear_reason, None);
        assert!(!runtime.step_in_flight);
        assert!(!runtime.dirty);
        assert_eq!(runtime.last_priority_class, None);
        assert_eq!(runtime.last_effective_place, None);
        assert_eq!(runtime.last_needs, None);
        assert!(runtime.last_wounds.is_empty());
        assert!(runtime.last_commodity_signature.is_empty());
        assert!(runtime.last_unique_item_signature.is_empty());
        assert!(runtime
            .materialization_bindings
            .hypothetical_to_authoritative
            .is_empty());
    }

    #[test]
    fn agent_decision_runtime_is_not_registered_as_a_component() {
        let component_schema = include_str!("../../worldwake-core/src/component_schema.rs");

        assert!(!component_schema.contains("AgentDecisionRuntime"));
    }

    #[test]
    fn materialization_bindings_bind_and_resolve_entries() {
        let mut bindings = MaterializationBindings::new();
        let hypothetical = HypotheticalEntityId(4);
        let authoritative = entity(9);

        bindings.bind(hypothetical, authoritative);

        assert_eq!(bindings.resolve(hypothetical), Some(authoritative));
    }

    #[test]
    fn materialization_bindings_clear_removes_all_entries() {
        let mut bindings = MaterializationBindings::new();
        bindings.bind(HypotheticalEntityId(1), entity(2));
        bindings.bind(HypotheticalEntityId(3), entity(4));

        bindings.clear();

        assert_eq!(bindings.resolve(HypotheticalEntityId(1)), None);
        assert_eq!(bindings.resolve(HypotheticalEntityId(3)), None);
        assert!(bindings.hypothetical_to_authoritative.is_empty());
    }

    #[test]
    fn has_journey_commitment_requires_goal_and_destination() {
        let goal = GoalKey::from(worldwake_core::GoalKind::Sleep);
        let destination = entity(77);
        assert!(!AgentDecisionRuntime::default().has_journey_commitment());
        assert!(!AgentDecisionRuntime {
            journey_committed_goal: Some(goal),
            ..AgentDecisionRuntime::default()
        }
        .has_journey_commitment());
        assert!(!AgentDecisionRuntime {
            journey_committed_destination: Some(destination),
            ..AgentDecisionRuntime::default()
        }
        .has_journey_commitment());
        assert!(AgentDecisionRuntime {
            journey_committed_goal: Some(goal),
            journey_committed_destination: Some(destination),
            ..AgentDecisionRuntime::default()
        }
        .has_journey_commitment());
    }

    #[test]
    fn has_active_journey_travel_requires_commitment_and_matching_travel_steps() {
        let destination = entity(77);
        let no_commitment = AgentDecisionRuntime {
            current_plan: Some(sample_plan(vec![sample_step(1, PlannerOpKind::Travel)])),
            ..AgentDecisionRuntime::default()
        };
        assert!(!no_commitment.has_active_journey_travel());

        let no_remaining_travel = AgentDecisionRuntime {
            current_plan: Some(sample_plan(vec![sample_step(1, PlannerOpKind::Consume)])),
            journey_committed_goal: Some(GoalKey::from(worldwake_core::GoalKind::Sleep)),
            journey_committed_destination: Some(destination),
            journey_established_at: Some(Tick(7)),
            ..AgentDecisionRuntime::default()
        };
        assert!(!no_remaining_travel.has_active_journey_travel());

        let mismatched_destination = AgentDecisionRuntime {
            current_plan: Some(sample_plan(vec![sample_step(1, PlannerOpKind::Travel)])),
            journey_committed_goal: Some(GoalKey::from(worldwake_core::GoalKind::Sleep)),
            journey_committed_destination: Some(destination),
            journey_established_at: Some(Tick(7)),
            current_step_index: 0,
            ..AgentDecisionRuntime::default()
        };
        assert!(!mismatched_destination.has_active_journey_travel());

        let current_travel_step_counts = AgentDecisionRuntime {
            current_plan: Some(sample_plan(vec![
                PlannedStep {
                    targets: vec![PlanningEntityRef::Authoritative(destination)],
                    ..sample_step(1, PlannerOpKind::Travel)
                },
                sample_step(2, PlannerOpKind::Consume),
            ])),
            journey_committed_goal: Some(GoalKey::from(worldwake_core::GoalKind::Sleep)),
            journey_committed_destination: Some(destination),
            journey_established_at: Some(Tick(7)),
            current_step_index: 0,
            ..AgentDecisionRuntime::default()
        };
        assert!(current_travel_step_counts.has_active_journey_travel());

        let suspended_commitment = AgentDecisionRuntime {
            journey_commitment_state: JourneyCommitmentState::Suspended,
            ..current_travel_step_counts.clone()
        };
        assert!(!suspended_commitment.has_active_journey_travel());
    }

    #[test]
    fn remaining_travel_steps_counts_from_current_index() {
        let runtime = AgentDecisionRuntime {
            current_plan: Some(sample_plan(vec![
                sample_step(1, PlannerOpKind::Travel),
                sample_step(2, PlannerOpKind::Consume),
                sample_step(3, PlannerOpKind::Travel),
                sample_step(4, PlannerOpKind::Travel),
            ])),
            current_step_index: 2,
            ..AgentDecisionRuntime::default()
        };

        assert_eq!(runtime.remaining_travel_steps(), 2);

        let beyond_end = AgentDecisionRuntime {
            current_plan: runtime.current_plan.clone(),
            current_step_index: 10,
            ..AgentDecisionRuntime::default()
        };
        assert_eq!(beyond_end.remaining_travel_steps(), 0);
        assert_eq!(AgentDecisionRuntime::default().remaining_travel_steps(), 0);
    }

    #[test]
    fn journey_committed_destination_requires_full_commitment() {
        let goal = GoalKey::from(worldwake_core::GoalKind::Sleep);
        let destination = entity(55);
        assert_eq!(
            AgentDecisionRuntime::default().journey_committed_destination(),
            None
        );
        assert_eq!(
            AgentDecisionRuntime {
                journey_committed_goal: Some(goal),
                journey_committed_destination: Some(destination),
                ..AgentDecisionRuntime::default()
            }
            .journey_committed_destination(),
            Some(destination)
        );
    }

    #[test]
    fn clear_journey_commitment_resets_anchor_and_temporal_state() {
        let mut runtime = AgentDecisionRuntime {
            journey_committed_goal: Some(GoalKey::from(worldwake_core::GoalKind::Sleep)),
            journey_committed_destination: Some(entity(77)),
            journey_commitment_state: JourneyCommitmentState::Suspended,
            journey_established_at: Some(Tick(3)),
            journey_last_progress_tick: Some(Tick(8)),
            consecutive_blocked_leg_ticks: 5,
            last_journey_clear_reason: Some(JourneyClearReason::Reprioritized),
            ..AgentDecisionRuntime::default()
        };

        runtime.clear_journey_commitment_with_reason(JourneyClearReason::PlanFailed);

        assert_eq!(runtime.journey_committed_goal, None);
        assert_eq!(runtime.journey_committed_destination, None);
        assert_eq!(
            runtime.journey_commitment_state,
            JourneyCommitmentState::Active
        );
        assert_eq!(runtime.journey_established_at, None);
        assert_eq!(runtime.journey_last_progress_tick, None);
        assert_eq!(runtime.consecutive_blocked_leg_ticks, 0);
        assert_eq!(
            runtime.last_journey_clear_reason,
            Some(JourneyClearReason::PlanFailed)
        );
    }

    #[test]
    fn journey_runtime_snapshot_reflects_anchor_plan_and_temporal_fields() {
        let committed_destination = entity(55);
        let active_plan_destination = entity(77);
        let runtime = AgentDecisionRuntime {
            current_plan: Some(sample_plan(vec![
                PlannedStep {
                    targets: vec![PlanningEntityRef::Authoritative(entity(12))],
                    ..sample_step(1, PlannerOpKind::Travel)
                },
                PlannedStep {
                    targets: vec![PlanningEntityRef::Authoritative(active_plan_destination)],
                    ..sample_step(2, PlannerOpKind::Travel)
                },
            ])),
            current_step_index: 1,
            journey_committed_goal: Some(GoalKey::from(worldwake_core::GoalKind::Sleep)),
            journey_committed_destination: Some(committed_destination),
            journey_commitment_state: JourneyCommitmentState::Suspended,
            journey_established_at: Some(Tick(3)),
            journey_last_progress_tick: Some(Tick(8)),
            consecutive_blocked_leg_ticks: 5,
            last_journey_clear_reason: Some(JourneyClearReason::LostTravelPlan),
            ..AgentDecisionRuntime::default()
        };

        let snapshot = runtime.journey_runtime_snapshot();

        assert_eq!(snapshot.committed_destination, Some(committed_destination));
        assert_eq!(
            snapshot.active_plan_destination,
            Some(active_plan_destination)
        );
        assert_eq!(snapshot.commitment_state, JourneyCommitmentState::Suspended);
        assert_eq!(snapshot.established_at, Some(Tick(3)));
        assert_eq!(snapshot.last_progress_tick, Some(Tick(8)));
        assert_eq!(snapshot.remaining_travel_steps, 1);
        assert_eq!(snapshot.consecutive_blocked_ticks, 5);
        assert!(!snapshot.has_active_journey_travel);
        assert_eq!(
            snapshot.last_clear_reason,
            Some(JourneyClearReason::LostTravelPlan)
        );
    }

    #[test]
    fn classify_journey_plan_relation_distinguishes_refresh_suspend_and_abandon() {
        let committed_goal = GoalKey::from(worldwake_core::GoalKind::Sleep);
        let committed_destination = entity(77);
        let refresh = PlannedPlan::new(
            committed_goal,
            vec![PlannedStep {
                targets: vec![PlanningEntityRef::Authoritative(committed_destination)],
                ..sample_step(1, PlannerOpKind::Travel)
            }],
            PlanTerminalKind::GoalSatisfied,
        );
        let suspend = PlannedPlan::new(
            GoalKey::from(worldwake_core::GoalKind::Relieve),
            vec![sample_step(2, PlannerOpKind::Relieve)],
            PlanTerminalKind::GoalSatisfied,
        );
        let abandon = PlannedPlan::new(
            GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: CommodityPurpose::SelfConsume,
            }),
            vec![PlannedStep {
                targets: vec![PlanningEntityRef::Authoritative(entity(88))],
                ..sample_step(3, PlannerOpKind::Travel)
            }],
            PlanTerminalKind::GoalSatisfied,
        );
        let runtime = AgentDecisionRuntime {
            journey_committed_goal: Some(committed_goal),
            journey_committed_destination: Some(committed_destination),
            journey_established_at: Some(Tick(3)),
            ..AgentDecisionRuntime::default()
        };

        assert_eq!(
            AgentDecisionRuntime::default().classify_journey_plan_relation(&refresh),
            JourneyPlanRelation::NoCommitment
        );
        assert_eq!(
            runtime.classify_journey_plan_relation(&refresh),
            JourneyPlanRelation::RefreshesCommitment
        );
        assert_eq!(
            runtime.classify_journey_plan_relation(&suspend),
            JourneyPlanRelation::SuspendsCommitment
        );
        assert_eq!(
            runtime.classify_journey_plan_relation(&abandon),
            JourneyPlanRelation::AbandonsCommitment
        );
    }
}
