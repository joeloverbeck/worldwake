use crate::{
    AgentDecisionTrace, DecisionOutcome, GoalPriorityClass, PlanAttemptTrace, PlanSearchOutcome,
    RankedGoalProvenance, RankedGoalProvenanceFamily, RankedGoalSummary, SelectedPlanSource,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use worldwake_core::{
    BlockerKey, CommodityKind, DriveThresholds, EntityId, GoalKey, HomeostaticNeedId,
    HomeostaticNeeds, Permille, PlaceTag, Quantity, Tick, WorkstationTag, World,
};
use worldwake_sim::{ActionInstance, ActionInstanceId, ActionTraceEvent, ActionTraceSink};

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
    pub top_blocker: Option<BlockerKey>,
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
        .filter(|discrepancy| discrepancy.blocker_key.goal_key == selected_goal)
        .collect::<Vec<_>>();
    if matching.is_empty() {
        return None;
    }
    let top = matching[0];
    Some(BlockerSummary {
        blocker_count: matching.len() as u16,
        top_blocker: Some(top.blocker_key),
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
        OpportunityAnchor, PrototypePlace, Quantity, ResourceSource, VisibilitySpec, WitnessData,
        WorkstationMarker, WorldTxn, build_prototype_world, prototype_place_entity,
    };

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
                top_blocker: Some(BlockerKey {
                    goal_key: goal,
                    place: Some(entity(40)),
                    target: Some(entity(41)),
                    action_def: Some(ActionDefId(5)),
                }),
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
                        provenance: None,
                        source_reliability_discount: None,
                        competition_discount: None,
                        feasibility: crate::FeasibilityHint::Likely,
                        acquisition_quantity: None,
                    }],
                    top_ranked_comparison: None,
                    suppressed: Vec::new(),
                    zero_motive: Vec::new(),
                    omitted_political: Vec::new(),
                    omitted_bandit: Vec::new(),
                    omitted_social: Vec::new(),
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
                        strategic_plan: None,
                        tactical_goal: None,
                        landmarks_extracted: 0,
                        landmark_orderings: 0,
                        target_belief_presence:
                            crate::decision_trace::TargetBeliefPresence::NotApplicable,
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
            blocker_key: BlockerKey {
                goal_key: goal,
                place: Some(entity(40)),
                target: Some(entity(41)),
                action_def: Some(ActionDefId(5)),
            },
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
