//! Golden tests for S101 activation-based belief decay.

mod golden_harness;

use golden_harness::*;
use worldwake_core::{
    AgentBeliefStore, AgentData, CommodityKind, ControlSource, EntityId, HomeostaticNeeds,
    MetabolismProfile, PerceptionProfile, PerceptionSource, Quantity, ResourceSource, Seed, Tick,
    UtilityProfile, WorkstationTag,
};

fn inert_metabolism() -> MetabolismProfile {
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

fn activation_profile() -> PerceptionProfile {
    PerceptionProfile {
        entity_activation_threshold: pm(100),
        claim_confidence_threshold: pm(50),
        observation_buffer_capacity: 5,
        need_salience_boost: pm(500),
        need_salience_urgency_threshold: pm(500),
        observation_fidelity: pm(1000),
        ..PerceptionProfile::default()
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
        .expect("golden activation-decay tests should keep control source writable");
    commit_txn(txn, &mut h.event_log);
}

fn place_ground_lot(
    h: &mut GoldenHarness,
    place: EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
) -> EntityId {
    let mut txn = new_txn(&mut h.world, 0);
    let lot = txn
        .create_item_lot(commodity, quantity)
        .expect("golden activation-decay tests should create item lots");
    txn.set_ground_location(lot, place)
        .expect("golden activation-decay tests should place item lots");
    commit_txn(txn, &mut h.event_log);
    lot
}

fn seed_report_claim_from_world(
    h: &mut GoldenHarness,
    agent: EntityId,
    subject: EntityId,
    from: EntityId,
    acquired_tick: Tick,
) {
    let source = PerceptionSource::Report { from, chain_len: 1 };
    let snapshot =
        worldwake_core::build_believed_entity_state(&h.world, subject, acquired_tick, source)
            .expect("claim-pruning scenario should only seed observable snapshots");
    let mut store = h
        .world
        .get_component_agent_belief_store(agent)
        .cloned()
        .unwrap_or_else(AgentBeliefStore::new);
    let prior_summary = store.get_entity(&subject).cloned();
    let profile = h
        .world
        .get_component_perception_profile(agent)
        .copied()
        .unwrap_or_default();
    store.record_entity_snapshot_claims(
        subject,
        &snapshot,
        prior_summary.as_ref(),
        acquired_tick,
        snapshot.last_observed_tick(),
        profile.observation_buffer_capacity,
        &profile.confidence_policy,
    );

    let mut txn = new_txn(&mut h.world, acquired_tick.0);
    txn.set_component_agent_belief_store(agent, store)
        .expect("claim-pruning scenario should keep belief stores writable");
    commit_txn(txn, &mut h.event_log);
}

fn step_ticks(h: &mut GoldenHarness, count: usize) {
    for _ in 0..count {
        h.step_once();
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DecayBoundaryObservation {
    present_at_100: bool,
    present_after_age_101_prune: bool,
}

fn run_decay_boundary_scenario(seed: Seed) -> DecayBoundaryObservation {
    let mut h = GoldenHarness::new(seed);
    let forgotten_apple = place_ground_lot(&mut h, ORCHARD_FARM, CommodityKind::Apple, Quantity(1));
    let observer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Observer",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        inert_metabolism(),
        UtilityProfile::default(),
    );
    set_control_source(&mut h, observer, ControlSource::None, 0);
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        observer,
        activation_profile(),
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        observer,
        forgotten_apple,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    step_ticks(&mut h, 100);
    let present_at_100 = agent_belief_about(&h.world, observer, forgotten_apple).is_some();

    step_ticks(&mut h, 2);
    let present_after_age_101_prune =
        agent_belief_about(&h.world, observer, forgotten_apple).is_some();

    DecayBoundaryObservation {
        present_at_100,
        present_after_age_101_prune,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NeedSalienceObservation {
    item_retained: bool,
    facility_retained: bool,
}

fn run_need_salience_scenario(seed: Seed) -> NeedSalienceObservation {
    let mut h = GoldenHarness::new(seed);
    let remembered_apple =
        place_ground_lot(&mut h, ORCHARD_FARM, CommodityKind::Apple, Quantity(1));
    let orchard = place_workstation_with_source(
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
    let hungry_observer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Hungry Observer",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(750), pm(0), pm(0), pm(0), pm(0)),
        inert_metabolism(),
        UtilityProfile::default(),
    );
    set_control_source(&mut h, hungry_observer, ControlSource::None, 0);
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        hungry_observer,
        activation_profile(),
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        hungry_observer,
        remembered_apple,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        hungry_observer,
        orchard,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    step_ticks(&mut h, 200);

    NeedSalienceObservation {
        item_retained: agent_belief_about(&h.world, hungry_observer, remembered_apple).is_some(),
        facility_retained: agent_belief_about(&h.world, hungry_observer, orchard).is_some(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ClaimConfidenceObservation {
    stale_known: bool,
    stale_claim_count: usize,
    fresh_known: bool,
    fresh_claim_count: usize,
}

fn run_claim_confidence_scenario(seed: Seed) -> ClaimConfidenceObservation {
    let mut h = GoldenHarness::new(seed);
    let informant = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Informant",
        ORCHARD_FARM,
        HomeostaticNeeds::new_sated(),
        inert_metabolism(),
        UtilityProfile::default(),
    );
    let listener = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Listener",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        inert_metabolism(),
        UtilityProfile::default(),
    );
    let stale_source = place_workstation_with_source(
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
    let fresh_source = place_workstation_with_source(
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

    set_control_source(&mut h, informant, ControlSource::None, 0);
    set_control_source(&mut h, listener, ControlSource::None, 0);
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        listener,
        activation_profile(),
    );

    seed_report_claim_from_world(&mut h, listener, stale_source, informant, Tick(0));
    seed_report_claim_from_world(&mut h, listener, fresh_source, informant, Tick(40));

    step_ticks(&mut h, 62);

    let store = h
        .world
        .get_component_agent_belief_store(listener)
        .cloned()
        .unwrap_or_else(AgentBeliefStore::new);

    ClaimConfidenceObservation {
        stale_known: store.known_entities.contains_key(&stale_source),
        stale_claim_count: store.entity_claims.get(&stale_source).map_or(0, Vec::len),
        fresh_known: store.known_entities.contains_key(&fresh_source),
        fresh_claim_count: store.entity_claims.get(&fresh_source).map_or(0, Vec::len),
    }
}

// Scenario 145: Activation Decay Prunes Stale Entities At The Threshold Boundary
//
// Systems: Perception
// ActionDomains: N/A
// Places: VillageSquare, OrchardFarm
// Principles: 3, 7, 15, 16, 18
//
// Setup: An inert observer at `VillageSquare` starts with one direct-observation belief about a
// remote Apple lot at `OrchardFarm`, with the live default `entity_activation_threshold = 100`.
//
// Proves: The activation-decay boundary stays faithful to the landed S101 helper contract: a
// single observation remains present at age `100` ticks and is pruned on the first prune pass
// after age crosses to `101`.
//
// Chain: seeded direct belief -> no re-observation refresh -> threshold-edge retention -> first
// below-threshold tick prunes the belief.
#[test]
fn golden_activation_decay_prunes_stale_entities() {
    let observation = run_decay_boundary_scenario(Seed([145; 32]));

    assert!(
        observation.present_at_100,
        "single-observation beliefs should survive exactly at the age-100 threshold boundary; observation={observation:?}"
    );
    assert!(
        !observation.present_after_age_101_prune,
        "single-observation beliefs should be pruned on the first prune pass after age crosses to 101 ticks; observation={observation:?}"
    );
}

#[test]
fn golden_activation_decay_prunes_stale_entities_replays_deterministically() {
    let first = run_decay_boundary_scenario(Seed([145; 32]));
    let second = run_decay_boundary_scenario(Seed([145; 32]));

    assert_eq!(first, second);
}

// Scenario 146: Need Salience Extends Item Retention But Not Facility Retention
//
// Systems: Perception
// ActionDomains: N/A
// Places: VillageSquare, OrchardFarm
// Principles: 3, 7, 15, 16, 18, 22
//
// Setup: An urgently hungry inert observer at `VillageSquare` starts with two remote
// direct-observation beliefs from the same tick at `OrchardFarm`: one Apple `ItemLot` and one
// apple-source facility.
//
// Proves: The live salience boost extends retention only for the item belief. The facility belief
// receives no salience extension and decays away on the normal activation schedule.
//
// Chain: seeded stale item + facility beliefs -> urgent hunger retained in agent state ->
// item-only salience boost during prune -> item remains while facility is forgotten.
#[test]
fn golden_need_salience_retains_hungry_item_belief() {
    let observation = run_need_salience_scenario(Seed([146; 32]));

    assert!(
        observation.item_retained,
        "urgent need should retain the stale item belief beyond the normal activation window; observation={observation:?}"
    );
    assert!(
        !observation.facility_retained,
        "need salience must not retain stale non-item beliefs; observation={observation:?}"
    );
}

#[test]
fn golden_need_salience_retains_hungry_item_belief_replays_deterministically() {
    let first = run_need_salience_scenario(Seed([146; 32]));
    let second = run_need_salience_scenario(Seed([146; 32]));

    assert_eq!(first, second);
}

// Scenario 147: Claim Confidence Threshold Prunes Stale Reports Before Fresh Reports
//
// Systems: Perception, Social Information
// ActionDomains: N/A
// Places: VillageSquare, OrchardFarm
// Principles: 3, 7, 15, 16, 18
//
// Setup: A listener at `VillageSquare` starts with two report-backed beliefs about remote
// facilities at `OrchardFarm`, both seeded through the live claim-recording path from the same
// informant. One report is old enough to decay below `claim_confidence_threshold`; the other is
// still fresh.
//
// Proves: Stale report claims are pruned from both `entity_claims` and the derived summary, while
// fresher report claims remain.
//
// Chain: report-backed claim ingestion -> confidence decays with claimed-event age -> stale claim
// falls below threshold first -> stale summary disappears while fresh report knowledge remains.
#[test]
fn golden_claim_confidence_threshold_prunes_stale_reports() {
    let observation = run_claim_confidence_scenario(Seed([147; 32]));

    assert!(
        !observation.stale_known,
        "stale report summaries should disappear once their backing claims decay below threshold; observation={observation:?}"
    );
    assert_eq!(
        observation.stale_claim_count, 0,
        "stale report claims should be removed from entity_claims once they fall below threshold; observation={observation:?}"
    );
    assert!(
        observation.fresh_known,
        "fresher report summaries should remain while still above the confidence threshold; observation={observation:?}"
    );
    assert!(
        observation.fresh_claim_count > 0,
        "fresher report claims should remain in entity_claims while still above threshold; observation={observation:?}"
    );
}

#[test]
fn golden_claim_confidence_threshold_prunes_stale_reports_replays_deterministically() {
    let first = run_claim_confidence_scenario(Seed([147; 32]));
    let second = run_claim_confidence_scenario(Seed([147; 32]));

    assert_eq!(first, second);
}
