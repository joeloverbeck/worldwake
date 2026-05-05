//! Golden tests for S131 source reliability wait and capacity learning data
//! paths. The motive-additive composite (S131SOURELWAI-004) was rolled back
//! after it perturbed cross-category goal ranking; the data substrate
//! (perception capacity write, queue wait observation) remains live and is
//! exercised by the goldens here. S132 reauthors the source-comparison
//! goldens against a same-commodity sub-rank tiebreaker — see
//! specs/S132-source-composite-tiebreaker.md.

mod golden_harness;

use std::num::{NonZeroU8, NonZeroU32};

use golden_harness::*;
use worldwake_core::{
    AgentData, BeliefConfidencePolicy, CommodityKind, ControlSource, EntityId, HomeostaticNeeds,
    MetabolismProfile, PerceptionProfile, PerceptionSource, ProductionOutputOwner, Quantity,
    ResourceSource, Seed, SourceKey, Tick, UtilityProfile, WorkstationTag,
};
use worldwake_sim::ActionTraceKind;

fn full_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        entity_activation_threshold: pm(64),
        claim_confidence_threshold: pm(50),
        observation_buffer_capacity: 64,
        observation_budget: 32,
        salience_policy: worldwake_core::SaliencePolicy::default(),
        omission_log_capacity: worldwake_core::default_omission_log_capacity(),
        need_salience_boost: pm(500),
        need_salience_urgency_threshold: pm(500),
        observation_fidelity: pm(1000),
        confidence_policy: BeliefConfidencePolicy::default(),
        institutional_memory_capacity: 20,
        consultation_speed_factor: pm(500),
        contradiction_tolerance: pm(300),
    }
}

fn hungry_agent(h: &mut GoldenHarness, name: &str, place: EntityId) -> EntityId {
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        name,
        place,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile {
            hunger_weight: pm(800),
            ..UtilityProfile::default()
        },
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        full_perception_profile(),
    );
    agent
}

fn set_control_source(h: &mut GoldenHarness, agent: EntityId, control_source: ControlSource) {
    let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
    txn.set_component_agent_data(agent, AgentData { control_source })
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn orchard_source(h: &mut GoldenHarness, place: EntityId, quantity: u32) -> EntityId {
    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        place,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(quantity),
            max_quantity: Quantity(quantity),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
            extraction_slots: NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: NonZeroU32::new(3).unwrap(),
        },
        ProductionOutputOwner::Actor,
    )
}

fn tick_until(
    h: &mut GoldenHarness,
    limit: usize,
    mut done: impl FnMut(&GoldenHarness) -> bool,
) -> bool {
    for _ in 0..limit {
        if done(h) {
            return true;
        }
        h.step_once();
    }
    done(h)
}

// Scenario 137: Resource Extraction Grant Writes Wait Memory
// ---------------------------------------------------------------------------
//
// Systems: AI, Production, SourceReliability
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Production
// Places: OrchardFarm
// Principles: 8, 14A, 26
//
// Setup: Two hungry agents compete for one orchard extraction slot.
//
// Proves: The losing agent's real queued harvest start is later promoted by
//   the resource-extraction grant path, and the grant writes an average wait
//   observation into that actor's SourceReliability.
//
// Chain: AI harvest request -> slot full start failure -> queue entry -> first
//   harvest commit releases slot -> queued actor re-requests -> grant writes
//   wait memory.
#[test]
fn resource_extraction_wait_observation_records_when_promoted() {
    let mut h = GoldenHarness::new(Seed([137; 32]));
    h.enable_action_tracing();
    let orchard = orchard_source(&mut h, ORCHARD_FARM, 8);
    let agent_a = hungry_agent(&mut h, "Aren", ORCHARD_FARM);
    let agent_b = hungry_agent(&mut h, "Bora", ORCHARD_FARM);
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent_a,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent_b,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    h.step_once();
    let queues = h
        .world
        .get_component_resource_extraction_queues(orchard)
        .expect("orchard should have extraction queues");
    let granted_actor = queues.queues[0]
        .granted
        .as_ref()
        .map(|grant| grant.actor)
        .expect("one agent should hold the single extraction slot");
    let queued_actor = queues.queues[0]
        .waiting
        .values()
        .next()
        .map(|waiter| waiter.actor)
        .expect("the other agent should be queued");
    assert_ne!(granted_actor, queued_actor);

    let first_committed = tick_until(&mut h, 12, |h| {
        h.action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(granted_actor)
            .iter()
            .any(|event| matches!(event.kind, ActionTraceKind::Committed { .. }))
    });
    assert!(first_committed, "first granted harvest should commit");

    let wait_recorded = tick_until(&mut h, 12, |h| {
        h.world
            .get_component_source_reliability(queued_actor)
            .and_then(|reliability| {
                reliability.sources.get(&SourceKey {
                    entity: orchard,
                    commodity: CommodityKind::Apple,
                })
            })
            .is_some_and(|record| {
                record.wait_observation_count == 1 && record.average_wait_ticks > 0
            })
    });
    assert!(
        wait_recorded,
        "queued actor should record a positive wait observation after promotion; queues={:?}",
        h.world.get_component_resource_extraction_queues(orchard)
    );
}

// Scenario 138: Perception Writes Capacity Memory
// ---------------------------------------------------------------------------
//
// Systems: Perception, SourceReliability
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Production
// Places: OrchardFarm
// Principles: 14A, 15, 26
//
// Setup: A hungry agent is co-located with a visible orchard source.
//
// Proves: The normal perception tick records the source's observed capacity
//   into the agent's SourceReliability for the same (source, commodity) key
//   used by ranking.
//
// Chain: Co-located resource source -> perception batch -> SourceReliability
//   capacity observation.
#[test]
fn capacity_observation_records_from_perception() {
    let mut h = GoldenHarness::new(Seed([138; 32]));
    let orchard = orchard_source(&mut h, ORCHARD_FARM, 18);
    let agent = hungry_agent(&mut h, "Cap Observer", ORCHARD_FARM);
    set_control_source(&mut h, agent, ControlSource::None);

    h.step_once();

    let record = h
        .world
        .get_component_source_reliability(agent)
        .and_then(|reliability| {
            reliability.sources.get(&SourceKey {
                entity: orchard,
                commodity: CommodityKind::Apple,
            })
        })
        .copied()
        .expect("perception should write capacity source reliability");
    assert_eq!(record.last_observed_capacity, 18);
    assert_eq!(record.last_observed_capacity_tick, Tick(0));
    assert_eq!(record.successful_acquisitions, 0);
    assert_eq!(record.failed_attempts, 0);
}
