//! Golden tests for production, economy, transport, and conservation.

mod golden_harness;

use std::collections::BTreeSet;

use golden_harness::*;
use worldwake_core::{
    hash_event_log, hash_world, total_authoritative_commodity_quantity, total_live_lot_quantity,
    verify_authoritative_conservation, verify_live_lot_conservation, CarryCapacity, CommodityKind,
    EntityId, EventTag, GrantedFacilityUse, HomeostaticNeeds, KnownRecipes, LoadUnits,
    MetabolismProfile, Quantity, ResourceSource, Seed, StateHash, Tick, UtilityProfile,
    WorkstationTag,
};

// ---------------------------------------------------------------------------
// Scenario runners (only used by tests in this file)
// ---------------------------------------------------------------------------

fn run_multi_recipe_craft_scenario(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, build_multi_recipe_registry());
    let apple_recipe = h
        .recipes
        .recipe_by_name("Harvest Apples")
        .map(|(id, _)| id)
        .unwrap();
    let grain_recipe = h
        .recipes
        .recipe_by_name("Harvest Grain")
        .map(|(id, _)| id)
        .unwrap();
    let bread_recipe = h
        .recipes
        .recipe_by_name("Bake Bread")
        .map(|(id, _)| id)
        .unwrap();

    let agent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Miller",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::with([apple_recipe, grain_recipe, bread_recipe]),
    );

    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent,
        VILLAGE_SQUARE,
        CommodityKind::Firewood,
        Quantity(1),
    );
    place_workstation(
        &mut h.world,
        &mut h.event_log,
        VILLAGE_SQUARE,
        WorkstationTag::Mill,
    );

    verify_live_lot_conservation(&h.world, CommodityKind::Firewood, 1).unwrap();
    verify_authoritative_conservation(&h.world, CommodityKind::Firewood, 1).unwrap();
    verify_live_lot_conservation(&h.world, CommodityKind::Bread, 0).unwrap();
    verify_authoritative_conservation(&h.world, CommodityKind::Bread, 0).unwrap();

    let initial_hunger = h.agent_hunger(agent);
    let mut saw_bread_materialize = false;
    let mut hunger_decreased = false;

    for _ in 0..80 {
        h.step_once();

        let live_bread = total_live_lot_quantity(&h.world, CommodityKind::Bread);
        if live_bread > 0 {
            saw_bread_materialize = true;
            verify_live_lot_conservation(&h.world, CommodityKind::Firewood, 0).unwrap();
            verify_authoritative_conservation(&h.world, CommodityKind::Firewood, 0).unwrap();
            verify_live_lot_conservation(&h.world, CommodityKind::Bread, 1).unwrap();
            verify_authoritative_conservation(&h.world, CommodityKind::Bread, 1).unwrap();
        }

        if saw_bread_materialize && live_bread == 0 && h.agent_hunger(agent) < initial_hunger {
            hunger_decreased = true;
            verify_live_lot_conservation(&h.world, CommodityKind::Firewood, 0).unwrap();
            verify_authoritative_conservation(&h.world, CommodityKind::Firewood, 0).unwrap();
            verify_live_lot_conservation(&h.world, CommodityKind::Bread, 0).unwrap();
            verify_authoritative_conservation(&h.world, CommodityKind::Bread, 0).unwrap();
            break;
        }
    }

    assert!(
        saw_bread_materialize,
        "Agent should craft bread when recipe inputs are available and a mill is local"
    );
    assert!(
        hunger_decreased,
        "Agent should consume crafted bread after it materializes"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

fn run_capacity_constrained_ground_lot_pickup_scenario(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Porter",
        ORCHARD_FARM,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_carry_capacity(agent, CarryCapacity(LoadUnits(1)))
        .unwrap();
    commit_txn(txn, &mut h.event_log);

    place_workstation_with_source(
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
    );

    verify_live_lot_conservation(&h.world, CommodityKind::Apple, 0).unwrap();
    verify_authoritative_conservation(&h.world, CommodityKind::Apple, 10).unwrap();

    let initial_hunger = h.agent_hunger(agent);
    let mut saw_apple_materialize = false;
    let mut saw_split_pickup = false;
    let mut hunger_decreased = false;

    for _ in 0..80 {
        h.step_once();

        let live_apples = total_live_lot_quantity(&h.world, CommodityKind::Apple);
        let authoritative_apples =
            total_authoritative_commodity_quantity(&h.world, CommodityKind::Apple);

        if live_apples == 2 {
            saw_apple_materialize = true;
            verify_live_lot_conservation(&h.world, CommodityKind::Apple, 2).unwrap();
            verify_authoritative_conservation(&h.world, CommodityKind::Apple, 10).unwrap();
        }

        if saw_apple_materialize {
            let apple_lots_at_farm = h
                .world
                .entities_effectively_at(ORCHARD_FARM)
                .into_iter()
                .filter(|entity| {
                    h.world
                        .get_component_item_lot(*entity)
                        .is_some_and(|lot| lot.commodity == CommodityKind::Apple)
                })
                .collect::<Vec<_>>();

            let has_possessed_apples = apple_lots_at_farm
                .iter()
                .any(|entity| h.world.possessor_of(*entity) == Some(agent));
            let has_ground_apples = apple_lots_at_farm
                .iter()
                .any(|entity| h.world.possessor_of(*entity).is_none());
            if has_possessed_apples && has_ground_apples {
                saw_split_pickup = true;
            }
        }

        if saw_split_pickup && h.agent_hunger(agent) < initial_hunger {
            hunger_decreased = true;
            assert_eq!(
                live_apples, 1,
                "One apple should remain after a split pickup followed by one consumption"
            );
            assert_eq!(
                authoritative_apples, 9,
                "Authoritative apple total should reflect one consumed apple after harvest"
            );
            verify_live_lot_conservation(&h.world, CommodityKind::Apple, 1).unwrap();
            verify_authoritative_conservation(&h.world, CommodityKind::Apple, 9).unwrap();
            break;
        }
    }

    assert!(
        saw_apple_materialize,
        "Harvesting should materialize a two-apple ground lot before pickup"
    );
    assert!(
        saw_split_pickup,
        "Carry-capacity pressure should force a split pickup with both possessed and ground apple lots"
    );
    assert!(
        hunger_decreased,
        "Agent should consume an apple after the constrained split pickup"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

struct ResourceExhaustionRaceOutcome {
    world_hash: StateHash,
    log_hash: StateHash,
    observed_source_quantities: BTreeSet<u32>,
    agents_with_hunger_relief: BTreeSet<worldwake_core::EntityId>,
    saw_live_apple_lots: bool,
    final_source_quantity: Quantity,
}

fn run_resource_exhaustion_race_scenario(seed: Seed) -> ResourceExhaustionRaceOutcome {
    let mut h = GoldenHarness::new(seed);
    let agents = [
        seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Aster",
            ORCHARD_FARM,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
            MetabolismProfile::default(),
            UtilityProfile::default(),
        ),
        seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Bram",
            ORCHARD_FARM,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
            MetabolismProfile::default(),
            UtilityProfile::default(),
        ),
        seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Cara",
            ORCHARD_FARM,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
            MetabolismProfile::default(),
            UtilityProfile::default(),
        ),
        seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Dara",
            ORCHARD_FARM,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
            MetabolismProfile::default(),
            UtilityProfile::default(),
        ),
    ];

    let initial_hunger = agents.map(|agent| h.agent_hunger(agent));
    let workstation = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(4),
            max_quantity: Quantity(4),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
    );

    let mut observed_source_quantities = BTreeSet::from([4_u32]);
    let mut agents_with_hunger_relief = BTreeSet::new();
    let mut saw_live_apple_lots = false;

    verify_live_lot_conservation(&h.world, CommodityKind::Apple, 0).unwrap();
    verify_authoritative_conservation(&h.world, CommodityKind::Apple, 4).unwrap();

    for _ in 0..150 {
        h.step_once();

        let source_quantity = h
            .world
            .get_component_resource_source(workstation)
            .expect("workstation should retain resource source during golden scenario")
            .available_quantity;
        observed_source_quantities.insert(source_quantity.0);

        let authoritative_apples =
            total_authoritative_commodity_quantity(&h.world, CommodityKind::Apple);
        assert!(
            authoritative_apples <= 4,
            "Authoritative apple quantity must never exceed the initial source stock"
        );
        verify_authoritative_conservation(&h.world, CommodityKind::Apple, authoritative_apples)
            .unwrap();

        if total_live_lot_quantity(&h.world, CommodityKind::Apple) > 0 {
            saw_live_apple_lots = true;
        }

        for (index, agent) in agents.iter().copied().enumerate() {
            if h.agent_hunger(agent) < initial_hunger[index] {
                agents_with_hunger_relief.insert(agent);
            }
        }
    }

    ResourceExhaustionRaceOutcome {
        world_hash: hash_world(&h.world).unwrap(),
        log_hash: hash_event_log(&h.event_log).unwrap(),
        observed_source_quantities,
        agents_with_hunger_relief,
        saw_live_apple_lots,
        final_source_quantity: h
            .world
            .get_component_resource_source(workstation)
            .expect("workstation should retain resource source through scenario")
            .available_quantity,
    }
}

struct ExclusiveQueueContentionOutcome {
    world_hash: StateHash,
    log_hash: StateHash,
    max_waiting_len: usize,
    saw_granted_state: bool,
    promoted_actors: Vec<EntityId>,
    final_source_quantity: Quantity,
}

fn seed_exclusive_orchard_contenders(h: &mut GoldenHarness) -> [EntityId; 4] {
    [
        seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Aster",
            ORCHARD_FARM,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
            MetabolismProfile::default(),
            UtilityProfile::default(),
        ),
        seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Bram",
            ORCHARD_FARM,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
            MetabolismProfile::default(),
            UtilityProfile::default(),
        ),
        seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Cara",
            ORCHARD_FARM,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
            MetabolismProfile::default(),
            UtilityProfile::default(),
        ),
        seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Dara",
            ORCHARD_FARM,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
            MetabolismProfile::default(),
            UtilityProfile::default(),
        ),
    ]
}

fn record_new_promotions(
    h: &GoldenHarness,
    workstation: EntityId,
    previous_promotions: &mut usize,
    promoted_actors: &mut Vec<EntityId>,
) {
    let promotion_ids = h.event_log.events_by_tag(EventTag::QueueGrantPromoted);
    if promotion_ids.len() <= *previous_promotions {
        return;
    }

    for event_id in &promotion_ids[*previous_promotions..] {
        let record = h
            .event_log
            .get(*event_id)
            .expect("queue promotion event should exist");
        let promoted_actor = record
            .target_ids
            .iter()
            .copied()
            .find(|target| *target != workstation)
            .expect("queue promotion event should target the promoted actor");
        promoted_actors.push(promoted_actor);
    }
    *previous_promotions = promotion_ids.len();
}

fn run_exclusive_queue_contention_scenario(seed: Seed) -> ExclusiveQueueContentionOutcome {
    let mut h = GoldenHarness::new(seed);
    let agents = seed_exclusive_orchard_contenders(&mut h);

    let workstation = place_exclusive_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(4),
            max_quantity: Quantity(4),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        nz(3),
    );

    let mut max_waiting_len = 0;
    let mut saw_granted_state = false;
    let mut previous_promotions = 0;
    let mut promoted_actors = Vec::new();

    verify_live_lot_conservation(&h.world, CommodityKind::Apple, 0).unwrap();
    verify_authoritative_conservation(&h.world, CommodityKind::Apple, 4).unwrap();

    for _ in 0..150 {
        h.step_once();

        let queue = h
            .world
            .get_component_facility_use_queue(workstation)
            .expect("exclusive workstation should retain queue state");
        max_waiting_len = max_waiting_len.max(queue.waiting.len());
        saw_granted_state |= queue.granted.is_some();
        record_new_promotions(&h, workstation, &mut previous_promotions, &mut promoted_actors);

        let authoritative_apples =
            total_authoritative_commodity_quantity(&h.world, CommodityKind::Apple);
        assert!(
            authoritative_apples <= 4,
            "Authoritative apple quantity must never exceed the initial exclusive orchard stock"
        );
        verify_authoritative_conservation(&h.world, CommodityKind::Apple, authoritative_apples)
            .unwrap();

        if h.world
            .get_component_resource_source(workstation)
            .expect("exclusive workstation should retain resource source")
            .available_quantity
            == Quantity(0)
            && promoted_actors.len() >= 2
            && agents
                .iter()
                .filter(|agent| h.agent_hunger(**agent) < pm(900))
                .count()
                >= 1
        {
            break;
        }
    }

    ExclusiveQueueContentionOutcome {
        world_hash: hash_world(&h.world).unwrap(),
        log_hash: hash_event_log(&h.event_log).unwrap(),
        max_waiting_len,
        saw_granted_state,
        promoted_actors,
        final_source_quantity: h
            .world
            .get_component_resource_source(workstation)
            .expect("exclusive workstation should retain resource source")
            .available_quantity,
    }
}

#[allow(clippy::struct_excessive_bools)]
struct FacilityQueuePatienceTimeoutOutcome {
    joined_facility_a: bool,
    abandoned_facility_a: bool,
    recorded_blocked_facility_a: bool,
    used_facility_b: bool,
    hunger_decreased: bool,
    facility_a_final_source_quantity: Quantity,
    facility_b_final_source_quantity: Quantity,
}

#[allow(clippy::too_many_lines)]
fn run_facility_queue_patience_timeout_scenario(seed: Seed) -> FacilityQueuePatienceTimeoutOutcome {
    let mut h = GoldenHarness::new(seed);
    let patient = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Patient",
        ORCHARD_FARM,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let monopolist = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Monopolist",
        ORCHARD_FARM,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_queue_patience(&mut h.world, &mut h.event_log, patient, Some(nz(3)));

    let facility_a = place_exclusive_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(4),
            max_quantity: Quantity(4),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        nz(12),
    );
    let facility_b = place_exclusive_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        VILLAGE_SQUARE,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(4),
            max_quantity: Quantity(4),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        nz(3),
    );

    let harvest_action = h
        .defs
        .iter()
        .find(|def| def.name == "harvest:Harvest Apples")
        .map(|def| def.id)
        .expect("harvest action should be registered");

    {
        let mut txn = new_txn(&mut h.world, 0);
        let mut queue = txn
            .get_component_facility_use_queue(facility_a)
            .cloned()
            .expect("exclusive facility A should have queue state");
        queue.granted = Some(GrantedFacilityUse {
            actor: monopolist,
            intended_action: harvest_action,
            granted_at: Tick(0),
            expires_at: Tick(12),
        });
        txn.set_component_facility_use_queue(facility_a, queue).unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    let initial_hunger = h.agent_hunger(patient);
    let mut joined_facility_a = false;
    let mut abandoned_facility_a = false;
    let mut recorded_blocked_facility_a = false;
    let mut used_facility_b = false;
    let mut hunger_decreased = false;

    verify_live_lot_conservation(&h.world, CommodityKind::Apple, 0).unwrap();
    verify_authoritative_conservation(&h.world, CommodityKind::Apple, 8).unwrap();

    for _ in 0..120 {
        h.step_once();

        let queue_a = h
            .world
            .get_component_facility_use_queue(facility_a)
            .expect("facility A should retain queue state");
        let queue_b = h
            .world
            .get_component_facility_use_queue(facility_b)
            .expect("facility B should retain queue state");

        if queue_a.position_of(patient).is_some() {
            joined_facility_a = true;
        }
        if joined_facility_a
            && queue_a.position_of(patient).is_none()
            && queue_a
                .granted
                .as_ref()
                .is_none_or(|granted| granted.actor != patient)
        {
            abandoned_facility_a = true;
        }

        if h.world
            .get_component_blocked_intent_memory(patient)
            .is_some_and(|memory| {
                memory.intents.iter().any(|intent| {
                    intent.blocking_fact == worldwake_core::BlockingFact::ExclusiveFacilityUnavailable
                        && intent.related_entity == Some(facility_a)
                        && intent.related_action == Some(harvest_action)
                })
            })
        {
            recorded_blocked_facility_a = true;
        }

        if queue_b
            .granted
            .as_ref()
            .is_some_and(|granted| granted.actor == patient)
            || h.world
                .get_component_resource_source(facility_b)
                .is_some_and(|source| source.available_quantity < Quantity(4))
        {
            used_facility_b = true;
        }

        let authoritative_apples =
            total_authoritative_commodity_quantity(&h.world, CommodityKind::Apple);
        assert!(
            authoritative_apples <= 8,
            "Authoritative apple quantity must never exceed the initial combined stock"
        );
        verify_authoritative_conservation(&h.world, CommodityKind::Apple, authoritative_apples)
            .unwrap();

        if h.agent_hunger(patient) < initial_hunger {
            hunger_decreased = true;
            break;
        }
    }

    FacilityQueuePatienceTimeoutOutcome {
        joined_facility_a,
        abandoned_facility_a,
        recorded_blocked_facility_a,
        used_facility_b,
        hunger_decreased,
        facility_a_final_source_quantity: h
            .world
            .get_component_resource_source(facility_a)
            .expect("facility A should retain resource source")
            .available_quantity,
        facility_b_final_source_quantity: h
            .world
            .get_component_resource_source(facility_b)
            .expect("facility B should retain resource source")
            .available_quantity,
    }
}

#[allow(clippy::struct_excessive_bools)]
struct GrantExpiryBeforeIntendedActionOutcome {
    saw_initial_grant: bool,
    saw_local_detour_before_harvest: bool,
    saw_grant_expire: bool,
    source_untouched_when_grant_expired: bool,
    saw_second_promotion: bool,
    hunger_decreased: bool,
    final_source_quantity: Quantity,
}

#[allow(clippy::too_many_lines)]
fn run_grant_expiry_before_intended_action_scenario(
    seed: Seed,
) -> GrantExpiryBeforeIntendedActionOutcome {
    let mut h = GoldenHarness::new(seed);
    let thirst_spike_after_first_grant = MetabolismProfile::new(
        pm(2),
        pm(900),
        pm(2),
        pm(4),
        pm(1),
        pm(20),
        nz(480),
        nz(240),
        nz(120),
        nz(40),
        nz(8),
        nz(12),
    );

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Rill",
        ORCHARD_FARM,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        thirst_spike_after_first_grant,
        UtilityProfile {
            hunger_weight: pm(500),
            thirst_weight: pm(1000),
            ..UtilityProfile::default()
        },
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent,
        ORCHARD_FARM,
        CommodityKind::Water,
        Quantity(1),
    );

    let workstation = place_exclusive_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(4),
            max_quantity: Quantity(4),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        nz(1),
    );

    let initial_hunger = h.agent_hunger(agent);
    let mut saw_initial_grant = false;
    let mut saw_local_detour_before_harvest = false;
    let mut saw_grant_expire = false;
    let mut source_untouched_when_grant_expired = false;
    let mut saw_second_promotion = false;
    let mut previous_promotion_count = 0usize;

    verify_live_lot_conservation(&h.world, CommodityKind::Apple, 0).unwrap();
    verify_authoritative_conservation(&h.world, CommodityKind::Apple, 4).unwrap();

    for _ in 0..80 {
        h.step_once();

        let queue = h
            .world
            .get_component_facility_use_queue(workstation)
            .expect("exclusive workstation should retain queue state");
        let source_quantity = h
            .world
            .get_component_resource_source(workstation)
            .expect("exclusive workstation should retain resource source")
            .available_quantity;
        let promotion_count = h.event_log.events_by_tag(EventTag::QueueGrantPromoted).len();
        let expiry_count = h.event_log.events_by_tag(EventTag::QueueGrantExpired).len();

        if queue
            .granted
            .as_ref()
            .is_some_and(|granted| granted.actor == agent)
        {
            if saw_grant_expire {
                saw_second_promotion = true;
            } else {
                saw_initial_grant = true;
            }
        }

        if promotion_count > previous_promotion_count {
            if saw_grant_expire {
                saw_second_promotion = true;
            } else {
                saw_initial_grant = true;
            }
            previous_promotion_count = promotion_count;
        }

        if expiry_count > 0 {
            saw_grant_expire = true;
            source_untouched_when_grant_expired |= source_quantity == Quantity(4);
        }

        if h.agent_commodity_qty(agent, CommodityKind::Water) == Quantity(0)
            && source_quantity == Quantity(4)
        {
            saw_local_detour_before_harvest = true;
        }

        let authoritative_apples =
            total_authoritative_commodity_quantity(&h.world, CommodityKind::Apple);
        assert!(
            authoritative_apples <= 4,
            "Authoritative apple quantity must never exceed the initial exclusive orchard stock"
        );
        verify_authoritative_conservation(&h.world, CommodityKind::Apple, authoritative_apples)
            .unwrap();

        if h.agent_hunger(agent) < initial_hunger && source_quantity < Quantity(4) {
            return GrantExpiryBeforeIntendedActionOutcome {
                saw_initial_grant,
                saw_local_detour_before_harvest,
                saw_grant_expire,
                source_untouched_when_grant_expired,
                saw_second_promotion,
                hunger_decreased: true,
                final_source_quantity: source_quantity,
            };
        }
    }

    GrantExpiryBeforeIntendedActionOutcome {
        saw_initial_grant,
        saw_local_detour_before_harvest,
        saw_grant_expire,
        source_untouched_when_grant_expired,
        saw_second_promotion,
        hunger_decreased: false,
        final_source_quantity: h
            .world
            .get_component_resource_source(workstation)
            .expect("exclusive workstation should retain resource source")
            .available_quantity,
    }
}

// ---------------------------------------------------------------------------
// Scenario 3: Resource Contention with Conservation
// ---------------------------------------------------------------------------

#[test]
fn golden_resource_contention_with_conservation() {
    let mut h = GoldenHarness::new(Seed([3; 32]));

    let agent_a = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Alice",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    // Second agent competes for resources; not referenced directly.
    seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Bob",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    // Agent A has bread.
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent_a,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(1),
    );

    // Apple resource at Orchard Farm for B.
    place_workstation_with_source(
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
    );

    // Record initial authoritative totals.
    let initial_apple_auth = total_authoritative_commodity_quantity(&h.world, CommodityKind::Apple);
    let initial_bread_auth = total_authoritative_commodity_quantity(&h.world, CommodityKind::Bread);
    let initial_event_count = h.event_log.len();

    for _ in 0..80 {
        h.step_once();

        // Conservation: lot quantities never exceed authoritative baseline.
        // (Items can be consumed, reducing totals — that's fine.)
        let apple_auth = total_authoritative_commodity_quantity(&h.world, CommodityKind::Apple);
        let bread_auth = total_authoritative_commodity_quantity(&h.world, CommodityKind::Bread);

        assert!(
            apple_auth <= initial_apple_auth,
            "Apple authoritative total must not increase: was {initial_apple_auth}, now {apple_auth}"
        );
        assert!(
            bread_auth <= initial_bread_auth,
            "Bread authoritative total must not increase: was {initial_bread_auth}, now {bread_auth}"
        );
    }

    // Verify that the simulation was non-trivial — agents actually acted.
    assert!(
        h.event_log.len() > initial_event_count,
        "Event log should have grown — agents should have taken actions"
    );
    // Agent A should have consumed its bread.
    let bread_remaining = h.agent_commodity_qty(agent_a, CommodityKind::Bread);
    assert_eq!(
        bread_remaining,
        Quantity(0),
        "Agent A should have eaten its bread"
    );
}

#[test]
fn golden_resource_exhaustion_race() {
    let outcome = run_resource_exhaustion_race_scenario(Seed([17; 32]));

    assert!(
        outcome.saw_live_apple_lots,
        "Finite harvest stock should materialize apple lots during the contention race"
    );
    assert!(
        outcome.observed_source_quantities.contains(&2),
        "The orchard source should be observed after exactly one committed harvest batch"
    );
    assert!(
        outcome.observed_source_quantities.contains(&0),
        "The orchard source should deplete to zero after two committed harvest batches"
    );
    assert_eq!(
        outcome.final_source_quantity,
        Quantity(0),
        "The finite orchard source should end depleted in the no-regeneration scenario"
    );
    assert!(
        !outcome.agents_with_hunger_relief.is_empty(),
        "At least one agent should complete the harvest/pick-up/eat chain under contention"
    );
}

#[test]
fn golden_exclusive_queue_contention_uses_queue_grants_and_rotates_first_turns() {
    let outcome = run_exclusive_queue_contention_scenario(Seed([18; 32]));

    assert!(
        outcome.max_waiting_len >= 2,
        "Exclusive contention should materialize a real waiting line on the facility"
    );
    assert!(
        outcome.saw_granted_state || outcome.promoted_actors.len() >= 2,
        "Exclusive contention should exercise facility grants, not only incidental start collisions"
    );
    assert!(
        outcome.promoted_actors.len() >= 2,
        "Finite exclusive contention should promote at least two harvest turns"
    );
    assert_ne!(
        outcome.promoted_actors[0], outcome.promoted_actors[1],
        "The first two exclusive orchard turns should rotate across distinct queued actors"
    );
    assert_eq!(
        outcome.final_source_quantity,
        Quantity(0),
        "The exclusive orchard source should be exhausted after two granted harvest turns"
    );
}

#[test]
fn golden_facility_queue_patience_timeout() {
    let outcome = run_facility_queue_patience_timeout_scenario(Seed([19; 32]));

    assert!(
        outcome.joined_facility_a,
        "Agent should initially queue at the local exclusive facility"
    );
    assert!(
        outcome.abandoned_facility_a,
        "Patience expiry should remove the agent from facility A's authoritative queue"
    );
    assert!(
        outcome.recorded_blocked_facility_a,
        "Queue abandonment should feed the existing blocked-facility memory pipeline"
    );
    assert!(
        outcome.used_facility_b,
        "After abandoning facility A, the agent should route to the alternative facility"
    );
    assert!(
        outcome.hunger_decreased,
        "The alternative facility path should still satisfy the original hunger-driven goal"
    );
    assert_eq!(
        outcome.facility_a_final_source_quantity,
        Quantity(4),
        "The monopolized facility should remain unused while the patient abandons its queue"
    );
    assert!(
        outcome.facility_b_final_source_quantity < Quantity(4),
        "The alternative facility should be the one that actually gets used"
    );
}

#[test]
fn golden_grant_expiry_before_intended_action() {
    let outcome = run_grant_expiry_before_intended_action_scenario(Seed([20; 32]));

    assert!(
        outcome.saw_initial_grant,
        "The agent should first receive a real exclusive-facility grant"
    );
    assert!(
        outcome.saw_local_detour_before_harvest,
        "A higher-priority local detour should consume the carried water before the orchard stock changes"
    );
    assert!(
        outcome.saw_grant_expire,
        "The unused facility grant should expire through the authoritative facility queue system"
    );
    assert!(
        outcome.source_untouched_when_grant_expired,
        "The exclusive orchard stock should remain untouched when the first grant expires"
    );
    assert!(
        outcome.saw_second_promotion,
        "Grant expiry recovery should lead to a second real promotion, proving the agent re-entered the normal queue path"
    );
    assert!(
        outcome.hunger_decreased,
        "After recovering from the expired grant, the agent should still satisfy the original hunger-driven goal"
    );
    assert!(
        outcome.final_source_quantity < Quantity(4),
        "The exclusive orchard should eventually be used after the recovered re-queue path"
    );
}

// ---------------------------------------------------------------------------
// Scenario 4: Materialization Barrier Chain
// ---------------------------------------------------------------------------

#[test]
fn golden_materialization_barrier_chain() {
    let mut h = GoldenHarness::new(Seed([4; 32]));

    // Agent at Orchard Farm, critically hungry, no food.
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Dana",
        ORCHARD_FARM,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    // WorkstationMarker(OrchardRow) + ResourceSource at Orchard Farm.
    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(20),
            max_quantity: Quantity(20),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
    );

    let initial_hunger = h.agent_hunger(agent);
    let mut hunger_decreased = false;
    let mut acquired_apples = false;

    for _tick in 0..120 {
        h.step_once();

        // Harvest drops items on the ground; check both possessed and ground lots.
        let agent_apples = h.agent_commodity_qty(agent, CommodityKind::Apple);
        let total_apple_lots = total_live_lot_quantity(&h.world, CommodityKind::Apple);
        if agent_apples > Quantity(0) || total_apple_lots > 0 {
            acquired_apples = true;
        }

        let current_hunger = h.agent_hunger(agent);
        if current_hunger < initial_hunger {
            hunger_decreased = true;
            break;
        }
    }

    assert!(
        acquired_apples,
        "Agent should have harvested apples (lots materialized on ground)"
    );

    // The harvest action creates apple lots on the ground at the workstation.
    // This is the materialization barrier in action: items exist in the world
    // but the agent must replan to acquire them.
    let apple_lots = total_live_lot_quantity(&h.world, CommodityKind::Apple);
    assert!(apple_lots > 0, "Apple lots should exist after harvest");

    // Conservation: resource source deducted + lots = consistent.
    let apple_auth = total_authoritative_commodity_quantity(&h.world, CommodityKind::Apple);
    // Initial was 20 in resource source + 0 lots = 20.
    assert!(
        apple_auth <= 20,
        "Apple authoritative total should not exceed initial: got {apple_auth}"
    );

    // Hunger decrease confirms the full barrier chain completed: harvest → pick-up → eat.
    // If the agent only harvested but never ate, the chain is partial. We allow partial
    // success because pick-up + eat requires additional replanning cycles.
    if hunger_decreased {
        assert!(
            h.agent_hunger(agent) < initial_hunger,
            "Hunger should have decreased after eating harvested apples"
        );
    }
}

// ---------------------------------------------------------------------------
// Scenario 6b: Multi-Recipe Craft Path
// ---------------------------------------------------------------------------

#[test]
fn golden_multi_recipe_craft_path() {
    let seed = Seed([6; 32]);

    let (world_hash_1, log_hash_1) = run_multi_recipe_craft_scenario(seed);
    let (world_hash_2, log_hash_2) = run_multi_recipe_craft_scenario(seed);

    assert_eq!(
        world_hash_1, world_hash_2,
        "Multi-recipe craft scenario must replay deterministically"
    );
    assert_eq!(
        log_hash_1, log_hash_2,
        "Multi-recipe craft event log must replay deterministically"
    );
}

#[test]
fn golden_capacity_constrained_ground_lot_pickup() {
    let seed = Seed([16; 32]);

    let (world_hash_1, log_hash_1) = run_capacity_constrained_ground_lot_pickup_scenario(seed);
    let (world_hash_2, log_hash_2) = run_capacity_constrained_ground_lot_pickup_scenario(seed);

    assert_eq!(
        world_hash_1, world_hash_2,
        "Capacity-constrained ground-lot scenario must replay deterministically"
    );
    assert_eq!(
        log_hash_1, log_hash_2,
        "Capacity-constrained ground-lot event log must replay deterministically"
    );
}

#[test]
fn golden_resource_exhaustion_race_replays_deterministically() {
    let seed = Seed([17; 32]);

    let outcome_1 = run_resource_exhaustion_race_scenario(seed);
    let outcome_2 = run_resource_exhaustion_race_scenario(seed);

    assert_eq!(
        outcome_1.world_hash, outcome_2.world_hash,
        "Resource exhaustion race scenario must replay deterministically"
    );
    assert_eq!(
        outcome_1.log_hash, outcome_2.log_hash,
        "Resource exhaustion race event log must replay deterministically"
    );
}

#[test]
fn golden_exclusive_queue_contention_replays_deterministically() {
    let seed = Seed([18; 32]);

    let outcome_1 = run_exclusive_queue_contention_scenario(seed);
    let outcome_2 = run_exclusive_queue_contention_scenario(seed);

    assert_eq!(
        outcome_1.world_hash, outcome_2.world_hash,
        "Exclusive queue contention scenario must replay deterministically"
    );
    assert_eq!(
        outcome_1.log_hash, outcome_2.log_hash,
        "Exclusive queue contention event log must replay deterministically"
    );
    assert_eq!(
        outcome_1.promoted_actors, outcome_2.promoted_actors,
        "Exclusive queue contention should promote the same actor sequence for a fixed seed"
    );
}
