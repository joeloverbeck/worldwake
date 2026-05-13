//! Golden coverage for S143's FND-14A belief wall.

mod golden_harness;

use std::collections::BTreeSet;
use std::num::NonZeroU32;

use golden_harness::*;
use worldwake_ai::{DecisionOutcome, generate_candidates};
use worldwake_core::{
    AgentBeliefStore, BlockerMemory, ClaimId, ClaimValue, CommodityKind, ControlSource, EntityId,
    EntityKind, GoalKind, HomeostaticNeeds, MetabolismProfile, PerceptionSource, Permille,
    Quantity, Seed, SuccessionLaw, TheftDispositionProfile, Tick, UtilityProfile,
};
use worldwake_core::{EntityBeliefAspect, EntityBeliefClaim};
use worldwake_sim::{BeliefRead, BelievedAuthorityView, LocalPhysicalObservationView};

struct BeliefWallFixture {
    h: GoldenHarness,
    actor: EntityId,
    owner: EntityId,
    chest: EntityId,
    building: EntityId,
    office: EntityId,
}

fn set_control_source(h: &mut GoldenHarness, agent: EntityId, control_source: ControlSource) {
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_agent_data(agent, worldwake_core::AgentData { control_source })
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn set_theft_profile(h: &mut GoldenHarness, agent: EntityId, profile: TheftDispositionProfile) {
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_theft_disposition_profile(agent, profile)
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn build_fixture(seed: Seed) -> BeliefWallFixture {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let actor = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Belief-Wall Actor",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let owner = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Remote Owner",
        ORCHARD_FARM,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_control_source(&mut h, owner, ControlSource::Human);
    set_theft_profile(
        &mut h,
        actor,
        TheftDispositionProfile {
            steal_duration_ticks: NonZeroU32::new(3).unwrap(),
            theft_motive_weight: Permille::new(1000).unwrap(),
            witness_risk_penalty: Permille::new(0).unwrap(),
        },
    );

    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "Market Warden",
        VILLAGE_SQUARE,
        SuccessionLaw::Support,
        12,
        Vec::new(),
    );
    seed_office_vacancy_entry(&mut h.world, &mut h.event_log, office, VILLAGE_SQUARE);

    let (chest, building) = {
        let mut txn = new_txn(&mut h.world, 0);
        let chest = txn
            .create_item_lot(CommodityKind::Coin, Quantity(3))
            .unwrap();
        txn.set_ground_location(chest, VILLAGE_SQUARE).unwrap();
        txn.set_owner(chest, owner).unwrap();

        let building = txn.create_entity(EntityKind::Facility);
        txn.set_ground_location(building, VILLAGE_SQUARE).unwrap();
        commit_txn(txn, &mut h.event_log);
        (chest, building)
    };

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        actor,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    BeliefWallFixture {
        h,
        actor,
        owner,
        chest,
        building,
        office,
    }
}

fn authority_view_for(fixture: &BeliefWallFixture) -> worldwake_sim::PerAgentBeliefView<'_> {
    let store = fixture
        .h
        .world
        .get_component_agent_belief_store(fixture.actor)
        .expect("fixture should seed actor belief store");
    worldwake_sim::PerAgentBeliefView::new_at_tick(fixture.actor, Tick(0), &fixture.h.world, store)
}

fn assert_no_authority_beliefs(fixture: &BeliefWallFixture) {
    let view = authority_view_for(fixture);
    assert!(matches!(
        view.believed_owner_of(fixture.chest),
        BeliefRead::Unknown
    ));
    assert!(matches!(
        view.believed_holder_of(fixture.chest),
        BeliefRead::Unknown
    ));
    assert!(matches!(
        view.believed_jurisdiction(fixture.building),
        BeliefRead::Unknown
    ));
    assert!(matches!(
        view.believed_office_holder(fixture.office),
        BeliefRead::Unknown
    ));
}

fn assert_local_physical_observation(fixture: &BeliefWallFixture) {
    let view = authority_view_for(fixture);
    let colocated = view.colocated_entities(fixture.actor);
    assert!(
        colocated.value.contains(&fixture.chest),
        "co-located physical observation should include the chest; got {:?}",
        colocated.value
    );
    assert!(
        colocated.value.contains(&fixture.building),
        "co-located physical observation should include the building; got {:?}",
        colocated.value
    );
    assert_eq!(
        view.observed_entity_kind(fixture.chest).value,
        Some(EntityKind::ItemLot)
    );
    assert_eq!(
        view.observed_entity_kind(fixture.building).value,
        Some(EntityKind::Facility)
    );
    assert_eq!(fixture.h.world.owner_of(fixture.chest), Some(fixture.owner));
}

fn is_steal_item(goal: GoalKind, target: EntityId) -> bool {
    matches!(goal, GoalKind::StealItem { target_item } if target_item == target)
}

fn assert_no_steal_candidate_from_generation(fixture: &BeliefWallFixture) {
    let view = authority_view_for(fixture);
    let candidates = generate_candidates(
        &view,
        fixture.actor,
        &BlockerMemory::default(),
        &fixture.h.recipes,
        Tick(0),
    );
    assert!(
        !candidates
            .iter()
            .any(|candidate| is_steal_item(candidate.key.kind, fixture.chest)),
        "ownerless-in-belief co-located chest must not emit StealItem; candidates={candidates:#?}"
    );
}

fn assert_no_steal_candidate_in_decision_trace(fixture: &BeliefWallFixture) {
    let trace_sink = fixture
        .h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled");
    let traces = trace_sink.traces_for(fixture.actor);
    assert!(
        traces
            .iter()
            .any(|trace| matches!(trace.outcome, DecisionOutcome::Planning(_))),
        "expected at least one planning trace; traces={traces:#?}"
    );
    for trace in traces {
        let DecisionOutcome::Planning(ref planning) = trace.outcome else {
            continue;
        };
        assert!(
            !planning
                .candidates
                .generated
                .iter()
                .any(|candidate| is_steal_item(candidate.goal_key.kind, fixture.chest)),
            "decision trace generated a forbidden StealItem candidate at {:?}: {:#?}",
            trace.tick,
            planning.candidates
        );
    }
}

fn assert_no_steal_action_committed(fixture: &BeliefWallFixture) {
    let action_sink = fixture
        .h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    assert!(
        !action_sink
            .events_for(fixture.actor)
            .iter()
            .any(|event| event.action_name == "steal"
                && matches!(event.kind, worldwake_sim::ActionTraceKind::Committed { .. })),
        "ownerless-in-belief co-located chest must not reach a committed steal action"
    );
}

// Scenario 420: Belief Wall Trap Suppresses Theft
//
// Systems: Perception, AI, Crime
// ActionDomains: Crime
// Places: VillageSquare, OrchardFarm
// Principles: 7, 14, 14A, 20
//
// Setup: An AI actor stands beside an authoritatively owned coin lot and a facility; the actor has direct local physical observations but no owner, holder, jurisdiction, or office-holder belief claims.
//
// Proves: FND-14A's co-location exception remains physical-only: local observation sees the chest/building, `BelievedAuthorityView` returns `Unknown` for social authority facts, and theft is absent from both candidate generation and the decision trace.
//
// Chain: bootstrap authoritative ownership -> local physical observation -> authority-belief absence -> theft candidate generation wall -> no committed steal.
#[test]
fn golden_belief_wall_trap_suppresses_theft_without_authority_belief() {
    let mut fixture = build_fixture(Seed([0x43; 32]));

    assert_local_physical_observation(&fixture);
    assert_no_authority_beliefs(&fixture);
    assert_no_steal_candidate_from_generation(&fixture);

    fixture.h.step_once();

    assert_no_steal_candidate_in_decision_trace(&fixture);
    assert_no_steal_action_committed(&fixture);
    assert_no_authority_beliefs(&fixture);
}

#[test]
fn golden_belief_wall_trap_replays_deterministically() {
    let first = build_fixture(Seed([0x44; 32]));
    let second = build_fixture(Seed([0x44; 32]));

    let first_view = authority_view_for(&first);
    let second_view = authority_view_for(&second);

    assert_eq!(
        first_view.colocated_entities(first.actor).value,
        second_view.colocated_entities(second.actor).value
    );
    assert!(matches!(
        first_view.believed_owner_of(first.chest),
        BeliefRead::Unknown
    ));
    assert!(matches!(
        second_view.believed_owner_of(second.chest),
        BeliefRead::Unknown
    ));
    assert!(matches!(
        first_view.believed_holder_of(first.chest),
        BeliefRead::Unknown
    ));
    assert!(matches!(
        second_view.believed_holder_of(second.chest),
        BeliefRead::Unknown
    ));

    let first_candidates = generate_candidates(
        &first_view,
        first.actor,
        &BlockerMemory::default(),
        &first.h.recipes,
        Tick(0),
    )
    .into_iter()
    .map(|candidate| candidate.key)
    .collect::<BTreeSet<_>>();
    let second_candidates = generate_candidates(
        &second_view,
        second.actor,
        &BlockerMemory::default(),
        &second.h.recipes,
        Tick(0),
    )
    .into_iter()
    .map(|candidate| candidate.key)
    .collect::<BTreeSet<_>>();

    assert_eq!(first_candidates, second_candidates);
}

#[test]
fn explicit_owner_belief_is_the_theft_candidate_gate() {
    let mut fixture = build_fixture(Seed([0x45; 32]));
    let mut store = fixture
        .h
        .world
        .get_component_agent_belief_store(fixture.actor)
        .cloned()
        .unwrap_or_else(AgentBeliefStore::new);
    store.record_entity_claim(EntityBeliefClaim {
        claim_id: ClaimId(1),
        subject: fixture.chest,
        aspect: EntityBeliefAspect::Owner,
        value: ClaimValue::Entity(Some(fixture.owner)),
        source: PerceptionSource::Report {
            from: fixture.owner,
            chain_len: 0,
        },
        acquired_tick: Tick(0),
        claimed_event_tick: Some(Tick(0)),
        confidence: Permille::new(1000).unwrap(),
        refuted_at_tick: None,
    });

    let mut txn = new_txn(&mut fixture.h.world, 0);
    txn.set_component_agent_belief_store(fixture.actor, store)
        .unwrap();
    commit_txn(txn, &mut fixture.h.event_log);

    let view = authority_view_for(&fixture);
    let candidates = generate_candidates(
        &view,
        fixture.actor,
        &BlockerMemory::default(),
        &fixture.h.recipes,
        Tick(0),
    );

    assert!(
        candidates
            .iter()
            .any(|candidate| is_steal_item(candidate.key.kind, fixture.chest)),
        "explicit owner belief should be sufficient to admit the theft candidate; candidates={candidates:#?}"
    );
}
