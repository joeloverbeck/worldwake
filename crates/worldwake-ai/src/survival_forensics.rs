use crate::{
    AgentDecisionTrace, DecisionOutcome, GoalPriorityClass, PlanAttemptTrace, PlanSearchOutcome,
    RankedGoalProvenance, RankedGoalProvenanceFamily, RankedGoalSummary, SelectedPlanSource,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use worldwake_core::{
    BlockerScope, CommodityKind, DriveThresholds, EntityId, GoalKey, HomeostaticNeedId,
    HomeostaticNeeds, Permille, PlaceTag, Quantity, SleepFailureCause, Tick, WorkstationTag, World,
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
            sleep_affordance_present,
            food_source_present,
        }
    }
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

    pub fn observe(
        &mut self,
        tick: Tick,
        needs: &HomeostaticNeeds,
        thresholds: &DriveThresholds,
        decision_trace: Option<&AgentDecisionTrace>,
        action_trace_snapshot: &ActionTraceSnapshot<'_>,
        local_state: &LocalSurvivalStateSummary,
    ) {
        for need in HomeostaticNeedId::ALL {
            let need_value = needs.value(need);
            let threshold = thresholds.critical(need);
            if need_value >= threshold {
                let frame = build_frame(
                    tick,
                    need,
                    need_value,
                    decision_trace,
                    action_trace_snapshot,
                    *local_state,
                );
                self.active_windows
                    .entry(need)
                    .or_insert_with(|| WindowBuilder::new(tick, threshold))
                    .observe(frame);
                continue;
            }

            if let Some(builder) = self.active_windows.remove(&need) {
                self.completed_reports.push(builder.flush(self.agent, need));
            }
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
}

impl WindowBuilder {
    fn new(start_tick: Tick, threshold: Permille) -> Self {
        Self {
            start_tick,
            threshold,
            peak_value: Permille::ZERO,
            frames: Vec::new(),
        }
    }

    fn observe(&mut self, frame: CriticalWindowFrame) {
        self.peak_value = self.peak_value.max(frame.need_value);
        self.frames.push(frame);
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
        }
    }
}

fn build_frame(
    tick: Tick,
    need: HomeostaticNeedId,
    need_value: Permille,
    decision_trace: Option<&AgentDecisionTrace>,
    action_trace_snapshot: &ActionTraceSnapshot<'_>,
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
        AcquisitionQuantity, ActionDefId, CauseRef, ControlSource, DriveThresholds, GoalKind,
        OpportunityAnchor, PrototypePlace, Quantity, ResourceSource, SleepFailureCause,
        VisibilitySpec, WitnessData, WorkstationMarker, WorldTxn, build_prototype_world,
        prototype_place_entity,
    };
    use worldwake_sim::{ActionTraceDetail, ActionTraceEvent, ActionTraceKind};

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
                &local,
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
                &local,
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
                &local,
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
            &sample_local_summary(),
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

    #[test]
    fn critical_window_frame_deserializes_missing_failed_rest_as_empty() {
        let mut frame_value = serde_json::to_value(build_frame(
            Tick(3),
            HomeostaticNeedId::Fatigue,
            pm(930),
            None,
            &ActionTraceSnapshot::empty(),
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
            &sample_local_summary(),
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
            &local,
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
            &sample_local_summary(),
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
            &local,
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
            &sample_local_summary(),
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
        }
    }

    fn sample_local_summary() -> LocalSurvivalStateSummary {
        LocalSurvivalStateSummary {
            place: Some(entity(50)),
            water_source_present: true,
            wash_basin_present: false,
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
