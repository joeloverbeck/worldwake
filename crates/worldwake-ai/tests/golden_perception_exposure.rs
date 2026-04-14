//! Golden tests for S56 context-modulated witnessed-event perception exposure.
//!
//! These tests exercise the full witnessed-event path through real action
//! requests, place concealment, fatigue, and active-action attention cost,
//! then prove the resulting modulation at the strongest live golden surface:
//! `PerceptionTraceEvent.effective_fidelity`.

mod golden_harness;

use golden_harness::*;
use worldwake_core::{
    AgentData, CommodityKind, ControlSource, EntityId, EventTag, EventView, HomeostaticNeeds,
    MetabolismProfile, PerceptionProfile, PlaceVisibilityProfile, PrototypePlace, Quantity,
    ResourceSource, Seed, WorkstationTag, prototype_place_entity,
};
use worldwake_sim::{
    ActionRequestMode, ActionTraceKind, InputKind, PerceptionTraceEvent, RequestProvenance,
};

const FOREST_PATH: worldwake_core::EntityId = prototype_place_entity(PrototypePlace::ForestPath);

fn set_control_source(
    h: &mut GoldenHarness,
    agent: worldwake_core::EntityId,
    control_source: ControlSource,
    tick: u64,
) {
    let mut txn = new_txn(&mut h.world, tick);
    txn.set_component_agent_data(agent, AgentData { control_source })
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn request_simple_action(
    h: &mut GoldenHarness,
    actor: worldwake_core::EntityId,
    def_name: &str,
    targets: Vec<worldwake_core::EntityId>,
) {
    let def_id = h.defs.iter().find(|def| def.name == def_name).map_or_else(
        || panic!("full registries should include {def_name}"),
        |def| def.id,
    );
    let tick = h.scheduler.current_tick();
    let _ = h.scheduler.input_queue_mut().enqueue(
        tick,
        InputKind::RequestAction {
            actor,
            def_id,
            targets,
            payload_override: None,
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );
}

fn set_place_visibility_profile(
    h: &mut GoldenHarness,
    place: worldwake_core::EntityId,
    concealment: u16,
) {
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_place_visibility_profile(
        place,
        PlaceVisibilityProfile {
            base_concealment: pm(concealment),
        },
    )
    .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn witnessed_wilderness_relief_trace(
    h: &GoldenHarness,
    observer: worldwake_core::EntityId,
) -> Option<&PerceptionTraceEvent> {
    let sink = h
        .perception_trace_sink()
        .expect("perception tracing should be enabled");
    sink.events_for(observer).into_iter().find(|trace_event| {
        h.event_log
            .get(trace_event.event_id)
            .is_some_and(|record| record.tags().contains(&EventTag::WildernessRelief))
    })
}

fn stable_test_metabolism() -> MetabolismProfile {
    MetabolismProfile::new(
        pm(0),
        pm(0),
        pm(0),
        pm(0),
        pm(0),
        pm(20),
        nz(480),
        nz(240),
        nz(120),
        nz(200),
        nz(8),
        nz(12),
        pm(0),
        pm(0),
        pm(0),
        pm(0),
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResourceSourceBeliefObservation {
    known_entities: Vec<EntityId>,
    retained_waste_entities: Vec<EntityId>,
    apple_source_believed: bool,
    water_source_believed: bool,
}

fn place_ground_lots(
    h: &mut GoldenHarness,
    place: EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
    count: usize,
) -> Vec<EntityId> {
    let mut txn = new_txn(&mut h.world, 0);
    let mut lots = Vec::with_capacity(count);
    for _ in 0..count {
        let lot = txn.create_item_lot(commodity, quantity).unwrap();
        txn.set_ground_location(lot, place).unwrap();
        lots.push(lot);
    }
    commit_txn(txn, &mut h.event_log);
    lots
}

fn run_perception_forms_resource_source_beliefs(seed: Seed) -> ResourceSourceBeliefObservation {
    let mut h = GoldenHarness::new(seed);

    let apple_source = place_workstation_with_source(
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
        ProductionOutputOwner::Actor,
    );
    let water_source = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::Well,
        ResourceSource {
            commodity: CommodityKind::Water,
            available_quantity: Quantity(4),
            max_quantity: Quantity(4),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );
    let waste_lots = place_ground_lots(&mut h, ORCHARD_FARM, CommodityKind::Waste, Quantity(1), 6);

    let observer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Observer",
        ORCHARD_FARM,
        HomeostaticNeeds::new_sated(),
        stable_test_metabolism(),
        worldwake_core::UtilityProfile::default(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        observer,
        PerceptionProfile {
            entity_activation_threshold: pm(125),
            claim_confidence_threshold: pm(50),
            observation_buffer_capacity: 4,
            need_salience_boost: pm(500),
            need_salience_urgency_threshold: pm(500),
            observation_fidelity: pm(1000),
            ..PerceptionProfile::default()
        },
    );

    for _ in 0..50 {
        h.step_once();
    }

    let store = h
        .world
        .get_component_agent_belief_store(observer)
        .cloned()
        .unwrap_or_else(worldwake_core::AgentBeliefStore::new);
    let known_entities = store.known_entities.keys().copied().collect::<Vec<_>>();
    let retained_waste_entities = waste_lots
        .iter()
        .copied()
        .filter(|entity| store.known_entities.contains_key(entity))
        .collect::<Vec<_>>();

    ResourceSourceBeliefObservation {
        apple_source_believed: store.known_entities.contains_key(&apple_source),
        water_source_believed: store.known_entities.contains_key(&water_source),
        known_entities,
        retained_waste_entities,
    }
}

fn run_witnessed_relief_trace(
    seed: Seed,
    observer_base_fidelity: u16,
    observer_needs: HomeostaticNeeds,
    concealment: Option<u16>,
    observer_starts_defend: bool,
) -> (GoldenHarness, worldwake_core::EntityId) {
    let mut h = GoldenHarness::new(seed);
    h.enable_action_tracing();
    h.enable_perception_tracing();

    if let Some(concealment) = concealment {
        set_place_visibility_profile(&mut h, FOREST_PATH, concealment);
    }

    let observer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Observer",
        FOREST_PATH,
        observer_needs,
        stable_test_metabolism(),
        worldwake_core::UtilityProfile::default(),
    );
    let actor = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Reliever",
        FOREST_PATH,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(950), pm(0)),
        stable_test_metabolism(),
        worldwake_core::UtilityProfile::default(),
    );

    set_control_source(&mut h, observer, ControlSource::Human, 0);
    set_control_source(&mut h, actor, ControlSource::Human, 0);
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        observer,
        PerceptionProfile {
            observation_fidelity: pm(observer_base_fidelity),
            ..PerceptionProfile::default()
        },
    );

    if observer_starts_defend {
        request_simple_action(&mut h, observer, "defend", vec![]);
        h.step_once();

        let defend_started = h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(observer)
            .iter()
            .any(|event| {
                event.action_name == "defend"
                    && matches!(event.kind, ActionTraceKind::Started { .. })
            });
        assert!(
            defend_started,
            "observer should have started defend before the witnessed event"
        );
    }

    request_simple_action(&mut h, actor, "relieve_wilderness", vec![]);
    for _ in 0..8 {
        h.step_once();
        if witnessed_wilderness_relief_trace(&h, observer).is_some() {
            break;
        }
    }

    assert!(
        witnessed_wilderness_relief_trace(&h, observer).is_some(),
        "observer should have a perception trace entry for the witnessed wilderness relief event"
    );

    (h, observer)
}

// Scenario 116: Concealment Reduces Witnessed-Event Fidelity
//
// Systems: Needs, Perception
// ActionDomains: Needs
// Places: ForestPath
// Principles: 7, 15, 16
//
// Setup: A human-controlled reliever triggers `relieve_wilderness` at ForestPath while a human-controlled observer with `observation_fidelity = 800` witnesses the event once in the open baseline and once with `PlaceVisibilityProfile(400)`.
//
// Proves: Place concealment lowers witnessed-event `effective_fidelity` from `800` to `480` in the real perception pipeline.
//
// Chain: same-place action request -> wilderness relief event -> perception witness resolution -> concealment modulation -> perception trace.
#[test]
fn golden_concealment_reduces_witnessed_event_fidelity() {
    let (open_harness, open_observer) = run_witnessed_relief_trace(
        Seed([116; 32]),
        800,
        HomeostaticNeeds::new_sated(),
        None,
        false,
    );
    let (concealed_harness, concealed_observer) = run_witnessed_relief_trace(
        Seed([116; 32]),
        800,
        HomeostaticNeeds::new_sated(),
        Some(400),
        false,
    );

    let open_trace = witnessed_wilderness_relief_trace(&open_harness, open_observer).unwrap();
    let concealed_trace =
        witnessed_wilderness_relief_trace(&concealed_harness, concealed_observer).unwrap();

    assert_eq!(open_trace.effective_fidelity, 800);
    assert_eq!(concealed_trace.effective_fidelity, 480);
    assert!(
        concealed_trace.effective_fidelity < open_trace.effective_fidelity,
        "concealment should reduce witnessed-event fidelity"
    );
}

// Scenario 117: Fatigue Reduces Witnessed-Event Fidelity
//
// Systems: Needs, Perception
// ActionDomains: Needs
// Places: ForestPath
// Principles: 2, 15, 16
//
// Setup: A human-controlled reliever triggers `relieve_wilderness` while a human-controlled observer with `observation_fidelity = 1000` witnesses the event once while rested and once with `fatigue = 800`.
//
// Proves: Observer fatigue reduces witnessed-event `effective_fidelity` from `1000` to `820` in the real perception pipeline.
//
// Chain: same-place action request -> wilderness relief event -> perception witness resolution -> fatigue modulation -> perception trace.
#[test]
fn golden_fatigue_reduces_witnessed_event_fidelity() {
    let (rested_harness, rested_observer) = run_witnessed_relief_trace(
        Seed([117; 32]),
        1000,
        HomeostaticNeeds::new_sated(),
        None,
        false,
    );
    let (fatigued_harness, fatigued_observer) = run_witnessed_relief_trace(
        Seed([117; 32]),
        1000,
        HomeostaticNeeds::new(pm(0), pm(0), pm(800), pm(0), pm(0)),
        None,
        false,
    );

    let rested_trace = witnessed_wilderness_relief_trace(&rested_harness, rested_observer).unwrap();
    let fatigued_trace =
        witnessed_wilderness_relief_trace(&fatigued_harness, fatigued_observer).unwrap();

    assert_eq!(rested_trace.effective_fidelity, 1000);
    assert_eq!(fatigued_trace.effective_fidelity, 820);
    assert!(
        fatigued_trace.effective_fidelity < rested_trace.effective_fidelity,
        "fatigue should reduce witnessed-event fidelity"
    );
}

// Scenario 118: Attention Cost Reduces Witnessed-Event Fidelity
//
// Systems: Combat, Needs, Perception
// ActionDomains: Combat, Needs
// Places: ForestPath
// Principles: 8, 15, 16, 26
//
// Setup: A human-controlled observer starts `defend` (attention cost `400`) before a co-located human-controlled reliever triggers `relieve_wilderness`, with baseline fidelity `1000`.
//
// Proves: An active combat action lawfully lowers the observer's witnessed-event `effective_fidelity` to `600`, and the scenario proves the active action through the real action trace rather than direct state editing.
//
// Chain: external defend request -> defend start -> same-place wilderness relief event -> perception witness resolution -> attention-cost modulation -> trace.
#[test]
fn golden_attention_cost_reduces_witnessed_event_fidelity() {
    let (harness, observer) = run_witnessed_relief_trace(
        Seed([118; 32]),
        1000,
        HomeostaticNeeds::new_sated(),
        None,
        true,
    );

    let trace = witnessed_wilderness_relief_trace(&harness, observer).unwrap();
    assert_eq!(trace.effective_fidelity, 600);
}

// Scenario 119: Multiplicative Stacking
//
// Systems: Combat, Needs, Perception
// ActionDomains: Combat, Needs
// Places: ForestPath
// Principles: 2, 8, 11, 15, 16, 26
//
// Setup: A human-controlled observer with `observation_fidelity = 800`, `fatigue = 700`, active `defend` (`attention_cost = 400`), and `PlaceVisibilityProfile(400)` witnesses a co-located `relieve_wilderness` event.
//
// Proves: The full witnessed-event modulation chain composes multiplicatively to `effective_fidelity = 253`.
//
// Chain: defend start + fatigued observer + concealed place -> witnessed event -> perception witness resolution -> multiplicative modulation -> trace.
#[test]
fn golden_modulation_stacks_multiplicatively_for_witnessed_event_fidelity() {
    let (harness, observer) = run_witnessed_relief_trace(
        Seed([119; 32]),
        800,
        HomeostaticNeeds::new(pm(0), pm(0), pm(700), pm(0), pm(0)),
        Some(400),
        true,
    );

    let trace = witnessed_wilderness_relief_trace(&harness, observer).unwrap();
    assert_eq!(trace.effective_fidelity, 253);
}

// Scenario 128: Perception Forms Beliefs About Resource Sources
//
// Systems: Perception
// ActionDomains: N/A
// Places: OrchardFarm
// Principles: 7, 14, 15, 16
//
// Setup: An AI observer with `observation_buffer_capacity = 4` spends 50 ticks at `OrchardFarm` with two facility-backed resource sources (apple and water) and six competing waste lots on the ground.
//
// Proves: The perception-to-belief pipeline retains both resource source entities in `AgentBeliefStore.known_entities` even when total perceived entities exceed the observation-history buffer width.
//
// Chain: same-place repeated observation -> activation refresh -> retained infrastructure beliefs without a hard entity-count clamp.
#[test]
fn golden_perception_forms_resource_source_beliefs() {
    let observation = run_perception_forms_resource_source_beliefs(Seed([178; 32]));

    assert!(
        observation.apple_source_believed,
        "observer should retain the apple source belief under capacity pressure; observation={observation:?}"
    );
    assert!(
        observation.water_source_believed,
        "observer should retain the water source belief under capacity pressure; observation={observation:?}"
    );
    assert!(
        observation.known_entities.len() > 4,
        "activation-based retention should not clamp known entities to the observation buffer width; observation={observation:?}"
    );
}

#[test]
fn golden_perception_forms_resource_source_beliefs_replays_deterministically() {
    let first = run_perception_forms_resource_source_beliefs(Seed([178; 32]));
    let second = run_perception_forms_resource_source_beliefs(Seed([178; 32]));

    assert_eq!(first, second);
}
