use std::num::NonZeroU32;

use worldwake_core::{
    build_prototype_world, CauseRef, CommodityKind, ComponentKind, Container, ControlSource,
    DeprivationExposure, DeprivationKind, DriveThresholds, EventLog, HomeostaticNeeds, LoadUnits,
    MetabolismProfile, Permille, Quantity, Seed, Tick, VisibilitySpec, WitnessData, World,
    WorldTxn, WoundCause,
};
use worldwake_sim::{
    ActionDefId, ActionDefRegistry, ActionHandlerRegistry, ControllerState, DeterministicRng,
    InputKind, Scheduler, SystemManifest, TickStepError, TickStepServices, step_tick,
};
use worldwake_systems::{dispatch_table, register_needs_actions};

fn pm(value: u16) -> Permille {
    Permille::new(value).unwrap()
}

fn nz(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).unwrap()
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
    place: worldwake_core::EntityId,
}

impl Harness {
    fn new(needs: HomeostaticNeeds, profile: MetabolismProfile) -> Self {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_component_homeostatic_needs(actor, needs).unwrap();
            txn.set_component_deprivation_exposure(actor, DeprivationExposure::default())
                .unwrap();
            txn.set_component_drive_thresholds(actor, DriveThresholds::default())
                .unwrap();
            txn.set_component_metabolism_profile(actor, profile).unwrap();
            commit_txn(txn);
            actor
        };

        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        register_needs_actions(&mut defs, &mut handlers);

        Self {
            world,
            event_log: EventLog::new(),
            scheduler: Scheduler::new(SystemManifest::canonical()),
            controller: ControllerState::with_entity(actor),
            rng: DeterministicRng::new(Seed([7; 32])),
            defs,
            handlers,
            actor,
            place,
        }
    }

    fn step_once(&mut self) -> Result<worldwake_sim::TickStepResult, TickStepError> {
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

    fn step_ticks(&mut self, count: u32) {
        for _ in 0..count {
            self.step_once().unwrap();
        }
    }

    fn queue_action(&mut self, name: &str, targets: Vec<worldwake_core::EntityId>) {
        let def_id = action_def_id(&self.defs, name);
        let tick = self.scheduler.current_tick();
        self.scheduler.input_queue_mut().enqueue(
            tick,
            InputKind::RequestAction {
                actor: self.actor,
                def_id,
                targets,
            },
        );
    }

    fn run_queued_action_to_completion(&mut self, max_ticks: u32) {
        let mut committed = false;
        for _ in 0..max_ticks {
            let result = self.step_once().unwrap();
            committed |= result.actions_completed > 0;
            if committed && self.scheduler.active_actions().is_empty() {
                return;
            }
        }

        panic!("queued action did not complete within {max_ticks} ticks");
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
    let mut log = EventLog::new();
    let _ = txn.commit(&mut log);
}

fn action_def_id(defs: &ActionDefRegistry, name: &str) -> ActionDefId {
    defs.iter()
        .find(|def| def.name == name)
        .map(|def| def.id)
        .unwrap_or_else(|| panic!("missing action def {name}"))
}

fn add_controlled_lot(
    harness: &mut Harness,
    commodity: CommodityKind,
    quantity: u32,
) -> worldwake_core::EntityId {
    let mut txn = new_txn(&mut harness.world, 2);
    let lot = txn.create_item_lot(commodity, Quantity(quantity)).unwrap();
    txn.set_ground_location(lot, harness.place).unwrap();
    txn.set_possessor(lot, harness.actor).unwrap();
    commit_txn(txn);
    lot
}

fn add_controlled_bread_in_satchel(harness: &mut Harness, quantity: u32) -> worldwake_core::EntityId {
    let mut txn = new_txn(&mut harness.world, 2);
    let bread = txn.create_item_lot(CommodityKind::Bread, Quantity(quantity)).unwrap();
    let satchel = txn
        .create_container(Container {
            capacity: LoadUnits(20),
            allowed_commodities: None,
            allows_unique_items: true,
            allows_nested_containers: true,
        })
        .unwrap();
    txn.set_ground_location(satchel, harness.place).unwrap();
    txn.set_possessor(satchel, harness.actor).unwrap();
    txn.put_into_container(bread, satchel).unwrap();
    commit_txn(txn);
    bread
}

fn metabolism_profile(
    hunger_rate: u16,
    thirst_rate: u16,
    fatigue_rate: u16,
    bladder_rate: u16,
    dirtiness_rate: u16,
    rest_efficiency: u16,
    starvation_tolerance_ticks: u32,
    dehydration_tolerance_ticks: u32,
    exhaustion_collapse_ticks: u32,
    bladder_accident_tolerance_ticks: u32,
    toilet_ticks: u32,
    wash_ticks: u32,
) -> MetabolismProfile {
    MetabolismProfile::new(
        pm(hunger_rate),
        pm(thirst_rate),
        pm(fatigue_rate),
        pm(bladder_rate),
        pm(dirtiness_rate),
        pm(rest_efficiency),
        nz(starvation_tolerance_ticks),
        nz(dehydration_tolerance_ticks),
        nz(exhaustion_collapse_ticks),
        nz(bladder_accident_tolerance_ticks),
        nz(toilet_ticks),
        nz(wash_ticks),
    )
}

fn run_metabolism_progression_scenario() -> HomeostaticNeeds {
    let mut harness = Harness::new(
        HomeostaticNeeds::new_sated(),
        metabolism_profile(2, 3, 4, 5, 6, 20, 10, 10, 10, 10, 2, 3),
    );
    harness.step_ticks(3);
    *harness
        .world
        .get_component_homeostatic_needs(harness.actor)
        .unwrap()
}

#[test]
fn scheduler_progresses_metabolism_deterministically_without_inputs() {
    let expected = HomeostaticNeeds::new(pm(6), pm(9), pm(12), pm(15), pm(18));

    assert_eq!(run_metabolism_progression_scenario(), expected);
    assert_eq!(run_metabolism_progression_scenario(), expected);
}

#[test]
fn scheduler_driven_care_actions_apply_effects_and_preserve_conservation() {
    let profile = metabolism_profile(0, 0, 0, 0, 0, 40, 20, 20, 20, 20, 2, 3);
    let mut harness = Harness::new(
        HomeostaticNeeds::new(pm(700), pm(650), pm(400), pm(200), pm(350)),
        profile,
    );
    let bread = add_controlled_bread_in_satchel(&mut harness, 2);
    let water = add_controlled_lot(&mut harness, CommodityKind::Water, 3);

    harness.queue_action("eat", vec![bread]);
    harness.run_queued_action_to_completion(10);

    let bread_profile = CommodityKind::Bread.spec().consumable_profile.unwrap();
    let needs_after_eat = harness
        .world
        .get_component_homeostatic_needs(harness.actor)
        .unwrap();
    assert_eq!(harness.world.get_component_item_lot(bread).unwrap().quantity, Quantity(1));
    assert_eq!(
        *needs_after_eat,
        HomeostaticNeeds::new(
            pm(700).saturating_sub(bread_profile.hunger_relief_per_unit),
            pm(650).saturating_sub(bread_profile.thirst_relief_per_unit),
            pm(400),
            pm(200).saturating_add(bread_profile.bladder_fill_per_unit),
            pm(350),
        )
    );

    harness.queue_action("drink", vec![water]);
    harness.run_queued_action_to_completion(10);

    let water_profile = CommodityKind::Water.spec().consumable_profile.unwrap();
    let needs_after_drink = harness
        .world
        .get_component_homeostatic_needs(harness.actor)
        .unwrap();
    assert_eq!(harness.world.get_component_item_lot(water).unwrap().quantity, Quantity(2));
    assert_eq!(
        *needs_after_drink,
        HomeostaticNeeds::new(
            pm(700)
                .saturating_sub(bread_profile.hunger_relief_per_unit)
                .saturating_sub(water_profile.hunger_relief_per_unit),
            pm(650)
                .saturating_sub(bread_profile.thirst_relief_per_unit)
                .saturating_sub(water_profile.thirst_relief_per_unit),
            pm(400),
            pm(200)
                .saturating_add(bread_profile.bladder_fill_per_unit)
                .saturating_add(water_profile.bladder_fill_per_unit),
            pm(350),
        )
    );

    harness.queue_action("sleep", Vec::new());
    harness.run_queued_action_to_completion(2);
    assert_eq!(
        harness
            .world
            .get_component_homeostatic_needs(harness.actor)
            .unwrap()
            .fatigue,
        pm(400).saturating_sub(profile.rest_efficiency)
    );

    harness.queue_action("toilet", Vec::new());
    harness.run_queued_action_to_completion(5);
    assert_eq!(
        harness
            .world
            .get_component_homeostatic_needs(harness.actor)
            .unwrap()
            .bladder,
        pm(0)
    );
    let waste_count = harness
        .world
        .ground_entities_at(harness.place)
        .into_iter()
        .filter(|entity| {
            harness
                .world
                .get_component_item_lot(*entity)
                .is_some_and(|lot| lot.commodity == CommodityKind::Waste)
        })
        .count();
    assert_eq!(waste_count, 1);

    harness.queue_action("wash", vec![water]);
    harness.run_queued_action_to_completion(5);
    assert_eq!(harness.world.get_component_item_lot(water).unwrap().quantity, Quantity(1));
    assert_eq!(
        harness
            .world
            .get_component_homeostatic_needs(harness.actor)
            .unwrap()
            .dirtiness,
        pm(0)
    );
}

#[test]
fn scheduler_rejects_eat_request_for_uncontrolled_ground_item() {
    let mut harness = Harness::new(
        HomeostaticNeeds::new(pm(700), pm(650), pm(400), pm(200), pm(350)),
        metabolism_profile(0, 0, 0, 0, 0, 40, 20, 20, 20, 20, 2, 3),
    );
    let bread = {
        let mut txn = new_txn(&mut harness.world, 2);
        let bread = txn.create_item_lot(CommodityKind::Bread, Quantity(1)).unwrap();
        txn.set_ground_location(bread, harness.place).unwrap();
        commit_txn(txn);
        bread
    };

    harness.queue_action("eat", vec![bread]);

    let error = harness.step_once().unwrap_err();
    assert_eq!(
        error,
        TickStepError::RequestedAffordanceUnavailable {
            actor: harness.actor,
            def_id: action_def_id(&harness.defs, "eat"),
            targets: vec![bread],
        }
    );
}

#[test]
fn scheduler_applies_starvation_and_dehydration_consequences_after_tolerance_windows() {
    let thresholds = DriveThresholds::default();
    let mut harness = Harness::new(
        HomeostaticNeeds::new(
            thresholds.hunger.critical(),
            thresholds.thirst.critical(),
            pm(0),
            pm(0),
            pm(0),
        ),
        metabolism_profile(0, 0, 0, 0, 0, 20, 1, 1, 10, 10, 2, 3),
    );

    harness.step_once().unwrap();

    let wounds = harness.world.get_component_wound_list(harness.actor).unwrap();
    assert_eq!(wounds.wounds.len(), 2);
    assert_eq!(
        wounds.wounds[0].cause,
        WoundCause::Deprivation(DeprivationKind::Starvation)
    );
    assert_eq!(
        wounds.wounds[1].cause,
        WoundCause::Deprivation(DeprivationKind::Dehydration)
    );
}

#[test]
fn divergent_metabolism_profiles_produce_divergent_scheduler_outcomes() {
    let mut slow = Harness::new(
        HomeostaticNeeds::new_sated(),
        metabolism_profile(1, 1, 1, 1, 1, 20, 10, 10, 10, 10, 2, 3),
    );
    let mut fast = Harness::new(
        HomeostaticNeeds::new_sated(),
        metabolism_profile(4, 5, 6, 7, 8, 20, 10, 10, 10, 10, 2, 3),
    );

    slow.step_ticks(4);
    fast.step_ticks(4);

    assert_ne!(
        slow.world.get_component_homeostatic_needs(slow.actor),
        fast.world.get_component_homeostatic_needs(fast.actor)
    );
    assert_eq!(
        slow.world.get_component_homeostatic_needs(slow.actor),
        Some(&HomeostaticNeeds::new(pm(4), pm(4), pm(4), pm(4), pm(4)))
    );
    assert_eq!(
        fast.world.get_component_homeostatic_needs(fast.actor),
        Some(&HomeostaticNeeds::new(pm(16), pm(20), pm(24), pm(28), pm(32)))
    );
}

#[test]
fn authoritative_schema_has_only_expected_e09_components_and_fields() {
    let component_kinds = ComponentKind::ALL;
    assert_eq!(
        component_kinds,
        [
            ComponentKind::Name,
            ComponentKind::AgentData,
            ComponentKind::WoundList,
            ComponentKind::DriveThresholds,
            ComponentKind::HomeostaticNeeds,
            ComponentKind::DeprivationExposure,
            ComponentKind::MetabolismProfile,
            ComponentKind::ItemLot,
            ComponentKind::UniqueItem,
            ComponentKind::Container,
        ]
    );

    let HomeostaticNeeds {
        hunger,
        thirst,
        fatigue,
        bladder,
        dirtiness,
    } = HomeostaticNeeds::default();
    let _ = (hunger, thirst, fatigue, bladder, dirtiness);

    let DeprivationExposure {
        hunger_critical_ticks,
        thirst_critical_ticks,
        fatigue_critical_ticks,
        bladder_critical_ticks,
    } = DeprivationExposure::default();
    let _ = (
        hunger_critical_ticks,
        thirst_critical_ticks,
        fatigue_critical_ticks,
        bladder_critical_ticks,
    );

    let MetabolismProfile {
        hunger_rate,
        thirst_rate,
        fatigue_rate,
        bladder_rate,
        dirtiness_rate,
        rest_efficiency,
        starvation_tolerance_ticks,
        dehydration_tolerance_ticks,
        exhaustion_collapse_ticks,
        bladder_accident_tolerance_ticks,
        toilet_ticks,
        wash_ticks,
    } = MetabolismProfile::default();
    let _ = (
        hunger_rate,
        thirst_rate,
        fatigue_rate,
        bladder_rate,
        dirtiness_rate,
        rest_efficiency,
        starvation_tolerance_ticks,
        dehydration_tolerance_ticks,
        exhaustion_collapse_ticks,
        bladder_accident_tolerance_ticks,
        toilet_ticks,
        wash_ticks,
    );
}
