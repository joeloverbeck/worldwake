//! Layer 0 golden tests for the authored survival baseline scenario.

mod golden_harness;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use golden_harness::*;
use worldwake_ai::{CommodityPurpose, DecisionOutcome, PlanSearchOutcome};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{
    AgentBeliefStore, CommodityKind, EntityId, GoalKind, HomeostaticNeeds, Tick, WorkstationTag,
};
use worldwake_sim::ActionTraceKind;

const SURVIVAL_TICKS: u32 = 1440;
const NEED_CRITICAL_THRESHOLD: u16 = 750;
const MAX_CRITICAL_RUN_TICKS: u32 = 100;
/// Minimum idle window length (ticks) to consider an agent "stuck".
const IDLE_THRESHOLD: u32 = 20;
/// Maximum need value (permille) below which idle behavior is expected.
const NEEDS_LOW_CEILING: u16 = 300;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct NeedRunTracker {
    hunger_current: u32,
    hunger_max: u32,
    thirst_current: u32,
    thirst_max: u32,
    fatigue_current: u32,
    fatigue_max: u32,
    bladder_current: u32,
    bladder_max: u32,
    dirtiness_current: u32,
    dirtiness_max: u32,
}

impl NeedRunTracker {
    fn observe(&mut self, needs: &HomeostaticNeeds) {
        update_need_run(
            &mut self.hunger_current,
            &mut self.hunger_max,
            needs.hunger >= pm(NEED_CRITICAL_THRESHOLD),
        );
        update_need_run(
            &mut self.thirst_current,
            &mut self.thirst_max,
            needs.thirst >= pm(NEED_CRITICAL_THRESHOLD),
        );
        update_need_run(
            &mut self.fatigue_current,
            &mut self.fatigue_max,
            needs.fatigue >= pm(NEED_CRITICAL_THRESHOLD),
        );
        update_need_run(
            &mut self.bladder_current,
            &mut self.bladder_max,
            needs.bladder >= pm(NEED_CRITICAL_THRESHOLD),
        );
        update_need_run(
            &mut self.dirtiness_current,
            &mut self.dirtiness_max,
            needs.dirtiness >= pm(NEED_CRITICAL_THRESHOLD),
        );
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AgentSurvivalObservation {
    alive: bool,
    critical_need_runs: NeedRunTracker,
    committed_actions: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BudgetExhaustionObservation {
    agent_name: String,
    tick: Tick,
    goal: String,
}

/// An idle window where an agent had elevated needs but no action.
#[derive(Clone, Debug, Eq, PartialEq)]
struct StuckIdleWindow {
    agent_name: String,
    start_tick: u32,
    end_tick: u32,
    max_need_at_start: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SurvivalBaselineObservation {
    agents: BTreeMap<String, AgentSurvivalObservation>,
    explorer_reached_fertile_fields: bool,
    explorer_food_belief_entities: Vec<EntityId>,
    survival_budget_exhaustions: Vec<BudgetExhaustionObservation>,
    /// Idle windows >= 20 ticks where at least one need exceeded 300 permille.
    stuck_idle_windows: Vec<StuckIdleWindow>,
}

fn update_need_run(current: &mut u32, max: &mut u32, above_threshold: bool) {
    if above_threshold {
        *current += 1;
        *max = (*max).max(*current);
    } else {
        *current = 0;
    }
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-baseline.ron")
}

fn load_survival_baseline_harness() -> (GoldenHarness, ScenarioDef) {
    let path = scenario_path();
    let def = load_scenario_file(&path).expect("survival baseline scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival baseline scenario should spawn");
    let mut harness = GoldenHarness::from_simulation_state(&spawned.state);
    harness.driver.enable_tracing();
    harness.enable_action_tracing();
    (harness, def)
}

fn scenario_place_id(def: &ScenarioDef, place_name: &str) -> EntityId {
    let slot = def
        .places
        .iter()
        .position(|place| place.name == place_name)
        .and_then(|index| u32::try_from(index).ok())
        .expect("scenario place should exist within u32 slot bounds");
    EntityId {
        slot,
        generation: 0,
    }
}

fn named_agents(h: &GoldenHarness) -> BTreeMap<String, EntityId> {
    h.world
        .query_name_and_agent_data()
        .map(|(entity, name, _)| (name.0.clone(), entity))
        .collect()
}

fn is_survival_goal(goal: &GoalKind) -> bool {
    matches!(
        goal,
        GoalKind::AcquireCommodity {
            purpose: CommodityPurpose::SelfConsume,
            ..
        } | GoalKind::ConsumeOwnedCommodity { .. }
            | GoalKind::Sleep
            | GoalKind::Relieve
            | GoalKind::ExploreLocation { .. }
    )
}

fn run_survival_baseline() -> SurvivalBaselineObservation {
    let (mut h, def) = load_survival_baseline_harness();
    let agents = named_agents(&h);
    let fertile_fields = scenario_place_id(&def, "Fertile Fields");
    let explorer = *agents
        .get("Agent B")
        .expect("scenario should include Agent B");
    let mut critical_need_runs = agents
        .keys()
        .cloned()
        .map(|name| (name, NeedRunTracker::default()))
        .collect::<BTreeMap<_, _>>();
    let mut explorer_reached_fertile_fields = false;

    // Idle window tracking: (current_idle_start, max_need_at_start, consecutive_idle)
    let mut idle_state: BTreeMap<String, (Option<u32>, u16, u32)> = agents
        .keys()
        .cloned()
        .map(|name| (name, (None, 0, 0)))
        .collect();
    let mut stuck_idle_windows = Vec::new();

    for tick_num in 0..SURVIVAL_TICKS {
        h.step_once();

        if h.world.effective_place(explorer) == Some(fertile_fields) {
            explorer_reached_fertile_fields = true;
        }

        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");

        for (agent_name, agent) in &agents {
            let needs = h
                .world
                .get_component_homeostatic_needs(*agent)
                .expect("survival scenario agents should always have needs");
            critical_need_runs
                .get_mut(agent_name)
                .expect("every agent should have a run tracker")
                .observe(needs);

            // Track idle windows.
            let had_action = action_sink
                .events_for_at(*agent, Tick(u64::from(tick_num)))
                .iter()
                .any(|e| !matches!(e.kind, ActionTraceKind::StartFailed { .. }));

            let (start, max_need, count) = idle_state
                .get_mut(agent_name)
                .expect("every agent should have idle state");

            if had_action {
                // Close any open idle window.
                if let Some(s) = start.take()
                    && *count >= IDLE_THRESHOLD
                    && *max_need > NEEDS_LOW_CEILING
                {
                    stuck_idle_windows.push(StuckIdleWindow {
                        agent_name: agent_name.clone(),
                        start_tick: s,
                        end_tick: tick_num.saturating_sub(1),
                        max_need_at_start: *max_need,
                    });
                }
                *count = 0;
            } else {
                if start.is_none() {
                    *start = Some(tick_num);
                    let max_n = needs
                        .hunger
                        .value()
                        .max(needs.thirst.value())
                        .max(needs.fatigue.value())
                        .max(needs.bladder.value())
                        .max(needs.dirtiness.value());
                    *max_need = max_n;
                }
                *count += 1;
            }
        }
    }

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let decision_sink = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled");
    let explorer_belief_store = h
        .world
        .get_component_agent_belief_store(explorer)
        .cloned()
        .unwrap_or_else(AgentBeliefStore::new);
    let explorer_food_belief_entities = explorer_belief_store
        .iter_known_entities()
        .filter_map(|(entity, state)| {
            (state.last_known_place == Some(fertile_fields)
                && state.workstation_tag == Some(WorkstationTag::OrchardRow)
                && state
                    .resource_source
                    .as_ref()
                    .is_some_and(|source| source.commodity == CommodityKind::Apple))
            .then_some(*entity)
        })
        .collect::<Vec<_>>();

    let agents = agents
        .into_iter()
        .map(|(name, agent)| {
            let committed_actions = action_sink
                .events_for(agent)
                .iter()
                .filter(|event| matches!(event.kind, ActionTraceKind::Committed { .. }))
                .map(|event| event.action_name.clone())
                .collect::<BTreeSet<_>>();
            (
                name.clone(),
                AgentSurvivalObservation {
                    alive: !h.agent_is_dead(agent),
                    critical_need_runs: critical_need_runs
                        .remove(&name)
                        .expect("every agent should have final need tracking"),
                    committed_actions,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut survival_budget_exhaustions = Vec::new();
    for (agent_name, agent) in named_agents(&h) {
        for trace in decision_sink.traces_for(agent) {
            let DecisionOutcome::Planning(planning) = &trace.outcome else {
                continue;
            };

            for attempt in &planning.planning.attempts {
                if matches!(attempt.outcome, PlanSearchOutcome::BudgetExhausted { .. })
                    && is_survival_goal(&attempt.goal.kind)
                {
                    survival_budget_exhaustions.push(BudgetExhaustionObservation {
                        agent_name: agent_name.clone(),
                        tick: trace.tick,
                        goal: format!("{:?}", attempt.goal.kind),
                    });
                }
            }
        }
    }

    SurvivalBaselineObservation {
        agents,
        explorer_reached_fertile_fields,
        explorer_food_belief_entities,
        survival_budget_exhaustions,
        stuck_idle_windows,
    }
}

fn assert_survival_action_coverage(agent_name: &str, actions: &BTreeSet<String>) {
    let has_relieve = actions.contains("toilet") || actions.contains("relieve_wilderness");
    assert!(
        actions.contains("eat"),
        "{agent_name} should commit eat within the 1440-tick baseline; committed_actions={actions:?}"
    );
    assert!(
        actions.contains("drink"),
        "{agent_name} should commit drink within the 1440-tick baseline; committed_actions={actions:?}"
    );
    assert!(
        actions.contains("sleep"),
        "{agent_name} should commit sleep within the 1440-tick baseline; committed_actions={actions:?}"
    );
    assert!(
        has_relieve,
        "{agent_name} should commit a relief action within the 1440-tick baseline; committed_actions={actions:?}"
    );
    assert!(
        actions.contains("wash"),
        "{agent_name} should commit wash within the 1440-tick baseline; committed_actions={actions:?}"
    );
}

// ---------------------------------------------------------------------------
// Scenario 148: Survival Baseline Keeps All Agents Alive For 1440 Ticks
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Travel, Production, Perception
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity, Sleep, Relieve, Wash
// ActionDomains: Needs, Travel, Production
// Places: Riverside Camp, Fertile Fields, Forest Clearing, Hillside Shelter
// Principles: 1, 6, 22, 31
//
// Setup: Load the authored `survival-baseline.ron` scenario and run the real
// simulation for 1440 ticks with decision and action tracing enabled.
//
// Proves: all authored agents remain alive and none of the five tracked needs
// stays above pm(750) for more than 100 consecutive ticks.
//
// Chain: authored survival substrate -> exploration/perception discovers food
// and water -> repeated self-care actions keep critical runs bounded -> no
// deaths by tick 1440.
#[test]
fn all_agents_survive_1440_ticks() {
    let observation = run_survival_baseline();

    for (agent_name, agent) in &observation.agents {
        assert!(
            agent.alive,
            "{agent_name} should still be alive at tick {SURVIVAL_TICKS}"
        );
        assert!(
            agent.critical_need_runs.hunger_max <= MAX_CRITICAL_RUN_TICKS,
            "{agent_name} hunger exceeded pm({NEED_CRITICAL_THRESHOLD}) for {} consecutive ticks",
            agent.critical_need_runs.hunger_max
        );
        assert!(
            agent.critical_need_runs.thirst_max <= MAX_CRITICAL_RUN_TICKS,
            "{agent_name} thirst exceeded pm({NEED_CRITICAL_THRESHOLD}) for {} consecutive ticks",
            agent.critical_need_runs.thirst_max
        );
        assert!(
            agent.critical_need_runs.fatigue_max <= MAX_CRITICAL_RUN_TICKS,
            "{agent_name} fatigue exceeded pm({NEED_CRITICAL_THRESHOLD}) for {} consecutive ticks",
            agent.critical_need_runs.fatigue_max
        );
        assert!(
            agent.critical_need_runs.bladder_max <= MAX_CRITICAL_RUN_TICKS,
            "{agent_name} bladder exceeded pm({NEED_CRITICAL_THRESHOLD}) for {} consecutive ticks",
            agent.critical_need_runs.bladder_max
        );
        assert!(
            agent.critical_need_runs.dirtiness_max <= MAX_CRITICAL_RUN_TICKS,
            "{agent_name} dirtiness exceeded pm({NEED_CRITICAL_THRESHOLD}) for {} consecutive ticks",
            agent.critical_need_runs.dirtiness_max
        );
    }
}

// ---------------------------------------------------------------------------
// Scenario 149: Survival Baseline Exercises All Five Self-Care Action Families
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Travel, Production
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity, Sleep, Relieve, Wash
// ActionDomains: Needs, Travel, Production
// Places: Riverside Camp, Fertile Fields, Forest Clearing, Hillside Shelter
// Principles: 3, 6, 31
//
// Setup: Run the authored survival baseline for 1440 ticks and collect action
// traces for each authored agent.
//
// Proves: every agent commits eat, drink, sleep, relieve, and wash actions in
// the scenario window.
//
// Chain: varied initial needs and authored place affordances -> repeated
// self-care planning -> committed action traces for all five survival families.
#[test]
fn all_agents_perform_survival_actions() {
    let observation = run_survival_baseline();

    for (agent_name, agent) in &observation.agents {
        assert_survival_action_coverage(agent_name, &agent.committed_actions);
    }
}

// ---------------------------------------------------------------------------
// Scenario 150: Survival Baseline Explorer Discovers Fertile Fields Orchard
// ---------------------------------------------------------------------------
//
// Systems: AI, Exploration, Perception, Production
// GoalKinds: ExploreLocation, AcquireCommodity(SelfConsume)
// ActionDomains: Travel, Production, Needs
// Places: Riverside Camp, Fertile Fields
// Principles: 1, 6, 14, 31
//
// Setup: Agent B starts at Riverside Camp in the authored survival baseline
// and must discover the apple source at Fertile Fields through normal
// exploration and perception.
//
// Proves: Agent B reaches Fertile Fields and retains a belief about the
// orchard-backed apple resource source there.
//
// Chain: no authored seeded food knowledge -> exploration candidate selected
// under survival pressure -> arrival/perception at Fertile Fields -> orchard
// source retained in Agent B's belief store.
#[test]
fn explorer_discovers_food_source() {
    let observation = run_survival_baseline();

    assert!(
        observation.explorer_reached_fertile_fields,
        "Agent B should reach Fertile Fields during the 1440-tick baseline"
    );
    assert!(
        !observation.explorer_food_belief_entities.is_empty(),
        "Agent B should retain a belief about the Fertile Fields orchard resource source"
    );
}

// ---------------------------------------------------------------------------
// Scenario 151: Survival Baseline Avoids Budget Exhaustion On Survival Goals
// ---------------------------------------------------------------------------
//
// Systems: AI, Search, Needs, Travel, Production
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity, Sleep, Relieve, ExploreLocation
// ActionDomains: Needs, Travel, Production
// Places: Riverside Camp, Fertile Fields, Forest Clearing, Hillside Shelter
// Principles: 6, 14, 31
//
// Setup: Run the authored survival baseline with decision tracing enabled and
// inspect all traced plan-search attempts for the three authored agents.
//
// Proves: no survival-goal planning attempt in the baseline ends in
// `BudgetExhausted`.
//
// Chain: cleaned survival candidate/planner surface from `S104SURBASREC-007`
// -> authored baseline self-care plans stay executable -> no traced
// `BudgetExhausted` attempt on survival goals.
#[test]
fn no_budget_exhaustion_on_survival_goals() {
    let observation = run_survival_baseline();

    assert!(
        observation.survival_budget_exhaustions.is_empty(),
        "survival baseline should not hit planner budget exhaustion on survival goals: {:?}",
        observation.survival_budget_exhaustions
    );
}

// ---------------------------------------------------------------------------
// Scenario 152: Survival Baseline Has No Stuck Idle Windows With Elevated Needs
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Travel, Production, Perception
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity, Sleep, Relieve, Wash
// ActionDomains: Needs, Travel, Production
// Places: Riverside Camp, Fertile Fields, Forest Clearing, Hillside Shelter
// Principles: 6, 22, 31
//
// Setup: Run the authored survival baseline for 1440 ticks and track idle
// windows (>= 20 consecutive ticks with no action trace events). For each
// idle window, capture the agent's needs at the start of the window.
//
// Proves: no agent is idle for 20+ consecutive ticks while any need exceeds
// 300 permille. Idle windows with all needs low are expected behavior in a
// survival-only scenario (no higher-order goals).
//
// Chain: agents plan from beliefs under need pressure -> self-care actions
// are always planned when needs are elevated -> idle windows only occur when
// no pressing goal exists -> zero stuck idle windows with elevated needs.
#[test]
fn no_stuck_idle_windows_with_elevated_needs() {
    let observation = run_survival_baseline();

    assert!(
        observation.stuck_idle_windows.is_empty(),
        "survival baseline should have no idle windows >= 20 ticks with needs > 300 permille: {:?}",
        observation.stuck_idle_windows
    );
}
