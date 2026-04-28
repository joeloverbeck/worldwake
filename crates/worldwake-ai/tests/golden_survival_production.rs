//! Golden tests for the survival production roadmap landing.

mod golden_harness;

use std::collections::BTreeSet;
use std::path::PathBuf;

use golden_harness::*;
use worldwake_ai::DecisionOutcome;
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{
    CommodityKind, DriveThresholds, GoalKind, PerceptionSource, Tick, total_live_lot_quantity,
};
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
struct ProductionObservation {
    contract: worldwake_cli::scenario::types::SurvivalHealthContractDef,
    agent: AgentSurvivalObservation,
    stuck_idle_windows: Vec<StuckIdleWindow>,
    first_produce_tick: Tick,
    first_craft_commit_tick: Tick,
    first_bread_stock_tick: Tick,
    first_bread_consume_goal_tick: Tick,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-production.ron")
}

fn load_survival_production_harness() -> (GoldenHarness, ScenarioDef) {
    let path = scenario_path();
    let def = load_scenario_file(&path).expect("survival production scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival production scenario should spawn");
    let mut harness = GoldenHarness::from_simulation_state(&spawned.state);
    let agent = harness
        .world
        .query_name_and_agent_data()
        .find_map(|(entity, name, _)| (name.0 == "Baker Nila").then_some(entity))
        .expect("scenario should include Baker Nila");
    seed_actor_local_beliefs(
        &mut harness.world,
        &mut harness.event_log,
        agent,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    harness.driver.enable_tracing();
    harness.enable_action_tracing();
    (harness, def)
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

fn run_survival_production() -> ProductionObservation {
    let (mut h, def) = load_survival_production_harness();
    let contract = expect_survival_health_contract(
        def.survival_health_contract.as_ref(),
        "survival production",
    )
    .clone();
    let agent = h
        .world
        .query_name_and_agent_data()
        .find_map(|(entity, name, _)| (name.0 == "Baker Nila").then_some(entity))
        .expect("scenario should include Baker Nila");
    let thresholds = h
        .world
        .get_component_drive_thresholds(agent)
        .copied()
        .expect("survival production agent should have drive thresholds");
    let mut critical_need_runs = SurvivalNeedRunTracker::default();
    let mut stuck_idle_windows = Vec::new();
    let mut idle_state: (Option<u32>, u16, u32) = (None, 0, 0);

    let mut first_produce_tick = None;
    let mut first_craft_commit_tick = None;
    let mut first_bread_stock_tick = None;
    let mut first_bread_consume_goal_tick = None;

    for tick_num in 0..SURVIVAL_TICKS {
        h.step_once();
        let tick = Tick(u64::from(tick_num));
        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");

        let needs = h
            .world
            .get_component_homeostatic_needs(agent)
            .expect("survival production agent should always have needs");
        critical_need_runs.observe(needs, &thresholds);

        let had_action =
            golden_harness::agent_has_non_failed_action_or_active(&h, action_sink, agent, tick);
        let (start, max_need, count) = &mut idle_state;
        if had_action {
            if let Some(s) = start.take()
                && *count >= contract.max_idle_window_ticks_with_elevated_need
                && *max_need > contract.elevated_need_floor.value()
            {
                stuck_idle_windows.push(StuckIdleWindow {
                    agent_name: "Baker Nila".to_string(),
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

        if first_produce_tick.is_none() {
            let maybe_tick = h
                .driver
                .trace_sink()
                .expect("decision tracing should be enabled")
                .trace_at(agent, tick)
                .and_then(|trace| match &trace.outcome {
                    DecisionOutcome::Planning(planning)
                        if planning.selection.selected_goal().is_some_and(|goal| {
                            matches!(goal.kind, GoalKind::ProduceCommodity { .. })
                        }) =>
                    {
                        Some(trace.tick)
                    }
                    _ => None,
                });
            first_produce_tick = first_produce_tick.or(maybe_tick);
        }

        if first_craft_commit_tick.is_none()
            && action_sink.events_for_at(agent, tick).iter().any(|event| {
                event.action_name == "craft:Bake Bread"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            })
        {
            first_craft_commit_tick = Some(tick);
        }

        if first_bread_stock_tick.is_none()
            && total_live_lot_quantity(&h.world, CommodityKind::Bread) > 0
        {
            first_bread_stock_tick = Some(tick);
        }

        if first_bread_consume_goal_tick.is_none() {
            let maybe_tick = h
                .driver
                .trace_sink()
                .expect("decision tracing should be enabled")
                .trace_at(agent, tick)
                .and_then(|trace| match &trace.outcome {
                    DecisionOutcome::Planning(planning)
                        if planning.selection.selected_goal().is_some_and(|goal| {
                            matches!(
                                goal.kind,
                                GoalKind::ConsumeOwnedCommodity {
                                    commodity: CommodityKind::Bread,
                                }
                            )
                        }) =>
                    {
                        Some(trace.tick)
                    }
                    _ => None,
                });
            first_bread_consume_goal_tick = first_bread_consume_goal_tick.or(maybe_tick);
        }
    }

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let committed_actions = action_sink
        .events_for(agent)
        .iter()
        .filter(|event| matches!(event.kind, ActionTraceKind::Committed { .. }))
        .map(|event| event.action_name.clone())
        .collect::<BTreeSet<_>>();
    let trace_summaries = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .traces_for(agent)
        .into_iter()
        .map(|trace| format!("{:?}: {}", trace.tick, trace.outcome.summary()))
        .collect::<Vec<_>>();

    ProductionObservation {
        contract,
        agent: AgentSurvivalObservation {
            alive: !h.agent_is_dead(agent),
            critical_thresholds: thresholds,
            critical_need_runs,
            committed_actions: committed_actions.clone(),
        },
        stuck_idle_windows,
        first_produce_tick: first_produce_tick.unwrap_or_else(|| {
            panic!(
                "scenario should select a ProduceCommodity branch under survival pressure; committed_actions={committed_actions:?}; traces={trace_summaries:?}"
            )
        }),
        first_craft_commit_tick: first_craft_commit_tick.unwrap_or_else(|| {
            panic!(
                "scenario should commit craft:Bake Bread after selecting production; committed_actions={committed_actions:?}; traces={trace_summaries:?}"
            )
        }),
        first_bread_stock_tick: first_bread_stock_tick.unwrap_or_else(|| {
            panic!(
                "scenario should materialize Bread after the craft commit; committed_actions={committed_actions:?}; traces={trace_summaries:?}"
            )
        }),
        first_bread_consume_goal_tick: first_bread_consume_goal_tick.unwrap_or_else(|| {
            panic!(
                "scenario should later select ConsumeOwnedCommodity(Bread) rather than surviving forever on a non-production branch; committed_actions={committed_actions:?}; traces={trace_summaries:?}"
            )
        }),
    }
}

// ---------------------------------------------------------------------------
// Scenario 172: Survival Production Lands Roadmap Row Eight
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Production
// GoalKinds: ProduceCommodity, ConsumeOwnedCommodity, Drink, Wash, Sleep, Relieve
// ActionDomains: Production, Needs
// Places: Bakery Yard
// Principles: 6, 8, 14, 20
//
// Setup: Run the authored survival production scenario for 1440 ticks. The
// tracked baker starts with only three lawful supports at one place: a mill, a
// well, and a stocked Firewood pile. No orchard, grain field, trade, or social
// fallback exists; the only authored food path is `Bake Bread` at the mill.
//
// Proves: the agent satisfies the authored survival contract, selects a real
// `ProduceCommodity` branch, commits `craft:Bake Bread`, materializes Bread in
// authoritative world state, and later selects Bread consumption rather than
// surviving through a rival food source.
//
// Chain: local mill + possessed Firewood belief -> selected ProduceCommodity
// plan -> committed `craft:Bake Bread` -> Bread lot appears in world state ->
// later ConsumeOwnedCommodity(Bread) selection under the same survival loop.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_production_lands_row_eight() {
    let observation = run_survival_production();
    let run_limit_overrides =
        contract_run_limit_overrides(observation.contract.critical_run_limits.as_ref());

    assert!(
        observation.agent.alive,
        "Baker Nila should remain alive for the full {SURVIVAL_TICKS}-tick scenario; observation={observation:?}"
    );
    assert_authored_critical_runs_with_overrides(
        observation.contract.max_authored_critical_run_ticks,
        run_limit_overrides,
        "Baker Nila",
        &observation.agent.critical_thresholds,
        &observation.agent.critical_need_runs,
    );
    assert_required_self_care_families(
        &observation.contract.required_self_care_families,
        "Baker Nila",
        &observation.agent.committed_actions,
        "survival-production",
    );
    assert_no_stuck_idle_windows(
        observation
            .contract
            .max_idle_window_ticks_with_elevated_need,
        observation.contract.elevated_need_floor.value(),
        "survival-production",
        &observation.stuck_idle_windows,
    );
    assert!(
        observation.first_produce_tick <= observation.first_craft_commit_tick,
        "craft commit should follow a real ProduceCommodity selection; observation={observation:?}"
    );
    assert!(
        observation.first_craft_commit_tick <= observation.first_bread_stock_tick,
        "Bread should only appear after the craft commit; observation={observation:?}"
    );
    assert!(
        observation.first_bread_stock_tick <= observation.first_bread_consume_goal_tick,
        "the later Bread-consumption plan should only appear after Bread was actually produced; observation={observation:?}"
    );
}

#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_production_replays_deterministically() {
    assert_eq!(run_survival_production(), run_survival_production());
}
