//! Shared 20-agent, 10-place world setup used by soak, resilience,
//! and replay-determinism test binaries.

use super::*;
use std::collections::BTreeSet;
use std::num::{NonZeroU8, NonZeroU32};
use worldwake_core::{
    BanditCamp, BanditFactionPolicy, CombatProfile, CommodityKind, DemandMemory, EligibilityRule,
    EntityId, HomeostaticNeeds, JusticeDispositionProfile, MerchandiseProfile, MetabolismProfile,
    PatrolProfile, PatrolRoute, PerceptionProfile, PerceptionSource, PlaceTag, PursuitProfile,
    Quantity, ResourceSource, Seed, SuccessionLaw, TellProfile, TheftDispositionProfile, Tick,
    Topology, TradeDispositionProfile, TravelEdge, TravelEdgeId, UtilityProfile,
    ViolationDispositionProfile, WorkstationTag,
};
use worldwake_sim::ControllerState;

pub const fn entity(slot: u32) -> EntityId {
    EntityId {
        slot,
        generation: 0,
    }
}

fn place(name: &str, tags: &[PlaceTag]) -> worldwake_core::Place {
    worldwake_core::Place {
        name: name.to_string(),
        capacity: None,
        tags: tags.iter().copied().collect(),
    }
}

pub fn build_harness_with_topology(seed: Seed, topology: Topology) -> GoldenHarness {
    let mut h = GoldenHarness::new(seed);
    h.world = worldwake_core::World::new(topology).unwrap();
    h.event_log = worldwake_core::EventLog::new();
    h.scheduler = worldwake_sim::Scheduler::new(worldwake_sim::SystemManifest::canonical());
    h.controller = ControllerState::new();
    h
}

// T30 place entity IDs (outside prototype and other scenario ranges).
pub const PLACE_T30_HUB: EntityId = entity(200);
pub const PLACE_T30_MARKET: EntityId = entity(201);
pub const PLACE_T30_FARM: EntityId = entity(202);
pub const PLACE_T30_FORGE: EntityId = entity(203);
pub const PLACE_T30_BARRACKS: EntityId = entity(204);
pub const PLACE_T30_RULERS_HALL: EntityId = entity(205);
pub const PLACE_T30_FOREST: EntityId = entity(206);
pub const PLACE_T30_BANDIT_CAMP: EntityId = entity(207);
pub const PLACE_T30_ROAD: EntityId = entity(208);
pub const PLACE_T30_ORCHARD: EntityId = entity(209);

/// 10-place topology for the soak test.
///
/// ```text
///   Hub -- Market -- Road -- Forest -- BanditCamp
///    |       |                  |
///   Farm   Forge             Orchard
///    |
///  Barracks -- RulersHall
/// ```
pub fn build_t30_topology() -> Topology {
    let mut t = Topology::new();
    t.add_place(PLACE_T30_HUB, place("T30Hub", &[PlaceTag::Village]))
        .unwrap();
    t.add_place(
        PLACE_T30_MARKET,
        place("T30Market", &[PlaceTag::Store, PlaceTag::Village]),
    )
    .unwrap();
    t.add_place(
        PLACE_T30_FARM,
        place("T30Farm", &[PlaceTag::Farm, PlaceTag::Field]),
    )
    .unwrap();
    t.add_place(PLACE_T30_FORGE, place("T30Forge", &[PlaceTag::Village]))
        .unwrap();
    t.add_place(
        PLACE_T30_BARRACKS,
        place("T30Barracks", &[PlaceTag::Barracks, PlaceTag::Village]),
    )
    .unwrap();
    t.add_place(
        PLACE_T30_RULERS_HALL,
        place("T30RulersHall", &[PlaceTag::Hall, PlaceTag::Village]),
    )
    .unwrap();
    t.add_place(
        PLACE_T30_FOREST,
        place("T30Forest", &[PlaceTag::Forest, PlaceTag::Trail]),
    )
    .unwrap();
    t.add_place(
        PLACE_T30_BANDIT_CAMP,
        place("T30BanditCamp", &[PlaceTag::Camp, PlaceTag::Forest]),
    )
    .unwrap();
    t.add_place(PLACE_T30_ROAD, place("T30Road", &[PlaceTag::Road]))
        .unwrap();
    t.add_place(
        PLACE_T30_ORCHARD,
        place("T30Orchard", &[PlaceTag::Farm, PlaceTag::Field]),
    )
    .unwrap();

    let mut edge_id = 400u32;
    let mut add = |t: &mut Topology, a: EntityId, b: EntityId, ticks: u32| {
        t.add_edge(TravelEdge::new(TravelEdgeId(edge_id), a, b, ticks, None).unwrap())
            .unwrap();
        edge_id += 1;
        t.add_edge(TravelEdge::new(TravelEdgeId(edge_id), b, a, ticks, None).unwrap())
            .unwrap();
        edge_id += 1;
    };
    add(&mut t, PLACE_T30_HUB, PLACE_T30_MARKET, 2);
    add(&mut t, PLACE_T30_HUB, PLACE_T30_FARM, 3);
    add(&mut t, PLACE_T30_FARM, PLACE_T30_BARRACKS, 2);
    add(&mut t, PLACE_T30_BARRACKS, PLACE_T30_RULERS_HALL, 1);
    add(&mut t, PLACE_T30_MARKET, PLACE_T30_FORGE, 1);
    add(&mut t, PLACE_T30_MARKET, PLACE_T30_ROAD, 3);
    add(&mut t, PLACE_T30_ROAD, PLACE_T30_FOREST, 4);
    add(&mut t, PLACE_T30_FOREST, PLACE_T30_BANDIT_CAMP, 3);
    add(&mut t, PLACE_T30_FOREST, PLACE_T30_ORCHARD, 2);
    t
}

pub fn t30_default_perception() -> PerceptionProfile {
    PerceptionProfile {
        entity_activation_threshold: pm(22),
        claim_confidence_threshold: pm(50),
        observation_buffer_capacity: 40,
        observation_budget: 24,
        salience_policy: worldwake_core::SaliencePolicy::default(),
        omission_log_capacity: worldwake_core::default_omission_log_capacity(),
        need_salience_boost: pm(500),
        need_salience_urgency_threshold: pm(500),
        observation_fidelity: pm(800),
        institutional_memory_capacity: 10,
        consultation_speed_factor: pm(500),
        contradiction_tolerance: pm(300),
        ..PerceptionProfile::default()
    }
}

pub fn t30_default_metabolism() -> MetabolismProfile {
    MetabolismProfile {
        hunger_rate: pm(3),
        thirst_rate: pm(2),
        fatigue_rate: pm(1),
        bladder_rate: pm(2),
        dirtiness_rate: pm(1),
        ..MetabolismProfile::default()
    }
}

/// Build the full T30 world and return the harness plus key entity IDs.
pub fn build_t30_world(
    seed: Seed,
) -> (
    GoldenHarness,
    Vec<EntityId>, // all agents
    EntityId,      // ruling_faction
    EntityId,      // bandit_faction
    EntityId,      // office
) {
    let mut h = build_harness_with_topology(seed, build_t30_topology());

    // --- Factions ---
    let ruling_faction;
    let bandit_faction;
    {
        let mut txn = new_txn(&mut h.world, 0);
        ruling_faction = txn.create_faction("T30 Town Ward").unwrap();
        bandit_faction = txn.create_faction("T30 Forest Bandits").unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    // --- Office ---
    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "Town Leader",
        PLACE_T30_RULERS_HALL,
        SuccessionLaw::Force,
        48,
        vec![EligibilityRule::FactionMember(ruling_faction)],
    );

    // --- Workstations & resources ---
    // Farm: orchard row for apples
    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        PLACE_T30_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            max_quantity: Quantity(30),
            available_quantity: Quantity(20),
            regeneration_ticks_per_unit: std::num::NonZeroU32::new(50),
            last_regeneration_tick: None,
            extraction_slots: NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: NonZeroU32::new(1).unwrap(),
            quality: None,
        },
        ProductionOutputOwner::Actor,
    );
    // Orchard: another apple source
    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        PLACE_T30_ORCHARD,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            max_quantity: Quantity(20),
            available_quantity: Quantity(15),
            regeneration_ticks_per_unit: std::num::NonZeroU32::new(80),
            last_regeneration_tick: None,
            extraction_slots: NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: NonZeroU32::new(1).unwrap(),
            quality: None,
        },
        ProductionOutputOwner::Actor,
    );
    // Farm: field plot for grain
    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        PLACE_T30_FARM,
        WorkstationTag::FieldPlot,
        ResourceSource {
            commodity: CommodityKind::Grain,
            max_quantity: Quantity(20),
            available_quantity: Quantity(15),
            regeneration_ticks_per_unit: std::num::NonZeroU32::new(60),
            last_regeneration_tick: None,
            extraction_slots: NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: NonZeroU32::new(1).unwrap(),
            quality: None,
        },
        ProductionOutputOwner::Actor,
    );
    // Forge: mill for bread
    place_workstation(
        &mut h.world,
        &mut h.event_log,
        PLACE_T30_FORGE,
        WorkstationTag::Mill,
        ProductionOutputOwner::Actor,
    );

    let mut all_agents = Vec::new();

    // Helper for default agent setup with perception + tell + belief seeding.
    let setup_agent = |h: &mut GoldenHarness, agent: EntityId| {
        set_agent_perception_profile(
            &mut h.world,
            &mut h.event_log,
            agent,
            t30_default_perception(),
        );
        set_agent_tell_profile(
            &mut h.world,
            &mut h.event_log,
            agent,
            TellProfile::default(),
        );
    };

    // --- 1 Ruler ---
    let ruler = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "T30Ruler",
        PLACE_T30_RULERS_HALL,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile {
            hunger_rate: pm(1),
            thirst_rate: pm(1),
            ..MetabolismProfile::default()
        },
        UtilityProfile {
            enterprise_weight: pm(700),
            social_weight: pm(500),
            courage: pm(800),
            ..UtilityProfile::default()
        },
    );
    setup_agent(&mut h, ruler);
    add_faction_membership(&mut h.world, &mut h.event_log, ruler, ruling_faction);
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.assign_office(office, ruler).unwrap();
        txn.set_component_office_data(
            office,
            worldwake_core::OfficeData {
                title: "Town Leader".to_string(),
                seat: PLACE_T30_RULERS_HALL,
                jurisdiction: BTreeSet::from([PLACE_T30_RULERS_HALL]),
                succession_law: SuccessionLaw::Force,
                eligibility_rules: vec![EligibilityRule::FactionMember(ruling_faction)],
                succession_period_ticks: 48,
                vacancy_since: None,
            },
        )
        .unwrap();
        txn.set_component_combat_profile(
            ruler,
            CombatProfile::new(
                pm(600),
                pm(700),
                pm(400),
                pm(400),
                pm(100),
                pm(200),
                pm(50),
                pm(150),
                pm(50),
                std::num::NonZeroU32::new(5).unwrap(),
                std::num::NonZeroU32::new(3).unwrap(),
            ),
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    all_agents.push(ruler);

    // --- 2 Claimants ---
    for i in 0..2 {
        let claimant = seed_agent(
            &mut h.world,
            &mut h.event_log,
            &format!("T30Claimant{}", i + 1),
            PLACE_T30_HUB,
            HomeostaticNeeds::new_sated(),
            t30_default_metabolism(),
            UtilityProfile {
                enterprise_weight: pm(800),
                social_weight: pm(400),
                courage: pm(700),
                ..UtilityProfile::default()
            },
        );
        setup_agent(&mut h, claimant);
        add_faction_membership(&mut h.world, &mut h.event_log, claimant, ruling_faction);
        all_agents.push(claimant);
    }

    // --- 1 Merchant ---
    let merchant = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "T30Merchant",
        PLACE_T30_MARKET,
        HomeostaticNeeds::new_sated(),
        t30_default_metabolism(),
        UtilityProfile {
            enterprise_weight: pm(900),
            social_weight: pm(300),
            ..UtilityProfile::default()
        },
    );
    setup_agent(&mut h, merchant);
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_merchandise_profile(
            merchant,
            MerchandiseProfile {
                sale_kinds: [CommodityKind::Apple, CommodityKind::Bread]
                    .into_iter()
                    .collect(),
                home_facility: Some(PLACE_T30_MARKET),
            },
        )
        .unwrap();
        txn.set_component_trade_disposition_profile(
            merchant,
            TradeDispositionProfile {
                negotiation_round_ticks: std::num::NonZeroU32::new(3).unwrap(),
                initial_offer_bias: pm(600),
                concession_rate: pm(100),
                rejection_escalation_rate: pm(200),
                demand_memory_retention_ticks: 500,
                market_presence_ticks: std::num::NonZeroU32::new(30).unwrap(),
            },
        )
        .unwrap();
        txn.set_component_demand_memory(
            merchant,
            DemandMemory {
                observations: vec![],
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        merchant,
        PLACE_T30_MARKET,
        CommodityKind::Apple,
        Quantity(8),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        merchant,
        PLACE_T30_MARKET,
        CommodityKind::Bread,
        Quantity(5),
    );
    all_agents.push(merchant);

    // --- 1 Carrier ---
    let carrier = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "T30Carrier",
        PLACE_T30_FARM,
        HomeostaticNeeds::new_sated(),
        t30_default_metabolism(),
        UtilityProfile {
            enterprise_weight: pm(600),
            ..UtilityProfile::default()
        },
    );
    setup_agent(&mut h, carrier);
    all_agents.push(carrier);

    // --- 3 Guards ---
    for i in 0..3 {
        let guard_place = match i {
            0 => PLACE_T30_BARRACKS,
            1 => PLACE_T30_MARKET,
            _ => PLACE_T30_ROAD,
        };
        let guard = seed_agent(
            &mut h.world,
            &mut h.event_log,
            &format!("T30Guard{}", i + 1),
            guard_place,
            HomeostaticNeeds::new_sated(),
            t30_default_metabolism(),
            UtilityProfile {
                courage: pm(900),
                danger_weight: pm(200),
                ..UtilityProfile::default()
            },
        );
        setup_agent(&mut h, guard);
        {
            let mut txn = new_txn(&mut h.world, 0);
            txn.set_component_patrol_route(
                guard,
                PatrolRoute {
                    assigned_places: vec![PLACE_T30_MARKET, PLACE_T30_ROAD, PLACE_T30_BARRACKS],
                    current_index: i,
                },
            )
            .unwrap();
            txn.set_component_patrol_profile(
                guard,
                PatrolProfile {
                    base_dwell_ticks: 10,
                    dwell_vigilance_scale_ticks: 5,
                    vigilance: pm(700),
                    route_adaptation_sensitivity: pm(300),
                    patrol_motive_weight: pm(600),
                },
            )
            .unwrap();
            txn.set_component_combat_profile(
                guard,
                CombatProfile::new(
                    pm(700),
                    pm(800),
                    pm(500),
                    pm(500),
                    pm(150),
                    pm(250),
                    pm(80),
                    pm(200),
                    pm(80),
                    std::num::NonZeroU32::new(4).unwrap(),
                    std::num::NonZeroU32::new(3).unwrap(),
                ),
            )
            .unwrap();
            txn.set_component_violation_disposition_profile(
                guard,
                ViolationDispositionProfile {
                    investigation_duration_ticks: std::num::NonZeroU32::new(5).unwrap(),
                    violation_memory_retention_ticks: 500,
                    investigation_motive_weight: pm(500),
                    ownership_motive_bonus: pm(200),
                },
            )
            .unwrap();
            txn.set_component_justice_disposition_profile(
                guard,
                JusticeDispositionProfile {
                    accusation_motive_weight: pm(500),
                    fine_severity: pm(300),
                },
            )
            .unwrap();
            commit_txn(txn, &mut h.event_log);
        }
        all_agents.push(guard);
    }

    // --- 3 Bandits ---
    // Place edible supplies at bandit camp for camp establishment.
    // Faction-owned: create lot, place at camp, assign possessor to faction.
    // BanditCamp component belongs on the Place entity (per component schema),
    // not on individual bandit agents. Agents carry BanditFactionPolicy instead.
    let bandit_supplies;
    {
        let mut txn = new_txn(&mut h.world, 0);
        bandit_supplies = txn
            .create_item_lot(CommodityKind::Bread, Quantity(5))
            .unwrap();
        txn.set_ground_location(bandit_supplies, PLACE_T30_BANDIT_CAMP)
            .unwrap();
        txn.set_possessor(bandit_supplies, bandit_faction).unwrap();
        txn.set_component_bandit_camp(
            PLACE_T30_BANDIT_CAMP,
            BanditCamp {
                faction: bandit_faction,
                supplies: bandit_supplies,
                empty_since_tick: None,
            },
        )
        .unwrap();
        txn.set_component_bandit_faction_policy(
            bandit_faction,
            BanditFactionPolicy {
                min_regroup_count: 2,
                establishment_duration_ticks: std::num::NonZeroU32::new(10).unwrap(),
                abandonment_grace_ticks: std::num::NonZeroU32::new(50).unwrap(),
                flee_wound_threshold: pm(600),
                rally_place: Some(PLACE_T30_FOREST),
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    for i in 0..3 {
        let bandit = seed_agent(
            &mut h.world,
            &mut h.event_log,
            &format!("T30Bandit{}", i + 1),
            PLACE_T30_BANDIT_CAMP,
            HomeostaticNeeds {
                hunger: pm(300),
                thirst: pm(200),
                ..HomeostaticNeeds::new_sated()
            },
            t30_default_metabolism(),
            UtilityProfile {
                courage: pm(800),
                danger_weight: pm(300),
                enterprise_weight: pm(500),
                ..UtilityProfile::default()
            },
        );
        setup_agent(&mut h, bandit);
        add_faction_membership(&mut h.world, &mut h.event_log, bandit, bandit_faction);
        {
            let mut txn = new_txn(&mut h.world, 0);
            txn.set_component_combat_profile(
                bandit,
                CombatProfile::new(
                    pm(500),
                    pm(600),
                    pm(400),
                    pm(300),
                    pm(100),
                    pm(150),
                    pm(40),
                    pm(180),
                    pm(60),
                    std::num::NonZeroU32::new(4).unwrap(),
                    std::num::NonZeroU32::new(3).unwrap(),
                ),
            )
            .unwrap();
            txn.set_component_pursuit_profile(
                bandit,
                PursuitProfile {
                    min_location_confidence: pm(300),
                    max_pursuit_travel_ticks: std::num::NonZeroU32::new(12).unwrap(),
                },
            )
            .unwrap();
            commit_txn(txn, &mut h.event_log);
        }
        all_agents.push(bandit);
    }

    // --- 2 Thieves ---
    for i in 0..2 {
        let thief_place = if i == 0 {
            PLACE_T30_MARKET
        } else {
            PLACE_T30_HUB
        };
        let thief = seed_agent(
            &mut h.world,
            &mut h.event_log,
            &format!("T30Thief{}", i + 1),
            thief_place,
            HomeostaticNeeds {
                hunger: pm(400),
                thirst: pm(300),
                ..HomeostaticNeeds::new_sated()
            },
            t30_default_metabolism(),
            UtilityProfile {
                enterprise_weight: pm(500),
                courage: pm(400),
                ..UtilityProfile::default()
            },
        );
        setup_agent(&mut h, thief);
        {
            let mut txn = new_txn(&mut h.world, 0);
            txn.set_component_theft_disposition_profile(
                thief,
                TheftDispositionProfile {
                    steal_duration_ticks: std::num::NonZeroU32::new(3).unwrap(),
                    theft_motive_weight: pm(700),
                    witness_risk_penalty: pm(200),
                },
            )
            .unwrap();
            commit_txn(txn, &mut h.event_log);
        }
        all_agents.push(thief);
    }

    // --- 4 Civilians ---
    for i in 0..4 {
        let civ_place = match i {
            0 => PLACE_T30_HUB,
            1 => PLACE_T30_MARKET,
            2 => PLACE_T30_FARM,
            _ => PLACE_T30_FORGE,
        };
        let civ = seed_agent(
            &mut h.world,
            &mut h.event_log,
            &format!("T30Civ{}", i + 1),
            civ_place,
            HomeostaticNeeds {
                hunger: pm(500 + u16::try_from(i).unwrap() * 50),
                thirst: pm(300 + u16::try_from(i).unwrap() * 30),
                ..HomeostaticNeeds::new_sated()
            },
            t30_default_metabolism(),
            UtilityProfile {
                social_weight: pm(400),
                enterprise_weight: pm(300),
                ..UtilityProfile::default()
            },
        );
        setup_agent(&mut h, civ);
        // Give civilians some coins for trade.
        give_commodity(
            &mut h.world,
            &mut h.event_log,
            civ,
            civ_place,
            CommodityKind::Coin,
            Quantity(5),
        );
        all_agents.push(civ);
    }

    // --- 3 Workers ---
    for i in 0..3 {
        let worker_place = match i {
            0 => PLACE_T30_FARM,
            1 => PLACE_T30_FORGE,
            _ => PLACE_T30_ORCHARD,
        };
        let worker = seed_agent(
            &mut h.world,
            &mut h.event_log,
            &format!("T30Worker{}", i + 1),
            worker_place,
            HomeostaticNeeds {
                hunger: pm(400),
                thirst: pm(300),
                ..HomeostaticNeeds::new_sated()
            },
            t30_default_metabolism(),
            UtilityProfile {
                enterprise_weight: pm(700),
                ..UtilityProfile::default()
            },
        );
        setup_agent(&mut h, worker);
        all_agents.push(worker);
    }

    // --- Seed initial beliefs for all agents ---
    for &agent in &all_agents {
        seed_actor_local_beliefs(
            &mut h.world,
            &mut h.event_log,
            agent,
            Tick(0),
            PerceptionSource::DirectObservation,
        );
    }

    (h, all_agents, ruling_faction, bandit_faction, office)
}
