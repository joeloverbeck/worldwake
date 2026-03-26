use crate::{DirtySet, GoalKey, GoalPriorityClass, HypotheticalEntityId, PlannedPlan};
use std::collections::BTreeMap;
use worldwake_core::{
    ActionDefId, CommodityKind, EntityId, FrameClearReason, FrameState, HomeostaticNeeds,
    IntentionDomain, IntentionFrame, Quantity, Tick, UniqueItemKind, Wound,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FramePlanRelation {
    NoFrame,
    RefreshesFrame,
    SuspendsFrame,
    AbandonsFrame,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameRuntimeSnapshot {
    pub committed_destination: Option<EntityId>,
    pub active_plan_destination: Option<EntityId>,
    pub frame_state: Option<FrameState>,
    pub established_at: Option<Tick>,
    pub last_progress_tick: Option<Tick>,
    pub remaining_travel_steps: usize,
    pub stalled_ticks: u32,
    pub has_active_frame_travel: bool,
    pub last_clear_reason: Option<FrameClearReason>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MaterializationBindings {
    pub hypothetical_to_authoritative: BTreeMap<HypotheticalEntityId, EntityId>,
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
    pub current_plan: Option<PlannedPlan>,
    pub current_step_index: usize,
    pub last_frame_clear_reason: Option<FrameClearReason>,
    pub step_in_flight: bool,
    pub dirty: DirtySet,
    pub last_priority_class: Option<GoalPriorityClass>,
    pub last_effective_place: Option<EntityId>,
    pub last_needs: Option<HomeostaticNeeds>,
    pub last_wounds: Vec<Wound>,
    pub last_commodity_signature: Vec<(CommodityKind, Quantity)>,
    pub last_unique_item_signature: Vec<(UniqueItemKind, u32)>,
    pub last_facility_access_signature: Vec<(EntityId, bool, Option<ActionDefId>)>,
    pub materialization_bindings: MaterializationBindings,
    /// Goals whose plan search exhausted the budget on the previous planning
    /// cycle.  These are skipped on the next cycle unless significant world
    /// changes occur (position, commodity, wounds, or facility changes).
    pub search_exhausted_goals: std::collections::BTreeSet<GoalKey>,
}

impl AgentDecisionRuntime {
    #[must_use]
    pub fn remaining_travel_steps(&self) -> usize {
        self.current_plan.as_ref().map_or(0, |plan| {
            plan.remaining_travel_steps_from(self.current_step_index)
        })
    }
}

// ── Free functions operating on IntentionFrame component ──

/// Returns `true` if the agent has an intention frame (component present).
#[must_use]
pub fn has_frame(frame: Option<&IntentionFrame>) -> bool {
    frame.is_some()
}

/// Returns the committed travel destination if the frame is a Travel domain frame.
#[must_use]
pub fn frame_travel_destination(frame: Option<&IntentionFrame>) -> Option<EntityId> {
    frame.and_then(|f| match f.domain {
        IntentionDomain::Travel { destination } => Some(destination),
        _ => None,
    })
}

/// Returns `true` if the agent has an active travel frame with remaining travel
/// steps matching the frame's destination.
#[must_use]
pub fn has_active_frame_travel(
    frame: Option<&IntentionFrame>,
    plan: Option<&PlannedPlan>,
    step_index: usize,
) -> bool {
    let Some(f) = frame else {
        return false;
    };
    if f.state != FrameState::Active {
        return false;
    }
    let Some(destination) = (match f.domain {
        IntentionDomain::Travel { destination } => Some(destination),
        _ => None,
    }) else {
        return false;
    };
    plan.is_some_and(|plan| {
        plan.has_remaining_travel_steps_from(step_index)
            && plan.terminal_travel_destination() == Some(destination)
    })
}

/// Builds a snapshot of frame-related runtime state for diagnostic/debug use.
#[must_use]
pub fn frame_runtime_snapshot(
    frame: Option<&IntentionFrame>,
    runtime: &AgentDecisionRuntime,
) -> FrameRuntimeSnapshot {
    FrameRuntimeSnapshot {
        committed_destination: frame_travel_destination(frame),
        active_plan_destination: runtime
            .current_plan
            .as_ref()
            .and_then(PlannedPlan::terminal_travel_destination),
        frame_state: frame.map(|f| f.state),
        established_at: frame.map(|f| f.established_at),
        last_progress_tick: frame.and_then(|f| f.last_progress_tick),
        remaining_travel_steps: runtime.remaining_travel_steps(),
        stalled_ticks: frame.map_or(0, |f| f.stalled_ticks),
        has_active_frame_travel: has_active_frame_travel(
            frame,
            runtime.current_plan.as_ref(),
            runtime.current_step_index,
        ),
        last_clear_reason: runtime.last_frame_clear_reason,
    }
}

/// Classifies how a proposed plan relates to the current intention frame.
#[must_use]
pub fn classify_frame_plan_relation(
    frame: Option<&IntentionFrame>,
    plan: &PlannedPlan,
) -> FramePlanRelation {
    let Some(f) = frame else {
        return FramePlanRelation::NoFrame;
    };

    let frame_destination = match f.domain {
        IntentionDomain::Travel { destination } => Some(destination),
        _ => None,
    };

    if plan.goal == f.goal && plan.terminal_travel_destination() == frame_destination {
        return FramePlanRelation::RefreshesFrame;
    }

    if !plan.has_remaining_travel_steps_from(0) {
        return FramePlanRelation::SuspendsFrame;
    }

    FramePlanRelation::AbandonsFrame
}

#[cfg(test)]
mod tests {
    use super::{
        classify_frame_plan_relation, has_active_frame_travel, has_frame,
        frame_travel_destination, frame_runtime_snapshot, AgentDecisionRuntime,
        FramePlanRelation, MaterializationBindings,
    };
    use worldwake_core::{
        FrameClearReason, FrameState, IntentionDomain, IntentionFrame,
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

    fn sample_travel_frame(goal: GoalKey, destination: EntityId) -> IntentionFrame {
        IntentionFrame {
            goal,
            domain: IntentionDomain::Travel { destination },
            assumptions: Vec::new(),
            state: FrameState::Active,
            established_at: Tick(3),
            last_progress_tick: None,
            stalled_ticks: 0,
            patience_limit: 30,
        }
    }

    #[test]
    fn agent_decision_runtime_defaults_to_empty_clean_state() {
        let runtime = AgentDecisionRuntime::default();

        assert_eq!(runtime.current_plan, None);
        assert_eq!(runtime.current_step_index, 0);
        assert_eq!(runtime.last_frame_clear_reason, None);
        assert!(!runtime.step_in_flight);
        assert!(runtime.dirty.is_empty());
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
    fn has_frame_returns_true_when_component_present() {
        let goal = GoalKey::from(worldwake_core::GoalKind::Sleep);
        let destination = entity(77);
        assert!(!has_frame(None));
        assert!(has_frame(Some(&sample_travel_frame(goal, destination))));
    }

    #[test]
    fn has_active_frame_travel_requires_frame_and_matching_travel_steps() {
        let destination = entity(77);
        let goal = GoalKey::from(worldwake_core::GoalKind::Sleep);
        let frame_active = sample_travel_frame(goal, destination);

        // No frame
        let plan_with_travel = sample_plan(vec![sample_step(1, PlannerOpKind::Travel)]);
        assert!(!has_active_frame_travel(None, Some(&plan_with_travel), 0));

        // No remaining travel
        let plan_no_travel = sample_plan(vec![sample_step(1, PlannerOpKind::Consume)]);
        assert!(!has_active_frame_travel(Some(&frame_active), Some(&plan_no_travel), 0));

        // Mismatched destination
        assert!(!has_active_frame_travel(Some(&frame_active), Some(&plan_with_travel), 0));

        // Matching: travel step with correct destination
        let plan_matching = sample_plan(vec![
            PlannedStep {
                targets: vec![PlanningEntityRef::Authoritative(destination)],
                ..sample_step(1, PlannerOpKind::Travel)
            },
            sample_step(2, PlannerOpKind::Consume),
        ]);
        assert!(has_active_frame_travel(Some(&frame_active), Some(&plan_matching), 0));

        // Suspended frame
        let frame_suspended = IntentionFrame {
            state: FrameState::Suspended {
                reason: worldwake_core::SuspensionReason::PriorityInterrupt,
                suspended_at: Tick(5),
            },
            ..frame_active.clone()
        };
        assert!(!has_active_frame_travel(Some(&frame_suspended), Some(&plan_matching), 0));
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
    fn frame_travel_destination_returns_destination_when_present() {
        let goal = GoalKey::from(worldwake_core::GoalKind::Sleep);
        let destination = entity(55);
        assert_eq!(frame_travel_destination(None), None);
        assert_eq!(
            frame_travel_destination(Some(&sample_travel_frame(goal, destination))),
            Some(destination)
        );
    }

    #[test]
    fn clearing_frame_sets_option_to_none_and_records_reason_on_runtime() {
        let goal = GoalKey::from(worldwake_core::GoalKind::Sleep);
        let mut frame: Option<IntentionFrame> = Some(IntentionFrame {
            state: FrameState::Suspended {
                reason: worldwake_core::SuspensionReason::PriorityInterrupt,
                suspended_at: Tick(4),
            },
            last_progress_tick: Some(Tick(8)),
            stalled_ticks: 5,
            ..sample_travel_frame(goal, entity(77))
        });
        let mut runtime = AgentDecisionRuntime {
            last_frame_clear_reason: Some(FrameClearReason::Reprioritized),
            ..AgentDecisionRuntime::default()
        };

        // Simulate clearing: set frame to None and record reason on runtime.
        if frame.is_some() {
            runtime.last_frame_clear_reason = Some(FrameClearReason::PlanFailed);
        }
        frame = None;

        assert!(frame.is_none());
        assert_eq!(
            runtime.last_frame_clear_reason,
            Some(FrameClearReason::PlanFailed)
        );
    }

    #[test]
    fn frame_runtime_snapshot_reflects_anchor_plan_and_temporal_fields() {
        let committed_destination = entity(55);
        let active_plan_destination = entity(77);
        let goal = GoalKey::from(worldwake_core::GoalKind::Sleep);
        let frame = IntentionFrame {
            state: FrameState::Suspended {
                reason: worldwake_core::SuspensionReason::PriorityInterrupt,
                suspended_at: Tick(4),
            },
            last_progress_tick: Some(Tick(8)),
            stalled_ticks: 5,
            ..sample_travel_frame(goal, committed_destination)
        };
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
            last_frame_clear_reason: Some(FrameClearReason::LostPlan),
            ..AgentDecisionRuntime::default()
        };

        let snapshot = frame_runtime_snapshot(Some(&frame), &runtime);

        assert_eq!(snapshot.committed_destination, Some(committed_destination));
        assert_eq!(
            snapshot.active_plan_destination,
            Some(active_plan_destination)
        );
        assert_eq!(snapshot.frame_state, Some(FrameState::Suspended {
            reason: worldwake_core::SuspensionReason::PriorityInterrupt,
            suspended_at: Tick(4),
        }));
        assert_eq!(snapshot.established_at, Some(Tick(3)));
        assert_eq!(snapshot.last_progress_tick, Some(Tick(8)));
        assert_eq!(snapshot.remaining_travel_steps, 1);
        assert_eq!(snapshot.stalled_ticks, 5);
        assert!(!snapshot.has_active_frame_travel);
        assert_eq!(
            snapshot.last_clear_reason,
            Some(FrameClearReason::LostPlan)
        );
    }

    #[test]
    fn classify_frame_plan_relation_distinguishes_refresh_suspend_and_abandon() {
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
        let frame = sample_travel_frame(committed_goal, committed_destination);

        assert_eq!(
            classify_frame_plan_relation(None, &refresh),
            FramePlanRelation::NoFrame
        );
        assert_eq!(
            classify_frame_plan_relation(Some(&frame), &refresh),
            FramePlanRelation::RefreshesFrame
        );
        assert_eq!(
            classify_frame_plan_relation(Some(&frame), &suspend),
            FramePlanRelation::SuspendsFrame
        );
        assert_eq!(
            classify_frame_plan_relation(Some(&frame), &abandon),
            FramePlanRelation::AbandonsFrame
        );
    }
}
