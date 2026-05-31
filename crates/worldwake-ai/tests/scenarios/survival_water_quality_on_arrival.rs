//! S177 focused golden: water-source quality is corrected only after local
//! arrival observation.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::golden_harness::*;
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario};
use worldwake_core::{
    CommodityKind, DecisionEventPayload, EntityId, EventTag, EventView, ReliabilityRecord,
    SourceKey, SourceReliability, Tick, WaterQuality,
};

const TICKS: u32 = 12;

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/survival-water-quality-on-arrival.ron")
}

fn named_entity(h: &GoldenHarness, target_name: &str) -> EntityId {
    h.world
        .query_name()
        .find(|(_, name)| name.0 == target_name)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("scenario should include named entity {target_name}"))
}

fn named_agent(h: &GoldenHarness, target_name: &str) -> EntityId {
    h.world
        .query_name_and_agent_data()
        .find(|(_, name, _)| name.0 == target_name)
        .map(|(entity, _, _)| entity)
        .unwrap_or_else(|| panic!("scenario should include named agent {target_name}"))
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

fn load_harness() -> GoldenHarness {
    let def = load_scenario_file(&scenario_path()).expect("scenario should parse");
    let spawned = spawn_scenario(&def).expect("scenario should spawn");
    let mut h = GoldenHarness::from_simulation_state(&spawned.state);
    h.driver.enable_tracing();
    h.enable_action_tracing();
    h
}

fn seed_clean_quality_belief(h: &mut GoldenHarness, agent: EntityId, source: EntityId) {
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );
    let mut record = ReliabilityRecord::default();
    record.observe_quality(WaterQuality::Clean, Tick(0));
    record.observe_capacity(20, Tick(0));
    let reliability = SourceReliability {
        sources: BTreeMap::from([(
            SourceKey {
                entity: source,
                commodity: CommodityKind::Water,
            },
            record,
        )]),
    };
    let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
    txn.set_component_source_reliability(agent, reliability)
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn quality_observation_event_ticks(
    h: &GoldenHarness,
    agent: EntityId,
    source: EntityId,
) -> Vec<Tick> {
    h.event_log
        .events_by_tag(EventTag::ResourceSourceQualityObserved)
        .iter()
        .filter_map(|event_id| h.event_log.get(*event_id))
        .filter_map(|record| match record.decision_payload()? {
            DecisionEventPayload::ResourceSourceQualityObserved(payload)
                if payload.observer == agent
                    && payload.source.entity == source
                    && payload.source.commodity == CommodityKind::Water =>
            {
                Some(record.tick())
            }
            _ => None,
        })
        .collect()
}

fn run() -> (GoldenHarness, EntityId, EntityId, Vec<Tick>, Tick) {
    let mut h = load_harness();
    let agent = named_agent(&h, "Quality Scout");
    let source = named_entity(&h, "Believed Well");
    let source_place = named_place(&h, "Muddy Spring");
    seed_clean_quality_belief(&mut h, agent, source);

    let mut pre_arrival_events = Vec::new();
    let mut arrival_tick = None;
    for _ in 0..TICKS {
        if h.world.effective_place(agent) == Some(source_place) && arrival_tick.is_none() {
            arrival_tick = Some(h.scheduler.current_tick());
        }
        if arrival_tick.is_none() {
            pre_arrival_events.extend(quality_observation_event_ticks(&h, agent, source));
        }
        h.step_once();
    }

    (
        h,
        agent,
        source,
        pre_arrival_events,
        arrival_tick.expect("agent should arrive at the muddy source"),
    )
}

// Scenario 488: Water Quality On Arrival Records Belief Correction
// ---------------------------------------------------------------------------
//
// Systems: AI, Perception, SourceReliability
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Production, Travel
// Places: Trailhead, Muddy Spring, Clear Spring
// Principles: 7, 14A, 15, 16, 31
//
// Setup: the agent knows a remote water source, has a fresh reliability record claiming it is Clean, and the authoritative source is Muddy.
//
// Proves: the normal co-located perception path overwrites the quality belief with Muddy and emits `ResourceSourceQualityObserved` at/after arrival.
//
// Chain: reported clean-source belief -> travel to source -> local observation -> SourceReliability quality correction + attribution event.
#[test]
fn golden_survival_water_quality_on_arrival_records_belief_correction() {
    let (h, agent, source, _, arrival_tick) = run();
    let record = h
        .world
        .get_component_source_reliability(agent)
        .and_then(|reliability| {
            reliability.sources.get(&SourceKey {
                entity: source,
                commodity: CommodityKind::Water,
            })
        })
        .copied()
        .expect("arrival perception should retain the source reliability record");

    assert_eq!(record.last_observed_quality, Some(WaterQuality::Muddy));
    assert!(
        record.last_observed_quality_tick >= arrival_tick,
        "quality correction should not precede arrival; arrival={arrival_tick:?}, record={record:?}"
    );
    assert!(
        quality_observation_event_ticks(&h, agent, source)
            .into_iter()
            .any(|tick| tick >= arrival_tick),
        "arrival should emit a quality-observation event"
    );
}

// Scenario 489: Water Quality On Arrival Has No Omniscient Pre-Arrival Update
// ---------------------------------------------------------------------------
//
// Systems: AI, Perception, SourceReliability
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Travel
// Places: Trailhead, Muddy Spring
// Principles: 7, 14, 14A, 15, 31
//
// Setup: same as Scenario 488.
//
// Proves: no quality-observation event for the muddy source is emitted before the actor reaches the source place.
#[test]
fn golden_survival_water_quality_on_arrival_emits_no_omniscient_belief_correction() {
    let (_, _, _, pre_arrival_events, arrival_tick) = run();
    assert!(
        pre_arrival_events.is_empty(),
        "quality observations before arrival {arrival_tick:?} would be omniscient; events={pre_arrival_events:?}"
    );
}

// Scenario 490: Water Quality On Arrival Replay Is Deterministic
// ---------------------------------------------------------------------------
//
// Systems: AI, Perception, SourceReliability
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Production, Travel
// Places: Trailhead, Muddy Spring, Clear Spring
// Principles: 2, 9, 31
#[test]
fn golden_survival_water_quality_on_arrival_replays_deterministically() {
    let run_hash = || {
        let (h, _, _, _, _) = run();
        (
            worldwake_core::hash_world(&h.world).expect("world should hash canonically"),
            worldwake_core::hash_event_log(&h.event_log)
                .expect("event log should hash canonically"),
        )
    };
    assert_eq!(run_hash(), run_hash());
}
