//! Golden tests for S131 source reliability wait and capacity learning.

mod golden_harness;

use std::collections::BTreeMap;
use std::num::{NonZeroU8, NonZeroU32};

use golden_harness::*;
use worldwake_ai::{DecisionOutcome, decision_trace::SourceReliabilityDiscount};
use worldwake_core::{
    AcquisitionQuantity, AgentData, BeliefConfidencePolicy, CommodityKind, CommodityPurpose,
    ControlSource, EntityId, GoalKind, HomeostaticNeeds, MetabolismProfile, PerceptionProfile,
    PerceptionSource, Permille, Place, PlaceTag, PreferenceProfile, ProductionOutputOwner,
    Quantity, ReliabilityRecord, ResourceSource, Seed, SourceKey, SourceReliability, Tick,
    Topology, TravelEdge, TravelEdgeId, UtilityProfile, WorkstationTag, World,
};
use worldwake_sim::{ActionTraceKind, Scheduler, SystemManifest};

const START: EntityId = entity(610);
const CLOSE_ORCHARD: EntityId = entity(611);
const FAR_ORCHARD: EntityId = entity(612);

const fn entity(slot: u32) -> EntityId {
    EntityId {
        slot,
        generation: 0,
    }
}

fn place(name: &str, tags: &[PlaceTag]) -> Place {
    Place {
        name: name.to_string(),
        capacity: None,
        tags: tags.iter().copied().collect(),
    }
}

fn connect(topology: &mut Topology, edge: u32, from: EntityId, to: EntityId, ticks: u32) {
    topology
        .add_edge(TravelEdge::new(TravelEdgeId(edge), from, to, ticks, None).unwrap())
        .unwrap();
    topology
        .add_edge(TravelEdge::new(TravelEdgeId(edge + 1), to, from, ticks, None).unwrap())
        .unwrap();
}

fn source_reliability_topology() -> Topology {
    let mut topology = Topology::new();
    topology
        .add_place(START, place("S131 Market", &[PlaceTag::Village]))
        .unwrap();
    topology
        .add_place(
            CLOSE_ORCHARD,
            place("S131 Close Orchard", &[PlaceTag::Farm, PlaceTag::Field]),
        )
        .unwrap();
    topology
        .add_place(
            FAR_ORCHARD,
            place("S131 Far Orchard", &[PlaceTag::Farm, PlaceTag::Field]),
        )
        .unwrap();
    connect(&mut topology, 6100, START, CLOSE_ORCHARD, 1);
    connect(&mut topology, 6110, START, FAR_ORCHARD, 2);
    topology
}

fn build_source_harness(seed: Seed) -> GoldenHarness {
    let mut h = GoldenHarness::new(seed);
    h.world = World::new(source_reliability_topology()).unwrap();
    h.event_log = worldwake_core::EventLog::new();
    h.scheduler = Scheduler::new(SystemManifest::canonical());
    h.controller = worldwake_sim::ControllerState::new();
    h
}

fn full_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        entity_activation_threshold: pm(64),
        claim_confidence_threshold: pm(50),
        observation_buffer_capacity: 64,
        observation_budget: 32,
        need_salience_boost: pm(500),
        need_salience_urgency_threshold: pm(500),
        observation_fidelity: pm(1000),
        confidence_policy: BeliefConfidencePolicy::default(),
        institutional_memory_capacity: 20,
        consultation_speed_factor: pm(500),
        contradiction_tolerance: pm(300),
    }
}

fn source_preference(wait_sensitivity_weight: u16) -> PreferenceProfile {
    PreferenceProfile {
        route_caution_weight: Permille::ZERO,
        source_trust_weight: Permille::ZERO,
        route_memory_capacity: 8,
        source_memory_capacity: 8,
        memory_retention_ticks: 400,
        wait_sensitivity_weight: pm(wait_sensitivity_weight),
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

fn set_preference(h: &mut GoldenHarness, agent: EntityId, profile: PreferenceProfile) {
    let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
    txn.set_component_preference_profile(agent, profile)
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn set_source_reliability(
    h: &mut GoldenHarness,
    agent: EntityId,
    records: BTreeMap<SourceKey, ReliabilityRecord>,
) {
    let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
    txn.set_component_source_reliability(agent, SourceReliability { sources: records })
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

fn selected_apple_anchor(
    h: &GoldenHarness,
    agent: EntityId,
) -> Option<worldwake_core::OpportunityAnchor> {
    h.driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .traces_for(agent)
        .into_iter()
        .rev()
        .find_map(|trace| match &trace.outcome {
            DecisionOutcome::Planning(planning)
                if planning.selection.selected_goal().is_some_and(|goal| {
                    goal.kind
                        == GoalKind::AcquireCommodity {
                            commodity: CommodityKind::Apple,
                            purpose: CommodityPurpose::SelfConsume,
                            quantity: AcquisitionQuantity::single(),
                        }
                }) =>
            {
                planning
                    .selection
                    .selected_opportunity
                    .map(|opportunity| opportunity.anchor)
            }
            _ => None,
        })
}

fn ranked_discount_for(
    h: &GoldenHarness,
    agent: EntityId,
    source: EntityId,
) -> Option<SourceReliabilityDiscount> {
    h.driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .traces_for(agent)
        .into_iter()
        .rev()
        .find_map(|trace| match &trace.outcome {
            DecisionOutcome::Planning(planning) => {
                planning.candidates.ranked.iter().find_map(|ranked| {
                    ranked
                        .source_reliability_discount
                        .as_ref()
                        .and_then(|discount| {
                            (discount.source_entity == source
                                && discount.commodity == CommodityKind::Apple)
                                .then_some(discount.clone())
                        })
                })
            }
            _ => None,
        })
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

// Scenario 139: Fresh Capacity Signal Reaches Ranking
// ---------------------------------------------------------------------------
//
// Systems: SourceReliability, AI ranking, decision trace
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Travel, Production
// Places: S131 Market, S131 Close Orchard
// Principles: 3, 15, 27, 29
//
// Setup: A hungry agent at the market knows a remote orchard and has a fresh
//   capacity observation for that source.
//
// Proves: The next planning pass reads the stored capacity observation and
//   emits a positive capacity signal in the ranked goal decision trace.
//
// Chain: Stored capacity memory -> AcquireCommodity candidate -> ranking
//   composite -> decision trace SourceReliabilityDiscount.
#[test]
fn capacity_signal_within_retention_window_contributes_to_motive() {
    let mut h = build_source_harness(Seed([139; 32]));
    let close = orchard_source(&mut h, CLOSE_ORCHARD, 18);
    let agent = hungry_agent(&mut h, "Fresh Capacity", START);
    set_preference(&mut h, agent, source_preference(150));
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        PerceptionSource::Inference,
    );
    let mut record = ReliabilityRecord::new(Tick(0));
    record.observe_capacity(18, Tick(0));
    set_source_reliability(
        &mut h,
        agent,
        BTreeMap::from([(
            SourceKey {
                entity: close,
                commodity: CommodityKind::Apple,
            },
            record,
        )]),
    );

    h.driver.enable_tracing();
    h.step_once();

    let discount = ranked_discount_for(&h, agent, close)
        .expect("fresh capacity should surface a source reliability discount");
    assert!(discount.capacity_signal > 0);
    assert!(discount.post_discount_motive > discount.pre_discount_motive);
}

// Scenario 140: Stale Capacity Signal Is Discounted To Zero
// ---------------------------------------------------------------------------
//
// Systems: SourceReliability, AI ranking, decision trace
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Travel, Production
// Places: S131 Market, S131 Close Orchard
// Principles: 16, 27, 29A
//
// Setup: A hungry agent's old capacity observation is older than
//   PreferenceProfile.memory_retention_ticks before AI control resumes.
//
// Proves: Ranking preserves the stored observation but gives it zero
//   capacity_signal once it is stale.
//
// Chain: Old capacity memory -> delayed AI tick -> ranking composite ->
//   decision trace with cap_sig=0.
#[test]
fn capacity_freshness_zeros_signal_after_retention_window() {
    let mut h = build_source_harness(Seed([140; 32]));
    let close = orchard_source(&mut h, CLOSE_ORCHARD, 18);
    let agent = hungry_agent(&mut h, "Stale Capacity", START);
    let mut blind_profile = full_perception_profile();
    blind_profile.observation_budget = 0;
    set_agent_perception_profile(&mut h.world, &mut h.event_log, agent, blind_profile);
    set_control_source(&mut h, agent, ControlSource::None);
    set_preference(&mut h, agent, source_preference(150));
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        PerceptionSource::Inference,
    );
    for _ in 0..405 {
        h.step_once();
    }
    let current_tick = h.scheduler.current_tick();
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        current_tick,
        PerceptionSource::Inference,
    );
    let mut record = ReliabilityRecord::new(current_tick);
    record.observe_capacity(18, Tick(0));
    record.observe_wait(10);
    set_source_reliability(
        &mut h,
        agent,
        BTreeMap::from([(
            SourceKey {
                entity: close,
                commodity: CommodityKind::Apple,
            },
            record,
        )]),
    );
    set_control_source(&mut h, agent, ControlSource::Ai);

    h.driver.enable_tracing();
    h.step_once();

    let discount = ranked_discount_for(&h, agent, close).unwrap_or_else(|| {
        panic!(
            "stale capacity should still trace once another axis is present; selected={:?}; reliability={:?}; traces={:?}",
            selected_apple_anchor(&h, agent),
            h.world.get_component_source_reliability(agent),
            h.driver
                .trace_sink()
                .expect("decision tracing should be enabled")
                .traces_for(agent)
                .into_iter()
                .map(|trace| format!("{:?}: {}", trace.tick, trace.outcome.summary()))
                .collect::<Vec<_>>()
        )
    });
    assert!(discount.capacity_freshness_ticks > 400);
    assert_eq!(discount.last_observed_capacity, 18);
    assert_eq!(discount.capacity_signal, 0);
    assert!(discount.wait_penalty > 0);
}

// Scenario 141: Wait Memory Re-Ranks Source Choice
// ---------------------------------------------------------------------------
//
// Systems: SourceReliability, AI ranking, decision trace
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Travel, Production
// Places: S131 Market, S131 Close Orchard, S131 Far Orchard
// Principles: 21, 22, 27, 29
//
// Setup: The close orchard is initially the preferred apple source. The same
//   agent then retains repeated wait observations for the close orchard while
//   the farther orchard has no wait history.
//
// Proves: A high wait_sensitivity_weight causes the next ranking pass to pick
//   the farther source and exposes the close source's wait penalty in the
//   decision trace.
//
// Chain: Stored wait memory -> next AI planning pass -> composite ranking ->
//   selected OpportunityAnchor changes to the alternative source.
#[test]
fn high_wait_sensitivity_agent_prefers_alternative_after_three_wait_observations() {
    let mut baseline = build_source_harness(Seed([141; 32]));
    let _close_baseline = orchard_source(&mut baseline, CLOSE_ORCHARD, 10);
    let _far_baseline = orchard_source(&mut baseline, FAR_ORCHARD, 10);
    let baseline_agent = hungry_agent(&mut baseline, "Baseline", START);
    set_preference(&mut baseline, baseline_agent, source_preference(800));
    seed_actor_world_beliefs(
        &mut baseline.world,
        &mut baseline.event_log,
        baseline_agent,
        Tick(0),
        PerceptionSource::Inference,
    );
    baseline.driver.enable_tracing();
    baseline.step_once();
    assert_eq!(
        selected_apple_anchor(&baseline, baseline_agent),
        Some(worldwake_core::OpportunityAnchor::Place(CLOSE_ORCHARD)),
        "without wait memory, the closer orchard should be selected",
    );

    let mut h = build_source_harness(Seed([142; 32]));
    let close = orchard_source(&mut h, CLOSE_ORCHARD, 10);
    let _far = orchard_source(&mut h, FAR_ORCHARD, 10);
    let agent = hungry_agent(&mut h, "Learner", START);
    set_preference(&mut h, agent, source_preference(800));
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        PerceptionSource::Inference,
    );
    let mut record = ReliabilityRecord::new(Tick(0));
    record.observe_wait(30);
    record.observe_wait(30);
    record.observe_wait(30);
    set_source_reliability(
        &mut h,
        agent,
        BTreeMap::from([(
            SourceKey {
                entity: close,
                commodity: CommodityKind::Apple,
            },
            record,
        )]),
    );

    h.driver.enable_tracing();
    h.step_once();

    assert_eq!(
        selected_apple_anchor(&h, agent),
        Some(worldwake_core::OpportunityAnchor::Place(FAR_ORCHARD)),
        "high wait sensitivity should make the farther uncontested source preferable",
    );
    let discount = ranked_discount_for(&h, agent, close)
        .expect("close source should carry a wait penalty trace");
    assert_eq!(discount.average_wait_ticks, 30);
    assert_eq!(discount.wait_penalty, 24);
    assert!(discount.post_discount_motive < discount.pre_discount_motive);
}
