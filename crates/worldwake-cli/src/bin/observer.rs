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
use worldwake_cli::display::entity_display_name;
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario};
use worldwake_core::{EntityId, EventId, EventView};
use worldwake_sim::{
    ActionTraceKind, ActionTraceSink, AutonomousControllerRuntime,
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

struct NeedsSample {
    hunger: u16,
    thirst: u16,
    fatigue: u16,
    bladder: u16,
    dirtiness: u16,
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
}

impl AnomalyKind {
    fn label(self) -> &'static str {
        match self {
            Self::RedundantPerception => "REDUNDANT_PERCEPTION",
            Self::ActionLoop => "ACTION_LOOP",
            Self::StuckAgent => "STUCK_AGENT",
            Self::FailedActionSpiral => "FAILED_ACTION_SPIRAL",
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
    }

    // Cross-reference redundant perception more precisely using event log
    refine_redundant_perception(&mut anomalies, agent_stats, perception_trace, event_log);

    anomalies
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
    for stats in agent_stats.values() {
        writeln!(out, "### {}\n", stats.name).unwrap();

        // Action counts
        writeln!(out, "**Actions** (total lifecycle events: {})\n", stats.total_actions()).unwrap();
        let all_action_names: BTreeSet<&String> = stats
            .actions_started
            .keys()
            .chain(stats.actions_committed.keys())
            .chain(stats.actions_aborted.keys())
            .chain(stats.actions_start_failed.keys())
            .collect();
        if !all_action_names.is_empty() {
            writeln!(out, "| Action | Started | Committed | Aborted | StartFailed |").unwrap();
            writeln!(out, "|--------|---------|-----------|---------|-------------|").unwrap();
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
            writeln!(
                out,
                "| Hunger | {} | {} | {} |",
                h_min,
                h_max,
                h_sum / len
            )
            .unwrap();
            writeln!(
                out,
                "| Thirst | {} | {} | {} |",
                t_min,
                t_max,
                t_sum / len
            )
            .unwrap();
            writeln!(
                out,
                "| Fatigue | {} | {} | {} |",
                f_min,
                f_max,
                f_sum / len
            )
            .unwrap();
            writeln!(
                out,
                "| Bladder | {} | {} | {} |",
                b_min,
                b_max,
                b_sum / len
            )
            .unwrap();
            writeln!(
                out,
                "| Dirtiness | {} | {} | {} |",
                d_min,
                d_max,
                d_sum / len
            )
            .unwrap();
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
    writeln!(out, "Total action trace events: {}\n", action_trace.events().len()).unwrap();
    writeln!(out, "```").unwrap();
    // Show last 50 action trace events
    let at_events = action_trace.events();
    let at_start = at_events.len().saturating_sub(50);
    for event in &at_events[at_start..] {
        writeln!(out, "{}", event.summary()).unwrap();
    }
    writeln!(out, "```\n").unwrap();

    // Perception trace summary
    writeln!(
        out,
        "### Perception Trace Summary\n\nTotal perception trace events: {}\n",
        perception_trace.events().len()
    )
    .unwrap();
    writeln!(out, "```").unwrap();
    // Show last 50 perception trace events
    let pt_events = perception_trace.events();
    let pt_start = pt_events.len().saturating_sub(50);
    for event in &pt_events[pt_start..] {
        writeln!(out, "{}", event.summary()).unwrap();
    }
    writeln!(out, "```\n").unwrap();

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
        let (world, event_log, scheduler, controller, rng, recipe_registry) =
            sim.tick_parts_mut();

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
                        stats.action_sequence.push(format!("FAIL:{}", event.action_name));
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
                    *stats
                        .observation_entity_counts
                        .entry(*entity)
                        .or_insert(0) += 1;
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

    eprintln!(
        "Found {} anomalies. Writing report...",
        anomalies.len()
    );

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
