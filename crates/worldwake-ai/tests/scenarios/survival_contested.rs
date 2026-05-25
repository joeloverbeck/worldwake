//! Layer 0 golden tests for the contested survival scenario.
//!
//! Tier 3 stress-test above `survival-scattered`: 4 agents, tight resource
//! capacities, two water sources, chokepoint topology, and wash co-located
//! with one well.  Uses only the profile set already exercised by
//! `survival-baseline` and `survival-scattered` — no `DiversificationProfile`
//! or other opt-in feature profiles.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::golden_harness::*;
use worldwake_ai::{
    CommodityPurpose, CriticalWindowReport, DecisionOutcome, PlanSearchOutcome,
    SurvivalForensicExtractor,
};
use worldwake_cli::scenario::{
    load_scenario_file, spawn_scenario,
    types::{ScenarioDef, SurvivalCriticalRunLimitsDef},
};
use worldwake_core::{
    DecisionEventPayload, DriveThresholds, EntityId, EventTag, EventView, GoalKind, Tick,
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
struct SurvivalContestedObservation {
    contract: worldwake_cli::scenario::types::SurvivalHealthContractDef,
    agents: BTreeMap<String, AgentSurvivalObservation>,
    /// Places (by name) where at least one agent committed a `drink` action.
    drink_places: BTreeSet<String>,
    /// Food-producing places reached by at least one North-side agent (A or B).
    north_reached_food: bool,
    /// Food-producing places reached by at least one South-side agent (C or D).
    south_reached_food: bool,
    /// Agents with at least one persisted `WashFacilityUsed` decision payload.
    wash_facility_users: BTreeSet<String>,
    survival_budget_exhaustions: Vec<BudgetExhaustionObservation>,
    stuck_idle_windows: Vec<StuckIdleWindow>,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-contested.ron")
}

fn load_survival_contested_harness() -> (GoldenHarness, ScenarioDef) {
    let path = scenario_path();
    let def = load_scenario_file(&path).expect("survival contested scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival contested scenario should spawn");
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

fn is_budget_checked_survival_goal(goal: &GoalKind) -> bool {
    is_survival_goal(goal)
}

fn food_place_ids(def: &ScenarioDef) -> Vec<EntityId> {
    ["East Orchard", "West Grainfield"]
        .iter()
        .map(|name| scenario_place_id(def, name))
        .collect()
}

fn water_place_names() -> BTreeSet<String> {
    ["Stone Well", "Spring Basin"]
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

fn contract_run_limit_overrides(
    limits: Option<&SurvivalCriticalRunLimitsDef>,
) -> SurvivalCriticalRunLimitOverrides {
    let Some(limits) = limits else {
        return SurvivalCriticalRunLimitOverrides::default();
    };

    SurvivalCriticalRunLimitOverrides {
        hunger: limits.hunger,
        thirst: limits.thirst,
        fatigue: limits.fatigue,
        bladder: limits.bladder,
        dirtiness: limits.dirtiness,
    }
}

fn run_survival_contested() -> SurvivalContestedObservation {
    let (mut h, def) = load_survival_contested_harness();
    let contract = expect_survival_health_contract(
        def.survival_health_contract.as_ref(),
        "survival contested",
    )
    .clone();
    let agents = named_agents(&h);
    let mut falsification_probes = commodity_assumption_falsification_probes_from_env();
    let food_places = food_place_ids(&def);
    let place_name_by_id: BTreeMap<EntityId, String> = def
        .places
        .iter()
        .enumerate()
        .map(|(index, place)| {
            let slot = u32::try_from(index).expect("scenario place index fits in u32");
            (
                EntityId {
                    slot,
                    generation: 0,
                },
                place.name.clone(),
            )
        })
        .collect();
    let north_agents: BTreeSet<EntityId> = ["Agent A", "Agent B"]
        .iter()
        .filter_map(|name| agents.get(*name).copied())
        .collect();
    let south_agents: BTreeSet<EntityId> = ["Agent C", "Agent D"]
        .iter()
        .filter_map(|name| agents.get(*name).copied())
        .collect();

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
    let mut north_reached_food = false;
    let mut south_reached_food = false;
    let mut drink_places: BTreeSet<String> = BTreeSet::new();

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
                    panic!("survival contested falsification probe failed: {err}")
                });
        }

        for agent in &north_agents {
            if let Some(place) = h.world.effective_place(*agent)
                && food_places.contains(&place)
            {
                north_reached_food = true;
            }
        }
        for agent in &south_agents {
            if let Some(place) = h.world.effective_place(*agent)
                && food_places.contains(&place)
            {
                south_reached_food = true;
            }
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

            let tick_events = action_sink.events_for_at(*agent, tick);

            let had_action = crate::golden_harness::agent_has_non_failed_action_or_active(
                &h,
                action_sink,
                *agent,
                tick,
            );

            // Capture drink commits with the agent's current place (committed
            // actions occur at the actor's place, so effective_place post-step
            // reflects where the drink happened).
            let committed_drink = tick_events.iter().any(|e| {
                matches!(e.kind, ActionTraceKind::Committed { .. }) && e.action_name == "drink"
            });
            if committed_drink
                && let Some(place) = h.world.effective_place(*agent)
                && let Some(name) = place_name_by_id.get(&place)
            {
                drink_places.insert(name.clone());
            }

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

    let agent_name_by_id = agents
        .iter()
        .map(|(name, agent)| (*agent, name.clone()))
        .collect::<BTreeMap<_, _>>();

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

    let wash_facility_users = h
        .event_log
        .events_by_tag(EventTag::WashFacilityUsed)
        .iter()
        .filter_map(|id| h.event_log.get(*id))
        .filter_map(|record| match record.decision_payload()? {
            DecisionEventPayload::WashFacilityUsed(payload) => {
                agent_name_by_id.get(&payload.user).cloned()
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();

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

    SurvivalContestedObservation {
        contract,
        agents,
        drink_places,
        north_reached_food,
        south_reached_food,
        wash_facility_users,
        survival_budget_exhaustions,
        stuck_idle_windows,
    }
}

// ---------------------------------------------------------------------------
// Scenario 158: Contested Survival Keeps All Four Agents Alive For 1440 Ticks
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Travel, Production, Perception
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity, Sleep, Relieve, Wash
// ActionDomains: Needs, Travel, Production
// Places: North Camp, South Camp, Forest Glade, Central Crossing,
//         Stone Well, Spring Basin, East Orchard, West Grainfield
// Principles: 1, 6, 7, 10, 22, 31
//
// Setup: Load the authored `survival-contested.ron` scenario and run the real
// simulation for 1440 ticks with decision and action tracing enabled.  Four
// AI agents (2 starting at North Camp, 2 at South Camp) share two low-capacity
// water sources (cap=4 each), one contested orchard (cap=6), and one
// contested grainfield (cap=5).  Aligned starting needs force overlapping
// demand windows so instantaneous contention is real.
//
// Proves: all four agents remain alive and none of the five tracked needs
// stays above that agent's authored critical threshold for more than the
// scenario-authored contract limit.
// Demonstrates population-level survival under contention using only the
// profile set exercised by the first two survival scenarios — no
// `DiversificationProfile` opt-in.
//
// Chain: aligned starting needs -> concurrent demand at low-capacity wells ->
// belief invalidation when one agent draws water before another -> planner
// replans toward alternate well or waits for regen -> repeated self-care
// completes within bounded critical runs.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn all_agents_survive_1440_ticks() {
    let observation = run_survival_contested();

    for (agent_name, agent) in &observation.agents {
        assert!(
            agent.alive,
            "{agent_name} should still be alive at tick {SURVIVAL_TICKS}"
        );
        assert_authored_critical_runs_with_overrides(
            observation.contract.max_authored_critical_run_ticks,
            contract_run_limit_overrides(observation.contract.critical_run_limits.as_ref()),
            agent_name,
            &agent.critical_thresholds,
            &agent.critical_need_runs,
        );
    }
}

// ---------------------------------------------------------------------------
// Scenario 159: Contested Survival Exercises All Five Self-Care Action Families
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Travel, Production
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity, Sleep, Relieve, Wash
// ActionDomains: Needs, Travel, Production
// Places: all 8 contested-scenario places
// Principles: 3, 6, 7, 31
//
// Setup: Run the authored survival contested scenario for 1440 ticks and
// collect action traces for each of the four authored agents.
//
// Proves: every agent commits the scenario-authored self-care families despite
// sharing low-capacity resources and needing to cross chokepoint topology for
// cross-camp resources.
//
// Chain: contested resources + chokepoint topology -> agents explore, queue,
// and replan under contention -> committed action traces for all five
// survival families across all four agents.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn all_agents_perform_survival_actions() {
    let observation = run_survival_contested();

    for (agent_name, agent) in &observation.agents {
        assert_required_self_care_families(
            &observation.contract.required_self_care_families,
            agent_name,
            &agent.committed_actions,
            "contested scenario",
        );
    }
}

// ---------------------------------------------------------------------------
// Scenario 160: Contested Survival Draws From Both Water Sources Across The Run
// ---------------------------------------------------------------------------
//
// Systems: AI, Travel, Production, Perception
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity
// ActionDomains: Needs, Travel, Production
// Places: Stone Well, Spring Basin
// Principles: 1, 6, 7, 10, 31
//
// Setup: Run the authored survival contested scenario and record the place
// of every `drink` commit across all agents.  Stone Well is 3 ticks from
// North Camp; Spring Basin is 3 ticks from South Camp.  Without belief
// invalidation and replanning, agents would never visit the "other side"
// well.
//
// Proves: at least one agent draws from Stone Well AND at least one agent
// draws from Spring Basin during the run.  Demonstrates that contention at
// low-capacity wells drives agents to discover and use the alternative
// source via exploration + belief invalidation (not via a feature opt-in).
//
// Chain: aligned demand -> capacity saturation at nearest well -> belief
// dirtied when a peer consumes water -> replan toward alternate source via
// S102 frontier-aware exploration -> drink commit at the alternate place.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn both_water_sources_are_used() {
    let observation = run_survival_contested();

    let expected = water_place_names();
    let actual: BTreeSet<String> = observation
        .drink_places
        .iter()
        .filter(|name| expected.contains(*name))
        .cloned()
        .collect();
    assert_eq!(
        actual, expected,
        "both water sources should be drawn from across the run; \
         expected={expected:?} drink_places={:?}",
        observation.drink_places
    );
}

// ---------------------------------------------------------------------------
// Scenario 161: Contested Survival Has Both Camp Sides Reach A Food Source
// ---------------------------------------------------------------------------
//
// Systems: AI, Exploration, Perception, Travel
// GoalKinds: ExploreLocation, AcquireCommodity(SelfConsume)
// ActionDomains: Travel, Needs
// Places: East Orchard, West Grainfield
// Principles: 1, 6, 7, 14, 31
//
// Setup: Agents A and B start at North Camp; C and D start at South Camp.
// Food sources (East Orchard, West Grainfield) are reachable only through
// Central Crossing from either side.  No seeded food knowledge.
//
// Proves: at least one North-side agent reaches a food-producing place AND
// at least one South-side agent reaches a food-producing place during the
// 1440-tick run.  Confirms the chokepoint topology is genuinely traversed
// from both starting positions under survival pressure.
//
// Chain: hunger rising -> exploration drive activated -> multi-hop travel
// through Central Crossing -> arrival at East Orchard or West Grainfield
// on both camp sides.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn both_camp_sides_reach_food() {
    let observation = run_survival_contested();

    assert!(
        observation.north_reached_food,
        "at least one North-side agent (A or B) should reach East Orchard or West Grainfield"
    );
    assert!(
        observation.south_reached_food,
        "at least one South-side agent (C or D) should reach East Orchard or West Grainfield"
    );
}

// ---------------------------------------------------------------------------
// Scenario 162: Contested Survival Avoids Budget Exhaustion On Survival Goals
// ---------------------------------------------------------------------------
//
// Systems: AI, Search, Needs, Travel, Production
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity, Sleep, Relieve, ExploreLocation
// ActionDomains: Needs, Travel, Production
// Places: all 8 contested-scenario places
// Principles: 6, 14, 31
//
// Setup: Run the authored survival contested scenario with decision tracing
// and inspect all traced plan-search attempts.  The scenario adds a fourth
// agent and tighter resource capacities over scattered but preserves the
// same cognitive budget (640 expansions, beam_width 12).
//
// Proves: no survival-goal planning attempt ends in `BudgetExhausted`.
// Wash is included through the same goal-key inspection convention as Eat,
// Drink, Sleep, Relieve, and ExploreLocation.
//
// Chain: preserved travel-branch cap (4) + 640 planner expansions +
// chokepoint topology + contention -> survival plans complete within
// budget -> no BudgetExhausted.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn no_budget_exhaustion_on_survival_goals() {
    let observation = run_survival_contested();

    assert!(
        observation.survival_budget_exhaustions.is_empty(),
        "survival contested should not hit planner budget exhaustion on survival goals: {:?}",
        observation.survival_budget_exhaustions
    );
}

// ---------------------------------------------------------------------------
// Scenario 163: Contested Survival Persists Wash Facility Commit Payloads
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Travel, Production
// GoalKinds: Wash
// ActionDomains: Needs
// Places: Spring Basin
// Principles: 4, 14, 29A, 31
//
// Setup: Run the authored survival contested scenario for 1440 ticks and
// inspect the append-only decision event log for `WashFacilityUsed` payloads.
//
// Proves: every authored agent that is required to satisfy Wash commits through
// the existing generic Wash facility payload surface.  The assertion proves the
// D5 commit branch without adding a Wash-specific failure-attribution variant.
//
// Chain: scenario-authored Wash self-care requirement -> belief-backed basin
// discovery -> wash action commit -> persisted `WashFacilityUsed` payload keyed
// by user and basin.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn wash_facility_payloads_record_every_agent() {
    let observation = run_survival_contested();

    let expected = observation.agents.keys().cloned().collect::<BTreeSet<_>>();
    assert_eq!(
        observation.wash_facility_users, expected,
        "each contested-scenario agent should emit at least one WashFacilityUsed payload"
    );
}

// ---------------------------------------------------------------------------
// Scenario 164: Contested Survival Has No Stuck Idle Windows With Elevated Needs
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Travel, Production, Perception
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity, Sleep, Relieve, Wash
// ActionDomains: Needs, Travel, Production
// Places: all 8 contested-scenario places
// Principles: 6, 7, 22, 31
//
// Setup: Run the authored survival contested scenario for 1440 ticks and
// track idle windows using the scenario-authored survival-health contract.
//
// Proves: no agent is idle beyond the scenario-authored bound while any need
// exceeds the scenario-authored elevated-need floor.
//
// Chain: agents plan from beliefs under need pressure -> self-care actions
// always planned when needs elevated -> contention at one well triggers
// replan to the alternate source -> idle windows only occur with low
// needs -> zero stuck idle windows.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn no_stuck_idle_windows_with_elevated_needs() {
    let observation = run_survival_contested();
    assert_no_stuck_idle_windows(
        observation
            .contract
            .max_idle_window_ticks_with_elevated_need,
        observation.contract.elevated_need_floor.value(),
        "survival contested",
        &observation.stuck_idle_windows,
    );
}

#[test]
fn per_need_critical_run_limit_override_beats_default_for_dirtiness_only() {
    let thresholds = DriveThresholds::default();
    let runs = SurvivalNeedRunTracker {
        hunger_max: 10,
        dirtiness_max: 500,
        ..SurvivalNeedRunTracker::default()
    };

    assert_authored_critical_runs_with_overrides(
        100,
        SurvivalCriticalRunLimitOverrides {
            dirtiness: Some(600),
            ..SurvivalCriticalRunLimitOverrides::default()
        },
        "Agent A",
        &thresholds,
        &runs,
    );
}
