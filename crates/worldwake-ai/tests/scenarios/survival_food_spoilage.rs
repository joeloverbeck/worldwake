//! S178 golden coverage for perishable food spoilage.

use std::path::PathBuf;

use crate::golden_harness::*;
use worldwake_ai::{SpoiledFoodDiscovery, SpoiledFoodOutcome, SurvivalForensicExtractor};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario};
use worldwake_core::{
    CommodityKind, Container, DecisionEventPayload, EntityId, EventTag, EventView, Freshness,
    HomeostaticNeedId, LoadUnits, LotOperation, PerishableState, Permille, Quantity, Tick,
};
use worldwake_sim::ActionTraceKind;

const LIFECYCLE_TICKS: u32 = 500;
const CACHE_TICKS: u32 = 36;
const LONG_TICKS: u32 = 1440;

fn scenario_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios")
        .join(name)
}

fn load_harness(name: &str) -> GoldenHarness {
    let def = load_scenario_file(&scenario_path(name)).expect("scenario should parse");
    let spawned = spawn_scenario(&def).expect("scenario should spawn");
    let mut h = GoldenHarness::from_simulation_state(&spawned.state);
    h.driver.enable_tracing();
    h.enable_action_tracing();
    h
}

fn named_agent(h: &GoldenHarness, target_name: &str) -> EntityId {
    h.world
        .query_name_and_agent_data()
        .find(|(_, name, _)| name.0 == target_name)
        .map_or_else(
            || panic!("scenario should include named agent {target_name}"),
            |(entity, _, _)| entity,
        )
}

fn named_place(h: &GoldenHarness, target_name: &str) -> EntityId {
    h.world
        .topology()
        .place_ids()
        .find(|place| {
            h.world
                .topology()
                .place(*place)
                .is_some_and(|data| data.name == target_name)
        })
        .unwrap_or_else(|| panic!("scenario should include place {target_name}"))
}

fn apple_lot_by_quantity(h: &GoldenHarness, quantity: u32) -> EntityId {
    h.world
        .query_item_lot()
        .find(|(_, lot)| {
            lot.commodity == CommodityKind::Apple && lot.quantity == Quantity(quantity)
        })
        .map_or_else(
            || panic!("scenario should include Apple lot quantity {quantity}"),
            |(entity, _)| entity,
        )
}

fn put_lot_in_test_container(h: &mut GoldenHarness, lot: EntityId, place: EntityId) -> EntityId {
    let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
    let container = txn
        .create_container(Container {
            capacity: LoadUnits(10),
            allowed_commodities: None,
            allows_unique_items: false,
            allows_nested_containers: false,
        })
        .expect("test container should be valid");
    txn.set_ground_location(container, place)
        .expect("container should be placeable");
    txn.put_into_container(lot, container)
        .expect("Apple lot should fit into test container");
    commit_txn(txn, &mut h.event_log);
    container
}

fn set_lot_condition(h: &mut GoldenHarness, lot: EntityId, condition: Permille) {
    let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
    txn.set_component_perishable_state(
        lot,
        PerishableState {
            condition,
            last_advanced_tick: h.scheduler.current_tick(),
            decay_remainder: 0,
        },
    )
    .expect("lot perishable state should be writable");
    commit_txn(txn, &mut h.event_log);
}

fn set_spoiled_food_threshold(h: &mut GoldenHarness, agent: EntityId, threshold: Permille) {
    let mut profile = *h
        .world
        .get_component_metabolism_profile(agent)
        .expect("scenario agent should have metabolism profile");
    profile.spoiled_food_hunger_threshold = threshold;
    let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
    txn.set_component_metabolism_profile(agent, profile)
        .expect("metabolism profile should be writable");
    commit_txn(txn, &mut h.event_log);
}

fn condition(h: &GoldenHarness, lot: EntityId) -> Permille {
    h.world
        .get_component_perishable_state(lot)
        .map(|state| state.condition)
        .expect("Apple lot should be perishable")
}

fn spoilage_events_for(h: &GoldenHarness, lot: EntityId) -> Vec<Tick> {
    h.event_log
        .events_by_tag(EventTag::ItemSpoiled)
        .iter()
        .filter_map(|event_id| h.event_log.get(*event_id))
        .filter(|record| record.target_ids().contains(&lot))
        .map(EventView::tick)
        .collect()
}

fn lot_has_spoiled_provenance(h: &GoldenHarness, lot: EntityId) -> bool {
    h.world.get_component_item_lot(lot).is_some_and(|item_lot| {
        item_lot
            .provenance
            .iter()
            .any(|entry| matches!(entry.operation, LotOperation::Spoiled))
    })
}

fn mismatch_ticks(h: &GoldenHarness, agent: EntityId, lot: EntityId) -> Vec<Tick> {
    h.event_log
        .events_by_tag(EventTag::ExpectationMismatch)
        .iter()
        .filter_map(|event_id| h.event_log.get(*event_id))
        .filter_map(|record| match record.decision_payload()? {
            DecisionEventPayload::LotConditionExpectationMismatch(payload)
                if payload.observer == agent && payload.lot == lot =>
            {
                Some(record.tick())
            }
            _ => None,
        })
        .collect()
}

fn observe_agent_window(
    extractor: &mut SurvivalForensicExtractor,
    h: &GoldenHarness,
    agent: EntityId,
    tick: Tick,
) {
    let needs = *h
        .world
        .get_component_homeostatic_needs(agent)
        .expect("scenario agent should have needs");
    let thresholds = *h
        .world
        .get_component_drive_thresholds(agent)
        .expect("scenario agent should have drive thresholds");
    observe_critical_windows(extractor, h, agent, tick, &needs, &thresholds);
}

fn committed_actions(h: &GoldenHarness, agent: EntityId, action_name: &str) -> Vec<Tick> {
    let Some(sink) = h.action_trace_sink() else {
        return Vec::new();
    };
    sink.events_for(agent)
        .iter()
        .filter(|event| {
            event.action_name == action_name
                && matches!(event.kind, ActionTraceKind::Committed { .. })
        })
        .map(|event| event.tick)
        .collect()
}

#[derive(Debug)]
struct CacheObservation {
    arrival_tick: Tick,
    pre_arrival_mismatch_ticks: Vec<Tick>,
    mismatch_ticks: Vec<Tick>,
    discoveries: Vec<SpoiledFoodDiscovery>,
    eat_ticks: Vec<Tick>,
    final_cache_condition: Permille,
}

fn run_cache(threshold: Permille, spoil_fallback: bool) -> CacheObservation {
    let mut h = load_harness("survival-food-spoilage-cache.ron");
    let agent = named_agent(&h, "Cache Scout");
    let cache_place = named_place(&h, "Remembered Cache");
    let cache_lot = apple_lot_by_quantity(&h, 1);
    let fallback_lot = apple_lot_by_quantity(&h, 2);

    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        agent,
        cache_lot,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );
    set_spoiled_food_threshold(&mut h, agent, threshold);
    set_lot_condition(&mut h, cache_lot, pm(100));
    if spoil_fallback {
        set_lot_condition(&mut h, fallback_lot, pm(100));
        let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
        txn.set_ground_location(agent, cache_place)
            .expect("above-threshold branch should start at spoiled cache");
        commit_txn(txn, &mut h.event_log);
    }

    let mut extractor = SurvivalForensicExtractor::new(agent);
    let mut arrival_tick =
        (h.world.effective_place(agent) == Some(cache_place)).then_some(h.scheduler.current_tick());
    for tick_num in 0..CACHE_TICKS {
        h.step_once();
        let tick = Tick(u64::from(tick_num));
        if h.world.effective_place(agent) == Some(cache_place) && arrival_tick.is_none() {
            arrival_tick = Some(tick);
        }
        observe_agent_window(&mut extractor, &h, agent, tick);
    }

    let reports = extractor.finalize();
    let discoveries = reports
        .iter()
        .filter(|report| report.need == HomeostaticNeedId::Hunger)
        .flat_map(|report| &report.frames)
        .flat_map(|frame| frame.spoiled_food_discoveries.iter().copied())
        .collect();

    let mismatch_ticks = mismatch_ticks(&h, agent, cache_lot);
    let arrival_tick = arrival_tick.expect("agent should travel to the remembered cache");
    let pre_arrival_mismatch_ticks = mismatch_ticks
        .iter()
        .copied()
        .filter(|tick| *tick < arrival_tick)
        .collect();
    let eat_ticks = committed_actions(&h, agent, "eat");
    CacheObservation {
        arrival_tick,
        pre_arrival_mismatch_ticks,
        mismatch_ticks,
        discoveries,
        eat_ticks,
        final_cache_condition: condition(&h, cache_lot),
    }
}

fn long_run_digest() -> (worldwake_core::StateHash, worldwake_core::StateHash, usize) {
    let mut h = load_harness("survival-food-spoilage-cache-1440.ron");
    for _ in 0..LONG_TICKS {
        h.step_once();
    }
    let spoiled_count = h.event_log.events_by_tag(EventTag::ItemSpoiled).len();
    (
        worldwake_core::hash_world(&h.world).expect("world should hash canonically"),
        worldwake_core::hash_event_log(&h.event_log).expect("event log should hash canonically"),
        spoiled_count,
    )
}

// Scenario 512: Food Spoilage Lifecycle Differentiates Storage Contexts
// ---------------------------------------------------------------------------
//
// Systems: ItemDecay, EventLog, ItemLot provenance
// GoalKinds: none
// ActionDomains: none
// Places: Spoilage Pantry
// Principles: 3, 7, 9, 12, 31
//
// Setup: three authored Apple lots start fresh; one remains on the ground, one
// is moved into a test container, and one is possessed by the agent.
//
// Proves: condition decay is concrete per lot, ground spoilage crosses the
// threshold before container/possession contexts, spoilage is recorded in the
// append-only event log, and the spoiled lot remains in the world.
#[test]
fn golden_survival_food_spoilage_lifecycle() {
    let mut h = load_harness("survival-food-spoilage-lifecycle.ron");
    let place = named_place(&h, "Spoilage Pantry");
    let ground_lot = apple_lot_by_quantity(&h, 1);
    let container_lot = apple_lot_by_quantity(&h, 2);
    let possessed_lot = apple_lot_by_quantity(&h, 3);
    let _container = put_lot_in_test_container(&mut h, container_lot, place);
    let profile = h
        .world
        .commodity_perish_profiles()
        .get(&CommodityKind::Apple)
        .copied()
        .expect("scenario should define Apple perish profile");

    for _ in 0..LIFECYCLE_TICKS {
        h.step_once();
    }

    let ground_condition = condition(&h, ground_lot);
    let container_condition = condition(&h, container_lot);
    let possessed_condition = condition(&h, possessed_lot);
    assert!(
        ground_condition.value() < profile.spoiled_threshold.value(),
        "ground lot should be spoiled by {LIFECYCLE_TICKS} ticks; condition={ground_condition:?}"
    );
    assert!(
        container_condition.value() > profile.spoiled_threshold.value(),
        "container lot should decay slower than ground; condition={container_condition:?}"
    );
    assert!(
        possessed_condition.value() > ground_condition.value()
            && possessed_condition.value() < container_condition.value(),
        "possessed rate should sit between ground and container rates; ground={ground_condition:?} possessed={possessed_condition:?} container={container_condition:?}"
    );
    assert_eq!(
        Freshness::derive_from(ground_condition, &profile),
        Freshness::Spoiled
    );
    assert_eq!(spoilage_events_for(&h, ground_lot).len(), 1);
    assert!(lot_has_spoiled_provenance(&h, ground_lot));
    assert!(
        h.world.get_component_item_lot(ground_lot).is_some(),
        "spoiled lot should persist instead of disappearing"
    );
}

// Scenario 513: Food Spoilage Cache Corrects Belief On Arrival
// ---------------------------------------------------------------------------
//
// Systems: AI, Perception, SurvivalForensics
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Travel, Needs
// Places: Trail Camp, Remembered Cache, Fallback Grove
// Principles: 7, 14, 14A, 15, 17, 22, 31
//
// Setup: the agent starts with a fresh belief about a remote Apple cache; the
// authoritative lot is spoiled before local arrival.
//
// Proves: no omniscient condition correction appears before arrival, local
// perception emits a lot-condition expectation mismatch, and forensics records
// the spoiled-cache discovery.
#[test]
fn golden_survival_food_spoilage_cache() {
    let obs = run_cache(pm(100), true);
    assert!(
        obs.pre_arrival_mismatch_ticks.is_empty(),
        "condition mismatch before arrival {:?} would be omniscient; obs={obs:?}",
        obs.arrival_tick
    );
    assert!(
        obs.mismatch_ticks
            .iter()
            .any(|tick| *tick >= obs.arrival_tick),
        "arrival should emit a lot-condition expectation mismatch; obs={obs:?}"
    );
    assert!(
        !obs.discoveries.is_empty(),
        "cache correction should produce a spoiled-food forensic discovery; obs={obs:?}"
    );
    assert!(
        obs.final_cache_condition.value() < 333,
        "cache should remain authoritatively spoiled at correction time; obs={obs:?}"
    );

    let below = run_cache(pm(1000), false);
    assert!(
        below
            .discoveries
            .iter()
            .all(|discovery| discovery.outcome != SpoiledFoodOutcome::AteAnyway),
        "high spoiled-food threshold should suppress eating spoiled food; obs={below:?}"
    );
    assert!(
        below.eat_ticks.is_empty(),
        "below-threshold branch should avoid Eat; high={obs:?} low={below:?}"
    );
}

// Scenario 514: Food Spoilage Cache Long Run Replays Deterministically
// ---------------------------------------------------------------------------
//
// Systems: ItemDecay, EventLog, Replay hashing
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Travel, Needs
// Places: Shared Camp, North Orchard, South Orchard
// Principles: 2, 3, 9, 22, 31
//
// Setup: multiple agents share more authored Apple stock than they can consume
// before the spoilage threshold.
//
// Proves: the long-horizon scenario exercises normal spoilage over 1440 ticks
// and replay hashes remain deterministic.
#[test]
#[ignore = "CI-only: long-running 1440-tick food-spoilage collision; run via golden-survival workflow"]
fn golden_survival_food_spoilage_cache_1440() {
    let first = long_run_digest();
    let second = long_run_digest();
    assert_eq!(first, second);
    assert!(
        first.2 >= 3,
        "stockpile should produce multiple spoiled-lot events; digest={first:?}"
    );
}
