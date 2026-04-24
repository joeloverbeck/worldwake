//! Golden tests for the survival justice roadmap row.

mod golden_harness;

use std::collections::BTreeSet;
use std::path::PathBuf;

use golden_harness::*;
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{DriveThresholds, EntityId, Tick};
use worldwake_sim::ActionTraceKind;

const SURVIVAL_TICKS: u32 = 1440;

#[derive(Clone, Debug, Eq, PartialEq)]
struct AgentSurvivalObservation {
    alive: bool,
    critical_thresholds: DriveThresholds,
    critical_need_runs: SurvivalNeedRunTracker,
    committed_actions: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct JusticeObservation {
    contract: worldwake_cli::scenario::types::SurvivalHealthContractDef,
    merchant: AgentSurvivalObservation,
    stuck_idle_windows: Vec<StuckIdleWindow>,
    stage_tick: Tick,
    theft_tick: Tick,
    office_holder_tick: Tick,
    investigate_tick: Tick,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-justice.ron")
}

fn load_survival_justice_harness() -> (GoldenHarness, ScenarioDef) {
    let path = scenario_path();
    let def = load_scenario_file(&path).expect("survival justice scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival justice scenario should spawn");
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
            worldwake_core::PerceptionSource::DirectObservation,
        );
    }
    harness.driver.enable_tracing();
    harness.enable_action_tracing();
    harness.enable_perception_tracing();
    (harness, def)
}

fn find_named_agent(h: &GoldenHarness, expected_name: &str) -> EntityId {
    h.world
        .query_name_and_agent_data()
        .find_map(|(entity, name, _)| (name.0 == expected_name).then_some(entity))
        .unwrap_or_else(|| panic!("scenario should include {expected_name}"))
}

fn find_named_entity(h: &GoldenHarness, expected_name: &str) -> EntityId {
    h.world
        .query_name()
        .find_map(|(entity, name)| (name.0 == expected_name).then_some(entity))
        .unwrap_or_else(|| panic!("scenario should include {expected_name}"))
}

fn contract_run_limit_overrides(
    limits: Option<&worldwake_cli::scenario::types::SurvivalCriticalRunLimitsDef>,
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

fn observe_idle_window(
    idle_state: &mut (Option<u32>, u16, u32),
    had_action: bool,
    needs: &worldwake_core::HomeostaticNeeds,
    tick_num: u32,
    contract: &worldwake_cli::scenario::types::SurvivalHealthContractDef,
    windows: &mut Vec<StuckIdleWindow>,
    agent_name: &str,
) {
    if had_action {
        if let Some(start_tick) = idle_state.0.take()
            && idle_state.2 >= contract.max_idle_window_ticks_with_elevated_need
            && idle_state.1 > contract.elevated_need_floor.value()
        {
            windows.push(StuckIdleWindow {
                agent_name: agent_name.to_string(),
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

fn run_survival_justice() -> JusticeObservation {
    let (mut h, def) = load_survival_justice_harness();
    let contract =
        expect_survival_health_contract(def.survival_health_contract.as_ref(), "survival justice")
            .clone();
    let merchant = find_named_agent(&h, "Merchant Sera");
    let thief = find_named_agent(&h, "Thief Rana");
    let office = find_named_entity(&h, "Market Warden");
    let _searcher = find_named_agent(&h, "Searcher Ivo");
    let merchant_thresholds = h
        .world
        .get_component_drive_thresholds(merchant)
        .copied()
        .expect("merchant should have drive thresholds");
    let mut merchant_need_runs = SurvivalNeedRunTracker::default();
    let mut merchant_idle_state: (Option<u32>, u16, u32) = (None, 0, 0);
    let mut stuck_idle_windows = Vec::new();

    let mut stage_tick = None;
    let mut theft_tick = None;
    let mut office_holder_tick = None;
    let mut investigate_tick = None;

    for tick_num in 0..SURVIVAL_TICKS {
        h.step_once();
        let tick = Tick(u64::from(tick_num));
        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");

        let merchant_needs = h
            .world
            .get_component_homeostatic_needs(merchant)
            .copied()
            .expect("merchant should always have needs");
        merchant_need_runs.observe(&merchant_needs, &merchant_thresholds);
        let merchant_had_action = action_sink
            .events_for_at(merchant, tick)
            .iter()
            .any(|event| !matches!(event.kind, ActionTraceKind::StartFailed { .. }));
        observe_idle_window(
            &mut merchant_idle_state,
            merchant_had_action,
            &merchant_needs,
            tick_num,
            &contract,
            &mut stuck_idle_windows,
            "Merchant Sera",
        );

        if office_holder_tick.is_none() && h.world.office_holder(office) == Some(merchant) {
            office_holder_tick = Some(tick);
        }

        for event in action_sink.events_for_at(merchant, tick) {
            if stage_tick.is_none()
                && event.action_name == "stage_stock_for_sale"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
            {
                stage_tick = Some(tick);
            }
            if investigate_tick.is_none()
                && event.action_name == "investigate"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
            {
                investigate_tick = Some(tick);
            }
        }

        if theft_tick.is_none()
            && action_sink.events_for_at(thief, tick).iter().any(|event| {
                event.action_name == "steal"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            })
        {
            theft_tick = Some(tick);
        }
    }

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let merchant_actions = action_sink
        .events_for(merchant)
        .iter()
        .filter(|event| matches!(event.kind, ActionTraceKind::Committed { .. }))
        .map(|event| event.action_name.clone())
        .collect::<BTreeSet<_>>();
    let merchant_violation_memory = h.world.get_component_violation_memory(merchant).cloned();
    let merchant_social_observations = h
        .world
        .get_component_agent_belief_store(merchant)
        .map(|store| store.iter_social_observations().collect::<Vec<_>>())
        .unwrap_or_default();

    JusticeObservation {
        contract,
        merchant: AgentSurvivalObservation {
            alive: h.world.is_alive(merchant),
            critical_thresholds: merchant_thresholds,
            critical_need_runs: merchant_need_runs,
            committed_actions: merchant_actions.clone(),
        },
        stuck_idle_windows,
        stage_tick: stage_tick.unwrap_or_else(|| {
            panic!("merchant should commit stage_stock_for_sale in survival justice")
        }),
        theft_tick: theft_tick.unwrap_or_else(|| {
            panic!("thief should commit steal in survival justice")
        }),
        office_holder_tick: office_holder_tick.unwrap_or_else(|| {
            panic!("merchant should become Market Warden holder in survival justice")
        }),
        investigate_tick: investigate_tick.unwrap_or_else(|| {
            panic!(
                "merchant should commit investigate in survival justice; committed_actions={merchant_actions:?}; violation_memory={merchant_violation_memory:?}; social_observations={merchant_social_observations:?}"
            )
        }),
    }
}

// ---------------------------------------------------------------------------
// Scenario 177: Survival Justice Proves Theft Investigation Substrate
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Trade, Perception, Offices, Investigation
// GoalKinds: StealItem, InvestigateViolation
// ActionDomains: Social, Trade, Needs
// Places: Market Square
// Principles: 4, 6, 7, 8, 12, 20, 21
//
// Setup: Run the authored survival justice scenario for 1440 ticks. `Merchant
//   Sera` begins as lawful `Market Warden` holder at `Market Square`, stages
//   owned apples for sale, and then responds to local stock disappearance with
//   the live investigation action under survival pressure.
//
// Proves: the tracked merchant satisfies the authored survival-health
//   contract; the merchant starts from a lawful office-holder substrate, stages
//   sale stock, and the live investigation action remains active in the same
//   authored survival run where theft also occurs.
//   The scenario intentionally stops short of claiming that accusation,
//   punishment, or search/report_found are already truthful retained seams
//   here.
//
// Chain: lawful office-holder substrate -> staged apples become stealable ->
//   the scenario reaches a real `steal` commit and a real `investigate`
//   commit under the same survival envelope, while the exact theft-to-case
//   binding remains a downstream blocked seam.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_justice_proves_theft_investigation_substrate() {
    let observation = run_survival_justice();
    let run_limit_overrides =
        contract_run_limit_overrides(observation.contract.critical_run_limits.as_ref());

    assert!(
        observation.merchant.alive,
        "Merchant Sera should remain alive for the full {SURVIVAL_TICKS}-tick scenario; observation={observation:?}"
    );
    assert_authored_critical_runs_with_overrides(
        observation.contract.max_authored_critical_run_ticks,
        run_limit_overrides,
        "Merchant Sera",
        &observation.merchant.critical_thresholds,
        &observation.merchant.critical_need_runs,
    );
    assert_required_self_care_families(
        &observation.contract.required_self_care_families,
        "Merchant Sera",
        &observation.merchant.committed_actions,
        "survival-justice",
    );
    assert_no_stuck_idle_windows(
        observation
            .contract
            .max_idle_window_ticks_with_elevated_need,
        observation.contract.elevated_need_floor.value(),
        "survival-justice",
        &observation.stuck_idle_windows,
    );

    assert!(
        observation.office_holder_tick <= observation.stage_tick,
        "merchant should begin from a lawful office-holder substrate before staging stock; observation={observation:?}"
    );
    assert!(
        observation.stage_tick <= observation.theft_tick,
        "merchant should stage sale stock before the theft commit; observation={observation:?}"
    );
    assert!(
        observation.stage_tick <= observation.theft_tick,
        "theft should still happen against staged sale stock; observation={observation:?}"
    );
}

#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_justice_replays_deterministically() {
    assert_eq!(run_survival_justice(), run_survival_justice());
}
