use std::collections::BTreeSet;
use std::num::NonZeroU32;

use worldwake_core::{
    total_authoritative_commodity_quantity, verify_live_lot_conservation, BodyCostPerTick,
    CarryCapacity, CauseRef, CommodityKind, ControlSource, EventLog, LoadUnits, Place, Quantity,
    ResourceSource, Seed, Tick, Topology, TravelEdge, TravelEdgeId, VisibilitySpec, WitnessData,
    WorkstationMarker, WorkstationTag, World, WorldTxn,
};
use worldwake_sim::{
    get_affordances, step_tick, ActionDefId, ActionDefRegistry, ActionHandlerRegistry,
    ControllerState, DeterministicRng, InputKind, RecipeDefinition, RecipeRegistry, Scheduler,
    SystemManifest, TickStepError, TickStepResult, TickStepServices,
};
use worldwake_systems::{
    dispatch_table, register_craft_actions, register_harvest_actions, register_transport_actions,
    register_travel_actions,
};

fn nz(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).unwrap()
}

fn entity(slot: u32) -> worldwake_core::EntityId {
    worldwake_core::EntityId {
        slot,
        generation: 0,
    }
}

fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
    WorldTxn::new(
        world,
        Tick(tick),
        CauseRef::Bootstrap,
        None,
        None,
        VisibilitySpec::SamePlace,
        WitnessData::default(),
    )
}

fn commit_txn(txn: WorldTxn<'_>) {
    let mut event_log = EventLog::new();
    let _ = txn.commit(&mut event_log);
}

fn recipe_registry() -> RecipeRegistry {
    let mut recipes = RecipeRegistry::new();
    recipes.register(RecipeDefinition {
        name: "Harvest Apples".to_string(),
        inputs: Vec::new(),
        outputs: vec![(CommodityKind::Apple, Quantity(2))],
        work_ticks: nz(2),
        required_workstation_tag: Some(WorkstationTag::OrchardRow),
        required_tool_kinds: Vec::new(),
        body_cost_per_tick: BodyCostPerTick::zero(),
    });
    recipes.register(RecipeDefinition {
        name: "Bake Bread".to_string(),
        inputs: vec![(CommodityKind::Grain, Quantity(2))],
        outputs: vec![(CommodityKind::Bread, Quantity(1))],
        work_ticks: nz(2),
        required_workstation_tag: Some(WorkstationTag::Mill),
        required_tool_kinds: Vec::new(),
        body_cost_per_tick: BodyCostPerTick::zero(),
    });
    recipes
}

fn integration_topology() -> Topology {
    let mut topology = Topology::new();
    for (slot, name) in [(1, "Orchard"), (2, "Bridge"), (3, "Bakery")] {
        topology
            .add_place(
                entity(slot),
                Place {
                    name: name.to_string(),
                    capacity: None,
                    tags: BTreeSet::new(),
                },
            )
            .unwrap();
    }
    topology
        .add_edge(TravelEdge::new(TravelEdgeId(10), entity(1), entity(2), 2, None).unwrap())
        .unwrap();
    topology
        .add_edge(TravelEdge::new(TravelEdgeId(11), entity(2), entity(1), 2, None).unwrap())
        .unwrap();
    topology
        .add_edge(TravelEdge::new(TravelEdgeId(12), entity(2), entity(3), 2, None).unwrap())
        .unwrap();
    topology
        .add_edge(TravelEdge::new(TravelEdgeId(13), entity(3), entity(2), 2, None).unwrap())
        .unwrap();
    topology
}

struct Harness {
    world: World,
    event_log: EventLog,
    scheduler: Scheduler,
    controller: ControllerState,
    rng: DeterministicRng,
    defs: ActionDefRegistry,
    handlers: ActionHandlerRegistry,
    actor: worldwake_core::EntityId,
    orchard_place: worldwake_core::EntityId,
    bridge_place: worldwake_core::EntityId,
    bakery_place: worldwake_core::EntityId,
    orchard_workstation: worldwake_core::EntityId,
    mill_workstation: worldwake_core::EntityId,
}

impl Harness {
    fn new(source: ResourceSource) -> Self {
        let mut world = World::new(integration_topology()).unwrap();
        let orchard_place = entity(1);
        let bridge_place = entity(2);
        let bakery_place = entity(3);
        let (actor, orchard_workstation, mill_workstation) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let orchard = txn.create_entity(worldwake_core::EntityKind::Facility);
            let mill = txn.create_entity(worldwake_core::EntityKind::Facility);
            txn.set_ground_location(actor, orchard_place).unwrap();
            txn.set_component_carry_capacity(actor, CarryCapacity(LoadUnits(10)))
                .unwrap();
            txn.set_component_known_recipes(
                actor,
                worldwake_core::KnownRecipes::with([
                    worldwake_core::RecipeId(0),
                    worldwake_core::RecipeId(1),
                ]),
            )
            .unwrap();

            txn.set_ground_location(orchard, orchard_place).unwrap();
            txn.set_component_workstation_marker(
                orchard,
                WorkstationMarker(WorkstationTag::OrchardRow),
            )
            .unwrap();
            txn.set_component_resource_source(orchard, source).unwrap();

            txn.set_ground_location(mill, orchard_place).unwrap();
            txn.set_component_workstation_marker(mill, WorkstationMarker(WorkstationTag::Mill))
                .unwrap();

            commit_txn(txn);
            (actor, orchard, mill)
        };

        let recipes = recipe_registry();
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        register_harvest_actions(&mut defs, &mut handlers, &recipes);
        register_craft_actions(&mut defs, &mut handlers, &recipes);
        let _ = register_transport_actions(&mut defs, &mut handlers);
        let _ = register_travel_actions(&mut defs, &mut handlers);

        Self {
            world,
            event_log: EventLog::new(),
            scheduler: Scheduler::new(SystemManifest::canonical()),
            controller: ControllerState::with_entity(actor),
            rng: DeterministicRng::new(Seed([7; 32])),
            defs,
            handlers,
            actor,
            orchard_place,
            bridge_place,
            bakery_place,
            orchard_workstation,
            mill_workstation,
        }
    }

    fn step_once(&mut self) -> Result<TickStepResult, TickStepError> {
        step_tick(
            &mut self.world,
            &mut self.event_log,
            &mut self.scheduler,
            &mut self.controller,
            &mut self.rng,
            TickStepServices {
                action_defs: &self.defs,
                action_handlers: &self.handlers,
                systems: &dispatch_table(),
            },
        )
    }

    fn queue_action(&mut self, name: &str, targets: Vec<worldwake_core::EntityId>) {
        let tick = self.scheduler.current_tick();
        let def_id = self.action_def_id(name);
        self.scheduler.input_queue_mut().enqueue(
            tick,
            InputKind::RequestAction {
                actor: self.actor,
                def_id,
                targets,
                payload_override: None,
            },
        );
    }

    fn run_queued_action_to_completion(&mut self, max_ticks: u32) {
        let mut completed = false;
        for _ in 0..max_ticks {
            let result = self.step_once().unwrap();
            completed |= result.actions_completed > 0;
            if completed && self.scheduler.active_actions().is_empty() {
                return;
            }
        }

        panic!("queued action did not complete within {max_ticks} ticks");
    }

    fn action_def_id(&self, name: &str) -> ActionDefId {
        self.defs
            .iter()
            .find(|def| def.name == name)
            .map_or_else(|| panic!("missing action def {name}"), |def| def.id)
    }

    fn affordances_for(&self, name: &str) -> Vec<Vec<worldwake_core::EntityId>> {
        let def_id = self.action_def_id(name);
        get_affordances(
            &worldwake_sim::OmniscientBeliefView::new(&self.world),
            self.actor,
            &self.defs,
        )
        .into_iter()
        .filter(|affordance| affordance.def_id == def_id)
        .map(|affordance| affordance.bound_targets)
        .collect()
    }
}

fn add_controlled_lot(
    harness: &mut Harness,
    commodity: CommodityKind,
    quantity: u32,
) -> worldwake_core::EntityId {
    let mut txn = new_txn(&mut harness.world, 2);
    let lot = txn.create_item_lot(commodity, Quantity(quantity)).unwrap();
    txn.set_ground_location(lot, harness.orchard_place).unwrap();
    txn.set_possessor(lot, harness.actor).unwrap();
    commit_txn(txn);
    lot
}

fn apple_material_total(world: &World, source: worldwake_core::EntityId) -> u64 {
    assert!(world.get_component_resource_source(source).is_some());
    total_authoritative_commodity_quantity(world, CommodityKind::Apple)
}

fn apple_lot_at_place(world: &World, place: worldwake_core::EntityId) -> worldwake_core::EntityId {
    world
        .ground_entities_at(place)
        .into_iter()
        .find(|entity| {
            world
                .get_component_item_lot(*entity)
                .is_some_and(|lot| lot.commodity == CommodityKind::Apple)
        })
        .unwrap_or_else(|| panic!("missing apple lot at place {place}"))
}

fn assert_transit_invariants(world: &World, entity: worldwake_core::EntityId) {
    if world.is_in_transit(entity) {
        assert_eq!(world.effective_place(entity), None);
    }
}

#[allow(clippy::too_many_lines)]
#[test]
fn scheduler_multi_step_transport_preserves_stock_accounting_and_route_occupancy() {
    let mut harness = Harness::new(ResourceSource {
        commodity: CommodityKind::Apple,
        available_quantity: Quantity(4),
        max_quantity: Quantity(4),
        regeneration_ticks_per_unit: None,
        last_regeneration_tick: None,
    });

    assert_eq!(
        apple_material_total(&harness.world, harness.orchard_workstation),
        4
    );

    harness.queue_action("harvest:Harvest Apples", vec![harness.orchard_workstation]);
    harness.run_queued_action_to_completion(4);

    assert_eq!(
        apple_material_total(&harness.world, harness.orchard_workstation),
        4
    );
    assert_eq!(
        harness
            .world
            .get_component_resource_source(harness.orchard_workstation)
            .unwrap()
            .available_quantity,
        Quantity(2)
    );

    let apples = apple_lot_at_place(&harness.world, harness.orchard_place);
    harness.queue_action("pick_up", vec![apples]);
    harness.run_queued_action_to_completion(2);

    assert_eq!(harness.world.possessor_of(apples), Some(harness.actor));
    assert_eq!(
        harness.world.effective_place(apples),
        Some(harness.orchard_place)
    );
    assert_eq!(
        apple_material_total(&harness.world, harness.orchard_workstation),
        4
    );
    assert_transit_invariants(&harness.world, harness.actor);
    assert_transit_invariants(&harness.world, apples);

    harness.queue_action("travel", vec![harness.bridge_place]);
    let _ = harness.step_once().unwrap();

    assert!(harness.world.is_in_transit(harness.actor));
    assert!(harness.world.is_in_transit(apples));
    assert_eq!(harness.world.get_component_in_transit_on_edge(apples), None);
    let first_leg = harness
        .world
        .get_component_in_transit_on_edge(harness.actor)
        .unwrap();
    assert_eq!(
        (first_leg.edge_id, first_leg.origin, first_leg.destination),
        (
            TravelEdgeId(10),
            harness.orchard_place,
            harness.bridge_place
        )
    );
    assert_eq!(first_leg.arrival_tick.0 - first_leg.departure_tick.0, 2);
    assert_eq!(
        apple_material_total(&harness.world, harness.orchard_workstation),
        4
    );
    assert_transit_invariants(&harness.world, harness.actor);
    assert_transit_invariants(&harness.world, apples);

    harness.run_queued_action_to_completion(3);
    assert_eq!(
        harness.world.effective_place(harness.actor),
        Some(harness.bridge_place)
    );
    assert_eq!(
        harness.world.effective_place(apples),
        Some(harness.bridge_place)
    );

    harness.queue_action("travel", vec![harness.bakery_place]);
    let _ = harness.step_once().unwrap();
    assert!(harness.world.is_in_transit(harness.actor));
    assert!(harness.world.is_in_transit(apples));
    let second_leg = harness
        .world
        .get_component_in_transit_on_edge(harness.actor)
        .unwrap();
    assert_eq!(
        (
            second_leg.edge_id,
            second_leg.origin,
            second_leg.destination
        ),
        (TravelEdgeId(12), harness.bridge_place, harness.bakery_place)
    );
    assert_eq!(second_leg.arrival_tick.0 - second_leg.departure_tick.0, 2);
    assert_eq!(
        apple_material_total(&harness.world, harness.orchard_workstation),
        4
    );

    harness.run_queued_action_to_completion(3);
    assert_eq!(
        harness.world.effective_place(harness.actor),
        Some(harness.bakery_place)
    );
    assert_eq!(
        harness.world.effective_place(apples),
        Some(harness.bakery_place)
    );

    harness.queue_action("put_down", vec![apples]);
    harness.run_queued_action_to_completion(2);

    assert_eq!(harness.world.possessor_of(apples), None);
    assert_eq!(
        harness.world.effective_place(apples),
        Some(harness.bakery_place)
    );
    assert_eq!(
        apple_material_total(&harness.world, harness.orchard_workstation),
        4
    );
}

#[test]
fn scheduler_harvest_depletion_and_regeneration_gate_affordances_on_concrete_stock() {
    let mut harness = Harness::new(ResourceSource {
        commodity: CommodityKind::Apple,
        available_quantity: Quantity(2),
        max_quantity: Quantity(2),
        regeneration_ticks_per_unit: Some(nz(2)),
        last_regeneration_tick: Some(Tick(0)),
    });

    harness.queue_action("harvest:Harvest Apples", vec![harness.orchard_workstation]);
    harness.run_queued_action_to_completion(4);

    assert_eq!(
        harness
            .world
            .get_component_resource_source(harness.orchard_workstation)
            .unwrap()
            .available_quantity,
        Quantity(0)
    );
    assert_eq!(
        apple_material_total(&harness.world, harness.orchard_workstation),
        2
    );
    assert!(harness.affordances_for("harvest:Harvest Apples").is_empty());

    harness.step_once().unwrap();
    assert_eq!(
        harness
            .world
            .get_component_resource_source(harness.orchard_workstation)
            .unwrap()
            .available_quantity,
        Quantity(1)
    );
    assert_eq!(
        apple_material_total(&harness.world, harness.orchard_workstation),
        3
    );
    assert!(harness.affordances_for("harvest:Harvest Apples").is_empty());

    harness.step_once().unwrap();
    assert_eq!(
        apple_material_total(&harness.world, harness.orchard_workstation),
        3
    );
    assert!(harness.affordances_for("harvest:Harvest Apples").is_empty());

    harness.step_once().unwrap();
    assert_eq!(
        harness
            .world
            .get_component_resource_source(harness.orchard_workstation)
            .unwrap()
            .available_quantity,
        Quantity(2)
    );
    assert_eq!(
        apple_material_total(&harness.world, harness.orchard_workstation),
        4
    );
    assert_eq!(
        harness.affordances_for("harvest:Harvest Apples"),
        vec![vec![harness.orchard_workstation]]
    );

    harness.queue_action("harvest:Harvest Apples", vec![harness.orchard_workstation]);
    harness.run_queued_action_to_completion(4);
    assert_eq!(
        harness
            .world
            .get_component_resource_source(harness.orchard_workstation)
            .unwrap()
            .available_quantity,
        Quantity(1)
    );
    assert_eq!(
        apple_material_total(&harness.world, harness.orchard_workstation),
        5
    );
    assert!(harness.affordances_for("harvest:Harvest Apples").is_empty());
}

#[test]
fn scheduler_craft_preserves_staged_inputs_and_applies_exact_recipe_deltas() {
    let mut harness = Harness::new(ResourceSource {
        commodity: CommodityKind::Apple,
        available_quantity: Quantity(0),
        max_quantity: Quantity(0),
        regeneration_ticks_per_unit: None,
        last_regeneration_tick: None,
    });
    let grain = add_controlled_lot(&mut harness, CommodityKind::Grain, 2);

    verify_live_lot_conservation(&harness.world, CommodityKind::Grain, 2).unwrap();
    verify_live_lot_conservation(&harness.world, CommodityKind::Bread, 0).unwrap();

    harness.queue_action("craft:Bake Bread", vec![harness.mill_workstation]);
    let _ = harness.step_once().unwrap();

    let job = harness
        .world
        .get_component_production_job(harness.mill_workstation)
        .unwrap()
        .clone();
    assert_eq!(job.worker, harness.actor);
    verify_live_lot_conservation(&harness.world, CommodityKind::Grain, 2).unwrap();
    verify_live_lot_conservation(&harness.world, CommodityKind::Bread, 0).unwrap();
    assert!(harness
        .world
        .recursive_contents_of(job.staged_inputs_container)
        .into_iter()
        .any(|entity| entity == grain));

    harness.run_queued_action_to_completion(3);

    assert!(harness
        .world
        .get_component_production_job(harness.mill_workstation)
        .is_none());
    verify_live_lot_conservation(&harness.world, CommodityKind::Grain, 0).unwrap();
    verify_live_lot_conservation(&harness.world, CommodityKind::Bread, 1).unwrap();
}
