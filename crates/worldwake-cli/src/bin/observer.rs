//! Headless diagnostic observation binary.
//!
//! Loads a `.ron` scenario, runs N ticks with all trace sinks enabled,
//! computes per-agent statistics and anomaly flags, then writes a structured
//! markdown report for LLM-driven behavioral analysis.
//!
//! This is a **tooling boundary** (FOUNDATIONS Principle 28): it reads
//! simulation state and traces without modifying world meaning.

use clap::Parser;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as FmtWrite;
use std::path::PathBuf;
use worldwake_ai::decision_trace::{
    AffordanceSummary, AffordanceTrace, AgentDecisionTrace, DecisionOutcome, PlanAttemptTrace,
    PlanSearchOutcome, TargetBeliefPresence,
};
use worldwake_ai::{
    ActionTraceSnapshot, AgendaEntry, AgendaState, AgentTickDriver, CriticalWindowReport,
    ExhaustionSummary, KillCondition, LocalSurvivalStateSummary, RevivalTrigger,
    SurvivalForensicExtractor,
};
use worldwake_cli::display::entity_display_name;
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, spawn_scenario_ignoring_lints};
use worldwake_core::{
    ActionInterruptReasonTag, AgentBeliefStore, CommodityKind, DeadAt, DeathCause,
    DecisionEventPayload, EntityId, EntityKind, EventId, EventView, GoalAbandonReason,
    HomeostaticNeedId, KnownRecipes, MetabolismProfile, PlaceTag, PlanInvalidationReason, Quantity,
    RecipeId, ReplanReason, Tick, WorkstationTag,
};
use worldwake_sim::{
    ActionTraceEvent, ActionTraceKind, ActionTraceSink, AutonomousControllerRuntime,
    InstitutionalKnowledgeTraceSink, PerceptionTraceSink, PoliticalTraceSink, RecipeDefinition,
    RecipeRegistry, RequestResolutionTraceSink, TickStepServices, step_tick,
};

#[derive(Parser)]
#[command(
    name = "worldwake-observer",
    about = "Headless diagnostic observation run"
)]
struct ObserverCli {
    /// Path to RON scenario file
    scenario: PathBuf,
    /// Number of ticks to simulate (default: 1440 = 1 day)
    #[arg(long, default_value_t = 1440)]
    ticks: u64,
    /// Output path for the observation dump
    #[arg(long, default_value = "reports/simulation-observer-dump.md")]
    output: PathBuf,
    /// Number of longest authored-critical windows to render in Section 10 (0 disables the section)
    #[arg(long, default_value_t = 3)]
    critical_window_top_n: usize,
    /// Bypass scenario lint failures for ad-hoc debugging.
    #[arg(long)]
    ignore_lints: bool,
}

// ---------------------------------------------------------------------------
// Per-agent statistics
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NeedsSample {
    hunger: u16,
    thirst: u16,
    fatigue: u16,
    bladder: u16,
    dirtiness: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BehavioralTransition {
    tick: u64,
    types_before: usize,
    types_after: usize,
    needs: NeedsSample,
}

/// A contiguous window where the agent had no action trace events.
#[derive(Clone, Debug)]
struct IdleWindow {
    start_tick: u64,
    end_tick: u64,
    needs_at_start: NeedsSample,
}

struct AgentStats {
    name: String,
    // Action counts by name and lifecycle kind
    actions_started: BTreeMap<String, u32>,
    actions_committed: BTreeMap<String, u32>,
    actions_aborted: BTreeMap<String, u32>,
    actions_start_failed: BTreeMap<String, u32>,
    // Perception
    observations_total: u32,
    observations_passed: u32,
    observation_entity_counts: BTreeMap<EntityId, u32>,
    /// Per-entity: set of distinct ticks at which the entity was observed.
    observation_tick_sets: BTreeMap<EntityId, BTreeSet<u64>>,
    // Needs trajectory
    needs_samples: Vec<NeedsSample>,
    // Location tracking
    location_ticks: BTreeMap<EntityId, u32>,
    location_history: Vec<Option<EntityId>>,
    // Action sequence (for loop detection)
    action_sequence: Vec<String>,
    // Idle tracking
    consecutive_idle_ticks: u32,
    max_consecutive_idle: u32,
    /// Start tick of the current idle window (None if not idle).
    idle_window_start: Option<u64>,
    /// Needs snapshot taken when the current idle window began.
    idle_window_needs: Option<NeedsSample>,
    /// All idle windows that lasted >= 2 ticks.
    idle_windows: Vec<IdleWindow>,
    // Whether agent has a patrol route (exempt from loop detection)
    has_patrol_route: bool,
}

impl AgentStats {
    fn new(name: String, has_patrol_route: bool) -> Self {
        Self {
            name,
            actions_started: BTreeMap::new(),
            actions_committed: BTreeMap::new(),
            actions_aborted: BTreeMap::new(),
            actions_start_failed: BTreeMap::new(),
            observations_total: 0,
            observations_passed: 0,
            observation_entity_counts: BTreeMap::new(),
            observation_tick_sets: BTreeMap::new(),
            needs_samples: Vec::new(),
            location_ticks: BTreeMap::new(),
            location_history: Vec::new(),
            action_sequence: Vec::new(),
            consecutive_idle_ticks: 0,
            max_consecutive_idle: 0,
            idle_window_start: None,
            idle_window_needs: None,
            idle_windows: Vec::new(),
            has_patrol_route,
        }
    }

    fn total_actions(&self) -> u32 {
        let sum = |m: &BTreeMap<String, u32>| m.values().sum::<u32>();
        sum(&self.actions_started)
            + sum(&self.actions_committed)
            + sum(&self.actions_aborted)
            + sum(&self.actions_start_failed)
    }

    fn record_idle_tick(
        &mut self,
        had_action: bool,
        current_tick: u64,
        current_needs: NeedsSample,
    ) {
        if had_action {
            // Close any open idle window.
            if let Some(start) = self.idle_window_start.take() {
                let needs_at_start = self.idle_window_needs.take().unwrap_or(current_needs);
                self.idle_windows.push(IdleWindow {
                    start_tick: start,
                    end_tick: current_tick.saturating_sub(1),
                    needs_at_start,
                });
            }
            self.consecutive_idle_ticks = 0;
        } else {
            // Open a new idle window on the first idle tick.
            if self.idle_window_start.is_none() {
                self.idle_window_start = Some(current_tick);
                self.idle_window_needs = Some(current_needs);
            }
            self.consecutive_idle_ticks += 1;
            if self.consecutive_idle_ticks > self.max_consecutive_idle {
                self.max_consecutive_idle = self.consecutive_idle_ticks;
            }
        }
    }

    /// Flush any open idle window at simulation end.
    fn flush_idle_window(&mut self, final_tick: u64) {
        if let Some(start) = self.idle_window_start.take() {
            let needs_at_start = self.idle_window_needs.take().unwrap_or(NeedsSample {
                hunger: 0,
                thirst: 0,
                fatigue: 0,
                bladder: 0,
                dirtiness: 0,
            });
            self.idle_windows.push(IdleWindow {
                start_tick: start,
                end_tick: final_tick,
                needs_at_start,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Budget exhaustion snapshots
// ---------------------------------------------------------------------------

/// Captured world state at the moment a planner search returns
/// [`BudgetExhausted`]. Deduplicated by (agent, `goal_kind`, location) —
/// only the first occurrence is kept.
struct BudgetExhaustionSnapshot {
    tick: u64,
    agent_id: EntityId,
    agent_name: String,
    goal_debug: String,
    /// Needs at the tick of exhaustion.
    needs: NeedsSample,
    /// Agent location at the tick of exhaustion.
    location: EntityId,
    location_name: String,
    /// Agent inventory grouped by commodity type.
    inventory: BTreeMap<String, u64>,
    /// Believed entity locations: place name to list of entity descriptions.
    beliefs: BTreeMap<String, Vec<String>>,
    /// Total known entities in belief store.
    known_entity_count: usize,
    /// Contents of the agent's current location.
    place_contents: Vec<String>,
    /// Contents of adjacent places: place name to contents list.
    adjacent_contents: BTreeMap<String, Vec<String>>,
    /// Cognitive profile fields relevant to search budget.
    max_node_expansions: u16,
    max_plan_depth: u8,
    max_candidates_per_expansion: u16,
    /// Execution budget fields.
    max_prerequisite_locations: u8,
    beam_width: u8,
    preferred_operator_boost: u8,
    /// Search outcome metrics from the attempt.
    expansions_used: u16,
    max_depth_reached: u8,
    total_candidates: u32,
}

/// Deduplication key for budget exhaustion snapshots.
#[derive(Eq, Ord, PartialEq, PartialOrd)]
struct BudgetExhaustionKey {
    agent_id: EntityId,
    goal_kind: String,
    location: EntityId,
}

/// Collect inventory as commodity -> count map.
fn collect_inventory(world: &worldwake_core::World, agent_id: EntityId) -> BTreeMap<String, u64> {
    let mut result: BTreeMap<String, u64> = BTreeMap::new();
    for entity in world.possessions_of(agent_id) {
        if let Some(lot) = world.get_component_item_lot(entity) {
            *result.entry(format!("{:?}", lot.commodity)).or_insert(0) += u64::from(lot.quantity.0);
        } else {
            let name = entity_display_name(world, entity);
            *result.entry(name).or_insert(0) += 1;
        }
    }
    result
}

/// Collect place contents as a list of display strings.
fn collect_place_contents(world: &worldwake_core::World, place_id: EntityId) -> Vec<String> {
    let ground = world.ground_entities_at(place_id);
    let mut commodity_totals: BTreeMap<String, u64> = BTreeMap::new();
    let mut non_item_entries: Vec<String> = Vec::new();
    for entity in &ground {
        if let Some(lot) = world.get_component_item_lot(*entity) {
            *commodity_totals
                .entry(format!("{:?}", lot.commodity))
                .or_insert(0) += u64::from(lot.quantity.0);
        } else {
            let name = entity_display_name(world, *entity);
            let annotation = match world.entity_kind(*entity) {
                Some(EntityKind::Agent) => " (agent)".to_string(),
                Some(EntityKind::Facility) => {
                    if let Some(ws) = world.get_component_workstation_marker(*entity) {
                        format!(" ({:?})", ws.0)
                    } else if let Some(rs) = world.get_component_resource_source(*entity) {
                        format!(" (resource: {:?})", rs.commodity)
                    } else {
                        " (facility)".to_string()
                    }
                }
                _ => String::new(),
            };
            non_item_entries.push(format!("{name}{annotation}"));
        }
    }
    let mut parts: Vec<String> = Vec::new();
    parts.extend(non_item_entries);
    for (commodity, total) in &commodity_totals {
        parts.push(format!("{total}\u{00d7} {commodity}"));
    }
    parts
}

/// Collect believed entity locations from an agent's belief store.
fn collect_beliefs(
    world: &worldwake_core::World,
    agent_id: EntityId,
) -> (usize, BTreeMap<String, Vec<String>>) {
    let Some(store) = world.get_component_agent_belief_store(agent_id) else {
        return (0, BTreeMap::new());
    };
    let known_count = store.known_entities.len();
    let mut by_place: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (known_id, state) in &store.known_entities {
        let place_label = state.last_known_place.map_or_else(
            || "(unknown)".to_string(),
            |pid| entity_display_name(world, pid),
        );
        let entity_label = entity_display_name(world, *known_id);
        by_place.entry(place_label).or_default().push(entity_label);
    }
    (known_count, by_place)
}

/// Extract a normalized goal kind string for deduplication.
fn goal_kind_dedup_key(goal_debug: &str) -> String {
    // Strip entity IDs and specific field values to get a canonical goal kind.
    // E.g. "AcquireCommodity { commodity: Water, purpose: SelfConsume }" ->
    //      "AcquireCommodity(Water)"
    // Simple heuristic: use the first word + commodity if present.
    if let Some(brace) = goal_debug.find('{') {
        let kind = goal_debug[..brace].trim();
        // Try to extract commodity
        if let Some(comm_start) = goal_debug.find("commodity: ") {
            let after = &goal_debug[comm_start + 11..];
            let end = after
                .find(',')
                .or_else(|| after.find('}'))
                .unwrap_or(after.len());
            let commodity = after[..end].trim();
            return format!("{kind}({commodity})");
        }
        // Try to extract patient or target
        if goal_debug.contains("patient: ") {
            return format!("{kind}(patient)");
        }
        return kind.to_string();
    }
    goal_debug.to_string()
}

/// Compute total candidates and max depth from expansion summaries.
fn compute_search_metrics(
    summaries: &[worldwake_ai::decision_trace::SearchExpansionSummary],
) -> (u32, u8) {
    let mut total_candidates: u32 = 0;
    let mut max_depth: u8 = 0;
    for s in summaries {
        total_candidates += u32::from(s.candidates_generated);
        if s.depth > max_depth {
            max_depth = s.depth;
        }
    }
    (total_candidates, max_depth)
}

fn decision_payload_agent(payload: &DecisionEventPayload) -> EntityId {
    match payload {
        DecisionEventPayload::GoalOffered(inner) => inner.agent,
        DecisionEventPayload::GoalSuppressed(inner) => inner.agent,
        DecisionEventPayload::GoalCommitted(inner) => inner.agent,
        DecisionEventPayload::GoalSuspended(inner) => inner.agent,
        DecisionEventPayload::GoalAbandoned(inner) => inner.agent,
        DecisionEventPayload::SleepEpisodeStarted(inner) => inner.sleeper,
        DecisionEventPayload::SleepEpisodeEnded(inner) => inner.sleeper,
        DecisionEventPayload::PlanAdopted(inner) => inner.agent,
        DecisionEventPayload::PlanInvalidated(inner) => inner.agent,
        DecisionEventPayload::ExpectationMismatch(inner) => inner.agent,
        DecisionEventPayload::SourceExpectationFailure(inner) => inner.agent,
        DecisionEventPayload::RepairApplied(inner) => inner.agent,
        DecisionEventPayload::ReplanTriggered(inner) => inner.agent,
        DecisionEventPayload::BlockerRecorded(inner) => inner.agent,
    }
}

fn decision_event_name(payload: &DecisionEventPayload) -> &'static str {
    match payload {
        DecisionEventPayload::GoalOffered(_) => "GoalOffered",
        DecisionEventPayload::GoalSuppressed(_) => "GoalSuppressed",
        DecisionEventPayload::GoalCommitted(_) => "GoalCommitted",
        DecisionEventPayload::GoalSuspended(_) => "GoalSuspended",
        DecisionEventPayload::GoalAbandoned(_) => "GoalAbandoned",
        DecisionEventPayload::SleepEpisodeStarted(_) => "SleepEpisodeStarted",
        DecisionEventPayload::SleepEpisodeEnded(_) => "SleepEpisodeEnded",
        DecisionEventPayload::PlanAdopted(_) => "PlanAdopted",
        DecisionEventPayload::PlanInvalidated(_) => "PlanInvalidated",
        DecisionEventPayload::ExpectationMismatch(_) => "ExpectationMismatch",
        DecisionEventPayload::SourceExpectationFailure(_) => "SourceExpectationFailure",
        DecisionEventPayload::RepairApplied(_) => "RepairApplied",
        DecisionEventPayload::ReplanTriggered(_) => "ReplanTriggered",
        DecisionEventPayload::BlockerRecorded(_) => "BlockerRecorded",
    }
}

fn format_decision_evidence_counts(payload: &worldwake_core::EvidenceSummary) -> String {
    payload
        .evidence_kind_counts
        .iter()
        .map(|(kind, count)| format!("{kind:?}x{count}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn decision_payload_summary(payload: &DecisionEventPayload) -> String {
    match payload {
        DecisionEventPayload::GoalOffered(inner) => format!(
            "goal={:?} emitter={:?} evidence={}",
            inner.goal_key.kind,
            inner.emitter,
            format_decision_evidence_counts(&inner.source_evidence)
        ),
        DecisionEventPayload::GoalSuppressed(inner) => {
            format!("goal={:?} reason={:?}", inner.goal_key.kind, inner.reason)
        }
        DecisionEventPayload::GoalCommitted(inner) => format!(
            "goal={:?} motive={} alts={}",
            inner.goal_key.kind,
            inner.motive_score,
            inner.rejected_alternatives.len()
        ),
        DecisionEventPayload::GoalSuspended(inner) => {
            format!("goal={:?} reason={:?}", inner.goal_key.kind, inner.reason)
        }
        DecisionEventPayload::GoalAbandoned(inner) => format!(
            "goal={:?} reason={}",
            inner.goal_key.kind,
            match &inner.reason {
                GoalAbandonReason::FrameCleared { reason } => {
                    format!("FrameCleared({reason:?})")
                }
                GoalAbandonReason::GoalSwitched {
                    new_goal,
                    switch_kind,
                } => format!("GoalSwitched({switch_kind:?}->{:?})", new_goal.kind),
            }
        ),
        DecisionEventPayload::SleepEpisodeStarted(inner) => format!(
            "place={} min={} max={} target={} modifier={} wake_conditions={:?}",
            inner.place,
            inner.intended_min_ticks,
            inner.intended_max_ticks,
            inner.target_recovery.value(),
            inner.recovery_modifier.value(),
            inner.wake_conditions
        ),
        DecisionEventPayload::SleepEpisodeEnded(inner) => format!(
            "place={} ticks={}->{} reason={:?} recovery={} fatigue={}",
            inner.place,
            inner.start_tick.0,
            inner.end_tick.0,
            inner.end_reason,
            inner.accumulated_recovery.value(),
            inner.final_fatigue.value()
        ),
        DecisionEventPayload::PlanAdopted(inner) => {
            format!(
                "goal={:?} steps={}",
                inner.goal_key.kind, inner.plan_step_count
            )
        }
        DecisionEventPayload::PlanInvalidated(inner) => format!(
            "goal={:?} reason={}",
            inner.goal_key.kind,
            format_plan_invalidation_reason(&inner.reason)
        ),
        DecisionEventPayload::ExpectationMismatch(inner) => format!(
            "goal={:?} step={} expected={:?}",
            inner.goal_key.kind, inner.step_index, inner.expected_materializations
        ),
        DecisionEventPayload::SourceExpectationFailure(inner) => format!(
            "opportunity={:?} source={:?}:{:?} phase={:?} cause={:?} outcome={:?}",
            inner.opportunity.goal_key.kind,
            inner.source.entity,
            inner.source.commodity,
            inner.phase,
            inner.cause,
            inner.attribution_outcome
        ),
        DecisionEventPayload::RepairApplied(inner) => format!(
            "goal={:?} step={} kind={:?} target={:?}",
            inner.goal_key.kind, inner.step_index, inner.repair_kind, inner.substitute_target
        ),
        DecisionEventPayload::ReplanTriggered(inner) => format!(
            "goal={:?} reason={}",
            inner.goal_key.kind,
            format_replan_reason(&inner.reason)
        ),
        DecisionEventPayload::BlockerRecorded(inner) => {
            let class = match (inner.discrepancy, inner.blocking_fact) {
                (Some(discrepancy), None) => format!("Discrepancy({discrepancy:?})"),
                (None, Some(blocking_fact)) => format!("BlockingFact({blocking_fact:?})"),
                _ => "Unclassified".to_string(),
            };
            format!(
                "key={:?} class={} expires={}",
                inner.blocker_key, class, inner.expires_tick.0
            )
        }
    }
}

fn format_plan_invalidation_reason(reason: &PlanInvalidationReason) -> String {
    match reason {
        PlanInvalidationReason::BeliefUpdate { claim_key } => {
            format!("BeliefUpdate({claim_key:?})")
        }
        PlanInvalidationReason::TargetGone { target } => format!("TargetGone({target})"),
        PlanInvalidationReason::ExpectationMismatch { step_index } => {
            format!("ExpectationMismatch(step={step_index})")
        }
        PlanInvalidationReason::ContentionLost { place, action } => {
            format!("ContentionLost(place={place},action={action:?})")
        }
        PlanInvalidationReason::DiscrepancyRecorded { discrepancy } => {
            format!("DiscrepancyRecorded({discrepancy:?})")
        }
        PlanInvalidationReason::PreemptedByHigherGoal { new_goal } => {
            format!("PreemptedByHigherGoal({:?})", new_goal.kind)
        }
        PlanInvalidationReason::PursuitInvalidated { reason } => {
            format!("PursuitInvalidated({reason:?})")
        }
        PlanInvalidationReason::AssumptionFailed { assumption } => {
            format!("AssumptionFailed({assumption:?})")
        }
        PlanInvalidationReason::AgentIncapacitated => "AgentIncapacitated".to_string(),
    }
}

fn format_replan_reason(reason: &ReplanReason) -> String {
    match reason {
        ReplanReason::PlanInvalidated { reason } => {
            format!(
                "PlanInvalidated({})",
                format_plan_invalidation_reason(reason)
            )
        }
        ReplanReason::ActionInterrupted { reason } => {
            format!(
                "ActionInterrupted({})",
                format_action_interrupt_reason(*reason)
            )
        }
        ReplanReason::ActionStartFailed => "ActionStartFailed".to_string(),
        ReplanReason::BlockingFactRecorded { blocking_fact } => {
            format!("BlockingFactRecorded({blocking_fact:?})")
        }
        ReplanReason::DiscrepancyRecorded { discrepancy } => {
            format!("DiscrepancyRecorded({discrepancy:?})")
        }
        ReplanReason::LocalRepairExhausted => "LocalRepairExhausted".to_string(),
        ReplanReason::SearchBudgetExhausted => "SearchBudgetExhausted".to_string(),
        ReplanReason::GoalSwitched {
            new_goal,
            switch_kind,
        } => format!("GoalSwitched({switch_kind:?}->{:?})", new_goal.kind),
    }
}

fn format_action_interrupt_reason(reason: ActionInterruptReasonTag) -> &'static str {
    match reason {
        ActionInterruptReasonTag::DangerNearby => "DangerNearby",
        ActionInterruptReasonTag::Reprioritized => "Reprioritized",
        ActionInterruptReasonTag::Other => "Other",
    }
}

fn render_decision_history_section(
    out: &mut String,
    event_log: &worldwake_core::EventLog,
    agents: &[(EntityId, String)],
) {
    writeln!(out, "## Section 3 — Decision History\n").unwrap();
    let agent_names = agents
        .iter()
        .map(|(id, name)| (*id, name.as_str()))
        .collect::<BTreeMap<_, _>>();

    let mut rendered_any = false;
    writeln!(out, "| Tick | Agent | Event | Payload Summary |").unwrap();
    writeln!(out, "|------|-------|-------|-----------------|").unwrap();
    for index in 0..event_log.len() {
        let Some(record) = event_log.get(EventId(index as u64)) else {
            continue;
        };
        let Some(payload) = record.decision_payload() else {
            continue;
        };
        let agent = decision_payload_agent(payload);
        let agent_name = agent_names
            .get(&agent)
            .copied()
            .map_or_else(|| agent.to_string(), str::to_owned);
        writeln!(
            out,
            "| {} | {} | {} | {} |",
            record.tick().0,
            agent_name,
            decision_event_name(payload),
            decision_payload_summary(payload)
        )
        .unwrap();
        rendered_any = true;
    }
    if !rendered_any {
        writeln!(out, "| - | - | - | No decision events recorded. |").unwrap();
    }
    writeln!(out).unwrap();
}

fn format_budget_exhaustion_snapshots(out: &mut String, snapshots: &[BudgetExhaustionSnapshot]) {
    writeln!(out, "## Section 9 — Budget Exhaustion Snapshots\n").unwrap();
    if snapshots.is_empty() {
        writeln!(out, "No budget exhaustion events detected.\n").unwrap();
        return;
    }
    writeln!(
        out,
        "{} unique budget-exhaustion signatures captured (deduplicated by agent+goal+location).\n",
        snapshots.len()
    )
    .unwrap();

    for (i, snap) in snapshots.iter().enumerate() {
        writeln!(
            out,
            "### Snapshot {} — {} at tick {}\n",
            i + 1,
            snap.agent_name,
            snap.tick
        )
        .unwrap();

        writeln!(out, "**Agent**: {} ({})", snap.agent_name, snap.agent_id).unwrap();
        writeln!(out, "**Goal**: `{}`", snap.goal_debug).unwrap();
        writeln!(
            out,
            "**Location**: {} ({})",
            snap.location_name, snap.location
        )
        .unwrap();
        writeln!(out).unwrap();

        // Search metrics
        writeln!(out, "**Search metrics**:").unwrap();
        writeln!(out, "- Expansions used: {}", snap.expansions_used).unwrap();
        writeln!(out, "- Max depth reached: {}", snap.max_depth_reached).unwrap();
        writeln!(
            out,
            "- Total candidates generated: {}",
            snap.total_candidates
        )
        .unwrap();
        writeln!(out).unwrap();

        // Cognitive/execution profile
        writeln!(out, "**Planner configuration**:").unwrap();
        writeln!(out, "- max_node_expansions: {}", snap.max_node_expansions).unwrap();
        writeln!(out, "- max_plan_depth: {}", snap.max_plan_depth).unwrap();
        if snap.max_candidates_per_expansion > 0 {
            writeln!(
                out,
                "- max_candidates_per_expansion: {}",
                snap.max_candidates_per_expansion
            )
            .unwrap();
        }
        writeln!(
            out,
            "- max_prerequisite_locations: {}",
            snap.max_prerequisite_locations
        )
        .unwrap();
        writeln!(out, "- beam_width: {}", snap.beam_width).unwrap();
        writeln!(
            out,
            "- preferred_operator_boost: {}",
            snap.preferred_operator_boost
        )
        .unwrap();
        writeln!(out).unwrap();

        // Needs
        writeln!(out, "**Agent needs** (\u{2030}):").unwrap();
        writeln!(
            out,
            "- hunger={}, thirst={}, fatigue={}, bladder={}, dirtiness={}",
            snap.needs.hunger,
            snap.needs.thirst,
            snap.needs.fatigue,
            snap.needs.bladder,
            snap.needs.dirtiness
        )
        .unwrap();
        writeln!(out).unwrap();

        // Inventory
        writeln!(out, "**Agent inventory**:").unwrap();
        if snap.inventory.is_empty() {
            writeln!(out, "- (empty)").unwrap();
        } else {
            for (item, count) in &snap.inventory {
                writeln!(out, "- {count}\u{00d7} {item}").unwrap();
            }
        }
        writeln!(out).unwrap();

        // Beliefs
        writeln!(
            out,
            "**Beliefs** ({} known entities):",
            snap.known_entity_count
        )
        .unwrap();
        for (place, entities) in &snap.beliefs {
            writeln!(out, "- {place}: {}", entities.join(", ")).unwrap();
        }
        writeln!(out).unwrap();

        // Place contents
        writeln!(out, "**Current place contents**:").unwrap();
        if snap.place_contents.is_empty() {
            writeln!(out, "- (empty)").unwrap();
        } else {
            for item in &snap.place_contents {
                writeln!(out, "- {item}").unwrap();
            }
        }
        writeln!(out).unwrap();

        // Adjacent place contents
        writeln!(out, "**Adjacent place contents**:").unwrap();
        if snap.adjacent_contents.is_empty() {
            writeln!(out, "- (none)").unwrap();
        } else {
            for (place, contents) in &snap.adjacent_contents {
                writeln!(out, "- {place}: {}", contents.join(", ")).unwrap();
            }
        }
        writeln!(out).unwrap();
    }
}

fn format_critical_window_forensics(
    out: &mut String,
    agents: &[(EntityId, String)],
    world: &worldwake_core::World,
    reports: &[CriticalWindowReport],
    total_windows_detected: usize,
) {
    writeln!(out, "## Section 10 — Critical Window Forensics\n").unwrap();
    if reports.is_empty() {
        writeln!(out, "No authored-critical windows detected.\n").unwrap();
        return;
    }

    writeln!(
        out,
        "Showing {} longest authored-critical windows out of {} detected.\n",
        reports.len(),
        total_windows_detected
    )
    .unwrap();

    let agent_names = agents
        .iter()
        .map(|(agent, name)| (*agent, name.as_str()))
        .collect::<BTreeMap<_, _>>();

    for (idx, report) in reports.iter().enumerate() {
        let agent_name = agent_names
            .get(&report.agent)
            .copied()
            .unwrap_or("Unknown agent");
        writeln!(
            out,
            "### Window {} — {} / {:?}\n",
            idx + 1,
            agent_name,
            report.need
        )
        .unwrap();
        writeln!(out, "**Agent**: {agent_name} ({})", report.agent).unwrap();
        writeln!(out, "**Need**: {:?}", report.need).unwrap();
        writeln!(
            out,
            "**Window**: tick {}..{}",
            report.start_tick.0, report.end_tick.0
        )
        .unwrap();
        writeln!(
            out,
            "**Authored critical threshold**: {} per mille",
            report.threshold.value()
        )
        .unwrap();
        writeln!(
            out,
            "**Peak need value**: {} per mille",
            report.peak_value.value()
        )
        .unwrap();
        writeln!(
            out,
            "**Selected goals across captured frames**: {}",
            summarize_selected_goals(report)
        )
        .unwrap();
        writeln!(
            out,
            "**Selected plan sources**: {}",
            summarize_plan_sources(report)
        )
        .unwrap();
        writeln!(
            out,
            "**Exhaustion states**: {}",
            summarize_exhaustion_states(report)
        )
        .unwrap();
        writeln!(
            out,
            "**Blocker summaries**: {}",
            summarize_blocker_states(report)
        )
        .unwrap();
        writeln!(out).unwrap();

        writeln!(
            out,
            "| Tick | Need | Selected Goal | Plan Source | Top Competitors | Active Action | Exhaustion | Blocker | Local Summary |"
        )
        .unwrap();
        writeln!(
            out,
            "|------|------|---------------|-------------|-----------------|---------------|------------|---------|---------------|"
        )
        .unwrap();
        for frame in &report.frames {
            let selected_goal = frame
                .selected_goal
                .map_or_else(|| "-".to_string(), |goal| format!("{:?}", goal.kind));
            let selected_plan_source = frame
                .selected_plan_source
                .map_or_else(|| "-".to_string(), |source| format!("{source:?}"));
            let top_competitors = if frame.top_competitors.is_empty() {
                "-".to_string()
            } else {
                frame
                    .top_competitors
                    .iter()
                    .map(|competitor| format!("{:?}", competitor.goal.kind))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let active_action = frame.active_action.as_ref().map_or_else(
                || "-".to_string(),
                |action| format!("{}@{}", action.action_name, action.started_at.0),
            );
            writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                frame.tick.0,
                frame.need_value.value(),
                selected_goal,
                selected_plan_source,
                top_competitors,
                active_action,
                format_frame_exhaustion(frame.exhaustion_state.as_ref()),
                format_frame_blocker(frame.blocker_summary.as_ref()),
                format_local_survival_state_summary(world, &frame.local_authoritative_summary),
            )
            .unwrap();
        }
        writeln!(out).unwrap();
    }
}

fn summarize_selected_goals(report: &CriticalWindowReport) -> String {
    summarize_counts(
        report
            .frames
            .iter()
            .filter_map(|frame| frame.selected_goal.map(|goal| format!("{:?}", goal.kind))),
    )
}

fn summarize_plan_sources(report: &CriticalWindowReport) -> String {
    summarize_counts(report.frames.iter().filter_map(|frame| {
        frame
            .selected_plan_source
            .map(|source| format!("{source:?}"))
    }))
}

fn summarize_exhaustion_states(report: &CriticalWindowReport) -> String {
    summarize_counts(
        report
            .frames
            .iter()
            .filter_map(|frame| frame.exhaustion_state.as_ref().map(format_exhaustion_state)),
    )
}

fn summarize_blocker_states(report: &CriticalWindowReport) -> String {
    summarize_counts(
        report
            .frames
            .iter()
            .filter_map(|frame| frame.blocker_summary.as_ref().map(format_blocker_summary)),
    )
}

fn summarize_counts(values: impl IntoIterator<Item = String>) -> String {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for value in values {
        *counts.entry(value).or_insert(0) += 1;
    }
    if counts.is_empty() {
        return "none".to_string();
    }
    counts
        .into_iter()
        .map(|(label, count)| format!("{label} x{count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_exhaustion_state(exhaustion: &ExhaustionSummary) -> String {
    match exhaustion {
        ExhaustionSummary::FrontierExhausted { expansions_used } => {
            format!("frontier-exhausted (expansions_used={expansions_used})")
        }
        ExhaustionSummary::BudgetExhausted { expansions_used } => {
            format!("budget-exhausted (expansions_used={expansions_used})")
        }
        ExhaustionSummary::Unsupported => "unsupported".to_string(),
    }
}

fn format_blocker_summary(summary: &worldwake_ai::BlockerSummary) -> String {
    match summary.top_blocker {
        Some(blocker) => format!("{} blockers (top={blocker:?})", summary.blocker_count),
        None => format!("{} blockers", summary.blocker_count),
    }
}

fn format_frame_exhaustion(exhaustion: Option<&ExhaustionSummary>) -> String {
    exhaustion.map_or_else(|| "-".to_string(), format_exhaustion_state)
}

fn format_frame_blocker(summary: Option<&worldwake_ai::BlockerSummary>) -> String {
    summary.map_or_else(|| "-".to_string(), format_blocker_summary)
}

fn format_local_survival_state_summary(
    world: &worldwake_core::World,
    summary: &LocalSurvivalStateSummary,
) -> String {
    let place_name = summary.place.map_or_else(
        || "In transit".to_string(),
        |place| entity_display_name(world, place),
    );
    format!(
        "{}: water={}, wash={}, sleep={}, food={}",
        place_name,
        yes_no(summary.water_source_present),
        yes_no(summary.wash_basin_present),
        yes_no(summary.sleep_affordance_present),
        yes_no(summary.food_source_present)
    )
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

// ---------------------------------------------------------------------------
// Anomaly detection
// ---------------------------------------------------------------------------

struct Anomaly {
    kind: AnomalyKind,
    agent_name: String,
    additional_agent_names: Option<Vec<String>>,
    description: String,
    tick_range: Option<(u64, u64)>,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
enum AnomalyKind {
    RedundantPerception,
    ActionLoop,
    StuckAgent,
    FailedActionSpiral,
    SustainedCriticalNeed,
    UnaddressedNeed,
    GeographicConvergence,
    MaintenanceStarvation,
    RecipeMonoculture,
    AcuteNeedSpike,
}

impl AnomalyKind {
    fn label(self) -> &'static str {
        match self {
            Self::RedundantPerception => "REDUNDANT_PERCEPTION",
            Self::ActionLoop => "ACTION_LOOP",
            Self::StuckAgent => "STUCK_AGENT",
            Self::FailedActionSpiral => "FAILED_ACTION_SPIRAL",
            Self::SustainedCriticalNeed => "SUSTAINED_CRITICAL_NEED",
            Self::UnaddressedNeed => "UNADDRESSED_NEED",
            Self::GeographicConvergence => "GEOGRAPHIC_CONVERGENCE",
            Self::MaintenanceStarvation => "MAINTENANCE_STARVATION",
            Self::RecipeMonoculture => "RECIPE_MONOCULTURE",
            Self::AcuteNeedSpike => "ACUTE_NEED_SPIKE",
        }
    }
}

fn format_anomaly_header(index: usize, anomaly: &Anomaly) -> String {
    let mut names = vec![anomaly.agent_name.as_str()];
    if let Some(additional_names) = anomaly.additional_agent_names.as_ref()
        && !additional_names.is_empty()
    {
        names.extend(additional_names.iter().map(String::as_str));
    }

    format!(
        "### Anomaly {} — {} ({})",
        index,
        anomaly.kind.label(),
        names.join(", ")
    )
}

fn detect_anomalies(
    agent_stats: &BTreeMap<EntityId, AgentStats>,
    perception_trace: &PerceptionTraceSink,
    event_log: &worldwake_core::EventLog,
    world: &worldwake_core::World,
    recipe_registry: &RecipeRegistry,
) -> Vec<Anomaly> {
    let mut anomalies = Vec::new();

    for stats in agent_stats.values() {
        // 1. Redundant perception: observed same entity many times
        //    We flag if an entity was observed >= 10 times. A more precise
        //    check (no intervening state change) requires cross-referencing
        //    the event log which is expensive; we use count as a heuristic.
        for (entity, count) in &stats.observation_entity_counts {
            if *count >= 10 {
                let entity_name = entity_display_name(world, *entity);
                let distinct_ticks = stats
                    .observation_tick_sets
                    .get(entity)
                    .map_or(0, BTreeSet::len);
                let entity_kind = world
                    .entity_kind(*entity)
                    .map_or_else(|| "unknown".to_string(), |k| format!("{k:?}"));
                anomalies.push(Anomaly {
                    kind: AnomalyKind::RedundantPerception,
                    agent_name: stats.name.clone(),
                    additional_agent_names: None,
                    description: format!(
                        "Observed entity {entity} ({entity_name}, {entity_kind}) {count} times \
                         across {distinct_ticks} distinct ticks via event witnessing",
                    ),
                    tick_range: None,
                });
            }
        }

        // 2. Action loops (skip patrol agents)
        if !stats.has_patrol_route
            && let Some(loop_desc) = detect_action_loop(&stats.action_sequence)
        {
            anomalies.push(Anomaly {
                kind: AnomalyKind::ActionLoop,
                agent_name: stats.name.clone(),
                additional_agent_names: None,
                description: loop_desc,
                tick_range: None,
            });
        }

        // 3. Stuck agents: no actions for >= 20 consecutive ticks
        if stats.max_consecutive_idle >= 20 {
            // Find the longest idle window(s) that match the max.
            let significant_windows: Vec<_> = stats
                .idle_windows
                .iter()
                .filter(|w| (w.end_tick - w.start_tick + 1) as u32 >= 20)
                .collect();
            let mut desc = format!(
                "No actions for {} consecutive ticks.",
                stats.max_consecutive_idle
            );
            for w in &significant_windows {
                let duration = w.end_tick - w.start_tick + 1;
                write!(
                    desc,
                    "\n  Window ticks {}-{} ({} ticks): needs at start: \
                     hunger={}, thirst={}, fatigue={}, bladder={}, dirtiness={}",
                    w.start_tick,
                    w.end_tick,
                    duration,
                    w.needs_at_start.hunger,
                    w.needs_at_start.thirst,
                    w.needs_at_start.fatigue,
                    w.needs_at_start.bladder,
                    w.needs_at_start.dirtiness,
                )
                .unwrap();
            }
            let tick_range = significant_windows
                .first()
                .map(|w| (w.start_tick, w.end_tick));
            anomalies.push(Anomaly {
                kind: AnomalyKind::StuckAgent,
                agent_name: stats.name.clone(),
                additional_agent_names: None,
                description: desc,
                tick_range,
            });
        }

        // 4. Failed action spirals: > 75% failure rate for an action with >= 5 attempts
        for (action_name, started_count) in &stats.actions_started {
            let failed_count = stats
                .actions_start_failed
                .get(action_name)
                .copied()
                .unwrap_or(0);
            let total_attempts = started_count + failed_count;
            if total_attempts >= 5 && failed_count * 4 > total_attempts * 3 {
                anomalies.push(Anomaly {
                    kind: AnomalyKind::FailedActionSpiral,
                    agent_name: stats.name.clone(),
                    additional_agent_names: None,
                    description: format!(
                        "Action '{}': {} failed out of {} attempts ({:.0}% failure rate)",
                        action_name,
                        failed_count,
                        total_attempts,
                        (f64::from(failed_count) / f64::from(total_attempts)) * 100.0
                    ),
                    tick_range: None,
                });
            }
        }

        // 5. Sustained critical needs: a need stays above 750 for 100+ consecutive ticks
        detect_sustained_critical_needs(stats, &mut anomalies);

        // 6. Unaddressed needs: need avg > 750 but no corresponding relief action attempted
        detect_unaddressed_needs(stats, &mut anomalies);
    }

    detect_geographic_convergence(agent_stats, world, &mut anomalies);
    let thresholds_by_agent = agent_stats
        .keys()
        .map(|agent_id| {
            (
                *agent_id,
                world
                    .get_component_drive_thresholds(*agent_id)
                    .copied()
                    .unwrap_or_default(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    detect_maintenance_starvation(agent_stats, &thresholds_by_agent, &mut anomalies);
    let known_recipes_by_agent = agent_stats
        .keys()
        .map(|agent_id| {
            (
                *agent_id,
                world
                    .get_component_known_recipes(*agent_id)
                    .map(|known| known.recipes.clone())
                    .unwrap_or_default(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    detect_recipe_monoculture(
        agent_stats,
        &known_recipes_by_agent,
        recipe_registry,
        world,
        &mut anomalies,
    );
    let metabolism_by_agent = agent_stats
        .keys()
        .map(|agent_id| {
            (
                *agent_id,
                world
                    .get_component_metabolism_profile(*agent_id)
                    .copied()
                    .unwrap_or_default(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    detect_acute_need_spike(
        agent_stats,
        &thresholds_by_agent,
        &metabolism_by_agent,
        &mut anomalies,
    );

    // Cross-reference redundant perception more precisely using event log
    refine_redundant_perception(&mut anomalies, agent_stats, perception_trace, event_log);

    // Remove stuck-agent anomalies that are just agents correctly idling
    // with all needs low.
    refine_stuck_agents(&mut anomalies, agent_stats);

    anomalies
}

const GEOGRAPHIC_CONVERGENCE_SHARE_THRESHOLD_PERMILLE: u32 = 600;
const ANOMALY_ROLLING_WINDOW_TICKS: usize = 200;
const ACUTE_NEED_SPIKE_MIN_TICKS: usize = 30;
const ACUTE_NEED_SPIKE_MAX_TICKS: usize = 100;

fn commodity_is_edible(commodity: CommodityKind) -> bool {
    commodity.spec().consumable_profile.is_some()
}

fn place_survival_state_summary(
    world: &worldwake_core::World,
    place: EntityId,
) -> LocalSurvivalStateSummary {
    let water_source_present = world.query_resource_source().any(|(entity, source)| {
        world.effective_place(entity) == Some(place)
            && source.commodity == CommodityKind::Water
            && source.available_quantity > Quantity(0)
    });
    let wash_basin_present = world.query_workstation_marker().any(|(entity, marker)| {
        world.effective_place(entity) == Some(place) && marker.0 == WorkstationTag::WashBasin
    });
    let sleep_affordance_present = [PlaceTag::Inn, PlaceTag::Barracks, PlaceTag::Camp]
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

    LocalSurvivalStateSummary {
        place: Some(place),
        water_source_present,
        wash_basin_present,
        sleep_affordance_present,
        food_source_present,
    }
}

fn is_lawful_split_support_convergence_place(
    world: &worldwake_core::World,
    place: EntityId,
) -> bool {
    let summary = place_survival_state_summary(world, place);
    let support_count = [
        summary.water_source_present,
        summary.wash_basin_present,
        summary.sleep_affordance_present,
        summary.food_source_present,
    ]
    .into_iter()
    .filter(|present| *present)
    .count();
    if support_count != 1 {
        return false;
    }

    world.topology().place_ids().any(|other_place| {
        if other_place == place {
            return false;
        }
        let other_summary = place_survival_state_summary(world, other_place);
        (summary.water_source_present != other_summary.water_source_present
            && other_summary.water_source_present)
            || (summary.wash_basin_present != other_summary.wash_basin_present
                && other_summary.wash_basin_present)
            || (summary.sleep_affordance_present != other_summary.sleep_affordance_present
                && other_summary.sleep_affordance_present)
            || (summary.food_source_present != other_summary.food_source_present
                && other_summary.food_source_present)
    })
}

fn detect_geographic_convergence(
    agent_stats: &BTreeMap<EntityId, AgentStats>,
    world: &worldwake_core::World,
    anomalies: &mut Vec<Anomaly>,
) {
    let sample_len = agent_stats
        .values()
        .map(|stats| stats.location_history.len())
        .max()
        .unwrap_or(0);
    if sample_len < ANOMALY_ROLLING_WINDOW_TICKS {
        return;
    }

    let mut merged_spans: BTreeMap<(BTreeSet<EntityId>, EntityId), (usize, usize)> =
        BTreeMap::new();

    for window_start in 0..=sample_len - ANOMALY_ROLLING_WINDOW_TICKS {
        let window_end = window_start + ANOMALY_ROLLING_WINDOW_TICKS - 1;
        let mut per_place_counts: BTreeMap<EntityId, BTreeMap<EntityId, usize>> = BTreeMap::new();

        for (agent_id, stats) in agent_stats {
            let Some(window) = stats.location_history.get(window_start..=window_end) else {
                continue;
            };

            let mut counts_by_place: BTreeMap<EntityId, usize> = BTreeMap::new();
            for place in window.iter().flatten() {
                *counts_by_place.entry(*place).or_insert(0) += 1;
            }

            for (place, count) in counts_by_place {
                per_place_counts
                    .entry(place)
                    .or_default()
                    .insert(*agent_id, count);
            }
        }

        for (place, counts_by_agent) in per_place_counts {
            let qualifying_agents = counts_by_agent
                .iter()
                .filter_map(|(agent_id, count)| {
                    let share_permille =
                        (*count as u32 * 1000) / ANOMALY_ROLLING_WINDOW_TICKS as u32;
                    (share_permille >= GEOGRAPHIC_CONVERGENCE_SHARE_THRESHOLD_PERMILLE)
                        .then_some(*agent_id)
                })
                .collect::<BTreeSet<_>>();

            if qualifying_agents.len() < 2 {
                continue;
            }
            if is_lawful_split_support_convergence_place(world, place) {
                continue;
            }

            let entry = merged_spans
                .entry((qualifying_agents, place))
                .or_insert((window_start, window_end));
            entry.0 = entry.0.min(window_start);
            entry.1 = entry.1.max(window_end);
        }
    }

    for ((agent_ids, place), (start_tick, end_tick)) in merged_spans {
        let Some((&lead_agent, remaining_agents)) = agent_ids.iter().next().map(|lead| {
            (
                lead,
                agent_ids.iter().copied().skip(1).collect::<Vec<EntityId>>(),
            )
        }) else {
            continue;
        };

        let Some(lead_stats) = agent_stats.get(&lead_agent) else {
            continue;
        };
        let Some(window) = lead_stats.location_history.get(start_tick..=end_tick) else {
            continue;
        };
        let lead_share_ticks = window
            .iter()
            .filter(|sample| **sample == Some(place))
            .count();
        let span_ticks = end_tick - start_tick + 1;
        let lead_share_ticks_u32 = u32::try_from(lead_share_ticks).unwrap_or(u32::MAX);
        let span_ticks_u32 = u32::try_from(span_ticks).unwrap_or(u32::MAX);
        let lead_share_percent =
            (f64::from(lead_share_ticks_u32) * 100.0) / f64::from(span_ticks_u32);

        anomalies.push(Anomaly {
            kind: AnomalyKind::GeographicConvergence,
            agent_name: lead_stats.name.clone(),
            additional_agent_names: Some(
                remaining_agents
                    .into_iter()
                    .filter_map(|agent_id| {
                        agent_stats.get(&agent_id).map(|stats| stats.name.clone())
                    })
                    .collect(),
            ),
            description: format!(
                "{} agents spent at least {:.1}% of ticks {}–{} at {} (lead agent share: {:.1}%).",
                agent_ids.len(),
                f64::from(GEOGRAPHIC_CONVERGENCE_SHARE_THRESHOLD_PERMILLE) / 10.0,
                start_tick,
                end_tick,
                entity_display_name(world, place),
                lead_share_percent,
            ),
            tick_range: Some((start_tick as u64, end_tick as u64)),
        });
    }
}

const MAINTENANCE_STARVATION_NEEDS: [HomeostaticNeedId; 5] = [
    HomeostaticNeedId::Hunger,
    HomeostaticNeedId::Thirst,
    HomeostaticNeedId::Fatigue,
    HomeostaticNeedId::Bladder,
    HomeostaticNeedId::Dirtiness,
];

fn need_label(need: HomeostaticNeedId) -> &'static str {
    match need {
        HomeostaticNeedId::Hunger => "hunger",
        HomeostaticNeedId::Thirst => "thirst",
        HomeostaticNeedId::Fatigue => "fatigue",
        HomeostaticNeedId::Bladder => "bladder",
        HomeostaticNeedId::Dirtiness => "dirtiness",
    }
}

fn need_table_label(need: HomeostaticNeedId) -> &'static str {
    match need {
        HomeostaticNeedId::Hunger => "Hunger",
        HomeostaticNeedId::Thirst => "Thirst",
        HomeostaticNeedId::Fatigue => "Fatigue",
        HomeostaticNeedId::Bladder => "Bladder",
        HomeostaticNeedId::Dirtiness => "Dirtiness",
    }
}

fn need_value(sample: &NeedsSample, need: HomeostaticNeedId) -> u16 {
    match need {
        HomeostaticNeedId::Hunger => sample.hunger,
        HomeostaticNeedId::Thirst => sample.thirst,
        HomeostaticNeedId::Fatigue => sample.fatigue,
        HomeostaticNeedId::Bladder => sample.bladder,
        HomeostaticNeedId::Dirtiness => sample.dirtiness,
    }
}

fn need_high_threshold(
    thresholds: &worldwake_core::DriveThresholds,
    need: HomeostaticNeedId,
) -> u16 {
    match need {
        HomeostaticNeedId::Hunger => thresholds.hunger.high().value(),
        HomeostaticNeedId::Thirst => thresholds.thirst.high().value(),
        HomeostaticNeedId::Fatigue => thresholds.fatigue.high().value(),
        HomeostaticNeedId::Bladder => thresholds.bladder.high().value(),
        HomeostaticNeedId::Dirtiness => thresholds.dirtiness.high().value(),
    }
}

fn maintenance_window_stats(samples: &[NeedsSample], need: HomeostaticNeedId) -> (u32, u32, u32) {
    let deltas = samples
        .windows(2)
        .map(|pair| i32::from(need_value(&pair[1], need)) - i32::from(need_value(&pair[0], need)))
        .collect::<Vec<_>>();
    let accumulation = deltas
        .iter()
        .copied()
        .filter(|delta| *delta > 0)
        .map(i32::cast_unsigned)
        .sum::<u32>();
    let relief = deltas
        .iter()
        .copied()
        .filter(|delta| *delta < 0)
        .map(i32::unsigned_abs)
        .sum::<u32>();
    let avg = samples
        .iter()
        .map(|sample| u32::from(need_value(sample, need)))
        .sum::<u32>()
        / samples.len() as u32;

    (accumulation, relief, avg)
}

#[derive(Clone, Copy)]
struct MaintenanceStarvationWindow {
    start_tick: usize,
    end_tick: usize,
    accumulation: u32,
    relief: u32,
    avg: u32,
}

fn maintenance_window_is_starvation(
    accumulation: u32,
    relief: u32,
    avg: u32,
    high_threshold: u16,
) -> bool {
    accumulation > 0 && avg > u32::from(high_threshold) && relief.saturating_mul(2) < accumulation
}

fn maintenance_window_is_better(
    candidate: MaintenanceStarvationWindow,
    current: MaintenanceStarvationWindow,
) -> bool {
    let candidate_deficit = candidate.accumulation.saturating_sub(candidate.relief);
    let current_deficit = current.accumulation.saturating_sub(current.relief);
    candidate_deficit > current_deficit
        || (candidate_deficit == current_deficit
            && (candidate.avg > current.avg
                || (candidate.avg == current.avg && candidate.start_tick < current.start_tick)))
}

fn compute_maintenance_rates(samples: &[NeedsSample]) -> [(HomeostaticNeedId, u32, u32, i64); 5] {
    MAINTENANCE_STARVATION_NEEDS.map(|need| {
        let mut accumulation = 0u32;
        let mut relief = 0u32;
        for pair in samples.windows(2) {
            let delta =
                i32::from(need_value(&pair[1], need)) - i32::from(need_value(&pair[0], need));
            if delta > 0 {
                accumulation += delta.cast_unsigned();
            } else if delta < 0 {
                relief += delta.unsigned_abs();
            }
        }
        (
            need,
            accumulation,
            relief,
            i64::from(accumulation) - i64::from(relief),
        )
    })
}

fn render_maintenance_rates_table(samples: &[NeedsSample]) -> Option<String> {
    if samples.is_empty() {
        return None;
    }

    let mut out = String::new();
    writeln!(out, "**Maintenance rates** (‰)\n").unwrap();
    writeln!(out, "| Need | Accumulation | Relief | Net |").unwrap();
    writeln!(out, "|------|--------------|--------|-----|").unwrap();
    for (need, accumulation, relief, net) in compute_maintenance_rates(samples) {
        writeln!(
            out,
            "| {} | {} | {} | {} |",
            need_table_label(need),
            accumulation,
            relief,
            net
        )
        .unwrap();
    }
    writeln!(out).unwrap();

    Some(out)
}

fn detect_maintenance_starvation(
    agent_stats: &BTreeMap<EntityId, AgentStats>,
    thresholds_by_agent: &BTreeMap<EntityId, worldwake_core::DriveThresholds>,
    anomalies: &mut Vec<Anomaly>,
) {
    let mut strongest_windows: BTreeMap<
        (EntityId, HomeostaticNeedId),
        MaintenanceStarvationWindow,
    > = BTreeMap::new();

    for (agent_id, stats) in agent_stats {
        if stats.needs_samples.len() < ANOMALY_ROLLING_WINDOW_TICKS {
            continue;
        }
        let Some(thresholds) = thresholds_by_agent.get(agent_id) else {
            continue;
        };

        for need in MAINTENANCE_STARVATION_NEEDS {
            let high_threshold = need_high_threshold(thresholds, need);
            for window_start in 0..=stats.needs_samples.len() - ANOMALY_ROLLING_WINDOW_TICKS {
                let window_end = window_start + ANOMALY_ROLLING_WINDOW_TICKS - 1;
                let Some(window) = stats.needs_samples.get(window_start..=window_end) else {
                    continue;
                };
                let (accumulation, relief, avg) = maintenance_window_stats(window, need);
                if maintenance_window_is_starvation(accumulation, relief, avg, high_threshold) {
                    let candidate = MaintenanceStarvationWindow {
                        start_tick: window_start,
                        end_tick: window_end,
                        accumulation,
                        relief,
                        avg,
                    };
                    match strongest_windows.entry((*agent_id, need)) {
                        std::collections::btree_map::Entry::Vacant(entry) => {
                            entry.insert(candidate);
                        }
                        std::collections::btree_map::Entry::Occupied(mut entry) => {
                            if maintenance_window_is_better(candidate, *entry.get()) {
                                entry.insert(candidate);
                            }
                        }
                    }
                }
            }
        }
    }

    for ((agent_id, need), window) in strongest_windows {
        let Some(stats) = agent_stats.get(&agent_id) else {
            continue;
        };
        let Some(thresholds) = thresholds_by_agent.get(&agent_id) else {
            continue;
        };
        let high_threshold = need_high_threshold(thresholds, need);
        let need_label = need_label(need);
        let deficit = window.accumulation.saturating_sub(window.relief);

        anomalies.push(Anomaly {
            kind: AnomalyKind::MaintenanceStarvation,
            agent_name: stats.name.clone(),
            additional_agent_names: None,
            description: format!(
                "{need_label} accumulated {accumulation} permille but was relieved only {relief} permille over ticks {start_tick}–{end_tick}. Net deficit: {deficit} permille; average {need_label} in window: {avg} permille (above high threshold {high_threshold}).",
                accumulation = window.accumulation,
                relief = window.relief,
                start_tick = window.start_tick,
                end_tick = window.end_tick,
                avg = window.avg,
            ),
            tick_range: Some((window.start_tick as u64, window.end_tick as u64)),
        });
    }
}

const RECIPE_MONOCULTURE_SHARE_THRESHOLD_PERMILLE: u32 = 950;

fn primary_satisfied_need(recipe: &RecipeDefinition) -> Option<HomeostaticNeedId> {
    let (commodity, _) = recipe.outputs.first()?;
    let spec = commodity.spec();
    let _profile = spec.consumable_profile?;
    match spec.trade_category {
        worldwake_core::TradeCategory::Food => Some(HomeostaticNeedId::Hunger),
        worldwake_core::TradeCategory::Water => Some(HomeostaticNeedId::Thirst),
        _ => None,
    }
}

fn recipe_facility_evidence_label(
    recipe: &RecipeDefinition,
    belief_store: &AgentBeliefStore,
) -> Option<String> {
    if let Some(tag) = recipe.required_workstation_tag
        && belief_store
            .known_entities
            .values()
            .any(|state| state.workstation_tag == Some(tag))
    {
        return Some(format!("workstation {tag:?}"));
    }

    let (primary_input, _) = recipe.inputs.first()?;
    belief_store
        .known_entities
        .values()
        .find(|state| {
            state
                .resource_source
                .as_ref()
                .is_some_and(|source| source.commodity == *primary_input)
        })
        .map(|_| format!("resource source {primary_input:?}"))
}

fn agent_believes_recipe_facility_reachable(
    recipe: &RecipeDefinition,
    belief_store: &AgentBeliefStore,
) -> bool {
    recipe_facility_evidence_label(recipe, belief_store).is_some()
}

fn action_name_recipe_name(action_name: &str) -> &str {
    action_name
        .strip_prefix("harvest:")
        .or_else(|| action_name.strip_prefix("craft:"))
        .unwrap_or(action_name)
}

fn recipe_commit_count(agent_stats: &AgentStats, recipe_name: &str) -> u32 {
    agent_stats
        .actions_committed
        .iter()
        .filter(|(action_name, _)| action_name_recipe_name(action_name) == recipe_name)
        .map(|(_, commits)| *commits)
        .sum()
}

fn recipe_usage_rows(
    agent_stats: &AgentStats,
    known_recipes: Option<&KnownRecipes>,
    registry: &RecipeRegistry,
) -> Vec<(String, u32)> {
    let mut rows = Vec::new();
    let known_recipes = known_recipes.map(|known| &known.recipes);
    let mut known_names = BTreeSet::new();

    if let Some(recipe_ids) = known_recipes {
        for recipe_id in recipe_ids {
            let recipe_name = registry.get(*recipe_id).map_or_else(
                || format!("Recipe#{} (unknown)", recipe_id.0),
                |def| def.name.clone(),
            );
            let commits = registry
                .get(*recipe_id)
                .map_or(0, |def| recipe_commit_count(agent_stats, &def.name));
            known_names.insert(recipe_name.clone());
            rows.push((recipe_name, commits));
        }
    }

    for (action_name, commits) in &agent_stats.actions_committed {
        if *commits == 0 {
            continue;
        }
        let recipe_name = action_name_recipe_name(action_name);
        if known_names.contains(recipe_name) {
            continue;
        }
        let Some((recipe_id, _)) = registry.recipe_by_name(recipe_name) else {
            continue;
        };
        if known_recipes.is_some_and(|recipes| recipes.contains(&recipe_id)) {
            continue;
        }
        rows.push((format!("{recipe_name} (unknown)"), *commits));
    }

    rows
}

fn render_recipe_usage_table(
    agent_stats: &AgentStats,
    known_recipes: Option<&KnownRecipes>,
    registry: &RecipeRegistry,
) -> Option<String> {
    let rows = recipe_usage_rows(agent_stats, known_recipes, registry);
    if rows.is_empty() {
        return None;
    }

    let mut out = String::new();
    writeln!(out, "**Recipe usage**\n").unwrap();
    writeln!(out, "| Recipe | Commits |").unwrap();
    writeln!(out, "|--------|---------|").unwrap();
    for (recipe_name, commits) in rows {
        writeln!(out, "| {recipe_name} | {commits} |").unwrap();
    }
    writeln!(out).unwrap();

    Some(out)
}

fn detect_recipe_monoculture(
    agent_stats: &BTreeMap<EntityId, AgentStats>,
    known_recipes_by_agent: &BTreeMap<EntityId, BTreeSet<RecipeId>>,
    recipe_registry: &RecipeRegistry,
    world: &worldwake_core::World,
    anomalies: &mut Vec<Anomaly>,
) {
    for (agent_id, stats) in agent_stats {
        let Some(belief_store) = world.get_component_agent_belief_store(*agent_id) else {
            continue;
        };
        let Some(known_recipes) = known_recipes_by_agent.get(agent_id) else {
            continue;
        };

        let mut recipes_by_need: BTreeMap<HomeostaticNeedId, Vec<RecipeId>> = BTreeMap::new();
        for recipe_id in known_recipes {
            let Some(recipe) = recipe_registry.get(*recipe_id) else {
                continue;
            };
            let Some(need) = primary_satisfied_need(recipe) else {
                continue;
            };
            recipes_by_need.entry(need).or_default().push(*recipe_id);
        }

        for (need, recipe_ids) in recipes_by_need {
            if recipe_ids.len() < 2 {
                continue;
            }

            let mut recipe_counts = recipe_ids
                .into_iter()
                .filter_map(|recipe_id| {
                    let recipe = recipe_registry.get(recipe_id)?;
                    Some((
                        recipe_id,
                        recipe.name.clone(),
                        recipe_commit_count(stats, &recipe.name),
                    ))
                })
                .collect::<Vec<_>>();
            recipe_counts.sort_by(|left, right| {
                right
                    .2
                    .cmp(&left.2)
                    .then_with(|| left.1.cmp(&right.1))
                    .then_with(|| left.0.cmp(&right.0))
            });

            let total_bucket_commits = recipe_counts
                .iter()
                .map(|(_, _, count)| *count)
                .sum::<u32>();
            if total_bucket_commits == 0 {
                continue;
            }

            let Some((_, _, top_count)) = recipe_counts.first() else {
                continue;
            };
            let top_share_permille = (*top_count * 1000) / total_bucket_commits;
            if top_share_permille < RECIPE_MONOCULTURE_SHARE_THRESHOLD_PERMILLE {
                continue;
            }

            let alternative_evidence =
                recipe_counts.iter().skip(1).find_map(|(recipe_id, _, _)| {
                    let recipe = recipe_registry.get(*recipe_id)?;
                    agent_believes_recipe_facility_reachable(recipe, belief_store).then(|| {
                        recipe_facility_evidence_label(recipe, belief_store)
                            .unwrap_or_else(|| "alternative facility".to_string())
                    })
                });
            let Some(alternative_evidence_label) = alternative_evidence else {
                continue;
            };

            let parts = recipe_counts
                .iter()
                .map(|(_, recipe_name, count)| {
                    let share_percent =
                        (f64::from(*count) * 100.0) / f64::from(total_bucket_commits);
                    format!("{share_percent:.0}% {recipe_name} ({count} actions)")
                })
                .collect::<Vec<_>>();
            let run_end_tick = stats
                .needs_samples
                .len()
                .checked_sub(1)
                .map_or(0, |tick| tick as u64);

            anomalies.push(Anomaly {
                kind: AnomalyKind::RecipeMonoculture,
                agent_name: stats.name.clone(),
                additional_agent_names: None,
                description: format!(
                    "{} actions: {}. Both recipes known; final belief store includes {} evidence.",
                    need_label(need),
                    parts.join(", "),
                    alternative_evidence_label
                ),
                tick_range: Some((0, run_end_tick)),
            });
        }
    }
}

fn acute_need_tolerance_context(
    need: HomeostaticNeedId,
    metabolism: &MetabolismProfile,
) -> Option<(&'static str, u32)> {
    match need {
        HomeostaticNeedId::Hunger => {
            Some(("starvation", metabolism.starvation_tolerance_ticks.get()))
        }
        HomeostaticNeedId::Thirst => {
            Some(("dehydration", metabolism.dehydration_tolerance_ticks.get()))
        }
        HomeostaticNeedId::Fatigue | HomeostaticNeedId::Bladder | HomeostaticNeedId::Dirtiness => {
            None
        }
    }
}

fn acute_need_spike_description(
    need: HomeostaticNeedId,
    critical_permille: u16,
    run_length: usize,
    run_start: usize,
    run_end: usize,
    peak: u16,
    metabolism: &MetabolismProfile,
) -> String {
    let need_label = need_label(need);
    let mut description = format!(
        "{need_label} above critical threshold ({critical_permille} permille) for {run_length} consecutive ticks (ticks {run_start}–{run_end}), peak {peak} permille. Below the 100-tick sustained-critical bar"
    );

    if let Some((tolerance_label, tolerance_ticks)) = acute_need_tolerance_context(need, metabolism)
    {
        let run_length_u32 = u32::try_from(run_length).expect("acute run length fits in u32");
        let percent_of_tolerance = (f64::from(run_length_u32) * 100.0) / f64::from(tolerance_ticks);
        write!(
            description,
            " but within {percent_of_tolerance:.0}% of {tolerance_label} tolerance ({tolerance_ticks} ticks)."
        )
        .unwrap();
    } else {
        description.push('.');
    }

    description
}

fn detect_acute_need_spike(
    agent_stats: &BTreeMap<EntityId, AgentStats>,
    thresholds_by_agent: &BTreeMap<EntityId, worldwake_core::DriveThresholds>,
    metabolism_by_agent: &BTreeMap<EntityId, MetabolismProfile>,
    anomalies: &mut Vec<Anomaly>,
) {
    for (agent_id, stats) in agent_stats {
        let Some(thresholds) = thresholds_by_agent.get(agent_id) else {
            continue;
        };
        let Some(metabolism) = metabolism_by_agent.get(agent_id) else {
            continue;
        };

        for need in MAINTENANCE_STARVATION_NEEDS {
            let critical_permille = thresholds.critical(need).value();
            let mut run_start = None;
            let mut peak = 0;

            for (tick, sample) in stats.needs_samples.iter().enumerate() {
                let value = need_value(sample, need);
                if value >= critical_permille {
                    run_start.get_or_insert(tick);
                    peak = peak.max(value);
                    continue;
                }

                if let Some(start_tick) = run_start.take() {
                    let run_end = tick.saturating_sub(1);
                    let run_length = run_end - start_tick + 1;
                    if (ACUTE_NEED_SPIKE_MIN_TICKS..ACUTE_NEED_SPIKE_MAX_TICKS)
                        .contains(&run_length)
                    {
                        anomalies.push(Anomaly {
                            kind: AnomalyKind::AcuteNeedSpike,
                            agent_name: stats.name.clone(),
                            additional_agent_names: None,
                            description: acute_need_spike_description(
                                need,
                                critical_permille,
                                run_length,
                                start_tick,
                                run_end,
                                peak,
                                metabolism,
                            ),
                            tick_range: Some((start_tick as u64, run_end as u64)),
                        });
                    }
                    peak = 0;
                }
            }

            if let Some(start_tick) = run_start.take() {
                let run_end = stats.needs_samples.len().saturating_sub(1);
                let run_length = run_end - start_tick + 1;
                if (ACUTE_NEED_SPIKE_MIN_TICKS..ACUTE_NEED_SPIKE_MAX_TICKS).contains(&run_length) {
                    anomalies.push(Anomaly {
                        kind: AnomalyKind::AcuteNeedSpike,
                        agent_name: stats.name.clone(),
                        additional_agent_names: None,
                        description: acute_need_spike_description(
                            need,
                            critical_permille,
                            run_length,
                            start_tick,
                            run_end,
                            peak,
                            metabolism,
                        ),
                        tick_range: Some((start_tick as u64, run_end as u64)),
                    });
                }
            }
        }
    }
}

type NeedExtractor = (&'static str, fn(&NeedsSample) -> u16);
type NeedActionPair = (
    &'static str,
    fn(&NeedsSample) -> u16,
    &'static [&'static str],
);

/// Detect needs that stay above 750 permille for 100+ consecutive ticks.
fn detect_sustained_critical_needs(stats: &AgentStats, anomalies: &mut Vec<Anomaly>) {
    const THRESHOLD: u16 = 750;
    const MIN_CONSECUTIVE: u32 = 100;

    let need_extractors: &[NeedExtractor] = &[
        ("hunger", |s| s.hunger),
        ("thirst", |s| s.thirst),
        ("fatigue", |s| s.fatigue),
        ("bladder", |s| s.bladder),
        ("dirtiness", |s| s.dirtiness),
    ];

    for &(need_name, extractor) in need_extractors {
        let mut consecutive: u32 = 0;
        let mut max_consecutive: u32 = 0;
        let mut max_end_tick: u32 = 0;
        for (i, sample) in stats.needs_samples.iter().enumerate() {
            if extractor(sample) > THRESHOLD {
                consecutive += 1;
                if consecutive > max_consecutive {
                    max_consecutive = consecutive;
                    max_end_tick = i as u32;
                }
            } else {
                consecutive = 0;
            }
        }
        if max_consecutive >= MIN_CONSECUTIVE {
            let start_tick = max_end_tick.saturating_sub(max_consecutive) + 1;
            anomalies.push(Anomaly {
                kind: AnomalyKind::SustainedCriticalNeed,
                agent_name: stats.name.clone(),
                additional_agent_names: None,
                description: format!(
                    "{need_name} above {THRESHOLD}‰ for {max_consecutive} consecutive ticks (ticks {start_tick}–{max_end_tick})"
                ),
                tick_range: Some((u64::from(start_tick), u64::from(max_end_tick))),
            });
        }
    }
}

/// Detect needs with avg > 750 but no corresponding relief action attempted.
fn detect_unaddressed_needs(stats: &AgentStats, anomalies: &mut Vec<Anomaly>) {
    if stats.needs_samples.is_empty() {
        return;
    }
    let len = stats.needs_samples.len() as u32;

    let need_action_pairs: &[NeedActionPair] = &[
        ("hunger", |s| s.hunger, &["eat"]),
        ("thirst", |s| s.thirst, &["drink"]),
        ("fatigue", |s| s.fatigue, &["sleep"]),
        ("bladder", |s| s.bladder, &["toilet", "relieve_wilderness"]),
        ("dirtiness", |s| s.dirtiness, &["wash"]),
    ];

    for &(need_name, extractor, relief_actions) in need_action_pairs {
        let avg: u32 = stats
            .needs_samples
            .iter()
            .map(|s| u32::from(extractor(s)))
            .sum::<u32>()
            / len;
        if avg > 750 {
            let any_attempted = relief_actions.iter().any(|action| {
                let started = stats.actions_started.get(*action).copied().unwrap_or(0);
                let failed = stats
                    .actions_start_failed
                    .get(*action)
                    .copied()
                    .unwrap_or(0);
                started + failed > 0
            });
            if !any_attempted {
                anomalies.push(Anomaly {
                    kind: AnomalyKind::UnaddressedNeed,
                    agent_name: stats.name.clone(),
                    additional_agent_names: None,
                    description: format!(
                        "{need_name} avg {avg}‰ but no relief action ({}) was ever attempted",
                        relief_actions.join("/")
                    ),
                    tick_range: None,
                });
            }
        }
    }
}

/// Look for repeating subsequences in the action history.
/// Check window sizes 2..=6: if the last 2*W actions are the same W-action
/// sequence repeated, that's a loop.
fn detect_action_loop(sequence: &[String]) -> Option<String> {
    for window_size in 2..=6 {
        let needed = window_size * 3; // require at least 3 repetitions
        if sequence.len() < needed {
            continue;
        }
        let tail = &sequence[sequence.len() - needed..];
        let pattern = &tail[..window_size];
        let mut is_loop = true;
        for chunk in tail.chunks_exact(window_size) {
            if chunk != pattern {
                is_loop = false;
                break;
            }
        }
        if is_loop {
            return Some(format!(
                "Repeating action loop detected (length {}): [{}] repeated {} times",
                window_size,
                pattern.join(" → "),
                needed / window_size
            ));
        }
    }
    None
}

/// Refine redundant perception anomalies by removing cases where the
/// observation rate is ~1 per tick (normal event witnessing from co-location).
///
/// When `observation_count / distinct_ticks <= 1.5`, the agent is seeing the
/// entity roughly once per tick through ordinary event witnessing — expected
/// behavior when agents are co-located. We remove these anomalies.
fn refine_redundant_perception(
    anomalies: &mut Vec<Anomaly>,
    agent_stats: &BTreeMap<EntityId, AgentStats>,
    _perception_trace: &PerceptionTraceSink,
    _event_log: &worldwake_core::EventLog,
) {
    anomalies.retain(|a| {
        if !matches!(a.kind, AnomalyKind::RedundantPerception) {
            return true;
        }
        // Find the matching agent stats.
        let Some(stats) = agent_stats.values().find(|s| s.name == a.agent_name) else {
            return true;
        };
        // Parse entity id from the description (format: "Observed entity eXgY ...")
        // Instead of parsing, iterate entity counts and match by description content.
        for (entity, count) in &stats.observation_entity_counts {
            if !a.description.contains(&format!("entity {entity}")) {
                continue;
            }
            let distinct_ticks = stats
                .observation_tick_sets
                .get(entity)
                .map_or(0, BTreeSet::len);
            if distinct_ticks == 0 {
                return true;
            }
            // If observations are roughly 1-per-tick, this is normal co-location
            // event witnessing, not genuinely redundant perception.
            // Use integer arithmetic: count * 2 <= distinct_ticks * 3 is ratio <= 1.5.
            let count_u64 = u64::from(*count);
            let ticks_u64 = distinct_ticks as u64;
            if count_u64 * 2 <= ticks_u64 * 3 {
                return false; // Remove this anomaly.
            }
        }
        true
    });
}

/// Refine stuck-agent anomalies by removing cases where the agent's needs
/// were all low during the idle window (the agent correctly had nothing to do).
///
/// An agent is not "stuck" if all five needs are below 300 permille at the
/// start of the idle window — there is genuinely no pressing goal.
fn refine_stuck_agents(anomalies: &mut Vec<Anomaly>, agent_stats: &BTreeMap<EntityId, AgentStats>) {
    const NEEDS_LOW_CEILING: u16 = 300;

    anomalies.retain(|a| {
        if !matches!(a.kind, AnomalyKind::StuckAgent) {
            return true;
        }
        let Some(stats) = agent_stats.values().find(|s| s.name == a.agent_name) else {
            return true;
        };
        // Check all idle windows that triggered the anomaly (>= 20 ticks).
        // If EVERY significant window started with all needs below the ceiling,
        // the agent was correctly idle.
        let significant_windows: Vec<_> = stats
            .idle_windows
            .iter()
            .filter(|w| (w.end_tick - w.start_tick + 1) as u32 >= 20)
            .collect();
        if significant_windows.is_empty() {
            return true;
        }
        let all_low = significant_windows.iter().all(|w| {
            w.needs_at_start.hunger <= NEEDS_LOW_CEILING
                && w.needs_at_start.thirst <= NEEDS_LOW_CEILING
                && w.needs_at_start.fatigue <= NEEDS_LOW_CEILING
                && w.needs_at_start.bladder <= NEEDS_LOW_CEILING
                && w.needs_at_start.dirtiness <= NEEDS_LOW_CEILING
        });
        !all_low // Keep the anomaly only if some window had elevated needs.
    });
}

// ---------------------------------------------------------------------------
// Output formatting
// ---------------------------------------------------------------------------

fn failed_plan_max_depth(attempt: &PlanAttemptTrace) -> u8 {
    attempt
        .expansion_summaries
        .iter()
        .map(|summary| summary.depth)
        .max()
        .unwrap_or(0)
}

fn failed_plan_candidates(attempt: &PlanAttemptTrace) -> u32 {
    attempt
        .expansion_summaries
        .iter()
        .map(|summary| u32::from(summary.candidates_generated))
        .sum()
}

fn failed_plan_location(place: Option<EntityId>) -> String {
    place.map_or_else(|| "?".to_string(), |place| place.to_string())
}

fn failed_plan_target_beliefs(attempt: &PlanAttemptTrace) -> &'static str {
    match attempt.target_belief_presence {
        TargetBeliefPresence::Present => "true",
        TargetBeliefPresence::Absent => "false",
        TargetBeliefPresence::NotApplicable => "n/a",
    }
}

fn action_timeline_bins_for_agent<'a>(
    action_trace: &'a ActionTraceSink,
    agent_id: EntityId,
) -> BTreeMap<u64, BTreeMap<&'a str, u32>> {
    let mut bins: BTreeMap<u64, BTreeMap<&'a str, u32>> = BTreeMap::new();
    for event in action_trace.events_for(agent_id).iter().filter(|event| {
        matches!(
            event.kind,
            ActionTraceKind::Started { .. } | ActionTraceKind::StartFailed { .. }
        )
    }) {
        let bin = event.tick.0 / 100;
        *bins
            .entry(bin)
            .or_default()
            .entry(&event.action_name)
            .or_insert(0) += 1;
    }
    bins
}

fn planning_affordance_snapshots<'a>(
    traces: &'a [&'a AgentDecisionTrace],
) -> Vec<(Tick, &'a AffordanceTrace)> {
    traces
        .iter()
        .filter_map(|trace| match &trace.outcome {
            DecisionOutcome::Planning(planning) => planning
                .affordances
                .as_ref()
                .map(|affordances| (trace.tick, affordances)),
            _ => None,
        })
        .collect()
}

fn committed_travel_ticks(events: &[&ActionTraceEvent]) -> Vec<Tick> {
    events
        .iter()
        .filter(|event| {
            event.action_name == "travel" && matches!(event.kind, ActionTraceKind::Committed { .. })
        })
        .map(|event| event.tick)
        .collect()
}

fn post_travel_affordance_snapshots<'a>(
    affordance_snapshots: &[(Tick, &'a AffordanceTrace)],
    travel_commit_ticks: &[Tick],
) -> Vec<(Tick, &'a AffordanceTrace)> {
    let mut snapshots = Vec::new();
    let mut last_recorded_place = None;
    let mut pending_post_travel = false;
    let mut travel_index = 0usize;

    for (tick, affordances) in affordance_snapshots {
        while let Some(travel_tick) = travel_commit_ticks.get(travel_index) {
            if *travel_tick >= *tick {
                break;
            }
            pending_post_travel = true;
            travel_index += 1;
        }

        if pending_post_travel {
            if affordances.place != last_recorded_place {
                snapshots.push((*tick, *affordances));
                last_recorded_place = affordances.place;
            }
            pending_post_travel = false;
        } else {
            last_recorded_place = affordances.place;
        }
    }

    snapshots
}

fn final_affordance_snapshot<'a>(
    affordance_snapshots: &[(Tick, &'a AffordanceTrace)],
) -> Option<(Tick, &'a AffordanceTrace)> {
    affordance_snapshots.last().copied()
}

struct AffordanceChangeEvent<'a> {
    tick: Tick,
    affordances: &'a AffordanceTrace,
    appeared: Vec<String>,
    disappeared: Vec<String>,
    place_changed: bool,
}

fn affordance_change_snapshots<'a>(
    affordance_snapshots: &[(Tick, &'a AffordanceTrace)],
) -> Vec<AffordanceChangeEvent<'a>> {
    let mut changes = Vec::new();

    for snapshots in affordance_snapshots.windows(2) {
        let (previous_tick, previous) = snapshots[0];
        let (current_tick, current) = snapshots[1];
        debug_assert!(current_tick >= previous_tick);

        let previous_names: BTreeSet<&str> = previous
            .available
            .iter()
            .map(|summary| summary.action_name.as_str())
            .collect();
        let current_names: BTreeSet<&str> = current
            .available
            .iter()
            .map(|summary| summary.action_name.as_str())
            .collect();

        let appeared: Vec<String> = current_names
            .difference(&previous_names)
            .map(|name| (*name).to_string())
            .collect();
        let disappeared: Vec<String> = previous_names
            .difference(&current_names)
            .map(|name| (*name).to_string())
            .collect();

        if appeared.is_empty() && disappeared.is_empty() {
            continue;
        }

        changes.push(AffordanceChangeEvent {
            tick: current_tick,
            affordances: current,
            appeared,
            disappeared,
            place_changed: previous.place != current.place,
        });
    }

    changes
}

fn format_affordance_summary(summary: &AffordanceSummary) -> String {
    if summary.target_count == 0 {
        summary.action_name.clone()
    } else {
        format!("{} ({} targets)", summary.action_name, summary.target_count)
    }
}

fn write_affordance_list(
    out: &mut String,
    heading: &str,
    affordances: &AffordanceTrace,
) -> std::fmt::Result {
    writeln!(out, "{heading}\n")?;
    for affordance in &affordances.available {
        writeln!(out, "- {}", format_affordance_summary(affordance))?;
    }
    writeln!(out)
}

fn believed_location_parts(world: &worldwake_core::World, entities: &[EntityId]) -> Vec<String> {
    let mut commodity_totals: BTreeMap<String, u64> = BTreeMap::new();
    let mut non_item_names: Vec<String> = Vec::new();

    for id in entities {
        if let Some(lot) = world.get_component_item_lot(*id) {
            *commodity_totals
                .entry(format!("{:?}", lot.commodity))
                .or_insert(0) += u64::from(lot.quantity.0);
        } else {
            non_item_names.push(entity_display_name(world, *id));
        }
    }

    let mut parts: Vec<String> = Vec::new();
    parts.extend(non_item_names);
    for (commodity, total) in &commodity_totals {
        parts.push(format!("{total}\u{00d7} {commodity}"));
    }
    parts
}

fn unknown_location_entity_groups(
    entities: &[EntityId],
    store: &worldwake_core::AgentBeliefStore,
) -> Vec<(String, Vec<EntityId>)> {
    let mut place_entities = Vec::new();
    let mut unknown_entities = Vec::new();

    for entity in entities {
        match store
            .known_entities
            .get(entity)
            .and_then(|state| state.believed_kind)
        {
            Some(EntityKind::Place) => place_entities.push(*entity),
            _ => unknown_entities.push(*entity),
        }
    }

    let mut groups = Vec::new();
    if !place_entities.is_empty() {
        groups.push((
            "(place entity \u{2014} no parent location)".to_string(),
            place_entities,
        ));
    }
    if !unknown_entities.is_empty() {
        groups.push(("Unknown location".to_string(), unknown_entities));
    }
    groups
}

fn behavioral_transitions(
    bins: &BTreeMap<u64, BTreeMap<&str, u32>>,
    needs_samples: &[NeedsSample],
) -> Vec<BehavioralTransition> {
    let mut transitions = Vec::new();
    let mut previous: Option<(u64, usize)> = None;

    for (bin, action_counts) in bins {
        let current_types = action_counts.len();
        if let Some((_, previous_types)) = previous
            && previous_types > 0
            && current_types * 2 <= previous_types
            && !needs_samples.is_empty()
        {
            let tick = bin * 100;
            let needs_index = usize::min(tick as usize, needs_samples.len() - 1);
            transitions.push(BehavioralTransition {
                tick,
                types_before: previous_types,
                types_after: current_types,
                needs: needs_samples[needs_index],
            });
        }
        previous = Some((*bin, current_types));
    }

    transitions
}

fn format_behavioral_transition(transition: &BehavioralTransition) -> String {
    format!(
        "**Behavioral transition** at tick {}: action repertoire narrowed ({} types -> {} types)\n  Needs: hunger={}, thirst={}, fatigue={}, bladder={}, dirtiness={}",
        transition.tick,
        transition.types_before,
        transition.types_after,
        transition.needs.hunger,
        transition.needs.thirst,
        transition.needs.fatigue,
        transition.needs.bladder,
        transition.needs.dirtiness
    )
}

fn failed_plan_outcome_label(attempt: &PlanAttemptTrace) -> String {
    match &attempt.outcome {
        PlanSearchOutcome::FrontierExhausted { expansions_used } => {
            if *expansions_used <= 1
                && let Some(summary) = attempt
                    .expansion_summaries
                    .iter()
                    .find(|summary| summary.depth == 0)
            {
                let generated = summary.candidates_generated;
                let skipped = summary.candidates_skipped;
                let terminal = summary.terminal_successors;
                let after_beam = summary.non_terminal_after_beam;
                if generated == 0 {
                    return "frontier-exhausted at depth 0: 0 candidates generated".to_string();
                }
                if skipped == generated {
                    return format!(
                        "frontier-exhausted at depth 0: {generated} candidates generated, all skipped (build_successor returned None)"
                    );
                }
                if after_beam == 0 && terminal == 0 {
                    return format!(
                        "frontier-exhausted at depth 0: {generated} candidates generated, all pruned by beam"
                    );
                }
                return format!(
                    "frontier-exhausted at depth 0: {generated} generated, {skipped} skipped, {terminal} terminal, {after_beam} after beam"
                );
            }
            "frontier-exhausted".to_string()
        }
        PlanSearchOutcome::BudgetExhausted { .. } => "budget-exhausted".to_string(),
        PlanSearchOutcome::Found { .. } | PlanSearchOutcome::Unsupported => unreachable!(),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FailedPlanBreakdown {
    total: u32,
    frontier_exhausted: u32,
    budget_exhausted: u32,
    max_depth_zero: u32,
    target_beliefs_false: u32,
}

fn failed_plan_breakdown(attempts: &[&PlanAttemptTrace]) -> FailedPlanBreakdown {
    let mut breakdown = FailedPlanBreakdown {
        total: attempts.len() as u32,
        frontier_exhausted: 0,
        budget_exhausted: 0,
        max_depth_zero: 0,
        target_beliefs_false: 0,
    };

    for attempt in attempts {
        match attempt.outcome {
            PlanSearchOutcome::FrontierExhausted { .. } => breakdown.frontier_exhausted += 1,
            PlanSearchOutcome::BudgetExhausted { .. } => breakdown.budget_exhausted += 1,
            _ => {}
        }
        if failed_plan_max_depth(attempt) == 0 {
            breakdown.max_depth_zero += 1;
        }
        if matches!(attempt.target_belief_presence, TargetBeliefPresence::Absent) {
            breakdown.target_beliefs_false += 1;
        }
    }

    breakdown
}

fn collect_failed_plan_attempts<'a>(
    traces: &'a [&'a AgentDecisionTrace],
) -> Vec<(u64, Option<EntityId>, &'a PlanAttemptTrace)> {
    traces
        .iter()
        .flat_map(|trace| match &trace.outcome {
            DecisionOutcome::Planning(planning) => planning
                .planning
                .attempts
                .iter()
                .filter(|attempt| {
                    matches!(
                        attempt.outcome,
                        PlanSearchOutcome::FrontierExhausted { .. }
                            | PlanSearchOutcome::BudgetExhausted { .. }
                    )
                })
                .map(|attempt| {
                    (
                        trace.tick.0,
                        planning.affordances.as_ref().and_then(|a| a.place),
                        attempt,
                    )
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect()
}

fn format_death_cause(cause: DeathCause) -> String {
    match cause {
        DeathCause::NeedDeprivation { need } => format!("NeedDeprivation {{ {need:?} }}"),
        DeathCause::CombatWounds => "CombatWounds".to_string(),
    }
}

fn death_summary_line(world: &worldwake_core::World, agent_id: EntityId) -> Option<String> {
    world
        .get_component_dead_at(agent_id)
        .map(|DeadAt { tick, cause }| {
            format!(
                "**Death**: Tick {} (cause: {})",
                tick.0,
                format_death_cause(*cause)
            )
        })
}

fn format_agenda_goal(entry: &AgendaEntry) -> String {
    format!("{:?}", entry.key.goal_key.kind)
}

fn format_revival_trigger(world: &worldwake_core::World, trigger: &RevivalTrigger) -> String {
    match trigger {
        RevivalTrigger::CommodityAvailable { place, kind, min } => format!(
            "commodity {:?} x{} at {}",
            kind,
            min.0,
            entity_display_name(world, *place)
        ),
        RevivalTrigger::TargetPresent { target, place } => format!(
            "target {} at {}",
            entity_display_name(world, *target),
            entity_display_name(world, *place)
        ),
        RevivalTrigger::RouteLearned { from, to } => format!(
            "route {} -> {}",
            entity_display_name(world, *from),
            entity_display_name(world, *to)
        ),
        RevivalTrigger::CounterpartyAvailable {
            counterparty,
            place,
        } => format!(
            "counterparty {} at {}",
            entity_display_name(world, *counterparty),
            entity_display_name(world, *place)
        ),
        RevivalTrigger::TickElapsed { at_tick } => format!("tick {}", at_tick.0),
    }
}

fn format_kill_condition(world: &worldwake_core::World, kill: &KillCondition) -> String {
    match kill {
        KillCondition::TickExpiry { at_tick } => format!("expires at tick {}", at_tick.0),
        KillCondition::ObligationResolved { expectation } => {
            format!("until expectation {expectation:?} resolves")
        }
        KillCondition::TargetDead { target } => {
            format!("until {} dies", entity_display_name(world, *target))
        }
        KillCondition::External => "external".to_string(),
    }
}

fn write_agenda_state_summary(
    out: &mut String,
    world: &worldwake_core::World,
    agenda_state: &AgendaState,
) {
    let committed = agenda_state
        .committed
        .as_ref()
        .map_or_else(|| "none".to_string(), format_agenda_goal);
    writeln!(
        out,
        "**Agenda state**: committed={committed}, pending={}, suspended={}",
        agenda_state.pending.len(),
        agenda_state.suspended.len()
    )
    .unwrap();

    if !agenda_state.pending.is_empty() {
        writeln!(out, "**Pending goals**:").unwrap();
        for entry in agenda_state.pending.values() {
            let trigger = entry.revival_trigger.as_ref().map_or_else(
                || "none".to_string(),
                |trigger| format_revival_trigger(world, trigger),
            );
            writeln!(
                out,
                "- {} | revive on {}",
                format_agenda_goal(entry),
                trigger
            )
            .unwrap();
        }
    }

    if !agenda_state.suspended.is_empty() {
        writeln!(out, "**Suspended goals**:").unwrap();
        for entry in agenda_state.suspended.values() {
            writeln!(
                out,
                "- {} | {}",
                format_agenda_goal(entry),
                format_kill_condition(world, &entry.kill_condition)
            )
            .unwrap();
        }
    }

    writeln!(out).unwrap();
}

#[allow(clippy::too_many_arguments)]
fn format_report(
    scenario_path: &str,
    seed: u64,
    tick_count: u64,
    agents: &[(EntityId, String)],
    places: &[(EntityId, String)],
    agent_stats: &BTreeMap<EntityId, AgentStats>,
    anomalies: &[Anomaly],
    event_log: &worldwake_core::EventLog,
    action_trace: &ActionTraceSink,
    perception_trace: &PerceptionTraceSink,
    recipe_registry: &RecipeRegistry,
    world: &worldwake_core::World,
    driver: &AgentTickDriver,
    budget_exhaustion_snapshots: &[BudgetExhaustionSnapshot],
    critical_window_section_enabled: bool,
    critical_window_reports: &[CriticalWindowReport],
    total_critical_window_count: usize,
) -> String {
    let mut out = String::new();

    // Section 1: Run Metadata
    writeln!(out, "# Simulation Observer Dump\n").unwrap();
    writeln!(out, "## Section 1 — Run Metadata\n").unwrap();
    writeln!(out, "- **Scenario**: `{scenario_path}`").unwrap();
    writeln!(out, "- **Seed**: {seed}").unwrap();
    writeln!(out, "- **Ticks simulated**: {tick_count}").unwrap();
    writeln!(out, "- **Total events**: {}", event_log.len()).unwrap();
    writeln!(out).unwrap();

    writeln!(out, "### Agents\n").unwrap();
    writeln!(out, "| Name | EntityId |").unwrap();
    writeln!(out, "|------|----------|").unwrap();
    for (id, name) in agents {
        writeln!(out, "| {name} | {id} |").unwrap();
    }
    writeln!(out).unwrap();

    writeln!(out, "### Places\n").unwrap();
    writeln!(out, "| Name | EntityId |").unwrap();
    writeln!(out, "|------|----------|").unwrap();
    for (id, name) in places {
        writeln!(out, "| {name} | {id} |").unwrap();
    }
    writeln!(out).unwrap();

    // Section 2: Per-Agent Summary
    writeln!(out, "## Section 2 — Per-Agent Summary\n").unwrap();
    for (agent_id, stats) in agent_stats {
        writeln!(out, "### {}\n", stats.name).unwrap();
        if let Some(line) = death_summary_line(world, *agent_id) {
            writeln!(out, "{line}").unwrap();
            writeln!(out).unwrap();
        }

        // Action counts
        writeln!(
            out,
            "**Actions** (total lifecycle events: {})\n",
            stats.total_actions()
        )
        .unwrap();
        let all_action_names: BTreeSet<&String> = stats
            .actions_started
            .keys()
            .chain(stats.actions_committed.keys())
            .chain(stats.actions_aborted.keys())
            .chain(stats.actions_start_failed.keys())
            .collect();
        if !all_action_names.is_empty() {
            writeln!(
                out,
                "| Action | Started | Committed | Aborted | StartFailed |"
            )
            .unwrap();
            writeln!(
                out,
                "|--------|---------|-----------|---------|-------------|"
            )
            .unwrap();
            for name in &all_action_names {
                writeln!(
                    out,
                    "| {} | {} | {} | {} | {} |",
                    name,
                    stats.actions_started.get(*name).unwrap_or(&0),
                    stats.actions_committed.get(*name).unwrap_or(&0),
                    stats.actions_aborted.get(*name).unwrap_or(&0),
                    stats.actions_start_failed.get(*name).unwrap_or(&0),
                )
                .unwrap();
            }
            writeln!(out).unwrap();
        }

        // Perception
        writeln!(
            out,
            "**Perception**: {} total observations, {} passed, {} unique entities observed\n",
            stats.observations_total,
            stats.observations_passed,
            stats.observation_entity_counts.len(),
        )
        .unwrap();

        // Needs trajectory
        if !stats.needs_samples.is_empty() {
            let len = stats.needs_samples.len() as u32;
            let (mut h_min, mut h_max, mut h_sum) = (u16::MAX, 0u16, 0u32);
            let (mut t_min, mut t_max, mut t_sum) = (u16::MAX, 0u16, 0u32);
            let (mut f_min, mut f_max, mut f_sum) = (u16::MAX, 0u16, 0u32);
            let (mut b_min, mut b_max, mut b_sum) = (u16::MAX, 0u16, 0u32);
            let (mut d_min, mut d_max, mut d_sum) = (u16::MAX, 0u16, 0u32);
            for s in &stats.needs_samples {
                h_min = h_min.min(s.hunger);
                h_max = h_max.max(s.hunger);
                h_sum += u32::from(s.hunger);
                t_min = t_min.min(s.thirst);
                t_max = t_max.max(s.thirst);
                t_sum += u32::from(s.thirst);
                f_min = f_min.min(s.fatigue);
                f_max = f_max.max(s.fatigue);
                f_sum += u32::from(s.fatigue);
                b_min = b_min.min(s.bladder);
                b_max = b_max.max(s.bladder);
                b_sum += u32::from(s.bladder);
                d_min = d_min.min(s.dirtiness);
                d_max = d_max.max(s.dirtiness);
                d_sum += u32::from(s.dirtiness);
            }
            writeln!(out, "**Needs trajectory** (‰)\n").unwrap();
            writeln!(out, "| Need | Min | Max | Avg |").unwrap();
            writeln!(out, "|------|-----|-----|-----|").unwrap();
            writeln!(out, "| Hunger | {} | {} | {} |", h_min, h_max, h_sum / len).unwrap();
            writeln!(out, "| Thirst | {} | {} | {} |", t_min, t_max, t_sum / len).unwrap();
            writeln!(out, "| Fatigue | {} | {} | {} |", f_min, f_max, f_sum / len).unwrap();
            writeln!(out, "| Bladder | {} | {} | {} |", b_min, b_max, b_sum / len).unwrap();
            writeln!(
                out,
                "| Dirtiness | {} | {} | {} |",
                d_min,
                d_max,
                d_sum / len
            )
            .unwrap();
            writeln!(out).unwrap();

            // Ticks above 750 threshold for each need
            let count_above = |samples: &[NeedsSample], f: fn(&NeedsSample) -> u16| -> u32 {
                samples.iter().filter(|s| f(s) > 750).count() as u32
            };
            let h_above = count_above(&stats.needs_samples, |s| s.hunger);
            let t_above = count_above(&stats.needs_samples, |s| s.thirst);
            let f_above = count_above(&stats.needs_samples, |s| s.fatigue);
            let b_above = count_above(&stats.needs_samples, |s| s.bladder);
            let d_above = count_above(&stats.needs_samples, |s| s.dirtiness);
            writeln!(
                out,
                "**Ticks above 750‰**: hunger={h_above}, thirst={t_above}, fatigue={f_above}, bladder={b_above}, dirtiness={d_above}\n"
            )
            .unwrap();
        }

        let transitions = behavioral_transitions(
            &action_timeline_bins_for_agent(action_trace, *agent_id),
            &stats.needs_samples,
        );
        for transition in &transitions {
            writeln!(out, "{}", format_behavioral_transition(transition)).unwrap();
            writeln!(out).unwrap();
        }

        // Location time
        if !stats.location_ticks.is_empty() {
            writeln!(out, "**Locations visited**\n").unwrap();
            writeln!(out, "| Place | Ticks |").unwrap();
            writeln!(out, "|-------|-------|").unwrap();
            for (place_id, ticks) in &stats.location_ticks {
                // We don't have world access here, so use EntityId
                writeln!(out, "| {place_id} | {ticks} |").unwrap();
            }
            writeln!(out).unwrap();
        }

        if let Some(table) = render_maintenance_rates_table(&stats.needs_samples) {
            write!(out, "{table}").unwrap();
        }

        if let Some(table) = render_recipe_usage_table(
            stats,
            world.get_component_known_recipes(*agent_id),
            recipe_registry,
        ) {
            write!(out, "{table}").unwrap();
        }

        // Idle tracking
        writeln!(
            out,
            "**Max consecutive idle ticks**: {}\n",
            stats.max_consecutive_idle
        )
        .unwrap();
    }

    render_decision_history_section(&mut out, event_log, agents);

    // Section 4: Anomaly Flags
    writeln!(out, "## Section 4 — Anomaly Flags\n").unwrap();
    if anomalies.is_empty() {
        writeln!(out, "No anomalies detected.\n").unwrap();
    } else {
        writeln!(out, "{} anomalies detected:\n", anomalies.len()).unwrap();
        for (i, anomaly) in anomalies.iter().enumerate() {
            writeln!(out, "{}\n", format_anomaly_header(i + 1, anomaly)).unwrap();
            writeln!(out, "{}\n", anomaly.description).unwrap();
            if let Some((start, end)) = anomaly.tick_range {
                writeln!(out, "Tick range: {start}–{end}\n").unwrap();
            }
        }
    }

    // Section 5: Raw Event Sample
    writeln!(out, "## Section 5 — Raw Event Sample\n").unwrap();
    let total_events = event_log.len();

    // First 100 events
    let first_n = total_events.min(100);
    writeln!(out, "### First {first_n} events\n").unwrap();
    writeln!(out, "```").unwrap();
    for i in 0..first_n {
        if let Some(record) = event_log.get(EventId(i as u64)) {
            writeln!(
                out,
                "[{}] tick={} actor={:?} action={:?} place={:?} tags={:?} deltas={}",
                i,
                record.tick().0,
                record.actor_id(),
                record.action_name(),
                record.place_id(),
                record.tags(),
                record.state_deltas().len(),
            )
            .unwrap();
        }
    }
    writeln!(out, "```\n").unwrap();

    // Last 100 events
    if total_events > 100 {
        let start = total_events - 100;
        writeln!(out, "### Last 100 events\n").unwrap();
        writeln!(out, "```").unwrap();
        for i in start..total_events {
            if let Some(record) = event_log.get(EventId(i as u64)) {
                writeln!(
                    out,
                    "[{}] tick={} actor={:?} action={:?} place={:?} tags={:?} deltas={}",
                    i,
                    record.tick().0,
                    record.actor_id(),
                    record.action_name(),
                    record.place_id(),
                    record.tags(),
                    record.state_deltas().len(),
                )
                .unwrap();
            }
        }
        writeln!(out, "```\n").unwrap();
    }

    // Action trace summary
    writeln!(out, "### Action Trace Summary\n").unwrap();
    writeln!(
        out,
        "Total action trace events: {}\n",
        action_trace.events().len()
    )
    .unwrap();

    // Per-agent action timeline (100-tick bins), counting only Started + StartFailed
    writeln!(out, "#### Per-Agent Action Timeline (100-tick bins)\n").unwrap();
    for (agent_id, agent_name) in agents {
        let agent_events = action_trace.events_for(*agent_id);
        // Filter to Started and StartFailed (agent decisions)
        let decision_events: Vec<_> = agent_events
            .iter()
            .filter(|e| {
                matches!(
                    e.kind,
                    ActionTraceKind::Started { .. } | ActionTraceKind::StartFailed { .. }
                )
            })
            .collect();
        if decision_events.is_empty() {
            continue;
        }

        writeln!(out, "**{agent_name} ({agent_id})**\n").unwrap();
        writeln!(out, "| Ticks | Actions |").unwrap();
        writeln!(out, "|-------|---------|").unwrap();

        let bins = action_timeline_bins_for_agent(action_trace, *agent_id);

        for (bin, action_counts) in &bins {
            let lo = bin * 100;
            let hi = lo + 99;
            let mut pairs: Vec<_> = action_counts.iter().collect();
            pairs.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
            let cells: Vec<String> = pairs
                .iter()
                .map(|(name, count)| format!("{name}\u{00d7}{count}"))
                .collect();
            writeln!(out, "| {lo}\u{2013}{hi} | {} |", cells.join(", ")).unwrap();
        }

        writeln!(out).unwrap();
    }

    // Raw tail of action trace events
    writeln!(out, "#### Raw Action Trace (last 50 events)\n").unwrap();
    writeln!(out, "```").unwrap();
    let at_events = action_trace.events();
    let at_start = at_events.len().saturating_sub(50);
    for event in &at_events[at_start..] {
        writeln!(out, "{}", event.summary()).unwrap();
    }
    writeln!(out, "```\n").unwrap();

    // Perception trace — per-agent timeline (100-tick bins)
    writeln!(
        out,
        "### Perception Trace Summary\n\nTotal perception trace events: {}\n",
        perception_trace.events().len()
    )
    .unwrap();

    for (agent_id, agent_name) in agents {
        let agent_events = perception_trace.events_for(*agent_id);
        if agent_events.is_empty() {
            continue;
        }
        writeln!(
            out,
            "**{agent_name} ({agent_id})** \u{2014} {} observations\n",
            agent_events.len()
        )
        .unwrap();
        writeln!(out, "| Ticks | Passed | Failed | Entities Observed |").unwrap();
        writeln!(out, "|-------|--------|--------|-------------------|").unwrap();

        let mut bins: BTreeMap<u64, (u32, u32, BTreeSet<EntityId>)> = BTreeMap::new();
        for event in &agent_events {
            let bin = event.tick.0 / 100;
            let entry = bins.entry(bin).or_insert((0, 0, BTreeSet::new()));
            if event.observation_passed {
                entry.0 += 1;
            } else {
                entry.1 += 1;
            }
            for entity in &event.entity_observations {
                entry.2.insert(*entity);
            }
        }

        for (bin, (passed, failed, entities)) in &bins {
            let lo = bin * 100;
            let hi = lo + 99;
            writeln!(
                out,
                "| {lo}\u{2013}{hi} | {passed} | {failed} | {} |",
                entities.len()
            )
            .unwrap();
        }
        writeln!(out).unwrap();
    }

    // Raw perception tail (last 50 events for detailed inspection)
    writeln!(out, "#### Raw Perception Trace (last 50 events)\n").unwrap();
    writeln!(out, "```").unwrap();
    let pt_events = perception_trace.events();
    let pt_start = pt_events.len().saturating_sub(50);
    for event in &pt_events[pt_start..] {
        writeln!(out, "{}", event.summary()).unwrap();
    }
    writeln!(out, "```\n").unwrap();

    // Section 6: Per-Agent Belief Summary
    writeln!(out, "## Section 6 — Per-Agent Belief Summary\n").unwrap();
    for (agent_id, agent_name) in agents {
        writeln!(out, "### {agent_name}\n").unwrap();

        let Some(store) = world.get_component_agent_belief_store(*agent_id) else {
            writeln!(out, "No belief store.\n").unwrap();
            continue;
        };

        // Count known entities by kind
        let total_known = store.known_entities.len();
        let mut agents_count: u32 = 0;
        let mut places_count: u32 = 0;
        let mut items_count: u32 = 0;
        let mut other_count: u32 = 0;
        for known_id in store.known_entities.keys() {
            match world.entity_kind(*known_id) {
                Some(EntityKind::Agent) => agents_count += 1,
                Some(EntityKind::Place) => places_count += 1,
                Some(EntityKind::ItemLot | EntityKind::UniqueItem) => items_count += 1,
                _ => other_count += 1,
            }
        }
        writeln!(out, "**Known entities**: {total_known}").unwrap();
        writeln!(out, "- Agents: {agents_count}").unwrap();
        writeln!(out, "- Places: {places_count}").unwrap();
        writeln!(out, "- Items: {items_count}").unwrap();
        if other_count > 0 {
            writeln!(out, "- Other: {other_count}").unwrap();
        }
        writeln!(out).unwrap();

        // Group known entities by believed location
        let mut by_place: BTreeMap<Option<EntityId>, Vec<EntityId>> = BTreeMap::new();
        for (known_id, state) in &store.known_entities {
            by_place
                .entry(state.last_known_place)
                .or_default()
                .push(*known_id);
        }
        if !by_place.is_empty() {
            writeln!(out, "**Believed entity locations**:").unwrap();
            for (place_opt, entities) in &by_place {
                match place_opt {
                    Some(pid) => {
                        let place_label = entity_display_name(world, *pid);
                        let parts = believed_location_parts(world, entities);
                        writeln!(out, "- {place_label}: {}", parts.join(", ")).unwrap();
                    }
                    None => {
                        for (place_label, grouped_entities) in
                            unknown_location_entity_groups(entities, store)
                        {
                            let parts = believed_location_parts(world, &grouped_entities);
                            writeln!(out, "- {place_label}: {}", parts.join(", ")).unwrap();
                        }
                    }
                }
            }
            writeln!(out).unwrap();
        }

        // Social observations
        writeln!(
            out,
            "**Social observations**: {}",
            store.social_observations.len()
        )
        .unwrap();

        // Told beliefs with unique counterparties
        let told_count = store.told_beliefs.len();
        let told_counterparties: BTreeSet<EntityId> =
            store.told_beliefs.keys().map(|k| k.counterparty).collect();
        if told_counterparties.is_empty() {
            writeln!(out, "**Told beliefs**: {told_count}").unwrap();
        } else {
            let cp_names: Vec<String> = told_counterparties
                .iter()
                .map(|id| entity_display_name(world, *id))
                .collect();
            writeln!(
                out,
                "**Told beliefs**: {told_count} (counterparties: {})",
                cp_names.join(", ")
            )
            .unwrap();
        }

        // Heard beliefs
        writeln!(out, "**Heard beliefs**: {}", store.heard_beliefs.len()).unwrap();

        // Institutional beliefs
        writeln!(
            out,
            "**Institutional beliefs**: {}",
            store.institutional_beliefs.len()
        )
        .unwrap();

        writeln!(out).unwrap();
    }

    // Section 7: End-State Inventory & Resources
    writeln!(out, "## Section 7 — End-State Inventory & Resources\n").unwrap();

    // Agent Inventories
    writeln!(out, "### Agent Inventories\n").unwrap();
    for (agent_id, agent_name) in agents {
        let possessions = world.possessions_of(*agent_id);
        if possessions.is_empty() {
            writeln!(out, "**{agent_name}**: (empty)\n").unwrap();
        } else {
            // Group item lots by commodity, collect non-lot items separately
            let mut commodity_totals: BTreeMap<String, u64> = BTreeMap::new();
            let mut non_lot_items: Vec<String> = Vec::new();
            for entity in &possessions {
                if let Some(lot) = world.get_component_item_lot(*entity) {
                    *commodity_totals
                        .entry(format!("{:?}", lot.commodity))
                        .or_insert(0) += u64::from(lot.quantity.0);
                } else {
                    non_lot_items.push(entity_display_name(world, *entity));
                }
            }
            let mut parts: Vec<String> = Vec::new();
            for (commodity, total) in &commodity_totals {
                parts.push(format!("{total}\u{00d7} {commodity}"));
            }
            parts.extend(non_lot_items);
            writeln!(out, "**{agent_name}**: {}\n", parts.join(", ")).unwrap();
        }
    }

    // Place Contents
    writeln!(out, "### Place Contents\n").unwrap();
    for (place_id, place_name) in places {
        let ground = world.ground_entities_at(*place_id);
        if ground.is_empty() {
            writeln!(out, "**{place_name} ({place_id})**: (empty)\n").unwrap();
        } else {
            // Aggregate item lots by commodity, list non-items individually
            let mut commodity_totals: BTreeMap<String, u64> = BTreeMap::new();
            let mut non_item_entries: Vec<String> = Vec::new();
            for entity in &ground {
                if let Some(lot) = world.get_component_item_lot(*entity) {
                    *commodity_totals
                        .entry(format!("{:?}", lot.commodity))
                        .or_insert(0) += u64::from(lot.quantity.0);
                } else {
                    let name = entity_display_name(world, *entity);
                    let annotation = match world.entity_kind(*entity) {
                        Some(EntityKind::Agent) => " (agent)".to_string(),
                        Some(EntityKind::Facility) => {
                            if let Some(ws) = world.get_component_workstation_marker(*entity) {
                                format!(" ({:?})", ws.0)
                            } else if let Some(rs) = world.get_component_resource_source(*entity) {
                                format!(" (resource: {:?})", rs.commodity)
                            } else {
                                " (facility)".to_string()
                            }
                        }
                        _ => String::new(),
                    };
                    non_item_entries.push(format!("{name}{annotation}"));
                }
            }
            let mut parts: Vec<String> = Vec::new();
            parts.extend(non_item_entries);
            for (commodity, total) in &commodity_totals {
                parts.push(format!("{total}\u{00d7} {commodity}"));
            }
            writeln!(out, "**{place_name} ({place_id})**: {}\n", parts.join(", ")).unwrap();
        }
    }

    // Section 8: Per-Agent Decision Summary
    writeln!(out, "## Section 8 — Per-Agent Decision Summary\n").unwrap();
    if let Some(sink) = driver.trace_sink() {
        for (agent_id, agent_name) in agents {
            let traces = sink.traces_for(*agent_id);
            let runtime = driver.runtime(*agent_id);
            writeln!(out, "### {agent_name} ({} decision ticks)\n", traces.len()).unwrap();

            if let Some(runtime) = runtime {
                write_agenda_state_summary(&mut out, world, &runtime.agenda_state);
            }

            if traces.is_empty() {
                writeln!(out, "No decision traces recorded.\n").unwrap();
                continue;
            }

            // Aggregate statistics
            let mut plans_found: u32 = 0;
            let mut plans_budget_exhausted: u32 = 0;
            let mut plans_frontier_exhausted: u32 = 0;
            let mut plans_unsupported: u32 = 0;
            let mut dead_ticks: u32 = 0;
            let mut active_ticks: u32 = 0;
            let mut planning_ticks: u32 = 0;
            let mut unique_goals_selected: BTreeSet<String> = BTreeSet::new();

            for trace in &traces {
                match &trace.outcome {
                    DecisionOutcome::Dead => dead_ticks += 1,
                    DecisionOutcome::ActiveAction { .. } => active_ticks += 1,
                    DecisionOutcome::Planning(planning) => {
                        planning_ticks += 1;
                        if let Some(goal) = planning.selection.selected_goal() {
                            unique_goals_selected.insert(format!("{:?}", goal.kind));
                        }
                        for attempt in &planning.planning.attempts {
                            match &attempt.outcome {
                                PlanSearchOutcome::Found { .. } => plans_found += 1,
                                PlanSearchOutcome::BudgetExhausted { .. } => {
                                    plans_budget_exhausted += 1;
                                }
                                PlanSearchOutcome::FrontierExhausted { .. } => {
                                    plans_frontier_exhausted += 1;
                                }
                                PlanSearchOutcome::Unsupported => plans_unsupported += 1,
                            }
                        }
                    }
                }
            }

            writeln!(
                out,
                "**Tick breakdown**: {planning_ticks} planning, {active_ticks} active-action, {dead_ticks} dead"
            )
            .unwrap();
            writeln!(
                out,
                "**Plan search outcomes**: {plans_found} found, {plans_frontier_exhausted} frontier-exhausted, {plans_budget_exhausted} budget-exhausted, {plans_unsupported} unsupported"
            )
            .unwrap();
            if !unique_goals_selected.is_empty() {
                writeln!(
                    out,
                    "**Goals selected**: {}",
                    unique_goals_selected
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                )
                .unwrap();
            }
            writeln!(out).unwrap();

            // Per-tick decision timeline in 100-tick bins
            writeln!(out, "**Decision timeline** (100-tick bins)\n").unwrap();
            writeln!(out, "| Ticks | Decisions |").unwrap();
            writeln!(out, "|-------|-----------|").unwrap();

            let mut bins: BTreeMap<u64, Vec<String>> = BTreeMap::new();
            for trace in &traces {
                let bin = trace.tick.0 / 100;
                bins.entry(bin).or_default().push(trace.outcome.summary());
            }
            for (bin, summaries) in &bins {
                let lo = bin * 100;
                let hi = lo + 99;
                // Deduplicate repeated summaries within a bin
                let mut counts: BTreeMap<&str, u32> = BTreeMap::new();
                for s in summaries {
                    *counts.entry(s.as_str()).or_insert(0) += 1;
                }
                let mut cells: Vec<String> = Vec::new();
                for (summary, count) in &counts {
                    if *count > 1 {
                        cells.push(format!("{summary} (\u{00d7}{count})"));
                    } else {
                        cells.push(summary.to_string());
                    }
                }
                // Truncate if too many unique entries in one bin
                if cells.len() > 5 {
                    let total = cells.len();
                    cells.truncate(5);
                    cells.push(format!("... and {} more", total - 5));
                }
                writeln!(out, "| {lo}\u{2013}{hi} | {} |", cells.join("; ")).unwrap();
            }
            writeln!(out).unwrap();

            // Failed plan attempts detail
            let failed_attempts = collect_failed_plan_attempts(&traces);

            if !failed_attempts.is_empty() {
                let total_failures = failed_attempts.len();
                writeln!(
                    out,
                    "**Failed plan attempts** (showing first 20 of {total_failures})\n"
                )
                .unwrap();
                writeln!(
                    out,
                    "| Tick | Goal | Outcome | Expansions | Max Depth | Candidates | Location | Had Target Beliefs |"
                )
                .unwrap();
                writeln!(
                    out,
                    "|------|------|---------|------------|-----------|------------|----------|--------------------|"
                )
                .unwrap();
                let mut shown = 0u32;
                let mut shown_attempts: Vec<&PlanAttemptTrace> = Vec::new();
                for (tick, place, attempt) in &failed_attempts {
                    if shown >= 20 {
                        break;
                    }
                    let (PlanSearchOutcome::FrontierExhausted {
                        expansions_used: expansions,
                    }
                    | PlanSearchOutcome::BudgetExhausted {
                        expansions_used: expansions,
                    }) = &attempt.outcome
                    else {
                        continue;
                    };
                    writeln!(
                        out,
                        "| {} | {:?} | {} | {} | {} | {} | {} | {} |",
                        tick,
                        attempt.goal.kind,
                        failed_plan_outcome_label(attempt),
                        expansions,
                        failed_plan_max_depth(attempt),
                        failed_plan_candidates(attempt),
                        failed_plan_location(*place),
                        failed_plan_target_beliefs(attempt)
                    )
                    .unwrap();
                    shown += 1;
                    shown_attempts.push(*attempt);
                }
                writeln!(out).unwrap();

                let breakdown = failed_plan_breakdown(&shown_attempts);
                writeln!(out, "### Failed Plan Frequency Breakdown").unwrap();
                writeln!(
                    out,
                    "- frontier-exhausted: {} / {}",
                    breakdown.frontier_exhausted, breakdown.total
                )
                .unwrap();
                writeln!(
                    out,
                    "- budget-exhausted: {} / {}",
                    breakdown.budget_exhausted, breakdown.total
                )
                .unwrap();
                writeln!(
                    out,
                    "- Max Depth = 0 (no operators available): {} / {}",
                    breakdown.max_depth_zero, breakdown.total
                )
                .unwrap();
                writeln!(
                    out,
                    "- Had Target Beliefs = false: {} / {}",
                    breakdown.target_beliefs_false, breakdown.total
                )
                .unwrap();
                writeln!(out).unwrap();
            }

            // Blocked desires summary
            let blocked_desires: BTreeMap<String, u32> = traces
                .iter()
                .filter_map(|t| {
                    if let DecisionOutcome::Planning(planning) = &t.outcome {
                        Some(&planning.candidates.fully_blocked_desires)
                    } else {
                        None
                    }
                })
                .flatten()
                .fold(BTreeMap::new(), |mut acc, blocked| {
                    *acc.entry(format!("{:?}", blocked.goal_key.kind))
                        .or_insert(0) += 1;
                    acc
                });

            if !blocked_desires.is_empty() {
                writeln!(
                    out,
                    "**Fully blocked desires** (goal generated but all opportunities blocked)\n"
                )
                .unwrap();
                writeln!(out, "| Goal | Times Blocked |").unwrap();
                writeln!(out, "|------|---------------|").unwrap();
                for (goal, count) in &blocked_desires {
                    writeln!(out, "| {goal} | {count} |").unwrap();
                }
                writeln!(out).unwrap();
            }

            let affordance_snapshots = planning_affordance_snapshots(&traces);
            let travel_commit_ticks = committed_travel_ticks(&action_trace.events_for(*agent_id));

            // Affordances available (from first planning tick that has them)
            if let Some((tick, affordances)) = affordance_snapshots.first().copied() {
                write_affordance_list(
                    &mut out,
                    &format!(
                        "**Affordances available at tick {}** (at {})",
                        tick.0,
                        affordances
                            .place
                            .map_or_else(|| "unknown".to_string(), |p| p.to_string())
                    ),
                    affordances,
                )
                .unwrap();
            }

            for (tick, affordances) in
                post_travel_affordance_snapshots(&affordance_snapshots, &travel_commit_ticks)
            {
                let place_label = affordances.place.map_or_else(
                    || "unknown".to_string(),
                    |place| entity_display_name(world, place),
                );
                write_affordance_list(
                    &mut out,
                    &format!(
                        "**Affordances after travel** (tick {}, arrived at {})",
                        tick.0, place_label
                    ),
                    affordances,
                )
                .unwrap();
            }

            for event in affordance_change_snapshots(&affordance_snapshots) {
                let mut parts = Vec::new();
                for name in &event.appeared {
                    parts.push(format!("+{name}"));
                }
                for name in &event.disappeared {
                    parts.push(format!("-{name}"));
                }
                let hint = if event.place_changed {
                    event.affordances.place.map_or_else(String::new, |place| {
                        format!(" (at {})", entity_display_name(world, place))
                    })
                } else {
                    String::new()
                };
                writeln!(
                    out,
                    "**Affordance changes** (tick {}): {}{hint}",
                    event.tick.0,
                    parts.join(", ")
                )
                .unwrap();
            }

            if let Some((tick, affordances)) = final_affordance_snapshot(&affordance_snapshots) {
                write_affordance_list(
                    &mut out,
                    &format!("**Final affordances** (tick {})", tick.0),
                    affordances,
                )
                .unwrap();
            }
        }
    } else {
        writeln!(out, "Decision tracing was not enabled.\n").unwrap();
    }

    // Section 9: Budget Exhaustion Snapshots
    format_budget_exhaustion_snapshots(&mut out, budget_exhaustion_snapshots);

    // Section 10: Critical Window Forensics
    if critical_window_section_enabled {
        format_critical_window_forensics(
            &mut out,
            agents,
            world,
            critical_window_reports,
            total_critical_window_count,
        );
    }

    out
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let cli = ObserverCli::parse();

    let def = match load_scenario_file(&cli.scenario) {
        Ok(def) => def,
        Err(e) => {
            eprintln!("Failed to load scenario: {e}");
            std::process::exit(1);
        }
    };

    let seed = def.seed;

    if cli.ignore_lints {
        let report = worldwake_cli::scenario::lints::run_lints(&def);
        let report = match worldwake_cli::scenario::lints::filter_overrides(
            report,
            &def.scenario_lint_overrides,
        ) {
            Ok(report) => report,
            Err(e) => {
                eprintln!("Failed to spawn scenario: {e}");
                std::process::exit(1);
            }
        };
        for failure in &report.failures {
            eprintln!(
                "WARNING (lint suppressed by --ignore-lints): {:?} [{}] {}",
                failure.rule,
                failure.affected_agents.join(", "),
                failure.detail
            );
        }
    }

    let spawned = match if cli.ignore_lints {
        spawn_scenario_ignoring_lints(&def)
    } else {
        spawn_scenario(&def)
    } {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to spawn scenario: {e}");
            std::process::exit(1);
        }
    };

    let mut sim = spawned.state;
    let mut driver = AgentTickDriver::new();
    driver.enable_tracing();

    // Collect agent and place info
    let world = sim.world();
    let agents: Vec<(EntityId, String)> = world
        .entities_with_name_and_agent_data()
        .map(|id| (id, entity_display_name(world, id)))
        .collect();
    let places: Vec<(EntityId, String)> = world
        .topology()
        .place_ids()
        .map(|id| {
            let name = world
                .topology()
                .place(id)
                .map_or_else(|| format!("{id}"), |p| p.name.clone());
            (id, name)
        })
        .collect();

    // Initialize per-agent stats
    let mut agent_stats: BTreeMap<EntityId, AgentStats> = agents
        .iter()
        .map(|(id, name)| {
            let has_patrol = world.get_component_patrol_route(*id).is_some();
            (*id, AgentStats::new(name.clone(), has_patrol))
        })
        .collect();

    // Budget exhaustion snapshot collection
    let mut budget_exhaustion_snapshots: Vec<BudgetExhaustionSnapshot> = Vec::new();
    let mut budget_exhaustion_seen: BTreeSet<BudgetExhaustionKey> = BTreeSet::new();
    let mut survival_forensics: BTreeMap<EntityId, SurvivalForensicExtractor> =
        if cli.critical_window_top_n == 0 {
            BTreeMap::new()
        } else {
            agents
                .iter()
                .map(|(agent_id, _)| (*agent_id, SurvivalForensicExtractor::new(*agent_id)))
                .collect()
        };

    // Create all trace sinks (persistent across all ticks)
    let mut action_trace = ActionTraceSink::new();
    let mut perception_trace = PerceptionTraceSink::new();
    let mut request_resolution_trace = RequestResolutionTraceSink::new();
    let mut politics_trace = PoliticalTraceSink::new();
    let mut institutional_knowledge_trace = InstitutionalKnowledgeTraceSink::new();
    let mut open_frame: BTreeMap<EntityId, bool> = BTreeMap::new();

    eprintln!(
        "Running {} ticks on scenario '{}' (seed {})...",
        cli.ticks,
        cli.scenario.display(),
        seed
    );

    for tick_num in 0..cli.ticks {
        let mut controllers = AutonomousControllerRuntime::new(vec![&mut driver]);
        let (world, event_log, scheduler, controller, rng, recipe_registry) = sim.tick_parts_mut();

        let current_tick = scheduler.current_tick();

        let _result = match step_tick(
            world,
            event_log,
            scheduler,
            controller,
            rng,
            TickStepServices {
                action_defs: &spawned.action_registries.defs,
                action_handlers: &spawned.action_registries.handlers,
                recipe_registry,
                systems: &spawned.dispatch_table,
                input_producer: Some(&mut controllers),
                action_trace: Some(&mut action_trace),
                request_resolution_trace: Some(&mut request_resolution_trace),
                politics_trace: Some(&mut politics_trace),
                perception_trace: Some(&mut perception_trace),
                institutional_knowledge_trace: Some(&mut institutional_knowledge_trace),
            },
        ) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Tick error at tick {}: {e:?}", current_tick.0);
                eprintln!("Writing partial report up to tick {}...", current_tick.0);
                break;
            }
        };

        // Gather per-tick stats
        // Action trace events for this tick
        for event in action_trace.events_at(current_tick) {
            match &event.kind {
                ActionTraceKind::Started { .. } => {
                    open_frame.insert(event.actor, true);
                }
                ActionTraceKind::Committed { .. } | ActionTraceKind::Aborted { .. } => {
                    open_frame.insert(event.actor, false);
                }
                ActionTraceKind::StartFailed { .. } => {}
            }

            if let Some(stats) = agent_stats.get_mut(&event.actor) {
                match &event.kind {
                    ActionTraceKind::Started { .. } => {
                        *stats
                            .actions_started
                            .entry(event.action_name.clone())
                            .or_insert(0) += 1;
                        stats.action_sequence.push(event.action_name.clone());
                    }
                    ActionTraceKind::Committed { .. } => {
                        *stats
                            .actions_committed
                            .entry(event.action_name.clone())
                            .or_insert(0) += 1;
                    }
                    ActionTraceKind::Aborted { .. } => {
                        *stats
                            .actions_aborted
                            .entry(event.action_name.clone())
                            .or_insert(0) += 1;
                    }
                    ActionTraceKind::StartFailed { .. } => {
                        *stats
                            .actions_start_failed
                            .entry(event.action_name.clone())
                            .or_insert(0) += 1;
                        stats
                            .action_sequence
                            .push(format!("FAIL:{}", event.action_name));
                    }
                }
            }
        }

        // Perception trace events for this tick
        for event in perception_trace.events_at(current_tick) {
            if let Some(stats) = agent_stats.get_mut(&event.observer) {
                stats.observations_total += 1;
                if event.observation_passed {
                    stats.observations_passed += 1;
                }
                for entity in &event.entity_observations {
                    *stats.observation_entity_counts.entry(*entity).or_insert(0) += 1;
                    stats
                        .observation_tick_sets
                        .entry(*entity)
                        .or_default()
                        .insert(current_tick.0);
                }
            }
        }

        // Needs, location sampling, and idle tracking (read from world after tick)
        for (agent_id, stats) in &mut agent_stats {
            // Needs
            let current_needs =
                if let Some(needs) = world.get_component_homeostatic_needs(*agent_id) {
                    let sample = NeedsSample {
                        hunger: needs.hunger.value(),
                        thirst: needs.thirst.value(),
                        fatigue: needs.fatigue.value(),
                        bladder: needs.bladder.value(),
                        dirtiness: needs.dirtiness.value(),
                    };
                    stats.needs_samples.push(sample);
                    sample
                } else {
                    NeedsSample {
                        hunger: 0,
                        thirst: 0,
                        fatigue: 0,
                        bladder: 0,
                        dirtiness: 0,
                    }
                };

            // Location
            let current_place = world.effective_place(*agent_id);
            stats.location_history.push(current_place);
            if let Some(place) = current_place {
                *stats.location_ticks.entry(place).or_insert(0) += 1;
            }

            // Idle tracking: did this agent have any action trace events this tick?
            let had_event = action_trace
                .events_for_at(*agent_id, current_tick)
                .iter()
                .any(|e| !matches!(e.kind, ActionTraceKind::StartFailed { .. }));
            let in_open_frame = open_frame.get(agent_id).copied().unwrap_or(false);
            let had_action = had_event || in_open_frame;
            stats.record_idle_tick(had_action, current_tick.0, current_needs);
        }

        if !survival_forensics.is_empty() {
            let decision_trace_sink = driver.trace_sink();
            for (agent_id, extractor) in &mut survival_forensics {
                let Some(needs) = world.get_component_homeostatic_needs(*agent_id) else {
                    continue;
                };
                let local_state = LocalSurvivalStateSummary::capture(world, *agent_id);
                let thresholds = world
                    .get_component_drive_thresholds(*agent_id)
                    .copied()
                    .unwrap_or_default();
                let decision_trace =
                    decision_trace_sink.and_then(|sink| sink.trace_at(*agent_id, current_tick));
                let active_action = scheduler
                    .active_actions()
                    .values()
                    .find(|instance| instance.actor == *agent_id);
                let active_action_name = active_action
                    .and_then(|instance| spawned.action_registries.defs.get(instance.def_id))
                    .map(|def| def.name.as_str());
                let action_snapshot = ActionTraceSnapshot::from_sink(
                    *agent_id,
                    current_tick,
                    &action_trace,
                    active_action,
                    active_action_name,
                );
                extractor.observe(
                    current_tick,
                    needs,
                    &thresholds,
                    decision_trace,
                    &action_snapshot,
                    &local_state,
                );
            }
        }

        // Budget exhaustion snapshot collection
        if let Some(sink) = driver.trace_sink() {
            for (agent_id, agent_name) in &agents {
                let Some(trace) = sink.trace_at(*agent_id, current_tick) else {
                    continue;
                };
                let DecisionOutcome::Planning(planning) = &trace.outcome else {
                    continue;
                };
                for attempt in &planning.planning.attempts {
                    if let PlanSearchOutcome::BudgetExhausted { expansions_used } = &attempt.outcome
                    {
                        let goal_debug = format!("{:?}", attempt.goal.kind);
                        let goal_kind = goal_kind_dedup_key(&goal_debug);
                        let location = world.effective_place(*agent_id).unwrap_or(EntityId {
                            slot: u32::MAX,
                            generation: u32::MAX,
                        });
                        let key = BudgetExhaustionKey {
                            agent_id: *agent_id,
                            goal_kind,
                            location,
                        };
                        if budget_exhaustion_seen.contains(&key) {
                            continue;
                        }
                        budget_exhaustion_seen.insert(key);

                        let needs = world.get_component_homeostatic_needs(*agent_id).map_or(
                            NeedsSample {
                                hunger: 0,
                                thirst: 0,
                                fatigue: 0,
                                bladder: 0,
                                dirtiness: 0,
                            },
                            |n| NeedsSample {
                                hunger: n.hunger.value(),
                                thirst: n.thirst.value(),
                                fatigue: n.fatigue.value(),
                                bladder: n.bladder.value(),
                                dirtiness: n.dirtiness.value(),
                            },
                        );

                        let location_name = entity_display_name(world, location);
                        let inventory = collect_inventory(world, *agent_id);
                        let (known_entity_count, beliefs) = collect_beliefs(world, *agent_id);
                        let place_contents = collect_place_contents(world, location);

                        // Collect adjacent place contents
                        let mut adjacent_contents: BTreeMap<String, Vec<String>> = BTreeMap::new();
                        for neighbor in world.topology().neighbors(location) {
                            let neighbor_name = entity_display_name(world, neighbor);
                            let contents = collect_place_contents(world, neighbor);
                            if !contents.is_empty() {
                                adjacent_contents.insert(neighbor_name, contents);
                            }
                        }

                        let cognitive = world.get_component_cognitive_profile(*agent_id);
                        let exec_budget = world.get_component_execution_budget(*agent_id);

                        let (total_candidates, max_depth_reached) =
                            compute_search_metrics(&attempt.expansion_summaries);

                        budget_exhaustion_snapshots.push(BudgetExhaustionSnapshot {
                            tick: current_tick.0,
                            agent_id: *agent_id,
                            agent_name: agent_name.clone(),
                            goal_debug: goal_debug.clone(),
                            needs,
                            location,
                            location_name,
                            inventory,
                            beliefs,
                            known_entity_count,
                            place_contents,
                            adjacent_contents,
                            max_node_expansions: cognitive.map_or(224, |c| c.max_node_expansions),
                            max_plan_depth: cognitive.map_or(10, |c| c.max_plan_depth),
                            max_candidates_per_expansion: cognitive
                                .map_or(0, |c| c.max_candidates_per_expansion),
                            max_prerequisite_locations: exec_budget.map_or(
                                2,
                                worldwake_core::ExecutionBudget::max_prerequisite_locations,
                            ),
                            beam_width: exec_budget
                                .map_or(5, worldwake_core::ExecutionBudget::beam_width),
                            preferred_operator_boost: exec_budget.map_or(
                                0,
                                worldwake_core::ExecutionBudget::preferred_operator_boost,
                            ),
                            expansions_used: *expansions_used,
                            max_depth_reached,
                            total_candidates,
                        });
                    }
                }
            }
        }

        // Progress indicator
        if tick_num > 0 && tick_num % 100 == 0 {
            eprintln!("  tick {tick_num}/{}", cli.ticks);
        }
    }

    // Flush any open idle windows at simulation end.
    for stats in agent_stats.values_mut() {
        stats.flush_idle_window(cli.ticks.saturating_sub(1));
    }

    eprintln!("Simulation complete. Detecting anomalies...");

    let anomalies = detect_anomalies(
        &agent_stats,
        &perception_trace,
        sim.event_log(),
        sim.world(),
        sim.recipe_registry(),
    );

    eprintln!("Found {} anomalies. Writing report...", anomalies.len());

    let all_critical_window_reports = survival_forensics
        .into_values()
        .flat_map(SurvivalForensicExtractor::finalize)
        .collect::<Vec<_>>();
    let critical_window_reports = SurvivalForensicExtractor::top_n_longest(
        &all_critical_window_reports,
        cli.critical_window_top_n,
    )
    .into_iter()
    .cloned()
    .collect::<Vec<_>>();

    let scenario_path_str = cli.scenario.display().to_string();
    let report = format_report(
        &scenario_path_str,
        seed,
        cli.ticks,
        &agents,
        &places,
        &agent_stats,
        &anomalies,
        sim.event_log(),
        &action_trace,
        &perception_trace,
        sim.recipe_registry(),
        sim.world(),
        &driver,
        &budget_exhaustion_snapshots,
        cli.critical_window_top_n > 0,
        &critical_window_reports,
        all_critical_window_reports.len(),
    );

    // Ensure parent directory exists
    if let Some(parent) = cli.output.parent() {
        std::fs::create_dir_all(parent).unwrap_or_else(|e| {
            eprintln!("Failed to create output directory: {e}");
            std::process::exit(1);
        });
    }

    std::fs::write(&cli.output, &report).unwrap_or_else(|e| {
        eprintln!("Failed to write report: {e}");
        std::process::exit(1);
    });

    eprintln!(
        "Report written to {} ({} bytes)",
        cli.output.display(),
        report.len()
    );
}

#[cfg(test)]
mod tests {
    use super::{
        ANOMALY_ROLLING_WINDOW_TICKS, AgentStats, Anomaly, AnomalyKind, BehavioralTransition,
        NeedsSample, PlanAttemptTrace, PlanSearchOutcome, affordance_change_snapshots,
        behavioral_transitions, committed_travel_ticks, compute_maintenance_rates,
        death_summary_line, decision_payload_summary, detect_acute_need_spike, detect_anomalies,
        detect_geographic_convergence, detect_maintenance_starvation, detect_recipe_monoculture,
        failed_plan_breakdown, failed_plan_candidates, failed_plan_location, failed_plan_max_depth,
        failed_plan_outcome_label, failed_plan_target_beliefs, final_affordance_snapshot,
        format_affordance_summary, format_anomaly_header, format_behavioral_transition,
        format_death_cause, format_report, need_high_threshold, post_travel_affordance_snapshots,
        primary_satisfied_need, recipe_usage_rows, render_decision_history_section,
        render_maintenance_rates_table, render_recipe_usage_table, unknown_location_entity_groups,
    };
    use std::collections::BTreeMap;
    use std::collections::BTreeSet;
    use std::num::NonZeroU32;
    use worldwake_ai::decision_trace::{
        AffordanceSummary, AffordanceTrace, AgentDecisionTrace, CandidateTrace, DecisionOutcome,
        ExecutionTrace, PatrolRouteSnapshotTrace, PlanSearchTrace, PlanningPipelineTrace,
        SearchExpansionSummary, SelectionTrace, TargetBeliefPresence,
    };
    use worldwake_ai::{
        ActiveActionSummary, AgendaEntry, AgendaEntrySnapshot, AgendaOrigin, AgendaPhase,
        AgentDecisionRuntime, AgentTickDriver, BlockerSummary, CriticalWindowFrame,
        CriticalWindowReport, DirtySet, ExhaustionSummary, GoalOffer, GoalPriorityClass,
        KillCondition, LocalSurvivalStateSummary, RevivalTrigger, SelectedPlanSource,
    };
    use worldwake_core::PerceptionSource;
    use worldwake_core::{
        AcquisitionQuantity, ActionDefId, AgentBeliefStore, BeliefClaimKey, BelievedEntityState,
        BlockerKey, BlockerRecordedPayload, BodyCostPerTick, CauseRef, CommodityKind,
        CommodityPurpose, ControlSource, DeadAt, DeathCause, DecisionEventPayload, DriveThresholds,
        EmitterTag, EntityBeliefAspect, EntityId, EntityKind, EventLog, EventPayload, EventTag,
        GoalAbandonReason, GoalAbandonedPayload, GoalCommittedPayload, GoalKey, GoalKind,
        GoalOfferedPayload, GoalRejectionReason, GoalSuppressedPayload, GoalSuspendedPayload,
        GoalSwitchReason, HomeostaticNeedId, KnownRecipes, MetabolismProfile, OpportunityAnchor,
        PendingEvent, Permille, PlanAdoptedPayload, PlanInvalidatedPayload, PlanInvalidationReason,
        PrototypePlace, Quantity, RecipeId, ResourceSource, Tick, VisibilitySpec, WitnessData,
        WorkstationTag, World, WorldTxn, build_prototype_world, prototype_place_entity,
    };
    use worldwake_sim::{
        ActionInstanceId, ActionTraceEvent, ActionTraceKind, ActionTraceSink, CommitOutcome,
        PerceptionTraceSink, RecipeDefinition, RecipeRegistry, RequestAttemptTrace,
        RequestBindingKind, RequestProvenance, ResolvedRequestTrace,
    };

    fn sample_summary(depth: u8, candidates_generated: u16) -> SearchExpansionSummary {
        SearchExpansionSummary {
            depth,
            remaining_travel_ticks: 0,
            combined_places_count: 0,
            prerequisite_places_count: 0,
            candidates_generated,
            candidates_skipped: 0,
            preferred_candidates: 0,
            terminal_successors: 0,
            non_terminal_before_beam: 0,
            non_terminal_after_beam: 0,
            found_goal_satisfied: false,
            landmark_heuristic: 0,
            ff_heuristic: None,
            helpful_action_count: 0,
            travel_pruning: None,
            prerequisite_guidance: None,
            expansion_candidates: vec![],
            root_candidates: Vec::new(),
            root_omissions: Vec::new(),
        }
    }

    fn sample_summary_with_counts(
        depth: u8,
        candidates_generated: u16,
        candidates_skipped: u16,
        terminal_successors: u16,
        non_terminal_after_beam: u16,
    ) -> SearchExpansionSummary {
        SearchExpansionSummary {
            depth,
            remaining_travel_ticks: 0,
            combined_places_count: 0,
            prerequisite_places_count: 0,
            candidates_generated,
            candidates_skipped,
            preferred_candidates: 0,
            terminal_successors,
            non_terminal_before_beam: 0,
            non_terminal_after_beam,
            found_goal_satisfied: false,
            landmark_heuristic: 0,
            ff_heuristic: None,
            helpful_action_count: 0,
            travel_pruning: None,
            prerequisite_guidance: None,
            expansion_candidates: vec![],
            root_candidates: Vec::new(),
            root_omissions: Vec::new(),
        }
    }

    fn sample_attempt(expansion_summaries: Vec<SearchExpansionSummary>) -> PlanAttemptTrace {
        PlanAttemptTrace {
            goal: GoalKey::from(GoalKind::Sleep),
            opportunity_anchor: OpportunityAnchor::None,
            outcome: PlanSearchOutcome::FrontierExhausted { expansions_used: 3 },
            target_belief_presence: TargetBeliefPresence::NotApplicable,
            strategic_plan: None,
            tactical_goal: None,
            landmarks_extracted: 0,
            landmark_orderings: 0,
            binding_rejections: Vec::new(),
            expansion_summaries,
        }
    }

    fn need_sample(value: u16) -> NeedsSample {
        NeedsSample {
            hunger: value,
            thirst: value + 1,
            fatigue: value + 2,
            bladder: value + 3,
            dirtiness: value + 4,
        }
    }

    fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(tick),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        )
    }

    fn commit_txn(txn: WorldTxn<'_>) {
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn emit_decision_event(
        log: &mut EventLog,
        tick: u64,
        actor: EntityId,
        tag: EventTag,
        payload: DecisionEventPayload,
    ) {
        let _ = log.emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(tick),
            cause: CauseRef::SystemTick(Tick(tick)),
            actor_id: Some(actor),
            action_name: None,
            target_ids: Vec::new(),
            evidence: Vec::new(),
            place_id: None,
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::Hidden,
            witness_data: WitnessData::default(),
            tags: BTreeSet::from([tag]),
            decision_payload: Some(payload),
        }));
    }

    fn sample_decision_event_log(agent: EntityId, target: EntityId) -> EventLog {
        let mut log = EventLog::new();
        let acquire_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let sleep_goal = GoalKey::from(GoalKind::Sleep);
        let patrol_goal = GoalKey::from(GoalKind::Patrol { place: entity(20) });
        let produce_goal = GoalKey::from(GoalKind::ProduceCommodity {
            recipe_id: RecipeId(3),
        });
        let move_goal = GoalKey::from(GoalKind::MoveCargo {
            commodity: CommodityKind::Water,
            destination: entity(21),
        });
        let support_goal = GoalKey::from(GoalKind::SupportCandidateForOffice {
            office: entity(22),
            candidate: entity(23),
        });

        emit_decision_event(
            &mut log,
            1,
            agent,
            EventTag::GoalOffered,
            DecisionEventPayload::GoalOffered(GoalOfferedPayload {
                agent,
                goal_key: acquire_goal,
                emitter: EmitterTag::Enterprise,
                source_evidence: worldwake_core::EvidenceSummary {
                    evidence_kind_counts: BTreeMap::from([
                        (worldwake_core::EvidenceKindTag::LearnedOpportunity, 1),
                        (worldwake_core::EvidenceKindTag::PerceptionObservation, 2),
                    ]),
                },
            }),
        );
        emit_decision_event(
            &mut log,
            2,
            agent,
            EventTag::GoalSuppressed,
            DecisionEventPayload::GoalSuppressed(GoalSuppressedPayload {
                agent,
                goal_key: sleep_goal,
                reason: GoalRejectionReason::SuppressedByStressPolicy,
            }),
        );
        emit_decision_event(
            &mut log,
            3,
            agent,
            EventTag::GoalCommitted,
            DecisionEventPayload::GoalCommitted(GoalCommittedPayload {
                agent,
                goal_key: produce_goal,
                motive_score: 420,
                rejected_alternatives: vec![worldwake_core::RejectedAlternativeSummary {
                    goal_key: acquire_goal,
                    rejection_reason: GoalRejectionReason::LowerMotive,
                    score_gap: 17,
                }],
            }),
        );
        emit_decision_event(
            &mut log,
            4,
            agent,
            EventTag::GoalSuspended,
            DecisionEventPayload::GoalSuspended(GoalSuspendedPayload {
                agent,
                goal_key: move_goal,
                reason: worldwake_core::SuspensionReason::RouteBlocked,
            }),
        );
        emit_decision_event(
            &mut log,
            5,
            agent,
            EventTag::GoalAbandoned,
            DecisionEventPayload::GoalAbandoned(GoalAbandonedPayload {
                agent,
                goal_key: patrol_goal,
                reason: GoalAbandonReason::GoalSwitched {
                    new_goal: acquire_goal,
                    switch_kind: GoalSwitchReason::HigherPriorityGoal,
                },
            }),
        );
        emit_decision_event(
            &mut log,
            6,
            agent,
            EventTag::PlanAdopted,
            DecisionEventPayload::PlanAdopted(PlanAdoptedPayload {
                agent,
                goal_key: acquire_goal,
                plan_step_count: 3,
            }),
        );
        emit_decision_event(
            &mut log,
            7,
            agent,
            EventTag::PlanInvalidated,
            DecisionEventPayload::PlanInvalidated(PlanInvalidatedPayload {
                agent,
                goal_key: move_goal,
                reason: PlanInvalidationReason::BeliefUpdate {
                    claim_key: BeliefClaimKey {
                        subject: target,
                        aspect: EntityBeliefAspect::Inventory(CommodityKind::Bread),
                    },
                },
                belief_snapshot: None,
            }),
        );
        emit_decision_event(
            &mut log,
            8,
            agent,
            EventTag::ExpectationMismatch,
            DecisionEventPayload::ExpectationMismatch(worldwake_core::ExpectationMismatchPayload {
                agent,
                goal_key: acquire_goal,
                step_index: 1,
                expected_materializations: vec![worldwake_core::MaterializationTag::SplitOffLot],
                expectation_kind: None,
                mismatch_detail: None,
            }),
        );
        emit_decision_event(
            &mut log,
            9,
            agent,
            EventTag::RepairApplied,
            DecisionEventPayload::RepairApplied(worldwake_core::RepairAppliedPayload {
                agent,
                goal_key: support_goal,
                step_index: 2,
                repair_kind: worldwake_core::RepairKind::AlternateMerchant,
                substitute_target: Some(target),
            }),
        );
        emit_decision_event(
            &mut log,
            10,
            agent,
            EventTag::ReplanTriggered,
            DecisionEventPayload::ReplanTriggered(worldwake_core::ReplanTriggeredPayload {
                agent,
                goal_key: move_goal,
                reason: worldwake_core::ReplanReason::ActionInterrupted {
                    reason: worldwake_core::ActionInterruptReasonTag::Reprioritized,
                },
            }),
        );
        emit_decision_event(
            &mut log,
            11,
            agent,
            EventTag::BlockerRecorded,
            DecisionEventPayload::BlockerRecorded(BlockerRecordedPayload {
                agent,
                blocker_key: BlockerKey {
                    goal_key: move_goal,
                    place: Some(entity(21)),
                    target: Some(target),
                    action_def: Some(ActionDefId(6)),
                },
                discrepancy: Some(worldwake_core::Discrepancy::RouteUnknown),
                blocking_fact: None,
                expires_tick: Tick(99),
                belief_snapshot: None,
            }),
        );

        log
    }

    fn affordance_trace(place: Option<EntityId>, entries: &[(&str, usize)]) -> AffordanceTrace {
        AffordanceTrace {
            place,
            available: entries
                .iter()
                .enumerate()
                .map(|(idx, (action_name, target_count))| AffordanceSummary {
                    def_id: ActionDefId(idx as u32 + 1),
                    action_name: (*action_name).to_string(),
                    target_count: *target_count,
                })
                .collect(),
        }
    }

    fn planning_affordance_trace(
        agent: EntityId,
        tick: u64,
        affordances: AffordanceTrace,
    ) -> AgentDecisionTrace {
        AgentDecisionTrace {
            agent,
            tick: Tick(tick),
            outcome: DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
                affordances: Some(affordances),
                dirty: DirtySet::default(),
                plan_continued: false,
                candidates: CandidateTrace {
                    generated: Vec::new(),
                    evidence: Vec::new(),
                    fully_blocked_desires: Vec::new(),
                    places_reachable: 0,
                    places_after_belief_filter: 0,
                    ranked: Vec::new(),
                    top_ranked_comparison: None,
                    suppressed: Vec::new(),
                    zero_motive: Vec::new(),
                    omitted_political: Vec::new(),
                    omitted_bandit: Vec::new(),
                    omitted_social: Vec::new(),
                    omitted_violation_detection: Vec::new(),
                },
                planning: PlanSearchTrace {
                    attempts: Vec::new(),
                    same_goal_trace: None,
                },
                selection: SelectionTrace {
                    selected_opportunity: None,
                    selected_plan: None,
                    selected_plan_source: None,
                    goal_switch: None,
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
                action_start_failures: Vec::new(),
                discrepancy_trace: Vec::new(),
                exhaustion_snapshot: Vec::new(),
                frame_transition: None,
                patrol_route: PatrolRouteSnapshotTrace::default(),
                selected_patrol_anchor: None,
                pursuit_invalidation: None,
            })),
        }
    }

    fn belief_state(
        kind: Option<EntityKind>,
        last_known_place: Option<EntityId>,
    ) -> BelievedEntityState {
        BelievedEntityState {
            believed_kind: kind,
            last_known_place,
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            ..BelievedEntityState::single_observation_defaults(
                Tick(0),
                PerceptionSource::DirectObservation,
            )
        }
    }

    fn sample_local_survival_state_summary() -> LocalSurvivalStateSummary {
        LocalSurvivalStateSummary {
            place: Some(entity(20)),
            water_source_present: true,
            wash_basin_present: false,
            sleep_affordance_present: true,
            food_source_present: false,
        }
    }

    fn sample_critical_window_report(agent: EntityId) -> CriticalWindowReport {
        CriticalWindowReport {
            agent,
            need: HomeostaticNeedId::Fatigue,
            start_tick: Tick(12),
            end_tick: Tick(16),
            threshold: Permille::new(900).expect("threshold"),
            peak_value: Permille::new(940).expect("peak"),
            frames: vec![CriticalWindowFrame {
                tick: Tick(12),
                need_value: Permille::new(940).expect("need value"),
                selected_goal: Some(GoalKey::from(GoalKind::Sleep)),
                selected_plan_source: Some(SelectedPlanSource::SearchSelection),
                top_competitors: vec![AgendaEntrySnapshot {
                    goal: GoalKey::from(GoalKind::Sleep),
                    priority_class: GoalPriorityClass::Critical,
                    motive_score: 940,
                    provenance_family: None,
                }],
                active_action: Some(ActiveActionSummary {
                    action_name: "sleep".to_string(),
                    instance: ActionInstanceId(9),
                    started_at: Tick(12),
                }),
                exhaustion_state: Some(ExhaustionSummary::FrontierExhausted { expansions_used: 7 }),
                blocker_summary: Some(BlockerSummary {
                    blocker_count: 2,
                    top_blocker: None,
                }),
                local_authoritative_summary: sample_local_survival_state_summary(),
            }],
        }
    }

    fn sample_agenda_entry(
        goal: GoalKey,
        anchor: OpportunityAnchor,
        phase: AgendaPhase,
        tick: Tick,
    ) -> AgendaEntry {
        AgendaEntry {
            key: worldwake_core::OpportunityKey {
                goal_key: goal,
                anchor,
            },
            offer: GoalOffer {
                key: goal,
                anchor,
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
                obligation_source: None,
                commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
                acquisition_quantity: None,
            },
            phase,
            origin: AgendaOrigin::NeedDrive,
            introduced_tick: tick,
            last_reconsidered_tick: tick,
            revival_trigger: None,
            kill_condition: KillCondition::External,
            priority_class: GoalPriorityClass::Background,
            motive_score: 250,
            provenance: None,
            source_reliability_discount: None,
            competition_discount: None,
            feasibility: worldwake_ai::FeasibilityHint::Uncertain,
        }
    }

    fn driver_with_runtime(agent: EntityId, runtime: AgentDecisionRuntime) -> AgentTickDriver {
        let mut driver = AgentTickDriver::new();
        driver.set_runtime(agent, runtime);
        driver
    }

    fn agent_stats_with_locations(name: &str, locations: &[EntityId]) -> AgentStats {
        let mut stats = AgentStats::new(name.to_string(), false);
        stats.location_history = locations.iter().copied().map(Some).collect();
        for place in locations {
            *stats.location_ticks.entry(*place).or_insert(0) += 1;
        }
        stats
    }

    fn agent_stats_with_dirtiness(name: &str, dirtiness: &[u16]) -> AgentStats {
        let mut stats = AgentStats::new(name.to_string(), false);
        stats.needs_samples = dirtiness
            .iter()
            .map(|value| NeedsSample {
                hunger: 0,
                thirst: 0,
                fatigue: 0,
                bladder: 0,
                dirtiness: *value,
            })
            .collect();
        stats
    }

    fn agent_stats_with_need_values(
        name: &str,
        need: HomeostaticNeedId,
        values: &[u16],
    ) -> AgentStats {
        let mut stats = AgentStats::new(name.to_string(), false);
        stats.needs_samples = values
            .iter()
            .map(|value| match need {
                HomeostaticNeedId::Hunger => NeedsSample {
                    hunger: *value,
                    thirst: 0,
                    fatigue: 0,
                    bladder: 0,
                    dirtiness: 0,
                },
                HomeostaticNeedId::Thirst => NeedsSample {
                    hunger: 0,
                    thirst: *value,
                    fatigue: 0,
                    bladder: 0,
                    dirtiness: 0,
                },
                HomeostaticNeedId::Fatigue => NeedsSample {
                    hunger: 0,
                    thirst: 0,
                    fatigue: *value,
                    bladder: 0,
                    dirtiness: 0,
                },
                HomeostaticNeedId::Bladder => NeedsSample {
                    hunger: 0,
                    thirst: 0,
                    fatigue: 0,
                    bladder: *value,
                    dirtiness: 0,
                },
                HomeostaticNeedId::Dirtiness => NeedsSample {
                    hunger: 0,
                    thirst: 0,
                    fatigue: 0,
                    bladder: 0,
                    dirtiness: *value,
                },
            })
            .collect();
        stats
    }

    fn support_resource_source(commodity: CommodityKind) -> ResourceSource {
        ResourceSource {
            commodity,
            available_quantity: Quantity(10),
            max_quantity: Quantity(10),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
            extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
        }
    }

    fn build_split_support_convergence_world() -> World {
        let mut world = World::new(build_prototype_world()).expect("world");
        let mut txn = new_txn(&mut world, 1);
        txn.create_item_lot_with_owner(
            CommodityKind::Apple,
            Quantity(4),
            prototype_place_entity(PrototypePlace::OrchardFarm),
            None,
        )
        .expect("orchard food lot");
        txn.set_component_resource_source(
            prototype_place_entity(PrototypePlace::BanditCamp),
            support_resource_source(CommodityKind::Water),
        )
        .expect("camp water source");
        commit_txn(txn);
        world
    }

    fn build_bundled_support_convergence_world() -> World {
        let mut world = World::new(build_prototype_world()).expect("world");
        let mut txn = new_txn(&mut world, 1);
        txn.set_component_resource_source(
            prototype_place_entity(PrototypePlace::BanditCamp),
            support_resource_source(CommodityKind::Water),
        )
        .expect("camp water source");
        txn.create_item_lot_with_owner(
            CommodityKind::Apple,
            Quantity(4),
            prototype_place_entity(PrototypePlace::BanditCamp),
            None,
        )
        .expect("camp food lot");
        commit_txn(txn);
        world
    }

    fn sample_recipe(
        name: &str,
        inputs: Vec<(CommodityKind, Quantity)>,
        outputs: Vec<(CommodityKind, Quantity)>,
        workstation: Option<WorkstationTag>,
    ) -> RecipeDefinition {
        RecipeDefinition {
            name: name.to_string(),
            inputs,
            outputs,
            work_ticks: NonZeroU32::new(3).expect("work ticks"),
            required_workstation_tag: workstation,
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: BodyCostPerTick::zero(),
        }
    }

    fn belief_state_with_workstation(
        kind: Option<EntityKind>,
        last_known_place: Option<EntityId>,
        workstation_tag: Option<WorkstationTag>,
    ) -> BelievedEntityState {
        BelievedEntityState {
            workstation_tag,
            ..belief_state(kind, last_known_place)
        }
    }

    #[test]
    fn failed_plan_metrics_derive_from_expansion_summaries() {
        let attempt = sample_attempt(vec![
            sample_summary(0, 2),
            sample_summary(2, 5),
            sample_summary(1, 3),
        ]);

        assert_eq!(failed_plan_max_depth(&attempt), 2);
        assert_eq!(failed_plan_candidates(&attempt), 10);
    }

    #[test]
    fn failed_plan_metrics_default_when_no_expansions_recorded() {
        let attempt = sample_attempt(Vec::new());

        assert_eq!(failed_plan_max_depth(&attempt), 0);
        assert_eq!(failed_plan_candidates(&attempt), 0);
    }

    #[test]
    fn failed_plan_location_uses_entity_id_or_fallback() {
        let place = EntityId {
            slot: 7,
            generation: 2,
        };

        assert_eq!(failed_plan_location(Some(place)), "e7g2");
        assert_eq!(failed_plan_location(None), "?");
    }

    #[test]
    fn failed_plan_breakdown_counts_outcomes_and_zero_depth_rows() {
        let frontier_zero = PlanAttemptTrace {
            target_belief_presence: TargetBeliefPresence::Absent,
            ..sample_attempt(vec![sample_summary(0, 2)])
        };
        let budget_nonzero = PlanAttemptTrace {
            outcome: PlanSearchOutcome::BudgetExhausted { expansions_used: 5 },
            ..sample_attempt(vec![sample_summary(2, 3)])
        };
        let frontier_nonzero = sample_attempt(vec![sample_summary(1, 4)]);

        let breakdown =
            failed_plan_breakdown(&[&frontier_zero, &budget_nonzero, &frontier_nonzero]);

        assert_eq!(breakdown.total, 3);
        assert_eq!(breakdown.frontier_exhausted, 2);
        assert_eq!(breakdown.budget_exhausted, 1);
        assert_eq!(breakdown.max_depth_zero, 1);
        assert_eq!(breakdown.target_beliefs_false, 1);
    }

    #[test]
    fn failed_plan_target_belief_labels_render_expected_strings() {
        let absent = PlanAttemptTrace {
            target_belief_presence: TargetBeliefPresence::Absent,
            ..sample_attempt(Vec::new())
        };
        let present = PlanAttemptTrace {
            target_belief_presence: TargetBeliefPresence::Present,
            ..sample_attempt(Vec::new())
        };

        assert_eq!(failed_plan_target_beliefs(&absent), "false");
        assert_eq!(failed_plan_target_beliefs(&present), "true");
        assert_eq!(
            failed_plan_target_beliefs(&sample_attempt(Vec::new())),
            "n/a"
        );
    }

    #[test]
    fn failed_plan_outcome_label_explains_zero_depth_zero_candidate_frontier_exhaustion() {
        let attempt = PlanAttemptTrace {
            outcome: PlanSearchOutcome::FrontierExhausted { expansions_used: 1 },
            ..sample_attempt(vec![sample_summary_with_counts(0, 0, 0, 0, 0)])
        };

        assert_eq!(
            failed_plan_outcome_label(&attempt),
            "frontier-exhausted at depth 0: 0 candidates generated"
        );
    }

    #[test]
    fn failed_plan_outcome_label_explains_all_pruned_by_beam() {
        let attempt = PlanAttemptTrace {
            outcome: PlanSearchOutcome::FrontierExhausted { expansions_used: 1 },
            ..sample_attempt(vec![sample_summary_with_counts(0, 3, 1, 0, 0)])
        };

        assert_eq!(
            failed_plan_outcome_label(&attempt),
            "frontier-exhausted at depth 0: 3 candidates generated, all pruned by beam"
        );
    }

    #[test]
    fn failed_plan_outcome_label_preserves_skip_specific_reason_before_beam_fallback() {
        let attempt = PlanAttemptTrace {
            outcome: PlanSearchOutcome::FrontierExhausted { expansions_used: 1 },
            ..sample_attempt(vec![sample_summary_with_counts(0, 3, 3, 0, 0)])
        };

        assert_eq!(
            failed_plan_outcome_label(&attempt),
            "frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None)"
        );
    }

    #[test]
    fn failed_plan_outcome_label_leaves_non_depth_zero_frontier_exhaustion_unchanged() {
        let attempt = PlanAttemptTrace {
            outcome: PlanSearchOutcome::FrontierExhausted { expansions_used: 3 },
            ..sample_attempt(vec![sample_summary_with_counts(1, 4, 0, 0, 0)])
        };

        assert_eq!(failed_plan_outcome_label(&attempt), "frontier-exhausted");
    }

    #[test]
    fn behavioral_transition_detected_when_action_types_drop_by_half() {
        let bins = BTreeMap::from([
            (
                0,
                BTreeMap::from([
                    ("eat", 1),
                    ("drink", 1),
                    ("sleep", 1),
                    ("wash", 1),
                    ("wander", 1),
                ]),
            ),
            (5, BTreeMap::from([("eat", 2), ("drink", 1)])),
        ]);
        let needs_samples = (0..600).map(|_| need_sample(750)).collect::<Vec<_>>();

        let transitions = behavioral_transitions(&bins, &needs_samples);

        assert_eq!(
            transitions,
            vec![BehavioralTransition {
                tick: 500,
                types_before: 5,
                types_after: 2,
                needs: need_sample(750),
            }]
        );
        assert_eq!(
            format_behavioral_transition(&transitions[0]),
            "**Behavioral transition** at tick 500: action repertoire narrowed (5 types -> 2 types)\n  Needs: hunger=750, thirst=751, fatigue=752, bladder=753, dirtiness=754"
        );
    }

    #[test]
    fn behavioral_transition_not_detected_when_action_types_are_stable() {
        let bins = BTreeMap::from([
            (0, BTreeMap::from([("eat", 1), ("drink", 1), ("sleep", 1)])),
            (5, BTreeMap::from([("eat", 2), ("drink", 1), ("sleep", 1)])),
        ]);
        let needs_samples = (0..600).map(|_| need_sample(700)).collect::<Vec<_>>();

        assert!(behavioral_transitions(&bins, &needs_samples).is_empty());
    }

    #[test]
    fn behavioral_transition_only_fires_when_threshold_is_crossed() {
        let bins = BTreeMap::from([
            (
                0,
                BTreeMap::from([
                    ("eat", 1),
                    ("drink", 1),
                    ("sleep", 1),
                    ("wash", 1),
                    ("wander", 1),
                ]),
            ),
            (
                1,
                BTreeMap::from([("eat", 1), ("drink", 1), ("sleep", 1), ("wash", 1)]),
            ),
            (2, BTreeMap::from([("eat", 1), ("drink", 1), ("sleep", 1)])),
        ]);
        let needs_samples = (0..300).map(|_| need_sample(600)).collect::<Vec<_>>();

        assert!(behavioral_transitions(&bins, &needs_samples).is_empty());
    }

    #[test]
    fn stuck_detector_does_not_treat_startfailed_as_active_frame() {
        let actor = entity(1);
        let mut stats = AgentStats::new("Alice".to_string(), false);
        let mut action_trace = ActionTraceSink::new();
        let needs = NeedsSample {
            hunger: 450,
            thirst: 0,
            fatigue: 0,
            bladder: 0,
            dirtiness: 0,
        };

        for tick in 0..25 {
            action_trace.record(ActionTraceEvent::new(
                Tick(tick),
                actor,
                ActionDefId(1),
                "harvest:Harvest Water".to_string(),
                ActionTraceKind::StartFailed {
                    reason: "resource unavailable".to_string(),
                    request: ResolvedRequestTrace {
                        attempt: RequestAttemptTrace {
                            input_sequence_no: tick,
                            provenance: RequestProvenance::AiPlan,
                        },
                        binding: RequestBindingKind::ReproducedAffordance,
                    },
                    legality: None,
                },
            ));
            let had_event = action_trace
                .events_for_at(actor, Tick(tick))
                .iter()
                .any(|event| !matches!(event.kind, ActionTraceKind::StartFailed { .. }));
            stats.record_idle_tick(had_event, tick, needs);
        }
        stats.flush_idle_window(24);

        let world = World::new(build_prototype_world()).expect("world");
        let anomalies = detect_anomalies(
            &BTreeMap::from([(actor, stats)]),
            &PerceptionTraceSink::new(),
            &EventLog::new(),
            &world,
            &RecipeRegistry::new(),
        );

        assert_eq!(
            anomalies
                .iter()
                .filter(|anomaly| matches!(anomaly.kind, AnomalyKind::StuckAgent))
                .count(),
            1
        );
    }

    #[test]
    fn committed_travel_ticks_only_include_committed_travel_events() {
        let actor = entity(1);
        let events = [
            ActionTraceEvent::new(
                Tick(10),
                actor,
                ActionDefId(1),
                "travel".to_string(),
                ActionTraceKind::Started { targets: vec![] },
            ),
            ActionTraceEvent::new(
                Tick(20),
                actor,
                ActionDefId(1),
                "travel".to_string(),
                ActionTraceKind::Committed {
                    instance_id: ActionInstanceId(7),
                    outcome: CommitOutcome::empty(),
                },
            ),
            ActionTraceEvent::new(
                Tick(30),
                actor,
                ActionDefId(2),
                "eat".to_string(),
                ActionTraceKind::Committed {
                    instance_id: ActionInstanceId(8),
                    outcome: CommitOutcome::empty(),
                },
            ),
        ];
        let event_refs = events.iter().collect::<Vec<_>>();

        assert_eq!(committed_travel_ticks(&event_refs), vec![Tick(20)]);
    }

    #[test]
    fn post_travel_affordance_snapshot_uses_first_new_place_after_travel() {
        let initial = affordance_trace(Some(entity(10)), &[("sleep", 0)]);
        let after_travel = affordance_trace(Some(entity(20)), &[("harvest", 2), ("sell", 1)]);
        let later_same_place = affordance_trace(Some(entity(20)), &[("drink", 0)]);
        let snapshots = vec![
            (Tick(0), &initial),
            (Tick(12), &after_travel),
            (Tick(18), &later_same_place),
        ];

        let post_travel = post_travel_affordance_snapshots(&snapshots, &[Tick(8)]);

        assert_eq!(post_travel.len(), 1);
        assert_eq!(post_travel[0].0, Tick(12));
        assert_eq!(post_travel[0].1.place, Some(entity(20)));
        assert_eq!(post_travel[0].1.available.len(), 2);
    }

    #[test]
    fn final_affordances_use_last_planning_snapshot() {
        let initial = affordance_trace(Some(entity(10)), &[("sleep", 0)]);
        let final_trace = affordance_trace(Some(entity(20)), &[("relieve", 0)]);
        let snapshots = vec![(Tick(0), &initial), (Tick(42), &final_trace)];

        let final_snapshot = final_affordance_snapshot(&snapshots).expect("final affordance");
        assert_eq!(final_snapshot.0, Tick(42));
        assert_eq!(final_snapshot.1.place, Some(entity(20)));
        assert_eq!(final_snapshot.1.available[0].action_name, "relieve");
    }

    #[test]
    fn no_post_travel_affordance_snapshot_without_travel_commit() {
        let initial = affordance_trace(Some(entity(10)), &[("sleep", 0)]);
        let later = affordance_trace(Some(entity(20)), &[("harvest", 2)]);
        let snapshots = vec![(Tick(0), &initial), (Tick(12), &later)];

        assert!(post_travel_affordance_snapshots(&snapshots, &[]).is_empty());
    }

    #[test]
    fn affordance_summary_omits_target_count_when_zero() {
        let no_targets = AffordanceSummary {
            def_id: ActionDefId(1),
            action_name: "sleep".to_string(),
            target_count: 0,
        };
        let with_targets = AffordanceSummary {
            def_id: ActionDefId(2),
            action_name: "harvest".to_string(),
            target_count: 3,
        };

        assert_eq!(format_affordance_summary(&no_targets), "sleep");
        assert_eq!(
            format_affordance_summary(&with_targets),
            "harvest (3 targets)"
        );
    }

    #[test]
    fn affordance_change_detects_appeared_action() {
        let initial = affordance_trace(Some(entity(10)), &[("sleep", 0)]);
        let next_snapshot = affordance_trace(Some(entity(10)), &[("eat", 1), ("sleep", 0)]);
        let snapshots = vec![(Tick(0), &initial), (Tick(5), &next_snapshot)];

        let events = affordance_change_snapshots(&snapshots);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].tick, Tick(5));
        assert_eq!(events[0].appeared, vec!["eat".to_string()]);
        assert!(events[0].disappeared.is_empty());
        assert!(!events[0].place_changed);
    }

    #[test]
    fn affordance_change_detects_disappeared_action() {
        let initial = affordance_trace(Some(entity(10)), &[("eat", 1), ("sleep", 0)]);
        let next_snapshot = affordance_trace(Some(entity(10)), &[("sleep", 0)]);
        let snapshots = vec![(Tick(0), &initial), (Tick(5), &next_snapshot)];

        let events = affordance_change_snapshots(&snapshots);

        assert_eq!(events.len(), 1);
        assert!(events[0].appeared.is_empty());
        assert_eq!(events[0].disappeared, vec!["eat".to_string()]);
    }

    #[test]
    fn affordance_change_ignores_target_count_changes() {
        let initial = affordance_trace(Some(entity(10)), &[("harvest", 1)]);
        let changed = affordance_trace(Some(entity(10)), &[("harvest", 3)]);
        let snapshots = vec![(Tick(0), &initial), (Tick(5), &changed)];

        assert!(affordance_change_snapshots(&snapshots).is_empty());
    }

    #[test]
    fn affordance_change_detects_place_change() {
        let initial = affordance_trace(Some(entity(10)), &[("sleep", 0)]);
        let next_snapshot = affordance_trace(Some(entity(20)), &[("harvest", 2)]);
        let snapshots = vec![(Tick(0), &initial), (Tick(5), &next_snapshot)];

        let events = affordance_change_snapshots(&snapshots);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].appeared, vec!["harvest".to_string()]);
        assert_eq!(events[0].disappeared, vec!["sleep".to_string()]);
        assert!(events[0].place_changed);
    }

    #[test]
    fn no_affordance_change_when_sets_identical() {
        let initial = affordance_trace(Some(entity(10)), &[("eat", 1), ("sleep", 0)]);
        let changed = affordance_trace(Some(entity(10)), &[("eat", 2), ("sleep", 3)]);
        let snapshots = vec![(Tick(0), &initial), (Tick(5), &changed)];

        assert!(affordance_change_snapshots(&snapshots).is_empty());
    }

    #[test]
    fn format_report_includes_affordance_change_lines() {
        let mut driver = AgentTickDriver::new();
        driver.enable_tracing();
        let registry = RecipeRegistry::new();
        let agent = entity(1);
        let initial = affordance_trace(Some(entity(10)), &[("sleep", 0)]);
        let changed = affordance_trace(Some(entity(20)), &[("harvest", 2)]);
        driver
            .trace_sink_mut()
            .expect("trace sink")
            .record(planning_affordance_trace(agent, 0, initial));
        driver
            .trace_sink_mut()
            .expect("trace sink")
            .record(planning_affordance_trace(agent, 5, changed));

        let world = World::new(build_prototype_world()).expect("world");
        let report = format_report(
            "scenario.ron",
            7,
            10,
            &[(agent, "Guard Theron".to_string())],
            &[],
            &BTreeMap::from([(agent, AgentStats::new("Guard Theron".to_string(), false))]),
            &[],
            &EventLog::new(),
            &ActionTraceSink::new(),
            &PerceptionTraceSink::new(),
            &registry,
            &world,
            &driver,
            &[],
            false,
            &[],
            0,
        );

        assert!(
            report.contains("**Affordance changes** (tick 5): +harvest, -sleep (at Unknown#20)")
        );
    }

    #[test]
    fn format_report_renders_agenda_state_summary() {
        let world = World::new(build_prototype_world()).expect("world");
        let registry = RecipeRegistry::new();
        let agent = entity(1);
        let pending_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let suspended_goal = GoalKey::from(GoalKind::MoveCargo {
            commodity: CommodityKind::Bread,
            destination: entity(44),
        });

        let mut pending = sample_agenda_entry(
            pending_goal,
            OpportunityAnchor::Place(entity(18)),
            AgendaPhase::Pending,
            Tick(12),
        );
        pending.revival_trigger = Some(RevivalTrigger::CounterpartyAvailable {
            counterparty: entity(77),
            place: entity(18),
        });

        let mut suspended = sample_agenda_entry(
            suspended_goal,
            OpportunityAnchor::Entity(entity(88)),
            AgendaPhase::Suspended,
            Tick(13),
        );
        suspended.kill_condition = KillCondition::TickExpiry { at_tick: Tick(25) };

        let mut driver = driver_with_runtime(
            agent,
            AgentDecisionRuntime {
                agenda_state: worldwake_ai::AgendaState {
                    committed: Some(sample_agenda_entry(
                        GoalKey::from(GoalKind::Sleep),
                        OpportunityAnchor::Place(entity(17)),
                        AgendaPhase::Committed,
                        Tick(11),
                    )),
                    pending: BTreeMap::from([(pending.key, pending)]),
                    suspended: BTreeMap::from([(suspended.key, suspended)]),
                },
                ..AgentDecisionRuntime::default()
            },
        );
        driver.enable_tracing();

        let report = format_report(
            "scenario.ron",
            7,
            10,
            &[(agent, "Guard Theron".to_string())],
            &[],
            &BTreeMap::from([(agent, AgentStats::new("Guard Theron".to_string(), false))]),
            &[],
            &EventLog::new(),
            &ActionTraceSink::new(),
            &PerceptionTraceSink::new(),
            &registry,
            &world,
            &driver,
            &[],
            false,
            &[],
            0,
        );

        assert!(report.contains("**Agenda state**: committed=Sleep, pending=1, suspended=1"));
        assert!(report.contains("**Pending goals**:"));
        assert!(report.contains("- AcquireCommodity { commodity: Water, purpose: SelfConsume,"));
        assert!(report.contains("revive on counterparty Unknown#77 at Unknown#18"));
        assert!(report.contains("**Suspended goals**:"));
        assert!(report.contains(
            "- MoveCargo { commodity: Bread, destination: EntityId { slot: 44, generation: 0 } } | expires at tick 25"
        ));
    }

    #[test]
    fn render_decision_history_section_covers_all_variants() {
        let agent = entity(1);
        let target = entity(2);
        let log = sample_decision_event_log(agent, target);
        let mut out = String::new();

        render_decision_history_section(&mut out, &log, &[(agent, "Guard Theron".to_string())]);

        assert!(out.contains("## Section 3 — Decision History"));
        assert!(out.contains("| Tick | Agent | Event | Payload Summary |"));
        assert_eq!(
            out.lines().filter(|line| line.starts_with("| ")).count(),
            12
        );
        for event_name in [
            "GoalOffered",
            "GoalSuppressed",
            "GoalCommitted",
            "GoalSuspended",
            "GoalAbandoned",
            "PlanAdopted",
            "PlanInvalidated",
            "ExpectationMismatch",
            "RepairApplied",
            "ReplanTriggered",
            "BlockerRecorded",
        ] {
            assert!(
                out.contains(event_name),
                "missing event row for {event_name}"
            );
        }
        assert!(out.contains("Guard Theron"));
        assert!(out.contains("goal=ProduceCommodity { recipe_id: RecipeId(3) } motive=420 alts=1"));
    }

    #[test]
    fn decision_payload_summary_is_single_line_for_goal_committed() {
        let summary =
            decision_payload_summary(&DecisionEventPayload::GoalCommitted(GoalCommittedPayload {
                agent: entity(1),
                goal_key: GoalKey::from(GoalKind::Sleep),
                motive_score: 420,
                rejected_alternatives: vec![worldwake_core::RejectedAlternativeSummary {
                    goal_key: GoalKey::from(GoalKind::AcquireCommodity {
                        commodity: CommodityKind::Bread,
                        purpose: CommodityPurpose::SelfConsume,
                        quantity: AcquisitionQuantity::single(),
                    }),
                    rejection_reason: GoalRejectionReason::LowerMotive,
                    score_gap: 17,
                }],
            }));

        assert_eq!(summary, "goal=Sleep motive=420 alts=1");
        assert!(!summary.contains('\n'));
    }

    #[test]
    fn format_report_renders_critical_window_section_for_synthetic_report() {
        let world = World::new(build_prototype_world()).expect("world");
        let registry = RecipeRegistry::new();
        let agent = entity(1);
        let log = sample_decision_event_log(agent, entity(2));
        let report = format_report(
            "scenario.ron",
            7,
            10,
            &[(agent, "Guard Theron".to_string())],
            &[],
            &BTreeMap::from([(agent, AgentStats::new("Guard Theron".to_string(), false))]),
            &[],
            &log,
            &ActionTraceSink::new(),
            &PerceptionTraceSink::new(),
            &registry,
            &world,
            &AgentTickDriver::new(),
            &[],
            true,
            &[sample_critical_window_report(agent)],
            1,
        );

        assert!(report.contains("## Section 10 — Critical Window Forensics"));
        assert!(report.contains("### Window 1 — Guard Theron / Fatigue"));
        assert!(report.contains("## Section 3 — Decision History"));
        assert!(report.contains("**Selected goals across captured frames**: Sleep x1"));
        assert!(
            report.contains("**Exhaustion states**: frontier-exhausted (expansions_used=7) x1")
        );
        assert!(report.contains(
            "| 12 | 940 | Sleep | SearchSelection | Sleep | sleep@12 | frontier-exhausted (expansions_used=7) | 2 blockers | Unknown#20: water=yes, wash=no, sleep=yes, food=no |"
        ));
    }

    #[test]
    fn format_report_renders_critical_window_empty_state() {
        let world = World::new(build_prototype_world()).expect("world");
        let registry = RecipeRegistry::new();
        let agent = entity(1);
        let log = sample_decision_event_log(agent, entity(2));
        let report = format_report(
            "scenario.ron",
            7,
            10,
            &[(agent, "Guard Theron".to_string())],
            &[],
            &BTreeMap::from([(agent, AgentStats::new("Guard Theron".to_string(), false))]),
            &[],
            &log,
            &ActionTraceSink::new(),
            &PerceptionTraceSink::new(),
            &registry,
            &world,
            &AgentTickDriver::new(),
            &[],
            true,
            &[],
            0,
        );

        assert!(report.contains("## Section 10 — Critical Window Forensics"));
        assert!(report.contains("No authored-critical windows detected."));
    }

    #[test]
    fn format_report_omits_critical_window_section_when_disabled() {
        let world = World::new(build_prototype_world()).expect("world");
        let registry = RecipeRegistry::new();
        let agent = entity(1);
        let report = format_report(
            "scenario.ron",
            7,
            10,
            &[(agent, "Guard Theron".to_string())],
            &[],
            &BTreeMap::from([(agent, AgentStats::new("Guard Theron".to_string(), false))]),
            &[],
            &EventLog::new(),
            &ActionTraceSink::new(),
            &PerceptionTraceSink::new(),
            &registry,
            &world,
            &AgentTickDriver::new(),
            &[],
            false,
            &[],
            0,
        );

        assert!(!report.contains("## Section 10 — Critical Window Forensics"));
    }

    #[test]
    fn format_report_keeps_section_10_after_section_9() {
        let world = World::new(build_prototype_world()).expect("world");
        let registry = RecipeRegistry::new();
        let agent = entity(1);
        let log = sample_decision_event_log(agent, entity(2));
        let report = format_report(
            "scenario.ron",
            7,
            10,
            &[(agent, "Guard Theron".to_string())],
            &[],
            &BTreeMap::from([(agent, AgentStats::new("Guard Theron".to_string(), false))]),
            &[],
            &log,
            &ActionTraceSink::new(),
            &PerceptionTraceSink::new(),
            &registry,
            &world,
            &AgentTickDriver::new(),
            &[],
            true,
            &[sample_critical_window_report(agent)],
            1,
        );

        let section_8 = report
            .find("## Section 9 — Budget Exhaustion Snapshots")
            .expect("section 8");
        let section_9 = report
            .find("## Section 10 — Critical Window Forensics")
            .expect("section 9");
        assert!(section_8 < section_9);
    }

    #[test]
    fn test_compute_maintenance_rates_tracks_accumulation_and_relief() {
        let samples = vec![
            NeedsSample {
                hunger: 0,
                thirst: 0,
                fatigue: 0,
                bladder: 0,
                dirtiness: 0,
            },
            NeedsSample {
                hunger: 10,
                thirst: 5,
                fatigue: 0,
                bladder: 0,
                dirtiness: 3,
            },
            NeedsSample {
                hunger: 4,
                thirst: 9,
                fatigue: 2,
                bladder: 0,
                dirtiness: 1,
            },
        ];

        let rates = compute_maintenance_rates(&samples);

        assert_eq!(rates[0], (HomeostaticNeedId::Hunger, 10, 6, 4));
        assert_eq!(rates[1], (HomeostaticNeedId::Thirst, 9, 0, 9));
        assert_eq!(rates[2], (HomeostaticNeedId::Fatigue, 2, 0, 2));
        assert_eq!(rates[3], (HomeostaticNeedId::Bladder, 0, 0, 0));
        assert_eq!(rates[4], (HomeostaticNeedId::Dirtiness, 3, 2, 1));
    }

    #[test]
    fn test_recipe_usage_rows_iteration_order_is_deterministic() {
        let mut registry = RecipeRegistry::new();
        let grain_id = registry.register(sample_recipe(
            "Harvest Grain",
            vec![(CommodityKind::Grain, Quantity(1))],
            vec![(CommodityKind::Grain, Quantity(1))],
            Some(WorkstationTag::FieldPlot),
        ));
        let apple_id = registry.register(sample_recipe(
            "Harvest Apples",
            vec![(CommodityKind::Apple, Quantity(1))],
            vec![(CommodityKind::Apple, Quantity(1))],
            Some(WorkstationTag::OrchardRow),
        ));
        let mut stats = AgentStats::new("Alice".to_string(), false);
        stats
            .actions_committed
            .insert("harvest:Harvest Grain".to_string(), 2);
        stats
            .actions_committed
            .insert("harvest:Harvest Apples".to_string(), 5);
        stats
            .actions_committed
            .insert("harvest:Harvest Water".to_string(), 3);
        let known = KnownRecipes::with([grain_id, apple_id]);

        let rows = recipe_usage_rows(&stats, Some(&known), &registry);

        assert_eq!(
            rows,
            vec![
                ("Harvest Grain".to_string(), 2),
                ("Harvest Apples".to_string(), 5),
            ]
        );
    }

    #[test]
    fn test_maintenance_rates_table_renders_for_sampled_agent() {
        let stats = agent_stats_with_dirtiness("Alice", &[10, 15, 12]);

        let rendered =
            render_maintenance_rates_table(&stats.needs_samples).expect("maintenance table");

        assert!(rendered.contains("**Maintenance rates** (‰)"));
        assert!(rendered.contains("| Need | Accumulation | Relief | Net |"));
        assert!(rendered.contains("| Dirtiness | 5 | 3 | 2 |"));
    }

    #[test]
    fn test_recipe_usage_table_renders_for_agent_with_known_recipes() {
        let mut registry = RecipeRegistry::new();
        let apple_id = registry.register(sample_recipe(
            "Harvest Apples",
            vec![(CommodityKind::Apple, Quantity(1))],
            vec![(CommodityKind::Apple, Quantity(1))],
            Some(WorkstationTag::OrchardRow),
        ));
        let grain_id = registry.register(sample_recipe(
            "Harvest Grain",
            vec![(CommodityKind::Grain, Quantity(1))],
            vec![(CommodityKind::Grain, Quantity(1))],
            Some(WorkstationTag::FieldPlot),
        ));
        let mut stats = AgentStats::new("Alice".to_string(), false);
        stats
            .actions_committed
            .insert("harvest:Harvest Apples".to_string(), 4);
        let known = KnownRecipes::with([apple_id, grain_id]);

        let rendered =
            render_recipe_usage_table(&stats, Some(&known), &registry).expect("recipe table");

        assert!(rendered.contains("**Recipe usage**"));
        assert!(rendered.contains("| Recipe | Commits |"));
        assert!(rendered.contains("| Harvest Apples | 4 |"));
        assert!(rendered.contains("| Harvest Grain | 0 |"));
    }

    #[test]
    fn unknown_location_group_labels_place_entities_separately() {
        let place = entity(10);
        let mut store = AgentBeliefStore::new();
        store
            .known_entities
            .insert(place, belief_state(Some(EntityKind::Place), None));

        let groups = unknown_location_entity_groups(&[place], &store);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].0, "(place entity — no parent location)");
        assert_eq!(groups[0].1, vec![place]);
    }

    #[test]
    fn unknown_location_group_keeps_non_place_entities_unknown() {
        let agent = entity(20);
        let mut store = AgentBeliefStore::new();
        store
            .known_entities
            .insert(agent, belief_state(Some(EntityKind::Agent), None));

        let groups = unknown_location_entity_groups(&[agent], &store);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].0, "Unknown location");
        assert_eq!(groups[0].1, vec![agent]);
    }

    #[test]
    fn unknown_location_group_splits_place_and_non_place_entities() {
        let place = entity(30);
        let item = entity(31);
        let mut store = AgentBeliefStore::new();
        store
            .known_entities
            .insert(place, belief_state(Some(EntityKind::Place), None));
        store
            .known_entities
            .insert(item, belief_state(Some(EntityKind::ItemLot), None));

        let groups = unknown_location_entity_groups(&[place, item], &store);
        let labels = groups
            .iter()
            .map(|(label, _)| label.clone())
            .collect::<Vec<_>>();
        let grouped_entities = groups
            .iter()
            .map(|(_, ids)| ids.iter().copied().collect::<BTreeSet<_>>())
            .collect::<Vec<_>>();

        assert_eq!(
            labels,
            vec![
                "(place entity — no parent location)".to_string(),
                "Unknown location".to_string()
            ]
        );
        assert_eq!(
            grouped_entities,
            vec![BTreeSet::from([place]), BTreeSet::from([item]),]
        );
    }

    #[test]
    fn format_death_cause_renders_spec_strings() {
        assert_eq!(
            format_death_cause(DeathCause::NeedDeprivation {
                need: HomeostaticNeedId::Hunger,
            }),
            "NeedDeprivation { Hunger }"
        );
        assert_eq!(format_death_cause(DeathCause::CombatWounds), "CombatWounds");
    }

    #[test]
    fn death_summary_line_includes_tick_and_cause_for_dead_agent() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 42);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_component_dead_at(
                agent,
                DeadAt {
                    tick: Tick(42),
                    cause: DeathCause::NeedDeprivation {
                        need: HomeostaticNeedId::Hunger,
                    },
                },
            )
            .unwrap();
            commit_txn(txn);
            agent
        };

        assert_eq!(
            death_summary_line(&world, agent).as_deref(),
            Some("**Death**: Tick 42 (cause: NeedDeprivation { Hunger })")
        );
    }

    #[test]
    fn death_summary_line_is_absent_for_alive_agent() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            commit_txn(txn);
            agent
        };

        assert_eq!(death_summary_line(&world, agent), None);
    }

    #[test]
    fn test_anomaly_kind_label_emits_new_labels() {
        assert_eq!(
            AnomalyKind::GeographicConvergence.label(),
            "GEOGRAPHIC_CONVERGENCE"
        );
        assert_eq!(
            AnomalyKind::MaintenanceStarvation.label(),
            "MAINTENANCE_STARVATION"
        );
        assert_eq!(AnomalyKind::RecipeMonoculture.label(), "RECIPE_MONOCULTURE");
        assert_eq!(AnomalyKind::AcuteNeedSpike.label(), "ACUTE_NEED_SPIKE");
    }

    #[test]
    fn test_anomaly_render_single_agent_header_unchanged() {
        let anomaly = Anomaly {
            kind: AnomalyKind::RedundantPerception,
            agent_name: "Alice".to_string(),
            additional_agent_names: None,
            description: "desc".to_string(),
            tick_range: None,
        };

        assert_eq!(
            format_anomaly_header(1, &anomaly),
            "### Anomaly 1 — REDUNDANT_PERCEPTION (Alice)"
        );
    }

    #[test]
    fn test_anomaly_render_multi_agent_header() {
        let anomaly = Anomaly {
            kind: AnomalyKind::GeographicConvergence,
            agent_name: "Alice".to_string(),
            additional_agent_names: Some(vec!["Bob".to_string(), "Carol".to_string()]),
            description: "desc".to_string(),
            tick_range: None,
        };

        assert_eq!(
            format_anomaly_header(1, &anomaly),
            "### Anomaly 1 — GEOGRAPHIC_CONVERGENCE (Alice, Bob, Carol)"
        );
    }

    #[test]
    fn test_geographic_convergence_fires_when_three_agents_share_place_for_window() {
        let shared_place = entity(10);
        let world = World::new(build_prototype_world()).expect("world");
        let stats = BTreeMap::from([
            (
                entity(1),
                agent_stats_with_locations("Alice", &vec![shared_place; 250]),
            ),
            (
                entity(2),
                agent_stats_with_locations("Bob", &vec![shared_place; 250]),
            ),
            (
                entity(3),
                agent_stats_with_locations("Carol", &vec![shared_place; 250]),
            ),
        ]);
        let mut anomalies = Vec::new();

        detect_geographic_convergence(&stats, &world, &mut anomalies);

        assert_eq!(anomalies.len(), 1);
        assert!(matches!(
            anomalies[0].kind,
            AnomalyKind::GeographicConvergence
        ));
        assert_eq!(anomalies[0].agent_name, "Alice");
        assert_eq!(
            anomalies[0].additional_agent_names.as_deref(),
            Some(&["Bob".to_string(), "Carol".to_string()][..])
        );
    }

    #[test]
    fn test_geographic_convergence_deduplicates_overlapping_windows() {
        let shared_place = entity(10);
        let world = World::new(build_prototype_world()).expect("world");
        let stats = BTreeMap::from([
            (
                entity(1),
                agent_stats_with_locations("Alice", &vec![shared_place; 250]),
            ),
            (
                entity(2),
                agent_stats_with_locations("Bob", &vec![shared_place; 250]),
            ),
            (
                entity(3),
                agent_stats_with_locations("Carol", &vec![shared_place; 250]),
            ),
        ]);
        let mut anomalies = Vec::new();

        detect_geographic_convergence(&stats, &world, &mut anomalies);

        assert_eq!(anomalies.len(), 1);
        assert_eq!(anomalies[0].tick_range, Some((0, 249)));
    }

    #[test]
    fn test_geographic_convergence_does_not_fire_on_rotating_agents() {
        let place_a = entity(10);
        let place_b = entity(11);
        let world = World::new(build_prototype_world()).expect("world");
        let rotating = (0..250)
            .map(|tick| {
                if (tick / 50) % 2 == 0 {
                    place_a
                } else {
                    place_b
                }
            })
            .collect::<Vec<_>>();
        let inverse = (0..250)
            .map(|tick| {
                if (tick / 50) % 2 == 0 {
                    place_b
                } else {
                    place_a
                }
            })
            .collect::<Vec<_>>();
        let stats = BTreeMap::from([
            (entity(1), agent_stats_with_locations("Alice", &rotating)),
            (entity(2), agent_stats_with_locations("Bob", &inverse)),
            (entity(3), agent_stats_with_locations("Carol", &rotating)),
        ]);
        let mut anomalies = Vec::new();

        detect_geographic_convergence(&stats, &world, &mut anomalies);

        assert!(anomalies.is_empty());
    }

    #[test]
    fn test_geographic_convergence_suppresses_split_support_food_node() {
        let orchard = prototype_place_entity(PrototypePlace::OrchardFarm);
        let world = build_split_support_convergence_world();
        let stats = BTreeMap::from([
            (
                entity(1),
                agent_stats_with_locations("Alice", &vec![orchard; 250]),
            ),
            (
                entity(2),
                agent_stats_with_locations("Bob", &vec![orchard; 250]),
            ),
            (
                entity(3),
                agent_stats_with_locations("Carol", &vec![orchard; 250]),
            ),
        ]);
        let mut anomalies = Vec::new();

        detect_geographic_convergence(&stats, &world, &mut anomalies);

        assert!(anomalies.is_empty());
    }

    #[test]
    fn test_geographic_convergence_still_fires_on_bundled_support_hub() {
        let camp = prototype_place_entity(PrototypePlace::BanditCamp);
        let world = build_bundled_support_convergence_world();
        let stats = BTreeMap::from([
            (
                entity(1),
                agent_stats_with_locations("Alice", &vec![camp; 250]),
            ),
            (
                entity(2),
                agent_stats_with_locations("Bob", &vec![camp; 250]),
            ),
            (
                entity(3),
                agent_stats_with_locations("Carol", &vec![camp; 250]),
            ),
        ]);
        let mut anomalies = Vec::new();

        detect_geographic_convergence(&stats, &world, &mut anomalies);

        assert_eq!(anomalies.len(), 1);
        assert!(matches!(
            anomalies[0].kind,
            AnomalyKind::GeographicConvergence
        ));
    }

    #[test]
    fn test_maintenance_starvation_fires_on_rising_dirtiness_over_window() {
        let thresholds = DriveThresholds::default();
        let high_threshold = need_high_threshold(&thresholds, HomeostaticNeedId::Dirtiness);
        let dirtiness = (0..ANOMALY_ROLLING_WINDOW_TICKS)
            .map(|tick| high_threshold + 50 + u16::try_from(tick.min(49)).expect("tick fits"))
            .collect::<Vec<_>>();
        let stats = BTreeMap::from([(entity(1), agent_stats_with_dirtiness("Alice", &dirtiness))]);
        let thresholds_by_agent = BTreeMap::from([(entity(1), thresholds)]);
        let mut anomalies = Vec::new();

        detect_maintenance_starvation(&stats, &thresholds_by_agent, &mut anomalies);

        assert_eq!(anomalies.len(), 1);
        assert!(matches!(
            anomalies[0].kind,
            AnomalyKind::MaintenanceStarvation
        ));
        assert!(
            anomalies[0]
                .description
                .contains("dirtiness accumulated 49 permille")
        );
        assert!(
            anomalies[0]
                .description
                .contains("relieved only 0 permille")
        );
        assert!(
            anomalies[0]
                .description
                .contains("Net deficit: 49 permille")
        );
        assert!(
            anomalies[0]
                .description
                .contains("above high threshold 850")
        );
        assert_eq!(anomalies[0].tick_range, Some((0, 199)));
    }

    #[test]
    fn test_maintenance_starvation_does_not_fire_when_relief_keeps_up() {
        let dirtiness = (0..ANOMALY_ROLLING_WINDOW_TICKS)
            .map(|tick| if tick % 2 == 0 { 900 } else { 1000 })
            .collect::<Vec<_>>();
        let stats = BTreeMap::from([(entity(1), agent_stats_with_dirtiness("Alice", &dirtiness))]);
        let thresholds_by_agent = BTreeMap::from([(entity(1), DriveThresholds::default())]);
        let mut anomalies = Vec::new();

        detect_maintenance_starvation(&stats, &thresholds_by_agent, &mut anomalies);

        assert!(anomalies.is_empty());
    }

    #[test]
    fn test_maintenance_starvation_does_not_fire_when_avg_below_high() {
        let dirtiness = (0..ANOMALY_ROLLING_WINDOW_TICKS)
            .map(|tick| if tick % 2 == 0 { 800 } else { 900 })
            .collect::<Vec<_>>();
        let stats = BTreeMap::from([(entity(1), agent_stats_with_dirtiness("Alice", &dirtiness))]);
        let thresholds_by_agent = BTreeMap::from([(entity(1), DriveThresholds::default())]);
        let mut anomalies = Vec::new();

        detect_maintenance_starvation(&stats, &thresholds_by_agent, &mut anomalies);

        assert!(anomalies.is_empty());
    }

    #[test]
    fn test_maintenance_starvation_reports_strongest_qualifying_window() {
        let thresholds = DriveThresholds::default();
        let high_threshold = need_high_threshold(&thresholds, HomeostaticNeedId::Dirtiness);
        let dirtiness = (0u16..400u16)
            .map(|tick| {
                if tick < 200 {
                    high_threshold + 50 + tick.min(49)
                } else {
                    high_threshold + 10
                }
            })
            .collect::<Vec<_>>();
        let stats = BTreeMap::from([(entity(1), agent_stats_with_dirtiness("Alice", &dirtiness))]);
        let thresholds_by_agent = BTreeMap::from([(entity(1), thresholds)]);
        let mut anomalies = Vec::new();

        detect_maintenance_starvation(&stats, &thresholds_by_agent, &mut anomalies);

        assert_eq!(anomalies.len(), 1);
        assert_eq!(anomalies[0].tick_range, Some((0, 199)));
    }

    #[test]
    fn test_primary_satisfied_need_classifies_apple_as_hunger() {
        let recipe = sample_recipe(
            "Harvest Apples",
            vec![(CommodityKind::Apple, Quantity(1))],
            vec![(CommodityKind::Apple, Quantity(1))],
            Some(WorkstationTag::OrchardRow),
        );

        assert_eq!(
            primary_satisfied_need(&recipe),
            Some(HomeostaticNeedId::Hunger)
        );
    }

    #[test]
    fn test_primary_satisfied_need_classifies_water_as_thirst() {
        let recipe = sample_recipe(
            "Harvest Water",
            vec![(CommodityKind::Water, Quantity(1))],
            vec![(CommodityKind::Water, Quantity(1))],
            Some(WorkstationTag::Well),
        );

        assert_eq!(
            primary_satisfied_need(&recipe),
            Some(HomeostaticNeedId::Thirst)
        );
    }

    #[test]
    fn test_primary_satisfied_need_returns_none_for_non_consumable() {
        let recipe = sample_recipe(
            "Gather Firewood",
            vec![],
            vec![(CommodityKind::Firewood, Quantity(1))],
            None,
        );

        assert_eq!(primary_satisfied_need(&recipe), None);
    }

    #[test]
    fn test_recipe_monoculture_fires_on_100_percent_apple_share() {
        let mut registry = RecipeRegistry::new();
        let apple_id = registry.register(sample_recipe(
            "Harvest Apples",
            vec![(CommodityKind::Apple, Quantity(1))],
            vec![(CommodityKind::Apple, Quantity(1))],
            Some(WorkstationTag::OrchardRow),
        ));
        let grain_id = registry.register(sample_recipe(
            "Harvest Grain",
            vec![(CommodityKind::Grain, Quantity(1))],
            vec![(CommodityKind::Grain, Quantity(1))],
            Some(WorkstationTag::FieldPlot),
        ));
        let mut stats = AgentStats::new("Alice".to_string(), false);
        stats
            .actions_committed
            .insert("harvest:Harvest Apples".to_string(), 16);
        stats
            .actions_committed
            .insert("harvest:Harvest Grain".to_string(), 0);
        stats.needs_samples = vec![NeedsSample {
            hunger: 0,
            thirst: 0,
            fatigue: 0,
            bladder: 0,
            dirtiness: 0,
        }];
        let mut world = World::new(build_prototype_world()).expect("world");
        let mut store = AgentBeliefStore::new();
        store.known_entities.insert(
            entity(10),
            belief_state_with_workstation(
                Some(EntityKind::Facility),
                Some(entity(20)),
                Some(WorkstationTag::FieldPlot),
            ),
        );
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Alice", ControlSource::Ai).expect("agent");
            txn.set_component_agent_belief_store(agent, store)
                .expect("belief store");
            commit_txn(txn);
            agent
        };
        let stats_by_agent = BTreeMap::from([(agent, stats)]);
        let known_recipes_by_agent =
            BTreeMap::from([(agent, BTreeSet::from([apple_id, grain_id]))]);
        let mut anomalies = Vec::new();

        detect_recipe_monoculture(
            &stats_by_agent,
            &known_recipes_by_agent,
            &registry,
            &world,
            &mut anomalies,
        );

        assert_eq!(anomalies.len(), 1);
        assert!(matches!(anomalies[0].kind, AnomalyKind::RecipeMonoculture));
        assert!(anomalies[0].description.contains(
            "hunger actions: 100% Harvest Apples (16 actions), 0% Harvest Grain (0 actions)"
        ));
        assert!(
            anomalies[0]
                .description
                .contains("final belief store includes workstation FieldPlot evidence")
        );
    }

    #[test]
    fn test_recipe_monoculture_does_not_fire_without_belief_gate() {
        let mut registry = RecipeRegistry::new();
        let apple_id = registry.register(sample_recipe(
            "Harvest Apples",
            vec![(CommodityKind::Apple, Quantity(1))],
            vec![(CommodityKind::Apple, Quantity(1))],
            Some(WorkstationTag::OrchardRow),
        ));
        let grain_id = registry.register(sample_recipe(
            "Harvest Grain",
            vec![(CommodityKind::Grain, Quantity(1))],
            vec![(CommodityKind::Grain, Quantity(1))],
            Some(WorkstationTag::FieldPlot),
        ));
        let mut stats = AgentStats::new("Alice".to_string(), false);
        stats
            .actions_committed
            .insert("harvest:Harvest Apples".to_string(), 16);
        stats
            .actions_committed
            .insert("harvest:Harvest Grain".to_string(), 0);
        stats.needs_samples = vec![NeedsSample {
            hunger: 0,
            thirst: 0,
            fatigue: 0,
            bladder: 0,
            dirtiness: 0,
        }];
        let mut world = World::new(build_prototype_world()).expect("world");
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Alice", ControlSource::Ai).expect("agent");
            commit_txn(txn);
            agent
        };
        let stats_by_agent = BTreeMap::from([(agent, stats)]);
        let known_recipes_by_agent =
            BTreeMap::from([(agent, BTreeSet::from([apple_id, grain_id]))]);
        let mut anomalies = Vec::new();

        detect_recipe_monoculture(
            &stats_by_agent,
            &known_recipes_by_agent,
            &registry,
            &world,
            &mut anomalies,
        );

        assert!(anomalies.is_empty());
    }

    #[test]
    fn test_recipe_monoculture_does_not_fire_on_single_known_recipe() {
        let mut registry = RecipeRegistry::new();
        let apple_id = registry.register(sample_recipe(
            "Harvest Apples",
            vec![(CommodityKind::Apple, Quantity(1))],
            vec![(CommodityKind::Apple, Quantity(1))],
            Some(WorkstationTag::OrchardRow),
        ));
        let mut stats = AgentStats::new("Alice".to_string(), false);
        stats
            .actions_committed
            .insert("harvest:Harvest Apples".to_string(), 16);
        stats.needs_samples = vec![NeedsSample {
            hunger: 0,
            thirst: 0,
            fatigue: 0,
            bladder: 0,
            dirtiness: 0,
        }];
        let mut world = World::new(build_prototype_world()).expect("world");
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Alice", ControlSource::Ai).expect("agent");
            commit_txn(txn);
            agent
        };
        let stats_by_agent = BTreeMap::from([(agent, stats)]);
        let known_recipes_by_agent = BTreeMap::from([(agent, BTreeSet::from([apple_id]))]);
        let mut anomalies = Vec::new();

        detect_recipe_monoculture(
            &stats_by_agent,
            &known_recipes_by_agent,
            &registry,
            &world,
            &mut anomalies,
        );

        assert!(anomalies.is_empty());
    }

    #[test]
    fn test_acute_need_spike_fires_on_40_tick_run() {
        let thresholds = DriveThresholds::default();
        let metabolism = MetabolismProfile::default();
        let critical = thresholds.critical(HomeostaticNeedId::Thirst).value();
        let values = (0..50)
            .map(|tick| if tick < 40 { critical } else { 100 })
            .collect::<Vec<_>>();
        let stats = BTreeMap::from([(
            entity(1),
            agent_stats_with_need_values("Alice", HomeostaticNeedId::Thirst, &values),
        )]);
        let thresholds_by_agent = BTreeMap::from([(entity(1), thresholds)]);
        let metabolism_by_agent = BTreeMap::from([(entity(1), metabolism)]);
        let mut anomalies = Vec::new();

        detect_acute_need_spike(
            &stats,
            &thresholds_by_agent,
            &metabolism_by_agent,
            &mut anomalies,
        );

        assert_eq!(anomalies.len(), 1);
        assert!(matches!(anomalies[0].kind, AnomalyKind::AcuteNeedSpike));
        assert_eq!(anomalies[0].tick_range, Some((0, 39)));
        assert!(
            anomalies[0].description.contains(
                "thirst above critical threshold (850 permille) for 40 consecutive ticks"
            )
        );
        assert!(
            anomalies[0]
                .description
                .contains("within 17% of dehydration tolerance (240 ticks)")
        );
    }

    #[test]
    fn test_acute_need_spike_does_not_fire_below_30_ticks() {
        let thresholds = DriveThresholds::default();
        let critical = thresholds.critical(HomeostaticNeedId::Thirst).value();
        let values = (0..40)
            .map(|tick| if tick < 29 { critical } else { 100 })
            .collect::<Vec<_>>();
        let stats = BTreeMap::from([(
            entity(1),
            agent_stats_with_need_values("Alice", HomeostaticNeedId::Thirst, &values),
        )]);
        let thresholds_by_agent = BTreeMap::from([(entity(1), thresholds)]);
        let metabolism_by_agent = BTreeMap::from([(entity(1), MetabolismProfile::default())]);
        let mut anomalies = Vec::new();

        detect_acute_need_spike(
            &stats,
            &thresholds_by_agent,
            &metabolism_by_agent,
            &mut anomalies,
        );

        assert!(anomalies.is_empty());
    }

    #[test]
    fn test_acute_need_spike_does_not_fire_at_or_above_100_ticks() {
        let thresholds = DriveThresholds::default();
        let critical = thresholds.critical(HomeostaticNeedId::Thirst).value();
        let values = (0..110)
            .map(|tick| if tick < 100 { critical } else { 100 })
            .collect::<Vec<_>>();
        let stats = BTreeMap::from([(
            entity(1),
            agent_stats_with_need_values("Alice", HomeostaticNeedId::Thirst, &values),
        )]);
        let thresholds_by_agent = BTreeMap::from([(entity(1), thresholds)]);
        let metabolism_by_agent = BTreeMap::from([(entity(1), MetabolismProfile::default())]);
        let mut anomalies = Vec::new();

        detect_acute_need_spike(
            &stats,
            &thresholds_by_agent,
            &metabolism_by_agent,
            &mut anomalies,
        );

        assert!(anomalies.is_empty());
    }

    #[test]
    fn test_acute_need_spike_emits_once_per_maximal_run() {
        let thresholds = DriveThresholds::default();
        let critical = thresholds.critical(HomeostaticNeedId::Thirst).value();
        let values = (0..60)
            .map(|tick| if tick < 50 { critical } else { 100 })
            .collect::<Vec<_>>();
        let stats = BTreeMap::from([(
            entity(1),
            agent_stats_with_need_values("Alice", HomeostaticNeedId::Thirst, &values),
        )]);
        let thresholds_by_agent = BTreeMap::from([(entity(1), thresholds)]);
        let metabolism_by_agent = BTreeMap::from([(entity(1), MetabolismProfile::default())]);
        let mut anomalies = Vec::new();

        detect_acute_need_spike(
            &stats,
            &thresholds_by_agent,
            &metabolism_by_agent,
            &mut anomalies,
        );

        assert_eq!(anomalies.len(), 1);
        assert_eq!(anomalies[0].tick_range, Some((0, 49)));
    }

    #[test]
    fn test_acute_need_spike_treats_gaps_as_distinct() {
        let thresholds = DriveThresholds::default();
        let critical = thresholds.critical(HomeostaticNeedId::Thirst).value();
        let values = (0..90)
            .map(|tick| {
                if tick < 40 || (41..81).contains(&tick) {
                    critical
                } else {
                    100
                }
            })
            .collect::<Vec<_>>();
        let stats = BTreeMap::from([(
            entity(1),
            agent_stats_with_need_values("Alice", HomeostaticNeedId::Thirst, &values),
        )]);
        let thresholds_by_agent = BTreeMap::from([(entity(1), thresholds)]);
        let metabolism_by_agent = BTreeMap::from([(entity(1), MetabolismProfile::default())]);
        let mut anomalies = Vec::new();

        detect_acute_need_spike(
            &stats,
            &thresholds_by_agent,
            &metabolism_by_agent,
            &mut anomalies,
        );

        assert_eq!(anomalies.len(), 2);
        assert_eq!(anomalies[0].tick_range, Some((0, 39)));
        assert_eq!(anomalies[1].tick_range, Some((41, 80)));
    }
}
