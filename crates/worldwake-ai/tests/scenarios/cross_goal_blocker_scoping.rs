//! Golden coverage for S150 cross-goal blocker scopes.

use crate::golden_harness::{
    GoldenHarness, ORCHARD_FARM, ProductionOutputOwner, VILLAGE_SQUARE, commit_txn, new_txn,
    place_workstation_with_source, seed_actor_beliefs, stable_wound_list,
};
use worldwake_ai::generate_candidates;
use worldwake_core::{
    AgentBeliefStore, Blocker, BlockerClearingCondition, BlockerMemory, BlockerScope, BlockingFact,
    CommodityKind, CommunicationProfile, ControlSource, DeprivationExposure, Discrepancy,
    DiscrepancyClearing, DiscrepancyEntry, DiscrepancyMemory, DiscrepancySource, DriveThresholds,
    EntityId, EventId, GoalKind, HomeostaticNeeds, MerchandiseProfile, MetabolismProfile,
    PerceptionProfile, PerceptionSource, Permille, Quantity, ResourceSource, RouteSegment, Tick,
    UtilityProfile, WorkstationTag,
};
use worldwake_sim::{PerAgentBeliefView, SpatialBeliefView};

fn pm(value: u16) -> Permille {
    Permille::new(value).expect("test permille should be valid")
}

fn active_blocker(scope: BlockerScope, fact: BlockingFact, expires_tick: Tick) -> Blocker {
    Blocker {
        scope,
        blocking_fact: fact,
        diagnostic_context: None,
        observed_tick: Tick(0),
        expires_tick,
        clearing_condition: BlockerClearingCondition::for_scope_and_fact(
            scope,
            fact,
            BlockerClearingCondition::TtlOnly,
        ),
        baseline_snapshot: None,
        source_event: None,
    }
}

fn route_scope(destination: EntityId) -> BlockerScope {
    BlockerScope::RouteSegment(RouteSegment::new(VILLAGE_SQUARE, destination))
}

fn candidate_kinds(
    h: &GoldenHarness,
    actor: EntityId,
    blocked: &BlockerMemory,
    tick: Tick,
) -> Vec<GoalKind> {
    let view = PerAgentBeliefView::from_world_at_tick_with_recipes(
        actor,
        tick,
        &h.world,
        Some(&h.recipes),
    );
    generate_candidates(&view, actor, blocked, &h.recipes, tick)
        .into_iter()
        .map(|offer| offer.key.kind)
        .collect()
}

fn has_goal(kinds: &[GoalKind], expected: GoalKind) -> bool {
    kinds.contains(&expected)
}

fn seed_decision_agent(h: &mut GoldenHarness, name: &str, place: EntityId) -> EntityId {
    let mut txn = new_txn(&mut h.world, 0);
    let actor = txn.create_agent(name, ControlSource::Ai).unwrap();
    txn.set_ground_location(actor, place).unwrap();
    txn.set_component_homeostatic_needs(
        actor,
        HomeostaticNeeds::new(pm(850), pm(0), pm(0), pm(0), pm(0)),
    )
    .unwrap();
    txn.set_component_deprivation_exposure(actor, DeprivationExposure::default())
        .unwrap();
    txn.set_component_drive_thresholds(actor, DriveThresholds::default())
        .unwrap();
    txn.set_component_metabolism_profile(actor, MetabolismProfile::default())
        .unwrap();
    txn.set_component_perception_profile(actor, PerceptionProfile::default())
        .unwrap();
    txn.set_component_utility_profile(
        actor,
        UtilityProfile {
            care_weight: pm(900),
            social_weight: pm(900),
            ..UtilityProfile::default()
        },
    )
    .unwrap();
    txn.set_component_communication_profile(actor, CommunicationProfile::default())
        .unwrap();
    txn.set_component_epistemic_disposition_profile(
        actor,
        worldwake_core::EpistemicDispositionProfile::default(),
    )
    .unwrap();
    txn.set_component_blocker_memory(actor, BlockerMemory::default())
        .unwrap();
    txn.set_component_discrepancy_memory(actor, DiscrepancyMemory::default())
        .unwrap();
    commit_txn(txn, &mut h.event_log);
    actor
}

fn seed_remote_apple_source(h: &mut GoldenHarness, place: EntityId) -> EntityId {
    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        place,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(10),
            max_quantity: Quantity(10),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
            extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
        },
        ProductionOutputOwner::Actor,
    )
}

fn seed_wounded_patient(h: &mut GoldenHarness, place: EntityId) -> EntityId {
    let mut txn = new_txn(&mut h.world, 0);
    let patient = txn.create_agent("Patient", ControlSource::Ai).unwrap();
    txn.set_ground_location(patient, place).unwrap();
    txn.set_component_wound_list(patient, stable_wound_list(300))
        .unwrap();
    commit_txn(txn, &mut h.event_log);
    patient
}

fn seed_counterparty(h: &mut GoldenHarness, actor: EntityId) -> EntityId {
    let mut txn = new_txn(&mut h.world, 0);
    let counterparty = txn.create_agent("Counterparty", ControlSource::Ai).unwrap();
    let subject = txn.create_agent("Subject", ControlSource::Ai).unwrap();
    txn.set_ground_location(counterparty, VILLAGE_SQUARE)
        .unwrap();
    txn.set_ground_location(subject, VILLAGE_SQUARE).unwrap();
    txn.set_component_agent_belief_store(actor, AgentBeliefStore::new())
        .unwrap();
    txn.set_component_merchandise_profile(
        counterparty,
        MerchandiseProfile {
            sale_kinds: [CommodityKind::Apple].into_iter().collect(),
            home_facility: Some(VILLAGE_SQUARE),
        },
    )
    .unwrap();
    let (facility, _stock, display) = txn
        .create_merchant_facility(
            VILLAGE_SQUARE,
            counterparty,
            worldwake_core::LoadUnits(200),
            Some(worldwake_core::LoadUnits(100)),
        )
        .unwrap();
    let apple = txn
        .create_item_lot(CommodityKind::Apple, Quantity(3))
        .unwrap();
    txn.put_into_container(apple, display.unwrap()).unwrap();
    txn.set_component_stock_assignment(
        apple,
        worldwake_core::StockAssignment {
            facility,
            kind: worldwake_core::StockAssignmentKind::Displayed,
        },
    )
    .unwrap();
    txn.set_component_sale_listing(apple, worldwake_core::SaleListing { listed_at: Tick(0) })
        .unwrap();
    commit_txn(txn, &mut h.event_log);
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        actor,
        &[counterparty, subject],
        Tick(0),
        PerceptionSource::Report {
            from: counterparty,
            chain_len: 1,
        },
    );
    counterparty
}

fn route_fixture() -> (GoldenHarness, EntityId, EntityId, EntityId, EntityId) {
    let mut h = GoldenHarness::new(worldwake_core::Seed([150; 32]));
    let actor = seed_decision_agent(&mut h, "Route Actor", VILLAGE_SQUARE);
    let patient = seed_wounded_patient(&mut h, VILLAGE_SQUARE);
    let second_patient = seed_wounded_patient(&mut h, VILLAGE_SQUARE);
    let destination = {
        let view = PerAgentBeliefView::from_world_at_tick(actor, Tick(0), &h.world);
        view.adjacent_places_with_travel_ticks(VILLAGE_SQUARE)
            .into_iter()
            .map(|(place, _)| place)
            .next()
            .expect("prototype village should have an adjacent route")
    };
    let source = seed_remote_apple_source(&mut h, destination);
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        actor,
        &[patient, second_patient, source],
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    (h, actor, patient, second_patient, destination)
}

#[test]
fn route_segment_blocker_suppresses_multiple_goals_on_same_segment() {
    let (h, actor, patient, second_patient, destination) = route_fixture();
    let first_escort = GoalKind::EscortToSafety {
        subject: patient,
        destination,
    };
    let second_escort = GoalKind::EscortToSafety {
        subject: second_patient,
        destination,
    };
    let baseline = candidate_kinds(&h, actor, &BlockerMemory::default(), Tick(5));
    assert!(has_goal(&baseline, first_escort));
    assert!(has_goal(&baseline, second_escort));

    let mut blocked = BlockerMemory::default();
    blocked.record(active_blocker(
        route_scope(destination),
        BlockingFact::DangerTooHigh,
        Tick(20),
    ));
    let suppressed = candidate_kinds(&h, actor, &blocked, Tick(5));

    assert!(!has_goal(&suppressed, first_escort));
    assert!(!has_goal(&suppressed, second_escort));
}

#[test]
fn counterparty_blocker_suppresses_trade_and_ask_witness_goals() {
    let mut h = GoldenHarness::new(worldwake_core::Seed([151; 32]));
    let actor = seed_decision_agent(&mut h, "Counterparty Actor", VILLAGE_SQUARE);
    let counterparty = seed_counterparty(&mut h, actor);
    let baseline = candidate_kinds(&h, actor, &BlockerMemory::default(), Tick(45));
    assert!(
        baseline.iter().any(|kind| matches!(
            kind,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                ..
            }
        )),
        "local seller should produce an acquisition candidate"
    );
    assert!(
        baseline.iter().any(|kind| matches!(
            kind,
            GoalKind::AskWitness { witness, .. } if *witness == counterparty
        )),
        "stale report from the counterparty should produce an AskWitness candidate"
    );

    let mut blocked = BlockerMemory::default();
    blocked.record(active_blocker(
        BlockerScope::Counterparty(counterparty),
        BlockingFact::NoBuyer,
        Tick(100),
    ));
    let suppressed = candidate_kinds(&h, actor, &blocked, Tick(45));

    assert!(!suppressed.iter().any(|kind| matches!(
        kind,
        GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            ..
        }
    )));
    assert!(!suppressed.iter().any(|kind| matches!(
        kind,
        GoalKind::AskWitness { witness, .. } if *witness == counterparty
    )));
}

#[test]
fn route_segment_ttl_expiry_restores_candidate_emission() {
    let (h, actor, patient, _second_patient, destination) = route_fixture();
    let escort = GoalKind::EscortToSafety {
        subject: patient,
        destination,
    };
    let mut blocked = BlockerMemory::default();
    blocked.record(active_blocker(
        route_scope(destination),
        BlockingFact::DangerTooHigh,
        Tick(10),
    ));

    assert!(!has_goal(
        &candidate_kinds(&h, actor, &blocked, Tick(9)),
        escort
    ));
    blocked.expire(Tick(10));
    assert!(has_goal(
        &candidate_kinds(&h, actor, &blocked, Tick(10)),
        escort
    ));
}

#[test]
fn route_retraversed_safely_clears_matching_blocker_before_ttl() {
    let mut blocked = BlockerMemory::default();
    blocked.record(active_blocker(
        route_scope(ORCHARD_FARM),
        BlockingFact::DangerTooHigh,
        Tick(240),
    ));
    blocked.sweep_cleared(|blocker| {
        matches!(
            blocker.clearing_condition,
            BlockerClearingCondition::RouteRetraversedSafely(segment)
                if segment == RouteSegment::new(VILLAGE_SQUARE, ORCHARD_FARM)
        )
    });

    assert!(blocked.intents.is_empty());
}

#[test]
fn counterparty_accepted_clears_matching_blocker_before_ttl() {
    let counterparty = EntityId {
        slot: 42,
        generation: 0,
    };
    let other = EntityId {
        slot: 43,
        generation: 0,
    };
    let mut blocked = BlockerMemory::default();
    blocked.record(active_blocker(
        BlockerScope::Counterparty(counterparty),
        BlockingFact::NoBuyer,
        Tick(360),
    ));
    blocked.record(active_blocker(
        BlockerScope::Counterparty(other),
        BlockingFact::NoBuyer,
        Tick(360),
    ));
    blocked.sweep_cleared(|blocker| {
        matches!(
            blocker.clearing_condition,
            BlockerClearingCondition::CounterpartyAccepted(cleared) if cleared == counterparty
        )
    });

    assert!(
        !blocked
            .intents
            .contains_key(&BlockerScope::Counterparty(counterparty))
    );
    assert!(
        blocked
            .intents
            .contains_key(&BlockerScope::Counterparty(other))
    );
}

#[test]
fn discrepancy_memory_preserves_parallel_route_scope_suppression() {
    let scope = route_scope(ORCHARD_FARM);
    let mut discrepancies = DiscrepancyMemory::default();
    discrepancies.record(DiscrepancyEntry {
        scope,
        discrepancy: Discrepancy::RouteUnknown,
        observed_tick: Tick(0),
        expires_tick: Tick(30),
        source: DiscrepancySource::ReadPhaseInference,
        clearing_condition: DiscrepancyClearing::TtlExpiry,
    });

    assert!(discrepancies.is_suppressed(&scope, Tick(29)));
    discrepancies.expire(Tick(30));
    assert!(!discrepancies.is_suppressed(&scope, Tick(30)));
}

#[test]
fn blocker_source_event_points_to_recorded_event() {
    let mut h = GoldenHarness::new(worldwake_core::Seed([152; 32]));
    let actor = seed_decision_agent(&mut h, "Source Actor", VILLAGE_SQUARE);
    let source_event = EventId(0);
    assert!(
        h.event_log.get(source_event).is_some(),
        "agent setup should create a concrete source event"
    );
    let mut blocked = BlockerMemory::default();
    let mut stored_intent = active_blocker(
        BlockerScope::Counterparty(actor),
        BlockingFact::PatienceExhausted,
        Tick(60),
    );
    stored_intent.source_event = Some(source_event);
    blocked.record(stored_intent);

    let recorded = blocked
        .intents
        .get(&BlockerScope::Counterparty(actor))
        .expect("blocker should be stored");
    assert_eq!(recorded.source_event, Some(source_event));
    assert!(h.event_log.get(recorded.source_event.unwrap()).is_some());
}

#[test]
fn same_seed_blocker_memory_serializes_identically() {
    fn memory_bytes() -> Vec<u8> {
        let (_h, _actor, _patient, _second_patient, destination) = route_fixture();
        let mut blocked = BlockerMemory::default();
        blocked.record(active_blocker(
            route_scope(destination),
            BlockingFact::DangerTooHigh,
            Tick(240),
        ));
        bincode::serialize(&blocked.intents).expect("blocker memory should serialize")
    }

    assert_eq!(memory_bytes(), memory_bytes());
}
