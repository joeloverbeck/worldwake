//! Golden tests for the survival trade roadmap row.

mod golden_harness;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use golden_harness::*;
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{CommodityKind, DriveThresholds, EntityId, PerceptionSource, Quantity, Tick};
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
struct TradeObservation {
    initial_buyer_coin: Quantity,
    listing_tick: Tick,
    stage_tick: Tick,
    first_trade_tick: Tick,
    first_eat_tick: Tick,
    successful_trade_count: u32,
    buyer_bread_after_trade: Quantity,
    buyer_coin_after_trade: Quantity,
    merchant_coin_after_trade: Quantity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SurvivalTradeObservation {
    contract: worldwake_cli::scenario::types::SurvivalHealthContractDef,
    agents: BTreeMap<String, AgentSurvivalObservation>,
    stuck_idle_windows: Vec<StuckIdleWindow>,
    trade: TradeObservation,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-trade.ron")
}

fn load_survival_trade_harness() -> (GoldenHarness, ScenarioDef) {
    let path = scenario_path();
    let def = load_scenario_file(&path).expect("survival trade scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival trade scenario should spawn");
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

fn merchant_has_listed_bread(h: &GoldenHarness, merchant: EntityId) -> bool {
    h.world.entities().any(|entity| {
        h.world
            .get_component_item_lot(entity)
            .is_some_and(|lot| lot.commodity == CommodityKind::Bread && lot.quantity > Quantity(0))
            && h.world.get_component_sale_listing(entity).is_some()
            && h.world.effective_place(entity) == h.world.effective_place(merchant)
    })
}

fn run_survival_trade() -> SurvivalTradeObservation {
    let (mut h, def) = load_survival_trade_harness();
    let contract =
        expect_survival_health_contract(def.survival_health_contract.as_ref(), "survival trade")
            .clone();
    let agents = named_agents(&h);
    let merchant = *agents
        .get("Merchant Sera")
        .expect("scenario should include Merchant Sera");
    let buyer = *agents
        .get("Buyer Nila")
        .expect("scenario should include Buyer Nila");
    let critical_thresholds = agents
        .iter()
        .map(|(name, agent)| {
            (
                name.clone(),
                h.world
                    .get_component_drive_thresholds(*agent)
                    .copied()
                    .expect("survival trade agents should have drive thresholds"),
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

    let mut listing_tick = None;
    let mut stage_tick = None;
    let mut first_trade_tick = None;
    let mut first_eat_tick = None;
    let mut successful_trade_count = 0_u32;
    let mut buyer_bread_after_trade = None;
    let mut buyer_coin_after_trade = None;
    let mut merchant_coin_after_trade = None;
    let initial_buyer_coin = h
        .world
        .controlled_commodity_quantity(buyer, CommodityKind::Coin);

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

        if listing_tick.is_none() && merchant_has_listed_bread(&h, merchant) {
            listing_tick = Some(tick);
        }
        for event in action_sink.events_for_at(merchant, tick) {
            if !matches!(event.kind, ActionTraceKind::Committed { .. }) {
                continue;
            }
            match event.action_name.as_str() {
                "stage_stock_for_sale" if stage_tick.is_none() => stage_tick = Some(tick),
                _ => {}
            }
        }

        for event in action_sink.events_for_at(buyer, tick) {
            if event.action_name == "trade"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
            {
                successful_trade_count += 1;
                if first_trade_tick.is_none() {
                    first_trade_tick = Some(tick);
                    buyer_bread_after_trade = Some(
                        h.world
                            .controlled_commodity_quantity(buyer, CommodityKind::Bread),
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
            }
            if event.action_name == "eat"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && first_eat_tick.is_none()
            {
                first_eat_tick = Some(tick);
            }
        }
    }

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let merchant_actions = action_sink
        .events_for(merchant)
        .iter()
        .map(|event| format!("{:?}: {}", event.tick, event.summary()))
        .collect::<Vec<_>>();
    let buyer_actions = action_sink
        .events_for(buyer)
        .iter()
        .map(|event| format!("{:?}: {}", event.tick, event.summary()))
        .collect::<Vec<_>>();

    let trade = TradeObservation {
        listing_tick: listing_tick.unwrap_or_else(|| {
            panic!(
                "merchant should expose a listed bread lot before the buyer's local market branch commits; merchant_actions={merchant_actions:?}"
            )
        }),
        initial_buyer_coin,
        stage_tick: stage_tick.unwrap_or_else(|| {
            panic!("merchant should commit stage_stock_for_sale; merchant_actions={merchant_actions:?}")
        }),
        first_trade_tick: first_trade_tick.unwrap_or_else(|| {
            panic!("buyer should commit a bread trade in the survival-trade scenario; buyer_actions={buyer_actions:?}")
        }),
        first_eat_tick: first_eat_tick.unwrap_or_else(|| {
            panic!("buyer should eventually commit eat after the bread trade; buyer_actions={buyer_actions:?}")
        }),
        successful_trade_count,
        buyer_bread_after_trade: buyer_bread_after_trade.expect("trade commit should snapshot buyer bread"),
        buyer_coin_after_trade: buyer_coin_after_trade.expect("trade commit should snapshot buyer coin"),
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

    SurvivalTradeObservation {
        contract,
        agents,
        stuck_idle_windows,
        trade,
    }
}

// ---------------------------------------------------------------------------
// Scenario 173: Survival Trade Proves the Live Market Branch
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Trade, Travel, Perception
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity, SellCommodity, Drink, Wash, Sleep, Relieve
// ActionDomains: Trade, Travel, Needs
// Places: Market Square, South Orchard
// Principles: 6, 7, 8, 14, 20
//
// Setup: Run the authored survival trade scenario for 1440 ticks. The buyer
// starts hungry at Market Square with coin but no local food except the
// merchant's bread stock. A remote orchard exists in the authored world, but
// the proved branch stays local: repeated bread trade sustains the buyer.
//
// Proves: both agents satisfy the authored survival-health contract; the
// merchant lawfully stages a listed bread lot at the market seam; and the
// buyer repeatedly commits bread trades, with authoritative bread transfer and
// coin transfer visible at the trade seam before the first eat commit.
//
// Chain: merchant-owned bread stock -> committed stage_stock_for_sale ->
// SaleListing present -> buyer committed trade -> buyer holds Bread and spends
// Coin -> buyer later commits eat -> repeated local trades keep the survival
// loop viable.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_trade_proves_live_market_branch() {
    let observation = run_survival_trade();
    let run_limit_overrides =
        contract_run_limit_overrides(observation.contract.critical_run_limits.as_ref());

    for (agent_name, agent) in &observation.agents {
        assert!(
            agent.alive,
            "{agent_name} should remain alive for the full {SURVIVAL_TICKS}-tick scenario; observation={agent:?}"
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
            "survival-trade",
        );
    }
    assert_no_stuck_idle_windows(
        observation
            .contract
            .max_idle_window_ticks_with_elevated_need,
        observation.contract.elevated_need_floor.value(),
        "survival-trade",
        &observation.stuck_idle_windows,
    );
    assert!(
        observation.trade.stage_tick <= observation.trade.listing_tick,
        "bread should not become listed before the merchant stages it for sale; observation={observation:?}"
    );
    assert!(
        observation.trade.listing_tick <= observation.trade.first_trade_tick,
        "buyer trade should only happen after a listed bread lot exists; observation={observation:?}"
    );
    assert!(
        observation.trade.first_trade_tick <= observation.trade.first_eat_tick,
        "buyer should only eat after the trade branch acquires bread; observation={observation:?}"
    );
    assert!(
        observation.trade.successful_trade_count >= 2,
        "survival-trade should sustain multiple successful bread purchases, not just one lucky opening trade; observation={observation:?}"
    );
    assert!(
        observation.trade.buyer_bread_after_trade > Quantity(0),
        "trade should leave the buyer holding bread at the authoritative seam; observation={observation:?}"
    );
    assert!(
        observation.trade.buyer_coin_after_trade < observation.trade.initial_buyer_coin,
        "trade should reduce buyer coin from the authored starting amount; observation={observation:?}"
    );
    assert!(
        observation.trade.merchant_coin_after_trade > Quantity(0),
        "trade should increase merchant coin at the authoritative seam; observation={observation:?}"
    );
}

#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_trade_replays_deterministically() {
    assert_eq!(run_survival_trade(), run_survival_trade());
}
