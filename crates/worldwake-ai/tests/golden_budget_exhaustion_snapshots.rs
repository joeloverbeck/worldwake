mod golden_harness;

use golden_harness::*;
use std::num::NonZeroU32;
use worldwake_ai::{GoalKind, PlanSearchResult, build_planning_snapshot, build_semantics_table, generate_candidates, search_plan};
use worldwake_core::{
    BelievedEntityState, CognitiveProfile, CommodityKind, EntityId, ExecutionBudget, GoalKey,
    HomeostaticNeeds, KnownRecipes, MetabolismProfile, PerceptionSource, Quantity, ResourceSource,
    Tick, WorkstationTag, build_believed_entity_state,
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
            place("Dusty Trail", &[PlaceTag::Trail, PlaceTag::Road, PlaceTag::Crossroads]),
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
        max_plan_depth: 10,
        max_node_expansions: 300,
        max_candidates_per_expansion: 150,
        speculative_acquisition: true,
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

fn assert_budget_exhausted(
    result: PlanSearchResult,
    minimum_expansions: u16,
    context: &str,
) {
    match result {
        PlanSearchResult::BudgetExhausted { expansions_used } => {
            assert!(
                expansions_used >= minimum_expansions,
                "{context}: expected at least {minimum_expansions} expansions before exhaustion, got {expansions_used}"
            );
        }
        other => panic!("{context}: expected BudgetExhausted, got {other:?}"),
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

    let _coin = place_ground_commodity(&mut h, THORNWALL_VILLAGE, CommodityKind::Coin, Quantity(20));
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

    let _medicine = place_ground_commodity(&mut h, HEARTHSTONE_INN, CommodityKind::Medicine, Quantity(2));

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
    let _medicine = place_ground_commodity(&mut h, HEARTHSTONE_INN, CommodityKind::Medicine, Quantity(2));

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
fn merchant_vara_treat_wounds_at_dusty_trail_budgets_exhaust() {
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

    assert_budget_exhausted(
        result,
        300,
        "Merchant Vara TreatWounds snapshot should exhaust the expansion budget",
    );
}

#[ignore = "planner fix follow-up: treat-wounds budget exhaustion still reproduces"]
#[test]
fn merchant_vara_treat_wounds_at_dusty_trail_found_after_fix() {
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

    match result {
        PlanSearchResult::Found(_) => {}
        other => panic!("expected Found after planner fix, got {other:?}"),
    }
}

#[test]
fn kael_treat_wounds_vara_at_dusty_trail_budgets_exhaust() {
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

    assert_budget_exhausted(
        result,
        224,
        "Kael TreatWounds snapshot should exhaust the expansion budget",
    );
}

#[ignore = "planner fix follow-up: treat-wounds budget exhaustion still reproduces"]
#[test]
fn kael_treat_wounds_vara_at_dusty_trail_found_after_fix() {
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

    match result {
        PlanSearchResult::Found(_) => {}
        other => panic!("expected Found after planner fix, got {other:?}"),
    }
}
