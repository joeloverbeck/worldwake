use crate::{
    authoritative_target, classify_frame_plan_relation, has_active_frame_travel,
    AgentDecisionRuntime, FrameRuntimeSnapshot, PlannerOpKind, PlannedStep, PlanningBudget,
};
use crate::{GoalPriorityClass, RankedGoal};
use worldwake_core::{
    BlockedIntent, BlockedIntentMemory, BlockerKey, BlockingFact, EntityId, FrameAssumption,
    FrameClearReason, FrameState, IntentionDomain, IntentionFrame, Permille, SuspensionReason,
    Tick,
};
use worldwake_sim::RuntimeBeliefView;

/// All `PlannerOpKind` variants — used for `IntentionDomain::Generic` where
/// every completed action counts as progress.
static GENERIC_PROGRESS_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Consume,
    PlannerOpKind::Sleep,
    PlannerOpKind::Relieve,
    PlannerOpKind::Wash,
    PlannerOpKind::Trade,
    PlannerOpKind::QueueForFacilityUse,
    PlannerOpKind::Harvest,
    PlannerOpKind::Craft,
    PlannerOpKind::MoveCargo,
    PlannerOpKind::Heal,
    PlannerOpKind::Loot,
    PlannerOpKind::Bury,
    PlannerOpKind::Tell,
    PlannerOpKind::ConsultRecord,
    PlannerOpKind::Attack,
    PlannerOpKind::Defend,
    PlannerOpKind::Bribe,
    PlannerOpKind::Threaten,
    PlannerOpKind::DeclareSupport,
    PlannerOpKind::PressForceClaim,
    PlannerOpKind::YieldForceClaim,
];

/// Returns the set of `PlannerOpKind`s that count as forward progress for a
/// given intention domain. Only step completions matching one of these op
/// kinds will reset `stalled_ticks` and update `last_progress_tick`.
#[allow(clippy::match_same_arms)] // Travel and Escort intentionally listed separately for domain clarity.
pub(super) fn progress_op_kinds(domain: &IntentionDomain) -> &'static [PlannerOpKind] {
    match domain {
        IntentionDomain::Travel { .. } => &[PlannerOpKind::Travel],
        IntentionDomain::Care { .. } => &[PlannerOpKind::Heal, PlannerOpKind::Travel],
        IntentionDomain::Escort { .. } => &[PlannerOpKind::Travel],
        IntentionDomain::Errand { .. } => &[
            PlannerOpKind::Travel,
            PlannerOpKind::DeclareSupport,
            PlannerOpKind::PressForceClaim,
            PlannerOpKind::YieldForceClaim,
        ],
        IntentionDomain::Generic => GENERIC_PROGRESS_OPS,
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FrameSwitchMarginSource {
    BudgetDefault,
    FrameProfile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameDebugSnapshot {
    pub runtime: FrameRuntimeSnapshot,
    pub effective_switch_margin: Permille,
    pub switch_margin_source: FrameSwitchMarginSource,
}

/// Updates the intention frame for a newly adopted plan.
///
/// Returns the updated frame (or `None` if it was cleared).
pub(super) fn update_frame_for_adopted_plan(
    frame: Option<&IntentionFrame>,
    selected_plan: &crate::PlannedPlan,
    tick: Tick,
    runtime: &mut AgentDecisionRuntime,
) -> Option<IntentionFrame> {
    let relation = classify_frame_plan_relation(frame, selected_plan);

    if relation == crate::FramePlanRelation::SuspendsFrame {
        return frame.map(|f| IntentionFrame {
            state: FrameState::Suspended {
                reason: SuspensionReason::PriorityInterrupt,
                suspended_at: tick,
            },
            ..f.clone()
        });
    }

    let Some(destination) = selected_plan.terminal_travel_destination() else {
        if frame.is_some() {
            runtime.last_frame_clear_reason = Some(FrameClearReason::LostPlan);
        }
        return None;
    };

    let same_frame = relation == crate::FramePlanRelation::RefreshesFrame;

    if same_frame {
        if let Some(existing) = frame {
            return Some(IntentionFrame {
                goal: selected_plan.goal,
                domain: IntentionDomain::Travel { destination },
                state: FrameState::Active,
                ..existing.clone()
            });
        }
    }

    Some(IntentionFrame {
        goal: selected_plan.goal,
        domain: IntentionDomain::Travel { destination },
        assumptions: Vec::new(),
        state: FrameState::Active,
        established_at: tick,
        last_progress_tick: None,
        stalled_ticks: 0,
        patience_limit: 30, // default; caller may override from profile
    })
}

/// Handles a blocked travel step during an active frame. Returns `true`
/// if the blockage was handled (caller should not fall through to generic
/// failure handling).
///
/// Returns `(handled, updated_frame)`. When `handled` is true, the
/// caller should use `updated_frame` as the new frame state.
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_recoverable_travel_step_blockage(
    view: &dyn RuntimeBeliefView,
    frame: Option<&IntentionFrame>,
    runtime: &mut AgentDecisionRuntime,
    active_goal: Option<worldwake_core::GoalKey>,
    blocked_memory: &mut BlockedIntentMemory,
    agent: EntityId,
    step: &PlannedStep,
    tick: Tick,
    budget: &PlanningBudget,
) -> (bool, Option<IntentionFrame>) {
    if step.op_kind != crate::PlannerOpKind::Travel
        || !has_active_frame_travel(
            frame,
            runtime.current_plan.as_ref(),
            runtime.current_step_index,
        )
    {
        return (false, frame.cloned());
    }

    let f = frame.expect("active frame travel requires a frame");
    let new_stalled = f
        .stalled_ticks
        .checked_add(1)
        .expect("stalled ticks overflowed");

    let patience_exhausted = view
        .intention_disposition_profile(agent)
        .is_some_and(|profile| {
            new_stalled >= profile.patience_for(f.domain.domain_tag())
        });

    let updated_frame = if patience_exhausted {
        let goal_key = active_goal.unwrap_or_else(|| {
            runtime
                .current_plan
                .as_ref()
                .map(|plan| plan.goal)
                .expect("active frame travel must retain a current goal")
        });
        blocked_memory.record(BlockedIntent {
            blocker_key: BlockerKey {
                goal_key,
                place: blocked_leg_target(step),
                target: None,
                action_def: Some(step.def_id),
            },
            blocking_fact: worldwake_core::BlockingFact::NoKnownPath,
            diagnostic_context: None,
            observed_tick: tick,
            expires_tick: tick + u64::from(budget.structural_block_ticks),
        });
        runtime.last_frame_clear_reason = Some(FrameClearReason::PatienceExhausted);
        None
    } else {
        Some(IntentionFrame {
            stalled_ticks: new_stalled,
            ..f.clone()
        })
    };

    runtime.current_plan = None;
    runtime.current_step_index = 0;
    runtime.materialization_bindings.clear();
    runtime.dirty = true;
    (true, updated_frame)
}

/// Result of evaluating a frame's assumptions against the agent's beliefs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum AssumptionEvalResult {
    /// All assumptions hold.
    AllPass,
    /// A recoverable assumption failed — frame should be suspended.
    RecoverableFailure(SuspensionReason),
    /// A critical assumption failed — frame should be exhausted.
    CriticalFailure,
    /// Evaluation was deferred (contains `NoCriticalThreat` that needs ranked candidates).
    Deferred,
}

/// Populate assumptions for an intention frame based on its domain and the
/// agent's current believed position. This is a standalone function (not a
/// method on `IntentionFrame`) to keep `worldwake-core` free of `BeliefView`
/// dependencies.
pub(super) fn populate_assumptions(
    domain: &IntentionDomain,
    agent: EntityId,
    view: &dyn RuntimeBeliefView,
) -> Vec<FrameAssumption> {
    let current_place = view.effective_place(agent);
    match *domain {
        IntentionDomain::Travel { destination } | IntentionDomain::Errand { destination } => {
            let mut assumptions = Vec::with_capacity(1);
            if let Some(from) = current_place {
                assumptions.push(FrameAssumption::RouteExists {
                    from,
                    to: destination,
                });
            }
            assumptions
        }
        IntentionDomain::Care { patient } => {
            let mut assumptions = Vec::with_capacity(2);
            assumptions.push(FrameAssumption::TargetAlive(patient));
            if let Some(from) = current_place {
                if let Some(patient_place) = view.effective_place(patient) {
                    assumptions.push(FrameAssumption::RouteExists {
                        from,
                        to: patient_place,
                    });
                }
            }
            assumptions
        }
        IntentionDomain::Escort { ward, destination } => {
            let mut assumptions = Vec::with_capacity(2);
            assumptions.push(FrameAssumption::TargetAlive(ward));
            if let Some(from) = current_place {
                assumptions.push(FrameAssumption::RouteExists {
                    from,
                    to: destination,
                });
            }
            assumptions
        }
        IntentionDomain::Generic => {
            vec![FrameAssumption::NoCriticalThreat]
        }
    }
}

/// Evaluate a set of assumptions against the agent's beliefs. Returns the
/// evaluation result.
///
/// `NoCriticalThreat` requires ranked candidates to check for
/// `GoalPriorityClass::Critical`. If `ranked_candidates` is `None`,
/// `NoCriticalThreat` is skipped (returns `Deferred` if it was the only
/// assumption that could fail). If `Some`, it is evaluated immediately.
///
/// `CommodityAvailableAt` is stubbed as always-true (future work).
pub(super) fn evaluate_assumptions(
    assumptions: &[FrameAssumption],
    view: &dyn RuntimeBeliefView,
    ranked_candidates: Option<&[RankedGoal]>,
) -> AssumptionEvalResult {
    let mut has_deferred = false;

    for assumption in assumptions {
        match *assumption {
            FrameAssumption::TargetAlive(entity) => {
                if !view.is_alive(entity) {
                    return AssumptionEvalResult::CriticalFailure;
                }
            }
            FrameAssumption::RouteExists { from, to } => {
                if !view.route_exists(from, to) {
                    return AssumptionEvalResult::RecoverableFailure(SuspensionReason::RouteBlocked);
                }
            }
            FrameAssumption::NoCriticalThreat => {
                if let Some(candidates) = ranked_candidates {
                    let has_critical = candidates
                        .iter()
                        .any(|c| c.priority_class == GoalPriorityClass::Critical);
                    if has_critical {
                        return AssumptionEvalResult::RecoverableFailure(
                            SuspensionReason::SurvivalNeed,
                        );
                    }
                } else {
                    has_deferred = true;
                }
            }
            FrameAssumption::CommodityAvailableAt { .. } => {
                // Stubbed as always-true — future work.
            }
        }
    }

    if has_deferred {
        AssumptionEvalResult::Deferred
    } else {
        AssumptionEvalResult::AllPass
    }
}

/// Apply assumption evaluation result to a frame, returning the updated frame.
pub(super) fn apply_assumption_result(
    frame: &IntentionFrame,
    result: &AssumptionEvalResult,
    tick: Tick,
    runtime: &mut AgentDecisionRuntime,
) -> IntentionFrame {
    match result {
        AssumptionEvalResult::CriticalFailure => {
            runtime.last_frame_clear_reason = Some(FrameClearReason::AssumptionFailed);
            IntentionFrame {
                state: FrameState::Exhausted,
                ..frame.clone()
            }
        }
        AssumptionEvalResult::RecoverableFailure(reason) => IntentionFrame {
            state: FrameState::Suspended {
                reason: *reason,
                suspended_at: tick,
            },
            ..frame.clone()
        },
        AssumptionEvalResult::AllPass => {
            // If was suspended, resume to Active (do NOT reset stalled_ticks per spec).
            if matches!(frame.state, FrameState::Suspended { .. }) {
                IntentionFrame {
                    state: FrameState::Active,
                    ..frame.clone()
                }
            } else {
                frame.clone()
            }
        }
        AssumptionEvalResult::Deferred => frame.clone(),
    }
}

/// Extract the domain-specific target entity for a `BlockerKey` from an
/// `IntentionDomain`. Used when creating `BlockedIntent`s on frame exhaustion
/// (patience or assumption failure).
pub(super) fn frame_blocker_target(domain: &IntentionDomain) -> Option<EntityId> {
    match *domain {
        IntentionDomain::Travel { destination } | IntentionDomain::Errand { destination } => {
            Some(destination)
        }
        IntentionDomain::Care { patient } => Some(patient),
        IntentionDomain::Escort { ward, .. } => Some(ward),
        IntentionDomain::Generic => None,
    }
}

/// Check whether a frame's `stalled_ticks` has reached `patience_limit` after an
/// increment. If so, record a `BlockedIntent` with `PatienceExhausted`,
/// transition the frame to `Exhausted`, clear the plan, and return `true`.
///
/// The caller must have already incremented `frame.stalled_ticks`. This
/// function only handles the creation of the blocked intent and state
/// transition when the threshold is met.
pub(super) fn check_patience_exhaustion(
    frame: &IntentionFrame,
    agent_place: Option<EntityId>,
    blocked_memory: &mut BlockedIntentMemory,
    runtime: &mut AgentDecisionRuntime,
    tick: Tick,
    structural_block_ticks: u32,
) -> bool {
    if frame.stalled_ticks < frame.patience_limit {
        return false;
    }
    blocked_memory.record(BlockedIntent {
        blocker_key: BlockerKey {
            goal_key: frame.goal,
            place: agent_place,
            target: frame_blocker_target(&frame.domain),
            action_def: None,
        },
        blocking_fact: BlockingFact::PatienceExhausted,
        diagnostic_context: None,
        observed_tick: tick,
        expires_tick: tick + u64::from(structural_block_ticks),
    });
    runtime.last_frame_clear_reason = Some(FrameClearReason::PatienceExhausted);
    runtime.current_plan = None;
    runtime.current_step_index = 0;
    runtime.materialization_bindings.clear();
    runtime.dirty = true;
    true
}

/// Record a `BlockedIntent` with `AssumptionFailed` for a frame whose critical
/// assumption has failed.
pub(super) fn record_assumption_failure_blocked_intent(
    frame: &IntentionFrame,
    agent_place: Option<EntityId>,
    blocked_memory: &mut BlockedIntentMemory,
    tick: Tick,
    structural_block_ticks: u32,
) {
    blocked_memory.record(BlockedIntent {
        blocker_key: BlockerKey {
            goal_key: frame.goal,
            place: agent_place,
            target: frame_blocker_target(&frame.domain),
            action_def: None,
        },
        blocking_fact: BlockingFact::AssumptionFailed,
        diagnostic_context: None,
        observed_tick: tick,
        expires_tick: tick + u64::from(structural_block_ticks),
    });
}

fn blocked_leg_target(step: &PlannedStep) -> Option<EntityId> {
    step.targets.first().copied().and_then(authoritative_target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GoalPriorityClass, GroundedGoal, RankedGoal};
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        BeliefConfidencePolicy, CombatProfile, CommodityConsumableProfile, CommodityKind,
        DemandObservation, DriveThresholds, EntityKind, GoalKey, GoalKind, HomeostaticNeeds,
        InTransitOnEdge, IntentionDispositionProfile, LoadUnits, MerchandiseProfile,
        MetabolismProfile, Quantity, RecipeId, ResourceSource, Tick, TickRange,
        TradeDispositionProfile, UniqueItemKind, WorkstationTag, Wound,
    };
    use worldwake_sim::{ActionDuration, ActionPayload, DurationExpr, RuntimeBeliefView};

    /// Minimal mock for assumption tests.
    struct MockBeliefView {
        alive: BTreeSet<EntityId>,
        places: BTreeMap<EntityId, EntityId>,
        routes: BTreeSet<(EntityId, EntityId)>,
    }

    impl MockBeliefView {
        fn new() -> Self {
            Self {
                alive: BTreeSet::new(),
                places: BTreeMap::new(),
                routes: BTreeSet::new(),
            }
        }
    }

    worldwake_sim::impl_goal_belief_view!(MockBeliefView);

    impl RuntimeBeliefView for MockBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.contains(&entity)
        }
        fn entity_kind(&self, _entity: EntityId) -> Option<EntityKind> {
            None
        }
        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.places.get(&entity).copied()
        }
        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }
        fn entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn adjacent_places(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn knows_recipe(&self, _actor: EntityId, _recipe: RecipeId) -> bool {
            false
        }
        fn unique_item_count(&self, _holder: EntityId, _kind: UniqueItemKind) -> u32 {
            0
        }
        fn commodity_quantity(&self, _holder: EntityId, _kind: CommodityKind) -> Quantity {
            Quantity(0)
        }
        fn controlled_commodity_quantity_at_place(
            &self,
            _a: EntityId,
            _p: EntityId,
            _c: CommodityKind,
        ) -> Quantity {
            Quantity(0)
        }
        fn local_controlled_lots_for(
            &self,
            _a: EntityId,
            _p: EntityId,
            _c: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }
        fn item_lot_commodity(&self, _entity: EntityId) -> Option<CommodityKind> {
            None
        }
        fn item_lot_consumable_profile(
            &self,
            _entity: EntityId,
        ) -> Option<CommodityConsumableProfile> {
            None
        }
        fn direct_container(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }
        fn direct_possessor(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }
        fn believed_owner_of(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }
        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }
        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }
        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }
        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            false
        }
        fn has_control(&self, _entity: EntityId) -> bool {
            false
        }
        fn carry_capacity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }
        fn load_of_entity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }
        fn reservation_conflicts(&self, _entity: EntityId, _range: TickRange) -> bool {
            false
        }
        fn reservation_ranges(&self, _entity: EntityId) -> Vec<TickRange> {
            Vec::new()
        }
        fn is_dead(&self, entity: EntityId) -> bool {
            !self.alive.contains(&entity)
        }
        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }
        fn has_wounds(&self, _entity: EntityId) -> bool {
            false
        }
        fn homeostatic_needs(&self, _agent: EntityId) -> Option<HomeostaticNeeds> {
            None
        }
        fn drive_thresholds(&self, _agent: EntityId) -> Option<DriveThresholds> {
            None
        }
        fn belief_confidence_policy(&self, _agent: EntityId) -> BeliefConfidencePolicy {
            BeliefConfidencePolicy::default()
        }
        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
            None
        }
        fn trade_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<TradeDispositionProfile> {
            None
        }
        fn intention_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<IntentionDispositionProfile> {
            None
        }
        fn route_exists(&self, from: EntityId, to: EntityId) -> bool {
            self.routes.contains(&(from, to))
        }
        fn combat_profile(&self, _entity: EntityId) -> Option<CombatProfile> {
            None
        }
        fn wounds(&self, _entity: EntityId) -> Vec<Wound> {
            Vec::new()
        }
        fn visible_hostiles_for(&self, _entity: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn current_attackers_of(&self, _entity: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn agents_selling_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }
        fn known_recipes(&self, _actor: EntityId) -> Vec<RecipeId> {
            Vec::new()
        }
        fn matching_workstations_at(
            &self,
            _place: EntityId,
            _tag: WorkstationTag,
        ) -> Vec<EntityId> {
            Vec::new()
        }
        fn resource_sources_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }
        fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> {
            Vec::new()
        }
        fn merchandise_profile(&self, _entity: EntityId) -> Option<MerchandiseProfile> {
            None
        }
        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn in_transit_state(&self, _entity: EntityId) -> Option<InTransitOnEdge> {
            None
        }
        fn adjacent_places_with_travel_ticks(
            &self,
            _place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
            Vec::new()
        }
        fn estimate_duration(
            &self,
            _actor: EntityId,
            _expr: &DurationExpr,
            _targets: &[EntityId],
            _payload: &ActionPayload,
        ) -> Option<ActionDuration> {
            None
        }
    }

    fn make_entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn make_frame(domain: IntentionDomain, state: FrameState) -> IntentionFrame {
        IntentionFrame {
            goal: GoalKey::new(GoalKind::Sleep),
            domain,
            assumptions: Vec::new(),
            state,
            established_at: Tick(0),
            last_progress_tick: None,
            stalled_ticks: 0,
            patience_limit: 30,
        }
    }

    fn make_ranked_goal(priority_class: GoalPriorityClass) -> RankedGoal {
        RankedGoal {
            grounded: GroundedGoal {
                key: GoalKey::new(GoalKind::Sleep),
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
            },
            priority_class,
            motive_score: 100,
            provenance: None,
        }
    }

    // ── populate_assumptions tests ──

    #[test]
    fn populate_travel_produces_route_exists() {
        let agent = make_entity(0);
        let place_a = make_entity(10);
        let dest = make_entity(20);
        let mut view = MockBeliefView::new();
        view.alive.insert(agent);
        view.places.insert(agent, place_a);

        let assumptions =
            populate_assumptions(&IntentionDomain::Travel { destination: dest }, agent, &view);
        assert_eq!(
            assumptions,
            vec![FrameAssumption::RouteExists {
                from: place_a,
                to: dest
            }]
        );
    }

    #[test]
    fn populate_care_produces_target_alive_and_route() {
        let agent = make_entity(0);
        let patient = make_entity(1);
        let place_a = make_entity(10);
        let place_b = make_entity(11);
        let mut view = MockBeliefView::new();
        view.alive.insert(agent);
        view.alive.insert(patient);
        view.places.insert(agent, place_a);
        view.places.insert(patient, place_b);

        let assumptions =
            populate_assumptions(&IntentionDomain::Care { patient }, agent, &view);
        assert_eq!(
            assumptions,
            vec![
                FrameAssumption::TargetAlive(patient),
                FrameAssumption::RouteExists {
                    from: place_a,
                    to: place_b
                },
            ]
        );
    }

    #[test]
    fn populate_escort_produces_target_alive_and_route() {
        let agent = make_entity(0);
        let ward = make_entity(1);
        let dest = make_entity(20);
        let place_a = make_entity(10);
        let mut view = MockBeliefView::new();
        view.alive.insert(agent);
        view.alive.insert(ward);
        view.places.insert(agent, place_a);

        let assumptions = populate_assumptions(
            &IntentionDomain::Escort {
                ward,
                destination: dest,
            },
            agent,
            &view,
        );
        assert_eq!(
            assumptions,
            vec![
                FrameAssumption::TargetAlive(ward),
                FrameAssumption::RouteExists {
                    from: place_a,
                    to: dest
                },
            ]
        );
    }

    #[test]
    fn populate_errand_produces_route_exists() {
        let agent = make_entity(0);
        let dest = make_entity(20);
        let place_a = make_entity(10);
        let mut view = MockBeliefView::new();
        view.alive.insert(agent);
        view.places.insert(agent, place_a);

        let assumptions =
            populate_assumptions(&IntentionDomain::Errand { destination: dest }, agent, &view);
        assert_eq!(
            assumptions,
            vec![FrameAssumption::RouteExists {
                from: place_a,
                to: dest
            }]
        );
    }

    #[test]
    fn populate_generic_produces_no_critical_threat() {
        let agent = make_entity(0);
        let view = MockBeliefView::new();

        let assumptions = populate_assumptions(&IntentionDomain::Generic, agent, &view);
        assert_eq!(assumptions, vec![FrameAssumption::NoCriticalThreat]);
    }

    // ── evaluate_assumptions tests ──

    #[test]
    fn target_alive_dead_produces_critical_failure() {
        let dead_entity = make_entity(1);
        let view = MockBeliefView::new(); // entity not in alive set

        let result = evaluate_assumptions(
            &[FrameAssumption::TargetAlive(dead_entity)],
            &view,
            Some(&[]),
        );
        assert_eq!(result, AssumptionEvalResult::CriticalFailure);
    }

    #[test]
    fn route_exists_severed_produces_recoverable_route_blocked() {
        let from = make_entity(10);
        let to = make_entity(20);
        let view = MockBeliefView::new(); // no routes

        let result = evaluate_assumptions(
            &[FrameAssumption::RouteExists { from, to }],
            &view,
            Some(&[]),
        );
        assert_eq!(
            result,
            AssumptionEvalResult::RecoverableFailure(SuspensionReason::RouteBlocked)
        );
    }

    #[test]
    fn no_critical_threat_with_critical_candidate_produces_survival_need() {
        let view = MockBeliefView::new();
        let candidates = vec![make_ranked_goal(GoalPriorityClass::Critical)];

        let result = evaluate_assumptions(
            &[FrameAssumption::NoCriticalThreat],
            &view,
            Some(&candidates),
        );
        assert_eq!(
            result,
            AssumptionEvalResult::RecoverableFailure(SuspensionReason::SurvivalNeed)
        );
    }

    #[test]
    fn all_assumptions_pass_returns_all_pass() {
        let entity = make_entity(1);
        let from = make_entity(10);
        let to = make_entity(20);
        let mut view = MockBeliefView::new();
        view.alive.insert(entity);
        view.routes.insert((from, to));

        let result = evaluate_assumptions(
            &[
                FrameAssumption::TargetAlive(entity),
                FrameAssumption::RouteExists { from, to },
            ],
            &view,
            Some(&[]),
        );
        assert_eq!(result, AssumptionEvalResult::AllPass);
    }

    #[test]
    fn no_critical_threat_without_candidates_returns_deferred() {
        let view = MockBeliefView::new();

        let result = evaluate_assumptions(
            &[FrameAssumption::NoCriticalThreat],
            &view,
            None,
        );
        assert_eq!(result, AssumptionEvalResult::Deferred);
    }

    #[test]
    fn commodity_available_at_stubbed_as_pass() {
        let view = MockBeliefView::new();

        let result = evaluate_assumptions(
            &[FrameAssumption::CommodityAvailableAt {
                commodity: CommodityKind::Grain,
                place: make_entity(10),
            }],
            &view,
            Some(&[]),
        );
        assert_eq!(result, AssumptionEvalResult::AllPass);
    }

    // ── apply_assumption_result tests ──

    #[test]
    fn critical_failure_transitions_to_exhausted() {
        let frame = make_frame(
            IntentionDomain::Care {
                patient: make_entity(1),
            },
            FrameState::Active,
        );
        let mut runtime = AgentDecisionRuntime::default();

        let result =
            apply_assumption_result(&frame, &AssumptionEvalResult::CriticalFailure, Tick(5), &mut runtime);
        let updated = result;
        assert_eq!(updated.state, FrameState::Exhausted);
        assert_eq!(
            runtime.last_frame_clear_reason,
            Some(FrameClearReason::AssumptionFailed)
        );
    }

    #[test]
    fn recoverable_failure_transitions_to_suspended() {
        let frame = make_frame(
            IntentionDomain::Travel {
                destination: make_entity(20),
            },
            FrameState::Active,
        );
        let mut runtime = AgentDecisionRuntime::default();

        let result = apply_assumption_result(
            &frame,
            &AssumptionEvalResult::RecoverableFailure(SuspensionReason::RouteBlocked),
            Tick(5),
            &mut runtime,
        );
        let updated = result;
        assert_eq!(
            updated.state,
            FrameState::Suspended {
                reason: SuspensionReason::RouteBlocked,
                suspended_at: Tick(5),
            }
        );
    }

    #[test]
    fn all_pass_on_suspended_frame_resumes_to_active() {
        let frame = make_frame(
            IntentionDomain::Travel {
                destination: make_entity(20),
            },
            FrameState::Suspended {
                reason: SuspensionReason::RouteBlocked,
                suspended_at: Tick(3),
            },
        );
        let mut runtime = AgentDecisionRuntime::default();

        let result =
            apply_assumption_result(&frame, &AssumptionEvalResult::AllPass, Tick(7), &mut runtime);
        let updated = result;
        assert_eq!(updated.state, FrameState::Active);
    }

    #[test]
    fn resume_does_not_reset_stalled_ticks() {
        let mut frame = make_frame(
            IntentionDomain::Travel {
                destination: make_entity(20),
            },
            FrameState::Suspended {
                reason: SuspensionReason::RouteBlocked,
                suspended_at: Tick(3),
            },
        );
        frame.stalled_ticks = 5;
        let mut runtime = AgentDecisionRuntime::default();

        let result =
            apply_assumption_result(&frame, &AssumptionEvalResult::AllPass, Tick(7), &mut runtime);
        let updated = result;
        assert_eq!(updated.state, FrameState::Active);
        assert_eq!(updated.stalled_ticks, 5, "stalled_ticks must not be reset on resume");
    }

    #[test]
    fn exhausted_frame_not_re_evaluated() {
        // This test verifies the guard in process_agent. At the frame.rs
        // level, we test that apply_assumption_result on an Exhausted frame
        // does not change state.
        let frame = make_frame(
            IntentionDomain::Travel {
                destination: make_entity(20),
            },
            FrameState::Exhausted,
        );
        let mut runtime = AgentDecisionRuntime::default();

        // AllPass on Exhausted should leave it as Exhausted (no resume).
        let result =
            apply_assumption_result(&frame, &AssumptionEvalResult::AllPass, Tick(7), &mut runtime);
        let updated = result;
        assert_eq!(
            updated.state,
            FrameState::Exhausted,
            "Exhausted frames must not resume"
        );
    }

    // ── progress_op_kinds tests ──

    #[test]
    fn progress_ops_travel_returns_travel_only() {
        let ops = progress_op_kinds(&IntentionDomain::Travel {
            destination: make_entity(20),
        });
        assert_eq!(ops, &[PlannerOpKind::Travel]);
    }

    #[test]
    fn progress_ops_care_returns_heal_and_travel() {
        let ops = progress_op_kinds(&IntentionDomain::Care {
            patient: make_entity(1),
        });
        assert_eq!(ops, &[PlannerOpKind::Heal, PlannerOpKind::Travel]);
    }

    #[test]
    fn progress_ops_escort_returns_travel_only() {
        let ops = progress_op_kinds(&IntentionDomain::Escort {
            ward: make_entity(1),
            destination: make_entity(20),
        });
        assert_eq!(ops, &[PlannerOpKind::Travel]);
    }

    #[test]
    fn progress_ops_errand_returns_travel_and_political() {
        let ops = progress_op_kinds(&IntentionDomain::Errand {
            destination: make_entity(20),
        });
        assert_eq!(
            ops,
            &[
                PlannerOpKind::Travel,
                PlannerOpKind::DeclareSupport,
                PlannerOpKind::PressForceClaim,
                PlannerOpKind::YieldForceClaim,
            ]
        );
    }

    #[test]
    fn progress_ops_generic_returns_all_variants() {
        let ops = progress_op_kinds(&IntentionDomain::Generic);
        // Generic should contain every PlannerOpKind variant.
        assert!(ops.contains(&PlannerOpKind::Travel));
        assert!(ops.contains(&PlannerOpKind::Consume));
        assert!(ops.contains(&PlannerOpKind::Sleep));
        assert!(ops.contains(&PlannerOpKind::Relieve));
        assert!(ops.contains(&PlannerOpKind::Wash));
        assert!(ops.contains(&PlannerOpKind::Trade));
        assert!(ops.contains(&PlannerOpKind::QueueForFacilityUse));
        assert!(ops.contains(&PlannerOpKind::Harvest));
        assert!(ops.contains(&PlannerOpKind::Craft));
        assert!(ops.contains(&PlannerOpKind::MoveCargo));
        assert!(ops.contains(&PlannerOpKind::Heal));
        assert!(ops.contains(&PlannerOpKind::Loot));
        assert!(ops.contains(&PlannerOpKind::Bury));
        assert!(ops.contains(&PlannerOpKind::Tell));
        assert!(ops.contains(&PlannerOpKind::ConsultRecord));
        assert!(ops.contains(&PlannerOpKind::Attack));
        assert!(ops.contains(&PlannerOpKind::Defend));
        assert!(ops.contains(&PlannerOpKind::Bribe));
        assert!(ops.contains(&PlannerOpKind::Threaten));
        assert!(ops.contains(&PlannerOpKind::DeclareSupport));
        assert!(ops.contains(&PlannerOpKind::PressForceClaim));
        assert!(ops.contains(&PlannerOpKind::YieldForceClaim));
        assert_eq!(ops.len(), 22);
    }

    #[test]
    fn progress_ops_travel_excludes_consume() {
        let ops = progress_op_kinds(&IntentionDomain::Travel {
            destination: make_entity(20),
        });
        assert!(
            !ops.contains(&PlannerOpKind::Consume),
            "Eating during travel must not count as progress"
        );
    }

    // ── frame_blocker_target tests ──

    #[test]
    fn frame_blocker_target_travel_returns_destination() {
        let dest = make_entity(20);
        assert_eq!(
            frame_blocker_target(&IntentionDomain::Travel { destination: dest }),
            Some(dest)
        );
    }

    #[test]
    fn frame_blocker_target_care_returns_patient() {
        let patient = make_entity(5);
        assert_eq!(
            frame_blocker_target(&IntentionDomain::Care { patient }),
            Some(patient)
        );
    }

    #[test]
    fn frame_blocker_target_escort_returns_ward() {
        let ward = make_entity(3);
        let dest = make_entity(20);
        assert_eq!(
            frame_blocker_target(&IntentionDomain::Escort {
                ward,
                destination: dest
            }),
            Some(ward)
        );
    }

    #[test]
    fn frame_blocker_target_errand_returns_destination() {
        let dest = make_entity(20);
        assert_eq!(
            frame_blocker_target(&IntentionDomain::Errand { destination: dest }),
            Some(dest)
        );
    }

    #[test]
    fn frame_blocker_target_generic_returns_none() {
        assert_eq!(frame_blocker_target(&IntentionDomain::Generic), None);
    }
}
