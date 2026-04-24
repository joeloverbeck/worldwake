//! Golden tests for the survival-combat roadmap row.

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
struct CombatSurvivalObservation {
    alive: bool,
    critical_thresholds: DriveThresholds,
    critical_need_runs: SurvivalNeedRunTracker,
    committed_actions: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SurvivalCombatObservation {
    contract: worldwake_cli::scenario::types::SurvivalHealthContractDef,
    guard: CombatSurvivalObservation,
    stuck_idle_windows: Vec<StuckIdleWindow>,
    engage_hostile_selected: bool,
    attack_committed_tick: Tick,
    raider_dead_tick: Tick,
    camp_empty_tick: Tick,
    camp_abandoned_tick: Tick,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-combat.ron")
}

fn load_survival_combat_harness() -> (GoldenHarness, ScenarioDef) {
    let path = scenario_path();
    let def = load_scenario_file(&path).expect("survival combat scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival combat scenario should spawn");
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
                agent_name: "Sentinel Rowan".to_string(),
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

fn run_survival_combat() -> SurvivalCombatObservation {
    let (mut h, def) = load_survival_combat_harness();
    let contract =
        expect_survival_health_contract(def.survival_health_contract.as_ref(), "survival combat")
            .clone();
    let guard = find_named_agent(&h, "Sentinel Rowan");
    let raider = find_named_agent(&h, "Raider Voss");
    let raider_camp = find_named_place(&h, "Raider Camp");
    let thresholds = h
        .world
        .get_component_drive_thresholds(guard)
        .copied()
        .expect("guard should have drive thresholds");
    let mut need_runs = SurvivalNeedRunTracker::default();
    let mut stuck_idle_windows = Vec::new();
    let mut idle_state: (Option<u32>, u16, u32) = (None, 0, 0);
    let mut committed_actions = BTreeSet::new();
    let mut engage_hostile_selected = false;
    let mut attack_committed_tick = None;
    let mut raider_dead_tick = None;
    let mut camp_empty_tick = None;
    let mut camp_abandoned_tick = None;

    assert!(
        h.world.get_component_bandit_camp(raider_camp).is_some(),
        "scenario should author an active bandit camp at Raider Camp"
    );

    for tick_num in 0..SURVIVAL_TICKS {
        h.step_once();
        let tick = Tick(u64::from(tick_num));
        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");

        for event in action_sink.events_for_at(guard, tick) {
            if matches!(event.kind, ActionTraceKind::Committed { .. }) {
                committed_actions.insert(event.action_name.clone());
                if event.action_name == "attack" {
                    attack_committed_tick.get_or_insert(tick);
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
                        worldwake_ai::GoalKey::from(GoalKind::EngageHostile { target: raider });
                    if planning.selection.selected_goal_is(goal) {
                        engage_hostile_selected = true;
                    }
                }
            }
        }

        if h.world.get_component_dead_at(raider).is_some() {
            raider_dead_tick.get_or_insert(tick);
        }
        match h.world.get_component_bandit_camp(raider_camp) {
            Some(camp) if camp.empty_since_tick.is_some() => {
                camp_empty_tick.get_or_insert(tick);
            }
            None => {
                camp_abandoned_tick.get_or_insert(tick);
            }
            Some(_) => {}
        }

        let needs = h
            .world
            .get_component_homeostatic_needs(guard)
            .copied()
            .expect("guard should always have needs");
        need_runs.observe(&needs, &thresholds);
        let guard_had_action = action_sink
            .events_for_at(guard, tick)
            .iter()
            .any(|event| !matches!(event.kind, ActionTraceKind::StartFailed { .. }));
        observe_idle_window(
            &mut idle_state,
            guard_had_action,
            &needs,
            tick_num,
            &contract,
            &mut stuck_idle_windows,
        );
    }

    SurvivalCombatObservation {
        contract,
        guard: CombatSurvivalObservation {
            alive: h.world.is_alive(guard),
            critical_thresholds: thresholds,
            critical_need_runs: need_runs,
            committed_actions,
        },
        stuck_idle_windows,
        engage_hostile_selected,
        attack_committed_tick: attack_committed_tick
            .expect("guard should commit an attack against the camp member"),
        raider_dead_tick: raider_dead_tick.expect("raider should die from combat wounds"),
        camp_empty_tick: camp_empty_tick.expect("camp should record when it becomes empty"),
        camp_abandoned_tick: camp_abandoned_tick
            .expect("empty camp should be abandoned after faction grace period"),
    }
}

// Scenario 347: survival-combat
// This is the row-15 survival-combat landing seam. It proves the 1440-tick
// survival contract, same-place hostile combat selection and execution, and the
// authored bandit camp's abandonment after combat removes its last living member.
#[test]
#[ignore = "scenario-backed survival golden runs in the golden-survival workflow"]
fn survival_combat_proves_combat_and_bandit_camp_abandonment() {
    let observation = run_survival_combat();

    assert!(observation.guard.alive, "Sentinel Rowan should survive");
    assert_authored_critical_runs(
        observation.contract.max_authored_critical_run_ticks,
        "Sentinel Rowan",
        &observation.guard.critical_thresholds,
        &observation.guard.critical_need_runs,
    );
    assert_no_stuck_idle_windows(
        observation
            .contract
            .max_idle_window_ticks_with_elevated_need,
        observation.contract.elevated_need_floor.value(),
        "survival combat",
        &observation.stuck_idle_windows,
    );
    assert_required_self_care_families(
        &observation.contract.required_self_care_families,
        "Sentinel Rowan",
        &observation.guard.committed_actions,
        "survival combat",
    );
    assert!(
        observation.engage_hostile_selected,
        "the guard should select the same-place hostile combat goal"
    );
    assert!(
        observation.attack_committed_tick <= observation.raider_dead_tick,
        "death should be the downstream consequence of the committed attack"
    );
    assert!(
        observation.raider_dead_tick <= observation.camp_empty_tick,
        "camp emptiness should follow the last living member's death"
    );
    assert!(
        observation.camp_empty_tick < observation.camp_abandoned_tick,
        "camp abandonment should follow the authored grace-period empty marker"
    );
}

#[test]
#[ignore = "scenario-backed survival golden runs in the golden-survival workflow"]
fn survival_combat_replay_is_deterministic() {
    assert_eq!(run_survival_combat(), run_survival_combat());
}
