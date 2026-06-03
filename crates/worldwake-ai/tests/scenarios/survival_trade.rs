//! Golden tests for the survival trade roadmap row.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::golden_harness::*;
use worldwake_ai::{DecisionOutcome, PlannerOpKind};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{
    AcquisitionQuantity, CommodityKind, CommodityPurpose, DriveThresholds, EntityId, EventTag,
    EventView, GoalKey, GoalKind, PerceptionSource, Quantity, Tick,
};
use worldwake_sim::ActionTraceKind;

const SURVIVAL_TICKS: u32 = 1440;
const DIRECT_COMMODITY: CommodityKind = CommodityKind::Bread;
const SUBSTITUTE_COMMODITY: CommodityKind = CommodityKind::Apple;

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
    selected_trade_branch_tick: Tick,
    first_trade_tick: Tick,
    first_eat_tick: Tick,
    successful_trade_count: u32,
    saw_listed_bread: bool,
    selected_trade_payload_commodity: CommodityKind,
    buyer_substitute_after_trade: Quantity,
    buyer_bread_after_trade: Quantity,
    buyer_coin_after_trade: Quantity,
    merchant_coin_after_trade: Quantity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FacilityContentionObservation {
    facility: EntityId,
    queue_commit_ticks: BTreeMap<String, Tick>,
    grant_promoted_ticks: Vec<Tick>,
    first_harvest_after_queue_tick: Option<Tick>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SurvivalTradeObservation {
    contract: worldwake_cli::scenario::types::SurvivalHealthContractDef,
    agents: BTreeMap<String, AgentSurvivalObservation>,
    stuck_idle_windows: Vec<StuckIdleWindow>,
    trade: TradeObservation,
    facility_contention: FacilityContentionObservation,
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

fn named_entity(h: &GoldenHarness, target_name: &str) -> EntityId {
    h.world
        .query_name()
        .find_map(|(entity, name)| (name.0 == target_name).then_some(entity))
        .unwrap_or_else(|| panic!("scenario should include named entity '{target_name}'"))
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

fn merchant_has_listed_substitute(h: &GoldenHarness, merchant: EntityId) -> bool {
    h.world.entities().any(|entity| {
        h.world
            .get_component_item_lot(entity)
            .is_some_and(|lot| lot.commodity == SUBSTITUTE_COMMODITY && lot.quantity > Quantity(0))
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
    let village_well = named_entity(&h, "Village Well");
    let substitute_goal = GoalKey::from(GoalKind::AcquireCommodity {
        commodity: SUBSTITUTE_COMMODITY,
        purpose: CommodityPurpose::SelfConsume,
        quantity: AcquisitionQuantity::single(),
    });
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
    let mut selected_trade_branch_tick = None;
    let mut first_trade_tick = None;
    let mut first_eat_tick = None;
    let mut successful_trade_count = 0_u32;
    let mut saw_listed_bread = false;
    let mut selected_trade_payload_commodity = None;
    let mut buyer_substitute_after_trade = None;
    let mut buyer_bread_after_trade = None;
    let mut buyer_coin_after_trade = None;
    let mut merchant_coin_after_trade = None;
    let mut queue_commit_ticks = BTreeMap::new();
    let mut first_harvest_after_queue_tick = None;
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

            let had_action = crate::golden_harness::agent_has_non_failed_action_or_active(
                &h,
                action_sink,
                *agent,
                tick,
            );
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

        if listing_tick.is_none() && merchant_has_listed_substitute(&h, merchant) {
            listing_tick = Some(tick);
        }
        saw_listed_bread |= h.world.entities().any(|entity| {
            h.world
                .get_component_item_lot(entity)
                .is_some_and(|lot| lot.commodity == DIRECT_COMMODITY && lot.quantity > Quantity(0))
                && h.world.get_component_sale_listing(entity).is_some()
                && h.world.effective_place(entity) == h.world.effective_place(merchant)
        });
        for event in action_sink.events_for_at(merchant, tick) {
            if !matches!(event.kind, ActionTraceKind::Committed { .. }) {
                continue;
            }
            match event.action_name.as_str() {
                "stage_stock_for_sale" if stage_tick.is_none() => stage_tick = Some(tick),
                "queue_for_facility_use" => {
                    queue_commit_ticks
                        .entry("Merchant Sera".to_string())
                        .or_insert(tick);
                }
                "harvest:Harvest Water" if first_harvest_after_queue_tick.is_none() => {
                    first_harvest_after_queue_tick = Some(tick);
                }
                _ => {}
            }
        }

        if selected_trade_branch_tick.is_none() {
            let Some(trace) = h
                .driver
                .trace_sink()
                .and_then(|sink| sink.trace_at(buyer, tick))
            else {
                continue;
            };
            let Some(runtime) = h.driver.runtime(buyer) else {
                continue;
            };
            let Some(plan) = runtime.current_plan.as_ref() else {
                continue;
            };
            let Some(step) = plan.steps.first() else {
                continue;
            };
            let Some(payload) = step
                .payload_override
                .as_ref()
                .and_then(|payload| payload.as_trade())
            else {
                continue;
            };
            let Some(lot) = h.world.get_component_item_lot(payload.sale_lot) else {
                continue;
            };
            let DecisionOutcome::Planning(planning) = &trace.outcome else {
                continue;
            };
            if planning.selection.selected_goal() == Some(substitute_goal)
                && h.world.effective_place(buyer) == h.world.effective_place(merchant)
                && step.op_kind == PlannerOpKind::Trade
                && payload.counterparty == merchant
            {
                selected_trade_branch_tick = Some(tick);
                selected_trade_payload_commodity = Some(lot.commodity);
            }
        }

        for event in action_sink.events_for_at(buyer, tick) {
            if event.action_name == "trade"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
            {
                successful_trade_count += 1;
                if first_trade_tick.is_none() {
                    first_trade_tick = Some(tick);
                    buyer_substitute_after_trade = Some(
                        h.world
                            .controlled_commodity_quantity(buyer, SUBSTITUTE_COMMODITY),
                    );
                    buyer_bread_after_trade = Some(
                        h.world
                            .controlled_commodity_quantity(buyer, DIRECT_COMMODITY),
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
            if event.action_name == "queue_for_facility_use"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
            {
                queue_commit_ticks
                    .entry("Buyer Nila".to_string())
                    .or_insert(tick);
            }
            if event.action_name == "harvest:Harvest Water"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && first_harvest_after_queue_tick.is_none()
            {
                first_harvest_after_queue_tick = Some(tick);
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
                "merchant should expose a listed substitute lot before the buyer's local market branch commits; merchant_actions={merchant_actions:?}"
            )
        }),
        initial_buyer_coin,
        stage_tick: stage_tick.unwrap_or_else(|| {
            panic!("merchant should commit stage_stock_for_sale; merchant_actions={merchant_actions:?}")
        }),
        selected_trade_branch_tick: selected_trade_branch_tick.unwrap_or_else(|| {
            panic!(
                "buyer should reach a local substitute-trade binding with an explicit trade payload before commit; buyer_actions={buyer_actions:?}"
            )
        }),
        first_trade_tick: first_trade_tick.unwrap_or_else(|| {
            panic!(
                "buyer should commit a substitute trade in the survival-trade scenario; buyer_actions={buyer_actions:?}"
            )
        }),
        first_eat_tick: first_eat_tick.unwrap_or_else(|| {
            panic!(
                "buyer should eventually commit eat after the substitute trade; buyer_actions={buyer_actions:?}"
            )
        }),
        successful_trade_count,
        saw_listed_bread,
        selected_trade_payload_commodity: selected_trade_payload_commodity
            .expect("selected substitute branch should snapshot payload commodity"),
        buyer_substitute_after_trade: buyer_substitute_after_trade
            .expect("trade commit should snapshot buyer substitute commodity"),
        buyer_bread_after_trade: buyer_bread_after_trade
            .expect("trade commit should snapshot buyer bread"),
        buyer_coin_after_trade: buyer_coin_after_trade.expect("trade commit should snapshot buyer coin"),
        merchant_coin_after_trade: merchant_coin_after_trade
            .expect("trade commit should snapshot merchant coin"),
    };
    let grant_promoted_ticks = h
        .event_log
        .events_by_tag(EventTag::QueueGrantPromoted)
        .iter()
        .filter_map(|event_id| h.event_log.get(*event_id))
        .filter(|event| event.target_ids().contains(&village_well))
        .map(EventView::tick)
        .collect::<Vec<_>>();

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
        facility_contention: FacilityContentionObservation {
            facility: village_well,
            queue_commit_ticks,
            grant_promoted_ticks,
            first_harvest_after_queue_tick,
        },
    }
}

// ---------------------------------------------------------------------------
// Scenario 350: Survival Trade Proves the Substitute Market Branch
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Trade, Travel, Perception, Production
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity, SellCommodity, Drink, Wash, Sleep, Relieve
// ActionDomains: Trade, Travel, Needs, Production
// Places: Market Square, South Orchard
// Principles: 6, 7, 8, 9, 14, 20
//
// Setup: Run the authored survival trade scenario for 1440 ticks. The buyer
// starts hungry at Market Square with coin but no direct bread-market branch.
// The merchant stages only apples, the buyer's authored food substitutes are
// `[Apple, Grain]`, and the Market Square well is contention-managed with
// queue patience on both principals. The proved food branch stays local: a
// substitute-backed apple trade sustains the buyer before later self-care
// continues.
//
// Proves: both agents satisfy the authored survival-health contract; the
// merchant lawfully stages a listed apple lot at the market seam; the buyer
// reaches a local `AcquireCommodity(SelfConsume)` substitute branch whose live
// runtime plan keeps an explicit `trade` payload bound to that apple lot; and
// the buyer later commits substitute trade with authoritative apple and coin
// transfer visible at the trade seam before the first eat commit. The same run
// also proves the remaining facility-contention extension: the authored Market
// Square well uses the real `queue_for_facility_use` action, the contention
// system promotes a grant, and a water harvest commits only after the queue
// branch has made exclusive use lawful.
//
// Chain: no listed bread market branch -> committed stage_stock_for_sale ->
// Apple SaleListing present -> buyer selects local `AcquireCommodity(Apple,
// SelfConsume)` -> current plan retains explicit `trade` payload against the
// merchant's apple lot -> buyer holds Apple and spends Coin -> buyer later
// commits eat; Market Square well carries `ContentionPolicy` -> agent commits
// `queue_for_facility_use` -> QueueGrantPromoted event targets the well ->
// harvest-water commit follows the grant path -> the combined trade and queue
// branches keep the survival loop viable.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_trade_proves_substitute_market_branch() {
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
        "substitute commodity should not become listed before the merchant stages it for sale; observation={observation:?}"
    );
    assert!(
        observation.trade.listing_tick <= observation.trade.selected_trade_branch_tick,
        "buyer should only reach the substitute trade binding after a listed substitute lot exists; observation={observation:?}"
    );
    assert!(
        observation.trade.selected_trade_branch_tick <= observation.trade.first_trade_tick,
        "buyer should only commit trade after the substitute trade branch is selected; observation={observation:?}"
    );
    assert!(
        observation.trade.first_trade_tick <= observation.trade.first_eat_tick,
        "buyer should only eat after the trade branch acquires the substitute commodity; observation={observation:?}"
    );
    assert!(
        !observation.trade.saw_listed_bread,
        "the authored scenario should never expose a listed bread lot, so the direct bread-market branch stays impossible; observation={observation:?}"
    );
    assert!(
        observation.trade.selected_trade_payload_commodity == SUBSTITUTE_COMMODITY,
        "selected trade branch should retain an explicit substitute commodity payload; observation={observation:?}"
    );
    assert!(
        observation.trade.successful_trade_count >= 1,
        "survival-trade should commit at least one successful substitute purchase; observation={observation:?}"
    );
    assert!(
        observation.trade.buyer_substitute_after_trade > Quantity(0),
        "trade should leave the buyer holding the substitute commodity at the authoritative seam; observation={observation:?}"
    );
    assert!(
        observation.trade.buyer_bread_after_trade == Quantity(0),
        "substitute trade should not silently restore the excluded bread branch at the authoritative seam; observation={observation:?}"
    );
    assert!(
        observation.trade.buyer_coin_after_trade < observation.trade.initial_buyer_coin,
        "trade should reduce buyer coin from the authored starting amount; observation={observation:?}"
    );
    assert!(
        observation.trade.merchant_coin_after_trade > Quantity(0),
        "trade should increase merchant coin at the authoritative seam; observation={observation:?}"
    );
    assert!(
        !observation
            .facility_contention
            .queue_commit_ticks
            .is_empty(),
        "survival-trade should commit queue_for_facility_use against the Market Square well before a grant-backed harvest; observation={observation:?}"
    );
    let first_queue_tick = observation
        .facility_contention
        .queue_commit_ticks
        .values()
        .min()
        .copied()
        .expect("queue branch should have at least one commit tick");
    let first_grant_tick = observation
        .facility_contention
        .grant_promoted_ticks
        .iter()
        .filter(|tick| **tick >= first_queue_tick)
        .min()
        .copied()
        .expect("contention system should promote a grant for the Market Square well after an observed queue commit");
    assert!(
        first_queue_tick <= first_grant_tick,
        "facility grant should not be promoted before an agent queues; observation={observation:?}"
    );
    let first_harvest_after_queue = observation
        .facility_contention
        .first_harvest_after_queue_tick
        .expect("a water harvest should commit after the queue/grant branch");
    assert!(
        first_grant_tick <= first_harvest_after_queue,
        "exclusive water harvest should commit only after the queue branch promotes a grant; observation={observation:?}"
    );
}

#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_trade_replays_deterministically() {
    assert_eq!(run_survival_trade(), run_survival_trade());
}
