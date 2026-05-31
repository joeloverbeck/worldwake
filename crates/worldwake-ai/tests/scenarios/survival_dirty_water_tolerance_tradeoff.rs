//! S177 focused golden: water tolerance changes same-world source choice.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::golden_harness::*;
use worldwake_ai::{CommodityPurpose, DecisionOutcome, GoalKind, RankedGoalComparisonDimension};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario};
use worldwake_core::{
    AcquisitionQuantity, CommodityKind, EntityId, GoalKey, ReliabilityRecord, SourceKey,
    SourceReliability, Tick, WaterQuality,
};

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/survival-dirty-water-tolerance-tradeoff.ron")
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

fn named_entity(h: &GoldenHarness, target_name: &str) -> EntityId {
    h.world
        .query_name()
        .find(|(_, name)| name.0 == target_name)
        .map_or_else(
            || panic!("scenario should include named entity {target_name}"),
            |(entity, _)| entity,
        )
}

fn load_harness() -> GoldenHarness {
    let def = load_scenario_file(&scenario_path()).expect("scenario should parse");
    let spawned = spawn_scenario(&def).expect("scenario should spawn");
    let mut h = GoldenHarness::from_simulation_state(&spawned.state);
    h.driver.enable_tracing();
    h.enable_action_tracing();
    h
}

fn seed_quality_beliefs(h: &mut GoldenHarness, agent: EntityId, muddy: EntityId, clean: EntityId) {
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );
    let mut muddy_record = ReliabilityRecord::default();
    muddy_record.observe_quality(WaterQuality::Muddy, Tick(0));
    muddy_record.observe_capacity(20, Tick(0));
    let mut clean_record = ReliabilityRecord::default();
    clean_record.observe_quality(WaterQuality::Clean, Tick(0));
    clean_record.observe_capacity(20, Tick(0));
    let reliability = SourceReliability {
        sources: BTreeMap::from([
            (
                SourceKey {
                    entity: muddy,
                    commodity: CommodityKind::Water,
                },
                muddy_record,
            ),
            (
                SourceKey {
                    entity: clean,
                    commodity: CommodityKind::Water,
                },
                clean_record,
            ),
        ]),
    };
    let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
    txn.set_component_source_reliability(agent, reliability)
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn acquire_water_goal() -> GoalKey {
    GoalKey::from(GoalKind::AcquireCommodity {
        commodity: CommodityKind::Water,
        purpose: CommodityPurpose::SelfConsume,
        quantity: AcquisitionQuantity::single(),
    })
}

fn selected_source(
    h: &GoldenHarness,
    agent: EntityId,
) -> (EntityId, Option<RankedGoalComparisonDimension>) {
    let trace = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .trace_at(agent, Tick(0))
        .expect("expected initial planning trace");
    let DecisionOutcome::Planning(planning) = &trace.outcome else {
        panic!("expected planning trace; outcome={:?}", trace.outcome);
    };
    assert_eq!(
        planning.selection.selected_goal().map(|goal| goal.kind),
        Some(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        })
    );
    let dimension = planning
        .candidates
        .top_ranked_comparison
        .as_ref()
        .map(|comparison| comparison.decisive_dimension);
    let source = planning
        .candidates
        .ranked
        .first()
        .and_then(|summary| {
            (summary.opportunity.goal_key == acquire_water_goal())
                .then_some(summary.source_composite?.source_entity)
        })
        .unwrap_or_else(|| {
            panic!(
                "expected top ranked water source; ranked={:?}",
                planning.candidates.ranked
            )
        });
    (source, dimension)
}

fn run_to_initial_choices() -> (GoldenHarness, EntityId, EntityId, EntityId, EntityId) {
    let mut h = load_harness();
    let hardy = named_agent(&h, "Hardy Drinker");
    let fragile = named_agent(&h, "Fragile Drinker");
    let muddy = named_entity(&h, "Muddy Pump");
    let clean = named_entity(&h, "Clear Well");
    seed_quality_beliefs(&mut h, hardy, muddy, clean);
    seed_quality_beliefs(&mut h, fragile, muddy, clean);
    h.step_once();
    (h, hardy, fragile, muddy, clean)
}

// Scenario 491: Dirty-Water Tolerance Tradeoff Hardy Agent Chooses Muddy
// ---------------------------------------------------------------------------
//
// Systems: AI, SourceReliability, WaterToleranceProfile
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Production, Travel
// Places: Dust Camp, Clear Spring
// Principles: 3, 15, 22, 31
//
// Setup: two agents have the same beliefs and world state; the hardy agent's profile makes Muddy water neutral, so local/source tiebreakers may pick it.
//
// Proves: the hardy agent's decision trace selects the local muddy source.
#[test]
fn golden_dirty_water_tolerance_tradeoff_hardy_agent_drinks_muddy() {
    let (h, hardy, _, muddy, _) = run_to_initial_choices();
    assert_eq!(selected_source(&h, hardy).0, muddy);
}

// Scenario 492: Dirty-Water Tolerance Tradeoff Fragile Agent Travels To Fallback
// ---------------------------------------------------------------------------
//
// Systems: AI, SourceReliability, WaterToleranceProfile
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Production, Travel
// Places: Dust Camp, Clear Spring
// Principles: 3, 15, 22, 31
//
// Setup: same world and beliefs as Scenario 491; the fragile agent's profile heavily discounts Muddy water.
//
// Proves: the fragile agent's decision trace selects the farther clean source.
#[test]
fn golden_dirty_water_tolerance_tradeoff_fragile_agent_travels_to_fallback() {
    let (h, _, fragile, _, clean) = run_to_initial_choices();
    let (source, dimension) = selected_source(&h, fragile);
    assert_eq!(source, clean);
    assert_eq!(
        dimension,
        Some(RankedGoalComparisonDimension::SourceComposite),
        "fragile tolerance should make quality-aware source composite reject muddy water"
    );
}

// Scenario 493: Dirty-Water Tolerance Tradeoff Replay Is Deterministic
// ---------------------------------------------------------------------------
//
// Systems: AI, SourceReliability, WaterToleranceProfile
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Production, Travel
// Places: Dust Camp, Clear Spring
// Principles: 2, 9, 31
#[test]
fn golden_dirty_water_tolerance_tradeoff_replays_deterministically() {
    let run_hash = || {
        let (h, _, _, _, _) = run_to_initial_choices();
        (
            worldwake_core::hash_world(&h.world).expect("world should hash canonically"),
            worldwake_core::hash_event_log(&h.event_log)
                .expect("event log should hash canonically"),
        )
    };
    assert_eq!(run_hash(), run_hash());
}
