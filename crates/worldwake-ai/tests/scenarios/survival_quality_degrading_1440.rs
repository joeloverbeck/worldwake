//! S177 CI-owned collision golden: finite clean water drives muddy fallback
//! observation and quality-tolerance divergence over a 1440-tick survival run.
//!
//! The authored scenario isolates water pressure: no trade, no combat, no sleep
//! pressure, and no social relays. The test seeds entity/location beliefs so
//! agents know the reachable source candidates, then strips water quality from
//! those source snapshots; muddy quality enters through normal co-located
//! perception after travel.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::golden_harness::*;
use worldwake_ai::{SourceFailureCause, SourceFailureOutcome, SurvivalForensicExtractor};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario};
use worldwake_core::{
    DecisionEventPayload, EntityId, EventTag, EventView, HomeostaticNeedId, Permille, Tick,
    WaterQuality,
};
use worldwake_sim::ActionTraceKind;

const TICKS: u32 = 1440;

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/survival-quality-degrading-1440.ron")
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
        .find(|(_, name)| name.0 == target_name)
        .map_or_else(
            || panic!("scenario should include named entity {target_name}"),
            |(entity, _)| entity,
        )
}

fn seed_source_location_beliefs_without_quality(h: &mut GoldenHarness, agent: EntityId) {
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );
    let mut store = h
        .world
        .get_component_agent_belief_store(agent)
        .cloned()
        .expect("seeded agent should have a belief store");
    for state in store.known_entities.values_mut() {
        if let Some(source) = state.resource_source.as_mut() {
            source.quality = None;
        }
    }
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_agent_belief_store(agent, store)
        .expect("golden harness should keep belief stores writable");
    commit_txn(txn, &mut h.event_log);
}

#[derive(Debug)]
struct QualityDegradingObservation {
    quality_observations: usize,
    muddy_quality_observations: usize,
    backup_drinks: BTreeMap<String, usize>,
    clear_ridge_drinks: BTreeMap<String, usize>,
    max_basin_dirtiness: u16,
    thirst_critical_windows: usize,
    source_failure_causes: BTreeSet<SourceFailureCause>,
    source_failure_outcomes: BTreeSet<SourceFailureOutcome>,
}

fn run() -> QualityDegradingObservation {
    let def = load_scenario_file(&scenario_path()).expect("scenario should parse");
    let spawned = spawn_scenario(&def).expect("scenario should spawn");
    let mut h = GoldenHarness::from_simulation_state(&spawned.state);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let agents = named_agents(&h);
    let backup_camp = h
        .world
        .effective_place(named_entity(&h, "Muddy Spring"))
        .expect("muddy spring should be placed at Backup Camp");
    let clear_ridge = h
        .world
        .effective_place(named_entity(&h, "Clear Ridge Well"))
        .expect("clear ridge well should be placed at Clear Ridge");
    let backup_basin = named_entity(&h, "Backup Wash Basin");
    for agent in agents.values() {
        seed_source_location_beliefs_without_quality(&mut h, *agent);
    }

    let thresholds: BTreeMap<String, _> = agents
        .iter()
        .map(|(name, agent)| {
            (
                name.clone(),
                h.world
                    .get_component_drive_thresholds(*agent)
                    .copied()
                    .expect("agent should have drive thresholds"),
            )
        })
        .collect();
    let mut extractors: BTreeMap<String, SurvivalForensicExtractor> = agents
        .iter()
        .map(|(name, agent)| (name.clone(), SurvivalForensicExtractor::new(*agent)))
        .collect();
    let mut backup_drinks = agents
        .keys()
        .cloned()
        .map(|name| (name, 0))
        .collect::<BTreeMap<_, _>>();
    let mut clear_ridge_drinks = backup_drinks.clone();
    let mut max_basin_dirtiness = 0;

    for tick_num in 0..TICKS {
        h.step_once();
        let tick = Tick(u64::from(tick_num));
        let action_sink = h.action_trace_sink().expect("action tracing enabled");

        for (name, agent) in &agents {
            for event in action_sink.events_for_at(*agent, tick) {
                if event.action_name == "drink"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
                {
                    match h.world.effective_place(*agent) {
                        Some(place) if place == backup_camp => {
                            *backup_drinks.get_mut(name).unwrap() += 1;
                        }
                        Some(place) if place == clear_ridge => {
                            *clear_ridge_drinks.get_mut(name).unwrap() += 1;
                        }
                        _ => {}
                    }
                }
            }

            let needs = h
                .world
                .get_component_homeostatic_needs(*agent)
                .copied()
                .expect("agent should always have needs");
            observe_critical_windows(
                extractors.get_mut(name).unwrap(),
                &h,
                *agent,
                tick,
                &needs,
                thresholds.get(name).unwrap(),
            );
        }

        if let Some(state) = h.world.get_component_wash_basin_state(backup_basin) {
            max_basin_dirtiness = max_basin_dirtiness.max(state.dirtiness_level.value());
        }
    }

    let mut thirst_critical_windows = 0;
    let mut source_failure_causes = BTreeSet::new();
    let mut source_failure_outcomes = BTreeSet::new();
    for extractor in extractors.into_values() {
        for report in extractor.finalize() {
            if report.need == HomeostaticNeedId::Thirst {
                thirst_critical_windows += 1;
            }
            for frame in &report.frames {
                for failure in &frame.source_acquisition_failures {
                    source_failure_causes.insert(failure.cause);
                    source_failure_outcomes.insert(failure.outcome);
                }
            }
        }
    }

    let mut quality_observations = 0;
    let mut muddy_quality_observations = 0;
    for event_id in h
        .event_log
        .events_by_tag(EventTag::ResourceSourceQualityObserved)
    {
        let record = h
            .event_log
            .get(*event_id)
            .expect("tagged event should exist");
        if let Some(DecisionEventPayload::ResourceSourceQualityObserved(payload)) =
            record.decision_payload()
        {
            quality_observations += 1;
            if payload.quality == WaterQuality::Muddy {
                muddy_quality_observations += 1;
            }
        }
    }

    QualityDegradingObservation {
        quality_observations,
        muddy_quality_observations,
        backup_drinks,
        clear_ridge_drinks,
        max_basin_dirtiness,
        thirst_critical_windows,
        source_failure_causes,
        source_failure_outcomes,
    }
}

fn run_digest() -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let def = load_scenario_file(&scenario_path()).expect("scenario should parse");
    let spawned = spawn_scenario(&def).expect("scenario should spawn");
    let mut h = GoldenHarness::from_simulation_state(&spawned.state);
    for agent in named_agents(&h).values() {
        seed_source_location_beliefs_without_quality(&mut h, *agent);
    }
    for _ in 0..TICKS {
        h.step_once();
    }
    (
        worldwake_core::hash_world(&h.world).expect("world should hash canonically"),
        worldwake_core::hash_event_log(&h.event_log).expect("event log should hash canonically"),
    )
}

// Scenario 496: Quality-Degrading Water Collision Completes 1440 Ticks
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Production, SourceReliability, WaterToleranceProfile
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity
// ActionDomains: Production, Needs, Travel
// Places: Riverside Camp, Backup Camp, Clear Ridge
// Principles: 1, 3, 7, 14, 22, 31
//
// Setup: three agents with seeded source/location beliefs share a finite clean source and can travel to muddy or clean fallback sources; unrelated trade/combat/sleep/social branches are absent.
//
// Proves: the authored 1440-tick scenario loads and runs to completion while exercising water-quality observation.
#[test]
#[ignore = "CI-only: long-running 1440-tick water-quality collision; run via golden-survival workflow"]
fn golden_survival_quality_degrading_1440_completes_1440_ticks_without_panic() {
    let obs = run();
    assert!(
        obs.quality_observations > 0,
        "run should emit source-quality observations; obs={obs:?}"
    );
}

// Scenario 497: Quality-Degrading Water Collision Records Muddy Beliefs
// ---------------------------------------------------------------------------
//
// Systems: Perception, SourceReliability
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Travel, Production
// Places: Backup Camp
// Principles: 14A, 15, 17, 22A, 31
//
// Setup: source/location beliefs are seeded, but source-reliability quality memory is empty until co-located perception observes the fallback source.
//
// Proves: the normal perception path emits `ResourceSourceQualityObserved` for Muddy water and forensics records it as a quality-rejected source-acquisition failure candidate.
#[test]
#[ignore = "CI-only: long-running 1440-tick water-quality collision; run via golden-survival workflow"]
fn golden_survival_quality_degrading_1440_records_quality_beliefs() {
    let obs = run();
    assert!(
        obs.muddy_quality_observations > 0,
        "agents should observe the muddy backup source; obs={obs:?}"
    );
    assert!(
        obs.source_failure_causes
            .contains(&SourceFailureCause::QualityRejected),
        "critical-window forensics should classify muddy observations as quality-rejected failures; obs={obs:?}"
    );
}

// Scenario 498: Quality-Degrading Water Collision Diverges By Tolerance
// ---------------------------------------------------------------------------
//
// Systems: AI, SourceReliability, WaterToleranceProfile
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity
// ActionDomains: Production, Needs, Travel
// Places: Backup Camp, Clear Ridge
// Principles: 3, 20, 22, 31
//
// Setup: Aria has high Muddy tolerance, Bram uses the default profile, and Cael has low Muddy relief plus high dirtiness penalty in the same source layout.
//
// Proves: tolerance diversity produces different committed drink locations: at least one agent drinks at the muddy backup, and at least one agent reaches the farther clean fallback for drinking.
#[test]
#[ignore = "CI-only: long-running 1440-tick water-quality collision; run via golden-survival workflow"]
fn golden_survival_quality_degrading_1440_diverges_agents_by_tolerance() {
    let obs = run();
    assert!(
        obs.backup_drinks.values().any(|count| *count > 0),
        "at least one agent should drink at the muddy backup; obs={obs:?}"
    );
    assert!(
        obs.clear_ridge_drinks.values().any(|count| *count > 0),
        "at least one agent should travel on to drink from the farther clean source; obs={obs:?}"
    );
}

// Scenario 499: Quality-Degrading Water Collision Produces Critical Window
// ---------------------------------------------------------------------------
//
// Systems: Needs, SurvivalForensicExtractor
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Production, Needs, Travel
// Places: Riverside Camp, Backup Camp, Clear Ridge
// Principles: 10, 29, 31
//
// Setup: clean-source capacity is two units with 24-tick regeneration under three thirsty agents, so depletion and travel delay push at least one thirst run into the authored critical band.
//
// Proves: the long-run collision emits thirst critical-window forensics and records a concrete drink-anyway or travel-to-fallback source outcome.
#[test]
#[ignore = "CI-only: long-running 1440-tick water-quality collision; run via golden-survival workflow"]
fn golden_survival_quality_degrading_1440_produces_critical_window() {
    let obs = run();
    assert!(
        obs.thirst_critical_windows > 0,
        "at least one thirst critical window should form; obs={obs:?}"
    );
    assert!(
        obs.source_failure_outcomes
            .contains(&SourceFailureOutcome::DrankAnyway)
            || obs
                .source_failure_outcomes
                .contains(&SourceFailureOutcome::TraveledToFallback),
        "critical windows should capture a concrete source-failure outcome; obs={obs:?}"
    );
}

// Scenario 500: Quality-Degrading Water Collision Dirties Backup Basin
// ---------------------------------------------------------------------------
//
// Systems: ItemDecay, Dirtiness
// GoalKinds: Wash
// ActionDomains: Needs
// Places: Backup Camp
// Principles: 3, 4, 26, 31
//
// Setup: the Backup Camp basin starts empty and refills from the colocated Muddy Spring, whose authored refill penalty is 90‰.
//
// Proves: basin refill from muddy water raises the basin's concrete dirtiness level during the 1440-tick run.
#[test]
#[ignore = "CI-only: long-running 1440-tick water-quality collision; run via golden-survival workflow"]
fn golden_survival_quality_degrading_1440_raises_basin_dirtiness() {
    let obs = run();
    assert!(
        obs.max_basin_dirtiness >= Permille::new(90).unwrap().value(),
        "muddy refill should raise basin dirtiness by at least the authored penalty; obs={obs:?}"
    );
}

// Scenario 501: Quality-Degrading Water Collision Replay Is Deterministic
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Production, Perception
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity
// ActionDomains: Production, Needs, Travel
// Places: Riverside Camp, Backup Camp, Clear Ridge
// Principles: 2, 9, 31
#[test]
#[ignore = "CI-only: long-running 1440-tick replay-equivalence check"]
fn golden_survival_quality_degrading_1440_replays_deterministically() {
    assert_eq!(
        run_digest(),
        run_digest(),
        "quality-degrading collision should replay deterministically"
    );
}
