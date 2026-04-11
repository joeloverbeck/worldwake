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
use worldwake_ai::AgentTickDriver;
use worldwake_ai::decision_trace::{
    AffordanceSummary, AffordanceTrace, AgentDecisionTrace, DecisionOutcome, PlanAttemptTrace,
    PlanSearchOutcome, TargetBeliefPresence,
};
use worldwake_cli::display::entity_display_name;
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario};
use worldwake_core::{DeadAt, DeathCause, EntityId, EntityKind, EventId, EventView, Tick};
use worldwake_sim::{
    ActionTraceEvent, ActionTraceKind, ActionTraceSink, AutonomousControllerRuntime,
    InstitutionalKnowledgeTraceSink, PerceptionTraceSink, PoliticalTraceSink,
    RequestResolutionTraceSink, TickStepServices, step_tick,
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
    // Needs trajectory
    needs_samples: Vec<NeedsSample>,
    // Location tracking
    location_ticks: BTreeMap<EntityId, u32>,
    // Action sequence (for loop detection)
    action_sequence: Vec<String>,
    // Idle tracking
    consecutive_idle_ticks: u32,
    max_consecutive_idle: u32,
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
            needs_samples: Vec::new(),
            location_ticks: BTreeMap::new(),
            action_sequence: Vec::new(),
            consecutive_idle_ticks: 0,
            max_consecutive_idle: 0,
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

    fn record_idle_tick(&mut self, had_action: bool) {
        if had_action {
            self.consecutive_idle_ticks = 0;
        } else {
            self.consecutive_idle_ticks += 1;
            if self.consecutive_idle_ticks > self.max_consecutive_idle {
                self.max_consecutive_idle = self.consecutive_idle_ticks;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Anomaly detection
// ---------------------------------------------------------------------------

struct Anomaly {
    kind: AnomalyKind,
    agent_name: String,
    description: String,
    tick_range: Option<(u64, u64)>,
}

#[derive(Clone, Copy)]
enum AnomalyKind {
    RedundantPerception,
    ActionLoop,
    StuckAgent,
    FailedActionSpiral,
    SustainedCriticalNeed,
    UnaddressedNeed,
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
        }
    }
}

fn detect_anomalies(
    agent_stats: &BTreeMap<EntityId, AgentStats>,
    perception_trace: &PerceptionTraceSink,
    event_log: &worldwake_core::EventLog,
) -> Vec<Anomaly> {
    let mut anomalies = Vec::new();

    for stats in agent_stats.values() {
        // 1. Redundant perception: observed same entity many times
        //    We flag if an entity was observed >= 10 times. A more precise
        //    check (no intervening state change) requires cross-referencing
        //    the event log which is expensive; we use count as a heuristic.
        for (entity, count) in &stats.observation_entity_counts {
            if *count >= 10 {
                anomalies.push(Anomaly {
                    kind: AnomalyKind::RedundantPerception,
                    agent_name: stats.name.clone(),
                    description: format!(
                        "Observed entity {entity} {count} times (may indicate redundant perception if entity state unchanged)",
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
                description: loop_desc,
                tick_range: None,
            });
        }

        // 3. Stuck agents: no actions for >= 20 consecutive ticks
        if stats.max_consecutive_idle >= 20 {
            anomalies.push(Anomaly {
                kind: AnomalyKind::StuckAgent,
                agent_name: stats.name.clone(),
                description: format!(
                    "No actions for {} consecutive ticks",
                    stats.max_consecutive_idle
                ),
                tick_range: None,
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

    // Cross-reference redundant perception more precisely using event log
    refine_redundant_perception(&mut anomalies, agent_stats, perception_trace, event_log);

    anomalies
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

/// Refine redundant perception anomalies by checking if the observed entity
/// actually had state changes between observations. This is done by looking
/// at the event log for events that modify the observed entity between the
/// perception events.
fn refine_redundant_perception(
    anomalies: &mut Vec<Anomaly>,
    _agent_stats: &BTreeMap<EntityId, AgentStats>,
    _perception_trace: &PerceptionTraceSink,
    _event_log: &worldwake_core::EventLog,
) {
    // For v1, the count-based heuristic is sufficient.
    // A future version can cross-reference perception_trace.events_for(agent)
    // with event_log entries that modify the observed entity, and only flag
    // truly redundant observations (no intervening state change).
    let _ = anomalies;
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
    world: &worldwake_core::World,
    driver: &AgentTickDriver,
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

        // Idle tracking
        writeln!(
            out,
            "**Max consecutive idle ticks**: {}\n",
            stats.max_consecutive_idle
        )
        .unwrap();
    }

    // Section 3: Anomaly Flags
    writeln!(out, "## Section 3 — Anomaly Flags\n").unwrap();
    if anomalies.is_empty() {
        writeln!(out, "No anomalies detected.\n").unwrap();
    } else {
        writeln!(out, "{} anomalies detected:\n", anomalies.len()).unwrap();
        for (i, anomaly) in anomalies.iter().enumerate() {
            writeln!(
                out,
                "### Anomaly {} — {} ({})\n",
                i + 1,
                anomaly.kind.label(),
                anomaly.agent_name
            )
            .unwrap();
            writeln!(out, "{}\n", anomaly.description).unwrap();
            if let Some((start, end)) = anomaly.tick_range {
                writeln!(out, "Tick range: {start}–{end}\n").unwrap();
            }
        }
    }

    // Section 4: Raw Event Sample
    writeln!(out, "## Section 4 — Raw Event Sample\n").unwrap();
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

    // Section 5: Per-Agent Belief Summary
    writeln!(out, "## Section 5 — Per-Agent Belief Summary\n").unwrap();
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

    // Section 6: End-State Inventory & Resources
    writeln!(out, "## Section 6 — End-State Inventory & Resources\n").unwrap();

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

    // Section 7: Per-Agent Decision Summary
    writeln!(out, "## Section 7 — Per-Agent Decision Summary\n").unwrap();
    if let Some(sink) = driver.trace_sink() {
        for (agent_id, agent_name) in agents {
            let traces = sink.traces_for(*agent_id);
            if traces.is_empty() {
                writeln!(out, "### {agent_name}\n").unwrap();
                writeln!(out, "No decision traces recorded.\n").unwrap();
                continue;
            }

            writeln!(out, "### {agent_name} ({} decision ticks)\n", traces.len()).unwrap();

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

    let spawned = match spawn_scenario(&def) {
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

    // Create all trace sinks (persistent across all ticks)
    let mut action_trace = ActionTraceSink::new();
    let mut perception_trace = PerceptionTraceSink::new();
    let mut request_resolution_trace = RequestResolutionTraceSink::new();
    let mut politics_trace = PoliticalTraceSink::new();
    let mut institutional_knowledge_trace = InstitutionalKnowledgeTraceSink::new();

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
                std::process::exit(1);
            }
        };

        // Gather per-tick stats
        // Action trace events for this tick
        for event in action_trace.events_at(current_tick) {
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
                }
            }
        }

        // Needs and location sampling (read from world after tick)
        let world = sim.world();
        for (agent_id, stats) in &mut agent_stats {
            // Needs
            if let Some(needs) = world.get_component_homeostatic_needs(*agent_id) {
                stats.needs_samples.push(NeedsSample {
                    hunger: needs.hunger.value(),
                    thirst: needs.thirst.value(),
                    fatigue: needs.fatigue.value(),
                    bladder: needs.bladder.value(),
                    dirtiness: needs.dirtiness.value(),
                });
            }

            // Location
            if let Some(place) = world.effective_place(*agent_id) {
                *stats.location_ticks.entry(place).or_insert(0) += 1;
            }

            // Idle tracking: did this agent have any action trace events this tick?
            let had_action = action_trace
                .events_for_at(*agent_id, current_tick)
                .iter()
                .any(|e| !matches!(e.kind, ActionTraceKind::StartFailed { .. }));
            stats.record_idle_tick(had_action);
        }

        // Progress indicator
        if tick_num > 0 && tick_num % 100 == 0 {
            eprintln!("  tick {tick_num}/{}", cli.ticks);
        }
    }

    eprintln!("Simulation complete. Detecting anomalies...");

    let anomalies = detect_anomalies(&agent_stats, &perception_trace, sim.event_log());

    eprintln!("Found {} anomalies. Writing report...", anomalies.len());

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
        sim.world(),
        &driver,
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
        BehavioralTransition, NeedsSample, PlanAttemptTrace, PlanSearchOutcome,
        behavioral_transitions, committed_travel_ticks, death_summary_line, failed_plan_breakdown,
        failed_plan_candidates, failed_plan_location, failed_plan_max_depth,
        failed_plan_outcome_label, failed_plan_target_beliefs, final_affordance_snapshot,
        format_affordance_summary, format_behavioral_transition, format_death_cause,
        post_travel_affordance_snapshots, unknown_location_entity_groups,
    };
    use std::collections::BTreeMap;
    use std::collections::BTreeSet;
    use worldwake_ai::decision_trace::{
        AffordanceSummary, AffordanceTrace, SearchExpansionSummary, TargetBeliefPresence,
    };
    use worldwake_core::PerceptionSource;
    use worldwake_core::{
        ActionDefId, AgentBeliefStore, BelievedEntityState, CauseRef, ControlSource, DeadAt,
        DeathCause, EntityId, EntityKind, EventLog, GoalKey, GoalKind, HomeostaticNeedId,
        OpportunityAnchor, Tick, VisibilitySpec, WitnessData, World, WorldTxn,
        build_prototype_world,
    };
    use worldwake_sim::{ActionInstanceId, ActionTraceEvent, ActionTraceKind, CommitOutcome};

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
            travel_pruning: None,
            prerequisite_guidance: None,
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
            travel_pruning: None,
            prerequisite_guidance: None,
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
            observed_tick: Tick(0),
            source: PerceptionSource::DirectObservation,
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
}
