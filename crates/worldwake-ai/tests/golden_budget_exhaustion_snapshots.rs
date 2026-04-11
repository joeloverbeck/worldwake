mod golden_harness;

use golden_harness::*;
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU8;
use std::num::NonZeroU32;
use worldwake_ai::{
    CommodityPurpose, GoalKind, PlanSearchResult, build_planning_snapshot, build_semantics_table,
    generate_candidates, search_plan,
};
use worldwake_core::{
    BelievedEntityState, CarryCapacity, CognitiveProfile, CommodityKind, CommodityValuationProfile,
    CommunicationProfile, ContentionDispositionProfile, EntityId, EpistemicDispositionProfile,
    ExecutionBudget, GoalKey, HomeostaticNeeds, IntentionDispositionProfile, IntentionDomainTag,
    JusticeDispositionProfile, KnownRecipes, LastSeenMemory, LoadUnits, MerchandiseProfile,
    MetabolismProfile, PatrolProfile, PatrolRoute, PerceptionSource, PursuitProfile, Quantity,
    ResourceSource, SubstitutePreferences, TellProfile, ThresholdBand, Tick, TradeCategory,
    TradeDispositionProfile, UtilityProfile, ViolationDispositionProfile, WorkstationTag,
    build_believed_entity_state,
};
use worldwake_sim::PerAgentBeliefView;

const THORNWALL_VILLAGE: EntityId = entity(700);
const DUSTY_TRAIL: EntityId = entity(701);
const ELDERGROVE_FOREST: EntityId = entity(702);
const HEARTHSTONE_INN: EntityId = entity(703);
const GOLDEN_FIELDS: EntityId = entity(704);

const fn entity(slot: u32) -> EntityId {
    EntityId {
        slot,
        generation: 0,
    }
}

fn place(name: &str, tags: &[worldwake_core::PlaceTag]) -> worldwake_core::Place {
    worldwake_core::Place {
        name: name.to_string(),
        capacity: None,
        tags: tags.iter().copied().collect(),
    }
}

fn connect(
    topology: &mut worldwake_core::Topology,
    base_id: u32,
    from: EntityId,
    to: EntityId,
    ticks: u32,
) {
    topology
        .add_edge(
            worldwake_core::TravelEdge::new(
                worldwake_core::TravelEdgeId(base_id),
                from,
                to,
                ticks,
                None,
            )
            .unwrap(),
        )
        .unwrap();
    topology
        .add_edge(
            worldwake_core::TravelEdge::new(
                worldwake_core::TravelEdgeId(base_id + 1),
                to,
                from,
                ticks,
                None,
            )
            .unwrap(),
        )
        .unwrap();
}

fn build_cli_evaluation_topology() -> worldwake_core::Topology {
    use worldwake_core::{PlaceTag, Topology};

    let mut topology = Topology::new();
    topology
        .add_place(
            THORNWALL_VILLAGE,
            place(
                "Thornwall Village",
                &[
                    PlaceTag::Village,
                    PlaceTag::Store,
                    PlaceTag::Hall,
                    PlaceTag::Gate,
                ],
            ),
        )
        .unwrap();
    topology
        .add_place(
            ELDERGROVE_FOREST,
            place("Eldergrove Forest", &[PlaceTag::Forest, PlaceTag::Camp]),
        )
        .unwrap();
    topology
        .add_place(
            DUSTY_TRAIL,
            place(
                "Dusty Trail",
                &[PlaceTag::Trail, PlaceTag::Road, PlaceTag::Crossroads],
            ),
        )
        .unwrap();
    topology
        .add_place(
            HEARTHSTONE_INN,
            place(
                "Hearthstone Inn",
                &[PlaceTag::Inn, PlaceTag::Latrine, PlaceTag::Barracks],
            ),
        )
        .unwrap();
    topology
        .add_place(
            GOLDEN_FIELDS,
            place("Golden Fields", &[PlaceTag::Field, PlaceTag::Farm]),
        )
        .unwrap();

    connect(&mut topology, 800, THORNWALL_VILLAGE, ELDERGROVE_FOREST, 3);
    connect(&mut topology, 810, THORNWALL_VILLAGE, DUSTY_TRAIL, 2);
    connect(&mut topology, 820, THORNWALL_VILLAGE, HEARTHSTONE_INN, 4);
    connect(&mut topology, 830, THORNWALL_VILLAGE, GOLDEN_FIELDS, 5);
    topology
        .add_edge(
            worldwake_core::TravelEdge::new(
                worldwake_core::TravelEdgeId(840),
                ELDERGROVE_FOREST,
                DUSTY_TRAIL,
                2,
                None,
            )
            .unwrap(),
        )
        .unwrap();
    topology
        .add_edge(
            worldwake_core::TravelEdge::new(
                worldwake_core::TravelEdgeId(850),
                HEARTHSTONE_INN,
                GOLDEN_FIELDS,
                8,
                None,
            )
            .unwrap(),
        )
        .unwrap();
    topology
        .add_edge(
            worldwake_core::TravelEdge::new(
                worldwake_core::TravelEdgeId(851),
                GOLDEN_FIELDS,
                HEARTHSTONE_INN,
                8,
                None,
            )
            .unwrap(),
        )
        .unwrap();
    topology
}

fn build_pathology_harness(seed: worldwake_core::Seed) -> GoldenHarness {
    let mut h = GoldenHarness::with_recipes(seed, build_multi_recipe_registry());
    h.world = worldwake_core::World::new(build_cli_evaluation_topology()).unwrap();
    h.event_log = worldwake_core::EventLog::new();
    h.scheduler = worldwake_sim::Scheduler::new(worldwake_sim::SystemManifest::canonical());
    h.controller = worldwake_sim::ControllerState::new();
    h
}

fn merchant_vara_cognitive_profile() -> CognitiveProfile {
    CognitiveProfile {
        max_candidates_to_plan: 4,
        max_plan_depth: 10,
        max_node_expansions: 300,
        max_candidates_per_expansion: 150,
        speculative_acquisition: true,
        landmark_extraction_depth: 3,
        ..CognitiveProfile::default()
    }
}

fn merchant_vara_execution_budget() -> ExecutionBudget {
    ExecutionBudget {
        beam_width: 10,
        max_prerequisite_locations: 3,
        preferred_operator_boost: 3,
    }
}

fn guard_perception_profile() -> worldwake_core::PerceptionProfile {
    worldwake_core::PerceptionProfile {
        entity_memory_capacity: 16,
        entity_claim_capacity: 16,
        memory_retention_ticks: 64,
        observation_fidelity: pm(950),
        confidence_policy: worldwake_core::BeliefConfidencePolicy::default(),
        institutional_memory_capacity: 20,
        consultation_speed_factor: pm(500),
        contradiction_tolerance: pm(300),
    }
}

fn merchant_vara_utility_profile() -> UtilityProfile {
    UtilityProfile {
        hunger_weight: pm(300),
        thirst_weight: pm(300),
        fatigue_weight: pm(200),
        bladder_weight: pm(200),
        dirtiness_weight: pm(200),
        pain_weight: pm(400),
        danger_weight: pm(600),
        enterprise_weight: pm(800),
        social_weight: pm(400),
        activity_awareness_weight: pm(300),
        side_benefit_weight: pm(500),
        bounty_posting_weight: pm(0),
        notice_posting_weight: pm(0),
        courage: pm(300),
        care_weight: pm(200),
    }
}

fn configure_merchant_vara_cli_components(h: &mut GoldenHarness, agent: EntityId) {
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        TellProfile {
            max_tell_candidates: 5,
            max_relay_chain_len: 4,
            conversation_memory_capacity: 16,
            conversation_memory_retention_ticks: 64,
        },
    );
    set_agent_communication_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        CommunicationProfile {
            alarm_acceptance: pm(950),
            testimony_acceptance: pm(800),
            gossip_acceptance: pm(350),
        },
    );
    set_agent_cognitive_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        merchant_vara_cognitive_profile(),
    );
    set_agent_execution_budget(
        &mut h.world,
        &mut h.event_log,
        agent,
        merchant_vara_execution_budget(),
    );

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_utility_profile(agent, merchant_vara_utility_profile())
        .unwrap();
    txn.set_component_merchandise_profile(
        agent,
        MerchandiseProfile {
            sale_kinds: BTreeSet::from([
                CommodityKind::Grain,
                CommodityKind::Apple,
                CommodityKind::Bread,
            ]),
            home_facility: Some(THORNWALL_VILLAGE),
        },
    )
    .unwrap();
    txn.set_component_trade_disposition_profile(
        agent,
        TradeDispositionProfile {
            negotiation_round_ticks: NonZeroU32::new(2).unwrap(),
            initial_offer_bias: pm(600),
            concession_rate: pm(100),
            rejection_escalation_rate: pm(200),
            demand_memory_retention_ticks: 50,
            market_presence_ticks: NonZeroU32::new(30).unwrap(),
        },
    )
    .unwrap();
    txn.set_component_contention_disposition_profile(
        agent,
        ContentionDispositionProfile {
            queue_patience_ticks: Some(NonZeroU32::new(30).unwrap()),
        },
    )
    .unwrap();
    txn.set_component_commodity_valuation_profile(
        agent,
        CommodityValuationProfile {
            recipe_opportunity_depth: NonZeroU8::new(2).unwrap(),
            recipe_place_horizon: 3,
            indirect_value_decay_per_step: pm(300),
        },
    )
    .unwrap();
    txn.set_component_substitute_preferences(
        agent,
        SubstitutePreferences {
            preferences: BTreeMap::from([(
                TradeCategory::Food,
                vec![
                    CommodityKind::Grain,
                    CommodityKind::Apple,
                    CommodityKind::Bread,
                ],
            )]),
        },
    )
    .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn seed_merchant_vara_cli_agent(
    h: &mut GoldenHarness,
    place: EntityId,
    needs: HomeostaticNeeds,
) -> EntityId {
    let agent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Merchant Vara",
        place,
        needs,
        MetabolismProfile::default(),
        merchant_vara_utility_profile(),
        KnownRecipes::default(),
    );
    configure_merchant_vara_cli_components(h, agent);
    agent
}

fn configure_guard_theron_cli_components(h: &mut GoldenHarness, agent: EntityId) {
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        guard_perception_profile(),
    );

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_drive_thresholds(
        agent,
        worldwake_core::DriveThresholds {
            hunger: ThresholdBand::new(pm(250), pm(500), pm(750), pm(900)).unwrap(),
            thirst: ThresholdBand::new(pm(200), pm(450), pm(700), pm(850)).unwrap(),
            fatigue: ThresholdBand::new(pm(300), pm(550), pm(800), pm(920)).unwrap(),
            bladder: ThresholdBand::new(pm(350), pm(600), pm(800), pm(930)).unwrap(),
            dirtiness: ThresholdBand::new(pm(400), pm(650), pm(850), pm(950)).unwrap(),
            pain: ThresholdBand::new(pm(200), pm(450), pm(700), pm(900)).unwrap(),
            danger: ThresholdBand::new(pm(200), pm(500), pm(750), pm(950)).unwrap(),
        },
    )
    .unwrap();
    txn.set_component_combat_profile(
        agent,
        worldwake_core::CombatProfile::new(
            pm(900),
            pm(750),
            pm(600),
            pm(550),
            pm(150),
            pm(400),
            pm(60),
            pm(250),
            pm(120),
            NonZeroU32::new(3).unwrap(),
            NonZeroU32::new(10).unwrap(),
        ),
    )
    .unwrap();
    txn.set_component_utility_profile(
        agent,
        UtilityProfile {
            hunger_weight: pm(400),
            thirst_weight: pm(400),
            fatigue_weight: pm(300),
            bladder_weight: pm(200),
            dirtiness_weight: pm(200),
            pain_weight: pm(600),
            danger_weight: pm(800),
            enterprise_weight: pm(200),
            social_weight: pm(300),
            activity_awareness_weight: pm(400),
            side_benefit_weight: pm(500),
            bounty_posting_weight: pm(700),
            notice_posting_weight: pm(900),
            courage: pm(850),
            care_weight: pm(400),
        },
    )
    .unwrap();
    txn.set_component_patrol_profile(
        agent,
        PatrolProfile {
            base_dwell_ticks: 5,
            dwell_vigilance_scale_ticks: 3,
            vigilance: pm(700),
            route_adaptation_sensitivity: pm(400),
            patrol_motive_weight: pm(600),
        },
    )
    .unwrap();
    txn.set_component_patrol_route(
        agent,
        PatrolRoute {
            assigned_places: vec![DUSTY_TRAIL, THORNWALL_VILLAGE],
            current_index: 0,
        },
    )
    .unwrap();
    txn.set_component_pursuit_profile(
        agent,
        PursuitProfile {
            min_location_confidence: pm(500),
            max_pursuit_travel_ticks: NonZeroU32::new(8).unwrap(),
        },
    )
    .unwrap();
    txn.set_component_intention_disposition_profile(
        agent,
        IntentionDispositionProfile {
            domain_patience: BTreeMap::from([
                (IntentionDomainTag::Travel, NonZeroU32::new(15).unwrap()),
                (IntentionDomainTag::Care, NonZeroU32::new(20).unwrap()),
            ]),
            default_patience_ticks: NonZeroU32::new(30).unwrap(),
            commitment_switch_margin: pm(200),
        },
    )
    .unwrap();
    txn.set_component_violation_disposition_profile(
        agent,
        ViolationDispositionProfile {
            investigation_duration_ticks: NonZeroU32::new(3).unwrap(),
            violation_memory_retention_ticks: 100,
            investigation_motive_weight: pm(600),
            ownership_motive_bonus: pm(300),
        },
    )
    .unwrap();
    txn.set_component_last_seen_memory(
        agent,
        LastSeenMemory {
            records: BTreeMap::new(),
            capacity: 30,
        },
    )
    .unwrap();
    txn.set_component_justice_disposition_profile(
        agent,
        JusticeDispositionProfile {
            accusation_motive_weight: pm(700),
            fine_severity: pm(500),
        },
    )
    .unwrap();
    txn.set_component_epistemic_disposition_profile(agent, EpistemicDispositionProfile::default())
        .unwrap();
    txn.set_component_carry_capacity(agent, CarryCapacity(LoadUnits(40)))
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn seed_guard_theron_cli_agent(
    h: &mut GoldenHarness,
    place: EntityId,
    needs: HomeostaticNeeds,
) -> EntityId {
    let agent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Guard Theron",
        place,
        needs,
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::default(),
    );
    configure_guard_theron_cli_components(h, agent);
    agent
}

fn seed_kael_cli_agent(
    h: &mut GoldenHarness,
    place: EntityId,
    needs: HomeostaticNeeds,
) -> EntityId {
    let agent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Kael",
        place,
        needs,
        MetabolismProfile::default(),
        UtilityProfile {
            hunger_weight: pm(500),
            thirst_weight: pm(500),
            fatigue_weight: pm(500),
            bladder_weight: pm(500),
            dirtiness_weight: pm(500),
            pain_weight: pm(500),
            danger_weight: pm(500),
            enterprise_weight: pm(500),
            social_weight: pm(500),
            activity_awareness_weight: pm(500),
            side_benefit_weight: pm(500),
            bounty_posting_weight: pm(0),
            notice_posting_weight: pm(0),
            courage: pm(500),
            care_weight: pm(500),
        },
        KnownRecipes::default(),
    );

    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        worldwake_core::PerceptionProfile {
            entity_memory_capacity: 16,
            entity_claim_capacity: 16,
            memory_retention_ticks: 64,
            observation_fidelity: pm(950),
            confidence_policy: worldwake_core::BeliefConfidencePolicy {
                direct_observation_base: pm(900),
                report_base: pm(700),
                rumor_base: pm(400),
                inference_base: pm(500),
                report_chain_penalty: pm(100),
                rumor_chain_penalty: pm(150),
                staleness_penalty_per_tick: pm(2),
            },
            institutional_memory_capacity: 20,
            consultation_speed_factor: pm(500),
            contradiction_tolerance: pm(300),
        },
    );
    set_agent_communication_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        CommunicationProfile {
            alarm_acceptance: pm(990),
            testimony_acceptance: pm(850),
            gossip_acceptance: pm(600),
        },
    );
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_epistemic_disposition_profile(
        agent,
        EpistemicDispositionProfile {
            stale_evidence_barrier_threshold: pm(400),
            witness_query_duration_ticks: NonZeroU32::new(2).unwrap(),
            ask_memory_retention_ticks: 12,
        },
    )
    .unwrap();
    commit_txn(txn, &mut h.event_log);
    agent
}

fn seed_forager_lina_cli_agent(h: &mut GoldenHarness) -> EntityId {
    let harvest_apples = h
        .recipes
        .recipe_by_name("Harvest Apples")
        .expect("pathology harness should include Harvest Apples")
        .0;
    let lina = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Forager Lina",
        ELDERGROVE_FOREST,
        HomeostaticNeeds::new(pm(200), pm(600), pm(100), pm(100), pm(100)),
        MetabolismProfile {
            hunger_rate: pm(2),
            thirst_rate: pm(5),
            fatigue_rate: pm(2),
            bladder_rate: pm(4),
            dirtiness_rate: pm(1),
            rest_efficiency: pm(20),
            starvation_tolerance_ticks: NonZeroU32::new(480).unwrap(),
            dehydration_tolerance_ticks: NonZeroU32::new(180).unwrap(),
            exhaustion_collapse_ticks: NonZeroU32::new(120).unwrap(),
            bladder_accident_tolerance_ticks: NonZeroU32::new(40).unwrap(),
            toilet_ticks: NonZeroU32::new(8).unwrap(),
            wash_ticks: NonZeroU32::new(12).unwrap(),
            travel_fatigue_multiplier: pm(0),
            travel_thirst_multiplier: pm(0),
            travel_bladder_multiplier: pm(0),
            wilderness_relief_dirtiness_penalty: pm(0),
        },
        UtilityProfile {
            hunger_weight: pm(600),
            thirst_weight: pm(700),
            fatigue_weight: pm(300),
            bladder_weight: pm(200),
            dirtiness_weight: pm(200),
            pain_weight: pm(500),
            danger_weight: pm(400),
            enterprise_weight: pm(400),
            social_weight: pm(300),
            activity_awareness_weight: pm(500),
            side_benefit_weight: pm(500),
            bounty_posting_weight: pm(0),
            notice_posting_weight: pm(0),
            courage: pm(400),
            care_weight: pm(300),
        },
        KnownRecipes::with([harvest_apples]),
    );

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_drive_thresholds(
        lina,
        worldwake_core::DriveThresholds {
            hunger: ThresholdBand::new(pm(250), pm(500), pm(750), pm(900)).unwrap(),
            thirst: ThresholdBand::new(pm(100), pm(300), pm(550), pm(750)).unwrap(),
            fatigue: ThresholdBand::new(pm(300), pm(550), pm(800), pm(920)).unwrap(),
            bladder: ThresholdBand::new(pm(350), pm(600), pm(800), pm(930)).unwrap(),
            dirtiness: ThresholdBand::new(pm(400), pm(650), pm(850), pm(950)).unwrap(),
            pain: ThresholdBand::new(pm(150), pm(350), pm(600), pm(850)).unwrap(),
            danger: ThresholdBand::new(pm(100), pm(300), pm(550), pm(800)).unwrap(),
        },
    )
    .unwrap();
    txn.set_component_preference_profile(
        lina,
        worldwake_core::PreferenceProfile {
            route_caution_weight: pm(400),
            source_trust_weight: pm(300),
            route_memory_capacity: 24,
            source_memory_capacity: 18,
            memory_retention_ticks: 400,
        },
    )
    .unwrap();
    txn.set_component_theft_disposition_profile(
        lina,
        worldwake_core::TheftDispositionProfile {
            steal_duration_ticks: NonZeroU32::new(3).unwrap(),
            theft_motive_weight: pm(200),
            witness_risk_penalty: pm(400),
        },
    )
    .unwrap();
    txn.set_component_exploration_profile(
        lina,
        worldwake_core::ExplorationProfile {
            curiosity_weight: pm(650),
            need_activation_threshold: pm(350),
            max_consecutive_explorations: 4,
            visit_lookback_ticks: 150,
            consecutive_exploration_count: 0,
        },
    )
    .unwrap();
    txn.set_component_disposal_profile(
        lina,
        worldwake_core::DisposalProfile {
            capacity_strain_threshold: pm(700),
        },
    )
    .unwrap();
    txn.set_component_carry_capacity(lina, CarryCapacity(LoadUnits(20)))
        .unwrap();
    commit_txn(txn, &mut h.event_log);
    lina
}

fn cli_scenario_seed(seed: u64) -> worldwake_core::Seed {
    let mut bytes = [0_u8; 32];
    bytes[..8].copy_from_slice(&seed.to_le_bytes());
    worldwake_core::Seed(bytes)
}

struct CliScenarioAgents {
    _kael: EntityId,
    merchant_vara: EntityId,
    _forager_lina: EntityId,
    guard_theron: EntityId,
}

fn setup_cli_evaluation_scenario_until_tick(
    target_tick: u32,
) -> (GoldenHarness, CliScenarioAgents) {
    let mut h = build_pathology_harness(cli_scenario_seed(7777));

    let kael = seed_kael_cli_agent(
        &mut h,
        THORNWALL_VILLAGE,
        HomeostaticNeeds::new(pm(500), pm(300), pm(100), pm(100), pm(100)),
    );
    let merchant_vara = seed_merchant_vara_cli_agent(
        &mut h,
        THORNWALL_VILLAGE,
        HomeostaticNeeds::new(pm(200), pm(200), pm(100), pm(100), pm(100)),
    );
    let forager_lina = seed_forager_lina_cli_agent(&mut h);
    let guard_theron = seed_guard_theron_cli_agent(
        &mut h,
        DUSTY_TRAIL,
        HomeostaticNeeds::new(pm(300), pm(300), pm(100), pm(100), pm(100)),
    );

    give_commodity(
        &mut h.world,
        &mut h.event_log,
        kael,
        THORNWALL_VILLAGE,
        CommodityKind::Coin,
        Quantity(20),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        kael,
        THORNWALL_VILLAGE,
        CommodityKind::Water,
        Quantity(5),
    );
    place_ground_commodity(&mut h, ELDERGROVE_FOREST, CommodityKind::Water, Quantity(5));
    place_ground_commodity(
        &mut h,
        THORNWALL_VILLAGE,
        CommodityKind::Grain,
        Quantity(10),
    );
    place_ground_commodity(&mut h, THORNWALL_VILLAGE, CommodityKind::Bread, Quantity(5));
    place_ground_commodity(&mut h, ELDERGROVE_FOREST, CommodityKind::Apple, Quantity(8));
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        guard_theron,
        DUSTY_TRAIL,
        CommodityKind::Sword,
        Quantity(1),
    );
    place_ground_commodity(
        &mut h,
        HEARTHSTONE_INN,
        CommodityKind::Firewood,
        Quantity(3),
    );
    place_ground_commodity(
        &mut h,
        HEARTHSTONE_INN,
        CommodityKind::Medicine,
        Quantity(2),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        guard_theron,
        DUSTY_TRAIL,
        CommodityKind::Bow,
        Quantity(1),
    );

    let _mill = place_workstation(
        &mut h.world,
        &mut h.event_log,
        THORNWALL_VILLAGE,
        WorkstationTag::Mill,
        ProductionOutputOwner::Actor,
    );
    let _loom = place_workstation(
        &mut h.world,
        &mut h.event_log,
        THORNWALL_VILLAGE,
        WorkstationTag::Loom,
        ProductionOutputOwner::Actor,
    );
    let _well = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        THORNWALL_VILLAGE,
        WorkstationTag::Well,
        ResourceSource {
            commodity: CommodityKind::Water,
            available_quantity: Quantity(15),
            max_quantity: Quantity(15),
            regeneration_ticks_per_unit: Some(NonZeroU32::new(3).unwrap()),
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );
    let _forge = place_workstation(
        &mut h.world,
        &mut h.event_log,
        HEARTHSTONE_INN,
        WorkstationTag::Forge,
        ProductionOutputOwner::Actor,
    );
    let _wash_basin = place_workstation(
        &mut h.world,
        &mut h.event_log,
        HEARTHSTONE_INN,
        WorkstationTag::WashBasin,
        ProductionOutputOwner::Actor,
    );
    let _chopping_block = place_workstation(
        &mut h.world,
        &mut h.event_log,
        ELDERGROVE_FOREST,
        WorkstationTag::ChoppingBlock,
        ProductionOutputOwner::Actor,
    );
    let _field_plot = place_workstation(
        &mut h.world,
        &mut h.event_log,
        GOLDEN_FIELDS,
        WorkstationTag::FieldPlot,
        ProductionOutputOwner::Actor,
    );
    let _grave_plot = place_workstation(
        &mut h.world,
        &mut h.event_log,
        GOLDEN_FIELDS,
        WorkstationTag::GravePlot,
        ProductionOutputOwner::Actor,
    );
    let _orchard = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ELDERGROVE_FOREST,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(20),
            max_quantity: Quantity(20),
            regeneration_ticks_per_unit: Some(NonZeroU32::new(2).unwrap()),
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );

    for _ in 0..target_tick {
        h.step_once();
    }

    (
        h,
        CliScenarioAgents {
            _kael: kael,
            merchant_vara,
            _forager_lina: forager_lina,
            guard_theron,
        },
    )
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

fn place_many_ground_commodities(
    h: &mut GoldenHarness,
    place: EntityId,
    commodity: CommodityKind,
    count: usize,
) -> Vec<EntityId> {
    (0..count)
        .map(|_| place_ground_commodity(h, place, commodity, Quantity(1)))
        .collect()
}

fn seed_agent_at(
    h: &mut GoldenHarness,
    name: &str,
    place: EntityId,
    needs: HomeostaticNeeds,
) -> EntityId {
    seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        name,
        place,
        needs,
        MetabolismProfile::default(),
        worldwake_core::UtilityProfile::default(),
        KnownRecipes::default(),
    )
}

fn set_wounds(h: &mut GoldenHarness, agent: EntityId, severity: u16) {
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_wound_list(agent, stable_wound_list(severity))
        .unwrap();
    txn.set_component_combat_profile(agent, no_recovery_combat_profile())
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn seed_custom_belief(
    h: &mut GoldenHarness,
    actor: EntityId,
    subject: EntityId,
    tick: Tick,
    source: PerceptionSource,
    update: impl FnOnce(&mut BelievedEntityState),
) {
    let mut belief = build_believed_entity_state(&h.world, subject, tick, source)
        .expect("subject should support belief projection");
    update(&mut belief);
    seed_belief(&mut h.world, &mut h.event_log, actor, subject, belief);
}

fn seed_beliefs_from_world(
    h: &mut GoldenHarness,
    actor: EntityId,
    tick: Tick,
    source: PerceptionSource,
    subjects: impl IntoIterator<Item = EntityId>,
) {
    for subject in subjects {
        seed_belief_from_world(&mut h.world, &mut h.event_log, actor, subject, tick, source);
    }
}

fn seed_subject_belief_at_place(
    h: &mut GoldenHarness,
    actor: EntityId,
    subject: EntityId,
    tick: Tick,
    source: PerceptionSource,
    place: EntityId,
) {
    seed_custom_belief(h, actor, subject, tick, source, |belief| {
        belief.last_known_place = Some(place);
    });
}

fn find_acquire_commodity_candidate(
    h: &GoldenHarness,
    actor: EntityId,
    commodity: CommodityKind,
    tick: Tick,
) -> worldwake_ai::GroundedGoal {
    let blocked = h
        .world
        .get_component_blocked_intent_memory(actor)
        .cloned()
        .unwrap_or_default();
    let view = PerAgentBeliefView::from_world_at_tick_with_recipes(
        actor,
        tick,
        &h.world,
        Some(&h.recipes),
    );
    generate_candidates(&view, actor, &blocked, &h.recipes, tick)
        .into_iter()
        .find(|candidate| {
            candidate.key
                == GoalKey::from(GoalKind::AcquireCommodity {
                    commodity,
                    purpose: CommodityPurpose::SelfConsume,
                })
        })
        .expect("snapshot should generate AcquireCommodity(SelfConsume)")
}

fn search_acquire_commodity(
    h: &GoldenHarness,
    actor: EntityId,
    commodity: CommodityKind,
    tick: Tick,
    cognitive: &CognitiveProfile,
    execution_budget: ExecutionBudget,
) -> PlanSearchResult {
    let blocked = h
        .world
        .get_component_blocked_intent_memory(actor)
        .cloned()
        .unwrap_or_default();
    let grounded = find_acquire_commodity_candidate(h, actor, commodity, tick);
    let view = PerAgentBeliefView::from_world_at_tick_with_recipes(
        actor,
        tick,
        &h.world,
        Some(&h.recipes),
    );
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &grounded.evidence_entities,
        &grounded.evidence_places,
        cognitive.snapshot_travel_horizon,
    );
    let semantics = build_semantics_table(&h.defs);

    search_plan(
        &snapshot,
        &grounded,
        &semantics,
        &h.defs,
        &h.handlers,
        cognitive,
        &execution_budget,
        &h.recipes,
        &blocked,
        tick,
        None,
        None,
    )
}

fn find_treat_wounds_candidate(
    h: &GoldenHarness,
    actor: EntityId,
    patient: EntityId,
    tick: Tick,
) -> worldwake_ai::GroundedGoal {
    let blocked = h
        .world
        .get_component_blocked_intent_memory(actor)
        .cloned()
        .unwrap_or_default();
    let view = PerAgentBeliefView::from_world_at_tick_with_recipes(
        actor,
        tick,
        &h.world,
        Some(&h.recipes),
    );
    generate_candidates(&view, actor, &blocked, &h.recipes, tick)
        .into_iter()
        .find(|candidate| candidate.key == GoalKey::from(GoalKind::TreatWounds { patient }))
        .expect("snapshot should generate TreatWounds")
}

fn search_treat_wounds(
    h: &GoldenHarness,
    actor: EntityId,
    patient: EntityId,
    tick: Tick,
    cognitive: &CognitiveProfile,
    execution_budget: ExecutionBudget,
) -> PlanSearchResult {
    let blocked = h
        .world
        .get_component_blocked_intent_memory(actor)
        .cloned()
        .unwrap_or_default();
    let grounded = find_treat_wounds_candidate(h, actor, patient, tick);
    let view = PerAgentBeliefView::from_world_at_tick_with_recipes(
        actor,
        tick,
        &h.world,
        Some(&h.recipes),
    );
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &grounded.evidence_entities,
        &grounded.evidence_places,
        cognitive.snapshot_travel_horizon,
    );
    let semantics = build_semantics_table(&h.defs);

    search_plan(
        &snapshot,
        &grounded,
        &semantics,
        &h.defs,
        &h.handlers,
        cognitive,
        &execution_budget,
        &h.recipes,
        &blocked,
        tick,
        None,
        None,
    )
}

enum ExpectedExhaustion {
    Budget(u16),
    Frontier(u16),
}

fn assert_exact_exhaustion(result: PlanSearchResult, expected: ExpectedExhaustion, context: &str) {
    match (result, expected) {
        (
            PlanSearchResult::BudgetExhausted { expansions_used },
            ExpectedExhaustion::Budget(expected_expansions),
        ) => {
            assert_eq!(
                expansions_used, expected_expansions,
                "{context}: expected BudgetExhausted with {expected_expansions} expansions"
            );
        }
        (
            PlanSearchResult::FrontierExhausted { expansions_used },
            ExpectedExhaustion::Frontier(expected_expansions),
        ) => {
            assert_eq!(
                expansions_used, expected_expansions,
                "{context}: expected FrontierExhausted with {expected_expansions} expansions"
            );
        }
        (other, ExpectedExhaustion::Budget(expected_expansions)) => {
            panic!(
                "{context}: expected BudgetExhausted {{ expansions_used: {expected_expansions} }}, got {other:?}"
            );
        }
        (other, ExpectedExhaustion::Frontier(expected_expansions)) => {
            panic!(
                "{context}: expected FrontierExhausted {{ expansions_used: {expected_expansions} }}, got {other:?}"
            );
        }
    }
}

fn setup_merchant_vara_treat_wounds_snapshot() -> (GoldenHarness, EntityId) {
    let tick = Tick(456);
    let mut h = build_pathology_harness(worldwake_core::Seed([145; 32]));

    let merchant_vara = seed_agent_at(
        &mut h,
        "Merchant Vara",
        DUSTY_TRAIL,
        HomeostaticNeeds::new(pm(214), pm(1000), pm(294), pm(120), pm(557)),
    );
    set_agent_cognitive_profile(
        &mut h.world,
        &mut h.event_log,
        merchant_vara,
        merchant_vara_cognitive_profile(),
    );
    set_agent_execution_budget(
        &mut h.world,
        &mut h.event_log,
        merchant_vara,
        merchant_vara_execution_budget(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        merchant_vara,
        DUSTY_TRAIL,
        CommodityKind::Grain,
        Quantity(5),
    );
    set_wounds(&mut h, merchant_vara, 500);

    let _guard_theron = seed_agent_at(
        &mut h,
        "Guard Theron",
        DUSTY_TRAIL,
        HomeostaticNeeds::new(pm(100), pm(100), pm(100), pm(100), pm(100)),
    );
    let kael = seed_agent_at(
        &mut h,
        "Kael",
        THORNWALL_VILLAGE,
        HomeostaticNeeds::new(pm(100), pm(100), pm(100), pm(100), pm(100)),
    );

    let _bow = place_ground_commodity(&mut h, DUSTY_TRAIL, CommodityKind::Bow, Quantity(1));
    let _sword = place_ground_commodity(&mut h, DUSTY_TRAIL, CommodityKind::Sword, Quantity(1));
    let waste = place_many_ground_commodities(&mut h, DUSTY_TRAIL, CommodityKind::Waste, 15);

    let _coin =
        place_ground_commodity(&mut h, THORNWALL_VILLAGE, CommodityKind::Coin, Quantity(20));
    let _mill = place_workstation(
        &mut h.world,
        &mut h.event_log,
        THORNWALL_VILLAGE,
        WorkstationTag::Mill,
        ProductionOutputOwner::Actor,
    );
    let _loom = place_workstation(
        &mut h.world,
        &mut h.event_log,
        THORNWALL_VILLAGE,
        WorkstationTag::Loom,
        ProductionOutputOwner::Actor,
    );
    let _well = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        THORNWALL_VILLAGE,
        WorkstationTag::Well,
        ResourceSource {
            commodity: CommodityKind::Water,
            available_quantity: Quantity(20),
            max_quantity: Quantity(20),
            regeneration_ticks_per_unit: Some(NonZeroU32::new(2).unwrap()),
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );

    let _medicine = place_ground_commodity(
        &mut h,
        HEARTHSTONE_INN,
        CommodityKind::Medicine,
        Quantity(2),
    );

    seed_custom_belief(
        &mut h,
        merchant_vara,
        kael,
        tick,
        PerceptionSource::Inference,
        |belief| belief.last_known_place = Some(DUSTY_TRAIL),
    );
    for waste_entity in waste.iter().take(9) {
        seed_belief_from_world(
            &mut h.world,
            &mut h.event_log,
            merchant_vara,
            *waste_entity,
            tick,
            PerceptionSource::Inference,
        );
    }

    (h, merchant_vara)
}

fn setup_merchant_vara_water_at_thornwall_snapshot() -> (GoldenHarness, EntityId) {
    let (h, agents) = setup_cli_evaluation_scenario_until_tick(11);
    assert_eq!(
        h.world.effective_place(agents.merchant_vara),
        Some(THORNWALL_VILLAGE),
        "Merchant Vara should be at Thornwall Village by tick 11",
    );
    (h, agents.merchant_vara)
}

fn setup_guard_theron_water_at_thornwall_snapshot() -> (GoldenHarness, EntityId) {
    let (h, agents) = setup_cli_evaluation_scenario_until_tick(25);
    assert_eq!(
        h.world.effective_place(agents.guard_theron),
        Some(THORNWALL_VILLAGE),
        "Guard Theron should be at Thornwall Village by tick 25",
    );
    (h, agents.guard_theron)
}

fn setup_merchant_vara_apple_at_dusty_trail_snapshot() -> (GoldenHarness, EntityId) {
    let tick = Tick(85);
    let mut h = build_pathology_harness(worldwake_core::Seed([143; 32]));

    let merchant_vara = seed_merchant_vara_cli_agent(
        &mut h,
        DUSTY_TRAIL,
        HomeostaticNeeds::new(pm(192), pm(458), pm(272), pm(76), pm(186)),
    );
    let kael = seed_agent_at(
        &mut h,
        "Kael",
        DUSTY_TRAIL,
        HomeostaticNeeds::new(pm(100), pm(100), pm(100), pm(100), pm(100)),
    );
    let guard_theron = seed_agent_at(
        &mut h,
        "Guard Theron",
        THORNWALL_VILLAGE,
        HomeostaticNeeds::new(pm(100), pm(100), pm(100), pm(100), pm(100)),
    );

    let grain: Vec<_> = (0..9)
        .map(|_| {
            give_commodity(
                &mut h.world,
                &mut h.event_log,
                merchant_vara,
                DUSTY_TRAIL,
                CommodityKind::Grain,
                Quantity(1),
            )
        })
        .collect();
    let _coin = place_ground_commodity(&mut h, DUSTY_TRAIL, CommodityKind::Coin, Quantity(20));
    let bread = place_many_ground_commodities(&mut h, DUSTY_TRAIL, CommodityKind::Bread, 3);
    let _water = place_many_ground_commodities(&mut h, DUSTY_TRAIL, CommodityKind::Water, 3);
    let waste = place_many_ground_commodities(&mut h, DUSTY_TRAIL, CommodityKind::Waste, 4);
    let bow = place_ground_commodity(&mut h, THORNWALL_VILLAGE, CommodityKind::Bow, Quantity(1));
    let sword =
        place_ground_commodity(&mut h, THORNWALL_VILLAGE, CommodityKind::Sword, Quantity(1));
    let mill = place_workstation(
        &mut h.world,
        &mut h.event_log,
        THORNWALL_VILLAGE,
        WorkstationTag::Mill,
        ProductionOutputOwner::Actor,
    );
    let loom = place_workstation(
        &mut h.world,
        &mut h.event_log,
        THORNWALL_VILLAGE,
        WorkstationTag::Loom,
        ProductionOutputOwner::Actor,
    );
    let well = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        THORNWALL_VILLAGE,
        WorkstationTag::Well,
        ResourceSource {
            commodity: CommodityKind::Water,
            available_quantity: Quantity(20),
            max_quantity: Quantity(20),
            regeneration_ticks_per_unit: Some(NonZeroU32::new(2).unwrap()),
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );
    let _forager_lina = seed_agent_at(
        &mut h,
        "Forager Lina",
        ELDERGROVE_FOREST,
        HomeostaticNeeds::new(pm(100), pm(100), pm(100), pm(100), pm(100)),
    );
    let _chopping_block = place_workstation(
        &mut h.world,
        &mut h.event_log,
        ELDERGROVE_FOREST,
        WorkstationTag::ChoppingBlock,
        ProductionOutputOwner::Actor,
    );
    let _orchard = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ELDERGROVE_FOREST,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(20),
            max_quantity: Quantity(20),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );
    let _eldergrove_waste =
        place_ground_commodity(&mut h, ELDERGROVE_FOREST, CommodityKind::Waste, Quantity(8));

    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        merchant_vara,
        THORNWALL_VILLAGE,
        tick,
        PerceptionSource::Inference,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        merchant_vara,
        DUSTY_TRAIL,
        tick,
        PerceptionSource::DirectObservation,
    );
    seed_beliefs_from_world(
        &mut h,
        merchant_vara,
        tick,
        PerceptionSource::DirectObservation,
        [kael],
    );
    seed_beliefs_from_world(
        &mut h,
        merchant_vara,
        tick,
        PerceptionSource::DirectObservation,
        bread.iter().chain(grain.iter()).copied(),
    );
    seed_subject_belief_at_place(
        &mut h,
        merchant_vara,
        merchant_vara,
        tick,
        PerceptionSource::DirectObservation,
        DUSTY_TRAIL,
    );
    seed_subject_belief_at_place(
        &mut h,
        merchant_vara,
        guard_theron,
        tick,
        PerceptionSource::Inference,
        DUSTY_TRAIL,
    );
    seed_beliefs_from_world(
        &mut h,
        merchant_vara,
        tick,
        PerceptionSource::Inference,
        [mill, loom, well],
    );
    for waste_entity in waste.iter().take(3) {
        seed_belief_from_world(
            &mut h.world,
            &mut h.event_log,
            merchant_vara,
            *waste_entity,
            tick,
            PerceptionSource::DirectObservation,
        );
    }
    let _ = (bow, sword);

    (h, merchant_vara)
}

fn setup_kael_water_at_thornwall_late_game_snapshot() -> (GoldenHarness, EntityId) {
    let tick = Tick(411);
    let mut h = build_pathology_harness(worldwake_core::Seed([144; 32]));

    let kael = seed_agent_at(
        &mut h,
        "Kael",
        THORNWALL_VILLAGE,
        HomeostaticNeeds::new(pm(42), pm(411), pm(284), pm(156), pm(512)),
    );
    let merchant_vara = seed_agent_at(
        &mut h,
        "Merchant Vara",
        DUSTY_TRAIL,
        HomeostaticNeeds::new(pm(100), pm(100), pm(100), pm(100), pm(100)),
    );
    let guard_theron = seed_agent_at(
        &mut h,
        "Guard Theron",
        DUSTY_TRAIL,
        HomeostaticNeeds::new(pm(100), pm(100), pm(100), pm(100), pm(100)),
    );

    let _bow = place_ground_commodity(&mut h, DUSTY_TRAIL, CommodityKind::Bow, Quantity(1));
    let _grain = place_ground_commodity(&mut h, DUSTY_TRAIL, CommodityKind::Grain, Quantity(5));
    let _sword = place_ground_commodity(&mut h, DUSTY_TRAIL, CommodityKind::Sword, Quantity(1));
    let waste = place_many_ground_commodities(&mut h, DUSTY_TRAIL, CommodityKind::Waste, 14);
    let coin = give_commodity(
        &mut h.world,
        &mut h.event_log,
        kael,
        THORNWALL_VILLAGE,
        CommodityKind::Coin,
        Quantity(20),
    );
    let mill = place_workstation(
        &mut h.world,
        &mut h.event_log,
        THORNWALL_VILLAGE,
        WorkstationTag::Mill,
        ProductionOutputOwner::Actor,
    );
    let loom = place_workstation(
        &mut h.world,
        &mut h.event_log,
        THORNWALL_VILLAGE,
        WorkstationTag::Loom,
        ProductionOutputOwner::Actor,
    );
    let well = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        THORNWALL_VILLAGE,
        WorkstationTag::Well,
        ResourceSource {
            commodity: CommodityKind::Water,
            available_quantity: Quantity(20),
            max_quantity: Quantity(20),
            regeneration_ticks_per_unit: Some(NonZeroU32::new(2).unwrap()),
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );

    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        kael,
        THORNWALL_VILLAGE,
        tick,
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        kael,
        DUSTY_TRAIL,
        tick,
        PerceptionSource::Inference,
    );
    seed_beliefs_from_world(
        &mut h,
        kael,
        tick,
        PerceptionSource::DirectObservation,
        [coin, mill, loom, well],
    );
    seed_subject_belief_at_place(
        &mut h,
        kael,
        kael,
        tick,
        PerceptionSource::Inference,
        DUSTY_TRAIL,
    );
    seed_subject_belief_at_place(
        &mut h,
        kael,
        merchant_vara,
        tick,
        PerceptionSource::Inference,
        DUSTY_TRAIL,
    );
    seed_subject_belief_at_place(
        &mut h,
        kael,
        guard_theron,
        tick,
        PerceptionSource::Inference,
        DUSTY_TRAIL,
    );
    for waste_entity in waste.iter().take(7) {
        seed_subject_belief_at_place(
            &mut h,
            kael,
            *waste_entity,
            tick,
            PerceptionSource::Inference,
            DUSTY_TRAIL,
        );
    }

    (h, kael)
}

fn setup_kael_treats_vara_snapshot() -> (GoldenHarness, EntityId, EntityId) {
    let tick = Tick(471);
    let mut h = build_pathology_harness(worldwake_core::Seed([146; 32]));

    let kael = seed_agent_at(
        &mut h,
        "Kael",
        DUSTY_TRAIL,
        HomeostaticNeeds::new(pm(162), pm(591), pm(304), pm(8), pm(572)),
    );
    let merchant_vara = seed_agent_at(
        &mut h,
        "Merchant Vara",
        DUSTY_TRAIL,
        HomeostaticNeeds::new(pm(214), pm(1000), pm(294), pm(120), pm(557)),
    );
    set_wounds(&mut h, merchant_vara, 500);

    let _guard_theron = seed_agent_at(
        &mut h,
        "Guard Theron",
        DUSTY_TRAIL,
        HomeostaticNeeds::new(pm(100), pm(100), pm(100), pm(100), pm(100)),
    );

    give_commodity(
        &mut h.world,
        &mut h.event_log,
        kael,
        DUSTY_TRAIL,
        CommodityKind::Coin,
        Quantity(20),
    );
    let _grain = place_ground_commodity(&mut h, DUSTY_TRAIL, CommodityKind::Grain, Quantity(5));
    let _bow = place_ground_commodity(&mut h, DUSTY_TRAIL, CommodityKind::Bow, Quantity(1));
    let _sword = place_ground_commodity(&mut h, DUSTY_TRAIL, CommodityKind::Sword, Quantity(1));
    let waste = place_many_ground_commodities(&mut h, DUSTY_TRAIL, CommodityKind::Waste, 16);

    let thornwall_place_belief = build_believed_entity_state(
        &h.world,
        THORNWALL_VILLAGE,
        tick,
        PerceptionSource::Inference,
    )
    .expect("place beliefs should build");
    seed_belief(
        &mut h.world,
        &mut h.event_log,
        kael,
        THORNWALL_VILLAGE,
        thornwall_place_belief,
    );

    let mill = place_workstation(
        &mut h.world,
        &mut h.event_log,
        THORNWALL_VILLAGE,
        WorkstationTag::Mill,
        ProductionOutputOwner::Actor,
    );
    let loom = place_workstation(
        &mut h.world,
        &mut h.event_log,
        THORNWALL_VILLAGE,
        WorkstationTag::Loom,
        ProductionOutputOwner::Actor,
    );
    let well = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        THORNWALL_VILLAGE,
        WorkstationTag::Well,
        ResourceSource {
            commodity: CommodityKind::Water,
            available_quantity: Quantity(20),
            max_quantity: Quantity(20),
            regeneration_ticks_per_unit: Some(NonZeroU32::new(2).unwrap()),
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );
    let _medicine = place_ground_commodity(
        &mut h,
        HEARTHSTONE_INN,
        CommodityKind::Medicine,
        Quantity(2),
    );

    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        kael,
        merchant_vara,
        tick,
        PerceptionSource::DirectObservation,
    );
    for subject in [mill, loom, well] {
        seed_belief_from_world(
            &mut h.world,
            &mut h.event_log,
            kael,
            subject,
            tick,
            PerceptionSource::Inference,
        );
    }
    for waste_entity in waste.iter().take(9) {
        seed_belief_from_world(
            &mut h.world,
            &mut h.event_log,
            kael,
            *waste_entity,
            tick,
            PerceptionSource::Inference,
        );
    }

    (h, kael, merchant_vara)
}

#[test]
fn merchant_vara_water_at_thornwall_residual_budget_exhaustion_contract() {
    let tick = Tick(11);
    let (h, merchant_vara) = setup_merchant_vara_water_at_thornwall_snapshot();
    let result = search_acquire_commodity(
        &h,
        merchant_vara,
        CommodityKind::Water,
        tick,
        &merchant_vara_cognitive_profile(),
        merchant_vara_execution_budget(),
    );

    assert_exact_exhaustion(
        result,
        ExpectedExhaustion::Budget(300),
        "Merchant Vara AcquireCommodity(Water) snapshot should still budget-exhaust after commodity pruning",
    );
}

#[test]
fn guard_theron_water_at_thornwall_residual_budget_exhaustion_contract() {
    let tick = Tick(25);
    let (h, guard_theron) = setup_guard_theron_water_at_thornwall_snapshot();
    let result = search_acquire_commodity(
        &h,
        guard_theron,
        CommodityKind::Water,
        tick,
        &CognitiveProfile::default(),
        ExecutionBudget::default(),
    );

    assert_exact_exhaustion(
        result,
        ExpectedExhaustion::Budget(224),
        "Guard Theron AcquireCommodity(Water) snapshot should still budget-exhaust after commodity pruning",
    );
}

#[test]
fn merchant_vara_apple_at_dusty_trail_residual_budget_exhaustion_contract() {
    let tick = Tick(85);
    let (h, merchant_vara) = setup_merchant_vara_apple_at_dusty_trail_snapshot();
    let result = search_acquire_commodity(
        &h,
        merchant_vara,
        CommodityKind::Apple,
        tick,
        &merchant_vara_cognitive_profile(),
        merchant_vara_execution_budget(),
    );

    assert_exact_exhaustion(
        result,
        ExpectedExhaustion::Budget(300),
        "Merchant Vara AcquireCommodity(Apple) snapshot should still budget-exhaust after commodity pruning",
    );
}

#[test]
fn kael_water_at_thornwall_late_game_residual_budget_exhaustion_contract() {
    let tick = Tick(411);
    let (h, kael) = setup_kael_water_at_thornwall_late_game_snapshot();
    let result = search_acquire_commodity(
        &h,
        kael,
        CommodityKind::Water,
        tick,
        &CognitiveProfile::default(),
        ExecutionBudget::default(),
    );

    assert_exact_exhaustion(
        result,
        ExpectedExhaustion::Budget(224),
        "Kael AcquireCommodity(Water) late-game snapshot should still budget-exhaust after commodity pruning",
    );
}

#[test]
fn merchant_vara_treat_wounds_at_dusty_trail_residual_budget_exhaustion_contract() {
    let tick = Tick(456);
    let (h, merchant_vara) = setup_merchant_vara_treat_wounds_snapshot();
    let result = search_treat_wounds(
        &h,
        merchant_vara,
        merchant_vara,
        tick,
        &merchant_vara_cognitive_profile(),
        merchant_vara_execution_budget(),
    );

    assert_exact_exhaustion(
        result,
        ExpectedExhaustion::Budget(300),
        "Merchant Vara TreatWounds snapshot should still budget-exhaust after commodity pruning",
    );
}

#[test]
fn kael_treat_wounds_vara_at_dusty_trail_residual_frontier_exhaustion_contract() {
    let tick = Tick(471);
    let (h, kael, merchant_vara) = setup_kael_treats_vara_snapshot();
    let result = search_treat_wounds(
        &h,
        kael,
        merchant_vara,
        tick,
        &CognitiveProfile::default(),
        ExecutionBudget::default(),
    );

    assert_exact_exhaustion(
        result,
        ExpectedExhaustion::Frontier(54),
        "Kael TreatWounds snapshot should now frontier-exhaust after commodity pruning",
    );
}
