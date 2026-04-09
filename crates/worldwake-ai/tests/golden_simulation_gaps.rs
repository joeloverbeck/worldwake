//! Golden tests covering observer-identified simulation gaps.

mod golden_harness;

use std::collections::BTreeSet;

use golden_harness::*;
use worldwake_core::{
    BeliefConfidencePolicy, CognitiveProfile, CommodityKind, EntityId, HomeostaticNeeds,
    MetabolismProfile, PerceptionProfile, Quantity, ResourceSource, Seed, StateHash, Tick,
    UtilityProfile, WorkstationTag, hash_event_log, hash_world, verify_authoritative_conservation,
    verify_live_lot_conservation,
};
use worldwake_sim::ActionTraceKind;

fn remote_resource_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        entity_memory_capacity: 64,
        entity_claim_capacity: 64,
        memory_retention_ticks: 64,
        observation_fidelity: pm(875),
        confidence_policy: BeliefConfidencePolicy::default(),
        institutional_memory_capacity: 20,
        consultation_speed_factor: pm(500),
        contradiction_tolerance: pm(300),
    }
}

fn configure_remote_resource_agent(h: &mut GoldenHarness, agent: EntityId) {
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        remote_resource_perception_profile(),
    );
    set_agent_cognitive_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        CognitiveProfile::default(),
    );
}

fn place_ground_commodity(
    h: &mut GoldenHarness,
    place: EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
) -> EntityId {
    let mut txn = new_txn(&mut h.world, 0);
    let lot = txn.create_item_lot(commodity, quantity).unwrap();
    txn.set_ground_location(lot, place).unwrap();
    commit_txn(txn, &mut h.event_log);
    lot
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RemoteTravelObservation {
    reached_orchard_farm: bool,
    ate_or_drank: bool,
    left_origin_tick: Option<u32>,
    stationary_origin_ticks: u32,
    committed_actions: BTreeSet<String>,
}

fn run_remote_travel_when_local_supply_exhausted(
    seed: Seed,
) -> (RemoteTravelObservation, StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.enable_action_tracing();

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "RemoteTraveler",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(500), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile {
            hunger_weight: pm(950),
            thirst_weight: pm(400),
            ..UtilityProfile::default()
        },
    );
    configure_remote_resource_agent(&mut h, agent);

    let orchard_source = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(10),
            max_quantity: Quantity(10),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        agent,
        ORCHARD_FARM,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        agent,
        orchard_source,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let mut observation = RemoteTravelObservation {
        reached_orchard_farm: false,
        ate_or_drank: false,
        left_origin_tick: None,
        stationary_origin_ticks: 0,
        committed_actions: BTreeSet::new(),
    };
    let mut seen_events = 0usize;

    for tick in 0..300u32 {
        h.step_once();

        if h.world.is_in_transit(agent) || h.world.effective_place(agent) != Some(VILLAGE_SQUARE) {
            observation.left_origin_tick.get_or_insert(tick);
        } else {
            observation.stationary_origin_ticks += 1;
        }

        if h.world.effective_place(agent) == Some(ORCHARD_FARM) {
            observation.reached_orchard_farm = true;
        }

        let events = h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(agent);
        for event in &events[seen_events..] {
            if matches!(event.kind, ActionTraceKind::Committed { .. })
                && let Some(def) = h.defs.get(event.def_id)
            {
                observation.committed_actions.insert(def.name.clone());
                if matches!(def.name.as_str(), "eat" | "drink") {
                    observation.ate_or_drank = true;
                }
            }
        }
        seen_events = events.len();

        if observation.reached_orchard_farm && observation.ate_or_drank {
            break;
        }
    }

    verify_live_lot_conservation(&h.world, CommodityKind::Apple, 1).unwrap();
    verify_authoritative_conservation(&h.world, CommodityKind::Apple, 9).unwrap();

    assert!(
        observation.reached_orchard_farm,
        "Agent should reach Orchard Farm when no local food exists; left_origin_tick={:?}, committed_actions={:?}",
        observation.left_origin_tick,
        observation.committed_actions
    );
    assert!(
        observation.ate_or_drank,
        "Agent should eat or drink after reaching remote resources; committed_actions={:?}",
        observation.committed_actions
    );
    assert!(
        observation.left_origin_tick.is_some_and(|tick| tick < 200),
        "Agent should not remain at VillageSquare for 200+ ticks under local scarcity; left_origin_tick={:?}, stationary_origin_ticks={}",
        observation.left_origin_tick,
        observation.stationary_origin_ticks
    );

    (
        observation,
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct IdleScarcityObservation {
    max_consecutive_idle: u32,
    committed_actions: BTreeSet<String>,
    reached_remote_resource_place: bool,
}

fn run_max_idle_under_remote_resource_scarcity(
    seed: Seed,
) -> (IdleScarcityObservation, StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.enable_action_tracing();

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "ScarcitySurvivor",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(500), pm(500), pm(500), pm(500), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile {
            hunger_weight: pm(800),
            thirst_weight: pm(800),
            fatigue_weight: pm(700),
            bladder_weight: pm(700),
            ..UtilityProfile::default()
        },
    );
    configure_remote_resource_agent(&mut h, agent);

    let orchard_source = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(10),
            max_quantity: Quantity(10),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );
    let remote_water = place_ground_commodity(
        &mut h,
        ORCHARD_FARM,
        CommodityKind::Water,
        Quantity(2),
    );

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );
    for entity in [ORCHARD_FARM, orchard_source, remote_water] {
        seed_belief_from_world(
            &mut h.world,
            &mut h.event_log,
            agent,
            entity,
            Tick(0),
            worldwake_core::PerceptionSource::Inference,
        );
    }

    let mut observation = IdleScarcityObservation {
        max_consecutive_idle: 0,
        committed_actions: BTreeSet::new(),
        reached_remote_resource_place: false,
    };
    let mut consecutive_idle = 0u32;
    let mut seen_events = 0usize;

    for _ in 0..300u32 {
        h.step_once();

        if h.world.effective_place(agent) == Some(ORCHARD_FARM) {
            observation.reached_remote_resource_place = true;
        }

        let events = h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(agent);
        let new_events = &events[seen_events..];
        let mut had_lifecycle_activity = false;
        for event in new_events {
            if let Some(def) = h.defs.get(event.def_id)
                && matches!(
                    event.kind,
                    ActionTraceKind::Started { .. }
                        | ActionTraceKind::Committed { .. }
                        | ActionTraceKind::Aborted { .. }
                        | ActionTraceKind::StartFailed { .. }
                )
            {
                had_lifecycle_activity = true;
                if matches!(event.kind, ActionTraceKind::Committed { .. }) {
                    observation.committed_actions.insert(def.name.clone());
                }
            }
        }
        seen_events = events.len();

        if had_lifecycle_activity || h.agent_has_active_action(agent) {
            consecutive_idle = 0;
        } else {
            consecutive_idle += 1;
            observation.max_consecutive_idle =
                observation.max_consecutive_idle.max(consecutive_idle);
        }
    }

    assert!(
        observation.max_consecutive_idle < 100,
        "Agent should not idle for 100+ consecutive ticks under remote resource scarcity; max_idle={}, committed_actions={:?}, reached_remote_resource_place={}",
        observation.max_consecutive_idle,
        observation.committed_actions,
        observation.reached_remote_resource_place
    );
    assert!(
        !observation.committed_actions.is_empty() || observation.reached_remote_resource_place,
        "Agent should eventually act under remote resource scarcity; max_idle={}",
        observation.max_consecutive_idle
    );

    (
        observation,
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

// ---------------------------------------------------------------------------
// Scenario 126: Remote Travel To Resource Under Local Scarcity
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI, Travel, Production
// GoalKinds: AcquireCommodity, ConsumeOwnedCommodity
// ActionDomains: Travel, Needs, Production
// Places: VillageSquare, OrchardFarm
// Principles: 7, 14, 20
//
// Setup: One agent at VillageSquare with critical hunger and no local food.
//   OrchardFarm has an apple source, and the agent starts with seeded beliefs
//   about both the place and the remote source.
//
// Proves: The agent leaves the barren local start, reaches OrchardFarm, and
//   commits eat/drink within the tick budget instead of stalling in local
//   sleep/relieve loops.
//
// Chain: hunger pressure -> remote resource candidate generation -> travel
//   plan -> OrchardFarm arrival -> remote acquisition -> consumption.

#[test]
fn golden_remote_travel_when_local_supply_exhausted() {
    let _ = run_remote_travel_when_local_supply_exhausted(Seed([176; 32]));
}

#[test]
fn golden_remote_travel_when_local_supply_exhausted_replays_deterministically() {
    let first = run_remote_travel_when_local_supply_exhausted(Seed([176; 32]));
    let second = run_remote_travel_when_local_supply_exhausted(Seed([176; 32]));
    assert_eq!(
        first, second,
        "remote travel under local scarcity should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 127: Idle Cap Under Remote Resource Scarcity
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI, Travel, Production
// GoalKinds: Sleep, Relieve, AcquireCommodity, ConsumeOwnedCommodity
// ActionDomains: Travel, Needs, Production
// Places: VillageSquare, OrchardFarm
// Principles: 8, 20, 22
//
// Setup: One agent at VillageSquare with moderate hunger, thirst, fatigue,
//   and bladder pressure. Local food and water are absent; OrchardFarm holds
//   both apples and remote water, while local self-care affordances remain
//   lawful. The agent starts with seeded beliefs about the remote resources.
//
// Proves: Under multi-need scarcity, the agent remains behaviorally active
//   instead of entering a prolonged idle streak.
//
// Chain: multiple active needs -> candidate generation over remote resources
//   plus local self-care -> planner fallback/execution -> bounded idle streak.

#[test]
fn golden_max_idle_under_remote_resource_scarcity() {
    let _ = run_max_idle_under_remote_resource_scarcity(Seed([177; 32]));
}

#[test]
fn golden_max_idle_under_remote_resource_scarcity_replays_deterministically() {
    let first = run_max_idle_under_remote_resource_scarcity(Seed([177; 32]));
    let second = run_max_idle_under_remote_resource_scarcity(Seed([177; 32]));
    assert_eq!(
        first, second,
        "idle cap under remote resource scarcity should replay deterministically"
    );
}
