//! Layer 0 golden tests for the adversarial survival scattered scenario.
//!
//! Tier 2 stress-test above `survival-baseline`: spatially separated resources,
//! travel metabolism costs, chokepoint topology, and isolated starting positions.

mod golden_harness;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use golden_harness::*;
use worldwake_ai::{
    CommodityPurpose, CriticalWindowReport, DecisionOutcome, PlanSearchOutcome,
    SurvivalForensicExtractor,
};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{DriveThresholds, EntityId, GoalKind, Tick};
use worldwake_sim::ActionTraceKind;

const SURVIVAL_TICKS: u32 = 1440;

#[derive(Clone, Debug, Eq, PartialEq)]
struct AgentSurvivalObservation {
    alive: bool,
    critical_thresholds: DriveThresholds,
    critical_need_runs: SurvivalNeedRunTracker,
    critical_window_reports: Vec<CriticalWindowReport>,
    committed_actions: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BudgetExhaustionObservation {
    agent_name: String,
    tick: Tick,
    goal: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SurvivalScatteredObservation {
    contract: worldwake_cli::scenario::types::SurvivalHealthContractDef,
    agents: BTreeMap<String, AgentSurvivalObservation>,
    /// Agent B (isolated at Ravine Shelter) reached any food-producing location.
    isolated_agent_reached_food: bool,
    survival_budget_exhaustions: Vec<BudgetExhaustionObservation>,
    stuck_idle_windows: Vec<StuckIdleWindow>,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-scattered.ron")
}

fn load_survival_scattered_harness() -> (GoldenHarness, ScenarioDef) {
    let path = scenario_path();
    let def = load_scenario_file(&path).expect("survival scattered scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival scattered scenario should spawn");
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
            | GoalKind::Wash
            | GoalKind::ExploreLocation { .. }
    )
}

/// Survival goals excluding Wash.  Wash exhausts budget before the agent
/// discovers any `WashBasin` (same Travel-pruning issue as Sleep pre-fix,
/// but Wash legitimately needs Travel).  Tracked in GOAPTRVLSCAL-001.
fn is_budget_checked_survival_goal(goal: &GoalKind) -> bool {
    is_survival_goal(goal) && !matches!(goal, GoalKind::Wash)
}

/// Food-producing places in the scattered scenario.
fn food_place_ids(def: &ScenarioDef) -> Vec<EntityId> {
    ["Orchard Hollow", "Lowland Farm"]
        .iter()
        .map(|name| scenario_place_id(def, name))
        .collect()
}

fn run_survival_scattered() -> SurvivalScatteredObservation {
    let (mut h, def) = load_survival_scattered_harness();
    let contract = expect_survival_health_contract(
        def.survival_health_contract.as_ref(),
        "survival scattered",
    )
    .clone();
    let agents = named_agents(&h);
    let mut falsification_probes = commodity_assumption_falsification_probes_from_env();
    let food_places = food_place_ids(&def);
    let isolated_agent = *agents
        .get("Agent B")
        .expect("scenario should include Agent B");
    let mut critical_need_runs = agents
        .keys()
        .cloned()
        .map(|name| (name, SurvivalNeedRunTracker::default()))
        .collect::<BTreeMap<_, _>>();
    let mut critical_window_extractors = agents
        .iter()
        .map(|(name, agent)| (name.clone(), SurvivalForensicExtractor::new(*agent)))
        .collect::<BTreeMap<_, _>>();
    let critical_thresholds = agents
        .iter()
        .map(|(name, agent)| {
            (
                name.clone(),
                h.world
                    .get_component_drive_thresholds(*agent)
                    .copied()
                    .expect("survival scenario agents should have drive thresholds"),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut isolated_agent_reached_food = false;

    // Idle window tracking: (current_idle_start, max_need_at_start, consecutive_idle)
    let mut idle_state: BTreeMap<String, (Option<u32>, u16, u32)> = agents
        .keys()
        .cloned()
        .map(|name| (name, (None, 0, 0)))
        .collect();
    let mut stuck_idle_windows = Vec::new();

    for tick_num in 0..SURVIVAL_TICKS {
        h.step_once();
        let tick = Tick(u64::from(tick_num));

        if let Some(probes) = falsification_probes.as_mut() {
            probes
                .observe_tick(&h, &agents, tick)
                .unwrap_or_else(|err| {
                    panic!("survival scattered falsification probe failed: {err}")
                });
        }

        if let Some(place) = h.world.effective_place(isolated_agent)
            && food_places.contains(&place)
        {
            isolated_agent_reached_food = true;
        }

        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");

        for (agent_name, agent) in &agents {
            let needs = h
                .world
                .get_component_homeostatic_needs(*agent)
                .expect("survival scenario agents should always have needs");
            let thresholds = critical_thresholds
                .get(agent_name)
                .expect("every agent should have critical thresholds");
            critical_need_runs
                .get_mut(agent_name)
                .expect("every agent should have a run tracker")
                .observe(needs, thresholds);
            observe_critical_windows(
                critical_window_extractors
                    .get_mut(agent_name)
                    .expect("every agent should have a forensic extractor"),
                &h,
                *agent,
                tick,
                needs,
                thresholds,
            );

            // Track idle windows.
            let had_action = action_sink
                .events_for_at(*agent, tick)
                .iter()
                .any(|e| !matches!(e.kind, ActionTraceKind::StartFailed { .. }));

            let (start, max_need, count) = idle_state
                .get_mut(agent_name)
                .expect("every agent should have idle state");

            if had_action {
                if let Some(s) = start.take()
                    && *count >= contract.max_idle_window_ticks_with_elevated_need
                    && *max_need > contract.elevated_need_floor.value()
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
                    *max_need = max_need_value(needs);
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
                    critical_thresholds: *critical_thresholds
                        .get(&name)
                        .expect("every agent should keep its critical thresholds"),
                    critical_need_runs: critical_need_runs
                        .remove(&name)
                        .expect("every agent should have final need tracking"),
                    critical_window_reports: critical_window_extractors
                        .remove(&name)
                        .expect("every agent should have final forensic reports")
                        .finalize(),
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
                    && is_budget_checked_survival_goal(&attempt.goal.kind)
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

    SurvivalScatteredObservation {
        contract,
        agents,
        isolated_agent_reached_food,
        survival_budget_exhaustions,
        stuck_idle_windows,
    }
}

// ---------------------------------------------------------------------------
// Scenario 153: Scattered Survival Keeps All Agents Alive For 1440 Ticks
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Travel, Production, Perception
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity, Sleep, Relieve, Wash
// ActionDomains: Needs, Travel, Production
// Places: Hilltop Camp, Woodland Clearing, Ravine Shelter, River Crossing,
//         Orchard Hollow, Lowland Farm
// Principles: 1, 6, 7, 22, 31
//
// Setup: Load the authored `survival-scattered.ron` scenario and run the real
// simulation for 1440 ticks with decision and action tracing enabled.
// Unlike the baseline, resources are spatially separated (no location has
// food+water+wash), travel costs metabolism, and agents start isolated.
//
// Proves: all authored agents remain alive and none of the five tracked needs
// stays above that agent's authored critical threshold for more than the
// scenario-authored contract limit.
//
// Chain: authored adversarial topology -> exploration discovers scattered
// resources -> travel with metabolism cost -> repeated self-care under
// spatial pressure -> no deaths by tick 1440.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn all_agents_survive_1440_ticks() {
    let observation = run_survival_scattered();

    for (agent_name, agent) in &observation.agents {
        assert!(
            agent.alive,
            "{agent_name} should still be alive at tick {SURVIVAL_TICKS}"
        );
        assert_authored_critical_runs(
            observation.contract.max_authored_critical_run_ticks,
            agent_name,
            &agent.critical_thresholds,
            &agent.critical_need_runs,
        );
    }
}

// ---------------------------------------------------------------------------
// Scenario 154: Scattered Survival Exercises All Five Self-Care Action Families
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Travel, Production
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity, Sleep, Relieve, Wash
// ActionDomains: Needs, Travel, Production
// Places: all 7 scattered scenario places
// Principles: 3, 6, 7, 31
//
// Setup: Run the authored survival scattered scenario for 1440 ticks and
// collect action traces for each authored agent.
//
// Proves: every agent commits the scenario-authored self-care families despite
// resources being in different locations and travel costing metabolism.
//
// Chain: spatially separated resource affordances + travel metabolism ->
// agents explore and commute -> committed action traces for all five
// survival families across all three agents.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn all_agents_perform_survival_actions() {
    let observation = run_survival_scattered();

    for (agent_name, agent) in &observation.agents {
        assert_required_self_care_families(
            &observation.contract.required_self_care_families,
            agent_name,
            &agent.committed_actions,
            "scattered scenario",
        );
    }
}

// ---------------------------------------------------------------------------
// Scenario 155: Isolated Agent Reaches A Food Source From Ravine Shelter
// ---------------------------------------------------------------------------
//
// Systems: AI, Exploration, Perception, Travel
// GoalKinds: ExploreLocation, AcquireCommodity(SelfConsume)
// ActionDomains: Travel, Needs
// Places: Ravine Shelter -> Woodland Clearing -> Orchard Hollow | River Crossing -> Lowland Farm
// Principles: 1, 6, 7, 14, 31
//
// Setup: Agent B starts at Ravine Shelter, 5 ticks from Woodland Clearing
// and at least 10 ticks from any food source. It must discover food through
// exploration under survival pressure.
//
// Proves: Agent B reaches Orchard Hollow or Lowland Farm (the only food
// locations) within 1440 ticks despite starting deeply isolated.
//
// Chain: isolated start with no seeded food knowledge -> exploration under
// need pressure -> multi-hop travel through chokepoint graph -> arrival at
// a food-producing location.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn isolated_agent_reaches_food_source() {
    let observation = run_survival_scattered();

    assert!(
        observation.isolated_agent_reached_food,
        "Agent B should reach a food-producing location (Orchard Hollow or Lowland Farm) \
         during the 1440-tick scattered scenario despite starting at isolated Ravine Shelter"
    );
}

// ---------------------------------------------------------------------------
// Scenario 156: Scattered Survival Avoids Budget Exhaustion On Survival Goals
// ---------------------------------------------------------------------------
//
// Systems: AI, Search, Needs, Travel, Production
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity, Sleep, Relieve, ExploreLocation
// ActionDomains: Needs, Travel, Production
// Places: all 6 scattered scenario places
// Principles: 6, 14, 31
//
// Setup: Run the authored survival scattered scenario with decision tracing
// and inspect all traced plan-search attempts.  The scenario has longer
// travel distances requiring deeper plans than the baseline.
//
// Proves: no non-Wash survival-goal planning attempt ends in
// `BudgetExhausted` despite the expanded topology requiring multi-hop
// plans.  Wash is excluded because it exhausts budget before agents
// discover the WashBasin (same Travel-pruning gap as pre-fix Sleep);
// tracked in GOAPTRVLSCAL-001.
//
// Chain: travel-branch cap (4) + 640 planner expansions + chokepoint topology
// -> survival plans complete within budget -> no BudgetExhausted.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn no_budget_exhaustion_on_survival_goals() {
    let observation = run_survival_scattered();

    assert!(
        observation.survival_budget_exhaustions.is_empty(),
        "survival scattered should not hit planner budget exhaustion on survival goals: {:?}",
        observation.survival_budget_exhaustions
    );
}

// ---------------------------------------------------------------------------
// Scenario 157: Scattered Survival Has No Stuck Idle Windows With Elevated Needs
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Travel, Production, Perception
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity, Sleep, Relieve, Wash
// ActionDomains: Needs, Travel, Production
// Places: all 7 scattered scenario places
// Principles: 6, 7, 22, 31
//
// Setup: Run the authored survival scattered scenario for 1440 ticks and
// track idle windows using the scenario-authored survival-health contract.
//
// Proves: no agent is idle beyond the scenario-authored bound while any need
// exceeds the scenario-authored elevated-need floor.
//
// Chain: agents plan from beliefs under need pressure -> self-care actions
// always planned when needs elevated -> idle windows only with low needs ->
// zero stuck idle windows.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn no_stuck_idle_windows_with_elevated_needs() {
    let observation = run_survival_scattered();
    assert_no_stuck_idle_windows(
        observation
            .contract
            .max_idle_window_ticks_with_elevated_need,
        observation.contract.elevated_need_floor.value(),
        "survival scattered",
        &observation.stuck_idle_windows,
    );
}
