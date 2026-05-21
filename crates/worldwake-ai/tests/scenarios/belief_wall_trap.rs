//! Golden coverage for S143's FND-14A belief wall.

use std::collections::BTreeSet;
use std::num::NonZeroU32;

use crate::golden_harness::*;
use worldwake_ai::{DecisionOutcome, GoalKey, PlannerOpKind, generate_candidates};
use worldwake_core::{
    ActionDomain, AgentBeliefStore, BeliefConfidencePolicy, BlockerMemory, CarryCapacity, ClaimId,
    ClaimValue, CommodityKind, ControlSource, EntityId, EntityKind, GoalKind, HomeostaticNeeds,
    LoadUnits, MetabolismProfile, PerceptionSource, Permille, ProductionJob, PursuitProfile,
    Quantity, RecipeId, SaleListing, Seed, StockAssignment, StockAssignmentKind, SuccessionLaw,
    TheftDispositionProfile, Tick, UtilityProfile, WorkstationMarker, WorkstationTag,
};
use worldwake_core::{EntityBeliefAspect, EntityBeliefClaim};
use worldwake_sim::{
    BeliefRead, BelievedAuthorityView, EconomicBeliefView, FacilityBeliefView, InventoryBeliefView,
    LocalPhysicalObservationView, PerAgentBeliefView, SpatialBeliefView, get_affordances,
};

struct BeliefWallFixture {
    h: GoldenHarness,
    actor: EntityId,
    owner: EntityId,
    chest: EntityId,
    building: EntityId,
    office: EntityId,
}

struct RemotePursuitFixture {
    h: GoldenHarness,
    actor: EntityId,
    target: EntityId,
    last_seen_place: EntityId,
    current_place: EntityId,
}

struct RemoteSaleFixture {
    h: GoldenHarness,
    actor: EntityId,
    merchant: EntityId,
    market: EntityId,
    listed_lot: EntityId,
}

struct RemoteProductionFixture {
    h: GoldenHarness,
    actor: EntityId,
    remote_workstation: EntityId,
    local_workstation: EntityId,
}

struct RemoteLoadFixture {
    h: GoldenHarness,
    actor: EntityId,
    remote_carrier: EntityId,
    remote_lot: EntityId,
    local_lot: EntityId,
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

fn set_pursuit_profile(h: &mut GoldenHarness, agent: EntityId, profile: PursuitProfile) {
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_pursuit_profile(agent, profile).unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn add_hostility(h: &mut GoldenHarness, subject: EntityId, target: EntityId) {
    let mut txn = new_txn(&mut h.world, 0);
    txn.add_hostility(subject, target).unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn move_entity(h: &mut GoldenHarness, entity: EntityId, place: EntityId, tick: u64) {
    let mut txn = new_txn(&mut h.world, tick);
    txn.set_ground_location(entity, place).unwrap();
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

fn build_remote_pursuit_fixture(seed: Seed) -> RemotePursuitFixture {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let actor = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Stale-Location Actor",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile {
            courage: Permille::new(1000).unwrap(),
            ..UtilityProfile::default()
        },
    );
    let target = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Moved Target",
        ORCHARD_FARM,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_control_source(&mut h, target, ControlSource::Human);
    set_pursuit_profile(
        &mut h,
        actor,
        PursuitProfile {
            min_location_confidence: Permille::new(0).unwrap(),
            max_pursuit_travel_ticks: NonZeroU32::new(20).unwrap(),
        },
    );

    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        actor,
        target,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    add_hostility(&mut h, actor, target);
    move_entity(&mut h, target, RULERS_HALL, 1);

    RemotePursuitFixture {
        h,
        actor,
        target,
        last_seen_place: ORCHARD_FARM,
        current_place: RULERS_HALL,
    }
}

fn build_remote_sale_fixture(seed: Seed) -> RemoteSaleFixture {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let actor = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Remote-Market Actor",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let merchant = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Remote Seller",
        ORCHARD_FARM,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_control_source(&mut h, merchant, ControlSource::Human);

    let listed_lot = {
        let mut txn = new_txn(&mut h.world, 0);
        let (facility, _stock, display) = txn
            .create_merchant_facility(
                ORCHARD_FARM,
                merchant,
                worldwake_core::LoadUnits(200),
                Some(worldwake_core::LoadUnits(100)),
            )
            .unwrap();
        let display = display.expect("merchant facility should include a display container");
        let listed_lot = txn
            .create_item_lot(CommodityKind::Bread, Quantity(5))
            .unwrap();
        txn.put_into_container(listed_lot, display).unwrap();
        txn.set_component_stock_assignment(
            listed_lot,
            StockAssignment {
                facility,
                kind: StockAssignmentKind::Displayed,
            },
        )
        .unwrap();
        txn.set_component_sale_listing(listed_lot, SaleListing { listed_at: Tick(0) })
            .unwrap();
        commit_txn(txn, &mut h.event_log);
        listed_lot
    };

    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        actor,
        merchant,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        actor,
        listed_lot,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    RemoteSaleFixture {
        h,
        actor,
        merchant,
        market: ORCHARD_FARM,
        listed_lot,
    }
}

fn build_remote_production_fixture(seed: Seed) -> RemoteProductionFixture {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let actor = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Production-Wall Actor",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let worker = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Remote Worker",
        ORCHARD_FARM,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_control_source(&mut h, worker, ControlSource::Human);

    let (remote_workstation, local_workstation) = {
        let mut txn = new_txn(&mut h.world, 0);
        let remote = txn.create_entity(EntityKind::Facility);
        txn.set_ground_location(remote, ORCHARD_FARM).unwrap();
        txn.set_component_workstation_marker(remote, WorkstationMarker(WorkstationTag::Mill))
            .unwrap();
        let local = txn.create_entity(EntityKind::Facility);
        txn.set_ground_location(local, VILLAGE_SQUARE).unwrap();
        txn.set_component_workstation_marker(local, WorkstationMarker(WorkstationTag::Mill))
            .unwrap();
        commit_txn(txn, &mut h.event_log);
        (remote, local)
    };

    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        actor,
        remote_workstation,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        actor,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    let mut txn = new_txn(&mut h.world, 1);
    txn.set_component_production_job(
        remote_workstation,
        ProductionJob {
            recipe_id: RecipeId(1),
            worker,
            staged_inputs_container: remote_workstation,
            progress_ticks: 0,
        },
    )
    .unwrap();
    txn.set_component_production_job(
        local_workstation,
        ProductionJob {
            recipe_id: RecipeId(1),
            worker: actor,
            staged_inputs_container: local_workstation,
            progress_ticks: 0,
        },
    )
    .unwrap();
    commit_txn(txn, &mut h.event_log);

    RemoteProductionFixture {
        h,
        actor,
        remote_workstation,
        local_workstation,
    }
}

fn build_remote_load_fixture(seed: Seed) -> RemoteLoadFixture {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let actor = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Load-Wall Actor",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let remote_carrier = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Remote Carrier",
        ORCHARD_FARM,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_control_source(&mut h, remote_carrier, ControlSource::Human);

    let (remote_lot, local_lot) = {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_carry_capacity(remote_carrier, CarryCapacity(LoadUnits(77)))
            .unwrap();
        let remote_lot = txn
            .create_item_lot(CommodityKind::Water, Quantity(3))
            .unwrap();
        txn.set_ground_location(remote_lot, ORCHARD_FARM).unwrap();
        let local_lot = txn
            .create_item_lot(CommodityKind::Water, Quantity(2))
            .unwrap();
        txn.set_ground_location(local_lot, VILLAGE_SQUARE).unwrap();
        commit_txn(txn, &mut h.event_log);
        (remote_lot, local_lot)
    };

    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        actor,
        remote_carrier,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        actor,
        remote_lot,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        actor,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    RemoteLoadFixture {
        h,
        actor,
        remote_carrier,
        remote_lot,
        local_lot,
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

fn remote_sale_view_for(fixture: &RemoteSaleFixture) -> PerAgentBeliefView<'_> {
    PerAgentBeliefView::from_world(fixture.actor, &fixture.h.world)
}

fn remote_pursuit_view_for(
    fixture: &RemotePursuitFixture,
) -> worldwake_sim::PerAgentBeliefView<'_> {
    worldwake_sim::PerAgentBeliefView::from_world(fixture.actor, &fixture.h.world)
}

fn remote_production_view_for(fixture: &RemoteProductionFixture) -> PerAgentBeliefView<'_> {
    PerAgentBeliefView::from_world(fixture.actor, &fixture.h.world)
}

fn remote_load_view_for(fixture: &RemoteLoadFixture) -> PerAgentBeliefView<'_> {
    PerAgentBeliefView::from_world(fixture.actor, &fixture.h.world)
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

fn assert_remote_pursuit_uses_last_seen_place(fixture: &RemotePursuitFixture) {
    let view = remote_pursuit_view_for(fixture);
    assert_eq!(
        fixture.h.world.effective_place(fixture.target),
        Some(fixture.current_place)
    );
    assert_eq!(
        SpatialBeliefView::effective_place(&view, fixture.target),
        Some(fixture.last_seen_place)
    );

    let goal = GoalKey::from(GoalKind::EngageHostile {
        target: fixture.target,
    });
    let candidates = generate_candidates(
        &view,
        fixture.actor,
        &BlockerMemory::default(),
        &fixture.h.recipes,
        Tick(1),
    );
    let pursuit_offer = candidates
        .iter()
        .find(|candidate| candidate.key == goal)
        .unwrap_or_else(|| {
            panic!(
                "expected remote EngageHostile candidate from stale belief; candidates={candidates:#?}"
            )
        });
    assert!(
        pursuit_offer
            .evidence_places
            .contains(&fixture.last_seen_place),
        "pursuit evidence should contain the last-seen place; offer={pursuit_offer:#?}"
    );
    assert!(
        !pursuit_offer
            .evidence_places
            .contains(&fixture.current_place),
        "pursuit evidence must not contain the target's current remote place; offer={pursuit_offer:#?}"
    );
}

fn assert_remote_pursuit_trace_never_targets_current_place(fixture: &RemotePursuitFixture) {
    let goal = GoalKey::from(GoalKind::EngageHostile {
        target: fixture.target,
    });
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

    let mut saw_last_seen_evidence = false;
    for trace in &traces {
        let DecisionOutcome::Planning(ref planning) = trace.outcome else {
            continue;
        };
        for evidence in planning.candidates.evidence_for_goal(goal) {
            let Some(pursuit) = evidence.pursuit else {
                continue;
            };
            if pursuit.believed_place == Some(fixture.last_seen_place) {
                saw_last_seen_evidence = true;
            }
            assert_ne!(
                pursuit.believed_place,
                Some(fixture.current_place),
                "decision trace must not use the target's current remote place: {pursuit:#?}"
            );
        }
        if planning.selection.selected_goal_is(goal)
            && let Some(plan) = planning.selection.selected_plan.as_ref()
        {
            assert!(
                !plan
                    .steps
                    .iter()
                    .any(|step| step.op_kind == PlannerOpKind::Travel
                        && step.targets.contains(&fixture.current_place)),
                "selected pursuit plan must not travel to the target's current remote place; plan={plan:#?}"
            );
        }
    }
    assert!(
        saw_last_seen_evidence,
        "expected decision trace evidence for stale last-seen place; traces={traces:#?}"
    );
}

fn assert_remote_sale_listing_does_not_use_live_truth(fixture: &RemoteSaleFixture) {
    let view = remote_sale_view_for(fixture);
    assert_eq!(
        fixture.h.world.effective_place(fixture.actor),
        Some(VILLAGE_SQUARE)
    );
    assert_eq!(
        fixture.h.world.effective_place(fixture.listed_lot),
        Some(fixture.market)
    );
    assert_eq!(
        fixture.h.world.effective_place(fixture.merchant),
        Some(fixture.market)
    );

    assert_eq!(
        EconomicBeliefView::listed_sale_lots_at(&view, fixture.market, CommodityKind::Bread),
        Vec::<EntityId>::new(),
        "remote listed sale lots must not be enumerated from current world truth"
    );
    assert_eq!(
        EconomicBeliefView::seller_for_sale_lot(&view, fixture.listed_lot),
        None,
        "remote seller identity must not be read through the live sale listing"
    );
    assert!(
        !EconomicBeliefView::has_sale_listing(&view, fixture.listed_lot),
        "remote sale-listing presence must not come from current world truth"
    );
}

fn assert_remote_production_job_does_not_use_live_truth(fixture: &RemoteProductionFixture) {
    let view = remote_production_view_for(fixture);
    assert_eq!(
        fixture.h.world.effective_place(fixture.actor),
        Some(VILLAGE_SQUARE)
    );
    assert_eq!(
        fixture.h.world.effective_place(fixture.remote_workstation),
        Some(ORCHARD_FARM)
    );
    assert!(
        fixture
            .h
            .world
            .has_component_production_job(fixture.remote_workstation),
        "fixture must contain a live remote production job to prove the leak is reachable"
    );

    assert!(
        !FacilityBeliefView::has_production_job(&view, fixture.remote_workstation),
        "remote production-job presence must not be read from current world truth"
    );
    assert!(
        FacilityBeliefView::has_production_job(&view, fixture.local_workstation),
        "co-located workstation busy/idle state remains directly observable"
    );
}

fn assert_remote_production_activity_belief_is_used(fixture: &mut RemoteProductionFixture) {
    let mut store = fixture
        .h
        .world
        .get_component_agent_belief_store(fixture.actor)
        .cloned()
        .unwrap_or_else(AgentBeliefStore::new);
    assert!(
        store.update_believed_activity(
            &fixture.remote_workstation,
            Some(worldwake_core::BelievedActivity {
                action_domain: ActionDomain::Production,
                target: Some(fixture.remote_workstation),
                observed_tick: Tick(0),
            }),
            Tick(1),
            &BeliefConfidencePolicy::default(),
        ),
        "fixture should update the known remote workstation's activity belief"
    );
    let mut txn = new_txn(&mut fixture.h.world, 1);
    txn.set_component_agent_belief_store(fixture.actor, store)
        .unwrap();
    commit_txn(txn, &mut fixture.h.event_log);

    let view = remote_production_view_for(fixture);
    assert!(
        FacilityBeliefView::has_production_job(&view, fixture.remote_workstation),
        "explicit remote activity belief should expose the believed production job"
    );
}

fn assert_remote_load_and_capacity_do_not_use_live_truth(fixture: &RemoteLoadFixture) {
    let view = remote_load_view_for(fixture);
    assert_eq!(
        fixture.h.world.effective_place(fixture.actor),
        Some(VILLAGE_SQUARE)
    );
    assert_eq!(
        fixture.h.world.effective_place(fixture.remote_carrier),
        Some(ORCHARD_FARM)
    );
    assert_eq!(
        fixture.h.world.effective_place(fixture.remote_lot),
        Some(ORCHARD_FARM)
    );
    assert_eq!(
        fixture
            .h
            .world
            .get_component_carry_capacity(fixture.remote_carrier),
        Some(&CarryCapacity(LoadUnits(77))),
        "fixture must contain a live remote carry capacity to prove the leak is reachable"
    );
    assert_eq!(
        worldwake_core::load_of_entity(&fixture.h.world, fixture.remote_lot).ok(),
        Some(LoadUnits(6)),
        "fixture must contain live remote load to prove the leak is reachable"
    );

    assert_eq!(
        InventoryBeliefView::carry_capacity(&view, fixture.remote_carrier),
        None,
        "remote carry capacity must not be read from current world truth"
    );
    assert_eq!(
        InventoryBeliefView::load_of_entity(&view, fixture.remote_lot),
        None,
        "remote load must not be read from current world truth"
    );
    assert_eq!(
        InventoryBeliefView::load_of_entity(&view, fixture.local_lot),
        Some(LoadUnits(4)),
        "co-located load remains directly observable"
    );
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

fn affordance_fingerprint(fixture: &BeliefWallFixture) -> Vec<String> {
    let view = authority_view_for(fixture);
    get_affordances(&view, fixture.actor, &fixture.h.defs, &fixture.h.handlers)
        .into_iter()
        .map(|affordance| {
            format!(
                "{:?}|{:?}|{:?}",
                affordance.def_id, affordance.bound_targets, affordance.payload_override
            )
        })
        .collect()
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

// Scenario 454: Stale Remote Pursuit Uses Last-Seen Place
//
// Systems: Perception, AI, Combat
// ActionDomains: Combat, Travel
// Places: VillageSquare, OrchardFarm, RulersHall
// Principles: 7, 14, 14A, 15, 29
//
// Setup: An AI actor has a direct stale belief that a hostile target was at Orchard Farm. The target is then moved to Ruler's Hall with no witness, testimony, record, or local observation channel delivering that move to the actor.
//
// Proves: Remote pursuit candidate generation and the decision trace use the actor's stale last-known place and never the target's current authoritative remote place.
//
// Chain: direct observation belief -> hidden remote move -> belief-view effective_place -> remote hostile candidate evidence -> decision trace excludes live remote truth.
#[test]
fn golden_belief_wall_trap_remote_pursuit_uses_stale_location_not_live_truth() {
    let mut fixture = build_remote_pursuit_fixture(Seed([0x46; 32]));

    assert_remote_pursuit_uses_last_seen_place(&fixture);

    fixture.h.step_once();

    assert_remote_pursuit_trace_never_targets_current_place(&fixture);
}

// Scenario 456: Remote Sale Listing Does Not Leak Live Truth
//
// Systems: Perception, AI, Trade
// ActionDomains: Trade
// Places: VillageSquare, OrchardFarm
// Principles: 7, 14, 14A, 16, 19
//
// Setup: An actor at Village Square has stale prior beliefs about a remote
// seller and displayed bread lot at Orchard Farm. The authoritative world still
// has a live sale listing, but no witness, record, testimony, or local
// observation carries that current listing state back to the actor.
//
// Proves: The economic belief-view accessors do not enumerate remote sale
// listings, seller identity, or listing presence from live world truth for a
// known-but-remote lot.
//
// Chain: prior remote belief -> live remote sale listing remains authoritative
// -> economic belief view -> no remote listing/seller/listing-presence leak.
#[test]
fn golden_belief_wall_trap_remote_sale_listing_does_not_leak_live_truth() {
    let fixture = build_remote_sale_fixture(Seed([0x48; 32]));

    assert_remote_sale_listing_does_not_use_live_truth(&fixture);
}

// Scenario 457: Remote Production Job Does Not Leak Live Truth
//
// Systems: Perception, AI, Production
// ActionDomains: Production
// Places: VillageSquare, OrchardFarm
// Principles: 7, 14, 14A, 16, 19
//
// Setup: An actor at Village Square has stale prior belief of a remote mill at Orchard Farm; the authoritative remote mill starts a production job without testimony, record, or local observation carrying that activity to the actor.
//
// Proves: `PerAgentBeliefView::has_production_job` does not expose live remote production-job state, while co-located busy/idle workstation state and an explicit `Activity` belief remain lawful sources.
//
// Chain: prior remote workstation belief -> hidden remote job start -> production belief view -> no remote busy/free leak -> explicit activity belief restores believed remote busy state.
#[test]
fn golden_belief_wall_trap_remote_production_job_unseen() {
    let mut fixture = build_remote_production_fixture(Seed([0x49; 32]));

    assert_remote_production_job_does_not_use_live_truth(&fixture);
    assert_remote_production_activity_belief_is_used(&mut fixture);
}

// Scenario 458: Remote Load Change Does Not Leak Live Truth
//
// Systems: Perception, AI, Inventory
// ActionDomains: Trade, Travel
// Places: VillageSquare, OrchardFarm
// Principles: 7, 14, 14A, 16, 19
//
// Setup: An actor at Village Square has stale prior beliefs about a remote carrier and remote water lot at Orchard Farm; the authoritative remote carry capacity and lot load exist, but no lawful carrier brings those physical facts back to the actor.
//
// Proves: `carry_capacity` and `load_of_entity` do not expose live remote physical state, while co-located load remains directly observable.
//
// Chain: prior remote entity belief -> live remote capacity/load remains authoritative -> inventory belief view -> no remote physical-state leak.
#[test]
fn golden_belief_wall_trap_remote_load_change_unseen() {
    let fixture = build_remote_load_fixture(Seed([0x4a; 32]));

    assert_remote_load_and_capacity_do_not_use_live_truth(&fixture);
}

// Scenario 455: Control Source Swap Preserves Belief Affordances
//
// Systems: Perception, AI
// ActionDomains: Crime, Travel, Production
// Places: VillageSquare, OrchardFarm
// Principles: 14, 14A, 19
//
// Setup: One agent body stands in an unchanged belief and world state beside a locally observed but socially unknown chest; only `ControlSource` is swapped from AI to Human.
//
// Proves: The lawful belief-facing affordance set is identical for AI and human control of the same body, preserving agent symmetry while social control facts remain belief-gated.
//
// Chain: stable body/world/belief state -> AI affordance enumeration -> control-source swap -> Human affordance enumeration -> identical affordance fingerprint.
#[test]
fn golden_belief_wall_trap_control_source_swap_preserves_affordances() {
    let mut fixture = build_fixture(Seed([0x47; 32]));

    let ai_affordances = affordance_fingerprint(&fixture);
    set_control_source(&mut fixture.h, fixture.actor, ControlSource::Human);
    let human_affordances = affordance_fingerprint(&fixture);

    assert_eq!(ai_affordances, human_affordances);
    assert_no_authority_beliefs(&fixture);
}
