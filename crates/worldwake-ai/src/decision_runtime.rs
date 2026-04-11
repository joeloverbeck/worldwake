use crate::{
    DirtySet, ExhaustionBaseline, ExhaustionInvalidationCondition, GoalPriorityClass,
    HypotheticalEntityId, PlannedPlan,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use worldwake_core::{
    ActionDefId, CognitiveProfile, CommodityKind, EntityId, FrameClearReason, FrameState,
    HomeostaticNeeds, IntentionDomain, IntentionFrame, OpportunityKey, PatrolRoute, Quantity, Tick,
    UniqueItemKind, Wound,
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

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ExhaustionRetryState {
    FrontierExhausted,
    BudgetRetryPending,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ExhaustionEntry {
    pub retry_state: ExhaustionRetryState,
    pub invalidation_conditions: Vec<ExhaustionInvalidationCondition>,
    pub baseline: ExhaustionBaseline,
    #[serde(default)]
    pub next_retry_tick: Option<Tick>,
    #[serde(default)]
    pub consecutive_failures: u8,
}

impl ExhaustionEntry {
    #[must_use]
    pub fn frontier_exhausted(
        invalidation_conditions: Vec<ExhaustionInvalidationCondition>,
        baseline: ExhaustionBaseline,
    ) -> Self {
        Self {
            retry_state: ExhaustionRetryState::FrontierExhausted,
            invalidation_conditions,
            baseline,
            next_retry_tick: None,
            consecutive_failures: 0,
        }
    }

    #[must_use]
    pub fn budget_retry_pending(
        invalidation_conditions: Vec<ExhaustionInvalidationCondition>,
        baseline: ExhaustionBaseline,
        current_tick: Tick,
        cognitive: &CognitiveProfile,
    ) -> Self {
        let mut entry = Self {
            retry_state: ExhaustionRetryState::BudgetRetryPending,
            invalidation_conditions,
            baseline,
            next_retry_tick: None,
            consecutive_failures: 0,
        };
        entry.record_budget_exhaustion(current_tick, cognitive);
        entry
    }

    #[must_use]
    pub fn is_retry_eligible(&self, current_tick: Tick) -> bool {
        match self.retry_state {
            ExhaustionRetryState::FrontierExhausted => false,
            ExhaustionRetryState::BudgetRetryPending => self
                .next_retry_tick
                .is_none_or(|next_retry_tick| current_tick >= next_retry_tick),
        }
    }

    #[must_use]
    pub fn suppresses_planning(&self) -> bool {
        matches!(self.retry_state, ExhaustionRetryState::FrontierExhausted)
    }

    pub fn record_budget_exhaustion(&mut self, current_tick: Tick, cognitive: &CognitiveProfile) {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        let shift = u32::from(self.consecutive_failures.saturating_sub(1).min(6));
        let cooldown = cognitive
            .initial_cooldown_ticks
            .checked_shl(shift)
            .unwrap_or(u32::MAX)
            .min(cognitive.max_cooldown_ticks);
        self.next_retry_tick = Some(Tick(current_tick.0.saturating_add(u64::from(cooldown))));
        self.retry_state = ExhaustionRetryState::BudgetRetryPending;
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentDecisionRuntime {
    pub current_plan: Option<PlannedPlan>,
    pub current_step_index: usize,
    #[serde(default)]
    pub last_frame_clear_reason: Option<FrameClearReason>,
    pub step_in_flight: bool,
    /// Set after dead-agent cleanup has run, so subsequent ticks can skip
    /// the full `process_agent` path for dead agents.
    #[serde(default)]
    pub dead_cleanup_done: bool,
    #[serde(default)]
    pub dirty: DirtySet,
    #[serde(default)]
    pub last_priority_class: Option<GoalPriorityClass>,
    pub last_effective_place: Option<EntityId>,
    pub last_needs: Option<HomeostaticNeeds>,
    pub last_wounds: Vec<Wound>,
    pub last_commodity_signature: Vec<(CommodityKind, Quantity)>,
    pub last_unique_item_signature: Vec<(UniqueItemKind, u32)>,
    pub last_facility_access_signature: Vec<(EntityId, bool, Option<ActionDefId>)>,
    pub last_patrol_route: Option<PatrolRoute>,
    /// Whether the agent was in transit on the last observed tick.
    pub last_in_transit: bool,
    pub materialization_bindings: MaterializationBindings,
    /// Planner retry state keyed by grounded goal.
    /// `FrontierExhausted` entries suppress planning until invalidated by
    /// concrete local fact changes; `BudgetRetryPending` entries keep the goal
    /// eligible for the next compatible planning pass.
    pub exhaustion_cache: std::collections::BTreeMap<OpportunityKey, ExhaustionEntry>,
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
        AgentDecisionRuntime, ExhaustionEntry, ExhaustionRetryState, FramePlanRelation,
        MaterializationBindings, classify_frame_plan_relation, frame_runtime_snapshot,
        frame_travel_destination, has_active_frame_travel, has_frame,
    };
    use crate::{
        CommodityPurpose, DirtySet, ExhaustionBaseline, ExhaustionInvalidationCondition, GoalKey,
        GoalPriorityClass, HypotheticalEntityId, OpportunityAnchor, OpportunityKey,
        PlanTerminalKind, PlannedPlan, PlannedStep, PlannerOpKind, PlanningEntityRef,
        ProfileFixture,
    };
    use std::collections::BTreeMap;
    use worldwake_core::ActionDefId;
    use worldwake_core::{
        BodyPart, CognitiveProfile, CommodityKind, EntityId, FrameClearReason, FrameState,
        HomeostaticNeeds, IntentionDomain, IntentionFrame, PatrolRoute, Quantity, Tick,
        UniqueItemKind, Wound, WoundCause, WoundId,
    };

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

    fn opportunity(goal: GoalKey) -> OpportunityKey {
        OpportunityKey {
            goal_key: goal,
            anchor: OpportunityAnchor::None,
        }
    }

    fn sample_plan(steps: Vec<PlannedStep>) -> PlannedPlan {
        let goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        PlannedPlan::new(
            opportunity(goal),
            goal,
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

    fn cognitive(reasoning: &ProfileFixture) -> CognitiveProfile {
        CognitiveProfile {
            max_candidates_to_plan: reasoning.max_candidates_to_plan,
            max_candidates_per_expansion: CognitiveProfile::default()
                .max_candidates_per_expansion,
            max_plan_depth: reasoning.max_plan_depth,
            snapshot_travel_horizon: reasoning.snapshot_travel_horizon,
            max_node_expansions: reasoning.max_node_expansions,
            switch_margin: reasoning.switch_margin,
            planning_switch_margin: CognitiveProfile::default().planning_switch_margin,
            transient_block_ticks: reasoning.transient_block_ticks,
            unknown_block_ticks: reasoning.unknown_block_ticks,
            structural_block_ticks: reasoning.structural_block_ticks,
            initial_cooldown_ticks: reasoning.initial_cooldown_ticks,
            max_cooldown_ticks: reasoning.max_cooldown_ticks,
            max_snapshot_entities_per_place: CognitiveProfile::default()
                .max_snapshot_entities_per_place,
            speculative_acquisition: CognitiveProfile::default().speculative_acquisition,
            landmark_extraction_depth: CognitiveProfile::default().landmark_extraction_depth,
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
        assert_eq!(runtime.last_patrol_route, None);
        assert!(
            runtime
                .materialization_bindings
                .hypothetical_to_authoritative
                .is_empty()
        );
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
    fn materialization_bindings_bincode_round_trip_preserves_entries() {
        let mut bindings = MaterializationBindings::new();
        bindings.bind(HypotheticalEntityId(1), entity(2));
        bindings.bind(HypotheticalEntityId(3), entity(4));

        let encoded =
            bincode::serialize(&bindings).expect("materialization bindings should serialize");
        let decoded: MaterializationBindings =
            bincode::deserialize(&encoded).expect("materialization bindings should deserialize");

        assert_eq!(decoded, bindings);
    }

    #[test]
    fn agent_decision_runtime_bincode_round_trip_preserves_all_fields() {
        let last_needs = HomeostaticNeeds::new(
            worldwake_core::Permille::new(500).unwrap(),
            worldwake_core::Permille::new(200).unwrap(),
            worldwake_core::Permille::new(50).unwrap(),
            worldwake_core::Permille::new(300).unwrap(),
            worldwake_core::Permille::new(100).unwrap(),
        );
        let runtime = AgentDecisionRuntime {
            current_plan: Some(sample_plan(vec![
                PlannedStep {
                    targets: vec![PlanningEntityRef::Authoritative(entity(77))],
                    ..sample_step(1, PlannerOpKind::Travel)
                },
                sample_step(2, PlannerOpKind::Consume),
            ])),
            current_step_index: 1,
            last_frame_clear_reason: Some(FrameClearReason::PlanFailed),
            step_in_flight: true,
            dirty: DirtySet::NEEDS | DirtySet::POSITION,
            last_priority_class: Some(GoalPriorityClass::Critical),
            last_effective_place: Some(entity(11)),
            last_needs: Some(last_needs),
            last_wounds: vec![Wound {
                id: WoundId(9),
                body_part: BodyPart::Torso,
                cause: WoundCause::Deprivation(worldwake_core::DeprivationKind::Starvation),
                severity: worldwake_core::Permille::new(400).unwrap(),
                inflicted_at: Tick(8),
                bleed_rate_per_tick: worldwake_core::Permille::new(25).unwrap(),
            }],
            last_commodity_signature: vec![(CommodityKind::Bread, Quantity(3))],
            last_unique_item_signature: vec![(UniqueItemKind::SimpleTool, 1)],
            last_facility_access_signature: vec![(entity(12), true, Some(ActionDefId(6)))],
            last_patrol_route: Some(PatrolRoute {
                assigned_places: vec![entity(13), entity(14)],
                current_index: 1,
            }),
            last_in_transit: true,
            materialization_bindings: MaterializationBindings {
                hypothetical_to_authoritative: BTreeMap::from([
                    (HypotheticalEntityId(4), entity(5)),
                    (HypotheticalEntityId(6), entity(7)),
                ]),
            },
            dead_cleanup_done: false,
            exhaustion_cache: BTreeMap::from([(
                OpportunityKey {
                    goal_key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
                        commodity: CommodityKind::Water,
                        purpose: CommodityPurpose::SelfConsume,
                    }),
                    anchor: OpportunityAnchor::Place(entity(44)),
                },
                ExhaustionEntry {
                    retry_state: ExhaustionRetryState::BudgetRetryPending,
                    invalidation_conditions: vec![
                        ExhaustionInvalidationCondition::PositionChanged,
                        ExhaustionInvalidationCondition::CommodityChanged(CommodityKind::Water),
                    ],
                    baseline: ExhaustionBaseline {
                        position: Some(entity(11)),
                        needs: Some(last_needs),
                        commodity_quantities: vec![(CommodityKind::Water, Quantity(1))],
                        unique_item_counts: vec![(UniqueItemKind::SimpleTool, 1)],
                        steal_target_states: Vec::new(),
                        wound_count: 1,
                        hostile_count: 0,
                    },
                    next_retry_tick: Some(Tick(19)),
                    consecutive_failures: 3,
                },
            )]),
        };

        let encoded =
            bincode::serialize(&runtime).expect("agent decision runtime should serialize");
        let decoded: AgentDecisionRuntime =
            bincode::deserialize(&encoded).expect("agent decision runtime should deserialize");

        assert_eq!(decoded.current_plan, runtime.current_plan);
        assert_eq!(decoded.current_step_index, runtime.current_step_index);
        assert_eq!(decoded.step_in_flight, runtime.step_in_flight);
        assert_eq!(decoded.last_effective_place, runtime.last_effective_place);
        assert_eq!(decoded.last_needs, runtime.last_needs);
        assert_eq!(decoded.last_wounds, runtime.last_wounds);
        assert_eq!(
            decoded.last_commodity_signature,
            runtime.last_commodity_signature
        );
        assert_eq!(
            decoded.last_unique_item_signature,
            runtime.last_unique_item_signature
        );
        assert_eq!(
            decoded.last_facility_access_signature,
            runtime.last_facility_access_signature
        );
        assert_eq!(decoded.last_patrol_route, runtime.last_patrol_route);
        assert_eq!(decoded.last_in_transit, runtime.last_in_transit);
        assert_eq!(
            decoded.materialization_bindings,
            runtime.materialization_bindings
        );
        assert_eq!(decoded.exhaustion_cache, runtime.exhaustion_cache);

        // Previously-skipped fields are now serialized for lossless save/load.
        assert_eq!(
            decoded.last_frame_clear_reason,
            runtime.last_frame_clear_reason
        );
        assert_eq!(decoded.dirty, runtime.dirty);
        assert_eq!(decoded.last_priority_class, runtime.last_priority_class);
        assert_eq!(decoded.dead_cleanup_done, runtime.dead_cleanup_done);
    }

    #[test]
    fn budget_retry_pending_factory_sets_initial_cooldown_from_budget() {
        let budget = ProfileFixture {
            initial_cooldown_ticks: 10,
            max_cooldown_ticks: 100,
            ..ProfileFixture::default()
        };

        let entry = ExhaustionEntry::budget_retry_pending(
            vec![ExhaustionInvalidationCondition::PositionChanged],
            ExhaustionBaseline::default(),
            Tick(7),
            &cognitive(&budget),
        );

        assert_eq!(entry.retry_state, ExhaustionRetryState::BudgetRetryPending);
        assert_eq!(entry.consecutive_failures, 1);
        assert_eq!(entry.next_retry_tick, Some(Tick(17)));
    }

    #[test]
    fn record_budget_exhaustion_doubles_cooldown_until_cap() {
        let budget = ProfileFixture {
            initial_cooldown_ticks: 4,
            max_cooldown_ticks: 16,
            ..ProfileFixture::default()
        };
        let mut entry =
            ExhaustionEntry::frontier_exhausted(Vec::new(), ExhaustionBaseline::default());

        entry.record_budget_exhaustion(Tick(1), &cognitive(&budget));
        assert_eq!(entry.consecutive_failures, 1);
        assert_eq!(entry.next_retry_tick, Some(Tick(5)));

        entry.record_budget_exhaustion(Tick(5), &cognitive(&budget));
        assert_eq!(entry.consecutive_failures, 2);
        assert_eq!(entry.next_retry_tick, Some(Tick(13)));

        entry.record_budget_exhaustion(Tick(13), &cognitive(&budget));
        assert_eq!(entry.consecutive_failures, 3);
        assert_eq!(entry.next_retry_tick, Some(Tick(29)));

        entry.record_budget_exhaustion(Tick(29), &cognitive(&budget));
        assert_eq!(entry.consecutive_failures, 4);
        assert_eq!(entry.next_retry_tick, Some(Tick(45)));
    }

    #[test]
    fn retry_eligibility_respects_retry_tick_and_frontier_suppression() {
        let frontier =
            ExhaustionEntry::frontier_exhausted(Vec::new(), ExhaustionBaseline::default());
        let retry = ExhaustionEntry {
            retry_state: ExhaustionRetryState::BudgetRetryPending,
            invalidation_conditions: Vec::new(),
            baseline: ExhaustionBaseline::default(),
            next_retry_tick: Some(Tick(12)),
            consecutive_failures: 2,
        };
        let immediate = ExhaustionEntry {
            retry_state: ExhaustionRetryState::BudgetRetryPending,
            invalidation_conditions: Vec::new(),
            baseline: ExhaustionBaseline::default(),
            next_retry_tick: None,
            consecutive_failures: 0,
        };

        assert!(!frontier.is_retry_eligible(Tick(99)));
        assert!(!retry.is_retry_eligible(Tick(11)));
        assert!(retry.is_retry_eligible(Tick(12)));
        assert!(immediate.is_retry_eligible(Tick(0)));
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
        assert!(!has_active_frame_travel(
            Some(&frame_active),
            Some(&plan_no_travel),
            0
        ));

        // Mismatched destination
        assert!(!has_active_frame_travel(
            Some(&frame_active),
            Some(&plan_with_travel),
            0
        ));

        // Matching: travel step with correct destination
        let plan_matching = sample_plan(vec![
            PlannedStep {
                targets: vec![PlanningEntityRef::Authoritative(destination)],
                ..sample_step(1, PlannerOpKind::Travel)
            },
            sample_step(2, PlannerOpKind::Consume),
        ]);
        assert!(has_active_frame_travel(
            Some(&frame_active),
            Some(&plan_matching),
            0
        ));

        // Suspended frame
        let frame_suspended = IntentionFrame {
            state: FrameState::Suspended {
                reason: worldwake_core::SuspensionReason::PriorityInterrupt,
                suspended_at: Tick(5),
            },
            ..frame_active.clone()
        };
        assert!(!has_active_frame_travel(
            Some(&frame_suspended),
            Some(&plan_matching),
            0
        ));
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
        assert_eq!(
            snapshot.frame_state,
            Some(FrameState::Suspended {
                reason: worldwake_core::SuspensionReason::PriorityInterrupt,
                suspended_at: Tick(4),
            })
        );
        assert_eq!(snapshot.established_at, Some(Tick(3)));
        assert_eq!(snapshot.last_progress_tick, Some(Tick(8)));
        assert_eq!(snapshot.remaining_travel_steps, 1);
        assert_eq!(snapshot.stalled_ticks, 5);
        assert!(!snapshot.has_active_frame_travel);
        assert_eq!(snapshot.last_clear_reason, Some(FrameClearReason::LostPlan));
    }

    #[test]
    fn classify_frame_plan_relation_distinguishes_refresh_suspend_and_abandon() {
        let committed_goal = GoalKey::from(worldwake_core::GoalKind::Sleep);
        let committed_destination = entity(77);
        let refresh = PlannedPlan::new(
            OpportunityKey {
                goal_key: committed_goal,
                anchor: OpportunityAnchor::Place(committed_destination),
            },
            committed_goal,
            vec![PlannedStep {
                targets: vec![PlanningEntityRef::Authoritative(committed_destination)],
                ..sample_step(1, PlannerOpKind::Travel)
            }],
            PlanTerminalKind::GoalSatisfied,
        );
        let suspend_goal = GoalKey::from(worldwake_core::GoalKind::Relieve);
        let suspend = PlannedPlan::new(
            opportunity(suspend_goal),
            suspend_goal,
            vec![sample_step(2, PlannerOpKind::Relieve)],
            PlanTerminalKind::GoalSatisfied,
        );
        let abandon_goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
        });
        let abandon = PlannedPlan::new(
            OpportunityKey {
                goal_key: abandon_goal,
                anchor: OpportunityAnchor::Place(entity(88)),
            },
            abandon_goal,
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
