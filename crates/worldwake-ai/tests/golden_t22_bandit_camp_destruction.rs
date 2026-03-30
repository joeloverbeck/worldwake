//! Golden T22 coverage for the bandit-camp destruction and diaspora chain.

mod golden_harness;

use golden_harness::*;
use std::collections::BTreeSet;
use worldwake_ai::{
    BanditCandidateOmissionReason, CommodityPurpose, DecisionOutcome, GoalTraceStatus,
    PlannerOpKind, SelectedPlanSource,
};
use worldwake_core::{
    build_believed_entity_state, hash_event_log, hash_world, verify_authoritative_conservation,
    ActionDomain, AgentData, BeliefConfidencePolicy, BelievedActivity, BanditCamp,
    BanditFactionPolicy, CombatProfile, CommodityKind, Container, ControlSource, EntityId,
    DemandMemory, DemandObservation, DemandObservationReason, EventLog, GoalKey, GoalKind,
    HomeostaticNeeds, InstitutionalBeliefRead, InstitutionalKnowledgeSource, KnownRecipes,
    MerchandiseProfile, MetabolismProfile, PerceptionProfile, PerceptionSource, Place, PlaceTag,
    Quantity, ResourceSource, Seed, TellProfile, Tick, Topology, TradeDispositionProfile,
    TravelEdge, TravelEdgeId, UtilityProfile, World, WorkstationTag,
};
use worldwake_sim::{ActionTraceKind, ControllerState, Scheduler, SystemManifest};
use worldwake_sim::{
    ActionPayload, ActionRequestMode, CombatActionPayload, RequestProvenance,
};

const HOME_MARKET: EntityId = entity(100);
const BANDIT_ROAD: EntityId = entity(101);
const CAMP_PLACE: EntityId = entity(102);
const SAFE_FARM: EntityId = entity(103);
const RALLY_GLEN: EntityId = entity(104);
const S47_BANDIT_CAMP: EntityId = entity(120);
const S47_ROAD_JUNCTION: EntityId = entity(121);
const S47_SAFE_VILLAGE: EntityId = entity(122);
const S48_MARKET: EntityId = entity(130);
const S48_DANGEROUS_ROAD: EntityId = entity(131);
const S48_BANDIT_CAMP: EntityId = entity(132);
const S48_SAFE_ROUTE: EntityId = entity(133);
const S48_REMOTE_FARM: EntityId = entity(134);
const S49_BANDIT_CAMP: EntityId = entity(140);
const S49_ROAD_JUNCTION: EntityId = entity(141);

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

fn build_t22_topology() -> Topology {
    let mut topology = Topology::new();
    topology
        .add_place(HOME_MARKET, place("Home Market", &[PlaceTag::Village, PlaceTag::Store]))
        .unwrap();
    topology
        .add_place(BANDIT_ROAD, place("Bandit Road", &[PlaceTag::Road, PlaceTag::Forest]))
        .unwrap();
    topology
        .add_place(CAMP_PLACE, place("Bandit Camp", &[PlaceTag::Camp, PlaceTag::Forest]))
        .unwrap();
    topology
        .add_place(SAFE_FARM, place("Safe Farm", &[PlaceTag::Farm, PlaceTag::Field]))
        .unwrap();
    topology
        .add_place(RALLY_GLEN, place("Rally Glen", &[PlaceTag::Forest, PlaceTag::Camp]))
        .unwrap();

    connect(&mut topology, 200, HOME_MARKET, BANDIT_ROAD, 1);
    connect(&mut topology, 210, BANDIT_ROAD, CAMP_PLACE, 1);
    connect(&mut topology, 220, HOME_MARKET, SAFE_FARM, 3);
    connect(&mut topology, 230, BANDIT_ROAD, RALLY_GLEN, 1);
    topology
}

fn default_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        memory_capacity: 64,
        memory_retention_ticks: 240,
        observation_fidelity: pm(875),
        confidence_policy: BeliefConfidencePolicy::default(),
        institutional_memory_capacity: 20,
        consultation_speed_factor: pm(500),
        contradiction_tolerance: pm(300),
    }
}

fn bandit_profile() -> CombatProfile {
    CombatProfile::new(
        pm(700),
        pm(450),
        pm(220),
        pm(250),
        pm(50),
        pm(15),
        pm(0),
        pm(150),
        pm(40),
        nz(2),
        nz(4),
    )
}

fn guard_profile() -> CombatProfile {
    CombatProfile::new(
        pm(1000),
        pm(700),
        pm(900),
        pm(800),
        pm(120),
        pm(40),
        pm(0),
        pm(320),
        pm(120),
        nz(1),
        nz(4),
    )
}

fn traveler_profile() -> CombatProfile {
    CombatProfile::new(
        pm(350),
        pm(250),
        pm(200),
        pm(180),
        pm(40),
        pm(12),
        pm(0),
        pm(90),
        pm(25),
        nz(2),
        nz(3),
    )
}

fn bandit_utility_profile() -> UtilityProfile {
    UtilityProfile {
        social_weight: pm(650),
        danger_weight: pm(900),
        courage: pm(150),
        enterprise_weight: pm(0),
        ..UtilityProfile::default()
    }
}

fn s47_bandit_utility_profile() -> UtilityProfile {
    UtilityProfile {
        social_weight: pm(0),
        danger_weight: pm(900),
        courage: pm(150),
        enterprise_weight: pm(0),
        ..UtilityProfile::default()
    }
}

fn merchant_utility_profile() -> UtilityProfile {
    UtilityProfile {
        social_weight: pm(0),
        danger_weight: pm(900),
        courage: pm(400),
        enterprise_weight: pm(900),
        ..UtilityProfile::default()
    }
}

fn s49_bandit_utility_profile() -> UtilityProfile {
    UtilityProfile {
        social_weight: pm(0),
        danger_weight: pm(900),
        courage: pm(200),
        enterprise_weight: pm(0),
        ..UtilityProfile::default()
    }
}

fn s49_bandit_profile() -> CombatProfile {
    CombatProfile::new(
        pm(800),
        pm(500),
        pm(220),
        pm(120),
        pm(20),
        pm(10),
        pm(0),
        pm(150),
        pm(40),
        nz(2),
        nz(4),
    )
}

fn s49_traveler_profile() -> CombatProfile {
    CombatProfile::new(
        pm(350),
        pm(250),
        pm(450),
        pm(180),
        pm(40),
        pm(10),
        pm(0),
        pm(180),
        pm(25),
        nz(2),
        nz(3),
    )
}

fn accepting_tell_profile() -> TellProfile {
    TellProfile {
        max_tell_candidates: 3,
        max_relay_chain_len: 3,
        acceptance_fidelity: pm(1000),
        ..TellProfile::default()
    }
}

fn enterprise_trade_disposition_profile() -> TradeDispositionProfile {
    TradeDispositionProfile {
        negotiation_round_ticks: nz(4),
        initial_offer_bias: pm(500),
        concession_rate: pm(100),
        demand_memory_retention_ticks: 240,
    }
}

fn set_control_source(
    h: &mut GoldenHarness,
    agent: EntityId,
    control_source: ControlSource,
    tick: u64,
) {
    let mut txn = new_txn(&mut h.world, tick);
    txn.set_component_agent_data(agent, AgentData { control_source })
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn build_harness_with_topology(seed: Seed, topology: Topology) -> GoldenHarness {
    let mut h = GoldenHarness::new(seed);
    h.world = World::new(topology).unwrap();
    h.event_log = EventLog::new();
    h.scheduler = Scheduler::new(SystemManifest::canonical());
    h.controller = ControllerState::new();
    h
}

fn build_custom_harness(seed: Seed) -> GoldenHarness {
    build_harness_with_topology(seed, build_t22_topology())
}

struct ScenarioIds {
    faction: EntityId,
    camp_supplies: EntityId,
    bandits: Vec<EntityId>,
    outsider: EntityId,
    guards: Vec<EntityId>,
    traveler_fresh: EntityId,
    traveler_late: EntityId,
}

struct S47Ids {
    bandits: Vec<EntityId>,
    traveler: EntityId,
}

struct S48Ids {
    bandits: Vec<EntityId>,
    witness: EntityId,
    merchant: EntityId,
    orchard_workstation: EntityId,
}

struct S49Ids {
    bandit: EntityId,
    travelers: [EntityId; 3],
}

fn build_s47_topology() -> Topology {
    let mut topology = Topology::new();
    topology
        .add_place(
            S47_BANDIT_CAMP,
            place("S47 Bandit Camp", &[PlaceTag::Camp, PlaceTag::Forest]),
        )
        .unwrap();
    topology
        .add_place(
            S47_ROAD_JUNCTION,
            place("S47 Road Junction", &[PlaceTag::Road, PlaceTag::Forest]),
        )
        .unwrap();
    topology
        .add_place(
            S47_SAFE_VILLAGE,
            place("S47 Safe Village", &[PlaceTag::Village, PlaceTag::Store]),
        )
        .unwrap();

    connect(&mut topology, 320, S47_BANDIT_CAMP, S47_ROAD_JUNCTION, 1);
    connect(&mut topology, 330, S47_ROAD_JUNCTION, S47_SAFE_VILLAGE, 2);
    topology
}

fn build_s48_topology() -> Topology {
    let mut topology = Topology::new();
    topology
        .add_place(
            S48_MARKET,
            place("S48 Market", &[PlaceTag::Village, PlaceTag::Store]),
        )
        .unwrap();
    topology
        .add_place(
            S48_DANGEROUS_ROAD,
            place("S48 Dangerous Road", &[PlaceTag::Road, PlaceTag::Forest]),
        )
        .unwrap();
    topology
        .add_place(
            S48_BANDIT_CAMP,
            place("S48 Bandit Camp", &[PlaceTag::Camp, PlaceTag::Forest]),
        )
        .unwrap();
    topology
        .add_place(
            S48_SAFE_ROUTE,
            place("S48 Safe Route", &[PlaceTag::Road, PlaceTag::Field]),
        )
        .unwrap();
    topology
        .add_place(
            S48_REMOTE_FARM,
            place("S48 Remote Farm", &[PlaceTag::Farm, PlaceTag::Field]),
        )
        .unwrap();

    connect(&mut topology, 340, S48_MARKET, S48_DANGEROUS_ROAD, 2);
    connect(&mut topology, 350, S48_DANGEROUS_ROAD, S48_BANDIT_CAMP, 1);
    connect(&mut topology, 360, S48_DANGEROUS_ROAD, S48_REMOTE_FARM, 1);
    connect(&mut topology, 370, S48_MARKET, S48_SAFE_ROUTE, 2);
    connect(&mut topology, 380, S48_SAFE_ROUTE, S48_REMOTE_FARM, 2);
    topology
}

fn build_s49_topology() -> Topology {
    let mut topology = Topology::new();
    topology
        .add_place(
            S49_BANDIT_CAMP,
            place("S49 Bandit Camp", &[PlaceTag::Camp, PlaceTag::Forest]),
        )
        .unwrap();
    topology
        .add_place(
            S49_ROAD_JUNCTION,
            place("S49 Road Junction", &[PlaceTag::Road, PlaceTag::Forest]),
        )
        .unwrap();

    connect(&mut topology, 390, S49_BANDIT_CAMP, S49_ROAD_JUNCTION, 1);
    topology
}

fn seed_danger_belief(h: &mut GoldenHarness, traveler: EntityId, subject: EntityId, place: EntityId) {
    let mut belief =
        build_believed_entity_state(&h.world, subject, Tick(0), PerceptionSource::DirectObservation)
            .expect("should build belief for live combatant");
    belief.last_known_place = Some(place);
    belief.believed_activity = Some(BelievedActivity {
        action_domain: ActionDomain::Combat,
        target: None,
        observed_tick: Tick(0),
    });
    seed_belief(&mut h.world, &mut h.event_log, traveler, subject, belief);
}

fn seed_hungry_traveler(
    h: &mut GoldenHarness,
    name: &str,
    control: ControlSource,
) -> EntityId {
    let traveler = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        name,
        HOME_MARKET,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile {
            hunger_rate: pm(0),
            thirst_rate: pm(0),
            fatigue_rate: pm(0),
            bladder_rate: pm(0),
            dirtiness_rate: pm(0),
            ..MetabolismProfile::default()
        },
        UtilityProfile::default(),
        KnownRecipes::new(),
    );
    set_control_source(h, traveler, control, 0);
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        traveler,
        default_perception_profile(),
    );
    traveler
}

fn seed_scenario(h: &mut GoldenHarness) -> ScenarioIds {
    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        CAMP_PLACE,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(12),
            max_quantity: Quantity(12),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );
    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        SAFE_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(12),
            max_quantity: Quantity(12),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );

    let traveler_fresh = seed_hungry_traveler(h, "Fresh Traveler", ControlSource::Ai);
    let traveler_late = seed_hungry_traveler(h, "Late Traveler", ControlSource::None);

        let (
            faction,
            bandits,
            outsider,
            guards,
            camp_supplies,
            camp_apples,
            safe_apples,
        ) = {
            let mut txn = new_txn(&mut h.world, 0);
            let faction = txn.create_faction("Forest Bandits").unwrap();

        let mut bandits = Vec::new();
        for name in ["Rook", "Mora", "Tarn", "Sable"] {
            let bandit = txn.create_agent(name, ControlSource::Ai).unwrap();
            txn.add_member(bandit, faction).unwrap();
            txn.set_ground_location(bandit, CAMP_PLACE).unwrap();
            txn.set_component_perception_profile(bandit, default_perception_profile())
                .unwrap();
            txn.set_component_combat_profile(bandit, bandit_profile()).unwrap();
            txn.set_component_utility_profile(bandit, bandit_utility_profile())
                .unwrap();
            bandits.push(bandit);
        }
        txn.set_component_wound_list(bandits[0], stable_wound_list(350))
            .unwrap();
        txn.set_component_wound_list(bandits[1], stable_wound_list(350))
            .unwrap();

        let outsider = txn.create_agent("Stray", ControlSource::Ai).unwrap();
        txn.add_member(outsider, faction).unwrap();
        txn.set_ground_location(outsider, HOME_MARKET).unwrap();
        txn.set_component_perception_profile(outsider, default_perception_profile())
            .unwrap();
        txn.set_component_combat_profile(outsider, bandit_profile()).unwrap();
        txn.set_component_utility_profile(outsider, bandit_utility_profile())
            .unwrap();

        txn.set_component_bandit_faction_policy(
            faction,
            BanditFactionPolicy {
                min_regroup_count: 2,
                establishment_duration_ticks: nz(2),
                abandonment_grace_ticks: nz(2),
                flee_wound_threshold: pm(300),
                rally_place: Some(RALLY_GLEN),
            },
        )
        .unwrap();

        let camp_supplies = txn
            .create_container(Container {
                capacity: worldwake_core::LoadUnits(20),
                allowed_commodities: None,
                allows_unique_items: false,
                allows_nested_containers: false,
            })
            .unwrap();
        txn.set_ground_location(camp_supplies, CAMP_PLACE).unwrap();
        txn.set_owner(camp_supplies, faction).unwrap();
        txn.set_component_bandit_camp(
            CAMP_PLACE,
            BanditCamp {
                faction,
                supplies: camp_supplies,
                empty_since_tick: None,
            },
        )
        .unwrap();
        let bread = txn.create_item_lot(CommodityKind::Bread, Quantity(6)).unwrap();
        txn.set_ground_location(bread, CAMP_PLACE).unwrap();
        txn.set_owner(bread, faction).unwrap();
        txn.put_into_container(bread, camp_supplies).unwrap();
        let camp_apples = txn.create_item_lot(CommodityKind::Apple, Quantity(4)).unwrap();
        txn.set_ground_location(camp_apples, CAMP_PLACE).unwrap();
        let safe_apples = txn.create_item_lot(CommodityKind::Apple, Quantity(4)).unwrap();
        txn.set_ground_location(safe_apples, SAFE_FARM).unwrap();
        let rally_bread = txn.create_item_lot(CommodityKind::Bread, Quantity(2)).unwrap();
        txn.set_ground_location(rally_bread, RALLY_GLEN).unwrap();
        txn.set_owner(rally_bread, faction).unwrap();

        let mut guards = Vec::new();
        for name in ["Marshal", "Pike"] {
            let guard = txn.create_agent(name, ControlSource::None).unwrap();
            txn.set_ground_location(guard, CAMP_PLACE).unwrap();
            txn.set_component_perception_profile(guard, default_perception_profile())
                .unwrap();
            txn.set_component_combat_profile(guard, guard_profile()).unwrap();
            txn.set_component_utility_profile(
                guard,
                UtilityProfile {
                    danger_weight: pm(900),
                    social_weight: pm(0),
                    enterprise_weight: pm(0),
                    courage: pm(900),
                    ..UtilityProfile::default()
                },
            )
            .unwrap();
            guards.push(guard);
        }

        txn.commit(&mut h.event_log);
        (
            faction,
            bandits,
            outsider,
            guards,
            camp_supplies,
            camp_apples,
            safe_apples,
        )
    };

    for traveler in [traveler_fresh, traveler_late] {
        for lot in [camp_apples, safe_apples] {
            let belief = build_believed_entity_state(
                &h.world,
                lot,
                Tick(0),
                PerceptionSource::DirectObservation,
            )
            .expect("traveler food-lot belief should build");
            seed_belief(&mut h.world, &mut h.event_log, traveler, lot, belief);
        }
        seed_danger_belief(h, traveler, bandits[0], BANDIT_ROAD);
        seed_danger_belief(h, traveler, bandits[1], CAMP_PLACE);
    }

    ScenarioIds {
        faction,
        camp_supplies,
        bandits,
        outsider,
        guards,
        traveler_fresh,
        traveler_late,
    }
}

fn seed_s47_scenario(h: &mut GoldenHarness) -> S47Ids {
    let mut bandits = Vec::new();
    for name in ["S47 Rook", "S47 Mora", "S47 Tarn"] {
        let bandit = seed_agent(
            &mut h.world,
            &mut h.event_log,
            name,
            S47_BANDIT_CAMP,
            HomeostaticNeeds::new(pm(400), pm(0), pm(0), pm(0), pm(0)),
            MetabolismProfile {
                hunger_rate: pm(20),
                thirst_rate: pm(0),
                fatigue_rate: pm(0),
                bladder_rate: pm(0),
                dirtiness_rate: pm(0),
                ..MetabolismProfile::default()
            },
            s47_bandit_utility_profile(),
        );
        set_agent_perception_profile(
            &mut h.world,
            &mut h.event_log,
            bandit,
            default_perception_profile(),
        );
        bandits.push(bandit);
    }

    let traveler = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "S47 Traveler",
        S47_ROAD_JUNCTION,
        HomeostaticNeeds::new(pm(200), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_control_source(h, traveler, ControlSource::Human, 0);
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        traveler,
        default_perception_profile(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        traveler,
        S47_ROAD_JUNCTION,
        CommodityKind::Apple,
        Quantity(4),
    );

    let mut txn = new_txn(&mut h.world, 0);
    let faction = txn.create_faction("S47 Forest Bandits").unwrap();
    for bandit in &bandits {
        txn.add_member(*bandit, faction).unwrap();
        txn.set_component_combat_profile(*bandit, bandit_profile()).unwrap();
    }
    txn.set_component_combat_profile(traveler, traveler_profile())
        .unwrap();
    txn.set_component_bandit_faction_policy(
        faction,
        BanditFactionPolicy {
            min_regroup_count: 2,
            establishment_duration_ticks: nz(2),
            abandonment_grace_ticks: nz(2),
            flee_wound_threshold: pm(300),
            rally_place: None,
        },
    )
    .unwrap();
    let camp_supplies = txn
        .create_container(Container {
            capacity: worldwake_core::LoadUnits(20),
            allowed_commodities: None,
            allows_unique_items: false,
            allows_nested_containers: false,
        })
        .unwrap();
    txn.set_ground_location(camp_supplies, S47_BANDIT_CAMP).unwrap();
    txn.set_owner(camp_supplies, faction).unwrap();
    txn.set_component_bandit_camp(
        S47_BANDIT_CAMP,
        BanditCamp {
            faction,
            supplies: camp_supplies,
            empty_since_tick: None,
        },
    )
    .unwrap();
    txn.commit(&mut h.event_log);

    for bandit in &bandits {
        for other_bandit in &bandits {
            if bandit == other_bandit {
                continue;
            }
            seed_faction_membership_belief(
                &mut h.world,
                &mut h.event_log,
                *bandit,
                faction,
                *other_bandit,
                true,
                Tick(0),
                InstitutionalKnowledgeSource::WitnessedEvent,
                Some(S47_BANDIT_CAMP),
            );
        }
        seed_actor_local_beliefs(
            &mut h.world,
            &mut h.event_log,
            *bandit,
            Tick(0),
            PerceptionSource::DirectObservation,
        );
    }

    S47Ids { bandits, traveler }
}

fn seed_s48_scenario(h: &mut GoldenHarness) -> S48Ids {
    let orchard_workstation = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        S48_REMOTE_FARM,
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

    let mut bandits = Vec::new();
    for name in ["S48 Rook", "S48 Mora"] {
        let bandit = seed_agent(
            &mut h.world,
            &mut h.event_log,
            name,
            S48_DANGEROUS_ROAD,
            HomeostaticNeeds::new(pm(400), pm(0), pm(0), pm(0), pm(0)),
            MetabolismProfile {
                hunger_rate: pm(20),
                thirst_rate: pm(0),
                fatigue_rate: pm(0),
                bladder_rate: pm(0),
                dirtiness_rate: pm(0),
                ..MetabolismProfile::default()
            },
            bandit_utility_profile(),
        );
        set_agent_perception_profile(
            &mut h.world,
            &mut h.event_log,
            bandit,
            default_perception_profile(),
        );
        bandits.push(bandit);
    }

    let witness = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "S48 Witness",
        S48_DANGEROUS_ROAD,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            social_weight: pm(900),
            ..UtilityProfile::default()
        },
    );
    set_control_source(h, witness, ControlSource::Human, 0);
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        witness,
        default_perception_profile(),
    );
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        witness,
        accepting_tell_profile(),
    );

    let traveler = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "S48 Traveler",
        S48_DANGEROUS_ROAD,
        HomeostaticNeeds::new(pm(200), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_control_source(h, traveler, ControlSource::Human, 0);
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        traveler,
        default_perception_profile(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        traveler,
        S48_DANGEROUS_ROAD,
        CommodityKind::Apple,
        Quantity(4),
    );

    let merchant = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "S48 Merchant",
        S48_MARKET,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        merchant_utility_profile(),
        KnownRecipes::with([worldwake_core::RecipeId(0)]),
    );
    set_control_source(h, merchant, ControlSource::None, 0);
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        merchant,
        default_perception_profile(),
    );
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        merchant,
        accepting_tell_profile(),
    );

    let mut txn = new_txn(&mut h.world, 0);
    let faction = txn.create_faction("S48 Forest Bandits").unwrap();
    for bandit in &bandits {
        txn.add_member(*bandit, faction).unwrap();
        txn.set_component_combat_profile(*bandit, bandit_profile()).unwrap();
    }
    txn.set_component_combat_profile(traveler, traveler_profile())
        .unwrap();
    txn.set_component_bandit_faction_policy(
        faction,
        BanditFactionPolicy {
            min_regroup_count: 2,
            establishment_duration_ticks: nz(2),
            abandonment_grace_ticks: nz(2),
            flee_wound_threshold: pm(300),
            rally_place: None,
        },
    )
    .unwrap();
    let camp_supplies = txn
        .create_container(Container {
            capacity: worldwake_core::LoadUnits(20),
            allowed_commodities: None,
            allows_unique_items: false,
            allows_nested_containers: false,
        })
        .unwrap();
    txn.set_ground_location(camp_supplies, S48_BANDIT_CAMP).unwrap();
    txn.set_owner(camp_supplies, faction).unwrap();
    txn.set_component_bandit_camp(
        S48_BANDIT_CAMP,
        BanditCamp {
            faction,
            supplies: camp_supplies,
            empty_since_tick: None,
        },
    )
    .unwrap();
    txn.set_component_merchandise_profile(
        merchant,
        MerchandiseProfile {
            sale_kinds: BTreeSet::from([CommodityKind::Apple]),
            home_market: Some(S48_MARKET),
        },
    )
    .unwrap();
    txn.set_component_trade_disposition_profile(merchant, enterprise_trade_disposition_profile())
        .unwrap();
    txn.set_component_demand_memory(
        merchant,
        DemandMemory {
            observations: vec![DemandObservation {
                commodity: CommodityKind::Apple,
                quantity: Quantity(2),
                place: S48_MARKET,
                tick: Tick(0),
                counterparty: None,
                reason: DemandObservationReason::WantedToBuyButSellerOutOfStock,
            }],
        },
    )
    .unwrap();
    txn.commit(&mut h.event_log);

    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        &[orchard_workstation],
        Tick(0),
        PerceptionSource::Inference,
    );

    S48Ids {
        bandits,
        witness,
        merchant,
        orchard_workstation,
    }
}

fn seed_s49_scenario(h: &mut GoldenHarness) -> S49Ids {
    let bandit = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "S49 Rook",
        S49_BANDIT_CAMP,
        HomeostaticNeeds::new(pm(850), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile {
            hunger_rate: pm(20),
            thirst_rate: pm(0),
            fatigue_rate: pm(0),
            bladder_rate: pm(0),
            dirtiness_rate: pm(0),
            ..MetabolismProfile::default()
        },
        s49_bandit_utility_profile(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        bandit,
        default_perception_profile(),
    );

    let travelers = ["S49 Traveler One", "S49 Traveler Two", "S49 Traveler Three"].map(|name| {
        let traveler = seed_agent(
            &mut h.world,
            &mut h.event_log,
            name,
            S49_ROAD_JUNCTION,
            HomeostaticNeeds::new(pm(200), pm(0), pm(0), pm(0), pm(0)),
            MetabolismProfile::default(),
            UtilityProfile::default(),
        );
        set_control_source(h, traveler, ControlSource::Human, 0);
        set_agent_perception_profile(
            &mut h.world,
            &mut h.event_log,
            traveler,
            default_perception_profile(),
        );
        traveler
    });

    for traveler in travelers {
        give_commodity(
            &mut h.world,
            &mut h.event_log,
            traveler,
            S49_ROAD_JUNCTION,
            CommodityKind::Apple,
            Quantity(4),
        );
    }

    let mut txn = new_txn(&mut h.world, 0);
    let faction = txn.create_faction("S49 Forest Bandits").unwrap();
    txn.add_member(bandit, faction).unwrap();
    txn.set_component_combat_profile(bandit, s49_bandit_profile())
        .unwrap();
    txn.set_component_bandit_faction_policy(
        faction,
        BanditFactionPolicy {
            min_regroup_count: 1,
            establishment_duration_ticks: nz(2),
            abandonment_grace_ticks: nz(2),
            flee_wound_threshold: pm(300),
            rally_place: None,
        },
    )
    .unwrap();
    let camp_supplies = txn
        .create_container(Container {
            capacity: worldwake_core::LoadUnits(20),
            allowed_commodities: None,
            allows_unique_items: false,
            allows_nested_containers: false,
        })
        .unwrap();
    txn.set_ground_location(camp_supplies, S49_BANDIT_CAMP).unwrap();
    txn.set_owner(camp_supplies, faction).unwrap();
    txn.set_component_bandit_camp(
        S49_BANDIT_CAMP,
        BanditCamp {
            faction,
            supplies: camp_supplies,
            empty_since_tick: None,
        },
    )
    .unwrap();
    for traveler in travelers {
        txn.set_component_combat_profile(traveler, s49_traveler_profile())
            .unwrap();
    }
    txn.commit(&mut h.event_log);

    S49Ids { bandit, travelers }
}

fn request_travel_to_place(h: &mut GoldenHarness, traveler: EntityId, destination: EntityId) {
    let travel_def_id = h
        .defs
        .iter()
        .find(|def| def.name == "travel")
        .map(|def| def.id)
        .expect("full registries should include travel");
    let tick = h.scheduler.current_tick();
    let _ = h.scheduler.input_queue_mut().enqueue(
        tick,
        worldwake_sim::InputKind::RequestAction {
            actor: traveler,
            def_id: travel_def_id,
            targets: vec![destination],
            payload_override: None,
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );
}

fn request_travel_to_camp(h: &mut GoldenHarness, traveler: EntityId) {
    request_travel_to_place(h, traveler, S47_BANDIT_CAMP);
}

fn activate_attack(h: &mut GoldenHarness, ids: &ScenarioIds, tick: u64) {
    let attack_def_id = h
        .defs
        .iter()
        .find(|def| def.name == "attack")
        .map(|def| def.id)
        .expect("full registries should include attack");
    for guard in &ids.guards {
        set_control_source(h, *guard, ControlSource::Ai, tick);
    }
    for (index, guard) in ids.guards.iter().enumerate() {
        for bandit in &ids.bandits {
            add_hostility(&mut h.world, &mut h.event_log, *guard, *bandit);
            add_hostility(&mut h.world, &mut h.event_log, *bandit, *guard);
        }
        let target = ids.bandits[index % ids.bandits.len()];
        let _ = h.scheduler.input_queue_mut().enqueue(
            Tick(tick),
            worldwake_sim::InputKind::RequestAction {
                actor: *guard,
                def_id: attack_def_id,
                targets: vec![target],
                payload_override: Some(ActionPayload::Combat(CombatActionPayload {
                    target,
                    weapon: worldwake_core::CombatWeaponRef::Unarmed,
                })),
                mode: ActionRequestMode::BestEffort,
                provenance: RequestProvenance::External,
            },
        );
    }
}

fn latest_traveler_selected_travel_destination(h: &GoldenHarness, agent: EntityId) -> Option<EntityId> {
    h.driver.trace_sink()?.traces_for(agent).into_iter().rev().find_map(|trace| {
        let DecisionOutcome::Planning(planning) = &trace.outcome else {
            return None;
        };
        if planning.selection.selected_plan_source != Some(SelectedPlanSource::SearchSelection) {
            return None;
        }
        if !planning
            .selection
            .selected_goal_is(GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
            }))
        {
            return None;
        }
        planning
            .selection
            .selected_plan
            .as_ref()?
            .steps
            .iter()
            .rev()
            .find(|step| step.op_kind == PlannerOpKind::Travel)
            .and_then(|step| step.targets.first().copied())
    })
}

fn latest_restock_selected_travel_destination(
    h: &GoldenHarness,
    agent: EntityId,
    commodity: CommodityKind,
) -> Option<EntityId> {
    h.driver.trace_sink()?.traces_for(agent).into_iter().rev().find_map(|trace| {
        let DecisionOutcome::Planning(planning) = &trace.outcome else {
            return None;
        };
        if planning.selection.selected_plan_source != Some(SelectedPlanSource::SearchSelection) {
            return None;
        }
        if !planning
            .selection
            .selected_goal_is(GoalKey::from(GoalKind::RestockCommodity { commodity }))
        {
            return None;
        }
        planning
            .selection
            .selected_plan
            .as_ref()?
            .steps
            .iter()
            .find(|step| step.op_kind == PlannerOpKind::Travel)
            .and_then(|step| step.targets.first().copied())
    })
}

fn agent_knows_witnessed_conflict_at_place(
    h: &GoldenHarness,
    agent: EntityId,
    place: EntityId,
    subjects: &[EntityId],
) -> bool {
    h.world
        .get_component_agent_belief_store(agent)
        .is_some_and(|store| {
            store.social_observations.iter().any(|observation| {
                observation.place == place
                    && matches!(
                        observation.detail,
                        worldwake_core::SocialObservationDetail::WitnessedConflict { actor, .. }
                            if subjects.contains(&actor)
                    )
            })
        })
}

fn wound_total(h: &GoldenHarness, agent: EntityId) -> u16 {
    h.world
        .get_component_wound_list(agent)
        .map(|wounds| {
            wounds
                .wounds
                .iter()
                .map(|wound| wound.severity.value())
                .sum::<u16>()
        })
        .unwrap_or(0)
}

fn set_wound_total(h: &mut GoldenHarness, agent: EntityId, severity: u16, tick: u64) {
    let mut txn = new_txn(&mut h.world, tick);
    txn.set_component_wound_list(agent, stable_wound_list(severity))
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn latest_planning_trace_selected_goal(h: &GoldenHarness, agent: EntityId) -> Option<GoalKey> {
    h.driver
        .trace_sink()?
        .traces_for(agent)
        .into_iter()
        .rev()
        .find_map(|trace| match &trace.outcome {
            DecisionOutcome::Planning(planning) => planning.selection.selected_goal(),
            _ => None,
        })
}

fn run_t22_scenario(seed: Seed) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = build_custom_harness(seed);
    let ids = seed_scenario(&mut h);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    h.step_once();

    let fresh_destination = latest_traveler_selected_travel_destination(&h, ids.traveler_fresh);
    if fresh_destination.is_none() {
        h.driver
            .trace_sink()
            .expect("decision tracing should stay enabled")
            .dump_agent(ids.traveler_fresh, &h.defs);
    }
    let fresh_trace_summaries = h
        .driver
        .trace_sink()
        .expect("decision tracing should stay enabled")
        .traces_for(ids.traveler_fresh)
        .into_iter()
        .map(|trace| format!("{:?}: {}", trace.tick, trace.outcome.summary()))
        .collect::<Vec<_>>();
    assert_eq!(
        fresh_destination,
        Some(SAFE_FARM),
        "fresh danger beliefs should route the first traveler toward the safe farm; traces={fresh_trace_summaries:?}"
    );
    assert_eq!(
        h.world
            .get_component_agent_belief_store(ids.bandits[0])
            .expect("bandit should have a belief store")
            .believed_faction_rally_point(ids.faction),
        InstitutionalBeliefRead::Certain(Some(RALLY_GLEN)),
        "bandits co-located with the active camp should learn the rally doctrine before the attack"
    );
    assert_eq!(
        h.world
            .get_component_agent_belief_store(ids.outsider)
            .expect("outsider should have a belief store")
            .believed_faction_rally_point(ids.faction),
        InstitutionalBeliefRead::Unknown,
        "remote faction members should not learn rally doctrine without lawful camp observation"
    );

    activate_attack(&mut h, &ids, 1);

    let mut saw_bandit_departure = false;
    let mut saw_regroup_selection = false;
    let mut saw_regroup_travel = false;
    let mut saw_establish_goal = false;
    let mut old_camp_removed = false;
    let mut new_camp_established = false;
    let mut saw_establish_commit = false;

    for _ in 0..200 {
        h.step_once();
        saw_bandit_departure |= ids.bandits.iter().any(|bandit| {
            h.world.effective_place(*bandit).is_some_and(|place| place != CAMP_PLACE)
                || h.world.is_in_transit(*bandit)
        });
        saw_regroup_selection |= ids.bandits.iter().any(|bandit| {
            h.driver
                .trace_sink()
                .is_some_and(|sink| {
                    sink.traces_for(*bandit).into_iter().any(|trace| match &trace.outcome {
                        DecisionOutcome::Planning(planning) => planning
                            .selection
                            .selected_goal_is(GoalKey::from(GoalKind::RegroupWithFaction {
                                faction: ids.faction,
                            })),
                        _ => false,
                    })
                })
        });
        saw_regroup_travel |= ids.bandits.iter().any(|bandit| {
            h.action_trace_sink()
                .is_some_and(|sink| {
                    sink.events_for(*bandit).into_iter().any(|event| {
                        event.action_name == "travel"
                            && matches!(event.kind, ActionTraceKind::Committed { .. })
                    })
                })
        });
        saw_establish_goal |= ids.bandits.iter().any(|bandit| {
            h.driver.trace_sink().is_some_and(|sink| {
                sink.traces_for(*bandit).into_iter().any(|trace| match &trace.outcome {
                    DecisionOutcome::Planning(planning) => {
                        planning
                            .selection
                            .selected_goal_is(GoalKey::from(GoalKind::EstablishBanditCamp {
                                faction: ids.faction,
                            }))
                            || planning.candidates.generated.iter().any(|goal| {
                                goal.goal_key
                                    == GoalKey::from(GoalKind::EstablishBanditCamp {
                                        faction: ids.faction,
                                    })
                            })
                    }
                    _ => false,
                })
            })
        });
        old_camp_removed |= h.world.get_component_bandit_camp(CAMP_PLACE).is_none();
        new_camp_established |= h
            .world
            .get_component_bandit_camp(RALLY_GLEN)
            .is_some_and(|camp| camp.faction == ids.faction);
        saw_establish_commit |= ids.bandits.iter().any(|bandit| {
            h.action_trace_sink()
                .is_some_and(|sink| {
                    sink.events_for(*bandit).into_iter().any(|event| {
                        event.action_name == "establish_camp"
                            && matches!(event.kind, ActionTraceKind::Committed { .. })
                    })
                })
        });

        if saw_bandit_departure
            && saw_regroup_selection
            && saw_regroup_travel
            && saw_establish_goal
            && old_camp_removed
            && new_camp_established
            && saw_establish_commit
        {
            break;
        }
    }

    assert!(
        ids.bandits.iter().any(|bandit| {
            h.world.get_component_dead_at(*bandit).is_some()
                || h.world
                    .get_component_wound_list(*bandit)
                    .is_some_and(|wounds| !wounds.wounds.is_empty())
        }),
        "the external attack should leave at least one bandit dead or wounded"
    );
    assert!(
        saw_bandit_departure,
        "at least one surviving bandit should leave the original camp under attack pressure"
    );
    assert!(
        saw_regroup_selection,
        "a surviving bandit with rally doctrine should later select RegroupWithFaction after the camp starts breaking apart"
    );
    assert!(
        saw_regroup_travel,
        "regrouping should require ordinary travel rather than any teleport or direct camp transfer"
    );
    assert!(
        saw_establish_goal,
        "survivors at the rally should eventually generate or select EstablishBanditCamp once regrouping makes camp recreation lawful"
    );
    assert!(
        old_camp_removed,
        "the original camp should be removed through the abandonment path once no living faction members remain there"
    );
    assert!(
        new_camp_established,
        "surviving bandits should eventually establish a new camp at the rally place"
    );
    assert!(
        saw_establish_commit,
        "camp recreation should commit through the ordinary establish_camp action lifecycle"
    );
    assert_eq!(
        h.world.effective_place(ids.camp_supplies),
        Some(CAMP_PLACE),
        "abandonment should leave the old camp supplies behind as concrete aftermath"
    );
    assert!(
        h.driver
            .trace_sink()
            .expect("decision tracing should stay enabled")
            .traces_for(ids.outsider)
            .into_iter()
            .all(|trace| match &trace.outcome {
                DecisionOutcome::Planning(planning) => !planning
                    .selection
                    .selected_goal_is(GoalKey::from(GoalKind::RegroupWithFaction {
                        faction: ids.faction,
                    })),
                _ => true,
        }),
        "a same-faction outsider without rally doctrine should never select regroup"
    );
    assert!(
        h.driver
            .trace_sink()
            .expect("decision tracing should stay enabled")
            .traces_for(ids.outsider)
            .into_iter()
            .any(|trace| {
                trace.goal_status(&GoalKind::RegroupWithFaction { faction: ids.faction })
                    == GoalTraceStatus::OmittedBandit(
                        BanditCandidateOmissionReason::MissingRallyBelief,
                    )
            }),
        "a same-faction outsider without rally doctrine should expose the missing-rally omission reason in decision traces"
    );

    let late_activation_tick = h.scheduler.current_tick().0;
    set_control_source(&mut h, ids.traveler_late, ControlSource::Ai, late_activation_tick);

    let mut late_destination = None;
    let mut late_trace_summaries = Vec::new();
    for _ in 0..12 {
        h.step_once();
        late_destination = latest_traveler_selected_travel_destination(&h, ids.traveler_late);
        if late_destination.is_some() {
            break;
        }
    }
    if late_destination.is_none() {
        late_trace_summaries = h
            .driver
            .trace_sink()
            .expect("decision tracing should stay enabled")
            .traces_for(ids.traveler_late)
            .into_iter()
            .map(|trace| format!("{:?}: {}", trace.tick, trace.outcome.summary()))
            .collect::<Vec<_>>();
    }

    assert_eq!(
        late_destination,
        Some(CAMP_PLACE),
        "after the camp diaspora and normal belief aging, the later traveler should reselect the shorter abandoned-camp source; traces={late_trace_summaries:?}"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

fn run_s48_scenario(seed: Seed) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = build_harness_with_topology(seed, build_s48_topology());
    let ids = seed_s48_scenario(&mut h);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let mut saw_raid_commit = false;
    let mut witness_gained_conflict_observation = false;
    for _ in 0..80 {
        h.step_once();
        saw_raid_commit |= ids.bandits.iter().any(|bandit| {
            h.action_trace_sink()
                .expect("action tracing should stay enabled")
                .events_for(*bandit)
                .iter()
                .any(|event| {
                    event.action_name == "attack"
                        && matches!(event.kind, ActionTraceKind::Committed { .. })
                })
        });
        witness_gained_conflict_observation |= agent_knows_witnessed_conflict_at_place(
            &h,
            ids.witness,
            S48_DANGEROUS_ROAD,
            &ids.bandits,
        );
        if saw_raid_commit && witness_gained_conflict_observation {
            break;
        }
    }

    assert!(
        saw_raid_commit,
        "Scenario 48 should begin with a lawful bandit raid at the dangerous road"
    );
    assert!(
        witness_gained_conflict_observation,
        "the witness should acquire a witnessed-conflict observation at the dangerous road through perception"
    );
    assert!(
        !agent_knows_witnessed_conflict_at_place(&h, ids.merchant, S48_DANGEROUS_ROAD, &ids.bandits),
        "the merchant must remain ignorant of the dangerous road before the witness arrives"
    );

    request_travel_to_place(&mut h, ids.witness, S48_MARKET);
    let mut witness_traveled_to_market = false;
    for _ in 0..8 {
        h.step_once();
        witness_traveled_to_market |= h.world.effective_place(ids.witness) == Some(S48_MARKET);
        if witness_traveled_to_market {
            break;
        }
    }
    assert!(
        witness_traveled_to_market,
        "the witness should reach the market through the ordinary travel action"
    );
    assert!(
        h.action_trace_sink()
            .expect("action tracing should stay enabled")
            .events_for(ids.witness)
            .iter()
            .any(|event| {
                event.action_name == "travel"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            }),
        "witness travel should commit through the normal travel lifecycle"
    );

    let co_locate_tick = h.scheduler.current_tick();
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        ids.witness,
        co_locate_tick,
        PerceptionSource::DirectObservation,
    );
    set_control_source(&mut h, ids.witness, ControlSource::Ai, co_locate_tick.0);

    let mut merchant_learned_conflict_observation = false;
    let mut saw_tell_commit = false;
    for _ in 0..40 {
        h.step_once();
        merchant_learned_conflict_observation |= agent_knows_witnessed_conflict_at_place(
            &h,
            ids.merchant,
            S48_DANGEROUS_ROAD,
            &ids.bandits,
        );
        saw_tell_commit |= h
            .action_trace_sink()
            .expect("action tracing should stay enabled")
            .events_for(ids.witness)
            .iter()
            .any(|event| {
                event.action_name == "tell"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
                    && matches!(
                        event.detail,
                        Some(worldwake_sim::ActionTraceDetail::Tell { listener, .. })
                            if listener == ids.merchant
                    )
            });
        if merchant_learned_conflict_observation && saw_tell_commit {
            break;
        }
    }

    assert!(
        saw_tell_commit,
        "the witness should relay the danger belief to the merchant through the tell action"
    );
    assert!(
        merchant_learned_conflict_observation,
        "the merchant should learn the dangerous-road conflict observation from the witness"
    );

    let merchant_activation_tick = h.scheduler.current_tick();
    set_control_source(&mut h, ids.merchant, ControlSource::Ai, merchant_activation_tick.0);

    let mut selected_destination = None;
    let mut trace_summaries = Vec::new();
    for _ in 0..20 {
        h.step_once();
        selected_destination =
            latest_restock_selected_travel_destination(&h, ids.merchant, CommodityKind::Apple);
        if selected_destination.is_some() {
            break;
        }
    }
    if selected_destination.is_none() {
        trace_summaries = h
            .driver
            .trace_sink()
            .expect("decision tracing should stay enabled")
            .traces_for(ids.merchant)
            .into_iter()
            .map(|trace| format!("{:?}: {}", trace.tick, trace.outcome.summary()))
            .collect::<Vec<_>>();
    }

    assert_eq!(
        selected_destination,
        Some(S48_SAFE_ROUTE),
        "after learning the danger belief, the merchant should select the safe route for restock; traces={trace_summaries:?}"
    );
    assert!(
        h.world
            .get_component_agent_belief_store(ids.merchant)
            .is_some_and(|store| store.known_entities.contains_key(&ids.orchard_workstation)),
        "the merchant should retain explicit remote orchard knowledge so the route flip tests the danger belief rather than missing source evidence"
    );
    assert!(
        !h.agent_is_dead(ids.merchant),
        "the merchant must remain alive through the economic cascade scenario"
    );
    assert!(
        !h.agent_is_dead(ids.witness),
        "the witness must survive long enough to transport the danger belief"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

fn run_s47_scenario(seed: Seed) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = build_harness_with_topology(seed, build_s47_topology());
    let ids = seed_s47_scenario(&mut h);
    let initial_authoritative_apples = 4;
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let raid_goal = |target| GoalKey::from(GoalKind::RaidTarget { target });

    for _ in 0..1 {
        h.step_once();
        for bandit in &ids.bandits {
            let trace = h
                .driver
                .trace_sink()
                .expect("decision tracing should stay enabled")
                .traces_for(*bandit)
                .into_iter()
                .last()
                .expect("bandit should emit a planning trace each tick");
            if let DecisionOutcome::Planning(planning) = &trace.outcome {
                assert!(
                    !planning.selection.selected_goal_is(raid_goal(ids.traveler)),
                    "bandits should not select RaidTarget before the traveler is co-located"
                );
            }
        }
    }

    request_travel_to_camp(&mut h, ids.traveler);
    for _ in 0..4 {
        h.step_once();
        if h.world.effective_place(ids.traveler) == Some(S47_BANDIT_CAMP) {
            break;
        }
    }
    assert_eq!(
        h.world.effective_place(ids.traveler),
        Some(S47_BANDIT_CAMP),
        "traveler should reach the camp through the ordinary travel action"
    );
    assert!(
        h.action_trace_sink()
            .expect("action tracing should stay enabled")
            .events_for(ids.traveler)
            .iter()
            .any(|event| {
                event.action_name == "travel"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            }),
        "traveler arrival should commit through the normal travel lifecycle"
    );

    let mut saw_raid_candidate = false;
    let mut saw_raid_selection = false;
    let mut saw_attack_commit = false;
    let mut saw_loot_commit = false;
    let mut traveler_died = false;
    let mut corpse_emptied = false;

    for _ in 0..60 {
        h.step_once();
        traveler_died |= h.world.get_component_dead_at(ids.traveler).is_some();
        corpse_emptied |= traveler_died
            && h.agent_commodity_qty(ids.traveler, CommodityKind::Apple) == Quantity(0);

        for bandit in &ids.bandits {
            let traces = h
                .driver
                .trace_sink()
                .expect("decision tracing should stay enabled")
                .traces_for(*bandit);
            let trace = traces
                .into_iter()
                .last()
                .expect("bandit should continue producing planning traces");
            if let DecisionOutcome::Planning(planning) = &trace.outcome {
                saw_raid_candidate |= planning
                    .candidates
                    .generated
                    .iter()
                    .any(|goal| goal.goal_key == raid_goal(ids.traveler));
                saw_raid_selection |= planning.selection.selected_goal_is(raid_goal(ids.traveler));
            }
        }

        saw_attack_commit |= ids.bandits.iter().any(|bandit| {
            h.action_trace_sink()
                .expect("action tracing should stay enabled")
                .events_for(*bandit)
                .iter()
                .any(|event| {
                    event.action_name == "attack"
                        && matches!(event.kind, ActionTraceKind::Committed { .. })
                })
        });
        saw_loot_commit |= ids.bandits.iter().any(|bandit| {
            h.action_trace_sink()
                .expect("action tracing should stay enabled")
                .events_for(*bandit)
                .iter()
                .any(|event| {
                    event.action_name == "loot"
                        && matches!(event.kind, ActionTraceKind::Committed { .. })
                })
        });

        if saw_raid_candidate
            && saw_raid_selection
            && saw_attack_commit
            && saw_loot_commit
            && traveler_died
            && corpse_emptied
        {
            break;
        }
    }

    assert!(
        saw_raid_candidate,
        "co-located prey carrying visible food should generate RaidTarget"
    );
    assert!(
        saw_raid_selection,
        "at least one bandit should select RaidTarget after the traveler arrives"
    );
    assert!(
        saw_attack_commit,
        "the raid should resolve through the ordinary attack action"
    );
    assert!(traveler_died, "the traveler should die during the raid chain");
    assert!(
        saw_loot_commit,
        "the raid aftermath should resolve through the ordinary loot action"
    );
    assert!(
        corpse_emptied,
        "the traveler's apple inventory should be emptied by the raid aftermath"
    );
    verify_authoritative_conservation(&h.world, CommodityKind::Apple, initial_authoritative_apples)
        .unwrap();

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

fn run_s49_scenario(seed: Seed) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = build_harness_with_topology(seed, build_s49_topology());
    let ids = seed_s49_scenario(&mut h);
    let deterrence_threshold = 240u16;
    h.driver.enable_tracing();
    h.enable_action_tracing();
    for (phase_index, traveler) in ids.travelers.iter().take(2).enumerate() {
        if phase_index == 1 {
            let tick = h.scheduler.current_tick().0;
            set_wound_total(&mut h, ids.bandit, 220, tick);
            assert_eq!(
                wound_total(&h, ids.bandit),
                220,
                "phase 2 should begin below the deterrence threshold"
            );
        }

        request_travel_to_place(&mut h, *traveler, S49_BANDIT_CAMP);

        let mut traveler_arrived = false;
        let mut saw_raid_candidate = false;
        let mut saw_raid_selection = false;
        let mut saw_attack_commit = false;
        let mut retreat_requested = false;
        let phase_start_tick = h.scheduler.current_tick();

        for _ in 0..60 {
            h.step_once();
            traveler_arrived |= h.world.effective_place(*traveler) == Some(S49_BANDIT_CAMP);
            if !traveler_arrived {
                continue;
            }

            if let Some(sink) = h.driver.trace_sink() {
                if let Some(trace) = sink.traces_for(ids.bandit).into_iter().last() {
                    if let DecisionOutcome::Planning(planning) = &trace.outcome {
                        let raid_goal = GoalKey::from(GoalKind::RaidTarget { target: *traveler });
                        saw_raid_candidate |= planning
                            .candidates
                            .generated
                            .iter()
                            .any(|goal| goal.goal_key == raid_goal);
                        saw_raid_selection |= planning.selection.selected_goal() == Some(raid_goal);
                    }
                }
            }

            saw_attack_commit |= h
                .action_trace_sink()
                .expect("action tracing should stay enabled")
                .events_for(ids.bandit)
                .iter()
                .any(|event| {
                    event.action_name == "attack"
                        && matches!(event.kind, ActionTraceKind::Committed { .. })
                        && event.tick >= phase_start_tick
                });

            if saw_attack_commit && !retreat_requested {
                request_travel_to_place(&mut h, *traveler, S49_ROAD_JUNCTION);
                retreat_requested = true;
            }

            if retreat_requested && h.world.effective_place(*traveler) == Some(S49_ROAD_JUNCTION) {
                break;
            }
        }

        assert!(traveler_arrived, "traveler should reach the camp before the raid phase resolves");
        assert!(
            saw_raid_candidate,
            "bandit should generate RaidTarget before the deterrence threshold is crossed"
        );
        assert!(
            saw_raid_selection,
            "bandit should select RaidTarget before the deterrence threshold is crossed"
        );
        assert!(
            saw_attack_commit,
            "pre-threshold phases should still resolve through the normal attack action"
        );
        assert!(
            !h.agent_is_dead(ids.bandit),
            "bandit must survive long enough to demonstrate threshold-sensitive raid behavior"
        );
    }

    let tick = h.scheduler.current_tick().0;
    set_wound_total(&mut h, ids.bandit, 260, tick);
    assert!(
        wound_total(&h, ids.bandit) >= deterrence_threshold,
        "the final phase should begin at or above the deterrence threshold"
    );

    let final_traveler = ids.travelers[2];
    request_travel_to_place(&mut h, final_traveler, S49_BANDIT_CAMP);

    let mut final_arrived = false;
    let mut saw_post_threshold_planning = false;
    let mut saw_post_threshold_raid_candidate = false;
    let mut saw_post_threshold_raid_selection = false;
    let mut saw_post_threshold_attack = false;
    let final_phase_start_tick = h.scheduler.current_tick();

    for _ in 0..40 {
        h.step_once();
        final_arrived |= h.world.effective_place(final_traveler) == Some(S49_BANDIT_CAMP);
        if !final_arrived {
            continue;
        }

        if let Some(sink) = h.driver.trace_sink() {
            if let Some(trace) = sink.traces_for(ids.bandit).into_iter().last() {
                if let DecisionOutcome::Planning(planning) = &trace.outcome {
                    saw_post_threshold_planning = true;
                    let raid_goal = GoalKey::from(GoalKind::RaidTarget {
                        target: final_traveler,
                    });
                    saw_post_threshold_raid_candidate |= planning
                        .candidates
                        .generated
                        .iter()
                        .any(|goal| goal.goal_key == raid_goal);
                    saw_post_threshold_raid_selection |=
                        planning.selection.selected_goal() == Some(raid_goal);
                }
            }
        }

        saw_post_threshold_attack |= h
            .action_trace_sink()
            .expect("action tracing should stay enabled")
            .events_for(ids.bandit)
            .iter()
            .any(|event| {
                event.action_name == "attack"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
                    && event.tick >= final_phase_start_tick
            });
    }

    assert!(final_arrived, "final traveler should reach the camp for the suppressed phase");
    assert!(
        saw_post_threshold_planning,
        "after the final traveler arrives, the bandit should still produce planning traces"
    );
    assert!(
        !saw_post_threshold_raid_candidate,
        "once wound load crosses the deterrence threshold, RaidTarget should stop generating"
    );
    assert!(
        !saw_post_threshold_raid_selection,
        "once wound load crosses the deterrence threshold, RaidTarget should stop selecting"
    );
    assert!(
        !saw_post_threshold_attack,
        "the deterred bandit should not launch a third raid"
    );
    assert!(
        h.world.get_component_dead_at(final_traveler).is_none(),
        "the final traveler should remain alive once raid deterrence is active"
    );
    assert!(
        latest_planning_trace_selected_goal(&h, ids.bandit)
            != Some(GoalKey::from(GoalKind::RaidTarget { target: final_traveler })),
        "the first post-threshold selected goal should no longer be RaidTarget"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

// ---------------------------------------------------------------------------
// Scenario 22: Bandit Camp Destruction Chain
// ---------------------------------------------------------------------------
//
// Systems: Combat, Perception, Beliefs, AI, Travel, Production
// GoalKinds: ReduceDanger, RegroupWithFaction, AcquireCommodity(SelfConsume)
// ActionDomains: Combat, Generic, Production, Travel
// Places: HomeMarket, BanditRoad, BanditCamp, SafeFarm, RallyGlen
// Principles: 1, 4, 7, 12, 17, 25
//
// Setup: A custom five-place topology makes the danger-cost flip legible.
// Bandits occupy an active camp with faction policy and rally doctrine. Two
// otherwise identical hungry travelers hold the same source and threat
// beliefs. One plans immediately under fresh danger beliefs; one stays
// inactive until those same beliefs age to zero confidence. External guards
// attack the camp, survivors disperse, regroup, and re-establish camp at a
// rally glen.
//
// Proves:
// 1. Rally doctrine is acquired locally at the active camp and not by remote faction members.
// 2. External attack can break camp cohesion, produce abandonment, and leave concrete aftermath.
// 3. Survivors can later select RegroupWithFaction, travel, and re-establish camp elsewhere.
// 4. Fresh danger beliefs steer food acquisition toward the safe source, while stale beliefs later
//    reopen the shorter abandoned-camp route.
//
// Chain: active camp -> local rally belief -> external attack -> death / departure
// -> abandonment -> regroup travel -> establish camp -> stale danger decay
// -> downstream travel-route reversal.

#[test]
fn golden_t22_bandit_camp_destruction() {
    let _ = run_t22_scenario(Seed([222; 32]));
}

#[test]
fn golden_t22_bandit_camp_destruction_replays_deterministically() {
    let first = run_t22_scenario(Seed([222; 32]));
    let second = run_t22_scenario(Seed([222; 32]));
    assert_eq!(
        first, second,
        "T22 bandit camp destruction scenario should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 47: Pressure-Driven Raid Emergence
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI, Combat, Loot, Conservation
// GoalKinds: RaidTarget, LootCorpse, AcquireCommodity(SelfConsume)
// ActionDomains: Combat, Corpse, Production, Travel
// Places: S47BanditCamp, S47RoadJunction, S47SafeVillage
// Principles: 1, 4, 7, 17, 24
//
// Setup: Hungry bandits occupy a camp with a nearly empty bread supply and a
// local orchard row as the lawful non-raid alternative. A traveler carrying
// apples reaches the camp through an ordinary travel request. The raid should
// emerge only after co-location and local loot visibility make `RaidTarget`
// more valuable than the background harvest path.
//
// Proves:
// 1. `RaidTarget` is a proactive offensive path for co-located non-faction prey.
// 2. The raid resolves through ordinary `attack` and `loot` actions.
// 3. Local harvest remains lawful before prey arrives, so the raid is emergent rather than scripted.
//
// Chain: scarce camp supplies -> non-raid self-supply behavior -> traveler arrives
// with visible food -> RaidTarget generated/selected -> attack commit -> corpse loot.

#[test]
fn golden_pressure_driven_raid_emergence() {
    let _ = run_s47_scenario(Seed([47; 32]));
}

#[test]
fn golden_pressure_driven_raid_emergence_replays_deterministically() {
    let first = run_s47_scenario(Seed([47; 32]));
    let second = run_s47_scenario(Seed([47; 32]));
    assert_eq!(
        first, second,
        "Scenario 47 pressure-driven raid emergence should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 48: Raid-Belief Economic Cascade
// ---------------------------------------------------------------------------
//
// Systems: Combat, Perception, Beliefs, Social Tell, Enterprise, Travel, AI
// GoalKinds: RaidTarget, ShareBelief, RestockCommodity
// ActionDomains: Combat, Social, Travel, Production
// Places: S48Market, S48DangerousRoad, S48BanditCamp, S48SafeRoute, S48RemoteFarm
// Principles: 3, 7, 12, 14
//
// Setup: Bandits raid a traveler at a dangerous road while a witness observes.
// The witness then reaches the market through ordinary travel and relays the
// danger belief to an otherwise idle merchant who already knows a lawful remote
// apple source. After the tell commits, the merchant's next restock plan should
// prefer the longer safe route instead of the shorter dangerous road.
//
// Proves:
// 1. Witnessed raid danger can propagate through the generic `tell` path.
// 2. Merchant route adaptation happens only after lawful information transfer.
// 3. The route flip is planner-local perceived travel cost, not authoritative edge state.
//
// Chain: raid -> witness combat belief -> witness travel to market -> tell ->
// merchant danger belief -> safe-route restock selection.

#[test]
fn golden_raid_belief_economic_cascade() {
    let _ = run_s48_scenario(Seed([48; 32]));
}

#[test]
fn golden_raid_belief_economic_cascade_replays_deterministically() {
    let first = run_s48_scenario(Seed([48; 32]));
    let second = run_s48_scenario(Seed([48; 32]));
    assert_eq!(
        first, second,
        "Scenario 48 raid-belief economic cascade should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 49: Wound-Dampened Raid Spiral
// ---------------------------------------------------------------------------
//
// Systems: Combat, Wounds, AI, Needs
// GoalKinds: RaidTarget, ConsumeOwnedCommodity
// ActionDomains: Combat, Needs, Travel
// Places: S49BanditCamp, S49RoadJunction
// Principles: 1, 3, 8, 17
//
// Setup: A single hungry bandit at camp faces three travelers arriving one at
// a time with visible food. The first two arrivals should still be raided,
// producing accumulating concrete wounds on the bandit. Once those wounds pass
// the faction flee threshold after courage scaling, the third arrival should
// no longer generate or select `RaidTarget`.
//
// Proves:
// 1. Repeated raid combat produces concrete wound accumulation on the raider.
// 2. The bandit can still raid before crossing the wound deterrence threshold.
// 3. After threshold crossing, `RaidTarget` disappears without introducing a cooldown or hidden alias path.
//
// Chain: raid opportunity -> attack commit -> wound accumulation -> second raid
// -> threshold crossed -> third prey arrives -> RaidTarget omitted.

#[test]
fn golden_wound_dampened_raid_spiral() {
    let _ = run_s49_scenario(Seed([49; 32]));
}

#[test]
fn golden_wound_dampened_raid_spiral_replays_deterministically() {
    let first = run_s49_scenario(Seed([49; 32]));
    let second = run_s49_scenario(Seed([49; 32]));
    assert_eq!(
        first, second,
        "Scenario 49 wound-dampened raid spiral should replay deterministically"
    );
}
