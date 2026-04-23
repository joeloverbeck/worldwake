//! Golden tests for the survival items-decay roadmap row.

mod golden_harness;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use golden_harness::*;
use worldwake_ai::DecisionOutcome;
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{
    CommodityKind, DriveThresholds, EntityId, EventTag, EventView, GoalKey, GoalKind,
    PerceptionSource, Quantity, Tick,
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
struct DecayObservation {
    tracked_waste_lot: EntityId,
    selected_disposal_tick: Tick,
    drop_commit_tick: Tick,
    dropped_ground_tick: Tick,
    decay_tick: Tick,
    first_trade_tick: Tick,
    first_buyer_eat_tick: Tick,
    buyer_apples_after_trade: Quantity,
    buyer_coin_after_trade: Quantity,
    merchant_coin_after_trade: Quantity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SurvivalItemsDecayObservation {
    contract: worldwake_cli::scenario::types::SurvivalHealthContractDef,
    agents: BTreeMap<String, AgentSurvivalObservation>,
    stuck_idle_windows: Vec<StuckIdleWindow>,
    decay: DecayObservation,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-items-decay.ron")
}

fn load_survival_items_decay_harness() -> (GoldenHarness, ScenarioDef) {
    let path = scenario_path();
    let def = load_scenario_file(&path).expect("survival items-decay scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival items-decay scenario should spawn");
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

fn named_agents(h: &GoldenHarness) -> BTreeMap<String, EntityId> {
    h.world
        .query_name_and_agent_data()
        .map(|(entity, name, _)| (name.0.clone(), entity))
        .collect()
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

fn tracked_merchant_waste_lot(h: &GoldenHarness, carrier: EntityId) -> EntityId {
    let mut waste_lots = h
        .world
        .possessions_of(carrier)
        .into_iter()
        .filter(|entity| {
            h.world.get_component_item_lot(*entity).is_some_and(|lot| {
                lot.commodity == CommodityKind::Waste && lot.quantity > Quantity(0)
            })
        })
        .collect::<Vec<_>>();
    waste_lots.sort();
    *waste_lots
        .first()
        .expect("merchant should start with one carried Waste lot")
}

fn run_survival_items_decay() -> SurvivalItemsDecayObservation {
    let (mut h, def) = load_survival_items_decay_harness();
    let contract = expect_survival_health_contract(
        def.survival_health_contract.as_ref(),
        "survival items decay",
    )
    .clone();
    let agents = named_agents(&h);
    let merchant = *agents
        .get("Merchant Sera")
        .expect("scenario should include Merchant Sera");
    let buyer = *agents
        .get("Buyer Nila")
        .expect("scenario should include Buyer Nila");
    let caretaker = *agents
        .get("Caretaker Oren")
        .expect("scenario should include Caretaker Oren");
    let disposal_goal = GoalKey::from(GoalKind::FreeCarryCapacity);
    let tracked_waste_lot = tracked_merchant_waste_lot(&h, caretaker);

    let critical_thresholds = agents
        .iter()
        .map(|(name, agent)| {
            (
                name.clone(),
                h.world
                    .get_component_drive_thresholds(*agent)
                    .copied()
                    .expect("survival items-decay agents should have drive thresholds"),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut critical_need_runs = agents
        .keys()
        .cloned()
        .map(|name| (name, SurvivalNeedRunTracker::default()))
        .collect::<BTreeMap<_, _>>();
    let mut idle_state: BTreeMap<String, (Option<u32>, u16, u32)> = agents
        .keys()
        .cloned()
        .map(|name| (name, (None, 0, 0)))
        .collect();
    let mut stuck_idle_windows = Vec::new();

    let mut selected_disposal_tick = None;
    let mut drop_commit_tick = None;
    let mut dropped_ground_tick = None;
    let mut decay_tick = None;
    let mut first_trade_tick = None;
    let mut first_buyer_eat_tick = None;
    let mut buyer_apples_after_trade = None;
    let mut buyer_coin_after_trade = None;
    let mut merchant_coin_after_trade = None;

    for tick_num in 0..SURVIVAL_TICKS {
        h.step_once();
        let tick = Tick(u64::from(tick_num));
        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");

        for (agent_name, agent) in &agents {
            let needs = h
                .world
                .get_component_homeostatic_needs(*agent)
                .expect("scenario agents should always have needs");
            critical_need_runs
                .get_mut(agent_name)
                .expect("run tracker should exist")
                .observe(
                    needs,
                    critical_thresholds
                        .get(agent_name)
                        .expect("threshold should exist for agent"),
                );

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

        if selected_disposal_tick.is_none()
            && let Some(trace) = h
                .driver
                .trace_sink()
                .and_then(|sink| sink.trace_at(caretaker, tick))
            && let DecisionOutcome::Planning(planning) = &trace.outcome
            && planning.selection.selected_goal() == Some(disposal_goal)
        {
            selected_disposal_tick = Some(tick);
        }

        for event in action_sink.events_for_at(caretaker, tick) {
            if event.action_name == "drop_item"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && drop_commit_tick.is_none()
            {
                drop_commit_tick = Some(tick);
            }
        }

        if dropped_ground_tick.is_none()
            && h.world.possessor_of(tracked_waste_lot).is_none()
            && h.world
                .get_component_ground_since(tracked_waste_lot)
                .is_some()
        {
            dropped_ground_tick = Some(tick);
        }

        if decay_tick.is_none()
            && h.event_log
                .events_by_tag(EventTag::ItemDecay)
                .iter()
                .any(|event_id| {
                    h.event_log.get(*event_id).is_some_and(|record| {
                        record.target_ids().contains(&tracked_waste_lot) && record.tick() == tick
                    })
                })
        {
            decay_tick = Some(tick);
        }

        for event in action_sink.events_for_at(buyer, tick) {
            if event.action_name == "trade"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && first_trade_tick.is_none()
            {
                first_trade_tick = Some(tick);
                buyer_apples_after_trade = Some(
                    h.world
                        .controlled_commodity_quantity(buyer, CommodityKind::Apple),
                );
                buyer_coin_after_trade = Some(
                    h.world
                        .controlled_commodity_quantity(buyer, CommodityKind::Coin),
                );
                merchant_coin_after_trade = Some(
                    h.world
                        .controlled_commodity_quantity(merchant, CommodityKind::Coin),
                );
            }
            if event.action_name == "eat"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && first_buyer_eat_tick.is_none()
            {
                first_buyer_eat_tick = Some(tick);
            }
        }
    }

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let merchant_actions = action_sink
        .events_for(caretaker)
        .iter()
        .map(|event| format!("{:?}: {}", event.tick, event.summary()))
        .collect::<Vec<_>>();
    let buyer_actions = action_sink
        .events_for(buyer)
        .iter()
        .map(|event| format!("{:?}: {}", event.tick, event.summary()))
        .collect::<Vec<_>>();

    let decay = DecayObservation {
        tracked_waste_lot,
        selected_disposal_tick: selected_disposal_tick.unwrap_or_else(|| {
            panic!(
                "merchant should select FreeCarryCapacity in the authored scenario; merchant_actions={merchant_actions:?}"
            )
        }),
        drop_commit_tick: drop_commit_tick.unwrap_or_else(|| {
            panic!(
                "merchant should commit drop_item for the tracked Waste lot; merchant_actions={merchant_actions:?}"
            )
        }),
        dropped_ground_tick: dropped_ground_tick.unwrap_or_else(|| {
            panic!(
                "tracked Waste lot should become a ground item after drop_item; merchant_actions={merchant_actions:?}"
            )
        }),
        decay_tick: decay_tick.unwrap_or_else(|| {
            panic!(
                "tracked Waste lot should later be archived by ItemDecay; merchant_actions={merchant_actions:?}"
            )
        }),
        first_trade_tick: first_trade_tick.unwrap_or_else(|| {
            panic!(
                "buyer should still complete a trade while the maintenance seam stays live; buyer_actions={buyer_actions:?}"
            )
        }),
        first_buyer_eat_tick: first_buyer_eat_tick.unwrap_or_else(|| {
            panic!(
                "buyer should still commit eat after the first trade; buyer_actions={buyer_actions:?}"
            )
        }),
        buyer_apples_after_trade: buyer_apples_after_trade
            .expect("trade commit should snapshot buyer apples"),
        buyer_coin_after_trade: buyer_coin_after_trade
            .expect("trade commit should snapshot buyer coin"),
        merchant_coin_after_trade: merchant_coin_after_trade
            .expect("trade commit should snapshot merchant coin"),
    };

    let agents = agents
        .into_iter()
        .map(|(name, agent)| {
            (
                name.clone(),
                AgentSurvivalObservation {
                    alive: !h.agent_is_dead(agent),
                    critical_thresholds: *critical_thresholds
                        .get(&name)
                        .expect("final thresholds should exist"),
                    critical_need_runs: critical_need_runs
                        .remove(&name)
                        .expect("final run tracker should exist"),
                    committed_actions: action_sink
                        .events_for(agent)
                        .iter()
                        .filter(|event| matches!(event.kind, ActionTraceKind::Committed { .. }))
                        .map(|event| event.action_name.clone())
                        .collect(),
                },
            )
        })
        .collect();

    assert!(
        h.world.get_component_item_lot(tracked_waste_lot).is_none(),
        "tracked Waste lot should be archived after its ItemDecay event"
    );

    SurvivalItemsDecayObservation {
        contract,
        agents,
        stuck_idle_windows,
        decay,
    }
}

// ---------------------------------------------------------------------------
// Scenario 174: Survival Item Decay Lands Roadmap Row Ten
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Trade, Transport, ItemDecay
// GoalKinds: FreeCarryCapacity, AcquireCommodity(SelfConsume), ConsumeOwnedCommodity, Drink, Wash, Sleep, Relieve
// ActionDomains: Trade, Transport, Needs
// Places: Market Square
// Principles: 4, 6, 8, 10, 11, 14, 20, 21
//
// Setup: Run the authored survival items-decay scenario for 1440 ticks.
//   `Merchant Sera` starts at `Market Square` with apples for sale, `Buyer
//   Nila` starts hungry with coin but no food, and `Caretaker Oren` starts
//   above his authored disposal threshold with one carried Waste lot plus an
//   explicit scenario `commodity_decay` map. The local market branch and the
//   maintenance branch therefore have to coexist in one survival run.
//
// Proves: the scenario satisfies the authored survival contract for the
//   merchant and caretaker, the caretaker reaches a real `FreeCarryCapacity`
//   selection and commits `drop_item`, the same tracked Waste lot becomes a
//   ground item and is later archived by `ItemDecay`, and the buyer still
//   completes a real apple trade followed by `eat` during the same run.
//
// Chain: carried Waste above disposal threshold -> selected
//   `FreeCarryCapacity` goal -> committed `drop_item` -> tracked Waste lot on
//   ground -> `ItemDecay` archives that exact lot -> local apple trade still
//   commits under the same survival loop.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_items_decay_lands_row_ten() {
    let observation = run_survival_items_decay();
    let run_limit_overrides =
        contract_run_limit_overrides(observation.contract.critical_run_limits.as_ref());

    for agent_name in ["Merchant Sera", "Caretaker Oren"] {
        let agent = observation
            .agents
            .get(agent_name)
            .expect("tracked scenario agent should exist");
        assert!(
            agent.alive,
            "{agent_name} should remain alive for the full {SURVIVAL_TICKS}-tick scenario; observation={observation:?}"
        );
        assert_authored_critical_runs_with_overrides(
            observation.contract.max_authored_critical_run_ticks,
            run_limit_overrides,
            agent_name,
            &agent.critical_thresholds,
            &agent.critical_need_runs,
        );
        assert_required_self_care_families(
            &observation.contract.required_self_care_families,
            agent_name,
            &agent.committed_actions,
            "survival-items-decay",
        );
    }

    assert_no_stuck_idle_windows(
        observation
            .contract
            .max_idle_window_ticks_with_elevated_need,
        observation.contract.elevated_need_floor.value(),
        "survival-items-decay",
        &observation
            .stuck_idle_windows
            .iter()
            .filter(|window| {
                window.agent_name == "Merchant Sera" || window.agent_name == "Caretaker Oren"
            })
            .cloned()
            .collect::<Vec<_>>(),
    );
    assert!(
        observation.decay.selected_disposal_tick <= observation.decay.drop_commit_tick,
        "drop_item should follow a real FreeCarryCapacity selection; observation={observation:?}"
    );
    assert!(
        observation.decay.drop_commit_tick <= observation.decay.dropped_ground_tick,
        "the tracked Waste lot should only become a ground lot after drop_item commits; observation={observation:?}"
    );
    assert!(
        observation.decay.dropped_ground_tick <= observation.decay.decay_tick,
        "ItemDecay should only archive the tracked Waste lot after it reaches the ground; observation={observation:?}"
    );
    assert!(
        observation.decay.buyer_apples_after_trade > Quantity(0),
        "the buyer should hold apples after the first committed trade; observation={observation:?}"
    );
    assert!(
        observation.decay.first_trade_tick <= observation.decay.first_buyer_eat_tick,
        "the buyer should only eat after a real trade commit materializes apples; observation={observation:?}"
    );
    assert!(
        observation.decay.buyer_coin_after_trade < Quantity(90),
        "the buyer should spend coin in the first committed trade; observation={observation:?}"
    );
    assert!(
        observation.decay.merchant_coin_after_trade > Quantity(0),
        "the merchant should receive coin in the first committed trade; observation={observation:?}"
    );
}

#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_items_decay_replays_deterministically() {
    assert_eq!(run_survival_items_decay(), run_survival_items_decay());
}
