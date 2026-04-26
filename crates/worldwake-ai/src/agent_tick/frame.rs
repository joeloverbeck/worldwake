use crate::{
    AgentDecisionRuntime, DirtySet, FrameRuntimeSnapshot, PatrolRouteSnapshotTrace, PlannedStep,
    PlannerOpKind, authoritative_target, classify_frame_plan_relation, has_active_frame_travel,
};
use crate::{GoalPriorityClass, ranking::OrderedRanked};
use worldwake_core::{
    Blocker, BlockerKey, BlockerMemory, BlockingFact, CognitiveProfile, CommodityKind,
    ContentionIntents, Discrepancy, DiscrepancyClearing, DiscrepancyEntry, DiscrepancyMemory,
    EntityId, FrameAssumption, FrameClearReason, FrameState, HomeostaticNeedId, IntentionDomain,
    IntentionFrame, Permille, Quantity, SuspensionReason, Tick,
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
    PlannerOpKind::EstablishCamp,
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
    PlannerOpKind::Investigate,
    PlannerOpKind::AskWitness,
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
    CognitiveProfile,
    FrameProfile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameDebugSnapshot {
    pub runtime: FrameRuntimeSnapshot,
    pub effective_switch_margin: Permille,
    pub switch_margin_source: FrameSwitchMarginSource,
    pub patrol_route: PatrolRouteSnapshotTrace,
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

    if same_frame && let Some(existing) = frame {
        return Some(IntentionFrame {
            goal: selected_plan.goal,
            domain: IntentionDomain::Travel { destination },
            state: FrameState::Active,
            ..existing.clone()
        });
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
    blocked_memory: &mut BlockerMemory,
    facility_intents: &mut ContentionIntents,
    agent: EntityId,
    step: &PlannedStep,
    tick: Tick,
    cognitive: &CognitiveProfile,
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
        .is_some_and(|profile| new_stalled >= profile.patience_for(f.domain.domain_tag()));

    let updated_frame = if patience_exhausted {
        let goal_key = active_goal.unwrap_or_else(|| {
            runtime
                .current_plan
                .as_ref()
                .map(|plan| plan.goal)
                .expect("active frame travel must retain a current goal")
        });
        blocked_memory.record(Blocker {
            blocker_key: BlockerKey {
                goal_key,
                place: blocked_leg_target(step),
                target: None,
                action_def: Some(step.def_id),
            },
            blocking_fact: worldwake_core::BlockingFact::NoKnownPath,
            diagnostic_context: None,
            observed_tick: tick,
            expires_tick: tick + u64::from(cognitive.structural_block_ticks),
            clearing_condition: worldwake_core::BlockerClearingCondition::TtlOnly,
            baseline_snapshot: None,
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
    facility_intents.intents.clear();
    runtime.dirty.insert(DirtySet::FRAME_BLOCKAGE);
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
    CriticalFailure(FrameAssumption),
    /// Evaluation was deferred (contains `NoCriticalThreat` that needs ranked candidates).
    Deferred,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Eq, PartialEq)]
pub(super) enum AvailabilityVerdict {
    Believed,
    Refuted,
    UnknownOrStale,
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn assess_commodity_availability(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    commodity: CommodityKind,
    place: EntityId,
) -> AvailabilityVerdict {
    if view.effective_place(agent) == Some(place) {
        let found_local_match = view.entities_at(place).into_iter().any(|entity| {
            view.item_lot_commodity(entity) == Some(commodity)
                || view.resource_source(entity).is_some_and(|source| {
                    source.commodity == commodity && source.available_quantity > Quantity(0)
                })
        });
        return if found_local_match {
            AvailabilityVerdict::Believed
        } else {
            AvailabilityVerdict::Refuted
        };
    }

    let Some(store) = view.agent_belief_store(agent) else {
        return AvailabilityVerdict::UnknownOrStale;
    };

    let mut saw_place_belief = false;
    for state in store.known_entities.values() {
        if state.last_known_place != Some(place) {
            continue;
        }
        saw_place_belief = true;
        let believed_source = state.resource_source.as_ref().is_some_and(|source| {
            source.commodity == commodity && source.available_quantity > Quantity(0)
        });
        let believed_inventory = state
            .last_known_inventory
            .get(&commodity)
            .copied()
            .unwrap_or(Quantity(0))
            > Quantity(0);
        if believed_source || believed_inventory {
            return AvailabilityVerdict::Believed;
        }
    }

    let _ = saw_place_belief;
    AvailabilityVerdict::UnknownOrStale
}

/// Sum of `estimated_ticks` for the steps that have already been completed
/// in `plan` according to `current_step_index`.
pub(super) fn completed_step_ticks(plan: &crate::PlannedPlan, current_step_index: usize) -> u64 {
    plan.steps
        .iter()
        .take(current_step_index)
        .map(|step| u64::from(step.estimated_ticks))
        .sum()
}

/// Compute the tick at which the agent's currently active plan is expected
/// to complete, given the runtime's plan-and-step-index state. Returns
/// `current_tick` when no plan is active (suppresses need-horizon assumption
/// population on the trivial no-plan branch).
pub(super) fn plan_completion_tick(runtime: &AgentDecisionRuntime, current_tick: Tick) -> Tick {
    let Some(plan) = runtime.current_plan.as_ref() else {
        return current_tick;
    };
    let completed = completed_step_ticks(plan, runtime.current_step_index);
    let remaining = u64::from(plan.total_estimated_ticks).saturating_sub(completed);
    Tick(current_tick.0.saturating_add(remaining))
}

/// Compute the tick at which `plan` is expected to complete, given that no
/// steps have been completed yet (the just-being-adopted case). Returns
/// `current_tick + plan.total_estimated_ticks`.
pub(super) fn plan_completion_tick_for_adoption(
    plan: &crate::PlannedPlan,
    current_tick: Tick,
) -> Tick {
    Tick(
        current_tick
            .0
            .saturating_add(u64::from(plan.total_estimated_ticks)),
    )
}

/// Populate assumptions for an intention frame based on its domain and the
/// agent's current believed position. This is a standalone function (not a
/// method on `IntentionFrame`) to keep `worldwake-core` free of `BeliefView`
/// dependencies.
///
/// `current_tick` and `plan_completion_tick` drive per-need horizon
/// projection (S126 D4). When `plan_completion_tick == current_tick` (the
/// no-plan branch), need-horizon population is trivially skipped.
pub(super) fn populate_assumptions(
    frame: &IntentionFrame,
    agent: EntityId,
    view: &dyn RuntimeBeliefView,
    current_tick: Tick,
    plan_completion_tick: Tick,
) -> Vec<FrameAssumption> {
    let domain = &frame.domain;
    let current_place = view.effective_place(agent);
    let mut assumptions = match *domain {
        IntentionDomain::Travel { destination } | IntentionDomain::Errand { destination } => {
            let mut assumptions = Vec::with_capacity(2);
            if let Some(from) = current_place {
                assumptions.push(FrameAssumption::RouteExists {
                    from,
                    to: destination,
                });
            }
            if let Some((commodity, place)) = frame.expected_commodity() {
                assumptions.push(FrameAssumption::CommodityAvailableAt { commodity, place });
            }
            assumptions
        }
        IntentionDomain::Care { patient } => {
            let mut assumptions = Vec::with_capacity(2);
            assumptions.push(FrameAssumption::TargetAlive(patient));
            if let Some(from) = current_place
                && let Some(patient_place) = view.effective_place(patient)
            {
                assumptions.push(FrameAssumption::RouteExists {
                    from,
                    to: patient_place,
                });
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
    };

    let (Some(metabolism), Some(needs), Some(thresholds)) = (
        view.metabolism_profile(agent),
        view.homeostatic_needs(agent),
        view.drive_thresholds(agent),
    ) else {
        return assumptions;
    };
    for &need in &HomeostaticNeedId::ALL {
        let projected = needs.projected_tick_of(
            need,
            thresholds.high(need),
            metabolism.rate(need),
            current_tick,
        );
        // The assumption "I will stay safe until X" requires that the agent is
        // currently safe — i.e., the projected breach is genuinely in the
        // future (`breach_tick > current_tick`). When the level is already at
        // or above the high threshold, `projected_tick_of` returns
        // `Some(current_tick)`, indicating the breach has already occurred.
        // Pushing the assumption in that case would pre-falsify it for any
        // non-trivial plan. The agent's reactive ranking already handles
        // already-breached needs; the projection assumption only adds value
        // when the agent has time to revise before breach.
        if let Some(breach_tick) = projected
            && breach_tick > current_tick
            && breach_tick < plan_completion_tick
        {
            assumptions.push(FrameAssumption::NeedSafeUntilTick {
                need,
                until_tick: plan_completion_tick,
            });
        }
    }
    assumptions
}

/// Evaluate a set of assumptions against the agent's beliefs. Returns the
/// evaluation result.
///
/// `NoCriticalThreat` requires ranked candidates to check for
/// `GoalPriorityClass::Critical`. If `ranked_candidates` is `None`,
/// `NoCriticalThreat` is skipped (returns `Deferred` if it was the only
/// assumption that could fail). If `Some`, it is evaluated immediately.
///
pub(super) fn evaluate_assumptions(
    assumptions: &[FrameAssumption],
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    ranked_candidates: Option<&OrderedRanked<'_>>,
    current_tick: Tick,
) -> AssumptionEvalResult {
    let mut has_deferred = false;

    for assumption in assumptions {
        match *assumption {
            FrameAssumption::TargetAlive(entity) => {
                if !view.is_alive(entity) {
                    return AssumptionEvalResult::CriticalFailure(*assumption);
                }
            }
            FrameAssumption::RouteExists { from, to } => {
                if !view.route_exists(from, to) {
                    return AssumptionEvalResult::RecoverableFailure(
                        SuspensionReason::RouteBlocked,
                    );
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
            FrameAssumption::CommodityAvailableAt { commodity, place } => {
                match assess_commodity_availability(view, agent, commodity, place) {
                    AvailabilityVerdict::Believed => {}
                    AvailabilityVerdict::Refuted => {
                        return AssumptionEvalResult::CriticalFailure(*assumption);
                    }
                    AvailabilityVerdict::UnknownOrStale => has_deferred = true,
                }
            }
            FrameAssumption::NeedSafeUntilTick { need, until_tick } => {
                let (Some(metabolism), Some(needs), Some(thresholds)) = (
                    view.metabolism_profile(agent),
                    view.homeostatic_needs(agent),
                    view.drive_thresholds(agent),
                ) else {
                    has_deferred = true;
                    continue;
                };
                let projected = needs.projected_tick_of(
                    need,
                    thresholds.high(need),
                    metabolism.rate(need),
                    current_tick,
                );
                if let Some(breach_tick) = projected
                    && breach_tick < until_tick
                {
                    return AssumptionEvalResult::CriticalFailure(*assumption);
                }
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
        AssumptionEvalResult::CriticalFailure(_) => {
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
/// `IntentionDomain`. Used when creating `Blocker`s on frame exhaustion
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
/// increment. If so, record a `Blocker` with `PatienceExhausted`,
/// transition the frame to `Exhausted`, clear the plan, and return `true`.
///
/// The caller must have already incremented `frame.stalled_ticks`. This
/// function only handles the creation of the blocked intent and state
/// transition when the threshold is met.
pub(super) fn check_patience_exhaustion(
    frame: &IntentionFrame,
    agent_place: Option<EntityId>,
    blocked_memory: &mut BlockerMemory,
    facility_intents: &mut ContentionIntents,
    runtime: &mut AgentDecisionRuntime,
    tick: Tick,
    structural_block_ticks: u32,
) -> bool {
    if frame.stalled_ticks < frame.patience_limit {
        return false;
    }
    blocked_memory.record(Blocker {
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
        clearing_condition: worldwake_core::BlockerClearingCondition::TtlOnly,
        baseline_snapshot: None,
    });
    runtime.last_frame_clear_reason = Some(FrameClearReason::PatienceExhausted);
    runtime.current_plan = None;
    runtime.current_step_index = 0;
    runtime.materialization_bindings.clear();
    facility_intents.intents.clear();
    runtime.dirty.insert(DirtySet::FRAME_PATIENCE);
    true
}

/// Record a typed discrepancy for a frame whose critical assumption has
/// failed. Pre-S109 this path emitted `BlockingFact::AssumptionFailed`
/// recorded with `BlockerClearingCondition::TtlOnly` and a TTL of
/// `structural_block_ticks` (200 by default). The S109 migration must
/// preserve that structural suppression window, but commodity-availability
/// failures also need a place-level clearing path so fresh local evidence can
/// unblock the goal before TTL expiry. Other assumption failures remain
/// TTL-only because re-perceiving the same target does not prove that the
/// failed plan-level assumption is now resolved.
pub(super) fn record_assumption_failure(
    frame: &IntentionFrame,
    agent_place: Option<EntityId>,
    blocker_target: Option<EntityId>,
    discrepancy_memory: &mut DiscrepancyMemory,
    tick: Tick,
    structural_block_ticks: u32,
    failed_assumption: FrameAssumption,
) {
    let target = blocker_target.or_else(|| frame_blocker_target(&frame.domain));
    let (discrepancy, clearing_condition) =
        if let FrameAssumption::NeedSafeUntilTick { need, until_tick } = failed_assumption {
            (
                Discrepancy::NeedHorizonExceeded {
                    need,
                    projected_breach_tick: until_tick,
                },
                DiscrepancyClearing::TtlExpiry,
            )
        } else {
            let clearing = frame.expected_commodity().map_or(
                DiscrepancyClearing::TtlExpiry,
                |(commodity, place)| DiscrepancyClearing::CommodityAvailabilityChanged {
                    commodity,
                    place,
                },
            );
            let discrepancy = if target.is_some() {
                Discrepancy::BeliefContradicted
            } else {
                Discrepancy::PartialExecutionDrift
            };
            (discrepancy, clearing)
        };
    discrepancy_memory.record(DiscrepancyEntry {
        blocker_key: BlockerKey {
            goal_key: frame.goal,
            place: agent_place,
            target,
            action_def: None,
        },
        discrepancy,
        observed_tick: tick,
        expires_tick: tick + u64::from(structural_block_ticks),
        clearing_condition,
    });
}

pub(super) fn record_source_invalidation(
    frame: &IntentionFrame,
    discrepancy_memory: &mut DiscrepancyMemory,
    tick: Tick,
    structural_block_ticks: u32,
) {
    discrepancy_memory.record(DiscrepancyEntry {
        blocker_key: BlockerKey {
            goal_key: frame.goal,
            place: None,
            target: None,
            action_def: None,
        },
        discrepancy: Discrepancy::SourceInvalidated,
        observed_tick: tick,
        expires_tick: tick + u64::from(structural_block_ticks),
        clearing_condition: DiscrepancyClearing::TtlExpiry,
    });
}

fn blocked_leg_target(step: &PlannedStep) -> Option<EntityId> {
    step.targets.first().copied().and_then(authoritative_target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgendaEntry, GoalOffer, GoalPriorityClass};
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        AcquisitionQuantity, AgentBeliefStore, BeliefConfidencePolicy, BelievedEntityState,
        CombatProfile, CommodityConsumableProfile, CommodityKind, CommodityPurpose,
        DemandObservation, DriveThresholds, EntityKind, GoalKey, GoalKind, HomeostaticNeeds,
        InTransitOnEdge, IntentionDispositionProfile, LoadUnits, MerchandiseProfile,
        MetabolismProfile, PerceptionSource, Quantity, RecipeId, ResourceSource, Tick, TickRange,
        TradeDispositionProfile, UniqueItemKind, WorkstationTag, Wound,
    };
    use worldwake_sim::{
        ActionDuration, ActionPayload, CombatBeliefView, ControlBeliefView, DurationExpr,
        EconomicBeliefView, EntityBeliefView, ProfileBeliefView, RuntimeBeliefView,
        SpatialBeliefView, TemporalBeliefView,
    };

    /// Minimal mock for assumption tests.
    struct MockBeliefView {
        alive: BTreeSet<EntityId>,
        places: BTreeMap<EntityId, EntityId>,
        routes: BTreeSet<(EntityId, EntityId)>,
        entities_at: BTreeMap<EntityId, Vec<EntityId>>,
        item_lot_commodities: BTreeMap<EntityId, CommodityKind>,
        resource_sources: BTreeMap<EntityId, ResourceSource>,
        belief_stores: BTreeMap<EntityId, AgentBeliefStore>,
        homeostatic_needs: BTreeMap<EntityId, HomeostaticNeeds>,
        metabolism_profiles: BTreeMap<EntityId, MetabolismProfile>,
        drive_thresholds: BTreeMap<EntityId, DriveThresholds>,
    }

    impl MockBeliefView {
        fn new() -> Self {
            Self {
                alive: BTreeSet::new(),
                places: BTreeMap::new(),
                routes: BTreeSet::new(),
                entities_at: BTreeMap::new(),
                item_lot_commodities: BTreeMap::new(),
                resource_sources: BTreeMap::new(),
                belief_stores: BTreeMap::new(),
                homeostatic_needs: BTreeMap::new(),
                metabolism_profiles: BTreeMap::new(),
                drive_thresholds: BTreeMap::new(),
            }
        }
    }

    impl ControlBeliefView for MockBeliefView {
        fn believed_owner_of(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            false
        }

        fn has_control(&self, _entity: EntityId) -> bool {
            false
        }
    }

    impl EntityBeliefView for MockBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.contains(&entity)
        }
        fn entity_kind(&self, _entity: EntityId) -> Option<EntityKind> {
            None
        }
        fn is_dead(&self, entity: EntityId) -> bool {
            !self.alive.contains(&entity)
        }
        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }
        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
    }

    impl ProfileBeliefView for MockBeliefView {
        fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds> {
            self.homeostatic_needs.get(&agent).copied()
        }
        fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds> {
            self.drive_thresholds.get(&agent).copied()
        }
        fn metabolism_profile(&self, agent: EntityId) -> Option<MetabolismProfile> {
            self.metabolism_profiles.get(&agent).copied()
        }
    }

    impl SpatialBeliefView for MockBeliefView {
        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.places.get(&entity).copied()
        }
        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }
        fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
            self.entities_at.get(&place).cloned().unwrap_or_default()
        }
        fn adjacent_places(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn route_exists(&self, from: EntityId, to: EntityId) -> bool {
            self.routes.contains(&(from, to))
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
    }

    impl TemporalBeliefView for MockBeliefView {
        fn reservation_conflicts(&self, _entity: EntityId, _range: TickRange) -> bool {
            false
        }
        fn reservation_ranges(&self, _entity: EntityId) -> Vec<TickRange> {
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

    impl RuntimeBeliefView for MockBeliefView {}

    impl worldwake_sim::SocialBeliefView for MockBeliefView {
        fn belief_confidence_policy(&self, _agent: EntityId) -> BeliefConfidencePolicy {
            BeliefConfidencePolicy::default()
        }
        fn intention_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<IntentionDispositionProfile> {
            None
        }
        fn agent_belief_store(&self, agent: EntityId) -> Option<&AgentBeliefStore> {
            self.belief_stores.get(&agent)
        }
    }

    impl worldwake_sim::PoliticalBeliefView for MockBeliefView {}

    impl CombatBeliefView for MockBeliefView {
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
        fn has_wounds(&self, _entity: EntityId) -> bool {
            false
        }
    }

    impl EconomicBeliefView for MockBeliefView {
        fn trade_disposition_profile(&self, _agent: EntityId) -> Option<TradeDispositionProfile> {
            None
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
        fn listed_sale_lots_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }
        fn seller_for_sale_lot(&self, _lot: EntityId) -> Option<EntityId> {
            None
        }
        fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> {
            Vec::new()
        }
        fn merchandise_profile(&self, _entity: EntityId) -> Option<MerchandiseProfile> {
            None
        }
    }

    impl worldwake_sim::InventoryBeliefView for MockBeliefView {
        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
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
        fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind> {
            self.item_lot_commodities.get(&entity).copied()
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
        fn carry_capacity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }
        fn load_of_entity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }
        fn known_recipes(&self, _actor: EntityId) -> Vec<RecipeId> {
            Vec::new()
        }
    }

    impl worldwake_sim::FacilityBeliefView for MockBeliefView {
        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }
        fn resource_source(&self, entity: EntityId) -> Option<ResourceSource> {
            self.resource_sources.get(&entity).cloned()
        }
        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
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

    fn make_ranked_goal(priority_class: GoalPriorityClass) -> AgendaEntry {
        AgendaEntry {
            key: worldwake_core::OpportunityKey {
                goal_key: GoalKey::new(GoalKind::Sleep),
                anchor: worldwake_core::OpportunityAnchor::None,
            },
            offer: GoalOffer {
                anchor: worldwake_core::OpportunityAnchor::None,
                key: GoalKey::new(GoalKind::Sleep),
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
                obligation_source: None,
                commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
            },
            priority_class,
            motive_score: 100,
            provenance: None,
            source_reliability_discount: None,
            competition_discount: None,
            feasibility: crate::feasibility::FeasibilityHint::Uncertain,
            phase: crate::AgendaPhase::Pending,
            origin: crate::AgendaOrigin::NeedDrive,
            introduced_tick: Tick(0),
            last_reconsidered_tick: Tick(0),
            revival_trigger: None,
            kill_condition: crate::KillCondition::External,
        }
    }

    fn ordered(ranked: &[AgendaEntry]) -> crate::ranking::OrderedRanked<'_> {
        crate::ranking::OrderedRanked::from_sorted_for_test(ranked)
    }

    fn observed_entity_state(place: EntityId) -> BelievedEntityState {
        let mut state = BelievedEntityState::single_observation_defaults(
            Tick(0),
            PerceptionSource::DirectObservation,
        );
        state.last_known_place = Some(place);
        state
    }

    fn store_with_known_entity(entity: EntityId, state: BelievedEntityState) -> AgentBeliefStore {
        let mut store = AgentBeliefStore::new();
        store.known_entities.insert(entity, state);
        store
    }

    #[test]
    fn assess_commodity_availability_co_located_lot_returns_believed() {
        let agent = make_entity(0);
        let place = make_entity(10);
        let lot = make_entity(20);
        let mut view = MockBeliefView::new();
        view.places.insert(agent, place);
        view.entities_at.insert(place, vec![lot]);
        view.item_lot_commodities.insert(lot, CommodityKind::Apple);

        let verdict = assess_commodity_availability(&view, agent, CommodityKind::Apple, place);
        assert_eq!(verdict, AvailabilityVerdict::Believed);
    }

    #[test]
    fn assess_commodity_availability_co_located_resource_source_returns_believed() {
        let agent = make_entity(0);
        let place = make_entity(10);
        let source = make_entity(21);
        let mut view = MockBeliefView::new();
        view.places.insert(agent, place);
        view.entities_at.insert(place, vec![source]);
        view.resource_sources.insert(
            source,
            ResourceSource {
                commodity: CommodityKind::Apple,
                available_quantity: Quantity(3),
                max_quantity: Quantity(5),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            },
        );

        let verdict = assess_commodity_availability(&view, agent, CommodityKind::Apple, place);
        assert_eq!(verdict, AvailabilityVerdict::Believed);
    }

    #[test]
    fn assess_commodity_availability_co_located_no_match_returns_refuted() {
        let agent = make_entity(0);
        let place = make_entity(10);
        let lot = make_entity(20);
        let mut view = MockBeliefView::new();
        view.places.insert(agent, place);
        view.entities_at.insert(place, vec![lot]);
        view.item_lot_commodities.insert(lot, CommodityKind::Bread);

        let verdict = assess_commodity_availability(&view, agent, CommodityKind::Apple, place);
        assert_eq!(verdict, AvailabilityVerdict::Refuted);
    }

    #[test]
    fn assess_commodity_availability_belief_backed_lot_returns_believed() {
        let agent = make_entity(0);
        let place = make_entity(10);
        let remembered_lot = make_entity(30);
        let mut view = MockBeliefView::new();
        view.places.insert(agent, make_entity(11));
        let mut state = observed_entity_state(place);
        state
            .last_known_inventory
            .insert(CommodityKind::Apple, Quantity(2));
        view.belief_stores
            .insert(agent, store_with_known_entity(remembered_lot, state));

        let verdict = assess_commodity_availability(&view, agent, CommodityKind::Apple, place);
        assert_eq!(verdict, AvailabilityVerdict::Believed);
    }

    #[test]
    fn assess_commodity_availability_no_belief_returns_unknown_or_stale() {
        let agent = make_entity(0);
        let place = make_entity(10);
        let mut view = MockBeliefView::new();
        view.places.insert(agent, make_entity(11));

        let verdict = assess_commodity_availability(&view, agent, CommodityKind::Apple, place);
        assert_eq!(verdict, AvailabilityVerdict::UnknownOrStale);
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
        let frame = make_frame(
            IntentionDomain::Travel { destination: dest },
            FrameState::Active,
        );

        let assumptions = populate_assumptions(&frame, agent, &view, Tick(0), Tick(0));
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
        let frame = make_frame(IntentionDomain::Care { patient }, FrameState::Active);

        let assumptions = populate_assumptions(&frame, agent, &view, Tick(0), Tick(0));
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
        let frame = make_frame(
            IntentionDomain::Escort {
                ward,
                destination: dest,
            },
            FrameState::Active,
        );

        let assumptions = populate_assumptions(&frame, agent, &view, Tick(0), Tick(0));
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
        let frame = make_frame(
            IntentionDomain::Errand { destination: dest },
            FrameState::Active,
        );

        let assumptions = populate_assumptions(&frame, agent, &view, Tick(0), Tick(0));
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
        let frame = make_frame(IntentionDomain::Generic, FrameState::Active);

        let assumptions = populate_assumptions(&frame, agent, &view, Tick(0), Tick(0));
        assert_eq!(assumptions, vec![FrameAssumption::NoCriticalThreat]);
    }

    #[test]
    fn populate_travel_with_acquire_commodity_produces_route_and_commodity() {
        let agent = make_entity(0);
        let place_a = make_entity(10);
        let dest = make_entity(20);
        let mut view = MockBeliefView::new();
        view.alive.insert(agent);
        view.places.insert(agent, place_a);
        let frame = IntentionFrame {
            goal: GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
            domain: IntentionDomain::Travel { destination: dest },
            assumptions: Vec::new(),
            state: FrameState::Active,
            established_at: Tick(0),
            last_progress_tick: None,
            stalled_ticks: 0,
            patience_limit: 30,
        };

        let assumptions = populate_assumptions(&frame, agent, &view, Tick(0), Tick(0));
        assert_eq!(
            assumptions,
            vec![
                FrameAssumption::RouteExists {
                    from: place_a,
                    to: dest,
                },
                FrameAssumption::CommodityAvailableAt {
                    commodity: CommodityKind::Apple,
                    place: dest,
                },
            ]
        );
    }

    // ── populate_assumptions need-horizon tests (S126 D4) ──

    /// Build a `MetabolismProfile` with a single non-zero rate for `need` (set
    /// to `rate`) and zero rates for all other needs. Other (non-rate) fields
    /// are taken from `MetabolismProfile::default()`. Used to isolate per-need
    /// projection arithmetic in horizon tests.
    fn metabolism_with_only(need: HomeostaticNeedId, rate: Permille) -> MetabolismProfile {
        let mut profile = MetabolismProfile::default();
        let zero = Permille::ZERO;
        profile.hunger_rate = zero;
        profile.thirst_rate = zero;
        profile.fatigue_rate = zero;
        profile.bladder_rate = zero;
        profile.dirtiness_rate = zero;
        match need {
            HomeostaticNeedId::Hunger => profile.hunger_rate = rate,
            HomeostaticNeedId::Thirst => profile.thirst_rate = rate,
            HomeostaticNeedId::Fatigue => profile.fatigue_rate = rate,
            HomeostaticNeedId::Bladder => profile.bladder_rate = rate,
            HomeostaticNeedId::Dirtiness => profile.dirtiness_rate = rate,
        }
        profile
    }

    fn permille(value: u16) -> Permille {
        Permille::new(value).expect("test permille in range")
    }

    fn seed_physiology(
        view: &mut MockBeliefView,
        agent: EntityId,
        needs: HomeostaticNeeds,
        metabolism: MetabolismProfile,
    ) {
        view.homeostatic_needs.insert(agent, needs);
        view.metabolism_profiles.insert(agent, metabolism);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
    }

    #[test]
    fn populate_produces_need_safe_until_tick_when_breach_before_plan_completion() {
        // hunger=400, hunger_rate=50, hunger.high()=750 (DriveThresholds::default).
        // breach_tick = 10 + ⌈(750-400)/50⌉ = 10 + 7 = Tick(17).
        // plan_completion_tick = Tick(20) > breach_tick → assumption pushed.
        let agent = make_entity(0);
        let view_place = make_entity(10);
        let dest = make_entity(20);
        let mut view = MockBeliefView::new();
        view.alive.insert(agent);
        view.places.insert(agent, view_place);
        seed_physiology(
            &mut view,
            agent,
            HomeostaticNeeds::new(
                permille(400),
                Permille::ZERO,
                Permille::ZERO,
                Permille::ZERO,
                Permille::ZERO,
            ),
            metabolism_with_only(HomeostaticNeedId::Hunger, permille(50)),
        );
        let frame = make_frame(
            IntentionDomain::Travel { destination: dest },
            FrameState::Active,
        );

        let assumptions = populate_assumptions(&frame, agent, &view, Tick(10), Tick(20));
        assert!(
            assumptions.contains(&FrameAssumption::NeedSafeUntilTick {
                need: HomeostaticNeedId::Hunger,
                until_tick: Tick(20),
            }),
            "expected hunger NeedSafeUntilTick assumption, got {assumptions:?}",
        );
    }

    #[test]
    fn populate_omits_need_safe_until_tick_when_breach_after_plan_completion() {
        // hunger=400, hunger_rate=50, hunger.high()=750.
        // breach_tick = 10 + 7 = Tick(17). plan_completion_tick = Tick(15).
        // breach_tick (17) >= plan_completion_tick (15) → no assumption.
        let agent = make_entity(0);
        let view_place = make_entity(10);
        let dest = make_entity(20);
        let mut view = MockBeliefView::new();
        view.alive.insert(agent);
        view.places.insert(agent, view_place);
        seed_physiology(
            &mut view,
            agent,
            HomeostaticNeeds::new(
                permille(400),
                Permille::ZERO,
                Permille::ZERO,
                Permille::ZERO,
                Permille::ZERO,
            ),
            metabolism_with_only(HomeostaticNeedId::Hunger, permille(50)),
        );
        let frame = make_frame(
            IntentionDomain::Travel { destination: dest },
            FrameState::Active,
        );

        let assumptions = populate_assumptions(&frame, agent, &view, Tick(10), Tick(15));
        assert!(
            !assumptions
                .iter()
                .any(|a| matches!(a, FrameAssumption::NeedSafeUntilTick { .. })),
            "expected no NeedSafeUntilTick assumption, got {assumptions:?}",
        );
    }

    #[test]
    fn populate_omits_need_safe_until_tick_when_no_plan() {
        // plan_completion_tick == current_tick (no plan branch). breach_tick is
        // strictly less than plan_completion_tick is impossible because
        // breach_tick >= current_tick by construction → no assumption.
        let agent = make_entity(0);
        let view_place = make_entity(10);
        let dest = make_entity(20);
        let mut view = MockBeliefView::new();
        view.alive.insert(agent);
        view.places.insert(agent, view_place);
        seed_physiology(
            &mut view,
            agent,
            HomeostaticNeeds::new(
                permille(990),
                Permille::ZERO,
                Permille::ZERO,
                Permille::ZERO,
                Permille::ZERO,
            ),
            metabolism_with_only(HomeostaticNeedId::Hunger, permille(50)),
        );
        let frame = make_frame(
            IntentionDomain::Travel { destination: dest },
            FrameState::Active,
        );

        let assumptions = populate_assumptions(&frame, agent, &view, Tick(10), Tick(10));
        assert!(
            !assumptions
                .iter()
                .any(|a| matches!(a, FrameAssumption::NeedSafeUntilTick { .. })),
            "no-plan branch must skip need-horizon assumption, got {assumptions:?}",
        );
    }

    #[test]
    fn populate_skips_need_horizon_when_profile_missing() {
        // No physiology seeded → metabolism_profile/homeostatic_needs/drive_thresholds
        // return None. Need-horizon population is skipped; domain assumptions stand.
        let agent = make_entity(0);
        let view_place = make_entity(10);
        let dest = make_entity(20);
        let mut view = MockBeliefView::new();
        view.alive.insert(agent);
        view.places.insert(agent, view_place);
        let frame = make_frame(
            IntentionDomain::Travel { destination: dest },
            FrameState::Active,
        );

        let assumptions = populate_assumptions(&frame, agent, &view, Tick(0), Tick(1000));
        assert_eq!(
            assumptions,
            vec![FrameAssumption::RouteExists {
                from: view_place,
                to: dest,
            }],
            "missing profile must skip need-horizon population while preserving \
             domain assumptions",
        );
    }

    #[test]
    fn populate_produces_need_safe_until_tick_per_breaching_need() {
        // All five needs at 600, all rates at 100. With DriveThresholds::default
        // each high() is in [700, 800] range; every need projects a breach
        // strictly before a far-future plan_completion_tick. Expect one
        // NeedSafeUntilTick per need (5 entries).
        let agent = make_entity(0);
        let view_place = make_entity(10);
        let dest = make_entity(20);
        let mut view = MockBeliefView::new();
        view.alive.insert(agent);
        view.places.insert(agent, view_place);
        let needs = HomeostaticNeeds::new(
            permille(600),
            permille(600),
            permille(600),
            permille(600),
            permille(600),
        );
        let metabolism = MetabolismProfile {
            hunger_rate: permille(100),
            thirst_rate: permille(100),
            fatigue_rate: permille(100),
            bladder_rate: permille(100),
            dirtiness_rate: permille(100),
            ..MetabolismProfile::default()
        };
        seed_physiology(&mut view, agent, needs, metabolism);
        let frame = make_frame(
            IntentionDomain::Travel { destination: dest },
            FrameState::Active,
        );

        let assumptions = populate_assumptions(&frame, agent, &view, Tick(0), Tick(1000));
        let need_horizon_count = assumptions
            .iter()
            .filter(|a| matches!(a, FrameAssumption::NeedSafeUntilTick { .. }))
            .count();
        assert_eq!(
            need_horizon_count,
            HomeostaticNeedId::ALL.len(),
            "expected one NeedSafeUntilTick per breaching need, got {assumptions:?}",
        );
        for &need in &HomeostaticNeedId::ALL {
            assert!(
                assumptions.contains(&FrameAssumption::NeedSafeUntilTick {
                    need,
                    until_tick: Tick(1000),
                }),
                "missing NeedSafeUntilTick for {need:?}: {assumptions:?}",
            );
        }
    }

    // ── evaluate_assumptions tests ──

    #[test]
    fn target_alive_dead_produces_critical_failure() {
        let dead_entity = make_entity(1);
        let view = MockBeliefView::new(); // entity not in alive set

        let result = evaluate_assumptions(
            &[FrameAssumption::TargetAlive(dead_entity)],
            &view,
            make_entity(0),
            Some(&ordered(&[])),
            Tick(0),
        );
        assert!(matches!(
            result,
            AssumptionEvalResult::CriticalFailure(FrameAssumption::TargetAlive(entity))
                if entity == dead_entity
        ));
    }

    #[test]
    fn route_exists_severed_produces_recoverable_route_blocked() {
        let from = make_entity(10);
        let to = make_entity(20);
        let view = MockBeliefView::new(); // no routes

        let result = evaluate_assumptions(
            &[FrameAssumption::RouteExists { from, to }],
            &view,
            make_entity(0),
            Some(&ordered(&[])),
            Tick(0),
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
            make_entity(0),
            Some(&ordered(&candidates)),
            Tick(0),
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
            make_entity(0),
            Some(&ordered(&[])),
            Tick(0),
        );
        assert_eq!(result, AssumptionEvalResult::AllPass);
    }

    #[test]
    fn no_critical_threat_without_candidates_returns_deferred() {
        let view = MockBeliefView::new();

        let result = evaluate_assumptions(
            &[FrameAssumption::NoCriticalThreat],
            &view,
            make_entity(0),
            None,
            Tick(0),
        );
        assert_eq!(result, AssumptionEvalResult::Deferred);
    }

    #[test]
    fn evaluate_commodity_available_at_returns_critical_failure_when_refuted() {
        let agent = make_entity(0);
        let place = make_entity(10);
        let lot = make_entity(20);
        let mut view = MockBeliefView::new();
        view.places.insert(agent, place);
        view.entities_at.insert(place, vec![lot]);
        view.item_lot_commodities.insert(lot, CommodityKind::Bread);

        let result = evaluate_assumptions(
            &[FrameAssumption::CommodityAvailableAt {
                commodity: CommodityKind::Apple,
                place,
            }],
            &view,
            agent,
            Some(&ordered(&[])),
            Tick(0),
        );
        assert_eq!(
            result,
            AssumptionEvalResult::CriticalFailure(FrameAssumption::CommodityAvailableAt {
                commodity: CommodityKind::Apple,
                place,
            })
        );
    }

    #[test]
    fn evaluate_commodity_available_at_returns_all_pass_when_believed() {
        let agent = make_entity(0);
        let place = make_entity(10);
        let lot = make_entity(20);
        let mut view = MockBeliefView::new();
        view.places.insert(agent, make_entity(11));
        let mut state = observed_entity_state(place);
        state
            .last_known_inventory
            .insert(CommodityKind::Apple, Quantity(2));
        view.belief_stores
            .insert(agent, store_with_known_entity(lot, state));

        let result = evaluate_assumptions(
            &[FrameAssumption::CommodityAvailableAt {
                commodity: CommodityKind::Apple,
                place,
            }],
            &view,
            agent,
            Some(&ordered(&[])),
            Tick(0),
        );
        assert_eq!(result, AssumptionEvalResult::AllPass);
    }

    #[test]
    fn evaluate_commodity_available_at_returns_deferred_when_unknown() {
        let agent = make_entity(0);
        let place = make_entity(10);
        let mut view = MockBeliefView::new();
        view.places.insert(agent, make_entity(11));

        let result = evaluate_assumptions(
            &[FrameAssumption::CommodityAvailableAt {
                commodity: CommodityKind::Apple,
                place,
            }],
            &view,
            agent,
            Some(&ordered(&[])),
            Tick(0),
        );
        assert_eq!(result, AssumptionEvalResult::Deferred);
    }

    #[test]
    fn evaluate_commodity_available_at_co_located_resource_source_returns_all_pass() {
        let agent = make_entity(0);
        let place = make_entity(10);
        let source = make_entity(21);
        let mut view = MockBeliefView::new();
        view.places.insert(agent, place);
        view.entities_at.insert(place, vec![source]);
        view.resource_sources.insert(
            source,
            ResourceSource {
                commodity: CommodityKind::Apple,
                available_quantity: Quantity(3),
                max_quantity: Quantity(5),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            },
        );

        let result = evaluate_assumptions(
            &[FrameAssumption::CommodityAvailableAt {
                commodity: CommodityKind::Apple,
                place,
            }],
            &view,
            agent,
            Some(&ordered(&[])),
            Tick(0),
        );
        assert_eq!(result, AssumptionEvalResult::AllPass);
    }

    // ── evaluate_assumptions need-horizon tests (S126 D5) ──

    #[test]
    fn evaluate_need_safe_until_tick_returns_critical_failure_when_breach_before_until_tick() {
        // hunger=400, hunger_rate=50, hunger.high()=750 (DriveThresholds::default).
        // breach_tick = 10 + ⌈(750-400)/50⌉ = 10 + 7 = Tick(17).
        // until_tick = Tick(20) > breach_tick → CriticalFailure.
        let agent = make_entity(0);
        let mut view = MockBeliefView::new();
        view.alive.insert(agent);
        seed_physiology(
            &mut view,
            agent,
            HomeostaticNeeds::new(
                permille(400),
                Permille::ZERO,
                Permille::ZERO,
                Permille::ZERO,
                Permille::ZERO,
            ),
            metabolism_with_only(HomeostaticNeedId::Hunger, permille(50)),
        );

        let assumption = FrameAssumption::NeedSafeUntilTick {
            need: HomeostaticNeedId::Hunger,
            until_tick: Tick(20),
        };
        let result =
            evaluate_assumptions(&[assumption], &view, agent, Some(&ordered(&[])), Tick(10));
        assert_eq!(
            result,
            AssumptionEvalResult::CriticalFailure(assumption),
            "breach (17) < until_tick (20) must produce CriticalFailure",
        );
    }

    #[test]
    fn evaluate_need_safe_until_tick_returns_all_pass_when_breach_at_or_after_until_tick() {
        // Same setup as above but until_tick = Tick(15).
        // breach_tick = Tick(17) >= until_tick (15) → AllPass.
        let agent = make_entity(0);
        let mut view = MockBeliefView::new();
        view.alive.insert(agent);
        seed_physiology(
            &mut view,
            agent,
            HomeostaticNeeds::new(
                permille(400),
                Permille::ZERO,
                Permille::ZERO,
                Permille::ZERO,
                Permille::ZERO,
            ),
            metabolism_with_only(HomeostaticNeedId::Hunger, permille(50)),
        );

        let result = evaluate_assumptions(
            &[FrameAssumption::NeedSafeUntilTick {
                need: HomeostaticNeedId::Hunger,
                until_tick: Tick(15),
            }],
            &view,
            agent,
            Some(&ordered(&[])),
            Tick(10),
        );
        assert_eq!(
            result,
            AssumptionEvalResult::AllPass,
            "breach (17) >= until_tick (15) must produce AllPass",
        );
    }

    #[test]
    fn evaluate_need_safe_until_tick_returns_deferred_when_profile_missing() {
        // No physiology seeded → metabolism_profile/homeostatic_needs/drive_thresholds
        // return None. The arm marks has_deferred and continues; with no other
        // assumptions to fail, the loop exits Deferred.
        let agent = make_entity(0);
        let view = MockBeliefView::new();

        let result = evaluate_assumptions(
            &[FrameAssumption::NeedSafeUntilTick {
                need: HomeostaticNeedId::Hunger,
                until_tick: Tick(20),
            }],
            &view,
            agent,
            Some(&ordered(&[])),
            Tick(10),
        );
        assert_eq!(result, AssumptionEvalResult::Deferred);
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

        let result = apply_assumption_result(
            &frame,
            &AssumptionEvalResult::CriticalFailure(FrameAssumption::TargetAlive(make_entity(99))),
            Tick(5),
            &mut runtime,
        );
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

        let result = apply_assumption_result(
            &frame,
            &AssumptionEvalResult::AllPass,
            Tick(7),
            &mut runtime,
        );
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

        let result = apply_assumption_result(
            &frame,
            &AssumptionEvalResult::AllPass,
            Tick(7),
            &mut runtime,
        );
        let updated = result;
        assert_eq!(updated.state, FrameState::Active);
        assert_eq!(
            updated.stalled_ticks, 5,
            "stalled_ticks must not be reset on resume"
        );
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
        let result = apply_assumption_result(
            &frame,
            &AssumptionEvalResult::AllPass,
            Tick(7),
            &mut runtime,
        );
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
        assert!(ops.contains(&PlannerOpKind::EstablishCamp));
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
        assert!(ops.contains(&PlannerOpKind::Investigate));
        assert!(ops.contains(&PlannerOpKind::AskWitness));
        assert_eq!(ops.len(), 25);
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

    // ── Regression: record_assumption_failure suppression duration ──────
    //
    // Pre-S109 this path emitted `BlockingFact::AssumptionFailed` recorded
    // with `BlockerClearingCondition::TtlOnly` and a TTL of
    // `structural_block_ticks` (200 by default). The S109 typed-discrepancy
    // migration must preserve that effective suppression duration while still
    // allowing commodity-availability failures to clear on fresh local
    // reavailability. These tests pin the TTL and clearing condition against
    // accidental regression in either direction.

    fn assumption_failure_frame(domain: IntentionDomain) -> IntentionFrame {
        IntentionFrame {
            goal: GoalKey::from(GoalKind::Sleep),
            domain,
            assumptions: Vec::new(),
            state: FrameState::Active,
            established_at: Tick(0),
            last_progress_tick: None,
            stalled_ticks: 0,
            patience_limit: 30,
        }
    }

    fn structural_block_ticks_default() -> u32 {
        worldwake_core::CognitiveProfile::default().structural_block_ticks
    }

    #[test]
    fn record_assumption_failure_uses_structural_block_ticks_with_target() {
        let target = make_entity(7);
        let agent_place = make_entity(1);
        let frame = assumption_failure_frame(IntentionDomain::Care { patient: target });
        let mut memory = worldwake_core::DiscrepancyMemory::default();
        let tick = Tick(50);
        let ttl = structural_block_ticks_default();

        record_assumption_failure(
            &frame,
            Some(agent_place),
            Some(target),
            &mut memory,
            tick,
            ttl,
            FrameAssumption::TargetAlive(target),
        );

        let entry = memory
            .entries
            .values()
            .next()
            .expect("entry recorded with target");
        assert_eq!(entry.expires_tick, Tick(tick.0 + u64::from(ttl)));
        assert_eq!(
            entry.clearing_condition,
            worldwake_core::DiscrepancyClearing::TtlExpiry,
            "with-target assumption failure must clear by TTL, not on \
             ReobservationOf — re-perceiving the target does not validate \
             that the failed plan-level assumption is now resolved"
        );
        assert_eq!(entry.discrepancy, Discrepancy::BeliefContradicted);
        assert_eq!(entry.blocker_key.target, Some(target));
        assert_eq!(entry.blocker_key.place, Some(agent_place));
    }

    #[test]
    fn record_assumption_failure_for_expected_commodity_clears_on_reavailability() {
        let destination = make_entity(7);
        let source = make_entity(11);
        let frame = IntentionFrame {
            goal: GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
            domain: IntentionDomain::Travel { destination },
            assumptions: Vec::new(),
            state: FrameState::Active,
            established_at: Tick(0),
            last_progress_tick: None,
            stalled_ticks: 0,
            patience_limit: 30,
        };
        let mut memory = worldwake_core::DiscrepancyMemory::default();
        let tick = Tick(50);
        let ttl = structural_block_ticks_default();

        record_assumption_failure(
            &frame,
            Some(destination),
            Some(source),
            &mut memory,
            tick,
            ttl,
            FrameAssumption::CommodityAvailableAt {
                commodity: CommodityKind::Apple,
                place: destination,
            },
        );

        let entry = memory
            .entries
            .values()
            .next()
            .expect("entry recorded with commodity expectation");
        assert_eq!(entry.expires_tick, Tick(tick.0 + u64::from(ttl)));
        assert_eq!(
            entry.clearing_condition,
            worldwake_core::DiscrepancyClearing::CommodityAvailabilityChanged {
                commodity: CommodityKind::Apple,
                place: destination,
            }
        );
        assert_eq!(entry.discrepancy, Discrepancy::BeliefContradicted);
        assert_eq!(entry.blocker_key.target, Some(source));
        assert_eq!(entry.blocker_key.place, Some(destination));
    }

    #[test]
    fn record_assumption_failure_uses_structural_block_ticks_without_target() {
        let agent_place = make_entity(1);
        let frame = assumption_failure_frame(IntentionDomain::Generic);
        let mut memory = worldwake_core::DiscrepancyMemory::default();
        let tick = Tick(75);
        let ttl = structural_block_ticks_default();

        record_assumption_failure(
            &frame,
            Some(agent_place),
            None,
            &mut memory,
            tick,
            ttl,
            FrameAssumption::NoCriticalThreat,
        );

        let entry = memory
            .entries
            .values()
            .next()
            .expect("entry recorded without target");
        assert_eq!(entry.expires_tick, Tick(tick.0 + u64::from(ttl)));
        assert_eq!(
            entry.clearing_condition,
            worldwake_core::DiscrepancyClearing::TtlExpiry
        );
        assert_eq!(entry.discrepancy, Discrepancy::PartialExecutionDrift);
        assert_eq!(entry.blocker_key.target, None);
    }

    #[test]
    fn record_assumption_failure_overwrites_prior_entry_for_same_key() {
        let target = make_entity(11);
        let agent_place = make_entity(2);
        let frame = assumption_failure_frame(IntentionDomain::Care { patient: target });
        let mut memory = worldwake_core::DiscrepancyMemory::default();
        let ttl = structural_block_ticks_default();

        record_assumption_failure(
            &frame,
            Some(agent_place),
            Some(target),
            &mut memory,
            Tick(10),
            ttl,
            FrameAssumption::TargetAlive(target),
        );
        record_assumption_failure(
            &frame,
            Some(agent_place),
            Some(target),
            &mut memory,
            Tick(40),
            ttl,
            FrameAssumption::TargetAlive(target),
        );

        // Same blocker_key (goal/place/target) → second record replaces first.
        assert_eq!(memory.entries.len(), 1);
        let entry = memory.entries.values().next().unwrap();
        assert_eq!(entry.observed_tick, Tick(40));
        assert_eq!(entry.expires_tick, Tick(40 + u64::from(ttl)));
    }

    #[test]
    fn record_assumption_failure_writes_need_horizon_exceeded() {
        // NeedSafeUntilTick failure routes to Discrepancy::NeedHorizonExceeded
        // with TtlExpiry clearing — independent of target/commodity framing.
        let target = make_entity(7);
        let agent_place = make_entity(1);
        let frame = assumption_failure_frame(IntentionDomain::Care { patient: target });
        let mut memory = worldwake_core::DiscrepancyMemory::default();
        let tick = Tick(50);
        let ttl = structural_block_ticks_default();

        record_assumption_failure(
            &frame,
            Some(agent_place),
            Some(target),
            &mut memory,
            tick,
            ttl,
            FrameAssumption::NeedSafeUntilTick {
                need: HomeostaticNeedId::Hunger,
                until_tick: Tick(20),
            },
        );

        let entry = memory
            .entries
            .values()
            .next()
            .expect("entry recorded for need-horizon failure");
        assert_eq!(
            entry.discrepancy,
            Discrepancy::NeedHorizonExceeded {
                need: HomeostaticNeedId::Hunger,
                projected_breach_tick: Tick(20),
            },
        );
        assert_eq!(entry.clearing_condition, DiscrepancyClearing::TtlExpiry);
        assert_eq!(entry.expires_tick, Tick(tick.0 + u64::from(ttl)));
        assert_eq!(entry.blocker_key.target, Some(target));
        assert_eq!(entry.blocker_key.place, Some(agent_place));
    }

    #[test]
    fn record_assumption_failure_preserves_commodity_availability_clearing() {
        // Regression guard: passing a CommodityAvailableAt failed_assumption
        // with a frame whose expected_commodity is Some preserves the existing
        // CommodityAvailabilityChanged clearing path.
        let destination = make_entity(7);
        let source = make_entity(11);
        let frame = IntentionFrame {
            goal: GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
            domain: IntentionDomain::Travel { destination },
            assumptions: Vec::new(),
            state: FrameState::Active,
            established_at: Tick(0),
            last_progress_tick: None,
            stalled_ticks: 0,
            patience_limit: 30,
        };
        let mut memory = worldwake_core::DiscrepancyMemory::default();
        let tick = Tick(50);
        let ttl = structural_block_ticks_default();

        record_assumption_failure(
            &frame,
            Some(destination),
            Some(source),
            &mut memory,
            tick,
            ttl,
            FrameAssumption::CommodityAvailableAt {
                commodity: CommodityKind::Apple,
                place: destination,
            },
        );

        let entry = memory
            .entries
            .values()
            .next()
            .expect("entry recorded for commodity-availability failure");
        assert_eq!(entry.discrepancy, Discrepancy::BeliefContradicted);
        assert_eq!(
            entry.clearing_condition,
            DiscrepancyClearing::CommodityAvailabilityChanged {
                commodity: CommodityKind::Apple,
                place: destination,
            },
        );
    }

    #[test]
    fn record_source_invalidation_uses_structural_block_ticks_without_target() {
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let frame = IntentionFrame {
            goal,
            domain: IntentionDomain::Errand {
                destination: make_entity(40),
            },
            assumptions: vec![FrameAssumption::CommodityAvailableAt {
                commodity: CommodityKind::Apple,
                place: make_entity(40),
            }],
            state: FrameState::Exhausted,
            established_at: Tick(1),
            last_progress_tick: None,
            stalled_ticks: 0,
            patience_limit: 3,
        };
        let mut memory = worldwake_core::DiscrepancyMemory::default();
        let tick = Tick(12);
        let ttl = 17;

        record_source_invalidation(&frame, &mut memory, tick, ttl);

        let entry = memory.entries.values().next().unwrap();
        assert_eq!(entry.discrepancy, Discrepancy::SourceInvalidated);
        assert_eq!(entry.blocker_key.goal_key, goal);
        assert_eq!(entry.blocker_key.place, None);
        assert_eq!(entry.blocker_key.target, None);
        assert_eq!(entry.observed_tick, tick);
        assert_eq!(entry.expires_tick, Tick(12 + u64::from(ttl)));
        assert_eq!(entry.clearing_condition, DiscrepancyClearing::TtlExpiry);
    }
}
