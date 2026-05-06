//! Layer 0 golden tests for the authored survival baseline scenario.

mod golden_harness;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use golden_harness::*;
use worldwake_ai::{
    CommodityPurpose, CriticalWindowReport, DecisionOutcome, PlanSearchOutcome,
    SurvivalForensicExtractor,
};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{
    AgentBeliefStore, CommodityKind, DriveThresholds, EntityId, GoalKind, Tick, WorkstationTag,
};
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
struct SurvivalBaselineObservation {
    contract: worldwake_cli::scenario::types::SurvivalHealthContractDef,
    agents: BTreeMap<String, AgentSurvivalObservation>,
    explorer_reached_fertile_fields: bool,
    explorer_food_belief_entities: Vec<EntityId>,
    survival_budget_exhaustions: Vec<BudgetExhaustionObservation>,
    stuck_idle_windows: Vec<StuckIdleWindow>,
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
    let contract =
        expect_survival_health_contract(def.survival_health_contract.as_ref(), "survival baseline");
    let agents = named_agents(&h);
    let mut falsification_probes = commodity_assumption_falsification_probes_from_env();
    let fertile_fields = scenario_place_id(&def, "Fertile Fields");
    let explorer = *agents
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
        let tick = Tick(u64::from(tick_num));

        if let Some(probes) = falsification_probes.as_mut() {
            probes
                .observe_tick(&h, &agents, tick)
                .unwrap_or_else(|err| {
                    panic!("survival baseline falsification probe failed: {err}")
                });
        }

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
            let had_action = golden_harness::agent_has_non_failed_action_or_active(
                &h,
                action_sink,
                *agent,
                tick,
            );

            let (start, max_need, count) = idle_state
                .get_mut(agent_name)
                .expect("every agent should have idle state");

            if had_action {
                // Close any open idle window.
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
        contract: contract.clone(),
        agents,
        explorer_reached_fertile_fields,
        explorer_food_belief_entities,
        survival_budget_exhaustions,
        stuck_idle_windows,
    }
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
// stays above that agent's authored critical threshold for more than the
// scenario-authored contract limit.
//
// Chain: authored survival substrate -> exploration/perception discovers food
// and water -> repeated self-care actions keep critical runs bounded -> no
// deaths by tick 1440.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn all_agents_survive_1440_ticks() {
    let observation = run_survival_baseline();

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
// Proves: every agent commits the scenario-authored self-care families in the
// scenario window.
//
// Chain: varied initial needs and authored place affordances -> repeated
// self-care planning -> committed action traces for all five survival families.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn all_agents_perform_survival_actions() {
    let observation = run_survival_baseline();

    for (agent_name, agent) in &observation.agents {
        assert_required_self_care_families(
            &observation.contract.required_self_care_families,
            agent_name,
            &agent.committed_actions,
            "baseline scenario",
        );
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
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
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
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
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
// windows using the scenario-authored survival-health contract.
//
// Proves: no agent is idle beyond the scenario-authored bound while any need
// exceeds the scenario-authored elevated-need floor. Idle windows with all
// needs low are expected behavior in a survival-only scenario.
//
// Chain: agents plan from beliefs under need pressure -> self-care actions
// are always planned when needs are elevated -> idle windows only occur when
// no pressing goal exists -> zero stuck idle windows with elevated needs.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn no_stuck_idle_windows_with_elevated_needs() {
    let observation = run_survival_baseline();
    assert_no_stuck_idle_windows(
        observation
            .contract
            .max_idle_window_ticks_with_elevated_need,
        observation.contract.elevated_need_floor.value(),
        "survival baseline",
        &observation.stuck_idle_windows,
    );
}

#[test]
fn survival_contract_guard_rejects_missing_authored_contract() {
    let def = ScenarioDef {
        seed: 1,
        places: vec![worldwake_cli::scenario::types::PlaceDef {
            name: "Village".into(),
            tags: vec![],
            visibility_profile: None,
            sleep_quality: None,
            place_dirtiness: None,
            latrine_fullness: None,
        }],
        edges: vec![],
        agents: vec![],
        bandit_camps: Vec::new(),
        offices: vec![],
        artifacts: vec![],
        items: vec![],
        facilities: vec![],
        resource_sources: vec![],
        hostilities: vec![],

        commodity_decay: None,
        survival_health_contract: None,
        compaction_interval: 0,
        scenario_lint_overrides: std::collections::BTreeMap::new(),
        harvest_trace_retention_ticks: None,
    };

    let result = std::panic::catch_unwind(|| {
        let _ = expect_survival_health_contract(def.survival_health_contract.as_ref(), "synthetic");
    });
    assert!(
        result.is_err(),
        "survival goldens should reject scenarios missing survival_health_contract"
    );
}

#[test]
fn survival_need_run_tracker_uses_authored_drive_thresholds() {
    let mut tracker = SurvivalNeedRunTracker::default();
    let thresholds = DriveThresholds::new(
        worldwake_core::ThresholdBand::new(pm(250), pm(450), pm(650), pm(850)).unwrap(),
        worldwake_core::ThresholdBand::new(pm(200), pm(400), pm(650), pm(820)).unwrap(),
        worldwake_core::ThresholdBand::new(pm(300), pm(550), pm(780), pm(900)).unwrap(),
        worldwake_core::ThresholdBand::new(pm(300), pm(550), pm(780), pm(900)).unwrap(),
        worldwake_core::ThresholdBand::new(pm(350), pm(600), pm(820), pm(920)).unwrap(),
        worldwake_core::ThresholdBand::new(pm(150), pm(350), pm(600), pm(850)).unwrap(),
        worldwake_core::ThresholdBand::new(pm(100), pm(300), pm(550), pm(800)).unwrap(),
    );
    let needs = worldwake_core::HomeostaticNeeds {
        hunger: pm(800),
        thirst: pm(830),
        fatigue: pm(899),
        bladder: pm(901),
        dirtiness: pm(930),
    };

    tracker.observe(&needs, &thresholds);

    assert_eq!(
        tracker.hunger_max, 0,
        "hunger should stay below authored critical"
    );
    assert_eq!(
        tracker.thirst_max, 1,
        "thirst should count once above authored critical"
    );
    assert_eq!(
        tracker.fatigue_max, 0,
        "fatigue should stay below authored critical"
    );
    assert_eq!(
        tracker.bladder_max, 1,
        "bladder should count once above authored critical"
    );
    assert_eq!(
        tracker.dirtiness_max, 1,
        "dirtiness should count once above authored critical"
    );
}
