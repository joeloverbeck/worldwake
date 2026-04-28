//! Golden tests for the final-integration roadmap row.

mod golden_harness;

use std::collections::BTreeSet;
use std::path::PathBuf;

use golden_harness::*;
use worldwake_cli::scenario::{
    load_scenario_file, spawn_scenario,
    types::{AgentDef, ScenarioDef},
};
use worldwake_core::{DriveThresholds, EntityId, PerceptionSource, Tick};
use worldwake_sim::ActionTraceKind;

const SURVIVAL_TICKS: u32 = 1440;

#[derive(Clone, Debug, Eq, PartialEq)]
struct EscortSurvivalObservation {
    alive: bool,
    critical_thresholds: DriveThresholds,
    critical_need_runs: SurvivalNeedRunTracker,
    committed_actions: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FinalIntegrationObservation {
    contract: worldwake_cli::scenario::types::SurvivalHealthContractDef,
    caretaker: EscortSurvivalObservation,
    stuck_idle_windows: Vec<StuckIdleWindow>,
    ward_wounded_tick: Tick,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/final-integration.ron")
}

fn load_final_integration_harness() -> (GoldenHarness, ScenarioDef) {
    let path = scenario_path();
    let def = load_scenario_file(&path).expect("final integration scenario should parse");
    assert_final_integration_full_catalog_authored(&def);
    let spawned = spawn_scenario(&def).expect("final integration scenario should spawn");
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

fn find_agent_def<'a>(def: &'a ScenarioDef, expected_name: &str) -> &'a AgentDef {
    def.agents
        .iter()
        .find(|agent| agent.name == expected_name)
        .unwrap_or_else(|| panic!("scenario should include agent {expected_name}"))
}

fn assert_positive_weight(weight: worldwake_core::Permille, label: &str) {
    assert!(weight.value() > 0, "{label} should be active");
}

fn assert_final_integration_full_catalog_authored(def: &ScenarioDef) {
    assert!(
        def.survival_health_contract.is_some(),
        "final integration should author a survival-health contract"
    );
    assert!(
        def.commodity_decay.is_some(),
        "final integration should structurally activate item decay"
    );
    assert!(
        !def.offices.is_empty(),
        "final integration should structurally activate offices"
    );
    assert!(
        !def.bandit_camps.is_empty(),
        "final integration should structurally activate bandit camps"
    );
    assert!(
        def.places.iter().any(|place| place
            .visibility_profile
            .as_ref()
            .is_some_and(|profile| profile.base_concealment.value() > 0)),
        "final integration should structurally activate place concealment"
    );

    let witness = find_agent_def(def, "Integration Witness");
    let utility = witness
        .utility_profile
        .as_ref()
        .expect("integration witness should carry utility activators");
    assert_positive_weight(utility.hunger_weight, "hunger utility");
    assert_positive_weight(utility.thirst_weight, "thirst utility");
    assert_positive_weight(utility.fatigue_weight, "fatigue utility");
    assert_positive_weight(utility.bladder_weight, "bladder utility");
    assert_positive_weight(utility.dirtiness_weight, "dirtiness utility");
    assert_positive_weight(utility.social_weight, "social utility");
    assert_positive_weight(utility.bounty_posting_weight, "bounty posting utility");
    assert_positive_weight(utility.notice_posting_weight, "notice posting utility");
    assert_positive_weight(utility.care_weight, "escort/care utility");

    let metabolism = witness
        .metabolism_profile
        .expect("integration witness should carry travel physiology");
    assert_positive_weight(
        metabolism.travel_fatigue_multiplier,
        "travel fatigue multiplier",
    );
    assert_positive_weight(
        metabolism.travel_thirst_multiplier,
        "travel thirst multiplier",
    );
    assert_positive_weight(
        metabolism.travel_bladder_multiplier,
        "travel bladder multiplier",
    );
    assert_positive_weight(
        metabolism.wilderness_relief_dirtiness_penalty,
        "wilderness relief dirtiness penalty",
    );

    let tell = witness
        .tell_profile
        .expect("integration witness should carry tell profile");
    assert!(
        tell.max_tell_candidates > 0 && tell.max_relay_chain_len > 0,
        "tell profile should be active"
    );
    assert!(
        witness.communication_profile.is_some(),
        "communication profile should be authored"
    );
    assert!(
        witness.perception_profile.is_some(),
        "perception profile should be authored"
    );
    assert!(
        witness.drive_escalation_profile.is_some(),
        "drive escalation profile should be authored"
    );
    assert!(
        witness.exploration_profile.is_some_and(|profile| {
            profile.curiosity_weight.value() > 0 && profile.max_consecutive_explorations > 0
        }),
        "need-driven exploration should be structurally active"
    );
    assert!(
        witness.diversification_profile.is_some(),
        "diversification profile should be authored"
    );
    assert!(
        witness.preference_profile.is_some(),
        "experience preference profile should be authored"
    );
    assert!(
        witness
            .known_recipes
            .as_ref()
            .is_some_and(|recipes| recipes.iter().any(|recipe| recipe == "Bake Bread")),
        "facility-backed production recipe should be authored"
    );
    assert!(
        witness.merchandise_profile.is_some()
            && witness.trade_disposition.is_some()
            && witness.commodity_valuation.is_some()
            && witness.substitute_preferences.is_some(),
        "trade and stock profiles should be authored"
    );
    assert!(
        witness.disposal_profile.is_some() && witness.contention_disposition.is_some(),
        "disposal and contention profiles should be authored"
    );
    assert!(
        witness.obligation_satiation_profile.is_some(),
        "obligation satiation profile should be authored"
    );
    assert!(
        witness.artifact_posting_profile.is_some(),
        "artifact posting profile should be authored"
    );
    assert!(
        witness.epistemic_disposition.is_some()
            && witness.theft_disposition.is_some()
            && witness.violation_disposition.is_some()
            && witness.justice_disposition.is_some(),
        "epistemic, theft, investigation, and justice profiles should be authored"
    );
    assert!(
        witness.patrol_profile.is_some()
            && witness.patrol_route.is_some()
            && witness.pursuit_profile.is_some()
            && witness.combat_profile.is_some(),
        "patrol, pursuit, and combat profiles should be authored"
    );
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

fn run_final_integration() -> FinalIntegrationObservation {
    let (mut h, def) = load_final_integration_harness();
    let contract =
        expect_survival_health_contract(def.survival_health_contract.as_ref(), "final integration")
            .clone();
    let caretaker = find_named_agent(&h, "Caretaker Ilen");
    let ward = find_named_agent(&h, "Ward Mira");
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

    FinalIntegrationObservation {
        contract,
        caretaker: EscortSurvivalObservation {
            alive: h.world.is_alive(caretaker),
            critical_thresholds: thresholds,
            critical_need_runs: need_runs,
            committed_actions,
        },
        stuck_idle_windows,
        ward_wounded_tick: ward_wounded_tick.expect("ward should be wounded before escort"),
    }
}

// Scenario 349: Final Integration Authors The Full Coexistence Stack
// This is the row-17 final-integration landing seam. It proves the scenario
// structurally activates every gameplay feature row, then runs a 1440-tick
// survival contract while authored hostile pressure still creates a concrete
// wound in the same full-stack world.
#[test]
#[ignore = "scenario-backed survival golden runs in the golden-survival workflow"]
fn final_integration_proves_full_stack_coexistence() {
    let observation = run_final_integration();

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
        "final integration",
        &observation.stuck_idle_windows,
    );
    assert_required_self_care_families(
        &observation.contract.required_self_care_families,
        "Caretaker Ilen",
        &observation.caretaker.committed_actions,
        "final integration",
    );
    assert!(
        observation.ward_wounded_tick.0 < u64::from(SURVIVAL_TICKS),
        "the hostile pressure branch should occur during the survival run"
    );
}

#[test]
#[ignore = "scenario-backed survival golden runs in the golden-survival workflow"]
fn final_integration_replay_is_deterministic() {
    assert_eq!(run_final_integration(), run_final_integration());
}
