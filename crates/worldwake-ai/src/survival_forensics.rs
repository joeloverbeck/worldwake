use crate::{
    AgentDecisionTrace, DecisionOutcome, GoalPriorityClass, PlanAttemptTrace, PlanSearchOutcome,
    RankedGoalProvenance, RankedGoalProvenanceFamily, RankedGoalSummary, SelectedPlanSource,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use worldwake_core::{
    BlockerScope, CommodityKind, DeathCause, DecisionEventPayload, DeprivationKind,
    DriveThresholds, EntityId, EventLog, EventView, ExpectationFailureCauseTag, GoalKey,
    HomeostaticNeedId, HomeostaticNeeds, Permille, PlaceTag, Quantity, SleepFailureCause, Tick,
    WaterQuality, WorkstationTag, World, WoundCause, default_commodity_perish_profile_map,
};
use worldwake_sim::{
    ActionInstance, ActionInstanceId, ActionTraceDetail, ActionTraceEvent, ActionTraceKind,
    ActionTraceSink,
};

const MAX_INTERIOR_FRAMES: usize = 5;
const MAX_COMPETITOR_FRAMES: usize = 3;
const FIRST_LAST_FRAME_COUNT: usize = 5;
const SLEEP_PLACE_TAGS: [PlaceTag; 3] = [PlaceTag::Inn, PlaceTag::Barracks, PlaceTag::Camp];

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CriticalWindowReport {
    pub agent: EntityId,
    pub need: HomeostaticNeedId,
    pub start_tick: Tick,
    pub end_tick: Tick,
    pub threshold: Permille,
    pub peak_value: Permille,
    pub frames: Vec<CriticalWindowFrame>,
    /// Derived signal: this critical window's span contained an exhaustion collapse
    /// (an `Exhaustion` deprivation wound created/worsened, or a fatigue-attributed
    /// death). Recomputable from `WoundList` + `DeadAt`; never authoritative (FND-27).
    #[serde(default)]
    pub exhaustion_collapse_observed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CriticalWindowFrame {
    pub tick: Tick,
    pub need_value: Permille,
    pub selected_goal: Option<GoalKey>,
    pub selected_plan_source: Option<SelectedPlanSource>,
    pub top_competitors: Vec<AgendaEntrySnapshot>,
    pub active_action: Option<ActiveActionSummary>,
    pub exhaustion_state: Option<ExhaustionSummary>,
    pub blocker_summary: Option<BlockerSummary>,
    pub local_authoritative_summary: LocalSurvivalStateSummary,
    #[serde(default)]
    pub failed_rest_opportunities: Vec<FailedRestOpportunity>,
    #[serde(default)]
    pub degraded_self_care_opportunities: Vec<DegradedSelfCareOpportunity>,
    #[serde(default)]
    pub source_acquisition_failures: Vec<SourceAcquisitionFailure>,
    #[serde(default)]
    pub spoiled_food_discoveries: Vec<SpoiledFoodDiscovery>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedRestOpportunity {
    pub tick: Tick,
    pub place: EntityId,
    pub kind: FailedRestKind,
    pub was_rough: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailedRestKind {
    Interrupted { cause: SleepFailureCause },
    PreconditionRejected,
    RoughFallbackToKnownRestSite,
    PreemptedByHigherNeed { need: HomeostaticNeedId },
}

/// S176 D8: derived evidence that a self-care affordance was degraded or
/// blocked by facility wear, and what the agent did about it. Mirrors
/// [`FailedRestOpportunity`]; derived forensic state (FND-27), never
/// authoritative.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DegradedSelfCareOpportunity {
    pub tick: Tick,
    pub facility: EntityId,
    pub cause: DegradedSelfCareCause,
    pub outcome: DegradedSelfCareOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DegradedSelfCareCause {
    BasinTooDirty,
    BasinDry,
    LatrineFull,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DegradedSelfCareOutcome {
    WildernessRelief,
    Cleaned,
    Queued,
    DidNothing,
}

/// S177 D7: derived evidence that an agent's critical thirst window included
/// a failed or rejected source acquisition input. Derived from event history
/// and action trace; never authoritative.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceAcquisitionFailure {
    pub tick: Tick,
    pub source: EntityId,
    pub cause: SourceFailureCause,
    pub outcome: SourceFailureOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SourceFailureCause {
    Depleted,
    QualityRejected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SourceFailureOutcome {
    DrankAnyway,
    TraveledToFallback,
    GaveUp,
}

/// S178 D6: derived evidence that an agent reached a food lot whose believed
/// condition was still edible but whose observed condition is now spoiled.
/// This is forensic state only; it never feeds authoritative simulation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpoiledFoodDiscovery {
    pub tick: Tick,
    pub lot: EntityId,
    pub believed_condition: Permille,
    pub observed_condition: Permille,
    pub outcome: SpoiledFoodOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SpoiledFoodOutcome {
    AteAnyway,
    TraveledToFallback,
    GaveUp,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgendaEntrySnapshot {
    pub goal: GoalKey,
    pub priority_class: GoalPriorityClass,
    pub motive_score: u32,
    pub provenance_family: Option<RankedGoalProvenanceFamily>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActiveActionSummary {
    pub action_name: String,
    pub instance: ActionInstanceId,
    pub started_at: Tick,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ExhaustionSummary {
    FrontierExhausted { expansions_used: u16 },
    BudgetExhausted { expansions_used: u16 },
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockerSummary {
    pub blocker_count: u16,
    pub top_blocker: Option<BlockerScope>,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LocalSurvivalStateSummary {
    pub place: Option<EntityId>,
    pub water_source_present: bool,
    pub wash_basin_present: bool,
    pub latrine_present: bool,
    pub sleep_affordance_present: bool,
    pub food_source_present: bool,
}

impl LocalSurvivalStateSummary {
    #[must_use]
    pub fn capture(world: &World, agent: EntityId) -> Self {
        let Some(place) = world.effective_place(agent) else {
            return Self {
                place: None,
                water_source_present: false,
                wash_basin_present: false,
                latrine_present: false,
                sleep_affordance_present: false,
                food_source_present: false,
            };
        };
        let water_source_present = world.query_resource_source().any(|(entity, source)| {
            world.effective_place(entity) == Some(place)
                && source.commodity == CommodityKind::Water
                && source.available_quantity > Quantity(0)
        });
        let wash_basin_present = world.query_workstation_marker().any(|(entity, marker)| {
            world.effective_place(entity) == Some(place) && marker.0 == WorkstationTag::WashBasin
        });
        // A latrine is present when the place tracks fullness or carries the
        // Latrine tag. Used to attribute wilderness relief to a degraded latrine.
        let latrine_present = world.get_component_latrine_fullness(place).is_some()
            || world.place_has_tag(place, PlaceTag::Latrine);
        let sleep_affordance_present = SLEEP_PLACE_TAGS
            .into_iter()
            .any(|tag| world.place_has_tag(place, tag));
        let food_source_present = world.query_resource_source().any(|(entity, source)| {
            world.effective_place(entity) == Some(place)
                && source.available_quantity > Quantity(0)
                && commodity_is_edible(source.commodity)
        }) || world.query_item_lot().any(|(entity, lot)| {
            world.effective_place(entity) == Some(place)
                && lot.quantity > Quantity(0)
                && commodity_is_edible(lot.commodity)
        });

        Self {
            place: Some(place),
            water_source_present,
            wash_basin_present,
            latrine_present,
            sleep_affordance_present,
            food_source_present,
        }
    }
}

/// Per-tick exhaustion-collapse signal derived from authoritative state.
///
/// Returns `true` when, *this tick*, the agent either gained or worsened an
/// `Exhaustion` deprivation wound (S175 D2) or died with a fatigue-attributed
/// death cause (S175 D4). This is the input the caller threads into
/// [`SurvivalForensicExtractor::observe`] so the active fatigue critical window
/// latches `exhaustion_collapse_observed`. It is a pure read over `WoundList`
/// and `DeadAt`; the flag it feeds is never authoritative (FND-27).
#[must_use]
pub fn exhaustion_collapse_signal(world: &World, agent: EntityId, tick: Tick) -> bool {
    let wound_inflicted = world.get_component_wound_list(agent).is_some_and(|wounds| {
        wounds.wounds.iter().any(|wound| {
            wound.inflicted_at == tick
                && matches!(
                    wound.cause,
                    WoundCause::Deprivation(DeprivationKind::Exhaustion)
                )
        })
    });
    let fatigue_death = world.get_component_dead_at(agent).is_some_and(|dead| {
        dead.tick == tick
            && matches!(
                dead.cause,
                DeathCause::NeedDeprivation {
                    need: HomeostaticNeedId::Fatigue
                }
            )
    });
    wound_inflicted || fatigue_death
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActionTraceSnapshot<'a> {
    pub active_action: Option<ActiveActionSummary>,
    pub tick_events: Vec<&'a ActionTraceEvent>,
}

impl<'a> ActionTraceSnapshot<'a> {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn from_sink(
        actor: EntityId,
        tick: Tick,
        sink: &'a ActionTraceSink,
        active_action: Option<&ActionInstance>,
        active_action_name: Option<&str>,
    ) -> Self {
        Self {
            active_action: active_action
                .zip(active_action_name)
                .map(|(instance, action_name)| ActiveActionSummary {
                    action_name: action_name.to_string(),
                    instance: instance.instance_id,
                    started_at: instance.start_tick,
                }),
            tick_events: sink.events_for_at(actor, tick),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurvivalForensicExtractor {
    agent: EntityId,
    active_windows: BTreeMap<HomeostaticNeedId, WindowBuilder>,
    completed_reports: Vec<CriticalWindowReport>,
}

impl SurvivalForensicExtractor {
    #[must_use]
    pub fn new(agent: EntityId) -> Self {
        Self {
            agent,
            active_windows: BTreeMap::new(),
            completed_reports: Vec::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn observe(
        &mut self,
        tick: Tick,
        needs: &HomeostaticNeeds,
        thresholds: &DriveThresholds,
        decision_trace: Option<&AgentDecisionTrace>,
        action_trace_snapshot: &ActionTraceSnapshot<'_>,
        event_log: &EventLog,
        local_state: &LocalSurvivalStateSummary,
        exhaustion_collapse_signal: bool,
    ) {
        for need in HomeostaticNeedId::ALL {
            let need_value = needs.value(need);
            let threshold = thresholds.critical(need);
            if need_value >= threshold {
                let frame = build_frame(
                    self.agent,
                    tick,
                    need,
                    need_value,
                    decision_trace,
                    action_trace_snapshot,
                    event_log,
                    *local_state,
                );
                let builder = self
                    .active_windows
                    .entry(need)
                    .or_insert_with(|| WindowBuilder::new(tick, threshold));
                builder
                    .update_pending_source_outcomes(source_failure_outcome(action_trace_snapshot));
                builder.update_pending_spoiled_food_outcomes(spoiled_food_outcome(
                    action_trace_snapshot,
                ));
                builder.observe(frame);
                continue;
            }

            if let Some(builder) = self.active_windows.remove(&need) {
                self.completed_reports.push(builder.flush(self.agent, need));
            }
        }

        // Exhaustion collapse is a fatigue consequence; latch it onto the active
        // fatigue window, mirroring how `failed_rest_opportunities` attach only to
        // the fatigue window. The latch persists through `flush`.
        if exhaustion_collapse_signal
            && let Some(builder) = self.active_windows.get_mut(&HomeostaticNeedId::Fatigue)
        {
            builder.exhaustion_collapse_observed = true;
        }
    }

    #[must_use]
    pub fn finalize(mut self) -> Vec<CriticalWindowReport> {
        for (need, builder) in self.active_windows {
            self.completed_reports.push(builder.flush(self.agent, need));
        }
        sort_reports(&mut self.completed_reports);
        self.completed_reports
    }

    #[must_use]
    pub fn top_n_longest(reports: &[CriticalWindowReport], n: usize) -> Vec<&CriticalWindowReport> {
        let mut ranked = reports.iter().collect::<Vec<_>>();
        ranked.sort_by(|left, right| compare_reports(left, right));
        ranked.truncate(n);
        ranked
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WindowBuilder {
    start_tick: Tick,
    threshold: Permille,
    peak_value: Permille,
    frames: Vec<CriticalWindowFrame>,
    exhaustion_collapse_observed: bool,
}

impl WindowBuilder {
    fn new(start_tick: Tick, threshold: Permille) -> Self {
        Self {
            start_tick,
            threshold,
            peak_value: Permille::ZERO,
            frames: Vec::new(),
            exhaustion_collapse_observed: false,
        }
    }

    fn observe(&mut self, frame: CriticalWindowFrame) {
        self.peak_value = self.peak_value.max(frame.need_value);
        self.frames.push(frame);
    }

    fn update_pending_source_outcomes(&mut self, outcome: SourceFailureOutcome) {
        if outcome == SourceFailureOutcome::GaveUp {
            return;
        }
        for frame in &mut self.frames {
            for failure in &mut frame.source_acquisition_failures {
                if failure.outcome == SourceFailureOutcome::GaveUp {
                    failure.outcome = outcome;
                }
            }
        }
    }

    fn update_pending_spoiled_food_outcomes(&mut self, outcome: SpoiledFoodOutcome) {
        if outcome == SpoiledFoodOutcome::GaveUp {
            return;
        }
        for frame in &mut self.frames {
            for discovery in &mut frame.spoiled_food_discoveries {
                if discovery.outcome == SpoiledFoodOutcome::GaveUp {
                    discovery.outcome = outcome;
                }
            }
        }
    }

    fn flush(self, agent: EntityId, need: HomeostaticNeedId) -> CriticalWindowReport {
        let end_tick = self
            .frames
            .last()
            .map_or(self.start_tick, |frame| frame.tick);
        CriticalWindowReport {
            agent,
            need,
            start_tick: self.start_tick,
            end_tick,
            threshold: self.threshold,
            peak_value: self.peak_value,
            frames: bounded_frames(self.frames),
            exhaustion_collapse_observed: self.exhaustion_collapse_observed,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_frame(
    agent: EntityId,
    tick: Tick,
    need: HomeostaticNeedId,
    need_value: Permille,
    decision_trace: Option<&AgentDecisionTrace>,
    action_trace_snapshot: &ActionTraceSnapshot<'_>,
    event_log: &EventLog,
    local_state: LocalSurvivalStateSummary,
) -> CriticalWindowFrame {
    let selected_goal = decision_trace.and_then(selected_goal_from_trace);
    let selected_plan_source = decision_trace.and_then(selected_plan_source_from_trace);
    let top_competitors = decision_trace
        .map(|trace| top_competitors_from_trace(trace, selected_goal))
        .unwrap_or_default();
    let active_action = action_trace_snapshot.active_action.clone();
    let exhaustion_state =
        decision_trace.and_then(|trace| exhaustion_summary(trace, selected_goal));
    let blocker_summary = decision_trace.and_then(|trace| blocker_summary(trace, selected_goal));

    CriticalWindowFrame {
        tick,
        need_value,
        selected_goal,
        selected_plan_source,
        top_competitors,
        active_action,
        exhaustion_state,
        blocker_summary,
        local_authoritative_summary: local_state,
        failed_rest_opportunities: failed_rest_opportunities(
            tick,
            need,
            decision_trace,
            action_trace_snapshot,
            local_state,
        ),
        degraded_self_care_opportunities: degraded_self_care_opportunities(
            tick,
            need,
            action_trace_snapshot,
            local_state,
        ),
        source_acquisition_failures: source_acquisition_failures(
            tick,
            need,
            agent,
            event_log,
            action_trace_snapshot,
        ),
        spoiled_food_discoveries: spoiled_food_discoveries(
            tick,
            need,
            agent,
            event_log,
            action_trace_snapshot,
        ),
    }
}

fn spoiled_food_discoveries(
    tick: Tick,
    active_need: HomeostaticNeedId,
    agent: EntityId,
    event_log: &EventLog,
    action_trace_snapshot: &ActionTraceSnapshot<'_>,
) -> Vec<SpoiledFoodDiscovery> {
    if active_need != HomeostaticNeedId::Hunger {
        return Vec::new();
    }

    let profiles = default_commodity_perish_profile_map();
    let outcome = spoiled_food_outcome(action_trace_snapshot);
    event_log
        .events_at_tick(tick)
        .iter()
        .filter_map(|event_id| event_log.get(*event_id))
        .filter_map(|event| match event.decision_payload()? {
            DecisionEventPayload::LotConditionExpectationMismatch(payload)
                if payload.observer == agent =>
            {
                let profile = profiles.get(&payload.commodity)?;
                (payload.believed_condition.value() >= profile.spoiled_threshold.value()
                    && payload.observed_condition.value() < profile.spoiled_threshold.value())
                .then_some(SpoiledFoodDiscovery {
                    tick,
                    lot: payload.lot,
                    believed_condition: payload.believed_condition,
                    observed_condition: payload.observed_condition,
                    outcome,
                })
            }
            _ => None,
        })
        .collect()
}

fn spoiled_food_outcome(action_trace_snapshot: &ActionTraceSnapshot<'_>) -> SpoiledFoodOutcome {
    if action_trace_snapshot.tick_events.iter().any(|event| {
        event.action_name == "eat"
            && matches!(
                event.kind,
                ActionTraceKind::Started { .. } | ActionTraceKind::Committed { .. }
            )
    }) {
        return SpoiledFoodOutcome::AteAnyway;
    }
    if action_trace_snapshot.tick_events.iter().any(|event| {
        event.action_name == "travel"
            && matches!(
                event.kind,
                ActionTraceKind::Started { .. } | ActionTraceKind::Committed { .. }
            )
    }) {
        return SpoiledFoodOutcome::TraveledToFallback;
    }
    SpoiledFoodOutcome::GaveUp
}

fn source_acquisition_failures(
    tick: Tick,
    active_need: HomeostaticNeedId,
    agent: EntityId,
    event_log: &EventLog,
    action_trace_snapshot: &ActionTraceSnapshot<'_>,
) -> Vec<SourceAcquisitionFailure> {
    if active_need != HomeostaticNeedId::Thirst {
        return Vec::new();
    }

    let outcome = source_failure_outcome(action_trace_snapshot);
    event_log
        .events_at_tick(tick)
        .iter()
        .filter_map(|event_id| event_log.get(*event_id))
        .filter_map(|event| match event.decision_payload()? {
            DecisionEventPayload::SourceExpectationFailure(payload)
                if payload.agent == agent
                    && payload.cause == ExpectationFailureCauseTag::SourceDepletedLocally =>
            {
                Some(SourceAcquisitionFailure {
                    tick,
                    source: payload.source.entity,
                    cause: SourceFailureCause::Depleted,
                    outcome,
                })
            }
            DecisionEventPayload::ResourceSourceQualityObserved(payload)
                if payload.observer == agent && payload.quality != WaterQuality::Clean =>
            {
                Some(SourceAcquisitionFailure {
                    tick,
                    source: payload.source.entity,
                    cause: SourceFailureCause::QualityRejected,
                    outcome,
                })
            }
            _ => None,
        })
        .collect()
}

fn source_failure_outcome(action_trace_snapshot: &ActionTraceSnapshot<'_>) -> SourceFailureOutcome {
    if action_trace_snapshot.tick_events.iter().any(|event| {
        event.action_name == "drink" && matches!(event.kind, ActionTraceKind::Committed { .. })
    }) {
        return SourceFailureOutcome::DrankAnyway;
    }
    if action_trace_snapshot.tick_events.iter().any(|event| {
        event.action_name == "travel"
            && matches!(
                event.kind,
                ActionTraceKind::Started { .. } | ActionTraceKind::Committed { .. }
            )
    }) {
        return SourceFailureOutcome::TraveledToFallback;
    }
    SourceFailureOutcome::GaveUp
}

/// S176 D8: derive degraded/blocked self-care evidence for the active window.
/// Basin degradation attaches to the Dirtiness window; latrine degradation to
/// the Bladder window — mirroring how `failed_rest_opportunities` attaches only
/// to the Fatigue window. Signals are read from the tick's action-trace events:
/// committed recovery/fallback actions (`clean_wash_basin` / `empty_latrine` /
/// `relieve_wilderness`) and start-failed self-care rejected by a degradation gate.
fn degraded_self_care_opportunities(
    tick: Tick,
    active_need: HomeostaticNeedId,
    action_trace_snapshot: &ActionTraceSnapshot<'_>,
    local_state: LocalSurvivalStateSummary,
) -> Vec<DegradedSelfCareOpportunity> {
    let Some(facility) = local_state.place else {
        return Vec::new();
    };
    action_trace_snapshot
        .tick_events
        .iter()
        .filter_map(|event| {
            let committed = matches!(event.kind, ActionTraceKind::Committed { .. });
            let start_failed = matches!(event.kind, ActionTraceKind::StartFailed { .. });
            let cause_outcome = match active_need {
                HomeostaticNeedId::Dirtiness => match event.action_name.as_str() {
                    "clean_wash_basin" if committed => Some((
                        DegradedSelfCareCause::BasinTooDirty,
                        DegradedSelfCareOutcome::Cleaned,
                    )),
                    "wash" if start_failed => match start_failure_reason(event) {
                        Some(reason) if reason.contains("TargetWashBasinNotTooDirty") => Some((
                            DegradedSelfCareCause::BasinTooDirty,
                            DegradedSelfCareOutcome::DidNothing,
                        )),
                        Some(reason) if reason.contains("TargetHasWashBasinClean") => Some((
                            DegradedSelfCareCause::BasinDry,
                            DegradedSelfCareOutcome::DidNothing,
                        )),
                        _ => None,
                    },
                    _ => None,
                },
                HomeostaticNeedId::Bladder => match event.action_name.as_str() {
                    "empty_latrine" if committed => Some((
                        DegradedSelfCareCause::LatrineFull,
                        DegradedSelfCareOutcome::Cleaned,
                    )),
                    // Wilderness relief is degraded self-care only when a latrine
                    // is present here — otherwise it is ordinary outdoor relief.
                    "relieve_wilderness" if committed && local_state.latrine_present => Some((
                        DegradedSelfCareCause::LatrineFull,
                        DegradedSelfCareOutcome::WildernessRelief,
                    )),
                    "toilet" if start_failed => start_failure_reason(event)
                        .filter(|reason| reason.contains("PlaceLatrineNotFull"))
                        .map(|_| {
                            (
                                DegradedSelfCareCause::LatrineFull,
                                DegradedSelfCareOutcome::DidNothing,
                            )
                        }),
                    _ => None,
                },
                _ => None,
            };
            cause_outcome.map(|(cause, outcome)| DegradedSelfCareOpportunity {
                tick,
                facility,
                cause,
                outcome,
            })
        })
        .collect()
}

fn start_failure_reason(event: &ActionTraceEvent) -> Option<&str> {
    match &event.kind {
        ActionTraceKind::StartFailed { reason, .. } => Some(reason.as_str()),
        _ => None,
    }
}

fn failed_rest_opportunities(
    tick: Tick,
    active_need: HomeostaticNeedId,
    decision_trace: Option<&AgentDecisionTrace>,
    action_trace_snapshot: &ActionTraceSnapshot<'_>,
    local_state: LocalSurvivalStateSummary,
) -> Vec<FailedRestOpportunity> {
    if active_need != HomeostaticNeedId::Fatigue {
        return Vec::new();
    }

    let mut opportunities = action_trace_snapshot
        .tick_events
        .iter()
        .filter_map(|event| failed_rest_opportunity_from_trace_event(tick, event, local_state))
        .collect::<Vec<_>>();
    if let Some(opportunity) = preempted_sleep_failed_rest(tick, decision_trace, local_state) {
        opportunities.push(opportunity);
    }
    opportunities.extend(rough_fallback_failed_rest(tick, decision_trace));
    opportunities
}

fn failed_rest_opportunity_from_trace_event(
    tick: Tick,
    event: &ActionTraceEvent,
    local_state: LocalSurvivalStateSummary,
) -> Option<FailedRestOpportunity> {
    match &event.detail {
        Some(ActionTraceDetail::SleepInterrupted {
            place,
            cause,
            was_rough_sleep,
            ..
        }) => Some(FailedRestOpportunity {
            tick,
            place: *place,
            kind: FailedRestKind::Interrupted { cause: *cause },
            was_rough: *was_rough_sleep,
        }),
        _ if event.action_name == "sleep"
            && matches!(event.kind, ActionTraceKind::StartFailed { .. })
            && sleep_start_failure_is_rest_site_precondition(event) =>
        {
            Some(FailedRestOpportunity {
                tick,
                place: local_state.place?,
                kind: FailedRestKind::PreconditionRejected,
                was_rough: false,
            })
        }
        _ => None,
    }
}

fn sleep_start_failure_is_rest_site_precondition(event: &ActionTraceEvent) -> bool {
    let ActionTraceKind::StartFailed { reason, .. } = &event.kind else {
        return false;
    };
    reason.contains("rest site") && reason.contains("full")
}

fn preempted_sleep_failed_rest(
    tick: Tick,
    decision_trace: Option<&AgentDecisionTrace>,
    local_state: LocalSurvivalStateSummary,
) -> Option<FailedRestOpportunity> {
    let DecisionOutcome::Planning(planning) = &decision_trace?.outcome else {
        return None;
    };
    let switch = planning.selection.goal_switch.as_ref()?;
    if switch.from.kind != worldwake_core::GoalKind::Sleep {
        return None;
    }
    let selected_goal = planning.selection.selected_goal()?;
    let need = goal_kind_need(selected_goal.kind)?;
    if need == HomeostaticNeedId::Fatigue {
        return None;
    }

    Some(FailedRestOpportunity {
        tick,
        place: local_state.place?,
        kind: FailedRestKind::PreemptedByHigherNeed { need },
        was_rough: false,
    })
}

fn rough_fallback_failed_rest(
    tick: Tick,
    decision_trace: Option<&AgentDecisionTrace>,
) -> Vec<FailedRestOpportunity> {
    let Some(trace) = decision_trace else {
        return Vec::new();
    };
    let DecisionOutcome::Planning(planning) = &trace.outcome else {
        return Vec::new();
    };
    let selected = planning.selection.selected_opportunity;
    let selected_rough_sleep = selected.is_some_and(|opportunity| {
        opportunity.goal_key.kind == worldwake_core::GoalKind::Sleep
            && opportunity.anchor == crate::OpportunityAnchor::None
    });
    if !selected_rough_sleep {
        return Vec::new();
    }

    planning
        .candidates
        .generated
        .iter()
        .filter_map(|opportunity| {
            if opportunity.goal_key.kind != worldwake_core::GoalKind::Sleep {
                return None;
            }
            let crate::OpportunityAnchor::Place(place) = opportunity.anchor else {
                return None;
            };
            Some(FailedRestOpportunity {
                tick,
                place,
                kind: FailedRestKind::RoughFallbackToKnownRestSite,
                was_rough: true,
            })
        })
        .collect()
}

fn goal_kind_need(kind: worldwake_core::GoalKind) -> Option<HomeostaticNeedId> {
    match kind {
        worldwake_core::GoalKind::AcquireCommodity { commodity, .. }
        | worldwake_core::GoalKind::ConsumeOwnedCommodity { commodity } => {
            if commodity == CommodityKind::Water {
                Some(HomeostaticNeedId::Thirst)
            } else if commodity_is_edible(commodity) {
                Some(HomeostaticNeedId::Hunger)
            } else {
                None
            }
        }
        worldwake_core::GoalKind::Relieve => Some(HomeostaticNeedId::Bladder),
        worldwake_core::GoalKind::Wash => Some(HomeostaticNeedId::Dirtiness),
        _ => None,
    }
}

fn bounded_frames(frames: Vec<CriticalWindowFrame>) -> Vec<CriticalWindowFrame> {
    if frames.len() <= FIRST_LAST_FRAME_COUNT * 2 {
        return frames;
    }

    let len = frames.len();
    let interior_end = len.saturating_sub(FIRST_LAST_FRAME_COUNT);
    let stride = ((len.saturating_sub(FIRST_LAST_FRAME_COUNT * 2)) / MAX_INTERIOR_FRAMES).max(1);
    let mut keep = vec![false; len];

    for slot in keep.iter_mut().take(FIRST_LAST_FRAME_COUNT.min(len)) {
        *slot = true;
    }
    for slot in keep
        .iter_mut()
        .take(len)
        .skip(len.saturating_sub(FIRST_LAST_FRAME_COUNT))
    {
        *slot = true;
    }

    let mut interior_idx = FIRST_LAST_FRAME_COUNT;
    let mut interior_kept = 0usize;
    while interior_idx < interior_end && interior_kept < MAX_INTERIOR_FRAMES {
        keep[interior_idx] = true;
        interior_idx += stride;
        interior_kept += 1;
    }

    for idx in 1..len {
        if frame_change_detected(&frames[idx - 1], &frames[idx]) {
            keep[idx] = true;
        }
    }

    frames
        .into_iter()
        .enumerate()
        .filter_map(|(idx, frame)| keep[idx].then_some(frame))
        .collect()
}

fn frame_change_detected(previous: &CriticalWindowFrame, current: &CriticalWindowFrame) -> bool {
    previous.selected_goal != current.selected_goal
        || previous.selected_plan_source != current.selected_plan_source
        || previous.active_action != current.active_action
        || previous.exhaustion_state != current.exhaustion_state
        || previous.blocker_summary != current.blocker_summary
        || previous.failed_rest_opportunities != current.failed_rest_opportunities
        || previous.degraded_self_care_opportunities != current.degraded_self_care_opportunities
        || previous.source_acquisition_failures != current.source_acquisition_failures
}

fn selected_goal_from_trace(trace: &AgentDecisionTrace) -> Option<GoalKey> {
    match &trace.outcome {
        DecisionOutcome::Planning(planning) => planning.selection.selected_goal(),
        DecisionOutcome::Dead | DecisionOutcome::ActiveAction { .. } => None,
    }
}

fn selected_plan_source_from_trace(trace: &AgentDecisionTrace) -> Option<SelectedPlanSource> {
    match &trace.outcome {
        DecisionOutcome::Planning(planning) => planning.selection.selected_plan_source,
        DecisionOutcome::Dead | DecisionOutcome::ActiveAction { .. } => None,
    }
}

fn top_competitors_from_trace(
    trace: &AgentDecisionTrace,
    selected_goal: Option<GoalKey>,
) -> Vec<AgendaEntrySnapshot> {
    let DecisionOutcome::Planning(planning) = &trace.outcome else {
        return Vec::new();
    };
    let selected_opportunity = planning.selection.selected_opportunity;
    planning
        .candidates
        .ranked
        .iter()
        .filter(|summary| {
            if let Some(opportunity) = selected_opportunity {
                return summary.opportunity != opportunity;
            }
            Some(summary.opportunity.goal_key) != selected_goal
        })
        .take(MAX_COMPETITOR_FRAMES)
        .map(ranked_goal_snapshot)
        .collect()
}

fn exhaustion_summary(
    trace: &AgentDecisionTrace,
    selected_goal: Option<GoalKey>,
) -> Option<ExhaustionSummary> {
    let goal = selected_goal?;
    let DecisionOutcome::Planning(planning) = &trace.outcome else {
        return None;
    };
    let selected_anchor = planning
        .selection
        .selected_opportunity
        .map(|opportunity| opportunity.anchor);
    let attempt = planning.planning.attempts.iter().find(|attempt| {
        attempt.goal == goal
            && selected_anchor.is_none_or(|anchor| attempt.opportunity_anchor == anchor)
    })?;
    exhaustion_summary_from_attempt(attempt)
}

fn exhaustion_summary_from_attempt(attempt: &PlanAttemptTrace) -> Option<ExhaustionSummary> {
    match attempt.outcome {
        PlanSearchOutcome::FrontierExhausted { expansions_used } => {
            Some(ExhaustionSummary::FrontierExhausted { expansions_used })
        }
        PlanSearchOutcome::BudgetExhausted { expansions_used } => {
            Some(ExhaustionSummary::BudgetExhausted { expansions_used })
        }
        PlanSearchOutcome::Unsupported => Some(ExhaustionSummary::Unsupported),
        PlanSearchOutcome::Found { .. } => None,
    }
}

fn blocker_summary(
    trace: &AgentDecisionTrace,
    selected_goal: Option<GoalKey>,
) -> Option<BlockerSummary> {
    let DecisionOutcome::Planning(planning) = &trace.outcome else {
        return None;
    };
    let selected_goal = selected_goal?;
    let matching = planning
        .discrepancy_trace
        .iter()
        .filter(|discrepancy| discrepancy.scope.exact_goal_key().unwrap() == selected_goal)
        .collect::<Vec<_>>();
    if matching.is_empty() {
        return None;
    }
    let top = matching[0];
    Some(BlockerSummary {
        blocker_count: matching.len() as u16,
        top_blocker: Some(top.scope),
    })
}

fn ranked_goal_snapshot(summary: &RankedGoalSummary) -> AgendaEntrySnapshot {
    AgendaEntrySnapshot {
        goal: summary.opportunity.goal_key,
        priority_class: summary.priority_class,
        motive_score: summary.motive_score,
        provenance_family: summary.provenance.as_ref().map(provenance_family),
    }
}

fn provenance_family(provenance: &RankedGoalProvenance) -> RankedGoalProvenanceFamily {
    match provenance {
        RankedGoalProvenance::Danger(_) => RankedGoalProvenanceFamily::Danger,
        RankedGoalProvenance::Drive(_) => RankedGoalProvenanceFamily::Drive,
    }
}

fn commodity_is_edible(commodity: CommodityKind) -> bool {
    commodity
        .spec()
        .consumable_profile
        .is_some_and(|profile| profile.hunger_relief_per_unit.value() > 0)
}

fn report_duration(report: &CriticalWindowReport) -> u64 {
    report
        .end_tick
        .0
        .saturating_sub(report.start_tick.0)
        .saturating_add(1)
}

fn compare_reports(
    left: &CriticalWindowReport,
    right: &CriticalWindowReport,
) -> std::cmp::Ordering {
    report_duration(right)
        .cmp(&report_duration(left))
        .then_with(|| left.start_tick.cmp(&right.start_tick))
        .then_with(|| left.end_tick.cmp(&right.end_tick))
        .then_with(|| left.need.cmp(&right.need))
        .then_with(|| left.agent.cmp(&right.agent))
}

fn sort_reports(reports: &mut [CriticalWindowReport]) {
    reports.sort_by(compare_reports);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ActionStartFailureSummary, CandidateTrace, DirtySet, ExecutionTrace, ExhaustionTraceEntry,
        GoalSwitchSummary, OpportunityKey, PatrolRouteSnapshotTrace, PlanSearchTrace,
        PlanningPipelineTrace, SelectedPlanTrace, SelectionTrace, decision_trace::DiscrepancyTrace,
    };
    use worldwake_core::{
        AcquisitionQuantity, ActionDefId, CauseRef, CommodityPurpose, ControlSource,
        DriveThresholds, EventPayload, EventTag, ExpectationFailurePhaseTag, GoalKind,
        OpportunityAnchor, OpportunityExpectationKindTag, PendingEvent, PrototypePlace, Quantity,
        ResourceSource, ResourceSourceQualityObservedPayload, SleepFailureCause,
        SourceAttributionOutcomeTag, SourceExpectationFailurePayload, SourceKeyPayload,
        VisibilitySpec, WitnessData, WorkstationMarker, WorldTxn, build_prototype_world,
        prototype_place_entity,
    };
    use worldwake_sim::{ActionTraceDetail, ActionTraceEvent, ActionTraceKind, CommitOutcome};

    #[test]
    fn detects_window_start_and_end_ticks() {
        let agent = entity(1);
        let mut extractor = SurvivalForensicExtractor::new(agent);
        let thresholds = DriveThresholds::default();
        let local = sample_local_summary();

        for tick in 1..=20 {
            let hunger = if (6..=15).contains(&tick) { 910 } else { 100 };
            extractor.observe(
                Tick(tick),
                &HomeostaticNeeds::new(pm(hunger), pm(0), pm(0), pm(0), pm(0)),
                &thresholds,
                None,
                &ActionTraceSnapshot::empty(),
                &EventLog::new(),
                &local,
                false,
            );
        }

        let reports = extractor.finalize();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].need, HomeostaticNeedId::Hunger);
        assert_eq!(reports[0].start_tick, Tick(6));
        assert_eq!(reports[0].end_tick, Tick(15));
        assert_eq!(reports[0].peak_value, pm(910));
    }

    #[test]
    fn bounded_frame_capture_uses_expected_ticks_for_long_windows() {
        let agent = entity(1);
        let mut extractor = SurvivalForensicExtractor::new(agent);
        let thresholds = DriveThresholds::default();
        let local = sample_local_summary();

        for tick in 1..=100 {
            extractor.observe(
                Tick(tick),
                &HomeostaticNeeds::new(pm(920), pm(0), pm(0), pm(0), pm(0)),
                &thresholds,
                None,
                &ActionTraceSnapshot::empty(),
                &EventLog::new(),
                &local,
                false,
            );
        }

        let reports = extractor.finalize();
        let ticks = reports[0]
            .frames
            .iter()
            .map(|frame| frame.tick.0)
            .collect::<Vec<_>>();
        assert_eq!(
            ticks,
            vec![1, 2, 3, 4, 5, 6, 24, 42, 60, 78, 96, 97, 98, 99, 100]
        );
    }

    #[test]
    fn change_point_capture_retains_goal_switch_ticks() {
        let agent = entity(1);
        let mut extractor = SurvivalForensicExtractor::new(agent);
        let thresholds = DriveThresholds::default();
        let local = sample_local_summary();

        for tick in 1..=20 {
            let goal = if tick < 13 {
                GoalKey::from(GoalKind::Sleep)
            } else {
                GoalKey::from(GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: worldwake_core::CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                })
            };
            extractor.observe(
                Tick(tick),
                &HomeostaticNeeds::new(pm(0), pm(0), pm(930), pm(0), pm(0)),
                &thresholds,
                Some(&planning_trace(agent, Tick(tick), goal)),
                &ActionTraceSnapshot::empty(),
                &EventLog::new(),
                &local,
                false,
            );
        }

        let reports = extractor.finalize();
        let ticks = reports[0]
            .frames
            .iter()
            .map(|frame| frame.tick.0)
            .collect::<Vec<_>>();
        assert!(
            ticks.contains(&13),
            "change point tick should be retained: {ticks:?}"
        );
    }

    #[test]
    fn summaries_populate_from_trace_inputs() {
        let agent = entity(1);
        let mut extractor = SurvivalForensicExtractor::new(agent);
        let thresholds = DriveThresholds::default();
        let goal = GoalKey::from(GoalKind::Sleep);
        let trace = planning_trace_with_exhaustion_and_blocker(agent, Tick(4), goal);
        let action_snapshot = ActionTraceSnapshot {
            active_action: Some(ActiveActionSummary {
                action_name: "sleep".to_string(),
                instance: ActionInstanceId(9),
                started_at: Tick(2),
            }),
            tick_events: Vec::new(),
        };

        extractor.observe(
            Tick(4),
            &HomeostaticNeeds::new(pm(0), pm(0), pm(930), pm(0), pm(0)),
            &thresholds,
            Some(&trace),
            &action_snapshot,
            &EventLog::new(),
            &sample_local_summary(),
            false,
        );

        let reports = extractor.finalize();
        let frame = &reports[0].frames[0];
        assert_eq!(
            frame.exhaustion_state,
            Some(ExhaustionSummary::BudgetExhausted { expansions_used: 7 })
        );
        assert_eq!(
            frame.blocker_summary,
            Some(BlockerSummary {
                blocker_count: 1,
                top_blocker: Some(
                    worldwake_core::BlockerKey {
                        goal_key: goal,
                        place: Some(entity(40)),
                        target: Some(entity(41)),
                        action_def: Some(ActionDefId(5)),
                    }
                    .into()
                ),
            })
        );
        assert_eq!(
            frame.active_action,
            Some(ActiveActionSummary {
                action_name: "sleep".to_string(),
                instance: ActionInstanceId(9),
                started_at: Tick(2),
            })
        );
    }

    #[test]
    fn failed_rest_types_cover_interrupted_precondition_fallback_and_preemption_kinds() {
        let interrupted = FailedRestOpportunity {
            tick: Tick(7),
            place: entity(50),
            kind: FailedRestKind::Interrupted {
                cause: SleepFailureCause::HostileProximity,
            },
            was_rough: true,
        };
        assert_eq!(interrupted.tick, Tick(7));
        assert!(matches!(
            interrupted.kind,
            FailedRestKind::Interrupted {
                cause: SleepFailureCause::HostileProximity
            }
        ));
        assert_eq!(
            FailedRestKind::PreconditionRejected,
            FailedRestKind::PreconditionRejected
        );
        assert_eq!(
            FailedRestKind::RoughFallbackToKnownRestSite,
            FailedRestKind::RoughFallbackToKnownRestSite
        );
        assert_eq!(
            FailedRestKind::PreemptedByHigherNeed {
                need: HomeostaticNeedId::Thirst,
            },
            FailedRestKind::PreemptedByHigherNeed {
                need: HomeostaticNeedId::Thirst,
            }
        );
    }

    fn committed_event(action_name: &str) -> ActionTraceEvent {
        ActionTraceEvent {
            tick: Tick(5),
            sequence_in_tick: 0,
            actor: entity(1),
            def_id: ActionDefId(0),
            action_name: action_name.to_string(),
            detail: None,
            kind: ActionTraceKind::Committed {
                instance_id: ActionInstanceId(1),
                outcome: CommitOutcome::empty(),
            },
        }
    }

    #[test]
    fn degraded_self_care_records_cleaning_emptying_and_wilderness_fallback() {
        let basin_place = LocalSurvivalStateSummary {
            place: Some(entity(70)),
            water_source_present: true,
            wash_basin_present: true,
            latrine_present: false,
            sleep_affordance_present: false,
            food_source_present: false,
        };
        let latrine_place = LocalSurvivalStateSummary {
            place: Some(entity(71)),
            water_source_present: false,
            wash_basin_present: false,
            latrine_present: true,
            sleep_affordance_present: false,
            food_source_present: false,
        };

        // clean_wash_basin in the Dirtiness window → BasinTooDirty / Cleaned.
        let clean_event = committed_event("clean_wash_basin");
        let clean_snapshot = ActionTraceSnapshot {
            active_action: None,
            tick_events: vec![&clean_event],
        };
        let cleaned = degraded_self_care_opportunities(
            Tick(5),
            HomeostaticNeedId::Dirtiness,
            &clean_snapshot,
            basin_place,
        );
        assert_eq!(
            cleaned,
            vec![DegradedSelfCareOpportunity {
                tick: Tick(5),
                facility: entity(70),
                cause: DegradedSelfCareCause::BasinTooDirty,
                outcome: DegradedSelfCareOutcome::Cleaned,
            }]
        );

        // empty_latrine in the Bladder window → LatrineFull / Cleaned.
        let empty_event = committed_event("empty_latrine");
        let empty_snapshot = ActionTraceSnapshot {
            active_action: None,
            tick_events: vec![&empty_event],
        };
        let emptied = degraded_self_care_opportunities(
            Tick(5),
            HomeostaticNeedId::Bladder,
            &empty_snapshot,
            latrine_place,
        );
        assert_eq!(emptied[0].cause, DegradedSelfCareCause::LatrineFull);
        assert_eq!(emptied[0].outcome, DegradedSelfCareOutcome::Cleaned);

        // relieve_wilderness with a latrine present → LatrineFull / WildernessRelief.
        let wild_event = committed_event("relieve_wilderness");
        let wild_snapshot = ActionTraceSnapshot {
            active_action: None,
            tick_events: vec![&wild_event],
        };
        let wild = degraded_self_care_opportunities(
            Tick(5),
            HomeostaticNeedId::Bladder,
            &wild_snapshot,
            latrine_place,
        );
        assert_eq!(wild[0].outcome, DegradedSelfCareOutcome::WildernessRelief);

        // relieve_wilderness with NO latrine present → not degraded self-care.
        let no_latrine = degraded_self_care_opportunities(
            Tick(5),
            HomeostaticNeedId::Bladder,
            &wild_snapshot,
            basin_place,
        );
        assert!(no_latrine.is_empty());
    }

    #[test]
    fn source_acquisition_failure_serialization_roundtrip() {
        let samples = [
            SourceAcquisitionFailure {
                tick: Tick(1),
                source: entity(10),
                cause: SourceFailureCause::Depleted,
                outcome: SourceFailureOutcome::GaveUp,
            },
            SourceAcquisitionFailure {
                tick: Tick(2),
                source: entity(11),
                cause: SourceFailureCause::QualityRejected,
                outcome: SourceFailureOutcome::DrankAnyway,
            },
            SourceAcquisitionFailure {
                tick: Tick(3),
                source: entity(12),
                cause: SourceFailureCause::QualityRejected,
                outcome: SourceFailureOutcome::TraveledToFallback,
            },
        ];

        for sample in samples {
            let encoded = bincode::serialize(&sample).unwrap();
            let decoded: SourceAcquisitionFailure = bincode::deserialize(&encoded).unwrap();
            assert_eq!(decoded, sample);
        }
    }

    #[test]
    fn source_acquisition_failure_depleted_cause_from_expectation_failure_event() {
        let agent = entity(1);
        let source = entity(80);
        let event_log = event_log_with_decision_payload(
            Tick(5),
            agent,
            EventTag::SourceExpectationFailure,
            DecisionEventPayload::SourceExpectationFailure(source_expectation_failure_payload(
                agent, source,
            )),
        );
        let mut extractor = SurvivalForensicExtractor::new(agent);

        extractor.observe(
            Tick(5),
            &HomeostaticNeeds::new(pm(0), pm(930), pm(0), pm(0), pm(0)),
            &DriveThresholds::default(),
            None,
            &ActionTraceSnapshot::empty(),
            &event_log,
            &sample_local_summary(),
            false,
        );

        let reports = extractor.finalize();
        assert_eq!(
            reports[0].frames[0].source_acquisition_failures,
            vec![SourceAcquisitionFailure {
                tick: Tick(5),
                source,
                cause: SourceFailureCause::Depleted,
                outcome: SourceFailureOutcome::GaveUp,
            }]
        );
    }

    #[test]
    fn source_acquisition_failure_quality_rejected_cause_from_quality_observed_event() {
        let agent = entity(1);
        let source = entity(81);
        let event_log = event_log_with_decision_payload(
            Tick(6),
            agent,
            EventTag::ResourceSourceQualityObserved,
            DecisionEventPayload::ResourceSourceQualityObserved(
                ResourceSourceQualityObservedPayload {
                    observer: agent,
                    source: SourceKeyPayload {
                        entity: source,
                        commodity: CommodityKind::Water,
                    },
                    quality: WaterQuality::Muddy,
                    observed_at_tick: Tick(6),
                },
            ),
        );
        let drink_event = committed_event("drink");
        let snapshot = ActionTraceSnapshot {
            active_action: None,
            tick_events: vec![&drink_event],
        };
        let mut extractor = SurvivalForensicExtractor::new(agent);

        extractor.observe(
            Tick(6),
            &HomeostaticNeeds::new(pm(0), pm(930), pm(0), pm(0), pm(0)),
            &DriveThresholds::default(),
            None,
            &snapshot,
            &event_log,
            &sample_local_summary(),
            false,
        );

        let reports = extractor.finalize();
        assert_eq!(
            reports[0].frames[0].source_acquisition_failures,
            vec![SourceAcquisitionFailure {
                tick: Tick(6),
                source,
                cause: SourceFailureCause::QualityRejected,
                outcome: SourceFailureOutcome::DrankAnyway,
            }]
        );
    }

    #[test]
    fn source_acquisition_failure_travel_and_clean_negative_cases() {
        let agent = entity(1);
        let muddy_source = entity(82);
        let clean_source = entity(83);
        let mut event_log = EventLog::new();
        emit_decision_payload(
            &mut event_log,
            Tick(7),
            agent,
            EventTag::ResourceSourceQualityObserved,
            DecisionEventPayload::ResourceSourceQualityObserved(
                ResourceSourceQualityObservedPayload {
                    observer: agent,
                    source: SourceKeyPayload {
                        entity: muddy_source,
                        commodity: CommodityKind::Water,
                    },
                    quality: WaterQuality::Stale,
                    observed_at_tick: Tick(7),
                },
            ),
        );
        emit_decision_payload(
            &mut event_log,
            Tick(7),
            agent,
            EventTag::ResourceSourceQualityObserved,
            DecisionEventPayload::ResourceSourceQualityObserved(
                ResourceSourceQualityObservedPayload {
                    observer: agent,
                    source: SourceKeyPayload {
                        entity: clean_source,
                        commodity: CommodityKind::Water,
                    },
                    quality: WaterQuality::Clean,
                    observed_at_tick: Tick(7),
                },
            ),
        );
        let travel_event = ActionTraceEvent::new(
            Tick(7),
            agent,
            ActionDefId(6),
            "travel".to_string(),
            ActionTraceKind::Started {
                targets: vec![entity(90)],
            },
        );
        let snapshot = ActionTraceSnapshot {
            active_action: None,
            tick_events: vec![&travel_event],
        };
        let mut extractor = SurvivalForensicExtractor::new(agent);

        extractor.observe(
            Tick(7),
            &HomeostaticNeeds::new(pm(0), pm(930), pm(0), pm(0), pm(0)),
            &DriveThresholds::default(),
            None,
            &snapshot,
            &event_log,
            &sample_local_summary(),
            false,
        );

        let reports = extractor.finalize();
        assert_eq!(
            reports[0].frames[0].source_acquisition_failures,
            vec![SourceAcquisitionFailure {
                tick: Tick(7),
                source: muddy_source,
                cause: SourceFailureCause::QualityRejected,
                outcome: SourceFailureOutcome::TraveledToFallback,
            }]
        );
    }

    #[test]
    fn source_acquisition_failure_updates_prior_gave_up_when_window_later_travels() {
        let agent = entity(1);
        let source = entity(84);
        let failure_log = event_log_with_decision_payload(
            Tick(5),
            agent,
            EventTag::SourceExpectationFailure,
            DecisionEventPayload::SourceExpectationFailure(source_expectation_failure_payload(
                agent, source,
            )),
        );
        let travel_log = EventLog::new();
        let travel_event = ActionTraceEvent::new(
            Tick(6),
            agent,
            ActionDefId(6),
            "travel".to_string(),
            ActionTraceKind::Started {
                targets: vec![entity(91)],
            },
        );
        let travel_snapshot = ActionTraceSnapshot {
            active_action: None,
            tick_events: vec![&travel_event],
        };
        let mut extractor = SurvivalForensicExtractor::new(agent);
        let critical_thirst = HomeostaticNeeds::new(pm(0), pm(930), pm(0), pm(0), pm(0));

        extractor.observe(
            Tick(5),
            &critical_thirst,
            &DriveThresholds::default(),
            None,
            &ActionTraceSnapshot::empty(),
            &failure_log,
            &sample_local_summary(),
            false,
        );
        extractor.observe(
            Tick(6),
            &critical_thirst,
            &DriveThresholds::default(),
            None,
            &travel_snapshot,
            &travel_log,
            &sample_local_summary(),
            false,
        );

        let reports = extractor.finalize();
        assert_eq!(
            reports[0].frames[0].source_acquisition_failures,
            vec![SourceAcquisitionFailure {
                tick: Tick(5),
                source,
                cause: SourceFailureCause::Depleted,
                outcome: SourceFailureOutcome::TraveledToFallback,
            }]
        );
    }

    #[test]
    fn spoiled_food_discovery_recorded_when_belief_fresh_and_observed_spoiled() {
        let agent = entity(1);
        let lot = entity(85);
        let event_log =
            event_log_with_decision_payload(
                Tick(5),
                agent,
                EventTag::ExpectationMismatch,
                DecisionEventPayload::LotConditionExpectationMismatch(
                    lot_condition_mismatch_payload(agent, lot, pm(900), pm(200)),
                ),
            );
        let mut extractor = SurvivalForensicExtractor::new(agent);

        extractor.observe(
            Tick(5),
            &HomeostaticNeeds::new(pm(930), pm(0), pm(0), pm(0), pm(0)),
            &DriveThresholds::default(),
            None,
            &ActionTraceSnapshot::empty(),
            &event_log,
            &sample_local_summary(),
            false,
        );

        let reports = extractor.finalize();
        assert_eq!(
            reports[0].frames[0].spoiled_food_discoveries,
            vec![SpoiledFoodDiscovery {
                tick: Tick(5),
                lot,
                believed_condition: pm(900),
                observed_condition: pm(200),
                outcome: SpoiledFoodOutcome::GaveUp,
            }]
        );
    }

    #[test]
    fn spoiled_food_discovery_outcome_ate_anyway_when_agent_eats_spoiled_lot() {
        let agent = entity(1);
        let lot = entity(86);
        let event_log =
            event_log_with_decision_payload(
                Tick(5),
                agent,
                EventTag::ExpectationMismatch,
                DecisionEventPayload::LotConditionExpectationMismatch(
                    lot_condition_mismatch_payload(agent, lot, pm(900), pm(200)),
                ),
            );
        let eat_event = committed_event("eat");
        let snapshot = ActionTraceSnapshot {
            active_action: None,
            tick_events: vec![&eat_event],
        };
        let mut extractor = SurvivalForensicExtractor::new(agent);

        extractor.observe(
            Tick(5),
            &HomeostaticNeeds::new(pm(930), pm(0), pm(0), pm(0), pm(0)),
            &DriveThresholds::default(),
            None,
            &snapshot,
            &event_log,
            &sample_local_summary(),
            false,
        );

        let reports = extractor.finalize();
        assert_eq!(
            reports[0].frames[0].spoiled_food_discoveries[0].outcome,
            SpoiledFoodOutcome::AteAnyway
        );
        assert_eq!(reports[0].frames[0].spoiled_food_discoveries[0].lot, lot);
    }

    #[test]
    fn spoiled_food_discovery_outcome_traveled_to_fallback_when_agent_seeks_other_food() {
        let agent = entity(1);
        let lot = entity(87);
        let discovery_log =
            event_log_with_decision_payload(
                Tick(5),
                agent,
                EventTag::ExpectationMismatch,
                DecisionEventPayload::LotConditionExpectationMismatch(
                    lot_condition_mismatch_payload(agent, lot, pm(900), pm(200)),
                ),
            );
        let travel_log = EventLog::new();
        let travel_event = ActionTraceEvent::new(
            Tick(6),
            agent,
            ActionDefId(6),
            "travel".to_string(),
            ActionTraceKind::Started {
                targets: vec![entity(90)],
            },
        );
        let travel_snapshot = ActionTraceSnapshot {
            active_action: None,
            tick_events: vec![&travel_event],
        };
        let mut extractor = SurvivalForensicExtractor::new(agent);
        let critical_hunger = HomeostaticNeeds::new(pm(930), pm(0), pm(0), pm(0), pm(0));

        extractor.observe(
            Tick(5),
            &critical_hunger,
            &DriveThresholds::default(),
            None,
            &ActionTraceSnapshot::empty(),
            &discovery_log,
            &sample_local_summary(),
            false,
        );
        extractor.observe(
            Tick(6),
            &critical_hunger,
            &DriveThresholds::default(),
            None,
            &travel_snapshot,
            &travel_log,
            &sample_local_summary(),
            false,
        );

        let reports = extractor.finalize();
        assert_eq!(
            reports[0].frames[0].spoiled_food_discoveries[0].outcome,
            SpoiledFoodOutcome::TraveledToFallback
        );
    }

    #[test]
    fn spoiled_food_discovery_outcome_gave_up_when_agent_idles_past_window() {
        let agent = entity(1);
        let lot = entity(88);
        let event_log =
            event_log_with_decision_payload(
                Tick(5),
                agent,
                EventTag::ExpectationMismatch,
                DecisionEventPayload::LotConditionExpectationMismatch(
                    lot_condition_mismatch_payload(agent, lot, pm(900), pm(200)),
                ),
            );
        let mut extractor = SurvivalForensicExtractor::new(agent);
        let critical_hunger = HomeostaticNeeds::new(pm(930), pm(0), pm(0), pm(0), pm(0));

        extractor.observe(
            Tick(5),
            &critical_hunger,
            &DriveThresholds::default(),
            None,
            &ActionTraceSnapshot::empty(),
            &event_log,
            &sample_local_summary(),
            false,
        );
        extractor.observe(
            Tick(6),
            &critical_hunger,
            &DriveThresholds::default(),
            None,
            &ActionTraceSnapshot::empty(),
            &EventLog::new(),
            &sample_local_summary(),
            false,
        );

        let reports = extractor.finalize();
        assert_eq!(
            reports[0].frames[0].spoiled_food_discoveries[0].outcome,
            SpoiledFoodOutcome::GaveUp
        );
    }

    #[test]
    fn spoiled_food_discovery_does_not_fire_without_prior_belief() {
        let agent = entity(1);
        let mut extractor = SurvivalForensicExtractor::new(agent);

        extractor.observe(
            Tick(5),
            &HomeostaticNeeds::new(pm(930), pm(0), pm(0), pm(0), pm(0)),
            &DriveThresholds::default(),
            None,
            &ActionTraceSnapshot::empty(),
            &EventLog::new(),
            &sample_local_summary(),
            false,
        );

        let reports = extractor.finalize();
        assert!(reports[0].frames[0].spoiled_food_discoveries.is_empty());
    }

    #[test]
    fn critical_window_frame_deserializes_missing_failed_rest_as_empty() {
        let mut frame_value = serde_json::to_value(build_frame(
            entity(1),
            Tick(3),
            HomeostaticNeedId::Fatigue,
            pm(930),
            None,
            &ActionTraceSnapshot::empty(),
            &EventLog::new(),
            sample_local_summary(),
        ))
        .unwrap();
        frame_value
            .as_object_mut()
            .unwrap()
            .remove("failed_rest_opportunities");

        let frame: CriticalWindowFrame = serde_json::from_value(frame_value).unwrap();
        assert!(frame.failed_rest_opportunities.is_empty());
    }

    #[test]
    fn critical_window_report_deserializes_missing_exhaustion_collapse_as_false() {
        let mut report_value =
            serde_json::to_value(report(entity(1), HomeostaticNeedId::Fatigue, 3, 7)).unwrap();
        report_value
            .as_object_mut()
            .unwrap()
            .remove("exhaustion_collapse_observed");

        let report: CriticalWindowReport = serde_json::from_value(report_value).unwrap();
        assert!(!report.exhaustion_collapse_observed);
    }

    #[test]
    fn exhaustion_collapse_signal_latches_onto_active_fatigue_window() {
        let agent = entity(1);
        let mut extractor = SurvivalForensicExtractor::new(agent);
        let thresholds = DriveThresholds::default();
        let critical_fatigue = HomeostaticNeeds::new(pm(0), pm(0), pm(930), pm(0), pm(0));

        // Tick 1: fatigue critical, no collapse yet.
        extractor.observe(
            Tick(1),
            &critical_fatigue,
            &thresholds,
            None,
            &ActionTraceSnapshot::empty(),
            &EventLog::new(),
            &sample_local_summary(),
            false,
        );
        // Tick 2: collapse signal fires (exhaustion wound created / fatigue death).
        extractor.observe(
            Tick(2),
            &critical_fatigue,
            &thresholds,
            None,
            &ActionTraceSnapshot::empty(),
            &EventLog::new(),
            &sample_local_summary(),
            true,
        );

        let reports = extractor.finalize();
        let fatigue_report = reports
            .iter()
            .find(|report| report.need == HomeostaticNeedId::Fatigue)
            .expect("fatigue critical window should be recorded");
        assert!(
            fatigue_report.exhaustion_collapse_observed,
            "fatigue window must latch the collapse signal"
        );
    }

    #[test]
    fn exhaustion_collapse_flag_stays_false_when_window_recovers() {
        let agent = entity(1);
        let mut extractor = SurvivalForensicExtractor::new(agent);
        let thresholds = DriveThresholds::default();
        let critical_fatigue = HomeostaticNeeds::new(pm(0), pm(0), pm(930), pm(0), pm(0));
        let recovered_fatigue = HomeostaticNeeds::new(pm(0), pm(0), pm(100), pm(0), pm(0));

        // Two critical ticks with no collapse signal, then recovery below critical
        // flushes the window.
        extractor.observe(
            Tick(1),
            &critical_fatigue,
            &thresholds,
            None,
            &ActionTraceSnapshot::empty(),
            &EventLog::new(),
            &sample_local_summary(),
            false,
        );
        extractor.observe(
            Tick(2),
            &critical_fatigue,
            &thresholds,
            None,
            &ActionTraceSnapshot::empty(),
            &EventLog::new(),
            &sample_local_summary(),
            false,
        );
        extractor.observe(
            Tick(3),
            &recovered_fatigue,
            &thresholds,
            None,
            &ActionTraceSnapshot::empty(),
            &EventLog::new(),
            &sample_local_summary(),
            false,
        );

        let reports = extractor.finalize();
        let fatigue_report = reports
            .iter()
            .find(|report| report.need == HomeostaticNeedId::Fatigue)
            .expect("fatigue critical window should be recorded");
        assert!(
            !fatigue_report.exhaustion_collapse_observed,
            "a window that recovers without collapse must report false"
        );
    }

    #[test]
    fn exhaustion_collapse_signal_helper_reads_wound_and_death_state() {
        use worldwake_core::{
            BodyPart, CauseRef, ControlSource, DeadAt, Tick as CoreTick, VisibilitySpec,
            WitnessData, WorldTxn, Wound, WoundCause, WoundId, WoundList, build_prototype_world,
        };

        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = {
            let mut txn = WorldTxn::new(
                &mut world,
                CoreTick(1),
                CauseRef::Bootstrap,
                None,
                None,
                VisibilitySpec::Hidden,
                WitnessData::default(),
            );
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let mut log = worldwake_core::EventLog::new();
            let _ = txn.commit(&mut log);
            agent
        };

        // No wound, no death -> no signal.
        assert!(!exhaustion_collapse_signal(&world, agent, CoreTick(5)));

        // Exhaustion wound inflicted on tick 5 -> signal on tick 5 only.
        {
            let mut txn = WorldTxn::new(
                &mut world,
                CoreTick(5),
                CauseRef::Bootstrap,
                None,
                None,
                VisibilitySpec::Hidden,
                WitnessData::default(),
            );
            txn.set_component_wound_list(
                agent,
                WoundList {
                    wounds: vec![Wound {
                        id: WoundId(1),
                        body_part: BodyPart::Torso,
                        cause: WoundCause::Deprivation(DeprivationKind::Exhaustion),
                        severity: pm(500),
                        inflicted_at: CoreTick(5),
                        bleed_rate_per_tick: pm(0),
                    }],
                },
            )
            .unwrap();
            let mut log = worldwake_core::EventLog::new();
            let _ = txn.commit(&mut log);
        }
        assert!(exhaustion_collapse_signal(&world, agent, CoreTick(5)));
        assert!(!exhaustion_collapse_signal(&world, agent, CoreTick(6)));

        // A non-exhaustion (starvation) wound never triggers the signal.
        {
            let mut txn = WorldTxn::new(
                &mut world,
                CoreTick(7),
                CauseRef::Bootstrap,
                None,
                None,
                VisibilitySpec::Hidden,
                WitnessData::default(),
            );
            txn.set_component_wound_list(
                agent,
                WoundList {
                    wounds: vec![Wound {
                        id: WoundId(2),
                        body_part: BodyPart::Torso,
                        cause: WoundCause::Deprivation(DeprivationKind::Starvation),
                        severity: pm(500),
                        inflicted_at: CoreTick(7),
                        bleed_rate_per_tick: pm(0),
                    }],
                },
            )
            .unwrap();
            let mut log = worldwake_core::EventLog::new();
            let _ = txn.commit(&mut log);
        }
        assert!(!exhaustion_collapse_signal(&world, agent, CoreTick(7)));

        // Fatigue-attributed death on tick 9 -> signal on tick 9.
        {
            let mut txn = WorldTxn::new(
                &mut world,
                CoreTick(9),
                CauseRef::Bootstrap,
                None,
                None,
                VisibilitySpec::Hidden,
                WitnessData::default(),
            );
            txn.set_component_dead_at(
                agent,
                DeadAt {
                    tick: CoreTick(9),
                    cause: DeathCause::NeedDeprivation {
                        need: HomeostaticNeedId::Fatigue,
                    },
                },
            )
            .unwrap();
            let mut log = worldwake_core::EventLog::new();
            let _ = txn.commit(&mut log);
        }
        assert!(exhaustion_collapse_signal(&world, agent, CoreTick(9)));
    }

    #[test]
    fn critical_fatigue_window_records_sleep_interruption_failed_rest() {
        let agent = entity(1);
        let place = entity(50);
        let mut extractor = SurvivalForensicExtractor::new(agent);
        let thresholds = DriveThresholds::default();
        let event = ActionTraceEvent::new(
            Tick(4),
            agent,
            ActionDefId(5),
            "sleep".to_string(),
            ActionTraceKind::Aborted {
                instance_id: ActionInstanceId(9),
                reason: "Interrupted".to_string(),
            },
        )
        .with_detail(Some(ActionTraceDetail::SleepInterrupted {
            place,
            cause: SleepFailureCause::HostileProximity,
            accumulated_recovery: pm(40),
            was_rough_sleep: false,
        }));
        let snapshot = ActionTraceSnapshot {
            active_action: None,
            tick_events: vec![&event],
        };

        extractor.observe(
            Tick(4),
            &HomeostaticNeeds::new(pm(0), pm(0), pm(930), pm(0), pm(0)),
            &thresholds,
            None,
            &snapshot,
            &EventLog::new(),
            &sample_local_summary(),
            false,
        );

        let reports = extractor.finalize();
        assert_eq!(
            reports[0].frames[0].failed_rest_opportunities,
            vec![FailedRestOpportunity {
                tick: Tick(4),
                place,
                kind: FailedRestKind::Interrupted {
                    cause: SleepFailureCause::HostileProximity,
                },
                was_rough: false,
            }]
        );
    }

    #[test]
    fn critical_fatigue_window_records_rest_site_start_rejection_failed_rest() {
        let agent = entity(1);
        let mut extractor = SurvivalForensicExtractor::new(agent);
        let thresholds = DriveThresholds::default();
        let event = ActionTraceEvent::new(
            Tick(8),
            agent,
            ActionDefId(5),
            "sleep".to_string(),
            ActionTraceKind::StartFailed {
                reason: "PreconditionFailed(\"rest site is full\")".to_string(),
                request: worldwake_sim::ResolvedRequestTrace {
                    attempt: worldwake_sim::RequestAttemptTrace {
                        input_sequence_no: 1,
                        provenance: worldwake_sim::RequestProvenance::AiPlan,
                    },
                    binding: worldwake_sim::RequestBindingKind::BestEffortFallback,
                },
                legality: None,
            },
        );
        let snapshot = ActionTraceSnapshot {
            active_action: None,
            tick_events: vec![&event],
        };
        let local = sample_local_summary();

        extractor.observe(
            Tick(8),
            &HomeostaticNeeds::new(pm(0), pm(0), pm(930), pm(0), pm(0)),
            &thresholds,
            None,
            &snapshot,
            &EventLog::new(),
            &local,
            false,
        );

        let reports = extractor.finalize();
        assert_eq!(
            reports[0].frames[0].failed_rest_opportunities,
            vec![FailedRestOpportunity {
                tick: Tick(8),
                place: local.place.unwrap(),
                kind: FailedRestKind::PreconditionRejected,
                was_rough: false,
            }]
        );
    }

    #[test]
    fn critical_fatigue_window_records_rough_fallback_from_known_rest_site() {
        let agent = entity(1);
        let shelter = entity(50);
        let mut extractor = SurvivalForensicExtractor::new(agent);
        let thresholds = DriveThresholds::default();
        let goal = GoalKey::from(GoalKind::Sleep);
        let mut trace = planning_trace(agent, Tick(10), goal);
        let DecisionOutcome::Planning(planning) = &mut trace.outcome else {
            unreachable!();
        };
        planning.candidates.generated = vec![
            OpportunityKey {
                goal_key: goal,
                anchor: OpportunityAnchor::Place(shelter),
            },
            OpportunityKey {
                goal_key: goal,
                anchor: OpportunityAnchor::None,
            },
        ];
        planning.selection.selected_opportunity = Some(OpportunityKey {
            goal_key: goal,
            anchor: OpportunityAnchor::None,
        });

        extractor.observe(
            Tick(10),
            &HomeostaticNeeds::new(pm(0), pm(0), pm(930), pm(0), pm(0)),
            &thresholds,
            Some(&trace),
            &ActionTraceSnapshot::empty(),
            &EventLog::new(),
            &sample_local_summary(),
            false,
        );

        let reports = extractor.finalize();
        assert_eq!(
            reports[0].frames[0].failed_rest_opportunities,
            vec![FailedRestOpportunity {
                tick: Tick(10),
                place: shelter,
                kind: FailedRestKind::RoughFallbackToKnownRestSite,
                was_rough: true,
            }]
        );
    }

    #[test]
    fn critical_fatigue_window_records_sleep_preempted_by_higher_need() {
        let agent = entity(1);
        let mut extractor = SurvivalForensicExtractor::new(agent);
        let thresholds = DriveThresholds::default();
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: worldwake_core::CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let trace = planning_trace(agent, Tick(9), goal);
        let local = sample_local_summary();

        extractor.observe(
            Tick(9),
            &HomeostaticNeeds::new(pm(0), pm(930), pm(930), pm(0), pm(0)),
            &thresholds,
            Some(&trace),
            &ActionTraceSnapshot::empty(),
            &EventLog::new(),
            &local,
            false,
        );

        let reports = extractor.finalize();
        let fatigue_report = reports
            .iter()
            .find(|report| report.need == HomeostaticNeedId::Fatigue)
            .expect("fatigue critical window should be recorded");
        assert_eq!(
            fatigue_report.frames[0].failed_rest_opportunities,
            vec![FailedRestOpportunity {
                tick: Tick(9),
                place: local.place.unwrap(),
                kind: FailedRestKind::PreemptedByHigherNeed {
                    need: HomeostaticNeedId::Thirst,
                },
                was_rough: false,
            }]
        );
    }

    #[test]
    fn noncritical_sleep_interruption_does_not_emit_failed_rest_report() {
        let agent = entity(1);
        let event = ActionTraceEvent::new(
            Tick(4),
            agent,
            ActionDefId(5),
            "sleep".to_string(),
            ActionTraceKind::Aborted {
                instance_id: ActionInstanceId(9),
                reason: "Interrupted".to_string(),
            },
        )
        .with_detail(Some(ActionTraceDetail::SleepInterrupted {
            place: entity(50),
            cause: SleepFailureCause::Generic,
            accumulated_recovery: Permille::ZERO,
            was_rough_sleep: true,
        }));
        let snapshot = ActionTraceSnapshot {
            active_action: None,
            tick_events: vec![&event],
        };
        let mut extractor = SurvivalForensicExtractor::new(agent);

        extractor.observe(
            Tick(4),
            &HomeostaticNeeds::new(pm(0), pm(0), pm(400), pm(0), pm(0)),
            &DriveThresholds::default(),
            None,
            &snapshot,
            &EventLog::new(),
            &sample_local_summary(),
            false,
        );

        assert!(extractor.finalize().is_empty());
    }

    #[test]
    fn top_n_longest_orders_ties_deterministically() {
        let agent = entity(1);
        let reports = vec![
            report(agent, HomeostaticNeedId::Thirst, 10, 14),
            report(agent, HomeostaticNeedId::Hunger, 5, 9),
            report(agent, HomeostaticNeedId::Fatigue, 3, 4),
        ];

        let top = SurvivalForensicExtractor::top_n_longest(&reports, 2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].need, HomeostaticNeedId::Hunger);
        assert_eq!(top[1].need, HomeostaticNeedId::Thirst);
    }

    #[test]
    fn local_survival_state_summary_reads_authoritative_local_affordances() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = prototype_place_entity(PrototypePlace::CommonHouse);
        let mut txn = WorldTxn::new(
            &mut world,
            Tick(1),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::PublicRecord,
            WitnessData::default(),
        );
        let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.set_ground_location(agent, place).unwrap();

        let basin = txn.create_entity(worldwake_core::EntityKind::Facility);
        txn.set_ground_location(basin, place).unwrap();
        txn.set_component_workstation_marker(basin, WorkstationMarker(WorkstationTag::WashBasin))
            .unwrap();

        let well = txn.create_entity(worldwake_core::EntityKind::Facility);
        txn.set_ground_location(well, place).unwrap();
        txn.set_component_resource_source(
            well,
            ResourceSource {
                commodity: CommodityKind::Water,
                available_quantity: Quantity(5),
                max_quantity: Quantity(5),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                quality: None,
            },
        )
        .unwrap();

        let bread = txn
            .create_item_lot(CommodityKind::Bread, Quantity(2))
            .unwrap();
        txn.set_ground_location(bread, place).unwrap();
        txn.commit(&mut worldwake_core::EventLog::new());

        let summary = LocalSurvivalStateSummary::capture(&world, agent);
        assert_eq!(summary.place, Some(place));
        assert!(summary.water_source_present);
        assert!(summary.wash_basin_present);
        assert!(summary.sleep_affordance_present);
        assert!(summary.food_source_present);
    }

    #[test]
    fn local_survival_state_summary_capture_marks_in_transit_agents_without_place() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut txn = WorldTxn::new(
            &mut world,
            Tick(1),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::PublicRecord,
            WitnessData::default(),
        );
        let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.commit(&mut worldwake_core::EventLog::new());

        let summary = LocalSurvivalStateSummary::capture(&world, agent);
        assert_eq!(summary.place, None);
        assert!(!summary.water_source_present);
        assert!(!summary.wash_basin_present);
        assert!(!summary.sleep_affordance_present);
        assert!(!summary.food_source_present);
    }

    fn planning_trace(agent: EntityId, tick: Tick, goal: GoalKey) -> AgentDecisionTrace {
        AgentDecisionTrace {
            agent,
            tick,
            compiled_opportunities: Vec::new(),
            opportunity_compiler_load: None,
            snapshot_admissions: None,
            snapshot_cache_counters: None,
            planning_state_cache_counters: None,
            repair_attempts: Vec::new(),
            partial_plan_resumes: Vec::new(),
            causal_link_cap_hits: Vec::new(),
            outcome: DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
                affordances: None,
                dirty: DirtySet::default(),
                plan_continued: false,
                candidates: CandidateTrace {
                    generated: vec![OpportunityKey {
                        goal_key: goal,
                        anchor: OpportunityAnchor::None,
                    }],
                    evidence: Vec::new(),
                    fully_blocked_desires: Vec::new(),
                    places_reachable: 0,
                    places_after_belief_filter: 0,
                    ranked: vec![RankedGoalSummary {
                        opportunity: OpportunityKey {
                            goal_key: goal,
                            anchor: OpportunityAnchor::None,
                        },
                        priority_class: GoalPriorityClass::Critical,
                        motive_score: 900,
                        motive_source_contributions: Vec::new(),
                        provenance: None,
                        source_reliability_discount: None,
                        competition_discount: None,
                        learned_opportunity_bonus: None,
                        repair_memory_bonus: None,
                        source_composite: None,
                        feasibility: crate::FeasibilityHint::Likely,
                        acquisition_quantity: None,
                        artifact_axes: None,
                    }],
                    top_ranked_comparison: None,
                    suppressed: Vec::new(),
                    damped: Vec::new(),
                    zero_motive: Vec::new(),
                    omitted_political: Vec::new(),
                    omitted_bandit: Vec::new(),
                    omitted_social: Vec::new(),
                    omitted_testimony: Vec::new(),
                    omitted_violation_detection: Vec::new(),
                },
                planning: PlanSearchTrace {
                    attempts: vec![PlanAttemptTrace {
                        goal,
                        opportunity_anchor: OpportunityAnchor::None,
                        outcome: PlanSearchOutcome::Found {
                            steps: Vec::new(),
                            terminal_kind: crate::PlanTerminalKind::GoalSatisfied,
                        },
                        goal_budget: worldwake_core::GoalPlanningBudget::TRAVEL_PURCHASE,
                        strategic_budget: None,
                        strategic_plan: None,
                        tactical_goal: None,
                        landmarks_extracted: 0,
                        landmark_orderings: 0,
                        target_belief_presence:
                            crate::decision_trace::TargetBeliefPresence::NotApplicable,
                        method_trace: None,
                        binding_rejections: Vec::new(),
                        expansion_summaries: Vec::new(),
                    }],
                    same_goal_trace: None,
                },
                selection: SelectionTrace {
                    selected_opportunity: Some(OpportunityKey {
                        goal_key: goal,
                        anchor: OpportunityAnchor::None,
                    }),
                    selected_plan: Some(SelectedPlanTrace {
                        steps: Vec::new(),
                        terminal_kind: crate::PlanTerminalKind::GoalSatisfied,
                        next_step_index: None,
                        next_step: None,
                        search_provenance: None,
                        primary_motive: 900,
                        total_value: 900,
                        side_benefits: Vec::new(),
                    }),
                    selected_plan_source: Some(SelectedPlanSource::SearchSelection),
                    goal_switch: Some(GoalSwitchSummary {
                        from: GoalKey::from(GoalKind::Sleep),
                        to: goal,
                        kind: crate::GoalSwitchKind::HigherPriorityGoal,
                    }),
                    previous_goal: None,
                    plan_replacement: None,
                    snapshot_continuation: None,
                },
                portfolio: None,
                execution: ExecutionTrace {
                    enqueued_step: None,
                    revalidation_passed: None,
                    failure: None,
                },
                action_start_failures: Vec::<ActionStartFailureSummary>::new(),
                discrepancy_trace: Vec::<DiscrepancyTrace>::new(),
                exhaustion_snapshot: Vec::<ExhaustionTraceEntry>::new(),
                frame_transition: None,
                patrol_route: PatrolRouteSnapshotTrace::default(),
                selected_patrol_anchor: None,
                pursuit_invalidation: None,
            })),
        }
    }

    fn planning_trace_with_exhaustion_and_blocker(
        agent: EntityId,
        tick: Tick,
        goal: GoalKey,
    ) -> AgentDecisionTrace {
        let mut trace = planning_trace(agent, tick, goal);
        let DecisionOutcome::Planning(planning) = &mut trace.outcome else {
            unreachable!();
        };
        planning.planning.attempts[0].outcome =
            PlanSearchOutcome::BudgetExhausted { expansions_used: 7 };
        planning.discrepancy_trace.push(DiscrepancyTrace {
            discrepancy: worldwake_core::Discrepancy::ImproperPlanningState,
            scope: worldwake_core::BlockerKey {
                goal_key: goal,
                place: Some(entity(40)),
                target: Some(entity(41)),
                action_def: Some(ActionDefId(5)),
            }
            .into(),
            expires_tick: Tick(9),
        });
        trace
    }

    fn report(
        agent: EntityId,
        need: HomeostaticNeedId,
        start_tick: u32,
        end_tick: u32,
    ) -> CriticalWindowReport {
        CriticalWindowReport {
            agent,
            need,
            start_tick: Tick(u64::from(start_tick)),
            end_tick: Tick(u64::from(end_tick)),
            threshold: pm(900),
            peak_value: pm(950),
            frames: Vec::new(),
            exhaustion_collapse_observed: false,
        }
    }

    fn source_expectation_failure_payload(
        agent: EntityId,
        source: EntityId,
    ) -> SourceExpectationFailurePayload {
        let goal_key = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        SourceExpectationFailurePayload {
            agent,
            opportunity: OpportunityKey {
                goal_key,
                anchor: OpportunityAnchor::Entity(source),
            },
            source: SourceKeyPayload {
                entity: source,
                commodity: CommodityKind::Water,
            },
            expectation_kind: OpportunityExpectationKindTag::AcquireCommodityFromConcreteSource,
            phase: ExpectationFailurePhaseTag::Observation,
            cause: ExpectationFailureCauseTag::SourceDepletedLocally,
            detected_at_tick: Tick(5),
            attribution_outcome: SourceAttributionOutcomeTag::SourceReliabilityDecremented,
            decisive_beliefs: Vec::new(),
            decisive_records: Vec::new(),
            decisive_world_observations: Vec::new(),
        }
    }

    fn lot_condition_mismatch_payload(
        agent: EntityId,
        lot: EntityId,
        believed_condition: Permille,
        observed_condition: Permille,
    ) -> worldwake_core::LotConditionExpectationMismatchPayload {
        worldwake_core::LotConditionExpectationMismatchPayload {
            observer: agent,
            lot,
            commodity: CommodityKind::Apple,
            believed_condition,
            observed_condition,
        }
    }

    fn event_log_with_decision_payload(
        tick: Tick,
        actor: EntityId,
        tag: EventTag,
        decision_payload: DecisionEventPayload,
    ) -> EventLog {
        let mut event_log = EventLog::new();
        emit_decision_payload(&mut event_log, tick, actor, tag, decision_payload);
        event_log
    }

    fn emit_decision_payload(
        event_log: &mut EventLog,
        tick: Tick,
        actor: EntityId,
        tag: EventTag,
        decision_payload: DecisionEventPayload,
    ) {
        event_log.emit(PendingEvent::from_payload(EventPayload {
            tick,
            cause: CauseRef::SystemTick(tick),
            actor_id: Some(actor),
            action_name: None,
            target_ids: Vec::new(),
            evidence: Vec::new(),
            place_id: None,
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::Hidden,
            witness_data: WitnessData::default(),
            tags: std::collections::BTreeSet::from([tag]),
            contention_event_payload: None,
            decision_payload: Some(decision_payload),
            artifact_transition_payload: None,
            personality_assigned_payload: None,
        }));
    }

    fn sample_local_summary() -> LocalSurvivalStateSummary {
        LocalSurvivalStateSummary {
            place: Some(entity(50)),
            water_source_present: true,
            wash_basin_present: false,
            latrine_present: false,
            sleep_affordance_present: true,
            food_source_present: true,
        }
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }
}
