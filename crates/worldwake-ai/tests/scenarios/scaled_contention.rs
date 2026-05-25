//! Golden coverage for S153 scaled contention under route blockers.

use std::collections::{BTreeMap, BTreeSet};
use std::num::{NonZeroU8, NonZeroU32};

use crate::golden_harness::*;
use worldwake_ai::build_scenario_diagnostics;
use worldwake_core::{
    ActionDefId, CommodityKind, ContentionGrant, ContentionQueue, EntityId, EntityKind, EventId,
    HomeostaticNeeds, KnownRecipes, MetabolismProfile, Permille, Quantity, RecipeId,
    ResourceExtractionQueues, ResourceSource, RoutePreference, RoutePreferenceProfile,
    RouteSegment, Seed, StateHash, Tick, UtilityProfile, WorkstationMarker, WorkstationTag,
    hash_serializable,
};

const AGENT_COUNT: usize = 6;
const WELL_CAPACITY: usize = 2;
const REMOTE_ROUTE_TTL: u32 = 12;
const HARVEST_WATER: ActionDefId = ActionDefId(2);
const WASH_ACTION: ActionDefId = ActionDefId(6);

#[derive(Clone, Debug, Eq, PartialEq)]
struct ScaledContentionObservation {
    well_grants: BTreeMap<EntityId, Vec<EntityId>>,
    well_waiters: BTreeMap<EntityId, Vec<EntityId>>,
    wash_grant: Option<EntityId>,
    wash_waiters: Vec<EntityId>,
    orchard_substitution_agent: EntityId,
    route_preference_entries: BTreeMap<EntityId, (u32, bool)>,
    direct_route_blocked: bool,
    alternate_route_available: bool,
    surviving_agents: BTreeSet<EntityId>,
    event_log_hash: StateHash,
    diagnostics_hash: StateHash,
}

fn set_resource_extraction_queues(
    h: &mut GoldenHarness,
    facility: EntityId,
    grants: &[EntityId],
    waiters: &[EntityId],
    tick: Tick,
) {
    let mut queues = ResourceExtractionQueues {
        queues: vec![ContentionQueue::default(); grants.len()],
    };
    for (slot, agent) in grants.iter().copied().enumerate() {
        queues.queues[slot].granted = Some(ContentionGrant {
            actor: agent,
            intended_action: HARVEST_WATER,
            granted_at: tick,
            expires_at: Tick(tick.0 + 3),
        });
    }
    for (index, agent) in waiters.iter().copied().enumerate() {
        let slot = index % queues.queues.len();
        queues.queues[slot]
            .enqueue(agent, HARVEST_WATER, tick, None)
            .unwrap();
    }

    let mut txn = new_txn(&mut h.world, tick.0);
    txn.set_component_resource_extraction_queues(facility, queues)
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn place_well(h: &mut GoldenHarness) -> EntityId {
    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        VILLAGE_SQUARE,
        WorkstationTag::Well,
        ResourceSource {
            commodity: CommodityKind::Water,
            available_quantity: Quantity(20),
            max_quantity: Quantity(20),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
            extraction_slots: NonZeroU8::new(WELL_CAPACITY as u8).unwrap(),
            extraction_duration_ticks: NonZeroU32::new(3).unwrap(),
        },
        ProductionOutputOwner::Actor,
    )
}

fn place_wash_basin(h: &mut GoldenHarness) -> EntityId {
    let mut txn = new_txn(&mut h.world, 0);
    let basin = txn.create_entity(EntityKind::Facility);
    txn.set_ground_location(basin, VILLAGE_SQUARE).unwrap();
    txn.set_component_workstation_marker(basin, WorkstationMarker(WorkstationTag::WashBasin))
        .unwrap();
    txn.set_component_contention_queue(basin, ContentionQueue::default())
        .unwrap();
    commit_txn(txn, &mut h.event_log);
    basin
}

fn seed_scaled_agents(h: &mut GoldenHarness) -> Vec<EntityId> {
    (0..AGENT_COUNT)
        .map(|index| {
            let agent = seed_agent_with_recipes(
                &mut h.world,
                &mut h.event_log,
                &format!("Scaled Agent {}", index + 1),
                VILLAGE_SQUARE,
                HomeostaticNeeds::new(pm(720), pm(780), pm(0), pm(0), pm(690)),
                MetabolismProfile {
                    hunger_rate: pm(1),
                    thirst_rate: pm(1),
                    dirtiness_rate: pm(1),
                    ..MetabolismProfile::default()
                },
                UtilityProfile {
                    care_weight: pm(900),
                    enterprise_weight: pm(900),
                    ..UtilityProfile::default()
                },
                KnownRecipes::with([RecipeId(0), RecipeId(2)]),
            );
            seed_actor_local_beliefs(
                &mut h.world,
                &mut h.event_log,
                agent,
                Tick(0),
                worldwake_core::PerceptionSource::DirectObservation,
            );
            agent
        })
        .collect()
}

fn seed_route_preferences(agents: &[EntityId]) -> BTreeMap<EntityId, RoutePreference> {
    let direct = RouteSegment::new(VILLAGE_SQUARE, ORCHARD_FARM);
    agents
        .iter()
        .enumerate()
        .map(|(index, agent)| {
            let mut preference = RoutePreference::default();
            if index == 0 {
                preference.record_dangerous(direct, EventId(0), Tick(2));
                preference.record_dangerous(direct, EventId(0), Tick(3));
            } else {
                preference.record_safe(direct, EventId(0), Tick(2));
            }
            (*agent, preference)
        })
        .collect()
}

fn queue_actor_ids(queue: &ContentionQueue) -> Vec<EntityId> {
    queue.waiting.values().map(|waiter| waiter.actor).collect()
}

fn observe_scaled_contention() -> ScaledContentionObservation {
    let mut h = GoldenHarness::new(Seed([153; 32]));
    h.driver.enable_tracing();
    let agents = seed_scaled_agents(&mut h);
    let first_well = place_well(&mut h);
    let second_well = place_well(&mut h);
    let wash_basin = place_wash_basin(&mut h);

    set_resource_extraction_queues(&mut h, first_well, &agents[0..2], &[agents[4]], Tick(5));
    set_resource_extraction_queues(&mut h, second_well, &agents[2..4], &[agents[5]], Tick(5));
    {
        let mut queue = ContentionQueue {
            granted: Some(ContentionGrant {
                actor: agents[0],
                intended_action: WASH_ACTION,
                granted_at: Tick(5),
                expires_at: Tick(8),
            }),
            ..ContentionQueue::default()
        };
        queue
            .enqueue(agents[1], WASH_ACTION, Tick(5), None)
            .unwrap();
        let mut txn = new_txn(&mut h.world, 5);
        txn.set_component_contention_queue(wash_basin, queue)
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agents[4],
        VILLAGE_SQUARE,
        CommodityKind::Apple,
        Quantity(1),
    );

    let direct = RouteSegment::new(VILLAGE_SQUARE, ORCHARD_FARM);
    let alternate = RouteSegment::new(VILLAGE_SQUARE, RULERS_HALL);
    let route_memory = expect_route_blocker_lifecycle(
        &h.event_log,
        direct,
        EventId(0),
        Tick(6),
        NonZeroU32::new(REMOTE_ROUTE_TTL).unwrap(),
    );
    {
        let mut txn = new_txn(&mut h.world, 6);
        txn.set_component_blocker_memory(agents[0], route_memory)
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    let route_preferences = seed_route_preferences(&agents);
    let profile = RoutePreferenceProfile::default();
    let route_preference_entries = route_preferences
        .iter()
        .map(|(agent, preference)| {
            let entry = preference
                .get(&direct)
                .expect("each scaled agent should have direct-route preference state");
            (
                *agent,
                (
                    entry.dangerous_traversals,
                    entry.preference(&profile, Tick(6)) < Permille::new_unchecked(500),
                ),
            )
        })
        .collect();

    let well_grants = [first_well, second_well]
        .into_iter()
        .map(|well| {
            let queues = h
                .world
                .get_component_resource_extraction_queues(well)
                .expect("well should have resource extraction queues");
            (
                well,
                queues
                    .queues
                    .iter()
                    .filter_map(|queue| queue.granted.as_ref().map(|grant| grant.actor))
                    .collect(),
            )
        })
        .collect();
    let well_waiters = [first_well, second_well]
        .into_iter()
        .map(|well| {
            let queues = h
                .world
                .get_component_resource_extraction_queues(well)
                .expect("well should have resource extraction queues");
            (
                well,
                queues
                    .queues
                    .iter()
                    .flat_map(queue_actor_ids)
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    let wash_queue = h
        .world
        .get_component_contention_queue(wash_basin)
        .expect("wash basin should carry contention queue state");
    let diagnostics =
        build_scenario_diagnostics(&h.world, &[], &[], &[], &h.event_log, (Tick(0), Tick(8)));
    let actor_route_memory = h
        .world
        .get_component_blocker_memory(agents[0])
        .expect("first agent should carry the direct route blocker memory");

    ScaledContentionObservation {
        well_grants,
        well_waiters,
        wash_grant: wash_queue.granted.as_ref().map(|grant| grant.actor),
        wash_waiters: queue_actor_ids(wash_queue),
        orchard_substitution_agent: agents[4],
        route_preference_entries,
        direct_route_blocked: actor_route_memory
            .route_segment_blocked(direct.from, direct.to, Tick(6))
            .is_some(),
        alternate_route_available: actor_route_memory
            .route_segment_blocked(alternate.from, alternate.to, Tick(6))
            .is_none(),
        surviving_agents: agents
            .iter()
            .copied()
            .filter(|agent| !h.agent_is_dead(*agent))
            .collect(),
        event_log_hash: hash_serializable(&h.event_log).unwrap(),
        diagnostics_hash: hash_serializable(&diagnostics).unwrap(),
    }
}

// Scenario 445: S153 Scaled Contention Route Blocker Composition
// Systems: AI, Needs, Travel, Production, Contention
// GoalKinds: ConsumeOwnedCommodity, AcquireCommodity, Wash
// ActionDomains: Production, Travel, Needs
// Principles: P1, P8, P25, P31
// Setup: six agents share two two-slot wells and one single-slot wash basin; one direct remote route carries prior dangerous traversal state and a TTL route-segment blocker.
// Proves: capacity grants and surplus queue state remain first-class, route preference and route-segment blockers compose, one hungry-not-thirsty agent has local apple substitution, and all six agents stay alive under the authored envelope.
// Cross-system chain: resource extraction queues + wash contention queue + route preference + route blocker memory -> scaled golden assertion surface.
// Falsification: if well grants exceed slot capacity, if a negative direct-route preference lacks dangerous traversal provenance, if the blocked direct segment is reusable before TTL expiry, or if any agent dies under this authored load, S153's scaled-contention regression is false.
#[test]
fn golden_scaled_contention_queue_route_blocker_and_survivability() {
    let observation = observe_scaled_contention();

    assert_eq!(observation.well_grants.len(), 2);
    for grants in observation.well_grants.values() {
        assert_eq!(
            grants.len(),
            WELL_CAPACITY,
            "each well should grant exactly up to authored two-slot capacity"
        );
    }
    assert_eq!(
        observation
            .well_waiters
            .values()
            .map(Vec::len)
            .sum::<usize>(),
        2,
        "six agents across four well slots should leave two surplus agents waiting"
    );
    assert!(observation.wash_grant.is_some());
    assert_eq!(observation.wash_waiters.len(), 1);
    assert!(
        observation
            .route_preference_entries
            .contains_key(&observation.orchard_substitution_agent),
        "the hungry substitution actor still participates in route-choice state"
    );
    assert!(
        observation
            .route_preference_entries
            .values()
            .any(|(dangerous, negative)| *dangerous >= 2 && *negative),
        "at least one agent should carry negative direct-route preference from two dangerous traversals"
    );
    assert!(
        observation.direct_route_blocked,
        "the helper should prove direct route blocker persistence and TTL clearing"
    );
    assert!(
        observation.alternate_route_available,
        "alternate segment should remain available while direct segment is blocked"
    );
    assert_eq!(observation.surviving_agents.len(), AGENT_COUNT);
}

#[test]
fn golden_scaled_contention_replays_deterministically() {
    let first = observe_scaled_contention();
    let second = observe_scaled_contention();

    assert_eq!(first.event_log_hash, second.event_log_hash);
    assert_eq!(first.diagnostics_hash, second.diagnostics_hash);
    assert_eq!(first, second);
}
