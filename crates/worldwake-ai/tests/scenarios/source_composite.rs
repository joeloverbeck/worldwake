//! Golden tests for S133 source-composite same-commodity tiebreaking.

use std::collections::BTreeMap;
use std::num::{NonZeroU8, NonZeroU32};

use crate::golden_harness::*;
use worldwake_ai::{CommodityPurpose, DecisionOutcome, RankedGoalComparisonDimension};
use worldwake_core::{
    AcquisitionQuantity, AgentData, BeliefConfidencePolicy, CommodityKind, ControlSource, EntityId,
    EventLog, GoalKey, GoalKind, HomeostaticNeeds, MetabolismProfile, PerceptionProfile,
    PerceptionSource, Permille, Place, PlaceTag, PreferenceProfile, Quantity, ReliabilityRecord,
    ResourceSource, Seed, SourceKey, SourceReliability, Tick, Topology, TravelEdge, TravelEdgeId,
    UtilityProfile, WashBasinState, WorkstationTag, World,
};
use worldwake_sim::{ControllerState, Scheduler, SystemManifest};

const HOME: EntityId = entity(13_300);
const CLOSE_ORCHARD: EntityId = entity(13_301);
const FAR_ORCHARD: EntityId = entity(13_302);
const LOW_CAPACITY_ORCHARD: EntityId = entity(13_303);

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

fn connect(topology: &mut Topology, base_id: u32, from: EntityId, to: EntityId, ticks: u32) {
    topology
        .add_edge(TravelEdge::new(TravelEdgeId(base_id), from, to, ticks, None).unwrap())
        .unwrap();
    topology
        .add_edge(TravelEdge::new(TravelEdgeId(base_id + 1), to, from, ticks, None).unwrap())
        .unwrap();
}

fn source_composite_topology() -> Topology {
    let mut topology = Topology::new();
    topology
        .add_place(HOME, place("Source Composite Home", &[PlaceTag::Village]))
        .unwrap();
    topology
        .add_place(
            CLOSE_ORCHARD,
            place("Close Orchard", &[PlaceTag::Farm, PlaceTag::Field]),
        )
        .unwrap();
    topology
        .add_place(
            FAR_ORCHARD,
            place("Far Orchard", &[PlaceTag::Farm, PlaceTag::Field]),
        )
        .unwrap();
    topology
        .add_place(
            LOW_CAPACITY_ORCHARD,
            place("Low Capacity Orchard", &[PlaceTag::Farm, PlaceTag::Field]),
        )
        .unwrap();
    connect(&mut topology, 13_310, HOME, CLOSE_ORCHARD, 2);
    connect(&mut topology, 13_320, HOME, FAR_ORCHARD, 2);
    connect(&mut topology, 13_330, HOME, LOW_CAPACITY_ORCHARD, 2);
    topology
}

fn harness(seed: Seed) -> GoldenHarness {
    let mut h = GoldenHarness::new(seed);
    h.world = World::new(source_composite_topology()).unwrap();
    h.event_log = EventLog::new();
    h.scheduler = Scheduler::new(SystemManifest::canonical());
    h.controller = ControllerState::new();
    h.driver.enable_tracing();
    h
}

fn quiet_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        entity_activation_threshold: pm(64),
        claim_confidence_threshold: pm(50),
        observation_buffer_capacity: 64,
        observation_budget: 0,
        salience_policy: worldwake_core::SaliencePolicy::default(),
        omission_log_capacity: worldwake_core::default_omission_log_capacity(),
        opportunity_floor_permille: worldwake_core::default_opportunity_floor_permille(),
        need_salience_boost: pm(500),
        need_salience_urgency_threshold: pm(500),
        observation_fidelity: pm(1000),
        confidence_policy: BeliefConfidencePolicy::default(),
        institutional_memory_capacity: 20,
        consultation_speed_factor: pm(500),
        contradiction_tolerance: pm(300),
    }
}

fn source_profile(wait_sensitivity_weight: u16, memory_retention_ticks: u64) -> PreferenceProfile {
    PreferenceProfile {
        route_caution_weight: pm(0),
        source_trust_weight: pm(0),
        route_memory_capacity: 8,
        source_memory_capacity: 8,
        memory_retention_ticks,
        wait_sensitivity_weight: pm(wait_sensitivity_weight),
        capacity_observation_weight: Permille::new(20).unwrap(),
    }
}

fn set_preference_profile(h: &mut GoldenHarness, agent: EntityId, profile: PreferenceProfile) {
    let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
    txn.set_component_preference_profile(agent, profile)
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn set_control_source(h: &mut GoldenHarness, agent: EntityId, control_source: ControlSource) {
    let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
    txn.set_component_agent_data(agent, AgentData { control_source })
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn set_source_reliability(
    h: &mut GoldenHarness,
    agent: EntityId,
    source_reliability: SourceReliability,
) {
    let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
    txn.set_component_source_reliability(agent, source_reliability)
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn hungry_agent(
    h: &mut GoldenHarness,
    name: &str,
    hunger: u16,
    dirtiness: u16,
    hunger_weight: u16,
    dirtiness_weight: u16,
) -> EntityId {
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        name,
        HOME,
        HomeostaticNeeds::new(pm(hunger), pm(0), pm(0), pm(0), pm(dirtiness)),
        MetabolismProfile::default(),
        UtilityProfile {
            hunger_weight: pm(hunger_weight),
            dirtiness_weight: pm(dirtiness_weight),
            ..UtilityProfile::default()
        },
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        quiet_perception_profile(),
    );
    agent
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
            quality: None,
        },
        ProductionOutputOwner::Actor,
    )
}

fn place_wash_basin(h: &mut GoldenHarness) -> EntityId {
    let basin = place_workstation(
        &mut h.world,
        &mut h.event_log,
        HOME,
        WorkstationTag::WashBasin,
        ProductionOutputOwner::Actor,
    );
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_wash_basin_state(
        basin,
        WashBasinState {
            clean_water_units: 4,
            max_clean_water: 4,
            refill_per_tick: 0,
            units_per_full_wash: 2,
            dirtiness_level: pm(0),
            dirtiness_per_use: pm(50),
            max_effective_dirtiness: pm(1000),
            ..WashBasinState::default()
        },
    )
    .unwrap();
    commit_txn(txn, &mut h.event_log);
    basin
}

fn record(
    average_wait_ticks: u32,
    wait_observation_count: u32,
    last_observed_capacity: u16,
    last_observed_capacity_tick: Tick,
) -> ReliabilityRecord {
    ReliabilityRecord {
        successful_acquisitions: 1,
        failed_attempts: 0,
        last_attempt_tick: last_observed_capacity_tick,
        provenance_events: [None; 8],
        average_wait_ticks,
        wait_observation_count,
        last_observed_capacity,
        last_observed_capacity_tick,
        last_observed_quality: None,
        last_observed_quality_tick: Tick(0),
    }
}

fn source_reliability(records: &[(EntityId, ReliabilityRecord)]) -> SourceReliability {
    SourceReliability {
        sources: records
            .iter()
            .map(|(source, record)| {
                (
                    SourceKey {
                        entity: *source,
                        commodity: CommodityKind::Apple,
                    },
                    *record,
                )
            })
            .collect::<BTreeMap<_, _>>(),
    }
}

fn seed_world_beliefs(h: &mut GoldenHarness, agent: EntityId) {
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        h.scheduler.current_tick(),
        PerceptionSource::Inference,
    );
}

fn tick_to(h: &mut GoldenHarness, target_tick: u64) {
    while h.scheduler.current_tick().0 < target_tick {
        h.step_once();
    }
}

fn planning(
    h: &GoldenHarness,
    agent: EntityId,
    tick: Tick,
) -> &worldwake_ai::PlanningPipelineTrace {
    let trace = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .trace_at(agent, tick)
        .unwrap_or_else(|| panic!("expected decision trace at {tick:?}"));
    let DecisionOutcome::Planning(planning) = &trace.outcome else {
        panic!(
            "expected planning trace at {tick:?}; outcome={:?}",
            trace.outcome
        );
    };
    planning
}

fn acquire_apple_goal() -> GoalKey {
    GoalKey::from(GoalKind::AcquireCommodity {
        commodity: CommodityKind::Apple,
        purpose: CommodityPurpose::SelfConsume,
        quantity: AcquisitionQuantity::single(),
    })
}

fn source_rank(
    planning: &worldwake_ai::PlanningPipelineTrace,
    source: EntityId,
) -> &worldwake_ai::RankedGoalSummary {
    planning
        .candidates
        .ranked
        .iter()
        .find(|summary| {
            summary.opportunity.goal_key == acquire_apple_goal()
                && summary
                    .source_composite
                    .is_some_and(|rank| rank.source_entity == source)
        })
        .unwrap_or_else(|| {
            panic!(
                "expected ranked AcquireCommodity Apple summary for source {source}; ranked={:?}",
                planning.candidates.ranked
            )
        })
}

fn top_ranked_source(planning: &worldwake_ai::PlanningPipelineTrace) -> EntityId {
    planning
        .candidates
        .ranked
        .first()
        .and_then(|summary| summary.source_composite.map(|rank| rank.source_entity))
        .unwrap_or_else(|| {
            panic!(
                "top ranked opportunity should have source composite; ranked={:?}",
                planning.candidates.ranked
            )
        })
}

fn assert_top_comparison_dimension(
    planning: &worldwake_ai::PlanningPipelineTrace,
    dimension: RankedGoalComparisonDimension,
) {
    assert_eq!(
        planning
            .candidates
            .top_ranked_comparison
            .as_ref()
            .map(|comparison| comparison.decisive_dimension),
        Some(dimension),
        "unexpected top comparison; ranked={:?}",
        planning.candidates.ranked
    );
}

// Scenario 375: Source Composite Wait Reranks Same-Commodity Siblings
// Systems: AI, SourceReliability
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Production, Travel
// Places: SourceCompositeHome, CloseOrchard, FarOrchard
// Principles: 3, 15, 20, 26
// Setup: Two equal-distance apple sources; only the close source has three
//   remembered wait observations.
// Proves: The same-commodity comparator selects the source with the neutral
//   wait factor and attributes the choice to SourceComposite.
#[test]
fn same_commodity_wait_reranking_picks_far_orchard_when_close_orchard_has_observed_waits() {
    let mut h = harness(Seed([139; 32]));
    let close = orchard_source(&mut h, CLOSE_ORCHARD, 12);
    let far = orchard_source(&mut h, FAR_ORCHARD, 12);
    let agent = hungry_agent(&mut h, "Wait Learner", 900, 0, 800, 0);
    set_preference_profile(&mut h, agent, source_profile(800, 400));
    set_source_reliability(
        &mut h,
        agent,
        source_reliability(&[(close, record(30, 3, 1, Tick(0)))]),
    );
    seed_world_beliefs(&mut h, agent);

    h.step_once();
    let planning = planning(&h, agent, Tick(0));
    assert_eq!(top_ranked_source(planning), far);
    assert_top_comparison_dimension(planning, RankedGoalComparisonDimension::SourceComposite);
    assert!(
        source_rank(planning, close)
            .source_composite
            .unwrap()
            .wait_factor_permille
            < 1000
    );
    assert_eq!(
        source_rank(planning, far)
            .source_composite
            .unwrap()
            .composite_permille,
        1000
    );
}

// Scenario 376: Source Composite Does Not Cross Goal Categories
// Systems: AI, SourceReliability, Dirtiness
// GoalKinds: Wash, AcquireCommodity(SelfConsume)
// ActionDomains: Needs, Production
// Places: SourceCompositeHome, FarOrchard
// Principles: 3, 20, 26
// Setup: A dirty, mildly hungry agent knows a high-capacity apple source and a
//   usable local wash basin.
// Proves: Wash wins before SourceComposite is eligible; SourceComposite
//   remains intra-commodity.
#[test]
fn cross_category_neutrality_keeps_wash_above_acquire_apple_when_motive_higher() {
    let mut h = harness(Seed([140; 32]));
    let orchard = orchard_source(&mut h, FAR_ORCHARD, 20);
    let _basin = place_wash_basin(&mut h);
    let agent = hungry_agent(&mut h, "Dirty Learner", 350, 900, 250, 900);
    set_preference_profile(&mut h, agent, source_profile(800, 400));
    set_source_reliability(
        &mut h,
        agent,
        source_reliability(&[(orchard, record(0, 1, 20, Tick(0)))]),
    );
    seed_world_beliefs(&mut h, agent);

    h.step_once();
    let planning = planning(&h, agent, Tick(0));
    assert_eq!(
        planning.selection.selected_goal().map(|goal| goal.kind),
        Some(GoalKind::Wash)
    );
    assert_top_comparison_dimension(planning, RankedGoalComparisonDimension::PriorityClass);
    assert_ne!(
        planning
            .candidates
            .top_ranked_comparison
            .as_ref()
            .map(|comparison| comparison.decisive_dimension),
        Some(RankedGoalComparisonDimension::SourceComposite)
    );
}

// Scenario 377: Fresh Capacity Bonus Reranks Same-Commodity Siblings
// Systems: AI, SourceReliability
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Production, Travel
// Places: SourceCompositeHome, FarOrchard, LowCapacityOrchard
// Principles: 3, 15, 20, 26
// Setup: Two equal-distance apple sources have fresh observed capacities of
//   18 and 4 units.
// Proves: The higher-capacity source wins through SourceComposite.
#[test]
fn fresh_capacity_bonus_picks_higher_capacity_orchard_when_motive_tied() {
    let mut h = harness(Seed([141; 32]));
    let high = orchard_source(&mut h, FAR_ORCHARD, 18);
    let low = orchard_source(&mut h, LOW_CAPACITY_ORCHARD, 4);
    let agent = hungry_agent(&mut h, "Capacity Learner", 900, 0, 800, 0);
    set_preference_profile(&mut h, agent, source_profile(0, 400));
    set_source_reliability(
        &mut h,
        agent,
        source_reliability(&[
            (high, record(0, 1, 18, Tick(0))),
            (low, record(0, 1, 4, Tick(0))),
        ]),
    );
    seed_world_beliefs(&mut h, agent);

    h.step_once();
    let planning = planning(&h, agent, Tick(0));
    assert_eq!(top_ranked_source(planning), high);
    assert_top_comparison_dimension(planning, RankedGoalComparisonDimension::SourceComposite);
    assert!(
        source_rank(planning, high)
            .source_composite
            .unwrap()
            .capacity_factor_permille
            > 1000
    );
    let high_capacity_factor = source_rank(planning, high)
        .source_composite
        .unwrap()
        .capacity_factor_permille;
    let low_capacity_factor = source_rank(planning, low)
        .source_composite
        .unwrap()
        .capacity_factor_permille;
    assert!(low_capacity_factor > 1000);
    assert!(low_capacity_factor < high_capacity_factor);
}

// Scenario 378: Stale Capacity Observation Is Neutral
// Systems: AI, SourceReliability
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Production, Travel
// Places: SourceCompositeHome, FarOrchard, LowCapacityOrchard
// Principles: 3, 15, 16, 26
// Setup: One source has an old high-capacity observation; the other has a
//   fresh low-capacity observation.
// Proves: The stale observation contributes `capacity_factor_permille = 1000`.
#[test]
fn stale_capacity_observation_returns_neutral_factor() {
    let mut h = harness(Seed([142; 32]));
    let stale_high = orchard_source(&mut h, FAR_ORCHARD, 18);
    let fresh_low = orchard_source(&mut h, LOW_CAPACITY_ORCHARD, 4);
    let agent = hungry_agent(&mut h, "Stale Capacity Learner", 900, 0, 800, 0);
    set_control_source(&mut h, agent, ControlSource::None);
    set_preference_profile(&mut h, agent, source_profile(0, 10));
    set_source_reliability(
        &mut h,
        agent,
        source_reliability(&[
            (stale_high, record(0, 1, 18, Tick(0))),
            (fresh_low, record(0, 1, 4, Tick(11))),
        ]),
    );
    tick_to(&mut h, 11);
    set_control_source(&mut h, agent, ControlSource::Ai);
    seed_world_beliefs(&mut h, agent);

    h.step_once();
    let planning = planning(&h, agent, Tick(11));
    assert_eq!(
        source_rank(planning, stale_high)
            .source_composite
            .unwrap()
            .capacity_factor_permille,
        1000
    );
    assert_eq!(top_ranked_source(planning), fresh_low);
    assert_top_comparison_dimension(planning, RankedGoalComparisonDimension::SourceComposite);
}

// Scenario 379: Empty Fresh Capacity Demotes Source
// Systems: AI, SourceReliability
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Production, Travel
// Places: SourceCompositeHome, FarOrchard, LowCapacityOrchard
// Principles: 3, 15, 16, 26
// Setup: Two equal-distance apple sources are both observed, but one fresh
//   observation says the source is empty.
// Proves: The empty-observed source ranks lower while remaining a candidate.
#[test]
fn empty_but_fresh_observation_demotes_depleted_source() {
    let mut h = harness(Seed([143; 32]));
    let empty = orchard_source(&mut h, FAR_ORCHARD, 4);
    let available = orchard_source(&mut h, LOW_CAPACITY_ORCHARD, 4);
    let agent = hungry_agent(&mut h, "Empty Capacity Learner", 900, 0, 800, 0);
    set_preference_profile(&mut h, agent, source_profile(0, 400));
    set_source_reliability(
        &mut h,
        agent,
        source_reliability(&[
            (empty, record(0, 1, 0, Tick(0))),
            (available, record(0, 1, 4, Tick(0))),
        ]),
    );
    seed_world_beliefs(&mut h, agent);

    h.step_once();
    let planning = planning(&h, agent, Tick(0));
    assert_eq!(top_ranked_source(planning), available);
    assert_top_comparison_dimension(planning, RankedGoalComparisonDimension::SourceComposite);
    let empty_capacity_factor = source_rank(planning, empty)
        .source_composite
        .unwrap()
        .capacity_factor_permille;
    assert!((500..1000).contains(&empty_capacity_factor));
}

// Scenario 380: Missing Source Records Are Neutral
// Systems: AI, SourceReliability
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Production, Travel
// Places: SourceCompositeHome, CloseOrchard, FarOrchard
// Principles: 3, 15, 16, 26
// Setup: A fresh agent knows two equal-distance apple sources but has no
//   reliability record for either source.
// Proves: Both sources receive neutral source-composite factors; SourceComposite
//   does not decide the final ordering.
#[test]
fn no_record_neutrality_falls_through_to_lower_tiebreakers() {
    let mut h = harness(Seed([144; 32]));
    let close = orchard_source(&mut h, CLOSE_ORCHARD, 12);
    let far = orchard_source(&mut h, FAR_ORCHARD, 12);
    let agent = hungry_agent(&mut h, "Neutral Learner", 900, 0, 800, 0);
    set_preference_profile(&mut h, agent, source_profile(800, 400));
    set_source_reliability(&mut h, agent, SourceReliability::default());
    seed_world_beliefs(&mut h, agent);

    h.step_once();
    let planning = planning(&h, agent, Tick(0));
    for source in [close, far] {
        let rank = source_rank(planning, source).source_composite.unwrap();
        assert_eq!(rank.trust_factor_permille, 1000);
        assert_eq!(rank.wait_factor_permille, 1000);
        assert_eq!(rank.capacity_factor_permille, 1000);
        assert_eq!(rank.quality_factor_permille, 1000);
        assert_eq!(rank.composite_permille, 1000);
    }
    assert_ne!(
        planning
            .candidates
            .top_ranked_comparison
            .as_ref()
            .map(|comparison| comparison.decisive_dimension),
        Some(RankedGoalComparisonDimension::SourceComposite),
        "neutral source composites should not decide the final ordering"
    );
    assert!(
        matches!(
            planning
                .candidates
                .top_ranked_comparison
                .as_ref()
                .map(|comparison| comparison.decisive_dimension),
            None | Some(
                RankedGoalComparisonDimension::PlaceKey | RankedGoalComparisonDimension::EntityKey
            )
        ),
        "expected no source-composite decision or a lower deterministic tiebreaker; comparison={:?}",
        planning.candidates.top_ranked_comparison
    );
}
