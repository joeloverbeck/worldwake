//! Golden tests for the survival-escort roadmap row.

mod golden_harness;

use std::collections::BTreeSet;
use std::path::PathBuf;

use golden_harness::*;
use worldwake_ai::{DecisionOutcome, GoalKind};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{DriveThresholds, EntityId, GoalKey, PerceptionSource, Tick};
use worldwake_sim::{ActionTraceDetail, ActionTraceKind};

const SURVIVAL_TICKS: u32 = 1440;

#[derive(Clone, Debug, Eq, PartialEq)]
struct EscortSurvivalObservation {
    alive: bool,
    critical_thresholds: DriveThresholds,
    critical_need_runs: SurvivalNeedRunTracker,
    committed_actions: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SurvivalEscortObservation {
    contract: worldwake_cli::scenario::types::SurvivalHealthContractDef,
    caretaker: EscortSurvivalObservation,
    stuck_idle_windows: Vec<StuckIdleWindow>,
    ward_wounded_tick: Tick,
    escort_selected_tick: Tick,
    escort_started_tick: Tick,
    escort_committed_tick: Tick,
    caretaker_place_at_escort_commit: EntityId,
    ward_place_at_escort_commit: EntityId,
    clinic: EntityId,
    care_queue_installed: bool,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-escort.ron")
}

fn load_survival_escort_harness() -> (GoldenHarness, ScenarioDef) {
    let path = scenario_path();
    let def = load_scenario_file(&path).expect("survival escort scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival escort scenario should spawn");
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
                agent_name: "Caretaker Ilen".to_string(),
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

fn run_survival_escort() -> SurvivalEscortObservation {
    let (mut h, def) = load_survival_escort_harness();
    let contract =
        expect_survival_health_contract(def.survival_health_contract.as_ref(), "survival escort")
            .clone();
    let caretaker = find_named_agent(&h, "Caretaker Ilen");
    let ward = find_named_agent(&h, "Ward Mira");
    let clinic = find_named_place(&h, "Village Clinic");
    let thresholds = h
        .world
        .get_component_drive_thresholds(caretaker)
        .copied()
        .expect("caretaker should have drive thresholds");
    let mut need_runs = SurvivalNeedRunTracker::default();
    let mut stuck_idle_windows = Vec::new();
    let mut idle_state: (Option<u32>, u16, u32) = (None, 0, 0);
    let mut committed_actions = BTreeSet::new();
    let mut ward_wounded_tick = None;
    let mut escort_selected_tick = None;
    let mut escort_started_tick = None;
    let mut escort_committed_tick = None;
    let mut caretaker_place_at_escort_commit = None;
    let mut ward_place_at_escort_commit = None;

    for tick_num in 0..SURVIVAL_TICKS {
        h.step_once();
        let tick = Tick(u64::from(tick_num));
        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");

        if ward_wounded_tick.is_none()
            && h.world
                .get_component_wound_list(ward)
                .is_some_and(|wounds| !wounds.wounds.is_empty())
        {
            ward_wounded_tick = Some(tick);
        }

        for event in action_sink.events_for_at(caretaker, tick) {
            if matches!(event.kind, ActionTraceKind::Committed { .. }) {
                committed_actions.insert(event.action_name.clone());
            }
            if event.action_name == "escort_to_safety"
                && matches!(
                    event.detail,
                    Some(ActionTraceDetail::EscortToSafety {
                        subject,
                        destination,
                    }) if subject == ward && destination == clinic
                )
            {
                match event.kind {
                    ActionTraceKind::Started { .. } => {
                        escort_started_tick.get_or_insert(tick);
                    }
                    ActionTraceKind::Committed { .. } => {
                        escort_committed_tick.get_or_insert(tick);
                        caretaker_place_at_escort_commit.get_or_insert_with(|| {
                            h.world
                                .effective_place(caretaker)
                                .expect("caretaker should be placed at escort commit")
                        });
                        ward_place_at_escort_commit.get_or_insert_with(|| {
                            h.world
                                .effective_place(ward)
                                .expect("ward should be placed at escort commit")
                        });
                    }
                    ActionTraceKind::Aborted { .. } | ActionTraceKind::StartFailed { .. } => {}
                }
            }
        }

        if let Some(trace_sink) = h.driver.trace_sink() {
            for trace in trace_sink
                .traces_for(caretaker)
                .into_iter()
                .filter(|trace| trace.tick == tick)
            {
                if let DecisionOutcome::Planning(planning) = &trace.outcome {
                    let goal = GoalKey::from(GoalKind::EscortToSafety {
                        subject: ward,
                        destination: clinic,
                    });
                    if planning.selection.selected_goal_is(goal) {
                        escort_selected_tick.get_or_insert(tick);
                    }
                }
            }
        }

        let needs = h
            .world
            .get_component_homeostatic_needs(caretaker)
            .copied()
            .expect("caretaker should always have needs");
        need_runs.observe(&needs, &thresholds);
        let caretaker_had_action =
            golden_harness::agent_has_non_failed_action_or_active(&h, action_sink, caretaker, tick);
        observe_idle_window(
            &mut idle_state,
            caretaker_had_action,
            &needs,
            tick_num,
            &contract,
            &mut stuck_idle_windows,
        );
    }

    SurvivalEscortObservation {
        contract,
        caretaker: EscortSurvivalObservation {
            alive: h.world.is_alive(caretaker),
            critical_thresholds: thresholds,
            critical_need_runs: need_runs,
            committed_actions,
        },
        stuck_idle_windows,
        ward_wounded_tick: ward_wounded_tick.expect("ward should be wounded before escort"),
        escort_selected_tick: escort_selected_tick
            .expect("caretaker should select EscortToSafety for the wounded ward"),
        escort_started_tick: escort_started_tick
            .expect("caretaker should start escort_to_safety for the ward"),
        escort_committed_tick: escort_committed_tick
            .expect("caretaker should commit escort_to_safety for the ward"),
        caretaker_place_at_escort_commit: caretaker_place_at_escort_commit
            .expect("caretaker should have a place at escort commit"),
        ward_place_at_escort_commit: ward_place_at_escort_commit
            .expect("ward should have a place at escort commit"),
        clinic,
        care_queue_installed: h.world.get_component_contention_queue(ward).is_some(),
    }
}

// Scenario 348: Survival Escort Lands Coordinated Care Travel
// This is the row-16 survival-escort landing seam. It proves the 1440-tick
// survival contract, wounded-ward observation under hostile pressure,
// EscortToSafety selection, committed coordinated travel, and the care handoff
// queue installed for the ward at the destination.
#[test]
#[ignore = "scenario-backed survival golden runs in the golden-survival workflow"]
fn survival_escort_proves_coordinated_care_travel() {
    let observation = run_survival_escort();

    assert!(observation.caretaker.alive, "Caretaker Ilen should survive");
    assert_authored_critical_runs(
        observation.contract.max_authored_critical_run_ticks,
        "Caretaker Ilen",
        &observation.caretaker.critical_thresholds,
        &observation.caretaker.critical_need_runs,
    );
    assert_no_stuck_idle_windows(
        observation
            .contract
            .max_idle_window_ticks_with_elevated_need,
        observation.contract.elevated_need_floor.value(),
        "survival escort",
        &observation.stuck_idle_windows,
    );
    assert_required_self_care_families(
        &observation.contract.required_self_care_families,
        "Caretaker Ilen",
        &observation.caretaker.committed_actions,
        "survival escort",
    );
    assert!(
        observation.ward_wounded_tick <= observation.escort_selected_tick,
        "escort selection should follow the ward becoming wounded"
    );
    assert!(
        observation.escort_selected_tick <= observation.escort_started_tick,
        "selection should precede escort start"
    );
    assert!(
        observation.escort_started_tick <= observation.escort_committed_tick,
        "escort start should precede commit"
    );
    assert_eq!(
        observation.caretaker_place_at_escort_commit, observation.clinic,
        "caretaker should be at the authored clinic destination when escort commits"
    );
    assert_eq!(
        observation.ward_place_at_escort_commit, observation.clinic,
        "ward should be moved to the authored clinic destination when escort commits"
    );
    assert!(
        observation.care_queue_installed,
        "escort commit should install a care contention queue for the ward"
    );
}

#[test]
#[ignore = "scenario-backed survival golden runs in the golden-survival workflow"]
fn survival_escort_replay_is_deterministic() {
    assert_eq!(run_survival_escort(), run_survival_escort());
}
