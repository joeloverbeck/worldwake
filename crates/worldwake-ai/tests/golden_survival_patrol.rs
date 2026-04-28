//! Golden tests for the partial survival patrol roadmap row.

mod golden_harness;

use std::collections::BTreeSet;
use std::path::PathBuf;

use golden_harness::*;
use worldwake_ai::{DecisionOutcome, GoalKind};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{DriveThresholds, EntityId, PerceptionSource, Tick};
use worldwake_sim::ActionTraceKind;

const SURVIVAL_TICKS: u32 = 1440;

#[derive(Clone, Debug, Eq, PartialEq)]
struct PatrolSurvivalObservation {
    alive: bool,
    critical_thresholds: DriveThresholds,
    critical_need_runs: SurvivalNeedRunTracker,
    committed_actions: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SurvivalPatrolObservation {
    contract: worldwake_cli::scenario::types::SurvivalHealthContractDef,
    guard: PatrolSurvivalObservation,
    stuck_idle_windows: Vec<StuckIdleWindow>,
    first_watch_post_patrol_tick: Tick,
    first_market_road_patrol_tick: Tick,
    first_remote_pursuit_candidate_tick: Tick,
    remote_pursuit_route_cost: u32,
    remote_pursuit_selected: bool,
    remote_pursuit_plan_travel_then_attack: bool,
    attack_committed: bool,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-patrol.ron")
}

fn load_survival_patrol_harness() -> (GoldenHarness, ScenarioDef) {
    let path = scenario_path();
    let def = load_scenario_file(&path).expect("survival patrol scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival patrol scenario should spawn");
    let mut harness = GoldenHarness::from_simulation_state(&spawned.state);
    let agents = harness
        .world
        .query_name_and_agent_data()
        .map(|(agent, _, _)| agent)
        .collect::<Vec<_>>();
    for agent in agents {
        let place = harness
            .world
            .effective_place(agent)
            .expect("scenario agents should have a starting place");
        let local_entities = harness
            .world
            .entities()
            .filter(|entity| harness.world.effective_place(*entity) == Some(place))
            .filter(|entity| *entity != agent)
            .collect::<Vec<_>>();
        seed_actor_beliefs(
            &mut harness.world,
            &mut harness.event_log,
            agent,
            &local_entities,
            Tick(0),
            PerceptionSource::DirectObservation,
        );
    }
    harness.driver.enable_tracing();
    harness.enable_action_tracing();
    (harness, def)
}

fn find_named_agent(h: &GoldenHarness, expected_name: &str) -> EntityId {
    h.world
        .query_name_and_agent_data()
        .find_map(|(entity, name, _)| (name.0 == expected_name).then_some(entity))
        .unwrap_or_else(|| panic!("scenario should include {expected_name}"))
}

fn find_named_place(h: &GoldenHarness, expected_name: &str) -> EntityId {
    h.world
        .topology()
        .place_ids()
        .find(|place| {
            h.world
                .topology()
                .place(*place)
                .is_some_and(|data| data.name == expected_name)
        })
        .unwrap_or_else(|| panic!("scenario should include place {expected_name}"))
}

fn observe_idle_window(
    idle_state: &mut (Option<u32>, u16, u32),
    had_action: bool,
    needs: &worldwake_core::HomeostaticNeeds,
    tick_num: u32,
    contract: &worldwake_cli::scenario::types::SurvivalHealthContractDef,
    windows: &mut Vec<StuckIdleWindow>,
) {
    if had_action {
        if let Some(start_tick) = idle_state.0.take()
            && idle_state.2 >= contract.max_idle_window_ticks_with_elevated_need
            && idle_state.1 > contract.elevated_need_floor.value()
        {
            windows.push(StuckIdleWindow {
                agent_name: "Guard Mira".to_string(),
                start_tick,
                end_tick: tick_num.saturating_sub(1),
                max_need_at_start: idle_state.1,
            });
        }
        idle_state.2 = 0;
    } else {
        if idle_state.0.is_none() {
            idle_state.0 = Some(tick_num);
            idle_state.1 = max_need_value(needs);
        }
        idle_state.2 += 1;
    }
}

fn run_survival_patrol() -> SurvivalPatrolObservation {
    let (mut h, def) = load_survival_patrol_harness();
    let contract =
        expect_survival_health_contract(def.survival_health_contract.as_ref(), "survival patrol")
            .clone();
    let guard = find_named_agent(&h, "Guard Mira");
    let fugitive = find_named_agent(&h, "Fugitive Vale");
    let watch_post = find_named_place(&h, "Watch Post");
    let market_road = find_named_place(&h, "Market Road");
    let old_mill = find_named_place(&h, "Old Mill");
    let thresholds = h
        .world
        .get_component_drive_thresholds(guard)
        .copied()
        .expect("guard should have drive thresholds");
    let mut need_runs = SurvivalNeedRunTracker::default();
    let mut stuck_idle_windows = Vec::new();
    let mut idle_state: (Option<u32>, u16, u32) = (None, 0, 0);
    let mut committed_actions = BTreeSet::new();
    let mut first_watch_post_patrol_tick = None;
    let mut first_market_road_patrol_tick = None;
    let mut first_remote_pursuit_candidate_tick = None;
    let mut remote_pursuit_route_cost = None;
    let mut remote_pursuit_selected = false;
    let mut remote_pursuit_plan_travel_then_attack = false;
    let mut attack_committed = false;

    for tick_num in 0..SURVIVAL_TICKS {
        h.step_once();
        let tick = Tick(u64::from(tick_num));
        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");

        for event in action_sink.events_for_at(guard, tick) {
            if matches!(event.kind, ActionTraceKind::Committed { .. }) {
                committed_actions.insert(event.action_name.clone());
                if event.action_name == "patrol"
                    && h.world.effective_place(guard) == Some(watch_post)
                {
                    first_watch_post_patrol_tick.get_or_insert(tick);
                }
                if event.action_name == "patrol"
                    && h.world.effective_place(guard) == Some(market_road)
                {
                    first_market_road_patrol_tick.get_or_insert(tick);
                }
                if event.action_name == "attack" {
                    attack_committed = true;
                }
            }
        }

        if let Some(trace_sink) = h.driver.trace_sink() {
            for trace in trace_sink
                .traces_for(guard)
                .into_iter()
                .filter(|trace| trace.tick == tick)
            {
                if let DecisionOutcome::Planning(planning) = &trace.outcome {
                    let goal =
                        worldwake_ai::GoalKey::from(GoalKind::EngageHostile { target: fugitive });
                    for evidence in planning.candidates.evidence_for_goal(goal) {
                        let Some(pursuit) = evidence.pursuit else {
                            continue;
                        };
                        if pursuit.omission.is_none()
                            && pursuit.believed_place == Some(old_mill)
                            && let Some(route_cost) = pursuit.route_cost
                        {
                            first_remote_pursuit_candidate_tick.get_or_insert(tick);
                            remote_pursuit_route_cost.get_or_insert(route_cost);
                        }
                    }
                    if planning.selection.selected_goal_is(goal) {
                        remote_pursuit_selected = true;
                        if planning
                            .selection
                            .selected_plan
                            .as_ref()
                            .is_some_and(|plan| {
                                let travel_index = plan.steps.iter().position(|step| {
                                    step.op_kind == worldwake_ai::PlannerOpKind::Travel
                                });
                                let attack_index = plan.steps.iter().position(|step| {
                                    step.op_kind == worldwake_ai::PlannerOpKind::Attack
                                        && step.targets.contains(&fugitive)
                                });
                                travel_index
                                    .zip(attack_index)
                                    .is_some_and(|(travel, attack)| travel < attack)
                            })
                        {
                            remote_pursuit_plan_travel_then_attack = true;
                        }
                    }
                }
            }
        }

        let needs = h
            .world
            .get_component_homeostatic_needs(guard)
            .copied()
            .expect("guard should always have needs");
        need_runs.observe(&needs, &thresholds);
        let guard_had_action =
            golden_harness::agent_has_non_failed_action_or_active(&h, action_sink, guard, tick);
        observe_idle_window(
            &mut idle_state,
            guard_had_action,
            &needs,
            tick_num,
            &contract,
            &mut stuck_idle_windows,
        );
    }

    SurvivalPatrolObservation {
        contract,
        guard: PatrolSurvivalObservation {
            alive: h.world.is_alive(guard),
            critical_thresholds: thresholds,
            critical_need_runs: need_runs,
            committed_actions,
        },
        stuck_idle_windows,
        first_watch_post_patrol_tick: first_watch_post_patrol_tick
            .expect("guard should commit patrol at Watch Post"),
        first_market_road_patrol_tick: first_market_road_patrol_tick
            .expect("guard should commit patrol at Market Road"),
        first_remote_pursuit_candidate_tick: first_remote_pursuit_candidate_tick
            .expect("remote pursuit candidate should be emitted from last-seen memory"),
        remote_pursuit_route_cost: remote_pursuit_route_cost
            .expect("route cost should be recorded"),
        remote_pursuit_selected,
        remote_pursuit_plan_travel_then_attack,
        attack_committed,
    }
}

// Scenario 346: survival-patrol
// This is the row-14 survival-patrol landing seam. It proves the 1440-tick
// survival contract, scheduled patrol commits at both authored waypoints, and
// remote pursuit is selected and executed from authored hostility plus
// last-seen memory.
#[test]
#[ignore = "scenario-backed survival golden runs in the golden-survival workflow"]
fn survival_patrol_proves_patrol_and_remote_pursuit_execution() {
    let observation = run_survival_patrol();

    assert!(observation.guard.alive, "Guard Mira should survive");
    assert_authored_critical_runs(
        observation.contract.max_authored_critical_run_ticks,
        "Guard Mira",
        &observation.guard.critical_thresholds,
        &observation.guard.critical_need_runs,
    );
    assert_no_stuck_idle_windows(
        observation
            .contract
            .max_idle_window_ticks_with_elevated_need,
        observation.contract.elevated_need_floor.value(),
        "survival patrol",
        &observation.stuck_idle_windows,
    );
    assert_required_self_care_families(
        &observation.contract.required_self_care_families,
        "Guard Mira",
        &observation.guard.committed_actions,
        "survival patrol",
    );
    assert!(
        observation.first_watch_post_patrol_tick < observation.first_market_road_patrol_tick,
        "the guard should complete the starting waypoint before the remote waypoint"
    );
    assert_eq!(
        observation.remote_pursuit_route_cost, 3,
        "the emitted pursuit candidate should be grounded in the Market Road -> Old Mill route"
    );
    assert!(
        observation.first_remote_pursuit_candidate_tick > observation.first_watch_post_patrol_tick,
        "the in-range remote pursuit candidate should appear after the starting patrol waypoint"
    );
    assert!(
        observation.remote_pursuit_selected,
        "the remote pursuit candidate should be selected under the survival envelope"
    );
    assert!(
        observation.remote_pursuit_plan_travel_then_attack,
        "the selected remote pursuit plan should travel toward the last-seen target place before attacking"
    );
    assert!(
        observation.attack_committed,
        "the selected remote pursuit branch should commit its terminal attack"
    );
}

#[test]
#[ignore = "scenario-backed survival golden runs in the golden-survival workflow"]
fn survival_patrol_replay_is_deterministic() {
    assert_eq!(run_survival_patrol(), run_survival_patrol());
}
