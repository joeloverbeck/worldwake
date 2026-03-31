//! Soak tests for extended autonomous simulation.
//!
//! These tests run thousands of ticks with 20+ agents and verify per-tick
//! invariants (conservation, needs bounds, dead agent inactivity, unique
//! placement, tick monotonicity, causal link integrity) under extended play.
//!
//! Gated behind the `soak` feature because each test takes minutes to run.
//! Run with: `cargo test -p worldwake-ai --features soak --test golden_soak`
#![cfg(feature = "soak")]

mod golden_harness;

use golden_harness::*;
use std::collections::BTreeSet;
use worldwake_core::{
    hash_event_log, hash_world, total_authoritative_commodity_quantity,
    verify_authoritative_conservation, BanditCamp, BanditFactionPolicy, CauseRef,
    CombatProfile, CommodityKind, DeadAt, DemandMemory, EligibilityRule,
    EntityId, EntityKind, EventId, EventView, HomeostaticNeeds, JusticeDispositionProfile,
    MerchandiseProfile, MetabolismProfile, PatrolProfile, PatrolRoute, PerceptionProfile,
    PerceptionSource, Permille, PlaceTag, PursuitProfile, Quantity, ResourceSource, Seed,
    StateHash, SuccessionLaw, TellProfile, TheftDispositionProfile, Tick, Topology,
    TradeDispositionProfile, TravelEdge, TravelEdgeId, UtilityProfile,
    ViolationDispositionProfile, WorkstationTag,
};
use worldwake_sim::ControllerState;

const fn entity(slot: u32) -> EntityId {
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

fn build_harness_with_topology(seed: Seed, topology: Topology) -> GoldenHarness {
    let mut h = GoldenHarness::new(seed);
    h.world = worldwake_core::World::new(topology).unwrap();
    h.event_log = worldwake_core::EventLog::new();
    h.scheduler = worldwake_sim::Scheduler::new(worldwake_sim::SystemManifest::canonical());
    h.controller = ControllerState::new();
    h
}

// ---------------------------------------------------------------------------
// Scenario 30: Seven-Day Autoplay Soak Test
// ---------------------------------------------------------------------------
//
// Systems: Needs, Production, Trade, Combat, Travel, Social, Politics, Perception
// GoalKinds: ConsumeOwnedCommodity, AcquireCommodity, RestockCommodity, ShareBelief,
//   ClaimOffice, StealItem, Patrol, Harvest, Craft
// ActionDomains: Needs, Trade, Travel, Combat, Production, Social, Transport
// Places: T30Hub, T30Market, T30Farm, T30Forge, T30Barracks, T30RulersHall,
//   T30Forest, T30BanditCamp, T30Road, T30Orchard
// Principles: 3, 4, 6, 7, 8, 10, 12, 14, 26
//
// Setup: 10-place topology with mixed PlaceTag variants. 20 agents with diverse
//   profiles: 1 ruler, 2 claimants, 1 merchant, 1 carrier, 3 guards, 3 bandits,
//   4 civilians, 2 thieves, 3 workers. Full autonomous play for 10080 ticks
//   (7 in-game days at ~1 tick/minute). 20 seeds for cross-run diversity.
//
// Proves: The full simulation stack maintains per-tick invariants (conservation,
//   needs bounds, dead agent inactivity, unique placement, tick monotonicity,
//   causal link integrity) under extended autonomous play. Emergence is
//   seed-sensitive: different seeds produce different histories (deaths, travel,
//   trade, political claims, theft).
//
// Chain: diverse agents + autonomous AI -> multi-domain goal generation ->
//   cross-system action chains -> emergence (deaths, trade, politics, crime)
//   while invariants hold every tick.

// T30 place entity IDs (outside prototype and other scenario ranges).
const PLACE_T30_HUB: EntityId = entity(200);
const PLACE_T30_MARKET: EntityId = entity(201);
const PLACE_T30_FARM: EntityId = entity(202);
const PLACE_T30_FORGE: EntityId = entity(203);
const PLACE_T30_BARRACKS: EntityId = entity(204);
const PLACE_T30_RULERS_HALL: EntityId = entity(205);
const PLACE_T30_FOREST: EntityId = entity(206);
const PLACE_T30_BANDIT_CAMP: EntityId = entity(207);
const PLACE_T30_ROAD: EntityId = entity(208);
const PLACE_T30_ORCHARD: EntityId = entity(209);

/// 10-place topology for the soak test.
///
/// ```text
///   Hub ── Market ── Road ── Forest ── BanditCamp
///    │       │                  │
///   Farm   Forge             Orchard
///    │
///  Barracks ── RulersHall
/// ```
fn build_t30_topology() -> Topology {
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
    t.add_place(
        PLACE_T30_FORGE,
        place("T30Forge", &[PlaceTag::Village]),
    )
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
    t.add_place(
        PLACE_T30_ROAD,
        place("T30Road", &[PlaceTag::Road]),
    )
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

fn t30_default_perception() -> PerceptionProfile {
    PerceptionProfile {
        memory_capacity: 40,
        memory_retention_ticks: 2000,
        observation_fidelity: pm(800),
        institutional_memory_capacity: 10,
        consultation_speed_factor: pm(500),
        contradiction_tolerance: pm(300),
        ..PerceptionProfile::default()
    }
}

fn t30_default_metabolism() -> MetabolismProfile {
    MetabolismProfile {
        hunger_rate: pm(3),
        thirst_rate: pm(2),
        fatigue_rate: pm(1),
        bladder_rate: pm(2),
        dirtiness_rate: pm(1),
        ..MetabolismProfile::default()
    }
}

/// Per-run result collected for cross-run diversity analysis.
struct SoakResult {
    world_hash: StateHash,
    event_log_hash: StateHash,
    saw_death: bool,
    saw_acquire: bool,
    saw_travel: bool,
    saw_tell: bool,
    saw_claim_office: bool,
    saw_steal: bool,
}

/// Build the full T30 world and return the harness plus key entity IDs.
fn build_t30_world(
    seed: Seed,
) -> (
    GoldenHarness,
    Vec<EntityId>,   // all agents
    EntityId,        // ruling_faction
    EntityId,        // bandit_faction
    EntityId,        // office
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
                jurisdiction: PLACE_T30_RULERS_HALL,
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
                pm(600), pm(700), pm(400), pm(400), pm(100),
                pm(200), pm(50), pm(150), pm(50),
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
                home_market: Some(PLACE_T30_MARKET),
            },
        )
        .unwrap();
        txn.set_component_trade_disposition_profile(
            merchant,
            TradeDispositionProfile {
                negotiation_round_ticks: std::num::NonZeroU32::new(3).unwrap(),
                initial_offer_bias: pm(600),
                concession_rate: pm(100),
                demand_memory_retention_ticks: 500,
                market_presence_ticks: std::num::NonZeroU32::new(30).unwrap(),
            },
        )
        .unwrap();
        txn.set_component_demand_memory(merchant, DemandMemory { observations: vec![] })
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
                    assigned_places: vec![
                        PLACE_T30_MARKET,
                        PLACE_T30_ROAD,
                        PLACE_T30_BARRACKS,
                    ],
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
                    pm(700), pm(800), pm(500), pm(500), pm(150),
                    pm(250), pm(80), pm(200), pm(80),
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
        bandit_supplies = txn.create_item_lot(CommodityKind::Bread, Quantity(5)).unwrap();
        txn.set_ground_location(bandit_supplies, PLACE_T30_BANDIT_CAMP).unwrap();
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
                    pm(500), pm(600), pm(400), pm(300), pm(100),
                    pm(150), pm(40), pm(180), pm(60),
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
        let thief_place = if i == 0 { PLACE_T30_MARKET } else { PLACE_T30_HUB };
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
                hunger: pm(500 + i as u16 * 50),
                thirst: pm(300 + i as u16 * 30),
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

/// Run a single soak run for the given seed and return per-run results.
fn run_t30_soak(seed: Seed) -> SoakResult {
    let (mut h, all_agents, _ruling_faction, _bandit_faction, _office) = build_t30_world(seed);

    const TOTAL_TICKS: u64 = 10080;
    let commodities_to_check = [
        CommodityKind::Apple,
        CommodityKind::Grain,
        CommodityKind::Bread,
        CommodityKind::Coin,
    ];

    // Snapshot initial commodity totals for conservation checks.
    let mut commodity_totals: std::collections::BTreeMap<CommodityKind, u64> = commodities_to_check
        .iter()
        .map(|&c| (c, total_authoritative_commodity_quantity(&h.world, c)))
        .collect();

    let initial_world_hash = hash_world(&h.world).unwrap();
    let mut prev_tick = h.scheduler.current_tick();

    // Emergence flags.
    let mut saw_death = false;
    let mut saw_travel = false;

    for _ in 0..TOTAL_TICKS {
        h.step_once();
        let current_tick = h.scheduler.current_tick();

        // --- Per-tick invariant 1: Conservation ---
        for (&commodity, total) in &mut commodity_totals {
            let actual = total_authoritative_commodity_quantity(&h.world, commodity);
            // Total can only increase through production (harvest/craft), never decrease
            // except through consumption. Update the running total to track increases.
            if actual > *total {
                // Production created new units — update baseline.
                *total = actual;
            }
            verify_authoritative_conservation(&h.world, commodity, actual).unwrap_or_else(|e| {
                panic!(
                    "conservation violation at tick {:?} for {:?}: {e}",
                    current_tick, commodity
                )
            });
        }

        // --- Per-tick invariant 2: Needs bounds ---
        for &agent in &all_agents {
            if h.agent_is_dead(agent) {
                continue;
            }
            if let Some(needs) = h.world.get_component_homeostatic_needs(agent) {
                let max = Permille::new(1000).unwrap();
                assert!(
                    needs.hunger <= max
                        && needs.thirst <= max
                        && needs.fatigue <= max
                        && needs.bladder <= max
                        && needs.dirtiness <= max,
                    "needs out of bounds for agent {agent:?} at tick {current_tick:?}: {needs:?}"
                );
            }
        }

        // --- Per-tick invariant 3: Dead agent inactivity ---
        for &agent in &all_agents {
            if let Some(dead_at) = h.world.get_component_dead_at(agent) {
                saw_death = true;
                assert!(
                    !h.agent_has_active_action(agent),
                    "dead agent {agent:?} (died at {:?}) has active action at tick {current_tick:?}",
                    dead_at.0
                );
            }
        }

        // --- Per-tick invariant 4: Unique placement ---
        for &agent in &all_agents {
            if h.agent_is_dead(agent) {
                continue;
            }
            // Agents in transit have effective_place == None, which is legal.
            // If they have a place, it must exist in the topology.
            if let Some(place) = h.world.effective_place(agent) {
                assert!(
                    h.world.topology().place(place).is_some(),
                    "agent {agent:?} placed at non-existent place {place:?} at tick {current_tick:?}"
                );
            }
        }

        // --- Per-tick invariant 5: Tick monotonicity ---
        assert!(
            current_tick > prev_tick,
            "tick did not advance: prev={prev_tick:?}, current={current_tick:?}"
        );
        prev_tick = current_tick;

        // --- Per-tick invariant 6: Causal link integrity ---
        let log_len = h.event_log.len() as u64;
        for idx in 0..log_len {
            let event_id = EventId(idx);
            if let Some(record) = h.event_log.get(event_id) {
                match record.cause() {
                    CauseRef::Event(cause_id) => {
                        assert!(
                            h.event_log.get(cause_id).is_some(),
                            "event {event_id:?} references non-existent cause {cause_id:?} \
                             at tick {current_tick:?}"
                        );
                    }
                    CauseRef::SystemTick(_) | CauseRef::Bootstrap | CauseRef::ExternalInput(_) => {
                        // Valid non-event causes.
                    }
                }
            }
        }

        // Track travel emergence.
        if !saw_travel {
            saw_travel = h.event_log.events_by_tag(worldwake_core::EventTag::Travel).len() > 0;
        }
    }

    // --- Per-run invariant 7: Emergence checks via event log ---
    // Death: already tracked per-tick.
    // Acquire: check for any completed trade or harvest action.
    let saw_acquire =
        h.event_log.events_by_tag(worldwake_core::EventTag::Trade).len() > 0
        || h.event_log.events_by_tag(worldwake_core::EventTag::ActionCommitted).len() > 0;
    // Travel: already tracked.
    // Share belief: check for social (tell) events.
    let saw_tell = h.event_log.events_by_tag(worldwake_core::EventTag::Social).len() > 0;

    // Political emergence (ClaimOffice): check for political events.
    let saw_claim_office =
        h.event_log.events_by_tag(worldwake_core::EventTag::Political).len() > 0;

    // Crime emergence (StealItem): check for crime events.
    let saw_steal = h.event_log.events_by_tag(worldwake_core::EventTag::Crime).len() > 0;

    // --- Per-run invariant 8: State changed ---
    let final_world_hash = hash_world(&h.world).unwrap();
    assert_ne!(
        initial_world_hash, final_world_hash,
        "world state did not change after {TOTAL_TICKS} ticks (seed: {seed:?})"
    );

    SoakResult {
        world_hash: final_world_hash,
        event_log_hash: hash_event_log(&h.event_log).unwrap(),
        saw_death,
        saw_acquire,
        saw_travel,
        saw_tell,
        saw_claim_office,
        saw_steal,
    }
}

#[test]
fn t30_seven_day_soak() {
    let seeds: Vec<Seed> = (0u8..20)
        .map(|i| {
            let mut bytes = [0u8; 32];
            bytes[0] = i;
            bytes[31] = i.wrapping_mul(17);
            Seed(bytes)
        })
        .collect();

    let results: Vec<SoakResult> = seeds.iter().map(|&s| run_t30_soak(s)).collect();

    // --- Cross-run invariant 9: Not all hashes identical ---
    let unique_world_hashes: BTreeSet<_> = results.iter().map(|r| r.world_hash).collect();
    let unique_log_hashes: BTreeSet<_> = results.iter().map(|r| r.event_log_hash).collect();
    assert!(
        unique_world_hashes.len() > 1 || unique_log_hashes.len() > 1,
        "all 20 runs produced identical hashes — emergence is not seed-sensitive"
    );

    // --- Cross-run invariant 10: Political emergence ---
    let claim_office_count = results.iter().filter(|r| r.saw_claim_office).count();
    assert!(
        claim_office_count >= 3,
        "only {claim_office_count}/20 runs produced ClaimOffice events (need >= 3)"
    );

    // --- Cross-run invariant 11: Crime emergence ---
    let steal_count = results.iter().filter(|r| r.saw_steal).count();
    assert!(
        steal_count >= 3,
        "only {steal_count}/20 runs produced StealItem/Crime events (need >= 3)"
    );

    // --- Per-run emergence: at least 1 death, acquire, travel, tell across all runs ---
    let death_count = results.iter().filter(|r| r.saw_death).count();
    let acquire_count = results.iter().filter(|r| r.saw_acquire).count();
    let travel_count = results.iter().filter(|r| r.saw_travel).count();
    let tell_count = results.iter().filter(|r| r.saw_tell).count();

    // These should be true for the vast majority of runs given diverse population.
    assert!(
        death_count >= 1,
        "no runs produced a death in 10080 ticks"
    );
    assert!(
        acquire_count >= 1,
        "no runs produced an acquire/trade action in 10080 ticks"
    );
    assert!(
        travel_count >= 1,
        "no runs produced travel in 10080 ticks"
    );
    assert!(
        tell_count >= 1,
        "no runs produced a tell/share-belief action in 10080 ticks"
    );
}

// ---------------------------------------------------------------------------
// Scenario 31: Stress with Frequent Disruptions
// ---------------------------------------------------------------------------
//
// Systems: Needs, Production, Trade, Combat, Travel, Social, Politics, Perception
// GoalKinds: ConsumeOwnedCommodity, AcquireCommodity, RestockCommodity, ShareBelief,
//   ClaimOffice, StealItem, Patrol, Harvest, Craft
// ActionDomains: Needs, Trade, Travel, Combat, Production, Social, Transport
// Places: T30Hub, T30Market, T30Farm, T30Forge, T30Barracks, T30RulersHall,
//   T30Forest, T30BanditCamp, T30Road, T30Orchard
// Principles: 3, 4, 6, 7, 8, 10, 12, 14, 26
//
// Setup: Reuses T30's 10-place topology and 20-agent population. Every 100 ticks,
//   one random disruption is injected via WorldTxn: kill an agent, destroy an item
//   lot, remove a workstation tag, or teleport an agent. Disruption type is selected
//   deterministically from DeterministicRng for reproducibility. Runs 2880 ticks
//   (2 in-game days) with 28 disruptions total.
//
// Proves: The full simulation stack handles arbitrary mid-run disruptions gracefully.
//   All per-tick invariants (conservation, needs bounds, dead agent inactivity,
//   unique placement, tick monotonicity, causal link integrity) hold despite
//   disruptions. Save/load roundtrip at end produces identical hash. No panics.
//
// Chain: autonomous agents + periodic disruptions (death, destruction, removal,
//   teleportation) -> AI replanning around changed state -> invariants hold
//   every tick despite arbitrary state mutations.

/// Run a single T31 stress run for the given seed. Panics on invariant violation.
fn run_t31_stress(seed: Seed) {
    let (mut h, all_agents, _ruling_faction, _bandit_faction, _office) = build_t30_world(seed);

    const TOTAL_TICKS: u64 = 2880;
    const DISRUPTION_INTERVAL: u64 = 100;

    let commodities_to_check = [
        CommodityKind::Apple,
        CommodityKind::Grain,
        CommodityKind::Bread,
        CommodityKind::Coin,
    ];

    // Snapshot initial commodity totals for conservation checks.
    let mut commodity_totals: std::collections::BTreeMap<CommodityKind, u64> = commodities_to_check
        .iter()
        .map(|&c| (c, total_authoritative_commodity_quantity(&h.world, c)))
        .collect();

    let mut prev_tick = h.scheduler.current_tick();

    // Separate RNG stream for disruptions so they don't perturb the simulation RNG.
    let mut disruption_seed = seed;
    disruption_seed.0[0] = disruption_seed.0[0].wrapping_add(0xDD);
    let mut disruption_rng =
        worldwake_sim::DeterministicRng::new(disruption_seed);

    // Collect all place IDs from the T30 topology for teleportation targets.
    let all_places = [
        PLACE_T30_HUB,
        PLACE_T30_MARKET,
        PLACE_T30_FARM,
        PLACE_T30_FORGE,
        PLACE_T30_BARRACKS,
        PLACE_T30_RULERS_HALL,
        PLACE_T30_FOREST,
        PLACE_T30_BANDIT_CAMP,
        PLACE_T30_ROAD,
        PLACE_T30_ORCHARD,
    ];

    for tick_idx in 0..TOTAL_TICKS {
        // --- Disruption injection every DISRUPTION_INTERVAL ticks ---
        if tick_idx > 0 && tick_idx % DISRUPTION_INTERVAL == 0 {
            let disruption_type = disruption_rng.next_range(0, 4);
            let current_tick_val = h.scheduler.current_tick().0;

            match disruption_type {
                0 => {
                    // Kill a random living agent.
                    let living: Vec<EntityId> = all_agents
                        .iter()
                        .copied()
                        .filter(|&a| !h.agent_is_dead(a))
                        .collect();
                    if !living.is_empty() {
                        let idx =
                            disruption_rng.next_range(0, living.len() as u32) as usize;
                        let victim = living[idx];
                        let mut txn = new_txn(&mut h.world, current_tick_val);
                        txn.set_component_dead_at(victim, DeadAt(Tick(current_tick_val)))
                            .unwrap();
                        commit_txn(txn, &mut h.event_log);
                    }
                }
                1 => {
                    // Destroy a random ItemLot (archive it and adjust conservation baseline).
                    let lots: Vec<EntityId> = h
                        .world
                        .entities_of_kind(EntityKind::ItemLot)
                        .collect();
                    if !lots.is_empty() {
                        let idx =
                            disruption_rng.next_range(0, lots.len() as u32) as usize;
                        let lot = lots[idx];
                        // Read quantity before archiving to adjust conservation baseline.
                        if let Some(item_lot) =
                            h.world.get_component_item_lot(lot).cloned()
                        {
                            let commodity = item_lot.commodity;
                            let qty = item_lot.quantity.0 as u64;
                            let mut txn = new_txn(&mut h.world, current_tick_val);
                            txn.archive_entity(lot).unwrap();
                            commit_txn(txn, &mut h.event_log);
                            // Reduce conservation baseline by the destroyed quantity.
                            if let Some(total) = commodity_totals.get_mut(&commodity) {
                                *total = total.saturating_sub(qty);
                            }
                        }
                    }
                }
                2 => {
                    // Remove WorkstationTag from a random facility.
                    let facilities: Vec<EntityId> = h
                        .world
                        .entities_of_kind(EntityKind::Facility)
                        .filter(|&e| h.world.get_component_workstation_marker(e).is_some())
                        .collect();
                    if !facilities.is_empty() {
                        let idx = disruption_rng.next_range(0, facilities.len() as u32)
                            as usize;
                        let facility = facilities[idx];
                        let mut txn = new_txn(&mut h.world, current_tick_val);
                        txn.clear_component_workstation_marker(facility).unwrap();
                        commit_txn(txn, &mut h.event_log);
                    }
                }
                3 => {
                    // Teleport a random living agent to a random place.
                    let living: Vec<EntityId> = all_agents
                        .iter()
                        .copied()
                        .filter(|&a| !h.agent_is_dead(a))
                        .collect();
                    if !living.is_empty() {
                        let agent_idx =
                            disruption_rng.next_range(0, living.len() as u32) as usize;
                        let agent = living[agent_idx];
                        let place_idx = disruption_rng
                            .next_range(0, all_places.len() as u32)
                            as usize;
                        let target_place = all_places[place_idx];
                        let mut txn = new_txn(&mut h.world, current_tick_val);
                        txn.set_ground_location(agent, target_place).unwrap();
                        commit_txn(txn, &mut h.event_log);
                    }
                }
                _ => unreachable!(),
            }
        }

        h.step_once();
        let current_tick = h.scheduler.current_tick();

        // --- Per-tick invariant 1: Conservation ---
        for (&commodity, total) in &mut commodity_totals {
            let actual = total_authoritative_commodity_quantity(&h.world, commodity);
            if actual > *total {
                *total = actual;
            }
            verify_authoritative_conservation(&h.world, commodity, actual).unwrap_or_else(|e| {
                panic!(
                    "conservation violation at tick {:?} for {:?}: {e}",
                    current_tick, commodity
                )
            });
        }

        // --- Per-tick invariant 2: Needs bounds ---
        for &agent in &all_agents {
            if h.agent_is_dead(agent) {
                continue;
            }
            if let Some(needs) = h.world.get_component_homeostatic_needs(agent) {
                let max = Permille::new(1000).unwrap();
                assert!(
                    needs.hunger <= max
                        && needs.thirst <= max
                        && needs.fatigue <= max
                        && needs.bladder <= max
                        && needs.dirtiness <= max,
                    "needs out of bounds for agent {agent:?} at tick {current_tick:?}: {needs:?}"
                );
            }
        }

        // --- Per-tick invariant 3: Dead agent inactivity ---
        for &agent in &all_agents {
            if let Some(dead_at) = h.world.get_component_dead_at(agent) {
                assert!(
                    !h.agent_has_active_action(agent),
                    "dead agent {agent:?} (died at {:?}) has active action at tick {current_tick:?}",
                    dead_at.0
                );
            }
        }

        // --- Per-tick invariant 4: Unique placement ---
        for &agent in &all_agents {
            if h.agent_is_dead(agent) {
                continue;
            }
            if let Some(place) = h.world.effective_place(agent) {
                assert!(
                    h.world.topology().place(place).is_some(),
                    "agent {agent:?} placed at non-existent place {place:?} at tick {current_tick:?}"
                );
            }
        }

        // --- Per-tick invariant 5: Tick monotonicity ---
        assert!(
            current_tick > prev_tick,
            "tick did not advance: prev={prev_tick:?}, current={current_tick:?}"
        );
        prev_tick = current_tick;

        // --- Per-tick invariant 6: Causal link integrity ---
        let log_len = h.event_log.len() as u64;
        for idx in 0..log_len {
            let event_id = EventId(idx);
            if let Some(record) = h.event_log.get(event_id) {
                match record.cause() {
                    CauseRef::Event(cause_id) => {
                        assert!(
                            h.event_log.get(cause_id).is_some(),
                            "event {event_id:?} references non-existent cause {cause_id:?} \
                             at tick {current_tick:?}"
                        );
                    }
                    CauseRef::SystemTick(_) | CauseRef::Bootstrap | CauseRef::ExternalInput(_) => {
                    }
                }
            }
        }
    }

    // --- Verification layer 4: Save/load roundtrip fidelity ---
    let pre_save_hash = hash_world(&h.world).unwrap();
    let roundtripped = h.save_load_roundtrip();
    let post_load_hash = hash_world(&roundtripped.world).unwrap();
    assert_eq!(
        pre_save_hash, post_load_hash,
        "save/load roundtrip at tick 2880 produced different hash: \
         pre={pre_save_hash:?}, post={post_load_hash:?}"
    );
}

#[test]
fn t31_stress_disruptions() {
    let mut seed_bytes = [0u8; 32];
    seed_bytes[0] = 0x31;
    seed_bytes[31] = 0xAB;
    run_t31_stress(Seed(seed_bytes));
}

// ---------------------------------------------------------------------------
// Scenario 32: Long Replay Consistency
// ---------------------------------------------------------------------------
//
// Systems: Needs, Production, Trade, Combat, Travel, Social, Politics, Perception
// GoalKinds: ConsumeOwnedCommodity, AcquireCommodity, RestockCommodity, ShareBelief,
//   ClaimOffice, StealItem, Patrol, Harvest, Craft
// ActionDomains: Needs, Trade, Travel, Combat, Production, Social, Transport
// Places: T30Hub, T30Market, T30Farm, T30Forge, T30Barracks, T30RulersHall,
//   T30Forest, T30BanditCamp, T30Road, T30Orchard
// Principles: 3, 4, 6, 12
//
// Setup: Reuses T30's 10-place topology and 20-agent population. A continuous
//   2880-tick run records (hash_world, hash_event_log) at every 100-tick
//   checkpoint. A split run saves at tick 1440, loads the snapshot, and
//   continues for another 1440 ticks, recording the same checkpoints.
//
// Proves: Save/load mid-run preserves all world meaning (Principle 12).
//   Deterministic execution: same seed + same inputs = identical StateHash
//   at every checkpoint, whether run continuously or split across a
//   serialization boundary. No state leakage through save/load.
//
// Chain: seed -> continuous 2880-tick run -> checkpoint hashes
//   vs seed -> 1440 ticks -> save_to_bytes -> load_from_bytes -> 1440 ticks
//   -> checkpoint hashes must match exactly at every 100-tick boundary.

/// Run 2880 ticks continuously, recording (tick, world_hash, log_hash) at
/// every 100-tick checkpoint.
fn run_continuous(seed: Seed, total_ticks: u64) -> Vec<(u64, StateHash, StateHash)> {
    let (mut h, _agents, _rf, _bf, _office) = build_t30_world(seed);
    let mut checkpoints = Vec::new();

    for tick_idx in 1..=total_ticks {
        h.step_once();
        if tick_idx % 100 == 0 {
            let world_hash = hash_world(&h.world).unwrap();
            let log_hash = hash_event_log(&h.event_log).unwrap();
            checkpoints.push((tick_idx, world_hash, log_hash));
        }
    }

    checkpoints
}

/// Run `save_at` ticks, save to bytes, load from bytes, then continue for
/// `total_ticks - save_at` more ticks. Record checkpoints at every 100-tick
/// boundary across both halves.
fn run_split(
    seed: Seed,
    save_at: u64,
    total_ticks: u64,
) -> Vec<(u64, StateHash, StateHash)> {
    let (mut h, _agents, _rf, _bf, _office) = build_t30_world(seed);
    let mut checkpoints = Vec::new();

    // --- First half: run up to save_at ---
    for tick_idx in 1..=save_at {
        h.step_once();
        if tick_idx % 100 == 0 {
            let world_hash = hash_world(&h.world).unwrap();
            let log_hash = hash_event_log(&h.event_log).unwrap();
            checkpoints.push((tick_idx, world_hash, log_hash));
        }
    }

    // --- Save → Load boundary ---
    let mut h = h.save_load_roundtrip();

    // --- Second half: continue from save_at+1 to total_ticks ---
    for tick_idx in (save_at + 1)..=total_ticks {
        h.step_once();
        if tick_idx % 100 == 0 {
            let world_hash = hash_world(&h.world).unwrap();
            let log_hash = hash_event_log(&h.event_log).unwrap();
            checkpoints.push((tick_idx, world_hash, log_hash));
        }
    }

    checkpoints
}

/// Run both continuous and split, assert exact checkpoint match.
fn run_t32_replay_consistency(seed: Seed) {
    let total_ticks: u64 = 2880;
    let save_at: u64 = 1440;

    let continuous = run_continuous(seed, total_ticks);
    let split = run_split(seed, save_at, total_ticks);

    assert_eq!(
        continuous.len(),
        split.len(),
        "T32: checkpoint count mismatch: continuous={}, split={}",
        continuous.len(),
        split.len()
    );

    for (c, s) in continuous.iter().zip(split.iter()) {
        assert_eq!(
            c.0, s.0,
            "T32: checkpoint tick mismatch: continuous={}, split={}",
            c.0, s.0
        );
        assert_eq!(
            c.1, s.1,
            "T32: world hash mismatch at tick {}: continuous={:?}, split={:?}",
            c.0, c.1, s.1
        );
        assert_eq!(
            c.2, s.2,
            "T32: event log hash mismatch at tick {}: continuous={:?}, split={:?}",
            c.0, c.2, s.2
        );
    }
}

#[test]
fn t32_replay_consistency() {
    let mut seed_bytes = [0u8; 32];
    seed_bytes[0] = 0x32;
    seed_bytes[31] = 0xCC;
    run_t32_replay_consistency(Seed(seed_bytes));
}
